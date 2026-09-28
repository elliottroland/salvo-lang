//! A mutable, pre-order visitor over the syntax tree — every expression,
//! statement, type, pattern and span a declaration holds.
//!
//! Written for the comptime expansion [comptime-instantiate], which needs two
//! whole-tree rewrites of a cloned body: substituting the binders and the
//! bound type parameter, and moving every span into a fresh region so two
//! unrolled copies never share one (the checker's side tables are keyed by
//! span — see the `Spans` note in `desugar.rs` for the afternoon that cost).
//! The traversal is exhaustive over the AST by construction: a new variant is
//! a compile error here, which is the point.

use crate::ast::*;
use crate::span::Span;

/// The hooks. Each is called **before** the walk descends into the node, so a
/// hook may replace the node wholesale and the walk continues into the
/// replacement.
pub trait MutVisitor {
    fn visit_span(&mut self, _span: &mut Span) {}
    fn visit_ident(&mut self, ident: &mut Ident) {
        self.visit_span(&mut ident.span);
    }
    fn visit_expr(&mut self, _expr: &mut Expr) {}
    fn visit_stmt(&mut self, _stmt: &mut Stmt) {}
    fn visit_type(&mut self, _ty: &mut Type) {}
    fn visit_type_ref(&mut self, _ty: &mut TypeRef) {}
    fn visit_block(&mut self, _block: &mut Block) {}
    fn visit_struct_lit_field(&mut self, _field: &mut StructLitField) {}
}

pub fn walk_fn<V: MutVisitor>(v: &mut V, f: &mut FnDecl) {
    v.visit_span(&mut f.span);
    v.visit_ident(&mut f.name);
    if let Some(s) = &mut f.scoped_to {
        v.visit_ident(s);
    }
    for g in &mut f.generics {
        v.visit_ident(g);
    }
    for (g, q) in &mut f.generic_canbe {
        v.visit_ident(g);
        walk_type_ref(v, q);
    }
    if let Some(d) = &mut f.derived_return {
        v.visit_ident(d);
    }
    for p in &mut f.params {
        v.visit_span(&mut p.span);
        v.visit_ident(&mut p.name);
        walk_type(v, &mut p.ty);
    }
    for g in &mut f.implicit_groups {
        walk_type_ref(v, g);
    }
    if let Some(effects) = &mut f.effects {
        for e in effects {
            walk_effect_ref(v, e);
        }
    }
    if let Some(ds) = &mut f.deductions {
        for d in ds {
            walk_deduction(v, d);
        }
    }
    if let Some(t) = &mut f.return_type {
        walk_type(v, t);
    }
    if let Some(c) = &mut f.constructs {
        walk_type_ref(v, c);
    }
    for st in &mut f.iter_state {
        walk_field_decl(v, st);
    }
    if let Some(b) = &mut f.body {
        walk_block(v, b);
    }
    if let Some(by) = &mut f.by {
        v.visit_span(&mut by.span);
        for p in &mut by.path {
            v.visit_ident(p);
        }
    }
    if let Some(c) = &mut f.compfn {
        v.visit_span(&mut c.span);
        if let Some((_, id)) = &mut c.bound {
            v.visit_ident(id);
        }
    }
}

pub fn walk_field_decl<V: MutVisitor>(v: &mut V, f: &mut FieldDecl) {
    v.visit_span(&mut f.span);
    v.visit_ident(&mut f.name);
    walk_type(v, &mut f.ty);
    if let Some(d) = &mut f.default {
        walk_expr(v, d);
    }
}

pub fn walk_effect_ref<V: MutVisitor>(v: &mut V, e: &mut EffectRef) {
    match e {
        EffectRef::Use(s) | EffectRef::Spawn(s) => v.visit_span(s),
        EffectRef::Effect(t) | EffectRef::AnyEffect(t) => walk_type_ref(v, t),
    }
}

pub fn walk_deduction<V: MutVisitor>(v: &mut V, d: &mut Deduction) {
    v.visit_span(&mut d.span);
    match &mut d.target {
        DeductionTarget::Param { name, path } => {
            v.visit_ident(name);
            for p in path {
                v.visit_ident(p);
            }
        }
        DeductionTarget::Result { path } => {
            for p in path {
                v.visit_ident(p);
            }
        }
        DeductionTarget::Opaque => {}
    }
    match &mut d.kind {
        DeductionKind::Exhaustive { quals, reapplied } => {
            for q in quals.iter_mut().chain(reapplied.iter_mut()) {
                walk_type_ref(v, q);
            }
        }
        DeductionKind::Remove(quals) | DeductionKind::Preserve(quals) => {
            for q in quals {
                walk_type_ref(v, q);
            }
        }
        DeductionKind::CanBe { others, .. } => {
            for path in others {
                for id in path {
                    v.visit_ident(id);
                }
            }
        }
        DeductionKind::With { others } => {
            for id in others {
                v.visit_ident(id);
            }
        }
        DeductionKind::Proj(ids) => {
            for id in ids {
                v.visit_ident(id);
            }
        }
        DeductionKind::KeepAll | DeductionKind::Moved | DeductionKind::Deferred => {}
    }
}

pub fn walk_type<V: MutVisitor>(v: &mut V, t: &mut Type) {
    v.visit_type(t);
    match t {
        Type::Named { qualifiers, base } => {
            for q in qualifiers {
                walk_type_ref(v, q);
            }
            walk_type_ref(v, base);
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } => {
            v.visit_span(span);
            for q in qualifiers {
                walk_type_ref(v, q);
            }
            walk_type(v, base);
        }
        Type::Union { arms, span } => {
            v.visit_span(span);
            for a in arms {
                walk_type(v, a);
            }
        }
        Type::Tuple { elems, span } => {
            v.visit_span(span);
            for e in elems {
                walk_type(v, e);
            }
        }
        Type::Array { elem, span } => {
            v.visit_span(span);
            walk_type(v, elem);
        }
        Type::Nullable { inner, span } => {
            v.visit_span(span);
            walk_type(v, inner);
        }
        Type::Fn {
            params,
            param_names,
            effects,
            deductions,
            ret,
            span,
        } => {
            v.visit_span(span);
            for p in params {
                walk_type(v, p);
            }
            for n in param_names.iter_mut().flatten() {
                v.visit_ident(n);
            }
            if let Some(effects) = effects {
                for e in effects {
                    walk_effect_ref(v, e);
                }
            }
            if let Some(ds) = deductions {
                for d in ds {
                    walk_deduction(v, d);
                }
            }
            walk_type(v, ret);
        }
    }
}

pub fn walk_type_ref<V: MutVisitor>(v: &mut V, t: &mut TypeRef) {
    v.visit_type_ref(t);
    v.visit_span(&mut t.span);
    v.visit_ident(&mut t.name);
    for a in t.args.iter_mut().chain(t.value_args.iter_mut()) {
        walk_type(v, a);
    }
    for f in &mut t.from {
        v.visit_ident(f);
    }
    if let Some(at) = &mut t.at {
        v.visit_ident(at);
    }
    if let Some(al) = &mut t.alias {
        v.visit_ident(al);
    }
}

pub fn walk_pattern<V: MutVisitor>(v: &mut V, p: &mut Pattern) {
    match p {
        Pattern::Ident(id) => v.visit_ident(id),
        Pattern::Tuple { elems, span } => {
            v.visit_span(span);
            for e in elems {
                walk_pattern(v, e);
            }
        }
        Pattern::Struct { fields, span } => {
            v.visit_span(span);
            for f in fields {
                v.visit_span(&mut f.span);
                v.visit_ident(&mut f.field);
                v.visit_ident(&mut f.binding);
            }
        }
    }
}

pub fn walk_block<V: MutVisitor>(v: &mut V, b: &mut Block) {
    v.visit_block(b);
    v.visit_span(&mut b.span);
    for s in &mut b.stmts {
        walk_stmt(v, s);
    }
}

pub fn walk_stmt<V: MutVisitor>(v: &mut V, s: &mut Stmt) {
    v.visit_stmt(s);
    match s {
        Stmt::Let {
            pattern,
            ty,
            value,
            span,
        } => {
            v.visit_span(span);
            walk_pattern(v, pattern);
            if let Some(t) = ty {
                walk_type(v, t);
            }
            walk_expr(v, value);
        }
        Stmt::Assign {
            target,
            value,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, target);
            walk_expr(v, value);
        }
        Stmt::Use {
            handler,
            with_items,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, handler);
            for w in with_items {
                walk_expr(v, w);
            }
        }
        Stmt::Rename(r) => {
            v.visit_span(&mut r.span);
            v.visit_ident(&mut r.name);
            v.visit_ident(&mut r.target);
            for g in &mut r.generics {
                v.visit_ident(g);
            }
            for p in &mut r.params {
                v.visit_span(&mut p.span);
                v.visit_ident(&mut p.name);
                walk_type(v, &mut p.ty);
            }
        }
        Stmt::Expr(e) => walk_expr(v, e),
        Stmt::Comp(c) => walk_comp_stmt(v, c),
    }
}

pub fn walk_comp_ty<V: MutVisitor>(v: &mut V, t: &mut CompTy) {
    v.visit_span(&mut t.span);
    v.visit_ident(&mut t.root);
}

pub fn walk_comp_cond<V: MutVisitor>(v: &mut V, c: &mut CompCond) {
    match c {
        CompCond::Kind { ty, span, .. } => {
            v.visit_span(span);
            walk_comp_ty(v, ty);
        }
        CompCond::Is { ty, target, span } => {
            v.visit_span(span);
            walk_comp_ty(v, ty);
            walk_type(v, target);
        }
        CompCond::Canbe { ty, qual, span } => {
            v.visit_span(span);
            walk_comp_ty(v, ty);
            walk_type_ref(v, qual);
        }
        CompCond::NameEq { binder, span, .. } | CompCond::Flag { binder, span, .. } => {
            v.visit_span(span);
            v.visit_ident(binder);
        }
        CompCond::IndexCmp { a, b, span, .. } => {
            v.visit_span(span);
            v.visit_ident(a);
            v.visit_ident(b);
        }
        CompCond::Not(inner, span) => {
            v.visit_span(span);
            walk_comp_cond(v, inner);
        }
    }
}

pub fn walk_comp_stmt<V: MutVisitor>(v: &mut V, c: &mut CompStmt) {
    match c {
        CompStmt::For {
            binder,
            seq,
            body,
            span,
        } => {
            v.visit_span(span);
            v.visit_ident(binder);
            v.visit_span(&mut seq.span);
            walk_comp_ty(v, &mut seq.ty);
            walk_block(v, body);
        }
        CompStmt::If {
            cond,
            then,
            else_,
            span,
        } => {
            v.visit_span(span);
            walk_comp_cond(v, cond);
            walk_block(v, then);
            if let Some(b) = else_ {
                walk_block(v, b);
            }
        }
        CompStmt::WhenKind {
            ty,
            arms,
            else_,
            span,
        } => {
            v.visit_span(span);
            walk_comp_ty(v, ty);
            for arm in arms {
                v.visit_span(&mut arm.span);
                walk_block(v, &mut arm.body);
            }
            if let Some(b) = else_ {
                walk_block(v, b);
            }
        }
        CompStmt::WhenArms {
            value,
            binder,
            body,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, value);
            v.visit_ident(binder);
            walk_block(v, body);
        }
        CompStmt::Refuse { message, span } => {
            v.visit_span(span);
            walk_expr(v, message);
        }
    }
}

pub fn walk_struct_lit_field<V: MutVisitor>(v: &mut V, f: &mut StructLitField) {
    v.visit_struct_lit_field(f);
    v.visit_span(&mut f.span);
    match &mut f.kind {
        StructLitFieldKind::Named { name, value } => {
            v.visit_ident(name);
            walk_expr(v, value);
        }
        StructLitFieldKind::Spread(e) => walk_expr(v, e),
        StructLitFieldKind::InlineFor {
            binder,
            seq,
            entries,
        } => {
            v.visit_ident(binder);
            v.visit_span(&mut seq.span);
            walk_comp_ty(v, &mut seq.ty);
            for e in entries {
                walk_struct_lit_field(v, e);
            }
        }
    }
}

pub fn walk_expr<V: MutVisitor>(v: &mut V, e: &mut Expr) {
    v.visit_expr(e);
    match e {
        Expr::Int { span, .. }
        | Expr::Float { span, .. }
        | Expr::Bool { span, .. }
        | Expr::Char { span, .. }
        | Expr::Placeholder { span }
        | Expr::Continue { span } => v.visit_span(span),
        Expr::Str { parts, span } => {
            v.visit_span(span);
            for p in parts {
                if let StrExprPart::Interp(inner) = p {
                    walk_expr(v, inner);
                }
            }
        }
        Expr::Ident(id) => v.visit_ident(id),
        Expr::Field { base, field, span } => {
            v.visit_span(span);
            walk_expr(v, base);
            v.visit_ident(field);
        }
        Expr::TupleIndex { base, span, .. } => {
            v.visit_span(span);
            walk_expr(v, base);
        }
        Expr::Scoped {
            base,
            name,
            module,
            span,
        } => {
            v.visit_span(span);
            if let Some(b) = base {
                walk_expr(v, b);
            }
            v.visit_ident(name);
            for m in module {
                v.visit_ident(m);
            }
        }
        Expr::EffectScoped {
            base,
            name,
            effect,
            span,
        } => {
            v.visit_span(span);
            if let Some(b) = base {
                walk_expr(v, b);
            }
            v.visit_ident(name);
            v.visit_ident(effect);
        }
        Expr::Call {
            callee,
            type_args,
            args,
            named,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, callee);
            for t in type_args {
                walk_type(v, t);
            }
            for a in args {
                walk_expr(v, a);
            }
            for n in named {
                v.visit_span(&mut n.span);
                v.visit_ident(&mut n.name);
                walk_expr(v, &mut n.value);
            }
        }
        Expr::Index { base, index, span } => {
            v.visit_span(span);
            walk_expr(v, base);
            walk_expr(v, index);
        }
        Expr::ArrayLit { elems, span }
        | Expr::SetLit { elems, span }
        | Expr::Tuple { elems, span } => {
            v.visit_span(span);
            for e in elems {
                walk_expr(v, e);
            }
        }
        Expr::MapLit { entries, span } => {
            v.visit_span(span);
            for (k, val) in entries {
                walk_expr(v, k);
                walk_expr(v, val);
            }
        }
        Expr::StructLit { ty, fields, span } => {
            v.visit_span(span);
            if let Some(t) = ty {
                walk_type(v, t);
            }
            for f in fields {
                walk_struct_lit_field(v, f);
            }
        }
        Expr::Unary { operand, span, .. } => {
            v.visit_span(span);
            walk_expr(v, operand);
        }
        Expr::Binary { lhs, rhs, span, .. } => {
            v.visit_span(span);
            walk_expr(v, lhs);
            walk_expr(v, rhs);
        }
        Expr::Is {
            subject,
            check,
            binding,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, subject);
            for c in check {
                walk_type_ref(v, c);
            }
            if let Some(b) = binding {
                v.visit_ident(b);
            }
        }
        Expr::Widen {
            subject,
            quals,
            binding,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, subject);
            for q in quals {
                walk_type_ref(v, q);
            }
            if let Some(b) = binding {
                v.visit_ident(b);
            }
        }
        Expr::NonNull { operand, span } => {
            v.visit_span(span);
            walk_expr(v, operand);
        }
        Expr::Assert {
            cond,
            message,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, cond);
            if let Some(m) = message {
                walk_expr(v, m);
            }
        }
        Expr::Unreachable { message, span } => {
            v.visit_span(span);
            if let Some(m) = message {
                walk_expr(v, m);
            }
        }
        Expr::IncDec { operand, span, .. } => {
            v.visit_span(span);
            walk_expr(v, operand);
        }
        Expr::If {
            branches,
            else_block,
            span,
        } => {
            v.visit_span(span);
            for (c, b) in branches {
                walk_expr(v, c);
                walk_block(v, b);
            }
            if let Some(b) = else_block {
                walk_block(v, b);
            }
        }
        Expr::When {
            subject,
            branches,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, subject);
            for br in branches {
                v.visit_span(&mut br.span);
                for c in &mut br.check {
                    walk_type_ref(v, c);
                }
                if let Some(b) = &mut br.binding {
                    v.visit_ident(b);
                }
                walk_block(v, &mut br.body);
            }
        }
        Expr::WhenCond {
            branches,
            else_block,
            span,
        } => {
            v.visit_span(span);
            for (c, b) in branches {
                walk_expr(v, c);
                walk_block(v, b);
            }
            walk_block(v, else_block);
        }
        Expr::While {
            cond,
            body,
            else_block,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, cond);
            walk_block(v, body);
            if let Some(b) = else_block {
                walk_block(v, b);
            }
        }
        Expr::For {
            pattern,
            iterable,
            body,
            else_block,
            span,
        } => {
            v.visit_span(span);
            walk_pattern(v, pattern);
            walk_expr(v, iterable);
            walk_block(v, body);
            if let Some(b) = else_block {
                walk_block(v, b);
            }
        }
        Expr::Lambda { params, body, span } => {
            v.visit_span(span);
            for p in params {
                v.visit_span(&mut p.span);
                v.visit_ident(&mut p.name);
                if let Some(t) = &mut p.ty {
                    walk_type(v, t);
                }
            }
            match body {
                LambdaBody::Expr(e) => walk_expr(v, e),
                LambdaBody::Block(b) => walk_block(v, b),
            }
        }
        Expr::Try { body, span } => {
            v.visit_span(span);
            walk_block(v, body);
        }
        Expr::Elvis {
            subject,
            pick,
            rhs,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, subject);
            if let Some(p) = pick {
                v.visit_span(&mut p.span);
                for q in &mut p.quals {
                    walk_type_ref(v, q);
                }
            }
            walk_expr(v, rhs);
        }
        Expr::SafeField { base, inner, span } => {
            v.visit_span(span);
            walk_expr(v, base);
            walk_expr(v, inner);
        }
        Expr::Return { value, span } | Expr::Break { value, span } => {
            v.visit_span(span);
            if let Some(val) = value {
                walk_expr(v, val);
            }
        }
        Expr::Spread { operand, span } => {
            v.visit_span(span);
            walk_expr(v, operand);
        }
        Expr::SelfScoped { name, span } => {
            v.visit_span(span);
            v.visit_ident(name);
        }
        Expr::SelfAddr { face, span } => {
            v.visit_span(span);
            walk_type_ref(v, face);
        }
        Expr::Spawn {
            handler,
            with_items,
            pool,
            join,
            span,
        } => {
            v.visit_span(span);
            walk_expr(v, handler);
            for w in with_items {
                walk_expr(v, w);
            }
            if let Some(p) = pool {
                walk_expr(v, p);
            }
            if let Some(j) = join {
                walk_expr(v, j);
            }
        }
        Expr::ReplyTo {
            member,
            captures,
            pool,
            span,
            ..
        } => {
            v.visit_span(span);
            v.visit_ident(member);
            for c in captures {
                walk_expr(v, c);
            }
            if let Some(p) = pool {
                walk_expr(v, p);
            }
        }
        Expr::WaitFor {
            binding,
            ty,
            body,
            span,
        } => {
            v.visit_span(span);
            v.visit_ident(binding);
            if let Some(t) = ty {
                walk_type(v, t);
            }
            walk_block(v, body);
        }
        Expr::Error { span } => v.visit_span(span),
    }
}
