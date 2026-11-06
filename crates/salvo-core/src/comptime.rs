//! [comptime-instantiate] The comptime expansion: every `by` site stamps a
//! `compfn` at a concrete type, and every comptime construct in the copy is
//! unrolled, selected or refused, leaving an ordinary fn with an ordinary body
//! (user decisions 2026-09-28, the comptime rounds; COMPLETED.md's log).
//!
//! Runs between the interfaces and the bodies (the pipeline record C4 B), over
//! the whole source set, because a `by auto` in any file reads
//! `std.core.auto`'s compfns and the target type's declaration as written.
//! Heads are compile-time expressions, evaluated here [comptime-eval]. What it
//! leaves behind:
//!
//! - one stamped fn per (`by` site, group member), declared on the target type
//!   [fn-attached] and pushed into the target's module — the checker's
//!   [group-obligation] check finds it like a hand-written fulfilment;
//! - the body of every `fn … by X` declaration, in place;
//! - the body of every **concrete** compfn (no bound), expanded in place;
//! - no generic compfn at all: they are removed, since nothing downstream
//!   could type `T.fields`.
//!
//! Origins [node-origin]: every node of a stamped body keeps the span it has in the
//! template, and carries a fresh **expansion** id (`Span::expn`), one per
//! unrolled copy, so the checker's span-keyed side tables never see two copies
//! as one node. The `Stamp` on the fn records each expansion (what it is a copy
//! of, the field or arm it was for), and a diagnostic inside one renders through
//! that chain (`crate::origin::render`): at the field or arm the copy was for,
//! prefixed with what was being stamped.

use std::collections::HashMap;

use salvo_syntax::ast::*;
use salvo_syntax::visit_mut::{self, MutVisitor};
use salvo_syntax::diag::IntoMessage;
use salvo_syntax::known;
use salvo_syntax::messages;
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
    /// [comptime-instantiate] [decl-id] The interfaces the stamping read
    /// (the pipeline record C4 B), with the modules it changed the declarations of:
    /// what the stamped program's resolution views again.
    pub interfaces: crate::resolve::Interfaces,
}

/// [comptime-instantiate] What the world keeps of its own: every comptime fn
/// as the files declared it, since stamping removes the generic ones, and
/// each template's site names. Everything else it looks up in the program's
/// interfaces (`resolve::Interfaces`), over the declarations as written.
#[derive(Default)]
struct Tables {
    compfns: Vec<CompFnDecl>,
    /// [comptime-hygiene] Each template's site names (`Stamp::site_names`),
    /// by its file and its name's start.
    site_names: HashMap<(usize, u32), Vec<String>>,
}

/// [lsp-warm-std] Whether the per-file stamping has anything to do in a
/// module: a comptime fn, a `fn … by`, or a `by` clause. One that has none is
/// left exactly as it is, so it is not even made mutable (a shared module
/// stays shared).
pub(crate) fn stamps_anything(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Fn(f) => f.compfn.is_some() || f.by.is_some(),
        Item::Struct(s) => s.obligations.iter().any(|o| o.by.is_some()),
        Item::Type(t) => t.obligations.iter().any(|o| o.by.is_some()),
        _ => false,
    })
}

/// Runs the expansion. `modules` is aligned with `files`.
pub fn expand_comptime<M: crate::expand::ModuleSlot>(files: &[SourceFile], modules: &mut [M]) -> ComptimeOutput {
    run(files, modules, &[], &[]).0
}

/// [lsp-scope-cache] [`expand_comptime`] with the scopes of some files
/// `given` (by id, as an earlier analysis built them): those are not built.
pub fn expand_comptime_given<M: crate::expand::ModuleSlot>(files: &[SourceFile], modules: &mut [M], given: &[Option<crate::resolve::GivenScope>]) -> ComptimeOutput {
    run(files, modules, given, &[]).0
}

/// [lsp-expand-cache] [`expand_comptime_given`] keeping each stamped file's
/// result in `cache`: a file whose stamping can read nothing new since the
/// last expansion (`ExpandCache::plan`) takes its last stamped module and
/// what stamping it reported, and is not stamped again.
pub fn expand_comptime_cached(
    files: &[SourceFile],
    modules: &mut [std::sync::Arc<Module>],
    given: &[Option<crate::resolve::GivenScope>],
    cache: &mut crate::check_cache::ExpandCache,
    record: bool,
) -> ComptimeOutput {
    let reuse = cache.plan(files, modules);
    let pre: Vec<std::sync::Arc<Module>> = modules.to_vec();
    let (out, fresh) = run(files, modules, given, &reuse);
    if record {
        cache.record(files, &pre, modules, reuse, fresh, &out.interfaces.scope_ids);
    }
    out
}

/// [lsp-expand-cache] What stamping one file made and reported.
#[derive(Clone)]
pub struct StampedFile {
    pub module: std::sync::Arc<Module>,
    pub diagnostics: Vec<Diagnostic>,
    pub hovers: Vec<CompHover>,
    pub when_arms: Vec<CompWhenArms>,
    /// Whether stamping changed the module's declarations.
    pub changed: bool,
}

/// What `run` stamped of one file, but its module (set in place).
pub(crate) type FreshStamp = (usize, Vec<Diagnostic>, Vec<CompHover>, Vec<CompWhenArms>, bool);

fn run<M: crate::expand::ModuleSlot>(
    files: &[SourceFile],
    modules: &mut [M],
    given: &[Option<crate::resolve::GivenScope>],
    reuse: &[Option<StampedFile>],
) -> (ComptimeOutput, Vec<FreshStamp>) {
    let mut out = Vec::new();
    let mut hovers: Vec<CompHover> = Vec::new();
    let mut when_arms: Vec<CompWhenArms> = Vec::new();

    let mut tables = Tables::default();
    for idx in 0..files.len() {
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
                _ => {}
            }
        }
    }
    // [comptime-instantiate] [decl-id] The program's interfaces, over the
    // declarations as written: what every name the stamping reads means
    // (the pipeline record C4 B). The stamped program's resolution views them again.
    let t = crate::timing::phase("  expand: comptime interfaces");
    let mut interfaces = crate::resolve::interfaces(files, &*modules, given);
    drop(t);
    let pre: Vec<&Module> = modules.iter().map(|m| m.borrow()).collect();
    let decls = interfaces.decls.clone();
    // [comptime-hygiene] Each comptime fn's written names, once, where it is
    // written: a template says which place each fn name it can see means, and
    // the bare ones it cannot see are the site's — by template, its file and
    // its name's start.
    for idx in 0..files.len() {
        // What a file can call, for its templates: only a file holding one
        // needs it (building it for every file was most of an expansion).
        if !pre[idx].items.iter().any(|i| matches!(i, Item::Fn(f) if f.compfn.is_some())) {
            continue;
        }
        let names = callables(&interfaces.scope_ids[idx], &decls, &pre);
        for item in &pre[idx].items {
            if let Item::Fn(f) = item {
                if let Some(cf) = &f.compfn {
                    let (diags, site) = template_names(f, &names, cf.bound.is_some());
                    out.extend(diags.into_iter().map(|d| (idx, d)));
                    tables.site_names.insert((idx, f.name.span.start), site);
                }
            }
        }
    }

    let _stamp = crate::timing::phase("  expand: comptime stamps");
    let tag_files: HashMap<u32, usize> = pre
        .iter()
        .enumerate()
        .map(|(i, m)| (m.file_tag, i))
        .filter(|(tag, _)| *tag != salvo_syntax::span::NO_FILE)
        .collect();
    let world = World {
        scopes: &interfaces.scope_ids,
        decls: &decls,
        pre: &pre,
        tag_files,
        here: std::cell::Cell::new(0),
        tables,
        own: std::cell::RefCell::new(ModulePath(Vec::new())),
        module_paths: files.iter().map(|f| f.module.clone()).collect(),
    };

    // [decl-id] Each file is stamped into a copy of its module, which takes
    // the module's place once every file is stamped: until then the world
    // reads the declarations as written.
    let mut stamped: Vec<(usize, Module)> = Vec::new();
    let mut kept: Vec<(usize, std::sync::Arc<Module>)> = Vec::new();
    let mut fresh: Vec<FreshStamp> = Vec::new();
    for file_idx in 0..files.len() {
        if !stamps_anything(pre[file_idx]) {
            continue;
        }
        let file = &files[file_idx];
        // [lsp-expand-cache] What stamping this file made last time stands.
        if let Some(Some(r)) = reuse.get(file_idx) {
            out.extend(r.diagnostics.iter().cloned().map(|d| (file_idx, d)));
            hovers.extend(r.hovers.iter().cloned().map(|h| CompHover { file: file_idx, ..h }));
            when_arms.extend(r.when_arms.iter().cloned().map(|w| CompWhenArms { file: file_idx, ..w }));
            if r.changed {
                interfaces.changed.insert(file.module.clone());
            }
            kept.push((file_idx, r.module.clone()));
            continue;
        }
        let (hovers_before, arms_before) = (hovers.len(), when_arms.len());
        let mut copy: Module = pre[file_idx].clone();
        let module = &mut copy;
        *world.own.borrow_mut() = file.module.clone();
        world.here.set(file_idx);
        let mut diags: Vec<Diagnostic> = Vec::new();
        // [node-origin] The file's expansion ids: 0 is written code.
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
                let Some(group) = world.group_named(ob.group.name.name.as_str(), ob.group.name.span) else {
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
                        messages::obligation_by_group_arg_not_self(&ob.group.name.name, &by.text()),
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
                                s.site_names = world.site_names_of(template_file, &template);
                            }
                            generated.push(Item::Fn(f));
                        }
                        // One mistake, one diagnostic: the member is still
                        // declared, with a body that says nothing more, so
                        // the clause is not also reported unfulfilled
                        // [obligation-by] — as a written declaration keeps
                        // its signature.
                        Err(errs) => {
                            diags.extend(errs);
                            fallback_body(&mut f);
                            generated.push(Item::Fn(f));
                        }
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
                // [fn-by] The type stamped at is where the template's bound
                // parameter stands in its signature, matched against the
                // declaration's (user decision 2026-11-05): `fn gen() ->
                // Gen<Point> by random` against `comptime fn gen<T is
                // Struct>() -> Gen<T>` stamps at `Point`. A template with
                // the parameter first gives the first parameter's type, the
                // rule until then; that rule stays the fallback.
                let unified: Option<Type> = [CompKind::Struct, CompKind::Union].into_iter().find_map(|kind| {
                    let mut scratch = Vec::new();
                    let (tmpl, _, _) = world.pick_with_module(&f.name.name, &by, kind, &mut scratch)?;
                    let bound = tmpl.compfn.as_ref()?.bound.as_ref()?.1.name.clone();
                    if tmpl.params.len() != f.params.len() {
                        return None;
                    }
                    let found = tmpl
                        .params
                        .iter()
                        .zip(&f.params)
                        .find_map(|(a, b)| bound_match(&a.ty, &b.ty, &bound))
                        .or_else(|| match (&tmpl.return_type, &f.return_type) {
                            (Some(a), Some(b)) => bound_match(a, b, &bound),
                            _ => None,
                        })?;
                    let name = named_base(&found)?;
                    let t = world.target(&name, found.span())?;
                    (t.kind == kind).then_some(found)
                });
                let target_ty: Type = match (unified, f.params.first()) {
                    (Some(t), _) => t,
                    (None, Some(first)) => first.ty.clone(),
                    (None, None) => {
                        diags.push(Diagnostic::error(
                            messages::fn_by_no_param(&f.name.name),
                            by.span,
                        ));
                        fallback_body(f);
                        continue;
                    }
                };
                let first = &target_ty;
                // [actor-message] `fn key(m: Message<Inventory>) … by shard`:
                // a protocol's messages are a union target.
                let message_target = world.message_arms(first).map(|_| Target {
                    name: first.to_string(),
                    kind: CompKind::Union,
                    ty: first.clone(),
                    generics: Vec::new(),
                    canbe: Vec::new(),
                });
                let Some(target_name) = named_base(first) else {
                    diags.push(Diagnostic::error(
                        messages::fn_by_first_param_not_target(&f.name.name),
                        first.span(),
                    ));
                    fallback_body(f);
                    continue;
                };
                let Some(target) = message_target.or_else(|| world.target(&target_name, first.span())) else {
                    diags.push(Diagnostic::error(
                        if world.opaque_here(&target_name, first.span()) {
                            messages::fn_by_opaque_target(&f.name.name, &target_name)
                        } else {
                            messages::fn_by_target_not_struct_or_union(&f.name.name, &target_name)
                        },
                        first.span(),
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
                            s.site_names = world.site_names_of(template_file, &template);
                        }
                    }
                    Err(errs) => {
                        diags.extend(errs);
                        fallback_body(f);
                    }
                }
            } else if f.compfn.as_ref().is_some_and(|c| c.bound.is_none()) && !f.intrinsic {
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
                let (expansions, scopes, mut errs) = ex.finish();
                // Warnings ride along; only an error drops the expansion.
                diags.extend(errs.iter().filter(|d| !d.is_error()).cloned());
                errs.retain(|d| d.is_error());
                // [comptime-root-visible] Only names the expansion read as a
                // declared type are roots; the checker requires each visible.
                let roots: Vec<Ident> = roots_by_fn
                    .remove(&f.name.span.start)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|r| world.declares_type(&r.name, r.span))
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
                        site_names: Vec::new(),
                        scopes,
                    });
                } else {
                    diags.extend(errs);
                    fallback_body(f);
                }
                f.compfn = None;
            }
        }

        // Generic compfns leave the program: nothing downstream could type
        // them, and every use of one has been stamped above. So do the
        // compile-time intrinsics [comptime-eval-fns]: heads, which call
        // them, are gone.
        let before = module.items.len();
        module
            .items
            .retain(|item| !matches!(item, Item::Fn(f) if f.compfn.as_ref().is_some_and(|c| c.bound.is_some()) || (f.compfn.is_some() && f.intrinsic)));
        // [decl-id] A module that gained or lost a declaration: a scope that
        // took it in is built again over the stamped program, the rest are
        // viewed (`resolve::resolve`).
        let changed = module.items.len() != before || !generated.is_empty();
        if changed {
            interfaces.changed.insert(file.module.clone());
        }
        module.items.extend(generated);
        // [node-id] The file's tree is final.
        salvo_syntax::node_id::number(module);
        fresh.push((file_idx, diags.clone(), hovers[hovers_before..].to_vec(), when_arms[arms_before..].to_vec(), changed));
        out.extend(diags.into_iter().map(|d| (file_idx, d)));
        stamped.push((file_idx, copy));
    }
    drop(world);
    drop(pre);
    for (file_idx, module) in stamped {
        modules[file_idx].set(module);
    }
    for (file_idx, module) in kept {
        modules[file_idx].set_shared(&module);
    }
    (
        ComptimeOutput {
            diagnostics: out,
            hovers,
            when_arms,
            interfaces,
        },
        fresh,
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
                let owner = if is_arm { known::COMPTIME_ARM } else { known::COMPTIME_FIELD };
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

    fn binder(&mut self, id: &Ident, is_arm: bool) {
        let ty = if is_arm { known::COMPTIME_ARM } else { known::COMPTIME_FIELD };
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

    /// [comptime-eval] The comptime names in a head, at their written spans.
    fn cexpr(&mut self, e: &CompExpr) {
        match e {
            CompExpr::Name { id, args, span } => {
                if self.binder_kind(&id.name).is_none() && !id.name.starts_with(|c: char| c.is_lowercase()) || self.bound.as_ref().is_some_and(|(p, _)| *p == id.name) {
                    self.comp_ty(&CompTy { root: id.clone(), via_type: false, args: args.clone(), span: *span });
                }
            }
            CompExpr::Proj { base, field, span } => {
                if let CompExpr::Name { id: b, .. } = &**base {
                    if let Some(is_arm) = self.binder_kind(&b.name) {
                        let elem = if is_arm { known::COMPTIME_ARM } else { known::COMPTIME_FIELD };
                        match field.name.as_str() {
                            "type" => self.comp_ty(&CompTy { root: b.clone(), via_type: true, args: Vec::new(), span: *span }),
                            "kind" => self.push(
                                *span,
                                &format!("{}.kind: Kind", b.name),
                                "The kind of the binder's type — an arm of `core.comptime`'s `Kind`: \
                                 `Struct | Union | Tuple | FnType | Opaque` [comptime-kind].",
                            ),
                            "name" => self.push(
                                *span,
                                &format!("{}.name", b.name),
                                "The declared name — a literal of the union of the struct's field \
                                 names (or the union's arm names), tested at compile time.",
                            ),
                            p @ ("first" | "last") => self.push(*span, &format!("{}.{p}: Bool", b.name), "Whether this copy is for the first/last element."),
                            "index" => self.push(*span, &format!("{}.index: Int", b.name), "The declared position, from zero."),
                            "docs" => self.push(*span, &format!("{}.docs", b.name), "Whether there is a doc comment: `is None` when there is not [comptime-docs]."),
                            "default" => self.push(*span, &format!("{}.default", b.name), "Whether the field declares a default: `is None` when it does not [comptime-default]."),
                            _ => {
                                let _ = elem;
                            }
                        }
                        return;
                    }
                }
                match field.name.as_str() {
                    what @ ("fields" | "arms" | "literals") => {
                        self.cexpr(base);
                        let elem = match what {
                            "literals" => known::COMPTIME_LITERAL,
                            "arms" => known::COMPTIME_ARM,
                            _ => known::COMPTIME_FIELD,
                        };
                        self.push(
                            *span,
                            &format!("{}.{what}: List<{elem}>", comp_expr_text(base)),
                            &format!(
                                "The declared {what}, in declaration order; a `[for …]` over it is \
                                 unrolled, one copy of the body per `{elem}`."
                            ),
                        );
                    }
                    "mutable" => {
                        self.cexpr(base);
                        self.push(
                            *span,
                            &format!("{}.mutable: Bool", comp_expr_text(base)),
                            "Whether the type declares `canbe Mut` (`Struct.mutable`).",
                        );
                    }
                    _ => self.cexpr(base),
                }
            }
            CompExpr::Is { subject, pattern, span, .. } => {
                if let (CompExpr::Name { id, .. }, CompPattern::Type(Type::Named { base, .. })) = (&**subject, pattern) {
                    if self.binder_kind(&id.name).is_some() {
                        self.push(
                            *span,
                            &format!("{} is {}", id.name, base.name.name),
                            "Whether the field carries this comptime qualifier, written before its name \
                             [comptime-qualifier].",
                        );
                        return;
                    }
                }
                self.cexpr(subject);
            }
            CompExpr::Call { args, .. } => {
                for a in args {
                    self.cexpr(a);
                }
            }
            CompExpr::Lambda { params, body, .. } => {
                // A lambda's parameter is an element of the list it is
                // passed with, as a walk's binder is.
                for p in params {
                    self.binders.push((p.name.clone(), false));
                }
                self.cexpr(body);
                for _ in params {
                    self.binders.pop();
                }
            }
            CompExpr::Not(inner, _) | CompExpr::Neg(inner, _) => self.cexpr(inner),
            CompExpr::Binary { lhs, rhs, .. } => {
                self.cexpr(lhs);
                self.cexpr(rhs);
            }
            CompExpr::Bool(..) | CompExpr::Int { .. } | CompExpr::Char(..) | CompExpr::Str(..) | CompExpr::None(_) => {}
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
                    self.cexpr(seq);
                    let is_arm = seq_is_arm(seq);
                    self.binder(binder, is_arm);
                    self.binders.push((binder.name.clone(), is_arm));
                    self.block(body);
                    self.binders.pop();
                }
                CompStmt::Let { name, value, .. } => {
                    self.cexpr(value);
                    self.push(name.span, &format!("[let {}]", name.name), "A compile-time value for the rest of the block [comptime-eval].");
                }
                CompStmt::If {
                    cond, then, else_, ..
                } => {
                    self.cexpr(cond);
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
            Stmt::Retype { target, .. } => self.expr(target),
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
                    let owner = if is_arm { known::COMPTIME_ARM } else { known::COMPTIME_FIELD };
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
            Expr::Field { base, field, span, .. } => {
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
                            "value" => (known::COMPTIME_VALUE, "The literal member itself, as a literal in each copy [comptime-literals]."),
                            "last" => ("Bool", "Whether this copy is for the last element."),
                            "type" => (known::COMPTIME_TYPE, "The declared type itself (`core.comptime`)."),
                            "kind" => (known::COMPTIME_KIND, "The declared type's kind (`core.comptime`), tested in a comptime head."),
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
            Expr::Is { subject, .. } | Expr::Retype { subject, .. } => self.expr(subject),
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
                            self.cexpr(seq);
                            let is_arm = seq_is_arm(seq);
                            self.binder(binder, is_arm);
                            self.binders.push((binder.name.clone(), is_arm));
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

/// [comptime-eval] How a hover names a head's type expression.
fn comp_expr_text(e: &CompExpr) -> String {
    match e {
        CompExpr::Name { id, .. } => id.name.clone(),
        CompExpr::Proj { base, field, .. } => format!("{}.{}", comp_expr_text(base), field.name),
        _ => "…".to_string(),
    }
}

/// [comptime-eval] Whether a walk's sequence is of arms or literals (as
/// written: a hover's guess, before any evaluation).
fn seq_is_arm(e: &CompExpr) -> bool {
    match e {
        CompExpr::Proj { field, .. } => matches!(field.name.as_str(), "arms" | "literals"),
        CompExpr::Call { args, .. } => args.first().is_some_and(seq_is_arm),
        _ => false,
    }
}

/// The callees of a compile-time expression's calls, by span.
fn comp_callees(e: &CompExpr, out: &mut Vec<Span>) {
    match e {
        CompExpr::Call { callee, args, .. } => {
            out.push(callee.span);
            args.iter().for_each(|a| comp_callees(a, out));
        }
        CompExpr::Proj { base, .. } => comp_callees(base, out),
        CompExpr::Lambda { body, .. } => comp_callees(body, out),
        CompExpr::Not(inner, _) | CompExpr::Neg(inner, _) => comp_callees(inner, out),
        CompExpr::Binary { lhs, rhs, .. } => {
            comp_callees(lhs, out);
            comp_callees(rhs, out);
        }
        CompExpr::Is { subject, .. } => comp_callees(subject, out),
        _ => {}
    }
}

/// Every name a compile-time expression reads.
fn comp_names(e: &CompExpr, out: &mut Vec<String>) {
    match e {
        CompExpr::Name { id, .. } => out.push(id.name.clone()),
        CompExpr::Proj { base, .. } => comp_names(base, out),
        CompExpr::Call { args, .. } => args.iter().for_each(|a| comp_names(a, out)),
        CompExpr::Lambda { body, .. } => comp_names(body, out),
        CompExpr::Not(inner, _) | CompExpr::Neg(inner, _) => comp_names(inner, out),
        CompExpr::Binary { lhs, rhs, .. } => {
            comp_names(lhs, out);
            comp_names(rhs, out);
        }
        CompExpr::Is { subject, .. } => comp_names(subject, out),
        _ => {}
    }
}

/// Every pattern binder a compile-time expression declares, and every
/// lambda parameter.
fn comp_binders(e: &CompExpr, out: &mut Vec<String>) {
    match e {
        CompExpr::Is { subject, binders, .. } => {
            out.extend(binders.iter().map(|b| b.name.clone()));
            comp_binders(subject, out);
        }
        CompExpr::Lambda { params, body, .. } => {
            out.extend(params.iter().map(|p| p.name.clone()));
            comp_binders(body, out);
        }
        CompExpr::Proj { base, .. } => comp_binders(base, out),
        CompExpr::Call { args, .. } => args.iter().for_each(|a| comp_binders(a, out)),
        CompExpr::Not(inner, _) | CompExpr::Neg(inner, _) => comp_binders(inner, out),
        CompExpr::Binary { lhs, rhs, .. } => {
            comp_binders(lhs, out);
            comp_binders(rhs, out);
        }
        _ => {}
    }
}

/// The `is` tests of a compile-time expression whose subject reads one of
/// `names`, by the binders they declare.
fn comp_is_binders_over(e: &CompExpr, depends: &dyn Fn(&CompExpr) -> bool, out: &mut Vec<String>) {
    match e {
        CompExpr::Is { subject, binders, .. } if depends(subject) => out.extend(binders.iter().map(|b| b.name.clone())),
        CompExpr::Not(inner, _) => comp_is_binders_over(inner, depends, out),
        CompExpr::Binary { lhs, rhs, .. } => {
            comp_is_binders_over(lhs, depends, out);
            comp_is_binders_over(rhs, depends, out);
        }
        _ => {}
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
        fn depends_expr(&self, e: &CompExpr) -> bool {
            let mut names = Vec::new();
            comp_names(e, &mut names);
            names.iter().any(|n| *n == self.bound || self.derived.contains(n))
        }
    }
    impl Visitor for RangeCheck {
        fn visit_stmt(&mut self, s: &Stmt) {
            let Stmt::Comp(c) = s else { return };
            match c {
                CompStmt::For { binder, seq, .. } => {
                    if self.depends_expr(seq) {
                        self.derived.insert(binder.name.clone());
                    }
                }
                CompStmt::Let { name, value, .. } => {
                    if self.depends_expr(value) {
                        self.derived.insert(name.name.clone());
                    }
                }
                CompStmt::If { cond, .. } => {
                    let mut out = Vec::new();
                    comp_is_binders_over(cond, &|e| self.depends_expr(e), &mut out);
                    self.derived.extend(out);
                }
                CompStmt::WhenArms { binder, .. } => {
                    self.derived.insert(binder.name.clone());
                }
                CompStmt::WhenKind { arms, span, .. } => {
                    self.errors.extend(kind_arms_incomplete(arms).map(|m| Diagnostic::error(m, *span)));
                }
                CompStmt::WhenType { ty, arms, span, .. } if self.depends(ty) => {
                    for arm in arms {
                        self.derived.extend(arm.binders.iter().map(|b| b.name.clone()));
                    }
                    let subject = if ty.via_type { format!("{}.type", ty.root.name) } else { ty.root.name.clone() };
                    self.errors.push(Diagnostic::error(
                        messages::comptime_pattern_when_over_derived_type(&subject, &self.bound, &self.fn_name, &ty.root.name),
                        *span,
                    ));
                }
                CompStmt::WhenName { binder, span, .. } if self.derived.contains(&binder.name) => {
                    self.errors.push(Diagnostic::error(
                        messages::comptime_name_when_over_derived_name(&binder.name, &self.bound, &self.fn_name),
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
        TypeKindWord::Basic if opaque => Some(known::COMPTIME_OPAQUE.to_string()),
        TypeKindWord::Generic if opaque => None,
        k => Some(k.word().to_string()),
    })
    .collect()
}

/// [when-exhaustive] [comptime-kind] A `[when X.kind]` that leaves an arm of
/// `Kind` out — known from the arms alone, so a template is told where it is
/// declared rather than at every stamp.
fn kind_arms_incomplete(arms: &[KindArm]) -> Option<messages::Message> {
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
    Some(messages::when_exhaustive_kind_arms_missing(
        &missing.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(", "),
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

/// [fn-by] Where the template's bound parameter `name` stands in its
/// signature type `t`, the declaration's type at the same place in `d`: the
/// type the declaration stamps at. Matched structurally (same base, same
/// arity), qualifiers aside.
fn bound_match(t: &Type, d: &Type, name: &str) -> Option<Type> {
    match (t, d) {
        (Type::Named { base: tb, .. }, _) if tb.name.name == name && tb.args.is_empty() => Some(d.clone()),
        (Type::Named { base: tb, .. }, Type::Named { base: db, .. }) if tb.name.name == db.name.name && tb.args.len() == db.args.len() => {
            tb.args.iter().zip(&db.args).find_map(|(a, b)| bound_match(a, b, name))
        }
        (Type::Nullable { inner: a, .. }, Type::Nullable { inner: b, .. }) | (Type::Array { elem: a, .. }, Type::Array { elem: b, .. }) => bound_match(a, b, name),
        (Type::Tuple { elems: a, .. }, Type::Tuple { elems: b, .. }) if a.len() == b.len() => a.iter().zip(b).find_map(|(x, y)| bound_match(x, y, name)),
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

impl Target {
    /// [comptime-hygiene] Where the target is declared: what its own name
    /// is looked up by.
    fn at(&self) -> Span {
        match &self.ty {
            Type::Named { base, .. } => base.name.span,
            other => other.span(),
        }
    }
}

/// [comptime-hygiene] What the names a template's file sees as fns are: every
/// fn and effect member name, with the selectors that name each place it
/// comes from (`cmp@compare`, `println@Console`) — what a template written
/// there must choose between. A comptime fn is no place: it is stamped, not
/// called.
#[derive(Default)]
struct FileNames {
    callables: HashMap<String, Vec<String>>,
}

/// [comptime-hygiene] [decl-id] A file's [`FileNames`], read off its scope by
/// id over the program as written.
fn callables(ids: &crate::scope_ids::ScopeIds, decls: &crate::decl_id::DeclIndex, pre: &[&Module]) -> FileNames {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    let is_compfn = |id: crate::decl_id::DeclId| {
        decls.fn_decl(pre, id).is_some_and(|f| f.compfn.is_some())
    };
    for (name, entries) in &ids.fns {
        let places = out.entry(name.to_string()).or_default();
        for e in entries.iter().filter(|e| !is_compfn(e.key)) {
            let place = format!("`{name}@{}`", e.module.0.last().map(String::as_str).unwrap_or(""));
            if !places.contains(&place) {
                places.push(place);
            }
        }
    }
    for (name, members) in &ids.effect_members {
        let places = out.entry(name.to_string()).or_default();
        for (e, _) in members {
            let place = format!("`{name}@{}`", e.name());
            if !places.contains(&place) {
                places.push(place);
            }
        }
    }
    FileNames { callables: out }
}

struct World<'a> {
    /// [type-identity] [decl-id] Per file, what its names mean: its scope by
    /// id in the program's interfaces; `here` is the file being expanded.
    scopes: &'a [std::sync::Arc<crate::scope_ids::ScopeIds>],
    /// [decl-id] The program's declarations as written, and where each is.
    decls: &'a crate::decl_id::DeclIndex,
    pre: &'a [&'a Module],
    /// [comptime-hygiene] Each file by its node tag [node-file]: a name is
    /// looked up in the file that wrote it, which a copy keeps until its
    /// file is numbered again after stamping.
    tag_files: HashMap<u32, usize>,
    here: std::cell::Cell<usize>,
    tables: Tables,
    /// [struct-opaque] The module being expanded: where the stamped code
    /// reads fields from.
    own: std::cell::RefCell<ModulePath>,
    module_paths: Vec<ModulePath>,
}

impl<'a> World<'a> {
    /// [comptime-hygiene] The bare names a template writes that its own file
    /// does not see.
    fn site_names_of(&self, file: Option<usize>, template: &FnDecl) -> Vec<String> {
        let key = (file.unwrap_or(usize::MAX), template.name.span.start);
        self.tables
            .site_names
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    /// [decl-id] The item `id` is, as written.
    fn item(&self, id: crate::decl_id::DeclId) -> Option<&'a Item> {
        let loc = self.decls.loc(id)?;
        let module: &'a Module = self.pre.get(loc.file)?;
        module.items.get(loc.item)
    }

    /// [comptime-root-visible] Whether the file that wrote `at` sees a struct
    /// or a `type` named `name`.
    fn declares_type(&self, name: &str, at: Span) -> bool {
        self.names_at(at).is_some_and(|h| h.structs.contains_key(name) || h.type_aliases.contains_key(name) || h.opaque_types.contains_key(name))
    }

    /// [diag-import-suggest] Every module declaring a struct or a `type` named
    /// `name`, wherever it is visible: what an unknown name suggests importing.
    fn homes_of(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (module, ast) in self.module_paths.iter().zip(self.pre) {
            let declares = ast.items.iter().any(|item| match item {
                Item::Struct(s) => s.name.name == name,
                Item::Type(t) => t.name.name == name,
                _ => false,
            });
            if declares {
                out.push(module.to_string());
            }
        }
        out
    }

    /// Every comptime fn.
    fn compfns(&self) -> impl Iterator<Item = &CompFnDecl> {
        self.tables.compfns.iter()
    }

    /// [obligation-by] Whether the file being expanded can see `module`: its
    /// own, an imported one, or one implicitly visible.
    fn sees_module(&self, module: &ModulePath) -> bool {
        self.names_here().is_some_and(|h| h.modules.iter().any(|m| *m == module))
    }

    /// [struct-opaque] Whether `name` is an opaque struct whose fields the
    /// module being expanded may not see: declared elsewhere, and this is not
    /// its `*.test.sv` annex (module `m.test`).
    fn opaque_here(&self, name: &str, at: Span) -> bool {
        let Some((module, decl)) = self.struct_decl(name, at) else {
            return false;
        };
        let own = self.own.borrow().clone();
        if !decl.opaque || *module == own {
            return false;
        }
        let annex = own.0.len() == module.0.len() + 1
            && own.0.last().is_some_and(|l| l == "test")
            && own.0[..module.0.len()] == module.0[..];
        !annex
    }

    /// [type-identity] The names of the file being expanded.
    fn names_here(&self) -> Option<&'a crate::scope_ids::ScopeIds> {
        self.scopes.get(self.here.get()).map(|s| &**s)
    }

    /// [comptime-hygiene] [type-identity] The names of the file that wrote
    /// `at` — a template's text, a field's type where its struct is declared
    /// — or, for a span no file tagged, of the file being expanded.
    fn names_at(&self, at: Span) -> Option<&'a crate::scope_ids::ScopeIds> {
        match self.tag_files.get(&at.file) {
            Some(&f) => self.scopes.get(f).map(|s| &**s),
            None => self.names_here(),
        }
    }

    /// The type a file sees by `name` — an alias or an opaque `type` — by id.
    fn type_id(here: &crate::scope_ids::ScopeIds, name: &str) -> Option<crate::decl_id::DeclId> {
        here.type_aliases.get(name).or_else(|| here.opaque_types.get(name)).copied()
    }

    /// A `type` the file sees outranks a struct of the same name, as in the
    /// checker, which expands an alias before it reads a nominal. A name's
    /// declaration is the one the file that wrote it resolves it to, through
    /// its scope (imports, renames, rungs) — nothing else (no program-wide
    /// fallback since 2026-10-30, C3.3a).
    fn struct_decl(&self, name: &str, at: Span) -> Option<(&'static ModulePath, &'a StructDecl)> {
        let here = self.names_at(at)?;
        if Self::type_id(here, name).is_some() {
            return None;
        }
        let id = *here.structs.get(name)?;
        match self.item(id)? {
            Item::Struct(s) => Some((id.module(), s)),
            _ => None,
        }
    }

    fn struct_named(&self, name: &str, at: Span) -> Option<&'a StructDecl> {
        self.struct_decl(name, at).map(|(_, d)| d)
    }

    fn type_decl(&self, name: &str, at: Span) -> Option<(&'static ModulePath, &'a TypeDecl)> {
        let id = Self::type_id(self.names_at(at)?, name)?;
        match self.item(id)? {
            Item::Type(t) => Some((id.module(), t)),
            _ => None,
        }
    }

    fn type_named(&self, name: &str, at: Span) -> Option<&'a TypeDecl> {
        self.type_decl(name, at).map(|(_, d)| d)
    }

    /// [type-identity] The `fns` group a clause names, in its file.
    fn group_named(&self, name: &str, at: Span) -> Option<&'a FnsDecl> {
        match self.item(*self.names_at(at)?.fn_groups.get(name)?)? {
            Item::Fns(g) => Some(g),
            _ => None,
        }
    }

    /// [comptime-qualifier] The comptime qualifier `name` means where `at`
    /// is written.
    fn comptime_qual(&self, name: &str, at: Span) -> Option<&'a QualifierDecl> {
        self.qual_named(name, at).filter(|q| q.comptime)
    }

    /// [comptime-eval-fns] Whether the file that wrote `at` sees a
    /// compile-time function `name`: a `comptime intrinsic fn`, which only
    /// `std.core.comptime` declares.
    fn comptime_intrinsic(&self, name: &str, at: Span) -> bool {
        self.names_at(at).and_then(|h| h.fns.get(name)).is_some_and(|entries| {
            entries.iter().any(|e| self.decls.fn_decl(self.pre, e.key).is_some_and(|f| f.compfn.is_some() && f.intrinsic))
        })
    }

    /// Whether the file that wrote `at` sees an ordinary fn `name`.
    fn runtime_fn(&self, name: &str, at: Span) -> bool {
        self.names_at(at).and_then(|h| h.fns.get(name)).is_some_and(|entries| {
            entries.iter().any(|e| self.decls.fn_decl(self.pre, e.key).is_some_and(|f| f.compfn.is_none()))
        })
    }

    /// The qualifier `name` means where `at` is written, comptime or not:
    /// the first of the name the file sees.
    fn qual_named(&self, name: &str, at: Span) -> Option<&'a QualifierDecl> {
        match self.item(*self.names_at(at)?.qualifiers.get(name)?.first()?)? {
            Item::Qualifier(q) => Some(q),
            _ => None,
        }
    }

    /// [comptime-pattern] The declaration a named type means where it is
    /// written — its module and name — for comparing two types by what they
    /// declare rather than how they are spelled.
    fn decl_of(&self, base: &TypeRef) -> Option<(ModulePath, String)> {
        let name = base.name.name.as_str();
        let here = self.names_at(base.name.span)?;
        let id = Self::type_id(here, name).or_else(|| here.structs.get(name).copied())?;
        Some((id.module().clone(), name.to_string()))
    }

    /// [comptime-docs] The doc comment of the declaration a type names — a
    /// struct, or a `type` declaration — as text.
    fn docs_of(&self, ty: &Type) -> Option<String> {
        let Type::Named { base, .. } = ty else { return None };
        let name = base.name.name.as_str();
        match self.struct_named(name, base.name.span) {
            Some(s) => doc_text(&s.docs),
            None => self.type_named(name, base.name.span).and_then(|t| doc_text(&t.docs)),
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
    fn target(&self, name: &str, at: Span) -> Option<Target> {
        // [struct-opaque] Kind `opaque` here: not a struct a comptime fn of kind
        // `Struct` may be stamped at.
        if self.opaque_here(name, at) {
            return None;
        }
        if let Some(s) = self.struct_named(name, at) {
            return Some(Target {
                name: name.to_string(),
                kind: CompKind::Struct,
                ty: Self::simple_named(name, s.name.span),
                generics: s.generics.iter().map(|g| g.name.clone()).collect(),
                canbe: s.auto_qualifiers.iter().map(|q| q.name.name.clone()).collect(),
            });
        }
        if let Some(t) = self.type_named(name, at) {
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

    /// [actor-message] The arms of `Message<X>`, for an actor effect `X` the
    /// file that wrote the type sees: `X.Reserve`, … in send order — the
    /// structs declared beside `X` [actor-message], named where `X` is
    /// written. `None` for any other type.
    fn message_arms(&self, ty: &Type) -> Option<Vec<Type>> {
        let Type::Named { qualifiers, base } = ty else { return None };
        if !qualifiers.is_empty() || base.name.name != known::MESSAGE_TYPE || base.args.len() != 1 {
            return None;
        }
        let (module, _) = self.type_decl(known::MESSAGE_TYPE, base.name.span)?;
        if *module != ModulePath::std(&["core", "actor"]) {
            return None;
        }
        let Type::Named { qualifiers: q, base: arg } = &base.args[0] else { return None };
        if !q.is_empty() || !arg.args.is_empty() {
            return None;
        }
        let id = *self.names_at(arg.name.span)?.effects.get(arg.name.name.as_str())?;
        let Item::Effect(e) = self.item(id)? else { return None };
        if !e.is_actor || !e.generics.is_empty() {
            return None;
        }
        // The arms as the file names them: after the effect as written.
        let written = &arg.name.name;
        let arms: Vec<Type> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_send)
            .map(|(idx, _)| {
                let arm = crate::message::arm_name(e, idx);
                let tail = arm.split_once('.').map(|(_, t)| t.to_string()).unwrap_or(arm);
                Self::simple_named(&format!("{written}.{tail}"), arg.name.span)
            })
            .collect();
        (!arms.is_empty()).then_some(arms)
    }

    /// The declared arms of a union type, flattened through nullables and
    /// nested written unions (what a `T?` of a union means) but **not**
    /// through an alias, whose arms are positional over its own declaration
    /// [type-union-arm-identity].
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
                messages::obligation_by_ambiguous_by(&by.text(), module, fn_module),
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
                    messages::obligation_by_several_modules(&by.text(), modules.len(), &names.join(", ")),
                    by.span,
                ));
                return None;
            }
            let Some(module) = modules.first() else {
                diags.push(Diagnostic::error(
                    messages::obligation_by_no_module(&by.text(), &self.unseen(&path)),
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
                    messages::obligation_by_module_lacks_member(module, member, &by.text()),
                    by.span,
                ));
                return None;
            }
            candidates.retain(|c| c.compfn.as_ref().and_then(|c| c.bound.as_ref()).is_some_and(|(k, _)| *k == kind));
            return match candidates.len() {
                0 => {
                    diags.push(Diagnostic::error(
                        messages::comptime_bound_wrong_kind(module, member, kind.word()),
                        by.span,
                    ));
                    None
                }
                1 => Some(candidates[0].clone()),
                _ => {
                    diags.push(Diagnostic::error(
                        messages::comptime_bound_duplicate(module, member, kind.word()),
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
                    messages::fn_by_ambiguous_compfn(&by.text(), n, kind.word(), &places.join(", ")),
                    by.span,
                ));
                return None;
            }
            _ => {}
        }
        diags.push(Diagnostic::error(
            messages::obligation_by_names_nothing(&by.text(), member, &self.unseen(&path)),
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
                // [actor-message] `Message<X>` is the union of its arms.
                if self.message_arms(ty).is_some() {
                    return TypeKindWord::Union;
                }
                let at = base.name.span;
                if self.struct_named(name, at).is_some() {
                    // [struct-opaque] Outside its module an opaque struct is
                    // kind `opaque`: a comptime body there calls the type's
                    // own functions instead of walking fields it may not see.
                    if self.opaque_here(name, at) {
                        return TypeKindWord::Basic;
                    }
                    return TypeKindWord::Struct;
                }
                if let Some(t) = self.type_named(name, at) {
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
                if let Some(s) = self.struct_named(base.name.name.as_str(), base.name.span) {
                    return s.auto_qualifiers.iter().any(|q| q.name.name == qual);
                }
                if let Some(t) = self.type_named(base.name.name.as_str(), base.name.span) {
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
                if let Some(t) = self.type_named(base.name.name.as_str(), base.name.span) {
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
    world: &'w World<'w>,
    target: &'w Target,
    template: &'w FnDecl,
    by: &'w ByRef,
    /// [node-origin] The file's next expansion id.
    next_expn: &'w mut u32,
}

impl<'w> Stamper<'w> {
    fn new(
        world: &'w World<'w>,
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
            messages::comptime_generic_generic_target(
                &self.target.name,
                &self.template.name.name,
                &self.target.generics[0],
                &group_for(&self.template.name.name),
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
            "cmp" => known::ORDERED_GROUP,
            "eq" => known::EQ_GROUP,
            "hash" => known::HASHED_GROUP,
            "to_str" => known::TO_STR_GROUP,
            _ => return None,
        };
        let (generics, canbe, parts): (Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<Type>) =
            if let Some(s) = self.world.struct_named(&self.target.name, self.target.at()) {
                (s.generics.clone(), s.generic_canbe.clone(), s.fields.iter().filter_map(|f| f.ty.clone()).collect())
            } else if let Some(t) = self.world.type_named(&self.target.name, self.target.at()) {
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
    /// written declaration. The member's nodes are made by the stamp [node-origin]
    /// (`ExpansionKind::Made`), so a diagnostic about the signature lands at
    /// the `by` site.
    fn member_decl(self, group: &FnsDecl, member: &FnDecl) -> Result<FnDecl, Vec<Diagnostic>> {
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
        // [effect-var] A `by` clause's arguments are all `self`, so a trailing
        // effect variable of the group is left out — `[]`: the declared
        // member performs nothing.
        if let Some(effs) = &mut f.effects {
            effs.retain(|e| !matches!(e, EffectRef::Effect(r) | EffectRef::Claimed { effect: r, .. } if group.effect_generics.iter().any(|g| g.name == r.name.name)));
        }
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
        // its name's span: each gets an expansion of its own [node-origin], whose
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
            site_names: Vec::new(),
            scopes: Vec::new(),
        });
        f.span = site;
        f.name.span = name_span;
        Ok(f)
    }

    /// [fn-by] [obligation-by] Stamps the body of a declaration — a written
    /// `fn … by X`, or the one a clause declares per member
    /// ([`Stamper::member_decl`]). **The declaration's signature is the
    /// stamped fn's** (user decision 2026-10-14): its types, return type,
    /// effect list and deduction clause (written or defaulted
    /// [deduce-default]). The template supplies only the body: it is
    /// instantiated at the target, and its parameter types and return type
    /// are checked against the declaration's, positionally (a clearer error
    /// than the body's type errors would be), and its parameter names too —
    /// the declaration names its parameters as the template does (user
    /// decision 2026-10-30), so the body is the template's as written. Its
    /// own effect list and deductions do not count: the checker holds the
    /// body to the declared contract, as it holds any body to its signature.
    fn stamp_into(self, f: &mut FnDecl, declared: Declared<'_>) -> Result<(), Vec<Diagnostic>> {
        let site = f.by.as_ref().map(|b| b.span).unwrap_or(f.span);
        let mut inst = self.template.clone();
        // [node-origin] The template copied whole: every node keeps its span in
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
                messages::fn_by_param_count(
                    &what,
                    f.params.len(),
                    &self.by.text(),
                    &self.template.name.name,
                    &self.target.name,
                    inst.params.len(),
                ),
                site,
            ));
        } else {
            for (w, i) in f.params.iter().zip(&inst.params) {
                if self.world.normalized(&w.ty) != self.world.normalized(&i.ty) {
                    errs.push(Diagnostic::error(
                        messages::fn_by_param_type(&what, &w.name.name, &w.ty, &self.template.name.name, &i.ty),
                        at(w.ty.span()),
                    ));
                }
            }
            // [fn-by] The parameters are the template's, so they carry its
            // names (user decision 2026-10-30): the body is copied as written,
            // nothing in it is renamed, and nothing a declared name could
            // capture. A clause's member is named once, at the clause.
            for (w, i) in f.params.iter().zip(&inst.params) {
                if w.name.name != i.name.name {
                    errs.push(Diagnostic::error(
                        messages::fn_by_param_name(&what, &w.name.name, &self.by.text(), &self.template.name.name, &i.name.name, !written),
                        at(w.name.span),
                    ));
                    if !written {
                        break;
                    }
                }
            }
        }
        let written_ret = f.return_type.as_ref().map(|t| self.world.normalized(t)).unwrap_or_else(|| "None".into());
        let inst_ret = inst.return_type.as_ref().map(|t| self.world.normalized(t)).unwrap_or_else(|| "None".into());
        if written_ret != inst_ret {
            errs.push(Diagnostic::error(
                messages::fn_by_return_type(
                    &what,
                    &f.return_type.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "None".into()),
                    &self.template.name.name,
                    &inst.return_type.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "None".into()),
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
        let (expansions, scopes, mut errs) = ex.finish();
        // A stamp reports errors at its `by` site; a template's warnings are
        // its author's business, not every stamping site's.
        errs.retain(|d| d.is_error());
        if !errs.is_empty() {
            return Err(errs);
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
            site_names: Vec::new(),
            scopes,
        });
        Ok(())
    }
}

fn group_for(member: &str) -> &'static str {
    match member {
        "cmp" => known::ORDERED_GROUP,
        "hash" | "eq" => known::HASHED_GROUP,
        "to_str" => known::TO_STR_GROUP,
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

/// [comptime-hygiene] A comptime fn's written fn names, against what its own
/// file sees (user decision 2026-10-30). In a **template** (`bound`), a bare
/// fn name the file can see is refused: the template has to say whether it
/// means that one (`name@module`) or the stamping site's (`name@[site]`); a
/// name the file cannot see is the site's. An expanded-in-place comptime fn
/// is stamped nowhere, so it follows the ordinary rules and may not write
/// `@[site]`. A name some binding of the body has is taken as that local
/// wherever it is written — Salvo does not shadow, so this misses an error
/// only where a body names a local and a function alike.
fn template_names(f: &FnDecl, names: &FileNames, bound: bool) -> (Vec<Diagnostic>, Vec<String>) {
    use salvo_syntax::visit::{self as v, Visitor};
    struct Bindings(std::collections::HashSet<String>);
    impl Bindings {
        fn pattern(&mut self, p: &Pattern) {
            match p {
                Pattern::Ident(id) => {
                    self.0.insert(id.name.clone());
                }
                Pattern::Tuple { elems, .. } => elems.iter().for_each(|e| self.pattern(e)),
                Pattern::Struct { fields, .. } => {
                    for f in fields {
                        self.0.insert(f.binding.name.clone());
                    }
                }
            }
        }
        fn cexpr(&mut self, e: &CompExpr) {
            let mut out = Vec::new();
            comp_binders(e, &mut out);
            self.0.extend(out);
        }
    }
    impl Visitor for Bindings {
        fn visit_stmt(&mut self, stmt: &Stmt) {
            match stmt {
                Stmt::Let { pattern, .. } => self.pattern(pattern),
                Stmt::Comp(c) => match c {
                    CompStmt::For { binder, seq, .. } => {
                        self.0.insert(binder.name.clone());
                        self.cexpr(seq);
                    }
                    CompStmt::WhenArms { binder, .. } => {
                        self.0.insert(binder.name.clone());
                    }
                    CompStmt::Let { name, value, .. } => {
                        self.0.insert(name.name.clone());
                        self.cexpr(value);
                    }
                    CompStmt::If { cond, .. } => self.cexpr(cond),
                    CompStmt::WhenType { arms, .. } => {
                        for a in arms {
                            self.0.extend(a.binders.iter().map(|b| b.name.clone()));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        fn visit_expr(&mut self, expr: &Expr) {
            match expr {
                Expr::Is { binding: Some(b), .. } | Expr::WaitFor { binding: b, .. } => {
                    self.0.insert(b.name.clone());
                }
                Expr::For { pattern, .. } => self.pattern(pattern),
                Expr::Lambda { params, .. } => self.0.extend(params.iter().map(|p| p.name.name.clone())),
                Expr::When { binding, branches, .. } => {
                    self.0.extend(binding.iter().map(|b| b.name.clone()));
                    self.0.extend(branches.iter().filter_map(|b| b.binding.as_ref()).map(|b| b.name.clone()));
                }
                _ => {}
            }
        }
        fn visit_struct_lit_field(&mut self, field: &salvo_syntax::ast::StructLitField) {
            if let salvo_syntax::ast::StructLitFieldKind::InlineFor { binder, .. } = &field.kind {
                self.0.insert(binder.name.clone());
            }
        }
    }
    struct Names<'a> {
        names: &'a FileNames,
        locals: &'a std::collections::HashSet<String>,
        template: &'a str,
        bound: bool,
        out: Vec<Diagnostic>,
        site: Vec<String>,
        skip: &'a std::collections::HashSet<(u32, u32)>,
    }
    impl Names<'_> {
        fn written(&mut self, id: &Ident) {
            if self.skip.contains(&(id.span.start, id.span.end)) {
                return;
            }
            let plain = id.name.starts_with(|c: char| c.is_lowercase() || c == '_') && !id.name.contains(['.', '[']);
            if !plain || self.locals.contains(&id.name) {
                return;
            }
            let Some(places) = self.names.callables.get(&id.name).filter(|p| !p.is_empty()) else {
                if !self.site.contains(&id.name) {
                    self.site.push(id.name.clone());
                }
                return;
            };
            {
                // [dump-spelling] Three places, then how many more.
                let shown = match places.len() {
                    n if n > 3 => format!("{} or {} more", places[..3].join(", "), n - 3),
                    _ => places.join(" or "),
                };
                self.out.push(Diagnostic::error(
                    messages::comptime_hygiene_ambiguous_name(&id.name, self.template, &shown),
                    id.span,
                ));
            }
        }
    }
    impl Visitor for Names<'_> {
        fn visit_expr(&mut self, expr: &Expr) {
            match expr {
                // The dot form names a fn too: `xs.size()` is `size(xs)`.
                Expr::Call { callee, .. } if self.bound => {
                    if let Expr::Field { field, grouped: false, .. } = &**callee {
                        self.written(field);
                    }
                }
                Expr::Ident(id) if self.bound => self.written(id),
                Expr::Scoped { name, module, .. } if !self.bound => {
                    if module.len() == 1 && module[0].name == salvo_syntax::parser::SITE_SELECTOR {
                        self.out.push(Diagnostic::error(messages::comptime_hygiene_site_in_place(&name.name), module[0].span));
                    }
                }
                _ => {}
            }
        }
    }
    let mut locals = Bindings(f.params.iter().map(|p| p.name.name.clone()).collect());
    if let Some(body) = &f.body {
        v::walk_block(&mut locals, body);
    }
    // [comptime-eval] A `refuse!` message's interpolations are compile-time
    // expressions: their calls are compile-time functions, no place's.
    struct RefuseNames(std::collections::HashSet<(u32, u32)>);
    impl Visitor for RefuseNames {
        fn visit_stmt(&mut self, stmt: &Stmt) {
            if let Stmt::Comp(CompStmt::Refuse { message: Expr::Str { parts, .. }, .. }) = stmt {
                for p in parts {
                    if let StrExprPart::Interp(inner) = p {
                        if let Some(ce) = comp_of_expr(inner) {
                            let mut calls = Vec::new();
                            comp_callees(&ce, &mut calls);
                            self.0.extend(calls.iter().map(|s| (s.start, s.end)));
                        }
                    }
                }
            }
        }
    }
    let mut refused = RefuseNames(Default::default());
    if let Some(body) = &f.body {
        v::walk_block(&mut refused, body);
    }
    let mut walk = Names { names, locals: &locals.0, template: &f.name.name, bound, out: Vec::new(), site: Vec::new(), skip: &refused.0 };
    if let Some(body) = &f.body {
        v::walk_block(&mut walk, body);
    }
    (walk.out, walk.site)
}

/// [node-origin] Tags every span of a tree with a fresh expansion id: the nodes
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

/// [comptime-eval] Which sequence of a type a projection names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SeqKind {
    Fields,
    Arms,
    Literals,
}

/// What `elements_of` reads of the sequence it walks.
struct SeqSpec {
    arms: bool,
    literals: bool,
    span: Span,
}

/// [comptime-eval] A compile-time value: what a head evaluates to. The
/// model's declarations (`std.core.comptime`) are what these are, so a head
/// has the types a reader can look up.
#[derive(Clone)]
enum CVal {
    Bool(bool),
    Int(i64),
    Long(i64),
    Char(char),
    Str(String),
    None,
    /// A field, arm or literal of a walk.
    Elem(Binding),
    /// An element's `.name`: a literal of its siblings' names
    /// [comptime-name].
    Name(Binding),
    /// A type. `root` is the bound parameter or the declared type a head
    /// named (`T`, `Reading`); a binder's `.type` has none, and `label` is
    /// how a diagnostic names it.
    Type { ty: Type, root: Option<Ident>, label: String },
    Kind(TypeKindWord),
    /// A field's declared default, which a head can only test for.
    Default,
    List(Vec<CVal>),
    Lambda(Vec<Ident>, Box<CompExpr>),
}

impl CVal {
    fn describe(&self) -> &'static str {
        match self {
            CVal::Bool(_) => "a `Bool`",
            CVal::Int(_) => "an `Int`",
            CVal::Long(_) => "a `Long`",
            CVal::Char(_) => "a `Char`",
            CVal::Str(_) | CVal::Name(_) => "a `Str`",
            CVal::None => "`None`",
            CVal::Elem(b) if b.literal.is_some() => "a `Literal`",
            CVal::Elem(b) if b.is_arm => "an `Arm`",
            CVal::Elem(_) => "a `Field`",
            CVal::Type { .. } => "a `Type`",
            CVal::Kind(_) => "a `Kind`",
            CVal::Default => "a `Value`",
            CVal::List(_) => "a `List`",
            CVal::Lambda(..) => "a lambda",
        }
    }

    /// The text of a scalar, as an interpolation shows it.
    fn text(&self) -> Option<String> {
        Some(match self {
            CVal::Bool(b) => b.to_string(),
            CVal::Int(v) | CVal::Long(v) => v.to_string(),
            CVal::Char(c) => c.to_string(),
            CVal::Str(s) => s.clone(),
            CVal::Name(b) => b.name.clone(),
            CVal::None => "None".to_string(),
            _ => return None,
        })
    }
}

/// The body rewriter: unrolls, selects, refuses, and substitutes.
struct Expander<'w> {
    world: &'w World<'w>,
    /// The bound parameter and what it stands for, or `None` in a concrete
    /// compfn.
    bound: Option<(String, Target)>,
    env: Vec<(String, Binding)>,
    /// The fn's parameters, for an expanded-in-place compfn's `[when …]` over one.
    params: Vec<Param>,
    site: Span,
    /// [node-origin] The file's next expansion id.
    next_expn: &'w mut u32,
    /// [node-origin] The expansions made: the unrolled copies, and the nodes made
    /// up, in the order made (ascending ids).
    expansions: Vec<ExpansionRecord>,
    /// [node-origin] The expansion the code being expanded is in: the template's
    /// whole copy in a stamp, 0 (written) in an expanded-in-place fn, and the
    /// copy itself while a copy is expanded — the parent of a copy made now.
    current: u32,
    errors: Vec<Diagnostic>,
    /// [comptime-scope] The scopes spliced flat (`Stamp::scopes`), in the
    /// order they were opened: outermost first.
    scopes: Vec<(Span, Span)>,
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
    /// [comptime-eval] The `[let]`s and lambda parameters in scope whose
    /// value is not a field, arm or literal (those are binders, in `env`):
    /// name and value, innermost last.
    comp_locals: Vec<(String, CVal)>,
    /// [lsp-completion] Per `[when f.name]` / `[when f.type]` copy, at its
    /// written span: what that copy found uncovered (expanded-in-place fns
    /// only, where the span is the author's own).
    when_arms: Vec<(Span, Vec<String>, bool, Vec<Span>)>,
}

impl<'w> Expander<'w> {
    fn new(world: &'w World<'w>, bound: Option<(String, Target)>, site: Span, next_expn: &'w mut u32) -> Self {
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
            scopes: Vec::new(),
            in_place: false,
            fn_name: None,
            fn_generics: Vec::new(),
            type_binders: Vec::new(),
            value_binders: Vec::new(),
            value_binds: Vec::new(),
            binder_used: HashMap::new(),
            comp_locals: Vec::new(),
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

    fn finish(mut self) -> (Vec<ExpansionRecord>, Vec<(Span, Span)>, Vec<Diagnostic>) {
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
                messages::comptime_pattern_unused_binder(&name),
                at,
            ));
        }
        self.expansions.sort_by_key(|e| e.id);
        // An empty body declares nothing, and has no statement to mark.
        let scopes = self.scopes.into_iter().filter(|(first, _)| *first != Span::default()).collect();
        (self.expansions, scopes, errors)
    }

    /// The span a span in a copy was copied from [node-origin]: copies nest, so
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

    fn error(&mut self, span: Span, msg: impl IntoMessage) {
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
                    self.error(ct.span, messages::comptime_fields_not_a_binder(&ct.root.name));
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
        if self.world.declares_type(&ct.root.name, ct.root.span) {
            let mut t = World::simple_named(&ct.root.name, ct.span);
            if let Type::Named { base, .. } = &mut t {
                base.args = ct.args.clone();
            }
            return Some(t);
        }
        // [comptime-root-visible] A type the program declares where this file
        // cannot see it is an unknown type name here, as anywhere else.
        let mut homes: Vec<String> = self.world.homes_of(&ct.root.name);
        homes.sort();
        homes.dedup();
        if homes.is_empty() {
            self.error(ct.span, messages::comptime_fields_unknown_type(&ct.root.name));
        } else {
            let imports: Vec<String> = homes.iter().map(|m| format!("`import {m}.{}`", ct.root.name)).collect();
            let hint = format!(": add {}", imports.join(" or "));
            self.error(ct.root.span, messages::name_resolve_unknown_type(&"type", &ct.root.name, &hint));
        }
        None
    }

    /// The elements an `inline for` walks.
    /// [comptime-fields] [comptime-eval] The fields, arms or literals of
    /// `ty`, as the walk's elements.
    fn elements_of(&mut self, ty: Type, which: SeqKind, span: Span) -> Option<Vec<Binding>> {
        let seq = SeqSpec { arms: which == SeqKind::Arms, literals: which == SeqKind::Literals, span };
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
                self.error(seq.span, messages::comptime_literals_not_literal_union(&self.world.normalized(&ty)));
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
                self.error(seq.span, messages::comptime_fields_arms_of_non_union(&self.world.normalized(&ty), kind.word()));
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
            self.error(seq.span, messages::comptime_fields_fields_of_non_struct(&self.world.normalized(&ty), kind.word()));
            return None;
        }
        let Some(name) = named_base(&ty) else { return None };
        let Some(s) = self.world.struct_named(&name, ty.span()) else { return None };
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
                    messages::comptime_generic_fn_uninstantiated(&name, what, &wanted),
                );
            } else {
                self.error(
                    seq.span,
                    messages::comptime_generic_fn_type_arg_count(&name, params.len(), given.len()),
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
                    // [handler-state-infer] An inferred field has no written type; a
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
        if let Some(arms) = self.world.message_arms(ty) {
            return arms;
        }
        match ty {
            Type::Named { base, .. } => match self.world.type_named(base.name.name.as_str(), base.name.span).and_then(|t| t.alias.as_ref()) {
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
                self.error(span, messages::comptime_name_unknown_name(&b.sibling_owner, b.is_arm, lit));
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
                // [comptime-hygiene] A name the file that wrote the pattern
                // sees.
                let at = base.name.span;
                let declared = self.world.struct_named(name, at).is_some_and(|s| !s.comptime)
                    || self.world.type_named(name, at).is_some_and(|t| !t.comptime);
                let mut ok = true;
                if !(declared || name == "None" || generics.iter().any(|g| g == name)) {
                    if TypeKindWord::parse(name).is_some() {
                        self.error(
                            span,
                            messages::comptime_kind_kind_as_type(&name),
                        );
                    } else {
                        self.error(span, messages::comptime_inline_unknown_type(&name));
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
        let Some(decl) = self.world.comptime_qual(&qual.name.name, qual.name.span) else {
            self.error(span, messages::comptime_qualifier_not_comptime(&qual.name.name));
            return None;
        };
        let of = match &decl.of {
            Type::Named { base, .. } => base.name.name.clone(),
            _ => String::new(),
        };
        let slots = decl.value_slots.len();
        if of != subject {
            self.error(span, messages::comptime_qualifier_wrong_subject(&qual.name.name, &of, subject));
            return None;
        }
        if let Some(b) = binders.iter().find(|b| !b.name.starts_with(|c: char| c.is_lowercase())) {
            self.error(span, messages::comptime_qualifier_type_binder(&b.name));
            return None;
        }
        if !binders.is_empty() && !can_bind {
            self.error(span, messages::comptime_pattern_binder_under_or_not());
            return None;
        }
        if !qual.value_args.is_empty() && qual.value_args.len() != slots {
            self.error(span, messages::comptime_qualifier_constant_count(&qual.name.name, slots, qual.value_args.len()));
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
                        self.error(span, messages::comptime_pattern_duplicate_binder(&b.name));
                        return None;
                    }
                    return Some(Some(b.clone()));
                }
                if const_arg(pattern).is_some() {
                    return Some(None);
                }
            }
        }
        self.error(span, messages::comptime_qualifier_pattern_argument(&pattern));
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
                    if let Some(t) = self.world.type_named(base.name.name.as_str(), base.name.span) {
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
            let hides = self.world.declares_type(name, b.span)
                || self.generics().iter().any(|g| g == name)
                || self.type_binders.iter().any(|(n, ..)| n == name)
                || binders[..i].iter().any(|o| o.name == name);
            if hides {
                self.error(
                    span,
                    messages::comptime_pattern_binder_hides_name(&name),
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
                    if let Some(t) = self.world.type_named(base.name.name.as_str(), base.name.span) {
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
            // [comptime-pattern] [type-identity] Two named types match when
            // they name one declaration, each as its own file sees it (until
            // 2026-10-30 by spelling, so two modules' `Item` matched); a name
            // that declares nothing (`None`) by spelling.
            (Type::Named { base: pb, .. }, Type::Named { base: ab, .. }) => {
                let same = match (self.world.decl_of(pb), self.world.decl_of(ab)) {
                    (Some(p), Some(a)) => p == a,
                    _ => pb.name.name == ab.name.name,
                };
                same && all(self, &pb.args, &ab.args, out)
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
        let slot = self.open_scope();
        self.expand_block(&mut block);
        self.close_scope(slot, &block);
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
    /// body is one: recorded by its first and last statement
    /// (`Stamp::scopes`), so nothing it declares is visible after it (user
    /// decision 2026-10-20; the body was spliced flat before, and a `let` in
    /// it leaked into the code that followed). A terminal `[if …] ...` takes
    /// the rest of the block, which holds every use, so it reads as before.
    fn expand_scoped(&mut self, mut block: Block, out: &mut Vec<Stmt>) {
        let slot = self.open_scope();
        self.expand_block(&mut block);
        self.close_scope(slot, &block);
        out.extend(block.stmts);
    }

    /// [comptime-scope] Opens a scope the expansion is about to splice flat;
    /// [`Self::close_scope`] marks it once its statements are final.
    fn open_scope(&mut self) -> usize {
        self.scopes.push((Span::default(), Span::default()));
        self.scopes.len() - 1
    }

    fn close_scope(&mut self, slot: usize, block: &Block) {
        if let (Some(first), Some(last)) = (block.stmts.first(), block.stmts.last()) {
            self.scopes[slot] = (first.span(), last.span());
        }
    }

    /// [comptime-eval] Evaluates a head's condition: a `Bool`. `binds`
    /// collects what its patterns bind, when `can_bind`: what a test proves
    /// holds only where the test did, so under `||`, `!` and a call nothing is
    /// bound [flow-or-not] [comptime-pattern]. `None` when it was refused,
    /// already reported.
    fn eval_bool(&mut self, e: &CompExpr, binds: &mut Vec<(Ident, Type)>, can_bind: bool) -> Option<bool> {
        match self.eval(e, binds, can_bind)? {
            CVal::Bool(b) => Some(b),
            other => {
                self.error(e.span(), messages::comptime_eval_type(&"a `Bool`", &other.describe()));
                None
            }
        }
    }

    /// [comptime-eval] Evaluates a compile-time expression over the model.
    /// Evaluation is dynamic: a value of the wrong kind is reported where the
    /// stamp evaluates it, as a template's body is checked where it is
    /// stamped. Every evaluation ends: a `[for]` walks a finite list, and a
    /// head has no loops and calls only std's intrinsics.
    fn eval(&mut self, e: &CompExpr, binds: &mut Vec<(Ident, Type)>, can_bind: bool) -> Option<CVal> {
        match e {
            CompExpr::Bool(b, _) => Some(CVal::Bool(*b)),
            CompExpr::Int { value, long: false, .. } => Some(CVal::Int(*value)),
            CompExpr::Int { value, long: true, .. } => Some(CVal::Long(*value)),
            CompExpr::Char(c, _) => Some(CVal::Char(*c)),
            CompExpr::Str(t, _) => Some(CVal::Str(t.clone())),
            CompExpr::None(_) => Some(CVal::None),
            CompExpr::Name { id, args, span } => self.eval_name(id, args, *span),
            CompExpr::Proj { base, field, span } => {
                let v = self.eval(base, binds, false)?;
                let named = match &**base {
                    CompExpr::Name { id, .. } => Some(id.name.clone()),
                    _ => None,
                };
                let mut r = self.project(v, field, *span, named.as_deref())?;
                // A binder's `.type` is named after the binder.
                if let (CVal::Type { label, .. }, CompExpr::Name { id, .. }) = (&mut r, &**base) {
                    if field.name == "type" {
                        *label = format!("{}.type", id.name);
                    }
                }
                Some(r)
            }
            CompExpr::Lambda { params, body, .. } => Some(CVal::Lambda(params.clone(), body.clone())),
            CompExpr::Call { callee, args, span } => self.eval_call(callee, args, *span),
            CompExpr::Not(inner, _) => {
                let b = self.eval_bool(inner, binds, false)?;
                Some(CVal::Bool(!b))
            }
            CompExpr::Neg(inner, span) => match self.eval(inner, binds, false)? {
                CVal::Int(v) => Some(CVal::Int(-v)),
                CVal::Long(v) => Some(CVal::Long(-v)),
                other => {
                    self.error(*span, messages::comptime_eval_type(&"a number", &other.describe()));
                    None
                }
            },
            // [comptime-logic] Both sides are always evaluated: a misspelled
            // name on the side that would short-circuit away is still a
            // mistake, and is still reported.
            // [comptime-pattern] What the left side bound is in scope on the
            // right: `field.type is <S> S? && S.kind is Struct`.
            CompExpr::Binary { op: BinaryOp::And, lhs, rhs, .. } => {
                let before = binds.len();
                let x = self.eval_bool(lhs, binds, can_bind);
                // A failed test bound nothing, so the right side cannot read
                // what it would have bound: decided here.
                if x == Some(false) && expr_binds(lhs) {
                    return Some(CVal::Bool(false));
                }
                let base = self.type_binders.len();
                if x == Some(true) {
                    for (id, ty) in &binds[before..] {
                        self.type_binders.push((id.name.clone(), ty.clone(), id.span, 0));
                    }
                }
                let y = self.eval_bool(rhs, binds, can_bind);
                let lent: Vec<(String, Type, Span, usize)> = self.type_binders.drain(base..).collect();
                for (name, _, span, uses) in lent {
                    if uses > 0 {
                        let written = self.unmap(span);
                        self.binder_used.entry(((written.expn, written.start), name)).or_insert((written, false)).1 = true;
                    }
                }
                Some(CVal::Bool(x? && y?))
            }
            CompExpr::Binary { op: BinaryOp::Or, lhs, rhs, .. } => {
                let x = self.eval_bool(lhs, binds, false);
                let y = self.eval_bool(rhs, binds, false);
                Some(CVal::Bool(x? || y?))
            }
            CompExpr::Binary { op, lhs, rhs, span } => {
                let x = self.eval(lhs, binds, false);
                let y = self.eval(rhs, binds, false);
                self.binary(*op, x?, y?, *span)
            }
            CompExpr::Is { subject, binders, pattern, span } => {
                let v = self.eval(subject, binds, false)?;
                let b = self.eval_is(v, subject, binders, pattern, binds, can_bind, *span)?;
                Some(CVal::Bool(b))
            }
        }
    }

    /// [comptime-eval] A name in a head: a lambda parameter or `[let]`, a
    /// walk's binder, a qualifier pattern's constant, or a type — the bound
    /// parameter, a pattern binder, or a declared type.
    fn eval_name(&mut self, id: &Ident, args: &[Type], span: Span) -> Option<CVal> {
        if args.is_empty() {
            if let Some((_, v)) = self.comp_locals.iter().rev().find(|(n, _)| *n == id.name) {
                return Some(v.clone());
            }
            if let Some(b) = self.binding(&id.name) {
                return Some(CVal::Elem(b.clone()));
            }
            if let Some(i) = self.value_binders.iter().rposition(|(n, ..)| *n == id.name) {
                self.value_binders[i].3 += 1;
                return Some(match self.value_binders[i].1.clone() {
                    crate::types::Const::Int(v) => CVal::Int(v),
                    crate::types::Const::Char(c) => CVal::Char(c),
                    crate::types::Const::Str(t) => CVal::Str(t),
                });
            }
        }
        let is_type = id.name.starts_with(|c: char| c.is_uppercase())
            || self.bound.as_ref().is_some_and(|(p, _)| *p == id.name)
            || self.type_binders.iter().any(|(n, ..)| *n == id.name);
        if !is_type {
            self.error(span, messages::comptime_inline_not_a_binder(&id.name));
            return None;
        }
        let ct = CompTy { root: id.clone(), via_type: false, args: args.to_vec(), span };
        let ty = self.resolve_ty(&ct)?;
        Some(CVal::Type { ty, root: Some(id.clone()), label: id.name.clone() })
    }

    /// [comptime-eval] `v.field`: a projection of the model.
    fn project(&mut self, v: CVal, field: &Ident, span: Span, named: Option<&str>) -> Option<CVal> {
        let name = field.name.as_str();
        match v {
            CVal::Elem(b) => Some(match name {
                "name" => CVal::Name(b),
                "index" => CVal::Int(b.index as i64),
                "first" => CVal::Bool(b.first),
                "last" => CVal::Bool(b.last),
                "type" => CVal::Type { label: format!("{}.type", b.name), ty: b.ty.clone(), root: None },
                "kind" => CVal::Kind(self.world.classify(&b.ty, &self.generics(), 0)),
                "docs" => match &b.docs {
                    Some(t) => CVal::Str(t.clone()),
                    None => CVal::None,
                },
                "default" if b.is_arm => {
                    self.error(span, messages::comptime_default_arm_default(&named.unwrap_or(&b.name)));
                    return None;
                }
                "default" => match b.default_of {
                    Some(_) => CVal::Default,
                    None => CVal::None,
                },
                // [comptime-literals] The member itself.
                "value" => match &b.literal {
                    Some(salvo_syntax::ast::TypeLit::Str(t)) => CVal::Str(t.clone()),
                    Some(salvo_syntax::ast::TypeLit::Int(v)) => CVal::Int(*v),
                    Some(salvo_syntax::ast::TypeLit::Long(v)) => CVal::Long(*v),
                    Some(salvo_syntax::ast::TypeLit::Bool(v)) => CVal::Bool(*v),
                    None => {
                        self.error(span, messages::comptime_literals_value_outside_literals(&named.unwrap_or(&b.name)));
                        return None;
                    }
                },
                other => {
                    self.error(span, messages::comptime_fields_unknown_property(&other));
                    return None;
                }
            }),
            CVal::Type { ty, root, label } => {
                let bound_root = root.as_ref().is_some_and(|r| self.bound.as_ref().is_some_and(|(p, _)| *p == r.name));
                Some(match name {
                    "fields" | "arms" | "literals" => {
                        let which = match name {
                            "fields" => SeqKind::Fields,
                            "arms" => SeqKind::Arms,
                            _ => SeqKind::Literals,
                        };
                        let elems = self.elements_of(ty, which, span)?;
                        CVal::List(elems.into_iter().map(CVal::Elem).collect())
                    }
                    "kind" => CVal::Kind(self.world.classify(&ty, &self.generics(), 0)),
                    "mutable" if bound_root => {
                        let canbe = self.bound.as_ref().is_some_and(|(_, t)| t.canbe.iter().any(|q| q == "Mut"));
                        CVal::Bool(canbe)
                    }
                    "mutable" => CVal::Bool(self.world.canbe(&ty, "Mut")),
                    "name" if bound_root => CVal::Str(self.bound.as_ref().map(|(_, t)| t.name.clone()).unwrap_or_default()),
                    "name" if root.is_some() => CVal::Str(label),
                    "docs" if root.is_some() => {
                        let docs = if bound_root {
                            self.bound.as_ref().and_then(|(_, t)| self.world.docs_of(&t.ty))
                        } else {
                            root.as_ref().and_then(|r| self.world.docs_of(&World::simple_named(&r.name, r.span)))
                        };
                        match docs {
                            Some(t) => CVal::Str(t),
                            None => CVal::None,
                        }
                    }
                    "docs" => {
                        self.error(span, messages::comptime_docs_not_binder_or_type(&label));
                        return None;
                    }
                    other => {
                        self.error(span, messages::comptime_eval_no_property(&other, &"a `Type`"));
                        return None;
                    }
                })
            }
            other => {
                self.error(span, messages::comptime_eval_no_property(&name, &other.describe()));
                None
            }
        }
    }

    /// [comptime-eval] An operator other than `&&` and `||`.
    fn binary(&mut self, op: BinaryOp, x: CVal, y: CVal, span: Span) -> Option<CVal> {
        use std::cmp::Ordering;
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
            let same = match (&x, &y) {
                // [comptime-name] A name compared with a literal: the literal
                // must be one of the names it could be.
                (CVal::Name(b), CVal::Str(t)) | (CVal::Str(t), CVal::Name(b)) => {
                    if !self.check_names(b, std::slice::from_ref(t), span) {
                        return None;
                    }
                    b.name == *t
                }
                (CVal::Name(a), CVal::Name(b)) => a.name == b.name,
                (CVal::Str(a), CVal::Str(b)) => a == b,
                (CVal::Bool(a), CVal::Bool(b)) => a == b,
                (CVal::Int(a), CVal::Int(b)) | (CVal::Long(a), CVal::Long(b)) => a == b,
                (CVal::Char(a), CVal::Char(b)) => a == b,
                (CVal::Kind(a), CVal::Kind(b)) => a == b,
                (CVal::None, CVal::None) => true,
                (CVal::None, _) | (_, CVal::None) => false,
                _ => {
                    self.error(span, messages::comptime_eval_compare(&x.describe(), &y.describe()));
                    return None;
                }
            };
            return Some(CVal::Bool(same == (op == BinaryOp::Eq)));
        }
        if matches!(op, BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq) {
            let ord: Ordering = match (&x, &y) {
                (CVal::Int(a), CVal::Int(b)) | (CVal::Long(a), CVal::Long(b)) => a.cmp(b),
                (CVal::Char(a), CVal::Char(b)) => a.cmp(b),
                (CVal::Str(a), CVal::Str(b)) => a.cmp(b),
                _ => {
                    self.error(span, messages::comptime_eval_compare(&x.describe(), &y.describe()));
                    return None;
                }
            };
            let r = match op {
                BinaryOp::Lt => ord == Ordering::Less,
                BinaryOp::Gt => ord == Ordering::Greater,
                BinaryOp::LtEq => ord != Ordering::Greater,
                _ => ord != Ordering::Less,
            };
            return Some(CVal::Bool(r));
        }
        let long = match (&x, &y) {
            (CVal::Int(_), CVal::Int(_)) => false,
            (CVal::Long(_), CVal::Long(_)) => true,
            (CVal::Str(a), CVal::Str(b)) if op == BinaryOp::Add => return Some(CVal::Str(format!("{a}{b}"))),
            _ => {
                self.error(span, messages::comptime_eval_arith(&x.describe(), &y.describe()));
                return None;
            }
        };
        let (CVal::Int(a) | CVal::Long(a), CVal::Int(b) | CVal::Long(b)) = (x, y) else { return None };
        let v = match op {
            BinaryOp::Add => a.checked_add(b),
            BinaryOp::Sub => a.checked_sub(b),
            BinaryOp::Mul => a.checked_mul(b),
            BinaryOp::Div => a.checked_div(b),
            BinaryOp::Rem => a.checked_rem(b),
            _ => {
                self.error(span, messages::comptime_eval_arith(&"a number", &"`**`"));
                return None;
            }
        };
        let Some(v) = v.filter(|v| long || i32::try_from(*v).is_ok()) else {
            self.error(span, messages::comptime_eval_overflow());
            return None;
        };
        Some(if long { CVal::Long(v) } else { CVal::Int(v) })
    }

    /// [comptime-eval] `v is pattern`: what the test means is what `v` is.
    #[allow(clippy::too_many_arguments)]
    fn eval_is(&mut self, v: CVal, subject: &CompExpr, binders: &[Ident], pattern: &CompPattern, binds: &mut Vec<(Ident, Type)>, can_bind: bool, span: Span) -> Option<bool> {
        let target = match pattern {
            CompPattern::Strs(lits) => {
                let lits: Vec<String> = lits.iter().map(|(l, _)| l.clone()).collect();
                return match &v {
                    // [comptime-name] `binder.name` is a literal of the union of
                    // its sequence's names: a literal outside it is a mistake at
                    // the template, caught at the first instantiation.
                    CVal::Name(b) => {
                        if !self.check_names(b, &lits, span) {
                            return None;
                        }
                        Some(lits.iter().any(|l| *l == b.name))
                    }
                    CVal::Str(t) => Some(lits.iter().any(|l| l == t)),
                    other => {
                        self.error(span, messages::comptime_eval_type(&"a `Str`", &other.describe()));
                        None
                    }
                };
            }
            CompPattern::Type(t) => t,
        };
        let none_pattern = matches!(target, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() && base.name.name == "None");
        match v {
            // [comptime-kind] A kind test, against an arm of `Kind`: a program
            // type named `Union` cannot interfere.
            CVal::Kind(actual) => {
                let word = match target {
                    Type::Named { qualifiers, base } if qualifiers.is_empty() => TypeKindWord::parse(&base.name.name),
                    _ => None,
                };
                let Some(word) = word else {
                    self.error(span, messages::comptime_inline_expected_kind_arm(&format!("`{target}`")));
                    return None;
                };
                Some(kind_matches(word, actual))
            }
            // [comptime-qualifier] `field is Short`, `field is <c> Short(c)`.
            CVal::Elem(b) => {
                let name = match subject {
                    CompExpr::Name { id, .. } => id.name.clone(),
                    _ => b.name.clone(),
                };
                if b.is_arm {
                    self.error(span, messages::comptime_qualifier_on_arm(&name));
                    return None;
                }
                let qual = match target {
                    Type::Named { qualifiers, base } if qualifiers.is_empty() => base,
                    _ => {
                        self.error(span, messages::comptime_qualifier_type_test(&name));
                        return None;
                    }
                };
                self.match_annotation(&b.annotations, qual, binders, known::COMPTIME_FIELD, span, can_bind)
            }
            CVal::Type { ty: t, root, label } => {
                // [comptime-qualifier] `T is Named`, `T is <n> Named(n)`: the
                // struct's own comptime qualifiers, not a type test.
                if let Type::Named { qualifiers, base } = target {
                    if qualifiers.is_empty() && self.world.comptime_qual(&base.name.name, base.name.span).is_some() {
                        let Some(root) = root else {
                            self.error(span, messages::comptime_qualifier_on_type(&base.name.name, &label.trim_end_matches(".type")));
                            return None;
                        };
                        let annotations = match &self.bound {
                            Some((param, target)) if *param == root.name => self.world.struct_named(&target.name, target.at()).map(|s| s.annotations.clone()),
                            _ => self.world.struct_named(&root.name, root.span).map(|s| s.annotations.clone()),
                        };
                        let Some(annotations) = annotations else {
                            self.error(span, messages::comptime_qualifier_not_a_struct(&root.name));
                            return None;
                        };
                        return self.match_annotation(&annotations, base, binders, known::COMPTIME_STRUCT, span, can_bind);
                    }
                }
                self.type_test(&t, binders, target, binds, can_bind, span)
            }
            // [comptime-default] [comptime-docs] `field.default is None`,
            // `field.docs is None`.
            CVal::None if none_pattern => Some(true),
            CVal::Default | CVal::Str(_) if none_pattern => Some(false),
            other => {
                self.error(span, messages::comptime_eval_bad_is(&other.describe()));
                None
            }
        }
    }

    /// [comptime-pattern] A type tested against a pattern: its qualifiers
    /// are tested and their constants bound (`<lo, hi> InRange(lo, hi) *`),
    /// then the base, binding the parts the binders stand at.
    fn type_test(&mut self, t: &Type, binders: &[Ident], target: &Type, binds: &mut Vec<(Ident, Type)>, can_bind: bool, span: Span) -> Option<bool> {
        let (target, quals_ok) = match target {
            Type::Named { qualifiers, base } if !qualifiers.is_empty() => {
                let values = binders.iter().any(|b| b.name.starts_with(|c: char| c.is_lowercase()));
                if values && !can_bind {
                    self.error(span, messages::comptime_pattern_binder_under_or_not());
                    return None;
                }
                let ok = self.match_type_quals(qualifiers, t, binders, span)?;
                (Type::Named { qualifiers: Vec::new(), base: base.clone() }, ok)
            }
            other => (other.clone(), true),
        };
        let target = &target;
        let type_binders: Vec<Ident> = binders.iter().filter(|b| !b.name.starts_with(|c: char| c.is_lowercase())).cloned().collect();
        let binders = &type_binders;
        if !self.check_pattern(binders, target, span) {
            return None;
        }
        if !quals_ok {
            return Some(false);
        }
        if !binders.is_empty() && !can_bind {
            self.error(span, messages::comptime_pattern_binder_under_or_not());
            return None;
        }
        let names: Vec<String> = binders.iter().map(|b| b.name.clone()).collect();
        let mut got: Vec<(String, Type)> = Vec::new();
        let matched = self.match_pattern(target, t, &names, &mut got);
        if matched {
            for (name, ty) in got {
                let Some(id) = binders.iter().find(|b| b.name == name) else { continue };
                if binds.iter().any(|(b, _)| b.name == name) {
                    self.error(span, messages::comptime_pattern_duplicate_binder(&name));
                    return None;
                }
                binds.push((id.clone(), ty));
            }
        }
        Some(matched)
    }

    /// [comptime-eval-fns] A call in a head: only a compile-time function,
    /// one `std.core.comptime` declares `comptime intrinsic`, is visible
    /// there (user decision 2026-11-05: heads start limited).
    fn eval_call(&mut self, callee: &Ident, args: &[CompExpr], span: Span) -> Option<CVal> {
        let name = callee.name.as_str();
        if !self.world.comptime_intrinsic(name, callee.span) {
            if self.world.runtime_fn(name, callee.span) {
                self.error(callee.span, messages::comptime_eval_runtime_fn(&name));
            } else {
                self.error(callee.span, messages::comptime_eval_unknown_fn(&name));
            }
            return None;
        }
        let arity = match name {
            "size" => 1,
            _ => 2,
        };
        if args.len() != arity {
            self.error(span, messages::comptime_eval_arity(&name, arity, args.len()));
            return None;
        }
        let mut none = Vec::new();
        let list = match self.eval(&args[0], &mut none, false)? {
            CVal::List(xs) => xs,
            other => {
                self.error(args[0].span(), messages::comptime_eval_type(&"a `List`", &other.describe()));
                return None;
            }
        };
        if name == "size" {
            return Some(CVal::Int(list.len() as i64));
        }
        if name == "get" {
            let i = match self.eval(&args[1], &mut none, false)? {
                CVal::Int(i) => i,
                other => {
                    self.error(args[1].span(), messages::comptime_eval_type(&"an `Int`", &other.describe()));
                    return None;
                }
            };
            return Some(usize::try_from(i).ok().and_then(|i| list.get(i).cloned()).unwrap_or(CVal::None));
        }
        let (params, body) = match self.eval(&args[1], &mut none, false)? {
            CVal::Lambda(params, body) => (params, body),
            other => {
                self.error(args[1].span(), messages::comptime_eval_type(&"a lambda", &other.describe()));
                return None;
            }
        };
        if params.len() != 1 {
            self.error(args[1].span(), messages::comptime_eval_lambda_arity(&name, params.len()));
            return None;
        }
        let n = list.len();
        let mut kept = Vec::new();
        for x in list {
            let keep = self.apply(&params[0], &body, x.clone())?;
            if keep {
                kept.push(x);
            }
        }
        let total = kept.len();
        Some(match name {
            "filter" => CVal::List(kept),
            "count" => CVal::Int(total as i64),
            "any" => CVal::Bool(total > 0),
            // `all`: every element kept.
            _ => CVal::Bool(total == n),
        })
    }

    /// [comptime-eval] A lambda's body, with its parameter bound to `x`: as a
    /// binder when `x` is a field, arm or literal, so the body reads it as
    /// one (`f is Key`, `f.name`).
    fn apply(&mut self, param: &Ident, body: &CompExpr, x: CVal) -> Option<bool> {
        let mut none = Vec::new();
        let r = match x {
            CVal::Elem(b) => {
                self.env.push((param.name.clone(), b));
                let r = self.eval_bool(body, &mut none, false);
                self.env.pop();
                r
            }
            other => {
                self.comp_locals.push((param.name.clone(), other));
                let r = self.eval_bool(body, &mut none, false);
                self.comp_locals.pop();
                r
            }
        };
        r
    }

    /// [comptime-eval] A `[for … in e]`'s or an `inline for`'s sequence:
    /// fields, arms or literals.
    fn eval_seq(&mut self, e: &CompExpr) -> Option<Vec<Binding>> {
        let mut none = Vec::new();
        match self.eval(e, &mut none, false)? {
            CVal::List(xs) => {
                let mut out = Vec::with_capacity(xs.len());
                for x in xs {
                    match x {
                        CVal::Elem(b) => out.push(b),
                        other => {
                            self.error(e.span(), messages::comptime_eval_not_walkable(&other.describe()));
                            return None;
                        }
                    }
                }
                Some(out)
            }
            other => {
                self.error(e.span(), messages::comptime_eval_not_walkable(&other.describe()));
                None
            }
        }
    }

    /// One unrolled copy of `template` with `binder` bound to `b`: fresh
    /// spans, each a scope (`Stamp::scopes`) so two copies in one block do
    /// not redeclare,
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
        // [comptime-scope] The copy is a scope: two copies' `let c` are two
        // bindings, and neither is visible after its copy.
        let slot = self.open_scope();
        self.env.push((binder.to_string(), b.clone()));
        let enclosing = std::mem::replace(&mut self.current, id);
        self.expand_block(&mut block);
        self.current = enclosing;
        self.env.pop();
        self.close_scope(slot, &block);
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
        // [comptime-eval] A `[let]` holds for the rest of its block.
        let (env_len, locals_len) = (self.env.len(), self.comp_locals.len());
        for stmt in stmts {
            match stmt {
                Stmt::Comp(CompStmt::Let { span, .. }) if !walk_plain => {
                    // The block's plain statements are walked after this, by
                    // the visitor, where the `[let]` would no longer hold.
                    self.error(span, messages::comptime_eval_let_nested());
                }
                Stmt::Comp(c) => self.expand_comp(c, &mut out),
                mut other => {
                    // [comptime-eval] No shadowing: a runtime `let` may not
                    // take a `[let]`'s name.
                    if let Stmt::Let { pattern: Pattern::Ident(id), .. } = &other {
                        if self.comp_locals[locals_len..].iter().any(|(n, _)| *n == id.name) || self.env[env_len..].iter().any(|(n, _)| *n == id.name) {
                            self.error(id.span, messages::comptime_eval_let_shadow(&id.name));
                        }
                    }
                    if walk_plain {
                        visit_mut::walk_stmt(self, &mut other);
                    }
                    out.push(other);
                }
            }
        }
        if walk_plain {
            self.env.truncate(env_len);
            self.comp_locals.truncate(locals_len);
        }
        block.stmts = out;
    }

    fn expand_comp(&mut self, c: CompStmt, out: &mut Vec<Stmt>) {
        match c {
            CompStmt::Let { name, value, span } => {
                let taken = self.comp_locals.iter().any(|(n, _)| *n == name.name)
                    || self.binding(&name.name).is_some()
                    || self.params.iter().any(|p| p.name.name == name.name)
                    || self.bound.as_ref().is_some_and(|(p, _)| *p == name.name);
                if taken {
                    self.error(name.span, messages::comptime_eval_let_shadow(&name.name));
                    return;
                }
                let mut none = Vec::new();
                let Some(v) = self.eval(&value, &mut none, false) else { return };
                let _ = span;
                match v {
                    CVal::Elem(b) => self.env.push((name.name.clone(), b)),
                    other => self.comp_locals.push((name.name.clone(), other)),
                }
            }
            CompStmt::For { binder, seq, body, .. } => {
                let Some(elements) = self.eval_seq(&seq) else { return };
                for b in &elements {
                    let copy = self.copy(&body, &binder.name, b);
                    out.extend(copy.stmts);
                }
            }
            CompStmt::If { cond, then, else_, .. } => {
                let mut binds = Vec::new();
                let saved_values = std::mem::take(&mut self.value_binds);
                let taken = self.eval_bool(&cond, &mut binds, true);
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
                    messages::comptime_pattern_no_arm_matches(&subject, &t),
                );
            }
            CompStmt::WhenKind { ty, arms, span } => {
                let Some(t) = self.resolve_ty(&ty) else { return };
                let actual = self.world.classify(&t, &self.generics(), 0);
                // Exhaustive over the kinds (comptime round 3), with no `else`
                // [when-exhaustive]: a kind added to the language is an error in
                // every compfn that did not consider it.
                if let Some(m) = kind_arms_incomplete(&arms) {
                    // At the written construct, as every expansion error is
                    // (a copy's nodes are the template's [node-origin]).
                    self.error(span, m);
                    return;
                }
                let chosen = arms
                    .into_iter()
                    .find(|a| kind_matches(a.kind, actual))
                    .map(|a| a.body);
                let Some(chosen) = chosen else {
                    self.error(span, messages::comptime_inline_unhandled_kind(&self.world.normalized(&t), actual.word()));
                    return;
                };
                self.expand_scoped(chosen, out);
            }
            CompStmt::WhenName { binder, arms, span } => {
                let Some(b) = self.binding(&binder.name).cloned() else {
                    self.error(span, messages::comptime_name_not_a_binder(&binder.name));
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
                            self.error(*lspan, messages::comptime_name_covered_twice(&lit, &binder.name));
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
                        self.error(
                            span,
                            messages::comptime_name_missing_names(&binder.name, b.is_arm, &b.sibling_owner, &missing.join(", ")),
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
                    self.error(span, messages::comptime_inline_when_value_subject());
                    return;
                };
                if self.world.classify(&subject_ty, &self.generics(), 0) != TypeKindWord::Union {
                    self.error(span, messages::comptime_inline_when_value_not_union(&self.world.normalized(&subject_ty)));
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
                        self.error(span, messages::comptime_inline_unnamed_arm(&arm, &self.world.normalized(&subject_ty)));
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
                        or: Vec::new(),
                        hole: None,
                        binding: None,
                        body,
                        span: bspan,
                    });
                }
                // [actor-message] A protocol of one send: its message *is*
                // the one arm, not a union of one, so the copy is the code.
                if n == 1 && self.world.message_arms(&subject_ty).is_some() {
                    if let Some(only) = branches.pop() {
                        out.extend(only.body.stmts);
                    }
                    return;
                }
                let when_span = span.in_expansion(self.made().expn);
                out.push(Stmt::Expr(Expr::When {
                    binding: None,
                    subject: Box::new(value),
                    branches,
                    origin: None,
                    span: when_span,
                }));
            }
            CompStmt::Refuse { mut message, span } => {
                // [comptime-eval] An interpolation of a compile-time
                // expression is its value: `${size(keys)}`, `${T.name}`.
                if let Expr::Str { parts, .. } = &mut message {
                    for part in parts.iter_mut() {
                        if let StrExprPart::Interp(inner) = part {
                            if let Some(ce) = comp_of_expr(inner) {
                                let mut none = Vec::new();
                                let Some(v) = self.eval(&ce, &mut none, false) else { return };
                                match v.text() {
                                    Some(t) => *part = StrExprPart::Text(t),
                                    None => {
                                        self.error(ce.span(), messages::comptime_eval_type(&"a value with a text", &v.describe()));
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    let folded: Vec<StrExprPart> = std::mem::take(parts);
                    for p in folded {
                        match p {
                            StrExprPart::Text(t) => push_text(parts, &t),
                            other => parts.push(other),
                        }
                    }
                }
                visit_mut::walk_expr(self, &mut message);
                let text = match literal_text(&message) {
                    Some(t) => t,
                    None => {
                        self.error(span, messages::comptime_refuse_non_literal_message());
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
                    messages::comptime_refuse_refused(&at, &text),
                    where_,
                ));
            }
        }
    }

    /// [node-origin] A fresh expansion for nodes the expansion makes up (the
    /// `when` an `[when value]` unrolls into, its arms' tests): they keep the
    /// range of what they were made from, and their diagnostics land at the
    /// site.
    fn made(&mut self) -> Retag {
        let made = Retag::fresh(self.next_expn);
        self.expansions.push(ExpansionRecord { id: made.expn, parent: self.current, kind: ExpansionKind::Made });
        made
    }

    /// [comptime-hygiene] A type put in place of a splice, as its own
    /// expansion (`ExpansionKind::Splice`) that names the file it was written
    /// in: numbering the stamped file retags its nodes with that file's tag,
    /// and the expansion is how resolution still reads it where it was
    /// written (a field's type where its struct is declared).
    fn splice(&mut self, mut ty: Type) -> Type {
        let Some(&file) = self.world.tag_files.get(&ty.span().file) else { return ty };
        let mut tag = Retag::fresh(self.next_expn);
        self.expansions.push(ExpansionRecord { id: tag.expn, parent: self.current, kind: ExpansionKind::Splice { file } });
        visit_mut::walk_type(&mut tag, &mut ty);
        ty
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
                let bound = self.type_binders[i].1.clone();
                let mut new = self.splice(bound);
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
                    self.error(span, messages::comptime_fields_not_a_binder(&b));
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
        let Some(new) = replacement else { return };
        let mut new = self.splice(new);
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
        // [comptime-eval] A `[let]` reads as its value: a scalar as its
        // literal; anything else exists only in heads.
        if let Expr::Ident(id) = expr {
            if let Some((_, v)) = self.comp_locals.iter().rev().find(|(n, _)| *n == id.name).cloned() {
                let span = id.span;
                *expr = match v {
                    CVal::Bool(value) => Expr::Bool { value, span },
                    CVal::Int(value) => Expr::Int { value, long: false, span },
                    CVal::Long(value) => Expr::Int { value, long: true, span },
                    CVal::Char(value) => Expr::Char { value, span },
                    CVal::Str(t) => self.str_lit(&t, span),
                    CVal::Name(b) => self.str_lit(&b.name, span),
                    CVal::None => Expr::Ident(Ident { name: "None".to_string(), span }),
                    other => {
                        let name = id.name.clone();
                        self.error(span, messages::comptime_eval_let_in_body(&name, &other.describe()));
                        return;
                    }
                };
                return;
            }
        }
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
                            self.error(span, messages::comptime_arm_none_called());
                        }
                        Some(Type::Named { qualifiers, base }) => {
                            if qualifiers.is_empty() {
                                id.name = base.name.name.clone();
                            } else {
                                id.name = qualifiers.iter().map(|q| q.name.name.clone()).collect::<Vec<_>>().join("_");
                                // A generic claim over its subject (`Ok<T> of
                                // T`) is told the subject: `Ok<Str>(value)`.
                                let generic = qualifiers.len() == 1
                                    && self.world.qual_named(&qualifiers[0].name.name, qualifiers[0].name.span).is_some_and(|q| {
                                        q.generics.len() == 1
                                            && matches!(&q.of, Type::Named { qualifiers, base: of } if qualifiers.is_empty() && of.args.is_empty() && of.name.name == q.generics[0].name)
                                    });
                                if generic && type_args.is_empty() {
                                    type_args.push(Type::Named { qualifiers: Vec::new(), base: base.clone() });
                                }
                            }
                        }
                        Some(other) => {
                            self.error(span, messages::comptime_arm_no_constructor(&other));
                        }
                        None => {
                            self.error(span, messages::comptime_fields_not_a_binder(&b));
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
                        None => self.error(span, messages::comptime_is_splice_untestable_type(&b, &ty)),
                    },
                    None => self.error(span, messages::comptime_fields_not_a_binder(&b)),
                }
            }
        }
        match expr {
            // `v.[field]` → `v.field`; `field.name` / `field.index` /
            // `field.first` / `field.last`; `T.name`.
            Expr::Field { base, field, span, .. } => {
                if let Some(b) = field.name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    match self.binding(b) {
                        Some(bd) if !bd.is_arm => {
                            field.name = bd.name.clone();
                        }
                        Some(_) => {
                            let span = *span;
                            self.error(span, messages::comptime_access_arm_binder(&b));
                        }
                        None => {
                            let span = *span;
                            self.error(span, messages::comptime_access_not_a_binder(&b));
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
                            None => self.error(span, messages::comptime_literals_value_outside_literals(&root.name)),
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
                            self.error(span, messages::comptime_default_arm_default(&root.name));
                        }
                        "default" => {
                            *expr = match bd.default_of.clone() {
                                Some(owner) => Expr::FieldDefault {
                                    owner: self.splice(owner),
                                    field: Ident { name: bd.name.clone(), span },
                                    ty: self.splice(bd.ty.clone()),
                                    span,
                                },
                                None => Expr::Ident(Ident { name: "None".to_string(), span }),
                            };
                        }
                        "kind" => {
                            self.error(span, messages::comptime_kind_kind_as_value(&root.name));
                        }
                        other => {
                            self.error(span, messages::comptime_fields_unknown_property(&other));
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
                    && self.world.declares_type(&root.name, root.span)
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
                        self.error(span, messages::comptime_access_not_a_field_binder(&b));
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
                    let Some(elements) = self.eval_seq(&seq) else { continue };
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
fn expr_binds(c: &CompExpr) -> bool {
    match c {
        CompExpr::Is { binders, .. } => binders.iter().any(|b| !b.name.starts_with(|c: char| c.is_lowercase())),
        CompExpr::Binary { op: BinaryOp::And, lhs, rhs, .. } => expr_binds(lhs) || expr_binds(rhs),
        _ => false,
    }
}

/// [comptime-eval] An ordinary expression as a compile-time one, where it
/// is one: what an interpolation in `refuse!` evaluates.
fn comp_of_expr(e: &Expr) -> Option<CompExpr> {
    Some(match e {
        Expr::Ident(id) => CompExpr::Name { id: id.clone(), args: Vec::new(), span: id.span },
        Expr::Int { value, long, span } => CompExpr::Int { value: *value, long: *long, span: *span },
        Expr::Bool { value, span } => CompExpr::Bool(*value, *span),
        Expr::Char { value, span } => CompExpr::Char(*value, *span),
        Expr::Field { base, field, span, .. } if !field.name.starts_with('[') => {
            CompExpr::Proj { base: Box::new(comp_of_expr(base)?), field: field.clone(), span: *span }
        }
        Expr::Call { callee, args, named, type_args, span, .. } if named.is_empty() && type_args.is_empty() => {
            let Expr::Ident(id) = &**callee else { return None };
            CompExpr::Call { callee: id.clone(), args: args.iter().map(comp_of_expr).collect::<Option<Vec<_>>>()?, span: *span }
        }
        Expr::Binary { op, lhs, rhs, span } => CompExpr::Binary { op: *op, lhs: Box::new(comp_of_expr(lhs)?), rhs: Box::new(comp_of_expr(rhs)?), span: *span },
        _ => return None,
    })
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

/// [comptime-qualifier] The constant a written value argument holds.
fn const_arg(t: &Type) -> Option<crate::types::Const> {
    match t {
        Type::Named { qualifiers, base } if qualifiers.is_empty() && !base.binder => crate::types::Const::parse(&base.name.name),
        _ => None,
    }
}
