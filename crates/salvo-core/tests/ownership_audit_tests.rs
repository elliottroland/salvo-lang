//! [ownership-audit] The evidence that ownership never changes what an
//! accepted program means (PIPELINE.md R5, step 7 piece 2): every example —
//! and std, whose bodies every example checks — is checked twice, normally
//! and with ownership's feedback into typing switched off
//! (`check_program_ownership_blind`: a consumed value keeps its type, a
//! partial move reads as itself), and the typing tables must agree. A
//! difference is a place where a move or a poison changed a type, an
//! overload, a coercion or a narrowing in a program the checker accepts —
//! which would make moving ownership out of the typing walk a change of
//! meaning.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::Path;

use salvo_core::{Program, SourceSet};

fn example_program(dir: &Path) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let project = salvo_core::Project::load(&dir.join(salvo_core::MANIFEST_FILE)).expect("manifest");
    let src = project.src();
    let errors = sources.add_dir(&src, "kt", false, false);
    assert!(errors.is_empty(), "failed to read {}: {errors:?}", src.display());
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(!expansion.diagnostics.iter().any(|d| d.is_error()));
    Program { files: sources.files, modules, companions: sources.companions }
}

/// The keys of two tables whose values differ (or are present in one only),
/// rendered with `Debug`.
fn differences<K: Eq + Hash + Debug + Clone, V: Debug>(
    what: &str,
    a: &HashMap<K, V>,
    b: &HashMap<K, V>,
) -> Vec<String> {
    let mut keys: Vec<&K> = a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))).collect();
    keys.sort_by_key(|k| format!("{k:?}"));
    keys.into_iter()
        .filter_map(|k| {
            let (x, y) = (a.get(k).map(|v| identity_free(&format!("{v:?}"))), b.get(k).map(|v| identity_free(&format!("{v:?}"))));
            (x != y).then(|| format!("{what} at {k:?}: normal {x:?}, ownership-blind {y:?}"))
        })
        .collect()
}

/// A dependent qualifier's value argument names its roots by the checker's
/// variable numbers (`ValueRef { path: "heap", roots: [29] }`), which count
/// every variable the file's walk declared so far. The two walks number
/// std's `heap.sv` differently (by three, from somewhere before `pop`; no
/// consumed value is read in either), so the comparison is over the path
/// the claim names, not the number: identities are compared within a walk,
/// never across two.
fn identity_free(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("roots: [") {
        out.push_str(&rest[..i]);
        out.push_str("roots: [_]");
        rest = &rest[i..];
        rest = &rest[rest.find(']').map(|j| j + 1).unwrap_or(rest.len())..];
    }
    out.push_str(rest);
    out
}

#[test]
fn ownership_never_changes_the_typing_of_an_accepted_program() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.join("salvo.toml").exists()
                && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]")
        })
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty());
    let mut found: Vec<String> = Vec::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let program = example_program(&dir);
        let symbols = salvo_core::Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let normal = salvo_core::check_program(&program, &resolution, &symbols);
        assert!(!normal.errors.iter().any(|d| d.is_error()), "{name} does not check clean");
        let blind = salvo_core::check_program_ownership_blind(&program, &resolution, &symbols);
        let mut diffs = Vec::new();
        diffs.extend(differences("type", &normal.expr_ty, &blind.expr_ty));
        diffs.extend(differences("representation", &normal.repr_ty, &blind.repr_ty));
        diffs.extend(differences("callee", &normal.call_fn, &blind.call_fn));
        diffs.extend(differences("coercion", &normal.coerce, &blind.coerce));
        diffs.extend(differences("narrowing test", &normal.is_tests, &blind.is_tests));
        found.extend(diffs.into_iter().map(|d| {
            let file = d
                .split("at (")
                .nth(1)
                .and_then(|r| r.split(',').next())
                .and_then(|n| n.trim().parse::<usize>().ok())
                .map(|f| program.files[f].name.clone())
                .unwrap_or_default();
            format!("{name}: {file}: {d}")
        }));
    }
    assert!(found.is_empty(), "{} difference(s):\n{}", found.len(), found.join("\n"));
}
