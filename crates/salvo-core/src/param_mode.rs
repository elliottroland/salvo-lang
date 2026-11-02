//! Parameter modes [fn-param-mode]: how a callee holds each parameter.
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

use salvo_syntax::ast::{DeductionKind, EffectDecl, FnDecl, Item, Param, Type};

use crate::check::Checked;
use crate::program::{Program, Symbols};
use crate::DeclId;

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
struct Modes<'a, 'p> {
    symbols: &'a Symbols<'p>,
    checked: &'a Checked,
    program: &'a Program,
}

/// [fn-param-mode] Records every declaration's parameter modes at its name
/// (`Checked::param_modes`, the pipeline record D3.2), one per written parameter in
/// order, an implicit one lent: a top-level fn's from its deductions, an
/// effect member's from its clause, a handler member's from the effect
/// member it implements (a send member's from its own clause), and a
/// qualifier fn's and a handler constructor's kept. Run once the deductions
/// and the parameter mutations are known; what reads a mode reads this.
pub fn record(program: &Program, symbols: &Symbols<'_>, checked: &mut Checked) {
    let mut out: Vec<(DeclId, Vec<PassMode>)> = Vec::new();
    {
        let modes = Modes { symbols, checked, program };
        let each = |ps: &[Param], f: &dyn Fn(&Param) -> PassMode| -> Vec<PassMode> {
            ps.iter().map(|p| if p.implicit { PassMode::Lent } else { f(p) }).collect()
        };
        let decls = &checked.decls;
        for (file, module) in program.modules.iter().enumerate() {
            for (item, it) in module.items.iter().enumerate() {
                match it {
                    Item::Fn(f) => {
                        let Some(id) = decls.item(file, item) else { continue };
                        out.push((id, each(&f.params, &|p| modes.fn_param(Some(id), p))));
                    }
                    Item::Qualifier(q) => {
                        for (i, f) in q.fns.iter().enumerate() {
                            let Some(id) = decls.member(file, item, i + 1) else { continue };
                            out.push((id, each(&f.params, &|p| modes.default_param(&p.ty, p.variadic))));
                        }
                    }
                    Item::Effect(e) => {
                        for (i, m) in e.fns.iter().enumerate() {
                            let Some(id) = decls.member(file, item, i + 1) else { continue };
                            out.push((id, each(&m.params, &|p| modes.member_param(m, p))));
                        }
                    }
                    Item::Handler(h) => {
                        if let Some(id) = decls.item(file, item) {
                            out.push((id, each(&h.params, &|p| modes.default_param(&p.ty, p.variadic))));
                        }
                        let faces: Vec<&EffectDecl> = h
                            .of
                            .iter()
                            .filter_map(|t| match t {
                                Type::Named { base, .. } => symbols.effects.get(checked.written_key(base)).copied(),
                                _ => None,
                            })
                            .collect();
                        let members = h.fns.iter().enumerate().map(|(i, f)| (i + 1, f)).chain(h.init.iter().map(|f| (h.fns.len() + 1, f)));
                        for (sub, f) in members {
                            let Some(id) = decls.member(file, item, sub) else { continue };
                            let implemented = faces.iter().find_map(|e| crate::effect_member_index(e, f).map(|i| &e.fns[i]));
                            let positional: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                            out.push((
                                id,
                                each(&f.params, &|p| match implemented {
                                    Some(m) => {
                                        let at = positional.iter().position(|x| std::ptr::eq(*x, p)).unwrap_or(0);
                                        match m.params.iter().filter(|q| !q.implicit).nth(at) {
                                            Some(ep) => modes.member_param(m, ep),
                                            None => modes.default_param(&p.ty, p.variadic),
                                        }
                                    }
                                    None if f.is_send => modes.member_param(f, p),
                                    None => modes.default_param(&p.ty, p.variadic),
                                }),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    checked.param_modes = out.into_iter().map(|(id, m)| (crate::check::Key::name_of(id), m)).collect();
}

impl Checked {
    /// [fn-param-mode] How the declaration `id` takes each written
    /// parameter, in order (`record`).
    pub fn param_modes_of(&self, id: DeclId) -> Option<&[PassMode]> {
        self.param_modes.get(&crate::check::Key::name_of(id)).map(Vec::as_slice)
    }
}

impl<'a, 'p> Modes<'a, 'p> {
    /// The mode of one top-level-fn parameter: deduction-driven (omitted =
    /// moved; kept = lent, mutably when the declared type carries `Mut` or
    /// its elements do).
    fn fn_param(&self, key: Option<DeclId>, param: &Param) -> PassMode {
        if param.variadic || is_fn_group(&param.ty) {
            return PassMode::Moved;
        }
        if matches!(param.ty, Type::Fn { .. }) {
            // [fn-param-mode] A callback the callee **stores** is moved in: a
            // composed pass calls it once per element, long after this
            // returns. Otherwise it is lent for the call [fn-contract].
            return if key.is_some_and(|k| self.owns_callbacks(k)) { PassMode::Moved } else { PassMode::Lent };
        }
        let kept = key
            .and_then(|k| self.checked.deductions.get(&crate::check::Key::name_of(k)))
            .and_then(|ds| ds.iter().find(|d| d.param == param.name.name))
            .map(|d| d.kept)
            .unwrap_or(true); // default kept (lenient, like deduce.rs)
        if !kept {
            return PassMode::Moved;
        }
        // [ref-handle] A container the fn hands a mutable handle into — the
        // source of a `-> ref(c) Mut T` mint, or the container a `ref(c)
        // Mut` parameter is anchored at [ref-anchor] — is lent mutably,
        // whatever container it is.
        if let Some(decl) = key.and_then(|k| self.fn_by_key(k)) {
            let mints_from = decl.derived_return.as_ref().is_some_and(|d| d.name == param.name.name)
                && decl.return_type.as_ref().is_some_and(crate::mut_lends::fn_type_lends_mut);
            let anchors = decl.params.iter().any(|p| {
                crate::check::ref_anchor_of(&p.ty).is_some_and(|c| c.name == param.name.name)
                    && type_has_mut(&p.ty)
            });
            // A container written through a handle minted in the body (a
            // generic `c: C` read through `?Ref`'s `at`, then mutated).
            // Only a type-parameter container: any other type says itself
            // whether it is lent mutably.
            let generic = matches!(&param.ty, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() && decl.generics.iter().any(|g| g.name == base.name.name));
            let mutated = generic && key.is_some_and(|k| self.checked.param_mutations.get(&crate::check::Key::name_of(k)).is_some_and(|m| m.contains(&param.name.name)));
            if mints_from || anchors || mutated {
                return PassMode::LentMut;
            }
        }
        kept_mode(&param.ty)
    }

    /// The mode of an effect member's parameter (and of the handler members
    /// that implement it), from the clause written on the member.
    fn member_param(&self, member: &FnDecl, p: &Param) -> PassMode {
        if p.variadic || is_fn_group(&p.ty) {
            return PassMode::Moved;
        }
        if matches!(p.ty, Type::Fn { .. }) {
            return PassMode::Lent;
        }
        let moved = member.deductions.iter().flatten().any(|d| {
            d.param_name().is_some_and(|n| n.name == p.name.name)
                // [linear-defer] A deferral is consumed like a move.
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
    fn default_param(&self, ty: &Type, variadic: bool) -> PassMode {
        if variadic || is_fn_group(ty) {
            return PassMode::Moved;
        }
        if matches!(ty, Type::Fn { .. }) {
            return PassMode::Lent;
        }
        kept_mode(ty)
    }

    /// [fn-param-mode] Whether this fn **stores** a callback it is given: a
    /// fn-typed parameter (or an implicit group, which spreads fn
    /// parameters), and a struct with a fn-typed field as the result. That is
    /// the composed-pass shape, and nothing else in the language keeps a
    /// callback past the call.
    fn owns_callbacks(&self, key: DeclId) -> bool {
        let Some(decl) = self.fn_by_key(key) else { return false };
        if !decl.params.iter().any(|p| matches!(p.ty, Type::Fn { .. })) && decl.implicit_groups.is_empty() {
            return false;
        }
        let Some(ret) = decl.return_type.as_ref().and_then(type_base_name) else { return false };
        self.symbols.structs.get(ret).is_some_and(|s| s.fields.iter().any(|f| matches!(f.ty.as_ref(), Some(Type::Fn { .. }))))
    }

    fn fn_by_key(&self, key: DeclId) -> Option<&'a FnDecl> {
        self.checked.decls.fn_decl(&self.program.modules, key)
    }
}

/// A kept parameter: lent mutably when the declared type carries `Mut`,
/// else lent.
fn kept_mode(ty: &Type) -> PassMode {
    if type_has_mut(ty) || type_has_elem_mut(ty) {
        // A container with `Mut` elements lends mutable
        // handles, so it is lent mutably even without its own `Mut`
        // [ref-handle].
        PassMode::LentMut
    } else {
        PassMode::Lent
    }
}

/// A `once` fn group: passes by value.
pub fn is_fn_group(ty: &Type) -> bool {
    matches!(ty, Type::QualifiedGroup { base, .. } if matches!(base.as_ref(), Type::Fn { .. }))
}

/// Whether an AST type carries the `Mut` qualifier.
pub fn type_has_mut(ty: &Type) -> bool {
    match ty {
        Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().any(|q| q.name.name == "Mut")
        }
        Type::Nullable { inner, .. } => type_has_mut(inner),
        // A union whose arm is `Mut` (`Mut A | Mut B`) is mutated through
        // that arm, as an optional is [proj-opt-slot].
        Type::Union { arms, .. } => arms.iter().any(type_has_mut),
        _ => false,
    }
}

/// Whether a container type's **elements** carry `Mut` —
/// `List<Mut T>`, `Mut T[]` — which makes the container lend mutable handles
/// [ref-handle]. Element depth only.
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
