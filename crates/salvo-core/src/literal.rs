//! [type-literal] [union-arm-identity] The runtime representation of literal
//! types (user decisions 2026-09-30).
//!
//! A literal type *is* its base type at run time. In a union, the literals of
//! one base collapse into one arm of that base, and the arms keep the order of
//! their first appearance:
//!
//! * `"A" | "B"` is a `Str`; `"A" | "B" | None` is a `Str?`.
//! * `"A" | "B" | Other Str` is still a `Str`: a non-literal arm whose base is
//!   the literals' base joins their arm — when it is the only such arm, since
//!   two of them (`Ok Str | Err Str | "A"`) are told apart by the wrapper's
//!   tag, and a literal cannot say which of them it would belong to.
//! * `"name" | 3 | "other" | Person` is `Str | Int | Person`.
//!
//! The checker reasons over the *declared* union — the literals are what
//! narrowing and exhaustiveness are about — and every table it hands a backend
//! is rewritten into these runtime arms at the end of checking
//! ([`runtime_arm`]), so a backend only ever sees collapsed types. One
//! definition, so the checker and both emitters cannot disagree.

use salvo_syntax::ast::{self, TypeLit};

use crate::types::Ty;

/// How one declared value arm (non-`None`) maps into the runtime union.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Layout {
    /// For each declared value arm: its runtime arm index.
    to_runtime: Vec<usize>,
    /// The runtime value arms, in order.
    runtime: Vec<Ty>,
}

/// The base name a type erases to when it is a plain named type with no
/// arguments (qualifiers stripped): `Other Str` → `Str`.
fn plain_base(ty: &Ty) -> Option<&str> {
    match ty.strip_quals() {
        Ty::Named { name, args } if args.is_empty() => Some(name.as_str()),
        _ => None,
    }
}

fn layout(value_arms: &[&Ty]) -> Layout {
    // The bases that have literals, and how many non-literal arms share each.
    let lit_bases: Vec<&str> = value_arms
        .iter()
        .filter_map(|a| match a {
            Ty::Lit(l) => Some(l.base()),
            _ => None,
        })
        .collect();
    let joiners = |base: &str| {
        value_arms
            .iter()
            .filter(|a| !matches!(a, Ty::Lit(_)) && plain_base(a) == Some(base))
            .count()
    };
    let mut runtime: Vec<Ty> = Vec::new();
    // Runtime arm index per collapsed base.
    let mut base_slot: Vec<(&str, usize)> = Vec::new();
    let mut to_runtime = Vec::with_capacity(value_arms.len());
    for arm in value_arms {
        let base = match arm {
            Ty::Lit(l) => Some(l.base()),
            other => plain_base(other).filter(|b| lit_bases.contains(b) && joiners(b) == 1),
        };
        match base {
            Some(b) => {
                if let Some((_, slot)) = base_slot.iter().find(|(n, _)| *n == b) {
                    to_runtime.push(*slot);
                    // A joining non-literal arm keeps its own spelling (and
                    // qualifiers, which erase) in the runtime union.
                    if !matches!(arm, Ty::Lit(_)) {
                        runtime[*slot] = collapse_ty(arm);
                    }
                } else {
                    let ty = if matches!(arm, Ty::Lit(_)) { Ty::named(b) } else { collapse_ty(arm) };
                    runtime.push(ty);
                    base_slot.push((b, runtime.len() - 1));
                    to_runtime.push(runtime.len() - 1);
                }
            }
            None => {
                runtime.push(collapse_ty(arm));
                to_runtime.push(runtime.len() - 1);
            }
        }
    }
    Layout { to_runtime, runtime }
}

/// Whether a type mentions a literal anywhere (the cheap test that lets
/// every other type through [`collapse_ty`] untouched).
pub fn mentions_lit(ty: &Ty) -> bool {
    match ty {
        Ty::Lit(_) => true,
        Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().any(mentions_lit),
        Ty::Named { args, .. } => args.iter().any(mentions_lit),
        Ty::Qualified { base, .. } => mentions_lit(base),
        Ty::Array(e) => mentions_lit(e),
        Ty::Fn { params, ret, .. } => params.iter().any(mentions_lit) || mentions_lit(ret),
        _ => false,
    }
}

/// The runtime representation of a type: every literal becomes its base, and
/// every union collapses its literals as the module documentation says.
pub fn collapse_ty(ty: &Ty) -> Ty {
    if !mentions_lit(ty) {
        return ty.clone();
    }
    match ty {
        Ty::Lit(l) => Ty::named(l.base()),
        Ty::Union(arms) => {
            let values: Vec<&Ty> = arms.iter().filter(|a| !a.is_none_ty()).collect();
            let has_none = values.len() < arms.len();
            let lay = layout(&values);
            let mut out = lay.runtime;
            if has_none {
                out.push(Ty::none());
            }
            Ty::union_of(out)
        }
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(collapse_ty).collect() },
        Ty::Qualified { quals, base } => Ty::Qualified { quals: quals.clone(), base: Box::new(collapse_ty(base)) },
        Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(collapse_ty).collect()),
        Ty::Array(e) => Ty::Array(Box::new(collapse_ty(e))),
        Ty::Fn { params, ret, contract, effects } => Ty::Fn {
            params: params.iter().map(collapse_ty).collect(),
            ret: Box::new(collapse_ty(ret)),
            contract: contract.clone(),
            effects: effects.clone(),
        },
        other => other.clone(),
    }
}

/// [type-literal] The *widened* reading of a type with literals, for where
/// its base is expected: like [`collapse_ty`], but an arm that holds literals
/// is its plain base — the open arm's qualifier included, so `"A" | "B" |
/// Other Str` widens to `Str`. The widening rule is exactly this: a value of a
/// literal union is its runtime shape.
pub fn widen(ty: &Ty) -> Ty {
    if !mentions_lit(ty) {
        return ty.clone();
    }
    match ty {
        Ty::Lit(l) => Ty::named(l.base()),
        Ty::Union(arms) => {
            let values: Vec<&Ty> = arms.iter().filter(|a| !a.is_none_ty()).collect();
            let has_none = values.len() < arms.len();
            let lay = layout(&values);
            let mut out: Vec<Ty> = lay
                .runtime
                .iter()
                .enumerate()
                .map(|(r, t)| {
                    let has_lit = values
                        .iter()
                        .zip(&lay.to_runtime)
                        .any(|(a, at)| *at == r && matches!(a, Ty::Lit(_)));
                    if has_lit {
                        t.strip_quals().clone()
                    } else {
                        t.clone()
                    }
                })
                .collect();
            if has_none {
                out.push(Ty::none());
            }
            Ty::union_of(out)
        }
        other => collapse_ty(other),
    }
}

/// Where declared value arm [arm] of [union] lives at run time: the index
/// among the runtime union's value arms, or `None` when the runtime type is
/// not a union at all (every value arm collapsed into one base).
pub fn runtime_arm(union: &Ty, arm: usize) -> Option<usize> {
    let values = union.value_arms();
    let lay = layout(&values);
    if lay.runtime.len() < 2 {
        return None;
    }
    lay.to_runtime.get(arm).copied()
}

/// How many value arms the runtime union has (1 when it is not a union).
pub fn runtime_size(union: &Ty) -> usize {
    layout(&union.value_arms()).runtime.len()
}

/// [type-literal] [`collapse_ast`] after expanding, inside a union, every arm
/// that names a non-generic type alias — `Kms | "QueueDoesNotExist" | Other
/// Str` must see `Kms`'s literals to collapse them. [alias] answers an alias's
/// definition by name.
pub fn collapse_ast_with<'a>(ty: &ast::Type, alias: &dyn Fn(&str) -> Option<&'a ast::Type>) -> ast::Type {
    fn expand<'a>(ty: &ast::Type, alias: &dyn Fn(&str) -> Option<&'a ast::Type>, depth: usize) -> ast::Type {
        match ty {
            ast::Type::Union { arms, span } if depth < 16 => {
                let mut out = Vec::new();
                for arm in arms {
                    let arm = match arm {
                        ast::Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() => {
                            match alias(&base.name.name) {
                                Some(def) => expand(def, alias, depth + 1),
                                None => arm.clone(),
                            }
                        }
                        other => expand(other, alias, depth + 1),
                    };
                    match arm {
                        ast::Type::Union { arms: inner, .. } => out.extend(inner),
                        other => out.push(other),
                    }
                }
                ast::Type::Union { arms: out, span: *span }
            }
            ast::Type::Nullable { inner, span } => {
                ast::Type::Nullable { inner: Box::new(expand(inner, alias, depth + 1)), span: *span }
            }
            other => other.clone(),
        }
    }
    let expanded = expand(ty, alias, 0);
    let collapsed = collapse_ast(&expanded);
    // Nothing had literals: keep the written type, aliases and all.
    if collapsed == expanded {
        ty.clone()
    } else {
        collapsed
    }
}

/// [type-literal] The written-type counterpart of [`collapse_ty`], for the
/// backends' renderings of declared (AST) types.
pub fn collapse_ast(ty: &ast::Type) -> ast::Type {
    fn mentions(ty: &ast::Type) -> bool {
        match ty {
            ast::Type::Literal { .. } => true,
            ast::Type::Union { arms, .. } | ast::Type::Tuple { elems: arms, .. } => arms.iter().any(mentions),
            ast::Type::Nullable { inner, .. } | ast::Type::Array { elem: inner, .. } => mentions(inner),
            ast::Type::QualifiedGroup { base, .. } => mentions(base),
            ast::Type::Named { base, .. } => base.args.iter().any(mentions),
            ast::Type::Fn { params, ret, .. } => params.iter().any(mentions) || mentions(ret),
        }
    }
    if !mentions(ty) {
        return ty.clone();
    }
    fn plain(ty: &ast::Type) -> Option<&str> {
        match ty {
            ast::Type::Named { base, .. } if base.args.is_empty() => Some(base.name.name.as_str()),
            _ => None,
        }
    }
    match ty {
        ast::Type::Literal { value, span } => value.base_type(*span),
        ast::Type::Union { arms, span } => {
            let lits: Vec<&TypeLit> = arms
                .iter()
                .filter_map(|a| match a {
                    ast::Type::Literal { value, .. } => Some(value),
                    _ => None,
                })
                .collect();
            let joiners = |b: &str| arms.iter().filter(|a| plain(a) == Some(b)).count();
            let mut out: Vec<ast::Type> = Vec::new();
            let mut seen: Vec<(&str, usize)> = Vec::new();
            for arm in arms {
                let base = match arm {
                    ast::Type::Literal { value, .. } => Some(value.base()),
                    other => plain(other).filter(|b| lits.iter().any(|l| l.base() == *b) && joiners(b) == 1),
                };
                match base {
                    Some(b) => match seen.iter().find(|(n, _)| *n == b) {
                        Some((_, at)) => {
                            if !matches!(arm, ast::Type::Literal { .. }) {
                                out[*at] = arm.clone();
                            }
                        }
                        None => {
                            out.push(match arm {
                                ast::Type::Literal { value, span } => value.base_type(*span),
                                other => other.clone(),
                            });
                            seen.push((b, out.len() - 1));
                        }
                    },
                    None => out.push(collapse_ast(arm)),
                }
            }
            if out.len() == 1 {
                out.pop().unwrap()
            } else {
                ast::Type::Union { arms: out, span: *span }
            }
        }
        ast::Type::Nullable { inner, span } => ast::Type::Nullable { inner: Box::new(collapse_ast(inner)), span: *span },
        ast::Type::Array { elem, span } => ast::Type::Array { elem: Box::new(collapse_ast(elem)), span: *span },
        ast::Type::Tuple { elems, span } => ast::Type::Tuple { elems: elems.iter().map(collapse_ast).collect(), span: *span },
        other => other.clone(),
    }
}

/// [union-arm-identity] Rewrites the checker's coercions into the runtime
/// arms, once, after the last round: an arm index into a declared union with
/// literals becomes its runtime arm, a wrap into a union that collapsed to its
/// base is no wrap at all (or an optional wrap, where `None` remains), and a
/// rewrap between two unions with the same runtime shape is nothing. The `is`
/// tests are already lowered where they are recorded.
pub fn lower_tables(out: &mut crate::check::Checked) {
    use crate::check::Coercion;
    fn lower(c: &Coercion) -> Option<Coercion> {
        match c {
            Coercion::WrapUnion { target, arm, inner } if mentions_lit(target) => {
                let inner = inner.as_ref().and_then(|i| lower(i).map(Box::new));
                // A backend renders the target arm by arm, so it gets the
                // runtime union itself.
                match runtime_arm(target, *arm) {
                    Some(r) => Some(Coercion::WrapUnion { target: collapse_ty(target), arm: r, inner }),
                    None if target.has_none_arm() => Some(Coercion::WrapOption { target: collapse_ty(target) }),
                    None => None,
                }
            }
            Coercion::WrapOption { target } if mentions_lit(target) => {
                Some(Coercion::WrapOption { target: collapse_ty(target) })
            }
            Coercion::Rewrap { from, to } if (mentions_lit(from) || mentions_lit(to)) => {
                let (f, t) = (collapse_ty(from), collapse_ty(to));
                if f == t {
                    None
                } else {
                    Some(Coercion::Rewrap { from: f, to: t })
                }
            }
            other => Some(other.clone()),
        }
    }
    // The declared type of a narrowed identifier decides whether a backend
    // unwraps it (`.value`), so it must be the runtime shape too.
    for ty in out.repr_ty.values_mut() {
        if mentions_lit(ty) {
            *ty = collapse_ty(ty);
        }
    }
    let keys: Vec<_> = out.coerce.keys().copied().collect();
    for k in keys {
        let c = out.coerce[&k].clone();
        match lower(&c) {
            Some(l) => {
                out.coerce.insert(k, l);
            }
            None => {
                out.coerce.remove(&k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Ty {
        Ty::Lit(TypeLit::Str(v.to_string()))
    }

    #[test]
    fn one_base_is_that_base() {
        assert_eq!(collapse_ty(&Ty::union_of(vec![s("A"), s("B")])), Ty::named("Str"));
        assert_eq!(
            collapse_ty(&Ty::union_of(vec![s("A"), s("B"), Ty::none()])),
            Ty::union_of(vec![Ty::named("Str"), Ty::none()])
        );
        let u = Ty::union_of(vec![s("A"), s("B")]);
        assert_eq!(runtime_arm(&u, 1), None);
    }

    #[test]
    fn mixed_arms_keep_first_appearance() {
        let u = Ty::union_of(vec![s("name"), Ty::Lit(TypeLit::Int(3)), s("other"), Ty::named("Person")]);
        assert_eq!(
            collapse_ty(&u),
            Ty::union_of(vec![Ty::named("Str"), Ty::named("Int"), Ty::named("Person")])
        );
        assert_eq!(runtime_arm(&u, 0), Some(0));
        assert_eq!(runtime_arm(&u, 1), Some(1));
        assert_eq!(runtime_arm(&u, 2), Some(0));
        assert_eq!(runtime_arm(&u, 3), Some(2));
    }
}
