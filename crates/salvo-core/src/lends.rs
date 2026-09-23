//! [proj-infer] Which parameters a function's result *holds a borrow of*.
//!
//! A struct with `proj` fields is an owned object that projects other
//! values: `Mut ListYield<T> { items: list, at: 0 }` is a pass of its own,
//! whose `items` borrows `list`. A function returning such a struct hands
//! the caller a value tied to some of its arguments, and the caller has to
//! know which — that is what refuses `move(xs)` while `let p = iter(xs)`
//! is alive.
//!
//! The rule the user set (2026-09-11): **infer what can be inferred, let
//! the user write the rest.** So:
//!
//! * A fn with a body: the lent set is read off the body — every value the
//!   fn returns, followed through struct literals (a `proj` field takes
//!   the roots of what is stored in it; an owned field holding a view
//!   takes that view's lends), through calls to other lending fns, through
//!   local bindings, `!`, and branches. Per field, exact.
//! * A fn without a body (an intrinsic, an effect member) or a fn *type*:
//!   conservatively every kept parameter, which is exact for the
//!   one-argument case (`iter(list)`) and only ever over-links.
//! * `[p: proj]` written in the deduction list declares the set outright.
//!   With a body it must equal the inferred set (over-declaring is as much
//!   an error as under-declaring, so an annotation cannot go stale); it is
//!   the only way to say anything for a bodiless declaration.
//!
//! The analysis is syntactic and conservative: anything it cannot follow
//! (a value produced by an unknown shape) falls back to "all kept
//! parameters" for that fn. It never under-approximates.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{
    Block, Deduction, DeductionKind, DeductionTarget, Expr, FieldDecl, FnDecl, Pattern, Stmt,
    StructDecl, StructLitFieldKind, Type, TypeRef,
};

/// Everything the inference needs to see of the program.
pub struct LendsEnv<'a, 'p> {
    pub structs: &'a HashMap<&'p str, &'p StructDecl>,
    /// All visible fn declarations with a given name (overloads together).
    pub fns: &'a dyn Fn(&str) -> Vec<&'p FnDecl>,
    /// Results per declaration, keyed by the declaration's address. An
    /// entry inserted before a body is analysed is the cycle guard (the
    /// conservative set), replaced by the exact set afterwards.
    pub memo: &'a mut HashMap<usize, Vec<usize>>,
}

/// Whether a `proj` qualifier appears anywhere in `ty`.
pub fn type_has_proj(ty: &Type) -> bool {
    fn in_ref(r: &TypeRef) -> bool {
        r.name.name == "proj" || r.args.iter().any(type_has_proj)
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers.iter().any(in_ref) || in_ref(base),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(in_ref) || type_has_proj(base),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().any(type_has_proj)
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => type_has_proj(elem),
        Type::Fn { .. } => false,
    }
}

/// Whether a `proj` appears inside a type *argument* of `ty` (not on the
/// value itself — that is a wholesale projection).
pub fn type_arg_has_proj(ty: &Type) -> bool {
    fn in_ref(r: &TypeRef) -> bool {
        r.args.iter().any(type_has_proj)
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers.iter().any(in_ref) || in_ref(base),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(in_ref) || type_arg_has_proj(base),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().any(type_arg_has_proj)
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => type_arg_has_proj(elem),
        Type::Fn { .. } => false,
    }
}

/// The struct names a type is built from (union arms, nullable inner,
/// tuple elements, arrays — every position a value of a struct type could
/// sit in).
fn struct_names(ty: &Type, out: &mut Vec<String>) {
    match ty {
        Type::Named { base, .. } => out.push(base.name.name.clone()),
        Type::QualifiedGroup { base, .. } => struct_names(base, out),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().for_each(|a| struct_names(a, out))
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => struct_names(elem, out),
        Type::Fn { .. } => {}
    }
}

/// Whether a value of this type can hold a borrow: it is (or contains) a
/// struct with a `proj` field, directly or through an owned field whose
/// type does.
pub fn holds_proj(ty: &Type, structs: &HashMap<&str, &StructDecl>) -> bool {
    fn go(ty: &Type, structs: &HashMap<&str, &StructDecl>, seen: &mut HashSet<String>) -> bool {
        // A `proj` in a type argument (`List<proj T>`) is a borrow the
        // container holds.
        if type_arg_has_proj(ty) {
            return true;
        }
        let mut names = Vec::new();
        struct_names(ty, &mut names);
        names.into_iter().any(|n| {
            if !seen.insert(n.clone()) {
                return false;
            }
            structs.get(n.as_str()).is_some_and(|d| {
                d.fields
                    .iter()
                    .any(|f| type_has_proj(&f.ty) || go(&f.ty, structs, seen))
            })
        })
    }
    go(ty, structs, &mut HashSet::new())
}

/// The struct's fields that hold borrows: `(name, is_proj_field)`, where a
/// non-`proj` field is listed when its own type holds a borrow.
fn borrowing_fields<'p>(
    decl: &'p StructDecl,
    structs: &HashMap<&str, &StructDecl>,
) -> Vec<(&'p FieldDecl, bool)> {
    decl.fields
        .iter()
        .filter_map(|f| {
            if type_has_proj(&f.ty) {
                Some((f, true))
            } else if holds_proj(&f.ty, structs) {
                Some((f, false))
            } else {
                None
            }
        })
        .collect()
}

/// The parameters a fn keeps (indices into `decl.params`, implicit
/// parameters excluded): a written entry that is not `Never`, or —
/// without a written list — every parameter, conservatively.
pub fn kept_params(decl: &FnDecl) -> Vec<usize> {
    decl.params
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.implicit)
        .filter(|(_, p)| match &decl.deductions {
            None => true,
            // [deduce-syntax] Unmentioned is kept (inferred, or the
            // bodiless default); only a written move consumes.
            Some(list) => !list.iter().any(|d| {
                d.param_name().is_some_and(|n| n.name == p.name.name)
                    && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
            }),
        })
        .map(|(i, _)| i)
        .collect()
}

/// The parameters named as sources by written projection entries about the
/// *result* — `=> proj(a)`, `=> .items: proj(a)` — if any were
/// written. (An entry about a *parameter's* field, `v.items: proj(x)`,
/// re-points that parameter and is not a lend of the result.)
pub fn declared_lends(decl: &FnDecl) -> Option<Vec<usize>> {
    let list = decl.deductions.as_ref()?;
    let result_entries: Vec<&Deduction> = list
        .iter()
        .filter(|d| {
            d.proj_sources().is_some()
                && matches!(
                    d.target,
                    DeductionTarget::Opaque | DeductionTarget::Result { .. }
                )
        })
        .collect();
    if result_entries.is_empty() {
        return None;
    }
    let mut out: Vec<usize> = result_entries
        .iter()
        .filter_map(|d| d.proj_sources())
        .flatten()
        .filter_map(|src| decl.params.iter().position(|p| p.name.name == src.name))
        .collect();
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// The lent parameters of `decl`: declared, or inferred from the body, or
/// conservative for a bodiless declaration. Empty when the result cannot
/// hold a borrow at all.
pub fn lends_of(decl: &FnDecl, env: &mut LendsEnv<'_, '_>) -> Vec<usize> {
    let key = decl as *const FnDecl as usize;
    if let Some(v) = env.memo.get(&key) {
        return v.clone();
    }
    // [proj-infer] A written projection entry decides outright — *before*
    // the written-return gate below, because instantiation can make a
    // result hold borrows the written type does not show (`-> Mut List<T>`
    // with `T = proj Str`): the author's `=> proj(it)` names the
    // lends exactly, and must win over the every-kept-argument fallback
    // the caller would otherwise apply.
    if let Some(declared) = declared_lends(decl) {
        env.memo.insert(key, declared.clone());
        return declared;
    }
    let Some(ret) = &decl.return_type else {
        env.memo.insert(key, Vec::new());
        return Vec::new();
    };
    if !holds_proj(ret, env.structs) {
        env.memo.insert(key, Vec::new());
        return Vec::new();
    }
    let conservative = kept_params(decl);
    // Cycle guard: a recursive fn sees itself as conservative.
    env.memo.insert(key, conservative.clone());
    let Some(body) = &decl.body else {
        return conservative;
    };
    let result = match infer_from_body(decl, body, env) {
        Some(set) => {
            let mut v: Vec<usize> = set.into_iter().collect();
            v.sort_unstable();
            v
        }
        None => conservative,
    };
    env.memo.insert(key, result.clone());
    result
}

/// The inferred set for a fn with a body, or `None` when some returned
/// value could not be followed.
pub fn infer_from_body(
    decl: &FnDecl,
    body: &Block,
    env: &mut LendsEnv<'_, '_>,
) -> Option<HashSet<usize>> {
    let mut defs: HashMap<String, Vec<&Expr>> = HashMap::new();
    collect_defs(body, &mut defs);
    let mut returned: Vec<&Expr> = Vec::new();
    collect_returns(body, &mut returned, true);
    let mut w = Walk {
        decl,
        defs: &defs,
        env,
        chasing: HashSet::new(),
    };
    let mut out = HashSet::new();
    for e in returned {
        out.extend(w.lends_of_expr(e)?);
    }
    Some(out)
}

struct Walk<'w, 'a, 'p> {
    decl: &'w FnDecl,
    defs: &'w HashMap<String, Vec<&'w Expr>>,
    env: &'w mut LendsEnv<'a, 'p>,
    /// Locals being chased right now (a local defined in terms of itself,
    /// `x = f(x)`, would otherwise loop).
    chasing: HashSet<String>,
}

impl<'p> Walk<'_, '_, 'p> {
    fn param_index(&self, name: &str) -> Option<usize> {
        self.decl.params.iter().position(|p| p.name.name == name)
    }

    /// The parameters a *value* holds borrows of.
    fn lends_of_expr(&mut self, e: &Expr) -> Option<HashSet<usize>> {
        match e {
            // [assert-fn] An assertion lends nothing: it answers `None` (or
            // `Never`), and its parts are only read.
            Expr::Assert { .. } | Expr::Unreachable { .. } => None,
            // [elvis] Neither side lends: the picked value is the optional's
            // own payload, handed out by value.
            Expr::Elvis { .. } | Expr::Placeholder { .. } | Expr::SafeField { .. } => {
                Some(HashSet::new())
            }
            // A struct literal: `proj` fields take the roots of what they
            // store; owned fields that are themselves views take their lends.
            Expr::StructLit { ty, fields, .. } => {
                let mut names = Vec::new();
                if let Some(t) = ty {
                    struct_names(t, &mut names);
                }
                let Some(sdecl) = names
                    .first()
                    .and_then(|n| self.env.structs.get(n.as_str()).copied())
                else {
                    // A bare `{...}` literal: only an expected type would
                    // name the struct. Conservative.
                    return None;
                };
                let borrowing = borrowing_fields(sdecl, self.env.structs);
                let mut out = HashSet::new();
                for f in fields {
                    match &f.kind {
                        StructLitFieldKind::Named { name, value } => {
                            match borrowing.iter().find(|(fd, _)| fd.name.name == name.name) {
                                Some((_, true)) => out.extend(self.roots_of_expr(value)?),
                                Some((_, false)) => out.extend(self.lends_of_expr(value)?),
                                None => {}
                            }
                        }
                        StructLitFieldKind::Spread(inner) => out.extend(self.lends_of_expr(inner)?),
                    }
                }
                Some(out)
            }
            Expr::Call { .. } => {
                let (all_args, decls) = self.resolve_call(e)?;
                // Overloads are not resolved here: the union over them is
                // conservative and exact when they agree (the common case).
                let mut lent_positions: HashSet<usize> = HashSet::new();
                for d in decls {
                    for i in lends_of(d, self.env) {
                        lent_positions.insert(i);
                    }
                }
                let mut out = HashSet::new();
                for i in lent_positions {
                    if let Some(arg) = all_args.get(i) {
                        // The argument in a lent position is borrowed into
                        // the result: its roots (a place) or, when it is a
                        // view itself, what it holds.
                        out.extend(self.roots_of_expr(arg)?);
                    }
                }
                Some(out)
            }
            // A named value: a parameter holds what the caller gave it (a
            // view parameter returned outright ties the result to that
            // argument); a local holds what it was assigned.
            Expr::Ident(id) => {
                if let Some(i) = self.param_index(&id.name) {
                    return Some(HashSet::from([i]));
                }
                if id.name == "None" {
                    return Some(HashSet::new());
                }
                self.chase_local(&id.name, |w, init| w.lends_of_expr(init))
            }
            // A field of a view holds what the view holds, at most.
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => self.roots_of_expr(base),
            Expr::Index { base, .. } => self.roots_of_expr(base),
            Expr::NonNull { operand, .. } | Expr::Spread { operand, .. } => {
                self.lends_of_expr(operand)
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                let mut out = HashSet::new();
                for (_, b) in branches {
                    out.extend(self.lends_of_block(b)?);
                }
                if let Some(b) = else_block {
                    out.extend(self.lends_of_block(b)?);
                }
                Some(out)
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                let mut out = HashSet::new();
                for (_, b) in branches {
                    out.extend(self.lends_of_block(b)?);
                }
                out.extend(self.lends_of_block(else_block)?);
                Some(out)
            }
            Expr::When { branches, .. } => {
                let mut out = HashSet::new();
                for b in branches {
                    out.extend(self.lends_of_block(&b.body)?);
                }
                Some(out)
            }
            Expr::Try { body, .. } => self.lends_of_block(body),
            // [expr-escape] An escape never yields a value, so it lends
            // nothing — whatever its operand is, no borrow leaves through it.
            Expr::Return { .. } | Expr::Break { .. } | Expr::Continue { .. } => {
                Some(HashSet::new())
            }
            Expr::Widen { subject, .. } => self.lends_of_expr(subject),
            // [actor-spawn-expr] [actor-replyto] [actor-waitfor] None of the
            // three yields a view: an `Addr` and a `Reply` are owned tokens,
            // and a `waitfor` yields a value another actor sent — nothing
            // crossing an actor boundary can be a borrow of a local.
            Expr::Spawn { .. }
            | Expr::ReplyTo { .. }
            | Expr::WaitFor { .. }
            | Expr::SelfScoped { .. } => Some(HashSet::new()),
            // Owned leaves and computations hold nothing.
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Str { .. }
            | Expr::Is { .. }
            | Expr::Binary { .. }
            | Expr::Unary { .. }
            | Expr::IncDec { .. }
            | Expr::Lambda { .. }
            | Expr::While { .. }
            | Expr::For { .. } => Some(HashSet::new()),
            Expr::MapLit { entries, .. } => {
                let mut out = HashSet::new();
                for (k, v) in entries {
                    out.extend(self.lends_of_expr(k)?);
                    out.extend(self.lends_of_expr(v)?);
                }
                Some(out)
            }
            Expr::ArrayLit { elems, .. }
            | Expr::SetLit { elems, .. }
            | Expr::Tuple { elems, .. } => {
                let mut out = HashSet::new();
                for e in elems {
                    out.extend(self.lends_of_expr(e)?);
                }
                Some(out)
            }
            Expr::Scoped { .. } | Expr::EffectScoped { .. } | Expr::Error { .. } => None,
        }
    }

    /// The positional arguments (receiver first) and the visible
    /// declarations of a call by name. `None` for a call through a local
    /// of fn type (a lambda, a fn-typed parameter): no body to read, and
    /// its type's `[p: proj]` entries are the caller's business.
    fn resolve_call<'e>(&self, e: &'e Expr) -> Option<(Vec<&'e Expr>, Vec<&'p FnDecl>)> {
        let Expr::Call { callee, args, .. } = e else {
            return None;
        };
        let (name, all_args): (String, Vec<&Expr>) = match callee.as_ref() {
            Expr::Ident(id) => (id.name.clone(), args.iter().collect()),
            Expr::Field { base, field, .. } => {
                let mut v: Vec<&Expr> = vec![base];
                v.extend(args.iter());
                (field.name.clone(), v)
            }
            Expr::Scoped { name, base, .. } | Expr::EffectScoped { name, base, .. } => {
                let mut v: Vec<&Expr> = Vec::new();
                if let Some(b) = base {
                    v.push(b);
                }
                v.extend(args.iter());
                (name.name.clone(), v)
            }
            _ => return None,
        };
        if self.param_index(&name).is_some() || self.defs.contains_key(&name) {
            return None;
        }
        let decls = (self.env.fns)(&name);
        if decls.is_empty() {
            return None;
        }
        Some((all_args, decls))
    }

    /// The parameters a *place* is rooted in — what a `proj` field stores.
    /// A call in place position is either a wholesale projection
    /// (`get(ts, i)`: rooted where its source argument is) or a forwarded
    /// view (what it holds).
    fn roots_of_expr(&mut self, e: &Expr) -> Option<HashSet<usize>> {
        match e {
            Expr::Ident(id) => {
                if let Some(i) = self.param_index(&id.name) {
                    return Some(HashSet::from([i]));
                }
                if id.name == "None" {
                    return Some(HashSet::new());
                }
                self.chase_local(&id.name, |w, init| w.roots_of_expr(init))
            }
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } | Expr::Index { base, .. } => {
                self.roots_of_expr(base)
            }
            Expr::NonNull { operand, .. } | Expr::Spread { operand, .. } => {
                self.roots_of_expr(operand)
            }
            Expr::Call { .. } => {
                let (all_args, decls) = self.resolve_call(e)?;
                let mut out = HashSet::new();
                for d in decls {
                    if let Some(from) = &d.derived_return {
                        if let Some(i) = d.params.iter().position(|p| p.name.name == from.name) {
                            if let Some(arg) = all_args.get(i) {
                                out.extend(self.roots_of_expr(arg)?);
                            }
                        }
                    }
                }
                out.extend(self.lends_of_expr(e)?);
                Some(out)
            }
            Expr::StructLit { .. } => self.lends_of_expr(e),
            // A literal or computed temporary has no root; storing it in a
            // `proj` field is refused elsewhere, so nothing is lent here.
            _ => Some(HashSet::new()),
        }
    }

    fn lends_of_block(&mut self, b: &Block) -> Option<HashSet<usize>> {
        let mut tails: Vec<&Expr> = Vec::new();
        collect_returns(b, &mut tails, true);
        let mut out = HashSet::new();
        for e in tails {
            out.extend(self.lends_of_expr(e)?);
        }
        Some(out)
    }

    /// Follows a local through every assignment to it. A local the body
    /// never defines (a `for` element, a destructured name, a capture) is
    /// unknown — conservative.
    fn chase_local(
        &mut self,
        name: &str,
        f: impl Fn(&mut Self, &Expr) -> Option<HashSet<usize>>,
    ) -> Option<HashSet<usize>> {
        if !self.chasing.insert(name.to_string()) {
            return Some(HashSet::new());
        }
        let inits = self.defs.get(name)?.clone();
        let mut out = HashSet::new();
        for init in inits {
            match f(self, init) {
                Some(s) => out.extend(s),
                None => {
                    self.chasing.remove(name);
                    return None;
                }
            }
        }
        self.chasing.remove(name);
        Some(out)
    }
}

/// Every `let x = e` and `x = e` in the body, by name (nested blocks
/// included; lambdas excluded — their bodies are a different frame).
fn collect_defs<'e>(block: &'e Block, out: &mut HashMap<String, Vec<&'e Expr>>) {
    fn expr<'e>(e: &'e Expr, out: &mut HashMap<String, Vec<&'e Expr>>) {
        match e {
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    expr(c, out);
                    collect_defs(b, out);
                }
                if let Some(b) = else_block {
                    collect_defs(b, out);
                }
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    expr(c, out);
                    collect_defs(b, out);
                }
                collect_defs(else_block, out);
            }
            Expr::When {
                subject, branches, ..
            } => {
                expr(subject, out);
                for b in branches {
                    collect_defs(&b.body, out);
                }
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                expr(cond, out);
                collect_defs(body, out);
                if let Some(b) = else_block {
                    collect_defs(b, out);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                expr(iterable, out);
                collect_defs(body, out);
                if let Some(b) = else_block {
                    collect_defs(b, out);
                }
            }
            Expr::Try { body, .. } => collect_defs(body, out),
            // [expr-escape] The escapes carry a value expression.
            Expr::Return { value: Some(v), .. } | Expr::Break { value: Some(v), .. } => {
                expr(v, out);
            }
            _ => {}
        }
    }
    for s in &block.stmts {
        match s {
            Stmt::Let { pattern, value, .. } => {
                if let Pattern::Ident(id) = pattern {
                    out.entry(id.name.clone()).or_default().push(value);
                }
                expr(value, out);
            }
            Stmt::Assign { target, value, .. } => {
                if let Expr::Ident(id) = target {
                    out.entry(id.name.clone()).or_default().push(value);
                }
                expr(value, out);
            }
            Stmt::Expr(e) => expr(e, out),
            _ => {}
        }
    }
}

/// Every value the block can produce for its fn: `return e` anywhere
/// inside (not inside lambdas), plus — when `tail` — the block's own
/// trailing expression.
fn collect_returns<'e>(block: &'e Block, out: &mut Vec<&'e Expr>, tail: bool) {
    fn expr<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
        match e {
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    expr(c, out);
                    collect_returns(b, out, false);
                }
                if let Some(b) = else_block {
                    collect_returns(b, out, false);
                }
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    expr(c, out);
                    collect_returns(b, out, false);
                }
                collect_returns(else_block, out, false);
            }
            Expr::When {
                subject, branches, ..
            } => {
                expr(subject, out);
                for b in branches {
                    collect_returns(&b.body, out, false);
                }
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                expr(cond, out);
                collect_returns(body, out, false);
                if let Some(b) = else_block {
                    collect_returns(b, out, false);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                expr(iterable, out);
                collect_returns(body, out, false);
                if let Some(b) = else_block {
                    collect_returns(b, out, false);
                }
            }
            Expr::Try { body, .. } => collect_returns(body, out, false),
            // [expr-escape] The escapes carry a value expression.
            Expr::Return { value: Some(v), .. } | Expr::Break { value: Some(v), .. } => {
                expr(v, out);
            }
            // A lambda's returns are its own.
            Expr::Lambda { .. } => {}
            _ => {}
        }
    }
    let n = block.stmts.len();
    for (i, s) in block.stmts.iter().enumerate() {
        match s {
            // [expr-escape] A `return v` reaches here as an expression
            // statement now; its value is still one of the block's outgoing
            // values, so this arm comes *before* the general expression case.
            Stmt::Expr(Expr::Return { value: Some(v), .. }) => {
                out.push(v);
                expr(v, out);
            }
            Stmt::Expr(Expr::Break { value: Some(v), .. }) => expr(v, out),
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } => expr(value, out),
            Stmt::Expr(e) => {
                if tail && i + 1 == n {
                    // The trailing expression is the block's value when it
                    // is one (`if`/`when` as the last statement yield through
                    // their branches; a bare call yields itself).
                    match e {
                        Expr::If { .. } | Expr::When { .. } | Expr::WhenCond { .. } => {
                            collect_tail_branches(e, out)
                        }
                        Expr::While { .. } | Expr::For { .. } => expr(e, out),
                        other => out.push(other),
                    }
                } else {
                    expr(e, out);
                }
            }
            _ => {}
        }
    }
}

/// The trailing values of a branching expression used as a block tail.
fn collect_tail_branches<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match e {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                let _ = c;
                collect_returns(b, out, true);
            }
            if let Some(b) = else_block {
                collect_returns(b, out, true);
            }
        }
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (_, b) in branches {
                collect_returns(b, out, true);
            }
            collect_returns(else_block, out, true);
        }
        Expr::When { branches, .. } => {
            for b in branches {
                collect_returns(&b.body, out, true);
            }
        }
        _ => out.push(e),
    }
}
