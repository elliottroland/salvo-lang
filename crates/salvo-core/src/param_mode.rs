//! Parameter modes [param-mode]: how a callee holds each parameter.
//!
//! A parameter is *moved in* (the callee consumes it), *lent* (the callee
//! reads it and the caller keeps it) or *lent mutably* (the callee may change
//! it in place). That is a fact about Salvo's ownership, decided here from
//! the deduction tables, the written `=>` clause and the declared type.
//! Nothing here knows how a target passes a value: that a scalar is copied
//! for free, that a fn value is a `&mut impl FnMut`, that a variadic tail is
//! a `Vec` are a backend's to apply on top (the Rust backend does, in
//! `param_mode`). A backend whose values are all references ignores the modes.
//!
//! Three families of fn get it from different inputs:
//!
//! * a top-level fn: the checker's deductions (`Checked::deductions`);
//! * an effect member (and the handler members that implement one): the
//!   clause written on the member (`=> !p`);
//! * anything else (qualifier fns, handler-local members): every parameter is
//!   kept.
//!
//! A fn-typed parameter is lent (the callee calls it and the caller keeps
//! it), or moved in when the callee stores its callbacks. A variadic tail and
//! a `once` fn group are moved in: the callee receives a fresh list, and
//! calls a `once` closure once.

use salvo_syntax::ast::{DeductionKind, FnDecl, Item, Param, Type};

use crate::check::Checked;
use crate::program::{Program, Symbols};
use crate::FnKey;

/// How a callee takes one parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassMode {
    /// Consumed: the caller gives it up.
    Moved,
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
    /// moved; kept = lent, mutably when the declared type carries `Mut` or
    /// its elements do).
    pub fn fn_param(&self, key: Option<FnKey>, param: &Param) -> PassMode {
        if param.variadic || is_fn_group(&param.ty) {
            return PassMode::Moved;
        }
        if matches!(param.ty, Type::Fn { .. }) {
            // [rs-fn-field] A callback the callee **stores** is moved in: a
            // composed pass calls it once per element, long after this
            // returns. Otherwise it is lent for the call [fn-contract].
            return if key.is_some_and(|k| self.owns_callbacks(k)) { PassMode::Moved } else { PassMode::Lent };
        }
        let kept = key
            .and_then(|k| self.checked.deductions.get(&k))
            .and_then(|ds| ds.iter().find(|d| d.param == param.name.name))
            .map(|d| d.kept)
            .unwrap_or(true); // default kept (lenient, like deduce.rs)
        if !kept {
            return PassMode::Moved;
        }
        kept_mode(&param.ty)
    }

    /// The mode of an effect member's parameter (and of the handler members
    /// that implement it), from the clause written on the member.
    pub fn member_param(&self, member: &FnDecl, p: &Param) -> PassMode {
        if p.variadic || is_fn_group(&p.ty) {
            return PassMode::Moved;
        }
        if matches!(p.ty, Type::Fn { .. }) {
            return PassMode::Lent;
        }
        let moved = member.deductions.iter().flatten().any(|d| {
            d.param_name().is_some_and(|n| n.name == p.name.name)
                // [defer-deduction] A deferral is consumed like a move.
                && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
        });
        if moved {
            PassMode::Moved
        } else {
            kept_mode(&p.ty)
        }
    }

    /// The mode of a parameter of a fn outside the deduction tables
    /// (qualifier fns, handler-local members): always kept.
    pub fn default_param(&self, ty: &Type, variadic: bool) -> PassMode {
        if variadic || is_fn_group(ty) {
            return PassMode::Moved;
        }
        if matches!(ty, Type::Fn { .. }) {
            return PassMode::Lent;
        }
        kept_mode(ty)
    }

    /// Whether a type is one of the scalars (`Int`, `Long`, `Float`, `Double`,
    /// `Bool`, `Char`, `Byte`), through aliases. A fact about the type; what a
    /// target does with it (copy it for free, say) is the backend's.
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
    /// way: kept and `Mut` is lent mutably, kept is lent, moved is moved.
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
        } else if kept {
            PassMode::Lent
        } else {
            PassMode::Moved
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
