//! Compiler middle-end: source-set assembly, name resolution, type
//! checking, and (later) IR lowering.

pub mod check;
pub mod deadlock;
pub mod deduce;
pub mod diag;
pub mod effects;
pub mod lends;
pub mod place;
pub mod platform;
pub mod program;
pub mod reach;
pub mod refine;
pub mod resolve;
pub mod source;
pub mod types;

pub use check::{
    check_program, Checked, Coercion, ImplicitArg, ImplicitParam, PassDriver, PassMember,
    ThrowSite,
    UnionTest, UseKind, OK_QUALIFIER, THROWN_QUALIFIER, THROW_EFFECT,
};
pub use deduce::ParamDeduction;
pub use effects::{
    effect_member_index, effect_member_name, effect_members_named, handler_handle_deps,
    handler_member_faces,
};
pub use diag::FileDiagnostic;
pub use place::{Place, Step};
pub use platform::{
    declares_platform_effect, host_file, host_rel_path, missing_handler_host_error,
    missing_host_error, platform_effects, platform_entry, platform_handlers,
};
pub use program::{Program, Symbols};
pub use reach::reachable_modules;
pub use resolve::{resolve, DefSite, FnKey, ModuleScope, Resolution};
pub use source::{
    CompanionFile, ModulePath, SourceFile, SourceSet, PLATFORM_DIR,
};
pub use types::{is_subtype, Qual, QualEffect, Ty};
