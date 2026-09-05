//! Platform-effect queries shared by the backends [platform-tree].
//!
//! A `platform effect` is implemented outside Salvo, in the target
//! language, by a file in the source root's `platform/` tree. Both backends
//! need the same three answers — which effects a module hands to the host,
//! whether its `main` needs one (and is therefore *not* the program's entry
//! point any more), and whether the host file exists — so they live here
//! rather than twice in the emitters.

use salvo_syntax::ast::{EffectDecl, EffectRef, FnDecl, Item, Module};

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

/// [platform-effect] Whether a declared effect list mentions a platform
/// effect — the condition that moves `main` out of the generated code and
/// makes it an entry point the host calls.
pub fn declares_platform_effect(f: &FnDecl, symbols: &Symbols<'_>) -> bool {
    f.effects.iter().flatten().any(|eff| match eff {
        EffectRef::Effect(r) => symbols
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
