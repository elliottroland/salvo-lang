//! Fn bodies: statements, places, reads and the narrowing bindings
//! (IR.md §3, §5). Expressions are in `exprs.rs`.

use std::collections::HashMap;

use salvo_core::types::Ty;
use salvo_core::FnKey;
use salvo_syntax::ast::{self, Expr as AExpr, Pattern, Stmt as AStmt};
use salvo_syntax::Span;

use super::{erase, unsupported, Ctx};
use crate::ir::*;

mod exprs;

/// A name in scope: the local it reads as, and that local's type.
#[derive(Clone)]
pub(crate) struct Bound {
    pub local: Local,
    pub ty: Ty,
    /// The local owns its value (a `let`, a moved parameter): a read in a
    /// consuming position moves it. A lent parameter's read never does.
    pub owned: bool,
}

pub struct Lower<'a, 'p> {
    pub(crate) ctx: &'a Ctx<'p>,
    pub(crate) file_idx: usize,
    #[allow(dead_code)]
    pub(crate) fn_key: Option<FnKey>,
    pub(crate) scopes: Vec<HashMap<String, Bound>>,
    /// Inside a handler member: the handler's constructor parameters and
    /// state fields, in scope as locals.
    pub self_fields: Vec<(String, Ty)>,
    /// The effect instances in scope, innermost last: effect parameters,
    /// `use` bindings, a handler's dependencies.
    pub effect_env: Vec<(Ty, Local)>,
    /// [implicit-param] the enclosing fn's implicits by resolution name.
    pub implicit_locals: HashMap<String, Local>,
    pub in_handler: Option<String>,
    next_id: u32,
    fresh: u32,
    errors: Vec<String>,
    /// Statements to put in front of the one being lowered (narrowing
    /// bindings justified by an earlier branch).
    pending: Vec<Stmt>,
    /// The last branch or switch at the current block level, which justifies
    /// a narrowing after it.
    last_branch: Option<NodeId>,
    /// The test node whose arm is being lowered, for `Cond` justifications.
    current_test: Option<Justification>,
    pub(crate) loops: usize,
    /// Per enclosing loop: the result local a `break value` assigns, when the
    /// loop's value is used.
    pub(crate) loop_results: Vec<Option<Local>>,
    /// Narrowing bindings a test introduced, for the arm it guards.
    pub(crate) pending_after_test: Vec<Stmt>,
    /// [placeholder] the subject local `_` reads inside a `?:` right side.
    pub(crate) placeholder: Option<Local>,
    /// The storage local (and its declared type) behind each narrowing
    /// alias, for assignments through a narrowed name.
    pub(crate) storage: HashMap<Local, (Local, Ty)>,
    /// Inside a condition, where an `is` test narrows what follows.
    pub(crate) in_condition: bool,
    /// [ir-narrow] Projection places an enclosing safe call bound.
    pub(crate) place_aliases: Vec<(Place, Local, Ty)>,
    /// [ir-narrow] Projection places a test narrowed (a lifted claim,
    /// `h.f is ^Ok`): the binding, valid until the scope at that depth closes.
    pub(crate) scoped_aliases: Vec<(usize, Place, Local, Ty)>,
}

impl<'a, 'p> Lower<'a, 'p> {
    pub fn new(ctx: &'a Ctx<'p>, file_idx: usize, fn_key: Option<FnKey>) -> Self {
        Lower {
            ctx,
            file_idx,
            fn_key,
            scopes: vec![HashMap::new()],
            self_fields: Vec::new(),
            effect_env: Vec::new(),
            implicit_locals: HashMap::new(),
            in_handler: None,
            next_id: 0,
            fresh: 0,
            errors: Vec::new(),
            pending: Vec::new(),
            last_branch: None,
            current_test: None,
            loops: 0,
            loop_results: Vec::new(),
            pending_after_test: Vec::new(),
            placeholder: None,
            storage: HashMap::new(),
            in_condition: false,
            place_aliases: Vec::new(),
            scoped_aliases: Vec::new(),
        }
    }

    pub fn finish(self) -> Vec<String> {
        self.errors
    }

    pub(crate) fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(format!("{}: {}", span.start, msg.into()));
    }

    pub(crate) fn id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id)
    }

    pub(crate) fn fresh(&mut self, base: &str) -> Local {
        self.fresh += 1;
        Local(format!("{base}~{}", self.fresh))
    }

    pub fn bind(&mut self, name: &str, ty: Ty) -> Local {
        self.bind_owned(name, ty, true)
    }

    pub fn bind_owned(&mut self, name: &str, ty: Ty, owned: bool) -> Local {
        // `_` binds nothing a program can read: a fresh local of its own.
        if name == "_" {
            return self.fresh("__unused");
        }
        let local = Local(name.to_string());
        self.scopes.last_mut().unwrap().insert(name.to_string(), Bound { local: local.clone(), ty, owned });
        local
    }

    /// Rebinds `name` to a narrowing alias of what it was bound to: the
    /// storage behind the alias is remembered, so an assignment through the
    /// name writes the storage [ir-narrow].
    pub(crate) fn rebind(&mut self, name: &str, local: Local, ty: Ty) {
        let prev = self.lookup(name).cloned();
        let owned = prev.as_ref().map(|b| b.owned).unwrap_or(true);
        if let Some(b) = prev {
            if b.local != local {
                let root = self.storage.get(&b.local).cloned().unwrap_or((b.local.clone(), b.ty.clone()));
                self.storage.insert(local.clone(), root);
            }
        }
        self.scopes.last_mut().unwrap().insert(name.to_string(), Bound { local, ty, owned });
    }

    /// Marks a read as consuming when it is the whole of an owned local
    /// [deduce-consume]; a projection or a lent parameter is read, not moved.
    pub(crate) fn consume_if_owned(&self, e: &mut Expr) {
        // Dropping `Mut` from what is moved moves it.
        if let ExprKind::DropMut { value } = &mut e.kind {
            return self.consume_if_owned(value);
        }
        if let ExprKind::Read { place, consume } = &mut e.kind {
            if place.steps.is_empty() {
                let owned = self.scopes.iter().rev().find_map(|s| s.values().find(|b| b.local == place.root)).map(|b| b.owned).unwrap_or(true);
                if owned {
                    *consume = true;
                }
            }
        }
    }

    /// [fate-move-mode] [fate-partial-move] A move-mode binding's value:
    /// the checker proved every ancestor owned, so the whole of a local or
    /// a field path out of one is moved.
    pub(crate) fn consume_bound(&self, e: &mut Expr) {
        if let ExprKind::Read { place, consume } = &mut e.kind {
            if place.steps.iter().all(|s| matches!(s, Step::Field(_))) {
                let owned = self.scopes.iter().rev().find_map(|s| s.values().find(|b| b.local == place.root)).map(|b| b.owned).unwrap_or(true);
                if owned {
                    *consume = true;
                }
            }
        }
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<&Bound> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    pub(crate) fn key(&self, span: Span) -> (usize, Span) {
        (self.file_idx, span)
    }

    pub(crate) fn ty_of(&mut self, span: Span) -> Ty {
        match self.ctx.ty_of(self.file_idx, span) {
            Some(t) => t,
            None => Ty::Unknown,
        }
    }

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn pop_scope(&mut self) {
        let depth = self.scopes.len();
        self.scoped_aliases.retain(|(d, ..)| *d < depth);
        self.scopes.pop();
    }

    // ------------------------------------------------------------ bodies --

    /// A fn body: the block, with a trailing expression that is the result
    /// where the fn returns a value and the block falls off its end.
    pub fn fn_body(&mut self, block: &ast::Block, ret: &Ty) -> Block {
        for (n, t) in self.self_fields.clone() {
            self.bind(&n, t);
        }
        let value = !ret.is_none_ty() && !ret.is_unknown();
        self.block(block, value)
    }

    /// `value`: the block's last expression statement is its value.
    pub(crate) fn block(&mut self, block: &ast::Block, value: bool) -> Block {
        self.push_scope();
        let saved_env = self.effect_env.len();
        let saved_last = self.last_branch.take();
        let saved_pending = std::mem::take(&mut self.pending);
        let mut stmts = Vec::new();
        let n = block.stmts.len();
        let mut tail: Option<Box<Expr>> = None;
        for (i, s) in block.stmts.iter().enumerate() {
            let is_tail = value && i + 1 == n;
            match s {
                AStmt::Expr(e) if is_tail && !super::stmt_is_divergent(e) && !is_statement_expr(e) => {
                    let lowered = self.expr(e);
                    stmts.append(&mut self.pending);
                    tail = Some(Box::new(lowered));
                }
                _ => {
                    let lowered = self.stmt(s);
                    stmts.append(&mut self.pending);
                    stmts.extend(lowered);
                }
            }
        }
        self.pending = saved_pending;
        self.last_branch = saved_last;
        self.effect_env.truncate(saved_env);
        self.pop_scope();
        Block { stmts, value: tail }
    }

    pub(crate) fn stmt(&mut self, s: &AStmt) -> Vec<Stmt> {
        match s {
            AStmt::Let { pattern, ty, value, span } => self.let_stmt(pattern, ty.as_ref(), value, *span),
            AStmt::Assign { target, value, span } => {
                let mut v = self.expr(value);
                // [fate-move-mode] an assignment that takes ownership.
                if self.ctx.checked.binding_modes.contains(&self.key(*span)) {
                    self.consume_bound(&mut v);
                }
                match self.assign_place_of(target) {
                    Some(place) => vec![Stmt::Assign { place, value: v }],
                    None => {
                        let span = target.span();
                        self.error(span, "assignment to a non-place");
                        vec![Stmt::Expr(unsupported(Ty::Unknown, span, "assign to non-place"))]
                    }
                }
            }
            AStmt::Use { handler, with_items, span } => self.use_stmt(handler, with_items, *span),
            AStmt::Rename(_) | AStmt::Comp(_) => Vec::new(),
            AStmt::Expr(e) => self.expr_stmt(e),
        }
    }

    fn expr_stmt(&mut self, e: &AExpr) -> Vec<Stmt> {
        match e {
            AExpr::Return { value, .. } => {
                let v = value.as_ref().map(|v| {
                    let mut x = self.expr(v);
                    // A returned local is consumed [deduce-consume].
                    self.consume_if_owned(&mut x);
                    x
                });
                vec![Stmt::Return(v)]
            }
            AExpr::Break { value, .. } => {
                let mut out = Vec::new();
                if let Some(v) = value {
                    let x = self.expr(v);
                    match self.loop_results.last().cloned() {
                        Some(Some(result)) => out.push(Stmt::Assign { place: Place { root: result, steps: Vec::new() }, value: x }),
                        _ => out.push(Stmt::Expr(x)),
                    }
                }
                out.push(Stmt::Break);
                out
            }
            AExpr::Continue { .. } => vec![Stmt::Continue],
            AExpr::While { cond, body, else_block, span } => self.while_stmt(cond, body, else_block.as_ref(), *span, None),
            AExpr::For { pattern, iterable, body, else_block, span } => {
                self.for_stmt(pattern, iterable, body, else_block.as_ref(), *span, None)
            }
            AExpr::If { branches, else_block, span } if else_block.is_none() || true => {
                let x = self.if_expr(branches, else_block.as_ref(), *span, false);
                vec![Stmt::Expr(x)]
            }
            AExpr::When { subject, branches, span } => {
                let x = self.when_expr(subject, branches, *span, false);
                vec![Stmt::Expr(x)]
            }
            AExpr::WhenCond { branches, else_block, span } => {
                let x = self.when_cond_expr(branches, Some(else_block), *span, false);
                vec![Stmt::Expr(x)]
            }
            other => {
                let x = self.expr(other);
                vec![Stmt::Expr(x)]
            }
        }
    }

    fn let_stmt(&mut self, pattern: &Pattern, written: Option<&ast::Type>, value: &AExpr, span: Span) -> Vec<Stmt> {
        let v = self.expr(value);
        // The binding's type is the written one when there is one: `let x:
        // Int? = None` holds an `Int?`, whatever the initializer's type.
        let declared: Option<Ty> = written.map(|w| self.ctx.written_ty(self.file_idx, w)).filter(|t| !t.is_unknown());
        let mut consumed_bind = self.ctx.checked.binding_modes.contains(&self.key(span));
        let mut v = v;
        if consumed_bind {
            self.consume_bound(&mut v);
            consumed_bind = false;
        }
        let _ = consumed_bind;
        match pattern {
            Pattern::Ident(name) => {
                let ty = declared.unwrap_or_else(|| v.ty.clone());
                let local = self.bind(&name.name, ty.clone());
                let id = self.id();
                // [deduce-field] a binding the checker keeps as its place.
                if self.ctx.checked.virtual_place_binds.contains_key(&self.key(span)) {
                    if let ExprKind::Read { place, .. } = &v.kind {
                        return vec![Stmt::Alias { id, local, ty, place: place.clone() }];
                    }
                }
                vec![Stmt::Let { id, local, ty, value: v }]
            }
            other => {
                let tmp = self.fresh("__destructured");
                let ty = declared.unwrap_or_else(|| v.ty.clone());
                let id = self.id();
                let mut out = vec![Stmt::Let { id, local: tmp.clone(), ty: ty.clone(), value: v }];
                out.extend(self.destructure(other, tmp, &ty, span));
                out
            }
        }
    }

    /// [let-destructure] Binds a tuple or struct pattern's names from `root`.
    pub(crate) fn destructure(&mut self, pattern: &Pattern, root: Local, ty: &Ty, span: Span) -> Vec<Stmt> {
        let mut out = Vec::new();
        match pattern {
            Pattern::Ident(id) => {
                let local = self.bind(&id.name, ty.clone());
                let nid = self.id();
                out.push(Stmt::Let { id: nid, local, ty: ty.clone(), value: Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root, steps: Vec::new() }, consume: true } } });
            }
            Pattern::Tuple { elems, .. } => {
                let elem_tys: Vec<Ty> = match ty.strip_quals() {
                    Ty::Tuple(es) => es.clone(),
                    _ => vec![Ty::Unknown; elems.len()],
                };
                for (i, p) in elems.iter().enumerate() {
                    let ety = elem_tys.get(i).cloned().unwrap_or(Ty::Unknown);
                    match p {
                        Pattern::Ident(id_) => {
                            let local = self.bind(&id_.name, ety.clone());
                            let nid = self.id();
                            out.push(Stmt::Let {
                                id: nid,
                                local,
                                ty: ety.clone(),
                                value: Expr { ty: ety, span: id_.span, kind: ExprKind::Read { place: Place { root: root.clone(), steps: vec![Step::Tuple(i)] }, consume: true } },
                            });
                        }
                        nested => {
                            let tmp = self.fresh("__elem");
                            let nid = self.id();
                            out.push(Stmt::Let { id: nid, local: tmp.clone(), ty: ety.clone(), value: Expr { ty: ety.clone(), span, kind: ExprKind::Read { place: Place { root: root.clone(), steps: vec![Step::Tuple(i)] }, consume: true } } });
                            out.extend(self.destructure(nested, tmp, &ety, span));
                        }
                    }
                }
            }
            Pattern::Struct { fields, .. } => {
                let decl = match ty.strip_quals() {
                    Ty::Named { name, .. } => self.ctx.symbols.structs.get(name.as_str()).copied(),
                    _ => None,
                };
                for f in fields {
                    let fty = decl
                        .and_then(|d| d.fields.iter().find(|x| x.name.name == f.field.name))
                        .map(|x| self.ctx.written_ty(self.file_idx, &x.ty))
                        .unwrap_or(Ty::Unknown);
                    let local = self.bind(&f.binding.name, fty.clone());
                    let nid = self.id();
                    out.push(Stmt::Let {
                        id: nid,
                        local,
                        ty: fty.clone(),
                        value: Expr { ty: fty, span: f.span, kind: ExprKind::Read { place: Place { root: root.clone(), steps: vec![Step::Field(f.field.name.clone())] }, consume: true } },
                    });
                }
            }
        }
        out
    }

    // ------------------------------------------------------------ places --

    /// The place an expression names, when it is one; a read of a narrowed
    /// place goes through its narrowing binding.
    pub(crate) fn place_of(&mut self, e: &AExpr) -> Option<Place> {
        match e {
            AExpr::Ident(id) => {
                let b = self.lookup(&id.name)?.clone();
                let local = self.narrowed_local(&id.name, &b, id.span);
                Some(Place { root: local, steps: Vec::new() })
            }
            AExpr::Field { base, field, .. } => {
                let mut p = self.place_of(base)?;
                p.steps.push(Step::Field(field.name.clone()));
                Some(p)
            }
            AExpr::TupleIndex { base, index, .. } => {
                let mut p = self.place_of(base)?;
                p.steps.push(Step::Tuple(*index));
                Some(p)
            }
            AExpr::Index { base, index, .. } => {
                let mut p = self.place_of(base)?;
                let i = self.expr(index);
                p.steps.push(Step::Index(Box::new(i)));
                Some(p)
            }
            _ => None,
        }
    }

    /// The place an expression names, read *as stored*: the binding in scope,
    /// without a narrowing alias for this read. What a test's subject is
    /// narrowed from [ir-narrow].
    pub(crate) fn stored_place_of(&mut self, e: &AExpr) -> Option<Place> {
        match e {
            AExpr::Ident(id) => {
                let b = self.lookup(&id.name)?.clone();
                Some(Place { root: b.local, steps: Vec::new() })
            }
            AExpr::Field { base, field, .. } => {
                let mut p = self.stored_place_of(base)?;
                p.steps.push(Step::Field(field.name.clone()));
                Some(p)
            }
            AExpr::TupleIndex { base, index, .. } => {
                let mut p = self.stored_place_of(base)?;
                p.steps.push(Step::Tuple(*index));
                Some(p)
            }
            AExpr::Index { base, index, .. } => {
                let mut p = self.stored_place_of(base)?;
                let i = self.expr(index);
                p.steps.push(Step::Index(Box::new(i)));
                Some(p)
            }
            _ => None,
        }
    }

    /// The place an assignment writes: the *storage* behind a narrowed name,
    /// after which the name is the storage again (the narrowing no longer
    /// holds) [ir-narrow].
    pub(crate) fn assign_place_of(&mut self, e: &AExpr) -> Option<Place> {
        match e {
            AExpr::Ident(id) => {
                let b = self.lookup(&id.name)?.clone();
                match self.storage.get(&b.local).cloned() {
                    Some((root, ty)) => {
                        self.rebind(&id.name, root.clone(), ty);
                        Some(Place { root, steps: Vec::new() })
                    }
                    None => Some(Place { root: b.local, steps: Vec::new() }),
                }
            }
            // A projection writes into the value, which a narrowing alias
            // shares with its storage.
            other => self.place_of(other),
        }
    }

    /// [ir-narrow] A read of a name the flow narrowed (`repr_ty` differs
    /// from `expr_ty`) goes through a narrowing binding: the one an enclosing
    /// arm introduced, or a fresh one justified by the last branch, which the
    /// flow said left on the other arm.
    fn narrowed_local(&mut self, name: &str, bound: &Bound, span: Span) -> Local {
        let Some(declared) = self.ctx.checked.repr_ty.get(&self.key(span)) else {
            return bound.local.clone();
        };
        let logical = self.ty_of(span);
        if logical.is_unknown() || erase(declared) == logical || bound.ty == logical {
            return bound.local.clone();
        }
        // A narrowing binding for the whole declared type is in scope under
        // this name already when `bound.ty == logical`; otherwise make one.
        let local = self.fresh(name);
        let because = match (self.current_test, self.last_branch) {
            (Some(j), _) => j,
            (None, Some(b)) => Justification::After { branch: b, arm: 0 },
            (None, None) => Justification::After { branch: NodeId(0), arm: 0 },
        };
        let id = self.id();
        self.pending.push(Stmt::Narrow {
            id,
            local: local.clone(),
            ty: logical.clone(),
            from: Place { root: bound.local.clone(), steps: Vec::new() },
            from_ty: bound.ty.clone(),
            because,
        });
        self.rebind(name, local.clone(), logical);
        local
    }

    /// A narrowing binding at the top of an arm: `name` is the subject's
    /// name (or the `is` binding's), `ty` the arm's type.
    pub(crate) fn narrow_in_arm(&mut self, name: &str, from: Place, from_ty: Ty, ty: Ty, because: Justification) -> Stmt {
        debug_assert!(!ty.is_none_ty());
        let local = self.fresh(name);
        self.rebind(name, local.clone(), ty.clone());
        let id = self.id();
        Stmt::Narrow { id, local, ty, from, from_ty, because }
    }

    pub(crate) fn set_last_branch(&mut self, id: NodeId) {
        self.last_branch = Some(id);
    }

    pub(crate) fn with_test<T>(&mut self, j: Option<Justification>, f: impl FnOnce(&mut Self) -> T) -> T {
        let saved = std::mem::replace(&mut self.current_test, j);
        let out = f(self);
        self.current_test = saved;
        out
    }
}

/// Expressions that are statements by shape: lowered through `expr_stmt`
/// even in tail position, since they produce no value of their own.
fn is_statement_expr(e: &AExpr) -> bool {
    matches!(e, AExpr::While { .. } | AExpr::For { .. }) && false
}
