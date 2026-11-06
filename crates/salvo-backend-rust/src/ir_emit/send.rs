//! [rs-fn-field-send] Which type parameters of a generic fn must be
//! `Send + Sync + 'static`.
//!
//! A closure kept past the call — a struct's fn field (`Arc<dyn Fn + Send +
//! Sync>`), a fn value returned, a callback moved into a callee that stores
//! it (`impl Fn + Send + Sync + 'static`) — must be `Send + Sync + 'static`,
//! and so must everything it captures. A generic fn whose closure captures a
//! value of a type mentioning `T` therefore needs `T` bounded so, and so does
//! every generic caller passing a type mentioning its own parameters to such
//! a `T`. Computed once over the program, to a fixpoint, before emission.

use std::collections::{HashMap, HashSet};

use salvo_core::ir::{Decl, DeclId, Expr, ExprKind, FnDecl, FnRef, PassMode, Stmt};
use salvo_core::types::Ty;

/// What one generic fn's body says, before the fixpoint.
struct Summary {
    /// Parameters a stored closure of its own captures.
    direct: HashSet<String>,
    /// Calls to other fns: the callee, and, per callee type parameter left
    /// unknown, the caller's parameters the call's types mention.
    calls: Vec<(DeclId, HashSet<String>)>,
}

/// The type parameters each generic fn must bound `Send + Sync + 'static`.
pub(crate) fn compute(decls: &HashMap<DeclId, &Decl>) -> HashMap<DeclId, HashSet<String>> {
    let fns: HashMap<&DeclId, &FnDecl> = decls
        .iter()
        .filter_map(|(id, d)| match d {
            Decl::Fn(f) if f.body.is_some() => Some((id, f)),
            _ => None,
        })
        .collect();
    let mut summaries: HashMap<DeclId, Summary> = HashMap::new();
    for (id, f) in &fns {
        let generics: HashSet<String> = f.type_params.iter().filter(|t| !t.effect).map(|t| t.name.clone()).collect();
        if generics.is_empty() {
            continue;
        }
        summaries.insert((*id).clone(), summarize(f, &generics, &fns));
    }
    let mut out: HashMap<DeclId, HashSet<String>> = HashMap::new();
    loop {
        let mut grew = false;
        for (id, s) in &summaries {
            let mut set = out.get(id).cloned().unwrap_or_default();
            let before = set.len();
            set.extend(s.direct.iter().cloned());
            for (callee, mentioned) in &s.calls {
                if out.get(callee).is_some_and(|c| !c.is_empty()) {
                    set.extend(mentioned.iter().cloned());
                }
            }
            if set.len() != before {
                grew = true;
                out.insert(id.clone(), set);
            }
        }
        if !grew {
            break;
        }
    }
    out.retain(|_, s| !s.is_empty());
    out
}

fn summarize(f: &FnDecl, generics: &HashSet<String>, fns: &HashMap<&DeclId, &FnDecl>) -> Summary {
    let mut f = f.clone();
    // Every local's type, for a capture's.
    let mut locals: HashMap<String, Ty> = f.params.iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect();
    let fn_ret_is_fn = matches!(f.ret.strip_quals(), Ty::Fn { .. });
    let mut returned: Vec<Expr> = Vec::new();
    salvo_core::ir::walk::fn_stmt_lists_mut(&mut f, &mut |stmts| {
        for s in stmts.iter() {
            match s {
                Stmt::Let { local, ty, .. } | Stmt::ForEach { local, ty, .. } | Stmt::Alias { local, ty, .. } | Stmt::Narrow { local, ty, .. } => {
                    locals.insert(local.0.clone(), ty.clone());
                }
                Stmt::Unpack { locals: ls, .. } => {
                    for (l, t) in ls {
                        locals.insert(l.0.clone(), t.clone());
                    }
                }
                Stmt::Return(Some(e), _) if fn_ret_is_fn => returned.push(e.clone()),
                _ => {}
            }
        }
    });
    let mut stored: Vec<Expr> = returned;
    let mut calls: Vec<(DeclId, HashSet<String>)> = Vec::new();
    salvo_core::ir::walk::fn_exprs_mut(&mut f, &mut |e| {
        match &e.kind {
            ExprKind::Lambda { params, .. } => {
                for p in params {
                    locals.insert(p.local.0.clone(), p.ty.clone());
                }
            }
            ExprKind::Construct { fields } => {
                stored.extend(fields.iter().filter(|(_, v)| matches!(v.kind, ExprKind::Lambda { .. })).map(|(_, v)| v.clone()));
            }
            ExprKind::Call { target: FnRef::Decl(id), args, .. } => {
                if let Some(callee) = fns.get(id) {
                    for (p, a) in callee.params.iter().zip(args) {
                        if p.mode == PassMode::Moved && matches!(p.ty.strip_quals(), Ty::Fn { .. }) && matches!(a.kind, ExprKind::Lambda { .. }) {
                            stored.push(a.clone());
                        }
                    }
                    let mut mentioned = HashSet::new();
                    for a in args {
                        mentions(&a.ty, generics, &mut mentioned);
                    }
                    mentions(&e.ty, generics, &mut mentioned);
                    if !mentioned.is_empty() {
                        calls.push((id.clone(), mentioned));
                    }
                }
            }
            _ => {}
        }
    });
    let mut direct = HashSet::new();
    for e in &stored {
        if let ExprKind::Lambda { captures, .. } = &e.kind {
            for c in captures {
                if let Some(t) = locals.get(&c.local.0) {
                    mentions(t, generics, &mut direct);
                }
            }
        }
    }
    Summary { direct, calls }
}

/// Adds to `out` every one of `generics` that `t` mentions.
fn mentions(t: &Ty, generics: &HashSet<String>, out: &mut HashSet<String>) {
    match t {
        Ty::Var(v) => {
            if generics.contains(&v.to_string()) {
                out.insert(v.to_string());
            }
        }
        Ty::Named { name, args } => {
            if args.is_empty() && generics.contains(&name.to_string()) {
                out.insert(name.to_string());
            }
            for a in args {
                mentions(a, generics, out);
            }
        }
        Ty::Union(args) | Ty::Tuple(args) => {
            for a in args {
                mentions(a, generics, out);
            }
        }
        Ty::Qualified { base, .. } => mentions(base, generics, out),
        Ty::Array(e) => mentions(e, generics, out),
        Ty::Fn { params, ret, effects, .. } => {
            for a in params.iter().chain(effects) {
                mentions(a, generics, out);
            }
            mentions(ret, generics, out);
        }
        _ => {}
    }
}
