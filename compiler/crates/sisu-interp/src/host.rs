//! The host contract: everything a program does to the world.
//!
//! An intrinsic that touches the world calls a [`Host`] method and nothing
//! else, so a run under [`TestHost`] is deterministic: its output is
//! captured, its clock is fixed, its random bits are a seeded stream, it
//! runs no program, and its files live in memory over an optional
//! read-only directory. These are the root objects of a `System`
//! (`HostFs`, `HostClock`, `HostRand`, `HostEnv`, `HostProc`).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::PathBuf;

/// Why a file operation failed; these become the `IoError` variants of
/// the prelude.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    /// The file is not valid UTF-8.
    InvalidData,
    Interrupted,
    Other(String),
}

pub trait Host: Send {
    fn stdout(&mut self, text: &str);
    fn stderr(&mut self, text: &str);
    /// The command-line arguments after the program's own name.
    fn args(&self) -> Vec<String>;
    fn env_var(&self, name: &str) -> Option<String>;
    /// The current time in seconds since 1970-01-01T00:00:00Z.
    fn now(&self) -> i64;
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind>;
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind>;
    /// The whole file's bytes.
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        self.read_file(path).map(String::into_bytes)
    }
    /// Creates or replaces the file with `data`.
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        match std::str::from_utf8(data) {
            Ok(t) => self.write_file(path, t),
            Err(_) => Err(IoErrorKind::InvalidData),
        }
    }
    fn exists(&self, path: &str) -> bool;
    /// The current time in nanoseconds since 1970-01-01T00:00:00Z.
    fn now_nanos(&self) -> i128 {
        i128::from(self.now()) * 1_000_000_000
    }
    /// Whether the host's clock is virtual: it advances only when the
    /// scheduler has nothing to run (17, "Isolation and parallelism").
    fn virtual_time(&self) -> bool {
        false
    }
    /// Waits `nanos` of real time (the scheduler has nothing to run until
    /// a timer of the host's clock).
    fn sleep_nanos(&mut self, nanos: u64) {
        std::thread::sleep(std::time::Duration::from_nanos(nanos));
    }
    /// A file's or directory's size in bytes, time of last change in
    /// nanoseconds since the epoch, and whether it is a directory.
    fn metadata(&self, _path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        Err(IoErrorKind::PermissionDenied)
    }
    fn remove_file(&mut self, _path: &str) -> Result<(), IoErrorKind> {
        Err(IoErrorKind::PermissionDenied)
    }
    /// The names of a directory's entries (not paths), sorted.
    fn list_dir(&self, _path: &str) -> Result<Vec<String>, IoErrorKind> {
        Err(IoErrorKind::PermissionDenied)
    }
    /// Creates a directory and its missing parents.
    fn create_dir(&mut self, _path: &str) -> Result<(), IoErrorKind> {
        Err(IoErrorKind::PermissionDenied)
    }
    /// 64 random bits.
    fn random_bits(&mut self) -> u64 {
        0
    }
    /// Runs a program to its end with no input: its exit status, standard
    /// output and standard error.
    fn run_process(
        &mut self,
        _program: &str,
        _args: &[String],
    ) -> Result<(i64, String, String), IoErrorKind> {
        Err(IoErrorKind::PermissionDenied)
    }
    /// A TCP connection to `host` at `port`, by the host's handle.
    fn tcp_connect(&mut self, _host: &str, _port: u16) -> Result<u64, NetFail> {
        Err(NetFail::NotAllowed)
    }
    /// A TCP listener at `host` and `port`, by handle, and its port.
    fn tcp_listen(&mut self, _host: &str, _port: u16) -> Result<(u64, u16), NetFail> {
        Err(NetFail::NotAllowed)
    }
    /// A connection the listener received, by handle, and the peer's
    /// address; `Wait` when none has arrived.
    fn tcp_accept(&mut self, _listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        Err(NetFail::NotAllowed)
    }
    /// At most `max` bytes, none at the end of the stream; `Wait` when
    /// none has arrived.
    fn tcp_read(&mut self, _conn: u64, _max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        Err(NetFail::NotAllowed)
    }
    /// Sends what it can of `data` now: how many bytes, or `Wait`.
    fn tcp_write(&mut self, _conn: u64, _data: &[u8]) -> Result<Poll<usize>, NetFail> {
        Err(NetFail::NotAllowed)
    }
    /// Closes the sending side of a connection, or a listener.
    fn tcp_close(&mut self, _handle: u64) {}
}

/// Why a network operation failed: these become the `NetError` variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetFail {
    /// Nothing listens there (or the name does not resolve).
    Refused,
    /// The host allows no network.
    NotAllowed,
    /// The other end closed the connection.
    Closed,
    Other(String),
}

/// A network operation that would wait: the scheduler polls it again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Poll<T> {
    Ready(T),
    Wait,
}

/// SplitMix64: the next value of a seeded stream (`TestHost`'s random bits).
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The real world: the process's streams, arguments, environment, clock,
/// file system, randomness, processes and network.
pub struct StdHost {
    pub args: Vec<String>,
    /// The sockets, by handle; every one is nonblocking.
    pub sockets: Vec<Option<Socket>>,
}

impl StdHost {
    pub fn new(args: Vec<String>) -> StdHost {
        StdHost { args, sockets: Vec::new() }
    }

    fn add(&mut self, s: Socket) -> u64 {
        self.sockets.push(Some(s));
        (self.sockets.len() - 1) as u64
    }

    fn socket(&mut self, h: u64) -> Result<&mut Socket, NetFail> {
        self.sockets.get_mut(h as usize).and_then(|s| s.as_mut()).ok_or(NetFail::Closed)
    }
}

/// A socket of the real host.
pub enum Socket {
    Stream(std::net::TcpStream),
    Listener(std::net::TcpListener),
}

fn net_fail(e: &std::io::Error) -> NetFail {
    match e.kind() {
        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound => NetFail::Refused,
        std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted
        | std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::UnexpectedEof => NetFail::Closed,
        std::io::ErrorKind::PermissionDenied => NetFail::NotAllowed,
        _ => NetFail::Other(e.to_string()),
    }
}

fn io_kind(e: &std::io::Error) -> IoErrorKind {
    match e.kind() {
        std::io::ErrorKind::NotFound => IoErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied => IoErrorKind::PermissionDenied,
        std::io::ErrorKind::AlreadyExists => IoErrorKind::AlreadyExists,
        std::io::ErrorKind::InvalidData => IoErrorKind::InvalidData,
        std::io::ErrorKind::Interrupted => IoErrorKind::Interrupted,
        _ => IoErrorKind::Other(e.to_string()),
    }
}

impl Host for StdHost {
    fn stdout(&mut self, text: &str) {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    }

    fn stderr(&mut self, text: &str) {
        let _ = std::io::stderr().lock().write_all(text.as_bytes());
    }

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }

    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn now(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn now_nanos(&self) -> i128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i128)
            .unwrap_or(0)
    }

    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        let bytes = std::fs::read(path).map_err(|e| io_kind(&e))?;
        String::from_utf8(bytes).map_err(|_| IoErrorKind::InvalidData)
    }

    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        std::fs::write(path, text).map_err(|e| io_kind(&e))
    }

    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        std::fs::read(path).map_err(|e| io_kind(&e))
    }

    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        std::fs::write(path, data).map_err(|e| io_kind(&e))
    }

    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        let m = std::fs::metadata(path).map_err(|e| io_kind(&e))?;
        let modified = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        Ok((i64::try_from(m.len()).unwrap_or(i64::MAX), modified, m.is_dir()))
    }

    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        std::fs::remove_file(path).map_err(|e| io_kind(&e))
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        let mut names = Vec::new();
        for e in std::fs::read_dir(path).map_err(|e| io_kind(&e))? {
            let e = e.map_err(|e| io_kind(&e))?;
            names.push(e.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        std::fs::create_dir_all(path).map_err(|e| io_kind(&e))
    }

    fn random_bits(&mut self) -> u64 {
        use std::io::Read as _;
        let mut b = [0u8; 8];
        if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok() {
            return u64::from_le_bytes(b);
        }
        // No system source: the clock and a per-process key, mixed.
        let mut s = self.now_nanos() as u64
            ^ std::hash::BuildHasher::hash_one(&std::collections::hash_map::RandomState::new(), 0u8);
        splitmix64(&mut s)
    }

    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, NetFail> {
        let s = std::net::TcpStream::connect((host, port)).map_err(|e| net_fail(&e))?;
        s.set_nonblocking(true).map_err(|e| net_fail(&e))?;
        Ok(self.add(Socket::Stream(s)))
    }

    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), NetFail> {
        let l = std::net::TcpListener::bind((host, port)).map_err(|e| net_fail(&e))?;
        l.set_nonblocking(true).map_err(|e| net_fail(&e))?;
        let at = l.local_addr().map(|a| a.port()).unwrap_or(port);
        Ok((self.add(Socket::Listener(l)), at))
    }

    fn tcp_accept(&mut self, listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        let Socket::Listener(l) = self.socket(listener)? else { return Err(NetFail::Closed) };
        match l.accept() {
            Ok((s, peer)) => {
                s.set_nonblocking(true).map_err(|e| net_fail(&e))?;
                Ok(Poll::Ready((self.add(Socket::Stream(s)), peer.to_string())))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(Poll::Wait),
            Err(e) => Err(net_fail(&e)),
        }
    }

    fn tcp_read(&mut self, conn: u64, max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        use std::io::Read as _;
        let Socket::Stream(s) = self.socket(conn)? else { return Err(NetFail::Closed) };
        let mut buf = vec![0u8; max.clamp(1, 1 << 20)];
        match s.read(&mut buf) {
            Ok(n) => {
                buf.truncate(n);
                Ok(Poll::Ready(buf))
            }
            Err(e)
                if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted) =>
            {
                Ok(Poll::Wait)
            }
            Err(e) => Err(net_fail(&e)),
        }
    }

    fn tcp_write(&mut self, conn: u64, data: &[u8]) -> Result<Poll<usize>, NetFail> {
        use std::io::Write as _;
        let Socket::Stream(s) = self.socket(conn)? else { return Err(NetFail::Closed) };
        match s.write(data) {
            Ok(n) => Ok(Poll::Ready(n)),
            Err(e)
                if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted) =>
            {
                Ok(Poll::Wait)
            }
            Err(e) => Err(net_fail(&e)),
        }
    }

    fn tcp_close(&mut self, handle: u64) {
        match self.sockets.get_mut(handle as usize) {
            Some(Some(Socket::Stream(s))) => {
                let _ = s.shutdown(std::net::Shutdown::Write);
            }
            Some(slot @ Some(Socket::Listener(_))) => *slot = None,
            _ => {}
        }
    }

    fn run_process(&mut self, program: &str, args: &[String]) -> Result<(i64, String, String), IoErrorKind> {
        let out = std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| io_kind(&e))?;
        Ok((
            i64::from(out.status.code().unwrap_or(-1)),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ))
    }
}

/// A deterministic host for tests and the conformance suite.
#[derive(Clone, Debug, Default)]
pub struct TestHost {
    pub out: String,
    pub err: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub now: i64,
    /// Files by path. Writes land here; reads look here first.
    pub files: BTreeMap<String, Vec<u8>>,
    /// Reads of relative paths not in `files` come from this directory.
    pub dir: Option<PathBuf>,
    /// The directories made by `create_dir` (a file's parents exist too).
    pub dirs: BTreeSet<String>,
    /// The state of the random bits, SplitMix64 from 0.
    pub rand: u64,
}

impl Host for TestHost {
    fn stdout(&mut self, text: &str) {
        self.out.push_str(text);
    }

    fn stderr(&mut self, text: &str) {
        self.err.push_str(text);
    }

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }

    fn env_var(&self, name: &str) -> Option<String> {
        self.env.get(name).cloned()
    }

    fn now(&self) -> i64 {
        self.now
    }

    fn virtual_time(&self) -> bool {
        true
    }

    fn sleep_nanos(&mut self, _nanos: u64) {}

    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        let bytes = self.read_bytes(path)?;
        String::from_utf8(bytes).map_err(|_| IoErrorKind::InvalidData)
    }

    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        self.write_bytes(path, text.as_bytes())
    }

    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        if let Some(t) = self.files.get(path) {
            return Ok(t.clone());
        }
        let Some(dir) = &self.dir else { return Err(IoErrorKind::NotFound) };
        if !self.inside(path) {
            return Err(IoErrorKind::PermissionDenied);
        }
        std::fs::read(dir.join(path)).map_err(|e| io_kind(&e))
    }

    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        self.files.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
            || self.is_dir(path)
            || self.dir.as_ref().is_some_and(|d| self.inside(path) && d.join(path).exists())
    }

    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        let now = i64::try_from(self.now_nanos()).unwrap_or(i64::MAX);
        if let Some(t) = self.files.get(path) {
            return Ok((t.len() as i64, now, false));
        }
        if self.is_dir(path) {
            return Ok((0, now, true));
        }
        let Some(dir) = &self.dir else { return Err(IoErrorKind::NotFound) };
        if !self.inside(path) {
            return Err(IoErrorKind::PermissionDenied);
        }
        let m = std::fs::metadata(dir.join(path)).map_err(|e| io_kind(&e))?;
        Ok((i64::try_from(m.len()).unwrap_or(i64::MAX), now, m.is_dir()))
    }

    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        match self.files.remove(path) {
            Some(_) => Ok(()),
            None if self.exists(path) => Err(IoErrorKind::PermissionDenied),
            None => Err(IoErrorKind::NotFound),
        }
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        let dir = path.trim_end_matches('/');
        if !self.is_dir(dir) && !self.dir.as_ref().is_some_and(|d| self.inside(dir) && d.join(dir).is_dir()) {
            return Err(IoErrorKind::NotFound);
        }
        let prefix = if dir.is_empty() || dir == "." { String::new() } else { format!("{dir}/") };
        let mut names = BTreeSet::new();
        for k in self.files.keys().chain(self.dirs.iter()) {
            if let Some(rest) = k.strip_prefix(&prefix)
                && !rest.is_empty()
            {
                names.insert(rest.split('/').next().unwrap_or(rest).to_string());
            }
        }
        if let Some(d) = &self.dir
            && let Ok(entries) = std::fs::read_dir(d.join(dir))
        {
            for e in entries.flatten() {
                names.insert(e.file_name().to_string_lossy().into_owned());
            }
        }
        Ok(names.into_iter().collect())
    }

    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        let mut at = String::new();
        for part in path.trim_end_matches('/').split('/') {
            if !at.is_empty() {
                at.push('/');
            }
            at.push_str(part);
            self.dirs.insert(at.clone());
        }
        Ok(())
    }

    fn random_bits(&mut self) -> u64 {
        splitmix64(&mut self.rand)
    }
}

impl TestHost {
    /// Whether `path` is a directory made, or the parent of a file.
    fn is_dir(&self, path: &str) -> bool {
        let dir = path.trim_end_matches('/');
        if dir.is_empty() || dir == "." {
            return true;
        }
        let prefix = format!("{dir}/");
        self.dirs.contains(dir) || self.files.keys().any(|k| k.starts_with(&prefix))
    }

    /// Whether a relative path stays inside the read-only directory.
    fn inside(&self, path: &str) -> bool {
        let rel = std::path::Path::new(path);
        !rel.is_absolute() && !rel.components().any(|c| matches!(c, std::path::Component::ParentDir))
    }
}
