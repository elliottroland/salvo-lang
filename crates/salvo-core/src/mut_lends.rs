//! Mutable lends [ref-handle]: whether a written return type mints a mutable
//! handle into an argument, the Salvo fact a parameter's mode follows from
//! (`param_mode`). How a target spells such a lend is its own business.

use salvo_syntax::ast::*;

use crate::types::is_proj_name;

/// [ref-handle] Whether a fn type's return is a **wholesale mutable lend** —
/// `-> ref(c) Mut T` or its optional: the fn mints a mutable handle into
/// its argument, so that argument is lent mutably. The held form
/// (`-> T holds proj(c)`) is an owned value carrying borrows and is not
/// this case.
pub fn fn_type_lends_mut(ret: &Type) -> bool {
    fn is_top_ref(t: &Type) -> bool {
        match t {
            Type::Named { qualifiers, .. } => qualifiers.iter().any(|q| q.name.name == "ref"),
            Type::QualifiedGroup { qualifiers, .. } => {
                qualifiers.iter().any(|q| q.name.name == "ref")
            }
            _ => false,
        }
    }
    fn wholesale_mut(t: &Type) -> bool {
        let is_ref_here = is_top_ref(t);
        match strip_top_proj_ast(t) {
            Some(inner) => match &inner {
                Type::Named { qualifiers, .. } => {
                    is_ref_here && qualifiers.iter().any(|q| q.name.name == "Mut")
                }
                other => wholesale_mut(other),
            },
            None => match t {
                Type::Nullable { inner, .. } => wholesale_mut(inner),
                _ => false,
            },
        }
    }
    wholesale_mut(ret)
}

/// [proj-type] The type under a *top-level* `proj` (on a named type or a
/// qualified group), or `None` when the type is not a projection at its
/// top. A Copy scalar under `proj` is the scalar itself [copy-scalar-free].
pub fn strip_top_proj_ast(ty: &Type) -> Option<Type> {
    let is_scalar = salvo_syntax::is_copy_scalar_type;
    match ty {
        Type::Named { qualifiers, base } if qualifiers.iter().any(|q| is_proj_name(&q.name.name)) => {
            let rest: Vec<salvo_syntax::ast::TypeRef> = qualifiers
                .iter()
                .filter(|q| !is_proj_name(&q.name.name))
                .cloned()
                .collect();
            let inner = Type::Named {
                qualifiers: rest,
                base: base.clone(),
            };
            if is_scalar(&inner) {
                None
            } else {
                Some(inner)
            }
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } if qualifiers.iter().any(|q| is_proj_name(&q.name.name)) => {
            let rest: Vec<salvo_syntax::ast::TypeRef> = qualifiers
                .iter()
                .filter(|q| !is_proj_name(&q.name.name))
                .cloned()
                .collect();
            if rest.is_empty() {
                Some((**base).clone())
            } else {
                Some(Type::QualifiedGroup {
                    qualifiers: rest,
                    base: base.clone(),
                    span: *span,
                })
            }
        }
        _ => None,
    }
}
