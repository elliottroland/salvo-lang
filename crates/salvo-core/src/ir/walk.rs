//! A mutable walk over every expression of a program, children before
//! parents: what a backend's pre-pass rewrites with (a node it reads as
//! another, say), so no backend keeps a walker of its own in step with
//! [`ExprKind`].

use crate::ir::*;

/// Calls `f` on every expression of the program, each after its children.
pub fn exprs_mut(program: &mut Program, f: &mut impl FnMut(&mut Expr)) {
    for m in &mut program.modules {
        for d in &mut m.decls {
            match d {
                Decl::Fn(fd) => fn_decl(fd, f),
                Decl::Impl(h) => {
                    for s in &mut h.state {
                        if let Some(e) = &mut s.default {
                            expr(e, f);
                        }
                    }
                    if let Some(e) = &mut h.mailbox {
                        expr(e, f);
                    }
                    for fd in h.members.iter_mut().chain(h.init.iter_mut()) {
                        fn_decl(fd, f);
                    }
                }
                Decl::Struct(s) => {
                    for fl in &mut s.fields {
                        if let Some(e) = &mut fl.default {
                            expr(e, f);
                        }
                    }
                }
                Decl::Static(s) => {
                    for st in &mut s.stmts {
                        stmt(st, f);
                    }
                }
                _ => {}
            }
        }
    }
}

fn fn_decl(fd: &mut FnDecl, f: &mut impl FnMut(&mut Expr)) {
    if let Some(b) = &mut fd.body {
        block(b, f);
    }
}

fn block(b: &mut Block, f: &mut impl FnMut(&mut Expr)) {
    for s in &mut b.stmts {
        stmt(s, f);
    }
    if let Some(v) = &mut b.value {
        expr(v, f);
    }
}

fn place(p: &mut Place, f: &mut impl FnMut(&mut Expr)) {
    for s in &mut p.steps {
        if let Step::Index(e) = s {
            expr(e, f);
        }
    }
}

fn stmt(s: &mut Stmt, f: &mut impl FnMut(&mut Expr)) {
    match s {
        Stmt::Loop { body, .. } => block(body, f),
        Stmt::ForEach { iterable, body, .. } => {
            expr(iterable, f);
            block(body, f);
        }
        Stmt::Let { value, .. } => expr(value, f),
        Stmt::Alias { place: p, .. } | Stmt::Narrow { from: p, .. } | Stmt::Unpack { from: p, .. } => place(p, f),
        Stmt::Assign { place: p, value } => {
            place(p, f);
            expr(value, f);
        }
        Stmt::Expr(e) | Stmt::Return(Some(e)) => expr(e, f),
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        Stmt::Obligation(o) => {
            for r in &mut o.refs {
                place(&mut r.place, f);
            }
        }
    }
}

fn all(xs: &mut [Expr], f: &mut impl FnMut(&mut Expr)) {
    for x in xs {
        expr(x, f);
    }
}

fn expr(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    match &mut e.kind {
        ExprKind::Int(_)
        | ExprKind::Long(_)
        | ExprKind::Float(_)
        | ExprKind::Double(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Unit
        | ExprKind::MakeNone
        | ExprKind::FnValue(_)
        | ExprKind::SelfAddr
        | ExprKind::Unsupported(_) => {}
        ExprKind::Read { place: p, .. } => place(p, f),
        ExprKind::Call { args, .. } | ExprKind::Op { args, .. } | ExprKind::SelfSend { args, .. } => all(args, f),
        ExprKind::MemberCall { instance, args, .. } | ExprKind::HandlerCall { instance, args, .. } => {
            expr(instance, f);
            all(args, f);
        }
        ExprKind::Construct { fields } => {
            for (_, v) in fields {
                expr(v, f);
            }
        }
        ExprKind::MakeUnion { value, .. }
        | ExprKind::Rewrap { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::Present { value }
        | ExprKind::BorrowAs { value, .. }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value } => expr(value, f),
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Array(xs) | ExprKind::Concat(xs) => all(xs, f),
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms {
                expr(c, f);
                block(b, f);
            }
            if let Some(o) = otherwise {
                block(o, f);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            expr(subject, f);
            for a in arms {
                block(&mut a.body, f);
            }
        }
        ExprKind::Test { subject, .. } => expr(subject, f),
        ExprKind::Lambda { body, .. } | ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => block(body, f),
        ExprKind::Throw { message } => expr(message, f),
        ExprKind::Assert { cond, message, .. } => {
            expr(cond, f);
            if let Some(m) = message {
                expr(m, f);
            }
        }
        ExprKind::Unreachable { message, .. } => {
            if let Some(m) = message {
                expr(m, f);
            }
        }
        ExprKind::Spawn { handler, deps, pool, join, .. } => {
            expr(handler, f);
            all(deps, f);
            if let Some(p) = pool {
                expr(p, f);
            }
            if let Some(j) = join {
                expr(j, f);
            }
        }
        ExprKind::Send { addr, args, .. } => {
            expr(addr, f);
            all(args, f);
        }
        ExprKind::ReplyTo { target, captures, pool, .. } => {
            if let ReplyTarget::Task { effects, .. } = target {
                all(effects, f);
            }
            all(captures, f);
            if let Some(p) = pool {
                expr(p, f);
            }
        }
        ExprKind::Handle { instance } => expr(instance, f),
        ExprKind::AddrInstance { addr } => expr(addr, f),
        ExprKind::UnionToStr { value, arms } => {
            expr(value, f);
            all(arms, f);
        }
    }
    f(e);
}

/// [proj-opt-slot] An opt-in pass for a backend whose borrow of a single
/// value *is* the borrowed value's own rendering (both backends, today): a
/// [`ExprKind::BorrowAs`] over one arm (`proj X` → `X`, `ref Mut X` → `Mut
/// X`) becomes its value — kept as a [`ExprKind::DropMut`] when the reading
/// drops a `Mut`, which a backend may still render as a conversion. Unions
/// are left for the backend to lower.
pub fn unwrap_single_borrows(program: &mut Program) {
    exprs_mut(program, &mut |e| {
        let ExprKind::BorrowAs { value, arms } = &mut e.kind else { return };
        if arms.len() != 1 || matches!(e.ty, Ty::Union(_)) || matches!(value.ty, Ty::Union(_)) {
            return;
        }
        let drops = arms[0].from.as_ref().is_some_and(|f| f.quals().iter().any(|q| q.name == "Mut"))
            && !arms[0].to.quals().iter().any(|q| q.name == "Mut");
        let inner = std::mem::replace(value.as_mut(), Expr { ty: Ty::none(), span: e.span, kind: ExprKind::Unit });
        *e = if drops {
            Expr { ty: e.ty.clone(), span: e.span, kind: ExprKind::DropMut { value: Box::new(inner) } }
        } else {
            inner
        };
    });
}
