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
        Stmt::Expr(e) | Stmt::Return(Some(e), _) => expr(e, f),
        Stmt::Return(None, _) | Stmt::Break | Stmt::Continue => {}
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

/// Calls `f` on every expression inside `e`, `e` included, each after its
/// children (so in evaluation order).
pub fn expr_mut(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    expr(e, f);
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
        | ExprKind::IntoMut { value }
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

/// [actor-sent] Calls `f` on every type a declaration holds: its
/// signature's, its statements' and every expression's, at any depth. What
/// an instantiation of a generic declaration rewrites with.
pub fn decl_tys_mut(d: &mut Decl, f: &mut impl FnMut(&mut Ty)) {
    match d {
        Decl::Fn(fd) => fn_tys(fd, f),
        Decl::Impl(h) => impl_tys(h, f),
        Decl::Struct(s) => {
            for fl in &mut s.fields {
                field_tys(fl, f);
            }
        }
        Decl::Union(u) => f(&mut u.ty),
        Decl::Interface(i) => {
            for m in &mut i.members {
                for p in &mut m.params {
                    f(&mut p.ty);
                }
                f(&mut m.ret);
            }
            for p in &mut i.prereqs {
                f(p);
            }
        }
        Decl::Static(st) => {
            f(&mut st.ty);
            for s in &mut st.stmts {
                stmt_tys(s, f);
            }
        }
        Decl::Enum(e) => {
            for v in &mut e.variants {
                for (_, t) in &mut v.fields {
                    f(t);
                }
            }
        }
        Decl::PlatformType(_) => {}
    }
}

/// [actor-sent] Every type an impl holds.
pub fn impl_tys(h: &mut ImplDecl, f: &mut impl FnMut(&mut Ty)) {
    for t in &mut h.faces {
        f(t);
    }
    for p in &mut h.ctor_params {
        f(&mut p.ty);
    }
    for t in &mut h.deps {
        f(t);
    }
    for fl in &mut h.state {
        field_tys(fl, f);
    }
    if let Some(e) = &mut h.mailbox {
        expr_tys(e, f);
    }
    for fd in h.members.iter_mut().chain(h.init.iter_mut()) {
        fn_tys(fd, f);
    }
}

fn field_tys(fl: &mut Field, f: &mut impl FnMut(&mut Ty)) {
    f(&mut fl.ty);
    if let Some(e) = &mut fl.default {
        expr_tys(e, f);
    }
}

/// [actor-sent] Every type a fn holds.
pub fn fn_tys(fd: &mut FnDecl, f: &mut impl FnMut(&mut Ty)) {
    for p in &mut fd.params {
        f(&mut p.ty);
    }
    f(&mut fd.ret);
    if let Some(t) = &mut fd.throws {
        f(t);
    }
    if let Some(b) = &mut fd.body {
        block_tys(b, f);
    }
}

fn block_tys(b: &mut Block, f: &mut impl FnMut(&mut Ty)) {
    for s in &mut b.stmts {
        stmt_tys(s, f);
    }
    if let Some(v) = &mut b.value {
        expr_tys(v, f);
    }
}

fn place_tys(p: &mut Place, f: &mut impl FnMut(&mut Ty)) {
    for s in &mut p.steps {
        if let Step::Index(e) = s {
            expr_tys(e, f);
        }
    }
}

fn stmt_tys(s: &mut Stmt, f: &mut impl FnMut(&mut Ty)) {
    match s {
        Stmt::Loop { body, .. } => block_tys(body, f),
        Stmt::ForEach { ty, iterable, body, .. } => {
            f(ty);
            expr_tys(iterable, f);
            block_tys(body, f);
        }
        Stmt::Let { ty, value, .. } => {
            f(ty);
            expr_tys(value, f);
        }
        Stmt::Alias { ty, place, .. } => {
            f(ty);
            place_tys(place, f);
        }
        Stmt::Narrow { ty, from, from_ty, .. } => {
            f(ty);
            f(from_ty);
            place_tys(from, f);
        }
        Stmt::Unpack { locals, from, from_ty, .. } => {
            for (_, t) in locals {
                f(t);
            }
            f(from_ty);
            place_tys(from, f);
        }
        Stmt::Assign { place, value } => {
            place_tys(place, f);
            expr_tys(value, f);
        }
        Stmt::Expr(e) | Stmt::Return(Some(e), _) => expr_tys(e, f),
        Stmt::Return(None, _) | Stmt::Break | Stmt::Continue | Stmt::Obligation(_) => {}
    }
}

fn exprs_tys(xs: &mut [Expr], f: &mut impl FnMut(&mut Ty)) {
    for x in xs {
        expr_tys(x, f);
    }
}

fn expr_tys(e: &mut Expr, f: &mut impl FnMut(&mut Ty)) {
    f(&mut e.ty);
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
        ExprKind::Read { place, .. } => place_tys(place, f),
        ExprKind::Call { type_args, args, .. } => {
            for t in type_args {
                f(t);
            }
            exprs_tys(args, f);
        }
        ExprKind::Op { args, .. } | ExprKind::SelfSend { args, .. } => exprs_tys(args, f),
        ExprKind::MemberCall { instance, type_args, args, .. } => {
            expr_tys(instance, f);
            for t in type_args {
                f(t);
            }
            exprs_tys(args, f);
        }
        ExprKind::HandlerCall { instance, args, .. } => {
            expr_tys(instance, f);
            exprs_tys(args, f);
        }
        ExprKind::Construct { fields } => {
            for (_, v) in fields {
                expr_tys(v, f);
            }
        }
        ExprKind::Rewrap { from, value } => {
            f(from);
            expr_tys(value, f);
        }
        ExprKind::BorrowAs { value, arms } => {
            for a in arms {
                if let Some(t) = &mut a.from {
                    f(t);
                }
                f(&mut a.to);
            }
            expr_tys(value, f);
        }
        ExprKind::MakeUnion { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::IntoMut { value }
        | ExprKind::Present { value }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value } => expr_tys(value, f),
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Array(xs) | ExprKind::Concat(xs) => exprs_tys(xs, f),
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms {
                expr_tys(c, f);
                block_tys(b, f);
            }
            if let Some(o) = otherwise {
                block_tys(o, f);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            expr_tys(subject, f);
            for a in arms {
                block_tys(&mut a.body, f);
            }
        }
        ExprKind::Test { subject, .. } => expr_tys(subject, f),
        ExprKind::Lambda { params, ret, body, .. } => {
            for p in params {
                f(&mut p.ty);
            }
            f(ret);
            block_tys(body, f);
        }
        ExprKind::Try { body } => block_tys(body, f),
        ExprKind::WaitFor { token_ty, body, .. } => {
            f(token_ty);
            block_tys(body, f);
        }
        ExprKind::Throw { message } => expr_tys(message, f),
        ExprKind::Assert { cond, message, .. } => {
            expr_tys(cond, f);
            if let Some(m) = message {
                expr_tys(m, f);
            }
        }
        ExprKind::Unreachable { message, .. } => {
            if let Some(m) = message {
                expr_tys(m, f);
            }
        }
        ExprKind::Spawn { handler, deps, pool, join, effects } => {
            expr_tys(handler, f);
            exprs_tys(deps, f);
            if let Some(p) = pool {
                expr_tys(p, f);
            }
            if let Some(j) = join {
                expr_tys(j, f);
            }
            for t in effects {
                f(t);
            }
        }
        ExprKind::Send { addr, args, .. } => {
            expr_tys(addr, f);
            exprs_tys(args, f);
        }
        ExprKind::ReplyTo { target, captures, pool, .. } => {
            if let ReplyTarget::Task { effects, .. } = target {
                exprs_tys(effects, f);
            }
            exprs_tys(captures, f);
            if let Some(p) = pool {
                expr_tys(p, f);
            }
        }
        ExprKind::Handle { instance } => expr_tys(instance, f),
        ExprKind::AddrInstance { addr } => expr_tys(addr, f),
        ExprKind::UnionToStr { value, arms } => {
            expr_tys(value, f);
            exprs_tys(arms, f);
        }
    }
}

/// [actor-sent] Calls `f` on every statement list of a fn, innermost
/// first: what a rewrite that replaces one statement by several walks.
pub fn fn_stmt_lists_mut(fd: &mut FnDecl, f: &mut impl FnMut(&mut Vec<Stmt>)) {
    if let Some(b) = &mut fd.body {
        block_lists(b, f);
    }
}

fn block_lists(b: &mut Block, f: &mut impl FnMut(&mut Vec<Stmt>)) {
    for s in &mut b.stmts {
        stmt_lists(s, f);
    }
    if let Some(v) = &mut b.value {
        expr_lists(v, f);
    }
    f(&mut b.stmts);
}

fn stmt_lists(s: &mut Stmt, f: &mut impl FnMut(&mut Vec<Stmt>)) {
    match s {
        Stmt::Loop { body, .. } => block_lists(body, f),
        Stmt::ForEach { iterable, body, .. } => {
            expr_lists(iterable, f);
            block_lists(body, f);
        }
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => expr_lists(value, f),
        Stmt::Expr(e) | Stmt::Return(Some(e), _) => expr_lists(e, f),
        _ => {}
    }
}

fn expr_lists(e: &mut Expr, f: &mut impl FnMut(&mut Vec<Stmt>)) {
    match &mut e.kind {
        ExprKind::Branch { arms, otherwise, .. } => {
            for (_, b) in arms {
                block_lists(b, f);
            }
            if let Some(o) = otherwise {
                block_lists(o, f);
            }
        }
        ExprKind::Switch { arms, .. } => {
            for a in arms {
                block_lists(&mut a.body, f);
            }
        }
        ExprKind::Lambda { body, .. } | ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => block_lists(body, f),
        _ => {}
    }
}
