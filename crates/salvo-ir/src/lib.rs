//! The IR: a minimal, decided form of a Salvo program (IR.md).

pub mod build;
pub mod dump;
pub mod ir;

pub use build::build_program;
pub use ir::*;

/// Checks `program` the way a build does, builds the IR of its modules and
/// renders them: every user module, or with `all` every reached module, or
/// only `module`.
pub fn dump_program(
    program: &salvo_core::Program,
    all: bool,
    module: Option<&str>,
) -> Result<(String, Vec<String>), Vec<String>> {
    let symbols = salvo_core::Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    if checked.errors.iter().any(|d| d.is_error()) {
        return Err(checked.errors.iter().map(|d| d.render(&program.files)).collect());
    }
    let reachable = salvo_core::reachable_modules(program, &resolution, &checked);
    let wanted: std::collections::HashSet<&salvo_core::ModulePath> = program
        .units()
        .filter(|u| match module {
            Some(m) => u.file.module.0.join(".") == m,
            None => (all && reachable.contains(&u.file.module)) || (!u.file.is_std && !u.file.is_test),
        })
        .map(|u| &u.file.module)
        .collect();
    let (ir, errors) = build::build_program(program, &symbols, &resolution, &checked, Some(&wanted));
    let mut text = String::new();
    for m in &ir.modules {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&dump::Dumper::module(&ir, m));
    }
    Ok((text, errors))
}
