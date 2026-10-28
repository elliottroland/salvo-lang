//! The `salvo` command's analysis, language server, hover docs and grammar
//! generator, as a library: the binary (`main.rs`) drives them, and the
//! tests call them in-process (TESTS.md C6).

pub mod analysis;
pub mod docs;
pub mod lang;
pub mod lsp;
