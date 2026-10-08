//! Declarations: structs, unions, interfaces (effects), impls (handlers),
//! fns and platform types (IR.md §3, §6).

use std::collections::HashMap;

use salvo_core::param_mode::{Modes, PassMode};
use salvo_core::source::SourceFile;
use salvo_core::types::Ty;
use salvo_core::FnKey;
use salvo_syntax::ast::{self, Item};

use super::{body::Lower, erase, AModule, Ctx};
use crate::ir::*;

pub fn build_module(ctx: &Ctx<'_>, file_idx: usize, file: &SourceFile, module: &AModule) -> (Module, Vec<String>) {
    let mut out = Module { path: file.module.clone(), file_name: file.name.clone(), decls: Vec::new() };
    let mut errors = Vec::new();
    // [mod-use] Module-level instances, in scope for every fn below.
    let mut statics: Vec<(Ty, Local)> = Vec::new();
    for (i, u) in module.uses.iter().enumerate() {
        let mut lower = Lower::new(ctx, file_idx, None);
        let stmts = lower.use_stmt(&u.handler, &[], u.span);
        let bound: Vec<(Ty, Local)> = lower.effect_env.clone();
        errors.extend(lower.finish());
        // The instance is one static; each face's handle [effect-handle] is
        // another, over it (`use addr` binds no handle: the one `Let` is the
        // face's). Every local the `use` bound is renamed to its static's.
        let inst_local = Local(format!("__module_use{i}"));
        let mut own: Vec<Stmt> = Vec::new();
        let mut inst_ty = Ty::Unknown;
        let mut orig_inst: Option<Local> = None;
        let mut faces: Vec<StaticDecl> = Vec::new();
        for s in stmts {
            match s {
                Stmt::Let { id, local, ty, value } if orig_inst.is_none() => {
                    // The first binding is the instance (or, for `use addr`,
                    // the one face binding).
                    orig_inst = Some(local.clone());
                    if let Some((t, _)) = bound.iter().find(|(_, l)| *l == local) {
                        statics.push((t.clone(), inst_local.clone()));
                    }
                    inst_ty = ty.clone();
                    own.push(Stmt::Let { id, local: inst_local.clone(), ty, value });
                }
                Stmt::Let { id, local, ty, value } => {
                    // A face binding over the instance: its own static, its
                    // read of the instance renamed to the instance's static.
                    let j = faces.len();
                    let sl = Local(format!("__module_use{i}_{j}"));
                    let rename = |e: Expr| -> Expr {
                        match e.kind {
                            ExprKind::Read { place, consume } if Some(&place.root) == orig_inst.as_ref() => Expr { ty: e.ty, span: e.span, kind: ExprKind::Read { place: Place { root: inst_local.clone(), steps: place.steps }, consume } },
                            ExprKind::Handle { instance } => {
                                let inner = match instance.kind {
                                    ExprKind::Read { place, consume } if Some(&place.root) == orig_inst.as_ref() => Expr { ty: instance.ty, span: instance.span, kind: ExprKind::Read { place: Place { root: inst_local.clone(), steps: place.steps }, consume } },
                                    other => Expr { ty: instance.ty, span: instance.span, kind: other },
                                };
                                Expr { ty: e.ty, span: e.span, kind: ExprKind::Handle { instance: Box::new(inner) } }
                            }
                            other => Expr { ty: e.ty, span: e.span, kind: other },
                        }
                    };
                    let value = rename(value);
                    if let Some((t, _)) = bound.iter().find(|(_, l)| *l == local) {
                        statics.push((t.clone(), sl.clone()));
                    }
                    faces.push(StaticDecl { id: DeclId { module: file.module.clone(), item: usize::MAX - i * 16 - 1 - j, sub: 0 }, local: sl.clone(), ty: ty.clone(), stmts: vec![Stmt::Let { id, local: sl, ty, value }], span: u.span });
                }
                other => own.push(other),
            }
        }
        out.decls.push(Decl::Static(StaticDecl { id: DeclId { module: file.module.clone(), item: usize::MAX - i * 16, sub: 0 }, local: inst_local, ty: inst_ty, stmts: own, span: u.span }));
        out.decls.extend(faces.into_iter().map(Decl::Static));
    }
    for (item_idx, item) in module.items.iter().enumerate() {
        let id = ctx.item_id(file_idx, item_idx);
        match item {
            Item::Struct(s) if s.comptime => {}
            Item::Struct(s) => out.decls.push(Decl::Struct(struct_decl(ctx, file_idx, id, s, &mut errors))),
            Item::Type(t) => {
                if let Some(alias) = &t.alias {
                    if matches!(alias, ast::Type::Union { .. } | ast::Type::Nullable { .. }) {
                        out.decls.push(Decl::Union(UnionDecl {
                            id,
                            name: t.name.name.clone(),
                            exported: t.exported,
                            type_params: type_params(&t.generics, &t.generic_canbe),
                            ty: ctx.written_ty(file_idx, alias),
                            factories: ctx.checked.factories.get(&(file_idx, t.name.span)).cloned(),
                            span: t.span,
                        }));
                    }
                } else if t.platform || t.intrinsic {
                    out.decls.push(Decl::PlatformType(PlatformTypeDecl {
                        id,
                        name: t.name.name.clone(),
                        exported: t.exported,
                        type_params: type_params(&t.generics, &t.generic_canbe),
                        linear: t.linear,
                        canbe_mut: t.auto_qualifiers.iter().any(|q| q.name.name == "Mut"),
                        platform: t.platform,
                        threadsafe: t.threadsafe,
                        slots: !t.fn_slots.is_empty(),
                        iterable: t.iterable,
                        iter_elem: t
                            .obligations
                            .iter()
                            .find(|o| o.group.name.name == "Iter")
                            .and_then(|o| o.group.args.get(1))
                            .map(|a| ctx.written_ty(file_idx, a)),
                        span: t.span,
                    }));
                }
            }
            Item::Effect(e) if e.name.name == salvo_core::THROW_EFFECT => {}
            Item::Effect(e) => out.decls.push(Decl::Interface(interface_decl(ctx, file_idx, id, e))),
            Item::Handler(h) => out.decls.push(Decl::Impl(impl_decl(ctx, file_idx, id, h, &statics, &mut errors))),
            Item::Fn(f) if f.body.is_some() || f.platform || f.intrinsic => {
                let key = FnKey { file: file_idx, item: item_idx };
                let kind = if f.intrinsic {
                    FnKind::Intrinsic
                } else if f.platform {
                    FnKind::Platform
                } else {
                    FnKind::Plain
                };
                out.decls.push(Decl::Fn(fn_decl(ctx, file_idx, id, Some(key), f, kind, None, &statics, &mut errors)));
            }
            Item::Fn(_) => {}
            Item::Qualifier(q) => {
                for (i, f) in q.fns.iter().enumerate() {
                    if f.body.is_none() {
                        continue;
                    }
                    let qid = DeclId { module: id.module.clone(), item: item_idx, sub: i as u16 + 1 };
                    out.decls.push(Decl::Fn(fn_decl(ctx, file_idx, qid, None, f, FnKind::Qualifies, Some(q), &statics, &mut errors)));
                }
            }
            Item::Import(_) | Item::Params(_) | Item::Refn(_) | Item::Rename(_) | Item::Test(_) => {}
        }
    }
    (out, errors)
}

fn type_params(generics: &[ast::Ident], canbe: &[(ast::Ident, ast::TypeRef)]) -> Vec<TypeParam> {
    generics
        .iter()
        .map(|g| TypeParam {
            name: g.name.clone(),
            canbe_linear: canbe.iter().any(|(n, q)| n.name == g.name && q.name.name == "linear"),
        })
        .collect()
}

fn fields(ctx: &Ctx<'_>, file_idx: usize, decls: &[ast::FieldDecl], errors: &mut Vec<String>) -> Vec<Field> {
    decls
        .iter()
        .map(|f| Field {
            name: f.name.name.clone(),
            ty: ctx.written_ty(file_idx, &f.ty),
            canbe_mut: f.canbe_mut,
            default: f.default.as_ref().map(|d| {
                let mut lower = Lower::new(ctx, file_idx, None);
                let e = lower.expr(d);
                errors.extend(lower.finish());
                e
            }),
        })
        .collect()
}

fn struct_decl(ctx: &Ctx<'_>, file_idx: usize, id: DeclId, s: &ast::StructDecl, errors: &mut Vec<String>) -> StructDecl {
    StructDecl {
        id,
        name: s.name.name.clone(),
        exported: s.exported,
        type_params: type_params(&s.generics, &s.generic_canbe),
        fields: fields(ctx, file_idx, &s.fields, errors),
        linear: s.linear,
        opaque: s.opaque,
        canbe_mut: s.auto_qualifiers.iter().any(|q| q.name.name == "Mut"),
        has_wire_form: salvo_core::struct_has_wire_form(ctx.symbols, s),
        span: s.span,
    }
}

fn interface_decl(ctx: &Ctx<'_>, file_idx: usize, id: DeclId, e: &ast::EffectDecl) -> InterfaceDecl {
    let modes = Modes { symbols: ctx.symbols, checked: ctx.checked, program: ctx.program };
    let prereqs: Vec<Ty> = e
        .prereqs
        .iter()
        .filter_map(|p| match p {
            ast::EffectRef::Effect(r) | ast::EffectRef::AnyEffect(r) => Some(Ty::Named {
                name: ctx.checked.written_key(r).to_string(),
                args: r.args.iter().map(|a| ctx.written_ty(file_idx, a)).collect(),
            }),
            _ => None,
        })
        .collect();
    let members: Vec<Member> = e
        .fns
        .iter()
        .enumerate()
        .map(|(i, m)| {
            // [effect-prereq] A prerequisite is a handler's dependency, not
            // a member's parameter: the members carry only what is written.
            let mut params: Vec<Param> = Vec::new();
            for p in m.params.iter().filter(|p| !p.implicit) {
                params.push(Param {
                    local: Local(p.name.name.clone()),
                    ty: ctx.written_ty(file_idx, &p.ty),
                    mode: modes.member_param(m, p),
                    variadic: p.variadic,
                    // [platform-check] a `Reply<T>` the host answers on.
                    check: ctx.checked.boundary_checks.get(&(file_idx, p.name.span)).cloned(),
                });
            }
            // [implicit-param] a member's implicits are trailing parameters.
            if let Some(imps) = ctx.checked.implicit_members.get(&(file_idx, m.name.span)) {
                for ip in imps {
                    params.push(Param { local: Local(ip.local.clone()), ty: erase(&ip.ty), mode: PassMode::Lent, variadic: false, check: None });
                }
            }
            // [platform-check] [platform-factory] the host boundary of a
            // platform-handled member: its result check, and the factories of
            // its result and reply payloads.
            let result_check = ctx.checked.boundary_checks.get(&(file_idx, m.name.span)).cloned();
            let mut factories: Vec<salvo_core::abi::Factories> = Vec::new();
            factories.extend(ctx.checked.factories.get(&(file_idx, m.name.span)).cloned());
            for p in &m.params {
                factories.extend(ctx.checked.factories.get(&(file_idx, p.name.span)).cloned());
            }
            Member {
                name: m.name.name.clone(),
                emitted_name: salvo_core::effect_member_name(e, i),
                type_params: type_params(&m.generics, &m.generic_canbe),
                params,
                ret: m.return_type.as_ref().map(|t| ctx.written_ty(file_idx, t)).unwrap_or_else(Ty::none),
                send: m.is_send,
                result_check,
                factories,
                span: m.span,
            }
        })
        .collect();
    // [protocol-hash] every message payload has a wire form (the types are
    // the resolved ones, so a clashing name is told from its namesakes).
    let has_wire = members.iter().filter(|m| m.send).all(|m| m.params.iter().all(|p| salvo_core::wire_blocker(ctx.symbols, p.ty.strip_quals()).is_none()));
    let protocol_hash = if e.is_actor && has_wire {
        ctx.checked.protocol_hashes.get(ctx.symbols.key_or(e, &e.name.name)).cloned()
    } else {
        None
    };
    InterfaceDecl {
        id,
        name: e.name.name.clone(),
        exported: e.exported,
        type_params: type_params(&e.generics, &[]),
        actor: e.is_actor,
        protocol_hash,
        prereqs,
        members,
        span: e.span,
    }
}

fn impl_decl(ctx: &Ctx<'_>, file_idx: usize, id: DeclId, h: &ast::HandlerDecl, statics: &[(Ty, Local)], errors: &mut Vec<String>) -> ImplDecl {
    let modes = Modes { symbols: ctx.symbols, checked: ctx.checked, program: ctx.program };
    let faces: Vec<Ty> = h.of.iter().map(|t| ctx.written_ty(file_idx, t)).collect();
    let deps: Vec<Ty> = h
        .effects
        .iter()
        .flatten()
        .filter_map(|p| match p {
            ast::EffectRef::Effect(r) | ast::EffectRef::AnyEffect(r) => Some(Ty::Named {
                name: ctx.checked.written_key(r).to_string(),
                args: r.args.iter().map(|a| ctx.written_ty(file_idx, a)).collect(),
            }),
            _ => None,
        })
        .collect();
    let ctor_params: Vec<Param> = h
        .params
        .iter()
        .map(|p| Param {
            local: Local(p.name.name.clone()),
            ty: ctx.written_ty(file_idx, &p.ty),
            mode: if p.implicit { PassMode::Lent } else { modes.default_param(&p.ty, p.variadic) },
            variadic: p.variadic,
            check: None,
        })
        .collect();
    let state = fields(ctx, file_idx, &h.state, errors);
    // The handler's fields are in scope inside its members.
    let self_fields: Vec<(String, Ty)> = ctor_params
        .iter()
        .map(|p| (p.local.0.clone(), p.ty.clone()))
        .chain(state.iter().map(|f| (f.name.clone(), f.ty.clone())))
        .collect();
    let dep_locals: Vec<(Ty, Local)> =
        deps.iter().enumerate().map(|(i, t)| (t.clone(), Local(format!("__dep{i}")))).collect();
    let face_decls: Vec<&ast::EffectDecl> = h
        .of
        .iter()
        .filter_map(|t| match t {
            ast::Type::Named { base, .. } => ctx.symbols.effects.get(ctx.checked.written_key(base)).copied(),
            _ => None,
        })
        .collect();
    let member_fn = |f: &ast::FnDecl, errors: &mut Vec<String>| {
        let faces_n = salvo_core::effects::handler_member_faces(&face_decls, f).len();
        let mut d = fn_decl(ctx, file_idx, id.clone(), None, f, FnKind::Member { faces: faces_n }, None, &[], errors);
        // Members have no FnKey: build the body with the handler's scope.
        d.body = f.body.as_ref().map(|b| {
            let mut lower = Lower::new(ctx, file_idx, None);
            lower.self_fields = self_fields.clone();
            lower.effect_env = statics.to_vec();
            lower.effect_env.extend(dep_locals.clone());
            lower.in_handler = Some(h.name.name.clone());
            let eff_member = face_decls.iter().find_map(|e| {
                salvo_core::effect_member_index(e, f).map(|i| (*e, i))
            });
            let mut params: Vec<Param> = Vec::new();
            for p in f.params.iter().filter(|p| !p.implicit) {
                let mode = match eff_member {
                    Some((e, i)) => {
                        let m = &e.fns[i];
                        match m.params.iter().filter(|q| !q.implicit).nth(f.params.iter().position(|x| std::ptr::eq(x, p)).unwrap_or(0)) {
                            Some(ep) => modes.member_param(m, ep),
                            None => modes.default_param(&p.ty, p.variadic),
                        }
                    }
                    None if f.is_send => modes.member_param(f, p),
                    None => modes.default_param(&p.ty, p.variadic),
                };
                params.push(Param { local: Local(p.name.name.clone()), ty: ctx.written_ty(file_idx, &p.ty), mode, variadic: p.variadic, check: None });
            }
            for p in &params {
                lower.bind_owned(&p.local.0, p.ty.clone(), p.mode == PassMode::Moved);
            }
            if let Some(imps) = ctx.checked.implicit_members.get(&(file_idx, f.name.span)) {
                for ip in imps {
                    lower.bind(&ip.local, erase(&ip.ty));
                }
            }
            let ret = f.return_type.as_ref().map(|t| ctx.written_ty(file_idx, t)).unwrap_or_else(Ty::none);
            let block = lower.fn_body(b, &ret);
            errors.extend(lower.finish());
            block
        });
        d
    };
    let init = h.init.as_ref().map(|f| member_fn(f, errors));
    let members: Vec<FnDecl> = h.fns.iter().map(|f| member_fn(f, errors)).collect();
    let mailbox = h.mailbox.as_ref().map(|m| {
        let mut lower = Lower::new(ctx, file_idx, None);
        for (n, t) in &self_fields {
            lower.bind(n, t.clone());
        }
        let e = lower.expr(m);
        errors.extend(lower.finish());
        e
    });
    let parks = ctx.checked.parking_handlers.contains(&h.name.name);
    ImplDecl {
        id,
        name: h.name.name.clone(),
        exported: h.exported,
        type_params: type_params(&h.generics, &[]),
        faces,
        face_any: h.of_any.clone(),
        ctor_params,
        deps,
        state,
        mailbox,
        init,
        members,
        stateful: salvo_core::handler_is_stateful(h, parks),
        platform: h.platform,
        threadsafe: h.threadsafe,
        intrinsic: h.intrinsic,
        span: h.span,
    }
}

#[allow(clippy::too_many_arguments)]
fn fn_decl(
    ctx: &Ctx<'_>,
    file_idx: usize,
    id: DeclId,
    key: Option<FnKey>,
    f: &ast::FnDecl,
    kind: FnKind,
    qualifier: Option<&ast::QualifierDecl>,
    statics: &[(Ty, Local)],
    errors: &mut Vec<String>,
) -> FnDecl {
    let modes = Modes { symbols: ctx.symbols, checked: ctx.checked, program: ctx.program };
    let mut lower = Lower::new(ctx, file_idx, key);
    lower.effect_env.extend(statics.iter().cloned());
    // The tables are keyed by the fn's key; a qualifier fn has one through
    // its name span, a member none.
    let table_key = key.or_else(|| ctx.checked.fn_refs.get(&(file_idx, f.name.span)).copied());
    let declared_effects: Vec<Ty> = f
        .effects
        .iter()
        .flatten()
        .filter_map(|p| match p {
            ast::EffectRef::Effect(r) | ast::EffectRef::AnyEffect(r) => Some(Ty::Named {
                name: ctx.checked.written_key(r).to_string(),
                args: r.args.iter().map(|a| ctx.written_ty(file_idx, a)).collect(),
            }),
            _ => None,
        })
        .collect();
    let mut params: Vec<Param> = Vec::new();
    // [effect-fn-deps] Effects as values: leading parameters.
    let effects: Vec<Ty> = table_key
        .and_then(|k| ctx.checked.fn_effects.get(&k))
        .cloned()
        .unwrap_or(declared_effects)
        .into_iter()
        .filter(|t| !matches!(t, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT))
        .map(|t| erase(&t))
        .collect();
    let throws: Option<Ty> = table_key.and_then(|k| ctx.checked.fn_effects.get(&k)).and_then(|effs| {
        effs.iter().find_map(|t| match t {
            Ty::Named { name, args } if name == salvo_core::THROW_EFFECT => Some(args.first().map(erase).unwrap_or(Ty::Unknown)),
            _ => None,
        })
    });
    // Each named after its effect (`console`), away from the written
    // parameters' names and from each other.
    let taken: Vec<String> = f.params.iter().map(|p| p.name.name.clone()).collect();
    let mut used: Vec<String> = Vec::new();
    for t in &effects {
        let mut name = effect_local_name(t);
        let base = name.clone();
        let mut n = 1;
        while taken.contains(&name) || used.contains(&name) {
            name = format!("{base}__{n}");
            n += 1;
        }
        used.push(name.clone());
        let local = Local(name);
        lower.effect_env.push((t.clone(), local.clone()));
        params.push(Param { local, ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None });
    }
    let effect_params = params.len();
    for p in f.params.iter().filter(|p| !p.implicit) {
        let mode = if kind == FnKind::Qualifies || qualifier.is_some() {
            modes.default_param(&p.ty, p.variadic)
        } else {
            modes.fn_param(key, p)
        };
        let ty = ctx.written_ty(file_idx, &p.ty);
        lower.bind_owned(&p.name.name, ty.clone(), mode == PassMode::Moved);
        params.push(Param { local: Local(p.name.name.clone()), ty, mode, variadic: p.variadic, check: None });
    }
    let mut implicit_params = 0;
    if let Some(k) = key {
        if let Some(imps) = ctx.checked.implicit_params.get(&k) {
            for ip in imps {
                let ty = erase(&ip.ty);
                lower.bind(&ip.local, ty.clone());
                lower.implicit_locals.insert(ip.name.clone(), Local(ip.local.clone()));
                params.push(Param { local: Local(ip.local.clone()), ty, mode: PassMode::Lent, variadic: false, check: None });
                implicit_params += 1;
            }
        }
    } else if let Some(imps) = ctx.checked.implicit_members.get(&(file_idx, f.name.span)) {
        // A member or qualifier fn: keyed by its name span [implicit-param].
        for ip in imps {
            let ty = erase(&ip.ty);
            lower.bind(&ip.local, ty.clone());
            lower.implicit_locals.insert(ip.name.clone(), Local(ip.local.clone()));
            params.push(Param { local: Local(ip.local.clone()), ty, mode: PassMode::Lent, variadic: false, check: None });
            implicit_params += 1;
        }
    }
    let mut ret = f.return_type.as_ref().map(|t| ctx.written_ty(file_idx, t)).unwrap_or_else(Ty::none);
    // [iter-type] a written `iter T` is the iterator struct the checker resolved.
    if ret.mentions_iter_marker() {
        if let Some(r) = table_key.and_then(|k| ctx.checked.iter_returns.get(&k)) {
            ret = erase(r);
        }
    }
    let mut borrows: Vec<usize> = table_key
        .and_then(|k| ctx.checked.fn_lends.get(&k))
        .map(|ls| ls.iter().map(|i| i + effect_params).collect())
        .unwrap_or_default();
    // [proj-anywhere] The sources a written `proj(…)` names, in the result
    // type or a result entry of the clause.
    let index_of = |n: &str| params.iter().position(|p| p.local.0 == n);
    let mut sources: Vec<String> = Vec::new();
    if let Some(rt) = &f.return_type {
        proj_from(rt, &mut sources);
    }
    let mut holds: Vec<(usize, usize)> = Vec::new();
    for d in f.deductions.iter().flatten() {
        let Some(srcs) = d.proj_sources() else { continue };
        match &d.target {
            ast::DeductionTarget::Result { .. } | ast::DeductionTarget::Opaque => sources.extend(srcs.iter().map(|s| s.name.clone())),
            ast::DeductionTarget::Param { name, path } if !path.is_empty() => {
                for s in srcs {
                    if let (Some(a), Some(b)) = (index_of(&name.name), index_of(&s.name)) {
                        holds.push((a, b));
                    }
                }
            }
            _ => {}
        }
    }
    for s in sources {
        if let Some(i) = index_of(&s) {
            if !borrows.contains(&i) {
                borrows.push(i);
            }
        }
    }
    let body = if matches!(kind, FnKind::Member { .. }) {
        None // filled by the caller with the handler's scope
    } else {
        f.body.as_ref().map(|b| lower.fn_body(b, &ret))
    };
    errors.extend(lower.finish());
    let may_alias = may_alias(f, &params);
    FnDecl {
        id,
        name: f.name.name.clone(),
        exported: f.exported,
        kind,
        send: f.is_send,
        type_params: {
            // A qualifier's fns are generic in the qualifier's parameters too.
            let mut tps = qualifier.map(|q| type_params(&q.generics, &q.generic_canbe)).unwrap_or_default();
            tps.extend(type_params(&f.generics, &f.generic_canbe));
            tps
        },
        params,
        effect_params,
        implicit_params,
        ret,
        borrows,
        may_alias,
        holds,
        throws,
        result_check: if f.platform { ctx.checked.boundary_checks.get(&(file_idx, f.name.span)).cloned() } else { None },
        factories: if f.platform { ctx.checked.factories.get(&(file_idx, f.name.span)).cloned() } else { None },
        body,
        span: f.span,
    }
}

pub(crate) fn _unused(_: &HashMap<String, Ty>) {}

/// The names every `proj(…)` in a written type borrows from.
fn proj_from(t: &ast::Type, out: &mut Vec<String>) {
    fn in_ref(r: &ast::TypeRef, out: &mut Vec<String>) {
        if r.name.name == "proj" {
            out.extend(r.from.iter().map(|i| i.name.clone()));
        }
        for a in &r.args {
            proj_from(a, out);
        }
    }
    match t {
        ast::Type::Named { qualifiers, base } => {
            qualifiers.iter().for_each(|q| in_ref(q, out));
            in_ref(base, out);
        }
        ast::Type::QualifiedGroup { qualifiers, base, .. } => {
            qualifiers.iter().for_each(|q| in_ref(q, out));
            proj_from(base, out);
        }
        ast::Type::Union { arms, .. } | ast::Type::Tuple { elems: arms, .. } => arms.iter().for_each(|a| proj_from(a, out)),
        ast::Type::Array { elem, .. } | ast::Type::Nullable { inner: elem, .. } => proj_from(elem, out),
        _ => {}
    }
}

/// The local an effect instance parameter is named by: the effect's base
/// name in snake case (`Console` → `console`, `Random<Int>` → `random`).
pub(crate) fn effect_local_name(t: &Ty) -> String {
    let base = match t.strip_quals() {
        Ty::Named { name, .. } => salvo_core::typekey::plain(name).to_string(),
        other => other.to_string(),
    };
    let mut out = String::new();
    for c in base.chars() {
        if c.is_uppercase() {
            if out.chars().last().is_some_and(|p| p.is_lowercase() || p.is_ascii_digit()) {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_alphanumeric() || c == '_' {
            out.push(c);
        }
    }
    if out.is_empty() { "effect".to_string() } else { out }
}

/// [canbe-entry] A fn's `canbe` entries, by IR parameter index (effects come
/// first in `params`, so the AST's positions do not carry over). An anchor
/// path is rooted at a parameter; a numeric segment is a tuple element.
fn may_alias(f: &ast::FnDecl, params: &[Param]) -> Vec<MayAlias> {
    let idx = |n: &str| params.iter().position(|p| p.local.0 == n);
    let mut out = Vec::new();
    for d in f.deductions.iter().flatten() {
        let ast::DeductionKind::CanBe { others, anchored } = &d.kind else { continue };
        let ast::DeductionTarget::Param { name, .. } = &d.target else { continue };
        let Some(subject) = idx(&name.name) else { continue };
        for path in others {
            let Some(head) = path.first().and_then(|i| idx(&i.name)) else { continue };
            if *anchored {
                let steps = path[1..]
                    .iter()
                    .map(|seg| match seg.name.parse::<usize>() {
                        Ok(n) => AnchorStep::Tuple(n),
                        Err(_) => AnchorStep::Field(seg.name.clone()),
                    })
                    .collect();
                out.push(MayAlias::In { param: subject, root: head, path: steps });
            } else {
                out.push(MayAlias::Params(subject, head));
            }
        }
    }
    out
}
