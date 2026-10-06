//! [comptime-instantiate] The comptime expansion: every `by` site stamps a
//! `compfn` at a concrete type, and every comptime construct in the copy is
//! unrolled, selected or refused, leaving an ordinary fn with an ordinary body
//! (user decisions 2026-09-28, the comptime rounds; COMPLETED.md's log).
//!
//! Runs before resolution, over the whole source set, because a `by auto` in
//! any file reads `core.auto`'s compfns and the target type's declaration,
//! neither of which resolution has looked at yet. What it leaves behind:
//!
//! - one stamped fn per (`by` site, group member), declared on the target type
//!   [fn-attached] and pushed into the target's module — the checker's
//!   [group-obligation] check finds it like a hand-written fulfilment;
//! - the body of every `fn … by X` declaration, in place;
//! - the body of every **concrete** compfn (no bound), expanded in place;
//! - no generic compfn at all: they are removed, since nothing downstream
//!   could type `T.fields`.
//!
//! Spans: every node of a stamped body is given a **synthetic** span past the
//! end of the target file, one fresh region per unrolled copy, so the checker's
//! span-keyed side tables never see two copies as one node. The `Stamp` on the
//! fn records the regions, and the checker redirects a diagnostic inside one to
//! the field or arm the copy was for, prefixed with what was being stamped.

use std::collections::HashMap;

use salvo_syntax::ast::*;
use salvo_syntax::visit_mut::{self, MutVisitor};
use salvo_syntax::{Diagnostic, Span};

use crate::source::{ModulePath, SourceFile};

/// [comptime-fields] A hover the expansion recorded inside a `comptime fn`
/// body: the compile-time type of a comptime name at its written span. The
/// body itself is never checked (it has no meaning until stamped), so this is
/// the one source of hover text for `T`, a binder, or a projection.
#[derive(Clone, Debug)]
pub struct CompHover {
    pub file: usize,
    pub span: Span,
    /// Markdown: a code line with the name and its `core.comptime` type, then
    /// a sentence.
    pub text: String,
    /// [doc-module] The module a `by` site took its comptime fn from, when
    /// the hover is a `by` site: the language server appends that module's
    /// own documentation, since `by @auto` names the module as much as
    /// `size@list` does.
    pub module: Option<ModulePath>,
}

pub struct ComptimeOutput {
    pub diagnostics: Vec<(usize, Diagnostic)>,
    pub hovers: Vec<CompHover>,
}

/// Runs the expansion. `modules` is aligned with `files`.
pub fn expand_comptime(files: &[SourceFile], modules: &mut [Module]) -> ComptimeOutput {
    let mut out = Vec::new();
    let mut hovers: Vec<CompHover> = Vec::new();

    // Program-wide tables. Names are flat here — the resolver's visibility
    // rules run later on what this produces — and a collision between two
    // modules' same-named types is the resolver's error, not a reason to
    // stamp nothing.
    let mut compfns: Vec<CompFnDecl> = Vec::new();
    let mut structs: HashMap<String, Vec<(ModulePath, StructDecl)>> = HashMap::new();
    let mut types: HashMap<String, Vec<(ModulePath, TypeDecl)>> = HashMap::new();
    let mut groups: HashMap<String, ParamsDecl> = HashMap::new();
    // [std-shadow] The repository root opened as a workspace loads `std/` a
    // second time as user files (`std.core.auto` beside the embedded
    // `core.auto`, ROADMAP §4). A copy — same content, module ending in the
    // std module's path — is not a second candidate for `by auto`.
    let shadow_copy = |idx: usize| -> bool {
        let f = &files[idx];
        !f.is_std
            && files.iter().any(|g| {
                g.is_std
                    && g.content == f.content
                    && f.module.matches_suffix(&g.module.0.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                    && f.module.0.len() > g.module.0.len()
            })
    };
    for (idx, (file, module)) in files.iter().zip(modules.iter()).enumerate() {
        if shadow_copy(idx) {
            continue;
        }
        for item in &module.items {
            match item {
                Item::Fn(f) if f.compfn.as_ref().is_some_and(|c| c.bound.is_some()) => {
                    compfns.push(CompFnDecl {
                        module: file.module.clone(),
                        file: idx,
                        decl: f.clone(),
                    });
                }
                // [type-identity] Every declaration of a name, by module: one
                // stamped in a module reads that module's own first.
                Item::Struct(s) => {
                    structs.entry(s.name.name.clone()).or_default().push((file.module.clone(), s.clone()));
                }
                Item::Type(t) => {
                    types.entry(t.name.name.clone()).or_default().push((file.module.clone(), t.clone()));
                }
                Item::Params(g) => {
                    groups.entry(g.name.name.clone()).or_insert_with(|| g.clone());
                }
                _ => {}
            }
        }
    }
    let world = World {
        compfns,
        structs,
        types,
        own: std::cell::RefCell::new(ModulePath(Vec::new())),
        groups,
        module_paths: files
            .iter()
            .enumerate()
            .filter(|(i, _)| !shadow_copy(*i))
            .map(|(_, f)| f.module.clone())
            .collect(),
    };

    for (file_idx, (file, module)) in files.iter().zip(modules.iter_mut()).enumerate() {
        *world.own.borrow_mut() = file.module.clone();
        let mut diags: Vec<Diagnostic> = Vec::new();
        let mut virtual_next = file.content.len() as u32 + 1;
        let mut generated: Vec<Item> = Vec::new();

        // [comptime-fields] Hovers over every comptime fn body, at the written
        // spans, before the bodies are consumed by stamping.
        for item in &module.items {
            let Item::Fn(f) = item else { continue };
            let Some(cf) = &f.compfn else { continue };
            // The declaration's own name: the checker never sees a generic
            // comptime fn, so this is its one hover.
            hovers.push(CompHover {
                file: file_idx,
                span: f.name.span,
                text: compfn_hover_text(f, &file.module, None),
                module: None,
            });
            let mut rec = HoverRecorder {
                file: file_idx,
                bound: cf.bound.as_ref().map(|(k, id)| (id.name.clone(), *k)),
                binders: Vec::new(),
                out: &mut hovers,
            };
            rec.fn_decl(f);
        }

        // [obligation-by] Clause sites: structs and type declarations.
        for item in &module.items {
            let (name, exported, obligations) = match item {
                Item::Struct(s) => (&s.name, s.exported, &s.obligations),
                Item::Type(t) => (&t.name, t.exported, &t.obligations),
                _ => continue,
            };
            for ob in obligations {
                let Some(by) = &ob.by else { continue };
                let Some(group) = world.groups.get(ob.group.name.name.as_str()) else {
                    // The checker reports the unknown group at the clause.
                    continue;
                };
                let Some(target) = world.target(&name.name) else { continue };
                if !ob
                    .group
                    .args
                    .iter()
                    .all(|a| matches!(a, Type::Named { base, .. } if base.name.name == "self"))
                {
                    diags.push(Diagnostic::error(
                        format!(
                            "`by` stamps the members for the type it is written on, so the \
                             group's argument is `self`: write `: {}<self> by {}` \
                             [obligation-by]",
                            ob.group.name.name,
                            by.text()
                        ),
                        ob.group.span,
                    ));
                    continue;
                }
                let mut members: Vec<&FnDecl> = group.fns.iter().collect();
                // Two groups asking for the same member ask for one
                // declaration: `Eq<self> by auto` beside `Hashed<self> by
                // auto` is one `eq`.
                members.retain(|m| {
                    !generated.iter().any(|g| matches!(g, Item::Fn(f)
                        if f.name.name == m.name.name
                            && f.scoped_to.as_ref().is_some_and(|t| t.name == name.name)))
                });
                let mut site_hover: Vec<String> = Vec::new();
                let mut site_module: Option<ModulePath> = None;
                for member in members {
                    let Some((template, module)) =
                        world.pick_with_module(&member.name.name, by, target.kind, &mut diags)
                    else {
                        continue;
                    };
                    site_hover.push(compfn_hover_text(&template, &module, Some(&name.name)));
                    site_module = Some(module);
                    let stamp = Stamper::new(&world, &target, &template, by, &mut virtual_next);
                    match stamp.stamp_whole(by.span) {
                        Ok(mut f) => {
                            f.exported = exported;
                            f.scoped_to = Some(name.clone());
                            // The function form (`by tag`) fulfils the member
                            // under the *member's* name.
                            f.name.name = member.name.name.clone();
                            generated.push(Item::Fn(f));
                        }
                        Err(errs) => diags.extend(errs),
                    }
                }
                if !site_hover.is_empty() {
                    hovers.push(CompHover {
                        file: file_idx,
                        span: by.span,
                        text: site_hover.join("\n\n---\n\n"),
                        module: site_module,
                    });
                }
            }
        }

        // [fn-by] Declaration sites, and concrete compfns.
        for item in &mut module.items {
            let Item::Fn(f) = item else { continue };
            if let Some(by) = f.by.clone() {
                let Some(first) = f.params.first() else {
                    diags.push(Diagnostic::error(
                        format!(
                            "`fn {}` says `by`, but has no parameter to say which type it \
                             is stamped at [fn-by]",
                            f.name.name
                        ),
                        by.span,
                    ));
                    fallback_body(f);
                    continue;
                };
                let Some(target_name) = named_base(&first.ty) else {
                    diags.push(Diagnostic::error(
                        format!(
                            "`fn {}` is stamped at the type of its first parameter, which \
                             has to name a struct or a declared union type [fn-by]",
                            f.name.name
                        ),
                        first.ty.span(),
                    ));
                    fallback_body(f);
                    continue;
                };
                let Some(target) = world.target(&target_name) else {
                    diags.push(Diagnostic::error(
                        if world.opaque_here(&target_name) {
                            format!(
                                "`fn {}` is stamped at `{target_name}`, which is opaque outside its \
                                 module: its fields are not this module's to walk, so stamp it \
                                 where `{target_name}` is declared [struct-opaque] [fn-by]",
                                f.name.name
                            )
                        } else {
                            format!(
                                "`fn {}` is stamped at `{target_name}`, which is not a struct or a \
                                 declared union type [fn-by]",
                                f.name.name
                            )
                        },
                        first.ty.span(),
                    ));
                    fallback_body(f);
                    continue;
                };
                let Some((template, module)) = world.pick_with_module(&f.name.name, &by, target.kind, &mut diags) else {
                    fallback_body(f);
                    continue;
                };
                hovers.push(CompHover {
                    file: file_idx,
                    span: by.span,
                    text: compfn_hover_text(&template, &module, Some(&target.name)),
                    module: Some(module.clone()),
                });
                let stamper = Stamper::new(&world, &target, &template, &by, &mut virtual_next);
                match stamper.stamp_into(f) {
                    Ok(()) => {}
                    Err(errs) => {
                        diags.extend(errs);
                        fallback_body(f);
                    }
                }
            } else if f.compfn.as_ref().is_some_and(|c| c.bound.is_none()) {
                // A concrete compfn is its own instantiation: the roots it
                // names are looked up directly.
                let site = f.compfn.as_ref().map(|c| c.span).unwrap_or(f.span);
                let mut ex = Expander::new(&world, None, site, &mut virtual_next);
                ex.params = f.params.clone();
                let mut body = f.body.take();
                if let Some(b) = &mut body {
                    ex.expand_block(b);
                }
                let (regions, errs) = ex.finish();
                if errs.is_empty() {
                    f.body = body;
                    f.stamped = Some(Stamp {
                        compfn: f.name.name.clone(),
                        from: "this declaration".to_string(),
                        at_type: first_param_type_text(f),
                        site,
                        regions,
                    });
                } else {
                    diags.extend(errs);
                    fallback_body(f);
                }
                f.compfn = None;
            }
        }

        // Generic compfns leave the program: nothing downstream could type
        // them, and every use of one has been stamped above.
        module
            .items
            .retain(|item| !matches!(item, Item::Fn(f) if f.compfn.as_ref().is_some_and(|c| c.bound.is_some())));
        module.items.extend(generated);
        out.extend(diags.into_iter().map(|d| (file_idx, d)));
    }
    ComptimeOutput {
        diagnostics: out,
        hovers,
    }
}

/// [comptime-fields] Walks a comptime fn's written body and records, for
/// every comptime name, its type in `core.comptime`'s terms: the bound
/// parameter (`T is Struct`), an `[for …]`/`[when …]` binder (`field: Field`,
/// `arm: Arm`), and the projections (`field.name: Str`, `field.type: Type`,
/// `T.fields: List<Field>`, `T.name: Str`). Read-only over the original AST,
/// so the spans are the file's own and the language server can find them.
struct HoverRecorder<'w> {
    file: usize,
    bound: Option<(String, CompKind)>,
    /// Binders in scope: name → (`Field`/`Arm`, what it walks).
    binders: Vec<(String, bool)>,
    out: &'w mut Vec<CompHover>,
}

impl HoverRecorder<'_> {
    fn push(&mut self, span: Span, code: &str, note: &str) {
        self.out.push(CompHover {
            file: self.file,
            span,
            text: format!("```salvo\n{code}\n```\n\n{note}"),
            module: None,
        });
    }

    fn fn_decl(&mut self, f: &FnDecl) {
        if let Some(cf) = &f.compfn {
            if let Some((kind, id)) = &cf.bound {
                self.push(
                    id.span,
                    &format!("{} is {}", id.name, kind.word()),
                    &format!(
                        "The type this `comptime fn` is stamped at — a `{}` of \
                         `core.comptime`. In a type position it is the type itself; \
                         `{}.name` and `{}.{}` read its compile-time description.",
                        kind.word(),
                        id.name,
                        id.name,
                        if *kind == CompKind::Struct { "fields" } else { "arms" }
                    ),
                );
            }
        }
        if let Some(body) = &f.body {
            self.block(body);
        }
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            self.stmt(s);
        }
    }

    fn comp_ty(&mut self, ct: &CompTy) {
        if ct.via_type {
            if let Some(is_arm) = self.binder_kind(&ct.root.name) {
                let owner = if is_arm { "Arm" } else { "Field" };
                self.push(
                    ct.span,
                    &format!("{}.type: Type", ct.root.name),
                    &format!(
                        "The `{owner}`'s type, as `core.comptime`'s `Type` — `Struct | Union | \
                         Tuple | FnType | Opaque`. Usable wherever a type is written inside \
                         the body; test its arm with `is Struct` or dispatch with `[when …]`."
                    ),
                );
            }
        } else if let Some((param, kind)) = &self.bound {
            if *param == ct.root.name {
                self.push(
                    ct.span,
                    &format!("{} is {}", param, kind.word()),
                    "The bound type parameter, as its compile-time description.",
                );
            }
        }
    }

    fn binder_kind(&self, name: &str) -> Option<bool> {
        self.binders.iter().rev().find(|(n, _)| n == name).map(|(_, arm)| *arm)
    }

    fn seq(&mut self, seq: &CompSeq) {
        self.comp_ty(&seq.ty);
        let (what, elem) = if seq.arms { ("arms", "Arm") } else { ("fields", "Field") };
        self.push(
            seq.span,
            &format!("{}.{what}: List<{elem}>", comp_ty_text(&seq.ty)),
            &format!(
                "The declared {what}, in declaration order; a `[for …]` over it is \
                 unrolled, one copy of the body per `{elem}`."
            ),
        );
    }

    fn binder(&mut self, id: &Ident, is_arm: bool) {
        let ty = if is_arm { "Arm" } else { "Field" };
        self.push(
            id.span,
            &format!("{}: {ty}", id.name),
            &format!(
                "A `core.comptime` `{ty}`: `.name: Str`, `.type: Type`, `.index: Int`, \
                 `.first: Bool`, `.last: Bool`. {}",
                if is_arm {
                    "The value dispatched on reads as this arm inside the block."
                } else {
                    "`v.[field]` reads the field; `[field]: …` names it in a literal."
                }
            ),
        );
    }

    fn cond(&mut self, c: &CompCond) {
        match c {
            CompCond::Kind { ty, .. } | CompCond::Is { ty, .. } | CompCond::Mutable { ty, .. } => {
                self.comp_ty(ty);
                if let CompCond::Mutable { span, .. } = c {
                    self.push(
                        *span,
                        &format!("{}.mutable: Bool", comp_ty_text(ty)),
                        "Whether the type declares `canbe Mut` (`Struct.mutable`).",
                    );
                }
            }
            CompCond::NameEq { binder, span, .. } => {
                if self.binder_kind(&binder.name).is_some() {
                    self.push(
                        Span::new(span.start, binder.span.end + 5),
                        &format!("{}.name: Str", binder.name),
                        "The declared name, compared to a literal at compile time.",
                    );
                }
            }
            CompCond::Flag { binder, last, span } => {
                if self.binder_kind(&binder.name).is_some() {
                    self.push(
                        *span,
                        &format!("{}.{}: Bool", binder.name, if *last { "last" } else { "first" }),
                        "Whether this copy is for the first/last element.",
                    );
                }
            }
            CompCond::IndexCmp { a, b, .. } => {
                for id in [a, b] {
                    if self.binder_kind(&id.name).is_some() {
                        self.push(
                            Span::new(id.span.start, id.span.end + 6),
                            &format!("{}.index: Int", id.name),
                            "The declared position, from zero.",
                        );
                    }
                }
            }
            CompCond::Not(inner, _) => self.cond(inner),
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Comp(c) => match c {
                CompStmt::For {
                    binder,
                    seq,
                    body,
                    ..
                } => {
                    self.seq(seq);
                    self.binder(binder, seq.arms);
                    self.binders.push((binder.name.clone(), seq.arms));
                    self.block(body);
                    self.binders.pop();
                }
                CompStmt::If {
                    cond, then, else_, ..
                } => {
                    self.cond(cond);
                    self.block(then);
                    if let Some(b) = else_ {
                        self.block(b);
                    }
                }
                CompStmt::WhenKind { ty, arms, else_, .. } => {
                    self.comp_ty(ty);
                    for arm in arms {
                        self.block(&arm.body);
                    }
                    if let Some(b) = else_ {
                        self.block(b);
                    }
                }
                CompStmt::WhenArms {
                    value,
                    binder,
                    body,
                    ..
                } => {
                    self.expr(value);
                    self.binder(binder, true);
                    self.binders.push((binder.name.clone(), true));
                    self.block(body);
                    self.binders.pop();
                }
                CompStmt::Refuse { message, .. } => self.expr(message),
            },
            Stmt::Let { value, ty, .. } => {
                if let Some(t) = ty {
                    self.ty(t);
                }
                self.expr(value);
            }
            Stmt::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Stmt::Use { handler, .. } => self.expr(handler),
            Stmt::Rename(_) => {}
            Stmt::Expr(e) => self.expr(e),
        }
    }

    fn ty(&mut self, t: &Type) {
        // `binder.type` in a type position is carried as a dotted name.
        if let Type::Named { base, .. } = t {
            if let Some(b) = base.name.name.strip_suffix(".type") {
                if let Some(is_arm) = self.binder_kind(b) {
                    let owner = if is_arm { "Arm" } else { "Field" };
                    self.push(
                        base.span,
                        &format!("{b}.type: Type"),
                        &format!("The `{owner}`'s type, standing here as the type itself."),
                    );
                }
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        // A small read-only walk: the comptime names an expression can hold
        // are a `Field` whose base is a binder or the bound parameter, and a
        // `v.[binder]` access; everything else recurses.
        match e {
            Expr::Field { base, field, span } => {
                if let Some(b) = field.name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    if self.binder_kind(b).is_some() {
                        self.push(
                            field.span,
                            &format!("[{b}]"),
                            &format!("The field `{b}` is at, in this copy: rewritten to `.name` per field."),
                        );
                    }
                    self.expr(base);
                    return;
                }
                if let Expr::Ident(root) = &**base {
                    if let Some(is_arm) = self.binder_kind(&root.name) {
                        let (ty, note) = match field.name.as_str() {
                            "name" => ("Str", "The declared name, as a literal in each copy."),
                            "index" => ("Int", "The declared position, from zero."),
                            "first" => ("Bool", "Whether this copy is for the first element."),
                            "last" => ("Bool", "Whether this copy is for the last element."),
                            "type" => ("Type", "The compile-time type (`core.comptime`)."),
                            _ => ("?", "Not a projection a `Field`/`Arm` has."),
                        };
                        let _ = is_arm;
                        self.push(*span, &format!("{}.{}: {ty}", root.name, field.name), note);
                        return;
                    }
                    if let Some((param, _)) = &self.bound {
                        if root.name == *param && field.name == "name" {
                            self.push(*span, &format!("{param}.name: Str"), "The stamped type's declared name, as a literal.");
                            return;
                        }
                    }
                }
                self.expr(base);
            }
            Expr::Str { parts, .. } => {
                for p in parts {
                    if let StrExprPart::Interp(inner) = p {
                        self.expr(inner);
                    }
                }
            }
            Expr::Call { callee, args, named, .. } => {
                self.expr(callee);
                for a in args {
                    self.expr(a);
                }
                for n in named {
                    self.expr(&n.value);
                }
            }
            Expr::Unary { operand, .. } | Expr::NonNull { operand, .. } | Expr::IncDec { operand, .. } | Expr::Spread { operand, .. } => self.expr(operand),
            Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Is { subject, .. } | Expr::Widen { subject, .. } => self.expr(subject),
            Expr::If { branches, else_block, .. } => {
                for (c, b) in branches {
                    self.expr(c);
                    self.block(b);
                }
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::When { subject, branches, .. } => {
                self.expr(subject);
                for br in branches {
                    self.block(&br.body);
                }
            }
            Expr::WhenCond { branches, else_block, .. } => {
                for (c, b) in branches {
                    self.expr(c);
                    self.block(b);
                }
                self.block(else_block);
            }
            Expr::While { cond, body, else_block, .. } => {
                self.expr(cond);
                self.block(body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::For { iterable, body, else_block, .. } => {
                self.expr(iterable);
                self.block(body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::Return { value: Some(v), .. } | Expr::Break { value: Some(v), .. } => self.expr(v),
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    match &f.kind {
                        StructLitFieldKind::Named { name, value } => {
                            if let Some(b) = name.name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                                if self.binder_kind(b).is_some() {
                                    self.push(name.span, &format!("[{b}]"), "The entry for the field this copy is for.");
                                }
                            }
                            self.expr(value);
                        }
                        StructLitFieldKind::Spread(e) => self.expr(e),
                        StructLitFieldKind::InlineFor { binder, seq, entries } => {
                            self.seq(seq);
                            self.binder(binder, seq.arms);
                            self.binders.push((binder.name.clone(), seq.arms));
                            for entry in entries {
                                if let StructLitFieldKind::Named { name, value } = &entry.kind {
                                    if let Some(b) = name.name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                                        if self.binder_kind(b).is_some() {
                                            self.push(name.span, &format!("[{b}]"), "The entry for the field this copy is for.");
                                        }
                                    }
                                    self.expr(value);
                                }
                            }
                            self.binders.pop();
                        }
                    }
                }
            }
            Expr::Elvis { subject, rhs, .. } => {
                self.expr(subject);
                self.expr(rhs);
            }
            Expr::Try { body, .. } => self.block(body),
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(e) => self.expr(e),
                LambdaBody::Block(b) => self.block(b),
            },
            Expr::ArrayLit { elems, .. } | Expr::SetLit { elems, .. } | Expr::Tuple { elems, .. } => {
                for e in elems {
                    self.expr(e);
                }
            }
            Expr::MapLit { entries, .. } => {
                for (k, v) in entries {
                    self.expr(k);
                    self.expr(v);
                }
            }
            Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            _ => {}
        }
    }
}

fn comp_ty_text(ct: &CompTy) -> String {
    if ct.via_type {
        format!("{}.type", ct.root.name)
    } else {
        ct.root.name.clone()
    }
}

fn fallback_body(f: &mut FnDecl) {
    // Something the checker will accept without a second diagnostic about
    // the same site: `unreachable!()` is `Never`, so the return check passes.
    let span = f.span;
    f.body = Some(Block {
        stmts: vec![Stmt::Expr(Expr::Unreachable {
            message: None,
            span,
        })],
        span,
    });
    f.by = None;
    f.compfn = None;
}

fn first_param_type_text(f: &FnDecl) -> String {
    f.params
        .first()
        .map(|p| p.ty.to_string())
        .unwrap_or_default()
}

fn named_base(ty: &Type) -> Option<String> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.clone()),
        Type::QualifiedGroup { base, .. } => named_base(base),
        _ => None,
    }
}

struct CompFnDecl {
    module: ModulePath,
    #[allow(dead_code)]
    file: usize,
    decl: FnDecl,
}

/// [comptime-fields] The signature of a comptime fn as a hover code line —
/// `comptime fn cmp<T is Struct>(a: T, b: T) -> Int` — with its module, for
/// the `by` site and the declaration itself.
fn compfn_signature(decl: &FnDecl) -> String {
    let bound = decl
        .compfn
        .as_ref()
        .and_then(|c| c.bound.as_ref())
        .map(|(k, id)| format!("<{} is {}>", id.name, k.word()))
        .unwrap_or_default();
    let params: Vec<String> = decl
        .params
        .iter()
        .map(|p| format!("{}{}: {}", if p.implicit { "?" } else { "" }, p.name.name, p.ty))
        .collect();
    let ret = decl
        .return_type
        .as_ref()
        .map(|t| format!(" -> {t}"))
        .unwrap_or_default();
    format!("comptime fn {}{bound}({}){ret}", decl.name.name, params.join(", "))
}

fn compfn_hover_text(decl: &FnDecl, module: &ModulePath, at_type: Option<&str>) -> String {
    let mut text = format!("```salvo\n{}\n```\n\n", compfn_signature(decl));
    text.push_str(&format!("From `{module}`"));
    if let Some(t) = at_type {
        text.push_str(&format!(", stamped at `{t}`"));
    }
    text.push_str(". Instantiated by `by`, never called as itself [comptime-bound].");
    if !decl.docs.is_empty() {
        text.push_str("\n\n");
        text.push_str(&decl.docs.join("\n"));
    }
    text
}

/// What a `by` site stamps at.
#[derive(Clone)]
struct Target {
    name: String,
    kind: CompKind,
    ty: Type,
    generics: Vec<String>,
    canbe: Vec<String>,
}

struct World {
    compfns: Vec<CompFnDecl>,
    structs: HashMap<String, Vec<(ModulePath, StructDecl)>>,
    types: HashMap<String, Vec<(ModulePath, TypeDecl)>>,
    /// [type-identity] The module being expanded: its own declaration of a
    /// name is the one meant.
    own: std::cell::RefCell<ModulePath>,
    groups: HashMap<String, ParamsDecl>,
    module_paths: Vec<ModulePath>,
}

impl World {
    /// [struct-opaque] Whether `name` is an opaque struct whose fields the
    /// module being expanded may not see: declared elsewhere, and this is not
    /// its `*.test.sv` annex (module `m.test`).
    fn opaque_here(&self, name: &str) -> bool {
        let Some(all) = self.structs.get(name) else { return false };
        let own = self.own.borrow();
        let Some((module, decl)) = all.iter().find(|(m, _)| *m == *own).or_else(|| all.first()) else {
            return false;
        };
        if !decl.opaque || *module == *own {
            return false;
        }
        let annex = own.0.len() == module.0.len() + 1
            && own.0.last().is_some_and(|l| l == "test")
            && own.0[..module.0.len()] == module.0[..];
        !annex
    }

    fn struct_named(&self, name: &str) -> Option<&StructDecl> {
        let all = self.structs.get(name)?;
        let own = self.own.borrow();
        all.iter().find(|(m, _)| *m == *own).or_else(|| all.first()).map(|(_, d)| d)
    }

    fn type_named(&self, name: &str) -> Option<&TypeDecl> {
        let all = self.types.get(name)?;
        let own = self.own.borrow();
        all.iter().find(|(m, _)| *m == *own).or_else(|| all.first()).map(|(_, d)| d)
    }

    fn simple_named(name: &str, span: Span) -> Type {
        Type::Named {
            qualifiers: vec![],
            base: TypeRef {
                name: Ident {
                    name: name.to_string(),
                    span,
                },
                args: vec![],
                value_args: vec![],
                from: vec![],
                at: None,
                binder: false,
                established: false,
                alias: None,
                span,
            },
        }
    }

    /// The target a type name denotes: a struct, or a `type` declaration
    /// whose alias is a union. Anything else is not a `by` target.
    fn target(&self, name: &str) -> Option<Target> {
        // [struct-opaque] Kind `opaque` here: not a struct a comptime fn of kind
        // `Struct` may be stamped at.
        if self.opaque_here(name) {
            return None;
        }
        if let Some(s) = self.struct_named(name) {
            return Some(Target {
                name: name.to_string(),
                kind: CompKind::Struct,
                ty: Self::simple_named(name, s.name.span),
                generics: s.generics.iter().map(|g| g.name.clone()).collect(),
                canbe: s.auto_qualifiers.iter().map(|q| q.name.name.clone()).collect(),
            });
        }
        if let Some(t) = self.type_named(name) {
            // Only a declared *union* is a target: an intrinsic type has no
            // arms, and an alias of something else is not a kind a compfn is
            // bound to.
            let is_union = t.alias.as_ref().is_some_and(|a| matches!(a, Type::Union { .. } | Type::Nullable { .. }));
            if !is_union {
                return None;
            }
            return Some(Target {
                name: name.to_string(),
                kind: CompKind::Union,
                ty: Self::simple_named(name, t.name.span),
                generics: t.generics.iter().map(|g| g.name.clone()).collect(),
                canbe: t.auto_qualifiers.iter().map(|q| q.name.name.clone()).collect(),
            });
        }
        None
    }

    /// The declared arms of a union type, flattened through nullables and
    /// nested written unions (what a `T?` of a union means) but **not**
    /// through an alias, whose arms are positional over its own declaration
    /// [union-arm-identity].
    fn arms_of(&self, ty: &Type) -> Vec<Type> {
        match ty {
            Type::Union { arms, .. } => arms.iter().flat_map(|a| self.arms_of(a)).collect(),
            Type::Nullable { inner, span } => {
                let mut arms = self.arms_of(inner);
                arms.push(Self::simple_named("None", *span));
                arms
            }
            Type::QualifiedGroup { base, .. } => self.arms_of(base),
            other => vec![other.clone()],
        }
    }

    /// [obligation-by] [fn-by] Which compfn a `by` names for `member` at a
    /// type of `kind`: a module's compfn of that name and kind, or a compfn
    /// named directly (the function form).
    /// The module a picked comptime fn lives in, beside it.
    fn module_of(&self, decl: &FnDecl) -> ModulePath {
        self.compfns
            .iter()
            .find(|c| std::ptr::eq(&c.decl, decl) || (c.decl.name.span == decl.name.span && c.decl.name.name == decl.name.name))
            .map(|c| c.module.clone())
            .unwrap_or(ModulePath(vec![]))
    }

    fn pick_with_module(
        &self,
        member: &str,
        by: &ByRef,
        kind: CompKind,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<(FnDecl, ModulePath)> {
        let decl = self.pick_inner(member, by, kind, diags)?;
        let module = self.module_of(&decl);
        Some((decl, module))
    }

    fn pick_inner(
        &self,
        member: &str,
        by: &ByRef,
        kind: CompKind,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<FnDecl> {
        let path: Vec<&str> = by.path.iter().map(|p| p.name.as_str()).collect();
        // The module reading: any unambiguous suffix of the path [mod-suffix],
        // so `by auto` reaches `core.auto` as `size@list` reaches `core.list`.
        let modules: Vec<&ModulePath> = self
            .module_paths
            .iter()
            .filter(|m| m.matches_suffix(&path))
            .collect();
        let mut seen = std::collections::HashSet::new();
        let modules: Vec<&ModulePath> = modules.into_iter().filter(|m| seen.insert(m.to_string())).collect();
        // The function reading: a comptime fn of that name, of the target's
        // kind, in any module — or in the module the `@` selector names.
        let fn_candidates: Vec<&CompFnDecl> = if path.len() == 1 {
            self.compfns
                .iter()
                .filter(|c| c.decl.name.name == path[0])
                .filter(|c| c.decl.compfn.as_ref().and_then(|c| c.bound.as_ref()).is_some_and(|(k, _)| *k == kind))
                .filter(|c| match &by.at {
                    Some(ByAt::Fn(sel)) => {
                        let sel: Vec<&str> = sel.iter().map(|p| p.name.as_str()).collect();
                        c.module.matches_suffix(&sel)
                    }
                    _ => true,
                })
                .collect()
        } else {
            Vec::new()
        };
        // [obligation-by] Both readings fit and nothing was written to choose:
        // refused, naming the two spellings, rather than defaulting to one
        // (user decision 2026-09-29).
        if by.at.is_none() && !modules.is_empty() && !fn_candidates.is_empty() {
            let module = modules[0];
            let fn_module = &fn_candidates[0].module;
            diags.push(Diagnostic::error(
                format!(
                    "`by {0}` is ambiguous: `{0}` is a module (`{module}`) and a `comptime fn` \
                     (in `{fn_module}`). Write `by @{0}` for the module, or \
                     `by {0}@{fn_module}` for the function [obligation-by]",
                    by.text()
                ),
                by.span,
            ));
            return None;
        }
        let want_module = match &by.at {
            Some(ByAt::Module(_)) => true,
            Some(ByAt::Fn(_)) => false,
            None => !modules.is_empty(),
        };
        if want_module {
            if modules.len() > 1 {
                let names: Vec<String> = modules.iter().map(|m| format!("`{m}`")).collect();
                diags.push(Diagnostic::error(
                    format!(
                        "`by {}` names {} modules — {}; write more of the path [obligation-by]",
                        by.text(),
                        modules.len(),
                        names.join(", ")
                    ),
                    by.span,
                ));
                return None;
            }
            let Some(module) = modules.first() else {
                diags.push(Diagnostic::error(
                    format!("`by @{}` names no module [obligation-by]", by.text()),
                    by.span,
                ));
                return None;
            };
            let mut candidates: Vec<&FnDecl> = self
                .compfns
                .iter()
                .filter(|c| &c.module == *module && c.decl.name.name == member)
                .map(|c| &c.decl)
                .collect();
            if candidates.is_empty() {
                diags.push(Diagnostic::error(
                    format!(
                        "`{module}` has no `comptime fn {member}`, so `by {}` cannot stamp \
                         one [obligation-by]",
                        by.text()
                    ),
                    by.span,
                ));
                return None;
            }
            candidates.retain(|c| c.compfn.as_ref().and_then(|c| c.bound.as_ref()).is_some_and(|(k, _)| *k == kind));
            return match candidates.len() {
                0 => {
                    diags.push(Diagnostic::error(
                        format!(
                            "`{module}` has no `comptime fn {member}<T is {}>`: its `{member}` is \
                             not written for a `{}` [comptime-bound]",
                            kind.word(),
                            kind.word()
                        ),
                        by.span,
                    ));
                    None
                }
                1 => Some(candidates[0].clone()),
                _ => {
                    diags.push(Diagnostic::error(
                        format!(
                            "`{module}` declares `comptime fn {member}<T is {}>` more than once \
                             [comptime-bound]",
                            kind.word()
                        ),
                        by.span,
                    ));
                    None
                }
            };
        }
        match fn_candidates.len() {
            1 => return Some(fn_candidates[0].decl.clone()),
            n if n > 1 => {
                let places: Vec<String> = fn_candidates.iter().map(|c| format!("`{}`", c.module)).collect();
                diags.push(Diagnostic::error(
                    format!(
                        "`by {}` names {n} comptime fns of that name for a `{}` (in {}); select \
                         one with `by {}@<module>` [fn-by]",
                        by.text(),
                        kind.word(),
                        places.join(", "),
                        by.text()
                    ),
                    by.span,
                ));
                return None;
            }
            _ => {}
        }
        diags.push(Diagnostic::error(
            format!(
                "`by {}` names neither a module nor a `comptime fn` — nothing to stamp \
                 `{member}` from [obligation-by] [fn-by]",
                by.text()
            ),
            by.span,
        ));
        None
    }

    fn classify(&self, ty: &Type, generics: &[String], depth: u32) -> TypeKindWord {
        match ty {
            Type::Literal { .. } => TypeKindWord::Basic,
            Type::Union { .. } | Type::Nullable { .. } => TypeKindWord::Union,
            Type::Tuple { .. } => TypeKindWord::Tuple,
            Type::Fn { .. } => TypeKindWord::FnType,
            Type::Array { .. } => TypeKindWord::Basic,
            Type::QualifiedGroup { base, .. } => self.classify(base, generics, depth),
            Type::Named { base, .. } => {
                let name = base.name.name.as_str();
                if generics.iter().any(|g| g == name) {
                    return TypeKindWord::Generic;
                }
                if self.structs.contains_key(name) {
                    // [struct-opaque] Outside its module an opaque struct is
                    // kind `opaque`: a comptime body there calls the type's
                    // own functions instead of walking fields it may not see.
                    if self.opaque_here(name) {
                        return TypeKindWord::Basic;
                    }
                    return TypeKindWord::Struct;
                }
                if let Some(t) = self.type_named(name) {
                    if t.intrinsic {
                        return TypeKindWord::Basic;
                    }
                    if let Some(alias) = &t.alias {
                        if depth < 8 {
                            return self.classify(alias, generics, depth + 1);
                        }
                    }
                }
                TypeKindWord::Basic
            }
        }
    }

    fn canbe(&self, ty: &Type, qual: &str) -> bool {
        match ty {
            Type::Named { base, .. } => {
                if let Some(s) = self.struct_named(base.name.name.as_str()) {
                    return s.auto_qualifiers.iter().any(|q| q.name.name == qual);
                }
                if let Some(t) = self.type_named(base.name.name.as_str()) {
                    return t.auto_qualifiers.iter().any(|q| q.name.name == qual);
                }
                false
            }
            Type::QualifiedGroup { base, .. } => self.canbe(base, qual),
            _ => false,
        }
    }

    /// A type's text with aliases expanded one level and qualifiers erased
    /// [qual-erasure], for the compile-time `is <Type>` test.
    fn normalized(&self, ty: &Type) -> String {
        match ty {
            Type::Literal { value, .. } => value.to_string(),
            Type::Named { base, .. } => {
                if let Some(t) = self.type_named(base.name.name.as_str()) {
                    if let Some(alias) = &t.alias {
                        if base.args.is_empty() && !t.intrinsic {
                            return self.normalized(alias);
                        }
                    }
                }
                let mut stripped = base.clone();
                stripped.span = Span::new(0, 0);
                let mut out = stripped.name.name.clone();
                if !stripped.args.is_empty() {
                    let args: Vec<String> = stripped.args.iter().map(|a| self.normalized(a)).collect();
                    out.push('<');
                    out.push_str(&args.join(", "));
                    out.push('>');
                }
                out
            }
            Type::QualifiedGroup { base, .. } => self.normalized(base),
            Type::Union { arms, .. } => {
                let arms: Vec<String> = arms.iter().map(|a| self.normalized(a)).collect();
                arms.join(" | ")
            }
            Type::Nullable { inner, .. } => format!("{} | None", self.normalized(inner)),
            Type::Tuple { elems, .. } => {
                let elems: Vec<String> = elems.iter().map(|e| self.normalized(e)).collect();
                format!("({})", elems.join(", "))
            }
            Type::Array { elem, .. } => format!("{}[]", self.normalized(elem)),
            Type::Fn { .. } => ty.to_string(),
        }
    }
}

/// One `by` site's stamping of one compfn at one target.
struct Stamper<'w> {
    world: &'w World,
    target: &'w Target,
    template: &'w FnDecl,
    by: &'w ByRef,
    virtual_next: &'w mut u32,
}

impl<'w> Stamper<'w> {
    fn new(
        world: &'w World,
        target: &'w Target,
        template: &'w FnDecl,
        by: &'w ByRef,
        virtual_next: &'w mut u32,
    ) -> Self {
        Stamper {
            world,
            target,
            template,
            by,
            virtual_next,
        }
    }

    fn generic_refused(&self, site: Span) -> Option<Diagnostic> {
        if self.target.generics.is_empty() {
            return None;
        }
        Some(Diagnostic::error(
            format!(
                "`{}` is generic, and stamping `{}` at a generic type is not supported yet: \
                 the copy for a field of type `{}` would need an implicit for it — write \
                 the fulfilment by hand, with `?{}<{}>` (ROADMAP §2c)",
                self.target.name,
                self.template.name.name,
                self.target.generics[0],
                group_for(&self.template.name.name),
                self.target.generics[0]
            ),
            site,
        ))
    }

    /// [obligation-by] A whole new fn: the template's signature with `T`
    /// substituted, and its body expanded.
    /// [comptime-generic] What stamping at a **generic** target needs (ROADMAP
    /// §0j 6i, decided with §2c): the target's type parameters, their `canbe`
    /// opt-ins, and one implicit group per parameter a field (or arm) mentions
    /// — `?Ordered<T>` for `cmp`, `?Eq<T>` for `eq`, `?Hashed<T>` for `hash`,
    /// `?ToStr<T>` for `to_str`. A copy meeting an opaque `T` has nothing to
    /// call but what the caller hands in, so the need is the signature's.
    /// `None` for a template whose capability has no group to ask for.
    fn generic_needs(&self, site: Span) -> Option<(Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<TypeRef>)> {
        let group = match self.template.name.name.as_str() {
            "cmp" => "Ordered",
            "eq" => "Eq",
            "hash" => "Hashed",
            "to_str" => "ToStr",
            _ => return None,
        };
        let (generics, canbe, parts): (Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<Type>) =
            if let Some(s) = self.world.struct_named(&self.target.name) {
                (s.generics.clone(), s.generic_canbe.clone(), s.fields.iter().map(|f| f.ty.clone()).collect())
            } else if let Some(t) = self.world.type_named(&self.target.name) {
                let arms = t.alias.as_ref().map(|a| self.world.arms_of(a)).unwrap_or_default();
                (t.generics.clone(), t.generic_canbe.clone(), arms)
            } else {
                return None;
            };
        let needs = generics
            .iter()
            .filter(|g| parts.iter().any(|t| crate::check::type_mentions_generic(t, &g.name)))
            .map(|g| {
                let Type::Named { mut base, .. } = World::simple_named(group, site) else { unreachable!() };
                base.args = vec![World::simple_named(&g.name, site)];
                base
            })
            .collect();
        let generics = generics.into_iter().map(|g| Ident { name: g.name, span: site }).collect();
        Some((generics, canbe, needs))
    }

    fn stamp_whole(self, site: Span) -> Result<FnDecl, Vec<Diagnostic>> {
        let generic = if self.target.generics.is_empty() {
            None
        } else {
            match self.generic_needs(site) {
                Some(needs) => Some(needs),
                None => return Err(vec![self.generic_refused(site).expect("generic target")]),
            }
        };
        let mut f = self.template.clone();
        // Everything the template carries moves into virtual space first, so
        // the target file's own spans are never reused.
        let mut remap = Remap::fresh(&mut f, self.virtual_next);
        visit_mut::walk_fn(&mut remap, &mut f);
        let mut param = self.template.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(_, id)| id.name.clone());
        // [comptime-generic] The template's own parameter is renamed out of
        // the way of the target's (`cmp<T is Struct>` stamped at `Wrapper<T>`):
        // the substitution `T` → `Wrapper<T>` would otherwise meet its own `T`.
        let mut target = self.target.clone();
        if generic.is_some() {
            if let Some(p) = &param {
                if self.target.generics.contains(p) {
                    let fresh = "__Self".to_string();
                    let mut rename = RenameIdent { from: p.clone(), to: fresh.clone() };
                    visit_mut::walk_fn(&mut rename, &mut f);
                    param = Some(fresh);
                }
            }
            if let Type::Named { base, .. } = &mut target.ty {
                base.args = self.target.generics.iter().map(|g| World::simple_named(g, site)).collect();
            }
        }
        let mut ex = Expander::new(self.world, param.map(|p| (p, target)), site, self.virtual_next);
        // Signature substitution: `T` → the target.
        let mut sig = SigSubst { ex: &mut ex };
        for p in &mut f.params {
            visit_mut::walk_type(&mut sig, &mut p.ty);
        }
        if let Some(r) = &mut f.return_type {
            visit_mut::walk_type(&mut sig, r);
        }
        if let Some(b) = &mut f.body {
            ex.expand_block(b);
        }
        let (mut regions, errs) = ex.finish();
        if !errs.is_empty() {
            return Err(errs);
        }
        // [comptime-generic] One `by` site stamps several members (`Hashed`'s
        // `hash` and `eq`), and a generic stamp's implicits are looked up by
        // its name's span: each gets a span of its own, which diagnostics
        // redirect to the site.
        let name_span = if generic.is_some() {
            let start = *self.virtual_next;
            *self.virtual_next = start + 2;
            regions.push(StampRegion {
                start,
                end: start + 1,
                target: site,
                label: format!("`{}` stamped at `{}`", self.template.name.name, self.target.name),
            });
            Span::new(start, start + 1)
        } else {
            site
        };
        f.generics.clear();
        f.generic_canbe.clear();
        if let Some((generics, canbe, needs)) = generic {
            f.generics = generics;
            f.generic_canbe = canbe;
            f.implicit_groups.extend(needs);
        }
        f.compfn = None;
        f.by = None;

        f.docs = vec![format!(
            "The `{}` of [{}], stamped from `{}`'s `compfn` [obligation-by].",
            f.name.name, self.target.name, self.by.text()
        )];
        f.stamped = Some(Stamp {
            compfn: self.template.name.name.clone(),
            from: self.by.text(),
            at_type: self.target.name.clone(),
            site,
            regions,
        });
        f.span = site;
        f.name.span = name_span;
        Ok(f)
    }

    /// [fn-by] Into a written declaration: the body is stamped, and the written
    /// signature is checked against the instantiated one.
    fn stamp_into(self, f: &mut FnDecl) -> Result<(), Vec<Diagnostic>> {
        let site = f.by.as_ref().map(|b| b.span).unwrap_or(f.span);
        if let Some(d) = self.generic_refused(site) {
            return Err(vec![d]);
        }
        let mut inst = self.template.clone();
        let mut remap = Remap::fresh(&mut inst, self.virtual_next);
        visit_mut::walk_fn(&mut remap, &mut inst);
        let param = self.template.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(_, id)| id.name.clone());
        let mut ex = Expander::new(self.world, param.map(|p| (p, self.target.clone())), site, self.virtual_next);
        let mut sig = SigSubst { ex: &mut ex };
        for p in &mut inst.params {
            visit_mut::walk_type(&mut sig, &mut p.ty);
        }
        if let Some(r) = &mut inst.return_type {
            visit_mut::walk_type(&mut sig, r);
        }
        // The written signature has to be the instantiation's, positionally.
        let mut errs = Vec::new();
        if inst.params.len() != f.params.len() {
            errs.push(Diagnostic::error(
                format!(
                    "`fn {}` takes {} parameter(s), but `{}`'s `compfn {}` stamped at `{}` \
                     takes {} [fn-by]",
                    f.name.name,
                    f.params.len(),
                    self.by.text(),
                    self.template.name.name,
                    self.target.name,
                    inst.params.len()
                ),
                site,
            ));
        } else {
            for (w, i) in f.params.iter().zip(&inst.params) {
                if self.world.normalized(&w.ty) != self.world.normalized(&i.ty) {
                    errs.push(Diagnostic::error(
                        format!(
                            "`fn {}`'s parameter `{}` is `{}`, but the stamped `{}` takes `{}` \
                             there [fn-by]",
                            f.name.name,
                            w.name.name,
                            w.ty,
                            self.template.name.name,
                            i.ty
                        ),
                        w.ty.span(),
                    ));
                }
            }
        }
        let written_ret = f.return_type.as_ref().map(|t| self.world.normalized(t)).unwrap_or_else(|| "None".into());
        let inst_ret = inst.return_type.as_ref().map(|t| self.world.normalized(t)).unwrap_or_else(|| "None".into());
        if written_ret != inst_ret {
            errs.push(Diagnostic::error(
                format!(
                    "`fn {}` returns `{}`, but the stamped `{}` returns `{}` [fn-by]",
                    f.name.name,
                    f.return_type.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "None".into()),
                    self.template.name.name,
                    inst.return_type.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "None".into())
                ),
                site,
            ));
        }
        if !errs.is_empty() {
            return Err(errs);
        }
        // The body reads the template's parameter names; rename to the
        // written ones so the written signature is the one the body uses.
        let renames: HashMap<String, String> = inst
            .params
            .iter()
            .zip(&f.params)
            .filter(|(i, w)| i.name.name != w.name.name)
            .map(|(i, w)| (i.name.name.clone(), w.name.name.clone()))
            .collect();
        if let Some(b) = &mut inst.body {
            if !renames.is_empty() {
                let mut r = RenameParams { renames: &renames };
                visit_mut::walk_block(&mut r, b);
            }
            ex.expand_block(b);
        }
        let (regions, errs) = ex.finish();
        if !errs.is_empty() {
            return Err(errs);
        }
        f.body = inst.body;
        if f.effects.is_none() {
            f.effects = inst.effects.clone();
        }
        if f.deductions.is_none() {
            f.deductions = inst.deductions.clone().map(|ds| {
                ds.into_iter()
                    .map(|mut d| {
                        if let DeductionTarget::Param { name, .. } = &mut d.target {
                            if let Some(n) = renames.get(&name.name) {
                                name.name = n.clone();
                            }
                        }
                        d
                    })
                    .collect()
            });
        }
        f.by = None;
        f.stamped = Some(Stamp {
            compfn: self.template.name.name.clone(),
            from: self.by.text(),
            at_type: self.target.name.clone(),
            site,
            regions,
        });
        Ok(())
    }
}

fn group_for(member: &str) -> &'static str {
    match member {
        "cmp" => "Ordered",
        "hash" | "eq" => "Hashed",
        "to_str" => "ToStr",
        _ => "Group",
    }
}

/// [comptime-generic] Renames every identifier spelled `from` — a type
/// parameter's name, which no value, field or fn can share (casing).
struct RenameIdent {
    from: String,
    to: String,
}

impl MutVisitor for RenameIdent {
    fn visit_ident(&mut self, ident: &mut Ident) {
        if ident.name == self.from {
            ident.name = self.to.clone();
        }
    }
}

/// Renames parameter references in a body (the template's names → the
/// written declaration's).
struct RenameParams<'a> {
    renames: &'a HashMap<String, String>,
}

impl MutVisitor for RenameParams<'_> {
    fn visit_expr(&mut self, expr: &mut Expr) {
        if let Expr::Ident(id) = expr {
            if let Some(n) = self.renames.get(&id.name) {
                id.name = n.clone();
            }
        }
    }
}

/// Moves every span of a tree into a fresh virtual region: `new = base + (old
/// - min)`, where `min` is the least span start the tree holds.
struct Remap {
    min: u32,
    base: u32,
}

impl Remap {
    fn fresh<T>(tree: &mut T, virtual_next: &mut u32) -> Remap
    where
        T: SpanBounds,
    {
        let (min, max) = tree.bounds();
        let base = *virtual_next;
        *virtual_next = base + (max.saturating_sub(min)) + 2;
        Remap { min, base }
    }

    fn fresh_for_type_ref(t: &mut TypeRef, virtual_next: &mut u32) -> Remap {
        let mut b = Bounds { min: u32::MAX, max: 0 };
        visit_mut::walk_type_ref(&mut b, t);
        let (min, max) = if b.min == u32::MAX { (0, 0) } else { (b.min, b.max) };
        let base = *virtual_next;
        *virtual_next = base + (max.saturating_sub(min)) + 2;
        Remap { min, base }
    }

    fn region_end(&self, max: u32) -> u32 {
        self.base + (max - self.min) + 1
    }
}

impl MutVisitor for Remap {
    fn visit_span(&mut self, span: &mut Span) {
        let len = span.end.saturating_sub(span.start);
        let start = self.base + span.start.saturating_sub(self.min);
        *span = Span::new(start, start + len);
    }
}

/// The least and greatest span start in a tree.
trait SpanBounds {
    fn bounds(&mut self) -> (u32, u32);
}

struct Bounds {
    min: u32,
    max: u32,
}

impl MutVisitor for Bounds {
    fn visit_span(&mut self, span: &mut Span) {
        self.min = self.min.min(span.start);
        self.max = self.max.max(span.start);
    }
}

impl SpanBounds for FnDecl {
    fn bounds(&mut self) -> (u32, u32) {
        let mut b = Bounds { min: u32::MAX, max: 0 };
        visit_mut::walk_fn(&mut b, self);
        if b.min == u32::MAX {
            (0, 0)
        } else {
            (b.min, b.max)
        }
    }
}

impl SpanBounds for Block {
    fn bounds(&mut self) -> (u32, u32) {
        let mut b = Bounds { min: u32::MAX, max: 0 };
        visit_mut::walk_block(&mut b, self);
        if b.min == u32::MAX {
            (0, 0)
        } else {
            (b.min, b.max)
        }
    }
}

/// A binder's element: one field or one arm.
#[derive(Clone)]
struct Binding {
    ty: Type,
    name: String,
    index: usize,
    first: bool,
    last: bool,
    /// Where a diagnostic about this copy lands.
    decl_span: Span,
    label: String,
    is_arm: bool,
    /// The struct the field belongs to (for a field binder): what a
    /// `field.name == "…"` is checked against.
    owner: Option<String>,
}

/// The body rewriter: unrolls, selects, refuses, and substitutes.
struct Expander<'w> {
    world: &'w World,
    /// The bound parameter and what it stands for, or `None` in a concrete
    /// compfn.
    bound: Option<(String, Target)>,
    env: Vec<(String, Binding)>,
    /// The fn's parameters, for a concrete compfn's `inline when` over one.
    params: Vec<Param>,
    site: Span,
    virtual_next: &'w mut u32,
    regions: Vec<StampRegion>,
    errors: Vec<Diagnostic>,
    rename_counter: u32,
}

impl<'w> Expander<'w> {
    fn new(world: &'w World, bound: Option<(String, Target)>, site: Span, virtual_next: &'w mut u32) -> Self {
        Expander {
            world,
            bound,
            env: Vec::new(),
            params: Vec::new(),
            site,
            virtual_next,
            regions: Vec::new(),
            errors: Vec::new(),
            rename_counter: 0,
        }
    }

    fn finish(self) -> (Vec<StampRegion>, Vec<Diagnostic>) {
        (self.regions, self.errors)
    }

    fn error(&mut self, span: Span, msg: impl Into<String>) {
        // A diagnostic here is about the template or the site; the site is
        // where the author can act, and the message says which.
        let _ = span;
        self.errors.push(Diagnostic::error(msg, self.site));
    }

    fn binding(&self, name: &str) -> Option<&Binding> {
        self.env.iter().rev().find(|(n, _)| n == name).map(|(_, b)| b)
    }

    fn generics(&self) -> Vec<String> {
        self.bound.as_ref().map(|(_, t)| t.generics.clone()).unwrap_or_default()
    }

    /// The type a `CompTy` denotes here.
    fn resolve_ty(&mut self, ct: &CompTy) -> Option<Type> {
        if ct.via_type {
            return match self.binding(&ct.root.name) {
                Some(b) => Some(b.ty.clone()),
                None => {
                    self.error(ct.span, format!("`{}` is not a binder of an enclosing `inline for` or `inline when` [comptime-fields]", ct.root.name));
                    None
                }
            };
        }
        if let Some((param, target)) = &self.bound {
            if *param == ct.root.name {
                return Some(target.ty.clone());
            }
        }
        if self.world.structs.contains_key(&ct.root.name) || self.world.types.contains_key(&ct.root.name) {
            return Some(World::simple_named(&ct.root.name, ct.span));
        }
        self.error(
            ct.span,
            format!(
                "`{}` is not the bound type parameter, a binder's `.type`, or a declared \
                 struct or union type [comptime-fields]",
                ct.root.name
            ),
        );
        None
    }

    /// The elements an `inline for` walks.
    fn elements(&mut self, seq: &CompSeq) -> Option<Vec<Binding>> {
        let ty = self.resolve_ty(&seq.ty)?;
        let kind = self.world.classify(&ty, &self.generics(), 0);
        if seq.arms {
            if kind != TypeKindWord::Union {
                self.error(seq.span, format!("`{}` is a `{}`, and only a `Union` has `.arms` [comptime-fields]", self.world.normalized(&ty), kind.word()));
                return None;
            }
            let arms = self.arms_of_type(&ty);
            let n = arms.len();
            return Some(
                arms.into_iter()
                    .enumerate()
                    .map(|(i, a)| Binding {
                        name: arm_name(&a),
                        decl_span: a.span(),
                        label: format!("{} arm `{}`", self.world.normalized(&ty), a),
                        ty: a,
                        index: i,
                        first: i == 0,
                        last: i + 1 == n,
                        is_arm: true,
                        owner: None,
                    })
                    .collect(),
            );
        }
        if kind != TypeKindWord::Struct {
            self.error(seq.span, format!("`{}` is a `{}`, and only a `Struct` has `.fields` [comptime-fields]", self.world.normalized(&ty), kind.word()));
            return None;
        }
        let Some(name) = named_base(&ty) else { return None };
        let Some(s) = self.world.struct_named(&name) else { return None };
        let n = s.fields.len();
        let type_name = name.clone();
        Some(
            s.fields
                .iter()
                .enumerate()
                .map(|(i, f)| Binding {
                    ty: f.ty.clone(),
                    name: f.name.name.clone(),
                    index: i,
                    first: i == 0,
                    last: i + 1 == n,
                    decl_span: f.span,
                    label: format!("{type_name}.{}: {}", f.name.name, f.ty),
                    is_arm: false,
                    owner: Some(type_name.clone()),
                })
                .collect(),
        )
    }

    fn arms_of_type(&self, ty: &Type) -> Vec<Type> {
        match ty {
            Type::Named { base, .. } => match self.world.type_named(base.name.name.as_str()).and_then(|t| t.alias.as_ref()) {
                Some(alias) => self.world.arms_of(alias),
                None => vec![],
            },
            other => self.world.arms_of(other),
        }
    }

    fn eval_cond(&mut self, cond: &CompCond) -> Option<bool> {
        match cond {
            CompCond::Not(inner, _) => self.eval_cond(inner).map(|b| !b),
            CompCond::Kind { ty, kind, .. } => {
                let t = self.resolve_ty(ty)?;
                let actual = self.world.classify(&t, &self.generics(), 0);
                Some(kind_matches(*kind, actual))
            }
            CompCond::Is { ty, target, .. } => {
                let t = self.resolve_ty(ty)?;
                Some(self.world.normalized(&t) == self.world.normalized(target))
            }
            CompCond::Mutable { ty, .. } => {
                let t = self.resolve_ty(ty)?;
                if let Some((param, target)) = &self.bound {
                    if !ty.via_type && ty.root.name == *param {
                        return Some(target.canbe.iter().any(|q| q == "Mut"));
                    }
                }
                Some(self.world.canbe(&t, "Mut"))
            }
            CompCond::NameEq { binder, lit, span } => {
                let Some(b) = self.binding(&binder.name).cloned() else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", binder.name));
                    return None;
                };
                // A name no field has is a mistake at the template, caught at
                // the first instantiation.
                if let Some(owner) = &b.owner {
                    if let Some(st) = self.world.struct_named(owner) {
                        if !st.fields.iter().any(|f| f.name.name == *lit) {
                            self.error(*span, format!("`{owner}` has no field `{lit}` [comptime-inline]"));
                            return None;
                        }
                    }
                }
                Some(b.name == *lit)
            }
            CompCond::Flag { binder, last, span } => {
                let Some(b) = self.binding(&binder.name) else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", binder.name));
                    return None;
                };
                Some(if *last { b.last } else { b.first })
            }
            CompCond::IndexCmp { a, b, op, span } => {
                let (Some(x), Some(y)) = (self.binding(&a.name).map(|b| b.index), self.binding(&b.name).map(|b| b.index)) else {
                    self.error(*span, "both sides of an `.index` comparison must be binders [comptime-inline]");
                    return None;
                };
                Some(match op {
                    BinaryOp::Eq => x == y,
                    BinaryOp::NotEq => x != y,
                    BinaryOp::Lt => x < y,
                    BinaryOp::Gt => x > y,
                    BinaryOp::LtEq => x <= y,
                    BinaryOp::GtEq => x >= y,
                    _ => false,
                })
            }
        }
    }

    /// One unrolled copy of `template` with `binder` bound to `b`: fresh
    /// spans, locals renamed so two copies in one block do not redeclare,
    /// then substituted and expanded.
    fn copy(&mut self, template: &Block, binder: &str, b: &Binding) -> Block {
        let mut block = template.clone();
        let mut remap = Remap::fresh(&mut block, self.virtual_next);
        let (_, max) = block.bounds();
        let end = remap.region_end(max);
        visit_mut::walk_block(&mut remap, &mut block);
        self.regions.push(StampRegion {
            start: remap.base,
            end,
            target: b.decl_span,
            label: b.label.clone(),
        });
        self.rename_counter += 1;
        rename_locals(&mut block, self.rename_counter);
        self.env.push((binder.to_string(), b.clone()));
        self.expand_block(&mut block);
        self.env.pop();
        block
    }

    /// Expands the comptime statements of a block in place and substitutes
    /// the ordinary ones.
    fn expand_block(&mut self, block: &mut Block) {
        self.expand_comp_stmts(block, true);
    }

    /// The comptime statements only: what the `visit_block` hook runs on a
    /// nested block, whose ordinary statements the walk is about to visit
    /// anyway.
    fn expand_comp_stmts(&mut self, block: &mut Block, walk_plain: bool) {
        let stmts = std::mem::take(&mut block.stmts);
        let mut out: Vec<Stmt> = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            match stmt {
                Stmt::Comp(c) => self.expand_comp(c, &mut out),
                mut other => {
                    if walk_plain {
                        visit_mut::walk_stmt(self, &mut other);
                    }
                    out.push(other);
                }
            }
        }
        block.stmts = out;
    }

    fn expand_comp(&mut self, c: CompStmt, out: &mut Vec<Stmt>) {
        match c {
            CompStmt::For { binder, seq, body, .. } => {
                let Some(elements) = self.elements(&seq) else { return };
                for b in &elements {
                    let copy = self.copy(&body, &binder.name, b);
                    out.extend(copy.stmts);
                }
            }
            CompStmt::If { cond, then, else_, .. } => {
                let Some(taken) = self.eval_cond(&cond) else { return };
                let chosen = if taken { Some(then) } else { else_ };
                if let Some(mut chosen) = chosen {
                    self.expand_block(&mut chosen);
                    out.extend(chosen.stmts);
                }
            }
            CompStmt::WhenKind { ty, arms, else_, span } => {
                let Some(t) = self.resolve_ty(&ty) else { return };
                let actual = self.world.classify(&t, &self.generics(), 0);
                // Exhaustive over the five kinds unless `else` closes it
                // (comptime round 3): a kind added to the language is an error
                // in every compfn that did not consider it.
                if else_.is_none() {
                    let covered = |k: TypeKindWord| arms.iter().any(|a| kind_matches(a.kind, k));
                    let missing: Vec<&str> = [
                        TypeKindWord::Struct,
                        TypeKindWord::Union,
                        TypeKindWord::Tuple,
                        TypeKindWord::FnType,
                        TypeKindWord::Basic,
                        TypeKindWord::Generic,
                    ]
                    .into_iter()
                    .filter(|k| !covered(*k))
                    .map(|k| k.word())
                    .collect();
                    if !missing.is_empty() {
                        self.error(
                            span,
                            format!(
                                "this `[when …]` over a type does not consider every arm of `Type`: \
                                 missing {} — add the arms, or an `else` [comptime-inline] \
                                 [when-exhaustive]",
                                missing.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(", ")
                            ),
                        );
                        return;
                    }
                }
                let chosen = arms
                    .into_iter()
                    .find(|a| kind_matches(a.kind, actual))
                    .map(|a| a.body)
                    .or(else_);
                let Some(mut chosen) = chosen else {
                    self.error(span, format!("`{}` is a `{}`, which this `[when …]` does not handle [comptime-inline]", self.world.normalized(&t), actual.word()));
                    return;
                };
                self.expand_block(&mut chosen);
                out.extend(chosen.stmts);
            }
            CompStmt::WhenArms { mut value, binder, body, span } => {
                // The subject's arms come from what it reads: a `v.[field]`
                // (the field's type) or a parameter of the bound type.
                let subject_ty = match &value {
                    Expr::Field { field, .. } if field.name.starts_with('[') => {
                        let b = field.name.trim_start_matches('[').trim_end_matches(']').to_string();
                        self.binding(&b).map(|b| b.ty.clone())
                    }
                    Expr::Ident(id) => match &self.bound {
                        Some((_, t)) => Some(t.ty.clone()),
                        // A concrete compfn: the parameter's written type.
                        None => self.params.iter().find(|p| p.name.name == id.name).map(|p| p.ty.clone()),
                    },
                    _ => None,
                };
                let Some(subject_ty) = subject_ty else {
                    self.error(span, "a `[when value]` dispatches on a field read (`v.[field]`) or a parameter of the bound type [comptime-inline]");
                    return;
                };
                if self.world.classify(&subject_ty, &self.generics(), 0) != TypeKindWord::Union {
                    self.error(span, format!("`{}` is not a `Union`, so there are no arms to dispatch on [comptime-inline]", self.world.normalized(&subject_ty)));
                    return;
                }
                visit_mut::walk_expr(self, &mut value);
                let arms = self.arms_of_type(&subject_ty);
                let n = arms.len();
                let mut branches = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    let Some(mut check) = arm_check(arm) else {
                        self.error(span, format!("arm `{arm}` of `{}` cannot be dispatched on: only a named arm can be tested with `is` [comptime-inline]", self.world.normalized(&subject_ty)));
                        return;
                    };
                    let b = Binding {
                        ty: arm.clone(),
                        name: arm_name(arm),
                        index: i,
                        first: i == 0,
                        last: i + 1 == n,
                        decl_span: arm.span(),
                        label: format!("{} arm `{}`", self.world.normalized(&subject_ty), arm),
                        is_arm: true,
                        owner: None,
                    };
                    let body = self.copy(&body, &binder.name, &b);
                    // The check's spans come from the union's declaration; two
                    // stamps at one union would key the checker's `is` tables on
                    // the same node, so each copy gets its own.
                    for c in &mut check {
                        let mut remap = Remap::fresh_for_type_ref(c, self.virtual_next);
                        visit_mut::walk_type_ref(&mut remap, c);
                    }
                    let bspan = self.fresh_span();
                    branches.push(WhenBranch {
                        check,
                        binding: None,
                        widen: false,
                        body,
                        span: bspan,
                    });
                }
                let when_span = self.fresh_span();
                out.push(Stmt::Expr(Expr::When {
                    subject: Box::new(value),
                    branches,
                    span: when_span,
                }));
            }
            CompStmt::Refuse { mut message, span } => {
                visit_mut::walk_expr(self, &mut message);
                let text = match literal_text(&message) {
                    Some(t) => t,
                    None => {
                        self.error(span, "a `refuse` message is literal text, with `${T.name}`-style names substituted [comptime-refuse]");
                        return;
                    }
                };
                let at = self.bound.as_ref().map(|(_, t)| t.name.clone()).unwrap_or_default();
                self.errors.push(Diagnostic::error(
                    if at.is_empty() { text } else { format!("`{at}` refused: {text} [comptime-refuse]") },
                    self.site,
                ));
            }
        }
    }

    fn fresh_span(&mut self) -> Span {
        let at = *self.virtual_next;
        *self.virtual_next += 2;
        Span::new(at, at + 1)
    }

    fn str_lit(&self, text: &str, span: Span) -> Expr {
        Expr::Str {
            parts: vec![StrExprPart::Text(text.to_string())],
            span,
        }
    }
}

/// Signature substitution only: `T` → the target, `b.type` → nothing (a
/// signature has no binders).
struct SigSubst<'a, 'w> {
    ex: &'a mut Expander<'w>,
}

impl MutVisitor for SigSubst<'_, '_> {
    fn visit_type(&mut self, ty: &mut Type) {
        self.ex.visit_type(ty);
    }
}

impl MutVisitor for Expander<'_> {
    fn visit_type(&mut self, ty: &mut Type) {
        let Type::Named { qualifiers, base } = ty else { return };
        let replacement = if let Some(b) = base.name.name.strip_suffix(".type") {
            match self.binding(b) {
                Some(bd) => Some(bd.ty.clone()),
                None => {
                    let span = base.span;
                    self.error(span, format!("`{b}` is not a binder of an enclosing `inline for` or `inline when` [comptime-fields]"));
                    None
                }
            }
        } else if let Some((param, target)) = &self.bound {
            if base.name.name == *param && base.args.is_empty() {
                Some(target.ty.clone())
            } else {
                None
            }
        } else {
            None
        };
        let Some(mut new) = replacement else { return };
        if !qualifiers.is_empty() {
            match &mut new {
                Type::Named { qualifiers: q2, .. } => {
                    let mut all = qualifiers.clone();
                    all.extend(q2.drain(..));
                    *q2 = all;
                }
                other => {
                    let span = other.span();
                    new = Type::QualifiedGroup {
                        qualifiers: qualifiers.clone(),
                        base: Box::new(other.clone()),
                        span,
                    };
                }
            }
        }
        *ty = new;
    }

    fn visit_expr(&mut self, expr: &mut Expr) {
        match expr {
            // `v.[field]` → `v.field`; `field.name` / `field.index` /
            // `field.first` / `field.last`; `T.name`.
            Expr::Field { base, field, span } => {
                if let Some(b) = field.name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    match self.binding(b) {
                        Some(bd) if !bd.is_arm => {
                            field.name = bd.name.clone();
                        }
                        Some(_) => {
                            let span = *span;
                            self.error(span, format!("`.[{b}]` reads a field, and `{b}` is an arm binder [comptime-access]"));
                        }
                        None => {
                            let span = *span;
                            self.error(span, format!("`{b}` is not a binder of an enclosing `inline for` [comptime-access]"));
                        }
                    }
                    return;
                }
                let Expr::Ident(root) = &**base else { return };
                let span = *span;
                if let Some(bd) = self.binding(&root.name).cloned() {
                    match field.name.as_str() {
                        "name" => *expr = self.str_lit(&bd.name, span),
                        "index" => {
                            *expr = Expr::Int {
                                value: bd.index as i64,
                                long: false,
                                span,
                            }
                        }
                        "first" => *expr = Expr::Bool { value: bd.first, span },
                        "last" => *expr = Expr::Bool { value: bd.last, span },
                        other => {
                            self.error(span, format!("a `Field`/`Arm` has `.name`, `.type`, `.index`, `.first` and `.last`, not `.{other}` [comptime-fields]"));
                        }
                    }
                    return;
                }
                if let Some((param, target)) = &self.bound {
                    if root.name == *param && field.name == "name" {
                        let name = target.name.clone();
                        *expr = self.str_lit(&name, span);
                    }
                    return;
                }
                if field.name == "name" && root.name.starts_with(|c: char| c.is_uppercase())
                    && (self.world.structs.contains_key(&root.name) || self.world.types.contains_key(&root.name))
                {
                    let name = root.name.clone();
                    *expr = self.str_lit(&name, span);
                }
            }
            // [comptime-inline] A struct literal's `inline for` entries unroll
            // before the walk sees the literal's fields.
            Expr::StructLit { fields, .. } => {
                if fields.iter().any(|f| matches!(f.kind, StructLitFieldKind::InlineFor { .. })) {
                    self.expand_struct_lit_entries(fields);
                }
            }
            // An interpolation of a literal folds into text, so a substituted
            // `${field.name}` reads as the name itself.
            Expr::Str { parts, .. } => {
                let mut folded: Vec<StrExprPart> = Vec::with_capacity(parts.len());
                for part in parts.drain(..) {
                    match part {
                        StrExprPart::Interp(mut inner) => {
                            visit_mut::walk_expr(self, &mut inner);
                            match *inner {
                                Expr::Str { parts: ref ps, .. } if ps.iter().all(|p| matches!(p, StrExprPart::Text(_))) => {
                                    for p in ps {
                                        if let StrExprPart::Text(t) = p {
                                            push_text(&mut folded, t);
                                        }
                                    }
                                }
                                other => folded.push(StrExprPart::Interp(Box::new(other))),
                            }
                        }
                        StrExprPart::Text(t) => push_text(&mut folded, &t),
                    }
                }
                *parts = folded;
            }
            _ => {}
        }
    }

    fn visit_block(&mut self, block: &mut Block) {
        // A nested block (an `if` body, a `when` arm) may hold comptime
        // statements of its own; the walk visits the ordinary ones next.
        if block.stmts.iter().any(|s| matches!(s, Stmt::Comp(_))) {
            self.expand_comp_stmts(block, false);
        }
    }

    fn visit_struct_lit_field(&mut self, field: &mut StructLitField) {
        if let StructLitFieldKind::Named { name, .. } = &mut field.kind {
            if let Some(b) = name.name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                match self.binding(b) {
                    Some(bd) if !bd.is_arm => name.name = bd.name.clone(),
                    _ => {
                        let span = name.span;
                        self.error(span, format!("`[{b}]:` names a field through a binder, and `{b}` is not a field binder of an enclosing `inline for` [comptime-access]"));
                    }
                }
            }
        }
    }

}

impl Expander<'_> {
    /// Struct literals need their `inline for` entries unrolled before the
    /// generic walk sees them; done in `walk_expr`'s hook order by rewriting
    /// the literal here when visited.
    fn expand_struct_lit_entries(&mut self, fields: &mut Vec<StructLitField>) {
        let entries = std::mem::take(fields);
        for entry in entries {
            match entry.kind {
                StructLitFieldKind::InlineFor { binder, seq, entries } => {
                    let Some(elements) = self.elements(&seq) else { continue };
                    for b in &elements {
                        // Each entry is copied per element like a block copy.
                        let mut block = Block {
                            stmts: vec![Stmt::Expr(Expr::StructLit {
                                ty: None,
                                fields: entries.clone(),
                                span: entry.span,
                            })],
                            span: entry.span,
                        };
                        let copy = self.copy(&block, &binder.name, b);
                        block = copy;
                        if let Some(Stmt::Expr(Expr::StructLit { fields: got, .. })) = block.stmts.into_iter().next() {
                            fields.extend(got);
                        }
                    }
                }
                other => fields.push(StructLitField {
                    kind: other,
                    span: entry.span,
                }),
            }
        }
    }
}

fn push_text(parts: &mut Vec<StrExprPart>, t: &str) {
    if let Some(StrExprPart::Text(last)) = parts.last_mut() {
        last.push_str(t);
    } else {
        parts.push(StrExprPart::Text(t.to_string()));
    }
}

fn literal_text(e: &Expr) -> Option<String> {
    let Expr::Str { parts, .. } = e else { return None };
    let mut out = String::new();
    for p in parts {
        match p {
            StrExprPart::Text(t) => out.push_str(t),
            StrExprPart::Interp(inner) => out.push_str(&literal_text(inner)?),
        }
    }
    Some(out)
}

fn kind_matches(written: TypeKindWord, actual: TypeKindWord) -> bool {
    match written {
        TypeKindWord::Opaque => matches!(actual, TypeKindWord::Basic | TypeKindWord::Generic),
        w => w == actual,
    }
}

fn arm_name(arm: &Type) -> String {
    match arm {
        Type::Named { base, .. } => base.name.name.clone(),
        other => other.to_string(),
    }
}

/// The `is` check that selects an arm: its qualifiers then its base.
fn arm_check(arm: &Type) -> Option<Vec<TypeRef>> {
    match arm {
        Type::Named { qualifiers, base } => {
            let mut out = qualifiers.clone();
            out.push(base.clone());
            Some(out)
        }
        _ => None,
    }
}

/// Renames every local an unrolled copy declares (`let` patterns, `is`
/// bindings, `for` patterns), so two copies spliced into one block do not
/// redeclare a name [var-no-shadow]. Field names and parameters are untouched:
/// only names bound *inside* the copy are renamed.
fn rename_locals(block: &mut Block, tag: u32) {
    let mut names: Vec<String> = Vec::new();
    let mut collect = CollectLocals { names: &mut names };
    visit_mut::walk_block(&mut collect, block);
    if names.is_empty() {
        return;
    }
    let renames: HashMap<String, String> = names.into_iter().map(|n| (n.clone(), format!("{n}__c{tag}"))).collect();
    let mut r = RenameLocals { renames: &renames };
    visit_mut::walk_block(&mut r, block);
}

struct CollectLocals<'a> {
    names: &'a mut Vec<String>,
}

impl CollectLocals<'_> {
    fn pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Ident(id) => {
                if !self.names.contains(&id.name) {
                    self.names.push(id.name.clone());
                }
            }
            Pattern::Tuple { elems, .. } => elems.iter().for_each(|e| self.pattern(e)),
            Pattern::Struct { fields, .. } => {
                for f in fields {
                    if !self.names.contains(&f.binding.name) {
                        self.names.push(f.binding.name.clone());
                    }
                }
            }
        }
    }
}

impl MutVisitor for CollectLocals<'_> {
    fn visit_stmt(&mut self, stmt: &mut Stmt) {
        if let Stmt::Let { pattern, .. } = stmt {
            self.pattern(pattern);
        }
    }
    fn visit_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Is { binding: Some(b), .. } | Expr::Widen { binding: Some(b), .. } => {
                if !self.names.contains(&b.name) {
                    self.names.push(b.name.clone());
                }
            }
            Expr::For { pattern, .. } => self.pattern(pattern),
            _ => {}
        }
    }
}

struct RenameLocals<'a> {
    renames: &'a HashMap<String, String>,
}

impl RenameLocals<'_> {
    fn pattern(&self, p: &mut Pattern) {
        match p {
            Pattern::Ident(id) => {
                if let Some(n) = self.renames.get(&id.name) {
                    id.name = n.clone();
                }
            }
            Pattern::Tuple { elems, .. } => elems.iter_mut().for_each(|e| self.pattern(e)),
            Pattern::Struct { fields, .. } => {
                for f in fields {
                    if let Some(n) = self.renames.get(&f.binding.name) {
                        f.binding.name = n.clone();
                    }
                }
            }
        }
    }
}

impl MutVisitor for RenameLocals<'_> {
    fn visit_stmt(&mut self, stmt: &mut Stmt) {
        if let Stmt::Let { pattern, .. } = stmt {
            self.pattern(pattern);
        }
    }
    fn visit_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Ident(id) => {
                if let Some(n) = self.renames.get(&id.name) {
                    id.name = n.clone();
                }
            }
            Expr::Is { binding: Some(b), .. } | Expr::Widen { binding: Some(b), .. } => {
                if let Some(n) = self.renames.get(&b.name) {
                    b.name = n.clone();
                }
            }
            Expr::For { pattern, .. } => self.pattern(pattern),
            _ => {}
        }
    }
}
