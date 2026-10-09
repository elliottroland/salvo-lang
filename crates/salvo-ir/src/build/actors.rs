//! [actor-msg] The declarations an actor needs that no source wrote: the
//! message enum of an actor interface, the private-message and continuation
//! enums of an actor handler, and the functions that deliver a message to
//! the handler's members (IR record §6, step 4). Generated after every module is
//! built, since a handler's faces live in other modules.

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

/// The type of a generated enum: a key naming its module, since the same
/// name may be generated in two.
pub fn enum_key(name: &str, module: &salvo_core::ModulePath) -> String {
    format!("{name}{}{}", salvo_core::typekey::SEP, module.0.join("."))
}

struct Iface {
    key: String,
    name: String,
    id: DeclId,
    actor: bool,
    sends: Vec<String>,
}

pub fn generate(ctx: &Ctx<'_>, program: &mut Program) -> Vec<(DeclId, String)> {
    // The interface a face names, wherever it is declared.
    let key_of = |id: &DeclId, name: &str| -> String {
        for (k, m) in &ctx.symbols.key_modules {
            if **m == id.module && salvo_core::typekey::plain(k) == name {
                return k.to_string();
            }
        }
        name.to_string()
    };
    let mut ifaces: Vec<Iface> = Vec::new();
    for m in &program.modules {
        for d in &m.decls {
            if let Decl::Interface(i) = d {
                ifaces.push(Iface { key: key_of(&i.id, &i.name), name: i.name.clone(), id: i.id.clone(), actor: i.actor, sends: i.members.iter().filter(|x| x.send).map(|x| x.name.clone()).collect() });
            }
        }
    }
    let find = |face: &Ty| -> Option<usize> {
        let Ty::Named { name, .. } = face.strip_quals() else { return None };
        ifaces.iter().position(|i| i.key == *name).or_else(|| ifaces.iter().position(|i| i.name == salvo_core::typekey::plain(name)))
    };

    let mut made: Vec<(usize, EnumDecl)> = Vec::new();
    let mut stubs: Vec<(usize, ImplDecl)> = Vec::new();
    let mut counter = 0usize;
    let mut next_id = |module: &salvo_core::ModulePath| -> DeclId {
        counter += 1;
        DeclId { module: module.clone(), item: GENERATED_BASE + counter, sub: 0 }
    };
    // Phase 1: the message enum of every actor interface.
    let mut message_of: Vec<(DeclId, DeclId, salvo_core::ModulePath)> = Vec::new(); // interface, enum, module
    for (mi, m) in program.modules.iter().enumerate() {
        for d in &m.decls {
            let Decl::Interface(i) = d else { continue };
            if !i.actor {
                continue;
            }
            let sends: Vec<&Member> = i.members.iter().filter(|x| x.send).collect();
            if sends.is_empty() {
                continue;
            }
            let variants = sends
                .iter()
                .map(|x| Variant { name: msg_variant_name(&x.emitted_name), member: Some(x.emitted_name.clone()), fields: x.params.iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect() })
                .collect();
            let id = next_id(&m.path);
            message_of.push((i.id.clone(), id.clone(), m.path.clone()));
            made.push((mi, EnumDecl { id, name: format!("__Msg_{}", i.name), kind: EnumKind::Message { interface: i.id.clone() }, variants, protocol_hash: i.protocol_hash.clone(), span: i.span }));
            // [actor-use-addr] The stub: the interface implemented by sending.
            let face = Ty::Named { name: key_of(&i.id, &i.name), args: Vec::new() };
            let addr_ty = Ty::Named { name: "Addr".into(), args: vec![face.clone()] };
            let addr = Local("addr".into());
            let members: Vec<FnDecl> = i
                .members
                .iter()
                .enumerate()
                .filter(|(_, x)| x.send)
                .map(|(index, x)| {
                    let reads: Vec<Expr> = x.params.iter().map(|p| Expr { ty: p.ty.clone(), span: x.span, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: true } }).collect();
                    let to = Expr { ty: addr_ty.clone(), span: x.span, kind: ExprKind::Read { place: Place { root: addr.clone(), steps: Vec::new() }, consume: false } };
                    let send = Expr { ty: Ty::none(), span: x.span, kind: ExprKind::Send { addr: Box::new(to), member: MemberRef { interface: i.id.clone(), index }, args: reads } };
                    FnDecl {
                        id: DeclId { module: m.path.clone(), item: GENERATED_BASE + 1_000_000 + index, sub: 0 },
                        name: x.name.clone(),
                        exported: false,
                        kind: FnKind::Member { faces: 0 },
                        send: false,
                        type_params: Vec::new(),
                        params: x.params.clone(),
                        effect_params: 0,
                        implicit_params: 0,
                        ret: Ty::none(),
                        borrows: Vec::new(),
                        holds: Vec::new(),
                        throws: None,
                        result_check: None,
                        factories: None,
                        body: Some(Block { stmts: vec![Stmt::Expr(send)], value: None }),
                        span: x.span,
                    }
                })
                .collect();
            stubs.push((
                mi,
                ImplDecl {
                    id: next_id(&m.path),
                    name: format!("__Stub_{}", i.name),
                    exported: false,
                    type_params: Vec::new(),
                    faces: vec![face],
                    face_any: vec![false],
                    ctor_params: vec![Param { local: addr, ty: addr_ty, mode: salvo_core::param_mode::PassMode::Moved, variadic: false, check: None }],
                    deps: Vec::new(),
                    state: Vec::new(),
                    mailbox: None,
                    init: None,
                    members,
                    stateful: false,
                    platform: false,
                    threadsafe: false,
                    intrinsic: false,
                    dispatch: Vec::new(),
                    stub: true,
                    span: i.span,
                },
            ));
        }
    }
    // Phase 2: an actor handler's own enums, and its dispatch functions.
    let mut dispatched: Vec<(usize, usize, Vec<Dispatch>)> = Vec::new();
    let mut fns: Vec<(usize, FnDecl)> = Vec::new();
    let mut names: Vec<(DeclId, String)> = Vec::new();
    for (mi, m) in program.modules.iter().enumerate() {
        for (di, d) in m.decls.iter().enumerate() {
            let Decl::Impl(h) = d else { continue };
            if h.platform || h.intrinsic || h.stub {
                continue;
            }
            let faces: Vec<usize> = h.faces.iter().filter_map(|f| find(f)).filter(|i| ifaces[*i].actor).collect();
            let mixed = !h.faces.is_empty() && h.members.iter().any(|x| x.send) && faces.is_empty();
            if faces.is_empty() && !mixed {
                continue;
            }
            let face_sends = |name: &str| faces.iter().any(|i| ifaces[*i].sends.iter().any(|s| s == name));
            let privs: Vec<&FnDecl> = h.members.iter().filter(|x| x.send && !face_sends(&x.name)).collect();
            // Every send member that can be resumed: those with a declared
            // parameter (the answer is the last).
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
                private.push(Variant { name: msg_variant_name(&x.name), member: Some(x.name.clone()), fields: declared_params(x).iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect() });
            }
            let cont: Vec<Variant> = parking
                .iter()
                .map(|x| {
                    let ps = declared_params(x);
                    Variant { name: msg_variant_name(&x.name), member: Some(x.name.clone()), fields: ps[..ps.len() - 1].iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect() }
                })
                .collect();
            made.push((mi, EnumDecl { id: next_id(&m.path), name: format!("__Cont_{}", h.name), kind: EnumKind::Continuation { handler: h.id.clone() }, variants: cont, protocol_hash: None, span: h.span }));
            let handler_ty = Ty::Named { name: key_of(&h.id, &h.name), args: Vec::new() };
            let mut mine: Vec<Dispatch> = Vec::new();
            // One dispatcher per face with messages, one for the private ones.
            let mut targets: Vec<(String, DeclId, String, Option<usize>)> = Vec::new(); // fn name, enum id, enum key, face
            for i in &faces {
                if let Some((_, eid, emod)) = message_of.iter().find(|(iid, _, _)| *iid == ifaces[*i].id) {
                    targets.push((format!("__dispatch_{}_{}", h.name, ifaces[*i].name), eid.clone(), enum_key(&format!("__Msg_{}", ifaces[*i].name), emod), Some(*i)));
                }
            }
            if !private.is_empty() {
                let eid = next_id(&m.path);
                let name = format!("__Priv_{}", h.name);
                targets.push((format!("__dispatch_priv_{}", h.name), eid.clone(), enum_key(&name, &m.path), None));
                made.push((mi, EnumDecl { id: eid, name, kind: EnumKind::Private { handler: h.id.clone() }, variants: private, protocol_hash: None, span: h.span }));
            }
            for (fname, eid, ekey, face) in targets {
                // The variants, from the enum just made or the interface's.
                let variants: Vec<(String, Option<String>, Vec<(String, Ty)>)> = made
                    .iter()
                    .find(|(_, e)| e.id == eid)
                    .map(|(_, e)| e.variants.iter().map(|v| (v.name.clone(), v.member.clone(), v.fields.clone())).collect())
                    .unwrap_or_default();
                let fid = next_id(&m.path);
                let enum_ty = Ty::Named { name: ekey, args: Vec::new() };
                let (handler_l, msg_l) = (Local("__handler".into()), Local("__msg".into()));
                let read = |l: &Local, ty: &Ty| Expr { ty: ty.clone(), span: h.span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false } };
                let sw_id = NodeId(1);
                let mut arms = Vec::new();
                for (vi, (_, member, fields)) in variants.iter().enumerate() {
                    let locals: Vec<(Local, Ty)> = fields.iter().map(|(n, t)| (Local(n.clone()), t.clone())).collect();
                    let args: Vec<Expr> = locals.iter().map(|(l, t)| Expr { ty: t.clone(), span: h.span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: true } }).collect();
                    let target = match (face, member) {
                        (Some(i), Some(name)) => {
                            let iface = &ifaces[i];
                            // The interface's member index.
                            let index = program.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
                                Decl::Interface(x) if x.id == iface.id => x.members.iter().position(|mm| &mm.emitted_name == name),
                                _ => None,
                            });
                            match index {
                                Some(index) => HandlerMember::Face(MemberRef { interface: iface.id.clone(), index }),
                                None => HandlerMember::Own(name.clone()),
                            }
                        }
                        (None, Some(name)) => HandlerMember::Own(name.clone()),
                        (_, None) => HandlerMember::Own("init".into()),
                    };
                    let call = Expr { ty: Ty::none(), span: h.span, kind: ExprKind::HandlerCall { instance: Box::new(read(&handler_l, &handler_ty)), member: target, args } };
                    let unpack = Stmt::Unpack { id: NodeId(2 + vi as u32), locals, from: Place { root: msg_l.clone(), steps: Vec::new() }, from_ty: enum_ty.clone(), variant: vi };
                    arms.push(SwitchArm { test: ArmTest::Arm(vi), body: Block { stmts: vec![unpack, Stmt::Expr(call)], value: None } });
                }
                let switch = Expr { ty: Ty::none(), span: h.span, kind: ExprKind::Switch { id: sw_id, subject: Box::new(read(&msg_l, &enum_ty)), arms } };
                let param = |local: Local, ty: Ty, mode: salvo_core::param_mode::PassMode| Param { local, ty, mode, variadic: false, check: None };
                fns.push((
                    mi,
                    FnDecl {
                        id: fid.clone(),
                        name: fname.clone(),
                        exported: false,
                        kind: FnKind::Plain,
                        send: false,
                        type_params: Vec::new(),
                        params: vec![param(handler_l, handler_ty.clone(), salvo_core::param_mode::PassMode::LentMut), param(msg_l, enum_ty, salvo_core::param_mode::PassMode::Moved)],
                        effect_params: 0,
                        implicit_params: 0,
                        ret: Ty::none(),
                        borrows: Vec::new(),
                        holds: Vec::new(),
                        throws: None,
                        result_check: None,
                        factories: None,
                        body: Some(Block { stmts: vec![Stmt::Expr(switch)], value: None }),
                        span: h.span,
                    },
                ));
                names.push((fid.clone(), fname));
                mine.push(Dispatch { message: eid, func: fid });
            }
            dispatched.push((mi, di, mine));
        }
    }
    for (mi, e) in made {
        program.modules[mi].decls.push(Decl::Enum(e));
    }
    for (mi, s) in stubs {
        program.modules[mi].decls.push(Decl::Impl(s));
    }
    for (mi, f) in fns {
        program.modules[mi].decls.push(Decl::Fn(f));
    }
    for (mi, di, mine) in dispatched {
        if let Decl::Impl(h) = &mut program.modules[mi].decls[di] {
            h.dispatch = mine;
        }
    }
    names
}
