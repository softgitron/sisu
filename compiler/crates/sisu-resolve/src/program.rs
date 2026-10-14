//! The resolved program: every definition of every module, with resolved
//! signatures, module scopes, method tables and the well-known
//! definitions of the prelude.

use crate::ty::{Ty, TyKey};
use sisu_diag::{FileId, Span};
use sisu_syntax::ast;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ModuleId(pub u32);

#[derive(Clone, Debug)]
pub struct Def {
    pub name: String,
    pub module: ModuleId,
    pub span: Span,
    pub vis: ast::Vis,
    pub docs: Vec<String>,
    pub kind: DefKind,
}

#[derive(Clone, Debug)]
pub enum DefKind {
    Struct(StructInfo),
    Enum(EnumInfo),
    ErrorSet(EnumInfo),
    Fn(FnInfo),
    Const(ConstInfo),
    Module(ModuleId),
    TypeAlias(Ty),
    Trait(TraitInfo),
    /// `impl Trait for Type` (an inherent `impl Type` registers its methods
    /// and is not a definition of its own).
    Impl(ImplInfo),
    /// A declaration this stage does not implement, kept so that names
    /// resolve and nothing cascades.
    Unsupported,
}

/// A trait with its type arguments, `Self` not among them: `Add[int]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TraitRef {
    pub def: DefId,
    pub args: Vec<Ty>,
}

/// `ty: bound`, from a generic parameter's bounds or a `where` clause.
#[derive(Clone, Debug, PartialEq)]
pub struct Predicate {
    pub ty: Ty,
    pub bound: TraitRef,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AssocInfo {
    pub name: String,
    /// `type Item = T` in the trait: the binding an impl that omits it gets.
    pub default: Option<Ty>,
    pub span: Span,
}

/// A trait (03, "Traits"). In its signatures `Self` is `Param(0)` and the
/// trait's own parameter `i` is `Param(i + 1)`.
#[derive(Clone, Debug, Default)]
pub struct TraitInfo {
    pub generics: Vec<Generic>,
    /// `trait Container(Len)`: every implementer implements these too.
    pub supertraits: Vec<TraitRef>,
    pub assoc: Vec<AssocInfo>,
    /// Required (no body) and provided methods, in declaration order.
    pub methods: Vec<DefId>,
    /// `@sealed`: only the standard library implements it.
    pub sealed: bool,
    /// `@marker`: no methods; implemented by the language's rules (`Copy`).
    pub marker: bool,
}

/// `impl[...] Trait[...] for Type`.
#[derive(Clone, Debug)]
pub struct ImplInfo {
    pub generics: Vec<Generic>,
    pub predicates: Vec<Predicate>,
    /// The trait, with arguments in terms of the impl's parameters.
    pub trait_ref: TraitRef,
    pub self_ty: Ty,
    /// The methods the impl defines (a provided method it omits is the
    /// trait's).
    pub methods: Vec<(String, DefId)>,
    /// The associated types it binds.
    pub assoc: Vec<(String, Ty)>,
    /// Made by `@derive` (or the implicit `Debug` every type has): the
    /// structural implementation the interpreter also knows.
    pub derived: bool,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub generics: Vec<Generic>,
    pub fields: Vec<FieldInfo>,
    /// For a host-backed type, the name in `@builtin("...")`.
    pub builtin: Option<String>,
    pub copy: bool,
    pub drop: Option<DefId>,
    pub view: bool,
    /// `@must_consume` (D-054).
    pub must_consume: bool,
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
    pub default: Option<ast::Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub generics: Vec<Generic>,
    pub variants: Vec<VariantInfo>,
    pub open: bool,
    pub copy: bool,
    pub drop: Option<DefId>,
    /// `@must_consume` (D-054).
    pub must_consume: bool,
}

#[derive(Clone, Debug)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<(Option<String>, Ty)>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FnKind {
    Free,
    Method,
    Assoc,
    Test(String),
    Drop,
}

#[derive(Clone, Debug)]
pub struct Generic {
    pub name: String,
    /// `A: Allocator = Global`: the type a use site that omits it gets, in
    /// terms of the earlier parameters.
    pub default: Option<Ty>,
}

#[derive(Clone, Debug)]
pub struct ParamInfo {
    pub name: String,
    pub mode: ast::Mode,
    pub ty: Ty,
    pub default: Option<ast::Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FnInfo {
    /// The impl's generic parameters first, then the function's own. For a
    /// trait's method, the impl's are `Self` and the trait's parameters.
    pub generics: Vec<Generic>,
    pub impl_generics: usize,
    /// The bounds and `where` clauses in scope in the signature and the
    /// body (the impl's, then the function's own; a trait method's
    /// include `Self: Trait`).
    pub predicates: Vec<Predicate>,
    /// A method of a trait (its declaration, and its body when provided).
    pub trait_: Option<DefId>,
    /// A method of a trait impl.
    pub impl_: Option<DefId>,
    /// `Some(mode)` for a method: how `self` is passed.
    pub self_mode: Option<ast::Mode>,
    /// The type a method or associated function belongs to.
    pub owner: Option<Ty>,
    pub params: Vec<ParamInfo>,
    /// Parameters from this index on are keyword-only (after a bare `*`).
    pub keyword_only_from: Option<usize>,
    /// Parameters before this index are positional-only (before a bare `/`).
    pub positional_only_until: usize,
    pub ret: Ty,
    /// Where the result type is written, for diagnostics about it.
    pub ret_span: Option<Span>,
    /// The `from` clause as written (`-> ref str from a, b`): parameter
    /// names, `self` included.
    pub from: Vec<(String, Span)>,
    /// `T !`: the error set is inferred from the body.
    pub err_inferred: bool,
    pub pure: bool,
    pub intrinsic: Option<String>,
    pub body: Option<ast::Block>,
    pub kind: FnKind,
    pub file: FileId,
    /// The header has a syntax error: the result type is `Error`, calls are
    /// not checked against the parameters, and names the body cannot find
    /// (most likely parameters lost in the error) are not reported.
    pub header_broken: bool,
    /// The body has a syntax error: a name it cannot find may be a binding
    /// lost in that error, so it is not reported.
    pub body_broken: bool,
}

#[derive(Clone, Debug)]
pub struct ConstInfo {
    pub ty: Ty,
    pub value: Option<ast::Expr>,
    pub file: FileId,
}

#[derive(Clone, Debug, Default)]
pub struct ModuleData {
    pub name: String,
    pub file: FileId,
    /// Names declared in or imported into the module.
    pub items: HashMap<String, DefId>,
    /// Declarations in source order.
    pub order: Vec<DefId>,
    /// Whether this is a standard-library module (intrinsics allowed).
    pub is_std: bool,
    /// For the impls `@derive` writes: the module of the derived types,
    /// whose names this module sees.
    pub parent: Option<ModuleId>,
}

impl ModuleData {
    /// The package the module belongs to, for the orphan rule (03,
    /// "Coherence"): the standard library is one package and the root
    /// module's file is the root package.
    pub fn package(&self) -> u32 {
        if self.is_std { 1 } else { 0 }
    }
}

/// Definitions of the prelude that the checker refers to by role.
#[derive(Clone, Debug, Default)]
pub struct WellKnown {
    pub option: Option<DefId>,
    pub result: Option<DefId>,
    pub ordering: Option<DefId>,
    pub string: Option<DefId>,
    pub list: Option<DefId>,
    pub dict: Option<DefId>,
    pub set: Option<DefId>,
    pub iter: Option<DefId>,
    pub entry: Option<DefId>,
    pub iterable: Option<DefId>,
    pub path: Option<DefId>,
    pub system: Option<DefId>,
    pub io_error: Option<DefId>,
    pub range: Option<DefId>,
    pub formatter: Option<DefId>,
    pub boxed: Option<DefId>,
    /// The traits the language itself uses (operators, interpolation,
    /// `for`, comparisons, `Drop`).
    pub display: Option<DefId>,
    pub debug: Option<DefId>,
    pub eq: Option<DefId>,
    pub partial_ord: Option<DefId>,
    pub ord: Option<DefId>,
    pub hash: Option<DefId>,
    pub len: Option<DefId>,
    pub copy: Option<DefId>,
    pub drop: Option<DefId>,
    pub iterator: Option<DefId>,
    pub add: Option<DefId>,
    pub sub: Option<DefId>,
    pub mul: Option<DefId>,
    pub div: Option<DefId>,
    pub rem: Option<DefId>,
    pub neg: Option<DefId>,
    pub num: Option<DefId>,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub defs: Vec<Def>,
    pub modules: Vec<ModuleData>,
    pub root: ModuleId,
    pub prelude: ModuleId,
    pub wk: WellKnown,
    /// Methods and associated functions by receiver type and name.
    pub methods: HashMap<(TyKey, String), DefId>,
    /// Every variant by its unqualified name: (enum or error set, index).
    pub variants: HashMap<String, Vec<(DefId, u32)>>,
    /// Tests in declaration order.
    pub tests: Vec<DefId>,
    /// Every trait impl, in declaration order.
    pub impls: Vec<DefId>,
    /// Names in top-level code of the root module that failed to parse; an
    /// undefined use of one is not reported (see `ast::Module::broken_names`).
    pub poisoned: std::collections::HashSet<String>,
}

impl Program {
    pub fn def(&self, d: DefId) -> &Def {
        &self.defs[d.0 as usize]
    }

    pub fn fn_info(&self, d: DefId) -> Option<&FnInfo> {
        match &self.def(d).kind {
            DefKind::Fn(f) => Some(f),
            _ => None,
        }
    }

    pub fn struct_info(&self, d: DefId) -> Option<&StructInfo> {
        match &self.def(d).kind {
            DefKind::Struct(s) => Some(s),
            _ => None,
        }
    }

    pub fn trait_info(&self, d: DefId) -> Option<&TraitInfo> {
        match &self.def(d).kind {
            DefKind::Trait(t) => Some(t),
            _ => None,
        }
    }

    pub fn impl_info(&self, d: DefId) -> Option<&ImplInfo> {
        match &self.def(d).kind {
            DefKind::Impl(i) => Some(i),
            _ => None,
        }
    }

    /// The generic parameters of a struct or enum.
    pub fn type_generics(&self, d: DefId) -> &[Generic] {
        match &self.def(d).kind {
            DefKind::Struct(s) => &s.generics,
            DefKind::Enum(e) | DefKind::ErrorSet(e) => &e.generics,
            _ => &[],
        }
    }

    pub fn enum_info(&self, d: DefId) -> Option<&EnumInfo> {
        match &self.def(d).kind {
            DefKind::Enum(e) | DefKind::ErrorSet(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_error_set(&self, d: DefId) -> bool {
        matches!(self.def(d).kind, DefKind::ErrorSet(_))
    }

    pub fn builtin(&self, d: DefId) -> Option<&str> {
        self.struct_info(d).and_then(|s| s.builtin.as_deref())
    }

    /// Looks a name up in a module (and the module it derives for), then
    /// in the prelude.
    pub fn lookup(&self, module: ModuleId, name: &str) -> Option<DefId> {
        let mut m = Some(module);
        while let Some(id) = m {
            let data = self.modules.get(id.0 as usize)?;
            if let Some(d) = data.items.get(name) {
                return Some(*d);
            }
            m = data.parent;
        }
        self.modules.get(self.prelude.0 as usize).and_then(|m| m.items.get(name).copied())
    }

    /// The module whose names `module` sees: itself, or for a module of
    /// derived impls, the module of the types.
    pub fn scope_module(&self, module: ModuleId) -> ModuleId {
        self.modules.get(module.0 as usize).and_then(|m| m.parent).unwrap_or(module)
    }

    /// Looks a name up in one module only.
    pub fn lookup_in(&self, module: ModuleId, name: &str) -> Option<DefId> {
        self.modules.get(module.0 as usize).and_then(|m| m.items.get(name).copied())
    }

    pub fn method(&self, key: TyKey, name: &str) -> Option<DefId> {
        self.methods.get(&(key, name.to_string())).copied()
    }

    /// The function `main` of the root module.
    pub fn main(&self) -> Option<DefId> {
        self.lookup_in(self.root, "main").filter(|&d| self.fn_info(d).is_some())
    }

    /// Every function with a body, in a stable order: the root module's
    /// declarations first.
    pub fn functions_with_bodies(&self) -> Vec<DefId> {
        let mut v: Vec<DefId> = (0..self.defs.len() as u32)
            .map(DefId)
            .filter(|&d| self.fn_info(d).is_some_and(|f| f.body.is_some()))
            .collect();
        v.sort_by_key(|d| (self.def(*d).module != self.root, *d));
        v
    }
}
