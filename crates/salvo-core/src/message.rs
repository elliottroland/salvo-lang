//! [actor-message] The messages of an actor effect, as values.
//!
//! A `send fn` is sugar for a message the actor machinery already carries:
//! the IR's `__Msg_E` enum, one variant per send member (user decision
//! 2026-11-05, ROUTES.md R2). `Message<E>` makes that message a Salvo value:
//! a union with one arm per send member, each arm a struct of the member's
//! parameters. The arms are ordinary structs this module declares beside the
//! effect, before resolution, so every later phase (resolution, comptime,
//! the checker, the IR, both backends) reads them as the declarations they
//! are: `Inventory.Reserve` for `Inventory`'s `reserve`, named after the
//! effect as a namespace member is, in the CamelCase the message enum's
//! variants already use (`naming::msg_variant_name`), since a lowercase
//! segment after a dot is a field read [name-dot].

use salvo_syntax::ast::{EffectDecl, FieldDecl, Ident, Item, Module, StructDecl};

/// The arm struct's name for send member `idx` of `effect`: `E.Reserve`.
pub fn arm_name(effect: &EffectDecl, idx: usize) -> String {
    format!("{}.{}", effect.name.name, crate::naming::msg_variant_name(&crate::effect_member_name(effect, idx)))
}

/// [actor-message] Declares the arm structs of every non-generic actor effect
/// the module declares, after its declarations. A generic protocol has no
/// messages of its own to declare: its members' types are not known until it
/// is instantiated.
pub fn declare_arms(module: &mut Module) {
    let mut arms = Vec::new();
    for item in &module.items {
        let Item::Effect(e) = item else { continue };
        if !e.is_actor || !e.generics.is_empty() {
            continue;
        }
        for (idx, f) in e.fns.iter().enumerate().filter(|(_, f)| f.is_send) {
            let fields: Vec<FieldDecl> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| FieldDecl {
                    docs: Vec::new(),
                    annotations: p.annotations.clone(),
                    name: p.name.clone(),
                    ty: Some(p.ty.clone()),
                    canbe_mut: false,
                    default: None,
                    span: p.span,
                })
                .collect();
            arms.push(Item::Struct(StructDecl {
                docs: f.docs.clone(),
                annotations: Vec::new(),
                exported: e.exported,
                name: Ident { name: arm_name(e, idx), span: f.name.span },
                generics: Vec::new(),
                generic_canbe: Vec::new(),
                fn_slots: Vec::new(),
                effect_generics: Vec::new(),
                obligations: Vec::new(),
                auto_qualifiers: Vec::new(),
                fields,
                fns: Vec::new(),
                refns: Vec::new(),
                linear: false,
                opaque: false,
                noremote: false,
                comptime: false,
                message_arm: Some(e.name.clone()),
                span: f.span,
            }));
        }
    }
    module.items.extend(arms);
}
