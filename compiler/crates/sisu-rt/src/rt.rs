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

/// The type numbered `id`, if the table has it.
pub(crate) fn ty_checked(id: u32) -> Option<&'static TypeEntry> {
    TYPES.get().and_then(|t| t.get(id as usize))
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
                | Kind::Bytes
                | Kind::List(_)
                | Kind::Iter(_)
                | Kind::Box(_)
                | Kind::Dict(..)
                | Kind::Set(_)
                | Kind::Dyn
                | Kind::Sender(_)
                | Kind::Receiver(_)
                | Kind::Guard(..)
                | Kind::Shared(_)
                | Kind::Weak(_)
                | Kind::Arena(_) => true,
                Kind::Mutex(v) | Kind::Cell(v) => get(*v),
                Kind::Entry(k, _) => get(*k),
                Kind::Option(p) => get(*p),
                Kind::Result(a, b) => get(*a) || get(*b),
                Kind::Tuple(slots) => slots.iter().any(|s| get(s.ty)),
                // An error holds its return trace (decision 214).
                Kind::Enum { error: true, .. } => true,
                Kind::Struct { glue: Some(_), .. } | Kind::Enum { glue: Some(_), .. } => true,
                // The fields the glue leaves out (decision 217).
                Kind::Struct { fields, .. } => fields.iter().any(|f| get(f.ty)),
                Kind::Enum { variants, .. } => variants.iter().any(|v| v.fields.iter().any(|f| get(f.ty))),
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
#[derive(Clone, Debug)]
pub(crate) struct Panic {
    pub message: String,
    pub stack: Vec<String>,
}

thread_local! {
    /// The panics in flight of the running task, the newest last: a second
    /// one is raised while the first unwinds, and reaching a frame that
    /// runs cleanup aborts. A task switch swaps them with the next task's
    /// (`sched`).
    static PANICS: RefCell<Vec<Panic>> = const { RefCell::new(Vec::new()) };
    /// Standard output, written in large pieces.
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// The panics in flight, after raising the one a failed stack probe left
/// pending: the probe returns at once, calling nothing, so its panic is
/// raised here, before any other, the first time the runtime looks.
fn with_panics<R>(f: impl FnOnce(&mut Vec<Panic>) -> R) -> R {
    let failed = crate::sched::take_stack_failure();
    PANICS.with(|p| {
        let mut p = p.borrow_mut();
        if let Some(message) = failed {
            p.push(Panic { message, stack: Vec::new() });
        }
        f(&mut p)
    })
}

/// Exchanges the panics in flight with `other`: a task switch keeps each
/// task's own.
pub(crate) fn swap_panics(other: &mut Vec<Panic>) {
    PANICS.with(|p| std::mem::swap(&mut *p.borrow_mut(), other));
}

/// Whether a panic is in flight in the running task (a mutex guard dropped
/// then checks its value's invariant, D-077).
pub(crate) fn unwinding() -> bool {
    with_panics(|p| !p.is_empty())
}

/// A catalog message (D-065), as the interpreter renders run-time ones.
pub(crate) fn render(id: MsgId, params: &[(&str, String)]) -> String {
    Catalog::builtin().render(id.id(), params)
}

/// Raises a panic with this message; the caller returns
/// [`crate::abi::UNWINDING`].
pub(crate) fn raise(message: String) -> u8 {
    with_panics(|p| p.push(Panic { message, stack: Vec::new() }));
    crate::abi::UNWINDING
}

/// Raises a panic another task's failure carried here, with its trace.
pub(crate) fn raise_panic(panic: Panic) -> u8 {
    with_panics(|p| p.push(panic));
    crate::abi::UNWINDING
}

/// Raises the catalog message `id`.
pub(crate) fn raise_msg(id: MsgId, params: &[(&str, String)]) -> u8 {
    raise(render(id, params))
}

/// Adds a line to the stack trace of the newest panic.
pub(crate) fn add_frame(line: String) {
    with_panics(|p| {
        if let Some(last) = p.last_mut() {
            last.stack.push(line);
        }
    });
}

/// The newest panic, taken.
pub(crate) fn take_panic() -> Option<Panic> {
    with_panics(|p| p.pop())
}

/// A panic reached a frame that runs cleanup for an earlier one: the
/// program aborts with both messages (05, "Panics").
pub(crate) fn abort_unwinding() -> ! {
    let (first, message) = with_panics(|p| {
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
