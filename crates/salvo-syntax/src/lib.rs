//! Lexer, parser, AST, and diagnostics for the Salvo language.

pub mod ast;
pub mod desugar;
pub mod diag;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use diag::Diagnostic;
pub use span::Span;

/// Parse a single Salvo source file into a module AST.
///
/// Always returns an AST (possibly partial) together with any diagnostics
/// collected along the way. The parse is considered failed if any diagnostic
/// is an error.
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (mut module, mut diagnostics) = parse_module_deferred(source);
    // [iter-fn] An `iter fn` is expanded into ordinary declarations here, so
    // every consumer of a parsed module — resolve, the checker, both emitters,
    // the LSP — sees the shape it already supports.
    diagnostics.extend(desugar::expand_iter_fns(&mut module));
    (module, diagnostics)
}

/// `parse_module` without the `iter fn` expansion: for a *program*, where the
/// expansion should see every file's struct declarations rather than this
/// file's alone (the per-field snapshot tier [iter-fn]). The caller **must**
/// run `desugar::expand_iter_fns_with` on the module before resolution —
/// an unexpanded `iter fn` is a shape no later stage supports, and resolve
/// reports one loudly rather than checking it as an ordinary fn.
pub fn parse_module_deferred(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let lexed = lexer::lex(source);
    let mut diagnostics = lexed.diagnostics;
    let mut parser = parser::Parser::new(source, lexed.tokens, lexed.comments);
    let module = parser.parse_module();
    diagnostics.extend(parser.into_diagnostics());
    (module, diagnostics)
}
