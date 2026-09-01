//! Deduction inference and validation [deduce-syntax] [deduce-infer].
//!
//! A function's deduction list (`-> [list: Mut] T`) states what a call
//! does to each parameter: a listed parameter is given back to the caller
//! (borrowed) with exactly the listed qualifiers still known; a parameter
//! omitted from a *written* list is moved. Deductions are interpreted
//! relative to the qualifiers *declared on the parameter*: a call removes
//! exactly the set `declared − kept` from the argument's known
//! qualifiers, so qualifiers the argument has beyond the declared ones
//! are unaffected.
//!
//! An unwritten list is inferred as the strictest deduction over all uses
//! of each parameter in the body (including moves), iterated to a
//! fixpoint over the call graph: inference starts optimistic (everything
//! kept with its declared qualifiers) and constraints only remove facts,
//! so the iteration terminates. Written lists are validated against the
//! same body facts: promising a parameter back that the body moves, or a
//! qualifier the body may remove, is an error.
//!
//! The Kotlin backend ignores deductions; they are the Rust backend's
//! ownership/borrow contract, computed and stored here so the typed IR
//! carries them.
//!
//! Lenient like the rest of the checker: uses that cannot be resolved
//! (backend interop, effect-member calls, non-`fn` callees) are treated
//! as plain borrows that preserve all qualifiers. Value flow of a bare
//! parameter out of a branch/loop tail is not tracked as a move yet.

use std::collections::HashMap;

use salvo_syntax::ast::{
    Block, Deduction, Expr, FnDecl, Item, LambdaBody, Param, Stmt, StrExprPart,
    StructLitFieldKind, Type,
};
use salvo_syntax::Span;

use crate::check::{Checked, Key};
use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::resolve::FnKey;

/// The deduction facts for one parameter [deduce-syntax].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDeduction {
    pub param: String,
    /// False when a call moves the parameter (omitted from a written
    /// deduction list, or inferred as moved from the body).
    pub kept: bool,
    /// Qualifier names still known after a call, in declared order —
    /// a subset of the parameter's declared outer qualifiers. Only
    /// meaningful when `kept`.
    pub quals: Vec<String>,
}

/// One function participating in inference.
struct FnInfo<'p> {
    key: FnKey,
    decl: &'p FnDecl,
}

/// Computes (or validates) the deduction list of every top-level `fn` and
/// stores the result in `checked.deductions`.
pub(crate) fn infer(program: &Program, checked: &mut Checked) {
    let mut fns: Vec<FnInfo<'_>> = Vec::new();
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for (item_idx, item) in ast.items.iter().enumerate() {
            if let Item::Fn(f) = item {
                fns.push(FnInfo {
                    key: FnKey {
                        file: file_idx,
                        item: item_idx,
                    },
                    decl: f,
                });
            }
        }
    }
    let fn_decls: HashMap<FnKey, &FnDecl> = fns.iter().map(|f| (f.key, f.decl)).collect();

    // Initial state: written lists as declared (validating their shape),
    // unwritten lists optimistic (everything kept with declared quals).
    let mut errors: Vec<FileDiagnostic> = Vec::new();
    let mut states: HashMap<FnKey, Vec<ParamDeduction>> = HashMap::new();
    for f in &fns {
        let state = match &f.decl.deductions {
            Some(list) => from_written(f.decl, list, |span, msg| {
                errors.push(FileDiagnostic::error(f.key.file, span, msg));
            }),
            None => optimistic(f.decl),
        };
        states.insert(f.key, state);
    }

    // Fixpoint over the call graph for the inferred (unwritten) lists.
    // Constraints are monotone (facts only disappear), so this terminates.
    loop {
        let mut changed = false;
        for f in &fns {
            if f.decl.deductions.is_some() {
                continue;
            }
            let Some(body) = &f.decl.body else { continue };
            let new = infer_body(f, body, &checked.call_fn, &fn_decls, &states);
            if states.get(&f.key) != Some(&new) {
                states.insert(f.key, new);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Validate written lists against the body facts [deduce-infer]: the
    // contract may be *stricter* than the body (dropping qualifiers or
    // moving parameters the body gives back), never looser.
    for f in &fns {
        let (Some(list), Some(body)) = (&f.decl.deductions, &f.decl.body) else {
            continue;
        };
        let inferred = infer_body(f, body, &checked.call_fn, &fn_decls, &states);
        let written = &states[&f.key];
        for (w, i) in written.iter().zip(&inferred) {
            let Some(entry) = list.iter().find(|d| d.param.name == w.param) else {
                continue;
            };
            if w.kept && !i.kept {
                errors.push(FileDiagnostic::error(
                    f.key.file,
                    entry.span,
                    format!(
                        "deduction promises `{}` back to the caller, but the \
                         body moves it",
                        w.param
                    ),
                ));
                continue;
            }
            for q in &w.quals {
                if w.kept && !i.quals.contains(q) {
                    errors.push(FileDiagnostic::error(
                        f.key.file,
                        entry.span,
                        format!(
                            "deduction promises qualifier `{q}` on `{}`, but \
                             the body may remove it",
                            w.param
                        ),
                    ));
                }
            }
        }
    }

    checked.errors.extend(errors);
    checked.deductions = states;
}

/// The qualifier names written on the outside of a parameter type, in
/// declaration order (`Mut NonEmpty List<T>` -> `["Mut", "NonEmpty"]`).
fn declared_quals(ty: &Type) -> Vec<String> {
    match ty {
        Type::Named { qualifiers, .. } => {
            qualifiers.iter().map(|q| q.name.name.clone()).collect()
        }
        Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().map(|q| q.name.name.clone()).collect()
        }
        Type::Nullable { inner, .. } => declared_quals(inner),
        _ => Vec::new(),
    }
}

/// Everything kept with its declared qualifiers (the optimistic starting
/// point of inference, and the state of unannotated bodyless fns).
fn optimistic(decl: &FnDecl) -> Vec<ParamDeduction> {
    decl.params
        .iter()
        .map(|p| ParamDeduction {
            param: p.name.name.clone(),
            kept: true,
            quals: declared_quals(&p.ty),
        })
        .collect()
}

/// A written deduction list, validated for shape: entries must name a
/// parameter (once), and may only keep qualifiers declared on that
/// parameter's type [deduce-syntax].
fn from_written(
    decl: &FnDecl,
    list: &[Deduction],
    mut error: impl FnMut(Span, String),
) -> Vec<ParamDeduction> {
    for (i, d) in list.iter().enumerate() {
        if !decl.params.iter().any(|p| p.name.name == d.param.name) {
            error(
                d.span,
                format!("deduction names unknown parameter `{}`", d.param.name),
            );
        }
        if list[..i].iter().any(|prev| prev.param.name == d.param.name) {
            error(
                d.span,
                format!("duplicate deduction for parameter `{}`", d.param.name),
            );
        }
    }
    decl.params
        .iter()
        .map(|p| {
            let declared = declared_quals(&p.ty);
            match list.iter().find(|d| d.param.name == p.name.name) {
                Some(d) => {
                    let mut quals = Vec::new();
                    for q in &d.qualifiers {
                        if !declared.contains(&q.name.name) {
                            error(
                                q.span,
                                format!(
                                    "deduction keeps qualifier `{}`, which is not \
                                     declared on parameter `{}`",
                                    q.name.name, p.name.name
                                ),
                            );
                        } else {
                            quals.push(q.name.name.clone());
                        }
                    }
                    ParamDeduction {
                        param: p.name.name.clone(),
                        kept: true,
                        quals,
                    }
                }
                None => ParamDeduction {
                    param: p.name.name.clone(),
                    kept: false,
                    quals: Vec::new(),
                },
            }
        })
        .collect()
}

/// Re-derives one fn's deduction facts from its body, given the current
/// state of every other fn [deduce-infer].
fn infer_body(
    f: &FnInfo<'_>,
    body: &Block,
    call_fn: &HashMap<Key, FnKey>,
    fn_decls: &HashMap<FnKey, &FnDecl>,
    states: &HashMap<FnKey, Vec<ParamDeduction>>,
) -> Vec<ParamDeduction> {
    let mut walk = Walk {
        file: f.key.file,
        call_fn,
        fn_decls,
        states,
        params: optimistic(f.decl),
    };
    walk.block(body);
    walk.params
}

/// The body walker applying use-constraints to the parameter facts.
struct Walk<'a, 'p> {
    file: usize,
    call_fn: &'a HashMap<Key, FnKey>,
    fn_decls: &'a HashMap<FnKey, &'p FnDecl>,
    states: &'a HashMap<FnKey, Vec<ParamDeduction>>,
    params: Vec<ParamDeduction>,
}

/// The parameter an argument passes *itself* (spreads forward the value
/// whole). Shadowing is rejected by the checker, so a name match is the
/// parameter.
fn bare_ident(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident(id) => Some(&id.name),
        Expr::Spread { operand, .. } => bare_ident(operand),
        _ => None,
    }
}

/// The callee parameter an argument position binds to (trailing arguments
/// bind the variadic parameter) [fn-variadic].
fn param_for_arg(params: &[Param], i: usize) -> Option<usize> {
    if i < params.len() {
        return Some(i);
    }
    match params.last() {
        Some(p) if p.variadic => Some(params.len() - 1),
        _ => None,
    }
}

impl Walk<'_, '_> {
    fn mark_moved(&mut self, name: &str) {
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            p.kept = false;
            p.quals.clear();
        }
    }

    /// Removes `removed` qualifier names from a parameter's kept set.
    fn remove_quals(&mut self, name: &str, removed: &[String]) {
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            p.quals.retain(|q| !removed.contains(q));
        }
    }

    fn block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // Binding a bare parameter to something else transfers
            // ownership out of the parameter.
            Stmt::Let { value, .. } => self.moving_expr(value),
            Stmt::Assign { target, value, .. } => {
                self.expr(target);
                self.moving_expr(value);
            }
            Stmt::Return { value: Some(v), .. }
            | Stmt::Break { value: Some(v), .. }
            | Stmt::Yield { value: v, .. } => self.moving_expr(v),
            Stmt::Use { handler, .. } => {
                // Handler constructor arguments are stored in the handler.
                if let Expr::Call { args, .. } = handler {
                    for a in args {
                        self.moving_expr(a);
                    }
                } else {
                    self.expr(handler);
                }
            }
            Stmt::Expr(e) => self.expr(e),
            _ => {}
        }
    }

    /// An expression whose value flows somewhere the caller keeps:
    /// passing a bare parameter here moves it.
    fn moving_expr(&mut self, e: &Expr) {
        match bare_ident(e) {
            Some(name) => {
                let name = name.to_string();
                self.mark_moved(&name);
            }
            None => self.expr(e),
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Call {
                callee, args, span, ..
            } => self.call(callee, args, *span),
            // Struct/array/tuple construction stores the value.
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    match &f.kind {
                        StructLitFieldKind::Named { value, .. } => self.moving_expr(value),
                        StructLitFieldKind::Spread(v) => self.moving_expr(v),
                    }
                }
            }
            Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
                for el in elems {
                    self.moving_expr(el);
                }
            }
            Expr::ArrayInit { size, init, .. } => {
                self.expr(size);
                self.expr(init);
            }
            Expr::Str { parts, .. } => {
                for p in parts {
                    if let StrExprPart::Interp(i) = p {
                        self.expr(i);
                    }
                }
            }
            Expr::Field { base, .. } => self.expr(base),
            Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            Expr::Unary { operand, .. }
            | Expr::NonNull { operand, .. }
            | Expr::PostIncrement { operand, .. }
            | Expr::Spread { operand, .. } => self.expr(operand),
            Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Is { subject, .. } => self.expr(subject),
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    self.expr(c);
                    self.block(b);
                }
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::When {
                subject, branches, ..
            } => {
                self.expr(subject);
                for b in branches {
                    self.block(&b.body);
                }
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                self.expr(cond);
                self.block(body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                self.expr(iterable);
                self.block(body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            // Uses inside a lambda body count like any other use (the
            // lambda may run any number of times).
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(e) => self.expr(e),
                LambdaBody::Block(b) => self.block(b),
            },
            _ => {}
        }
    }

    /// A call site: bare-parameter arguments take the callee's deduction —
    /// moved when the callee moves them, otherwise stripped of exactly the
    /// callee's removal set (declared − kept) [deduce-syntax]. Unresolved
    /// callees (interop, effect members) borrow and preserve everything.
    fn call(&mut self, callee: &Expr, args: &[Expr], span: Span) {
        // Dot notation: the receiver is argument 0 [fn-dot].
        let mut arg_exprs: Vec<&Expr> = Vec::new();
        match callee {
            Expr::Field { base, .. } => arg_exprs.push(base),
            Expr::Ident(_) => {}
            other => self.expr(other),
        }
        arg_exprs.extend(args.iter());

        let resolved = self
            .call_fn
            .get(&(self.file, span))
            .and_then(|key| Some((self.fn_decls.get(key)?, self.states.get(key)?)));
        let Some((callee_decl, callee_state)) = resolved else {
            for a in arg_exprs {
                self.expr(a);
            }
            return;
        };
        for (i, a) in arg_exprs.into_iter().enumerate() {
            let Some(name) = bare_ident(a) else {
                self.expr(a);
                continue;
            };
            let name = name.to_string();
            let Some(pidx) = param_for_arg(&callee_decl.params, i) else {
                continue;
            };
            let ded = &callee_state[pidx];
            if !ded.kept {
                self.mark_moved(&name);
                continue;
            }
            let declared = declared_quals(&callee_decl.params[pidx].ty);
            let removed: Vec<String> = declared
                .into_iter()
                .filter(|q| !ded.quals.contains(q))
                .collect();
            if !removed.is_empty() {
                self.remove_quals(&name, &removed);
            }
        }
    }
}
