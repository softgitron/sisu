//! The interceptor and the fake of a program's own trait (06,
//! "Interceptors and trace context"; 17, "Fakes"; D-075): for a trait a
//! program declares, `cap.Wrap(inner=x, hook=h)` implements it by showing
//! each call to the interceptor and forwarding it to `x`, and
//! `cap.Fake[Trait]` is a recording fake of it.
//!
//! In Sisu both are compile-time code over the trait's methods (the
//! reflection behind `Fake[T]`); how compile-time code writes a method's
//! signature waits for Q-27, so stage 0 writes them itself, from the
//! trait's declaration: source text in a module whose scope is the
//! trait's, declared and checked like any other code. A module gets them
//! only when its text names `Wrap` or `Fake`, and a trait only when each
//! of its methods can be forwarded: a `self` or `mut self` receiver, no
//! type parameters of its own, and no borrow or `Self` in its signature;
//! the trait itself has no type parameters, associated types or
//! supertraits.

use crate::collect::Resolver;
use crate::program::ModuleId;
use sisu_diag::TextRange;
use sisu_syntax::ast::{self, ItemKind, Mode, ParamKind, TypeKind};

/// A method of a trait, as the shims write it.
struct Method {
    name: String,
    /// The receiver and parameters as written: `mut self, x: int`.
    params: String,
    /// `-> T ! E` as written, or empty.
    ret: String,
    /// The result type as written (`T ! E`), when it is not `()`.
    result: Option<String>,
    /// The parameters' names, and whether each is passed `mut`.
    args: Vec<(String, bool)>,
}

fn slice(source: &str, r: TextRange) -> &str {
    source.get(r.start as usize..r.end as usize).unwrap_or("")
}

/// Whether the type mentions a borrow or `Self`.
fn borrows(t: &ast::TypeExpr) -> bool {
    match &t.kind {
        TypeKind::Ref { .. } => true,
        TypeKind::Path { segments, args } => {
            segments.iter().any(|s| s.text == "Self") || args.iter().any(borrows)
        }
        TypeKind::Option(x) | TypeKind::Dyn(x) => borrows(x),
        TypeKind::Result { ok, err } => {
            ok.as_deref().is_some_and(borrows) || err.as_deref().is_some_and(borrows)
        }
        TypeKind::Union(xs) => xs.iter().any(borrows),
        TypeKind::Tuple(xs) => xs.iter().any(|(_, x)| borrows(x)),
        TypeKind::Fn { params, ret, .. } => params.iter().any(borrows) || borrows(ret),
        TypeKind::ConstArg(_) => false,
        TypeKind::Unsupported(_) | TypeKind::Error => true,
    }
}

/// The methods of `t`, when every one can be forwarded.
fn methods(source: &str, t: &ast::TraitDef) -> Option<Vec<Method>> {
    if !t.generics.is_empty() || !t.supertraits.is_empty() || !t.assoc_types.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for m in t.methods.iter().filter(|m| !m.is_comptime) {
        if !m.generics.is_empty() || !m.where_.is_empty() || !m.from.is_empty() || m.header_broken {
            return None;
        }
        let (first, rest) = m.params.split_first()?;
        if !matches!(first.kind, ParamKind::SelfParam) || matches!(first.mode, Mode::Own) {
            return None;
        }
        let mut args = Vec::new();
        for p in rest {
            let ParamKind::Named { name, ty, .. } = &p.kind else { return None };
            if borrows(ty) {
                return None;
            }
            args.push((name.text.clone(), matches!(p.mode, Mode::Mut)));
        }
        if m.ret.as_ref().is_some_and(borrows) {
            return None;
        }
        let params: Vec<&str> = m.params.iter().map(|p| slice(source, p.range)).collect();
        let result = m.ret.as_ref().map(|r| slice(source, r.range).to_string()).filter(|r| r != "()");
        out.push(Method {
            name: m.name.text.clone(),
            params: params.join(", "),
            ret: m.ret.as_ref().map(|r| format!(" -> {}", slice(source, r.range))).unwrap_or_default(),
            result,
            args,
        });
    }
    if out.is_empty() { None } else { Some(out) }
}

/// `"{a:?}, {b:?}"`: the arguments of a call as an interceptor and a fake
/// show them.
fn shown_args(m: &Method) -> String {
    let parts: Vec<String> = m.args.iter().map(|(n, _)| format!("{{{n}:?}}")).collect();
    parts.join(", ")
}

/// `x, mut y`: the arguments of the forwarded call.
fn passed(m: &Method) -> String {
    let parts: Vec<String> =
        m.args.iter().map(|(n, by_mut)| if *by_mut { format!("mut {n}") } else { n.clone() }).collect();
    parts.join(", ")
}

/// The impl of `trait_name` for `cap.Wrap`.
fn wrap_impl(trait_name: &str, ms: &[Method]) -> String {
    let mut s =
        format!("impl[__T: {trait_name}, __I: cap.Interceptor] {trait_name} for cap.Wrap[__T, __I]:\n");
    for m in ms {
        s.push_str(&format!("    def {}({}){}:\n", m.name, m.params, m.ret));
        s.push_str(&format!(
            "        let __call = cap.Call(capability=\"{trait_name}\".to_string(), method=\"{}\".to_string(), args=\"{}\")\n",
            m.name,
            shown_args(m)
        ));
        s.push_str("        self.hook.before(__call.copy())\n");
        match &m.result {
            Some(_) => {
                s.push_str(&format!("        let __result = self.inner.{}({})\n", m.name, passed(m)));
                s.push_str("        self.hook.after(__call, \"{__result:?}\")\n");
                s.push_str("        return __result\n");
            }
            None => {
                s.push_str(&format!("        self.inner.{}({})\n", m.name, passed(m)));
                s.push_str("        self.hook.after(__call, \"()\")\n");
            }
        }
    }
    s
}

/// The fake of `trait_name`: a struct, its own methods, and its impl of
/// the trait.
fn fake(trait_name: &str, ms: &[Method]) -> String {
    let ty = format!("__Fake_{trait_name}");
    let mut s = format!(
        "## A recording fake of `{trait_name}` (17): `cap.Fake[{trait_name}]`. Its copies share\n\
         ## what it recorded.\n@derive(Copy)\nstruct {ty}:\n    __log: Cap[Mutex[list[String]]]\n"
    );
    for m in ms {
        if let Some(r) = &m.result {
            s.push_str(&format!("    __on_{}: Cap[Mutex[list[{r}]]]\n", m.name));
        }
    }
    let mut fields = vec!["__log=Cap(Mutex(list()))".to_string()];
    for m in ms.iter().filter(|m| m.result.is_some()) {
        fields.push(format!("__on_{}=Cap(Mutex(list()))", m.name));
    }
    s.push_str(&format!("\nimpl {ty}:\n"));
    s.push_str(&format!(
        "    ## A fake of `{trait_name}` that has seen no calls and has no results queued.\n"
    ));
    s.push_str(&format!("    def new() -> {ty}:\n        return {ty}({})\n", fields.join(", ")));
    s.push_str("    ## The calls so far, in order, each written `method(arguments)`.\n");
    s.push_str("    def calls(self) -> list[String]:\n        return self.__log.get().lock().value.copy()\n");
    for m in ms {
        if let Some(r) = &m.result {
            s.push_str(&format!(
                "    ## Queues what a call of `{0}` returns; the last result queued is returned\n\
                 \x20   ## again once the others are used.\n\
                 \x20   def on_{0}(self, own result: {r}):\n\
                 \x20       self.__on_{0}.get().lock().value.push(result)\n",
                m.name
            ));
        }
    }
    s.push_str(&format!("\nimpl {trait_name} for {ty}:\n"));
    for m in ms {
        s.push_str(&format!("    def {}({}){}:\n", m.name, m.params, m.ret));
        s.push_str(&format!(
            "        self.__log.get().lock().value.push(\"{}({})\")\n",
            m.name,
            shown_args(m)
        ));
        if m.result.is_some() {
            s.push_str(&format!("        var __queued = self.__on_{}.get().lock()\n", m.name));
            s.push_str("        if __queued.value.is_empty():\n");
            s.push_str(&format!(
                "            panic(\"`cap.Fake[{trait_name}]`: `{0}` was called with no result queued (`on_{0}`)\")\n",
                m.name
            ));
            s.push_str("        if len(__queued.value) == 1:\n");
            s.push_str("            return __queued.value[0].copy()\n");
            s.push_str("        return __queued.value.remove(0)\n");
        }
    }
    s
}

impl Resolver<'_> {
    /// Declares the interceptors and fakes of the traits among `items`, a
    /// module of the program's own, when its text asks for them.
    pub(crate) fn declare_trait_shims(&mut self, module: ModuleId, items: &[ast::Item]) {
        let m = &self.prog.modules[module.0 as usize];
        if m.is_std || m.name.starts_with('<') {
            return;
        }
        let source = self.sources.get(m.file).map(|f| f.text.clone()).unwrap_or_default();
        let (wrap, fakes) = (source.contains("Wrap"), source.contains("Fake"));
        if !wrap && !fakes {
            return;
        }
        let mut text = String::new();
        let mut faked = Vec::new();
        for item in items {
            let ItemKind::Trait(t) = &item.kind else { continue };
            let Some(ms) = methods(&source, t) else { continue };
            if wrap {
                text.push('\n');
                text.push_str(&wrap_impl(&t.name.text, &ms));
            }
            if fakes {
                text.push('\n');
                text.push_str(&fake(&t.name.text, &ms));
                faked.push(t.name.text.clone());
            }
        }
        if text.is_empty() {
            return;
        }
        let Some(g) = self.declare_generated_in(module, "<shims>", &text) else { return };
        for name in faked {
            let tr = self.prog.modules[module.0 as usize].items.get(&name).copied();
            let st = self.prog.modules[g.0 as usize].items.get(&format!("__Fake_{name}")).copied();
            if let (Some(tr), Some(st)) = (tr, st) {
                self.prog.fakes.insert(tr, st);
            }
        }
    }
}
