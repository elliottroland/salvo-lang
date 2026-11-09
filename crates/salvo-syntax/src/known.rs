//! [doc-compilerknown] The std declarations the compiler names.
//!
//! A std declaration is **compiler-known** when renaming or reshaping it would
//! break something in the compiler, because the compiler refers to it by its
//! name: the throw effect, the iterator protocol's groups, the comptime type
//! model, the actor runtime's types. Each such name is a constant here, and
//! the compiler uses the constant rather than a string of its own; every
//! declaration listed in [`ALL`] carries `@compilerknown` in its doc comment,
//! and every `@compilerknown` in std is listed here
//! (`std_compiler_known_names_are_tagged` in `salvo-cli`'s docs tests).
//!
//! Not listed, by design:
//!
//! - `intrinsic` declarations: the keyword already says the compiler supplies
//!   them.
//! - What a rule depends on only by its *shape* — a qualifier with a
//!   `qualifies` (`Idx`, `NonEmpty`), a `Checked` field — which a rename does
//!   not break.
//! - Member names reached through a listed declaration (`Throw`'s `throw`,
//!   `ActorGroup`'s `join`): the declaration's tag covers them, and its note
//!   says which.

/// One compiler-known std declaration: the module that declares it and its
/// name. A `fn` entry covers every overload of the name in that module; a
/// type entry (a struct, type, effect, handler, qualifier or `fns` group)
/// the one declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Known {
    pub module: &'static str,
    pub name: &'static str,
    pub kind: KnownKind,
}

/// Whether a [`Known`] entry names a function or a type-level declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownKind {
    Fn,
    Type,
}

// ===== std.throw =====

/// [throw] The throw effect; its `throw` member is the operation.
pub const THROW_EFFECT: &str = "Throw";
/// [throw-try] What a `try` hands back for a caught throw.
pub const THROWN_QUALIFIER: &str = "Thrown";

// ===== std.core.result =====

/// The success provenance qualifier.
pub const OK_QUALIFIER: &str = "Ok";

// ===== std.test =====

/// [test-run] A test's failure, which the harness builds.
pub const FAILURE_STRUCT: &str = "Failure";

// ===== std.core.iterator =====

/// [iter-protocol] The arm a `next` answers with an element in.
pub const EMITTED_QUALIFIER: &str = "Emitted";
/// [iter-protocol] The arm a `next` answers at the end.
pub const FINISHED_STRUCT: &str = "Finished";
/// [iter-protocol] The group a `for` drives (`next`).
pub const NEXT_GROUP: &str = "Next";
/// [iter-mint] The group a `for` mints an iterator through (`iter`).
pub const ITER_GROUP: &str = "Iter";

// ===== std.core.compare and std.core.basic =====

/// [cmp-groups] The ordering group (`cmp`).
pub const ORDERED_GROUP: &str = "Ordered";
/// [cmp-groups] The equality group (`eq`).
pub const EQ_GROUP: &str = "Eq";
/// [cmp-groups] The hashing group (`hash`, `eq`).
pub const HASHED_GROUP: &str = "Hashed";
/// [interp-to-str] The rendering group (`to_str`).
pub const TO_STR_GROUP: &str = "ToStr";

// ===== std.core.comptime: the type model a `compfn` reads =====

pub const COMPTIME_STRUCT: &str = "Struct";
pub const COMPTIME_UNION: &str = "Union";
pub const COMPTIME_TUPLE: &str = "Tuple";
pub const COMPTIME_FN_TYPE: &str = "FnType";
pub const COMPTIME_BASIC: &str = "Basic";
pub const COMPTIME_GENERIC: &str = "Generic";
pub const COMPTIME_OPAQUE: &str = "Opaque";
pub const COMPTIME_KIND: &str = "Kind";
pub const COMPTIME_TYPE: &str = "Type";
pub const COMPTIME_LITERAL: &str = "Literal";
pub const COMPTIME_VALUE: &str = "Value";
pub const COMPTIME_FIELD: &str = "Field";
pub const COMPTIME_ARM: &str = "Arm";
/// [comptime-tuple] One element position of a tuple, as `[for index in T.indices]` binds it.
pub const COMPTIME_INDEX: &str = "Index";

/// [effect-claim] The subject a claim is declared for: `provenance qualifier
/// Ordered for Effect` qualifies an effect's binding, not a type of values.
/// Not a declared type: the name in that position is the marker.
pub const CLAIM_SUBJECT: &str = "Effect";

// ===== std.core.actor =====

/// [actor-mailbox] An actor's mailbox settings.
pub const MAILBOX_TYPE: &str = "Mailbox";
/// The qualifier giving an actor its own thread.
pub const DEDICATED_QUALIFIER: &str = "Dedicated";
/// [actor-pool-fault-sink] What a failed actor reports.
pub const FAULT_STRUCT: &str = "Fault";
/// [actor-pool-fault-sink] The effect faults are reported through.
pub const FAULTS_EFFECT: &str = "Faults";
/// The pool constructor.
pub const POOL_FN: &str = "pool";
/// [actor-message] One message of an actor effect, as a value.
pub const MESSAGE_TYPE: &str = "Message";
/// [actor-sent] Sends a message to an actor: the `send(to: eff[E], m:
/// Message<E>)` overload, beside the reply's `send(reply, value)`.
pub const MESSAGE_SEND_FN: &str = "send";
/// [actor-sent] The send member that takes every message of a face (a name
/// and a shape: `send fn sent(m: Message<F>)`).
pub const SENT_MEMBER: &str = "sent";

// ===== std.net =====

/// [actor-group] A group of actors; its `join` member is the compiler's too.
pub const ACTOR_GROUP_EFFECT: &str = "ActorGroup";
/// A protocol's description.
pub const PROTOCOL_STRUCT: &str = "Protocol";

// ===== std.args =====

/// [args-runtime] The command line's effect.
pub const ARGS_EFFECT: &str = "Args";
/// [args-runtime] Its host handler.
pub const HOST_ARGS_HANDLER: &str = "HostArgs";

// ===== the collections =====

/// [cmp-carry] The heap claim.
pub const HEAP_QUALIFIER: &str = "Heap";
/// [col-sorted-list] The sorted-list claim.
pub const SORTED_QUALIFIER: &str = "Sorted";
/// [ref-notsame] The claim that two handles are different elements.
pub const NOT_SAME_QUALIFIER: &str = "NotSame";
pub const SET_TYPE: &str = "Set";
pub const MAP_TYPE: &str = "Map";
pub const SORTED_SET_TYPE: &str = "SortedSet";
pub const SORTED_MAP_TYPE: &str = "SortedMap";
/// [ref-handle] The reads and handle mints a borrow path is lowered to.
pub const GET_FN: &str = "get";
pub const GET_AT_FN: &str = "get_at";
pub const AT_PROVEN_FN: &str = "at_proven";
pub const AT_FN: &str = "at";
pub const GET_PRESENT_FN: &str = "get_present";
pub const AT_PRESENT_FN: &str = "at_present";

// ===== modules =====

/// [test-run] The test harness's module, by its last segment.
pub const TEST_MODULE: &str = "test";
/// [throw] The throw effect's module, by its last segment.
pub const THROW_MODULE: &str = "throw";
/// std's internal module, by its last segment.
pub const STD_INTERNAL: &str = "runtime";

/// The compiler-known modules, by full path: each one's leading doc comment
/// carries `@compilerknown`.
pub const MODULES: &[&str] = &["std.test", "std.throw", "std.runtime"];

const fn ty(module: &'static str, name: &'static str) -> Known {
    Known { module, name, kind: KnownKind::Type }
}

const fn func(module: &'static str, name: &'static str) -> Known {
    Known { module, name, kind: KnownKind::Fn }
}

/// Every compiler-known declaration.
pub const ALL: &[Known] = &[
    ty("std.throw", THROW_EFFECT),
    ty("std.throw", THROWN_QUALIFIER),
    ty("std.core.result", OK_QUALIFIER),
    ty("std.test", FAILURE_STRUCT),
    ty("std.core.iterator", EMITTED_QUALIFIER),
    ty("std.core.iterator", FINISHED_STRUCT),
    ty("std.core.iterator", NEXT_GROUP),
    ty("std.core.iterator", ITER_GROUP),
    ty("std.core.compare", ORDERED_GROUP),
    ty("std.core.compare", EQ_GROUP),
    ty("std.core.compare", HASHED_GROUP),
    ty("std.core.basic", TO_STR_GROUP),
    ty("std.core.comptime", COMPTIME_STRUCT),
    ty("std.core.comptime", COMPTIME_UNION),
    ty("std.core.comptime", COMPTIME_TUPLE),
    ty("std.core.comptime", COMPTIME_FN_TYPE),
    ty("std.core.comptime", COMPTIME_BASIC),
    ty("std.core.comptime", COMPTIME_GENERIC),
    ty("std.core.comptime", COMPTIME_OPAQUE),
    ty("std.core.comptime", COMPTIME_KIND),
    ty("std.core.comptime", COMPTIME_TYPE),
    ty("std.core.comptime", COMPTIME_LITERAL),
    ty("std.core.comptime", COMPTIME_VALUE),
    ty("std.core.comptime", COMPTIME_FIELD),
    ty("std.core.comptime", COMPTIME_ARM),
    ty("std.core.comptime", COMPTIME_INDEX),
    ty("std.core.actor", MAILBOX_TYPE),
    ty("std.core.actor", DEDICATED_QUALIFIER),
    ty("std.core.actor", FAULT_STRUCT),
    ty("std.core.actor", FAULTS_EFFECT),
    func("std.core.actor", POOL_FN),
    ty("std.core.actor", MESSAGE_TYPE),
    func("std.core.actor", MESSAGE_SEND_FN),
    ty("std.net", ACTOR_GROUP_EFFECT),
    ty("std.net", PROTOCOL_STRUCT),
    ty("std.args", ARGS_EFFECT),
    ty("std.args", HOST_ARGS_HANDLER),
    ty("std.heap", HEAP_QUALIFIER),
    ty("std.core.list", SORTED_QUALIFIER),
    ty("std.core.ref", NOT_SAME_QUALIFIER),
    ty("std.core.set", SET_TYPE),
    ty("std.core.map", MAP_TYPE),
    ty("std.core.sorted", SORTED_SET_TYPE),
    ty("std.core.sorted", SORTED_MAP_TYPE),
    func("std.core.list", GET_FN),
    func("std.core.deque", GET_FN),
    func("std.core.map", GET_FN),
    func("std.core.list", GET_AT_FN),
    func("std.core.deque", GET_AT_FN),
    func("std.core.list", AT_PROVEN_FN),
    func("std.core.deque", AT_PROVEN_FN),
    func("std.core.list", AT_FN),
    func("std.core.deque", AT_FN),
    func("std.core.map", AT_FN),
    func("std.core.map", GET_PRESENT_FN),
    func("std.core.map", AT_PRESENT_FN),
];
