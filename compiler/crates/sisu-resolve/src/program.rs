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
    Fn(Box<FnInfo>),
    Const(ConstInfo),
    Module(ModuleId),
    TypeAlias(Ty),
    Trait(TraitInfo),
    /// `impl Trait for Type` (an inherent `impl Type` registers its methods
    /// and is not a definition of its own).
    Impl(ImplInfo),
    /// `type Port = int where 1 <= self <= 65535` (07): a subtype of its
    /// base whose values satisfy the predicate.
    Refinement(RefinementInfo),
    /// A declaration this stage does not implement, kept so that names
    /// resolve and nothing cascades.
    Unsupported,
}

/// A refinement type (07, "Refinement types"): a nominal subtype of `base`
/// whose every value satisfies its predicate. A value of the base becomes
/// one by `X.try_from(v)`, checked when it runs, or by a constant written
/// where one is expected, checked at compile time; a value of the
/// refinement is used wherever its base is.
#[derive(Clone, Debug)]
pub struct RefinementInfo {
    pub generics: Vec<Generic>,
    /// The base type, in terms of the refinement's parameters.
    pub base: Ty,
    /// The predicate, a `pure` function of the base value (`self`) that
    /// the compiler writes from the `where` clause: `Port#where`.
    pub predicate: Option<DefId>,
    /// The predicate as written.
    pub text: String,
    /// For an integer refinement, the range `[lo, hi)` its predicate
    /// bounds `self` to, when it is a conjunction of comparisons of `self`
    /// with constants: what `exhaustive[T]` enumerates.
    pub domain: Option<(i64, i64)>,
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
    /// Its `comptime def`s but `derive`: compile-time functions a program
    /// names through the trait, `Trait.name(...)` (decision 251).
    pub comptime_fns: Vec<(String, DefId)>,
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
    /// The file the default is written in: the struct's, or the text its
    /// unfolding made when it reflects (decision 251).
    pub default_file: FileId,
    pub span: Span,
    /// `pub x: int` (09): who may read, write and construct with it.
    pub vis: ast::Vis,
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
    /// The predicate of the refinement type, which the compiler writes
    /// from its `where` clause (`Port#where`).
    Predicate(DefId),
    /// The check of a struct's `invariant` lines, which the compiler
    /// writes as a method of the struct (`Account.__check_invariant`).
    Invariant(DefId),
}

#[derive(Clone, Debug)]
pub struct Generic {
    pub name: String,
    /// `A: Allocator = Global`: the type a use site that omits it gets, in
    /// terms of the earlier parameters.
    pub default: Option<Ty>,
    /// `const N: int` (03, "Const generics"): the argument is a constant
    /// (`Ty::Const`), and the name is an `int` in expressions.
    pub is_const: bool,
}

#[derive(Clone, Debug)]
pub struct ParamInfo {
    pub name: String,
    pub mode: ast::Mode,
    pub ty: Ty,
    pub default: Option<ast::Expr>,
    /// The file the default is written in: the function's, or the text its
    /// unfolding made when it reflects (decision 251).
    pub default_file: FileId,
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
    /// `comptime def` (08): callable only at compile time.
    pub comptime: bool,
    pub intrinsic: Option<String>,
    /// `@structural("name")` on a std function with a body: the intrinsic
    /// `name` computes the same result when every bound of the call's type
    /// arguments is met by a structural impl, and lowering calls it then.
    pub structural: Option<String>,
    /// `@keeps(self, x)` on an intrinsic: the `own` and `mut` arguments
    /// (by position, `self` first) whose every value it keeps or returns.
    pub keeps: Vec<u32>,
    /// `@internal` on a std function: only the standard library may call
    /// it (the forwarding of lent capabilities, the assembling of a
    /// `System`).
    pub internal: bool,
    pub body: Option<std::sync::Arc<ast::Block>>,
    pub kind: FnKind,
    pub file: FileId,
    /// Written by the compiler from code that is checked elsewhere too (a
    /// struct's invariant as a `bool`): its diagnostics are not reported
    /// again.
    pub quiet: bool,
    /// A generic function that reflects on its type parameters (08,
    /// "Reflection"), or calls one with them: a template, checked and
    /// lowered once per list of type arguments (`Program::instances`).
    pub template: bool,
    /// The template and type arguments an instance was made for.
    pub instance_of: Option<(DefId, Vec<Ty>)>,
    /// The `T: type` parameters of a `comptime def` (08, "Reflection";
    /// decision 251): each one's position among the arguments a call
    /// writes and its name, and the generic parameter whose argument the
    /// call's argument, a type, is. They are not among `params`.
    pub type_params: Vec<(usize, String, u32)>,
    /// The header has a syntax error: the result type is `Error`, calls are
    /// not checked against the parameters, and names the body cannot find
    /// (most likely parameters lost in the error) are not reported.
    pub header_broken: bool,
    /// The body has a syntax error: a name it cannot find may be a binding
    /// lost in that error, so it is not reported.
    pub body_broken: bool,
}

impl FnInfo {
    /// Whether this template is made once, for all its type arguments at
    /// once: one without type parameters (decision 244), or a method of a
    /// trait impl, whose instance is as generic as the template, so that a
    /// trait's call reaches it whatever the type arguments (decision 251).
    pub fn made_once(&self) -> bool {
        self.template && (self.generics.is_empty() || self.impl_.is_some())
    }
}

#[derive(Clone, Debug)]
pub struct ConstInfo {
    pub ty: Ty,
    pub value: Option<ast::Expr>,
    pub file: FileId,
}

#[derive(Clone, Debug, Default)]
pub struct ModuleData {
    /// The module's name in SIR: `main` for the root module, the import
    /// path for the program's other modules (`weather.parser`), the std
    /// module's name (`json`), or a generated module's (`<derive>/main.sisu`).
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
    /// The package the module belongs to (an index of
    /// `Program::packages`): [`ROOT_PACKAGE`] for the program's own code,
    /// [`STD_PACKAGE`] for the standard library, and the others for the
    /// dependencies.
    pub package: u32,
    /// The names an import bound here (09, "Modules"), each with the
    /// import's visibility: only a `pub` import is seen through the
    /// module from another (`pub from a import x` re-exports `x`).
    pub imported: HashMap<String, ast::Vis>,
    /// The path an import names the module by (`weather.parser`); empty
    /// for a module no import names.
    pub path: Vec<String>,
}

/// The program's own package: the one being built, which nothing depends
/// on (the orphan rule's exception, D-013).
pub const ROOT_PACKAGE: u32 = 0;
/// The standard library, one package.
pub const STD_PACKAGE: u32 = 1;

impl ModuleData {
    /// The package the module belongs to, for the orphan rule (03,
    /// "Coherence") and `pub(package)`.
    pub fn package(&self) -> u32 {
        if self.is_std { STD_PACKAGE } else { self.package }
    }
}

/// A package of the program (09): the root package, the standard library,
/// or a dependency.
#[derive(Clone, Debug, Default)]
pub struct PackageData {
    /// Its name as `sisu.toml` gives it (and imports name it); empty for a
    /// single file, which is a package of its own without a name.
    pub name: String,
    /// `[trust] unsafe` lists it, or it is the root package or std: it may
    /// contain `unsafe` and `extern` (09, "`[trust]` is where capabilities
    /// become real").
    pub trusted: bool,
    /// The packages its imports may name, by the names they are imported
    /// by: its own dependencies (and only those: a package sees its
    /// dependencies' dependencies only through them).
    pub deps: Vec<(String, u32)>,
    /// Its root module.
    pub root: Option<ModuleId>,
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
    pub from_literal: Option<DefId>,
    /// Tasks (06).
    pub task_error: Option<DefId>,
    pub scope: Option<DefId>,
    pub handle: Option<DefId>,
    pub sender: Option<DefId>,
    pub receiver: Option<DefId>,
    pub send: Option<DefId>,
    pub sync: Option<DefId>,
    pub escapable: Option<DefId>,
    /// Capabilities (06): the operations trait of each handle type.
    pub fs_ops: Option<DefId>,
    pub net_ops: Option<DefId>,
    pub clock_ops: Option<DefId>,
    pub rand_ops: Option<DefId>,
    pub env_ops: Option<DefId>,
    pub stdin_ops: Option<DefId>,
    pub proc_ops: Option<DefId>,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub defs: Vec<Def>,
    pub modules: Vec<ModuleData>,
    pub root: ModuleId,
    pub prelude: ModuleId,
    /// The packages, by [`ModuleData::package`]: the root package first,
    /// then std, then the dependencies.
    pub packages: Vec<PackageData>,
    /// The modules of the program's own source (the root package's and its
    /// dependencies'), the root module first, in the order they were given.
    pub user_modules: Vec<ModuleId>,
    pub wk: WellKnown,
    /// Methods and associated functions by receiver type and name.
    pub methods: HashMap<(TyKey, String), DefId>,
    /// Associated constants (`Money.ZERO`) by type and name.
    pub assoc_consts: HashMap<(TyKey, String), DefId>,
    /// The generator the compiler wrote for each type written in
    /// `arbitrary[T]` or `exhaustive[T](...)`, by its spelling without
    /// spaces, or why it has none (17).
    pub arbitrary_sites: HashMap<String, Result<String, String>>,
    /// Every variant by its unqualified name: (enum or error set, index).
    pub variants: HashMap<String, Vec<(DefId, u32)>>,
    /// Tests in declaration order.
    pub tests: Vec<DefId>,
    /// The `@tag("...")`s of each test that has some (17).
    pub test_tags: HashMap<DefId, Vec<String>>,
    /// Benchmarks in declaration order (`bench "name"(b: Bench):`, 17).
    pub benches: Vec<DefId>,
    /// Every trait impl, in declaration order.
    pub impls: Vec<DefId>,
    /// Names in top-level code of the root module that failed to parse; an
    /// undefined use of one is not reported (see `ast::Module::broken_names`).
    pub poisoned: std::collections::HashSet<String>,
    /// `@comptime_budget(steps=N)` on a function or constant: the steps its
    /// `comptime` expressions may take (08), instead of the default.
    pub comptime_budgets: HashMap<DefId, u64>,
    /// The instance of each template for each list of type arguments it is
    /// called with (08, "Reflection").
    pub instances: HashMap<(DefId, Vec<Ty>), DefId>,
    /// Where each instance was first needed, in the program's own code.
    pub instance_sites: HashMap<DefId, Span>,
    /// The fake the compiler wrote for each of the program's traits that
    /// `cap.Fake[Trait]` can name (17).
    pub fakes: HashMap<DefId, DefId>,
    /// The files `embed("path")` names, read by the driver: the text, or
    /// why it could not be read (08).
    pub embedded: HashMap<String, Result<String, String>>,
}

/// What the type checker asks of the next resolution (08, "Reflection"):
/// an instance of a template for concrete type arguments, or that a
/// generic function become a template itself, since it calls one with its
/// own type parameters.
///
/// An instance request carries the call that first needed it: in the
/// program's own code, which an instance needed by another instance
/// inherits, so that an instance that cannot be made is reported there.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Request {
    Instance { def: DefId, args: Vec<Ty>, site: Span },
    Promote(DefId),
}

impl Request {
    /// Whether the two ask for the same thing (wherever it was needed).
    pub fn same(&self, other: &Request) -> bool {
        match (self, other) {
            (Request::Instance { def: a, args: x, .. }, Request::Instance { def: b, args: y, .. }) => {
                a == b && x == y
            }
            (Request::Promote(a), Request::Promote(b)) => a == b,
            _ => false,
        }
    }
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

    /// The generic parameters of a struct, enum or refinement type.
    pub fn type_generics(&self, d: DefId) -> &[Generic] {
        match &self.def(d).kind {
            DefKind::Struct(s) => &s.generics,
            DefKind::Enum(e) | DefKind::ErrorSet(e) => &e.generics,
            DefKind::Refinement(r) => &r.generics,
            _ => &[],
        }
    }

    pub fn refinement(&self, d: DefId) -> Option<&RefinementInfo> {
        match &self.def(d).kind {
            DefKind::Refinement(r) => Some(r),
            _ => None,
        }
    }

    /// The base of a refinement type with these arguments (`list[int]` for
    /// `NonEmpty[int]`).
    pub fn refinement_base(&self, d: DefId, args: &[Ty]) -> Option<Ty> {
        self.refinement(d).map(|r| r.base.subst(args))
    }

    /// The type with every refinement replaced by its base, at the top
    /// only (`Port` is `int`; `list[Port]` stays as it is).
    pub fn unrefine(&self, ty: &Ty) -> Ty {
        let mut t = ty.clone();
        for _ in 0..16 {
            match &t {
                Ty::Adt(d, args) => match self.refinement_base(*d, args) {
                    Some(b) => t = b,
                    None => return t,
                },
                _ => return t,
            }
        }
        t
    }

    /// The type with every refinement anywhere in it replaced by its base:
    /// how values of it are represented.
    pub fn erase_refinements(&self, ty: &Ty) -> Ty {
        let mut has = false;
        ty.walk(&mut |t| has |= matches!(t, Ty::Adt(d, _) if self.refinement(*d).is_some()));
        if !has {
            return ty.clone();
        }
        self.erase_depth(ty, 0)
    }

    fn erase_depth(&self, ty: &Ty, depth: u32) -> Ty {
        if depth > 32 {
            return ty.clone();
        }
        let e = |t: &Ty| self.erase_depth(t, depth + 1);
        match ty {
            Ty::Adt(d, args) => {
                let args: Vec<Ty> = args.iter().map(e).collect();
                match self.refinement_base(*d, &args) {
                    Some(b) => e(&b),
                    None => Ty::Adt(*d, args),
                }
            }
            Ty::Tuple(es, l) => Ty::Tuple(es.iter().map(e).collect(), l.clone()),
            Ty::Fn(f) => Ty::Fn(Box::new(crate::ty::FnTy {
                params: f.params.iter().map(e).collect(),
                ret: e(&f.ret),
                pure: f.pure,
            })),
            Ty::Ref(t, m) => Ty::Ref(Box::new(e(t)), *m),
            Ty::Dyn(tr) => Ty::Dyn(TraitRef { def: tr.def, args: tr.args.iter().map(e).collect() }),
            Ty::Assoc(a) => Ty::Assoc(Box::new(crate::ty::AssocTy {
                self_ty: e(&a.self_ty),
                trait_: TraitRef { def: a.trait_.def, args: a.trait_.args.iter().map(e).collect() },
                name: a.name.clone(),
            })),
            other => other.clone(),
        }
    }

    /// The check of a struct's invariant, `T.__check_invariant`, when the
    /// struct has `invariant` lines.
    pub fn invariant_check(&self, d: DefId) -> Option<DefId> {
        let m = self.method(crate::ty::TyKey::Adt(d), "__check_invariant")?;
        matches!(self.fn_info(m)?.kind, FnKind::Invariant(_)).then_some(m)
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

    /// The operations trait of a capability handle type (`Fs`: `FsOps`).
    pub fn capability_ops(&self, d: DefId) -> Option<DefId> {
        let wk = &self.wk;
        match self.builtin(d)? {
            "Fs" => wk.fs_ops,
            "Net" => wk.net_ops,
            "Clock" => wk.clock_ops,
            "Rand" => wk.rand_ops,
            "Env" => wk.env_ops,
            "Proc" => wk.proc_ops,
            "Stdin" => wk.stdin_ops,
            _ => None,
        }
    }

    /// Whether a value of the type is or lends a capability, or is the
    /// `System` of them: what a `pure` function cannot take (06).
    pub fn is_capability(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Ref(t, _) => self.is_capability(t),
            Ty::Adt(d, args) => {
                self.capability_ops(*d).is_some()
                    || matches!(
                        self.builtin(*d),
                        Some(
                            "System"
                                | "TestClock"
                                | "HostFs"
                                | "HostNet"
                                | "HostClock"
                                | "HostRand"
                                | "HostEnv"
                                | "HostProc"
                                | "HostStdin"
                        )
                    )
                    || (self.builtin(*d) == Some("Lent") && args.iter().any(|a| self.is_capability(a)))
            }
            _ => false,
        }
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

    /// Whether code in `from` may name `def`, an item of another module
    /// (09, "Modules"): items are private to their module unless `pub`,
    /// and `pub(package)` reaches the modules of the same package. A
    /// module is always reachable by its path. The standard library's
    /// items are all public in stage 0 (its `@internal` functions are
    /// checked apart), and code the compiler writes sees what the module
    /// it was written for sees.
    pub fn visible_from(&self, def: DefId, from: ModuleId) -> bool {
        let d = self.def(def);
        self.vis_allows(d.vis, d.module, from) || matches!(d.kind, DefKind::Module(_))
    }

    /// Whether an item of `owner` with visibility `vis` may be named from
    /// `from`.
    pub fn vis_allows(&self, vis: ast::Vis, owner: ModuleId, from: ModuleId) -> bool {
        let owner = self.scope_module(owner);
        let from = self.scope_module(from);
        if owner == from {
            return true;
        }
        let (Some(o), Some(f)) = (self.modules.get(owner.0 as usize), self.modules.get(from.0 as usize))
        else {
            return true;
        };
        if o.is_std || f.is_std {
            return true;
        }
        match vis {
            ast::Vis::Pub => true,
            ast::Vis::PubPackage => o.package() == f.package(),
            ast::Vis::Private => false,
        }
    }

    /// `name` in module `m` as code in `from` sees it through the module
    /// (`m.name`, `from m import name`): the item, and whether `from` may
    /// name it there. A name an import bound in `m` is seen only when the
    /// import is `pub` (a re-export).
    pub fn member(&self, m: ModuleId, name: &str, from: ModuleId) -> Option<(DefId, bool)> {
        let data = self.modules.get(m.0 as usize)?;
        let d = *data.items.get(name)?;
        let ok = match data.imported.get(name) {
            Some(vis) => self.vis_allows(*vis, m, from),
            None => self.visible_from(d, from),
        };
        Some((d, ok))
    }

    /// The diagnostic for naming `d` through module `m` where it is not
    /// visible (09): private to its module, `pub(package)` in another
    /// package, or imported into `m` without being re-exported.
    pub fn private_item(&self, d: DefId, m: ModuleId, span: Span) -> sisu_diag::Diagnostic {
        use sisu_diag::{Diagnostic, msg};
        let def = self.def(d);
        let name = def.name.clone();
        let data = &self.modules[m.0 as usize];
        if data.imported.contains_key(&name) || self.scope_module(def.module) != self.scope_module(m) {
            // Seen through a module that imported it.
            if data.items.get(&name) == Some(&d) && data.imported.contains_key(&name) {
                return Diagnostic::new(msg::RESOLVE_NOT_REEXPORTED, span)
                    .with("name", name)
                    .with("module", self.module_path(m))
                    .with("origin", self.module_path(def.module));
            }
        }
        let owner = def.module;
        match def.vis {
            ast::Vis::PubPackage => {
                let package = self.modules[owner.0 as usize].package();
                let pname = self.packages.get(package as usize).map(|p| p.name.clone()).unwrap_or_default();
                Diagnostic::new(msg::RESOLVE_PACKAGE_ITEM, span)
                    .with("name", name)
                    .with("module", self.module_path(owner))
                    .with("package", pname)
            }
            _ => Diagnostic::new(msg::RESOLVE_PRIVATE_ITEM, span)
                .with("name", name)
                .with("module", self.module_path(owner)),
        }
    }

    /// The module's path as an import writes it, for messages: `std.json`
    /// for a std module, `weather.parser` for a module of a package.
    pub fn module_path(&self, m: ModuleId) -> String {
        match self.modules.get(m.0 as usize) {
            Some(data) if !data.path.is_empty() => data.path.join("."),
            Some(data) => data.name.clone(),
            None => String::new(),
        }
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

/// The candidate closest to a misspelt name, if one is close enough to be
/// the likely intent: an edit distance of at most a third of the name's
/// length, and at least one; the first by spelling of equals.
pub fn closest(n: &str, mut names: Vec<String>) -> Option<String> {
    let limit = (n.chars().count() / 3).max(1);
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|c| c != n)
        .map(|c| (edit_distance(n, &c), c))
        .filter(|(d, _)| *d <= limit)
        .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, c)| c)
}

/// The edit distance between two names, by characters, where swapping
/// two adjacent characters counts as one edit (optimal string alignment).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
        }
    }
    d[a.len()][b.len()]
}
