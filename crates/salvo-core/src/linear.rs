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
            .filter(|(id, _)| s.fields.iter().any(|f| f.ty.as_ref().is_some_and(|t| type_mentions_generic(t, &id.name))))
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

/// [linear-group] The linear arms of a union, each paired with the display
/// name of its arm type (`(runtime index, "Checked<FsError>")`) — so a leak
/// diagnostic can name the arm (`the `Err Checked<FsError>` arm`) rather than a
/// bare index. Parallel to [`linear_arms`]: same indices, same runtime layout.
pub fn linear_arm_names(symbols: &Symbols, own: &HashSet<String>, ty: &Ty) -> Vec<(usize, String)> {
    let stripped = ty.strip_quals();
    let Ty::Union(_) = stripped else { return Vec::new() };
    let collapsed = crate::literal::collapse_ty(stripped);
    let Ty::Union(arms) = collapsed.strip_quals() else { return Vec::new() };
    let value_arms: Vec<&Ty> = arms.iter().filter(|a| !a.is_none_ty()).collect();
    value_arms
        .iter()
        .enumerate()
        .filter(|(_, a)| !a.is_proj() && ty_own_linear(symbols, own, a))
        .map(|(i, a)| (i, a.to_string()))
        .collect()
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
        .filter(|f| f.ty.as_ref().is_some_and(|t| field_ty_linear(symbols, own, t, &subst, &generic_names)))
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
            // [type-identity] By the key the name has where it was written:
            // runtime's `token: Token` is runtime's struct, not `args.lex`'s
            // alias of the same name.
            if let Type::Named { qualifiers, base } = t {
                let key = symbols.ref_key(base);
                if let Some(alias) = symbols.type_aliases.get(key).filter(|_| qualifiers.is_empty() && base.args.is_empty()) {
                    if let Some(def) = &alias.alias {
                        if alias.generics.is_empty() {
                            return field_ty_linear(symbols, own, def, subst, generics);
                        }
                    }
                }
            }
            crate::approx_ty(t, subst, symbols).is_some_and(|ty| ty_own_linear(symbols, own, &ty))
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

/// [linear-group] The dischargers of a linear type `type_name`: the same-file
/// fns and effect members that consume a value of it — what a leak diagnostic
/// names as the remedy ("discharge it with `drain` / `close`"). Shared by the
/// AST checker (declaration-site "no discharger" check, leak hints) and the
/// IR-level linear check (`ir::analyze`), so both name the same set.
///
/// `scope` is the resolution scope the leak (or the declaration) is reported
/// in — a discharger must be visible there; `deductions` is the inferred
/// contract table (`Checked::deductions`), consulted when a fn has no written
/// `!p` clause.
pub fn discharge_set(
    symbols: &Symbols,
    resolution: &crate::resolve::Resolution,
    scope: &crate::resolve::ModuleScope,
    deductions: &std::collections::HashMap<crate::decl_id::DeclId, Vec<crate::deduce::ParamDeduction>>,
    type_name: &str,
) -> Vec<String> {
    use salvo_syntax::ast;
    // The declaring file keys the rule: either a `linear struct` or a
    // `linear intrinsic type` [linear-group] / conditional container.
    let file_declaring = |addr: usize| resolution.decl_files.get(&addr).copied();
    let struct_file = match symbols.structs.get(type_name) {
        Some(d) => file_declaring(*d as *const ast::StructDecl as usize),
        None if opaque_linear(symbols, type_name) || !linear_opt_in_positions(symbols, type_name).is_empty() => {
            match symbols.intrinsic_types.get(type_name) {
                Some(d) => file_declaring(*d as *const ast::TypeDecl as usize),
                None => scope.opaque_type_files.get(type_name).copied(),
            }
        }
        None => return Vec::new(),
    };
    let mut out: Vec<String> = Vec::new();
    for (fn_name, entries) in scope.fns.iter() {
        for e in entries {
            if Some(e.file) != struct_file {
                continue;
            }
            let consumes_self_typed = e.decl.params.iter().any(|p| {
                let base_matches = match &p.ty {
                    ast::Type::Named { base, .. } => base.name.name == crate::typekey::plain(type_name),
                    ast::Type::QualifiedGroup { base, .. } => matches!(
                        base.as_ref(),
                        ast::Type::Named { base: b, .. } if b.name.name == crate::typekey::plain(type_name)
                    ),
                    _ => false,
                };
                base_matches && param_consumed_by_entry(e, deductions, &p.name.name)
            });
            if consumes_self_typed {
                out.push((*fn_name).to_string());
                break;
            }
        }
    }
    // [linear-group] Consuming *members* of an effect declared in the type's
    // own file; a member has no body, so its written clause is the contract.
    for (effect_name, effect) in scope.effects.iter() {
        if scope.effect_files.get(effect_name).copied() != struct_file {
            continue;
        }
        for m in &effect.fns {
            if member_consumes_type(m, type_name) {
                out.push(m.name.name.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// [linear-group] The leak-diagnostic remedy phrase for a linear type `ty`:
/// its dischargers named (`` `drain` / `close` ``), or the generic "its
/// discharger" when none is visible. The `ty` is the leaked value's type; for
/// a union the first auto-linear arm names the type.
pub fn discharge_hint(
    symbols: &Symbols,
    resolution: &crate::resolve::Resolution,
    scope: &crate::resolve::ModuleScope,
    deductions: &std::collections::HashMap<crate::decl_id::DeclId, Vec<crate::deduce::ParamDeduction>>,
    ty: &Ty,
) -> String {
    let name = match ty.strip_quals() {
        Ty::Named { name, .. } => Some(name.clone()),
        Ty::Union(arms) => arms.iter().find_map(|a| match a.strip_quals() {
            Ty::Named { name, .. } if has_auto_linear(symbols, name) => Some(name.clone()),
            _ => None,
        }),
        _ => None,
    };
    let set = name.as_deref().map(|n| discharge_set(symbols, resolution, scope, deductions, n)).unwrap_or_default();
    if set.is_empty() {
        "its discharger".to_string()
    } else {
        set.iter().map(|f| format!("`{f}`")).collect::<Vec<_>>().join(" / ")
    }
}

/// Whether a fn entry's written/inferred contract consumes `param`.
fn param_consumed_by_entry(
    entry: &crate::resolve::FnEntry,
    deductions: &std::collections::HashMap<crate::decl_id::DeclId, Vec<crate::deduce::ParamDeduction>>,
    param: &str,
) -> bool {
    use salvo_syntax::ast;
    if let Some(list) = &entry.decl.deductions {
        if list.iter().any(|d| {
            d.param_name().is_some_and(|n| n.name == param)
                && matches!(d.kind, ast::DeductionKind::Moved | ast::DeductionKind::Deferred)
        }) {
            return true;
        }
    }
    if let Some(facts) = deductions.get(&entry.key) {
        return facts.iter().any(|f| f.param == param && !f.kept);
    }
    false
}

/// Whether an effect member's written clause consumes a value of `type_name`.
fn member_consumes_type(member: &salvo_syntax::ast::FnDecl, type_name: &str) -> bool {
    use salvo_syntax::ast;
    member.params.iter().any(|p| {
        let base_matches = match &p.ty {
            ast::Type::Named { base, .. } => base.name.name == crate::typekey::plain(type_name),
            ast::Type::QualifiedGroup { base, .. } => matches!(
                base.as_ref(),
                ast::Type::Named { base: b, .. } if b.name.name == crate::typekey::plain(type_name)
            ),
            _ => false,
        };
        base_matches
            && member.deductions.iter().flatten().any(|d| {
                d.param_name().is_some_and(|n| n.name == p.name.name) && matches!(d.kind, ast::DeductionKind::Moved)
            })
    })
}

/// The name of a leaked linear type, for naming its dischargers in a leak
/// diagnostic: a linear leaf's own name, or a union's first linear arm.
/// `None` when none applies.
pub fn linear_type_name(symbols: &Symbols, _own: &HashSet<String>, ty: &Ty) -> Option<String> {
    let names_linear = |name: &str| has_auto_linear(symbols, name) || !linear_opt_in_positions(symbols, name).is_empty();
    match ty.strip_quals() {
        Ty::Named { name, .. } if names_linear(name) => Some(name.clone()),
        Ty::Union(arms) => arms.iter().find_map(|a| match a.strip_quals() {
            Ty::Named { name, .. } if names_linear(name) => Some(name.clone()),
            _ => None,
        }),
        _ => None,
    }
}
