//! Raw memory in native code (22, "Raw pointers", "Plain data and
//! reinterpretation"; decision 302): the intrinsics of std.mem on the
//! machine's memory, and the layout of a type in raw memory.
//!
//! The layout is the interpreter's (`sisu-interp/src/rawmem.rs`), so that a
//! program computes the same sizes either way: a sized integer narrower than
//! 64 bits and an `f32` take their own width, a struct and a tuple their
//! parts in C's order with their padding, an option of a raw pointer one
//! word (null for `None`), and every other type the words of its value
//! (decision 188). A value whose layout in memory is its layout as a value
//! is read and written as its bytes; another is converted part by part. The
//! interpreter checks every access against its checked memory model (D-085);
//! native code makes none of those checks, as 22 has it: the raw memory
//! operations are the unchecked ones, and the interpreter is where unsafe
//! code is checked. The panics native code does raise are those of the safe
//! operations (`read_plain` past the end) and of sizes the front end does not
//! check yet (`bit_cast`, `transmute`), with the interpreter's messages.

use crate::abi::{Intrinsic, Kind, TypeEntry};
use crate::intrinsics::Call;
use crate::mem::{self, List, Text};
use crate::rt::{self, ty};
use crate::value;
use sisu_diag::msg;
use std::sync::OnceLock;

/// The size and alignment of a type in raw memory, and whether it lies
/// there as its value does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub size: u64,
    pub align: u64,
    pub same: bool,
}

/// The layout of every type of the program, by number.
static LAYOUTS: OnceLock<Vec<Layout>> = OnceLock::new();

fn align_up(n: u64, align: u64) -> u64 {
    n.div_ceil(align.max(1)) * align.max(1)
}

/// Computes the layouts of the type table, once, when the program starts.
pub(crate) fn set_up(types: &[TypeEntry]) {
    let mut memo: Vec<Option<Layout>> = vec![None; types.len()];
    for t in 0..types.len() {
        compute(types, &mut memo, t as u32, 0);
    }
    let _ = LAYOUTS.set(memo.into_iter().map(|l| l.unwrap_or(WORDS_NONE)).collect());
}

const WORDS_NONE: Layout = Layout { size: 0, align: 1, same: true };

fn compute(types: &[TypeEntry], memo: &mut [Option<Layout>], t: u32, depth: u32) -> Layout {
    if let Some(Some(l)) = memo.get(t as usize) {
        return *l;
    }
    let Some(entry) = types.get(t as usize) else { return WORDS_NONE };
    let words = Layout { size: u64::from(entry.size), align: 8, same: true };
    let l = if depth > 64 {
        words
    } else {
        match &entry.kind {
            Kind::Unit => Layout { size: 0, align: 1, same: true },
            Kind::Sized(n) => {
                let w = width(n);
                Layout { size: w, align: w, same: false }
            }
            Kind::F32 => Layout { size: 4, align: 4, same: false },
            Kind::Option(p) if matches!(types.get(*p as usize).map(|e| &e.kind), Some(Kind::Ptr(_))) => {
                Layout { size: 8, align: 8, same: false }
            }
            Kind::Tuple(slots) => {
                let parts: Vec<u32> = slots.iter().map(|s| s.ty).collect();
                c_layout(types, memo, &parts, depth)
            }
            Kind::Struct { fields, .. } => {
                let parts: Vec<u32> = fields.iter().map(|f| f.ty).collect();
                c_layout(types, memo, &parts, depth)
            }
            _ => words,
        }
    };
    if let Some(slot) = memo.get_mut(t as usize) {
        *slot = Some(l);
    }
    l
}

/// Parts in C's order, each at the next offset its alignment allows.
fn c_layout(types: &[TypeEntry], memo: &mut [Option<Layout>], parts: &[u32], depth: u32) -> Layout {
    let (mut off, mut align, mut same) = (0, 1, true);
    for &p in parts {
        let l = compute(types, memo, p, depth + 1);
        off = align_up(off, l.align);
        off += l.size;
        align = align.max(l.align);
        same &= l.same;
    }
    Layout { size: align_up(off, align), align, same }
}

/// The bytes of a sized integer type narrower than 64 bits.
fn width(name: &str) -> u64 {
    match name {
        "u8" | "i8" => 1,
        "u16" | "i16" => 2,
        _ => 4,
    }
}

/// The layout of type `t` in raw memory.
pub(crate) fn layout(t: u32) -> Layout {
    LAYOUTS.get().and_then(|l| l.get(t as usize)).copied().unwrap_or(WORDS_NONE)
}

/// The memory offsets of a struct's or tuple's parts, with their types and
/// their offsets in the value.
fn parts(entry: &TypeEntry) -> Vec<(u64, u64, u32)> {
    let value_parts: Vec<(u64, u32)> = match &entry.kind {
        Kind::Tuple(slots) => slots.iter().map(|s| (u64::from(s.offset), s.ty)).collect(),
        Kind::Struct { fields, .. } => fields.iter().map(|f| (u64::from(f.offset), f.ty)).collect(),
        _ => Vec::new(),
    };
    let mut off = 0;
    let mut out = Vec::with_capacity(value_parts.len());
    for (voff, t) in value_parts {
        let l = layout(t);
        off = align_up(off, l.align);
        out.push((off, voff, t));
        off += l.size;
    }
    out
}

/// Writes the value of type `t` at `src` at `dst` as raw memory holds it.
///
/// # Safety
/// `src` must hold a value of type `t`, and `dst` have room for its layout.
pub(crate) unsafe fn to_memory(dst: *mut u8, src: *const u8, t: u32) {
    let l = layout(t);
    // SAFETY: as the caller guarantees.
    unsafe {
        if l.same {
            mem::copy(dst, src, l.size as usize);
            return;
        }
        let entry = ty(t);
        match &entry.kind {
            Kind::Sized(n) => mem::copy(dst, src, width(n) as usize),
            Kind::F32 => mem::copy(dst, src, 4),
            Kind::Option(_) => {
                let p = if mem::word(src, 0) == 0 { mem::word(src, 8) } else { 0 };
                (dst as *mut u64).write_unaligned(p);
            }
            Kind::Tuple(_) | Kind::Struct { .. } => {
                for (moff, voff, pt) in parts(entry) {
                    to_memory(dst.add(moff as usize), src.add(voff as usize), pt);
                }
            }
            _ => mem::copy(dst, src, l.size as usize),
        }
    }
}

/// Writes at `dst` the value of type `t` that raw memory holds at `src`.
///
/// # Safety
/// `src` must hold the layout of a value of type `t`, and `dst` have room
/// for the value.
pub(crate) unsafe fn to_value(dst: *mut u8, src: *const u8, t: u32) {
    let l = layout(t);
    // SAFETY: as the caller guarantees.
    unsafe {
        if l.same {
            mem::copy(dst, src, l.size as usize);
            return;
        }
        let entry = ty(t);
        match &entry.kind {
            Kind::Sized(n) => {
                let w = width(n) as usize;
                let mut b = [0u8; 8];
                std::ptr::copy_nonoverlapping(src, b.as_mut_ptr(), w);
                let mut v = i64::from_le_bytes(b);
                if n.starts_with('i') {
                    let shift = 64 - 8 * w as u32;
                    v = (v << shift) >> shift;
                }
                mem::set_word(dst, 0, v as u64);
            }
            Kind::F32 => {
                let mut b = [0u8; 4];
                std::ptr::copy_nonoverlapping(src, b.as_mut_ptr(), 4);
                mem::set_word(dst, 0, u64::from(u32::from_le_bytes(b)));
            }
            Kind::Option(_) => {
                let p = (src as *const u64).read_unaligned();
                mem::set_word(dst, 0, u64::from(p == 0));
                mem::set_word(dst, 8, p);
            }
            Kind::Tuple(_) | Kind::Struct { .. } => {
                for (moff, voff, pt) in parts(entry) {
                    to_value(dst.add(voff as usize), src.add(moff as usize), pt);
                }
            }
            _ => mem::copy(dst, src, l.size as usize),
        }
    }
}

/// Whether the intrinsic is raw memory's.
pub(crate) fn handles(i: Intrinsic) -> bool {
    i.name().starts_with("mem.")
}

/// What a raw pointer of type `t` (through references) points to.
fn pointee(mut t: u32) -> u32 {
    loop {
        match &ty(t).kind {
            Kind::Ref(inner) => t = *inner,
            Kind::Ptr(e) => return *e,
            _ => rt::internal("a raw pointer's operation on something that is not one"),
        }
    }
}

/// The word a pointer argument holds.
///
/// # Safety
/// Argument `i` must be a raw pointer, or a reference to one.
unsafe fn pointer(c: &Call<'_>, i: usize) -> *mut u8 {
    // SAFETY: as the caller guarantees.
    unsafe { c.word(i) as *mut u8 }
}

/// A block of `n` words for a value, aligned for any of them.
fn scratch(bytes: u64) -> Vec<u64> {
    vec![0u64; (bytes as usize).div_ceil(8).max(1)]
}

/// A call of a raw memory intrinsic.
///
/// # Safety
/// As `sisu_rt_intrinsic`: the arguments are of the types the descriptor
/// gives, and the program meets the obligations the intrinsics' `Safety:`
/// lists state.
pub(crate) unsafe fn call(i: Intrinsic, c: &Call<'_>) -> u8 {
    use Intrinsic as I;
    // SAFETY: as the caller guarantees.
    unsafe {
        match i {
            // A pointer to a place: the address the reference holds.
            I::MemPtrOf | I::MemPtrOfMut => c.set_word(mem::word(c.arg(0), 0)),
            I::MemPtrDangling => c.set_word(layout(c.targ(0)).align),
            I::MemPtrFromAddress | I::MemPtrWithAddr => c.set_word(c.int(c.args.len() - 1) as u64),
            I::MemPtrCast
            | I::MemPtrAsConst
            | I::MemPtrCastMut
            | I::MemPtrAddr
            | I::MemPtrAsRef
            | I::MemPtrAsMut => c.set_word(pointer(c, 0) as u64),
            I::MemPtrOffset => {
                let size = layout(pointee(c.ty(0))).size as i64;
                let p = pointer(c, 0) as i64;
                c.set_word(p.wrapping_add(c.int(1).wrapping_mul(size)) as u64);
            }
            I::MemPtrRead | I::MemPtrReadVolatile => to_value(c.ret, pointer(c, 0), pointee(c.ty(0))),
            I::MemPtrWrite | I::MemPtrWriteVolatile => {
                let e = pointee(c.ty(0));
                // The value itself, or a reference to it for a `Copy` one
                // passed by reference.
                let src = if c.ty(1) == e { c.arg(1) } else { value::target(c.arg(1), c.ty(1)).0 };
                to_memory(pointer(c, 0), src, e);
            }
            I::MemPtrDropInPlace => {
                let e = pointee(c.ty(0));
                if !rt::needs_drop(e) {
                    return crate::abi::OK;
                }
                let mut buf = scratch(u64::from(ty(e).size));
                let tmp = buf.as_mut_ptr() as *mut u8;
                to_value(tmp, pointer(c, 0), e);
                return value::drop_value(tmp, e);
            }
            I::MemPtrCopyFrom => {
                let size = layout(pointee(c.ty(0))).size;
                let n = c.int(2).max(0) as u64;
                mem::copy(pointer(c, 0), pointer(c, 1), (size * n) as usize);
            }
            I::MemSliceAsPtr => {
                let (p, t) = c.place(0);
                let e = match &ty(t).kind {
                    Kind::List(e) => *e,
                    _ => rt::internal("`as_ptr` of something that is not a slice"),
                };
                let list = mem::at::<List>(p);
                let addr = if list.ptr.is_null() { layout(e).align } else { list.ptr as u64 };
                c.set_word(addr);
            }
            I::MemSizeOf => c.set_word(layout(c.targ(0)).size),
            I::MemAlignOf => c.set_word(layout(c.targ(0)).align),
            I::MemGlobalAcquire => {
                let (p, _) = c.place(1);
                let (size, align) = (mem::word(p, 0) as i64, mem::word(p, 8) as i64);
                let block = if size < 0 || align <= 0 || align & (align - 1) != 0 {
                    std::ptr::null_mut()
                } else {
                    mem::alloc_aligned(size as usize, align as usize)
                };
                if block.is_null() {
                    c.none();
                } else {
                    mem::set_word(c.ret, 0, 0);
                    mem::set_word(c.ret, 8, block as u64);
                }
            }
            I::MemGlobalRelease => mem::release(pointer(c, 1)),
            I::MemBitCast | I::MemTransmute => {
                let (to, from) = (c.targ(0), c.targ(1));
                let (lt, lf) = (layout(to), layout(from));
                if lt.size != lf.size {
                    let name = if i == I::MemBitCast { "bit_cast" } else { "transmute" };
                    return rt::raise_msg(
                        msg::PANIC_RAW_SIZES,
                        &[
                            ("op", name.to_string()),
                            ("from", type_name(from)),
                            ("from_size", lf.size.to_string()),
                            ("to", type_name(to)),
                            ("to_size", lt.size.to_string()),
                        ],
                    );
                }
                let src = if c.ty(0) == from { c.arg(0) } else { value::target(c.arg(0), c.ty(0)).0 };
                let mut buf = scratch(lf.size);
                let tmp = buf.as_mut_ptr() as *mut u8;
                to_memory(tmp, src, from);
                to_value(c.ret, tmp, to);
            }
            I::MemAsBytes => {
                let (p, t) = c.place(0);
                let l = layout(t);
                let mut buf = scratch(l.size);
                let tmp = buf.as_mut_ptr() as *mut u8;
                to_memory(tmp, p, t);
                let bytes = std::slice::from_raw_parts(tmp, l.size as usize);
                (c.ret as *mut Text).write(Text::from_bytes(bytes));
            }
            I::MemReadPlain => {
                let t = c.targ(0);
                let data = bytes_arg(c, 0);
                let at = c.int(1);
                let l = layout(t);
                let end = at.checked_add(l.size as i64);
                let Some(part) = end.filter(|_| at >= 0).and_then(|e| data.get(at as usize..e as usize))
                else {
                    return rt::raise_msg(
                        msg::PANIC_RAW_PLAIN_SHORT,
                        &[
                            ("ty", type_name(t)),
                            ("size", l.size.to_string()),
                            ("at", at.to_string()),
                            ("len", data.len().to_string()),
                        ],
                    );
                };
                let mut buf = scratch(l.size);
                let tmp = buf.as_mut_ptr() as *mut u8;
                std::ptr::copy_nonoverlapping(part.as_ptr(), tmp, part.len());
                to_value(c.ret, tmp, t);
            }
            I::MemViewPlain => {
                let t = c.targ(0);
                let data = bytes_arg(c, 0);
                let l = layout(t);
                let start = data.as_ptr() as u64;
                if (data.len() as u64) < l.size || (l.size > 0 && !start.is_multiple_of(l.align)) {
                    c.none();
                    return crate::abi::OK;
                }
                // The bytes in place; for a type laid out otherwise in a
                // value, a copy as a value, which goes with the bytes.
                let at = if l.same || l.size == 0 {
                    start
                } else {
                    let copy = mem::alloc(ty(t).size as usize);
                    to_value(copy, data.as_ptr(), t);
                    crate::views::hold(data.as_ptr() as *mut u8, copy);
                    copy as u64
                };
                mem::set_word(c.ret, 0, 0);
                mem::set_word(c.ret, 8, at);
            }
            I::MemSliceFromRaw => {
                let e = pointee(c.ty(0));
                let n = c.int(1).max(0) as u64;
                let (msize, vsize) = (layout(e).size, ty(e).size);
                let mut list = List::empty();
                list.reserve(n, vsize);
                let mut buf = scratch(u64::from(vsize));
                let tmp = buf.as_mut_ptr() as *mut u8;
                let p = pointer(c, 0);
                for k in 0..n {
                    to_value(tmp, p.add((k * msize) as usize), e);
                    value::copy_value(list.at(k, vsize), tmp, e);
                }
                list.len = n;
                (c.ret as *mut List).write(list);
            }
            _ => rt::internal("an intrinsic that is not raw memory's"),
        }
    }
    crate::abi::OK
}

/// A type's name as the interpreter's messages write it, for the types a
/// plain value is made of; another is shown by its kind.
fn type_name(t: u32) -> String {
    match &ty(t).kind {
        Kind::Unit => "()".into(),
        Kind::Int => "int".into(),
        Kind::Sized(n) => n.clone(),
        Kind::U64 => "u64".into(),
        Kind::I128 => "i128".into(),
        Kind::U128 => "u128".into(),
        Kind::Float => "f64".into(),
        Kind::F32 => "f32".into(),
        Kind::Bool => "bool".into(),
        Kind::Char => "char".into(),
        Kind::DateTime => "DateTime".into(),
        Kind::Struct { name, .. } | Kind::Enum { name, .. } => name.clone(),
        Kind::Tuple(slots) => {
            let parts: Vec<String> = slots.iter().map(|s| type_name(s.ty)).collect();
            format!("({})", parts.join(", "))
        }
        Kind::Ptr(e) => format!("ptr[{}]", type_name(*e)),
        other => format!("{other:?}"),
    }
}

/// The bytes a `bytes` argument (or a reference to one) holds.
///
/// # Safety
/// Argument `i` must be `bytes`, or a reference to one, which outlives the
/// slice.
unsafe fn bytes_arg<'a>(c: &Call<'_>, i: usize) -> &'a [u8] {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = c.place(i);
        match &ty(t).kind {
            Kind::Bytes | Kind::String | Kind::Path => mem::at::<Text>(p).bytes(),
            _ => rt::internal("plain data read from something that is not `bytes`"),
        }
    }
}

/// What the program printed so far, written, before a foreign call that
/// may write to the same streams (decision 302).
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_flush() {
    crate::sched::system(crate::rt::flush);
}

thread_local! {
    /// The thread's slot (22, "Machine intrinsics"): one word, which the
    /// runtime written in Sisu points at the thread's runtime value.
    static SLOT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The address of the thread's slot, the machine intrinsic
/// `rt.thread_slot`.
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_thread_slot() -> u64 {
    SLOT.with(|s| s.as_ptr() as u64)
}

// `rt.switch(save, load)` (22, "Machine intrinsics"): pushes the registers
// the C calling convention keeps across a call, saves the stack pointer,
// the frame pointer and the address to resume at in the `Context` at
// `save` (three words: `sp`, `fp`, `ip`), and continues from the context
// at `load`. A context it saved resumes at its second half, which pops the
// registers it pushed and returns from the `switch` that saved it.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
std::arch::global_asm!(
    ".globl sisu_rt_switch",
    ".type sisu_rt_switch, @function",
    ".p2align 4",
    "sisu_rt_switch:",
    "push rbx",
    "push rbp",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "mov [rdi], rsp",
    "mov [rdi + 8], rbp",
    "lea rax, [rip + 2f]",
    "mov [rdi + 16], rax",
    "mov rsp, [rsi]",
    "mov rbp, [rsi + 8]",
    "jmp qword ptr [rsi + 16]",
    "2:",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop rbp",
    "pop rbx",
    "ret",
    ".size sisu_rt_switch, . - sisu_rt_switch",
);

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
std::arch::global_asm!(
    ".globl sisu_rt_switch",
    ".type sisu_rt_switch, %function",
    ".p2align 2",
    "sisu_rt_switch:",
    "sub sp, sp, #176",
    "stp x19, x20, [sp, #0]",
    "stp x21, x22, [sp, #16]",
    "stp x23, x24, [sp, #32]",
    "stp x25, x26, [sp, #48]",
    "stp x27, x28, [sp, #64]",
    "stp x29, x30, [sp, #80]",
    "stp d8, d9, [sp, #96]",
    "stp d10, d11, [sp, #112]",
    "stp d12, d13, [sp, #128]",
    "stp d14, d15, [sp, #144]",
    "mov x9, sp",
    "str x9, [x0]",
    "str x29, [x0, #8]",
    "adr x9, 2f",
    "str x9, [x0, #16]",
    "ldr x9, [x1]",
    "mov sp, x9",
    "ldr x29, [x1, #8]",
    "ldr x9, [x1, #16]",
    "br x9",
    "2:",
    "ldp x19, x20, [sp, #0]",
    "ldp x21, x22, [sp, #16]",
    "ldp x23, x24, [sp, #32]",
    "ldp x25, x26, [sp, #48]",
    "ldp x27, x28, [sp, #64]",
    "ldp x29, x30, [sp, #80]",
    "ldp d8, d9, [sp, #96]",
    "ldp d10, d11, [sp, #112]",
    "ldp d12, d13, [sp, #128]",
    "ldp d14, d15, [sp, #144]",
    "add sp, sp, #176",
    "ret",
    ".size sisu_rt_switch, . - sisu_rt_switch",
);

/// On a target without the switch's instructions, `rt.switch` stops the
/// program.
#[cfg(not(all(any(target_arch = "x86_64", target_arch = "aarch64"), target_os = "linux")))]
#[unsafe(no_mangle)]
extern "C" fn sisu_rt_switch(_save: u64, _load: u64) {
    crate::rt::internal("`rt.switch` has no instructions on this target yet");
}

#[cfg(all(test, target_arch = "x86_64", target_os = "linux"))]
mod tests {
    use std::cell::Cell;

    /// `std.rt`'s `Context`.
    #[repr(C)]
    struct Context {
        sp: u64,
        fp: u64,
        ip: u64,
    }

    unsafe extern "C" {
        fn sisu_rt_switch(save: *mut Context, load: *const Context);
    }

    static mut MAIN: Context = Context { sp: 0, fp: 0, ip: 0 };
    static mut OTHER: Context = Context { sp: 0, fp: 0, ip: 0 };

    thread_local! {
        static HITS: Cell<u32> = const { Cell::new(0) };
    }

    /// Runs on a stack of its own: counts a visit and switches back, for
    /// ever.
    extern "C" fn visit() -> ! {
        loop {
            HITS.with(|h| h.set(h.get() + 1));
            // SAFETY: MAIN was saved by the switch that came here.
            unsafe { sisu_rt_switch(&raw mut OTHER, &raw const MAIN) };
        }
    }

    #[test]
    fn a_switch_runs_another_stack_and_comes_back() {
        let mut stack = vec![0u64; 16 * 1024];
        let top = (stack.as_mut_ptr() as u64 + 8 * stack.len() as u64) & !15;
        // SAFETY: the other stack is live for the whole test, and its first
        // context enters `visit` as a call would, the stack pointer just
        // below a return address's word.
        unsafe {
            OTHER = Context { sp: top - 8, fp: 0, ip: visit as *const () as usize as u64 };
            for _ in 0..3 {
                sisu_rt_switch(&raw mut MAIN, &raw const OTHER);
            }
        }
        assert_eq!(HITS.with(Cell::get), 3);
        drop(stack);
    }
}
