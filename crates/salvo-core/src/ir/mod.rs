//! The IR: a minimal, decided form of a Salvo program (IR record).
//!
//! Moved into `salvo-core` from the former `salvo-ir` crate on 2026-10-12 so
//! that core is the single home of checking: the IR is built from the checked
//! AST here, and the "real" semantic validation (linear obligations,
//! deductions) runs on this IR. See IR_CHECKER.md.

pub mod build;
pub mod check;
pub mod dump;
pub mod nodes;
pub mod obligations;
pub mod walk;

pub use build::actors::enum_key;
pub use build::build_program;
pub use nodes::*;

/// Checks `program` the way a build does, builds the IR of its modules and
/// renders them: every user module, or with `all` every reached module, or
/// only `module`.
pub fn dump_program(
    program: &crate::Program,
    all: bool,
    module: Option<&str>,
) -> Result<(String, Vec<String>), Vec<String>> {
    let symbols = crate::Symbols::collect(program);
    let resolution = crate::resolve(program);
    let checked = crate::check_program(program, &resolution, &symbols);
    if checked.errors.iter().any(|d| d.is_error()) {
        return Err(checked.errors.iter().map(|d| d.render(&program.files)).collect());
    }
    let reachable = crate::reachable_modules(program, &resolution, &checked);
    let wanted: std::collections::HashSet<&crate::ModulePath> = program
        .units()
        .filter(|u| match module {
            Some(m) => u.file.module.0.join(".") == m,
            None => (all && reachable.contains(&u.file.module)) || (!u.file.is_std && !u.file.is_test),
        })
        .map(|u| &u.file.module)
        .collect();
    let (mut ir, errors) = build::build_program(program, &symbols, &resolution, &checked, Some(&wanted));
    // [ir-obligation] The ledger is for reading and checking, not emission, so
    // it is inserted only here (and before `ir::check`), never in the IR the
    // backends consume.
    obligations::annotate(&mut ir, &symbols);
    let mut text = String::new();
    for m in &ir.modules {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&dump::Dumper::module(&ir, m));
    }
    Ok((text, errors))
}
