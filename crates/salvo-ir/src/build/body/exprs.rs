//! Expressions (IR record §2): every construct lowered to the node set, every
//! resolution copied from the checker.

use salvo_core::check::{CompareVia, ImplicitArg, UnionTest};
use salvo_core::param_mode::PassMode;
use salvo_core::types::Ty;
use salvo_syntax::ast::{self, BinaryOp, Expr as AExpr, LambdaBody, Pattern, StrExprPart, UnaryOp};
use salvo_syntax::Span;

use super::Lower;
use crate::build::{apply_coercion, erase, unsupported};
use crate::ir::*;

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
                    Some(k) => Expr { ty: self.ty_of(span), span, kind: ExprKind::FnValue(FnRef::Decl(self.ctx.decl_id(k))) },
                    None => {
                        self.error(span, "unresolved scoped reference");
                        unsupported(self.ty_of(span), span, "scoped reference")
                    }
                }
            }
            AExpr::Call { callee, type_args: _, args, named, .. } => self.call(callee, args, named, span),
            AExpr::ArrayLit { elems, .. } => {
                let ty = self.ty_of(span);
                let es: Vec<Expr> = elems.iter().map(|x| self.expr(x)).collect();
                let kind = if matches!(ty.strip_quals(), Ty::Array(_)) { ExprKind::Array(es) } else { ExprKind::List(es) };
                Expr { ty, span, kind }
            }
            AExpr::SetLit { .. } | AExpr::MapLit { .. } => {
                // [col-literal] resolved as a constructor call at the literal's span.
                match self.ctx.checked.call_fn.get(&self.key(span)).copied() {
                    Some(k) => {
                        let decl = self.ctx.fn_by_key(k);
                        let elems: Vec<AExpr> = match e {
                            AExpr::SetLit { elems, .. } => elems.clone(),
                            AExpr::MapLit { entries, .. } => entries
                                .iter()
                                .map(|(k, v)| AExpr::Tuple { elems: vec![k.clone(), v.clone()], span: Span::new(k.span().start, v.span().end) })
                                .collect(),
                            _ => unreachable!(),
                        };
                        let refs: Vec<&AExpr> = elems.iter().collect();
                        let type_args = self.type_args_of(span);
                        let mut all = self.args_for(decl.map(|d| Callee::Fn(k, d)), &refs, span);
                        all.extend(self.implicit_args_at(span));
                        Expr { ty: self.ty_of(span), span, kind: ExprKind::Call { target: FnRef::Decl(self.ctx.decl_id(k)), type_args, args: all } }
                    }
                    None => {
                        self.error(span, "collection literal without a resolved constructor");
                        unsupported(self.ty_of(span), span, "collection literal")
                    }
                }
            }
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
            AExpr::NonNull { operand, .. } => self.non_null(operand, span),
            AExpr::Assert { cond, message, .. } => {
                let c = self.expr(cond);
                let m = message.as_ref().map(|m| Box::new(self.expr(m)));
                Expr { ty: Ty::none(), span, kind: ExprKind::Assert { cond: Box::new(c), message: m, at: self.location(span) } }
            }
            AExpr::Unreachable { message, .. } => {
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
            AExpr::When { subject, branches, .. } => self.when_expr(subject, branches, span, true),
            AExpr::WhenCond { branches, else_block, .. } => self.when_cond_expr(branches, Some(else_block), span, true),
            AExpr::While { cond, body, else_block, .. } => {
                let result = self.fresh("__loop");
                let ty = self.ty_of(span);
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
            AExpr::Elvis { subject, pick, rhs, .. } => self.elvis(subject, pick.as_ref(), rhs, span),
            AExpr::SafeField { base, inner, .. } => self.safe_field(base, inner, span),
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
                        arms: vec![(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) }, Block { stmts, value: None })],
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
            AExpr::Spawn { handler, with_items, pool, join, .. } => {
                let key = self.key(span);
                let deps: Vec<Ty> = self.ctx.checked.spawn_deps.get(&key).map(|v| v.iter().map(erase).collect()).unwrap_or_default();
                let items: Vec<Option<usize>> = self.ctx.checked.spawn_dep_items.get(&key).cloned().unwrap_or_else(|| vec![None; deps.len()]);
                let h = match self.construct_handler(handler, &deps, &items, with_items, span) {
                    Some(h) => h,
                    None => self.expr(handler),
                };
                let pool = pool.as_ref().map(|p| Box::new(self.expr(p)));
                let join = join.as_ref().map(|j| Box::new(self.expr(j)));
                let effects: Vec<Ty> = self.ctx.checked.spawn_effects.get(&key).map(|v| v.iter().map(erase).collect()).unwrap_or_default();
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Spawn { handler: Box::new(h), deps: Vec::new(), pool, join, effects } }
            }
            AExpr::ReplyTo { member, captures, gated, pool, .. } => {
                let key = self.key(span);
                let caps: Vec<Expr> = captures.iter().map(|x| self.expr(x)).collect();
                let pool = pool.as_ref().map(|p| Box::new(self.expr(p)));
                let target = match self.ctx.checked.replyto_tasks.get(&key).copied() {
                    Some(k) => {
                        let effects: Vec<Expr> = self.ctx.checked.task_mint_effects.get(&key).cloned().unwrap_or_default().iter().map(|t| self.effect_instance(t, span)).collect();
                        ReplyTarget::Task { target: self.ctx.decl_id(k), effects }
                    }
                    None => ReplyTarget::Member(self.ctx.checked.replyto_members.get(&key).cloned().unwrap_or(member.name.clone())),
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
            AExpr::Error { .. } => unsupported(Ty::Unknown, span, "parse error"),
        }
    }

    // ------------------------------------------------------------- reads --

    /// [fn-effects] A named fn as a value. Where the fn type it is passed as
    /// threads other effects than the fn declares (a pure fn where effects
    /// are threaded — the variance rule), an adapter lambda takes the
    /// type's effects and passes on the ones the fn needs.
    fn fn_value(&mut self, k: salvo_core::FnKey, span: Span) -> Expr {
        let ty = self.ty_of(span);
        let id = self.ctx.decl_id(k);
        let taken: Vec<Ty> = self.ctx.checked.lambda_effects.get(&self.key(span)).map(|v| v.iter().map(erase).collect()).unwrap_or_default();
        let declared: Vec<Ty> = self.ctx.checked.fn_effects.get(&k).map(|v| v.iter().map(erase).collect()).unwrap_or_default();
        if taken == declared {
            return Expr { ty, span, kind: ExprKind::FnValue(FnRef::Decl(id)) };
        }
        let (params, ret) = match ty.strip_quals() {
            Ty::Fn { params, ret, .. } => (params.clone(), (**ret).clone()),
            _ => (Vec::new(), Ty::Unknown),
        };
        let mut ps: Vec<Param> = Vec::new();
        for (i, t) in taken.iter().enumerate() {
            ps.push(Param { local: Local(format!("__fx{i}")), ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None });
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
            ps.push(Param { local: l.clone(), ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None });
            args.push(Expr { ty: t.clone(), span, kind: ExprKind::Read { place: Place { root: l, steps: Vec::new() }, consume: false } });
        }
        let call = Expr { ty: ret.clone(), span, kind: ExprKind::Call { target: FnRef::Decl(id), type_args: Vec::new(), args } };
        Expr { ty, span, kind: ExprKind::Lambda { params: ps, ret, body: Block { stmts: Vec::new(), value: Some(Box::new(call)) }, captures: Vec::new() } }
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
            return self.fn_value(k, span);
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
                Ty::Named { name, .. } => self.ctx.symbols.structs.get(name.as_str()).and_then(|sd| sd.fields.iter().find(|f| f.name.name == field.name).map(|f| self.ctx.written_ty(self.file_idx, &f.ty))),
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
                arms: vec![(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) }, Block { stmts, value: Some(Box::new(value)) })],
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
        let str_ty = Ty::named("Str");
        if let Some(local) = self.ctx.checked.interp_implicit.get(&self.key(span)).cloned() {
            return Expr { ty: str_ty, span, kind: ExprKind::Call { target: FnRef::Local(Local(local)), type_args: Vec::new(), args: vec![value] } };
        }
        // [interp-union] A union is the text of the arm it holds: a switch on
        // the value, each arm calling its own `to_str`.
        if let Some(arms) = self.ctx.checked.interp_union.get(&self.key(span)).cloned() {
            return self.interp_union(value, &arms, span);
        }
        if let Some(k) = self.ctx.checked.interp_to_str.get(&self.key(span)).copied() {
            let at = Span::new(span.end, span.end);
            let mut args = vec![value];
            args.extend(self.implicit_args_at(at));
            return Expr { ty: str_ty, span, kind: ExprKind::Call { target: FnRef::Decl(self.ctx.decl_id(k)), type_args: Vec::new(), args } };
        }
        if matches!(value.ty.strip_quals(), Ty::Named { name, .. } if name == "Str") {
            return value;
        }
        // A scalar the target renders natively: the std `to_str` for it.
        match self.intrinsic_fn("to_str", &[value.ty.clone()]) {
            Some(id) => Expr { ty: str_ty, span, kind: ExprKind::Call { target: FnRef::Decl(id), type_args: Vec::new(), args: vec![value] } },
            None => {
                self.error(span, format!("no `to_str` for `{}` in an interpolation", value.ty));
                unsupported(str_ty, span, "interpolation")
            }
        }
    }

    /// [interp-union] `${u}` for a union `u`: the union's own `to_str`, given
    /// the function that renders each arm (the backend dispatches on the arm
    /// the value holds).
    fn interp_union(&mut self, value: Expr, arms: &[salvo_core::check::ArmText], span: Span) -> Expr {
        use salvo_core::check::ArmText;
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
                    let p = Param { local: Local("__s".into()), ty: arm_ty.clone(), mode: PassMode::Lent, variadic: false, check: None };
                    let read = Expr { ty: arm_ty.clone(), span, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: false } };
                    Expr { ty: fn_ty, span, kind: ExprKind::Lambda { params: vec![p], ret: str_ty.clone(), body: Block { stmts: Vec::new(), value: Some(Box::new(read)) }, captures: Vec::new() } }
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
    fn pass_member_full(&mut self, m: &salvo_core::check::PassMember, subject_ty: &Ty) -> (FnRef, Ty, Vec<Ty>) {
        match m {
            salvo_core::check::PassMember::Fn(k) => {
                let effects: Vec<Ty> = self
                    .ctx
                    .checked
                    .fn_effects
                    .get(k)
                    .map(|v| v.iter().filter(|t| !matches!(t, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT)).map(erase).collect())
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
            salvo_core::check::PassMember::Implicit(n) => {
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
                let right = Block { stmts, value: Some(Box::new(r)) };
                let id = self.id();
                let lit = |v: bool| Expr { ty: bool_ty.clone(), span, kind: ExprKind::Bool(v) };
                let (arms, otherwise) = if op == BinaryOp::And {
                    (vec![(l, right)], Some(Block { stmts: Vec::new(), value: Some(Box::new(lit(false))) }))
                } else {
                    (vec![(l, Block { stmts: Vec::new(), value: Some(Box::new(lit(true))) })], Some(right))
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
                        let ps: Vec<Param> = params.iter().enumerate().map(|(i, t)| Param { local: Local(format!("__i{i}")), ty: t.clone(), mode: PassMode::Lent, variadic: false, check: None }).collect();
                        let mut args: Vec<Expr> = ps.iter().map(|p| Expr { ty: p.ty.clone(), span: at, kind: ExprKind::Read { place: Place { root: p.local.clone(), steps: Vec::new() }, consume: false } }).collect();
                        args.extend(self.implicit_list(nested, &[], at));
                        let call = Expr { ty: ret.clone(), span: at, kind: ExprKind::Call { target: FnRef::Decl(id), type_args: Vec::new(), args } };
                        out.push(Expr { ty: want, span: at, kind: ExprKind::Lambda { params: ps, ret, body: Block { stmts: Vec::new(), value: Some(Box::new(call)) }, captures: Vec::new() } });
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
        // [throw] `throw(m)`.
        if let Some(site) = checked.may_throw.get(&key) {
            if site.performs {
                let m = args.first().map(|a| self.expr(a));
                return match m {
                    Some(m) => Expr { ty: Ty::Never, span, kind: ExprKind::Throw { message: Box::new(m) } },
                    None => unsupported(ty, span, "throw without a message"),
                };
            }
        }
        // [actor-use-addr] a send to an actor through an addr.
        if let Some(effect) = checked.addr_calls.get(&key).cloned() {
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
        if let Some(member) = checked.self_sends.get(&key).cloned().or_else(|| checked.facade_sends.get(&key).cloned()) {
            let a: Vec<Expr> = args.iter().map(|x| self.expr(x)).collect();
            return Expr { ty, span, kind: ExprKind::SelfSend { member, args: a } };
        }
        // Dot-notation [fn-dot]: the receiver is the first argument.
        let mut all_args: Vec<&AExpr> = Vec::with_capacity(args.len() + 1);
        if checked.dot_calls.contains(&key) {
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
        if let Some(mc) = checked.member_calls.get(&key).cloned() {
            let Some(interface) = self.ctx.decl_of_effect(&mc.effect) else {
                self.error(span, format!("effect `{}` has no declaration", mc.effect));
                return unsupported(ty, span, "member call");
            };
            let instance_ty = checked.effect_calls.get(&key).cloned().unwrap_or_else(|| Ty::named(mc.effect.clone()));
            let instance = self.effect_instance(&instance_ty, span);
            let decl = self.ctx.symbols.effects.get(mc.effect.as_str()).copied();
            let member_decl = decl.and_then(|d| d.fns.get(mc.index));
            let mut lowered = self.args_for(member_decl.map(Callee::Member), &all_args, span);
            lowered.extend(self.implicit_list(&checked.implicit_args.get(&key).cloned().unwrap_or_default(), named, span));
            let type_args = self.type_args_of(span);
            return Expr { ty, span, kind: ExprKind::MemberCall { instance: Box::new(instance), member: MemberRef { interface, index: mc.index }, type_args, args: lowered } };
        }
        // A fn-typed local, or the fn's own implicit [call-resolve].
        if checked.local_calls.contains(&key) {
            let name = match callee {
                AExpr::Ident(id) => id.name.clone(),
                AExpr::Field { field, .. } => field.name.clone(),
                _ => String::new(),
            };
            let local_name = checked.local_call_names.get(&key).cloned().unwrap_or(name.clone());
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
                for t in effects {
                    a.push(self.effect_instance(&t, span));
                }
            }
            let rest: Vec<Expr> = all_args.iter().map(|x| self.expr(x)).collect();
            let rest = self.mark_consumed_by_contract(rest, &key);
            a.extend(rest);
            return Expr { ty, span, kind: ExprKind::Call { target: FnRef::Local(local), type_args: Vec::new(), args: a } };
        }
        if let Some(k) = checked.call_fn.get(&key).copied() {
            let decl = self.ctx.fn_by_key(k);
            let mut lowered: Vec<Expr> = Vec::new();
            // Effects as values: the instances a callee declaring effects takes.
            if let Some(effs) = checked.call_effects.get(&key).cloned() {
                for t in effs {
                    if matches!(&t, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT) {
                        continue;
                    }
                    lowered.push(self.effect_instance(&t, span));
                }
            }
            lowered.extend(self.args_for(decl.map(|d| Callee::Fn(k, d)), &all_args, span));
            lowered.extend(self.implicit_list(&checked.implicit_args.get(&key).cloned().unwrap_or_default(), named, span));
            let type_args = self.type_args_of(span);
            return Expr { ty, span, kind: ExprKind::Call { target: FnRef::Decl(self.ctx.decl_id(k)), type_args, args: lowered } };
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
    /// one list argument.
    fn args_for(&mut self, callee: Option<Callee<'p>>, args: &[&AExpr], span: Span) -> Vec<Expr> {
        let modes = salvo_core::param_mode::Modes { symbols: self.ctx.symbols, checked: self.ctx.checked, program: self.ctx.program };
        let params: Option<&[ast::Param]> = callee.as_ref().map(|c| match c { Callee::Fn(_, d) => d.params.as_slice(), Callee::Member(m) => m.params.as_slice() });
        let fixed: Vec<&ast::Param> = params.map(|ps| ps.iter().filter(|p| !p.implicit).collect()).unwrap_or_default();
        let variadic_at = fixed.iter().position(|p| p.variadic);
        let mut out = Vec::new();
        let mut tail: Vec<Expr> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let is_tail = variadic_at.is_some_and(|v| i >= v);
            let mut x = self.expr(a);
            let param = if is_tail { fixed.get(variadic_at.unwrap()) } else { fixed.get(i) };
            if let Some(p) = param {
                let mode = match &callee {
                    Some(Callee::Fn(k, _)) => modes.fn_param(Some(*k), p),
                    Some(Callee::Member(m)) => modes.member_param(m, p),
                    None => modes.default_param(&p.ty, p.variadic),
                };
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
                        return out;
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
        out
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
                let fty = self.ctx.written_ty(self.file_idx, &fd.ty);
                all.push((name.clone(), Expr { ty: fty, span, kind: ExprKind::Read { place: Place { root: src.clone(), steps: vec![Step::Field(name.clone())] }, consume: true } }));
            } else if let Some(d) = &fd.default {
                all.push((name.clone(), self.expr(d)));
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
            for (i, t) in effs.iter().enumerate() {
                let local = Local(format!("__leff{i}"));
                let t = erase(t);
                self.effect_env.push((t.clone(), local.clone()));
                ps.push(Param { local, ty: t, mode: PassMode::Lent, variadic: false, check: None });
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
            ps.push(Param { local, ty: pty, mode, variadic: false, check: None });
        }
        let block = match body {
            LambdaBody::Expr(e) => {
                let v = self.expr(e);
                Block { stmts: Vec::new(), value: Some(Box::new(v)) }
            }
            LambdaBody::Block(b) => self.block(b, !ret.is_none_ty() && !ret.is_unknown()),
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

    fn qualifies_call(&mut self, check: &salvo_core::check::PredicateCheck, subject: Expr, span: Span) -> Expr {
        let bool_ty = Ty::named("Bool");
        let Some(target) = self.qualifies_ref(&check.name, &subject.ty) else {
            self.error(span, format!("qualifier `{}` has no `qualifies`", check.name));
            return unsupported(bool_ty, span, "qualifies");
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
            let parts: Vec<&str> = a.path.split('.').collect();
            match self.lookup(parts[0]).cloned() {
                Some(b) => {
                    let steps = parts[1..].iter().map(|s| Step::Field(s.to_string())).collect();
                    args.push(Expr { ty: Ty::Unknown, span, kind: ExprKind::Read { place: Place { root: b.local, steps }, consume: false } });
                }
                // [qual-value-args] a literal argument, as spelled.
                None => match literal_arg(&a.path) {
                    Some(e) => args.push(Expr { span, ..e }),
                    None => args.push(unsupported(Ty::Unknown, span, format!("qualifier argument `{}`", a.path))),
                },
            }
        }
        if let Some(at) = check.implicits_at {
            args.extend(self.implicit_args_at(at));
        }
        Expr { ty: bool_ty, span, kind: ExprKind::Call { target: FnRef::Decl(target), type_args: Vec::new(), args } }
    }

    fn qualifies_ref(&self, name: &str, subject: &Ty) -> Option<DeclId> {
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
                many.iter().copied().find(|d| salvo_core::refine::of_base(&d.of, &d.generics).is_some_and(|of| of == base)).or_else(|| candidates.first().copied())?
            }
        };
        let fi = decl.fns.iter().position(|f| f.name.name == "qualifies")?;
        for (file_idx, m) in self.ctx.program.modules.iter().enumerate() {
            if let Some(item) = m.items.iter().position(|i| matches!(i, ast::Item::Qualifier(q) if std::ptr::eq(q, decl))) {
                return Some(DeclId { module: self.ctx.program.files[file_idx].module.clone(), item, sub: fi as u16 + 1 });
            }
        }
        None
    }

    fn non_null(&mut self, operand: &AExpr, span: Span) -> Expr {
        // `x!`: switch on x; `None` traps; else the narrowed value.
        let ty = self.ty_of(span);
        let s = self.expr_subject(operand);
        let id = self.id();
        let subj_ty = s.ty.clone();
        let tmp = self.fresh("__nn");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: s };
        let msg = "value is absent".to_string();
        let trap = Block { stmts: vec![Stmt::Expr(Expr { ty: Ty::Never, span, kind: ExprKind::Unreachable { message: Some(Box::new(Expr { ty: Ty::named("Str"), span, kind: ExprKind::Str(msg) })), at: self.location(span) } })], value: None };
        let nar = self.fresh("__some");
        let nid = self.id();
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Arm { switch: id, arm: 1 } };
        let value = Expr { ty: ty.clone(), span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: true } };
        let sw = Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::Switch {
                id,
                subject: Box::new(Expr { ty: subj_ty, span, kind: ExprKind::Read { place: Place { root: tmp, steps: Vec::new() }, consume: false } }),
                arms: vec![SwitchArm { test: ArmTest::None, body: trap }, SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![narrow], value: Some(Box::new(value)) } }],
            },
        };
        self.block_expr(vec![bind], sw, span)
    }

    fn elvis(&mut self, subject: &AExpr, pick: Option<&ast::ElvisPick>, rhs: &AExpr, span: Span) -> Expr {
        let ty = self.ty_of(span);
        if pick.is_some() {
            // [pick] `s ^Q?: r`: the picked arm is the value; the rest goes
            // to the right side, where `_` is what remains.
            let test = self.ctx.checked.is_tests.get(&self.key(span)).cloned();
            let s = self.expr_subject(subject);
            let subj_ty = s.ty.clone();
            let tmp = self.fresh("__pick");
            let lid = self.id();
            let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: s };
            let id = self.id();
            let picked_ty = self.ctx.checked.elvis_picks.get(&self.key(span)).map(erase).unwrap_or(Ty::Unknown);
            let rest_ty = self.ctx.checked.pick_left.get(&self.key(span)).map(erase).unwrap_or(Ty::Unknown);
            let nar = self.fresh("__picked");
            let nid = self.id();
            let picked_ty_c = picked_ty.clone();
            let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: picked_ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Arm { switch: id, arm: 0 } };
            let value = Expr { ty: picked_ty, span, kind: ExprKind::Read { place: Place { root: nar.clone(), steps: Vec::new() }, consume: true } };
            let rest = self.fresh("__rest");
            let rid = self.id();
            let rest_c = rest.clone();
            let rest_ty_c = rest_ty.clone();
            let rest_narrow = Stmt::Narrow { id: rid, local: rest.clone(), ty: rest_ty, from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Arm { switch: id, arm: 1 } };
            let saved = self.placeholder.replace(rest);
            let r = self.expr(rhs);
            self.placeholder = saved;
            if let Some(checks) = self.ctx.checked.predicate_tests.get(&self.key(span)).cloned() {
                // [qual-predicate] a predicate pick: a branch on the
                // `qualifies` conjunction, the rest in `otherwise`.
                let read_tmp = |l: &Local| Expr { ty: subj_ty.clone(), span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false } };
                let mut cond: Option<Expr> = None;
                for check in checks {
                    let call = self.qualifies_call(&check, read_tmp(&tmp), span);
                    cond = Some(match cond {
                        None => call,
                        Some(prev) => Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::And, args: vec![prev, call] } },
                    });
                }
                let cond = cond.unwrap_or(Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) });
                let narrow = Stmt::Narrow { id: nid, local: nar, ty: picked_ty_c.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Cond { branch: id, arm: 0 } };
                let rest_narrow = Stmt::Narrow { id: rid, local: rest_c, ty: rest_ty_c, from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::After { branch: id, arm: 0 } };
                let br = Expr {
                    ty: ty.clone(),
                    span,
                    kind: ExprKind::Branch {
                        id,
                        arms: vec![(cond, Block { stmts: vec![narrow], value: Some(Box::new(value)) })],
                        otherwise: Some(Block { stmts: vec![rest_narrow], value: Some(Box::new(r)) }),
                    },
                };
                return self.block_expr(vec![bind], br, span);
            }
            let arm_test_ = match &test {
                Some(t) => arm_test(t),
                None => ArmTest::Else,
            };
            let sw = Expr {
                ty: ty.clone(),
                span,
                kind: ExprKind::Switch {
                    id,
                    subject: Box::new(Expr { ty: subj_ty, span, kind: ExprKind::Read { place: Place { root: tmp, steps: Vec::new() }, consume: false } }),
                    arms: vec![
                        SwitchArm { test: arm_test_, body: Block { stmts: vec![narrow], value: Some(Box::new(value)) } },
                        SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![rest_narrow], value: Some(Box::new(r)) } },
                    ],
                },
            };
            return self.block_expr(vec![bind], sw, span);
        }
        // `s ?: r`: switch on s; `None` → r (with `_` bound to the subject); else the value.
        let s = self.expr_subject(subject);
        let subj_ty = s.ty.clone();
        let tmp = self.fresh("__elv");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: s };
        let id = self.id();
        let saved = self.placeholder.replace(tmp.clone());
        let r = self.expr(rhs);
        self.placeholder = saved;
        let picked_ty = self.ctx.checked.elvis_picks.get(&self.key(span)).map(erase).unwrap_or_else(|| subj_ty.without_none());
        let nar = self.fresh("__some");
        let nid = self.id();
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: picked_ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Arm { switch: id, arm: 1 } };
        let value = Expr { ty: picked_ty, span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: true } };
        let sw = Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::Switch {
                id,
                subject: Box::new(Expr { ty: subj_ty, span, kind: ExprKind::Read { place: Place { root: tmp, steps: Vec::new() }, consume: false } }),
                arms: vec![
                    SwitchArm { test: ArmTest::None, body: Block { stmts: Vec::new(), value: Some(Box::new(r)) } },
                    SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![narrow], value: Some(Box::new(value)) } },
                ],
            },
        };
        self.block_expr(vec![bind], sw, span)
    }

    fn safe_field(&mut self, base: &AExpr, inner: &AExpr, span: Span) -> Expr {
        // `b?.f`: switch on b; `None` → None; else the inner, with `b` narrowed.
        let ty = self.ty_of(span);
        let s = self.expr_subject(base);
        let subj_ty = s.ty.clone();
        let tmp = self.fresh("__safe");
        let lid = self.id();
        let bind = Stmt::Let { id: lid, local: tmp.clone(), ty: subj_ty.clone(), value: s };
        let id = self.id();
        let nar = self.fresh("__some");
        let nid = self.id();
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: subj_ty.without_none(), from: Place { root: tmp.clone(), steps: Vec::new() }, from_ty: subj_ty.clone(), because: Justification::Arm { switch: id, arm: 1 } };
        // The inner expression reads the base: bind it under the base's name.
        self.push_scope();
        if let AExpr::Ident(b) = base {
            self.rebind(&b.name, nar.clone(), subj_ty.without_none());
        }
        let alias = self.stored_place_of(base).filter(|p| !p.steps.is_empty());
        if let Some(p) = &alias {
            self.place_aliases.push((p.clone(), nar.clone(), subj_ty.without_none()));
        }
        let in_ = self.expr(inner);
        if alias.is_some() {
            self.place_aliases.pop();
        }
        self.pop_scope();
        let none = Expr { ty: ty.clone(), span, kind: ExprKind::MakeNone };
        let sw = Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::Switch {
                id,
                subject: Box::new(Expr { ty: subj_ty, span, kind: ExprKind::Read { place: Place { root: tmp, steps: Vec::new() }, consume: false } }),
                arms: vec![
                    SwitchArm { test: ArmTest::None, body: Block { stmts: Vec::new(), value: Some(Box::new(none)) } },
                    SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![narrow], value: Some(Box::new(in_)) } },
                ],
            },
        };
        self.block_expr(vec![bind], sw, span)
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
                    (true, (stmts, e)) => Block { stmts, value: Some(Box::new(e)) },
                    (false, (mut stmts, e)) => {
                        stmts.push(Stmt::Expr(e));
                        Block { stmts, value: None }
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
                otherwise = else_block.map(|b| self.block(b, value));
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
            otherwise = else_block.map(|b| self.block(b, value));
        } else {
            let e = self.if_expr(&branches[1..], else_block, span, value);
            otherwise = Some(if value { Block { stmts: Vec::new(), value: Some(Box::new(e)) } } else { Block { stmts: vec![Stmt::Expr(e)], value: None } });
        }
        self.set_last_branch(id);
        (pending, Expr { ty, span, kind: ExprKind::Branch { id, arms, otherwise } })
    }

    pub(crate) fn when_cond_expr(&mut self, branches: &[(AExpr, ast::Block)], else_block: Option<&ast::Block>, span: Span, value: bool) -> Expr {
        self.if_expr(branches, else_block, span, value)
    }

    pub(crate) fn when_expr(&mut self, subject: &AExpr, branches: &[ast::WhenBranch], span: Span, value: bool) -> Expr {
        let ty = if value { self.ty_of(span) } else { Ty::none() };
        let s = self.expr_subject(subject);
        let subj_ty = s.ty.clone();
        let id = self.id();
        let place = self.stored_place_of(subject);
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
        self.set_last_branch(id);
        Expr { ty, span, kind: ExprKind::Switch { id, subject: Box::new(s), arms } }
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
        let saved_pending = std::mem::take(&mut self.pending);
        let c = self.condition(cond);
        stmts.append(&mut self.pending);
        self.pending = saved_pending;
        let narrows = std::mem::take(&mut self.pending_after_test);
        let not = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Op { op: Op::Not, args: vec![c] } };
        let bid = self.id();
        stmts.push(Stmt::Expr(Expr { ty: Ty::none(), span, kind: ExprKind::Branch { id: bid, arms: vec![(not, Block { stmts: vec![Stmt::Break], value: None })], otherwise: None } }));
        stmts.extend(narrows);
        if let Some(r) = &ran {
            stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) } });
        }
        let b = self.with_test(Some(Justification::Cond { branch: bid, arm: 0 }), |me| me.block(body, result.is_some()));
        stmts.extend(b.stmts);
        match (b.value, &result) {
            (Some(v), Some((r, _))) if loop_body_value(&v) => stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v }),
            (Some(v), _) => stmts.push(Stmt::Expr(*v)),
            _ => {}
        }
        self.loop_results.pop();
        self.loops -= 1;
        self.pop_scope();
        out.push(Stmt::Loop { id: loop_id, body: Block { stmts, value: None } });
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
                elem_ty = elem_ty.remove_quals(&std::iter::once("proj".to_string()).collect());
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
                            SwitchArm { test: ArmTest::Arm(d.emitted_arm), body: Block { stmts: arm_stmts, value: None } },
                            SwitchArm { test: ArmTest::Else, body: Block { stmts: vec![Stmt::Break], value: None } },
                        ],
                    },
                }));
                self.loop_results.pop();
                self.loops -= 1;
                self.pop_scope();
                out.push(Stmt::Loop { id: loop_id, body: Block { stmts, value: None } });
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
                out.push(Stmt::ForEach { id: loop_id, local, ty: elem_ty, iterable: subject, body: Block { stmts, value: None } });
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

    pub(crate) fn use_stmt(&mut self, handler: &AExpr, with_items: &[AExpr], span: Span) -> Vec<Stmt> {
        let key = self.key(span);
        let faces: Vec<Ty> = self.ctx.checked.use_effects.get(&key).map(|v| v.iter().map(erase).collect()).unwrap_or_default();
        // `use addr`: the addr is the instance.
        if let Some(effect) = self.ctx.checked.use_addrs.get(&key).map(erase) {
            let a = self.expr(handler);
            let local = self.fresh("__use");
            let id = self.id();
            self.effect_env.push((effect.clone(), local.clone()));
            let inst = Expr { ty: effect.clone(), span, kind: ExprKind::AddrInstance { addr: Box::new(a) } };
            return vec![Stmt::Let { id, local, ty: effect, value: inst }];
        }
        // `use H(args)`: a construct, then one instance per face.
        let deps: Vec<Ty> = self.ctx.checked.use_deps.get(&key).map(|v| v.iter().map(erase).collect()).unwrap_or_default();
        let items: Vec<Option<usize>> = self.ctx.checked.use_with_items.get(&key).cloned().unwrap_or_else(|| vec![None; deps.len()]);
        let Some(construct) = self.construct_handler(handler, &deps, &items, with_items, span) else {
            self.error(span, "`use` of something that is not a handler construction");
            return vec![Stmt::Expr(unsupported(Ty::Unknown, span, "use"))];
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
        for f in faces {
            let h = self.fresh("__handle");
            let hid = self.id();
            let read = Expr { ty: handler_ty.clone(), span, kind: ExprKind::Read { place: Place { root: inst.clone(), steps: Vec::new() }, consume: false } };
            let value = if threadsafe { Expr { ty: f.clone(), ..read } } else { Expr { ty: f.clone(), span, kind: ExprKind::Handle { instance: Box::new(read) } } };
            out.push(Stmt::Let { id: hid, local: h.clone(), ty: f.clone(), value });
            self.effect_env.push((f, h));
        }
        out
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
    Fn(salvo_core::FnKey, &'p ast::FnDecl),
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
        ExprKind::MakeUnion { value, .. } | ExprKind::Rewrap { value, .. } | ExprKind::DropMut { value } | ExprKind::Widen { value } | ExprKind::Present { value } | ExprKind::Spread { value } => visit_reads(value, f),
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
