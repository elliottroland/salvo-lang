//! The Rust backend's entry points and the naming and runtime-file helpers
//! its emitter shares. The emitter itself reads the IR (`ir_emit`,
//! [rs-ir]); rustc is the safety net: anything it gets wrong about
//! ownership fails to *compile*, never silently misbehaves
//! [backend-never-wrong].

use std::collections::{BTreeMap, BTreeSet, HashSet};

use salvo_core::{ModulePath, Program};

pub use salvo_backend::emit_util::*;

/// Crate-root lint allowances [rs-crate]: the generator does not fight
/// cosmetic lints (every local is `let mut`, Salvo naming is snake/camel
/// mixed, defensive code may be unreachable, a block expression may be
/// braced where none is needed).
pub(crate) const CRATE_ATTRS: &str = "#![allow(non_snake_case, non_camel_case_types, unused_mut, \
                           unused_parens, unused_imports, dead_code, unreachable_code, \
                           unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, \
                           non_upper_case_globals, unused_braces)]\n";

/// Emits Rust for every *reachable* module that produces code
/// [mod-used-only], plus the generated `unions.rs` and the crate-root
/// module header [rs-crate].
pub fn emit_program(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_with_entry(program, None)
}

/// [rs-crate] As [`emit_program`], with the crate root chosen explicitly:
/// `entry` names the module whose `main` is the program's entry point
/// (`salvo run --main`). `None` keeps the default — the first emitted module
/// that declares one.
///
/// Warnings are **dropped** here, which is the shape the golden tests want;
/// the driver calls [`emit_program_reporting`] and hands them to the user
/// [qual-refn-ambiguous].
pub fn emit_program_with_entry(program: &Program, entry: Option<&ModulePath>) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_reporting(program, entry).map(|(files, _warnings)| files)
}

/// [`emit_program_with_entry`] with the non-fatal diagnostics:
/// `(files, warnings)`, each warning rendered with its own `warning:` prefix
/// and location [diag-structured].
pub fn emit_program_reporting(program: &Program, entry: Option<&ModulePath>) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    crate::ir_emit::emit_program_ir(program, entry, false)
}

/// [platform-abi] [rs-abi] The declaration files of a platform root's host
/// crate: every struct, type alias and effect the platform surface reaches
/// (`salvo_core::abi::platform_closure`), emitted as the build emits them, the
/// runtime modules they need, and a crate root, `lib.sv.rs`, that mounts each
/// at the **same path it has in the build**, so an implementation file's
/// `crate::…` paths resolve alike in both. Never read by the build (ABI D4).
pub fn emit_abi(program: &Program, entry: Option<&ModulePath>) -> Result<Vec<EmittedFile>, Vec<String>> {
    crate::ir_emit::emit_program_ir(program, entry, true).map(|(files, _)| files)
}

/// [platform-tree] The host implementation skeletons, one file per project
/// module declaring platform types, fns or handlers.
pub fn platform_skeletons(program: &Program, entry: Option<&ModulePath>) -> Result<Vec<EmittedFile>, Vec<String>> {
    crate::ir_emit::platform_skeletons_ir(program, entry)
}

/// [platform-abi] The host-facing trait of an effect's platform handlers:
/// `EPlatform` (`&mut self`), or `EPlatformSync` (`&self`) for `threadsafe`.
pub(crate) fn platform_trait_name(effect: &str, threadsafe: bool) -> String {
    // [type-identity] A path a clashing name renders with stays in front.
    if let Some((path, last)) = split_path(effect) {
        return format!("{path}::{}", platform_trait_name(last, threadsafe));
    }
    if threadsafe {
        format!("{effect}PlatformSync")
    } else {
        format!("{effect}Platform")
    }
}

/// [platform-abi] The adapter of an effect (`__Platform_E<T>`) or of one
/// platform handler (`__Platform_H`): generated, never host-facing.
pub(crate) fn platform_adapter_name(name: &str) -> String {
    // [type-identity] A path a clashing name renders with stays in front.
    if let Some((path, last)) = split_path(name) {
        return format!("{path}::{}", platform_adapter_name(last));
    }
    format!("__Platform_{name}")
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
pub(crate) fn module_mod_names(
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
pub(crate) fn host_mod_name(module: &ModulePath) -> String {
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
pub(crate) fn relative_path(from_dir: &std::path::Path, target: &std::path::Path) -> String {
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
pub(crate) fn generate_strings_file() -> String {
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
pub(crate) fn generate_seq_file() -> String {
    include_str!("../runtime/seq.rs").to_string()
}

/// [rs-actor] The actor scheduler asynchronous effect handlers run on:
/// run-to-completion activations on pool threads, per-actor bounded queues,
/// reply tokens, the gate, death by faulted activation, and the
/// idle-with-parked-gates report. A **library**, not a runtime baked into
/// generated code (user decision 2026-09-15) — emitted only when a program
/// spawns. Source in `runtime/scheduler.rs`, included verbatim and compiled
/// directly by `runtime_tests.rs`.
pub(crate) fn generate_scheduler_file() -> String {
    include_str!("../runtime/scheduler.rs").to_string()
}

/// [time-types] [rs-time] The two clock readings `time`'s effects are built
/// on: the monotonic one (with the process-wide origin that makes two
/// readings comparable) and the wall-clock one. Emitted only when a program
/// reads time. Source in `runtime/hosttime.rs`.
pub(crate) fn generate_time_file() -> String {
    include_str!("../runtime/hosttime.rs").to_string()
}

/// [rs-host-abi] `UnionN` and its `U1`…`Un` variants are part of the host
/// ABI: host code builds and matches them, so their shape is a contract.
pub(crate) fn generate_unions_file(sizes: &BTreeSet<usize>, wire: bool) -> String {
    let mut out = String::from("// Generated by the Salvo compiler: enums for union types.\n");
    for &n in sizes {
        // [wire-format] [rs-wire] A union's codec: one tag byte holding the
        // arm's declared index, then the arm. Generated only when the wire
        // runtime is mounted, since the impl names it.
        if wire {
            let params: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
            let bounds: Vec<String> = (1..=n)
                .map(|i| format!("T{i}: crate::wire::__Wire"))
                .collect();
            let mut enc = String::new();
            let mut dec = String::new();
            for i in 1..=n {
                enc.push_str(&format!(
                    "            Union{n}::U{i}(v) => {{\n                out.push({});\n                \
                     crate::wire::__Wire::__enc(v, out);\n            }}\n",
                    i - 1
                ));
                dec.push_str(&format!(
                    "            {} => Some(Union{n}::U{i}(crate::wire::__Wire::__dec(r)?)),\n",
                    i - 1
                ));
            }
            out.push_str(&format!(
                "\nimpl<{}> crate::wire::__Wire for Union{n}<{}> {{\n    \
                 fn __enc(&self, out: &mut Vec<u8>) {{\n        match self {{\n{enc}        }}\n    }}\n    \
                 fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {{\n        \
                 match r.u8()? {{\n{dec}            _ => None,\n        }}\n    }}\n}}\n",
                bounds.join(", "),
                params.join(", ")
            ));
        }
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

/// [type-identity] A clashing name renders under its module
/// (`crate::lib_a::Clock`): the path and the name, or `None` for a bare one.
pub(crate) fn split_path(name: &str) -> Option<(&str, &str)> {
    let base_end = name.find('<').unwrap_or(name.len());
    let i = name[..base_end].rfind("::")?;
    Some((&name[..i], &name[i + 2..]))
}

pub(crate) fn rs_ident(name: &str) -> String {
    // [type-identity] A key is declared and spelled under the name it was
    // written as; its module path, where one is needed, comes from the
    // caller (`type_path`, `effect_path`).
    let name = salvo_core::typekey::plain(name);
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

/// [rs-handle] The trait a **stateless** handler implements: the effect's
/// members with `&self` receivers, shared through an `Arc` with no lock.
pub(crate) fn stateless_trait_name(effect: &str) -> String {
    // [type-identity] A path a clashing name renders with stays in front.
    if let Some((path, last)) = split_path(effect) {
        return format!("{path}::{}", stateless_trait_name(last));
    }
    format!("__Stateless_{}", rs_ident(effect))
}

/// [rs-handle] The trait a **stateful** handler implements: the effect's
/// members with `&mut self` receivers, reached through the handle's lock.
pub(crate) fn stateful_trait_name(effect: &str) -> String {
    // [type-identity] A path a clashing name renders with stays in front.
    if let Some((path, last)) = split_path(effect) {
        return format!("{path}::{}", stateful_trait_name(last));
    }
    format!("__Stateful_{}", rs_ident(effect))
}

pub(crate) fn escape_string(text: &str) -> String {
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
pub(crate) fn escape_format_text(text: &str) -> String {
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

pub(crate) fn escape_char(c: char) -> String {
    match c {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        c => c.to_string(),
    }
}


const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "static",
    "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual",
    "where", "while", "yield",
];

/// Keywords that cannot be raw identifiers: rename with a trailing `_`.
const RUST_UNRAW: &[&str] = &["self", "Self", "super", "crate"];
