//! The IR builder [ir-build]: from the checked program to the IR, copying
//! every answer the checker recorded (IR record §2). Nothing here infers; where
//! the checker left no answer the node is `Unsupported` and an error names it.

use std::collections::{HashMap, HashSet};

use crate::check::{Checked, Coercion};
use crate::program::{Program as CoreProgram, Symbols};
use crate::resolve::Resolution;
use crate::types::{is_proj_name, Qual, Ty};
use crate::DeclId;
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
    /// [fate-move-mode] The IR flow pass's decision the bodies are built
    /// from (empty on the first build: every binding borrow-mode).
    pub decision: Decision,
    /// [fate-move-mode] Every file's binding facts, by stamp: what a body's
    /// decision reads about the bindings in it.
    pub facts_by_decl: HashMap<DeclId, Vec<&'p crate::check::BindFact>>,
    /// [ir-analyze] What the program's errors broke: those declarations are
    /// built without their code, a broken file not at all.
    pub gate: &'p super::gate::Gate,
}

impl<'p> Ctx<'p> {
    /// [fate-move-mode] Whether the binding statement (or the loop, by its
    /// iterable) at `key` takes ownership: lowered as a move.
    pub fn mode(&self, key: crate::check::Key) -> bool {
        self.decision.modes.contains(&key) || self.checked.by_value_loops.contains(&key)
    }

    /// [deduce-field] Whether the binding at `key` is a virtual place.
    pub fn virtual_bind(&self, key: crate::check::Key) -> bool {
        self.decision.virtual_binds.contains_key(&key)
    }

    /// [fate-move-mode] A binding's links as the flow pass reads them, and
    /// whether it took ownership: the checker's facts (`bind_facts`), less
    /// the copies of links an ancestor binding that took ownership made
    /// (its descendant links to the owner, not to what the owner took),
    /// one per root.
    pub fn bind_links(&self, file_idx: usize, at: Span) -> Option<(bool, Vec<crate::check::FlowLink>)> {
        let hint = std::cell::Cell::new(0);
        let fact = self.checked.bind_facts.get(&self.checked.decls.key(file_idx, at, &hint))?;
        let decided = |s: Span| self.decision.decided.contains(&self.checked.decls.key(file_idx, s, &hint));
        let mut links: Vec<crate::check::FlowLink> = Vec::new();
        for l in &fact.links {
            if l.stamp != fact.stamp && decided(l.stamp) {
                continue;
            }
            if !links.iter().any(|e| e.root == l.root) {
                links.push(l.clone());
            }
        }
        Some((decided(fact.stamp), links))
    }
    pub fn fn_by_key(&self, key: DeclId) -> Option<&'p ast::FnDecl> {
        self.resolution.decls.fn_decl(&self.program.modules, key)
    }

    /// [decl-id] The id of the declaration at (`file_idx`, `item`), if it
    /// declares something.
    pub fn item_id(&self, file_idx: usize, item: usize) -> Option<DeclId> {
        self.resolution.decls.item(file_idx, item)
    }

    /// [decl-id] The item index `id` is declared at in its file (a member's
    /// container's); `usize::MAX` for a generated declaration.
    pub fn item_of(&self, id: DeclId) -> usize {
        self.resolution.decls.loc(id).map_or(usize::MAX, |l| l.item)
    }

    /// Where a struct/effect/handler declaration lives, by symbol key.
    pub fn decl_of_effect(&self, key: &str) -> Option<DeclId> {
        let decl: &ast::EffectDecl = self.symbols.effects.get(key).copied()?;
        let file = *self.resolution.decl_files.get(&(decl as *const ast::EffectDecl as usize))?;
        let item = self.program.modules[file]
            .items
            .iter()
            .position(|i| matches!(i, Item::Effect(e) if std::ptr::eq(e, decl)))?;
        self.item_id(file, item)
    }

    pub fn written_ty(&self, file_idx: usize, ty: &ast::Type) -> Ty {
        let _ = file_idx;
        if let Some(t) = self.checked.written_types.get(&crate::check::written_type_key(ty)) {
            return erase(t);
        }
        match self.approx_keyed(ty, &HashMap::new(), 0) {
            Some(t) => erase(&t),
            None => Ty::Unknown,
        }
    }

    /// [default-predicate] A written type as the checker lowered it, **before
    /// erasure**: its qualifiers intact, for the one place the IR acts on
    /// them, a default tested where it is applied. `None` when the checker
    /// never lowered it (then there is nothing it let through to test).
    pub fn written_ty_unerased(&self, ty: &ast::Type) -> Option<Ty> {
        self.checked.written_types.get(&crate::check::written_type_key(ty)).cloned()
    }

    /// A written type the checker never lowered (a struct field only ever
    /// read through a substitution, say): approximated from the AST with
    /// every name resolved to its key and every alias expanded
    /// [type-identity], which is what the checker's lowering would give.
    fn approx_keyed(&self, t: &ast::Type, subst: &HashMap<String, Ty>, depth: usize) -> Option<Ty> {
        use crate::types::Qual;
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
            ast::Type::Spread { inner, .. } => Some(Ty::spread_marker(self.approx_keyed(inner, subst, depth + 1)?)),
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

    fn compute_names(program: &CoreProgram, decls: &crate::decl_id::DeclIndex) -> HashMap<DeclId, String> {
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
                let Some(id) = decls.item(file_idx, item) else { continue };
                out.insert(id, name);
                if let Item::Qualifier(q) = decl {
                    for (i, f) in q.fns.iter().enumerate() {
                        if let Some(id) = decls.member(file_idx, item, i + 1) {
                            out.insert(id, format!("{prefix}::{}.{}", q.name.name, f.name.name));
                        }
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
        self.names.get(id).cloned().unwrap_or_else(|| format!("{}::#{}", id.module().0.join("."), id.path().join(".")))
    }
}

/// [ir-types] Qualifiers are erased from every type the IR carries, except
/// `Mut`, the one a backend may represent as a different type.
pub fn erase(ty: &Ty) -> Ty {
    // [union-spread] The `...T` marker is a signature-only fact for call-site
    // union matching; the IR and backends never see it.
    if ty.as_spread_marker().is_some() {
        return erase(&ty.strip_spread());
    }
    // [type-literal] literals collapse into their base first, so a union of
    // literals of one base is that base.
    if crate::literal::mentions_lit(ty) {
        return erase(&crate::literal::collapse_ty(ty));
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

/// Builds the IR of every module in `modules` (all of them when `None`), of
/// a program with no errors.
pub fn build_program<'p>(
    program: &'p CoreProgram,
    symbols: &'p Symbols<'p>,
    resolution: &'p Resolution<'p>,
    checked: &'p Checked,
    modules: Option<&HashSet<&crate::ModulePath>>,
) -> (Program, Vec<String>) {
    let gate = super::gate::Gate::default();
    let (ir, errors, _) = build_program_gated(program, symbols, resolution, checked, modules, &gate);
    (ir, errors)
}

/// [ir-analyze] `build_program` past a program's errors: a declaration
/// `gate` says broke is built without its code, a broken file not at all.
/// Also answers the items (by file and item index) whose build reported an
/// error.
pub fn build_program_gated<'p>(
    program: &'p CoreProgram,
    symbols: &'p Symbols<'p>,
    resolution: &'p Resolution<'p>,
    checked: &'p Checked,
    modules: Option<&HashSet<&crate::ModulePath>>,
    gate: &'p super::gate::Gate,
) -> (Program, Vec<String>, HashSet<(usize, usize)>) {
    let _t = crate::timing::phase("ir build");
    let mut facts_by_decl: HashMap<DeclId, Vec<&crate::check::BindFact>> = HashMap::new();
    for (k, fact) in &checked.bind_facts {
        facts_by_decl.entry(k.decl).or_default().push(fact);
    }
    for v in facts_by_decl.values_mut() {
        v.sort_by_key(|f| f.stamp);
    }
    let mut ctx = Ctx { program, symbols, resolution, checked, names: Ctx::compute_names(program, &resolution.decls), decision: Decision::default(), facts_by_decl, gate };
    let mut out = Program { modules: Vec::new(), names: HashMap::new(), decision: Decision::default() };
    let mut errors = Vec::new();
    let mut failed: HashSet<(usize, usize)> = HashSet::new();
    let mut built: Vec<usize> = Vec::new();
    let t = crate::timing::phase("  ir build: bodies");
    for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
        if let Some(wanted) = modules {
            if !wanted.contains(&file.module) {
                continue;
            }
        }
        if gate.files.contains(&file_idx) {
            continue;
        }
        let (m, errs, failed_items) = decls::build_module(&ctx, file_idx, file, module);
        failed.extend(failed_items.into_iter().map(|i| (file_idx, i)));
        out.modules.push(m);
        built.push(file_idx);
        errors.push(errs.into_iter().map(|e| format!("{}: {e}", file.name)).collect::<Vec<_>>());
    }
    // [fate-move-mode] The flow pass decides, over the bodies built with
    // every binding borrow-mode, which bindings take ownership (and which
    // stay virtual places [deduce-field]); a module with any is built
    // again from the decision, which is what the emitters and the flow
    // pass's own check read.
    drop(t);
    {
        let t = crate::timing::phase("  ir build: decide");
        let mut redo: Vec<usize> = Vec::new();
        for (i, m) in out.modules.iter().enumerate() {
            let file_idx = built[i];
            let mut any = false;
            for decl in &m.decls {
                any |= super::flow::decide(decl, file_idx, &checked.decls, &mut ctx.decision);
            }
            if any {
                redo.push(i);
            }
        }
        drop(t);
        let _t = crate::timing::phase("  ir build: again");
        for i in redo {
            let file_idx = built[i];
            let (m, errs, failed_items) = decls::build_module(&ctx, file_idx, &program.files[file_idx], &program.modules[file_idx]);
            failed.extend(failed_items.into_iter().map(|i| (file_idx, i)));
            out.modules[i] = m;
            errors[i] = errs.into_iter().map(|e| format!("{}: {e}", program.files[file_idx].name)).collect();
        }
    }
    let errors: Vec<String> = errors.into_iter().flatten().collect();
    let generated = actors::generate(&ctx, &mut out);
    // [ref-handle] Every `ref` names its container's type.
    refs::annotate(&mut out);
    out.names = ctx.names;
    out.names.extend(generated);
    out.decision = ctx.decision;
    (out, errors, failed)
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
        Coercion::BorrowAs { from, to } => {
            let arms = crate::types::read_arm_map(&erase(from), &erase(to))
                .unwrap_or_default()
                .into_iter()
                .map(|(from, to)| crate::ir::BorrowArm { from, to })
                .collect();
            Expr { ty: erase(to), span, kind: ExprKind::BorrowAs { value: Box::new(value), arms } }
        }
        Coercion::DropMut { from, then } => {
            // An optional drops `Mut` on its present arm [proj-opt-slot].
            let ty = match erase(from) {
                Ty::Union(arms) => {
                    let m: std::collections::HashSet<String> = ["Mut".to_string()].into_iter().collect();
                    Ty::Union(arms.into_iter().map(|a| a.remove_quals(&m)).collect())
                }
                t => t.strip_quals().clone(),
            };
            let dropped = Expr {
                ty,
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
