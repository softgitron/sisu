//! The driver's server of stage 1 for every command (decision 980, after
//! decision 942's design): a process of this executable, `sisu
//! __stage1-serve SOCKET SISU1`, that keeps processes of `sisu1 sir
//! --repl-serve` (stage 1's R158, `sisu1 0.13.0` and later) and answers
//! the commands they serve over a Unix socket under `target/stage1/serve/`,
//! so that a command pays its own check and not the standard library's
//! base.
//!
//! A command that stage 1 serves (`sir`, `check`, `parse`, `fmt`, `lint`,
//! `doc`, `tokens`) sends its request, R158's line of the command's words
//! separated by tabs with `--dir` and the working directory, and reads the
//! answer, R158's frame, `answer STATUS OUT ERR` and the bytes; the server
//! hands it to one of its processes of stage 1, up to one for each core,
//! started when every one is busy. The first command that finds no server
//! starts one under a lock beside the socket, so that one starts. The
//! server ends after [`IDLE`] without a request and when this executable
//! or the `sisu1` it serves is built again, answering `stale` to the
//! request that finds it so, which the command then sends to a new server
//! of the new `sisu1` (decision 1362: the stage 1 that `SISU_STAGE1`
//! names is checked by nothing else); a process of stage 1 that answers
//! `stale` (`sisu1` was built again) is replaced and the request sent
//! again. Whatever goes wrong, the command
//! runs stage 1 as a process of its own, as it does under
//! `SISU_STAGE1_SERVE=0`.

use std::io::{BufRead as _, Read as _, Write as _};
use std::path::{Path, PathBuf};

/// `SISU_STAGE1_SERVE=0`: every command runs stage 1 as a process of its
/// own, whatever its version says (decision 980).
pub const SERVE_VAR: &str = "SISU_STAGE1_SERVE";

/// `SISU_STAGE1_SERVE_IDLE=SECONDS`: how long the server waits for a
/// request before it ends, [`IDLE`] without it.
pub const IDLE_VAR: &str = "SISU_STAGE1_SERVE_IDLE";

/// How long the server waits for a request before it ends.
const IDLE: std::time::Duration = std::time::Duration::from_secs(300);

/// The command of this executable that is the server.
pub const COMMAND: &str = "__stage1-serve";

/// Whether `sisu1 sir --repl-serve` serves the command `words` (stage 1's
/// R158): its first word is one of the commands it runs.
pub fn served(words: &[String]) -> bool {
    matches!(
        words.first().map(String::as_str),
        Some("sir" | "check" | "parse" | "fmt" | "lint" | "doc" | "tokens")
    ) && !words.iter().any(|w| w == "--repl" || w == "--worksheet" || w == "--repl-serve")
}

/// The size and time of a file, by which the server tells that it was
/// built again.
fn stamp(path: &Path) -> Option<(u64, u128)> {
    let meta = std::fs::metadata(path).ok()?;
    let time = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some((meta.len(), time.as_nanos()))
}

/// This executable, when it is the `sisu` command (and not a test of the
/// crate, whose executable is another), which the server runs as.
fn sisu_exe() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?;
    let name = me.file_stem()?.to_str()?;
    (name == "sisu").then_some(me)
}

/// The socket of the server for this executable and the stage 1 `sisu1`:
/// `target/stage1/serve/HASH.sock` of the compiler's directory, its name
/// hashed from the two paths, so that a debug and a release driver, or a
/// stage 1 given by `SISU_STAGE1`, each have their own.
fn socket(compiler: &Path, me: &Path, sisu1: &Path) -> PathBuf {
    let key = format!("{}\n{}", me.display(), sisu1.display());
    let hash = sisu_interp::record::content_hash(key.as_bytes());
    compiler.join("target").join("stage1").join("serve").join(format!("{hash:016x}.sock"))
}

/// What the command `words` prints on its standard output and its standard
/// error and the status it gives, as `sisu1 sir --repl-serve` answers it
/// in the working directory, through the server, which is started when
/// there is none; none when it cannot be asked, and the command runs
/// stage 1 as a process of its own.
#[cfg(unix)]
pub fn ask(compiler: &Path, sisu1: &Path, words: &[String]) -> Option<(Vec<u8>, Vec<u8>, i32)> {
    use std::os::unix::net::UnixStream;
    let me = sisu_exe()?;
    let cwd = std::env::current_dir().ok()?;
    let cwd = cwd.to_str()?;
    if words.iter().map(String::as_str).chain([cwd]).any(|w| w.contains(['\t', '\n', '\r'])) {
        return None;
    }
    let mut line = words.join("\t");
    line.push_str("\t--dir\t");
    line.push_str(cwd);
    line.push('\n');
    let sock = socket(compiler, &me, sisu1);
    // A path a socket cannot have (108 bytes on Linux, with its end).
    if sock.as_os_str().len() >= 100 {
        return None;
    }
    // A server that ends as it is asked (idle, or this executable built
    // again) answers `stale` or nothing, and a new one is started.
    for _ in 0..3 {
        let stream = match UnixStream::connect(&sock) {
            Ok(s) => s,
            Err(_) => start(&sock, &me, sisu1, compiler)?,
        };
        match exchange(stream, &line) {
            Exchanged::Answer(a) => return Some(a),
            Exchanged::Again => continue,
            Exchanged::Lost => return None,
        }
    }
    None
}

#[cfg(not(unix))]
pub fn ask(_compiler: &Path, _sisu1: &Path, _words: &[String]) -> Option<(Vec<u8>, Vec<u8>, i32)> {
    None
}

/// What a request to the server came to.
enum Exchanged {
    /// Its answer: standard output, standard error and the status.
    Answer((Vec<u8>, Vec<u8>, i32)),
    /// The server was ending (`stale`, or no answer at all): ask a new one.
    Again,
    /// The server could not answer it (its stage 1 went away or answered
    /// out of its frame): run stage 1 as a process.
    Lost,
}

#[cfg(unix)]
fn exchange(mut stream: std::os::unix::net::UnixStream, line: &str) -> Exchanged {
    if stream.write_all(line.as_bytes()).and_then(|()| stream.flush()).is_err() {
        return Exchanged::Again;
    }
    let mut answers = std::io::BufReader::new(stream);
    let mut head = String::new();
    match answers.read_line(&mut head) {
        Ok(0) | Err(_) => return Exchanged::Again,
        Ok(_) => {}
    }
    if head == "stale\n" {
        return Exchanged::Again;
    }
    let Some((status, out, err)) = crate::front::answer_head(&head) else { return Exchanged::Lost };
    let mut stdout = vec![0; out];
    let mut stderr = vec![0; err];
    if answers.read_exact(&mut stdout).and_then(|()| answers.read_exact(&mut stderr)).is_err() {
        return Exchanged::Lost;
    }
    Exchanged::Answer((stdout, stderr, status))
}

/// The server started, under the lock beside its socket, and a connection
/// to it, or to the one another command started while this one waited
/// for the lock.
#[cfg(unix)]
fn start(sock: &Path, me: &Path, sisu1: &Path, compiler: &Path) -> Option<std::os::unix::net::UnixStream> {
    use std::os::unix::net::UnixStream;
    use std::os::unix::process::CommandExt as _;
    let dir = sock.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(sock.with_extension("lock"))
        .ok()?;
    lock.lock().ok()?;
    if let Ok(s) = UnixStream::connect(sock) {
        return Some(s);
    }
    // Its standard streams are none of the command's, so nothing that reads
    // what the command prints waits for it to end, and its own process
    // group, so that a signal to the command's is not one to it.
    let mut child = std::process::Command::new(me)
        .arg(COMMAND)
        .arg(sock)
        .arg(sisu1)
        .current_dir(compiler)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .ok()?;
    for _ in 0..1000 {
        if let Ok(s) = UnixStream::connect(sock) {
            return Some(s);
        }
        if child.try_wait().ok().flatten().is_some() {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    None
}

/// `sisu __stage1-serve SOCKET SISU1`: the server, until it is idle for
/// [`IDLE`] or this executable is built again.
#[cfg(unix)]
pub fn serve(args: &[String]) -> i32 {
    use std::os::unix::net::UnixListener;
    use std::sync::Arc;
    let [sock, sisu1] = args else { return sisu_interp::exit::ERROR };
    let sock = PathBuf::from(sock);
    let sisu1 = PathBuf::from(sisu1);
    let Some(me) = sisu_exe() else { return sisu_interp::exit::ERROR };
    let idle = std::env::var(IDLE_VAR)
        .ok()
        .and_then(|s| s.parse().ok())
        .map_or(IDLE, std::time::Duration::from_secs);
    // The command that started the server holds the lock, and no server
    // answered at the socket: what is there was left by one that was
    // stopped.
    let _ = std::fs::remove_file(&sock);
    let Ok(listener) = UnixListener::bind(&sock) else { return sisu_interp::exit::ERROR };
    let state = Arc::new(State {
        built: (stamp(&me), stamp(&sisu1)),
        stage1: crate::front::Stage1::at(sisu1.clone(), crate::front::compiler_dir()),
        sisu1,
        me,
        sock,
        most: std::thread::available_parallelism().map_or(2, |n| n.get()),
        pool: std::sync::Mutex::new(Pool { idle: Vec::new(), running: 0 }),
        freed: std::sync::Condvar::new(),
        last: std::sync::Mutex::new(std::time::Instant::now()),
        busy: std::sync::atomic::AtomicUsize::new(0),
        ending: std::sync::atomic::AtomicBool::new(false),
    });
    // The watch: the server ends when it was idle long enough, or this
    // executable or its stage 1 was built again, between requests.
    {
        let state = state.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let quiet = state.last.lock().map(|t| t.elapsed() >= idle).unwrap_or(true);
                let rebuilt = state.rebuilt();
                if (quiet || rebuilt || state.ending.load(std::sync::atomic::Ordering::SeqCst))
                    && state.busy.load(std::sync::atomic::Ordering::SeqCst) == 0
                {
                    state.end();
                    std::process::exit(sisu_interp::exit::OK);
                }
            }
        });
    }
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let state = state.clone();
        state.busy.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(move || {
            state.answer(stream);
            if let Ok(mut t) = state.last.lock() {
                *t = std::time::Instant::now();
            }
            state.busy.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        });
    }
    sisu_interp::exit::OK
}

#[cfg(not(unix))]
pub fn serve(_args: &[String]) -> i32 {
    sisu_interp::exit::ERROR
}

/// What the server holds.
#[cfg(unix)]
struct State {
    stage1: crate::front::Stage1,
    me: PathBuf,
    /// The stage 1 it serves.
    sisu1: PathBuf,
    /// This executable's stamp and its stage 1's when the server started.
    built: (Option<(u64, u128)>, Option<(u64, u128)>),
    sock: PathBuf,
    /// How many processes of stage 1 it keeps at most: one for each core.
    most: usize,
    pool: std::sync::Mutex<Pool>,
    /// A process of stage 1 went back to the pool, or ended.
    freed: std::sync::Condvar,
    /// When the last request was answered.
    last: std::sync::Mutex<std::time::Instant>,
    /// How many requests are being answered.
    busy: std::sync::atomic::AtomicUsize,
    /// The server is ending: every request is answered `stale`.
    ending: std::sync::atomic::AtomicBool,
}

/// The processes of stage 1 the server keeps: those not answering a
/// request, and how many there are in all.
#[cfg(unix)]
struct Pool {
    idle: Vec<Served>,
    running: usize,
}

/// One process of `sisu1 sir --repl-serve`.
#[cfg(unix)]
struct Served {
    child: std::process::Child,
    requests: std::process::ChildStdin,
    answers: std::io::BufReader<std::process::ChildStderr>,
}

/// What a process of stage 1 made of a request.
#[cfg(unix)]
enum Reply {
    /// The frame of its answer, as it wrote it.
    Frame(Vec<u8>),
    /// `stale`: it ended, since `sisu1` was built again.
    Stale,
    /// It went away or wrote something else.
    Lost,
}

#[cfg(unix)]
impl Served {
    fn reply(&mut self, line: &str) -> Reply {
        if self.requests.write_all(line.as_bytes()).and_then(|()| self.requests.flush()).is_err() {
            return Reply::Lost;
        }
        let mut head = String::new();
        if self.answers.read_line(&mut head).map_or(true, |n| n == 0) {
            return Reply::Lost;
        }
        if head == "stale\n" {
            return Reply::Stale;
        }
        let Some((_, out, err)) = crate::front::answer_head(&head) else { return Reply::Lost };
        let mut frame = head.into_bytes();
        let start = frame.len();
        frame.resize(start + out + err, 0);
        if self.answers.read_exact(&mut frame[start..]).is_err() {
            return Reply::Lost;
        }
        Reply::Frame(frame)
    }
}

#[cfg(unix)]
impl State {
    /// Whether this executable or the stage 1 it serves was built again
    /// since the server started: a stage 1 renamed into place by its build
    /// has a new size or time (decision 1362).
    fn rebuilt(&self) -> bool {
        (stamp(&self.me), stamp(&self.sisu1)) != self.built
    }

    /// One request: read, handed to a process of stage 1, its answer
    /// written back; a stream closed with no answer when none could give
    /// one.
    fn answer(&self, stream: std::os::unix::net::UnixStream) {
        let Ok(back) = stream.try_clone() else { return };
        let mut back = back;
        let mut requests = std::io::BufReader::new(stream);
        let mut line = String::new();
        if requests.read_line(&mut line).map_or(true, |n| n == 0) || !line.ends_with('\n') {
            return;
        }
        if self.ending.load(std::sync::atomic::Ordering::SeqCst) || self.rebuilt() {
            self.end();
            let _ = back.write_all(b"stale\n");
            return;
        }
        // A process of stage 1 that answers `stale` is replaced, once.
        for _ in 0..2 {
            let Some(mut served) = self.take() else { return };
            match served.reply(&line) {
                Reply::Frame(frame) => {
                    self.give_back(served);
                    let _ = back.write_all(&frame).and_then(|()| back.flush());
                    return;
                }
                Reply::Stale => {
                    let _ = served.child.wait();
                    self.gone();
                }
                Reply::Lost => {
                    let _ = served.child.kill();
                    let _ = served.child.wait();
                    self.gone();
                    return;
                }
            }
        }
    }

    /// A process of stage 1 that answers no request now: one of the idle
    /// ones, or a new one while there are fewer than [`State::most`], or the
    /// first to be given back.
    fn take(&self) -> Option<Served> {
        let mut pool = self.pool.lock().ok()?;
        loop {
            if let Some(s) = pool.idle.pop() {
                return Some(s);
            }
            if pool.running < self.most {
                pool.running += 1;
                drop(pool);
                let started = self.start_one();
                if started.is_none() {
                    self.gone();
                }
                return started;
            }
            pool = self.freed.wait(pool).ok()?;
        }
    }

    fn start_one(&self) -> Option<Served> {
        let mut cmd = self.stage1.serve_command();
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().ok()?;
        let requests = child.stdin.take()?;
        let answers = std::io::BufReader::new(child.stderr.take()?);
        Some(Served { child, requests, answers })
    }

    fn give_back(&self, served: Served) {
        if let Ok(mut pool) = self.pool.lock() {
            pool.idle.push(served);
        }
        self.freed.notify_one();
    }

    /// A process of stage 1 ended.
    fn gone(&self) {
        if let Ok(mut pool) = self.pool.lock() {
            pool.running -= 1;
        }
        self.freed.notify_one();
    }

    /// The server ends: its socket removed, so that the next command starts
    /// a new one, and its processes of stage 1 asked to end (an empty line).
    fn end(&self) {
        if !self.ending.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let _ = std::fs::remove_file(&self.sock);
        }
        if let Ok(mut pool) = self.pool.lock() {
            for mut s in pool.idle.drain(..) {
                let _ = s.requests.write_all(b"\n");
                let _ = s.child.wait();
            }
        }
    }
}
