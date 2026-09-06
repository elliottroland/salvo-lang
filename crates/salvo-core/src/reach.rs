//! Module reachability [mod-used-only]: only the modules a program
//! actually uses are transpiled.
//!
//! Roots are the user modules that declare `fn main` (or *all* user
//! modules when no `main` exists — a library compile). From there,
//! modules are reached through *name usage*: every identifier and type
//! name a file mentions is looked up in that file's resolved scope
//! (`ModuleScope::name_origins`), and every module declaring the name
//! under it becomes reachable. This is deliberately conservative — a
//! name shadowed by a local still pulls in the modules that declare it,
//! and an overloaded name pulls in every declaring module — but it never
//! drops a module the emitted code could reference.
//!
//! Only `.sv` sources contribute usage; companion files are carried along
//! with their module [backend-companion].

use std::collections::HashSet;

use salvo_syntax::ast::{
    Block, EffectRef, Expr, FnDecl, Item, LambdaBody, Module, Stmt, StrExprPart,
    StructLitFieldKind, Type, TypeRef,
};

use crate::program::Program;
use crate::resolve::Resolution;
use crate::source::ModulePath;

/// The modules reachable from the program's roots [mod-used-only].
pub fn reachable_modules<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
) -> HashSet<&'p ModulePath> {
    // Roots: user modules declaring `fn main`, else every user module.
    let mut roots: Vec<&ModulePath> = program
        .units()
        .filter(|u| {
            !u.file.is_std
                && u.ast.items.iter().any(
                    |i| matches!(i, Item::Fn(f) if f.name.name == "main" && f.body.is_some()),
                )
        })
        .map(|u| &u.file.module)
        .collect();
    if roots.is_empty() {
        roots = program
            .units()
            .filter(|u| !u.file.is_std)
            .map(|u| &u.file.module)
            .collect();
    }

    let mut reachable: HashSet<&ModulePath> = HashSet::new();
    let mut queue: Vec<&ModulePath> = roots;
    while let Some(module) = queue.pop() {
        if !reachable.insert(module) {
            continue;
        }
        for (file_idx, unit) in program.units().enumerate() {
            if unit.file.module != *module {
                continue;
            }
            let scope = &resolution.scopes[file_idx];
            for name in used_names(unit.ast) {
                for dep in scope.name_origins.get(name).into_iter().flatten() {
                    if !reachable.contains(*dep) {
                        queue.push(dep);
                    }
                }
            }
        }
    }
    reachable
}

/// Every identifier and type name a module's source mentions (types in
/// signatures and annotations, expression identifiers, call targets,
/// handler names, effect references, qualifier names).
pub fn used_names(module: &Module) -> HashSet<&str> {
    let mut used = HashSet::new();
    for item in &module.items {
        match item {
            Item::Fn(f) => fn_names(f, &mut used),
            Item::Struct(s) => {
                for q in &s.auto_qualifiers {
                    type_ref_names(q, &mut used);
                }
                for field in &s.fields {
                    type_names(&field.ty, &mut used);
                    if let Some(d) = &field.default {
                        expr_names(d, &mut used);
                    }
                }
            }
            Item::Effect(e) => {
                for f in &e.fns {
                    fn_names(f, &mut used);
                }
            }
            // [implicit-group] A group's members are signatures, like an
            // effect's: their types are what the group depends on.
            Item::Params(g) => {
                for f in &g.fns {
                    fn_names(f, &mut used);
                }
            }
            Item::Handler(h) => {
                type_names(&h.of, &mut used);
                for p in &h.params {
                    type_names(&p.ty, &mut used);
                }
                for field in &h.state {
                    type_names(&field.ty, &mut used);
                    if let Some(d) = &field.default {
                        expr_names(d, &mut used);
                    }
                }
                for f in &h.fns {
                    fn_names(f, &mut used);
                }
            }
            Item::Qualifier(q) => {
                type_names(&q.of, &mut used);
                for w in &q.with {
                    type_ref_names(w, &mut used);
                }
                for field in &q.field_overrides {
                    type_names(&field.ty, &mut used);
                }
                for f in &q.fns {
                    fn_names(f, &mut used);
                }
            }
            Item::Type(t) => {
                if let Some(alias) = &t.alias {
                    type_names(alias, &mut used);
                }
            }
            // Imports contribute usage only through references to the
            // imported name.
            Item::Import(_)
 => {}
        }
    }
    used
}

fn fn_names<'p>(f: &'p FnDecl, used: &mut HashSet<&'p str>) {
    for p in &f.params {
        type_names(&p.ty, used);
    }
    // [implicit-group] A `?Field<T>` spread makes the group reachable, and
    // its member *names* are what the call site resolves, so a module
    // spreading a group depends on it.
    for g in &f.implicit_groups {
        type_ref_names(g, used);
    }
    for eff in f.effects.iter().flatten() {
        if let EffectRef::Effect(r) = eff {
            type_ref_names(r, used);
        }
    }
    if let Some(rt) = &f.return_type {
        type_names(rt, used);
    }
    if let Some(c) = &f.constructs {
        type_ref_names(c, used);
    }
    if let Some(body) = &f.body {
        block_names(body, used);
    }
}

fn type_names<'p>(ty: &'p Type, used: &mut HashSet<&'p str>) {
    match ty {
        Type::Named { qualifiers, base } => {
            for q in qualifiers {
                type_ref_names(q, used);
            }
            type_ref_names(base, used);
        }
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => {
            for q in qualifiers {
                type_ref_names(q, used);
            }
            type_names(base, used);
        }
        Type::Union { arms, .. } => {
            for a in arms {
                type_names(a, used);
            }
        }
        Type::Tuple { elems, .. } => {
            for e in elems {
                type_names(e, used);
            }
        }
        Type::Array { elem, .. } => type_names(elem, used),
        Type::Nullable { inner, .. } => type_names(inner, used),
        Type::Fn { params, ret, .. } => {
            for p in params {
                type_names(p, used);
            }
            type_names(ret, used);
        }
    }
}

fn type_ref_names<'p>(r: &'p TypeRef, used: &mut HashSet<&'p str>) {
    used.insert(&r.name.name);
    for a in &r.args {
        type_names(a, used);
    }
}

fn block_names<'p>(block: &'p Block, used: &mut HashSet<&'p str>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { ty, value, .. } => {
                if let Some(t) = ty {
                    type_names(t, used);
                }
                expr_names(value, used);
            }
            Stmt::Assign { target, value, .. } => {
                expr_names(target, used);
                expr_names(value, used);
            }
            Stmt::Return { value, .. } | Stmt::Break { value, .. } => {
                if let Some(v) = value {
                    expr_names(v, used);
                }
            }
            Stmt::Yield { value, .. } => expr_names(value, used),
            Stmt::Use { handler, .. } => expr_names(handler, used),
            // [defer] The body is ordinary code run at the block's exits.
            Stmt::Defer { body, .. } => block_names(body, used),
            Stmt::Expr(e) => expr_names(e, used),
            Stmt::Continue { .. } => {}
        }
    }
}

fn expr_names<'p>(expr: &'p Expr, used: &mut HashSet<&'p str>) {
    match expr {
        Expr::Ident(id) => {
            used.insert(&id.name);
        }
        // [try] The delimiter's body is ordinary code.
        Expr::Try { body, .. } => block_names(body, used),
        // [qual-widen] The qualifier names are type references.
        Expr::Widen { subject, quals, .. } => {
            expr_names(subject, used);
            for q in quals {
                type_ref_names(q, used);
            }
        }
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            expr_names(base, used)
        }
        Expr::Call {
            callee,
            type_args,
            args,
            ..
        } => {
            // Dot-notation targets are names too: `x.f()` may resolve to
            // a foreign fn `f`.
            if let Expr::Field { base, field, .. } = callee.as_ref() {
                used.insert(&field.name);
                expr_names(base, used);
            } else {
                expr_names(callee, used);
            }
            for t in type_args {
                type_names(t, used);
            }
            for a in args {
                expr_names(a, used);
            }
        }
        Expr::Index { base, index, .. } => {
            expr_names(base, used);
            expr_names(index, used);
        }
        Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
            for e in elems {
                expr_names(e, used);
            }
        }
        Expr::ArrayInit {
            elem_type,
            size,
            init,
            ..
        } => {
            type_ref_names(elem_type, used);
            expr_names(size, used);
            expr_names(init, used);
        }
        Expr::StructLit { ty, fields, .. } => {
            if let Some(t) = ty {
                type_names(t, used);
            }
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => expr_names(value, used),
                    StructLitFieldKind::Spread(e) => expr_names(e, used),
                }
            }
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::PostIncrement { operand, .. }
        | Expr::Spread { operand, .. } => expr_names(operand, used),
        Expr::Binary { lhs, rhs, .. } => {
            expr_names(lhs, used);
            expr_names(rhs, used);
        }
        Expr::Is { subject, check, .. } => {
            expr_names(subject, used);
            for r in check {
                type_ref_names(r, used);
            }
        }
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    expr_names(e, used);
                }
            }
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                expr_names(c, used);
                block_names(b, used);
            }
            if let Some(b) = else_block {
                block_names(b, used);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            expr_names(subject, used);
            for b in branches {
                for r in &b.check {
                    type_ref_names(r, used);
                }
                block_names(&b.body, used);
            }
        }
        // [when-condition] The subject-less form is a condition chain.
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (cond, body) in branches {
                expr_names(cond, used);
                block_names(body, used);
            }
            block_names(else_block, used);
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            expr_names(cond, used);
            block_names(body, used);
            if let Some(b) = else_block {
                block_names(b, used);
            }
        }
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => {
            expr_names(iterable, used);
            block_names(body, used);
            if let Some(b) = else_block {
                block_names(b, used);
            }
        }
        Expr::Lambda { params, body, .. } => {
            for p in params {
                if let Some(t) = &p.ty {
                    type_names(t, used);
                }
            }
            match body {
                LambdaBody::Expr(e) => expr_names(e, used),
                LambdaBody::Block(b) => block_names(b, used),
            }
        }
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}
