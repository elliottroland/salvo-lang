//! [op-safe-nav] Recognizing a lowered safe-navigation link in the IR, so a
//! backend can reconstruct a native optional chain (`a?.b` on Kotlin, an
//! `.and_then`/`.map` combinator chain on Rust) instead of emitting the raw
//! `Switch`.
//!
//! `?.` is a `when` by the time the IR sees it [when-lowered]:
//! `when __navN = base { is #{first arm} { __navN.link } is #{rest} { __navN } }`.
//! On a `T?` receiver that builds a `block_expr` wrapper binding the receiver
//! once into `__navN`, then a two-arm `Switch` over it — a first arm whose
//! first statement is a `Narrow` carrying [`Justification::SafeNav`] (the
//! hint) and whose value is the link performed on the narrowed receiver, and
//! an `Else` arm yielding `MakeNone`. The `SafeNav` justification is what
//! tells this shape apart from a hand-written `when`; everything else is
//! matched structurally.

use crate::ir::nodes::*;

/// The parts of a recognized safe-navigation link.
pub struct SafeNav<'a> {
    /// The receiver expression (bound once, tested for `None`).
    pub receiver: &'a Expr,
    /// The local the present arm narrows the receiver to.
    pub nav: &'a Local,
    /// The link performed on `nav`: a `Read` of a field, or a `Call`/
    /// `MemberCall` with `Read(nav)` as the receiver argument. Already lifted
    /// into the optional result by a `Present` wrapper unless the link was
    /// itself optional (`already_opt`).
    pub value: &'a Expr,
    /// The link's own result was already optional, so `value` is not wrapped
    /// in a `Present`.
    pub already_opt: bool,
    /// The result type of the whole `?.` (an optional).
    pub ty: &'a Ty,
}

/// Recognize a `?.` lowering at `e`. Returns the parts when `e` is the
/// `block_expr`-wrapped two-arm `Switch` the builder emits for `?.`, with the
/// present arm's `Narrow` justified by [`Justification::SafeNav`].
pub fn match_safe_nav(e: &Expr) -> Option<SafeNav<'_>> {
    // Outer wrapper: `block_expr` → single always-true `Branch` arm, no else.
    let ExprKind::Branch {
        arms,
        otherwise: None,
        ..
    } = &e.kind
    else {
        return None;
    };
    let [(cond, body)] = arms.as_slice() else {
        return None;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return None;
    }
    // One `Let` (the receiver temp), then a `Switch` on it as the value.
    let [Stmt::Let {
        local: safe,
        value: receiver,
        ..
    }] = body.stmts.as_slice()
    else {
        return None;
    };
    let sw = body.value.as_deref()?;
    let ExprKind::Switch {
        subject,
        arms: sarms,
        ..
    } = &sw.kind
    else {
        return None;
    };
    if bare_local(subject) != Some(safe) {
        return None;
    }
    // Exactly two arms: the first arm → narrow + link, `Else` → `None`.
    let [present_arm, absent_arm] = sarms.as_slice() else {
        return None;
    };
    if matches!(present_arm.test, ArmTest::Else) || !matches!(absent_arm.test, ArmTest::Else) {
        return None;
    }
    if !absent_arm.body.stmts.is_empty() || !matches!(absent_arm.body.value.as_deref().map(|v| &v.kind), Some(ExprKind::MakeNone)) {
        return None;
    }
    // The present arm opens with the SafeNav-justified narrow.
    let [Stmt::Narrow {
        local: nav,
        because: Justification::SafeNav { .. },
        ..
    }] = present_arm.body.stmts.as_slice()
    else {
        return None;
    };
    let value = present_arm.body.value.as_deref()?;
    // A `Present` wrapper means the link was lifted into the optional; peel it
    // so the caller sees the raw link, and remember it was not already
    // optional.
    let (value, already_opt) = match &value.kind {
        ExprKind::Present { value } => (value.as_ref(), false),
        _ => (value, true),
    };
    Some(SafeNav {
        receiver,
        nav,
        value,
        already_opt,
        ty: &e.ty,
    })
}

/// The local a bare `Read` of a whole place names, if `e` is exactly that.
fn bare_local(e: &Expr) -> Option<&Local> {
    match &e.kind {
        ExprKind::Read { place, .. } if place.steps.is_empty() => Some(&place.root),
        _ => None,
    }
}
