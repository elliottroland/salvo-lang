//! Compiler middle-end: source-set assembly, name resolution, type
//! checking, and (later) IR lowering.

pub mod check;
pub mod deduce;
pub mod diag;
pub mod place;
pub mod program;
pub mod reach;
pub mod resolve;
pub mod source;
pub mod types;

pub use check::{
    check_program, AbortSite, Checked, Coercion, UnionTest, ABORTED_QUALIFIER, ABORT_EFFECT,
    OK_QUALIFIER,
};
pub use deduce::ParamDeduction;
pub use diag::FileDiagnostic;
pub use place::{Place, Proj};
pub use program::{check_define_pairing, Program, Symbols};
pub use reach::reachable_modules;
pub use resolve::{resolve, DefSite, FnKey, ModuleScope, Resolution};
pub use source::{CompanionFile, ModulePath, SourceFile, SourceKind, SourceSet};
pub use types::{is_subtype, Qual, QualEffect, Ty};
