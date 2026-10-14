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
    /// A declaration this stage does not implement (a trait), kept so that
    /// names resolve and nothing cascades.
    Unsupported,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub generics: Vec<String>,
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
    pub generics: Vec<String>,
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
    pub bounds: Vec<String>,
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
    /// The impl's generic parameters first, then the function's own.
    pub generics: Vec<Generic>,
    pub impl_generics: usize,
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
    /// Whether this is a standard-library module (generics and intrinsics allowed).
    pub is_std: bool,
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

    /// Looks a name up in a module, then in the prelude.
    pub fn lookup(&self, module: ModuleId, name: &str) -> Option<DefId> {
        self.modules
            .get(module.0 as usize)
            .and_then(|m| m.items.get(name).copied())
            .or_else(|| self.modules.get(self.prelude.0 as usize).and_then(|m| m.items.get(name).copied()))
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
