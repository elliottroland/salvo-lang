//! [comptime-instantiate] The comptime expansion: every `by` site stamps a
//! `compfn` at a concrete type, and every comptime construct in the copy is
//! unrolled, selected or refused, leaving an ordinary fn with an ordinary body
//! (user decisions 2026-09-28, COMPTIME.md rounds 1–7).
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

/// Runs the expansion. `modules` is aligned with `files`.
pub fn expand_comptime(files: &[SourceFile], modules: &mut [Module]) -> Vec<(usize, Diagnostic)> {
    let mut out = Vec::new();

    // Program-wide tables. Names are flat here — the resolver's visibility
    // rules run later on what this produces — and a collision between two
    // modules' same-named types is the resolver's error, not a reason to
    // stamp nothing.
    let mut compfns: Vec<CompFnDecl> = Vec::new();
    let mut structs: HashMap<String, StructDecl> = HashMap::new();
    let mut types: HashMap<String, TypeDecl> = HashMap::new();
    let mut groups: HashMap<String, ParamsDecl> = HashMap::new();
    for (idx, (file, module)) in files.iter().zip(modules.iter()).enumerate() {
        for item in &module.items {
            match item {
                Item::Fn(f) if f.compfn.as_ref().is_some_and(|c| c.bound.is_some()) => {
                    compfns.push(CompFnDecl {
                        module: file.module.clone(),
                        file: idx,
                        decl: f.clone(),
                    });
                }
                Item::Struct(s) => {
                    structs.entry(s.name.name.clone()).or_insert_with(|| s.clone());
                }
                Item::Type(t) => {
                    types.entry(t.name.name.clone()).or_insert_with(|| t.clone());
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
        groups,
        module_paths: files.iter().map(|f| f.module.clone()).collect(),
    };

    for (file_idx, (file, module)) in files.iter().zip(modules.iter_mut()).enumerate() {
        let mut diags: Vec<Diagnostic> = Vec::new();
        let mut virtual_next = file.content.len() as u32 + 1;
        let mut generated: Vec<Item> = Vec::new();

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
                for member in members {
                    let Some(template) =
                        world.pick(&member.name.name, by, target.kind, &mut diags)
                    else {
                        continue;
                    };
                    let stamp = Stamper::new(&world, &target, &template, by, &mut virtual_next);
                    match stamp.stamp_whole(by.span) {
                        Ok(mut f) => {
                            f.exported = exported;
                            f.scoped_to = Some(name.clone());
                            generated.push(Item::Fn(f));
                        }
                        Err(errs) => diags.extend(errs),
                    }
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
                        format!(
                            "`fn {}` is stamped at `{target_name}`, which is not a struct or a \
                             declared union type [fn-by]",
                            f.name.name
                        ),
                        first.ty.span(),
                    ));
                    fallback_body(f);
                    continue;
                };
                let Some(template) = world.pick(&f.name.name, &by, target.kind, &mut diags) else {
                    fallback_body(f);
                    continue;
                };
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
    out
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

/// What a `by` site stamps at.
#[derive(Clone)]
struct Target {
    name: String,
    kind: CompKind,
    ty: Type,
    generics: Vec<String>,
    fields: Vec<FieldDecl>,
    canbe: Vec<String>,
}

struct World {
    compfns: Vec<CompFnDecl>,
    structs: HashMap<String, StructDecl>,
    types: HashMap<String, TypeDecl>,
    groups: HashMap<String, ParamsDecl>,
    module_paths: Vec<ModulePath>,
}

impl World {
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
        if let Some(s) = self.structs.get(name) {
            return Some(Target {
                name: name.to_string(),
                kind: CompKind::Struct,
                ty: Self::simple_named(name, s.name.span),
                generics: s.generics.iter().map(|g| g.name.clone()).collect(),
                fields: s.fields.clone(),
                canbe: s.auto_qualifiers.iter().map(|q| q.name.name.clone()).collect(),
            });
        }
        if let Some(t) = self.types.get(name) {
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
                fields: vec![],
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
    fn pick(
        &self,
        member: &str,
        by: &ByRef,
        kind: CompKind,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<FnDecl> {
        let path: Vec<&str> = by.path.iter().map(|p| p.name.as_str()).collect();
        // A module: matched by path suffix, so `by auto` reaches `core.auto`
        // without an import — being in `core` means nameable, nothing more.
        let modules: Vec<&ModulePath> = self
            .module_paths
            .iter()
            .filter(|m| m.0.len() >= path.len() && m.0[m.0.len() - path.len()..] == path[..])
            .collect();
        let mut seen = std::collections::HashSet::new();
        let modules: Vec<&ModulePath> = modules.into_iter().filter(|m| seen.insert(m.to_string())).collect();
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
        if let Some(module) = modules.first() {
            let mut candidates: Vec<&FnDecl> = self
                .compfns
                .iter()
                .filter(|c| &c.module == *module && c.decl.name.name == member)
                .map(|c| &c.decl)
                .collect();
            if candidates.is_empty() {
                diags.push(Diagnostic::error(
                    format!(
                        "`{module}` has no `compfn {member}`, so `by {}` cannot stamp one \
                         [obligation-by]",
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
                            "`{module}` has no `compfn {member}<{} T>`: its `{member}` is not \
                             written for a {} [comptime-bound]",
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
                            "`{module}` declares `compfn {member}<{} T>` more than once \
                             [comptime-bound]",
                            kind.word()
                        ),
                        by.span,
                    ));
                    None
                }
            };
        }
        // The function form: a compfn named directly.
        if path.len() == 1 {
            let candidates: Vec<&FnDecl> = self
                .compfns
                .iter()
                .filter(|c| c.decl.name.name == path[0])
                .filter(|c| c.decl.compfn.as_ref().and_then(|c| c.bound.as_ref()).is_some_and(|(k, _)| *k == kind))
                .map(|c| &c.decl)
                .collect();
            match candidates.len() {
                1 => return Some(candidates[0].clone()),
                n if n > 1 => {
                    diags.push(Diagnostic::error(
                        format!(
                            "`by {}` names {n} compfns of that name for a {}; write the module \
                             path instead [fn-by]",
                            by.text(),
                            kind.word()
                        ),
                        by.span,
                    ));
                    return None;
                }
                _ => {}
            }
        }
        diags.push(Diagnostic::error(
            format!(
                "`by {}` names neither a module nor a `compfn` — nothing to stamp `{member}` \
                 from [obligation-by] [fn-by]",
                by.text()
            ),
            by.span,
        ));
        None
    }

    fn classify(&self, ty: &Type, generics: &[String], depth: u32) -> TypeKindWord {
        match ty {
            Type::Union { .. } | Type::Nullable { .. } => TypeKindWord::Union,
            Type::Tuple { .. } => TypeKindWord::Tuple,
            Type::Fn { .. } => TypeKindWord::Fn,
            Type::Array { .. } => TypeKindWord::Basic,
            Type::QualifiedGroup { base, .. } => self.classify(base, generics, depth),
            Type::Named { base, .. } => {
                let name = base.name.name.as_str();
                if generics.iter().any(|g| g == name) {
                    return TypeKindWord::Generic;
                }
                if self.structs.contains_key(name) {
                    return TypeKindWord::Struct;
                }
                if let Some(t) = self.types.get(name) {
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
                if let Some(s) = self.structs.get(base.name.name.as_str()) {
                    return s.auto_qualifiers.iter().any(|q| q.name.name == qual);
                }
                if let Some(t) = self.types.get(base.name.name.as_str()) {
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
            Type::Named { base, .. } => {
                if let Some(t) = self.types.get(base.name.name.as_str()) {
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
                 the fulfilment by hand, with `?{}<{}>` (COMPTIME.md 12.3)",
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
    fn stamp_whole(self, site: Span) -> Result<FnDecl, Vec<Diagnostic>> {
        if let Some(d) = self.generic_refused(site) {
            return Err(vec![d]);
        }
        let mut f = self.template.clone();
        // Everything the template carries moves into virtual space first, so
        // the target file's own spans are never reused.
        let mut remap = Remap::fresh(&mut f, self.virtual_next);
        visit_mut::walk_fn(&mut remap, &mut f);
        let param = self.template.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(_, id)| id.name.clone());
        let mut ex = Expander::new(self.world, param.map(|p| (p, self.target.clone())), site, self.virtual_next);
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
        let (regions, errs) = ex.finish();
        if !errs.is_empty() {
            return Err(errs);
        }
        f.generics.clear();
        f.generic_canbe.clear();
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
        f.name.span = site;
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
}

/// The body rewriter: unrolls, selects, refuses, and substitutes.
struct Expander<'w> {
    world: &'w World,
    /// The bound parameter and what it stands for, or `None` in a concrete
    /// compfn.
    bound: Option<(String, Target)>,
    env: Vec<(String, Binding)>,
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
                self.error(seq.span, format!("`{}` is a {}, and only a union has `.arms` [comptime-fields]", self.world.normalized(&ty), kind.word()));
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
                    })
                    .collect(),
            );
        }
        if kind != TypeKindWord::Struct {
            self.error(seq.span, format!("`{}` is a {}, and only a struct has `.fields` [comptime-fields]", self.world.normalized(&ty), kind.word()));
            return None;
        }
        let Some(name) = named_base(&ty) else { return None };
        let Some(s) = self.world.structs.get(&name) else { return None };
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
                })
                .collect(),
        )
    }

    fn arms_of_type(&self, ty: &Type) -> Vec<Type> {
        match ty {
            Type::Named { base, .. } => match self.world.types.get(base.name.name.as_str()).and_then(|t| t.alias.as_ref()) {
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
            CompCond::Canbe { ty, qual, .. } => {
                let t = self.resolve_ty(ty)?;
                if let Some((param, target)) = &self.bound {
                    if !ty.via_type && ty.root.name == *param {
                        return Some(target.canbe.iter().any(|q| *q == qual.name.name));
                    }
                }
                Some(self.world.canbe(&t, &qual.name.name))
            }
            CompCond::NameEq { binder, lit, span } => {
                let Some(b) = self.binding(&binder.name).cloned() else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", binder.name));
                    return None;
                };
                // A name no field has is a mistake at the template, caught at
                // the first instantiation.
                if !b.is_arm {
                    if let Some(t) = self.bound.as_ref().map(|(_, t)| t.clone()) {
                        if t.kind == CompKind::Struct && !t.fields.iter().any(|f| f.name.name == *lit) {
                            self.error(*span, format!("`{}` has no field `{lit}` [comptime-inline]", t.name));
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
                        TypeKindWord::Fn,
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
                                "this `inline when` does not consider every kind: missing {} — add \
                                 the arms, or an `else` [comptime-inline]",
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
                    self.error(span, format!("`{}` is a {}, which this `inline when` does not handle [comptime-inline]", self.world.normalized(&t), actual.word()));
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
                    Expr::Ident(_) => self.bound.as_ref().map(|(_, t)| t.ty.clone()),
                    _ => None,
                };
                let Some(subject_ty) = subject_ty else {
                    self.error(span, "an `inline when` over a value dispatches on a field read (`v.[field]`) or a parameter of the bound type [comptime-inline]");
                    return;
                };
                if self.world.classify(&subject_ty, &self.generics(), 0) != TypeKindWord::Union {
                    self.error(span, format!("`{}` is not a union, so there are no arms to dispatch on [comptime-inline]", self.world.normalized(&subject_ty)));
                    return;
                }
                visit_mut::walk_expr(self, &mut value);
                let arms = self.arms_of_type(&subject_ty);
                let n = arms.len();
                let mut branches = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    let Some(check) = arm_check(arm) else {
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
                    };
                    let body = self.copy(&body, &binder.name, &b);
                    let bspan = body.span;
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
                            self.error(span, format!("a binder has `.name`, `.type`, `.index`, `.first` and `.last`, not `.{other}` [comptime-fields]"));
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
