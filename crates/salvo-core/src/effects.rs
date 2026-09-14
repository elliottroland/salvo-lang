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

use salvo_syntax::ast::{EffectDecl, FnDecl};

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
