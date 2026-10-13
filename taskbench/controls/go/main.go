// Command gocontrol runs the Go control workloads of the bet-3 stack harness
// (taskbench): spawn-join, idle-memory, echo, idle-connections, starvation and
// max-tasks, writing rows in the same CSV schema as the Rust harness.
//
// Go is the baseline the bet-3 thresholds are stated against: memory per idle
// task within 4x of Go, spawn-plus-join median within 3x, echo p99 within 2x,
// and probe latency under compute load no worse than Go's asynchronous
// preemption. Run it with GOMAXPROCS equal to the harness's --threads (the flag
// -threads sets it).
//
// Usage: gocontrol <benchmark> [-threads N] [-tasks N] [-csv PATH] [-machine-id ID] ...
package main

import (
	"bufio"
	"flag"
	"fmt"
	"io"
	"math"
	"net"
	"os"
	"runtime"
	"runtime/debug"
	"sort"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

// header is the shared CSV schema (taskbench src/csv.rs HEADER).
const header = "schema,run_id,timestamp_utc,machine_id,implementation,benchmark,strategy,reservation_bytes,stack_bytes,threads,tasks,params,metric,value,unit,status,note"

type ctx struct {
	out     *os.File
	runID   string
	machine string
	bench   string
	threads int
	tasks   int64
	params  string
	quiet   bool
}

func escape(s string) string {
	if strings.ContainsAny(s, ",\"\n\r") {
		return "\"" + strings.ReplaceAll(s, "\"", "\"\"") + "\""
	}
	return s
}

func fmtValue(v float64) string {
	if math.IsNaN(v) || math.IsInf(v, 0) {
		return ""
	}
	if v == math.Trunc(v) && math.Abs(v) < 1e15 {
		return strconv.FormatInt(int64(v), 10)
	}
	return strconv.FormatFloat(v, 'f', 3, 64)
}

func (c *ctx) row(metric string, value *float64, unit, status, note string) {
	v := ""
	if value != nil {
		v = fmtValue(*value)
	}
	if !c.quiet {
		extra := ""
		if status != "ok" {
			extra = " [" + status + "]"
		}
		if note != "" {
			extra += "  (" + note + ")"
		}
		fmt.Printf("  %-34s %14s %-6s%s\n", metric, v, unit, extra)
	}
	if c.out == nil {
		return
	}
	fields := []string{
		"1", escape(c.runID), time.Now().UTC().Format("2006-01-02T15:04:05Z"), escape(c.machine),
		"go", escape(c.bench), "go", "", "", strconv.Itoa(c.threads), strconv.FormatInt(c.tasks, 10),
		escape(c.params), escape(metric), v, escape(unit), status, escape(note),
	}
	fmt.Fprintln(c.out, strings.Join(fields, ","))
}

func (c *ctx) metric(metric string, v float64, unit string) { c.row(metric, &v, unit, "ok", "") }

// percentile is nearest-rank on sorted samples, as in taskbench src/stats.rs.
func percentile(sorted []int64, p float64) int64 {
	if len(sorted) == 0 {
		return 0
	}
	rank := int(math.Ceil(p / 100 * float64(len(sorted))))
	if rank < 1 {
		rank = 1
	}
	if rank > len(sorted) {
		rank = len(sorted)
	}
	return sorted[rank-1]
}

func (c *ctx) summary(prefix string, s []int64) {
	sort.Slice(s, func(i, j int) bool { return s[i] < s[j] })
	var sum float64
	for _, v := range s {
		sum += float64(v)
	}
	mean := 0.0
	if len(s) > 0 {
		mean = math.Round(sum / float64(len(s)))
	}
	maxv := int64(0)
	if len(s) > 0 {
		maxv = s[len(s)-1]
	}
	for _, kv := range []struct {
		n string
		v float64
	}{{"p50", float64(percentile(s, 50))}, {"p90", float64(percentile(s, 90))}, {"p99", float64(percentile(s, 99))},
		{"p999", float64(percentile(s, 99.9))}, {"max", float64(maxv)}, {"mean", mean}} {
		c.metric(prefix+"_"+kv.n, kv.v, "ns")
	}
	c.metric(prefix+"_samples", float64(len(s)), "count")
}

// memSnapshot reads VmRSS and VmPTE (bytes) from /proc/self/status.
type memSnapshot struct{ rss, pte int64 }

func snapshot() memSnapshot {
	f, err := os.Open("/proc/self/status")
	if err != nil {
		return memSnapshot{}
	}
	defer f.Close()
	var m memSnapshot
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		fields := strings.Fields(sc.Text())
		if len(fields) < 2 {
			continue
		}
		v, _ := strconv.ParseInt(fields[1], 10, 64)
		switch fields[0] {
		case "VmRSS:":
			m.rss = v * 1024
		case "VmPTE:":
			m.pte = v * 1024
		}
	}
	return m
}

func perItem(before, after, n int64) float64 {
	if n == 0 {
		return 0
	}
	return math.Round(float64(after-before) / float64(n))
}

//go:noinline
func shallowWork(seed int64) int64 {
	var buf [192]byte
	for i := range buf {
		buf[i] = byte(seed)
	}
	return shallowInner(seed^int64(buf[7]), 3)
}

//go:noinline
func shallowInner(x int64, depth int) int64 {
	var buf [128]byte
	buf[5] = byte(x)
	if depth == 0 {
		return int64(buf[5]) + x
	}
	return shallowInner(x*31^int64(buf[3]), depth-1)
}

// excursion uses about `bytes` of stack below the caller (4000-byte frames, as
// in taskbench's work::excursion), which makes Go grow (copy) the stack.
//
//go:noinline
func excursion(bytes int) int {
	var page [4000]byte
	for i := 0; i < len(page); i += 512 {
		page[i] = byte(i)
	}
	if bytes > 4096 {
		return excursion(bytes-4096) + int(page[512])
	}
	return int(page[512])
}

//go:noinline
func spinIterations(x uint64, n uint64) uint64 {
	for i := uint64(0); i < n; i++ {
		x = x*6364136223846793005 + 1442695040888963407
		x ^= x >> 29
	}
	return x
}

var sink atomic.Int64

func spawnJoin(c *ctx, rounds, warmup, batch int) {
	samples := make([]int64, 0, rounds)
	done := make(chan int, batch)
	for r := 0; r < warmup+rounds; r++ {
		t0 := time.Now()
		for i := 0; i < batch; i++ {
			go func(v int) { done <- v }(r ^ i)
		}
		for i := 0; i < batch; i++ {
			sink.Add(int64(<-done))
		}
		dt := time.Since(t0).Nanoseconds() / int64(batch)
		if r >= warmup {
			samples = append(samples, dt)
		}
	}
	c.summary("spawn_join", samples)
}

// idleMemory mirrors taskbench's idle-memory, including its memory guard: once
// the headroom is below guard during the excursion phase, goroutines that have
// not made their excursion yet skip it and the phase is reported as failed. With
// skip (the harness's --skip-excursion), the excursion phase is not run and its
// cells are reported as skipped: at 1M goroutines it needs about 32 GiB before a
// garbage collection could shrink the stacks (the owner's policy 7).
//
// The goroutines check the headroom themselves (every 256th, before its
// excursion). A watcher goroutine does not work: closing gate0 readies all n
// goroutines, the closing goroutine is preempted while it does so and requeued
// behind them in the global run queue, and at 1M tasks it next ran only after
// every excursion had been made (or the OOM killer had struck).
func idleMemory(c *ctx, n int, exc int, settle time.Duration, guard int64, skip bool) {
	doExc := exc > 0 && !skip
	var ready, skipped, arrived atomic.Int64
	var stop atomic.Bool
	gate0 := make(chan struct{})
	gate1 := make(chan struct{})
	var wg sync.WaitGroup
	runtime.GC()
	debug.FreeOSMemory()
	base := snapshot()
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			sink.Add(shallowWork(int64(i)))
			ready.Add(1)
			<-gate0
			if doExc && arrived.Add(1)%256 == 0 && !stop.Load() {
				if h := memHeadroom(); h < guard && stop.CompareAndSwap(false, true) {
					fmt.Fprintf(os.Stderr, "idle-memory: headroom %d below the guard %d; the remaining goroutines skip the excursion\n", h, guard)
				}
			}
			if doExc && stop.Load() {
				skipped.Add(1)
			} else if doExc {
				sink.Add(int64(excursion(exc)))
			}
			ready.Add(1)
			<-gate1
		}(i)
	}
	// waitFor reports whether target was reached before the deadline.
	waitFor := func(target int64) bool {
		deadline := time.Now().Add(60*time.Second + time.Duration(n)*50*time.Microsecond)
		for ready.Load() < target && time.Now().Before(deadline) {
			time.Sleep(2 * time.Millisecond)
		}
		return ready.Load() >= target
	}
	failed := map[string]string{}
	notRun := map[string]string{}
	if skip && exc > 0 {
		why := fmt.Sprintf("not run (--skip-excursion): the %d-byte excursion phase is required only of strategies that release pages on park (owner's policy 7)", exc)
		notRun["excursion"], notRun["excursion_after_gc"] = why, why
	}
	if !waitFor(int64(n)) {
		failed["shallow"] = "goroutines did not all park before the deadline"
	}
	time.Sleep(settle)
	shallow := snapshot()
	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)
	stackShallow := ms.StackInuse
	close(gate0)
	if !waitFor(2 * int64(n)) {
		failed["excursion"] = "goroutines did not all park again after the excursion before the deadline"
	}
	if skipped.Load() > 0 {
		failed["excursion"] = fmt.Sprintf("%d of %d tasks skipped the excursion: memory headroom below %d bytes",
			skipped.Load(), n, guard)
	}
	failed["excursion_after_gc"] = failed["excursion"]
	time.Sleep(settle)
	exSnap := snapshot()
	runtime.ReadMemStats(&ms)
	stackExc := ms.StackInuse
	// Stacks shrink during garbage collection (to half when less than a quarter
	// is in use); a running Go program collects periodically. Not after the
	// guard stopped the excursions: shrinking allocates the smaller stacks
	// before the large ones are freed, and there is no headroom left for that.
	gcSnap, stackGC := exSnap, stackExc
	if !stop.Load() && doExc {
		for i := 0; i < 4; i++ {
			runtime.GC()
		}
		debug.FreeOSMemory()
		time.Sleep(settle)
		gcSnap = snapshot()
		runtime.ReadMemStats(&ms)
		stackGC = ms.StackInuse
	}
	close(gate1)
	wg.Wait()
	nn := int64(n)
	c.metric("tasks_created", float64(n), "count")
	for _, ph := range []struct {
		name  string
		s     memSnapshot
		stack uint64
	}{{"shallow", shallow, stackShallow}, {"excursion", exSnap, stackExc}, {"excursion_after_gc", gcSnap, stackGC}} {
		if why := notRun[ph.name]; why != "" {
			c.row("total_per_task_"+ph.name, nil, "", "skipped", why)
			continue
		}
		if why := failed[ph.name]; why != "" {
			c.row("total_per_task_"+ph.name, nil, "", "failed", why)
			continue
		}
		c.metric("rss_per_task_"+ph.name, perItem(base.rss, ph.s.rss, nn), "bytes")
		c.metric("pte_per_task_"+ph.name, perItem(base.pte, ph.s.pte, nn), "bytes")
		c.metric("total_per_task_"+ph.name, perItem(base.rss+base.pte, ph.s.rss+ph.s.pte, nn), "bytes")
		c.metric("go_stack_inuse_per_task_"+ph.name, math.Round(float64(ph.stack)/float64(n)), "bytes")
		c.metric("rss_total_"+ph.name, float64(ph.s.rss), "bytes")
	}
}

// maxTasks spawns parked goroutines up to limit (or the memory guard). With hold,
// a run that reached the cap keeps them parked that long and checks again: the
// pre-registered scale row passes when all park and the process survives 60 s.
func maxTasks(c *ctx, limit int, guard int64, hold time.Duration) {
	gate := make(chan struct{})
	var ready atomic.Int64
	base := snapshot()
	t0 := time.Now()
	created := 0
	reason := "cap"
	for created < limit {
		if created%4096 == 0 && memHeadroom() < guard {
			reason = "mem_guard"
			break
		}
		go func() {
			sink.Add(shallowWork(1))
			ready.Add(1)
			<-gate
		}()
		created++
	}
	spawnSecs := time.Since(t0).Seconds()
	for ready.Load() < int64(created) {
		time.Sleep(5 * time.Millisecond)
	}
	s := snapshot()
	held := hold > 0 && created == limit
	var after memSnapshot
	var heldSecs float64
	var parked int64
	if held {
		t := time.Now()
		time.Sleep(hold)
		heldSecs = time.Since(t).Seconds()
		parked = ready.Load()
		after = snapshot()
	}
	close(gate)
	c.row("max_tasks", ptr(float64(created)), "count", "ok", "limit="+reason)
	c.row("limit", nil, "", "ok", reason)
	c.metric("rss_at_stop", float64(s.rss), "bytes")
	c.metric("pte_at_stop", float64(s.pte), "bytes")
	c.metric("total_per_task_at_stop", perItem(base.rss+base.pte, s.rss+s.pte, int64(created)), "bytes")
	c.metric("spawn_rate", float64(created)/spawnSecs, "1/s")
	if held {
		c.metric("hold_secs", math.Round(heldSecs*1e3)/1e3, "s")
		status := "ok"
		if parked != int64(created) {
			status = "failed"
		}
		c.row("parked_after_hold", ptr(float64(parked)), "count", status,
			fmt.Sprintf("%d of %d tasks still parked after the hold", parked, created))
		c.metric("total_per_task_after_hold", perItem(base.rss+base.pte, after.rss+after.pte, int64(created)), "bytes")
	}
}

func ptr(v float64) *float64 { return &v }

func memAvailable() int64 {
	b, err := os.ReadFile("/proc/meminfo")
	if err != nil {
		return math.MaxInt64
	}
	for _, l := range strings.Split(string(b), "\n") {
		if strings.HasPrefix(l, "MemAvailable:") {
			f := strings.Fields(l)
			v, _ := strconv.ParseInt(f[1], 10, 64)
			return v * 1024
		}
	}
	return math.MaxInt64
}

// cgroupFile reads a file of this process's cgroup (v1 file in the controller's
// hierarchy, or v2 file), falling back to the hierarchy's root, as taskbench's
// mem::cgroup_file does.
func cgroupFile(controller, v1, v2 string) (string, bool) {
	own, err := os.ReadFile("/proc/self/cgroup")
	if err != nil {
		return "", false
	}
	var candidates []string
	for _, l := range strings.Split(strings.TrimSpace(string(own)), "\n") {
		parts := strings.SplitN(l, ":", 3)
		if len(parts) != 3 {
			continue
		}
		isV1 := false
		for _, ctl := range strings.Split(parts[1], ",") {
			isV1 = isV1 || ctl == controller
		}
		if isV1 {
			candidates = append(candidates, "/sys/fs/cgroup/"+controller+parts[2]+"/"+v1, "/sys/fs/cgroup/"+controller+"/"+v1)
		} else if parts[1] == "" {
			candidates = append(candidates, "/sys/fs/cgroup"+parts[2]+"/"+v2, "/sys/fs/cgroup/"+v2)
		}
	}
	for _, f := range candidates {
		if b, err := os.ReadFile(f); err == nil {
			return strings.TrimSpace(string(b)), true
		}
	}
	return "", false
}

// memHeadroom is MemAvailable, lowered to the memory cgroup's limit minus its
// usage (without inactive file pages) when that is smaller.
func memHeadroom() int64 {
	h := memAvailable()
	limitText, ok := cgroupFile("memory", "memory.limit_in_bytes", "memory.max")
	limit, err := strconv.ParseInt(limitText, 10, 64)
	if !ok || err != nil || limit >= 1<<60 {
		return h
	}
	usageText, _ := cgroupFile("memory", "memory.usage_in_bytes", "memory.current")
	usage, _ := strconv.ParseInt(usageText, 10, 64)
	stat, _ := cgroupFile("memory", "memory.stat", "memory.stat")
	for _, l := range strings.Split(stat, "\n") {
		f := strings.Fields(l)
		if len(f) == 2 && (f[0] == "total_inactive_file" || f[0] == "inactive_file") {
			inactive, _ := strconv.ParseInt(f[1], 10, 64)
			usage -= inactive
			break
		}
	}
	if limit-usage < h {
		return limit - usage
	}
	return h
}

const perPort = 20000

func listeners(n int) ([]net.Listener, error) {
	ports := (n + perPort - 1) / perPort
	if ports < 1 {
		ports = 1
	}
	var ls []net.Listener
	for i := 0; i < ports; i++ {
		l, err := net.Listen("tcp", "127.0.0.1:0")
		if err != nil {
			return nil, err
		}
		ls = append(ls, l)
	}
	return ls, nil
}

func serve(c net.Conn, ready *atomic.Int64) {
	defer c.Close()
	buf := make([]byte, 4096)
	ready.Add(1)
	for {
		n, err := c.Read(buf)
		if err != nil {
			return
		}
		if _, err := c.Write(buf[:n]); err != nil {
			return
		}
	}
}

func echo(c *ctx, n, msg int, duration, warmup time.Duration, idle bool, settle time.Duration) {
	ls, err := listeners(n)
	if err != nil {
		c.row("echo", nil, "", "failed", "listen: "+err.Error())
		return
	}
	var ready atomic.Int64
	base := snapshot()
	for pi, l := range ls {
		q := n/len(ls) + btoi(pi < n%len(ls))
		go func(l net.Listener, q int) {
			for i := 0; i < q; i++ {
				conn, err := l.Accept()
				if err != nil {
					return
				}
				go serve(conn, &ready)
			}
		}(l, q)
	}
	t0 := time.Now()
	conns := make([]net.Conn, 0, n)
	for i := 0; i < n; i++ {
		conn, err := net.Dial("tcp", ls[i%len(ls)].Addr().String())
		if err != nil {
			c.row("echo", nil, "", "failed", "dial: "+err.Error())
			return
		}
		conns = append(conns, conn)
	}
	c.metric("connect_time", time.Since(t0).Seconds()*1e3, "ms")
	if idle {
		deadline := time.Now().Add(60 * time.Second)
		for ready.Load() < int64(n) && time.Now().Before(deadline) {
			time.Sleep(2 * time.Millisecond)
		}
		time.Sleep(settle)
		s := snapshot()
		nn := int64(n)
		c.metric("rss_per_connection", perItem(base.rss, s.rss, nn), "bytes")
		c.metric("pte_per_connection", perItem(base.pte, s.pte, nn), "bytes")
		c.metric("total_per_connection", perItem(base.rss+base.pte, s.rss+s.pte, nn), "bytes")
		c.metric("rss_total", float64(s.rss), "bytes")
		for _, conn := range conns {
			conn.Close()
		}
		return
	}
	var recording, stop atomic.Bool
	results := make(chan []int64, n)
	for _, conn := range conns {
		go func(conn net.Conn) {
			out := make([]byte, msg)
			for i := range out {
				out[i] = 'x'
			}
			in := make([]byte, msg)
			var lat []int64
			for !stop.Load() {
				t := time.Now()
				if _, err := conn.Write(out); err != nil {
					break
				}
				if _, err := io.ReadFull(conn, in); err != nil {
					break
				}
				if recording.Load() {
					lat = append(lat, time.Since(t).Nanoseconds())
				}
			}
			conn.Close()
			results <- lat
		}(conn)
	}
	time.Sleep(warmup)
	recording.Store(true)
	w0 := time.Now()
	time.Sleep(duration)
	recording.Store(false)
	secs := time.Since(w0).Seconds()
	stop.Store(true)
	var all []int64
	for i := 0; i < n; i++ {
		all = append(all, <-results...)
	}
	reqs := float64(len(all))
	c.summary("echo_rtt", all)
	c.metric("echo_requests", reqs, "count")
	c.metric("echo_throughput", reqs/secs, "req/s")
}

func btoi(b bool) int {
	if b {
		return 1
	}
	return 0
}

func starvation(c *ctx, n int, k uint64, duration, baseline, interval time.Duration) {
	// The first deadline is the spawn time plus one interval, and at least one
	// sample is recorded, so a probe that cannot start reports how late it was.
	probe := func(stop *atomic.Bool, spawned time.Time, out chan<- []int64) {
		var v []int64
		deadline := spawned.Add(interval)
		for {
			if d := time.Until(deadline); d > 0 {
				time.Sleep(d)
			}
			now := time.Now()
			late := now.Sub(deadline).Nanoseconds()
			if late < 0 {
				late = 0
			}
			v = append(v, late)
			if stop.Load() {
				break
			}
			deadline = now.Add(interval)
		}
		out <- v
	}
	var stop0 atomic.Bool
	ch := make(chan []int64, 1)
	go probe(&stop0, time.Now(), ch)
	time.Sleep(baseline)
	stop0.Store(true)
	base := <-ch
	var stop atomic.Bool
	var iters atomic.Uint64
	var wg sync.WaitGroup
	chunk := k
	if chunk == 0 {
		chunk = 100000
	}
	t0 := time.Now()
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			x, done := uint64(1), uint64(0)
			for !stop.Load() {
				x = spinIterations(x, chunk)
				done += chunk
				if k > 0 {
					runtime.Gosched()
				}
			}
			sink.Add(int64(x & 1))
			iters.Add(done)
		}()
	}
	go probe(&stop, time.Now(), ch)
	// Stop from a timer: with yield-every 0 the compute goroutines never yield,
	// and only asynchronous preemption lets the probe and this goroutine run.
	time.AfterFunc(duration, func() { stop.Store(true) })
	loaded := <-ch
	wg.Wait()
	secs := time.Since(t0).Seconds()
	c.summary("probe_baseline_late", base)
	c.summary("probe_loaded_late", loaded)
	c.metric("compute_iterations_per_sec", float64(iters.Load())/secs, "1/s")
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: gocontrol <spawn-join|idle-memory|max-tasks|echo|idle-connections|starvation> [flags]")
		os.Exit(2)
	}
	bench := os.Args[1]
	fs := flag.NewFlagSet(bench, flag.ExitOnError)
	threads := fs.Int("threads", runtime.NumCPU(), "GOMAXPROCS")
	tasks := fs.Int64("tasks", 0, "scale (meaning per benchmark)")
	csvPath := fs.String("csv", "", "append rows to this CSV file")
	machine := fs.String("machine-id", "", "machine column (default: host name)")
	rounds := fs.Int("rounds", 20000, "spawn-join rounds")
	warmupRounds := fs.Int("warmup", 2000, "spawn-join warm-up rounds")
	batch := fs.Int("batch", 1, "spawn-join tasks per round")
	exc := fs.Int("excursion", 32768, "idle-memory stack excursion (bytes)")
	settleMs := fs.Int("settle-ms", 200, "settle time before memory readings")
	guard := fs.Int64("mem-guard", 1<<30, "max-tasks and idle-memory: stop when the memory headroom drops below (bytes)")
	connections := fs.Int("connections", 0, "echo connections")
	msg := fs.Int("msg-bytes", 64, "echo message size")
	duration := fs.Float64("duration", 0, "recorded seconds (echo 3, starvation 2)")
	warmupSecs := fs.Float64("warmup-secs", 1, "echo warm-up seconds")
	yieldEvery := fs.Uint64("yield-every", 0, "starvation: iterations between runtime.Gosched (0: rely on async preemption)")
	probeUs := fs.Int("probe-interval-us", 1000, "starvation probe sleep")
	baselineSecs := fs.Float64("baseline-secs", 1, "starvation probe-alone seconds")
	quiet := fs.Bool("quiet", false, "no per-metric output")
	skipExc := fs.Bool("skip-excursion", false, "idle-memory: measure the shallow phase only; report the excursion cells as skipped")
	holdSecs := fs.Float64("hold-secs", 0, "max-tasks: keep a run that reached the cap parked this long, then check again")
	fs.Parse(os.Args[2:])
	runtime.GOMAXPROCS(*threads)
	if *machine == "" {
		h, _ := os.Hostname()
		*machine = h
	}
	c := &ctx{machine: *machine, bench: bench, threads: *threads, quiet: *quiet,
		runID: fmt.Sprintf("%s-%d", time.Now().UTC().Format("20060102T150405Z"), os.Getpid())}
	if *csvPath != "" {
		f, err := os.OpenFile(*csvPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		defer f.Close()
		if st, _ := f.Stat(); st.Size() == 0 {
			fmt.Fprintln(f, header)
		}
		c.out = f
	}
	settle := time.Duration(*settleMs) * time.Millisecond
	secs := func(v, def float64) time.Duration {
		if v == 0 {
			v = def
		}
		return time.Duration(v * float64(time.Second))
	}
	fmt.Printf("gocontrol %s | %s GOMAXPROCS=%d\n", bench, runtime.Version(), *threads)
	switch bench {
	case "spawn-join":
		r := *rounds
		if *tasks > 0 {
			r = int(*tasks)
		}
		c.tasks = int64(r * *batch)
		c.params = fmt.Sprintf("rounds=%d;warmup=%d;batch=%d;go=%s", r, *warmupRounds, *batch, runtime.Version())
		spawnJoin(c, r, *warmupRounds, *batch)
	case "idle-memory":
		n := 10000
		if *tasks > 0 {
			n = int(*tasks)
		}
		c.tasks = int64(n)
		c.params = fmt.Sprintf("excursion=%d;settle_ms=%d;mem_guard=%d;go=%s", *exc, *settleMs, *guard, runtime.Version())
		if *skipExc {
			c.params += ";skip_excursion=true"
		}
		idleMemory(c, n, *exc, settle, *guard, *skipExc)
	case "max-tasks":
		n := 100000
		if *tasks > 0 {
			n = int(*tasks)
		}
		c.tasks = int64(n)
		c.params = fmt.Sprintf("cap=%d;mem_guard=%d;go=%s", n, *guard, runtime.Version())
		if *holdSecs > 0 {
			c.params += fmt.Sprintf(";hold_s=%g", *holdSecs)
		}
		maxTasks(c, n, *guard, time.Duration(*holdSecs*float64(time.Second)))
	case "echo", "idle-connections":
		n := *connections
		if *tasks > 0 {
			n = int(*tasks)
		}
		if n == 0 {
			n = 1000
			if bench == "idle-connections" {
				n = 5000
			}
		}
		c.tasks = int64(n)
		if bench == "echo" {
			c.params = fmt.Sprintf("connections=%d;msg_bytes=%d;duration_s=%g;warmup_s=%g;go=%s", n, *msg, secs(*duration, 3).Seconds(), *warmupSecs, runtime.Version())
		} else {
			c.params = fmt.Sprintf("connections=%d;go=%s", n, runtime.Version())
		}
		echo(c, n, *msg, secs(*duration, 3), secs(*warmupSecs, 1), bench == "idle-connections", settle)
	case "starvation":
		n := 2 * *threads
		if *tasks > 0 {
			n = int(*tasks)
		}
		c.tasks = int64(n)
		c.params = fmt.Sprintf("yield_every=%d;duration_s=%g;probe_interval_us=%d;go=%s", *yieldEvery, secs(*duration, 2).Seconds(), *probeUs, runtime.Version())
		starvation(c, n, *yieldEvery, secs(*duration, 2), secs(*baselineSecs, 1), time.Duration(*probeUs)*time.Microsecond)
	default:
		fmt.Fprintln(os.Stderr, "unknown benchmark", bench)
		os.Exit(2)
	}
}
