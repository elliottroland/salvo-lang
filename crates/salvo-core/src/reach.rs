//! Module reachability [mod-used-only]: only the modules a program
//! actually uses are transpiled.
//!
//! Roots are the user modules that declare `fn main` (or *all* user
//! modules when no `main` exists — a library compile). From there, a
//! module is reached two ways:
//!
//! * **By resolution**, for functions (user decision 2026-09-25): the
//!   checker recorded which declaration every call, fn-name reference,
//!   filled implicit, resolved operator, interpolated `to_str` and driven
//!   `next`/`iter` actually resolved to, so the edge follows *that*. A
//!   name-based edge cannot: it resolves a *name* to every module
//!   declaring it, so a program that merely iterates — using the name
//!   `next` — pulled in `core.range` and whatever it drags, because
//!   `core.range` declares a `next` too.
//! * **By name**, for everything else: a type, struct, effect, handler,
//!   qualifier or `params` group a file mentions is looked up in that
//!   file's resolved scope (`ModuleScope::name_origins`) and every module
//!   declaring it becomes reachable. These names are not overload sets, so
//!   the conservatism costs little — and a name that is *both* a function
//!   and a type somewhere in scope keeps its name edge, since only the
//!   pure-fn case is precise enough to narrow.
//!
//! The direction of the two errors is worth keeping in mind: an edge too
//! many emits a module nothing calls (wasteful), an edge too few emits a
//! call with nothing to bind to (broken). That is why the fn narrowing
//! draws on every resolution table there is rather than only on calls.
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
///
/// Edges are *name* usage (see the module docs) **plus** the checker's
/// resolved-but-unnamed callees (`resolved_dep_files`): an interpolation never
/// names the `to_str` it resolved to and a `for` names neither its `iter` nor
/// its `next`, so a module reached only that way was left un-emitted and the
/// call had nothing to bind to (fixed 2026-09-25).
pub fn reachable_modules<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    checked: &crate::check::Checked,
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
                // [mod-used-only] A name that is *only* a function here is
                // resolved rather than guessed: the tables below say which
                // declaration each use meant.
                if is_only_a_fn_name(scope, name) {
                    continue;
                }
                for dep in scope.name_origins.get(name).into_iter().flatten() {
                    if !reachable.contains(*dep) {
                        queue.push(dep);
                    }
                }
            }
            for dep_file in resolved_dep_files(checked, file_idx) {
                let Some(dep) = program.units().nth(dep_file) else { continue };
                if !reachable.contains(&dep.file.module) {
                    queue.push(&dep.file.module);
                }
            }
        }
    }
    reachable
}

/// [mod-used-only] The files whose **functions** a file resolved to: every
/// call, fn-name reference, filled implicit, resolved comparison, carried
/// identity, interpolated `to_str` and driven `next`/`iter`. This is both the
/// reachability edge for fn names and what each backend's import computation
/// reads, so the two cannot disagree about what a file depends on.
///
/// Every table that records "this reference means *that* declaration" belongs
/// here. Some are the only record there is — an interpolation never names the
/// `to_str` the checker picked for it, and a `for` names neither the `iter` it
/// mints with nor the `next` it drives — and the rest replace a name-based
/// guess with the answer.
///
/// Answers *file* indices, which each backend maps to its own module naming.
pub fn resolved_dep_files(checked: &crate::check::Checked, file_idx: usize) -> HashSet<usize> {
    let mut out: HashSet<usize> = HashSet::new();
    let mine = |key: &&(usize, salvo_syntax::Span)| key.0 == file_idx;
    // Calls, and fn *names* used any other way (a fn passed by value, a
    // callee, a declaration site) [fn-ref-table].
    for (_, fn_key) in checked.call_fn.iter().filter(|(k, _)| mine(k)) {
        out.insert(fn_key.file);
    }
    for (_, fn_key) in checked.fn_refs.iter().filter(|(k, _)| mine(k)) {
        out.insert(fn_key.file);
    }
    // [implicit-resolve] What fills an implicit position — `?cmp`, `?hash`,
    // a group's `next` — is a function the emitted call hands over.
    for (_, args) in checked.implicit_args.iter().filter(|(k, _)| mine(k)) {
        for arg in args {
            if let crate::check::ImplicitArg::Resolved { key, .. } = arg {
                out.insert(key.file);
            }
        }
    }
    // [cmp-groups] An operator names nothing: `a < b` resolves to a `cmp`.
    for (_, via) in checked.comparisons.iter().filter(|(k, _)| mine(k)) {
        if let crate::check::CompareVia::Call(key) = via {
            out.insert(key.file);
        }
    }
    // [interp-to-str] [interp-struct] The text form an interpolation resolved.
    for (_, fn_key) in checked.interp_to_str.iter().filter(|(k, _)| mine(k)) {
        out.insert(fn_key.file);
    }
    // [iter-resolve] The `next` a `for` drives and the `iter` it mints with.
    for (_, driver) in checked.for_drivers.iter().filter(|(k, _)| mine(k)) {
        if let Some(key) = driver.next.key() {
            out.insert(key.file);
        }
        if let Some(key) = driver.mint_iter_fn {
            out.insert(key.file);
        }
    }
    // [cmp-carry] An identity a keyed container carries reaches this answer
    // through `fn_refs`, where the checker records it at the span that names
    // it — per *file*, which the `carried_identities` table is not: it is
    // keyed by (name, subject), so scanning it whole made every program
    // depend on every identity any program mentioned (an empty `main` emitted
    // the sequence helpers, which is how it was caught).
    out.remove(&file_idx);
    out
}

/// [mod-used-only] Whether a name is, in this scope, **only** a function —
/// the case resolution can answer precisely. A name that also denotes a
/// struct, effect, handler, qualifier, `params` group or type keeps its
/// name-based edge: those are not overload sets, so the conservatism is
/// cheap, and mixing the two rules for one name would need per-module
/// knowledge that `name_origins` does not carry.
fn is_only_a_fn_name(scope: &crate::resolve::ModuleScope<'_>, name: &str) -> bool {
    scope.fns.contains_key(name)
        && !scope.structs.contains_key(name)
        && !scope.effects.contains_key(name)
        && !scope.handlers.contains_key(name)
        && !scope.qualifiers.contains_key(name)
        && !scope.param_groups.contains_key(name)
        && !scope.type_aliases.contains_key(name)
        && !scope.opaque_types.contains_key(name)
        // An effect *member* shares a name with no module-level fn worth
        // narrowing: the member travels with its effect, which is a type
        // name and keeps its own edge.
        && !scope.effect_members.contains_key(name)
}

/// Every identifier and type name a module's source mentions (types in
/// signatures and annotations, expression identifiers, call targets,
/// handler names, effect references, qualifier names).
pub fn used_names(module: &Module) -> HashSet<&str> {
    let mut used = HashSet::new();
    for item in &module.items {
        match item {
            Item::Fn(f) => fn_names(f, &mut used),
            // [fn-rename] The renamed function's name and its parameter types
            // are mentioned, so whatever module declares them is reached
            // [mod-used-only].
            Item::Rename(r) => {
                used.insert(&r.target.name);
                for p in &r.params {
                    type_names(&p.ty, &mut used);
                }
            }
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
                for of in &h.of {
                    type_names(of, &mut used);
                }
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
            // [qual-refn] A refinement contributes *no* reachability edge.
            // It emits nothing, and everything it names is compile-time
            // only: qualifiers are erased [qual-erasure] and a refinement
            // never calls the function it refines. Counting its names would
            // pull a module into the output for a statement no generated
            // code mentions.
            Item::Refn(_) => {}
            // [test-decl] An unexpanded `test` reaches here only in a
            // production file, where resolution refuses it [test-file]; in an
            // annex it is already an ordinary fn. Its body's names are
            // counted anyway, so the refusal and this agree about what the
            // module depends on.
            Item::Test(t) => block_names(&t.body, &mut used),
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
        if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
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
            // [fn-rename]
            Stmt::Rename(r) => {
                used.insert(&r.target.name);
                for p in &r.params {
                    type_names(&p.ty, used);
                }
            }
            Stmt::Assign { target, value, .. } => {
                expr_names(target, used);
                expr_names(value, used);
            }
            Stmt::Use { handler, .. } => expr_names(handler, used),
            Stmt::Expr(e) => expr_names(e, used),
        }
    }
}

fn expr_names<'p>(expr: &'p Expr, used: &mut HashSet<&'p str>) {
    match expr {
        Expr::Assert { cond, message, .. } => {
            expr_names(cond, used);
            if let Some(m) = message {
                expr_names(m, used);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                expr_names(m, used);
            }
        }
        // [elvis] Names used on either side keep their modules reachable.
        Expr::Elvis { subject, rhs, .. } => {
            expr_names(subject, used);
            expr_names(rhs, used);
        }
        // [safe-call] The inner access names the function [mod-used-only].
        Expr::SafeField { inner, .. } => expr_names(inner, used),
        Expr::Placeholder { .. } => {}
        Expr::Ident(id) => {
            used.insert(&id.name);
        }
        // [try] The delimiter's body is ordinary code.
        Expr::Try { body, .. } => block_names(body, used),
        // [expr-escape] `return f(x)` uses `f` and `x`.
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                expr_names(v, used);
            }
        }
        Expr::Continue { .. } => {}
        // [actor-spawn-expr] The handler name and every clause count as
        // uses: this is what stops an imported handler or a `pool` function
        // being reported as unused because it is only spawned.
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            expr_names(handler, used);
            for handler in with_items {
                expr_names(handler, used);
            }
            if let Some(pool) = pool {
                expr_names(pool, used);
            }
        }
        // [actor-self-send] The member is the enclosing handler's, so the
        // selector names nothing a module could provide.
        Expr::SelfScoped { .. } => {}
        // [actor-replyto] The member name is resolved against the enclosing
        // handler, not the module, so only the captures name things here.
        Expr::ReplyTo {
            member,
            captures,
            pool,
            ..
        } => {
            // [task-mint] The target may be a **free `send fn`** in another
            // module, and the emitted continuation names it — so the mint is a
            // use of that name. Conservative in the way this whole pass is: a
            // *member* mint names nothing importable, and a module pulled in
            // for a name the file does not otherwise use costs one glob.
            used.insert(member.name.as_str());
            for capture in captures {
                expr_names(capture, used);
            }
            if let Some(pool) = pool {
                expr_names(pool, used);
            }
        }
        // [actor-waitfor] Its block is ordinary code, and the token's
        // written type names `Reply` (and its payload) for real.
        Expr::WaitFor { ty, body, .. } => {
            // [waitfor-infer] The type is optional; an inferred one names no
            // module of its own beyond what the block already uses.
            if let Some(ty) = ty {
                type_names(ty, used);
            }
            block_names(body, used);
        }
        // [qual-lift] The qualifier names are type references.
        Expr::Widen { subject, quals, .. } => {
            expr_names(subject, used);
            for q in quals {
                type_ref_names(q, used);
            }
        }
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            expr_names(base, used)
        }
        // [fn-overload-at] `f@core.list(x)` uses the name *and* names the
        // module explicitly — both matter for reachability [mod-used-only].
        Expr::Scoped { base, name, .. } | Expr::EffectScoped { base, name, .. } => {
            used.insert(&name.name);
            if let Some(base) = base {
                expr_names(base, used);
            }
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
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                expr_names(e, used);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                expr_names(k, used);
                expr_names(v, used);
            }
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
        | Expr::IncDec { operand, .. }
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
