//! [actor-msg] The declarations an actor needs that no source wrote: the
//! message enum of an actor interface, the private-message and continuation
//! enums of an actor handler, and the functions that deliver a message to
//! the handler's members (IR record §6, step 4). Generated after every module is
//! built, since a handler's faces live in other modules.

use crate::naming::msg_variant_name;
use crate::types::Ty;
use salvo_syntax::Span;
use salvo_syntax::known;

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

struct Iface {
    key: String,
    name: String,
    id: DeclId,
    actor: bool,
    sends: Vec<String>,
}

pub fn generate(ctx: &Ctx<'_>, program: &mut Program) -> Vec<(DeclId, String)> {
    // [actor-sent] Phase 0: a handler generic over the effect it serves
    // (`handler Relay<E>(…) for E`) is built once per effect it is
    // constructed at, as an ordinary handler of that effect.
    instantiate_generic_faces(ctx, program);
    // [actor-sent] A message `send` the instantiations did not lower is over
    // a protocol only a type parameter names (a generic fn's `send(to, m)`):
    // refused, never emitted — a backend would read it as the reply's
    // `send`, its overload of the same name.
    crate::ir::walk::exprs_mut(program, &mut |e: &mut Expr| {
        if let ExprKind::Call { target: FnRef::Decl(d), args, .. } = &e.kind {
            if is_forward(d) && args.len() == 2 {
                *e = crate::ir::build::unsupported(
                    e.ty.clone(),
                    e.span,
                    "a message `send` over a protocol a type parameter stands for: send from a handler `for E`, which is built for each protocol it serves",
                );
            }
        }
    });
    // The interface a face names, wherever it is declared.
    let key_of = |id: &DeclId, name: &str| -> String {
        // [actor-router] A generated declaration is keyed by its module.
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
    // [actor-sent] Phase 1c: a handler that takes a face's messages with
    // `send fn sent(m: Message<F>)` answers each of `F`'s send members
    // with a member of its own: the member's parameters made into its arm,
    // bound to `sent`'s parameter, then `sent`'s body. From here on
    // the handler is an ordinary one: Phase 2 gives it its dispatch, and a
    // `use` of it delegates member by member as to any handler.
    for mi in 0..program.modules.len() {
        for di in 0..program.modules[mi].decls.len() {
            let Decl::Impl(h) = &program.modules[mi].decls[di] else { continue };
            if h.platform || h.intrinsic || h.stub {
                continue;
            }
            let Some(ri) = h.members.iter().position(|f| f.send && f.name == known::SENT_MEMBER && declared_params(f).len() == 1) else { continue };
            let receive = &h.members[ri];
            let rparam = declared_params(receive)[0].clone();
            // The face it answers: the one whose messages its parameter is.
            let face = h.faces.iter().find_map(|f| {
                let Ty::Named { name, args } = f.strip_quals() else { return None };
                if !args.is_empty() || !ctx.symbols.message_arms.contains_key(name.as_str()) {
                    return None;
                }
                let msg = crate::check::expand_message_in(ctx.symbols, Ty::Named { name: known::MESSAGE_TYPE.into(), args: vec![f.strip_quals().clone()] });
                (super::erase(&msg).strip_quals() == rparam.ty.strip_quals()).then(|| name.clone())
            });
            let Some(face) = face else { continue };
            let Some(ii) = ifaces.iter().position(|i| i.key == face && i.actor) else { continue };
            let Some(iface) = program.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
                Decl::Interface(x) if x.id == ifaces[ii].id => Some(x),
                _ => None,
            }) else {
                continue;
            };
            let arms = ctx.symbols.message_arms.get(face.as_str()).cloned().unwrap_or_default();
            let union_ty = rparam.ty.clone();
            let mut made_members: Vec<FnDecl> = Vec::new();
            for (k, x) in iface.members.iter().filter(|x| x.send).enumerate() {
                let Some(arm_key) = arms.get(k) else { continue };
                let Some(arm) = ctx.symbols.structs.get(arm_key) else { continue };
                let arm_ty = Ty::Named { name: (*arm_key).to_string(), args: Vec::new() };
                // Where `sent` takes the message: what a diagnostic about
                // it names.
                let span = rparam.span;
                // Fresh parameter names: `sent`'s body is spliced after them.
                let params: Vec<Param> = x.params.iter().enumerate().map(|(j, p)| Param { local: Local(format!("__msg_p{j}")), ..p.clone() }).collect();
                let fields: Vec<(String, Expr)> = arm
                    .fields
                    .iter()
                    .zip(&params)
                    .map(|(f, p)| (f.name.name.clone(), Expr { ty: p.ty.clone(), span, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: true, moved: false } }))
                    .collect();
                let built = Expr { ty: arm_ty.clone(), span, kind: ExprKind::Construct { fields } };
                let value = if arms.len() == 1 { built } else { Expr { ty: union_ty.clone(), span, kind: ExprKind::MakeUnion { arm: k, value: Box::new(built) } } };
                let mut body = receive.body.clone().unwrap_or(Block { stmts: Vec::new(), value: None, span });
                body.stmts.insert(0, Stmt::Let { id: NodeId(u32::MAX - 1), local: rparam.local.clone(), ty: union_ty.clone(), value });
                made_members.push(FnDecl {
                    id: DeclId::generated(h.id.module(), &[&h.name, &x.emitted_name]),
                    name: x.name.clone(),
                    exported: receive.exported,
                    kind: FnKind::Member { faces: 1 },
                    send: true,
                    type_params: Vec::new(),
                    params,
                    effect_params: 0,
                    implicit_params: 0,
                    ret: Ty::none(),
                    borrows: Vec::new(),
                    holds: Vec::new(),
                    ref_anchors: Vec::new(),
                    throws: receive.throws.clone(),
                    result_check: None,
                    factories: None,
                    body: Some(body),
                    span: receive.span,
                    fate: receive.fate.clone(),
                });
            }
            if let Decl::Impl(h) = &mut program.modules[mi].decls[di] {
                h.members.remove(ri);
                h.members.extend(made_members);
            }
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
                // [actor-send-fn] An overloaded send member cannot be a mint's
                // target (the checker refuses it, the pipeline record B3.7), so it has
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

/// [actor-sent] Whether `decl` is std's message `send(to: eff[E], m:
/// Message<E>)` — not its overload beside it, the reply's `send(reply,
/// value)`, which the overload's signature tells apart.
pub(crate) fn is_forward(decl: &DeclId) -> bool {
    decl.name() == known::MESSAGE_SEND_FN
        && *decl.module() == crate::ModulePath::std(&["core", "actor"])
        && decl.sig().is_some_and(|s| s.contains(known::MESSAGE_TYPE))
}

/// [actor-sent] The effect key of an address type: `E` of `eff[E]`, for a
/// concrete `E`.
pub(crate) fn addr_effect_key(ty: &Ty) -> Option<String> {
    match ty.strip_quals() {
        Ty::Named { name, args } if name == "Addr" && args.len() == 1 => match args[0].strip_quals() {
            Ty::Named { name, args } if args.is_empty() => Some(name.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// [actor-sent] `send(to, m)` as statements: `to` and `m` bound once,
/// then the send of the member `m` is a message of, with `m`'s fields as its
/// arguments — a `Switch` over the arms, each narrowed and sent; one arm is
/// sent as it is. The effect is `to`'s, which must be concrete.
pub(crate) fn forward_stmts(
    ctx: &Ctx<'_>,
    file_idx: usize,
    to: Expr,
    m: Expr,
    span: Span,
    fresh: &mut dyn FnMut(&str) -> Local,
    id: &mut dyn FnMut() -> NodeId,
) -> Result<Vec<Stmt>, &'static str> {
    let effect = addr_effect_key(&to.ty).ok_or("`send` to an address of no concrete effect")?;
    let symbols = ctx.symbols;
    let (Some(decl), Some(arms), Some(module)) = (symbols.effects.get(effect.as_str()), symbols.message_arms.get(effect.as_str()), symbols.key_modules.get(effect.as_str())) else {
        return Err("`send` of a protocol with no messages");
    };
    let interface = DeclId::intern(crate::decl_id::DeclPath { module: (*module).clone(), kind: crate::decl_id::DeclKind::Effect, path: vec![decl.name.name.clone()], sig: None, ordinal: 0 });
    let sends: Vec<usize> = decl.fns.iter().enumerate().filter(|(_, f)| f.is_send).map(|(i, _)| i).collect();
    let union_ty = crate::check::expand_message_in(symbols, m.ty.clone());
    let to_ty = to.ty.clone();
    let to_l = fresh("__fwd_to");
    let msg_l = fresh("__fwd_msg");
    let mut stmts = vec![
        Stmt::Let { id: id(), local: to_l.clone(), ty: to_ty.clone(), value: to },
        Stmt::Let { id: id(), local: msg_l.clone(), ty: union_ty.clone(), value: Expr { ty: union_ty.clone(), ..m } },
    ];
    let read = |l: &Local, ty: &Ty, steps: Vec<Step>, consume: bool| Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: l.clone(), steps }, consume, moved: false } };
    let send_of = |k: usize, from: &Local| -> Option<Expr> {
        let s = symbols.structs.get(arms.get(k)?)?;
        let args: Vec<Expr> = s
            .fields
            .iter()
            .map(|f| {
                let fty = f.ty.as_ref().map(|t| ctx.written_ty(file_idx, t)).unwrap_or(Ty::Unknown);
                read(from, &fty, vec![Step::Field(f.name.name.clone())], true)
            })
            .collect();
        Some(Expr { ty: Ty::none(), span, kind: ExprKind::Send { addr: Box::new(read(&to_l, &to_ty, Vec::new(), false)), member: MemberRef { interface, index: *sends.get(k)? }, args } })
    };
    // The arms the message can be, by its own type: every arm of the
    // protocol for a `Message<E>`, one for a value already known to be one
    // (`Inventory.Clear {}`, or `m` after `is .Clear`), a few after a
    // narrowing. Each is paired with its member's index in the protocol.
    let arm_of = |t: &Ty| match t.strip_quals() {
        Ty::Named { name, .. } => arms.iter().position(|a| a == name),
        _ => None,
    };
    let present: Vec<usize> = match union_ty.strip_quals() {
        Ty::Union(xs) => xs.iter().map(|x| arm_of(x).ok_or("`send` of a value that is not a message of its protocol")).collect::<Result<_, _>>()?,
        other => vec![arm_of(other).ok_or("`send` of a value that is not a message of its protocol")?],
    };
    if let [k] = present.as_slice() {
        stmts.push(Stmt::Expr(send_of(*k, &msg_l).ok_or("`send` of an unknown message")?));
        return Ok(stmts);
    }
    let sw = id();
    let mut switch_arms = Vec::new();
    for (pos, &k) in present.iter().enumerate() {
        let arm_ty = Ty::Named { name: arms[k].to_string(), args: Vec::new() };
        let nar = fresh("__fwd_arm");
        let narrow = Stmt::Narrow { id: id(), local: nar.clone(), ty: arm_ty, from: Place { root: msg_l.clone(), steps: Vec::new() }, from_ty: union_ty.clone(), because: Justification::Arm { switch: sw, arm: pos } };
        let send = send_of(k, &nar).ok_or("`send` of an unknown message")?;
        switch_arms.push(SwitchArm { test: ArmTest::Arm(pos), body: Block { stmts: vec![narrow, Stmt::Expr(send)], value: None, span } });
    }
    stmts.push(Stmt::Expr(Expr { ty: Ty::none(), span, kind: ExprKind::Switch { id: sw, subject: Box::new(read(&msg_l, &union_ty, Vec::new(), false)), arms: switch_arms } }));
    Ok(stmts)
}

/// [actor-sent] Lowers every `send` call left in `fd` (a handler
/// generic over its face, now instantiated): a statement `send(to, m)`
/// becomes [`forward_stmts`]. One anywhere else is refused, never emitted.
fn lower_forwards(ctx: &Ctx<'_>, fd: &mut FnDecl) {
    let mut n = 0u32;
    let mut next_id = u32::MAX - 1000;
    crate::ir::walk::fn_stmt_lists_mut(fd, &mut |stmts: &mut Vec<Stmt>| {
        let taken = std::mem::take(stmts);
        for s in taken {
            match s {
                Stmt::Expr(Expr { kind: ExprKind::Call { target: FnRef::Decl(d), args, .. }, span, ty }) if is_forward(&d) && args.len() == 2 => {
                    let mut it = args.into_iter();
                    let (to, mut m) = (it.next().unwrap(), it.next().unwrap());
                    // `send` moves its message: in the generic body its
                    // type owed nothing yet, so the read was not a move.
                    if let ExprKind::Read { consume, .. } = &mut m.kind {
                        *consume = true;
                    }
                    let mut fresh = |base: &str| {
                        n += 1;
                        Local(format!("{base}~m{n}"))
                    };
                    let mut id = || {
                        next_id -= 1;
                        NodeId(next_id)
                    };
                    match forward_stmts(ctx, 0, to, m, span, &mut fresh, &mut id) {
                        Ok(more) => stmts.extend(more),
                        Err(what) => stmts.push(Stmt::Expr(crate::ir::build::unsupported(ty, span, what))),
                    }
                }
                other => stmts.push(other),
            }
        }
    });
}

/// [actor-sent] Calls `f` on every named type in `t`, at any depth.
fn each_named(t: &Ty, f: &mut impl FnMut(&str, &[Ty])) {
    match t {
        Ty::Named { name, args } => {
            f(name, args);
            for a in args {
                each_named(a, f);
            }
        }
        Ty::Qualified { base, .. } => each_named(base, f),
        Ty::Union(xs) | Ty::Tuple(xs) => xs.iter().for_each(|x| each_named(x, f)),
        Ty::Array(e) => each_named(e, f),
        Ty::Fn { params, ret, effects, .. } => {
            params.iter().for_each(|x| each_named(x, f));
            each_named(ret, f);
            effects.iter().for_each(|x| each_named(x, f));
        }
        _ => {}
    }
}

/// [actor-sent] `t` with each type parameter of `map` replaced, then each
/// `Message<X>` it now names expanded to `X`'s arms.
fn subst_ty(t: &Ty, map: &std::collections::HashMap<String, Ty>, rename: &dyn Fn(&str, &[Ty]) -> Option<String>) -> Ty {
    let out = match t {
        Ty::Var(v) => map.get(v).cloned().unwrap_or_else(|| t.clone()),
        Ty::Named { name, args } => {
            let args: Vec<Ty> = args.iter().map(|a| subst_ty(a, map, rename)).collect();
            match rename(name, &args) {
                Some(new) => Ty::Named { name: new, args: Vec::new() },
                None => Ty::Named { name: name.clone(), args },
            }
        }
        Ty::Qualified { base, quals } => Ty::Qualified { base: Box::new(subst_ty(base, map, rename)), quals: quals.clone() },
        Ty::Union(xs) => Ty::Union(xs.iter().map(|x| subst_ty(x, map, rename)).collect()),
        Ty::Tuple(xs) => Ty::Tuple(xs.iter().map(|x| subst_ty(x, map, rename)).collect()),
        Ty::Array(e) => Ty::Array(Box::new(subst_ty(e, map, rename))),
        Ty::Fn { params, ret, contract, effects } => Ty::Fn {
            params: params.iter().map(|x| subst_ty(x, map, rename)).collect(),
            ret: Box::new(subst_ty(ret, map, rename)),
            contract: contract.clone(),
            effects: effects.iter().map(|x| subst_ty(x, map, rename)).collect(),
        },
        other => other.clone(),
    };
    out
}

/// [actor-sent] Phase 0: every handler generic over the effect it serves
/// (a face that is one of its own type parameters), instantiated at each
/// list of type arguments the program constructs it at — `Relay<Inventory>`
/// becomes `Relay__Inventory`, an ordinary handler of `Inventory` with
/// `Message<Inventory>` expanded to its arms and its `send`s lowered —
/// and every type naming the instantiation renamed to it. The generic
/// declaration leaves the program: no backend could implement a type
/// parameter.
fn instantiate_generic_faces(ctx: &Ctx<'_>, program: &mut Program) {
    let mut generic: Vec<(String, usize, usize)> = Vec::new();
    for (mi, m) in program.modules.iter().enumerate() {
        for (di, d) in m.decls.iter().enumerate() {
            let Decl::Impl(h) = d else { continue };
            let faced = h.faces.iter().any(|f| matches!(f.strip_quals(), Ty::Var(v) if h.type_params.iter().any(|t| t.name == *v)));
            if faced {
                let key = ctx.symbols.key_of_id(h.id).unwrap_or(&h.name).to_string();
                generic.push((key, mi, di));
            }
        }
    }
    if generic.is_empty() {
        return;
    }
    // Every concrete list of type arguments each is named at.
    let mut insts: Vec<(usize, Vec<Ty>)> = Vec::new();
    for m in &mut program.modules {
        for d in &mut m.decls {
            crate::ir::walk::decl_tys_mut(d, &mut |t: &mut Ty| {
                each_named(t, &mut |name, args| {
                    if args.is_empty() || args.iter().any(crate::types::mentions_var) {
                        return;
                    }
                    if let Some(g) = generic.iter().position(|(k, _, _)| k == name) {
                        if !insts.iter().any(|(gi, a)| *gi == g && a.as_slice() == args) {
                            insts.push((g, args.to_vec()));
                        }
                    }
                });
            });
        }
    }
    let mangle = |args: &[Ty]| -> String {
        args.iter()
            .map(|a| match a.strip_quals() {
                Ty::Named { name, .. } => crate::typekey::plain(name).replace('.', "_"),
                other => format!("{other:?}").chars().filter(|c| c.is_alphanumeric()).collect(),
            })
            .collect::<Vec<_>>()
            .join("_")
    };
    // The module each instantiation lives in: the protocol's, as the
    // message enum's (a std router's instantiation for a program's protocol
    // is the program's code, not std's).
    let home = |args: &[Ty], fallback: usize| -> usize {
        let Some(Ty::Named { name, .. }) = args.first().map(|a| a.strip_quals()) else { return fallback };
        let Some(m) = ctx.symbols.key_modules.get(name.as_str()) else { return fallback };
        program.modules.iter().position(|x| x.path == **m).unwrap_or(fallback)
    };
    // The name and key each instantiation takes.
    let mut names: Vec<(String, Vec<Ty>, String, String, usize)> = Vec::new(); // generic key, args, name, key, module
    for (g, args) in &insts {
        let (key, mi, di) = &generic[*g];
        let Decl::Impl(h) = &program.modules[*mi].decls[*di] else { continue };
        let name = format!("{}__{}", h.name, mangle(args));
        let at = home(args, *mi);
        names.push((key.clone(), args.clone(), name.clone(), enum_key(&name, &program.modules[at].path), at));
    }
    let rename = |name: &str, args: &[Ty]| -> Option<String> {
        names.iter().find(|(k, a, _, _, _)| k == name && a.as_slice() == args).map(|(_, _, _, key, _)| key.clone())
    };
    // The instantiations, made from the generic declarations.
    let mut made: Vec<(usize, ImplDecl)> = Vec::new();
    for (g, args) in &insts {
        let (_, mi, di) = &generic[*g];
        let Decl::Impl(h) = &program.modules[*mi].decls[*di] else { continue };
        let Some((_, _, name, _, at)) = names.iter().find(|(k, a, _, _, _)| *k == generic[*g].0 && a == args) else { continue };
        let at = *at;
        let map: std::collections::HashMap<String, Ty> = h.type_params.iter().map(|t| t.name.clone()).zip(args.iter().cloned()).collect();
        let mut inst = h.clone();
        inst.name = name.clone();
        inst.id = DeclId::generated(&program.modules[at].path, &[name.as_str()]);
        inst.type_params = Vec::new();
        crate::ir::walk::impl_tys(&mut inst, &mut |t: &mut Ty| {
            *t = crate::check::expand_message_in(ctx.symbols, subst_ty(t, &map, &rename));
        });
        for fd in inst.members.iter_mut().chain(inst.init.iter_mut()) {
            lower_forwards(ctx, fd);
        }
        made.push((at, inst));
    }
    // Every other declaration names the instantiations by their own keys.
    let none: std::collections::HashMap<String, Ty> = std::collections::HashMap::new();
    for m in &mut program.modules {
        for d in &mut m.decls {
            crate::ir::walk::decl_tys_mut(d, &mut |t: &mut Ty| {
                let mut hit = false;
                each_named(t, &mut |n, a| hit |= rename(n, a).is_some());
                if hit {
                    *t = subst_ty(t, &none, &rename);
                }
            });
        }
    }
    // The generic declarations go; the instantiations take their place.
    let mut gone: Vec<(usize, usize)> = generic.iter().map(|(_, mi, di)| (*mi, *di)).collect();
    gone.sort();
    for (mi, di) in gone.into_iter().rev() {
        program.modules[mi].decls.remove(di);
    }
    for (mi, inst) in made {
        program.modules[mi].decls.push(Decl::Impl(inst));
    }
}
