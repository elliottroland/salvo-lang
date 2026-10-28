//! Compiler middle-end: source-set assembly, name resolution, type
//! checking, and (later) IR lowering.

pub mod features;
pub mod copyplan;
pub mod param_mode;
pub mod partition;
pub mod mut_lends;
pub mod borrows;
pub mod abi;
pub mod case;
pub mod check;
pub mod check_cache;
pub mod comptime;
pub mod deadlock;
pub mod decl_id;
pub mod deduce;
pub mod diag;
pub mod effects;
pub mod erase;
pub mod expand;
pub mod facts;
pub mod ir;
pub mod lends;
pub mod linear;
pub mod literal;
pub mod lock;
pub mod naming;
pub mod origin;
pub mod manifest;
pub mod place;
pub mod platform;
pub mod program;
pub mod reach;
pub mod refine;
pub mod resolve;
pub mod source;
pub mod std_front;
pub mod timing;
pub mod typekey;
pub mod types;
pub mod unique_shape;
pub mod wire;

pub use check::{
    check_program, check_project, check_project_cached, Checked, Coercion, ImplicitArg, ImplicitParam, PassDriver,
    StdCache,
    ThrowSite,
    UnionTest, OK_QUALIFIER, THROWN_QUALIFIER, THROW_EFFECT,
};
pub use check_cache::CheckCache;
pub use std_front::{FrontProgram, StdFront};
pub use deduce::ParamDeduction;
pub use comptime::{CompHover, CompWhenArms};
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
pub use reach::{reachable_modules, routing_module, runtime_closure, runtime_module, streams_closure};
pub use wire::{
    approx_ty, effect_has_wire_form, protocol_canonical, protocol_hash, struct_has_wire_form,
    wire_blocker, WireBlock,
};
pub use resolve::{resolve, DefSite, ModuleScope, Resolution, STD_INTERNAL};
pub use decl_id::{DeclId, DeclIndex, DeclKind, DeclLoc, NodeId};
pub use source::{
    dependencies_with_reached_platform, CompanionFile, ModulePath, PlatformFile, PlatformRoots, SourceFile, SourceSet,
    PLATFORM_DIR, STD_ROOT, TEST_SUFFIX,
};
pub use types::{fn_type_throws, is_subtype, is_throw_effect, threaded_effects, Qual, QualEffect, Ty};
