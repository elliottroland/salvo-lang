//! Compiler middle-end: source-set assembly, name resolution, type
//! checking, and (later) IR lowering.

pub mod route;
pub mod abi;
pub mod case;
pub mod check;
pub mod comptime;
pub mod deadlock;
pub mod deduce;
pub mod diag;
pub mod effects;
pub mod erase;
pub mod expand;
pub mod lends;
pub mod literal;
pub mod lock;
pub mod manifest;
pub mod place;
pub mod platform;
pub mod prereq;
pub mod program;
pub mod reach;
pub mod refine;
pub mod resolve;
pub mod source;
pub mod types;
pub mod wire;

pub use check::{
    check_program, Checked, Coercion, CompareVia, ImplicitArg, ImplicitParam, PassDriver,
    PassMember,
    ThrowSite,
    UnionTest, OK_QUALIFIER, THROWN_QUALIFIER, THROW_EFFECT,
};
pub use deduce::ParamDeduction;
pub use comptime::CompHover;
pub use expand::{expand, Expansion, TestCase};
pub use effects::{
    effect_member_index, effect_member_name, effect_members_named, handler_is_stateful,
    handler_member_faces, has_any_router, effect_only_args,
};
pub use diag::FileDiagnostic;
pub use lock::{reconcile as reconcile_lock, Lock, LockOutcome};
pub use manifest::{
    is_nested_project, Dependency, HostDeps, Manifest, Project, LOCK_FILE, MANIFEST_FILE,
};
pub use place::{Place, Step};
pub use platform::{
    host_file, host_rel_path, missing_handler_host_error, platform_declarations, platform_effects,
    platform_handlers, PlatformEffect,
    platform_root_required, reply_contract_comment, reply_params, required_backends,
};
pub use program::{Program, Symbols};
pub use erase::{erase_effect_generics, erased_generics, Erased};
pub use reach::reachable_modules;
pub use wire::{
    approx_ty, effect_has_wire_form, protocol_canonical, protocol_hash, struct_has_wire_form,
    wire_blocker, WireBlock,
};
pub use resolve::{resolve, DefSite, FnKey, ModuleScope, Resolution};
pub use source::{
    dependencies_with_reached_platform, CompanionFile, ModulePath, PlatformFile, PlatformRoots, SourceFile, SourceSet,
    PLATFORM_DIR, TEST_SUFFIX,
};
pub use types::{is_subtype, Qual, QualEffect, Ty};
