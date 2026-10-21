//! Builds the AST from the lossless tree. Total: every node kind has a
//! case, and `ERROR` nodes or missing children become `Error` or `None`.

use super::*;
use crate::kinds::SyntaxKind;
use crate::kinds::SyntaxKind::*;
use crate::tree::{Element, TokenId, Tree};

/// Builds the AST of a whole file.
pub fn module(tree: &Tree) -> Module {
    let b = B { t: tree };
    let root = tree.root();
    let mut items = Vec::new();
    let mut docs = Vec::new();
    let mut broken_names = Vec::new();
    // A script's top-level statements are its `main` (10), not code that
    // failed to parse as a declaration.
    let script = tree.text().starts_with("#!");
    for e in tree.children(root) {
        match e {
            Element::Node(n) => {
                if !script && !Tree::is_item_kind(tree.kind(*n)) && tree.kind(*n) != EOF {
                    for t in
                        tree.preorder(*n).into_iter().flat_map(|m| tree.child_tokens(m).collect::<Vec<_>>())
                    {
                        if tree.token_kind(t) == IDENT {
                            broken_names.push(tree.token_text(t).to_string());
                        }
                    }
                }
                b.push_item(*n, &mut items)
            }
            Element::Token(t) => {
                if tree.token_kind(*t) == MODULE_DOC_COMMENT {
                    docs.push(doc_text(tree.token_text(*t), "##!"));
                }
            }
        }
    }
    broken_names.sort();
    broken_names.dedup();
    Module { items, docs, broken_names }
}

fn doc_text(text: &str, prefix: &str) -> String {
    let s = text.strip_prefix(prefix).unwrap_or(text);
    s.strip_prefix(' ').unwrap_or(s).to_string()
}

fn is_type_node(k: SyntaxKind) -> bool {
    matches!(
        k,
        PATH_TYPE
            | OPTION_TYPE
            | RESULT_TYPE
            | ERROR_SET_TYPE
            | TUPLE_TYPE
            | PAREN_TYPE
            | FN_TYPE
            | REF_TYPE
            | DYN_TYPE
            | IMPL_TYPE
            | PTR_TYPE
            | CONST_ARG
    )
}

fn is_expr_node(k: SyntaxKind) -> bool {
    matches!(
        k,
        LITERAL
            | STRING_EXPR
            | NAME_REF
            | PAREN_EXPR
            | TUPLE_EXPR
            | LIST_EXPR
            | DICT_EXPR
            | SET_EXPR
            | COMPREHENSION
            | BIN_EXPR
            | COMPARE_EXPR
            | PREFIX_EXPR
            | CALL_EXPR
            | FIELD_EXPR
            | OPT_FIELD_EXPR
            | INDEX_EXPR
            | TRY_EXPR
            | CLOSURE_EXPR
            | IF_EXPR
            | MATCH_EXPR
    )
}

fn is_pattern_node(k: SyntaxKind) -> bool {
    matches!(
        k,
        IDENT_PAT
            | WILDCARD_PAT
            | LITERAL_PAT
            | CTOR_PAT
            | TUPLE_PAT
            | PAREN_PAT
            | LIST_PAT
            | STAR_PAT
            | REST_PAT
            | OR_PAT
    )
}

fn is_item_node(k: SyntaxKind) -> bool {
    matches!(
        k,
        FN_DEF
            | STRUCT_DEF
            | ENUM_DEF
            | ERROR_DEF
            | IMPL_DEF
            | TRAIT_DEF
            | IMPORT_ITEM
            | FROM_IMPORT
            | TEST_DEF
            | SUITE_DEF
            | BENCH_DEF
            | CONST_DEF
            | TYPE_ALIAS
            | EXTERN_BLOCK
    )
}

struct B<'t> {
    t: &'t Tree,
}

impl B<'_> {
    fn kind(&self, n: NodeId) -> SyntaxKind {
        self.t.kind(n)
    }

    fn range(&self, n: NodeId) -> TextRange {
        self.t.range(n)
    }

    fn nodes(&self, n: NodeId) -> Vec<NodeId> {
        self.t.child_nodes(n).collect()
    }

    fn tokens(&self, n: NodeId) -> Vec<TokenId> {
        self.t.child_tokens(n).collect()
    }

    fn text(&self, t: TokenId) -> &str {
        self.t.token_text(t)
    }

    fn name(&self, t: TokenId) -> Name {
        Name { text: self.text(t).to_string(), range: self.t.token_range(t) }
    }

    fn has(&self, n: NodeId, k: SyntaxKind) -> bool {
        self.t.has_token(n, k)
    }

    fn child(&self, n: NodeId, k: SyntaxKind) -> Option<NodeId> {
        self.t.first_child(n, k)
    }

    /// The first IDENT token after the first token of kind `after`.
    fn ident_after(&self, n: NodeId, after: SyntaxKind) -> Option<Name> {
        let mut seen = false;
        for t in self.tokens(n) {
            let k = self.t.token_kind(t);
            if seen && (k == IDENT || k.is_keyword()) {
                return Some(self.name(t));
            }
            if k == after {
                seen = true;
            }
        }
        None
    }

    fn missing_name(&self, n: NodeId) -> Name {
        Name { text: String::new(), range: TextRange::empty(self.range(n).end) }
    }

    // ---------------------------------------------------------------- items

    fn push_item(&self, n: NodeId, out: &mut Vec<Item>) {
        let k = self.kind(n);
        if is_item_node(k) {
            out.push(self.item(n));
        } else if k == BLOCK {
            // An unexpectedly indented block: keep what is in it.
            for c in self.nodes(n) {
                self.push_item(c, out);
            }
        } else if k == ERROR {
            out.push(Item {
                node: n,
                range: self.range(n),
                attrs: Vec::new(),
                vis: Vis::Private,
                docs: Vec::new(),
                kind: ItemKind::Error,
            });
        } else {
            let range = self.range(n);
            for s in self.stmts_of(n) {
                out.push(Item {
                    node: n,
                    range,
                    attrs: Vec::new(),
                    vis: Vis::Private,
                    docs: Vec::new(),
                    kind: ItemKind::Stmt(s),
                });
            }
        }
    }

    fn docs_before(&self, n: NodeId) -> Vec<String> {
        let Some(parent) = self.t.parent(n) else { return Vec::new() };
        let kids = self.t.children(parent);
        let Some(pos) = kids.iter().position(|e| *e == Element::Node(n)) else { return Vec::new() };
        let mut docs = Vec::new();
        for e in kids[..pos].iter().rev() {
            match e {
                Element::Token(t) => match self.t.token_kind(*t) {
                    DOC_COMMENT => docs.push(doc_text(self.text(*t), "##")),
                    WHITESPACE | LINE_BREAK | NEWLINE | DEDENT | INDENT => {}
                    _ => break,
                },
                Element::Node(_) => break,
            }
        }
        docs.reverse();
        docs
    }

    fn attrs(&self, n: NodeId) -> Vec<Attr> {
        self.nodes(n)
            .into_iter()
            .filter(|&c| self.kind(c) == ATTRIBUTE)
            .map(|a| {
                let name = self
                    .tokens(a)
                    .into_iter()
                    .filter(|&t| self.t.token_kind(t) != AT && self.t.token_kind(t) != NEWLINE)
                    .take_while(|&t| self.t.token_kind(t) != L_PAREN)
                    .map(|t| self.text(t).to_string())
                    .collect::<Vec<_>>()
                    .join("");
                let args = self.child(a, ARG_LIST).map(|l| self.args(l)).unwrap_or_default();
                Attr { name, args, range: self.range(a) }
            })
            .collect()
    }

    fn vis(&self, n: NodeId) -> Vis {
        match self.child(n, VISIBILITY) {
            None => Vis::Private,
            Some(v) => {
                if self.tokens(v).iter().any(|&t| self.text(t) == "package") {
                    Vis::PubPackage
                } else {
                    Vis::Pub
                }
            }
        }
    }

    fn item(&self, n: NodeId) -> Item {
        let kind = match self.kind(n) {
            FN_DEF => ItemKind::Fn(self.fn_def(n)),
            STRUCT_DEF => ItemKind::Struct(self.struct_def(n)),
            ENUM_DEF => ItemKind::Enum(self.enum_def(n)),
            ERROR_DEF => ItemKind::ErrorSet(self.enum_def(n)),
            IMPL_DEF => ItemKind::Impl(self.impl_def(n)),
            TRAIT_DEF => ItemKind::Trait(self.trait_def(n)),
            IMPORT_ITEM => ItemKind::Import(self.import(n)),
            FROM_IMPORT => ItemKind::FromImport(self.import_from(n)),
            TEST_DEF => ItemKind::Test(self.test_def(n)),
            BENCH_DEF => ItemKind::Bench(self.test_def(n)),
            SUITE_DEF => ItemKind::Suite(self.suite_def(n)),
            CONST_DEF => ItemKind::Const(self.const_def(n)),
            TYPE_ALIAS => ItemKind::TypeAlias(TypeAlias {
                name: self.ident_after(n, TYPE_KW).unwrap_or(self.missing_name(n)),
                generics: self.generics(n),
                ty: self
                    .nodes(n)
                    .into_iter()
                    .find(|&c| is_type_node(self.kind(c)))
                    .map(|c| self.type_expr(c)),
                refinement: self.has(n, WHERE_KW),
                predicate: self
                    .nodes(n)
                    .into_iter()
                    .find(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR)
                    .map(|c| self.expr(c)),
            }),
            EXTERN_BLOCK => {
                let mut items = Vec::new();
                if let Some(b) = self.child(n, BLOCK) {
                    for c in self.nodes(b) {
                        if is_item_node(self.kind(c)) {
                            items.push(self.item(c));
                        }
                    }
                }
                ItemKind::Extern(items)
            }
            _ => ItemKind::Error,
        };
        Item {
            node: n,
            range: self.range(n),
            attrs: self.attrs(n),
            vis: self.vis(n),
            docs: self.docs_before(n),
            kind,
        }
    }

    fn generics(&self, n: NodeId) -> Vec<GenericParam> {
        let Some(g) = self.child(n, GENERIC_PARAMS) else { return Vec::new() };
        self.nodes(g)
            .into_iter()
            .filter(|&p| self.kind(p) == GENERIC_PARAM)
            .map(|p| {
                // The types after `:` are bounds; the one after `=` is the default.
                let mut bounds = Vec::new();
                let mut default = None;
                let mut after_eq = false;
                for e in self.t.children(p) {
                    match e {
                        Element::Token(t) if self.t.token_kind(*t) == EQ => after_eq = true,
                        Element::Node(c) if is_type_node(self.kind(*c)) || self.kind(*c) == ERROR => {
                            if after_eq {
                                default = Some(self.type_expr(*c));
                            } else {
                                bounds.push(self.type_expr(*c));
                            }
                        }
                        _ => {}
                    }
                }
                GenericParam {
                    name: self
                        .tokens(p)
                        .into_iter()
                        .find(|&t| self.t.token_kind(t) == IDENT)
                        .map(|t| self.name(t))
                        .unwrap_or(self.missing_name(p)),
                    bounds,
                    default,
                    is_const: self.has(p, CONST_KW),
                }
            })
            .collect()
    }

    /// The predicates of a `where` clause: `T: A + B, C.Item: D`.
    fn where_preds(&self, n: NodeId) -> Vec<WherePred> {
        let Some(w) = self.child(n, WHERE_CLAUSE) else { return Vec::new() };
        let mut out: Vec<WherePred> = Vec::new();
        let mut subject: Option<TypeExpr> = None;
        let mut bounds = Vec::new();
        let mut after_colon = false;
        let mut start = self.range(w).start;
        let finish = |subject: &mut Option<TypeExpr>,
                      bounds: &mut Vec<TypeExpr>,
                      out: &mut Vec<WherePred>,
                      start: u32| {
            if let Some(ty) = subject.take() {
                let end = bounds.last().map(|b: &TypeExpr| b.range.end).unwrap_or(ty.range.end);
                out.push(WherePred { ty, bounds: std::mem::take(bounds), range: TextRange::new(start, end) });
            }
        };
        for e in self.t.children(w) {
            match e {
                Element::Token(t) => match self.t.token_kind(*t) {
                    COLON => after_colon = true,
                    COMMA => {
                        finish(&mut subject, &mut bounds, &mut out, start);
                        after_colon = false;
                    }
                    _ => {}
                },
                Element::Node(c) if is_type_node(self.kind(*c)) || self.kind(*c) == ERROR => {
                    let ty = self.type_expr(*c);
                    if after_colon {
                        bounds.push(ty);
                    } else {
                        start = ty.range.start;
                        subject = Some(ty);
                    }
                }
                _ => {}
            }
        }
        finish(&mut subject, &mut bounds, &mut out, start);
        out
    }

    /// `type Item` or `type Item = T` inside a trait or an impl.
    fn assoc_type(&self, n: NodeId) -> AssocTypeDef {
        AssocTypeDef {
            name: self.ident_after(n, TYPE_KW).unwrap_or(self.missing_name(n)),
            ty: self.nodes(n).into_iter().find(|&c| is_type_node(self.kind(c))).map(|c| self.type_expr(c)),
            range: self.range(n),
        }
    }

    fn trait_def(&self, n: NodeId) -> TraitDef {
        let supertraits = self
            .nodes(n)
            .into_iter()
            .filter(|&c| is_type_node(self.kind(c)))
            .map(|c| self.type_expr(c))
            .collect();
        let mut methods = Vec::new();
        let mut assoc_types = Vec::new();
        if let Some(b) = self.child(n, BLOCK) {
            for c in self.nodes(b) {
                match self.kind(c) {
                    FN_DEF => methods.push(self.fn_def(c)),
                    TYPE_ALIAS => assoc_types.push(self.assoc_type(c)),
                    _ => {}
                }
            }
        }
        TraitDef {
            name: self.ident_after(n, TRAIT_KW).unwrap_or(self.missing_name(n)),
            generics: self.generics(n),
            supertraits,
            methods,
            assoc_types,
        }
    }

    fn fn_def(&self, n: NodeId) -> FnDef {
        let tokens = self.tokens(n);
        let is_pure = tokens.iter().any(|&t| self.t.token_kind(t) == IDENT && self.text(t) == "pure")
            && tokens.iter().position(|&t| self.text(t) == "pure")
                < tokens.iter().position(|&t| self.t.token_kind(t) == DEF_KW);
        let name = self.ident_after(n, DEF_KW).unwrap_or(self.missing_name(n));
        let params = self.child(n, PARAM_LIST).map(|p| self.params(p)).unwrap_or_default();
        let (ret, from) = match self.child(n, RET_TYPE) {
            Some(r) => {
                let ty =
                    self.nodes(r).into_iter().find(|&c| is_type_node(self.kind(c)) || self.kind(c) == ERROR);
                let mut from = Vec::new();
                let mut seen = false;
                for t in self.tokens(r) {
                    if self.t.token_kind(t) == FROM_KW {
                        seen = true;
                    } else if seen && matches!(self.t.token_kind(t), IDENT | SELF_KW) {
                        from.push(self.name(t));
                    }
                }
                (ty.map(|t| self.type_expr(t)), from)
            }
            None => (None, Vec::new()),
        };
        FnDef {
            node: n,
            range: self.range(n),
            name,
            is_pure,
            is_comptime: self.has(n, COMPTIME_KW),
            generics: self.generics(n),
            params,
            ret,
            from,
            where_: self.where_preds(n),
            body: self.child(n, BLOCK).map(|b| std::sync::Arc::new(self.block(b))),
            attrs: self.attrs(n),
            header_broken: self.error_outside_block(n),
            body_broken: self.child(n, BLOCK).is_some_and(|b| self.has_error(b)),
            vis: self.vis(n),
        }
    }

    /// Whether a node has an `ERROR` node anywhere inside it.
    fn has_error(&self, n: NodeId) -> bool {
        self.nodes(n).into_iter().any(|c| self.kind(c) == ERROR || self.has_error(c))
    }

    /// Whether a node has an `ERROR` node anywhere except inside its block.
    fn error_outside_block(&self, n: NodeId) -> bool {
        self.nodes(n).into_iter().any(|c| match self.kind(c) {
            ERROR => true,
            BLOCK => false,
            _ => self.error_outside_block(c),
        })
    }

    fn params(&self, list: NodeId) -> Vec<Param> {
        self.nodes(list)
            .into_iter()
            .filter(|&p| self.kind(p) == PARAM)
            .map(|p| {
                let toks = self.tokens(p);
                let mode = if self.has(p, MUT_KW) {
                    Mode::Mut
                } else if self.has(p, OWN_KW) {
                    Mode::Own
                } else {
                    Mode::Borrow
                };
                let ty =
                    self.nodes(p).into_iter().find(|&c| is_type_node(self.kind(c)) || self.kind(c) == ERROR);
                let default =
                    self.nodes(p).into_iter().find(|&c| is_expr_node(self.kind(c))).map(|c| self.expr(c));
                let kind = if self.has(p, SELF_KW) {
                    ParamKind::SelfParam
                } else if self.has(p, STAR) {
                    match toks.iter().find(|&&t| self.t.token_kind(t) == IDENT) {
                        Some(&t) => {
                            ParamKind::Variadic { name: self.name(t), ty: ty.map(|t| self.type_expr(t)) }
                        }
                        None => ParamKind::KeywordOnly,
                    }
                } else if self.has(p, SLASH) {
                    ParamKind::PositionalOnly
                } else {
                    match toks.iter().find(|&&t| {
                        self.t.token_kind(t) == IDENT
                            || self.t.token_kind(t).is_keyword()
                                && !matches!(self.t.token_kind(t), MUT_KW | OWN_KW)
                    }) {
                        Some(&t) => ParamKind::Named {
                            name: self.name(t),
                            ty: ty.map(|t| self.type_expr(t)).unwrap_or(TypeExpr {
                                node: p,
                                range: self.range(p),
                                kind: TypeKind::Error,
                            }),
                            default,
                        },
                        None => ParamKind::Error,
                    }
                };
                Param { node: p, range: self.range(p), mode, kind }
            })
            .collect()
    }

    fn struct_def(&self, n: NodeId) -> StructDef {
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut invariants = Vec::new();
        let mut unsupported = Vec::new();
        let mut consts = Vec::new();
        if let Some(b) = self.child(n, BLOCK) {
            for c in self.nodes(b) {
                match self.kind(c) {
                    FIELD_DEF => {
                        let name = self
                            .tokens(c)
                            .into_iter()
                            .find(|&t| self.t.token_kind(t) == IDENT)
                            .map(|t| self.name(t));
                        let ty = self
                            .nodes(c)
                            .into_iter()
                            .find(|&x| is_type_node(self.kind(x)) || self.kind(x) == ERROR);
                        let default = self
                            .nodes(c)
                            .into_iter()
                            .find(|&x| is_expr_node(self.kind(x)))
                            .map(|x| self.expr(x));
                        fields.push(FieldDef {
                            node: c,
                            range: self.range(c),
                            name: name.unwrap_or(self.missing_name(c)),
                            ty: ty.map(|t| self.type_expr(t)).unwrap_or(TypeExpr {
                                node: c,
                                range: self.range(c),
                                kind: TypeKind::Error,
                            }),
                            default,
                            vis: self.vis(c),
                        });
                    }
                    FN_DEF => methods.push(self.fn_def(c)),
                    CONST_DEF => consts.push(self.const_def(c)),
                    PASS_STMT | ATTRIBUTE => {}
                    CONTRACT_STMT => {
                        let contract = self.contract(c);
                        if contract.kind == ContractKind::Invariant {
                            invariants.push(contract);
                        } else {
                            unsupported.push((self.range(c), "a contract"));
                        }
                    }
                    // The parser reported any other line already.
                    _ => {}
                }
            }
        }
        StructDef {
            name: self.ident_after(n, STRUCT_KW).unwrap_or(self.missing_name(n)),
            generics: self.generics(n),
            is_view: self.has(n, VIEW_KW),
            fields,
            methods,
            invariants,
            consts,
            unsupported,
        }
    }

    fn enum_def(&self, n: NodeId) -> EnumDef {
        let keyword = if self.kind(n) == ERROR_DEF { ERROR_KW } else { ENUM_KW };
        let mut variants = Vec::new();
        let mut methods = Vec::new();
        let mut open = false;
        if let Some(b) = self.child(n, BLOCK) {
            for c in self.nodes(b) {
                match self.kind(c) {
                    VARIANT_DEF => {
                        let name = self
                            .tokens(c)
                            .into_iter()
                            .find(|&t| self.t.token_kind(t) == IDENT)
                            .map(|t| self.name(t));
                        let fields = self
                            .nodes(c)
                            .into_iter()
                            .filter(|&f| self.kind(f) == VARIANT_FIELD)
                            .map(|f| {
                                let fname = if self.has(f, COLON) {
                                    self.tokens(f)
                                        .into_iter()
                                        .find(|&t| self.t.token_kind(t) == IDENT)
                                        .map(|t| self.name(t))
                                } else {
                                    None
                                };
                                let ty = self
                                    .nodes(f)
                                    .into_iter()
                                    .find(|&x| is_type_node(self.kind(x)) || self.kind(x) == ERROR);
                                VariantField {
                                    name: fname,
                                    ty: ty.map(|t| self.type_expr(t)).unwrap_or(TypeExpr {
                                        node: f,
                                        range: self.range(f),
                                        kind: TypeKind::Error,
                                    }),
                                    range: self.range(f),
                                }
                            })
                            .collect();
                        variants.push(VariantDef {
                            node: c,
                            range: self.range(c),
                            name: name.unwrap_or(self.missing_name(c)),
                            fields,
                        });
                    }
                    OPEN_MARKER => open = true,
                    FN_DEF => methods.push(self.fn_def(c)),
                    _ => {}
                }
            }
        }
        EnumDef {
            name: self.ident_after(n, keyword).unwrap_or(self.missing_name(n)),
            generics: self.generics(n),
            variants,
            methods,
            open,
        }
    }

    fn impl_def(&self, n: NodeId) -> ImplDef {
        let types: Vec<NodeId> = self.nodes(n).into_iter().filter(|&c| is_type_node(self.kind(c))).collect();
        let has_for = self.has(n, FOR_KW);
        let (trait_ty, self_ty) = if has_for && types.len() >= 2 {
            (Some(self.type_expr(types[0])), self.type_expr(types[1]))
        } else {
            (
                None,
                types.first().map(|&t| self.type_expr(t)).unwrap_or(TypeExpr {
                    node: n,
                    range: self.range(n),
                    kind: TypeKind::Error,
                }),
            )
        };
        let mut methods = Vec::new();
        let mut assoc_types = Vec::new();
        let mut consts = Vec::new();
        if let Some(b) = self.child(n, BLOCK) {
            for c in self.nodes(b) {
                match self.kind(c) {
                    FN_DEF => methods.push(self.fn_def(c)),
                    TYPE_ALIAS => assoc_types.push(self.assoc_type(c)),
                    CONST_DEF => consts.push(self.const_def(c)),
                    _ => {}
                }
            }
        }
        ImplDef { generics: self.generics(n), trait_ty, self_ty, methods, assoc_types, consts }
    }

    fn path_names(&self, n: NodeId) -> Vec<Name> {
        match self.child(n, MODULE_PATH) {
            Some(p) => self
                .tokens(p)
                .into_iter()
                .filter(|&t| self.t.token_kind(t) != DOT)
                .map(|t| self.name(t))
                .collect(),
            None => Vec::new(),
        }
    }

    fn import(&self, n: NodeId) -> Import {
        let c_header = self.child(n, STRING_EXPR).map(|s| plain_text(&self.string_parts(s)));
        Import { path: self.path_names(n), alias: self.ident_after(n, AS_KW), c_header }
    }

    fn import_from(&self, n: NodeId) -> FromImport {
        let names = self
            .nodes(n)
            .into_iter()
            .filter(|&c| self.kind(c) == IMPORT_NAME)
            .filter_map(|c| {
                let toks: Vec<TokenId> =
                    self.tokens(c).into_iter().filter(|&t| self.t.token_kind(t) == IDENT).collect();
                let first = toks.first().map(|&t| self.name(t))?;
                Some((first, toks.get(1).map(|&t| self.name(t))))
            })
            .collect();
        FromImport { path: self.path_names(n), names }
    }

    fn test_def(&self, n: NodeId) -> TestDef {
        let (name, name_range) = match self.child(n, STRING_EXPR) {
            Some(s) => (plain_text(&self.string_parts(s)), self.range(s)),
            None => (String::new(), TextRange::empty(self.range(n).start)),
        };
        let cases = if self.has(n, FOR_KW) {
            let pat =
                self.nodes(n).into_iter().find(|&c| is_pattern_node(self.kind(c))).map(|c| self.pattern(c));
            let iter = self
                .nodes(n)
                .into_iter()
                .rfind(|&c| is_expr_node(self.kind(c)) && self.kind(c) != STRING_EXPR)
                .map(|c| self.expr(c));
            match (pat, iter) {
                (Some(p), Some(e)) => Some((p, e)),
                _ => None,
            }
        } else {
            None
        };
        TestDef {
            name,
            name_range,
            cases,
            params: self.child(n, PARAM_LIST).map(|p| self.params(p)).unwrap_or_default(),
            body: self.child(n, BLOCK).map(|b| std::sync::Arc::new(self.block(b))),
        }
    }

    fn suite_def(&self, n: NodeId) -> SuiteDef {
        let name = self.child(n, STRING_EXPR).map(|s| plain_text(&self.string_parts(s))).unwrap_or_default();
        let mut items = Vec::new();
        if let Some(b) = self.child(n, BLOCK) {
            for c in self.nodes(b) {
                self.push_item(c, &mut items);
            }
        }
        SuiteDef { name, items }
    }

    fn const_def(&self, n: NodeId) -> ConstDef {
        ConstDef {
            name: self.ident_after(n, CONST_KW).unwrap_or(self.missing_name(n)),
            ty: self.nodes(n).into_iter().find(|&c| is_type_node(self.kind(c))).map(|c| self.type_expr(c)),
            value: self.nodes(n).into_iter().find(|&c| is_expr_node(self.kind(c))).map(|c| self.expr(c)),
        }
    }

    // ---------------------------------------------------------------- statements

    fn block(&self, n: NodeId) -> Block {
        let mut stmts = Vec::new();
        for c in self.nodes(n) {
            stmts.extend(self.stmts_of(c));
        }
        Block { node: n, range: self.range(n), stmts }
    }

    /// The statements a child node of a block stands for: usually one, all
    /// of them for an unexpectedly indented block.
    fn stmts_of(&self, n: NodeId) -> Vec<Stmt> {
        let k = self.kind(n);
        if k == BLOCK {
            return self.block(n).stmts;
        }
        vec![self.stmt(n)]
    }

    fn stmt(&self, n: NodeId) -> Stmt {
        let range = self.range(n);
        let kind = match self.kind(n) {
            LET_STMT => self.let_stmt(n),
            ASSIGN_STMT => {
                let exprs: Vec<NodeId> = self
                    .nodes(n)
                    .into_iter()
                    .filter(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR)
                    .collect();
                let op = self.tokens(n).into_iter().find_map(|t| {
                    aug_op(self.t.token_kind(t)).map(Some).or(if self.t.token_kind(t) == EQ {
                        Some(None)
                    } else {
                        None
                    })
                });
                StmtKind::Assign {
                    target: exprs.first().map(|&e| self.expr(e)).unwrap_or(self.error_expr(n)),
                    op: op.flatten(),
                    value: exprs.get(1).map(|&e| self.expr(e)).unwrap_or(self.error_expr(n)),
                }
            }
            EXPR_STMT => StmtKind::Expr(self.first_expr(n)),
            RETURN_STMT => {
                if let Some(i) = self.child(n, IMPL_DEF) {
                    StmtKind::ReturnImpl(Box::new(self.impl_def(i)))
                } else {
                    StmtKind::Return(
                        self.nodes(n)
                            .into_iter()
                            .find(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR)
                            .map(|c| self.expr(c)),
                    )
                }
            }
            BREAK_STMT => StmtKind::Break,
            CONTINUE_STMT => StmtKind::Continue,
            PASS_STMT => StmtKind::Pass,
            IF_STMT => {
                let mut branches = Vec::new();
                if let (Some(c), Some(b)) = (self.child(n, COND), self.child(n, BLOCK)) {
                    branches.push((self.cond(c), self.block(b)));
                } else if let Some(c) = self.child(n, COND) {
                    branches.push((
                        self.cond(c),
                        Block { node: n, range: TextRange::empty(range.end), stmts: Vec::new() },
                    ));
                }
                let mut else_block = None;
                for c in self.nodes(n) {
                    match self.kind(c) {
                        ELIF_CLAUSE => {
                            let cond = self
                                .child(c, COND)
                                .map(|x| self.cond(x))
                                .unwrap_or(Cond { range: self.range(c), parts: Vec::new() });
                            let body = self.child(c, BLOCK).map(|b| self.block(b)).unwrap_or(Block {
                                node: c,
                                range: self.range(c),
                                stmts: Vec::new(),
                            });
                            branches.push((cond, body));
                        }
                        ELSE_CLAUSE => else_block = self.child(c, BLOCK).map(|b| self.block(b)),
                        _ => {}
                    }
                }
                StmtKind::If { branches, else_block }
            }
            WHILE_STMT => StmtKind::While {
                cond: self.child(n, COND).map(|c| self.cond(c)).unwrap_or(Cond { range, parts: Vec::new() }),
                body: self.child(n, BLOCK).map(|b| self.block(b)).unwrap_or(Block {
                    node: n,
                    range,
                    stmts: Vec::new(),
                }),
            },
            FOR_STMT => {
                let mut mode = Mode::Borrow;
                let mut after_in = false;
                for t in self.tokens(n) {
                    match self.t.token_kind(t) {
                        IN_KW => after_in = true,
                        MUT_KW if after_in => mode = Mode::Mut,
                        OWN_KW if after_in => mode = Mode::Own,
                        _ => {}
                    }
                }
                StmtKind::For {
                    pat: self
                        .nodes(n)
                        .into_iter()
                        .find(|&c| is_pattern_node(self.kind(c)))
                        .map(|c| self.pattern(c))
                        .unwrap_or(Pattern { node: n, range, kind: PatKind::Error }),
                    mode,
                    iter: self.first_expr(n),
                    body: self.child(n, BLOCK).map(|b| self.block(b)).unwrap_or(Block {
                        node: n,
                        range,
                        stmts: Vec::new(),
                    }),
                }
            }
            ASSERT_STMT => {
                let exprs: Vec<NodeId> = self
                    .nodes(n)
                    .into_iter()
                    .filter(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR)
                    .collect();
                StmtKind::Assert {
                    cond: exprs.first().map(|&e| self.expr(e)).unwrap_or(self.error_expr(n)),
                    message: exprs.get(1).map(|&e| self.expr(e)),
                }
            }
            EXPECT_STMT => StmtKind::Expect(self.first_expr(n)),
            DEFER_STMT => match self.child(n, BLOCK) {
                Some(b) => StmtKind::Defer(self.block(b)),
                None => {
                    let stmts = self.nodes(n).into_iter().map(|c| self.stmt(c)).collect();
                    StmtKind::Defer(Block { node: n, range, stmts })
                }
            },
            WITH_STMT => StmtKind::With {
                value: self.first_expr(n),
                name: self.ident_after(n, AS_KW).unwrap_or(self.missing_name(n)),
                body: self.child(n, BLOCK).map(|b| self.block(b)).unwrap_or(Block {
                    node: n,
                    range,
                    stmts: Vec::new(),
                }),
            },
            UNSAFE_BLOCK => StmtKind::Unsupported("an `unsafe` block"),
            SELECT_STMT => StmtKind::Select(
                self.child(n, MATCH_ARM_LIST)
                    .map(|l| {
                        self.nodes(l)
                            .into_iter()
                            .filter(|&c| self.kind(c) == MATCH_ARM)
                            .map(|c| self.select_arm(c))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            YIELD_STMT => StmtKind::Yield(
                self.nodes(n).into_iter().find(|&c| is_expr_node(self.kind(c))).map(|e| self.expr(e)),
            ),
            CONTRACT_STMT => StmtKind::Contract(self.contract(n)),
            COMPTIME_STMT => match self.nodes(n).into_iter().next() {
                Some(c) => StmtKind::Comptime(Box::new(self.stmt(c))),
                None => StmtKind::Error,
            },
            k if is_item_node(k) => StmtKind::Item(Box::new(self.item(n))),
            _ => StmtKind::Error,
        };
        Stmt { node: n, range, kind }
    }

    fn let_stmt(&self, n: NodeId) -> StmtKind {
        // Children in order: pattern, then type after `:`, then value after `=`.
        let mut role = 0;
        let mut pat = None;
        let mut ty = None;
        let mut init = None;
        let mut else_ = None;
        for e in self.t.children(n) {
            match e {
                Element::Token(t) => match self.t.token_kind(*t) {
                    COLON => role = 1,
                    EQ | WALRUS => role = 2,
                    _ => {}
                },
                Element::Node(c) => {
                    let k = self.kind(*c);
                    if k == LET_ELSE {
                        else_ = Some(self.let_else(*c));
                    } else if role == 0 && pat.is_none() {
                        pat = Some(self.pattern(*c));
                    } else if role == 1 && ty.is_none() {
                        ty = Some(self.type_expr(*c));
                    } else if role == 2 && init.is_none() {
                        init = Some(self.expr(*c));
                    }
                }
            }
        }
        StmtKind::Let {
            mutable: self.has(n, VAR_KW),
            pat: pat.unwrap_or(Pattern { node: n, range: self.range(n), kind: PatKind::Error }),
            ty,
            init,
            else_,
        }
    }

    fn let_else(&self, n: NodeId) -> LetElse {
        let body = if let Some(b) = self.child(n, BLOCK) {
            ElseBody::Block(self.block(b))
        } else if let Some(s) = self
            .nodes(n)
            .into_iter()
            .find(|&c| matches!(self.kind(c), RETURN_STMT | BREAK_STMT | CONTINUE_STMT))
        {
            ElseBody::Diverge(Box::new(self.stmt(s)))
        } else {
            ElseBody::Expr(self.first_expr(n))
        };
        LetElse { range: self.range(n), binding: self.ident_after(n, AS_KW), body }
    }

    fn cond(&self, n: NodeId) -> Cond {
        let parts = self
            .nodes(n)
            .into_iter()
            .filter_map(|c| match self.kind(c) {
                LET_COND => Some(CondPart::Let {
                    pat: self
                        .nodes(c)
                        .into_iter()
                        .find(|&x| is_pattern_node(self.kind(x)))
                        .map(|x| self.pattern(x))
                        .unwrap_or(Pattern { node: c, range: self.range(c), kind: PatKind::Error }),
                    value: self.first_expr(c),
                }),
                k if is_expr_node(k) || k == ERROR => Some(CondPart::Expr(self.expr(c))),
                _ => None,
            })
            .collect();
        Cond { range: self.range(n), parts }
    }

    // ---------------------------------------------------------------- expressions

    fn error_expr(&self, n: NodeId) -> Expr {
        Expr { node: n, range: TextRange::empty(self.range(n).end), kind: ExprKind::Error }
    }

    /// A contract clause: its keyword and expression.
    fn contract(&self, n: NodeId) -> Contract {
        let kind = match self.tokens(n).first().map(|&t| self.text(t)) {
            Some("requires") => ContractKind::Requires,
            Some("ensures") => ContractKind::Ensures,
            Some("invariant") => ContractKind::Invariant,
            _ => ContractKind::Decreases,
        };
        let range = self.range(n);
        // The clause without the line break that ends it.
        let end = self
            .tokens(n)
            .into_iter()
            .rev()
            .find(|&t| !matches!(self.t.token_kind(t), NEWLINE | DEDENT | INDENT | WHITESPACE | LINE_BREAK))
            .map(|t| self.t.token_range(t).end)
            .unwrap_or(range.end);
        let expr = self.first_expr(n);
        Contract { kind, expr: expr.clone(), range: TextRange::new(range.start, end.max(expr.range.end)) }
    }

    fn first_expr(&self, n: NodeId) -> Expr {
        self.nodes(n)
            .into_iter()
            .find(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR)
            .map(|c| self.expr(c))
            .unwrap_or(self.error_expr(n))
    }

    fn exprs(&self, n: NodeId) -> Vec<Expr> {
        self.nodes(n)
            .into_iter()
            .filter(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR)
            .map(|c| self.expr(c))
            .collect()
    }

    fn args(&self, list: NodeId) -> Vec<Arg> {
        self.nodes(list)
            .into_iter()
            .filter(|&a| self.kind(a) == ARG)
            .map(|a| {
                let toks = self.tokens(a);
                let name = if toks.len() >= 2 && self.t.token_kind(toks[1]) == EQ {
                    Some(self.name(toks[0]))
                } else {
                    None
                };
                Arg {
                    node: a,
                    range: self.range(a),
                    name,
                    mutable: self.has(a, MUT_KW),
                    value: self.first_expr(a),
                }
            })
            .collect()
    }

    fn expr(&self, n: NodeId) -> Expr {
        let range = self.range(n);
        let kind = match self.kind(n) {
            LITERAL => match self.tokens(n).first() {
                Some(&t) => self.literal(t),
                None => ExprKind::Error,
            },
            STRING_EXPR => ExprKind::Str(self.string_parts(n)),
            NAME_REF => match self.tokens(n).first() {
                Some(&t) => match self.t.token_kind(t) {
                    SELF_KW => ExprKind::SelfValue,
                    SELF_TYPE_KW => ExprKind::SelfType,
                    _ => ExprKind::Name(self.text(t).to_string()),
                },
                None => ExprKind::Error,
            },
            PAREN_EXPR => ExprKind::Paren(Box::new(self.first_expr(n))),
            TUPLE_EXPR => ExprKind::Tuple(self.exprs(n)),
            LIST_EXPR => ExprKind::List(self.exprs(n)),
            SET_EXPR => ExprKind::Set(self.exprs(n)),
            DICT_EXPR => ExprKind::Dict(
                self.nodes(n)
                    .into_iter()
                    .filter(|&c| self.kind(c) == DICT_ENTRY)
                    .map(|c| {
                        let kv = self.exprs(c);
                        let k = kv.first().cloned().unwrap_or(self.error_expr(c));
                        let v = kv.get(1).cloned().unwrap_or(self.error_expr(c));
                        (k, v)
                    })
                    .collect(),
            ),
            COMPREHENSION => self.comprehension(n),
            BIN_EXPR => {
                let operands = self.exprs(n);
                let op = self
                    .tokens(n)
                    .into_iter()
                    .find_map(|t| bin_op(self.t.token_kind(t)))
                    .unwrap_or(BinOp::Add);
                let lhs = operands.first().cloned().unwrap_or(self.error_expr(n));
                let rhs = operands.get(1).cloned().unwrap_or(self.error_expr(n));
                ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
            }
            COMPARE_EXPR => self.compare(n),
            PREFIX_EXPR => {
                let inner = Box::new(self.first_expr(n));
                let toks = self.tokens(n);
                match toks.first().map(|&t| self.t.token_kind(t)) {
                    Some(MINUS) => ExprKind::Unary { op: UnOp::Neg, expr: inner },
                    Some(TILDE) => ExprKind::Unary { op: UnOp::BitNot, expr: inner },
                    Some(NOT_KW) => ExprKind::Unary { op: UnOp::Not, expr: inner },
                    Some(REF_KW) => ExprKind::Ref { mutable: self.has(n, MUT_KW), expr: inner },
                    Some(CARET) => ExprKind::FromEnd(inner),
                    Some(COMPTIME_KW) => ExprKind::Comptime(inner),
                    Some(UNSAFE_KW) => ExprKind::Unsupported("`unsafe`"),
                    _ => ExprKind::Error,
                }
            }
            CALL_EXPR => {
                let callee = self.first_expr(n);
                let args = self.child(n, ARG_LIST).map(|l| self.args(l)).unwrap_or_default();
                let trailing = self.child(n, DO_BLOCK).map(|d| Box::new(self.do_block(d)));
                ExprKind::Call { callee: Box::new(callee), args, trailing }
            }
            FIELD_EXPR => {
                if self.has(n, L_BRACKET) {
                    let mut es = self.exprs(n).into_iter();
                    let base = es.next().unwrap_or(self.error_expr(n));
                    let field = es.next().unwrap_or(self.error_expr(n));
                    ExprKind::FieldOf { base: Box::new(base), field: Box::new(field) }
                } else {
                    let base = self.first_expr(n);
                    let name = self
                        .tokens(n)
                        .into_iter()
                        .rfind(|&t| self.t.token_kind(t) != DOT)
                        .map(|t| self.name(t))
                        .unwrap_or(self.missing_name(n));
                    ExprKind::Field { base: Box::new(base), name }
                }
            }
            OPT_FIELD_EXPR => {
                let base = self.first_expr(n);
                let name = self.ident_after(n, QUESTION_DOT).unwrap_or(self.missing_name(n));
                ExprKind::OptField { base: Box::new(base), name }
            }
            INDEX_EXPR => {
                let mut kids = self.nodes(n).into_iter();
                let base = kids.next().map(|c| self.expr(c)).unwrap_or(self.error_expr(n));
                let args = kids
                    .map(|c| {
                        if self.kind(c) == SLICE {
                            let mut lo = None;
                            let mut hi = None;
                            let mut after = false;
                            for e in self.t.children(c) {
                                match e {
                                    Element::Token(t) if self.t.token_kind(*t) == COLON => after = true,
                                    Element::Node(x) => {
                                        if after {
                                            hi = Some(self.expr(*x));
                                        } else {
                                            lo = Some(self.expr(*x));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            IndexArg::Slice { lo, hi }
                        } else {
                            IndexArg::Expr(self.expr(c))
                        }
                    })
                    .collect();
                ExprKind::Index { base: Box::new(base), args }
            }
            TRY_EXPR => ExprKind::Try(Box::new(self.first_expr(n))),
            CLOSURE_EXPR => {
                let params =
                    self.child(n, CLOSURE_PARAMS).map(|p| self.closure_params(p)).unwrap_or_default();
                let body =
                    self.nodes(n).into_iter().rfind(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR);
                ExprKind::Closure(Box::new(Closure {
                    node: n,
                    range,
                    own: self.has(n, OWN_KW),
                    params,
                    implicit_param: false,
                    body: ClosureBody::Expr(body.map(|b| self.expr(b)).unwrap_or(self.error_expr(n))),
                }))
            }
            IF_EXPR => {
                let parts = self.exprs(n);
                let mut branches = Vec::new();
                let mut i = 0;
                while i + 1 < parts.len() {
                    branches.push((parts[i].clone(), parts[i + 1].clone()));
                    i += 2;
                }
                let else_ = if parts.len() % 2 == 1 && self.has(n, ELSE_KW) {
                    parts.last().cloned().map(Box::new)
                } else {
                    None
                };
                ExprKind::If { branches, else_ }
            }
            MATCH_EXPR => {
                let scrutinee = self.first_expr(n);
                let arms = self.child(n, MATCH_ARM_LIST).map(|l| self.arms(l)).unwrap_or_default();
                ExprKind::Match { scrutinee: Box::new(scrutinee), arms }
            }
            _ => ExprKind::Error,
        };
        Expr { node: n, range, kind }
    }

    fn compare(&self, n: NodeId) -> ExprKind {
        let mut operands = Vec::new();
        let mut ops = Vec::new();
        let mut pending_not = false;
        for e in self.t.children(n) {
            match e {
                Element::Node(c) => operands.push(self.expr(*c)),
                Element::Token(t) => {
                    let k = self.t.token_kind(*t);
                    let op = match k {
                        EQ_EQ => Some(CmpOp::Eq),
                        BANG_EQ => Some(CmpOp::Ne),
                        LT => Some(CmpOp::Lt),
                        LT_EQ => Some(CmpOp::Le),
                        GT => Some(CmpOp::Gt),
                        GT_EQ => Some(CmpOp::Ge),
                        IN_KW if pending_not => Some(CmpOp::NotIn),
                        IN_KW => Some(CmpOp::In),
                        NOT_KW => {
                            pending_not = true;
                            None
                        }
                        _ => None,
                    };
                    if let Some(op) = op {
                        pending_not = false;
                        ops.push(op);
                    }
                }
            }
        }
        let mut it = operands.into_iter();
        let first = it.next().unwrap_or(self.error_expr(n));
        let rest: Vec<(CmpOp, Expr)> =
            ops.into_iter().map(|op| (op, it.next().unwrap_or(self.error_expr(n)))).collect();
        ExprKind::Compare { first: Box::new(first), rest }
    }

    fn comprehension(&self, n: NodeId) -> ExprKind {
        let open = self.tokens(n).first().map(|&t| self.t.token_kind(t));
        let mut elem = None;
        let mut value = None;
        let mut clauses = Vec::new();
        let mut dict = false;
        for c in self.nodes(n) {
            match self.kind(c) {
                DICT_ENTRY => {
                    dict = true;
                    let kv = self.exprs(c);
                    elem = kv.first().cloned();
                    value = kv.get(1).cloned();
                }
                COMP_FOR => {
                    let pat = self
                        .nodes(c)
                        .into_iter()
                        .find(|&x| is_pattern_node(self.kind(x)))
                        .map(|x| self.pattern(x));
                    clauses.push(CompClause::For {
                        pat: pat.unwrap_or(Pattern { node: c, range: self.range(c), kind: PatKind::Error }),
                        iter: self.first_expr(c),
                    });
                }
                COMP_IF => clauses.push(CompClause::If(self.first_expr(c))),
                k if elem.is_none() && (is_expr_node(k) || k == ERROR) => elem = Some(self.expr(c)),
                _ => {}
            }
        }
        let kind = match open {
            Some(L_BRACKET) => CompKind::List,
            Some(L_BRACE) if dict => CompKind::Dict,
            Some(L_BRACE) => CompKind::Set,
            _ => CompKind::Gen,
        };
        ExprKind::Comprehension {
            kind,
            elem: Box::new(elem.unwrap_or(self.error_expr(n))),
            value: value.map(Box::new),
            clauses,
        }
    }

    fn closure_params(&self, n: NodeId) -> Vec<ClosureParam> {
        self.nodes(n)
            .into_iter()
            .filter(|&c| self.kind(c) == CLOSURE_PARAM)
            .map(|c| ClosureParam {
                name: self
                    .tokens(c)
                    .into_iter()
                    .find(|&t| self.t.token_kind(t) == IDENT)
                    .map(|t| self.name(t))
                    .unwrap_or(self.missing_name(c)),
                ty: self
                    .nodes(c)
                    .into_iter()
                    .find(|&x| is_type_node(self.kind(x)))
                    .map(|x| self.type_expr(x)),
            })
            .collect()
    }

    fn do_block(&self, n: NodeId) -> Closure {
        let params = self.child(n, CLOSURE_PARAMS).map(|p| self.closure_params(p));
        let body = self.child(n, BLOCK).map(|b| self.block(b)).unwrap_or(Block {
            node: n,
            range: self.range(n),
            stmts: Vec::new(),
        });
        Closure {
            node: n,
            range: self.range(n),
            own: self.has(n, OWN_KW),
            implicit_param: params.is_none(),
            params: params.unwrap_or_default(),
            body: ClosureBody::Block(body),
        }
    }

    /// The arms of a `MATCH_ARM_LIST`, with those made at compile time.
    fn arms(&self, l: NodeId) -> Vec<Arm> {
        self.nodes(l)
            .into_iter()
            .filter_map(|a| match self.kind(a) {
                MATCH_ARM => Some(self.arm(a)),
                COMPTIME_STMT => self.arm_gen(a),
                _ => None,
            })
            .collect()
    }

    /// `comptime for p in e:` or `comptime if e:` with arms (08).
    fn arm_gen(&self, n: NodeId) -> Option<Arm> {
        let inner = self.nodes(n).into_iter().next()?;
        let arms_in = |c: NodeId| self.child(c, MATCH_ARM_LIST).map(|l| self.arms(l)).unwrap_or_default();
        let made = match self.kind(inner) {
            FOR_STMT => ArmGen::For {
                pat: self
                    .nodes(inner)
                    .into_iter()
                    .find(|&c| is_pattern_node(self.kind(c)))
                    .map(|c| self.pattern(c))
                    .unwrap_or(Pattern { node: inner, range: self.range(inner), kind: PatKind::Error }),
                iter: self.first_expr(inner),
                arms: arms_in(inner),
            },
            IF_STMT => ArmGen::If {
                cond: self.child(inner, COND).map(|c| self.first_expr(c)).unwrap_or(self.error_expr(inner)),
                then: arms_in(inner),
                else_: self.child(inner, ELSE_CLAUSE).map(arms_in).unwrap_or_default(),
            },
            _ => return None,
        };
        let range = self.range(n);
        Some(Arm {
            node: n,
            range,
            pat: Pattern { node: n, range, kind: PatKind::Wildcard },
            guard: None,
            body: ArmBody::Block(Block { node: n, range, stmts: Vec::new() }),
            generated: Some(Box::new(made)),
        })
    }

    fn select_arm(&self, n: NodeId) -> SelectArm {
        let range = self.range(n);
        let pat = self
            .nodes(n)
            .into_iter()
            .find(|&c| is_pattern_node(self.kind(c)) || self.kind(c) == ERROR)
            .map(|c| self.pattern(c))
            .unwrap_or(Pattern { node: n, range, kind: PatKind::Error });
        // The operation is the expression after `=`.
        let mut after_eq = false;
        let mut op = None;
        for e in self.t.children(n) {
            match e {
                Element::Token(t) if self.t.token_kind(*t) == EQ => after_eq = true,
                Element::Node(c) if after_eq && op.is_none() && is_expr_node(self.kind(*c)) => {
                    op = Some(self.expr(*c));
                }
                _ => {}
            }
        }
        SelectArm {
            node: n,
            range,
            pat,
            op: op.unwrap_or(self.error_expr(n)),
            body: self.child(n, BLOCK).map(|b| self.block(b)).unwrap_or(Block {
                node: n,
                range,
                stmts: Vec::new(),
            }),
        }
    }

    fn arm(&self, n: NodeId) -> Arm {
        let pat = self
            .nodes(n)
            .into_iter()
            .find(|&c| is_pattern_node(self.kind(c)) || self.kind(c) == ERROR)
            .map(|c| self.pattern(c));
        let guard = self.child(n, MATCH_GUARD).map(|g| self.first_expr(g));
        let body = if let Some(b) = self.child(n, BLOCK) {
            ArmBody::Block(self.block(b))
        } else {
            let e = self.nodes(n).into_iter().rfind(|&c| is_expr_node(self.kind(c)));
            ArmBody::Expr(e.map(|e| self.expr(e)).unwrap_or(self.error_expr(n)))
        };
        Arm {
            node: n,
            range: self.range(n),
            pat: pat.unwrap_or(Pattern { node: n, range: self.range(n), kind: PatKind::Error }),
            guard,
            body,
            generated: None,
        }
    }

    fn literal(&self, t: TokenId) -> ExprKind {
        let text = self.text(t);
        match self.t.token_kind(t) {
            INT_NUMBER => {
                let (digits, suffix) = split_suffix(text, false);
                ExprKind::Int { text: text.to_string(), value: parse_int(&digits), suffix }
            }
            FLOAT_NUMBER => {
                let (digits, suffix) = split_suffix(text, true);
                ExprKind::Float {
                    text: text.to_string(),
                    value: digits.replace('_', "").parse().unwrap_or(0.0),
                    suffix,
                }
            }
            CHAR_LIT => {
                let inner = text.strip_prefix('\'').unwrap_or(text);
                let inner = inner.strip_suffix('\'').unwrap_or(inner);
                ExprKind::Char(decode_escapes(inner).chars().next().unwrap_or('\0'))
            }
            RAW_STRING => {
                let s = text.strip_prefix('r').unwrap_or(text);
                let s = if s.starts_with("\"\"\"") {
                    s.trim_start_matches("\"\"\"").trim_end_matches("\"\"\"")
                } else {
                    s.trim_start_matches('"').trim_end_matches('"')
                };
                ExprKind::RawStr(s.to_string())
            }
            BYTE_STRING => {
                let s = text.strip_prefix('b').unwrap_or(text);
                let s = s.trim_start_matches('"').trim_end_matches('"');
                ExprKind::ByteStr(decode_byte_escapes(s))
            }
            _ => ExprKind::Error,
        }
    }

    fn string_parts(&self, n: NodeId) -> Vec<StrPart> {
        let mut parts = Vec::new();
        for e in self.t.children(n) {
            match e {
                Element::Token(t) if self.t.token_kind(*t) == STRING_TEXT => {
                    let text = decode_escapes(&self.text(*t).replace("{{", "{").replace("}}", "}"));
                    match parts.last_mut() {
                        Some(StrPart::Text(s)) => s.push_str(&text),
                        _ => parts.push(StrPart::Text(text)),
                    }
                }
                Element::Node(i) if self.kind(*i) == INTERPOLATION => {
                    let expr_node = self
                        .nodes(*i)
                        .into_iter()
                        .find(|&c| is_expr_node(self.kind(c)) || self.kind(c) == ERROR);
                    let expr = expr_node.map(|c| self.expr(c)).unwrap_or(self.error_expr(*i));
                    let debug = if self.has(*i, EQ) {
                        expr_node.map(|c| self.t.node_text(c).to_string())
                    } else {
                        None
                    };
                    let spec = self
                        .tokens(*i)
                        .into_iter()
                        .find(|&t| self.t.token_kind(t) == FORMAT_SPEC)
                        .map(|t| self.text(t).to_string());
                    parts.push(StrPart::Interp { expr, debug, spec, range: self.range(*i) });
                }
                _ => {}
            }
        }
        parts
    }

    // ---------------------------------------------------------------- types

    fn type_expr(&self, n: NodeId) -> TypeExpr {
        let range = self.range(n);
        let types: Vec<NodeId> = self
            .nodes(n)
            .into_iter()
            .filter(|&c| is_type_node(self.kind(c)) || self.kind(c) == ERROR)
            .collect();
        let kind = match self.kind(n) {
            PATH_TYPE => {
                let segments = self
                    .tokens(n)
                    .into_iter()
                    .filter(|&t| matches!(self.t.token_kind(t), IDENT | SELF_TYPE_KW | TYPE_KW))
                    .map(|t| self.name(t))
                    .collect();
                let args = self.child(n, GENERIC_ARGS).map(|g| {
                    self.nodes(g)
                        .into_iter()
                        .filter(|&c| is_type_node(self.kind(c)) || self.kind(c) == ERROR)
                        .map(|c| self.type_expr(c))
                        .collect()
                });
                TypeKind::Path { segments, args: args.unwrap_or_default() }
            }
            OPTION_TYPE => TypeKind::Option(Box::new(
                types.first().map(|&c| self.type_expr(c)).unwrap_or(self.error_type(n)),
            )),
            RESULT_TYPE => {
                let mut ok = None;
                let mut err = None;
                let mut after_bang = false;
                for e in self.t.children(n) {
                    match e {
                        Element::Token(t) if self.t.token_kind(*t) == BANG => after_bang = true,
                        Element::Node(c) if is_type_node(self.kind(*c)) || self.kind(*c) == ERROR => {
                            if after_bang {
                                err = Some(Box::new(self.type_expr(*c)));
                            } else {
                                ok = Some(Box::new(self.type_expr(*c)));
                            }
                        }
                        _ => {}
                    }
                }
                TypeKind::Result { ok, err }
            }
            ERROR_SET_TYPE => TypeKind::Union(types.iter().map(|&c| self.type_expr(c)).collect()),
            TUPLE_TYPE => TypeKind::Tuple(
                self.nodes(n)
                    .into_iter()
                    .filter_map(|c| {
                        if self.kind(c) == TUPLE_TYPE_FIELD {
                            let name = self
                                .tokens(c)
                                .into_iter()
                                .find(|&t| self.t.token_kind(t) == IDENT)
                                .map(|t| self.name(t));
                            let ty = self
                                .nodes(c)
                                .into_iter()
                                .find(|&x| is_type_node(self.kind(x)))
                                .map(|x| self.type_expr(x));
                            Some((name, ty.unwrap_or(self.error_type(c))))
                        } else if is_type_node(self.kind(c)) || self.kind(c) == ERROR {
                            Some((None, self.type_expr(c)))
                        } else {
                            None
                        }
                    })
                    .collect(),
            ),
            PAREN_TYPE => return types.first().map(|&c| self.type_expr(c)).unwrap_or(self.error_type(n)),
            FN_TYPE => {
                let mut params = Vec::new();
                let mut ret = None;
                let mut after_arrow = false;
                for e in self.t.children(n) {
                    match e {
                        Element::Token(t) if self.t.token_kind(*t) == ARROW => after_arrow = true,
                        Element::Node(c)
                            if is_type_node(self.kind(*c))
                                || self.kind(*c) == ERROR
                                || self.kind(*c) == TUPLE_TYPE_FIELD =>
                        {
                            let ty = if self.kind(*c) == TUPLE_TYPE_FIELD {
                                self.nodes(*c)
                                    .into_iter()
                                    .find(|&x| is_type_node(self.kind(x)))
                                    .map(|x| self.type_expr(x))
                                    .unwrap_or(self.error_type(*c))
                            } else {
                                self.type_expr(*c)
                            };
                            if after_arrow {
                                ret = Some(ty);
                            } else {
                                params.push(ty);
                            }
                        }
                        _ => {}
                    }
                }
                let pure = self.tokens(n).iter().any(|&t| self.text(t) == "pure");
                TypeKind::Fn { pure, params, ret: Box::new(ret.unwrap_or(self.error_type(n))) }
            }
            REF_TYPE => TypeKind::Ref {
                mutable: self.has(n, MUT_KW),
                inner: Box::new(types.first().map(|&c| self.type_expr(c)).unwrap_or(self.error_type(n))),
            },
            CONST_ARG => TypeKind::ConstArg(
                self.tokens(n)
                    .first()
                    .and_then(|&t| parse_int(&split_suffix(self.text(t), false).0))
                    .map(|v| v as i128)
                    .unwrap_or(0),
            ),
            DYN_TYPE => match types.first() {
                Some(&c) => TypeKind::Dyn(Box::new(self.type_expr(c))),
                None => TypeKind::Error,
            },
            IMPL_TYPE => TypeKind::Unsupported("`impl Trait for T` types"),
            PTR_TYPE => TypeKind::Unsupported("raw pointers"),
            _ => TypeKind::Error,
        };
        TypeExpr { node: n, range, kind }
    }

    fn error_type(&self, n: NodeId) -> TypeExpr {
        TypeExpr { node: n, range: TextRange::empty(self.range(n).end), kind: TypeKind::Error }
    }

    // ---------------------------------------------------------------- patterns

    fn pattern(&self, n: NodeId) -> Pattern {
        let range = self.range(n);
        let pats = || {
            self.nodes(n)
                .into_iter()
                .filter(|&c| is_pattern_node(self.kind(c)) || self.kind(c) == ERROR)
                .map(|c| self.pattern(c))
                .collect::<Vec<_>>()
        };
        let kind = match self.kind(n) {
            WILDCARD_PAT => PatKind::Wildcard,
            IDENT_PAT => match self.tokens(n).first() {
                Some(&t) => PatKind::Bind(self.name(t)),
                None => PatKind::Error,
            },
            LITERAL_PAT => {
                let toks = self.tokens(n);
                let negative = toks.first().is_some_and(|&t| self.t.token_kind(t) == MINUS);
                let lit = if let Some(s) = self.child(n, STRING_EXPR) {
                    Some(self.expr(s))
                } else {
                    toks.iter().find(|&&t| self.t.token_kind(t) != MINUS).map(|&t| Expr {
                        node: n,
                        range: self.t.token_range(t),
                        kind: self.literal(t),
                    })
                };
                match lit {
                    Some(lit) if negative => PatKind::Literal(Expr {
                        node: n,
                        range,
                        kind: ExprKind::Unary { op: UnOp::Neg, expr: Box::new(lit) },
                    }),
                    Some(lit) => PatKind::Literal(lit),
                    None => PatKind::Error,
                }
            }
            CTOR_PAT => {
                let path = self
                    .tokens(n)
                    .into_iter()
                    .filter(|&t| matches!(self.t.token_kind(t), IDENT | SELF_TYPE_KW))
                    .map(|t| self.name(t))
                    .collect();
                let args = self.child(n, PAT_ARGS).map(|a| {
                    self.nodes(a)
                        .into_iter()
                        .filter(|&x| self.kind(x) == PAT_ARG)
                        .map(|x| {
                            let inner = self
                                .nodes(x)
                                .into_iter()
                                .find(|&p| is_pattern_node(self.kind(p)) || self.kind(p) == ERROR);
                            let toks = self.tokens(x);
                            let keyword = toks.len() >= 2 && self.t.token_kind(toks[1]) == EQ;
                            match inner {
                                Some(p) if self.kind(p) == REST_PAT && !keyword => PatArg::Rest,
                                Some(p) if keyword => PatArg::Keyword(self.name(toks[0]), self.pattern(p)),
                                Some(p) => PatArg::Positional(self.pattern(p)),
                                None => PatArg::Positional(Pattern {
                                    node: x,
                                    range: self.range(x),
                                    kind: PatKind::Error,
                                }),
                            }
                        })
                        .collect()
                });
                PatKind::Ctor { path, args }
            }
            TUPLE_PAT => PatKind::Tuple(pats()),
            PAREN_PAT => {
                return pats().into_iter().next().unwrap_or(Pattern { node: n, range, kind: PatKind::Error });
            }
            LIST_PAT => PatKind::List(pats()),
            STAR_PAT => PatKind::Star(
                self.tokens(n).into_iter().find(|&t| self.t.token_kind(t) == IDENT).map(|t| self.name(t)),
            ),
            REST_PAT => PatKind::Rest,
            OR_PAT => PatKind::Or(pats()),
            _ => PatKind::Error,
        };
        Pattern { node: n, range, kind }
    }
}

fn aug_op(k: SyntaxKind) -> Option<BinOp> {
    Some(match k {
        PLUS_EQ => BinOp::Add,
        MINUS_EQ => BinOp::Sub,
        STAR_EQ => BinOp::Mul,
        SLASH_EQ => BinOp::Div,
        SLASH_SLASH_EQ => BinOp::FloorDiv,
        PERCENT_EQ => BinOp::Mod,
        STAR_STAR_EQ => BinOp::Pow,
        AMP_EQ => BinOp::BitAnd,
        PIPE_EQ => BinOp::BitOr,
        CARET_EQ => BinOp::BitXor,
        SHL_EQ => BinOp::Shl,
        SHR_EQ => BinOp::Shr,
        PLUS_PERCENT_EQ => BinOp::WrapAdd,
        MINUS_PERCENT_EQ => BinOp::WrapSub,
        STAR_PERCENT_EQ => BinOp::WrapMul,
        PLUS_PIPE_EQ => BinOp::SatAdd,
        MINUS_PIPE_EQ => BinOp::SatSub,
        STAR_PIPE_EQ => BinOp::SatMul,
        _ => return None,
    })
}

fn bin_op(k: SyntaxKind) -> Option<BinOp> {
    Some(match k {
        PLUS => BinOp::Add,
        MINUS => BinOp::Sub,
        STAR => BinOp::Mul,
        SLASH => BinOp::Div,
        SLASH_SLASH => BinOp::FloorDiv,
        PERCENT => BinOp::Mod,
        STAR_STAR => BinOp::Pow,
        PLUS_PERCENT => BinOp::WrapAdd,
        MINUS_PERCENT => BinOp::WrapSub,
        STAR_PERCENT => BinOp::WrapMul,
        PLUS_PIPE => BinOp::SatAdd,
        MINUS_PIPE => BinOp::SatSub,
        STAR_PIPE => BinOp::SatMul,
        SHL => BinOp::Shl,
        SHR => BinOp::Shr,
        AMP => BinOp::BitAnd,
        PIPE => BinOp::BitOr,
        CARET => BinOp::BitXor,
        AND_KW => BinOp::And,
        OR_KW => BinOp::Or,
        QUESTION_QUESTION => BinOp::Coalesce,
        _ => return None,
    })
}

const INT_SUFFIXES: &[&str] =
    &["i128", "u128", "i16", "i32", "i64", "u16", "u32", "u64", "uint", "int", "i8", "u8"];

/// Splits a number literal into digits and a type suffix.
fn split_suffix(text: &str, float: bool) -> (String, Option<String>) {
    let hex = text.len() > 2 && (text.starts_with("0x") || text.starts_with("0X"));
    let mut candidates: Vec<&str> = INT_SUFFIXES.to_vec();
    if float || !hex {
        candidates.push("f32");
        candidates.push("f64");
    }
    for s in candidates {
        if let Some(d) = text.strip_suffix(s)
            && !d.is_empty()
            && (hex || !d.ends_with(['e', 'E']))
        {
            return (d.to_string(), Some(s.to_string()));
        }
    }
    (text.to_string(), None)
}

/// The value of an integer literal without its suffix; `None` if it does
/// not fit in 128 bits.
pub fn parse_int(text: &str) -> Option<u128> {
    let t = text.replace('_', "");
    let lower = t.to_ascii_lowercase();
    if let Some(h) = lower.strip_prefix("0x") {
        u128::from_str_radix(h, 16).ok()
    } else if let Some(b) = lower.strip_prefix("0b") {
        u128::from_str_radix(b, 2).ok()
    } else if let Some(o) = lower.strip_prefix("0o") {
        u128::from_str_radix(o, 8).ok()
    } else {
        lower.parse().ok()
    }
}

/// Decodes `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\'` and `\u{...}`; an
/// unknown escape (already reported by the lexer) is kept as written.
/// The bytes of a byte string's text: its escapes, with `\xNN` for any
/// byte, and every other character as its UTF-8 bytes.
pub fn decode_byte_escapes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("\\x") {
        let hex = rest.get(i + 2..i + 4).filter(|h| h.chars().all(|d| d.is_ascii_hexdigit()));
        match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
            // `\\x` is a backslash and an `x`, not an escape: only an odd
            // run of backslashes escapes.
            Some(b) if rest[..i].chars().rev().take_while(|c| *c == '\\').count() % 2 == 0 => {
                out.extend(decode_escapes(&rest[..i]).into_bytes());
                out.push(b);
                rest = &rest[i + 4..];
            }
            _ => {
                out.extend(decode_escapes(&rest[..i + 2]).into_bytes());
                rest = &rest[i + 2..];
            }
        }
    }
    out.extend(decode_escapes(rest).into_bytes());
    out
}

pub fn decode_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') if chars.peek() == Some(&'{') => {
                chars.next();
                let mut hex = String::new();
                for h in chars.by_ref() {
                    if h == '}' {
                        break;
                    }
                    hex.push(h);
                }
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(ch) => out.push(ch),
                    None => out.push('\u{fffd}'),
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// The text of a string literal with no interpolation.
pub fn plain_text(parts: &[StrPart]) -> String {
    let mut s = String::new();
    for p in parts {
        if let StrPart::Text(t) = p {
            s.push_str(t);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_suffixes() {
        assert_eq!(split_suffix("42u8", false), ("42".into(), Some("u8".into())));
        assert_eq!(split_suffix("0xFF", false), ("0xFF".into(), None));
        assert_eq!(split_suffix("1.0f32", true), ("1.0".into(), Some("f32".into())));
        assert_eq!(parse_int("0b1010"), Some(10));
        assert_eq!(parse_int("1_000"), Some(1000));
    }

    #[test]
    fn escapes() {
        assert_eq!(decode_escapes("a\\nb\\u{41}\\\\"), "a\nbA\\");
    }
}
