//! Mutable lends [mut-lends]: which fns hand back a result that a `Mut`
//! position then reads, and which fns cover `canbe` parameters.
//!
//! Both are closures over Salvo-level facts: the checker's `mut_lend_calls`
//! (a call whose lent result serves a `Mut` position) resolved through
//! `call_fn` and followed along return paths; and the clause written on a fn
//! (`canbe`). What a target does with them is its own business: Rust spells a
//! lent mutable result as a locator and emits a twin of each fn in `fns`; a
//! garbage-collected target ignores them.

use std::collections::HashSet;

use salvo_syntax::ast::*;
use salvo_syntax::Span;

use crate::check::Checked;
use crate::program::Program;
use crate::types::{is_proj_name, Ty};

/// The fns whose lent result serves a `Mut` position (`fns`), the call sites
/// that read one (`sites`), and the return-path calls inside those fns that
/// pass one on (`forwards`)
/// (group-borrowing ladder step ③, user decision 2026-09-24 — option (a)
/// over promoting the total `get` to intrinsic, so *user-written* lending
/// accessors serve `Mut` positions too).
#[derive(Clone)]
pub struct MutLendUses {
    pub fns: HashSet<crate::FnKey>,
    pub sites: HashSet<(usize, Span)>,
    pub forwards: HashSet<(usize, Span)>,
}

/// Seeds are `Checked::mut_lend_calls` resolved through `call_fn`; the
/// closure follows **return-path** lending calls inside each demanded
/// body (an intrinsic forward becomes a mut splice, a named one demands
/// the callee's variant too). A seed with no named callee — a fn value or
/// effect member lending mutably — is the loud v1 cut.
pub fn mut_lend_uses(program: &Program, checked: &Checked) -> MutLendUses {
    let fn_of = |key: crate::FnKey| -> Option<&FnDecl> {
        match program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    };
    let mut demanded: HashSet<crate::FnKey> = HashSet::new();
    let mut seeds = HashSet::new();
    let mut forwards = HashSet::new();
    let mut worklist: Vec<crate::FnKey> = Vec::new();
    // [rs-loc] Bound mints join the seeds (④a slice 2): a `let` the
    // checker marked as a mutable-handle bind (`handle_muts`) whose value
    // is a lending call demands the callee's locator variant exactly as a
    // statement-scoped use does.
    let mut seeds_in: Vec<(usize, Span)> = checked.mut_lend_calls.iter().copied().collect();
    for (file_idx, module) in program.modules.iter().enumerate() {
        for item in &module.items {
            let bodies: Vec<&Block> = match item {
                Item::Fn(f) => f.body.iter().collect(),
                Item::Handler(h) => h.fns.iter().filter_map(|f| f.body.as_ref()).collect(),
                _ => Vec::new(),
            };
            for body in bodies {
                let mut lets: Vec<(Span, &Expr)> = Vec::new();
                collect_let_values(body, &mut lets);
                for (span, value) in lets {
                    if !checked.handle_muts.contains(&(file_idx, span)) {
                        continue;
                    }
                    if let Some(call_span) = mut_lend_call_of(value) {
                        seeds_in.push((file_idx, call_span));
                    }
                }
            }
        }
    }
    // [rs-loc] ④a slice 3′ — a named lending fn **passed as a value** (an
    // implicit fill, the `?at`/`Locate` idiom) is used as a locator by
    // whoever calls it, so its variant is demanded too. The checker
    // resolved every implicit fill, which is where the answer lives.
    let mut value_demanded: Vec<crate::FnKey> = Vec::new();
    for filled in checked.implicit_args.values() {
        for a in filled {
            let fk = match a {
                crate::ImplicitArg::Resolved { key, .. }
                | crate::ImplicitArg::OriginNext { next_fn: key, .. } => *key,
                _ => continue,
            };
            let Some(decl) = fn_of(fk) else { continue };
            if decl.intrinsic || decl.body.is_none() {
                continue;
            }
            if decl
                .return_type
                .as_ref()
                .is_some_and(|rt| fn_type_lends_mut(rt))
            {
                value_demanded.push(fk);
            }
        }
    }
    for key in &seeds_in {
        match checked.call_fn.get(key) {
            // [rs-loc] No named callee: a fn value or effect member,
            // both of which lend through locators (④a slices 3–4) —
            // rendered at the use site, nothing to demand here.
            None => {}
            Some(fk) => match fn_of(*fk) {
                Some(decl) if !decl.intrinsic => {
                    seeds.insert(*key);
                    worklist.push(*fk);
                }
                // An intrinsic lender in a `Mut` position is ①'s
                // statement-scoped splice territory [rs-elem-mut].
                _ => {}
            },
        }
    }
    worklist.extend(value_demanded.iter().copied());
    while let Some(fk) = worklist.pop() {
        if !demanded.insert(fk) {
            continue;
        }
        let Some(decl) = fn_of(fk) else { continue };
        let Some(body) = &decl.body else { continue };
        let mut returned: Vec<&Expr> = Vec::new();
        collect_returned_exprs(body, &mut returned);
        for mut e in returned {
            while let Expr::NonNull { operand, .. } = e {
                e = operand;
            }
            let Expr::Call { span, .. } = e else { continue };
            let Some(fk2) = checked.call_fn.get(&(fk.file, *span)) else {
                continue;
            };
            let Some(decl2) = fn_of(*fk2) else { continue };
            if decl2.derived_return.is_some() {
                forwards.insert((fk.file, *span));
                if !decl2.intrinsic {
                    worklist.push(*fk2);
                }
            }
        }
    }
    MutLendUses {
        fns: demanded,
        sites: seeds,
        forwards,
    }
}

/// [rs-loc] Every expression a block can `return` (explicit `return`s
/// plus the tails of value blocks), for the demand closure's forward walk.
/// Lambda bodies are a barrier: their returns are their own.
pub fn collect_returned_exprs<'a>(block: &'a Block, out: &mut Vec<&'a Expr>) {
    fn expr<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
        match e {
            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    out.push(v);
                    expr(v, out);
                }
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (cond, block) in branches {
                    expr(cond, out);
                    collect_returned_exprs(block, out);
                }
                if let Some(b) = else_block {
                    collect_returned_exprs(b, out);
                }
            }
            Expr::When {
                subject, branches, ..
            } => {
                expr(subject, out);
                for b in branches {
                    collect_returned_exprs(&b.body, out);
                }
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                for (cond, block) in branches {
                    expr(cond, out);
                    collect_returned_exprs(block, out);
                }
                collect_returned_exprs(else_block, out);
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                expr(cond, out);
                collect_returned_exprs(body, out);
                if let Some(b) = else_block {
                    collect_returned_exprs(b, out);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                expr(iterable, out);
                collect_returned_exprs(body, out);
                if let Some(b) = else_block {
                    collect_returned_exprs(b, out);
                }
            }
            Expr::Elvis { subject, rhs, .. } => {
                expr(subject, out);
                expr(rhs, out);
            }
            Expr::NonNull { operand, .. } => expr(operand, out),
            Expr::Try { body, .. } | Expr::WaitFor { body, .. } => {
                collect_returned_exprs(body, out)
            }
            // A lambda's returns are its own.
            Expr::Lambda { .. } => {}
            _ => {}
        }
    }
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { value, .. } => expr(value, out),
            Stmt::Assign { target, value, .. } => {
                expr(target, out);
                expr(value, out);
            }
            Stmt::Use { handler, .. } => expr(handler, out),
            Stmt::Expr(e) => expr(e, out),
            Stmt::Rename(_) => {}
            // [comptime-inline] Gone before emission.
            Stmt::Comp(_) => {}
        }
    }
    // A value block's tail is returned by the construct holding it.
    if let Some(Stmt::Expr(tail)) = block.stmts.last() {
        out.push(tail);
    }
}

/// [rs-loc] Every `let` in a block, recursively (bound mutable-handle
/// mints live in nested branches too). Lambda bodies are a barrier.
pub fn collect_let_values<'a>(block: &'a Block, out: &mut Vec<(Span, &'a Expr)>) {
    fn expr<'a>(e: &'a Expr, out: &mut Vec<(Span, &'a Expr)>) {
        match e {
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    expr(c, out);
                    collect_let_values(b, out);
                }
                if let Some(b) = else_block {
                    collect_let_values(b, out);
                }
            }
            Expr::When { branches, .. } => {
                for b in branches {
                    collect_let_values(&b.body, out);
                }
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    expr(c, out);
                    collect_let_values(b, out);
                }
                collect_let_values(else_block, out);
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                expr(cond, out);
                collect_let_values(body, out);
                if let Some(b) = else_block {
                    collect_let_values(b, out);
                }
            }
            Expr::For {
                body, else_block, ..
            } => {
                collect_let_values(body, out);
                if let Some(b) = else_block {
                    collect_let_values(b, out);
                }
            }
            Expr::Try { body, .. } | Expr::WaitFor { body, .. } => {
                collect_let_values(body, out)
            }
            _ => {}
        }
    }
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { value, span, .. } => {
                out.push((*span, value));
                expr(value, out);
            }
            Stmt::Assign { target, value, .. } => {
                expr(target, out);
                expr(value, out);
            }
            Stmt::Expr(e) => expr(e, out),
            _ => {}
        }
    }
}

/// [rs-loc] Whether a fn type's return is a **wholesale mutable lend** —
/// `-> proj(c) Mut T` or its optional — which is what renders as a
/// locator (④a slice 3): the closure answers position data, and the use
/// site materializes `&mut anchor[loc]`. The held form
/// (`-> T holds proj(c)`) is an owned value carrying borrows and is not
/// this case.
pub fn fn_type_lends_mut(ret: &Type) -> bool {
    fn is_top_ref(t: &Type) -> bool {
        match t {
            Type::Named { qualifiers, .. } => qualifiers.iter().any(|q| q.name.name == "ref"),
            Type::QualifiedGroup { qualifiers, .. } => {
                qualifiers.iter().any(|q| q.name.name == "ref")
            }
            _ => false,
        }
    }
    fn wholesale_mut(t: &Type) -> bool {
        let is_ref_here = is_top_ref(t);
        match strip_top_proj_ast(t) {
            Some(inner) => match &inner {
                Type::Named { qualifiers, .. } => {
                    is_ref_here && qualifiers.iter().any(|q| q.name.name == "Mut")
                }
                other => wholesale_mut(other),
            },
            None => match t {
                Type::Nullable { inner, .. } => wholesale_mut(inner),
                _ => false,
            },
        }
    }
    wholesale_mut(ret)
}

/// [rs-loc] The checker-type sibling of `fn_type_lends_mut`: whether a
/// return type is a **wholesale mutable lend** (`proj Mut T`, or its
/// optional), which renders as a locator.
pub fn ty_lends_mut(ret: &Ty) -> bool {
    let wholesale_mut = |t: &Ty| t.is_ref() && t.quals().iter().any(|q| q.name == "Mut");
    if wholesale_mut(ret) {
        return true;
    }
    match ret {
        Ty::Union(arms) => arms.iter().any(wholesale_mut),
        _ => false,
    }
}

/// [rs-loc] The call span a `Mut`-position argument bottoms out in,
/// through `!` and projection steps — the syntactic mirror of the
/// checker's `mut_lend_call_span`.
pub fn mut_lend_call_of(expr: &Expr) -> Option<Span> {
    let mut e = expr;
    loop {
        match e {
            Expr::NonNull { operand, .. } => e = operand,
            Expr::Field { base, .. } => e = base,
            Expr::TupleIndex { base, .. } => e = base,
            Expr::Index { base, .. } => e = base,
            _ => break,
        }
    }
    match e {
        Expr::Call { span, .. } => Some(*span),
        _ => None,
    }
}


/// [proj-type] The type under a *top-level* `proj` (on a named type or a
/// qualified group), or `None` when the type is not a projection at its
/// top. A Copy scalar under `proj` is the scalar itself [copy-scalar-free].
pub fn strip_top_proj_ast(ty: &Type) -> Option<Type> {
    let is_scalar = |t: &Type| {
        matches!(
            t,
            Type::Named { qualifiers, base }
                if qualifiers.is_empty()
                    && base.args.is_empty()
                    && matches!(
                        base.name.name.as_str(),
                        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                    )
        )
    };
    match ty {
        Type::Named { qualifiers, base } if qualifiers.iter().any(|q| is_proj_name(&q.name.name)) => {
            let rest: Vec<salvo_syntax::ast::TypeRef> = qualifiers
                .iter()
                .filter(|q| !is_proj_name(&q.name.name))
                .cloned()
                .collect();
            let inner = Type::Named {
                qualifiers: rest,
                base: base.clone(),
            };
            if is_scalar(&inner) {
                None
            } else {
                Some(inner)
            }
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } if qualifiers.iter().any(|q| is_proj_name(&q.name.name)) => {
            let rest: Vec<salvo_syntax::ast::TypeRef> = qualifiers
                .iter()
                .filter(|q| !is_proj_name(&q.name.name))
                .cloned()
                .collect();
            if rest.is_empty() {
                Some((**base).clone())
            } else {
                Some(Type::QualifiedGroup {
                    qualifiers: rest,
                    base: base.clone(),
                    span: *span,
                })
            }
        }
        _ => None,
    }
}
