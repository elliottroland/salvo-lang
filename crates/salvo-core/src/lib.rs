//! Compiler middle-end: source-set assembly and (later) name resolution,
//! type checking, and IR lowering.

pub mod program;
pub mod source;

pub use program::{Program, Symbols};
pub use source::{ModulePath, SourceFile, SourceKind, SourceSet};
