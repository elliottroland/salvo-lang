//! Lexer, parser, AST, and diagnostics for the Salvo language.

pub mod ast;
pub mod doc;
pub mod desugar;
pub mod diag;
pub mod lexer;
pub mod known;
pub mod messages;
pub mod node_id;
pub mod parser;
pub mod print;
pub mod span;
pub mod token;
pub mod visit;
pub mod visit_mut;

pub use diag::Diagnostic;
pub use span::Span;

/// [copy-scalar-free] The **Copy scalars**: a value of one is duplicated for
/// free on every backend, so moving one is a copy and never consumes the
/// source. The one list every "is this a scalar" question asks, from the
/// syntax passes (which have no types) to the backends.
pub const COPY_SCALARS: &[&str] = &["Int", "Long", "Float", "Double", "Bool", "Char", "Byte"];

/// [copy-scalar-free] Whether `name` is one of [`COPY_SCALARS`].
pub fn is_copy_scalar_name(name: &str) -> bool {
    COPY_SCALARS.contains(&name)
}

/// [copy-scalar-free] Whether a written type is a bare Copy scalar
/// (`Int`, no qualifiers, no arguments), by its written name. For the passes
/// that run before or without types; an alias to a scalar is not seen.
pub fn is_copy_scalar_type(ty: &ast::Type) -> bool {
    matches!(ty, ast::Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() && is_copy_scalar_name(&base.name.name))
}

/// Parse a single Salvo source file into a module AST.
///
/// Always returns an AST (possibly partial) together with any diagnostics
/// collected along the way. The parse is considered failed if any diagnostic
/// is an error.
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (mut module, mut diagnostics) = parse_module_deferred(source);
    // [mod-namespace] Dotted top-level declarations for the namespaces.
    diagnostics.extend(desugar::flatten_namespaces(&mut module));
    // [syntax-lowered] The sugar the parser kept, as the code it stands for.
    desugar::lower_sugar(&mut module);
    // [iter-fn] An `iter fn` is expanded into ordinary declarations here, so
    // every consumer of a parsed module — resolve, the checker, both emitters,
    // the LSP — sees the shape it already supports.
    diagnostics.extend(desugar::expand_iter_fns(&mut module));
    // [when-lowered] `?:` is the `when` it is.
    desugar::lower_picks(&mut module);
    // [node-id] The tree is final for this parse.
    node_id::number(&mut module);
    (module, diagnostics)
}

/// The module **as written** [syntax-lowered]: `parse_module` without the
/// namespace flattening [mod-namespace], the sugar lowering, the `iter fn`
/// expansion or the `?:` lowering [when-lowered] — what a printer writes back
/// as the source, and what a *program*'s driver expands with the test
/// expansion (`salvo_core::expand`). Such a caller **must** run
/// `desugar::flatten_namespaces`, `desugar::lower_sugar`,
/// `desugar::expand_iter_fns` and `desugar::lower_picks` on the module before
/// resolution (`salvo_core::expand::expand_file` runs them all) — an
/// unexpanded `iter fn` is a shape no later stage supports, and resolve
/// reports one loudly rather than checking it as an ordinary fn; unlowered
/// sugar or `?:` is a checker error, and a namespace's declarations are
/// found by no one.
pub fn parse_module_deferred(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let lexed = lexer::lex(source);
    let mut diagnostics = lexed.diagnostics;
    let mut parser = parser::Parser::new(source, lexed.tokens, lexed.comments);
    let module = parser.parse_module();
    diagnostics.extend(parser.into_diagnostics());
    (module, diagnostics)
}

