//! The front half of emission both backends run before rendering anything
//! (ROADMAP §0j step 3): check the program, erase its effect-only generics,
//! re-resolve the erased copy, and work out which modules a build or a
//! platform root carries. What differs per backend — which modules become
//! files, their names, the files themselves — stays in the backend.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use salvo_core::{Checked, ModulePath, PlatformEffect, Program, Resolution, Symbols};

/// Checks `program` and answers its emission copy — effect-only generics
/// erased [effect-generic-decl] — with the checker's tables (keyed by span,
/// so they still apply to the copy), what was erased, and the rendered
/// warnings. Only
/// *errors* stop emission [diag-structured]; on that path every
/// diagnostic is rendered, warnings included, so nothing is lost while the
/// author fixes the errors.
pub fn check_for_emission(
    program: &Program,
) -> Result<(Program, salvo_core::Erased, Checked, Vec<String>), Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    if checked.errors.iter().any(|d| d.is_error()) {
        return Err(checked.errors.iter().map(|d| d.render(&program.files)).collect());
    }
    let warnings: Vec<String> = checked
        .errors
        .iter()
        .filter(|d| !d.is_error())
        .map(|d| d.render(&program.files))
        .collect();
    let erased = salvo_core::erased_generics(program);
    let erased_program = salvo_core::erase_effect_generics(program, &erased);
    Ok((erased_program, erased, checked, warnings))
}

/// The erased program's symbols and resolution, re-collected over the copy:
/// the emitters match declarations by address between the scopes and the
/// symbol table, so both must point into the same tree. Extends `checked`'s
/// written type references with the copy's [type-identity].
pub fn resolve_for_emission<'p>(
    program: &'p Program,
    checked: &mut Checked,
) -> (Symbols<'p>, Resolution<'p>) {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    checked
        .type_ref_keys
        .extend(salvo_core::resolve::written_keys(program, &resolution, &symbols));
    (symbols, resolution)
}

/// What a build, or a platform root in ABI mode, reaches.
pub struct Reach<'p> {
    /// [mod-used-only] The modules the program uses.
    pub reachable: HashSet<&'p ModulePath>,
    /// [platform-abi] In ABI mode, the kept declarations by name.
    pub closure: Option<BTreeSet<String>>,
    /// [platform-abi] The effects whose host-facing interface and adapter
    /// are emitted beside them.
    pub platform_effects: BTreeMap<String, PlatformEffect>,
    /// [runtime-sched] [stream-table] [platform-abi] In ABI mode, the modules
    /// a host project carries in full: the runtime module and what it reaches
    /// when the build runs actors, and the stream table's service.
    pub abi_full: HashSet<&'p ModulePath>,
    /// [platform-abi] In ABI mode, the modules written for the host: those
    /// declaring a kept name, the project's modules declaring platform items
    /// (its implementation file imports them even when nothing is kept), and
    /// the full ones.
    pub abi_modules: HashSet<&'p ModulePath>,
    /// Every module the backend writes: in a build the reached ones that
    /// produce code, in ABI mode the host-facing ones. One rule for both
    /// backends.
    pub emitted: HashSet<&'p ModulePath>,
}

pub fn reach<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &Symbols<'p>,
    checked: &Checked,
    abi: bool,
) -> Reach<'p> {
    let reachable = salvo_core::reachable_modules(program, resolution, checked);
    let closure = abi.then(|| salvo_core::abi::platform_closure(program, symbols));
    let platform_effects = salvo_core::platform_effects(program, |m| abi || reachable.contains(m));
    let mut abi_full: HashSet<&ModulePath> = match salvo_core::runtime_module(program) {
        Some(m) if abi && reachable.contains(m) => salvo_core::runtime_closure(program, resolution, checked),
        _ => HashSet::new(),
    };
    if abi {
        abi_full.extend(salvo_core::streams_closure(program, resolution, checked, &reachable));
    }
    let abi_modules: HashSet<&ModulePath> = program
        .units()
        .filter(|u| {
            closure.as_ref().is_some_and(|c| {
                u.ast.items.iter().any(|i| crate::emit_util::abi_item_name(i).is_some_and(|n| c.contains(n)))
                    || (crate::emit_util::is_project_module(program, &u.file.module)
                        && !salvo_core::platform_declarations(u.ast).is_empty())
            }) || (abi_full.contains(&u.file.module) && crate::emit_util::module_produces_code(u.ast))
        })
        .map(|u| &u.file.module)
        .collect();
    let emitted: HashSet<&ModulePath> = if abi {
        abi_modules.clone()
    } else {
        program
            .units()
            .filter(|u| reachable.contains(&u.file.module) && crate::emit_util::module_produces_code(u.ast))
            .map(|u| &u.file.module)
            .collect()
    };
    Reach {
        reachable,
        closure,
        platform_effects,
        abi_full,
        abi_modules,
        emitted,
    }
}

/// Checks a program for skeleton writing: errors only, never warnings —
/// `salvo platform generate` writes host stubs once, and the program's
/// diagnostics are the compile path's and `salvo analyze`'s to report.
pub fn check_for_skeletons<'p>(
    program: &'p Program,
) -> Result<(Symbols<'p>, Resolution<'p>, Checked), Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    if checked.errors.iter().any(|d| d.is_error()) {
        return Err(checked
            .errors
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.render(&program.files))
            .collect());
    }
    Ok((symbols, resolution, checked))
}
