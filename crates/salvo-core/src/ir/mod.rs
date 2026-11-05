//! The IR: a minimal, decided form of a Salvo program (IR record).
//!
//! Moved into `salvo-core` from the former `salvo-ir` crate on 2026-10-12 so
//! that core is the single home of checking: the IR is built from the checked
//! AST here, and the "real" semantic validation (linear obligations,
//! deductions) runs on this IR. See the IR-checker record.

pub mod analyze;
pub mod build;
pub mod check;
pub mod dump;
pub mod flow;
pub mod gate;
pub mod nodes;
pub mod obligations;
pub mod safe_nav;
pub mod walk;

pub use build::actors::enum_key;
pub use build::{build_program, build_program_gated};
pub use nodes::*;

/// [assert-trap] A trap's `module:line:col` for a node of the file `file`:
/// the module path, not the file's name (which depends on the loader), and
/// [node-origin] a copied node where a diagnostic about it would land. The
/// IR carries it on `assert!` and `unreachable!`; a backend asks it for a
/// subscript's range check, from the index's span [col-index-trap].
pub fn trap_location(program: &crate::Program, file: usize, span: salvo_syntax::Span) -> String {
    let (at, span) = crate::origin::landing(&program.modules, file, span);
    let f = &program.files[at];
    let (line, col) = salvo_syntax::span::span_line_col(&f.content, span);
    format!("{}:{line}:{col}", f.module)
}

/// Checks `program` the way a build does, builds the IR of its modules and
/// renders them: every user module, or with `all` every reached module, or
/// only `module`.
pub fn dump_program(
    program: &crate::Program,
    all: bool,
    module: Option<&str>,
) -> Result<(String, Vec<String>), Vec<String>> {
    dump_program_over(program, all, module, None)
}

/// [lsp-check-cache] [`dump_program`] checked through a clone of `cache` (one
/// warmed with std, so only the program's own files are checked).
pub fn dump_program_over(
    program: &crate::Program,
    all: bool,
    module: Option<&str>,
    cache: Option<&crate::CheckCache>,
) -> Result<(String, Vec<String>), Vec<String>> {
    let symbols = crate::Symbols::collect(program);
    let resolution = match cache {
        Some(c) => c.transient().resolve(program),
        None => crate::resolve(program),
    };
    let checked = crate::check_over(program, &resolution, &symbols, cache);
    if checked.errors.iter().any(|d| d.is_error()) {
        // [ir-analyze] With the IR's diagnostics on the declarations the
        // errors leave clean, as a build lists them. [diag-std-hidden]
        let ir = analyze::analyze(program, &symbols, &resolution, &checked);
        return Err(crate::diag::render_shown(checked.errors.iter().chain(&ir), &program.files));
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
    // [ir-linear-check] D4b: the IR dump (`salvo dump --phase generate-ir`) dumps even an invalid program and
    // reports each linear leak inline — on the line that produced it (the
    // dumper matches the leak's span against each statement). The dump is a
    // debugging view; `salvo build` is what fails (D4a). A leak whose span
    // matches no dumped line (e.g. a whole-fn `GainParams`) still appears,
    // appended after its module.
    let throws = check::throws_map(&ir);
    let file_content = |file_name: &str| -> Option<&str> { program.files.iter().find(|f| f.name == file_name).map(|f| f.content.as_str()) };
    // [node-origin] A copied node's location is where it lands (`origin::landing`):
    // the file and the span there.
    let landing = |file_name: &str, span: Span| -> Option<(&crate::source::SourceFile, Span)> {
        let i = program.files.iter().position(|f| f.name == file_name)?;
        let (at, span) = crate::origin::landing(&program.modules, i, span);
        Some((&program.files[at], span))
    };
    let locate = |m: &nodes::Module, span: Span| -> String {
        match landing(&m.file_name, span) {
            Some((f, at)) => {
                let (line, col) = salvo_syntax::span::span_line_col(&f.content, at);
                format!("{}:{line}:{col}", f.module)
            }
            None => format!("{}:{}", m.path.0.join("."), span.start),
        }
    };
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
                inline.push((d.span, format!("{}: {}", locate(m, d.span), d.message)));
            }
            // [flow-use-after-move] A use of a moved value, inline the same way.
            for mut d in flow::check_one(decl) {
                let moved = landing(&m.file_name, d.moved_at);
                d.moved_at = moved.map_or(d.moved_at, |(_, at)| at);
                let message = d.render(moved.map_or(content.unwrap_or(""), |(f, _)| f.content.as_str())).text;
                inline.push((d.span, format!("{}: {message}", locate(m, d.span))));
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
