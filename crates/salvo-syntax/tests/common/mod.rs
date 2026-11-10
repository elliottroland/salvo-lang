//! What the parser's tests share.

use salvo_syntax::ast::Module;
use salvo_syntax::Diagnostic;

/// [syntax-lowered] One file parsed and lowered (`desugar::lower_module`),
/// its nodes numbered [node-id]: for a test about what a construct lowers
/// to, rather than how it is written. The parse's diagnostics come first.
pub fn lowered(source: &str) -> (Module, Vec<Diagnostic>) {
    let (mut module, mut diagnostics) = salvo_syntax::parse_module(source);
    diagnostics.extend(salvo_syntax::desugar::lower_module(&mut module));
    salvo_syntax::node_id::number(&mut module);
    (module, diagnostics)
}
