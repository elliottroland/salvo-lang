//! The IR builder [ir-build]: from the checked program to the IR, copying
//! every answer the checker recorded (IR record §2). Nothing here infers; where
//! the checker left no answer the node is `Unsupported` and an error names it.

use std::collections::{HashMap, HashSet};

use salvo_core::check::{Checked, Coercion};
use salvo_core::program::{Program as CoreProgram, Symbols};
use salvo_core::resolve::Resolution;
use salvo_core::types::{is_proj_name, Qual, Ty};
use salvo_core::FnKey;
use salvo_syntax::ast::{self, Expr as AExpr, Item};
use salvo_syntax::Span;

use crate::ir::*;

mod body;
pub mod actors;
pub mod refs;
mod decls;

pub use body::Lower;

/// Everything the builder reads, shared by every module.
pub struct Ctx<'p> {
    pub program: &'p CoreProgram,
    pub symbols: &'p Symbols<'p>,
    pub resolution: &'p Resolution<'p>,
    pub checked: &'p Checked,
    /// Every fn declaration's dump name, program-wide (references may reach
    /// modules that are not built).
    pub names: HashMap<DeclId, String>,
}

impl<'p> Ctx<'p> {
    pub fn decl_id(&self, key: FnKey) -> DeclId {
        DeclId { module: self.program.files[key.file].module.clone(), item: key.item, sub: 0 }
    }

    pub fn fn_by_key(&self, key: FnKey) -> Option<&'p ast::FnDecl> {
        match self.program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }

    /// The `DeclId` of a declaration (any item kind) by its address.
    pub fn item_id(&self, file_idx: usize, item: usize) -> DeclId {
        DeclId { module: self.program.files[file_idx].module.clone(), item, sub: 0 }
    }

    /// Where a struct/effect/handler declaration lives, by symbol key.
    pub fn decl_of_effect(&self, key: &str) -> Option<DeclId> {
        let decl: &ast::EffectDecl = self.symbols.effects.get(key).copied()?;
        let file = *self.resolution.decl_files.get(&(decl as *const ast::EffectDecl as usize))?;
        let item = self.program.modules[file]
            .items
            .iter()
            .position(|i| matches!(i, Item::Effect(e) if std::ptr::eq(e, decl)))?;
        Some(self.item_id(file, item))
    }

    pub fn written_ty(&self, file_idx: usize, ty: &ast::Type) -> Ty {
        let _ = file_idx;
        if let Some(t) = self.checked.written_types.get(&salvo_core::check::written_type_key(ty)) {
            return erase(t);
        }
        match self.approx_keyed(ty, &HashMap::new(), 0) {
            Some(t) => erase(&t),
            None => Ty::Unknown,
        }
    }

    /// A written type the checker never lowered (a struct field only ever
    /// read through a substitution, say): approximated from the AST with
    /// every name resolved to its key and every alias expanded
    /// [type-identity], which is what the checker's lowering would give.
    fn approx_keyed(&self, t: &ast::Type, subst: &HashMap<String, Ty>, depth: usize) -> Option<Ty> {
        use salvo_core::types::Qual;
        if depth > 32 {
            return None;
        }
        match t {
            ast::Type::Literal { value, .. } => Some(Ty::Lit(value.clone())),
            ast::Type::Named { qualifiers, base } => {
                if qualifiers.is_empty() && base.args.is_empty() {
                    if let Some(ty) = subst.get(&base.name.name) {
                        return Some(ty.clone());
                    }
                }
                let args: Vec<Ty> = base.args.iter().map(|a| self.approx_keyed(a, subst, depth + 1)).collect::<Option<_>>()?;
                let key = self.checked.written_key(base).to_string();
                // A name that is no declaration and no builtin is a generic
                // parameter in scope.
                let builtin = matches!(key.as_str(), "None" | "Str" | "List" | "Array" | "Bytes" | "Addr" | "Reply" | "Pool" | "Map" | "Set" | "Deque" | "SortedSet" | "SortedMap" | "Heap");
                if args.is_empty() && !builtin && !self.symbols.key_modules.contains_key(key.as_str()) && !self.symbols.intrinsic_types.contains_key(key.as_str()) && !self.symbols.structs.contains_key(key.as_str()) && !self.symbols.type_aliases.contains_key(key.as_str()) && !self.symbols.effects.contains_key(key.as_str()) {
                    let quals: Vec<Qual> = qualifiers.iter().map(|q| Qual { effect: false, name: q.name.name.clone(), args: Vec::new() }).collect();
                    return Some(Ty::Var(key).qualify(quals));
                }
                let named = match self.symbols.type_aliases.get(key.as_str()).copied().and_then(|a| a.alias.as_ref().map(|t| (a, t))) {
                    Some((alias, target)) => {
                        let inner: HashMap<String, Ty> = alias.generics.iter().map(|g| g.name.clone()).zip(args.iter().cloned()).collect();
                        self.approx_keyed(target, &inner, depth + 1)?
                    }
                    None => Ty::Named { name: key, args },
                };
                let quals: Vec<Qual> = qualifiers.iter().map(|q| Qual { effect: false, name: q.name.name.clone(), args: Vec::new() }).collect();
                Some(named.qualify(quals))
            }
            ast::Type::QualifiedGroup { qualifiers, base, .. } => {
                let inner = self.approx_keyed(base, subst, depth + 1)?;
                let quals: Vec<Qual> = qualifiers.iter().map(|q| Qual { effect: false, name: q.name.name.clone(), args: Vec::new() }).collect();
                Some(inner.qualify(quals))
            }
            ast::Type::Union { arms, .. } => Some(Ty::union_of(arms.iter().map(|a| self.approx_keyed(a, subst, depth + 1)).collect::<Option<_>>()?)),
            ast::Type::Tuple { elems, .. } => Some(Ty::Tuple(elems.iter().map(|e| self.approx_keyed(e, subst, depth + 1)).collect::<Option<_>>()?)),
            ast::Type::Array { elem, .. } => Some(Ty::Array(Box::new(self.approx_keyed(elem, subst, depth + 1)?))),
            ast::Type::Nullable { inner, .. } => Some(Ty::union_of(vec![self.approx_keyed(inner, subst, depth + 1)?, Ty::none()])),
            ast::Type::Fn { params, effects, ret, .. } => {
                // [fn-effects] the effects a fn value performs lead its type.
                let effects: Vec<Ty> = effects
                    .iter()
                    .flatten()
                    .filter_map(|e| match e {
                        ast::EffectRef::Effect(r) | ast::EffectRef::AnyEffect(r) => Some(Ty::Named {
                            name: self.checked.written_key(r).to_string(),
                            args: r.args.iter().filter_map(|a| self.approx_keyed(a, subst, depth + 1)).collect(),
                        }),
                        _ => None,
                    })
                    .collect();
                Some(Ty::Fn {
                    params: params.iter().map(|p| self.approx_keyed(p, subst, depth + 1)).collect::<Option<_>>()?,
                    ret: Box::new(self.approx_keyed(ret, subst, depth + 1)?),
                    contract: None,
                    effects,
                })
            }
        }
    }

    pub fn ty_of(&self, file_idx: usize, span: Span) -> Option<Ty> {
        self.checked.ty_of(file_idx, span).map(erase)
    }

    fn compute_names(program: &CoreProgram) -> HashMap<DeclId, String> {
        let mut out = HashMap::new();
        for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
            let _ = file_idx;
            let mut by_name: HashMap<&str, usize> = HashMap::new();
            for item in &module.items {
                if let Item::Fn(f) = item {
                    *by_name.entry(f.name.name.as_str()).or_default() += 1;
                }
            }
            let prefix = file.module.0.join(".");
            for (item, decl) in module.items.iter().enumerate() {
                let name = match decl {
                    Item::Fn(f) => {
                        if by_name.get(f.name.name.as_str()).copied().unwrap_or(0) > 1 {
                            let params: Vec<String> = f
                                .params
                                .iter()
                                .filter(|p| !p.implicit)
                                .map(|p| p.ty.to_string())
                                .collect();
                            format!("{prefix}::{}({})", f.name.name, params.join(", "))
                        } else {
                            format!("{prefix}::{}", f.name.name)
                        }
                    }
                    Item::Struct(s) => format!("{prefix}::{}", s.name.name),
                    Item::Effect(e) => format!("{prefix}::{}", e.name.name),
                    Item::Handler(h) => format!("{prefix}::{}", h.name.name),
                    Item::Type(t) => format!("{prefix}::{}", t.name.name),
                    Item::Qualifier(q) => format!("{prefix}::{}", q.name.name),
                    _ => continue,
                };
                out.insert(DeclId { module: file.module.clone(), item, sub: 0 }, name);
                if let Item::Qualifier(q) = decl {
                    for (i, f) in q.fns.iter().enumerate() {
                        out.insert(DeclId { module: file.module.clone(), item, sub: i as u16 + 1 }, format!("{prefix}::{}.{}", q.name.name, f.name.name));
                    }
                }
            }
        }
        out
    }
}

impl Program {
    /// The dump spelling of a declaration reference.
    pub fn ref_name(&self, id: &DeclId) -> String {
        self.names.get(id).cloned().unwrap_or_else(|| format!("{}::#{}.{}", id.module.0.join("."), id.item, id.sub))
    }
}

/// [ir-types] Qualifiers are erased from every type the IR carries, except
/// `Mut`, the one a backend may represent as a different type.
pub fn erase(ty: &Ty) -> Ty {
    // [type-literal] literals collapse into their base first, so a union of
    // literals of one base is that base.
    if salvo_core::literal::mentions_lit(ty) {
        return erase(&salvo_core::literal::collapse_ty(ty));
    }
    match ty {
        Ty::Qualified { quals, base } => {
            // [type-none-unit] A *tagged* `None` (`Ok None`) is a value arm —
            // the unit — not the absent arm, so its tag survives erasure as
            // the mark of that.
            if base.is_none_ty() && !quals.is_empty() {
                return Ty::Qualified { quals: vec![Qual::plain(&quals[0].name, Vec::new())], base: Box::new(Ty::none()) };
            }
            let base = erase(base);
            // [ir-types] Two qualifiers survive: `Mut`, which a backend may
            // represent differently, and the projection flavour [proj-type]
            // [ref-handle] — `proj` or `ref` — which a backend with
            // ownership renders as a borrow (its source list kept).
            let mut kept = Vec::new();
            if quals.iter().any(|q| q.name == "Mut") {
                kept.push(Qual::plain("Mut", Vec::new()));
            }
            if let Some(p) = quals.iter().find(|q| is_proj_name(&q.name)) {
                kept.push(p.clone());
            }
            base.qualify(kept)
        }
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(erase).collect() },
        // [union-arm-identity] Arms are positional over the runtime union:
        // two arms that erase alike (`Err Str | Thrown Str`) keep their tag,
        // so a rewrap or a narrowing can still tell them apart.
        Ty::Union(arms) => {
            let erased: Vec<Ty> = arms.iter().map(erase).collect();
            Ty::Union(
                arms.iter()
                    .zip(&erased)
                    .map(|(orig, e)| {
                        let clash = erased.iter().filter(|x| *x == e).count() > 1;
                        match orig {
                            Ty::Qualified { quals, base } if clash && !quals.is_empty() => {
                                Ty::Qualified { quals: vec![Qual::plain(&quals[0].name, Vec::new())], base: Box::new(erase(base)) }
                            }
                            _ => e.clone(),
                        }
                    })
                    .collect(),
            )
        }
        Ty::Tuple(es) => Ty::Tuple(es.iter().map(erase).collect()),
        Ty::Array(e) => Ty::Array(Box::new(erase(e))),
        Ty::Fn { params, ret, contract, effects } => Ty::Fn {
            params: params.iter().map(erase).collect(),
            ret: Box::new(erase(ret)),
            contract: contract.clone(),
            effects: effects.iter().map(erase).collect(),
        },
        // A literal type is its base at run time [type-literal].
        Ty::Lit(l) => Ty::named(l.base()),
        other => other.clone(),
    }
}

/// Builds the IR of every module in `modules` (all of them when `None`).
pub fn build_program<'p>(
    program: &'p CoreProgram,
    symbols: &'p Symbols<'p>,
    resolution: &'p Resolution<'p>,
    checked: &'p Checked,
    modules: Option<&HashSet<&salvo_core::ModulePath>>,
) -> (Program, Vec<String>) {
    let ctx = Ctx { program, symbols, resolution, checked, names: Ctx::compute_names(program) };
    let mut out = Program { modules: Vec::new(), names: HashMap::new() };
    let mut errors = Vec::new();
    for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
        if let Some(wanted) = modules {
            if !wanted.contains(&file.module) {
                continue;
            }
        }
        let (m, errs) = decls::build_module(&ctx, file_idx, file, module);
        out.modules.push(m);
        errors.extend(errs.into_iter().map(|e| format!("{}: {e}", file.name)));
    }
    let generated = actors::generate(&ctx, &mut out);
    // [ref-handle] Every `ref` names its container's type.
    refs::annotate(&mut out);
    out.names = ctx.names;
    out.names.extend(generated);
    (out, errors)
}

/// The span-keyed coercion a slot recorded for an expression, applied as IR
/// nodes around it [ir-coerce].
pub fn apply_coercion(c: &Coercion, value: Expr, span: Span) -> Expr {
    match c {
        Coercion::WrapUnion { target, arm, inner } => {
            let value = match inner {
                Some(i) => apply_coercion(i, value, span),
                None => value,
            };
            Expr { ty: erase(target), span, kind: ExprKind::MakeUnion { arm: *arm, value: Box::new(value) } }
        }
        Coercion::Rewrap { from, to } => {
            Expr { ty: erase(to), span, kind: ExprKind::Rewrap { from: erase(from), value: Box::new(value) } }
        }
        Coercion::WrapOption { target } => {
            // The absent arm is `MakeNone`; a present value is itself, typed
            // at the optional.
            Expr { ty: erase(target), span, kind: ExprKind::Present { value: Box::new(value) } }
        }
        Coercion::NoneUnit => Expr { ty: Ty::none(), span, kind: ExprKind::Unit },
        Coercion::DropMut { from, then } => {
            let dropped = Expr {
                ty: erase(from).strip_quals().clone(),
                span,
                kind: ExprKind::DropMut { value: Box::new(value) },
            };
            match then {
                Some(t) => apply_coercion(t, dropped, span),
                None => dropped,
            }
        }
    }
}

pub(crate) fn unsupported(ty: Ty, span: Span, what: impl Into<String>) -> Expr {
    Expr { ty, span, kind: ExprKind::Unsupported(what.into()) }
}

pub(crate) fn stmt_is_divergent(e: &AExpr) -> bool {
    matches!(e, AExpr::Return { .. } | AExpr::Break { .. } | AExpr::Continue { .. })
}

pub(crate) use salvo_syntax::ast::Module as AModule;
