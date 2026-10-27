//! [comptime-instantiate] The comptime expansion: every `by` site stamps a
//! `compfn` at a concrete type, and every comptime construct in the copy is
//! unrolled, selected or refused, leaving an ordinary fn with an ordinary body
//! (user decisions 2026-09-28, the comptime rounds; COMPLETED.md's log).
//!
//! Runs before resolution, over the whole source set, because a `by auto` in
//! any file reads `std.core.auto`'s compfns and the target type's declaration,
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
//! Origins [origin]: every node of a stamped body keeps the span it has in the
//! template, and carries a fresh **expansion** id (`Span::expn`), one per
//! unrolled copy, so the checker's span-keyed side tables never see two copies
//! as one node. The `Stamp` on the fn records each expansion (what it is a copy
//! of, the field or arm it was for), and a diagnostic inside one renders through
//! that chain (`crate::origin::render`): at the field or arm the copy was for,
//! prefixed with what was being stamped.

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
    /// Markdown: a code line with the name and its `std.core.comptime` type, then
    /// a sentence.
    pub text: String,
    /// [doc-module] The module a `by` site took its comptime fn from, when
    /// the hover is a `by` site: the language server appends that module's
    /// own documentation, since `by @auto` names the module as much as
    /// `size@list` does.
    pub module: Option<ModulePath>,
}

/// [lsp-completion] [when-exhaustive] The arms a comptime `[when …]` has yet
/// to cover, at its written span: kind words for `[when x.kind]`, quoted names
/// for `[when f.name]`, the uncovered types for `[when f.type]` — what the
/// language server offers as the arms still to write. Empty when it covers
/// its range. Only where the range is known where the fn is written: an
/// expanded-in-place fn, and any `[when x.kind]`.
#[derive(Clone, Debug)]
pub struct CompWhenArms {
    pub file: usize,
    pub span: Span,
    /// What follows `is` in each missing arm.
    pub missing: Vec<String>,
    /// A `[when f.type]`: `is *` covers the rest, so it is offered too.
    pub types: bool,
    /// The written arms' bodies: a position inside one is not between arms.
    /// (The language server cannot read them off the program, whose comptime
    /// fns are expanded by then.)
    pub bodies: Vec<Span>,
}

pub struct ComptimeOutput {
    pub diagnostics: Vec<(usize, Diagnostic)>,
    pub hovers: Vec<CompHover>,
    pub when_arms: Vec<CompWhenArms>,
}

/// [comptime-instantiate] What the world looks declarations up in: every
/// comptime fn, and every struct, `type`, `params` group and qualifier by
/// name, as the files declared them **before** any was stamped (stamping
/// removes the generic compfns). Names are flat here — the resolver's
/// visibility rules run later on what this produces — and a collision between
/// two modules' same-named types is the resolver's error, not a reason to
/// stamp nothing.
#[derive(Default)]
struct Tables {
    compfns: Vec<CompFnDecl>,
    structs: HashMap<String, Vec<(ModulePath, StructDecl)>>,
    types: HashMap<String, Vec<(ModulePath, TypeDecl)>>,
    groups: HashMap<String, Vec<(ModulePath, ParamsDecl)>>,
    /// [comptime-qualifier] Every qualifier, by name: a head tests for the
    /// comptime ones, and an arm's constructor reads an ordinary one's
    /// generics [comptime-arm].
    quals: HashMap<String, Vec<(ModulePath, QualifierDecl)>>,
}

/// [lsp-std-front] The comptime expansion of the embedded std on its own,
/// kept so a language server can expand a project over std's already
/// expanded modules ([`expand_comptime_after`]): std's tables as they were
/// before std was stamped.
pub struct StdComptime {
    /// How many leading files of a program the std is.
    files: usize,
    tables: std::sync::Arc<Tables>,
}

impl StdComptime {
    /// How many leading files of a program the std is.
    pub fn files(&self) -> usize {
        self.files
    }

}

/// [lsp-std-front] Whether the per-file stamping has anything to do in a
/// module: a comptime fn, a `fn … by`, or a `by` clause. One that has none is
/// left exactly as it is, so it is not even made mutable (a shared module
/// stays shared).
fn stamps_anything(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Fn(f) => f.compfn.is_some() || f.by.is_some(),
        Item::Struct(s) => s.obligations.iter().any(|o| o.by.is_some()),
        Item::Type(t) => t.obligations.iter().any(|o| o.by.is_some()),
        _ => false,
    })
}

/// Runs the expansion. `modules` is aligned with `files`.
pub fn expand_comptime<M: crate::expand::ModuleSlot>(files: &[SourceFile], modules: &mut [M]) -> ComptimeOutput {
    run(files, modules, None).0
}

/// [lsp-std-front] Runs the expansion over a std on its own (`files` is the
/// embedded std, nothing else), and keeps what [`expand_comptime_after`]
/// needs to expand a project over the result.
pub fn expand_comptime_prefix<M: crate::expand::ModuleSlot>(
    files: &[SourceFile],
    modules: &mut [M],
) -> (ComptimeOutput, StdComptime) {
    let (out, tables) = run(files, modules, None);
    let prefix = StdComptime {
        files: files.len(),
        tables: std::sync::Arc::new(tables),
    };
    (out, prefix)
}

/// [lsp-std-front] Runs the expansion over a program whose leading
/// `prefix.files()` files are the std `prefix` was recorded from, already
/// expanded: the world reads std's tables from the prefix, scopes are
/// resolved for the later files only, and only the later files are stamped.
/// The std files' own output (diagnostics, hovers, `[when]` arms) is the
/// prefix's, not repeated here. Sound for every program: std sees only std
/// [obligation-by] — a `by` path names only the modules its file can see, a
/// name std's scope does not see falls back to std's declarations alone, and
/// what std sees implicitly is `std.core.*`, which no project can declare
/// [std-root].
pub fn expand_comptime_after(
    files: &[SourceFile],
    modules: &mut [std::sync::Arc<Module>],
    prefix: &StdComptime,
) -> ComptimeOutput {
    run(files, modules, Some((prefix.tables.clone(), prefix.files))).0
}

/// The expansion: the files from `start` on (every file without a `base`)
/// are collected into the world's tables, scoped and stamped; `base` holds
/// the tables of the files before `start`. Returns the run's own tables
/// beside its output.
fn run<M: crate::expand::ModuleSlot>(
    files: &[SourceFile],
    modules: &mut [M],
    base: Option<(std::sync::Arc<Tables>, usize)>,
) -> (ComptimeOutput, Tables) {
    let mut out = Vec::new();
    let mut hovers: Vec<CompHover> = Vec::new();
    let mut when_arms: Vec<CompWhenArms> = Vec::new();
    let start = base.as_ref().map_or(0, |(_, n)| *n);

    let mut tables = Tables::default();
    for idx in start..files.len() {
        let (file, module) = (&files[idx], modules[idx].borrow());
        for item in &module.items {
            match item {
                Item::Fn(f) if f.compfn.as_ref().is_some_and(|c| c.bound.is_some()) => {
                    // [when-exhaustive] A subject `[when]` over the bound
                    // parameter's range is refused at the template; the
                    // template still answers its `by` sites, with a body that
                    // says nothing more, so the one mistake is one diagnostic.
                    let refused = unchecked_subject_whens(f);
                    let mut decl = f.clone();
                    if !refused.is_empty() {
                        out.extend(refused.into_iter().map(|d| (idx, d)));
                        let compfn = decl.compfn.take();
                        fallback_body(&mut decl);
                        decl.compfn = compfn;
                    }
                    tables.compfns.push(CompFnDecl {
                        module: file.module.clone(),
                        file: idx,
                        decl,
                    });
                }
                // [type-identity] Every declaration of a name, by module: one
                // stamped in a module reads that module's own first.
                Item::Struct(s) => {
                    tables.structs.entry(s.name.name.clone()).or_default().push((file.module.clone(), s.clone()));
                }
                Item::Type(t) => {
                    tables.types.entry(t.name.name.clone()).or_default().push((file.module.clone(), t.clone()));
                }
                Item::Params(g) => {
                    tables.groups.entry(g.name.name.clone()).or_default().push((file.module.clone(), g.clone()));
                }
                // [comptime-qualifier] Comptime qualifiers, which a head tests
                // for (`[if field is Short]`).
                Item::Qualifier(q) => {
                    tables.quals.entry(q.name.name.clone()).or_default().push((file.module.clone(), q.clone()));
                }
                _ => {}
            }
        }
    }
    let t = crate::timing::phase("  expand: comptime scopes");
    let scoped = scoped_names(files, modules, start);
    drop(t);
    let _stamp = crate::timing::phase("  expand: comptime stamps");
    let world = World {
        scoped,
        here: std::cell::Cell::new(0),
        base: base.map(|(tables, _)| tables),
        tables,
        std_here: std::cell::Cell::new(false),
        std_modules: files.iter().filter(|f| f.is_std).map(|f| f.module.clone()).collect(),
        own: std::cell::RefCell::new(ModulePath(Vec::new())),
        imports: std::cell::RefCell::new(Vec::new()),
        module_paths: files.iter().map(|f| f.module.clone()).collect(),
    };

    for file_idx in start..files.len() {
        if !stamps_anything(modules[file_idx].borrow()) {
            continue;
        }
        let file = &files[file_idx];
        let module = modules[file_idx].module_mut();
        *world.own.borrow_mut() = file.module.clone();
        world.here.set(file_idx);
        world.std_here.set(file.is_std);
        *world.imports.borrow_mut() = module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Import(i) => Some(i.path.iter().map(|p| p.name.clone()).collect()),
                _ => None,
            })
            .collect();
        let mut diags: Vec<Diagnostic> = Vec::new();
        // [origin] The file's expansion ids: 0 is written code.
        let mut next_expn: u32 = 1;
        let mut generated: Vec<Item> = Vec::new();

        // [comptime-fields] Hovers over every comptime fn body, at the written
        // spans, before the bodies are consumed by stamping. The same walk
        // collects the type names an expanded-in-place fn names directly
        // [comptime-root-visible].
        let mut roots_by_fn: HashMap<u32, Vec<Ident>> = HashMap::new();
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
                roots: Vec::new(),
                out: &mut hovers,
                when_arms: &mut when_arms,
            };
            rec.fn_decl(f);
            if cf.bound.is_none() {
                roots_by_fn.insert(f.name.span.start, std::mem::take(&mut rec.roots));
            }
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
                // [type-identity] The group the clause's file means.
                let Some(group) = world.group_named(ob.group.name.name.as_str()) else {
                    // The checker reports the unknown group at the clause.
                    continue;
                };
                // [type-identity] The target is the declaration the clause is
                // written on, not whatever its name finds program-wide.
                let Some(target) = World::target_of_item(item) else { continue };
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
                    let Some((template, module, template_file)) =
                        world.pick_with_module(&member.name.name, by, target.kind, &mut diags)
                    else {
                        continue;
                    };
                    site_hover.push(compfn_hover_text(&template, &module, Some(&name.name)));
                    site_module = Some(module);
                    // [obligation-by] [fn-by] The clause is one `fn … by X` per
                    // member (user decision 2026-10-14): the member's
                    // signature with `self` substituted is the declaration,
                    // and the stamp fills its body exactly as it fills a
                    // written one.
                    let stamp = Stamper::new(&world, &target, &template, by, &mut next_expn);
                    let mut f = match stamp.member_decl(group, member) {
                        Ok(f) => f,
                        Err(errs) => {
                            diags.extend(errs);
                            continue;
                        }
                    };
                    f.exported = exported;
                    f.scoped_to = Some(name.clone());
                    let stamp = Stamper::new(&world, &target, &template, by, &mut next_expn);
                    match stamp.stamp_into(&mut f, Declared::Member { group: &group.name.name }) {
                        Ok(()) => {
                            if let Some(s) = &mut f.stamped {
                                s.template_file = template_file;
                            }
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

        // [fn-by] Declaration sites, and expanded-in-place compfns.
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
                let Some((template, module, template_file)) = world.pick_with_module(&f.name.name, &by, target.kind, &mut diags) else {
                    fallback_body(f);
                    continue;
                };
                hovers.push(CompHover {
                    file: file_idx,
                    span: by.span,
                    text: compfn_hover_text(&template, &module, Some(&target.name)),
                    module: Some(module.clone()),
                });
                let stamper = Stamper::new(&world, &target, &template, &by, &mut next_expn);
                // [comptime-generic] A written declaration at a generic type
                // would have to spell the generic additions itself; not yet.
                if let Some(d) = stamper.generic_refused(by.span) {
                    diags.push(d);
                    fallback_body(f);
                    continue;
                }
                match stamper.stamp_into(f, Declared::Written) {
                    Ok(()) => {
                        if let Some(s) = &mut f.stamped {
                            s.template_file = template_file;
                        }
                    }
                    Err(errs) => {
                        diags.extend(errs);
                        fallback_body(f);
                    }
                }
            } else if f.compfn.as_ref().is_some_and(|c| c.bound.is_none()) {
                // [comptime-bound] An expanded-in-place compfn is its own
                // instantiation: expanded once, here, and an ordinary
                // (callable) fn from then on. The roots it names are looked
                // up directly.
                let site = f.compfn.as_ref().map(|c| c.span).unwrap_or(f.span);
                let mut ex = Expander::new(&world, None, site, &mut next_expn);
                ex.params = f.params.clone();
                ex.in_place = true;
                ex.fn_name = Some(f.name.name.clone());
                ex.fn_generics = f.generics.iter().map(|g| g.name.clone()).collect();
                let mut body = f.body.take();
                if let Some(b) = &mut body {
                    ex.expand_block(b);
                }
                // [lsp-completion] One record per written `[when]`: the copies'
                // uncovered arms, merged in order.
                for (span, missing, types, bodies) in std::mem::take(&mut ex.when_arms) {
                    match when_arms.iter_mut().find(|w| w.file == file_idx && w.span == span) {
                        Some(w) => {
                            for m in missing {
                                if !w.missing.contains(&m) {
                                    w.missing.push(m);
                                }
                            }
                        }
                        None => when_arms.push(CompWhenArms { file: file_idx, span, missing, types, bodies }),
                    }
                }
                let (expansions, mut errs) = ex.finish();
                // Warnings ride along; only an error drops the expansion.
                diags.extend(errs.iter().filter(|d| !d.is_error()).cloned());
                errs.retain(|d| d.is_error());
                // [comptime-root-visible] Only names the expansion read as a
                // declared type are roots; the checker requires each visible.
                let roots: Vec<Ident> = roots_by_fn
                    .remove(&f.name.span.start)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|r| world.declares_type(&r.name))
                    .collect();
                if errs.is_empty() {
                    f.body = body;
                    f.stamped = Some(Stamp {
                        compfn: f.name.name.clone(),
                        from: "this declaration".to_string(),
                        at_type: first_param_type_text(f),
                        site,
                        expansions,
                        roots,
                        template_file: None,
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
    let World { tables, .. } = world;
    (
        ComptimeOutput {
            diagnostics: out,
            hovers,
            when_arms,
        },
        tables,
    )
}

/// [comptime-fields] Walks a comptime fn's written body and records, for
/// every comptime name, its type in `std.core.comptime`'s terms: the bound
/// parameter (`T is Struct`), an `[for …]`/`[when …]` binder (`field: Field`,
/// `arm: Arm`), and the projections (`field.name: Str`, `field.type: Type`,
/// `T.fields: List<Field>`, `T.name: Str`). Read-only over the original AST,
/// so the spans are the file's own and the language server can find them.
struct HoverRecorder<'w> {
    file: usize,
    bound: Option<(String, CompKind)>,
    /// Binders in scope: name → (`Field`/`Arm`, what it walks).
    binders: Vec<(String, bool)>,
    /// [comptime-root-visible] Type names written as comptime roots.
    roots: Vec<Ident>,
    out: &'w mut Vec<CompHover>,
    /// [lsp-completion] Each `[when x.kind]`'s missing kinds: known from its
    /// arms alone, so recorded from the written body.
    when_arms: &'w mut Vec<CompWhenArms>,
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
                    &format!("{}.type", ct.root.name),
                    &format!(
                        "The `{owner}`'s type — usable wherever a type is written inside \
                         the body, and tested with `is <Type>`. Its kind (`core.comptime`'s \
                         `Kind`: `Struct | Union | Tuple | FnType | Opaque`) is \
                         `{}.kind`, tested with `is Struct` or dispatched with `[when {}.kind]`.",
                        ct.root.name, ct.root.name
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
            } else {
                self.roots.push(ct.root.clone());
            }
        } else {
            self.roots.push(ct.root.clone());
        }
        // [comptime-root-visible] The type names written in an instantiated
        // root's arguments (`Int` in `Arg<Int>.fields`) are written here too.
        for a in &ct.args {
            collect_type_names(a, &mut self.roots);
        }
    }

    fn binder_kind(&self, name: &str) -> Option<bool> {
        self.binders.iter().rev().find(|(n, _)| n == name).map(|(_, arm)| *arm)
    }

    fn seq(&mut self, seq: &CompSeq) {
        self.comp_ty(&seq.ty);
        let (what, elem) = if seq.literals {
            ("literals", "Literal")
        } else if seq.arms {
            ("arms", "Arm")
        } else {
            ("fields", "Field")
        };
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
                "A `core.comptime` `{ty}`: `.name` (a literal of the names), `.kind: Kind`, \
                 `.type`, `.index: Int`, `.first: Bool`, `.last: Bool`. {}",
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
            CompCond::Kind { ty, span, .. } if ty.via_type => {
                if self.binder_kind(&ty.root.name).is_some() {
                    self.push(
                        ty.span,
                        &format!("{}.kind: Kind", ty.root.name),
                        "The kind of the binder's type — an arm of `core.comptime`'s `Kind`: \
                         `Struct | Union | Tuple | FnType | Opaque` [comptime-kind].",
                    );
                }
                let _ = span;
            }
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
            CompCond::NameIs { binder, span, .. } => {
                if self.binder_kind(&binder.name).is_some() {
                    self.push(
                        Span::new(span.start, binder.span.end + 5),
                        &format!("{}.name", binder.name),
                        "The declared name — a literal of the union of the struct's field \
                         names (or the union's arm names), tested at compile time.",
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
            CompCond::Annotated { subject, qual, span, .. } => {
                if self.binder_kind(&subject.name).is_some() {
                    self.push(
                        *span,
                        &format!("{} is {}", subject.name, qual.name.name),
                        "Whether the field carries this comptime qualifier, written before its name \
                         [comptime-qualifier].",
                    );
                }
            }
            CompCond::Unset { binder, docs, span } => {
                if self.binder_kind(&binder.name).is_some() {
                    let (proj, note) = if *docs {
                        ("docs", "Whether there is a doc comment: `is None` when there is not [comptime-docs].")
                    } else {
                        ("default", "Whether the field declares a default: `is None` when it does not [comptime-default].")
                    };
                    self.push(*span, &format!("{}.{proj}", binder.name), note);
                }
            }
            CompCond::NameEq { a, b, .. } => {
                for id in [a, b] {
                    if self.binder_kind(&id.name).is_some() {
                        self.push(
                            Span::new(id.span.start, id.span.end + 5),
                            &format!("{}.name", id.name),
                            "The declared name, compared with another binder's [comptime-name].",
                        );
                    }
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
            CompCond::And(a, b, _) | CompCond::Or(a, b, _) => {
                self.cond(a);
                self.cond(b);
            }
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
                    self.binder(binder, seq.arms || seq.literals);
                    self.binders.push((binder.name.clone(), seq.arms || seq.literals));
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
                CompStmt::WhenKind { ty, arms, span } => {
                    self.comp_ty(ty);
                    self.when_arms.push(CompWhenArms {
                        file: self.file,
                        span: *span,
                        missing: kind_arms_missing(arms),
                        types: false,
                        bodies: arms.iter().map(|a| a.body.span).collect(),
                    });
                    for arm in arms {
                        self.block(&arm.body);
                    }
                }
                CompStmt::WhenType { ty, arms, .. } => {
                    self.comp_ty(ty);
                    for arm in arms {
                        self.block(&arm.body);
                    }
                }
                CompStmt::WhenName { binder, arms, .. } => {
                    if self.binder_kind(&binder.name).is_some() {
                        self.push(
                            binder.span,
                            &format!("{}.name", binder.name),
                            "A dispatch over the declared name — exhaustive over the struct's \
                             field names (or the union's arm names).",
                        );
                    }
                    for arm in arms {
                        self.block(&arm.body);
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
                        &format!("The `{owner}`'s type, spliced here as the type itself (`[{b}.type]`)."),
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
                            "value" => ("Value", "The literal member itself, as a literal in each copy [comptime-literals]."),
                            "last" => ("Bool", "Whether this copy is for the last element."),
                            "type" => ("Type", "The declared type itself (`core.comptime`)."),
                            "kind" => ("Kind", "The declared type's kind (`core.comptime`), tested in a comptime head."),
                            "docs" => ("Str?", "The doc comment above the declaration, as text, or `None` [comptime-docs]."),
                            "default" => ("Value?", "The declared default, evaluated where the struct is declared; `None` where the field has none [comptime-default]."),
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
                    // `Reading.name`, `Reading.docs`: a declared type named as a root.
                    if (field.name == "name" || field.name == "docs") && root.name.starts_with(|c: char| c.is_uppercase()) {
                        self.roots.push(root.clone());
                        return;
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
            Expr::Unary { operand, .. } | Expr::IncDec { operand, .. } | Expr::Spread { operand, .. } => self.expr(operand),
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
                            self.binder(binder, seq.arms || seq.literals);
                            self.binders.push((binder.name.clone(), seq.arms || seq.literals));
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
            Expr::Pipe { lhs, rhs, .. } | Expr::SafePipe { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Try { body, .. } => self.block(body),
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(e) => self.expr(e),
                LambdaBody::Block(b) => self.block(b),
                LambdaBody::Terminal => {}
            },
            Expr::Tuple { elems, .. } => {
                for e in elems {
                    self.expr(e);
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

/// [when-exhaustive] [comptime-pattern] [comptime-name] A `[when]` with a
/// subject is exhaustive the way an ordinary `when` is, with no `else`: the
/// arms must cover the subject's range — so the range has to be one the
/// declaration can see. In a **stamped** comptime fn (`<T is Struct>`) a
/// field's (or arm's) type and name range over the fields of whatever `T` is
/// stamped at, which nothing knows where the fn is declared: such a dispatch
/// is refused there (user decision 2026-10-20). `[when f.kind]` stays — its
/// range is `std.core.comptime`'s `Kind`, the same everywhere — and since its
/// coverage is known from the arms alone, it is checked here too.
fn unchecked_subject_whens(f: &FnDecl) -> Vec<Diagnostic> {
    use salvo_syntax::visit::{self as v, Visitor};
    struct RangeCheck {
        bound: String,
        fn_name: String,
        /// The comptime names whose range comes from the bound parameter:
        /// binders of a walk over it (or over one of these), pattern binders
        /// of a test on one of these, and a union value's arm binder.
        derived: std::collections::HashSet<String>,
        errors: Vec<Diagnostic>,
    }
    impl RangeCheck {
        fn depends(&self, ty: &CompTy) -> bool {
            (!ty.via_type && ty.root.name == self.bound) || self.derived.contains(&ty.root.name)
        }
        fn cond(&mut self, c: &CompCond) {
            match c {
                CompCond::Is { ty, binders, .. } if self.depends(ty) => {
                    self.derived.extend(binders.iter().map(|b| b.name.clone()));
                }
                CompCond::Not(inner, _) => self.cond(inner),
                CompCond::And(a, b, _) | CompCond::Or(a, b, _) => {
                    self.cond(a);
                    self.cond(b);
                }
                _ => {}
            }
        }
    }
    impl Visitor for RangeCheck {
        fn visit_stmt(&mut self, s: &Stmt) {
            let Stmt::Comp(c) = s else { return };
            match c {
                CompStmt::For { binder, seq, .. } => {
                    if self.depends(&seq.ty) {
                        self.derived.insert(binder.name.clone());
                    }
                }
                CompStmt::If { cond, .. } => self.cond(cond),
                CompStmt::WhenArms { binder, .. } => {
                    self.derived.insert(binder.name.clone());
                }
                CompStmt::WhenKind { arms, span, .. } => {
                    self.errors.extend(kind_arms_incomplete(arms, *span));
                }
                CompStmt::WhenType { ty, arms, span, .. } if self.depends(ty) => {
                    for arm in arms {
                        self.derived.extend(arm.binders.iter().map(|b| b.name.clone()));
                    }
                    let subject = if ty.via_type { format!("{}.type", ty.root.name) } else { ty.root.name.clone() };
                    self.errors.push(Diagnostic::error(
                        format!(
                            "a `[when {subject}]` cannot be checked here: `{subject}` ranges over the                              types `{bound}` has, which are known only where `{f}` is stamped, and a                              `[when]` with a subject must cover its range — dispatch on `[when {root}.kind]`,                              or test with `[if {subject} is …]` [when-exhaustive] [comptime-pattern]",
                            bound = self.bound,
                            f = self.fn_name,
                            root = ty.root.name,
                        ),
                        *span,
                    ));
                }
                CompStmt::WhenName { binder, span, .. } if self.derived.contains(&binder.name) => {
                    self.errors.push(Diagnostic::error(
                        format!(
                            "a `[when {b}.name]` cannot be checked here: `{b}.name` ranges over the                              names `{bound}` has, which are known only where `{f}` is stamped, and a                              `[when]` with a subject must cover its range — test with `[if {b}.name is …]`                              [when-exhaustive] [comptime-name]",
                            b = binder.name,
                            bound = self.bound,
                            f = self.fn_name,
                        ),
                        *span,
                    ));
                }
                _ => {}
            }
        }
    }
    let Some((_, bound)) = f.compfn.as_ref().and_then(|c| c.bound.as_ref()) else {
        return Vec::new();
    };
    let mut check = RangeCheck {
        bound: bound.name.clone(),
        fn_name: f.name.name.clone(),
        derived: std::collections::HashSet::new(),
        errors: Vec::new(),
    };
    v::walk_fn(&mut check, f);
    check.errors
}

/// [lsp-completion] The kind words a `[when X.kind]`'s arms leave out, as
/// arms would name them: `Opaque` for both its halves.
fn kind_arms_missing(arms: &[KindArm]) -> Vec<String> {
    let covered = |k: TypeKindWord| arms.iter().any(|a| kind_matches(a.kind, k));
    let opaque = !covered(TypeKindWord::Basic) && !covered(TypeKindWord::Generic);
    [
        TypeKindWord::Struct,
        TypeKindWord::Union,
        TypeKindWord::Tuple,
        TypeKindWord::FnType,
        TypeKindWord::Basic,
        TypeKindWord::Generic,
    ]
    .into_iter()
    .filter(|k| !covered(*k))
    .filter_map(|k| match k {
        TypeKindWord::Basic if opaque => Some("Opaque".to_string()),
        TypeKindWord::Generic if opaque => None,
        k => Some(k.word().to_string()),
    })
    .collect()
}

/// [when-exhaustive] [comptime-kind] A `[when X.kind]` that leaves an arm of
/// `Kind` out — known from the arms alone, so a template is told where it is
/// declared rather than at every stamp.
fn kind_arms_incomplete(arms: &[KindArm], span: Span) -> Option<Diagnostic> {
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
    if missing.is_empty() {
        return None;
    }
    Some(Diagnostic::error(
        format!(
            "this `[when …]` over a kind does not consider every arm of `Kind`: missing {} — \
             add the arms (`is Opaque` covers `Basic` and `Generic`) [comptime-inline] \
             [when-exhaustive]",
            missing.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(", ")
        ),
        span,
    ))
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
    /// [comptime-hygiene] The template's file, recorded on every stamp.
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

/// [type-identity] What one file's names mean, as the resolver's scope for
/// that file says: the module declaring each visible struct, `type`, group
/// and qualifier. Owned, so the resolution that produced it can be dropped
/// before the modules are stamped into.
#[derive(Default)]
struct FileNames {
    structs: HashMap<String, ModulePath>,
    types: HashMap<String, ModulePath>,
    groups: HashMap<String, ModulePath>,
    quals: HashMap<String, ModulePath>,
    /// [obligation-by] The modules the file can see (`ModuleScope::modules`):
    /// what its `by` paths name.
    modules: Vec<ModulePath>,
}

/// [comptime-instantiate] [type-identity] Resolves the unstamped program, and
/// keeps of it what the expansion looks names up by: per file, which module's
/// declaration each name means. The expansion still runs before the
/// program's own resolution (PIPELINE.md step 6), so this is a second,
/// read-only one, over what the files declare before stamping. Only the
/// files from `start` on are stamped, so only their scopes are built
/// [lsp-std-front]; the files before keep an empty entry.
fn scoped_names<M: crate::decl_id::ModuleRef>(files: &[SourceFile], modules: &[M], start: usize) -> Vec<FileNames> {
    let mut module_of: HashMap<usize, ModulePath> = HashMap::new();
    for (file, module) in files.iter().zip(modules) {
        for item in &module.borrow().items {
            let addr = match item {
                Item::Struct(s) => s as *const StructDecl as usize,
                Item::Type(t) => t as *const TypeDecl as usize,
                Item::Params(g) => g as *const ParamsDecl as usize,
                Item::Qualifier(q) => q as *const QualifierDecl as usize,
                _ => continue,
            };
            module_of.insert(addr, file.module.clone());
        }
    }
    let t = crate::timing::phase("    comptime: resolve");
    let resolution = crate::resolve::resolve_scopes_where(files, modules, &|i| i >= start);
    drop(t);
    let _t = crate::timing::phase("    comptime: names");
    let of = |addr: usize| module_of.get(&addr).cloned();
    resolution
        .scopes
        .iter()
        .map(|s| FileNames {
            structs: s.structs.iter().filter_map(|(n, d)| Some((n.to_string(), of(*d as *const StructDecl as usize)?))).collect(),
            types: s
                .type_aliases
                .iter()
                .chain(s.opaque_types.iter())
                .filter_map(|(n, d)| Some((n.to_string(), of(*d as *const TypeDecl as usize)?)))
                .collect(),
            groups: s.param_groups.iter().filter_map(|(n, d)| Some((n.to_string(), of(*d as *const ParamsDecl as usize)?))).collect(),
            quals: s
                .qualifiers
                .iter()
                .filter_map(|(n, ds)| Some((n.to_string(), of(*ds.first()? as *const QualifierDecl as usize)?)))
                .collect(),
            modules: s.modules.iter().map(|m| (*m).clone()).collect(),
        })
        .collect()
}

struct World {
    /// [type-identity] Per file, what its names mean; `here` is the file
    /// being expanded.
    scoped: Vec<FileNames>,
    here: std::cell::Cell<usize>,
    /// [lsp-std-front] The tables of the files before the ones stamped (an
    /// already expanded std), which come first in every lookup, as their
    /// files come first in the program.
    base: Option<std::sync::Arc<Tables>>,
    tables: Tables,
    /// [obligation-by] Whether the file being expanded is std's: std sees
    /// only std, so a name its scope does not see falls back to std's
    /// declarations alone.
    std_here: std::cell::Cell<bool>,
    /// Every module a std file declares.
    std_modules: std::collections::HashSet<ModulePath>,
    /// [type-identity] The module being expanded: its own declaration of a
    /// name is the one meant.
    own: std::cell::RefCell<ModulePath>,
    /// [comptime-root-visible] The `import` paths of the file being
    /// expanded: of two same-named declarations, the one this file imports
    /// is the one it can mean (the checker then requires the name to be
    /// visible at all).
    imports: std::cell::RefCell<Vec<Vec<String>>>,
    module_paths: Vec<ModulePath>,
}

impl World {
    /// Every declaration of `name` in one of the tables, the base's first.
    fn entries<'s, D>(
        &'s self,
        name: &str,
        table: impl Fn(&'s Tables) -> &'s HashMap<String, Vec<(ModulePath, D)>>,
    ) -> Vec<&'s (ModulePath, D)> {
        let mut out = Vec::new();
        if let Some(base) = &self.base {
            out.extend(table(base).get(name).into_iter().flatten());
        }
        out.extend(table(&self.tables).get(name).into_iter().flatten());
        out
    }

    fn structs_of(&self, name: &str) -> Vec<&(ModulePath, StructDecl)> {
        self.entries(name, |t| &t.structs)
    }

    fn types_of(&self, name: &str) -> Vec<&(ModulePath, TypeDecl)> {
        self.entries(name, |t| &t.types)
    }

    /// Whether any file declares a struct or a `type` named `name` (only a
    /// std file's, when std is being expanded: std sees only std).
    fn declares_type(&self, name: &str) -> bool {
        let seen = |m: &ModulePath| !self.std_here.get() || self.std_modules.contains(m);
        self.structs_of(name).iter().any(|(m, _)| seen(m)) || self.types_of(name).iter().any(|(m, _)| seen(m))
    }

    /// Every comptime fn, the base's first.
    fn compfns(&self) -> impl Iterator<Item = &CompFnDecl> {
        self.base.iter().flat_map(|b| b.compfns.iter()).chain(self.tables.compfns.iter())
    }

    /// [obligation-by] Whether the file being expanded can see `module`: its
    /// own, an imported one, or one implicitly visible.
    fn sees_module(&self, module: &ModulePath) -> bool {
        self.names_here().is_some_and(|h| h.modules.contains(module))
    }

    /// [type-identity] [comptime-root-visible] Which of a name's
    /// declarations the file being expanded means: its own module's, else
    /// one from a module it imports (the module itself, or the item by
    /// name), else one from `std.core` (implicitly visible), else the first.
    /// [obligation-by] A std file considers std's declarations only.
    fn pick_decl<'a, D>(&self, name: &str, all: &[&'a (ModulePath, D)]) -> Option<&'a (ModulePath, D)> {
        let std_only: Vec<&'a (ModulePath, D)>;
        let all = if self.std_here.get() {
            std_only = all.iter().copied().filter(|(m, _)| self.std_modules.contains(m)).collect();
            &std_only[..]
        } else {
            all
        };
        let own = self.own.borrow();
        if let Some(d) = all.iter().find(|(m, _)| *m == *own) {
            return Some(*d);
        }
        let imports = self.imports.borrow();
        let imported = |m: &ModulePath| {
            imports.iter().any(|path| {
                path.as_slice() == m.0.as_slice()
                    || (path.len() == m.0.len() + 1
                        && path[..m.0.len()] == m.0[..]
                        && path.last().is_some_and(|l| l == name))
            })
        };
        all.iter()
            .find(|(m, _)| imported(m))
            .or_else(|| all.iter().find(|(m, _)| m.is_std_core() && self.std_modules.contains(m)))
            .or_else(|| all.first())
            .copied()
    }

    /// [struct-opaque] Whether `name` is an opaque struct whose fields the
    /// module being expanded may not see: declared elsewhere, and this is not
    /// its `*.test.sv` annex (module `m.test`).
    fn opaque_here(&self, name: &str) -> bool {
        let all = self.structs_of(name);
        if all.is_empty() {
            return false;
        }
        let own = self.own.borrow().clone();
        let here = self.names_here();
        let other = here.is_some_and(|h| h.types.contains_key(name));
        let visible = here.and_then(|h| h.structs.get(name)).filter(|_| !other);
        let Some((module, decl)) = self.scoped_decl(name, &all, visible, other) else {
            return false;
        };
        if !decl.opaque || *module == own {
            return false;
        }
        let annex = own.0.len() == module.0.len() + 1
            && own.0.last().is_some_and(|l| l == "test")
            && own.0[..module.0.len()] == module.0[..];
        !annex
    }

    /// [type-identity] The names of the file being expanded.
    fn names_here(&self) -> Option<&FileNames> {
        self.scoped.get(self.here.get())
    }

    /// A name's declaration as the file being expanded resolves it — through
    /// its scope (imports, renames, rungs) — and only when the scope does not
    /// see the name at all, the old ladder (a template's own names, which the
    /// stamping site may not see [comptime-hygiene]).
    fn scoped_decl<'a, D>(
        &self,
        name: &str,
        all: &[&'a (ModulePath, D)],
        visible: Option<&ModulePath>,
        other_kind: bool,
    ) -> Option<&'a (ModulePath, D)> {
        match visible {
            Some(m) => all.iter().find(|(dm, _)| dm == m).copied(),
            // The name is visible here as the other kind of type: this
            // kind's declaration elsewhere is not what it means.
            None if other_kind => None,
            None => self.pick_decl(name, all),
        }
    }

    /// A `type` the file sees outranks a struct of the same name, as in the
    /// checker, which expands an alias before it reads a nominal.
    fn struct_named(&self, name: &str) -> Option<&StructDecl> {
        let all = self.structs_of(name);
        let here = self.names_here();
        let other = here.is_some_and(|h| h.types.contains_key(name));
        let visible = here.and_then(|h| h.structs.get(name)).filter(|_| !other);
        self.scoped_decl(name, &all, visible, other).map(|(_, d)| d)
    }

    fn type_named(&self, name: &str) -> Option<&TypeDecl> {
        let all = self.types_of(name);
        let here = self.names_here();
        let visible = here.and_then(|h| h.types.get(name));
        let other = visible.is_none() && here.is_some_and(|h| h.structs.contains_key(name));
        self.scoped_decl(name, &all, visible, other).map(|(_, d)| d)
    }

    /// [type-identity] The `params` group a clause names, in its file.
    fn group_named(&self, name: &str) -> Option<&ParamsDecl> {
        let all = self.entries(name, |t| &t.groups);
        let visible = self.names_here().and_then(|h| h.groups.get(name));
        self.scoped_decl(name, &all, visible, false).map(|(_, d)| d)
    }

    /// [comptime-qualifier] The comptime qualifier `name` means here.
    fn comptime_qual(&self, name: &str) -> Option<&QualifierDecl> {
        self.qual_named(name).filter(|q| q.comptime)
    }

    /// The qualifier `name` means here, comptime or not.
    fn qual_named(&self, name: &str) -> Option<&QualifierDecl> {
        let all = self.entries(name, |t| &t.quals);
        let visible = self.names_here().and_then(|h| h.quals.get(name));
        self.scoped_decl(name, &all, visible, false).map(|(_, d)| d)
    }

    /// [comptime-docs] The doc comment of the declaration a type names — a
    /// struct, or a `type` declaration — as text.
    fn docs_of(&self, ty: &Type) -> Option<String> {
        let Type::Named { base, .. } = ty else { return None };
        let name = base.name.name.as_str();
        match self.struct_named(name) {
            Some(s) => doc_text(&s.docs),
            None => self.type_named(name).and_then(|t| doc_text(&t.docs)),
        }
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

    /// [obligation-by] The target a clause is written on: the declaration in
    /// hand (its own module's, so never opaque to it).
    fn target_of_item(item: &Item) -> Option<Target> {
        match item {
            Item::Struct(s) => Some(Target {
                name: s.name.name.clone(),
                kind: CompKind::Struct,
                ty: Self::simple_named(&s.name.name, s.name.span),
                generics: s.generics.iter().map(|g| g.name.clone()).collect(),
                canbe: s.auto_qualifiers.iter().map(|q| q.name.name.clone()).collect(),
            }),
            Item::Type(t) => {
                let is_union = t.alias.as_ref().is_some_and(|a| matches!(a, Type::Union { .. } | Type::Nullable { .. }));
                is_union.then(|| Target {
                    name: t.name.name.clone(),
                    kind: CompKind::Union,
                    ty: Self::simple_named(&t.name.name, t.name.span),
                    generics: t.generics.iter().map(|g| g.name.clone()).collect(),
                    canbe: t.auto_qualifiers.iter().map(|q| q.name.name.clone()).collect(),
                })
            }
            _ => None,
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
    fn compfn_of(&self, decl: &FnDecl) -> Option<&CompFnDecl> {
        self.compfns()
            .find(|c| std::ptr::eq(&c.decl, decl) || (c.decl.name.span == decl.name.span && c.decl.name.name == decl.name.name))
    }

    /// The picked template, its module, and [comptime-hygiene] its file.
    fn pick_with_module(
        &self,
        member: &str,
        by: &ByRef,
        kind: CompKind,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<(FnDecl, ModulePath, Option<usize>)> {
        let decl = self.pick_inner(member, by, kind, diags)?;
        let from = self.compfn_of(&decl);
        let module = from.map(|c| c.module.clone()).unwrap_or(ModulePath(vec![]));
        let file = from.map(|c| c.file);
        Some((decl, module, file))
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
        // so `by auto` reaches `std.core.auto` as `size@list` reaches `std.core.list`
        // — among the modules the stamping file can see [obligation-by]: its
        // own, what it imports, what is implicitly visible. A project module
        // `auto` is no second candidate for std's `by auto`, nor for a file
        // that does not import it.
        let modules: Vec<&ModulePath> = self
            .module_paths
            .iter()
            .filter(|m| m.matches_suffix(&path) && self.sees_module(m))
            .collect();
        let mut seen = std::collections::HashSet::new();
        let modules: Vec<&ModulePath> = modules.into_iter().filter(|m| seen.insert(m.to_string())).collect();
        // The function reading: a comptime fn of that name, of the target's
        // kind, in a module the file can see — or in the module the `@`
        // selector names.
        let fn_candidates: Vec<&CompFnDecl> = if path.len() == 1 {
            self.compfns()
                .filter(|c| c.decl.name.name == path[0] && self.sees_module(&c.module))
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
                    format!("`by @{}` names no module this file can see{} [obligation-by]", by.text(), self.unseen(&path)),
                    by.span,
                ));
                return None;
            };
            let mut candidates: Vec<&FnDecl> = self
                .compfns()
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
                "`by {}` names neither a module nor a `comptime fn` this file can see — \
                 nothing to stamp `{member}` from{} [obligation-by] [fn-by]",
                by.text(),
                self.unseen(&path)
            ),
            by.span,
        ));
        None
    }

    /// [obligation-by] For a `by` path that names nothing the file can see:
    /// the imports that would make it name something, as the end of the
    /// message (empty when there are none).
    fn unseen(&self, path: &[&str]) -> String {
        let mut found: Vec<String> = self
            .module_paths
            .iter()
            .filter(|m| m.matches_suffix(path) && !self.sees_module(m))
            .map(|m| m.to_string())
            .collect();
        if let [only] = path {
            found.extend(
                self.compfns()
                    .filter(|c| c.decl.name.name == *only && !self.sees_module(&c.module))
                    .map(|c| c.module.to_string()),
            );
        }
        found.sort();
        found.dedup();
        if found.is_empty() {
            return String::new();
        }
        let imports: Vec<String> = found.iter().map(|m| format!("`import {m}`")).collect();
        format!("; `{}` is not imported here: add {}", found.join("`, `"), imports.join(" or "))
    }

    fn classify(&self, ty: &Type, generics: &[String], depth: u32) -> TypeKindWord {
        match ty {
            Type::Literal { .. } => TypeKindWord::Basic,
            Type::Union { .. } | Type::Nullable { .. } => TypeKindWord::Union,
            Type::Spread { .. } => TypeKindWord::Union,
            Type::Tuple { .. } => TypeKindWord::Tuple,
            Type::Fn { .. } => TypeKindWord::FnType,
            Type::Array { .. } => TypeKindWord::Basic,
            Type::QualifiedGroup { base, .. } => self.classify(base, generics, depth),
            Type::Named { base, .. } => {
                let name = base.name.name.as_str();
                if generics.iter().any(|g| g == name) {
                    return TypeKindWord::Generic;
                }
                if self.struct_named(name).is_some() {
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
            Type::Spread { inner, .. } => format!("...{}", self.normalized(inner)),
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
    /// [origin] The file's next expansion id.
    next_expn: &'w mut u32,
}

impl<'w> Stamper<'w> {
    fn new(
        world: &'w World,
        target: &'w Target,
        template: &'w FnDecl,
        by: &'w ByRef,
        next_expn: &'w mut u32,
    ) -> Self {
        Stamper {
            world,
            target,
            template,
            by,
            next_expn,
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

    /// [comptime-generic] What stamping at a **generic** target needs (ROADMAP
    /// §0j 6i, decided with §2c): the target's type parameters, their `canbe`
    /// opt-ins, and one implicit group per parameter a field (or arm) mentions
    /// — `?Ordered<T>` for `cmp`, `?Eq<T>` for `eq`, `?Hashed<T>` for `hash`,
    /// `?ToStr<T>` for `to_str`. A copy meeting an opaque `T` has nothing to
    /// call but what the caller hands in, so the need is the signature's —
    /// and so it follows the **member** being declared, not the template's
    /// name. `None` for a member whose capability has no group to ask for.
    fn generic_needs(&self, member: &str, site: Span) -> Option<(Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<TypeRef>)> {
        let group = match member {
            "cmp" => "Ordered",
            "eq" => "Eq",
            "hash" => "Hashed",
            "to_str" => "ToStr",
            _ => return None,
        };
        let (generics, canbe, parts): (Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<Type>) =
            if let Some(s) = self.world.struct_named(&self.target.name) {
                (s.generics.clone(), s.generic_canbe.clone(), s.fields.iter().filter_map(|f| f.ty.clone()).collect())
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

    /// [obligation-by] [fn-by] The declaration a clause stamps for one member
    /// of its group: exactly the `fn … by X` it stands for (user decision
    /// 2026-10-14). Its signature is the **member's**, with the group's
    /// parameter — `self` at the clause — substituted by the target, and for
    /// a generic target the additions [comptime-generic] makes: the target's
    /// type parameters, their `canbe` opt-ins and the implicit groups its
    /// fields need. No body: [`Stamper::stamp_into`] supplies it, as for a
    /// written declaration. The member's nodes are made by the stamp [origin]
    /// (`ExpansionKind::Made`), so a diagnostic about the signature lands at
    /// the `by` site.
    fn member_decl(self, group: &ParamsDecl, member: &FnDecl) -> Result<FnDecl, Vec<Diagnostic>> {
        let site = self.by.span;
        let generic = if self.target.generics.is_empty() {
            None
        } else {
            match self.generic_needs(&member.name.name, site) {
                Some(needs) => Some(needs),
                None => return Err(vec![self.generic_refused(site).expect("generic target")]),
            }
        };
        let mut f = member.clone();
        let mut made = Retag::fresh(self.next_expn);
        let mut expansions = vec![ExpansionRecord { id: made.expn, parent: 0, kind: ExpansionKind::Made }];
        visit_mut::walk_fn(&mut made, &mut f);
        // `self`: the group's parameters stand for the target (at a generic
        // one, applied to its own parameters). Each is renamed apart first,
        // so a target parameter of the same name (`Ordered<T>` at
        // `Wrapper<T>`) is never substituted again.
        let mut target = self.target.ty.clone();
        if generic.is_some() {
            if let Type::Named { base, .. } = &mut target {
                base.args = self.target.generics.iter().map(|g| World::simple_named(g, site)).collect();
            }
        }
        let mut names = Vec::new();
        for (i, g) in group.generics.iter().enumerate() {
            let fresh = format!("__Group{i}");
            let mut rename = RenameIdent { from: g.name.clone(), to: fresh.clone() };
            visit_mut::walk_fn(&mut rename, &mut f);
            names.push(fresh);
        }
        let mut subst = SubstSelf { names: &names, target: &target };
        visit_mut::walk_fn(&mut subst, &mut f);
        // [comptime-generic] One `by` site stamps several members (`Hashed`'s
        // `hash` and `eq`), and a generic stamp's implicits are looked up by
        // its name's span: each gets an expansion of its own [origin], whose
        // diagnostics land at the site.
        let name_span = if generic.is_some() {
            let named = Retag::fresh(self.next_expn);
            expansions.push(ExpansionRecord {
                id: named.expn,
                parent: 0,
                kind: ExpansionKind::Member {
                    label: format!("`{}` stamped at `{}`", self.template.name.name, self.target.name),
                },
            });
            member.name.span.in_expansion(named.expn)
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
        f.body = None;
        f.by = Some(self.by.clone());
        f.docs = vec![format!(
            "The `{}` of [{}], stamped from `{}`'s `compfn` [obligation-by].",
            f.name.name, self.target.name, self.by.text()
        )];
        // Carries the name's region to the stamp `stamp_into` records.
        f.stamped = Some(Stamp {
            compfn: self.template.name.name.clone(),
            from: self.by.text(),
            at_type: self.target.name.clone(),
            site,
            expansions,
            roots: Vec::new(),
            template_file: None,
        });
        f.span = site;
        f.name.span = name_span;
        Ok(f)
    }

    /// [fn-by] [obligation-by] Stamps the body of a declaration — a written
    /// `fn … by X`, or the one a clause declares per member
    /// ([`Stamper::member_decl`]). **The declaration's signature is the
    /// stamped fn's** (user decision 2026-10-14): its parameter names, types,
    /// return type, effect list and deduction clause (written or defaulted
    /// [deduce-default]). The template supplies only the body: it is
    /// instantiated at the target, its parameter types and return type are
    /// checked against the declaration's, positionally (a clearer error than
    /// the body's type errors would be), and its parameters are bound to the
    /// declaration's by position — the body's references to the template's
    /// names are renamed to the declared ones. Its own effect list and
    /// deductions do not count: the checker holds the body to the declared
    /// contract, as it holds any body to its signature.
    fn stamp_into(self, f: &mut FnDecl, declared: Declared<'_>) -> Result<(), Vec<Diagnostic>> {
        let site = f.by.as_ref().map(|b| b.span).unwrap_or(f.span);
        let mut inst = self.template.clone();
        // [origin] The template copied whole: every node keeps its span in
        // the template, under this stamp's expansion.
        let mut whole = Retag::fresh(self.next_expn);
        let template_expn = whole.expn;
        visit_mut::walk_fn(&mut whole, &mut inst);
        let mut param = self.template.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(_, id)| id.name.clone());
        // [comptime-generic] At a generic target (a clause's: a written
        // declaration there is refused) the template's own parameter is
        // renamed out of the way of the target's (`cmp<T is Struct>` stamped
        // at `Wrapper<T>`): the substitution `T` → `Wrapper<T>` would
        // otherwise meet its own `T`.
        let mut target = self.target.clone();
        if !self.target.generics.is_empty() {
            if let Some(p) = &param {
                if self.target.generics.contains(p) {
                    let fresh = "__Self".to_string();
                    let mut rename = RenameIdent { from: p.clone(), to: fresh.clone() };
                    visit_mut::walk_fn(&mut rename, &mut inst);
                    param = Some(fresh);
                }
            }
            if let Type::Named { base, .. } = &mut target.ty {
                base.args = self.target.generics.iter().map(|g| World::simple_named(g, site)).collect();
            }
        }
        let mut ex = Expander::new(self.world, param.map(|p| (p, target)), site, self.next_expn);
        ex.current = template_expn;
        let mut sig = SigSubst { ex: &mut ex };
        for p in &mut inst.params {
            visit_mut::walk_type(&mut sig, &mut p.ty);
        }
        if let Some(r) = &mut inst.return_type {
            visit_mut::walk_type(&mut sig, r);
        }
        // The declared signature has to be the instantiation's, positionally.
        // A written one is reported where it is written; a clause's member at
        // the clause.
        let (what, written) = match declared {
            Declared::Written => (format!("`fn {}`", f.name.name), true),
            Declared::Member { group } => (format!("`{group}<self>`'s `fn {}`", f.name.name), false),
        };
        let at = |s: Span| if written { s } else { site };
        let mut errs = Vec::new();
        if inst.params.len() != f.params.len() {
            errs.push(Diagnostic::error(
                format!(
                    "{what} takes {} parameter(s), but `{}`'s `compfn {}` stamped at `{}` \
                     takes {} [fn-by]",
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
                            "{what}'s parameter `{}` is `{}`, but the stamped `{}` takes `{}` \
                             there [fn-by]",
                            w.name.name,
                            w.ty,
                            self.template.name.name,
                            i.ty
                        ),
                        at(w.ty.span()),
                    ));
                }
            }
        }
        let written_ret = f.return_type.as_ref().map(|t| self.world.normalized(t)).unwrap_or_else(|| "None".into());
        let inst_ret = inst.return_type.as_ref().map(|t| self.world.normalized(t)).unwrap_or_else(|| "None".into());
        if written_ret != inst_ret {
            errs.push(Diagnostic::error(
                format!(
                    "{what} returns `{}`, but the stamped `{}` returns `{}` [fn-by]",
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
        if let Some(b) = &mut inst.body {
            ex.expand_block(b);
        }
        let (expansions, mut errs) = ex.finish();
        // A stamp reports errors at its `by` site; a template's warnings are
        // its author's business, not every stamping site's.
        errs.retain(|d| d.is_error());
        if !errs.is_empty() {
            return Err(errs);
        }
        // The template's parameters are the declaration's, by position: the
        // expanded body's references to the template's names are renamed to
        // the declared ones (after the expansion, so no comptime binder can
        // meet a declared name), and a local of the body that a declared name
        // would capture is renamed apart first.
        let renames: HashMap<String, String> = inst
            .params
            .iter()
            .zip(&f.params)
            .filter(|(i, w)| i.name.name != w.name.name)
            .map(|(i, w)| (i.name.name.clone(), w.name.name.clone()))
            .collect();
        if let Some(b) = &mut inst.body {
            if !renames.is_empty() {
                rename_capturing_locals(b, &renames);
                // [fn-by] A declared name the body also uses for something
                // outside it — a function it calls, or names as a value —
                // would capture that use once the parameters are renamed:
                // the declared names are the contract (named arguments use
                // them), and a local shadows a name [call-resolve], so the
                // declaration has to change. One error at the parameter, and
                // nothing stamped.
                if let Some((name, called)) = hidden_free_name(b, &renames) {
                    let used = if called {
                        format!("the function `{name}` that `{}`'s `{}` calls in its body", self.by.text(), self.template.name.name)
                    } else {
                        format!("the name `{name}` that `{}`'s `{}` uses in its body", self.by.text(), self.template.name.name)
                    };
                    let message = match declared {
                        Declared::Written => {
                            format!("parameter `{name}` hides {used}: name the parameter differently [fn-by]")
                        }
                        Declared::Member { group } => format!(
                            "`{group}<self>`'s `fn {}` names its parameter `{name}`, which hides {used}: \
                             name the group member's parameter differently [fn-by]",
                            f.name.name
                        ),
                    };
                    let span = f.params.iter().find(|p| p.name.name == name).map(|p| at(p.name.span)).unwrap_or(site);
                    return Err(vec![Diagnostic::error(message, span)]);
                }
                let mut r = RenameParams { renames: &renames };
                visit_mut::walk_block(&mut r, b);
            }
        }
        f.body = inst.body;
        f.by = None;
        let mut all = f.stamped.take().map(|s| s.expansions).unwrap_or_default();
        all.push(ExpansionRecord { id: template_expn, parent: 0, kind: ExpansionKind::Template });
        all.extend(expansions);
        all.sort_by_key(|e| e.id);
        f.stamped = Some(Stamp {
            compfn: self.template.name.name.clone(),
            from: self.by.text(),
            at_type: self.target.name.clone(),
            site,
            expansions: all,
            roots: Vec::new(),
            template_file: None,
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
/// declaration's).
struct RenameParams<'a> {
    renames: &'a HashMap<String, String>,
}

/// [fn-by] [obligation-by] Which declaration a stamp fills: one written `fn
/// … by X`, or the one an obligation clause declares for a member of its
/// group. They stamp alike; only where a signature mismatch is reported, and
/// how it is named, differ.
#[derive(Clone, Copy)]
enum Declared<'a> {
    Written,
    Member { group: &'a str },
}

/// [obligation-by] `self` in a group member's signature: each of the group's
/// (renamed-apart) parameters stands for the target type, its qualifiers
/// kept (`Mut T` is `Mut Point`).
struct SubstSelf<'a> {
    names: &'a [String],
    target: &'a Type,
}

impl MutVisitor for SubstSelf<'_> {
    fn visit_type(&mut self, ty: &mut Type) {
        let Type::Named { qualifiers, base } = ty else { return };
        if !base.args.is_empty() || !self.names.contains(&base.name.name) {
            return;
        }
        let mut new = self.target.clone();
        if !qualifiers.is_empty() {
            match &mut new {
                Type::Named { qualifiers: q2, .. } => {
                    let mut all = qualifiers.clone();
                    all.append(q2);
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
}

/// [fn-by] Before a template's parameters are renamed to the declared names,
/// every local the body binds under one of those names — a `let`, an `is`
/// binding, a `for` pattern, a lambda parameter — is renamed apart
/// (`h` → `h__p`, written `h` in diagnostics [comptime-scope]), so a declared
/// `fn hash(h: P) -> Long by auto` reads its parameter where the template read
/// `value`, and its own `h` where it declared one.
fn rename_capturing_locals(body: &mut Block, renames: &HashMap<String, String>) {
    struct Lambdas<'a>(&'a mut Vec<String>);
    impl MutVisitor for Lambdas<'_> {
        fn visit_expr(&mut self, expr: &mut Expr) {
            if let Expr::Lambda { params, .. } = expr {
                self.0.extend(params.iter().map(|p| p.name.name.clone()));
            }
        }
    }
    let mut names: Vec<String> = Vec::new();
    visit_mut::walk_block(&mut CollectLocals { names: &mut names }, body);
    visit_mut::walk_block(&mut Lambdas(&mut names), body);
    let declared: Vec<&String> = renames.values().collect();
    let apart: HashMap<String, String> = names
        .into_iter()
        .filter(|n| declared.contains(&n) && !renames.contains_key(n))
        .map(|n| (n.clone(), format!("{n}{APART_SUFFIX}")))
        .collect();
    if !apart.is_empty() {
        visit_mut::walk_block(&mut RenameLocals { renames: &apart }, body);
    }
}

/// [fn-by] The first name a stamped body uses that one of the declared
/// parameter names would capture: a bare identifier spelled like a declared
/// name that is no template parameter (those are renamed) and no local (those
/// were renamed apart, [`rename_capturing_locals`]) — so a function the body
/// calls, or a name it reads from outside. Whether it is a call's callee
/// comes with it.
fn hidden_free_name(body: &mut Block, renames: &HashMap<String, String>) -> Option<(String, bool)> {
    struct Find<'a> {
        renames: &'a HashMap<String, String>,
        callees: Vec<Span>,
        found: Option<(String, bool)>,
    }
    impl MutVisitor for Find<'_> {
        fn visit_expr(&mut self, expr: &mut Expr) {
            if self.found.is_some() {
                return;
            }
            match expr {
                Expr::Call { callee, .. } => {
                    if let Expr::Ident(id) = &**callee {
                        self.callees.push(id.span);
                    }
                }
                Expr::Ident(id) => {
                    if !self.renames.contains_key(&id.name) && self.renames.values().any(|n| *n == id.name) {
                        self.found = Some((id.name.clone(), self.callees.contains(&id.span)));
                    }
                }
                _ => {}
            }
        }
    }
    let mut find = Find { renames, callees: Vec::new(), found: None };
    visit_mut::walk_block(&mut find, body);
    find.found
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

/// [origin] Tags every span of a tree with a fresh expansion id: the nodes
/// keep the ranges of the code they were copied from, and the id tells this
/// copy apart from every other.
struct Retag {
    expn: u32,
}

impl Retag {
    fn fresh(next_expn: &mut u32) -> Retag {
        let expn = *next_expn;
        *next_expn += 1;
        Retag { expn }
    }
}

impl MutVisitor for Retag {
    fn visit_span(&mut self, span: &mut Span) {
        span.expn = self.expn;
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
    /// [comptime-name] Every name of the sequence this element is in — the
    /// literal union `binder.name` is a member of — and how to name the
    /// sequence in a diagnostic ("`P` has no field", "`Source` has no arm").
    siblings: Vec<String>,
    sibling_owner: String,
    /// [comptime-default] The struct a field binder walks (with an instance's
    /// type arguments), when the field declares a default; `None` for a field
    /// without one, and for an arm.
    default_of: Option<Type>,
    /// [comptime-docs] The element's doc comment as text, or `None`: a
    /// field's own, an arm's the declaration it names.
    docs: Option<String>,
    /// [comptime-qualifier] The comptime qualifiers written before a field's
    /// name; an arm has none.
    annotations: Vec<TypeRef>,
    /// [comptime-literals] The member a `[for lit in X.literals]` copy is
    /// for; `None` for a field or an arm.
    literal: Option<salvo_syntax::ast::TypeLit>,
}

/// [comptime-docs] A doc block as a value: its prose lines joined, as hover
/// joins them, with `[symbol]` references left as written; `None` when
/// blank. The tags are not part of it [doc-tags].
fn doc_text(docs: &[String]) -> Option<String> {
    // [doc-tags] The prose only: a tag is for the reader of the declaration
    // (hover), not text a derive shows.
    let prose = salvo_syntax::doc::split(docs).prose;
    if prose.iter().all(|l| l.trim().is_empty()) {
        return None;
    }
    Some(prose.join("\n").trim_start_matches('\n').trim_end().to_string())
}

/// The body rewriter: unrolls, selects, refuses, and substitutes.
struct Expander<'w> {
    world: &'w World,
    /// The bound parameter and what it stands for, or `None` in a concrete
    /// compfn.
    bound: Option<(String, Target)>,
    env: Vec<(String, Binding)>,
    /// The fn's parameters, for an expanded-in-place compfn's `[when …]` over one.
    params: Vec<Param>,
    site: Span,
    /// [origin] The file's next expansion id.
    next_expn: &'w mut u32,
    /// [origin] The expansions made: the unrolled copies, and the nodes made
    /// up, in the order made (ascending ids).
    expansions: Vec<ExpansionRecord>,
    /// [origin] The expansion the code being expanded is in: the template's
    /// whole copy in a stamp, 0 (written) in an expanded-in-place fn, and the
    /// copy itself while a copy is expanded — the parent of a copy made now.
    current: u32,
    errors: Vec<Diagnostic>,
    rename_counter: u32,
    /// [comptime-inline] Whether this is an **expanded-in-place** fn, whose
    /// copies are of its own text: a diagnostic about a written construct
    /// lands where it is written (after following the copies back to it)
    /// rather than on the declaration. Not in a stamp, whose template lives
    /// in another declaration — there the `by` site is where the author can
    /// act.
    in_place: bool,
    /// The expanded-in-place fn's name, which a `refuse!` names in place of
    /// a stamped type [comptime-refuse].
    fn_name: Option<String>,
    /// [comptime-generic-fn] An expanded-in-place fn's own type parameters:
    /// opaque inside it (kind `Generic`), and names a type test may use.
    fn_generics: Vec<String>,
    /// [comptime-pattern] The pattern binders in scope (innermost last):
    /// name, the type it bound, its declaration's span, and how often this
    /// scope used it.
    type_binders: Vec<(String, Type, Span, usize)>,
    /// [comptime-qualifier] The constant binders in scope (`c` of `[if field
    /// is <c> Short(c)]`): name, the constant, its declaration's span, and
    /// how often this scope used it — the value twin of `type_binders`.
    value_binders: Vec<(String, crate::types::Const, Span, usize)>,
    /// [comptime-qualifier] What the condition being evaluated bound as
    /// constants, consumed by the `[if]` that keeps its branch.
    value_binds: Vec<(Ident, crate::types::Const)>,
    /// [comptime-pattern] Per binder declaration (keyed by its written span
    /// and name): whether *any* copy used it — an unused binder warns once,
    /// and only if no copy used it.
    binder_used: HashMap<((u32, u32), String), (Span, bool)>,
    /// [lsp-completion] Per `[when f.name]` / `[when f.type]` copy, at its
    /// written span: what that copy found uncovered (expanded-in-place fns
    /// only, where the span is the author's own).
    when_arms: Vec<(Span, Vec<String>, bool, Vec<Span>)>,
}

impl<'w> Expander<'w> {
    fn new(world: &'w World, bound: Option<(String, Target)>, site: Span, next_expn: &'w mut u32) -> Self {
        Expander {
            world,
            bound,
            env: Vec::new(),
            params: Vec::new(),
            site,
            next_expn,
            expansions: Vec::new(),
            current: 0,
            errors: Vec::new(),
            rename_counter: 0,
            in_place: false,
            fn_name: None,
            fn_generics: Vec::new(),
            type_binders: Vec::new(),
            value_binders: Vec::new(),
            value_binds: Vec::new(),
            binder_used: HashMap::new(),
            when_arms: Vec::new(),
        }
    }

    /// [lsp-completion] Records a `[when]` copy's uncovered arms at the
    /// construct's written span — an expanded-in-place fn's own text only.
    fn record_when_arms(&mut self, span: Span, missing: Vec<String>, types: bool, bodies: Vec<Span>) {
        if self.in_place {
            let written = self.unmap(span);
            if written.is_written() && written.end > written.start {
                let bodies = bodies.into_iter().map(|b| self.unmap(b)).collect();
                self.when_arms.push((written, missing, types, bodies));
            }
        }
    }

    fn finish(mut self) -> (Vec<ExpansionRecord>, Vec<Diagnostic>) {
        // One mistake, one diagnostic: a condition in an unrolled body is
        // evaluated once per copy, and so is reported once per copy.
        let mut errors: Vec<Diagnostic> = Vec::new();
        for e in self.errors {
            if !errors.iter().any(|d| d.message == e.message && d.span == e.span) {
                errors.push(e);
            }
        }
        // [comptime-pattern] A binder no copy used: `*` says the same thing.
        let mut unused: Vec<(Span, String)> = self
            .binder_used
            .iter()
            .filter(|(_, (_, used))| !*used)
            .map(|((_, name), (span, _))| (*span, name.clone()))
            .collect();
        unused.sort_by_key(|(s, _)| s.order_key());
        for (span, name) in unused {
            let at = if self.in_place && span.is_written() { span } else { self.site };
            errors.push(Diagnostic::warning(
                format!("pattern binder `{name}` is never used: write `*` in its place [comptime-pattern]"),
                at,
            ));
        }
        self.expansions.sort_by_key(|e| e.id);
        (self.expansions, errors)
    }

    /// The span a span in a copy was copied from [origin]: copies nest, so
    /// follow each copy back to the code it copied — the fn's own text in an
    /// expanded-in-place fn, the template's whole copy in a stamp. What the
    /// expansion made up is no copy, and stays.
    fn unmap(&self, mut span: Span) -> Span {
        let mut guard = 0;
        while let Some(e) = self
            .expansions
            .iter()
            .rev()
            .find(|e| e.id == span.expn && matches!(e.kind, ExpansionKind::Copy { .. }))
        {
            span = span.in_expansion(e.parent);
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        span
    }

    /// Where a diagnostic about the construct at `span` lands.
    fn report_span(&self, span: Span) -> Span {
        if self.in_place {
            let written = self.unmap(span);
            if written.is_written() && written.end > written.start {
                written
            } else {
                self.site
            }
        } else {
            self.site
        }
    }

    fn error(&mut self, span: Span, msg: impl Into<String>) {
        // A diagnostic here is about the template or the site: in an
        // expanded-in-place fn the construct is the author's own text, so it
        // lands there; in a stamp, the site is where the author can act.
        let at = self.report_span(span);
        self.errors.push(Diagnostic::error(msg, at));
    }

    fn binding(&self, name: &str) -> Option<&Binding> {
        self.env.iter().rev().find(|(n, _)| n == name).map(|(_, b)| b)
    }

    fn generics(&self) -> Vec<String> {
        let mut out = self.bound.as_ref().map(|(_, t)| t.generics.clone()).unwrap_or_default();
        out.extend(self.fn_generics.iter().cloned());
        out
    }

    /// The type a `CompTy` denotes here.
    fn resolve_ty(&mut self, ct: &CompTy) -> Option<Type> {
        if ct.via_type {
            return match self.binding(&ct.root.name) {
                Some(b) => Some(b.ty.clone()),
                None => {
                    self.error(ct.span, format!("`{}` is not a binder of an enclosing `[for …]` or `[when …]` [comptime-fields]", ct.root.name));
                    None
                }
            };
        }
        if let Some((param, target)) = &self.bound {
            if *param == ct.root.name {
                return Some(target.ty.clone());
            }
        }
        // [comptime-pattern] A binder is the type it bound (`E.fields`).
        if ct.args.is_empty() {
            if let Some(i) = self.type_binders.iter().rposition(|(n, ..)| *n == ct.root.name) {
                self.type_binders[i].3 += 1;
                return Some(self.type_binders[i].1.clone());
            }
        }
        if self.world.declares_type(&ct.root.name) {
            let mut t = World::simple_named(&ct.root.name, ct.span);
            if let Type::Named { base, .. } = &mut t {
                base.args = ct.args.clone();
            }
            return Some(t);
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
        // [comptime-literals] A literal union's members, in declaration order:
        // the literals the runtime collapses into one arm [type-literal].
        if seq.literals {
            let arms = match &ty {
                Type::Literal { .. } => vec![ty.clone()],
                _ => self.arms_of_type(&ty),
            };
            let lits: Vec<(Type, salvo_syntax::ast::TypeLit)> = arms
                .into_iter()
                .filter_map(|a| match &a {
                    Type::Literal { value, .. } => Some((a.clone(), value.clone())),
                    Type::QualifiedGroup { base, .. } => match &**base {
                        Type::Literal { value, .. } => Some((a.clone(), value.clone())),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            if lits.is_empty() {
                self.error(seq.span, format!("`{}` has no literals: `.literals` walks a literal union's members (`\"json\" | \"text\"`) [comptime-literals]", self.world.normalized(&ty)));
                return None;
            }
            let n = lits.len();
            let text = |l: &salvo_syntax::ast::TypeLit| match l {
                salvo_syntax::ast::TypeLit::Str(s) => s.clone(),
                other => other.to_string(),
            };
            let siblings: Vec<String> = lits.iter().map(|(_, l)| text(l)).collect();
            let owner = named_base(&ty).unwrap_or_else(|| self.world.normalized(&ty));
            return Some(
                lits.into_iter()
                    .enumerate()
                    .map(|(i, (t, l))| Binding {
                        name: text(&l),
                        decl_span: t.span(),
                        label: format!("{owner} literal {l}"),
                        ty: t,
                        index: i,
                        first: i == 0,
                        last: i + 1 == n,
                        is_arm: true,
                        siblings: siblings.clone(),
                        sibling_owner: owner.clone(),
                        default_of: None,
                        docs: None,
                        annotations: Vec::new(),
                        literal: Some(l),
                    })
                    .collect(),
            );
        }
        if seq.arms {
            if kind != TypeKindWord::Union {
                self.error(seq.span, format!("`{}` is a `{}`, and only a `Union` has `.arms` [comptime-fields]", self.world.normalized(&ty), kind.word()));
                return None;
            }
            let arms = self.arms_of_type(&ty);
            let n = arms.len();
            let siblings: Vec<String> = arms.iter().map(arm_name).collect();
            let sibling_owner = named_base(&ty).unwrap_or_else(|| self.world.normalized(&ty));
            return Some(
                arms.into_iter()
                    .enumerate()
                    .map(|(i, a)| Binding {
                        docs: self.world.docs_of(&a),
                        name: arm_name(&a),
                        decl_span: a.span(),
                        label: format!("{} arm `{}`", self.world.normalized(&ty), a),
                        ty: a,
                        index: i,
                        first: i == 0,
                        last: i + 1 == n,
                        is_arm: true,
                        siblings: siblings.clone(),
                        sibling_owner: sibling_owner.clone(),
                        default_of: None,
                        annotations: Vec::new(),
                        literal: None,
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
        // [comptime-generic-fn] A generic struct is walked at an instance:
        // its field types come out with the instance's arguments in place of
        // its parameters. A bare root has no arguments to put there.
        let given: Vec<Type> = match &ty {
            Type::Named { base, .. } => base.args.clone(),
            _ => Vec::new(),
        };
        let params: Vec<String> = s.generics.iter().map(|g| g.name.clone()).collect();
        if params.len() != given.len() {
            let what = if seq.arms { "arms" } else { "fields" };
            let wanted = params.join(", ");
            if given.is_empty() {
                self.error(
                    seq.span,
                    format!(
                        "`{name}` is generic, so its {what} are walked at an instance: write \
                         `{name}<{wanted}>.{what}` with the type arguments meant (a type \
                         parameter of this fn, or a concrete type) [comptime-generic-fn]"
                    ),
                );
            } else {
                self.error(
                    seq.span,
                    format!(
                        "`{name}` takes {} type argument(s), found {} [comptime-generic-fn]",
                        params.len(),
                        given.len()
                    ),
                );
            }
            return None;
        }
        let map: HashMap<String, Type> = params.into_iter().zip(given).collect();
        let n = s.fields.len();
        let type_name = name.clone();
        let siblings: Vec<String> = s.fields.iter().map(|f| f.name.name.clone()).collect();
        Some(
            s.fields
                .iter()
                .enumerate()
                .map(|(i, f)| Binding {
                    // [state-infer] An inferred field has no written type; a
                    // comptime walk never targets a generated iterator struct,
                    // so this fallback is unreachable in practice.
                    ty: f.ty.as_ref().map(|t| subst_generics(t, &map)).unwrap_or_else(|| Type::Named {
                        qualifiers: Vec::new(),
                        base: salvo_syntax::ast::TypeRef {
                            alias: None, at: None, binder: false, established: false,
                            value_args: Vec::new(), from: Vec::new(),
                            name: Ident { name: "Int".to_string(), span: f.span },
                            args: Vec::new(), span: f.span,
                        },
                    }),
                    name: f.name.name.clone(),
                    index: i,
                    first: i == 0,
                    last: i + 1 == n,
                    decl_span: f.span,
                    label: format!(
                        "{type_name}.{}: {}",
                        f.name.name,
                        f.ty.as_ref().map(|t| subst_generics(t, &map).to_string()).unwrap_or_else(|| "_".into())
                    ),
                    is_arm: false,
                    siblings: siblings.clone(),
                    sibling_owner: type_name.clone(),
                    default_of: f.default.as_ref().map(|_| ty.clone()),
                    docs: doc_text(&f.docs),
                    annotations: f.annotations.clone(),
                    literal: None,
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

    /// [comptime-name] Whether every literal is a name of the binder's
    /// sequence — a member of the literal union `binder.name` is drawn from.
    /// Reports each one that is not, naming the sequence's owner.
    fn check_names(&mut self, b: &Binding, lits: &[String], span: Span) -> bool {
        let mut ok = true;
        for lit in lits {
            if !b.siblings.iter().any(|s| s == lit) {
                let what = if b.is_arm { "arm" } else { "field" };
                self.error(span, format!("`{}` has no {what} `{lit}` [comptime-name]", b.sibling_owner));
                ok = false;
            }
        }
        ok
    }

    /// [comptime-kind] A type a comptime test compares against must name
    /// types: a declared type (not one of `std.core.comptime`'s compile-time
    /// structs), `None`, or a type parameter in scope. A kind word that
    /// names no type is the old kind-test spelling, and says so.
    fn check_type_names(&mut self, ty: &Type, span: Span, allowed: &[String]) -> bool {
        match ty {
            Type::Named { base, .. } => {
                let name = base.name.name.as_str();
                let mut generics = self.generics();
                // [comptime-pattern] `*`, the pattern's own binders, and the
                // binders of enclosing patterns are names here too.
                generics.push("*".to_string());
                generics.extend(allowed.iter().cloned());
                generics.extend(self.type_binders.iter().map(|(n, ..)| n.clone()));
                let declared = self.world.structs_of(name).iter().any(|(_, s)| !s.comptime)
                    || self.world.types_of(name).iter().any(|(_, t)| !t.comptime);
                let mut ok = true;
                if !(declared || name == "None" || generics.iter().any(|g| g == name)) {
                    if TypeKindWord::parse(name).is_some() {
                        self.error(
                            span,
                            format!(
                                "`{name}` is a kind, not a type: a kind is tested on `.kind` \
                                 (`field.kind is {name}`), and `is` after a type compares \
                                 types [comptime-kind]"
                            ),
                        );
                    } else {
                        self.error(span, format!("unknown type `{name}` in a compile-time type test [comptime-inline]"));
                    }
                    ok = false;
                }
                for a in &base.args {
                    ok &= self.check_type_names(a, span, allowed);
                }
                ok
            }
            Type::Union { arms, .. } => arms.iter().fold(true, |ok, a| self.check_type_names(a, span, allowed) && ok),
            Type::Nullable { inner, .. } | Type::Spread { inner, .. } => self.check_type_names(inner, span, allowed),
            Type::QualifiedGroup { base, .. } => self.check_type_names(base, span, allowed),
            Type::Tuple { elems, .. } => elems.iter().fold(true, |ok, e| self.check_type_names(e, span, allowed) && ok),
            Type::Array { elem, .. } => self.check_type_names(elem, span, allowed),
            Type::Fn { params, ret, .. } => {
                params.iter().fold(true, |ok, p| self.check_type_names(p, span, allowed) && ok)
                    && self.check_type_names(ret, span, allowed)
            }
            Type::Literal { .. } => true,
        }
    }

    /// [comptime-qualifier] Whether `annotations` (a field's or a struct's)
    /// carry the comptime qualifier `qual`: without arguments it asks only
    /// that; with them, each is a constant binder (bound to the written
    /// constant) or a constant (compared).
    fn match_annotation(&mut self, annotations: &[TypeRef], qual: &TypeRef, binders: &[Ident], subject: &str, span: Span, can_bind: bool) -> Option<bool> {
        let Some(decl) = self.world.comptime_qual(&qual.name.name) else {
            self.error(span, format!("`{}` is not a comptime qualifier [comptime-qualifier]", qual.name.name));
            return None;
        };
        let of = match &decl.of {
            Type::Named { base, .. } => base.name.name.clone(),
            _ => String::new(),
        };
        let slots = decl.value_slots.len();
        if of != subject {
            self.error(span, format!("`{}` describes a `{of}`, not a `{subject}` [comptime-qualifier]", qual.name.name));
            return None;
        }
        if let Some(b) = binders.iter().find(|b| !b.name.starts_with(|c: char| c.is_lowercase())) {
            self.error(span, format!("`{}` would bind a type, and a comptime qualifier holds constants: name a constant binder in lowercase [comptime-qualifier]", b.name));
            return None;
        }
        if !binders.is_empty() && !can_bind {
            self.error(span, "a pattern's binders bind nothing under `||` or `!` — what the test proves holds only where it did: write `*` there [comptime-pattern] [flow-or-not]");
            return None;
        }
        if !qual.value_args.is_empty() && qual.value_args.len() != slots {
            self.error(span, format!("`{}` takes {slots} constant(s), found {} [comptime-qualifier]", qual.name.name, qual.value_args.len()));
            return None;
        }
        let Some(found) = annotations.iter().find(|a| a.name.name == qual.name.name) else {
            return Some(false);
        };
        let mut bound = Vec::new();
        for (p, a) in qual.value_args.iter().zip(&found.value_args) {
            let Some(actual) = const_arg(a) else { return Some(false) };
            match self.match_const(p, binders, span)? {
                Some(id) => bound.push((id, actual)),
                None if Some(&actual) == const_arg(p).as_ref() => {}
                None => return Some(false),
            }
        }
        self.value_binds.extend(bound);
        Some(true)
    }

    /// [comptime-qualifier] A pattern argument: `Some(binder)` when it is one
    /// of `binders` (the caller binds it), `None` when it is a constant to
    /// compare, and an error for anything else.
    fn match_const(&mut self, pattern: &Type, binders: &[Ident], span: Span) -> Option<Option<Ident>> {
        if let Type::Named { qualifiers, base } = pattern {
            if qualifiers.is_empty() {
                if let Some(b) = binders.iter().find(|b| b.name == base.name.name) {
                    if self.value_binds.iter().any(|(v, _)| v.name == b.name) {
                        self.error(span, format!("pattern binder `{}` is declared twice in one condition [comptime-pattern]", b.name));
                        return None;
                    }
                    return Some(Some(b.clone()));
                }
                if const_arg(pattern).is_some() {
                    return Some(None);
                }
            }
        }
        self.error(span, format!("a qualifier's argument in a pattern is a constant binder declared before it (`<c> Short(c)`) or a constant, not `{pattern}` [comptime-qualifier]"));
        None
    }

    /// [comptime-qualifier] Whether the type `actual` carries every
    /// qualifier `pattern` writes, binding or comparing their constants
    /// (`<lo, hi> InRange(lo, hi) *`). An alias is looked through.
    fn match_type_quals(&mut self, pattern: &[TypeRef], actual: &Type, binders: &[Ident], span: Span) -> Option<bool> {
        let have = self.type_quals(actual, 0);
        let mut bound = Vec::new();
        for want in pattern {
            let Some(found) = have.iter().find(|h| h.name.name == want.name.name) else {
                return Some(false);
            };
            if !want.value_args.is_empty() && want.value_args.len() != found.value_args.len() {
                return Some(false);
            }
            for (p, a) in want.value_args.iter().zip(&found.value_args) {
                let Some(actual) = const_arg(a) else { return Some(false) };
                match self.match_const(p, binders, span)? {
                    Some(id) => bound.push((id, actual)),
                    None if Some(&actual) == const_arg(p).as_ref() => {}
                    None => return Some(false),
                }
            }
        }
        self.value_binds.extend(bound);
        Some(true)
    }

    /// The qualifiers written on a type, through an argument-less alias.
    fn type_quals(&self, ty: &Type, depth: u32) -> Vec<TypeRef> {
        match ty {
            Type::Named { qualifiers, base } => {
                let mut out = qualifiers.clone();
                if base.args.is_empty() && depth < 8 {
                    if let Some(t) = self.world.type_named(base.name.name.as_str()) {
                        if let (Some(alias), false) = (&t.alias, t.intrinsic) {
                            out.extend(self.type_quals(alias, depth + 1));
                        }
                    }
                }
                out
            }
            Type::QualifiedGroup { qualifiers, base, .. } => {
                let mut out = qualifiers.clone();
                out.extend(self.type_quals(base, depth + 1));
                out
            }
            _ => Vec::new(),
        }
    }

    /// [comptime-pattern] A pattern's binders and the names in it: a binder
    /// may not reuse the name of a type, a type parameter or an enclosing
    /// binder (it would hide it, as [var-no-shadow] refuses for a local), and
    /// every other name must name a type.
    fn check_pattern(&mut self, binders: &[Ident], pattern: &Type, span: Span) -> bool {
        let mut ok = true;
        for (i, b) in binders.iter().enumerate() {
            let name = b.name.as_str();
            let hides = self.world.declares_type(name)
                || self.generics().iter().any(|g| g == name)
                || self.type_binders.iter().any(|(n, ..)| n == name)
                || binders[..i].iter().any(|o| o.name == name);
            if hides {
                self.error(
                    span,
                    format!(
                        "pattern binder `{name}` would hide a type, type parameter or binder of                          that name: pick a fresh one [comptime-pattern]"
                    ),
                );
                ok = false;
            }
        }
        let names: Vec<String> = binders.iter().map(|b| b.name.clone()).collect();
        ok && self.check_type_names(pattern, span, &names)
    }

    /// [comptime-pattern] The shape a pattern matches against: qualifiers
    /// erased, an argument-less alias expanded, and a union (or `T?`)
    /// flattened to its arms with `None` last.
    fn peel(&self, ty: &Type, depth: u32) -> Type {
        match ty {
            Type::QualifiedGroup { base, .. } => self.peel(base, depth),
            Type::Named { base, .. } => {
                if base.args.is_empty() && depth < 8 {
                    if let Some(t) = self.world.type_named(base.name.name.as_str()) {
                        if let (Some(alias), false) = (&t.alias, t.intrinsic) {
                            return self.peel(alias, depth + 1);
                        }
                    }
                }
                Type::Named { qualifiers: Vec::new(), base: base.clone() }
            }
            Type::Union { span, .. } | Type::Nullable { span, .. } => Type::Union {
                arms: self.world.arms_of(ty),
                span: *span,
            },
            other => other.clone(),
        }
    }

    /// [comptime-pattern] Whether `actual` matches `pattern`, binding each of
    /// `binders` to the part it stands at — as written, qualifiers kept. A
    /// binder met twice must meet the same type both times (`<K> Map<K, K>`).
    /// Unions match arm by arm, in order.
    fn match_pattern(&self, pattern: &Type, actual: &Type, binders: &[String], out: &mut Vec<(String, Type)>) -> bool {
        if let Type::Named { base, .. } = pattern {
            if base.args.is_empty() && base.name.name == "*" {
                return true;
            }
            if base.args.is_empty() && binders.iter().any(|b| *b == base.name.name) {
                if let Some((_, prev)) = out.iter().find(|(n, _)| *n == base.name.name) {
                    return self.world.normalized(prev) == self.world.normalized(actual);
                }
                out.push((base.name.name.clone(), actual.clone()));
                return true;
            }
        }
        // [comptime-pattern] `X?` matches any union with a `None` arm, `X` standing for the
        // rest: `*?` is every optional, and `<E> E?` binds `"a" | "b"` in
        // `("a" | "b")?` — whose arms are three, so arm-by-arm matching
        // against the pattern's two found none.
        if let Type::Nullable { inner, span } = pattern {
            let Type::Union { arms, .. } = self.peel(actual, 0) else { return false };
            let is_none = |t: &Type| matches!(t, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() && base.name.name == "None");
            let rest: Vec<Type> = arms.iter().filter(|a| !is_none(a)).cloned().collect();
            if rest.len() + 1 != arms.len() || rest.is_empty() {
                return false;
            }
            let rest = if rest.len() == 1 { rest.into_iter().next().unwrap() } else { Type::Union { arms: rest, span: *span } };
            return self.match_pattern(inner, &rest, binders, out);
        }
        let p = self.peel(pattern, 0);
        let a = self.peel(actual, 0);
        let all = |this: &Self, ps: &[Type], as_: &[Type], out: &mut Vec<(String, Type)>| {
            ps.len() == as_.len() && ps.iter().zip(as_).all(|(p, a)| this.match_pattern(p, a, binders, out))
        };
        match (&p, &a) {
            (Type::Named { base: pb, .. }, Type::Named { base: ab, .. }) => {
                pb.name.name == ab.name.name && all(self, &pb.args, &ab.args, out)
            }
            (Type::Union { arms: pa, .. }, Type::Union { arms: aa, .. }) => all(self, pa, aa, out),
            (Type::Tuple { elems: pe, .. }, Type::Tuple { elems: ae, .. }) => all(self, pe, ae, out),
            (Type::Array { elem: pe, .. }, Type::Array { elem: ae, .. }) => self.match_pattern(pe, ae, binders, out),
            (Type::Spread { inner: pi, .. }, Type::Spread { inner: ai, .. }) => self.match_pattern(pi, ai, binders, out),
            (Type::Fn { params: pp, ret: pr, .. }, Type::Fn { params: ap, ret: ar, .. }) => {
                all(self, pp, ap, out) && self.match_pattern(pr, ar, binders, out)
            }
            (Type::Literal { value: pv, .. }, Type::Literal { value: av, .. }) => pv.to_string() == av.to_string(),
            _ => false,
        }
    }

    /// [comptime-pattern] Expands `block` with `binds` in scope as types, and
    /// records for each binder whether this scope used it.
    fn expand_bound(&mut self, binds: Vec<(Ident, Type)>, mut block: Block, out: &mut Vec<Stmt>) {
        let base = self.type_binders.len();
        for (id, ty) in binds {
            self.type_binders.push((id.name.clone(), ty, id.span, 0));
        }
        // [comptime-scope] A kept body is a scope.
        self.rename_counter += 1;
        rename_locals(&mut block, self.rename_counter);
        self.expand_block(&mut block);
        let scope: Vec<(String, Type, Span, usize)> = self.type_binders.drain(base..).collect();
        for (name, _, span, uses) in scope {
            let written = self.unmap(span);
            let entry = self.binder_used.entry(((written.expn, written.start), name)).or_insert((written, false));
            entry.1 |= uses > 0;
        }
        out.extend(block.stmts);
    }

    /// [comptime-scope] Splices the body a comptime test kept (an `[if]`'s
    /// branch, a `[when]`'s arm) as a scope, as an ordinary `if` or `when`
    /// body is one: its locals are renamed apart, so nothing it declares is
    /// visible after it (user decision 2026-10-20; the body was spliced flat
    /// before, and a `let` in it leaked into the code that followed). A
    /// terminal `[if …] ...` renames the rest of the block, which holds every
    /// use, so it reads as before.
    fn expand_scoped(&mut self, mut block: Block, out: &mut Vec<Stmt>) {
        self.rename_counter += 1;
        rename_locals(&mut block, self.rename_counter);
        self.expand_block(&mut block);
        out.extend(block.stmts);
    }

    /// Evaluates a condition. `binds` collects what its patterns bind, when
    /// `can_bind`: what a test proves holds only where the test did, so
    /// under `||` and `!` nothing is bound [flow-or-not] [comptime-pattern].
    fn eval_cond(&mut self, cond: &CompCond, binds: &mut Vec<(Ident, Type)>, can_bind: bool) -> Option<bool> {
        match cond {
            CompCond::Not(inner, _) => self.eval_cond(inner, binds, false).map(|b| !b),
            // [comptime-logic] Both sides are always evaluated: a misspelled
            // name on the side that would short-circuit away is still a
            // mistake, and is still reported.
            // [comptime-pattern] What the left side bound is in scope on the
            // right: `field.type is <S> S? && S.kind is Struct`.
            CompCond::And(a, b, _) => {
                let before = binds.len();
                let x = self.eval_cond(a, binds, can_bind);
                // A failed test bound nothing, so the right side cannot read
                // what it would have bound: decided here.
                if x == Some(false) && cond_binds(a) {
                    return Some(false);
                }
                let base = self.type_binders.len();
                if x == Some(true) {
                    for (id, ty) in &binds[before..] {
                        self.type_binders.push((id.name.clone(), ty.clone(), id.span, 0));
                    }
                }
                let y = self.eval_cond(b, binds, can_bind);
                let lent: Vec<(String, Type, Span, usize)> = self.type_binders.drain(base..).collect();
                for (name, _, span, uses) in lent {
                    if uses > 0 {
                        let written = self.unmap(span);
                        self.binder_used.entry(((written.expn, written.start), name)).or_insert((written, false)).1 = true;
                    }
                }
                Some(x? && y?)
            }
            CompCond::Or(a, b, _) => {
                let x = self.eval_cond(a, binds, false);
                let y = self.eval_cond(b, binds, false);
                Some(x? || y?)
            }
            CompCond::Kind { ty, kind, .. } => {
                let t = self.resolve_ty(ty)?;
                let actual = self.world.classify(&t, &self.generics(), 0);
                Some(kind_matches(*kind, actual))
            }
            CompCond::Is { ty, binders, target, span } => {
                // [comptime-qualifier] `T is Named`, `T is <n> Named(n)`: the
                // struct's own comptime qualifiers, not a type test.
                if let Type::Named { qualifiers, base } = target {
                    if qualifiers.is_empty() && self.world.comptime_qual(&base.name.name).is_some() {
                        if ty.via_type {
                            self.error(*span, format!("`{}` describes a declaration, not a type: test the binder itself (`{} is {}`) [comptime-qualifier]", base.name.name, ty.root.name, base.name.name));
                            return None;
                        }
                        let annotations = match &self.bound {
                            Some((param, target)) if *param == ty.root.name => self.world.struct_named(&target.name).map(|s| s.annotations.clone()),
                            _ => self.world.struct_named(&ty.root.name).map(|s| s.annotations.clone()),
                        };
                        let Some(annotations) = annotations else {
                            self.error(*span, format!("`{}` is not a struct, so it carries no comptime qualifiers [comptime-qualifier]", ty.root.name));
                            return None;
                        };
                        return self.match_annotation(&annotations, base, binders, "Struct", *span, can_bind);
                    }
                }
                let t = self.resolve_ty(ty)?;
                // [comptime-qualifier] A pattern's qualifiers are tested, and
                // their constants bound (`<lo, hi> InRange(lo, hi) *`); a
                // pattern without qualifiers still ignores the type's.
                let (target, quals_ok) = match target {
                    Type::Named { qualifiers, base } if !qualifiers.is_empty() => {
                        let values = binders.iter().any(|b| b.name.starts_with(|c: char| c.is_lowercase()));
                        if values && !can_bind {
                            self.error(*span, "a pattern's binders bind nothing under `||` or `!` — what the test proves holds only where it did: write `*` there [comptime-pattern] [flow-or-not]");
                            return None;
                        }
                        let ok = self.match_type_quals(qualifiers, &t, binders, *span)?;
                        (Type::Named { qualifiers: Vec::new(), base: base.clone() }, ok)
                    }
                    other => (other.clone(), true),
                };
                let target = &target;
                let type_binders: Vec<Ident> = binders.iter().filter(|b| !b.name.starts_with(|c: char| c.is_lowercase())).cloned().collect();
                let binders = &type_binders;
                if !self.check_pattern(binders, target, *span) {
                    return None;
                }
                if !quals_ok {
                    return Some(false);
                }
                if !binders.is_empty() && !can_bind {
                    self.error(
                        *span,
                        "a pattern's binders bind nothing under `||` or `!` — what the test \
                         proves holds only where it did: write `*` there [comptime-pattern] \
                         [flow-or-not]",
                    );
                    return None;
                }
                let names: Vec<String> = binders.iter().map(|b| b.name.clone()).collect();
                let mut got: Vec<(String, Type)> = Vec::new();
                let matched = self.match_pattern(target, &t, &names, &mut got);
                if matched {
                    for (name, ty) in got {
                        let Some(id) = binders.iter().find(|b| b.name == name) else { continue };
                        if binds.iter().any(|(b, _)| b.name == name) {
                            self.error(*span, format!("pattern binder `{name}` is declared twice in one condition [comptime-pattern]"));
                            return None;
                        }
                        binds.push((id.clone(), ty));
                    }
                }
                Some(matched)
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
            CompCond::NameIs { binder, lits, span } => {
                let Some(b) = self.binding(&binder.name).cloned() else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", binder.name));
                    return None;
                };
                // [comptime-name] `binder.name` is a literal of the union of
                // its sequence's names: a literal outside it is a mistake at
                // the template, caught at the first instantiation.
                if !self.check_names(&b, lits, *span) {
                    return None;
                }
                Some(lits.iter().any(|l| *l == b.name))
            }
            CompCond::Flag { binder, last, span } => {
                let Some(b) = self.binding(&binder.name) else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", binder.name));
                    return None;
                };
                Some(if *last { b.last } else { b.first })
            }
            CompCond::Annotated { subject, binders, qual, span } => {
                let Some(b) = self.binding(&subject.name).cloned() else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", subject.name));
                    return None;
                };
                if b.is_arm {
                    self.error(*span, format!("`{}` is an arm, and an arm carries no comptime qualifiers yet [comptime-qualifier]", subject.name));
                    return None;
                }
                self.match_annotation(&b.annotations, qual, binders, "Field", *span, can_bind)
            }
            CompCond::Unset { binder, docs: true, span } => {
                if let Some(b) = self.binding(&binder.name) {
                    return Some(b.docs.is_none());
                }
                if let Some((param, target)) = &self.bound {
                    if binder.name == *param {
                        return Some(self.world.docs_of(&target.ty).is_none());
                    }
                }
                if self.world.declares_type(&binder.name) {
                    return Some(self.world.docs_of(&World::simple_named(&binder.name, binder.span)).is_none());
                }
                self.error(*span, format!("`{}` is neither a binder nor a type [comptime-docs]", binder.name));
                None
            }
            CompCond::Unset { binder, docs: false, span } => {
                let Some(b) = self.binding(&binder.name) else {
                    self.error(*span, format!("`{}` is not a binder [comptime-inline]", binder.name));
                    return None;
                };
                if b.is_arm {
                    self.error(*span, format!("`{}` is an arm, and only a field has a default [comptime-default]", binder.name));
                    return None;
                }
                Some(b.default_of.is_none())
            }
            CompCond::NameEq { a, b, span } => {
                let (Some(x), Some(y)) = (self.binding(&a.name).map(|b| b.name.clone()), self.binding(&b.name).map(|b| b.name.clone())) else {
                    self.error(*span, "both sides of a `.name` comparison must be binders [comptime-name]");
                    return None;
                };
                Some(x == y)
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
        let mut copy = Retag::fresh(self.next_expn);
        let id = copy.expn;
        visit_mut::walk_block(&mut copy, &mut block);
        self.expansions.push(ExpansionRecord {
            id,
            parent: self.current,
            kind: ExpansionKind::Copy {
                target: b.decl_span,
                label: b.label.clone(),
                written: self.in_place.then(|| {
                    if b.is_arm {
                        format!("`{binder}` is `{}`", b.name)
                    } else {
                        format!("`{binder}` is `{}` (`{}`)", b.name, b.ty)
                    }
                }),
            },
        });
        self.rename_counter += 1;
        rename_locals(&mut block, self.rename_counter);
        self.env.push((binder.to_string(), b.clone()));
        let enclosing = std::mem::replace(&mut self.current, id);
        self.expand_block(&mut block);
        self.current = enclosing;
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
                let mut binds = Vec::new();
                let saved_values = std::mem::take(&mut self.value_binds);
                let taken = self.eval_cond(&cond, &mut binds, true);
                let values = std::mem::replace(&mut self.value_binds, saved_values);
                let Some(taken) = taken else { return };
                if taken {
                    let base = self.value_binders.len();
                    for (id, c) in values {
                        self.value_binders.push((id.name.clone(), c, id.span, 0));
                    }
                    self.expand_bound(binds, then, out);
                    let scope: Vec<(String, crate::types::Const, Span, usize)> = self.value_binders.drain(base..).collect();
                    for (name, _, span, uses) in scope {
                        let written = self.unmap(span);
                        let entry = self.binder_used.entry(((written.expn, written.start), name)).or_insert((written, false));
                        entry.1 |= uses > 0;
                    }
                } else if let Some(chosen) = else_ {
                    self.expand_scoped(chosen, out);
                }
            }
            CompStmt::WhenType { ty, arms, span } => {
                let Some(t) = self.resolve_ty(&ty) else { return };
                let bodies: Vec<Span> = arms.iter().map(|a| a.body.span).collect();
                let mut ok = true;
                for arm in &arms {
                    ok &= self.check_pattern(&arm.binders, &arm.pattern, arm.span);
                }
                if !ok {
                    return;
                }
                for arm in arms {
                    let names: Vec<String> = arm.binders.iter().map(|b| b.name.clone()).collect();
                    let mut got: Vec<(String, Type)> = Vec::new();
                    if self.match_pattern(&arm.pattern, &t, &names, &mut got) {
                        let binds = got
                            .into_iter()
                            .filter_map(|(n, ty)| arm.binders.iter().find(|b| b.name == n).map(|id| (id.clone(), ty)))
                            .collect();
                        self.record_when_arms(span, Vec::new(), true, bodies.clone());
                        self.expand_bound(binds, arm.body, out);
                        return;
                    }
                }
                self.record_when_arms(span, vec![t.to_string()], true, bodies);
                // [comptime-pattern] [when-exhaustive] The arms cover the type
                // this copy meets, as a `when` with a subject covers its arms.
                // Types are not a closed set, but the subject's range is: every
                // expansion knows the struct's fields (or the union's arms) it
                // walks, so each copy either finds its arm or is reported.
                let subject = if ty.via_type { format!("{}.type", ty.root.name) } else { ty.root.name.clone() };
                self.error(
                    span,
                    format!(
                        "no arm of this `[when {subject}]` matches `{t}`, which `{subject}` is \
                         here: add an arm for it (`is *` matches every type the arms above it \
                         did not) [comptime-pattern] [when-exhaustive]"
                    ),
                );
            }
            CompStmt::WhenKind { ty, arms, span } => {
                let Some(t) = self.resolve_ty(&ty) else { return };
                let actual = self.world.classify(&t, &self.generics(), 0);
                // Exhaustive over the kinds (comptime round 3), with no `else`
                // [when-exhaustive]: a kind added to the language is an error in
                // every compfn that did not consider it.
                if let Some(d) = kind_arms_incomplete(&arms, span) {
                    // At the written construct, as every expansion error is
                    // (a copy's nodes are the template's [origin]).
                    self.error(span, d.message);
                    return;
                }
                let chosen = arms
                    .into_iter()
                    .find(|a| kind_matches(a.kind, actual))
                    .map(|a| a.body);
                let Some(chosen) = chosen else {
                    self.error(span, format!("`{}` is a `{}`, which this `[when …]` does not handle [comptime-inline]", self.world.normalized(&t), actual.word()));
                    return;
                };
                self.expand_scoped(chosen, out);
            }
            CompStmt::WhenName { binder, arms, span } => {
                let Some(b) = self.binding(&binder.name).cloned() else {
                    self.error(span, format!("`{}` is not a binder of an enclosing `[for …]` [comptime-name]", binder.name));
                    return;
                };
                // [comptime-name] Every literal names a member of the union,
                // and no name is covered twice: the second arm could never be
                // taken [when-exhaustive].
                let mut covered: Vec<String> = Vec::new();
                let mut ok = true;
                for arm in &arms {
                    for (lit, lspan) in &arm.lits {
                        if covered.contains(lit) {
                            self.error(*lspan, format!("`{lit}` is already covered by an earlier arm of this `[when {}.name]` [comptime-name] [when-exhaustive]", binder.name));
                            ok = false;
                        }
                        ok &= self.check_names(&b, std::slice::from_ref(lit), *lspan);
                        covered.push(lit.clone());
                    }
                }
                if !ok {
                    return;
                }
                {
                    let uncovered: Vec<String> =
                        b.siblings.iter().filter(|s| !covered.contains(s)).map(|s| format!("\"{s}\"")).collect();
                    let bodies = arms.iter().map(|a| a.body.span).collect();
                    self.record_when_arms(span, uncovered, false, bodies);
                    let missing: Vec<String> = b
                        .siblings
                        .iter()
                        .filter(|s| !covered.contains(s))
                        .map(|s| format!("`{s}`"))
                        .collect();
                    if !missing.is_empty() {
                        let what = if b.is_arm { "arm" } else { "field" };
                        self.error(
                            span,
                            format!(
                                "this `[when {}.name]` does not cover every {what} of `{}`: \
                                 missing {} — add them [comptime-name] [when-exhaustive]",
                                binder.name,
                                b.sibling_owner,
                                missing.join(", ")
                            ),
                        );
                        return;
                    }
                }
                let chosen = arms
                    .into_iter()
                    .find(|a| a.lits.iter().any(|(l, _)| *l == b.name))
                    .map(|a| a.body);
                if let Some(chosen) = chosen {
                    self.expand_scoped(chosen, out);
                }
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
                        // An expanded-in-place compfn: the parameter's written type.
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
                let siblings: Vec<String> = arms.iter().map(arm_name).collect();
                let sibling_owner = named_base(&subject_ty).unwrap_or_else(|| self.world.normalized(&subject_ty));
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
                        siblings: siblings.clone(),
                        sibling_owner: sibling_owner.clone(),
                        default_of: None,
                        docs: self.world.docs_of(arm),
                        annotations: Vec::new(),
                        literal: None,
                    };
                    let body = self.copy(&body, &binder.name, &b);
                    // The check's spans come from the union's declaration; two
                    // stamps at one union would key the checker's `is` tables on
                    // the same node, so each copy gets its own.
                    for c in &mut check {
                        let mut made = self.made();
                        visit_mut::walk_type_ref(&mut made, c);
                    }
                    let bspan = span.in_expansion(self.made().expn);
                    branches.push(WhenBranch {
                        check,
                        binding: None,
                        widen: false,
                        body,
                        span: bspan,
                    });
                }
                let when_span = span.in_expansion(self.made().expn);
                out.push(Stmt::Expr(Expr::When {
                    binding: None,
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
                let at = self
                    .bound
                    .as_ref()
                    .map(|(_, t)| t.name.clone())
                    .or_else(|| self.fn_name.clone())
                    .unwrap_or_default();
                let where_ = self.report_span(span);
                self.errors.push(Diagnostic::error(
                    if at.is_empty() { text } else { format!("`{at}` refused: {text} [comptime-refuse]") },
                    where_,
                ));
            }
        }
    }

    /// [origin] A fresh expansion for nodes the expansion makes up (the
    /// `when` an `[when value]` unrolls into, its arms' tests): they keep the
    /// range of what they were made from, and their diagnostics land at the
    /// site.
    fn made(&mut self) -> Retag {
        let made = Retag::fresh(self.next_expn);
        self.expansions.push(ExpansionRecord { id: made.expn, parent: self.current, kind: ExpansionKind::Made });
        made
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
        // [comptime-pattern] A pattern binder stands for the type it bound.
        if base.args.is_empty() {
            if let Some(i) = self.type_binders.iter().rposition(|(n, ..)| *n == base.name.name) {
                self.type_binders[i].3 += 1;
                let mut new = self.type_binders[i].1.clone();
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
                return;
            }
        }
        let replacement = if let Some(b) = base.name.name.strip_suffix(".type") {
            match self.binding(b) {
                Some(bd) => Some(bd.ty.clone()),
                None => {
                    let span = base.span;
                    self.error(span, format!("`{b}` is not a binder of an enclosing `[for …]` or `[when …]` [comptime-fields]"));
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
        // [comptime-qualifier] A constant binder reads as its constant.
        if let Expr::Ident(id) = expr {
            if let Some(i) = self.value_binders.iter().rposition(|(n, ..)| *n == id.name) {
                self.value_binders[i].3 += 1;
                let span = id.span;
                *expr = match self.value_binders[i].1.clone() {
                    crate::types::Const::Int(value) => Expr::Int { value, long: false, span },
                    crate::types::Const::Char(value) => Expr::Char { value, span },
                    crate::types::Const::Str(text) => self.str_lit(&text, span),
                };
                return;
            }
        }
        // [comptime-arm] `[arm.type](value)`: the constructor of the arm's
        // type — its qualifiers' type-fn for a qualified arm (`Ok(value)` for
        // `Ok Str`, as one writes it), the base's for a plain one.
        if let Expr::Call { callee, type_args, .. } = expr {
            if let Expr::Ident(id) = &mut **callee {
                if let Some(b) = id.name.strip_suffix(".type").map(str::to_string) {
                    let span = id.span;
                    match self.binding(&b).map(|bd| bd.ty.clone()) {
                        Some(Type::Named { base, .. }) if base.name.name == "None" => {
                            self.error(span, "a `None` arm is built with `None`, not called [comptime-arm]");
                        }
                        Some(Type::Named { qualifiers, base }) => {
                            if qualifiers.is_empty() {
                                id.name = base.name.name.clone();
                            } else {
                                id.name = qualifiers.iter().map(|q| q.name.name.clone()).collect::<Vec<_>>().join("_");
                                // A generic claim over its subject (`Ok<T> of
                                // T`) is told the subject: `Ok<Str>(value)`.
                                let generic = qualifiers.len() == 1
                                    && self.world.qual_named(&qualifiers[0].name.name).is_some_and(|q| {
                                        q.generics.len() == 1
                                            && matches!(&q.of, Type::Named { qualifiers, base: of } if qualifiers.is_empty() && of.args.is_empty() && of.name.name == q.generics[0].name)
                                    });
                                if generic && type_args.is_empty() {
                                    type_args.push(Type::Named { qualifiers: Vec::new(), base: base.clone() });
                                }
                            }
                        }
                        Some(other) => {
                            self.error(span, format!("`{other}` has no constructor to call: an arm built this way is a named type, qualified or not; a `None` arm is `None` [comptime-arm]"));
                        }
                        None => {
                            self.error(span, format!("`{b}` is not a binder of an enclosing `[for …]` or `[when …]` [comptime-fields]"));
                        }
                    }
                }
            }
        }
        // [comptime-is-splice] `v is [field.type]` tests what the type says:
        // its qualifiers, then its base, as an arm's `is` check is written.
        if let Expr::Is { check, .. } = expr {
            if let Some(b) = check.first().and_then(|r| r.name.name.strip_suffix(".type")).map(str::to_string) {
                let span = check[0].span;
                match self.binding(&b).map(|bd| bd.ty.clone()) {
                    Some(ty) => match arm_check(&ty) {
                        Some(refs) => {
                            let rest: Vec<TypeRef> = check.drain(1..).collect();
                            *check = refs.into_iter().map(|mut r| {
                                r.span = span;
                                r
                            }).chain(rest).collect();
                        }
                        None => self.error(span, format!("`{b}.type` is `{}` here, which an `is` cannot test: an `is` tests a named type, qualified or not [comptime-is-splice]", ty)),
                    },
                    None => self.error(span, format!("`{b}` is not a binder of an enclosing `[for …]` or `[when …]` [comptime-fields]")),
                }
            }
        }
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
                            self.error(span, format!("`{b}` is not a binder of an enclosing `[for …]` [comptime-access]"));
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
                        // [comptime-literals] The member itself, as a literal.
                        "value" => match &bd.literal {
                            Some(salvo_syntax::ast::TypeLit::Str(s)) => *expr = self.str_lit(s, span),
                            Some(salvo_syntax::ast::TypeLit::Int(v)) => *expr = Expr::Int { value: *v, long: false, span },
                            Some(salvo_syntax::ast::TypeLit::Long(v)) => *expr = Expr::Int { value: *v, long: true, span },
                            Some(salvo_syntax::ast::TypeLit::Bool(v)) => *expr = Expr::Bool { value: *v, span },
                            None => self.error(span, format!("`{}.value` is a literal member's, read in a `[for … in x.literals]` [comptime-literals]", root.name)),
                        },
                        "last" => *expr = Expr::Bool { value: bd.last, span },
                        // [comptime-docs] The element's doc comment, as a
                        // `Str` literal, or `None`.
                        "docs" => {
                            *expr = match &bd.docs {
                                Some(text) => self.str_lit(text, span),
                                None => Expr::Ident(Ident { name: "None".to_string(), span }),
                            };
                        }
                        // [comptime-default] The field's declared default,
                        // evaluated where the struct is declared; `None`
                        // where it declares none, so a copy can pass it to a
                        // `T?` either way.
                        "default" if bd.is_arm => {
                            self.error(span, format!("`{}` is an arm, and only a field has a default [comptime-default]", root.name));
                        }
                        "default" => {
                            *expr = match &bd.default_of {
                                Some(owner) => Expr::FieldDefault {
                                    owner: owner.clone(),
                                    field: Ident { name: bd.name.clone(), span },
                                    ty: bd.ty.clone(),
                                    span,
                                },
                                None => Expr::Ident(Ident { name: "None".to_string(), span }),
                            };
                        }
                        "kind" => {
                            self.error(span, format!("`{}.kind` is tested in a comptime head (`[if {}.kind is Struct]`, `[when {}.kind]`), not read as a value [comptime-kind]", root.name, root.name, root.name));
                        }
                        other => {
                            self.error(span, format!("a `Field`/`Arm` has `.name`, `.kind`, `.type`, `.index`, `.first`, `.last`, `.docs` and (a field) `.default`, not `.{other}` [comptime-fields]"));
                        }
                    }
                    return;
                }
                if let Some((param, target)) = &self.bound {
                    if root.name == *param && field.name == "name" {
                        let name = target.name.clone();
                        *expr = self.str_lit(&name, span);
                    } else if root.name == *param && field.name == "docs" {
                        // [comptime-docs] The stamped type's doc comment.
                        let docs = self.world.docs_of(&target.ty);
                        *expr = match docs {
                            Some(text) => self.str_lit(&text, span),
                            None => Expr::Ident(Ident { name: "None".to_string(), span }),
                        };
                    }
                    return;
                }
                if root.name.starts_with(|c: char| c.is_uppercase())
                    && self.world.declares_type(&root.name)
                {
                    if field.name == "name" {
                        let name = root.name.clone();
                        *expr = self.str_lit(&name, span);
                    } else if field.name == "docs" {
                        // [comptime-docs] `Reading.docs`: a named root's.
                        let docs = self.world.docs_of(&World::simple_named(&root.name, root.span));
                        *expr = match docs {
                            Some(text) => self.str_lit(&text, span),
                            None => Expr::Ident(Ident { name: "None".to_string(), span }),
                        };
                    }
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
                                // [comptime-qualifier] A constant binder's
                                // `Int` or `Char` reads as its text, as it
                                // would at run time.
                                Expr::Int { value, long: false, .. } => push_text(&mut folded, &value.to_string()),
                                Expr::Char { value, .. } => push_text(&mut folded, &value.to_string()),
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
                        self.error(span, format!("`[{b}]:` names a field through a binder, and `{b}` is not a field binder of an enclosing `[for …]` [comptime-access]"));
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

/// Every named type's name in `ty`, at its written span.
fn collect_type_names(ty: &Type, out: &mut Vec<Ident>) {
    match ty {
        Type::Named { base, .. } => {
            out.push(base.name.clone());
            for a in &base.args {
                collect_type_names(a, out);
            }
        }
        Type::QualifiedGroup { base, .. } => collect_type_names(base, out),
        Type::Union { arms, .. } => arms.iter().for_each(|a| collect_type_names(a, out)),
        Type::Spread { inner, .. } | Type::Nullable { inner, .. } => collect_type_names(inner, out),
        Type::Array { elem, .. } => collect_type_names(elem, out),
        Type::Tuple { elems, .. } => elems.iter().for_each(|e| collect_type_names(e, out)),
        Type::Fn { params, ret, .. } => {
            params.iter().for_each(|p| collect_type_names(p, out));
            collect_type_names(ret, out);
        }
        Type::Literal { .. } => {}
    }
}

/// [comptime-generic-fn] `ty` with each bare type parameter in `map` replaced
/// by its argument — a field type of a generic struct, at the instance a root
/// names (`default: T?` of `Arg<T>` walked as `Arg<Int>` is `Int?`). Written
/// out rather than as a visitor: a visitor walks into its own replacement,
/// which loops on `T` → `List<T>`. A replaced parameter keeps the qualifiers
/// written on it, in front of the argument's own.
fn subst_generics(ty: &Type, map: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Named { qualifiers, base } => {
            if base.args.is_empty() {
                if let Some(arg) = map.get(&base.name.name) {
                    if qualifiers.is_empty() {
                        return arg.clone();
                    }
                    return match arg.clone() {
                        Type::Named { qualifiers: q2, base: b2 } => {
                            let mut all = qualifiers.clone();
                            all.extend(q2);
                            Type::Named { qualifiers: all, base: b2 }
                        }
                        other => {
                            let span = other.span();
                            Type::QualifiedGroup {
                                qualifiers: qualifiers.clone(),
                                base: Box::new(other),
                                span,
                            }
                        }
                    };
                }
            }
            let mut base = base.clone();
            base.args = base.args.iter().map(|a| subst_generics(a, map)).collect();
            Type::Named {
                qualifiers: qualifiers.clone(),
                base,
            }
        }
        Type::QualifiedGroup { qualifiers, base, span } => Type::QualifiedGroup {
            qualifiers: qualifiers.clone(),
            base: Box::new(subst_generics(base, map)),
            span: *span,
        },
        Type::Union { arms, span } => Type::Union {
            arms: arms.iter().map(|a| subst_generics(a, map)).collect(),
            span: *span,
        },
        Type::Spread { inner, span } => Type::Spread {
            inner: Box::new(subst_generics(inner, map)),
            span: *span,
        },
        Type::Tuple { elems, span } => Type::Tuple {
            elems: elems.iter().map(|e| subst_generics(e, map)).collect(),
            span: *span,
        },
        Type::Array { elem, span } => Type::Array {
            elem: Box::new(subst_generics(elem, map)),
            span: *span,
        },
        Type::Nullable { inner, span } => Type::Nullable {
            inner: Box::new(subst_generics(inner, map)),
            span: *span,
        },
        Type::Fn {
            params,
            param_names,
            effects,
            deductions,
            ret,
            span,
        } => Type::Fn {
            params: params.iter().map(|p| subst_generics(p, map)).collect(),
            param_names: param_names.clone(),
            effects: effects.clone(),
            deductions: deductions.clone(),
            ret: Box::new(subst_generics(ret, map)),
            span: *span,
        },
        Type::Literal { .. } => ty.clone(),
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

/// [comptime-pattern] Whether a condition declares a pattern binder.
fn cond_binds(c: &CompCond) -> bool {
    match c {
        CompCond::Is { binders, .. } => binders.iter().any(|b| !b.name.starts_with(|c: char| c.is_lowercase())),
        CompCond::And(a, b, _) => cond_binds(a) || cond_binds(b),
        _ => false,
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
        // [comptime-is-splice] A literal arm tests the value: `v is "json"`.
        Type::Literal { value, span } => Some(vec![value.as_type_ref(*span)]),
        _ => None,
    }
}

/// The suffix [`rename_locals`] appends: `k` declared in a scoped comptime
/// body becomes `k__c3`.
const SCOPED_SUFFIX: &str = "__c";

/// [fn-by] The suffix [`rename_capturing_locals`] appends: a template's local
/// `h` that a declared parameter `h` would capture becomes `h__p`. Not a
/// scoped body's suffix, so the checker's no-shadowing rule does not read it
/// as the `h` it was written as; diagnostics still do.
const APART_SUFFIX: &str = "__p";

/// [comptime-scope] The name one scoped body's renaming wrapped: `k__c1__c3`
/// was `k__c1` before the innermost body renamed it, and `None` for a name no
/// body renamed.
pub fn scoped_outer(name: &str) -> Option<&str> {
    let at = name.rfind(SCOPED_SUFFIX)?;
    let digits = &name[at + SCOPED_SUFFIX.len()..];
    (at > 0 && !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then(|| &name[..at])
}

/// [comptime-scope] The name a renamed local was written as: every scoped
/// body's suffix stripped, and a local renamed apart from a declared
/// parameter [fn-by] as written too.
pub fn written_local(mut name: &str) -> &str {
    loop {
        if let Some(outer) = scoped_outer(name) {
            name = outer;
        } else if let Some(outer) = name.strip_suffix(APART_SUFFIX).filter(|o| !o.is_empty()) {
            name = outer;
        } else {
            return name;
        }
    }
}

/// [comptime-scope] A diagnostic in the author's terms: every renamed local
/// it names (`k__c3`, `_k__c3`) written as it was (`k`, `_k`).
pub fn unmangle_message(msg: &str) -> std::borrow::Cow<'_, str> {
    if !msg.contains(SCOPED_SUFFIX) && !msg.contains(APART_SUFFIX) {
        return std::borrow::Cow::Borrowed(msg);
    }
    let mut out = String::with_capacity(msg.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        out.push_str(written_local(word));
        word.clear();
    };
    for ch in msg.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    std::borrow::Cow::Owned(out)
}

/// Renames every local an unrolled copy declares (`let` patterns, `is`
/// bindings, `for` patterns), so two copies spliced into one block do not
/// redeclare a name [var-no-shadow]. Field names and parameters are untouched:
/// only names bound *inside* the copy are renamed. The same renaming makes a
/// kept `[if]` / `[when]` body a scope [comptime-scope]: what it declares is
/// not visible after it. The checker still refuses a renamed local that
/// shadows a visible one, by its written name ([`scoped_outer`]).
fn rename_locals(block: &mut Block, tag: u32) {
    let mut names: Vec<String> = Vec::new();
    let mut collect = CollectLocals { names: &mut names };
    visit_mut::walk_block(&mut collect, block);
    if names.is_empty() {
        return;
    }
    let renames: HashMap<String, String> = names.into_iter().map(|n| (n.clone(), format!("{n}{SCOPED_SUFFIX}{tag}"))).collect();
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
            // A lambda's parameter is renamed with the uses in its body, which
            // the walk renames: `(v: Int) -> v` in a body that also declares
            // a `v` stays one name.
            Expr::Lambda { params, .. } => {
                for p in params {
                    if let Some(n) = self.renames.get(&p.name.name) {
                        p.name.name = n.clone();
                    }
                }
            }
            _ => {}
        }
    }
}

/// [comptime-qualifier] The constant a written value argument holds.
fn const_arg(t: &Type) -> Option<crate::types::Const> {
    match t {
        Type::Named { qualifiers, base } if qualifiers.is_empty() && !base.binder => crate::types::Const::parse(&base.name.name),
        _ => None,
    }
}
