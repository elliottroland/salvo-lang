//! Compiler middle-end: source-set assembly, name resolution, type
//! checking, and (later) IR lowering.

pub mod check;
pub mod program;
pub mod resolve;
pub mod source;
pub mod types;

pub use check::{check_program, Checked, Coercion, UnionTest};
pub use program::{Program, Symbols};
pub use resolve::{resolve, FnKey, ModuleScope, Resolution};
pub use source::{ModulePath, SourceFile, SourceKind, SourceSet};
pub use types::{is_subtype, Qual, Ty};
