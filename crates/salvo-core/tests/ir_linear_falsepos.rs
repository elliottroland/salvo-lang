//! TRACKING HARNESS: measures the IR-level linear check's false positives on
//! known-good code (std + every example). The check is NOT yet wired as a
//! build gate (IR_CHECKER.md Phase 1) precisely because this count is nonzero;
//! driving it to zero is the Phase 2 flow-analysis work. `#[ignore]` so the
//! suite stays green — run with `cargo test -p salvo-core --test
//! ir_linear_falsepos -- --ignored --nocapture` to see the current leaks.

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

#[test]
#[ignore = "tracking harness: nonzero until the Phase 2 flow analysis lands"]
fn ir_linear_check_has_no_false_positives() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("salvo.toml").exists() && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]"))
        .collect();
    dirs.sort();
    let mut all_false_pos = Vec::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let program = example_program(&dir);
        let symbols = salvo_core::Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let checked = salvo_core::check_program(&program, &resolution, &symbols);
        if checked.errors.iter().any(|d| d.is_error()) {
            continue;
        }
        let (mut ir, _errs) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, None);
        salvo_core::ir::obligations::annotate(&mut ir, &symbols);
        // Report per-module+fn so the leak site is identifiable.
        for module in &ir.modules {
            for decl in &module.decls {
                let fname = decl.name().to_string();
                let one = salvo_core::ir::check::check_one(decl);
                for d in one {
                    all_false_pos.push(format!("{}::{fname}: {}", module.path.0.join("."), d.message));
                }
            }
        }
        let _ = &name;
    }
    // Deduplicate (std is rebuilt per example dir).
    all_false_pos.sort();
    all_false_pos.dedup();
    assert!(
        all_false_pos.is_empty(),
        "IR linear check reported {} false positive(s) on known-good code:\n{}",
        all_false_pos.len(),
        all_false_pos.join("\n")
    );
}
