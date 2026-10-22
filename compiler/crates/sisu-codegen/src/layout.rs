//! Layouts of concrete SIR types and the type table the runtime reads
//! (`sisu_rt::abi`): every type is a whole number of words, a struct or a
//! tuple its parts in order, an enum a tag word and its variants' payloads
//! over each other, an error set three words (its type's number, its
//! return trace, its tag) and the payload.

use crate::Error;
use sisu_rt::abi::{self, Field, Kind, Slot, TypeEntry, Variant};
use sisu_sir as sir;
use sisu_sir::Type;
use std::collections::HashMap;

/// The interned types of a program and their entries.
pub(crate) struct Types<'p> {
    prog: &'p sir::Program,
    decls: HashMap<&'p str, &'p sir::TypeDecl>,
    ids: HashMap<Type, u32>,
    /// `None` while a type's entry is being made.
    entries: Vec<Option<TypeEntry>>,
    /// The type each number stands for.
    by_id: Vec<Type>,
    sizes: HashMap<Type, u32>,
    /// The types whose sizes are being computed.
    sizing: Vec<Type>,
    /// The drop glue instances the entries name, by their index in the
    /// function table: (glue function, type arguments).
    pub glue: Vec<(String, Vec<Type>)>,
    glue_index: HashMap<(String, Vec<Type>), u32>,
}

impl<'p> Types<'p> {
    pub(crate) fn new(prog: &'p sir::Program) -> Types<'p> {
        let decls = prog.types.iter().map(|t| (t.name(), t)).collect();
        Types {
            prog,
            decls,
            ids: HashMap::new(),
            entries: Vec::new(),
            by_id: Vec::new(),
            sizes: HashMap::new(),
            sizing: Vec::new(),
            glue: Vec::new(),
            glue_index: HashMap::new(),
        }
    }

    pub(crate) fn decl(&self, name: &str) -> Option<&'p sir::TypeDecl> {
        self.decls.get(name).copied()
    }

    /// The finished table, in the order of the types' numbers.
    pub(crate) fn table(&self) -> Result<Vec<TypeEntry>, Error> {
        self.entries
            .iter()
            .map(|e| e.clone().ok_or_else(|| Error::internal("a type's entry was never finished")))
            .collect()
    }

    /// The bytes of a value of the type. Computed from the type alone, so
    /// that a type may hold itself through a list, a box or a reference
    /// while its entry is being made.
    pub(crate) fn size(&mut self, t: &Type) -> Result<u32, Error> {
        if let Some(&s) = self.sizes.get(t) {
            return Ok(s);
        }
        if self.sizing.contains(t) {
            return Err(Error::unsupported(format!(
                "the type `{}`, which holds itself without a `Box`",
                sir::write::ty(t)
            )));
        }
        self.sizing.push(t.clone());
        let r = self.size_of(t);
        self.sizing.pop();
        let s = r?;
        self.sizes.insert(t.clone(), s);
        Ok(s)
    }

    fn size_of(&mut self, t: &Type) -> Result<u32, Error> {
        let w = abi::WORD;
        Ok(match t {
            Type::Ref(inner, _) if matches!(**inner, Type::Dyn(..)) => 2 * w,
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Ref(..) => w,
            Type::Str | Type::Fn(..) | Type::Dyn(..) => 2 * w,
            Type::Unit | Type::Never | Type::Const(_) => 0,
            Type::Tuple(items) => {
                let mut n = 0;
                for it in items {
                    n += self.size(it)?;
                }
                n
            }
            Type::ErrSet(names) => {
                let mut n = abi::ERROR_PAYLOAD;
                for name in names {
                    n = n.max(self.size(&Type::Named(name.clone(), Vec::new()))?);
                }
                n
            }
            Type::Named(n, args) => {
                let arg = |i: usize| args.get(i).cloned().unwrap_or(Type::Unit);
                match n.as_str() {
                    "String" | "Path" | "list" | "Array" | "Slice" => 3 * w,
                    "i128" | "u128" => 2 * w,
                    "u64" | "f32" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "DateTime" | "Iter"
                    | "Box" | "dict" | "set" => w,
                    "Entry" => w + self.size(&arg(0))?,
                    "System" => abi::CAPABILITIES.len() as u32 * w,
                    c if abi::CAPABILITIES.iter().any(|(h, o)| *h == c || *o == c) => w,
                    "Option" => w + self.size(&arg(0))?,
                    "Result" => w + self.size(&arg(0))?.max(self.size(&arg(1))?),
                    _ => match self.decl(n) {
                        Some(sir::TypeDecl::Struct { fields, .. }) => {
                            let mut s = 0;
                            for f in fields {
                                s += self.size(&f.ty.subst(args))?;
                            }
                            s
                        }
                        Some(sir::TypeDecl::Enum { variants, error, .. }) => {
                            let start = if *error { abi::ERROR_PAYLOAD } else { w };
                            let mut s = start;
                            for v in variants {
                                let mut off = start;
                                for (_, ft) in &v.fields {
                                    off += self.size(&ft.subst(args))?;
                                }
                                s = s.max(off);
                            }
                            s
                        }
                        None => return Err(Error::unsupported(format!("values of the builtin type `{n}`"))),
                    },
                }
            }
            other => {
                return Err(Error::unsupported(format!("values of the type `{}`", sir::write::ty(other))));
            }
        })
    }

    /// The number of a concrete type, interning it (and its parts).
    pub(crate) fn id(&mut self, t: &Type) -> Result<u32, Error> {
        if let Some(&id) = self.ids.get(t) {
            return Ok(id);
        }
        let id = self.entries.len() as u32;
        self.ids.insert(t.clone(), id);
        self.entries.push(None);
        self.by_id.push(t.clone());
        let entry = self.make(t)?;
        // The size a value of the type is given where its entry is not
        // needed must be its entry's.
        if entry.size != self.size(t)? {
            return Err(Error::internal(format!("two sizes for the type `{}`", sir::write::ty(t))));
        }
        self.entries[id as usize] = Some(entry);
        Ok(id)
    }

    /// A part held by value: its number and size.
    fn part(&mut self, t: &Type) -> Result<(u32, u32), Error> {
        let size = self.size(t)?;
        Ok((self.id(t)?, size))
    }

    fn make(&mut self, t: &Type) -> Result<TypeEntry, Error> {
        let w = abi::WORD;
        let simple = |kind: Kind, size: u32| Ok(TypeEntry { size, kind });
        match t {
            Type::Int => simple(Kind::Int, w),
            Type::Float => simple(Kind::Float, w),
            Type::Bool => simple(Kind::Bool, w),
            Type::Char => simple(Kind::Char, w),
            Type::Str => simple(Kind::Str, 2 * w),
            Type::Unit | Type::Never => simple(Kind::Unit, 0),
            Type::Ref(inner, _) => {
                let target = self.id(inner)?;
                // A reference to a `dyn` is the `dyn`'s own two words.
                let size = if matches!(**inner, Type::Dyn(..)) { 2 * w } else { w };
                simple(Kind::Ref(target), size)
            }
            Type::Dyn(..) => simple(Kind::Dyn, 2 * w),
            Type::Fn(..) => simple(Kind::Fn, 2 * w),
            Type::Tuple(items) => {
                let mut slots = Vec::new();
                let mut off = 0;
                for it in items {
                    let (ty, size) = self.part(it)?;
                    slots.push(Slot { offset: off, ty });
                    off += size;
                }
                simple(Kind::Tuple(slots), off)
            }
            Type::ErrSet(names) => {
                let mut members = Vec::new();
                let mut size = abi::ERROR_PAYLOAD;
                for n in names {
                    let (id, s) = self.part(&Type::Named(n.clone(), Vec::new()))?;
                    members.push(id);
                    size = size.max(s);
                }
                simple(Kind::ErrSet(members), size)
            }
            Type::Named(n, args) => self.named(n, args),
            Type::Const(n) => match i64::try_from(*n) {
                Ok(v) => simple(Kind::Const(v), 0),
                Err(_) => Err(Error::unsupported("a constant argument beyond `int`")),
            },
            other => Err(Error::unsupported(format!("values of the type `{}`", sir::write::ty(other)))),
        }
    }

    fn named(&mut self, n: &str, args: &[Type]) -> Result<TypeEntry, Error> {
        let w = abi::WORD;
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Type::Unit);
        let simple = |kind: Kind, size: u32| Ok(TypeEntry { size, kind });
        match n {
            "String" => return simple(Kind::String, 3 * w),
            "Path" => return simple(Kind::Path, 3 * w),
            "u64" => return simple(Kind::U64, w),
            "i128" => return simple(Kind::I128, 2 * w),
            "u128" => return simple(Kind::U128, 2 * w),
            "f32" => return simple(Kind::F32, w),
            "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => return simple(Kind::Sized(n.to_string()), w),
            "DateTime" => return simple(Kind::Int, w),
            "System" => {
                let mut fields = Vec::new();
                for (i, (cap, _)) in abi::CAPABILITIES.iter().enumerate() {
                    let ty = self.id(&Type::Named(cap.to_string(), Vec::new()))?;
                    fields.push(Field { name: Some(cap.to_lowercase()), offset: i as u32 * w, ty });
                }
                return simple(
                    Kind::Struct { name: "System".into(), fields, glue: None },
                    abi::CAPABILITIES.len() as u32 * w,
                );
            }
            c if abi::CAPABILITIES.iter().any(|(h, o)| *h == c || *o == c) => {
                return simple(Kind::Opaque(c.to_string()), w);
            }
            "list" | "Array" | "Slice" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::List(e), 3 * w);
            }
            "Iter" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Iter(e), w);
            }
            "dict" => {
                let (k, v) = (self.id(&arg(0))?, self.id(&arg(1))?);
                return simple(Kind::Dict(k, v), w);
            }
            "set" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Set(e), w);
            }
            "Entry" => {
                let (k, ks) = self.part(&arg(0))?;
                let v = self.id(&arg(1))?;
                return simple(Kind::Entry(k, v), w + ks);
            }
            "Option" => {
                let (p, size) = self.part(&arg(0))?;
                return simple(Kind::Option(p), w + size);
            }
            "Result" => {
                let (a, sa) = self.part(&arg(0))?;
                let (b, sb) = self.part(&arg(1))?;
                return simple(Kind::Result(a, b), w + sa.max(sb));
            }
            "Box" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Box(e), w);
            }
            _ => {}
        }
        let Some(decl) = self.decl(n) else {
            return Err(Error::unsupported(format!("values of the builtin type `{n}`")));
        };
        let glue = self.glue_of(n, args);
        match decl {
            sir::TypeDecl::Struct { fields, .. } => {
                let mut out = Vec::new();
                let mut off = 0;
                for f in fields {
                    let ft = f.ty.subst(args);
                    let (ty, size) = self.part(&ft)?;
                    out.push(Field { name: Some(f.name.clone()), offset: off, ty });
                    off += size;
                }
                Ok(TypeEntry { size: off, kind: Kind::Struct { name: n.to_string(), fields: out, glue } })
            }
            sir::TypeDecl::Enum { variants, error, .. } => {
                let start = if *error { abi::ERROR_PAYLOAD } else { w };
                let mut vs = Vec::new();
                let mut size = start;
                for v in variants {
                    let mut fields = Vec::new();
                    let mut off = start;
                    for (name, ft) in &v.fields {
                        let ft = ft.subst(args);
                        let (ty, s) = self.part(&ft)?;
                        fields.push(Field { name: name.clone(), offset: off, ty });
                        off += s;
                    }
                    size = size.max(off);
                    vs.push(Variant { name: v.name.clone(), fields });
                }
                Ok(TypeEntry {
                    size,
                    kind: Kind::Enum { name: n.to_string(), variants: vs, glue, error: *error },
                })
            }
        }
    }

    /// The function-table index of a declared type's drop glue, when it
    /// has one.
    fn glue_of(&mut self, n: &str, args: &[Type]) -> Option<u32> {
        let name = sir::types::glue_name(n);
        self.prog.function(&name)?;
        let key = (name, args.to_vec());
        if let Some(&i) = self.glue_index.get(&key) {
            return Some(i);
        }
        let i = self.glue.len() as u32;
        self.glue.push(key.clone());
        self.glue_index.insert(key, i);
        Some(i)
    }

    /// Whether dropping a value of the type can do anything.
    pub(crate) fn needs_drop(&mut self, t: &Type) -> Result<bool, Error> {
        let mut seen = Vec::new();
        self.needs_drop_in(t, &mut seen)
    }

    fn needs_drop_in(&mut self, t: &Type, seen: &mut Vec<u32>) -> Result<bool, Error> {
        let id = self.id(t)?;
        if seen.contains(&id) {
            return Ok(false);
        }
        seen.push(id);
        let kind = match &self.entries[id as usize] {
            Some(e) => e.kind.clone(),
            None => return Ok(false),
        };
        Ok(match kind {
            Kind::String
            | Kind::Path
            | Kind::List(_)
            | Kind::Iter(_)
            | Kind::Box(_)
            | Kind::Dict(..)
            | Kind::Set(_)
            | Kind::Dyn => true,
            Kind::Struct { glue, .. } | Kind::Enum { glue, .. } => glue.is_some(),
            Kind::Entry(..) | Kind::Option(_) | Kind::Result(..) | Kind::Tuple(_) | Kind::ErrSet(_) => {
                let parts: Vec<u32> = match kind {
                    Kind::Option(p) | Kind::Entry(p, _) => vec![p],
                    Kind::Result(a, b) => vec![a, b],
                    Kind::Tuple(slots) => slots.iter().map(|s| s.ty).collect(),
                    Kind::ErrSet(m) => m,
                    _ => Vec::new(),
                };
                let mut any = false;
                for p in parts {
                    let pt = self.type_of_id(p);
                    if let Some(pt) = pt
                        && self.needs_drop_in(&pt, seen)?
                    {
                        any = true;
                    }
                }
                any
            }
            _ => false,
        })
    }

    /// The type a number stands for.
    pub(crate) fn type_of_id(&self, id: u32) -> Option<Type> {
        self.by_id.get(id as usize).cloned()
    }

    /// Where field `i` is in a value of type `t` (of `variant`, after a
    /// downcast), and its type.
    pub(crate) fn field(&mut self, t: &Type, variant: Option<u32>, i: u32) -> Result<(u32, Type), Error> {
        let bad = || Error::internal(format!("no field {i} in `{}`", sir::write::ty(t)));
        match (t, variant) {
            (Type::Tuple(items), None) => {
                let mut off = 0;
                for (k, it) in items.iter().enumerate() {
                    if k as u32 == i {
                        return Ok((off, it.clone()));
                    }
                    off += self.size(it)?;
                }
                Err(bad())
            }
            (Type::Named(n, args), Some(v)) if n == "Option" || n == "Result" => {
                let pt = match (n.as_str(), v) {
                    ("Option", _) | ("Result", 0) => args.first(),
                    _ => args.get(1),
                };
                let pt = pt.cloned().ok_or_else(bad)?;
                if i != 0 {
                    return Err(bad());
                }
                Ok((abi::WORD, pt))
            }
            (Type::Named(n, _), None) if n == "System" => {
                let (cap, _) = abi::CAPABILITIES.get(i as usize).ok_or_else(bad)?;
                Ok((i * abi::WORD, Type::Named(cap.to_string(), Vec::new())))
            }
            (Type::Named(n, args), _) => {
                let decl = self.decl(n).ok_or_else(bad)?;
                match (decl, variant) {
                    (sir::TypeDecl::Struct { fields, .. }, None) => {
                        let mut off = 0;
                        for (k, f) in fields.iter().enumerate() {
                            let ft = f.ty.subst(args);
                            if k as u32 == i {
                                return Ok((off, ft));
                            }
                            off += self.size(&ft)?;
                        }
                        Err(bad())
                    }
                    (sir::TypeDecl::Enum { variants, error, .. }, Some(v)) => {
                        let var = variants.get(v as usize).ok_or_else(bad)?;
                        let mut off = if *error { abi::ERROR_PAYLOAD } else { abi::WORD };
                        for (k, (_, ft)) in var.fields.iter().enumerate() {
                            let ft = ft.subst(args);
                            if k as u32 == i {
                                return Ok((off, ft));
                            }
                            off += self.size(&ft)?;
                        }
                        Err(bad())
                    }
                    _ => Err(bad()),
                }
            }
            _ => Err(bad()),
        }
    }

    /// The index of variant `name` of the enum `ty` (`Option`, `Result`
    /// or declared).
    pub(crate) fn variant_index(&self, ty: &str, name: &str) -> Option<u32> {
        match (ty, name) {
            ("Option", "Some") | ("Result", "Ok") => Some(0),
            ("Option", "None") | ("Result", "Err") => Some(1),
            _ => match self.decl(ty)? {
                sir::TypeDecl::Enum { variants, .. } => {
                    variants.iter().position(|v| v.name == name).map(|i| i as u32)
                }
                _ => None,
            },
        }
    }

    /// Whether a declared enum is an error set (its tag after two words).
    pub(crate) fn is_error(&self, t: &Type) -> bool {
        match t {
            Type::ErrSet(_) => true,
            Type::Named(n, _) => matches!(self.decl(n), Some(sir::TypeDecl::Enum { error: true, .. })),
            _ => false,
        }
    }
}
