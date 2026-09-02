//! Compiler middle-end: source-set assembly, name resolution, type
//! checking, and (later) IR lowering.

pub mod check;
pub mod deduce;
pub mod diag;
pub mod program;
pub mod reach;
pub mod resolve;
pub mod source;
pub mod types;

pub use check::{check_program, Checked, Coercion, UnionTest};
pub use deduce::ParamDeduction;
pub use diag::FileDiagnostic;
pub use program::{Program, Symbols};
pub use reach::reachable_modules;
pub use resolve::{resolve, DefSite, FnKey, ModuleScope, Resolution};
pub use source::{CompanionFile, ModulePath, SourceFile, SourceKind, SourceSet};
pub use types::{is_subtype, Qual, QualEffect, Ty};
