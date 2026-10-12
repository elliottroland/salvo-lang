//! [ir-linear-check] The IR-level linear-obligation check (`ir::check`) must
//! report **zero** leaks on known-good code (std + every example the AST
//! checker accepts) — a false positive there would break a valid program once
//! the check gates the build (IR_CHECKER.md Phase 1). It must also *catch* a
//! genuine leak the AST checker misses: a linear union arm dropped by an elvis
//! pick (noticeboard #2).

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

/// The IR check reports nothing on code the AST checker accepts.
#[test]
fn ir_linear_check_has_no_false_positives() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("salvo.toml").exists() && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]"))
        .collect();
    dirs.sort();
    let mut leaks = Vec::new();
    for dir in dirs {
        let _name = dir.file_name().unwrap().to_string_lossy().to_string();
        let program = example_program(&dir);
        let symbols = salvo_core::Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let checked = salvo_core::check_program(&program, &resolution, &symbols);
        if checked.errors.iter().any(|d| d.is_error()) {
            continue;
        }
        let (ir, _errs) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, None);
        let _ = &ir;
        for d in salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked) {
            leaks.push(format!("file{}: {}", d.file, d.message));
        }
    }
    leaks.sort();
    leaks.dedup();
    assert!(leaks.is_empty(), "IR linear check reported {} false positive(s):\n{}", leaks.len(), leaks.join("\n"));
}

/// The IR check catches a dropped linear union arm an elvis pick discards —
/// the case the AST checker misses (noticeboard #2).
#[test]
fn ir_linear_check_catches_a_dropped_linear_arm() {
    let src = "\
export linear struct Problem { code: Int }\n\
export fn close(p: Problem) -> None => !p { discard(p) }\n\
export fn attempt() -> Ok Int | Err Problem { return Err Problem { code: 1 } }\n\
export fn drops_it() -> Int { return attempt() ?: return 0 }\n";
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(!expansion.diagnostics.iter().any(|d| d.is_error()));
    let program = Program { files: sources.files, modules, companions: sources.companions };
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_core::check_program(&program, &resolution, &symbols);
    // The AST checker accepts this program — it misses the dropped `Problem`.
    assert!(!checked.errors.iter().any(|d| d.is_error()), "AST checker unexpectedly rejected the repro");
    let (mut ir, _errs) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, None);
    salvo_core::ir::obligations::annotate(&mut ir, &symbols);
    let diags = salvo_core::ir::check::check(&ir);
    assert!(
        diags.iter().any(|d| d.message.contains("still owns a linear value")),
        "IR check did not catch the dropped linear arm; got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// The IR check catches a linear value held across a call that may throw —
/// the throw-path leak, now a plain consequence of the `Never`/leave model
/// (a `throws` call is a conditional fn-leave, not a special case).
#[test]
fn ir_linear_check_catches_a_leak_across_a_throwing_call() {
    let src = "\
import throw.Throw\n\
export linear struct Handle { fd: Int }\n\
export fn close(x: Handle) -> None => !x { discard(x) }\n\
export fn open_handle(fd: Int) -> Handle { return Handle { fd: fd } }\n\
export fn risky(flag: Bool) [Throw<Str>] -> None { if flag { throw(\"no\") } }\n\
export fn probe(flag: Bool) [Throw<Str>] -> None {\n\
    let h = open_handle(3)\n\
    risky(flag)\n\
    close(h)\n\
}\n";
    let diags = leaks_of(src);
    assert!(
        diags.iter().any(|m| m.contains("still owns a linear value") && m.contains("throw")),
        "IR check did not catch the throw-path leak; got: {diags:?}"
    );
}

/// Build the IR of `src` (with std) and return the IR check's leak messages,
/// regardless of whether the AST checker also flags them.
fn leaks_of(src: &str) -> Vec<String> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(!expansion.diagnostics.iter().any(|d| d.is_error()));
    let program = Program { files: sources.files, modules, companions: sources.companions };
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_core::check_program(&program, &resolution, &symbols);
    let (mut ir, _errs) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, None);
    salvo_core::ir::obligations::annotate(&mut ir, &symbols);
    salvo_core::ir::check::check(&ir).into_iter().map(|d| d.message).collect()
}
