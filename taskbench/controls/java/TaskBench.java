// Java control for the bet-3 stack harness (taskbench): JDK 21+ virtual threads.
//
// Workloads: spawn-join, idle-memory, echo, idle-connections, starvation and
// max-tasks, writing rows in the same CSV schema as the Rust harness and the Go
// control. Virtual threads are not a non-moving-stack precedent (a parked
// virtual thread's frames are copied into heap stack-chunk objects), which is why
// the harness measures them: they are the other managed-runtime baseline.
//
// Run with the scheduler's parallelism matched to the harness's --threads:
//   java -Djdk.virtualThreadScheduler.parallelism=N -Djdk.virtualThreadScheduler.maxPoolSize=N \
//        TaskBench <benchmark> --threads N [--tasks N] [--csv PATH] [--machine-id ID] ...
// Blocking waits use java.util.concurrent (CountDownLatch, AtomicInteger), never
// `synchronized`, which pins the carrier thread on JDK 21-23.

import java.io.*;
import java.net.*;
import java.nio.file.*;
import java.time.*;
import java.time.format.DateTimeFormatter;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;

public final class TaskBench {
    static final String HEADER = "schema,run_id,timestamp_utc,machine_id,implementation,benchmark,strategy,reservation_bytes,stack_bytes,threads,tasks,params,metric,value,unit,status,note";

    static PrintStream csv;
    static String runId, machine, bench, params = "";
    static int threads;
    static long tasks;
    static boolean quiet;
    static volatile long sink;

    static String esc(String s) {
        if (s.indexOf(',') >= 0 || s.indexOf('"') >= 0 || s.indexOf('\n') >= 0 || s.indexOf('\r') >= 0)
            return "\"" + s.replace("\"", "\"\"") + "\"";
        return s;
    }

    static String fmt(double v) {
        if (Double.isNaN(v) || Double.isInfinite(v)) return "";
        if (v == Math.rint(v) && Math.abs(v) < 1e15) return Long.toString((long) v);
        return String.format(Locale.ROOT, "%.3f", v);
    }

    static void row(String metric, Double value, String unit, String status, String note) {
        String v = value == null ? "" : fmt(value);
        if (!quiet) {
            String extra = (status.equals("ok") ? "" : " [" + status + "]") + (note.isEmpty() ? "" : "  (" + note + ")");
            System.out.printf(Locale.ROOT, "  %-34s %14s %-6s%s%n", metric, v, unit, extra);
        }
        if (csv == null) return;
        String ts = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss'Z'").format(LocalDateTime.now(ZoneOffset.UTC));
        String[] f = {"1", esc(runId), ts, esc(machine), "java", esc(bench), "java", "", "", Integer.toString(threads),
                Long.toString(tasks), esc(params), esc(metric), v, esc(unit), status, esc(note)};
        csv.println(String.join(",", f));
        csv.flush();
    }

    static void metric(String m, double v, String unit) { row(m, v, unit, "ok", ""); }

    /** Nearest-rank percentile, as in taskbench src/stats.rs. */
    static long pct(long[] sorted, double p) {
        if (sorted.length == 0) return 0;
        int rank = (int) Math.ceil(p / 100.0 * sorted.length);
        rank = Math.max(1, Math.min(rank, sorted.length));
        return sorted[rank - 1];
    }

    static void summary(String prefix, long[] s) {
        Arrays.sort(s);
        double sum = 0;
        for (long v : s) sum += v;
        metric(prefix + "_p50", pct(s, 50), "ns");
        metric(prefix + "_p90", pct(s, 90), "ns");
        metric(prefix + "_p99", pct(s, 99), "ns");
        metric(prefix + "_p999", pct(s, 99.9), "ns");
        metric(prefix + "_max", s.length == 0 ? 0 : s[s.length - 1], "ns");
        metric(prefix + "_mean", s.length == 0 ? 0 : Math.rint(sum / s.length), "ns");
        metric(prefix + "_samples", s.length, "count");
    }

    record Mem(long rss, long pte) {}

    static Mem snapshot() {
        long rss = 0, pte = 0;
        try {
            for (String l : Files.readAllLines(Path.of("/proc/self/status"))) {
                String[] f = l.trim().split("\\s+");
                if (f.length < 2) continue;
                if (f[0].equals("VmRSS:")) rss = Long.parseLong(f[1]) * 1024;
                if (f[0].equals("VmPTE:")) pte = Long.parseLong(f[1]) * 1024;
            }
        } catch (IOException e) { }
        return new Mem(rss, pte);
    }

    static double perItem(long before, long after, long n) { return n == 0 ? 0 : Math.rint((double) (after - before) / n); }

    static long heapUsed() {
        Runtime r = Runtime.getRuntime();
        return r.totalMemory() - r.freeMemory();
    }

    static void gc() {
        for (int i = 0; i < 3; i++) System.gc();
        try { Thread.sleep(100); } catch (InterruptedException e) { }
    }

    static long shallowWork(long seed) {
        byte[] buf = new byte[0]; // Java has no stack arrays; a few calls deep instead
        return shallowInner(seed, 3) + buf.length;
    }

    static long shallowInner(long x, int depth) {
        if (depth == 0) return x;
        return shallowInner(x * 31 ^ depth, depth - 1);
    }

    /** About 32 KiB of frames: 512 levels of a method with several live longs. */
    static long excursion(int levels, long a, long b, long c, long d) {
        if (levels == 0) return a ^ b ^ c ^ d;
        long r = excursion(levels - 1, a + 1, b ^ a, c * 3, d - b);
        return r + a + b + c + d;
    }

    static long spin(long x, long n) {
        for (long i = 0; i < n; i++) {
            x = x * 6364136223846793005L + 1442695040888963407L;
            x ^= x >>> 29;
        }
        return x;
    }

    static void spawnJoin(int rounds, int warmup, int batch) throws Exception {
        long[] samples = new long[rounds];
        Thread[] ts = new Thread[batch];
        for (int r = 0; r < warmup + rounds; r++) {
            final int rr = r;
            long t0 = System.nanoTime();
            for (int i = 0; i < batch; i++) {
                final int v = rr ^ i;
                ts[i] = Thread.ofVirtual().start(() -> { sink += v; });
            }
            for (int i = 0; i < batch; i++) ts[i].join();
            long dt = (System.nanoTime() - t0) / batch;
            if (r >= warmup) samples[r - warmup] = dt;
        }
        summary("spawn_join", samples);
    }

    static void waitFor(AtomicLong c, long target) throws InterruptedException {
        waitFor(c, target, () -> { });
    }

    /** Waits until c reaches target (or 60 s plus 50 us per item), running check every 2 ms; true if reached. */
    static boolean waitFor(AtomicLong c, long target, Runnable check) throws InterruptedException {
        long deadline = System.nanoTime() + 60_000_000_000L + 50_000L * target;
        while (c.get() < target && System.nanoTime() < deadline) {
            check.run();
            Thread.sleep(2);
        }
        return c.get() >= target;
    }

    /**
     * Mirrors taskbench's idle-memory, including its memory guard: once the
     * headroom (system, cgroup or Java heap) is below guard during the excursion
     * phase, virtual threads that have not made their excursion yet skip it and
     * the phase is reported as failed.
     */
    static void idleMemory(int n, int requestedLevels, long settleMs, long guard, boolean skip) throws Exception {
        // With skip (the harness's --skip-excursion) the excursion phase is not run
        // and its cells are reported as skipped.
        final int excursionLevels = skip ? 0 : requestedLevels;
        AtomicLong ready = new AtomicLong(), skipped = new AtomicLong();
        AtomicBoolean stop = new AtomicBoolean();
        CountDownLatch gate0 = new CountDownLatch(1), gate1 = new CountDownLatch(1);
        gc();
        Mem base = snapshot();
        long heap0 = heapUsed();
        Thread[] ts = new Thread[n];
        for (int i = 0; i < n; i++) {
            final long seed = i;
            ts[i] = Thread.ofVirtual().start(() -> {
                try {
                    sink += shallowWork(seed);
                    ready.incrementAndGet();
                    gate0.await();
                    if (excursionLevels > 0 && stop.get()) skipped.incrementAndGet();
                    else if (excursionLevels > 0) {
                        try {
                            sink += excursion(excursionLevels, seed, 1, 2, 3);
                        } catch (OutOfMemoryError e) {
                            stop.set(true);
                            skipped.incrementAndGet();
                        }
                    }
                    ready.incrementAndGet();
                    gate1.await();
                } catch (InterruptedException e) { }
            });
        }
        Map<String, String> failed = new HashMap<>();
        Map<String, String> notRun = new HashMap<>();
        if (skip && requestedLevels > 0) {
            String why = "not run (--skip-excursion): the excursion phase is required only of strategies that release pages on park (owner's policy 7)";
            notRun.put("excursion", why);
            notRun.put("excursion_after_gc", why);
        }
        if (!waitFor(ready, n, () -> { })) failed.put("shallow", "virtual threads did not all park before the deadline");
        Thread.sleep(settleMs);
        Mem shallow = snapshot();
        gc();
        long heapShallow = heapUsed();
        gate0.countDown();
        if (!waitFor(ready, 2L * n, () -> { if (!stop.get() && memHeadroom() < guard) stop.set(true); }))
            failed.put("excursion", "virtual threads did not all park again after the excursion before the deadline");
        if (skipped.get() > 0)
            failed.put("excursion", skipped.get() + " of " + n + " tasks skipped the excursion: memory headroom below "
                + guard + " bytes");
        if (failed.containsKey("excursion")) failed.put("excursion_after_gc", failed.get("excursion"));
        Thread.sleep(settleMs);
        Mem exc = snapshot();
        gc();
        Mem excGc = snapshot();
        long heapExc = heapUsed();
        gate1.countDown();
        for (Thread t : ts) t.join();
        metric("tasks_created", n, "count");
        Object[][] phases = {{"shallow", shallow, heapShallow}, {"excursion", exc, heapExc}, {"excursion_after_gc", excGc, heapExc}};
        for (Object[] ph : phases) {
            String name = (String) ph[0];
            Mem s = (Mem) ph[1];
            if (notRun.containsKey(name)) {
                row("total_per_task_" + name, null, "", "skipped", notRun.get(name));
                continue;
            }
            if (failed.containsKey(name)) {
                row("total_per_task_" + name, null, "", "failed", failed.get(name));
                continue;
            }
            metric("rss_per_task_" + name, perItem(base.rss(), s.rss(), n), "bytes");
            metric("pte_per_task_" + name, perItem(base.pte(), s.pte(), n), "bytes");
            metric("total_per_task_" + name, perItem(base.rss() + base.pte(), s.rss() + s.pte(), n), "bytes");
            metric("java_heap_used_per_task_" + name, perItem(heap0, (Long) ph[2], n), "bytes");
            metric("rss_total_" + name, s.rss(), "bytes");
        }
    }

    static long memAvailable() {
        try {
            for (String l : Files.readAllLines(Path.of("/proc/meminfo")))
                if (l.startsWith("MemAvailable:")) return Long.parseLong(l.trim().split("\\s+")[1]) * 1024;
        } catch (IOException e) { }
        return Long.MAX_VALUE;
    }

    /** A file of this process's cgroup (v1 in the controller's hierarchy, or v2), as taskbench reads it. */
    static String cgroupFile(String controller, String v1, String v2) {
        List<String> candidates = new ArrayList<>();
        try {
            for (String l : Files.readAllLines(Path.of("/proc/self/cgroup"))) {
                String[] parts = l.split(":", 3);
                if (parts.length != 3) continue;
                if (Arrays.asList(parts[1].split(",")).contains(controller)) {
                    candidates.add("/sys/fs/cgroup/" + controller + parts[2] + "/" + v1);
                    candidates.add("/sys/fs/cgroup/" + controller + "/" + v1);
                } else if (parts[1].isEmpty()) {
                    candidates.add("/sys/fs/cgroup" + parts[2] + "/" + v2);
                    candidates.add("/sys/fs/cgroup/" + v2);
                }
            }
        } catch (IOException e) { return null; }
        for (String f : candidates) {
            try { return Files.readString(Path.of(f)).trim(); } catch (IOException e) { }
        }
        return null;
    }

    /**
     * Bytes that can still be allocated: MemAvailable, lowered to the memory
     * cgroup's headroom and to the Java heap's headroom when those are smaller.
     */
    static long memHeadroom() {
        long h = memAvailable();
        try {
            long limit = Long.parseLong(cgroupFile("memory", "memory.limit_in_bytes", "memory.max"));
            if (limit < (1L << 60)) {
                long usage = Long.parseLong(cgroupFile("memory", "memory.usage_in_bytes", "memory.current"));
                String stat = cgroupFile("memory", "memory.stat", "memory.stat");
                for (String l : stat == null ? new String[0] : stat.split("\n")) {
                    String[] f = l.trim().split("\\s+");
                    if (f.length == 2 && (f[0].equals("total_inactive_file") || f[0].equals("inactive_file"))) {
                        usage -= Long.parseLong(f[1]);
                        break;
                    }
                }
                h = Math.min(h, limit - usage);
            }
        } catch (RuntimeException e) { }
        Runtime rt = Runtime.getRuntime();
        return Math.min(h, rt.maxMemory() - (rt.totalMemory() - rt.freeMemory()));
    }

    /**
     * Spawns parked virtual threads up to cap (or the memory guard). With holdMs, a
     * run that reached the cap keeps them parked that long and checks again: the
     * pre-registered scale row passes when all park and the process survives 60 s.
     */
    static void maxTasks(int cap, long guard, long holdMs) throws Exception {
        CountDownLatch gate = new CountDownLatch(1);
        AtomicLong ready = new AtomicLong();
        Mem base = snapshot();
        long t0 = System.nanoTime();
        int created = 0;
        String reason = "cap";
        List<Thread> ts = new ArrayList<>();
        try {
            while (created < cap) {
                if (created % 4096 == 0 && memHeadroom() < guard) { reason = "mem_guard"; break; }
                ts.add(Thread.ofVirtual().start(() -> {
                    sink += shallowWork(1);
                    ready.incrementAndGet();
                    try { gate.await(); } catch (InterruptedException e) { }
                }));
                created++;
            }
        } catch (OutOfMemoryError e) {
            reason = "memory";
        }
        double spawnSecs = (System.nanoTime() - t0) / 1e9;
        waitFor(ready, created);
        Mem s = snapshot();
        boolean held = holdMs > 0 && created == cap;
        Mem after = s;
        double heldSecs = 0;
        long parked = 0;
        if (held) {
            long h0 = System.nanoTime();
            Thread.sleep(holdMs);
            heldSecs = (System.nanoTime() - h0) / 1e9;
            parked = ready.get();
            after = snapshot();
        }
        gate.countDown();
        for (Thread t : ts) t.join();
        row("max_tasks", (double) created, "count", "ok", "limit=" + reason);
        row("limit", null, "", "ok", reason);
        metric("rss_at_stop", s.rss(), "bytes");
        metric("pte_at_stop", s.pte(), "bytes");
        metric("total_per_task_at_stop", perItem(base.rss() + base.pte(), s.rss() + s.pte(), created), "bytes");
        metric("spawn_rate", created / spawnSecs, "1/s");
        if (held) {
            metric("hold_secs", Math.rint(heldSecs * 1e3) / 1e3, "s");
            row("parked_after_hold", (double) parked, "count", parked == created ? "ok" : "failed",
                parked + " of " + created + " tasks still parked after the hold");
            metric("total_per_task_after_hold", perItem(base.rss() + base.pte(), after.rss() + after.pte(), created), "bytes");
        }
    }

    static final int PER_PORT = 20000;

    static void echo(int n, int msg, double durSecs, double warmSecs, boolean idle, long settleMs) throws Exception {
        int ports = Math.max(1, (n + PER_PORT - 1) / PER_PORT);
        List<ServerSocket> ls = new ArrayList<>();
        for (int i = 0; i < ports; i++) {
            ServerSocket ss = new ServerSocket();
            ss.bind(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 4096);
            ls.add(ss);
        }
        AtomicLong ready = new AtomicLong();
        Mem base = snapshot();
        for (int pi = 0; pi < ports; pi++) {
            ServerSocket ss = ls.get(pi);
            int q = n / ports + (pi < n % ports ? 1 : 0);
            Thread.ofVirtual().start(() -> {
                for (int i = 0; i < q; i++) {
                    try {
                        Socket c = ss.accept();
                        c.setTcpNoDelay(true);
                        Thread.ofVirtual().start(() -> {
                            byte[] buf = new byte[4096];
                            ready.incrementAndGet();
                            try (c) {
                                InputStream in = c.getInputStream();
                                OutputStream out = c.getOutputStream();
                                int k;
                                while ((k = in.read(buf)) > 0) out.write(buf, 0, k);
                            } catch (IOException e) { }
                        });
                    } catch (IOException e) { return; }
                }
            });
        }
        long t0 = System.nanoTime();
        List<Socket> conns = new ArrayList<>(n);
        for (int i = 0; i < n; i++) {
            Socket s = new Socket(InetAddress.getLoopbackAddress(), ls.get(i % ports).getLocalPort());
            s.setTcpNoDelay(true);
            conns.add(s);
        }
        metric("connect_time", (System.nanoTime() - t0) / 1e6, "ms");
        if (idle) {
            waitFor(ready, n);
            Thread.sleep(settleMs);
            Mem s = snapshot();
            metric("rss_per_connection", perItem(base.rss(), s.rss(), n), "bytes");
            metric("pte_per_connection", perItem(base.pte(), s.pte(), n), "bytes");
            metric("total_per_connection", perItem(base.rss() + base.pte(), s.rss() + s.pte(), n), "bytes");
            metric("rss_total", s.rss(), "bytes");
            for (Socket c : conns) c.close();
            return;
        }
        AtomicBoolean recording = new AtomicBoolean(), stop = new AtomicBoolean();
        ConcurrentLinkedQueue<long[]> results = new ConcurrentLinkedQueue<>();
        List<Thread> clients = new ArrayList<>();
        for (Socket c : conns) {
            clients.add(Thread.ofVirtual().start(() -> {
                byte[] out = new byte[msg], in = new byte[msg];
                Arrays.fill(out, (byte) 'x');
                long[] lat = new long[1 << 12];
                int k = 0;
                try (c) {
                    InputStream is = c.getInputStream();
                    OutputStream os = c.getOutputStream();
                    while (!stop.get()) {
                        long t = System.nanoTime();
                        os.write(out);
                        int got = 0;
                        while (got < msg) {
                            int r = is.read(in, got, msg - got);
                            if (r < 0) throw new EOFException();
                            got += r;
                        }
                        if (recording.get()) {
                            if (k == lat.length) lat = Arrays.copyOf(lat, k * 2);
                            lat[k++] = System.nanoTime() - t;
                        }
                    }
                } catch (IOException e) { }
                results.add(Arrays.copyOf(lat, k));
            }));
        }
        Thread.sleep((long) (warmSecs * 1000));
        recording.set(true);
        long w0 = System.nanoTime();
        Thread.sleep((long) (durSecs * 1000));
        recording.set(false);
        double secs = (System.nanoTime() - w0) / 1e9;
        stop.set(true);
        for (Thread t : clients) t.join();
        long total = 0;
        for (long[] l : results) total += l.length;
        long[] all = new long[(int) total];
        int at = 0;
        for (long[] l : results) { System.arraycopy(l, 0, all, at, l.length); at += l.length; }
        summary("echo_rtt", all);
        metric("echo_requests", total, "count");
        metric("echo_throughput", total / secs, "req/s");
    }

    static void starvation(int n, long k, double durSecs, double baseSecs, long intervalUs) throws Exception {
        long intervalNs = intervalUs * 1000;
        AtomicBoolean stop0 = new AtomicBoolean();
        CompletableFuture<long[]> base = new CompletableFuture<>();
        long spawned0 = System.nanoTime();
        Thread.ofVirtual().start(() -> base.complete(probe(stop0, intervalNs, spawned0)));
        Thread.sleep((long) (baseSecs * 1000));
        stop0.set(true);
        long[] b = base.get();
        AtomicBoolean stop = new AtomicBoolean();
        AtomicLong iters = new AtomicLong();
        long chunk = k == 0 ? 100_000 : k;
        long t0 = System.nanoTime();
        List<Thread> ts = new ArrayList<>();
        for (int i = 0; i < n; i++) {
            ts.add(Thread.ofVirtual().start(() -> {
                long x = 1, done = 0;
                while (!stop.get()) {
                    x = spin(x, chunk);
                    done += chunk;
                    if (k > 0) Thread.yield();
                }
                sink += x & 1;
                iters.addAndGet(done);
            }));
        }
        CompletableFuture<long[]> loaded = new CompletableFuture<>();
        long spawned = System.nanoTime();
        Thread.ofVirtual().start(() -> loaded.complete(probe(stop, intervalNs, spawned)));
        // A platform thread stops the run: with k == 0 the virtual threads never
        // yield and no virtual thread may get a carrier until they finish.
        Thread.ofPlatform().start(() -> {
            try { Thread.sleep((long) (durSecs * 1000)); } catch (InterruptedException e) { }
            stop.set(true);
        });
        long[] l = loaded.get();
        for (Thread t : ts) t.join();
        double secs = (System.nanoTime() - t0) / 1e9;
        summary("probe_baseline_late", b);
        summary("probe_loaded_late", l);
        metric("compute_iterations_per_sec", iters.get() / secs, "1/s");
    }

    /** First deadline: spawn time plus one interval; always at least one sample. */
    static long[] probe(AtomicBoolean stop, long intervalNs, long spawned) {
        long[] v = new long[1024];
        int k = 0;
        long deadline = spawned + intervalNs;
        while (true) {
            long rest = deadline - System.nanoTime();
            if (rest > 0) {
                try { Thread.sleep(Duration.ofNanos(rest)); } catch (InterruptedException e) { break; }
            }
            long now = System.nanoTime();
            if (k == v.length) v = Arrays.copyOf(v, k * 2);
            v[k++] = Math.max(0, now - deadline);
            if (stop.get()) break;
            deadline = now + intervalNs;
        }
        return Arrays.copyOf(v, k);
    }

    public static void main(String[] argv) throws Exception {
        if (argv.length < 1) {
            System.err.println("usage: TaskBench <spawn-join|idle-memory|max-tasks|echo|idle-connections|starvation> [--option value]...");
            System.exit(2);
        }
        bench = argv[0];
        Map<String, String> o = new HashMap<>();
        for (int i = 1; i < argv.length; i++) {
            String a = argv[i];
            if (!a.startsWith("--")) throw new IllegalArgumentException("unexpected " + a);
            String k = a.substring(2);
            if (k.contains("=")) { o.put(k.substring(0, k.indexOf('=')), k.substring(k.indexOf('=') + 1)); continue; }
            if (k.equals("quiet") || k.equals("skip-excursion")) { o.put(k, "true"); continue; }
            o.put(k, argv[++i]);
        }
        threads = Integer.parseInt(o.getOrDefault("threads", Integer.toString(Runtime.getRuntime().availableProcessors())));
        String par = System.getProperty("jdk.virtualThreadScheduler.parallelism", "default");
        if (!par.equals(Integer.toString(threads)))
            System.err.println("warning: jdk.virtualThreadScheduler.parallelism=" + par + " differs from --threads " + threads);
        machine = o.getOrDefault("machine-id", InetAddress.getLocalHost().getHostName());
        quiet = o.containsKey("quiet");
        runId = DateTimeFormatter.ofPattern("yyyyMMdd'T'HHmmss'Z'").format(LocalDateTime.now(ZoneOffset.UTC)) + "-" + ProcessHandle.current().pid();
        if (o.containsKey("csv")) {
            File f = new File(o.get("csv"));
            boolean empty = !f.exists() || f.length() == 0;
            csv = new PrintStream(new FileOutputStream(f, true));
            if (empty) csv.println(HEADER);
        }
        // The flags this run set; arguments injected through JAVA_TOOL_OPTIONS (proxy
        // settings and the like) are recorded once in controls-versions.txt instead.
        Set<String> injected = new HashSet<>(Arrays.asList(System.getenv().getOrDefault("JAVA_TOOL_OPTIONS", "").trim().split("\\s+")));
        List<String> flags = new ArrayList<>();
        for (String a : java.lang.management.ManagementFactory.getRuntimeMXBean().getInputArguments())
            if (!injected.contains(a)) flags.add(a.replace(';', ' '));
        String jvm = System.getProperty("java.vm.version") + " " + String.join(" ", flags);
        long t = Long.parseLong(o.getOrDefault("tasks", "0"));
        long settle = Long.parseLong(o.getOrDefault("settle-ms", "200"));
        System.out.printf("javacontrol %s | %s | parallelism=%s%n", bench, System.getProperty("java.version"), par);
        switch (bench) {
            case "spawn-join" -> {
                int rounds = t > 0 ? (int) t : Integer.parseInt(o.getOrDefault("rounds", "20000"));
                int warmup = Integer.parseInt(o.getOrDefault("warmup", "20000"));
                int batch = Integer.parseInt(o.getOrDefault("batch", "1"));
                tasks = (long) rounds * batch;
                params = "rounds=" + rounds + ";warmup=" + warmup + ";batch=" + batch + ";jvm=" + jvm;
                // The joiner is itself a virtual thread, as in the Go and Rust versions.
                Thread vt = Thread.ofVirtual().start(() -> {
                    try { spawnJoin(rounds, warmup, batch); } catch (Exception e) { throw new RuntimeException(e); }
                });
                vt.join();
            }
            case "idle-memory" -> {
                int n = t > 0 ? (int) t : 10000;
                int exc = Integer.parseInt(o.getOrDefault("excursion", "32768"));
                int levels = exc / 64;
                tasks = n;
                long guard = Long.parseLong(o.getOrDefault("mem-guard", Long.toString(1L << 30)));
                boolean skip = o.containsKey("skip-excursion");
                params = "excursion=" + exc + ";excursion_levels=" + levels + ";settle_ms=" + settle + ";mem_guard=" + guard
                    + (skip ? ";skip_excursion=true" : "") + ";jvm=" + jvm;
                idleMemory(n, levels, settle, guard, skip);
            }
            case "max-tasks" -> {
                int n = t > 0 ? (int) t : 100000;
                long guard = Long.parseLong(o.getOrDefault("mem-guard", Long.toString(1L << 30)));
                tasks = n;
                double hold = Double.parseDouble(o.getOrDefault("hold-secs", "0"));
                params = "cap=" + n + ";mem_guard=" + guard + (hold > 0 ? ";hold_s=" + fmt(hold) : "") + ";jvm=" + jvm;
                maxTasks(n, guard, (long) (hold * 1000));
            }
            case "echo", "idle-connections" -> {
                boolean idle = bench.equals("idle-connections");
                int n = t > 0 ? (int) t : Integer.parseInt(o.getOrDefault("connections", idle ? "5000" : "1000"));
                int msg = Integer.parseInt(o.getOrDefault("msg-bytes", "64"));
                double dur = Double.parseDouble(o.getOrDefault("duration", "3"));
                double warm = Double.parseDouble(o.getOrDefault("warmup-secs", "1"));
                tasks = n;
                params = idle ? "connections=" + n + ";jvm=" + jvm
                        : "connections=" + n + ";msg_bytes=" + msg + ";duration_s=" + dur + ";warmup_s=" + warm + ";jvm=" + jvm;
                echo(n, msg, dur, warm, idle, settle);
            }
            case "starvation" -> {
                int n = t > 0 ? (int) t : 2 * threads;
                long k = Long.parseLong(o.getOrDefault("yield-every", "0"));
                double dur = Double.parseDouble(o.getOrDefault("duration", "2"));
                double base = Double.parseDouble(o.getOrDefault("baseline-secs", "1"));
                long us = Long.parseLong(o.getOrDefault("probe-interval-us", "1000"));
                tasks = n;
                params = "yield_every=" + k + ";duration_s=" + dur + ";probe_interval_us=" + us + ";jvm=" + jvm;
                starvation(n, k, dur, base, us);
            }
            default -> { System.err.println("unknown benchmark " + bench); System.exit(2); }
        }
        if (csv != null) csv.close();
        System.exit(0);
    }
}
