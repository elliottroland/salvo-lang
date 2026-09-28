//! [effect-member-overload] Effect-member naming, shared by the backends.
//!
//! An effect may declare one member name several times, as an *overload*
//! (`close(InStream)` and `close(OutStream)` on phase 4's `Fs`). Salvo
//! resolves the overload in the checker, and both target languages must be
//! kept from having a second opinion — Rust cannot overload a trait method at
//! all, and Kotlin would resolve by *Kotlin's* type lattice
//! ([kt-fn-mangling] is the same hazard for top-level fns). So every overload
//! after the first gets a distinct emitted name.
//!
//! The rule lives here rather than twice in the emitters because the two must
//! agree by construction: an interface declares the name, a handler overrides
//! it, a fused forwarder calls it, and a `platform generate` skeleton
//! implements it — four renderers, one answer.

use salvo_syntax::ast::{EffectDecl, EffectRef, FnDecl, HandlerDecl};

/// [effect-handler-deps] Whether a handler's dependency form is **owned
/// handles**: a plain-face, non-mixed, dep-bearing handler whose declared
/// effects are plain non-generic effects (no `use`, no `spawn`). Such a
/// handler captures its dependencies as handles at construction and its
/// members reach them through fields; every other dep-bearing handler keeps
/// the fusion form. Transitional (ROADMAP §2b): every dependency becomes a
/// handle in steps ②/③ and this predicate goes.
///
/// Shared by the checker and both emitters so the struct shape and the
/// classification cannot disagree; `is_actor_effect` answers for a face name
/// because only the caller has the effect declarations.
pub fn handler_handle_deps(h: &HandlerDecl, is_actor_effect: impl Fn(&str) -> bool) -> bool {
    let mut has_deps = false;
    for eff in h.effects.iter().flatten() {
        match eff {
            // An actor-effect dependency has no owned-handle form (an addr
            // travels as a constructor parameter instead), and a generic
            // effect instance has no `__Mon_E` — either pins the fusion
            // form. [effect-generic-decl] Except an
            // instance whose every argument is an effect (`Pick<Ping>`): that
            // erases to the monomorphic `__Mon_Pick`, so the handle exists.
            EffectRef::Effect(r) | EffectRef::AnyEffect(r) => {
                if is_actor_effect(&r.name.name) || !effect_only_args(r, &is_actor_effect) {
                    return false;
                }
                has_deps = true;
            }
            EffectRef::Use(_) | EffectRef::Spawn(_) => return false,
        }
    }
    if !has_deps || h.fns.iter().any(|f| f.is_send) || h.of.is_empty() {
        return false;
    }
    h.of.iter().all(|of| {
        let name = match of {
            salvo_syntax::ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
            salvo_syntax::ast::Type::QualifiedGroup { base, .. } => match base.as_ref() {
                salvo_syntax::ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
                _ => None,
            },
            _ => None,
        };
        name.is_some_and(|n| !is_actor_effect(n))
    })
}

/// [effect-generic-decl] Whether every type argument of `r` names an effect
/// (an instance that erases to a monomorphic type), `true` for no arguments.
/// `is_effect` answers for a name; an actor effect counts, since it is the
/// common argument (`Pick<Ping>`).
pub fn effect_only_args(r: &salvo_syntax::ast::TypeRef, is_effect: &impl Fn(&str) -> bool) -> bool {
    r.args.iter().all(|a| match a {
        salvo_syntax::ast::Type::Named { base, qualifiers } => {
            qualifiers.is_empty() && base.args.is_empty() && is_effect(&base.name.name)
        }
        _ => false,
    })
}

/// [effect-any] Whether `program` declares a handler `of any E` for the
/// actor effect `effect` — a router. Only then does an actor effect need
/// the monitor adapter a plain effect always has: a router is the one
/// actor-effect handler whose `use` binds shareable (as a lock, or bare),
/// since it forwards to members and holds no protocol state of its own.
pub fn has_any_router<'a>(
    handlers: impl IntoIterator<Item = &'a HandlerDecl>,
    effect: &str,
) -> bool {
    handlers.into_iter().any(|h| {
        h.of.iter().zip(h.of_any.iter()).any(|(of, any)| {
            *any && of_base_name(of) == Some(effect)
        })
    })
}

/// The base name of an `of` clause entry.
fn of_base_name(of: &salvo_syntax::ast::Type) -> Option<&str> {
    match of {
        salvo_syntax::ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
        salvo_syntax::ast::Type::QualifiedGroup { base, .. } => of_base_name(base),
        _ => None,
    }
}

/// [effect-member-overload] The emitted name of the member at `idx` in
/// `effect`'s declaration order: the plain name for a member whose name is
/// declared once, and for the **first** of an overload set; `name__2`,
/// `name__3`, … for the ones after it.
///
/// Positional suffixes need no qualifier pass ([kt-qual-mangling] exists for
/// fns because two overloads may erase to one target signature): here every
/// overload but the first is renamed regardless, so no erasure collision can
/// survive. The backends pass the result through their own identifier
/// escaping.
pub fn effect_member_name(effect: &EffectDecl, idx: usize) -> String {
    let Some(member) = effect.fns.get(idx) else {
        return String::new();
    };
    let name = &member.name.name;
    let before = effect.fns[..idx]
        .iter()
        .filter(|f| f.name.name == *name)
        .count();
    if before == 0 {
        name.clone()
    } else {
        format!("{name}__{}", before + 1)
    }
}

/// [effect-member-overload] Where `member` sits in `effect`'s declaration
/// order. Identity first (a member *of* this effect), then — for a
/// declaration that only mirrors one, like a handler's implementing fn or a
/// checker-side clone — by name and written parameter types, which is what
/// distinguishes one overload from another.
pub fn effect_member_index(effect: &EffectDecl, member: &FnDecl) -> Option<usize> {
    if let Some(i) = effect
        .fns
        .iter()
        .position(|f| std::ptr::eq(f as *const FnDecl, member as *const FnDecl))
    {
        return Some(i);
    }
    let same_name: Vec<usize> = effect
        .fns
        .iter()
        .enumerate()
        .filter(|(_, f)| f.name.name == member.name.name)
        .map(|(i, _)| i)
        .collect();
    match same_name.as_slice() {
        [] => None,
        [only] => Some(*only),
        several => {
            let sig = |f: &FnDecl| -> Vec<String> {
                f.params
                    .iter()
                    .filter(|p| !p.implicit)
                    .map(|p| p.ty.to_string())
                    .collect()
            };
            let mine = sig(member);
            several
                .iter()
                .copied()
                .find(|&i| sig(&effect.fns[i]) == mine)
                // A signature that matches nothing exactly (a type spelled
                // through an alias, say) still has to emit *something*
                // deterministic rather than silently taking the first
                // overload's name: the arity narrows it, then declaration
                // order decides.
                .or_else(|| {
                    several
                        .iter()
                        .copied()
                        .find(|&i| sig(&effect.fns[i]).len() == mine.len())
                })
        }
    }
}

/// [effect-member-overload] Every member of `effect` named `name`, in
/// declaration order — the candidate set a call resolves within.
pub fn effect_members_named<'e>(effect: &'e EffectDecl, name: &str) -> Vec<&'e FnDecl> {
    effect.fns.iter().filter(|f| f.name.name == name).collect()
}

/// [effect-handler-multi] Which member of which **face** a handler's member
/// implements: one entry per face it satisfies, in the handler's declaration
/// order of faces.
///
/// Why this is not just [`effect_member_index`] per face: that function is
/// deliberately lenient — a face declaring exactly one member of the name
/// matches by name alone, so a type spelled through an alias still lands on it
/// — and across faces that leniency makes `A.ping(Int)` and `B.ping(Str)`
/// both claim a handler's `ping(n: Int)`. So a member matching several faces
/// keeps those whose **written parameter types** agree exactly, which is what
/// overloading distinguishes them by; if none agree, all of them are kept, so
/// the mismatch surfaces as a diagnostic rather than being silently dropped.
///
/// Several entries therefore means one method implements a same-named member of
/// several faces, which is legal only when the signatures are identical — the
/// checker's rule, and the reason both emitters may treat any of them as *the*
/// one that names the member.
pub fn handler_member_faces<'a>(
    faces: &[&'a EffectDecl],
    member: &FnDecl,
) -> Vec<(&'a EffectDecl, usize)> {
    let matched: Vec<(&'a EffectDecl, usize)> = faces
        .iter()
        .filter_map(|e| effect_member_index(e, member).map(|i| (*e, i)))
        .collect();
    if matched.len() < 2 {
        return matched;
    }
    let sig = |f: &FnDecl| -> Vec<String> {
        f.params
            .iter()
            .filter(|p| !p.implicit)
            .map(|p| p.ty.to_string())
            .collect()
    };
    let mine = sig(member);
    let exact: Vec<(&'a EffectDecl, usize)> = matched
        .iter()
        .copied()
        .filter(|(e, i)| e.fns.get(*i).map(sig).as_ref() == Some(&mine))
        .collect();
    if exact.is_empty() {
        matched
    } else {
        exact
    }
}
