//! Emission helpers both backends share (ROADMAP §0j step 3): syntax
//! walkers, type-name helpers and generated-name policies that were
//! duplicated byte for byte in the two emitters. Backend-neutral by rule:
//! nothing here may cite a backend-prefixed label (AGENTS.md).

use std::collections::{HashMap, HashSet};

use salvo_core::types::Ty;
use salvo_core::check::UnionTest;
use salvo_syntax::ast::*;
use salvo_syntax::Span;

/// One generated file: a path relative to the target directory and its
/// content.
#[derive(Debug, Clone)]
pub struct EmittedFile {
    pub rel_path: std::path::PathBuf,
    pub content: String,
}

/// [backend-companion] Two emitted files with one path is a **clobber**: the
/// second write wins and the first module's code silently vanishes. It has
/// happened twice — a runtime file and a std module of the same name — so the
/// assembly refuses rather than overwriting [backend-never-wrong]. Renaming the
/// runtime file is the fix each time (`hosttime`, `throwsignal`); the durable
/// one is a namespace for the runtime, recorded in ROADMAP.
pub fn no_duplicate_paths(files: &[EmittedFile]) -> Result<(), Vec<String>> {
    let mut seen: std::collections::HashMap<&std::path::Path, usize> =
        std::collections::HashMap::new();
    for f in files {
        *seen.entry(f.rel_path.as_path()).or_default() += 1;
    }
    let mut clashes: Vec<String> = seen
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(path, n)| {
            format!(
                "two emitted files claim `{}` ({n} of them): a module's path and a \
                 runtime file collide, so one would silently overwrite the other — \
                 rename the runtime file [backend-companion]",
                path.display()
            )
        })
        .collect();
    if clashes.is_empty() {
        Ok(())
    } else {
        clashes.sort();
        Err(clashes)
    }
}

/// [platform-check] The payload type of a `Reply<T>` parameter.
pub fn reply_payload(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Named { base, .. } if base.name.name == "Reply" && base.args.len() == 1 => base.args.first(),
        _ => None,
    }
}

/// [platform-abi] The name an item is kept by in ABI mode.
pub fn abi_item_name(item: &Item) -> Option<&str> {
    match item {
        Item::Struct(s) => Some(&s.name.name),
        Item::Type(t) if t.alias.is_some() => Some(&t.name.name),
        Item::Effect(e) => Some(&e.name.name),
        _ => None,
    }
}

/// [qual-lift] Visits every `^` check a condition applies, including
/// through `&&` chains and a `!`-free `||` (the same shape `is` bindings
/// walk).
pub fn collect_widen_checks<'a>(cond: &'a Expr, f: &mut impl FnMut(&'a Expr, Span)) {
    match cond {
        // [qual-lift] A lift **with a binding** materializes the value into
        // that name instead, so there is no shadow to make: the binding is
        // emitted by the `is`-binding path, and a multi-arm lift narrows
        // nothing to shadow anyway.
        Expr::Widen {
            subject,
            binding: None,
            span,
            ..
        } => f(subject, *span),
        Expr::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
            ..
        } => {
            collect_widen_checks(lhs, f);
            collect_widen_checks(rhs, f);
        }
        _ => {}
    }
}

/// [actor-types] A message variant's name, after the member: upper-camel,
/// which both hosts accept without a warning.
pub fn msg_variant_name(member: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in member.chars() {
        if c == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether an effect instance is the throw effect [throw]: it is not a
/// capability parameter, it is a return-shape change.
pub fn is_throw_effect_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT)
}

/// Does this module contain anything that turns into Rust code?
pub fn module_produces_code(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Struct(s) => !s.comptime,
        Item::Effect(_) => true,
        Item::Handler(_) => true,
        Item::Fn(f) => f.body.is_some() || f.platform,
        Item::Qualifier(q) => q.fns.iter().any(|f| f.body.is_some()),
        _ => false,
    })
}

/// [type-identity] [`type_base_name`] for a reference to a declaration:
/// the key the written name resolved to.
pub fn type_key_name<'a>(checked: &'a salvo_core::Checked, ty: &'a Type) -> Option<&'a str> {
    match ty {
        Type::Named { base, .. } => Some(checked.written_key(base)),
        Type::Nullable { inner, .. } => type_key_name(checked, inner),
        Type::QualifiedGroup { base, .. } => type_key_name(checked, base),
        _ => type_base_name(ty),
    }
}

pub fn type_base_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => type_base_name(inner),
        Type::QualifiedGroup { base, .. } => type_base_name(base),
        Type::Array { .. } => Some("[]"),
        // [col-hashed-ordered] A tuple receiver, for the interim structural
        // `cmp`/`eq`/`hash` intrinsics over tuples.
        Type::Tuple { .. } => Some("()"),
        _ => None,
    }
}

/// The base type name of a *checker* type, aligned with
/// [`type_base_name`]'s conventions so the two are comparable.
pub fn ty_base_name(ty: &Ty) -> Option<&str> {
    match ty {
        Ty::Named { name, .. } => Some(name),
        Ty::Qualified { base, .. } => ty_base_name(base),
        Ty::Array(_) => Some("[]"),
        Ty::Union(_) => {
            // `T?` compares as its value arm (AST `Nullable` does too).
            let arms = ty.value_arms();
            if arms.len() == 1 {
                ty_base_name(arms[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// True when a checker type contains no `Unknown`.
pub fn ty_is_concrete(ty: &Ty) -> bool {
    match ty {
        Ty::Unknown => false,
        Ty::Named { args, .. } => args.iter().all(ty_is_concrete),
        Ty::Qualified { base, .. } => ty_is_concrete(base),
        Ty::Union(arms) => arms.iter().all(ty_is_concrete),
        Ty::Tuple(elems) => elems.iter().all(ty_is_concrete),
        Ty::Array(elem) => ty_is_concrete(elem),
        Ty::Fn { params, ret, .. } => params.iter().all(ty_is_concrete) && ty_is_concrete(ret),
        _ => true,
    }
}

pub fn is_none_type(ty: &Type) -> bool {
    matches!(ty, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == "None")
}

/// Names assigned or incremented anywhere in the block: what decides a
/// mutable binding (`var` on Kotlin, a `mut` binder on Rust).
pub fn collect_mutated(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Assign { target, value, .. } => {
                if let Expr::Ident(id) = target {
                    out.insert(id.name.clone());
                }
                collect_mutated_expr(target, out);
                collect_mutated_expr(value, out);
            }
            Stmt::Let { value, .. } => collect_mutated_expr(value, out),
            Stmt::Use { handler, .. } => collect_mutated_expr(handler, out),
            Stmt::Expr(e) => collect_mutated_expr(e, out),
            _ => {}
        }
    }
}

pub fn collect_mutated_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        // [assert-fn] A condition or a message may mutate, like any expression.
        Expr::Assert { cond, message, .. } => {
            collect_mutated_expr(cond, out);
            if let Some(m) = message {
                collect_mutated_expr(m, out);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                collect_mutated_expr(m, out);
            }
        }
        // [elvis] Both sides may mutate.
        Expr::Elvis { subject, rhs, .. } => {
            collect_mutated_expr(subject, out);
            collect_mutated_expr(rhs, out);
        }
        Expr::SafeField { inner, .. } => collect_mutated_expr(inner, out),
        Expr::Placeholder { .. } => {}
        // [expr-escape] The escapes carry a value expression.
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_mutated_expr(v, out);
            }
        }
        Expr::Continue { .. } => {}
        Expr::IncDec { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
        }
        // [fn-overload-at] Only the dot-notation receiver is an expression.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(base) = base {
                collect_mutated_expr(base, out);
            }
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (cond, block) in branches {
                collect_mutated_expr(cond, out);
                collect_mutated(block, out);
            }
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_mutated_expr(cond, out);
            collect_mutated(body, out);
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_mutated_expr(iterable, out);
            collect_mutated(body, out);
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_mutated_expr(subject, out);
            for b in branches {
                collect_mutated(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (cond, block) in branches {
                collect_mutated_expr(cond, out);
                collect_mutated(block, out);
            }
            collect_mutated(else_block, out);
        }
        Expr::Call { callee, args, .. } => {
            collect_mutated_expr(callee, out);
            for a in args {
                collect_mutated_expr(a, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_mutated_expr(lhs, out);
            collect_mutated_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::Spread { operand, .. } => collect_mutated_expr(operand, out),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => collect_mutated_expr(base, out),
        Expr::Index { base, index, .. } => {
            collect_mutated_expr(base, out);
            collect_mutated_expr(index, out);
        }
        Expr::Is { subject, .. } => collect_mutated_expr(subject, out),
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_mutated_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_mutated_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_mutated_expr(k, out);
                collect_mutated_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_mutated_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_mutated_expr(e, out),
                    // [comptime-inline] Gone before emission.
                    StructLitFieldKind::InlineFor { .. } => {}
                }
            }
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(e) => collect_mutated_expr(e, out),
            LambdaBody::Block(b) => collect_mutated(b, out),
        },
        // [try] The delimiter's body is ordinary code: a variable mutated
        // only inside it still needs the mutable declaration.
        Expr::Try { body, .. } => collect_mutated(body, out),
        // [actor-spawn-expr] [actor-replyto] [actor-waitfor] The clauses,
        // captures and bridge block are ordinary code.
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            collect_mutated_expr(handler, out);
            for handler in with_items {
                collect_mutated_expr(handler, out);
            }
            if let Some(pool) = pool {
                collect_mutated_expr(pool, out);
            }
        }
        Expr::ReplyTo { captures, .. } => {
            for capture in captures {
                collect_mutated_expr(capture, out);
            }
        }
        // [actor-self-send] A leaf.
        Expr::SelfScoped { .. } | Expr::SelfAddr { .. } => {}
        Expr::WaitFor { body, .. } => collect_mutated(body, out),
        // [qual-lift] The check reads its subject.
        Expr::Widen { subject, .. } => collect_mutated_expr(subject, out),
        // Leaves: no sub-expression, so nothing can be mutated inside.
        // Listed rather than defaulted, because a missed form emits an
        // immutable declaration for a variable the code assigns and the
        // target compiler is what reports it [backend-never-wrong].
        Expr::Ident(_)
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}

/// Every name a block binds (generated effect-parameter and `use`
/// variable names must avoid them).
pub fn collect_declared(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { pattern, value, .. } => {
                collect_pattern_names(pattern, out);
                collect_declared_expr(value, out);
            }
            Stmt::Assign { target, value, .. } => {
                collect_declared_expr(target, out);
                collect_declared_expr(value, out);
            }
            Stmt::Use { handler, .. } => collect_declared_expr(handler, out),
            Stmt::Expr(e) => collect_declared_expr(e, out),
            _ => {}
        }
    }
}

pub fn collect_declared_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Assert { cond, message, .. } => {
            collect_declared_expr(cond, out);
            if let Some(m) = message {
                collect_declared_expr(m, out);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                collect_declared_expr(m, out);
            }
        }
        Expr::Elvis { subject, rhs, .. } => {
            collect_declared_expr(subject, out);
            collect_declared_expr(rhs, out);
        }
        Expr::SafeField { inner, .. } => collect_declared_expr(inner, out),
        Expr::Placeholder { .. } => {}
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_declared_expr(v, out);
            }
        }
        Expr::Continue { .. } => {}
        Expr::Is {
            subject, binding, ..
        } => {
            if let Some(b) = binding {
                out.insert(b.name.clone());
            }
            collect_declared_expr(subject, out);
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_declared_expr(c, out);
                collect_declared(b, out);
            }
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_declared_expr(subject, out);
            for b in branches {
                if let Some(binding) = &b.binding {
                    out.insert(binding.name.clone());
                }
                collect_declared(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_declared_expr(c, out);
                collect_declared(b, out);
            }
            collect_declared(else_block, out);
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_declared_expr(cond, out);
            collect_declared(body, out);
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::For {
            pattern,
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_pattern_names(pattern, out);
            collect_declared_expr(iterable, out);
            collect_declared(body, out);
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::Lambda { params, body, .. } => {
            for p in params {
                out.insert(p.name.name.clone());
            }
            match body {
                LambdaBody::Expr(e) => collect_declared_expr(e, out),
                LambdaBody::Block(b) => collect_declared(b, out),
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_declared_expr(callee, out);
            for a in args {
                collect_declared_expr(a, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_declared_expr(lhs, out);
            collect_declared_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::IncDec { operand, .. }
        | Expr::Spread { operand, .. } => collect_declared_expr(operand, out),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            collect_declared_expr(base, out)
        }
        // [fn-overload-at] Only the dot-notation receiver is an expression.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(base) = base {
                collect_declared_expr(base, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_declared_expr(base, out);
            collect_declared_expr(index, out);
        }
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_declared_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_declared_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_declared_expr(k, out);
                collect_declared_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_declared_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_declared_expr(e, out),
                    // [comptime-inline] Gone before emission.
                    StructLitFieldKind::InlineFor { .. } => {}
                }
            }
        }
        // [qual-lift] No binding of its own — the subject reads widened —
        // but the subject expression may declare one.
        Expr::Widen { subject, .. } => collect_declared_expr(subject, out),
        // [try] The delimiter's body is ordinary code and declares its own
        // locals.
        Expr::Try { body, .. } => collect_declared(body, out),
        // [actor-spawn-expr] [actor-replyto] The clauses and captures are
        // expressions, which may declare inside a nested body.
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            collect_declared_expr(handler, out);
            for handler in with_items {
                collect_declared_expr(handler, out);
            }
            if let Some(pool) = pool {
                collect_declared_expr(pool, out);
            }
        }
        Expr::ReplyTo { captures, .. } => {
            for capture in captures {
                collect_declared_expr(capture, out);
            }
        }
        // [actor-self-send] A leaf: it declares nothing.
        Expr::SelfScoped { .. } | Expr::SelfAddr { .. } => {}
        // [actor-waitfor] The token binder is a declaration of its own, and
        // the block declares like any other.
        Expr::WaitFor { binding, body, .. } => {
            out.insert(binding.name.clone());
            collect_declared(body, out);
        }
        // Leaves: nothing declared inside. Listed rather than defaulted so
        // a new binding form cannot escape the name census that keeps
        // generated locals from colliding with user names.
        Expr::Ident(_)
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}

pub fn collect_pattern_names(pattern: &Pattern, out: &mut HashSet<String>) {
    match pattern {
        Pattern::Ident(id) => {
            out.insert(id.name.clone());
        }
        Pattern::Tuple { elems, .. } => {
            for p in elems {
                collect_pattern_names(p, out);
            }
        }
        Pattern::Struct { fields, .. } => {
            for f in fields {
                out.insert(f.binding.name.clone());
            }
        }
    }
}

/// Walks a condition for `is`-checks with bindings.
/// [is-bind-once] Whether an expression is a **place** — a name or a
/// field/index/tuple chain over one — and so free of side effects to read
/// twice. Everything else (a call above all) has to be evaluated once into a
/// temporary before an `is` binding reads it.
/// [actor-mailbox] The `capacity` field's value expression inside a handler's
/// `mailbox { … }` slot — the slot *is* a struct literal, so this is one field
/// lookup rather than a new AST shape.
pub fn mailbox_capacity_expr(mailbox: &Expr) -> Option<&Expr> {
    let Expr::StructLit { fields, .. } = mailbox else {
        return None;
    };
    fields.iter().find_map(|f| match &f.kind {
        salvo_syntax::ast::StructLitFieldKind::Named { name, value } if name.name == "capacity" => {
            Some(value)
        }
        _ => None,
    })
}

pub fn is_place_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(_) => true,
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => is_place_expr(base),
        Expr::Index { base, index, .. } => is_place_expr(base) && is_place_expr(index),
        _ => false,
    }
}

/// [is-and-chain] The `is` bindings [cond] makes that [later] reads: what an
/// `&&` has to bind before its right side.
pub fn bindings_read_later(cond: &Expr, later: &Expr) -> bool {
    let mut names: Vec<String> = Vec::new();
    collect_is_bindings(cond, &mut |_, _, b, _, _| names.push(b.name.clone()));
    if names.is_empty() {
        return false;
    }
    struct Reads<'n> {
        names: &'n [String],
        found: bool,
    }
    impl salvo_syntax::visit::Visitor for Reads<'_> {
        fn visit_expr(&mut self, e: &Expr) {
            if let Expr::Ident(id) = e {
                if self.names.contains(&id.name) {
                    self.found = true;
                }
            }
        }
    }
    let mut r = Reads { names: &names, found: false };
    salvo_syntax::visit::walk_expr(&mut r, later);
    r.found
}

pub fn collect_is_bindings<'a>(
    cond: &'a Expr,
    f: &mut impl FnMut(&'a Expr, &'a [TypeRef], &'a Ident, Span, bool),
) {
    match cond {
        Expr::Is {
            subject,
            check,
            binding: Some(b),
            span,
        } => f(subject, check, b, *span, false),
        // [qual-lift] `is ^Ok inner` binds too, at the *lifted* type. The
        // binding's own recorded type is what the read is built against, so
        // the same emission serves both — which is why a widen shadow is
        // only needed when there is no binding.
        Expr::Widen {
            subject,
            quals,
            binding: Some(b),
            span,
        } => f(subject, quals, b, *span, true),
        Expr::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
            ..
        } => {
            collect_is_bindings(lhs, f);
            collect_is_bindings(rhs, f);
        }
        _ => {}
    }
}

/// [type-literal] Whether an `is` test carries value conditions, or tests a
/// union of literals that collapsed to its base (no wrapper, no `None`).
pub fn literal_test(test: &UnionTest) -> bool {
    !test.match_none && (!test.values.is_empty() || (test.size == 1 && !test.nullable))
}

/// Substitutes generic parameters in an AST type (alias expansion).
pub fn subst_ast_type(ty: &Type, map: &HashMap<&str, &Type>) -> Type {
    match ty {
        Type::Literal { .. } => ty.clone(),
        Type::Named { qualifiers, base } => {
            if qualifiers.is_empty() && base.args.is_empty() {
                if let Some(replacement) = map.get(base.name.name.as_str()) {
                    return (*replacement).clone();
                }
            }
            Type::Named {
                qualifiers: qualifiers.clone(),
                base: TypeRef {
                    value_args: Vec::new(),
                    at: None,
                    binder: false,
                    established: false,
                    alias: None,
                    name: base.name.clone(),
                    args: base.args.iter().map(|a| subst_ast_type(a, map)).collect(),
                    from: base.from.clone(),
                    span: base.span,
                },
            }
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } => Type::QualifiedGroup {
            qualifiers: qualifiers.clone(),
            base: Box::new(subst_ast_type(base, map)),
            span: *span,
        },
        Type::Union { arms, span } => Type::Union {
            arms: arms.iter().map(|a| subst_ast_type(a, map)).collect(),
            span: *span,
        },
        Type::Tuple { elems, span } => Type::Tuple {
            elems: elems.iter().map(|e| subst_ast_type(e, map)).collect(),
            span: *span,
        },
        Type::Array { elem, span } => Type::Array {
            elem: Box::new(subst_ast_type(elem, map)),
            span: *span,
        },
        Type::Nullable { inner, span } => Type::Nullable {
            inner: Box::new(subst_ast_type(inner, map)),
            span: *span,
        },
        Type::Fn {
            params,
            param_names,
            effects,
            deductions,
            ret,
            span,
        } => Type::Fn {
            params: params.iter().map(|p| subst_ast_type(p, map)).collect(),
            param_names: param_names.clone(),
            effects: effects.clone(),
            deductions: deductions.clone(),
            ret: Box::new(subst_ast_type(ret, map)),
            span: *span,
        },
    }
}

/// Writes emitted files under `target_dir`, creating directories, and
/// answers their relative paths: the body of every backend's
/// `Backend::emit` (ROADMAP §0j step 3).
pub fn write_emitted(
    target_dir: &std::path::Path,
    files: Vec<EmittedFile>,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut written = Vec::with_capacity(files.len());
    for file in files {
        let path = target_dir.join(&file.rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &file.content)?;
        written.push(file.rel_path);
    }
    Ok(written)
}

/// [platform-abi] The first lines of every generated file of a platform
/// root, in a backend whose line comment is `//`: never edited, never read
/// by the build.
pub const ABI_HEADER: &str = "// GENERATED by salvo for the host project of this platform root [platform-abi]:\n\
    // the declarations the platform code uses, as the build emits them. Rewritten\n\
    // by every build — do not edit; the build never reads this file.";

/// [platform-abi] [platform-stamp] The program's ABI stamp, and `files`
/// with [`ABI_HEADER`] and the stamp prefixed to each — what a consumer
/// compares before building the root.
pub fn stamp_abi_files(
    program: &salvo_core::Program,
    files: Vec<EmittedFile>,
) -> (String, Vec<(std::path::PathBuf, String)>) {
    let symbols = salvo_core::Symbols::collect(program);
    let stamp = salvo_core::abi::abi_stamp(program, &symbols, None).unwrap_or_default();
    let out = files
        .into_iter()
        .map(|f| (f.rel_path, format!("{ABI_HEADER}\n// {stamp}\n{}", f.content)))
        .collect();
    (stamp, out)
}

/// Removes `path` if it is a file *we* wrote, recognised by its first
/// line(s) being `header`: how a manifest a previous build generated is
/// retired without touching a hand-written one.
pub fn remove_if_ours(path: &std::path::Path, header: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(path).is_ok_and(|text| text.starts_with(header)) {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Skeleton files as `(path, content)` pairs, the shape
/// `Backend::platform_skeletons` answers.
pub fn as_pairs(files: Vec<EmittedFile>) -> Vec<(std::path::PathBuf, String)> {
    files.into_iter().map(|f| (f.rel_path, f.content)).collect()
}

/// [platform-abi] Whether `module` is the project's own (not std, not a
/// dependency): its implementation file is mounted in a host project, where
/// a std or dependency module's host file is carried as a copy.
pub fn is_project_module(program: &salvo_core::Program, module: &salvo_core::ModulePath) -> bool {
    program
        .units()
        .any(|u| u.file.module == *module && (!u.file.is_std || u.file.is_shadow) && u.file.dependency.is_none())
}
