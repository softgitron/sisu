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
/// Whether a copy of a value of each type is a copy of its bytes.
static PLAIN: OnceLock<Vec<bool>> = OnceLock::new();
/// The parts of each struct, tuple and enum variant that a copy copies
/// again over the value's bytes (decision 262).
static COPY_PARTS: OnceLock<Vec<CopyParts>> = OnceLock::new();
/// The parts of each struct, tuple and enum variant that a drop drops
/// (decision 266).
static DROP_PARTS: OnceLock<Vec<DropParts>> = OnceLock::new();
/// The addresses of the program's functions the type table names (drop
/// glue), as integers.
static FNS: OnceLock<Vec<usize>> = OnceLock::new();

/// How a value of a type that is not plain is copied, when it is not by
/// its kind's own rule: its bytes, then each part that owns something
/// (its offset and type) copied over them, a struct's or a tuple's
/// fields, through the structs and tuples it holds in place; for an enum,
/// such parts of each variant.
#[derive(Clone, Debug, Default)]
pub(crate) enum CopyParts {
    #[default]
    ByKind,
    Parts(Vec<(u32, u32)>),
    Variants(Vec<Vec<(u32, u32)>>),
}

/// How a value of a type that needs a drop is dropped, when it is not by
/// its kind's own rule: each part that needs one (its offset and type), in
/// the order that rule drops them, a struct's fields and a tuple's
/// elements last to first, through the structs without drop glue and the
/// tuples it holds in place; for an enum without glue that is not an
/// error set, such parts of each variant.
#[derive(Clone, Debug, Default)]
pub(crate) enum DropParts {
    #[default]
    ByKind,
    Parts(Vec<(u32, u32)>),
    Variants(Vec<Vec<(u32, u32)>>),
}

/// A compiled function that takes one value: `fn(ret, value) -> status`.
pub(crate) type ValueFn = unsafe extern "C" fn(*mut u8, *mut u8) -> u8;

pub(crate) fn init(types: Vec<TypeEntry>, fns: Vec<usize>) {
    let needs = needs_drop_table(&types);
    let plain = plain_table(&types, &needs);
    let parts = copy_parts_table(&types, &plain);
    let drops = drop_parts_table(&types, &needs);
    crate::rawmem::set_up(&types);
    let _ = TYPES.set(types);
    let _ = NEEDS_DROP.set(needs);
    let _ = PLAIN.set(plain);
    let _ = COPY_PARTS.set(parts);
    let _ = DROP_PARTS.set(drops);
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

/// Whether a copy of a value of type `id` is a copy of its bytes: it owns
/// nothing and holds no lock, whose copy starts unlocked.
pub(crate) fn plain(id: u32) -> bool {
    PLAIN.get().and_then(|t| t.get(id as usize)).copied().unwrap_or(false)
}

/// How a value of type `id` that is not plain is copied ([`CopyParts`]).
pub(crate) fn copy_parts(id: u32) -> &'static CopyParts {
    static BY_KIND: CopyParts = CopyParts::ByKind;
    COPY_PARTS.get().and_then(|t| t.get(id as usize)).unwrap_or(&BY_KIND)
}

/// How a value of type `id` that needs a drop is dropped ([`DropParts`]).
pub(crate) fn drop_parts(id: u32) -> &'static DropParts {
    static BY_KIND: DropParts = DropParts::ByKind;
    DROP_PARTS.get().and_then(|t| t.get(id as usize)).unwrap_or(&BY_KIND)
}

/// [`DropParts`] for every type that needs a drop: a struct without drop
/// glue whose fields no glue drops, a tuple, and an enum whose variants
/// are such, that is not an error set (whose drop lets go of its return
/// trace). What a drop by the parts does is what the kinds' rules do: a
/// struct without glue drops its fields that need it last to first, as a
/// tuple its elements, so opening one in place keeps the order, and the
/// status after each part is combined as before.
fn drop_parts_table(types: &[TypeEntry], needs: &[bool]) -> Vec<DropParts> {
    fn opened(entry: &TypeEntry) -> bool {
        match &entry.kind {
            Kind::Struct { fields, glue: None, .. } => fields.iter().all(|f| !f.glue),
            Kind::Tuple(_) => true,
            _ => false,
        }
    }
    fn open(types: &[TypeEntry], needs: &[bool], t: u32, at: u32, out: &mut Vec<(u32, u32)>, depth: u32) {
        if !needs.get(t as usize).copied().unwrap_or(false) {
            return;
        }
        let inner: Vec<(u32, u32)> = match types.get(t as usize) {
            Some(entry) if depth < 16 && opened(entry) => match &entry.kind {
                Kind::Struct { fields, .. } => fields.iter().rev().map(|f| (f.offset, f.ty)).collect(),
                Kind::Tuple(slots) => slots.iter().rev().map(|s| (s.offset, s.ty)).collect(),
                _ => Vec::new(),
            },
            _ => {
                out.push((at, t));
                return;
            }
        };
        for (off, ty) in inner {
            open(types, needs, ty, at + off, out, depth + 1);
        }
    }
    types
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            if !needs.get(i).copied().unwrap_or(false) {
                return DropParts::ByKind;
            }
            match &entry.kind {
                Kind::Struct { .. } | Kind::Tuple(_) if opened(entry) => {
                    let mut parts = Vec::new();
                    open(types, needs, i as u32, 0, &mut parts, 0);
                    DropParts::Parts(parts)
                }
                Kind::Enum { variants, glue: None, error: false, .. }
                    if variants.iter().all(|v| v.fields.iter().all(|f| !f.glue)) =>
                {
                    let all = variants
                        .iter()
                        .map(|v| {
                            let mut parts = Vec::new();
                            for f in v.fields.iter().rev() {
                                open(types, needs, f.ty, f.offset, &mut parts, 1);
                            }
                            parts
                        })
                        .collect();
                    DropParts::Variants(all)
                }
                _ => DropParts::ByKind,
            }
        })
        .collect()
}

/// [`CopyParts`] for every type: the parts of a struct's fields, a tuple's
/// elements and each variant's fields that are not plain, each in place,
/// a struct or tuple among them opened into its own parts at its offset,
/// in order. A struct or tuple that holds a lock in place keeps its kind's
/// rule, which a copy of a lock needs.
fn copy_parts_table(types: &[TypeEntry], plain: &[bool]) -> Vec<CopyParts> {
    fn open(
        types: &[TypeEntry],
        plain: &[bool],
        t: u32,
        at: u32,
        out: &mut Vec<(u32, u32)>,
        depth: u32,
    ) -> bool {
        if plain.get(t as usize).copied().unwrap_or(false) {
            return true;
        }
        let Some(entry) = types.get(t as usize) else { return false };
        let inner: Vec<(u32, u32)> = match &entry.kind {
            Kind::Struct { fields, .. } if depth < 16 => fields.iter().map(|f| (f.offset, f.ty)).collect(),
            Kind::Tuple(slots) if depth < 16 => slots.iter().map(|s| (s.offset, s.ty)).collect(),
            Kind::Mutex(_) | Kind::RwLock(_) => return false,
            _ => {
                out.push((at, t));
                return true;
            }
        };
        inner.into_iter().all(|(off, ty)| open(types, plain, ty, at + off, out, depth + 1))
    }
    types
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            if plain.get(i).copied().unwrap_or(false) {
                return CopyParts::ByKind;
            }
            let mut parts = Vec::new();
            match &entry.kind {
                Kind::Struct { .. } | Kind::Tuple(_) => {
                    if open(types, plain, i as u32, 0, &mut parts, 0) {
                        CopyParts::Parts(parts)
                    } else {
                        CopyParts::ByKind
                    }
                }
                Kind::Enum { variants, .. } => {
                    let mut all = Vec::new();
                    for v in variants {
                        let mut parts = Vec::new();
                        if !v.fields.iter().all(|f| open(types, plain, f.ty, f.offset, &mut parts, 1)) {
                            return CopyParts::ByKind;
                        }
                        all.push(parts);
                    }
                    CopyParts::Variants(all)
                }
                _ => CopyParts::ByKind,
            }
        })
        .collect()
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

/// Which types [`plain`] holds of: those with nothing to drop that hold no
/// `Mutex` or `RwLock` in place, to a fixed point.
fn plain_table(types: &[TypeEntry], needs: &[bool]) -> Vec<bool> {
    let mut locks = vec![false; types.len()];
    loop {
        let mut changed = false;
        for (i, t) in types.iter().enumerate() {
            if locks[i] {
                continue;
            }
            let get = |j: u32| locks.get(j as usize).copied().unwrap_or(false);
            let l = match &t.kind {
                Kind::Mutex(_) | Kind::RwLock(_) => true,
                Kind::Cell(v) | Kind::Option(v) | Kind::Entry(v, _) => get(*v),
                Kind::Result(a, b) => get(*a) || get(*b),
                Kind::Tuple(slots) => slots.iter().any(|s| get(s.ty)),
                Kind::Struct { fields, .. } => fields.iter().any(|f| get(f.ty)),
                Kind::Enum { variants, .. } => variants.iter().any(|v| v.fields.iter().any(|f| get(f.ty))),
                Kind::ErrSet(members) => members.iter().any(|m| get(*m)),
                _ => false,
            };
            if l {
                locks[i] = true;
                changed = true;
            }
        }
        if !changed {
            return needs.iter().zip(&locks).map(|(n, l)| !n && !l).collect();
        }
    }
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
                | Kind::ReadGuard(_)
                | Kind::Shared(_)
                | Kind::Weak(_)
                | Kind::Arena(_) => true,
                // A capability handle's copies are counted (decision 237),
                // and so are those a function value holds (decision 248).
                Kind::Opaque(n) => crate::value::counted(n),
                Kind::Fn => true,
                Kind::Mutex(v) | Kind::RwLock(v) | Kind::Cell(v) => get(*v),
                Kind::Entry(k, _) => get(*k),
                Kind::Option(p) => get(*p),
                Kind::Result(a, b) => get(*a) || get(*b),
                Kind::Tuple(slots) => slots.iter().any(|s| get(s.ty)),
                // An error holds its return trace (decision 215).
                Kind::Enum { error: true, .. } => true,
                Kind::Struct { glue: Some(_), .. } | Kind::Enum { glue: Some(_), .. } => true,
                // The fields the glue leaves out (decision 218).
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
        let first = if n >= 2 { Some(p[n - 2].message.clone()) } else { None };
        let message = p.last().map(|x| x.message.clone()).unwrap_or_default();
        (first, message)
    });
    let text = match first {
        // A generator dropped before its end cleans up without a panic of
        // its own (decision 236).
        None if crate::sched::abandoning() => render(msg::PANIC_DURING_ABANDON, &[("message", message)]),
        first => {
            render(msg::PANIC_DURING_UNWIND, &[("first", first.unwrap_or_default()), ("message", message)])
        }
    };
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
