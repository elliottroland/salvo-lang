//! [ir-analyze] The IR-analysis stage: build the IR of a type-checked program,
//! run every IR-level analysis over it, and return the diagnostics as
//! `FileDiagnostic`s the gates (emission, `analyze`, the LSP) merge with the
//! checker's own.
//!
//! This is the home for analyses that are about *what happens as the code runs*
//! — flow, consumption, control-leaves — as opposed to the typing/resolution/
//! declaration-shape checks that stay in `check.rs` (IR_CHECKER.md Phase 2).
//! Its first tenant is the linear-obligation check (`ir::check`); the
//! fate/release, loop-totality, exhaustiveness, and deadlock analyses are to
//! follow.
//!
//! Runs only on a program the type-checker accepted: the IR builder copies the
//! checker's answers and assumes no type errors, and the type errors are the
//! actionable signal when there are any.

use std::collections::HashMap;

use crate::check::Checked;
use crate::diag::FileDiagnostic;
use crate::program::{Program, Symbols};
use crate::resolve::Resolution;

/// Build the IR of `program` and run the IR-level analyses, returning their
/// diagnostics attributed to source files. The caller must have confirmed the
/// program type-checks (no `checked` errors).
pub fn analyze(program: &Program, symbols: &Symbols, resolution: &Resolution, checked: &Checked) -> Vec<FileDiagnostic> {
    // Analyze only the user's own modules. std (and a dependency) is checked
    // elsewhere — and under the std-cache path its bodies are not re-walked, so
    // the checker tables the IR reads (`linear_moves`, …) are not populated for
    // it; building and checking its IR would report spurious leaks. The user
    // modules are always fully checked, so their tables are complete.
    let wanted: std::collections::HashSet<&crate::ModulePath> = program
        .units()
        .filter(|u| !u.file.is_std && !u.file.is_test)
        .map(|u| &u.file.module)
        .collect();
    let (mut ir, _build_errors) = super::build::build_program(program, symbols, resolution, checked, Some(&wanted));
    super::obligations::annotate(&mut ir, symbols);

    // Map each IR module's source file name to its index into `program.files`,
    // so a file-local span becomes a file-attributed diagnostic.
    let file_of: HashMap<&str, usize> = program.files.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();

    let mut out = Vec::new();
    let throws = super::check::throws_map(&ir);
    for module in &ir.modules {
        let file = file_of.get(module.file_name.as_str()).copied().unwrap_or(0);
        let scope = resolution.scopes.get(file);
        for decl in &module.decls {
            for d in super::check::check_one(decl, &throws) {
                // [linear-group] Name the dischargers in the remedy ("discharge
                // it with `drain` / `close`"), computed from the leaked type in
                // the file's resolution scope.
                let mut message = d.message;
                if let (Some(ty), Some(scope)) = (&d.discharge_type, scope) {
                    let set = crate::linear::discharge_set(symbols, resolution, scope, &checked.deductions, ty);
                    if !set.is_empty() && message.ends_with("discharge it") {
                        let named: Vec<String> = set.iter().map(|f| format!("`{f}`")).collect();
                        message.push_str(&format!(" with {}", named.join(" / ")));
                    }
                }
                out.push(FileDiagnostic::error(file, d.span, message));
            }
        }
    }
    // A leak reachable by more than one walk path reports once.
    out.sort_by(|a, b| (a.file, a.span.start, &a.message).cmp(&(b.file, b.span.start, &b.message)));
    out.dedup_by(|a, b| a.file == b.file && a.span.start == b.span.start && a.message == b.message);
    out
}
