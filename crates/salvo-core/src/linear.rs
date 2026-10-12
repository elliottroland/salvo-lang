//! [linear-group] [linear-container] [linear-generics] Linearity predicates
//! over types and the symbol table, in one place so the AST checker
//! (`check.rs`) and the IR-level linear check (`ir::check`) decide
//! linearity identically. The `Checker` methods `has_auto_linear`,
//! `opaque_linear`, `linear_opt_in_positions`, and `ty_own_linear` delegate
//! here; see IR_CHECKER.md.

use std::collections::HashSet;

use crate::check::type_mentions_generic;
use crate::program::Symbols;
use crate::types::Ty;

/// [linear-group] A declaration is **linear** when it carries the `linear`
/// modifier: a `linear struct`, or (the opaque half) a `linear intrinsic
/// type`. A linear leaf has no fields for linearity to reach through.
pub fn has_auto_linear(symbols: &Symbols, name: &str) -> bool {
    symbols.structs.get(name).is_some_and(|s| s.linear) || opaque_linear(symbols, name)
}

/// [linear-group] The opaque half: `linear intrinsic type Reply<T>`.
pub fn opaque_linear(symbols: &Symbols, name: &str) -> bool {
    symbols.intrinsic_types.get(name).is_some_and(|t| t.linear)
}

/// [linear-container] The generic parameter positions of `name` that make a
/// value conditionally linear: a `<T canbe linear>` parameter that reaches a
/// field (for a struct), or any `canbe linear` parameter (for an opaque
/// `intrinsic type`, which has no fields to check against).
pub fn linear_opt_in_positions(symbols: &Symbols, name: &str) -> Vec<usize> {
    if let Some(s) = symbols.structs.get(name) {
        return s
            .generic_canbe
            .iter()
            .filter(|(_, q)| q.name.name == "linear")
            .filter(|(id, _)| s.fields.iter().any(|f| type_mentions_generic(&f.ty, &id.name)))
            .filter_map(|(id, _)| s.generics.iter().position(|g| g.name == id.name))
            .collect();
    }
    if let Some(t) = symbols.intrinsic_types.get(name) {
        return t
            .generic_canbe
            .iter()
            .filter(|(_, q)| q.name.name == "linear")
            .filter_map(|(id, _)| t.generics.iter().position(|g| g.name == id.name))
            .collect();
    }
    Vec::new()
}

/// [linear-container] [linear-generics] Whether `ty` carries a linear
/// obligation: a linear leaf, a conditional container instantiated at a
/// linear type in an opted-in position, a union with any linear arm
/// [linear-union-arm], or a type variable the enclosing fn opted in with
/// `<T canbe linear>` (`own_linear_generics`). Depth-guarded so mutually
/// recursive containers terminate conservatively (`false`).
pub fn ty_own_linear(symbols: &Symbols, own_linear_generics: &HashSet<String>, ty: &Ty) -> bool {
    ty_own_linear_guarded(symbols, own_linear_generics, ty, 0)
}

fn ty_own_linear_guarded(symbols: &Symbols, own: &HashSet<String>, ty: &Ty, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    match ty.strip_quals() {
        Ty::Named { name, args } => {
            if has_auto_linear(symbols, name) {
                return true;
            }
            if args.is_empty() {
                return false;
            }
            linear_opt_in_positions(symbols, name.as_str())
                .into_iter()
                .filter_map(|i| args.get(i))
                .any(|arg| ty_own_linear_guarded(symbols, own, arg, depth + 1))
        }
        Ty::Union(arms) => arms.iter().any(|a| ty_own_linear_guarded(symbols, own, a, depth + 1)),
        Ty::Var(name) => own.contains(name),
        _ => false,
    }
}

/// [linear-union-arm] The runtime arm indices [union-arm-identity] of a union
/// `ty` whose arms carry a linear obligation. For a non-union linear type this
/// is `None` (the value is linear as a whole); for a union with linear arms it
/// is `Some(indices into the runtime arm layout)`; for a non-linear type,
/// `None` with the caller checking `ty_own_linear` first.
pub fn linear_arms(symbols: &Symbols, own: &HashSet<String>, ty: &Ty) -> Option<Vec<usize>> {
    let stripped = ty.strip_quals();
    let Ty::Union(_) = stripped else { return None };
    // Runtime arm layout collapses a base's literals into one arm
    // [type-literal] and is positional over the non-`None` value arms
    // [union-arm-identity]; `literal::layout` is the one definition.
    let collapsed = crate::literal::collapse_ty(stripped);
    let Ty::Union(arms) = collapsed.strip_quals() else { return None };
    let value_arms: Vec<&Ty> = arms.iter().filter(|a| !a.is_none_ty()).collect();
    let linear: Vec<usize> = value_arms
        .iter()
        .enumerate()
        // [proj-type] [ref-handle] A borrowed arm (`ref`/`proj`) owes nothing.
        .filter(|(_, a)| !a.is_proj() && ty_own_linear(symbols, own, a))
        .map(|(i, _)| i)
        .collect();
    (!linear.is_empty()).then_some(linear)
}

/// [linear-group] The names of a struct type's fields that themselves carry a
/// linear obligation. Empty for a non-struct, a struct with no linear fields,
/// or an unknown type. Used to decide when a linear container is settled by
/// decomposition (all its linear fields moved out).
pub fn linear_fields(symbols: &Symbols, own: &HashSet<String>, ty: &Ty) -> Vec<String> {
    let Ty::Named { name, args } = ty.strip_quals() else { return Vec::new() };
    let Some(s) = symbols.structs.get(name.as_str()) else { return Vec::new() };
    let generic_names: HashSet<&str> = s.generics.iter().map(|g| g.name.as_str()).collect();
    let subst: std::collections::HashMap<String, Ty> = s.generics.iter().map(|g| g.name.clone()).zip(args.iter().cloned()).collect();
    s.fields
        .iter()
        .filter(|f| field_ty_linear(symbols, own, &f.ty, &subst, &generic_names))
        .map(|f| f.name.name.clone())
        .collect()
}

/// Whether a struct field's written type carries a linear obligation, handling
/// a bare generic parameter (via the instantiation or the enclosing fn's
/// opt-ins) and a union field arm-by-arm (so one unresolvable arm does not
/// hide a linear one).
fn field_ty_linear(symbols: &Symbols, own: &HashSet<String>, t: &salvo_syntax::ast::Type, subst: &std::collections::HashMap<String, Ty>, generics: &HashSet<&str>) -> bool {
    use salvo_syntax::ast::Type;
    match t {
        Type::Union { arms, .. } => arms.iter().any(|a| field_ty_linear(symbols, own, a, subst, generics)),
        Type::Nullable { inner, .. } => field_ty_linear(symbols, own, inner, subst, generics),
        Type::QualifiedGroup { base, .. } => field_ty_linear(symbols, own, base, subst, generics),
        _ => {
            if let Some(gname) = bare_name(t).filter(|n| generics.contains(n.as_str())) {
                return match subst.get(&gname) {
                    Some(arg) => ty_own_linear(symbols, own, arg),
                    None => own.contains(&gname),
                };
            }
            // Expand a type alias (`Target = ToActor | ToWaiter | ToTask`) so a
            // field written at the alias is tested against its definition.
            if let Some(name) = bare_name(t) {
                if let Some(alias) = symbols.type_aliases.get(name.as_str()) {
                    if let Some(def) = &alias.alias {
                        if alias.generics.is_empty() {
                            return field_ty_linear(symbols, own, def, subst, generics);
                        }
                    }
                }
            }
            crate::approx_ty(t, subst).is_some_and(|ty| ty_own_linear(symbols, own, &ty))
        }
    }
}

/// The name of an argument-less, unqualified named type reference, generic or
/// not; the caller decides whether it is a generic parameter.
fn bare_name(t: &salvo_syntax::ast::Type) -> Option<String> {
    match t {
        salvo_syntax::ast::Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() => Some(base.name.name.clone()),
        _ => None,
    }
}
