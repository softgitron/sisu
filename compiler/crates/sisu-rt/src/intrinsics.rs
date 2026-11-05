//! The intrinsics, by [`Intrinsic`]: each implements the contract its
//! `@intrinsic` declaration in compiler/std states, with the interpreter's
//! results, messages and order of effects (`sisu-interp/src/intrinsics.rs`).
//!
//! Arguments arrive as the interpreter's do: a parameter the prelude takes
//! by borrow is a reference (or a copy, for a `Copy` type), a `mut` one a
//! reference, an `own` one the value, moved in. The accessors look through
//! references, so an intrinsic does not care which it got.

use crate::abi::{self, Intrinsic, Kind};
use crate::iter;
use crate::mem::{self, List, Str, Text};
use crate::rt::{self, ty};
use crate::value::{self, target};
use sisu_diag::msg;
use std::cmp::Ordering;

pub(crate) struct Call<'a> {
    pub ret: *mut u8,
    pub ret_ty: u32,
    pub args: &'a [*mut u8],
    pub tys: &'a [u32],
    pub targs: &'a [u32],
}

impl Call<'_> {
    pub(crate) fn arg(&self, i: usize) -> *mut u8 {
        self.args.get(i).copied().unwrap_or_else(|| rt::internal("an intrinsic missing an argument"))
    }

    pub(crate) fn ty(&self, i: usize) -> u32 {
        self.tys.get(i).copied().unwrap_or(0)
    }

    pub(crate) fn targ(&self, i: usize) -> u32 {
        self.targs.get(i).copied().unwrap_or_else(|| rt::internal("an intrinsic missing a type argument"))
    }

    /// The item type of an `Iterable[T]` argument, when the call gives it.
    fn item(&self, i: usize) -> Option<u32> {
        self.targs.get(i).copied()
    }

    /// The value argument `i` is or refers to, and its type.
    pub(crate) unsafe fn place(&self, i: usize) -> (*mut u8, u32) {
        // SAFETY: the argument holds a value of its type.
        unsafe { target(self.arg(i), self.ty(i)) }
    }

    pub(crate) unsafe fn word(&self, i: usize) -> u64 {
        // SAFETY: as `place`.
        unsafe { mem::word(self.place(i).0, 0) }
    }

    pub(crate) unsafe fn int(&self, i: usize) -> i64 {
        // SAFETY: as `place`.
        unsafe { self.word(i) as i64 }
    }

    unsafe fn float(&self, i: usize) -> f64 {
        // SAFETY: as `place`.
        unsafe { f64::from_bits(self.word(i)) }
    }

    unsafe fn f32(&self, i: usize) -> f32 {
        // SAFETY: as `place`; an `f32` is the low half of its word.
        f32::from_bits(unsafe { self.word(i) } as u32)
    }

    unsafe fn set_f32(&self, f: f32) {
        // SAFETY: the result is an `f32`'s word.
        unsafe { self.set_word(u64::from(f.to_bits())) }
    }

    pub(crate) unsafe fn bool(&self, i: usize) -> bool {
        // SAFETY: as `place`.
        unsafe { self.word(i) != 0 }
    }

    unsafe fn char(&self, i: usize) -> char {
        // SAFETY: as `place`.
        char::from_u32(unsafe { self.word(i) } as u32).unwrap_or('\u{fffd}')
    }

    pub(crate) unsafe fn text<'b>(&self, i: usize) -> &'b [u8] {
        // SAFETY: as `place`.
        unsafe { value::text(self.arg(i), self.ty(i)).unwrap_or(&[]) }
    }

    pub(crate) unsafe fn set_word(&self, v: u64) {
        // SAFETY: the result has room for a word.
        unsafe { mem::set_word(self.ret, 0, v) }
    }

    pub(crate) unsafe fn set_bool(&self, b: bool) {
        // SAFETY: as `set_word`.
        unsafe { self.set_word(u64::from(b)) }
    }

    unsafe fn set_string(&self, b: &[u8]) {
        // SAFETY: the result is a `String`.
        unsafe { value::write_string(self.ret, b) }
    }

    pub(crate) unsafe fn none(&self) {
        // SAFETY: the result is an option.
        unsafe { mem::set_word(self.ret, 0, 1) }
    }

    /// `Some` of a word.
    unsafe fn some_word(&self, v: u64) {
        // SAFETY: the result is an option of a one-word value.
        unsafe {
            mem::set_word(self.ret, 0, 0);
            mem::set_word(self.ret, 8, v);
        }
    }

    unsafe fn ordering(&self, o: Ordering) {
        let tag = match o {
            Ordering::Less => 0,
            Ordering::Equal => 1,
            Ordering::Greater => 2,
        };
        // SAFETY: the result is an `Ordering`.
        unsafe { self.set_word(tag) }
    }
}

/// The call a descriptor and its arguments' addresses describe, and the
/// intrinsic.
///
/// # Safety
/// As `sisu_rt_intrinsic`.
pub(crate) unsafe fn decode<'a>(
    desc: *const u32,
    ret: *mut u8,
    args: *const *mut u8,
) -> (Intrinsic, Call<'a>) {
    // SAFETY: the descriptor is `CallDesc::encode`'s and `args` holds one
    // address per argument.
    let c = unsafe {
        let n = *desc.add(1) as usize;
        let nt = *desc.add(2) as usize;
        let ret_ty = *desc.add(3);
        Call {
            ret,
            ret_ty,
            args: std::slice::from_raw_parts(args, n),
            tys: std::slice::from_raw_parts(desc.add(4), n),
            targs: std::slice::from_raw_parts(desc.add(4 + n), nt),
        }
    };
    // SAFETY: as above.
    let id = unsafe { *desc };
    let Some(i) = Intrinsic::from_id(id) else { rt::internal("an unknown intrinsic") };
    (i, c)
}

/// A call of an intrinsic that neither waits nor runs Sisu code itself:
/// on the system stack.
///
/// # Safety
/// As `sisu_rt_intrinsic`.
pub(crate) unsafe fn call(i: Intrinsic, c: &Call<'_>) -> u8 {
    // SAFETY: each intrinsic reads its arguments as their types say.
    unsafe { dispatch(i, c) }
}

unsafe fn dispatch(i: Intrinsic, c: &Call<'_>) -> u8 {
    use Intrinsic as I;
    if crate::rawmem::handles(i) {
        // SAFETY: as this function's caller guarantees.
        return unsafe { crate::rawmem::call(i, c) };
    }
    // SAFETY: the arguments are of the types the descriptor gives.
    unsafe {
        match i {
            I::Print | I::PrintErr => {
                let mut out = Vec::new();
                value::display(&mut out, c.arg(0), c.ty(0));
                out.push(b'\n');
                if i == I::Print {
                    rt::stdout(&out);
                } else {
                    rt::stderr(&String::from_utf8_lossy(&out));
                }
            }
            I::Len => {
                let (p, t) = c.place(0);
                let n = match ty(t).kind {
                    Kind::Str | Kind::String | Kind::Path => {
                        let b = value::text(p, t).unwrap_or(&[]);
                        String::from_utf8_lossy(b).chars().count() as u64
                    }
                    Kind::List(_) => mem::at::<List>(p).len,
                    Kind::Bytes => mem::at::<Text>(p).len,
                    Kind::Dict(..) | Kind::Set(_) => crate::dict::table(p).len(),
                    Kind::Arena(_) => crate::cells::arena_len(p),
                    _ => rt::internal("len of something without a length"),
                };
                c.set_word(n);
            }
            I::Panic => {
                let m = String::from_utf8_lossy(c.text(0)).into_owned();
                return rt::raise_msg(msg::PANIC_EXPLICIT, &[("message", m)]);
            }
            I::Unreachable => {
                let m = String::from_utf8_lossy(c.text(0)).into_owned();
                return if m.is_empty() {
                    rt::raise_msg(msg::PANIC_UNREACHABLE_PLAIN, &[])
                } else {
                    rt::raise_msg(msg::PANIC_UNREACHABLE, &[("message", m)])
                };
            }
            I::Todo => {
                let n = String::from_utf8_lossy(c.text(0)).into_owned();
                rt::note_hole(&n);
                let shown = if n.is_empty() {
                    "`todo()`".to_string()
                } else {
                    format!("`todo({})`", crate::bridge::quote(&n))
                };
                return rt::raise_msg(msg::PANIC_HOLE, &[("hole", shown)]);
            }
            I::Abs => {
                let (p, t) = c.place(0);
                match ty(t).kind {
                    Kind::Float => c.set_word(f64::from_bits(mem::word(p, 0)).abs().to_bits()),
                    _ => match crate::entry::int_op(abi::IntOp::Abs as u32, mem::word(p, 0) as i64, 0) {
                        Ok(v) => c.set_word(v as u64),
                        Err(s) => return s,
                    },
                }
            }
            I::Min | I::Max => return min_max(c, i == I::Min),
            I::Range => {
                let (start, stop, step) = (c.int(0), c.int(1), c.int(2));
                if step == 0 {
                    return rt::raise_msg(msg::PANIC_RANGE_STEP, &[]);
                }
                c.set_word(iter::into_word(iter::range(start, stop, step)));
            }
            I::Enumerate => {
                let inner = iter::iterable(c.arg(0), c.ty(0), c.item(0));
                c.set_word(iter::into_word(iter::enumerate(inner)));
            }
            I::Zip | I::ZipShortest => {
                let a = iter::iterable(c.arg(0), c.ty(0), c.item(0));
                let b = iter::iterable(c.arg(1), c.ty(1), c.item(1));
                c.set_word(iter::into_word(iter::zip(a, b, i == I::Zip, c.targ(0), c.targ(1))));
            }
            I::Reversed => {
                let it = iter::iterable(c.arg(0), c.ty(0), c.item(0));
                let mut items = match iter::collect_all(it, Some(c.targ(0))) {
                    Ok(l) => l,
                    Err(s) => return s,
                };
                crate::bridge::list_reverse((&raw mut items).cast::<u8>(), ty(c.targ(0)).size);
                c.set_word(iter::into_word(iter::items(items, c.targ(0))));
            }
            I::IterableList => {
                let it = iter::iterable(c.arg(0), c.ty(0), c.item(0));
                match iter::collect_all(it, c.item(0)) {
                    Ok(l) => (c.ret as *mut List).write(l),
                    Err(s) => return s,
                }
            }
            I::Sorted => {
                let t = c.targ(0);
                let it = iter::iterable(c.arg(0), c.ty(0), Some(t));
                let mut items = match iter::collect_all(it, Some(t)) {
                    Ok(l) => l,
                    Err(s) => return s,
                };
                if !crate::bridge::list_sort((&raw mut items).cast::<u8>(), t) {
                    let s = rt::raise_msg(msg::PANIC_UNORDERED, &[]);
                    let _ = value::drop_value(&mut items as *mut List as *mut u8, c.ret_ty);
                    return s;
                }
                (c.ret as *mut List).write(items);
            }
            I::Sum => return sum(c),
            I::Any | I::All => {
                let want = i == I::Any;
                let it = iter::iterable(c.arg(0), c.ty(0), c.item(0));
                let mut tmp = [0u64; 2];
                let t = tmp.as_mut_ptr() as *mut u8;
                loop {
                    let st = iter::next(it, t);
                    if st != abi::OK {
                        return value::after_part(st, iter::drop_iter(iter::into_word(it) as *mut u8));
                    }
                    if mem::word(t, 0) != 0 {
                        c.set_bool(!want);
                        break;
                    }
                    if (mem::word(t, 8) != 0) == want {
                        let st = iter::drop_iter(iter::into_word(it) as *mut u8);
                        c.set_bool(want);
                        return st;
                    }
                }
                return iter::drop_iter(iter::into_word(it) as *mut u8);
            }
            I::IterRef | I::IterMut => {
                let (p, t) = c.place(0);
                match ty(t).kind {
                    Kind::List(e) => c.set_word(iter::into_word(iter::list_ref(p as *mut List, e))),
                    Kind::Dict(..) => {
                        c.set_word(iter::into_word(iter::table_ref(crate::dict::table(p), iter::Part::Items)))
                    }
                    Kind::Set(_) => {
                        c.set_word(iter::into_word(iter::table_ref(crate::dict::table(p), iter::Part::Keys)))
                    }
                    Kind::Receiver(e) => {
                        c.set_word(iter::into_word(iter::receiver(mem::word(p, 0) as u32, e)))
                    }
                    _ => rt::internal("iterating over a borrowed value that is not a collection"),
                }
            }

            I::IterOwn | I::IterOf => {
                let t = c.ty(0);
                let item = match ty(c.ret_ty).kind {
                    Kind::Iter(item) => item,
                    _ => rt::internal("an iterator of an unknown type"),
                };
                c.set_word(iter::into_word(iter::iterable(c.arg(0), t, Some(item))));
            }
            I::IterNext => {
                let (p, _) = c.place(0);
                let obj = mem::word(p, 0) as *mut iter::IterObj;
                if obj.is_null() {
                    c.none();
                } else {
                    return iter::next(obj, c.ret);
                }
            }
            I::FmtNew => (c.ret as *mut Text).write(Text::empty()),
            I::FmtText | I::FmtDisplay | I::FmtDebug => {
                let mut out = Vec::new();
                match i {
                    I::FmtText => out.extend_from_slice(c.text(1)),
                    I::FmtDisplay => value::display(&mut out, c.arg(1), c.ty(1)),
                    _ => {
                        let (p, t) = c.place(1);
                        value::debug(&mut out, p, t);
                    }
                }
                let (s, _) = c.place(0);
                mem::at::<Text>(s).push(&out);
            }
            // The runtime in Sisu's (decision 350).
            I::FmtDigits => {
                let (n, base, upper) = (c.int(0), c.int(1), c.bool(2));
                crate::bridge::digits(c.ret, u128::from(n.unsigned_abs()), base, upper);
            }
            I::FmtWideDigits => {
                let (p, t) = c.place(0);
                let mag = crate::wide::num(p, t).map(|n| n.magnitude().1).unwrap_or(0);
                crate::bridge::digits(c.ret, mag, c.int(1), c.bool(2));
            }
            I::FmtFloatDigits => {
                let x = c.float(0);
                let kind = std::str::from_utf8(c.text(1)).ok().and_then(|s| s.chars().next());
                let precision = usize::try_from(c.int(2)).ok();
                c.set_string(float_digits(x, kind, precision).as_bytes());
            }
            I::FmtFloat32Digits => {
                let (p, _) = c.place(0);
                let x = (p as *const f32).read();
                let kind = std::str::from_utf8(c.text(1)).ok().and_then(|s| s.chars().next());
                let precision = usize::try_from(c.int(2)).ok();
                let s = match (kind, precision) {
                    (Some('f' | 'F' | 'e' | 'E' | '%'), _) | (_, Some(_)) => {
                        float_digits(f64::from(x), kind, precision)
                    }
                    _ if x.is_infinite() => "inf".into(),
                    _ => value::float32_repr(x),
                };
                c.set_string(s.as_bytes());
            }
            I::FmtFormatter => {
                // `Formatter(out="", spec=spec, depth=0)`.
                let Kind::Struct { fields, .. } = &ty(c.ret_ty).kind else {
                    rt::internal("a formatter that is not a struct")
                };
                let spec = c.text(0);
                for f in fields {
                    let at = c.ret.add(f.offset as usize);
                    match f.name.as_deref() {
                        Some("out") => (at as *mut Text).write(Text::empty()),
                        Some("spec") => value::write_string(at, spec),
                        _ => mem::set_word(at, 0, 0),
                    }
                }
            }
            I::StructuralDisplay | I::StructuralDebug => {
                let mut out = Vec::new();
                if i == I::StructuralDisplay {
                    value::display(&mut out, c.arg(0), c.ty(0));
                } else {
                    let (p, t) = c.place(0);
                    value::debug(&mut out, p, t);
                }
                let (f, ft) = c.place(1);
                let Kind::Struct { fields, .. } = &ty(ft).kind else {
                    rt::internal("formatting into something that is not a formatter")
                };
                let off = fields.first().map(|x| x.offset).unwrap_or(0);
                mem::at::<Text>(f.add(off as usize)).push(&out);
            }
            I::StructuralEq => c.set_bool(value::equal(c.arg(0), c.ty(0), c.arg(1), c.ty(1))),
            I::StructuralPartialCmp => match value::compare(c.arg(0), c.ty(0), c.arg(1), c.ty(1)) {
                Some(o) => {
                    let tag = match o {
                        Ordering::Less => 0,
                        Ordering::Equal => 1,
                        Ordering::Greater => 2,
                    };
                    c.some_word(tag);
                }
                None => c.none(),
            },
            I::StructuralCmp | I::IntCmp | I::BoolCmp | I::CharCmp | I::F64Cmp | I::StrCmp => {
                match value::compare(c.arg(0), c.ty(0), c.arg(1), c.ty(1)) {
                    Some(o) => c.ordering(o),
                    None => return rt::raise_msg(msg::PANIC_UNORDERED, &[]),
                }
            }
            I::StructuralHash => {
                // The hasher's state is its one field (decision 256).
                let (v, t) = c.place(0);
                let (h, _) = c.place(1);
                let state = crate::dict::structural_hash(mem::word(h, 0), v, t);
                mem::set_word(h, 0, state);
            }
            I::AssertFail
            | I::AssertFailCompare
            | I::AssertFailParts
            | I::ExpectFail
            | I::ExpectFailCompare
            | I::ExpectFailParts => return assert_fail(c, i),
            I::ErrorTracePush => {
                let (p, _) = c.place(0);
                let loc = c.text(1).to_vec().into_boxed_slice();
                // The error's hold on its trace passes to the new node.
                let prev = mem::word(p, abi::ERROR_TRACE) as *mut TraceNode;
                let node = Box::into_raw(Box::new(TraceNode { refs: 1, loc, prev, _live: mem::Live::new() }));
                mem::set_word(p, abi::ERROR_TRACE, node as u64);
            }
            I::ErrorTrace => {
                let (p, t) = c.place(0);
                let lines: Vec<String> = trace_lines(p, t).into_iter().map(|l| format!("at {l}")).collect();
                c.set_string(lines.join("\n").as_bytes());
            }
            I::ErrorIs => {
                let (p, t) = c.place(0);
                c.set_bool(value::concrete(p, t) == c.targ(0));
            }
            I::IntToF64 => c.set_word((c.int(0) as f64).to_bits()),
            I::IntPow => match crate::entry::int_op(abi::IntOp::Pow as u32, c.int(0), c.int(1)) {
                Ok(v) => c.set_word(v as u64),
                Err(s) => return s,
            },
            I::F64Sqrt => c.set_word(c.float(0).sqrt().to_bits()),
            I::F64Floor => c.set_word(c.float(0).floor().to_bits()),
            I::F64Ceil => c.set_word(c.float(0).ceil().to_bits()),
            I::F64Round => c.set_word(c.float(0).round().to_bits()),
            I::F64Abs => c.set_word(c.float(0).abs().to_bits()),
            I::F64IsNan => c.set_bool(c.float(0).is_nan()),
            I::F64MulAdd => c.set_word(c.float(0).mul_add(c.float(1), c.float(2)).to_bits()),
            I::F64WithinUlps => {
                let (f, g, n) = (c.float(0), c.float(1), c.int(2));
                c.set_bool(n >= 0 && ulps_between(f, g).is_some_and(|d| d <= n as u64));
            }
            I::F64TruncateToInt => {
                let f = c.float(0);
                let t = f.trunc();
                if t.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&t) {
                    return rt::raise_msg(
                        msg::PANIC_CONVERSION,
                        &[("value", value::float_repr(f)), ("ty", "int".into())],
                    );
                }
                c.set_word(t as i64 as u64);
            }
            I::CharFromCode => match u32::try_from(c.int(0)).ok().and_then(char::from_u32) {
                Some(ch) => c.some_word(u64::from(ch as u32)),
                None => c.none(),
            },
            I::CharIsDigit => c.set_bool(c.char(0).is_ascii_digit()),
            I::CharToDigit => match c.char(0).to_digit(10) {
                Some(d) => c.some_word(u64::from(d)),
                None => c.none(),
            },
            I::CharIsAlphabetic => c.set_bool(c.char(0).is_alphabetic()),
            I::CharIsWhitespace => c.set_bool(c.char(0).is_whitespace()),
            I::CharIsUppercase => c.set_bool(c.char(0).is_uppercase()),
            I::CharIsLowercase => c.set_bool(c.char(0).is_lowercase()),
            I::CharCode => c.set_word(u64::from(c.char(0) as u32)),
            I::StrByteLen => c.set_word(c.text(0).len() as u64),
            I::StrByteAt => {
                let idx = c.int(1);
                let s = c.text(0);
                match usize::try_from(idx).ok().and_then(|k| s.get(k)) {
                    Some(b) => c.set_word(u64::from(*b)),
                    None => {
                        return rt::raise_msg(
                            msg::PANIC_INDEX_OUT_OF_BOUNDS,
                            &[("len", s.len().to_string()), ("index", idx.to_string())],
                        );
                    }
                }
            }
            I::StrByteSlice => {
                let (start, end) = (c.int(1), c.int(2));
                let s = c.text(0);
                let len = s.len() as i64;
                if start < 0 || end < start || end > len {
                    let bad = if start < 0 || start > len { start } else { end };
                    return rt::raise_msg(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", len.to_string()), ("index", bad.to_string())],
                    );
                }
                let (start, end) = (start as usize, end as usize);
                for k in [start, end] {
                    if !is_char_boundary(s, k) {
                        return rt::raise_msg(msg::PANIC_CHAR_BOUNDARY, &[("index", k.to_string())]);
                    }
                }
                value::write_str(c.ret, &s[start..end]);
            }
            I::StrBytes => {
                let s = c.text(0);
                c.set_word(iter::into_word(iter::bytes(Str { ptr: s.as_ptr(), len: s.len() as u64 })));
            }
            I::StrChars => {
                let s = c.text(0);
                c.set_word(iter::into_word(iter::chars(Str { ptr: s.as_ptr(), len: s.len() as u64 })));
            }
            I::StrParseFloat => {
                let s = String::from_utf8_lossy(c.text(0)).into_owned();
                match decimal(&s).then(|| s.parse::<f64>().ok()).flatten() {
                    Some(f) => c.some_word(f.to_bits()),
                    None => c.none(),
                }
            }
            I::StrParseF32 => {
                let s = String::from_utf8_lossy(c.text(0)).into_owned();
                match decimal(&s).then(|| s.parse::<f32>().ok()).flatten() {
                    Some(f) => c.some_word(u64::from(f.to_bits())),
                    None => c.none(),
                }
            }
            I::StrToLowercase => c.set_string(String::from_utf8_lossy(c.text(0)).to_lowercase().as_bytes()),
            I::StrToUppercase => c.set_string(String::from_utf8_lossy(c.text(0)).to_uppercase().as_bytes()),
            I::StrToString | I::StringNew | I::PathNew => c.set_string(c.text(0)),
            I::PathAsStr => value::write_str(c.ret, c.text(0)),
            I::OpAdd | I::OpSub | I::OpMul | I::OpDiv | I::OpRem | I::OpNeg => return number_op(c, i),
            I::ContractFail => {
                let kind = c.int(0);
                let owner = String::from_utf8_lossy(c.text(1)).into_owned();
                let clause = String::from_utf8_lossy(c.text(2)).into_owned();
                let parts = shown_parts(c.arg(3), c.ty(3));
                if kind == 3 {
                    let value = parts.first().map(|(_, v)| v.clone()).unwrap_or_default();
                    return rt::raise_msg(
                        msg::CONTRACT_REFINEMENT_FAILED,
                        &[("value", value), ("ty", owner), ("clause", clause)],
                    );
                }
                let (id, key) = match kind {
                    0 => (msg::CONTRACT_REQUIRES_FAILED, "function"),
                    1 => (msg::CONTRACT_ENSURES_FAILED, "function"),
                    _ => (msg::CONTRACT_INVARIANT_FAILED, "ty"),
                };
                let mut m = rt::render(id, &[(key, owner), ("clause", clause.clone())]);
                m.push_str(&parts_diagram(&clause, &parts));
                return rt::raise_msg(msg::PANIC_EXPLICIT, &[("message", m)]);
            }
            I::StringPush | I::StringPushChar | I::StringClear => {
                let extra: Vec<u8> = match i {
                    I::StringPush => c.text(1).to_vec(),
                    I::StringPushChar => c.char(1).to_string().into_bytes(),
                    _ => Vec::new(),
                };
                let (s, _) = c.place(0);
                let t = mem::at::<Text>(s);
                if i == I::StringClear {
                    t.len = 0;
                } else {
                    t.push(&extra);
                }
            }
            I::OptionIsSome | I::ResultIsOk => {
                let (p, _) = c.place(0);
                c.set_bool(mem::word(p, 0) == 0);
            }
            I::OptionIsNone | I::ResultIsErr => {
                let (p, _) = c.place(0);
                c.set_bool(mem::word(p, 0) != 0);
            }
            I::OptionExpect | I::ResultExpect | I::OptionUnwrap | I::ResultUnwrap => return expect(c, i),
            I::OptionOr | I::ResultOr => {
                let (p, t) = c.place(0);
                let entry = ty(t);
                let size = ty(c.ret_ty).size;
                if mem::word(p, 0) == 0 {
                    mem::copy(c.ret, p.add(8), size as usize);
                    return value::drop_value(c.arg(1), c.ty(1));
                }
                if let Kind::Result(_, err) = entry.kind {
                    let st = value::drop_value(p.add(8), err);
                    mem::copy(c.ret, c.arg(1), size as usize);
                    return st;
                }
                mem::copy(c.ret, c.arg(1), size as usize);
            }
            I::ResultOk | I::ResultErr => {
                let want_ok = i == I::ResultOk;
                let (p, t) = c.place(0);
                let Kind::Result(ok, err) = ty(t).kind else {
                    rt::internal("ok of something that is not a result")
                };
                let first = mem::word(p, 0) == 0;
                let (keep, other) = if want_ok { (ok, err) } else { (err, ok) };
                if first == want_ok {
                    mem::set_word(c.ret, 0, 0);
                    mem::copy(c.ret.add(8), p.add(8), ty(keep).size as usize);
                } else {
                    c.none();
                    return value::drop_value(p.add(8), other);
                }
            }
            I::IntWiden => c.set_word(c.word(0)),
            I::F32Cmp => match c.f32(0).partial_cmp(&c.f32(1)) {
                Some(o) => c.ordering(o),
                None => return rt::raise_msg(msg::PANIC_UNORDERED, &[]),
            },
            I::F32ToF64 => c.set_word(f64::from(c.f32(0)).to_bits()),
            I::F32Sqrt => c.set_f32(f64::from(c.f32(0)).sqrt() as f32),
            I::F32Floor => c.set_f32(c.f32(0).floor()),
            I::F32Ceil => c.set_f32(c.f32(0).ceil()),
            I::F32Round => c.set_f32(c.f32(0).round()),
            I::F32Abs => c.set_f32(c.f32(0).abs()),
            I::F32IsNan => c.set_bool(c.f32(0).is_nan()),
            I::F32MulAdd => c.set_f32(c.f32(0).mul_add(c.f32(1), c.f32(2))),
            I::F32TruncateToInt => {
                let f = f64::from(c.f32(0));
                let t = f.trunc();
                if t.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&t) {
                    return rt::raise_msg(
                        msg::PANIC_CONVERSION,
                        &[("value", value::float32_repr(f as f32)), ("ty", "int".into())],
                    );
                }
                c.set_word(t as i64 as u64);
            }
            I::F32ToBits => c.set_word(u64::from(c.f32(0).to_bits())),
            I::F32FromBits => c.set_f32(f32::from_bits(c.word(0) as u32)),
            I::F32FromLiteral | I::F64ToF32 => c.set_f32(c.float(0) as f32),
            I::IntToF32 => c.set_f32(c.int(0) as f32),
            I::HostEnvArgs => {
                // Views of the process's arguments, which live as long as it.
                let args = crate::entry::args();
                let size = 16;
                let mut l = List::empty();
                l.reserve(args.len() as u64, size);
                for (k, a) in args.iter().enumerate() {
                    value::write_str(l.at(k as u64, size), a.as_bytes());
                }
                l.len = args.len() as u64;
                (c.ret as *mut List).write(l);
            }
            I::HostEnvGet => {
                let name = String::from_utf8_lossy(c.text(1)).into_owned();
                match std::env::var(&name) {
                    Ok(v) => {
                        mem::set_word(c.ret, 0, 0);
                        value::write_string(c.ret.add(8), v.as_bytes());
                    }
                    Err(_) => c.none(),
                }
            }
            I::HostStdinReadLine => {
                let mut line = String::new();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) | Err(_) => c.none(),
                    Ok(_) => {
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }
                        mem::set_word(c.ret, 0, 0);
                        value::write_string(c.ret.add(8), line.as_bytes());
                    }
                }
            }
            I::HostRandBits => c.set_word(random_bits()),
            I::HostFsReadToString => {
                let path = String::from_utf8_lossy(c.text(1)).into_owned();
                match std::fs::read(&path) {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(t) => {
                            mem::set_word(c.ret, 0, 0);
                            value::write_string(c.ret.add(8), t.as_bytes());
                        }
                        Err(_) => io_error(c, IoKind::InvalidData, &path),
                    },
                    Err(e) => io_error(c, io_kind(&e), &path),
                }
            }
            I::HostFsRead => {
                let path = String::from_utf8_lossy(c.text(1)).into_owned();
                match std::fs::read(&path) {
                    Ok(b) => {
                        mem::set_word(c.ret, 0, 0);
                        crate::bytes::write(c.ret.add(8), &b);
                    }
                    Err(e) => io_error(c, io_kind(&e), &path),
                }
            }
            I::HostFsWrite | I::HostFsWriteText | I::HostFsRemove | I::HostFsCreateDir | I::HostFsSync => {
                let path = String::from_utf8_lossy(c.text(1)).into_owned();
                let r = match i {
                    I::HostFsWrite => std::fs::write(&path, crate::bytes::arg(c, 2)),
                    I::HostFsWriteText => std::fs::write(&path, c.text(2)),
                    I::HostFsRemove => std::fs::remove_file(&path),
                    I::HostFsCreateDir => std::fs::create_dir_all(&path),
                    _ => std::fs::File::open(&path).and_then(|f| f.sync_all()),
                };
                match r {
                    Ok(()) => mem::set_word(c.ret, 0, 0),
                    Err(e) => io_error(c, io_kind(&e), &path),
                }
            }
            I::TestCaseSeed => {
                let k = std::env::var("SISU_TEST_CASE_SEED").ok().and_then(|v| v.trim().parse::<u64>().ok());
                c.set_word(k.unwrap_or(0));
            }
            I::TestReplayCase => match rt::take_replay_case() {
                Some(choices) => {
                    let mut l = List::empty();
                    l.reserve(choices.len() as u64, 8);
                    for (k, v) in choices.iter().enumerate() {
                        mem::set_word(l.at(k as u64, 8), 0, *v as u64);
                    }
                    l.len = choices.len() as u64;
                    mem::set_word(c.ret, 0, 0);
                    (c.ret.add(8) as *mut List).write(l);
                }
                None => c.none(),
            },
            I::TestRecordCase => {
                let (p, _) = c.place(0);
                let l = (p as *const List).read();
                let shown: Vec<String> =
                    (0..l.len).map(|k| (mem::word(l.at(k, 8), 0) as i64).to_string()).collect();
                if let Some(path) = std::env::var_os("SISU_TEST_REPORT") {
                    let mut path = std::path::PathBuf::from(path).into_os_string();
                    path.push(".case");
                    let _ = std::fs::write(path, shown.join("."));
                }
            }
            I::TestSnapshot => {
                let text = String::from_utf8_lossy(c.text(0)).into_owned();
                if let Some(s) = rt::snapshot(text) {
                    return s;
                }
            }
            I::TestSimFault => c.set_bool(crate::faults::decide(c)),
            I::TestSimFaultError => crate::faults::write(c),
            I::SimChoose => {
                let n = c.int(0);
                if n < 1 {
                    return rt::raise_msg(msg::SIM_CHOOSE_NONE, &[]);
                }
                let n = n.min(i64::from(u32::MAX)) as u32;
                c.set_word(u64::from(crate::sched::with(|s| s.choose(n, false))));
            }
            I::TestEntry => {
                let k = std::env::var("SISU_TEST_ENTRY").ok().and_then(|v| v.trim().parse::<i64>().ok());
                c.set_word(k.unwrap_or(0) as u64);
            }
            I::HostFsRename => {
                // A failure names the source when it is not there, else the
                // target (decision 420).
                let from = String::from_utf8_lossy(c.text(1)).into_owned();
                let to = String::from_utf8_lossy(c.text(2)).into_owned();
                match std::fs::rename(&from, &to) {
                    Ok(()) => mem::set_word(c.ret, 0, 0),
                    Err(e) => {
                        let kind = io_kind(&e);
                        let gone = matches!(kind, IoKind::NotFound) && !std::path::Path::new(&from).exists();
                        io_error(c, kind, if gone { &from } else { &to })
                    }
                }
            }
            I::HostFsExists => {
                let path = String::from_utf8_lossy(c.text(1)).into_owned();
                c.set_bool(std::path::Path::new(&path).exists());
            }
            I::HostFsMetadata => {
                let path = String::from_utf8_lossy(c.text(1)).into_owned();
                match std::fs::metadata(&path) {
                    Ok(m) => {
                        let modified = m
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
                            .unwrap_or(0);
                        mem::set_word(c.ret, 0, 0);
                        // `Metadata(len, modified, is_dir)`: three words.
                        mem::set_word(c.ret, 8, i64::try_from(m.len()).unwrap_or(i64::MAX) as u64);
                        mem::set_word(c.ret, 16, modified as u64);
                        mem::set_word(c.ret, 24, u64::from(m.is_dir()));
                    }
                    Err(e) => io_error(c, io_kind(&e), &path),
                }
            }
            I::HostFsListDir => {
                let path = String::from_utf8_lossy(c.text(1)).into_owned();
                let names = std::fs::read_dir(&path).and_then(|rd| {
                    let mut names = Vec::new();
                    for e in rd {
                        names.push(e?.file_name().to_string_lossy().into_owned());
                    }
                    names.sort();
                    Ok(names)
                });
                match names {
                    Ok(names) => {
                        let dir = path.trim_end_matches('/');
                        let mut l = List::empty();
                        l.reserve(names.len() as u64, 24);
                        for (k, n) in names.iter().enumerate() {
                            let p =
                                if dir.is_empty() || dir == "." { n.clone() } else { format!("{dir}/{n}") };
                            value::write_string(l.at(k as u64, 24), p.as_bytes());
                        }
                        l.len = names.len() as u64;
                        mem::set_word(c.ret, 0, 0);
                        (c.ret.add(8) as *mut List).write(l);
                    }
                    Err(e) => io_error(c, io_kind(&e), &path),
                }
            }
            I::ClockNow => {
                let clock = crate::tasks::clock_of(c);
                let now = crate::sched::with(|s| s.clock_now(clock));
                c.set_word(i64::try_from(now).unwrap_or(i64::MAX) as u64);
            }
            I::ClockMonotonic => {
                let clock = crate::tasks::clock_of(c);
                let now = crate::sched::with(|s| s.clock_monotonic(clock));
                c.set_word(i64::try_from(now).unwrap_or(i64::MAX) as u64);
            }
            I::DateTimeFromUnixNanos | I::DateTimeUnixNanos => c.set_word(c.word(0)),
            // A native run has no hot replacement (`--watch` runs the
            // interpreter): every task stays in epoch 0.
            I::LiveBoundary => {}
            I::LiveEpoch => c.set_word(0),
            I::FmtStrftime => {
                let pattern = String::from_utf8_lossy(c.text(1)).into_owned();
                c.set_string(value::strftime(c.int(0), &pattern).as_bytes());
            }
            I::GenericValue => match ty(c.targ(0)).kind {
                Kind::Const(n) => c.set_word(n as u64),
                _ => rt::internal("a `const` parameter without a constant"),
            },
            I::ArrayFilled => {
                // `N - 1` copies of the value, and the value itself last.
                let n = match ty(c.targ(1)).kind {
                    Kind::Const(n) if n >= 0 => n as u64,
                    _ => rt::internal("an array length that is not a constant"),
                };
                let e = c.ty(0);
                let size = ty(e).size;
                let mut l = List::empty();
                l.reserve(n, size);
                for k in 1..n {
                    value::copy_value(l.at(k - 1, size), c.arg(0), e);
                }
                if n > 0 {
                    mem::copy(l.at(n - 1, size), c.arg(0), size as usize);
                } else {
                    let st = value::drop_value(c.arg(0), e);
                    if st != abi::OK {
                        return st;
                    }
                }
                l.len = n;
                (c.ret as *mut List).write(l);
            }
            I::ArrayToList => {
                let (p, t) = c.place(0);
                value::copy_value(c.ret, p, t);
            }
            I::StringAsStr | I::StringLen | I::ListIndexOf => {
                return rt::raise_msg(
                    msg::RUN_INTERNAL,
                    &[("detail", format!("`{}` is not in the native runtime yet", i.name()))],
                );
            }
            // The cells', the bytes', the capabilities' and the host's are
            // their modules', asked after the commonest here (each module
            // tells by a match of its own whether it has the intrinsic).
            // Tasks, channels, timers, generators and mutexes are `tasks`'s,
            // called before this one (`sisu_rt_intrinsic`).
            _ => {
                if let Some(st) = crate::cells::call(i, c) {
                    return st;
                }
                if let Some(st) = crate::bytes::call(i, c) {
                    return st;
                }
                if let Some(st) = crate::caps::call(i, c) {
                    return st;
                }
                if let Some(st) = crate::host::call_system(i, c) {
                    return st;
                }
                rt::internal("a task intrinsic outside the task module")
            }
        }
    }
    abi::OK
}

/// The numbers' operator traits (`op.add` and the others, from
/// `<std>/builtin_impls.sisu`): the operators of `int`, `f64` and `f32`.
///
/// # Safety
/// As [`call`].
unsafe fn number_op(c: &Call<'_>, i: Intrinsic) -> u8 {
    use crate::abi::IntOp;
    // SAFETY: the arguments are numbers.
    unsafe {
        let (_, t) = c.place(0);
        match ty(t).kind {
            Kind::Float | Kind::F32 => {
                let f32 = matches!(ty(t).kind, Kind::F32);
                let get = |k: usize| if f32 { f64::from(c.f32(k)) } else { c.float(k) };
                let x = get(0);
                let r = match i {
                    Intrinsic::OpNeg => -x,
                    Intrinsic::OpAdd => x + get(1),
                    Intrinsic::OpSub => x - get(1),
                    Intrinsic::OpMul => x * get(1),
                    Intrinsic::OpDiv => x / get(1),
                    _ => crate::entry::float_mod(x, get(1)),
                };
                if f32 { c.set_f32(r as f32) } else { c.set_word(r.to_bits()) }
            }
            _ => {
                let op = match i {
                    Intrinsic::OpAdd => IntOp::Add,
                    Intrinsic::OpSub => IntOp::Sub,
                    Intrinsic::OpMul => IntOp::Mul,
                    Intrinsic::OpDiv => IntOp::FloorDiv,
                    Intrinsic::OpRem => IntOp::Mod,
                    _ => IntOp::Neg,
                };
                let y = if op == IntOp::Neg { 0 } else { c.int(1) };
                match crate::entry::int_op(op as u32, c.int(0), y) {
                    Ok(v) => c.set_word(v as u64),
                    Err(s) => return s,
                }
            }
        }
        abi::OK
    }
}

/// The kinds of `IoError` a host operation reports (decision 11).
#[derive(Clone, Debug)]
enum IoKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidData,
    Interrupted,
    Other(String),
}

fn io_kind(e: &std::io::Error) -> IoKind {
    match e.kind() {
        std::io::ErrorKind::NotFound => IoKind::NotFound,
        std::io::ErrorKind::PermissionDenied => IoKind::PermissionDenied,
        std::io::ErrorKind::AlreadyExists => IoKind::AlreadyExists,
        std::io::ErrorKind::InvalidData => IoKind::InvalidData,
        std::io::ErrorKind::Interrupted => IoKind::Interrupted,
        _ => IoKind::Other(e.to_string()),
    }
}

/// Writes `Err(e)` at the result, `e` the `IoError` of `kind` about `path`
/// (the result's error type is `IoError` or a union holding it).
///
/// # Safety
/// The result must be a `Result` whose error can be an `IoError`.
unsafe fn io_error(c: &Call<'_>, kind: IoKind, path: &str) {
    let Kind::Result(_, err) = ty(c.ret_ty).kind else { rt::internal("an I/O result that is not a result") };
    let decl = match &ty(err).kind {
        Kind::Enum { .. } => err,
        Kind::ErrSet(members) => members
            .iter()
            .copied()
            .find(|m| matches!(&ty(*m).kind, Kind::Enum { name, .. } if name == "IoError"))
            .unwrap_or_else(|| rt::internal("an I/O result that cannot hold an `IoError`")),
        _ => rt::internal("an I/O result that cannot hold an `IoError`"),
    };
    let Kind::Enum { variants, .. } = &ty(decl).kind else { rt::internal("`IoError` is not an enum") };
    let (name, text) = match &kind {
        IoKind::NotFound => ("NotFound", Some(path.to_string())),
        IoKind::PermissionDenied => ("PermissionDenied", Some(path.to_string())),
        IoKind::AlreadyExists => ("AlreadyExists", Some(path.to_string())),
        IoKind::InvalidData => ("InvalidData", Some(path.to_string())),
        IoKind::Interrupted => ("Interrupted", None),
        IoKind::Other(m) => ("Other", Some(m.clone())),
    };
    let Some(tag) = variants.iter().position(|v| v.name == name) else {
        rt::internal("an `IoError` variant missing")
    };
    // SAFETY: the payload is an error set's value.
    unsafe {
        mem::set_word(c.ret, 0, 1);
        let e = c.ret.add(8);
        mem::set_word(e, abi::ERROR_DECL, u64::from(decl));
        mem::set_word(e, abi::ERROR_TRACE, 0);
        mem::set_word(e, abi::ERROR_TAG, tag as u64);
        if let (Some(t), Some(f)) = (text, variants[tag].fields.first()) {
            value::write_string(e.add(f.offset as usize), t.as_bytes());
        }
    }
}

/// A field of a value a runtime operation writes: a word, or the text of
/// a `String` or `Path`.
pub(crate) enum Part<'a> {
    Word(u64),
    Text(&'a [u8]),
}

/// Writes `parts` at the fields of `fields` from `p`.
///
/// # Safety
/// `p` must have room for the fields, which must be of the parts' kinds.
pub(crate) unsafe fn write_parts(p: *mut u8, fields: &[abi::Field], parts: &[Part<'_>]) {
    for (f, part) in fields.iter().zip(parts) {
        // SAFETY: as the caller guarantees.
        unsafe {
            match part {
                Part::Word(w) => mem::set_word(p, f.offset, *w),
                Part::Text(t) => value::write_string(p.add(f.offset as usize), t),
            }
        }
    }
}

/// Writes `Err(e)` at the result, `e` the variant `variant` of the error
/// set `set` with its fields (the result's error type is `set` or a union
/// holding it).
///
/// # Safety
/// The result must be a `Result` whose error can be a `set`, and the
/// parts of the variant's fields' kinds.
pub(crate) unsafe fn write_err(c: &Call<'_>, set: &str, variant: &str, parts: &[Part<'_>]) {
    let Kind::Result(_, err) = ty(c.ret_ty).kind else {
        rt::internal("an error result that is not a result")
    };
    let named = |t: u32| matches!(&ty(t).kind, Kind::Enum { name, .. } if name == set);
    let decl = match &ty(err).kind {
        Kind::Enum { .. } if named(err) => err,
        Kind::ErrSet(members) => members
            .iter()
            .copied()
            .find(|m| named(*m))
            .unwrap_or_else(|| rt::internal("a result that cannot hold its error")),
        _ => rt::internal("a result that cannot hold its error"),
    };
    let Kind::Enum { variants, .. } = &ty(decl).kind else {
        rt::internal("an error set that is not an enum")
    };
    let Some(tag) = variants.iter().position(|v| v.name == variant) else {
        rt::internal("an error variant missing")
    };
    // SAFETY: the payload is an error set's value.
    unsafe {
        mem::set_word(c.ret, 0, 1);
        let e = c.ret.add(8);
        mem::set_word(e, abi::ERROR_DECL, u64::from(decl));
        mem::set_word(e, abi::ERROR_TRACE, 0);
        mem::set_word(e, abi::ERROR_TAG, tag as u64);
        write_parts(e, &variants[tag].fields, parts);
    }
}

/// 64 random bits: a SplitMix64 stream seeded once from the clock and the
/// process.
fn random_bits() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering as O};
    static STATE: AtomicU64 = AtomicU64::new(0);
    if STATE.load(O::Relaxed) == 0 {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
            ^ u64::from(std::process::id()).rotate_left(32);
        STATE.store(seed | 1, O::Relaxed);
    }
    let mut z = STATE.fetch_add(0x9E37_79B9_7F4A_7C15, O::Relaxed).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A node of an error's return trace: the location and the trace before
/// it. Copies of an error share their trace, which never changes, so a
/// node counts what holds it (errors and the next nodes) and is freed with
/// the last (decision 215).
struct TraceNode {
    refs: u64,
    loc: Box<[u8]>,
    prev: *mut TraceNode,
    _live: mem::Live,
}

/// Another holder of the trace whose newest node is at `node` (0 for none).
///
/// # Safety
/// `node` must be 0 or a live trace node.
pub(crate) unsafe fn retain_trace(node: u64) {
    let n = node as *mut TraceNode;
    if !n.is_null() {
        // SAFETY: as the caller guarantees.
        unsafe { (*n).refs += 1 };
    }
}

/// A holder of the trace at `node` is gone: the nodes nothing holds any
/// more are freed, newest first.
///
/// # Safety
/// `node` must be 0 or a live trace node the caller held.
pub(crate) unsafe fn release_trace(node: u64) {
    let mut n = node as *mut TraceNode;
    // SAFETY: as the caller guarantees; a node is freed once, when its
    // count reaches 0.
    unsafe {
        while !n.is_null() {
            (*n).refs -= 1;
            if (*n).refs > 0 {
                return;
            }
            let b = Box::from_raw(n);
            n = b.prev;
        }
    }
}

/// The locations an error passed through, oldest first.
///
/// # Safety
/// `p` must hold a value of type `t`.
pub(crate) unsafe fn trace_lines(p: *mut u8, t: u32) -> Vec<String> {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = target(p, t);
        let decl = value::concrete(p, t);
        if !matches!(ty(decl).kind, Kind::Enum { error: true, .. }) {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut node = mem::word(p, abi::ERROR_TRACE) as *const TraceNode;
        while !node.is_null() {
            out.push(String::from_utf8_lossy(&(*node).loc).into_owned());
            node = (*node).prev;
        }
        out.reverse();
        out
    }
}

fn is_char_boundary(s: &[u8], i: usize) -> bool {
    i == 0 || i == s.len() || s.get(i).is_some_and(|b| (*b as i8) >= -0x40)
}

/// `min` and `max`: the chosen value, copied out of a reference or moved;
/// the other is dropped.
///
/// # Safety
/// As [`call`].
unsafe fn min_max(c: &Call<'_>, min: bool) -> u8 {
    // SAFETY: the arguments are of their types.
    unsafe {
        let o = match value::compare(c.arg(0), c.ty(0), c.arg(1), c.ty(1)) {
            Some(o) => o,
            None => return rt::raise_msg(msg::PANIC_UNORDERED, &[]),
        };
        let pick_y = if min { o == Ordering::Greater } else { o == Ordering::Less };
        let (keep, other) = if pick_y { (1, 0) } else { (0, 1) };
        if matches!(ty(c.ty(keep)).kind, Kind::Ref(_)) {
            let (p, t) = c.place(keep);
            value::copy_value(c.ret, p, t);
            abi::OK
        } else {
            mem::copy(c.ret, c.arg(keep), ty(c.ret_ty).size as usize);
            value::drop_value(c.arg(other), c.ty(other))
        }
    }
}

/// `sum`: floats pairwise in a fixed shape (D-074), integers with
/// overflow checked.
///
/// # Safety
/// As [`call`].
unsafe fn sum(c: &Call<'_>) -> u8 {
    // SAFETY: the arguments are of their types.
    unsafe {
        let it = iter::iterable(c.arg(0), c.ty(0), c.item(0));
        // The items are the result's numbers.
        let size = ty(c.ret_ty).size;
        let items = match iter::collect_all(it, c.item(0)) {
            Ok(l) => l,
            Err(s) => return s,
        };
        let words: Vec<u64> = (0..items.len).map(|k| mem::word(items.at(k, size), 0)).collect();
        mem::release(items.ptr);
        match ty(c.ret_ty).kind {
            Kind::Float => {
                let xs: Vec<f64> = words.iter().map(|w| f64::from_bits(*w)).collect();
                c.set_word(pairwise(&xs).to_bits());
            }
            Kind::F32 => {
                let xs: Vec<f32> = words.iter().map(|w| f32::from_bits(*w as u32)).collect();
                mem::set_word(c.ret, 0, 0);
                (c.ret as *mut f32).write(pairwise32(&xs));
            }
            _ => {
                let mut total = 0i64;
                for w in words {
                    let v = w as i64;
                    match total.checked_add(v) {
                        Some(t) => total = t,
                        None => {
                            return rt::raise_msg(msg::PANIC_OVERFLOW, &[("expr", format!("{total} + {v}"))]);
                        }
                    }
                }
                c.set_word(total as u64);
            }
        }
        abi::OK
    }
}

fn pairwise(xs: &[f64]) -> f64 {
    match xs.len() {
        0 => 0.0,
        1 => xs[0],
        n => pairwise(&xs[..n / 2]) + pairwise(&xs[n / 2..]),
    }
}

fn pairwise32(xs: &[f32]) -> f32 {
    match xs.len() {
        0 => 0.0,
        1 => xs[0],
        n => pairwise32(&xs[..n / 2]) + pairwise32(&xs[n / 2..]),
    }
}

/// `expect` and `unwrap` of an option or a result (decision 28).
///
/// # Safety
/// As [`call`].
unsafe fn expect(c: &Call<'_>, i: Intrinsic) -> u8 {
    // SAFETY: the arguments are of their types.
    unsafe {
        let (p, t) = c.place(0);
        let entry = ty(t);
        let first = mem::word(p, 0) == 0;
        if first {
            mem::copy(c.ret, p.add(8), ty(c.ret_ty).size as usize);
            return abi::OK;
        }
        let m = if matches!(i, Intrinsic::OptionUnwrap | Intrinsic::ResultUnwrap) {
            String::new()
        } else {
            String::from_utf8_lossy(c.text(1)).into_owned()
        };
        let payload = match entry.kind {
            Kind::Result(_, err) => Some((p.add(8), err)),
            _ => None,
        };
        let mut text = match (payload, m.is_empty()) {
            (Some((e, et)), true) => {
                let mut out = Vec::new();
                value::debug(&mut out, e, et);
                rt::render(msg::PANIC_EXPECT_ERROR, &[("value", String::from_utf8_lossy(&out).into_owned())])
            }
            (Some((e, et)), false) => {
                let mut out = Vec::new();
                value::debug(&mut out, e, et);
                rt::render(
                    msg::PANIC_EXPECT,
                    &[("message", m), ("value", String::from_utf8_lossy(&out).into_owned())],
                )
            }
            (None, true) => rt::render(msg::PANIC_EXPECT_NONE, &[]),
            (None, false) => rt::render(msg::PANIC_EXPLICIT, &[("message", m)]),
        };
        if let Some((e, et)) = payload {
            let lines = trace_lines(e, et);
            if !lines.is_empty() {
                text.push_str("\n  ");
                text.push_str(&rt::render(msg::RUN_TRACE_HEADER, &[]));
            }
            for line in lines {
                text.push_str("\n    ");
                text.push_str(&rt::render(msg::RUN_TRACE_LINE, &[("location", line)]));
            }
            let _ = value::drop_value(e, et);
        }
        rt::raise_msg(msg::PANIC_EXPLICIT, &[("message", text)])
    }
}

/// `assert.fail`, `fail_compare` and `fail_parts` (17, "Assertions").
///
/// # Safety
/// As [`call`].
unsafe fn assert_fail(c: &Call<'_>, i: Intrinsic) -> u8 {
    // SAFETY: the arguments are of their types.
    unsafe {
        let expr = String::from_utf8_lossy(c.text(0)).into_owned();
        let message = String::from_utf8_lossy(c.text(1)).into_owned();
        let expect =
            matches!(i, Intrinsic::ExpectFail | Intrinsic::ExpectFailCompare | Intrinsic::ExpectFailParts);
        // In a test, a failed `expect` is recorded in the words of a failed
        // expectation (decision 545).
        let recorded = expect && rt::records_expect();
        let (plain, with) = if recorded {
            (msg::RUN_EXPECT_FAILED, msg::RUN_EXPECT_FAILED_MESSAGE)
        } else {
            (msg::PANIC_ASSERT, msg::PANIC_ASSERT_MESSAGE)
        };
        let mut m = if message.is_empty() {
            rt::render(plain, &[("expr", expr.clone())])
        } else {
            rt::render(with, &[("expr", expr.clone()), ("message", message)])
        };
        if matches!(i, Intrinsic::AssertFailParts | Intrinsic::ExpectFailParts) {
            let parts = shown_parts(c.arg(2), c.ty(2));
            m.push_str(&parts_diagram(&expr, &parts));
        }
        if matches!(i, Intrinsic::AssertFailCompare | Intrinsic::ExpectFailCompare) {
            for (k, id) in [(2, msg::RUN_ASSERT_LEFT), (3, msg::RUN_ASSERT_RIGHT)] {
                let (p, t) = c.place(k);
                let mut out = Vec::new();
                value::debug(&mut out, p, t);
                m.push_str("\n  ");
                m.push_str(&rt::render(id, &[("value", String::from_utf8_lossy(&out).into_owned())]));
            }
        }
        // In a test, `expect` records the failure and goes on (decision 500).
        if recorded && let Some(status) = rt::expect_failed(&m) {
            return status;
        }
        rt::raise_msg(msg::PANIC_EXPLICIT, &[("message", m)])
    }
}

/// The parts a failed check shows: `(column, Debug text)` pairs from a
/// `list[(int, String)]`.
///
/// # Safety
/// `p` must hold a value of type `t`.
unsafe fn shown_parts(p: *mut u8, t: u32) -> Vec<(usize, String)> {
    // SAFETY: as the caller guarantees.
    unsafe {
        let (p, t) = target(p, t);
        let Kind::List(e) = ty(t).kind else { return Vec::new() };
        let Kind::Tuple(slots) = &ty(e).kind else { return Vec::new() };
        let (Some(col), Some(txt)) = (slots.first(), slots.get(1)) else { return Vec::new() };
        let l = mem::at::<List>(p);
        let size = ty(e).size;
        let mut out = Vec::new();
        for k in 0..l.len {
            let at = l.at(k, size);
            let c = (mem::word(at.add(col.offset as usize), 0) as i64).max(0) as usize;
            let s = value::text(at.add(txt.offset as usize), txt.ty).unwrap_or(&[]);
            out.push((c, String::from_utf8_lossy(s).into_owned()));
        }
        out
    }
}

/// The values of a failed condition's parts drawn under it, as the
/// interpreter draws them (`sisu-interp/src/format.rs`, `parts_diagram`).
fn parts_diagram(text: &str, parts: &[(usize, String)]) -> String {
    if parts.is_empty() || text.contains('\n') {
        return String::new();
    }
    let mut parts: Vec<(usize, String)> = parts.to_vec();
    parts.sort_by_key(|p| std::cmp::Reverse(p.0));
    parts.dedup_by_key(|p| p.0);
    let mut rows: Vec<Vec<(usize, String)>> = Vec::new();
    let mut depth = Vec::new();
    for (c, v) in &parts {
        let len = v.chars().count();
        let fits =
            |row: &Vec<(usize, String)>| row.iter().all(|(k, w)| c + len < *k || *k + w.chars().count() < *c);
        let r = match rows.iter().position(fits) {
            Some(r) => r,
            None => {
                rows.push(Vec::new());
                rows.len() - 1
            }
        };
        rows[r].push((*c, v.clone()));
        depth.push(r);
    }
    let width = parts.iter().map(|(c, v)| c + v.chars().count()).max().unwrap_or(0) + 1;
    let mut out = format!("\n    {text}");
    for r in 0..=rows.len() {
        let mut line: Vec<char> = vec![' '; width];
        for (i, (c, _)) in parts.iter().enumerate() {
            if (r == 0 || depth[i] >= r)
                && let Some(slot) = line.get_mut(*c)
            {
                *slot = '│';
            }
        }
        if r > 0 {
            for (c, v) in &rows[r - 1] {
                for (k, ch) in v.chars().enumerate() {
                    if let Some(slot) = line.get_mut(c + k) {
                        *slot = ch;
                    }
                }
            }
        }
        let s: String = line.into_iter().collect();
        out.push_str("\n    ");
        out.push_str(s.trim_end());
    }
    out
}

/// `fmt.float_digits`: the digits are the runtime in Sisu's (decision 358).
fn float_digits(x: f64, kind: Option<char>, precision: Option<usize>) -> String {
    use crate::bridge::{EXPONENT, FIXED, float_text};
    if x.is_infinite() {
        return "inf".into();
    }
    match kind {
        Some('f' | 'F') => float_text(x.to_bits(), FIXED, precision.unwrap_or(6)),
        Some('e' | 'E') => {
            let s = float_text(x.to_bits(), EXPONENT, precision.unwrap_or(6));
            if kind == Some('E') { s.to_uppercase() } else { s }
        }
        Some('%') => float_text((x * 100.0).to_bits(), FIXED, precision.unwrap_or(6)) + "%",
        _ => match precision {
            Some(p) => float_text(x.to_bits(), FIXED, p),
            None => value::float_repr(x),
        },
    }
}

/// Whether `s` is a decimal number as `str.parse_float` reads it: an
/// optional sign, digits with an optional fraction (or a fraction alone),
/// and an optional exponent; not `inf` or `nan`.
fn decimal(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - int_start;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - frac_start;
    }
    if digits == 0 {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == b.len()
}

/// How many `f64`s lie between two, when both are numbers.
fn ulps_between(a: f64, b: f64) -> Option<u64> {
    if a.is_nan() || b.is_nan() {
        return None;
    }
    let key = |x: f64| {
        let bits = x.to_bits() as i64;
        if bits < 0 { i64::MIN - bits } else { bits }
    };
    Some(key(a).abs_diff(key(b)))
}
