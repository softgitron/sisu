//! The runtime's state: the type table, the function table, the panics in
//! flight, and standard output.

use crate::abi::{Kind, TypeEntry};
use sisu_diag::{Catalog, MsgId, msg};
use std::cell::RefCell;
use std::io::Write as _;
use std::sync::OnceLock;

/// The program's types, decoded once when it starts.
static TYPES: OnceLock<Vec<TypeEntry>> = OnceLock::new();
/// Whether dropping a value of each type can do anything.
static NEEDS_DROP: OnceLock<Vec<bool>> = OnceLock::new();
/// The addresses of the program's functions the type table names (drop
/// glue), as integers.
static FNS: OnceLock<Vec<usize>> = OnceLock::new();

/// A compiled function that takes one value: `fn(ret, value) -> status`.
pub(crate) type ValueFn = unsafe extern "C" fn(*mut u8, *mut u8) -> u8;

pub(crate) fn init(types: Vec<TypeEntry>, fns: Vec<usize>) {
    let needs = needs_drop_table(&types);
    let _ = TYPES.set(types);
    let _ = NEEDS_DROP.set(needs);
    let _ = FNS.set(fns);
}

static UNIT: TypeEntry = TypeEntry { size: 0, kind: Kind::Unit };

/// The type numbered `id`; a number outside the table (a broken program)
/// reads as `()`, which owns nothing.
pub(crate) fn ty(id: u32) -> &'static TypeEntry {
    TYPES.get().and_then(|t| t.get(id as usize)).unwrap_or(&UNIT)
}

pub(crate) fn needs_drop(id: u32) -> bool {
    NEEDS_DROP.get().and_then(|t| t.get(id as usize)).copied().unwrap_or(false)
}

/// The function at `index` of the function table.
pub(crate) fn function(index: u32) -> Option<ValueFn> {
    let addr = *FNS.get()?.get(index as usize)?;
    if addr == 0 {
        return None;
    }
    // SAFETY: the code generator fills the table with the addresses of
    // functions of this signature.
    Some(unsafe { std::mem::transmute::<usize, ValueFn>(addr) })
}

/// Which types own something a drop must release, to a fixed point (a
/// type may contain itself through a list or a box).
fn needs_drop_table(types: &[TypeEntry]) -> Vec<bool> {
    let mut needs = vec![false; types.len()];
    loop {
        let mut changed = false;
        for (i, t) in types.iter().enumerate() {
            if needs[i] {
                continue;
            }
            let get = |j: u32| needs.get(j as usize).copied().unwrap_or(false);
            let n = match &t.kind {
                Kind::String
                | Kind::Path
                | Kind::List(_)
                | Kind::Iter(_)
                | Kind::Box(_)
                | Kind::Dict(..)
                | Kind::Set(_)
                | Kind::Dyn => true,
                Kind::Entry(k, _) => get(*k),
                Kind::Option(p) => get(*p),
                Kind::Result(a, b) => get(*a) || get(*b),
                Kind::Tuple(slots) => slots.iter().any(|s| get(s.ty)),
                Kind::Struct { glue, .. } | Kind::Enum { glue, .. } => glue.is_some(),
                Kind::ErrSet(members) => members.iter().any(|m| get(*m)),
                _ => false,
            };
            if n {
                needs[i] = true;
                changed = true;
            }
        }
        if !changed {
            return needs;
        }
    }
}

/// A panic in flight: its message and the lines of its stack trace,
/// innermost first.
pub(crate) struct Panic {
    pub message: String,
    pub stack: Vec<String>,
}

thread_local! {
    /// The panics in flight, the newest last: a second one is raised while
    /// the first unwinds, and reaching a frame that runs cleanup aborts.
    static PANICS: RefCell<Vec<Panic>> = const { RefCell::new(Vec::new()) };
    /// Standard output, written in large pieces.
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A catalog message (D-065), as the interpreter renders run-time ones.
pub(crate) fn render(id: MsgId, params: &[(&str, String)]) -> String {
    Catalog::builtin().render(id.id(), params)
}

/// Raises a panic with this message; the caller returns
/// [`crate::abi::UNWINDING`].
pub(crate) fn raise(message: String) -> u8 {
    PANICS.with(|p| p.borrow_mut().push(Panic { message, stack: Vec::new() }));
    crate::abi::UNWINDING
}

/// Raises the catalog message `id`.
pub(crate) fn raise_msg(id: MsgId, params: &[(&str, String)]) -> u8 {
    raise(render(id, params))
}

/// Adds a line to the stack trace of the newest panic.
pub(crate) fn add_frame(line: String) {
    PANICS.with(|p| {
        if let Some(last) = p.borrow_mut().last_mut() {
            last.stack.push(line);
        }
    });
}

/// The newest panic, taken.
pub(crate) fn take_panic() -> Option<Panic> {
    PANICS.with(|p| p.borrow_mut().pop())
}

/// A panic reached a frame that runs cleanup for an earlier one: the
/// program aborts with both messages (05, "Panics").
pub(crate) fn abort_unwinding() -> ! {
    let (first, message) = PANICS.with(|p| {
        let p = p.borrow();
        let n = p.len();
        let first = if n >= 2 { p[n - 2].message.clone() } else { String::new() };
        let message = p.last().map(|x| x.message.clone()).unwrap_or_default();
        (first, message)
    });
    let text = render(msg::PANIC_DURING_UNWIND, &[("first", first), ("message", message)]);
    abort(&text)
}

/// Stops the program at once: the message on standard error, status 134.
pub(crate) fn abort(message: &str) -> ! {
    stderr(&format!("{message}\n"));
    flush();
    std::process::exit(crate::entry::EXIT_ABORT)
}

/// An internal error: the native code broke a rule the runtime relies on.
pub(crate) fn internal(detail: &str) -> ! {
    abort(&render(msg::RUN_NATIVE_INTERNAL, &[("detail", detail.to_string())]))
}

pub(crate) fn stdout(bytes: &[u8]) {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        o.extend_from_slice(bytes);
        if o.len() >= 1 << 16 {
            let _ = std::io::stdout().write_all(&o);
            o.clear();
        }
    });
}

/// Writes to standard error, after what is waiting for standard output.
pub(crate) fn stderr(text: &str) {
    flush();
    let _ = std::io::stderr().write_all(text.as_bytes());
}

pub(crate) fn flush() {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        let mut out = std::io::stdout();
        let _ = out.write_all(&o);
        let _ = out.flush();
        o.clear();
    });
}
