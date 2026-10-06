//! Borrowing results [borrowed-results]: which results a call hands back as
//! borrows of its arguments rather than as fresh values.
//!
//! A fn with a derived return (`=> p`, [readonly-return]) lends a view of
//! what it was given, and a written `proj` on a union arm says that arm is a
//! borrow. Both are facts about the declaration; a target that spells
//! borrows (Rust: `Option<&T>`, `Union2<&T, Finished>`) reads them, and a
//! garbage-collected one ignores them.

use salvo_syntax::ast::{Expr, Type, TypeRef};

use crate::check::Checked;
use crate::program::Program;

/// [rs-opt-borrow] Whether `value` is a derived-return call
/// [readonly-return] whose declared result is optional: the shape that
/// produces an optional borrow.
pub fn is_optional_derived_call(checked: &Checked, file_idx: usize, value: &Expr) -> bool {
    let Expr::Call { span, .. } = value else { return false };
    if !checked.derived_calls.contains_key(&(file_idx, *span)) {
        return false;
    }
    checked.ty_of(file_idx, *span).is_some_and(|t| t.strip_quals().has_none_arm())
}

/// [rs-proj-arm] The value-arm indices a call's result holds as borrows: the
/// `proj` arms of the resolved callee's written return type. Empty for
/// anything else.
pub fn call_borrowed_arms(program: &Program, checked: &Checked, file_idx: usize, call: &Expr) -> Vec<usize> {
    let Expr::Call { span, .. } = call else { return Vec::new() };
    let Some(key) = checked.call_fn.get(&(file_idx, *span)).copied() else { return Vec::new() };
    let decl = match program.modules.get(key.file).and_then(|m| m.items.get(key.item)) {
        Some(salvo_syntax::ast::Item::Fn(f)) => f,
        _ => return Vec::new(),
    };
    let Some(Type::Union { arms, .. }) = decl.return_type.as_ref() else { return Vec::new() };
    arms.iter()
        .filter(|a| !is_none_type(a))
        .enumerate()
        .filter(|(_, a)| type_has_proj(a))
        .map(|(i, _)| i)
        .collect()
}

fn is_none_type(ty: &Type) -> bool {
    matches!(ty, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == "None")
}

/// [rs-proj-struct] Whether a written type carries a `proj` anywhere.
pub fn type_has_proj(ty: &Type) -> bool {
    fn in_ref(r: &TypeRef) -> bool {
        r.name.name == "proj" || r.args.iter().any(type_has_proj)
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers.iter().any(in_ref) || in_ref(base),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(in_ref) || type_has_proj(base),
        Type::Nullable { inner, .. } | Type::Array { elem: inner, .. } => type_has_proj(inner),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().any(type_has_proj)
        }
        _ => false,
    }
}
