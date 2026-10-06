//! Parameter modes [param-mode]: how a callee holds each parameter.
//!
//! A parameter is *moved in* (the callee consumes it), *lent* (the callee
//! reads it and the caller keeps it) or *lent mutably* (the callee may change
//! it in place). That is a fact about Salvo's ownership, decided here from
//! the deduction tables, the written `=>` clause and the declared type; a
//! backend with values that are all references ignores it, and one that
//! spells ownership (Rust) renders it.
//!
//! Three families of fn get it from different inputs, and the differences are
//! kept exactly:
//!
//! * a top-level fn: the checker's deductions (`Checked::deductions`), with
//!   fn-typed parameters lent mutably unless the fn stores its callbacks;
//! * an effect member (and the handler members that implement one): the
//!   clause written on the member (`=> !p`), whose moved parameters are
//!   consumed;
//! * anything else (qualifier fns, handler-local members): every parameter is
//!   kept.

use salvo_syntax::ast::{DeductionKind, FnDecl, Item, Param, Type};

use crate::check::Checked;
use crate::program::{Program, Symbols};
use crate::FnKey;

/// How a callee takes one parameter [rs-borrows].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassMode {
    /// Consumed: passed by value, and the caller gives it up. Also every
    /// value the target can only pass by value (a scalar, a variadic
    /// `Vec`, a `once` fn group).
    Owned,
    /// Kept by the caller and read by the callee.
    Lent,
    /// Kept by the caller and changed in place by the callee.
    LentMut,
}

/// What the mode functions read.
pub struct Modes<'a, 'p> {
    pub symbols: &'a Symbols<'p>,
    pub checked: &'a Checked,
    pub program: &'a Program,
}

impl<'a, 'p> Modes<'a, 'p> {
    /// The mode of one top-level-fn parameter: deduction-driven (omitted =
    /// moved = by value; kept = lent, mutably when the declared type carries
    /// `Mut`), with the scalar, variadic and fn-value exceptions.
    pub fn fn_param(&self, key: Option<FnKey>, param: &Param) -> PassMode {
        if param.variadic || self.is_copy(&param.ty) || is_fn_group(&param.ty) {
            return PassMode::Owned;
        }
        if matches!(param.ty, Type::Fn { .. }) {
            // [rs-fn-field] A callback the callee **stores** arrives owned
            // rather than lent for the call: a composed pass calls it once
            // per element, long after this returns.
            if key.is_some_and(|k| self.owns_callbacks(k)) {
                return PassMode::Owned;
            }
            // [fn-contract] Fn values are lent mutably.
            return PassMode::LentMut;
        }
        let kept = key
            .and_then(|k| self.checked.deductions.get(&k))
            .and_then(|ds| ds.iter().find(|d| d.param == param.name.name))
            .map(|d| d.kept)
            .unwrap_or(true); // default kept (lenient, like deduce.rs)
        if !kept {
            return PassMode::Owned;
        }
        kept_mode(&param.ty)
    }

    /// The mode of an effect member's parameter (and of the handler members
    /// that implement it), from the clause written on the member.
    pub fn member_param(&self, member: &FnDecl, p: &Param) -> PassMode {
        if p.variadic || self.is_copy(&p.ty) || is_fn_group(&p.ty) {
            return PassMode::Owned;
        }
        if matches!(p.ty, Type::Fn { .. }) {
            return PassMode::LentMut;
        }
        let moved = member.deductions.iter().flatten().any(|d| {
            d.param_name().is_some_and(|n| n.name == p.name.name)
                // [defer-deduction] A deferral is consumed like a move.
                && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
        });
        if moved {
            PassMode::Owned
        } else if type_has_mut(&p.ty) {
            PassMode::LentMut
        } else {
            PassMode::Lent
        }
    }

    /// The mode of a parameter of a fn outside the deduction tables
    /// (qualifier fns, handler-local members): always kept.
    pub fn default_param(&self, ty: &Type, variadic: bool) -> PassMode {
        if variadic || self.is_copy(ty) || is_fn_group(ty) {
            return PassMode::Owned;
        }
        if matches!(ty, Type::Fn { .. }) {
            return PassMode::LentMut;
        }
        kept_mode(ty)
    }

    /// Whether a type is a scalar the target copies for free [rs-borrows]:
    /// `Int`, `Long`, `Float`, `Double`, `Bool`, `Char` or `Byte`, through
    /// aliases.
    pub fn is_copy(&self, ty: &Type) -> bool {
        let Type::Named { qualifiers, base } = ty else { return false };
        if !qualifiers.is_empty() || !base.args.is_empty() {
            return false;
        }
        if let Some(alias) = self.symbols.type_aliases.get(self.checked.written_key(base)) {
            if let Some(target) = &alias.alias {
                return self.is_copy(target);
            }
        }
        matches!(base.name.name.as_str(), "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte")
    }

    /// [rs-fn-field] Whether this fn **stores** a callback it is given: a
    /// fn-typed parameter (or an implicit group, which spreads fn
    /// parameters), and a struct with a fn-typed field as the result. That is
    /// the composed-pass shape, and nothing else in the language keeps a
    /// callback past the call.
    pub fn owns_callbacks(&self, key: FnKey) -> bool {
        let Some(decl) = self.fn_by_key(key) else { return false };
        if !decl.params.iter().any(|p| matches!(p.ty, Type::Fn { .. })) && decl.implicit_groups.is_empty() {
            return false;
        }
        let Some(ret) = decl.return_type.as_ref().and_then(type_base_name) else { return false };
        self.symbols.structs.get(ret).is_some_and(|s| s.fields.iter().any(|f| matches!(f.ty, Type::Fn { .. })))
    }

    /// [rs-fn-param-convention] How the *declaration* of a fn type takes its
    /// `i`th parameter, so a lambda passed into that position binds the same
    /// way: kept and `Mut` is lent mutably, kept and not a scalar is lent,
    /// moved (or a scalar) is by value.
    pub fn fn_type_position(
        &self,
        params: &[Type],
        param_names: &[Option<salvo_syntax::ast::Ident>],
        deductions: &Option<Vec<salvo_syntax::ast::Deduction>>,
        i: usize,
    ) -> PassMode {
        let (kept, is_mut) = fn_type_contract(params, param_names, deductions, i);
        if kept && is_mut {
            PassMode::LentMut
        } else if kept && !params.get(i).is_some_and(|t| self.is_copy(t)) {
            PassMode::Lent
        } else {
            PassMode::Owned
        }
    }

    fn fn_by_key(&self, key: FnKey) -> Option<&'a FnDecl> {
        match self.program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }
}

/// A kept parameter: lent mutably when the declared type carries `Mut`,
/// else lent.
fn kept_mode(ty: &Type) -> PassMode {
    if type_has_mut(ty) || type_has_elem_mut(ty) {
        // [rs-elem-mut] A container with `Mut` elements lends mutable
        // handles, so it is lent mutably even without its own `Mut`
        // [proj-mut].
        PassMode::LentMut
    } else {
        PassMode::Lent
    }
}

/// A `once` fn group: passes by value.
pub fn is_fn_group(ty: &Type) -> bool {
    matches!(ty, Type::QualifiedGroup { base, .. } if matches!(base.as_ref(), Type::Fn { .. }))
}

/// Whether an AST type carries the `Mut` qualifier [rs-borrows].
pub fn type_has_mut(ty: &Type) -> bool {
    match ty {
        Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().any(|q| q.name.name == "Mut")
        }
        Type::Nullable { inner, .. } => type_has_mut(inner),
        _ => false,
    }
}

/// [rs-elem-mut] Whether a container type's **elements** carry `Mut` —
/// `List<Mut T>`, `Mut T[]` — which makes the container lend mutable handles
/// [proj-mut]. Element depth only.
pub fn type_has_elem_mut(ty: &Type) -> bool {
    match ty {
        Type::Named { base, .. } if base.name.name == "List" => base.args.iter().any(type_has_mut),
        Type::Array { elem, .. } => type_has_mut(elem),
        Type::Nullable { inner, .. } => type_has_elem_mut(inner),
        _ => false,
    }
}

fn type_base_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => type_base_name(inner),
        Type::QualifiedGroup { base, .. } => type_base_name(base),
        _ => None,
    }
}

/// The (kept, mutable) contract of one fn-type parameter [fn-contract]: kept
/// unless the written deduction list moves its name [deduce-syntax];
/// mutable when the declared type carries `Mut`.
pub fn fn_type_contract(
    params: &[Type],
    param_names: &[Option<salvo_syntax::ast::Ident>],
    deductions: &Option<Vec<salvo_syntax::ast::Deduction>>,
    i: usize,
) -> (bool, bool) {
    let is_mut = params
        .get(i)
        .map(|t| match t {
            Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                qualifiers.iter().any(|q| q.name.name == "Mut")
            }
            _ => false,
        })
        .unwrap_or(false);
    let kept = match (param_names.get(i).and_then(|n| n.as_ref()), deductions) {
        (Some(name), Some(list)) => !list.iter().any(|d| {
            d.param_name().is_some_and(|n| n.name == name.name)
                && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
        }),
        _ => true,
    };
    (kept, is_mut)
}
