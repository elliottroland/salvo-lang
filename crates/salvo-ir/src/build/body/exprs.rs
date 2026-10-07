//! Expressions (IR.md §2): every construct lowered to the node set, every
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
                let kind = if *long { ExprKind::Long(*value) } else { ExprKind::Int(*value) };
                self.lit(span, kind)
            }
            AExpr::Float { value, single, .. } => {
                let kind = if *single { ExprKind::Float(*value) } else { ExprKind::Double(*value) };
                self.lit(span, kind)
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
                        let args: Vec<Expr> = match e {
                            AExpr::SetLit { elems, .. } => elems.iter().map(|x| self.expr(x)).collect(),
                            AExpr::MapLit { entries, .. } => entries
                                .iter()
                                .map(|(k, v)| {
                                    let kk = self.expr(k);
                                    let vv = self.expr(v);
                                    let ty = Ty::Tuple(vec![kk.ty.clone(), vv.ty.clone()]);
                                    Expr { ty, span: k.span(), kind: ExprKind::Tuple(vec![kk, vv]) }
                                })
                                .collect(),
                            _ => unreachable!(),
                        };
                        let type_args = self.type_args_of(span);
                        let mut all = args;
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
                let es = elems.iter().map(|x| self.expr(x)).collect();
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Tuple(es) }
            }
            AExpr::StructLit { fields, .. } => self.struct_lit(fields, span),
            AExpr::Unary { op, operand, .. } => {
                let x = self.expr(operand);
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
                    let from = self.place_of(subject).unwrap_or(Place { root: Local("<subject>".into()), steps: Vec::new() });
                    let local = Local(b.name.clone());
                    self.rebind(&b.name, local.clone(), ty.clone());
                    let nid = self.id();
                    self.pending_after_test.push(Stmt::Narrow { id: nid, local, ty, from, because: Justification::Test { test: id } });
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
                Expr { ty: Ty::none(), span, kind: ExprKind::Assert { cond: Box::new(c), message: m } }
            }
            AExpr::Unreachable { message, .. } => {
                let m = message.as_ref().map(|m| Box::new(self.expr(m)));
                Expr { ty: self.ty_of(span), span, kind: ExprKind::Unreachable { message: m } }
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
                self.block_value(stmts, result, ty, span)
            }
            AExpr::For { pattern, iterable, body, else_block, .. } => {
                let result = self.fresh("__loop");
                let ty = self.ty_of(span);
                let stmts = self.for_stmt(pattern, iterable, body, else_block.as_ref(), span, Some((result.clone(), ty.clone())));
                self.block_value(stmts, result, ty, span)
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
                let caps: Vec<Expr> = captures.iter().map(|x| self.expr(x)).collect();
                let pool = pool.as_ref().map(|p| Box::new(self.expr(p)));
                Expr { ty: self.ty_of(span), span, kind: ExprKind::ReplyTo { member: member.name.clone(), captures: caps, gated: *gated, pool } }
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
            return Expr { ty: self.ty_of(span), span, kind: ExprKind::FnValue(FnRef::Decl(self.ctx.decl_id(k))) };
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
        let consume = self.ctx.checked.linear_moves.contains(&key)
            || self.ctx.checked.state_takes.contains(&key)
            || self.ctx.checked.moved_projections.contains(&key);
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

    fn binary(&mut self, op: BinaryOp, lhs: &AExpr, rhs: &AExpr, span: Span) -> Expr {
        let ty = self.ty_of(span);
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let l = self.expr(lhs);
                let r = self.expr(rhs);
                Expr { ty, span, kind: ExprKind::Op { op: if op == BinaryOp::And { Op::And } else { Op::Or }, args: vec![l, r] } }
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                let l = self.expr(lhs);
                let r = self.expr(rhs);
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
                let l = self.expr(lhs);
                let r = self.expr(rhs);
                let equality = matches!(op, BinaryOp::Eq | BinaryOp::NotEq);
                // `x == None` is a `None` test of the other side.
                if equality && (l.ty.is_none_ty() || r.ty.is_none_ty()) {
                    let subject = if r.ty.is_none_ty() { l } else { r };
                    let id = self.id();
                    let test = Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Test { id, subject: Box::new(subject), test: ArmTest::None } };
                    return if op == BinaryOp::Eq { test } else { Expr { ty, span, kind: ExprKind::Op { op: Op::Not, args: vec![test] } } };
                }
                let via = self.ctx.checked.comparisons.get(&self.key(span)).cloned();
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
                        let ps: Vec<Param> = params.iter().enumerate().map(|(i, t)| Param { local: Local(format!("__i{i}")), ty: t.clone(), mode: PassMode::Lent, variadic: false }).collect();
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
            let local = self
                .implicit_locals
                .get(&local_name)
                .cloned()
                .or_else(|| self.lookup(&local_name).map(|b| b.local.clone()))
                .or_else(|| self.lookup(&name).map(|b| b.local.clone()));
            let Some(local) = local else {
                self.error(span, format!("call through unknown local `{name}`"));
                return unsupported(ty, span, "local call");
            };
            // The contract says how each position is taken.
            let a: Vec<Expr> = all_args.iter().map(|x| self.expr(x)).collect();
            let a = self.mark_consumed_by_contract(a, &key);
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
                if mode == PassMode::Moved || is_tail {
                    self.consume_if_owned(&mut x);
                }
            }
            if is_tail {
                if matches!(a, AExpr::Spread { .. }) {
                    // `...xs` passes the list itself.
                    out.push(x);
                    return out;
                }
                tail.push(x);
            } else {
                out.push(x);
            }
        }
        if variadic_at.is_some() {
            // [fn-variadic] the tail is one list, typed by its elements.
            let ety = tail.first().map(|e| e.ty.clone()).unwrap_or(Ty::Unknown);
            let list_ty = Ty::Named { name: "List".to_string(), args: vec![ety] };
            out.push(Expr { ty: list_ty, span, kind: ExprKind::List(tail) });
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

    fn struct_lit(&mut self, fields: &[ast::StructLitField], span: Span) -> Expr {
        let ty = self.ty_of(span);
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
                ps.push(Param { local, ty: t, mode: PassMode::Lent, variadic: false });
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
            ps.push(Param { local, ty: pty, mode, variadic: false });
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
        Expr { ty, span, kind: ExprKind::Lambda { params: ps, ret, body: block, captures } }
    }

    // --------------------------------------------------------- narrowing --

    /// `x is T [name]` as a condition.
    fn is_test(&mut self, subject: &AExpr, binding: Option<&ast::Ident>, span: Span) -> Expr {
        let bool_ty = Ty::named("Bool");
        if let Some(checks) = self.ctx.checked.predicate_tests.get(&self.key(span)).cloned() {
            // [qual-predicate] a conjunction of `qualifies` calls.
            let subj_ty = self.ty_of(subject.span());
            let mut acc: Option<Expr> = None;
            for check in checks {
                let call = self.qualifies_call(&check, subject, &subj_ty, span);
                acc = Some(match acc {
                    None => call,
                    Some(prev) => Expr { ty: bool_ty.clone(), span, kind: ExprKind::Op { op: Op::And, args: vec![prev, call] } },
                });
            }
            if let (Some(b), Some(place)) = (binding, self.place_of(subject)) {
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
        let s = self.expr_subject(subject);
        let id = self.id();
        let arm_ty = self.narrowed_ty_of_test(&s.ty, &test);
        let place = self.place_of(subject);
        if let Some(b) = binding {
            let from = place.clone().unwrap_or(Place { root: Local("<subject>".into()), steps: Vec::new() });
            let ty = self.ty_of(b.span).or_unknown(arm_ty.clone()).unwrap_or(Ty::Unknown);
            let local = Local(b.name.clone());
            self.rebind(&b.name, local.clone(), ty.clone());
            let nid = self.id();
            self.pending_after_test.push(Stmt::Narrow { id: nid, local, ty, from, because: Justification::Test { test: id } });
        } else if let (Some(place), AExpr::Ident(name)) = (place, subject) {
            if let Some(t) = arm_ty {
                let narrow = self.narrow_in_arm(&name.name, place, t, Justification::Test { test: id });
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
        self.expr(subject)
    }

    /// The type a successful test narrows the subject to.
    pub(crate) fn narrowed_ty_of_test(&self, subject: &Ty, test: &UnionTest) -> Option<Ty> {
        if test.match_none && test.arms.is_empty() {
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

    fn qualifies_call(&mut self, check: &salvo_core::check::PredicateCheck, subject: &AExpr, subj_ty: &Ty, span: Span) -> Expr {
        let bool_ty = Ty::named("Bool");
        let Some(target) = self.qualifies_ref(&check.name, subj_ty) else {
            self.error(span, format!("qualifier `{}` has no `qualifies`", check.name));
            return unsupported(bool_ty, span, "qualifies");
        };
        let mut args = vec![self.expr(subject)];
        for a in &check.args {
            let parts: Vec<&str> = a.path.split('.').collect();
            match self.lookup(parts[0]).cloned() {
                Some(b) => {
                    let steps = parts[1..].iter().map(|s| Step::Field(s.to_string())).collect();
                    args.push(Expr { ty: Ty::Unknown, span, kind: ExprKind::Read { place: Place { root: b.local, steps }, consume: false } });
                }
                None => args.push(unsupported(Ty::Unknown, span, format!("qualifier argument `{}`", a.path))),
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
        let msg = format!("salvo: value is absent at {}:{}", self.ctx.program.files[self.file_idx].name, span.start);
        let trap = Block { stmts: vec![Stmt::Expr(Expr { ty: Ty::Never, span, kind: ExprKind::Unreachable { message: Some(Box::new(Expr { ty: Ty::named("Str"), span, kind: ExprKind::Str(msg) })) } })], value: None };
        let nar = self.fresh("__some");
        let nid = self.id();
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, because: Justification::Arm { switch: id, arm: 1 } };
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
            let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: picked_ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, because: Justification::Arm { switch: id, arm: 0 } };
            let value = Expr { ty: picked_ty, span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: true } };
            let rest = self.fresh("__rest");
            let rid = self.id();
            let rest_narrow = Stmt::Narrow { id: rid, local: rest.clone(), ty: rest_ty, from: Place { root: tmp.clone(), steps: Vec::new() }, because: Justification::Arm { switch: id, arm: 1 } };
            let saved = self.placeholder.replace(rest);
            let r = self.expr(rhs);
            self.placeholder = saved;
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
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: picked_ty.clone(), from: Place { root: tmp.clone(), steps: Vec::new() }, because: Justification::Arm { switch: id, arm: 1 } };
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
        let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: subj_ty.without_none(), from: Place { root: tmp.clone(), steps: Vec::new() }, because: Justification::Arm { switch: id, arm: 1 } };
        // The inner expression reads the base: bind it under the base's name.
        self.push_scope();
        if let AExpr::Ident(b) = base {
            self.rebind(&b.name, nar.clone(), subj_ty.without_none());
        }
        let in_ = self.expr(inner);
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
        for (i, (cond, block)) in branches.iter().enumerate() {
            self.push_scope();
            let c = self.expr(cond);
            // Bindings the test introduced are in scope for the arm.
            let narrows = std::mem::take(&mut self.pending_after_test);
            let mut b = self.with_test(Some(Justification::Cond { branch: id, arm: i }), |me| me.block(block, value));
            let mut stmts = narrows;
            stmts.append(&mut b.stmts);
            b.stmts = stmts;
            self.pop_scope();
            arms.push((c, b));
        }
        let otherwise = else_block.map(|b| self.block(b, value));
        self.set_last_branch(id);
        Expr { ty, span, kind: ExprKind::Branch { id, arms, otherwise } }
    }

    pub(crate) fn when_cond_expr(&mut self, branches: &[(AExpr, ast::Block)], else_block: Option<&ast::Block>, span: Span, value: bool) -> Expr {
        self.if_expr(branches, else_block, span, value)
    }

    pub(crate) fn when_expr(&mut self, subject: &AExpr, branches: &[ast::WhenBranch], span: Span, value: bool) -> Expr {
        let ty = if value { self.ty_of(span) } else { Ty::none() };
        let s = self.expr_subject(subject);
        let subj_ty = s.ty.clone();
        let id = self.id();
        let place = self.place_of(subject);
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
                if let Some(arm_ty) = self.narrowed_ty_of_test(&subj_ty, t) {
                    let name = br.binding.as_ref().map(|b| b.name.clone()).or_else(|| match subject { AExpr::Ident(n) => Some(n.name.clone()), _ => None });
                    if let Some(name) = name {
                        if br.binding.is_some() {
                            let local = Local(name.clone());
                            let bty = self.ty_of(br.binding.as_ref().unwrap().span).or_unknown(Some(arm_ty.clone())).unwrap_or(arm_ty.clone());
                            self.rebind(&name, local.clone(), bty.clone());
                            let nid = self.id();
                            stmts.push(Stmt::Narrow { id: nid, local, ty: bty, from: place, because: Justification::Arm { switch: id, arm: i } });
                        } else {
                            stmts.push(self.narrow_in_arm(&name, place, arm_ty, Justification::Arm { switch: id, arm: i }));
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
        let c = self.expr(cond);
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
        if let (Some(v), Some((r, _))) = (b.value, &result) {
            stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v });
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
        let subject = self.expr(iterable);
        let elem_ty = match pattern {
            Pattern::Ident(id) => self.ty_of(id.span),
            other => self.ty_of(pattern_span(other)),
        };
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
        match driver {
            Some(d) => {
                // The pass: minted, or the subject itself.
                let pass = self.fresh("__pass");
                let pass_ty = subject.ty.clone();
                let minted = match &d.mint {
                    Some(m) => {
                        let target = match m {
                            salvo_core::check::PassMember::Fn(k) => FnRef::Decl(self.ctx.decl_id(*k)),
                            salvo_core::check::PassMember::Implicit(n) => FnRef::Local(self.implicit_locals.get(n).cloned().unwrap_or(Local(n.clone()))),
                        };
                        Expr { ty: Ty::Unknown, span, kind: ExprKind::Call { target, type_args: Vec::new(), args: vec![subject] } }
                    }
                    None => subject,
                };
                let mid = self.id();
                out.push(Stmt::Let { id: mid, local: pass.clone(), ty: minted.ty.clone().or_unknown(Some(pass_ty)).unwrap_or(Ty::Unknown), value: minted });
                let step = self.fresh("__step");
                let next_target = match &d.next {
                    salvo_core::check::PassMember::Fn(k) => FnRef::Decl(self.ctx.decl_id(*k)),
                    salvo_core::check::PassMember::Implicit(n) => FnRef::Local(self.implicit_locals.get(n).cloned().unwrap_or(Local(n.clone()))),
                };
                let step_ty = Ty::Union(vec![elem_ty.clone(), Ty::named("Finished")]);
                let sid = self.id();
                stmts.push(Stmt::Let { id: sid, local: step.clone(), ty: step_ty.clone(), value: Expr { ty: step_ty.clone(), span, kind: ExprKind::Call { target: next_target, type_args: Vec::new(), args: vec![Expr { ty: Ty::Unknown, span, kind: ExprKind::Read { place: Place { root: pass.clone(), steps: Vec::new() }, consume: false } }] } } });
                let swid = self.id();
                let nar = self.fresh("__emitted");
                let nid = self.id();
                let narrow = Stmt::Narrow { id: nid, local: nar.clone(), ty: elem_ty.clone(), from: Place { root: step.clone(), steps: Vec::new() }, because: Justification::Arm { switch: swid, arm: 0 } };
                let mut arm_stmts = vec![narrow];
                bind_pattern(self, Expr { ty: elem_ty.clone(), span, kind: ExprKind::Read { place: Place { root: nar, steps: Vec::new() }, consume: true } }, &mut arm_stmts);
                if let Some(r) = &ran {
                    arm_stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: Expr { ty: Ty::named("Bool"), span, kind: ExprKind::Bool(true) } });
                }
                let b = self.block(body, result.is_some());
                arm_stmts.extend(b.stmts);
                if let (Some(v), Some((r, _))) = (b.value, &result) {
                    arm_stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v });
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
                if let (Some(v), Some((r, _))) = (b.value, &result) {
                    stmts.push(Stmt::Assign { place: Place { root: r.clone(), steps: Vec::new() }, value: *v });
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
            return vec![Stmt::Let { id, local, ty: effect, value: a }];
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
        let out = vec![Stmt::Let { id, local: inst.clone(), ty: handler_ty, value: construct }];
        for f in faces {
            self.effect_env.push((f, inst.clone()));
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
            let v = match items.get(i).copied().flatten().and_then(|w| with_items.get(w)) {
                Some(item) => self.with_item(item),
                None => self.effect_instance(d, span),
            };
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
    if t.match_none && t.arms.is_empty() {
        return ArmTest::None;
    }
    if !t.values.is_empty() {
        let lits: Vec<Lit> = t
            .values
            .iter()
            .flat_map(|(_, _, ls)| ls.iter())
            .map(|l| match l {
                ast::TypeLit::Str(s) => Lit::Str(s.clone()),
                ast::TypeLit::Int(i) => Lit::Int(*i),
                ast::TypeLit::Long(i) => Lit::Long(*i),
                ast::TypeLit::Bool(b) => Lit::Bool(*b),
            })
            .collect();
        return ArmTest::Lit(lits);
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
