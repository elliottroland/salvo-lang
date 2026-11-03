//! [ir-types] The typed form's one way into the IR (the pipeline record D3.4, option
//! (a), user decision 2026-10-30): every type the builder takes from the
//! checker — an expression's, a written one's, the types a fact carries —
//! passes erasure here, and every conversion the checker recorded becomes
//! its IR nodes here (union wrapping, optional presence, rewraps, widening).
//! Nothing else in the builder calls `erase`; it asks `Ctx` for the IR type
//! of what it read.

use std::collections::HashMap;

use crate::check::{Convert, ConvertStep};
use crate::types::{is_proj_name, Qual, Ty};
use salvo_syntax::ast;
use salvo_syntax::Span;

use super::Ctx;
use crate::ir::*;

impl<'p> Ctx<'p> {
    /// [ir-types] A type a checker fact carries, as the IR holds it.
    pub(crate) fn ir_ty(&self, ty: &Ty) -> Ty {
        erase(ty)
    }

    /// [ir-types] A list of them, in order.
    pub(crate) fn ir_tys(&self, tys: &[Ty]) -> Vec<Ty> {
        tys.iter().map(erase).collect()
    }

    /// [effect-handle] The effects of the handles a node binds or yields
    /// (`Checked::handles`).
    pub(crate) fn handles_at(&self, key: crate::check::Key) -> Vec<Ty> {
        self.checked.handles.get(&key).map(|v| self.ir_tys(v)).unwrap_or_default()
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
}

/// [ir-types] Qualifiers are erased from every type the IR carries, except
/// `Mut`, the one a backend may represent as a different type.
pub fn erase(ty: &Ty) -> Ty {
    // [type-union-spread] The `...T` marker is a signature-only fact for call-site
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
        // [type-union-arm-identity] Arms are positional over the runtime union:
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

/// [ir-coerce] The conversion the checker recorded at an expression,
/// applied as IR nodes around it, step by step.
pub fn apply_convert(c: &Convert, value: Expr, span: Span) -> Expr {
    c.steps.iter().fold(value, |value, step| apply_step(step, value, span))
}

fn apply_step(step: &ConvertStep, value: Expr, span: Span) -> Expr {
    match step {
        ConvertStep::WrapUnion { target, arm } => {
            Expr { ty: erase(target), span, kind: ExprKind::MakeUnion { arm: *arm, value: Box::new(value) } }
        }
        ConvertStep::Rewrap { from, to } => {
            Expr { ty: erase(to), span, kind: ExprKind::Rewrap { from: erase(from), value: Box::new(value) } }
        }
        ConvertStep::WrapOption { target } => {
            // The absent arm is `MakeNone`; a present value is itself, typed
            // at the optional.
            Expr { ty: erase(target), span, kind: ExprKind::Present { value: Box::new(value) } }
        }
        ConvertStep::NoneUnit => Expr { ty: Ty::none(), span, kind: ExprKind::Unit },
        ConvertStep::BorrowAs { from, to } => {
            let arms = crate::types::read_arm_map(&erase(from), &erase(to))
                .unwrap_or_default()
                .into_iter()
                .map(|(from, to)| crate::ir::BorrowArm { from, to })
                .collect();
            Expr { ty: erase(to), span, kind: ExprKind::BorrowAs { value: Box::new(value), arms } }
        }
        // [var-drop-assign] A plain value into `Mut` storage.
        ConvertStep::IntoMut { to } => Expr { ty: erase(to), span, kind: ExprKind::IntoMut { value: Box::new(value) } },
        // An optional drops `Mut` on its present arm [proj-opt-slot].
        ConvertStep::DropMut { from } => {
            Expr { ty: crate::types::drop_mut_ty(&erase(from)), span, kind: ExprKind::DropMut { value: Box::new(value) } }
        }
        // [op-promote] A literal is retyped; anything else widened.
        ConvertStep::Promote { to } => {
            let target = erase(to);
            let span = value.span;
            match (&value.kind, target.strip_quals()) {
                (ExprKind::Int(v), Ty::Named { name, .. }) if name == "Long" => Expr { ty: target, span, kind: ExprKind::Long(*v) },
                (ExprKind::Float(v), Ty::Named { name, .. }) if name == "Double" => Expr { ty: target, span, kind: ExprKind::Double(*v) },
                (_, Ty::Named { .. }) => Expr { ty: target, span, kind: ExprKind::Widen { value: Box::new(value) } },
                _ => value,
            }
        }
    }
}

