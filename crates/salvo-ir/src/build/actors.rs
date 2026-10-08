//! [actor-msg] The declarations an actor needs that no source wrote: the
//! message enum of an actor interface, and the private-message and
//! continuation enums of an actor handler (IR.md §6, step 4). Generated after
//! every module is built, since a handler's faces live in other modules.

use salvo_core::naming::msg_variant_name;
use salvo_core::types::Ty;

use super::Ctx;
use crate::ir::*;

/// The first item index of a generated declaration: far above any source
/// item, so the two never meet.
const GENERATED_BASE: usize = usize::MAX / 2;

fn declared_params(m: &FnDecl) -> Vec<&Param> {
    m.params.iter().skip(m.effect_params).take(m.params.len() - m.effect_params - m.implicit_params).collect()
}

pub fn generate(ctx: &Ctx<'_>, program: &mut Program) {
    // The interface a face names, wherever it is declared.
    let key_of = |id: &DeclId, name: &str| -> String {
        for (k, m) in &ctx.symbols.key_modules {
            if **m == id.module && salvo_core::typekey::plain(k) == name {
                return k.to_string();
            }
        }
        name.to_string()
    };
    struct Iface {
        key: String,
        name: String,
        actor: bool,
        sends: Vec<String>,
    }
    let mut ifaces: Vec<Iface> = Vec::new();
    for m in &program.modules {
        for d in &m.decls {
            if let Decl::Interface(i) = d {
                ifaces.push(Iface { key: key_of(&i.id, &i.name), name: i.name.clone(), actor: i.actor, sends: i.members.iter().filter(|x| x.send).map(|x| x.name.clone()).collect() });
            }
        }
    }
    let find = |face: &Ty| -> Option<usize> {
        let Ty::Named { name, .. } = face.strip_quals() else { return None };
        ifaces.iter().position(|i| i.key == *name).or_else(|| ifaces.iter().position(|i| i.name == salvo_core::typekey::plain(name)))
    };

    let mut made: Vec<(usize, EnumDecl)> = Vec::new();
    let mut counter = 0usize;
    let mut next_id = |module: &salvo_core::ModulePath| -> DeclId {
        counter += 1;
        DeclId { module: module.clone(), item: GENERATED_BASE + counter, sub: 0 }
    };
    for (mi, m) in program.modules.iter().enumerate() {
        for d in &m.decls {
            match d {
                // The messages of an actor interface.
                Decl::Interface(i) if i.actor => {
                    let sends: Vec<&Member> = i.members.iter().filter(|x| x.send).collect();
                    if sends.is_empty() {
                        continue;
                    }
                    let variants = sends
                        .iter()
                        .map(|x| Variant {
                            name: msg_variant_name(&x.emitted_name),
                            member: Some(x.emitted_name.clone()),
                            fields: x.params.iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect(),
                        })
                        .collect();
                    made.push((
                        mi,
                        EnumDecl {
                            id: next_id(&m.path),
                            name: format!("__Msg_{}", i.name),
                            kind: EnumKind::Message { interface: i.id.clone() },
                            variants,
                            protocol_hash: i.protocol_hash.clone(),
                            span: i.span,
                        },
                    ));
                }
                // The private and continuation messages of an actor handler.
                Decl::Impl(h) if !h.platform && !h.intrinsic => {
                    let faces: Vec<usize> = h.faces.iter().filter_map(|f| find(f)).filter(|i| ifaces[*i].actor).collect();
                    let mixed = !h.faces.is_empty() && h.members.iter().any(|x| x.send) && faces.is_empty();
                    if faces.is_empty() && !mixed {
                        continue;
                    }
                    let face_sends = |name: &str| faces.iter().any(|i| ifaces[*i].sends.iter().any(|s| s == name));
                    let privs: Vec<&FnDecl> = h.members.iter().filter(|x| x.send && !face_sends(&x.name)).collect();
                    // Every send member that can be resumed: those with a
                    // declared parameter (the answer is the last).
                    let mut parking: Vec<&FnDecl> = Vec::new();
                    for i in &faces {
                        for s in &ifaces[*i].sends {
                            if let Some(x) = h.members.iter().find(|x| &x.name == s) {
                                if !declared_params(x).is_empty() {
                                    parking.push(x);
                                }
                            }
                        }
                    }
                    parking.extend(privs.iter().copied().filter(|x| !declared_params(x).is_empty()));
                    let mut private = Vec::new();
                    if h.init.is_some() {
                        private.push(Variant { name: "Init".into(), member: None, fields: Vec::new() });
                    }
                    for x in &privs {
                        private.push(Variant {
                            name: msg_variant_name(&x.name),
                            member: Some(x.name.clone()),
                            fields: declared_params(x).iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect(),
                        });
                    }
                    let cont: Vec<Variant> = parking
                        .iter()
                        .map(|x| {
                            let ps = declared_params(x);
                            Variant {
                                name: msg_variant_name(&x.name),
                                member: Some(x.name.clone()),
                                fields: ps[..ps.len() - 1].iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect(),
                            }
                        })
                        .collect();
                    made.push((
                        mi,
                        EnumDecl { id: next_id(&m.path), name: format!("__Cont_{}", h.name), kind: EnumKind::Continuation { handler: h.id.clone() }, variants: cont, protocol_hash: None, span: h.span },
                    ));
                    if !private.is_empty() {
                        made.push((
                            mi,
                            EnumDecl { id: next_id(&m.path), name: format!("__Priv_{}", h.name), kind: EnumKind::Private { handler: h.id.clone() }, variants: private, protocol_hash: None, span: h.span },
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    for (mi, e) in made {
        program.modules[mi].decls.push(Decl::Enum(e));
    }
}
