//! The type table as the runtime in Sisu reads it (`compiler/rt`, decision
//! 350): the program's types as words in memory, so that Sisu code over raw
//! memory walks a value by its type's parts, as `value.rs` walked the
//! decoded table. It is made once when the program starts, from the decoded
//! table and what `rt.rs` works out of it: which types need a drop, which
//! are copied as their bytes, and the parts a drop and a copy reach in
//! place (decisions 262 and 266).
//!
//! Type `t` has a header of [`WORDS`] words at word `t * WORDS`: its kind
//! (the number `abi` encodes it by), its size in bytes, its flags, two
//! words `a` and `b` (what it holds; a result's two types; a dict's key and
//! value; a struct's or enum's drop glue, plus one), the number of its name
//! (equal names, equal numbers), the positions of its lists, and the offset
//! of its tag. The lists follow the headers, each its length and then its
//! items: a struct's fields and a tuple's elements as offset, type and
//! whether the glue drops it; an enum's variants, each the position of its
//! fields' list; an error set's member types; the parts a drop reaches and
//! those a copy reaches, as offset and type, of the value or of each
//! variant. A position of 0 is no list.

use crate::abi::{Kind, TypeEntry};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The words of a type's header.
pub(crate) const WORDS: usize = 10;

const KIND: usize = 0;
const SIZE: usize = 1;
const FLAGS: usize = 2;
const A: usize = 3;
const B: usize = 4;
const NAME: usize = 5;
const PARTS: usize = 6;
const DROPS: usize = 7;
const COPIES: usize = 8;
const TAG: usize = 9;

/// The flags: dropping a value of the type can do something; a copy is a
/// copy of its bytes; an error value (an `error` enum or a union of error
/// sets), whose first word is its own type's number and second its trace;
/// a capability handle whose copies are counted (decision 237); the drop's
/// and the copy's parts are per variant.
const NEEDS_DROP: u64 = 1;
const PLAIN: u64 = 2;
const ERROR: u64 = 4;
const COUNTED: u64 = 8;
const DROPS_BY_VARIANT: u64 = 16;
const COPIES_BY_VARIANT: u64 = 32;

/// The parts a drop or a copy reaches in place (`rt::DropParts`,
/// `rt::CopyParts`): none when the kind's own rule applies, those of the
/// value, or those of each variant.
pub(crate) enum Parts<'a> {
    ByKind,
    Of(&'a [(u32, u32)]),
    ByVariant(&'a [Vec<(u32, u32)>]),
}

impl<'a> From<&'a crate::rt::DropParts> for Parts<'a> {
    fn from(p: &'a crate::rt::DropParts) -> Parts<'a> {
        match p {
            crate::rt::DropParts::ByKind => Parts::ByKind,
            crate::rt::DropParts::Parts(v) => Parts::Of(v),
            crate::rt::DropParts::Variants(v) => Parts::ByVariant(v),
        }
    }
}

impl<'a> From<&'a crate::rt::CopyParts> for Parts<'a> {
    fn from(p: &'a crate::rt::CopyParts) -> Parts<'a> {
        match p {
            crate::rt::CopyParts::ByKind => Parts::ByKind,
            crate::rt::CopyParts::Parts(v) => Parts::Of(v),
            crate::rt::CopyParts::Variants(v) => Parts::ByVariant(v),
        }
    }
}

/// The words, kept for the whole run.
static TABLE: OnceLock<Box<[u64]>> = OnceLock::new();

/// The address of the words, which the runtime in Sisu is given.
pub(crate) fn base() -> *const u64 {
    TABLE.get().map_or(std::ptr::null(), |t| t.as_ptr())
}

/// The words of a part list: its length, then each part's offset and type.
fn part_list(out: &mut Vec<u64>, parts: &[(u32, u32)]) -> u64 {
    let at = out.len() as u64;
    out.push(parts.len() as u64);
    for &(off, t) in parts {
        out.push(u64::from(off));
        out.push(u64::from(t));
    }
    at
}

/// The words of a field list: its length, then each field's offset, type
/// and whether the glue drops it.
fn field_list(out: &mut Vec<u64>, fields: impl ExactSizeIterator<Item = (u32, u32, bool)>) -> u64 {
    let at = out.len() as u64;
    out.push(fields.len() as u64);
    for (off, t, glue) in fields {
        out.push(u64::from(off));
        out.push(u64::from(t));
        out.push(u64::from(glue));
    }
    at
}

/// The lists of a drop's or a copy's parts, by variant or of the value, and
/// the flag that says which.
fn parts_words(out: &mut Vec<u64>, parts: Parts<'_>, by_variant: u64) -> (u64, u64) {
    match parts {
        Parts::ByKind => (0, 0),
        Parts::Of(p) => (part_list(out, p), 0),
        Parts::ByVariant(vs) => {
            let lists: Vec<u64> = vs.iter().map(|p| part_list(out, p)).collect();
            let at = out.len() as u64;
            out.push(lists.len() as u64);
            out.extend(lists);
            (at, by_variant)
        }
    }
}

/// Makes the words of the table once: `drops` and `copies` give each type's
/// parts, `needs_drop` and `plain` its flags.
pub(crate) fn set_up(
    types: &[TypeEntry],
    needs_drop: &[bool],
    plain: &[bool],
    drops: &[crate::rt::DropParts],
    copies: &[crate::rt::CopyParts],
) {
    let mut out = vec![0u64; types.len() * WORDS];
    let mut names: HashMap<&str, u64> = HashMap::new();
    for (i, t) in types.iter().enumerate() {
        let mut h = [0u64; WORDS];
        h[KIND] = u64::from(t.kind.code());
        h[SIZE] = u64::from(t.size);
        let mut flags = 0;
        if needs_drop.get(i).copied().unwrap_or(false) {
            flags |= NEEDS_DROP;
        }
        if plain.get(i).copied().unwrap_or(false) {
            flags |= PLAIN;
        }
        h[TAG] = u64::from(t.kind.tag_offset());
        let next = names.len() as u64 + 1;
        match &t.kind {
            Kind::Ref(x)
            | Kind::List(x)
            | Kind::Iter(x)
            | Kind::Option(x)
            | Kind::Box(x)
            | Kind::Set(x)
            | Kind::Handle(x)
            | Kind::Sender(x)
            | Kind::Receiver(x)
            | Kind::Mutex(x)
            | Kind::RwLock(x)
            | Kind::ReadGuard(x)
            | Kind::Shared(x)
            | Kind::Weak(x)
            | Kind::Cell(x)
            | Kind::Arena(x)
            | Kind::Ptr(x) => h[A] = u64::from(*x),
            Kind::Guard(x, check) => {
                h[A] = u64::from(*x);
                h[B] = check.map_or(0, |c| u64::from(c) + 1);
            }
            Kind::Result(x, y) | Kind::Dict(x, y) | Kind::Entry(x, y) => {
                h[A] = u64::from(*x);
                h[B] = u64::from(*y);
            }
            Kind::Const(n) => h[A] = *n as u64,
            Kind::Tuple(slots) => {
                h[PARTS] = field_list(&mut out, slots.iter().map(|s| (s.offset, s.ty, false)));
            }
            Kind::Struct { name, fields, glue } => {
                h[B] = glue.map_or(0, |g| u64::from(g) + 1);
                h[NAME] = *names.entry(name.as_str()).or_insert(next);
                h[PARTS] = field_list(&mut out, fields.iter().map(|f| (f.offset, f.ty, f.glue)));
            }
            Kind::Enum { name, variants, glue, error } => {
                h[B] = glue.map_or(0, |g| u64::from(g) + 1);
                h[NAME] = *names.entry(name.as_str()).or_insert(next);
                if *error {
                    flags |= ERROR;
                }
                let lists: Vec<u64> = variants
                    .iter()
                    .map(|v| field_list(&mut out, v.fields.iter().map(|f| (f.offset, f.ty, f.glue))))
                    .collect();
                h[PARTS] = out.len() as u64;
                out.push(lists.len() as u64);
                out.extend(lists);
            }
            Kind::ErrSet(members) => {
                flags |= ERROR;
                h[PARTS] = out.len() as u64;
                out.push(members.len() as u64);
                out.extend(members.iter().map(|m| u64::from(*m)));
            }
            Kind::Opaque(n) if crate::value::counted(n) => flags |= COUNTED,
            _ => {}
        }
        let (d, df) =
            parts_words(&mut out, drops.get(i).map_or(Parts::ByKind, Parts::from), DROPS_BY_VARIANT);
        let (c, cf) =
            parts_words(&mut out, copies.get(i).map_or(Parts::ByKind, Parts::from), COPIES_BY_VARIANT);
        h[DROPS] = d;
        h[COPIES] = c;
        h[FLAGS] = flags | df | cf;
        out[i * WORDS..(i + 1) * WORDS].copy_from_slice(&h);
    }
    let _ = TABLE.set(out.into_boxed_slice());
}
