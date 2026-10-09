//! The text form of the IR (IR record §8): for inspection and golden tests, not
//! for parsing back.

use std::fmt::Write;

use crate::ir::*;

pub struct Dumper<'a> {
    program: &'a Program,
    out: String,
}

impl<'a> Dumper<'a> {
    pub fn module(program: &'a Program, module: &Module) -> String {
        let mut d = Dumper { program, out: String::new() };
        let _ = writeln!(d.out, "module {}", module.path.0.join("."));
        for decl in &module.decls {
            d.out.push('\n');
            d.decl(decl);
        }
        d.out
    }

    fn name_of(&self, id: &DeclId) -> String {
        self.program.ref_name(id)
    }

    fn type_params(&mut self, tps: &[TypeParam]) {
        if tps.is_empty() {
            return;
        }
        let parts: Vec<String> = tps
            .iter()
            .map(|t| if t.canbe_linear { format!("{} canbe linear", t.name) } else { t.name.clone() })
            .collect();
        let _ = write!(self.out, "<{}>", parts.join(", "));
    }

    fn params(&mut self, params: &[Param]) {
        let mut first = true;
        for p in params {
            if !first {
                self.out.push_str(", ");
            }
            first = false;
            let mode = match p.mode {
                PassMode::Moved => "moved ",
                PassMode::Lent => "lent ",
                PassMode::LentMut => "lent_mut ",
            };
            let _ = write!(self.out, "{mode}{}{}: {}", if p.variadic { "..." } else { "" }, p.local.0, p.ty);
        }
    }

    fn decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Struct(s) => {
                let _ = write!(
                    self.out,
                    "{}{}{}struct {}",
                    if s.exported { "export " } else { "" },
                    if s.opaque { "opaque " } else { "" },
                    if s.linear { "linear " } else { "" },
                    s.name
                );
                self.type_params(&s.type_params);
                if s.canbe_mut {
                    self.out.push_str(" canbe Mut");
                }
                if s.has_wire_form {
                    self.out.push_str(" wire");
                }
                self.out.push_str(" {\n");
                for f in &s.fields {
                    let _ = write!(self.out, "  {}{}: {}", if f.canbe_mut { "canbe_mut " } else { "" }, f.name, f.ty);
                    if let Some(d) = &f.default {
                        self.out.push_str(" = ");
                        self.expr(d, 1);
                    }
                    self.out.push('\n');
                }
                self.out.push_str("}\n");
            }
            Decl::Enum(e) => {
                let _ = write!(self.out, "enum {}", e.name);
                if let Some(h) = &e.protocol_hash {
                    let _ = write!(self.out, " [proto {h}]");
                }
                self.out.push_str(" {\n");
                for v in &e.variants {
                    let fs: Vec<String> = v.fields.iter().map(|(n, t)| format!("{n}: {t}")).collect();
                    let _ = writeln!(self.out, "  {}({})", v.name, fs.join(", "));
                }
                self.out.push_str("}\n");
            }
            Decl::Union(u) => {
                let _ = write!(self.out, "{}union {}", if u.exported { "export " } else { "" }, u.name);
                self.type_params(&u.type_params);
                let _ = writeln!(self.out, " = {}", u.ty);
            }
            Decl::Interface(i) => {
                let _ = write!(
                    self.out,
                    "{}{}interface {}",
                    if i.exported { "export " } else { "" },
                    if i.actor { "actor " } else { "" },
                    i.name
                );
                self.type_params(&i.type_params);
                if !i.prereqs.is_empty() {
                    let ps: Vec<String> = i.prereqs.iter().map(|t| t.to_string()).collect();
                    let _ = write!(self.out, " [{}]", ps.join(", "));
                }
                self.out.push_str(" {\n");
                for m in &i.members {
                    let _ = write!(self.out, "  {}fn {}", if m.send { "send " } else { "" }, m.name);
                    if m.emitted_name != m.name {
                        let _ = write!(self.out, " as {}", m.emitted_name);
                    }
                    self.type_params(&m.type_params);
                    self.out.push('(');
                    self.params(&m.params);
                    let _ = writeln!(self.out, ") -> {}", m.ret);
                }
                self.out.push_str("}\n");
            }
            Decl::Impl(h) => {
                let _ = write!(
                    self.out,
                    "{}{}{}{}impl {}",
                    if h.exported { "export " } else { "" },
                    if h.platform { "platform " } else { "" },
                    if h.threadsafe { "threadsafe " } else { "" },
                    if h.intrinsic { "intrinsic " } else { "" },
                    h.name
                );
                self.type_params(&h.type_params);
                self.out.push('(');
                self.params(&h.ctor_params);
                self.out.push(')');
                let faces: Vec<String> = h
                    .faces
                    .iter()
                    .zip(&h.face_any)
                    .map(|(f, any)| if *any { format!("any {f}") } else { f.to_string() })
                    .collect();
                let _ = write!(self.out, " of {}", faces.join(", "));
                if !h.deps.is_empty() {
                    let ds: Vec<String> = h.deps.iter().map(|t| t.to_string()).collect();
                    let _ = write!(self.out, " deps [{}]", ds.join(", "));
                }
                let _ = writeln!(self.out, "{} {{", if h.stateful { " stateful" } else { "" });
                if let Some(m) = &h.mailbox {
                    self.out.push_str("  mailbox ");
                    self.expr(m, 1);
                    self.out.push('\n');
                }
                for f in &h.state {
                    let _ = write!(self.out, "  state {}: {}", f.name, f.ty);
                    if let Some(d) = &f.default {
                        self.out.push_str(" = ");
                        self.expr(d, 1);
                    }
                    self.out.push('\n');
                }
                if let Some(init) = &h.init {
                    self.fn_decl(init, 1);
                }
                for m in &h.members {
                    self.fn_decl(m, 1);
                }
                self.out.push_str("}\n");
            }
            Decl::Fn(f) => self.fn_decl(f, 0),
            Decl::Static(st) => {
                let _ = write!(self.out, "static {}: {} ", st.local.0, st.ty);
                self.block(&Block { stmts: st.stmts.clone(), value: None }, 0);
                self.out.push('\n');
            }
            Decl::PlatformType(t) => {
                let _ = write!(
                    self.out,
                    "{}{}platform type {}",
                    if t.exported { "export " } else { "" },
                    if t.linear { "linear " } else { "" },
                    t.name
                );
                self.type_params(&t.type_params);
                if t.canbe_mut {
                    self.out.push_str(" canbe Mut");
                }
                self.out.push('\n');
            }
        }
    }

    fn fn_decl(&mut self, f: &FnDecl, indent: usize) {
        let pad = "  ".repeat(indent);
        let kind = match f.kind {
            FnKind::Plain | FnKind::Member { .. } => "",
            FnKind::Intrinsic => "intrinsic ",
            FnKind::Platform => "platform ",
            FnKind::Qualifies => "qualifies ",
        };
        let shown = if matches!(f.kind, FnKind::Member { .. }) { f.name.clone() } else { self.name_of(&f.id) };
        let _ = write!(self.out, "{pad}{}{kind}fn {}", if f.exported { "export " } else { "" }, shown);
        self.type_params(&f.type_params);
        self.out.push('(');
        // Effects, declared, implicits: separated by `;` in the dump.
        let n = f.params.len();
        let declared_end = n - f.implicit_params;
        let mut first = true;
        for (i, p) in f.params.iter().enumerate() {
            if i == f.effect_params && f.effect_params > 0 {
                self.out.push_str("; ");
                first = true;
            }
            if i == declared_end && f.implicit_params > 0 {
                self.out.push_str("; ?");
                first = true;
            }
            if !first {
                self.out.push_str(", ");
            }
            first = false;
            self.params(std::slice::from_ref(p));
        }
        let _ = write!(self.out, ") -> {}", f.ret);
        if !f.borrows.is_empty() {
            let bs: Vec<String> = f.borrows.iter().map(|i| f.params[*i].local.0.clone()).collect();
            let _ = write!(self.out, " borrows [{}]", bs.join(", "));
        }
        if !f.holds.is_empty() {
            let hs: Vec<String> = f.holds.iter().map(|(a, b)| format!("{} <- {}", f.params[*a].local.0, f.params[*b].local.0)).collect();
            let _ = write!(self.out, " holds [{}]", hs.join(", "));
        }
        if let Some(t) = &f.throws {
            let _ = write!(self.out, " throws {t}");
        }
        // [canbe-entry] the alias relations, as the source spells them.
        if !f.may_alias.is_empty() {
            let name = |i: &usize| f.params[*i].local.0.clone();
            let entries: Vec<String> = f
                .may_alias
                .iter()
                .map(|m| match m {
                    MayAlias::Params(a, b) => format!("{} canbe {}", name(a), name(b)),
                    MayAlias::In { param, root, path } => {
                        let mut p = name(root);
                        for st in path {
                            match st {
                                AnchorStep::Field(f) => p.push_str(&format!(".{f}")),
                                AnchorStep::Tuple(n) => p.push_str(&format!(".{n}")),
                            }
                        }
                        format!("{} canbe in {p}", name(param))
                    }
                })
                .collect();
            let _ = write!(self.out, " => {}", entries.join(", "));
        }
        match &f.body {
            None => self.out.push('\n'),
            Some(b) => {
                self.out.push(' ');
                self.block(b, indent);
                self.out.push('\n');
            }
        }
    }

    fn block(&mut self, b: &Block, indent: usize) {
        let pad = "  ".repeat(indent + 1);
        self.out.push_str("{\n");
        for s in &b.stmts {
            self.out.push_str(&pad);
            self.stmt(s, indent + 1);
            self.out.push('\n');
        }
        if let Some(v) = &b.value {
            self.out.push_str(&pad);
            self.out.push_str("value ");
            self.expr(v, indent + 1);
            self.out.push('\n');
        }
        let _ = write!(self.out, "{}}}", "  ".repeat(indent));
    }

    fn place(&mut self, p: &Place, indent: usize) {
        self.out.push_str(&p.root.0);
        for s in &p.steps {
            match s {
                Step::Field(f) => {
                    let _ = write!(self.out, ".{f}");
                }
                Step::Tuple(i) => {
                    let _ = write!(self.out, ".{i}");
                }
                Step::Index(e) => {
                    self.out.push('[');
                    self.expr(e, indent);
                    self.out.push(']');
                }
            }
        }
    }

    fn stmt(&mut self, s: &Stmt, indent: usize) {
        match s {
            Stmt::Loop { id, body } => {
                let _ = write!(self.out, "loop#{} ", id.0);
                self.block(body, indent);
            }
            Stmt::ForEach { id, local, ty, iterable, body } => {
                let _ = write!(self.out, "foreach#{} {}: {ty} in ", id.0, local.0);
                self.expr(iterable, indent);
                self.out.push(' ');
                self.block(body, indent);
            }
            Stmt::Let { id, local, ty, value } => {
                let _ = write!(self.out, "bind#{} let {}: {ty} = ", id.0, local.0);
                self.expr(value, indent);
            }
            Stmt::Unpack { id, locals, from, from_ty: _, variant } => {
                let ls: Vec<String> = locals.iter().map(|(l, t)| format!("{}: {t}", l.0)).collect();
                let _ = write!(self.out, "unpack#{} ({}) = ", id.0, ls.join(", "));
                self.place(from, indent);
                let _ = write!(self.out, " as variant {variant}");
            }
            Stmt::Alias { id, local, ty, place } => {
                let _ = write!(self.out, "alias#{} {}: {ty} = ", id.0, local.0);
                self.place(place, indent);
            }
            Stmt::Narrow { id, local, ty, from, from_ty: _, because } => {
                let why = match because {
                    Justification::Test { test } => format!("check#{}", test.0),
                    Justification::Arm { switch, arm } => format!("switch#{}.arm#{arm}", switch.0),
                    Justification::Cond { branch, arm } => format!("branch#{}.cond#{arm}", branch.0),
                    Justification::After { branch, arm } => format!("after branch#{}.cond#{arm}", branch.0),
                    Justification::LoopValue { loop_ } => format!("loop#{} value", loop_.0),
                    Justification::Claim => "claim".to_string(),
                };
                let _ = write!(self.out, "narrow#{}[{why}] {}: {ty} = ", id.0, local.0);
                self.place(from, indent);
            }
            Stmt::Assign { place, value } => {
                self.out.push_str("assign ");
                self.place(place, indent);
                self.out.push_str(" = ");
                self.expr(value, indent);
            }
            Stmt::Expr(e) => self.expr(e, indent),
            Stmt::Return(None) => self.out.push_str("return"),
            Stmt::Return(Some(e)) => {
                self.out.push_str("return ");
                self.expr(e, indent);
            }
            Stmt::Break => self.out.push_str("break"),
            Stmt::Continue => self.out.push_str("continue"),
        }
    }

    fn args(&mut self, args: &[Expr], indent: usize) {
        self.out.push('(');
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.expr(a, indent);
        }
        self.out.push(')');
    }

    fn type_args(&mut self, tys: &[Ty]) {
        if tys.is_empty() {
            return;
        }
        let parts: Vec<String> = tys.iter().map(|t| t.to_string()).collect();
        let _ = write!(self.out, "<{}>", parts.join(", "));
    }

    fn fn_ref(&mut self, r: &FnRef) {
        match r {
            FnRef::Decl(id) => {
                let n = self.name_of(id);
                self.out.push_str(&n);
            }
            FnRef::Local(l) => {
                let _ = write!(self.out, "local {}", l.0);
            }
        }
    }

    pub fn expr(&mut self, e: &Expr, indent: usize) {
        match &e.kind {
            ExprKind::Int(v) => {
                let _ = write!(self.out, "{v}");
            }
            ExprKind::Long(v) => {
                let _ = write!(self.out, "{v}L");
            }
            ExprKind::Float(v) => {
                let _ = write!(self.out, "{v:?}f");
            }
            ExprKind::Double(v) => {
                let _ = write!(self.out, "{v:?}");
            }
            ExprKind::Bool(v) => {
                let _ = write!(self.out, "{v}");
            }
            ExprKind::Char(c) => {
                let _ = write!(self.out, "{c:?}");
            }
            ExprKind::Str(s) => {
                let _ = write!(self.out, "{s:?}");
            }
            ExprKind::Unit => self.out.push_str("unit"),
            ExprKind::MakeNone => {
                let _ = write!(self.out, "none[{}]", e.ty);
            }
            ExprKind::Read { place, consume } => {
                self.out.push_str(if *consume { "!read " } else { "read " });
                self.place(place, indent);
            }
            ExprKind::Call { target, type_args, args } => {
                self.out.push_str("call ");
                self.fn_ref(target);
                self.type_args(type_args);
                self.args(args, indent);
            }
            ExprKind::HandlerCall { instance, member, args } => {
                self.out.push_str("handler_call ");
                match member {
                    HandlerMember::Face(m) => {
                        let n = self.name_of(&m.interface);
                        let _ = write!(self.out, "{n}#{}", m.index);
                    }
                    HandlerMember::Own(n) => self.out.push_str(n),
                }
                self.out.push('(');
                self.expr(instance, indent);
                for a in args {
                    self.out.push_str(", ");
                    self.expr(a, indent);
                }
                self.out.push(')');
            }
            ExprKind::MemberCall { instance, member, type_args, args } => {
                self.out.push_str("member ");
                let n = self.name_of(&member.interface);
                let _ = write!(self.out, "{n}#{}", member.index);
                self.type_args(type_args);
                self.out.push('(');
                self.expr(instance, indent);
                for a in args {
                    self.out.push_str(", ");
                    self.expr(a, indent);
                }
                self.out.push(')');
            }
            // [ir-dump] Inline, as the source spells it. Every binary operation
            // is parenthesized, and a unary operand always is, so `!(read x)`
            // never reads as the consuming `!read x`.
            ExprKind::Op { op, args } => {
                let sym = match op {
                    Op::Add => "+",
                    Op::Sub => "-",
                    Op::Mul => "*",
                    Op::Div => "/",
                    Op::Rem => "%",
                    Op::And => "&&",
                    Op::Or => "||",
                    Op::Not => "!",
                    Op::Neg => "-",
                    Op::Lt => "<",
                    Op::Gt => ">",
                    Op::LtEq => "<=",
                    Op::GtEq => ">=",
                    Op::Eq => "==",
                    Op::NotEq => "!=",
                };
                match args.as_slice() {
                    [x] => {
                        // A binary operand brings its own parentheses.
                        let bracketed = matches!(&x.kind, ExprKind::Op { args, .. } if args.len() == 2);
                        self.out.push_str(sym);
                        if !bracketed {
                            self.out.push('(');
                        }
                        self.expr(x, indent);
                        if !bracketed {
                            self.out.push(')');
                        }
                    }
                    [l, r] => {
                        self.out.push('(');
                        self.expr(l, indent);
                        let _ = write!(self.out, " {sym} ");
                        self.expr(r, indent);
                        self.out.push(')');
                    }
                    _ => {
                        let _ = write!(self.out, "op {op:?}");
                        self.args(args, indent);
                    }
                }
            }
            ExprKind::Construct { fields } => {
                let _ = write!(self.out, "construct {} {{", e.ty);
                for (i, (n, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.out.push(',');
                    }
                    let _ = write!(self.out, " {n}: ");
                    self.expr(v, indent);
                }
                self.out.push_str(" }");
            }
            ExprKind::MakeUnion { arm, value } => {
                let _ = write!(self.out, "make_union[{}]#{arm}(", e.ty);
                self.expr(value, indent);
                self.out.push(')');
            }
            ExprKind::Rewrap { from, value } => {
                let _ = write!(self.out, "rewrap[{from} -> {}](", e.ty);
                self.expr(value, indent);
                self.out.push(')');
            }
            ExprKind::DropMut { value } => {
                self.out.push_str("drop_mut(");
                self.expr(value, indent);
                self.out.push(')');
            }
            ExprKind::Tuple(es) => {
                self.out.push_str("tuple");
                self.args(es, indent);
            }
            ExprKind::List(es) => {
                let _ = write!(self.out, "list[{}]", e.ty);
                self.args(es, indent);
            }
            ExprKind::Array(es) => {
                let _ = write!(self.out, "array[{}]", e.ty);
                self.args(es, indent);
            }
            ExprKind::Concat(es) => {
                self.out.push_str("concat");
                self.args(es, indent);
            }
            ExprKind::Branch { id, arms, otherwise } => {
                let _ = write!(self.out, "branch#{}", id.0);
                for (i, (c, b)) in arms.iter().enumerate() {
                    let _ = write!(self.out, " cond#{i} ");
                    self.expr(c, indent);
                    self.out.push(' ');
                    self.block(b, indent);
                }
                if let Some(o) = otherwise {
                    self.out.push_str(" else ");
                    self.block(o, indent);
                }
            }
            ExprKind::Switch { id, subject, arms } => {
                let _ = write!(self.out, "switch#{} ", id.0);
                self.expr(subject, indent);
                self.out.push_str(" {\n");
                let pad = "  ".repeat(indent + 1);
                for (i, arm) in arms.iter().enumerate() {
                    let test = match &arm.test {
                        ArmTest::Arm(a) => format!("arm {a}"),
                        ArmTest::Arms(a) => format!("arms {a:?}"),
                        ArmTest::None => "none".to_string(),
                        ArmTest::Lit(ls) => format!("lit {}", ls.iter().map(|a| format!("{}{}{:?}", a.arm, if a.negate { " not " } else { " " }, a.lits)).collect::<Vec<_>>().join(" | ")),
                        ArmTest::Else => "else".to_string(),
                    };
                    let _ = write!(self.out, "{pad}arm#{i} ({test}) ");
                    self.block(&arm.body, indent + 1);
                    self.out.push('\n');
                }
                let _ = write!(self.out, "{}}}", "  ".repeat(indent));
            }
            ExprKind::Test { id, subject, test } => {
                let t = match test {
                    ArmTest::Arm(a) => format!("arm {a}"),
                    ArmTest::Arms(a) => format!("arms {a:?}"),
                    ArmTest::None => "none".to_string(),
                    ArmTest::Lit(ls) => format!("lit {}", ls.iter().map(|a| format!("{}{}{:?}", a.arm, if a.negate { " not " } else { " " }, a.lits)).collect::<Vec<_>>().join(" | ")),
                    ArmTest::Else => "else".to_string(),
                };
                let _ = write!(self.out, "check#{} (", id.0);
                self.expr(subject, indent);
                let _ = write!(self.out, " is {t})");
            }
            ExprKind::Lambda { params, ret, body, captures } => {
                self.out.push_str("lambda(");
                self.params(params);
                let _ = write!(self.out, ") -> {ret}");
                if !captures.is_empty() {
                    let cs: Vec<String> = captures
                        .iter()
                        .map(|c| {
                            format!(
                                "{}{}{}",
                                if c.consumed { "!" } else { "" },
                                if c.mutable { "mut " } else { "" },
                                c.local.0
                            )
                        })
                        .collect();
                    let _ = write!(self.out, " captures [{}]", cs.join(", "));
                }
                self.out.push(' ');
                self.block(body, indent);
            }
            ExprKind::FnValue(r) => {
                self.out.push_str("fn ");
                self.fn_ref(r);
            }
            ExprKind::Try { body } => {
                let _ = write!(self.out, "try[{}] ", e.ty);
                self.block(body, indent);
            }
            ExprKind::Throw { message } => {
                self.out.push_str("throw(");
                self.expr(message, indent);
                self.out.push(')');
            }
            ExprKind::Assert { cond, message, at } => {
                self.out.push_str("assert(");
                self.expr(cond, indent);
                if let Some(m) = message {
                    self.out.push_str(", ");
                    self.expr(m, indent);
                }
                let _ = write!(self.out, ") at {at}");
            }
            ExprKind::Unreachable { message, at } => {
                self.out.push_str("unreachable(");
                if let Some(m) = message {
                    self.expr(m, indent);
                }
                let _ = write!(self.out, ") at {at}");
            }
            ExprKind::Spawn { handler, deps, pool, join, effects } => {
                let _ = write!(self.out, "spawn[{}](", e.ty);
                self.expr(handler, indent);
                if !deps.is_empty() {
                    self.out.push_str(", deps: ");
                    self.args(deps, indent);
                }
                if let Some(p) = pool {
                    self.out.push_str(", on: ");
                    self.expr(p, indent);
                }
                if let Some(j) = join {
                    self.out.push_str(", in: ");
                    self.expr(j, indent);
                }
                self.out.push(')');
                let es: Vec<String> = effects.iter().map(|t| t.to_string()).collect();
                let _ = write!(self.out, " serves [{}]", es.join(", "));
            }
            ExprKind::Send { addr, member, args } => {
                self.out.push_str("send ");
                let n = self.name_of(&member.interface);
                let _ = write!(self.out, "{n}#{}(", member.index);
                self.expr(addr, indent);
                for a in args {
                    self.out.push_str(", ");
                    self.expr(a, indent);
                }
                self.out.push(')');
            }
            ExprKind::ReplyTo { target, captures, gated, pool } => {
                let _ = write!(self.out, "replyto{} ", if *gated { "!" } else { "" });
                match target {
                    ReplyTarget::Member(m) => self.out.push_str(m),
                    ReplyTarget::Task { target, effects } => {
                        self.out.push_str("task ");
                        let n = self.name_of(target);
                        self.out.push_str(&n);
                        if !effects.is_empty() {
                            self.out.push_str(" effects");
                            self.args(effects, indent);
                        }
                    }
                }
                self.args(captures, indent);
                if let Some(p) = pool {
                    self.out.push_str(" on ");
                    self.expr(p, indent);
                }
            }
            ExprKind::WaitFor { local, token_ty, body } => {
                let _ = write!(self.out, "waitfor {}: {token_ty} ", local.0);
                self.block(body, indent);
            }
            ExprKind::Spread { value } => {
                self.out.push_str("...");
                self.expr(value, indent);
            }
            ExprKind::Present { value } => {
                self.out.push_str("present(");
                self.expr(value, indent);
                self.out.push(')');
            }
            ExprKind::UnionToStr { value, arms } => {
                self.out.push_str("union_to_str(");
                self.expr(value, indent);
                for a in arms {
                    self.out.push_str(", ");
                    self.expr(a, indent);
                }
                self.out.push(')');
            }
            ExprKind::Widen { value } => {
                let _ = write!(self.out, "widen[{}](", e.ty);
                self.expr(value, indent);
                self.out.push(')');
            }
            ExprKind::Handle { instance } => {
                let _ = write!(self.out, "handle[{}](", e.ty);
                self.expr(instance, indent);
                self.out.push(')');
            }
            ExprKind::AddrInstance { addr } => {
                self.out.push_str("instance_at(");
                self.expr(addr, indent);
                self.out.push(')');
            }
            ExprKind::SelfAddr => self.out.push_str("self_addr"),
            ExprKind::SelfSend { member, args } => {
                let _ = write!(self.out, "self_send {member}");
                self.args(args, indent);
            }
            ExprKind::Unsupported(what) => {
                let _ = write!(self.out, "UNSUPPORTED({what})");
            }
        }
    }
}
