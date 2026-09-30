//! Platform queries shared by the backends [platform-tree].
//!
//! The `platform/` tree holds host code, in the target language, for the two
//! declarations that have no Salvo body: a `platform effect`, whose whole
//! implementation is the host's [platform-effect], and a `platform handler`,
//! a host implementation of an *ordinary* Salvo effect [platform-handler].
//! Both backends need the same answers — which effects a module hands to the
//! host, which host classes it declares, whether its `main` needs a platform
//! effect (and is therefore *not* the program's entry point any more), and
//! whether the host file exists — so they live here rather than twice in the
//! emitters.

use salvo_syntax::ast::{EffectDecl, EffectRef, FnDecl, HandlerDecl, HostBlock, Item, Module};

use crate::program::Program;

use crate::program::Symbols;
use crate::source::{CompanionFile, ModulePath};

/// [platform-effect] The platform effects a module declares, in
/// declaration order: exactly the interfaces the host must implement.
pub fn platform_effects(ast: &Module) -> Vec<&EffectDecl> {
    ast.items
        .iter()
        .filter_map(|item| match item {
            Item::Effect(e) if e.platform => Some(e),
            _ => None,
        })
        .collect()
}

/// [platform-handler] The platform handlers a module declares, in
/// declaration order: the host classes the module's `platform/` companion
/// must define.
pub fn platform_handlers(ast: &Module) -> Vec<&HandlerDecl> {
    ast.items
        .iter()
        .filter_map(|item| match item {
            // [host-splice] One written in place has no companion.
            Item::Handler(h) if h.platform && !h.spliced => Some(h),
            _ => None,
        })
        .collect()
}

/// [platform-handler] [platform-tree] The message for a `use` of a platform
/// handler whose host class has nowhere to live. Like
/// [`missing_host_error`] it names the command, because the alternative is
/// the *target* compiler reporting an unresolved class in generated code.
pub fn missing_handler_host_error(
    handler: &str,
    module: &ModulePath,
    rel_path: &std::path::Path,
) -> String {
    format!(
        "error: `use {handler}` registers a platform handler, whose implementation \
         is a host class in `{}`, but that file does not exist: run `salvo platform \
         generate` to create the implementation skeleton for module `{module}`",
        rel_path.display()
    )
}

/// [platform-effect] Whether a declared effect list mentions a platform
/// effect — the condition that moves `main` out of the generated code and
/// makes it an entry point the host calls.
pub fn declares_platform_effect(f: &FnDecl, symbols: &Symbols<'_>) -> bool {
    f.effects.iter().flatten().any(|eff| match eff {
        EffectRef::Effect(r) | EffectRef::AnyEffect(r) => symbols
            .effects
            .get(r.name.name.as_str())
            .is_some_and(|e| e.platform),
        _ => false,
    })
}

/// [platform-effect] The module's `main`, if it declares one with a body
/// that needs a platform effect. `Some` means the host owns the entry
/// point: the generated `main` is renamed and the host's calls it.
pub fn platform_entry<'a>(ast: &'a Module, symbols: &Symbols<'_>) -> Option<&'a FnDecl> {
    ast.items.iter().find_map(|item| match item {
        Item::Fn(f)
            if f.name.name == "main"
                && f.body.is_some()
                && declares_platform_effect(f, symbols) =>
        {
            Some(f)
        }
        _ => None,
    })
}

/// [platform-tree] The host file for `module`, if the sources carry one.
pub fn host_file<'c>(
    companions: &'c [CompanionFile],
    module: &ModulePath,
) -> Option<&'c CompanionFile> {
    companions
        .iter()
        .find(|c| c.platform && c.module == *module)
}

/// [platform-tree] The message for a program whose host file is missing.
/// Naming the command is the whole point: without it the failure surfaces
/// as the *target* toolchain not finding an entry point, which points at
/// generated code instead of at the thing the developer has to do.
pub fn missing_host_error(module: &ModulePath, rel_path: &std::path::Path) -> String {
    format!(
        "error: module `{module}` declares a `main` that needs a platform effect, \
         so the host owns the program's entry point, but `{}` does not exist: run \
         `salvo platform generate` to create the implementation skeleton",
        rel_path.display()
    )
}

/// [platform-tree] Where a module's host file lives, relative to the source
/// root (and to the output directory, which mirrors it):
/// `app.entry` -> `platform/app/entry.<ext>`.
pub fn host_rel_path(module: &ModulePath, native_ext: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(crate::source::PLATFORM_DIR);
    for part in &module.0 {
        path.push(part);
    }
    path.set_extension(native_ext);
    path
}

/// [platform-reply] The parameters of a platform member that are
/// continuations — declared `Reply<T>` — by name. A host skeleton states the
/// contract above such a member: take the token with `hosted()`, complete it
/// exactly once, from any thread.
pub fn reply_params(f: &FnDecl) -> Vec<&str> {
    f.params
        .iter()
        .filter(|p| {
            matches!(&p.ty, salvo_syntax::ast::Type::Named { base, .. } if base.name.name == "Reply")
        })
        .map(|p| p.name.name.as_str())
        .collect()
}

/// [platform-reply] The comment a host skeleton carries above a member that
/// takes a continuation, in the host language's line-comment syntax (both
/// backends use `//`). Empty when the member takes none.
pub fn reply_contract_comment(f: &FnDecl) -> String {
    let names = reply_params(f);
    if names.is_empty() {
        return String::new();
    }
    let list = names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "    // {list}: a continuation [platform-reply]. Return at once and answer\n\
         \x20   // later: `.hosted()` hands it to host code, whose `send(value)` completes it\n\
         \x20   // exactly once, from any thread. Until then the scheduler counts the\n\
         \x20   // work as pending, so a waiter is not reported deadlocked.\n"
    )
}

/// [host-splice] The backends a project must build: its manifest's `[build]
/// backend` — `"*"` is every backend, absent is Rust, as the CLI defaults.
pub fn required_backends(project: &crate::Project) -> Vec<&'static str> {
    match project.manifest.build.backend.as_deref() {
        Some("*") => vec!["kotlin", "rust"],
        Some("kotlin") => vec!["kotlin"],
        _ => vec!["rust"],
    }
}

/// [host-splice] Every declaration written in host code has a block for each
/// backend the project builds (user decision 2026-09-30). A declaration whose
/// only block is Kotlin is allowed, as a warning — a code base reaches parity
/// before its Rust backend is enabled — and building Rust over it is then the
/// emitter's error where it is reached. A dependency's declarations are its
/// own project's concern.
pub fn host_block_coverage(program: &Program, backends: &[&str]) -> Vec<crate::FileDiagnostic> {
    let mut out = Vec::new();
    let mut check = |file: usize, what: String, span: salvo_syntax::Span, blocks: &[HostBlock]| {
        let langs: Vec<&str> = blocks.iter().map(|b| b.lang.as_str()).collect();
        let missing: Vec<&str> = backends.iter().copied().filter(|b| !langs.contains(b)).collect();
        if missing.is_empty() {
            return;
        }
        let names: Vec<String> = missing
            .iter()
            .map(|b| format!("{} (`.sv.{}`)", if *b == "kotlin" { "Kotlin" } else { "Rust" }, if *b == "kotlin" { "kt" } else { "rs" }))
            .collect();
        let msg = format!(
            "{what} has no {} implementation in its platform template, and this project builds {} [host-splice]",
            names.join(" or "),
            backends.join(" and ")
        );
        if langs == ["kotlin"] {
            out.push(crate::FileDiagnostic::warning(
                file,
                span,
                format!("{msg}: allowed while the code base reaches parity, but a Rust build that reaches it fails"),
            ));
        } else {
            out.push(crate::FileDiagnostic::error(file, span, msg));
        }
    };
    for (file, unit) in program.units().enumerate() {
        if unit.file.is_std || unit.file.dependency.is_some() {
            continue;
        }
        for item in &unit.ast.items {
            match item {
                Item::Fn(f) if !f.host.is_empty() => {
                    check(file, format!("`fn {}`", f.name.name), f.name.span, &f.host)
                }
                Item::Handler(h) if h.spliced => {
                    // Which backends implement the handler at all: any of its
                    // blocks, members or host fields.
                    let mut langs: Vec<HostBlock> = h.host.clone();
                    for f in &h.fns {
                        langs.extend(f.host.iter().cloned());
                    }
                    for fld in &h.host_fields {
                        langs.push(HostBlock { lang: fld.lang.clone(), parts: Vec::new(), span: h.name.span });
                    }
                    langs.dedup_by(|a, b| a.lang == b.lang);
                    let mut seen = std::collections::BTreeSet::new();
                    langs.retain(|b| seen.insert(b.lang.clone()));
                    check(file, format!("`platform handler {}`", h.name.name), h.name.span, &langs);
                    for f in &h.fns {
                        if !f.host.is_empty() {
                            check(file, format!("`{}.{}`", h.name.name, f.name.name), f.name.span, &f.host);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}
