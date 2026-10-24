//! Expressions (IR record §2): every construct lowered to the node set, every
//! resolution copied from the checker.

use crate::check::{CompareVia, ImplicitArg, UnionTest};
use crate::param_mode::PassMode;
use crate::types::Ty;
use salvo_syntax::ast::{self, BinaryOp, Expr as AExpr, LambdaBody, Pattern, StrExprPart, UnaryOp};
use salvo_syntax::Span;

use super::Lower;
use crate::ir::build::{apply_coercion, erase, unsupported};
use crate::ir::*;

/// [fn-param-default] Every name a default expression's `Ident`s mention —
/// the earlier parameters it may read. Used to decide, once per call, which
/// filled parameters are worth binding to a local before lowering the
/// defaults after them (`args_for`): binding every parameter unconditionally
/// would wrap an ordinary call with needless `Let`s, so only a name a
/// default actually names earns one.
fn mentioned_idents(expr: &AExpr) -> std::collections::HashSet<String> {
    struct Collect {
        found: std::collections::HashSet<String>,
    }
    impl salvo_syntax::visit::Visitor for Collect {
        fn visit_expr(&mut self, expr: &AExpr) {
            if let AExpr::Ident(id) = expr {
                self.found.insert(id.name.clone());
            }
        }
    }
    let mut v = Collect { found: std::collections::HashSet::new() };
    salvo_syntax::visit::walk_expr(&mut v, expr);
    v.found
}

impl<'a, 'p> Lower<'a, 'p> {
    /// An expression, with the slot's coercion applied [ir-coerce].
    pub fn expr(&mut self, e: &AExpr) -> Expr {
        let raw = self.expr_raw(e);
        match self.ctx.checked.coerce.get(&self.key(e.span())) {
            Some(c) => apply_coercion(c, raw, e.span()),
            None => raw,
        }
    }

    fn lit(&mut self, span: Span, kind: ExprKind) -> Expr {
        Expr { ty: self.ty_of(span), span, kind }
    }

    fn expr_raw(&mut self, e: &AExpr) -> Expr {
        let span = e.span();
        match e {
            AExpr::Int { value, long, .. } => {
                // [lit-numeric] an `Int` literal the checker typed as `Long`
                // (against a `Long` operand) is a `Long` value in the IR.
                let ty = self.ty_of(span);
                let base = match ty.strip_quals() {
                    Ty::Named { name, .. } => name.as_str(),
                    _ => "",
                };
                let kind = match base {
                    _ if *long => ExprKind::Long(*value),
                    "Long" => ExprKind::Long(*value),
                    "Double" => ExprKind::Double(*value as f64),
                    "Float" => ExprKind::Float(*value as f64),
                    _ => ExprKind::Int(*value),
                };
                Expr { ty, span, kind }
            }
            AExpr::Float { value, single, .. } => {
                // [lit-adopt] a literal the checker typed `Float` is one.
                let ty = self.ty_of(span);
                let is_float = *single || matches!(ty.strip_quals(), Ty::Named { name, .. } if name == "Float");
                let kind = if is_float { ExprKind::Float(*value) } else { ExprKind::Double(*value) };
                Expr { ty, span, kind }
            }
            AExpr::Bool { value, .. } => self.lit(span, ExprKind::Bool(*value)),
            AExpr::Char { value, .. } => self.lit(span, ExprKind::Char(*value)),
            AExpr::Str { parts, .. } => self.string(parts, span),
            AExpr::Ident(id) => self.ident(id),
            AExpr::Field { .. } | AExpr::TupleIndex { .. } | AExpr::Index { .. } => self.read(e),
            AExpr::Scoped { name, .. } | AExpr::EffectScoped { name, .. } => {
                // A fn named through a selector, as a value.
                match self.ctx.checked.fn_refs.get(&self.key(name.span)).copied() {
                    Some(k) => self.fn_value(k, name.span, span),
                    None => {
                        self.error(span, "unresolved scoped reference");
                        unsupported(self.ty_of(span), span, "scoped reference")
                    }
                }
            }
            AExpr::Call { callee, type_args: _, args, named, .. } => self.call(callee, args, named, span),
            AExpr::Tuple { elems, .. } => {
                let es: Vec<Expr> = elems.iter().map(|x| self.expr(x)).collect();
                let mut ty = self.ty_of(span);
                if ty.is_unknown() {
                    ty = Ty::Tuple(es.iter().map(|x| x.ty.clone()).collect());
                }
                Expr { ty, span, kind: ExprKind::Tuple(es) }
            }
            AExpr::StructLit { ty, fields, .. } => self.struct_lit(ty.as_ref(), fields, span),
            AExpr::Unary { op, operand, .. } => {
                // [ir-narrow] A test under `!` proves its opposite where it
                // holds: nothing it narrows outlives the operand.
                let negated = matches!(op, UnaryOp::Not);
                let mark = self.pending_after_test.len();
                if negated {
                    self.push_scope();
                }
                let x = self.expr(operand);
                if negated {
                    self.pop_scope();
                    self.pending_after_test.truncate(mark);
                }
                let op = match op {
                    UnaryOp::Neg => Op::Neg,
                    UnaryOp::Not => Op::Not,
                };
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Op { op, args: vec![x] } }
            }
            AExpr::Binary { op, lhs, rhs, .. } => self.binary(*op, lhs, rhs, span),
            AExpr::Is { subject, binding, .. } => self.is_test(subject, binding.as_ref(), span),
            AExpr::Widen { subject, binding, .. } => {
                // [qual-lift] the test is the same as `is`; the widening is
                // erased with the qualifiers, so the binding is the subject
                // under its widened type.
                let test = self.ctx.checked.is_tests.get(&self.key(span)).cloned();
                let s = self.expr_subject(subject);
                let id = self.id();
                if let Some(b) = binding {
                    let ty = self.ty_of(b.span);
                    let from = self.stored_place_of(subject).unwrap_or(Place { root: Local("<subject>".into()), steps: Vec::new() });
                    let local = Local(b.name.clone());
                    self.rebind(&b.name, local.clone(), ty.clone());
                    let nid = self.id();
                    let from_ty = s.ty.clone();
                    self.pending_after_test.push(Stmt::Narrow { id: nid, local, ty, from, from_ty, because: Justification::Test { test: id } });
                } else if let (Some(target), true) = (self.ctx.checked.widen_targets.get(&self.key(span)).map(erase), self.in_condition) {
                    // [qual-lift] A lift that peels a wrapper arm narrows the
                    // subject to the widened type for the branch.
                    if let Some(place) = self.stored_place_of(subject) {
                        let from_ty = s.ty.clone();
                        match subject.as_ref() {
                            AExpr::Ident(name) => {
                                let narrow = self.narrow_in_arm(&name.name, place, from_ty, target, Justification::Test { test: id });
                                self.pending_after_test.push(narrow);
                            }
                            // A projection has no name to rebind: the view is
                            // a local the place reads through until the
                            // branch closes.
                            AExpr::Field { .. } | AExpr::TupleIndex { .. } => {
                                let local = self.fresh("__lifted");
                                let nid = self.id();
                                self.pending_after_test.push(Stmt::Narrow { id: nid, local: local.clone(), ty: target.clone(), from: place.clone(), from_ty, because: Justification::Test { test: id } });
                                self.scoped_aliases.push((self.scopes.len(), place, local, target));
                            }
                            _ => {}
                        }
                    }
                }
                match test {
                    Some(t) => Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id, subject: Box::new(s), test: arm_test(&t) } },
                    None => Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) },
                }
            }
            AExpr::SafeNav { base, access, .. } => self.safe_nav(base, access, span),
            AExpr::Assert { cond, message, .. } => {
                let c = self.expr(cond);
                let mut m = message.as_ref().map(|m| Box::new(self.expr(m)));
                // [qual-whynot] `assert!(x is Q(…))` with no message of its
                // own fails with what `whynot?(x is Q(…))` would say.
                if m.is_none() {
                    m = self.assert_whynot(cond).map(Box::new);
                }
                Expr { ty: Ty::none(), span, kind: ExprKind::Assert { cond: Box::new(c), message: m, at: self.location(span) } }
            }
            AExpr::WhyNot { test, .. } => self.why_not(test, span),
            AExpr::Unreachable { message, .. } => {
                let m = message.as_ref().map(|m| Box::new(self.expr(m)));
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Unreachable { message: m, at: self.location(span) } }
            }
            // [assert-fn] `todo!()` lowers to the same trap as `unreachable!()`
            // — the warning is the checker's doing, so the IR and both
            // backends need not distinguish them.
            AExpr::Todo { message, .. } => {
                let m = message.as_ref().map(|m| Box::new(self.expr(m)));
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Unreachable { message: m, at: self.location(span) } }
            }
            AExpr::IncDec { operand, down, prefix, .. } => {
                // `x++`: `x = x + 1`; as a value, the new (prefix) or old
                // (postfix) value.
                let ty = self.ty_of(operand.span());
                let Some(place) = self.place_of(operand) else {
                    return unsupported(ty, span, "inc/dec of a non-place");
                };
                let read = Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: place.clone(), consume: false } };
                let one = Expr { ty: ty.clone(), span, kind: if matches!(ty.strip_quals(), Ty::Named { name, .. } if name == "Long") { ExprKind::Long(1) } else { ExprKind::Int(1) } };
                let value = Expr { ty: ty.clone(), span, kind: ExprKind::Op { op: if *down { Op::Sub } else { Op::Add }, args: vec![read.clone(), one] } };
                let assign = Stmt::Assign { place: place.clone(), value };
                if *prefix {
                    self.block_expr(vec![assign], read, span)
                } else {
                    let old = self.fresh("__old");
                    let id = self.id();
                    let bind = Stmt::Let { id, local: old.clone(), ty: ty.clone(), value: read };
                    let out = Expr { ty, span, kind: ExprKind::Read { place: Place { root: old, steps: Vec::new() }, consume: true } };
                    self.block_expr(vec![bind, assign], out, span)
                }
            }
            AExpr::If { branches, else_block, .. } => self.if_expr(branches, else_block.as_ref(), span, true),
            AExpr::When { binding, subject, branches, .. } => self.when_expr(binding.as_ref(), subject, branches, span, true),
            AExpr::WhenCond { branches, else_block, .. } => self.when_cond_expr(branches, Some(else_block), span, true),
            AExpr::While { cond, body, else_block, .. } => {
                let ty = self.ty_of(span);
                // [loop-never] A `while true` with no `break` is typed
                // `Never` by the checker: it produces no value, so build it
                // as a diverging statement block (no loop-value slot), the
                // same shape as an escape in value position.
                if matches!(ty, Ty::Never) {
                    let mut stmts = self.while_stmt(cond, body, else_block.as_ref(), span, None);
                    // The loop never terminates, so control never reaches
                    // here — but a trap gives the block a `Never` tail the
                    // backends can see (Rust reads `loop {}` as `!` on its
                    // own; Kotlin's `while (true) {}` is `Unit`, so the trap
                    // is what makes the fn type-check). Dead at runtime.
                    stmts.push(Stmt::Expr(Expr {
                        ty: Ty::Never,
                        span,
                        kind: ExprKind::Unreachable { message: None, at: self.location(span) },
                    }));
                    return Expr {
                        ty: Ty::Never,
                        span,
                        kind: ExprKind::Branch {
                            id: NodeId(0),
                            arms: vec![(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) }, Block { stmts, value: None, span })],
                            otherwise: None,
                        },
                    };
                }
                let result = self.fresh("__loop");
                let stmts = self.while_stmt(cond, body, else_block.as_ref(), span, Some((result.clone(), ty.clone())));
                self.loop_value(stmts, result, ty, span)
            }
            AExpr::For { pattern, iterable, body, else_block, .. } => {
                let result = self.fresh("__loop");
                let ty = self.ty_of(span);
                let stmts = self.for_stmt(pattern, iterable, body, else_block.as_ref(), span, Some((result.clone(), ty.clone())));
                self.loop_value(stmts, result, ty, span)
            }
            AExpr::Lambda { params, body, .. } => self.lambda(params, body, span),
            AExpr::Try { body, .. } => {
                let b = self.block(body, true);
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Try { body: b } }
            }
            AExpr::Elvis { subject, rhs, .. } => self.elvis(subject, rhs, span),
            AExpr::Pipe { lhs, rhs, .. } => self.pipe(lhs, rhs, span),
            AExpr::SafePipe { lhs, rhs, .. } => self.safe_pipe(lhs, rhs, span),
            AExpr::Placeholder { .. } => match self.placeholder.clone() {
                Some(local) => {
                    let ty = self.ty_of(span);
                    Expr { ty, span, kind: ExprKind::Read { place: Place { root: local, steps: Vec::new() }, consume: false } }
                }
                None => unsupported(self.ty_of(span), span, "placeholder outside `?:`"),
            },
            AExpr::Return { .. } | AExpr::Break { .. } | AExpr::Continue { .. } => {
                // [expr-escape] an escape in value position: a block whose
                // only statement is the escape.
                let stmts = self.stmt(&ast::Stmt::Expr(e.clone()));
                Expr {
                    ty: Ty::Never,
                    span,
                    kind: ExprKind::Branch {
                        id: NodeId(0),
                        arms: vec![(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) }, Block { stmts, value: None, span })],
                        otherwise: None,
                    },
                }
            }
            AExpr::Spread { operand, .. } => self.expr(operand),
            AExpr::SelfScoped { name, .. } => {
                self.error(span, format!("`{}@self` as a value", name.name));
                unsupported(self.ty_of(span), span, "self-scoped value")
            }
            AExpr::SelfAddr { .. } => Expr { ty: self.ty_of(span), span, kind: ExprKind::SelfAddr },
            AExpr::Use { handler, with_items, span } => self.use_expr(handler, with_items, *span),
            AExpr::Project { base, span, .. } => self.project_expr(base, *span),
            AExpr::Merge { items, all, span } => self.merge_expr(items, *all, *span),
            AExpr::Spawn { handler, with_items, pool, join, .. } => {
                let key = self.key(span);
                let info = self.ctx.checked.spawn_info(key);
                let deps: Vec<Ty> = info.spawn_deps.map(|v| v.iter().map(erase).collect()).unwrap_or_default();
                let items: Vec<Option<usize>> = info.spawn_dep_items.cloned().unwrap_or_else(|| vec![None; deps.len()]);
                let h = match self.construct_handler(handler, &deps, &items, with_items, span) {
                    Some(h) => h,
                    None => self.expr(handler),
                };
                let pool = pool.as_ref().map(|p| Box::new(self.expr(p)));
                let join = join.as_ref().map(|j| Box::new(self.expr(j)));
                let effects: Vec<Ty> = info.spawn_effects.map(|v| v.iter().map(erase).collect()).unwrap_or_default();
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Spawn { handler: Box::new(h), deps: Vec::new(), pool, join, effects } }
            }
            AExpr::ReplyTo { member, captures, gated, pool, .. } => {
                let key = self.key(span);
                let info = self.ctx.checked.spawn_info(key);
                let caps: Vec<Expr> = captures.iter().map(|x| self.expr(x)).collect();
                let pool = pool.as_ref().map(|p| Box::new(self.expr(p)));
                let target = match info.replyto_task.copied() {
                    Some(k) => {
                        let effects: Vec<Expr> = info.task_mint_effects.cloned().unwrap_or_default().iter().map(|t| self.effect_instance(t, span)).collect();
                        ReplyTarget::Task { target: self.ctx.decl_id(k), effects }
                    }
                    None => ReplyTarget::Member(info.replyto_member.cloned().unwrap_or(member.name.clone())),
                };
                Expr { ty: self.ty_of(span), span, kind: ExprKind::ReplyTo { target, captures: caps, gated: *gated, pool } }
            }
            AExpr::WaitFor { binding, body, .. } => {
                let ty = self.ty_of(span);
                self.push_scope();
                let token_ty = self.ty_of(binding.span);
                let local = self.bind(&binding.name, token_ty.clone());
                let b = self.block(body, false);
                self.pop_scope();
                Expr { ty, span, kind: ExprKind::WaitFor { local, token_ty, body: b } }
            }
            AExpr::FieldDefault { owner, field, .. } => self.field_default(owner, field, span),
            AExpr::Error { .. } => unsupported(Ty::Unknown, span, "parse error"),
        }
    }

    // ------------------------------------------------------------- reads --

    /// [fn-effects] A named fn as a value. Where the fn type it is passed as
    /// threads other effects than the fn declares (a pure fn where effects
    /// are threaded — the variance rule), an adapter lambda takes the
    /// type's effects and passes on the ones the fn needs. [fn-value] Where
    /// the fn has implicits of its own, the same lambda fills them, as an
    /// implicit filled by the fn does [implicit-recursive]. `name_span` is
    /// where the checker keyed the name, `span` the whole value.
    fn fn_value(&mut self, k: crate::FnKey, name_span: Span, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let id = self.ctx.decl_id(k);
        // [throw] Neither side threads a throw [`crate::is_throw_effect`].
        let taken: Vec<Ty> = self.ctx.checked.lambda_effects.get(&self.key(name_span)).map(|v| crate::threaded_effects(v).iter().map(erase).collect()).unwrap_or_default();
        let declared: Vec<Ty> = self.ctx.checked.fn_effects.get(&k).map(|v| crate::threaded_effects(v).iter().map(erase).collect()).unwrap_or_default();
        let nested = self.ctx.checked.fn_value_implicits.get(&self.key(name_span)).cloned().unwrap_or_default();
        if taken == declared && nested.is_empty() {
            return Expr { ty, span, kind: ExprKind::FnValue(FnRef::Decl(id)) };
        }
        let (params, ret) = match ty.strip_quals() {
            Ty::Fn { params, ret, .. } => (params.clone(), (**ret).clone()),
            _ => (Vec::new(), Ty::Unknown),
        };
        let mut ps: Vec<Param> = Vec::new();
        for (i, t) in taken.iter().enumerate() {
            ps.push(Param { local: Local(format!("__fx{i}")), ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None, span });
        }
        let mut args: Vec<Expr> = Vec::new();
        for t in &declared {
            match taken.iter().position(|x| x == t) {
                Some(i) => args.push(Expr { ty: t.clone(), span, kind: ExprKind::Read { place: Place { root: Local(format!("__fx{i}")), steps: Vec::new() }, consume: false } }),
                None => {
                    self.error(span, format!("a fn needs effect `{t}`, which the fn type it is passed as does not declare"));
                    return unsupported(ty, span, "fn value adapter");
                }
            }
        }
        for (i, t) in params.iter().enumerate() {
            let l = Local(format!("__a{i}"));
            ps.push(Param { local: l.clone(), ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None, span });
            args.push(Expr { ty: t.clone(), span, kind: ExprKind::Read { place: Place { root: l, steps: Vec::new() }, consume: false } });
        }
        args.extend(self.implicit_list(&nested, &[], span));
        let call = Expr { ty: ret.clone(), span, kind: ExprKind::Call { target: FnRef::Decl(id), type_args: Vec::new(), args } };
        Expr { ty, span, kind: ExprKind::Lambda { params: ps, ret, body: Block { stmts: Vec::new(), value: Some(Box::new(call)), span }, captures: Vec::new() } }
    }

    fn ident(&mut self, id: &ast::Ident) -> Expr {
        let span = id.span;
        if id.name == "None" {
            let ty = self.ty_of(span);
            return Expr { ty, span, kind: ExprKind::MakeNone };
        }
        if self.lookup(&id.name).is_some() {
            return self.read(&AExpr::Ident(id.clone()));
        }
        if let Some(k) = self.ctx.checked.fn_refs.get(&self.key(span)).copied() {
            return self.fn_value(k, span, span);
        }
        if let Some(local) = self.implicit_locals.get(&id.name).cloned() {
            return Expr { ty: self.ty_of(span), span, kind: ExprKind::FnValue(FnRef::Local(local)) };
        }
        self.error(span, format!("unresolved identifier `{}`", id.name));
        unsupported(self.ty_of(span), span, format!("ident {}", id.name))
    }

    pub(crate) fn read(&mut self, e: &AExpr) -> Expr {
        let span = e.span();
        let ty = self.ty_of(span);
        let Some(place) = self.place_of(e) else {
            // A projection of a non-place (`f().x`): read through a temp.
            return match e {
                AExpr::Field { base, field, .. } => {
                    let b = self.expr(base);
                    let tmp = self.fresh("__proj");
                    let id = self.id();
                    let bt = b.ty.clone();
                    let stmts = vec![Stmt::Let { id, local: tmp.clone(), ty: bt, value: b }];
                    let read = Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp, steps: vec![Step::Field(field.name.clone())] }, consume: true } };
                    self.block_expr(stmts, read, span)
                }
                AExpr::TupleIndex { base, index, .. } => {
                    let b = self.expr(base);
                    let tmp = self.fresh("__proj");
                    let id = self.id();
                    let bt = b.ty.clone();
                    let stmts = vec![Stmt::Let { id, local: tmp.clone(), ty: bt, value: b }];
                    let read = Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp, steps: vec![Step::Tuple(*index)] }, consume: true } };
                    self.block_expr(stmts, read, span)
                }
                AExpr::Index { base, index, .. } => {
                    let b = self.expr(base);
                    let i = self.expr(index);
                    let tmp = self.fresh("__proj");
                    let id = self.id();
                    let bt = b.ty.clone();
                    let stmts = vec![Stmt::Let { id, local: tmp.clone(), ty: bt, value: b }];
                    let read = Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp, steps: vec![Step::Index(Box::new(i))] }, consume: true } };
                    self.block_expr(stmts, read, span)
                }
                _ => unsupported(ty, span, "read of a non-place"),
            };
        };
        let key = self.key(span);
        // A projection whose prefix an enclosing safe call bound reads
        // through that binding.
        let projected = !place.steps.is_empty();
        let aliased = projected.then(|| self.place_alias(&place).map(|a| a.ty)).flatten();
        let place = self.through_alias(place);
        let consume = self.ctx.checked.linear_moves.contains(&key)
            || self.ctx.checked.state_takes.contains(&key)
            || self.ctx.checked.moved_projections.contains(&key);
        // [qual-field-override] A field a qualifier of the subject refines
        // is read through a narrowing binding justified by the claim.
        if let (AExpr::Field { base, field, .. }, Some(over)) = (e, self.ctx.checked.field_casts.get(&key).map(erase)) {
            let base_ty = self.ty_of(base.span());
            let declared = match base_ty.strip_quals() {
                Ty::Named { name, .. } => self.ctx.symbols.structs.get(name.as_str()).and_then(|sd| sd.fields.iter().find(|f| f.name.name == field.name).map(|f| match &f.ty {
                    Some(t) => self.ctx.written_ty(self.file_idx, t),
                    None => self.ctx.ty_of(self.file_idx, f.name.span).unwrap_or(Ty::Unknown),
                })),
                _ => None,
            };
            if let Some(declared) = declared.filter(|d| *d != over) {
                let local = self.fresh("__claimed");
                let id = self.id();
                self.pending.push(Stmt::Narrow { id, local: local.clone(), ty: over.clone(), from: place, from_ty: declared, because: Justification::Claim });
                return Expr { ty: over, span, kind: ExprKind::Read { place: Place { root: local, steps: Vec::new() }, consume } };
            }
        }
        // [ir-narrow] A projection the flow narrowed (`p.inner` after a test
        // on it) is read through a narrowing binding, like a local.
        if projected {
            // A place read through a lifted view is narrowed from the view.
            if let Some(declared) = aliased.clone().or_else(|| self.ctx.checked.repr_ty.get(&key).map(erase)) {
                if !ty.is_unknown() && declared != ty {
                    let local = self.fresh("__narrowed");
                    let because = match (self.current_test, self.last_branch) {
                        (Some(j), _) => j,
                        (None, Some(b)) => Justification::After { branch: b, arm: 0 },
                        (None, None) => Justification::After { branch: NodeId(0), arm: 0 },
                    };
                    let id = self.id();
                    self.pending.push(Stmt::Narrow { id, local: local.clone(), ty: ty.clone(), from: place, from_ty: declared, because });
                    return Expr { ty, span, kind: ExprKind::Read { place: Place { root: local, steps: Vec::new() }, consume } };
                }
            }
        }
        Expr { ty, span, kind: ExprKind::Read { place, consume } }
    }

    /// Statements followed by a value, as a `Branch` with one always-true arm
    /// (the IR has no block expression of its own).
    pub(crate) fn block_expr(&mut self, stmts: Vec<Stmt>, value: Expr, span: Span) -> Expr {
        let ty = value.ty.clone();
        let id = self.id();
        Expr {
            ty,
            span,
            kind: ExprKind::Branch {
                id,
                arms: vec![(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) }, Block { stmts, value: Some(Box::new(value)), span })],
                otherwise: None,
            },
        }
    }

    /// [ir-loop] A loop's value: the result local holds `T?` while it runs
    /// (`None` until a `break value` or the `else`), and is read narrowed
    /// afterwards, justified by the loop's totality.
    fn loop_value(&mut self, stmts: Vec<Stmt>, result: Local, ty: Ty, span: Span) -> Expr {
        let loop_id = stmts.iter().find_map(|s| match s { Stmt::Loop { id, .. } => Some(*id), _ => None }).unwrap_or(NodeId(0));
        let opt = Ty::union_of(vec![ty.clone(), Ty::none()]);
        let mut stmts: Vec<Stmt> = stmts
            .into_iter()
            .map(|s| match s {
                Stmt::Let { id, local, ty: t, value } if local == result && t == ty => Stmt::Let { id, local, ty: opt.clone(), value },
                other => other,
            })
            .collect();
        let narrowed = self.fresh("__loop_value");
        let id = self.id();
        stmts.push(Stmt::Narrow { id, local: narrowed.clone(), ty: ty.clone(), from: Place { root: result, steps: Vec::new() }, from_ty: opt, because: Justification::LoopValue { loop_: loop_id } });
        self.block_value(stmts, narrowed, ty, span)
    }

    pub(crate) fn block_value(&mut self, stmts: Vec<Stmt>, result: Local, ty: Ty, span: Span) -> Expr {
        let read = Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: result, steps: Vec::new() }, consume: true } };
        self.block_expr(stmts, read, span)
    }

    // ----------------------------------------------------------- strings --

    fn string(&mut self, parts: &[StrExprPart], span: Span) -> Expr {
        let ty = Ty::named("Str");
        if parts.iter().all(|p| matches!(p, StrExprPart::Text(_))) {
            let text: String = parts.iter().map(|p| match p { StrExprPart::Text(t) => t.as_str(), _ => "" }).collect();
            return Expr { ty, span, kind: ExprKind::Str(text) };
        }
        let mut out = Vec::new();
        for p in parts {
            match p {
                StrExprPart::Text(t) => out.push(Expr { ty: ty.clone(), span, kind: ExprKind::Str(t.clone()) }),
                StrExprPart::Interp(e) => out.push(self.interp(e)),
            }
        }
        Expr { ty, span, kind: ExprKind::Concat(out) }
    }

    /// [interp-to-str] An interpolated part: the resolved `to_str` call, or
    /// the value itself when it is already a `Str`.
    fn interp(&mut self, e: &AExpr) -> Expr {
        let span = e.span();
        let value = self.expr(e);
        let ty = value.ty.clone();
        match self.text_of(value, span) {
            Some(text) => text,
            None => {
                self.error(span, format!("no `to_str` for `{ty}` in an interpolation"));
                unsupported(Ty::named("Str"), span, "interpolation")
            }
        }
    }

    /// [interp-to-str] The text of `value`, by the form the checker recorded
    /// at `span` (the expression's own), else natively; `None` when it has
    /// none.
    fn text_of(&mut self, value: Expr, span: Span) -> Option<Expr> {
        let str_ty = Ty::named("Str");
        let info = self.ctx.checked.interp_info(self.key(span));
        if let Some(local) = info.implicit.cloned() {
            return Some(Expr { ty: str_ty, span, kind: ExprKind::Call { target: FnRef::Local(Local(local)), type_args: Vec::new(), args: vec![value] } });
        }
        // [interp-union] A union is the text of the arm it holds: a switch on
        // the value, each arm calling its own `to_str`.
        if let Some(arms) = info.union.cloned() {
            return Some(self.interp_union(value, &arms, span));
        }
        if let Some(k) = info.to_str.copied() {
            let at = Span::new(span.end, span.end);
            let mut args = vec![value];
            args.extend(self.implicit_args_at(at));
            return Some(Expr { ty: str_ty, span, kind: ExprKind::Call { target: FnRef::Decl(self.ctx.decl_id(k)), type_args: Vec::new(), args } });
        }
        self.native_text(value, span)
    }

    /// The text of a `Str` (itself) or a scalar the target renders natively
    /// (the std `to_str` for it).
    fn native_text(&mut self, value: Expr, span: Span) -> Option<Expr> {
        if matches!(value.ty.strip_quals(), Ty::Named { name, .. } if name == "Str") {
            return Some(value);
        }
        let id = self.intrinsic_fn("to_str", &[value.ty.clone()])?;
        Some(Expr { ty: Ty::named("Str"), span, kind: ExprKind::Call { target: FnRef::Decl(id), type_args: Vec::new(), args: vec![value] } })
    }

    /// [interp-union] `${u}` for a union `u`: the union's own `to_str`, given
    /// the function that renders each arm (the backend dispatches on the arm
    /// the value holds).
    fn interp_union(&mut self, value: Expr, arms: &[crate::check::ArmText], span: Span) -> Expr {
        use crate::check::ArmText;
        let str_ty = Ty::named("Str");
        let runtime: Vec<Ty> = value.ty.strip_quals().value_arms().into_iter().cloned().collect();
        if runtime.len() != arms.len() {
            self.error(span, format!("the arms of `{}` do not match the text forms the checker found", value.ty));
            return unsupported(str_ty, span, "interpolation of a union");
        }
        let mut fns = Vec::new();
        for (arm_ty, text) in runtime.iter().zip(arms) {
            let fn_ty = Ty::Fn { params: vec![arm_ty.clone()], ret: Box::new(str_ty.clone()), contract: None, effects: Vec::new() };
            let f = match text {
                ArmText::Own(name) => Expr { ty: fn_ty, span, kind: ExprKind::FnValue(FnRef::Local(Local(name.clone()))) },
                ArmText::Fn { key, implicits } => {
                    let arg = ImplicitArg::Resolved { name: "to_str".into(), key: *key, nested: implicits.clone(), want: fn_ty };
                    self.implicit_list(&[arg], &[], span).pop().unwrap()
                }
                ArmText::Native if matches!(arm_ty.strip_quals(), Ty::Named { name, .. } if name == "Str") => {
                    // A `Str` is its own text.
                    let p = Param { local: Local("__s".into()), ty: arm_ty.clone(), mode: PassMode::Lent, variadic: false, check: None, span };
                    let read = Expr { ty: arm_ty.clone(), span, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: false } };
                    Expr { ty: fn_ty, span, kind: ExprKind::Lambda { params: vec![p], ret: str_ty.clone(), body: Block { stmts: Vec::new(), value: Some(Box::new(read)), span }, captures: Vec::new() } }
                }
                ArmText::Native => match self.intrinsic_fn("to_str", &[arm_ty.clone()]) {
                    Some(id) => Expr { ty: fn_ty, span, kind: ExprKind::FnValue(FnRef::Decl(id)) },
                    None => {
                        self.error(span, format!("no `to_str` for `{arm_ty}` in an interpolation"));
                        unsupported(fn_ty, span, "interpolation")
                    }
                },
            };
            fns.push(f);
        }
        Expr { ty: str_ty, span, kind: ExprKind::UnionToStr { value: Box::new(value), arms: fns } }
    }

    /// [iter-protocol] A pass member (`iter`, `next`): the fn reference and its
    /// result type at `subject_ty`.
    /// [iter-protocol] A pass member (`iter`, `next`): the fn reference, its
    /// result type at `subject_ty`, and the effect instances its declaration
    /// takes [fn-effects].
    fn pass_member_full(&mut self, m: &crate::check::PassMember, subject_ty: &Ty) -> (FnRef, Ty, Vec<Ty>) {
        match m {
            crate::check::PassMember::Fn(k) => {
                let effects: Vec<Ty> = self
                    .ctx
                    .checked
                    .fn_effects
                    .get(k)
                    .map(|v| v.iter().filter(|t| !matches!(t, Ty::Named { name, .. } if name == crate::THROW_EFFECT)).map(erase).collect())
                    .unwrap_or_default();
                let ret = match self.ctx.fn_by_key(*k) {
                    Some(d) => {
                        let declared = d.return_type.as_ref().map(|t| self.ctx.written_ty(self.file_idx, t)).unwrap_or_else(Ty::none);
                        let first = d.params.iter().find(|p| !p.implicit).map(|p| self.ctx.written_ty(self.file_idx, &p.ty));
                        let mut map = std::collections::HashMap::new();
                        if let Some(f) = first {
                            unify_vars(&f, subject_ty, &mut map);
                        }
                        subst_map(&declared, &map)
                    }
                    None => Ty::Unknown,
                };
                (FnRef::Decl(self.ctx.decl_id(*k)), ret, effects)
            }
            crate::check::PassMember::Implicit(n) => {
                let local = self.lookup(n).map(|b| b.local.clone()).or_else(|| self.implicit_locals.get(n).cloned()).unwrap_or(Local(n.clone()));
                let (ret, effects) = match self.lookup(&local.0).map(|b| b.ty.clone()) {
                    Some(Ty::Fn { ret, effects, .. }) => ((*ret).clone(), effects.clone()),
                    _ => (Ty::Unknown, Vec::new()),
                };
                (FnRef::Local(local), ret, effects)
            }
        }
    }

    /// The std intrinsic `name` declared over exactly these parameter types.
    pub(crate) fn intrinsic_fn(&self, name: &str, params: &[Ty]) -> Option<DeclId> {
        let fns = self.ctx.symbols.fns.get(name)?;
        let want: Vec<String> = params.iter().map(|t| erase(t).strip_quals().to_string()).collect();
        for f in fns {
            if !f.intrinsic || f.params.iter().filter(|p| !p.implicit).count() != params.len() {
                continue;
            }
            let have: Vec<String> = f.params.iter().filter(|p| !p.implicit).map(|p| p.ty.to_string()).collect();
            if have == want {
                return self.ctx.program.files.iter().enumerate().find_map(|(fi, _)| {
                    let m = &self.ctx.program.modules[fi];
                    m.items.iter().position(|i| matches!(i, ast::Item::Fn(g) if std::ptr::eq(g, *f))).map(|item| self.ctx.item_id(fi, item))
                });
            }
        }
        None
    }

    // --------------------------------------------------------- operators --

    /// [op-promote] An operand, widened to the operator's class width when
    /// the checker promoted it: a literal is retyped, anything else widened.
    fn promoted(&mut self, operand: &AExpr) -> Expr {
        let e = self.expr(operand);
        let Some(target) = self.ctx.checked.promotions.get(&self.key(operand.span())).map(erase) else { return e };
        let tname = match target.strip_quals() {
            Ty::Named { name, .. } => name.as_str(),
            _ => return e,
        };
        let span = e.span;
        match (&e.kind, tname) {
            (ExprKind::Int(v), "Long") => Expr { ty: target, span, kind: ExprKind::Long(*v) },
            (ExprKind::Float(v), "Double") => Expr { ty: target, span, kind: ExprKind::Double(*v) },
            _ => Expr { ty: target, span, kind: ExprKind::Widen { value: Box::new(e) } },
        }
    }

    fn binary(&mut self, op: BinaryOp, lhs: &AExpr, rhs: &AExpr, span: Span) -> Expr {
        let ty = self.ty_of(span);
        match op {
            BinaryOp::And | BinaryOp::Or => {
                // [ir-narrow] What a test proves holds where the test did:
                // after `a && b`, not after `a || b`. A narrowing (or a
                // binding) made inside an `||` does not outlive it.
                let (mark, scoped) = (self.pending_after_test.len(), op == BinaryOp::Or);
                if scoped {
                    self.push_scope();
                }
                let l = self.expr(lhs);
                if scoped {
                    self.pop_scope();
                    self.pending_after_test.truncate(mark);
                }
                // [ir-narrow] The right operand runs only when the left
                // decided nothing, so what it reads may be narrowed by the
                // left: a binding test's name (`&&`), or the failure of a
                // `None` test (`||`). Such narrowings are scoped to it: the
                // operand becomes a branch on the left.
                let after_left: Vec<Stmt> = self.pending_after_test.clone();
                let saved = std::mem::take(&mut self.pending);
                if scoped {
                    self.push_scope();
                }
                let r = self.expr(rhs);
                if scoped {
                    self.pop_scope();
                    self.pending_after_test.truncate(mark);
                }
                let right_pending = std::mem::replace(&mut self.pending, saved);
                let bool_ty = Ty::named("Bool");
                let needs_scope = !right_pending.is_empty() || (op == BinaryOp::And && !after_left.is_empty() && reads_any(&r, &after_left));
                if !needs_scope {
                    return Expr { ty, span, kind: ExprKind::Op { op: if op == BinaryOp::And { Op::And } else { Op::Or }, args: vec![l, r] } };
                }
                let mut stmts = Vec::new();
                if op == BinaryOp::And {
                    stmts.extend(after_left);
                }
                stmts.extend(right_pending);
                let right = Block { stmts, value: Some(Box::new(r)), span };
                let id = self.id();
                let lit = |v: bool| Expr { ty: bool_ty.clone(), span, kind: ExprKind::Bool(v) };
                let (arms, otherwise) = if op == BinaryOp::And {
                    (vec![(l, right)], Some(Block { stmts: Vec::new(), value: Some(Box::new(lit(false))), span }))
                } else {
                    (vec![(l, Block { stmts: Vec::new(), value: Some(Box::new(lit(true))), span })], Some(right))
                };
                Expr { ty, span, kind: ExprKind::Branch { id, arms, otherwise } }
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                let l = self.promoted(lhs);
                let r = self.promoted(rhs);
                let o = match op {
                    BinaryOp::Add => Op::Add,
                    BinaryOp::Sub => Op::Sub,
                    BinaryOp::Mul => Op::Mul,
                    BinaryOp::Div => Op::Div,
                    _ => Op::Rem,
                };
                Expr { ty, span, kind: ExprKind::Op { op: o, args: vec![l, r] } }
            }
            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
                // [op-arm-eq] `union == value` reduced by the checker to an arm
                // test plus a comparison of that arm: lower it to
                // `subject is arm && arm == value`.
                if let Some(ac) = self.ctx.checked.arm_compares.get(&self.key(span)).cloned() {
                    return self.arm_compare(op, &ac, lhs, rhs, span);
                }
                let l = self.promoted(lhs);
                let r = self.promoted(rhs);
                let equality = matches!(op, BinaryOp::Eq | BinaryOp::NotEq);
                // `x == None` is a `None` test of the other side.
                if equality && (l.ty.is_none_ty() || r.ty.is_none_ty()) {
                    let subject = if r.ty.is_none_ty() { l } else { r };
                    let id = self.id();
                    let test = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id, subject: Box::new(subject), test: ArmTest::None } };
                    return if op == BinaryOp::Eq { test } else { Expr { ty, span, kind: ExprKind::Op { op: Op::Not, args: vec![test] } } };
                }
                let via = self.ctx.checked.comparisons.get(&self.key(span)).cloned();
                // [type-basic] [ir-op] On basic types the operator is the
                // host's own; `cmp`/`eq` are only their function values.
                if l.ty.is_basic() && r.ty.is_basic() && !matches!(via, Some(CompareVia::Implicit(_))) {
                    let o = match op {
                        BinaryOp::Eq => Op::Eq,
                        BinaryOp::NotEq => Op::NotEq,
                        BinaryOp::Lt => Op::Lt,
                        BinaryOp::Gt => Op::Gt,
                        BinaryOp::LtEq => Op::LtEq,
                        _ => Op::GtEq,
                    };
                    return Expr { ty, span, kind: ExprKind::Op { op: o, args: vec![l, r] } };
                }
                let target = match via {
                    Some(CompareVia::Call(k)) => Some(FnRef::Decl(self.ctx.decl_id(k))),
                    Some(CompareVia::Implicit(name)) => Some(FnRef::Local(Local(name))),
                    None => {
                        let fname = if equality { "eq" } else { "cmp" };
                        let lt = l.ty.clone();
                        self.intrinsic_fn(fname, &[lt.clone(), lt]).map(FnRef::Decl)
                    }
                };
                let Some(target) = target else {
                    self.error(span, format!("comparison on `{}` resolves to no `cmp`/`eq`", l.ty));
                    return unsupported(ty, span, "comparison");
                };
                let mut args = vec![l, r];
                args.extend(self.implicit_args_at(span));
                let call_ty = if equality { Ty::named("Bool") } else { Ty::named("Int") };
                let call = Expr { ty: call_ty, span, kind: ExprKind::Call { target, type_args: Vec::new(), args } };
                match op {
                    BinaryOp::Eq => call,
                    BinaryOp::NotEq => Expr { ty, span, kind: ExprKind::Op { op: Op::Not, args: vec![call] } },
                    // `a < b` is `cmp(a, b) < 0`: the sign test of the ordering.
                    _ => {
                        let zero = Expr { ty: Ty::named("Int"), span, kind: ExprKind::Int(0) };
                        let o = match op { BinaryOp::Lt => Op::Lt, BinaryOp::Gt => Op::Gt, BinaryOp::LtEq => Op::LtEq, _ => Op::GtEq };
                        Expr { ty, span, kind: ExprKind::Op { op: o, args: vec![call, zero] } }
                    }
                }
            }
        }
    }

    /// [op-arm-eq] Lower `union == value` (checker-recorded) to
    /// `subject is arm && (subject narrowed to arm) == value`. The subject is
    /// bound once so it is read for the test and the comparison without
    /// re-evaluating; `!=` negates the whole thing.
    fn arm_compare(&mut self, op: BinaryOp, ac: &crate::check::ArmCompare, lhs: &AExpr, rhs: &AExpr, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let bool_ty = Ty::named("Bool");
        let (subject_ast, value_ast) = if ac.subject_is_lhs { (lhs, rhs) } else { (rhs, lhs) };
        let s = self.expr_subject(subject_ast);
        let subj_ty = s.ty.clone();
        // Bind the subject once.
        let tmp = self.fresh("__cmp");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: s };
        // `subject is arm`.
        let tid = self.id();
        let test = Expr {
            ty: bool_ty.clone(),
            span,
            kind: ExprKind::Test { id: tid, subject: Box::new(Expr { ty: subj_ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume: false } }), test: arm_test(&ac.test) },
        };
        // The present arm: narrow the subject to the arm and compare with the
        // value by the host's `==` (the arm is a basic type).
        let nar = self.fresh("__arm");
        let nid = self.id();
        let narrow = Stmt::Narrow {
            id: nid,
            local: nar.clone(),
            ty: ac.arm_ty.clone(),
            from: Place { root: tmp.clone(), steps: Vec::new() },
            from_ty: subj_ty.clone(),
            because: Justification::Cond { branch: tid, arm: 0 },
        };
        let value = self.promoted(value_ast);
        let read_arm = Expr { ty: ac.arm_ty.clone(), span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: false } };
        let cmp = Expr { ty: bool_ty.clone(), span, kind: ExprKind::Op { op: Op::Eq, args: vec![read_arm, value] } };
        let present = Block { stmts: vec![narrow], value: Some(Box::new(cmp)), span };
        // `test && present`, as a branch (the narrow holds only in the arm).
        let bid = self.id();
        let eq = Expr {
            ty: bool_ty.clone(),
            span,
            kind: ExprKind::Branch {
                id: bid,
                arms: vec![(test, present)],
                otherwise: Some(Block { stmts: Vec::new(), value: Some(Box::new(Expr { ty: bool_ty.clone(), span, kind: ExprKind::Bool(false) })), span }),
            },
        };
        let result = match op {
            BinaryOp::NotEq => Expr { ty: ty.clone(), span, kind: ExprKind::Op { op: Op::Not, args: vec![eq] } },
            _ => eq,
        };
        self.block_expr(vec![bind], result, span)
    }

    // ------------------------------------------------------------- calls --

    pub(crate) fn type_args_of(&mut self, span: Span) -> Vec<Ty> {
        self.ctx.checked.call_type_args.get(&self.key(span)).map(|v| v.iter().map(erase).collect()).unwrap_or_default()
    }

    /// [implicit-resolve] The arguments filling a callee's implicit
    /// positions, recorded at `at`.
    pub(crate) fn implicit_args_at(&mut self, at: Span) -> Vec<Expr> {
        let Some(filled) = self.ctx.checked.implicit_args.get(&self.key(at)).cloned() else { return Vec::new() };
        self.implicit_list(&filled, &[], at)
    }

    fn implicit_list(&mut self, filled: &[ImplicitArg], named: &[ast::NamedArg], at: Span) -> Vec<Expr> {
        let mut out = Vec::new();
        for arg in filled {
            match arg {
                ImplicitArg::Given { name, .. } => match named.iter().find(|a| a.name.name == *name) {
                    Some(a) => out.push(self.expr(&a.value)),
                    None => out.push(unsupported(Ty::Unknown, at, format!("given implicit `{name}` without a value"))),
                },
                ImplicitArg::Forwarded { name } => match self.lookup(name).map(|b| b.local.clone()).or_else(|| self.implicit_locals.get(name).cloned()) {
                    Some(l) => out.push(Expr { ty: self.lookup(&l.0).map(|b| b.ty.clone()).unwrap_or(Ty::Unknown), span: at, kind: ExprKind::FnValue(FnRef::Local(l)) }),
                    None => {
                        self.error(at, format!("forwarded implicit `{name}` is not in scope"));
                        out.push(unsupported(Ty::Unknown, at, "forwarded implicit"));
                    }
                },
                ImplicitArg::Resolved { key, nested, want, .. } => {
                    let id = self.ctx.decl_id(*key);
                    let want = erase(want);
                    if nested.is_empty() {
                        out.push(Expr { ty: want, span: at, kind: ExprKind::FnValue(FnRef::Decl(id)) });
                    } else {
                        // [implicit-recursive] a fn with implicits of its own
                        // is passed as a lambda that fills them.
                        let (params, ret) = match want.strip_quals() {
                            Ty::Fn { params, ret, .. } => (params.clone(), (**ret).clone()),
                            _ => (Vec::new(), Ty::Unknown),
                        };
                        let ps: Vec<Param> = params.iter().enumerate().map(|(i, t)| Param { local: Local(format!("__i{i}")), ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None, span: at }).collect();
                        let mut args: Vec<Expr> = ps.iter().map(|p| Expr { ty: p.ty.clone(), span: at, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: false } }).collect();
                        args.extend(self.implicit_list(nested, &[], at));
                        let call = Expr { ty: ret.clone(), span: at, kind: ExprKind::Call { target: FnRef::Decl(id), type_args: Vec::new(), args } };
                        out.push(Expr { ty: want, span: at, kind: ExprKind::Lambda { params: ps, ret, body: Block { stmts: Vec::new(), value: Some(Box::new(call)), span: at }, captures: Vec::new() } });
                    }
                }
                ImplicitArg::OriginNext { next_fn, .. } => {
                    let id = self.ctx.decl_id(*next_fn);
                    out.push(Expr { ty: Ty::Unknown, span: at, kind: ExprKind::FnValue(FnRef::Decl(id)) });
                }
            }
        }
        out
    }

    /// The instance in scope for an effect type [effect-available].
    pub(crate) fn effect_instance(&mut self, ty: &Ty, span: Span) -> Expr {
        let want = erase(ty);
        let found = self
            .effect_env
            .iter()
            .rev()
            .find(|(t, _)| *t == want)
            .or_else(|| self.effect_env.iter().rev().find(|(t, _)| base_of(t) == base_of(&want)))
            .map(|(t, l)| (t.clone(), l.clone()));
        match found {
            Some((t, l)) => Expr { ty: t, span, kind: ExprKind::Read { place: Place { root: l, steps: Vec::new() }, consume: false } },
            None => {
                self.error(span, format!("no instance of `{want}` in scope"));
                unsupported(want, span, "effect instance")
            }
        }
    }

    fn call(&mut self, callee: &AExpr, args: &[AExpr], named: &[ast::NamedArg], span: Span) -> Expr {
        let ty = self.ty_of(span);
        let key = self.key(span);
        let checked = self.ctx.checked;
        // [unify-checker] One lookup for the whole call cluster.
        let info = checked.call_info(key);
        // [qual-whynot] `InRange.qualifies(n, 1, 65535)`: a qualifier's own fn.
        if let Some((q, member, of)) = checked.qual_member_calls.get(&key).cloned() {
            let subject = match &of {
                Some(base) => Ty::named(base.clone()),
                None => Ty::Unknown,
            };
            let a: Vec<Expr> = args.iter().map(|x| self.expr(x)).collect();
            return match self.qual_fn_ref(&q, &subject, &member) {
                Some(target) => Expr { ty, span, kind: ExprKind::Call { target: FnRef::Decl(target), type_args: Vec::new(), args: a } },
                None => unsupported(ty, span, format!("`{q}.{member}`")),
            };
        }
        // [throw] `throw(m)`.
        if let Some(site) = info.may_throw {
            if site.performs {
                let m = args.first().map(|a| self.expr(a));
                return match m {
                    Some(m) => Expr { ty: Ty::Never, span, kind: ExprKind::Throw { message: Box::new(m) } },
                    None => unsupported(ty, span, "throw without a message"),
                };
            }
        }
        // [actor-use-addr] a send to an actor through an addr.
        if let Some(effect) = info.addr_call.cloned() {
            if let AExpr::Field { base, field, .. } = callee {
                let addr = self.expr(base);
                let member = self.member_ref_by_name(&effect, &field.name, span);
                let a: Vec<Expr> = args.iter().map(|x| self.expr(x)).collect();
                return match member {
                    Some(member) => Expr { ty, span, kind: ExprKind::Send { addr: Box::new(addr), member, args: a } },
                    None => unsupported(ty, span, "addr send"),
                };
            }
        }
        if let Some(member) = info.self_send.or(info.facade_send).cloned() {
            let a: Vec<Expr> = args.iter().map(|x| self.expr(x)).collect();
            return Expr { ty, span, kind: ExprKind::SelfSend { member, args: a } };
        }
        // Dot-notation [fn-dot]: the receiver is the first argument.
        let mut all_args: Vec<&AExpr> = Vec::with_capacity(args.len() + 1);
        if info.dot_call {
            if let AExpr::Field { base, .. } = callee {
                all_args.push(base);
            }
        }
        // [fn-overload-at] [effect-at] the dot form of a scoped name: the
        // receiver is the first argument.
        if let AExpr::Scoped { base: Some(base), .. } | AExpr::EffectScoped { base: Some(base), .. } = callee {
            all_args.push(base);
        }
        all_args.extend(args.iter());
        // [effect-dispatch]
        if let Some(mc) = info.member_call.cloned() {
            let Some(interface) = self.ctx.decl_of_effect(&mc.effect) else {
                self.error(span, format!("effect `{}` has no declaration", mc.effect));
                return unsupported(ty, span, "member call");
            };
            let instance_ty = info.effect_call.cloned().unwrap_or_else(|| Ty::named(mc.effect.clone()));
            let instance = self.effect_instance(&instance_ty, span);
            let decl = self.ctx.symbols.effects.get(mc.effect.as_str()).copied();
            let member_decl = decl.and_then(|d| d.fns.get(mc.index));
            let (args_lowered, prelude) = self.args_for(member_decl.map(Callee::Member), &all_args, named, span);
            let mut lowered = args_lowered;
            lowered.extend(self.implicit_list(&info.implicit_args.cloned().unwrap_or_default(), named, span));
            let type_args = self.type_args_of(span);
            let call = Expr { ty: ty.clone(), span, kind: ExprKind::MemberCall { instance: Box::new(instance), member: MemberRef { interface, index: mc.index }, type_args, args: lowered } };
            return if prelude.is_empty() { call } else { self.block_expr(prelude, call, span) };
        }
        // A fn-typed local, or the fn's own implicit [call-resolve].
        if info.local_call {
            let name = match callee {
                AExpr::Ident(id) => id.name.clone(),
                AExpr::Field { field, .. } => field.name.clone(),
                _ => String::new(),
            };
            let local_name = info.local_call_name.cloned().unwrap_or(name.clone());
            // By the chosen local's own name first: two implicits may share
            // a resolution name [implicit-same-name].
            let local = self
                .lookup(&local_name)
                .map(|b| b.local.clone())
                .or_else(|| self.implicit_locals.get(&local_name).cloned())
                .or_else(|| self.lookup(&name).map(|b| b.local.clone()));
            let Some(local) = local else {
                self.error(span, format!("call through unknown local `{name}`"));
                return unsupported(ty, span, "local call");
            };
            // [fn-effects] the effects the fn value performs are its leading
            // arguments; the contract says how each position is taken.
            let mut a: Vec<Expr> = Vec::new();
            if let Some(Ty::Fn { effects, .. }) = self.lookup(&local.0).map(|b| b.ty.clone()) {
                // [throw] A throw is not a threaded instance: it goes to the
                // delimiter, as a named callee's does.
                for t in crate::threaded_effects(&effects) {
                    a.push(self.effect_instance(&t, span));
                }
            }
            let rest: Vec<Expr> = all_args.iter().map(|x| self.expr(x)).collect();
            let rest = self.mark_consumed_by_contract(rest, &key);
            a.extend(rest);
            return Expr { ty, span, kind: ExprKind::Call { target: FnRef::Local(local), type_args: Vec::new(), args: a } };
        }
        if let Some(k) = info.call_fn.copied() {
            let decl = self.ctx.fn_by_key(k);
            let mut lowered: Vec<Expr> = Vec::new();
            // Effects as values: the instances a callee declaring effects takes.
            if let Some(effs) = info.call_effects.cloned() {
                for t in effs {
                    if matches!(&t, Ty::Named { name, .. } if name == crate::THROW_EFFECT) {
                        continue;
                    }
                    lowered.push(self.effect_instance(&t, span));
                }
            }
            let (args_lowered, prelude) = self.args_for(decl.map(|d| Callee::Fn(k, d)), &all_args, named, span);
            lowered.extend(args_lowered);
            lowered.extend(self.implicit_list(&info.implicit_args.cloned().unwrap_or_default(), named, span));
            let type_args = self.type_args_of(span);
            let call = Expr { ty: ty.clone(), span, kind: ExprKind::Call { target: FnRef::Decl(self.ctx.decl_id(k)), type_args, args: lowered } };
            return if prelude.is_empty() { call } else { self.block_expr(prelude, call, span) };
        }
        // A computed callee: a lambda or fn value expression.
        let target = self.expr(callee);
        let tmp = self.fresh("__callee");
        let id = self.id();
        let tt = target.ty.clone();
        let a: Vec<Expr> = all_args.iter().map(|x| self.expr(x)).collect();
        let call = Expr { ty: ty.clone(), span, kind: ExprKind::Call { target: FnRef::Local(tmp.clone()), type_args: Vec::new(), args: a } };
        self.block_expr(vec![Stmt::Let { id, local: tmp, ty: tt, value: target }], call, span)
    }

    fn mark_consumed_by_contract(&mut self, mut args: Vec<Expr>, key: &(usize, Span)) -> Vec<Expr> {
        if let Some(contract) = self.ctx.checked.fn_value_calls.get(key) {
            for (a, c) in args.iter_mut().zip(contract) {
                if !c.kept {
                    self.consume_if_owned(a);
                }
            }
        }
        args
    }

    /// The arguments of a call against the callee's declared parameters: a
    /// place handed to a `Moved` parameter is consumed; a variadic tail is
    /// The arguments of a call against the callee's declared parameters,
    /// plus any `Stmt`s that must run before them (empty unless a default
    /// value reads an earlier parameter): a place handed to a `Moved`
    /// parameter is consumed; a variadic tail is one list argument.
    fn args_for(&mut self, callee: Option<Callee<'p>>, args: &[&AExpr], named: &[ast::NamedArg], span: Span) -> (Vec<Expr>, Vec<Stmt>) {
        let modes = crate::param_mode::Modes { symbols: self.ctx.symbols, checked: self.ctx.checked, program: self.ctx.program };
        let params: Option<&[ast::Param]> = callee.as_ref().map(|c| match c { Callee::Fn(_, d) => d.params.as_slice(), Callee::Member(m) => m.params.as_slice() });
        let fixed: Vec<&ast::Param> = params.map(|ps| ps.iter().filter(|p| !p.implicit).collect()).unwrap_or_default();
        let variadic_at = fixed.iter().position(|p| p.variadic);
        // [default-scope] A default is lowered in the file that declares it.
        let default_file = match &callee {
            Some(Callee::Fn(k, _)) => Some(k.file),
            Some(Callee::Member(m)) => self.member_file(m),
            None => None,
        };
        let mode_of = |p: &ast::Param| match &callee {
            Some(Callee::Fn(k, _)) => modes.fn_param(Some(*k), p),
            Some(Callee::Member(m)) => modes.member_param(m, p),
            None => modes.default_param(&p.ty, p.variadic),
        };
        let mut out = Vec::new();
        let mut tail: Vec<Expr> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let is_tail = variadic_at.is_some_and(|v| i >= v);
            let mut x = self.expr(a);
            let param = if is_tail { fixed.get(variadic_at.unwrap()) } else { fixed.get(i) };
            if let Some(p) = param {
                let mode = mode_of(p);
                // A variadic tail moves its elements only when the callee's
                // deduction consumes the parameter [fn-variadic].
                let tail_moves = is_tail
                    && match &callee {
                        Some(Callee::Fn(k, _)) => self.ctx.checked.deductions.get(k).and_then(|ds| ds.iter().find(|d| d.param == p.name.name)).is_some_and(|d| !d.kept),
                        Some(Callee::Member(m)) => m.deductions.iter().flatten().any(|d| d.param_name().is_some_and(|n| n.name == p.name.name) && matches!(d.kind, ast::DeductionKind::Moved | ast::DeductionKind::Deferred)),
                        None => false,
                    };
                if (mode == PassMode::Moved && !is_tail) || tail_moves {
                    self.consume_if_owned(&mut x);
                }
            }
            if is_tail {
                if matches!(a, AExpr::Spread { .. }) {
                    // `...xs` alone passes the array itself; among other
                    // elements it spreads into the tail.
                    let lone = args.len() == variadic_at.unwrap() + 1;
                    if lone {
                        out.push(x);
                        return (out, Vec::new());
                    }
                    let xt = x.ty.clone();
                    let xs = x.span;
                    tail.push(Expr { ty: xt, span: xs, kind: ExprKind::Spread { value: Box::new(x) } });
                } else {
                    tail.push(x);
                }
            } else {
                out.push(x);
            }
        }
        // [fn-param-default] Parameters the positional arguments did not
        // reach: fill each from a `name = value` argument if the call gave
        // one, else from the parameter's own default — lowered fresh here,
        // exactly as `struct_lit` lowers a field default at the construction
        // site. Only without a variadic (a variadic absorbs the tail, so no
        // fixed parameter is left unfilled).
        //
        // A default may read an **earlier** parameter. Re-lowering that
        // parameter's own argument expression a second time would be unsound
        // for anything with a side effect or an identity, so instead every
        // parameter any default in this call actually mentions is bound
        // **once**, by name, in a fresh scope (`self.bind_owned`), before
        // the defaults are lowered: an `Ident` inside a default then
        // resolves through the ordinary `lookup` path to a `Read` of the
        // bound local, exactly as it would inside the callee's own body.
        // The resulting `Let`s are returned as a prelude the caller runs
        // ahead of the call (`Lower::block_expr`) — empty, and no scope
        // even pushed, when no default reads an earlier parameter, which is
        // the overwhelmingly common case.
        let mut prelude: Vec<Stmt> = Vec::new();
        if variadic_at.is_none() {
            let referenced: std::collections::HashSet<String> =
                fixed.iter().filter_map(|p| p.default.as_ref()).flat_map(mentioned_idents).collect();
            if !referenced.is_empty() {
                self.push_scope();
                for (i, p) in fixed.iter().enumerate() {
                    if !referenced.contains(p.name.name.as_str()) {
                        if i >= args.len() {
                            // Not referenced by any later default: fill it
                            // the plain way, with nothing to bind.
                            let named_value: Option<&AExpr> = named.iter().find(|a| a.name.name == p.name.name).map(|a| &a.value);
                            let lowered = match (named_value, &p.default) {
                                (Some(v), _) => Some(self.expr(v)),
                                (None, Some(d)) => Some(self.param_default(p, d, span, default_file)),
                                (None, None) => None, // a required gap — a checker error already
                            };
                            if let Some(mut x) = lowered {
                                if mode_of(p) == PassMode::Moved {
                                    self.consume_if_owned(&mut x);
                                }
                                out.push(x);
                            }
                        }
                        continue;
                    }
                    // Referenced by a later default: fill it (positionally,
                    // by name, or by its own default) and bind the result
                    // under its own name, so the defaults after it read the
                    // one bound value rather than re-evaluating it.
                    let value = if i < args.len() {
                        None // already lowered into `out[i]` above
                    } else {
                        let named_value: Option<&AExpr> = named.iter().find(|a| a.name.name == p.name.name).map(|a| &a.value);
                        let lowered = match (named_value, &p.default) {
                            (Some(v), _) => self.expr(v),
                            (None, Some(d)) => self.param_default(p, d, span, default_file),
                            (None, None) => continue, // a required gap — a checker error already
                        };
                        Some(lowered)
                    };
                    let mode = mode_of(p);
                    if let Some(mut x) = value {
                        if mode == PassMode::Moved {
                            self.consume_if_owned(&mut x);
                        }
                        let ty = x.ty.clone();
                        let id = self.id();
                        let local = self.bind_owned(&p.name.name, ty.clone(), mode != PassMode::Lent);
                        prelude.push(Stmt::Let { id, local: local.clone(), ty: ty.clone(), value: x });
                        out.push(Expr { ty, span, kind: ExprKind::Read { place: Place { root: local, steps: Vec::new() }, consume: false } });
                    } else {
                        let ty = out[i].ty.clone();
                        let id = self.id();
                        let local = self.bind_owned(&p.name.name, ty.clone(), mode != PassMode::Lent);
                        let moved = std::mem::replace(&mut out[i], Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: local.clone(), steps: Vec::new() }, consume: false } });
                        prelude.push(Stmt::Let { id, local, ty, value: moved });
                    }
                }
                self.pop_scope();
            } else {
                for (i, p) in fixed.iter().enumerate() {
                    if i < args.len() {
                        continue;
                    }
                    let named_value: Option<&AExpr> = named.iter().find(|a| a.name.name == p.name.name).map(|a| &a.value);
                    let lowered = match (named_value, &p.default) {
                        (Some(v), _) => Some(self.expr(v)),
                        (None, Some(d)) => Some(self.param_default(p, d, span, default_file)),
                        (None, None) => None, // a required gap — a checker error already
                    };
                    if let Some(mut x) = lowered {
                        if mode_of(p) == PassMode::Moved {
                            self.consume_if_owned(&mut x);
                        }
                        out.push(x);
                    }
                }
            }
        }
        if let Some(v) = variadic_at {
            // [fn-variadic] the tail is one array of the declared element type,
            // with the call's type arguments substituted.
            let declared = self.ctx.written_ty(self.file_idx, &fixed[v].ty);
            let generics: Vec<String> = match &callee {
                Some(Callee::Fn(_, d)) => d.generics.iter().map(|g| g.name.clone()).collect(),
                Some(Callee::Member(m)) => m.generics.iter().map(|g| g.name.clone()).collect(),
                None => Vec::new(),
            };
            let type_args = self.type_args_of(span);
            let arr_ty = subst_vars(&declared, &generics, &type_args);
            let arr_ty = match arr_ty.strip_quals() {
                Ty::Array(_) => arr_ty,
                other => Ty::Array(Box::new(other.clone())),
            };
            out.push(Expr { ty: arr_ty, span, kind: ExprKind::Array(tail) });
        }
        (out, prelude)
    }

    pub(crate) fn member_ref_by_name(&mut self, effect: &Ty, name: &str, span: Span) -> Option<MemberRef> {
        let eff_name = match effect.strip_quals() {
            Ty::Named { name, .. } => name.clone(),
            _ => return None,
        };
        let decl = self.ctx.symbols.effects.get(eff_name.as_str()).copied()?;
        let index = match self.ctx.checked.effect_member_calls.get(&self.key(span)) {
            Some(i) => *i,
            None => decl.fns.iter().position(|f| f.name.name == name)?,
        };
        let interface = self.ctx.decl_of_effect(&eff_name)?;
        Some(MemberRef { interface, index })
    }

    // ------------------------------------------------------------ structs --

    fn struct_lit(&mut self, written_ty: Option<&ast::Type>, fields: &[ast::StructLitField], span: Span) -> Expr {
        let mut ty = self.ty_of(span);
        // A literal the checker typed nowhere this file could see (a field
        // default, read at a use in another module) is of its written type.
        if ty.is_unknown() {
            if let Some(w) = written_ty {
                ty = self.ctx.written_ty(self.file_idx, w);
            }
        }
        let decl = match ty.strip_quals() {
            Ty::Named { name, .. } => self.ctx.symbols.structs.get(name.as_str()).copied(),
            _ => None,
        };
        let mut written: Vec<(String, Expr)> = Vec::new();
        let mut spreads: Vec<Expr> = Vec::new();
        for f in fields {
            match &f.kind {
                ast::StructLitFieldKind::Named { name, value } => {
                    let v = self.expr(value);
                    written.push((name.name.clone(), v));
                }
                ast::StructLitFieldKind::Spread(e) => spreads.push(self.expr(e)),
                ast::StructLitFieldKind::InlineFor { .. } => {
                    self.error(f.span, "comptime inline-for survived expansion");
                }
            }
        }
        // Every field in declaration order: written, from a spread, or the default.
        let Some(decl) = decl else {
            // A handler literal, or a struct the symbols do not name: as written.
            return Expr { ty, span, kind: ExprKind::Construct { fields: written } };
        };
        let mut stmts = Vec::new();
        let spread_locals: Vec<Local> = spreads
            .into_iter()
            .map(|s| {
                let tmp = self.fresh("__spread");
                let id = self.id();
                stmts.push(Stmt::Let { id, local: tmp.clone(), ty: s.ty.clone(), value: s });
                tmp
            })
            .collect();
        let mut all = Vec::new();
        for fd in &decl.fields {
            let name = &fd.name.name;
            if let Some(pos) = written.iter().position(|(n, _)| n == name) {
                all.push(written.remove(pos));
            } else if let Some(src) = spread_locals.last() {
                let fty = match &fd.ty { Some(t) => self.ctx.written_ty(self.file_idx, t), None => self.ctx.ty_of(self.file_idx, fd.name.span).unwrap_or(Ty::Unknown) };
                all.push((name.clone(), Expr { ty: fty, span, kind: ExprKind::Read { place: Place { root: src.clone(), steps: vec![Step::Field(name.clone())] }, consume: true } }));
            } else if let Some(d) = &fd.default {
                // [default-predicate] tested where it is applied.
                let slot = fd.ty.as_ref().and_then(|t| self.ctx.written_ty_unerased(t));
                let what = format!("field `{name}`");
                let file = self.struct_file(decl);
                all.push((name.clone(), self.applied_default(d, slot.as_ref(), &what, span, file)));
            }
        }
        all.extend(written);
        let construct = Expr { ty, span, kind: ExprKind::Construct { fields: all } };
        if stmts.is_empty() {
            construct
        } else {
            self.block_expr(stmts, construct, span)
        }
    }

    // ----------------------------------------------------------- lambdas --

    fn lambda(&mut self, params: &[ast::LambdaParam], body: &LambdaBody, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let (ptys, ret, contract) = match ty.strip_quals() {
            Ty::Fn { params, ret, contract, .. } => (params.clone(), (**ret).clone(), contract.clone()),
            _ => (Vec::new(), Ty::Unknown, None),
        };
        self.push_scope();
        let saved_loops = std::mem::take(&mut self.loop_results);
        let saved_env_len = self.effect_env.len();
        let mut ps = Vec::new();
        // [fn-effects] the effects a lambda takes as leading parameters.
        if let Some(effs) = self.ctx.checked.lambda_effects.get(&self.key(span)).cloned() {
            // [throw] A throw is not threaded in [`crate::is_throw_effect`].
            for (i, t) in crate::threaded_effects(&effs).iter().enumerate() {
                let local = Local(format!("__leff{i}"));
                let t = erase(t);
                self.effect_env.push((t.clone(), local.clone()));
                ps.push(Param { local, ty: t, mode: PassMode::Lent, variadic: false, check: None, span });
            }
        }
        for (i, p) in params.iter().enumerate() {
            let pty = ptys.get(i).cloned().or_else(|| p.ty.as_ref().map(|t| self.ctx.written_ty(self.file_idx, t))).unwrap_or(Ty::Unknown);
            let mode = match contract.as_ref().and_then(|c| c.get(i)) {
                Some(c) if !c.kept => PassMode::Moved,
                Some(c) if c.mutable => PassMode::LentMut,
                _ => PassMode::Lent,
            };
            let local = self.bind(&p.name.name, pty.clone());
            ps.push(Param { local, ty: pty, mode, variadic: false, check: None, span: p.span });
        }
        let block = match body {
            LambdaBody::Expr(e) => {
                let v = self.expr(e);
                Block { stmts: Vec::new(), value: Some(Box::new(v)), span: e.span() }
            }
            LambdaBody::Block(b) => self.block(b, !ret.is_none_ty() && !ret.is_unknown()),
            // [terminal-lambda] Filled by the parser before lowering.
            LambdaBody::Terminal => Block { stmts: Vec::new(), value: None, span },
        };
        self.effect_env.truncate(saved_env_len);
        self.loop_results = saved_loops;
        self.pop_scope();
        let captures = self
            .ctx
            .checked
            .lambda_captures
            .get(&self.key(span))
            .map(|cs| {
                cs.iter()
                    .map(|c| Capture { local: self.lookup(&c.name).map(|b| b.local.clone()).unwrap_or(Local(c.name.clone())), consumed: c.consumed, mutable: c.mutable })
                    .collect()
            })
            .unwrap_or_default();
        // A block lambda the checker typed before its `return`s were known
        // answers what its returns answer.
        let (ret, ty) = if ret.is_unknown() {
            let found = returned_ty(&block).unwrap_or_else(Ty::none);
            let ty = match ty.strip_quals() {
                Ty::Fn { params, contract, effects, .. } => Ty::Fn { params: params.clone(), ret: Box::new(found.clone()), contract: contract.clone(), effects: effects.clone() },
                _ => Ty::Fn { params: ps.iter().map(|p| p.ty.clone()).collect(), ret: Box::new(found.clone()), contract: None, effects: Vec::new() },
            };
            (found, ty)
        } else {
            (ret, ty)
        };
        Expr { ty, span, kind: ExprKind::Lambda { params: ps, ret, body: block, captures } }
    }

    // --------------------------------------------------------- narrowing --

    /// `x is T [name]` as a condition.
    fn is_test(&mut self, subject: &AExpr, binding: Option<&ast::Ident>, span: Span) -> Expr {
        let bool_ty = Ty::named("Bool");
        if let Some(checks) = self.ctx.checked.predicate_tests.get(&self.key(span)).cloned() {
            // [qual-predicate] a conjunction of `qualifies` calls.
            let mut acc: Option<Expr> = None;
            for check in checks {
                let subj = self.expr(subject);
                let call = self.qualifies_call(&check, subj, span);
                acc = Some(match acc {
                    None => call,
                    Some(prev) => Expr { ty: bool_ty.clone(), span, kind: ExprKind::Op { op: Op::And, args: vec![prev, call] } },
                });
            }
            if let (Some(b), Some(place)) = (binding, self.stored_place_of(subject)) {
                // The binding is the subject under a claim: an alias.
                let ty = self.ty_of(b.span);
                let local = self.bind(&b.name, ty.clone());
                let id = self.id();
                self.pending_after_test.push(Stmt::Let { id, local, ty: ty.clone(), value: Expr { ty, span, kind: ExprKind::Read { place, consume: false } } });
            }
            return acc.unwrap_or(Expr { ty: bool_ty, span, kind: ExprKind::Bool(true) });
        }
        let Some(test) = self.ctx.checked.is_tests.get(&self.key(span)).cloned() else {
            // A test the checker decided statically.
            return Expr { ty: bool_ty, span, kind: ExprKind::Bool(true) };
        };
        let mut s = self.expr_subject(subject);
        let id = self.id();
        let arm_ty = self.narrowed_ty_of_test(&s.ty, &test);
        let mut place = self.stored_place_of(subject);
        if place.is_none() && binding.is_some() {
            // [is-bind-once] a non-place subject is evaluated once, into a
            // temporary both the test and the binding read.
            let tmp = self.fresh("__subject");
            let lid = self.id();
            let sty = s.ty.clone();
            self.pending.push(Stmt::Let { id: lid, local: tmp.clone(), ty: sty.clone(), value: s });
            s = Expr { ty: sty, span: subject.span(), kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume: false } };
            place = Some(Place { root: tmp, steps: Vec::new() });
        }
        if !self.in_condition {
            // A test read as a value (`expect(x is T, …)`) narrows nothing
            // afterwards: the flow does not branch on it.
        } else if let Some(b) = binding {
            let from = place.clone().unwrap_or(Place { root: Local("<subject>".into()), steps: Vec::new() });
            let ty = self.ty_of(b.span).or_unknown(arm_ty.clone()).unwrap_or(Ty::Unknown);
            let local = Local(b.name.clone());
            self.rebind(&b.name, local.clone(), ty.clone());
            let nid = self.id();
            let from_ty = s.ty.clone();
            self.pending_after_test.push(Stmt::Narrow { id: nid, local, ty, from, from_ty, because: Justification::Test { test: id } });
        } else if let (Some(place), AExpr::Ident(name)) = (place, subject) {
            // A narrowing to `None` binds nothing worth reading.
            if let Some(t) = arm_ty.filter(|t| !t.is_none_ty()) {
                let from_ty = s.ty.clone();
                let narrow = self.narrow_in_arm(&name.name, place, from_ty, t, Justification::Test { test: id });
                self.pending_after_test.push(narrow);
            }
        }
        Expr { ty: bool_ty, span, kind: ExprKind::Test { id, subject: Box::new(s), test: arm_test(&test) } }
    }

    /// The subject of a test, read as stored (not through its narrowing).
    fn expr_subject(&mut self, subject: &AExpr) -> Expr {
        let span = subject.span();
        if let AExpr::Ident(id) = subject {
            if let Some(b) = self.lookup(&id.name).cloned() {
                let ty = self.ctx.checked.repr_ty.get(&self.key(span)).map(erase).unwrap_or(b.ty.clone());
                return Expr { ty, span, kind: ExprKind::Read { place: Place { root: b.local, steps: Vec::new() }, consume: false } };
            }
        }
        // A projection, read as stored too (its declared type), unless an
        // enclosing safe call already bound it.
        if matches!(subject, AExpr::Field { .. } | AExpr::TupleIndex { .. }) {
            if let Some(place) = self.stored_place_of(subject) {
                if let Some(alias) = self.place_alias(&place) {
                    return alias;
                }
                if let Some(declared) = self.ctx.checked.repr_ty.get(&self.key(span)).map(erase) {
                    return Expr { ty: declared, span, kind: ExprKind::Read { place, consume: false } };
                }
            }
        }
        self.expr(subject)
    }

    /// `place` with its longest prefix an enclosing safe call bound replaced
    /// by that binding.
    fn through_alias(&self, place: Place) -> Place {
        for n in (1..=place.steps.len()).rev() {
            let prefix = Place { root: place.root.clone(), steps: place.steps[..n].to_vec() };
            if let Some(Expr { kind: ExprKind::Read { place: alias, .. }, .. }) = self.place_alias(&prefix) {
                return Place { root: alias.root, steps: place.steps[n..].to_vec() };
            }
        }
        place
    }

    /// [ir-narrow] The narrowing binding an enclosing safe call made for a
    /// projection place (`a.b?.c` binds `a.b`), as a read.
    pub(crate) fn place_alias(&self, place: &Place) -> Option<Expr> {
        let scoped = self.scoped_aliases.iter().rev().map(|(_, p, l, t)| (p, l, t));
        scoped.chain(self.place_aliases.iter().rev().map(|(p, l, t)| (p, l, t))).find_map(|(p, l, t)| {
            let same = p.root == place.root && p.steps.len() == place.steps.len() && p.steps.iter().zip(&place.steps).all(|(a, b)| match (a, b) {
                (Step::Field(x), Step::Field(y)) => x == y,
                (Step::Tuple(x), Step::Tuple(y)) => x == y,
                _ => false,
            });
            same.then(|| Expr { ty: t.clone(), span: Span::default(), kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false } })
        })
    }

    /// The type a successful test narrows the subject to.
    pub(crate) fn narrowed_ty_of_test(&self, subject: &Ty, test: &UnionTest) -> Option<Ty> {
        if test.match_none {
            return Some(Ty::none());
        }
        let arms: Vec<&Ty> = subject.strip_quals().value_arms();
        if test.size <= 1 && !subject.strip_quals().is_wrapper_union() {
            return subject.strip_quals().value_arms().first().map(|t| (*t).clone());
        }
        let picked: Vec<Ty> = test.arms.iter().filter_map(|i| arms.get(*i).map(|t| (*t).clone())).collect();
        match picked.len() {
            0 => None,
            1 => Some(picked[0].clone()),
            _ => Some(Ty::Union(picked)),
        }
    }

    fn qualifies_call(&mut self, check: &crate::check::PredicateCheck, subject: Expr, span: Span) -> Expr {
        self.qual_fn_call(check, subject, span, "qualifies", Ty::named("Bool"))
    }

    /// [qual-whynot] A call to one of a predicate qualifier's own fns —
    /// `qualifies` — with the subject and the check's slot values.
    fn qual_fn_call(&mut self, check: &crate::check::PredicateCheck, subject: Expr, span: Span, member: &str, ret: Ty) -> Expr {
        let bool_ty = ret;
        let Some(target) = self.qual_fn_ref(&check.name, &subject.ty, member) else {
            self.error(span, format!("qualifier `{}` has no `{member}`", check.name));
            return unsupported(bool_ty, span, member.to_string());
        };
        // [is-qualifies-effects] the effects a `qualifies` performs lead.
        let mut args: Vec<Expr> = Vec::new();
        let effects: Vec<Ty> = self
            .ctx
            .program
            .files
            .iter()
            .position(|f| f.module == target.module)
            .and_then(|fi| match self.ctx.program.modules[fi].items.get(target.item) {
                Some(ast::Item::Qualifier(q)) => q.fns.get((target.sub as usize).wrapping_sub(1)).map(|f| {
                    f.effects
                        .iter()
                        .flatten()
                        .filter_map(|e| match e {
                            ast::EffectRef::Effect(r) | ast::EffectRef::AnyEffect(r) => Some(Ty::Named { name: self.ctx.checked.written_key(r).to_string(), args: r.args.iter().map(|a| self.ctx.written_ty(fi, a)).collect() }),
                            _ => None,
                        })
                        .collect()
                }),
                _ => None,
            })
            .unwrap_or_default();
        for t in &effects {
            args.push(self.effect_instance(t, span));
        }
        args.push(subject);
        for a in &check.args {
            args.push(self.slot_arg(a, Ty::Unknown, span));
        }
        if let Some(at) = check.implicits_at {
            args.extend(self.implicit_args_at(at));
        }
        Expr { ty: bool_ty, span, kind: ExprKind::Call { target: FnRef::Decl(target), type_args: Vec::new(), args } }
    }

    /// [qual-depend] A predicate check's slot value: the place it names,
    /// read (typed `ty`), or the literal it is.
    fn slot_arg(&mut self, a: &crate::check::PredicateArg, ty: Ty, span: Span) -> Expr {
        let parts: Vec<&str> = a.path.split('.').collect();
        match self.lookup(parts[0]).cloned() {
            Some(b) => {
                let steps = parts[1..].iter().map(|s| Step::Field(s.to_string())).collect();
                Expr { ty, span, kind: ExprKind::Read { place: Place { root: b.local, steps }, consume: false } }
            }
            // [qual-value-args] a literal argument, as spelled.
            None => match literal_arg(&a.path) {
                Some(e) => Expr { span, ..e },
                None => unsupported(Ty::Unknown, span, format!("qualifier argument `{}`", a.path)),
            },
        }
    }

    /// [default-predicate] A default applied to the slot it fills: lowered
    /// fresh, as every default is, and — when the slot carries a predicate
    /// qualifier the default's own type lacks — bound once and tested there,
    /// trapping when the test fails, as `x!` traps on `None`. The checker let
    /// such a default through only for predicate qualifiers with constant
    /// arguments, no implicits and no effects ([default-fits]), which is what
    /// makes the test buildable at any construction or call site.
    /// [default-predicate] A parameter's default, applied at a call.
    fn param_default(&mut self, p: &ast::Param, d: &AExpr, span: Span, file: Option<usize>) -> Expr {
        let what = format!("parameter `{}`", p.name.name);
        self.applied_default(d, self.ctx.written_ty_unerased(&p.ty).map(|t| t.strip_spread()).as_ref(), &what, span, file)
    }

    /// [comptime-default] `field.default` as the expansion placed it: the
    /// field's declared default, applied like any other — lowered in the
    /// declaring file, and tested there against a predicate qualifier.
    fn field_default(&mut self, owner: &ast::Type, field: &ast::Ident, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let decl = match self.ctx.written_ty(self.file_idx, owner).strip_quals() {
            Ty::Named { name, .. } => self.ctx.symbols.structs.get(name.as_str()).copied(),
            _ => None,
        };
        let Some(fd) = decl.and_then(|d| d.fields.iter().find(|f| f.name.name == field.name)) else {
            return unsupported(ty, span, format!("the default of field `{}`: its struct is not known here", field.name));
        };
        let Some(d) = &fd.default else {
            return unsupported(ty, span, format!("field `{}` declares no default", field.name));
        };
        let slot = fd.ty.as_ref().and_then(|t| self.ctx.written_ty_unerased(t));
        let file = decl.and_then(|d| self.struct_file(d));
        self.applied_default(d, slot.as_ref(), &format!("field `{}`", field.name), span, file)
    }

    /// [default-scope] The file that declares `decl`.
    fn struct_file(&self, decl: &ast::StructDecl) -> Option<usize> {
        self.ctx
            .program
            .modules
            .iter()
            .position(|m| m.items.iter().any(|i| matches!(i, ast::Item::Struct(s) if std::ptr::eq(s, decl))))
    }

    /// [default-scope] The file that declares the effect member `m`.
    fn member_file(&self, m: &ast::FnDecl) -> Option<usize> {
        self.ctx.program.modules.iter().position(|module| {
            module.items.iter().any(|i| matches!(i, ast::Item::Effect(e) if e.fns.iter().any(|f| std::ptr::eq(f, m))))
        })
    }

    ///
    /// `slot` is the slot's type **before erasure** (IR types keep only `Mut`
    /// and tags), so a default's own claims are not visible here: every
    /// predicate qualifier of the slot is tested, which is redundant (never
    /// wrong) for a default that already carried the claim.
    ///
    /// [default-scope] `file` is the file that declares the default: it was
    /// checked there, so the checker's facts about it are keyed by that
    /// file's index, and it is lowered under that index — a default that
    /// calls a private fn of its module, applied in another module, found
    /// nothing under the using file's (2026-10-20: "unresolved identifier").
    /// Only the value is lowered there; the test around it is the site's.
    pub(crate) fn applied_default(&mut self, default: &AExpr, slot: Option<&Ty>, what: &str, span: Span, file: Option<usize>) -> Expr {
        let saved = self.file_idx;
        if let Some(f) = file {
            self.file_idx = f;
        }
        let value = self.expr(default);
        self.file_idx = saved;
        let Some(slot) = slot else { return value };
        let checks: Vec<crate::types::Qual> = slot
            .quals()
            .iter()
            .filter(|q| q.args.iter().all(|a| matches!(a, Ty::Const(_))))
            .filter(|q| self.qualifies_ref(&q.name, slot).is_some())
            .cloned()
            .collect();
        if checks.is_empty() {
            return value;
        }
        let vty = value.ty.clone();
        let tmp = self.fresh("__default");
        let id = self.id();
        let mut stmts = vec![Stmt::Let { id, local: tmp.clone(), ty: vty.clone(), value }];
        let read = |consume: bool| Expr {
            ty: vty.clone(),
            span,
            kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume },
        };
        for q in &checks {
            let args = q
                .args
                .iter()
                .map(|a| match a {
                    Ty::Const(c) => crate::check::PredicateArg { path: c.encode(), copy: true },
                    other => crate::check::PredicateArg { path: other.to_string(), copy: true },
                })
                .collect();
            let check = crate::check::PredicateCheck { name: q.name.clone(), args, implicits_at: None };
            let cond = self.qualifies_call(&check, read(false), span);
            let shown = Ty::Qualified { quals: vec![q.clone()], base: Box::new(slot.strip_quals().clone()) };
            let message = Expr {
                ty: Ty::named("Str"),
                span,
                kind: ExprKind::Str(format!("the default for {what} is not `{shown}`")),
            };
            stmts.push(Stmt::Expr(Expr {
                ty: Ty::none(),
                span,
                kind: ExprKind::Assert { cond: Box::new(cond), message: Some(Box::new(message)), at: self.location(span) },
            }));
        }
        let value = read(true);
        self.block_expr(stmts, value, span)
    }

    fn qualifies_ref(&self, name: &str, subject: &Ty) -> Option<DeclId> {
        self.qual_fn_ref(name, subject, "qualifies")
    }

    /// [qual-whynot] The fn `member` of the qualifier `name` (the declaration
    /// for `subject` when the name is overloaded).
    fn qual_fn_ref(&self, name: &str, subject: &Ty, member: &str) -> Option<DeclId> {
        let candidates = self.ctx.symbols.qualifiers.get(name)?;
        let decl: &ast::QualifierDecl = match candidates.as_slice() {
            [] => return None,
            [one] => one,
            many => {
                let base = match subject.strip_quals() {
                    Ty::Named { name, .. } => name.clone(),
                    Ty::Array(_) => "[]".to_string(),
                    _ => return None,
                };
                many.iter().copied().find(|d| crate::refine::of_base(&d.of, &d.generics).is_some_and(|of| of == base)).or_else(|| candidates.first().copied())?
            }
        };
        let fi = decl.fns.iter().position(|f| f.name.name == member)?;
        for (file_idx, m) in self.ctx.program.modules.iter().enumerate() {
            if let Some(item) = m.items.iter().position(|i| matches!(i, ast::Item::Qualifier(q) if std::ptr::eq(q, decl))) {
                return Some(DeclId { module: self.ctx.program.files[file_idx].module.clone(), item, sub: fi as u16 + 1 });
            }
        }
        None
    }

    /// [qual-whynot] `whynot?(x is Q(…))`: `None` when the value qualifies,
    /// else the message of the first check it fails, in the order written.
    /// The subject is read per check when it is a place, and evaluated once
    /// into a temporary otherwise; a test with no predicate to call (decided
    /// statically) is `None`.
    fn why_not(&mut self, test: &AExpr, span: Span) -> Expr {
        let opt = Ty::union_of(vec![Ty::named("Str"), Ty::none()]);
        let AExpr::Is { subject, span: is_span, .. } = test else {
            return unsupported(opt, span, "`whynot?` of a non-`is` test");
        };
        let none = Expr { ty: opt.clone(), span, kind: ExprKind::MakeNone };
        let Some(checks) = self.ctx.checked.predicate_tests.get(&self.key(*is_span)).cloned() else {
            return none;
        };
        if checks.is_empty() {
            return none;
        }
        let mut bind = Vec::new();
        let tmp = if self.stored_place_of(subject).is_some() {
            None
        } else {
            let value = self.expr(subject);
            let local = self.fresh("__subject");
            let id = self.id();
            let ty = value.ty.clone();
            bind.push(Stmt::Let { id, local: local.clone(), ty: ty.clone(), value });
            Some((local, ty))
        };
        let mut arms = Vec::new();
        for check in &checks {
            let read = |this: &mut Self| match &tmp {
                Some((local, ty)) => Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: local.clone(), steps: Vec::new() }, consume: false } },
                None => this.expr(subject),
            };
            let subj = read(self);
            let call = self.qualifies_call(check, subj, *is_span);
            let failed = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::Not, args: vec![call] } };
            let subj = read(self);
            let message = self.whynot_message(check, subj, subject.span(), span);
            let present = Expr { ty: opt.clone(), span, kind: ExprKind::Present { value: Box::new(message) } };
            arms.push((failed, Block { stmts: Vec::new(), value: Some(Box::new(present)), span }));
        }
        let id = self.id();
        let br = Expr { ty: opt, span, kind: ExprKind::Branch { id, arms, otherwise: Some(Block { stmts: Vec::new(), value: Some(Box::new(none)), span }) } };
        if bind.is_empty() {
            br
        } else {
            self.block_expr(bind, br, span)
        }
    }

    /// [qual-whynot] The message a failed `assert!(x is Q(…))` gives when it
    /// writes none: the failed check's, for a test over a place (read again,
    /// only on failure). With several checks, the first that fails says it.
    fn assert_whynot(&mut self, cond: &AExpr) -> Option<Expr> {
        let AExpr::Is { subject, span, .. } = cond else { return None };
        let checks = self.ctx.checked.predicate_tests.get(&self.key(*span)).cloned()?;
        self.stored_place_of(subject)?;
        let (last, rest) = checks.split_last()?;
        let mut arms = Vec::new();
        for check in rest {
            let subj = self.expr(subject);
            let call = self.qualifies_call(check, subj, *span);
            let failed = Expr { ty: Ty::named("Bool"), span: *span, kind: ExprKind::Op { op: Op::Not, args: vec![call] } };
            let subj = self.expr(subject);
            let message = self.whynot_message(check, subj, subject.span(), *span);
            arms.push((failed, Block { stmts: Vec::new(), value: Some(Box::new(message)), span: *span }));
        }
        let subj = self.expr(subject);
        let last = self.whynot_message(last, subj, subject.span(), *span);
        if arms.is_empty() {
            return Some(last);
        }
        let id = self.id();
        Some(Expr { ty: Ty::named("Str"), span: *span, kind: ExprKind::Branch { id, arms, otherwise: Some(Block { stmts: Vec::new(), value: Some(Box::new(last)), span: *span }) } })
    }

    /// [qual-whynot] What a value that failed `check` is told: the
    /// qualifier's `@whynot` template, its `[name]`s filled with the subject
    /// (by the text form the checker recorded at `subject_span`, else "the
    /// value") and the check's slot values; with no `@whynot`, "`<subject>`
    /// is not Q(args)".
    fn whynot_message(&mut self, check: &crate::check::PredicateCheck, subject: Expr, subject_span: Span, span: Span) -> Expr {
        use salvo_syntax::doc::{self, TemplatePart};
        let str_ty = Ty::named("Str");
        let text = |s: String| Expr { ty: Ty::named("Str"), span, kind: ExprKind::Str(s) };
        let Some(target) = self.qual_fn_ref(&check.name, &subject.ty, "qualifies") else {
            return text(format!("the value is not {}", check.name));
        };
        let Some((fi, qf)) = self.decl_fn(&target) else {
            return text(format!("the value is not {}", check.name));
        };
        let params: Vec<(String, ast::Type)> = qf.params.iter().filter(|p| !p.implicit).map(|p| (p.name.name.clone(), p.ty.clone())).collect();
        let template = doc::split(&qf.docs).tag("whynot").map(|t| doc::template(&t.text));
        let subject_text = self.text_of(subject, subject_span).unwrap_or_else(|| text("the value".to_string()));
        let parts = match template {
            Some(parts) => parts,
            None => {
                let shown = if check.args.is_empty() {
                    check.name.clone()
                } else {
                    format!("{}({})", check.name, check.args.iter().map(|a| a.path.clone()).collect::<Vec<_>>().join(", "))
                };
                return Expr { ty: str_ty, span, kind: ExprKind::Concat(vec![subject_text, text(format!(" is not {shown}"))]) };
            }
        };
        let mut out: Vec<Expr> = Vec::new();
        for part in parts {
            match part {
                TemplatePart::Text(t) => out.push(text(t)),
                TemplatePart::Param(name) => match params.iter().position(|(n, _)| *n == name) {
                    Some(0) => out.push(subject_text.clone()),
                    Some(i) if i - 1 < check.args.len() => {
                        let a = &check.args[i - 1];
                        // A constant is shown as it reads; `Str` without its quotes.
                        let shown = match literal_arg(&a.path).map(|e| e.kind) {
                            Some(ExprKind::Str(s)) => Some(s),
                            Some(ExprKind::Int(v)) | Some(ExprKind::Long(v)) => Some(v.to_string()),
                            Some(ExprKind::Bool(b)) => Some(b.to_string()),
                            Some(ExprKind::Char(c)) => Some(c.to_string()),
                            _ => None,
                        };
                        match shown {
                            Some(s) => out.push(text(s)),
                            None => {
                                let ty = self.ctx.written_ty(fi, &params[i].1);
                                let value = self.slot_arg(a, ty, span);
                                match self.native_text(value, span) {
                                    Some(t) => out.push(t),
                                    None => out.push(text(a.path.clone())),
                                }
                            }
                        }
                    }
                    _ => out.push(text(format!("[{name}]"))),
                },
            }
        }
        // Adjacent text is one literal: `" is not in 1..65535"`.
        let mut merged: Vec<Expr> = Vec::new();
        for e in out {
            match (merged.last_mut().map(|l| &mut l.kind), &e.kind) {
                (Some(ExprKind::Str(prev)), ExprKind::Str(next)) => prev.push_str(next),
                _ => merged.push(e),
            }
        }
        match merged.len() {
            1 if matches!(merged[0].ty.strip_quals(), Ty::Named { name, .. } if name == "Str") => merged.pop().unwrap(),
            _ => Expr { ty: str_ty, span, kind: ExprKind::Concat(merged) },
        }
    }

    /// The file index and fn declaration a qualifier member's id names.
    fn decl_fn(&self, target: &DeclId) -> Option<(usize, ast::FnDecl)> {
        let fi = self.ctx.program.files.iter().position(|f| f.module == target.module)?;
        match self.ctx.program.modules[fi].items.get(target.item) {
            Some(ast::Item::Qualifier(q)) => q.fns.get((target.sub as usize).wrapping_sub(1)).map(|f| (fi, f.clone())),
            _ => None,
        }
    }

    /// [safe-nav] `base?.link`: switch on the receiver; `None` short-circuits
    /// the whole expression to `None`; otherwise the link is performed on the
    /// receiver narrowed to its non-`None` arm and lifted back into an
    /// optional. The present arm's `Narrow` carries `Justification::SafeNav`,
    /// the hint a backend reads to reconstruct a native optional chain. The
    /// shape mirrors `!` (bind once, switch `None`/else), differing only in
    /// the `None` arm (a value, not a trap) and the result being optional.
    fn safe_nav(&mut self, base: &AExpr, access: &ast::SafeAccess, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let info = self.ctx.checked.safe_navs.get(&self.key(span)).cloned();
        let inner = info.as_ref().map(|i| i.inner.clone()).unwrap_or(Ty::Unknown);
        let already_opt = info.as_ref().map(|i| i.already_opt).unwrap_or(false);
        let optional = info.as_ref().map(|i| i.optional).unwrap_or(true);
        let rest = info.as_ref().map(|i| i.rest.clone()).unwrap_or_else(Ty::none);
        // Bind the receiver once, read as stored.
        let s = self.expr_subject(base);
        let subj_ty = s.ty.clone();
        let id = self.id();
        let safe = self.fresh("__safe");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: safe.clone(), ty: subj_ty.clone(), value: s };
        // The present arm: narrow the receiver to its first arm, perform the
        // link on it, and lift the result into the result union unless it is
        // already there ([safe-nav], the `Option<Option<T>>`/arm-collapse
        // guard).
        let nav = self.fresh("__nav");
        let nid = self.id();
        let narrow = Stmt::Narrow {
            id: nid,
            local: nav.clone(),
            ty: inner.clone(),
            from: Place { root: safe.clone(), steps: Vec::new() },
            from_ty: subj_ty.clone(),
            because: Justification::SafeNav { switch: id },
        };
        let link = self.safe_link(&nav, &inner, access, span);
        if optional {
            // [kt-safe-nav] [rs-safe-nav] The `T | None` shape the backends
            // reconstruct as a native optional chain: a `None` arm yielding
            // `MakeNone`, an `Else` arm with the link lifted by `Present`.
            let present_value = if already_opt {
                link
            } else {
                Expr { ty: ty.clone(), span, kind: ExprKind::Present { value: Box::new(link) } }
            };
            let present = Block { stmts: vec![narrow], value: Some(Box::new(present_value)), span };
            let absent = Block {
                stmts: Vec::new(),
                value: Some(Box::new(Expr { ty: ty.clone(), span, kind: ExprKind::MakeNone })),
                span,
            };
            let sw = Expr {
                ty: ty.clone(),
                span,
                kind: ExprKind::Switch {
                    id,
                    subject: Box::new(Expr { ty: subj_ty, span, kind: ExprKind::Read { place: Place { root: safe, steps: Vec::new() }, consume: false } }),
                    arms: vec![
                        SwitchArm { test: ArmTest::None, body: absent },
                        SwitchArm { test: ArmTest::Else, body: present },
                    ],
                },
            };
            return self.block_expr(vec![bind], sw, span);
        }
        // General union: the present arm rewraps the link's result into the
        // result union; the `Else` arm narrows the receiver to the arms left
        // behind and rewraps those into the result. No native reconstruction.
        let present_value = self.rewrap_into(link, &ty, span);
        let present = Block { stmts: vec![narrow], value: Some(Box::new(present_value)), span };
        let rest_local = self.fresh("__rest");
        let rrid = self.id();
        let rest_narrow = Stmt::Narrow {
            id: rrid,
            local: rest_local.clone(),
            ty: rest.clone(),
            from: Place { root: safe.clone(), steps: Vec::new() },
            from_ty: subj_ty.clone(),
            because: Justification::Arm { switch: id, arm: 1 },
        };
        let rest_read = Expr { ty: rest.clone(), span, kind: ExprKind::Read { place: Place { root: rest_local, steps: Vec::new() }, consume: true } };
        let rest_value = self.rewrap_into(rest_read, &ty, span);
        let absent = Block { stmts: vec![rest_narrow], value: Some(Box::new(rest_value)), span };
        // Test the subject's first value arm directly (arm 0).
        let test = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id: self.id(), subject: Box::new(Expr { ty: subj_ty.clone(), span, kind: ExprKind::Read { place: Place { root: safe.clone(), steps: Vec::new() }, consume: false } }), test: ArmTest::Arm(0) } };
        let br = Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::Branch {
                id,
                arms: vec![(test, present)],
                otherwise: Some(absent),
            },
        };
        self.block_expr(vec![bind], br, span)
    }

    /// [safe-nav] Rewrap a value into the result union `ty` when its type is
    /// narrower, so a picked first-arm result or a passed-through remainder
    /// takes the whole union's representation [rewrap]. A `None`-typed value
    /// into an optional result is the absent arm (`MakeNone`), not a rewrap. A
    /// value already at the union type is handed back untouched.
    fn rewrap_into(&mut self, value: Expr, ty: &Ty, span: Span) -> Expr {
        if &value.ty == ty || ty.is_unknown() {
            return value;
        }
        if value.ty.is_none_ty() && ty.has_none_arm() {
            return Expr { ty: ty.clone(), span, kind: ExprKind::MakeNone };
        }
        match crate::types::read_arm_map(&value.ty, ty) {
            Some(_) if matches!(ty, Ty::Union(_)) => Expr {
                ty: ty.clone(),
                span,
                kind: ExprKind::Rewrap { from: value.ty.clone(), value: Box::new(value) },
            },
            _ => value,
        }
    }

    /// [safe-nav] The link performed on the narrowed receiver local `nav`
    /// (type `inner`): a field read, or a dot call built from the checker's
    /// recorded resolution with `nav` as the receiver argument.
    fn safe_link(&mut self, nav: &Local, inner: &Ty, access: &ast::SafeAccess, span: Span) -> Expr {
        match access {
            ast::SafeAccess::Field(field) => {
                // The field's type on the (non-`None`) receiver struct.
                let fty = match inner.strip_quals() {
                    Ty::Named { name, .. } => self
                        .ctx
                        .symbols
                        .structs
                        .get(name.as_str())
                        .and_then(|sd| sd.fields.iter().find(|f| f.name.name == field.name))
                        .map(|f| match &f.ty {
                            Some(t) => self.ctx.written_ty(self.file_idx, t),
                            None => self.ctx.ty_of(self.file_idx, f.name.span).unwrap_or(Ty::Unknown),
                        }),
                    _ => None,
                }
                .unwrap_or_else(|| self.ty_of(span));
                Expr { ty: fty, span, kind: ExprKind::Read { place: Place { root: nav.clone(), steps: vec![Step::Field(field.name.clone())] }, consume: false } }
            }
            ast::SafeAccess::Call { args, .. } => self.safe_call(nav, inner, args, span),
        }
    }

    /// [safe-nav] A `base?.m(args)` call, built from `call_info` recorded under
    /// the `SafeNav` span, with the narrowed receiver `nav` as argument zero.
    /// Handles the two resolutions a dot call reaches — a declared fn and an
    /// effect member — and refuses anything else rather than emit wrong code
    /// ([backend-never-wrong]).
    fn safe_call(&mut self, nav: &Local, inner: &Ty, args: &[AExpr], span: Span) -> Expr {
        let key = self.key(span);
        let ty = self.ty_of(span);
        let recv = Expr { ty: inner.clone(), span, kind: ExprKind::Read { place: Place { root: nav.clone(), steps: Vec::new() }, consume: false } };
        let info = self.ctx.checked.call_info(key);
        // Effect-member dot call.
        if let Some(mc) = info.member_call.cloned() {
            let Some(interface) = self.ctx.decl_of_effect(&mc.effect) else {
                return unsupported(ty, span, "safe-nav member call");
            };
            let instance_ty = info.effect_call.cloned().unwrap_or_else(|| Ty::named(mc.effect.clone()));
            let instance = self.effect_instance(&instance_ty, span);
            let mut call_args = vec![recv];
            call_args.extend(args.iter().map(|a| self.expr(a)));
            let type_args = self.type_args_of(span);
            return Expr { ty, span, kind: ExprKind::MemberCall { instance: Box::new(instance), member: MemberRef { interface, index: mc.index }, type_args, args: call_args } };
        }
        // Declared free fn (the common dot-call case).
        if let Some(k) = info.call_fn.copied() {
            let mut call_args: Vec<Expr> = Vec::new();
            if let Some(effs) = info.call_effects.cloned() {
                for t in effs {
                    if matches!(&t, Ty::Named { name, .. } if name == crate::THROW_EFFECT) {
                        continue;
                    }
                    call_args.push(self.effect_instance(&t, span));
                }
            }
            call_args.push(recv);
            call_args.extend(args.iter().map(|a| self.expr(a)));
            let type_args = self.type_args_of(span);
            return Expr { ty, span, kind: ExprKind::Call { target: FnRef::Decl(self.ctx.decl_id(k)), type_args, args: call_args } };
        }
        unsupported(ty, span, "safe-nav call")
    }

    /// [expr-pipe] `lhs |> rhs`: bind the left side to a temporary, make it the
    /// placeholder `_`, lower the right side (which reads `_`), and return the
    /// right side's value with the binding in front — a block, like the elvis.
    /// The temp owns the piped value; `_` reads it non-consuming, so the
    /// ownership rules treat it exactly as a reused local (the right side may
    /// read it more than once only when a plain local could).
    fn pipe(&mut self, lhs: &AExpr, rhs: &AExpr, span: Span) -> Expr {
        let l = self.expr(lhs);
        let piped_ty = l.ty.clone();
        let tmp = self.fresh("__piped");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: piped_ty, value: l };
        let saved = self.placeholder.replace(tmp);
        let r = self.expr(rhs);
        self.placeholder = saved;
        self.block_expr(vec![bind], r, span)
    }

    /// [expr-safe-pipe] `lhs ?> rhs` (user decision 2026-10-14): the null-safe
    /// pipe over any union. Bind `lhs` once and branch on its first arm: the
    /// present arm narrows to that arm, binds `_`, runs `rhs`, and rewraps the
    /// result into the union; the else arm narrows to the arms left behind and
    /// passes them through. A diverging `rhs` leaves only the pass-through
    /// arms. The `T | None` shape is the special case that recovers `?.`'s
    /// optional result.
    fn safe_pipe(&mut self, lhs: &AExpr, rhs: &AExpr, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let l = self.expr(lhs);
        let subj_ty = l.ty.clone();
        let declared: Vec<Ty> = match subj_ty.strip_quals() {
            Ty::Union(arms) => arms.clone(),
            other => vec![other.clone()],
        };
        let inner = declared.first().cloned().unwrap_or(Ty::Unknown);
        let rest = Ty::union_of(declared.iter().skip(1).cloned().collect());
        // The `T | None` shape: first arm the value, rest just `None`. The
        // result is an optional and the else arm is `MakeNone` — the same
        // shape `?.` uses, so a diverging rhs and the optional result both
        // behave as the old `?>`.
        let optional = subj_ty.has_none_arm() && declared.len() == 2 && rest.is_none_ty();
        let id = self.id();
        let tmp = self.fresh("__safepiped");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: l };
        // Present arm: narrow to the first arm, bind `_` to it, run the right
        // side. `_` reads non-consuming, like a plain pipe.
        let nav = self.fresh("__safenav");
        let nid = self.id();
        let narrow = Stmt::Narrow {
            id: nid,
            local: nav.clone(),
            ty: inner.clone(),
            from: Place { root: tmp.clone(), steps: Vec::new() },
            from_ty: subj_ty.clone(),
            because: Justification::Arm { switch: id, arm: 0 },
        };
        let saved = self.placeholder.replace(nav);
        let r = self.expr(rhs);
        self.placeholder = saved;
        if optional {
            // Lift the right side's value into the optional result unless it
            // is already one ([safe-nav]'s `Option<Option<T>>` guard) or it
            // diverged.
            let present_value = if matches!(r.ty, Ty::Never) || r.ty.has_none_arm() {
                r
            } else {
                Expr { ty: ty.clone(), span, kind: ExprKind::Present { value: Box::new(r) } }
            };
            let present = Block { stmts: vec![narrow], value: Some(Box::new(present_value)), span };
            let absent = Block {
                stmts: Vec::new(),
                value: Some(Box::new(Expr { ty: ty.clone(), span, kind: ExprKind::MakeNone })),
                span,
            };
            let sw = Expr {
                ty: ty.clone(),
                span,
                kind: ExprKind::Switch {
                    id,
                    subject: Box::new(Expr { ty: subj_ty, span, kind: ExprKind::Read { place: Place { root: tmp, steps: Vec::new() }, consume: false } }),
                    arms: vec![
                        SwitchArm { test: ArmTest::None, body: absent },
                        SwitchArm { test: ArmTest::Else, body: present },
                    ],
                },
            };
            return self.block_expr(vec![bind], sw, span);
        }
        // General union: the present arm rewraps the right side's result into
        // the result union; the else arm passes the remaining arms through.
        let present_value = self.rewrap_into(r, &ty, span);
        let present = Block { stmts: vec![narrow], value: Some(Box::new(present_value)), span };
        let rest_local = self.fresh("__rest");
        let rrid = self.id();
        let rest_narrow = Stmt::Narrow {
            id: rrid,
            local: rest_local.clone(),
            ty: rest.clone(),
            from: Place { root: tmp.clone(), steps: Vec::new() },
            from_ty: subj_ty.clone(),
            because: Justification::Arm { switch: id, arm: 1 },
        };
        let rest_read = Expr { ty: rest.clone(), span, kind: ExprKind::Read { place: Place { root: rest_local, steps: Vec::new() }, consume: true } };
        let rest_value = self.rewrap_into(rest_read, &ty, span);
        let absent = Block { stmts: vec![rest_narrow], value: Some(Box::new(rest_value)), span };
        let test = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id: self.id(), subject: Box::new(Expr { ty: subj_ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume: false } }), test: ArmTest::Arm(0) } };
        let br = Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::Branch {
                id,
                arms: vec![(test, present)],
                otherwise: Some(absent),
            },
        };
        self.block_expr(vec![bind], br, span)
    }

    fn elvis(&mut self, subject: &AExpr, rhs: &AExpr, span: Span) -> Expr {
        // [elvis] `s ?…?: r` / `s :?…? r`: evaluate `s` once into a temporary,
        // test whether it holds one of the **picked** arms; if so the value is
        // the subject narrowed to the picked type (widened into the result when
        // the right side widened it [rewrap]); otherwise `r`, with `_` bound to
        // the subject narrowed to the arms left behind.
        let ty = self.ty_of(span);
        // [unify-checker] Pull every fact out of the view up front, before any
        // `&mut self` call, so no shared borrow of `checked` is held.
        let (pick, picked_ty, rest_ty, widen) = {
            let info = self.ctx.checked.pick_info(self.key(span));
            (
                info.elvis_test.cloned(),
                info.elvis_pick.map(erase).unwrap_or(Ty::Unknown),
                info.pick_left.map(erase).unwrap_or(Ty::Unknown),
                self.ctx.checked.elvis_widen.get(&self.key(span)).cloned(),
            )
        };
        let s = self.expr_subject(subject);
        let subj_ty = s.ty.clone();
        let tmp = self.fresh("__pick");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: s };
        let id = self.id();
        // The picked value: the subject narrowed to the picked type, consumed,
        // and widened into the result if the pick widened.
        let nar = self.fresh("__picked");
        let nid = self.id();
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: picked_ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Cond { branch: id, arm: 0 } };
        let read_picked = Expr { ty: picked_ty.clone(), span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: true } };
        let value = match &widen {
            Some(c) => apply_coercion(c, read_picked, span),
            None => read_picked,
        };
        // The right side: `_` bound to the subject narrowed to the arms left.
        let rest = self.fresh("__rest");
        let rid = self.id();
        let rest_narrow = Stmt::Narrow { id: rid, local: rest.clone(), ty: rest_ty, from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::After { branch: id, arm: 0 } };
        let saved = self.placeholder.replace(rest.clone());
        let r = self.expr(rhs);
        self.placeholder = saved;
        // The `_` binding is dead unless the right side actually read it (a
        // plain `?:` that only supplies a default never does) — and binding it
        // to the arms left behind can produce a `None`-typed local the backends
        // reject (`val __rest: Unit = null`). So emit it only when used.
        let mut uses_rest = false;
        visit_reads(&r, &mut |l: &Local| {
            if *l == rest {
                uses_rest = true;
            }
        });
        let otherwise_stmts = if uses_rest { vec![rest_narrow] } else { Vec::new() };
        // The condition: the subject is one of the picked arms. A disjunction
        // of `Test` nodes — one per picked value arm, plus a `None` test when
        // the pick keeps the `None` arm. Testing the picked side directly
        // handles any mix of value arms and `None`.
        let read_tmp = || Expr { ty: subj_ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume: false } };
        let mut tests: Vec<Expr> = Vec::new();
        if let Some(p) = &pick {
            for arm in &p.value_arms {
                tests.push(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id: self.id(), subject: Box::new(read_tmp()), test: ArmTest::Arm(*arm) } });
            }
            if p.picks_none {
                tests.push(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id: self.id(), subject: Box::new(read_tmp()), test: ArmTest::None } });
            }
        }
        let cond = tests
            .into_iter()
            .reduce(|prev, t| Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::Or, args: vec![prev, t] } })
            .unwrap_or(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) });
        let br = Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::Branch {
                id,
                arms: vec![(cond, Block { stmts: vec![narrow], value: Some(Box::new(value)), span })],
                otherwise: Some(Block { stmts: otherwise_stmts, value: Some(Box::new(r)), span }),
            },
        };
        self.block_expr(vec![bind], br, span)
    }

    // --------------------------------------------------------- branching --

    pub(crate) fn if_expr(&mut self, branches: &[(AExpr, ast::Block)], else_block: Option<&ast::Block>, span: Span, value: bool) -> Expr {
        let ty = if value { self.ty_of(span) } else { Ty::none() };
        let id = self.id();
        let mut arms = Vec::new();
        let mut otherwise: Option<Block> = None;
        for (i, (cond, block)) in branches.iter().enumerate() {
            self.push_scope();
            // [ir-narrow] A later arm's condition may read what the earlier
            // arms' failure narrowed; such a narrowing holds only there, so
            // the rest of the chain nests in this one's `else`, after it.
            let saved = std::mem::take(&mut self.pending);
            let c = self.condition(cond);
            let cond_pending = std::mem::replace(&mut self.pending, saved);
            if i > 0 && !cond_pending.is_empty() {
                // The arm's scope stays open for the nested chain's first arm.
                let rest = self.if_chain_nested(&branches[i..], else_block, span, value, cond_pending, c);
                otherwise = Some(match (value, rest) {
                    (true, (stmts, e)) => Block { stmts, value: Some(Box::new(e)), span },
                    (false, (mut stmts, e)) => {
                        stmts.push(Stmt::Expr(e));
                        Block { stmts, value: None, span }
                    }
                });
                break;
            }
            self.pending.extend(cond_pending);
            // Bindings the test introduced are in scope for the arm.
            let narrows = std::mem::take(&mut self.pending_after_test);
            let mut b = self.with_test(Some(Justification::Cond { branch: id, arm: i }), |me| me.block(block, value));
            let mut stmts = narrows;
            stmts.append(&mut b.stmts);
            b.stmts = stmts;
            self.pop_scope();
            arms.push((c, b));
            if i + 1 == branches.len() {
                otherwise = else_block.map(|b| self.else_block(id, b, value));
            }
        }
        self.set_last_branch(id);
        Expr { ty, span, kind: ExprKind::Branch { id, arms, otherwise } }
    }

    /// [assert-trap] The Salvo location a trap names: `module:line:col` —
    /// the module path, not the file name, which depends on the loader.
    pub(crate) fn location(&self, span: Span) -> String {
        let file = &self.ctx.program.files[self.file_idx];
        let (line, col) = salvo_syntax::span::line_col(&file.content, span.start);
        format!("{}:{line}:{col}", file.module)
    }

    /// A condition: an expression whose `is` tests narrow what follows
    /// [ir-narrow].
    pub(crate) fn condition(&mut self, cond: &AExpr) -> Expr {
        let saved = std::mem::replace(&mut self.in_condition, true);
        let c = self.expr(cond);
        self.in_condition = saved;
        c
    }

    /// The tail of an `if` chain whose first condition `c` needed the
    /// statements `pending` (narrowings) before it: those statements, then
    /// the chain from that arm on.
    fn if_chain_nested(&mut self, branches: &[(AExpr, ast::Block)], else_block: Option<&ast::Block>, span: Span, value: bool, pending: Vec<Stmt>, c: Expr) -> (Vec<Stmt>, Expr) {
        let ty = if value { self.ty_of(span) } else { Ty::none() };
        let id = self.id();
        let mut arms = Vec::new();
        let otherwise: Option<Block>;
        // The first arm's condition is already lowered, in a scope still open.
        let narrows = std::mem::take(&mut self.pending_after_test);
        let mut b = self.with_test(Some(Justification::Cond { branch: id, arm: 0 }), |me| me.block(&branches[0].1, value));
        let mut stmts = narrows;
        stmts.append(&mut b.stmts);
        b.stmts = stmts;
        self.pop_scope();
        arms.push((c, b));
        if branches.len() == 1 {
            otherwise = else_block.map(|b| self.else_block(id, b, value));
        } else {
            let e = self.if_expr(&branches[1..], else_block, span, value);
            otherwise = Some(if value { Block { stmts: Vec::new(), value: Some(Box::new(e)), span } } else { Block { stmts: vec![Stmt::Expr(e)], value: None, span } });
        }
        self.set_last_branch(id);
        (pending, Expr { ty, span, kind: ExprKind::Branch { id, arms, otherwise } })
    }

    /// [ir-narrow] An `else` is where `branch`'s tests failed, so a name they
    /// narrowed reads as what remains — justified by the branch, not by an
    /// enclosing arm's test. Lowered under the enclosing justification, the
    /// `else` of `if one is A` inside a `when`'s `is Errs` arm read `one`
    /// as that arm's index: a `ClassCastException` on Kotlin, a type error
    /// on Rust.
    fn else_block(&mut self, branch: NodeId, b: &ast::Block, value: bool) -> Block {
        let saved = self.last_branch.replace(branch);
        let out = self.with_test(None, |me| me.block(b, value));
        self.last_branch = saved;
        out
    }

    pub(crate) fn when_cond_expr(&mut self, branches: &[(AExpr, ast::Block)], else_block: Option<&ast::Block>, span: Span, value: bool) -> Expr {
        self.if_expr(branches, else_block, span, value)
    }

    pub(crate) fn when_expr(&mut self, binding: Option<&ast::Ident>, subject: &AExpr, branches: &[ast::WhenBranch], span: Span, value: bool) -> Expr {
        let ty = if value { self.ty_of(span) } else { Ty::none() };
        let s = self.expr_subject(subject);
        let subj_ty = s.ty.clone();
        let id = self.id();
        // [when-bind] `when name = subject { … }`: bind the subject once into
        // the named local and switch on it; the arms narrow from that place.
        // The plain form switches on the subject's own stored place.
        let (prelude, subject_read, place) = match binding {
            Some(name) => {
                let local = self.bind(&name.name, subj_ty.clone());
                let lid = self.id();
                let bind = Stmt::Let { id: lid, local: local.clone(), ty: subj_ty.clone(), value: s };
                let read = Expr { ty: subj_ty.clone(), span, kind: ExprKind::Read { place: Place { root: local.clone(), steps: Vec::new() }, consume: false } };
                (vec![bind], read, Some(Place { root: local, steps: Vec::new() }))
            }
            None => {
                let place = self.stored_place_of(subject);
                (Vec::new(), s, place)
            }
        };
        let mut arms = Vec::new();
        for (i, br) in branches.iter().enumerate() {
            let test = self.ctx.checked.is_tests.get(&self.key(br.span)).cloned();
            let arm_test = match &test {
                Some(t) => arm_test(t),
                None if br.check.is_empty() => ArmTest::Else,
                None => ArmTest::Else,
            };
            self.push_scope();
            let mut stmts = Vec::new();
            if let (Some(t), Some(place)) = (&test, place.clone()) {
                if let Some(arm_ty) = self.narrowed_ty_of_test(&subj_ty, t).filter(|t| !t.is_none_ty()) {
                    let name = br.binding.as_ref().map(|b| b.name.clone()).or_else(|| match subject { AExpr::Ident(n) => Some(n.name.clone()), _ => None });
                    if let Some(name) = name {
                        if br.binding.is_some() {
                            let local = Local(name.clone());
                            let bty = self.ty_of(br.binding.as_ref().unwrap().span).or_unknown(Some(arm_ty.clone())).unwrap_or(arm_ty.clone());
                            self.rebind(&name, local.clone(), bty.clone());
                            let nid = self.id();
                            stmts.push(Stmt::Narrow { id: nid, local, ty: bty, from: place, from_ty: subj_ty.clone(), because: Justification::Arm { switch: id, arm: i } });
                        } else {
                            stmts.push(self.narrow_in_arm(&name, place, subj_ty.clone(), arm_ty, Justification::Arm { switch: id, arm: i }));
                        }
                    }
                }
            }
            let mut b = self.with_test(Some(Justification::Arm { switch: id, arm: i }), |me| me.block(&br.body, value));
            stmts.append(&mut b.stmts);
            b.stmts = stmts;
            self.pop_scope();
            arms.push(SwitchArm { test: arm_test, body: b });
        }
        // [when-exhaustive] [ir-switch] When the checker proved the arms cover every
        // runtime arm and no branch wrote an `else`, the fallthrough is
        // dead. Record that in the IR as an explicit `Else` arm trapping as
        // unreachable, so the backends emit it rather than each synthesizing
        // its own fallthrough. (A written `else` already produced an `Else`
        // arm above; `non_null`, pass-loops and actor dispatch supply their
        // own, so this is the one else-less `Switch` shape.)
        if self.ctx.checked.when_exhaustive.contains(&self.key(span)) && !arms.iter().any(|a| matches!(a.test, ArmTest::Else)) {
            let msg = "unhandled union arm".to_string();
            let trap = Expr { ty: Ty::Never, span, kind: ExprKind::Unreachable { message: Some(Box::new(Expr { ty: Ty::named("Str"), span, kind: ExprKind::Str(msg) })), at: self.location(span) } };
            arms.push(SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![Stmt::Expr(trap)], value: None, span } });
        }
        self.set_last_branch(id);
        let switch = Expr { ty: ty.clone(), span, kind: ExprKind::Switch { id, subject: Box::new(subject_read), arms } };
        if prelude.is_empty() {
            switch
        } else {
            self.block_expr(prelude, switch, span)
        }
    }

    // ------------------------------------------------------------- loops --

    /// `while c { body } else { e }`, as a statement; with `result`, the
    /// loop's value goes to that local.
    pub(crate) fn while_stmt(&mut self, cond: &AExpr, body: &ast::Block, else_block: Option<&ast::Block>, span: Span, result: Option<(Local, Ty)>) -> Vec<Stmt> {
        let mut out = Vec::new();
        let ran = else_block.map(|_| self.fresh("__ran"));
        if let Some((r, t)) = &result {
            let id = self.id();
            out.push(Stmt::Let { id, local: r.clone(), ty: t.clone(), value: Expr { ty: t.clone(), span, kind: ExprKind::MakeNone } });
        }
        if let Some(r) = &ran {
            let id = self.id();
            out.push(Stmt::Let { id, local: r.clone(), ty: Ty::named("Bool"), value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(false) } });
        }
        let loop_id = self.id();
        self.push_scope();
        self.loops += 1;
        self.loop_results.push(result.as_ref().map(|(r, _)| r.clone()));
        let mut stmts = Vec::new();
        // [loop-never] A literal-`true` condition needs no exit test: emit a
        // bare infinite loop, so the host's own never-typing applies (rustc
        // reads `loop {}` as `!`; a `loop { if !true { break } }` as `()`).
        // The `while true` idiom therefore lowers to a true infinite loop on
        // both backends, which is what makes a non-escaping one diverge.
        let always = matches!(cond, AExpr::Bool { value: true, .. });
        let body_just = if !always {
            let saved_pending = std::mem::take(&mut self.pending);
            let c = self.condition(cond);
            stmts.append(&mut self.pending);
            self.pending = saved_pending;
            let narrows = std::mem::take(&mut self.pending_after_test);
            let not = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::Not, args: vec![c] } };
            let bid = self.id();
            stmts.push(Stmt::Expr(Expr { ty: Ty::none(), span, kind: ExprKind::Branch { id: bid, arms: vec![(not, Block { stmts: vec![Stmt::Break], value: None, span })], otherwise: None } }));
            stmts.extend(narrows);
            Some(Justification::Cond { branch: bid, arm: 0 })
        } else {
            None
        };
        if let Some(r) = &ran {
            stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) } });
        }
        let b = self.with_test(body_just, |me| me.block(body, result.is_some()));
        stmts.extend(b.stmts);
        match (b.value, &result) {
            (Some(v), Some((r, _))) if loop_body_value(&v) => stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v }),
            (Some(v), _) => stmts.push(Stmt::Expr(*v)),
            _ => {}
        }
        self.loop_results.pop();
        self.loops -= 1;
        self.pop_scope();
        out.push(Stmt::Loop { id: loop_id, body: Block { stmts, value: None, span: body.span } });
        if let (Some(r), Some(eb)) = (ran, else_block) {
            let b = self.block(eb, result.is_some());
            let mut arm = b;
            if let (Some(v), Some((res, _))) = (arm.value.take(), &result) {
                arm.stmts.push(Stmt::Assign { place: Place { root: res.clone(), steps: Vec::new() }, value: *v });
            }
            let not_ran = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::Not, args: vec![Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Read { place: Place { root: r, steps: Vec::new() }, consume: false } }] } };
            let id = self.id();
            out.push(Stmt::Expr(Expr { ty: Ty::none(), span, kind: ExprKind::Branch { id, arms: vec![(not_ran, arm)], otherwise: None } }));
        }
        out
    }

    pub(crate) fn for_stmt(&mut self, pattern: &Pattern, iterable: &AExpr, body: &ast::Block, else_block: Option<&ast::Block>, span: Span, result: Option<(Local, Ty)>) -> Vec<Stmt> {
        let mut out = Vec::new();
        let ran = else_block.map(|_| self.fresh("__ran"));
        if let Some((r, t)) = &result {
            let id = self.id();
            out.push(Stmt::Let { id, local: r.clone(), ty: t.clone(), value: Expr { ty: t.clone(), span, kind: ExprKind::MakeNone } });
        }
        if let Some(r) = &ran {
            let id = self.id();
            out.push(Stmt::Let { id, local: r.clone(), ty: Ty::named("Bool"), value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(false) } });
        }
        let driver = self.ctx.checked.for_drivers.get(&self.key(iterable.span())).cloned();
        // [iter-step-call] a step call is re-invoked each turn, so it is
        // lowered inside the loop; every other subject is evaluated once.
        let step_call = driver.as_ref().is_some_and(|d| d.step_call);
        let mut subject = if step_call { Expr { ty: Ty::Unknown, span, kind: ExprKind::Unit } } else { self.expr(iterable) };
        let mut elem_ty = match pattern {
            Pattern::Ident(id) => self.ty_of(id.span),
            other => self.ty_of(pattern_span(other)),
        };
        // [fate-move-mode] a body that consumes the loop variable makes the
        // loop iterate by value: the subject is consumed, each element owned.
        if driver.is_none() && self.ctx.checked.binding_modes.contains(&self.key(iterable.span())) {
            self.consume_if_owned(&mut subject);
            if matches!(subject.kind, ExprKind::Read { consume: true, .. }) {
                elem_ty = elem_ty.remove_quals(&crate::types::proj_names());
            }
        }
        self.push_scope();
        self.loops += 1;
        self.loop_results.push(result.as_ref().map(|(r, _)| r.clone()));
        let loop_id = self.id();
        let mut stmts = Vec::new();
        let bind_pattern = |me: &mut Self, value: Expr, stmts: &mut Vec<Stmt>| match pattern {
            Pattern::Ident(id) => {
                let local = me.bind(&id.name, value.ty.clone());
                let nid = me.id();
                stmts.push(Stmt::Let { id: nid, local, ty: value.ty.clone(), value });
            }
            other => {
                let tmp = me.fresh("__elem");
                let nid = me.id();
                let ty = value.ty.clone();
                stmts.push(Stmt::Let { id: nid, local: tmp.clone(), ty: ty.clone(), value });
                stmts.extend(me.destructure(other, tmp, &ty, span));
            }
        };
        let mut pass_tys: Vec<(Local, Ty)> = Vec::new();
        match driver {
            Some(d) => {
                // The pass: minted, or the subject itself.
                let pass = self.fresh("__pass");
                let subject_ty = subject.ty.clone();
                let minted = match &d.mint {
                    Some(m) => {
                        let (target, ret, effects) = self.pass_member_full(m, &subject_ty);
                        let mut args: Vec<Expr> = effects.iter().map(|t| self.effect_instance(t, span)).collect();
                        args.push(subject);
                        Expr { ty: ret, span, kind: ExprKind::Call { target, type_args: Vec::new(), args } }
                    }
                    None => subject,
                };
                // [iter-drive-in-place] A named pass is driven where it
                // lives: the place itself, with no local of the loop's.
                let in_place = match &minted.kind {
                    ExprKind::Read { place, .. } if place.steps.is_empty() && !step_call => Some(place.root.clone()),
                    _ => None,
                };
                let pass = match in_place {
                    Some(root) => {
                        let ty = minted.ty.clone();
                        pass_tys.push((root.clone(), ty));
                        root
                    }
                    None => {
                        if !step_call {
                            let mid = self.id();
                            pass_tys.push((pass.clone(), minted.ty.clone()));
                            out.push(Stmt::Let { id: mid, local: pass.clone(), ty: minted.ty.clone(), value: minted });
                        }
                        pass
                    }
                };
                let step = self.fresh("__step");
                let step_ty = Ty::Union(vec![elem_ty.clone(), Ty::named("Finished")]);
                let sid = self.id();
                let step_value = if step_call {
                    self.expr(iterable)
                } else {
                    let pass_ty = pass_tys.iter().find(|(l, _)| *l == pass).map(|(_, t)| t.clone()).unwrap_or(Ty::Unknown);
                    let (next_target, _, effects) = self.pass_member_full(&d.next, &pass_ty);
                    let mut args: Vec<Expr> = effects.iter().map(|t| self.effect_instance(t, span)).collect();
                    args.push(Expr { ty: pass_ty, span, kind: ExprKind::Read { place: Place { root: pass.clone(), steps: Vec::new() }, consume: false } });
                    Expr { ty: step_ty.clone(), span, kind: ExprKind::Call { target: next_target, type_args: Vec::new(), args } }
                };
                stmts.push(Stmt::Let { id: sid, local: step.clone(), ty: step_ty.clone(), value: step_value });
                let swid = self.id();
                let nar = self.fresh("__emitted");
                let nid = self.id();
                let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: elem_ty.clone(), from: Place { root: step.clone(), steps: Vec::new() }, from_ty: step_ty.clone(), because: Justification::Arm { switch: swid, arm: 0 } };
                let mut arm_stmts = vec![narrow];
                bind_pattern(self, Expr { ty: elem_ty.clone(), span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: true } }, &mut arm_stmts);
                if let Some(r) = &ran {
                    arm_stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) } });
                }
                let b = self.block(body, result.is_some());
                arm_stmts.extend(b.stmts);
                match (b.value, &result) {
                    (Some(v), Some((r, _))) if loop_body_value(&v) => arm_stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v }),
                    (Some(v), _) => arm_stmts.push(Stmt::Expr(*v)),
                    _ => {}
                }
                stmts.push(Stmt::Expr(Expr {
                    ty: Ty::none(),
                    span,
                    kind: ExprKind::Switch {
                        id: swid,
                        subject: Box::new(Expr { ty: step_ty, span, kind: ExprKind::Read { place: Place { root: step, steps: Vec::new() }, consume: false } }),
                        arms: vec![
                            SwitchArm { test: ArmTest::Arm(d.emitted_arm), body: Block { stmts: arm_stmts, value: None, span: body.span } },
                            SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![Stmt::Break], value: None, span } },
                        ],
                    },
                }));
                self.loop_results.pop();
                self.loops -= 1;
                self.pop_scope();
                out.push(Stmt::Loop { id: loop_id, body: Block { stmts, value: None, span: body.span } });
            }
            None => {
                // [iter-for-native] an intrinsic container.
                let local = match pattern {
                    Pattern::Ident(id) => self.bind(&id.name, elem_ty.clone()),
                    other => {
                        let tmp = self.fresh("__elem");
                        stmts.extend(self.destructure(other, tmp.clone(), &elem_ty, span));
                        tmp
                    }
                };
                if let Some(r) = &ran {
                    stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) } });
                }
                let b = self.block(body, result.is_some());
                stmts.extend(b.stmts);
                match (b.value, &result) {
                    (Some(v), Some((r, _))) if loop_body_value(&v) => stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v }),
                    (Some(v), _) => stmts.push(Stmt::Expr(*v)),
                    _ => {}
                }
                self.loop_results.pop();
                self.loops -= 1;
                self.pop_scope();
                out.push(Stmt::ForEach { id: loop_id, local, ty: elem_ty, iterable: subject, body: Block { stmts, value: None, span: body.span } });
            }
        }
        if let (Some(r), Some(eb)) = (ran, else_block) {
            let mut arm = self.block(eb, result.is_some());
            if let (Some(v), Some((res, _))) = (arm.value.take(), &result) {
                arm.stmts.push(Stmt::Assign { place: Place { root: res.clone(), steps: Vec::new() }, value: *v });
            }
            let not_ran = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::Not, args: vec![Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Read { place: Place { root: r, steps: Vec::new() }, consume: false } }] } };
            let id = self.id();
            out.push(Stmt::Expr(Expr { ty: Ty::none(), span, kind: ExprKind::Branch { id, arms: vec![(not_ran, arm)], otherwise: None } }));
        }
        out
    }

    // --------------------------------------------------------------- use --

    /// [eff-handle] `eff[a, b]` / `eff[*]` merge: a handle (single or tuple)
    /// carrying the result effects, one handle each. For `eff[*]` each comes
    /// from the scope's bound handle (`effect_instance`); for explicit items,
    /// from the merged handle values, matched by effect.
    pub(crate) fn merge_expr(&mut self, items: &[AExpr], all: bool, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let effects: Vec<Ty> = self
            .ctx
            .checked
            .merge_effects
            .get(&self.key(span))
            .map(|v| v.iter().map(erase).collect())
            .unwrap_or_default();
        if all {
            // Each result effect is read from its handle bound in scope.
            let reads: Vec<Expr> = effects.iter().map(|e| self.effect_instance(e, span)).collect();
            let value = match reads.len() {
                0 => unsupported(ty.clone(), span, "eff[*] with no handle in scope"),
                1 => reads.into_iter().next().unwrap(),
                _ => Expr { ty, span, kind: ExprKind::Tuple(reads) },
            };
            return value;
        }
        // Explicit items: bind each once, gather its per-effect handles, then
        // assemble the result in `effects` order.
        let mut stmts: Vec<Stmt> = Vec::new();
        let mut carried: Vec<(Ty, Expr)> = Vec::new();
        for item in items {
            let v = self.expr(item);
            let vty = v.ty.clone();
            let tmp = self.fresh("__merge");
            let id = self.id();
            stmts.push(Stmt::Let { id, local: tmp.clone(), ty: vty.clone(), value: v });
            match vty.strip_quals() {
                Ty::Tuple(elems) => {
                    for (i, et) in elems.iter().enumerate() {
                        if let Some(eff) = handle_effect(et) {
                            let read = Expr { ty: et.clone(), span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: vec![Step::Tuple(i)] }, consume: false } };
                            carried.push((eff, read));
                        }
                    }
                }
                other => {
                    if let Some(eff) = handle_effect(other) {
                        let read = Expr { ty: vty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume: false } };
                        carried.push((eff, read));
                    }
                }
            }
        }
        let reads: Vec<Expr> = effects
            .iter()
            .map(|e| {
                carried
                    .iter()
                    .find(|(ce, _)| ce == e)
                    .map(|(_, ex)| ex.clone())
                    .unwrap_or_else(|| unsupported(Ty::Named { name: "Addr".into(), args: vec![e.clone()] }, span, "merge: effect not among items"))
            })
            .collect();
        let value = match reads.len() {
            0 => unsupported(ty.clone(), span, "merge with no effect"),
            1 => reads.into_iter().next().unwrap(),
            _ => Expr { ty, span, kind: ExprKind::Tuple(reads) },
        };
        self.block_expr(stmts, value, span)
    }

    /// [eff-handle] `e[A, B]` projection: a non-consuming copy of the handles
    /// carrying the requested effects, picked out of the base (a single handle
    /// or a tuple of handles). Yields the one handle (single) or a tuple of
    /// them (several), reading the base's elements by position — the base is
    /// bound once so a computed base is not re-evaluated.
    pub(crate) fn project_expr(&mut self, base: &AExpr, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let want: Vec<Ty> = self
            .ctx
            .checked
            .project_effects
            .get(&self.key(span))
            .map(|v| v.iter().map(erase).collect())
            .unwrap_or_default();
        let b = self.expr(base);
        let base_ty = b.ty.clone();
        let single = !matches!(base_ty.strip_quals(), Ty::Tuple(_));
        // The base's carried handles, by position.
        let have: Vec<Ty> = match base_ty.strip_quals() {
            Ty::Tuple(elems) => elems.to_vec(),
            other => vec![other.clone()],
        };
        let tmp = self.fresh("__proj");
        let id = self.id();
        let bind = Stmt::Let { id, local: tmp.clone(), ty: base_ty.clone(), value: b };
        // The handle carrying effect `w`: the whole base when it is a single
        // handle, else the matching tuple element (read non-consuming — a
        // projection is a copy).
        let pick = |w: &Ty| -> Expr {
            if single {
                return Expr { ty: base_ty.clone(), span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: Vec::new() }, consume: false } };
            }
            let idx = have.iter().position(|h| handle_carries(h, w)).unwrap_or(0);
            let et = have.get(idx).cloned().unwrap_or_else(|| Ty::Named { name: "Addr".into(), args: vec![w.clone()] });
            Expr { ty: et, span, kind: ExprKind::Read { place: Place { root: tmp.clone(), steps: vec![Step::Tuple(idx)] }, consume: false } }
        };
        let reads: Vec<Expr> = want.iter().map(&pick).collect();
        let value = match reads.len() {
            0 => unsupported(ty.clone(), span, "projection with no effect"),
            1 => reads.into_iter().next().unwrap(),
            _ => Expr { ty, span, kind: ExprKind::Tuple(reads) },
        };
        self.block_expr(vec![bind], value, span)
    }

    /// Lowers a `use` to the handle bindings it introduces. Returns the
    /// statements **and** the `(face, handle-local)` pairs it bound and pushed
    /// onto `effect_env` — the statement form discards the pairs; the
    /// expression form ([eff-handle]) reads them back as its value.
    pub(crate) fn use_stmt_bound(&mut self, handler: &AExpr, with_items: &[AExpr], span: Span) -> (Vec<Stmt>, Vec<(Ty, Local)>) {
        let key = self.key(span);
        let info = self.ctx.checked.use_info(key);
        let faces: Vec<Ty> = info.use_effects.map(|v| v.iter().map(erase).collect()).unwrap_or_default();
        // [eff-handle] `use <tuple>`: bind one effect per tuple element, in
        // order, each `instance_at(tuple.i)` — `use e.0; use e.1`.
        if let Some(effects) = info.use_addr_tuple.map(|v| v.iter().map(erase).collect::<Vec<_>>()) {
            let tup = self.expr(handler);
            let tup_ty = tup.ty.clone();
            let base = self.fresh("__use");
            let bid = self.id();
            let mut out = vec![Stmt::Let { id: bid, local: base.clone(), ty: tup_ty.clone(), value: tup }];
            let mut bound: Vec<(Ty, Local)> = Vec::new();
            for (i, effect) in effects.into_iter().enumerate() {
                let elem = Expr {
                    ty: effect.clone(),
                    span,
                    kind: ExprKind::Read { place: Place { root: base.clone(), steps: vec![Step::Tuple(i)] }, consume: false },
                };
                let inst = Expr { ty: effect.clone(), span, kind: ExprKind::AddrInstance { addr: Box::new(elem) } };
                let local = self.fresh("__use");
                let id = self.id();
                out.push(Stmt::Let { id, local: local.clone(), ty: effect.clone(), value: inst });
                self.effect_env.push((effect.clone(), local.clone()));
                bound.push((effect, local));
            }
            return (out, bound);
        }
        // `use addr`: the addr is the instance.
        if let Some(effect) = info.use_addr.map(erase) {
            let a = self.expr(handler);
            let local = self.fresh("__use");
            let id = self.id();
            self.effect_env.push((effect.clone(), local.clone()));
            let inst = Expr { ty: effect.clone(), span, kind: ExprKind::AddrInstance { addr: Box::new(a) } };
            return (
                vec![Stmt::Let { id, local: local.clone(), ty: effect.clone(), value: inst }],
                vec![(effect, local)],
            );
        }
        // `use H(args)`: a construct, then one instance per face.
        let deps: Vec<Ty> = info.use_deps.map(|v| v.iter().map(erase).collect()).unwrap_or_default();
        let items: Vec<Option<usize>> = info.use_with_items.cloned().unwrap_or_else(|| vec![None; deps.len()]);
        let Some(construct) = self.construct_handler(handler, &deps, &items, with_items, span) else {
            self.error(span, "`use` of something that is not a handler construction");
            return (vec![Stmt::Expr(unsupported(Ty::Unknown, span, "use"))], Vec::new());
        };
        let handler_ty = construct.ty.clone();
        let inst = self.fresh("__use");
        let id = self.id();
        let mut out = vec![Stmt::Let { id, local: inst.clone(), ty: handler_ty.clone(), value: construct }];
        // [effect-handle] one handle per face, over the one instance — the
        // instance itself for a `threadsafe platform handler`, which promises
        // its own synchronization [threadsafe-platform].
        let threadsafe = match handler_ty.strip_quals() {
            Ty::Named { name, .. } => self.ctx.symbols.handlers.get(name.as_str()).is_some_and(|h| h.threadsafe),
            _ => false,
        };
        let mut bound: Vec<(Ty, Local)> = Vec::new();
        for f in faces {
            let h = self.fresh("__handle");
            let hid = self.id();
            let read = Expr { ty: handler_ty.clone(), span, kind: ExprKind::Read { place: Place { root: inst.clone(), steps: Vec::new() }, consume: false } };
            let value = if threadsafe { Expr { ty: f.clone(), ..read } } else { Expr { ty: f.clone(), span, kind: ExprKind::Handle { instance: Box::new(read) } } };
            out.push(Stmt::Let { id: hid, local: h.clone(), ty: f.clone(), value });
            self.effect_env.push((f.clone(), h.clone()));
            bound.push((f, h));
        }
        (out, bound)
    }

    pub(crate) fn use_stmt(&mut self, handler: &AExpr, with_items: &[AExpr], span: Span) -> Vec<Stmt> {
        self.use_stmt_bound(handler, with_items, span).0
    }

    /// [eff-handle] `use H()` as a value: the bindings, then the handle it
    /// yields — the one bound handle for a single face, a tuple of them for
    /// several (mirroring how a multi-face `spawn` yields a tuple of addrs).
    pub(crate) fn use_expr(&mut self, handler: &AExpr, with_items: &[AExpr], span: Span) -> Expr {
        let ty = self.ty_of(span);
        let (stmts, bound) = self.use_stmt_bound(handler, with_items, span);
        // [eff-handle] Hoist the handle bindings to the enclosing statement
        // level (via `pending`) rather than nesting them in a block-expr: the
        // handle locals must stay in scope for the rest of the body (a later
        // `now()` or `eff[*]` reads them), and the IR `effect_env` entries this
        // pushed point at them.
        self.pending.extend(stmts);
        let reads: Vec<Expr> = bound
            .iter()
            .map(|(f, l)| Expr { ty: f.clone(), span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false } })
            .collect();
        match reads.len() {
            0 => unsupported(ty, span, "use expression with no handle"),
            1 => reads.into_iter().next().unwrap(),
            _ => Expr { ty, span, kind: ExprKind::Tuple(reads) },
        }
    }

    /// `H(args) [with …]` as a `Construct` of the handler: written arguments,
    /// resolved implicits, then one field per dependency [effect-handler-deps].
    pub(crate) fn construct_handler(&mut self, handler: &AExpr, deps: &[Ty], items: &[Option<usize>], with_items: &[AExpr], span: Span) -> Option<Expr> {
        let (name, args, named): (String, Vec<&AExpr>, Vec<ast::NamedArg>) = match handler {
            AExpr::Call { callee, args, named, .. } => match callee.as_ref() {
                AExpr::Ident(id) => (id.name.clone(), args.iter().collect(), named.clone()),
                _ => return None,
            },
            AExpr::Ident(id) => (id.name.clone(), Vec::new(), Vec::new()),
            _ => return None,
        };
        let key = self.key(span);
        let hname = self.ctx.checked.route_stubs.get(&key).cloned().unwrap_or(name);
        let hkey = self.ctx.checked.visible_handler_key(self.file_idx, &hname).to_string();
        let decl = self.ctx.symbols.handlers.get(hkey.as_str()).copied()?;
        let call_span = handler.span();
        let handler_ty = Ty::Named {
            name: hkey.clone(),
            args: self.ctx.checked.use_handler_args.get(&key).map(|v| v.iter().map(erase).collect()).unwrap_or_default(),
        };
        let mut fields: Vec<(String, Expr)> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let v = self.expr(a);
            let pname = decl.params.iter().filter(|p| !p.implicit).nth(i).map(|p| p.name.name.clone()).unwrap_or(format!("__arg{i}"));
            fields.push((pname, v));
        }
        let filled = self.ctx.checked.implicit_args.get(&key).or_else(|| self.ctx.checked.implicit_args.get(&self.key(call_span))).cloned().unwrap_or_default();
        let implicits = self.implicit_list(&filled, &named, span);
        for (i, v) in implicits.into_iter().enumerate() {
            let pname = decl.params.iter().filter(|p| p.implicit).nth(i).map(|p| p.name.name.clone()).unwrap_or(format!("__imp{i}"));
            fields.push((pname, v));
        }
        // The dependencies: the checker's list, or the declaration's when the
        // construction is a `with` item (which records none of its own).
        let declared: Vec<Ty> = decl
            .effects
            .iter()
            .flatten()
            .filter_map(|p| match p {
                ast::EffectRef::Effect(r) | ast::EffectRef::AnyEffect(r) => Some(Ty::Named {
                    name: self.ctx.checked.written_key(r).to_string(),
                    args: r.args.iter().map(|a| self.ctx.written_ty(self.file_idx, a)).collect(),
                }),
                _ => None,
            })
            .collect();
        let deps: Vec<Ty> = if deps.is_empty() { declared } else { deps.to_vec() };
        for (i, d) in deps.iter().enumerate() {
            let mut v = match items.get(i).copied().flatten().and_then(|w| with_items.get(w)) {
                Some(item) => self.with_item(item),
                None => self.effect_instance(d, span),
            };
            // [actor-use-addr] an addr given where an instance is wanted.
            if matches!(v.ty.strip_quals(), Ty::Named { name, .. } if name == "Addr") {
                let vspan = v.span;
                v = Expr { ty: d.clone(), span: vspan, kind: ExprKind::AddrInstance { addr: Box::new(v) } };
            }
            // [with-clause] [effect-handle] a private instance built for this
            // binding alone is bound behind its handle, like any `use`.
            if let Ty::Named { name, .. } = v.ty.strip_quals() {
                if let Some(h) = self.ctx.symbols.handlers.get(name.as_str()) {
                    if !h.threadsafe {
                        let vspan = v.span;
                        v = Expr { ty: d.clone(), span: vspan, kind: ExprKind::Handle { instance: Box::new(v) } };
                    }
                }
            }
            fields.push((format!("__dep{i}"), v));
        }
        Some(Expr { ty: handler_ty, span, kind: ExprKind::Construct { fields } })
    }

    /// [with-clause] A `with` item: an instance in scope, or a private handler
    /// constructed for this binding alone.
    fn with_item(&mut self, item: &AExpr) -> Expr {
        let is_handler = match item {
            AExpr::Call { callee, .. } => matches!(callee.as_ref(), AExpr::Ident(id) if self.ctx.symbols.handlers.contains_key(self.ctx.checked.visible_handler_key(self.file_idx, &id.name))),
            AExpr::Ident(id) => self.lookup(&id.name).is_none() && self.ctx.symbols.handlers.contains_key(self.ctx.checked.visible_handler_key(self.file_idx, &id.name)),
            _ => false,
        };
        if is_handler {
            if let Some(c) = self.construct_handler(item, &[], &[], &[], item.span()) {
                return c;
            }
        }
        self.expr(item)
    }
}

/// What a call resolved to, for argument modes.
pub(crate) enum Callee<'p> {
    Fn(crate::FnKey, &'p ast::FnDecl),
    Member(&'p ast::FnDecl),
}

/// The checker's union test, as an IR arm test.
pub(crate) fn arm_test(t: &UnionTest) -> ArmTest {
    // A `None` test, whatever declared arm the checker counted it under
    // (`is Ok` on `Ok None | Err E` is one).
    if t.match_none {
        return ArmTest::None;
    }
    if !t.values.is_empty() {
        let lit = |l: &ast::TypeLit| match l {
            ast::TypeLit::Str(s) => Lit::Str(s.clone()),
            ast::TypeLit::Int(i) => Lit::Int(*i),
            ast::TypeLit::Long(i) => Lit::Long(*i),
            ast::TypeLit::Bool(b) => Lit::Bool(*b),
        };
        let arms: Vec<LitArm> = t
            .arms
            .iter()
            .map(|a| match t.values.iter().find(|(arm, _, _)| arm == a) {
                Some((_, negate, ls)) => LitArm { arm: *a, negate: *negate, lits: ls.iter().map(lit).collect() },
                None => LitArm { arm: *a, negate: false, lits: Vec::new() },
            })
            .collect();
        return ArmTest::Lit(arms);
    }
    match t.arms.as_slice() {
        [one] => ArmTest::Arm(*one),
        many => ArmTest::Arms(many.to_vec()),
    }
}

trait OrUnknown {
    fn or_unknown(self, other: Option<Ty>) -> Option<Ty>;
}
impl OrUnknown for Ty {
    fn or_unknown(self, other: Option<Ty>) -> Option<Ty> {
        if self.is_unknown() { other } else { Some(self) }
    }
}

fn base_of(t: &Ty) -> Option<&str> {
    match t.strip_quals() {
        Ty::Named { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

pub(crate) fn pattern_span(p: &Pattern) -> Span {
    match p {
        Pattern::Ident(id) => id.span,
        Pattern::Tuple { span, .. } | Pattern::Struct { span, .. } => *span,
    }
}

/// A declared type with the callee's generic parameters replaced by the
/// call's type arguments.
pub(crate) fn subst_vars(t: &Ty, generics: &[String], args: &[Ty]) -> Ty {
    match t {
        Ty::Var(v) | Ty::Named { name: v, .. } if generics.iter().position(|g| g == v).is_some_and(|i| i < args.len()) => {
            args[generics.iter().position(|g| g == v).unwrap()].clone()
        }
        Ty::Named { name, args: a } => Ty::Named { name: name.clone(), args: a.iter().map(|x| subst_vars(x, generics, args)).collect() },
        Ty::Qualified { quals, base } => Ty::Qualified { quals: quals.clone(), base: Box::new(subst_vars(base, generics, args)) },
        Ty::Union(v) => Ty::Union(v.iter().map(|x| subst_vars(x, generics, args)).collect()),
        Ty::Tuple(v) => Ty::Tuple(v.iter().map(|x| subst_vars(x, generics, args)).collect()),
        Ty::Array(e) => Ty::Array(Box::new(subst_vars(e, generics, args))),
        other => other.clone(),
    }
}

/// Binds the variables of `pattern` to the parts of `concrete` they stand for.
pub(crate) fn unify_vars(pattern: &Ty, concrete: &Ty, map: &mut std::collections::HashMap<String, Ty>) {
    match (pattern.strip_quals(), concrete.strip_quals()) {
        (Ty::Var(v), c) => {
            map.entry(v.clone()).or_insert_with(|| c.clone());
        }
        (Ty::Named { name, args: _ }, c) if !matches!(c, Ty::Named { .. }) => {
            map.entry(name.clone()).or_insert_with(|| c.clone());
        }
        (Ty::Named { name: pn, args: pa }, Ty::Named { name: cn, args: ca }) => {
            if pn != cn && pa.is_empty() {
                map.entry(pn.clone()).or_insert_with(|| concrete.strip_quals().clone());
                return;
            }
            for (p, c) in pa.iter().zip(ca) {
                unify_vars(p, c, map);
            }
        }
        (Ty::Union(ps), Ty::Union(cs)) | (Ty::Tuple(ps), Ty::Tuple(cs)) => {
            for (p, c) in ps.iter().zip(cs) {
                unify_vars(p, c, map);
            }
        }
        (Ty::Array(p), Ty::Array(c)) => unify_vars(p, c, map),
        _ => {}
    }
}

pub(crate) fn subst_map(t: &Ty, map: &std::collections::HashMap<String, Ty>) -> Ty {
    match t {
        Ty::Var(v) => map.get(v).cloned().unwrap_or_else(|| t.clone()),
        Ty::Named { name, args } if args.is_empty() && map.contains_key(name) => map[name].clone(),
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(|x| subst_map(x, map)).collect() },
        Ty::Qualified { quals, base } => Ty::Qualified { quals: quals.clone(), base: Box::new(subst_map(base, map)) },
        Ty::Union(v) => Ty::Union(v.iter().map(|x| subst_map(x, map)).collect()),
        Ty::Tuple(v) => Ty::Tuple(v.iter().map(|x| subst_map(x, map)).collect()),
        Ty::Array(e) => Ty::Array(Box::new(subst_map(e, map))),
        other => other.clone(),
    }
}

/// Whether `e` reads any local the statements `binds` bind (a narrowing
/// binding a condition's later operand depends on).
/// [eff-handle] The effect a single-effect handle type carries, if it is one.
fn handle_effect(handle: &Ty) -> Option<Ty> {
    match handle.strip_quals() {
        Ty::Named { name, args } if name == "Addr" && args.len() == 1 => Some(args[0].clone()),
        _ => None,
    }
}

/// [eff-handle] Whether a single-effect handle type `eff[E]` (`Named{Addr,[E]}`)
/// carries effect `w` — its one arg equals `w`.
fn handle_carries(handle: &Ty, w: &Ty) -> bool {
    matches!(handle.strip_quals(), Ty::Named { name, args } if name == "Addr" && args.len() == 1 && &args[0] == w)
}

fn reads_any(e: &Expr, binds: &[Stmt]) -> bool {
    let locals: Vec<&Local> = binds
        .iter()
        .filter_map(|s| match s {
            Stmt::Narrow { local, .. } | Stmt::Let { local, .. } => Some(local),
            _ => None,
        })
        .collect();
    if locals.is_empty() {
        return false;
    }
    let mut found = false;
    visit_reads(e, &mut |l| {
        if locals.contains(&l) {
            found = true;
        }
    });
    found
}

/// Every local a (statement-free) expression reads, shallowly through its
/// sub-expressions; blocks inside are visited too.
fn visit_reads(e: &Expr, f: &mut dyn FnMut(&Local)) {
    fn block(b: &Block, f: &mut dyn FnMut(&Local)) {
        for s in &b.stmts {
            match s {
                Stmt::Let { value, .. } => visit_reads(value, f),
                Stmt::Narrow { from, .. } => f(&from.root),
                Stmt::Assign { place, value } => {
                    f(&place.root);
                    visit_reads(value, f);
                }
                Stmt::Expr(e) | Stmt::Return(Some(e)) => visit_reads(e, f),
                Stmt::Loop { body, .. } => block(body, f),
                _ => {}
            }
        }
        if let Some(v) = &b.value {
            visit_reads(v, f);
        }
    }
    match &e.kind {
        ExprKind::Read { place, .. } => {
            f(&place.root);
            for st in &place.steps {
                if let Step::Index(i) = st {
                    visit_reads(i, f);
                }
            }
        }
        ExprKind::Call { args, .. } | ExprKind::Op { args, .. } | ExprKind::Tuple(args) | ExprKind::List(args) | ExprKind::Array(args) | ExprKind::Concat(args) => {
            for a in args {
                visit_reads(a, f);
            }
        }
        ExprKind::MemberCall { instance, args, .. } => {
            visit_reads(instance, f);
            for a in args {
                visit_reads(a, f);
            }
        }
        ExprKind::UnionToStr { value, arms } => {
            visit_reads(value, f);
            for a in arms {
                visit_reads(a, f);
            }
        }
        ExprKind::Construct { fields } => {
            for (_, v) in fields {
                visit_reads(v, f);
            }
        }
        ExprKind::MakeUnion { value, .. } | ExprKind::Rewrap { value, .. } | ExprKind::DropMut { value } | ExprKind::BorrowAs { value, .. } | ExprKind::Widen { value } | ExprKind::Present { value } | ExprKind::Spread { value } => visit_reads(value, f),
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms {
                visit_reads(c, f);
                block(b, f);
            }
            if let Some(o) = otherwise {
                block(o, f);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            visit_reads(subject, f);
            for a in arms {
                block(&a.body, f);
            }
        }
        ExprKind::Test { subject, .. } => visit_reads(subject, f),
        ExprKind::Lambda { body, .. } | ExprKind::Try { body } => block(body, f),
        ExprKind::Throw { message } => visit_reads(message, f),
        ExprKind::Assert { cond, message, .. } => {
            visit_reads(cond, f);
            if let Some(m) = message {
                visit_reads(m, f);
            }
        }
        ExprKind::FnValue(FnRef::Local(l)) => f(l),
        ExprKind::Handle { instance } => visit_reads(instance, f),
        ExprKind::AddrInstance { addr } => visit_reads(addr, f),
        ExprKind::Send { addr, args, .. } => {
            visit_reads(addr, f);
            for a in args {
                visit_reads(a, f);
            }
        }
        ExprKind::SelfSend { args, .. } => {
            for a in args {
                visit_reads(a, f);
            }
        }
        ExprKind::ReplyTo { captures, .. } => {
            for a in captures {
                visit_reads(a, f);
            }
        }
        ExprKind::WaitFor { body, .. } => block(body, f),
        ExprKind::Spawn { handler, .. } => visit_reads(handler, f),
        _ => {}
    }
}

/// The type a block's `return`s (or its value) answer, when one is known;
/// nested lambdas are their own fns and are not looked into.
fn returned_ty(b: &Block) -> Option<Ty> {
    fn in_block(b: &Block) -> Option<Ty> {
        for s in &b.stmts {
            if let Some(t) = in_stmt(s) {
                return Some(t);
            }
        }
        b.value.as_ref().map(|v| v.ty.clone()).filter(|t| !t.is_unknown() && *t != Ty::Never)
    }
    fn in_stmt(s: &Stmt) -> Option<Ty> {
        match s {
            Stmt::Return(Some(e)) if !e.ty.is_unknown() && e.ty != Ty::Never => Some(e.ty.clone()),
            Stmt::Expr(e) | Stmt::Let { value: e, .. } => in_expr(e),
            Stmt::Loop { body, .. } => in_block(body),
            _ => None,
        }
    }
    fn in_expr(e: &Expr) -> Option<Ty> {
        match &e.kind {
            ExprKind::Branch { arms, otherwise, .. } => arms.iter().find_map(|(_, b)| in_block(b)).or_else(|| otherwise.as_ref().and_then(in_block)),
            ExprKind::Switch { arms, .. } => arms.iter().find_map(|a| in_block(&a.body)),
            ExprKind::Try { body } => in_block(body),
            _ => None,
        }
    }
    in_block(b)
}

/// A qualifier's value argument spelled as a literal (`0`, `65535L`,
/// `"name"`, `true`), as the expression it is.
fn literal_arg(text: &str) -> Option<Expr> {
    let span = Span::default();
    let mk = |ty: &str, kind: ExprKind| Some(Expr { ty: Ty::named(ty), span, kind });
    if text == "true" || text == "false" {
        return mk("Bool", ExprKind::Bool(text == "true"));
    }
    if let Some(inner) = text.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
        return mk("Str", ExprKind::Str(inner.to_string()));
    }
    // [qual-const] A `Char` constant, `'p'`.
    if let Some(crate::types::Const::Char(c)) = crate::types::Const::parse(text) {
        return mk("Char", ExprKind::Char(c));
    }
    if let Some(n) = text.strip_suffix('L') {
        return n.replace('_', "").parse::<i64>().ok().and_then(|v| mk("Long", ExprKind::Long(v)));
    }
    if let Ok(v) = text.replace('_', "").parse::<i64>() {
        return mk("Int", ExprKind::Int(v));
    }
    if let Some(n) = text.strip_suffix('f') {
        return n.parse::<f64>().ok().and_then(|v| mk("Float", ExprKind::Float(v)));
    }
    text.parse::<f64>().ok().and_then(|v| mk("Double", ExprKind::Double(v)))
}

/// [while-value] Whether a loop body's trailing expression is a value for
/// the loop's result: an `if` with no `else` is a statement (its arms may
/// only `break`), and so is anything of type `None`.
fn loop_body_value(v: &Expr) -> bool {
    !v.ty.is_none_ty() && !matches!(&v.kind, ExprKind::Branch { otherwise: None, arms, .. } if !(arms.len() == 1 && matches!(arms[0].0.kind, ExprKind::Bool(true))))
}
