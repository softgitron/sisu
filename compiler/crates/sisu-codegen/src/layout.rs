//! Layouts of concrete SIR types and the type table the runtime reads
//! (`sisu_rt::abi`): every type is a whole number of words, a struct or a
//! tuple its parts in order, an enum a tag word and its variants' payloads
//! over each other, an error set three words (its type's number, its
//! return trace, its tag) and the payload.

use crate::Error;
use crate::fx::FxMap;
use sisu_rt::abi::{self, Field, Kind, Slot, TypeEntry, Variant};
use sisu_sir as sir;
use sisu_sir::Type;

/// Whether a reference to the type is two words, the type of the value
/// and its address: a `dyn`, and an `Iterable[T]` (decision 210).
pub(crate) fn fat(t: &Type) -> bool {
    match t {
        Type::Dyn(..) => true,
        Type::Named(n, _) => n == "Iterable",
        _ => false,
    }
}

/// The interned types of a program and their entries.
pub(crate) struct Types<'p> {
    decls: FxMap<&'p str, &'p sir::TypeDecl>,
    ids: FxMap<Type, u32>,
    /// `None` while a type's entry is being made.
    entries: Vec<Option<TypeEntry>>,
    /// The type each number stands for.
    by_id: Vec<Type>,
    sizes: FxMap<Type, u32>,
    /// The types whose sizes are being computed.
    sizing: Vec<Type>,
    /// The function table: the instances the runtime calls itself (drop
    /// glue, a mutex guard's `invariant` check), which the entries name by
    /// their index: (function, type arguments).
    pub glue: Vec<(String, Vec<Type>)>,
    glue_index: FxMap<(String, Vec<Type>), u32>,
    /// Which declared types have drop glue, by name.
    with_glue: std::collections::HashMap<&'p str, bool>,
    /// The names of the program's functions.
    fn_names: FxMap<&'p str, ()>,
    /// Whether dropping a value of each type can do anything, by number,
    /// as far as asked.
    drops: Vec<Option<bool>>,
    /// The words of a function value's environment: the largest an `own`
    /// closure of the program needs, at least one (decision 224).
    pub env_words: u32,
}

impl<'p> Types<'p> {
    pub(crate) fn new(prog: &'p sir::Program, env_words: u32) -> Types<'p> {
        let decls = prog.types.iter().map(|t| (t.name(), t)).collect();
        let fn_names: FxMap<&'p str, ()> = prog.functions.iter().map(|f| (f.name.as_str(), ())).collect();
        let with_glue = prog
            .types
            .iter()
            .map(|t| (t.name(), fn_names.contains_key(sir::types::glue_name(t.name()).as_str())))
            .collect();
        Types {
            with_glue,
            fn_names,
            drops: Vec::new(),
            decls,
            ids: FxMap::default(),
            entries: Vec::new(),
            by_id: Vec::new(),
            sizes: FxMap::default(),
            sizing: Vec::new(),
            glue: Vec::new(),
            glue_index: FxMap::default(),
            env_words,
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
            Type::Ref(inner, _) if fat(inner) => 2 * w,
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Ref(..) => w,
            Type::Str | Type::Dyn(..) => 2 * w,
            Type::Fn(..) => (2 + self.env_words) * w,
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
                    "String" | "Path" | "list" | "Array" | "Slice" | "bytes" => 3 * w,
                    "i128" | "u128" => 2 * w,
                    "u64" | "f32" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "DateTime" | "Iter"
                    | "Box" | "dict" | "set" => w,
                    // A raw pointer is an address (22, "Raw pointers").
                    "ptr" | "ptr_mut" => w,
                    // The values of tasks and channels are numbers in the
                    // runtime's scheduler; a guard is the address of the
                    // value its mutex holds.
                    "Scope" | "Handle" | "Cancellation" | "Sender" | "Receiver" | "MutexGuard"
                    | "ReadGuard" => w,
                    "Timer" => 2 * w,
                    "Mutex" | "RwLock" => abi::MUTEX_HEADER + self.size(&arg(0))?,
                    // A `Shared` (or `Weak`) is the address of the value in
                    // its cell, an `Arena` the address of its slots, a
                    // `Cell` its value.
                    "Shared" | "Weak" | "Arena" => w,
                    // An `Atomic` is a `Cell` that tasks may share.
                    "Cell" | "Atomic" => self.size(&arg(0))?,
                    // A `Cap` is the address of its value's cell, a test
                    // clock its clock's number, and a `Lent` its handle.
                    "Cap" | "TestClock" | "HostStream" | "HostListener" => w,
                    "Iterable" => 2 * w,
                    "Lent" => self.size(&arg(0))?,
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

    /// Whether a value of the type lies in raw memory as it lies in a value
    /// (the runtime's `rawmem::layout`, decision 302): no sized integer
    /// narrower than 64 bits, `f32` or option of a raw pointer is among the
    /// parts it holds in place, which raw memory keeps at their own width.
    /// A reference into raw memory is only made to such a value, since the
    /// code reads a reference's value as a value is laid out.
    pub(crate) fn same_in_memory(&self, t: &Type) -> bool {
        self.same_in_memory_at(t, 0)
    }

    fn same_in_memory_at(&self, t: &Type, depth: u32) -> bool {
        if depth > 64 {
            return false;
        }
        match t {
            Type::Tuple(items) => items.iter().all(|x| self.same_in_memory_at(x, depth + 1)),
            Type::Named(n, args) => match n.as_str() {
                "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "f32" => false,
                "Option" => !matches!(args.first(), Some(Type::Named(p, _)) if p == "ptr" || p == "ptr_mut"),
                _ => match self.decl(n) {
                    // A builtin that SIR declares as a struct has no fields.
                    Some(sir::TypeDecl::Struct { fields, .. }) => {
                        fields.iter().all(|f| self.same_in_memory_at(&f.ty.subst(args), depth + 1))
                    }
                    _ => true,
                },
            },
            _ => true,
        }
    }

    /// The number of a concrete type, if a value of it was made (the code
    /// interned it).
    pub(crate) fn known_id(&self, t: &Type) -> Option<u32> {
        self.ids.get(t).copied()
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
                let size = if fat(inner) { 2 * w } else { w };
                simple(Kind::Ref(target), size)
            }
            Type::Dyn(..) => simple(Kind::Dyn, 2 * w),
            Type::Fn(..) => simple(Kind::Fn, (2 + self.env_words) * w),
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
            "bytes" => return simple(Kind::Bytes, 3 * w),
            // An `Iterable[T]` parameter is a `dyn` of what the caller
            // passed (decision 210).
            "Iterable" => return simple(Kind::Dyn, 2 * w),
            // A host socket is its number in the runtime's table.
            "Cap" | "TestClock" | "HostStream" | "HostListener" => {
                return simple(Kind::Opaque(n.to_string()), w);
            }
            // A lent capability is its handle (or test clock), at no cost,
            // but not counted: its name says it is lent (decision 237).
            "Lent" => {
                let inner = self.id(&arg(0))?;
                return match self.entries.get(inner as usize) {
                    Some(Some(TypeEntry { size, kind: Kind::Opaque(n) })) => {
                        Ok(TypeEntry { size: *size, kind: Kind::Opaque(format!("Lent {n}")) })
                    }
                    Some(Some(e)) => Ok(e.clone()),
                    _ => Err(Error::unsupported("a `Lent` of a type that holds itself")),
                };
            }
            "u64" => return simple(Kind::U64, w),
            "i128" => return simple(Kind::I128, 2 * w),
            "u128" => return simple(Kind::U128, 2 * w),
            "f32" => return simple(Kind::F32, w),
            "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => return simple(Kind::Sized(n.to_string()), w),
            "DateTime" => return simple(Kind::DateTime, w),
            "System" => {
                let mut fields = Vec::new();
                for (i, (cap, _)) in abi::CAPABILITIES.iter().enumerate() {
                    let ty = self.id(&Type::Named(cap.to_string(), Vec::new()))?;
                    fields.push(Field {
                        name: Some(cap.to_lowercase()),
                        offset: i as u32 * w,
                        ty,
                        glue: false,
                    });
                }
                return simple(
                    Kind::Struct { name: "System".into(), fields, glue: None },
                    abi::CAPABILITIES.len() as u32 * w,
                );
            }
            c if abi::CAPABILITIES.iter().any(|(h, o)| *h == c || *o == c) => {
                return simple(Kind::Opaque(c.to_string()), w);
            }
            "list" | "Array" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::List(e), 3 * w);
            }
            // A slice views its list in place (decision 940).
            "Slice" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Slice(e), 3 * w);
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
            "ptr" | "ptr_mut" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Ptr(e), w);
            }
            "Scope" => return simple(Kind::Scope, w),
            "Cancellation" => return simple(Kind::Cancellation, w),
            "Timer" => return simple(Kind::Timer, 2 * w),
            "Handle" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Handle(e), w);
            }
            "Sender" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Sender(e), w);
            }
            "Receiver" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Receiver(e), w);
            }
            "Mutex" => {
                let (e, size) = self.part(&arg(0))?;
                return simple(Kind::Mutex(e), abi::MUTEX_HEADER + size);
            }
            "RwLock" => {
                let (e, size) = self.part(&arg(0))?;
                return simple(Kind::RwLock(e), abi::MUTEX_HEADER + size);
            }
            "ReadGuard" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::ReadGuard(e), w);
            }
            "Shared" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Shared(e), w);
            }
            "Weak" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Weak(e), w);
            }
            "Arena" => {
                let e = self.id(&arg(0))?;
                return simple(Kind::Arena(e), w);
            }
            "Cell" | "Atomic" => {
                let (e, size) = self.part(&arg(0))?;
                return simple(Kind::Cell(e), size);
            }
            "MutexGuard" => {
                let value = arg(0);
                let e = self.id(&value)?;
                // A guard dropped by unwinding checks the value's invariant
                // (D-077), a function the runtime calls by its index.
                let check = match &value {
                    Type::Named(vn, vargs) => self.function_index(&format!("{vn}.__invariant"), vargs),
                    _ => None,
                };
                return simple(Kind::Guard(e, check), w);
            }
            _ => {}
        }
        let Some(decl) = self.decl(n) else {
            return Err(Error::unsupported(format!("values of the builtin type `{n}`")));
        };
        let glue = self.glue_of(n, args);
        // What the glue drops is decided on the declared types, so a field
        // of a type parameter is the glue's (decision 218).
        let by_glue = |t: &Type| glue.is_some() && sir::types::drops_something(t, &self.with_glue);
        let flags: Vec<Vec<bool>> = match decl {
            sir::TypeDecl::Struct { fields, .. } => vec![fields.iter().map(|f| by_glue(&f.ty)).collect()],
            sir::TypeDecl::Enum { variants, .. } => {
                variants.iter().map(|v| v.fields.iter().map(|(_, t)| by_glue(t)).collect()).collect()
            }
        };
        self.declared(n, args, decl, glue, &flags)
    }

    /// The entry of a declared struct or enum; `by_glue` says which fields
    /// (of each variant) its glue drops.
    fn declared(
        &mut self,
        n: &str,
        args: &[Type],
        decl: &'p sir::TypeDecl,
        glue: Option<u32>,
        by_glue: &[Vec<bool>],
    ) -> Result<TypeEntry, Error> {
        let w = abi::WORD;
        let flag = |v: usize, i: usize| by_glue.get(v).and_then(|f| f.get(i)).copied().unwrap_or(false);
        match decl {
            sir::TypeDecl::Struct { fields, .. } => {
                let mut out = Vec::new();
                let mut off = 0;
                for (i, f) in fields.iter().enumerate() {
                    let glue = flag(0, i);
                    let ft = f.ty.subst(args);
                    let (ty, size) = self.part(&ft)?;
                    out.push(Field { name: Some(f.name.clone()), offset: off, ty, glue });
                    off += size;
                }
                Ok(TypeEntry { size: off, kind: Kind::Struct { name: n.to_string(), fields: out, glue } })
            }
            sir::TypeDecl::Enum { variants, error, .. } => {
                let start = if *error { abi::ERROR_PAYLOAD } else { w };
                let mut vs = Vec::new();
                let mut size = start;
                for (k, v) in variants.iter().enumerate() {
                    let mut fields = Vec::new();
                    let mut off = start;
                    for (i, (name, ft)) in v.fields.iter().enumerate() {
                        let glue = flag(k, i);
                        let ft = ft.subst(args);
                        let (ty, s) = self.part(&ft)?;
                        fields.push(Field { name: name.clone(), offset: off, ty, glue });
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
        self.function_index(&sir::types::glue_name(n), args)
    }

    /// The function-table index of the instance of `name` for `args`, when
    /// the program has the function: what the runtime calls itself (drop
    /// glue, an `invariant` check).
    fn function_index(&mut self, name: &str, args: &[Type]) -> Option<u32> {
        if !self.fn_names.contains_key(name) {
            return None;
        }
        let key = (name.to_string(), args.to_vec());
        if let Some(&i) = self.glue_index.get(&key) {
            return Some(i);
        }
        let i = self.glue.len() as u32;
        self.glue.push(key.clone());
        self.glue_index.insert(key, i);
        Some(i)
    }

    /// Whether a program can observe a value's drop: a `Drop` runs, a
    /// count changes, a mutex is unlocked (SIR's `drops_something`). Any
    /// other drop only frees memory.
    pub(crate) fn observable_drop(&self, t: &Type) -> bool {
        sir::types::drops_something(t, &self.with_glue)
    }

    /// Whether dropping a value of the type can do anything.
    pub(crate) fn needs_drop(&mut self, t: &Type) -> Result<bool, Error> {
        let id = self.id(t)? as usize;
        if let Some(Some(known)) = self.drops.get(id) {
            return Ok(*known);
        }
        let mut seen = Vec::new();
        let answer = self.needs_drop_in(t, &mut seen)?;
        if self.drops.len() <= id {
            self.drops.resize(id + 1, None);
        }
        self.drops[id] = Some(answer);
        Ok(answer)
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
            | Kind::Bytes
            | Kind::List(_)
            | Kind::Slice(_)
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
            // A capability handle's copies are counted (decision 237), and
            // so are those a function value's environment holds (decision
            // 248).
            Kind::Opaque(n) => abi::counted_handle(&n),
            Kind::Fn => true,
            Kind::Mutex(v) | Kind::RwLock(v) | Kind::Cell(v) => match self.type_of_id(v) {
                Some(vt) => self.needs_drop_in(&vt, seen)?,
                None => false,
            },
            // An error holds its return trace (decision 215).
            Kind::Enum { error: true, .. } => true,
            Kind::Struct { glue: Some(_), .. } | Kind::Enum { glue: Some(_), .. } => true,
            // A field the glue leaves out, or every field without glue, is
            // the runtime's to drop (decision 218).
            Kind::Entry(..)
            | Kind::Option(_)
            | Kind::Result(..)
            | Kind::Tuple(_)
            | Kind::ErrSet(_)
            | Kind::Struct { .. }
            | Kind::Enum { .. } => {
                let parts: Vec<u32> = match kind {
                    Kind::Option(p) | Kind::Entry(p, _) => vec![p],
                    Kind::Result(a, b) => vec![a, b],
                    Kind::Tuple(slots) => slots.iter().map(|s| s.ty).collect(),
                    Kind::ErrSet(m) => m,
                    Kind::Struct { fields, .. } => fields.iter().map(|f| f.ty).collect(),
                    Kind::Enum { variants, .. } => {
                        variants.iter().flat_map(|v| v.fields.iter().map(|f| f.ty)).collect()
                    }
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

    /// Whether a value of the type holds a capability handle or a `Cap` by
    /// value, through options, results, tuples, structs and enums: a copy
    /// of it counts (decision 237). A `Lent` holds none.
    pub(crate) fn counted(&mut self, t: &Type) -> Result<bool, Error> {
        if !self.needs_drop(t)? {
            return Ok(false);
        }
        let mut seen = Vec::new();
        let id = self.id(t)?;
        Ok(self.counted_in(id, &mut seen))
    }

    fn counted_in(&self, id: u32, seen: &mut Vec<u32>) -> bool {
        if seen.contains(&id) {
            return false;
        }
        seen.push(id);
        let Some(Some(e)) = self.entries.get(id as usize) else { return false };
        let parts: Vec<u32> = match &e.kind {
            Kind::Opaque(n) => return abi::counted_handle(n),
            Kind::Fn => return true,
            Kind::Option(p) => vec![*p],
            Kind::Result(a, b) => vec![*a, *b],
            Kind::Tuple(slots) => slots.iter().map(|s| s.ty).collect(),
            Kind::Struct { fields, .. } => fields.iter().map(|f| f.ty).collect(),
            Kind::Enum { variants, .. } => {
                variants.iter().flat_map(|v| v.fields.iter().map(|f| f.ty)).collect()
            }
            _ => return false,
        };
        parts.into_iter().any(|p| self.counted_in(p, seen))
    }

    /// Whether a value of the type holds a function value in place,
    /// through options, results, tuples, structs and enums: an `own`
    /// closure's environment that does is wider than any function value,
    /// whose width is the widest environment's (decision 360).
    pub(crate) fn holds_fn(&mut self, t: &Type) -> Result<bool, Error> {
        let mut seen = Vec::new();
        let id = self.id(t)?;
        Ok(self.holds_fn_in(id, &mut seen))
    }

    fn holds_fn_in(&self, id: u32, seen: &mut Vec<u32>) -> bool {
        if seen.contains(&id) {
            return false;
        }
        seen.push(id);
        let Some(Some(e)) = self.entries.get(id as usize) else { return false };
        let parts: Vec<u32> = match &e.kind {
            Kind::Fn => return true,
            Kind::Option(p) => vec![*p],
            Kind::Result(a, b) => vec![*a, *b],
            Kind::Tuple(slots) => slots.iter().map(|s| s.ty).collect(),
            Kind::Struct { fields, .. } => fields.iter().map(|f| f.ty).collect(),
            Kind::Enum { variants, .. } => {
                variants.iter().flat_map(|v| v.fields.iter().map(|f| f.ty)).collect()
            }
            _ => return false,
        };
        parts.into_iter().any(|p| self.holds_fn_in(p, seen))
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
