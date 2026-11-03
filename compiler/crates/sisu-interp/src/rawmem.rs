//! The checked memory model (D-085; 22, "Interaction with the rest of the
//! language", "The dynamic monitor"; docs/architecture.md, "The checked
//! memory model"): raw pointers, the memory they point into, and the
//! intrinsics of `std.mem`, the foreign calls of `extern` blocks and the
//! machine intrinsics of `std.rt`, as the interpreter runs them.
//!
//! **Blocks.** Raw memory is a set of blocks, each with an identity, an
//! address, a size, an alignment and whether it is live: what
//! `Global.acquire` gave out (and the foreign `malloc` the interpreter
//! emulates), and the places of the program a pointer was made from
//! (`ptr.of`, `ptr.of_mut`, a slice's `as_ptr`), whose reads and writes go
//! to the place itself. A raw pointer is a block, an offset and the loan it
//! was made through (`value::RawPtr`); its type, and so the size and the
//! alignment of what it points to, is the static type of the operand that
//! holds it, as native code has it.
//!
//! **What a block holds.** A block of `Global` holds values at offsets,
//! each with the type it was written as, and bytes of plain data where a
//! write cut a plain value apart. A read at the type a value was written
//! as gives the value, moved out when the type needs a drop (the slot then
//! holds none); a read at another type goes through the bytes, which it
//! can only for types whose bits the model knows: the integers, the
//! floats, `bool`, `char`, and structs, tuples and enums of them. Every
//! access checks, in this order, that the pointer has memory behind it,
//! that the memory is live, that the access lies inside its block, that it
//! is aligned for its type, that what it reads holds a value of that type
//! with valid bits, and that a write neither goes through a pointer made
//! from a shared borrow nor overwrites a value that was never dropped. Each
//! failure is a panic that names the obligation of the operation's
//! `Safety:` list in std.mem that it broke.
//!
//! **The layout of a type in raw memory** (`mem.size_of`, `mem.align_of`)
//! is the native backend's, so that a program computes the same sizes
//! either way: a sized integer narrower than 64 bits and an `f32` take
//! their own width, a struct or a tuple its parts in C's order with their
//! padding, an `Option` of a raw pointer one word (null for `None`), and
//! every other type the words of its value (decision 188), aligned to a
//! word.
//!
//! **Loans.** Under the monitor a pointer made from a place carries the
//! loan of the borrow that made it, and every access through it goes
//! through that loan, as an access through a reference does; a reference
//! made from a pointer (`as_ref`, `as_mut`) is a loan under it. An access
//! that conflicts with a live loan ends it, and a later use of the loan
//! is the violation the monitor reports.

use crate::monitor::Op;
use crate::value::{Pointer, RAW, RawPtr, Step, Value};
use crate::{Interp, Unwind, ice, msg, panic};
use sisu_sir as sir;
use sisu_sir::Type;
use std::collections::{BTreeMap, HashMap};

/// The bytes of a word.
const WORD: u64 = 8;

/// The address the first block starts at; blocks are laid out after it in
/// the order made, aligned, with a gap between them, so that no address is
/// one block's end and the next one's start.
const FIRST_ADDRESS: u64 = 0x10_0000;

/// The C functions a foreign call may name in the interpreter (Q-55).
const EMULATED: &str = "strlen, memcpy, malloc, free, getenv, write";

/// The size and alignment of a type in raw memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub size: u64,
    pub align: u64,
}

/// A number, as raw memory holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scalar {
    Unit,
    Int,
    U64,
    Sized {
        bytes: u8,
        signed: bool,
    },
    I128,
    U128,
    F64,
    F32,
    /// A word holding 0 or 1.
    Bool,
    /// A word holding a Unicode scalar value.
    Char,
    DateTime,
}

/// How a type lies in raw memory.
enum Shape {
    Scalar(Scalar),
    /// A struct's fields or a tuple's elements, at their offsets.
    Parts(Vec<(u64, Type)>),
    /// `ptr[T]` and `ptr mut T`: an address with its provenance.
    Ptr,
    /// `Option` of a raw pointer: the pointer, or a null word for `None`.
    Nullable,
    /// The words of the value, as native code lays it out (decision 188).
    Words,
}

/// What a value of a type is to the bytes: why the model cannot give or
/// take its bits.
enum Bad {
    /// The model does not know the type's bits (a value that owns memory,
    /// a reference, a pointer).
    Opaque,
    /// The bits are not a valid value of the type: what they are.
    Invalid(String),
}

/// What a block of raw memory is.
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    /// `Global.acquire`'s, which `Global.release` takes back.
    Global,
    /// The emulated `malloc`'s, which `free` takes back.
    Malloc,
    /// What a foreign function handed out to be read (`getenv`'s text), or
    /// the value `view_plain` made of a `bytes`: never written or released.
    ReadOnly,
    /// A place of the program: a value of type `ty`, or for a slice's
    /// `as_ptr`, its `n` values of type `ty`.
    Place { at: Pointer, ty: Type, elems: Option<u64> },
}

/// What a block holds at an offset.
#[derive(Clone, Debug)]
pub(crate) enum Item {
    /// A value of the type it was written as.
    Value(Type, Value),
    /// One byte of plain data, what is left of a plain value a write cut.
    Byte(u8),
}

#[derive(Clone, Debug)]
pub(crate) struct Block {
    pub base: u64,
    pub size: u64,
    pub align: u64,
    pub live: bool,
    /// Its identity for the monitor's loans, from the run's serials.
    pub serial: u64,
    pub kind: Kind,
    pub items: BTreeMap<u64, Item>,
}

/// What names a place of the program: (task, frame, serial, local, path,
/// and whether the block is of a slice's values).
type PlaceKey = (u32, u32, u64, u32, Vec<Step>, bool);

/// The raw memory of a run.
#[derive(Default)]
pub(crate) struct RawMem {
    pub blocks: Vec<Block>,
    next: u64,
    /// The block made for each place, so that two pointers to one place
    /// have one address.
    places: HashMap<PlaceKey, u32>,
}

fn align_up(n: u64, align: u64) -> u64 {
    let a = align.max(1);
    n.div_ceil(a) * a
}

/// Whether a reference to the type is two words (a `dyn`, an `Iterable`).
fn fat(t: &Type) -> bool {
    match t {
        Type::Dyn(..) => true,
        Type::Named(n, _) => n == "Iterable",
        _ => false,
    }
}

/// The builtin types that are not laid out as structs, though SIR declares
/// some of them as structs without fields.
fn builtin(n: &str) -> bool {
    matches!(
        n,
        "String"
            | "Path"
            | "list"
            | "Array"
            | "Slice"
            | "bytes"
            | "Iter"
            | "Box"
            | "dict"
            | "set"
            | "Scope"
            | "Handle"
            | "Cancellation"
            | "Sender"
            | "Receiver"
            | "MutexGuard"
            | "ReadGuard"
            | "Timer"
            | "Mutex"
            | "RwLock"
            | "Shared"
            | "Weak"
            | "Arena"
            | "Cell"
            | "Atomic"
            | "Cap"
            | "TestClock"
            | "HostStream"
            | "HostListener"
            | "Iterable"
            | "Lent"
            | "Entry"
            | "System"
            | "Option"
            | "Result"
            | "Fs"
            | "Net"
            | "Clock"
            | "Rand"
            | "Env"
            | "Proc"
            | "Stdin"
            | "HostFs"
            | "HostNet"
            | "HostClock"
            | "HostRand"
            | "HostEnv"
            | "HostProc"
            | "HostStdin"
    )
}

/// A raw pointer's type: `ptr[T]`, `ptr mut T`, through references.
fn pointee(t: Option<&Type>) -> Option<Type> {
    let mut t = t?;
    while let Type::Ref(inner, _) = t {
        t = inner;
    }
    match t {
        Type::Named(n, args) if n == "ptr" || n == "ptr_mut" => args.first().cloned(),
        _ => None,
    }
}

/// The text of a type, for a message.
fn shown(t: &Type) -> String {
    sir::write::ty(t)
}

/// The operation an intrinsic's name stands for, as a message names it.
fn op_name(name: &str) -> String {
    match name.strip_prefix("mem.") {
        Some(rest) => rest.to_string(),
        None => name.strip_prefix("extern.c.").unwrap_or(name).to_string(),
    }
}

impl RawMem {
    /// A new block; its address follows the last block's.
    fn add(&mut self, size: u64, align: u64, kind: Kind, serial: u64) -> u32 {
        if self.next == 0 {
            self.next = FIRST_ADDRESS;
        }
        let align = align.max(1);
        let base = align_up(self.next, align.max(16));
        self.next = base + size.max(1) + 16;
        self.blocks.push(Block { base, size, align, live: true, serial, kind, items: BTreeMap::new() });
        (self.blocks.len() - 1) as u32
    }
}

impl<'p> Interp<'p, '_> {
    // ------------------------------------------------------------ layouts

    /// The size of a value of the type as native code lays it out (decision
    /// 188; `sisu-codegen/src/layout.rs`): the words of a value. A function
    /// value is taken to have one word of environment, the least a program
    /// gives it natively.
    fn value_size(&self, t: &Type, depth: u32) -> Result<u64, Unwind> {
        if depth > 64 {
            return Err(ice(format!("the type `{}` holds itself", shown(t))));
        }
        let size = |me: &Self, t: &Type| me.value_size(t, depth + 1);
        Ok(match t {
            Type::Ref(inner, _) if fat(inner) => 2 * WORD,
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Ref(..) => WORD,
            Type::Str | Type::Dyn(..) => 2 * WORD,
            Type::Fn(..) => 3 * WORD,
            Type::Unit | Type::Never | Type::Const(_) => 0,
            Type::Tuple(items) => {
                let mut n = 0;
                for it in items {
                    n += size(self, it)?;
                }
                n
            }
            Type::ErrSet(names) => {
                let mut n = 3 * WORD;
                for name in names {
                    n = n.max(size(self, &Type::Named(name.clone(), Vec::new()))?);
                }
                n
            }
            Type::Named(n, args) => {
                let arg = |i: usize| args.get(i).cloned().unwrap_or(Type::Unit);
                match n.as_str() {
                    "String" | "Path" | "list" | "Array" | "Slice" | "bytes" => 3 * WORD,
                    "i128" | "u128" | "Timer" | "Iterable" => 2 * WORD,
                    "Mutex" | "RwLock" => 4 * WORD + size(self, &arg(0))?,
                    "Cell" | "Atomic" | "Lent" => size(self, &arg(0))?,
                    "Entry" => WORD + size(self, &arg(0))?,
                    "System" => 7 * WORD,
                    "Option" => WORD + size(self, &arg(0))?,
                    "Result" => WORD + size(self, &arg(0))?.max(size(self, &arg(1))?),
                    other if builtin(other) => WORD,
                    "u64" | "f32" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "DateTime" | "ptr"
                    | "ptr_mut" => WORD,
                    other => match self.types.get(other) {
                        Some(sir::TypeDecl::Struct { fields, .. }) => {
                            let mut s = 0;
                            for f in fields {
                                s += size(self, &f.ty.subst(args))?;
                            }
                            s
                        }
                        Some(sir::TypeDecl::Enum { variants, error, .. }) => {
                            let start = if *error { 3 * WORD } else { WORD };
                            let mut s = start;
                            for v in variants {
                                let mut off = start;
                                for (_, ft) in &v.fields {
                                    off += size(self, &ft.subst(args))?;
                                }
                                s = s.max(off);
                            }
                            s
                        }
                        None => return Err(ice(format!("the size of the type `{other}`"))),
                    },
                }
            }
            other => return Err(ice(format!("the size of the type `{}`", shown(other)))),
        })
    }

    /// How a type lies in raw memory, and its layout there.
    fn shape(&self, t: &Type, depth: u32) -> Result<(Layout, Shape), Unwind> {
        if depth > 64 {
            return Err(ice(format!("the type `{}` holds itself", shown(t))));
        }
        let scalar =
            |size: u64, s: Scalar| Ok((Layout { size, align: size.clamp(1, WORD) }, Shape::Scalar(s)));
        match t {
            Type::Unit | Type::Never => {
                return Ok((Layout { size: 0, align: 1 }, Shape::Scalar(Scalar::Unit)));
            }
            Type::Int => return scalar(WORD, Scalar::Int),
            Type::Float => return scalar(WORD, Scalar::F64),
            Type::Bool => return scalar(WORD, Scalar::Bool),
            Type::Char => return scalar(WORD, Scalar::Char),
            Type::Tuple(items) => return self.c_layout(items.iter().cloned(), depth),
            Type::Named(n, args) => {
                let sized =
                    |bytes: u8, signed: bool| scalar(u64::from(bytes), Scalar::Sized { bytes, signed });
                match n.as_str() {
                    "u8" => return sized(1, false),
                    "i8" => return sized(1, true),
                    "u16" => return sized(2, false),
                    "i16" => return sized(2, true),
                    "u32" => return sized(4, false),
                    "i32" => return sized(4, true),
                    "u64" => return scalar(WORD, Scalar::U64),
                    "f32" => return scalar(4, Scalar::F32),
                    "i128" => return Ok((Layout { size: 16, align: WORD }, Shape::Scalar(Scalar::I128))),
                    "u128" => return Ok((Layout { size: 16, align: WORD }, Shape::Scalar(Scalar::U128))),
                    "DateTime" => return scalar(WORD, Scalar::DateTime),
                    "ptr" | "ptr_mut" => return Ok((Layout { size: WORD, align: WORD }, Shape::Ptr)),
                    "Option" if pointee(args.first()).is_some() => {
                        return Ok((Layout { size: WORD, align: WORD }, Shape::Nullable));
                    }
                    other if !builtin(other) => {
                        if let Some(sir::TypeDecl::Struct { fields, .. }) = self.types.get(other) {
                            let fs: Vec<Type> = fields.iter().map(|f| f.ty.subst(args)).collect();
                            return self.c_layout(fs.into_iter(), depth);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok((Layout { size: self.value_size(t, depth)?, align: WORD }, Shape::Words))
    }

    /// Parts in C's order: each at the next offset its alignment allows,
    /// the whole rounded up to the largest alignment.
    fn c_layout(&self, parts: impl Iterator<Item = Type>, depth: u32) -> Result<(Layout, Shape), Unwind> {
        let (mut off, mut align) = (0, 1);
        let mut out = Vec::new();
        for p in parts {
            let (l, _) = self.shape(&p, depth + 1)?;
            off = align_up(off, l.align);
            out.push((off, p));
            off += l.size;
            align = align.max(l.align);
        }
        Ok((Layout { size: align_up(off, align), align }, Shape::Parts(out)))
    }

    /// The layout of a type in raw memory (`mem.size_of`, `mem.align_of`).
    pub(crate) fn mem_layout(&self, t: &Type) -> Result<Layout, Unwind> {
        Ok(self.shape(t, 0)?.0)
    }

    /// Whether dropping a value of the type can do anything: whether a
    /// value of it that is never dropped is lost, and whether a read moves
    /// it out.
    pub(crate) fn type_needs_drop(&self, t: &Type) -> bool {
        self.needs_drop_at(t, 0)
    }

    fn needs_drop_at(&self, t: &Type, depth: u32) -> bool {
        if depth > 32 {
            return true;
        }
        let any = |ts: &[Type]| ts.iter().any(|x| self.needs_drop_at(x, depth + 1));
        match t {
            Type::Int
            | Type::Float
            | Type::Bool
            | Type::Char
            | Type::Str
            | Type::Unit
            | Type::Never
            | Type::Ref(..)
            | Type::Const(_)
            | Type::Fn(..) => false,
            Type::Tuple(ts) => any(ts),
            Type::Named(n, args) => match n.as_str() {
                "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i128" | "f32"
                | "DateTime" | "ptr" | "ptr_mut" => false,
                "Option" | "Result" | "Cell" | "Atomic" => any(args),
                other if builtin(other) => true,
                other => {
                    if self.glue.contains_key(other) {
                        return true;
                    }
                    match self.types.get(other) {
                        Some(sir::TypeDecl::Struct { fields, .. }) => {
                            fields.iter().any(|f| self.needs_drop_at(&f.ty.subst(args), depth + 1))
                        }
                        Some(sir::TypeDecl::Enum { variants, error, .. }) => {
                            *error
                                || variants.iter().any(|v| {
                                    v.fields
                                        .iter()
                                        .any(|(_, ft)| self.needs_drop_at(&ft.subst(args), depth + 1))
                                })
                        }
                        None => true,
                    }
                }
            },
            _ => true,
        }
    }

    // ------------------------------------------------------------ bits

    /// Appends the bytes of `v`, a value of type `t`, as raw memory holds
    /// it; `Bad::Opaque` for a value whose bits the model does not know.
    fn encode(&self, v: &Value, t: &Type, out: &mut Vec<u8>) -> Result<(), Bad> {
        let start = out.len();
        let (layout, shape) = self.shape(t, 0).map_err(|_| Bad::Opaque)?;
        match shape {
            Shape::Scalar(s) => encode_scalar(v, s, out)?,
            Shape::Parts(parts) => {
                let fields: &[Value] = match v {
                    Value::Struct(a) => &a.fields,
                    Value::Tuple(xs) => xs,
                    _ => return Err(Bad::Opaque),
                };
                for ((off, pt), fv) in parts.iter().zip(fields) {
                    out.resize(start + *off as usize, 0);
                    self.encode(fv, pt, out)?;
                }
            }
            Shape::Words => self.encode_words(v, t, out, 0)?,
            Shape::Ptr | Shape::Nullable => return Err(Bad::Opaque),
        }
        out.resize(start + layout.size as usize, 0);
        Ok(())
    }

    /// The words of a value as native code lays it out (decision 188), for
    /// the enums whose fields' bits the model knows.
    fn encode_words(&self, v: &Value, t: &Type, out: &mut Vec<u8>, depth: u32) -> Result<(), Bad> {
        if depth > 32 {
            return Err(Bad::Opaque);
        }
        let start = out.len();
        let size = self.value_size(t, 0).map_err(|_| Bad::Opaque)?;
        match (t, v) {
            (Type::Int | Type::Float | Type::Bool | Type::Char, _) => {
                let s = match t {
                    Type::Int => Scalar::Int,
                    Type::Float => Scalar::F64,
                    Type::Bool => Scalar::Bool,
                    _ => Scalar::Char,
                };
                encode_scalar(v, s, out)?
            }
            (Type::Unit, _) => {}
            (Type::Tuple(ts), Value::Tuple(xs)) => {
                for (x, xt) in xs.iter().zip(ts) {
                    self.encode_words(x, xt, out, depth + 1)?;
                }
            }
            (Type::Named(n, args), _) => match n.as_str() {
                "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => match v {
                    Value::Int(i) => out.extend_from_slice(&i.to_le_bytes()),
                    _ => return Err(Bad::Opaque),
                },
                "f32" => match v {
                    Value::F32(f) => out.extend_from_slice(&u64::from(f.to_bits()).to_le_bytes()),
                    _ => return Err(Bad::Opaque),
                },
                "u64" | "i128" | "u128" | "DateTime" => {
                    let s = match n.as_str() {
                        "u64" => Scalar::U64,
                        "i128" => Scalar::I128,
                        "u128" => Scalar::U128,
                        _ => Scalar::DateTime,
                    };
                    encode_scalar(v, s, out)?
                }
                _ => {
                    let (variant, fields): (u32, &[Value]) = match v {
                        Value::Struct(a) => (u32::MAX, &a.fields),
                        Value::Variant(a) => (a.variant, &a.fields),
                        _ => return Err(Bad::Opaque),
                    };
                    let field_tys: Vec<Type> = match (n.as_str(), self.types.get(n.as_str())) {
                        ("Option", _) if variant == 0 => args.first().cloned().into_iter().collect(),
                        ("Result", _) if variant <= 1 => {
                            args.get(variant as usize).cloned().into_iter().collect()
                        }
                        ("Option" | "Result", _) => Vec::new(),
                        (_, Some(sir::TypeDecl::Struct { fields, .. })) if variant == u32::MAX => {
                            fields.iter().map(|f| f.ty.subst(args)).collect()
                        }
                        (_, Some(sir::TypeDecl::Enum { variants, error: false, .. })) => variants
                            .get(variant as usize)
                            .map(|vd| vd.fields.iter().map(|(_, ft)| ft.subst(args)).collect())
                            .ok_or(Bad::Opaque)?,
                        _ => return Err(Bad::Opaque),
                    };
                    if variant != u32::MAX {
                        out.extend_from_slice(&u64::from(variant).to_le_bytes());
                    }
                    for (fv, ft) in fields.iter().zip(&field_tys) {
                        self.encode_words(fv, ft, out, depth + 1)?;
                    }
                }
            },
            _ => return Err(Bad::Opaque),
        }
        out.resize(start + size as usize, 0);
        Ok(())
    }

    /// The value of type `t` that `bytes` (exactly its size) hold.
    fn decode(&self, bytes: &[u8], t: &Type) -> Result<Value, Bad> {
        let (_, shape) = self.shape(t, 0).map_err(|_| Bad::Opaque)?;
        match shape {
            Shape::Scalar(s) => decode_scalar(bytes, s),
            Shape::Parts(parts) => {
                let mut fields = Vec::with_capacity(parts.len());
                for (off, pt) in &parts {
                    let l = self.mem_layout(pt).map_err(|_| Bad::Opaque)?;
                    let part = bytes.get(*off as usize..(*off + l.size) as usize).ok_or(Bad::Opaque)?;
                    fields.push(self.decode(part, pt)?);
                }
                Ok(match t {
                    Type::Tuple(_) => Value::Tuple(fields),
                    Type::Named(n, _) => Value::Struct(Value::adt(n, 0, fields)),
                    _ => return Err(Bad::Opaque),
                })
            }
            Shape::Words => self.decode_words(bytes, t, 0).map(|(v, _)| v),
            Shape::Ptr | Shape::Nullable => Err(Bad::Opaque),
        }
    }

    /// [`Interp::decode`] of the words of a value: the value, and the bytes
    /// it took.
    fn decode_words(&self, bytes: &[u8], t: &Type, depth: u32) -> Result<(Value, usize), Bad> {
        if depth > 32 {
            return Err(Bad::Opaque);
        }
        let size = self.value_size(t, 0).map_err(|_| Bad::Opaque)? as usize;
        let here = bytes.get(..size).ok_or(Bad::Opaque)?;
        let word = |b: &[u8]| -> Result<[u8; 8], Bad> {
            b.get(..8).and_then(|w| <[u8; 8]>::try_from(w).ok()).ok_or(Bad::Opaque)
        };
        let v = match t {
            Type::Int => decode_scalar(here, Scalar::Int)?,
            Type::Float => decode_scalar(here, Scalar::F64)?,
            Type::Bool => decode_scalar(here, Scalar::Bool)?,
            Type::Char => decode_scalar(here, Scalar::Char)?,
            Type::Unit => Value::Unit,
            Type::Tuple(ts) => {
                let mut at = 0;
                let mut xs = Vec::new();
                for xt in ts {
                    let (x, n) = self.decode_words(&here[at..], xt, depth + 1)?;
                    xs.push(x);
                    at += n;
                }
                Value::Tuple(xs)
            }
            Type::Named(n, args) => match n.as_str() {
                "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => {
                    let i = i64::from_le_bytes(word(here)?);
                    let (bytes, signed) = match n.as_str() {
                        "u8" => (1, false),
                        "i8" => (1, true),
                        "u16" => (2, false),
                        "i16" => (2, true),
                        "u32" => (4, false),
                        _ => (4, true),
                    };
                    let bits = 8 * bytes;
                    let ok = if signed {
                        i >= -(1i64 << (bits - 1)) && i < (1i64 << (bits - 1))
                    } else {
                        i >= 0 && i < (1i64 << bits)
                    };
                    if !ok {
                        return Err(Bad::Invalid(format!("{:#x}", i as u64)));
                    }
                    Value::Int(i)
                }
                "f32" => {
                    let w = u64::from_le_bytes(word(here)?);
                    Value::F32(f32::from_bits(w as u32))
                }
                "u64" => decode_scalar(here, Scalar::U64)?,
                "i128" => decode_scalar(here, Scalar::I128)?,
                "u128" => decode_scalar(here, Scalar::U128)?,
                "DateTime" => decode_scalar(here, Scalar::DateTime)?,
                _ => {
                    let decl = self.types.get(n.as_str());
                    if let Some(sir::TypeDecl::Struct { fields, .. }) = decl
                        && !builtin(n)
                    {
                        let mut at = 0;
                        let mut xs = Vec::new();
                        for f in fields {
                            let (x, k) = self.decode_words(&here[at..], &f.ty.subst(args), depth + 1)?;
                            xs.push(x);
                            at += k;
                        }
                        return Ok((Value::Struct(Value::adt(n, 0, xs)), size));
                    }
                    let tag = u64::from_le_bytes(word(here)?);
                    let field_tys: Vec<Type> = match (n.as_str(), decl) {
                        ("Option", _) if tag == 0 => args.first().cloned().into_iter().collect(),
                        ("Option", _) if tag == 1 => Vec::new(),
                        ("Result", _) if tag <= 1 => args.get(tag as usize).cloned().into_iter().collect(),
                        ("Option" | "Result", _) => return Err(Bad::Invalid(format!("the tag {tag}"))),
                        (_, Some(sir::TypeDecl::Enum { variants, error: false, .. })) => {
                            match variants.get(tag as usize) {
                                Some(vd) => vd.fields.iter().map(|(_, ft)| ft.subst(args)).collect(),
                                None => return Err(Bad::Invalid(format!("the tag {tag}"))),
                            }
                        }
                        _ => return Err(Bad::Opaque),
                    };
                    let mut at = WORD as usize;
                    let mut xs = Vec::new();
                    for ft in &field_tys {
                        let (x, k) = self.decode_words(&here[at..], ft, depth + 1)?;
                        xs.push(x);
                        at += k;
                    }
                    Value::variant(n, tag as u32, xs)
                }
            },
            _ => return Err(Bad::Opaque),
        };
        Ok((v, size))
    }

    // ------------------------------------------------------------ pointers

    /// The raw pointer a value is or refers to.
    fn raw_ptr(&self, v: &Value) -> Result<RawPtr, Unwind> {
        match self.target(v)? {
            Value::RawPtr(p) => Ok((**p).clone()),
            other => Err(ice(format!("expected a raw pointer, found {other:?}"))),
        }
    }

    /// The raw pointer an `Option` of one holds, if any.
    fn nullable(&self, v: &Value) -> Result<Option<RawPtr>, Unwind> {
        match self.target(v)? {
            Value::RawPtr(p) => Ok(Some((**p).clone())),
            Value::Variant(a) if a.variant == 0 => match a.fields.first() {
                Some(p) => self.raw_ptr(p).map(Some),
                None => Err(ice("an option without its payload")),
            },
            Value::Variant(_) => Ok(None),
            other => Err(ice(format!("expected a raw pointer, found {other:?}"))),
        }
    }

    fn ptr_value(p: RawPtr) -> Value {
        Value::RawPtr(Box::new(p))
    }

    /// The address a pointer holds.
    fn address(&self, p: &RawPtr) -> i64 {
        match p.block.and_then(|b| self.raw.blocks.get(b as usize)) {
            Some(b) => b.base as i64 + p.off,
            None => p.off,
        }
    }

    /// The block made for a place (or a slice's values), made once.
    fn place_block(&mut self, at: Pointer, ty: Type, elems: Option<u64>) -> Result<u32, Unwind> {
        let l = self.mem_layout(&ty)?;
        let size = l.size * elems.unwrap_or(1);
        let key = (at.task, at.frame, at.serial, at.local, at.path.clone(), elems.is_some());
        if let Some(&b) = self.raw.places.get(&key) {
            // A slice's values are as many as it has now.
            if let Some(block) = self.raw.blocks.get_mut(b as usize) {
                block.size = size;
                block.kind = Kind::Place { at, ty, elems };
            }
            return Ok(b);
        }
        self.serial += 1;
        let b = self.raw.add(size, l.align, Kind::Place { at, ty, elems }, self.serial);
        self.raw.places.insert(key, b);
        Ok(b)
    }

    /// Checks an access of `bytes` at `p` for `op`: memory behind the
    /// pointer, live, inside its block and aligned to `align`. Gives the
    /// block.
    #[allow(clippy::too_many_arguments)]
    fn check_access(
        &self,
        p: &RawPtr,
        op: &str,
        ty: &Type,
        l: Layout,
        obligation: &str,
    ) -> Result<u32, Unwind> {
        let Some(b) = p.block else {
            if l.size == 0 && p.off != 0 && (p.off as u64).is_multiple_of(l.align) {
                // A zero-sized access through an aligned pointer touches
                // nothing (a dangling pointer of an empty buffer).
                return Ok(u32::MAX);
            }
            return Err(panic(
                msg::PANIC_RAW_NO_MEMORY,
                &[("op", format!("ptr.{op}")), ("obligation", obligation.into())],
            ));
        };
        let block = self.raw.blocks.get(b as usize).ok_or_else(|| ice("a pointer into no block"))?;
        if !block.live {
            return Err(panic(
                msg::PANIC_RAW_RELEASED,
                &[("op", format!("ptr.{op}")), ("obligation", obligation.into())],
            ));
        }
        if p.off < 0 || p.off as u64 + l.size > block.size {
            return Err(panic(
                msg::PANIC_RAW_BOUNDS,
                &[
                    ("op", format!("ptr.{op}")),
                    ("bytes", l.size.to_string()),
                    ("offset", p.off.to_string()),
                    ("size", block.size.to_string()),
                    ("obligation", obligation.into()),
                ],
            ));
        }
        if !(block.base + p.off as u64).is_multiple_of(l.align) {
            return Err(panic(
                msg::PANIC_RAW_MISALIGNED,
                &[
                    ("op", format!("ptr.{op}")),
                    ("ty", shown(ty)),
                    ("offset", p.off.to_string()),
                    ("align", l.align.to_string()),
                ],
            ));
        }
        Ok(b)
    }

    /// The place a pointer into a place's block reaches, the type of the
    /// value there, and the offset into that value: `None` when the place
    /// is gone.
    fn place_of(&self, b: u32, off: u64) -> Result<(Pointer, Type, u64), Unwind> {
        let block = self.raw.blocks.get(b as usize).ok_or_else(|| ice("a pointer into no block"))?;
        let Kind::Place { at, ty, elems } = &block.kind else {
            return Err(ice("a place's block that is none"));
        };
        let size = self.mem_layout(ty)?.size;
        match elems {
            Some(_) if size > 0 => Ok((at.join(Step::Index((off / size) as usize)), ty.clone(), off % size)),
            Some(_) => Ok((at.join(Step::Index(0)), ty.clone(), 0)),
            None => Ok((at.clone(), ty.clone(), off)),
        }
    }

    /// Whether the frame or heap cell a place lives in is still there.
    fn place_alive(&self, at: &Pointer) -> bool {
        if at.frame == crate::value::HEAP {
            return self.heap.get(at.local as usize).is_some_and(|c| c.serial == at.serial && c.strong > 0);
        }
        if at.frame == RAW {
            return self.raw.blocks.get(at.local as usize).is_some_and(|b| b.serial == at.serial && b.live);
        }
        self.frames_of(at.task).get(at.frame as usize).is_some_and(|f| f.serial == at.serial)
    }

    /// The pointer the monitor sees an access through `p` at as: the
    /// place, or the block and offset, with the pointer's loan. A place
    /// that is gone is the panic of `op`, before the monitor looks at the
    /// loan, so that a run gives the same panic with the monitor or without.
    fn access_pointer(&self, p: &RawPtr, b: u32, op: &str, obligation: &str) -> Result<Pointer, Unwind> {
        let block = self.raw.blocks.get(b as usize).ok_or_else(|| ice("a pointer into no block"))?;
        let mut at = match &block.kind {
            Kind::Place { .. } => {
                let at = self.place_of(b, p.off as u64)?.0;
                if !self.place_alive(&at) {
                    return Err(panic(
                        msg::PANIC_RAW_GONE,
                        &[("op", format!("ptr.{op}")), ("obligation", obligation.into())],
                    ));
                }
                at
            }
            _ => Pointer {
                task: 0,
                frame: RAW,
                serial: block.serial,
                local: b,
                path: vec![Step::Raw(p.off as u64)],
                tag: 0,
            },
        };
        at.tag = p.tag;
        Ok(at)
    }

    // ------------------------------------------------------------ reads and writes

    /// Reads the `ty` at `p`: `None` when the memory holds no value there
    /// at all. A value of a type that needs a drop is moved out when `take`.
    fn slot_read(&mut self, p: &RawPtr, ty: &Type, take: bool, op: &str) -> Result<Option<Value>, Unwind> {
        let l = self.mem_layout(ty)?;
        let b = self.check_access(p, op, ty, l, "valid")?;
        if b == u32::MAX {
            return Ok(Some(self.decode(&[], ty).map_err(|_| ice("a zero-sized value of no type"))?));
        }
        let at = self.access_pointer(p, b, op, "valid")?;
        self.mon_access(&at, if take { Op::Move } else { Op::Read })?;
        let off = p.off as u64;
        if let Some(Kind::Place { .. }) = self.raw.blocks.get(b as usize).map(|x| &x.kind) {
            return self.place_read(b, off, ty, take, op);
        }
        let block = &self.raw.blocks[b as usize];
        // The value written at this offset as this type.
        if let Some(Item::Value(t, _)) = block.items.get(&off)
            && (t == ty || same_pointer(t, ty))
        {
            if take && self.type_needs_drop(ty) {
                let Some(Item::Value(_, v)) = self.raw.blocks[b as usize].items.remove(&off) else {
                    return Err(ice("a value read that went"));
                };
                return Ok(Some(v));
            }
            let Some(Item::Value(_, v)) = self.raw.blocks[b as usize].items.get(&off) else {
                return Err(ice("a value read that went"));
            };
            return Ok(Some(v.clone()));
        }
        if l.size == 0 {
            return Ok(Some(self.decode(&[], ty).map_err(|_| ice("a zero-sized value of no type"))?));
        }
        // Otherwise through the bytes.
        let bytes = self.block_bytes(b, off, l.size, ty, op, "valid")?;
        match bytes {
            None => Ok(None),
            Some(bytes) => match self.decode(&bytes, ty) {
                Ok(v) => Ok(Some(v)),
                Err(Bad::Invalid(bits)) => Err(panic(
                    msg::PANIC_RAW_INVALID_BITS,
                    &[
                        ("op", format!("ptr.{op}")),
                        ("bits", bits),
                        ("ty", shown(ty)),
                        ("obligation", "valid".into()),
                    ],
                )),
                Err(Bad::Opaque) => Err(self.mistyped(b, off, ty, op, "valid")),
            },
        }
    }

    /// The bytes of a block from `off` for `n` bytes, through the values
    /// and bytes it holds there: `None` when no byte of them holds a value.
    /// A byte that holds none among others that do, or part of a value
    /// whose bits the model does not know, is the panic of `op`.
    fn block_bytes(
        &self,
        b: u32,
        off: u64,
        n: u64,
        ty: &Type,
        op: &str,
        obligation: &str,
    ) -> Result<Option<Vec<u8>>, Unwind> {
        let block = &self.raw.blocks[b as usize];
        let mut out: Vec<Option<u8>> = vec![None; n as usize];
        let end = off + n;
        // The items that may overlap: from the last one starting at or
        // before `off`.
        let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
        for (&at, item) in block.items.range(first..end) {
            match item {
                Item::Byte(x) => {
                    if at >= off {
                        out[(at - off) as usize] = Some(*x);
                    }
                }
                Item::Value(t, v) => {
                    let size = self.mem_layout(t)?.size;
                    if at + size <= off {
                        continue;
                    }
                    let mut bytes = Vec::new();
                    if self.encode(v, t, &mut bytes).is_err() {
                        return Err(self.mistyped(b, off, ty, op, obligation));
                    }
                    for (k, x) in bytes.iter().enumerate() {
                        let a = at + k as u64;
                        if a >= off && a < end {
                            out[(a - off) as usize] = Some(*x);
                        }
                    }
                }
            }
        }
        if out.iter().all(Option::is_none) {
            return Ok(None);
        }
        if out.iter().any(Option::is_none) {
            return Err(self.uninitialised(b, off, ty, op, obligation));
        }
        Ok(Some(out.into_iter().flatten().collect()))
    }

    fn uninitialised(&self, b: u32, off: u64, ty: &Type, op: &str, obligation: &str) -> Unwind {
        let size = self.raw.blocks.get(b as usize).map_or(0, |x| x.size);
        panic(
            msg::PANIC_RAW_UNINITIALISED,
            &[
                ("op", format!("ptr.{op}")),
                ("ty", shown(ty)),
                ("offset", off.to_string()),
                ("size", size.to_string()),
                ("obligation", obligation.into()),
            ],
        )
    }

    /// What the memory at `off` holds that is not a `ty`.
    fn mistyped(&self, b: u32, off: u64, ty: &Type, op: &str, obligation: &str) -> Unwind {
        let block = &self.raw.blocks[b as usize];
        let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
        let held = block
            .items
            .range(first..)
            .find_map(|(_, i)| match i {
                Item::Value(t, _) if !self.encodable(t) => Some(shown(t)),
                _ => None,
            })
            .unwrap_or_else(|| "value of another type".into());
        panic(
            msg::PANIC_RAW_MISTYPED,
            &[
                ("op", format!("ptr.{op}")),
                ("ty", shown(ty)),
                ("offset", off.to_string()),
                ("held", held),
                ("obligation", obligation.into()),
            ],
        )
    }

    /// Whether the model knows the bits of the type's values.
    fn encodable(&self, t: &Type) -> bool {
        match self.shape(t, 0) {
            Ok((_, Shape::Scalar(_))) => true,
            Ok((_, Shape::Parts(ps))) => ps.iter().all(|(_, pt)| self.encodable(pt)),
            _ => false,
        }
    }

    /// A read through a pointer into a place of the program.
    fn place_read(
        &mut self,
        b: u32,
        off: u64,
        ty: &Type,
        take: bool,
        op: &str,
    ) -> Result<Option<Value>, Unwind> {
        let (at, pty, inner) = self.place_of(b, off)?;
        if !self.place_alive(&at) {
            return Err(panic(
                msg::PANIC_RAW_GONE,
                &[("op", format!("ptr.{op}")), ("obligation", "valid".into())],
            ));
        }
        let current = self.read(&at)?;
        if current.is_moved() {
            return Ok(None);
        }
        if inner == 0 && (pty == *ty || same_pointer(&pty, ty)) {
            if take && self.type_needs_drop(ty) {
                return Ok(Some(std::mem::replace(self.write(&at)?, Value::Moved)));
            }
            return Ok(Some(self.read(&at)?.clone()));
        }
        // Another type: through the bytes of the value there.
        let l = self.mem_layout(ty)?;
        let mut bytes = Vec::new();
        if self.encode(current, &pty, &mut bytes).is_err() {
            return Err(self.place_mistyped(off, ty, &pty, op, "valid"));
        }
        let Some(part) = bytes.get(inner as usize..(inner + l.size) as usize) else {
            return Err(self.place_mistyped(off, ty, &pty, op, "valid"));
        };
        match self.decode(part, ty) {
            Ok(v) => Ok(Some(v)),
            Err(Bad::Invalid(bits)) => Err(panic(
                msg::PANIC_RAW_INVALID_BITS,
                &[
                    ("op", format!("ptr.{op}")),
                    ("bits", bits),
                    ("ty", shown(ty)),
                    ("obligation", "valid".into()),
                ],
            )),
            Err(Bad::Opaque) => Err(self.place_mistyped(off, ty, &pty, op, "valid")),
        }
    }

    fn place_mistyped(&self, off: u64, ty: &Type, held: &Type, op: &str, obligation: &str) -> Unwind {
        panic(
            msg::PANIC_RAW_MISTYPED,
            &[
                ("op", format!("ptr.{op}")),
                ("ty", shown(ty)),
                ("offset", off.to_string()),
                ("held", shown(held)),
                ("obligation", obligation.into()),
            ],
        )
    }

    /// Writes `v`, a `ty`, at `p` (or makes the memory there hold no value,
    /// for `None`): the write of `op`, whose obligation is `writable`.
    fn slot_write(&mut self, p: &RawPtr, ty: &Type, v: Option<Value>, op: &str) -> Result<(), Unwind> {
        let l = self.mem_layout(ty)?;
        let b = self.check_access(p, op, ty, l, "writable")?;
        if b == u32::MAX {
            return Ok(());
        }
        if p.shared {
            return Err(panic(
                msg::PANIC_RAW_SHARED_WRITE,
                &[("op", format!("ptr.{op}")), ("obligation", "writable".into())],
            ));
        }
        if matches!(self.raw.blocks[b as usize].kind, Kind::ReadOnly) {
            return Err(panic(msg::PANIC_RAW_READ_ONLY, &[("op", format!("ptr.{op}"))]));
        }
        let at = self.access_pointer(p, b, op, "writable")?;
        self.mon_access(&at, Op::Write)?;
        let off = p.off as u64;
        if let Kind::Place { .. } = self.raw.blocks[b as usize].kind {
            return self.place_write(b, off, ty, v, op);
        }
        self.clear(b, off, l.size, op, None)?;
        if let Some(v) = v
            && l.size > 0
        {
            self.raw.blocks[b as usize].items.insert(off, Item::Value(ty.clone(), v));
        }
        Ok(())
    }

    /// Makes `n` bytes from `off` hold nothing, keeping the bytes of plain
    /// values the range cuts: what a write of `op` does first. A value
    /// that needs a drop and lies in the range (outside `keep`, the range
    /// a copy moves its values from) is never dropped: the panic.
    fn clear(&mut self, b: u32, off: u64, n: u64, op: &str, keep: Option<(u64, u64)>) -> Result<(), Unwind> {
        let end = off + n;
        let block = &self.raw.blocks[b as usize];
        let first = block.items.range(..=off).next_back().map(|(k, _)| *k).unwrap_or(off);
        let mut hit: Vec<u64> = Vec::new();
        for (&at, item) in block.items.range(first..end) {
            let size = match item {
                Item::Byte(_) => 1,
                Item::Value(t, _) => self.mem_layout(t)?.size,
            };
            if at + size > off || (size == 0 && at >= off) {
                hit.push(at);
            }
        }
        for at in hit {
            let Some(item) = self.raw.blocks[b as usize].items.remove(&at) else { continue };
            let Item::Value(t, v) = item else { continue };
            let size = self.mem_layout(&t)?.size;
            let moved_from = keep.is_some_and(|(s, e)| at >= s && at + size <= e);
            if !v.is_moved() && self.type_needs_drop(&t) && !moved_from {
                return Err(panic(
                    msg::PANIC_RAW_OVERWRITE,
                    &[("op", format!("ptr.{op}")), ("ty", shown(&t)), ("offset", at.to_string())],
                ));
            }
            // The bytes of a plain value outside the range stay.
            if at < off || at + size > end {
                let mut bytes = Vec::new();
                if self.encode(&v, &t, &mut bytes).is_ok() {
                    for (k, x) in bytes.iter().enumerate() {
                        let a = at + k as u64;
                        if a < off || a >= end {
                            self.raw.blocks[b as usize].items.insert(a, Item::Byte(*x));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// A write through a pointer into a place of the program.
    fn place_write(&mut self, b: u32, off: u64, ty: &Type, v: Option<Value>, op: &str) -> Result<(), Unwind> {
        let (at, pty, inner) = self.place_of(b, off)?;
        if !self.place_alive(&at) {
            return Err(panic(
                msg::PANIC_RAW_GONE,
                &[("op", format!("ptr.{op}")), ("obligation", "writable".into())],
            ));
        }
        let old = self.read(&at)?.clone();
        if inner == 0 && (pty == *ty || same_pointer(&pty, ty)) {
            if !old.is_moved() && self.type_needs_drop(&pty) && v.is_some() {
                return Err(panic(
                    msg::PANIC_RAW_OVERWRITE,
                    &[("op", format!("ptr.{op}")), ("ty", shown(&pty)), ("offset", off.to_string())],
                ));
            }
            *self.write(&at)? = v.unwrap_or(Value::Moved);
            return Ok(());
        }
        // Another type: through the bytes of the value there.
        let Some(v) = v else {
            *self.write(&at)? = Value::Moved;
            return Ok(());
        };
        let l = self.mem_layout(ty)?;
        let mut bytes = Vec::new();
        let mut part = Vec::new();
        if self.encode(&old, &pty, &mut bytes).is_err() || self.encode(&v, ty, &mut part).is_err() {
            return Err(self.place_mistyped(off, ty, &pty, op, "writable"));
        }
        let Some(dst) = bytes.get_mut(inner as usize..(inner + l.size) as usize) else {
            return Err(self.place_mistyped(off, ty, &pty, op, "writable"));
        };
        dst.copy_from_slice(&part);
        match self.decode(&bytes, &pty) {
            Ok(nv) => {
                *self.write(&at)? = nv;
                Ok(())
            }
            Err(Bad::Invalid(bits)) => Err(panic(
                msg::PANIC_RAW_INVALID_BITS,
                &[
                    ("op", format!("ptr.{op}")),
                    ("bits", bits),
                    ("ty", shown(&pty)),
                    ("obligation", "writable".into()),
                ],
            )),
            Err(Bad::Opaque) => Err(self.place_mistyped(off, ty, &pty, op, "writable")),
        }
    }

    /// The bytes at `p`, `n` of them, as plain data: what a foreign function
    /// reads.
    fn read_bytes(&mut self, p: &RawPtr, n: u64, op: &str) -> Result<Vec<u8>, Unwind> {
        let u8t = Type::Named("u8".into(), Vec::new());
        let l = Layout { size: n, align: 1 };
        let b = self.check_access(p, op, &u8t, l, "valid")?;
        if b == u32::MAX || n == 0 {
            return Ok(Vec::new());
        }
        let at = self.access_pointer(p, b, op, "valid")?;
        self.mon_access(&at, Op::Read)?;
        if let Kind::Place { .. } = self.raw.blocks[b as usize].kind {
            let mut out = Vec::with_capacity(n as usize);
            for k in 0..n {
                let q = RawPtr { off: p.off + k as i64, ..p.clone() };
                match self.place_read(b, q.off as u64, &u8t, false, op)? {
                    Some(Value::Int(x)) => out.push(x as u8),
                    _ => return Err(self.uninitialised(b, q.off as u64, &u8t, op, "valid")),
                }
            }
            return Ok(out);
        }
        match self.block_bytes(b, p.off as u64, n, &u8t, op, "valid")? {
            Some(bytes) => Ok(bytes),
            None => Err(self.uninitialised(b, p.off as u64, &u8t, op, "valid")),
        }
    }

    /// Writes `bytes` at `p` as plain data: what a foreign function writes.
    fn write_bytes(&mut self, p: &RawPtr, bytes: &[u8], op: &str) -> Result<(), Unwind> {
        let u8t = Type::Named("u8".into(), Vec::new());
        for (k, x) in bytes.iter().enumerate() {
            let q = RawPtr { off: p.off + k as i64, ..p.clone() };
            self.slot_write(&q, &u8t, Some(Value::Int(i64::from(*x))), op)?;
        }
        Ok(())
    }

    /// The text of a C string at `p`: its bytes up to the first NUL.
    fn c_string(&mut self, p: &RawPtr, op: &str) -> Result<Vec<u8>, Unwind> {
        let mut out = Vec::new();
        loop {
            let q = RawPtr { off: p.off + out.len() as i64, ..p.clone() };
            let byte = self.read_bytes(&q, 1, op)?;
            match byte.first() {
                Some(0) | None => return Ok(out),
                Some(x) => out.push(*x),
            }
        }
    }

    // ------------------------------------------------------------ the intrinsics

    /// The intrinsics of std.mem, a foreign call (`extern.c.NAME`) and the
    /// machine intrinsics of std.rt: `None` for any other name. `arg_tys`
    /// are the static types of the arguments, `dest` the result's, where the
    /// call gives them (a call through a trait gives neither).
    pub(crate) fn raw_intrinsic(
        &mut self,
        name: &str,
        tys: &[Type],
        args: Vec<Value>,
        arg_tys: &[Option<Type>],
        dest: Option<&Type>,
    ) -> Option<Result<Value, Unwind>> {
        if let Some(f) = name.strip_prefix("extern.") {
            let f = f.split_once('.').map_or(f, |(_, n)| n);
            return Some(self.foreign_call(f, name, args, arg_tys, dest));
        }
        if name.starts_with("rt.") {
            return Some(Err(panic(msg::PANIC_MACHINE_INTRINSIC, &[("name", name.to_string())])));
        }
        if !name.starts_with("mem.") {
            return None;
        }
        Some(self.mem_intrinsic(name, tys, args, arg_tys, dest))
    }

    /// The type a `mem.ptr.*` method's pointer points to: from the static
    /// type of its pointer argument, else the call's first type argument.
    fn target_type(&self, tys: &[Type], arg_tys: &[Option<Type>], name: &str) -> Result<Type, Unwind> {
        pointee(arg_tys.first().and_then(Option::as_ref))
            .or_else(|| tys.first().cloned())
            .ok_or_else(|| ice(format!("`{name}` without the type of its pointer")))
    }

    fn mem_intrinsic(
        &mut self,
        name: &str,
        tys: &[Type],
        args: Vec<Value>,
        arg_tys: &[Option<Type>],
        dest: Option<&Type>,
    ) -> Result<Value, Unwind> {
        let arg = |i: usize| args.get(i).ok_or_else(|| ice(format!("`{name}` without argument {i}")));
        let targ = |i: usize| {
            tys.get(i).cloned().ok_or_else(|| {
                ice(format!(
                    "`{name}` without its type argument {i}: the front end must write it in the SIR call"
                ))
            })
        };
        let op = op_name(name);
        let op = op.strip_prefix("ptr.").unwrap_or(&op).to_string();
        match name {
            "mem.ptr.of" | "mem.ptr.of_mut" => {
                let Value::Ref(r) = arg(0)? else {
                    return Err(ice(format!("`{name}` of something that is no reference")));
                };
                let ty = match arg_tys.first().and_then(Option::as_ref) {
                    Some(Type::Ref(t, _)) => (**t).clone(),
                    Some(t) => t.clone(),
                    None => return Err(ice(format!("`{name}` without the type of its place"))),
                };
                let tag = r.tag;
                let mut at = (**r).clone();
                at.tag = 0;
                let (block, off) = self.pointer_to_place(at, ty)?;
                Ok(Self::ptr_value(RawPtr { block: Some(block), off, shared: name == "mem.ptr.of", tag }))
            }
            "mem.ptr.dangling" => {
                let t = targ(0)?;
                let align = self.mem_layout(&t)?.align;
                Ok(Self::ptr_value(RawPtr { block: None, off: align as i64, shared: false, tag: 0 }))
            }
            "mem.ptr.from_address" => {
                let a = self.int(arg(0)?)?;
                // Only memory foreign code gave out has an address a
                // program may turn back into a pointer (`exposed`).
                let found = self.raw.blocks.iter().position(|b| {
                    b.live
                        && matches!(b.kind, Kind::Malloc | Kind::ReadOnly)
                        && a >= b.base as i64
                        && a <= (b.base + b.size) as i64
                });
                Ok(Self::ptr_value(match found {
                    Some(i) => {
                        let base = self.raw.blocks[i].base as i64;
                        RawPtr { block: Some(i as u32), off: a - base, shared: false, tag: 0 }
                    }
                    None => RawPtr { block: None, off: a, shared: false, tag: 0 },
                }))
            }
            "mem.ptr.cast" | "mem.ptr.as_const" | "mem.ptr.cast_mut" => {
                Ok(Self::ptr_value(self.raw_ptr(arg(0)?)?))
            }
            "mem.ptr.with_addr" => {
                let mut p = self.raw_ptr(arg(0)?)?;
                let a = self.int(arg(1)?)?;
                p.off = match p.block.and_then(|b| self.raw.blocks.get(b as usize)) {
                    Some(b) => a - b.base as i64,
                    None => a,
                };
                Ok(Self::ptr_value(p))
            }
            "mem.ptr.addr" => {
                let p = self.raw_ptr(arg(0)?)?;
                Ok(Value::Int(self.address(&p)))
            }
            "mem.ptr.offset" => {
                let mut p = self.raw_ptr(arg(0)?)?;
                let n = self.int(arg(1)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                let size = self.mem_layout(&t)?.size as i64;
                let to = n.checked_mul(size).and_then(|d| p.off.checked_add(d));
                if let Some(b) = p.block.and_then(|b| self.raw.blocks.get(b as usize)) {
                    let inside = to.is_some_and(|t| t >= 0 && t <= b.size as i64);
                    if !inside {
                        return Err(panic(
                            msg::PANIC_RAW_OFFSET,
                            &[
                                ("count", n.to_string()),
                                ("offset", p.off.to_string()),
                                ("size", b.size.to_string()),
                            ],
                        ));
                    }
                }
                p.off = to.unwrap_or(p.off);
                Ok(Self::ptr_value(p))
            }
            "mem.ptr.read" | "mem.ptr.read_volatile" => {
                let p = self.raw_ptr(arg(0)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                let take = name == "mem.ptr.read";
                match self.slot_read(&p, &t, take, &op)? {
                    Some(v) => Ok(v),
                    None => Err(self.uninitialised(p.block.unwrap_or(0), p.off as u64, &t, &op, "valid")),
                }
            }
            "mem.ptr.write" | "mem.ptr.write_volatile" => {
                let p = self.raw_ptr(arg(0)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                let v = args.into_iter().nth(1).ok_or_else(|| ice("`ptr.write` without its value"))?;
                // A value given by reference (a `Copy` one) is written as
                // what it refers to.
                let v = match v {
                    Value::Ref(_) if !matches!(t, Type::Ref(..)) => self.target(&v)?.clone(),
                    v => v,
                };
                self.slot_write(&p, &t, Some(v), &op)?;
                Ok(Value::Unit)
            }
            "mem.ptr.drop_in_place" => {
                let p = self.raw_ptr(arg(0)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                if p.shared {
                    return Err(panic(
                        msg::PANIC_RAW_SHARED_WRITE,
                        &[("op", format!("ptr.{op}")), ("obligation", "valid".into())],
                    ));
                }
                let Some(v) = self.slot_read(&p, &t, true, &op)? else {
                    return Err(self.uninitialised(p.block.unwrap_or(0), p.off as u64, &t, &op, "valid"));
                };
                // The memory holds no value from here on.
                if !self.type_needs_drop(&t) {
                    self.slot_write(&p, &t, None, &op)?;
                }
                self.queue_drop(v, Some(t));
                Ok(Value::Unit)
            }
            "mem.ptr.copy_from" => {
                let dst = self.raw_ptr(arg(0)?)?;
                let src = self.raw_ptr(arg(1)?)?;
                let n = self.int(arg(2)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                self.copy_slots(&dst, &src, n, &t, &op)?;
                Ok(Value::Unit)
            }
            "mem.ptr.as_ref" | "mem.ptr.as_mut" => {
                let p = self.raw_ptr(arg(0)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                let mutable = name == "mem.ptr.as_mut";
                self.reference_to(&p, &t, mutable, &op)
            }
            "mem.slice.as_ptr" => {
                let v = arg(0)?;
                let elem = match arg_tys.first().and_then(Option::as_ref) {
                    Some(t) => {
                        let mut t = t;
                        while let Type::Ref(inner, _) = t {
                            t = inner;
                        }
                        match t {
                            Type::Named(_, a) => a.first().cloned(),
                            _ => None,
                        }
                    }
                    None => None,
                }
                .ok_or_else(|| ice("`as_ptr` without the type of its slice"))?;
                let Value::Ref(r) = v else { return Err(ice("`as_ptr` of a slice that is no reference")) };
                let tag = r.tag;
                let mut at = self.pointer_of(v)?;
                at.tag = 0;
                let n = match self.read(&at)? {
                    Value::List(xs) => xs.len() as u64,
                    other => return Err(ice(format!("`as_ptr` of {other:?}"))),
                };
                let block = self.place_block(at, elem, Some(n))?;
                Ok(Self::ptr_value(RawPtr { block: Some(block), off: 0, shared: true, tag }))
            }
            "mem.size_of" => Ok(Value::Int(self.mem_layout(&targ(0)?)?.size as i64)),
            "mem.align_of" => Ok(Value::Int(self.mem_layout(&targ(0)?)?.align as i64)),
            "mem.global.acquire" => {
                let (size, align) = self.layout_arg(arg(1)?)?;
                if size < 0 || align <= 0 || align & (align - 1) != 0 {
                    return Ok(Value::none());
                }
                self.serial += 1;
                let b = self.raw.add(size as u64, align as u64, Kind::Global, self.serial);
                Ok(Value::some(Self::ptr_value(RawPtr { block: Some(b), off: 0, shared: false, tag: 0 })))
            }
            "mem.global.release" => {
                let p = self.raw_ptr(arg(1)?)?;
                let (size, align) = self.layout_arg(arg(2)?)?;
                self.release(&p, Some((size, align)), "Global.release", "Global.acquire")?;
                Ok(Value::Unit)
            }
            "mem.bit_cast" | "mem.transmute" => {
                let (to, from) = (targ(0)?, targ(1)?);
                let x = self.target(arg(0)?)?.clone();
                self.reinterpret(
                    &x,
                    &from,
                    &to,
                    &op,
                    if name == "mem.bit_cast" { "valid" } else { "valid_bits" },
                )
            }
            "mem.as_bytes" => {
                let t = arg_tys
                    .first()
                    .and_then(Option::as_ref)
                    .map(|t| match t {
                        Type::Ref(inner, _) => (**inner).clone(),
                        t => t.clone(),
                    })
                    .ok_or_else(|| ice("`as_bytes` without the type of its value"))?;
                let x = self.target(arg(0)?)?.clone();
                let mut bytes = Vec::new();
                if self.encode(&x, &t, &mut bytes).is_err() {
                    return Err(ice(format!("`as_bytes` of a `{}`, which is not plain", shown(&t))));
                }
                Ok(Value::Bytes(bytes.into()))
            }
            "mem.read_plain" => {
                let t = targ(0)?;
                let data = match self.target(arg(0)?)? {
                    Value::Bytes(b) => b.clone(),
                    other => return Err(ice(format!("`read_plain` of {other:?}"))),
                };
                let at = self.int(arg(1)?)?;
                let size = self.mem_layout(&t)?.size as i64;
                let part = (at >= 0).then(|| data.get(at as usize..(at + size) as usize)).flatten();
                let Some(part) = part else {
                    return Err(panic(
                        msg::PANIC_RAW_PLAIN_SHORT,
                        &[
                            ("ty", shown(&t)),
                            ("size", size.to_string()),
                            ("at", at.to_string()),
                            ("len", data.len().to_string()),
                        ],
                    ));
                };
                self.decode(part, &t).map_err(|_| ice(format!("`read_plain` of a `{}`", shown(&t))))
            }
            "mem.view_plain" => {
                let t = targ(0)?;
                let r = arg(0)?.clone();
                let tag = match &r {
                    Value::Ref(p) => p.tag,
                    _ => 0,
                };
                let data = match self.target(&r)? {
                    Value::Bytes(b) => b.clone(),
                    other => return Err(ice(format!("`view_plain` of {other:?}"))),
                };
                let l = self.mem_layout(&t)?;
                // A `bytes`' buffer is aligned for any plain value, as the
                // C allocator aligns it natively.
                let Some(part) = data.get(..l.size as usize) else { return Ok(Value::none()) };
                let v =
                    self.decode(part, &t).map_err(|_| ice(format!("`view_plain` of a `{}`", shown(&t))))?;
                self.serial += 1;
                let b = self.raw.add(l.size, l.align, Kind::ReadOnly, self.serial);
                let serial = self.raw.blocks[b as usize].serial;
                self.raw.blocks[b as usize].items.insert(0, Item::Value(t, v));
                let at = Pointer { task: 0, frame: RAW, serial, local: b, path: vec![Step::Raw(0)], tag };
                Ok(Value::some(Value::Ref(std::rc::Rc::new(at))))
            }
            "mem.slice_from_raw" => {
                let p = self.raw_ptr(arg(0)?)?;
                let n = self.int(arg(1)?)?;
                let t = self.target_type(tys, arg_tys, name)?;
                let size = self.mem_layout(&t)?.size as i64;
                let mut items = Vec::new();
                for k in 0..n.max(0) {
                    let q = RawPtr { off: p.off + k * size, ..p.clone() };
                    match self.slot_read(&q, &t, false, "slice_from_raw")? {
                        Some(v) => {
                            let c = self.copy_value(&v);
                            items.push(c);
                        }
                        None => {
                            return Err(self.uninitialised(
                                q.block.unwrap_or(0),
                                q.off as u64,
                                &t,
                                "slice_from_raw",
                                "valid",
                            ));
                        }
                    }
                }
                Ok(Value::List(items))
            }
            _ => {
                let _ = dest;
                Err(ice(format!("unknown intrinsic `{name}`")))
            }
        }
    }

    /// The pointer to a place: into the raw block a reference into raw
    /// memory leads to, else into a block made for the place.
    fn pointer_to_place(&mut self, at: Pointer, ty: Type) -> Result<(u32, i64), Unwind> {
        if at.frame == RAW
            && let Some(Step::Raw(off)) = at.path.first()
            && at.path.len() == 1
        {
            return Ok((at.local, *off as i64));
        }
        Ok((self.place_block(at, ty, None)?, 0))
    }

    /// A `Layout` argument's size and alignment.
    fn layout_arg(&self, v: &Value) -> Result<(i64, i64), Unwind> {
        match self.target(v)? {
            Value::Struct(a) => match (a.fields.first(), a.fields.get(1)) {
                (Some(Value::Int(s)), Some(Value::Int(al))) => Ok((*s, *al)),
                _ => Err(ice("a `Layout` without its fields")),
            },
            other => Err(ice(format!("a layout that is {other:?}"))),
        }
    }

    /// Takes back a block: `Global.release` (with the layout it was given)
    /// or `free`.
    fn release(
        &mut self,
        p: &RawPtr,
        layout: Option<(i64, i64)>,
        op: &str,
        allocator: &str,
    ) -> Result<(), Unwind> {
        let want = if layout.is_some() { Kind::Global } else { Kind::Malloc };
        let other = || {
            panic(
                msg::PANIC_RAW_RELEASE_OTHER,
                &[("op", op.to_string()), ("allocator", allocator.to_string())],
            )
        };
        let Some(b) = p.block else { return Err(other()) };
        let Some(block) = self.raw.blocks.get(b as usize) else { return Err(other()) };
        if p.off != 0 || std::mem::discriminant(&block.kind) != std::mem::discriminant(&want) {
            return Err(other());
        }
        if !block.live {
            return Err(panic(msg::PANIC_RAW_RELEASE_TWICE, &[("op", op.to_string())]));
        }
        if let Some((size, align)) = layout
            && (size as u64 != block.size || align as u64 != block.align)
        {
            return Err(panic(
                msg::PANIC_RAW_RELEASE_LAYOUT,
                &[
                    ("size", size.to_string()),
                    ("align", align.to_string()),
                    ("block_size", block.size.to_string()),
                    ("block_align", block.align.to_string()),
                ],
            ));
        }
        // A value left in the block that needs a drop would be lost; while
        // a panic unwinds, a half-done operation may leave one, which is
        // then forgotten rather than turning the panic into an abort.
        let unwinding = self.frames.iter().any(|f| f.pending.is_some());
        let left = block.items.iter().filter(|_| !unwinding).find_map(|(at, item)| match item {
            Item::Value(t, v) if !v.is_moved() && self.type_needs_drop(t) => Some((*at, t.clone())),
            _ => None,
        });
        if let Some((at, t)) = left {
            return Err(panic(
                msg::PANIC_RAW_RELEASE_LIVE,
                &[("op", op.to_string()), ("ty", shown(&t)), ("offset", at.to_string())],
            ));
        }
        let serial = block.serial;
        let block = &mut self.raw.blocks[b as usize];
        block.live = false;
        block.items.clear();
        self.mon_raw_released(serial, b);
        Ok(())
    }

    /// `p.copy_from(q, n)`: `n` slots of `t` from `q` to `p`, each holding
    /// what the one it was copied from held, as through a buffer; a value
    /// that needs a drop is the copy's alone afterwards.
    fn copy_slots(&mut self, dst: &RawPtr, src: &RawPtr, n: i64, t: &Type, op: &str) -> Result<(), Unwind> {
        let l = self.mem_layout(t)?;
        let bytes = l.size * n.max(0) as u64;
        let whole = Layout { size: bytes, align: l.align };
        let sb = self.check_access(src, op, t, whole, "readable")?;
        let db = self.check_access(dst, op, t, whole, "writable")?;
        if dst.shared {
            return Err(panic(
                msg::PANIC_RAW_SHARED_WRITE,
                &[("op", format!("ptr.{op}")), ("obligation", "writable".into())],
            ));
        }
        if n <= 0 || bytes == 0 {
            return Ok(());
        }
        let take = self.type_needs_drop(t);
        let mut slots = Vec::with_capacity(n as usize);
        for k in 0..n {
            let q = RawPtr { off: src.off + k * l.size as i64, ..src.clone() };
            slots.push(self.slot_read(&q, t, take, op)?);
        }
        // A copy within one block moves the values it overwrites itself.
        let keep = (sb == db).then_some((src.off as u64, src.off as u64 + bytes));
        if matches!(self.raw.blocks.get(db as usize).map(|b| &b.kind), Some(Kind::Global | Kind::Malloc)) {
            self.clear(db, dst.off as u64, bytes, op, keep)?;
        }
        for (k, v) in slots.into_iter().enumerate() {
            let q = RawPtr { off: dst.off + k as i64 * l.size as i64, ..dst.clone() };
            match v {
                Some(v) => self.slot_write(&q, t, Some(v), op)?,
                None => self.slot_write(&q, t, None, op)?,
            }
        }
        Ok(())
    }

    /// A reference to the `t` at `p` (`as_ref`, `as_mut`): into the place
    /// a pointer to a place points to, else to the value the block holds
    /// there, made from its bytes if it holds them as bytes. Under the
    /// monitor it is a loan under the pointer's.
    fn reference_to(&mut self, p: &RawPtr, t: &Type, mutable: bool, op: &str) -> Result<Value, Unwind> {
        let l = self.mem_layout(t)?;
        let b = self.check_access(p, op, t, l, "valid")?;
        if b == u32::MAX {
            return Err(panic(
                msg::PANIC_RAW_NO_MEMORY,
                &[("op", format!("ptr.{op}")), ("obligation", "valid".into())],
            ));
        }
        if mutable && p.shared {
            return Err(panic(
                msg::PANIC_RAW_SHARED_WRITE,
                &[("op", format!("ptr.{op}")), ("obligation", "valid_for_origin".into())],
            ));
        }
        let off = p.off as u64;
        let at = if let Kind::Place { .. } = self.raw.blocks[b as usize].kind {
            let (at, pty, inner) = self.place_of(b, off)?;
            if !self.place_alive(&at) {
                return Err(panic(
                    msg::PANIC_RAW_GONE,
                    &[("op", format!("ptr.{op}")), ("obligation", "valid".into())],
                ));
            }
            if inner != 0 || !(pty == *t || same_pointer(&pty, t)) {
                return Err(self.place_mistyped(off, t, &pty, op, "valid"));
            }
            if self.read(&at)?.is_moved() {
                return Err(self.uninitialised(b, off, t, op, "valid"));
            }
            at
        } else {
            let holds = matches!(
                self.raw.blocks[b as usize].items.get(&off),
                Some(Item::Value(it, _)) if it == t || same_pointer(it, t)
            );
            if !holds {
                // Made from the bytes there: the value, written back whole.
                let Some(v) = self.slot_read(p, t, false, op)? else {
                    return Err(self.uninitialised(b, off, t, op, "valid"));
                };
                if !matches!(self.raw.blocks[b as usize].kind, Kind::ReadOnly) {
                    self.clear(b, off, l.size, op, None)?;
                }
                self.raw.blocks[b as usize].items.insert(off, Item::Value(t.clone(), v));
            }
            let serial = self.raw.blocks[b as usize].serial;
            Pointer { task: 0, frame: RAW, serial, local: b, path: vec![Step::Raw(off)], tag: 0 }
        };
        let mut at = at;
        at.tag = p.tag;
        if self.monitoring() {
            at.tag = self.mon_borrow(&at, mutable, true)?;
        } else {
            at.tag = 0;
        }
        Ok(Value::Ref(std::rc::Rc::new(at)))
    }

    /// `bit_cast` and `transmute`: the bits of `x`, a `from`, as a `to` of
    /// the same size.
    fn reinterpret(
        &self,
        x: &Value,
        from: &Type,
        to: &Type,
        op: &str,
        obligation: &str,
    ) -> Result<Value, Unwind> {
        let (lf, lt) = (self.mem_layout(from)?, self.mem_layout(to)?);
        if lf.size != lt.size {
            return Err(panic(
                msg::PANIC_RAW_SIZES,
                &[
                    ("op", op.to_string()),
                    ("from", shown(from)),
                    ("from_size", lf.size.to_string()),
                    ("to", shown(to)),
                    ("to_size", lt.size.to_string()),
                ],
            ));
        }
        if from == to || (pointee(Some(from)).is_some() && pointee(Some(to)).is_some()) {
            return Ok(x.clone());
        }
        let mut bytes = Vec::new();
        if self.encode(x, from, &mut bytes).is_err() {
            return Err(self.place_mistyped(0, to, from, op, obligation));
        }
        match self.decode(&bytes, to) {
            Ok(v) => Ok(v),
            Err(Bad::Invalid(bits)) => Err(panic(
                msg::PANIC_RAW_INVALID_BITS,
                &[
                    ("op", op.to_string()),
                    ("bits", bits),
                    ("ty", shown(to)),
                    ("obligation", obligation.into()),
                ],
            )),
            Err(Bad::Opaque) => Err(self.place_mistyped(0, to, from, op, obligation)),
        }
    }

    // ------------------------------------------------------------ foreign calls

    /// A foreign call (`extern.c.NAME`): one of the C functions the
    /// interpreter emulates (Q-55), on the checked model; any other stops
    /// the run.
    fn foreign_call(
        &mut self,
        f: &str,
        name: &str,
        args: Vec<Value>,
        arg_tys: &[Option<Type>],
        dest: Option<&Type>,
    ) -> Result<Value, Unwind> {
        let arg = |i: usize| args.get(i).ok_or_else(|| ice(format!("`{name}` without argument {i}")));
        let int_result = |n: i128| -> Result<Value, Unwind> { int_as(dest, n) };
        let _ = arg_tys;
        match f {
            "strlen" => {
                let p = self.nullable(arg(0)?)?.ok_or_else(|| {
                    panic(
                        msg::PANIC_RAW_NO_MEMORY,
                        &[("op", "strlen".into()), ("obligation", "valid".into())],
                    )
                })?;
                let n = self.c_string(&p, "strlen")?.len();
                int_result(n as i128)
            }
            "memcpy" => {
                let dst = self.nullable(arg(0)?)?;
                let src = self.nullable(arg(1)?)?;
                let n = self.number(arg(2)?)?;
                let (Some(dst), Some(src)) = (dst, src) else {
                    return Err(panic(
                        msg::PANIC_RAW_NO_MEMORY,
                        &[("op", "memcpy".into()), ("obligation", "valid".into())],
                    ));
                };
                if dst.block.is_some()
                    && dst.block == src.block
                    && (dst.off - src.off).unsigned_abs() < n as u64
                    && n > 0
                {
                    return Err(panic(msg::PANIC_FOREIGN_OVERLAP, &[("bytes", n.to_string())]));
                }
                let bytes = self.read_bytes(&src, n as u64, "memcpy")?;
                self.write_bytes(&dst, &bytes, "memcpy")?;
                Ok(self.pointer_result(Some(dst), dest, name)?)
            }
            "malloc" => {
                let n = self.number(arg(0)?)?;
                self.serial += 1;
                let b = self.raw.add(n.max(0) as u64, 16, Kind::Malloc, self.serial);
                let p = RawPtr { block: Some(b), off: 0, shared: false, tag: 0 };
                self.pointer_result(Some(p), dest, name)
            }
            "free" => {
                if let Some(p) = self.nullable(arg(0)?)? {
                    self.release(&p, None, "free", "malloc")?;
                }
                Ok(Value::Unit)
            }
            "getenv" => {
                let p = self.nullable(arg(0)?)?.ok_or_else(|| {
                    panic(
                        msg::PANIC_RAW_NO_MEMORY,
                        &[("op", "getenv".into()), ("obligation", "valid".into())],
                    )
                })?;
                let key = self.c_string(&p, "getenv")?;
                let value = self.host.env_var(&String::from_utf8_lossy(&key));
                let Some(value) = value else { return self.pointer_result(None, dest, name) };
                let mut bytes = value.into_bytes();
                bytes.push(0);
                self.serial += 1;
                let b = self.raw.add(bytes.len() as u64, 16, Kind::ReadOnly, self.serial);
                for (k, x) in bytes.iter().enumerate() {
                    self.raw.blocks[b as usize].items.insert(k as u64, Item::Byte(*x));
                }
                self.pointer_result(Some(RawPtr { block: Some(b), off: 0, shared: true, tag: 0 }), dest, name)
            }
            "write" => {
                let fd = self.number(arg(0)?)?;
                let p = self.nullable(arg(1)?)?;
                let n = self.number(arg(2)?)?;
                if fd != 1 && fd != 2 {
                    return Err(panic(msg::PANIC_FOREIGN_FD, &[("fd", fd.to_string())]));
                }
                let bytes = match p {
                    Some(p) => self.read_bytes(&p, n.max(0) as u64, "write")?,
                    None if n == 0 => Vec::new(),
                    None => {
                        return Err(panic(
                            msg::PANIC_RAW_NO_MEMORY,
                            &[("op", "write".into()), ("obligation", "valid".into())],
                        ));
                    }
                };
                let text = String::from_utf8_lossy(&bytes);
                if fd == 1 {
                    self.host.stdout(&text);
                } else {
                    self.host.stderr(&text);
                }
                int_result(bytes.len() as i128)
            }
            _ => {
                Err(panic(msg::PANIC_FOREIGN_UNKNOWN, &[("name", f.to_string()), ("known", EMULATED.into())]))
            }
        }
    }

    /// An integer argument of any integer type.
    fn number(&self, v: &Value) -> Result<i128, Unwind> {
        match self.target(v)? {
            Value::Int(i) => Ok(i128::from(*i)),
            Value::U64(u) => Ok(i128::from(*u)),
            Value::I128(i) => Ok(*i),
            Value::U128(u) => i128::try_from(*u).map_err(|_| ice("an argument too large for C")),
            other => Err(ice(format!("expected an integer, found {other:?}"))),
        }
    }

    /// What a foreign function that returns a pointer gives, as its
    /// declared result type holds it: an option of one, or the pointer,
    /// which may not be null.
    fn pointer_result(&self, p: Option<RawPtr>, dest: Option<&Type>, name: &str) -> Result<Value, Unwind> {
        let nullable = matches!(dest, Some(Type::Named(n, _)) if n == "Option");
        match (p, nullable) {
            (Some(p), true) => Ok(Value::some(Self::ptr_value(p))),
            (None, true) => Ok(Value::none()),
            (Some(p), false) => Ok(Self::ptr_value(p)),
            (None, false) => Err(panic(
                msg::PANIC_FOREIGN_NULL,
                &[
                    ("name", name.strip_prefix("extern.c.").unwrap_or(name).to_string()),
                    ("ty", dest.map(shown).unwrap_or_else(|| "ptr".into())),
                ],
            )),
        }
    }
}

/// Whether two types are raw pointers, whatever they point to: memory a
/// pointer of one was written into reads as the other.
fn same_pointer(a: &Type, b: &Type) -> bool {
    pointee(Some(a)).is_some() && pointee(Some(b)).is_some()
}

/// The integer `n` as a value of the type `t` (an `int` without one).
fn int_as(t: Option<&Type>, n: i128) -> Result<Value, Unwind> {
    Ok(match t {
        Some(Type::Named(name, _)) => match name.as_str() {
            "u64" => Value::U64(n as u64),
            "u128" => Value::U128(n as u128),
            "i128" => Value::I128(n),
            _ => Value::Int(n as i64),
        },
        _ => Value::Int(n as i64),
    })
}

fn encode_scalar(v: &Value, s: Scalar, out: &mut Vec<u8>) -> Result<(), Bad> {
    match (s, v) {
        (Scalar::Unit, _) => {}
        (Scalar::Int, Value::Int(i)) => out.extend_from_slice(&i.to_le_bytes()),
        (Scalar::DateTime, Value::DateTime(i)) => out.extend_from_slice(&i.to_le_bytes()),
        (Scalar::U64, Value::U64(u)) => out.extend_from_slice(&u.to_le_bytes()),
        (Scalar::Sized { bytes, .. }, Value::Int(i)) => {
            out.extend_from_slice(&i.to_le_bytes()[..bytes as usize])
        }
        (Scalar::I128, Value::I128(i)) => out.extend_from_slice(&i.to_le_bytes()),
        (Scalar::U128, Value::U128(u)) => out.extend_from_slice(&u.to_le_bytes()),
        (Scalar::F64, Value::Float(f)) => out.extend_from_slice(&f.to_bits().to_le_bytes()),
        (Scalar::F32, Value::F32(f)) => out.extend_from_slice(&f.to_bits().to_le_bytes()),
        (Scalar::Bool, Value::Bool(b)) => out.extend_from_slice(&u64::from(*b).to_le_bytes()),
        (Scalar::Char, Value::Char(c)) => out.extend_from_slice(&u64::from(u32::from(*c)).to_le_bytes()),
        _ => return Err(Bad::Opaque),
    }
    Ok(())
}

fn decode_scalar(b: &[u8], s: Scalar) -> Result<Value, Bad> {
    let take = |n: usize| -> Result<&[u8], Bad> { b.get(..n).ok_or(Bad::Opaque) };
    let word = || -> Result<[u8; 8], Bad> { <[u8; 8]>::try_from(take(8)?).map_err(|_| Bad::Opaque) };
    let wide = || -> Result<[u8; 16], Bad> { <[u8; 16]>::try_from(take(16)?).map_err(|_| Bad::Opaque) };
    Ok(match s {
        Scalar::Unit => Value::Unit,
        Scalar::Int => Value::Int(i64::from_le_bytes(word()?)),
        Scalar::DateTime => Value::DateTime(i64::from_le_bytes(word()?)),
        Scalar::U64 => Value::U64(u64::from_le_bytes(word()?)),
        Scalar::Sized { bytes, signed } => {
            let mut w = [0u8; 8];
            w[..bytes as usize].copy_from_slice(take(bytes as usize)?);
            let mut i = i64::from_le_bytes(w);
            if signed {
                let shift = 64 - 8 * u32::from(bytes);
                i = (i << shift) >> shift;
            }
            Value::Int(i)
        }
        Scalar::I128 => Value::I128(i128::from_le_bytes(wide()?)),
        Scalar::U128 => Value::U128(u128::from_le_bytes(wide()?)),
        Scalar::F64 => Value::Float(f64::from_bits(u64::from_le_bytes(word()?))),
        Scalar::F32 => {
            let w = <[u8; 4]>::try_from(take(4)?).map_err(|_| Bad::Opaque)?;
            Value::F32(f32::from_bits(u32::from_le_bytes(w)))
        }
        Scalar::Bool => match u64::from_le_bytes(word()?) {
            0 => Value::Bool(false),
            1 => Value::Bool(true),
            other => return Err(Bad::Invalid(format!("{other:#x}"))),
        },
        Scalar::Char => {
            let w = u64::from_le_bytes(word()?);
            match u32::try_from(w).ok().and_then(char::from_u32) {
                Some(c) => Value::Char(c),
                None => return Err(Bad::Invalid(format!("{w:#x}"))),
            }
        }
    })
}

/// Whether the intrinsic is raw memory's, a foreign call or a machine
/// intrinsic (`Interp::raw_intrinsic`).
pub(crate) fn is_raw(name: &str) -> bool {
    name.starts_with("mem.") || name.starts_with("extern.") || name.starts_with("rt.")
}
