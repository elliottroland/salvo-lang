//! The Rust code emitter.
//!
//! Mirrors the Kotlin emitter's architecture (checker side tables drive
//! all representation decisions), with the Rust-specific lowerings of
//! BACKEND_SPEC.rust.md: unions as generated enums [rs-union-enums],
//! `T?` as a physical `Option` [rs-option], effects as traits with
//! `&mut dyn` threading [rs-effects], and — the heart of the backend —
//! deduction-driven parameter modes: kept parameters borrow (`&`/`&mut`
//! per `Mut`), omitted parameters move [rs-borrows].
//!
//! rustc is the safety net: anything this emitter gets wrong about
//! ownership fails to *compile*, never silently misbehaves
//! [backend-never-wrong].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use salvo_core::check::{Checked, Coercion, ThrowSite, UnionTest};
use salvo_core::types::{FnId, Ty};
use salvo_core::{ModulePath, Program, Symbols};
use salvo_syntax::ast::*;
use salvo_syntax::Span;

pub struct EmittedFile {
    /// Path relative to the target dir, e.g. `core/console.rs`.
    pub rel_path: std::path::PathBuf,
    pub content: String,
}

/// Crate-root lint allowances [rs-crate]: the generator does not fight
/// cosmetic lints (every local is `let mut`, Salvo naming is snake/camel
/// mixed, defensive code may be unreachable).
const CRATE_ATTRS: &str = "#![allow(non_snake_case, non_camel_case_types, unused_mut, \
                           unused_parens, unused_imports, dead_code, unreachable_code, \
                           unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]\n";

/// Emits Rust for every *reachable* module that produces code
/// [mod-used-only], plus the generated `unions.rs` and the crate-root
/// module header [rs-crate].
pub fn emit_program(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_with_entry(program, None)
}

/// [rs-crate] As [`emit_program`], with the crate root chosen explicitly:
/// `entry` names the module whose `main` is the program's entry point
/// (`salvo run --main`). `None` keeps the historical behaviour — the first
/// emitted module that declares one.
///
/// Warnings are **dropped** here, which is the shape the golden tests want;
/// the driver calls [`emit_program_reporting`] and hands them to the user
/// [qual-refn-conflict].
pub fn emit_program_with_entry(
    program: &Program,
    entry: Option<&ModulePath>,
) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_reporting(program, entry).map(|(files, _warnings)| files)
}

/// [`emit_program_with_entry`] with the non-fatal diagnostics:
/// `(files, warnings)`, each warning rendered with its own `warning:` prefix
/// and location [diag-structured].
///
/// Two returns rather than one, because they mean different things: a
/// warning must not stop emission (a suppressed refinement conflict leaves a
/// legal program [qual-refn-conflict]), so it cannot travel as an error —
/// and it must not be silently swallowed either, or the diagnostic exists
/// only in `salvo analyze`.
pub fn emit_program_reporting(
    program: &Program,
    entry: Option<&ModulePath>,
) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    // Only *errors* stop emission: a warning reports something the author
    // probably did not intend without rejecting the program
    // [diag-structured], which is what a suppressed refinement conflict
    // needs [qual-refn-conflict]. Checker diagnostics are structured;
    // render them here at the backend boundary.
    if checked.errors.iter().any(|d| d.is_error()) {
        // Every diagnostic is rendered on the failure path, warnings
        // included: each carries its own severity prefix, so they read
        // correctly next to the errors and nothing is lost while the author
        // fixes the errors.
        return Err(checked
            .errors
            .iter()
            .map(|d| d.render(&program.files))
            .collect());
    }
    let warnings: Vec<String> = checked
        .errors
        .iter()
        .filter(|d| !d.is_error())
        .map(|d| d.render(&program.files))
        .collect();
    let reachable = salvo_core::reachable_modules(program, &resolution);
    let emitted_modules: HashSet<&ModulePath> = program
        .units()
        .filter(|u| reachable.contains(&u.file.module) && module_produces_code(u.ast))
        .map(|u| &u.file.module)
        .collect();

    // The Rust module name of every emitted Salvo module [rs-crate]:
    // path parts joined with `_` (`core.console` -> `core_console`).
    let (mod_names, mut used) = module_mod_names(&emitted_modules);

    // The module declaring `fn main` becomes the crate root [rs-crate]. The
    // driver's choice wins when it made one: with several `main`s, only it
    // knows which the user asked for, and the crate root is the one file
    // that carries the `mod` declarations.
    let declares_main = |u: &salvo_core::program::Unit| {
        emitted_modules.contains(&u.file.module)
            && u.ast.items.iter().any(
                |item| matches!(item, Item::Fn(f) if f.name.name == "main" && f.body.is_some()),
            )
    };
    let root_module: Option<&ModulePath> = program
        .units()
        .find(|u| declares_main(u) && entry.is_some_and(|e| *e == u.file.module))
        .or_else(|| program.units().find(declares_main))
        .map(|u| &u.file.module);

    let mut errors: Vec<String> = Vec::new();

    // [fn-effects] Where each effect's *trait* lives, as an absolute crate
    // path prefix: the generated pass-trait file sits at the crate root and
    // mentions traits from arbitrary modules, so it names them in full
    // rather than importing them.
    let mut effect_paths: HashMap<String, String> = HashMap::new();
    for unit in program.units() {
        if !emitted_modules.contains(&unit.file.module) {
            continue;
        }
        let prefix = if Some(&unit.file.module) == root_module {
            "crate::".to_string()
        } else {
            match mod_names.get(&unit.file.module) {
                Some(m) => format!("crate::{}::", rs_ident(m)),
                None => continue,
            }
        };
        for item in &unit.ast.items {
            if let Item::Effect(e) = item {
                effect_paths.insert(e.name.name.clone(), prefix.clone());
            }
        }
    }

    let mut files = Vec::new();
    let mut union_sizes: BTreeSet<usize> = BTreeSet::new();
    // [platform-handler] [platform-tree] Modules whose host companion a
    // `use` of a platform handler needs; checked once every file is emitted.
    let mut platform_hosts: BTreeSet<ModulePath> = BTreeSet::new();
    // [rs-mut-str] Generated once for the whole program, when anything
    // needs the string helpers a `Mut Str` mutator uses (`set`).
    let mut needs_str = false;
    // [rs-seq] And for the sequence helpers the `List` fast paths of
    // `map`/`filter`/`reduce` lower to.
    let mut needs_seq = false;
    // [rs-collections] And for the insertion-ordered `Set`/`Map`
    // [col-insertion-order], which Rust's standard library has no
    // equivalent of.
    let mut needs_collections = false;
    // [rs-actor] And for the scheduler, when a program spawns.
    let mut needs_scheduler = false;
    // [time-types] And for the two clock readings, when a program reads time.
    let mut needs_time = false;
    // [rs-effect-fusion] The fusion switch is program-wide: a fn's
    // signature cannot depend on which of its callers happens to hold a
    // fusion, so either every effect site fuses or none does.
    let fusion = program_needs_fusion(&symbols, &reachable);
    for (file_idx, unit) in program.units().enumerate() {
        if !reachable.contains(&unit.file.module) || !module_produces_code(unit.ast) {
            continue;
        }
        let generated = generated_imports(
            unit.ast,
            &unit.file.module,
            &resolution.scopes[file_idx],
            &mod_names,
            root_module,
        );
        let mut emitter = Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);
        emitter.fusion = fusion;
        emitter.generated_imports = generated;
        emitter.root_module = root_module;
        emitter.effect_paths = effect_paths.clone();
        let content = emitter.emit_module(unit.ast);
        errors.extend(emitter.errors);
        union_sizes.extend(emitter.union_sizes);
        needs_str |= emitter.needs_str;
        needs_seq |= emitter.needs_seq;
        needs_collections |= emitter.needs_collections;
        needs_scheduler |= emitter.needs_scheduler;
        needs_time |= emitter.needs_time;
        platform_hosts.extend(emitter.platform_hosts);
        let mut rel_path = std::path::PathBuf::new();
        for part in &unit.file.module.0 {
            rel_path.push(part);
        }
        rel_path.set_extension("rs");
        files.push(EmittedFile { rel_path, content });
    }
    if !union_sizes.is_empty() {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("unions.rs"),
            content: generate_unions_file(&union_sizes),
        });
    }
    // [time-timer] The scheduler's deadline thread reads the monotonic clock,
    // so the time module travels with it: a `Fired` must sit on the same
    // timeline `tick()` reports, which is what one shared reading buys.
    let needs_time = needs_time || needs_scheduler;
    if needs_str {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("strings.rs"),
            content: generate_strings_file(),
        });
    }
    if needs_seq {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("seq.rs"),
            content: generate_seq_file(),
        });
    }
    if needs_collections {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("collections.rs"),
            content: generate_collections_file(),
        });
    }
    if needs_scheduler {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("scheduler.rs"),
            content: generate_scheduler_file(),
        });
    }
    if needs_time {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("hosttime.rs"),
            content: generate_time_file(),
        });
    }
    // [backend-companion] Companion `.rs` files are copied verbatim and
    // mounted like generated modules.
    let mut companion_mods: Vec<(String, std::path::PathBuf)> = Vec::new();
    for comp in &program.companions {
        if !reachable.contains(&comp.module) {
            continue;
        }
        if files.iter().any(|f| f.rel_path == comp.rel_path) {
            errors.push(format!(
                "companion file `{}` collides with the generated file of module \
                 `{}`: a companion cannot replace a module Salvo emits",
                comp.rel_path.display(),
                comp.module
            ));
            continue;
        }
        let mut name = if comp.platform {
            // [platform-tree] A host file is mounted alongside the module
            // it implements for, so it needs a name of its own.
            host_mod_name(&comp.module)
        } else {
            comp.module
                .0
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join("_")
        };
        while !used.insert(name.clone()) {
            name.push('_');
        }
        companion_mods.push((name, comp.rel_path.clone()));
        files.push(EmittedFile {
            rel_path: comp.rel_path.clone(),
            content: comp.content.clone(),
        });
    }

    // [platform-tree] A `main` that needs a platform effect is not the
    // program's entry point any more, so the host file must exist — and
    // the error names the command that creates it, or the failure only
    // surfaces as `rustc` reporting `E0601`.
    for unit in program.units() {
        if unit.file.is_std
            || !reachable.contains(&unit.file.module)
            || salvo_core::platform_entry(unit.ast, &symbols).is_none()
        {
            continue;
        }
        if salvo_core::host_file(&program.companions, &unit.file.module).is_none() {
            errors.push(salvo_core::missing_host_error(
                &unit.file.module,
                &salvo_core::host_rel_path(&unit.file.module, "rs"),
            ));
        }
    }
    // [platform-handler] [platform-tree] A `use` of a platform handler
    // constructs a host struct, so the companion that defines it must exist —
    // in std as much as in customer code, since std ships its own
    // `platform/` files.
    for module in &platform_hosts {
        if salvo_core::host_file(&program.companions, module).is_some() {
            continue;
        }
        let handlers: Vec<&str> = program
            .units()
            .filter(|u| u.file.module == *module)
            .flat_map(|u| salvo_core::platform_handlers(u.ast))
            .map(|h| h.name.name.as_str())
            .collect();
        errors.push(salvo_core::missing_handler_host_error(
            &handlers.join("`, `"),
            module,
            &salvo_core::host_rel_path(module, "rs"),
        ));
    }

    // Crate-root assembly [rs-crate]: attributes + `#[path]` mod
    // declarations for every other emitted file, prepended to the
    // main-declaring module's file (or a synthetic `lib.rs`).
    if !files.is_empty() {
        let root_rel: std::path::PathBuf = match root_module {
            Some(module) => {
                let mut p = std::path::PathBuf::new();
                for part in &module.0 {
                    p.push(part);
                }
                p.set_extension("rs");
                p
            }
            None => std::path::PathBuf::from("lib.rs"),
        };
        let root_dir = root_rel
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let mut header = String::from(CRATE_ATTRS);
        let mut mounts: Vec<(String, std::path::PathBuf)> = Vec::new();
        if union_sizes.is_empty() {
            // no unions.rs
        } else {
            mounts.push(("unions".to_string(), std::path::PathBuf::from("unions.rs")));
        }
        if needs_str {
            mounts.push((
                "strings".to_string(),
                std::path::PathBuf::from("strings.rs"),
            ));
        }
        if needs_seq {
            mounts.push(("seq".to_string(), std::path::PathBuf::from("seq.rs")));
        }
        if needs_collections {
            mounts.push((
                "collections".to_string(),
                std::path::PathBuf::from("collections.rs"),
            ));
        }
        if needs_scheduler {
            mounts.push((
                "scheduler".to_string(),
                std::path::PathBuf::from("scheduler.rs"),
            ));
        }
        if needs_time {
            mounts.push(("hosttime".to_string(), std::path::PathBuf::from("hosttime.rs")));
        }
        for (module, name) in &mod_names {
            if Some(module) == root_module {
                continue;
            }
            let mut p = std::path::PathBuf::new();
            for part in &module.0 {
                p.push(part);
            }
            p.set_extension("rs");
            mounts.push((name.clone(), p));
        }
        mounts.extend(companion_mods);
        for (name, path) in mounts {
            let rel = relative_path(&root_dir, &path);
            header.push_str(&format!(
                "#[path = \"{}\"]\npub mod {};\n",
                rel,
                rs_ident(&name)
            ));
        }
        header.push('\n');
        // [platform-tree] [rs-platform-host] Rust wants `fn main` in the
        // crate root, but the host's `main` lives in a mounted module — so
        // the crate root gets a one-line delegation to it. The generated
        // entry point next to it is `salvo_main`, which the host calls with
        // the implementations it constructed.
        let mut footer = String::new();
        if let Some(root) = root_module {
            let needs_host = program
                .units()
                .find(|u| u.file.module == *root)
                .and_then(|u| salvo_core::platform_entry(u.ast, &symbols))
                .is_some();
            if needs_host && salvo_core::host_file(&program.companions, root).is_some() {
                footer = format!(
                    "\nfn main() {{\n    crate::{}::main()\n}}\n",
                    rs_ident(&host_mod_name(root))
                );
            }
        }
        match files.iter_mut().find(|f| f.rel_path == root_rel) {
            Some(root_file) => {
                root_file.content = format!("{header}{}{footer}", root_file.content);
            }
            None => {
                // Library compile: a synthetic lib.rs mounts everything.
                files.push(EmittedFile {
                    rel_path: root_rel,
                    content: format!("{header}{footer}"),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok((files, warnings))
    } else {
        Err(errors)
    }
}

/// [platform-tree] The Rust type name a host implementation gets for effect
/// `E`: `EHost`. A named unit struct, not an anonymous one, because the file
/// is the customer's from the moment it is written — they need something to
/// hang state on.
fn host_struct(effect: &str) -> String {
    format!("{}Host", rs_ident(effect))
}

/// [platform-tree] [rs-platform-host] Renders the host implementation
/// skeleton for every module that declares platform effects, plus the module
/// whose `main` needs one (which is where the program's real entry point
/// goes). This is `salvo platform generate`'s whole output.
///
/// Every reference is written out in full (`crate::…`) rather than imported:
/// the host is a module mounted from the crate root [rs-crate], and a
/// qualified path is the one spelling that stays correct wherever the
/// mounting puts it.
pub fn platform_skeletons(
    program: &Program,
    entry: Option<&ModulePath>,
) -> Result<Vec<EmittedFile>, Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    // Only errors stop the skeleton renderer, and warnings are *not*
    // surfaced here: `salvo platform generate` writes host stubs once, and
    // nagging about the program's diagnostics is the compile path's job
    // (`emit_program_reporting`) and `salvo analyze`'s
    // [qual-refn-conflict].
    if checked.errors.iter().any(|d| d.is_error()) {
        return Err(checked
            .errors
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.render(&program.files))
            .collect());
    }
    let reachable = salvo_core::reachable_modules(program, &resolution);
    let emitted_modules: HashSet<&ModulePath> = program
        .units()
        .filter(|u| reachable.contains(&u.file.module) && module_produces_code(u.ast))
        .map(|u| &u.file.module)
        .collect();
    let (mod_names, _) = module_mod_names(&emitted_modules);
    let declares_main = |u: &salvo_core::program::Unit| {
        emitted_modules.contains(&u.file.module)
            && u.ast.items.iter().any(
                |item| matches!(item, Item::Fn(f) if f.name.name == "main" && f.body.is_some()),
            )
    };
    let root_module: Option<&ModulePath> = program
        .units()
        .find(|u| declares_main(u) && entry.is_some_and(|e| *e == u.file.module))
        .or_else(|| program.units().find(declares_main))
        .map(|u| &u.file.module);

    // How the host addresses an item of `module`: the crate root's items are
    // at `crate::`, everything else sits under its mount.
    let path_to = |module: &ModulePath| -> Option<String> {
        if Some(module) == root_module {
            Some("crate".to_string())
        } else {
            mod_names
                .get(module)
                .map(|name| format!("crate::{}", rs_ident(name)))
        }
    };
    let mut effect_module: HashMap<&str, &ModulePath> = HashMap::new();
    for unit in program.units() {
        for e in salvo_core::platform_effects(unit.ast) {
            effect_module.insert(e.name.name.as_str(), &unit.file.module);
        }
    }
    // [platform-handler] And every effect, platform or not: a platform
    // handler implements an *ordinary* effect, whose trait may live in
    // another module (std's, typically).
    let mut all_effect_module: HashMap<&str, &ModulePath> = HashMap::new();
    for unit in program.units() {
        for item in &unit.ast.items {
            if let Item::Effect(e) = item {
                all_effect_module.insert(e.name.name.as_str(), &unit.file.module);
            }
        }
    }

    let mut files = Vec::new();
    let mut errors = Vec::new();
    for (file_idx, unit) in program.units().enumerate() {
        if unit.file.is_std {
            continue;
        }
        let effects = salvo_core::platform_effects(unit.ast);
        let handlers = salvo_core::platform_handlers(unit.ast);
        let entry_fn = salvo_core::platform_entry(unit.ast, &symbols);
        if effects.is_empty() && handlers.is_empty() && entry_fn.is_none() {
            continue;
        }
        let module = &unit.file.module;
        let Some(own_path) = path_to(module) else {
            errors.push(format!(
                "{}: module `{module}` declares platform effects but emits no Rust \
                 module to attach them to",
                unit.file.name
            ));
            continue;
        };
        let mut emitter = Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);
        let mut body = String::new();
        // The modules whose items the host file has to see beyond its own
        // (an effect declared elsewhere — std's, typically).
        let mut impl_paths: BTreeSet<String> = BTreeSet::new();
        for e in &effects {
            body.push_str(&emitter.host_impl(e, &own_path));
        }
        // [platform-handler] One struct per platform handler, named after the
        // handler itself: the `use` site constructs *this* struct through
        // `::new(…)`, so neither the name nor the constructor is the host's
        // to choose. The trait it implements is the ordinary effect's, which
        // may live in another module (std's, typically).
        for h in &handlers {
            // [platform-handler] [effect-handler-multi] One face: the host
            // writes one class implementing one generated interface, and the
            // checker refuses a second.
            let effect_path = type_base_name(&h.of[0])
                .and_then(|name| all_effect_module.get(name))
                .and_then(|m| path_to(m));
            match effect_path {
                Some(path) => {
                    body.push_str(&emitter.host_handler_impl(h, &path));
                    if path != own_path {
                        impl_paths.insert(path);
                    }
                }
                None => errors.push(format!(
                    "{}: `platform handler {}` implements an effect whose module \
                     emits no Rust module to attach it to",
                    unit.file.name, h.name.name
                )),
            }
        }
        if let Some(f) = entry_fn {
            let mut args = Vec::new();
            for effect in emitter.platform_entry_effects(f) {
                let Some(other) = effect_module.get(effect.as_str()).copied() else {
                    errors.push(format!(
                        "{}: `main` needs the platform effect `{effect}`, whose \
                         declaration could not be located",
                        unit.file.name
                    ));
                    continue;
                };
                let owner = if other == module {
                    String::new()
                } else {
                    match path_to(other) {
                        // A host struct lives in the *other module's* host
                        // file, which is mounted under its own name.
                        Some(_) => format!("crate::{}::", rs_ident(&host_mod_name(other))),
                        None => continue,
                    }
                };
                args.push(format!("&mut {owner}{}", host_struct(&effect)));
            }
            body.push_str(&format!(
                "\n// The program's entry point [rs-platform-host]: Salvo's `main` \
                 needs a\n// platform effect, so it is emitted as `{SALVO_ENTRY}` and \
                 the crate root\n// calls this.\npub fn main() {{\n    {own_path}::\
                 {SALVO_ENTRY}({})\n}}\n",
                args.join(", ")
            ));
        }
        errors.extend(std::mem::take(&mut emitter.errors));

        // [rs-platform-host] The host file is a module of the *same crate*,
        // so every name its signatures mention has to be in scope there: the
        // declaring module's own items (the structs and type aliases the
        // members take), the union wrappers a fallible member's result
        // lowers to, and the ordered collections when one appears. Without
        // them the skeleton does not compile — which was invisible until a
        // `platform handler` whose members trade in more than primitives
        // arrived (std's `HostRawFs`, phase 4).
        let mut uses: BTreeSet<String> = BTreeSet::new();
        uses.insert(format!("use {own_path}::*;"));
        for path in &impl_paths {
            uses.insert(format!("use {path}::*;"));
        }
        if !emitter.union_sizes.is_empty() {
            uses.insert("use crate::unions::*;".to_string());
        }
        if emitter.needs_collections {
            uses.insert("use crate::collections::*;".to_string());
        }
        let preamble = if uses.is_empty() {
            String::new()
        } else {
            format!(
                "\n{}\n",
                uses.into_iter().collect::<Vec<_>>().join("\n")
            )
        };

        let content = format!(
            "// Host implementation of the platform declarations of Salvo module \
             `{module}`.\n//\n// Generated once by `salvo platform generate`; the \
             compiler never writes\n// this file again — it is yours. Nothing here is \
             checked by Salvo: rustc\n// checks it, against the traits the backend \
             generates from the\n// `platform effect` and `platform handler` \
             declarations.\n{preamble}{body}"
        );
        files.push(EmittedFile {
            rel_path: salvo_core::host_rel_path(module, "rs"),
            content,
        });
    }
    if errors.is_empty() {
        Ok(files)
    } else {
        Err(errors)
    }
}

/// [rs-crate] The Rust module name of every emitted Salvo module: the path
/// parts joined with `_` (`core.console` -> `core_console`), made unique by
/// suffixing. Returns the names taken so far as well, so later mounts
/// (companions, platform hosts) keep clear of them.
///
/// One function rather than two because `platform_skeletons` has to address
/// exactly the modules `emit_program` mounts: a skeleton naming
/// `crate::core_console::…` where the crate calls it something else would
/// be generated code that does not compile.
fn module_mod_names(
    emitted_modules: &HashSet<&ModulePath>,
) -> (BTreeMap<ModulePath, String>, HashSet<String>) {
    let mut mod_names: BTreeMap<ModulePath, String> = BTreeMap::new();
    let mut used: HashSet<String> = HashSet::new();
    used.insert("unions".to_string());
    for module in emitted_modules.iter().copied().collect::<BTreeSet<_>>() {
        let mut name = module
            .0
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("_");
        while !used.insert(name.clone()) {
            name.push('_');
        }
        mod_names.insert(module.clone(), name);
    }
    (mod_names, used)
}

/// [platform-tree] The Rust module name a module's host file is mounted
/// under: the module's own name with a `platform_` prefix, so a host and
/// the module it implements for can never collide.
fn host_mod_name(module: &ModulePath) -> String {
    format!(
        "platform_{}",
        module
            .0
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("_")
    )
}

/// The path of `target` relative to the directory `from` (both relative to
/// the output root): `#[path]` attributes resolve relative to the file
/// that contains them [rs-crate].
fn relative_path(from_dir: &std::path::Path, target: &std::path::Path) -> String {
    let ups = from_dir.components().count();
    let mut out = String::new();
    for _ in 0..ups {
        out.push_str("../");
    }
    let target = target
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");
    out.push_str(&target);
    out
}

/// The generated `use` items of one file [rs-imports]: a glob per foreign
/// emitted module whose names the file uses, plus alias imports for
/// aliased Salvo imports of Rust-visible items.
fn generated_imports(
    ast: &Module,
    own: &ModulePath,
    scope: &salvo_core::ModuleScope<'_>,
    mod_names: &BTreeMap<ModulePath, String>,
    root_module: Option<&ModulePath>,
) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    let module_use = |module: &ModulePath| -> Option<String> {
        if Some(module) == root_module {
            // Root-module items live at the crate root.
            Some("use crate::*;".to_string())
        } else {
            mod_names
                .get(module)
                .map(|m| format!("use crate::{}::*;", rs_ident(m)))
        }
    };
    for name in salvo_core::reach::used_names(ast) {
        for module in scope.name_origins.get(name).into_iter().flatten() {
            if *module != own {
                if let Some(import) = module_use(module) {
                    imports.insert(import);
                }
            }
        }
    }
    // [iter-protocol] The driving loop and the advance adapter are
    // *synthesized*, and they name the protocol's own types (`Finished`) which
    // the source need never mention — `for x in pass` is the whole of it. The
    // module declaring them is cheap to add unconditionally here: a glob import
    // of a module the file does not otherwise use is harmless under the crate's
    // `unused_imports` allowance, and the alternative is a second pass over the
    // emitted text.
    for module in scope.name_origins.get("Finished").into_iter().flatten() {
        if *module != own {
            if let Some(import) = module_use(module) {
                imports.insert(import);
            }
        }
    }
    for item in &ast.items {
        let Item::Import(imp) = item else { continue };
        let (Some(alias), Some(item_name)) = (&imp.alias, imp.path.last()) else {
            continue;
        };
        let alias_name = alias.name.as_str();
        // Only items with a Rust symbol can be alias-imported: fns with
        // bodies, structs, effects, handlers.
        let has_symbol = scope
            .fns
            .get(alias_name)
            .is_some_and(|entries| entries.iter().any(|e| e.decl.body.is_some()))
            || scope.structs.contains_key(alias_name)
            || scope.effects.contains_key(alias_name)
            || scope.handlers.contains_key(alias_name);
        if !has_symbol {
            continue;
        }
        let Some(module) = scope.name_origins.get(alias_name).and_then(|ms| ms.first()) else {
            continue;
        };
        let prefix = if Some(*module) == root_module {
            "crate".to_string()
        } else {
            match mod_names.get(*module) {
                Some(m) => format!("crate::{}", rs_ident(m)),
                None => continue,
            }
        };
        imports.insert(format!(
            "use {prefix}::{} as {};",
            rs_ident(&item_name.name),
            rs_ident(alias_name)
        ));
    }
    imports
}

/// [rs-mut-str] String helpers for the `Mut Str` mutators std declares that
/// Rust has no single method for.
///
/// A *trait* rather than free functions, so a call site is method syntax:
/// `set`'s receiver is both read and written, and an inline
/// `let s: &mut String = &mut place;` does not work for a `&mut String`
/// *parameter* (E0596 — the binding is not `mut`). Method syntax auto-refs
/// an owned local and re-borrows a reference alike, and mentions the
/// receiver once, so a call argument is never evaluated twice.
/// Source in `runtime/strings.rs`, included verbatim and compiled
/// directly by `runtime_tests.rs`.
fn generate_strings_file() -> String {
    include_str!("../runtime/strings.rs").to_string()
}

/// [rs-seq] The sequence helpers the `List` fast paths of
/// `map`/`filter`/`reduce` lower to.
///
/// Functions rather than inline expressions for one reason: a Rust closure
/// bound to a `let` cannot infer its parameter types, and neither can one
/// nested inside another closure's argument, so *every* inline shape needed
/// an annotation the emitter does not have. A generic parameter is an
/// expected type, which is exactly what closure inference wants — and it
/// also pins the callback's convention (`FnMut(&T)`), which is what a
/// fn-typed parameter of declared type `(T) -> U` renders as
/// [rs-fn-param-convention].
///
/// They take `&[T]`, so a call splices its receiver as `&place[..]` and
/// works for an owned `Vec`, a `&Vec` and a `&mut Vec` alike — and nothing
/// is cloned but the elements a `filter` keeps.
/// Source in `runtime/seq.rs`, included verbatim and compiled directly by
/// `runtime_tests.rs`.
fn generate_seq_file() -> String {
    include_str!("../runtime/seq.rs").to_string()
}

/// [rs-collections] The insertion-ordered `Set`/`Map` the collection
/// intrinsics lower to.
///
/// Rust's own `HashMap`/`HashSet` cannot serve: Salvo's collections iterate
/// in **insertion order** on every backend [col-insertion-order], which
/// Kotlin gets free from `LinkedHashMap`/`LinkedHashSet` and Rust's standard
/// library has no equivalent of. The file supplies that equivalent, with
/// LinkedHashMap's exact observable semantics — a position-preserving
/// overwrite, an order-preserving O(1) removal, and order-insensitive
/// equality.
/// Source in `runtime/collections.rs`, included verbatim and compiled
/// directly by `runtime_tests.rs`.
fn generate_collections_file() -> String {
    include_str!("../runtime/collections.rs").to_string()
}

/// [rs-actor] The actor scheduler asynchronous effect handlers run on:
/// run-to-completion activations on pool threads, per-actor bounded queues,
/// reply tokens, the gate, death by faulted activation, and the
/// idle-with-parked-gates report. A **library**, not a runtime baked into
/// generated code (user decision 2026-09-15) — emitted only when a program
/// spawns. Source in `runtime/scheduler.rs`, included verbatim and compiled
/// directly by `runtime_tests.rs`.
fn generate_scheduler_file() -> String {
    include_str!("../runtime/scheduler.rs").to_string()
}

/// [time-types] [rs-time] The two clock readings `time`'s effects are built
/// on: the monotonic one (with the process-wide origin that makes two
/// readings comparable) and the wall-clock one. Emitted only when a program
/// reads time. Source in `runtime/hosttime.rs`.
fn generate_time_file() -> String {
    include_str!("../runtime/hosttime.rs").to_string()
}
fn generate_unions_file(sizes: &BTreeSet<usize>) -> String {
    let mut out = String::from("// Generated by the Salvo compiler: enums for union types.\n");
    for &n in sizes {
        let params: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
        let params = params.join(", ");
        // [col-equality] `PartialEq` so a struct holding a union can derive
        // its own: the derive is *conditional* on the payloads, so a union of
        // comparable types is comparable and one of anything else simply is
        // not — which is what a struct field needs.
        out.push_str(&format!(
            "\n#[derive(Clone, Debug, PartialEq)]\npub enum Union{n}<{params}> {{\n"
        ));
        for i in 1..=n {
            out.push_str(&format!("    U{i}(T{i}),\n"));
        }
        out.push_str("}\n");
        out.push_str(&format!("\nimpl<{params}> Union{n}<{params}> {{\n"));
        for i in 1..=n {
            out.push_str(&format!(
                "    pub fn u{i}(&self) -> &T{i} {{\n        match self {{\n            \
                 Union{n}::U{i}(v) => v,\n            _ => panic!(\"unreachable union arm\"),\n        \
                 }}\n    }}\n"
            ));
            // [rs-narrow-mut] The mutable accessor: a value narrowed to this
            // arm and then *mutated* must reach the payload in place. The
            // read accessor above would force a clone at the call, and the
            // mutation would land on it.
            out.push_str(&format!(
                "    pub fn u{i}_mut(&mut self) -> &mut T{i} {{\n        match self {{\n            \
                 Union{n}::U{i}(v) => v,\n            _ => panic!(\"unreachable union arm\"),\n        \
                 }}\n    }}\n"
            ));
        }
        out.push_str("}\n");
        let bounds: Vec<String> = (1..=n)
            .map(|i| format!("T{i}: std::fmt::Display"))
            .collect();
        out.push_str(&format!(
            "\nimpl<{params}> std::fmt::Display for Union{n}<{params}>\nwhere\n    {},\n{{\n    \
             fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        \
             match self {{\n",
            bounds.join(",\n    ")
        ));
        for i in 1..=n {
            out.push_str(&format!(
                "            Union{n}::U{i}(v) => write!(f, \"{{}}\", v),\n"
            ));
        }
        out.push_str("        }\n    }\n}\n");
    }
    out
}

/// Does this module contain anything that turns into Rust code?
fn module_produces_code(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Struct(_) | Item::Effect(_) => true,
        Item::Handler(_) => true,
        Item::Fn(f) => f.body.is_some() || f.structural,
        Item::Qualifier(q) => q.fns.iter().any(|f| f.body.is_some()),
        _ => false,
    })
}

/// [rs-effect-fusion] Does any handler in the program declare an effect
/// dependency ([effect-handler-deps])? If so the whole program switches to
/// the fusion emission; otherwise effects thread as one `&mut dyn` parameter
/// each [rs-effects] and nothing below this line runs.
///
/// Purely syntactic since 2026-09-14: a dependency is an entry in the
/// handler's own effect list (`handler Stamped [Logger, Clock] of Logger`),
/// so nothing has to be resolved to answer the question.
///
/// **Reachable handlers only** (2026-09-14): std ships one, `DefaultFs
/// [RawFs]` in `core.hostfs`, and a program that never names it must not pay
/// the fused emission — which is also why that handler is not in `core.fs`,
/// a module every iterating program drags in [mod-used-only]. The gate reads
/// the same reachable set the emission loop does, and both backends read the
/// same one.
fn program_needs_fusion(symbols: &Symbols<'_>, reachable: &HashSet<&ModulePath>) -> bool {
    symbols.handlers.iter().any(|(name, h)| {
        // [effect-handler-multi] A handler of several effects needs the fusion
        // for the same reason a dependent one does, and it is the reason the
        // fusion exists: a fn declaring `[A, B]` takes one `&mut` per effect,
        // and when both are *this* handler's faces those are two mutable
        // borrows of one local, which rustc refuses (E0499). One fused value
        // carrying both accessors is the shape that borrows once.
        (h.effects.iter().flatten().count() > 0 || h.of.len() > 1)
            && symbols
                .handler_modules
                .get(name)
                .is_none_or(|m| reachable.contains(*m))
    })
}

/// Rust reserved words that need escaping as identifiers.
/// [platform-effect] The name a `main` that needs platform effects is
/// emitted under. Rust requires `fn main` in the crate root, so the host's
/// entry takes that name and calls this.
pub const SALVO_ENTRY: &str = "salvo_main";

/// [rs-effect-fusion] The stand-in a fusion struct is built under, so two
/// identical fusions can be recognized as one by comparing their text
/// (user decision 2026-09-14 — several `use` sites routinely produce the
/// same struct). Not a valid Rust identifier fragment on purpose: a
/// placeholder that survived substitution would be a loud compile error
/// rather than a silently odd name.
const FUSION_PLACEHOLDER: &str = "__Fx__PLACEHOLDER__";

const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "static",
    "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual",
    "where", "while", "yield",
];

/// Keywords that cannot be raw identifiers: rename with a trailing `_`.
const RUST_UNRAW: &[&str] = &["self", "Self", "super", "crate"];

/// Renders a Salvo name as a Rust identifier. Dot-names are *flattened*
/// (`Environment.Id` → `EnvironmentId`) [name-dot]: Rust puts modules and
/// structs in one type namespace, so a nested `mod Environment` beside
/// `struct Environment` is E0428 — concatenation is the only clean
/// rendering, which is why nothing else in scope may claim that spelling
/// (enforced in `resolve`). A dot cannot occur in any other Salvo name
/// that reaches here: dots are invalid in Rust identifiers.
/// [iter-fn] A pass field holding a callback: `emit_type` renders a
/// fn type inside an iterator fn as `impl Fn(..) -> R + 'static`, which a
/// struct field cannot spell, and the callback is shared by every pass
/// anyway. The rendering is exact, so this is surgery on it rather than a
/// second fn-type renderer that could drift from the first.
fn rc_fn_type(rendered: &str) -> String {
    // Each strip falls back to *its own* input: chaining them onto `rendered`
    // undid the prefix strip whenever the suffix was absent, which rendered
    // `Rc<dyn impl Fn(..)>` ("expected a trait, found type").
    let inner = rendered.strip_prefix("impl ").unwrap_or(rendered);
    let inner = inner.strip_suffix(" + 'static").unwrap_or(inner);
    format!("std::rc::Rc<dyn {inner}>")
}

fn rs_ident(name: &str) -> String {
    let flat = if name.contains('.') {
        name.replace('.', "")
    } else {
        name.to_string()
    };
    let name = flat.as_str();
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else if RUST_UNRAW.contains(&name) {
        format!("{name}_")
    } else {
        flat
    }
}

/// What a narrowed span's declared representation looks like, so the read
/// unwrap and the mutable one agree [rs-narrow-mut] [union-arm-identity].
#[derive(Clone, Copy)]
struct Narrowing {
    /// The wrapper-union arm the narrowed type identifies; `None` when the
    /// representation is a plain optional (`T?`) narrowed to its value.
    arm: Option<usize>,
    /// An `Option` sits in front of the payload, so it is unwrapped first.
    optional: bool,
    /// The payload is `Copy`, so a read dereferences rather than clones.
    copy: bool,
}

/// How a name in scope is bound in the emitted Rust [rs-borrows].
#[derive(Clone, Copy, PartialEq)]
enum BindKind {
    /// An owned binding (locals, moved parameters, lambda/loop bindings).
    Owned,
    /// A `&T` parameter.
    Ref,
    /// [rs-opt-borrow] A local holding `Option<&T>`: bound from a
    /// derived-return call whose result is optional (`first(xs)`, `get(xs,
    /// i)`). Narrowing it unwraps to a `&T` — the *reference*, not a clone
    /// of it — and reads through the unwrapped value render as a `Ref`
    /// would. Before this the unwrap emitted `.as_ref().unwrap().clone()`,
    /// which clones the reference (`&&T` → `&T`) and fails wherever an
    /// owned `T` is expected.
    OptRef,
    /// A `&mut T` parameter.
    RefMut,
    /// A handler constructor param or state field: accessed as `self.x`
    /// inside handler members.
    SelfField,
}

/// How a callee expects one parameter [rs-borrows].
#[derive(Clone, Copy, PartialEq)]
enum ParamMode {
    Owned,
    Ref,
    RefMut,
}

/// A handler registered in the effect environment: the Rust expression
/// used at member-call sites and the one used when threading it as a
/// `&mut dyn` argument differ for locals [rs-effects].
#[derive(Clone)]
struct EffectEntry {
    /// The checker's effect type when known — the primary lookup key,
    /// immune to rendering drift [effect-disambiguation].
    ty: Option<Ty>,
    /// Canonical rendered effect type, e.g. `Random<i32>`.
    key: String,
    /// Variable name (a `use` local or an effect parameter).
    var: String,
    /// True for `use` locals (thread as `&mut var`); false for `&mut dyn`
    /// parameters (thread as `var`, implicit reborrow).
    is_local: bool,
    /// [use-local] The eager **handle variable** for a shareable binding
    /// that a later construction in the same fn captures as a dependency:
    /// `let __handle_N = __Mon_E::new(Box::new(binding.clone()))`, emitted
    /// at the bind site because the binding value itself moves into the
    /// fusion. `None` when nothing captures the effect, or the binding is
    /// local.
    handle_var: Option<String>,
}

struct Emitter<'p> {
    symbols: &'p Symbols<'p>,
    checked: &'p Checked,
    program: &'p Program,
    file_idx: usize,
    file_name: String,
    imports: BTreeSet<String>,
    errors: Vec<String>,
    union_sizes: BTreeSet<usize>,
    effect_env: Vec<EffectEntry>,
    /// How each name in the current fn is bound [rs-borrows].
    bindings: HashMap<String, BindKind>,
    /// [placeholder] The code `_` renders as: the unpicked arm's read, set while
    /// a qualifier pick's right-hand side is emitted [pick].
    placeholder_code: Option<String>,
    pick_vars: usize,
    /// [use-local] The effect instances (rendered) that some construction
    /// in the current file captures as dependency handles — the bindings of
    /// these get an eager handle variable. Computed per file from
    /// `Checked::handle_captures`.
    captured_effects: HashSet<String>,
    /// [spawn-inherit] [rs-handle-bundle] Generated handle-bundle structs by
    /// *shape* (the rendered field list), so two fns needing the same handle
    /// set share one struct — the `__Fx_N` dedup rule, for handles.
    handle_bundles: HashMap<String, String>,
    /// [spawn-inherit] Where the current fn reads each required handle:
    /// rendered effect type → the place inside its bundle parameter.
    handle_fields: HashMap<String, String>,
    /// Parameter names reassigned in the body (they get a `mut` binder).
    mutated: HashSet<String>,
    generics: HashSet<String>,
    loop_results: Vec<Option<String>>,
    /// [let-destructure] Pending destructuring prologues, innermost last: a
    /// loop header whose pattern destructures pushes the statements its body
    /// must open with, and the body emission pops them.
    loop_destructures: Vec<String>,
    /// Whether each loop result local's join type is optional (assigns
    /// skip the `Some(...)`) [rs-loop-value].
    loop_optional: HashMap<String, bool>,
    loop_id: usize,
    destructure_id: usize,
    /// The current fn has a derived return (`proj[from: ...]`
    /// [readonly-return]): `return` values render as borrows.
    derived_return_fn: bool,
    /// [rs-proj-lends] Parameter names that an implicit's *lent* position
    /// (`?iter: (c: C) -> [c: proj] Mut It`) is named after, collected while
    /// the current fn's implicit parameters render: the enclosing fn keeps
    /// `c` under lifetime `'c`, which the position's `&'c C` shares.
    lent_position_params: Vec<String>,
    /// [proj-type] The lifetime a `proj` renders under while set (`'s` inside
    /// a borrowing struct or a `next` over one, `'a` on a tied return):
    /// `&'s T` instead of the elided `&T`.
    proj_lifetime: Option<String>,
    /// [proj-type] Set only while rendering a fn's **projected return type**,
    /// where a bare `proj T` over a generic parameter really is a borrow of a
    /// named parameter and Rust's elision ties it. Outside a return the
    /// generic exception below applies: at a definition site `proj T` renders
    /// `T`, because whether the instantiation is borrowed is the caller's
    /// fact. A `NonEmpty` accessor overload (`first(l: NonEmpty List<T>) ->
    /// proj[from: l] T`) is the case that needs the distinction — it returns
    /// the borrow, not an instantiation-dependent `T`.
    proj_return: bool,
    /// [rs-proj-arm] Locals whose union storage has *borrowed* arms: bound
    /// from a call whose return type has a `proj` arm (`let step =
    /// next(p)` is a `Union2<&T, Finished>`). Reading such an arm's payload
    /// yields a reference — a Copy scalar derefs once more, anything else
    /// binds as `Ref` [rs-opt-borrow].
    borrowed_arm_locals: HashMap<String, Vec<usize>>,
    /// Set by `emit_narrowed_read` when the payload it produced is a
    /// reference (a borrowed non-Copy arm); the caller binds it as `Ref`.
    narrowed_read_is_ref: bool,
    /// [copy-implicit] Set while rendering a `use`'s constructor implicits:
    /// the handler stores them, so the adapters are owned `move` closures,
    /// as a producer's are.
    emitting_producer_args: bool,
    /// [rs-proj-struct] Structs with a `proj` field — passes borrowing their
    /// source [proj-field]. Each is emitted with a lifetime `'s`
    /// (`ListYield<'s, T>`) and its `proj` fields as `&'s T`; every mention
    /// of the type carries the lifetime (`ListYield<'_, T>` in signatures).
    borrowing_structs: HashSet<String>,
    /// [rs-proj-struct] The current fn returns a borrowing struct by value.
    returns_borrowing_struct: bool,
    /// [rs-proj-arm] In a union-returning derived fn, the constructor
    /// names (`emitted`) whose arm is the `proj` one.
    proj_arm_ctors: Vec<String>,
    taken_names: HashSet<String>,
    generated_imports: BTreeSet<String>,
    /// [rs-crate] The module that became the crate root (the one declaring
    /// `main`), so a path to a top-level fn can be spelled correctly:
    /// `crate::…` for it, `crate::<mounted>::…` for every other module.
    root_module: Option<&'p salvo_core::ModulePath>,
    /// [fn-effects] Absolute crate-path prefix per effect name, for the
    /// generated pass-trait file (which imports nothing).
    effect_paths: HashMap<String, String>,
    /// [rs-none-unit] The fn being emitted returns `None`, i.e. Rust `()`:
    /// `return None;` must be a bare `return;`. Not decidable from the
    /// returned *value*'s type — `return None` in an `Option`-returning fn is
    /// `return None;` and correct.
    ret_is_unit: bool,
    /// [rs-effect-fusion] Program-wide: some handler declares an effect
    /// dependency, so every effect site threads one *fused* value.
    fusion: bool,
    /// Items the fusion generates for this file (conjunction traits,
    /// fusion structs and their forwarding impls, dependency adapters),
    /// appended after the module body [rs-effect-fusion].
    generated_items: Vec<String>,
    /// Conjunction traits already generated in this file, by name, with
    /// their supertrait list (to catch a name claimed by two effect sets).
    conj_traits: BTreeMap<String, String>,
    /// [cmp-carry] The ordering markers this module has already generated, name
    /// -> the element type it orders: one zero-sized struct and `SalvoCmp` impl
    /// per identity a keyed container here is kept by. Module-local, so nothing
    /// has to be imported and two modules naming one ordering each get their own.
    ordering_markers: BTreeMap<String, String>,
    /// Fusion-struct counter for this file.
    fusion_id: usize,
    /// [rs-effect-fusion] Fusion structs emitted in this file, keyed by
    /// their text under the placeholder name → the name they got, so one
    /// shape is one struct however many `use` sites need it.
    fusion_structs: HashMap<String, String>,
    /// [platform-handler] [platform-tree] The modules whose `platform/`
    /// companion this file's `use` sites depend on: registering a platform
    /// handler constructs a *host* struct, so the companion defining it has
    /// to exist. Collected per file and checked once, program-wide.
    platform_hosts: BTreeSet<ModulePath>,
    /// Hoisted-temporary counter for the current fn [rs-effect-fusion].
    hoist_id: usize,
    /// The fn currently being emitted, for readable generated names.
    current_fn: String,
    /// [actor-replyto] [actor-self-send] The **name** of the handler whose
    /// member body is being emitted, when one is: a `replyto` mints against
    /// the *enclosing* handler's protocol and a self-send calls its member.
    /// `None` in ordinary fns, which is where the checker has already refused
    /// both. A name rather than a reference because `FnStyle` carries its own
    /// lifetime; the declaration is one `symbols.handlers` lookup away.
    current_handler: Option<String>,
    /// Generic-parameter substitutions while rendering a forwarding impl
    /// for an *instantiated* effect (`Random<T>` members at `Random<i32>`).
    type_subst: HashMap<String, String>,
    /// [rs-exit-splice] Code the compiler owes at every exit of a block,
    /// innermost/latest last, each already rendered at indent 0. Today the
    /// only source is the release a `for` owes a pass it owns
    /// [linear-group]; there is no runtime representation, so the code is
    /// rendered once and re-indented at each splice site. (The `defer`
    /// statement was this mechanism's other customer until it was removed
    /// from the language, 2026-09-10.)
    exit_splices: Vec<String>,
    /// Index into `exit_splices` below which entries belong to an enclosing
    /// function: a `return` inside a closure only runs the closure's own.
    splice_floor: usize,
    /// `exit_splices` length at each enclosing loop's body entry: `break`
    /// and `continue` run the splices registered inside the loop.
    loop_splice_floors: Vec<usize>,
    /// `return`-value temporary counter [rs-exit-splice].
    splice_id: usize,
    /// [rs-throw-controlflow] The declared throw message type of the fn
    /// being emitted, when it declares `[Throw<M>]`: its Rust return type
    /// is then `ControlFlow<M, T>`, `return v` becomes
    /// `ControlFlow::Continue(v)`, and a propagating call unwraps.
    throw_message: Option<Ty>,
    /// [implicit-param] The implicit parameters of the fn being emitted, in
    /// the checker's order: trailing parameters of the signature, and the
    /// names a bare call inside the body reaches as *values*.
    implicits: Vec<salvo_core::ImplicitParam>,
    /// [iter-fn] Passes minted at the arguments of the call being
    /// rendered: (local name, construction, release). The mint is the
    /// *compiler's* value, so the compiler closes it — hoisted into a `let`
    /// and released after the call, whatever the callee did with it. That is
    /// the `for` lowering's discipline (construct, drive, close) at a call.
    pending_mints: Vec<(String, String, String)>,
    /// [iter-fn] The machine types minted at the call being emitted, in
    /// argument order: the advance adapter has to annotate its closure
    /// parameter with one, since rustc cannot infer it through the `&mut dyn
    /// FnMut` coercion the implicit position renders.
    mint_machines: Vec<String>,
    /// [iter-protocol] This file names the protocol's own types (`Finished`) in
    /// code the emitter *synthesized* — a driving loop or an advance adapter —
    /// so the module declaring them has to be imported even when the source
    /// never mentions them.
    needs_protocol: bool,
    /// [rs-mut-str] This file calls a string helper, so the program needs
    /// the generated string support file.
    needs_str: bool,
    /// [rs-seq] This file calls a sequence helper, so the program needs the
    /// generated sequence support file.
    needs_seq: bool,
    /// [rs-collections] Whether this module referenced `Set`/`Map`, so the
    /// ordered-collection runtime is emitted and mounted.
    needs_collections: bool,
    /// [rs-actor] This file spawns, sends to an addr, or bridges with
    /// `waitfor`, so the scheduler module is part of the program.
    needs_scheduler: bool,
    /// [time-types] Whether this module reads a clock, so the time module is
    /// part of the program.
    needs_time: bool,
    /// [is-bind-once] Hoisted `is` subjects: the span of a subject expression
    /// that is *not* a place, mapped to the temporary holding its single
    /// evaluation. Both the test and the binding read the temp through
    /// `place_storage`, which is the whole fix — the two used to emit the
    /// subject independently, so a call ran twice per test.
    is_temps: HashMap<(usize, Span), String>,
    /// Counter for those temporaries' names.
    is_temp_id: usize,
    /// [rs-iter-pass] An iterator fn's signature or body is being emitted:
    /// its fn-typed parameters arrive owned and `'static`, since they are
    /// used in every pass rather than during the call.
    in_iterator_fn: bool,
    /// [iter-fn] Inside an iterator fn's body: the names that are
    /// fields of the generated pass rather than locals. `bindings` renders
    /// their *reads* (`SelfField`); this set is what tells a `let` to assign
    /// the field instead of declaring a local, and a call of a fn-typed one
    /// to parenthesize the callee.
    gen_fields: HashSet<String>,
    /// [iter-fn] Of those, the ones held in an `Option` because their
    /// type has no zero value (`BindKind::SelfSlot`).
    gen_slots: HashSet<String>,
    /// [rs-fn-param-convention] Set just before a lambda argument is
    /// rendered into a fn-typed parameter: the binding modes the callee's
    /// *declared* fn type gives that position's parameters. Taken by
    /// `emit_lambda`, so it never leaks to a nested lambda.
    pending_lambda_conv: Option<Vec<BindKind>>,
    /// [yield-proj] Per lambda parameter, whether the declared parameter type
    /// is the call's *retagged* element generic (`T` instantiated at `&T`):
    /// the closure receives one more reference than its body was emitted
    /// for, and peels it at entry.
    pending_lambda_retag: Option<Vec<bool>>,
    /// The element generics retagged to `&T` at the call being emitted.
    retagged_generics: Vec<String>,
    /// [rs-iter-pass] Set the same way, for the same reason: an iterator fn's
    /// fn-typed parameter is `impl Fn + 'static`, so a lambda handed to one
    /// must be a `move` closure — it outlives the call that created the pass.
    pending_lambda_move: bool,
    /// Enclosing `try` delimiters being emitted [rs-try-label], innermost
    /// last: a throw inside one breaks its labelled block instead of
    /// returning.
    try_frames: Vec<TryFrame>,
    /// Labelled-block counter for `try` [rs-try-label].
    try_id: usize,
    /// The indentation of the statement being emitted: expression-position
    /// control transfers ([rs-throw-controlflow]) splice exit code,
    /// which are statements, so they need a column to write at.
    expr_indent: usize,
}

/// One `try` delimiter while its body is emitted [rs-try-label].
struct TryFrame {
    /// The Rust block label (`'try_0`).
    label: String,
    /// `exit_splices.len()` at entry: a throw into this delimiter runs the
    /// splices registered inside the `try` body, and only those.
    splice_floor: usize,
    /// The outcome union `Ok T | Thrown M`, for wrapping both arms.
    outcome: Option<Ty>,
}

#[derive(Clone, Copy, PartialEq)]
enum StmtCtx {
    /// The only context left: the `IteratorBody` one died with the `async`
    /// lowering — an iterator fn's `yield`s and `return`s are steps of the
    /// generated machine now [iter-fn], so they never reach
    /// `emit_stmt`. The parameter is kept because the next context to need
    /// one is cheaper to add than to thread (removal is part of the I6
    /// sweep).
    Normal,
}

impl<'p> Emitter<'p> {
    fn new(
        symbols: &'p Symbols<'p>,
        checked: &'p Checked,
        program: &'p Program,
        file_idx: usize,
        file_name: &str,
    ) -> Self {
        Emitter {
            symbols,
            checked,
            program,
            file_idx,
            file_name: file_name.to_string(),
            imports: BTreeSet::new(),
            errors: Vec::new(),
            union_sizes: BTreeSet::new(),
            effect_paths: HashMap::new(),
            ret_is_unit: false,
            effect_env: Vec::new(),
            bindings: HashMap::new(),
            placeholder_code: None,
            pick_vars: 0,
            captured_effects: HashSet::new(),
            handle_bundles: HashMap::new(),
            handle_fields: HashMap::new(),
            mutated: HashSet::new(),
            generics: HashSet::new(),
            loop_results: Vec::new(),
            loop_destructures: Vec::new(),
            loop_optional: HashMap::new(),
            loop_id: 0,
            destructure_id: 0,
            derived_return_fn: false,
            emitting_producer_args: false,
            lent_position_params: Vec::new(),
            proj_lifetime: None,
            proj_return: false,
            borrowed_arm_locals: HashMap::new(),
            narrowed_read_is_ref: false,
            returns_borrowing_struct: false,
            proj_arm_ctors: Vec::new(),
            // [rs-proj-struct] Program-wide: a borrowing struct is mentioned
            // by every file that uses it, and the lifetime has to appear at
            // each mention.
            // [proj-field] Transitive: a struct whose owned field holds a
            // borrowing struct (an `iter fn`'s state keeping an inner view,
            // say) carries the lifetime too.
            borrowing_structs: {
                let structs: HashMap<&str, &salvo_syntax::ast::StructDecl> = program
                    .modules
                    .iter()
                    .flat_map(|m| m.items.iter())
                    .filter_map(|item| match item {
                        Item::Struct(sd) => Some((sd.name.name.as_str(), sd)),
                        _ => None,
                    })
                    .collect();
                structs
                    .values()
                    .filter(|sd| {
                        sd.fields.iter().any(|f| {
                            type_has_proj(&f.ty) || salvo_core::lends::holds_proj(&f.ty, &structs)
                        })
                    })
                    .map(|sd| sd.name.name.clone())
                    .collect()
            },
            taken_names: HashSet::new(),
            generated_imports: BTreeSet::new(),
            root_module: None,
            fusion: false,
            generated_items: Vec::new(),
            conj_traits: BTreeMap::new(),
            ordering_markers: BTreeMap::new(),
            fusion_id: 0,
            fusion_structs: HashMap::new(),
            platform_hosts: BTreeSet::new(),
            hoist_id: 0,
            current_fn: String::new(),
            current_handler: None,
            type_subst: HashMap::new(),
            exit_splices: Vec::new(),
            splice_floor: 0,
            loop_splice_floors: Vec::new(),
            splice_id: 0,
            throw_message: None,
            implicits: Vec::new(),
            pending_mints: Vec::new(),
            mint_machines: Vec::new(),
            needs_protocol: false,
            needs_str: false,
            needs_seq: false,
            needs_collections: false,
            needs_scheduler: false,
            needs_time: false,
            is_temps: HashMap::new(),
            is_temp_id: 0,
            in_iterator_fn: false,
            gen_fields: HashSet::new(),
            gen_slots: HashSet::new(),
            pending_lambda_conv: None,
            pending_lambda_retag: None,
            retagged_generics: Vec::new(),
            pending_lambda_move: false,
            try_frames: Vec::new(),
            try_id: 0,
            expr_indent: 0,
        }
    }

    // ================= checker table access =================

    fn ty_of(&self, span: Span) -> Option<&'p Ty> {
        self.checked.expr_ty.get(&(self.file_idx, span))
    }

    /// [op-promote] Wraps an operand the checker widened in a cast to the
    /// promoted type. Only `Long` and `Double` can be targets (widening
    /// goes up within a class). The operand is parenthesized: `as` binds
    /// tighter than every arithmetic operator, so `(n * 2 as i64)` would
    /// cast only the `2`.
    fn promote_operand(&self, span: Span, code: String) -> String {
        match self.checked.promotions.get(&(self.file_idx, span)) {
            Some(Ty::Named { name, .. }) if name == "Long" => format!("(({code}) as i64)"),
            Some(Ty::Named { name, .. }) if name == "Double" => format!("(({code}) as f64)"),
            _ => code,
        }
    }

    fn repr_of(&self, span: Span) -> Option<&'p Ty> {
        self.checked.repr_ty.get(&(self.file_idx, span))
    }

    /// The representation change recorded at `span`, if any.
    ///
    /// [str-drop-mut] A `DropMut` record is *transparent* on this backend:
    /// `Mut` erases here — `Mut Str` and `Str` are both `String`, as
    /// `Mut List<T>` and `List<T>` are both `Vec<T>` [type-canbe-mut] — so
    /// what matters is only the change the drop carries with it. Unwrapping
    /// it here rather than in `apply_coercion` means every other reader
    /// (the "is this argument a fresh temporary?" tests) also sees the truth
    /// that nothing happens at a drop.
    fn coercion_of(&self, span: Span) -> Option<&'p Coercion> {
        let mut coercion = self.checked.coerce.get(&(self.file_idx, span));
        while let Some(Coercion::DropMut { then, .. }) = coercion {
            coercion = then.as_deref();
        }
        coercion
    }

    fn is_test_of(&self, span: Span) -> Option<&'p UnionTest> {
        self.checked.is_tests.get(&(self.file_idx, span))
    }

    /// [iter-protocol] How a `for` drives a **pass**, if its subject is one.
    fn pass_driver_of(&self, iterable: &Expr) -> Option<salvo_core::PassDriver> {
        self.checked
            .for_drivers
            .get(&(self.file_idx, iterable.span()))
            .cloned()
    }

    /// [iter-protocol] The loop header for a `for` over a **pass**: the
    /// subject is bound to a mutable local, and each turn calls the `next`
    /// the checker resolved and matches its `Emitted` arm.
    ///
    /// ```text
    /// let mut __pass0 = countdown(3);
    /// while let Union2::U1(mut n) = next(&mut __pass0) {
    /// ```
    ///
    /// A `while let` rather than `loop`/`match`, because the condition is
    /// re-evaluated per turn and the `Finished` arm needs no arm of its own.
    /// The subject is *moved* into the local: driving consumes a pass
    /// [once-fn], so nothing else can be looking at it.
    fn emit_pass_loop_header(
        &mut self,
        driver: salvo_core::PassDriver,
        pattern: &Pattern,
        iterable: &Expr,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // [iter-generic-drive] The `next` is either a declared overload or an
        // **implicit parameter** of this body — a generic pass has no
        // declaration to resolve against, and the parameter is what the caller
        // filled [implicit-param].
        let callee = match &driver.next {
            salvo_core::PassMember::Implicit(name) => rs_ident(name),
            salvo_core::PassMember::Fn(key) => match self.fn_by_key(*key) {
                Some(decl) => self.rust_fn_name(decl),
                None => {
                    self.error("the `next` this `for` resolved to is not available");
                    return String::new();
                }
            },
        };
        // [fn-effects] An effectful `next` takes its handlers as leading
        // arguments, threaded into every turn of the loop from the scope the
        // `for` is written in — the same arguments an ordinary call to it would
        // pass. An *implicit* `next` is refused instead: there the effects live
        // on a fn *value*, whose caller supplies them through a different path
        // [backend-never-wrong].
        let mut handler_args: Vec<String> = Vec::new();
        match &driver.next {
            salvo_core::PassMember::Fn(key) => {
                let effects: Vec<Ty> = self
                    .checked
                    .fn_effects
                    .get(key)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !is_throw_effect_ty(t))
                    .collect();
                for ty in &effects {
                    handler_args.push(self.thread_effect_by_ty(ty));
                }
                // [rs-effect-fusion] An effectful `next` is an ordinary
                // fused callee: one value carries its whole effect set.
                if self.fusion {
                    handler_args.dedup();
                    handler_args.truncate(1);
                }
            }
            salvo_core::PassMember::Implicit(_) => {
                if driver
                    .next
                    .key()
                    .and_then(|k| self.checked.fn_effects.get(&k))
                    .is_some_and(|e| !e.is_empty())
                {
                    self.error(
                        "a generic `next` that performs effects is not supported \
                         yet: the handlers would have to reach a fn value the \
                         caller supplied",
                    );
                }
            }
        }
        let lead = if handler_args.is_empty() {
            String::new()
        } else {
            format!("{}, ", handler_args.join(", "))
        };
        if driver.arms < 2 {
            // The checker only records a driver for the exact
            // `Emitted T | Finished` shape, so this cannot happen — and if it
            // ever does, saying so beats emitting a loop that never ends.
            self.error("a `next` result must have both an `Emitted` and a `Finished` arm");
            return String::new();
        }
        let place = format!("{}_pass", self.fresh_loop_var());
        let var = self.for_pattern_var(pattern, false, indent + 1);
        // The `Emitted` arm of the result, by the identity the *checker*
        // computed [union-arm-identity].
        self.union_sizes.insert(driver.arms);
        let arm = format!("Union{}::U{}", driver.arms, driver.emitted_arm + 1);
        // [iter-drive-in-place] A pass the fn *keeps* is advanced where it
        // lives: binding it into a local would clone it (the parameter is a
        // `&mut`), and the caller would never see the position the loop
        // reached — which is what Kotlin's aliasing did all along
        // [backend-parity].
        if driver.in_place {
            let subject = self.borrowed_mut_arg(iterable);
            return format!("{pad}while let {arm}({var}) = {callee}({lead}{subject}) {{\n");
        }
        // The loop *consumes* the pass (the checker moved it in), so the local
        // takes it over rather than cloning it: a clone would leave the original
        // unreleased, which for a linear pass is the leak `close` exists to
        // prevent — and cost an allocation for every other pass.
        let subject = match driver.mint_iter_fn {
            // [iter-pass] The subject is a *container*, not a pass: its `iter`
            // mints one, called once before the loop. The arguments go through
            // the ordinary machinery, so the parameter's mode decides whether
            // the container is borrowed or moved [rs-borrows].
            Some(key) => match self.fn_by_key(key) {
                Some(decl) => {
                    let callee = self.rust_fn_name(decl);
                    let params = decl.params.clone();
                    let args = self.emit_args_for_params(&params, &[iterable], Some(key));
                    format!("{callee}({})", args.join(", "))
                }
                None => {
                    self.error("the `iter` this `for` mints with is not available");
                    return String::new();
                }
            },
            None => {
                let code = self.emit_place(iterable);
                self.apply_coercion(iterable.span(), code)
            }
        };
        format!(
            "{pad}let mut {place} = {subject};\n\
             {pad}while let {arm}({var}) = {callee}({lead}&mut {place}) {{\n"
        )
    }

    fn fn_by_key(&self, key: salvo_core::FnKey) -> Option<&'p FnDecl> {
        match self.program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }

    /// [rs-fn-field] Whether this fn **stores** a callback it is given: a
    /// fn-typed parameter, and a struct with a fn-typed field as the result.
    /// That is the composed-pass shape — a pass wrapping a source pass and a
    /// callback — and nothing else in the language keeps a callback past the
    /// call, so its callbacks arrive owned and `'static`, shared internally
    /// through an `Rc`, rather than borrowed for the call. std stopped writing
    /// one when the lazy pair was removed (2026-09-10); a program still may.
    fn owns_callbacks(&self, key: salvo_core::FnKey) -> bool {
        let Some(decl) = self.fn_by_key(key) else {
            return false;
        };
        if !decl.params.iter().any(|p| matches!(p.ty, Type::Fn { .. })) {
            return false;
        }
        let Some(ret) = decl.return_type.as_ref().and_then(type_base_name) else {
            return false;
        };
        self.symbols
            .structs
            .get(ret)
            .is_some_and(|s| s.fields.iter().any(|f| matches!(f.ty, Type::Fn { .. })))
    }

    /// The stable key of a checker-resolved fn declaration (needed to look
    /// up its deductions [rs-borrows]).
    fn key_of_fn(&self, decl: &FnDecl) -> Option<salvo_core::FnKey> {
        for (file, module) in self.program.modules.iter().enumerate() {
            for (item, it) in module.items.iter().enumerate() {
                if let Item::Fn(f) = it {
                    if std::ptr::eq(f as *const FnDecl, decl as *const FnDecl) {
                        return Some(salvo_core::FnKey { file, item });
                    }
                }
            }
        }
        None
    }

    /// [rs-actor] An item declared beside an effect, as an absolute path:
    /// the effect's own module prefix plus the name. Used for a protocol's
    /// message enum and trait, which a spawn or a send may mention from
    /// another module.
    fn effect_path(&self, effect: &str, item: &str) -> String {
        match self.effect_paths.get(effect) {
            Some(prefix) => format!("{prefix}{item}"),
            None => item.to_string(),
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors
            .push(format!("{}: {}", self.file_name, msg.into()));
    }

    // ================= module =================

    fn emit_module(&mut self, module: &Module) -> String {
        // [use-local] What this file's constructions capture, so bind sites
        // know to mint an eager handle.
        // [spawn-inherit] A spawn's **synthesized** dependencies capture the
        // same way — the child holds the scope's handle — so the pre-scan
        // reads both tables; `spawn_dep_items` says which entries of
        // `spawn_deps` were inherited rather than written.
        let mut captured: Vec<Ty> = self
            .checked
            .handle_captures
            .iter()
            .filter(|((f, _), _)| *f == self.file_idx)
            .flat_map(|(_, tys)| tys.iter().cloned())
            .collect();
        for (key, tys) in &self.checked.spawn_deps {
            if key.0 != self.file_idx {
                continue;
            }
            let items = self.checked.spawn_dep_items.get(key);
            for (i, ty) in tys.iter().enumerate() {
                let inherited = items.map_or(true, |items| items.get(i) == Some(&None));
                if inherited {
                    captured.push(ty.clone());
                }
            }
        }
        self.captured_effects = captured.iter().map(|t| self.rust_ty(t)).collect();
        let mut body = String::new();
        for item in &module.items {
            match item {
                Item::Struct(s) => body.push_str(&self.emit_struct(s)),
                // [throw] The throw effect has no handlers — `try` delimits
                // it — so there is nothing to implement: emitting an
                // interface for it would be dead, misleading code.
                Item::Effect(e) if e.name.name == salvo_core::THROW_EFFECT => {}
                Item::Effect(e) => body.push_str(&self.emit_effect(e)),
                Item::Handler(h) => body.push_str(&self.emit_handler(h)),
                // [cmp-auto] A structural member has no body and is still
                // emitted: its body is the host's derived operation.
                Item::Fn(f) if f.body.is_some() || f.structural => {
                    body.push_str(&self.emit_fn(f))
                }
                Item::Qualifier(q) => body.push_str(&self.emit_qualifier(q)),
                _ => {}
            }
        }
        // [rs-imports] Generated module imports, template `imports:`
        // lines, and the union enums when this file uses any.
        let mut imports = self.generated_imports.clone();
        imports.extend(self.imports.iter().cloned());
        if !self.union_sizes.is_empty() {
            imports.insert("use crate::unions::*;".to_string());
        }
        if self.needs_str {
            imports.insert("use crate::strings::*;".to_string());
        }
        if self.needs_seq {
            imports.insert("use crate::seq::*;".to_string());
        }
        // [rs-collections] The ordered `Set`/`Map`: needed by any module
        // that so much as *names* one in a signature, not only by one that
        // calls into it.
        if self.needs_collections {
            imports.insert("use crate::collections::*;".to_string());
        }
        let mut out = String::new();
        if !imports.is_empty() {
            for import in &imports {
                out.push_str(import);
                out.push('\n');
            }
        }
        out.push_str(&body);
        // [rs-effect-fusion] Generated items go last: they are plain items
        // and Rust has no ordering requirement, so nothing above needs to
        // know they exist.
        for item in std::mem::take(&mut self.generated_items) {
            out.push_str(&item);
        }
        out
    }

    // ================= declarations =================

    fn emit_struct(&mut self, s: &StructDecl) -> String {
        let saved = self.enter_generics(&s.generics);
        let mut generics = self.emit_generic_params(&s.generics);
        // [rs-proj-struct] A pass borrowing its source carries the source's
        // lifetime: `pub struct ListYield<'s, T> { pub items: &'s Vec<T>, … }`.
        let borrowing = self.borrowing_structs.contains(&s.name.name);
        if borrowing {
            generics = if generics.is_empty() {
                "<'s>".to_string()
            } else {
                format!("<'s, {}", &generics[1..])
            };
        }
        // [rs-fn-field] A fn-typed field is held as `Rc<dyn Fn…>` — the same
        // representation a generated pass has always used for a stored
        // callback [iter-fn]. `dyn Fn` has no `Debug`, so a struct
        // with one gets a hand-written `Debug` instead of the derive.
        let fn_fields: Vec<String> = s
            .fields
            .iter()
            .filter(|f| matches!(f.ty, Type::Fn { .. }))
            .map(|f| rs_ident(&f.name.name))
            .collect();
        // [col-equality] [col-hashed-ordered] Every struct supports `==`, so
        // `PartialEq` is derived unless a fn-typed field makes equality
        // meaningless (`Rc<dyn Fn>` has none). The opt-ins add what a
        // collection needs on top: `auto Hashed<self>` gives `Eq + Hash`,
        // `auto Ordered<self>` the total order — both of which exclude float
        // fields, which is exactly what the checker validated at the
        // declaration, so the derives cannot fail here.
        // [cmp-auto] The derives an `auto fn` asks for: the generated member is
        // defined in terms of them [rs-cmp-groups]. Asked of the *declarations*,
        // not of an obligation clause — `auto` is a modifier on the function now
        // (user decision 2026-09-22), so a struct may generate a `hash` with no
        // clause at all.
        let hashed = self.program.has_auto_member(&s.name.name, "hash");
        let ordered = self.program.has_auto_member(&s.name.name, "cmp");
        let derives = if fn_fields.is_empty() {
            let mut items = vec!["Clone", "Debug", "PartialEq"];
            if hashed || ordered {
                items.push("Eq");
            }
            if hashed {
                items.push("Hash");
            }
            if ordered {
                items.push("PartialOrd");
                items.push("Ord");
            }
            format!("#[derive({})]", items.join(", "))
        } else {
            "#[derive(Clone)]".to_string()
        };
        let mut out = format!(
            "\n{derives}\npub struct {}{generics} {{\n",
            rs_ident(&s.name.name)
        );
        for field in &s.fields {
            // [backend-never-wrong] A `params` *group* in field position is
            // a bundle of functions with no single type to store; the
            // checker refuses it as a value [group-not-a-value], and this is
            // the backend's own guard.
            if is_fn_group(&field.ty) {
                self.error(format!(
                    "the rust backend cannot store a `params` group in a struct field \
                     (`{}.{}`): pass it as a parameter instead — an implicit \
                     parameter (`?{}: ...`) or a `params` group is how a bundle of \
                     functions travels",
                    s.name.name, field.name.name, field.name.name
                ));
            }
            let ty = if matches!(field.ty, Type::Fn { .. }) {
                // A stored callback is reached through a shared `Rc`, so it
                // has to be `Fn`, not `FnMut` — the same rendering an
                // iterator fn's callback gets [rs-iter-pass], reused rather
                // than duplicated.
                let saved_iter = self.in_iterator_fn;
                self.in_iterator_fn = true;
                let rendered = self.emit_type(&field.ty);
                self.in_iterator_fn = saved_iter;
                rc_fn_type(&rendered)
            } else if borrowing && type_has_proj(&field.ty) {
                // [rs-proj-struct] The borrowed field: `&'s T` of the type
                // under the `proj` — the general rendering, under `'s`.
                let saved = self.proj_lifetime.replace("'s".to_string());
                let rendered = self.emit_type(&field.ty);
                self.proj_lifetime = saved;
                rendered
            } else if borrowing {
                // [proj-field] An owned field holding a view shares the
                // struct's lifetime: `'_` is not allowed in a declaration.
                self.emit_type(&field.ty).replace("<'_", "<'s")
            } else {
                self.emit_type(&field.ty)
            };
            out.push_str(&format!("    pub {}: {ty},\n", rs_ident(&field.name.name)));
        }
        out.push_str("}\n");
        if !fn_fields.is_empty() {
            out.push_str(&self.emit_fn_field_debug(s, &fn_fields));
        }
        self.generics = saved;
        out
    }

    /// [rs-fn-field] The hand-written `Debug` for a struct holding a
    /// function: every other field prints as it would, the functions print
    /// as `<fn>`. Written rather than derived because `dyn Fn` has no
    /// `Debug` — and needed rather than dropped, since `{:?}` on a struct is
    /// how `${…}` interpolation renders one [rs-display].
    fn emit_fn_field_debug(&mut self, s: &StructDecl, fn_fields: &[String]) -> String {
        let name = rs_ident(&s.name.name);
        let args = if s.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                s.generics
                    .iter()
                    .map(|g| g.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        // A derive would have bounded every type parameter by `Debug`; this
        // impl prints the same fields, so it needs the same bound — a
        // composed pass holds its source as a `T`, and `{:?}` on it is only
        // available where the source has one.
        let generics = if s.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                s.generics
                    .iter()
                    .map(|g| format!("{}: Clone + 'static + std::fmt::Debug", g.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let mut body = format!(
            "\nimpl{generics} std::fmt::Debug for {name}{args} {{\n    \
             fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        \
             f.debug_struct(\"{}\")\n",
            s.name.name
        );
        for field in &s.fields {
            let fname = rs_ident(&field.name.name);
            if fn_fields.contains(&fname) {
                body.push_str(&format!(
                    "            .field(\"{}\", &\"<fn>\")\n",
                    field.name.name
                ));
            } else {
                body.push_str(&format!(
                    "            .field(\"{}\", &self.{fname})\n",
                    field.name.name
                ));
            }
        }
        body.push_str("            .finish()\n    }\n}\n");
        body
    }

    fn emit_effect(&mut self, e: &EffectDecl) -> String {
        let saved = self.enter_generics(&e.generics);
        let generics = self.emit_generic_params_unbounded(&e.generics);
        let mut out = format!("\npub trait {}{generics} {{\n", rs_ident(&e.name.name));
        for (i, f) in e.fns.iter().enumerate() {
            // [rs-effects] `dyn` traits cannot have generic methods.
            if !f.generics.is_empty() {
                self.error(format!(
                    "effect member `{}` has its own generic parameters, which the \
                     rust backend cannot dispatch dynamically yet",
                    f.name.name
                ));
                continue;
            }
            let params = format!(
                "{}{}",
                self.emit_member_param_list(f),
                self.emit_member_implicits(f)
            );
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret};\n",
                self.member_name(e, i)
            ));
        }
        out.push_str("}\n");
        // [rs-effect-fusion] The Has-accessor trait, next to the effect it
        // accesses (user decision 2026-09-14, §5.8.1 of FILE_SYSTEM.md):
        // fused values implement `__Has_E` per effect in scope instead of
        // the effect traits themselves, so member names can never collide
        // on a fused value and two instances of a generic effect
        // disambiguate with the trait's turbofish. Generic exactly as the
        // effect is, so one declaration serves every instance and its
        // identity crosses modules with the effect's own globs
        // [rs-imports]. Plain mode never mentions it, so it is only
        // emitted under the fusion gate.
        if self.fusion {
            let getter = has_getter_name(&e.name.name);
            let args: Vec<String> = e.generics.iter().map(|g| g.name.clone()).collect();
            out.push_str(&format!(
                "\npub trait {}{generics} {{\n    fn {getter}(&mut self) -> &mut dyn {};\n}}\n",
                has_trait_name(&e.name.name),
                trait_type(&e.name.name, &args)
            ));
        }
        // [rs-actor] [actor-use-addr] The forwarding stub: the effect,
        // implemented by sending to an addr.
        out.push_str(&self.emit_addr_stub(e));
        // [monitor-handler] [rs-monitor] The lock wrapper: the effect,
        // implemented by locking a shared instance and delegating — what a
        // monitor spawn answers and `Addr<E>` lowers to for a plain effect.
        out.push_str(&self.emit_monitor_stub(e));
        // [rs-actor] [actor-send-fn] The protocol's **message type**: one
        // variant per send member, carrying its payload. It belongs to the
        // *effect* rather than to a handler, because a sender holds an `Addr`
        // and knows only the effect it serves — which is the same reason the
        // binding swap works at all.
        out.push_str(&self.emit_message_enum(e));
        self.generics = saved;
        out
    }

    /// [rs-actor] `pub enum __Msg_E { Member(payload…), … }` — the messages
    /// an actor serving `E` receives, and the whole of what crosses the seam
    /// at runtime (the scheduler is untyped: `Box<dyn Any + Send>`).
    ///
    /// Emitted only for an effect with at least one `send fn`; a synchronous
    /// effect never becomes messages.
    /// [actor-use-addr] [rs-actor] `__Stub_E`: the effect implemented by
    /// **sending to an addr**. This is what `use addr` binds, and — from item 4
    /// on — what a spawned child receives when a dependency was supplied as an
    /// addr rather than as a construction.
    ///
    /// It is emitted per *effect*, not per use site, because that is all it
    /// depends on: a handler is compiled once and bound many ways, and this is
    /// the binding that turns a call into a message.
    fn emit_addr_stub(&mut self, e: &EffectDecl) -> String {
        if !e.is_actor || !e.generics.is_empty() {
            return String::new();
        }
        let sends: Vec<(usize, &FnDecl)> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_send)
            .collect();
        if sends.is_empty() {
            return String::new();
        }
        self.needs_scheduler = true;
        let name = stub_struct_name(&e.name.name);
        let msg = msg_enum_name(&e.name.name);
        let mut out = format!(
            "\npub struct {name} {{\n    addr: usize,\n}}\n\nimpl {name} {{\n    \
             pub fn new(addr: usize) -> Self {{\n        Self {{ addr }}\n    }}\n}}\n\
             \nimpl {} for {name} {{\n",
            rs_ident(&e.name.name)
        );
        for (i, f) in &sends {
            let member = salvo_core::effect_member_name(e, *i);
            let params = self.emit_member_param_list(f);
            let args: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| rs_ident(&p.name.name))
                .collect();
            let built = if args.is_empty() {
                format!("{msg}::{}", msg_variant_name(&member))
            } else {
                format!("{msg}::{}({})", msg_variant_name(&member), args.join(", "))
            };
            out.push_str(&format!(
                "    fn {}(&mut self{params}) {{\n        \
                 crate::scheduler::salvo_send(self.addr, Box::new({built}));\n    }}\n",
                rs_ident(&member)
            ));
        }
        out.push_str("}\n");
        out
    }

    /// [monitor-handler] [mixed-handler] [rs-monitor] The **shareable
    /// handle** of a plain effect, plus the monitor's lock adapter. What
    /// `Addr<E>` lowers to must cover two kinds of value — a monitor (a
    /// handler behind a lock) and a mixed handler's façade (an addr plus
    /// ctor params, whose sync members *wait*) — and the façade must NOT sit
    /// behind the monitor's mutex: a second caller blocked on it would serve
    /// nothing while the first waits inside, which wedges a shared
    /// single-threaded pool. So the handle is a **clone-boxed trait object**
    /// (`__Mon_E` over `Box<dyn __Share_E>`), and the lock is demoted into a
    /// per-effect generic adapter (`__Lock_E<H>`) that only monitor spawns
    /// wrap.
    ///
    /// The lock stays innermost by construction (the checker refused a
    /// monitor-spawned handler any dependencies), which is what keeps
    /// Rust's non-reentrant `Mutex` unobservable against the JVM's
    /// re-entrant monitor [backend-never-wrong].
    fn emit_monitor_stub(&mut self, e: &EffectDecl) -> String {
        if e.is_actor {
            return String::new();
        }
        // A plain effect's members are sync members (send fns need the actor
        // kind); a member with its own generics was refused and skipped by
        // the trait above, so it is skipped here to match.
        let members: Vec<(usize, &FnDecl)> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.is_send && f.generics.is_empty())
            .collect();
        let base = monitor_struct_name(&e.name.name);
        let share_base = share_trait_name(&e.name.name);
        let lock = lock_struct_name(&e.name.name);
        // [rs-monitor] Generic exactly as the effect is, so one declaration
        // serves every instance (`__Mon_Random<T>` for `Random<T>`) and a
        // stateful handler of a generic effect instance shares like any
        // other. `'static` because the handle boxes a trait object of the
        // instance — and every Salvo type is owned data with no lifetime of
        // its own, so the bound costs nothing.
        let g_params = if e.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                e.generics
                    .iter()
                    .map(|g| format!("{}: 'static", g.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let g_args: Vec<String> = e.generics.iter().map(|g| g.name.clone()).collect();
        let name = trait_type(&base, &g_args);
        let share = trait_type(&share_base, &g_args);
        let trait_name = trait_type(&e.name.name, &g_args);
        // The blanket impl's own parameter is `__H`, which an effect's
        // generics can never collide with [name-casing].
        let blanket_generics = if e.generics.is_empty() {
            format!("<__H: {trait_name} + Clone + Send + 'static>")
        } else {
            format!(
                "<{}, __H: {trait_name} + Clone + Send + 'static>",
                g_params.trim_start_matches('<').trim_end_matches('>')
            )
        };
        let mut out = format!(
            "\npub trait {share_base}{g_params}: {trait_name} + Send {{\n    \
             fn __clone_box(&self) -> Box<dyn {share}>;\n}}\n\n\
             impl{blanket_generics} {share} for __H {{\n    \
             fn __clone_box(&self) -> Box<dyn {share}> {{\n        \
             Box::new(self.clone())\n    }}\n}}\n\n\
             pub struct {base}{g_params} {{\n    inner: Box<dyn {share}>,\n}}\n\n\
             impl{g_params} Clone for {name} {{\n    fn clone(&self) -> Self {{\n        \
             Self {{ inner: self.inner.__clone_box() }}\n    }}\n}}\n\n\
             impl{g_params} {name} {{\n    pub fn new(inner: Box<dyn {share}>) -> Self {{\n        \
             Self {{ inner }}\n    }}\n}}\n\nimpl{g_params} {trait_name} for {name} {{\n"
        );
        let mut forwards = String::new();
        for (i, f) in &members {
            let member = self.member_name(e, *i);
            let params = format!(
                "{}{}",
                self.emit_member_param_list(f),
                self.emit_member_implicits(f)
            );
            let ret = self.emit_return_type(f.return_type.as_ref());
            let mut args: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| rs_ident(&p.name.name))
                .collect();
            args.extend(self.implicits_of(f).iter().map(|imp| rs_ident(&imp.name)));
            let args = args.join(", ");
            out.push_str(&format!(
                "    fn {member}(&mut self{params}){ret} {{\n        \
                 self.inner.{member}({args})\n    }}\n"
            ));
            forwards.push_str(&format!(
                "    fn {member}(&mut self{params}){ret} {{\n        \
                 self.inner.lock().unwrap().{member}({args})\n    }}\n"
            ));
        }
        out.push_str("}\n");
        // [rs-monitor] The lock adapter, generic over the handler so one
        // definition serves every monitor of this effect. The struct itself
        // is unbounded (bounds live on the trait impl, where the effect's
        // own generics can join them).
        let lock_impl_generics = if e.generics.is_empty() {
            format!("<H: {trait_name} + Send>")
        } else {
            format!(
                "<{}, H: {trait_name} + Send>",
                g_params.trim_start_matches('<').trim_end_matches('>')
            )
        };
        out.push_str(&format!(
            "\npub struct {lock}<H> {{\n    \
             inner: std::sync::Arc<std::sync::Mutex<H>>,\n}}\n\n\
             impl<H> Clone for {lock}<H> {{\n    \
             fn clone(&self) -> Self {{\n        \
             Self {{ inner: self.inner.clone() }}\n    }}\n}}\n\n\
             impl<H> {lock}<H> {{\n    \
             pub fn new(inner: H) -> Self {{\n        \
             Self {{ inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }}\n    }}\n}}\n\n\
             impl{lock_impl_generics} {trait_name} for {lock}<H> {{\n{forwards}}}\n"
        ));
        // [use-local] The handle is its own Has-provider: a handle-dep
        // handler's captured `__Mon_E` field satisfies the fused call
        // machinery directly, with a field-granular borrow (which is what
        // keeps the handler's own state usable in the same expression).
        // Under the fusion gate exactly as the `__Has_E` trait itself is:
        // plain mode declares no such trait, so the impl would dangle.
        if self.fusion {
            out.push_str(&emit_has_impl(&g_params, &name, &e.name.name, &g_args, "self"));
        }
        out
    }

    fn emit_message_enum(&mut self, e: &EffectDecl) -> String {
        // [actor-effect-kind] The kind is the gate: a plain effect never
        // becomes messages, and an actor one always does (its members are all
        // sends in this pass).
        if !e.is_actor {
            return String::new();
        }
        let sends: Vec<(usize, &FnDecl)> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_send)
            .collect();
        if sends.is_empty() {
            return String::new();
        }
        if !e.generics.is_empty() {
            // A generic protocol's message enum would have to be generic too,
            // and the runtime's `Any` box would then need the instantiation
            // at every downcast. Refused rather than guessed.
            self.error(format!(
                "effect `{}` is generic, which the rust backend cannot make a \
                 actor protocol of yet",
                e.name.name
            ));
            return String::new();
        }
        self.needs_scheduler = true;
        let mut out = format!("\npub enum {} {{\n", msg_enum_name(&e.name.name));
        for (i, f) in &sends {
            let payload: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                // A message *owns* its payload: it outlives the send.
                .map(|p| self.param_type(&p.ty, p.variadic, ParamMode::Owned))
                .collect();
            let variant = msg_variant_name(&salvo_core::effect_member_name(e, *i));
            if payload.is_empty() {
                out.push_str(&format!("    {variant},\n"));
            } else {
                out.push_str(&format!("    {variant}({}),\n", payload.join(", ")));
            }
        }
        out.push_str("}\n");
        out
    }

    /// [rs-actor] [actor-replyto] `__Cont_H`: the **parked continuation**,
    /// emitted beside the *handler* whose members it names
    /// [effect-handler-multi] — a mint is lexical, so the members are the
    /// handler's, and with several faces they come from several protocols.
    ///
    /// A `replyto k(captures)` stores one of these in the minting actor's
    /// `__parked` table under the slot the runtime handed back; when the
    /// answer arrives, `SalvoActor::resume` looks it up, and the variant is
    /// what says *which* member to call and therefore what type to downcast
    /// the answer to. So a variant carries the member's parameters **minus
    /// the trailing one**: that last parameter is the answer itself
    /// [actor-replyto], which arrives with the reply rather than being stored.
    ///
    /// A member with no parameters can never be a target (there is no answer
    /// for the token to carry), so it gets no variant.
    fn emit_cont_enum(&mut self, h: &HandlerDecl) -> String {
        // [mixed-handler] [rs-mixed] Handler-keyed for a mixed servant: the
        // variants are its own `send fn` members, named as `__Msg_H`'s are.
        if self.handler_is_mixed(h) {
            let targets = mixed_cont_targets(h);
            if targets.is_empty() {
                return String::new();
            }
            let mut out = format!("\npub enum {} {{\n", cont_enum_name(&h.name.name));
            for f in targets {
                let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                let captures: Vec<String> = fixed[..fixed.len() - 1]
                    .iter()
                    // A capture is *stored* in the continuation, so it is owned.
                    .map(|p| self.param_type(&p.ty, p.variadic, ParamMode::Owned))
                    .collect();
                let variant = msg_variant_name(&f.name.name);
                if captures.is_empty() {
                    out.push_str(&format!("    {variant},\n"));
                } else {
                    out.push_str(&format!("    {variant}({}),\n", captures.join(", ")));
                }
            }
            out.push_str("}\n");
            return out;
        }
        let targets = self.cont_targets(h);
        if targets.is_empty() {
            return String::new();
        }
        let mut out = format!("\npub enum {} {{\n", cont_enum_name(&h.name.name));
        for (e, i) in &targets {
            let Some(f) = e.fns.get(*i) else { continue };
            let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
            let captures: Vec<String> = fixed[..fixed.len() - 1]
                .iter()
                // A capture is *stored* in the continuation, so it is owned.
                .map(|p| self.param_type(&p.ty, p.variadic, ParamMode::Owned))
                .collect();
            let variant = msg_variant_name(&salvo_core::effect_member_name(e, *i));
            if captures.is_empty() {
                out.push_str(&format!("    {variant},\n"));
            } else {
                out.push_str(&format!("    {variant}({}),\n", captures.join(", ")));
            }
        }
        out.push_str("}\n");
        out
    }
    /// a unit struct implementing the generated trait, every member stubbed
    /// with `todo!`. The signatures come from [`Emitter::emit_effect`]'s own
    /// renderers, so a skeleton that drifts from the trait is impossible by
    /// construction.
    fn host_impl(&mut self, e: &EffectDecl, owner_path: &str) -> String {
        let name = host_struct(&e.name.name);
        let mut out = format!(
            "\npub struct {name};\n\nimpl {owner_path}::{} for {name} {{\n",
            rs_ident(&e.name.name)
        );
        for (i, f) in e.fns.iter().enumerate() {
            if !f.generics.is_empty() {
                continue;
            }
            let params = format!(
                "{}{}",
                self.emit_member_param_list(f),
                self.emit_member_implicits(f)
            );
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret} {{\n        \
                 todo!(\"implement {}.{}\")\n    }}\n",
                self.member_name(e, i),
                e.name.name,
                f.name.name
            ));
        }
        out.push_str("}\n");
        out
    }

    /// [platform-handler] [rs-platform-handler] One host *handler* skeleton: a
    /// struct named after the handler, with the handler's constructor
    /// parameters as its fields and a `new` taking them, implementing the
    /// generated trait of the ordinary effect it handles. The `use` site
    /// constructs exactly this — `HostX::new(args)` — so neither the name nor
    /// the constructor is the host's to choose.
    fn host_handler_impl(&mut self, h: &HandlerDecl, effect_path: &str) -> String {
        let of = self.emit_type(&h.of[0]);
        let Some(effect) = type_base_name(&h.of[0])
            .and_then(|n| self.symbols.effects.get(n))
            .copied()
        else {
            self.error(format!(
                "platform handler `{}` implements `{of}`, which is not a declared \
                 effect",
                h.name.name
            ));
            return String::new();
        };
        let name = rs_ident(&h.name.name);
        // [use-local] Stateless (bodyless, ctor params only), so a shareable
        // binding is bare and a seam clones it into a handle — `Clone` is
        // part of the thread-safety contract (user decision 2026-09-20:
        // platform/intrinsic handlers are assumed thread-safe; the written
        // contract is a follow-up).
        let mut out = format!("\n#[derive(Clone)]\npub struct {name} {{\n");
        for p in &h.params {
            let ty = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&p.name.name)));
        }
        out.push_str("}\n");
        let ctor_params: Vec<String> = h
            .params
            .iter()
            .map(|p| {
                format!(
                    "{}: {}",
                    rs_ident(&p.name.name),
                    self.param_type(&p.ty, p.variadic, ParamMode::Owned)
                )
            })
            .collect();
        out.push_str(&format!(
            "\nimpl {name} {{\n    pub fn new({}) -> Self {{\n        Self {{",
            ctor_params.join(", ")
        ));
        if h.params.is_empty() {
            out.push_str(" }\n    }\n}\n");
        } else {
            out.push('\n');
            for p in &h.params {
                out.push_str(&format!("            {},\n", rs_ident(&p.name.name)));
            }
            out.push_str("        }\n    }\n}\n");
        }
        out.push_str(&format!(
            "\nimpl {effect_path}::{} for {name} {{\n",
            rs_ident(&effect.name.name)
        ));
        for (i, f) in effect.fns.iter().enumerate() {
            if !f.generics.is_empty() {
                continue;
            }
            let params = format!(
                "{}{}",
                self.emit_member_param_list(f),
                self.emit_member_implicits(f)
            );
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret} {{\n        \
                 todo!(\"implement {}.{}\")\n    }}\n",
                self.member_name(effect, i),
                effect.name.name,
                f.name.name
            ));
        }
        out.push_str("}\n");
        out
    }

    /// [platform-tree] The platform effects `main` receives, as Salvo effect
    /// names *in parameter order* — the arguments the host's `main` must
    /// pass to the generated entry point. Read from the same checker table
    /// the parameters themselves come from, so the two cannot disagree.
    fn platform_entry_effects(&mut self, f: &FnDecl) -> Vec<String> {
        let checked_effects: Option<Vec<Ty>> = self
            .checked
            .fn_refs
            .get(&(self.file_idx, f.name.span))
            .and_then(|key| self.checked.fn_effects.get(key))
            .cloned();
        let mut out: Vec<String> = Vec::new();
        let push = |name: &str, out: &mut Vec<String>| {
            if !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        };
        match checked_effects {
            Some(tys) => {
                for ty in tys {
                    if let Ty::Named { name, .. } = ty.strip_quals() {
                        if self
                            .symbols
                            .effects
                            .get(name.as_str())
                            .is_some_and(|e| e.platform)
                        {
                            push(name, &mut out);
                        }
                    }
                }
            }
            None => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                        let name = r.name.name.as_str();
                        if self.symbols.effects.get(name).is_some_and(|e| e.platform) {
                            push(name, &mut out);
                        }
                    }
                }
            }
        }
        out
    }

    /// Effect/handler member parameters follow the default kept rule
    /// [rs-borrows]: scalars by value, everything else `&T`.
    fn emit_member_param_list(&mut self, member: &FnDecl) -> String {
        let mut out = String::new();
        for p in &member.params {
            if p.implicit {
                continue; // appended by `emit_member_implicits`, in order
            }
            let mode = self.member_param_mode(member, p);
            out.push_str(", ");
            out.push_str(&format!(
                "{}: {}",
                rs_ident(&p.name.name),
                self.param_type(&p.ty, p.variadic, mode)
            ));
        }
        out
    }

    /// [rs-borrows] [deduce-syntax] The mode of an **effect member's**
    /// parameter, from the member's own *written* deduction clause: a member
    /// has no body to infer from, so [decl-explicit] makes it mention every
    /// non-Copy parameter and the clause is the whole contract. A consumed
    /// parameter (`=> !s`) is therefore taken **by value**, a kept `Mut` one
    /// by `&mut`, a kept plain one by `&`.
    ///
    /// Until 2026-09-14 members used the default kept rule regardless, so a
    /// consuming member took `&T` and its body cloned — sound, but it made a
    /// *linear* token's `close` copy the token it was supposed to consume,
    /// and phase 4's `Fs` is built out of exactly those members.
    ///
    /// The same function answers for the trait method, every handler's
    /// implementation, the fusion's forwarding impls and the argument
    /// rendering at call sites, because a disagreement between any two of
    /// them is a rustc type error rather than something Salvo would notice.
    fn member_param_mode(&mut self, member: &FnDecl, p: &Param) -> ParamMode {
        if p.variadic || self.is_copy_ast_type(&p.ty) || is_fn_group(&p.ty) {
            return ParamMode::Owned;
        }
        if matches!(p.ty, Type::Fn { .. }) {
            // [fn-contract] Fn values are borrowed: `&mut impl FnMut`.
            return ParamMode::RefMut;
        }
        let moved = member.deductions.iter().flatten().any(|d| {
            d.param_name().is_some_and(|n| n.name == p.name.name)
                && matches!(
                    d.kind,
                    // [defer-deduction] A deferral is consumed like a move.
                    salvo_syntax::ast::DeductionKind::Moved
                        | salvo_syntax::ast::DeductionKind::Deferred
                )
        });
        if moved {
            ParamMode::Owned
        } else if type_has_mut(&p.ty) {
            ParamMode::RefMut
        } else {
            ParamMode::Ref
        }
    }

    /// [implicit-param] A member's implicit parameters, as its interface
    /// renders them. **`dyn`, not `impl`**: an effect trait is used as
    /// `&mut dyn E` [rs-effects], and `impl Trait` in argument position
    /// would make the trait not object-safe, so the one place a member's
    /// parameters are rendered has to dispatch dynamically. A plain fn keeps
    /// `impl FnMut` and monomorphises.
    fn emit_member_implicits(&mut self, f: &FnDecl) -> String {
        let implicits = self.implicits_of(f);
        let mut out = String::new();
        for imp in &implicits {
            let rendered = self.implicit_param_type_of(imp);
            out.push_str(&format!(", {}: {rendered}", rs_ident(&imp.name)));
        }
        out
    }

    /// [implicit-param] The implicit parameters of a fn or member: a
    /// top-level fn by its `FnKey`, a member (an effect member's signature,
    /// or a handler's implementation of one) by its own name span.
    fn implicits_of(&self, f: &FnDecl) -> Vec<salvo_core::ImplicitParam> {
        self.checked
            .fn_refs
            .get(&(self.file_idx, f.name.span))
            .and_then(|key| self.checked.implicit_params.get(key).cloned())
            .or_else(|| {
                self.checked
                    .implicit_members
                    .get(&(self.file_idx, f.name.span))
                    .cloned()
            })
            .unwrap_or_default()
    }

    fn emit_handler(&mut self, h: &HandlerDecl) -> String {
        if h.intrinsic {
            return self.emit_intrinsic_handler(h);
        }
        // [platform-handler] [rs-platform-handler] Nothing is emitted for a
        // platform handler: its struct is the host's, in the module's
        // `platform/` companion, and the `use` site constructs it through
        // `HostX::new(…)` (`handler_ctor_path`). The generated *trait* is the
        // effect's, emitted as any effect's is — which is what the host
        // struct implements.
        if h.platform {
            return String::new();
        }
        // [effect-handler-deps] Dependencies are the handler's own effect
        // list. For the **fusion** form the compiler supplies them per call,
        // so they are neither fields nor `new` parameters; for the
        // **handle-dep** form ([use-local], user decision 2026-09-20) they
        // are owned handles captured at construction — fields and trailing
        // `new` parameters, one per dep in declaration order.
        let deps: Vec<(String, Vec<String>)> = self.handler_dep_effects(h);
        let handle_dep = self.handler_is_handle_dep(h);
        if handle_dep {
            for (base, args) in &deps {
                if !args.is_empty() {
                    self.error(format!(
                        "handler `{}` depends on the generic effect instance \
                         `{base}<…>`, whose shared handle has no representation: \
                         declare the dependency `local {base}` and bind the handler \
                         `use local`",
                        h.name.name
                    ));
                    return String::new();
                }
            }
        }
        if !deps.is_empty() && !handle_dep && !self.fusion {
            // The gate is *exactly* "some handler declares a dependency",
            // so this is an internal inconsistency, not a language cut.
            self.error(format!(
                "internal: handler `{}` declares effect dependencies but the \
                 fusion emission is off",
                h.name.name
            ));
            return String::new();
        }
        let saved = self.enter_generics(&h.generics);
        let generics = self.emit_generic_params(&h.generics);
        let generic_args = self.emit_generic_args_plain(&h.generics);
        let name = rs_ident(&h.name.name);
        // Every constructor parameter is data now: dependencies moved to the
        // handler's effect list [effect-handler-deps].
        let own: Vec<Param> = h.params.to_vec();

        // Struct: own ctor params + state fields.
        //
        // [use-local] A **stateless shareable** handler derives `Clone`: a
        // seam turns the binding into a handle by boxing a clone (the
        // `__Share_E` blanket impl wants `Clone + Send`), and a stateless
        // clone is observationally the instance itself. Handle-dep fields
        // are `__Mon_E` (Clone by construction), data params are Salvo
        // types (Clone throughout).
        let shareable_stateless = h.state.is_empty()
            && !h.params.iter().any(|p| p.implicit)
            && !h.fns.iter().any(|f| f.is_send)
            && (deps.is_empty() || handle_dep)
            && !self.handler_is_actor(h);
        let mut out = if shareable_stateless {
            format!("\n#[derive(Clone)]\npub struct {name}{generics} {{\n")
        } else {
            format!("\npub struct {name}{generics} {{\n")
        };
        let mut field_types = String::new();
        for p in &own {
            // [copy-implicit] A fn-typed constructor parameter (an implicit
            // such as `?copy: (v: T) -> T`) is stored boxed: `impl Trait` is
            // not a field type, and the handler outlives the `use` that
            // built it. Called as `(self.copy)(…)`.
            let ty = if p.implicit {
                let rendered = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
                format!("Box<dyn {}>", rendered.trim_start_matches("impl "))
            } else {
                self.param_type(&p.ty, p.variadic, ParamMode::Owned)
            };
            field_types.push_str(&ty);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&p.name.name)));
        }
        for field in &h.state {
            let ty = self.emit_type(&field.ty);
            field_types.push_str(&ty);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&field.name.name)));
        }
        // [use-local] The handle-dep form's dependency fields, in
        // declaration order: the effect's shareable handle, owned.
        if handle_dep {
            for (base, _) in &deps {
                out.push_str(&format!(
                    "    __dep_{}: {},\n",
                    sanitize_ident(base),
                    self.effect_path(base, &monitor_struct_name(base))
                ));
            }
        }
        // [effect-handler-generics] A type parameter no field mentions is an
        // error in Rust (`E0392`) though the handler is perfectly well formed
        // in Salvo — a handler is a *behaviour*, and a generic one need hold
        // nothing. `PhantomData` is what says "generic over this, storing
        // none of it".
        let phantom: Vec<&Ident> = h
            .generics
            .iter()
            .filter(|g| !mentions_ident(&field_types, &g.name))
            .collect();
        for g in &phantom {
            out.push_str(&format!(
                "    __phantom_{}: std::marker::PhantomData<{}>,\n",
                g.name, g.name
            ));
        }
        // [rs-actor] [actor-self-send] [actor-replyto] The two generated
        // fields a handler of an `actor effect` carries, whichever way it is
        // bound (a handler is compiled once):
        //
        // * `__addr` — the address of the actor this instance *is*, written
        //   by `__Actor_H` from the activation's `SalvoCtx` at the start of
        //   every activation, and `None` when the instance was bound with
        //   `use` instead. That absence is the self-send's discriminator: a
        //   `k@self(…)` enqueues when it is present and calls the member
        //   inline when it is not (user decision 2026-09-15, D5-a).
        // * `__parked` — the parked-continuation table, slot → `__Cont_E`.
        //   Kept on the handler rather than on `__Actor_H` because the *mint*
        //   happens in a member body, which has `&mut self` on the handler and
        //   cannot see the actor struct; `resume` reaches it through
        //   `self.handler` (D5-b). Keeping it here is also what leaves
        //   `SalvoActor`'s decided signature untouched.
        // [mixed-handler] A mixed handler's servant is an actor too: its
        // send members run as activations, so it carries the same generated
        // fields — `__parked` included, keyed on its own send members
        // [defer-deduction].
        let mixed = self.handler_is_mixed(h);
        let actor_handler = self.handler_is_actor(h) || mixed;
        if actor_handler {
            // [rs-mailbox] [actor-mailbox] The mailbox bound, evaluated by the
            // constructor exactly as a state field's initialiser is — which is
            // what it is, structurally: an expression over constructor
            // parameters, computed once when the handler is built. The spawn
            // site reads it off the instance *before* handing it to the actor
            // body, so the scheduler learns the bound without the spawn line
            // saying it and without the arguments being evaluated twice.
            //
            // `pub`, because the spawn site need not be in this module: std's
            // own `DefaultTimer` is spawned from user code, and a private field
            // made that a raw rustc E0616 rather than a program that runs
            // [backend-never-wrong].
            out.push_str("    pub __mailbox_capacity: i32,\n");
            out.push_str("    __addr: Option<usize>,\n");
            if let Some(cont) = self.handler_cont_type(h) {
                out.push_str(&format!(
                    "    __parked: std::collections::HashMap<u64, {cont}>,\n"
                ));
            }
        }
        out.push_str("}\n");

        // Constructor: `new` takes ctor params owned (a `use` argument is
        // a move [deduce-infer]) and initializes state from defaults.
        // [use-local] Handle-dep handlers take their dependency handles as
        // trailing parameters, after the written ones, in declaration order.
        out.push_str(&format!("\nimpl{generics} {name}{generic_args} {{\n"));
        let mut ctor_params: Vec<String> = own
            .iter()
            .map(|p| {
                let ty = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
                // [copy-implicit] The adapter arrives as `impl FnMut + 'static`
                // and is boxed into the field.
                let ty = if p.implicit {
                    format!("{ty} + 'static")
                } else {
                    ty
                };
                format!("{}: {ty}", rs_ident(&p.name.name))
            })
            .collect();
        if handle_dep {
            for (base, _) in &deps {
                ctor_params.push(format!(
                    "__dep_{}: {}",
                    sanitize_ident(base),
                    self.effect_path(base, &monitor_struct_name(base))
                ));
            }
        }
        out.push_str(&format!(
            "    pub fn new({}) -> Self {{\n        Self {{\n",
            ctor_params.join(", ")
        ));
        for p in &own {
            if p.implicit {
                out.push_str(&format!(
                    "            {}: Box::new({}),\n",
                    rs_ident(&p.name.name),
                    rs_ident(&p.name.name)
                ));
            } else {
                out.push_str(&format!("            {},\n", rs_ident(&p.name.name)));
            }
        }
        for field in &h.state {
            let init = match &field.default {
                Some(expr) => self.emit_expr(expr),
                None => {
                    self.error(format!(
                        "handler state field `{}` has no initializer",
                        field.name.name
                    ));
                    "Default::default()".to_string()
                }
            };
            out.push_str(&format!(
                "            {}: {init},\n",
                rs_ident(&field.name.name)
            ));
        }
        for g in &phantom {
            out.push_str(&format!(
                "            __phantom_{}: std::marker::PhantomData,\n",
                g.name
            ));
        }
        // [use-local] The captured dependency handles.
        if handle_dep {
            for (base, _) in &deps {
                out.push_str(&format!("            __dep_{},\n", sanitize_ident(base)));
            }
        }
        // [rs-actor] `None` until an activation writes it: an instance that
        // never becomes an actor keeps it, and that is the marker.
        if actor_handler {
            let cap = self.mailbox_capacity_code(h);
            out.push_str(&format!("            __mailbox_capacity: {cap},\n"));
            out.push_str("            __addr: None,\n");
            if self.handler_cont_type(h).is_some() {
                out.push_str("            __parked: std::collections::HashMap::new(),\n");
            }
        }
        out.push_str("        }\n    }\n}\n");

        if mixed {
            // [mixed-handler] [rs-mixed] The split: send members are the
            // servant, emitted as inherent methods the actor body dispatches
            // to; sync members are the façade's and are emitted there —
            // nothing of them belongs on the handler struct, whose instance
            // lives behind the mailbox.
            out.push_str(&format!("\nimpl{generics} {name}{generic_args} {{\n"));
            for f in h.fns.iter().filter(|f| f.is_send) {
                out.push_str(&self.emit_fn_inner(f, FnStyle::HandlerMember(h), 1));
            }
            out.push_str("}\n");
            // [actor-replyto] [rs-mixed] The parked-continuation enum,
            // handler-keyed like the servant's protocol.
            out.push_str(&self.emit_cont_enum(h));
            out.push_str(&self.emit_facade(h));
            out.push_str(&self.emit_mixed_actor_parts(h));
            self.generics = saved;
            return out;
        }
        if deps.is_empty() || handle_dep {
            // [effect-handler-multi] One trait impl per face, each carrying the
            // members that face declares. A member that implements a same-named
            // member of two faces appears in both impls — Rust cannot share one
            // method between two traits, and the signatures are identical
            // wherever that is legal, so the body is emitted twice rather than
            // forwarded.
            //
            // [use-local] A handle-dep handler emits the independent shape;
            // its dependency access is wired in `emit_fn_inner` off the
            // captured `__Mon_E` fields, which are their own Has-providers.
            for (face, effect) in h.of.iter().zip(self.handler_faces(h)) {
                let of = self.emit_type(face);
                out.push_str(&format!(
                    "\nimpl{generics} {of} for {name}{generic_args} {{\n"
                ));
                for f in &h.fns {
                    if !self
                        .member_faces(h, f)
                        .iter()
                        .any(|(e, _)| std::ptr::eq(*e, effect))
                    {
                        continue;
                    }
                    out.push_str(&self.emit_fn_inner(f, FnStyle::HandlerMember(h), 1));
                }
                out.push_str("}\n");
            }
        } else {
            out.push_str(&self.emit_dependent_members(h, &deps));
        }
        // [actor-replyto] The parked-continuation enum, beside the handler
        // whose members it names.
        out.push_str(&self.emit_cont_enum(h));
        // [rs-actor] The body a `spawn` boxes, for a handler that can be
        // one: the message dispatch onto its members.
        out.push_str(&self.emit_actor_body(h, &deps));
        self.generics = saved;
        out
    }

    /// [rs-actor] `__Actor_H`: the `SalvoActor` a spawn hands the
    /// scheduler. It owns the handler instance — the actor's state *is* the
    /// handler's — and its `handle` downcasts the protocol's message enum and
    /// calls the member the variant names.
    ///
    /// A **dependent** handler's members do not take their dependencies from
    /// a scope: they take a fused value, one per member call
    /// ([rs-effect-fusion]). A `use` site builds that out of the effects
    /// around it; a child has no such scope, so the actor **owns** its
    /// dependencies — a generated flat provider `__Prov_H<__D0, …>`, one field
    /// per declared dependency in declaration order, implementing each
    /// dependency's Has-accessor trait. `handle` then does exactly what a
    /// fusion's forwarding impl does: build the Sized `__Deps_H` view over
    /// that one provider and call through `__Impl_H`.
    fn emit_actor_body(
        &mut self,
        h: &HandlerDecl,
        deps: &[(String, Vec<String>)],
    ) -> String {
        // [effect-handler-multi] One protocol per face, each with its own
        // message enum and its own dispatcher; one mailbox and one state serve
        // all of them.
        let faces: Vec<&'p EffectDecl> = self
            .handler_faces(h)
            .into_iter()
            .filter(|e| e.is_actor)
            .collect();
        if faces.len() != h.of.len() || faces.is_empty() || !h.generics.is_empty() {
            return String::new();
        }
        let sends: Vec<(&'p EffectDecl, Vec<(usize, &'p FnDecl)>)> = faces
            .iter()
            .map(|e| {
                let list: Vec<(usize, &'p FnDecl)> = e
                    .fns
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.is_send)
                    .collect();
                (*e, list)
            })
            .collect();
        // [actor-effect-kind] Only an actor protocol gets an actor body.
        if sends.iter().all(|(_, list)| list.is_empty()) {
            return String::new();
        }
        self.needs_scheduler = true;
        let name = rs_ident(&h.name.name);
        let proc_name = actor_struct_name(&h.name.name);
        // Per face: its message enum path, its trait path, and the name of the
        // dispatcher that runs its members. One face keeps the bare
        // `__dispatch`, which is what a single-protocol actor has always
        // emitted.
        let single = faces.len() == 1;
        let dispatchers: Vec<(String, String, String)> = faces
            .iter()
            .map(|e| {
                let effect_name = e.name.name.clone();
                let msg = msg_enum_name(&effect_name);
                let msg_path = self.effect_path(&effect_name, &msg);
                let trait_path = self.effect_path(&effect_name, &rs_ident(&effect_name));
                let dispatch = if single {
                    "__dispatch".to_string()
                } else {
                    format!("__dispatch_{}", sanitize_ident(&effect_name))
                };
                (msg_path, trait_path, dispatch)
            })
            .collect();
        // The provider: emitted only for a dependent handler, and generic in
        // its dependency instances so a construction and a forwarding stub
        // both fit without a `Box` (the fusion's own reason for preferring a
        // Sized generic to a `dyn` [rs-effect-fusion]).
        let dep_params: Vec<String> = (0..deps.len()).map(|i| format!("__D{i}")).collect();
        let mut out = String::new();
        // Three renderings of the same parameter list: bare on the struct and
        // on `new`, with the dependencies' own trait bounds on `__dispatch`
        // (which builds the `__Deps_H` view), and with `Send + 'static` added
        // on the `SalvoActor` impl (whose supertrait must be proved).
        let (proc_generics, proc_dispatch_generics, proc_impl_generics, prov_field, prov_ty) =
        if deps.is_empty() {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        } else {
            let prov = prov_struct_name(&h.name.name);
            let bounds: Vec<String> = deps
                .iter()
                .zip(&dep_params)
                .map(|((base, args), p)| format!("{p}: {}", trait_type(base, args)))
                .collect();
            let params = dep_params.join(", ");
            let prov_ty = format!("{prov}<{params}>");
            out.push_str(&format!("\npub struct {prov}<{params}> {{\n"));
            for (i, p) in dep_params.iter().enumerate() {
                // `pub`: the provider is declared beside its handler and
                // *built* at whichever spawn site supplies the dependencies.
                out.push_str(&format!("    pub __d{i}: {p},\n"));
            }
            out.push_str("}\n");
            for (i, (base, args)) in deps.iter().enumerate() {
                out.push_str(&emit_has_impl(
                    &format!("<{}>", bounds.join(", ")),
                    &prov_ty,
                    base,
                    args,
                    &format!("&mut self.__d{i}"),
                ));
            }
            // `SalvoActor: Send`, so the impl must prove it — which for a
            // generic provider means saying so.
            let send_bounds: Vec<String> = bounds
                .iter()
                .map(|b| format!("{b} + Send + 'static"))
                .collect();
            (
                format!("<{params}>"),
                format!("<{}>", bounds.join(", ")),
                format!("<{}>", send_bounds.join(", ")),
                format!("    prov: {prov_ty},\n"),
                prov_ty,
            )
        };
        let ctor_params = if deps.is_empty() {
            format!("handler: {name}")
        } else {
            format!("handler: {name}, prov: {prov_ty}")
        };
        let ctor_fields = if deps.is_empty() { "handler" } else { "handler, prov" };
        out.push_str(&format!(
            "\npub struct {proc_name}{proc_generics} {{\n    handler: {name},\n{prov_field}}}\n\
             \nimpl{proc_generics} {proc_name}{proc_generics} {{\n    \
             pub fn new({ctor_params}) -> Self {{\n        \
             Self {{ {ctor_fields} }}\n    }}\n}}\n"
        ));
        out.push_str(&format!(
            "\nimpl{proc_dispatch_generics} {proc_name}{proc_generics} {{\n"
        ));
        // [rs-actor] Member invocation lives in **one** place per protocol:
        // `handle` downcasts a message into it, `resume` rebuilds one from a
        // parked continuation plus the answer that just arrived. Factoring it
        // out is what keeps a dependent handler's `__Deps_H` view built once
        // [rs-effect-fusion].
        for ((effect, list), (msg_path, trait_path, dispatch)) in sends.iter().zip(&dispatchers) {
            out.push_str(&format!(
                "    fn {dispatch}(&mut self, msg: {msg_path}) {{\n"
            ));
            if !deps.is_empty() {
                // [rs-effect-fusion] The adapter: a Sized Has-implementing view
                // over the one provider field, exactly as a forwarding impl
                // builds one over `__outer`. Disjoint from `&mut self.handler`,
                // which is why both borrows can be live in the same call.
                out.push_str(&format!(
                    "        let mut __deps = __Deps_{name}{{ __p: &mut self.prov }};\n"
                ));
            }
            out.push_str("        match msg {\n");
            for (i, f) in list {
                let member = salvo_core::effect_member_name(effect, *i);
                let variant = msg_variant_name(&member);
                let params: Vec<String> = f
                    .params
                    .iter()
                    .filter(|p| !p.implicit)
                    .map(|p| rs_ident(&p.name.name))
                    .collect();
                let bind = if params.is_empty() {
                    String::new()
                } else {
                    format!("({})", params.join(", "))
                };
                // A dependent handler's members live in the generated
                // `__Impl_H` trait, not in `impl Effect for H`, and take the
                // fused value as their first argument after `self`.
                let mut call_args: Vec<String> = vec!["&mut self.handler".to_string()];
                if !deps.is_empty() {
                    call_args.push("&mut __deps".to_string());
                }
                call_args.extend(params);
                let path = if deps.is_empty() {
                    trait_path.clone()
                } else {
                    format!("__Impl_{name}")
                };
                out.push_str(&format!(
                    "            {msg_path}::{variant}{bind} => \
                     {path}::{}({}),\n",
                    rs_ident(&member),
                    call_args.join(", ")
                ));
            }
            out.push_str("        }\n    }\n");
        }
        out.push_str("}\n");
        // [rs-actor] [actor-self-send] Every activation writes the
        // actor's own address into the handler before running a member, so
        // `k@self(…)` and `replyto` can read it. Written per activation
        // rather than once at spawn because the addr only exists after
        // `salvo_spawn` allocates it, and the ctx carries it to every
        // delivery anyway.
        let cont = self.handler_cont_type(h);
        out.push_str(&format!(
            "\nimpl{proc_impl_generics} crate::scheduler::SalvoActor for \
             {proc_name}{proc_generics} {{\n    \
             fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {{\n        \
             self.handler.__addr = Some(_ctx.addr);\n"
        ));
        if single {
            let (msg_path, _, dispatch) = &dispatchers[0];
            out.push_str(&format!(
                "        let msg = *msg.downcast::<{msg_path}>().expect(\"message of this protocol\");\n        \
                 self.{dispatch}(msg);\n    }}\n"
            ));
        } else {
            // [effect-handler-multi] One mailbox carries every face's messages,
            // so the delivery asks each protocol in turn. `downcast` hands the
            // box back on a miss, which is what makes the chain possible at
            // all; falling off the end is a runtime that sent this actor a
            // message of no protocol it serves, which cannot happen.
            for (msg_path, _, dispatch) in &dispatchers {
                out.push_str(&format!(
                    "        let msg = match msg.downcast::<{msg_path}>() {{\n            \
                     Ok(__m) => return self.{dispatch}(*__m),\n            \
                     Err(__m) => __m,\n        }};\n"
                ));
            }
            out.push_str(
                "        let _ = msg;\n        \
                 unreachable!(\"a message of one of this actor's protocols\")\n    }\n",
            );
        }
        // [actor-replyto] The other half of the table: look the slot up, and
        // the variant says which member to resume and therefore what the
        // answer downcasts to — its *trailing* parameter's type.
        match &cont {
            None => out.push_str(
                "\n    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, \
                 _slot: u64, _value: crate::scheduler::SalvoMsg) {\n        \
                 unreachable!(\"this protocol has no continuation targets\")\n    }\n",
            ),
            Some(cont_path) => {
                out.push_str(&format!(
                    "\n    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, \
                     slot: u64, value: crate::scheduler::SalvoMsg) {{\n        \
                     self.handler.__addr = Some(_ctx.addr);\n        \
                     let Some(__cont) = self.handler.__parked.remove(&slot) else {{\n            \
                     return; // a reply whose continuation is gone: nothing to run\n        \
                     }};\n        match __cont {{\n"
                ));
                for ((effect, list), (msg_path, _, dispatch)) in sends.iter().zip(&dispatchers) {
                    for (i, f) in list {
                        let fixed: Vec<&Param> =
                            f.params.iter().filter(|p| !p.implicit).collect();
                        if fixed.is_empty() {
                            continue;
                        }
                        let member = salvo_core::effect_member_name(effect, *i);
                        let variant = msg_variant_name(&member);
                        let caps: Vec<String> = fixed[..fixed.len() - 1]
                            .iter()
                            .map(|p| rs_ident(&p.name.name))
                            .collect();
                        let bind = if caps.is_empty() {
                            String::new()
                        } else {
                            format!("({})", caps.join(", "))
                        };
                        let answer_ty = {
                            let last = fixed[fixed.len() - 1];
                            self.param_type(&last.ty, last.variadic, ParamMode::Owned)
                        };
                        let mut args: Vec<String> = caps.clone();
                        args.push(format!(
                            "*value.downcast::<{answer_ty}>().expect(\"the awaited answer\")"
                        ));
                        // [effect-handler-multi] The variant names a member of
                        // one face, so the message it rebuilds and the
                        // dispatcher it hands it to are that face's.
                        out.push_str(&format!(
                            "            {cont_path}::{variant}{bind} => \
                             self.{dispatch}({msg_path}::{variant}({})),\n",
                            args.join(", ")
                        ));
                    }
                }
                out.push_str("        }\n    }\n");
            }
        }
        out.push_str("}\n");
        out
    }

    /// [rs-actor] [actor-effect-kind] Does `h` implement an **`actor
    /// effect`**? The gate for the two generated fields and for the actor
    /// body: a plain effect is never actor-backed, so its handler carries
    /// nothing extra.
    fn handler_is_actor(&self, h: &HandlerDecl) -> bool {
        // [effect-handler-multi] Every face of a handler is of one kind, so any
        // of them answers.
        h.of.iter().any(|of| match of {
            Type::Named { base, .. } => self
                .symbols
                .effects
                .get(base.name.name.as_str())
                .is_some_and(|e| e.is_actor),
            _ => false,
        })
    }

    /// [actor-replyto] [effect-handler-multi] The `__Cont_H` type of the
    /// handler `h` — named after the **handler**, not the effect, because a
    /// mint is lexical: `replyto k(…)` targets a member of the handler it is
    /// written in, and with several faces those members come from several
    /// protocols. [mixed-handler] [rs-mixed] A mixed servant parks too, keyed
    /// on its own `send fn` members — no face declares them. `None` when no
    /// member could be a target (every send member is parameterless, so no
    /// answer could be carried), in which case no `__parked` table is
    /// emitted either.
    fn handler_cont_type(&mut self, h: &HandlerDecl) -> Option<String> {
        if self.handler_is_mixed(h) {
            if mixed_cont_targets(h).is_empty() {
                return None;
            }
            return Some(cont_enum_name(&h.name.name));
        }
        if !self.handler_is_actor(h) {
            return None;
        }
        if self.cont_targets(h).is_empty() {
            return None;
        }
        Some(cont_enum_name(&h.name.name))
    }

    /// [actor-replyto] The handler's members a continuation could target: a
    /// `send fn` with at least one parameter (the trailing one is the answer),
    /// paired with the face that declares it — which is what says *which*
    /// message enum `resume` rebuilds.
    fn cont_targets(&mut self, h: &HandlerDecl) -> Vec<(&'p EffectDecl, usize)> {
        let mut targets: Vec<(&'p EffectDecl, usize)> = Vec::new();
        for f in &h.fns {
            if !f.is_send || !f.params.iter().any(|p| !p.implicit) {
                continue;
            }
            if let Some(found) = self.member_faces(h, f).into_iter().next() {
                if found.0.is_actor {
                    targets.push(found);
                }
            }
        }
        targets
    }

    /// [rs-effect-fusion] The member bodies of a *dependent* handler. They
    /// cannot live in `impl Effect for H` — the trait signature has no room
    /// for the dependencies — so they move into a generated
    /// `trait __Impl_H` whose methods take the dependencies as one fused
    /// value. `&mut self` is kept, so `self.state` still works.
    ///
    /// Dependencies travel uniformly as a Has-bounded Sized generic
    /// (`__Fx: __Has_D1 + __Has_D2`), single dependency included (user
    /// decision 2026-09-14: consistency over a special case). The
    /// `__Deps_H` adapter is what makes such a value out of the fusion's
    /// single provider field.
    fn emit_dependent_members(
        &mut self,
        h: &HandlerDecl,
        deps: &[(String, Vec<String>)],
    ) -> String {
        let name = rs_ident(&h.name.name);
        let generics = self.emit_generic_params(&h.generics);
        let generic_args = self.emit_generic_args_plain(&h.generics);
        let dep_effects: Vec<(String, Vec<String>)> = deps.to_vec();
        let trait_name = format!("__Impl_{name}");
        let mut sigs = String::new();
        for f in &h.fns {
            sigs.push_str(&self.emit_fn_inner(f, FnStyle::DepMemberSig(h), 1));
        }
        // The generated trait is *not* generic: the fusion owns the handler
        // behind an opaque `__H` and never derives its type arguments, so it
        // could not supply one. A member signature or dependency that names
        // the handler's own generics is therefore a reported cut, not a
        // mis-emission [backend-never-wrong].
        for g in &h.generics {
            if mentions_ident(&sigs, &g.name) {
                self.error(format!(
                    "handler `{}` declares effect dependencies and uses its own \
                     generic parameters (`{}`) in a member signature — the rust \
                     backend cannot fuse that yet",
                    h.name.name, g.name
                ));
                return String::new();
            }
        }
        let mut out = String::new();
        out.push_str(&self.emit_deps_adapter(&name, &dep_effects));
        out.push_str(&format!("\npub trait {trait_name} {{\n{sigs}}}\n"));
        out.push_str(&format!(
            "\nimpl{generics} {trait_name} for {name}{generic_args} {{\n"
        ));
        for f in &h.fns {
            out.push_str(&self.emit_fn_inner(f, FnStyle::DepMember(h), 1));
        }
        out.push_str("}\n");
        out
    }

    /// [rs-effect-fusion] `__Deps_H`: a Sized view over one provider that
    /// implements the **Has-accessor trait** of every dependency of `H`, so
    /// a member's `__Fx: __Has_D1 + __Has_D2` parameter has a Sized value
    /// to bind. `__P` is the fusion's `dyn` provider; supertrait
    /// elaboration gives it the Has bounds the forwards need.
    fn emit_deps_adapter(&mut self, handler: &str, deps: &[(String, Vec<String>)]) -> String {
        let name = format!("__Deps_{handler}");
        // `pub __p`: the adapter is declared beside its handler but
        // *constructed* inside fusion impls in whichever module `use`s the
        // handler.
        let mut out =
            format!("\npub struct {name}<'a, __P: ?Sized> {{\n    pub __p: &'a mut __P,\n}}\n");
        for (base, args) in deps {
            let bound = has_trait_type(base, args);
            let body = format!(
                "{}::{}(&mut *self.__p)",
                has_trait_path(base, args),
                has_getter_name(base)
            );
            out.push_str(&emit_has_impl(
                &format!("<'a, __P: {bound} + ?Sized>"),
                &format!("{name}<'a, __P>"),
                base,
                args,
                &body,
            ));
        }
        out
    }

    /// [rs-effect-fusion] `impl Effect for <fusion or adapter>`: every
    /// member forwards, with the effect's own type parameters substituted
    /// by the instance's arguments.
    fn emit_forward_impl(
        &mut self,
        impl_generics: &str,
        self_ty: &str,
        effect_name: &str,
        effect_args: &[String],
        forward: &Forward,
    ) -> String {
        let Some(decl) = self.symbols.effects.get(effect_name).copied() else {
            self.error(format!(
                "internal: no effect `{effect_name}` to forward from a fusion"
            ));
            return String::new();
        };
        let saved_subst = std::mem::take(&mut self.type_subst);
        for (g, arg) in decl.generics.iter().zip(effect_args) {
            self.type_subst.insert(g.name.clone(), arg.clone());
        }
        let saved_generics = self.enter_generics(&decl.generics);
        let mut out = format!(
            "\nimpl{impl_generics} {} for {self_ty} {{\n",
            trait_type(effect_name, effect_args)
        );
        for (i, f) in decl.fns.iter().enumerate() {
            if !f.generics.is_empty() {
                continue; // already reported by `emit_effect`
            }
            let params = format!(
                "{}{}",
                self.emit_member_param_list(f),
                self.emit_member_implicits(f)
            );
            let ret = self.emit_return_type(f.return_type.as_ref());
            let member = self.member_name(decl, i);
            // [implicit-param] A forwarding impl passes the member's implicit
            // parameters straight through, like every other argument.
            let mut arg_names: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| {
                    let name = rs_ident(&p.name.name);
                    if self.forward_arg_needs_deref(p) {
                        format!("*{name}")
                    } else {
                        name
                    }
                })
                .collect();
            for imp in &self.implicits_of(f) {
                arg_names.push(format!("&mut *{}", rs_ident(&imp.name)));
            }
            let mut body = String::new();
            let Forward::Dependent { trait_name, deps } = forward;
            let _ = deps;
            body.push_str("        let Self { __outer, __h } = self;\n");
            // The adapter is unconditional (user decision 2026-09-14:
            // consistency over a single-dependency special case): a Sized
            // Has-implementing view over the one provider field.
            body.push_str(&format!(
                "        let mut __deps = __Deps_{}{{ __p: &mut **__outer }};\n",
                trait_name.trim_start_matches("__Impl_")
            ));
            let mut all = vec!["__h".to_string(), "&mut __deps".to_string()];
            all.extend(arg_names.iter().cloned());
            body.push_str(&format!(
                "        {trait_name}::{member}({})\n",
                all.join(", ")
            ));
            out.push_str(&format!(
                "    fn {member}(&mut self{params}){ret} {{\n{body}    }}\n"
            ));
        }
        out.push_str("}\n");
        self.type_subst = saved_subst;
        self.generics = saved_generics;
        out
    }

    /// [rs-effect-fusion] A forwarded member argument that the *trait*
    /// passes as `&T` but the handler's generated `__Impl_H` member takes
    /// **by value**: the effect declares the parameter as one of its own
    /// generics — borrowed at the declaration, since nothing is known about
    /// a `T` [rs-borrows] — while the instance binds that generic to a Copy
    /// scalar, which the handler's *concrete* member declaration therefore
    /// takes owned. The forward has to deref.
    ///
    /// Found 2026-09-14 while building interception, and pre-dating it: any
    /// dependent handler of a generic effect instance whose member parameter
    /// lands on a scalar (`handler Reporting(n: Note) of Store<Int>`) emitted
    /// a raw rustc E0308 — loud, never wrong, but a cut with no reason to
    /// exist.
    fn forward_arg_needs_deref(&mut self, p: &Param) -> bool {
        if p.variadic {
            return false; // a variadic is owned on both sides
        }
        let Type::Named { qualifiers, base } = &p.ty else {
            return false;
        };
        if !qualifiers.is_empty() || !base.args.is_empty() {
            return false;
        }
        self.type_subst
            .get(base.name.name.as_str())
            .is_some_and(|rendered| is_copy_rendered(rendered))
    }

    /// [effect-handler-deps] The effects `h` declares as dependencies, as
    /// (base name, rendered type arguments) in declaration order — the
    /// handler's own effect list since 2026-09-14 (user decision), which is
    /// why this needs no filtering: every entry is a dependency, and the
    /// checker has already refused `use` and `Throw` there.
    fn handler_dep_effects(&mut self, h: &HandlerDecl) -> Vec<(String, Vec<String>)> {
        let refs: Vec<TypeRef> = h
            .effects
            .iter()
            .flatten()
            .filter_map(|e| match e {
                // [effect-local] Dep locality is the checker's business; at
                // the fusion layer a `local E` dep threads like any other.
                EffectRef::Effect(r) | EffectRef::LocalEffect(r) => Some(r.clone()),
                EffectRef::Use(_) => None,
                // [actor-spawn-effect] A capability, not an effect type: no
                // handler parameter is threaded for it.
                EffectRef::Spawn(_) => None,
            })
            .collect();
        refs.iter()
            .map(|r| {
                let args: Vec<String> = r.args.iter().map(|a| self.emit_type(a)).collect();
                (r.name.name.clone(), args)
            })
            .collect()
    }

    /// The base name and rendered type arguments of a named type
    /// (`Random<Int>` -> `("Random", ["i32"])`).
    fn named_type_parts(&mut self, ty: &Type) -> Option<(String, Vec<String>)> {
        match ty {
            Type::Named { base, .. } => {
                let name = base.name.name.clone();
                let args: Vec<Type> = base.args.clone();
                let rendered = args.iter().map(|a| self.emit_type(a)).collect();
                Some((name, rendered))
            }
            _ => None,
        }
    }

    /// An `intrinsic handler` [backend-intrinsic]: a std handler whose
    /// members this backend implements directly — struct + `new()` + trait
    /// impl. The signatures come from the *effect* it implements (the
    /// handler declaration is bodyless), and the bodies from
    /// [`crate::intrinsics::handler_member`].
    fn emit_intrinsic_handler(&mut self, h: &HandlerDecl) -> String {
        // [effect-handler-multi] One face, like a platform handler's: the
        // members are the backend's, and the checker refuses a second.
        let of = self.emit_type(&h.of[0]);
        let Some(effect) = type_base_name(&h.of[0])
            .and_then(|n| self.symbols.effects.get(n))
            .copied()
        else {
            self.error(format!(
                "intrinsic handler `{}` implements `{of}`, which is not a declared \
                 effect",
                h.name.name
            ));
            return String::new();
        };
        let name = rs_ident(&h.name.name);
        // [use-local] Stateless (bodyless, ctor params only), so a shareable
        // binding is bare and a seam clones it into a handle — `Clone` is
        // part of the thread-safety contract (user decision 2026-09-20:
        // platform/intrinsic handlers are assumed thread-safe; the written
        // contract is a follow-up).
        let mut out = format!("\n#[derive(Clone)]\npub struct {name} {{\n");
        for p in &h.params {
            let ty = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&p.name.name)));
        }
        out.push_str("}\n");
        let ctor_params: Vec<String> = h
            .params
            .iter()
            .map(|p| {
                format!(
                    "{}: {}",
                    rs_ident(&p.name.name),
                    self.param_type(&p.ty, p.variadic, ParamMode::Owned)
                )
            })
            .collect();
        out.push_str(&format!(
            "\nimpl {name} {{\n    pub fn new({}) -> Self {{\n        Self {{",
            ctor_params.join(", ")
        ));
        if h.params.is_empty() {
            out.push_str(" }\n    }\n}\n");
        } else {
            out.push('\n');
            for p in &h.params {
                out.push_str(&format!("            {},\n", rs_ident(&p.name.name)));
            }
            out.push_str("        }\n    }\n}\n");
        }
        out.push_str(&format!("\nimpl {of} for {name} {{\n"));
        for (i, member) in effect.fns.iter().enumerate() {
            let params = format!(
                "{}{}",
                self.emit_member_param_list(member),
                self.emit_member_implicits(member)
            );
            let ret = self.emit_return_type(member.return_type.as_ref());
            let arg_names: Vec<String> = member
                .params
                .iter()
                .map(|p| rs_ident(&p.name.name))
                .collect();
            let Some(body) =
                crate::intrinsics::handler_member(&h.name.name, &member.name.name, &arg_names)
            else {
                self.error(format!(
                    "intrinsic handler `{}` has no rust lowering for member `{}`",
                    h.name.name, member.name.name
                ));
                continue;
            };
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret} {{\n",
                self.member_name(effect, i)
            ));
            for line in body.lines() {
                out.push_str(&format!("        {line}\n"));
            }
            out.push_str("    }\n");
        }
        out.push_str("}\n");
        out
    }

    /// [effect-handler-multi] The effects a handler implements, as declarations
    /// and in declaration order. A face whose name is not a declared effect is
    /// dropped here and reported where it is used.
    fn handler_faces(&mut self, h: &HandlerDecl) -> Vec<&'p EffectDecl> {
        h.of
            .iter()
            .filter_map(|of| type_base_name(of))
            .filter_map(|n| self.symbols.effects.get(n).copied())
            .collect()
    }

    /// [effect-handler-multi] The faces that declare the member `f` implements,
    /// each with the member's index in that face. Several when one method
    /// implements a same-named member of two effects — legal only when the
    /// signatures are the ones overloading cannot tell apart, which is to say
    /// identical, so any of them describes it.
    fn member_faces(&mut self, h: &HandlerDecl, f: &FnDecl) -> Vec<(&'p EffectDecl, usize)> {
        salvo_core::handler_member_faces(&self.handler_faces(h), f)
    }

    /// [effect-member-overload] The effect member a handler's member
    /// implements, matched by name and written parameter types.
    fn effect_member_of(&mut self, h: &HandlerDecl, f: &FnDecl) -> Option<&'p FnDecl> {
        let (effect, idx) = self.member_faces(h, f).into_iter().next()?;
        effect.fns.get(idx)
    }

    /// [effect-member-overload] The emitted name of a *handler's* member: the
    /// name of the effect member it implements. Matched by name and written
    /// parameter types (`salvo_core::effect_member_index`), which is what
    /// tells two overloads apart.
    fn handler_member_name(&mut self, h: &HandlerDecl, f: &FnDecl) -> String {
        match self.member_faces(h, f).into_iter().next() {
            Some((e, i)) => self.member_name(e, i),
            None => rs_ident(&f.name.name),
        }
    }

    /// [effect-member-overload] The emitted name of an effect member: the
    /// declared name, unless the effect *overloads* it, in which case every
    /// occurrence after the first is suffixed. Rust cannot overload a trait
    /// method at all, so this is not a preference; the rule is
    /// `salvo_core`'s so that the trait, every handler impl, the forwarding
    /// impls, the skeletons and the call sites cannot disagree — and so that
    /// the Kotlin backend picks the same names [kt-fn-mangling].
    fn member_name(&self, effect: &EffectDecl, idx: usize) -> String {
        rs_ident(&salvo_core::effect_member_name(effect, idx))
    }

    /// [effect-member-overload] The emitted name of the member a *call*
    /// resolved to: the checker records which overload
    /// (`Checked::effect_member_calls`), and where it did not the name is
    /// declared once, so the name itself answers.
    fn called_member_name(&mut self, effect: &str, name: &str, span: Span) -> String {
        let Some(decl) = self.symbols.effects.get(effect).copied() else {
            return rs_ident(name);
        };
        match self.checked.effect_member_calls.get(&(self.file_idx, span)) {
            Some(&idx) => self.member_name(decl, idx),
            None => match salvo_core::effect_members_named(decl, name).as_slice() {
                [_] | [] => rs_ident(name),
                _ => {
                    self.error(format!(
                        "internal: `{name}` is overloaded on effect `{effect}` and \
                         the checker recorded no resolution for this call"
                    ));
                    rs_ident(name)
                }
            },
        }
    }

    fn emit_fn(&mut self, f: &FnDecl) -> String {
        self.emit_fn_inner(f, FnStyle::TopLevel, 0)
    }

    /// [spawn-inherit] [rs-handle-bundle] The hidden handle-bundle parameter
    /// of `f`, when the checker recorded handle requirements for it: a
    /// generated struct with one `__Mon_E` field per required effect, in the
    /// fn's declared effect order, deduped per *shape* exactly as the `__Fx_N`
    /// fusions are — so two fns needing the same handle set share one struct
    /// and a caller can forward its own bundle unchanged.
    ///
    /// One parameter however many handles (user modification, 2026-09-20).
    /// Registers the fields in `handle_fields` so a capture inside the body
    /// reads `__hs.<field>` where a lexically-bound one reads its eager
    /// handle variable.
    fn handle_bundle_param(&mut self, f: &FnDecl) -> Option<String> {
        let key = self.key_of_fn(f)?;
        let needs = self.checked.handle_requirements.get(&key)?.clone();
        if needs.is_empty() {
            return None;
        }
        let mut fields: Vec<(String, String)> = Vec::new();
        for ty in &needs {
            let rendered = self.rust_ty(ty);
            let (base, args) = self.ty_effect_parts(ty);
            let mon = self.effect_path(&base, &monitor_struct_name(&base));
            let handle_ty = if args.is_empty() {
                mon
            } else {
                format!("{mon}<{}>", args.join(", "))
            };
            fields.push((effect_param_name(&rendered), handle_ty));
        }
        let shape: Vec<String> = fields
            .iter()
            .map(|(n, t)| format!("{n}: {t}"))
            .collect();
        let shape_key = shape.join(", ");
        let name = match self.handle_bundles.get(&shape_key) {
            Some(name) => name.clone(),
            None => {
                let name = format!("__Hs_{}", self.handle_bundles.len() + 1);
                self.handle_bundles.insert(shape_key, name.clone());
                self.generated_items.push(format!(
                    "\npub struct {name} {{\n{}}}\n",
                    fields
                        .iter()
                        .map(|(n, t)| format!("    pub {n}: {t},\n"))
                        .collect::<String>()
                ));
                name
            }
        };
        let var = self.unique_name("__hs".to_string());
        self.handle_fields = needs
            .iter()
            .zip(fields.iter())
            .map(|(ty, (field, _))| (self.rust_ty(ty), format!("{var}.{field}")))
            .collect();
        self.bindings.insert(var.clone(), BindKind::Ref);
        Some(format!("{var}: &{name}"))
    }

    /// [spawn-inherit] [rs-handle-bundle] The bundle argument a call site
    /// passes: the callee's required handles, taken from this frame's own
    /// bundle where the effect is signature-supplied here too, and from the
    /// binding's eager handle variable where it is bound in this function.
    /// `None` when the callee needs none.
    fn handle_bundle_arg(&mut self, callee: salvo_core::FnKey) -> Option<String> {
        let needs = self.checked.handle_requirements.get(&callee)?.clone();
        if needs.is_empty() {
            return None;
        }
        let mut fields: Vec<String> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        for ty in &needs {
            let rendered = self.rust_ty(ty);
            let field = effect_param_name(&rendered);
            // A handle this frame already holds — either its own bundle
            // field, or the eager handle variable a binding here minted.
            let source = self
                .handle_fields
                .get(&rendered)
                .cloned()
                .map(|place| format!("{place}.clone()"))
                .or_else(|| {
                    self.effect_entry_by_ty(ty)
                        .and_then(|e| e.handle_var.clone())
                        .map(|hv| format!("{hv}.clone()"))
                });
            match source {
                Some(code) => {
                    fields.push(format!("{field}: {code}"));
                    names.push(format!("{field}: {}", self.handle_type_of(ty)));
                }
                None => {
                    self.error(format!(
                        "internal: no handle in scope for `{rendered}`, which the \
                         callee captures at construction"
                    ));
                    fields.push(format!("{field}: todo!()"));
                    names.push(format!("{field}: {}", self.handle_type_of(ty)));
                }
            }
        }
        let shape_key = names.join(", ");
        let name = match self.handle_bundles.get(&shape_key) {
            Some(name) => name.clone(),
            None => {
                // The callee's own emission mints the struct; a call emitted
                // first registers the same shape under the same key.
                let name = format!("__Hs_{}", self.handle_bundles.len() + 1);
                self.handle_bundles.insert(shape_key, name.clone());
                let mut decl = String::new();
                for ty in &needs {
                    let rendered = self.rust_ty(ty);
                    let handle_ty = self.handle_type_of(ty);
                    decl.push_str(&format!(
                        "    pub {}: {handle_ty},\n",
                        effect_param_name(&rendered)
                    ));
                }
                self.generated_items
                    .push(format!("\npub struct {name} {{\n{decl}}}\n"));
                name
            }
        };
        Some(format!("&{name} {{ {} }}", fields.join(", ")))
    }

    /// [spawn-inherit] [rs-handle-bundle] Where this frame reads the handle
    /// of `ty`, if it has one: a field of its own hidden bundle parameter.
    fn handle_place(&mut self, ty: &Ty) -> Option<String> {
        let rendered = self.rust_ty(ty);
        self.handle_fields.get(&rendered).cloned()
    }

    /// [rs-handle-bundle] The rendered handle type of an effect instance:
    /// the effect's `__Mon_E`, at its instantiation.
    fn handle_type_of(&mut self, ty: &Ty) -> String {
        let (base, args) = self.ty_effect_parts(ty);
        let mon = self.effect_path(&base, &monitor_struct_name(&base));
        if args.is_empty() {
            mon
        } else {
            format!("{mon}<{}>", args.join(", "))
        }
    }

    /// [qual-overload] The emitted name of a qualifier member. Rust has no
    /// overloading, and qualifiers are erased [qual-erasure], so two
    /// same-named qualifiers over different subjects would both emit
    /// `Q_qualifies` and collide (E0428). The subject's base name
    /// disambiguates — and only when it has to, the way fn mangling only
    /// fires on a real collision [rs-fn-mangling].
    fn qualifier_member_name(&self, q: &QualifierDecl, member: &str) -> String {
        let overloaded = self
            .symbols
            .qualifiers
            .get(q.name.name.as_str())
            .is_some_and(|ds| ds.len() > 1);
        match (overloaded, salvo_core::refine::of_base(&q.of, &q.generics)) {
            (true, Some(subject)) => format!("{}__{subject}_{member}", q.name.name),
            _ => format!("{}_{member}", q.name.name),
        }
    }

    /// [qual-overload] Which same-named qualifier a subject means, by base
    /// type name — the same syntactic comparison the checker's resolution and
    /// the refinement matcher make.
    fn qualifier_for_subject(
        &self,
        name: &str,
        subject: Option<&salvo_core::types::Ty>,
    ) -> Option<&'p QualifierDecl> {
        let candidates = self.symbols.qualifiers.get(name)?;
        match candidates.as_slice() {
            [] => None,
            [one] => Some(*one),
            many => {
                let base = match subject?.strip_quals() {
                    salvo_core::types::Ty::Named { name, .. } => name.clone(),
                    salvo_core::types::Ty::Array(_) => "[]".to_string(),
                    _ => return None,
                };
                many.iter()
                    .copied()
                    .find(|d| {
                        salvo_core::refine::of_base(&d.of, &d.generics)
                            .is_some_and(|of| of == base)
                    })
                    .or_else(|| candidates.first().copied())
            }
        }
    }

    /// A predicate qualifier's `qualifies` fns become top-level
    /// `pub fn Q_qualifies(...)` [is-qualifies].
    fn emit_qualifier(&mut self, q: &QualifierDecl) -> String {
        let mut out = String::new();
        for f in &q.fns {
            if f.body.is_none() {
                continue;
            }
            let mut renamed = f.clone();
            renamed.name.name = self.qualifier_member_name(q, &f.name.name);
            renamed.generics = q
                .generics
                .iter()
                .cloned()
                .chain(f.generics.iter().cloned())
                .collect();
            out.push_str(&self.emit_fn_inner(&renamed, FnStyle::QualifierFn, 0));
        }
        out
    }

    /// The parameter mode of one top-level-fn parameter [rs-borrows]:
    /// deduction-driven (omitted = moved = by value; kept = borrowed,
    /// `&mut` when the declared type carries `Mut`), with the Copy-scalar
    /// and variadic exceptions.
    fn param_mode(&mut self, key: Option<salvo_core::FnKey>, param: &Param) -> ParamMode {
        if param.variadic {
            return ParamMode::Owned; // callers assemble a fresh Vec
        }
        if self.is_copy_ast_type(&param.ty) {
            return ParamMode::Owned;
        }
        if is_fn_group(&param.ty) {
            return ParamMode::Owned; // `once` closures pass by value
        }
        if matches!(param.ty, Type::Fn { .. }) {
            // [rs-fn-field] A callback the callee **stores** arrives owned (and
            // `Rc`-shared internally) rather than borrowed for the call: a
            // composed pass calls it once per element, long after this returns.
            if key.is_some_and(|k| self.owns_callbacks(k)) {
                return ParamMode::Owned;
            }
            // [fn-contract] Fn values are borrowed: `&mut impl FnMut`.
            return ParamMode::RefMut;
        }
        let kept = key
            .and_then(|k| self.checked.deductions.get(&k))
            .and_then(|ds| ds.iter().find(|d| d.param == param.name.name))
            .map(|d| d.kept)
            .unwrap_or(true); // default kept (lenient, like deduce.rs)
        if !kept {
            return ParamMode::Owned;
        }
        if type_has_mut(&param.ty) {
            ParamMode::RefMut
        } else {
            ParamMode::Ref
        }
    }

    /// The default kept rule for fns outside the deduction tables
    /// (qualifier/effect/handler members) [rs-borrows].
    fn default_param_mode(&mut self, ty: &Type, variadic: bool) -> ParamMode {
        if variadic || self.is_copy_ast_type(ty) || is_fn_group(ty) {
            return ParamMode::Owned;
        }
        if matches!(ty, Type::Fn { .. }) {
            // [fn-contract] Fn values are borrowed: `&mut impl FnMut`.
            return ParamMode::RefMut;
        }
        if type_has_mut(ty) {
            ParamMode::RefMut
        } else {
            ParamMode::Ref
        }
    }

    /// Renders a parameter's Rust type for its mode.
    fn param_type(&mut self, ty: &Type, variadic: bool, mode: ParamMode) -> String {
        let base = if variadic {
            match ty {
                Type::Array { elem, .. } => format!("Vec<{}>", self.emit_type(elem)),
                other => self.emit_type(other),
            }
        } else {
            self.emit_type(ty)
        };
        // [proj-type] A `proj`-typed parameter is already the reference its
        // type renders as; a kept mode adds no second `&`. A `proj` over a
        // bare generic renders owned (`T` — the instantiation carries the
        // borrow, see `emit_type`), so its mode still applies.
        if strip_top_proj_ast(ty).is_some() && !variadic && base.starts_with('&') {
            return base;
        }
        match mode {
            ParamMode::Owned => base,
            ParamMode::Ref => format!("&{base}"),
            ParamMode::RefMut => format!("&mut {base}"),
        }
    }

    // ================= functions =================

    /// [op-order] [op-equality] A comparison that goes through the capability's
    /// function: the call, then the operator applied to its answer.
    ///
    /// The call itself is emitted by the ordinary paths — an intrinsic through
    /// its lowering, anything else as a named call with its own parameter modes
    /// — so a canonical, a generated structural member [cmp-auto] and an
    /// intrinsic (`cmp(Str, Str)`, which is how `Str` gets code-point order on
    /// both backends) all need no special handling here.
    fn emit_compare_via(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        via: &salvo_core::CompareVia,
        span: Span,
    ) -> String {
        let args: Vec<&Expr> = vec![lhs, rhs];
        let call = match via {
            salvo_core::CompareVia::Call(key) => match self.fn_by_key(*key) {
                Some(decl) => {
                    let name = decl.name.name.clone();
                    if decl.intrinsic {
                        self.emit_intrinsic_call(decl, &args, span)
                    } else {
                        self.emit_fn_call(&name, decl, Some(*key), &args, &[], span)
                    }
                }
                None => {
                    self.error(
                        "internal: the `cmp`/`eq` a comparison resolved to is not available"
                            .to_string(),
                    );
                    "todo!()".to_string()
                }
            },
            // [implicit-forward] Through the enclosing fn's own parameter, which
            // is the only thing that can compare an opaque `T`. The arguments
            // follow the *position's* convention [rs-fn-param-convention].
            salvo_core::CompareVia::Implicit(name) => {
                let by_ref: Vec<bool> = self
                    .implicits
                    .iter()
                    .find(|i| i.name == *name)
                    .map(|i| match i.ty.strip_quals() {
                        Ty::Fn { params, .. } => {
                            params.iter().map(|p| !Self::is_copy_ty(p)).collect()
                        }
                        _ => Vec::new(),
                    })
                    .unwrap_or_default();
                let code: Vec<String> = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        if by_ref.get(i).copied().unwrap_or(false) {
                            match self.borrow_value(a) {
                                Some(b) => b,
                                None => format!("&{}", self.emit_owned(a)),
                            }
                        } else {
                            self.emit_owned(a)
                        }
                    })
                    .collect();
                format!("{}({})", rs_ident(name), code.join(", "))
            }
        };
        match op {
            BinaryOp::Eq => call,
            BinaryOp::NotEq => format!("!{call}"),
            other => format!("{call} {} 0", binary_op(other)),
        }
    }

    /// [cmp-auto] [rs-cmp-groups] A canonical generated by a `default`
    /// obligation: the host's own derived operation, wrapped in the fn the rest
    /// of the compiler resolved. Nothing else in the backend learns that
    /// `default` exists — a call, an adapter closure and a `cmp = cmp@Point`
    /// value all reach this like any named fn.
    ///
    /// The derive it stands on is the one the `default` clause asks for (user
    /// decision 2026-09-21: `default` inherits the derive-based lowering that
    /// `canbe ordered`/`canbe hashed` used to drive, hand-written implementations
    /// do not), so the generated member and the struct's own `Ord`/`Hash` cannot
    /// disagree.
    fn emit_structural_fn(&mut self, f: &FnDecl) -> String {
        let key = self.key_of_fn(f);
        let member = f.name.name.clone();
        let param = f.params.first().cloned();
        let Some(param) = param else {
            self.error(format!("internal: structural `{member}` has no parameter"));
            return String::new();
        };
        let ty = self.emit_type(&param.ty);
        let mode = self.param_mode(key, &param);
        let arg = |name: &str| match mode {
            ParamMode::Owned => format!("&{name}"),
            _ => name.to_string(),
        };
        let rendered = |name: &str| match mode {
            ParamMode::Owned => format!("{name}: {ty}"),
            ParamMode::Ref => format!("{name}: &{ty}"),
            ParamMode::RefMut => format!("{name}: &mut {ty}"),
        };
        // A generic struct's derive carries bounds, so the fn over it must too:
        // `#[derive(Ord)] struct Box<T>` is `impl<T: Ord> Ord for Box<T>`.
        let bound = match member.as_str() {
            "cmp" => "Ord",
            "eq" => "PartialEq",
            _ => "std::hash::Hash",
        };
        let generics = if f.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                f.generics
                    .iter()
                    .map(|g| format!("{}: Clone + {bound}", g.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let name = self.rust_fn_name(f);
        match member.as_str() {
            // `Ordering` is a fieldless `#[repr(i8)]` enum whose discriminants
            // are Salvo's sign convention, so the cast is the lowering.
            "cmp" => format!(
                "\npub fn {name}{generics}({}, {}) -> i32 {{\n    (Ord::cmp({}, {}) as i32)\n}}\n",
                rendered("a"),
                rendered("b"),
                arg("a"),
                arg("b")
            ),
            "eq" => format!(
                "\npub fn {name}{generics}({}, {}) -> bool {{\n    ({} == {})\n}}\n",
                rendered("a"),
                rendered("b"),
                arg("a"),
                arg("b")
            ),
            // [cmp-hash-values] This host's digest, one hasher per call.
            _ => format!(
                "\npub fn {name}{generics}({}) -> i64 {{\n    let mut __h = std::hash::DefaultHasher::new();\n    std::hash::Hash::hash({}, &mut __h);\n    (std::hash::Hasher::finish(&__h) as i64)\n}}\n",
                rendered("value"),
                arg("value")
            ),
        }
    }

    fn emit_fn_inner<'a>(&mut self, f: &FnDecl, style: FnStyle<'a>, indent: usize) -> String {
        // [cmp-auto] A generated structural member has no body to emit: its
        // body *is* the host's derived operation.
        if f.structural {
            return self.emit_structural_fn(f);
        }
        let Some(body) = &f.body else {
            return String::new();
        };
        // [rs-fn-field] A fn that *stores* its callback renders fn-typed
        // parameters and implicits as owned `Fn + 'static`: the struct it hands
        // back outlives the call, so a borrow could not survive.
        let stores_callback = matches!(style, FnStyle::TopLevel)
            && self.key_of_fn(f).is_some_and(|k| self.owns_callbacks(k));
        let saved_in_iterator = std::mem::replace(&mut self.in_iterator_fn, stores_callback);
        let saved_generics = self.enter_generics(&f.generics);
        let saved_env = std::mem::take(&mut self.effect_env);
        let saved_bindings = std::mem::take(&mut self.bindings);
        // [spawn-inherit] The handle-bundle places are this fn's own: a
        // sibling's bundle field is not in scope here.
        let saved_handle_fields = std::mem::take(&mut self.handle_fields);
        self.borrowed_arm_locals.clear();
        let saved_mutated = std::mem::take(&mut self.mutated);
        let saved_derived = self.derived_return_fn;
        collect_mutated(body, &mut self.mutated);
        let saved_taken = std::mem::take(&mut self.taken_names);
        for p in &f.params {
            self.taken_names.insert(p.name.name.clone());
        }
        collect_declared(body, &mut self.taken_names);

        let top_level = matches!(style, FnStyle::TopLevel);
        let is_main = top_level && f.name.name == "main";
        let fn_key = if top_level { self.key_of_fn(f) } else { None };
        // [rs-throw-controlflow] A fn declaring `[Throw<M>]` returns
        // `ControlFlow<M, T>`: the throw *is* the return, so intermediate
        // frames stay silent (no handler, no dispatch, no allocation).
        let saved_throw = std::mem::replace(
            &mut self.throw_message,
            fn_key
                .and_then(|key| self.checked.fn_effects.get(&key))
                .and_then(|effects| effects.iter().find_map(throw_message_of)),
        );
        let saved_try_frames = std::mem::take(&mut self.try_frames);
        let sig_only = matches!(style, FnStyle::DepMemberSig(..));
        let saved_fn = std::mem::replace(&mut self.current_fn, f.name.name.clone());
        let saved_hoist = std::mem::replace(&mut self.hoist_id, 0);

        // Handler members see ctor params and state as `self.` fields. A
        // dependency is not a field at all under the fusion
        // [rs-effect-fusion]: it arrives as the fused parameter, and it has
        // no name in the source either [effect-handler-deps].
        let handler_of_style = match style {
            FnStyle::HandlerMember(h) => Some(h),
            FnStyle::DepMember(h) | FnStyle::DepMemberSig(h) => Some(h),
            _ => None,
        };
        // [actor-replyto] [actor-self-send] Both forms resolve against the
        // enclosing handler, so the body needs to know which one it is in.
        let saved_handler = std::mem::replace(
            &mut self.current_handler,
            handler_of_style.map(|h| h.name.name.clone()),
        );
        if let Some(h) = handler_of_style {
            for p in &h.params {
                self.bindings
                    .insert(p.name.name.clone(), BindKind::SelfField);
            }
            for field in &h.state {
                self.bindings
                    .insert(field.name.name.clone(), BindKind::SelfField);
            }
            // [use-local] A handle-dep handler's members reach their
            // dependencies through the captured fields: `__Mon_E` is its
            // own Has-provider, so the environment points straight at
            // `self.__dep_e`, a field-granular borrow that leaves the
            // handler's own state usable beside it.
            if matches!(style, FnStyle::HandlerMember(_)) && self.handler_is_handle_dep(h) {
                for (base, args) in self.handler_dep_effects(h) {
                    self.effect_env.push(EffectEntry {
                        ty: None,
                        key: trait_type(&base, &args),
                        var: format!("self.__dep_{}", sanitize_ident(&base)),
                        is_local: true,
                        handle_var: None,
                    });
                }
            }
        }

        // [rs-borrows] The blanket `Clone` bound, plus `'static`: every
        // Salvo type is owned data with no lifetime of its own, and a lazy
        // iterator's captured state outlives the call [rs-iter-pass].
        let mut generic_parts: Vec<String> = f
            .generics
            .iter()
            // [rs-proj-arm] No `'static`: a borrowed element (`&'s T`) flows
            // through generic fns like `emitted`, and `'static` on `T` would
            // demand `'s: 'static`. The bound's original reason — a lazy
            // iterator's captured state outliving the call — went with the
            // lazy pair (2026-09-10); a stored callback's own `'static` is
            // spelled where the `Rc<dyn Fn>` is [rs-fn-field].
            .map(|g| format!("{}: Clone", g.name))
            .collect();
        let mut params: Vec<String> = Vec::new();
        if handler_of_style.is_some() {
            params.push("&mut self".to_string());
        }
        // Effect dependencies become leading parameters [rs-effects],
        // sourced from the checker's lowered effect list when available
        // (checker-`Ty` keys; the AST rendering is the unchecked fallback).
        // Under the fusion they collapse into *one* value [rs-effect-fusion].
        let mut body_prelude = String::new();
        let style_deps: Vec<(String, Vec<String>)> = match handler_of_style {
            Some(h) if matches!(style, FnStyle::DepMember(_) | FnStyle::DepMemberSig(_)) => {
                self.handler_dep_effects(h)
            }
            _ => Vec::new(),
        };
        if !style_deps.is_empty() {
            let dep_effects = style_deps;
            // Uniformly a Has-bounded generic, single dependency included
            // (user decision 2026-09-14: consistency over a special case):
            // the `__Deps_H` adapter is what the forwarding impl passes.
            let has_bounds: Vec<String> = dep_effects
                .iter()
                .map(|(b, a)| has_trait_type(b, a))
                .collect();
            let var = self.unique_name("__fx".to_string());
            generic_parts.push(format!("__Fx: {}", has_bounds.join(" + ")));
            for (b, a) in &dep_effects {
                self.effect_env.push(EffectEntry {
                    ty: None,
                    key: trait_type(b, a),
                    var: var.clone(),
                    is_local: false,
                    handle_var: None,
                });
            }
            self.bindings.insert(var.clone(), BindKind::RefMut);
            params.push(format!("{var}: &mut __Fx"));
        } else if (!is_main || self.declares_platform_effect(f)) && handler_of_style.is_none() {
            let checked_effects: Option<Vec<Ty>> = self
                .checked
                .fn_refs
                .get(&(self.file_idx, f.name.span))
                .and_then(|key| self.checked.fn_effects.get(key))
                .cloned();
            let mut effects: Vec<(Option<Ty>, String)> = Vec::new();
            match checked_effects {
                Some(tys) => {
                    for ty in tys {
                        // [rs-throw-controlflow] `Throw` is not a
                        // capability parameter: it changes the *return*
                        // shape instead, so it never becomes a `&mut dyn`.
                        if is_throw_effect_ty(&ty) {
                            continue;
                        }
                        let rendered = self.rust_ty(&ty);
                        // [platform-effect] `main` takes only its *platform*
                        // effects: the host supplies those by calling the
                        // entry point, while everything else it needs is
                        // registered inside it with `use`.
                        if is_main && !self.is_platform_effect(Some(&ty), &rendered) {
                            continue;
                        }
                        effects.push((Some(ty), rendered));
                    }
                }
                None => {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                            if r.name.name == salvo_core::THROW_EFFECT {
                                continue;
                            }
                            let rendered = self.emit_type_ref(r);
                            if is_main && !self.is_platform_effect(None, &rendered) {
                                continue;
                            }
                            effects.push((None, rendered));
                        }
                    }
                }
            }
            if self.fusion {
                if is_main && self.declares_platform_effect(f) {
                    // [rs-platform-entry] The host constructs one
                    // implementation per platform effect and calls
                    // `salvo_main` with them, so the entry keeps per-effect
                    // `&mut dyn` parameters — the one ABI the fusion cannot
                    // reshape. The body opens by combining them into a
                    // Sized `__Dyn` value implementing the Has traits over
                    // disjoint dyn fields, and everything below threads
                    // through it uniformly.
                    if !effects.is_empty() {
                        let var = self.unique_name("__fx".to_string());
                        self.fusion_id += 1;
                        let combiner = format!(
                            "__Dyn_{}_{}",
                            sanitize_ident(&self.current_fn),
                            self.fusion_id
                        );
                        let mut fields = String::new();
                        let mut impls = String::new();
                        let mut init: Vec<String> = Vec::new();
                        for (i, (ty, rendered)) in effects.iter().enumerate() {
                            let param = self.unique_name(effect_param_name(rendered));
                            params.push(format!("{param}: &mut dyn {rendered}"));
                            self.bindings.insert(param.clone(), BindKind::RefMut);
                            fields.push_str(&format!("    __e{i}: &'a mut dyn {rendered},\n"));
                            let (base, args) = match ty {
                                Some(ty) => {
                                    let ty = ty.clone();
                                    self.ty_effect_parts(&ty)
                                }
                                None => split_rendered_generic(rendered),
                            };
                            impls.push_str(&emit_has_impl(
                                "<'a>",
                                &format!("{combiner}<'a>"),
                                &base,
                                &args,
                                &format!("&mut *self.__e{i}"),
                            ));
                            init.push(format!("__e{i}: {param}"));
                        }
                        self.generated_items
                            .push(format!("\npub struct {combiner}<'a> {{\n{fields}}}\n{impls}"));
                        let pad_body = "    ".repeat(indent + 1);
                        body_prelude = format!(
                            "{pad_body}let mut {var} = {combiner} {{ {} }};\n",
                            init.join(", ")
                        );
                        for (ty, rendered) in effects {
                            self.effect_env.push(EffectEntry {
                                ty,
                                key: rendered,
                                var: var.clone(),
                                is_local: true,
                                handle_var: None,
                            });
                        }
                        self.bindings.insert(var.clone(), BindKind::Owned);
                    }
                } else if !effects.is_empty() {
                    // One fused value for *any* number of effects, single
                    // included (user decision 2026-09-14: consistency over
                    // the `&mut dyn` special case). The bounds are the
                    // Has-accessor traits, never the effect traits, so
                    // same-named members can never collide on `__Fx`.
                    let var = self.unique_name("__fx".to_string());
                    let bounds: Vec<String> = effects
                        .iter()
                        .map(|(_, rendered)| {
                            let (base, args) = split_rendered_generic(rendered);
                            has_trait_type(&base, &args)
                        })
                        .collect();
                    generic_parts.push(format!("__Fx: {}", bounds.join(" + ")));
                    for (ty, rendered) in effects {
                        self.effect_env.push(EffectEntry {
                            ty,
                            key: rendered,
                            var: var.clone(),
                            is_local: false,
                            handle_var: None,
                        });
                    }
                    self.bindings.insert(var.clone(), BindKind::RefMut);
                    params.push(format!("{var}: &mut __Fx"));
                }
                // [spawn-inherit] [rs-handle-bundle] The hidden handle
                // parameter: one fused bundle carrying a handle per
                // signature-supplied effect this fn (or something it calls)
                // captures at construction. Emitted only where the checker
                // recorded a requirement — which it only does for a fn whose
                // effect list carries `use`/`spawn` (user decision
                // 2026-09-20), so the hidden parameter is predictable from
                // the visible signature.
                if let Some(bundle) = self.handle_bundle_param(f) {
                    params.push(bundle);
                }
            } else {
                for (ty, rendered) in effects {
                    let param = self.unique_name(effect_param_name(&rendered));
                    self.effect_env.push(EffectEntry {
                        ty,
                        key: rendered.clone(),
                        var: param.clone(),
                        is_local: false,
                        handle_var: None,
                    });
                    self.bindings.insert(param.clone(), BindKind::RefMut);
                    params.push(format!("{param}: &mut dyn {rendered}"));
                }
            }
        }
        let mut ref_param_count = 0usize;
        let mut derived_param_idx: Option<usize> = None;
        for (i, p) in f.params.iter().enumerate() {
            if p.implicit {
                continue; // appended below, in the checker's order
            }
            let mode = match style {
                FnStyle::TopLevel => self.param_mode(fn_key, p),
                // [effect-member-overload] [rs-borrows] A handler member's
                // signature has to match the trait's, so its modes come from
                // the *effect member* it implements — the handler's own
                // clause is checked against that contract, not consulted
                // here.
                FnStyle::HandlerMember(h)
                | FnStyle::DepMember(h)
                | FnStyle::DepMemberSig(h) => match self.effect_member_of(h, f) {
                    Some(m) => {
                        let m = m.clone();
                        self.member_param_mode(&m, p)
                    }
                    // [mixed-handler] [rs-mixed] A handler-local send member
                    // has no effect contract to mirror; its own written
                    // clause is the contract (the checker required it
                    // complete and all-consumed), so its payloads are owned
                    // exactly as the message enum carries them.
                    None if f.is_send => self.member_param_mode(f, p),
                    None => self.default_param_mode(&p.ty, p.variadic),
                },
                _ => self.default_param_mode(&p.ty, p.variadic),
            };
            if matches!(mode, ParamMode::Ref | ParamMode::RefMut) {
                ref_param_count += 1;
            }
            if f.derived_return
                .as_ref()
                .is_some_and(|d| d.name == p.name.name)
            {
                derived_param_idx = Some(i);
            }
            let kind = match mode {
                ParamMode::Owned => BindKind::Owned,
                ParamMode::Ref => BindKind::Ref,
                ParamMode::RefMut => BindKind::RefMut,
            };
            self.bindings.insert(p.name.name.clone(), kind);
            // [rs-borrows] An owned parameter binds `mut` when the body
            // *reassigns* it — and also when its type says `Mut`, because that
            // is exactly the claim that it may be mutated through this
            // binding: a moved-in `Mut D` handed to a `Mut` position needs
            // `&mut dest`, which a non-`mut` binder refuses (E0596). Passing
            // it on is not an assignment, so `collect_mutated` cannot see it.
            // A spurious `mut` is harmless — `unused_mut` is allowed in
            // generated code — while a missing one does not compile.
            //
            // [rs-narrow-mut] `Mut` on a union *arm* counts too: peeling that
            // arm mutably borrows this binder (`o.u1_mut()`), which needs the
            // same `mut`.
            let mut_kw = if mode == ParamMode::Owned
                && (self.mutated.contains(&p.name.name) || type_has_mut_arm(&p.ty))
            {
                "mut "
            } else {
                ""
            };
            params.push(format!(
                "{mut_kw}{}: {}",
                rs_ident(&p.name.name),
                self.param_type(&p.ty, p.variadic, mode)
            ));
        }
        // [implicit-param] Implicit parameters are ordinary trailing
        // parameters of fn type, rendered like any other fn value
        // [fn-contract]: nothing about them survives into Rust.
        let own_implicits = self.implicits_of(f);
        let saved_implicits = std::mem::replace(&mut self.implicits, own_implicits);
        self.lent_position_params.clear();
        for imp in &self.implicits.clone() {
            // [rs-iter-pass] An iterator fn's callbacks arrive **owned** and
            // `'static`, because its pass calls them long after this returns —
            // and an implicit is a callback. The same convention exception a
            // written fn-typed parameter gets.
            if self.in_iterator_fn {
                let rendered = self.owned_fn_ty(&imp.ty);
                let owned = format!("{rendered} + 'static");
                self.bindings.insert(imp.name.clone(), BindKind::Owned);
                params.push(format!("{}: {owned}", rs_ident(&imp.name)));
                continue;
            }
            let rendered = self.implicit_param_type_of(imp);
            self.bindings.insert(imp.name.clone(), BindKind::RefMut);
            params.push(format!("{}: {rendered}", rs_ident(&imp.name)));
        }
        // [copy-implicit] A handler member also sees the handler's implicit
        // constructor parameters — after the signature, since they are
        // *fields* (`Box<dyn FnMut>`), called through `self`, not trailing
        // parameters of the member.
        if let Some(h) = handler_of_style {
            for p in h.params.iter().filter(|p| p.implicit) {
                if let Some(ty) = self
                    .checked
                    .expr_ty
                    .get(&(self.file_idx, p.name.span))
                    .cloned()
                {
                    self.implicits.push(salvo_core::ImplicitParam {
                        name: p.name.name.clone(),
                        ty,
                        span: p.span,
                        borrowed_arms: Vec::new(),
                        binder: false,
                        slot: None,
                    });
                    self.bindings
                        .insert(p.name.name.clone(), BindKind::SelfField);
                }
            }
        }

        // [rs-proj-lends] An implicit's lent position borrows under `'c`, the
        // enclosing fn's own borrow of the parameter the position is named
        // after: that parameter must be kept here (a consumed one has no
        // lifetime to name — the view would have nothing to outlive), and
        // gets `&'c` too.
        let mut extra_lifetimes: Vec<String> = Vec::new();
        if !self.lent_position_params.is_empty() {
            let wanted = std::mem::take(&mut self.lent_position_params);
            let mut ok = false;
            for want in &wanted {
                let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                let Some(i) = fixed.iter().position(|p| p.name.name == *want) else {
                    continue;
                };
                // Only the *leading* extras (self, effects) shift positions;
                // the trailing implicits do not.
                let leading = params
                    .len()
                    .saturating_sub(fixed.len() + self.implicits.len());
                let idx = leading + i;
                match params.get_mut(idx) {
                    Some(entry) if entry.contains(": &") && !entry.contains(": &mut ") => {
                        *entry = entry.replacen(": &", ": &'c ", 1);
                        ok = true;
                    }
                    _ => {
                        self.error(format!(
                            "`{}` is lent through an implicit (`[{want}: proj]`), so `{}` must \
                             keep it (`-> [{want}] ...`): a consumed value has nothing a view \
                             could outlive",
                            want, f.name.name
                        ));
                    }
                }
            }
            if ok {
                extra_lifetimes.push("'c".to_string());
            }
        }
        // [proj-field] Re-pointing entries (`v.items: proj[from: other]`):
        // the borrowing struct at `v` takes a borrow of `other`, so the two
        // share a named lifetime — `v: &mut View<'r>`, `other: &'r T`.
        if let Some(list) = &f.deductions {
            let mut tied = false;
            for d in list {
                let (
                    salvo_syntax::ast::DeductionTarget::Param { name, .. },
                    salvo_syntax::ast::DeductionKind::Proj(srcs),
                ) = (&d.target, &d.kind)
                else {
                    continue;
                };
                let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                let leading = params
                    .len()
                    .saturating_sub(fixed.len() + self.implicits.len());
                let mut names: Vec<&str> = vec![name.name.as_str()];
                names.extend(srcs.iter().map(|s| s.name.as_str()));
                for n in names {
                    let Some(i) = fixed.iter().position(|p| p.name.name == n) else {
                        continue;
                    };
                    if let Some(entry) = params.get_mut(leading + i) {
                        let re = if entry.contains("<'_") {
                            entry.replacen("<'_", "<'r", 1)
                        } else if entry.contains(": &mut ") {
                            entry.replacen(": &mut ", ": &'r mut ", 1)
                        } else {
                            entry.replacen(": &", ": &'r ", 1)
                        };
                        if re != *entry {
                            *entry = re;
                            tied = true;
                        }
                    }
                }
            }
            if tied {
                extra_lifetimes.push("'r".to_string());
            }
        }
        // [readonly-return] A derived-return fn returns a borrow of its
        // annotated parameter: `&T` (plain) or `Option<&T>` (optional).
        // With a single reference parameter, lifetime elision covers it;
        // with more, a `'a` is generated mechanically and tags the
        // annotated parameter and the return.
        let mut lifetime_generics = String::new();
        self.derived_return_fn = f.derived_return.is_some();
        self.returns_borrowing_struct = matches!(
            f.return_type.as_ref(),
            Some(Type::Named { base, .. }) if self.borrowing_structs.contains(&base.name.name)
        );
        // [rs-proj-arm] Which constructors build the `proj` arm: the
        // qualifier written on that arm names the constructive qualifier,
        // and its constructor fn shares the qualifier's name in lowercase
        // by std convention (`Emitted` ↔ `emitted`) — but the reliable link
        // is the `-> T as Q` declaration, so both are consulted.
        self.proj_arm_ctors = match f.return_type.as_ref() {
            Some(Type::Union { arms, .. }) => {
                arms.iter()
                    .filter(|arm| type_has_proj(arm))
                    .filter_map(|arm| match arm {
                        Type::Named { qualifiers, .. }
                        | Type::QualifiedGroup { qualifiers, .. } => qualifiers
                            .iter()
                            .find(|q| q.name.name != "proj")
                            .map(|q| q.name.name.clone()),
                        _ => None,
                    })
                    .flat_map(|qual| self.constructors_of(&qual))
                    .collect()
            }
            _ => Vec::new(),
        };
        let ret = if is_main {
            String::new()
        } else if f.derived_return.is_some() {
            // [rs-proj-struct] When the annotated parameter is itself a
            // borrowing struct (`p: &mut ListYield<'s, T>`), the returned
            // borrow is of the struct's *source*, not of the parameter: the
            // source lifetime is named on both and the reborrow of `p`
            // stays free for the next turn — which is what lets a caller
            // store the element and drive on.
            let derived_is_borrowing_struct = derived_param_idx.is_some_and(|i| {
                matches!(
                    &f.params[i].ty,
                    Type::Named { base, .. } if self.borrowing_structs.contains(&base.name.name)
                )
            });
            let lt = if derived_is_borrowing_struct {
                lifetime_generics = "'s".to_string();
                if let Some(i) = derived_param_idx {
                    let idx = if params.len() > f.params.len() {
                        params.len() - f.params.len() + i
                    } else {
                        i
                    };
                    if let Some(entry) = params.get_mut(idx) {
                        *entry = entry.replacen("<'_", "<'s", 1);
                    }
                }
                "'s "
            } else if ref_param_count > 1 {
                lifetime_generics = "'a".to_string();
                // Retag every source parameter's type with 'a
                // (`proj[from: a, b]` names several) [proj-anywhere].
                let sources: Vec<usize> = f
                    .return_type
                    .as_ref()
                    .and_then(|rt| proj_refs_with_from(rt).into_iter().next())
                    .map(|froms| {
                        froms
                            .iter()
                            .filter_map(|n| f.params.iter().position(|p| p.name.name == *n))
                            .collect()
                    })
                    .unwrap_or_else(|| derived_param_idx.into_iter().collect());
                for i in sources {
                    let idx = if params.len() > f.params.len() {
                        // Leading self/effect params shift positions.
                        params.len() - f.params.len() + i
                    } else {
                        i
                    };
                    if let Some(entry) = params.get_mut(idx) {
                        *entry = entry
                            .replacen(": &mut ", ": &'a mut ", 1)
                            .replacen(": &", ": &'a ", 1);
                    }
                }
                "'a "
            } else {
                ""
            };
            match f.return_type.as_ref() {
                // [rs-proj-struct] A borrowing struct is returned *by
                // value*: the struct itself carries the lifetime, so no `&`
                // wraps it — `ListYield<'_, T>` (or `'a` when generated).
                Some(Type::Named { base, .. })
                    if self.borrowing_structs.contains(&base.name.name) =>
                {
                    let ty = self.emit_return_type(f.return_type.as_ref());
                    let ty = ty.trim_start_matches(" -> ");
                    if lt.is_empty() {
                        format!(" -> {ty}")
                    } else {
                        format!(" -> {}", ty.replacen("<'_", &format!("<{}", lt.trim()), 1))
                    }
                }
                // [proj-type] Every other projected shape — `Option<&T>`, a
                // union with a `proj` arm (`Union2<&'a T, Finished>`, an
                // ordinary instantiation of the shared enum [rs-proj-arm]),
                // `&T` itself, a tuple — is the general rendering of the
                // written type, under the lifetime this fn names.
                Some(Type::Nullable { .. })
                | Some(Type::Union { .. })
                | Some(Type::Named { .. })
                | Some(Type::Array { .. })
                | Some(Type::Tuple { .. }) => {
                    let saved = self
                        .proj_lifetime
                        .replace(lt.trim().to_string())
                        .filter(|_| !lt.trim().is_empty());
                    if lt.trim().is_empty() {
                        self.proj_lifetime = None;
                    }
                    let saved_ret = std::mem::replace(&mut self.proj_return, true);
                    let ty = self.emit_return_type(f.return_type.as_ref());
                    self.proj_return = saved_ret;
                    self.proj_lifetime = saved;
                    ty
                }
                other => {
                    self.error(format!(
                        "`proj[from: ...]` returns support only plain, optional, \
                         struct-borrow and union shapes, not `{other:?}`"
                    ));
                    self.emit_return_type(f.return_type.as_ref())
                }
            }
        } else {
            let rendered = self.emit_return_type(f.return_type.as_ref());
            // [rs-proj-lends] A result that *holds* borrows (a struct with
            // `proj` fields) carries a lifetime that must reach the lent
            // parameters [proj-infer]. One reference parameter: elision ties
            // them. More: name `'a` on every lent parameter and the return.
            let lent: Vec<usize> = fn_key
                .and_then(|k| self.checked.fn_lends.get(&k).cloned())
                .unwrap_or_default();
            if rendered.contains("<'_") && ref_param_count > 1 && !lent.is_empty() {
                lifetime_generics = "'a".to_string();
                for &i in &lent {
                    let idx = if params.len() > f.params.len() {
                        params.len() - f.params.len() + i
                    } else {
                        i
                    };
                    if let Some(entry) = params.get_mut(idx) {
                        *entry = entry
                            .replacen(": &mut ", ": &'a mut ", 1)
                            .replacen(": &", ": &'a ", 1)
                            .replacen("<'_", "<'a", 1);
                    }
                }
                rendered.replace("<'_", "<'a")
            } else {
                rendered
            }
        };
        // [rs-none-unit] An empty rendered return type *is* Rust's `()`. Taken
        // before the `Throw` wrapping below, which replaces the type but not
        // the question this answers (a `ControlFlow`-returning fn still has a
        // `None` payload).
        let saved_ret_unit = std::mem::replace(&mut self.ret_is_unit, ret.is_empty());
        // [rs-throw-controlflow] The declared return type becomes
        // `ControlFlow`'s `Continue` payload; the message type is its
        // `Break` payload, which is why propagation is `?`.
        let ret = match self.throw_message.clone() {
            Some(message) => {
                self.imports
                    .insert("use std::ops::ControlFlow;".to_string());
                let msg = self.rust_ty(&message);
                let value = ret.trim_start_matches(" -> ").trim();
                let value = if value.is_empty() { "()" } else { value };
                format!(" -> ControlFlow<{msg}, {value}>")
            }
            None => ret,
        };

        let pad = "    ".repeat(indent);
        let name = if is_main {
            // [platform-effect] A `main` needing platform effects is not the
            // crate's `main` any more — the host's is, and it calls this
            // after constructing the implementations.
            if self.declares_platform_effect(f) {
                SALVO_ENTRY.to_string()
            } else {
                "main".to_string()
            }
        } else if top_level || matches!(style, FnStyle::QualifierFn) {
            self.rust_fn_name(f)
        } else {
            // [effect-member-overload] A handler member implements one
            // *overload* of its effect's member, and the trait names the
            // overloads apart, so the impl has to use the same name.
            match style {
                FnStyle::HandlerMember(h)
                | FnStyle::DepMember(h)
                | FnStyle::DepMemberSig(h) => self.handler_member_name(h, f),
                _ => rs_ident(&f.name.name),
            }
        };
        let vis = if handler_of_style.is_some() {
            ""
        } else {
            "pub "
        };
        let mut all_generics: Vec<String> = Vec::new();
        if !lifetime_generics.is_empty() {
            all_generics.push(lifetime_generics.clone());
        }
        all_generics.extend(extra_lifetimes);
        all_generics.extend(generic_parts);
        let generics = if all_generics.is_empty() {
            String::new()
        } else {
            format!("<{}>", all_generics.join(", "))
        };
        if sig_only {
            self.generics = saved_generics;
            self.effect_env = saved_env;
            self.bindings = saved_bindings;
            self.handle_fields = saved_handle_fields;
            self.mutated = saved_mutated;
            self.implicits = saved_implicits;
            self.in_iterator_fn = saved_in_iterator;
            self.derived_return_fn = saved_derived;
            self.taken_names = saved_taken;
            self.current_fn = saved_fn;
            self.current_handler = saved_handler;
            self.hoist_id = saved_hoist;
            return format!(
                "\n{pad}{vis}fn {name}{generics}({}){ret};\n",
                params.join(", ")
            );
        }
        let mut out = format!(
            "\n{pad}{vis}fn {name}{generics}({}){ret} {{\n",
            params.join(", ")
        );
        // [rs-exit-splice] Splices never cross a fn boundary.
        let saved_splices = std::mem::take(&mut self.exit_splices);
        let saved_splice_floor = std::mem::replace(&mut self.splice_floor, 0);
        let saved_loop_floors = std::mem::take(&mut self.loop_splice_floors);

        {
            // [rs-effect-fusion] A dyn-boundary fn (platform `main`) opens
            // by combining its `&mut dyn` parameters into the fused value
            // the rest of the body threads.
            out.push_str(&body_prelude);
            out.push_str(&self.emit_block_stmts(body, indent + 1, StmtCtx::Normal));
            // [rs-throw-controlflow] A `None`-returning fn that may throw
            // still has to produce a `ControlFlow` value on the way out.
            if self.throw_message.is_some()
                && self.emit_return_type(f.return_type.as_ref()).is_empty()
            {
                out.push_str(&format!("{pad}    return ControlFlow::Continue(());\n"));
            }
        }
        out.push_str(&format!("{pad}}}\n"));

        self.generics = saved_generics;
        self.effect_env = saved_env;
        self.bindings = saved_bindings;
        self.handle_fields = saved_handle_fields;
        self.mutated = saved_mutated;
        self.implicits = saved_implicits;
        self.in_iterator_fn = saved_in_iterator;
        self.derived_return_fn = saved_derived;
        self.taken_names = saved_taken;
        self.current_fn = saved_fn;
        self.current_handler = saved_handler;
        self.hoist_id = saved_hoist;
        self.ret_is_unit = saved_ret_unit;
        self.exit_splices = saved_splices;
        self.splice_floor = saved_splice_floor;
        self.loop_splice_floors = saved_loop_floors;
        self.throw_message = saved_throw;
        self.try_frames = saved_try_frames;
        out
    }

    /// [rs-shadowed-call] The Rust path prefix that reaches a top-level fn from
    /// inside this file: `crate::<mounted module>`. Used where a bare name
    /// would resolve to something else — a local of the same name.
    fn fn_module_path(&self, decl: &FnDecl) -> String {
        let module = self
            .checked
            .fn_refs
            .get(&(self.file_idx, decl.name.span))
            .map(|key| key.file)
            .and_then(|file| self.program.files.get(file))
            .map(|f| &f.module);
        match module {
            // The crate root's items are `crate::…` [rs-crate].
            Some(module) if Some(module) == self.root_module => "crate".to_string(),
            Some(module) => {
                let mangled = module
                    .0
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join("_");
                format!("crate::{}", rs_ident(&mangled))
            }
            // The declaring module is unknown only if a table lost an entry;
            // `self` still reaches a fn of this file.
            None => "self".to_string(),
        }
    }

    /// The Rust name for a top-level fn [rs-fn-mangling]: Rust has no
    /// overloading, so after the qualifier-suffix rule (shared with
    /// Kotlin, [kt-qual-mangling]) any *still*-colliding overloads with
    /// bodies get positional suffixes (`name__2`, `name__3`, ... in
    /// declaration order).
    fn rust_fn_name(&mut self, decl: &FnDecl) -> String {
        let name = decl.name.name.clone();
        let overloads: Vec<&FnDecl> = match self.symbols.fns.get(name.as_str()) {
            // [cmp-auto] A generated structural member has no body and is
            // still emitted, so it takes part in mangling like any overload —
            // without this, three `auto Eq` structs would all emit `eq`.
            Some(o) if o.len() > 1 => o
                .iter()
                .filter(|f| f.body.is_some() || f.structural)
                .copied()
                .collect(),
            _ => return rs_ident(&name),
        };
        if overloads.len() <= 1 {
            return rs_ident(&name);
        }
        // Qualifier-suffix pass: an overload whose erased signature
        // collides with another's gets its qualifier suffix.
        let mangled: Vec<String> = overloads
            .iter()
            .map(|f| {
                let suffix = qual_suffix(f);
                if suffix.is_empty() {
                    return name.clone();
                }
                let mine = self.erased_sig(f);
                let collides = overloads.iter().any(|other| {
                    !std::ptr::eq(*other as *const FnDecl, *f as *const FnDecl)
                        && self.erased_sig(other) == mine
                });
                if collides {
                    format!("{name}__{suffix}")
                } else {
                    name.clone()
                }
            })
            .collect();
        // Positional pass over what still collides.
        let mut final_names = mangled.clone();
        for i in 0..final_names.len() {
            let dup_before = mangled[..i].iter().filter(|m| **m == mangled[i]).count();
            if dup_before > 0 {
                final_names[i] = format!("{}__{}", mangled[i], dup_before + 1);
            }
        }
        for (f, final_name) in overloads.iter().zip(&final_names) {
            if std::ptr::eq(*f as *const FnDecl, decl as *const FnDecl) {
                return rs_ident(final_name);
            }
        }
        rs_ident(&name)
    }

    /// The erased parameter signature, for overload collision detection.
    fn erased_sig(&mut self, decl: &FnDecl) -> String {
        let saved = self.enter_generics(&decl.generics);
        let sig = decl
            .params
            .iter()
            .map(|p| self.emit_type(&p.ty))
            .collect::<Vec<_>>()
            .join(",");
        self.generics = saved;
        sig
    }

    /// Generic parameters with the blanket `Clone` bound [rs-borrows], plus
    /// `'static`: every Salvo type is owned data with no lifetime of its
    /// own, and a lazy iterator's captured state has to outlive the call
    /// that produced it [rs-iter-pass].
    fn emit_generic_params(&self, generics: &[Ident]) -> String {
        if generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                generics
                    .iter()
                    .map(|g| format!("{}: Clone + 'static", g.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    /// Generic parameters without bounds (trait declarations: bounds live
    /// on impls, keeping the traits dyn-friendly [rs-effects]).
    fn emit_generic_params_unbounded(&self, generics: &[Ident]) -> String {
        if generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                generics
                    .iter()
                    .map(|g| g.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn emit_generic_args_plain(&self, generics: &[Ident]) -> String {
        if generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                generics
                    .iter()
                    .map(|g| g.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn enter_generics(&mut self, generics: &[Ident]) -> HashSet<String> {
        let saved = self.generics.clone();
        for g in generics {
            self.generics.insert(g.name.clone());
        }
        saved
    }

    fn emit_return_type(&mut self, ty: Option<&Type>) -> String {
        match ty {
            None => String::new(),
            Some(Type::Named { base, .. }) if base.name.name == "None" => String::new(),
            Some(t) => format!(" -> {}", self.emit_type(t)),
        }
    }

    // ================= types =================

    fn emit_type(&mut self, ty: &Type) -> String {
        // [proj-type] `proj X` *is* a reference: `&X`, at whatever depth it
        // sits — a union arm (`Union2<&String, Finished>`), a type argument
        // (`Vec<&String>`), a field, a parameter. (A Copy scalar's `proj` is
        // erased by the checker and never reaches here written.)
        //
        // Except over a bare *generic parameter* with no lifetime context:
        // `proj T` at the definition site renders `T`. A generic body treats
        // its `T` uniformly, and whether a use is borrowed is the
        // instantiation's fact — the caller substitutes `T = proj Str`
        // (rendering `&String`) and the turbofish retag spells it
        // [rs-proj-arm]. Rendering `&T` here would borrow for *every*
        // instantiation, owned ones included. When `proj_lifetime` *is* set
        // (a borrowing struct's field, a `next` over one), the projection is
        // of the fn's own named borrow and renders `&'s T` even over a
        // generic.
        if let Some(inner) = strip_top_proj_ast(ty) {
            if self.proj_lifetime.is_none() && !self.proj_return {
                if let Type::Named { qualifiers, base } = &inner {
                    if qualifiers.is_empty()
                        && base.args.is_empty()
                        && self.generics.contains(&base.name.name)
                    {
                        return self.emit_type(&inner);
                    }
                }
            }
            let rendered = self.emit_type(&inner);
            let lt = self
                .proj_lifetime
                .clone()
                .map(|l| format!("{l} "))
                .unwrap_or_default();
            return format!("&{lt}{rendered}");
        }
        match ty {
            Type::Named { qualifiers, base } => self.emit_named_type(qualifiers, base),
            Type::Nullable { inner, .. } => format!("Option<{}>", self.emit_type(inner)),
            Type::Array { elem, .. } => format!("Vec<{}>", self.emit_type(elem)),
            Type::Union { arms, .. } => self.emit_union_type(arms),
            Type::Fn {
                params,
                param_names,
                deductions,
                ret,
                effects,
                ..
            } => {
                // Only meaningful in parameter position [fn-lambda].
                // [fn-contract] Argument types follow the contract:
                // kept non-Copy borrow, kept `Mut` borrows mutably,
                // moved (or Copy) owned. `FnMut` accepts both plain and
                // handler-mutating closures.
                let ps: Vec<String> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let (kept, is_mut) =
                            ast_fn_param_contract(params, param_names, deductions, i);
                        let base = self.emit_type(p);
                        if kept && is_mut {
                            format!("&mut {base}")
                        } else if kept && !self.is_copy_ast_type(p) {
                            format!("&{base}")
                        } else {
                            base
                        }
                    })
                    .collect();
                let ret = match ret.as_ref() {
                    Type::Named { base, .. } if base.name.name == "None" => String::new(),
                    other => format!(" -> {}", self.emit_type(other)),
                };
                // [fn-effects] The effects a call performs are threaded in,
                // so they are leading `&mut dyn E` parameters of the closure
                // type — nothing is captured, which is what lets an
                // effect-using fn value cross an effectful call.
                let mut all = self.fn_type_effect_params(effects.as_deref());
                all.extend(ps);
                // [rs-iter-pass] An iterator fn's fn-typed parameter has to
                // outlive the call and be usable in every pass, so it
                // arrives owned and `'static` instead of borrowed. `Fn`
                // rather than `FnMut`: a pass may run more than once, so a
                // callback with mutable state of its own would depend on
                // how many times the iterator was consumed — which is
                // exactly the divergence [fn-effects] rules out for
                // effects.
                if self.in_iterator_fn {
                    return format!("impl Fn({}){ret} + 'static", all.join(", "));
                }
                format!("impl FnMut({}){ret}", all.join(", "))
            }
            Type::Tuple { elems, .. } => {
                let parts: Vec<String> = elems.iter().map(|e| self.emit_type(e)).collect();
                format!("({})", parts.join(", "))
            }
            Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                // [once-fn] A `once` fn type emits `impl FnOnce`: the
                // checker guarantees at most one call, and rustc's
                // capture inference makes consuming closures `FnOnce`
                // on its own.
                if qualifiers.iter().any(|q| q.name.name == "once") {
                    if let Type::Fn {
                        params,
                        ret,
                        effects,
                        ..
                    } = base.as_ref()
                    {
                        let ps: Vec<String> = params.iter().map(|p| self.emit_type(p)).collect();
                        let ret = match ret.as_ref() {
                            Type::Named { base, .. } if base.name.name == "None" => String::new(),
                            other => format!(" -> {}", self.emit_type(other)),
                        };
                        // [fn-effects] as for `FnMut` above.
                        let mut all = self.fn_type_effect_params(effects.as_deref());
                        all.extend(ps);
                        return format!("impl FnOnce({}){ret}", all.join(", "));
                    }
                }
                self.emit_type(base)
            }
        }
    }

    /// [fn-effects] The leading parameters a fn type's effects contribute.
    /// Plain mode: one `&mut dyn Effect` each, in declaration order. Fusion
    /// mode: **one** `&mut dyn` provider for the whole set — the single
    /// effect's Has trait, or a `__Prov_…` conjunction of Has traits — since
    /// two separate reborrows of the caller's one fused value would alias
    /// (`E0499`). The lambda side rebuilds a Sized fused value from it
    /// ([rs-effect-fusion]).
    fn fn_type_effect_params(&mut self, effects: Option<&[EffectRef]>) -> Vec<String> {
        let mut rendered: Vec<String> = Vec::new();
        for eff in effects.into_iter().flatten() {
            if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                rendered.push(self.emit_type_ref(r));
            }
        }
        if self.fusion {
            if rendered.is_empty() {
                return Vec::new();
            }
            let prov = self.fn_value_prov(&rendered);
            return vec![format!("&mut dyn {prov}")];
        }
        rendered
            .into_iter()
            .map(|r| format!("&mut dyn {r}"))
            .collect()
    }

    /// [rs-effect-fusion] The nameable `dyn` type carrying an effect set
    /// across a fn-value boundary: the one effect's Has trait, or the
    /// provider conjunction for several.
    fn fn_value_prov(&mut self, keys: &[String]) -> String {
        if keys.len() == 1 {
            let (base, args) = split_rendered_generic(&keys[0]);
            has_trait_type(&base, &args)
        } else {
            self.prov_trait(keys)
        }
    }

    fn emit_named_type(&mut self, qualifiers: &[TypeRef], base: &TypeRef) -> String {
        let name = base.name.name.clone();
        // [type-canbe-mut] `Mut` erases here: on Rust, mutability lives in
        // the binding, not the type [rs-borrows], so `Mut List<T>` and
        // `List<T>` render identically and the deduction-driven parameter
        // mode decides `&` vs `&mut`.
        let _ = qualifiers;
        self.emit_type_ref_named(&name, &base.args)
    }
    fn emit_type_ref(&mut self, r: &TypeRef) -> String {
        self.emit_type_ref_named(&r.name.name, &r.args)
    }

    fn emit_type_ref_named(&mut self, name: &str, args: &[Type]) -> String {
        // Type aliases expand structurally [type-alias].
        if let Some(alias) = self.symbols.type_aliases.get(name) {
            let alias = *alias;
            if let Some(target) = &alias.alias {
                if alias.generics.is_empty() {
                    return self.emit_type(target);
                }
                let map: HashMap<&str, &Type> = alias
                    .generics
                    .iter()
                    .map(|g| g.name.as_str())
                    .zip(args.iter())
                    .collect();
                let substituted = subst_ast_type(target, &map);
                return self.emit_type(&substituted);
            }
        }
        // [monitor-handler] [rs-monitor] A plain effect's addr is the effect's
        // lock wrapper, not a scheduler index — decided on the *written* type
        // here, before the argument is rendered away.
        if name == "Addr" && args.len() == 1 {
            if let Type::Named { base: eff, .. } = &args[0] {
                let effect = eff.name.name.clone();
                if self
                    .symbols
                    .effects
                    .get(effect.as_str())
                    .is_some_and(|e| !e.is_actor)
                {
                    let eff_args: Vec<String> =
                        eff.args.iter().map(|a| self.emit_type(a)).collect();
                    return self.plain_addr_rendering(&effect, &eff_args);
                }
            }
        }
        // [cmp-carry] A written **identity** (`SortedSet<Person, by_age>`) is not
        // a type and has no rendering: the ordering is carried by the container's
        // store, so the emitted type mentions only the element. Recognised by
        // shape — a binder, a selector, or a bare lowercase name, since
        // [name-casing] reserves uppercase for types.
        let mut arg_strs: Vec<String> = args
            .iter()
            .filter(|a| !salvo_syntax::ast::is_identity_arg(a))
            .map(|a| self.emit_type(a))
            .collect();
        // [rs-proj-struct] A borrowing struct carries its source's lifetime
        // at every mention; `'_` lets rustc infer it in signatures and
        // locals, and the struct's own definition spells it `'s`.
        if self.borrowing_structs.contains(name) {
            arg_strs.insert(0, "'_".to_string());
        }
        self.emit_named_parts(name, &arg_strs)
    }

    /// Maps a named type to Rust: intrinsic types [backend-intrinsic], or
    /// pass-through [type-unknown-lenient].
    fn emit_named_parts(&mut self, name: &str, arg_strs: &[String]) -> String {
        // [rs-effect-fusion] Inside a forwarding impl for an instantiated
        // effect, the effect's own type parameters render as the instance's
        // arguments (`T` -> `i32` for `Random<i32>`).
        if arg_strs.is_empty() {
            if let Some(rendered) = self.type_subst.get(name) {
                return rendered.clone();
            }
        }
        let args = if arg_strs.is_empty() {
            String::new()
        } else {
            format!("<{}>", arg_strs.join(", "))
        };
        if let Some(rs) = crate::intrinsics::type_name(name) {
            // [rs-collections] Naming the type is enough to need its
            // runtime module imported, whether or not this module also
            // calls into it.
            if matches!(name, "Set" | "Map" | "SortedSet" | "SortedMap") {
                self.needs_collections = true;
            }
            // [rs-actor] The scheduler's handles are *not* generic in Rust:
            // an addr is an index and a token is one type, so the Salvo type
            // argument (the effect an addr serves, the payload a token carries)
            // has no rendering here — the generated message enum carries it.
            if matches!(name, "Addr" | "Pool" | "Reply") {
                self.needs_scheduler = true;
                return rs.to_string();
            }
            return format!("{rs}{args}");
        }
        if name == "Any" {
            // [backend-never-wrong] No Rust mapping yet.
            self.error("the `Any` type is not supported by the rust backend yet");
            return "()".to_string();
        }
        // [backend-intrinsic] [backend-never-wrong] The general form of the
        // same refusal: an `intrinsic type` without a mapping has no Rust
        // spelling, so passing its Salvo name through would emit a dangling
        // reference for rustc to trip over instead of reporting the gap
        // here. (`Addr`, `Reply` and `Pool` are exactly this until the
        // actor structs land.)
        if self.symbols.intrinsic_types.contains_key(name) {
            self.error(format!(
                "the `{name}` type is not supported by the rust backend yet"
            ));
            return format!("{}{args}", rs_ident(name));
        }
        // Structs, generics, effects, and unknown names pass through.
        format!("{}{args}", rs_ident(name))
    }

    /// Unions lower to the enum encoding [rs-union-enums]: `None` arms
    /// become an outer `Option`, a single remaining arm is `Option<T>`,
    /// two or more become `UnionN<...>`.
    fn emit_union_type(&mut self, arms: &[Type]) -> String {
        let mut nullable = false;
        let mut value_arms: Vec<(String, String)> = Vec::new();
        for arm in arms {
            if is_none_type(arm) {
                nullable = true;
                continue;
            }
            let quals: Vec<&str> = match arm {
                Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                    qualifiers.iter().map(|q| q.name.name.as_str()).collect()
                }
                _ => Vec::new(),
            };
            let emitted = self.emit_type(arm);
            let key = format!("{quals:?}|{emitted}");
            if !value_arms.iter().any(|(k, _)| *k == key) {
                value_arms.push((key, emitted));
            }
        }
        let inner = match value_arms.len() {
            0 => "()".to_string(),
            1 => value_arms[0].1.clone(),
            n => {
                self.union_sizes.insert(n);
                let args: Vec<String> = value_arms.into_iter().map(|(_, e)| e).collect();
                format!("Union{n}<{}>", args.join(", "))
            }
        };
        if nullable {
            format!("Option<{inner}>")
        } else {
            inner
        }
    }

    /// [platform-effect] Whether an effect is host-implemented, from either
    /// the checker's resolved type or (in unchecked contexts) its rendered
    /// name — the same table/fallback pair every effect lookup here uses.
    fn is_platform_effect(&self, ty: Option<&Ty>, rendered: &str) -> bool {
        if let Some(Ty::Named { name, .. }) = ty.map(Ty::strip_quals) {
            if let Some(e) = self.symbols.effects.get(name.as_str()) {
                return e.platform;
            }
        }
        // An effect's rendered Rust name is its Salvo name through
        // `rs_ident`, minus any generic arguments; platform effects may not
        // be generic, so the base name is the whole name.
        let base = rendered.split('<').next().unwrap_or(rendered);
        self.symbols.effects.keys().any(|n| rs_ident(n) == base)
            && self
                .symbols
                .effects
                .iter()
                .any(|(n, e)| rs_ident(n) == base && e.platform)
    }

    /// [platform-effect] Whether this fn's declared effect list mentions a
    /// platform effect — which is what turns `main` into an entry point the
    /// host calls rather than the crate's own `main`.
    fn declares_platform_effect(&self, f: &FnDecl) -> bool {
        f.effects.iter().flatten().any(|eff| match eff {
            EffectRef::Effect(r) | EffectRef::LocalEffect(r) => self
                .symbols
                .effects
                .get(r.name.name.as_str())
                .is_some_and(|e| e.platform),
            _ => false,
        })
    }

    /// [cmp-carry] The hash/equality **markers** a keyed container built here is
    /// kept by, rendered as one argument list since the pair always travels
    /// together [col-membership] — the host's own when its type names neither.
    fn keyed_markers(&mut self, span: Span) -> String {
        self.needs_collections = true;
        match self.container_identity_markers(span) {
            Some(pair) => pair,
            None => "HostHash, HostEq".to_string(),
        }
    }

    /// [cmp-carry] The generated markers for a `Set`/`Map` whose type names its
    /// hash and equality. `None` for the canonical path.
    fn container_identity_markers(&mut self, span: Span) -> Option<String> {
        let ty = self.checked.expr_ty.get(&(self.file_idx, span))?.clone();
        let Ty::Named { name, args } = ty.strip_quals() else {
            return None;
        };
        if !matches!(name.as_str(), "Set" | "Map") {
            return None;
        }
        let subject = args.first()?.clone();
        let ids: Vec<FnId> = args
            .iter()
            .filter_map(|a| match a {
                Ty::FnName(id) => Some(id.clone()),
                _ => None,
            })
            .collect();
        if ids.len() != 2 {
            return None;
        }
        let hash = self.identity_marker(&ids[0], &subject, "Hash")?;
        let eq = self.identity_marker(&ids[1], &subject, "Eq")?;
        Some(format!("{hash}, {eq}"))
    }

    /// [cmp-carry] One generated marker: a zero-sized struct and the impl of the
    /// capability's trait for it, registered on first use. The declaration behind
    /// the name is the *checker's* answer — resolving it here could disagree with
    /// what the checker type-checked against.
    fn identity_marker(&mut self, id: &FnId, subject: &Ty, kind: &str) -> Option<String> {
        let marker = format!("__{kind}_{}", rs_ident(&id.to_string().replace('@', "__")));
        if !self.ordering_markers.contains_key(&marker) {
            let decl = self
                .checked
                .carried_identities
                .get(&(id.clone(), subject.clone()))
                .copied()
                .and_then(|k| self.fn_by_key(k));
            let Some(decl) = decl else {
                self.error(format!(
                    "the `{id}` this collection is keyed by has no resolved declaration"
                ));
                return None;
            };
            let target = self.rust_fn_name(decl);
            let elem = self.rust_ty(subject);
            let body = match kind {
                "Hash" => format!(
                    "    fn hash(__v: &{elem}) -> i64 {{ {target}(__v) }}"
                ),
                "Eq" => format!(
                    "    fn eq(__a: &{elem}, __b: &{elem}) -> bool {{ {target}(__a, __b) }}"
                ),
                _ => format!(
                    "    fn cmp(__a: &{elem}, __b: &{elem}) -> i32 {{ {target}(__a, __b) }}"
                ),
            };
            self.needs_collections = true;
            self.ordering_markers.insert(marker.clone(), elem.clone());
            self.generated_items.push(format!(
                "\npub struct {marker};\nimpl Salvo{kind}<{elem}> for {marker} {{\n{body}\n}}\n"
            ));
        }
        Some(marker)
    }

    /// [cmp-carry] The marker type naming the ordering the keyed container this
    /// expression builds is kept by — `HostOrd` when its type names
    /// none, which is the canonical path every program took before orderings
    /// could be named, and a generated marker when it names one.
    ///
    /// Registers the marker's definition on first use, so the module emits one
    /// zero-sized struct and one `SalvoCmp` impl per ordering it actually
    /// mentions.
    fn ordering_marker_for(&mut self, span: Span) -> Option<String> {
        let ty = self.checked.expr_ty.get(&(self.file_idx, span))?.clone();
        let Ty::Named { name, args } = ty.strip_quals() else {
            return None;
        };
        if !matches!(name.as_str(), "SortedSet" | "SortedMap") {
            return None;
        }
        // Building one needs the runtime module, whether or not this module also
        // names the type [rs-collections].
        self.needs_collections = true;
        let subject = args.first()?.clone();
        let Some(Ty::FnName(id)) = args.iter().find(|a| matches!(a, Ty::FnName(_))) else {
            // No identity in the type: the host's own ordering, as ever.
            return Some("HostOrd".to_string());
        };
        let marker = format!("__Cmp_{}", rs_ident(&id.to_string().replace('@', "__")));
        if !self.ordering_markers.contains_key(&marker) {
            // The declaration behind the name is the *checker's* answer
            // [cmp-carry]: resolving it here could disagree with what the
            // checker type-checked against.
            let key = (id.clone(), subject.clone());
            let Some(decl) = self
                .checked
                .carried_identities
                .get(&key)
                .copied()
                .and_then(|k| self.fn_by_key(k))
            else {
                self.error(format!(
                    "the ordering `{id}` of this collection has no resolved declaration"
                ));
                return Some("HostOrd".to_string());
            };
            let target = self.rust_fn_name(decl);
            let elem = self.rust_ty(&subject);
            self.needs_collections = true;
            self.ordering_markers.insert(marker.clone(), elem.clone());
            self.generated_items.push(format!(
                "\npub struct {marker};\n\
                 impl SalvoCmp<{elem}> for {marker} {{\n    \
                 fn cmp(__a: &{elem}, __b: &{elem}) -> i32 {{ {target}(__a, __b) }}\n\
                 }}\n"
            ));
        }
        Some(marker)
    }

    /// Renders a checker `Ty` as Rust (qualifiers erased, unions as the
    /// enum encoding). Must agree with `emit_type` on the same source
    /// type — checker-resolved effect types key the effect environment.
    fn rust_ty(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Named { name, args } => {
                // [cmp-carry] The identities a keyed container carries are the
                // checker's, not a rendering: the store holds the ordering, so
                // the emitted type names only the element.
                let args: Vec<Ty> = args
                    .iter()
                    .filter(|a| !matches!(a, Ty::FnName(_)))
                    .cloned()
                    .collect();
                let args = &args;
                // [monitor-handler] [rs-monitor] A plain effect's addr is the
                // effect's lock wrapper, not a scheduler index.
                if name == "Addr" && args.len() == 1 {
                    if let Ty::Named { name: effect, args: eff_args } = args[0].strip_quals() {
                        let effect = effect.clone();
                        let eff_args = eff_args.clone();
                        if self
                            .symbols
                            .effects
                            .get(effect.as_str())
                            .is_some_and(|e| !e.is_actor)
                        {
                            let rendered: Vec<String> =
                                eff_args.iter().map(|a| self.rust_ty(a)).collect();
                            return self.plain_addr_rendering(&effect, &rendered);
                        }
                    }
                }
                let arg_strs: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
                self.emit_named_parts(name, &arg_strs)
            }
            Ty::Qualified { quals, base } => {
                // [proj-type] `proj X` is `&X` (a Copy scalar's `proj` never
                // reaches the type — the checker erases it). Except over a
                // bare in-scope generic with no lifetime context: `proj T`
                // at the definition site is `T` — the instantiation carries
                // the borrow (see `emit_type`).
                if quals.iter().any(|q| q.name == "proj") && !Self::is_copy_ty(base) {
                    let bare_generic = self.proj_lifetime.is_none()
                        && matches!(
                            &**base,
                            Ty::Named { name, args } if args.is_empty() && self.generics.contains(name)
                        );
                    if !bare_generic {
                        let inner = self.rust_ty(base);
                        let lt = self
                            .proj_lifetime
                            .clone()
                            .map(|l| format!("{l} "))
                            .unwrap_or_default();
                        return format!("&{lt}{inner}");
                    }
                }
                // [type-canbe-mut] Every other qualifier erases here, `Mut`
                // included: Rust carries mutability in the binding, not the
                // type [rs-borrows].
                self.rust_ty(base)
            }
            Ty::Union(_) => {
                let value_arms = ty.value_arms();
                let inner = match value_arms.len() {
                    0 => "()".to_string(),
                    1 => self.rust_ty(value_arms[0]),
                    n => {
                        self.union_sizes.insert(n);
                        let args: Vec<String> =
                            value_arms.iter().map(|a| self.rust_ty(a)).collect();
                        format!("Union{n}<{}>", args.join(", "))
                    }
                };
                if ty.has_none_arm() {
                    format!("Option<{inner}>")
                } else {
                    inner
                }
            }
            Ty::Tuple(elems) => {
                let strs: Vec<String> = elems.iter().map(|e| self.rust_ty(e)).collect();
                format!("({})", strs.join(", "))
            }
            Ty::Array(elem) => format!("Vec<{}>", self.rust_ty(elem)),
            Ty::Fn { params, ret, .. } => {
                let ps: Vec<String> = params.iter().map(|p| self.rust_ty(p)).collect();
                format!("impl Fn({}) -> {}", ps.join(", "), self.rust_ty(ret))
            }
            Ty::Var(name) => name.clone(),
            // [cmp-carry] An identity is not a value's type. On a
            // *qualifier* it erases with the qualifier [qual-erasure], and a
            // keyed container reads its own slots; reaching the general
            // renderer means one was written where a type belongs, which the
            // checker refuses — so this is an error, never output
            // [backend-never-wrong].
            Ty::FnName(id) => {
                self.error(format!(
                    "the function identity `{id}` reached code generation as a \
                     type: an identity may only fill a declaration's fn slot"
                ));
                "()".to_string()
            }
            Ty::Any | Ty::Unknown => {
                self.error(
                    "a value of unknown/`Any` type reached rust code generation \
                     (the rust backend cannot represent it yet)",
                );
                "()".to_string()
            }
            Ty::Never => "()".to_string(),
        }
    }
    fn is_copy_ty(ty: &Ty) -> bool {
        matches!(
            ty.strip_quals(),
            Ty::Named { name, args }
                if args.is_empty()
                    && matches!(
                        name.as_str(),
                        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                    )
        )
    }

    /// [monitor-handler] [rs-monitor] Whether a type is the `Addr` of a
    /// **plain** effect — a shared monitor's handle, which lowers to the
    /// effect's `__Mon_E` lock wrapper rather than to the scheduler's
    /// `usize` index.
    fn is_plain_addr_ty(&self, ty: &Ty) -> bool {
        let Ty::Named { name, args } = ty.strip_quals() else {
            return false;
        };
        if name != "Addr" || args.len() != 1 {
            return false;
        }
        let Ty::Named { name: effect, .. } = args[0].strip_quals() else {
            return false;
        };
        self.symbols
            .effects
            .get(effect.as_str())
            .is_some_and(|e| !e.is_actor)
    }

    /// [monitor-handler] [rs-monitor] The rendering of `Addr<E>` for a plain
    /// effect `E`: the `__Mon_E` wrapper, generic exactly as the effect is —
    /// `Addr<Random<Int>>` is `__Mon_Random<i64>` — so the instantiation the
    /// addr's own type argument carries is the wrapper's.
    fn plain_addr_rendering(&mut self, effect: &str, args: &[String]) -> String {
        let arity = self
            .symbols
            .effects
            .get(effect)
            .map_or(0, |e| e.generics.len());
        if args.len() != arity {
            // An uninstantiated generic effect has no concrete wrapper; the
            // checker resolves the instance everywhere it can, so this is
            // a leniency path, not a rule [type-unknown-lenient].
            self.error(format!(
                "the shared handle of effect `{effect}` needs its {arity} type \
                 argument(s), and this mention carries {}",
                args.len()
            ));
            return "()".to_string();
        }
        if args.is_empty() {
            monitor_struct_name(effect)
        } else {
            format!("{}<{}>", monitor_struct_name(effect), args.join(", "))
        }
    }

    /// [rs-monitor] The turbofish naming a generic effect instance's type
    /// arguments on its wrapper's constructor (`::<i64>` for `Random<Int>`),
    /// empty for a non-generic effect.
    fn effect_instance_turbofish(&mut self, ty: &Ty) -> String {
        let Ty::Named { args, .. } = ty.strip_quals() else {
            return String::new();
        };
        if args.is_empty() {
            return String::new();
        }
        let args = args.clone();
        let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
        format!("::<{}>", rendered.join(", "))
    }

    /// Whether an AST type is a Copy scalar (post alias expansion).
    /// [implicit-param] The Rust type of an implicit parameter: a borrowed
    /// **`dyn`** `FnMut`. Built from the checker's `Ty` rather than an AST
    /// type, since a group's members were never written in this signature.
    ///
    /// `dyn` rather than `impl`, uniformly, for two reasons that pull the
    /// same way. An effect member's implicits land in a trait used as
    /// `&mut dyn E` [rs-effects], where `impl Trait` in argument position
    /// would cost object safety. And *forwarding* has to compose in every
    /// direction: a member forwarding to a plain fn would otherwise hand a
    /// `dyn` value to an `impl` (`Sized`) parameter, which rustc refuses —
    /// so one convention everywhere is both simpler and the only sound
    /// choice. The cost is an indirect call, which is what every effect
    /// member call already pays.
    /// [rs-proj-arm] An implicit parameter's type, with the union arms its
    /// declaration marks as borrows rendered as references: `Yield`'s `next`
    /// is `&mut dyn FnMut(&mut It) -> Union2<&T, Finished>`.
    fn implicit_param_type_of(&mut self, imp: &salvo_core::ImplicitParam) -> String {
        self.implicit_param_type_borrowing(&imp.ty, &imp.borrowed_arms)
    }

    /// [implicit-forward] [rs-fn-param-convention] An implicit parameter passed
    /// straight through to an inner call — `&mut *cmp` — **unless the two sides
    /// render it differently**, in which case an adapter closure bridges them.
    ///
    /// The conventions can differ because they are computed from *types*, and
    /// the two fns may be generic to different degrees: a
    /// `drain(heap: Heap<Int, ?cmp> …)` holds `&mut dyn FnMut(i32, i32) -> i32`
    /// (a Copy scalar goes by value) while the generic `heap_pop<T>` it calls
    /// wants `FnMut(&T, &T)` (a type variable is never known to be Copy). The
    /// checker sees one capability and is right to; the rendering is where they
    /// part. Without the bridge the emitted call was rustc's E0308 — a
    /// [backend-never-wrong] violation, found 2026-09-22 while making the heap
    /// demo run, and the same disagreement [rs-fn-param-convention] records for
    /// a *lambda* argument, one level further out.
    fn forwarded_implicit(&mut self, name: &str, span: salvo_syntax::Span) -> String {
        let local = rs_ident(name);
        let want = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|k| self.checked.implicit_params.get(k))
            .and_then(|ps| ps.iter().find(|p| p.name == name))
            .map(|p| p.ty.clone());
        let have = self
            .implicits
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.ty.clone());
        let (Some(want), Some(have)) = (want, have) else {
            return format!("&mut *{local}");
        };
        // Derived from the one function that renders a fn type's parameters, so
        // the two answers cannot drift apart the way the conventions did.
        let by_ref = |r: &[String]| -> Vec<bool> {
            r.iter().map(|s| s.starts_with('&')).collect()
        };
        let wanted = by_ref(&self.fn_ty_param_renderings(&want));
        let held = by_ref(&self.fn_ty_param_renderings(&have));
        if wanted == held {
            return format!("&mut *{local}");
        }
        let args: Vec<String> = wanted
            .iter()
            .enumerate()
            .map(|(i, want_ref)| {
                let hold_ref = held.get(i).copied().unwrap_or(*want_ref);
                match (*want_ref, hold_ref) {
                    // Handed a borrow, wanted by value: by-value means the type
                    // is a Copy scalar, so a deref is the whole conversion.
                    (true, false) => format!("*__i{i}"),
                    (false, true) => format!("&__i{i}"),
                    _ => format!("__i{i}"),
                }
            })
            .collect();
        let params: Vec<String> = (0..wanted.len()).map(|i| format!("__i{i}")).collect();
        format!(
            "&mut |{}| {local}({})",
            params.join(", "),
            args.join(", ")
        )
    }

    fn implicit_param_type_borrowing(&mut self, ty: &Ty, borrowed_arms: &[usize]) -> String {
        let Ty::Fn { params, ret, .. } = ty.strip_quals() else {
            return self.rust_ty(ty);
        };
        let ps = self.fn_ty_param_renderings(ty);
        let _ = params;
        let ret = if ret.is_none_ty() {
            String::new()
        } else if !borrowed_arms.is_empty() {
            format!(
                " -> {}",
                self.union_ty_with_borrowed_arms(ret, borrowed_arms)
            )
        } else {
            format!(" -> {}", self.rust_ty(ret))
        };
        format!("&mut dyn FnMut({}){ret}", ps.join(", "))
    }

    /// [rs-proj-arm] A union type with the given value-arm indices rendered
    /// as `&T`.
    fn union_ty_with_borrowed_arms(&mut self, ty: &Ty, borrowed: &[usize]) -> String {
        let arms = ty.value_arms();
        if arms.len() < 2 {
            return self.rust_ty(ty);
        }
        self.union_sizes.insert(arms.len());
        let rendered: Vec<String> = arms
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let a = (*a).clone();
                let r = self.rust_ty(&a);
                // [proj-type] An arm already typed `proj` renders `&T` on
                // its own.
                if borrowed.contains(&i) && !a.is_proj() {
                    format!("&{r}")
                } else {
                    r
                }
            })
            .collect();
        let inner = format!("Union{}<{}>", arms.len(), rendered.join(", "));
        if ty.has_none_arm() {
            format!("Option<{inner}>")
        } else {
            inner
        }
    }

    /// [rs-iter-pass] A checker fn type as an **owned** `impl Fn`, the
    /// convention an iterator fn's callbacks arrive under: its pass calls them
    /// long after the call returns, so nothing may be borrowed.
    fn owned_fn_ty(&mut self, ty: &Ty) -> String {
        let Ty::Fn { ret, .. } = ty.strip_quals() else {
            return self.rust_ty(ty);
        };
        let ret = ret.clone();
        let ps = self.fn_ty_param_renderings(ty);
        let ret = if ret.is_none_ty() {
            String::new()
        } else {
            format!(" -> {}", self.rust_ty(&ret))
        };
        format!("impl Fn({}){ret}", ps.join(", "))
    }

    /// [rs-fn-param-convention] [fn-contract] The parameters of a *checker*
    /// fn type, rendered the way `emit_type`'s `Type::Fn` arm renders a
    /// written one: a kept `Mut` position borrows mutably, a kept non-`Copy`
    /// one borrows, and a moved one is owned.
    ///
    /// The contract is what makes an `?add: (dest: Mut D, elem: U) -> [dest:
    /// Mut] None` implicit render `FnMut(&mut Vec<i32>, i32)`. Reading only
    /// the parameter *types* (which erase `Mut` on this backend) rendered
    /// `FnMut(Vec<i32>, i32)` and the adapter could not mutate what it was
    /// handed — invisible until an implicit had a `Mut` parameter, which
    /// [seq-into]'s is the first to have.
    fn fn_ty_param_renderings(&mut self, ty: &Ty) -> Vec<String> {
        let Ty::Fn {
            params, contract, ..
        } = ty.strip_quals()
        else {
            return Vec::new();
        };
        let contract = contract.clone();
        let params = params.clone();
        params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let base = self.rust_ty(p);
                let entry = contract.as_ref().and_then(|c| c.get(i));
                let (kept, mutable, lent) = match entry {
                    Some(e) => (e.kept, e.mutable, e.lent),
                    // No contract means "keeps everything" [fn-contract].
                    None => (true, p.quals().iter().any(|q| q.name == "Mut"), false),
                };
                // A kept **`Mut`** position borrows mutably — you cannot
                // mutate what you were handed by value — and every other kept
                // position borrows too unless its type is a Copy scalar, which
                // is the convention a *written* fn type has always rendered
                // ([rs-fn-param-convention]: `f: (T) -> U` is `FnMut(&T)`).
                // Implicits used to be by-value here, which made a kept
                // position a *move* on this backend: `min_of<T>(a: T, b: T,
                // ?Ordered<T>)` calling `cmp(a, b)` and then returning `a`
                // was E0382, so the comparison capability was unusable over
                // anything but a Copy scalar (fixed 2026-09-21, the ordering
                // round's step 1).
                // [rs-proj-lends] A *lent* position (`[c: proj]`) is `&T` for
                // its own reason: the result holds a borrow of it, which a
                // by-value parameter could not outlive (E0515).
                if kept && mutable {
                    format!("&mut {base}")
                } else if kept && lent {
                    // The result's type (`It`) is fixed at the call site, so
                    // the borrow it holds cannot be a fresh per-call lifetime:
                    // it is the enclosing fn's borrow of the parameter this
                    // position is named after, `'c`, which `emit_fn` names on
                    // that parameter too. Recorded here, applied there.
                    let named = entry.and_then(|e| e.name.clone());
                    if let Some(n) = named {
                        self.lent_position_params.push(n);
                    }
                    format!("&'c {base}")
                } else if kept && !Self::is_copy_ty(p) {
                    format!("&{base}")
                } else {
                    base
                }
            })
            .collect()
    }

    /// [rs-fn-param-convention] How the *declaration* of a fn type renders
    /// each of its parameters, so a lambda passed into that position binds
    /// the same way. Kept in step with the `Type::Fn` arm of `emit_type`
    /// by construction: same contract, same `Copy` test, same order.
    fn fn_type_param_conventions(&mut self, fn_ty: &Type) -> Vec<BindKind> {
        let Type::Fn {
            params,
            param_names,
            deductions,
            ..
        } = fn_ty
        else {
            return Vec::new();
        };
        (0..params.len())
            .map(|i| {
                let (kept, is_mut) = ast_fn_param_contract(params, param_names, deductions, i);
                if kept && is_mut {
                    BindKind::RefMut
                } else if kept && !self.is_copy_ast_type(&params[i]) {
                    BindKind::Ref
                } else {
                    BindKind::Owned
                }
            })
            .collect()
    }

    fn is_copy_ast_type(&mut self, ty: &Type) -> bool {
        let Type::Named { qualifiers, base } = ty else {
            return false;
        };
        if !qualifiers.is_empty() || !base.args.is_empty() {
            return false;
        }
        if let Some(alias) = self.symbols.type_aliases.get(base.name.name.as_str()) {
            if let Some(target) = &(*alias).alias.clone() {
                return self.is_copy_ast_type(target);
            }
        }
        matches!(
            base.name.name.as_str(),
            "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
        )
    }
}

/// [rs-effect-fusion] Where a generated forwarding impl sends its members.
/// Only the *dependent handler* case remains: every other reach — inherited
/// effects, an independent handler, a `__Deps_H` adapter — is a Has-accessor
/// impl now ([`emit_has_impl`]), not a member-by-member forward.
enum Forward {
    /// A dependent handler: destructure `&mut self` into disjoint field
    /// borrows, then call through the handler's `__Impl_H` trait with a
    /// `__Deps_H` adapter over the provider.
    Dependent { trait_name: String, deps: usize },}

/// Is a *rendered* Rust type one of the Copy scalars a member parameter
/// passes by value [rs-borrows]? The rendered form is what a substituted
/// effect generic leaves behind, where the AST-level check
/// (`is_copy_ast_type`) sees only the type parameter's name.
fn is_copy_rendered(rendered: &str) -> bool {
    matches!(
        rendered,
        "i32" | "i64" | "f32" | "f64" | "bool" | "char" | "u8"
    )
}

/// An effect as a *type* (impl target, trait bound): `Random<i32>`.
fn trait_type(name: &str, args: &[String]) -> String {
    if args.is_empty() {
        rs_ident(name)
    } else {
        format!("{}<{}>", rs_ident(name), args.join(", "))
    }
}

/// [rs-effect-fusion] The accessor ("Has") trait of an effect *declaration*:
/// `__Has_Random`, generic exactly as the effect is, declared next to it —
/// so its identity travels with the effect through the same globs
/// [rs-imports] and no shared file is needed.
fn has_trait_name(name: &str) -> String {
    format!("__Has_{}", rs_ident(name))
}

/// [actor-use-addr] The forwarding stub of a protocol: `__Stub_Counter`, the
/// effect implemented by sending to an addr.
fn stub_struct_name(effect: &str) -> String {
    format!("__Stub_{}", rs_ident(effect))
}

/// [monitor-handler] [rs-monitor] The lock wrapper of a **plain** effect:
/// `__Mon_Random`, the effect implemented by locking a shared instance and
/// delegating — what an `Addr<Random>` *is* in Rust, and what a monitor
/// spawn answers. Per effect, like the send stub: a holder knows only the
/// effect the handle serves.
fn monitor_struct_name(effect: &str) -> String {
    format!("__Mon_{}", rs_ident(effect))
}

/// [rs-monitor] The clone-box supertrait behind the handle: what lets a
/// `Box<dyn __Share_E>` be cloned, which is what makes the handle freely
/// copyable whatever kind of value sits inside it.
fn share_trait_name(effect: &str) -> String {
    format!("__Share_{}", rs_ident(effect))
}

/// [rs-monitor] The monitor's lock adapter: `__Lock_E<H>`, generic over the
/// handler, wrapped by monitor spawns only — a mixed handler's façade must
/// never sit behind it.
fn lock_struct_name(effect: &str) -> String {
    format!("__Lock_{}", rs_ident(effect))
}

/// [mixed-handler] [rs-mixed] The façade of a mixed handler:
/// `__Fac_CyclicRandom`, the servant's addr plus the constructor parameters
/// plus the sync member bodies — per *handler*, since the bodies are its.
fn facade_struct_name(handler: &str) -> String {
    format!("__Fac_{}", rs_ident(handler))
}

/// [rs-actor] The message enum of a protocol: `__Msg_Counter`. Named after
/// the *effect*, since that is what a sender knows [actor-types].
fn msg_enum_name(effect: &str) -> String {
    format!("__Msg_{}", rs_ident(effect))
}

/// [rs-actor] [actor-replyto] The parked-continuation enum of a protocol:
/// `__Cont_Counter`. Beside the message enum and named the same way — a
/// continuation is a member invocation waiting for its last argument.
fn cont_enum_name(effect: &str) -> String {
    format!("__Cont_{}", rs_ident(effect))
}

/// [mixed-handler] [rs-mixed] The mixed servant's members a continuation
/// could target: a handler-local `send fn` with at least one parameter (the
/// trailing one is the answer) — the handler-keyed twin of
/// `Emitter::cont_targets`, named as `__Msg_H`'s variants are.
fn mixed_cont_targets(h: &HandlerDecl) -> Vec<&FnDecl> {
    h.fns
        .iter()
        .filter(|f| f.is_send && f.params.iter().any(|p| !p.implicit))
        .collect()
}

/// [rs-actor] One variant of it, named after the member. Upper-camel, since
/// a Rust variant that is not would warn in generated code.
fn msg_variant_name(member: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in member.chars() {
        if c == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// [rs-actor] The generated body a spawn boxes: `__Actor_Counting`, wrapping
/// the handler instance and dispatching messages onto its members.
fn actor_struct_name(handler: &str) -> String {
    format!("__Actor_{}", rs_ident(handler))
}

/// [rs-actor] The **flat provider** of a dependent handler's actor:
/// `__Prov_Counting<__D0, …>`, one field per declared dependency, generic in
/// the instance each spawn supplies and implementing every dependency's
/// Has-accessor trait. It is what a child holds in place of the fusion a
/// `use` site would have built around it [rs-effect-fusion].
fn prov_struct_name(handler: &str) -> String {
    format!("__Prov_{}", rs_ident(handler))
}

/// The accessor method of an effect's Has trait: `__get_Random`. One per
/// trait, so two *instances* of a generic effect disambiguate with the
/// trait's turbofish, never by name mangling.
fn has_getter_name(name: &str) -> String {
    format!("__get_{}", rs_ident(name))
}

/// A Has trait as a *bound / impl target*: `__Has_Random<i32>`.
fn has_trait_type(name: &str, args: &[String]) -> String {
    trait_type(&has_trait_name(name), args)
}

/// A Has trait as a *call path* (UFCS accessor dispatch):
/// `__Has_Random::<i32>`.
fn has_trait_path(name: &str, args: &[String]) -> String {
    trait_path(&has_trait_name(name), args)
}

/// [rs-effect-fusion] One Has-accessor impl: the fused value's way to an
/// effect instance. `body` is the accessor's expression — the owned handler
/// (`&mut self.__h`), a forward through the provider, a dyn field, or
/// `self` for a dependent handler's effect (whose raw impl lives on the
/// fusion itself).
fn emit_has_impl(
    impl_generics: &str,
    self_ty: &str,
    effect_name: &str,
    effect_args: &[String],
    body: &str,
) -> String {
    format!(
        "\nimpl{impl_generics} {} for {self_ty} {{\n    fn {}(&mut self) -> &mut dyn {} {{\n        {body}\n    }}\n}}\n",
        has_trait_type(effect_name, effect_args),
        has_getter_name(effect_name),
        trait_type(effect_name, effect_args)
    )
}

/// An effect as a *call path* (UFCS member dispatch): `Random::<i32>`.
fn trait_path(name: &str, args: &[String]) -> String {
    if args.is_empty() {
        rs_ident(name)
    } else {
        format!("{}::<{}>", rs_ident(name), args.join(", "))
    }
}

/// Does `code` use `name` as a whole identifier?
fn mentions_ident(code: &str, name: &str) -> bool {
    let bytes = code.as_bytes();
    let mut from = 0;
    while let Some(at) = code[from..].find(name) {
        let start = from + at;
        let end = start + name.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// What kind of fn is being emitted (parameter-mode and naming rules).
#[derive(Clone, Copy)]
enum FnStyle<'p> {
    TopLevel,
    QualifierFn,
    HandlerMember(&'p HandlerDecl),
    /// [rs-effect-fusion] A *dependent* handler's member, emitted into
    /// `impl __Impl_H for H` with the dependencies as one fused parameter.
    /// The dependencies are the handler's own effect list, so the handler is
    /// all this needs to carry [effect-handler-deps].
    DepMember(&'p HandlerDecl),
    /// The same, signature only, for the generated `trait __Impl_H`.
    DepMemberSig(&'p HandlerDecl),
}

impl<'p> Emitter<'p> {
    // ================= statements =================

    fn emit_block_stmts(&mut self, block: &Block, indent: usize, ctx: StmtCtx) -> String {
        let mut out = String::new();
        // [rs-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the block *rewrites* the outer entries to
        // thread through the inner fusion, which dies with the block.
        let saved_env = self.effect_env.clone();
        // [rs-exit-splice] Splices registered inside this block run when it
        // ends.
        let splice_floor = self.exit_splices.len();
        for stmt in &block.stmts {
            out.push_str(&self.emit_stmt(stmt, indent, ctx));
        }
        // A block whose last statement exits already ran them there.
        if block_terminates(block) {
            self.exit_splices.truncate(splice_floor);
        } else {
            out.push_str(&self.splice_exits(splice_floor, indent, true));
        }
        self.effect_env = saved_env;
        out
    }

    /// [rs-exit-splice] Renders the splices registered at or above `floor`,
    /// latest first, re-indented to `indent`. `pop` discards them (the block
    /// they belong to is ending); an early exit leaves them in place, since
    /// the block's own exit runs them too.
    fn splice_exits(&mut self, floor: usize, indent: usize, pop: bool) -> String {
        if self.exit_splices.len() <= floor {
            return String::new();
        }
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        for body in self.exit_splices[floor..].iter().rev() {
            for line in body.lines() {
                if line.trim().is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&format!("{pad}{line}\n"));
                }
            }
        }
        if pop {
            self.exit_splices.truncate(floor);
        }
        out
    }

    /// [rs-exit-splice] The splices an early exit runs: every one registered
    /// inside the construct being left.
    fn exit_splice_code(&mut self, indent: usize, loop_exit: bool) -> String {
        let floor = if loop_exit {
            self.loop_splice_floors
                .last()
                .copied()
                .unwrap_or(self.splice_floor)
        } else {
            self.splice_floor
        };
        self.splice_exits(floor, indent, false)
    }

    fn emit_stmt(&mut self, stmt: &Stmt, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
        self.expr_indent = indent;
        match stmt {
            // [fn-rename] Erased: a rename is a compile-time name for an
            // overload the call sites already resolved [fn-overload-scope].
            Stmt::Rename(_) => String::new(),
            Stmt::Let {
                pattern,
                ty,
                value,
                span,
            } => self.emit_let(pattern, ty.as_ref(), value, *span, indent),
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let t = self.emit_raw(target);
                // [proj-field] Assigning a `proj` field re-points a borrow:
                // the field is `&'s T`, so the value is borrowed, not moved
                // or cloned.
                if self.assigns_proj_field(target) {
                    let v = self.borrowed_arg(value);
                    return format!("{pad}{t} = {v};\n");
                }
                // A bare-identifier source is a fate link, not a move
                // [fate-link] — unless the assignment is a move-mode bind
                // event [fate-move-mode].
                let v = self.emit_bound_value(value, *span);
                format!("{pad}{t} = {v};\n")
            }
            // [expr-escape] The escapes are expressions now (2026-09-21), so
            // they arrive wrapped in a statement. These arms come before the
            // general `Stmt::Expr` case and keep every rule they had: the unit
            // return, the loop-result routing and the exit splices.
            Stmt::Expr(Expr::Return { value, .. }) => match (ctx, value.as_deref()) {
                // [rs-none-unit] `None` is Rust's `()`: a fn returning it has
                // no return type at all, so `return None;` is E0308. The
                // literal has nothing to evaluate; anything else may be a
                // call, so it runs for its effects and the return is bare.
                (_, Some(v))
                    if self.ret_is_unit && self.ty_of(v.span()).is_some_and(|t| t.is_none_ty()) =>
                {
                    let evaluated = if matches!(v, Expr::Ident(id) if id.name == "None") {
                        String::new()
                    } else {
                        self.emit_stmt(&Stmt::Expr(v.clone()), indent, StmtCtx::Normal)
                    };
                    let splices = self.exit_splice_code(indent, false);
                    let unit = self.wrap_continue("()".to_string());
                    let value = if self.throw_message.is_some() {
                        format!(" {unit}")
                    } else {
                        String::new()
                    };
                    format!("{evaluated}{splices}{pad}return{value};\n")
                }
                (_, Some(v)) => {
                    let code = self.emit_return_value(v);
                    // [rs-exit-splice] The value is evaluated before the
                    // exit splices run, so it is hoisted into a
                    // temporary when any of them follow it.
                    let splices = self.exit_splice_code(indent, false);
                    if splices.is_empty() {
                        let code = self.wrap_continue(code);
                        format!("{pad}return {code};\n")
                    } else {
                        let tmp = self.fresh_splice_var();
                        let out = self.wrap_continue(tmp.clone());
                        format!("{pad}let {tmp} = {code};\n{splices}{pad}return {out};\n")
                    }
                }
                (_, None) => {
                    let splices = self.exit_splice_code(indent, false);
                    let unit = self.wrap_continue("()".to_string());
                    let value = if self.throw_message.is_some() {
                        format!(" {unit}")
                    } else {
                        String::new()
                    };
                    format!("{splices}{pad}return{value};\n")
                }
            },
            Stmt::Expr(Expr::Break { value, .. }) => {
                let value = value.as_deref();
                let target = self.loop_results.last().cloned().flatten();
                // [rs-exit-splice] Leaving the loop runs the
                // blocks registered inside it; the break value is
                // evaluated first.
                match (value, target) {
                    // [while-value] route the value into the enclosing
                    // loop's result local before breaking [rs-loop-value].
                    (Some(v), Some(result)) => {
                        if self.ty_of(v.span()).is_some_and(|t| t.is_none_ty()) {
                            let stmt = self.emit_expr_stmt(v, indent, ctx);
                            let splices = self.exit_splice_code(indent, true);
                            format!("{stmt}{pad}{result} = None;\n{splices}{pad}break;\n")
                        } else {
                            let code = self.emit_loop_value_assign(v, &result);
                            let splices = self.exit_splice_code(indent, true);
                            format!("{pad}{code}\n{splices}{pad}break;\n")
                        }
                    }
                    (Some(v), None) => {
                        let stmt = self.emit_expr_stmt(v, indent, ctx);
                        let splices = self.exit_splice_code(indent, true);
                        format!("{stmt}{splices}{pad}break;\n")
                    }
                    (None, _) => {
                        let splices = self.exit_splice_code(indent, true);
                        format!("{splices}{pad}break;\n")
                    }
                }
            }
            Stmt::Expr(Expr::Continue { .. }) => {
                let splices = self.exit_splice_code(indent, true);
                format!("{splices}{pad}continue;\n")
            }
            Stmt::Use {
                handler,
                local,
                with_items,
                span,
            } => self.emit_use(handler, *local, with_items, *span, indent),
            Stmt::Expr(expr) => self.emit_expr_stmt(expr, indent, ctx),
        }
    }

    /// The code for a `return`'s value expression.
    ///
    /// [readonly-return] Derived-return fns return borrows: the place is
    /// borrowed (bare for already-`&` bindings), `Some(...)`-wrapped when
    /// the checker recorded the optional coercion, and `None` passes
    /// through.
    fn emit_return_value(&mut self, v: &Expr) -> String {
        // [rs-proj-struct] A borrowing struct literal is returned by value:
        // its `proj` fields are borrows (rendered in `emit_struct_lit`), the
        // struct itself is an owned cursor.
        if self.derived_return_fn && self.returns_borrowing_struct {
            return self.emit_expr(v);
        }
        // [rs-proj-arm] A union return with a `proj` arm: a value built for
        // the borrowing arm lends its argument (the `Option<&T>` local is
        // unwrapped to the `&T`, never cloned); a value for any other arm is
        // an ordinary owned return.
        if self.derived_return_fn && !self.proj_arm_ctors.is_empty() {
            if let Expr::Call { callee, args, .. } = v {
                if let Expr::Ident(name) = callee.as_ref() {
                    if self.proj_arm_ctors.contains(&name.name) && args.len() == 1 {
                        let arg = self.borrow_arg_for_arm(&args[0]);
                        let ctor = self.emit_expr_callee_name(v);
                        return self.apply_coercion(v.span(), format!("{ctor}({arg})"));
                    }
                }
            }
            return self.emit_expr(v);
        }
        if self.derived_return_fn && !matches!(v, Expr::Ident(id) if id.name == "None") {
            // A forwarded derived-return call already produces a borrow:
            // pass it through.
            let forwarded = matches!(
                v,
                Expr::Call { span, .. }
                    if self
                        .checked
                        .derived_calls
                        .contains_key(&(self.file_idx, *span))
            )
            // [rs-opt-borrow] `first(list)!` — an asserted optional
            // projection — is that same borrow, so it forwards too. The
            // ordinary `NonNull` path already renders it as `&T` (it asks
            // `is_optional_derived_call`), which is exactly what a
            // `proj[from: p]` return wants; without this arm the value
            // reached `borrow_place`, which knows only places, and a legal
            // program was refused. This is how a `NonEmpty` accessor
            // overload returns a non-optional projection.
            || matches!(
                v,
                Expr::NonNull { operand, .. } if self.is_optional_derived_call(operand)
            );
            if forwarded {
                return self.emit_expr(v);
            }
            let wrap = matches!(
                self.coercion_of(v.span()),
                Some(Coercion::WrapOption { .. })
            );
            if let Some(borrowed) = self.borrow_place(v) {
                return if wrap {
                    format!("Some({borrowed})")
                } else {
                    borrowed
                };
            }
            self.error(format!(
                "a `proj[from: ...]` return value must be a projection or \
                 alias of the annotated parameter (got `{v:?}`)"
            ));
        }
        self.emit_expr(v)
    }

    /// [yield-proj] The generic parameter that is the *element* of a
    /// combinator's `?Yield<It, T>` spread — the second spread argument, when
    /// it is a bare generic.
    fn yield_element_generic(&self, f: &FnDecl) -> Option<String> {
        for spread in &f.implicit_groups {
            if spread.name.name == "Yield" {
                if let Some(Type::Named { base, qualifiers }) = spread.args.get(1) {
                    if qualifiers.is_empty() && f.generics.iter().any(|g| g.name == base.name.name)
                    {
                        return Some(base.name.name.clone());
                    }
                }
            }
        }
        None
    }

    /// [copy-scalar-free] Whether a `next` that emits a borrow is filling a
    /// position whose element is a **Copy scalar written concretely** in the
    /// callee's spread (`?Yield<It, Int>`): there the borrow is copied out
    /// by the adapter. A generic element (`?Yield<It, T>`) is retagged to
    /// `&T` at the turbofish instead, and needs no adapter.
    fn scalar_position_wants_value(
        &self,
        next_decl: &FnDecl,
        callee: Option<&FnDecl>,
        implicit_name: &str,
    ) -> bool {
        if implicit_name != "next" {
            return false;
        }
        if !next_decl.return_type.as_ref().is_some_and(type_has_proj) {
            return false;
        }
        let Some(callee) = callee else { return false };
        callee.implicit_groups.iter().any(|spread| {
            spread.name.name == "Yield"
                && matches!(
                    spread.args.get(1),
                    Some(Type::Named { qualifiers, base })
                        if qualifiers.is_empty()
                            && base.args.is_empty()
                            && matches!(
                                base.name.name.as_str(),
                                "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                            )
                )
        })
    }

    /// [rs-proj-arm] The constructor fns (`-> T as Q`) of a constructive
    /// qualifier, program-wide.
    fn constructors_of(&self, qual: &str) -> Vec<String> {
        self.program
            .modules
            .iter()
            .flat_map(|m| m.items.iter())
            .filter_map(|item| match item {
                Item::Fn(f) if f.constructs.as_ref().is_some_and(|c| c.name.name == qual) => {
                    Some(f.name.name.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// [rs-proj-arm] The resolved Rust name of a call's callee.
    fn emit_expr_callee_name(&mut self, call: &Expr) -> String {
        let Expr::Call { callee, span, .. } = call else {
            return String::new();
        };
        if let Some(decl) = self
            .checked
            .call_fn
            .get(&(self.file_idx, *span))
            .and_then(|key| self.fn_by_key(*key))
        {
            return self.rust_fn_name(decl);
        }
        match callee.as_ref() {
            Expr::Ident(id) => rs_ident(&id.name),
            other => self.emit_expr(other),
        }
    }

    /// [rs-proj-arm] An argument lent into a `proj` arm, as a `&T`: an
    /// optional-borrow local is unwrapped to its reference, a reference
    /// binding passes as is, an owned place is borrowed.
    fn borrow_arg_for_arm(&mut self, arg: &Expr) -> String {
        if let Expr::Ident(id) = arg {
            match self.bindings.get(id.name.as_str()) {
                Some(BindKind::OptRef) => {
                    return format!("{}.unwrap()", self.binding_place(&id.name));
                }
                Some(BindKind::Ref) => return self.binding_place(&id.name),
                Some(BindKind::RefMut) => return format!("&*{}", self.binding_place(&id.name)),
                _ => {}
            }
        }
        match self.borrow_place(arg) {
            Some(b) => b,
            None => {
                self.error(format!(
                    "a value lent into a `proj` arm must be a borrow or a place (got `{arg:?}`)"
                ));
                self.emit_expr(arg)
            }
        }
    }

    /// A fresh local for a value that must be computed before exit
    /// blocks run [rs-exit-splice].
    fn fresh_splice_var(&mut self) -> String {
        self.splice_id += 1;
        format!("__exit_value{}", self.splice_id)
    }

    // ================= throw and `try` [rs-throw-controlflow] =================

    /// Wraps a returned value in `ControlFlow::Continue` when the current
    /// fn may throw [rs-throw-controlflow]; a pass-through otherwise.
    fn wrap_continue(&mut self, code: String) -> String {
        match self.throw_message {
            Some(_) => {
                self.imports
                    .insert("use std::ops::ControlFlow;".to_string());
                format!("ControlFlow::Continue({code})")
            }
            None => code,
        }
    }

    /// A fresh labelled-block label for a `try` [rs-try-label].
    fn fresh_try_label(&mut self) -> String {
        self.try_id += 1;
        format!("'try_{}", self.try_id)
    }

    /// The message value at a throw site, wrapped into the target's arm
    /// when several message types meet there [union-arm-identity].
    fn throw_message_value(&mut self, site: &ThrowSite, code: String) -> String {
        match site.arm {
            Some(arm) => self.wrap_union_value(&site.target, arm, code),
            None => code,
        }
    }

    /// The Rust code that *takes* the throw at a site: breaking the
    /// enclosing `try`'s labelled block with the thrown arm of its
    /// outcome, or returning `ControlFlow::Break` out of the fn
    /// [rs-throw-controlflow]. Splices pending inside the
    /// construct being left run first [rs-exit-splice].
    fn throw_transfer(&mut self, site: &ThrowSite, message: String, indent: usize) -> String {
        match self.try_frames.last() {
            Some(frame) => {
                let label = frame.label.clone();
                let floor = frame.splice_floor;
                let outcome = frame.outcome.clone();
                let payload = self.throw_message_value(site, message);
                // The thrown arm is arm 1 of `Ok T | Thrown M`.
                let wrapped = match &outcome {
                    Some(ty) => self.wrap_union_value(ty, 1, payload),
                    None => payload,
                };
                let splices = self.splice_exits(floor, indent, false);
                if splices.is_empty() {
                    format!("break {label} {wrapped}")
                } else {
                    let pad = "    ".repeat(indent);
                    format!("{{\n{splices}{pad}break {label} {wrapped};\n{pad}}}")
                }
            }
            None => {
                self.imports
                    .insert("use std::ops::ControlFlow;".to_string());
                let payload = self.throw_message_value(site, message);
                let splices = self.exit_splice_code(indent, false);
                if splices.is_empty() {
                    format!("return ControlFlow::Break({payload})")
                } else {
                    let pad = "    ".repeat(indent);
                    format!("{{\n{splices}{pad}return ControlFlow::Break({payload});\n{pad}}}")
                }
            }
        }
    }

    /// `throw(message)` [throw]: the control transfer itself, in expression
    /// position (`break`/`return` are expressions in Rust, so a
    /// `Never`-typed operand needs no special casing).
    fn emit_throw_call(&mut self, site: &ThrowSite, args: &[Expr], indent: usize) -> String {
        let message = match args.first() {
            Some(a) => self.emit_expr(a),
            None => {
                self.error("`throw` needs a message argument");
                "()".to_string()
            }
        };
        self.throw_transfer(site, message, indent)
    }

    /// A call to a fn that may throw [rs-throw-controlflow]: its
    /// `ControlFlow` result is unwrapped here. `?` does it in one character
    /// — but only when the throw would leave *this* fn unchanged: inside a
    /// `try`, when the message needs wrapping into a union, or when
    /// exit splices must run first, the propagation is written out as a
    /// `match` (an expression, so no hoisting is needed).
    fn wrap_may_throw_call(&mut self, site: &ThrowSite, call: String, indent: usize) -> String {
        self.imports
            .insert("use std::ops::ControlFlow;".to_string());
        let inside_try = !self.try_frames.is_empty();
        let pending_splices = match self.try_frames.last() {
            Some(frame) => self.exit_splices.len() > frame.splice_floor,
            None => self.exit_splices.len() > self.splice_floor,
        };
        if !inside_try && !pending_splices && site.arm.is_none() {
            return format!("{call}?");
        }
        let transfer = self.throw_transfer(site, "__m".to_string(), indent);
        format!(
            "match {call} {{ ControlFlow::Continue(__v) => __v, \
             ControlFlow::Break(__m) => {transfer} }}"
        )
    }

    /// `try { ... }` [rs-try-label]: a labelled block. No closure, so
    /// nothing is captured — the body reads the fn's effect parameters and
    /// locals directly — and a throw inside it `break`s the label with the
    /// thrown arm of the outcome.
    fn emit_try(&mut self, body: &Block, span: Span, indent: usize) -> String {
        let outcome = self.ty_of(span).cloned();
        if let Some(ty) = &outcome {
            let arms = ty.value_arms().len();
            if arms >= 2 {
                self.union_sizes.insert(arms);
            }
        }
        let label = self.fresh_try_label();
        self.try_frames.push(TryFrame {
            label: label.clone(),
            splice_floor: self.exit_splices.len(),
            outcome: outcome.clone(),
        });
        let inner = self.emit_try_body(body, outcome.as_ref(), indent + 1);
        self.try_frames.pop();
        let pad = "    ".repeat(indent);
        format!("{label}: {{\n{inner}{pad}}}")
    }

    /// The body of a `try`: ordinary statements, with the tail wrapped into
    /// the outcome's `Ok` arm (arm 0). A body that always leaves — every
    /// path throws or returns — still needs a value for the block, which
    /// the checker made `Ok None` [try].
    fn emit_try_body(&mut self, body: &Block, outcome: Option<&Ty>, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        let saved_env = self.effect_env.clone();
        let splice_floor = self.exit_splices.len();
        let mut tail_code: Option<String> = None;
        let n = body.stmts.len();
        for (i, stmt) in body.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    if !matches!(self.ty_of(e.span()), Some(Ty::Never)) {
                        tail_code = Some(self.emit_expr(e));
                        continue;
                    }
                }
            }
            out.push_str(&self.emit_stmt(stmt, indent, StmtCtx::Normal));
        }
        let value = tail_code.unwrap_or_else(|| "()".to_string());
        let wrapped = match outcome {
            Some(ty) => self.wrap_union_value(ty, 0, value),
            None => value,
        };
        // Splices in the body run before the block's value is
        // produced on the normal path [rs-exit-splice].
        let tmp = if self.exit_splices.len() > splice_floor {
            let tmp = self.fresh_splice_var();
            out.push_str(&format!("{pad}let {tmp} = {wrapped};\n"));
            out.push_str(&self.splice_exits(splice_floor, indent, true));
            tmp
        } else {
            wrapped
        };
        out.push_str(&format!("{pad}{tmp}\n"));
        self.effect_env = saved_env;
        out
    }

    /// [rs-opt-borrow] Whether `value` is a derived-return call
    /// [readonly-return] whose declared result is optional — the shape that
    /// produces an `Option<&T>` in this backend.
    fn is_optional_derived_call(&self, value: &Expr) -> bool {
        let Expr::Call { span, .. } = value else {
            return false;
        };
        if !self
            .checked
            .derived_calls
            .contains_key(&(self.file_idx, *span))
        {
            return false;
        }
        self.checked
            .ty_of(self.file_idx, *span)
            .is_some_and(|t| t.strip_quals().has_none_arm())
    }

    fn emit_let(
        &mut self,
        pattern: &Pattern,
        ty: Option<&Type>,
        value: &Expr,
        stmt_span: Span,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // [iter-fn] Inside an iterator fn the body's locals are fields
        // of the pass, so a `let` *assigns* one: the declaration happened in
        // the struct.
        if let Pattern::Ident(name) = pattern {
            if self.gen_fields.contains(name.name.as_str()) {
                let value_code = self.emit_bound_value(value, stmt_span);
                let field = rs_ident(&name.name);
                if self.gen_slots.contains(name.name.as_str()) {
                    return format!("{pad}self.{field} = Some({value_code});\n");
                }
                return format!("{pad}self.{field} = {value_code};\n");
            }
        }
        // [rs-borrow-locals] S3: a borrow-mode binding from a pure place
        // emits a real borrow — the local holds `&T` and reads thread
        // through the existing reference-binding rendering (clone in
        // owned positions, bare in borrow positions). Requires: an ident
        // pattern, not a move-mode bind event, the name never reassigned
        // in this fn (one Rust type per local), and a pure place value;
        // everything else keeps the fate-link clone. Decided before the
        // value is emitted so no spurious coercions are recorded.
        if let Pattern::Ident(name) = pattern {
            if !self
                .checked
                .binding_modes
                .contains(&(self.file_idx, stmt_span))
                && !self.mutated.contains(name.name.as_str())
            {
                if let Some(borrow) = self.borrow_value(value) {
                    self.bindings.insert(name.name.clone(), BindKind::Ref);
                    let annot = match ty {
                        Some(t) if !is_fn_type(t) => format!(": &{}", self.emit_type(t)),
                        _ => String::new(),
                    };
                    return format!("{pad}let mut {}{annot} = {borrow};\n", rs_ident(&name.name));
                }
            }
        }
        // [rs-opt-borrow] A local bound from a derived-return call whose
        // result is optional holds `Option<&T>`. Recorded so a later
        // narrowing unwraps the reference rather than cloning it.
        if let Pattern::Ident(name) = pattern {
            if !self.mutated.contains(name.name.as_str()) && self.is_optional_derived_call(value) {
                self.bindings.insert(name.name.clone(), BindKind::OptRef);
            }
            // [rs-proj-arm] `let step = next(p)`: the union's `proj` arm holds
            // a reference; later payload reads see it.
            let borrowed_arms = self.call_borrowed_arms(value);
            if borrowed_arms.is_empty() {
                self.borrowed_arm_locals.remove(&name.name);
            } else {
                self.borrowed_arm_locals
                    .insert(name.name.clone(), borrowed_arms);
            }
            // `let v = get(xs, i)!`: the `!` yields the `&T` itself, so the
            // local is a plain reference binding — no clone at the binding,
            // reads clone in owned positions as any `Ref` does [rs-borrows].
            if !self.mutated.contains(name.name.as_str()) {
                if let Expr::NonNull { operand, .. } = value {
                    if self.is_optional_derived_call(operand)
                        && !self
                            .ty_of(value.span())
                            .is_some_and(|t| Self::is_copy_ty(t))
                    {
                        let inner = self.emit_owned(operand);
                        self.bindings.insert(name.name.clone(), BindKind::Ref);
                        return format!(
                            "{pad}let mut {} = {inner}.unwrap();\n",
                            rs_ident(&name.name)
                        );
                    }
                }
            }
        }
        // Bare struct literals pick up the annotated type.
        let value_code = match (value, ty) {
            (
                Expr::StructLit {
                    ty: None,
                    fields,
                    span,
                },
                Some(annot),
            ) => {
                let code = self.emit_struct_lit(Some(annot), fields, *span);
                self.apply_coercion(*span, code)
            }
            _ => self.emit_bound_value(value, stmt_span),
        };
        match pattern {
            Pattern::Ident(name) => {
                // [rs-opt-borrow] Keep an optional-borrow classification
                // decided above; everything else is an owned local.
                if !matches!(
                    self.bindings.get(name.name.as_str()),
                    Some(BindKind::OptRef)
                ) {
                    self.bindings.insert(name.name.clone(), BindKind::Owned);
                }
                // A fn-type annotation cannot be spelled on a Rust
                // binding (`impl Trait` is invalid there [fn-contract]);
                // the closure's inferred type is already exact, so the
                // annotation is dropped.
                let annot = match ty {
                    Some(t) if !is_fn_type(t) => format!(": {}", self.emit_type(t)),
                    _ => String::new(),
                };
                // Every local is `let mut` [rs-borrows] (Salvo mutability
                // is not locally decidable; `unused_mut` is allowed).
                format!(
                    "{pad}let mut {}{annot} = {value_code};\n",
                    rs_ident(&name.name)
                )
            }
            Pattern::Tuple { elems, .. } => {
                let names: Vec<String> = elems
                    .iter()
                    .map(|p| match p {
                        Pattern::Ident(id) => {
                            self.bindings.insert(id.name.clone(), BindKind::Owned);
                            format!("mut {}", rs_ident(&id.name))
                        }
                        _ => {
                            self.error("nested destructuring patterns are not supported yet");
                            "_".to_string()
                        }
                    })
                    .collect();
                format!("{pad}let ({}) = {value_code};\n", names.join(", "))
            }
            Pattern::Struct { fields, .. } => {
                // [let-destructure] unique per fn; fields move out of the
                // owned temp (partial moves are fine — the temp is dead).
                self.destructure_id += 1;
                let temp = format!("__destructured{}", self.destructure_id);
                let mut out = format!("{pad}let {temp} = {value_code};\n");
                for f in fields {
                    self.bindings
                        .insert(f.binding.name.clone(), BindKind::Owned);
                    out.push_str(&format!(
                        "{pad}let mut {} = {temp}.{};\n",
                        rs_ident(&f.binding.name),
                        rs_ident(&f.field.name)
                    ));
                }
                out
            }
        }
    }

    /// `use Handler(...)` [effect-use] [rs-effects]: instantiate the
    /// handler into a `let mut` local and register it for the rest of the
    /// scope; member calls go through the local, threading takes
    /// `&mut local`.
    /// [actor-spawn-expr] [rs-actor] `spawn H(args) capacity N on P` →
    /// `salvo_spawn(pool, bound, Box::new(__Actor_H::new(H::new(args))))`,
    /// whose value is the addr. Construction is the `use` path's, minus the
    /// registration: a spawn does not put the handler in *this* scope.
    /// [rs-mailbox] [actor-mailbox] The `capacity` expression of a handler's
    /// `mailbox` slot, rendered as an `i32`. The checker has already required
    /// the slot on a handler of an `actor effect` and confined its expressions
    /// to constructor parameters, so this is an ordinary emission in the
    /// constructor's scope; a missing slot is an internal inconsistency rather
    /// than a language cut.
    fn mailbox_capacity_code(&mut self, h: &HandlerDecl) -> String {
        match h.mailbox.as_ref().and_then(mailbox_capacity_expr) {
            Some(expr) => self.emit_owned(expr),
            None => {
                self.error(format!(
                    "internal: actor handler `{}` has no `mailbox {{ capacity: … }}`",
                    h.name.name
                ));
                "0".to_string()
            }
        }
    }

    fn emit_spawn(
        &mut self,
        handler: &Expr,
        uses: &[Expr],
        pool: Option<&Expr>,
        span: Span,
    ) -> String {
        let (handler_name, args): (String, Vec<&Expr>) = match handler {
            Expr::Ident(id) => (id.name.clone(), Vec::new()),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => (id.name.clone(), args.iter().collect()),
                _ => {
                    self.error("`spawn` expects a handler name or constructor call");
                    return "todo!()".to_string();
                }
            },
            _ => {
                self.error("`spawn` expects a handler name or constructor call");
                return "todo!()".to_string();
            }
        };
        let Some(decl) = self.symbols.handlers.get(handler_name.as_str()).copied() else {
            self.error(format!("unknown handler `{handler_name}` in `spawn`"));
            return "todo!()".to_string();
        };
        if !decl.generics.is_empty() {
            self.error(format!(
                "spawning generic handler `{handler_name}` is not supported yet"
            ));
            return "todo!()".to_string();
        }
        // [monitor-handler] [rs-monitor] Every face a plain effect: the
        // **monitor spawn** — no mailbox, no scheduler, no pool. One shared
        // instance behind a lock, handed out as the effect's `__Mon_E`
        // wrapper. The checker restricted the handler (single face, no
        // dependencies, sendable state) and refused the `on` clause.
        let plain_faces = !decl.of.is_empty()
            && decl.of.iter().all(|of| {
                type_base_name(of).is_some_and(|n| {
                    self.symbols
                        .effects
                        .get(n)
                        .is_some_and(|e| !e.is_actor)
                })
            });
        if plain_faces {
            if decl.of.len() > 1 {
                self.error(format!(
                    "handler `{handler_name}` implements several plain effects, and a \
                     shared instance behind several faces is not supported yet"
                ));
                return "todo!()".to_string();
            }
            let effect = decl.of.first().and_then(type_base_name).unwrap_or_default();
            let mon = monitor_struct_name(effect);
            // [rs-monitor] A generic effect instance names the wrapper's
            // type arguments outright (the checker resolved the instance).
            let mon_args = match self
                .checked
                .spawn_effects
                .get(&(self.file_idx, span))
                .and_then(|tys| tys.first())
            {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    self.effect_instance_turbofish(&ty)
                }
                _ => String::new(),
            };
            let ctor = self.handler_ctor_path(&handler_name, decl);
            // [mixed-handler] [rs-mixed] The mixed spawn: build the handler,
            // read its mailbox bound, spawn the servant, and answer the
            // handle over the façade. Constructor arguments are evaluated
            // **once**, cloned into the handler and moved into the façade —
            // the checker proved them sendable and refused `Mut`, which is
            // what makes the copy legal and unobservable.
            if decl.fns.iter().any(|f| f.is_send) {
                self.needs_scheduler = true;
                let mut lets = String::new();
                let mut handler_args: Vec<String> = Vec::new();
                let mut fac_fields: Vec<String> = vec!["__addr: __a".to_string()];
                for (i, (a, p)) in args.iter().zip(&decl.params).enumerate() {
                    let code = self.emit_owned(a);
                    lets.push_str(&format!("let __c{i} = {code}; "));
                    handler_args.push(format!("__c{i}.clone()"));
                    fac_fields.push(format!("{}: __c{i}", rs_ident(&p.name.name)));
                }
                let pool_code = match pool {
                    Some(pool) => self.emit_owned(pool),
                    None => "crate::scheduler::salvo_current_pool()".to_string(),
                };
                return format!(
                    "({{ {lets}let __h = {ctor}::new({}); \
                     let __cap = __h.__mailbox_capacity; \
                     let __a = crate::scheduler::salvo_spawn({pool_code}, __cap as usize, \
                     Box::new({}::new(__h))); \
                     {mon}{mon_args}::new(Box::new({} {{ {} }})) }})",
                    handler_args.join(", "),
                    actor_struct_name(&handler_name),
                    facade_struct_name(&handler_name),
                    fac_fields.join(", ")
                );
            }
            // [monitor-handler] [rs-monitor] The monitor spawn: the handler
            // behind the per-effect lock adapter, boxed into the handle.
            // [use-local] Its captured dependency handles follow the written
            // arguments, cloned off the eager handle variables their
            // bindings minted.
            let mut arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            let saved_producer = self.emitting_producer_args;
            self.emitting_producer_args = true;
            arg_code.extend(self.emit_implicit_args(&[], span));
            self.emitting_producer_args = saved_producer;
            for dep in self
                .checked
                .handle_captures
                .get(&(self.file_idx, span))
                .cloned()
                .unwrap_or_default()
            {
                match self.effect_entry_by_ty(&dep) {
                    Some(entry) => match &entry.handle_var {
                        Some(hv) => arg_code.push(format!("{hv}.clone()")),
                        None => {
                            self.error(format!(
                                "internal: the binding for captured dependency `{dep}` \
                                 has no handle variable"
                            ));
                            arg_code.push("todo!()".to_string());
                        }
                    },
                    None => {
                        self.error(format!(
                            "internal: no binding in scope for captured dependency `{dep}`"
                        ));
                        arg_code.push("todo!()".to_string());
                    }
                }
            }
            return format!(
                "{mon}{mon_args}::new(Box::new({}::new({ctor}::new({}))))",
                lock_struct_name(effect),
                arg_code.join(", ")
            );
        }
        self.needs_scheduler = true;
        // [effect-handler-deps] [rs-effect-fusion] A dependent child owns its
        // dependencies rather than reaching a fusion: the clause's instances
        // go into the generated flat provider, in the *handler's declaration*
        // order, which is what the checker's `spawn_dep_items` records.
        let deps = self.handler_dep_effects(decl);
        let prov = if deps.is_empty() {
            None
        } else {
            match self.spawn_provider(&handler_name, &deps, uses, span) {
                Some(code) => Some(code),
                None => return "todo!()".to_string(),
            }
        };
        let mut arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        let saved_producer = self.emitting_producer_args;
        self.emitting_producer_args = true;
        arg_code.extend(self.emit_implicit_args(&[], span));
        self.emitting_producer_args = saved_producer;
        let ctor = self.handler_ctor_path(&handler_name, decl);
        // The actor body is emitted beside its handler, and generated
        // `use crate::<module>::*` globs bring both into scope unqualified —
        // the same reason `handler_ctor_path` answers a bare name.
        let held = format!("{}::new({})", ctor, arg_code.join(", "));
        // [actor-mailbox] The bound is the *handler's*, computed by its
        // constructor from its own parameters — so the instance is built into a
        // local, the bound read off it, and only then does it move into the
        // actor body. That ordering is the whole reason this is a block.
        let body = match prov {
            Some(prov) => format!("{}::new(__h, {prov})", actor_struct_name(&handler_name)),
            None => format!("{}::new(__h)", actor_struct_name(&handler_name)),
        };
        // [main-pool] An omitted `on` clause means the pool current where the
        // spawn runs — `main`'s own pool in `main`, the actor's in a member.
        let pool_code = match pool {
            Some(pool) => self.emit_owned(pool),
            None => "crate::scheduler::salvo_current_pool()".to_string(),
        };
        let spawn_call =
            format!("crate::scheduler::salvo_spawn({pool_code}, __cap as usize, Box::new({body}))");
        // [effect-handler-multi] One addr per implemented effect. There is one
        // actor, one mailbox and one scheduler index; the tuple hands the same
        // index out under each protocol's type, which is where "least authority
        // falls out of the types" is paid for — nothing at run time.
        if decl.of.len() > 1 {
            let faces: Vec<String> = (0..decl.of.len()).map(|_| "__a".to_string()).collect();
            return format!(
                "({{ let __h = {held}; let __cap = __h.__mailbox_capacity; \
                 let __a = {spawn_call}; ({}) }})",
                faces.join(", ")
            );
        }
        format!("({{ let __h = {held}; let __cap = __h.__mailbox_capacity; {spawn_call} }})")
    }

    /// [actor-spawn-expr] [rs-actor] The child's flat provider, built from
    /// the spawn's `with` clause: `__Prov_H { __d0: <instance>, … }`, its
    /// fields in the handler's *declaration* order while the clause items are
    /// in the order the program wrote them. The checker's `spawn_dep_items`
    /// is the map between the two.
    fn spawn_provider(
        &mut self,
        handler_name: &str,
        deps: &[(String, Vec<String>)],
        uses: &[Expr],
        span: Span,
    ) -> Option<String> {
        let items = match self.checked.spawn_dep_items.get(&(self.file_idx, span)) {
            Some(items) if items.len() == deps.len() => items.clone(),
            _ => {
                self.error(format!(
                    "internal: spawning `{handler_name}` has no resolved dependency \
                     items for its {} declared dependencies",
                    deps.len()
                ));
                return None;
            }
        };
        // [spawn-inherit] The instances the child's provider holds: a
        // written clause item is constructed here, and a **synthesized**
        // dependency (`None`) is the spawning scope's own handle, cloned —
        // the child then holds the same shared instance the parent does.
        let resolved = self
            .checked
            .spawn_deps
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        let mut fields: Vec<String> = Vec::new();
        for (i, at) in items.iter().enumerate() {
            let instance = match at {
                Some(at) => {
                    let Some(item) = uses.get(*at) else {
                        self.error(format!(
                            "internal: spawning `{handler_name}` names clause item \
                             {at}, which the form does not have"
                        ));
                        return None;
                    };
                    self.spawn_dep_instance(item, &deps[i])?
                }
                None => {
                    let Some(ty) = resolved.get(i).cloned() else {
                        self.error(format!(
                            "internal: spawning `{handler_name}` inherits dependency \
                             {i} with no resolved instance"
                        ));
                        return None;
                    };
                    match self.inherited_dep_handle(&ty) {
                        Some(code) => code,
                        None => return None,
                    }
                }
            };
            fields.push(format!("__d{i}: {instance}"));
        }
        Some(format!(
            "{} {{ {} }}",
            prov_struct_name(handler_name),
            fields.join(", ")
        ))
    }

    /// [spawn-inherit] A **synthesized** dependency's instance: the handle
    /// this frame holds for that effect — an eager handle variable a binding
    /// minted, or a field of this fn's own handle bundle
    /// ([rs-handle-bundle]) — cloned into the child, so parent and child
    /// share one instance.
    fn inherited_dep_handle(&mut self, ty: &Ty) -> Option<String> {
        if let Some(hv) = self
            .effect_entry_by_ty(ty)
            .and_then(|e| e.handle_var.clone())
        {
            return Some(format!("{hv}.clone()"));
        }
        if let Some(place) = self.handle_place(ty) {
            return Some(format!("{place}.clone()"));
        }
        self.error(format!(
            "internal: no handle in scope for inherited dependency `{ty}`"
        ));
        None
    }

    /// [actor-spawn-expr] One clause item as the expression that *makes* an
    /// instance: a handler construction is `D::new(args)`, an `Addr` is the
    /// forwarding stub over it (`__Stub_D::new(addr)`) — the same two shapes
    /// `use` binds, which is what makes a dependency swappable between a
    /// local handler and an actor without touching the child.
    fn spawn_dep_instance(&mut self, item: &Expr, dep: &(String, Vec<String>)) -> Option<String> {
        let named = match item {
            Expr::Ident(id) => Some((id.name.clone(), Vec::new())),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => Some((id.name.clone(), args.iter().collect())),
                _ => None,
            },
            _ => None,
        };
        if let Some((name, args)) = named {
            if let Some(decl) = self.symbols.handlers.get(name.as_str()).copied() {
                let arg_code: Vec<String> = args.iter().map(|a| self.emit_owned(a)).collect();
                let ctor = self.handler_ctor_path(&name, decl);
                return Some(format!("{ctor}::new({})", arg_code.join(", ")));
            }
        }
        // Not a handler name, so it is an addr: the stub is the instance.
        // [monitor-handler] [rs-monitor] A plain effect's handle *is* an
        // instance already — the effect's lock wrapper — so it passes through
        // as itself (`emit_owned` clones the handle), and the same monitor
        // serves every actor it is supplied to. An actor effect's addr is an
        // index the send stub wraps.
        let addr_code = self.emit_owned(item);
        if self
            .symbols
            .effects
            .get(dep.0.as_str())
            .is_some_and(|e| !e.is_actor)
        {
            return Some(addr_code);
        }
        Some(format!("{}::new({addr_code})", stub_struct_name(&dep.0)))
    }

    /// [actor-waitfor] `waitfor out: Reply<T> { … }` → a block expression:
    /// mint the waiter, run the block (which sends the token somewhere), then
    /// block this thread for the answer and downcast it.
    fn emit_waitfor(
        &mut self,
        binding: &Ident,
        ty: &Option<Type>,
        body: &Block,
        _span: Span,
    ) -> String {
        self.needs_scheduler = true;
        let indent = self.expr_indent;
        let pad = "    ".repeat(indent + 1);
        let close = "    ".repeat(indent);
        // [waitfor-infer] The payload type: off the written `Reply<T>` when
        // there is one, else the type the checker recorded for the whole
        // expression — which *is* the payload, since that is what a `waitfor`
        // yields [actor-waitfor].
        let payload = match ty {
            Some(Type::Named { base, .. })
                if base.name.name == "Reply" && base.args.len() == 1 =>
            {
                self.emit_type(&base.args[0])
            }
            None => match self.ty_of(_span).cloned() {
                Some(t) => self.rust_ty(&t),
                None => {
                    self.error("`waitfor` binds a `Reply<T>`");
                    return "todo!()".to_string();
                }
            },
            _ => {
                self.error("`waitfor` binds a `Reply<T>`");
                return "todo!()".to_string();
            }
        };
        let name = rs_ident(&binding.name);
        self.bindings.insert(binding.name.clone(), BindKind::Owned);
        let mut out = String::from("{\n");
        out.push_str(&format!(
            "{pad}let (mut {name}, __wid) = crate::scheduler::salvo_waiter();\n"
        ));
        let saved = std::mem::replace(&mut self.expr_indent, indent + 1);
        for stmt in &body.stmts {
            out.push_str(&self.emit_stmt(stmt, indent + 1, StmtCtx::Normal));
        }
        self.expr_indent = saved;
        out.push_str(&format!(
            "{pad}*crate::scheduler::salvo_wait(__wid).downcast::<{payload}>()\
             .expect(\"the awaited answer\")\n{close}}}"
        ));
        out
    }

    /// [actor-use-addr] [rs-actor] `addr.member(args)` →
    /// `salvo_send(addr, Box::new(__Msg_E::Member(args)))`. The message owns
    /// its payload: it outlives the send, so every argument is emitted owned.
    fn emit_addr_send(
        &mut self,
        effect: &salvo_core::Ty,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> String {
        self.needs_scheduler = true;
        let Expr::Field { base, field, .. } = callee else {
            self.error("internal: an addr send whose callee is not a dot-call");
            return "todo!()".to_string();
        };
        let effect_name = match effect {
            salvo_core::Ty::Named { name, .. } => name.clone(),
            _ => {
                self.error("internal: an addr send with no effect recorded");
                return "todo!()".to_string();
            }
        };
        let member = self.called_member_name(&effect_name, &field.name, span);
        let msg = self.effect_path(&effect_name, &msg_enum_name(&effect_name));
        let variant = msg_variant_name(&member);
        let payload: Vec<String> = args.iter().map(|a| self.emit_owned(a)).collect();
        let built = if payload.is_empty() {
            format!("{msg}::{variant}")
        } else {
            format!("{msg}::{variant}({})", payload.join(", "))
        };
        let target = self.emit_expr(base);
        format!("crate::scheduler::salvo_send({target}, Box::new({built}))")
    }

    /// [mixed-handler] [rs-mixed] A façade send: enqueue on the servant's
    /// mailbox through the façade's own addr. The member is one of the
    /// enclosing mixed handler's `send fn` members, so the message enum is
    /// the *handler's* (`__Msg_H`), not an effect's.
    fn emit_facade_send(&mut self, member: &str, args: &[Expr]) -> String {
        self.needs_scheduler = true;
        let Some(handler) = self.current_handler.clone() else {
            self.error("internal: a façade send outside a handler member");
            return "todo!()".to_string();
        };
        let msg = msg_enum_name(&handler);
        let variant = msg_variant_name(member);
        let payload: Vec<String> = args.iter().map(|a| self.emit_owned(a)).collect();
        let built = if payload.is_empty() {
            format!("{msg}::{variant}")
        } else {
            format!("{msg}::{variant}({})", payload.join(", "))
        };
        format!("crate::scheduler::salvo_send(self.__addr, Box::new({built}))")
    }

    /// [mixed-handler] [rs-mixed] Whether a handler is mixed: every face a
    /// plain effect, and at least one `send fn` member — the checker's
    /// classification, mirrored.
    fn handler_is_mixed(&self, h: &HandlerDecl) -> bool {
        h.fns.iter().any(|f| f.is_send)
            && !h.of.is_empty()
            && h.of.iter().all(|of| {
                type_base_name(of).is_some_and(|n| {
                    self.symbols.effects.get(n).is_some_and(|e| !e.is_actor)
                })
            })
    }

    /// [mixed-handler] [rs-mixed] The façade: the servant's addr plus the
    /// constructor parameters, implementing each plain face with the sync
    /// member bodies. Its fields are what the checker proved sendable, and
    /// `Clone` is what makes the handle it sits inside copyable.
    fn emit_facade(&mut self, h: &HandlerDecl) -> String {
        let fac = facade_struct_name(&h.name.name);
        let mut out = format!("\n#[derive(Clone)]\npub struct {fac} {{\n    __addr: usize,\n");
        for p in &h.params {
            let ty = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&p.name.name)));
        }
        out.push_str("}\n");
        for (face, effect) in h.of.iter().zip(self.handler_faces(h)) {
            let of = self.emit_type(face);
            out.push_str(&format!("\nimpl {of} for {fac} {{\n"));
            for f in &h.fns {
                if f.is_send {
                    continue;
                }
                if !self
                    .member_faces(h, f)
                    .iter()
                    .any(|(e, _)| std::ptr::eq(*e, effect))
                {
                    continue;
                }
                out.push_str(&self.emit_fn_inner(f, FnStyle::HandlerMember(h), 1));
            }
            out.push_str("}\n");
        }
        out
    }

    /// [mixed-handler] [rs-mixed] The servant's runtime parts: the
    /// handler-keyed message enum (`__Msg_H`, one variant per `send fn`
    /// member) and the actor body dispatching it — resume included, since a
    /// servant parks continuations of its own [defer-deduction]. The
    /// face-keyed twin lives in `emit_actor_body`; this one is simpler
    /// because a mixed servant has (for now) no dependencies and one
    /// protocol.
    fn emit_mixed_actor_parts(&mut self, h: &HandlerDecl) -> String {
        self.needs_scheduler = true;
        let name = rs_ident(&h.name.name);
        let msg = msg_enum_name(&h.name.name);
        let actor = actor_struct_name(&h.name.name);
        let sends: Vec<&FnDecl> = h.fns.iter().filter(|f| f.is_send).collect();
        let mut out = format!("\npub enum {msg} {{\n");
        for f in &sends {
            let payload: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                // A message *owns* its payload: it outlives the send.
                .map(|p| self.param_type(&p.ty, p.variadic, ParamMode::Owned))
                .collect();
            let variant = msg_variant_name(&f.name.name);
            if payload.is_empty() {
                out.push_str(&format!("    {variant},\n"));
            } else {
                out.push_str(&format!("    {variant}({}),\n", payload.join(", ")));
            }
        }
        out.push_str("}\n");
        out.push_str(&format!(
            "\npub struct {actor} {{\n    handler: {name},\n}}\n\n\
             impl {actor} {{\n    pub fn new(handler: {name}) -> Self {{\n        \
             Self {{ handler }}\n    }}\n}}\n\n\
             impl crate::scheduler::SalvoActor for {actor} {{\n    \
             fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {{\n        \
             self.handler.__addr = Some(_ctx.addr);\n        \
             let msg = *msg.downcast::<{msg}>().expect(\"a message of this servant's protocol\");\n        \
             match msg {{\n"
        ));
        for f in &sends {
            let variant = msg_variant_name(&f.name.name);
            let params: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| rs_ident(&p.name.name))
                .collect();
            let bind = if params.is_empty() {
                String::new()
            } else {
                format!("({})", params.join(", "))
            };
            out.push_str(&format!(
                "            {msg}::{variant}{bind} => self.handler.{}({}),\n",
                rs_ident(&f.name.name),
                params.join(", ")
            ));
        }
        out.push_str("        }\n    }\n");
        // [actor-replyto] [defer-deduction] The other half of the servant's
        // parked table, mirroring the face-keyed resume: look the slot up,
        // and the variant says which send member to run and therefore what
        // the answer downcasts to — its *trailing* parameter's type.
        match self.handler_cont_type(h) {
            None => out.push_str(
                "\n    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, \
                 _slot: u64, _value: crate::scheduler::SalvoMsg) {\n        \
                 unreachable!(\"this servant's protocol has no continuation targets\")\n    }\n",
            ),
            Some(cont) => {
                out.push_str(&format!(
                    "\n    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, \
                     slot: u64, value: crate::scheduler::SalvoMsg) {{\n        \
                     self.handler.__addr = Some(_ctx.addr);\n        \
                     let Some(__cont) = self.handler.__parked.remove(&slot) else {{\n            \
                     return; // a reply whose continuation is gone: nothing to run\n        \
                     }};\n        match __cont {{\n"
                ));
                for f in &sends {
                    let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                    if fixed.is_empty() {
                        continue;
                    }
                    let variant = msg_variant_name(&f.name.name);
                    let caps: Vec<String> = fixed[..fixed.len() - 1]
                        .iter()
                        .map(|p| rs_ident(&p.name.name))
                        .collect();
                    let bind = if caps.is_empty() {
                        String::new()
                    } else {
                        format!("({})", caps.join(", "))
                    };
                    let answer_ty = {
                        let last = fixed[fixed.len() - 1];
                        self.param_type(&last.ty, last.variadic, ParamMode::Owned)
                    };
                    let mut args: Vec<String> = caps.clone();
                    args.push(format!(
                        "*value.downcast::<{answer_ty}>().expect(\"the awaited answer\")"
                    ));
                    out.push_str(&format!(
                        "            {cont}::{variant}{bind} => self.handler.{}({}),\n",
                        rs_ident(&f.name.name),
                        args.join(", ")
                    ));
                }
                out.push_str("        }\n    }\n");
            }
        }
        out.push_str("}\n");
        out
    }

    fn emit_use(
        &mut self,
        handler: &Expr,
        local: bool,
        with_items: &[Expr],
        span: Span,
        indent: usize,
    ) -> String {
        let _ = local; // classification travels in `use_kinds` [use-local]
        let pad = "    ".repeat(indent);
        // [actor-use-addr] `use addr` binds the effect to a **forwarding stub**
        // over the addr — `__Stub_E::new(addr)` is an ordinary instance as far
        // as the rest of the scope is concerned, which is the whole point: no
        // call site learns the difference.
        if let Some(effect) = self.checked.use_addrs.get(&(self.file_idx, span)).cloned() {
            let rendered = self.rust_ty(&effect);
            let (stub, is_plain) = match &effect {
                Ty::Named { name, .. } => {
                    let is_plain = self
                        .symbols
                        .effects
                        .get(name.as_str())
                        .is_some_and(|e| !e.is_actor);
                    (
                        if is_plain {
                            monitor_struct_name(name)
                        } else {
                            stub_struct_name(name)
                        },
                        is_plain,
                    )
                }
                _ => {
                    self.error("internal: a `use addr` with no effect recorded");
                    return String::new();
                }
            };
            let addr_code = self.emit_owned(handler);
            // [monitor-handler] [rs-monitor] A plain effect's handle already
            // *is* the effect's lock wrapper, so binding it is binding the
            // value itself; an actor's addr is an index that the send stub
            // turns into an instance.
            let instance = if is_plain {
                addr_code
            } else {
                format!("{stub}::new({addr_code})")
            };
            // [use-local] A handle binding that a later construction
            // captures: the handle *is* the clonable thing, so the eager
            // variable is a plain clone minted before the value moves into
            // the fusion.
            let mut prelude = String::new();
            let mut handle_var: Option<String> = None;
            let mut instance = instance;
            if is_plain && self.captured_effects.contains(&rendered) {
                let bind = self.unique_name("__bind".to_string());
                let hv = self.unique_name("__handle".to_string());
                prelude.push_str(&format!("{pad}let mut {bind} = {instance};\n"));
                prelude.push_str(&format!("{pad}let {hv} = {bind}.clone();\n"));
                instance = bind;
                handle_var = Some(hv);
            }
            if self.fusion {
                let code = self.emit_fusion_instance(
                    Vec::new(),
                    instance,
                    Some(effect),
                    rendered.clone(),
                    indent,
                );
                if let Some(hv) = handle_var {
                    for entry in self.effect_env.iter_mut().rev() {
                        if entry.key == rendered && entry.handle_var.is_none() {
                            entry.handle_var = Some(hv.clone());
                            break;
                        }
                    }
                }
                return format!("{prelude}{code}");
            }
            let var = self.unique_name(effect_param_name(&rendered));
            self.effect_env.push(EffectEntry {
                ty: Some(effect),
                key: rendered,
                var: var.clone(),
                is_local: true,
                handle_var,
            });
            self.bindings.insert(var.clone(), BindKind::Owned);
            return format!("{prelude}{pad}let mut {var} = {instance};\n");
        }
        let (handler_name, args): (String, Vec<&Expr>) = match handler {
            Expr::Ident(id) => (id.name.clone(), Vec::new()),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => (id.name.clone(), args.iter().collect()),
                _ => {
                    self.error("`use` expects a handler name or constructor call");
                    return String::new();
                }
            },
            _ => {
                self.error("`use` expects a handler name or constructor call");
                return String::new();
            }
        };
        let Some(decl) = self.symbols.handlers.get(handler_name.as_str()) else {
            self.error(format!("unknown handler `{handler_name}` in `use`"));
            return String::new();
        };
        // [use-local] The binding kind, decided by the checker (shareable by
        // default, user decision 2026-09-20): a monitor binds lock-shaped
        // from birth — `__Lock_E::new(H::new(args))`, at the concrete type,
        // so local calls pay the lock and nothing else — and a stateless
        // handler binds bare. `use local` keeps the pre-2026-09-20 emission.
        let kind = self
            .checked
            .use_kinds
            .get(&(self.file_idx, span))
            .copied()
            .unwrap_or(salvo_core::UseKind::Local);
        // [use-local] [effect-handler-deps] Dependency handles this
        // construction captures (a handle-dep handler): one trailing `new`
        // argument per dep, cloned off the eager handle variable its
        // binding site created (the binding value itself lives inside the
        // fusion, so the handle is minted where the value is still
        // whole).
        let captures = self
            .checked
            .handle_captures
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        // [effect-handler-multi] One entry per implemented effect, in
        // declaration order: a `use` binds every face the handler wears.
        let faces: Vec<(Option<Ty>, String)> =
            match self.checked.use_effects.get(&(self.file_idx, span)) {
                Some(tys) if tys.iter().all(ty_is_concrete) => {
                    let tys = tys.clone();
                    tys.into_iter()
                        .map(|ty| {
                            let rendered = self.rust_ty(&ty);
                            (Some(ty), rendered)
                        })
                        .collect()
                }
                _ => decl
                    .of
                    .clone()
                    .iter()
                    .map(|of| (None, self.emit_type(of)))
                    .collect(),
            };
        // Ctor args are owned (a `use` argument is a move [deduce-infer]).
        let mut arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        // [copy-implicit] The constructor's implicit parameters follow, as
        // the adapters the checker resolved at this `use` — rendered like a
        // call's implicit arguments, owned (they are stored in the handler).
        let saved_producer = self.emitting_producer_args;
        self.emitting_producer_args = true;
        arg_code.extend(self.emit_implicit_args(&[], span));
        self.emitting_producer_args = saved_producer;
        // [effect-handler-generics] A generic handler is constructed *at* a
        // type: with no argument to infer from, rustc needs the turbofish
        // (`E0283` otherwise).
        let turbofish = match self.checked.use_handler_args.get(&(self.file_idx, span)) {
            Some(args) if args.iter().all(ty_is_concrete) => {
                let args = args.clone();
                let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
                format!("::<{}>", rendered.join(", "))
            }
            _ => String::new(),
        };
        // [with-clause] Which captured dependency came from a written item
        // (a private instance, built here) rather than from the scope.
        let clause_of = self
            .checked
            .use_with_items
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        for (i, dep) in captures.iter().enumerate() {
            // [with-clause] A written item is a **private instance**: it is
            // constructed at this site and boxed into the effect's handle,
            // exactly as a binding's eager handle is — so the handler's
            // field type is unchanged and nothing downstream learns where
            // the instance came from.
            if let Some(Some(at)) = clause_of.get(i) {
                match with_items.get(*at) {
                    Some(item) => {
                        arg_code.push(self.with_item_handle(item, dep));
                        continue;
                    }
                    None => {
                        self.error(format!(
                            "internal: the `with` clause has no item {at} for \
                             captured dependency `{dep}`"
                        ));
                        arg_code.push("todo!()".to_string());
                        continue;
                    }
                }
            }
            match self.effect_entry_by_ty(dep) {
                Some(entry) => match &entry.handle_var {
                    Some(hv) => arg_code.push(format!("{hv}.clone()")),
                    None => {
                        // [spawn-inherit] No eager handle here means the
                        // effect is **signature-supplied**: the handle came
                        // in through this fn's hidden bundle parameter.
                        match self.handle_place(dep) {
                            Some(place) => arg_code.push(format!("{place}.clone()")),
                            None => {
                                self.error(format!(
                                    "internal: the binding for captured dependency \
                                     `{dep}` has no handle variable and this fn has \
                                     no handle for it"
                                ));
                                arg_code.push("todo!()".to_string());
                            }
                        }
                    }
                },
                None => match self.handle_place(dep) {
                    Some(place) => arg_code.push(format!("{place}.clone()")),
                    None => {
                        self.error(format!(
                            "internal: no binding in scope for captured dependency \
                             `{dep}`"
                        ));
                        arg_code.push("todo!()".to_string());
                    }
                },
            }
        }
        // The construction, wrapped for a monitor binding ([use-local]
        // [rs-monitor]: the per-effect lock adapter at its concrete type, so
        // local calls pay the lock and nothing else).
        let mut ctor = format!(
            "{}{turbofish}::new({})",
            self.handler_ctor_path(&handler_name, decl),
            arg_code.join(", ")
        );
        // [rs-monitor] A monitor binding wraps in the per-effect lock
        // adapter. [rs-platform-handler] A **platform handler** classifies
        // bare (assumed thread-safe, user decision 2026-09-20) but shares
        // through the same adapter on this backend: its members take
        // `&mut self`, and a local binding coexisting with captured handles
        // needs shared ownership — which `Arc` provides and the host struct
        // (no `Clone`, real host state) cannot. The lock is the mechanics of
        // sharing here, not a semantic monitor claim; Kotlin shares the raw
        // instance.
        let lock_shared =
            kind == salvo_core::UseKind::Monitor || (kind != salvo_core::UseKind::Local && decl.platform);
        if lock_shared {
            if let Some((Some(Ty::Named { name, .. }), _)) = faces.first() {
                // The lock adapter is generic only over the handler, so its
                // one type argument is inferred from the construction.
                ctor = format!(
                    "{}::new({ctor})",
                    self.effect_path(name, &lock_struct_name(name))
                );
            }
        }
        // [use-local] The eager handle: minted beside the binding when a
        // later construction in this file captures the effect. A monitor's
        // clone is an `Arc` bump (same instance); a stateless clone is
        // indistinguishable from the instance.
        let mut prelude = String::new();
        let mut handle_var: Option<String> = None;
        if kind != salvo_core::UseKind::Local {
            if let Some((Some(ty @ Ty::Named { name, .. }), key)) = faces.first() {
                if self.captured_effects.contains(key) {
                    let bind = self.unique_name("__bind".to_string());
                    let hv = self.unique_name("__handle".to_string());
                    let ty = ty.clone();
                    let mon = self.effect_path(name, &monitor_struct_name(name));
                    // A generic instance names the wrapper's type arguments
                    // outright: inference would have to thread them through
                    // the unsize coercion, which rustc refuses to guess.
                    let mon_args = self.effect_instance_turbofish(&ty);
                    prelude.push_str(&format!("{pad}let mut {bind} = {ctor};\n"));
                    prelude.push_str(&format!(
                        "{pad}let {hv} = {mon}{mon_args}::new(Box::new({bind}.clone()));\n"
                    ));
                    ctor = bind;
                    handle_var = Some(hv);
                }
            }
        }
        if self.fusion {
            let code = self.emit_fusion_inner(
                Some(decl),
                &handler_name,
                "",
                Vec::new(),
                Some(ctor),
                faces.clone(),
                indent,
            );
            // The entries the fusion just pushed for these faces learn the
            // handle, so a later capture can clone it.
            if let Some(hv) = handle_var {
                let keys: Vec<String> = faces.iter().map(|(_, k)| k.clone()).collect();
                for entry in self.effect_env.iter_mut().rev() {
                    if keys.contains(&entry.key) && entry.handle_var.is_none() {
                        entry.handle_var = Some(hv.clone());
                    }
                }
            }
            return format!("{prelude}{code}");
        }
        let var = self.unique_name(effect_param_name(&faces[0].1));
        for (checked_ty, effect_ty) in faces.clone() {
            self.effect_env.push(EffectEntry {
                ty: checked_ty,
                key: effect_ty,
                var: var.clone(),
                is_local: true,
                handle_var: handle_var.clone(),
            });
        }
        self.bindings.insert(var.clone(), BindKind::Owned);
        format!("{prelude}{pad}let mut {var} = {ctor};\n")
    }

    /// [with-clause] [rs-monitor] A written `with` item as the **handle** the
    /// depending handler captures: a handler construction becomes a private
    /// instance boxed into the effect's `__Mon_E` (the same shape a binding's
    /// eager handle takes, so the field type is unchanged), and an `Addr`
    /// value is already a handle and passes through as itself.
    fn with_item_handle(&mut self, item: &Expr, dep: &Ty) -> String {
        let named = match item {
            Expr::Ident(id) => Some((id.name.clone(), Vec::new())),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => Some((id.name.clone(), args.iter().collect())),
                _ => None,
            },
            _ => None,
        };
        if let Some((name, args)) = named {
            if let Some(decl) = self.symbols.handlers.get(name.as_str()).copied() {
                let arg_code: Vec<String> = args.iter().map(|a| self.emit_owned(a)).collect();
                let ctor = self.handler_ctor_path(&name, decl);
                let inner = format!("{ctor}::new({})", arg_code.join(", "));
                let Ty::Named { name: effect, .. } = dep.strip_quals() else {
                    self.error(format!(
                        "internal: the `with` item for `{dep}` supplies no named effect"
                    ));
                    return "todo!()".to_string();
                };
                let effect = effect.clone();
                let mon = self.effect_path(&effect, &monitor_struct_name(&effect));
                let mon_args = self.effect_instance_turbofish(dep);
                // A private instance is stateful as often as not, and the
                // handle must own it: the lock adapter is what gives a
                // captured instance shared ownership on this backend, exactly
                // as it does for a monitor binding ([rs-platform-handler]
                // records the same mechanics for a host struct).
                let lock = self.effect_path(&effect, &lock_struct_name(&effect));
                return format!("{mon}{mon_args}::new(Box::new({lock}::new({inner})))");
            }
        }
        // Not a handler name, so it is an addr — already a handle.
        self.emit_owned(item)
    }

    /// [use-local] [effect-handler-deps] The shared dependency-form
    /// predicate, with the symbol table answering the face kind.
    fn handler_is_handle_dep(&self, decl: &HandlerDecl) -> bool {        salvo_core::handler_handle_deps(decl, |name| {
            self.symbols
                .effects
                .get(name)
                .is_some_and(|e| e.is_actor)
        })
    }

    /// [platform-handler] [rs-platform-handler] The type a `use` constructs. An
    /// ordinary handler's is the emitted struct of the same name; a platform
    /// handler's is the *host's*, in the mounted `platform/` companion of the
    /// module that declared it — named through `crate::`, because a `use` may
    /// sit in any module and nothing imports the host mount.
    fn handler_ctor_path(&mut self, name: &str, decl: &HandlerDecl) -> String {
        if !decl.platform {
            return rs_ident(name);
        }
        match self.symbols.handler_modules.get(name) {
            Some(module) => {
                self.platform_hosts.insert((*module).clone());
                format!("crate::{}::{}", host_mod_name(module), rs_ident(name))
            }
            None => {
                self.error(format!(
                    "internal: the declaring module of platform handler `{name}` \
                     could not be located"
                ));
                rs_ident(name)
            }
        }
    }

    /// [rs-effect-fusion] `use H(...)` under the fusion: one generated
    /// struct owning the handler, chained to whatever provided the effects
    /// already in scope, and implementing all of them.
    ///
    /// Chaining (rather than holding the outer scope's handlers as
    /// individual fields) is what lets an inner scope's fusion coexist with
    /// an outer one that is used again after the inner block: lexical
    /// nesting becomes borrow nesting. It also means a dependency is
    /// *always* reachable through `__outer` — it had to be registered
    /// before its dependent ([effect-handler-deps]), so it can never be a
    /// sibling field.
    /// [rs-effect-fusion] [actor-use-addr] Fuse one effect **instance** into the
    /// scope, given the expression that builds it. Two callers: `use H(…)`,
    /// whose instance is `H::new(args)`, and `use addr`, whose instance is the
    /// forwarding stub `__Stub_E::new(addr)` — identical from here on, which is
    /// what makes an actor and a local handler interchangeable.
    fn emit_fusion_instance(
        &mut self,
        arg_code: Vec<String>,
        instance: String,
        checked_ty: Option<Ty>,
        effect_ty: String,
        indent: usize,
    ) -> String {
        self.emit_fusion_inner(
            None,
            "",
            "",
            arg_code,
            Some(instance),
            vec![(checked_ty, effect_ty)],
            indent,
        )
    }

    #[allow(clippy::too_many_arguments)]
    /// The shared body: `decl` is present for a handler `use` (its dependencies
    /// and generics are checked), absent for a stub; `instance` overrides the
    /// constructor call when the caller already has the expression.
    #[allow(clippy::too_many_arguments)]
    fn emit_fusion_inner(
        &mut self,
        decl: Option<&HandlerDecl>,
        handler_name: &str,
        turbofish: &str,
        arg_code: Vec<String>,
        instance: Option<String>,
        faces: Vec<(Option<Ty>, String)>,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // [use-no-dup] [effect-intercept] The effects this fusion inherits,
        // innermost first and **deduplicated by instance**: a `use` may
        // shadow an earlier registration of the same effect, and two
        // `__Has_E` impls on one fusion struct is E0119. The entries are
        // re-listed outermost-first afterwards so field order and the
        // provider trait stay stable against the non-shadowing case.
        let mut covered: Vec<EffectEntry> = Vec::new();
        for entry in self.effect_env.iter().rev() {
            if !covered.iter().any(|e| self.same_instance(e, entry)) {
                covered.push(entry.clone());
            }
        }
        covered.reverse();
        let provider = self.fused_recv();
        // A stub has no dependencies of its own; a handler's are checked.
        // [use-local] A **handle-dep** handler's are inside the instance
        // (captured at construction), so the fusion treats it as
        // independent.
        let deps: Vec<(String, Vec<String>)> = match decl {
            Some(d) if !self.handler_is_handle_dep(d) => self.handler_dep_effects(d),
            _ => Vec::new(),
        };
        if !deps.is_empty() && covered.is_empty() {
            self.error(format!(
                "internal: handler `{handler_name}` has dependencies but no \
                 effect is in scope at its `use`"
            ));
            self.register_failed_fusion(faces);
            return String::new();
        }
        // [effect-handler-multi] The effects this handler adds — one per face,
        // preferring the checker's instance. Every one of them gets its own
        // accessor on the fusion, so a caller reaches whichever face its effect
        // list names.
        let mut new_effects: Vec<(String, Vec<String>)> = Vec::new();
        for (i, (checked_ty, _)) in faces.iter().enumerate() {
            let parts = match checked_ty {
                Some(Ty::Named { name, args }) => {
                    let name = name.clone();
                    let args = args.clone();
                    let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
                    Some((name, rendered))
                }
                _ => decl
                    .and_then(|d| d.of.get(i).cloned())
                    .and_then(|of| self.named_type_parts(&of)),
            };
            match parts {
                Some(parts) => new_effects.push(parts),
                None => {
                    self.error(format!(
                        "handler `{handler_name}` implements a type the rust \
                         backend cannot fuse"
                    ));
                    return String::new();
                }
            }
        }
        // A fusion names its effects in impl headers, so every one of them
        // must be a *concrete* instance. An unresolved type argument (a
        // generic handler whose arguments the checker could not infer, or an
        // effect list generic in the enclosing fn) is reported rather than
        // emitted as an undeclared `T` [backend-never-wrong].
        let mut named: Vec<String> = covered.iter().map(|e| e.key.clone()).collect();
        for (base, args) in &new_effects {
            named.push(trait_type(base, args));
        }
        let unresolved: Option<String> = decl.and_then(|decl| {
            decl.generics
                .iter()
                .map(|g| g.name.clone())
                .chain(self.generics.iter().cloned())
                .find(|g| named.iter().any(|n| mentions_ident(n, g)))
        });
        if let Some(g) = unresolved {
            self.error(format!(
                "`use {handler_name}` fuses effects whose type arguments are still \
                 generic (`{g}`) — the rust backend needs a concrete effect \
                 instance here"
            ));
            self.register_failed_fusion(faces);
            return String::new();
        }
        // The `__outer` field type: the single inherited effect's Has
        // trait, or a generated provider trait over all of them (one
        // field, because N reborrows of the same provider would alias).
        let outer_ty = match covered.len() {
            0 => None,
            1 => {
                let (base, args) = self.entry_effect_parts(&covered[0]);
                Some(has_trait_type(&base, &args))
            }
            _ => {
                let keys: Vec<String> = covered.iter().map(|e| e.key.clone()).collect();
                Some(self.prov_trait(&keys))
            }
        };
        // [rs-effect-fusion] The struct is built under a *placeholder* name
        // so identical fusions can be deduplicated by their text (user
        // decision 2026-09-14): several `use` sites — in one fn or across
        // fns — routinely register the same handler over the same inherited
        // set, and each was emitting its own struct and impls.
        let struct_name = FUSION_PLACEHOLDER.to_string();
        let (impl_generics, self_ty) = match &outer_ty {
            Some(_) => ("<'a, __H>".to_string(), format!("{struct_name}<'a, __H>")),
            None => ("<__H>".to_string(), format!("{struct_name}<__H>")),
        };
        let mut item = match &outer_ty {
            Some(ty) => format!(
                "\npub struct {struct_name}<'a, __H> {{\n    __outer: &'a mut dyn {ty},\n    \
                 __h: __H,\n}}\n"
            ),
            None => format!("\npub struct {struct_name}<__H> {{\n    __h: __H,\n}}\n"),
        };
        // Inherited effects: Has-accessor impls forwarding through the
        // provider. UFCS with the trait's turbofish — supertrait
        // elaboration makes `dyn __Prov_…: __Has_E<…>` hold, and the
        // turbofish disambiguates two instances of a generic effect.
        // [effect-intercept] The instance this `use` *shadows* is skipped:
        // its accessor is the new handler's (below), while `__outer` still
        // carries the shadowed one so the handler's own dependency reaches
        // it — which is the whole of "binds strictly outward" in emission.
        let mut inherited: Vec<(String, Vec<String>)> = covered
            .iter()
            .map(|e| self.entry_effect_parts(e))
            .filter(|parts| !new_effects.contains(parts))
            .collect();
        // Canonical order, so two orderings of one inherited set are one
        // struct after deduplication.
        inherited.sort();
        for (base, args) in &inherited {
            let body = format!(
                "{}::{}(&mut *self.__outer)",
                has_trait_path(base, args),
                has_getter_name(base)
            );
            item.push_str(&emit_has_impl(&impl_generics, &self_ty, base, args, &body));
        }
        // The new effect's accessor reaches the owned handler: directly for
        // an independent one; for a dependent one the *fusion itself* is
        // the `dyn` the accessor returns, since the raw effect impl (with
        // its disjoint field borrows) lives on the fusion.
        let handler_ident = rs_ident(handler_name);
        // [platform-handler] The *constructor* may be a host path
        // (`crate::platform_main::HostFs`) where the identifier above is
        // only ever a name — trait names derived from it stay identifiers.
        // A stub arrives as a ready expression; a handler is constructed here.
        let ctor_path = match decl {
            Some(d) => self.handler_ctor_path(handler_name, d),
            None => String::new(),
        };
        if deps.is_empty() {
            // [effect-handler-multi] The handler must satisfy every face it
            // implements, so the bound names all of them and each gets its own
            // accessor onto the one owned instance.
            let bound = new_effects
                .iter()
                .map(|(base, args)| trait_type(base, args))
                .collect::<Vec<String>>()
                .join(" + ");
            let bounded_generics = match &outer_ty {
                Some(_) => format!("<'a, __H: {bound}>"),
                None => format!("<__H: {bound}>"),
            };
            for (base, args) in &new_effects {
                item.push_str(&emit_has_impl(
                    &bounded_generics,
                    &self_ty,
                    base,
                    args,
                    "&mut self.__h",
                ));
            }
        } else {
            let trait_name = format!("__Impl_{handler_ident}");
            let bounded_generics = match &outer_ty {
                Some(_) => format!("<'a, __H: {trait_name}>"),
                None => format!("<__H: {trait_name}>"),
            };
            for (base, args) in &new_effects {
                item.push_str(&self.emit_forward_impl(
                    &bounded_generics,
                    &self_ty,
                    base,
                    args,
                    &Forward::Dependent {
                        trait_name: trait_name.clone(),
                        deps: deps.len(),
                    },
                ));
                item.push_str(&emit_has_impl(
                    &bounded_generics,
                    &self_ty,
                    base,
                    args,
                    "self",
                ));
            }
        }
        // Deduplicate: an identical fusion (same inherited accessors, same
        // new effect, same handler kind) is one struct, named after the fn
        // that needed it first.
        let struct_name = match self.fusion_structs.get(&item) {
            Some(existing) => existing.clone(),
            None => {
                self.fusion_id += 1;
                let name = format!(
                    "__Fx_{}_{}",
                    sanitize_ident(&self.current_fn),
                    self.fusion_id
                );
                self.fusion_structs.insert(item.clone(), name.clone());
                self.generated_items
                    .push(item.replace(FUSION_PLACEHOLDER, &name));
                name
            }
        };

        // The fusion local. Constructor arguments that reach the provider
        // must be evaluated before the struct literal borrows it.
        let (prelude, arg_code) = self.hoist_fused_args(arg_code);
        let var = self.unique_name("__fx".to_string());
        let mut out = String::new();
        for line in &prelude {
            out.push_str(&format!("{pad}{line}\n"));
        }
        // [actor-use-addr] The instance: a stub's expression as given, or the
        // handler's constructor call built here.
        let held = match &instance {
            Some(expr) => expr.clone(),
            None => format!("{ctor_path}{turbofish}::new({})", arg_code.join(", ")),
        };
        let fields = match provider {
            Some(p) => format!("__outer: {p}, __h: {held}"),
            None => format!("__h: {held}"),
        };
        out.push_str(&format!(
            "{pad}let mut {var} = {struct_name} {{ {fields} }};\n"
        ));
        // Every effect in scope now threads through the new fusion.
        for entry in self.effect_env.iter_mut() {
            entry.var = var.clone();
            entry.is_local = true;
        }
        for (checked_ty, effect_ty) in faces {
            self.effect_env.push(EffectEntry {
                ty: checked_ty,
                key: effect_ty,
                var: var.clone(),
                is_local: true,
                handle_var: None,
            });
        }
        self.bindings.insert(var, BindKind::Owned);
        out
    }

    /// After a reported fusion failure, still register the effect so the
    /// rest of the scope reports its *own* problems rather than a cascade of
    /// "no handler for effect" [type-unknown-lenient].
    fn register_failed_fusion(&mut self, faces: Vec<(Option<Ty>, String)>) {
        for (checked_ty, effect_ty) in faces {
            self.effect_env.push(EffectEntry {
                ty: checked_ty,
                key: effect_ty,
                var: "__fx_unemittable".to_string(),
                is_local: true,
                handle_var: None,
            });
        }
    }

    /// [rs-effect-fusion] The provider trait for a set of effects — the
    /// only place a fused value needs a *nameable* type (`__outer` fields,
    /// fn-value effect parameters). Its supertraits are the effects'
    /// **Has-accessor traits**, so a `dyn` provider reaches every effect
    /// through UFCS on the supertrait (supertrait elaboration makes
    /// `dyn __Prov_…: __Has_E<…>` hold). Generated per file that needs it;
    /// the blanket impl makes duplication across files harmless, since any
    /// type satisfying the parts satisfies every copy.
    fn prov_trait(&mut self, keys: &[String]) -> String {
        let mut parts: Vec<String> = keys.to_vec();
        parts.sort();
        parts.dedup();
        let name = format!(
            "__Prov_{}",
            parts
                .iter()
                .map(|p| sanitize_ident(p))
                .collect::<Vec<_>>()
                .join("_")
        );
        let supers = parts
            .iter()
            .map(|p| {
                let (base, args) = split_rendered_generic(p);
                has_trait_type(&base, &args)
            })
            .collect::<Vec<_>>()
            .join(" + ");
        match self.conj_traits.get(&name) {
            Some(existing) if *existing != supers => {
                // Sanitizing `<`/`,` to `_` is not injective (an effect
                // literally named `Random_i32` collides with `Random<i32>`).
                // Vanishingly unlikely, and silently reusing the wrong trait
                // would be wrong code, so it is reported [backend-never-wrong].
                self.error(format!(
                    "the generated provider trait `{name}` is claimed by two \
                     different effect sets (`{existing}` and `{supers}`) — rename \
                     one of the effects"
                ));
            }
            Some(_) => {}
            None => {
                self.conj_traits.insert(name.clone(), supers.clone());
                self.generated_items.push(format!(
                    "\npub trait {name}: {supers} {{}}\nimpl<T: {supers} + ?Sized> {name} for T {{}}\n"
                ));
            }
        }
        name
    }

    /// The effect base name and rendered type arguments behind an effect
    /// environment entry, preferring the checker's `Ty` (the rendered key
    /// is the unchecked fallback).
    fn entry_effect_parts(&mut self, entry: &EffectEntry) -> (String, Vec<String>) {
        if let Some(Ty::Named { name, args }) = &entry.ty {
            let name = name.clone();
            let args = args.clone();
            let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
            return (name, rendered);
        }
        split_rendered_generic(&entry.key)
    }

    fn emit_expr_stmt(&mut self, expr: &Expr, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if_stmt(branches, else_block.as_ref(), indent, ctx),
            // [when-condition] The subject-less `when` is a condition
            // chain, and Rust has no subject-less `match`: it lowers to
            // `if`/`else if`/`else` [rs-when-cond]. The `else` is
            // mandatory, so the chain is total without a filler arm.
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => self.emit_if_stmt(branches, Some(else_block), indent, ctx),
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                // Statement position: value discarded; `else` runs only
                // if the loop never did [while-value].
                let ran = else_block
                    .as_ref()
                    .map(|_| format!("{}_ran", self.fresh_loop_var()));
                let inner_pad = "    ".repeat(indent + 1);
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}let mut {ran} = false;\n"));
                }
                // [is-bind-once] A subject that is not a place is evaluated
                // **once per iteration**, into a temporary the test and the
                // binding share — so the loop becomes a `loop` with the test
                // inside it. `while <call> is T x` used to emit the call twice
                // per turn, which silently dropped every other value.
                match self.hoistable_is(cond) {
                    Some(subject) => {
                        out.push_str(&format!("{pad}loop {{\n"));
                        let temp = self.emit_is_temp(subject, indent + 1);
                        out.push_str(&temp);
                        let c = self.emit_expr(cond);
                        out.push_str(&format!(
                            "{inner_pad}if !({}) {{\n{inner_pad}    break;\n{inner_pad}}}\n",
                            cond_code(c)
                        ));
                    }
                    None => {
                        let c = self.emit_expr(cond);
                        out.push_str(&format!("{pad}while {} {{\n", cond_code(c)));
                    }
                }
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true;\n"));
                }
                out.push_str(&self.emit_is_bindings(cond, indent + 1));
                self.loop_results.push(None);
                self.loop_splice_floors.push(self.exit_splices.len());
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                self.loop_splice_floors.pop();
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if !{ran} {{\n"));
                    out.push_str(&self.emit_block_stmts(b, indent + 1, ctx));
                    out.push_str(&format!("{pad}}}\n"));
                }
                out
            }
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => {
                let ran = else_block
                    .as_ref()
                    .map(|_| format!("{}_ran", self.fresh_loop_var()));
                let inner_pad = "    ".repeat(indent + 1);
                // [iter-fn] [fn-effects] A **producer** is driven, not
                // iterated: minted, advanced, and closed. That is where the
                // injected `close` gets its caller — an `Iter<T>` reached as a
                // target-language iterator had nowhere to put one, so an
                // abandoned producer skipped its release. The handler
                // list is empty for a pure producer and the claimed set for a
                // claiming one; nothing else differs.
                // [iter-fn] An origin comes first: its machine is
                // constructed here rather than minted from a factory, and it
                // closes like a producer does.
                // [linear-group] No implicit discharge sites (user decision
                // 2026-09-12): the loop never closes a pass — a linear one
                // is checker-refused unless kept, so nothing here splices.
                let producer: Option<(String, String)> = None;
                // [iter-protocol] A **pass** is *driven*, not iterated: the
                // header calls the `next` the checker resolved. Everything
                // after it — the `else` bookkeeping, the body, the spliced
                // floors — is the same as for any other loop.
                let pass = if producer.is_some() {
                    None
                } else {
                    self.pass_driver_of(iterable)
                        .filter(|d| !d.origin)
                        .map(|driver| self.emit_pass_loop_header(driver, pattern, iterable, indent))
                };
                // [rs-borrow-locals] S3: a borrow-mode loop (not made
                // by-value by a move-mode event [fate-move-mode])
                // over a pure-place iterable with a plain ident binding
                // iterates *by reference* — no clone of the collection,
                // and the loop variable is a reference binding. Guarded
                // to concrete non-union element types: union/optional
                // elements go through `matches!`/unwrap lowering that
                // expects owned subjects, and keep the clone path.
                let by_value = self
                    .checked
                    .binding_modes
                    .contains(&(self.file_idx, iterable.span()));
                // A native `for` iterates data: borrowing the subject is what
                // makes the loop variable a reference [rs-borrow-locals].
                let iter_subject = false;
                let elem_ok = self
                    .ty_of(iterable.span())
                    .map(|t| match t.strip_quals() {
                        Ty::Named { name, args } if name == "List" => {
                            args.first().is_some_and(|e| {
                                matches!(e.strip_quals(), Ty::Named { .. })
                                    && !matches!(e.strip_quals(), Ty::Union(_))
                            })
                        }
                        Ty::Array(e) => {
                            matches!(e.strip_quals(), Ty::Named { .. })
                        }
                        _ => false,
                    })
                    .unwrap_or(false);
                let (var, iter) = if producer.is_some() || pass.is_some() {
                    // Both headers bound the element themselves.
                    (String::new(), String::new())
                } else {
                    let borrow_iter = if iter_subject {
                        self.borrow_value(iterable)
                    } else if !by_value && elem_ok && matches!(pattern, Pattern::Ident(_)) {
                        self.borrow_value(iterable)
                    } else {
                        None
                    };
                    let by_ref = borrow_iter.is_some() && !iter_subject;
                    let var = self.for_pattern_var(pattern, by_ref, indent + 1);
                    let iter = match borrow_iter {
                        Some(code) => code,
                        None => self.emit_bound_value(iterable, iterable.span()),
                    };
                    let iter = self.native_for_subject(iterable, iter);
                    (var, iter)
                };
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}let mut {ran} = false;\n"));
                }
                match (&producer, &pass) {
                    (Some((header, ..)), _) => out.push_str(header),
                    (None, Some(header)) => out.push_str(header),
                    (None, None) => out.push_str(&format!("{pad}for {var} in {iter} {{\n")),
                }
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true;\n"));
                }
                // [let-destructure] A destructuring pattern's bindings open the
                // body, read off the element the header bound.
                out.push_str(&self.take_loop_destructure());
                self.loop_results.push(None);
                self.loop_splice_floors.push(self.exit_splices.len());
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                self.loop_splice_floors.pop();
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                // [fn-effects] The injected `close`, once: `break` and
                // exhaustion both land here, and the flags inside make it
                // idempotent. A `return` out of the body ran it already,
                // through the exit splice the header registered — which is
                // what that entry is for.
                if let Some((_, close)) = &producer {
                    self.exit_splices.pop();
                    out.push_str(&format!("{pad}{close}\n"));
                }
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if !{ran} {{\n"));
                    out.push_str(&self.emit_block_stmts(b, indent + 1, ctx));
                    out.push_str(&format!("{pad}}}\n"));
                }
                out
            }
            Expr::When {
                subject, branches, ..
            } => {
                let code = self.emit_when(subject, branches, indent, ctx, false);
                format!("{pad}{code}\n")
            }
            Expr::IncDec { operand, down, .. } => {
                // [inc-dec] Statement position discards the value, so the
                // fixity makes no difference: both are a step of one.
                let t = self.emit_raw(operand);
                let op = if *down { "-=" } else { "+=" };
                format!("{pad}{t} {op} 1;\n")
            }
            _ => {
                let code = self.emit_expr(expr);
                format!("{pad}{code};\n")
            }
        }
    }

    /// An `if`/`elif`/`else` chain in statement position: branch tails
    /// are emitted as statements.
    fn emit_if_stmt(
        &mut self,
        branches: &[(Expr, Block)],
        else_block: Option<&Block>,
        indent: usize,
        ctx: StmtCtx,
    ) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        for (i, (cond, block)) in branches.iter().enumerate() {
            // [is-bind-once] The subject is evaluated once, before the test.
            // Only the first branch can carry a `let` here, and that is exactly
            // what the checker allows — a later one is refused there, so this
            // arm is a guard against the two drifting apart, never a path a
            // legal program takes [backend-never-wrong].
            if let Some(subject) = self.hoistable_is(cond) {
                if i == 0 {
                    let temp = self.emit_is_temp(subject, indent);
                    out.push_str(&temp);
                } else {
                    self.error(
                        "an `is` binding over a call in an `elif` condition cannot be \
                         lowered: bind the subject with `let` first"
                            .to_string(),
                    );
                }
            }
            let kw = if i == 0 {
                format!("{pad}if")
            } else {
                "else if".to_string()
            };
            let c = self.emit_expr(cond);
            out.push_str(&format!("{kw} {} {{\n", cond_code(c)));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            // [qual-lift] A `^` condition peels a wrapper for the branch.
            let (shadows, saved) = self.emit_widen_shadows(cond, indent + 1);
            out.push_str(&shadows);
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
            self.restore_bindings(saved);
            out.push_str(&format!("{pad}}} "));
        }
        if let Some(block) = else_block {
            out.push_str("else {\n");
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
            out.push_str(&format!("{pad}}}\n"));
        } else {
            out.pop();
            out.push('\n');
        }
        out
    }

    /// For a condition containing `x is T name`, emits the binding
    /// declarations at the top of the matched branch.
    /// [is-bind-once] The `is` subject a condition needs hoisted, if any: a
    /// binding `is` whose subject is not a place, and which *is* the whole
    /// condition. Anything else — a chain, a later `else if` — is refused by
    /// the checker, so this sees only shapes it can handle.
    fn hoistable_is<'a>(&self, cond: &'a Expr) -> Option<&'a Expr> {
        match cond {
            Expr::Is {
                subject,
                binding: Some(_),
                ..
            } if !is_place_expr(subject) => Some(subject),
            _ => None,
        }
    }

    /// Emits the single evaluation of a hoisted subject and registers the
    /// temporary. Answers the line to place before the test.
    fn emit_is_temp(&mut self, subject: &Expr, indent: usize) -> String {
        self.is_temp_id += 1;
        let name = format!("__is{}", self.is_temp_id);
        let code = self.emit_expr(subject);
        self.is_temps
            .insert((self.file_idx, subject.span()), name.clone());
        format!("{}let mut {name} = {code};\n", "    ".repeat(indent))
    }

    fn emit_is_bindings(&mut self, cond: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut collected: Vec<(&Expr, &[TypeRef], &Ident, Span, bool)> = Vec::new();
        collect_is_bindings(cond, &mut |subject, check, binding, is_span, lift| {
            collected.push((subject, check, binding, is_span, lift));
        });
        let mut out = String::new();
        for (subject, _check, binding, is_span, lift) in collected {
            // [qual-lift] A lift whose check peels **no wrapper** (`is ^Mut
            // plain`) binds the subject itself: qualifiers are erased, so the
            // lifted value *is* the value. Without this the narrowed-read path
            // assumed an `Option` in front of it and emitted
            // `list.as_ref().unwrap().clone()` for a plain `&mut Vec`.
            // [rewrap] A multi-arm lift binds a *sub-union*, so the value is
            // built by mapping arm to arm rather than by reading one payload.
            if let Some(from) = self
                .checked
                .rewrap_from
                .get(&(self.file_idx, binding.span))
                .cloned()
            {
                if let Some(to) = self.ty_of(binding.span).cloned() {
                    // The mapping *consumes* its input, and the subject is
                    // still readable afterwards ([elvis-guard] and the arms
                    // below), so it takes an owned copy — which is what the
                    // coercion path has always done (`match a.clone() { … }`).
                    let storage = format!("{}.clone()", self.place_storage(subject));
                    let code = self.emit_rewrap(storage, &from, &to);
                    out.push_str(&format!(
                        "{pad}let mut {} = {code};\n",
                        rs_ident(&binding.name)
                    ));
                    self.bindings.insert(binding.name.clone(), BindKind::Owned);
                    continue;
                }
            }
            if lift && self.is_test_of(is_span).is_none() {
                let code = self.emit_place(subject);
                out.push_str(&format!(
                    "{pad}let {} = {code};\n",
                    rs_ident(&binding.name)
                ));
                self.bindings.insert(binding.name.clone(), BindKind::Ref);
                continue;
            }
            self.bindings.insert(binding.name.clone(), BindKind::Owned);
            let code = if self
                .checked
                .predicate_tests
                .contains_key(&(self.file_idx, is_span))
            {
                // Predicate checks refine only qualifiers (erased): the
                // binding is an owned copy of the subject.
                self.emit_expr(subject)
            } else {
                let target = self.ty_of(binding.span).cloned();
                // [linear-container] [rs-linear-move] A binding that takes a
                // **linear** payload out of an optional moves it: the ordinary
                // narrowed read is `x.as_ref().unwrap().clone()`, which
                // duplicates an obligation, and a reply token is not `Clone`
                // at all — so `remove_at(pending, i) is Reply<Fired> token`
                // was a raw rustc E0507 before this. Only the plain-optional
                // shape, exactly as `linear_move_unwrap` restricts itself: a
                // narrowed union *arm* keeps the accessor path.
                let linear = self
                    .checked
                    .linear_moves
                    .contains(&(self.file_idx, binding.span));
                let plain_option = self.is_test_of(is_span).is_none_or(|t| t.size < 2);
                if linear && plain_option {
                    let subj = self.place_storage(subject);
                    out.push_str(&format!(
                        "{pad}let mut {} = {subj}.unwrap();\n",
                        rs_ident(&binding.name)
                    ));
                    continue;
                }
                let code = self.emit_narrowed_read(
                    subject,
                    target.as_ref(),
                    self.is_test_of(is_span).cloned(),
                );
                if self.narrowed_read_is_ref {
                    self.bindings.insert(binding.name.clone(), BindKind::Ref);
                }
                code
            };
            out.push_str(&format!(
                "{pad}let mut {} = {code};\n",
                rs_ident(&binding.name)
            ));
        }
        out
    }

    /// [qual-lift] Materializes the peel a `^` check performs: the widened
    /// value is bound to a **shadowing** local for the branch, so reads of
    /// the subject — and any nested `when` on it — see it at the widened
    /// type. Returns the emitted lines and the binding kinds to restore when
    /// the branch ends (the shadow is owned; the outer binding may not be).
    fn emit_widen_shadows(
        &mut self,
        cond: &Expr,
        indent: usize,
    ) -> (String, Vec<(String, Option<BindKind>)>) {
        let pad = "    ".repeat(indent);
        let mut sites: Vec<(&Expr, Span)> = Vec::new();
        collect_widen_checks(cond, &mut |subject, span| sites.push((subject, span)));
        let mut out = String::new();
        let mut saved: Vec<(String, Option<BindKind>)> = Vec::new();
        for (subject, span) in sites {
            let Some(target) = self
                .checked
                .widen_targets
                .get(&(self.file_idx, span))
                .cloned()
            else {
                // No wrapper was peeled (the qualifier was statically
                // present and is erased): nothing to materialize.
                continue;
            };
            let Expr::Ident(id) = subject else {
                let place = self.emit_raw(subject);
                self.error(format!(
                    "`^` on a projection is not supported yet: widening \
                     materializes a local for the branch, which needs a \
                     plain variable — bind `{place}` to one first"
                ));
                continue;
            };
            let test = self.is_test_of(span).cloned();
            // [rs-narrow-mut] A peeled payload that is `Mut` is bound as a
            // **mutable borrow into the storage**, not an owned clone: the
            // branch may mutate it, and a mutation of the clone is lost when
            // the branch ends (found 2026-09-20 — the repro is in
            // COMPLETED.md). Reads through the binding are unaffected, since a
            // `&mut T` binding is what an ordinary `Mut` parameter already is
            // (`BindKind::RefMut`).
            if target.quals().iter().any(|q| q.name == "Mut") {
                if let Some(code) = self.emit_narrowed_read_mut(subject, test.as_ref()) {
                    let displaced = self.bindings.insert(id.name.clone(), BindKind::RefMut);
                    saved.push((id.name.clone(), displaced));
                    // No `mut` on the binding: it is a reference, and the
                    // generated code must stay warning-free.
                    out.push_str(&format!("{pad}let {} = {code};\n", rs_ident(&id.name)));
                    continue;
                }
                continue;
            }
            let code = self.emit_narrowed_read(subject, Some(&target), test);
            let kind = if self.narrowed_read_is_ref {
                BindKind::Ref
            } else {
                BindKind::Owned
            };
            let displaced = self.bindings.insert(id.name.clone(), kind);
            saved.push((id.name.clone(), displaced));
            out.push_str(&format!("{pad}let mut {} = {code};\n", rs_ident(&id.name)));
        }
        (out, saved)
    }

    /// Restores binding kinds displaced by a `^` shadow [qual-lift].
    fn restore_bindings(&mut self, saved: Vec<(String, Option<BindKind>)>) {
        for (name, kind) in saved.into_iter().rev() {
            match kind {
                Some(k) => self.bindings.insert(name, k),
                None => self.bindings.remove(&name),
            };
        }
    }

    /// [proj-field] Whether an assignment target is a `proj` field of a
    /// struct (by the checker's type of the base).
    fn assigns_proj_field(&mut self, target: &Expr) -> bool {
        let Expr::Field { base, field, .. } = target else {
            return false;
        };
        let Some(base_ty) = self.ty_of(base.span()).cloned() else {
            return false;
        };
        let Ty::Named { name, .. } = base_ty.strip_quals() else {
            return false;
        };
        let Some(sd) = self.symbols.structs.get(name.as_str()) else {
            return false;
        };
        sd.fields
            .iter()
            .any(|f| f.name.name == field.name && type_has_proj(&f.ty))
    }

    /// [rs-proj-arm] The match adapter that copies a union's borrowed
    /// Copy arms out into the owned union the position wants, or `None`
    /// when no adaptation applies (no borrowed arms, or the position's own
    /// arm is `proj` too). Errors on a non-Copy payload.
    fn adapt_borrowed_arms_to_owned(
        &mut self,
        arg: &Expr,
        param_ty: Option<&Type>,
        mode: ParamMode,
    ) -> Option<String> {
        let borrowed = match arg {
            Expr::Ident(id) => self
                .borrowed_arm_locals
                .get(&id.name)
                .cloned()
                .unwrap_or_default(),
            Expr::Call { .. } => self.call_borrowed_arms(arg),
            _ => Vec::new(),
        };
        if borrowed.is_empty() {
            return None;
        }
        let Some(Type::Union { arms, .. }) = param_ty else {
            return None;
        };
        let value_arms: Vec<&Type> = arms.iter().filter(|a| !is_none_type(a)).collect();
        // The position keeps the borrow: nothing to adapt.
        if borrowed
            .iter()
            .all(|&i| value_arms.get(i).is_some_and(|a| type_has_proj(a)))
        {
            return None;
        }
        let n = value_arms.len();
        if n < 2 {
            return None;
        }
        let mut pats: Vec<String> = Vec::new();
        for (i, arm) in value_arms.iter().enumerate() {
            if borrowed.contains(&i) && !type_has_proj(arm) {
                let inner = strip_proj(arm);
                // The arm's own qualifier (`Emitted`) is erased; Copy-ness is
                // the base's.
                let bare = match &inner {
                    Type::Named { base, .. } => Type::Named {
                        qualifiers: Vec::new(),
                        base: base.clone(),
                    },
                    other => other.clone(),
                };
                if !self.is_copy_ast_type(&bare) {
                    self.error(format!(
                        "a borrowed `{}` element cannot flow into a position that owns it \
                         (`{}`): bind the element and pass `copy(...)` of it, or write the \
                         position's arm as `proj`",
                        inner_name(&inner),
                        inner_name(arm)
                    ));
                    return None;
                }
                pats.push(format!(
                    "Union{n}::U{k}(__e) => Union{n}::U{k}(*__e)",
                    k = i + 1
                ));
            } else {
                pats.push(format!(
                    "Union{n}::U{k}(__e) => Union{n}::U{k}(__e)",
                    k = i + 1
                ));
            }
        }
        self.union_sizes.insert(n);
        let code = self.emit_owned(arg);
        let adapted = format!("(match {code} {{ {} }})", pats.join(", "));
        Some(match mode {
            ParamMode::Owned => adapted,
            ParamMode::Ref => format!("&{adapted}"),
            ParamMode::RefMut => format!("&mut {adapted}"),
        })
    }

    /// [rs-proj-arm] Whether `arm` of the subject's storage holds a
    /// reference: the subject is a local bound from a `proj`-arm call, or
    /// is such a call itself.
    fn subject_arm_is_borrowed(&mut self, subject: &Expr, arm: usize) -> bool {
        match subject {
            Expr::Ident(id) => self
                .borrowed_arm_locals
                .get(&id.name)
                .is_some_and(|arms| arms.contains(&arm)),
            Expr::Call { .. } => self.call_borrowed_arms(subject).contains(&arm),
            _ => false,
        }
    }

    /// [rs-proj-arm] The value-arm indices a call's result holds as
    /// references: the `proj` arms of the resolved callee's written return
    /// type. Empty for anything else.
    fn call_borrowed_arms(&mut self, call: &Expr) -> Vec<usize> {
        let Expr::Call { span, .. } = call else {
            return Vec::new();
        };
        let Some(key) = self.checked.call_fn.get(&(self.file_idx, *span)).copied() else {
            return Vec::new();
        };
        let Some(decl) = self.fn_by_key(key) else {
            return Vec::new();
        };
        let Some(Type::Union { arms, .. }) = decl.return_type.as_ref() else {
            return Vec::new();
        };
        arms.iter()
            .filter(|a| !is_none_type(a))
            .enumerate()
            .filter(|(_, a)| type_has_proj(a))
            .map(|(i, _)| i)
            .collect()
    }

    /// Reads the narrowed payload out of a subject for an `is` binding:
    /// enum-arm accessor for wrapper unions, `Option` unwrap for `T?`
    /// representations [rs-union-enums] [rs-option].
    fn emit_narrowed_read(
        &mut self,
        subject: &Expr,
        target: Option<&Ty>,
        test: Option<UnionTest>,
    ) -> String {
        // The payload comes out of the *storage*: if the subject place is
        // itself narrowed (a nested check on the same place), its own
        // unwrap must not be applied on top [flow-place].
        let subj = self.place_storage(subject);
        match test {
            Some(test) if test.size >= 2 => {
                // Wrapper union: unwrap the (unique) matched arm.
                let arm = test.arms.first().copied().unwrap_or(0);
                let access = if test.nullable {
                    format!("{subj}.as_ref().unwrap()")
                } else {
                    subj
                };
                let clone = match target {
                    Some(t) if Self::is_copy_ty(t) => "*",
                    _ => "",
                };
                // [rs-proj-arm] A borrowed arm holds `&T`: the accessor's
                // `&&T` derefs twice for a Copy scalar; otherwise the read
                // *is* the reference, and the binding is a `Ref`.
                let borrowed_arm = self.subject_arm_is_borrowed(subject, arm);
                self.narrowed_read_is_ref = false;
                if borrowed_arm && clone == "*" {
                    format!("**{access}.u{}()", arm + 1)
                } else if borrowed_arm {
                    self.narrowed_read_is_ref = true;
                    format!("*{access}.u{}()", arm + 1)
                } else if clone == "*" {
                    format!("*{access}.u{}()", arm + 1)
                } else {
                    format!("{access}.u{}().clone()", arm + 1)
                }
            }
            _ => {
                // `T?` representation (or a checked field): Option unwrap.
                match target {
                    Some(t) if Self::is_copy_ty(t) => format!("{subj}.unwrap()"),
                    _ => format!("{subj}.as_ref().unwrap().clone()"),
                }
            }
        }
    }

    /// [rs-narrow-mut] `emit_narrowed_read`'s mutable twin, for peeling an
    /// arm whose payload is `Mut`: the same unwrap through the `_mut`
    /// accessors, so the result is a `&mut T` *into the storage* rather than
    /// an owned clone of it.
    ///
    /// Used by the `^` branch shadow [rs-widen-shadow]. The owned shadow was
    /// silently wrong for a `Mut` payload — a mutation inside the branch
    /// landed on the clone and was gone when the branch ended, while Kotlin
    /// (which casts the storage) kept it. Found 2026-09-20; the defect and
    /// its repro are in COMPLETED.md.
    ///
    /// `None` when there is no `&mut` to give, having reported why.
    fn emit_narrowed_read_mut(&mut self, subject: &Expr, test: Option<&UnionTest>) -> Option<String> {
        self.check_peel_root_mutable(subject)?;
        if let Some(t) = test {
            if t.size >= 2 {
                let arm = t.arms.first().copied().unwrap_or(0);
                // [rs-proj-arm] A borrowed arm holds `&T`, which cannot yield
                // a `&mut`. A projection is read-only anyway
                // ([fate-derived-readonly]), so this should be unreachable —
                // report rather than fall back to the clone that was the bug
                // [backend-never-wrong].
                if self.subject_arm_is_borrowed(subject, arm) {
                    self.error(
                        "a `Mut` value cannot be peeled out of a borrowed union arm: \
                         the arm holds a projection, which is read-only"
                            .to_string(),
                    );
                    return None;
                }
                let subj = self.place_storage_mut(subject);
                let access = if t.nullable {
                    format!("{subj}.as_mut().unwrap()")
                } else {
                    subj
                };
                return Some(format!("{access}.u{}_mut()", arm + 1));
            }
        }
        // `T?` representation: the Option is unwrapped mutably in place.
        let subj = self.place_storage_mut(subject);
        Some(format!("{subj}.as_mut().unwrap()"))
    }
}

impl<'p> Emitter<'p> {
    // ================= expressions =================

    /// Emits an expression as an *owned* value with any checker-recorded
    /// coercion applied [rs-borrows].
    fn emit_expr(&mut self, expr: &Expr) -> String {
        let code = self.emit_owned(expr);
        self.apply_coercion(expr.span(), code)
    }

    /// The narrowing unwrap for identifier uses [rs-union-enums]
    /// [rs-option]: `None` when the ident is not narrowed. The result is
    /// owned.
    fn ident_unwrap(&mut self, id: &Ident) -> Option<String> {
        if !self.checked.repr_ty.contains_key(&(self.file_idx, id.span)) {
            return None;
        }
        let storage = self.binding_place(&id.name);
        // [rs-opt-borrow] The storage is `Option<&T>`: `.unwrap()` yields
        // the `&T` itself (the `Option` is Copy, so no `as_ref`), and the
        // owned result is a clone *through* that reference — a `T`, where
        // the generic path's `.as_ref().unwrap().clone()` produced an `&T`.
        // A Copy payload needs no clone at all.
        if matches!(self.bindings.get(id.name.as_str()), Some(BindKind::OptRef)) {
            let n = self.narrowing_of(id.span)?;
            if n.arm.is_some() {
                // A union payload behind a borrow is a later stage (a view
                // of a union list); report rather than guess.
                self.error(
                    "a narrowed union arm read through an optional borrow is not \
                     supported yet on the Rust backend"
                        .to_string(),
                );
                return Some(storage);
            }
            return Some(if n.copy {
                format!("*{storage}.unwrap()")
            } else {
                format!("{storage}.unwrap().clone()")
            });
        }
        self.narrow_unwrap(id.span, storage)
    }

    /// [linear-container] [rs-linear-move] The **moving** form of a narrowed
    /// read, for a linear value being handed over: `first.unwrap()` moves the
    /// payload out of its `Option` where the ordinary path would clone through
    /// a borrow. Only the plain-optional shape (`T?`, which is what every
    /// take-by-move operation answers) — a narrowed *union arm* keeps the
    /// existing accessor path, where a linear payload is a `Clone` handle
    /// today and a non-`Clone` one is a loud rustc error rather than wrong
    /// code.
    fn linear_move_unwrap(&mut self, id: &Ident) -> Option<String> {
        if !self.checked.repr_ty.contains_key(&(self.file_idx, id.span)) {
            return None;
        }
        if matches!(self.bindings.get(id.name.as_str()), Some(BindKind::OptRef)) {
            return None;
        }
        let n = self.narrowing_of(id.span)?;
        if n.arm.is_some() || n.copy || !n.optional {
            return None;
        }
        let storage = self.binding_place(&id.name);
        Some(format!("{storage}.unwrap()"))
    }

    /// The narrowing unwrap for a *projection place* read [flow-place]:    /// `h.field` narrowed by `h.field is T` reads its payload out of the
    /// declared representation, exactly as a narrowed identifier does. The
    /// storage code keeps the base's own unwraps (a narrowed base must be
    /// unwrapped before its field can be reached) but not this place's.
    fn place_unwrap(&mut self, expr: &Expr) -> Option<String> {
        if !matches!(expr, Expr::Field { .. } | Expr::TupleIndex { .. }) {
            return None;
        }
        if !self
            .checked
            .repr_ty
            .contains_key(&(self.file_idx, expr.span()))
        {
            return None;
        }
        let storage = self.place_storage(expr);
        self.narrow_unwrap(expr.span(), storage)
    }

    /// [rs-narrow-mut] `ident_unwrap`'s mutable twin: the place to assign
    /// through or take `&mut` of, already a `&mut T` into the storage.
    fn ident_unwrap_mut(&mut self, id: &Ident) -> Option<String> {
        if !self.checked.repr_ty.contains_key(&(self.file_idx, id.span)) {
            return None;
        }
        let storage = self.binding_place(&id.name);
        self.narrow_unwrap_mut(id.span, storage)
    }

    /// [rs-narrow-mut] `place_unwrap`'s mutable twin. The base is rendered
    /// mutably too, so a chain of narrowed places stays a path into storage
    /// rather than becoming a temporary at any link.
    fn place_unwrap_mut(&mut self, expr: &Expr) -> Option<String> {
        if !matches!(expr, Expr::Field { .. } | Expr::TupleIndex { .. }) {
            return None;
        }
        if !self
            .checked
            .repr_ty
            .contains_key(&(self.file_idx, expr.span()))
        {
            return None;
        }
        let storage = self.place_storage_mut(expr);
        self.narrow_unwrap_mut(expr.span(), storage)
    }

    /// [rs-narrow-mut] `place_storage`'s mutable twin: the same chain with
    /// every *base* narrowing unwrapped mutably. A field cast
    /// [qual-field-override] is likewise unwrapped in place — writing
    /// through a cast field must reach the field, not a copy of it.
    fn place_storage_mut(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Field { base, field, span } => {
                let base_code = self.emit_place_mut(base);
                let code = format!("{base_code}.{}", rs_ident(&field.name));
                if self
                    .checked
                    .field_casts
                    .contains_key(&(self.file_idx, *span))
                {
                    return format!("{code}.as_mut().unwrap()");
                }
                code
            }
            Expr::TupleIndex { base, index, .. } => {
                format!("{}.{index}", self.emit_place_mut(base))
            }
            Expr::Ident(id) if id.name != "None" => self.binding_place(&id.name),
            other => self.emit_place_mut(other),
        }
    }

    /// [rs-narrow-mut] The mutable unwrap of a **narrowed** place, or `None`
    /// when the place carries no narrowing. Shared by the two argument paths
    /// that need to tell those cases apart: a declared call's `&mut` position
    /// ([`Self::borrowed_mut_arg`], which adds its own `&mut` for a plain
    /// name) and an intrinsic's `Mut` parameter (which splices the place as a
    /// method receiver and must not).
    fn narrowed_place_mut(&mut self, expr: &Expr) -> Option<String> {
        let code = match expr {
            Expr::Ident(id) if id.name != "None" => self.ident_unwrap_mut(id),
            Expr::Field { .. } | Expr::TupleIndex { .. } => self.place_unwrap_mut(expr),
            _ => None,
        }?;
        // The unwrap borrows the storage, so the storage has to be mutable.
        self.check_peel_root_mutable(expr)?;
        Some(code)
    }

    /// [rs-narrow-mut] [backend-never-wrong] Reports when a mutable peel would
    /// borrow a place this frame only *reads*, and answers `None` so the caller
    /// does not fall back to the read form — which is the clone that was the
    /// defect.
    ///
    /// The one reachable shape is a **union-typed parameter with a `Mut`
    /// arm**: `fn f(o: Ok Mut List<Int> | Err Str)` renders `o` as `&Union2<…>`,
    /// because an arm's `Mut` is not a claim about `o` itself — the checker
    /// refuses a written `=> o: Mut` for exactly that reason ("a deduction may
    /// preserve or drop qualifiers, not add them"), so nothing can ask for the
    /// `&mut`. Kotlin mutates the caller's value here and Rust cannot, so the
    /// honest answer is a diagnostic until the deduction question is decided
    /// (ROADMAP.md, "Open defects"). Before this it was rustc's E0596 with no
    /// Salvo diagnostic at all.
    fn check_peel_root_mutable(&mut self, expr: &Expr) -> Option<()> {
        let mut root = expr;
        loop {
            match root {
                Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => root = base,
                _ => break,
            }
        }
        let Expr::Ident(id) = root else {
            return Some(());
        };
        if matches!(
            self.bindings.get(id.name.as_str()),
            Some(BindKind::Ref | BindKind::OptRef)
        ) {
            self.error(format!(
                "`{}` is read-only here, so a `Mut` value cannot be mutated \
                 through its union arm: the arm's `Mut` is a claim about the \
                 arm, not about `{}`, so this frame received it borrowed. Take \
                 the payload as its own parameter (`Mut List<T>`) and check the \
                 arm at the call site.",
                id.name, id.name
            ));
            return None;
        }
        Some(())
    }

    /// [rs-narrow-mut] `emit_place`'s mutable twin, for the base of an
    /// assignment target and for an argument in a `&mut` position. The
    /// difference is only at a narrowed link: the read form clones out of
    /// the representation, this one borrows into it.
    fn emit_place_mut(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    return "None".to_string();
                }
                if let Some(unwrapped) = self.ident_unwrap_mut(id) {
                    return unwrapped;
                }
                self.binding_place(&id.name)
            }
            Expr::Field { .. } | Expr::TupleIndex { .. } => {
                if let Some(unwrapped) = self.place_unwrap_mut(expr) {
                    return unwrapped;
                }
                self.place_storage_mut(expr)
            }
            Expr::Index { base, index, .. } => {
                let base_code = self.emit_place_mut(base);
                let idx = self.emit_owned(index);
                format!("{base_code}[({idx}) as usize]")
            }
            other => self.emit_place(other),
        }
    }

    /// A place's storage rendering: the read *without* its own narrowing
    /// unwrap [flow-place]. What an `is` test, an `is` binding, and the
    /// narrowing unwrap itself must read.
    fn place_storage(&mut self, expr: &Expr) -> String {
        // [is-bind-once] A hoisted subject *is* its temporary: the test and the
        // binding both read it, so the subject is evaluated exactly once.
        if let Some(temp) = self.is_temps.get(&(self.file_idx, expr.span())) {
            return temp.clone();
        }
        match expr {
            Expr::Field { base, field, span } => {
                let base_code = self.emit_place(base);
                let code = format!("{base_code}.{}", rs_ident(&field.name));
                if let Some(cast_ty) = self.checked.field_casts.get(&(self.file_idx, *span)) {
                    let cast_ty = cast_ty.clone();
                    return if Self::is_copy_ty(&cast_ty) {
                        format!("{code}.unwrap()")
                    } else {
                        format!("{code}.as_ref().unwrap().clone()")
                    };
                }
                code
            }
            Expr::TupleIndex { base, index, .. } => {
                // [rs-tuple-index] Rust tuples index natively.
                let base_code = self.emit_place(base);
                format!("{base_code}.{index}")
            }
            Expr::Ident(id) if id.name != "None" => self.binding_place(&id.name),
            other => self.emit_place(other),
        }
    }

    /// How a narrowed span reads out of its declared representation
    /// [rs-union-enums] [rs-option]: which arm (if the representation is a
    /// wrapper union), whether an `Option` sits in front of it, and whether
    /// the payload is `Copy`. Computed once and shared by the read unwrap
    /// and the *mutable* one, so the two cannot disagree about arm identity
    /// [union-arm-identity].
    fn narrowing_of(&mut self, span: Span) -> Option<Narrowing> {
        let (repr, logical) = (self.repr_of(span)?, self.ty_of(span)?);
        // Wrapper union narrowed to a single non-`None` arm.
        if repr.is_wrapper_union() && !matches!(logical, Ty::Union(_)) && !logical.is_none_ty() {
            let logical = logical.clone();
            let arm = repr
                .value_arms()
                .iter()
                .position(|a| **a == logical)
                .or_else(|| {
                    // Fall back to subtype matching (e.g. extra quals).
                    let arms: Vec<&Ty> = repr.value_arms();
                    arms.iter()
                        .position(|a| salvo_core::is_subtype(&logical, a))
                });
            let Some(arm) = arm else {
                self.error(format!(
                    "narrowed type `{logical}` matches no arm of `{repr}`"
                ));
                return None;
            };
            return Some(Narrowing {
                arm: Some(arm),
                optional: repr.has_none_arm(),
                copy: Self::is_copy_ty(&logical),
            });
        }
        // `T?` representation narrowed to its value arm: unlike Kotlin,
        // Rust must unwrap the Option physically [rs-option].
        if matches!(repr, Ty::Union(_))
            && repr.has_none_arm()
            && !repr.is_wrapper_union()
            && !logical.has_none_arm()
            && !logical.is_none_ty()
            && !matches!(logical, Ty::Union(_))
        {
            return Some(Narrowing {
                arm: None,
                optional: true,
                copy: Self::is_copy_ty(logical),
            });
        }
        None
    }

    /// Reads a narrowed value out of the declared representation at
    /// `span`, given the code for its storage [rs-union-enums]
    /// [rs-option]. Shared by identifier and projection-place reads. The
    /// result is **owned** — see `narrow_unwrap_mut` for a mutable use.
    fn narrow_unwrap(&mut self, span: Span, storage: String) -> Option<String> {
        let n = self.narrowing_of(span)?;
        let name = storage;
        match n.arm {
            Some(arm) => {
                let access = if n.optional {
                    format!("{name}.as_ref().unwrap()")
                } else {
                    name
                };
                Some(if n.copy {
                    format!("*{access}.u{}()", arm + 1)
                } else {
                    format!("{access}.u{}().clone()", arm + 1)
                })
            }
            None => Some(if n.copy {
                format!("{name}.unwrap()")
            } else {
                format!("{name}.as_ref().unwrap().clone()")
            }),
        }
    }

    /// Reads a narrowed value as a **mutable** place [rs-narrow-mut]: the
    /// same unwrap through the mutable accessors, so the result is a
    /// `&mut T` *into the storage* rather than an owned temporary.
    ///
    /// This exists because the owned form is silently wrong for a mutation:
    /// `&mut (p.as_ref().unwrap().clone())` compiles, and mutates the
    /// clone — a pass driven through a narrowed handle re-emitted its first
    /// element forever, while Kotlin (whose smart cast is the storage
    /// itself) advanced. Both mutable sites go through here: an argument in
    /// a `&mut` position and the base of an assignment target.
    fn narrow_unwrap_mut(&mut self, span: Span, storage: String) -> Option<String> {
        let n = self.narrowing_of(span)?;
        let name = storage;
        // An `Option` in front of the payload is unwrapped mutably first;
        // then a wrapper union's arm accessor, whose `_mut` form the
        // generated union file carries for exactly this purpose.
        let access = if n.optional {
            format!("{name}.as_mut().unwrap()")
        } else {
            name
        };
        Some(match n.arm {
            Some(arm) => format!("{access}.u{}_mut()", arm + 1),
            None => access,
        })
    }

    /// The place expression for a bound name: `self.x` for handler
    /// fields, the (possibly escaped) name otherwise.
    fn binding_place(&self, name: &str) -> String {
        match self.bindings.get(name) {
            Some(BindKind::SelfField) => format!("self.{}", rs_ident(name)),
            // [iter-fn] A slot's default place is the *shared* borrow
            // through its `Option`: correct for every read, and a path that
            // wanted to mutate through it fails to compile rather than
            // mutating a temporary — rustc is the safety net.
            _ => rs_ident(name),
        }
    }

    /// Whether an expression is a *pure place*: a bare identifier of a
    /// bound local/parameter or a field/index chain over one, with no
    /// coercion, narrowing unwrap, or field cast anywhere — i.e. it can
    /// be borrowed directly [rs-borrow-locals].
    fn place_is_pure(&self, expr: &Expr) -> bool {
        if self.coercion_of(expr.span()).is_some() {
            return false;
        }
        match expr {
            Expr::Ident(id) => {
                id.name != "None"
                    && !self.checked.repr_ty.contains_key(&(self.file_idx, id.span))
                    && self.bindings.contains_key(id.name.as_str())
            }
            Expr::Field { base, span, .. } => {
                !self
                    .checked
                    .field_casts
                    .contains_key(&(self.file_idx, *span))
                    // A narrowed projection read is an owned temporary
                    // (the payload out of the wrapper), not a place
                    // [flow-place].
                    && !self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    && self.place_is_pure(base)
            }
            Expr::TupleIndex { base, span, .. } => {
                // A narrowed element read is an owned temporary
                // [flow-place].
                !self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    && self.place_is_pure(base)
            }
            Expr::Index { base, .. } => self.place_is_pure(base),
            _ => false,
        }
    }

    /// A borrow of a pure place [rs-borrow-locals]: `&place` for owned
    /// roots, the bare name for an already-`&` binding, a reborrow for
    /// `&mut` roots. `None` when the value is not a pure place (the
    /// caller falls back to the clone path).
    fn borrow_value(&mut self, value: &Expr) -> Option<String> {
        if !self.place_is_pure(value) {
            return None;
        }
        match value {
            Expr::Ident(id) => match self.bindings.get(id.name.as_str()) {
                Some(BindKind::Ref) => Some(self.binding_place(&id.name)),
                Some(BindKind::RefMut) => Some(format!("&*{}", self.binding_place(&id.name))),
                Some(BindKind::Owned) | Some(BindKind::SelfField) => {
                    Some(format!("&{}", self.binding_place(&id.name)))
                }
                // [rs-opt-borrow] The whole `Option<&T>` is a Copy value,
                // not a place worth borrowing: the clone path handles it.
                Some(BindKind::OptRef) | None => None,
            },
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                Some(format!("&{}", self.emit_place(value)))
            }
            _ => None,
        }
    }

    /// Like `borrow_value`, but tolerant of a coercion at the *top*
    /// span (the caller handles it — e.g. the derived-return `Some`
    /// wrap [readonly-return]); nested places must still be pure.
    fn borrow_place(&mut self, value: &Expr) -> Option<String> {
        match value {
            Expr::Ident(id) => {
                if id.name == "None" || self.checked.repr_ty.contains_key(&(self.file_idx, id.span))
                {
                    return None;
                }
                match self.bindings.get(id.name.as_str()) {
                    Some(BindKind::Ref) => Some(self.binding_place(&id.name)),
                    Some(BindKind::RefMut) => Some(format!("&*{}", self.binding_place(&id.name))),
                    Some(BindKind::Owned) | Some(BindKind::SelfField) => {
                        Some(format!("&{}", self.binding_place(&id.name)))
                    }
                    Some(BindKind::OptRef) | None => None,
                }
            }
            Expr::Field { base, span, .. } => {
                if self
                    .checked
                    .field_casts
                    .contains_key(&(self.file_idx, *span))
                    // A narrowed read is an owned temporary [flow-place].
                    || self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    || !self.place_is_pure(base)
                {
                    return None;
                }
                Some(format!("&{}", self.emit_place(value)))
            }
            Expr::TupleIndex { base, span, .. } => {
                if self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    || !self.place_is_pure(base)
                {
                    return None;
                }
                Some(format!("&{}", self.emit_place(value)))
            }
            Expr::Index { base, .. } => {
                if !self.place_is_pure(base) {
                    return None;
                }
                Some(format!("&{}", self.emit_place(value)))
            }
            _ => None,
        }
    }

    /// The value of a `let`/assignment bind event: a *move-mode* binding
    /// [fate-move-mode] took ownership — the checker consumed the
    /// ancestors at the binding — so the place is rendered directly (a
    /// real move, partial for projections, no clone). Borrow-mode falls
    /// back to the fate-link clone (`emit_linked_value`).
    fn emit_bound_value(&mut self, value: &Expr, bind_span: Span) -> String {
        if self
            .checked
            .binding_modes
            .contains(&(self.file_idx, bind_span))
        {
            let code = self.emit_place(value);
            return self.apply_coercion(value.span(), code);
        }
        self.emit_linked_value(value)
    }

    /// The value of a `let`/assignment/`for` whose source is a bare
    /// identifier: the binding fate-links to the source and the checker
    /// keeps both usable (reads never consume [fate-link]), so an owned
    /// non-Copy local must be *cloned*, not moved. S1 emission is
    /// clone-by-default; borrow emission is roadmap stage S3.
    fn emit_linked_value(&mut self, value: &Expr) -> String {
        if let Expr::Ident(id) = value {
            if self.ident_unwrap(id).is_none()
                && matches!(self.bindings.get(id.name.as_str()), Some(BindKind::Owned))
                && !self.ty_of(id.span).is_some_and(|t| Self::is_copy_ty(t))
            {
                let code = format!("{}.clone()", self.binding_place(&id.name));
                return self.apply_coercion(value.span(), code);
            }
        }
        self.emit_expr(value)
    }

    /// Owned rendering [rs-borrows]: reference-bound identifiers and
    /// field/index reads of non-Copy types clone; owned locals move;
    /// constructed values pass through.
    /// [assert-trap] The trap a failed assertion performs: a panic whose text is
    /// *Salvo's* — `salvo: <what> at <file>:<line>:<col>`, the Salvo source
    /// rather than the generated line, identical to the Kotlin backend's
    /// [kt-assert-trap]. A written message replaces `<what>` and is composed
    /// inside the panic, so it is evaluated only on failure.
    fn trap_call(&mut self, what: &str, message: Option<&Expr>, span: Span) -> String {
        let at = self.salvo_location(span);
        match message {
            Some(m) => {
                let text = self.emit_expr(m);
                format!("panic!(\"salvo: {{}} at {at}\", {text})")
            }
            None => format!("panic!(\"salvo: {what} at {at}\")"),
        }
    }

    /// The Salvo location of a span, for a trap message [assert-trap]:
    /// `<module>:<line>:<col>`.
    ///
    /// The **module path**, not the file name: a file's display name depends on
    /// how it was loaded (the CLI names an embedded std file `std/core/list.sv`,
    /// a directory walk names the same file `core/list.sv`), and a location that
    /// varies by loader would make the emitted output depend on who generated
    /// it. A module path is the language's own identity for a file and is the
    /// same either way.
    fn salvo_location(&self, span: Span) -> String {
        let file = &self.program.files[self.file_idx];
        let (line, col) = salvo_syntax::span::line_col(&file.content, span.start);
        format!("{}:{line}:{col}", file.module)
    }

    fn emit_owned(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    return "None".to_string();
                }
                // [linear-container] [rs-linear-move] A narrowed **linear**
                // value handed over is *moved*, not cloned: the generic path
                // reads `x.as_ref().unwrap().clone()`, which duplicates an
                // obligation — and a reply token is not `Clone` at all, so it
                // would not even compile. Moving out of the `Option` is both
                // legal and the semantics: the checker consumed the variable,
                // so nothing reads it again.
                if self
                    .checked
                    .linear_moves
                    .contains(&(self.file_idx, id.span))
                {
                    if let Some(code) = self.linear_move_unwrap(id) {
                        return code;
                    }
                }
                if let Some(unwrapped) = self.ident_unwrap(id) {
                    return unwrapped;
                }
                let place = self.binding_place(&id.name);
                let copy = self.ty_of(id.span).is_some_and(|t| Self::is_copy_ty(t));
                // [linear-state] [rs-state-take] A state field the checker
                // let the member **move** out of (LC-4: a container of
                // obligations being drained) cannot be moved out of `&mut
                // self` at all in Rust — and must not be cloned, which would
                // duplicate every obligation in it. `mem::take` is both the
                // legal and the honest rendering: the field is empty until the
                // member puts something back, which the checker required.
                if self
                    .checked
                    .state_takes
                    .contains(&(self.file_idx, id.span))
                {
                    return format!("std::mem::take(&mut {place})");
                }
                match self.bindings.get(id.name.as_str()) {
                    Some(BindKind::Ref) | Some(BindKind::RefMut) if !copy => {
                        format!("{place}.clone()")
                    }
                    Some(BindKind::Ref) | Some(BindKind::RefMut) => format!("*{place}"),
                    Some(BindKind::SelfField) if !copy => format!("{place}.clone()"),
                    // [monitor-handler] [rs-monitor] A plain effect's addr is
                    // a lock-wrapper value (`Arc`-backed), not a `Copy` index:
                    // an owned read clones the handle, so a handle bound once
                    // can be shared into any number of spawns and `use`s —
                    // the checker treats every `Addr` as freely reusable, and
                    // this is what keeps that true of the monitor kind.
                    _ if self.ty_of(id.span).is_some_and(|t| self.is_plain_addr_ty(t)) => {
                        format!("{place}.clone()")
                    }
                    _ => place,
                }
            }
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                // A narrowed projection read unwraps to an owned payload
                // [flow-place].
                if let Some(unwrapped) = self.place_unwrap(expr) {
                    return unwrapped;
                }
                let place = self.emit_place(expr);
                // A projection in a moved position whose roots the
                // checker consumed renders as the raw place — a real
                // partial move, no clone [fate-move-mode].
                if self
                    .checked
                    .moved_projections
                    .contains(&(self.file_idx, expr.span()))
                {
                    return place;
                }
                // Field-cast reads are already owned (clone inside).
                if let Expr::Field { span, .. } = expr {
                    if self
                        .checked
                        .field_casts
                        .contains_key(&(self.file_idx, *span))
                    {
                        return place;
                    }
                }
                let needs_clone = self
                    .ty_of(expr.span())
                    .map(|t| !Self::is_copy_ty(t) && ty_is_concrete(t))
                    .unwrap_or(false);
                if needs_clone {
                    format!("{place}.clone()")
                } else {
                    place
                }
            }
            other => self.emit_raw_like(other),
        }
    }

    /// Place rendering: raw chains for idents/fields/indexes (narrowing
    /// unwraps and field casts still apply — their results are owned
    /// temporaries, which method calls and borrows accept).
    fn emit_place(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    return "None".to_string();
                }
                if let Some(unwrapped) = self.ident_unwrap(id) {
                    return unwrapped;
                }
                self.binding_place(&id.name)
            }
            Expr::Field { .. } | Expr::TupleIndex { .. } => {
                // A narrowed projection place reads its payload out of the
                // declared representation [flow-place]; otherwise it is
                // the plain storage read (which applies a field cast
                // [qual-field-override]).
                if let Some(unwrapped) = self.place_unwrap(expr) {
                    return unwrapped;
                }
                self.place_storage(expr)
            }
            Expr::Index { base, index, .. } => {
                let base_code = self.emit_place(base);
                let idx = self.emit_owned(index);
                format!("{base_code}[({idx}) as usize]")
            }
            other => self.emit_raw_like(other),
        }
    }

    /// Raw rendering for assignment targets and `++` operands: no
    /// narrowing unwraps, no casts, no clones.
    ///
    /// [rs-narrow-mut] The *outermost* node keeps that rule — assigning to
    /// a narrowed variable writes its storage, not through the narrowing —
    /// but a **base** must be unwrapped mutably, or `p.at = 2` on a
    /// narrowed `Mut ListYield<Int>?` reaches for a field of the `Option`.
    fn emit_raw(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    "None".to_string()
                } else {
                    self.binding_place(&id.name)
                }
            }
            Expr::Field { base, field, .. } => {
                format!("{}.{}", self.emit_place_mut(base), rs_ident(&field.name))
            }
            Expr::TupleIndex { base, index, .. } => {
                format!("{}.{index}", self.emit_place_mut(base))
            }
            Expr::Index { base, index, .. } => {
                let idx = self.emit_owned(index);
                format!("{}[({idx}) as usize]", self.emit_place_mut(base))
            }
            other => self.emit_raw_like(other),
        }
    }

    /// Every non-place expression form (shared tail of the owned/place/raw
    /// renderings — these all produce owned values).
    fn emit_raw_like(&mut self, expr: &Expr) -> String {
        match expr {
            // [safe-call] [rs-safe-call] A `match` on the `Option`: the member is
            // reached only on `Some`, and the result is `Some(..)`/`None` — so a
            // chain re-tests at each link, as the type says.
            Expr::SafeField { base, inner, span } => {
                let subj = self.place_storage(base);
                // A member whose own type is **already optional** needs no
                // second wrapper: the result type dedupes its `None` arms
                // ([union-arm-identity]), so `p?.zip` on a `Str?` field is a
                // `Str?`, not an `Option<Option<String>>`. Wrapping regardless
                // was an E0308 rustc caught — Kotlin never saw it, having no
                // wrapper to double.
                let already = self
                    .checked
                    .safe_already_optional
                    .contains(&(self.file_idx, *span));
                let body = self.emit_expr(inner);
                if already {
                    format!("if {subj}.is_some() {{ {body} }} else {{ None }}")
                } else {
                    format!("if {subj}.is_some() {{ Some({body}) }} else {{ None }}")
                }
            }
            // [elvis] [rs-elvis] `subject ?: rhs` over the `T?` representation
            // is a `match` on the `Option`: the subject is evaluated **once**,
            // its payload becomes the expression's value, and the right side
            // runs only on `None`. A `match` rather than `unwrap_or_else`
            // precisely because the right side may be an escape — a `return`
            // inside a closure would return from the closure [expr-escape].
            Expr::Elvis {
                subject,
                pick: Some(_),
                rhs,
                span,
            } => {
                // [pick] The arm test the checker recorded, the picked payload
                // read, and the unpicked payload for `_` — all three are the
                // narrowed-read machinery `is` already uses, so the pick adds
                // only the conditional.
                // The subject is evaluated **once**, into a temporary: unlike
                // `?.` it need not be a place, since the picked value and `_`
                // both read out of the temporary. A block expression is what
                // gives it somewhere to live.
                // [is-bind-once] The subject is evaluated **once**, into a
                // temporary registered the way a hoisted `is` subject is — so
                // `place_storage` reaches the temporary from every read inside
                // the form, and the picked value and `_` share one evaluation.
                // Unlike `?.` the subject need not be a place: the temporary is
                // where it lives.
                let hoist = Self::elvis_needs_temp(subject);
                let tmp = if hoist {
                    self.fresh_pick_var()
                } else {
                    self.place_storage(subject)
                };
                let subj_code = if hoist {
                    self.emit_expr(subject)
                } else {
                    String::new()
                };
                let saved_subject = if hoist {
                    self.is_temps
                        .insert((self.file_idx, subject.span()), tmp.clone())
                } else {
                    None
                };
                let test = self.is_test_of(*span).cloned();
                let cond = match &test {
                    Some(t) => self.emit_union_test(&tmp, t),
                    None => "true".to_string(),
                };
                let picked = self.checked.elvis_picks.get(&(self.file_idx, *span)).cloned();
                // [rewrap] A side spanning several arms is a *sub-union*: mapped
                // arm to arm out of the storage, not read as one payload.
                let body = match (
                    self.checked.rewrap_from.get(&(self.file_idx, *span)).cloned(),
                    picked.clone(),
                ) {
                    (Some(from), Some(to)) => {
                        let storage = format!("{}.clone()", self.place_storage(subject));
                        self.emit_rewrap(storage, &from, &to)
                    }
                    _ => self.emit_narrowed_read(subject, picked.as_ref(), test),
                };
                let left = self.checked.pick_left.get(&(self.file_idx, *span)).cloned();
                let else_test = self
                    .checked
                    .pick_else_tests
                    .get(&(self.file_idx, *span))
                    .cloned();
                let else_read = match (
                    self.checked.pick_left_from.get(&(self.file_idx, *span)).cloned(),
                    left.clone(),
                ) {
                    (Some(from), Some(to)) => {
                        let storage = format!("{}.clone()", self.place_storage(subject));
                        self.emit_rewrap(storage, &from, &to)
                    }
                    _ => self.emit_narrowed_read(subject, left.as_ref(), else_test),
                };
                let saved = self.placeholder_code.replace(else_read);
                let r = self.emit_expr(rhs);
                self.placeholder_code = saved;
                if hoist {
                    match saved_subject {
                        Some(prev) => {
                            self.is_temps.insert((self.file_idx, subject.span()), prev);
                        }
                        None => {
                            self.is_temps.remove(&(self.file_idx, subject.span()));
                        }
                    }
                }
                if hoist {
                    format!("{{ let {tmp} = {subj_code}; if {cond} {{ {body} }} else {{ {r} }} }}")
                } else {
                    format!("if {cond} {{ {body} }} else {{ {r} }}")
                }
            }
            Expr::Elvis { subject, rhs, span, .. } => {
                // [elvis] The same shape the qualifier pick uses, and for the
                // same reason: the picked value is **read** out of the subject,
                // not moved out of it. A `match s { Some(v) => v, … }` moves,
                // which broke the moment [elvis-guard] let the subject be read
                // again below (rustc E0382) — every other narrowed read clones.
                // [is-bind-once] The subject is evaluated once, into a
                // temporary, so it need not be a place.
                let hoist = Self::elvis_needs_temp(subject);
                let tmp = if hoist {
                    self.fresh_pick_var()
                } else {
                    self.place_storage(subject)
                };
                let subj_code = if hoist {
                    self.emit_expr(subject)
                } else {
                    String::new()
                };
                let saved_subject = if hoist {
                    self.is_temps
                        .insert((self.file_idx, subject.span()), tmp.clone())
                } else {
                    None
                };
                let test = self.is_test_of(*span).cloned();
                let cond = match &test {
                    Some(t) => self.emit_union_test(&tmp, t),
                    None => format!("{tmp}.is_some()"),
                };
                let picked = self.checked.elvis_picks.get(&(self.file_idx, *span)).cloned();
                // A test matching **every** value arm is a pure `None` test, so
                // the read unwraps the `Option` and *keeps* the wrapper: the
                // picked value is the whole union (`Str | Int`), not one arm of
                // it. Passing the test on would have taken arm 0.
                let read_test = test
                    .clone()
                    .filter(|t| t.arms.len() != t.size);
                let body = self.emit_narrowed_read(subject, picked.as_ref(), read_test);
                let r = self.emit_expr(rhs);
                if hoist {
                    match saved_subject {
                        Some(prev) => {
                            self.is_temps.insert((self.file_idx, subject.span()), prev);
                        }
                        None => {
                            self.is_temps.remove(&(self.file_idx, subject.span()));
                        }
                    }
                }
                if hoist {
                    format!("{{ let {tmp} = {subj_code}; if {cond} {{ {body} }} else {{ {r} }} }}")
                } else {
                    format!("if {cond} {{ {body} }} else {{ {r} }}")
                }
            }
            // [placeholder] The unpicked arm, read out of the subject.
            Expr::Placeholder { .. } => self
                .placeholder_code
                .clone()
                .unwrap_or_else(|| "None".to_string()),
            // [expr-escape] An escape *nested inside* an expression — the
            // form statement position never produces, since `emit_stmt` takes
            // those. Rust has the same three as expressions, so the rendering
            // is direct; what it cannot do from here is run the exit splices
            // an enclosing block owes ([rs-exit-splice] needs statement
            // position), so a nested escape under an owed splice is reported
            // rather than emitted wrong [backend-never-wrong].
            Expr::Return { .. } | Expr::Break { .. } | Expr::Continue { .. } => {
                let value = match expr {
                    Expr::Return { value, .. } | Expr::Break { value, .. } => value.as_deref(),
                    _ => None,
                };
                if !self.exit_splice_code(0, true).is_empty() {
                    self.error(
                        "an early escape inside an expression is not supported \
                         where the enclosing block owes cleanup: bind the value \
                         with `let` first"
                            .to_string(),
                    );
                }
                let word = match expr {
                    Expr::Return { .. } => "return",
                    Expr::Break { .. } => "break",
                    _ => "continue",
                };
                match value {
                    Some(v) => {
                        let code = self.emit_expr(v);
                        format!("{word} {code}")
                    }
                    None => word.to_string(),
                }
            }
            // Literal suffixes emit explicit Rust types [lit-numeric]
            // [type-basic]: `1L` -> `1i64`, `1.2f` -> `1.2f32`. An
            // *unsuffixed* literal renders at its **checked** type
            // [lit-adopt] — `let x: Long = 1` emits `1i64`, `let d: Double
            // = 3` emits `3f64` — and stays bare at its default type, for
            // inference.
            Expr::Int { value, long, span } => {
                let suffix = if *long {
                    "i64"
                } else {
                    match self.ty_of(*span).map(|t| t.strip_quals()) {
                        Some(Ty::Named { name, .. }) if name == "Long" => "i64",
                        Some(Ty::Named { name, .. }) if name == "Double" => "f64",
                        Some(Ty::Named { name, .. }) if name == "Float" => "f32",
                        _ => "",
                    }
                };
                format!("{value}{suffix}")
            }
            Expr::Float { value, single, span } => {
                let s = value.to_string();
                let s = if s.contains('.') { s } else { format!("{s}.0") };
                let suffix = if *single {
                    "f32"
                } else {
                    match self.ty_of(*span).map(|t| t.strip_quals()) {
                        Some(Ty::Named { name, .. }) if name == "Float" => "f32",
                        _ => "",
                    }
                };
                format!("{s}{suffix}")
            }
            Expr::Bool { value, .. } => value.to_string(),
            Expr::Char { value, .. } => format!("'{}'", escape_char(*value)),
            Expr::Str { parts, .. } => self.emit_string(parts),
            Expr::Ident(_) | Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                unreachable!("place expressions are handled by the callers")
            }
            // [fn-overload-at] [fn-value-select] A scope-selected fn *value*
            // (`describe@main`): the selector is erased — the checker
            // recorded which declaration it means — so this renders like any
            // named fn passed by value [fn-contract].
            Expr::Scoped { name, .. } => {
                let span = name.span;
                match self
                    .checked
                    .fn_refs
                    .get(&(self.file_idx, span))
                    .copied()
                    .and_then(|k| self.fn_by_key(k))
                {
                    Some(decl) => self.rust_fn_name(decl),
                    None => {
                        self.error(format!(
                            "internal error: `{}@…` reached the Rust emitter \
                             unresolved (the checker records the declaration a \
                             scope-selected name means)",
                            name.name
                        ));
                        "todo!()".to_string()
                    }
                }
            }
            // [effect-at] Checker-refused as a value — *unless* the capitalized
            // name is a type, which makes this a canonical named as a value
            // ([cmp-canonical] `cmp = cmp@Person`). The two are told apart by
            // what the checker recorded: a resolved `fn_refs` entry means it
            // resolved a **function**, so this renders like any named fn passed
            // by value, selector erased, exactly as `Expr::Scoped` does.
            Expr::EffectScoped { name, .. } => {
                match self
                    .checked
                    .fn_refs
                    .get(&(self.file_idx, name.span))
                    .copied()
                    .and_then(|k| self.fn_by_key(k))
                {
                    Some(decl) => self.rust_fn_name(decl),
                    None => {
                        self.error(format!(
                            "internal error: `{}@Effect` reached the Rust emitter as a \
                             value (the checker refuses member values)",
                            name.name
                        ));
                        "todo!()".to_string()
                    }
                }
            }
            Expr::Call {
                callee,
                type_args,
                args,
                named,
                span,
            } => self.emit_call(callee, type_args, args, named, *span),
            Expr::ArrayLit { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("vec![{}]", items.join(", "))
            }
            // [col-literal] [rs-collections] The brace literals lower to
            // the ordered runtime types, like `set_of`/`map_of`.
            Expr::SetLit { elems, span } => {
                self.needs_collections = true;
                let items: Vec<String> = elems.iter().map(|e| self.emit_owned(e)).collect();
                // [col-literal] An empty `{}` takes its *kind* from the
                // position, and the checker is what resolved it — so the
                // emitter follows the checked type rather than the node:
                // `let m: Map<Str, Int> = {}` is an empty map, and a
                // `List` position an empty list.
                match self.ty_of(*span).map(|t| t.strip_quals()) {
                    Some(Ty::Named { name, .. }) if name == "Map" => {
                        "SalvoMap::from_entries::<HostHash, HostEq, _>(vec![])".to_string()
                    }
                    Some(Ty::Named { name, .. }) if name == "List" => {
                        format!("vec![{}]", items.join(", "))
                    }
                    _ => format!(
                        "SalvoSet::from_elements::<{}, _>(vec![{}])",
                        self.keyed_markers(*span),
                        items.join(", ")
                    ),
                }
            }
            Expr::MapLit { entries, span, .. } => {
                self.needs_collections = true;
                let items: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("({}, {})", self.emit_owned(k), self.emit_owned(v)))
                    .collect();
                format!(
                    "SalvoMap::from_entries::<{}, _>(vec![{}])",
                    self.keyed_markers(*span),
                    items.join(", ")
                )
            }
            Expr::Tuple { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", items.join(", "))
            }
            Expr::StructLit { ty, fields, span } => {
                self.emit_struct_lit(ty.as_ref(), fields, *span)
            }
            Expr::Unary { op, operand, .. } => {
                let inner = self.emit_operand(operand, 6);
                match op {
                    UnaryOp::Neg => format!("-{inner}"),
                    UnaryOp::Not => format!("!{inner}"),
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                // [op-order] [op-equality] A comparison the checker resolved to
                // a `cmp`/`eq` **is** that call: `a < b` is `cmp(a, b) < 0` and
                // `a == b` is `eq(a, b)`. Absent from the table means the host's
                // own operator — numerics, and equality at the other intrinsic
                // types, where the canonical implementation *is* the operator.
                if let Some(via) = self
                    .checked
                    .comparisons
                    .get(&(self.file_idx, *span))
                    .cloned()
                {
                    return self.emit_compare_via(*op, lhs, rhs, &via, *span);
                }
                let prec = bin_prec(*op);
                let l = self.emit_operand_left(lhs, prec);
                let r = self.emit_operand(rhs, prec);
                // [op-promote] A widened operand casts to the promoted
                // type: Rust has no mixed-width operators (`i32 + i64` is
                // E0277), where Kotlin's operator set covers the mixes.
                let l = self.promote_operand(lhs.span(), l);
                let r = self.promote_operand(rhs.span(), r);
                format!("{l} {} {r}", binary_op(*op))
            }
            Expr::Is {
                subject,
                check,
                span,
                ..
            } => self.emit_is_check(subject, check, *span),
            // [qual-lift] The same runtime test as `is`, or `true` when the
            // qualifiers are statically present: qualifiers are erased, so
            // widening is a typing act, not a run-time one.
            Expr::Widen { subject, span, .. } => match self.is_test_of(*span).cloned() {
                Some(test) => {
                    let subj = self.place_storage(subject);
                    self.emit_union_test(&subj, &test)
                }
                None => "true".to_string(),
            },
            Expr::NonNull { operand, span } => {
                // [rs-opt-borrow] `first!` over an `Option<&T>` yields the
                // reference; an *owned* position needs the value — a deref
                // for a Copy scalar [copy-scalar-free], a clone otherwise.
                // [proj-type] Unless the expression's own type *is* the
                // projection (`get(xs, i)!` is a `proj Str`): the borrow is
                // the value — a lambda whose tail returns it yields `&T`,
                // clone-free — and any position that truly needs ownership
                // is checker-refused without `copy` before emission.
                let opt_borrow = matches!(
                    operand.as_ref(),
                    Expr::Ident(id) if matches!(self.bindings.get(id.name.as_str()), Some(BindKind::OptRef))
                ) || self.is_optional_derived_call(operand);
                // [rs-opt-borrow] An owned `Option` held by a **local** is read
                // *through a borrow*, so a second read still can:
                // `p.as_ref().expect(…).clone()`, which is the form a narrowed
                // read of the same value already takes (`narrow_unwrap`). Moving
                // out of it — `p.expect(…)` — is right only where nothing reads
                // it again, and that is exactly the four cases excluded below.
                // Found 2026-09-23: two `!`s on one local were two moves (E0382)
                // while the checker allowed both, since reading an optional is
                // free. A *field* needed nothing: `emit_owned` already clones one.
                let reread_local = match operand.as_ref() {
                    Expr::Ident(id)
                        if !opt_borrow
                            // A narrowed read unwraps by itself, above.
                            && !self.checked.repr_ty.contains_key(&(self.file_idx, id.span))
                            // A value the checker consumed here *must* move: a
                            // clone would duplicate a linear obligation, and a
                            // `Reply<T>` is not `Clone` at all [rs-linear-move].
                            && !self.checked.linear_moves.contains(&(self.file_idx, id.span))
                            && !self.checked.state_takes.contains(&(self.file_idx, id.span))
                            // A Copy payload moves nothing: the `Option` is Copy too.
                            && !self.ty_of(*span).is_some_and(Self::is_copy_ty)
                            // A projection result *is* the borrow [proj-type].
                            && !self.ty_of(*span).is_some_and(|t| t.is_proj()) =>
                    {
                        Some(self.binding_place(&id.name))
                    }
                    _ => None,
                };
                if let Some(place) = reread_local {
                    return format!(
                        "{place}.as_ref().expect(\"salvo: value is absent at {}\").clone()",
                        self.salvo_location(*span)
                    );
                }
                // [assert-trap] [rs-assert-trap] The trap is Salvo's, not
                // `unwrap`'s: the message names the Salvo source and reads the
                // same on both backends (user decision 2026-09-23, A-2).
                let code = format!(
                    "{}.expect(\"salvo: value is absent at {}\")",
                    self.emit_owned(operand),
                    self.salvo_location(*span)
                );
                if opt_borrow {
                    let stays_ref = self.ty_of(*span).is_some_and(|t| t.is_proj());
                    let copy = self.ty_of(*span).is_some_and(|t| Self::is_copy_ty(t));
                    if stays_ref {
                        code
                    } else if copy {
                        format!("*{code}")
                    } else {
                        format!("{code}.clone()")
                    }
                } else {
                    code
                }
            }
            // [assert-fn] The two assertion forms. `assert!` is a check and a
            // `()`; `unreachable!` is the trap itself, typed `!`, so it stands
            // wherever a value is expected.
            Expr::Assert {
                cond,
                message,
                span,
            } => {
                let trap = self.trap_call("assertion failed", message.as_deref(), *span);
                format!("if !({}) {{ {trap} }}", self.emit_expr(cond))
            }
            Expr::Unreachable { message, span } => {
                self.trap_call("unreachable", message.as_deref(), *span)
            }
            Expr::IncDec {
                operand,
                down,
                prefix,
                ..
            } => {
                // [inc-dec] [rs-inc-dec] Rust has no `++`/`--`, so the value is produced
                // by a block: postfix yields the old value, prefix the new.
                let t = self.emit_raw(operand);
                let op = if *down { "-=" } else { "+=" };
                if *prefix {
                    format!("({{ {t} {op} 1; {t} }})")
                } else {
                    format!("({{ let __t = {t}; {t} {op} 1; __t }})")
                }
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                // Captured before emitting: statement emission inside the
                // branches moves `expr_indent`.
                let indent = self.expr_indent;
                self.emit_if_expr(branches, else_block.as_ref(), indent)
            }
            Expr::Lambda { params, body, span } => self.emit_lambda(params, body, *span),
            Expr::Try { body, span } => {
                let indent = self.expr_indent;
                self.emit_try(body, *span, indent)
            }
            Expr::Spread { operand, .. } => self.emit_owned(operand),
            Expr::While { .. } | Expr::For { .. } => self.emit_loop_value(expr),
            Expr::When {
                subject, branches, ..
            } => {
                let indent = self.expr_indent;
                self.emit_when(subject, branches, indent, StmtCtx::Normal, true)
            }
            // [when-condition] / [rs-when-cond]
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                let indent = self.expr_indent;
                self.emit_if_expr(branches, Some(else_block), indent)
            }
            Expr::Error { .. } => "todo!()".to_string(),
            // [actor-spawn-expr] [rs-actor] `spawn H(args) capacity N on P`
            // — construct the handler, wrap it in its generated actor body,
            // and hand it to the scheduler. Its value is the addr.
            Expr::Spawn {
                handler,
                with_items,
                pool,
                span,
            } => self.emit_spawn(handler, with_items, pool.as_deref(), *span),
            // [actor-waitfor] `main`'s bridge, as a block expression: mint a
            // waiter token, run the block that sends it somewhere, then block
            // this thread until the answer arrives.
            Expr::WaitFor {
                binding,
                ty,
                body,
                span,
            } => self.emit_waitfor(binding, ty, body, *span),
            // [actor-self-send] A bare selector outside a call is a checker
            // error; the call form is lowered in `emit_call`.
            Expr::SelfScoped { .. } => {
                self.error("internal: `@self` outside a call reached emission");
                "todo!()".to_string()
            }
            // [actor-replyto] The mint: allocate a slot, park the
            // continuation, hand back the token.
            Expr::ReplyTo {
                member,
                captures,
                gated,
                pool,
                span,
            } => self.emit_replyto(member, captures, *gated, pool.as_deref(), *span),
        }
    }

    /// [actor-replyto] [rs-actor] `replyto k(captures)` → mint a slot on
    /// **this actor**, store `__Cont_E::K(captures)` under it, and evaluate
    /// to the token:
    ///
    /// ```text
    /// { let (__r, __s) = salvo_mint(self.__addr.expect(…));
    ///   self.__parked.insert(__s, __Cont_E::K(caps)); __r }
    /// ```
    ///
    /// `replyto!` differs only in the mint (`salvo_mint_gated`), which is
    /// where the gate lives — the runtime's business, not the emitter's.
    ///
    /// The `expect` cannot fire: a handler that mints may only be spawned
    /// ([actor-replyto], checked), so its members run as activations and
    /// `__addr` was written before the body ran.
    fn emit_replyto(
        &mut self,
        member: &Ident,
        captures: &[Expr],
        gated: bool,
        pool: Option<&Expr>,
        span: Span,
    ) -> String {
        self.needs_scheduler = true;
        // [task-mint] [rs-task] A mint whose target is a free `send fn` needs
        // no continuation enum, no slot and no parked table: the closure *is*
        // the continuation, and the runtime schedules it on the pool the mint
        // chose. That is the whole of the task kernel's emission.
        if let Some(key) = self.checked.replyto_tasks.get(&(self.file_idx, span)).copied() {
            return self.emit_task_mint(member, key, captures, pool);
        }
        let Some(target) = self
            .checked
            .replyto_members
            .get(&(self.file_idx, span))
            .cloned()
        else {
            self.error(format!(
                "internal: no continuation target recorded for `replyto {}`",
                member.name
            ));
            return "todo!()".to_string();
        };
        let Some(cont) = self.current_cont_type() else {
            self.error(format!(
                "internal: `replyto {}` has no continuation enum in scope",
                member.name
            ));
            return "todo!()".to_string();
        };
        let variant = msg_variant_name(&target);
        // A capture is *stored* in the continuation, so it is emitted owned —
        // the same rule a message payload follows.
        let caps: Vec<String> = captures.iter().map(|c| self.emit_owned(c)).collect();
        let built = if caps.is_empty() {
            format!("{cont}::{variant}")
        } else {
            format!("{cont}::{variant}({})", caps.join(", "))
        };
        let mint = if gated {
            "salvo_mint_gated"
        } else {
            "salvo_mint"
        };
        format!(
            "{{ let (__r, __s) = crate::scheduler::{mint}(self.__addr\
             .expect(\"a parking handler runs as an actor\")); \
             self.__parked.insert(__s, {built}); __r }}"
        )
    }

    /// [task-mint] [rs-task] `replyto k(caps) on P` where `k` is a free
    /// `send fn`: `salvo_mint_task(P, Box::new(move |__v| k(caps…, payload)))`.
    ///
    /// The captures are bound to `let`s *outside* the closure, which is what
    /// makes them the values as they were at the mint rather than at the
    /// answer — they are stored in the continuation [deduce-consume], and the
    /// closure is `move` so it owns them. The payload is downcast to the
    /// target's trailing parameter type, the same way the `waitfor` bridge
    /// downcasts what a waiter was sent.
    fn emit_task_mint(
        &mut self,
        member: &Ident,
        key: salvo_core::FnKey,
        captures: &[Expr],
        pool: Option<&Expr>,
    ) -> String {
        let Some(target) = self.fn_by_key(key) else {
            self.error(format!(
                "internal: no declaration for the task target `{}`",
                member.name
            ));
            return "todo!()".to_string();
        };
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        let Some(last) = params.last() else {
            self.error(format!(
                "internal: task target `{}` has no answer parameter",
                member.name
            ));
            return "todo!()".to_string();
        };
        let payload = self.emit_type(&last.ty);
        let name = self.rust_fn_name(target);
        let mut lets = String::new();
        let mut args: Vec<String> = Vec::new();
        for (i, c) in captures.iter().enumerate() {
            let code = self.emit_owned(c);
            lets.push_str(&format!("let __c{i} = {code}; "));
            args.push(format!("__c{i}"));
        }
        args.push(format!(
            "*__v.downcast::<{payload}>().expect(\"the awaited answer\")"
        ));
        // [task-pool-inherit] An omitted `on` clause is the pool current where
        // the mint runs.
        let pool_code = match pool {
            Some(p) => self.emit_owned(p),
            None => "crate::scheduler::salvo_current_pool()".to_string(),
        };
        format!(
            "({{ {lets}crate::scheduler::salvo_mint_task({pool_code}, \
             Box::new(move |__v| {name}({}))) }})",
            args.join(", ")
        )
    }

    /// [actor-self-send] [rs-actor] `k@self(args)` — send to **the actor
    /// the enclosing member belongs to**, and its two readings, chosen at run
    /// time off `__addr` because a handler is compiled once and bound many
    /// ways:
    ///
    /// * `__addr` present — this instance *is* an actor, so the send is an
    ///   enqueue on its own mailbox: `k` becomes its own later activation,
    ///   which is the ordering the form exists to express.
    /// * `__addr` absent — the instance was bound with `use`, so the send is
    ///   the ordinary inline member call a local binding of a send protocol
    ///   already makes.
    ///
    /// A dependent handler's members live in `__Impl_H` and take the fused
    /// value, so the inline reading forwards the one the body already holds
    /// [rs-effect-fusion].
    fn emit_self_send(&mut self, member: &str, args: &[Expr]) -> String {
        self.needs_scheduler = true;
        let Some(handler) = self.current_handler.clone() else {
            self.error("internal: a self-send outside a handler member");
            return "todo!()".to_string();
        };
        let Some(decl) = self.symbols.handlers.get(handler.as_str()).copied() else {
            self.error(format!("internal: no handler `{handler}` for a self-send"));
            return "todo!()".to_string();
        };
        // The message owns its payload either way: it outlives the send when
        // it is queued, and the member's parameters are consumed when it is
        // called [actor-self-send].
        let payload: Vec<String> = args.iter().map(|a| self.emit_owned(a)).collect();
        // [mixed-handler] [rs-mixed] A mixed servant's self-send — a bare
        // sibling call or `k@self(…)` in a send member (user decision
        // 2026-09-19): the message enum is the *handler's* (`__Msg_H`), and
        // the enqueue is unconditional — a mixed handler is spawn-only, so
        // its send members always run as activations and `__addr` was
        // written before the body ran.
        if self.handler_is_mixed(decl) {
            let msg = msg_enum_name(&handler);
            let variant = msg_variant_name(member);
            let built = if payload.is_empty() {
                format!("{msg}::{variant}")
            } else {
                format!("{msg}::{variant}({})", payload.join(", "))
            };
            return format!(
                "crate::scheduler::salvo_send(self.__addr\
                 .expect(\"a mixed servant runs as an actor\"), Box::new({built}))"
            );
        }
        // [effect-handler-multi] The message enum is the *face's*, so the
        // protocol is the one whose members include this one.
        let Some(effect_name) = self
            .handler_faces(decl)
            .into_iter()
            .find(|e| e.fns.iter().any(|f| f.name.name == member))
            .map(|e| e.name.name.clone())
        else {
            self.error(format!(
                "internal: handler `{handler}` has no face declaring `{member}`"
            ));
            return "todo!()".to_string();
        };
        let msg = self.effect_path(&effect_name, &msg_enum_name(&effect_name));
        let variant = msg_variant_name(member);
        let built = if payload.is_empty() {
            format!("{msg}::{variant}")
        } else {
            format!("{msg}::{variant}({})", payload.join(", "))
        };
        // The inline reading: the dependent-member trait when the handler has
        // dependencies, the effect trait otherwise.
        let deps = self.handler_dep_effects(decl);
        let inline = if deps.is_empty() {
            let trait_path = self.effect_path(&effect_name, &rs_ident(&effect_name));
            format!("{trait_path}::{}(self, {})", rs_ident(member), "__PAYLOAD__")
        } else {
            let fx = self
                .effect_env
                .iter()
                .rev()
                .find(|e| !e.is_local)
                .map(|e| e.var.clone())
                .unwrap_or_else(|| "__fx".to_string());
            format!(
                "__Impl_{}::{}(self, &mut *{fx}, {})",
                rs_ident(&handler),
                rs_ident(member),
                "__PAYLOAD__"
            )
        };
        let inline = inline.replace("__PAYLOAD__", &payload.join(", "));
        // A trailing `, )` when the member takes nothing.
        let inline = inline.replace(", )", ")");
        format!(
            "match self.__addr {{ Some(__a) => crate::scheduler::salvo_send(__a, Box::new({built})), \
             None => {inline} }}"
        )
    }

    /// [actor-replyto] The `__Cont_E` path for the handler whose member is
    /// being emitted — the enclosing handler, since a mint is lexical.
    fn current_cont_type(&mut self) -> Option<String> {
        let name = self.current_handler.clone()?;
        let h = self.symbols.handlers.get(name.as_str()).copied()?;
        self.handler_cont_type(h)
    }

    /// A binary/unary operand, parenthesized when its precedence is lower
    /// than the parent operator's (Salvo's parser preserved the grouping;
    /// flat re-rendering must not change it).
    fn emit_operand(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let code = self.emit_expr(expr);
        match expr {
            Expr::Binary { op, .. } if bin_prec(*op) <= parent_prec => format!("({code})"),
            Expr::Is { .. } | Expr::Widen { .. } | Expr::Lambda { .. } => {
                format!("({code})")
            }
            _ => code,
        }
    }

    /// Left operands of the same precedence stay flat (left-assoc).
    fn emit_operand_left(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let code = self.emit_expr(expr);
        match expr {
            Expr::Binary { op, .. } if bin_prec(*op) < parent_prec => format!("({code})"),
            Expr::Is { .. } | Expr::Widen { .. } | Expr::Lambda { .. } => {
                format!("({code})")
            }
            _ => code,
        }
    }

    /// An `is` check in expression position: the checker's tables decide
    /// the lowering [rs-union-enums]; for unchecked `T?` subjects
    /// (struct fields) a null test still works [rs-option]; anything
    /// else is a codegen error [backend-never-wrong].
    fn emit_is_check(&mut self, subject: &Expr, check: &[TypeRef], span: Span) -> String {
        if let Some(test) = self.is_test_of(span).cloned() {
            // The test reads the storage, never the narrowed payload
            // [flow-place].
            let subj = self.place_storage(subject);
            return self.emit_union_test(&subj, &test);
        }
        if let Some(quals) = self
            .checked
            .predicate_tests
            .get(&(self.file_idx, span))
            .cloned()
        {
            return self.emit_predicate_test(subject, &quals);
        }
        // Fallbacks for unchecked subjects with an optional repr (e.g.
        // `person.surname is Str`).
        let subj = self.place_storage(subject);
        if check.len() == 1 && check[0].name.name == "None" {
            return format!("{subj}.is_none()");
        }
        if self
            .ty_of(subject.span())
            .is_some_and(|t| matches!(t, Ty::Union(_)) && t.has_none_arm() && !t.is_wrapper_union())
        {
            return format!("{subj}.is_some()");
        }
        self.error(
            "`is` check could not be lowered for the rust backend \
             (subject is not a checked union or optional)",
        );
        "false".to_string()
    }

    /// The runtime test for an `is` check against a union representation
    /// [rs-union-enums] [union-arm-identity].
    /// [is-bind-once] Whether a `?:` subject needs hoisting into a temporary.
    /// A **place** does not: reading one twice is free, and binding it to a temp
    /// would *move* it, which the guard narrowing then trips over (rustc E0382)
    /// — the subject is still readable below. Anything else is evaluated once,
    /// into the temp.
    fn elvis_needs_temp(expr: &Expr) -> bool {
        !matches!(
            expr,
            Expr::Ident(_) | Expr::Field { .. } | Expr::TupleIndex { .. }
        )
    }

    fn fresh_pick_var(&mut self) -> String {
        self.pick_vars += 1;
        format!("__pick{}", self.pick_vars)
    }

    fn emit_union_test(&mut self, subj: &str, test: &UnionTest) -> String {
        if test.match_none {
            return format!("{subj}.is_none()");
        }
        if test.size == 1 {
            // `T?` representation.
            return if test.arms.is_empty() {
                "false".to_string()
            } else {
                format!("{subj}.is_some()")
            };
        }
        self.union_sizes.insert(test.size);
        if test.arms.is_empty() {
            return "false".to_string();
        }
        if test.arms.len() == test.size {
            return if test.nullable {
                format!("{subj}.is_some()")
            } else {
                "true".to_string()
            };
        }
        let pats: Vec<String> = test
            .arms
            .iter()
            .map(|i| {
                let pat = format!("Union{}::U{}(_)", test.size, i + 1);
                if test.nullable {
                    format!("Some({pat})")
                } else {
                    pat
                }
            })
            .collect();
        format!("matches!({subj}, {})", pats.join(" | "))
    }

    /// A predicate-qualifier `is` check [is-qualifies]: `Q_qualifies`
    /// calls with the subject borrowed (default kept rule [rs-borrows])
    /// and `qualifies` effects threaded [is-qualifies-effects].
    fn emit_predicate_test(&mut self, subject: &Expr, quals: &[String]) -> String {
        // The `qualifies` parameter is `&T` for non-Copy subjects.
        let subject_ty = self.ty_of(subject.span()).cloned();
        let copy = subject_ty.as_ref().is_some_and(Self::is_copy_ty);
        let subj = if copy {
            self.emit_owned(subject)
        } else {
            self.borrowed_arg(subject)
        };
        let mut parts: Vec<String> = Vec::new();
        for q in quals {
            let mut args: Vec<String> = Vec::new();
            // [qual-overload] The subject picks which same-named qualifier is
            // being tested, and so which emitted name to call.
            let decl = self.qualifier_for_subject(q.as_str(), subject_ty.as_ref());
            let mut fn_name = format!("{q}_qualifies");
            if let Some(decl) = decl {
                fn_name = self.qualifier_member_name(decl, "qualifies");
                if let Some(f) = decl.fns.iter().find(|f| f.name.name == "qualifies") {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                            let ty = self.emit_type_ref(r);
                            args.push(self.thread_effect_by_key(&ty));
                        }
                    }
                }
            } else {
                self.error(format!("unknown qualifier `{q}` in predicate check"));
            }
            // [rs-effect-fusion] One fused value covers the whole set.
            if self.fusion {
                args.dedup();
                args.truncate(1);
            }
            args.push(subj.clone());
            parts.push(format!("{fn_name}({})", args.join(", ")));
        }
        if parts.len() == 1 {
            parts.into_iter().next().unwrap()
        } else {
            format!("({})", parts.join(" && "))
        }
    }

    /// An adapter closure for a named fn passed as a fn value
    /// [fn-contract]: `&mut |a0, a1| name(a0, &a1)` — adapter parameters
    /// arrive per the fn-type contract (owned for moved/Copy, references
    /// for kept), and the body forwards per the declaration's actual
    /// modes. `None` when the name is not a known fn.
    fn named_fn_adapter(
        &mut self,
        name: &str,
        span: Span,
        expected: &Type,
        value_mode: ParamMode,
    ) -> Option<String> {
        let key = self.checked.fn_refs.get(&(self.file_idx, span)).copied();
        let decl = match key.and_then(|k| self.fn_by_key(k)) {
            Some(d) => d,
            None => self
                .symbols
                .fns
                .get(name)
                .and_then(|v| v.first())
                .copied()?,
        };
        let Type::Fn {
            params: exp_params,
            param_names,
            deductions,
            effects: exp_effects,
            ..
        } = expected
        else {
            return None;
        };
        let rust_name = self.rust_fn_name(decl);
        // [fn-effects] The adapter takes the *expected* effect parameters —
        // the caller passes them whatever this fn does with them — and
        // forwards the ones the declaration actually needs. A named fn with
        // fewer effects simply ignores the rest (the variance rule).
        // [rs-effect-fusion] Under the fusion the fn type declares **one**
        // `&mut dyn` provider parameter; the adapter rebuilds a Sized fused
        // value from it (the callee's `__Fx` bound needs Sized), covering
        // the expected set — of which the callee's own bounds are a subset.
        let mut effect_params: Vec<String> = Vec::new();
        let mut effect_args: Vec<(String, String)> = Vec::new();
        let mut fx_prelude = String::new();
        let mut fused_forward: Option<String> = None;
        if self.fusion {
            let mut rendered: Vec<String> = Vec::new();
            for eff in exp_effects.iter().flatten() {
                if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                    rendered.push(self.emit_type_ref(r));
                }
            }
            if !rendered.is_empty() {
                let prov = self.fn_value_prov(&rendered);
                let prov_param = self.unique_name("__prov".to_string());
                effect_params.push(format!("{prov_param}: &mut dyn {prov}"));
                let needs_effects = decl.effects.iter().flatten().any(
                    |eff| matches!(eff, EffectRef::Effect(r) | EffectRef::LocalEffect(r) if r.name.name != salvo_core::THROW_EFFECT),
                );
                if needs_effects {
                    self.fusion_id += 1;
                    let combiner = format!(
                        "__FxDyn_{}_{}",
                        sanitize_ident(&self.current_fn),
                        self.fusion_id
                    );
                    let mut impls = String::new();
                    for r in &rendered {
                        let (base, args) = split_rendered_generic(r);
                        let body = format!(
                            "{}::{}(&mut *self.__outer)",
                            has_trait_path(&base, &args),
                            has_getter_name(&base)
                        );
                        impls.push_str(&emit_has_impl(
                            "<'a>",
                            &format!("{combiner}<'a>"),
                            &base,
                            &args,
                            &body,
                        ));
                    }
                    self.generated_items.push(format!(
                        "\npub struct {combiner}<'a> {{\n    __outer: &'a mut dyn {prov},\n}}\n{impls}"
                    ));
                    let var = self.unique_name("__fx".to_string());
                    fx_prelude =
                        format!("let mut {var} = {combiner} {{ __outer: {prov_param} }}; ");
                    fused_forward = Some(format!("&mut {var}"));
                }
            }
        } else {
            for eff in exp_effects.iter().flatten() {
                if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                    let rendered = self.emit_type_ref(r);
                    let var = format!("__fx{}", effect_params.len());
                    effect_params.push(format!("{var}: &mut dyn {rendered}"));
                    effect_args.push((rendered, var));
                }
            }
        }
        let mut forwarded_effects: Vec<String> = Vec::new();
        if let Some(fused) = fused_forward {
            forwarded_effects.push(fused);
        } else if !self.fusion {
            for eff in decl.effects.iter().flatten() {
                if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                    if r.name.name == salvo_core::THROW_EFFECT {
                        continue;
                    }
                    let rendered = self.emit_type_ref(r);
                    match effect_args.iter().find(|(key, _)| *key == rendered) {
                        Some((_, var)) => forwarded_effects.push(format!("&mut *{var}")),
                        None => {
                            // The checker's fits rule makes this unreachable:
                            // a fn value may only perform effects its type
                            // declares [fn-effects] [backend-never-wrong].
                            self.error(format!(
                                "internal error: `{name}` needs effect `{rendered}`, which \
                                 the function type it is passed as does not declare"
                            ));
                        }
                    }
                }
            }
        }
        let names: Vec<String> = (0..decl.params.len()).map(|i| format!("__a{i}")).collect();
        // [yield-proj] A position typed at a retagged element generic hands
        // the adapter `&&T`; peeling one reference at entry restores the
        // `&T` the forwarding below was written for.
        let peels: String = exp_params
            .iter()
            .enumerate()
            .filter(|(_, t)| match t {
                Type::Named { qualifiers, base } => {
                    qualifiers.is_empty()
                        && base.args.is_empty()
                        && self.retagged_generics.contains(&base.name.name)
                }
                _ => false,
            })
            .map(|(i, _)| format!("let __a{i} = *__a{i}; "))
            .collect();
        // [rs-effect-fusion] The combiner binding opens the adapter body,
        // before the peels.
        let peels = format!("{fx_prelude}{peels}");
        let fwd: Vec<String> = decl
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                // Incoming mode per the expected contract; forwarding per
                // the declaration's actual mode. Illegal combinations are
                // checker-rejected before emission [fn-contract].
                let (in_kept, in_mut) =
                    ast_fn_param_contract(exp_params, param_names, deductions, i);
                let in_ref =
                    in_kept && !exp_params.get(i).is_some_and(|t| self.is_copy_ast_type(t));
                let mode = self.param_mode(key, p);
                match (mode, in_ref, in_mut) {
                    (ParamMode::Owned, false, _) => format!("__a{i}"),
                    (ParamMode::Ref, true, false) => format!("__a{i}"),
                    (ParamMode::Ref, true, true) => format!("&*__a{i}"),
                    (ParamMode::Ref, false, _) => format!("&__a{i}"),
                    (ParamMode::RefMut, true, true) => format!("__a{i}"),
                    (ParamMode::RefMut, false, _) => format!("&mut __a{i}"),
                    // Owned decl fed by a reference: checker-rejected
                    // (consuming where keeping); render defensively.
                    (ParamMode::Owned, true, _) => format!("__a{i}.clone()"),
                    (ParamMode::RefMut, true, false) => format!("&mut *__a{i}"),
                }
            })
            .collect();
        let mut all_params = effect_params;
        all_params.extend(names.iter().map(|p| format!("mut {p}")));
        let mut all_args = forwarded_effects;
        all_args.extend(fwd);
        // [rs-iter-pass] An iterator fn's callback arrives owned, so the
        // adapter closure is the value itself rather than a borrow of one.
        let borrow = if value_mode == ParamMode::Owned {
            ""
        } else {
            "&mut "
        };
        Some(if peels.is_empty() {
            format!(
                "{borrow}|{}| {rust_name}({})",
                all_params.join(", "),
                all_args.join(", ")
            )
        } else {
            format!(
                "{borrow}|{}| {{ {peels}{rust_name}({}) }}",
                all_params.join(", "),
                all_args.join(", ")
            )
        })
    }

    /// Renders an argument for a `&T` parameter position [rs-borrows].
    fn borrowed_arg(&mut self, expr: &Expr) -> String {
        if let Expr::Ident(id) = expr {
            // [rs-opt-borrow] A narrowed optional borrow (`let row = get(rows,
            // i)`, then `row is None` ruled out) is `Option<&T>`: in a `&T`
            // position it is the reference itself, not a clone borrowed
            // back.
            if id.name != "None"
                && matches!(self.bindings.get(id.name.as_str()), Some(BindKind::OptRef))
            {
                if let Some(n) = self.narrowing_of(id.span) {
                    if n.arm.is_none() {
                        let storage = self.binding_place(&id.name);
                        return format!("{storage}.unwrap()");
                    }
                }
            }
            if id.name != "None" && self.ident_unwrap(id).is_none() {
                match self.bindings.get(id.name.as_str()) {
                    // Already a reference: pass through (deref coercion
                    // turns `&mut T` into `&T`).
                    Some(BindKind::Ref) | Some(BindKind::RefMut) => {
                        return self.binding_place(&id.name)
                    }
                    Some(BindKind::SelfField) => {
                        return format!("&{}", self.binding_place(&id.name))
                    }
                    _ => return format!("&{}", self.binding_place(&id.name)),
                }
            }
        }
        match expr {
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                format!("&{}", self.emit_place(expr))
            }
            other => {
                let code = self.emit_expr(other);
                format!("&({code})")
            }
        }
    }

    /// Renders an argument for a `&mut T` parameter position [rs-borrows].
    fn borrowed_mut_arg(&mut self, expr: &Expr) -> String {
        // [rs-narrow-mut] A *narrowed* place is reached through the mutable
        // unwrap, which is already a `&mut T` into the storage. Without
        // this the fallback below borrowed the read form — a clone — and
        // the mutation landed on the temporary. The gate is the unwrap
        // itself producing something: a recorded representation is not
        // necessarily one of the two narrowing shapes (a `Mut` drop records
        // one too), and a bare name still needs its `&mut`.
        if let Some(code) = self.narrowed_place_mut(expr) {
            return code;
        }
        if let Expr::Ident(id) = expr {
            if id.name != "None" && self.ident_unwrap(id).is_none() {
                match self.bindings.get(id.name.as_str()) {
                    // Already `&mut`: implicit reborrow at the call.
                    Some(BindKind::RefMut) => return self.binding_place(&id.name),
                    Some(BindKind::SelfField) => {
                        return format!("&mut {}", self.binding_place(&id.name))
                    }
                    _ => return format!("&mut {}", self.binding_place(&id.name)),
                }
            }
        }
        match expr {
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                format!("&mut {}", self.emit_place_mut(expr))
            }
            other => {
                let code = self.emit_expr(other);
                format!("&mut ({code})")
            }
        }
    }
}

impl<'p> Emitter<'p> {
    // ================= coercions =================

    /// Applies a checker-recorded representation change [union-arm-identity]
    /// [rs-union-enums] [rs-option].
    fn apply_coercion(&mut self, span: Span, code: String) -> String {
        let Some(coercion) = self.coercion_of(span) else {
            return code;
        };
        let coercion = coercion.clone();
        self.render_coercion(coercion, code)
    }

    /// One recorded representation change, rendered around `code`.
    fn render_coercion(&mut self, coercion: Coercion, code: String) -> String {
        match coercion {
            // [rs-option] Optionals are physical in Rust.
            Coercion::WrapOption { .. } => format!("Some({code})"),
            // [type-none-unit] The `None` *value* in a `None`-typed slot is
            // the unit value, not an absent optional: the code it was
            // rendered as (`None`) is replaced outright.
            Coercion::NoneUnit => "()".to_string(),
            Coercion::WrapUnion { target, arm, inner } => {
                // [qual-group] The inner wrap of a flattened nested group
                // runs first: the value is physically the bare inner value.
                let code = match inner {
                    Some(inner) => self.render_coercion(*inner, code),
                    None => code,
                };
                self.wrap_union_value(&target, arm, code)
            }
            Coercion::Rewrap { from, to } => self.emit_rewrap(code, &from, &to),
            // [str-drop-mut] Unreachable through `coercion_of`, which
            // unwraps a `Mut` drop because `Mut` erases on this backend.
            Coercion::DropMut { then, .. } => match then {
                Some(inner) => self.render_coercion(*inner, code),
                None => code,
            },
        }
    }

    fn wrap_union_value(&mut self, target: &Ty, arm: usize, code: String) -> String {
        let value_arms = target.value_arms();
        let n = value_arms.len();
        if n < 2 {
            return code;
        }
        self.union_sizes.insert(n);
        let args: Vec<String> = value_arms
            .iter()
            .map(|a| {
                let a = (*a).clone();
                self.rust_ty(&a)
            })
            .collect();
        // [rs-proj-arm] In a derived fn whose union has a `proj` arm the
        // checker's `Ty` has the borrow stripped, so spelling the arguments
        // would say `T` where the enum wants `&T`; rustc infers them from
        // the return type, so they are left out there.
        let wrapped = if self.derived_return_fn && !self.proj_arm_ctors.is_empty() {
            format!("Union{n}::U{}({code})", arm + 1)
        } else {
            format!("Union{n}::<{}>::U{}({code})", args.join(", "), arm + 1)
        };
        if target.has_none_arm() {
            format!("Some({wrapped})")
        } else {
            wrapped
        }
    }

    /// Re-wraps a value between two union representations: a `match`
    /// mapping arms by type equality [union-arm-identity].
    fn emit_rewrap(&mut self, code: String, from: &Ty, to: &Ty) -> String {
        let from_arms: Vec<Ty> = from.value_arms().into_iter().cloned().collect();
        let to_arms: Vec<Ty> = to.value_arms().into_iter().cloned().collect();
        let n = from_arms.len();
        let m = to_arms.len();
        if n >= 2 {
            self.union_sizes.insert(n);
        }
        if m >= 2 {
            self.union_sizes.insert(m);
        }
        let to_args: Vec<String> = to_arms.iter().map(|a| self.rust_ty(a)).collect();
        let to_args = to_args.join(", ");
        let mut branches = String::new();
        for (i, fa) in from_arms.iter().enumerate() {
            // Source arm pattern: enum variant for wrappers, the bare
            // payload for a `T?` source.
            let pat = if n >= 2 {
                format!("Union{n}::U{}(__v)", i + 1)
            } else {
                "__v".to_string()
            };
            let pat = if from.has_none_arm() {
                format!("Some({pat})")
            } else {
                pat
            };
            let result = match to_arms.iter().position(|ta| ta == fa) {
                Some(j) => {
                    let wrapped = if m >= 2 {
                        format!("Union{m}::<{to_args}>::U{}(__v)", j + 1)
                    } else {
                        "__v".to_string()
                    };
                    if to.has_none_arm() {
                        format!("Some({wrapped})")
                    } else {
                        wrapped
                    }
                }
                None => "unreachable!(\"unreachable union arm\")".to_string(),
            };
            branches.push_str(&format!("{pat} => {result}, "));
        }
        if from.has_none_arm() {
            if to.has_none_arm() {
                branches.push_str("None => None, ");
            } else {
                branches.push_str("None => unreachable!(\"unreachable union arm\"), ");
            }
        }
        format!("(match {code} {{ {branches}}})")
    }

    // ================= strings =================

    /// String literals and interpolation [type-str]: plain literals get
    /// `.to_string()`; interpolations lower to `format!` (still-union
    /// values are `Display` via the generated enum impls
    /// [rs-union-enums]).
    fn emit_string(&mut self, parts: &[StrExprPart]) -> String {
        let interps: Vec<&Expr> = parts
            .iter()
            .filter_map(|p| match p {
                StrExprPart::Interp(e) => Some(e.as_ref()),
                _ => None,
            })
            .collect();
        if interps.is_empty() {
            let text: String = parts
                .iter()
                .map(|p| match p {
                    StrExprPart::Text(t) => t.as_str(),
                    _ => "",
                })
                .collect();
            return format!("\"{}\".to_string()", escape_string(&text));
        }
        let mut fmt = String::new();
        let mut args: Vec<String> = Vec::new();
        for part in parts {
            match part {
                StrExprPart::Text(text) => fmt.push_str(&escape_format_text(text)),
                StrExprPart::Interp(expr) => {
                    fmt.push_str("{}");
                    // [interp-to-str] [rs-interp-to-str] A value with no
                    // native text form is rendered by the `to_str` the
                    // checker resolved at this site; scalars and `Str`
                    // format directly.
                    args.push(self.emit_interp_value(expr));
                }
            }
        }
        format!("format!(\"{fmt}\", {})", args.join(", "))
    }

    /// [interp-to-str] One interpolated value: the `to_str` the checker
    /// resolved for it, applied, or the value itself when it renders
    /// natively.
    fn emit_interp_value(&mut self, expr: &Expr) -> String {
        let key = (self.file_idx, expr.span());
        // [interp-struct] A struct with no `to_str` of its own renders
        // field-wise, in the language's format rather than Rust `Debug`'s.
        if let Some(name) = self.checked.interp_struct.get(&key).cloned() {
            if let Some(fields) = self.struct_field_names(&name) {
                let place = self.emit_expr(expr);
                let inner: Vec<String> = fields.iter().map(|f| format!("{f}: {{}}")).collect();
                let args: Vec<String> = fields
                    .iter()
                    .map(|f| format!("{place}.{}", rs_ident(f)))
                    .collect();
                return format!(
                    "format!(\"{name} {{{{ {} }}}}\", {})",
                    inner.join(", "),
                    args.join(", ")
                );
            }
        }
        let Some(fn_key) = self.checked.interp_to_str.get(&key).copied() else {
            return self.emit_expr(expr);
        };
        let Some(decl) = self.fn_by_key(fn_key) else {
            self.error("the `to_str` this interpolation resolved to is not available");
            return self.emit_expr(expr);
        };
        let arg = self.emit_expr(expr);
        if decl.intrinsic {
            // The template takes pre-rendered arguments, which is what lets
            // this stay off the `&'p Expr` path the general call emitter uses.
            let recv = decl.params.first().and_then(|p| type_base_name(&p.ty));
            return match crate::intrinsics::fn_call(
                &decl.name.name,
                recv,
                &[arg.clone()],
                &[],
                crate::intrinsics::Spread::None,
                None,
            ) {
                Some(code) => code,
                None => {
                    self.error(format!(
                        "`to_str` for this type has no lowering on the Rust backend"
                    ));
                    arg
                }
            };
        }
        let name = self.rust_fn_name(decl);
        format!("{name}(&{arg})")
    }

    /// [interp-struct] The field names of a declared struct, in order.
    fn struct_field_names(&self, name: &str) -> Option<Vec<String>> {
        for module in self.program.modules.iter() {
            for item in &module.items {
                if let Item::Struct(decl) = item {
                    if decl.name.name == name {
                        return Some(decl.fields.iter().map(|f| f.name.name.clone()).collect());
                    }
                }
            }
        }
        None
    }

    // ================= value-position control flow =================

    /// An `if`/`elif`/`else` chain in value position: Rust `if` is an
    /// expression; a missing `else` contributes `None` [if-else-none]
    /// (branch values carry their `WrapOption`/wrap coercions). `indent` is
    /// the column of the *statement* the expression sits in — the opening
    /// `if ` is written inline after whatever precedes it, so only the branch
    /// bodies and the closing braces need padding.
    fn emit_if_expr(
        &mut self,
        branches: &[(Expr, Block)],
        else_block: Option<&Block>,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // [is-bind-once] An if *expression* has nowhere to put a statement, so
        // a hoisted subject wraps the whole thing in a block: `({ let __is1 =
        // …; if … })`. Only the first branch can need it — a later one is
        // refused by the checker.
        let hoist = branches
            .first()
            .and_then(|(cond, _)| self.hoistable_is(cond))
            .map(|subject| self.emit_is_temp(subject, indent + 1));
        let mut out = String::new();
        if let Some(temp) = &hoist {
            out.push_str("({\n");
            out.push_str(temp);
            out.push_str(&pad);
        }
        for (i, (cond, block)) in branches.iter().enumerate() {
            let kw = if i == 0 { "if" } else { " else if" };
            let c = self.emit_expr(cond);
            out.push_str(&format!("{kw} {} {{\n", cond_code(c)));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            // [qual-lift]
            let (shadows, saved) = self.emit_widen_shadows(cond, indent + 1);
            out.push_str(&shadows);
            out.push_str(&self.emit_value_block(block, indent + 1));
            self.restore_bindings(saved);
            out.push_str(&format!("{pad}}}"));
        }
        match else_block {
            Some(block) => {
                out.push_str(" else {\n");
                out.push_str(&self.emit_value_block(block, indent + 1));
                out.push_str(&format!("{pad}}}"));
            }
            None => {
                let inner = "    ".repeat(indent + 1);
                out.push_str(&format!(" else {{\n{inner}None\n{pad}}}"));
            }
        }
        // [is-bind-once] Close the block the hoisted subject opened.
        if hoist.is_some() {
            out.push_str(&format!("\n{pad}}})"));
        }
        out
    }

    /// A block in value position: statements plus the trailing expression
    /// as the block's value. `indent` is the column its statements sit at.
    fn emit_value_block(&mut self, block: &Block, indent: usize) -> String {
        let mut out = String::new();
        // [rs-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the block *rewrites* the outer entries to
        // thread through the inner fusion, which dies with the block.
        let saved_env = self.effect_env.clone();
        let splice_floor = self.exit_splices.len();
        // [rs-exit-splice] Splices run *after* the block's value
        // is computed, so a tail with splice code behind it is hoisted
        // into a temporary.
        let mut tail_tmp: Option<String> = None;
        let pad = "    ".repeat(indent);
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    // The tail is the value. `Never`-typed tails
                    // (`return`-like) stay statements.
                    if matches!(self.ty_of(e.span()), Some(Ty::Never)) {
                        out.push_str(&self.emit_expr_stmt(e, indent, StmtCtx::Normal));
                    } else {
                        self.expr_indent = indent;
                        let code = self.emit_expr(e);
                        if self.exit_splices.len() > splice_floor {
                            let tmp = self.fresh_splice_var();
                            out.push_str(&format!("{pad}let {tmp} = {code};\n"));
                            tail_tmp = Some(tmp);
                        } else {
                            out.push_str(&format!("{pad}{code}\n"));
                        }
                    }
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, indent, StmtCtx::Normal));
        }
        out.push_str(&self.splice_exits(splice_floor, indent, true));
        if let Some(tmp) = tail_tmp {
            out.push_str(&format!("{pad}{tmp}\n"));
        }
        self.effect_env = saved_env;
        out
    }

    /// A `when` over a union-typed subject lowers to a `match`
    /// [when-union-subject]; `match` is an expression, so the value form
    /// is free. A wildcard arm covers repr arms the (possibly narrowed)
    /// subject can no longer hold [when-exhaustive].
    fn emit_when(
        &mut self,
        subject: &Expr,
        branches: &[WhenBranch],
        indent: usize,
        ctx: StmtCtx,
        value_pos: bool,
    ) -> String {
        let pad = "    ".repeat(indent);
        // `match` scrutinizes the storage, not a narrowed read
        // [flow-place].
        let subj = self.place_storage(subject);
        let tests: Vec<Option<UnionTest>> = branches
            .iter()
            .map(|b| self.is_test_of(b.span).cloned())
            .collect();
        let Some(size) = tests.iter().flatten().map(|t| t.size).next() else {
            self.error("`when` could not be lowered (subject is not a checked union)");
            return String::new();
        };
        let nullable = tests.iter().flatten().any(|t| t.nullable);
        if size >= 2 {
            self.union_sizes.insert(size);
        }

        let mut out = format!("match {subj} {{\n");
        let mut covered_arms: BTreeSet<usize> = BTreeSet::new();
        let mut covered_none = !nullable;
        for (branch, test) in branches.iter().zip(&tests) {
            let Some(test) = test else { continue };
            let pattern = if test.match_none {
                covered_none = true;
                "None".to_string()
            } else if size == 1 {
                covered_arms.insert(0);
                if nullable {
                    "Some(_)".to_string()
                } else {
                    "_".to_string()
                }
            } else {
                let pats: Vec<String> = test
                    .arms
                    .iter()
                    .map(|i| {
                        covered_arms.insert(*i);
                        let pat = format!("Union{}::U{}(_)", test.size, i + 1);
                        if nullable {
                            format!("Some({pat})")
                        } else {
                            pat
                        }
                    })
                    .collect();
                pats.join(" | ")
            };
            out.push_str(&format!("{pad}    {pattern} => {{\n"));
            if let Some(b) = &branch.binding {
                self.bindings.insert(b.name.clone(), BindKind::Owned);
                let target = self.ty_of(b.span).cloned();
                let read = self.emit_narrowed_read(subject, target.as_ref(), Some(test.clone()));
                if self.narrowed_read_is_ref {
                    self.bindings.insert(b.name.clone(), BindKind::Ref);
                }
                out.push_str(&format!(
                    "{pad}        let mut {} = {read};\n",
                    rs_ident(&b.name)
                ));
            }
            // [qual-lift] A `^` branch head peels the arm it matched: bind
            // the widened value to a shadowing local so the body — and any
            // nested `when` on the subject — sees it at the widened type.
            let mut saved_widen: Vec<(String, Option<BindKind>)> = Vec::new();
            if branch.widen {
                if let Some(target) = self
                    .checked
                    .widen_targets
                    .get(&(self.file_idx, branch.span))
                    .cloned()
                {
                    match subject {
                        Expr::Ident(id) => {
                            let code =
                                self.emit_narrowed_read(subject, Some(&target), Some(test.clone()));
                            let kind = if self.narrowed_read_is_ref {
                                BindKind::Ref
                            } else {
                                BindKind::Owned
                            };
                            let displaced = self.bindings.insert(id.name.clone(), kind);
                            saved_widen.push((id.name.clone(), displaced));
                            out.push_str(&format!(
                                "{pad}        let mut {} = {code};\n",
                                rs_ident(&id.name)
                            ));
                        }
                        other => {
                            let place = self.emit_raw(other);
                            self.error(format!(
                                "`^` on a projection is not supported yet: \
                                 widening materializes a local for the \
                                 branch, which needs a plain variable — bind \
                                 `{place}` to one first"
                            ));
                        }
                    }
                }
            }
            if value_pos {
                out.push_str(&self.emit_value_block(&branch.body, indent + 2));
            } else {
                out.push_str(&self.emit_block_stmts(&branch.body, indent + 2, ctx));
            }
            self.restore_bindings(saved_widen);
            out.push_str(&format!("{pad}    }}\n"));
        }
        // Arms the narrowed subject can no longer hold: the checker
        // proved them impossible [when-exhaustive]; rustc still needs the
        // match to be exhaustive over the repr.
        if covered_arms.len() < size || !covered_none {
            out.push_str(&format!(
                "{pad}    _ => unreachable!(\"unreachable union arm\"),\n"
            ));
        }
        out.push_str(&format!("{pad}}}"));
        out
    }

    /// A fresh `__loopN` local name for loop lowering.
    fn fresh_loop_var(&mut self) -> String {
        self.loop_id += 1;
        format!("__loop{}", self.loop_id)
    }

    fn unique_name(&mut self, base: String) -> String {
        let mut name = base.clone();
        let mut i = 1;
        while !self.taken_names.insert(name.clone()) {
            i += 1;
            name = format!("{base}{i}");
        }
        name
    }

    /// The `for <pat> in ...` binding for a Salvo loop pattern.
    ///
    /// [let-destructure] A **destructuring** pattern binds the element to a
    /// temporary here and leaves the pattern's own bindings to
    /// [`Self::take_loop_destructure`], which the body opens with. Two reasons
    /// for the detour rather than a native Rust pattern: the element may arrive
    /// *borrowed* (a pass hands out projections, `for x in &v` iterates
    /// references), and `mut k` in a pattern opts out of match ergonomics — so
    /// the same pattern that works for an owned element cannot move out of a
    /// shared reference (`E0507`, which is exactly what it used to emit); and
    /// one shape then serves tuples and structs alike.
    fn for_pattern_var(&mut self, pattern: &Pattern, by_ref: bool, body_indent: usize) -> String {
        match pattern {
            Pattern::Ident(id) => {
                // [rs-borrow-locals] A by-reference loop binds `&T`.
                if by_ref {
                    self.bindings.insert(id.name.clone(), BindKind::Ref);
                    return rs_ident(&id.name);
                }
                self.bindings.insert(id.name.clone(), BindKind::Owned);
                format!("mut {}", rs_ident(&id.name))
            }
            Pattern::Tuple { .. } | Pattern::Struct { .. } => {
                let temp = self.unique_name("__elem".to_string());
                self.bindings.insert(temp.clone(), BindKind::Owned);
                let prologue = self.loop_destructure(pattern, &temp, body_indent);
                self.loop_destructures.push(prologue);
                format!("mut {temp}")
            }
        }
    }

    /// [let-destructure] The statements a destructuring loop body opens with:
    /// one binding per name, read off the element temporary.
    ///
    /// Each name is bound **by reference** (`let k = &__elem.0;`, registered as
    /// a [`BindKind::Ref`]), which is what makes one lowering serve an owned
    /// element and a borrowed one: `&` of a field reaches through either, reads
    /// borrow, and a use that needs an owned value clones exactly as it does
    /// for a `&T` parameter. The alternative — binding by value — moves out of
    /// the element, which a borrowed one refuses.
    ///
    /// A name the body **assigns to** is the exception: a reference cannot be
    /// reassigned, and the assignment is to the *binding* rather than to the
    /// element (a loop binding is a local, and Salvo's collections are not
    /// written through one), so it takes an owned copy.
    fn loop_destructure(&mut self, pattern: &Pattern, temp: &str, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        let bind = |me: &mut Self, name: &str, part: String| {
            if me.mutated.contains(name) {
                me.bindings.insert(name.to_string(), BindKind::Owned);
                format!("{pad}let mut {} = {temp}.{part}.clone();\n", rs_ident(name))
            } else {
                me.bindings.insert(name.to_string(), BindKind::Ref);
                format!("{pad}let {} = &{temp}.{part};\n", rs_ident(name))
            }
        };
        match pattern {
            Pattern::Ident(_) => {}
            Pattern::Tuple { elems, .. } => {
                for (i, p) in elems.iter().enumerate() {
                    match p {
                        Pattern::Ident(id) => {
                            let line = bind(self, &id.name, i.to_string());
                            out.push_str(&line);
                        }
                        _ => self.error("nested destructuring patterns are not supported yet"),
                    }
                }
            }
            Pattern::Struct { fields, .. } => {
                for f in fields {
                    let line = bind(self, &f.binding.name, rs_ident(&f.field.name));
                    out.push_str(&line);
                }
            }
        }
        out
    }

    /// [let-destructure] The pending destructuring prologue for the loop whose
    /// header was just emitted, if its pattern needs one.
    fn take_loop_destructure(&mut self) -> String {
        self.loop_destructures.pop().unwrap_or_default()
    }

    /// [iter-for-native] The subject of a *native* `for`, as Rust iterates it.
    /// A `Vec` (a list or an array) is an iterator already; a `Str` is not —
    /// `String` has no `IntoIterator`, so the characters are asked for. The
    /// pass protocol never reaches here: a pass is driven by its own header.
    fn native_for_subject(&mut self, iterable: &Expr, code: String) -> String {
        let base: Option<String> = self.ty_of(iterable.span()).and_then(|t| {
            match t.strip_quals() {
                Ty::Named { name, .. } => Some(name.clone()),
                _ => None,
            }
        });
        match base.as_deref() {
            Some("Str") => format!("{code}.chars()"),
            // [col-map-iter] A map iterates its **keys** — that is what
            // `iter(map)` answers ([`MapKeyYield`]), and a `SalvoMap` is not
            // an iterator at all, so before this the emitted code did not
            // compile (found 2026-09-14 while writing `MemFs`; Kotlin
            // compiled *and* iterated entries, printing `a=1` for `a`).
            // Collected because the keys are borrowed out of the map.
            Some("Map") | Some("SortedMap") => {
                format!("{code}.keys().cloned().collect::<Vec<_>>()")
            }
            _ => code,
        }
    }

    /// Lowers a value-position loop [while-value] [rs-loop-value] to a
    /// block expression with an `Option` result local. When the checked
    /// join type is itself optional, the local *is* the join type (tail
    /// coercions already produce `Option` values); otherwise the local is
    /// `Option<join>` and the block ends `.unwrap()`.
    fn emit_loop_value(&mut self, expr: &Expr) -> String {
        let join = self.ty_of(expr.span()).cloned();
        let (local_ty, join_optional, needs_unwrap) = match &join {
            Some(t) if ty_is_concrete(t) && !t.is_none_ty() && !matches!(t, Ty::Never) => {
                if t.has_none_arm() {
                    (self.rust_ty(t), true, false)
                } else {
                    (format!("Option<{}>", self.rust_ty(t)), false, true)
                }
            }
            _ => {
                self.error(
                    "a value-position loop with an unchecked value type is not \
                     supported by the rust backend",
                );
                ("Option<()>".to_string(), false, false)
            }
        };
        let result = self.fresh_loop_var();
        let ran = format!("{result}_ran");
        let mut out = String::new();
        out.push_str("({\n");
        out.push_str(&format!("let mut {result}: {local_ty} = None;\n"));
        let (cond_or_iter, body, else_block, is_for, pattern) = match expr {
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => (cond.as_ref(), body, else_block, false, None),
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => (iterable.as_ref(), body, else_block, true, Some(pattern)),
            _ => unreachable!("emit_loop_value only receives loops"),
        };
        if else_block.is_some() {
            out.push_str(&format!("let mut {ran} = false;\n"));
        }
        if is_for {
            // [fn-effects] A claiming producer in *value* position would
            // need its `close` spliced into a block that is also producing a
            // value; refused rather than driven without one
            // [backend-never-wrong].
            if self
                .ty_of(cond_or_iter.span())
                .is_some_and(|t| !t.effect_claims().is_empty())
            {
                self.error(
                    "a `for` over a producer that performs effects is not supported in \
                     value position yet: write the loop as a statement",
                );
            }
            // [iter-protocol] A pass is driven; see the statement-position
            // arm. Value-position loops otherwise keep owned iteration (no
            // borrow refinement yet [rs-borrow-locals]).
            if let Some(driver) = self.pass_driver_of(cond_or_iter) {
                // [iter-fn] [backend-never-wrong] An origin's machine
                // has to be closed after the loop, which a value-position
                // loop has nowhere to put — the same cut a claiming producer
                // takes just above.
                if driver.origin {
                    self.error(
                        "a `for` over an origin in value position is not supported yet: \
                         its state machine has to be closed after the loop, so write \
                         the loop as a statement",
                    );
                }
                let header = self.emit_pass_loop_header(driver, pattern.unwrap(), cond_or_iter, 0);
                out.push_str(&header);
            } else {
                let var = self.for_pattern_var(pattern.unwrap(), false, 0);
                let iter = self.emit_expr(cond_or_iter);
                let iter = self.native_for_subject(cond_or_iter, iter);
                out.push_str(&format!("for {var} in {iter} {{\n"));
            }
        } else {
            // [is-bind-once] As in statement position: a non-place subject is
            // evaluated once per iteration, into a shared temporary.
            match self.hoistable_is(cond_or_iter) {
                Some(subject) => {
                    out.push_str("loop {\n");
                    let temp = self.emit_is_temp(subject, 0);
                    out.push_str(&temp);
                    let c = self.emit_expr(cond_or_iter);
                    out.push_str(&format!("if !({}) {{ break; }}\n", cond_code(c)));
                }
                None => {
                    let c = self.emit_expr(cond_or_iter);
                    out.push_str(&format!("while {} {{\n", cond_code(c)));
                }
            }
        }
        if else_block.is_some() {
            out.push_str(&format!("{ran} = true;\n"));
        }
        if !is_for {
            out.push_str(&self.emit_is_bindings(cond_or_iter, 0));
        }
        // [let-destructure] As in statement position: the pattern's bindings
        // open the body.
        if is_for {
            out.push_str(&self.take_loop_destructure());
        }
        self.loop_results.push(Some(result.clone()));
        self.loop_splice_floors.push(self.exit_splices.len());
        out.push_str(&self.emit_loop_body_value(body, &result, join_optional));
        self.loop_splice_floors.pop();
        self.loop_results.pop();
        out.push_str("}\n");
        if let Some(b) = else_block {
            out.push_str(&format!("if !{ran} {{\n"));
            out.push_str(&self.emit_loop_body_value(b, &result, join_optional));
            out.push_str("}\n");
        }
        if needs_unwrap {
            out.push_str(&format!("{result}.unwrap()\n}})"));
        } else {
            out.push_str(&format!("{result}\n}})"));
        }
        out
    }

    /// Whether the enclosing value-loop's join type is optional (its
    /// result local holds coerced values directly, no `Some(...)`).
    fn loop_join_optional(&self, result: &str) -> bool {
        // Recorded at emission time via the naming convention: resolved
        // through `emit_loop_value`'s bookkeeping (see loop_optional map).
        self.loop_optional.get(result).copied().unwrap_or(false)
    }

    /// Assigns a loop-value expression into the result local: wrapped in
    /// `Some(...)` unless the join is optional (tail coercions then
    /// already produce `Option` values) [rs-loop-value].
    fn emit_loop_value_assign(&mut self, value: &Expr, result: &str) -> String {
        let code = self.emit_expr(value);
        if self.loop_join_optional(result) {
            format!("{result} = {code};")
        } else {
            format!("{result} = Some({code});")
        }
    }

    /// A loop body (or `else` block) whose tail expression assigns the
    /// loop's result local [while-value].
    fn emit_loop_body_value(&mut self, block: &Block, result: &str, join_optional: bool) -> String {
        self.loop_optional.insert(result.to_string(), join_optional);
        let mut out = String::new();
        // [rs-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the block *rewrites* the outer entries to
        // thread through the inner fusion, which dies with the block.
        let saved_env = self.effect_env.clone();
        let splice_floor = self.exit_splices.len();
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    out.push_str(&self.emit_tail_assign(e, result, join_optional));
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, 0, StmtCtx::Normal));
        }
        // [rs-exit-splice] The tail already assigned the result local, so
        // splices run after it, like in any other block.
        out.push_str(&self.splice_exits(splice_floor, 0, true));
        self.effect_env = saved_env;
        out
    }

    /// Assigns a block-tail expression to a loop result local.
    /// `Never`-typed tails never fall through; `None`-typed tails
    /// record `None`.
    fn emit_tail_assign(&mut self, e: &Expr, result: &str, join_optional: bool) -> String {
        match self.ty_of(e.span()) {
            Some(Ty::Never) => self.emit_expr_stmt(e, 0, StmtCtx::Normal),
            Some(t) if t.is_none_ty() => {
                let stmt = self.emit_expr_stmt(e, 0, StmtCtx::Normal);
                format!("{stmt}{result} = None;\n")
            }
            _ => {
                let code = self.emit_expr(e);
                if join_optional {
                    format!("{result} = {code};\n")
                } else {
                    format!("{result} = Some({code});\n")
                }
            }
        }
    }

    fn emit_lambda(&mut self, params: &[LambdaParam], body: &LambdaBody, span: Span) -> String {
        // [fn-contract] Parameters the expected contract keeps are
        // reference bindings (mutably for `Mut`); moved (and
        // uncontracted) parameters stay owned.
        let contract = self
            .checked
            .lambda_contracts
            .get(&(self.file_idx, span))
            .cloned();
        // [rs-fn-param-convention] When this lambda is an argument in a
        // fn-typed position, the callee's declared type decides the
        // convention — see `fn_type_param_conventions`.
        let declared_conv = self.pending_lambda_conv.take();
        let retag = self.pending_lambda_retag.take().unwrap_or_default();
        let conv_at = |i: usize| declared_conv.as_ref().and_then(|c| c.get(i)).copied();
        let saved: Vec<(String, Option<BindKind>)> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let kind = match conv_at(i) {
                    Some(k) => k,
                    None => match contract.as_ref().and_then(|c| c.get(i)) {
                        Some(e) if e.kept && e.mutable => BindKind::RefMut,
                        Some(e)
                            if e.kept
                                && !p.ty.as_ref().is_some_and(|t| self.is_copy_ast_type(t)) =>
                        {
                            BindKind::Ref
                        }
                        _ => BindKind::Owned,
                    },
                };
                let old = self.bindings.insert(p.name.name.clone(), kind);
                (p.name.name.clone(), old)
            })
            .collect();
        // [fn-effects] The effects a call of this value performs arrive as
        // *leading parameters*, not captures: that is what keeps the closure
        // from holding a borrow across an effectful call [rs-effect-fusion].
        let lambda_effects: Vec<Ty> = self
            .checked
            .lambda_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        let saved_effect_env = self.effect_env.clone();
        let mut param_list: Vec<String> = Vec::new();
        // [rs-effect-fusion] Under the fusion the value's type declares one
        // `&mut dyn` provider parameter; the body opens by wrapping it in a
        // Sized combiner implementing the Has traits (UFCS through the
        // provider — supertrait elaboration), so everything inside threads
        // the combiner exactly like any fused value.
        let mut fx_prelude = String::new();
        if self.fusion && !lambda_effects.is_empty() {
            let rendered: Vec<String> = lambda_effects.iter().map(|ty| self.rust_ty(ty)).collect();
            let prov = self.fn_value_prov(&rendered);
            let prov_param = self.unique_name("__prov".to_string());
            param_list.push(format!("{prov_param}: &mut dyn {prov}"));
            self.bindings.insert(prov_param.clone(), BindKind::RefMut);
            self.fusion_id += 1;
            let combiner = format!(
                "__FxDyn_{}_{}",
                sanitize_ident(&self.current_fn),
                self.fusion_id
            );
            let mut impls = String::new();
            for r in &rendered {
                let (base, args) = split_rendered_generic(r);
                let body = format!(
                    "{}::{}(&mut *self.__outer)",
                    has_trait_path(&base, &args),
                    has_getter_name(&base)
                );
                impls.push_str(&emit_has_impl(
                    "<'a>",
                    &format!("{combiner}<'a>"),
                    &base,
                    &args,
                    &body,
                ));
            }
            self.generated_items.push(format!(
                "\npub struct {combiner}<'a> {{\n    __outer: &'a mut dyn {prov},\n}}\n{impls}"
            ));
            let var = self.unique_name("__fx".to_string());
            fx_prelude = format!("let mut {var} = {combiner} {{ __outer: {prov_param} }}; ");
            for (ty, r) in lambda_effects.iter().zip(rendered) {
                self.effect_env.push(EffectEntry {
                    ty: Some(ty.clone()),
                    key: r,
                    var: var.clone(),
                    is_local: true,
                    handle_var: None,
                });
            }
            self.bindings.insert(var, BindKind::Owned);
        } else {
            for ty in &lambda_effects {
                let rendered = self.rust_ty(ty);
                let var = self.unique_name(effect_param_name(&rendered));
                self.effect_env.push(EffectEntry {
                    ty: Some(ty.clone()),
                    key: rendered.clone(),
                    var: var.clone(),
                    is_local: false,
                    handle_var: None,
                });
                self.bindings.insert(var.clone(), BindKind::RefMut);
                param_list.push(format!("{var}: &mut dyn {rendered}"));
            }
        }
        param_list.extend(params.iter().enumerate().map(|(i, p)| match &p.ty {
            Some(t) => {
                let ty = self.emit_type(t);
                // [fn-contract] Annotations match the binding mode.
                let ty = match conv_at(i) {
                    Some(BindKind::RefMut) => format!("&mut {ty}"),
                    Some(BindKind::Ref) => format!("&{ty}"),
                    Some(_) => ty,
                    None => match contract.as_ref().and_then(|c| c.get(i)) {
                        Some(e) if e.kept && e.mutable => format!("&mut {ty}"),
                        Some(e) if e.kept && !self.is_copy_ast_type(t) => {
                            format!("&{ty}")
                        }
                        _ => ty,
                    },
                };
                // [yield-proj] A retagged element arrives one reference
                // deeper than the annotation says (the peel below restores
                // it); the written type follows.
                let ty = if retag.get(i).copied().unwrap_or(false) {
                    format!("&{ty}")
                } else {
                    ty
                };
                format!("{}: {ty}", rs_ident(&p.name.name))
            }
            None => rs_ident(&p.name.name),
        }));
        // [rs-fn-field] A **stored** callback outlives the call, so it owns its
        // captures: `move`. And it owns a *copy* of each — the clone is what lets
        // two stored callbacks read the same local, which is legal Salvo (an
        // immutable read) and would otherwise be E0382 at the second `move`.
        // The clones are shadowing `let`s in a block around the closure, so the
        // body needs no rewriting.
        let mv = if std::mem::take(&mut self.pending_lambda_move) {
            "move "
        } else {
            ""
        };
        let capture_clones: String = if mv.is_empty() {
            String::new()
        } else {
            self.checked
                .lambda_captures
                .get(&(self.file_idx, span))
                .map(|caps| {
                    caps.iter()
                        .filter(|c| {
                            matches!(self.bindings.get(c.name.as_str()), Some(BindKind::Owned))
                        })
                        .map(|c| {
                            let name = rs_ident(&c.name);
                            format!("let mut {name} = {name}.clone(); ")
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default()
        };
        // [yield-proj] A parameter typed at a retagged element generic
        // arrives as `&&T`; one `*` restores the `&T` the body was emitted
        // for (the peel rebinds the name, which is what makes it uniform).
        let peels: String = params
            .iter()
            .enumerate()
            .filter(|(i, _)| retag.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| {
                let name = rs_ident(&p.name.name);
                format!("let {name} = *{name}; ")
            })
            .collect();
        // [rs-effect-fusion] The fused-combiner binding opens the body,
        // before the peels, exactly like a dyn-boundary fn's prelude.
        let peels = format!("{fx_prelude}{peels}");
        let out = match body {
            LambdaBody::Expr(expr) => {
                let expr_code = self.emit_expr(expr);
                if peels.is_empty() {
                    format!("{mv}|{}| {expr_code}", param_list.join(", "))
                } else {
                    format!("{mv}|{}| {{ {peels}{expr_code} }}", param_list.join(", "))
                }
            }
            LambdaBody::Block(block) => {
                // Closures return their last expression; a trailing
                // `return X` becomes the value.
                let mut out = format!("{mv}|{}| {{\n", param_list.join(", "));
                if !peels.is_empty() {
                    out.push_str(&format!("    {peels}\n"));
                }
                // [rs-exit-splice] A closure is a function boundary: its
                // `return` runs only the splices registered inside it.
                let saved_floor = self.splice_floor;
                self.splice_floor = self.exit_splices.len();
                let saved_loop_floors = std::mem::take(&mut self.loop_splice_floors);
                let body_floor = self.exit_splices.len();
                let n = block.stmts.len();
                for (i, stmt) in block.stmts.iter().enumerate() {
                    if i + 1 == n {
                        if let Stmt::Expr(Expr::Return { value: Some(v), .. }) = stmt {
                            let code = self.emit_expr(v);
                            let splices = self.splice_exits(body_floor, 1, true);
                            if splices.is_empty() {
                                out.push_str(&format!("    {code}\n"));
                            } else {
                                let tmp = self.fresh_splice_var();
                                out.push_str(&format!("    let {tmp} = {code};\n"));
                                out.push_str(&splices);
                                out.push_str(&format!("    {tmp}\n"));
                            }
                            continue;
                        }
                    }
                    if matches!(stmt, Stmt::Expr(Expr::Return { .. })) {
                        self.error(
                            "early `return` inside a lambda is not supported yet \
                             (only as the final statement)",
                        );
                        continue;
                    }
                    out.push_str(&self.emit_stmt(stmt, 1, StmtCtx::Normal));
                }
                out.push_str(&self.splice_exits(body_floor, 1, true));
                self.loop_splice_floors = saved_loop_floors;
                self.splice_floor = saved_floor;
                out.push('}');
                out
            }
        };
        self.effect_env = saved_effect_env;
        for (name, old) in saved {
            match old {
                Some(kind) => self.bindings.insert(name, kind),
                None => self.bindings.remove(&name),
            };
        }
        if capture_clones.is_empty() {
            return out;
        }
        format!("{{ {capture_clones}{out} }}")
    }

    /// A struct literal [struct-decl] [struct-defaults] [struct-spread]:
    /// omitted fields inline their declared defaults; a spread becomes
    /// Rust functional-update syntax with a cloned base.
    fn emit_struct_lit(
        &mut self,
        ty: Option<&Type>,
        fields: &[StructLitField],
        _span: Span,
    ) -> String {
        let type_name = match ty {
            Some(Type::Named { base, .. }) => Some(base.name.name.clone()),
            Some(other) => Some(format!("{:?}", other.span())),
            None => None,
        };
        let spreads: Vec<&Expr> = fields
            .iter()
            .filter_map(|f| match &f.kind {
                StructLitFieldKind::Spread(e) => Some(e),
                _ => None,
            })
            .collect();
        // [rs-proj-struct] The `proj` fields of a borrowing struct take a
        // borrow of the stored value, not a move or clone of it.
        let proj_fields: Vec<String> = type_name
            .as_deref()
            .filter(|n| self.borrowing_structs.contains(*n))
            .and_then(|n| self.symbols.structs.get(n).copied())
            .map(|sd| {
                sd.fields
                    .iter()
                    .filter(|f| type_has_proj(&f.ty))
                    .map(|f| f.name.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let named: Vec<(String, String)> = fields
            .iter()
            .filter_map(|f| match &f.kind {
                StructLitFieldKind::Named { name, value } if proj_fields.contains(&name.name) => {
                    let code = self.borrow_arg_for_arm(value);
                    Some((rs_ident(&name.name), code))
                }
                StructLitFieldKind::Named { name, value } => {
                    Some((rs_ident(&name.name), self.emit_expr(value)))
                }
                _ => None,
            })
            .collect();
        let mut named_args: Vec<String> = named.iter().map(|(n, v)| format!("{n}: {v}")).collect();

        let Some(name) = type_name else {
            self.error("struct literals without a type annotation are not supported here");
            return "todo!()".to_string();
        };
        // [rs-fn-field] A fn-typed field is held as `Rc<dyn Fn…>`, so the
        // store wraps: `Rc::new` of an `impl Fn` value, and of an `Rc` one
        // too (it re-coerces, at one more indirection) — which keeps the
        // rendering the same whatever the value came from.
        if let Some(decl) = self.symbols.structs.get(name.as_str()).copied() {
            let fn_fields: HashSet<String> = decl
                .fields
                .iter()
                .filter(|f| matches!(f.ty, Type::Fn { .. }))
                .map(|f| rs_ident(&f.name.name))
                .collect();
            if !fn_fields.is_empty() {
                named_args = named
                    .iter()
                    .map(|(n, v)| {
                        if fn_fields.contains(n) {
                            format!("{n}: std::rc::Rc::new({v})")
                        } else {
                            format!("{n}: {v}")
                        }
                    })
                    .collect();
            }
        }
        match spreads.len() {
            0 => {
                // Inline declared defaults for omitted fields
                // [struct-defaults] (Rust has no default arguments).
                if let Some(decl) = self.symbols.structs.get(name.as_str()).copied() {
                    let provided: HashSet<String> = fields
                        .iter()
                        .filter_map(|f| match &f.kind {
                            StructLitFieldKind::Named { name, .. } => Some(rs_ident(&name.name)),
                            _ => None,
                        })
                        .collect();
                    for df in &decl.fields {
                        let fname = rs_ident(&df.name.name);
                        if provided.contains(&fname) {
                            continue;
                        }
                        if let Some(default) = &df.default {
                            let default = default.clone();
                            let code = self.emit_default_value(&default, &df.ty);
                            let code = if matches!(df.ty, Type::Fn { .. }) {
                                // [rs-fn-field] Same wrap as a written store.
                                format!("std::rc::Rc::new({code})")
                            } else {
                                code
                            };
                            named_args.push(format!("{fname}: {code}"));
                        }
                    }
                }
                format!("{} {{ {} }}", rs_ident(&name), named_args.join(", "))
            }
            1 => {
                // `P {...base, f: v}` -> `P { f: v, ..base.clone() }`
                // [struct-spread]. The checker consumes the spread base
                // [deduce-consume], so the clone-vs-move choice is
                // unobservable (a real move is a deferred perf
                // refinement); the clone also keeps unchecked contexts
                // safe.
                let base = match spreads[0] {
                    e @ (Expr::Ident(_)
                    | Expr::Field { .. }
                    | Expr::TupleIndex { .. }
                    | Expr::Index { .. }) => {
                        format!("{}.clone()", self.emit_place(e))
                    }
                    other => self.emit_expr(other),
                };
                format!(
                    "{} {{ {}..{base} }}",
                    rs_ident(&name),
                    if named_args.is_empty() {
                        String::new()
                    } else {
                        format!("{}, ", named_args.join(", "))
                    }
                )
            }
            _ => {
                self.error("struct literals with multiple spreads are not supported yet");
                "todo!()".to_string()
            }
        }
    }

    /// A struct-field default expression inlined at a literal site
    /// [struct-defaults]: emitted without checker tables (the spans
    /// belong to the struct's file), so the `T?` wrap is applied
    /// syntactically.
    fn emit_default_value(&mut self, default: &Expr, field_ty: &Type) -> String {
        let code = self.emit_expr(default);
        let optional_field = matches!(field_ty, Type::Nullable { .. })
            || matches!(field_ty, Type::Union { arms, .. } if arms.iter().any(is_none_type));
        if optional_field && !matches!(default, Expr::Ident(id) if id.name == "None") {
            format!("Some({code})")
        } else {
            code
        }
    }
}

impl<'p> Emitter<'p> {
    // ================= calls =================

    fn emit_call(
        &mut self,
        callee: &Expr,
        type_args: &[Type],
        args: &[Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        // [actor-use-addr] [rs-actor] A send to an actor: build the
        // protocol's message and enqueue it on the target's mailbox. The
        // receiver names *where* it goes, not an argument.
        if let Some(effect) = self.checked.addr_calls.get(&(self.file_idx, span)).cloned() {
            return self.emit_addr_send(&effect, callee, args, span);
        }
        // [mixed-handler] [rs-mixed] A façade send: a sync member of a mixed
        // handler calling one of the handler's own `send fn` members — an
        // enqueue on the servant's mailbox through the façade's own addr.
        if let Some(member) = self
            .checked
            .facade_sends
            .get(&(self.file_idx, span))
            .cloned()
        {
            return self.emit_facade_send(&member, args);
        }
        // [actor-self-send] `k@self(args)` — a message to the actor this
        // member belongs to.
        if let Some(member) = self
            .checked
            .self_sends
            .get(&(self.file_idx, span))
            .cloned()
        {
            return self.emit_self_send(&member, args);
        }
        // [throw] [rs-throw-controlflow] A call that may throw is not an
        // ordinary call: `throw` itself *is* the control transfer, and a
        // call that propagates one unwraps its `ControlFlow`.
        if let Some(site) = self.checked.may_throw.get(&(self.file_idx, span)).cloned() {
            if site.performs {
                return self.emit_throw_call(&site, args, self.expr_indent);
            }
            let call = self.emit_call_inner(callee, type_args, args, named, span);
            return self.wrap_may_throw_call(&site, call, self.expr_indent);
        }
        self.emit_call_inner(callee, type_args, args, named, span)
    }

    fn emit_call_inner(
        &mut self,
        callee: &Expr,
        type_args: &[Type],
        args: &[Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        // Normalize dot-notation [fn-dot].
        if let Expr::Field { base, field, .. } = callee {
            let name = field.name.as_str();
            let total = args.len() + 1;
            if self.symbols.effect_of_fn.contains_key(name)
                || self.symbols.resolve_fn(name, total).is_some()
            {
                let mut all_args: Vec<&Expr> = Vec::with_capacity(total);
                all_args.push(base);
                all_args.extend(args.iter());
                return self.emit_resolved_call(name, type_args, &all_args, named, span);
            }
            // [call-resolve] The checker rejects an undeclared dot-call, so
            // reaching here means a resolution table lost an entry without
            // reporting it — a compiler bug. Say so instead of inventing a
            // Rust method call: emitted code must never be a guess
            // [backend-never-wrong].
            self.error(format!(
                "internal error: dot-call `{name}` reached the Rust emitter \
                 unresolved (the checker should have rejected it, or resolved \
                 it to a fn or effect member)"
            ));
            "todo!()".to_string()
        } else if let Expr::Ident(id) = callee {
            let arg_refs: Vec<&Expr> = args.iter().collect();
            self.emit_resolved_call(&id.name, type_args, &arg_refs, named, span)
        } else if let Expr::Scoped { base, name, .. } = callee {
            // [fn-overload-at] The scope selector narrowed *which*
            // declaration the checker resolved, which `call_fn` already
            // records; emission is the ordinary call, with the receiver
            // folded in for the dot form [fn-dot].
            let mut all_args: Vec<&Expr> = Vec::with_capacity(args.len() + 1);
            if let Some(base) = base {
                all_args.push(base);
            }
            all_args.extend(args.iter());
            self.emit_resolved_call(&name.name, type_args, &all_args, named, span)
        } else if let Expr::EffectScoped { base, name, .. } = callee {
            // [effect-at] The effect selector narrowed *which* effect the
            // checker resolved, which `effect_calls` already records at
            // this span; emission is the ordinary member call, with the
            // receiver folded in for the dot form [fn-dot].
            let mut all_args: Vec<&Expr> = Vec::with_capacity(args.len() + 1);
            if let Some(base) = base {
                all_args.push(base);
            }
            all_args.extend(args.iter());
            self.emit_resolved_call(&name.name, type_args, &all_args, named, span)
        } else {
            // Calling a computed value (lambda etc.): owned args [fn-lambda],
            // with its effects threaded first [fn-effects].
            let callee_code = self.emit_owned(callee);
            let mut arg_code: Vec<String> = self.fn_value_effect_args(span);
            arg_code.extend(args.iter().map(|a| self.emit_expr(a)));
            format!("{callee_code}({})", arg_code.join(", "))
        }
    }

    fn emit_resolved_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[&Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        // 1. Effect member call: dispatch through the handler in scope
        // [rs-effects] ([effect-disambiguation], `effect_calls`). Under the
        // fusion one value implements every effect in scope, so dispatch is
        // UFCS: plain method syntax would be ambiguous between two effects
        // with a same-named member, and between two instances of a generic
        // effect [rs-effect-fusion].
        // [call-resolve] ...unless the checker resolved this callee to a
        // fn-typed **local**, which outranks any same-named declaration.
        // `effect_of_fn` is program-wide and knows nothing of scopes, so
        // without this a user effect member could hijack a std function's
        // own parameter (`filter`'s `keep`).
        if let Some(owners) = self
            .symbols
            .effect_of_fn
            .get(name)
            .filter(|_| {
                !self.checked.local_calls.contains(&(self.file_idx, span))
                    // [effect-available] ...or to an ordinary fn, because no
                    // instance of the owning effect was in scope: the name is
                    // a member somewhere, this call is not.
                    && !self
                        .checked
                        .fn_over_member_calls
                        .contains(&(self.file_idx, span))
            })
        {
            let owners = owners.clone();
            let checked_effect = self
                .checked
                .effect_calls
                .get(&(self.file_idx, span))
                .cloned();
            // [effect-member-overload] Several effects may declare the
            // member: the checker's per-call resolution names the owner;
            // with a sole owner the map answers directly (the unchecked
            // fallback path).
            let effect: &str = match &checked_effect {
                Some(Ty::Named { name: n, .. }) => owners
                    .iter()
                    .copied()
                    .find(|o| *o == n.as_str())
                    .unwrap_or(owners[0]),
                _ if owners.len() == 1 => owners[0],
                _ => {
                    self.error(format!(
                        "internal: `{name}` is a member of several effects and \
                         the checker recorded no resolution for this call"
                    ));
                    owners[0]
                }
            };
            let handler = match &checked_effect {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    self.member_dispatch_by_ty(&ty)
                }
                _ => self.member_dispatch_fallback(effect, type_args),
            };
            // Effect member params: default kept rule [rs-borrows].
            // [effect-member-overload] The *resolved* overload's parameters,
            // since two overloads differ in exactly what they take.
            let member = self.symbols.effects.get(effect).and_then(|e| {
                match self.checked.effect_member_calls.get(&(self.file_idx, span)) {
                    Some(&idx) => e.fns.get(idx),
                    None => e.fns.iter().find(|f| f.name.name == name),
                }
            });
            let mut arg_code = match member {
                Some(m) => {
                    let m = m.clone();
                    self.emit_args_for_params_of(&m.params, args, None, Some(&m))
                }
                None => args.iter().map(|a| self.emit_expr(a)).collect(),
            };
            // [implicit-param] A member's implicit parameters are part of its
            // signature, so they arrive as trailing arguments here exactly as
            // for a plain fn call [implicit-resolve].
            arg_code.extend(self.emit_implicit_args(named, span));
            let called = self.called_member_name(effect, name, span);
            if !self.fusion {
                return format!("{handler}.{called}({})", arg_code.join(", "));
            }
            // [rs-effect-fusion] Accessor-then-method: the fused value
            // implements `__Has_E` per effect, never the effects
            // themselves, so the member call goes through the accessor —
            // `__Has_Random::<i32>::__get_Random(recv).next_random(…)`.
            // The UFCS turbofish on the *Has* trait is what disambiguates
            // two instances of a generic effect; the member call itself is
            // on `&mut dyn E<…>`, which is never ambiguous.
            let (base, eff_args) = match &checked_effect {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    self.ty_effect_parts(&ty)
                }
                _ => {
                    let rendered = self.member_dispatch_key(effect, type_args);
                    split_rendered_generic(&rendered)
                }
            };
            let accessor = format!(
                "{}::{}({handler})",
                has_trait_path(&base, &eff_args),
                has_getter_name(&base)
            );
            let (prelude, arg_code) = self.hoist_effect_args(Some(&[handler.clone()]), arg_code);
            return Self::wrap_hoisted(
                &prelude,
                format!("{accessor}.{called}({})", arg_code.join(", ")),
            );
        }

        // [implicit-param] An implicit parameter shadows the fns of the same
        // name inside the body: it *is* one of them, chosen by the caller.
        if self.implicits.iter().any(|i| i.name == name) {
            // A kept `Mut` position of the implicit's fn type is a `&mut`
            // borrow; everything else is by value [fn-contract].
            let modes: Vec<bool> = self
                .implicits
                .iter()
                .find(|i| i.name == name)
                .map(|i| match i.ty.strip_quals() {
                    Ty::Fn {
                        params, contract, ..
                    } => (0..params.len())
                        .map(|k| {
                            contract
                                .as_ref()
                                .and_then(|c| c.get(k))
                                .is_some_and(|e| e.kept && e.mutable)
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .unwrap_or_default();
            // [rs-fn-param-convention] A kept non-`Mut` position is `&T`
            // unless its type is a Copy scalar — the same convention a written
            // fn type renders, and since 2026-09-21 the one implicits use too
            // (a *kept* position must not move what it was handed).
            // [copy-implicit] A handler's *constructor* implicit is a written
            // fn-typed parameter, so it has always followed that rule.
            // [rs-proj-lends] A *lent* position (`[c: proj]`) is `&T` on any
            // implicit: the result holds a borrow of it.
            let kept_ref: Vec<bool> = self
                .implicits
                .iter()
                .find(|i| i.name == name)
                .map(|i| match i.ty.strip_quals() {
                    Ty::Fn {
                        params, contract, ..
                    } => (0..params.len())
                        .map(|k| {
                            let entry = contract.as_ref().and_then(|c| c.get(k));
                            if entry.is_some_and(|e| e.kept && e.lent) {
                                return true;
                            }
                            let kept = entry.is_none_or(|e| e.kept && !e.mutable);
                            kept && !params.get(k).is_some_and(Self::is_copy_ty)
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .unwrap_or_default();
            let arg_code: Vec<String> = args
                .iter()
                .enumerate()
                .map(|(k, a)| {
                    if modes.get(k).copied().unwrap_or(false) {
                        self.borrowed_mut_arg(a)
                    } else if kept_ref.get(k).copied().unwrap_or(false) {
                        match self.borrow_value(a) {
                            Some(b) => b,
                            None => format!("&{}", self.emit_owned(a)),
                        }
                    } else {
                        self.emit_owned(a)
                    }
                })
                .collect();
            // [iter-fn] Inside a generated pass the implicit is a
            // *field* (an `Rc<dyn Fn…>`), so it is called through `self` and
            // nothing is re-borrowed.
            if matches!(self.bindings.get(name), Some(BindKind::SelfField)) {
                return format!("(self.{})({})", rs_ident(name), arg_code.join(", "));
            }
            // [effect-args-hoisted] Calling through the parameter borrows it,
            // so an argument that *also* reaches it (a recursive call
            // forwarding the same implicit) is hoisted out first.
            let borrowed = vec![format!("&mut *{}", rs_ident(name))];
            let (prelude, arg_code) = self.hoist_reborrows(&borrowed, arg_code);
            return Self::wrap_hoisted(
                &prelude,
                format!("{}({})", rs_ident(name), arg_code.join(", ")),
            );
        }

        // 2. Checker-resolved fn target (type-based overloads win)
        // [fn-overload]. An `intrinsic fn` lowers in the emitter
        // [intrinsic-fn]; anything else has a body, since a bodiless
        // top-level fn is a parse error [decl-body].
        let checker_resolved = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|key| self.fn_by_key(*key).map(|f| (*key, f)));
        if let Some((key, f)) = checker_resolved {
            if f.intrinsic {
                return self.emit_intrinsic_call(f, args, span);
            }
            return self.emit_fn_call(name, f, Some(key), args, named, span);
        }

        // 3. Known function (unchecked contexts): arity narrowed by the
        // checked argument types; ambiguous dispatch is a codegen error,
        // never a guess [backend-never-wrong] [fn-overload].
        //
        // [call-resolve] Not when the checker resolved the callee to a
        // fn-typed **local**: `Symbols::fns` is program-wide and scope-blind,
        // so a program declaring `fn keep(…)` made std's
        // `filter(it, keep: (T) -> Bool)` call *the program's* fn from inside
        // `core/seq.rs` — `self::keep(x)`, which does not even resolve there.
        // The same record the member branch above reads answers it (found
        // 2026-09-14, pre-existing).
        let fn_cands = if self.checked.local_calls.contains(&(self.file_idx, span)) {
            Vec::new()
        } else {
            self.symbols.fns_matching_arity(name, args.len())
        };
        if !fn_cands.is_empty() {
            let Some(f) = disambiguate_unchecked(self, &fn_cands, |f| f.params.as_slice(), args)
            else {
                self.error(format!(
                    "call to `{name}` is ambiguous here: multiple same-arity \
                     overloads match and the checker did not resolve the \
                     overload; annotate the argument types"
                ));
                return "todo!()".to_string();
            };
            if f.intrinsic {
                return self.emit_intrinsic_call(f, args, span);
            }
            let key = self.key_of_fn(f);
            return self.emit_fn_call(name, f, key, args, named, span);
        }

        // 4. Local callable / interop. A call through a fn-typed value
        // renders its arguments per the recorded contract [fn-contract]:
        // kept non-Copy borrows, kept `Mut` borrows mutably, moved (or
        // Copy) owned. Interop calls (no contract) keep owned
        // pass-through.
        let contract = self
            .checked
            .fn_value_calls
            .get(&(self.file_idx, span))
            .cloned();
        let arg_code: Vec<String> = match contract {
            Some(entries) => args
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let (kept, is_mut) = entries
                        .get(i)
                        .map(|e| (e.kept, e.mutable))
                        .unwrap_or((true, false));
                    let copy = self.ty_of(a.span()).is_some_and(|t| Self::is_copy_ty(t));
                    if kept && is_mut {
                        self.borrowed_mut_arg(a)
                    } else if kept && !copy {
                        self.borrowed_arg(a)
                    } else {
                        self.emit_expr(a)
                    }
                })
                .collect(),
            None => args.iter().map(|a| self.emit_expr(a)).collect(),
        };
        // [fn-effects] A fn value takes its effects as leading arguments.
        let mut all = self.fn_value_effect_args(span);
        all.extend(arg_code);
        let generics = self.emit_call_type_args(type_args);
        // [iter-fn] A callback that is a field of the generated pass
        // needs parentheses: `self.f(..)` would be a method call.
        let callee = if self.gen_fields.contains(name) {
            format!("(self.{})", rs_ident(name))
        } else {
            rs_ident(name)
        };
        format!("{callee}{generics}({})", all.join(", "))
    }

    /// [fn-effects] The effect arguments a fn-value call threads, from the
    /// instances the checker resolved for it. Under the fusion the value's
    /// type declares **one** `&mut dyn` provider parameter
    /// ([`Self::fn_type_effect_params`]), so the per-effect expressions —
    /// all reborrows of the same fused value — collapse to one, and the
    /// Sized fused value unsizes into the provider `dyn`.
    fn fn_value_effect_args(&mut self, span: Span) -> Vec<String> {
        let effects: Vec<Ty> = self
            .checked
            .call_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        let mut out: Vec<String> = effects
            .iter()
            .map(|ty| self.thread_effect_by_ty(ty))
            .collect();
        if self.fusion {
            out.dedup();
            out.truncate(1);
        }
        out
    }

    /// Explicit call-site generic args (`next_random<Int>()` outside the
    /// effect path) render as turbofish.
    fn emit_call_type_args(&mut self, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            String::new()
        } else {
            let strs: Vec<String> = type_args.iter().map(|t| self.emit_type(t)).collect();
            format!("::<{}>", strs.join(", "))
        }
    }

    /// Renders the arguments of a call against the callee's declared
    /// parameters: mode per parameter [rs-borrows], trailing arguments
    /// collected into a `vec![...]` for a variadic parameter
    /// [fn-variadic].
    fn emit_args_for_params(
        &mut self,
        params: &[Param],
        args: &[&Expr],
        fn_key: Option<salvo_core::FnKey>,
    ) -> Vec<String> {
        self.emit_args_for_params_of(params, args, fn_key, None)
    }

    /// [rs-borrows] The same, for a call whose callee is an **effect
    /// member**: modes come from the member's written clause
    /// (`member_param_mode`), so a consumed parameter's argument is rendered
    /// *owned* and a kept one borrowed — the trait method it is calling says
    /// exactly that.
    fn emit_args_for_params_of(
        &mut self,
        params: &[Param],
        args: &[&Expr],
        fn_key: Option<salvo_core::FnKey>,
        member: Option<&FnDecl>,
    ) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        // [rs-iter-pass] An iterator fn's fn-typed parameter is declared
        // `impl Fn(…) + 'static`, so a lambda in that position must own what
        // it captures: a borrowing closure is E0373 ("may outlive the current
        // function"), even for an immutable capture the parity rules call
        // free. The *type* said `'static` from the start; the closure has to
        // say `move`.
        let producer = fn_key.is_some_and(|k| self.owns_callbacks(k));
        let variadic_at = params.iter().position(|p| p.variadic);
        let fixed = variadic_at.unwrap_or(params.len());
        for (i, param) in params.iter().enumerate().take(fixed) {
            let Some(arg) = args.get(i) else { break };
            let mode = match (fn_key, member) {
                (Some(_), _) => self.param_mode(fn_key, param),
                (None, Some(m)) => {
                    let m = m.clone();
                    self.member_param_mode(&m, param)
                }
                (None, None) => self.default_param_mode(&param.ty, param.variadic),
            };
            self.pending_lambda_move = producer;
            out.push(self.emit_arg(arg, mode, Some(&param.ty)));
            self.pending_lambda_move = false;
        }
        if variadic_at.is_some() {
            let rest = args.get(fixed..).unwrap_or(&[]);
            // A single spread forwards the whole vector [fn-variadic] — but a
            // **place** is cloned, not moved. The variadic parameter is owned
            // in the emitted signature (it is built from the arguments), while
            // the checker does not track a variadic position, so the caller's
            // array stays live afterwards: moving it made `total(...rest)`
            // followed by any further use of `rest` a raw rustc E0382, with no
            // Salvo diagnostic [backend-never-wrong]. The intrinsic path
            // learned this with the sorted collections; this one had not.
            if rest.len() == 1 {
                if let Expr::Spread { operand, .. } = rest[0] {
                    let code = self.emit_owned(operand);
                    let place = matches!(
                        operand.as_ref(),
                        Expr::Ident(_) | Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. }
                    );
                    out.push(if place && !code.ends_with(".clone()") {
                        format!("{code}.clone()")
                    } else {
                        code
                    });
                    return out;
                }
            }
            // [fn-variadic] A **mixed** tail — plain arguments and a
            // `...spread` together — is assembled into one vector, in written
            // order, since the variadic parameter is a single `Vec<T>`.
            if rest.iter().any(|a| matches!(a, Expr::Spread { .. })) {
                let mut steps = String::new();
                for arg in rest {
                    match arg {
                        Expr::Spread { operand, .. } => {
                            let code = self.emit_owned(operand);
                            steps.push_str(&format!("__v.extend({code}.iter().cloned()); "));
                        }
                        other => {
                            let code = self.emit_owned(other);
                            steps.push_str(&format!("__v.push({code}); "));
                        }
                    }
                }
                out.push(format!("{{ let mut __v = Vec::new(); {steps}__v }}"));
                return out;
            }
            let items: Vec<String> = rest.iter().map(|a| self.emit_expr(a)).collect();
            out.push(format!("vec![{}]", items.join(", ")));
        }
        out
    }

    fn emit_arg(&mut self, arg: &Expr, mode: ParamMode, param_ty: Option<&Type>) -> String {
        // [fn-contract] A named fn passed into a fn-typed position wraps
        // in an adapter closure matching the expected contract.
        if let (Some(pt @ Type::Fn { .. }), Expr::Ident(id)) = (param_ty, arg) {
            if !self.bindings.contains_key(id.name.as_str()) {
                if let Some(code) = self.named_fn_adapter(&id.name, id.span, pt, mode) {
                    return code;
                }
            }
        }
        // [fn-contract] [rs-fn-param-convention] A *lambda* in a fn-typed
        // position must bind its parameters the way the callee's declared
        // fn type renders them — the callee is what fixes the calling
        // convention, and its declared type is the only thing both sides
        // can agree on. Deciding from the lambda's own annotation instead
        // disagrees exactly where the two types differ: the declaration of
        // `f: (T) -> U` renders `FnMut(&T)` (a type variable is never
        // known to be `Copy`), while an annotated `(n: Int) -> …` rendered
        // `|n: i32|` and rustc rejected the call (`E0631`).
        if let (Some(Type::Fn { .. }), Expr::Lambda { .. }) = (param_ty, arg) {
            self.pending_lambda_conv = param_ty.map(|pt| self.fn_type_param_conventions(pt));
            // [yield-proj] Parameters typed at a retagged element generic
            // arrive with one extra reference; the lambda peels it.
            if let Some(Type::Fn { params: fps, .. }) = param_ty {
                let retag: Vec<bool> = fps
                    .iter()
                    .map(|t| match t {
                        Type::Named { qualifiers, base } => {
                            qualifiers.is_empty()
                                && base.args.is_empty()
                                && self.retagged_generics.contains(&base.name.name)
                        }
                        _ => false,
                    })
                    .collect();
                if retag.iter().any(|b| *b) {
                    self.pending_lambda_retag = Some(retag);
                }
            }
        }
        // [rs-proj-arm] A union value whose `proj` arm is a reference
        // (`next(p)` is `Union2<&i32, Finished>`) flowing into a position
        // written as the owned union (`step: Emitted Int | Finished`): the
        // Salvo types agree (`proj` is not a type), the Rust ones do not.
        // A Copy payload is copied out by a match adapter
        // [copy-scalar-free]; anything else would need a hidden clone, which
        // this backend refuses to emit.
        if let Some(adapted) = self.adapt_borrowed_arms_to_owned(arg, param_ty, mode) {
            return adapted;
        }
        match mode {
            ParamMode::Owned => self.emit_expr(arg),
            ParamMode::Ref => {
                // A coerced argument is a fresh temporary: borrow it.
                if self.coercion_of(arg.span()).is_some() {
                    let code = self.emit_expr(arg);
                    format!("&({code})")
                } else {
                    self.borrowed_arg(arg)
                }
            }
            ParamMode::RefMut => {
                if self.coercion_of(arg.span()).is_some() {
                    let code = self.emit_expr(arg);
                    format!("&mut ({code})")
                } else {
                    self.borrowed_mut_arg(arg)
                }
            }
        }
    }

    /// A call to an `intrinsic fn`, lowered directly by the compiler
    /// [intrinsic-fn]. Two kinds live here:
    ///
    /// - `copy` and `discard` dispatch on the argument's *shape*, which is
    ///   the whole reason they are intrinsics — `copy` is `.clone()` on the
    ///   argument's place (reads never consume, and every generated type
    ///   derives `Clone`), while a non-place argument is already a fresh
    ///   owned value and passes through [rs-copy] [linear-discard].
    /// - everything else std declares is looked up in
    ///   [`crate::intrinsics::fn_call`], keyed by the declaration the
    ///   checker resolved.
    ///
    /// An intrinsic with no lowering is a codegen error naming it, never a
    /// pass-through [backend-never-wrong].
    fn emit_intrinsic_call(&mut self, f: &FnDecl, args: &[&Expr], span: Span) -> String {
        // [linear-discard] `discard(x)` moves the value into `drop`.
        if f.name.name == "discard" && args.len() == 1 {
            let code = self.emit_expr(args[0]);
            return format!("drop({code})");
        }
        if f.name.name != "copy" || args.len() != 1 {
            let recv = f.params.first().and_then(|p| type_base_name(&p.ty));
            let arg_code = self.intrinsic_arg_code(f, args);
            // [rs-mut-str] The same for the string helper trait: `set`'s
            // lowering is a method from the generated support module.
            if f.name.name == "set" && recv == Some("Str") {
                self.needs_str = true;
            }
            // [rs-actor] The scheduler's own intrinsics: answering a reply
            // token, building a pool (plain or dedicated), registering a
            // death watch, and registering a quiescence hook
            // [actor-on-idle].
            if recv == Some("Reply")
                || matches!(
                    f.name.name.as_str(),
                    "pool" | "thread" | "watch" | "on_idle"
                )
            {
                self.needs_scheduler = true;
            }
            // [rs-seq] The `List` fast paths lower to the generated
            // sequence helpers, which is what gives their callbacks an
            // expected type — a closure bound to a `let` cannot infer its
            // parameters, so an inline lowering would not compile.
            if recv == Some("List")
                && matches!(
                    f.name.name.as_str(),
                    // [linear-container] `remove_first`/`remove_at` are methods
                    // on the same generated trait file.
                    "map" | "filter" | "reduce" | "remove_first" | "remove_at"
                )
            {
                self.needs_seq = true;
            }
            // [rs-collections] The ordered `Set`/`Map` runtime: either the
            // receiver is one of them, or this is a constructor, whose
            // receiver is the variadic array rather than the collection it
            // builds.
            if recv == Some("Set")
                || recv == Some("Map")
                || matches!(
                    f.name.name.as_str(),
                    "set_of"
                        | "mut_set_of"
                        | "map_of"
                        | "mut_map_of"
                        | "set_by"
                        | "mut_set_by"
                        | "map_by"
                        | "mut_map_by"
                        | "to_set"
                        | "to_map"
                )
            {
                self.needs_collections = true;
            }
            // [time-types] [rs-time] The two clock readings live in their own
            // runtime module, so a program that reads a clock mounts it — and
            // one that never asks the time carries nothing.
            if matches!(
                f.name.name.as_str(),
                "monotonic_nanos" | "epoch_nanos" | "fire_after"
            ) {
                self.needs_time = true;
            }
            // [time-timer] A deadline is the scheduler's, and its reading comes
            // from the time module — so registering one needs both files.
            if f.name.name == "fire_after" {
                self.needs_scheduler = true;
            }
            // [fn-variadic] How the variadic tail arrived, which decides the
            // shape the constructor lowering wants. A lone `...spread`
            // forwards a *borrowed* collection; a tail that mixes plain
            // arguments with a spread was assembled into a fresh vector by
            // `intrinsic_arg_code`, so it arrives owned and must not be
            // cloned again.
            // [cmp-carry] When this call builds a keyed container, the identities
            // it will be kept by are in the call's own type — an ordering for the
            // sorted pair, a hash/equality pair for the hash pair. One slot,
            // since only one of the two applies to any call.
            let ordering = self
                .ordering_marker_for(span)
                .or_else(|| self.container_identity_markers(span));
            let variadic_at = f.params.iter().position(|p| p.variadic);
            let tail_len = variadic_at.map_or(0, |v| args.len().saturating_sub(v));
            let spread = if !args.iter().any(|a| matches!(a, Expr::Spread { .. })) {
                crate::intrinsics::Spread::None
            } else if tail_len > 1 {
                crate::intrinsics::Spread::Owned
            } else {
                crate::intrinsics::Spread::Borrowed
            };
            if let Some(code) =
                crate::intrinsics::fn_call(
                    &f.name.name,
                    recv,
                    &arg_code,
                    &[],
                    spread,
                    ordering.as_deref(),
                )
            {
                return code;
            }
            self.error(format!(
                "intrinsic fn `{}` is not supported by the rust backend",
                f.name.name
            ));
            return "todo!()".to_string();
        }
        let _ = span;
        let arg = args[0];
        // [rs-fn-field] `copy` of a **function value** is the value itself: a
        // callback is shared, not duplicated — an owned `impl Fn` parameter has
        // no `clone`, and an `Rc`-held field's clone is the share the store
        // makes anyway. Salvo asks for the `copy` because the position *keeps*
        // the callback [fn-contract]; in Rust there is nothing to copy.
        if self
            .ty_of(arg.span())
            .is_some_and(|t| matches!(t.strip_quals(), Ty::Fn { .. }))
        {
            return match arg {
                Expr::Field { .. } => self.emit_owned(arg),
                other => self.emit_expr(other),
            };
        }
        match arg {
            Expr::Ident(id) => {
                if let Some(unwrapped) = self.ident_unwrap(id) {
                    // The narrowing unwrap is already an owned clone.
                    return unwrapped;
                }
                format!("{}.clone()", self.binding_place(&id.name))
            }
            // Field/index reads already clone in owned position.
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                self.emit_owned(arg)
            }
            // [rs-copy] [proj-type] A call that answers a **projection**
            // renders as a borrow — `get(list, i)!` is `.get(i).unwrap()`, an
            // `&T` — so the "a call result is already owned" reading does not
            // hold for it and the clone is the whole point of the `copy`.
            // Found 2026-09-14 writing std's `RestrictedFs`
            // (`parts.add(copy(get(kept, j)!))` pushed an `&String`, E0308).
            other if self.ty_of(other.span()).is_some_and(|t| t.is_proj()) => {
                let code = self.emit_expr(other);
                format!("{code}.clone()")
            }
            other => self.emit_expr(other),
        }
    }

    /// The rendered arguments of an `intrinsic fn` call, in declaration
    /// order, preserving the distinction the intrinsic lowerings rely on
    /// [rs-borrows]: a *place* splices raw so a method-style lowering
    /// borrows it natively (`list.push(..)`), while a variadic tail
    /// splices owned because it lands inside a constructor (`vec![..]`).
    /// Getting this backwards either double-clones or moves out of a
    /// borrow.
    fn intrinsic_arg_code(&mut self, f: &FnDecl, args: &[&Expr]) -> Vec<String> {
        let variadic_at = f.params.iter().position(|p| p.variadic);
        // [fn-variadic] Whether the callee *stores* its variadic tail, which
        // decides how a place in that tail is rendered. A declaration that
        // names the tail in its deductions only reads it (`mut_str(...parts)
        // => parts`), so the parts are borrowed; one that does not, stores
        // them (`list_of(...elems)`), and since the flow analysis does not
        // track a variadic position the caller's variable is still live
        // afterwards — so the store must **clone** rather than move it, or
        // the same program is accepted on Kotlin (references) and rejected by
        // rustc with a raw E0382 [backend-never-wrong].
        let variadic_stored = variadic_at
            .and_then(|v| f.params.get(v))
            .is_some_and(|p| {
                let kept = f.deductions.as_ref().is_some_and(|ds| {
                    ds.iter()
                        .any(|d| d.param_name().is_some_and(|n| n.name == p.name.name))
                });
                !kept
            });
        let fixed: Vec<Param> = f.params.iter().filter(|p| !p.implicit).cloned().collect();
        // [deduce-syntax] Which fixed parameters the declaration says are
        // **consumed** (`=> !p`, equivalently `p: Never`). An intrinsic has
        // no body, so its written clause is the whole truth — and it must
        // mention every non-Copy, non-variadic parameter, so an unmentioned
        // one is genuinely kept. Six-odd std declarations are in this set
        // (`send`, `discard`, `add`, `insert_sorted_by`, `put`, `reduce`'s seed),
        // and each of them needs its argument *owned* rather than borrowed.
        let consumed: Vec<bool> = fixed
            .iter()
            .map(|p| {
                f.deductions.as_ref().is_some_and(|ds| {
                    ds.iter().any(|d| {
                        matches!(
                            d.kind,
                            salvo_syntax::ast::DeductionKind::Moved
                                | salvo_syntax::ast::DeductionKind::Deferred
                        ) && d.param_name().is_some_and(|n| n.name == p.name.name)
                    })
                })
            })
            .collect();
        let mut out: Vec<String> = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let is_variadic_part = variadic_at.is_some_and(|v| i >= v);
            let param_ty = fixed.get(i).map(|p| p.ty.clone());
            if let (Some(pt @ Type::Fn { .. }), Expr::Ident(id)) = (&param_ty, arg) {
                // [fn-contract] A *named fn* in a fn-typed position wraps in
                // an adapter closure here too: a fn item is not a closure,
                // and its parameter conventions are its own
                // (`map(xs, double)` passed `fn(i32) -> i32` where the
                // lowering wants `FnMut(&i32) -> _`, which rustc reports as
                // E0631).
                if !self.bindings.contains_key(id.name.as_str()) {
                    let pt = pt.clone();
                    if let Some(code) =
                        self.named_fn_adapter(&id.name, id.span, &pt, ParamMode::Owned)
                    {
                        out.push(code);
                        continue;
                    }
                }
            }
            // [rs-fn-param-convention] A lambda argument follows the
            // *declared* parameter type's conventions here too: an
            // intrinsic's `f: (T) -> U` renders `FnMut(&T)`, which is what
            // the lowering hands it. Without this the lambda would take its
            // parameter by value and the lowering would not typecheck
            // (`|n| n > 1` against `&&i32`).
            if let (Some(pt @ Type::Fn { .. }), Expr::Lambda { .. }) = (&param_ty, arg) {
                let pt = pt.clone();
                self.pending_lambda_conv = Some(self.fn_type_param_conventions(&pt));
                // [yield-proj] Which parameters are typed at a retagged
                // element generic.
                if let Type::Fn { params: fps, .. } = &pt {
                    let retag: Vec<bool> = fps
                        .iter()
                        .map(|t| match t {
                            Type::Named { qualifiers, base } => {
                                qualifiers.is_empty()
                                    && base.args.is_empty()
                                    && self.retagged_generics.contains(&base.name.name)
                            }
                            _ => false,
                        })
                        .collect();
                    if retag.iter().any(|b| *b) {
                        self.pending_lambda_retag = Some(retag);
                    }
                }
            }
            out.push(match arg {
                // A Copy scalar read out of a reference binding (a lambda
                // parameter under the `FnMut(&T)` convention
                // [rs-fn-param-convention]) is a *value* here, not a place:
                // lowerings use scalar arguments in casts (`(i) as usize`),
                // which a `&i32` place fails (E0606). The deref is free.
                Expr::Ident(id)
                    if !is_variadic_part
                        && matches!(
                            self.bindings.get(id.name.as_str()),
                            Some(BindKind::Ref | BindKind::RefMut)
                        )
                        && self.ty_of(id.span).is_some_and(|t| Self::is_copy_ty(t)) =>
                {
                    format!("*{}", self.binding_place(&id.name))
                }
                Expr::Ident(_)
                | Expr::Field { .. }
                | Expr::TupleIndex { .. }
                | Expr::Index { .. }
                    if !is_variadic_part && consumed.get(i).copied().unwrap_or(false) =>
                {
                    // The declaration says this parameter is **consumed**
                    // (`=> !value`), so the lowering takes ownership: a place
                    // is not enough. `emit_owned` is what knows how to make
                    // one — a real partial move stays a move
                    // [fate-move-mode], and a read the caller keeps (a field
                    // of the enclosing handler, a borrowed binding) clones.
                    //
                    // Without this, `send(out, last)` and `add(xs, last)` on a
                    // handler's own `Str` state field emitted `self.last` into
                    // a moving position and rustc reported E0507 with no Salvo
                    // diagnostic — loud, but a [backend-never-wrong] gap all
                    // the same. A `Copy` state field hid it: the first
                    // asynchronous test's `sum: Int` copied (found
                    // 2026-09-15).
                    self.emit_owned(arg)
                }
                // [rs-narrow-mut] A parameter the declaration types `Mut` is a
                // **mutable use** of its argument, so a narrowed place unwraps
                // through the mutable accessors — the read form clones out of
                // the representation and the mutation lands on the temporary.
                //
                // This is the same fault [rs-narrow-mut] closed for declared
                // calls in 2026-09-10, left open on the *intrinsic* path
                // because that path renders every place with `emit_place`
                // regardless of the parameter's mode: `add(xs, 3)` on a
                // narrowed `Mut List<Int>?` emitted
                // `xs.as_ref().unwrap().clone().push(3)`, which compiles,
                // warns about nothing and drops the element on the floor while
                // Kotlin (whose smart cast is the storage) appended it. Found
                // 2026-09-20 designing `?.`; the whole defect is in
                // COMPLETED.md.
                //
                // Falls back to the read form when the place is not narrowed,
                // so an ordinary `Mut` argument is unchanged.
                Expr::Ident(_) | Expr::Field { .. } | Expr::TupleIndex { .. }
                    if !is_variadic_part
                        && !consumed.get(i).copied().unwrap_or(false)
                        && param_ty.as_ref().is_some_and(type_has_mut) =>
                {
                    match self.narrowed_place_mut(arg) {
                        Some(code) => code,
                        None => self.emit_place(arg),
                    }
                }
                Expr::Ident(_)
                | Expr::Field { .. }
                | Expr::TupleIndex { .. }
                | Expr::Index { .. }
                    if !is_variadic_part =>
                {
                    self.emit_place(arg)
                }
                // [fn-variadic] A place in the **variadic tail** is cloned
                // rather than moved, because Salvo does not track a variadic
                // position: `list_of(a)` leaves `a` usable, which is what
                // Kotlin's `listOf(a)` does (references) and what the checker
                // therefore permits. Without the clone the same program was
                // accepted on Kotlin and rejected by *rustc* — a raw E0382,
                // with no Salvo diagnostic [backend-never-wrong]. Fixed
                // 2026-09-13 with the sorted collections, which surfaced it;
                // the alternative (tracking variadic moves in the checker)
                // would make the existing programs errors instead.
                Expr::Ident(_)
                | Expr::Field { .. }
                | Expr::TupleIndex { .. }
                | Expr::Index { .. }
                    if is_variadic_part && variadic_stored =>
                {
                    let code = self.emit_owned(arg);
                    let copy = self
                        .ty_of(arg.span())
                        .is_some_and(|t| Self::is_copy_ty(t));
                    if copy || code.ends_with(".clone()") {
                        code
                    } else {
                        format!("{code}.clone()")
                    }
                }
                Expr::Spread { operand, .. } => self.emit_owned(operand),
                other => self.emit_expr(other),
            });
        }
        // [fn-variadic] A tail that **mixes** plain arguments with a
        // `...spread` is assembled here into one owned vector, because a
        // constructor lowering takes the tail as a single collection. Pushing
        // and extending in written order keeps a spread legal anywhere in the
        // tail, and building the vector directly avoids the whole-vector
        // clone that handing an assembled vector to the borrowed-spread
        // shape would cost. Nothing about Rust prevented this — it was
        // simply unimplemented, and the intrinsic path silently dropped the
        // leading elements until 2026-09-13.
        if let Some(v) = variadic_at {
            let tail = args.get(v..).unwrap_or(&[]);
            if tail.len() > 1 && tail.iter().any(|a| matches!(a, Expr::Spread { .. })) {
                let mut steps = String::new();
                for (k, arg) in tail.iter().enumerate() {
                    let code = out.get(v + k).cloned().unwrap_or_default();
                    if matches!(arg, Expr::Spread { .. }) {
                        // A borrowed forward has to clone per element; the
                        // elements of an owned temporary can be moved.
                        steps.push_str(&format!("__v.extend({code}.iter().cloned()); "));
                    } else {
                        steps.push_str(&format!("__v.push({code}); "));
                    }
                }
                out.truncate(v);
                out.push(format!("{{ let mut __v = Vec::new(); {steps}__v }}"));
            }
        }
        out
    }

    /// A call to a declared function: effect handlers thread as leading
    /// `&mut` arguments [rs-effects]; parameter modes come from the
    /// deductions [rs-borrows].
    /// [implicit-override] A value written for an implicit parameter, adapted
    /// to the borrowed-`FnMut` position: a *fn name* is a fn item, not a
    /// closure, so it is wrapped like any named fn passed by value
    /// [fn-contract]; anything else (a lambda, a fn-typed local) is borrowed
    /// as it stands.
    ///
    /// `handed` is how the *position* passes each parameter on
    /// ([rs-fn-param-convention]: `Some(true)` for `&mut T`, `Some(false)` for
    /// `&T`, `None` for by value), which is what the adapter has to bridge to
    /// the written fn's own convention — and what a *lambda* argument binds
    /// its parameters under, exactly as in a written fn-typed position.
    fn implicit_value(&mut self, value: &Expr, arity: usize, handed: &[Option<bool>]) -> String {
        if let Expr::Ident(id) = value {
            let is_local = self.bindings.contains_key(id.name.as_str());
            if !is_local {
                if let Some(key) = self.checked.fn_refs.get(&(self.file_idx, id.span)).copied() {
                    if let Some(decl) = self.fn_by_key(key) {
                        let target = self.rust_fn_name(decl);
                        let fixed: Vec<Param> = decl
                            .params
                            .iter()
                            .filter(|p| !p.implicit)
                            .cloned()
                            .collect();
                        let ps: Vec<String> = (0..arity).map(|i| format!("__i{i}")).collect();
                        let args: Vec<String> = ps
                            .iter()
                            .enumerate()
                            .map(|(i, code)| match fixed.get(i) {
                                Some(p) => {
                                    let h = handed.get(i).copied().flatten();
                                    self.adapter_arg(Some(key), p, h, code, &decl.name.name)
                                }
                                None => code.clone(),
                            })
                            .collect();
                        return format!(
                            "&mut |{}| {target}({})",
                            ps.join(", "),
                            args.join(", ")
                        );
                    }
                }
            }
        }
        // [rs-fn-param-convention] A lambda written for the position binds its
        // parameters the way the position renders them — the same rule a
        // lambda in a *written* fn-typed parameter follows, and the reason an
        // annotated `add = (a: Int, b: Int) -> a * b` no longer collides with
        // a `FnMut(&T, &T)` position (`E0631`).
        if matches!(value, Expr::Lambda { .. }) && handed.iter().any(|h| h.is_some()) {
            self.pending_lambda_conv = Some(
                handed
                    .iter()
                    .map(|h| match h {
                        Some(true) => BindKind::RefMut,
                        Some(false) => BindKind::Ref,
                        None => BindKind::Owned,
                    })
                    .collect(),
            );
        }
        let code = self.emit_owned(value);
        format!("&mut ({code})")
    }

    /// [rs-fn-param-convention] One argument of an adapter closure that wraps
    /// a named fn into an implicit position: the position handed the parameter
    /// over under its own convention (`handed` — `Some(true)` for `&mut T`,
    /// `Some(false)` for `&T`), the callee wants the mode its own declaration
    /// gives, and this bridges the two.
    fn adapter_arg(
        &mut self,
        key: Option<salvo_core::FnKey>,
        p: &Param,
        handed: Option<bool>,
        code: &str,
        callee: &str,
    ) -> String {
        let already_mut = handed == Some(true);
        let already_ref = handed == Some(false);
        match self.param_mode(key, p) {
            ParamMode::Owned if already_mut || already_ref => format!("({code}).clone()"),
            ParamMode::Owned => code.to_string(),
            // `&mut T` coerces to `&T`; a borrowed position is `&T` already.
            ParamMode::Ref if already_mut || already_ref => code.to_string(),
            ParamMode::Ref => format!("&{code}"),
            ParamMode::RefMut if already_mut => code.to_string(),
            ParamMode::RefMut if already_ref => {
                self.error(format!(
                    "`{callee}` mutates `{}`, but the position lends it (`proj`): \
                     a lent argument is read-only",
                    p.name.name
                ));
                code.to_string()
            }
            ParamMode::RefMut => format!("&mut {code}"),
        }
    }

    /// [implicit-intrinsic] The body of the adapter closure an `intrinsic
    /// fn` becomes when it is passed as a *value*: the intrinsic's own
    /// lowering, applied to the closure's parameters. A lowering with no
    /// entry for this backend is a codegen error, as at any other reference
    /// [backend-never-wrong].
    fn intrinsic_fn_value_body(&mut self, decl: &FnDecl, params: &[String]) -> String {
        let recv = decl.params.first().and_then(|p| type_base_name(&p.ty));
        if decl.name.name == "set" && recv == Some("Str") {
            self.needs_str = true;
        }
        if recv == Some("List") && matches!(decl.name.name.as_str(), "map" | "filter" | "reduce") {
            self.needs_seq = true;
        }
        match crate::intrinsics::fn_call(
            &decl.name.name,
            recv,
            params,
            &[],
            crate::intrinsics::Spread::None,
            None,
        ) {
            Some(code) => code,
            None => {
                self.error(format!(
                    "intrinsic fn `{}` is not supported by the rust backend",
                    decl.name.name
                ));
                "todo!()".to_string()
            }
        }
    }

    /// [implicit-resolve] What a call passes for each implicit parameter:
    /// the value written at the call site, the enclosing fn's own implicit
    /// forwarded on, or the fn resolution found — wrapped in the adapter
    /// closure a fn-typed position expects [fn-contract].
    fn emit_implicit_args(&mut self, named: &[NamedArg], span: Span) -> Vec<String> {
        let filled = match self.checked.implicit_args.get(&(self.file_idx, span)) {
            Some(filled) => filled.clone(),
            None => return Vec::new(),
        };
        // The callee whose implicit positions these fill, for shape checks.
        let callee_decl: Option<&'p FnDecl> = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|key| self.fn_by_key(*key));
        let mut out = Vec::new();
        // [rs-iter-pass] An iterator fn's implicits arrive **owned** and
        // `'static` like its written callbacks, so the adapter is a `move`
        // closure rather than a `&mut` borrow of one: the pass calls it long
        // after this call returns.
        let producer = self.emitting_producer_args
            || self
                .checked
                .call_fn
                .get(&(self.file_idx, span))
                .is_some_and(|k| self.owns_callbacks(*k));
        // [implicit-param] How the *position* hands each parameter over: a
        // kept-`Mut` one arrives as `&mut T` already (`fn_ty_param_renderings`),
        // so the adapter must not borrow it a second time — `next(&mut __i0)`
        // where `__i0: &mut ListYield<i32>` is E0596. Keyed by implicit name,
        // since that is what the adapter loop has.
        // `Some(false)` marks a *lent* position, handed over as `&T`
        // [rs-proj-lends]; `Some(true)` a kept-`Mut` one (`&mut T`).
        let position_refmut: HashMap<String, Vec<Option<bool>>> = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|k| self.checked.implicit_params.get(k))
            .map(|params| {
                params
                    .iter()
                    .map(|p| {
                        let modes = match p.ty.strip_quals() {
                            Ty::Fn {
                                params, contract, ..
                            } => params
                                .iter()
                                .enumerate()
                                .map(|(i, pt)| match contract.as_ref().and_then(|c| c.get(i)) {
                                    Some(e) if e.kept && e.mutable => Some(true),
                                    Some(e) if e.kept && e.lent => Some(false),
                                    // [rs-fn-param-convention] A kept
                                    // non-Copy position arrives as `&T`.
                                    Some(e) if e.kept && !Self::is_copy_ty(pt) => Some(false),
                                    Some(_) => None,
                                    None if pt.quals().iter().any(|q| q.name == "Mut") => {
                                        Some(true)
                                    }
                                    None if !Self::is_copy_ty(pt) => Some(false),
                                    None => None,
                                })
                                .collect(),
                            _ => Vec::new(),
                        };
                        (p.name.clone(), modes)
                    })
                    .collect()
            })
            .unwrap_or_default();
        for arg in &filled {
            match arg {
                salvo_core::ImplicitArg::Given { name, arity } => {
                    match named.iter().find(|a| a.name.name == *name) {
                        Some(a) => {
                            let handed =
                                position_refmut.get(name).cloned().unwrap_or_default();
                            out.push(self.implicit_value(&a.value, *arity, &handed))
                        }
                        None => {
                            self.error(format!(
                                "internal: no value for implicit parameter `{name}`"
                            ));
                            out.push("todo!()".to_string());
                        }
                    }
                }
                salvo_core::ImplicitArg::Forwarded { name } => {
                    // Forwarding into a producer hands over a *share* of the
                    // callback (it is `Rc`-held), not a borrow of it.
                    if producer {
                        let held = match self.bindings.get(name.as_str()) {
                            Some(BindKind::SelfField) => format!("self.{}", rs_ident(name)),
                            _ => rs_ident(name),
                        };
                        out.push(format!("{held}.clone()"));
                    } else {
                        out.push(self.forwarded_implicit(name, span));
                    }
                }
                salvo_core::ImplicitArg::Resolved { name, key, .. } => {
                    match self.fn_by_key(*key) {
                        Some(decl) => {
                            let fixed: Vec<&Param> =
                                decl.params.iter().filter(|p| !p.implicit).collect();
                            let params: Vec<String> =
                                (0..fixed.len()).map(|i| format!("__i{i}")).collect();
                            // [yield-proj] The callee's element generic is
                            // retagged to `&T` for this call (its `next`
                            // borrows), so a position typed at that generic
                            // hands the adapter a reference where the resolved
                            // fn owns its parameter: the adapter clones it out —
                            // the copy the Salvo body wrote as `copy(x)`, landing
                            // here instead of in the identity `copy` adapter.
                            let retagged_positions: Vec<bool> = self
                                .checked
                                .call_fn
                                .get(&(self.file_idx, span))
                                .and_then(|k| self.checked.implicit_params.get(k))
                                .and_then(|ps| ps.iter().find(|p| p.name == *name))
                                .map(|p| match p.ty.strip_quals() {
                                    Ty::Fn { params, .. } => params
                                        .iter()
                                        .map(|t| match t.strip_quals() {
                                            Ty::Var(g) => self.retagged_generics.contains(g),
                                            Ty::Named { name: g, args } => {
                                                args.is_empty()
                                                    && self.retagged_generics.contains(g)
                                            }
                                            _ => false,
                                        })
                                        .collect(),
                                    _ => Vec::new(),
                                })
                                .unwrap_or_default();
                            let peels: String = fixed
                                .iter()
                                .enumerate()
                                .filter(|(i, p)| {
                                    retagged_positions.get(*i).copied().unwrap_or(false)
                                        && !decl.intrinsic
                                        && matches!(
                                            self.param_mode(Some(*key), p),
                                            ParamMode::Owned
                                        )
                                })
                                .map(|(i, _)| format!("let __i{i} = __i{i}.clone(); "))
                                .collect();
                            let peels = if decl.intrinsic && decl.name.name == "copy" {
                                // `copy` at a retagged `T` is `&&T -> &T`: its
                                // own `.clone()` below is exactly that peel.
                                String::new()
                            } else if decl.intrinsic {
                                // An intrinsic's lowering is applied to the
                                // parameters directly; a retagged one clones
                                // out first (`push` owns its element).
                                fixed
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, p)| {
                                        retagged_positions.get(*i).copied().unwrap_or(false)
                                            && !matches!(p.ty, Type::Named { ref qualifiers, .. } if qualifiers.iter().any(|q| q.name.name == "Mut"))
                                    })
                                    .map(|(i, _)| format!("let __i{i} = __i{i}.clone(); "))
                                    .collect()
                            } else {
                                peels
                            };
                            // [implicit-intrinsic] An `intrinsic fn` has no
                            // Rust function to name: it *is* a lowering, so
                            // the adapter's body is that lowering applied to
                            // the adapter's parameters. (`iter` would
                            // otherwise emit `iter(__i0)`, which names the
                            // generated `iter` *module* — E0423.)
                            let body = if decl.intrinsic && decl.name.name == "copy" {
                                // [copy-implicit] `copy` as a value: the
                                // position hands the argument over as `&T`
                                // (a kept fn-type parameter), and every Salvo
                                // type derives `Clone`, so the copy is one
                                // clone through the reference [rs-copy]. At a
                                // retagged `T` (`&i32`) the "copy" is the
                                // reference itself (a fn's own implicit takes
                                // its argument by value): identity.
                                if retagged_positions.first().copied().unwrap_or(false) {
                                    params[0].clone()
                                } else {
                                    format!("{}.clone()", params[0])
                                }
                            } else if decl.intrinsic {
                                self.intrinsic_fn_value_body(decl, &params)
                            } else {
                                let target = self.rust_fn_name(decl);
                                // [rs-borrows] The adapter is a *call*, so
                                // each argument takes the callee's own
                                // parameter mode: a kept struct parameter is
                                // `&T`, and passing it by value is E0308.
                                let args: Vec<String> = fixed
                                    .iter()
                                    .enumerate()
                                    .map(|(i, p)| {
                                        let p = (*p).clone();
                                        // The position may already have
                                        // handed this parameter over as
                                        // `&mut` or as `&T`.
                                        let handed = position_refmut
                                            .get(name)
                                            .and_then(|m| m.get(i))
                                            .copied()
                                            .flatten();
                                        self.adapter_arg(
                                            Some(*key),
                                            &p,
                                            handed,
                                            &params[i],
                                            &decl.name.name,
                                        )
                                    })
                                    .collect();
                                let call = format!("{target}({})", args.join(", "));
                                // [copy-scalar-free] A pass that walks data
                                // emits `&T`; when the position wants a
                                // *Copy scalar* by value (`?Yield<It, Int>`),
                                // the borrow is copied out — free, and the
                                // only shape whose element the position does
                                // not name as a generic to retag.
                                if self.scalar_position_wants_value(decl, callee_decl, name) {
                                    format!(
                                        "match {call} {{ Union2::U1(__e) => Union2::U1(*__e), \
                                         Union2::U2(__f) => Union2::U2(__f) }}"
                                    )
                                } else {
                                    call
                                }
                            };
                            // A fn item is not a closure: wrap it, so the
                            // parameter's `impl FnMut` bound is satisfied
                            // whatever the callee's convention is.
                            let body = if peels.is_empty() {
                                body
                            } else {
                                format!("{{ {peels}{body} }}")
                            };
                            if producer {
                                out.push(format!("move |{}| {body}", params.join(", ")));
                            } else {
                                out.push(format!("&mut |{}| {body}", params.join(", ")));
                            }
                        }
                        None => {
                            self.error(format!(
                                "internal: implicit parameter `{name}` resolved to no fn"
                            ));
                            out.push("todo!()".to_string());
                        }
                    }
                }
                salvo_core::ImplicitArg::OriginNext { name: _, next_fn } => {
                    // [iter-fn] The pass is the hidden machine minted
                    // at the argument, so the adapter wraps its own advance
                    // into the protocol's two arms — the same machine API the
                    // `for` lowering drives (`__advance` → `Option<T>`).
                    let Some(decl) = self.fn_by_key(*next_fn) else {
                        self.error(
                            "the `yield fn` behind a minted pass is not available".to_string(),
                        );
                        out.push("todo!()".to_string());
                        continue;
                    };
                    let elem = match decl.return_type.as_ref() {
                        Some(t) => self.emit_type(t),
                        None => "()".to_string(),
                    };
                    let effects: Vec<Ty> = self
                        .checked
                        .fn_effects
                        .get(next_fn)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|t| !is_throw_effect_ty(t))
                        .collect();
                    let mut hs: Vec<String> = effects
                        .iter()
                        .map(|ty| self.thread_effect_by_ty(ty))
                        .collect();
                    // [rs-effect-fusion] The machine's advance is generated
                    // under the same fused convention as any callee.
                    if self.fusion {
                        hs.dedup();
                        hs.truncate(1);
                    }
                    let hs = hs.join(", ");
                    self.union_sizes.insert(2);
                    self.needs_protocol = true;
                    // The machine's type: rustc cannot infer the closure's
                    // parameter through the `&mut dyn FnMut` coercion, and the
                    // mint at this call is what names it.
                    let ann = match self.mint_machines.last() {
                        Some(machine) => format!("__p: &mut {machine}"),
                        None => "__p".to_string(),
                    };
                    // [rs-fn-field] A callee that *keeps* the adapter takes it
                    // owned and `'static`, like any stored callback.
                    let lead = if producer { "move " } else { "&mut " };
                    out.push(format!(
                        "{lead}|{ann}| match __p.__advance({hs}) {{ \
                         Some(__v) => Union2::<{elem}, Finished>::U1(__v), \
                         None => Union2::<{elem}, Finished>::U2(Finished {{}}) }}"
                    ));
                }
            }
        }
        out
    }

    fn emit_fn_call(
        &mut self,
        name: &str,
        f: &FnDecl,
        key: Option<salvo_core::FnKey>,
        args: &[&Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        let mut all: Vec<String> = Vec::new();
        // [fn-effects] A producer call mints a pass and runs none of the
        // body, so it takes no handlers — even though `call_effects` records
        // the claim at this span when the call is a `for` subject: that entry
        // is the *drive* site's, and the drive site is where it is read.
        match self
            .checked
            .call_effects
            .get(&(self.file_idx, span))
            .cloned()
        {
            Some(effs) if effs.iter().all(ty_is_concrete) => {
                for ty in &effs {
                    // [rs-throw-controlflow] Throwing is a return shape,
                    // not a capability the caller hands over.
                    if is_throw_effect_ty(ty) {
                        continue;
                    }
                    all.push(self.thread_effect_by_ty(ty));
                }
            }
            _ => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                        if r.name.name == salvo_core::THROW_EFFECT {
                            continue;
                        }
                        let ty = self.emit_type_ref(r);
                        all.push(self.thread_effect_by_key(&ty));
                    }
                }
            }
        }
        // [rs-effect-fusion] The callee takes *one* fused value, whatever
        // the size of its effect set: every effect in scope threads through
        // the same value, so the per-effect arguments collapse to one. A
        // Sized fused value unsizes to the callee's `dyn` when the callee
        // needs fewer effects.
        if self.fusion {
            all.dedup();
            all.truncate(1);
        }
        // [spawn-inherit] [rs-handle-bundle] The callee's hidden handle
        // bundle, right after the fused effect value: built from the handles
        // this frame holds — its own bundle's fields, or the eager handle
        // variables its bindings minted.
        if self.fusion {
            if let Some(key) = key {
                if let Some(bundle) = self.handle_bundle_arg(key) {
                    all.push(bundle);
                }
            }
        }
        // [yield-proj] Which implicit `next`s filled here emit a *borrow*
        // (`Emitted (proj T)`): the checker's element type has the borrow
        // stripped, so the type argument the combinator is instantiated at
        // must be respelled `&T` — `map::<ListYield<String>, &String, i32>`.
        // The element generic is the one the spread's `Yield<It, T>` binds
        // second.
        let borrowed_elem_generics: Vec<String> = self
            .checked
            .implicit_args
            .get(&(self.file_idx, span))
            .map(|filled| {
                filled
                    .iter()
                    .filter_map(|a| match a {
                        salvo_core::ImplicitArg::Resolved { key, .. }
                        | salvo_core::ImplicitArg::OriginNext { next_fn: key, .. } => {
                            self.fn_by_key(*key)
                        }
                        _ => None,
                    })
                    .filter(|d| d.return_type.as_ref().is_some_and(type_has_proj))
                    .filter_map(|_| self.yield_element_generic(f))
                    .collect()
            })
            .unwrap_or_default();
        let saved_retag =
            std::mem::replace(&mut self.retagged_generics, borrowed_elem_generics.clone());
        let outer_mints = std::mem::take(&mut self.pending_mints);
        let outer_machines = std::mem::take(&mut self.mint_machines);
        let (mut prelude, args) = {
            let rendered = self.emit_args_for_params(&f.params, args, key);
            if all.is_empty() {
                (Vec::new(), rendered)
            } else {
                let threaded = all.clone();
                self.hoist_effect_args(Some(&threaded), rendered)
            }
        };
        all.extend(args);
        // [implicit-resolve] The implicit parameters, in the callee's order:
        // ordinary trailing arguments of fn type.
        let implicit_args = self.emit_implicit_args(named, span);
        let has_implicits = !implicit_args.is_empty();
        if !implicit_args.is_empty() {
            // [effect-args-hoisted] An argument that reborrows an implicit
            // *this* call also passes would borrow it twice (`E0499`), so it
            // is hoisted into a `let` first — the same rule, and the same
            // fix, as for a threaded effect value.
            let (extra, hoisted) = self.hoist_reborrows(&implicit_args, all);
            prelude.extend(extra);
            all = hoisted;
            all.extend(implicit_args);
        }
        // A call through an import alias keeps the alias [rs-imports]; a
        // [fn-rename] rename is erased instead, so the call spells the
        // declaration's own (mangled) name. The checker says which it is.
        let renamed = self.checked.renamed_calls.contains(&(self.file_idx, span));
        let mut rs_name = if name != f.name.name && !renamed {
            rs_ident(name)
        } else {
            self.rust_fn_name(f)
        };
        // [rs-shadowed-call] [fn-overload-at] A **local of the same name** shadows the function
        // in Rust's value namespace (E0618: "call expression requires
        // function"), where Kotlin keeps the two in separate namespaces. Such
        // a call is only reachable through `f@module(...)` — a plain call
        // would have gone through the local — so it is spelled as a path.
        if self.bindings.contains_key(name) {
            rs_name = format!("{}::{rs_name}", self.fn_module_path(f));
        }
        // [rs-implicit-turbofish] A **generic call that fills implicit
        // parameters** gets its type arguments spelled out. Each implicit
        // arrives as an adapter *closure* whose parameter types Rust infers
        // from the callee's bound — so with the callee's generics still open
        // there is nothing to infer them from, and inference stalls (E0282,
        // "type must be known at this point") on closures that are themselves
        // waiting on the answer. The checker already resolved the
        // instantiation [call-type-args], so the call states it.
        //
        // Nothing hit this before I5: `map`/`filter`/`reduce` over a list take
        // their `List` fast path, so the *generic* `?Iterable` body had never
        // been called with a subject whose element type only the adapters
        // could determine.
        if has_implicits && !f.generics.is_empty() {
            if let Some(args) = self
                .checked
                .call_type_args
                .get(&(self.file_idx, span))
                .cloned()
                .filter(|args| args.len() == f.generics.len() && args.iter().all(ty_is_concrete))
            {
                let mut rendered: Vec<String> = args.iter().map(|t| self.rust_ty(t)).collect();
                for g in &borrowed_elem_generics {
                    if let Some(i) = f.generics.iter().position(|x| x.name == *g) {
                        // [proj-type] Already a projection in the checker's
                        // type (`T = proj Str` renders `&String`): nothing to
                        // retag.
                        if args.get(i).is_some_and(|t| t.is_proj()) {
                            continue;
                        }
                        if let Some(r) = rendered.get_mut(i) {
                            *r = format!("&{r}");
                        }
                    }
                }
                // [iter-fn] Where an argument is an **origin**, the
                // value is the hidden machine, so the type argument is the
                // machine's — the origin struct itself is only the recipe.
                for machine in &self.mint_machines {
                    if let Some(origin) = machine.strip_prefix("__Pass_") {
                        for r in rendered.iter_mut() {
                            if r == origin {
                                *r = machine.clone();
                            }
                        }
                    }
                }
                rs_name = format!("{rs_name}::<{}>", rendered.join(", "));
            }
        }
        let call = Self::wrap_hoisted(&prelude, format!("{rs_name}({})", all.join(", ")));
        // [iter-fn] A minted pass is released here: the mint is the
        // compiler's value, so a combinator that abandons it early cannot
        // leak it. `__close` is idempotent, so a drained pass pays nothing.
        self.retagged_generics = saved_retag;
        self.mint_machines = outer_machines;
        let mints = std::mem::replace(&mut self.pending_mints, outer_mints);
        if mints.is_empty() {
            return call;
        }
        let ctors: String = mints
            .iter()
            .map(|(_, ctor, _)| format!("{ctor} "))
            .collect();
        let closes: String = mints
            .iter()
            .map(|(_, _, close)| format!("{close} "))
            .collect();
        format!("{{ {ctors}let __call = {call}; {closes}__call }}")
    }

    // ================= effect environment =================

    /// [rs-effect-fusion] The variable every effect in scope threads
    /// through under the fusion: the current scope's fusion local, or the
    /// enclosing fn's fused parameter.
    fn fused_var(&self) -> Option<(String, bool)> {
        if !self.fusion {
            return None;
        }
        self.effect_env.last().map(|e| (e.var.clone(), e.is_local))
    }

    /// The receiver expression for the fused value: a fresh reborrow, so
    /// the value stays usable afterwards.
    fn fused_recv(&self) -> Option<String> {
        self.fused_var().map(|(var, is_local)| {
            if is_local {
                format!("&mut {var}")
            } else {
                format!("&mut *{var}")
            }
        })
    }

    /// [rs-effect-fusion] An argument that itself reaches the fused value
    /// must be evaluated *before* the call borrows it: under the fusion one
    /// value carries every effect, so `fx.a(&fx.b())` is two overlapping
    /// `&mut` (`E0499`) where the per-effect parameters were disjoint.
    fn hoist_fused_args(&mut self, args: Vec<String>) -> (Vec<String>, Vec<String>) {
        self.hoist_effect_args(None, args)
    }

    /// [rs-effects] Hoists the arguments that reach an effect value *this
    /// call threads* into a prelude, so an effectful call inside an
    /// effectful call's arguments does not borrow the same `&mut dyn`
    /// parameter twice (`E0499`) — a shape Kotlin accepts and Rust rejects.
    /// `threaded` is the call's own effect-argument code; `None` means every
    /// effect in scope (the fusion's single value, or a constructor whose
    /// arguments reach the provider).
    fn hoist_effect_args(
        &mut self,
        threaded: Option<&[String]>,
        args: Vec<String>,
    ) -> (Vec<String>, Vec<String>) {
        let mut vars: Vec<String> = match self.fused_var() {
            Some((var, _)) => vec![var],
            None => Vec::new(),
        };
        for entry in &self.effect_env {
            let used = match threaded {
                // The effect value is threaded by this very call.
                Some(codes) => codes.iter().any(|c| mentions_ident(c, &entry.var)),
                None => true,
            };
            if used && !vars.contains(&entry.var) {
                vars.push(entry.var.clone());
            }
        }
        if vars.is_empty() {
            return (Vec::new(), args);
        }
        let mut prelude: Vec<String> = Vec::new();
        let mut out: Vec<String> = Vec::new();
        for code in args {
            if vars.iter().any(|var| mentions_ident(&code, var)) {
                self.hoist_id += 1;
                let name = format!("__a{}", self.hoist_id);
                prelude.push(format!("let {name} = {code};"));
                out.push(name);
            } else {
                out.push(code);
            }
        }
        (prelude, out)
    }

    /// [effect-args-hoisted] Hoists any argument whose code mentions one of
    /// `passed` — the implicit values this same call hands over — so the
    /// argument's borrow ends before the call takes its own.
    fn hoist_reborrows(
        &mut self,
        passed: &[String],
        args: Vec<String>,
    ) -> (Vec<String>, Vec<String>) {
        let names: Vec<String> = passed
            .iter()
            .filter_map(|code| {
                code.strip_prefix("&mut *")
                    .map(|rest| rest.trim().to_string())
            })
            .collect();
        if names.is_empty() {
            return (Vec::new(), args);
        }
        let mut prelude = Vec::new();
        let mut out = Vec::new();
        for code in args {
            if names.iter().any(|n| mentions_ident(&code, n)) {
                self.hoist_id += 1;
                let name = format!("__a{}", self.hoist_id);
                prelude.push(format!("let {name} = {code};"));
                out.push(name);
            } else {
                out.push(code);
            }
        }
        (prelude, out)
    }

    /// Wraps a call whose arguments were hoisted in a block expression, so
    /// it stays usable in expression position.
    fn wrap_hoisted(prelude: &[String], call: String) -> String {
        if prelude.is_empty() {
            call
        } else {
            format!("{{ {} {call} }}", prelude.join(" "))
        }
    }

    /// The expression a member call dispatches through for an effect
    /// instance (local handler variables and `&mut dyn` parameters both
    /// auto-reborrow on method calls) [rs-effects]. Under the fusion,
    /// dispatch is UFCS, so the receiver is spelled out as an explicit
    /// reborrow [rs-effect-fusion].
    fn member_dispatch_by_ty(&mut self, ty: &Ty) -> String {
        match self.effect_entry_by_ty(ty) {
            Some(entry) => self.entry_recv(&entry),
            None => {
                self.error(format!(
                    "no handler for effect `{ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// The receiver/threading expression for one environment entry.
    fn entry_recv(&self, entry: &EffectEntry) -> String {
        if !self.fusion {
            return entry.var.clone();
        }
        if entry.is_local {
            format!("&mut {}", entry.var)
        } else {
            format!("&mut *{}", entry.var)
        }
    }

    /// Threading variant of [`Self::member_dispatch_by_ty`].
    fn thread_effect_by_ty(&mut self, ty: &Ty) -> String {
        match self.effect_entry_by_ty(ty) {
            Some(entry) => {
                if self.fusion {
                    self.entry_recv(&entry)
                } else if entry.is_local {
                    format!("&mut {}", entry.var)
                } else {
                    entry.var
                }
            }
            None => {
                self.error(format!(
                    "no handler for effect `{ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    fn member_dispatch_by_key(&mut self, effect_ty: &str) -> String {
        match self.effect_entry(effect_ty) {
            Some(entry) => self.entry_recv(&entry),
            None => {
                self.error(format!(
                    "no handler for effect `{effect_ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// The expression that threads an effect instance as a `&mut dyn`
    /// argument: `&mut local` for `use` locals, the parameter itself
    /// (implicit reborrow) otherwise [rs-effects].
    fn thread_effect_by_key(&mut self, effect_ty: &str) -> String {
        match self.effect_entry(effect_ty) {
            Some(entry) => {
                if self.fusion {
                    self.entry_recv(&entry)
                } else if entry.is_local {
                    format!("&mut {}", entry.var)
                } else {
                    entry.var
                }
            }
            None => {
                self.error(format!(
                    "no handler for effect `{effect_ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// [use-no-dup] Do two environment entries name the same effect
    /// *instance*? The checker's type is the primary key, as everywhere
    /// else; entries that exist only as AST renderings compare by their
    /// rendered key. Used to hide a registration another one shadows.
    fn same_instance(&self, a: &EffectEntry, b: &EffectEntry) -> bool {
        match (&a.ty, &b.ty) {
            (Some(x), Some(y)) => x == y,
            _ => a.key == b.key,
        }
    }

    /// Exact-key lookup, then a unique same-base-name fallback (generic
    /// callee effects like `Random<T>` against a concrete `Random<i32>`).
    fn effect_entry(&self, effect_ty: &str) -> Option<EffectEntry> {
        // Innermost first, as in `effect_entry_by_ty` [fn-effects].
        if let Some(entry) = self.effect_env.iter().rev().find(|e| e.key == effect_ty) {
            return Some(entry.clone());
        }
        let base = effect_ty.split('<').next().unwrap_or(effect_ty);
        let matches: Vec<&EffectEntry> = self
            .effect_env
            .iter()
            .rev()
            .filter(|e| e.key.split('<').next().unwrap_or(&e.key) == base)
            .collect();
        if matches.len() == 1 {
            Some(matches[0].clone())
        } else if matches
            .first()
            .is_some_and(|first| matches.iter().all(|e| e.key == first.key))
        {
            // The same effect twice: an inner scope (a `use`, or a lambda's
            // effect parameter [fn-effects]) shadowing an outer entry. The
            // innermost wins; genuine ambiguity — *different* instances of a
            // generic effect — stays an error [effect-disambiguation].
            matches.first().copied().cloned()
        } else {
            None
        }
    }

    /// Lookup by the *checker's* effect type — the primary,
    /// rendering-drift-immune path; falls back to the rendered key for
    /// entries that only exist as AST renderings.
    fn effect_entry_by_ty(&mut self, ty: &Ty) -> Option<EffectEntry> {
        // Innermost first: a lambda's own effect *parameters* shadow the
        // enclosing fn's, which is what keeps the closure from capturing
        // them [fn-effects].
        if let Some(entry) = self
            .effect_env
            .iter()
            .rev()
            .find(|e| e.ty.as_ref() == Some(ty))
        {
            return Some(entry.clone());
        }
        let rendered = self.rust_ty(ty);
        self.effect_entry(&rendered)
    }

    /// Fallback dispatch for unchecked effect-member calls: unique
    /// base-name match (with explicit type args when given).
    fn member_dispatch_fallback(&mut self, effect: &str, type_args: &[Type]) -> String {
        let key = self.member_dispatch_key(effect, type_args);
        self.member_dispatch_by_key(&key)
    }

    /// The environment key an unchecked effect-member call looks up.
    fn member_dispatch_key(&mut self, effect: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            return effect.to_string();
        }
        let args: Vec<String> = type_args.iter().map(|t| self.emit_type(t)).collect();
        format!("{effect}<{}>", args.join(", "))
    }

    /// The effect base name and rendered type arguments of a checker `Ty`.
    fn ty_effect_parts(&mut self, ty: &Ty) -> (String, Vec<String>) {
        match ty {
            Ty::Named { name, args } => {
                let name = name.clone();
                let args = args.clone();
                let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
                (name, rendered)
            }
            other => {
                let rendered = self.rust_ty(other);
                split_rendered_generic(&rendered)
            }
        }
    }
}

// ================= helpers =================

/// Wraps a condition in parens when it starts with a block (Rust parses
/// `while { .. } < x` ambiguously) [rs-inc-dec].
fn cond_code(code: String) -> String {
    if code.starts_with('(') || code.starts_with('{') {
        format!("({code})")
    } else {
        code
    }
}

/// Whether an AST type is a fn type (possibly under qualifiers like
/// `once`): such types render as `impl Fn…`, which Rust only allows in
/// parameter/return position — never on a `let` binding [fn-contract].
fn is_fn_type(ty: &Type) -> bool {
    match ty {
        Type::Fn { .. } => true,
        Type::QualifiedGroup { base, .. } => is_fn_type(base),
        _ => false,
    }
}

/// Operator precedence for parenthesization (higher binds tighter).
fn bin_prec(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Lt
        | BinaryOp::Gt
        | BinaryOp::LtEq
        | BinaryOp::GtEq => 3,
        BinaryOp::Add | BinaryOp::Sub => 4,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => 5,
    }
}

/// [proj-anywhere] The `from` lists of every `proj` in a written type that
/// names sources, in order of appearance.
fn proj_refs_with_from(ty: &Type) -> Vec<Vec<String>> {
    fn in_ref(r: &salvo_syntax::ast::TypeRef, out: &mut Vec<Vec<String>>) {
        if r.name.name == "proj" && !r.from.is_empty() {
            out.push(r.from.iter().map(|i| i.name.clone()).collect());
        }
        for a in &r.args {
            walk(a, out);
        }
    }
    fn walk(ty: &Type, out: &mut Vec<Vec<String>>) {
        match ty {
            Type::Named { qualifiers, base } => {
                for q in qualifiers {
                    in_ref(q, out);
                }
                in_ref(base, out);
            }
            Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                for q in qualifiers {
                    in_ref(q, out);
                }
                walk(base, out);
            }
            Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
                for a in arms {
                    walk(a, out);
                }
            }
            Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => walk(elem, out),
            Type::Fn { .. } => {}
        }
    }
    let mut out = Vec::new();
    walk(ty, &mut out);
    out
}

/// [proj-type] The type under a *top-level* `proj` (on a named type or a
/// qualified group), or `None` when the type is not a projection at its
/// top. A Copy scalar under `proj` is the scalar itself [copy-scalar-free].
fn strip_top_proj_ast(ty: &Type) -> Option<Type> {
    let is_scalar = |t: &Type| {
        matches!(
            t,
            Type::Named { qualifiers, base }
                if qualifiers.is_empty()
                    && base.args.is_empty()
                    && matches!(
                        base.name.name.as_str(),
                        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                    )
        )
    };
    match ty {
        Type::Named { qualifiers, base } if qualifiers.iter().any(|q| q.name.name == "proj") => {
            let rest: Vec<salvo_syntax::ast::TypeRef> = qualifiers
                .iter()
                .filter(|q| q.name.name != "proj")
                .cloned()
                .collect();
            let inner = Type::Named {
                qualifiers: rest,
                base: base.clone(),
            };
            if is_scalar(&inner) {
                None
            } else {
                Some(inner)
            }
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } if qualifiers.iter().any(|q| q.name.name == "proj") => {
            let rest: Vec<salvo_syntax::ast::TypeRef> = qualifiers
                .iter()
                .filter(|q| q.name.name != "proj")
                .cloned()
                .collect();
            if rest.is_empty() {
                Some((**base).clone())
            } else {
                Some(Type::QualifiedGroup {
                    qualifiers: rest,
                    base: base.clone(),
                    span: *span,
                })
            }
        }
        _ => None,
    }
}

/// A written type's base name, for diagnostics.
fn inner_name(ty: &Type) -> String {
    match ty {
        Type::Named { base, .. } => base.name.name.clone(),
        Type::QualifiedGroup { base, .. } => inner_name(base),
        other => format!("{other:?}"),
    }
}

fn binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::LtEq => "<=",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

/// The base type name of an AST type, used to disambiguate overloads in
/// unchecked contexts.
/// The (kept, mutable) contract of one fn-type parameter [fn-contract]:
/// kept unless the written deduction list omits its name; mutable when
/// the declared type carries `Mut`. Defaults keep everything.
fn ast_fn_param_contract(
    params: &[Type],
    param_names: &[Option<salvo_syntax::ast::Ident>],
    deductions: &Option<Vec<salvo_syntax::ast::Deduction>>,
    i: usize,
) -> (bool, bool) {
    let is_mut = params
        .get(i)
        .map(|t| match t {
            Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                qualifiers.iter().any(|q| q.name.name == "Mut")
            }
            _ => false,
        })
        .unwrap_or(false);
    // [deduce-syntax] Unmentioned in a fn type's group is kept; only a
    // written move (`!x`) consumes.
    let kept = match (param_names.get(i).and_then(|n| n.as_ref()), deductions) {
        (Some(name), Some(list)) => !list.iter().any(|d| {
            d.param_name().is_some_and(|n| n.name == name.name)
                && matches!(
                    d.kind,
                    salvo_syntax::ast::DeductionKind::Moved
                        | salvo_syntax::ast::DeductionKind::Deferred
                )
        }),
        _ => true,
    };
    (kept, is_mut)
}
fn is_fn_group(ty: &Type) -> bool {
    matches!(
        ty,
        Type::QualifiedGroup { base, .. } if matches!(base.as_ref(), Type::Fn { .. })
    )
}

fn type_base_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => type_base_name(inner),
        Type::QualifiedGroup { base, .. } => type_base_name(base),
        Type::Array { .. } => Some("[]"),
        _ => None,
    }
}

/// The base type name of a *checker* type, aligned with
/// [`type_base_name`]'s conventions so the two are comparable.
fn ty_base_name(ty: &Ty) -> Option<&str> {
    match ty {
        Ty::Named { name, .. } => Some(name),
        Ty::Qualified { base, .. } => ty_base_name(base),
        Ty::Array(_) => Some("[]"),
        Ty::Union(_) => {
            // `T?` compares as its value arm (AST `Nullable` does too).
            let arms = ty.value_arms();
            if arms.len() == 1 {
                ty_base_name(arms[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Narrows same-arity unchecked-call candidates by comparing the checked
/// argument types' base names against the declared parameter types.
/// `None` unless exactly one candidate survives — ambiguous dispatch must
/// error, never guess [backend-never-wrong].
fn disambiguate_unchecked<'p, T: Copy>(
    em: &Emitter<'p>,
    candidates: &[T],
    params: impl Fn(T) -> &'p [Param],
    args: &[&Expr],
) -> Option<T> {
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let survivors: Vec<T> = candidates
        .iter()
        .copied()
        .filter(|c| {
            params(*c).iter().zip(args).all(|(p, a)| {
                let Some(pb) = type_base_name(&p.ty) else {
                    return true;
                };
                let Some(at) = em.ty_of(a.span()) else {
                    return true;
                };
                match ty_base_name(at) {
                    Some(ab) => pb == ab,
                    None => true,
                }
            })
        })
        .collect();
    if survivors.len() == 1 {
        Some(survivors[0])
    } else {
        None
    }
}

/// Whether an AST type carries the `Mut` qualifier [rs-borrows].
fn type_has_mut(ty: &Type) -> bool {
    match ty {
        Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().any(|q| q.name.name == "Mut")
        }
        Type::Nullable { inner, .. } => type_has_mut(inner),
        _ => false,
    }
}

/// [rs-narrow-mut] Whether `Mut` is reachable in a type *by peeling a union
/// arm* — `Ok Mut List<Int> | Err Str` as well as `Mut List<Int>` itself.
///
/// Only the owned parameter's `mut` binder reads this, and only to widen it:
/// a peel through the mutable accessors borrows the storage, which a non-`mut`
/// binder refuses (E0596), and a spurious `mut` costs nothing (`unused_mut` is
/// allowed in generated code). The *mode* deliberately keeps `type_has_mut`,
/// since an arm's `Mut` is not a claim about the parameter — see the open
/// defect note in ROADMAP.md.
fn type_has_mut_arm(ty: &Type) -> bool {
    if type_has_mut(ty) {
        return true;
    }
    match ty {
        Type::Union { arms, .. } => arms.iter().any(type_has_mut_arm),
        Type::Nullable { inner, .. } => type_has_mut_arm(inner),
        _ => false,
    }
}

/// True when a checker type contains no `Unknown`.
fn ty_is_concrete(ty: &Ty) -> bool {
    match ty {
        Ty::Unknown => false,
        Ty::Named { args, .. } => args.iter().all(ty_is_concrete),
        Ty::Qualified { base, .. } => ty_is_concrete(base),
        Ty::Union(arms) => arms.iter().all(ty_is_concrete),
        Ty::Tuple(elems) => elems.iter().all(ty_is_concrete),
        Ty::Array(elem) => ty_is_concrete(elem),
        Ty::Fn { params, ret, .. } => params.iter().all(ty_is_concrete) && ty_is_concrete(ret),
        _ => true,
    }
}

/// A parameter-ish name derived from a rendered effect type
/// (`Random<i32>` -> `random_i32`).
fn effect_param_name(effect_ty: &str) -> String {
    let mut out = String::new();
    for c in effect_ty.chars() {
        match c {
            '<' | ',' => out.push('_'),
            '>' | ' ' | '?' | '&' => {}
            c if c.is_uppercase() => {
                if out
                    .chars()
                    .last()
                    .is_some_and(|p| p.is_lowercase() || p.is_ascii_digit())
                {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            }
            c => out.push(c),
        }
    }
    out
}

/// A rendered type turned into an identifier fragment for a generated
/// name (`Random<i32>` -> `Random_i32`) [rs-effect-fusion].
fn sanitize_ident(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Splits a rendered generic type into its base and top-level arguments
/// (`Random<List<i32>, u8>` -> `("Random", ["List<i32>", "u8"])`). The
/// unchecked fallback for effect environment entries with no checker `Ty`.
fn split_rendered_generic(rendered: &str) -> (String, Vec<String>) {
    let Some(open) = rendered.find('<') else {
        return (rendered.to_string(), Vec::new());
    };
    let base = rendered[..open].to_string();
    let inner = rendered[open + 1..]
        .strip_suffix('>')
        .unwrap_or(&rendered[open + 1..]);
    let mut args: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '<' => {
                depth += 1;
                current.push(c);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current = String::new();
            }
            c => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    (base, args)
}

fn escape_string(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// `format!` literal text additionally escapes braces.
fn escape_format_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn escape_char(c: char) -> String {
    match c {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        c => c.to_string(),
    }
}

fn is_none_type(ty: &Type) -> bool {
    matches!(ty, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == "None")
}

/// The qualifier names on a fn's parameter types, joined for
/// overload-mangling suffixes [rs-fn-mangling].
fn qual_suffix(decl: &FnDecl) -> String {
    let mut parts: Vec<String> = Vec::new();
    for p in &decl.params {
        match &p.ty {
            Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                for q in qualifiers {
                    // Dot-names canonicalize to their flattened spelling:
                    // a mangled fn name is a single identifier [name-dot].
                    parts.push(q.name.name.replace('.', ""));
                }
            }
            _ => {}
        }
    }
    parts.join("_")
}

/// Substitutes generic parameters in an AST type (alias expansion).
fn subst_ast_type(ty: &Type, map: &HashMap<&str, &Type>) -> Type {
    match ty {
        Type::Named { qualifiers, base } => {
            if qualifiers.is_empty() && base.args.is_empty() {
                if let Some(replacement) = map.get(base.name.name.as_str()) {
                    return (*replacement).clone();
                }
            }
            Type::Named {
                qualifiers: qualifiers.clone(),
                base: TypeRef {
                    at: None,
                    binder: false,
                    alias: None,
                    name: base.name.clone(),
                    args: base.args.iter().map(|a| subst_ast_type(a, map)).collect(),
                    from: base.from.clone(),
                    span: base.span,
                },
            }
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } => Type::QualifiedGroup {
            qualifiers: qualifiers.clone(),
            base: Box::new(subst_ast_type(base, map)),
            span: *span,
        },
        Type::Union { arms, span } => Type::Union {
            arms: arms.iter().map(|a| subst_ast_type(a, map)).collect(),
            span: *span,
        },
        Type::Tuple { elems, span } => Type::Tuple {
            elems: elems.iter().map(|e| subst_ast_type(e, map)).collect(),
            span: *span,
        },
        Type::Array { elem, span } => Type::Array {
            elem: Box::new(subst_ast_type(elem, map)),
            span: *span,
        },
        Type::Nullable { inner, span } => Type::Nullable {
            inner: Box::new(subst_ast_type(inner, map)),
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
            params: params.iter().map(|p| subst_ast_type(p, map)).collect(),
            param_names: param_names.clone(),
            effects: effects.clone(),
            deductions: deductions.clone(),
            ret: Box::new(subst_ast_type(ret, map)),
            span: *span,
        },
    }
}

/// Names assigned or incremented anywhere in the block (reassigned
/// parameters need a `mut` binder [rs-borrows]).
fn collect_mutated(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Assign { target, value, .. } => {
                if let Expr::Ident(id) = target {
                    out.insert(id.name.clone());
                }
                collect_mutated_expr(target, out);
                collect_mutated_expr(value, out);
            }
            Stmt::Let { value, .. } => collect_mutated_expr(value, out),
            Stmt::Use { handler, .. } => collect_mutated_expr(handler, out),
            Stmt::Expr(e) => collect_mutated_expr(e, out),
            _ => {}
        }
    }
}

fn collect_mutated_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        // [assert-fn] A condition or a message may mutate, like any expression.
        Expr::Assert { cond, message, .. } => {
            collect_mutated_expr(cond, out);
            if let Some(m) = message {
                collect_mutated_expr(m, out);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                collect_mutated_expr(m, out);
            }
        }
        // [elvis] Both sides may mutate.
        Expr::Elvis { subject, rhs, .. } => {
            collect_mutated_expr(subject, out);
            collect_mutated_expr(rhs, out);
        }
        Expr::SafeField { inner, .. } => collect_mutated_expr(inner, out),
        Expr::Placeholder { .. } => {}
        // [expr-escape] The escapes carry a value expression.
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_mutated_expr(v, out);
            }
        }
        Expr::Continue { .. } => {}
        Expr::IncDec { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
        }
        // [fn-overload-at] Only the dot-notation receiver is an expression.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(base) = base {
                collect_mutated_expr(base, out);
            }
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (cond, block) in branches {
                collect_mutated_expr(cond, out);
                collect_mutated(block, out);
            }
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_mutated_expr(cond, out);
            collect_mutated(body, out);
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_mutated_expr(iterable, out);
            collect_mutated(body, out);
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_mutated_expr(subject, out);
            for b in branches {
                collect_mutated(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (cond, block) in branches {
                collect_mutated_expr(cond, out);
                collect_mutated(block, out);
            }
            collect_mutated(else_block, out);
        }
        Expr::Call { callee, args, .. } => {
            collect_mutated_expr(callee, out);
            for a in args {
                collect_mutated_expr(a, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_mutated_expr(lhs, out);
            collect_mutated_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::Spread { operand, .. } => collect_mutated_expr(operand, out),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => collect_mutated_expr(base, out),
        Expr::Index { base, index, .. } => {
            collect_mutated_expr(base, out);
            collect_mutated_expr(index, out);
        }
        Expr::Is { subject, .. } => collect_mutated_expr(subject, out),
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_mutated_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_mutated_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_mutated_expr(k, out);
                collect_mutated_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_mutated_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_mutated_expr(e, out),
                }
            }
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(e) => collect_mutated_expr(e, out),
            LambdaBody::Block(b) => collect_mutated(b, out),
        },
        // [try] The delimiter's body is ordinary code: a variable mutated
        // only inside it still needs the mutable declaration.
        Expr::Try { body, .. } => collect_mutated(body, out),
        // [actor-spawn-expr] [actor-replyto] [actor-waitfor] The clauses,
        // captures and bridge block are ordinary code.
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            collect_mutated_expr(handler, out);
            for handler in with_items {
                collect_mutated_expr(handler, out);
            }
            if let Some(pool) = pool {
                collect_mutated_expr(pool, out);
            }
        }
        Expr::ReplyTo { captures, .. } => {
            for capture in captures {
                collect_mutated_expr(capture, out);
            }
        }
        // [actor-self-send] A leaf.
        Expr::SelfScoped { .. } => {}
        Expr::WaitFor { body, .. } => collect_mutated(body, out),
        // [qual-lift] The check reads its subject.
        Expr::Widen { subject, .. } => collect_mutated_expr(subject, out),
        // Leaves: no sub-expression, so nothing can be mutated inside.
        // Listed rather than defaulted, because a missed form emits an
        // immutable declaration for a variable the code assigns and the
        // target compiler is what reports it [backend-never-wrong].
        Expr::Ident(_)
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}

/// Every name a block binds (generated effect-parameter and `use`
/// variable names must avoid them).
fn collect_declared(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { pattern, value, .. } => {
                collect_pattern_names(pattern, out);
                collect_declared_expr(value, out);
            }
            Stmt::Assign { target, value, .. } => {
                collect_declared_expr(target, out);
                collect_declared_expr(value, out);
            }
            Stmt::Use { handler, .. } => collect_declared_expr(handler, out),
            Stmt::Expr(e) => collect_declared_expr(e, out),
            _ => {}
        }
    }
}

fn collect_pattern_names(pattern: &Pattern, out: &mut HashSet<String>) {
    match pattern {
        Pattern::Ident(id) => {
            out.insert(id.name.clone());
        }
        Pattern::Tuple { elems, .. } => {
            for p in elems {
                collect_pattern_names(p, out);
            }
        }
        Pattern::Struct { fields, .. } => {
            for f in fields {
                out.insert(f.binding.name.clone());
            }
        }
    }
}

fn collect_declared_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Assert { cond, message, .. } => {
            collect_declared_expr(cond, out);
            if let Some(m) = message {
                collect_declared_expr(m, out);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                collect_declared_expr(m, out);
            }
        }
        Expr::Elvis { subject, rhs, .. } => {
            collect_declared_expr(subject, out);
            collect_declared_expr(rhs, out);
        }
        Expr::SafeField { inner, .. } => collect_declared_expr(inner, out),
        Expr::Placeholder { .. } => {}
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_declared_expr(v, out);
            }
        }
        Expr::Continue { .. } => {}
        Expr::Is {
            subject, binding, ..
        } => {
            if let Some(b) = binding {
                out.insert(b.name.clone());
            }
            collect_declared_expr(subject, out);
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_declared_expr(c, out);
                collect_declared(b, out);
            }
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_declared_expr(subject, out);
            for b in branches {
                if let Some(binding) = &b.binding {
                    out.insert(binding.name.clone());
                }
                collect_declared(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_declared_expr(c, out);
                collect_declared(b, out);
            }
            collect_declared(else_block, out);
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_declared_expr(cond, out);
            collect_declared(body, out);
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::For {
            pattern,
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_pattern_names(pattern, out);
            collect_declared_expr(iterable, out);
            collect_declared(body, out);
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::Lambda { params, body, .. } => {
            for p in params {
                out.insert(p.name.name.clone());
            }
            match body {
                LambdaBody::Expr(e) => collect_declared_expr(e, out),
                LambdaBody::Block(b) => collect_declared(b, out),
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_declared_expr(callee, out);
            for a in args {
                collect_declared_expr(a, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_declared_expr(lhs, out);
            collect_declared_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::IncDec { operand, .. }
        | Expr::Spread { operand, .. } => collect_declared_expr(operand, out),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            collect_declared_expr(base, out)
        }
        // [fn-overload-at] Only the dot-notation receiver is an expression.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(base) = base {
                collect_declared_expr(base, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_declared_expr(base, out);
            collect_declared_expr(index, out);
        }
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_declared_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_declared_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_declared_expr(k, out);
                collect_declared_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_declared_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_declared_expr(e, out),
                }
            }
        }
        // [qual-lift] No binding of its own — the subject reads widened —
        // but the subject expression may declare one.
        Expr::Widen { subject, .. } => collect_declared_expr(subject, out),
        // [try] The delimiter's body is ordinary code and declares its own
        // locals.
        Expr::Try { body, .. } => collect_declared(body, out),
        // [actor-spawn-expr] [actor-replyto] The clauses and captures are
        // expressions, which may declare inside a nested body.
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            collect_declared_expr(handler, out);
            for handler in with_items {
                collect_declared_expr(handler, out);
            }
            if let Some(pool) = pool {
                collect_declared_expr(pool, out);
            }
        }
        Expr::ReplyTo { captures, .. } => {
            for capture in captures {
                collect_declared_expr(capture, out);
            }
        }
        // [actor-self-send] A leaf: it declares nothing.
        Expr::SelfScoped { .. } => {}
        // [actor-waitfor] The token binder is a declaration of its own, and
        // the block declares like any other.
        Expr::WaitFor { binding, body, .. } => {
            out.insert(binding.name.clone());
            collect_declared(body, out);
        }
        // Leaves: nothing declared inside. Listed rather than defaulted so
        // a new binding form cannot escape the name census that keeps
        // generated locals from colliding with user names.
        Expr::Ident(_)
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}

/// [qual-lift] Visits every `^` check a condition applies, including
/// through `&&` chains and a `!`-free `||` (the same shape `is` bindings
/// walk).
fn collect_widen_checks<'a>(cond: &'a Expr, f: &mut impl FnMut(&'a Expr, Span)) {
    match cond {
        // [qual-lift] A lift **with a binding** materializes the value into
        // that name instead, so there is no shadow to make: the binding is
        // emitted by the `is`-binding path, and a multi-arm lift narrows
        // nothing to shadow anyway.
        Expr::Widen {
            subject,
            binding: None,
            span,
            ..
        } => f(subject, *span),
        Expr::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
            ..
        } => {
            collect_widen_checks(lhs, f);
            collect_widen_checks(rhs, f);
        }
        _ => {}
    }
}

/// Whether an effect instance is the throw effect [throw]: it is not a
/// capability parameter, it is a return-shape change
/// [rs-throw-controlflow].
fn is_throw_effect_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT)
}

/// The message type of a `Throw<M>` effect instance [throw].
fn throw_message_of(ty: &Ty) -> Option<Ty> {
    match ty {
        Ty::Named { name, args } if name == salvo_core::THROW_EFFECT => {
            Some(args.first().cloned().unwrap_or(Ty::Unknown))
        }
        _ => None,
    }
}

/// [rs-exit-splice] Whether the block's own statements always leave it/// (`return`/`break`/`continue`, or a branching construct all of whose
/// branches do). Exit splices were already emitted at those exits, so
/// splicing them again at the block's end would be dead code — and, for a
/// body that gave a value away, dead code rustc still borrow-checks.
fn block_terminates(block: &Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Expr(e) => expr_terminates(e),
        _ => false,
    })
}

fn expr_terminates(expr: &Expr) -> bool {
    match expr {
        // [assert-fn] `unreachable!()` panics, so the statement after it is
        // unreachable — which is what this answers for `return` and `throw`.
        Expr::Unreachable { .. } => true,
        Expr::Assert { .. } => false,
        // [elvis] Neither side terminates the block by itself.
        Expr::Elvis { .. } | Expr::Placeholder { .. } | Expr::SafeField { .. } => false,
        // [expr-escape] All three leave the block they are written in.
        Expr::Return { .. } | Expr::Break { .. } | Expr::Continue { .. } => true,
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            else_block.as_ref().is_some_and(block_terminates)
                && branches.iter().all(|(_, b)| block_terminates(b))
        }
        Expr::When { branches, .. } => {
            !branches.is_empty() && branches.iter().all(|b| block_terminates(&b.body))
        }
        // [when-condition] Total by construction: the `else` is mandatory.
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => block_terminates(else_block) && branches.iter().all(|(_, b)| block_terminates(b)),
        // Nothing else terminates the enclosing Rust block by itself.
        // Listed rather than defaulted: a missed control-flow form would
        // get unreachable code emitted after it, which rustc rejects.
        Expr::While { .. }
        | Expr::For { .. }
        | Expr::Lambda { .. }
        | Expr::Try { .. }
        // [actor-spawn-expr] [actor-replyto] [actor-waitfor] Each has a
        // value and none diverges: a `waitfor` blocks and then continues.
        // [actor-self-send] A callee, hence a leaf.
        | Expr::SelfScoped { .. }
        | Expr::Spawn { .. }
        | Expr::ReplyTo { .. }
        | Expr::WaitFor { .. }
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Str { .. }
        | Expr::Ident(_)
        | Expr::Field { .. }
        | Expr::TupleIndex { .. }
        | Expr::Call { .. }
        | Expr::Index { .. }
        | Expr::ArrayLit { .. }
        | Expr::SetLit { .. }
        | Expr::MapLit { .. }
        | Expr::Tuple { .. }
        | Expr::StructLit { .. }
        | Expr::Unary { .. }
        | Expr::Binary { .. }
        | Expr::Is { .. }
        | Expr::Widen { .. }
        | Expr::NonNull { .. }
        | Expr::IncDec { .. }
        | Expr::Spread { .. }
        | Expr::Scoped { .. }
        | Expr::EffectScoped { .. }
        | Expr::Error { .. } => false,
    }
}

/// Walks a condition for `is`-checks with bindings.
/// [is-bind-once] Whether an expression is a **place** — a name or a
/// field/index/tuple chain over one — and so free of side effects to read
/// twice. Everything else (a call above all) has to be evaluated once into a
/// temporary before an `is` binding reads it.
/// [actor-mailbox] The `capacity` field's value expression inside a handler's
/// `mailbox { … }` slot — the slot *is* a struct literal, so this is one field
/// lookup rather than a new AST shape.
fn mailbox_capacity_expr(mailbox: &Expr) -> Option<&Expr> {
    let Expr::StructLit { fields, .. } = mailbox else {
        return None;
    };
    fields.iter().find_map(|f| match &f.kind {
        salvo_syntax::ast::StructLitFieldKind::Named { name, value } if name.name == "capacity" => {
            Some(value)
        }
        _ => None,
    })
}

fn is_place_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(_) => true,
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => is_place_expr(base),
        Expr::Index { base, index, .. } => is_place_expr(base) && is_place_expr(index),
        _ => false,
    }
}

fn collect_is_bindings<'a>(
    cond: &'a Expr,
    f: &mut impl FnMut(&'a Expr, &'a [TypeRef], &'a Ident, Span, bool),
) {
    match cond {
        Expr::Is {
            subject,
            check,
            binding: Some(b),
            span,
        } => f(subject, check, b, *span, false),
        // [qual-lift] `is ^Ok inner` binds too, at the *lifted* type. The
        // binding's own recorded type is what the read is built against, so
        // the same emission serves both — which is why the widen shadow is
        // only needed when there is no binding [rs-widen-shadow].
        Expr::Widen {
            subject,
            quals,
            binding: Some(b),
            span,
        } => f(subject, quals, b, *span, true),
        Expr::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
            ..
        } => {
            collect_is_bindings(lhs, f);
            collect_is_bindings(rhs, f);
        }
        _ => {}
    }
}

/// [rs-proj-struct] Whether a written type carries a `proj` anywhere.
fn type_has_proj(ty: &Type) -> bool {
    fn in_ref(r: &TypeRef) -> bool {
        r.name.name == "proj" || r.args.iter().any(type_has_proj)
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers.iter().any(in_ref) || in_ref(base),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(in_ref) || type_has_proj(base),
        Type::Nullable { inner, .. } | Type::Array { elem: inner, .. } => type_has_proj(inner),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().any(type_has_proj)
        }
        _ => false,
    }
}

/// [rs-proj-struct] The type with its outermost `proj` qualifier removed —
/// what a `&'s …` wraps.
fn strip_proj(ty: &Type) -> Type {
    match ty {
        Type::Named { qualifiers, base } => Type::Named {
            qualifiers: qualifiers
                .iter()
                .filter(|q| q.name.name != "proj")
                .cloned()
                .collect(),
            base: base.clone(),
        },
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } => {
            let quals: Vec<TypeRef> = qualifiers
                .iter()
                .filter(|q| q.name.name != "proj")
                .cloned()
                .collect();
            if quals.is_empty() {
                (**base).clone()
            } else {
                Type::QualifiedGroup {
                    qualifiers: quals,
                    base: base.clone(),
                    span: *span,
                }
            }
        }
        other => other.clone(),
    }
}
