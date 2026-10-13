//! The host contract: everything a program does to the world.
//!
//! An intrinsic that touches the world calls a [`Host`] method and nothing
//! else, so a run under [`TestHost`] is deterministic: its output is
//! captured, its clock is fixed, and its files live in memory over an
//! optional read-only directory.

use std::collections::BTreeMap;
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
    fn exists(&self, path: &str) -> bool;
}

/// The real world: the process's streams, arguments, environment, clock
/// and file system.
pub struct StdHost {
    pub args: Vec<String>,
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

    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        let bytes = std::fs::read(path).map_err(|e| io_kind(&e))?;
        String::from_utf8(bytes).map_err(|_| IoErrorKind::InvalidData)
    }

    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        std::fs::write(path, text).map_err(|e| io_kind(&e))
    }

    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
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
    pub files: BTreeMap<String, String>,
    /// Reads of relative paths not in `files` come from this directory.
    pub dir: Option<PathBuf>,
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

    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        if let Some(t) = self.files.get(path) {
            return Ok(t.clone());
        }
        let Some(dir) = &self.dir else { return Err(IoErrorKind::NotFound) };
        let rel = std::path::Path::new(path);
        if rel.is_absolute() || rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(IoErrorKind::PermissionDenied);
        }
        let bytes = std::fs::read(dir.join(rel)).map_err(|e| io_kind(&e))?;
        String::from_utf8(bytes).map_err(|_| IoErrorKind::InvalidData)
    }

    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        self.files.insert(path.to_string(), text.to_string());
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path) || self.dir.as_ref().is_some_and(|d| d.join(path).exists())
    }
}
