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
pub mod trivia;
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

/// Parses one Salvo source file into its module **as written**
/// [syntax-lowered]: the tree the source spells, sugar, namespaces, `iter fn`s
/// and `?:` included, with no lowering and no node ids — what a printer writes
/// back as the source.
///
/// Always returns an AST (possibly partial) together with any diagnostics
/// collected along the way. The parse is considered failed if any diagnostic
/// is an error. A program's driver runs the per-file expansion
/// (`salvo_core::expand::expand_file`, whose lowering is
/// [`desugar::lower_module`]) before resolution: an unexpanded `iter fn` is a
/// shape no later stage supports, unlowered sugar or `?:` is a checker error,
/// and a namespace's declarations are found by no one.
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let lexed = lexer::lex(source);
    let mut diagnostics = lexed.diagnostics;
    let mut parser = parser::Parser::new(source, lexed.tokens, lexed.comments);
    let module = parser.parse_module();
    diagnostics.extend(parser.into_diagnostics());
    (module, diagnostics)
}

/// [test-data] Parses `source` as one type, the whole text: `None` when it
/// is not one (a parse error, or text after the type). For tools that read a
/// written type outside a program, such as the checker data's
/// `//~ type[…]` annotation.
pub fn parse_type(source: &str) -> Option<ast::Type> {
    let lexed = lexer::lex(source);
    if lexed.diagnostics.iter().any(|d| d.is_error()) {
        return None;
    }
    let mut parser = parser::Parser::new(source, lexed.tokens, lexed.comments);
    let ty = parser.parse_whole_type();
    if parser.into_diagnostics().iter().any(|d| d.is_error()) {
        return None;
    }
    ty
}

/// [test-data] [`parse_type`] of a type written at byte `offset` of a file
/// (an annotation's, in a comment): every span is moved there, so what is
/// recorded at them is placed in that text.
pub fn parse_type_at(source: &str, offset: u32) -> Option<ast::Type> {
    struct Shift(u32);
    impl visit_mut::MutVisitor for Shift {
        fn visit_span(&mut self, span: &mut Span) {
            span.start += self.0;
            span.end += self.0;
        }
    }
    let mut ty = parse_type(source)?;
    visit_mut::walk_type(&mut Shift(offset), &mut ty);
    Some(ty)
}
