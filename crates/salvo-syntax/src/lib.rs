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
    let lexed = lexer::lex(source);
    let mut diagnostics = lexed.diagnostics;
    let mut parser = parser::Parser::new(source, lexed.tokens, lexed.comments);
    let mut module = parser.parse_module();
    diagnostics.extend(parser.into_diagnostics());
    // [pass-fn] A `pass fn` is expanded into ordinary declarations here, so
    // every consumer of a parsed module — resolve, the checker, both emitters,
    // the LSP — sees the shape it already supports.
    diagnostics.extend(desugar::expand_pass_fns(&mut module));
    (module, diagnostics)
}
