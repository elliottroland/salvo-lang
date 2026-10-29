//! [actor-msg] The declarations an actor needs that no source wrote: the
//! message enum of an actor interface, the private-message and continuation
//! enums of an actor handler, and the functions that deliver a message to
//! the handler's members (IR record §6, step 4). Generated after every module is
//! built, since a handler's faces live in other modules.

use crate::naming::msg_variant_name;
use crate::types::Ty;

use super::Ctx;
use crate::ir::*;

/// The first item index of a generated declaration: far above any source
/// item, so the two never meet.

fn declared_params(m: &FnDecl) -> Vec<&Param> {
    m.params.iter().skip(m.effect_params).take(m.params.len() - m.effect_params - m.implicit_params).collect()
}

/// The type of a generated enum: a key naming its module, since the same
/// name may be generated in two.
pub fn enum_key(name: &str, module: &crate::ModulePath) -> String {
    format!("{name}{}{}", crate::typekey::SEP, module.0.join("."))
}

/// [actor-route-stub] The derived route over protocol `effect` (a type key): its
/// name, after the written name like every derived name (it is placed in the
/// protocol's module, where no other protocol has that name), and its key —
/// keyed by the protocol's module.
pub fn route_name(effect: &str) -> String {
    format!("__Route_{}", crate::typekey::plain(effect).replace('.', "_"))
}

pub fn route_key(effect: &str, module: &crate::ModulePath) -> String {
    enum_key(&route_name(effect), module)
}

/// [actor-route-stub] A `std.net` fn by name and arity, for the derived routes.
fn net_fn(ctx: &Ctx<'_>, name: &str, arity: usize) -> Option<DeclId> {
    for (file_idx, (file, module)) in ctx.program.files.iter().zip(&ctx.program.modules).enumerate() {
        if file.module != crate::ModulePath::std(&["net"]) {
            continue;
        }
        for (item, it) in module.items.iter().enumerate() {
            if let salvo_syntax::ast::Item::Fn(f) = it {
                if f.name.name == name && f.params.iter().filter(|p| !p.implicit).count() == arity {
                    return ctx.resolution.decls.item(file_idx, item);
                }
            }
        }
    }
    None
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
        // [actor-route-stub] A generated declaration is keyed by its module.
        if id.kind() == crate::decl_id::DeclKind::Generated {
            return enum_key(name, id.module());
        }
        ctx.symbols.key_of_id(*id).unwrap_or(name).to_string()
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
        ifaces.iter().position(|i| i.key == *name)
    };

    let mut made: Vec<(usize, EnumDecl)> = Vec::new();
    let mut stubs: Vec<(usize, ImplDecl)> = Vec::new();
    // [decl-id] A generated declaration is identified by its module and its
    // generated name.
    let next_id = |module: &crate::ModulePath, name: &str| -> DeclId { DeclId::generated(module, &[name]) };
    // Phase 1: the message enum of every actor interface.
    let mut message_of: Vec<(DeclId, DeclId, crate::ModulePath)> = Vec::new(); // interface, enum, module
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
            let id = next_id(&m.path, &format!("__Msg_{}", i.name));
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
                    let reads: Vec<Expr> = x.params.iter().map(|p| Expr { ty: p.ty.clone(), span: x.span, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: true, moved: false } }).collect();
                    let to = Expr { ty: addr_ty.clone(), span: x.span, kind: ExprKind::Read { place: Place { root: addr.clone(), steps: Vec::new() }, consume: false, moved: false } };
                    let send = Expr { ty: Ty::none(), span: x.span, kind: ExprKind::Send { addr: Box::new(to), member: MemberRef { interface: i.id.clone(), index }, args: reads } };
                    FnDecl {
                        id: DeclId::generated(&m.path, &[&format!("__Stub_{}", i.name), &x.name]),
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
                        ref_anchors: Vec::new(),
                        throws: None,
                        result_check: None,
                        factories: None,
                        body: Some(Block { stmts: vec![Stmt::Expr(send)], value: None, span: x.span }),
                        span: x.span,
                        fate: FateFacts::default(),
                    }
                })
                .collect();
            stubs.push((
                mi,
                ImplDecl {
                    id: next_id(&m.path, &format!("__Stub_{}", i.name)),
                    name: format!("__Stub_{}", i.name),
                    exported: false,
                    type_params: Vec::new(),
                    faces: vec![face],
                    face_any: vec![false],
                    ctor_params: vec![Param { local: addr, ty: addr_ty, mode: crate::param_mode::PassMode::Moved, variadic: false, check: None, span: i.span }],
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
    // [actor-route-stub] Phase 1b: the route over every protocol a `use any`
    // routes (user decision R7), one per protocol, in the protocol's module
    // — an ordinary actor-faced impl from here on, so Phase 2 gives it its
    // dispatch. Each member asks the selector for a member and forwards:
    //   let __pick = route_pick(group, seen[, key_hash(k)])
    //   seen = __pick.version
    //   send X#m(__pick.to, params…)
    let mut routed: Vec<&String> = ctx.checked.route_uses.values().collect();
    routed.sort();
    routed.dedup();
    let pick_plain = net_fn(ctx, "route_pick", 2);
    let pick_keyed = net_fn(ctx, "route_pick", 3);
    let key_hash = net_fn(ctx, "key_hash", 1);
    let mut routes: Vec<(usize, ImplDecl)> = Vec::new();
    for effect in routed {
        let Some(ii) = ifaces.iter().position(|i| i.key == *effect && i.actor) else { continue };
        let Some(mi) = program.modules.iter().position(|m| m.path == *ifaces[ii].id.module()) else { continue };
        let Some(Decl::Interface(iface)) = program.modules[mi].decls.iter().find(|d| matches!(d, Decl::Interface(x) if x.id == ifaces[ii].id)) else { continue };
        let span = iface.span;
        let face = Ty::Named { name: effect.clone(), args: Vec::new() };
        let addr_of = |t: Ty| Ty::Named { name: "Addr".into(), args: vec![t] };
        let group_ty = addr_of(Ty::Named { name: "ActorGroup".into(), args: vec![face.clone()] });
        let selector = Ty::Named { name: "RouteSelector".into(), args: vec![face.clone()] };
        let pick_ty = Ty::Named { name: "RoutePick".into(), args: vec![face.clone()] };
        let long = Ty::named("Long");
        let read = |l: &str, ty: &Ty, steps: Vec<Step>, consume: bool| Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: Local(l.into()), steps }, consume, moved: false } };
        let keys = ctx.checked.route_keys.get(effect.as_str());
        let mut members: Vec<FnDecl> = Vec::new();
        for (index, x) in iface.members.iter().enumerate().filter(|(_, x)| x.send) {
            let key_param = keys.and_then(|k| k.get(&x.name)).and_then(|n| x.params.iter().find(|p| p.local.0 == *n));
            let mut args = vec![read("__dep0", &selector, Vec::new(), false), read("group", &group_ty, Vec::new(), false), read("seen", &long, Vec::new(), false)];
            let target = match key_param {
                Some(p) => {
                    args.push(Expr { ty: long.clone(), span, kind: ExprKind::Call { target: FnRef::Decl(key_hash.clone().unwrap_or_else(|| ifaces[ii].id.clone())), type_args: vec![p.ty.clone()], args: vec![read(&p.local.0, &p.ty, Vec::new(), false)], moved: false } });
                    pick_keyed.clone()
                }
                None => pick_plain.clone(),
            };
            let Some(target) = target else { continue };
            let pick = Expr { ty: pick_ty.clone(), span, kind: ExprKind::Call { target: FnRef::Decl(target), type_args: vec![face.clone()], args, moved: false } };
            let to = read("__pick", &addr_of(face.clone()), vec![Step::Field("to".into())], false);
            let forward: Vec<Expr> = x.params.iter().map(|p| read(&p.local.0, &p.ty, Vec::new(), true)).collect();
            let stmts = vec![
                Stmt::Let { id: NodeId(1), local: Local("__pick".into()), ty: pick_ty.clone(), value: pick },
                Stmt::Assign { place: Place { root: Local("seen".into()), steps: Vec::new() }, value: read("__pick", &long, vec![Step::Field("version".into())], false) },
                Stmt::Expr(Expr { ty: Ty::none(), span, kind: ExprKind::Send { addr: Box::new(to), member: MemberRef { interface: iface.id.clone(), index }, args: forward } }),
            ];
            members.push(FnDecl {
                id: DeclId::generated(iface.id.module(), &[&route_name(effect), &x.name]),
                name: x.name.clone(),
                exported: false,
                kind: FnKind::Member { faces: 0 },
                send: true,
                type_params: Vec::new(),
                params: x.params.clone(),
                effect_params: 0,
                implicit_params: 0,
                ret: Ty::none(),
                borrows: Vec::new(),
                holds: Vec::new(),
                ref_anchors: Vec::new(),
                throws: None,
                result_check: None,
                factories: None,
                body: Some(Block { stmts, value: None, span }),
                span,
                fate: FateFacts::default(),
            });
        }
        let param = |name: &str, ty: Ty| Param { local: Local(name.into()), ty, mode: crate::param_mode::PassMode::Lent, variadic: false, check: None, span };
        let mailbox = Expr { ty: Ty::named("Mailbox"), span, kind: ExprKind::Construct { fields: vec![("capacity".into(), Expr { ty: Ty::named("Int"), span, kind: ExprKind::Int(1) })] } };
        routes.push((
            mi,
            ImplDecl {
                id: next_id(iface.id.module(), &route_name(effect)),
                name: route_name(effect),
                exported: true,
                type_params: Vec::new(),
                faces: vec![face],
                face_any: vec![true],
                ctor_params: vec![param("group", group_ty)],
                deps: vec![selector],
                state: vec![Field { name: "seen".into(), ty: long.clone(), canbe_mut: false, default: Some(Expr { ty: long, span, kind: ExprKind::Long(-1) }) }],
                mailbox: Some(mailbox),
                init: None,
                members,
                stateful: true,
                platform: false,
                threadsafe: false,
                intrinsic: false,
                dispatch: Vec::new(),
                stub: false,
                span,
            },
        ));
    }
    for (mi, r) in routes {
        program.modules[mi].decls.push(Decl::Impl(r));
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
                // [actor-send-fn] An overloaded send member cannot be a mint's
                // target (the checker refuses it, PIPELINE.md B3.7), so it has
                // no continuation — and a plain-named one would collide. The
                // backends' resume tables skip it alike.
                let sends = &ifaces[*i].sends;
                for s in sends.iter().filter(|s| sends.iter().filter(|o| o == s).count() == 1) {
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
            made.push((mi, EnumDecl { id: next_id(&m.path, &format!("__Cont_{}", h.name)), name: format!("__Cont_{}", h.name), kind: EnumKind::Continuation { handler: h.id.clone() }, variants: cont, protocol_hash: None, span: h.span }));
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
                let name = format!("__Priv_{}", h.name);
                let eid = next_id(&m.path, &name);
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
                let fid = next_id(&m.path, &fname);
                let enum_ty = Ty::Named { name: ekey, args: Vec::new() };
                let (handler_l, msg_l) = (Local("__handler".into()), Local("__msg".into()));
                let read = |l: &Local, ty: &Ty| Expr { ty: ty.clone(), span: h.span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false, moved: false } };
                let sw_id = NodeId(1);
                let mut arms = Vec::new();
                for (vi, (_, member, fields)) in variants.iter().enumerate() {
                    let locals: Vec<(Local, Ty)> = fields.iter().map(|(n, t)| (Local(n.clone()), t.clone())).collect();
                    let args: Vec<Expr> = locals.iter().map(|(l, t)| Expr { ty: t.clone(), span: h.span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: true, moved: false } }).collect();
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
                    arms.push(SwitchArm { test: ArmTest::Arm(vi), body: Block { stmts: vec![unpack, Stmt::Expr(call)], value: None, span: h.span } });
                }
                let switch = Expr { ty: Ty::none(), span: h.span, kind: ExprKind::Switch { id: sw_id, subject: Box::new(read(&msg_l, &enum_ty)), arms } };
                let param = |local: Local, ty: Ty, mode: crate::param_mode::PassMode| Param { local, ty, mode, variadic: false, check: None, span: h.span };
                fns.push((
                    mi,
                    FnDecl {
                        id: fid.clone(),
                        name: fname.clone(),
                        exported: false,
                        kind: FnKind::Plain,
                        send: false,
                        type_params: Vec::new(),
                        params: vec![param(handler_l, handler_ty.clone(), crate::param_mode::PassMode::LentMut), param(msg_l, enum_ty, crate::param_mode::PassMode::Moved)],
                        effect_params: 0,
                        implicit_params: 0,
                        ret: Ty::none(),
                        borrows: Vec::new(),
                        holds: Vec::new(),
                        ref_anchors: Vec::new(),
                        throws: None,
                        result_check: None,
                        factories: None,
                        body: Some(Block { stmts: vec![Stmt::Expr(switch)], value: None, span: h.span }),
                        span: h.span,
                        fate: FateFacts::default(),
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
