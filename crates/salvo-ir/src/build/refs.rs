//! [ref-handle] [ir-types] Which container a `ref` handle is a position in,
//! kept on the type: every `ref` qualifier in the IR carries the
//! **container's storage type** as its one argument (`ref(League) Mut
//! Player`). The checker's types drop the source (a `ref(c)`'s `c` is a fact
//! about the value, carried by its fate links); the IR states it so a
//! backend that renders a handle as a path into its container can name the
//! path's type without chasing the mint. A garbage-collected backend ignores
//! it.
//!
//! The pass runs after the program is built: a handle's container is the
//! storage type of
//! - a mint's lent parameter, for a fn's result (`FnDecl::borrows`);
//! - the named container parameter, for a `ref(c)` parameter
//!   (`FnDecl::ref_anchors`);
//! - the lent argument, for a call's result;
//! - the walked container, for a `for` element;
//! - the source, for a binding, a narrowing, a read or a `!`.

use std::collections::HashMap;

use salvo_core::types::Ty;

use crate::ir::*;

/// Whether `ty` carries a `ref` qualifier at its top or in a union arm.
pub fn has_ref(ty: &Ty) -> bool {
    match ty {
        Ty::Qualified { quals, base } => quals.iter().any(|q| q.name == "ref") || has_ref(base),
        Ty::Union(arms) => arms.iter().any(has_ref),
        _ => false,
    }
}

/// The container a handle type names (its `ref` qualifier's argument).
pub fn ref_container(ty: &Ty) -> Option<&Ty> {
    match ty {
        Ty::Qualified { quals, base } => quals.iter().find(|q| q.name == "ref").and_then(|q| q.args.first()).or_else(|| ref_container(base)),
        Ty::Union(arms) => arms.iter().find_map(ref_container),
        _ => None,
    }
}

/// What a value of `ty` is stored as: qualifiers and the `None` arm gone.
pub fn storage(ty: &Ty) -> Ty {
    let mut t = ty.strip_quals().clone();
    loop {
        let next = match &t {
            Ty::Union(_) => t.without_none(),
            Ty::Qualified { .. } => t.strip_quals().clone(),
            _ => return t,
        };
        if next == t {
            return t;
        }
        t = next;
    }
}

fn set_container(ty: &mut Ty, c: &Ty) {
    match ty {
        Ty::Qualified { quals, base } => {
            for q in quals.iter_mut().filter(|q| q.name == "ref") {
                q.args = vec![c.clone()];
            }
            set_container(base, c);
        }
        Ty::Union(arms) => arms.iter_mut().for_each(|a| set_container(a, c)),
        _ => {}
    }
}

fn is_scalar(t: &Ty) -> bool {
    matches!(t.strip_quals(), Ty::Named { name, args } if args.is_empty() && matches!(name.as_str(), "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"))
}

/// A fn type lending a handle answers one into its first non-scalar,
/// non-fn parameter (the locator convention [rs-loc]); fixed throughout `ty`.
fn fix_fn_types(ty: &mut Ty) {
    match ty {
        Ty::Fn { params, ret, effects, .. } => {
            params.iter_mut().chain(effects.iter_mut()).for_each(fix_fn_types);
            fix_fn_types(ret);
            if has_ref(ret) {
                if let Some(p) = params.iter().find(|p| !is_scalar(p) && !matches!(p.strip_quals(), Ty::Fn { .. })) {
                    let c = storage(p);
                    set_container(ret, &c);
                }
            }
        }
        Ty::Qualified { base, quals } => {
            fix_fn_types(base);
            for q in quals.iter_mut() {
                q.args.iter_mut().for_each(fix_fn_types);
            }
        }
        Ty::Named { args, .. } | Ty::Union(args) | Ty::Tuple(args) => args.iter_mut().for_each(fix_fn_types),
        Ty::Array(e) => fix_fn_types(e),
        _ => {}
    }
}

/// The parameter a fn's result is lent from, for its container.
fn lend_index(f: &FnDecl) -> Option<usize> {
    if let Some(i) = f.borrows.first() {
        return Some(*i);
    }
    f.params.iter().enumerate().skip(f.effect_params).find(|(_, p)| !is_scalar(&p.ty) && !matches!(p.ty.strip_quals(), Ty::Fn { .. })).map(|(i, _)| i)
}

pub fn annotate(program: &mut Program) {
    // Every fn's lend index, by id (a call's container is its lent argument).
    let mut lends: HashMap<DeclId, Option<usize>> = HashMap::new();
    for m in &program.modules {
        for d in &m.decls {
            if let Decl::Fn(f) = d {
                lends.insert(f.id.clone(), lend_index(f));
            }
        }
    }
    for m in &mut program.modules {
        for d in &mut m.decls {
            match d {
                Decl::Fn(f) => fn_decl(f, &lends),
                Decl::Impl(h) => {
                    for f in h.members.iter_mut().chain(h.init.iter_mut()) {
                        fn_decl(f, &lends);
                    }
                }
                Decl::Interface(i) => {
                    for mem in &mut i.members {
                        mem.params.iter_mut().for_each(|p| fix_fn_types(&mut p.ty));
                        fix_fn_types(&mut mem.ret);
                        if has_ref(&mem.ret) {
                            if let Some(p) = mem.params.iter().find(|p| !is_scalar(&p.ty) && !matches!(p.ty.strip_quals(), Ty::Fn { .. })) {
                                let c = storage(&p.ty);
                                set_container(&mut mem.ret, &c);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn fn_decl(f: &mut FnDecl, lends: &HashMap<DeclId, Option<usize>>) {
    for p in &mut f.params {
        fix_fn_types(&mut p.ty);
    }
    fix_fn_types(&mut f.ret);
    if has_ref(&f.ret) {
        if let Some(k) = lend_index(f) {
            let c = storage(&f.params[k].ty);
            set_container(&mut f.ret, &c);
        }
    }
    for &(a, c) in &f.ref_anchors.clone() {
        let ct = storage(&f.params[c].ty);
        set_container(&mut f.params[a].ty, &ct);
    }
    let mut env: HashMap<String, Ty> = f.params.iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect();
    if let Some(b) = &mut f.body {
        block(b, &mut env, lends);
    }
}

fn block(b: &mut Block, env: &mut HashMap<String, Ty>, lends: &HashMap<DeclId, Option<usize>>) {
    for s in &mut b.stmts {
        stmt(s, env, lends);
    }
    if let Some(v) = &mut b.value {
        expr(v, env, lends);
    }
}

fn copy_container(to: &mut Ty, from: &Ty) {
    if has_ref(to) {
        if let Some(c) = ref_container(from).cloned() {
            set_container(to, &c);
        }
    }
}

fn stmt(s: &mut Stmt, env: &mut HashMap<String, Ty>, lends: &HashMap<DeclId, Option<usize>>) {
    match s {
        Stmt::Loop { body, .. } => block(body, env, lends),
        Stmt::ForEach { local, ty, iterable, body, .. } => {
            expr(iterable, env, lends);
            fix_fn_types(ty);
            if has_ref(ty) {
                let c = storage(&iterable.ty);
                set_container(ty, &c);
            }
            env.insert(local.0.clone(), ty.clone());
            block(body, env, lends);
        }
        Stmt::Let { local, ty, value, .. } => {
            expr(value, env, lends);
            fix_fn_types(ty);
            copy_container(ty, &value.ty);
            env.insert(local.0.clone(), ty.clone());
        }
        Stmt::Alias { local, ty, place, .. } => {
            place_exprs(place, env, lends);
            fix_fn_types(ty);
            env.insert(local.0.clone(), ty.clone());
        }
        Stmt::Narrow { local, ty, from, from_ty, .. } => {
            place_exprs(from, env, lends);
            if from.steps.is_empty() {
                if let Some(src) = env.get(&from.root.0).cloned() {
                    copy_container(from_ty, &src);
                    copy_container(ty, &src);
                }
            }
            env.insert(local.0.clone(), ty.clone());
        }
        Stmt::Unpack { locals, from, .. } => {
            place_exprs(from, env, lends);
            for (l, t) in locals.iter_mut() {
                fix_fn_types(t);
                env.insert(l.0.clone(), t.clone());
            }
        }
        Stmt::Assign { place, value } => {
            place_exprs(place, env, lends);
            expr(value, env, lends);
        }
        Stmt::Expr(e) => expr(e, env, lends),
        Stmt::Return(Some(e)) => expr(e, env, lends),
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

fn place_exprs(p: &mut Place, env: &mut HashMap<String, Ty>, lends: &HashMap<DeclId, Option<usize>>) {
    for st in &mut p.steps {
        if let Step::Index(e) = st {
            expr(e, env, lends);
        }
    }
}

fn expr(e: &mut Expr, env: &mut HashMap<String, Ty>, lends: &HashMap<DeclId, Option<usize>>) {
    // Children first: a container is read off what the value came from.
    match &mut e.kind {
        ExprKind::Read { place, .. } => place_exprs(place, env, lends),
        ExprKind::Call { args, .. } | ExprKind::MemberCall { args, .. } | ExprKind::Op { args, .. } => {
            args.iter_mut().for_each(|a| expr(a, env, lends));
        }
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Array(xs) | ExprKind::Concat(xs) => xs.iter_mut().for_each(|a| expr(a, env, lends)),
        ExprKind::Construct { fields } => fields.iter_mut().for_each(|(_, a)| expr(a, env, lends)),
        ExprKind::MakeUnion { value, .. }
        | ExprKind::Rewrap { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::Present { value }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value } => expr(value, env, lends),
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms.iter_mut() {
                expr(c, env, lends);
                block(b, env, lends);
            }
            if let Some(o) = otherwise {
                block(o, env, lends);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            expr(subject, env, lends);
            for a in arms.iter_mut() {
                block(&mut a.body, env, lends);
            }
        }
        ExprKind::Test { subject, .. } => expr(subject, env, lends),
        ExprKind::Lambda { params, ret, body, .. } => {
            for p in params.iter_mut() {
                fix_fn_types(&mut p.ty);
                env.insert(p.local.0.clone(), p.ty.clone());
            }
            fix_fn_types(ret);
            if has_ref(ret) {
                if let Some(p) = params.iter().find(|p| !is_scalar(&p.ty) && !matches!(p.ty.strip_quals(), Ty::Fn { .. })) {
                    let c = storage(&p.ty);
                    set_container(ret, &c);
                }
            }
            block(body, env, lends);
        }
        ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => block(body, env, lends),
        ExprKind::Throw { message } => expr(message, env, lends),
        ExprKind::Assert { cond, message, .. } => {
            expr(cond, env, lends);
            if let Some(m) = message {
                expr(m, env, lends);
            }
        }
        ExprKind::Unreachable { message: Some(m), .. } => expr(m, env, lends),
        ExprKind::Spawn { handler, deps, pool, join, .. } => {
            expr(handler, env, lends);
            deps.iter_mut().for_each(|a| expr(a, env, lends));
            if let Some(p) = pool {
                expr(p, env, lends);
            }
            if let Some(j) = join {
                expr(j, env, lends);
            }
        }
        ExprKind::Send { addr, args, .. } => {
            expr(addr, env, lends);
            args.iter_mut().for_each(|a| expr(a, env, lends));
        }
        ExprKind::ReplyTo { captures, pool, .. } => {
            captures.iter_mut().for_each(|a| expr(a, env, lends));
            if let Some(p) = pool {
                expr(p, env, lends);
            }
        }
        _ => {}
    }
    fix_fn_types(&mut e.ty);
    if !has_ref(&e.ty) {
        return;
    }
    let c: Option<Ty> = match &e.kind {
        ExprKind::Read { place, .. } if place.steps.is_empty() => env.get(&place.root.0).and_then(ref_container).cloned(),
        // A callee outside the built modules (a partial dump) lends from its
        // first non-scalar argument, the locator convention.
        ExprKind::Call { target: FnRef::Decl(id), args, .. } => match lends.get(id) {
            Some(k) => k.and_then(|k| args.get(k)).map(|a| storage(&a.ty)),
            None => args.iter().find(|a| !is_scalar(&a.ty) && !matches!(a.ty.strip_quals(), Ty::Fn { .. })).map(|a| storage(&a.ty)),
        },
        ExprKind::Call { target: FnRef::Local(_), args, .. } | ExprKind::MemberCall { args, .. } => {
            args.iter().find(|a| !is_scalar(&a.ty) && !matches!(a.ty.strip_quals(), Ty::Fn { .. })).map(|a| storage(&a.ty))
        }
        ExprKind::Present { value } | ExprKind::Widen { value } | ExprKind::DropMut { value } | ExprKind::Rewrap { value, .. } => ref_container(&value.ty).cloned(),
        ExprKind::Branch { arms, otherwise, .. } => arms
            .iter()
            .filter_map(|(_, b)| b.value.as_ref())
            .chain(otherwise.iter().filter_map(|b| b.value.as_ref()))
            .find_map(|v| ref_container(&v.ty).cloned()),
        ExprKind::Switch { arms, .. } => arms.iter().filter_map(|a| a.body.value.as_ref()).find_map(|v| ref_container(&v.ty).cloned()),
        _ => None,
    };
    if let Some(c) = c {
        set_container(&mut e.ty, &c);
    }
}
