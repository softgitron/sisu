//! The checked memory model of native code (decisions 543 and 544): in a
//! program built in the debug profile, the runtime checks raw memory as
//! the interpreter's model does (`sisu-interp/src/rawmem.rs`, decision
//! 301), with the interpreter's panics, for the memory it can name: the
//! blocks `Global.acquire` gives out. Native pointers are addresses, so a
//! pointer belongs to the block whose bytes, or whose end, its address
//! falls on; an address below [`LOW`] has no memory behind it; any other
//! address (a place of the program, a block foreign code gave out) is not
//! checked.
//!
//! A block keeps what it holds as the interpreter's does: the values
//! written into it, each at its offset with the type it was written as,
//! and the bytes left of a plain value a write cut apart. A read at the
//! type a value was written as takes it out when the type needs a drop; a
//! read at another type goes through the bytes, which the values of plain
//! types give and the program's memory holds, its bits checked for the
//! type; a write over a value that needs a drop is refused, and so is a
//! release of a block that holds one. A released block keeps its address
//! for a while (its memory is not given back, [`QUARANTINE`] blocks at
//! most), so that a pointer into it is seen to point into released memory;
//! past that, its memory is freed and its record forgotten. Foreign code
//! reads and writes memory the runtime does not see: a foreign call marks
//! every live block exposed, whose bytes that hold no value then read as
//! plain bytes. The release profile checks none of it (22: the raw memory
//! operations are the unchecked ones).

use crate::abi::Kind;
use crate::rt::{self, ty};
use sisu_diag::msg;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// Whether the program checks raw memory (`abi::ENTRY_CHECKED`).
static ON: AtomicBool = AtomicBool::new(false);

/// The lowest address with memory behind it: Linux maps nothing below its
/// default `mmap_min_addr`, so a pointer below it (a dangling one, whose
/// address is its alignment) points to no memory.
const LOW: u64 = 65_536;

/// How many released blocks keep their address before the oldest is freed.
const QUARANTINE: usize = 4_096;

pub(crate) fn set_on(on: bool) {
    ON.store(on, Relaxed);
}

/// Whether the program checks raw memory.
pub(crate) fn on() -> bool {
    ON.load(Relaxed)
}

/// What a block holds at an offset.
#[derive(Clone, Copy, Debug)]
enum Item {
    /// A value written as this type.
    Value(u32),
    /// A byte of a plain value a write cut apart.
    Byte,
}

/// Who gave a block out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    /// `Global.acquire`.
    Global,
    /// C's `malloc`, which `free` takes back (decision 561).
    Malloc,
    /// C's `getenv`: text the program may read and not write or free.
    ReadOnly,
}

struct Block {
    origin: Origin,
    size: u64,
    align: u64,
    live: bool,
    /// A foreign call may have written it.
    exposed: bool,
    items: BTreeMap<u64, Item>,
}

/// A place of the program a pointer was made to (`ptr.of`, `ptr.of_mut`),
/// on the stack of a task (decision 560).
#[derive(Clone, Copy, Debug)]
struct Place {
    /// Where its bytes end.
    end: u64,
    /// The top of the stack it is on, which names the stack.
    top: u64,
    /// A pointer was made to it from a shared borrow (`ptr.of`), from a
    /// mutable one (`ptr.of_mut`).
    shared: bool,
    mutable: bool,
}

struct Shadow {
    /// The blocks by their address.
    blocks: BTreeMap<u64, Block>,
    /// The released blocks whose memory is kept, the oldest first.
    released: VecDeque<u64>,
    /// The places pointers were made to, by their address, and the order
    /// they were first seen in, the oldest first.
    places: BTreeMap<u64, Place>,
    place_order: VecDeque<u64>,
}

static SHADOW: Mutex<Shadow> = Mutex::new(Shadow {
    blocks: BTreeMap::new(),
    released: VecDeque::new(),
    places: BTreeMap::new(),
    place_order: VecDeque::new(),
});

/// How many places the model remembers before it forgets the oldest.
const PLACES: usize = 4_096;

/// How many records before an address a lookup of a place looks at.
const PLACE_SCAN: usize = 64;

/// `ptr.of(x)`, or `ptr.of_mut(x)` when `mutable`, made a pointer to the
/// `size` bytes at `p`: a place on the stack of the code that asked (at or
/// above its stack pointer, below its stack's top) is remembered, with the
/// kind of borrow it came from (decision 560). A place elsewhere, a
/// value on the heap, is not.
pub(crate) fn place(p: u64, size: u64, mutable: bool) {
    if !on() {
        return;
    }
    let sp = crate::tls::caller_sp();
    if sp == 0 || p < sp {
        return;
    }
    let Some(top) = crate::sched::stack_top(sp) else { return };
    if p >= top {
        return;
    }
    let end = p + size.max(1);
    shadow(|s| match s.places.get_mut(&p) {
        Some(r) if r.top == top => {
            r.end = r.end.max(end);
            r.shared |= !mutable;
            r.mutable |= mutable;
        }
        _ => {
            s.places.insert(p, Place { end, top, shared: !mutable, mutable });
            s.place_order.push_back(p);
            if s.place_order.len() > PLACES
                && let Some(old) = s.place_order.pop_front()
            {
                s.places.remove(&old);
            }
        }
    });
}

/// What the model knows of the place at `p` of the running code's stack:
/// whether its frame has returned (`p` is below every live frame, the
/// stack pointer of the call that asks), and whether every pointer made to
/// it came from a shared borrow. `None` for an address that is no place
/// the model remembers on this stack.
fn place_at(s: &Shadow, p: u64) -> Option<(bool, bool)> {
    if s.places.is_empty() {
        return None;
    }
    let sp = crate::tls::caller_sp();
    if sp == 0 {
        return None;
    }
    let top = crate::sched::stack_top(sp)?;
    let (mut found, mut shared, mut mutable) = (false, false, false);
    for (_, r) in s.places.range(..=p).rev().take(PLACE_SCAN) {
        if r.top == top && p < r.end {
            found = true;
            shared |= r.shared;
            mutable |= r.mutable;
        }
    }
    found.then_some((p < sp, shared && !mutable))
}

/// A write by `op` through `p`, whose obligation is `obligation`: refused
/// when `p` points to a place every pointer to which came from a shared
/// borrow (`R0139`, decision 560), as the interpreter refuses a write
/// through a pointer `ptr.of` made.
pub(crate) fn writable(p: u64, op: &str, obligation: &str) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    let shared = shadow(|s| match find(s, p) {
        Some((base, _)) => s.blocks[&base].origin == Origin::ReadOnly,
        None => place_at(s, p).is_some_and(|(gone, shared)| !gone && shared),
    });
    if shared {
        return Err(refuse(msg::PANIC_RAW_SHARED_WRITE, &op_params(op, obligation)));
    }
    Ok(())
}

fn shadow<R>(f: impl FnOnce(&mut Shadow) -> R) -> R {
    let mut g = SHADOW.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

/// A panic of the model: its status.
type Refused = u8;

/// A panic of the model to raise: its message and parameters.
type Report = (msg::MsgId, Vec<(&'static str, String)>);

fn refuse(id: msg::MsgId, params: &[(&str, String)]) -> Refused {
    crate::sched::system(|| rt::raise_msg(id, params))
}

fn op_params(op: &str, obligation: &str) -> Vec<(&'static str, String)> {
    vec![("op", format!("ptr.{op}")), ("obligation", obligation.to_string())]
}

/// The block an address points into or at the end of, and the offset.
fn find(s: &Shadow, p: u64) -> Option<(u64, u64)> {
    let (&base, b) = s.blocks.range(..=p).next_back()?;
    (p - base <= b.size).then_some((base, p - base))
}

/// `Global.acquire` gave out the block at `base`.
pub(crate) fn acquired(base: *mut u8, size: u64, align: u64) {
    if !on() || base.is_null() {
        return;
    }
    shadow(|s| {
        s.blocks.insert(
            base as u64,
            Block { origin: Origin::Global, size, align, live: true, exposed: false, items: BTreeMap::new() },
        );
    });
}

/// `Global.release(p, layout)`: refused unless `p` is the start of a live
/// block of that layout that holds no value never dropped; whether the
/// model keeps the block, whose memory the caller then must not free.
pub(crate) fn release(p: *mut u8, size: i64, align: i64) -> Result<bool, Refused> {
    if !on() {
        return Ok(false);
    }
    let kept = let_out(p, Some((size, align)))?;
    crate::mem::uncount();
    Ok(kept)
}

/// The release of the block at `p` by `Global.release` (with its layout)
/// or C's `free` (without), refused as the interpreter refuses it; the
/// block's memory is kept for a while (`true`), and the oldest released
/// block beyond the quarantine is freed as its allocator frees.
fn let_out(p: *mut u8, layout: Option<(i64, i64)>) -> Result<bool, Refused> {
    let unwinding = crate::sched::system(rt::unwinding);
    let (op, allocator, want) = match layout {
        Some(_) => ("Global.release", "Global.acquire", Origin::Global),
        None => ("free", "malloc", Origin::Malloc),
    };
    let verdict = shadow(|s| {
        let other = || {
            (msg::PANIC_RAW_RELEASE_OTHER, vec![("op", op.to_string()), ("allocator", allocator.to_string())])
        };
        let Some(block) = s.blocks.get(&(p as u64)) else { return Err(other()) };
        if block.origin != want {
            return Err(other());
        }
        if !block.live {
            return Err((msg::PANIC_RAW_RELEASE_TWICE, vec![("op", op.to_string())]));
        }
        if let Some((size, align)) = layout
            && (size as u64 != block.size || align as u64 != block.align)
        {
            return Err((
                msg::PANIC_RAW_RELEASE_LAYOUT,
                vec![
                    ("size", size.to_string()),
                    ("align", align.to_string()),
                    ("block_size", block.size.to_string()),
                    ("block_align", block.align.to_string()),
                ],
            ));
        }
        // While a panic unwinds, a half-done operation may leave a value,
        // which is then forgotten (the interpreter's rule).
        let left = block.items.iter().filter(|_| !unwinding).find_map(|(at, item)| match item {
            Item::Value(t) if rt::needs_drop(*t) => Some((*at, *t)),
            _ => None,
        });
        if let Some((at, t)) = left {
            return Err((
                msg::PANIC_RAW_RELEASE_LIVE,
                vec![("op", op.to_string()), ("ty", crate::rawmem::type_name(t)), ("offset", at.to_string())],
            ));
        }
        let Some(block) = s.blocks.get_mut(&(p as u64)) else {
            rt::internal("a block of `Global` that went")
        };
        block.live = false;
        block.items.clear();
        s.released.push_back(p as u64);
        // The oldest released block beyond the quarantine is freed.
        let mut freed = None;
        if s.released.len() > QUARANTINE
            && let Some(old) = s.released.pop_front()
            && let Some(b) = s.blocks.remove(&old)
        {
            freed = Some((old, b.origin));
        }
        Ok(freed)
    });
    match verdict {
        Err((id, params)) => Err(refuse(id, &params)),
        Ok(freed) => {
            match freed {
                Some((old, Origin::Global)) => crate::mem::free_uncounted(old as *mut u8),
                // SAFETY: C's `malloc` gave the block out, and the model
                // kept it from `free` until now.
                Some((old, Origin::Malloc)) => unsafe { libc::free(old as *mut libc::c_void) },
                Some((_, Origin::ReadOnly)) | None => {}
            }
            Ok(true)
        }
    }
}

/// The run is over: what the program's raw memory still holds is let go
/// of, as the interpreter's model drops it with the run, and as decision
/// 223 frees what the host holds: each value left in a live block that
/// needs a drop is forgotten (what it owns freed, no Sisu code run), and the
/// block freed, so that only the runtime's own allocations are counted at
/// exit.
pub(crate) fn let_go() {
    if !on() {
        return;
    }
    // Each live block: its address, who gave it out, and the values in it
    // that need a drop, by their offsets.
    type Live = (u64, Origin, Vec<(u64, u32)>);
    let live: Vec<Live> = shadow(|s| {
        let out = s
            .blocks
            .iter()
            .filter(|(_, b)| b.live && b.origin != Origin::ReadOnly)
            .map(|(&base, b)| {
                let values = b
                    .items
                    .iter()
                    .filter_map(|(&at, i)| match i {
                        Item::Value(t) if rt::needs_drop(*t) => Some((at, *t)),
                        _ => None,
                    })
                    .collect();
                (base, b.origin, values)
            })
            .collect();
        s.blocks.clear();
        s.released.clear();
        s.places.clear();
        s.place_order.clear();
        out
    });
    for (base, origin, values) in live {
        for (at, t) in values {
            let mut buf = vec![0u64; (ty(t).size as usize).div_ceil(8).max(1)];
            let tmp = buf.as_mut_ptr() as *mut u8;
            // SAFETY: the block holds a value of type `t` at `at`, written
            // there and never taken out.
            unsafe {
                crate::rawmem::to_value(tmp, (base + at) as *const u8, t);
                crate::value::forget_value(tmp, t);
            }
        }
        match origin {
            Origin::Global => crate::mem::release(base as *mut u8),
            // SAFETY: C's `malloc` gave the block out and nothing freed it.
            _ => unsafe { libc::free(base as *mut libc::c_void) },
        }
    }
}

/// A foreign call follows: every live block may be read and written by
/// code the runtime does not see.
pub(crate) fn expose() {
    if !on() {
        return;
    }
    shadow(|s| {
        for b in s.blocks.values_mut().filter(|b| b.live) {
            b.exposed = true;
        }
    });
}

/// Checks an access of `size` bytes aligned to `align` at `p`, of type `t`,
/// by `op`: memory behind the pointer, live, inside its block, aligned. The
/// block and the offset, when the model knows the memory.
fn access(
    s: &Shadow,
    p: u64,
    size: u64,
    align: u64,
    t: u32,
    op: &str,
    obligation: &str,
) -> Result<Option<(u64, u64)>, Report> {
    let Some((base, off)) = find(s, p) else {
        if p < LOW && !(size == 0 && p != 0 && p.is_multiple_of(align.max(1))) {
            return Err((msg::PANIC_RAW_NO_MEMORY, op_params(op, obligation)));
        }
        // A place of the program whose frame has returned (decision 560).
        if let Some((true, _)) = place_at(s, p) {
            return Err((msg::PANIC_RAW_GONE, op_params(op, obligation)));
        }
        return Ok(None);
    };
    let block = &s.blocks[&base];
    if !block.live {
        return Err((msg::PANIC_RAW_RELEASED, op_params(op, obligation)));
    }
    if off + size > block.size {
        return Err((
            msg::PANIC_RAW_BOUNDS,
            vec![
                ("op", format!("ptr.{op}")),
                ("bytes", size.to_string()),
                ("offset", off.to_string()),
                ("size", block.size.to_string()),
                ("obligation", obligation.to_string()),
            ],
        ));
    }
    if !p.is_multiple_of(align.max(1)) {
        return Err((
            msg::PANIC_RAW_MISALIGNED,
            vec![
                ("op", format!("ptr.{op}")),
                ("ty", crate::rawmem::type_name(t)),
                ("offset", off.to_string()),
                ("align", align.to_string()),
            ],
        ));
    }
    Ok(Some((base, off)))
}

/// `p.offset(n)` of a pointer to values of `size` bytes: refused when it
/// leaves the block the pointer points into.
pub(crate) fn offset(p: u64, n: i64, size: u64) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    let bad = shadow(|s| {
        let (base, off) = find(s, p)?;
        let bsize = s.blocks[&base].size;
        let to = n.checked_mul(size as i64).and_then(|d| (off as i64).checked_add(d));
        let inside = to.is_some_and(|t| t >= 0 && t <= bsize as i64);
        (!inside).then_some((off, bsize))
    });
    match bad {
        None => Ok(()),
        Some((off, bsize)) => Err(refuse(
            msg::PANIC_RAW_OFFSET,
            &[("count", n.to_string()), ("offset", off.to_string()), ("size", bsize.to_string())],
        )),
    }
}

/// Checks an access by `op` of a value of type `t` at `p` that reads
/// nothing (a reference made, a slice copied): memory, liveness, bounds and
/// alignment, for `n` values.
pub(crate) fn touch(p: u64, t: u32, n: u64, op: &str) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    let l = crate::rawmem::layout(t);
    shadow(|s| access(s, p, l.size * n, l.align, t, op, "valid").map(|_| ()))
        .map_err(|(id, params)| refuse(id, &params))
}

/// Whether the model knows the bits of the values of `t` (the
/// interpreter's `encode`): numbers, `bool`, `char`, and the structs,
/// tuples and enums of them; not a pointer, a text or a collection.
fn encodes(t: u32, depth: u32) -> bool {
    if depth > 32 {
        return false;
    }
    match &ty(t).kind {
        Kind::Unit
        | Kind::Int
        | Kind::Sized(_)
        | Kind::U64
        | Kind::I128
        | Kind::U128
        | Kind::Float
        | Kind::F32
        | Kind::Bool
        | Kind::Char
        | Kind::DateTime => true,
        Kind::Tuple(slots) => slots.iter().all(|s| encodes(s.ty, depth + 1)),
        Kind::Struct { fields, .. } => fields.iter().all(|f| encodes(f.ty, depth + 1)),
        Kind::Option(p) => !matches!(ty(*p).kind, Kind::Ptr(_)) && encodes(*p, depth + 1),
        Kind::Result(a, b) => encodes(*a, depth + 1) && encodes(*b, depth + 1),
        Kind::Enum { variants, error: false, .. } => {
            variants.iter().all(|v| v.fields.iter().all(|f| encodes(f.ty, depth + 1)))
        }
        _ => false,
    }
}

/// Whether `t` lies in raw memory as a scalar or as parts of them (the
/// interpreter's `encodable`, which names what memory holds).
fn scalar_or_parts(t: u32) -> bool {
    match &ty(t).kind {
        Kind::Tuple(slots) => slots.iter().all(|s| scalar_or_parts(s.ty)),
        Kind::Struct { fields, .. } => fields.iter().all(|f| scalar_or_parts(f.ty)),
        Kind::Unit
        | Kind::Int
        | Kind::Sized(_)
        | Kind::U64
        | Kind::I128
        | Kind::U128
        | Kind::Float
        | Kind::F32
        | Kind::Bool
        | Kind::Char
        | Kind::DateTime => true,
        _ => false,
    }
}

/// What is wrong with the bits of a `t`: the bits that are no value of it,
/// or nothing to say of them (`None` from `Err`) when the model does not
/// know its values.
enum Bad {
    Invalid(String),
    Opaque,
}

/// Checks the bits of the `t` at `p`, as raw memory lays it out (`words`:
/// as a value lays it out, inside an enum).
///
/// # Safety
/// `p` must point to the bytes of a `t`'s layout.
unsafe fn bits(p: *const u8, t: u32, words: bool, depth: u32) -> Result<(), Bad> {
    if depth > 32 {
        return Err(Bad::Opaque);
    }
    // SAFETY: as the caller guarantees.
    let word = |off: u32| unsafe { (p.add(off as usize) as *const u64).read_unaligned() };
    match &ty(t).kind {
        Kind::Unit
        | Kind::Int
        | Kind::U64
        | Kind::I128
        | Kind::U128
        | Kind::Float
        | Kind::F32
        | Kind::DateTime => Ok(()),
        Kind::Sized(n) if words => {
            let i = word(0) as i64;
            let (bytes, signed) = match n.as_str() {
                "u8" => (1, false),
                "i8" => (1, true),
                "u16" => (2, false),
                "i16" => (2, true),
                "u32" => (4, false),
                _ => (4, true),
            };
            let b = 8 * bytes;
            let ok = if signed {
                i >= -(1i64 << (b - 1)) && i < (1i64 << (b - 1))
            } else {
                i >= 0 && i < (1i64 << b)
            };
            if ok { Ok(()) } else { Err(Bad::Invalid(format!("{:#x}", i as u64))) }
        }
        Kind::Sized(_) => Ok(()),
        Kind::Bool => match word(0) {
            0 | 1 => Ok(()),
            other => Err(Bad::Invalid(format!("{other:#x}"))),
        },
        Kind::Char => {
            let w = word(0);
            if u32::try_from(w).ok().and_then(char::from_u32).is_some() {
                Ok(())
            } else {
                Err(Bad::Invalid(format!("{w:#x}")))
            }
        }
        Kind::Tuple(slots) if words => {
            for s in slots {
                // SAFETY: the part lies inside the value.
                unsafe { bits(p.add(s.offset as usize), s.ty, true, depth + 1)? };
            }
            Ok(())
        }
        Kind::Struct { fields, .. } if words => {
            for f in fields {
                // SAFETY: as above.
                unsafe { bits(p.add(f.offset as usize), f.ty, true, depth + 1)? };
            }
            Ok(())
        }
        Kind::Tuple(_) | Kind::Struct { .. } => {
            for (moff, _, pt) in crate::rawmem::parts(ty(t)) {
                // SAFETY: the part lies inside the layout.
                unsafe { bits(p.add(moff as usize), pt, false, depth + 1)? };
            }
            Ok(())
        }
        Kind::Option(inner) if !matches!(ty(*inner).kind, Kind::Ptr(_)) => match word(0) {
            // SAFETY: the payload follows the tag.
            0 => unsafe { bits(p.add(8), *inner, true, depth + 1) },
            1 => Ok(()),
            tag => Err(Bad::Invalid(format!("the tag {tag}"))),
        },
        Kind::Result(a, b) => match word(0) {
            // SAFETY: as above.
            0 => unsafe { bits(p.add(8), *a, true, depth + 1) },
            // SAFETY: as above.
            1 => unsafe { bits(p.add(8), *b, true, depth + 1) },
            tag => Err(Bad::Invalid(format!("the tag {tag}"))),
        },
        Kind::Enum { variants, error: false, .. } => {
            let tag = word(0);
            let Some(v) = variants.get(tag as usize) else {
                return Err(Bad::Invalid(format!("the tag {tag}")));
            };
            for f in &v.fields {
                // SAFETY: as above.
                unsafe { bits(p.add(f.offset as usize), f.ty, true, depth + 1)? };
            }
            Ok(())
        }
        _ => Err(Bad::Opaque),
    }
}

/// Checks that the bytes at `p` are a valid `t` for `op` (`transmute`,
/// `bit_cast`, a read through bytes): the panic otherwise.
///
/// # Safety
/// `p` must point to the bytes of a `t`'s layout.
pub(crate) unsafe fn valid_bits(p: *const u8, t: u32, op: &str, obligation: &str) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    // SAFETY: as the caller guarantees.
    match unsafe { bits(p, t, false, 0) } {
        Err(Bad::Invalid(b)) => Err(refuse(
            msg::PANIC_RAW_INVALID_BITS,
            &[
                ("op", op.to_string()),
                ("bits", b),
                ("ty", crate::rawmem::type_name(t)),
                ("obligation", obligation.to_string()),
            ],
        )),
        _ => Ok(()),
    }
}

/// What the memory at `off` holds that is no `t`, for the panic.
fn held_name(block: &Block, off: u64) -> String {
    let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
    block
        .items
        .range(first..)
        .find_map(|(_, i)| match i {
            Item::Value(t) if !scalar_or_parts(*t) => Some(crate::rawmem::type_name(*t)),
            _ => None,
        })
        .unwrap_or_else(|| "value of another type".into())
}

fn mistyped(block: &Block, off: u64, t: u32, op: &str, obligation: &str) -> Report {
    (
        msg::PANIC_RAW_MISTYPED,
        vec![
            ("op", format!("ptr.{op}")),
            ("ty", crate::rawmem::type_name(t)),
            ("offset", off.to_string()),
            ("held", held_name(block, off)),
            ("obligation", obligation.to_string()),
        ],
    )
}

fn uninitialised(block: &Block, off: u64, t: u32, op: &str, obligation: &str) -> Report {
    (
        msg::PANIC_RAW_UNINITIALISED,
        vec![
            ("op", format!("ptr.{op}")),
            ("ty", crate::rawmem::type_name(t)),
            ("offset", off.to_string()),
            ("size", block.size.to_string()),
            ("obligation", obligation.to_string()),
        ],
    )
}

fn same_type(a: u32, b: u32) -> bool {
    a == b || (matches!(ty(a).kind, Kind::Ptr(_)) && matches!(ty(b).kind, Kind::Ptr(_)))
}

/// What a read finds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Found {
    /// A value of the type, taken when `take` and it needs a drop.
    Value,
    /// Bytes to decode, whose bits the caller checks.
    Bytes,
    /// Nothing at all.
    Nothing,
}

/// The interpreter's `slot_read` on the block's record: what is at `off`.
fn read_slot(block: &mut Block, off: u64, t: u32, take: bool, op: &str) -> Result<Found, Report> {
    if let Some(Item::Value(held)) = block.items.get(&off).copied()
        && same_type(held, t)
    {
        if take && rt::needs_drop(t) {
            block.items.remove(&off);
        }
        return Ok(Found::Value);
    }
    let size = crate::rawmem::layout(t).size;
    if size == 0 {
        return Ok(Found::Value);
    }
    let end = off + size;
    let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
    let mut covered = vec![false; size as usize];
    for (&at, item) in block.items.range(first..end) {
        match item {
            Item::Byte => {
                if at >= off {
                    covered[(at - off) as usize] = true;
                }
            }
            Item::Value(h) => {
                let hsize = crate::rawmem::layout(*h).size;
                if at + hsize <= off {
                    continue;
                }
                if !encodes(*h, 0) {
                    return Err(mistyped(block, off, t, op, "valid"));
                }
                for a in at.max(off)..(at + hsize).min(end) {
                    covered[(a - off) as usize] = true;
                }
            }
        }
    }
    if covered.iter().all(|c| !c) {
        if block.exposed {
            return Ok(Found::Bytes);
        }
        return Ok(Found::Nothing);
    }
    if covered.iter().any(|c| !c) && !block.exposed {
        return Err(uninitialised(block, off, t, op, "valid"));
    }
    if !encodes(t, 0) {
        return Err(mistyped(block, off, t, op, "valid"));
    }
    Ok(Found::Bytes)
}

/// The interpreter's `clear`: the `n` bytes from `off` hold nothing, the
/// bytes of plain values the range cuts kept; a value that needs a drop in
/// the range (outside `keep`, what a copy within the block moves) is the
/// panic of `op`.
fn clear(block: &mut Block, off: u64, n: u64, op: &str, keep: Option<(u64, u64)>) -> Result<(), Report> {
    let end = off + n;
    let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
    let mut hit = Vec::new();
    for (&at, item) in block.items.range(first..end) {
        let size = match item {
            Item::Byte => 1,
            Item::Value(t) => crate::rawmem::layout(*t).size,
        };
        if at + size > off || (size == 0 && at >= off) {
            hit.push(at);
        }
    }
    // A value never dropped is refused before anything changes, so that it
    // is still there at the end of the run (`let_go`).
    for &at in &hit {
        let Some(Item::Value(t)) = block.items.get(&at).copied() else { continue };
        let size = crate::rawmem::layout(t).size;
        let moved_from = keep.is_some_and(|(s, e)| at >= s && at + size <= e);
        if rt::needs_drop(t) && !moved_from {
            return Err((
                msg::PANIC_RAW_OVERWRITE,
                vec![
                    ("op", format!("ptr.{op}")),
                    ("ty", crate::rawmem::type_name(t)),
                    ("offset", at.to_string()),
                ],
            ));
        }
    }
    for at in hit {
        let Some(item) = block.items.remove(&at) else { continue };
        let Item::Value(t) = item else { continue };
        let size = crate::rawmem::layout(t).size;
        if (at < off || at + size > end) && encodes(t, 0) {
            for a in (at..at + size).filter(|a| *a < off || *a >= end) {
                block.items.insert(a, Item::Byte);
            }
        }
    }
    Ok(())
}

/// `p.read()` (`take`: the value moves out when its type needs a drop),
/// `p.read_volatile()` and the read of `drop_in_place`, of a `t`: refused
/// as the interpreter refuses it. The bits of a value read through bytes
/// are checked after, by [`valid_bits`] on the memory.
pub(crate) fn read(p: u64, t: u32, take: bool, op: &str) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    let l = crate::rawmem::layout(t);
    let r = shadow(|s| {
        let Some((base, off)) = access(s, p, l.size, l.align, t, op, "valid")? else {
            return Ok(Found::Value);
        };
        let Some(block) = s.blocks.get_mut(&base) else { rt::internal("a block of `Global` that went") };
        match read_slot(block, off, t, take, op)? {
            Found::Nothing => Err(uninitialised(block, off, t, op, "valid")),
            found => Ok(found),
        }
    });
    match r {
        Err((id, params)) => Err(refuse(id, &params)),
        // SAFETY: the access was checked inside a live block.
        Ok(Found::Bytes) => unsafe { valid_bits(p as *const u8, t, &format!("ptr.{op}"), "valid") },
        Ok(_) => Ok(()),
    }
}

/// `p.write(v)` of a `t` (`present`: a value is written, else the memory
/// holds none afterwards): refused as the interpreter refuses it.
pub(crate) fn write(p: u64, t: u32, present: bool, op: &str) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    let l = crate::rawmem::layout(t);
    shadow(|s| {
        let Some((base, off)) = access(s, p, l.size, l.align, t, op, "writable")? else { return Ok(()) };
        let Some(block) = s.blocks.get_mut(&base) else { rt::internal("a block of `Global` that went") };
        clear(block, off, l.size, op, None)?;
        if present && l.size > 0 {
            block.items.insert(off, Item::Value(t));
        }
        Ok(())
    })
    .map_err(|(id, params)| refuse(id, &params))
}

/// `p.drop_in_place()` of a `t`: the value at `p` taken out, refused when
/// there is none; afterwards the memory holds none.
pub(crate) fn drop_in_place(p: u64, t: u32) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    read(p, t, true, "drop_in_place")?;
    if !rt::needs_drop(t) {
        write(p, t, false, "drop_in_place")?;
    }
    Ok(())
}

/// `dst.copy_from(src, n)` of `t`s: each slot holds afterwards what the one
/// it was copied from held, a value that needs a drop the copy's alone.
pub(crate) fn copy_from(dst: u64, src: u64, n: i64, t: u32) -> Result<(), Refused> {
    if !on() {
        return Ok(());
    }
    let l = crate::rawmem::layout(t);
    let bytes = l.size * n.max(0) as u64;
    let op = "copy_from";
    shadow(|s| {
        let sb = access(s, src, bytes, l.align, t, op, "readable")?;
        let db = access(s, dst, bytes, l.align, t, op, "writable")?;
        if n <= 0 || bytes == 0 {
            return Ok(());
        }
        // What each slot of the source holds; a source the model does not
        // know holds values.
        let mut slots = vec![true; n as usize];
        if let Some((base, off)) = sb {
            let Some(block) = s.blocks.get_mut(&base) else { rt::internal("a block of `Global` that went") };
            for (k, slot) in slots.iter_mut().enumerate() {
                let at = off + k as u64 * l.size;
                *slot = read_slot(block, at, t, rt::needs_drop(t), op)? != Found::Nothing;
            }
        }
        if let Some((base, off)) = db {
            let keep = (sb.map(|(b, _)| b) == Some(base)).then(|| {
                let so = sb.map_or(0, |(_, o)| o);
                (so, so + bytes)
            });
            let Some(block) = s.blocks.get_mut(&base) else { rt::internal("a block of `Global` that went") };
            clear(block, off, bytes, op, keep)?;
            for (k, present) in slots.into_iter().enumerate() {
                if present && l.size > 0 {
                    block.items.insert(off + k as u64 * l.size, Item::Value(t));
                }
            }
        }
        Ok(())
    })
    .map_err(|(id, params)| refuse(id, &params))
}

// ------------------------------------------------------------ memory C gives out

thread_local! {
    /// The status of the last checked C call (decision 561): what
    /// `sisu_rt_c_checked` gives the code after the call.
    static PENDING: std::cell::Cell<u8> = const { std::cell::Cell::new(crate::abi::OK) };
}

/// The checked C call refused what it was given: its panic is raised, and
/// the code after the call takes the failure path.
fn pend(r: Result<(), Refused>) -> bool {
    match r {
        Ok(()) => true,
        Err(st) => {
            PENDING.with(|p| p.set(st));
            false
        }
    }
}

/// After a checked C call: whether it refused what it was given, which the
/// generated code branches on (decision 561).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_c_checked() -> u8 {
    PENDING.with(|p| p.replace(crate::abi::OK))
}

/// A pointer a C function is given that may not be null: the panic of `op`
/// otherwise, as the interpreter's emulation raises it.
fn not_null(p: u64, op: &str) -> Result<(), Refused> {
    if p == 0 {
        return Err(refuse(
            msg::PANIC_RAW_NO_MEMORY,
            &[("op", op.to_string()), ("obligation", "valid".to_string())],
        ));
    }
    Ok(())
}

/// The `n` bytes at `p` that C reads as plain data, for `op`: live and in
/// bounds, and holding bytes unless foreign code may have written them.
fn plain_read(s: &mut Shadow, p: u64, n: u64, op: &str) -> Result<(), Report> {
    let Some((base, off)) = access(s, p, n, 1, 0, op, "valid")? else { return Ok(()) };
    let block = &s.blocks[&base];
    if block.exposed || n == 0 {
        return Ok(());
    }
    let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
    let mut covered = vec![false; n as usize];
    for (&at, item) in block.items.range(first..off + n) {
        let size = match item {
            Item::Byte => 1,
            Item::Value(t) if encodes(*t, 0) => crate::rawmem::layout(*t).size,
            Item::Value(_) => continue,
        };
        for a in at.max(off)..(at + size).min(off + n) {
            covered[(a - off) as usize] = true;
        }
    }
    match covered.iter().position(|c| !c) {
        None => Ok(()),
        Some(k) => Err((
            msg::PANIC_RAW_UNINITIALISED,
            vec![
                ("op", format!("ptr.{op}")),
                ("ty", "u8".to_string()),
                ("offset", (off + k as u64).to_string()),
                ("size", block.size.to_string()),
                ("obligation", "valid".to_string()),
            ],
        )),
    }
}

/// The text of a C string at `p` that `op` reads: its bytes up to the
/// first NUL inside its block, if the model knows its memory.
///
/// # Safety
/// `p` must point to memory the program may read.
unsafe fn c_string(p: u64, op: &str) -> Result<(), Refused> {
    not_null(p, op)?;
    shadow(|s| {
        let Some((base, off)) = find(s, p) else { return Ok(()) };
        let size = s.blocks[&base].size;
        let mut k = off;
        while k < size {
            plain_read(s, base + k, 1, op)?;
            // SAFETY: the byte lies inside a live block.
            if unsafe { *((base + k) as *const u8) } == 0 {
                return Ok(());
            }
            k += 1;
        }
        // The byte past the block's end is read next.
        access(s, base + k, 1, 1, 0, op, "valid").map(|_| ())
    })
    .map_err(|(id, params)| refuse(id, &params))
}

/// C's `malloc`, checked (decision 561): the block it gives out is one the
/// model knows, which `free` alone takes back.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_c_malloc(n: u64) -> *mut u8 {
    // SAFETY: `malloc` takes any size.
    let p = unsafe { libc::malloc(n as usize) } as *mut u8;
    if on() && !p.is_null() {
        shadow(|s| {
            s.blocks.insert(
                p as u64,
                Block {
                    origin: Origin::Malloc,
                    size: n,
                    align: 16,
                    live: true,
                    exposed: false,
                    items: BTreeMap::new(),
                },
            );
        });
    }
    p
}

/// C's `free`, checked: refused unless the pointer is the start of a live
/// block `malloc` gave out that holds no value never dropped.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_c_free(p: *mut u8) {
    if p.is_null() {
        return;
    }
    if !on() {
        // SAFETY: the program frees what it says `malloc` gave out.
        unsafe { libc::free(p as *mut libc::c_void) };
        return;
    }
    // The model keeps the block's memory for a while, or refused.
    let _ = pend(let_out(p, None).map(|_| ()));
}

/// C's `getenv`, checked: the name read as a C string, and the text it
/// gives back known as memory to read only.
///
/// # Safety
/// `name` must point to memory the program may read.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_c_getenv(name: *const libc::c_char) -> *const libc::c_char {
    // SAFETY: as the caller guarantees.
    if !pend(unsafe { c_string(name as u64, "getenv") }) {
        return std::ptr::null();
    }
    // SAFETY: the name is a C string.
    let v = unsafe { libc::getenv(name) };
    if on() && !v.is_null() {
        // SAFETY: `getenv` gives a C string.
        let len = unsafe { libc::strlen(v) } as u64 + 1;
        shadow(|s| {
            s.blocks.insert(
                v as u64,
                Block {
                    origin: Origin::ReadOnly,
                    size: len,
                    align: 1,
                    live: true,
                    exposed: true,
                    items: BTreeMap::new(),
                },
            );
        });
    }
    v
}

/// C's `strlen`, checked: the text read up to its NUL inside its block.
///
/// # Safety
/// `s` must point to memory the program may read.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_c_strlen(text: *const libc::c_char) -> u64 {
    // SAFETY: as the caller guarantees.
    if !pend(unsafe { c_string(text as u64, "strlen") }) {
        return 0;
    }
    // SAFETY: the text is a C string.
    unsafe { libc::strlen(text) as u64 }
}

/// C's `memcpy`, checked: `n` bytes read from `src` as plain data and
/// written at `dst`, which holds them afterwards, the ranges not
/// overlapping.
///
/// # Safety
/// The ranges must be memory the program may read and write.
#[unsafe(no_mangle)]
unsafe extern "C" fn sisu_rt_c_memcpy(dst: *mut u8, src: *const u8, n: u64) -> *mut u8 {
    let checked =
        not_null(dst as u64, "memcpy").and_then(|()| not_null(src as u64, "memcpy")).and_then(|()| {
            if !on() {
                return Ok(());
            }
            shadow(|s| {
                let (d, r) = (find(s, dst as u64), find(s, src as u64));
                if n > 0
                    && let (Some((db, doff)), Some((sb, soff))) = (d, r)
                    && db == sb
                    && doff.abs_diff(soff) < n
                {
                    return Err((msg::PANIC_FOREIGN_OVERLAP, vec![("bytes", n.to_string())]));
                }
                plain_read(s, src as u64, n, "memcpy")?;
                let Some((base, off)) = access(s, dst as u64, n, 1, 0, "memcpy", "writable")? else {
                    return Ok(());
                };
                let block = s.blocks.get_mut(&base).unwrap_or_else(|| rt::internal("a block that went"));
                if block.origin == Origin::ReadOnly {
                    return Err((msg::PANIC_RAW_SHARED_WRITE, op_params("memcpy", "writable")));
                }
                clear(block, off, n, "memcpy", None)?;
                for k in 0..n {
                    block.items.insert(off + k, Item::Byte);
                }
                Ok(())
            })
            .map_err(|(id, params)| refuse(id, &params))
        });
    if !pend(checked) {
        return dst;
    }
    // SAFETY: as the caller guarantees, the ranges checked where known.
    unsafe { libc::memcpy(dst as *mut libc::c_void, src as *const libc::c_void, n as usize) as *mut u8 }
}
