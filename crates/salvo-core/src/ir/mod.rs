//! The IR: a minimal, decided form of a Salvo program (IR record).
//!
//! Moved into `salvo-core` from the former `salvo-ir` crate on 2026-10-12 so
//! that core is the single home of checking: the IR is built from the checked
//! AST here, and the "real" semantic validation (linear obligations,
//! deductions) runs on this IR. See IR_CHECKER.md.

pub mod analyze;
pub mod build;
pub mod check;
pub mod dump;
pub mod flow;
pub mod nodes;
pub mod obligations;
pub mod safe_nav;
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
    // [ir-linear-check] D4b: `salvo ir` dumps even an invalid program and
    // reports each linear leak inline — on the line that produced it (the
    // dumper matches the leak's span against each statement). The dump is a
    // debugging view; `salvo build` is what fails (D4a). A leak whose span
    // matches no dumped line (e.g. a whole-fn `GainParams`) still appears,
    // appended after its module.
    let throws = check::throws_map(&ir);
    let file_content = |file_name: &str| -> Option<&str> { program.files.iter().find(|f| f.name == file_name).map(|f| f.content.as_str()) };
    let mut text = String::new();
    for m in &ir.modules {
        if !text.is_empty() {
            text.push('\n');
        }
        // Collect this module's leaks (per decl, so the file attribution is
        // exact), rendered with a `module:line:col` location prefix.
        let content = file_content(&m.file_name);
        let mut inline: Vec<(Span, String)> = Vec::new();
        for decl in &m.decls {
            for d in check::check_one(decl, &throws) {
                let loc = match content {
                    Some(c) => {
                        let (line, col) = salvo_syntax::span::line_col(c, d.span.start);
                        format!("{}:{line}:{col}", m.path.0.join("."))
                    }
                    None => format!("{}:{}", m.path.0.join("."), d.span.start),
                };
                inline.push((d.span, format!("{loc}: {}", d.message)));
            }
        }
        let module_text = dump::Dumper::module_with_leaks(&ir, m, &inline);
        // A leak whose span never appeared inline (no matching dumped line) is
        // appended so nothing is dropped from the debugging view.
        let mut unplaced: Vec<&String> = Vec::new();
        for (_span, msg) in &inline {
            if !module_text.contains(&format!("!!! {msg}")) {
                unplaced.push(msg);
            }
        }
        text.push_str(&module_text);
        for msg in unplaced {
            text.push_str(&format!("\n!!! {msg}"));
        }
        text.push('\n');
    }
    Ok((text, errors))
}
