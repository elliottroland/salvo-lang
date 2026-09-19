//! The type checker.
//!
//! Walks eery function body of every language file, inferring a type for
//! each expression and recording side tables the backend consults:
//!
//! * `expr_ty`: the logical type of each expression (after flow narrowing).
//! * `repr_ty`: for narrowed identifier uses, the *declared* type — the
//!   physical representation the variable is stored in.
//! * `coerce`: expressions that must be wrapped/re-wrapped into a union
//!   representation at a boundary (let/assign/return/argument/branch value).
//! * `is_tests`: how an `is` check (or `when` branch) lowers against a
//!   union representation (which sealed arms to test / null checks).
//! * `call_fn`: resolved fn targets for call sites (type-based overloads).
//!
//! The checker is deliberately lenient: anything it cannot type becomes
//! `Ty::Unknown`, which is compatible with everything and never produces
//! cascading errors. Hard errors are reserved for union/`when` misuse,
//! shadowing, widening assignments, and unresolvable overloads.

use std::collections::{BTreeSet, HashMap, HashSet};

use salvo_syntax::ast::{self, *};
use salvo_syntax::Span;

use crate::diag::FileDiagnostic;
use crate::place::{Place, Step};
use crate::program::{Program, Symbols};
use crate::resolve::{DefSite, FnKey, ModuleScope, Resolution};
use crate::types::{is_subtype, FnParamContract, Qual, QualEffect, Ty};

/// Table key: (file index, expression span).
pub type Key = (usize, Span);

/// [fn-overload-rank] One candidate of an overload set as a possible
/// *lead*: the source of the expected types its arguments are checked
/// against. Carries the candidate's own generic names and the substitution
/// the arguments typed so far have determined
/// [call-generic-progressive].
/// [fn-rename] One `rename fn` in force: the new name, the overload it
/// means, and the name that overload no longer answers to.
struct RenameBinding<'p> {
    new_name: String,
    target: String,
    entry: crate::resolve::FnEntry<'p>,
    /// Where it was written, for the "already named" diagnostic.
    span: Span,
}

/// [effect-member-overload] The outcome of picking a member overload: the
/// sole candidate, the one the argument types chose (with those types, so the
/// caller does not re-check the arguments), or nothing — a reported error.
enum Picked<'p> {
    Only(&'p FnDecl),
    ByArgs(&'p FnDecl, Vec<Ty>),
    None,
}

/// [effect-available] Which side of a name's **one overload set** a call
/// belongs to (user decision 2026-09-14), carrying the argument types typed
/// while deciding so neither path types them again. `Neither` means a
/// diagnostic naming both sides has already been reported.
enum Route {
    Member(Vec<Ty>),
    Fn(Vec<Ty>),
    Neither,
}

/// [fn-overload-rank] One candidate that *fits* a call: the declaration, the
/// bindings and coercion targets its match produced, and everything the
/// selection needs — the ranking view of its parameters, and the rung of the
/// visibility ladder it came in on [fn-overload-scope].
struct Viable<'p> {
    key: Option<FnKey>,
    decl: &'p FnDecl,
    subst: HashMap<String, Ty>,
    /// (argument index, substituted parameter type) — what an argument is
    /// checked and *coerced* against once this candidate wins.
    pairings: Vec<(usize, Ty)>,
    rank: crate::types::RankedCandidate,
    rung: crate::resolve::Rung,
    /// The declaring module, rendered — for `@module` matching and for the
    /// diagnostics.
    module: String,
}

struct LeadCandidate {
    /// Index into the call's candidate list.
    index: usize,
    /// The declared (un-substituted) parameter patterns, per argument slot,
    /// as the ranking compares them [fn-overload-rank].
    rank: crate::types::RankedCandidate,
    generics: HashSet<String>,
    subst: HashMap<String, Ty>,
    /// [fn-overload-scope] Which rung it came in on: the lead is picked from
    /// the most specific rung, like the winner.
    rung: crate::resolve::Rung,
}

/// How an `is` check (or `when` branch check) lowers at runtime against the
/// subject's *declared* representation.
#[derive(Clone, Debug)]
pub struct UnionTest {
    /// Number of non-`None` arms in the declared union (wrapper size).
    /// `1` means the nullable `T?` representation (no wrapper).
    pub size: usize,
    /// Indices (into the declared union's non-`None` arms) this check
    /// matches. Empty together with `match_none` means a null test.
    pub arms: Vec<usize>,
    /// Whether the declared union has a `None` arm (Kotlin `?` suffix).
    pub nullable: bool,
    /// True for `is None` checks (lowered to `== null`).
    pub match_none: bool,
}

/// [iter-protocol] Which function a `for` calls to drive a pass — the `next`
/// that advances it, or the `close` that releases it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassMember {
    /// A declared overload, resolved from the subject's own type.
    Fn(FnKey),
    /// [implicit-group] [iter-generic-drive] An **implicit parameter** of the
    /// enclosing fn. The subject's type is one of its type parameters, so there
    /// is no declaration to resolve against: the member came in through a
    /// `?Yield<It, T>` (or `?Linear<It>`) spread and is called by that name.
    Implicit(String),
}

impl PassMember {
    /// The declaration behind it, when there is one — what the emitters need
    /// for a mangled name, an effect list or a machine name.
    pub fn key(&self) -> Option<FnKey> {
        match self {
            PassMember::Fn(key) => Some(*key),
            PassMember::Implicit(_) => None,
        }
    }
}

/// [iter-protocol] What a `for` over a **pass** needs from the checker: which
/// `next` to call, and which arm of its result carries an element.
#[derive(Clone, Debug)]
pub struct PassDriver {
    /// The `next` this subject is driven by.
    pub next: PassMember,
    /// Index of the `Emitted` arm among the result's non-`None` arms
    /// [union-arm-identity] — positional over the *declared* type, so
    /// `Finished | Emitted T` is as valid as the usual order. Unused when
    /// `origin` is set: a hidden machine reports absence directly.
    pub emitted_arm: usize,
    /// Number of non-`None` arms, i.e. the union wrapper size. Unused when
    /// `origin` is set.
    pub arms: usize,
    /// [iter-fn] The resolved `next` is a **`yield fn`**: the subject
    /// is an *origin* struct, so the loop constructs the hidden state machine
    /// from it and drives that. `next` is the `yield fn`'s key — what the
    /// emitters name the machine after and read the handler order from — and
    /// the subject is *not* consumed, since a fresh machine is minted per
    /// loop.
    pub origin: bool,
    /// [iter-drive-in-place] The subject is a **place the enclosing fn keeps**
    /// — a `Mut` parameter its deduction list hands back, or a projection of one
    /// — so the loop advances the pass *where it lives* instead of taking it
    /// over: the position it reaches is what the caller sees next, and the
    /// release stays the caller's (a `close` here would be their
    /// use-after-close).
    ///
    /// Without this the two backends disagreed on the same program: Rust bound
    /// the subject into a local (a clone, since the parameter is a `&mut`), so
    /// the caller's pass never advanced, while Kotlin aliased it and the caller
    /// saw the new position [backend-parity].
    pub in_place: bool,
    /// [iter-pass] The `iter` overload that **mints** the pass, when the
    /// subject is a container rather than a pass itself (`for x in bag`, where
    /// `iter(bag)` answers a pass). The emitters call it once, before the
    /// driving loop, and drive its result. `None` when the subject *is* the
    /// pass — and also for the intrinsic containers (a list, an array, a
    /// `Str`), which record no driver at all because the backends iterate
    /// their own data natively [iter-for-native].
    pub mint_iter_fn: Option<FnKey>,
}

/// A representation change the emitter must apply to an expression.
#[derive(Clone, Debug)]
pub enum Coercion {
    /// Wrap a plain value into arm `arm` (index into non-`None` arms) of
    /// the `target` union.
    ///
    /// `inner` is a representation change applied to the value *first*, and
    /// exists for one shape: a qualified union group arm `Q (A | B)` reached
    /// by a value whose qualifier list flattened (`emitted(ok("x"))`
    /// [qual-group]). Such a value is physically a bare `A`, so it must be
    /// wrapped into the inner union before the outer one.
    WrapUnion {
        target: Ty,
        arm: usize,
        inner: Option<Box<Coercion>>,
    },
    /// Re-wrap a value between two union representations (matching arms by
    /// type equality; unmatched source arms are unreachable at runtime).
    Rewrap { from: Ty, to: Ty },
    /// Wrap a bare value into an optional (`T?`-style) representation
    /// [type-nullable]. Backends whose optionals are physical wrap
    /// (`Some(...)` in Rust); backends with transparent nullability
    /// (Kotlin) treat this as a no-op.
    WrapOption { target: Ty },
    /// [type-none-unit] The `None` **value** stands in a slot whose type is
    /// the bare `None` type, so its representation is the target's *unit*
    /// value rather than an absent optional (Rust `()`, Kotlin `Unit`).
    ///
    /// `None` is spelled the same in both roles — the absent arm of a `T?`,
    /// and the sole value of the `None` type — and the two lower
    /// differently, so the choice cannot be made from the expression alone.
    /// It is made here, where the slot is known: a generic argument slot the
    /// call inferred as `None` (`ok(None)` for an `Ok None | Err E` result,
    /// `emitted(None)` for a sequence of optionals) is the shape that needs
    /// it, and emitting the optional there is target code that does not
    /// compile.
    NoneUnit,
    /// [str-drop-mut] A `Mut` qualifier is dropped here: the value is used
    /// where the plain type is required. Every other qualifier erases, so
    /// widening is free — but `Mut` is the one qualifier a backend may
    /// render as a *different type* [type-canbe-mut], and where it does
    /// (Kotlin's `Mut Str` = `StringBuilder`, which is not a `String`) the
    /// drop is a real conversion. `from` is the qualified type being
    /// dropped, so the backend can decide from its base; a backend where
    /// `Mut` erases (Rust, and Kotlin's `MutableList`) renders nothing.
    ///
    /// `then` carries the representation change that would have been
    /// recorded here anyway (a union wrap, say), because one expression has
    /// one coercion slot and both have to happen — the drop first, since
    /// the wrap is about the *plain* type.
    DropMut {
        from: Ty,
        then: Option<Box<Coercion>>,
    },
}

/// The effect whose operation is non-resumptive [throw]. Declared in std
/// (`std/core/throw.sv`), known by name to the compiler: it has no handler
/// — `try` delimits it — and it is not threaded as an effect parameter.
pub const THROW_EFFECT: &str = "Throw";

/// [iter-fn] The name prefix of the pass type an **origin** mints at a
/// call site (user decision 2026-09-09). Not writable in Salvo source, so the
/// machine stays unnameable while being an ordinary inferred type argument; the
/// emitters name the generated machine identically, which is how a mint at an
/// argument and a `for` over the same origin agree.
pub const ORIGIN_PASS_PREFIX: &str = "__Pass_";
/// The value arm of a `try` outcome, from `core.result` [try].
pub const OK_QUALIFIER: &str = "Ok";
/// The message arm of a `try` outcome, from `core.throw` [try].
pub const THROWN_QUALIFIER: &str = "Thrown";
/// [actor-spawn-expr] The handle a `spawn` produces, from `core.actor`:
/// known by name to the compiler because its type argument is an **effect**,
/// which is the one sanctioned exception to [effect-not-data] (user decision
/// 2026-09-15).
pub const ADDR_TYPE: &str = "Addr";
/// [actor-replyto] The linear one-shot answer token, from `core.actor`.
pub const REPLY_TYPE: &str = "Reply";
/// [actor-mailbox] The std struct a handler's `mailbox { … }` slot builds,
/// from `core.actor`: the slot is that struct's literal with the type elided.
pub const MAILBOX_TYPE: &str = "Mailbox";

/// [actor-spawn-expr] The thread pool a spawn names in its `on` clause,
/// from `core.actor`.
pub const POOL_TYPE: &str = "Pool";

/// [waitfor-dedicated] The provenance qualifier `thread()` mints, from
/// `core.actor`: a pool of one thread, which the `on` clause consumes, and
/// the only placement a `[waitfor]`-carrying handler may be spawned onto.
pub const DEDICATED_QUALIFIER: &str = "Dedicated";

/// [actor-spawn-expr] The effect an `Addr<E>` serves, or `None` for anything
/// that is not an addr. What `use addr`, a dot-call through an addr, and a spawn's
/// `use` clause all ask.
fn addr_effect(ty: &Ty) -> Option<Ty> {
    match ty.strip_quals() {
        Ty::Named { name, args } if name == ADDR_TYPE && args.len() == 1 => Some(args[0].clone()),
        _ => None,
    }
}

/// One site that may throw [throw]: the `throw` operation itself, or a
/// call to a fn declaring `[Throw<M>]` that propagates one.
#[derive(Clone, Debug)]
pub struct ThrowSite {
    /// True for `throw(message)` itself; false for a call that merely
    /// propagates a throw performed further down.
    pub performs: bool,
    /// The message type at this site.
    pub message: Ty,
    /// Where the throw lands: `Some(span)` is the enclosing `try`
    /// expression's span (the innermost one [try-innermost]); `None` means
    /// it propagates out of the enclosing fn, which therefore declares
    /// `[Throw<M>]`.
    pub delimiter: Option<Span>,
    /// The message type at the *landing* site: the delimiter's computed
    /// `M`, or the fn's declared one. Equal to `message` unless the target
    /// is a union of several message types (user decision 2026-09-04).
    pub target: Ty,
    /// The arm of `target` to wrap the message into, when `target` is a
    /// wrapper union. `None` when no wrapping is needed.
    pub arm: Option<usize>,
}

/// The checker's output.
#[derive(Default)]
pub struct Checked {
    pub expr_ty: HashMap<Key, Ty>,
    /// [op-promote] Operator operands widened by numeric promotion, keyed
    /// by the operand expression's span, mapped to the promoted type
    /// (`Int + Long` promotes the `Int` side to `Long`; `Float`/`Double`
    /// likewise). Read by backends whose target has no native mixed-width
    /// operators (Rust inserts `as` casts; Kotlin's own operator set
    /// already covers the mixes and ignores this).
    pub promotions: HashMap<Key, Ty>,
    /// Declared (physical) type for identifier uses whose logical type was
    /// narrowed by flow analysis.
    pub repr_ty: HashMap<Key, Ty>,
    pub coerce: HashMap<Key, Coercion>,
    /// Keyed by the span of the `is` expression or the `when` branch.
    pub is_tests: HashMap<Key, UnionTest>,
    /// Predicate-qualifier `is` checks on non-union subjects, keyed by the
    /// span of the `is` expression: the qualifier names whose `qualifies`
    /// functions must be called (conjunction).
    pub predicate_tests: HashMap<Key, Vec<String>>,
    /// Field accesses whose type is refined by a predicate-qualifier field
    /// override, keyed by the field expression span: the overridden type
    /// (the backend casts + asserts).
    pub field_casts: HashMap<Key, Ty>,
    /// Resolved fn declaration for call sites (keyed by call span).
    pub call_fn: HashMap<Key, FnKey>,
    /// [iter-protocol] How a `for` loop drives a **pass**, keyed by the span
    /// of its *subject*. Present only when the subject has a `next`, rather
    /// than being an array or a value with an `iter`. There is
    /// no call node in the AST for the emitters to look at (the driving loop
    /// is synthesized), so the overload *and* the arm identity have to be
    /// handed over here — the alternative, each emitter re-deriving the arm
    /// index from the declaration, is exactly the checker/emitter
    /// disagreement the invariants forbid.
    pub for_drivers: HashMap<Key, PassDriver>,
    /// [fn-rename] Call sites (and fn-value uses) written with a **renamed**
    /// name, keyed the same way as `call_fn`. A rename is erased, so the
    /// emitters must spell the declaration's own name rather than the one in
    /// the source — and they cannot tell a rename from an import alias, which
    /// they *do* keep, without being told.
    pub renamed_calls: HashSet<Key>,
    /// [qual-refn] Every qualifier refinement that applies, keyed by the
    /// file a call is written in and the overload it resolves to.
    /// Scope-dependent by design: opting into a qualifier opts into what
    /// it knows, so the same callee refines differently in two files. Read
    /// by the call-site contract loop, by deduction inference, and by the
    /// language server's merged documentation [qual-refn-docs].
    pub refinements: crate::refine::Refinements,
    /// Concrete effect instance registered by each `use` statement (keyed
    /// by the statement span), with handler generics inferred from the
    /// constructor arguments (e.g. `Random<Int>` for
    /// `use CyclicRandom([1,2,3])`).
    /// [effect-handler-multi] Several entries when the handler wears several
    /// faces: a `use` binds every effect it implements, in declaration order.
    pub use_effects: HashMap<Key, Vec<Ty>>,
    /// Effect instances resolved for a `use`d handler's *dependencies*
    /// [effect-handler-deps], in the handler's declaration order (keyed by
    /// the `use` statement span). Only present when the handler declares
    /// dependencies. Nothing consumes it since the surface became an effect
    /// list on the declaration (2026-09-14): both emitters derive the list
    /// from the declaration and resolve each entry through their own
    /// environment. Kept because it is the only record of the *resolved
    /// instance* — which is what a generic dependency would need, and both
    /// backends refuse those today.
    pub use_deps: HashMap<Key, Vec<Ty>>,
    /// [actor-spawn-expr] The effect instance each `spawn` expression's child
    /// serves, keyed by the spawn's span — which is also the type of its
    /// value, an `Addr<E>`. The emitters' entry point for building an actor
    /// class.
    /// [effect-handler-multi] Several entries when the handler wears several
    /// faces, in declaration order — which is also the order of the addrs the
    /// spawn's tuple answers.
    pub spawn_effects: HashMap<Key, Vec<Ty>>,
    /// [actor-spawn-expr] The effect instances a spawn's `use` clause
    /// supplies for the child's dependencies, in the handler's declaration
    /// order (keyed by the spawn's span). Absent when the handler declares
    /// none. The clause items themselves stay in the AST; this records what
    /// each *resolved to*, which is what the child's construction needs.
    pub spawn_deps: HashMap<Key, Vec<Ty>>,
    /// [actor-spawn-expr] **Which clause item satisfied which declared
    /// dependency**: one index into the spawn's written `use` clause per
    /// declared dependency, in the *handler's declaration* order (keyed by
    /// the spawn's span). The two orders differ routinely — the declaration
    /// order fixes the child's provider fields and carrier arguments, the
    /// written order is what the program says — and `check_spawn` is where
    /// the matching happens, so recording it here is what keeps both
    /// emitters from re-deriving it (and from disagreeing about it).
    /// Parallel to `spawn_deps`: same length, same order.
    pub spawn_dep_items: HashMap<Key, Vec<usize>>,
    /// [actor-replyto] The enclosing handler's member each `replyto`
    /// delivers to, keyed by the `replyto` span. The name, because that is
    /// what identifies a continuation target; an overloaded send member is
    /// not expressible yet and will need the index instead.
    pub replyto_members: HashMap<Key, String>,
    /// [mixed-handler] Façade sends: a bare call inside a mixed handler's
    /// sync member that names one of the handler's own `send fn` members,
    /// mapped to that member's name. The emitters lower each to an enqueue
    /// on the façade value's addr.
    pub facade_sends: HashMap<Key, String>,
    /// [task-mint] The free `send fn` each *task* mint targets, keyed by the
    /// `replyto` span: a [`FnKey`] rather than a name, because a task target is
    /// resolved through the scope ladder and the emitters need the declaration
    /// the checker picked, not a name to re-resolve.
    pub replyto_tasks: HashMap<Key, FnKey>,
    /// [actor-use-addr] The effect each `use addr` statement binds, keyed by
    /// the statement's span: the emitters generate a forwarding stub over
    /// the addr rather than constructing a handler.
    pub use_addrs: HashMap<Key, Ty>,
    /// [actor-use-addr] Dot-calls that are **sends to an actor** rather than
    /// dispatches through a handler in scope, keyed by the call span and
    /// mapped to the effect instance the addr serves. The emitters need the
    /// distinction: the same written call is a method call on a handler in one
    /// case and an enqueue on a mailbox in the other.
    pub addr_calls: HashMap<Key, Ty>,
    /// [actor-self-send] `self.k(args)` sites, keyed by the call span and
    /// mapped to the member's name: a message to the actor the enclosing
    /// member belongs to. Distinct from `addr_calls` because there is no addr to
    /// read — the target is "this actor", which each backend spells its own
    /// way (an enqueue on the running activation's own mailbox, or, under a
    /// synchronous binding, an ordinary member call).
    pub self_sends: HashMap<Key, String>,
    /// [linear-container] Reads that **move a linear value**, keyed by the
    /// read's span: the emitters must hand the value over rather than copy
    /// it. Rust's narrowed-read path clones by default (`x.as_ref()
    /// .unwrap().clone()`), which duplicates an obligation — harmless for a
    /// handle type that happens to be `Clone`, impossible for one that is not
    /// (a reply token), and wrong either way.
    pub linear_moves: HashSet<Key>,
    /// [linear-state] Reads of a handler **state field** that *move* the
    /// value out (LC-4's allowance: a container of obligations being drained),
    /// keyed by the read's span. Rust cannot move out of a field behind
    /// `&mut self`, so the emitter renders these as `std::mem::take` — which
    /// is the semantics too, since the checker requires the member to put
    /// something back before it returns.
    pub state_takes: HashSet<Key>,
    /// [actor-deadlock-cycle] Gated mints (`replyto!`) inside a handler's
    /// continuation is outstanding the actor serves *nothing else*, so
    /// every gate is a wait-for edge's tail — which is why the graph is
    /// keyed on these and not on `replyto`, whose continuation leaves the
    /// mailbox open.
    pub actor_gates: Vec<(String, Key)>,
    /// [actor-deadlock-cycle] Sends from inside a handler's members to a
    /// actor of another protocol: `(handler name, target effect name, the
    /// call's site)`. Recorded for dot-calls through an `Addr`; sends
    /// reached through the handler's *declared dependencies* are read off
    /// the declaration instead, and a `k@self(…)` is not here at all — a
    /// self-send waits for no other actor.
    pub actor_sends: Vec<(String, String, Key)>,
    /// [task-mint] The same for a **free `send fn`**'s body: `(the send fn's
    /// name, the target effect, the site)`. A task belongs to no handler, so
    /// its sends are attributed to whoever *mints* it — which is what
    /// `task_mints` records.
    pub task_sends: Vec<(String, String, Key)>,
    /// [task-mint] Who mints which task: `(the minter, the target `send fn`)`,
    /// where the minter is the enclosing handler's name or — for a task that
    /// mints another task — the enclosing free `send fn`'s. Whole-program and
    /// conservative, exactly as coarse as the type-level graph already is
    /// (FC-6: a real cycle class would otherwise go unreported).
    pub task_mints: Vec<(String, String)>,
    /// The concrete effect instance an effect-member call dispatches
    /// through (keyed by the call span), after generic disambiguation.
    pub effect_calls: HashMap<Key, Ty>,
    /// [effect-member-overload] Which **overload** of an effect member a call
    /// resolved to: the member's index in its effect's declaration order,
    /// keyed by the call span. Recorded only where the effect declares the
    /// name more than once (`close(InStream)` / `close(OutStream)`), because
    /// that is exactly where the emitters' name-keyed lookup cannot answer —
    /// they name an overloaded member positionally
    /// (`salvo_core::effect_member_name`).
    pub effect_member_calls: HashMap<Key, usize>,
    /// [call-resolve] Call sites whose callee resolved to a **fn-typed
    /// local** (a parameter or `let`) rather than to a declaration, keyed
    /// by the call span.
    ///
    /// The emitters need this because their own first question — "is this
    /// name an effect member?" — is asked of a *program-wide* map
    /// (`Symbols::effect_of_fn`), which knows nothing of scopes or locals,
    /// while the checker resolves a local first. Without the record, a
    /// program declaring `effect Sink { fn keep(...) }` made std's
    /// `filter(it, keep: (T) -> Bool)` emit an effect dispatch for its own
    /// parameter, reported as "no handler for effect `Sink`" from inside
    /// `core/seq.sv` — the checker and the emitters disagreeing about what
    /// a call *is*.
    pub local_calls: HashSet<Key>,
    /// [effect-available] Call sites whose name *is* an effect member but
    /// which resolved to an ordinary **fn declaration**, because no instance
    /// of the owning effect was in scope (2026-09-14). Recorded for the same
    /// reason as `local_calls`: the emitters ask a program-wide,
    /// scope-blind map (`Symbols::effect_of_fn`) whether a name is a member,
    /// and would otherwise emit a member dispatch for a call the checker
    /// resolved to a function — std's `Fs` claims `close`, `write`,
    /// `read_line` and `position`, so this is every program that has one of
    /// those names and no filesystem.
    pub fn_over_member_calls: HashSet<Key>,
    /// Concrete effect instances threaded as leading handler arguments for
    /// a call to a fn that declares effect dependencies (keyed by the call
    /// span, in the callee's declaration order).
    pub call_effects: HashMap<Key, Vec<Ty>>,
    /// The lowered declared effect list of each fn [effect-fn-deps], in
    /// declaration order (duplicates and unresolved entries dropped) —
    /// the primary source for the emitters' effect-parameter
    /// environments, keyed by checker types rather than type renderings.
    pub fn_effects: HashMap<FnKey, Vec<Ty>>,
    /// [call-type-args] The type arguments a generic call resolved to, in
    /// the callee's declaration order (keyed by the call span). Inferred
    /// from the arguments, an explicit type-argument list, or the expected
    /// type. The backends' intrinsic lowerings render these
    /// [backend-intrinsic].
    pub call_type_args: HashMap<Key, Vec<Ty>>,
    /// Wrapper union sizes needed by the program (for `unions.kt`).
    pub union_sizes: BTreeSet<usize>,
    /// [fn-effects] The effect values a lambda takes as leading
    /// parameters, in order (keyed by the lambda expression's span): the
    /// declared set of the fn type it was checked against, or the set
    /// inferred from its body. Both emitters thread these *into* the
    /// closure instead of capturing them.
    pub lambda_effects: HashMap<Key, Vec<Ty>>,
    /// [qual-widen] The type a `^` check widens its subject to, keyed by the
    /// check's span (a `^` expression, or a `^` branch head in a `when`).
    /// Present only when the check *peels a wrapper arm*, which is when the
    /// emitters must materialize the peel: they bind the widened value to a
    /// shadowing local for the branch, so reads and any nested `when` see it
    /// at this type.
    pub widen_targets: HashMap<Key, Ty>,
    /// Calls that may throw [throw], keyed by the call span: the `throw`
    /// operation itself and every call whose callee declares `[Throw<M>]`.
    /// The emitters need each one because the control transfer is *their*
    /// job: a Rust `ControlFlow::Break` (or a labelled-block break), a
    /// Kotlin `throw`.
    pub may_throw: HashMap<Key, ThrowSite>,
    /// Per-fn deduction facts [deduce-infer]: for each parameter, whether
    /// a call gives the value back to the caller and which of its declared
    /// qualifiers are still known afterwards. Written lists are stored as
    /// validated; unwritten lists are inferred from the body. The Rust
    /// backend's ownership/borrow contract (Kotlin ignores them).
    pub deductions: HashMap<FnKey, Vec<crate::deduce::ParamDeduction>>,
    /// Fn-name references [fn-ref-table]: the span of a fn *name* — at
    /// its declaration, as a call-site callee, or a fn-by-name use —
    /// mapped to the declaration it resolves to. Drives the LSP's
    /// signature hover.
    pub fn_refs: HashMap<Key, FnKey>,
    /// Non-fn name references [lsp-definition]: the span of a type,
    /// struct, effect, handler, qualifier, type-alias, or effect-member
    /// name mapped to where its declaration's identifier is written.
    /// Drives go-to-definition for everything `fn_refs` does not cover.
    pub def_refs: HashMap<Key, DefSite>,
    /// Field accesses [lsp-definition]: the span of the field *name* in
    /// `base.field` mapped to where the field's declaration is written.
    /// Drives go-to-definition and doc hover on fields [doc-comment].
    pub field_refs: HashMap<Key, DefSite>,
    /// Reads of fate-linked (derived) variables [fate-link], keyed by the
    /// identifier span: the roots the variable shares fate with, and
    /// where each link was bound. Presentation-only: tooling renders the
    /// variable's type with a bare `proj` compiler qualifier and
    /// serves the parameters (roots, binding sites) as on-request detail
    /// (user decision 2026-09-02, progressive disclosure).
    pub fate_reads: HashMap<Key, Vec<FateRead>>,
    /// [interp-to-str] String interpolations whose value is not natively
    /// renderable, keyed by the *interpolated expression's* span: the
    /// `to_str` the checker resolved for it at that site. The emitters call
    /// it instead of formatting the value directly.
    pub interp_to_str: HashMap<Key, FnKey>,
    /// [proj-anywhere] Derived-return calls whose borrowed argument is a
    /// *temporary*: usable within the statement, but binding, returning or
    /// storing the view is an error (it would outlive what it borrows).
    pub temp_views: HashSet<Key>,
    /// [proj-infer] Calls whose result *holds borrows* of some arguments
    /// (a fn returning a struct with `proj` fields), keyed by the call span:
    /// the indices of the lent arguments (dot-notation receivers are
    /// argument 0). The checker links the result to them, held; the Rust
    /// backend ties the result's lifetime to those parameters.
    pub lending_calls: HashMap<Key, Vec<usize>>,
    /// [proj-infer] Per fn, the parameters its result holds borrows of
    /// (declared with `[p: proj]` or inferred from the body). Read by the
    /// Rust backend to name the lifetime on lent parameters and the return.
    pub fn_lends: HashMap<FnKey, Vec<usize>>,
    /// [interp-struct] Interpolations of a **struct** with no `to_str` of its
    /// own, whose every field renders natively: the struct's name, keyed by
    /// the interpolated expression's span. The emitters render it field-wise
    /// (`Person { name: ann, age: 3 }`) — the same text on both, which is why
    /// the *language* fixes the format rather than deferring to Rust's
    /// `Debug` or Kotlin's `toString` (those disagree).
    pub interp_struct: HashMap<Key, String>,
    /// Move-mode bind events [fate-move-mode]: the spans of `let`/
    /// assignment statements (and, for `for` loops, of the iterable
    /// expression) where the binding took ownership of the bound value —
    /// the ancestors were consumed at the binding. The Rust backend
    /// emits these as real moves instead of clones.
    pub binding_modes: HashSet<Key>,
    /// Projection expressions in *moved* positions whose provenance
    /// roots were consumed [fate-move-mode]: the Rust backend may render
    /// the place directly (a partial move) instead of cloning.
    pub moved_projections: HashSet<Key>,
    /// The captured outer locals of each lambda [fate-lambda], keyed by
    /// the lambda expression's span (for tooling and future emission
    /// refinements; both emitters currently capture lexically, which is
    /// an alias on both backends).
    pub lambda_captures: HashMap<Key, Vec<LambdaCapture>>,
    /// Calls to fns with a derived return (`proj[from: param]`
    /// [readonly-return]), keyed by the call span: the index of the
    /// argument the result borrows. The checker links the result to that
    /// argument; the Rust backend renders the result as a borrow.
    pub derived_calls: HashMap<Key, Vec<usize>>,
    /// Calls *through fn-typed values* [fn-contract], keyed by the call
    /// span: the effective per-argument contract (post-default), for the
    /// Rust backend's argument rendering.
    pub fn_value_calls: HashMap<Key, Vec<FnParamContract>>,
    /// Lambdas checked against a *contracted* fn type [fn-contract],
    /// keyed by the lambda span: the contract, for the Rust backend's
    /// parameter-binding modes.
    pub lambda_contracts: HashMap<Key, Vec<FnParamContract>>,
    /// The implicit parameters of each fn, expanded [implicit-param]
    /// [implicit-group]: the written `?name: FnType` ones in order, then each
    /// `?Group<T>` spread's members in declaration order. This is the single
    /// ordered list both sides render — the callee's trailing parameters and
    /// the caller's trailing arguments — so neither emitter needs to know
    /// that groups exist.
    pub implicit_params: HashMap<FnKey, Vec<ImplicitParam>>,
    /// What fills each implicit parameter at a call [implicit-resolve],
    /// keyed by the call span, in the callee's declared order.
    pub implicit_args: HashMap<Key, Vec<ImplicitArg>>,
    /// The type arguments a `use` gives its handler
    /// [effect-handler-generics], in the handler's declaration order: from
    /// the written list (`use Plain<Int>()`) and from what the constructor
    /// arguments bind. The emitters need them because a generic handler has
    /// to be constructed *at* a type, and neither target can infer one from
    /// an empty argument list.
    pub use_handler_args: HashMap<Key, Vec<Ty>>,
    /// The implicit parameters of each *effect member* [implicit-param]:
    /// members have no `FnKey`, so they are keyed by the span of the member's
    /// name. The emitters read this where they render the member — the
    /// interface or trait method, every handler's implementation of it, and
    /// the host skeleton — since an implicit is part of the member's
    /// signature like any other parameter.
    pub implicit_members: HashMap<Key, Vec<ImplicitParam>>,
    /// Type errors, structured for CLI/LSP consumption [diag-structured].
    pub errors: Vec<FileDiagnostic>,
}

/// Why an implicit parameter could not be resolved [implicit-resolve]. The
/// distinction *is* the diagnostic: "nothing of that name" and "declared,
/// but its contract does not fit" need different words, and the second
/// difference is invisible in a printed type.
enum ImplicitMiss {
    /// No fn of that name is visible.
    Unknown,
    /// One is, and here is why it does not fit.
    NearMiss(String),
    /// Several match, so choosing would be a guess.
    Ambiguous(usize),
}

/// One implicit parameter of a fn [implicit-param], after group expansion.
#[derive(Clone, Debug, PartialEq)]
pub struct ImplicitParam {
    /// The name that resolution, forwarding and a call-site override all
    /// key on — a member's own name, with no group qualifier: dropping the
    /// binder is what lets an inner fn declare `?add` directly, or reach the
    /// same parameter through a different grouping (user decision
    /// 2026-09-05).
    pub name: String,
    /// The fn type it must be filled with, as declared (generics
    /// unsubstituted).
    pub ty: Ty,
    /// Where it was written: the parameter, or the `?Group<T>` spread.
    pub span: Span,
    /// [proj-anywhere] Which union arms of the member's *written* return
    /// type carry `proj` — a borrow the lowered `Ty` no longer shows. `Yield`'s
    /// `next` has `[0]`: `Emitted (proj[from: it] T) | Finished`. A backend
    /// that distinguishes borrows from values (Rust) renders those arms as
    /// references. Empty for a non-union return or a plain implicit.
    pub borrowed_arms: Vec<usize>,
}

/// What a call site puts in an implicit parameter [implicit-resolve].
#[derive(Clone, Debug, PartialEq)]
pub enum ImplicitArg {
    /// The caller wrote `name = value`: the value is in the call's `named`
    /// list, and the emitters render it like any argument. The arity comes
    /// along because a backend may have to adapt a bare fn name into a
    /// closure, and only the parameter's type knows how many arguments it
    /// takes.
    Given { name: String, arity: usize },
    /// Forwarded from an implicit parameter of the enclosing fn, which has
    /// the same name and a matching type [implicit-forward]. Inside generic
    /// code this is the only possibility, since nothing about an opaque `T`
    /// is knowable [call-resolve].
    Forwarded { name: String },
    /// Resolved to a declared fn, by name and type [implicit-resolve].
    Resolved {
        name: String,
        key: FnKey,
        /// [copy-implicit] The fn type the position wanted, with the call's
        /// type arguments substituted — the *concrete* type a shape-lowered
        /// intrinsic (`copy`) needs to render itself as a value.
        want: Ty,
    },
    /// [iter-fn] The position wants a pass's `next` and the argument
    /// was an **origin**, so the pass is the hidden machine minted at that
    /// argument (user decision 2026-09-09). There is no declared fn to name:
    /// the emitters wrap the machine's own advance into the protocol's two
    /// arms, using `next_fn` — the `yield fn`'s key — for the machine's name
    /// and its effect order, exactly as a `for` over the origin does.
    OriginNext { name: String, next_fn: FnKey },
}

/// One fate link of a derived variable, exposed for tooling
/// [fate-link]: the root variable's name and the span of the binding
/// that created the link.
#[derive(Clone, Debug, PartialEq)]
pub struct FateRead {
    pub root: String,
    pub bind_span: Span,
    /// [fate-field-disjoint] The projection the value was taken from,
    /// rendered (`.name`, or empty for the whole variable). `None` when the
    /// derivation is not a plain projection chain. Tooling appends it to
    /// `root` so the hover names the storage actually shared — since L5,
    /// "shares fate with `p`" would overstate a link to `p.name` alone.
    pub path: Option<String>,
}

/// One captured variable of a lambda [fate-lambda]: the outer local the
/// body mentions, and whether the capture *consumed* it (the body
/// mutates it, so the closure took ownership at creation).
#[derive(Clone, Debug, PartialEq)]
pub struct LambdaCapture {
    pub name: String,
    pub consumed: bool,
    /// [fate-move-mode] The captured value is transitively **mutable**, so
    /// whether the closure holds a snapshot or an alias is observable.
    pub mutable: bool,
    /// The closure **writes** through this capture — including handing it to
    /// a `Mut` parameter. True even for a value whose type is not itself
    /// mutable (`let i = 0` reassigned inside the lambda), which is why it
    /// is a flag of its own rather than implied by `mutable`.
    pub mutated: bool,
}

impl Checked {
    pub fn ty_of(&self, file: usize, span: Span) -> Option<&Ty> {
        self.expr_ty.get(&(file, span))
    }
}

/// Checks the whole program (resolution must come from the same program).
///
/// Runs in two rounds so that *inferred* deductions are enforced at call
/// sites exactly like declared ones [deduce-consume]: round one checks
/// with only declared lists and runs whole-program deduction inference
/// [deduce-infer]; round two re-checks with the inferred lists injected
/// (moves consume arguments, kept parameters shed their removal set) and
/// re-infers against the final call resolutions. Round one's diagnostics
/// are discarded — checking is deterministic, so round two re-derives
/// them.
pub fn check_program<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &Symbols<'p>,
) -> Checked {
    // [fate-move-mode] S2 state carried across the rounds: bind events
    // whose bound variable an earlier round saw moved or mutated
    // (move-mode candidates), and parameters claimed by such bindings
    // (the binding takes ownership, so the parameter is moved — seeded
    // into deduction inference between the rounds).
    let mut candidates: HashSet<Key> = HashSet::new();
    let mut claims: HashMap<FnKey, HashSet<String>> = HashMap::new();
    // [deduce-syntax] Parameters whose data the body mutates: those may
    // not keep "everything else", since mutation can invalidate
    // qualifiers the signature never mentions.
    let mut mutations: HashMap<FnKey, HashSet<String>> = HashMap::new();
    // [actor-replyto] Handlers whose members mint a self-targeted
    // continuation. Only grows, and complete after round one — so the `use`
    // refusal it drives lands in the round whose diagnostics are kept.
    let mut parking: HashSet<String> = HashSet::new();

    // [qual-refn] Refinements are resolved and validated once: they depend
    // on declarations and per-file visibility only, not on anything a
    // checking round produces. Every round replays their diagnostics and
    // reads their table, so the two sides of a refinement — what it means
    // at a call site and what it means for an inferred contract — cannot
    // come from different data.
    let refinements = crate::refine::collect(program, resolution);

    // Round one: no inferred facts yet (strict S1 behavior).
    let mut out = check_once(
        program,
        resolution,
        symbols,
        None,
        &refinements,
        &mut candidates,
        &mut claims,
        &mut mutations,
        &mut parking,
    );
    crate::deduce::infer(program, &mut out, &claims, &mutations);
    let mut inferred = std::mem::take(&mut out.deductions);
    let mut prev_candidates = candidates.clone();
    let mut prev_claims = claims.clone();
    let mut prev_mutations = mutations.clone();

    // [deduce-fixpoint] Iterate check → infer until the driving facts
    // stabilize (inferred deductions, move-mode candidates, parameter
    // claims — checking is deterministic in these inputs), capped at
    // MAX_ROUNDS total checking rounds (decision L3a: extra rounds run
    // only when facts changed, so stable programs stay at two rounds; a
    // program still unstable at the cap gets a deterministic error
    // naming the oscillating fns, with the written-list remedy). Each
    // round's diagnostics replace the previous round's (re-derived
    // identically for the stable part).
    const MAX_ROUNDS: usize = 4;
    let mut round = 1;
    loop {
        round += 1;
        out = check_once(
            program,
            resolution,
            symbols,
            Some(&inferred),
            &refinements,
            &mut candidates,
            &mut claims,
            &mut mutations,
            &mut parking,
        );
        // Deductions are a whole-program fact (strictest over the call
        // graph), re-inferred against this round's final call
        // resolutions [deduce-infer].
        crate::deduce::infer(program, &mut out, &claims, &mutations);
        let stable = out.deductions == inferred
            && candidates == prev_candidates
            && claims == prev_claims
            && mutations == prev_mutations;
        if stable {
            return out;
        }
        if round >= MAX_ROUNDS {
            // Make the instability visible [deduce-fixpoint]: name every
            // fn whose inferred contract still changed in the last round.
            let mut unstable: Vec<FnKey> = out
                .deductions
                .iter()
                .filter(|(key, ded)| inferred.get(key) != Some(ded))
                .map(|(key, _)| *key)
                .collect();
            unstable.sort_by_key(|k| (k.file, k.item));
            for key in unstable {
                if let Some(Item::Fn(f)) = program.modules[key.file].items.get(key.item) {
                    out.errors.push(FileDiagnostic::error(
                        key.file,
                        f.name.span,
                        format!(
                            "the inferred deductions of `{}` did not stabilize after \
                             {MAX_ROUNDS} checking rounds (its contract oscillates \
                             with overload resolution); write the deduction list \
                             explicitly",
                            f.name.name
                        ),
                    ));
                }
            }
            return out;
        }
        inferred = std::mem::take(&mut out.deductions);
        prev_candidates = candidates.clone();
        prev_claims = claims.clone();
        prev_mutations = mutations.clone();
    }
}

/// One checking round; `inferred` carries the previous round's deduction
/// facts for fns without a written list [deduce-consume].
fn check_once<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &Symbols<'p>,
    inferred: Option<&HashMap<FnKey, Vec<crate::deduce::ParamDeduction>>>,
    refinements: &crate::refine::Refinements,
    move_candidates: &mut HashSet<Key>,
    param_claims: &mut HashMap<FnKey, HashSet<String>>,
    param_mutations: &mut HashMap<FnKey, HashSet<String>>,
    parking_handlers: &mut HashSet<String>,
) -> Checked {
    let mut out = Checked::default();
    out.errors.extend(resolution.errors.iter().cloned());
    // [qual-refn] Refinement validation is round-independent; its
    // diagnostics are replayed here so they land with everything else.
    out.errors.extend(refinements.errors.iter().cloned());
    out.refinements = refinements.clone();
    for (file_idx, (_file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        let mut checker = Checker {
            scope: &resolution.scopes[file_idx],
            resolution,
            symbols,
            inferred,
            refinements,
            refn_warned: HashSet::new(),
            file_idx,
            out: &mut out,
            locals: Vec::new(),
            generics: HashSet::new(),
            ret_ty: Ty::none(),
            own_qualifiers: HashSet::new(),
            effect_env: Vec::new(),
            handler_deps: Vec::new(),
            handler_ofs: Vec::new(),
            own_handler: None,
            handler_spawns: false,
            handler_waits: false,
            facade_handler: None,
            confined_state: Vec::new(),
            own_task: None,
            can_use: false,
            can_spawn: false,
            can_wait: false,
            loop_stack: Vec::new(),
            driven_origins: Vec::new(),
            next_var_id: 0,
            move_candidates,
            param_claims,
            parking_handlers,
            param_mutations,
            own_fn: None,
            own_contract: None,
            own_discharges: std::collections::HashSet::new(),
            own_written: Vec::new(),
            lambda_ctx: Vec::new(),
            lambda_links: HashMap::new(),
            assign_target: false,
            projection_base: 0,
            is_std: _file.is_std,
            own_linear_generics: HashSet::new(),
            own_derived_return: None,
            own_derived_sources: Vec::new(),
            lends_memo: HashMap::new(),
            own_lends: Vec::new(),
            own_lends_declared: false,
            own_proj_arms: Vec::new(),
            lending_ctor: None,
            own_implicits: Vec::new(),
            renames: Vec::new(),
            try_stack: Vec::new(),
            effect_uses: Vec::new(),
        };
        checker.check_module(ast);
    }
    check_intrinsic_is_std_only(program, &mut out);
    // [actor-deadlock-cycle] The static deadlock baseline, over the whole
    // program: a cycle of waiting actors is an error, a cycle of blocking
    // sends a warning. Last, because it reads what this round's checking
    // recorded (`actor_gates`, `actor_sends`) — and inside the round, so its
    // diagnostics land with the round whose diagnostics are kept.
    crate::deadlock::check(program, symbols, &mut out);
    out
}

/// [intrinsic-std-only] `intrinsic` marks a declaration the *compiler*
/// implements, so only the standard library may write it (user decision
/// 2026-09-05: a customer has no business declaring `intrinsic` if the
/// compiler does not already declare it).
///
/// The parser cannot enforce this — it sees one file's tokens and knows
/// nothing about where the file came from — so the rule lives here, where
/// `SourceFile::is_std` is at hand. It is not a cosmetic restriction: the
/// backends dispatch intrinsics from a table keyed by *name*
/// [backend-intrinsic], so an intrinsic the compiler does not know has no
/// lowering anywhere. The error names the one interop path customer code
/// does have.
fn check_intrinsic_is_std_only(program: &Program, out: &mut Checked) {
    for (file_idx, (file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        if file.is_std {
            continue;
        }
        for item in &ast.items {
            let (kind, name, span) = match item {
                Item::Fn(f) if f.intrinsic => ("fn", &f.name.name, f.name.span),
                Item::Type(t) if t.intrinsic => ("type", &t.name.name, t.name.span),
                Item::Handler(h) if h.intrinsic => ("handler", &h.name.name, h.name.span),
                Item::Qualifier(q) if q.intrinsic => ("qualifier", &q.name.name, q.name.span),
                _ => continue,
            };
            out.errors.push(crate::diag::FileDiagnostic::error(
                file_idx,
                span,
                format!(
                    "`intrinsic {kind} {name}` is the compiler's to declare, not \
                     yours: every backend lowers intrinsics from a table keyed by \
                     name, so one declared here has no implementation anywhere. To \
                     reach the target language, declare what you need from it as a \
                     member of a `platform effect`"
                ),
            ));
        }
    }
}

/// [effect-member-unique] A member name is unique **within its effect**.
/// Across effects the name may recur ([effect-member-overload], user
#[derive(Clone)]
struct LocalVar {
    declared: Ty,
    narrowed: Ty,
    /// Unique id of this binding (stable across scopes; names can recur
    /// in sibling scopes) [fate-link].
    id: u32,
    /// Fate links [fate-link]: the variables this one was bound from (a
    /// bare identifier or projection), flattened transitively. A linked
    /// (derived) variable is read-only in S1 [fate-derived-readonly] and
    /// is poisoned when a root is mutated or moved [fate-poison].
    links: Vec<FateLink>,
    /// Why this variable is unusable (set together with
    /// `narrowed = Nothing` when a fate root is mutated/moved/reassigned)
    /// [fate-poison].
    poison: Option<Poison>,
    /// What consumed this variable (set together with
    /// `narrowed = Nothing` when the variable itself was moved: a call,
    /// `return`/`break`/`yield`, a literal store, spread, or a `use`
    /// handler registration) — names the event in the use-site
    /// diagnostic [deduce-consume].
    consumed_by: Option<&'static str>,
    /// [linear-union-arm] The obligation of a linear-union value was
    /// discharged by narrowing to a non-linear arm on every path
    /// reaching here (set per-branch by `with_narrows`, joined by
    /// `merge_fallthrough`).
    linear_settled: bool,
    /// Whether this variable is a parameter (or handler state field) of
    /// the enclosing fn: a parameter root is *owned* for move-mode
    /// bindings only when the fn's effective contract moves it
    /// [fate-move-mode]; locals are always owned.
    is_param: bool,
    /// For `for`-loop bindings: the span of the iterable expression.
    /// When a move-mode binding consumes such a root, the loop itself
    /// becomes move-mode (it iterates by value) and this span keys the
    /// emitter's `binding_modes` entry [fate-move-mode].
    for_origin: Option<Span>,
    /// Where the variable was declared (for linear-obligation
    /// diagnostics [linear-obligation]).
    decl_span: Span,
    /// A lambda parameter whose fn-type contract *keeps* it
    /// [fn-contract]: the value belongs to the caller — read-only-ish
    /// (mutation is type-gated by `Mut`), never consumable.
    lambda_kept: bool,
    /// A handler *state* field [effect-handler]: it outlives every member
    /// call, so assigning a value into it is a **store** — the handler
    /// takes ownership, exactly as a struct literal does
    /// [effect-state-store]. Ordinary locals link instead [fate-link].
    is_handler_state: bool,
    /// [qual-widen] While a `^` check holds, the *physical* view of this
    /// variable: the arm test peeled a wrapper, so reads and any further
    /// narrowing compose on the inner value rather than on the storage.
    /// `None` outside a widening branch (`declared` is then the view). The
    /// emitters materialize the peel as a branch-local temporary.
    widened: Option<Ty>,
    /// Narrowed *projections* out of this variable [flow-place]: flow
    /// facts about `h.field`, keyed by the projection path. The
    /// variable's own narrowing stays in `narrowed`; keeping the
    /// projections here is what makes an event on the root — mutation,
    /// reassignment, a move — invalidate every fact below it, and what
    /// keeps `snapshot_narrows`/`restore_narrows`/`merge_fallthrough` the
    /// single source of truth for flow state.
    place_narrows: Vec<PlaceNarrow>,
    /// [unused-var] Whether the variable has been *read*. Assignment is
    /// not a read: `let x = 1` followed only by `x = 2` never uses the
    /// value. Checked when the scope closes; a `_` prefix opts out.
    used: bool,
    /// [fate-partial-move] Projections that have been **moved out** of
    /// this variable. The variable stays usable: a read of a disjoint
    /// projection passes, a read overlapping a moved one (or of the whole
    /// value) is an error naming what left. Union-merged across branches
    /// (moved on any path is moved), cleared per place by reassignment.
    moved_places: Vec<MovedPlace>,
}

/// One narrowed projection place [flow-place]: the path out of the
/// variable, the type reads of that place have while the fact holds, and
/// the *declared* (physical) type the storage keeps — narrowing never
/// re-wraps storage, so reads unwrap from `declared` to `narrowed` and
/// both the checker (`repr_ty`) and the emitters need it.
#[derive(Clone, PartialEq)]
struct PlaceNarrow {
    path: Vec<Step>,
    narrowed: Ty,
    declared: Ty,
}

/// [fate-partial-move] One projection that has been **moved out** of a
/// variable: the path out of the root, and where it left. The root itself
/// stays usable — reading a *disjoint* projection is fine, reading this one
/// (or a place overlapping it, or the whole value) is an error.
#[derive(Clone, Debug, PartialEq)]
struct MovedPlace {
    path: Vec<Step>,
    span: Span,
}

impl MovedPlace {
    /// Whether a use of `path` is blocked by this move: the two paths
    /// overlap ([`Place::overlaps`]), so the use could read storage that
    /// has left. A use of the *whole* variable (`[]`) is a prefix of every
    /// moved path and so is always blocked.
    fn blocks(&self, path: &[Step]) -> bool {
        let as_place = |p: &[Step]| Place {
            root: String::new(),
            path: p.to_vec(),
        };
        as_place(&self.path).overlaps(&as_place(path))
    }

    /// How the moved projection reads in a diagnostic: `.tags`, or "it"
    /// for the whole value.
    fn describe(&self) -> String {
        if self.path.is_empty() {
            "it".to_string()
        } else {
            self.path.iter().map(|p| p.to_string()).collect()
        }
    }
}

/// One fate link [fate-link]: the derived variable was bound from (a
/// projection of) the root variable at `bind_span`.
#[derive(Clone, Debug, PartialEq)]
struct FateLink {
    root_id: u32,
    root_name: String,
    bind_span: Span,
    /// [fate-field-disjoint] **Which projection of the root** the derived
    /// value came from, as the path out of it: `let n = p.name` links with
    /// `[.name]`, `let q = p` with `[]` (the whole variable). `None` when
    /// the derivation is not a plain projection chain (a `!` unwrap, say) —
    /// conservative: an unknown path overlaps every event.
    ///
    /// An event poisons this link only when the two paths *overlap*
    /// ([`Place::overlaps`], the substrate P1 built), so mutating `p.tags`
    /// leaves a value derived from `p.name` alone.
    path: Option<Vec<Step>>,
    /// The link passes through a derived-return call [readonly-return]:
    /// the value is *physically borrowed*, so move-mode can never take
    /// ownership through it [fate-move-mode].
    borrowed: bool,
    /// [proj-infer] The borrow is *held by fields*: the variable is an owned
    /// object of its own (a struct with `proj` fields — a pass over a list)
    /// whose fields project the root, rather than the root's data itself.
    /// Such a variable may be mutated (its own fields are its own) and moved
    /// (the object travels, the borrow with it); what it may not do is
    /// outlive the root. `false` for a wholesale projection (`get(xs, i)`,
    /// a `for` element of a view), which *is* the root's data
    /// [proj-readonly].
    held: bool,
}

impl FateLink {
    /// [fate-field-disjoint] Whether an event on `event_path` reaches this
    /// link. Either path unknown is conservative (they overlap): a
    /// derivation the analysis cannot spell must not be given precision it
    /// has not earned.
    fn overlaps_event(&self, event_path: Option<&[Step]>) -> bool {
        let (Some(link), Some(event)) = (self.path.as_deref(), event_path) else {
            return true;
        };
        // Same root by construction (the caller matched `root_id`), so the
        // paths alone decide.
        let as_place = |path: &[Step]| Place {
            root: String::new(),
            path: path.to_vec(),
        };
        as_place(link).overlaps(&as_place(event))
    }
}

/// What happened to a fate root [fate-poison].
#[derive(Clone, Copy, Debug, PartialEq)]
enum FateEvent {
    Moved,
    Mutated,
    Reassigned,
    /// A move-mode binding took ownership of the value [fate-move-mode]:
    /// the poisoned variable is the *root*, and `Poison::root_name`
    /// carries the binding's name (the diagnostic tells the story in
    /// the binding→root direction).
    BoundAway,
}

impl FateEvent {
    fn describe(self) -> &'static str {
        match self {
            FateEvent::Moved => "moved",
            FateEvent::Mutated => "mutated",
            FateEvent::Reassigned => "reassigned",
            FateEvent::BoundAway => "bound away",
        }
    }
}

/// The reason a derived variable became unusable [fate-poison].
#[derive(Clone, Debug, PartialEq)]
struct Poison {
    root_name: String,
    event: FateEvent,
    event_span: Span,
}

/// Flow state of one local: narrowed type plus fate links/poison
/// [deduce-consume] [fate-link], and the narrowed projections out of it
/// [flow-place].
#[derive(Clone, PartialEq)]
struct VarState {
    narrowed: Ty,
    /// [linear-union-arm] The path narrowed a linear union to a
    /// non-linear arm, discharging the obligation — a flow fact, like
    /// consumption, surviving the branch's narrow-restore.
    linear_settled: bool,
    links: Vec<FateLink>,
    poison: Option<Poison>,
    consumed_by: Option<&'static str>,
    place_narrows: Vec<PlaceNarrow>,
    /// [fate-partial-move] Moved-out projections, union-merged across
    /// branches: the dual of `place_narrows`, which intersects.
    moved_places: Vec<MovedPlace>,
}

/// Flow state of every local, per scope frame [deduce-consume].
type NarrowSnapshot = Vec<HashMap<String, VarState>>;

/// An `is`/`when`/`for` binding to declare in a branch scope: name, type,
/// the fate links inherited from the subject [fate-link], and — for
/// `for`-loop bindings — the span of the iterable expression
/// [fate-move-mode].
type Binding = (Ident, Ty, Vec<FateLink>, Option<Span>);

struct Checker<'p, 'r> {
    scope: &'r ModuleScope<'p>,
    /// The whole-program resolution (for import suggestions on
    /// unresolved names [diag-import-suggest]).
    resolution: &'r Resolution<'p>,
    /// Round one's inferred deduction facts, enforced at call sites for
    /// fns without a written list [deduce-consume]. `None` in round one.
    inferred: Option<&'r HashMap<FnKey, Vec<crate::deduce::ParamDeduction>>>,
    /// [qual-refn] Refinements applying per (file, callee), resolved once
    /// for the whole program.
    refinements: &'r crate::refine::Refinements,
    /// Call sites already warned about a suppressed refinement conflict
    /// [qual-refn-conflict], keyed by (callee, parameter): the conflict is
    /// a property of the file and the callee, not of the call, so it is
    /// reported once rather than at every call.
    refn_warned: HashSet<(FnKey, String)>,
    symbols: &'r Symbols<'p>,
    file_idx: usize,
    out: &'r mut Checked,
    /// Lexical scope stack of local variables (params + lets + bindings).
    locals: Vec<HashMap<String, LocalVar>>,
    /// Generic type parameters currently in scope.
    generics: HashSet<String>,
    /// Return type of the function being checked.
    ret_ty: Ty,
    /// Names of qualifiers declared in the file currently being checked
    /// (constructive-qualifier constructors must live in this file).
    own_qualifiers: HashSet<String>,
    /// Effect instances available to the code being checked: the current
    /// fn's declared effect dependencies plus `use`d handlers. Entries
    /// added by `use` are truncated at block boundaries.
    effect_env: Vec<Ty>,
    /// [effect-handler-deps] The effects declared by the handler whose
    /// members are being checked (`handler H [E1, E2] of E`), empty
    /// elsewhere. A member may use them exactly as if it had declared them,
    /// which it may not ([effect-member-no-effects]).
    handler_deps: Vec<Ty>,
    /// [effect-handler-deps] The effect implemented by the handler whose
    /// members are being checked, so a member calling *its own* effect can be
    /// told what is actually wrong: self-dispatch is not a feature yet, and
    /// neither remedy the general "no handler" diagnostic names is available
    /// inside a member.
    /// [effect-handler-multi] The effects the handler being checked
    /// implements, in declaration order — empty outside a handler. Several is
    /// the multi-face case, and every rule that used to read "the effect this
    /// handler implements" now reads "any of them".
    handler_ofs: Vec<Ty>,
    /// [actor-replyto] The handler whose members are being checked, so a
    /// `replyto k(...)` can find `k`: a continuation targets a member of the
    /// *enclosing* handler, which is what makes the form legal only inside
    /// one.
    own_handler: Option<&'p ast::HandlerDecl>,
    /// [actor-spawn-effect] Whether the handler whose members are being
    /// checked declared `spawn` among its dependencies. Its members inherit
    /// the capability, exactly as they inherit its effects.
    handler_spawns: bool,
    /// [task-mint] The **free `send fn`** whose body is being checked, if any:
    /// what its sends and its own mints are attributed to.
    own_task: Option<String>,
    /// [waitfor-effect] Whether the handler being checked declared `waitfor`
    /// among its dependencies. Its members inherit the capability, and its
    /// *spawn site* pays for it: the placement must be a `Dedicated Pool`.
    handler_waits: bool,
    /// [mixed-handler] The handler whose **sync member** is being checked,
    /// when that handler is mixed (plain faces + `send fn` members, SH-1):
    /// the member is the façade, running on the caller's thread over the
    /// façade value. What it changes: bare calls to the handler's own send
    /// members resolve as sends to the servant, state fields are out of
    /// scope (`confined_state` carries their names for the diagnostic),
    /// handler dependencies are invisible (they live in the servant), and a
    /// `waitfor` needs no capability (SH-5(d)'s down payment — the façade
    /// does not run on the spawned thread at all).
    facade_handler: Option<&'p ast::HandlerDecl>,
    /// [mixed-handler] The state field names confined to the servant while a
    /// façade member is checked — consulted by the unresolved-name path, so
    /// touching one is the confinement diagnostic rather than an "unknown
    /// variable".
    confined_state: Vec<String>,
    /// Whether the current fn declared the special `use` effect.
    can_use: bool,
    /// [actor-spawn-effect] Whether the current fn (or the handler whose
    /// member it is) declared the `spawn` capability — the gate a `spawn`
    /// expression checks.
    can_spawn: bool,
    /// [waitfor-effect] Whether the current fn (or the handler whose member
    /// it is) declared the `waitfor` capability — the gate a `waitfor`
    /// expression checks. What makes holding it safe is *placement*: whatever
    /// carries it runs on a `Dedicated Pool` [waitfor-dedicated], and `main`'s
    /// own thread is one.
    can_wait: bool,
    /// Enclosing loops of the code being checked; `break`/`continue`
    /// statements record their value contributions into the innermost
    /// entry [while-value]. Lambda bodies are a barrier.
    loop_stack: Vec<LoopCtx>,
    /// [iter-fn] Origins whose `for` loops are currently open: the
    /// root variable's name and the span to blame. A hidden machine reads
    /// its origin *across suspensions*, so a write while the loop runs has
    /// two defensible meanings — the machine's own copy, or the caller's
    /// object — and the two backends each pick one. Refused rather than
    /// sided with [backend-parity], which is also what keeps a value
    /// *derived* from the origin valid for the whole drive.
    driven_origins: Vec<(String, Span)>,
    /// Fresh-id counter for local bindings [fate-link].
    next_var_id: u32,
    /// Move-mode candidates [fate-move-mode]: bind events (keyed by bind
    /// span) whose bound variable round one saw moved or mutated —
    /// collected at `error_derived`, the single choke point for every
    /// derived move/mutation (round one's errors are discarded). Round
    /// two applies move-mode at these bind sites.
    move_candidates: &'r mut HashSet<Key>,
    /// Parameters claimed by move-mode bindings and moved-position
    /// projections [fate-move-mode]: the binding takes ownership, so the
    /// parameter is moved. Only recorded for fns *without* a written
    /// deduction list; seeded into deduction inference between rounds.
    param_claims: &'r mut HashMap<FnKey, HashSet<String>>,
    /// [deduce-syntax] Parameters whose data the body mutates — recorded
    /// for *every* fn (written lists included), since the written-list
    /// validation needs them.
    param_mutations: &'r mut HashMap<FnKey, HashSet<String>>,
    /// [actor-replyto] Handlers whose member bodies mint a **self**-targeted
    /// continuation — the names of every handler in which a `replyto` /
    /// `replyto!` resolved lexically. Collected across rounds like
    /// `move_candidates`, and for the same reason: a `use` may be checked
    /// before the handler it names, so round one fills this and round two
    /// (whose diagnostics are the ones kept) refuses.
    ///
    /// The refusal is [actor-replyto]'s: parking is the one thing only a
    /// actor can do, so a parking handler may only be `spawn`ed. Using the
    /// checker's own traversal to find the mints — rather than a tenth
    /// exhaustive expression walker — is what keeps this complete as the
    /// grammar grows.
    parking_handlers: &'r mut HashSet<String>,
    /// The fn currently being checked, when it is a top-level `fn` item
    /// (member fns have no key).
    own_fn: Option<FnKey>,
    /// The current fn's effective deduction contract (written list, else
    /// the previous round's inferred facts): decides whether a parameter
    /// root is *owned* (moved by the contract) for move-mode bindings
    /// [fate-move-mode]. `None` for member fns and in round one.
    own_contract: Option<Vec<crate::deduce::ParamDeduction>>,
    /// [linear-group] The parameter name of the designated `close` currently
    /// being checked, if this fn is one: its obligation is discharged by
    /// being closed.
    /// [linear-group] The linear types the *current fn* may `discard`: it
    /// is a **discharger** of each — declared in the same file as the type,
    /// consuming a parameter of it (user decision 2026-09-12; replaces the
    /// designated-`close` exemption). Inside the fn the obligation still
    /// owes until terminated on every path — `discard` or a forward into
    /// another consuming fn.
    own_discharges: std::collections::HashSet<String>,
    /// Whether the current fn has a *written* deduction list: written
    /// contracts never gain claims — a kept parameter stays kept and
    /// derived moves stay errors [fate-derived-readonly].
    /// [deduce-syntax] The parameters the current fn's clause *mentions*
    /// (plain entries): their contract is written, fixed; every other
    /// parameter's is inferred.
    own_written: Vec<String>,
    /// Enclosing lambdas of the code being checked [fate-lambda]: the
    /// scope-frame boundary of each (locals below it are *captures*)
    /// plus the captures recorded so far. Innermost last.
    lambda_ctx: Vec<LambdaCtx>,
    /// Fate links carried by lambda *values* [fate-lambda]: a lambda is
    /// derived from the transitively-mutable variables it reads (keyed
    /// by the lambda expression's span, this file only).
    lambda_links: HashMap<Span, Vec<FateLink>>,
    /// [unused-var] Whether this file is part of the embedded std: an
    /// unused-variable warning there is the compiler's own business, not
    /// the user's, so std is exempt.
    is_std: bool,
    /// [fate-partial-move] Whether the expression being checked is an
    /// assignment *target*: writing a place puts data back, so it is not a
    /// read of moved-out storage and must not be reported as one.
    assign_target: bool,
    /// [fate-partial-move] Nesting depth of *projection bases* being
    /// checked: inside `p.name`, the `p` read is not a use of the whole
    /// value, so the whole-value partial-move check is suppressed there
    /// and the enclosing projection does the precise check instead.
    projection_base: usize,
    /// Type parameters of the current fn that opted into linearity with
    /// `<T canbe linear>` [linear-generics]: `T`-typed values are treated
    /// as linear in the body, and callers may instantiate them with
    /// linear types.
    own_linear_generics: HashSet<String>,
    /// The current fn's derived-return parameter
    /// (`-> proj[from: p] T` [readonly-return]): every returned
    /// value must be derived from `p`, and the return is a *borrow*, not
    /// a move.
    own_derived_return: Option<String>,
    /// [proj-anywhere] Every source of the fn's wholesale `proj` return
    /// (`proj[from: a, b] T`): a returned value may derive from any of them.
    own_derived_sources: Vec<String>,
    /// [proj-anywhere] When the return type is a union, the *qualifier
    /// names* of the arms that carry `proj` (`Emitted` for
    /// `Emitted (proj[from: p] T) | Finished`). A returned value whose
    /// constructor builds one of these arms must be derived from the
    /// source; a value into any other arm is an ordinary move. Empty when
    /// the whole return type is the borrow (the pre-2b shape).
    own_proj_arms: Vec<String>,
    /// [proj-infer] Memo for `lends::lends_of`, keyed by declaration address.
    lends_memo: HashMap<usize, Vec<usize>>,
    /// [proj-infer] The current fn's lent parameter *names*: what a returned
    /// held view may be rooted in.
    own_lends: Vec<String>,
    /// [proj-infer] The current fn wrote `[p: proj]` entries.
    own_lends_declared: bool,
    /// [proj-anywhere] The span of a constructor call whose result is being
    /// returned into a `proj` arm: its argument is *lent* into the arm, not
    /// moved (the caller receives a borrow of it), so the call contract's
    /// consumption is suppressed for exactly that call.
    lending_ctor: Option<Span>,
    /// The implicit parameters of the fn being checked [implicit-forward]:
    /// what an inner call can have forwarded to it. Matching is by name and
    /// type, not by how they were declared, so a group spread here can fill
    /// an individually-declared `?add` there and the other way round.
    own_implicits: Vec<ImplicitParam>,
    /// [fn-rename] Renames in force, outermost first: a module-level one for
    /// the whole file, then one per `rename fn` statement, dropped when its
    /// block ends. Each takes an overload *out* of its own name and gives it
    /// the new one, so both directions read this table.
    renames: Vec<RenameBinding<'p>>,
    /// Enclosing `try` delimiters [try], innermost last: each collects the
    /// message types of the throws performed in its body.
    try_stack: Vec<TryCtx>,
    /// [fn-effects] Effect instances used inside each enclosing lambda
    /// body, innermost last: an un-annotated lambda's effect set is
    /// *inferred* from what its body performs.
    effect_uses: Vec<Vec<Ty>>,
}

/// One enclosing `try` delimiter while its body is checked [try].
struct TryCtx {
    /// `locals.len()` at entry: a throw leaves every frame above this
    /// one, which is the floor for the linear-obligation check
    /// [linear-obligation].
    entry_depth: usize,
    /// The sites that throw into this delimiter, in first-seen order:
    /// their span and message type. The outcome's `Thrown M` is the union
    /// of those types (user decision 2026-09-04), which is only known once
    /// the body is checked — so each site's wrap is filled in afterwards.
    sites: Vec<(Span, Ty)>,
}

/// One enclosing lambda during body checking [fate-lambda].
struct LambdaCtx {
    /// `locals.len()` at lambda entry: frames below this index belong to
    /// the enclosing scope, so variables in them are captures.
    boundary: usize,
    /// Captured variables recorded so far (deduplicated by id).
    captures: Vec<CaptureInfo>,
}

struct CaptureInfo {
    name: String,
    var_id: u32,
    /// The captured value is transitively mutable [fate-move-mode].
    mutable: bool,
    /// The body mutates the capture: the closure takes ownership at
    /// creation [fate-lambda].
    mutated: bool,
    /// The body *consumes* the capture: legal, but the lambda becomes
    /// `once` — callable at most once [once-fn].
    moved: bool,
}

/// A flow narrowing to apply in a branch [flow-place]: the place, the type
/// it holds there, and the physical (declared) type its storage keeps.
#[derive(Clone)]
struct Narrow {
    place: Place,
    narrowed: Ty,
    declared: Ty,
}

/// Narrowing facts derived from a condition.
#[derive(Default, Clone)]
struct CondInfo {
    /// Place narrowings that hold when the condition is true.
    then_narrows: Vec<Narrow>,
    /// Narrowings that hold when the condition is false.
    else_narrows: Vec<Narrow>,
    /// `is T name` bindings introduced in the true branch.
    bindings: Vec<Binding>,
}

impl<'p, 'r> Checker<'p, 'r> {
    fn key(&self, span: Span) -> Key {
        (self.file_idx, span)
    }

    fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.out
            .errors
            .push(FileDiagnostic::error(self.file_idx, span, msg));
    }

    /// Reports without rejecting: the program still compiles, but
    /// something the author wrote is not doing what it looks like
    /// [qual-refn-conflict].
    fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.out
            .errors
            .push(FileDiagnostic::warning(self.file_idx, span, msg));
    }

    /// An unresolved-name error carrying import suggestions: modules
    /// elsewhere in the program that declare `name`
    /// [diag-import-suggest].
    ///
    /// [mod-export] And, where the name exists but is private, the reason —
    /// appended to the message rather than offered as a help line, because it
    /// is not something the *calling* file can fix by adding an import. This is
    /// the one funnel every unresolved-name diagnostic goes through, which is
    /// why the note is attached here and not at each site.
    fn error_unresolved(&mut self, span: Span, msg: impl Into<String>, name: &str) {
        let imports = self.resolution.import_candidates(name);
        let mut msg: String = msg.into();
        // The note is about a name that is *nowhere* usable here. A name that is
        // in scope under another kind — a qualifier written where a type belongs
        // — has a different problem, and telling it about exports would send the
        // reader to the wrong file.
        if imports.is_empty() && !self.scope.declares_name(name) {
            if let Some(note) = self.resolution.export_note(name) {
                msg.push_str(&note);
            }
        }
        self.out
            .errors
            .push(FileDiagnostic::error(self.file_idx, span, msg).with_imports(imports));
    }

    // ================= module / function traversal =================

    fn check_module(&mut self, module: &'p Module) {
        // [fn-rename] Module-level renames are in force for the whole module,
        // in every file of it — order-independent, like every other
        // module-level declaration. They are validated once here (a duplicate
        // or a mismatch is reported per file, which is where the reader is).
        for decl in self.scope.renames.clone() {
            self.declare_rename(decl);
        }
        self.own_qualifiers = module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Qualifier(q) => Some(q.name.name.clone()),
                _ => None,
            })
            .collect();
        for (item_idx, item) in module.items.iter().enumerate() {
            match item {
                Item::Fn(f) => {
                    // The declaration's own name is a fn reference
                    // [fn-ref-table].
                    let key = FnKey {
                        file: self.file_idx,
                        item: item_idx,
                    };
                    self.out.fn_refs.insert(self.key(f.name.span), key);
                    // [fate-move-mode] The fn's own effective contract
                    // decides whether a parameter root is owned: the
                    // written list when present (never gains claims),
                    // else the previous round's inferred facts.
                    self.own_fn = Some(key);
                    self.own_written = f
                        .deductions
                        .as_ref()
                        .map(|l| {
                            l.iter()
                                .filter_map(|d| d.param_name().map(|n| n.name.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    self.own_contract = self.effective_contract(Some(key), f);
                    // [linear-group] Is this fn a **discharger**? For each
                    // consumed parameter whose type is a linear struct
                    // declared in this same file, `discard` becomes legal
                    // in this body (user decision 2026-09-12).
                    self.own_discharges = f
                        .params
                        .iter()
                        .filter_map(|p| {
                            let base = match &p.ty {
                                ast::Type::Named { base, .. } => &base.name.name,
                                _ => return None,
                            };
                            if !self.linear_capable(base) {
                                return None;
                            }
                            if self.scope.struct_files.get(base.as_str()).copied()
                                != Some(self.file_idx)
                            {
                                return None;
                            }
                            let consumed = self
                                .own_contract
                                .as_ref()
                                .is_some_and(|c| {
                                    c.iter().any(|d| d.param == p.name.name && !d.kept)
                                });
                            consumed.then(|| base.clone())
                        })
                        .collect();
                    // [free-send-fn] The send kind's refusal list, at the
                    // declaration where the author is deciding.
                    self.check_free_send_fn(f);
                    // [task-mint] What this body's sends are attributed to in
                    // the deadlock graph.
                    self.own_task = f.is_send.then(|| f.name.name.clone());
                    self.check_fn(f, &[], &[]);
                    self.own_task = None;
                    self.own_fn = None;
                    self.own_discharges.clear();
                    self.own_written.clear();
                    self.own_contract = None;
                }
                // [implicit-group] A group's members are signatures for
                // *parameters*, validated as declaration sites like an
                // effect's.
                Item::Params(g) => {
                    let saved = self.enter_generics(&g.generics);
                    for f in &g.fns {
                        self.reject_implicits(f, "a `params` group member");
                        if f.body.is_some() {
                            self.error(
                                f.name.span,
                                format!(
                                    "`{}.{}` is a signature, not an \
                                     implementation: a `params` group \
                                     declares what a caller must supply, and \
                                     the default comes from a matching top- \
                                     level fn",
                                    g.name.name, f.name.name
                                ),
                            );
                        }
                        let inner = self.enter_generics(&f.generics);
                        for p in &f.params {
                            self.validate_type(&p.ty);
                        }
                        if let Some(rt) = &f.return_type {
                            self.validate_type(rt);
                        }
                        self.generics = inner;
                    }
                    self.generics = saved;
                }
                Item::Handler(h) => {
                    let saved = self.enter_generics(&h.generics);
                    // [platform-handler] A host implementation of an ordinary
                    // effect: bodyless, dependency-free, non-generic.
                    self.check_platform_handler(h);
                    // [copy-implicit] A handler constructor may take implicit
                    // parameters (lifted 2026-09-11): the `use` site is where
                    // the handler's type arguments are known, so it resolves
                    // them exactly as a call resolves a fn's. The case that
                    // forced it: a generic handler that must `copy` a `T` —
                    // `?copy: (v: T) -> T` is filled at `use Cyclic([1, 2])`
                    // with the copy of `Int`, which the Kotlin backend can
                    // lower where a bare `copy` of `T` it cannot [kt-copy].
                    for p in &h.params {
                        if p.implicit && !matches!(p.ty, ast::Type::Fn { .. }) {
                            self.error(
                                p.span,
                                "an implicit parameter must have a function type: what \
                                 fills it is resolved as a function of that name"
                                    .to_string(),
                            );
                        }
                    }
                    let of_tys: Vec<Ty> =
                        h.of.iter().map(|t| self.lower_type(t)).collect();
                    // [effect-handler-multi] One face per effect, and the list
                    // is a set: naming an effect twice would make `spawn`
                    // answer the same addr twice and say nothing new.
                    for (i, (written, ty)) in h.of.iter().zip(&of_tys).enumerate() {
                        if of_tys[..i].contains(ty) {
                            self.error(
                                written.span(),
                                format!(
                                    "handler `{}` already implements `{ty}`: a handler \
                                     wears each face once",
                                    h.name.name
                                ),
                            );
                        }
                    }
                    // [effect-handler-multi] Every implemented effect must be
                    // of the **same kind**. A handler with both an `actor
                    // effect` and a plain one is the mixed handler
                    // SHAREABLE_HANDLERS.md is still designing: its plain
                    // members would run on the caller's thread while its send
                    // members run on the actor's, over one piece of state, and
                    // what protects that state is exactly the open question.
                    let kinds: Vec<(&ast::Type, bool)> = h
                        .of
                        .iter()
                        .zip(&of_tys)
                        .filter_map(|(written, ty)| {
                            let Ty::Named { name, .. } = ty.strip_quals() else {
                                return None;
                            };
                            let effect = self.symbols.effects.get(name.as_str())?;
                            Some((written, effect.is_actor))
                        })
                        .collect();
                    if let Some((_, first)) = kinds.first().copied() {
                        for (written, is_actor) in &kinds[1..] {
                            if *is_actor != first {
                                let (async_kind, sync_kind) = if *is_actor {
                                    ("this one", "the first")
                                } else {
                                    ("the first", "this one")
                                };
                                self.error(
                                    written.span(),
                                    format!(
                                        "handler `{}` implements effects of two kinds — \
                                         {async_kind} is an `actor effect` and {sync_kind} \
                                         is not: a handler is bound one way or the other \
                                         (`spawn` or `use`), so its faces must agree",
                                        h.name.name
                                    ),
                                );
                            }
                        }
                    }
                    // [effect-handler-multi] A **bodyless** handler wears one
                    // face: an `intrinsic handler`'s members are the backend's
                    // and a `platform handler`'s are the host's, and each is one
                    // implementation of one generated interface — so a second
                    // face has nobody to implement it.
                    if (h.intrinsic || h.platform) && h.of.len() > 1 {
                        let kind = if h.intrinsic {
                            "intrinsic"
                        } else {
                            "platform"
                        };
                        self.error(
                            h.of[1].span(),
                            format!(
                                "`{kind} handler {}` implements several effects: its \
                                 members are implemented outside Salvo, where one class \
                                 implements one generated interface — declare one \
                                 handler per effect",
                                h.name.name
                            ),
                        );
                    }
                    // [throw] There is no handler for throwing: the
                    // delimiter is `try`, and a handler would have to
                    // *resume* the operation, which `Nothing` forbids.
                    for (written, of_ty) in h.of.iter().zip(&of_tys) {
                        if matches!(
                            of_ty.strip_quals(),
                            Ty::Named { name, .. } if name == THROW_EFFECT
                        ) {
                            self.error(
                                written.span(),
                                format!(
                                    "`{THROW_EFFECT}` has no handlers: a throw is delimited by a \
                                     `try {{ ... }}` block, not handled"
                                ),
                            );
                        }
                        // [platform-effect] A platform effect's implementation
                        // is the host's, written in the target language and
                        // handed to the Salvo entry point. A Salvo handler for
                        // one would be a second, unreachable implementation.
                        if let Ty::Named { name, .. } = of_ty.strip_quals() {
                            if self
                                .symbols
                                .effects
                                .get(name.as_str())
                                .is_some_and(|e| e.platform)
                            {
                                self.error(
                                    written.span(),
                                    format!(
                                        "`{name}` is a platform effect: the host implements \
                                         it in the target language and supplies it to the \
                                         entry point, so it has no Salvo handler — declare \
                                         an ordinary `effect` if you mean to handle it here"
                                    ),
                                );
                            }
                        }
                    }
                    for p in &h.params {
                        // [effect-handler-deps] Dependencies are declared in
                        // the handler's own effect list now (user decision
                        // 2026-09-14), so a constructor parameter is always
                        // ordinary data — and an effect-typed one is the
                        // plain [effect-not-data] error, with no exception
                        // left anywhere in the language.
                        self.validate_type(&p.ty);
                    }
                    // [effect-handler-multi] Conformance, face by face: every
                    // member of every implemented effect needs an
                    // implementation here, and a member that implements two
                    // faces at once has to be *one* signature — which is what
                    // "legal when overloading distinguishes them, or when one
                    // method implements both" means when it is checked.
                    self.check_handler_conformance(h);
                    let handler_deps = self.handler_dep_effects(h);
                    let saved_deps =
                        std::mem::replace(&mut self.handler_deps, handler_deps);
                    // [actor-spawn-effect] [actor-replyto] The handler its
                    // members belong to: `spawn` in its dependency list is a
                    // capability they inherit, and `replyto` resolves its
                    // target member against this declaration.
                    let spawns = h
                        .effects
                        .iter()
                        .flatten()
                        .any(|e| matches!(e, EffectRef::Spawn(_)));
                    let saved_spawns = std::mem::replace(&mut self.handler_spawns, spawns);
                    // [waitfor-effect] And `waitfor` the same way: a handler
                    // that declares it may block from any member — which is
                    // why its spawn site must place it on a dedicated thread.
                    let waits = h
                        .effects
                        .iter()
                        .flatten()
                        .any(|e| matches!(e, EffectRef::WaitFor(_)));
                    let saved_waits = std::mem::replace(&mut self.handler_waits, waits);
                    let saved_own_handler = std::mem::replace(&mut self.own_handler, Some(h));
                    let saved_of = std::mem::replace(&mut self.handler_ofs, of_tys.clone());
                    // [mixed-handler] Plain faces + `send fn` members: the
                    // mixed handler (SH-1, user decision 2026-09-19). The
                    // send members and the state form the servant; the sync
                    // members form the façade.
                    let mixed = self.handler_is_mixed(h);
                    // [actor-mailbox] The actor settings slot, before the
                    // state fields: it is the one part of a handler that is
                    // computed *before* any state exists.
                    self.check_mailbox_slot(h, &of_tys);
                    for field in &h.state {
                        self.validate_type(&field.ty);
                        self.check_proj_field(&h.name.name, field);
                        self.check_linear_field(&h.name.name, field);
                        // A state field's initializer is checked against its
                        // declared type, exactly like a struct field's
                        // default [effect-handler]: it runs in `new()` with
                        // no locals in scope. (Until 2026-09-04 it was not
                        // checked at all, so `held: Mut List<Int> = "no"`
                        // was accepted and the emitters never saw the
                        // expression's types.)
                        if let Some(default) = &field.default {
                            let expected = self.lower_type(&field.ty);
                            self.locals.push(HashMap::new());
                            let got = self.check_expr(default, Some(&expected));
                            self.locals.pop();
                            if !is_subtype(&got, &expected) {
                                self.error(
                                    default.span(),
                                    format!("expected `{expected}`, found `{got}`"),
                                );
                            }
                        }
                    }
                    for f in &h.fns {
                        self.reject_member_effects(f, "handler member functions");
                        // [actor-send-fn] A handler's `send fn` answers
                        // nothing, exactly as the effect's declaration does.
                        self.check_send_member(f);
                        // [mixed-handler] A **mixed** handler's send member
                        // is the servant protocol, and it carries the free
                        // send fn's obligations, because no effect
                        // declaration mirrors it. (An *actor* handler's
                        // local send members stay as they are: private
                        // helpers under the face's regime.)
                        if f.is_send && mixed {
                            self.check_local_send_member(h, f);
                        }
                        // [mixed-handler] A sync member of a mixed handler is
                        // the façade: it runs on the caller's thread over the
                        // façade value, so it sees the constructor parameters
                        // and its own arguments — not the state (confined to
                        // the servant), not the handler's dependencies (they
                        // live in the servant) — and its `waitfor` needs no
                        // capability (SH-5(d)'s down payment: the façade does
                        // not run on the spawned thread, so there is no
                        // placement to check).
                        let facade = mixed && !f.is_send;
                        let saved_facade =
                            std::mem::replace(&mut self.facade_handler, facade.then_some(h));
                        let saved_confined = std::mem::replace(
                            &mut self.confined_state,
                            if facade {
                                h.state.iter().map(|s| s.name.name.clone()).collect()
                            } else {
                                Vec::new()
                            },
                        );
                        let saved_facade_deps = if facade {
                            std::mem::take(&mut self.handler_deps)
                        } else {
                            Vec::new()
                        };
                        let facade_waits = self.handler_waits || facade;
                        let saved_facade_waits =
                            std::mem::replace(&mut self.handler_waits, facade_waits);
                        let member_state: &[ast::FieldDecl] =
                            if facade { &[] } else { &h.state };
                        // [linear-group] [linear-discard] A handler member
                        // implementing a **consuming** effect member is a
                        // discharge context: discharger status attaches to
                        // the *member declaration*, so every handler's body
                        // for it may terminate the obligation with `discard`
                        // — wherever the handler lives, including a test
                        // double in another module (user decision
                        // 2026-09-14, entailed by stream ops being members).
                        let context = self.member_discharge_context(h, f);
                        let saved_discharges =
                            std::mem::replace(&mut self.own_discharges, context);
                        self.check_fn(f, &h.params, member_state);
                        self.own_discharges = saved_discharges;
                        self.facade_handler = saved_facade;
                        self.confined_state = saved_confined;
                        if facade {
                            self.handler_deps = saved_facade_deps;
                        }
                        self.handler_waits = saved_facade_waits;
                    }
                    self.handler_deps = saved_deps;
                    self.handler_spawns = saved_spawns;
                    self.handler_waits = saved_waits;
                    self.own_handler = saved_own_handler;
                    self.handler_ofs = saved_of;
                    self.generics = saved;
                }
                Item::Effect(e) => {
                    self.check_platform_effect(e);
                    self.check_effect_member_signatures(e);
                    // [actor-effect-kind] The kind's own rules: what an
                    // `actor effect` may declare, and that `send fn` needs one.
                    self.check_effect_kind(e);
                    let saved = self.enter_generics(&e.generics);
                    for f in &e.fns {
                        self.reject_member_effects(f, "effect member functions");
                        self.require_explicit_member(f);
                        self.require_full_clause(f, "an effect member");
                        // Member signatures are declaration sites like any
                        // other, but no body is checked, so validate them
                        // here [name-resolve].
                        let inner = self.enter_generics(&f.generics);
                        for p in &f.params {
                            self.validate_type(&p.ty);
                        }
                        // [implicit-param] An effect member is an ordinary
                        // signature, so it may declare implicit parameters.
                        // They belong to the member's signature — the
                        // interface method takes them, every handler's
                        // implementation takes them, and the call site fills
                        // them — so the expansion is recorded here, keyed by
                        // the member's name (a member has no `FnKey`).
                        let implicits = self.collect_implicits(f);
                        if !implicits.is_empty() {
                            self.out
                                .implicit_members
                                .insert(self.key(f.name.span), implicits);
                        }
                        if let Some(rt) = &f.return_type {
                            self.validate_type(rt);
                        }
                        self.generics = inner;
                    }
                    self.generics = saved;
                }
                Item::Qualifier(q) => {
                    let saved = self.enter_generics(&q.generics);
                    self.check_qualifier_decl(q);
                    for f in &q.fns {
                        self.check_fn(f, &[], &[]);
                    }
                    self.generics = saved;
                }
                Item::Struct(s) => {
                    let saved = self.enter_generics(&s.generics);
                    self.validate_auto_quals(&s.auto_qualifiers);
                    // [col-hashed-ordered] `canbe hashed` / `canbe ordered`
                    // are checked *here*, where the mistake is: the error
                    // names the field that is not hashable or orderable
                    // rather than surfacing at some distant `Set<Point>`.
                    self.check_key_optins(s);
                    self.check_obligations(s);
                    for field in &s.fields {
                        self.validate_type(&field.ty);
                        // [proj-field] Any struct may hold a borrow through a
                        // `proj` field; it is then a *view*, tied to whatever
                        // its literal stored there [proj-infer].
                        self.check_proj_field(&s.name.name, field);
                        self.check_linear_field(&s.name.name, field);
                        if let Some(default) = &field.default {
                            let expected = self.lower_type(&field.ty);
                            self.locals.push(HashMap::new());
                            self.check_expr(default, Some(&expected));
                            self.locals.pop();
                        }
                    }
                    self.generics = saved;
                }
                Item::Type(t) => {
                    let saved = self.enter_generics(&t.generics);
                    self.validate_auto_quals(&t.auto_qualifiers);
                    if let Some(alias) = &t.alias {
                        self.validate_type(alias);
                    }
                    // [linear-group] The opaque form of the legal-death
                    // rule, checked where the modifier is written.
                    self.check_linear_opaque(t);
                    self.generics = saved;
                }
                _ => {}
            }
        }
    }

    /// [group-obligation] `struct X<G> : Group<Args> …` — every obligation
    /// names a visible `params` group with the right arity, and every
    /// member of that group must be satisfied by a visible fn overload with
    /// `Self` bound to this declaration [group-self]. Checked *here*, at
    /// the struct, so a misspelled or missing member surfaces where the
    /// promise is written instead of as a puzzling failure at a use site.
    ///
    /// Satisfaction is by **types, positionally** — parameter types and the
    /// return type equal up to a bijective renaming of type variables. The
    /// member's parameter *names* belong to the group and are not required
    /// of the implementation (unlike [qual-refn-match], which names an
    /// overload someone already declared). Effects are deliberately not
    /// compared: the group declares none and each implementation declares
    /// its own.
    fn check_obligations(&mut self, s: &'p ast::StructDecl) {
        // [linear-group] A `linear struct` — and a conditional container
        // (`canbe linear` reaching a field [linear-generics]) — must have a
        // legal death: at least one fn in the *same file* consuming a
        // parameter of this type (user decision 2026-09-12 — the discharge
        // set is same-file consumption, not a designated member). Round
        // two, so inferred contracts count.
        let conditional = s.generic_canbe.iter().any(|(id, q)| {
            q.name.name == "linear"
                && s.fields.iter().any(|f| type_mentions_generic(&f.ty, &id.name))
        });
        if (s.linear || conditional) && self.inferred.is_some() {
            let set = self.discharge_set(&s.name.name);
            if set.is_empty() {
                let what = if s.linear {
                    format!("linear struct `{}`", s.name.name)
                } else {
                    format!(
                        "`{}` is conditionally linear (`canbe linear` reaches a field)",
                        s.name.name
                    )
                };
                self.error(
                    s.name.span,
                    format!(
                        "{what} has no discharger: nothing in this file consumes a \
                         `{0}` — no fn, and no member of an effect declared here \
                         — so the obligation has no legal death; declare one \
                         (e.g. `fn close(x: {0}) -> None => !x {{ discard(x) }}`)",
                        s.name.name
                    ),
                );
            }
        }
        let scope = self.scope;
        for (i, ob) in s.obligations.iter().enumerate() {
            // [linear-group] The pre-2026-09-12 spelling, caught before the
            // unknown-group error would puzzle: linearity is a declaration
            // modifier now.
            if ob.name.name == "Linear" {
                self.error(
                    ob.span,
                    "linearity is declared with the `linear struct` modifier, \
                     not as an obligation: write `linear struct …` and supply a \
                     same-file fn that consumes the value",
                );
                continue;
            }
            // The same group twice is a mistake, not an emphasis.
            if s.obligations[..i]
                .iter()
                .any(|p| p.name.name == ob.name.name)
            {
                self.error(
                    ob.span,
                    format!("obligation `{}` is declared more than once", ob.name.name),
                );
                continue;
            }
            // [lsp-definition] The group name in an obligation clause
            // (`: Linear<self>`) points at the group's declaration, so hover
            // and go-to-definition reach it like any other reference.
            self.record_def_ref(ob.name.span, &ob.name.name);
            let Some(group) = scope.param_groups.get(ob.name.name.as_str()).copied() else {
                // A name that exists as something else is a position
                // mistake, not a missing declaration: say which.
                let hint = if self.type_name_exists(&ob.name.name) {
                    format!(
                        " (`{}` is a type; an obligation names a `params` group)",
                        ob.name.name
                    )
                } else if self.qual_name_exists(&ob.name.name) {
                    format!(
                        " (`{}` is a qualifier — `canbe` grants qualifiers, `:` \
                         declares obligations)",
                        ob.name.name
                    )
                } else {
                    String::new()
                };
                let name = ob.name.name.clone();
                self.error_unresolved(
                    ob.span,
                    format!("unknown `params` group `{name}`{hint}"),
                    &name,
                );
                continue;
            };
            // [group-self] `self` in an obligation's argument list is the
            // declaring type. It is written where a type argument goes rather
            // than inside the group, which is what keeps the group ordinary:
            // its members mention only their own parameters, so the very same
            // group also spreads as `?Group<...>` implicits [implicit-group].
            let self_ty = Ty::Named {
                name: s.name.name.clone(),
                args: s.generics.iter().map(|g| Ty::Var(g.name.clone())).collect(),
            };
            for a in &ob.args {
                if !is_self_ref(a) {
                    self.validate_type(a);
                }
            }
            let args: Vec<Ty> = ob
                .args
                .iter()
                .map(|a| {
                    if is_self_ref(a) {
                        self_ty.clone()
                    } else {
                        self.lower_type(a)
                    }
                })
                .collect();
            if args.len() != group.generics.len() {
                self.error(
                    ob.span,
                    format!(
                        "`{}` takes {} type argument(s), found {}",
                        group.name.name,
                        group.generics.len(),
                        args.len()
                    ),
                );
                continue;
            }
            let subst: HashMap<String, Ty> = group
                .generics
                .iter()
                .map(|p| p.name.clone())
                .zip(args)
                .collect();
            let bound: HashSet<String> = group.generics.iter().map(|p| p.name.clone()).collect();
            for member in &group.fns {
                let outer = self.enter_generics(&group.generics);
                let member_ty = self.member_fn_ty(member);
                self.generics = outer;
                let expected = substitute_vars(&member_ty, &subst, &bound);
                let mut found = false;
                if let Some(entries) = scope.fns.get(member.name.name.as_str()) {
                    for e in entries {
                        let inner = self.enter_generics(&e.decl.generics);
                        let candidate = self.member_fn_ty(e.decl);
                        self.generics = inner;
                        let mut fwd = HashMap::new();
                        let mut rev = HashMap::new();
                        if tys_match_renamed(&expected, &candidate, &mut fwd, &mut rev) {
                            // [yield-proj] `proj` strips from types, so the
                            // shapes match either way; the *borrowness* has to
                            // agree separately. An obligation at `proj T`
                            // promises borrowed elements, which the member's
                            // written return must deliver — and vice versa:
                            // a `next` emitting `proj` under a plain
                            // `Yield<self, T>` would hand a borrow to callers
                            // expecting to own it.
                            let ob_proj = ob.args.iter().any(|a| first_proj_span(a).is_some());
                            let member_proj = e
                                .decl
                                .return_type
                                .as_ref()
                                .is_some_and(|t| !proj_arm_indices(t).is_empty());
                            if ob_proj != member_proj {
                                self.error(
                                    ob.span,
                                    if ob_proj {
                                        format!(
                                            "`{}` declares `: {}<self, proj …>`, but its `{}` \
                                             returns an owned element: write \
                                             `Emitted (proj[from: p] T) | Finished`, or drop \
                                             the `proj` from the obligation",
                                            s.name.name, ob.name.name, member.name.name
                                        )
                                    } else {
                                        format!(
                                            "`{}`'s `{}` returns a borrowed element \
                                             (`proj`), so its obligation must say so: \
                                             `: {}<self, proj T>`",
                                            s.name.name, member.name.name, ob.name.name
                                        )
                                    },
                                );
                            }
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    let Ty::Fn { params, ret, .. } = &expected else {
                        continue;
                    };
                    let shown: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                    self.error(
                        ob.span,
                        format!(
                            "`{}` declares `: {}` but no visible `{}` matches \
                             `fn {}({}) -> {}`",
                            s.name.name,
                            ob.name.name,
                            member.name.name,
                            member.name.name,
                            shown.join(", "),
                            ret
                        ),
                    );
                }
            }
        }
    }

    /// [canbe-optin] A `canbe` clause names one of the compiler's
    /// permission/obligation qualifiers; user qualifiers are applied in
    /// types, not granted by opt-in.
    ///
    /// `once` joined the list 2026-09-07 (user decision) so a hand-written
    /// **pass** — a type with a `next` [iter-protocol] — can say that
    /// driving it uses it up. Opting in is the author's call for the same
    /// reason `Linear` is declared rather than applied [linear-group]: an
    /// obligation should not attach to someone's type on the strength of a
    /// method name.
    fn validate_auto_quals(&mut self, quals: &[TypeRef]) {
        for q in quals {
            if matches!(q.name.name.as_str(), "Mut" | "once" | "hashed" | "ordered") {
                continue;
            }
            // [linear-group] `canbe` grants a *qualifier*; linearity is an
            // **obligation**, and declaring it means supplying the `close`
            // that discharges it. The two were spelled alike until R4; now
            // `canbe` means only "may be qualified thus".
            if q.name.name == "linear" {
                self.error(
                    q.span,
                    "linearity is declared as an obligation, not granted with \
                     `canbe`: write `linear struct` and supply a same-file discharger"
                        .to_string(),
                );
                continue;
            }
            self.error(
                q.span,
                format!(
                    "only `Mut`, `once`, `hashed` and `ordered` can be opted \
                     into with `canbe` (found `{}`)",
                    q.name.name
                ),
            );
        }
    }

    /// [decl-explicit] Nothing the compiler cannot see may be inferred: a
    /// fn with no body states its effects, deductions, and return type
    /// explicitly (user decision 2026-09-03). Inference from an absent
    /// body is a *guess*, and the most permissive one — which is how
    /// std's `add` came to keep an element the list had taken ownership
    /// of. Effect members are the same case with one exception: they may
    /// not declare effects at all [effect-member-no-effects].
    fn require_explicit_decl(&mut self, f: &FnDecl) {
        if f.body.is_some() {
            return;
        }
        if !f.intrinsic {
            return;
        }
        self.require_explicit(f, "intrinsic fn", true);
    }

    /// [effect-member-unique] [effect-member-overload] A member name may recur
    /// **within** its effect only as an *overload*: the parameter lists must
    /// differ, exactly as for top-level fns [fn-overload] (§5.10.2 sub-question
    /// A, user decision 2026-09-14 — phase 4's `Fs` declares `close(InStream)`
    /// and `close(OutStream)`, and `position` twice). Two members with the same
    /// name *and* the same parameter types are the old duplicate error.
    ///
    /// Across effects the name may recur freely ([effect-member-overload], user
    /// decision 2026-09-14, lifting the 2026-09-05 program-wide ban): a call
    /// disambiguates by which effect has a handler in scope, or explicitly
    /// with `member@Effect(…)` [effect-at] — the syntax whose absence was the
    /// original ban's reason.
    ///
    /// Signatures are compared as *lowered* types rather than as written text,
    /// so two spellings of one type (an alias, a differently-written generic
    /// argument) are the duplicate they are. Reported at the **second**
    /// declaration, so the diagnostic is deterministic and fires exactly once
    /// per collision.
    fn check_effect_member_signatures(&mut self, e: &'p EffectDecl) {
        // Lowered fixed-parameter lists, in declaration order.
        let mut seen: Vec<(&str, Vec<Ty>)> = Vec::new();
        for f in &e.fns {
            self.check_send_member(f);
            let inner = self.enter_generics(&f.generics);
            let params: Vec<Ty> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| self.lower_type(&p.ty))
                .collect();
            self.generics = inner;
            if seen
                .iter()
                .any(|(name, ps)| *name == f.name.name.as_str() && *ps == params)
            {
                self.error(
                    f.name.span,
                    format!(
                        "effect `{}` already declares a member named `{}` with these \
                         parameter types: members overload like functions, so two of \
                         them must differ in what they take",
                        e.name.name, f.name.name
                    ),
                );
            }
            seen.push((f.name.name.as_str(), params));
        }
    }

    /// [actor-send-fn] What a `send fn` may declare. A send member is a
    /// *message*: sending it enqueues an invocation and returns immediately,
    /// so there is no value to answer with — a reply travels as a `Reply<T>`
    /// parameter the sender mints with `replyto`. A written return type is
    /// therefore an error naming that shape, rather than a silently ignored
    /// annotation. (The later call-member sugar goes the other way: a `-> T`
    /// member desugars *into* a send member with a trailing token, so the
    /// two spellings must not both mean something here.)
    /// [actor-effect-kind] The effect-level kind rules (user decision
    /// 2026-09-15, EU-5). The kind is declared, not diagnosed: an author
    /// choosing between `effect` and `actor effect` is choosing whether the
    /// protocol crosses threads, and everything that cannot cross is refused
    /// **here**, where the choice is being made, rather than at some later
    /// binding.
    ///
    /// Two directions:
    /// * `send fn` needs an `actor effect` — a message has nowhere to go in a
    ///   synchronous protocol.
    /// * inside an `actor effect`, a member may not **keep** a parameter, take
    ///   a `Mut` one, return a `proj` view, or carry a **non-sendable** payload
    ///   [actor-sendable]. Each is a borrow or a share that a seam cannot
    ///   carry.
    fn check_effect_kind(&mut self, e: &'p EffectDecl) {
        let saved = self.enter_generics(&e.generics);
        for f in &e.fns {
            if f.is_send && !e.is_actor {
                self.error(
                    f.name.span,
                    format!(
                        "`send fn {}` needs an `actor effect`: a message is \
                         enqueued on an actor, which a synchronous effect \
                         never has — declare `actor effect {}` if this \
                         protocol is meant to be spawned",
                        f.name.name, e.name.name
                    ),
                );
            }
            if e.is_actor {
                self.check_actor_member(e, f);
            }
        }
        self.generics = saved;
    }

    /// [actor-effect-kind] [actor-sendable] One member of an `actor effect`,
    /// against the refusal list. The diagnostics name the *law* rather than the
    /// symptom ("a `Mut` parameter in an `actor effect`"), because the remedy
    /// is a design decision — send a copy, or make the protocol synchronous.
    fn check_actor_member(&mut self, e: &'p EffectDecl, f: &'p FnDecl) {
        let inner = self.enter_generics(&f.generics);
        // A kept parameter is a borrow that outlives the call, which a
        // message cannot carry: the payload crosses the seam and the sender
        // gives it up.
        for d in f.deductions.iter().flatten() {
            // Every entry but `Moved` keeps the parameter: a bare `=> p`, an
            // exhaustive or subtractive qualifier list, or a projection.
            if matches!(d.kind, ast::DeductionKind::Moved) {
                continue;
            }
            let Some(name) = d.param_name() else { continue };
            self.error(
                d.span,
                format!(
                    "member `{}` of `actor effect {}` cannot keep `{}`: a \
                     message payload crosses to another actor, so it is \
                     always consumed — write `=> !{}`, or make `{}` a plain \
                     effect",
                    f.name.name, e.name.name, name.name, name.name, e.name.name
                ),
            );
        }
        for p in f.params.iter().filter(|p| !p.implicit) {
            // A `Mut` parameter is an exclusive borrow of the caller's value.
            if let ast::Type::Named { qualifiers, .. } = &p.ty {
                if let Some(q) = qualifiers.iter().find(|q| q.name.name == "Mut") {
                    self.error(
                        q.span,
                        format!(
                            "member `{}` of `actor effect {}` cannot take a \
                             `Mut` parameter: mutating a value across a \
                             actor boundary would share it, and an actor \
                             owns its state alone",
                            f.name.name, e.name.name
                        ),
                    );
                }
            }
            // [actor-sendable] C-4(a)'s structural rule, at the declaration.
            let ty = self.lower_type(&p.ty);
            if let Some(why) = self.unsendable_reason(&ty) {
                self.error(
                    p.ty.span(),
                    format!(
                        "member `{}` of `actor effect {}` cannot carry \
                         `{ty}`: {why}, and everything crossing to another \
                         actor must be sendable",
                        f.name.name, e.name.name
                    ),
                );
            }
        }
        // A `proj` return is a borrow of something the actor owns.
        if let Some(rt) = &f.return_type {
            if let Some(span) = first_proj_span(rt) {
                self.error(
                    span,
                    format!(
                        "member `{}` of `actor effect {}` cannot return a \
                         `proj` view: it would borrow state the actor owns \
                         and keeps mutating",
                        f.name.name, e.name.name
                    ),
                );
            }
        }
        self.generics = inner;
    }

    /// [actor-sendable] Why a value of this type may not cross a seam, or
    /// `None` when it may (user decision 2026-09-15, C-4(a) as the structural
    /// rule). Two kinds of contents are refused, both because the *other* side
    /// could never own what it received:
    ///
    /// * a **function-typed** field or component — a callback is shared, and
    ///   the Rust backend holds one in an `Rc`, which is not `Send`
    ///   [rs-fn-field];
    /// * a **`proj` view** — a borrow of a value the sender still owns.
    ///
    /// `Arc`-where-sent inference is the recorded growth point (C-4(c)) for
    /// when sent closures become real; until then the answer is a diagnostic.
    fn unsendable_reason(&self, ty: &Ty) -> Option<&'static str> {
        if self.ty_holds_fn(ty, 0) {
            return Some("it holds a function value, which is shared rather than owned");
        }
        if self.ty_holds_proj(ty) {
            return Some("it holds a `proj` view, which borrows the sender's value");
        }
        None
    }

    /// [actor-sendable] Whether a (lowered) type is, or transitively holds, a
    /// function value. Structural like `ty_holds_proj`, with the same depth
    /// guard against a recursive struct.
    fn ty_holds_fn(&self, ty: &Ty, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        match ty.strip_quals() {
            Ty::Fn { .. } => true,
            Ty::Union(arms) | Ty::Tuple(arms) => {
                arms.iter().any(|a| self.ty_holds_fn(a, depth + 1))
            }
            Ty::Array(elem) => self.ty_holds_fn(elem, depth + 1),
            Ty::Named { name, args } => {
                if args.iter().any(|a| self.ty_holds_fn(a, depth + 1)) {
                    return true;
                }
                let Some(decl) = self.scope.structs.get(name.as_str()) else {
                    return false;
                };
                // A field's *written* type is enough: a fn type is syntactic
                // (`(T) -> U`), so no lowering is needed to recognise one, and
                // a named field type recurses through the same map.
                let fields: Vec<&ast::Type> = decl.fields.iter().map(|f| &f.ty).collect();
                fields.iter().any(|t| self.ast_type_holds_fn(t, depth + 1))
            }
            _ => false,
        }
    }

    /// [actor-sendable] The written-type half of `ty_holds_fn`: a struct
    /// field's declared type, without lowering it (a fn type is syntactic, and
    /// lowering here would need the declaring scope's generics).
    fn ast_type_holds_fn(&self, ty: &ast::Type, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        match ty {
            ast::Type::Fn { .. } => true,
            ast::Type::Union { arms, .. } => arms.iter().any(|a| self.ast_type_holds_fn(a, depth + 1)),
            ast::Type::Tuple { elems, .. } => {
                elems.iter().any(|e| self.ast_type_holds_fn(e, depth + 1))
            }
            ast::Type::Array { elem, .. } => self.ast_type_holds_fn(elem, depth + 1),
            ast::Type::Nullable { inner, .. } => self.ast_type_holds_fn(inner, depth + 1),
            ast::Type::QualifiedGroup { base, .. } => self.ast_type_holds_fn(base, depth + 1),
            ast::Type::Named { base, .. } => {
                if base.args.iter().any(|a| self.ast_type_holds_fn(a, depth + 1)) {
                    return true;
                }
                match self.scope.structs.get(base.name.name.as_str()) {
                    Some(decl) => decl
                        .fields
                        .iter()
                        .any(|f| self.ast_type_holds_fn(&f.ty, depth + 1)),
                    None => false,
                }
            }
        }
    }

    /// [free-send-fn] A **free `send fn`** — the send kind extended to a
    /// function (user decision 2026-09-17, FC-1(a)). It runs by being
    /// *scheduled*, never called: a `replyto` targets it [task-mint], its
    /// answer arrives as its trailing parameter, and it answers nothing.
    ///
    /// Every rule here is inherited rather than invented — it is
    /// [actor-effect-kind]'s refusal list, applied to a declaration that has
    /// no effect to belong to: no return type, no kept parameter (the payload
    /// crosses a thread boundary, so it is always consumed), no `Mut`
    /// parameter, no `proj` return, and every parameter sendable
    /// [actor-sendable]. The diagnostics name the law, because the remedy is a
    /// design decision.
    ///
    /// **First-pass cut**: no effects but `[waitfor]`. A task body runs
    /// detached from the scope that minted it, so there is nothing there to
    /// supply a handler — capturing the minting scope's handlers would send
    /// values the sender still owns, which [actor-sendable] exists to refuse.
    /// An `Addr` capture is the way to reach an actor, and it needs no effect
    /// declaration [actor-use-addr]. Lifting this for *actor-backed* effects
    /// (whose provider is an addr stub, and so sendable) is the recorded
    /// growth point.
    fn check_free_send_fn(&mut self, f: &'p FnDecl) {
        if !f.is_send {
            return;
        }
        self.check_send_member(f);
        // A generic target would need its type arguments at the mint, which
        // carries only captures; and a generic parameter tells [actor-sendable]
        // nothing, so the refusal list could not be checked here either. The
        // first pass refuses it rather than checking it at every mint.
        if let Some(g) = f.generics.first() {
            self.error(
                f.name.span,
                format!(
                    "`send fn {}` cannot be generic: a mint carries captures and no type \
                     arguments, so `{}` would have nothing to be chosen by",
                    f.name.name, g.name
                ),
            );
        }
        let inner = self.enter_generics(&f.generics);
        for d in f.deductions.iter().flatten() {
            if matches!(d.kind, ast::DeductionKind::Moved) {
                continue;
            }
            let Some(name) = d.param_name() else { continue };
            self.error(
                d.span,
                format!(
                    "`send fn {}` cannot keep `{}`: a scheduled body outlives the frame that \
                     minted it, so what it is given is always consumed — write `=> !{}`, or \
                     make `{}` an ordinary `fn`",
                    f.name.name, name.name, name.name, f.name.name
                ),
            );
        }
        for p in f.params.iter().filter(|p| !p.implicit) {
            if let ast::Type::Named { qualifiers, .. } = &p.ty {
                if let Some(q) = qualifiers.iter().find(|q| q.name.name == "Mut") {
                    self.error(
                        q.span,
                        format!(
                            "`send fn {}` cannot take a `Mut` parameter: mutating a value \
                             from a scheduled body would share what the minting frame still \
                             owns",
                            f.name.name
                        ),
                    );
                }
            }
            let ty = self.lower_type(&p.ty);
            if let Some(why) = self.unsendable_reason(&ty) {
                self.error(
                    p.ty.span(),
                    format!(
                        "`send fn {}` cannot take `{ty}`: {why}, and everything a scheduled \
                         body is given crosses a thread boundary",
                        f.name.name
                    ),
                );
            }
        }
        if let Some(rt) = &f.return_type {
            if let Some(span) = first_proj_span(rt) {
                self.error(
                    span,
                    format!(
                        "`send fn {}` cannot return a `proj` view: it would borrow a value \
                         the minting frame owns",
                        f.name.name
                    ),
                );
            }
        }
        // The first-pass cut: `[waitfor]` is a capability the placement check
        // validates [waitfor-dedicated]; anything else names an instance that
        // would have to be supplied, and a task has no scope to supply it from.
        for eff in f.effects.iter().flatten() {
            let span = match eff {
                EffectRef::WaitFor(_) => continue,
                EffectRef::Use(sp) | EffectRef::Spawn(sp) => *sp,
                EffectRef::Effect(r) => r.span,
            };
            self.error(
                span,
                format!(
                    "`send fn {}` cannot declare `{eff}`: a scheduled body runs detached from \
                     the frame that minted it, so there is no scope to supply a handler from. \
                     Take an `{ADDR_TYPE}` as a capture and send to it — reaching an actor \
                     needs no effect declaration — or do the effectful work in the function \
                     that mints",
                    f.name.name
                ),
            );
        }
        self.generics = inner;
    }

    fn check_send_member(&mut self, f: &'p FnDecl) {
        if !f.is_send {
            return;
        }
        if let Some(ret) = &f.return_type {
            self.error(
                ret.span(),
                format!(
                    "`send fn {}` cannot declare a return type: sending a member \
                     enqueues it and answers nothing, so a reply travels as a \
                     parameter — `send fn {}(…, out: Reply<T>)`, which the sender \
                     mints with `replyto`",
                    f.name.name, f.name.name
                ),
            );
        }
    }

    /// [platform-handler] The restrictions a `platform handler` carries, all
    /// of them consequences of the implementation being a class the *build*
    /// supplies rather than Salvo code (FS-1 resolved as O-M2, user decision
    /// 2026-09-14).
    ///
    /// It is the mirror image of a `platform effect`: there the *effect* is
    /// the host's and the instance arrives at the entry point, here the
    /// effect is an ordinary Salvo one and only this *handler* is the host's,
    /// so it registers with `use` like any other [platform-tree].
    fn check_platform_handler(&mut self, h: &'p ast::HandlerDecl) {
        if !h.platform {
            return;
        }
        // Bodyless: the members are the host's, in the target language.
        // Reported at the first offending declaration, since a body is a
        // whole-declaration mistake either way.
        if let Some(span) = h
            .fns
            .first()
            .map(|f| f.name.span)
            .or_else(|| h.state.first().map(|f| f.name.span))
        {
            self.error(
                span,
                format!(
                    "`platform handler {}` has no body in Salvo: the host \
                     implements its members in the target language, in the \
                     `platform/` companion of this module — run `salvo platform \
                     generate` to write the skeleton, or drop `platform` to \
                     handle the effect here",
                    h.name.name
                ),
            );
        }
        // Dependencies would have to be supplied *to host code*, which
        // cannot perform a Salvo effect: the host reaches the outside world
        // directly, which is why it is host code.
        if h.effects.as_ref().is_some_and(|e| !e.is_empty()) {
            self.error(
                h.name.span,
                format!(
                    "`platform handler {}` may not declare effect dependencies: \
                     its members are host code, which performs no Salvo effect — \
                     write a Salvo handler that depends on this one's effect if \
                     you need one in between",
                    h.name.name
                ),
            );
        }
        // Generic-free for the reason a platform effect is: the host writes
        // one concrete class, and there is no instance per type argument to
        // construct at a `use` site [backend-never-wrong].
        if !h.generics.is_empty() {
            self.error(
                h.name.span,
                format!(
                    "`platform handler {}` may not be generic: the host \
                     implements one concrete class, and an instance per type \
                     argument is not expressible on every backend",
                    h.name.name
                ),
            );
        }
    }

    /// [platform-effect] The restrictions a `platform effect` carries
    /// beyond an ordinary one, both because the *host* implements the
    /// generated interface rather than Salvo code (user decisions
    /// 2026-09-05).
    ///
    /// Neither the effect nor its members may be generic. A generic member
    /// is already a loud codegen error on the Rust backend
    /// ([effect-member-generics]), and a generic *effect* would need the
    /// host to implement one interface per instantiation — Kotlin's facets
    /// exist for exactly that ([kt-effect-fusion]) and Rust has no
    /// equivalent, so an instance the compiler cannot pin is refused here
    /// rather than at codegen [backend-never-wrong].
    fn check_platform_effect(&mut self, e: &'p EffectDecl) {
        if !e.platform {
            return;
        }
        if !e.generics.is_empty() {
            self.error(
                e.name.span,
                format!(
                    "platform effect `{}` may not be generic: the host implements \
                     one interface, and an instance per type argument is not \
                     expressible on every backend",
                    e.name.name
                ),
            );
        }
        for f in &e.fns {
            if !f.generics.is_empty() {
                self.error(
                    f.name.span,
                    format!(
                        "member `{}` of platform effect `{}` may not be generic: \
                         the host implements a concrete signature",
                        f.name.name, e.name.name
                    ),
                );
            }
        }
    }

    /// [decl-explicit] The effect-member form: return type and deductions
    /// are required, effects are forbidden [effect-member-no-effects].
    fn require_explicit_member(&mut self, f: &FnDecl) {
        self.require_explicit(f, "effect member", false);
    }

    fn require_explicit(&mut self, f: &FnDecl, kind: &str, want_effects: bool) {
        let name = &f.name.name;
        // [actor-send-fn] A `send fn` answers nothing — the reply, if there
        // is one, is a `Reply<T>` parameter — so there is no return type to
        // require, and writing one is its own error (`check_send_member`).
        if f.return_type.is_none() && !f.is_send {
            self.error(
                f.name.span,
                format!(
                    "{kind} `{name}` must declare its return type (write \
                     `-> None` when it returns no value): with no body to \
                     infer from, the signature is the whole contract"
                ),
            );
        }
        if want_effects && f.effects.is_none() {
            self.error(
                f.name.span,
                format!(
                    "{kind} `{name}` must declare its effect list (write `[]` \
                     when it is pure): with no body to infer from, the \
                     signature is the whole contract"
                ),
            );
        }
        // The deduction clause: per parameter, in `require_full_clause`
        // [deduce-syntax] — a declaration with nothing to deduce (only Copy
        // scalars, variadics, implicits) needs no clause at all.
    }

    /// Checks one function body. `extra_params`/`state` provide handler
    /// constructor parameters and state fields as in-scope variables.
    // ================= implicit parameters [implicit-param] =================

    /// Why a fn value does not fit a fn-typed position, in the words a
    /// printed type cannot supply.
    ///
    /// `Ty`'s Display shows parameters, effects and the result, but **not the
    /// contract** — what a call does to each argument — so two types that
    /// differ only there render identically, and the bare mismatch reads
    /// "expects `(Int, Int) -> Int`, found `(Int, Int) -> Int`". This names
    /// the argument, the direction, and what to write instead. Returns
    /// `None` when the printed types already differ, which is when they are
    /// explanation enough.
    fn fn_fit_reason(&self, want: &Ty, got: &Ty, got_name: &str) -> Option<String> {
        let (
            Ty::Fn {
                params: wp,
                contract: wc,
                effects: we,
                ..
            },
            Ty::Fn {
                params: gp,
                contract: gc,
                effects: ge,
                ..
            },
        ) = (want.strip_quals(), got.strip_quals())
        else {
            return None;
        };
        if wp.len() != gp.len() || want.to_string() != got.to_string() {
            return None; // the printed types differ; they speak for themselves
        }
        // [fn-effects] A value may perform *fewer* effects than the position
        // allows, never more.
        let extra: Vec<String> = ge
            .iter()
            .filter(|e| !we.contains(e))
            .map(|e| e.to_string())
            .collect();
        if !extra.is_empty() {
            return Some(format!(
                "`{got_name}` performs `[{}]`, which this position does not \
                 declare — a fn type without an effect list performs nothing, \
                 and a call site cannot supply what it was never told about",
                extra.join(", ")
            ));
        }
        // [fn-contract] Expected kept ⇒ supplied must keep. The reverse
        // direction is fine, so only this way round is worth explaining.
        let at = |c: &Option<Vec<FnParamContract>>, i: usize| -> (bool, bool, Option<String>) {
            match c {
                None => (true, false, None),
                Some(list) => list
                    .get(i)
                    .map(|e| (e.kept, e.mutable, e.name.clone()))
                    .unwrap_or((true, false, None)),
            }
        };
        for i in 0..wp.len() {
            let (w_kept, w_mut, w_name) = at(wc, i);
            let (g_kept, g_mut, g_name) = at(gc, i);
            let which = g_name
                .or(w_name)
                .map(|n| format!("`{n}`"))
                .unwrap_or_else(|| format!("argument {}", i + 1));
            if w_kept && !g_kept {
                return Some(format!(
                    "`{got_name}` *consumes* {which} while this position keeps it: \
                     the types match, the contracts do not. Either give \
                     `{got_name}` a deduction clause that gives it back \
                     (`=> {}`), or declare the position as consuming \
                     (`=>[param] !{}` on the enclosing declaration)",
                    which.trim_matches('`'),
                    which.trim_matches('`')
                ));
            }
            if g_mut && !w_mut && w_kept {
                return Some(format!(
                    "`{got_name}` mutates {which}, which this position does not \
                     permit: a `Mut` argument has to be declared where the \
                     function is *taken*, not only where it is written"
                ));
            }
        }
        None
    }

    /// [implicit-fn-only] Implicit parameters are declared on fns. Anywhere
    /// else there is no call site that could resolve them: an effect member
    /// is reached through a handler, a group member is a signature for a
    /// parameter, and a lambda's type has no room for one.
    fn reject_implicits(&mut self, f: &'p FnDecl, what: &str) {
        for p in f.params.iter().filter(|p| p.implicit) {
            self.error(
                p.span,
                format!(
                    "{what} cannot take implicit parameters: only a fn \
                     declaration has a call site that resolves them"
                ),
            );
        }
        for g in &f.implicit_groups {
            self.error(
                g.span,
                format!(
                    "{what} cannot spread a `params` group: only a fn \
                     declaration has a call site that resolves one"
                ),
            );
        }
    }

    /// The implicit parameters of a fn, in the order both sides render them:
    /// the written `?name: FnType` ones first, then each `?Group<T>` spread's
    /// members in declaration order [implicit-group].
    ///
    /// A group has no binder (user decision 2026-09-05): its members become
    /// implicit parameters in their own right, so an inner fn can declare
    /// `?add` directly, or reach the same parameter through a different
    /// grouping, and a call site overrides one by its own name.
    fn expand_implicits(&mut self, f: &'p FnDecl) -> Vec<ImplicitParam> {
        let out = self.collect_implicits(f);
        match self.own_fn {
            Some(key) => {
                self.out.implicit_params.insert(key, out.clone());
            }
            // A *member* — a handler's implementation of an effect member —
            // has no `FnKey`, so it is keyed by its own name span, exactly
            // as the effect member it implements is [implicit-param].
            None if !out.is_empty() => {
                self.out
                    .implicit_members
                    .insert(self.key(f.name.span), out.clone());
            }
            None => {}
        }
        out
    }

    /// The expansion itself, without recording it: shared by fns and by
    /// effect members, which have no `FnKey` to record under.
    ///
    /// Must be called with the declaration's generics in scope — for a member
    /// that means the effect's generics as well as its own, or the types
    /// would not lower to variables.
    fn collect_implicits(&mut self, f: &'p FnDecl) -> Vec<ImplicitParam> {
        let mut out: Vec<ImplicitParam> = Vec::new();
        for p in f.params.iter().filter(|p| p.implicit) {
            let ty = self.lower_type(&p.ty);
            // [implicit-param] Only a *function* can be resolved by name and
            // type: the name is a fn name, and what fills it is a fn value.
            if !matches!(ty.strip_quals(), Ty::Fn { .. }) && !ty.is_unknown() {
                self.error(
                    p.span,
                    format!(
                        "an implicit parameter must have a function type, but `{}` is \
                         `{ty}`: what fills it is resolved as a function of that name",
                        p.name.name
                    ),
                );
                continue;
            }
            out.push(ImplicitParam {
                name: p.name.name.clone(),
                ty,
                span: p.span,
                borrowed_arms: match &p.ty {
                    ast::Type::Fn { ret, .. } => proj_arm_indices(ret),
                    _ => Vec::new(),
                },
            });
        }
        for g in &f.implicit_groups {
            let Some(group) = self.scope.param_groups.get(g.name.name.as_str()).copied() else {
                self.error(
                    g.span,
                    format!(
                        "no `params` group named `{}` is in scope: `?{}` spreads a \
                         group's members as implicit parameters",
                        g.name.name, g.name.name
                    ),
                );
                continue;
            };
            // The spread's type arguments bind the group's generics. Written
            // types, so they are validated like any other declaration site —
            // which is also what reports `self` here, since only an obligation
            // binds it [group-self].
            for a in &g.args {
                self.validate_type(a);
            }
            let args: Vec<Ty> = g.args.iter().map(|a| self.lower_type(a)).collect();
            if args.len() != group.generics.len() {
                self.error(
                    g.span,
                    format!(
                        "`{}` takes {} type argument(s), found {}",
                        group.name.name,
                        group.generics.len(),
                        args.len()
                    ),
                );
                continue;
            }
            let subst: HashMap<String, Ty> = group
                .generics
                .iter()
                .map(|p| p.name.clone())
                .zip(args)
                .collect();
            let group_generics: HashSet<String> =
                group.generics.iter().map(|p| p.name.clone()).collect();
            for member in &group.fns {
                let saved = self.enter_generics(&group.generics);
                let ty = self.member_fn_ty(member);
                self.generics = saved;
                let ty = substitute_vars(&ty, &subst, &group_generics);
                // [yield-proj] `?Yield<It, proj T>`: a `proj` on a spread
                // argument marks the arms of the member's return that mention
                // that generic as borrows — the caller requires a pass that
                // *walks* data. The member's own return may mark arms too.
                let mut borrowed_arms = member
                    .return_type
                    .as_ref()
                    .map(proj_arm_indices)
                    .unwrap_or_default();
                for (gi, arg) in g.args.iter().enumerate() {
                    if first_proj_span(arg).is_none() {
                        continue;
                    }
                    let Some(generic) = group.generics.get(gi) else {
                        continue;
                    };
                    if let Some(ast::Type::Union { arms, .. }) = member.return_type.as_ref() {
                        for (i, arm) in arms.iter().enumerate() {
                            if type_mentions_generic(arm, &generic.name)
                                && !borrowed_arms.contains(&i)
                            {
                                borrowed_arms.push(i);
                            }
                        }
                    }
                }
                borrowed_arms.sort_unstable();
                out.push(ImplicitParam {
                    name: member.name.name.clone(),
                    ty,
                    span: g.span,
                    borrowed_arms,
                });
            }
        }
        // Two implicits of the same name cannot both be filled: with no
        // binder there is nothing to tell them apart, and [var-no-shadow]
        // would refuse them in the body anyway. The remedy is to write the
        // members out individually under distinct names.
        let mut seen: HashMap<&str, Span> = HashMap::new();
        let mut duplicates: Vec<(String, Span)> = Vec::new();
        for p in &out {
            if seen.contains_key(p.name.as_str()) {
                duplicates.push((p.name.clone(), p.span));
            } else {
                seen.insert(p.name.as_str(), p.span);
            }
        }
        for (name, span) in duplicates {
            self.error(
                span,
                format!(
                    "`{name}` is declared as an implicit parameter twice: with no \
                     binder there is no way to tell them apart, so write the ones \
                     that clash individually under distinct names"
                ),
            );
        }
        out
    }

    /// The fn type a *declared* fn has as a value: parameters, result,
    /// effects, and the **contract** — what a call does to each argument
    /// [fn-contract]. Resolution needs the contract, because a fn that
    /// consumes an argument cannot fill a position that keeps it, and that
    /// difference does not show in a printed type.
    fn fn_value_ty(&mut self, key: FnKey, decl: &'p FnDecl) -> Ty {
        let saved = self.enter_generics(&decl.generics);
        let params: Vec<Ty> = decl.params.iter().map(|p| self.lower_type(&p.ty)).collect();
        let ret = decl
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);
        self.generics = saved;
        let facts: Option<Vec<crate::deduce::ParamDeduction>> =
            self.effective_contract(Some(key), decl);
        let contract = facts.map(|facts| {
            decl.params
                .iter()
                .zip(&params)
                .map(|(p, pty)| {
                    let entry = facts.iter().find(|d| d.param == p.name.name);
                    FnParamContract {
                        name: Some(p.name.name.clone()),
                        kept: entry.map(|d| d.kept).unwrap_or(true),
                        effect: entry
                            .map(|d| d.effect.clone())
                            .unwrap_or(QualEffect::KeepAll),
                        mutable: pty.quals().iter().any(|q| q.name == "Mut"),
                        lent: entry.is_some_and(|d| d.lent),
                    }
                })
                .collect()
        });
        let effects = self.out.fn_effects.get(&key).cloned().unwrap_or_default();
        Ty::Fn {
            params,
            ret: Box::new(ret),
            contract,
            effects,
        }
    }

    /// The fn type of a `params` group member (or any bodiless signature),
    /// as a value of that type would have.
    fn member_fn_ty(&mut self, member: &'p FnDecl) -> Ty {
        let params: Vec<Ty> = member
            .params
            .iter()
            .map(|p| self.lower_type(&p.ty))
            .collect();
        let ret = member
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);
        // [fn-contract] [implicit-group] The member's own deduction list is
        // its contract, exactly as a declared fn's is. Without
        // it every parameter read as kept-and-immutable, so a member
        // declared `fn next(it: Mut It) -> [it: Mut] …` could not be filled
        // by any implementation — the supplied fn mutates, the position
        // (silently) did not permit it. That is `params Yield`'s shape, so
        // the whole composition rendering depended on this.
        let facts: Option<Vec<crate::deduce::ParamDeduction>> = member
            .deductions
            .as_ref()
            .map(|list| crate::deduce::from_written(member, list, &HashSet::new(), |_, _| {}));
        let contract = facts.map(|facts| {
            member
                .params
                .iter()
                .zip(&params)
                .map(|(p, pty)| {
                    let entry = facts.iter().find(|d| d.param == p.name.name);
                    FnParamContract {
                        name: Some(p.name.name.clone()),
                        kept: entry.map(|d| d.kept).unwrap_or(true),
                        effect: entry
                            .map(|d| d.effect.clone())
                            .unwrap_or(QualEffect::KeepAll),
                        mutable: pty.quals().iter().any(|q| q.name == "Mut"),
                        lent: entry.is_some_and(|d| d.lent),
                    }
                })
                .collect()
        });
        Ty::Fn {
            params,
            ret: Box::new(ret),
            contract,
            effects: Vec::new(),
        }
    }

    /// Fills a call's implicit parameters [implicit-resolve], in order:
    ///
    /// 1. `name = value` written at the call site [implicit-override];
    /// 2. an implicit parameter of the *enclosing* fn with the same name and
    ///    a matching type — forwarding, which is the only possibility inside
    ///    generic code, where nothing about an opaque `T` is knowable
    ///    [implicit-forward] [call-resolve];
    /// 3. a declared fn of that name whose signature matches the required
    ///    type — the same overload query the language already runs, only
    ///    against a type instead of an argument list [fn-overload];
    /// 4. otherwise an error naming both remedies.
    fn resolve_implicits(
        &mut self,
        decl: &'p FnDecl,
        key: Option<FnKey>,
        subst: &HashMap<String, Ty>,
        callee_generics: &HashSet<String>,
        named: &'p [ast::NamedArg],
        span: Span,
    ) {
        let Some(key) = key else { return };
        let implicits = self
            .out
            .implicit_params
            .get(&key)
            .cloned()
            .unwrap_or_default();
        self.fill_implicits(
            &implicits,
            &decl.name.name,
            subst,
            callee_generics,
            named,
            span,
        );
    }

    /// Fills a known list of implicit parameters at one call site — the
    /// shared half of [implicit-resolve], used for both a fn and an effect
    /// member, which is an ordinary signature that happens to be dispatched
    /// through a handler.
    fn fill_implicits(
        &mut self,
        implicits: &[ImplicitParam],
        callee: &str,
        subst: &HashMap<String, Ty>,
        callee_generics: &HashSet<String>,
        named: &'p [ast::NamedArg],
        span: Span,
    ) {
        if implicits.is_empty() {
            for arg in named {
                self.error(
                    arg.span,
                    format!(
                        "`{callee}` has no implicit parameter named `{}`",
                        arg.name.name
                    ),
                );
            }
            return;
        }
        let mut filled: Vec<ImplicitArg> = Vec::new();
        for imp in implicits {
            // The type as this call needs it, with the callee's type
            // arguments substituted in.
            let want = substitute_vars(&imp.ty, subst, callee_generics);
            // 1. Written at the call site.
            if let Some(arg) = named.iter().find(|a| a.name.name == imp.name) {
                let got = self.check_expr(&arg.value, Some(&want));
                if !got.is_unknown() && !is_subtype(&got, &want) {
                    let shown = match &arg.value {
                        Expr::Ident(id) => id.name.clone(),
                        _ => "the value given".to_string(),
                    };
                    let detail = match self.fn_fit_reason(&want, &got, &shown) {
                        Some(reason) => reason,
                        None => format!("expects `{want}`, found `{got}`"),
                    };
                    self.error(
                        arg.span,
                        format!("`{}` does not fit here: {detail}", imp.name),
                    );
                }
                filled.push(ImplicitArg::Given {
                    name: imp.name.clone(),
                    arity: match want.strip_quals() {
                        Ty::Fn { params, .. } => params.len(),
                        _ => 0,
                    },
                });
                continue;
            }
            // 2. Forwarded from the enclosing fn's own implicits.
            if let Some(own) = self.own_implicits.iter().find(|p| p.name == imp.name) {
                if is_subtype(&own.ty, &want) || own.ty.is_unknown() || want.is_unknown() {
                    filled.push(ImplicitArg::Forwarded {
                        name: imp.name.clone(),
                    });
                    continue;
                }
            }
            // 3. Resolved by name and type among the visible fns.
            match self.resolve_implicit_fn(&imp.name, &want) {
                Ok(found) => filled.push(ImplicitArg::Resolved {
                    name: imp.name.clone(),
                    key: found,
                    want: want.clone(),
                }),
                Err(why) => {
                    let remedy = format!(
                        "declare a matching `fn {}`, or pass one here with `{} = ...`",
                        imp.name, imp.name
                    );
                    let callee_name = callee;
                    match why {
                        // Nothing of that name at all: the remedy is the message.
                        ImplicitMiss::Unknown => self.error(
                            span,
                            format!(
                                "no `{}` for this call: `{}` needs `?{}: {want}` — {remedy}",
                                imp.name, callee_name, imp.name
                            ),
                        ),
                        // Declared, but it does not fit — say why, since the
                        // printed types may look identical.
                        ImplicitMiss::NearMiss(reason) => self.error(
                            span,
                            format!(
                                "no `{}` fits `?{}: {want}` for `{}`: {reason}",
                                imp.name, imp.name, callee_name
                            ),
                        ),
                        ImplicitMiss::Ambiguous(n) => self.error(
                            span,
                            format!(
                                "`{}` is ambiguous for `{}`: {n} declarations match \
                                 `?{}: {want}`, so the choice would be a guess — {remedy}",
                                imp.name, callee_name, imp.name
                            ),
                        ),
                    }
                }
            }
        }
        // A named argument matching no implicit parameter is a mistake, not
        // a silent no-op.
        for arg in named {
            if !implicits.iter().any(|p| p.name == arg.name.name) {
                self.error(
                    arg.span,
                    format!(
                        "`{callee}` has no implicit parameter named `{}`",
                        arg.name.name
                    ),
                );
            }
        }
        self.out.implicit_args.insert(self.key(span), filled);
    }

    /// The fn a name resolves to at a required fn type [implicit-resolve]:
    /// a *unique* visible overload whose signature matches. Ambiguity is an
    /// error rather than a guess, exactly as for an ordinary overloaded call
    /// [fn-overload].
    fn resolve_implicit_fn(&mut self, name: &str, want: &Ty) -> Result<FnKey, ImplicitMiss> {
        let Ty::Fn {
            params: want_params,
            ret: want_ret,
            ..
        } = want.strip_quals()
        else {
            return Err(ImplicitMiss::Unknown);
        };
        // [fn-rename] Only the overloads that still answer to this name.
        let entries: Vec<crate::resolve::FnEntry<'p>> = self.overloads_of(name);
        if entries.is_empty() {
            return Err(ImplicitMiss::Unknown);
        }
        let mut hits: Vec<(FnKey, crate::resolve::Rung)> = Vec::new();
        // The best explanation of a candidate that did not fit, for the
        // diagnostic when nothing does. Ranked, because the *interesting*
        // near-miss is the one a printed type cannot show: a candidate whose
        // shape matches and whose contract does not (rank 0) explains far
        // more than an unrelated overload of the same name (rank 2).
        let mut near: Option<(u8, String)> = None;
        let note = |rank: u8, reason: String, near: &mut Option<(u8, String)>| {
            if near.as_ref().is_none_or(|(r, _)| rank < *r) {
                *near = Some((rank, reason));
            }
        };
        for entry in entries {
            let decl = entry.decl;
            if decl.params.iter().any(|p| p.implicit) {
                // A default that itself needs implicits would have to be
                // resolved recursively; out of scope for now, and silently
                // skipping it is better than picking it and failing later.
                continue;
            }
            if decl.params.len() != want_params.len() {
                note(
                    2,
                    format!(
                        "the `{name}` in scope takes {} argument(s), but the position \
                         needs {}",
                        decl.params.len(),
                        want_params.len()
                    ),
                    &mut near,
                );
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            let have_params: Vec<Ty> = decl.params.iter().map(|p| self.lower_type(&p.ty)).collect();
            let have_ret = decl
                .return_type
                .as_ref()
                .map(|t| self.lower_type(t))
                .unwrap_or_else(Ty::none);
            self.generics = saved;
            // The candidate's own generics bind from the required type, so a
            // universal `fn cmp<T>(a: T, b: T) -> Int` matches every T.
            let mut binding: HashMap<String, Ty> = HashMap::new();
            let generics: HashSet<String> = decl.generics.iter().map(|g| g.name.clone()).collect();
            let matched = have_params
                .iter()
                .zip(want_params)
                .all(|(have, want)| unify(have, want, &mut binding))
                && unify(&have_ret, want_ret, &mut binding);
            if !matched {
                let shown = self.fn_value_ty(entry.key, decl);
                note(
                    1,
                    format!("the `{name}` in scope is `{shown}`, and the position needs `{want}`"),
                    &mut near,
                );
                continue;
            }
            // Parameters are contravariant and the result covariant, as for
            // any fn value [fn-contract]; the *contract* has to fit too, and
            // that part a printed type does not show.
            // The candidate's own generics bind from the required type, so a
            // universal `fn cmp<T>(a: T, b: T) -> Int` is compared *as
            // instantiated* — without this it would never fit a concrete
            // position, and a concrete sibling would win by default.
            let candidate = self.fn_value_ty(entry.key, decl);
            let candidate = substitute_vars(&candidate, &binding, &generics);
            if fn_value_fits(&candidate, want) {
                hits.push((entry.key, entry.rung));
            } else {
                let reason = self.fn_fit_reason(want, &candidate, name).unwrap_or_else(|| {
                    format!("the `{name}` in scope is `{candidate}`, and the position needs `{want}`")
                });
                note(0, reason, &mut near);
            }
        }
        // [fn-overload-scope] Like a call, the most specific *scope* that has
        // a fitting candidate wins before ambiguity is declared: a program
        // declaring its own pass under a name std also uses (`ListYield` plus
        // its `next`) resolves to its own `next` rather than colliding with
        // core's — the same ladder every named call already walks.
        if let Some(top) = hits.iter().map(|(_, rung)| *rung).max() {
            hits.retain(|(_, rung)| *rung == top);
        }
        match hits.len() {
            0 => Err(match near {
                Some((_, reason)) => ImplicitMiss::NearMiss(reason),
                None => ImplicitMiss::Unknown,
            }),
            1 => Ok(hits[0].0),
            n => Err(ImplicitMiss::Ambiguous(n)),
        }
    }

    /// [fn-value-select] A function passed **by name** (`apply(describe, x)`),
    /// with an optional `@module` selector. Selection is the same three-step
    /// rule a call uses [fn-overload-scope] — `@module`, then the most
    /// specific rung, then the most specific signature — except that what a
    /// candidate has to fit is the **expected fn type** rather than an
    /// argument list (user decision 2026-09-07). Before this, an overloaded
    /// name resolved to whichever overload was declared first, which then
    /// failed to match wherever it was going.
    ///
    /// With no expected fn type there is nothing to fit, so a single
    /// candidate is taken and an overloaded name is an error naming the two
    /// remedies — an annotation, or `rename`.
    fn fn_value_by_name(
        &mut self,
        name: &str,
        name_span: Span,
        at: Option<&'p [ast::Ident]>,
        expected: Option<&Ty>,
    ) -> Ty {
        let entries: Vec<crate::resolve::FnEntry<'p>> = self.overloads_of(name);
        if entries.is_empty() {
            return Ty::Unknown;
        }
        // [free-send-fn] A send-kind function is not a value: it runs by being
        // scheduled, and a fn value runs by being called — there is no
        // position a `send fn` could fill. (A mint is not a value use: it
        // names the target, and the name never becomes a `Ty::Fn`.)
        if entries.iter().all(|e| e.decl.is_send) {
            self.error(
                name_span,
                format!(
                    "`{name}` is a `send fn`, so it is not a value: it runs by being \
                     scheduled, and a function value runs by being called. Mint a \
                     continuation for it instead — `replyto {name}(…)`"
                ),
            );
            return Ty::Unknown;
        }
        // `@module` first, exactly as in a call.
        let mut pool: Vec<crate::resolve::FnEntry<'p>> = entries;
        if let Some(path) = at {
            let wanted = path
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let named: Vec<crate::resolve::FnEntry<'p>> = pool
                .iter()
                .copied()
                .filter(|e| e.module.to_string() == wanted)
                .collect();
            if named.is_empty() {
                self.error(
                    name_span,
                    format!("no function named `{name}` is declared in module `{wanted}`"),
                );
                return Ty::Unknown;
            }
            pool = named;
        }
        // The most specific rung that has a candidate.
        if let Some(top) = pool.iter().map(|e| e.rung).max() {
            pool.retain(|e| e.rung == top);
        }
        // Then the expected type, if there is one: keep the candidates whose
        // value type fits it.
        if pool.len() > 1 {
            if let Some(want) = expected {
                let fitting: Vec<crate::resolve::FnEntry<'p>> = pool
                    .iter()
                    .copied()
                    .filter(|e| {
                        let candidate = self.fn_value_ty(e.key, e.decl);
                        fn_value_fits(&candidate, want)
                    })
                    .collect();
                if fitting.is_empty() {
                    // None of them fits — which is a *mismatch*, not an
                    // ambiguity, and the reason is worth naming: a contract
                    // difference does not show in a printed type
                    // [fn-contract].
                    let mut shown: Vec<String> = pool
                        .iter()
                        .map(|e| {
                            let ty = self.fn_value_ty(e.key, e.decl);
                            match self.fn_fit_reason(want, &ty, name) {
                                Some(reason) => format!("`{ty}` ({reason})"),
                                None => format!("`{ty}`"),
                            }
                        })
                        .collect();
                    shown.sort();
                    shown.dedup();
                    self.error(
                        name_span,
                        format!(
                            "no overload of `{name}` fits `{want}` here: {}",
                            shown.join("; ")
                        ),
                    );
                    return Ty::Unknown;
                }
                pool = fitting;
            }
        }
        if pool.len() > 1 {
            let mut shown: Vec<String> = pool
                .iter()
                .map(|e| {
                    let ty = self.fn_value_ty(e.key, e.decl);
                    format!("`{ty}`")
                })
                .collect();
            shown.sort();
            shown.dedup();
            self.error(
                name_span,
                format!(
                    "`{name}` is overloaded, so passing it by name is ambiguous: \
                     {} all fit here. Annotate the position with the fn type you \
                     mean, or give one overload its own name with \
                     `rename fn <new> = {name}(...)`",
                    shown.join(", ")
                ),
            );
            return Ty::Unknown;
        }
        let entry = pool[0];
        // [fn-ref-table]
        self.out.fn_refs.insert(self.key(name_span), entry.key);
        // [fn-rename] A renamed *value* is erased too.
        if self.renamed(name).is_some() {
            self.out.renamed_calls.insert(self.key(name_span));
        }
        let decl = entry.decl;
        let saved = self.enter_generics(&decl.generics);
        let params: Vec<Ty> = decl.params.iter().map(|p| self.lower_type(&p.ty)).collect();
        let ret = self.fn_return_ty(decl);
        self.generics = saved;
        // [fn-contract] The fn value carries the declaration's contract
        // (written list, else the inferred facts), so boundary checks compare
        // real modes — a consuming fn no longer masquerades as
        // keeps-everything.
        let facts: Option<Vec<crate::deduce::ParamDeduction>> = match &decl.deductions {
            Some(_) => self.effective_contract(Some(entry.key), decl),
            None => self
                .inferred
                .and_then(|table| table.get(&entry.key).cloned()),
        };
        let contract = facts.map(|facts| {
            decl.params
                .iter()
                .zip(&params)
                .map(|(p, pty)| {
                    let entry = facts.iter().find(|d| d.param == p.name.name);
                    FnParamContract {
                        name: Some(p.name.name.clone()),
                        kept: entry.map(|d| d.kept).unwrap_or(true),
                        effect: entry
                            .map(|d| d.effect.clone())
                            .unwrap_or(QualEffect::KeepAll),
                        mutable: pty.quals().iter().any(|q| q.name == "Mut"),
                        lent: entry.is_some_and(|d| d.lent),
                    }
                })
                .collect()
        });
        // [fn-effects] A named fn passed by value performs exactly the
        // effects it declares (inherited entries included) — its callers
        // supply them.
        let effects = self
            .out
            .fn_effects
            .get(&entry.key)
            .cloned()
            .unwrap_or_default();
        // [fn-effects] What the *use site* expects the value to accept: a
        // pure fn passed where effects are expected must still take (and
        // ignore) them, so the emitters adapt against the expectation, not
        // the declaration.
        let taken = match expected.map(|t| t.strip_quals()) {
            Some(Ty::Fn {
                effects: exp_effects,
                ..
            }) => exp_effects.clone(),
            _ => effects.clone(),
        };
        self.out.lambda_effects.insert(self.key(name_span), taken);
        let candidate = Ty::Fn {
            params,
            ret: Box::new(ret),
            contract,
            effects,
        };
        // [fn-value-select] A **generic** fn passed by name instantiates
        // from the position's expected fn type (closed 2026-09-12 with the
        // consuming-callback pattern: `drain(counter, drop)` binds `drop`'s
        // `T = Counter`). Without this the value's type kept its unbound
        // `Var`s and no concrete position could unify with it. The target
        // languages infer the instantiation at the adapter's forwarding
        // call, so the emitters need nothing.
        if !decl.generics.is_empty() {
            if let Some(want) = expected.map(|t| t.strip_quals()) {
                if matches!(want, Ty::Fn { .. }) {
                    let mut subst: HashMap<String, Ty> = HashMap::new();
                    if unify(&candidate, want, &mut subst) && !subst.is_empty() {
                        let generic_names: HashSet<String> =
                            decl.generics.iter().map(|g| g.name.clone()).collect();
                        // [linear-generics] The instantiation ban applies
                        // here as at any call: an unopted generic cannot
                        // bind a linear type.
                        if self.inferred.is_some() {
                            let opted: HashSet<&str> = decl
                                .generic_canbe
                                .iter()
                                .filter(|(_, q)| q.name.name == "linear")
                                .map(|(id, _)| id.name.as_str())
                                .collect();
                            for g in &decl.generics {
                                if opted.contains(g.name.as_str()) {
                                    continue;
                                }
                                if let Some(bound) = subst.get(&g.name) {
                                    if self.ty_own_linear(bound) {
                                        self.error(
                                            name_span,
                                            format!(
                                                "cannot instantiate generic parameter \
                                                 `{}` of `{name}` with linear type \
                                                 `{bound}`: `{name}` does not declare \
                                                 `<{} canbe linear>`, so it does not \
                                                 honor the use obligation",
                                                g.name, g.name
                                            ),
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                        return substitute_vars(&candidate, &subst, &generic_names);
                    }
                }
            }
        }
        candidate
    }

    /// [fn-rename] Brings a `rename fn` into force: resolves the overload it
    /// names, checks the new name is free, and records the binding.
    ///
    /// The overload is matched by parameter names and types, positionally for
    /// type parameters — the same match a `refn` uses [qual-refn-match], so a
    /// std rename surfaces as a diagnostic here rather than as a rename that
    /// silently stops applying.
    fn declare_rename(&mut self, decl: &'p ast::RenameDecl) {
        // The rename's own type parameters are in scope for its parameter
        // list, exactly as a `refn`'s are [qual-refn-match].
        let saved = self.enter_generics(&decl.generics);
        for p in &decl.params {
            self.validate_type(&p.ty);
        }
        self.generics = saved;
        let new_name = decl.name.name.as_str();
        // The new name must be free: a rename exists to remove an ambiguity,
        // so adding one to an existing name would defeat it.
        if self.scope.fns.contains_key(new_name) {
            self.error(
                decl.name.span,
                format!(
                    "`{new_name}` is already a function in scope, so it cannot \
                     name a renamed overload too: pick a name of its own (a \
                     rename is not an alias — the overload stops answering to \
                     `{}`)",
                    decl.target.name
                ),
            );
            return;
        }
        if self.scope.effect_members.contains_key(new_name) {
            self.error(
                decl.name.span,
                format!("`{new_name}` is already an effect member in scope"),
            );
            return;
        }
        if let Some(prev) = self.renames.iter().find(|r| r.new_name == new_name) {
            let prev_span = prev.span;
            self.error(
                decl.name.span,
                format!(
                    "`{new_name}` already names a renamed overload in this scope \
                     (declared at {}..{})",
                    prev_span.start, prev_span.end
                ),
            );
            return;
        }
        let saved = self.enter_generics(&decl.generics);
        let matched = crate::refine::match_overload(
            self.scope,
            &decl.target.name,
            &decl.generics,
            &decl.params,
        );
        self.generics = saved;
        match matched {
            Ok(entry) => {
                // [lsp-definition] The new name points at the declaration it
                // renames, so go-to-definition works through it.
                self.out.fn_refs.insert(self.key(decl.name.span), entry.key);
                self.renames.push(RenameBinding {
                    new_name: new_name.to_string(),
                    target: decl.target.name.clone(),
                    entry,
                    span: decl.span,
                });
            }
            Err(shapes) if shapes.is_empty() => self.error(
                decl.target.span,
                format!(
                    "no function `{}` is visible here, so there is nothing to \
                     rename",
                    decl.target.name
                ),
            ),
            Err(shapes) => self.error(
                decl.target.span,
                format!(
                    "this names no `{}` in scope: a rename repeats one overload's \
                     parameters exactly — same names, same types (type \
                     parameters match by position). In scope: {}",
                    decl.target.name,
                    shapes.join(", ")
                ),
            ),
        }
    }

    /// [fn-rename] The overload a *renamed* name means, if `name` is one.
    fn renamed(&self, name: &str) -> Option<crate::resolve::FnEntry<'p>> {
        self.renames
            .iter()
            .rev()
            .find(|r| r.new_name == name)
            .map(|r| r.entry)
    }

    /// [fn-rename] Whether this overload has been renamed away from `name` —
    /// "*only* the new name is valid for that variant".
    fn renamed_away(&self, name: &str, key: FnKey) -> bool {
        self.renames
            .iter()
            .any(|r| r.target == name && r.entry.key == key)
    }

    /// [fn-rename] The overloads of `name` that still answer to it, in scope
    /// order — the candidate set every resolution path starts from.
    fn overloads_of(&self, name: &str) -> Vec<crate::resolve::FnEntry<'p>> {
        if let Some(entry) = self.renamed(name) {
            return vec![entry];
        }
        self.scope
            .fns
            .get(name)
            .map(|entries| {
                entries
                    .iter()
                    .copied()
                    .filter(|e| !self.renamed_away(name, e.key))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// [fn-rename] Whether anything answers to this name — a declared
    /// overload that has not been renamed away, or a rename itself.
    fn has_callable(&self, name: &str) -> bool {
        !self.overloads_of(name).is_empty()
    }

    /// [fn-overload-scope] [fn-overload-rank] [fn-overload-ambiguous] Picks
    /// the winner among the candidates that fit a call — the one rule the
    /// whole language routes through (user decisions 2026-09-07):
    ///
    /// 1. **`@module` first.** A call that names a module means that
    ///    module's overloads and no others; naming a module with no fitting
    ///    overload is an error rather than a silent fallback.
    /// 2. **The most specific *scope* wins.** Functions arrive from ever
    ///    more specific places — `core`, then this file's imports, then this
    ///    module — and only the most specific rung that has a fitting
    ///    candidate competes. That is what makes a module's own `map` mean
    ///    *its* `map`, whatever std declares.
    /// 3. **Then the most specific *signature*.** A partial order
    ///    [fn-overload-rank]: no single most specific candidate is an
    ///    ambiguity error naming the remedies, never a pick.
    ///
    /// Scope beats signature, so a broad overload in this module hides a
    /// precise one in `core` — deliberately, because the alternative is a
    /// rule nobody can predict without knowing std's surface. When that
    /// happens the call gets a **warning** naming the discarded candidate,
    /// and writing `@module` on the call silences it (either module: naming
    /// this one confirms the choice, naming the other takes the precise
    /// overload).
    fn select_overload(
        &mut self,
        name: &str,
        viable: &[Viable<'p>],
        arg_tys: &[Ty],
        at: Option<&'p [ast::Ident]>,
        span: Span,
    ) -> Option<usize> {
        let shown_args = || -> String {
            arg_tys
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        // 1. `@module`: only that module's overloads compete.
        let mut pool: Vec<usize> = (0..viable.len()).collect();
        if let Some(path) = at {
            let wanted = path
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let named_module: Vec<usize> = pool
                .iter()
                .copied()
                .filter(|&i| viable[i].module == wanted)
                .collect();
            if named_module.is_empty() {
                let mut modules: Vec<String> = viable.iter().map(|v| v.module.clone()).collect();
                modules.sort();
                modules.dedup();
                self.error(
                    span,
                    format!(
                        "no overload of `{name}({})` is declared in module \
                         `{wanted}`; the modules that declare a fitting one are: {}",
                        shown_args(),
                        modules.join(", ")
                    ),
                );
                return None;
            }
            pool = named_module;
        }
        // 2. The most specific rung that has a candidate.
        let top_rung = pool.iter().map(|&i| viable[i].rung).max()?;
        let (chosen, shadowed): (Vec<usize>, Vec<usize>) = pool
            .iter()
            .copied()
            .partition(|&i| viable[i].rung == top_rung);
        // 3. The most specific signature within that rung.
        let ranks: Vec<crate::types::RankedCandidate> =
            chosen.iter().map(|&i| viable[i].rank.clone()).collect();
        let winner = match crate::types::most_specific(&ranks) {
            Some(i) => chosen[i],
            None => {
                // [type-unknown-lenient] An un-inferred argument fits every
                // candidate, so it must not produce an ambiguity of its own:
                // one mistake, one diagnostic.
                if arg_tys.iter().any(ty_mentions_unknown) {
                    return Some(chosen[0]);
                }
                let mut shown: Vec<String> = chosen
                    .iter()
                    .map(|&i| Self::render_signature(name, &viable[i]))
                    .collect();
                shown.sort();
                shown.dedup();
                self.error(
                    span,
                    format!(
                        "ambiguous call to `{name}({})`: {} all fit and none is \
                         more specific — a broader union, a smaller qualifier \
                         set and a type variable are each less specific, but \
                         these differ in ways the rule does not rank. Narrow an \
                         argument, or give one overload its own name with \
                         `rename fn <new> = {name}(...)`",
                        shown_args(),
                        shown.join(", ")
                    ),
                );
                return None;
            }
        };
        // The scope-override warning: a *more specific signature* was
        // discarded because it sits on a less specific rung.
        if at.is_none() {
            let overridden = shadowed
                .into_iter()
                .find(|&i| crate::types::spec_dominates(&viable[i].rank, &viable[winner].rank));
            if let Some(other) = overridden {
                let chosen_sig = Self::render_signature(name, &viable[winner]);
                let other_sig = Self::render_signature(name, &viable[other]);
                let (chosen_mod, other_mod) =
                    (viable[winner].module.clone(), viable[other].module.clone());
                let other_rung = viable[other].rung.describe();
                let chosen_rung = viable[winner].rung.describe();
                self.warn(
                    span,
                    format!(
                        "`{name}` resolves to {chosen_sig} from `{chosen_mod}` \
                         because {chosen_rung} is the more specific scope, even \
                         though {other_sig} from `{other_mod}` ({other_rung}) is \
                         the more specific signature. Write \
                         `{name}@{chosen_mod}(...)` to confirm, or \
                         `{name}@{other_mod}(...)` to call that one"
                    ),
                );
            }
        }
        Some(winner)
    }

    /// How a candidate reads in a diagnostic: `name(ParamTy, ParamTy)`, from
    /// the *declared* patterns, since those are what the ranking compared.
    fn render_signature(name: &str, v: &Viable<'p>) -> String {
        let ps: Vec<String> = v.rank.patterns.iter().map(|p| p.to_string()).collect();
        format!("`{name}({})`", ps.join(", "))
    }

    /// [proj-type] [fn-overload-rank] Whether an argument of type `arg` fits
    /// a parameter whose (substituted) type is `param`.
    ///
    /// One predicate, shared by fn-overload selection and by the
    /// member-versus-fn ranking [effect-available], because a call routed to
    /// one path by a *different* fit test than that path applies is a call
    /// that reports "no overload" for something that fits. A kept,
    /// non-`Mut` position reads its argument, so a top-level projection fits
    /// it; a consumed or `Mut` position needs the owned value.
    fn arg_fits_param(&self, arg: &Ty, param: &Ty, kept: bool) -> bool {
        is_subtype(arg, param)
            || (kept
                && !Self::carries_mut(param)
                && arg.is_proj()
                && is_subtype(&arg.strip_top_proj(), param))
    }

    /// [effect-available] Where a name's **one overload set** sends a call
    /// (user decision 2026-09-14): an effect member of an available effect,
    /// or an ordinary fn. The arguments are typed here, once, and handed to
    /// whichever path runs.
    fn route_member_or_fn(
        &mut self,
        name: &str,
        effect: &'p EffectDecl,
        members: &[&'p FnDecl],
        fn_cands: &[crate::resolve::FnEntry<'p>],
        args: &[&'p Expr],
        span: Span,
    ) -> Route {
        // Typed without expected types: the lead-candidate machinery belongs
        // to the fn path and cannot run before the side is known. A bare
        // lambda in a *colliding* call therefore needs an annotation — an
        // error, never a silent difference (recorded cut, ROADMAP.md).
        let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a, None)).collect();
        let m = self.fitting_members(effect, members, &arg_tys);
        let f = self.fitting_fns(fn_cands, &arg_tys);
        let shown = |tys: &[Ty]| -> String {
            tys.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        match (m.is_empty(), f.is_empty()) {
            // Neither side accepts the arguments: one diagnostic listing
            // both, since to the caller they are one name.
            (true, true) => {
                let mut sigs: Vec<String> = Vec::new();
                for decl in members {
                    sigs.push(format!(
                        "`{}.{name}({})`",
                        effect.name.name,
                        shown(&self.declared_param_tys(decl))
                    ));
                }
                for entry in fn_cands {
                    sigs.push(format!(
                        "`{name}({})`",
                        shown(&self.declared_param_tys(entry.decl))
                    ));
                }
                self.error(
                    span,
                    format!(
                        "no `{name}` accepts ({}): the candidates are {}",
                        shown(&arg_tys),
                        sigs.join(", ")
                    ),
                );
                Route::Neither
            }
            (false, true) => Route::Member(arg_tys),
            (true, false) => Route::Fn(arg_tys),
            // Both fit: the more specific signature wins, exactly as two fn
            // overloads settle it [fn-overload-rank]. Each side's own best
            // stands for it — an internally ambiguous side reports that
            // itself, on the path it belongs to.
            (false, false) => {
                let m_ranks: Vec<crate::types::RankedCandidate> =
                    m.iter().map(|(_, r)| r.clone()).collect();
                let f_ranks: Vec<crate::types::RankedCandidate> =
                    f.iter().map(|(_, r)| r.clone()).collect();
                let mb = crate::types::most_specific(&m_ranks).unwrap_or(0);
                let fb = crate::types::most_specific(&f_ranks).unwrap_or(0);
                let pair = vec![m_ranks[mb].clone(), f_ranks[fb].clone()];
                match crate::types::most_specific(&pair) {
                    Some(0) => Route::Member(arg_tys),
                    Some(_) => Route::Fn(arg_tys),
                    None => {
                        let module = fn_cands[f[fb].0].module;
                        self.error(
                            span,
                            format!(
                                "ambiguous call to `{name}`: the member \
                                 `{}.{name}({})` and the function `{name}({})` \
                                 both accept ({}), and neither is more specific — \
                                 write `{name}@{}(…)` for the member or \
                                 `{name}@{module}(…)` for the function",
                                effect.name.name,
                                shown(&m_ranks[mb].patterns),
                                shown(&f_ranks[fb].patterns),
                                shown(&arg_tys),
                                effect.name.name,
                            ),
                        );
                        // Recover as the member: that is what the call meant
                        // before the two competed, so the rest of the check
                        // sees the same types it used to.
                        Route::Member(arg_tys)
                    }
                }
            }
        }
    }

    /// The declared parameter types of a fn or member, for a diagnostic that
    /// has no fitting patterns to show.
    fn declared_param_tys(&mut self, decl: &'p FnDecl) -> Vec<Ty> {
        let saved = self.enter_generics(&decl.generics);
        let tys: Vec<Ty> = decl
            .params
            .iter()
            .filter(|p| !p.variadic && !p.implicit)
            .map(|p| self.lower_type(&p.ty))
            .collect();
        self.generics = saved;
        tys
    }

    /// [effect-available] [fn-overload-rank] Which fn overloads of a name
    /// accept `arg_tys`, with the patterns compared — the fn side of the
    /// member-versus-fn comparison, and deliberately the *same* test the fn
    /// path itself applies (`arg_fits_param`, the callee's contract for
    /// `kept`, `unify` for generics), so a call is never routed to a path
    /// that then reports "no overload" for something that fits.
    ///
    /// Returned as (index into `candidates`, patterns). Selection proper —
    /// rungs, variadic collection, projection diagnostics — stays in the fn
    /// path: this answers only "does this side have a candidate, and how
    /// specific is its best one".
    fn fitting_fns(
        &mut self,
        candidates: &[crate::resolve::FnEntry<'p>],
        arg_tys: &[Ty],
    ) -> Vec<(usize, crate::types::RankedCandidate)> {
        let mut viable: Vec<(usize, crate::types::RankedCandidate)> = Vec::new();
        for (i, entry) in candidates.iter().enumerate() {
            let decl = entry.decl;
            let fixed: Vec<&Param> = decl
                .params
                .iter()
                .filter(|p| !p.variadic && !p.implicit)
                .collect();
            let variadic = decl.params.iter().find(|p| p.variadic);
            let arity_ok = if variadic.is_some() {
                arg_tys.len() >= fixed.len()
            } else {
                arg_tys.len() == fixed.len()
            };
            if !arity_ok {
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            let mut patterns: Vec<Ty> = Vec::with_capacity(arg_tys.len());
            for i in 0..arg_tys.len() {
                if i < fixed.len() {
                    patterns.push(self.lower_type(&fixed[i].ty));
                } else if let Some(vp) = variadic {
                    let arr = self.lower_type(&vp.ty);
                    patterns.push(match &arr {
                        Ty::Array(e) => (**e).clone(),
                        _ => arr.clone(),
                    });
                }
            }
            self.generics = saved;
            let callee_generics: HashSet<String> =
                decl.generics.iter().map(|g| g.name.clone()).collect();
            let mut subst: HashMap<String, Ty> = HashMap::new();
            if !patterns
                .iter()
                .zip(arg_tys)
                .all(|(p, a)| unify(p, a, &mut subst))
            {
                continue;
            }
            let contract = self.effective_contract(Some(entry.key), decl);
            let mut fits = true;
            for (slot, pattern) in patterns.iter().enumerate() {
                let sp = substitute_vars(pattern, &subst, &callee_generics);
                let kept = contract
                    .as_ref()
                    .and_then(|c| {
                        let pname = fixed.get(slot).map(|fp| fp.name.name.as_str());
                        c.iter()
                            .find(|d| Some(d.param.as_str()) == pname)
                            .map(|d| d.kept)
                    })
                    .unwrap_or(true);
                if !self.arg_fits_param(&arg_tys[slot], &sp, kept) {
                    fits = false;
                    break;
                }
            }
            if fits {
                viable.push((
                    i,
                    crate::types::RankedCandidate {
                        patterns,
                        variadic: variadic.is_some() && arg_tys.len() >= fixed.len(),
                    },
                ));
            }
        }
        viable
    }

    /// [fn-overload-rank] The pool of candidates that may *lead* a
    /// call — provide the expected types its arguments are checked against.
    ///
    /// A single candidate always leads, variadic or not (its fixed
    /// parameters are the patterns). With several, only the fixed-arity
    /// candidates that could take this call at all enter the pool: a
    /// variadic candidate's patterns are per-argument rather than
    /// per-parameter, so it would need the argument types it is supposed to
    /// help produce.
    fn lead_pool(
        &mut self,
        candidates: &[crate::resolve::FnEntry<'p>],
        arity: usize,
        type_args: &'p [ast::Type],
    ) -> Vec<LeadCandidate> {
        let mut pool: Vec<LeadCandidate> = Vec::new();
        for (i, entry) in candidates.iter().enumerate() {
            let decl = entry.decl;
            let fixed: Vec<&Param> = decl
                .params
                .iter()
                .filter(|p| !p.variadic && !p.implicit)
                .collect();
            if candidates.len() > 1
                && (decl.params.iter().any(|p| p.variadic) || fixed.len() != arity)
            {
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            let patterns: Vec<Ty> = fixed.iter().map(|p| self.lower_type(&p.ty)).collect();
            // Explicit type arguments pin the substitution from the start.
            let mut subst: HashMap<String, Ty> = HashMap::new();
            for (g, ta) in decl.generics.iter().zip(type_args) {
                let lowered = self.lower_type(ta);
                subst.insert(g.name.clone(), lowered);
            }
            self.generics = saved;
            pool.push(LeadCandidate {
                index: i,
                rank: crate::types::RankedCandidate {
                    patterns,
                    variadic: false,
                },
                generics: decl.generics.iter().map(|g| g.name.clone()).collect(),
                subst,
                rung: entry.rung,
            });
        }
        pool
    }

    /// [fn-overload-rank] Which pool entry leads: the only one, or the
    /// one whose parameter patterns dominate every other's on the
    /// specificity axis. `None` when none dominates — expected types from an
    /// arbitrary candidate really would be a bias, so the arguments keep the
    /// untyped probe.
    fn dominant_lead(pool: &[LeadCandidate]) -> Option<usize> {
        if pool.is_empty() {
            return None;
        }
        // [fn-overload-scope] The most specific rung first, exactly as the
        // winner is chosen — otherwise a `core` candidate would be typing
        // the arguments of a call that resolves in this module.
        let top = pool.iter().map(|c| c.rung).max()?;
        let at_top: Vec<usize> = (0..pool.len()).filter(|&i| pool[i].rung == top).collect();
        if at_top.len() == 1 {
            return Some(at_top[0]);
        }
        let ranks: Vec<crate::types::RankedCandidate> =
            at_top.iter().map(|&i| pool[i].rank.clone()).collect();
        crate::types::most_specific(&ranks).map(|i| at_top[i])
    }

    /// [lit-adopt] The numeric type an unsuffixed literal argument may adopt
    /// at position `i` of a call, read off the candidate pool.
    ///
    /// Deliberately conservative, because adoption must not silently re-rank
    /// an overload set [fn-overload-rank]:
    ///
    /// * a candidate whose parameter there is exactly the literal's own type
    ///   (`Int` for an integer literal, `Double` for a float one) blocks
    ///   adoption — that candidate already matches, and it should keep
    ///   winning;
    /// * otherwise the candidates must name exactly *one* numeric type
    ///   between them, which is then the adopted one. Two different numeric
    ///   parameters at one position leave the literal alone rather than
    ///   guessing which overload was meant.
    ///
    /// Non-numeric and generic patterns are ignored: they neither block
    /// adoption nor supply a target, so `twice(9)` over
    /// `twice(Long)`/`twice(Str)` adopts `Long`.
    fn adoptable_numeric_param(
        pool: &[LeadCandidate],
        i: usize,
        float_lit: bool,
    ) -> Option<Ty> {
        let own = if float_lit { "Double" } else { "Int" };
        let mut found: Option<&str> = None;
        for candidate in pool {
            let Some(pattern) = candidate.rank.patterns.get(i) else {
                continue;
            };
            let Some(name) = numeric_expectation(pattern) else {
                continue;
            };
            if name == own {
                return None;
            }
            match found {
                None => found = Some(name),
                Some(seen) if seen == name => {}
                Some(_) => return None,
            }
        }
        found.map(Ty::named)
    }

    /// [implicit-infer] Extends a call's progressive substitution with what
    /// its callee's **implicit parameters** determine (user design 2026-09-06,
    /// built with S-Seq).
    ///
    /// `params Yield<It, T> { fn next(it: Mut It) -> [it: Mut] Emitted T |
    /// Finished }` is how Salvo says "anything iterable": `map<It, T, U>(it:
    /// Mut It, f: (T) -> U, ?Yield<It, T>)` binds `It` from its first
    /// argument, and `T` is determined by *which `next` fills the implicit* —
    /// resolving it at `(Mut ListYield<Int>) -> Emitted T | Finished` finds
    /// std's `next(Mut ListYield<T'>)` and reads `T = Int` back off it.
    /// Without this step the lambda would be
    /// typed against an unbound `T`, and `U` would be undeterminable
    /// [call-type-args] — the generic half of a sequence function would only
    /// ever work with a written type-argument list.
    ///
    /// Two-sided, which is why it cannot reuse `resolve_implicit_fn`: the
    /// candidate's own generics bind from the *known* part of the pattern,
    /// and then the caller's variables bind from the instantiated candidate.
    /// Anything ambiguous or absent is left alone: [implicit-resolve] reports
    /// it at the end of the call, and one mistake gets one diagnostic.
    fn extend_subst_from_implicits(
        &mut self,
        key: Option<FnKey>,
        callee_generics: &HashSet<String>,
        progressive: &mut HashMap<String, Ty>,
    ) {
        let Some(key) = key else { return };
        let implicits = match self.out.implicit_params.get(&key) {
            Some(list) if !list.is_empty() => list.clone(),
            _ => return,
        };
        // [implicit-infer] Repeated until it stops learning, because one
        // implicit can determine another's type: `?iter: (c: C) -> Mut It`
        // teaches `It`, which is what makes the `?Yield<It, T>` beside it
        // resolvable at all (user decision 2026-09-10). Declaration order is
        // therefore not a constraint on the author, and the cap is the number of
        // implicits — each round has to bind at least one variable to continue.
        for _ in 0..=implicits.len() {
            let before = progressive.len();
            self.learn_from_implicits_once(&implicits, callee_generics, progressive);
            if progressive.len() == before {
                break;
            }
        }
    }

    /// One sweep of [implicit-infer]: each implicit that still mentions an
    /// unbound variable of the callee is resolved against what is known, and
    /// what that determines is read back.
    fn learn_from_implicits_once(
        &mut self,
        implicits: &[ImplicitParam],
        callee_generics: &HashSet<String>,
        progressive: &mut HashMap<String, Ty>,
    ) {
        for imp in implicits {
            // The implicit's type as far as this call knows it, with the
            // *unknown* parts left as variables (`substitute_vars` would
            // erase them to `Unknown`, and then there would be nothing to
            // bind).
            let pattern = substitute_known(&imp.ty, progressive, callee_generics);
            if !ty_mentions_vars(&pattern, callee_generics) {
                continue; // nothing left to learn from this one
            }
            let Ty::Fn {
                params: want_params,
                ret: want_ret,
                ..
            } = pattern.strip_quals().clone()
            else {
                continue;
            };
            // [fn-rename] A renamed overload no longer answers to this name,
            // so it cannot fill an implicit parameter of it either.
            let entries: Vec<crate::resolve::FnEntry<'p>> = self.overloads_of(&imp.name);
            if entries.is_empty() {
                continue;
            }
            let mut found: Option<Ty> = None;
            let mut hits = 0usize;
            for entry in entries {
                let decl = entry.decl;
                if decl.params.iter().any(|p| p.implicit || p.variadic) {
                    continue;
                }
                if decl.params.len() != want_params.len() {
                    continue;
                }
                let saved = self.enter_generics(&decl.generics);
                let have_params: Vec<Ty> =
                    decl.params.iter().map(|p| self.lower_type(&p.ty)).collect();
                let have_ret = decl
                    .return_type
                    .as_ref()
                    .map(|t| self.lower_type(t))
                    .unwrap_or_else(Ty::none);
                self.generics = saved;
                // Bind the candidate's own generics from the parts the call
                // already knows. A part that is still one of *our* variables
                // teaches nothing and must not match everything.
                let mut binding: HashMap<String, Ty> = HashMap::new();
                let matched = have_params.iter().zip(&want_params).all(|(have, want)| {
                    ty_mentions_vars(want, callee_generics) || unify(have, want, &mut binding)
                });
                if !matched {
                    continue;
                }
                let generics: HashSet<String> =
                    decl.generics.iter().map(|g| g.name.clone()).collect();
                hits += 1;
                found = Some(Ty::Fn {
                    params: have_params
                        .iter()
                        .map(|p| substitute_vars(p, &binding, &generics))
                        .collect(),
                    ret: Box::new(substitute_vars(&have_ret, &binding, &generics)),
                    contract: None,
                    effects: Vec::new(),
                });
            }
            if hits != 1 {
                continue;
            }
            let Some(Ty::Fn {
                params: have_params,
                ret: have_ret,
                ..
            }) = found
            else {
                continue;
            };
            // Read our own variables back off the instantiated candidate.
            let mut extended = progressive.clone();
            let ok = want_params
                .iter()
                .zip(&have_params)
                .all(|(want, have)| unify(want, have, &mut extended))
                && unify(&want_ret, &have_ret, &mut extended);
            if ok {
                *progressive = extended;
            }
        }
    }

    fn check_fn(&mut self, f: &'p FnDecl, extra_params: &'p [Param], state: &'p [FieldDecl]) {
        let saved_generics = self.enter_generics(&f.generics);
        self.require_explicit_decl(f);
        if f.body.is_none() {
            self.require_full_clause(f, "an `intrinsic fn`");
        }
        for p in &f.params {
            self.validate_type(&p.ty);
            // [proj-readonly] A parameter written with a *top-level*
            // `proj Mut` is self-contradictory, caught here at the
            // declaration (user decision 2026-09-12): the `proj` promises
            // to accept borrowed values, but the `Mut` makes this a `Mut`
            // position, which a `proj` value never satisfies — so every
            // projection argument would be refused, and the body could
            // never use the permission either (the parameter is a
            // projection to it). Nested occurrences (`Mut List<proj Mut
            // Str>`) stay legal: there the `Mut` belongs to the element
            // type a view really holds.
            if let Some(mut_ref) = top_level_proj_mut(&p.ty) {
                self.error(
                    mut_ref.span,
                    format!(
                        "parameter `{}` cannot be written `proj Mut`: a `proj` value \
                         can only be read, whatever its `Mut` says, so no projection \
                         could ever be passed here and the body could never mutate it. \
                         Drop the `Mut` (a `proj` position accepts `proj Mut` \
                         arguments), or drop the `proj` to mutate an owned value \
                         in place",
                        p.name.name
                    ),
                );
            }
        }
        // [implicit-param] [implicit-group] Expand and validate the implicit
        // parameters before anything else needs them: the body sees each as
        // a local of fn type, and every call site reads the same list.
        let implicits = self.expand_implicits(f);
        if let Some(rt) = &f.return_type {
            self.validate_type(rt);
        }
        if f.constructs.is_some() {
            self.check_constructor_sig(f);
        }
        // [readonly-return] `-> proj[from: p] T`: `p` must be a
        // parameter and must be *kept* — a moved parameter's data needs
        // no annotation (the callee owns it), and a borrow of a moved
        // value could not outlive the call.
        self.own_derived_return = f.derived_return.as_ref().map(|id| id.name.clone());
        self.own_derived_sources = f
            .return_type
            .as_ref()
            .and_then(|rt| proj_refs(rt).into_iter().find(|r| !r.from.is_empty()))
            .map(|r| r.from.iter().map(|i| i.name.clone()).collect())
            .unwrap_or_default();
        // [proj-infer] What this fn's result holds borrows of: declared with
        // `[p: proj]`, else inferred from the body, else (no body) every kept
        // parameter. A written list is checked against the body exactly, so
        // it cannot go stale in either direction.
        {
            let lent = self.infer_lends(f);
            self.own_lends_declared = crate::lends::declared_lends(f).is_some();
            self.own_lends = lent
                .iter()
                .filter_map(|&i| f.params.get(i).map(|p| p.name.name.clone()))
                .collect();
            if let Some(key) = self.fn_key_of_decl(f) {
                self.out.fn_lends.insert(key, lent.clone());
            }
            if let (Some(declared), Some(body)) = (crate::lends::declared_lends(f), &f.body) {
                let scope_fns = &self.scope.fns;
                let lookup = |name: &str| -> Vec<&'p FnDecl> {
                    scope_fns
                        .get(name)
                        .map(|v| v.iter().map(|e| e.decl).collect())
                        .unwrap_or_default()
                };
                let mut env = crate::lends::LendsEnv {
                    structs: &self.scope.structs,
                    fns: &lookup,
                    memo: &mut self.lends_memo,
                };
                let inferred: Option<Vec<usize>> = crate::lends::infer_from_body(f, body, &mut env)
                    .map(|set| {
                        let mut v: Vec<usize> = set.into_iter().collect();
                        v.sort_unstable();
                        v
                    });
                if let Some(inferred) = inferred {
                    // Declared must cover inferred: a lend the body visibly
                    // performs cannot go unwritten. The reverse is allowed —
                    // a generic body (`add(out, x)` with `x` an element of an
                    // opaque pass) lends through opacity the analysis cannot
                    // see, and the written entry is how it says so.
                    if inferred.iter().any(|i| !declared.contains(i)) {
                        let show = |v: &[usize]| -> String {
                            let names: Vec<String> = v
                                .iter()
                                .filter_map(|&i| f.params.get(i).map(|p| p.name.name.clone()))
                                .collect();
                            if names.is_empty() {
                                "nothing".to_string()
                            } else {
                                format!("`{}`", names.join("`, `"))
                            }
                        };
                        let span = f
                            .deductions
                            .as_ref()
                            .and_then(|l| {
                                l.iter()
                                    .find(|d| d.proj_sources().is_some())
                                    .map(|d| d.span)
                            })
                            .unwrap_or(f.name.span);
                        self.error(
                            span,
                            format!(
                                "the deduction list says the result projects {}, but the \
                                 body returns a value that projects {}: every lend the body \
                                 performs must be written (or omit the `proj` entries and \
                                 let them be inferred)",
                                show(&declared),
                                show(&inferred)
                            ),
                        );
                    }
                }
            }
            if let Some(list) = &f.deductions {
                // A parameter both consumed (`!p`) and named as a projection
                // source: a borrow needs the caller to keep the value.
                let moved: Vec<String> = list
                    .iter()
                    .filter(|d| matches!(d.kind, ast::DeductionKind::Moved))
                    .filter_map(|d| d.param_name().map(|n| n.name.clone()))
                    .collect();
                for d in list.iter() {
                    if let Some(sources) = d.proj_sources() {
                        for src in sources.iter().filter(|s| moved.contains(&s.name)) {
                            self.error(
                                src.span,
                                format!(
                                    "`{}` cannot be both consumed (`!{}`) and projected by the \
                                     result: a borrow needs the caller to keep the value",
                                    src.name, src.name
                                ),
                            );
                        }
                    }
                }
            }
        }
        self.own_proj_arms = f
            .return_type
            .as_ref()
            .map(proj_arm_qualifiers)
            .unwrap_or_default();
        // [proj-anywhere] Every *wholesale* `proj` in the return type — on the
        // result, an arm, a tuple element — must say what it borrows from:
        // without `[from: p]` there is nothing to link the result to, and the
        // caller could not know which argument it depends on. A `proj` inside
        // a type argument (`List<proj T>`) is a borrow the result *holds*:
        // it names no source, the lend does [proj-infer]. Parameters may write
        // a bare `proj` (it names the *kind* of value expected, not a source).
        if let Some(rt) = &f.return_type {
            let held: Vec<Span> = proj_refs_in_type_args(rt).iter().map(|r| r.span).collect();
            for r in proj_refs(rt) {
                let is_held = held.contains(&r.span);
                if is_held {
                    if let Some(from) = r.from.first() {
                        self.error(
                            from.span,
                            "a `proj` element names no source: the list holds the borrow, \
                             and which parameter it is of is inferred from the body (or \
                             written as `=> proj[from: p]` in the deduction clause)"
                                .to_string(),
                        );
                    }
                    continue;
                }
                if r.from.is_empty() {
                    self.error(
                        r.span,
                        "`proj` in a return type must name its source: `proj[from: param]`"
                            .to_string(),
                    );
                }
                for from in &r.from {
                    let is_param = f.params.iter().any(|p| p.name.name == from.name)
                        || extra_params.iter().any(|p| p.name.name == from.name);
                    if !is_param {
                        self.error(
                            from.span,
                            format!(
                                "`proj[from: {}]` names no parameter of this function",
                                from.name
                            ),
                        );
                    }
                }
            }
        }
        if let Some(id) = &f.derived_return {
            let is_param = f.params.iter().any(|p| p.name.name == id.name)
                || extra_params.iter().any(|p| p.name.name == id.name);
            // A source that names no parameter was reported by the
            // per-occurrence loop above; only the kept rule is left here.
            if is_param && self.inferred.is_some() {
                let kept = match &f.deductions {
                    Some(list) => crate::deduce::from_written(f, list, &HashSet::new(), |_, _| {})
                        .iter()
                        .find(|d| d.param == id.name)
                        .is_some_and(|d| d.kept),
                    None => self
                        .own_contract
                        .as_ref()
                        .and_then(|c| c.iter().find(|d| d.param == id.name))
                        .is_some_and(|d| d.kept),
                };
                if !kept {
                    self.error(
                        id.span,
                        format!(
                            "`proj[from: {}]` requires `{}` to be kept: a \
                             moved parameter is owned by this function, so its \
                             data is returned by ordinary moves",
                            id.name, id.name
                        ),
                    );
                }
            }
        }
        // [linear-generics] Type-parameter opt-ins: `<T canbe linear>`
        // treats `T`-typed values as linear in this body and admits
        // linear instantiation at call sites. Only `Linear` is
        // supported in a type-parameter `with` clause.
        self.own_linear_generics = f
            .generic_canbe
            .iter()
            .filter(|(_, q)| q.name.name == "linear")
            .map(|(id, _)| id.name.clone())
            .collect();
        for (_, q) in &f.generic_canbe {
            if q.name.name != "linear" {
                self.error(
                    q.span,
                    format!(
                        "only `linear` is supported in a type-parameter `with` \
                         clause (found `{}`)",
                        q.name.name
                    ),
                );
            }
        }
        // Validate the declared effect list (unknown effects, duplicates)
        // and build the fn's effect environment.
        let (mut fn_effects, can_use, mut can_spawn, mut can_wait) = self.check_effect_list(f);
        // [actor-spawn-effect] A handler member inherits its handler's
        // dependency list, and `[spawn]` is part of that list — a supervisor
        // spawns its children from a member body.
        can_spawn = can_spawn || self.handler_spawns;
        // [waitfor-effect] So is `[waitfor]`: a handler that declares it may
        // block from any member, and the *placement* check at its spawn site
        // is what makes that safe [waitfor-dedicated].
        can_wait = can_wait || self.handler_waits;
        // [throw-not-main] The entry point has nowhere to throw *to*: Rust
        // cannot express a `main` returning `ControlFlow` and Kotlin would
        // die on an uncaught signal, so the delimiter must be inside.
        if f.name.name == "main" && self.own_fn.is_some() {
            if let Some(span) = f
                .effects
                .iter()
                .flatten()
                .filter_map(|e| match e {
                    EffectRef::Effect(r) if r.name.name == THROW_EFFECT => Some(r.span),
                    _ => None,
                })
                .next()
            {
                self.error(
                    span,
                    format!(
                        "`main` cannot declare `{THROW_EFFECT}`: there is no caller to \
                         receive the throw — delimit it inside `main` with a \
                         `try {{ ... }}` block instead"
                    ),
                );
            }
        }
        // [effect-handler-deps] A handler member body may use the effects
        // its handler declares in its effect list, exactly as if the member
        // had declared them — which it may not
        // ([effect-member-no-effects]): the dependency belongs to the
        // implementation, so it is declared once on the handler.
        for dep in &self.handler_deps {
            if !fn_effects.contains(dep) {
                fn_effects.push(dep.clone());
            }
        }
        if let Some(key) = self.own_fn {
            self.out.fn_effects.insert(key, fn_effects.clone());
        }
        let Some(body) = &f.body else {
            self.generics = saved_generics;
            return;
        };
        let saved_env = std::mem::replace(&mut self.effect_env, fn_effects);
        let saved_can_use = std::mem::replace(&mut self.can_use, can_use);
        let saved_can_spawn = std::mem::replace(&mut self.can_spawn, can_spawn);
        // [waitfor-effect] `waitfor` is legal wherever the capability was
        // declared. `main` is no longer special — it was only ever special by
        // being a dedicated thread, and that is now what the rule says.
        let saved_can_wait = std::mem::replace(&mut self.can_wait, can_wait);
        // [implicit-forward] What this body can forward to the calls it makes.
        let saved_implicits = std::mem::replace(&mut self.own_implicits, implicits.clone());
        // Inside a qualifier constructor (`-> T as Qual`) return points
        // produce plain `T` values; the qualifier is applied by construction
        // (callers see `Qual T`).
        self.ret_ty = f
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);

        let mut top = HashMap::new();
        for p in extra_params.iter().chain(&f.params) {
            let ty = self.lower_type(&p.ty);
            // Parameter names hover like the variables they are
            // [doc-hover-narrowed].
            self.out
                .expr_ty
                .entry(self.key(p.name.span))
                .or_insert_with(|| ty.clone());
            let id = self.next_var_id;
            self.next_var_id += 1;
            top.insert(
                p.name.name.clone(),
                LocalVar {
                    declared: ty.clone(),
                    narrowed: ty,
                    id,
                    links: Vec::new(),
                    poison: None,
                    consumed_by: None,
                linear_settled: false,
                    is_param: true,
                    for_origin: None,
                    decl_span: p.name.span,
                    lambda_kept: false,
                    is_handler_state: false,
                    widened: None,
                    place_narrows: Vec::new(),
                    moved_places: Vec::new(),
                    used: false,
                },
            );
        }
        // [implicit-group] A group's members are not in `f.params`, so they
        // are declared here. Kept, never consumable: an implicit belongs to
        // whoever supplied it, exactly like a kept fn-typed parameter.
        for imp in &implicits {
            if top.contains_key(&imp.name) {
                continue; // a written `?name: FnType` is already a param
            }
            let id = self.next_var_id;
            self.next_var_id += 1;
            top.insert(
                imp.name.clone(),
                LocalVar {
                    declared: imp.ty.clone(),
                    narrowed: imp.ty.clone(),
                    id,
                    links: Vec::new(),
                    poison: None,
                    consumed_by: None,
                linear_settled: false,
                    is_param: true,
                    for_origin: None,
                    decl_span: imp.span,
                    lambda_kept: true,
                    is_handler_state: false,
                    widened: None,
                    place_narrows: Vec::new(),
                    moved_places: Vec::new(),
                    used: false,
                },
            );
        }
        for field in state {
            let ty = self.lower_type(&field.ty);
            self.out
                .expr_ty
                .entry(self.key(field.name.span))
                .or_insert_with(|| ty.clone());
            let id = self.next_var_id;
            self.next_var_id += 1;
            top.insert(
                field.name.name.clone(),
                LocalVar {
                    declared: ty.clone(),
                    narrowed: ty,
                    id,
                    links: Vec::new(),
                    poison: None,
                    consumed_by: None,
                linear_settled: false,
                    is_param: true,
                    for_origin: None,
                    decl_span: field.name.span,
                    lambda_kept: false,
                    is_handler_state: true,
                    widened: None,
                    place_narrows: Vec::new(),
                    moved_places: Vec::new(),
                    used: false,
                },
            );
        }
        self.locals.push(top);
        self.check_block_value(body);
        // [linear-obligation] A moved-in linear parameter must be
        // discharged by the body.
        self.check_linear_frame_drop();
        // [linear-state] …and a handler member must leave its state whole.
        self.check_state_whole(f.name.span, "end this member");
        self.locals.pop();
        // [fn-must-return] A fn with a non-`None` return type must return
        // on every path. Yield-based iterator fns are exempt: their body
        // produces elements, not a return value.
        if !self.ret_ty.is_none_ty()
            && !matches!(self.ret_ty, Ty::Unknown)
            && !self.block_returns(body)
        {
            self.error(
                f.name.span,
                format!(
                    "missing return: not all paths in `{}` return a value \
                     (declared return type is `{}`)",
                    f.name.name, self.ret_ty
                ),
            );
        }
        self.effect_env = saved_env;
        self.can_use = saved_can_use;
        self.can_spawn = saved_can_spawn;
        self.can_wait = saved_can_wait;
        self.own_implicits = saved_implicits;
        self.generics = saved_generics;
    }

    // ================= effects =================

    /// Validates a fn's declared effect list and lowers it into the
    /// starting effect environment [effect-fn-deps] [effect-no-dup].
    /// Returns `(env, can_use, can_spawn, can_wait)`.
    fn check_effect_list(&mut self, f: &'p FnDecl) -> (Vec<Ty>, bool, bool, bool) {
        let mut env: Vec<Ty> = Vec::new();
        let mut can_use = false;
        let mut can_spawn = false;
        let mut can_wait = false;
        // [fn-effects] A fn-typed parameter's effects are the enclosing fn's
        // too (user decision 2026-09-04): the only reason to take `f` is to
        // call it, and calling it needs those effects here — so they are
        // *inherited* rather than repeated in the written list. Callers
        // supply them like any declared effect.
        for p in &f.params {
            for ty in self.inherited_fn_effects(&p.ty) {
                if !env.contains(&ty) {
                    env.push(ty);
                }
            }
        }
        let mut written: Vec<Ty> = Vec::new();
        for eff in f.effects.iter().flatten() {
            match eff {
                EffectRef::Use(_) => can_use = true,
                // [actor-spawn-effect] The actor-creation capability. It
                // names no effect type, so there is nothing to lower and
                // nothing to thread; what it does is open the *gate* a
                // `spawn` expression checks [actor-spawn-expr].
                EffectRef::Spawn(_) => can_spawn = true,
                // [waitfor-effect] The right to occupy this thread until an
                // answer arrives. Like `spawn` it names no effect type, so
                // there is nothing to thread; what it opens is the `waitfor`
                // expression, and what validates it is the *placement* of
                // whatever carries it [waitfor-dedicated].
                EffectRef::WaitFor(_) => can_wait = true,
                EffectRef::Effect(r) => {
                    let Some(ty) = self.lower_effect_ref(r) else {
                        continue;
                    };
                    if written.contains(&ty) {
                        self.error(
                            r.span,
                            format!(
                                "duplicate effect `{ty}` in the effect list (two effects \
                                 of the same type must differ in their generic arguments)"
                            ),
                        );
                    } else {
                        written.push(ty.clone());
                        if !env.contains(&ty) {
                            env.push(ty);
                        }
                    }
                }
            }
        }
        (env, can_use, can_spawn, can_wait)
    }

    /// [actor-mailbox] The `mailbox { capacity: … }` slot on a handler (user
    /// decision 2026-09-16). Three rules, and each one is why the slot is
    /// better placed here than at every spawn:
    ///
    /// * **required** on a handler of an `actor effect` — the queue depth has
    ///   no default, exactly as it had none at the spawn site, but now it is
    ///   stated once by the author who knows the protocol's traffic;
    /// * **refused** on a handler of a plain effect, where there is no queue to
    ///   bound (a synchronous handler's members run on the caller's thread);
    /// * its field expressions see **constructor parameters only**. The value
    ///   is wanted before the actor exists — the scheduler needs the bound at
    ///   spawn time, ahead of any state initialiser — so state fields are not
    ///   in scope, and an effect cannot be performed to compute it. Consuming
    ///   one of those parameters is refused by [effect-state-store] already
    ///   (the handler stores what it was built with), so there is no rule here
    ///   for it.
    ///
    /// The braces are a `Mailbox` struct literal with the type elided, so the
    /// field names, their types, a missing one and an unknown one are all the
    /// ordinary struct diagnostics.
    fn check_mailbox_slot(&mut self, h: &'p ast::HandlerDecl, of: &[Ty]) {
        // [effect-handler-multi] One mailbox serves every face: the effects a
        // handler implements are all of one kind, so *any* of them answers
        // whether there is a queue at all.
        let is_actor = of.iter().any(|of| match of.strip_quals() {
            Ty::Named { name, .. } => self
                .scope
                .effects
                .get(name.as_str())
                .is_some_and(|e| e.is_actor),
            _ => false,
        });
        // [mixed-handler] A mixed handler's servant has a mailbox too: its
        // handler-local `send fn` members are delivered through one, so the
        // requirement keys on "has send members" as much as on the faces'
        // kind.
        let has_queue = is_actor || h.fns.iter().any(|f| f.is_send);
        match (&h.mailbox, has_queue) {
            (None, true) => {
                self.error(
                    h.name.span,
                    format!(
                        "`{}` has a mailbox — {} — and has to \
                         say how deep it is: add `mailbox {{ capacity: 16 }}` (or take \
                         it as a constructor parameter — `mailbox {{ capacity: \
                         capacity }}`). There is no default, because a queue bound \
                         chosen by the compiler is a performance cliff nobody wrote",
                        h.name.name,
                        if is_actor {
                            "it handles an actor effect"
                        } else {
                            "its `send fn` members are delivered through one"
                        }
                    ),
                );
            }
            (Some(mailbox), false) => {
                self.error(
                    mailbox.span(),
                    format!(
                        "only a handler with something to enqueue has a mailbox, and \
                         `{}` has no actor face and no `send fn` member: its members \
                         run on the caller's thread, so there is no queue to bound",
                        h.name.name
                    ),
                );
            }
            (Some(mailbox), true) => {
                // Constructor parameters only: the bound is computed when the
                // handler is built, before any state exists.
                let mut top: HashMap<String, LocalVar> = HashMap::new();
                for p in &h.params {
                    let ty = self.lower_type(&p.ty);
                    let id = self.next_var_id;
                    self.next_var_id += 1;
                    top.insert(
                        p.name.name.clone(),
                        LocalVar {
                            declared: ty.clone(),
                            narrowed: ty,
                            id,
                            links: Vec::new(),
                            poison: None,
                            consumed_by: None,
                            linear_settled: false,
                            is_param: true,
                            for_origin: None,
                            decl_span: p.name.span,
                            lambda_kept: false,
                            is_handler_state: false,
                            widened: None,
                            place_narrows: Vec::new(),
                            moved_places: Vec::new(),
                            used: true,
                        },
                    );
                }
                self.locals.push(top);
                let expected = Ty::named(MAILBOX_TYPE);
                let got = self.check_expr(mailbox, Some(&expected));
                if !got.is_unknown() && !is_subtype(&got, &expected) {
                    self.error(
                        mailbox.span(),
                        format!("a `mailbox` slot builds a `{expected}`, found `{got}`"),
                    );
                }
                self.locals.pop();
                // Consuming a constructor parameter here needs no rule of its
                // own: [effect-state-store] already refuses it, because the
                // handler stores what it was built with — and its message names
                // `copy`, which is the remedy here too.
            }
            (None, false) => {}
        }
    }

    /// Lowers one named entry of an effect list, validating that it refers
    /// to a declared effect with the right number of type arguments.
    fn lower_effect_ref(&mut self, r: &TypeRef) -> Option<Ty> {
        let name = r.name.name.as_str();
        let Some(effect) = self.scope.effects.get(name).copied() else {
            self.error_unresolved(r.span, format!("unknown effect `{name}`"), name);
            return None;
        };
        if r.args.len() != effect.generics.len() {
            self.error(
                r.span,
                format!(
                    "effect `{name}` expects {} type argument(s), found {}",
                    effect.generics.len(),
                    r.args.len()
                ),
            );
        }
        let empty = HashMap::new();
        Some(self.lower_base_ref(r, &empty, 0))
    }

    /// Rejects declared effect dependencies on effect/handler member fns
    /// [effect-member-no-effects]: dispatch call sites go through the
    /// handler instance and cannot thread extra handler arguments.
    fn reject_member_effects(&mut self, f: &'p FnDecl, what: &str) {
        for eff in f.effects.iter().flatten() {
            let span = match eff {
                EffectRef::Use(s) => *s,
                EffectRef::Spawn(s) => *s,
                // [waitfor-effect] The one exception (user decision
                // 2026-09-17, with T-5): a member may declare `[waitfor]`,
                // because "this member may occupy your thread" is a fact
                // about the *protocol* that its callers have to reckon with —
                // there is nothing to thread, so the objection above does not
                // apply.
                EffectRef::WaitFor(_) => continue,
                EffectRef::Effect(r) => r.span,
            };
            self.error(
                span,
                format!("{what} cannot declare effect dependencies yet"),
            );
        }
    }

    /// Checks a `use Handler(...)` statement [effect-use]: resolves the
    /// handler, types its constructor arguments (inferring the handler's
    /// generics from them), and registers the concrete effect instance in
    /// the current scope. A registration may **shadow** an earlier one for
    /// the same instance — innermost wins [use-no-dup] — which is what makes
    /// interception writable [effect-intercept].
    fn check_use(&mut self, handler: &'p Expr, span: Span) {
        // [actor-use-addr] `use addr` binds an effect to a forwarding stub over
        // an `Addr` instead of constructing a handler: first-pass surface, and
        // no new syntax — which means telling the two apart is this
        // statement's job.
        if let Expr::Ident(id) = handler {
            if !self.scope.handlers.contains_key(id.name.as_str()) && self.lookup(&id.name).is_some()
            {
                self.check_use_addr(handler, span);
                return;
            }
        }
        let Some((id, args, written_type_args)) = self.handler_construction(handler, "use") else {
            return;
        };
        let Some((concrete, deps)) =
            self.check_handler_construction(id, args, written_type_args, "use", span)
        else {
            return;
        };
        // [actor-replyto] A handler whose members mint a self-targeted
        // continuation may only be **spawned** (user decision 2026-09-15).
        // Bound with `use`, its member bodies run inline on the caller's
        // thread: the mint would target a member of a *local* instance, which
        // has no mailbox for the answer to arrive on and no dispatcher to run
        // it — so the continuation would silently never run. Parking is the
        // one thing only an actor can do, so this costs nothing real.
        //
        // The gate is syntactic per *handler* rather than per member, because
        // effects propagate: a fn declaring `[E]` may call any member, so a
        // `use` site cannot know which ones this scope will reach.
        if self.parking_handlers.contains(id.name.as_str()) {
            self.error(
                span,
                format!(
                    "handler `{}` mints a continuation with `replyto`, so it can only \
                     be `spawn`ed: bound with `use` its members run inline, and the \
                     answer would have nowhere to arrive — write `spawn {}(...) on \
                     pool(1)` instead",
                    id.name, id.name
                ),
            );
        }
        // [mixed-handler] The same reasoning, structural: a mixed handler's
        // sync members send to the servant and wait for its answers, so
        // bound with `use` they would send toward an instance with no
        // mailbox and no dispatcher. Spawn-only, refused where the binding
        // is chosen.
        if self
            .scope
            .handlers
            .get(id.name.as_str())
            .copied()
            .is_some_and(|h| self.handler_is_mixed(h))
        {
            self.error(
                span,
                format!(
                    "handler `{}` is mixed — its state is confined to `send fn` \
                     members, and its sync members send to that servant — so it can \
                     only be `spawn`ed: bound with `use` there would be no mailbox \
                     for the sends to arrive on. Write `let handle = spawn {}(...)` \
                     and `use handle`",
                    id.name, id.name
                ),
            );
        }
        // [waitfor-effect] Binding a handler that declares `[waitfor]` makes
        // *this* frame one that may be occupied until an answer arrives: the
        // capability flows outward from the binding, so the binder declares it
        // too — and in an actor's member that means the handler does, which is
        // what its own spawn site then pays for [waitfor-dedicated]. Effect
        // *callers* learn nothing, by the ordinary handler-dependency rule:
        // the hazard keys on the binding, which is where it is visible.
        if !self.can_wait
            && self
                .scope
                .handlers
                .get(id.name.as_str())
                .is_some_and(|h| {
                    h.effects
                        .iter()
                        .flatten()
                        .any(|e| matches!(e, EffectRef::WaitFor(_)))
                })
        {
            self.error(
                span,
                format!(
                    "handler `{}` declares `[waitfor]`, so a scope that binds it may \
                     be occupied until an answer arrives: this function must declare \
                     `[waitfor]` too (in a handler, that is its dependency list, and \
                     its spawn then needs `on thread()`)",
                    id.name
                ),
            );
        }
        self.finish_use(id, concrete, deps, span);
    }

    /// [actor-use-addr] `use addr` — bind the effect an actor serves in this
    /// scope, so its members are callable unqualified *and* can travel down
    /// through ordinary effect lists (`fn drive() [Roll]`). The addr is not
    /// consumed: an addr is freely copyable, and a send to a dead actor is a
    /// no-op, so binding one takes nothing away from the holder.
    fn check_use_addr(&mut self, addr: &'p Expr, span: Span) {
        let ty = self.check_expr(addr, None);
        let Some(effect) = addr_effect(&ty) else {
            if !ty.is_unknown() {
                self.error(
                    span,
                    format!(
                        "`use` registers a handler or binds an `{ADDR_TYPE}`, and `{ty}` is \
                         neither: write a handler construction (`use SomeHandler(...)`), \
                         or bind the addr a `spawn` answered"
                    ),
                );
            }
            return;
        };
        // [actor-effect-kind] Binding an addr binds an actor's protocol —
        // or, since SH-3 [monitor-handler], a shared monitor of a plain
        // effect: both kinds are handles a `use` puts in scope, and no call
        // site learns the difference.
        self.out.use_addrs.insert(self.key(span), effect.clone());
        self.out
            .use_effects
            .insert(self.key(span), vec![effect.clone()]);
        self.effect_env.push(effect);
    }

    /// The written shape of a handler construction — a name, or a name with
    /// constructor arguments — shared by `use` and `spawn`, since only those
    /// two may write one ([handler-not-value] means it is not an expression).
    fn handler_construction(
        &mut self,
        handler: &'p Expr,
        form: &str,
    ) -> Option<(&'p Ident, &'p [Expr], &'p [ast::Type])> {
        match handler {
            Expr::Ident(id) => Some((id, &[], &[])),
            Expr::Call {
                callee,
                args,
                type_args,
                ..
            } => match callee.as_ref() {
                Expr::Ident(id) => Some((id, args.as_slice(), type_args.as_slice())),
                _ => {
                    let span = handler.span();
                    self.error(
                        span,
                        format!("`{form}` expects a handler name or constructor call"),
                    );
                    self.check_expr(handler, None);
                    None
                }
            },
            _ => {
                let span = handler.span();
                self.error(
                    span,
                    format!("`{form}` expects a handler name or constructor call"),
                );
                self.check_expr(handler, None);
                None
            }
        }
    }

    /// Checks a handler *construction*: resolves the handler, types its
    /// constructor arguments (inferring its generics from them and from any
    /// written type arguments), consumes what it stores, and fills the
    /// constructor's implicits. Answers the concrete effect instance it
    /// implements — one per declared face [effect-handler-multi], in
    /// declaration order — and its **dependencies, already substituted**:
    /// everything both `use` and `spawn` need before they part ways over
    /// *where* those dependencies come from.
    fn check_handler_construction(
        &mut self,
        id: &'p Ident,
        args: &'p [Expr],
        written_type_args: &'p [ast::Type],
        form: &str,
        span: Span,
    ) -> Option<(Vec<Ty>, Vec<Ty>)> {
        let Some(decl) = self.scope.handlers.get(id.name.as_str()).copied() else {
            self.error_unresolved(
                id.span,
                format!("unknown handler `{}` in `{form}`", id.name),
                &id.name,
            );
            for a in args {
                self.check_expr(a, None);
            }
            return None;
        };
        let saved = self.enter_generics(&decl.generics);
        // [effect-handler-deps] Split the constructor parameters: those of
        // effect type are *dependencies* the compiler supplies from the
        // enclosing scope, and are not written at the `use` site; every
        // constructor parameter is therefore ordinary data.
        let deps: Vec<Ty> = self.handler_dep_effects(decl);
        let value_params: Vec<&Param> = decl.params.iter().filter(|p| !p.implicit).collect();
        let param_tys: Vec<Ty> = value_params
            .iter()
            .map(|p| self.lower_type(&p.ty))
            .collect();
        let of_tys: Vec<Ty> = decl.of.iter().map(|t| self.lower_type(t)).collect();
        self.generics = saved;
        // [lsp-definition] the handler name points at its declaration.
        self.record_def_ref(id.span, &id.name);

        let has_variadic = value_params.iter().any(|p| p.variadic);
        if !has_variadic && args.len() != value_params.len() {
            let dep_note = if deps.is_empty() {
                String::new()
            } else {
                format!(
                    " (its {} effect dependenc{} come from the enclosing scope)",
                    deps.len(),
                    if deps.len() == 1 { "y" } else { "ies" }
                )
            };
            self.error(
                span,
                format!(
                    "handler `{}` expects {} constructor argument(s), found {}{dep_note}",
                    id.name,
                    value_params.len(),
                    args.len()
                ),
            );
        }
        let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a, None)).collect();
        // A handler-constructor argument is stored in the handler for the
        // rest of the scope: passing a bare identifier moves it
        // [deduce-consume].
        for a in args {
            self.fate_move(a, "store", "a `use` handler registration", a.span());
        }
        let mut subst: HashMap<String, Ty> = HashMap::new();
        // [effect-handler-generics] A `use` may write the handler's type
        // arguments (`use Plain<Int>()`), and for a handler with no
        // constructor argument to infer from, that is the *only* thing that
        // can bind them — so they are read first, and a constructor argument
        // that disagrees is the error rather than the winner.
        if !written_type_args.is_empty() {
            if written_type_args.len() != decl.generics.len() {
                self.error(
                    id.span,
                    format!(
                        "handler `{}` takes {} type argument(s), found {}",
                        id.name,
                        decl.generics.len(),
                        written_type_args.len()
                    ),
                );
            }
            for (g, ta) in decl.generics.iter().zip(written_type_args) {
                let lowered = self.lower_type(ta);
                subst.insert(g.name.clone(), lowered);
            }
        }
        for (p, a) in param_tys.iter().zip(&arg_tys) {
            let mut from_args: HashMap<String, Ty> = HashMap::new();
            unify(p, a, &mut from_args);
            for (name, ty) in from_args {
                match subst.get(&name) {
                    Some(written) if !ty.is_unknown() && !is_subtype(&ty, written) => {
                        self.error(
                            span,
                            format!(
                                "handler `{}` was used at `{name} = {written}`, but an \
                                 argument makes it `{ty}`",
                                id.name
                            ),
                        );
                    }
                    Some(_) => {}
                    None => {
                        subst.insert(name, ty);
                    }
                }
            }
        }
        let generic_set: HashSet<String> = decl.generics.iter().map(|g| g.name.clone()).collect();
        // The handler's own type arguments, in declaration order, for the
        // emitters: a generic handler has to be *constructed* at a type
        // (`Plain<Int>()`, `Plain::<i32>::new()`), which neither target can
        // infer from an empty argument list.
        if !decl.generics.is_empty() {
            let args: Vec<Ty> = decl
                .generics
                .iter()
                .map(|g| subst.get(&g.name).cloned().unwrap_or(Ty::Unknown))
                .collect();
            self.out.use_handler_args.insert(self.key(span), args);
        }
        // Record argument coercions against the substituted param types.
        for (i, p) in param_tys.iter().enumerate().take(arg_tys.len()) {
            let sp = substitute_vars(p, &subst, &generic_set);
            let logical = arg_tys[i].clone();
            let repr = self.repr_of(&args[i], &logical);
            self.maybe_coerce(args[i].span(), &logical, &repr, &sp);
        }
        // [copy-implicit] The constructor's implicit parameters are filled
        // here, at the handler's now-known type arguments — recorded under
        // the `use` span, where the emitters build the instance.
        let ctor_implicits: Vec<ImplicitParam> = {
            let saved = self.enter_generics(&decl.generics);
            let list = decl
                .params
                .iter()
                .filter(|p| p.implicit)
                .map(|p| ImplicitParam {
                    name: p.name.name.clone(),
                    ty: self.lower_type(&p.ty),
                    span: p.span,
                    borrowed_arms: Vec::new(),
                })
                .collect();
            self.generics = saved;
            list
        };
        if !ctor_implicits.is_empty() {
            self.fill_implicits(&ctor_implicits, &id.name, &subst, &generic_set, &[], span);
        }
        let concrete: Vec<Ty> = of_tys
            .iter()
            .map(|of| substitute_vars(of, &subst, &generic_set))
            .collect();
        let deps: Vec<Ty> = deps
            .iter()
            .map(|d| substitute_vars(d, &subst, &generic_set))
            .collect();
        Some((concrete, deps))
    }

    /// The `use`-specific half: resolve the handler's dependencies from the
    /// **enclosing scope** and register the instance for the rest of it.
    fn finish_use(&mut self, id: &'p Ident, concrete: Vec<Ty>, deps: Vec<Ty>, span: Span) {
        // [use-no-dup] [effect-intercept] A `use` may *shadow* an earlier
        // registration for the same effect instance: the innermost wins for
        // the rest of the scope, which is what makes interception writable
        // (`use DefaultFs()` then `use RestrictedFs(root)`). What stays an
        // error is registering the same handler *instance* twice — and under
        // [handler-not-value] an instance exists only at its `use`, so that
        // clause is future-proofing rather than a check.
        // [effect-handler-deps] Every dependency must already have a handler
        // here: the registration is what wires them together, so this is the
        // point where "who provides it" is decided. Resolved instances are
        // recorded for the emitters, in declaration order. The lookup runs
        // *before* this `use` is registered, which is exactly the
        // binds-outward rule for a self-dependency [effect-intercept].
        let mut resolved_deps: Vec<Ty> = Vec::new();
        let visible = self.visible_effects();
        for want in &deps {
            let want = want.clone();
            let found = visible.iter().find(|c| **c == want).cloned().or_else(|| {
                let compatible: Vec<&Ty> = visible
                    .iter()
                    .filter(|c| unify(&want, c, &mut HashMap::new()))
                    .collect();
                match compatible.len() {
                    1 => Some(compatible[0].clone()),
                    _ => None,
                }
            });
            match found {
                Some(instance) => resolved_deps.push(instance),
                // [effect-intercept] A self-dependency binds *outward*, so
                // "nothing in scope" means there is nothing to intercept —
                // worth its own wording, since the remedy is not "register a
                // different handler first" but "register the one you are
                // wrapping".
                None if concrete.contains(&want) => self.error(
                    span,
                    format!(
                        "handler `{}` intercepts `{want}` — it depends on the effect \
                         it implements — but no handler for `{want}` is registered \
                         before this `use`: an intercepting handler wraps the \
                         instance already in scope",
                        id.name
                    ),
                ),
                None => self.error(
                    span,
                    format!(
                        "handler `{}` depends on effect `{want}`, which has no \
                         handler in scope here: register one before it",
                        id.name
                    ),
                ),
            }
        }
        if !resolved_deps.is_empty() {
            self.out.use_deps.insert(self.key(span), resolved_deps);
        }
        // [effect-handler-multi] Every face is registered, in declaration
        // order: one instance, one binding per effect it implements, so a
        // caller reaches whichever face its own effect list names.
        self.out
            .use_effects
            .insert(self.key(span), concrete.clone());
        for effect in concrete {
            self.effect_env.push(effect);
        }
    }

    // ================= scopes, locals, narrowing =================

    /// [use-no-dup] [effect-intercept] The effect instances visible here,
    /// **innermost first, shadowed duplicates hidden**. `effect_env` is a
    /// stack that a `use` pushes onto and a block truncates
    /// ([effect-scope]); since a `use` may shadow an earlier registration
    /// of the same instance, every lookup has to read it as a scope rather
    /// than as a set — otherwise the *outer* handler would answer a member
    /// call the inner one shadowed, and an intercepting handler would
    /// silently never run.
    fn visible_effects(&self) -> Vec<Ty> {
        let mut out: Vec<Ty> = Vec::new();
        for ty in self.effect_env.iter().rev() {
            if !out.contains(ty) {
                out.push(ty.clone());
            }
        }
        out
    }

    fn enter_generics(&mut self, generics: &[Ident]) -> HashSet<String> {
        let saved = self.generics.clone();
        for g in generics {
            self.generics.insert(g.name.clone());
        }
        saved
    }

    fn lookup(&self, name: &str) -> Option<&LocalVar> {
        self.locals.iter().rev().find_map(|s| s.get(name))
    }

    /// Records a non-fn name use as pointing at its declaration's
    /// identifier [lsp-definition]. Silent when the name resolves to
    /// nothing visible (locals, generics, backend interop).
    fn record_def_ref(&mut self, span: Span, name: &str) {
        if let Some(site) = self.scope.def_sites.get(name).copied() {
            self.out.def_refs.insert(self.key(span), site);
        }
    }

    /// [op-no-none] Rejects a possibly-`None` operand of an arithmetic or
    /// comparison operator (user decision 2026-09-02). Nullability is
    /// tested with `is None`, so an optional reaching an operator is
    /// always a missing narrowing — and the backends disagree about it
    /// (Kotlin compares/prints `null`, Rust rejects the `Option`), which
    /// makes leniency a parity hole. `Unknown`/`Nothing` stay lenient.
    fn reject_optional_operand(&mut self, op: BinaryOp, operand: &Expr, ty: &Ty) {
        if ty.is_unknown() || matches!(ty, Ty::Nothing) {
            return;
        }
        let stripped = ty.strip_quals();
        if !(stripped.has_none_arm() || stripped.is_none_ty()) {
            return;
        }
        let symbol = op_symbol(op);
        let message = if stripped.is_none_ty() {
            format!("`None` is not a valid operand of `{symbol}`")
        } else {
            format!(
                "operand of `{symbol}` may be `None` (its type is `{ty}`): narrow \
                 it first (`is` / `when`) or assert it with `!`"
            )
        };
        self.error(operand.span(), message);
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut LocalVar> {
        self.locals.iter_mut().rev().find_map(|s| s.get_mut(name))
    }

    /// Declares a new local, enforcing the no-shadowing rule
    /// [var-no-shadow].
    fn declare(&mut self, name: &Ident, ty: Ty) {
        self.declare_with_links(name, ty, Vec::new());
    }

    /// Declares a new local carrying fate links to the variables it was
    /// bound from [fate-link].
    fn declare_with_links(&mut self, name: &Ident, ty: Ty, links: Vec<FateLink>) {
        self.declare_var(name, ty, links, false, None);
    }

    fn declare_var(
        &mut self,
        name: &Ident,
        ty: Ty,
        links: Vec<FateLink>,
        is_param: bool,
        for_origin: Option<Span>,
    ) {
        // A declared *name* is not an expression, but tooling hovers it
        // like one: record its type so hover answers on the declaration
        // (parameter, `let`, `is`/`for` binding) and not only on uses
        // [doc-hover-narrowed]. `or_insert` keeps a more specific entry
        // an earlier pass recorded for the same span.
        self.out
            .expr_ty
            .entry(self.key(name.span))
            .or_insert_with(|| ty.clone());
        if self.lookup(&name.name).is_some() {
            self.error(
                name.span,
                format!(
                    "`{}` is already declared (shadowing is not allowed)",
                    name.name
                ),
            );
        }
        // [fate-move-mode] A move-mode binding takes ownership: its
        // ancestors are consumed here and the binding carries no links.
        let links = self.apply_binding_mode(&name.name, links);
        // [fate-link] Tooling hovers a *declaration* too, and that is where
        // a reader looks first to learn what a variable shares fate with —
        // so the presentation table is recorded here as well as at each
        // read. Recorded after the move-mode decision, which is what makes
        // the answer honest: a binding that took ownership has no links,
        // and its hover says so by carrying none.
        if !links.is_empty() {
            let reads: Vec<FateRead> = links
                .iter()
                .map(|l| FateRead {
                    root: l.root_name.clone(),
                    bind_span: l.bind_span,
                    path: l
                        .path
                        .as_ref()
                        .map(|p| p.iter().map(|s| s.to_string()).collect()),
                })
                .collect();
            self.out.fate_reads.insert(self.key(name.span), reads);
        }
        let id = self.next_var_id;
        self.next_var_id += 1;
        self.locals.last_mut().expect("scope stack").insert(
            name.name.clone(),
            LocalVar {
                declared: ty.clone(),
                narrowed: ty,
                id,
                links,
                poison: None,
                consumed_by: None,
                linear_settled: false,
                is_param,
                for_origin,
                decl_span: name.span,
                lambda_kept: false,
                is_handler_state: false,
                widened: None,
                place_narrows: Vec::new(),
                moved_places: Vec::new(),
                used: false,
            },
        );
    }

    // ================= shared fate [fate-link] =================

    /// The local variables an expression's value derives from: a bare
    /// identifier or a projection chain (field/index/`!`) of one
    /// [fate-link]. Anything else (calls — including `copy` [copy-fn] —
    /// literals, constructed values, branch values) produces an
    /// independent value.
    fn provenance<'e>(expr: &'e Expr, out: &mut Vec<&'e Ident>) {
        match expr {
            Expr::Ident(id) => out.push(id),
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => Self::provenance(base, out),
            Expr::Index { base, .. } => Self::provenance(base, out),
            Expr::NonNull { operand, .. } => Self::provenance(operand, out),
            _ => {}
        }
    }

    /// The fate links a binding from `value` carries: each source
    /// variable plus its own links, flattened (transitive links point at
    /// the ultimate roots) [fate-link], deduplicated by root id. The
    /// direct source link carries this bind event's span; transitive
    /// links *keep their original bind spans*, so a variable's links
    /// describe the whole derivation chain (`longest` → `name` →
    /// `person` → `persons`, each at its own bind site) — which is what
    /// lets move-mode candidates cover every binding of a consuming
    /// chain even after intermediate variables die [fate-move-mode].
    fn links_for_value(&self, value: &Expr, bind_span: Span) -> Vec<FateLink> {
        // [proj-field] A struct literal with `proj` fields is derived
        // from the values stored in them: a pass minted over `list` shares
        // fate with `list`. Other fields are moved in and carry no link.
        if let Expr::StructLit { fields, span, .. } = value {
            if let Some(struct_ty) = self.out.ty_of(self.file_idx, *span) {
                let proj_fields = self.proj_fields_of(struct_ty);
                let holds = self.ty_holds_proj(struct_ty);
                if !proj_fields.is_empty() || holds {
                    let mut links: Vec<FateLink> = Vec::new();
                    let push = |l: FateLink, links: &mut Vec<FateLink>| {
                        if !links.iter().any(|e| e.root_id == l.root_id) {
                            links.push(l);
                        }
                    };
                    for f in fields {
                        match &f.kind {
                            StructLitFieldKind::Named { name, value } => {
                                if proj_fields.contains(&name.name) {
                                    // [proj-infer] A `proj` field projects
                                    // what is stored in it: the literal holds
                                    // a borrow of that value's roots.
                                    for mut l in self.links_for_value(value, bind_span) {
                                        l.borrowed = true;
                                        l.held = true;
                                        push(l, &mut links);
                                    }
                                } else {
                                    // An owned field storing a *view* makes
                                    // the literal a view of the same roots;
                                    // an owned field storing owned data (or
                                    // a plain alias, which is a move) adds
                                    // nothing.
                                    for l in self.links_for_value(value, bind_span) {
                                        if l.held {
                                            push(l, &mut links);
                                        }
                                    }
                                }
                            }
                            StructLitFieldKind::Spread(inner) => {
                                for l in self.links_for_value(inner, bind_span) {
                                    if l.held {
                                        push(l, &mut links);
                                    }
                                }
                            }
                        }
                    }
                    return links;
                }
            }
        }
        // [readonly-return] `get(xs, i)!` is still the borrow `get` handed
        // back: a `!` unwrap changes the type, not the provenance.
        if let Expr::NonNull { operand, .. } = value {
            if matches!(operand.as_ref(), Expr::Call { .. }) {
                return self.links_for_value(operand, bind_span);
            }
        }
        // [readonly-return] The result of a derived-return call borrows
        // the annotated argument: it carries that argument's links
        // (dot-notation receivers are argument 0).
        if let Expr::Call {
            callee, args, span, ..
        } = value
        {
            // [proj-infer] The result of a lending call holds borrows of the
            // lent arguments: an owned value (a pass) tied to them.
            if let Some(lent) = self.out.lending_calls.get(&(self.file_idx, *span)).cloned() {
                let mut links: Vec<FateLink> = Vec::new();
                for idx in lent {
                    let arg: Option<&Expr> = if let Expr::Field { base, .. } = callee.as_ref() {
                        if idx == 0 {
                            Some(base)
                        } else {
                            args.get(idx - 1)
                        }
                    } else {
                        args.get(idx)
                    };
                    let Some(arg) = arg else { continue };
                    for mut l in self.links_for_value(arg, bind_span) {
                        l.borrowed = true;
                        l.held = true;
                        if !links.iter().any(|e| e.root_id == l.root_id) {
                            links.push(l);
                        }
                    }
                }
                return links;
            }
            if let Some(sources) = self.out.derived_calls.get(&(self.file_idx, *span)).cloned() {
                // [proj-readonly] A wholesale projection: the result *is* the
                // argument's data — of every named source.
                let mut links: Vec<FateLink> = Vec::new();
                for idx in sources {
                    let arg: Option<&Expr> = if let Expr::Field { base, .. } = callee.as_ref() {
                        if idx == 0 {
                            Some(base)
                        } else {
                            args.get(idx - 1)
                        }
                    } else {
                        args.get(idx)
                    };
                    let Some(arg) = arg else { continue };
                    for mut l in self.links_for_value(arg, bind_span) {
                        l.borrowed = true;
                        l.held = false;
                        if !links.iter().any(|e| e.root_id == l.root_id) {
                            links.push(l);
                        }
                    }
                }
                return links;
            }
        }
        // [lambda-view] A lambda value is a *view* of its non-Copy read
        // captures [fate-lambda]: binding it carries those held links (the
        // direct ones restamped to this bind event).
        if let Expr::Lambda { span, .. } = value {
            return self
                .lambda_links
                .get(span)
                .map(|ls| {
                    ls.iter()
                        .map(|l| FateLink {
                            root_id: l.root_id,
                            root_name: l.root_name.clone(),
                            bind_span,
                            path: l.path.clone(),
                            borrowed: l.borrowed,
                            held: l.held,
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
        let mut sources = Vec::new();
        Self::provenance(value, &mut sources);
        // [fate-field-disjoint] The projection this value reads out of its
        // source, as a path: `p.name` gives `[.name]`, a bare `p` gives
        // `[]`. `None` when the chain is not a plain projection (a `!`
        // unwrap), which stays conservative everywhere below.
        let extra: Option<Vec<Step>> = Place::of_expr(value).map(|p| p.path);
        let mut links: Vec<FateLink> = Vec::new();
        let push = |link: FateLink, links: &mut Vec<FateLink>| {
            if !links.iter().any(|l| l.root_id == link.root_id) {
                links.push(link);
            }
        };
        for src in sources {
            let Some(var) = self.lookup(&src.name) else {
                continue;
            };
            // A source that is itself physically borrowed propagates the
            // flag to its direct link [readonly-return].
            let src_borrowed = var.links.iter().any(|l| l.borrowed);
            push(
                FateLink {
                    root_id: var.id,
                    root_name: src.name.clone(),
                    bind_span,
                    path: extra.clone(),
                    borrowed: src_borrowed,
                    held: false,
                },
                &mut links,
            );
            for l in var.links.clone() {
                // [fate-field-disjoint] A transitive link's path is
                // relative to the *ultimate* root, so this value's own
                // projection extends it: with `let p = q.inner` and
                // `let n = p.name`, `n`'s link to `q` is `[.inner, .name]`.
                let composed = match (&l.path, &extra) {
                    (Some(base), Some(more)) => {
                        let mut path = base.clone();
                        path.extend(more.iter().cloned());
                        Some(path)
                    }
                    _ => None,
                };
                push(
                    FateLink {
                        path: composed,
                        ..l
                    },
                    &mut links,
                );
            }
        }
        links
    }

    /// [fate-partial-move] Assigning a place makes it whole again: drops
    /// every moved-out record the assigned place *covers* (itself and
    /// anything under it). Assigning the whole variable (`[]`) covers
    /// everything, which is why plain reassignment revives a partially
    /// moved value completely.
    fn revive_moved_places(&mut self, place: &Place) {
        let path = place.path.clone();
        if let Some(var) = self.lookup_mut(&place.root) {
            var.moved_places.retain(|m| {
                let assigned = Place {
                    root: String::new(),
                    path: path.clone(),
                };
                let moved = Place {
                    root: String::new(),
                    path: m.path.clone(),
                };
                !assigned.is_prefix_of(&moved)
            });
        }
    }

    /// [fate-partial-move] Reports a read of a place that overlaps
    /// something already moved out of its root. A *disjoint* projection
    /// passes: after `eat(p.tags)`, `p.name` is still there. The whole
    /// value never passes — a variable missing a part cannot be handed on.
    ///
    /// Returns whether an error was reported, so callers can stop rather
    /// than cascade [type-unknown-lenient].
    fn check_moved_place(&mut self, place: &Place, span: Span) -> bool {
        if self.assign_target {
            return false;
        }
        let Some(var) = self.lookup(&place.root) else {
            return false;
        };
        // A wholly consumed variable is the other machinery's business
        // [deduce-consume]: one mistake, one diagnostic.
        if matches!(var.narrowed, Ty::Nothing) {
            return false;
        }
        let Some(hit) = var
            .moved_places
            .iter()
            .find(|m| m.blocks(&place.path))
            .cloned()
        else {
            return false;
        };
        let root = place.root.clone();
        let what = hit.describe();
        let message = if place.path.is_empty() {
            format!(
                "`{root}` cannot be used as a whole here: `{root}{what}` was moved \
                 out of it, so part of the value is gone; move the remaining parts \
                 individually, or `copy` at the site that moved `{root}{what}`"
            )
        } else {
            format!(
                "`{place}` cannot be used here: `{root}{what}` was moved out of \
                 `{root}`, and this reads the same data; `copy` at the site that \
                 moved it to keep this readable"
            )
        };
        self.error(span, message);
        true
    }

    /// [interp-to-str] Whether a type renders natively in an interpolation on
    /// *both* backends: the scalars and `Str`. Rust formats these with
    /// `Display` and Kotlin with `toString`, and the text agrees — which is
    /// the requirement, since one program must print the same thing on both
    /// [backend-parity].
    fn interp_native(ty: &Ty) -> bool {
        // A union renders natively when *every* arm does: the value is one of
        // them, and both backends reach the payload (Kotlin through the
        // wrapper's `.value`, Rust through the arm accessor). `None` arms are
        // already refused before this [interp-no-none].
        if let Ty::Union(arms) = ty.strip_quals() {
            return arms.iter().all(Self::interp_native);
        }
        matches!(
            ty.strip_quals(),
            Ty::Named { name, args }
                if args.is_empty()
                    && matches!(
                        name.as_str(),
                        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char"
                            | "Byte" | "Str"
                    )
        )
    }

    /// [interp-to-str] Checks that an interpolated value has a text form, and
    /// records the `to_str` to reach it by when it is not native.
    ///
    /// Resolution is the ordinary implicit machinery at this site: a `to_str`
    /// in scope whose parameter accepts this type and which returns `Str`.
    /// std provides one for `List<T>`; `Mut Str` never arrives here (a
    /// builder is converted first [str-drop-mut]).
    fn check_interpolable(&mut self, expr: &'p Expr, ty: &Ty) {
        if ty.is_unknown() || matches!(ty, Ty::Nothing) || Self::interp_native(ty) {
            return;
        }
        let want = Ty::Fn {
            params: vec![ty.clone()],
            ret: Box::new(Ty::named("Str")),
            contract: None,
            effects: Vec::new(),
        };
        match self.resolve_implicit_fn("to_str", &want) {
            Ok(key) => {
                self.out.interp_to_str.insert(self.key(expr.span()), key);
                return;
            }
            Err(_) => {}
        }
        // [interp-struct] No `to_str` of its own: a struct whose every field
        // renders natively is rendered field-wise (user decision 2026-09-11 —
        // "structs should interpolate by default when all their fields do").
        // An explicit `to_str` wins, which is why this is tried second.
        if let Some(name) = self.struct_interp_name(ty) {
            self.out.interp_struct.insert(self.key(expr.span()), name);
            return;
        }
        self.error(
            expr.span(),
            format!(
                "`{ty}` has no text form, so it cannot be interpolated: \
                 declare `fn to_str({ty}) -> Str` (or one that accepts it) \
                 and it will be used here"
            ),
        );
    }

    /// [interp-struct] The struct name to render field-wise, when `ty` is a
    /// struct every one of whose fields renders natively. Fields that need a
    /// `to_str` of their own are deliberately *not* followed: the derivation
    /// exists for the simple cases, and anything else is clearer as a
    /// hand-written `to_str` (which takes precedence anyway).
    fn struct_interp_name(&self, ty: &Ty) -> Option<String> {
        let Ty::Named { name, args } = ty.strip_quals() else {
            return None;
        };
        if !args.is_empty() {
            // A generic struct's field types need substituting before they
            // can be judged; out of scope for the derivation.
            return None;
        }
        let decl = self.scope.structs.get(name.as_str())?;
        if decl.fields.is_empty() {
            return None;
        }
        // Judged on the *written* type: a native name, unqualified and
        // without type arguments. Reading the AST keeps this a pure query,
        // and every native type is spelled directly anyway (an alias to one
        // simply does not qualify — the remedy is a `to_str`).
        let all_native = decl.fields.iter().all(|f| match &f.ty {
            ast::Type::Named { qualifiers, base } => {
                qualifiers.is_empty()
                    && base.args.is_empty()
                    && matches!(
                        base.name.name.as_str(),
                        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte" | "Str"
                    )
            }
            _ => false,
        });
        all_native.then(|| name.clone())
    }

    /// Poisons every live variable fate-linked to `root_id` [fate-poison]:
    /// the root was mutated, moved, or reassigned, so derived values may
    /// no longer exist. They narrow to `Nothing` (error at a later use,
    /// revival by reassignment — the standard possibly-consumed
    /// machinery [deduce-consume]).
    ///
    /// [fate-field-disjoint] `event_path` says *which projection* of the
    /// root the event hit: only links that overlap it are poisoned, so
    /// mutating `p.tags` leaves a value derived from `p.name` usable. An
    /// event on the whole variable (`[]`) is a prefix of every path and so
    /// poisons everything, as before; `None` is the conservative unknown.
    fn poison_derived(
        &mut self,
        root_id: u32,
        root_name: &str,
        event: FateEvent,
        span: Span,
        event_path: Option<&[Step]>,
    ) {
        for frame in &mut self.locals {
            for var in frame.values_mut() {
                if var.id == root_id {
                    continue;
                }
                let hit = var
                    .links
                    .iter()
                    .any(|l| l.root_id == root_id && l.overlaps_event(event_path));
                if hit {
                    var.narrowed = Ty::Nothing;
                    var.poison = Some(Poison {
                        root_name: root_name.to_string(),
                        event,
                        event_span: span,
                    });
                }
            }
        }
    }

    /// Errors on an ownership-requiring operation (move or `Mut` op) on a
    /// fate-linked (derived) variable [fate-derived-readonly]: derived
    /// values are read-only; `copy` makes an independent value.
    ///
    /// This is also the S2 mode-inference probe [fate-move-mode]: every
    /// move/mutation of a derived variable lands here, so the binding
    /// event (the links' shared bind span) is recorded as a move-mode
    /// candidate for the next round, and parameter roots are claimed as
    /// moved when the fn's contract is inferable (no written list).
    fn error_derived(&mut self, span: Span, action: &str, name: &str, links: &[FateLink]) {
        // [copy-scalar-free] Moving a derived *Copy scalar* is a read: an
        // `Int` taken out of a borrow is the value itself on both backends,
        // so nothing can observe the difference and no `copy` is owed.
        // Poison still applies to it (a mutated root can change the number);
        // only the *move* is free.
        let scalar = self
            .lookup(name)
            .map(|v| v.narrowed.clone())
            .is_some_and(|t| {
                !matches!(t, Ty::Union(_))
                    && Self::interp_native(&t)
                    && t.strip_quals() != &Ty::named("Str")
            });
        if scalar {
            return;
        }
        self.record_move_candidates(links);
        self.record_param_claims(links);
        let root = &links[0].root_name;
        // [proj-readonly] A wholesale projection is the root's own data seen
        // through `proj`: read-only whatever its `Mut` says, and `copy` is
        // the way to a value of one's own.
        if links.iter().any(|l| l.borrowed && !l.held) {
            let why = if action == "mutate" {
                " — a `proj` value never satisfies a `Mut` position"
            } else {
                ""
            };
            self.error(
                span,
                format!(
                    "cannot {action} `{name}`: it is a projection (`proj`) of `{root}`, \
                     which can only be read{why}; use `copy({name})` for a value of your own"
                ),
            );
            return;
        }
        self.error(
            span,
            format!(
                "cannot {action} `{name}`: it was bound from `{root}` and shares \
                 its fate, so it can only be read; use `copy` to make an \
                 independent value (e.g. `copy({name})`, or bind it with \
                 `copy(...)`)"
            ),
        );
    }

    /// Records the *whole chain* of bind events behind `links` as
    /// move-mode candidates [fate-move-mode]: the links themselves carry
    /// the latest bind span, and each live root's own links carry the
    /// bind spans further down the derivation chain (`persons` → `person`
    /// → `name` → `longest`). Making every binding in a consuming
    /// pipeline move-mode is what lets the emitter produce a zero-clone
    /// chain of real moves.
    fn record_move_candidates(&mut self, links: &[FateLink]) {
        let mut spans: Vec<Span> = Vec::new();
        let mut stack: Vec<u32> = Vec::new();
        for l in links {
            spans.push(l.bind_span);
            stack.push(l.root_id);
        }
        let mut seen: HashSet<u32> = HashSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(var) = self.var_by_id(id) {
                for l in &var.links {
                    spans.push(l.bind_span);
                    stack.push(l.root_id);
                }
            }
        }
        for span in spans {
            self.move_candidates.insert((self.file_idx, span));
        }
    }

    /// Claims every parameter root among `links` as moved by the current
    /// fn [fate-move-mode] — only for parameters whose contract is
    /// *inferable* (not written in the clause): a binding or projection that
    /// takes ownership of data reached through a parameter makes the fn
    /// demand ownership from its callers. Claims are seeded into deduction
    /// inference between the checking rounds.
    fn record_param_claims(&mut self, links: &[FateLink]) {
        let Some(key) = self.own_fn else { return };
        for l in links {
            if self.own_written.contains(&l.root_name) {
                continue;
            }
            let is_param = self.var_by_id(l.root_id).is_some_and(|var| var.is_param);
            if is_param {
                self.param_claims
                    .entry(key)
                    .or_default()
                    .insert(l.root_name.clone());
            }
        }
    }

    /// [deduce-syntax] Records that the enclosing fn invalidates one of
    /// its parameters (a mutation of the parameter or of data reached
    /// through it). Drives the "mutation forces an exhaustive deduction
    /// list" rule; unlike claims, it is recorded even when the list is
    /// written, because that is exactly what the validation needs.
    fn record_param_mutation(&mut self, name: &str) {
        let Some(key) = self.own_fn else { return };
        let is_param = self.lookup(name).is_some_and(|var| var.is_param);
        if is_param {
            self.param_mutations
                .entry(key)
                .or_default()
                .insert(name.to_string());
        }
    }

    fn var_by_id(&self, id: u32) -> Option<&LocalVar> {
        self.locals
            .iter()
            .rev()
            .find_map(|frame| frame.values().find(|v| v.id == id))
    }

    /// Whether a parameter is *owned* by the current fn: its effective
    /// deduction contract (written list, else the previous round's
    /// inferred facts including claims) moves it [fate-move-mode].
    fn param_owned(&self, name: &str) -> bool {
        self.own_contract
            .as_ref()
            .and_then(|c| c.iter().find(|d| d.param == name))
            .is_some_and(|d| !d.kept)
    }

    /// [fate-move-mode] S2: applies the binding mode at a link-creating
    /// bind event. Borrow-mode (the default) keeps the links — the
    /// binding shares fate with its ancestors. Move-mode fires when the
    /// bind event is a candidate (round one saw the bound variable moved
    /// or mutated) and every ancestor is owned in-function (a local, or
    /// a parameter the fn's effective contract moves): the binding takes
    /// ownership — the ancestors are consumed *here* (poison lands at
    /// the binding), the event is recorded for the emitter (real move,
    /// no clone), and the binding carries no links. A written-kept
    /// parameter ancestor keeps borrow-mode, so the S1 error re-derives
    /// at the move site [fate-derived-readonly].
    fn apply_binding_mode(&mut self, binding_name: &str, links: Vec<FateLink>) -> Vec<FateLink> {
        // Round one is strict S1: candidates are being collected.
        if self.inferred.is_none() || links.is_empty() {
            return links;
        }
        // [readonly-return] A value reached through a derived-return
        // call is *physically borrowed*: move-mode cannot take ownership
        // through it — the binding stays borrow-mode and the move site
        // reports the S1 error with the `copy` remedy.
        if links.iter().any(|l| l.borrowed) {
            return links;
        }
        let bind_span = links[0].bind_span;
        if !self.move_candidates.contains(&(self.file_idx, bind_span)) {
            return links;
        }
        // Every live ancestor must be owned; a dead root has nothing
        // left to consume and does not block the move.
        for l in &links {
            let Some(var) = self.var_by_id(l.root_id) else {
                continue;
            };
            if (var.is_param && !self.param_owned(&l.root_name)) || var.lambda_kept {
                return links;
            }
        }
        // A lambda cannot consume a capture [fate-lambda]: a move-mode
        // binding inside a lambda whose ancestors live outside it stays
        // borrow-mode, and the move site reports the violation.
        for l in &links {
            let Some(frame) = self.frame_of_id(l.root_id) else {
                continue;
            };
            if self.lambda_ctx.iter().any(|ctx| frame < ctx.boundary) {
                return links;
            }
        }
        self.record_param_claims(&links);
        for l in &links {
            // Other variables derived from this root lose their value
            // [fate-poison] — those overlapping *this* link's projection:
            // taking ownership of `p.tags` says nothing about `p.name`
            // [fate-field-disjoint].
            self.poison_derived(
                l.root_id,
                &l.root_name,
                FateEvent::Moved,
                bind_span,
                l.path.as_deref(),
            );
            let file_idx = self.file_idx;
            let mut loop_origin = None;
            // [fate-partial-move] A binding that takes ownership of a
            // *projection* leaves the rest of the root readable: record
            // what left rather than consuming the whole variable.
            let partial = l.path.as_deref().is_some_and(|p| !p.is_empty());
            if partial {
                let path = l.path.clone().unwrap_or_default();
                if let Some(var) = self.var_by_id_mut(l.root_id) {
                    loop_origin = var.for_origin;
                    var.poison = None;
                    if !var.moved_places.iter().any(|m| m.path == path) {
                        var.moved_places.push(MovedPlace {
                            path,
                            span: bind_span,
                        });
                    }
                }
                if let Some(origin) = loop_origin {
                    self.out.binding_modes.insert((file_idx, origin));
                }
                continue;
            }
            if let Some(var) = self.var_by_id_mut(l.root_id) {
                loop_origin = var.for_origin;
                var.narrowed = Ty::Nothing;
                var.consumed_by = None;
                var.poison = Some(Poison {
                    root_name: binding_name.to_string(),
                    event: FateEvent::BoundAway,
                    event_span: bind_span,
                });
            }
            // Consuming a `for`-loop binding means the loop iterates by
            // value: the loop itself becomes a move-mode event, keyed by
            // its iterable span for the emitter.
            if let Some(origin) = loop_origin {
                self.out.binding_modes.insert((file_idx, origin));
            }
        }
        self.out.binding_modes.insert((self.file_idx, bind_span));
        Vec::new()
    }

    fn var_by_id_mut(&mut self, id: u32) -> Option<&mut LocalVar> {
        self.locals
            .iter_mut()
            .rev()
            .find_map(|frame| frame.values_mut().find(|v| v.id == id))
    }

    /// The scope-frame index a variable id lives in [fate-lambda].
    fn frame_of_id(&self, id: u32) -> Option<usize> {
        self.locals
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, frame)| frame.values().any(|v| v.id == id).then_some(i))
    }

    // ================= lambda captures [fate-lambda] =================

    /// Records a read of an outer local inside enclosing lambdas: each
    /// lambda whose boundary lies above the variable's frame captures it
    /// [fate-lambda]. `mutable` = the value is transitively mutable.
    fn record_capture(&mut self, frame: usize, name: &str, var_id: u32, mutable: bool) {
        for ctx in &mut self.lambda_ctx {
            if frame < ctx.boundary && !ctx.captures.iter().any(|c| c.var_id == var_id) {
                ctx.captures.push(CaptureInfo {
                    name: name.to_string(),
                    var_id,
                    mutable,
                    mutated: false,
                    moved: false,
                });
            }
        }
    }

    /// Marks a captured variable as mutated by the lambda body: the
    /// closure takes ownership at creation [fate-lambda].
    fn mark_capture_mutated(&mut self, frame: usize, var_id: u32) {
        for ctx in &mut self.lambda_ctx {
            if frame < ctx.boundary {
                if let Some(c) = ctx.captures.iter_mut().find(|c| c.var_id == var_id) {
                    c.mutated = true;
                }
            }
        }
    }

    /// Handles consumption of a variable inside a lambda body
    /// [fate-lambda] [once-fn]: consuming a capture makes the lambda
    /// `once` — callable at most once — so the consumption is legal and
    /// proceeds (the value is owned by the closure from creation on).
    /// Exception: a *linear* capture may not be swallowed (the closure
    /// would inherit an exactly-once obligation, a future feature) —
    /// that reports an error and returns `true` so the consumption is
    /// skipped.
    fn capture_move_violation(&mut self, frame: usize, name: &str, span: Span) -> bool {
        let captured = self.lambda_ctx.iter().any(|ctx| frame < ctx.boundary);
        if !captured {
            return false;
        }
        let is_linear = self
            .lookup(name)
            .is_some_and(|v| self.ty_own_linear(&v.declared));
        if is_linear {
            if self.inferred.is_some() {
                self.error(
                    span,
                    format!(
                        "a lambda cannot consume `{name}`: it holds a linear \
                         value, and the closure would inherit an obligation to \
                         be called exactly once (not supported yet); pass the \
                         value explicitly instead"
                    ),
                );
            }
            return true;
        }
        // Mark every enclosing lambda `once` (an outer lambda re-creating
        // an inner consuming closure would re-consume per run).
        let var_id = self.lookup(name).map(|v| v.id);
        if let Some(var_id) = var_id {
            for ctx in &mut self.lambda_ctx {
                if frame < ctx.boundary {
                    if let Some(c) = ctx.captures.iter_mut().find(|c| c.var_id == var_id) {
                        c.moved = true;
                    }
                }
            }
        }
        false
    }

    /// [proj-anywhere] The qualifier a constructor call builds (`emitted(x)`
    /// → `Emitted`), when `value` is a call to a `-> T as Q` fn.
    fn constructed_qualifier(&self, value: &Expr) -> Option<String> {
        let Expr::Call { span, .. } = value else {
            return None;
        };
        self.constructed_qualifier_at(*span)
    }

    /// [proj-anywhere] `constructed_qualifier` by the call's span.
    fn constructed_qualifier_at(&self, span: Span) -> Option<String> {
        let key = *self.out.call_fn.get(&(self.file_idx, span))?;
        self.scope
            .fns
            .values()
            .flatten()
            .find(|e| e.key == key)
            .and_then(|e| e.decl.constructs.as_ref())
            .map(|c| c.name.name.clone())
    }

    /// [proj-anywhere] The value a one-argument constructor call wraps, or
    /// the expression itself.
    fn constructor_operand(value: &'p Expr) -> &'p Expr {
        match value {
            Expr::Call { args, .. } if args.len() == 1 => &args[0],
            _ => value,
        }
    }

    /// [proj-field] The names of a struct type's `proj` fields (empty
    /// for anything that is not a struct with one).
    fn proj_fields_of(&self, ty: &Ty) -> Vec<String> {
        let Ty::Named { name, .. } = ty.strip_quals() else {
            return Vec::new();
        };
        let Some(decl) = self.scope.structs.get(name.as_str()) else {
            return Vec::new();
        };
        decl.fields
            .iter()
            .filter(|f| first_proj_span(&f.ty).is_some())
            .map(|f| f.name.name.clone())
            .collect()
    }

    /// [proj-infer] Whether a value of this (lowered) type can hold a
    /// borrow: it is, or contains, a struct with a `proj` field, directly or
    /// through an owned field whose type does.
    fn ty_holds_proj(&self, ty: &Ty) -> bool {
        fn names(ty: &Ty, out: &mut Vec<String>) {
            match ty {
                Ty::Named { name, .. } => out.push(name.clone()),
                Ty::Qualified { base, .. } => names(base, out),
                Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().for_each(|a| names(a, out)),
                Ty::Array(elem) => names(elem, out),
                _ => {}
            }
        }
        let mut ns = Vec::new();
        names(ty, &mut ns);
        ns.into_iter().any(|n| {
            self.scope.structs.get(n.as_str()).is_some_and(|d| {
                d.fields.iter().any(|f| {
                    crate::lends::type_has_proj(&f.ty)
                        || crate::lends::holds_proj(&f.ty, &self.scope.structs)
                })
            })
        })
    }

    /// [deduce-syntax] A declaration without a body has nothing to infer
    /// from, so its clause must say what happens to every parameter — kept
    /// (`=> p`), consumed (`=> !p`), or qualified (`=> p: Mut`). Copy
    /// scalars are exempt: an `Int`'s fate is nothing to deduce
    /// [copy-scalar-free]. Implicit and variadic parameters are exempt too
    /// (an implicit is a fn value the call fills; a variadic tail is owned).
    fn require_full_clause(&mut self, f: &FnDecl, what: &str) {
        let mentioned: Vec<String> = f
            .deductions
            .as_ref()
            .map(|l| {
                l.iter()
                    .filter_map(|d| d.param_name().map(|n| n.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        for p in f.params.iter().filter(|p| !p.implicit && !p.variadic) {
            if mentioned.contains(&p.name.name) {
                continue;
            }
            let scalar = matches!(
                &p.ty,
                ast::Type::Named { qualifiers, base }
                    if qualifiers.is_empty()
                        && base.args.is_empty()
                        && matches!(
                            base.name.name.as_str(),
                            "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                        )
            );
            if scalar {
                continue;
            }
            self.error(
                p.name.span,
                format!(
                    "{what} has no body to infer from, so its deduction clause must say \
                     what happens to `{}`: `=> {}` to keep it, `=> !{}` to consume it",
                    p.name.name, p.name.name, p.name.name
                ),
            );
        }
    }

    /// [deduce-syntax] A fn's *effective* contract: the whole-program table's
    /// entry when the deduce pass has produced one (written entries fixed,
    /// the rest inferred), else the written clause with its unmentioned
    /// parameters optimistic — round one, or a declaration with no key.
    fn effective_contract(
        &self,
        key: Option<FnKey>,
        decl: &FnDecl,
    ) -> Option<Vec<crate::deduce::ParamDeduction>> {
        if let Some(k) = key {
            if let Some(found) = self.inferred.and_then(|table| table.get(&k).cloned()) {
                return Some(found);
            }
        }
        decl.deductions
            .as_ref()
            .map(|list| crate::deduce::from_written(decl, list, &HashSet::new(), |_, _| {}))
    }

    /// The resolver's key for a declaration, by identity.
    fn fn_key_of_decl(&self, f: &FnDecl) -> Option<FnKey> {
        self.scope
            .fns
            .get(f.name.name.as_str())
            .and_then(|v| v.iter().find(|e| std::ptr::eq(e.decl, f)).map(|e| e.key))
    }

    /// [proj-infer] The parameters `decl`'s result holds borrows of —
    /// declared (`[p: proj]`), inferred from its body, or every kept
    /// parameter when there is no body. Memoised per declaration.
    fn infer_lends(&mut self, decl: &'p FnDecl) -> Vec<usize> {
        let scope_fns = &self.scope.fns;
        let lookup = |name: &str| -> Vec<&'p FnDecl> {
            scope_fns
                .get(name)
                .map(|v| v.iter().map(|e| e.decl).collect())
                .unwrap_or_default()
        };
        let mut env = crate::lends::LendsEnv {
            structs: &self.scope.structs,
            fns: &lookup,
            memo: &mut self.lends_memo,
        };
        crate::lends::lends_of(decl, &mut env)
    }

    /// [proj-infer] A returned held view may be rooted only in the fn's own
    /// lent parameters (declared or inferred); a root that is a local dies
    /// with this call, and a root that is a parameter the signature does
    /// not lend would tie the caller's result to something it was told is
    /// free.
    fn check_returned_view_roots(&mut self, links: &[FateLink], span: Span) {
        for l in links {
            // Only *ultimate* roots are reported: a derived intermediate
            // (`let a = get(ts, i)`) is in the list alongside what it
            // derives from, and naming both would say one thing twice.
            if self
                .var_by_id(l.root_id)
                .is_some_and(|v| !v.links.is_empty())
            {
                continue;
            }
            let is_param_root = self.var_by_id(l.root_id).is_some_and(|v| v.is_param)
                || self.var_by_id(l.root_id).is_some_and(|v| {
                    v.links
                        .iter()
                        .any(|x| self.var_by_id(x.root_id).is_some_and(|r| r.is_param))
                });
            let root = l.root_name.clone();
            if !is_param_root {
                self.error(
                    span,
                    format!(
                        "cannot return this value: it holds a borrow of `{root}`, a local \
                         that dies with this call; return a `copy`, or borrow from a \
                         parameter instead"
                    ),
                );
                continue;
            }
            let lent = self.own_lends.iter().any(|n| n == &root)
                || self.var_by_id(l.root_id).is_some_and(|v| {
                    v.links
                        .iter()
                        .any(|x| self.own_lends.iter().any(|n| n == &x.root_name))
                });
            // A written `[p: proj]` list is checked against the body as a
            // whole (in `check_fn`); the per-return report would repeat it.
            if !lent && !self.own_lends_declared {
                self.error(
                    span,
                    format!(
                        "cannot return this value: it holds a borrow of `{root}`, which \
                         this function's signature does not lend; declare it with \
                         `[{root}: proj]`, or keep `{root}` so the borrow can be inferred"
                    ),
                );
            }
        }
    }

    /// [proj-anywhere] Whether an argument is a *temporary* — not a variable
    /// or a projection of one, and not a call that forwards a view of one.
    fn is_temporary(&self, arg: &Expr) -> bool {
        // [lambda-view] A lambda expression is never a dangling source: a
        // capturing one is a view whose links pass *through* to the
        // captured variables, and a capture-free one holds nothing a
        // result could borrow.
        if matches!(arg, Expr::Lambda { .. }) {
            return false;
        }
        let mut sources = Vec::new();
        Self::provenance(arg, &mut sources);
        if !sources.is_empty() {
            return false;
        }
        let inner = match arg {
            Expr::NonNull { operand, .. } => operand.as_ref(),
            other => other,
        };
        let forwards_view = matches!(inner, Expr::Call { .. } | Expr::StructLit { .. })
            && !self.links_for_value(inner, inner.span()).is_empty();
        !forwards_view
    }

    /// [proj-anywhere] Errors when `value` is a view of a temporary being
    /// bound, returned or stored: the temporary dies at the end of the
    /// statement, and the view would outlive it. Sees through a `!` and
    /// through a call that *forwards* such a view.
    fn reject_temp_view(&mut self, value: &Expr, what: &str) {
        let inner = match value {
            Expr::NonNull { operand, .. } => operand.as_ref(),
            other => other,
        };
        let Expr::Call { span, callee, .. } = inner else {
            return;
        };
        if !self.out.temp_views.contains(&self.key(*span)) {
            return;
        }
        let name = match callee.as_ref() {
            Expr::Ident(id) => id.name.clone(),
            Expr::Field { field, .. } => field.name.clone(),
            _ => "this call".to_string(),
        };
        self.error(
            value.span(),
            format!(
                "cannot {what} a view of a temporary: `{name}` borrows an argument \
                 that dies at the end of this statement; bind that argument with \
                 `let` first, so the view has something to borrow from"
            ),
        );
    }

    /// Validates a returned value against the fn's derived-return
    /// annotation [readonly-return]: `None` is fine (no borrow), and
    /// everything else must be derived from the annotated parameter —
    /// its provenance chain must terminate at `from` and nowhere else.
    fn check_derived_return_value(&mut self, value: &Expr, from: &str) {
        if matches!(value, Expr::Ident(id) if id.name == "None") {
            return;
        }
        // Any of the declared sources will do (`proj[from: a, b]`).
        let sources: Vec<String> = if self.own_derived_sources.is_empty() {
            vec![from.to_string()]
        } else {
            self.own_derived_sources.clone()
        };
        let source_ids: Vec<u32> = sources
            .iter()
            .filter_map(|n| self.lookup(n).map(|v| v.id))
            .collect();
        if source_ids.is_empty() {
            return;
        }
        let links = self.links_for_value(value, value.span());
        let ok = !links.is_empty()
            && links.iter().all(|l| {
                source_ids.contains(&l.root_id)
                    || self
                        .var_by_id(l.root_id)
                        .is_some_and(|v| v.links.iter().any(|x| source_ids.contains(&x.root_id)))
            });
        if !ok {
            let shown = sources.join(", ");
            self.error(
                value.span(),
                format!(
                    "this function returns `proj[from: {shown}]`, so every \
                     returned value must be derived from `{shown}` (a projection, \
                     element, or alias) or be `None`"
                ),
            );
        }
    }

    /// Applies a fn *value's* contract to the arguments of a call
    /// through it [fn-contract] — the fn-type sibling of the named-call
    /// contract loop (keep the two in sync): moved arguments are
    /// consumed, kept `Mut` positions are mutation events, kept
    /// positions shed their removal set, projections follow
    /// the moved/kept-`Mut` rules, and same-call ordering applies
    /// [deduce-same-call]. An absent contract keeps everything (the
    /// default), so only its `Mut` mutation events fire.
    fn apply_fn_value_contract(
        &mut self,
        args: &[&'p Expr],
        params: &[Ty],
        contract: Option<&[FnParamContract]>,
        span: Span,
    ) {
        // Record the effective per-argument contract for the emitter's
        // argument rendering [fn-contract].
        let effective: Vec<FnParamContract> = (0..args.len())
            .map(|i| {
                let declared_q: Vec<String> = params
                    .get(i)
                    .map(|t| t.quals().iter().map(|q| q.name.clone()).collect())
                    .unwrap_or_default();
                match contract.and_then(|c| c.get(i)) {
                    Some(e) => e.clone(),
                    None => FnParamContract {
                        name: None,
                        kept: true,
                        effect: QualEffect::KeepAll,
                        mutable: declared_q.iter().any(|q| q == "Mut"),
                        lent: false,
                    },
                }
            })
            .collect();
        self.out.fn_value_calls.insert(self.key(span), effective);
        self.apply_call_contract(args, params, contract, span);
    }

    /// The flow half of a contract application, shared by calls through
    /// fn values [fn-contract] and effect-member calls [effect-decl]:
    /// moved arguments are consumed, kept `Mut` positions are mutation
    /// events, kept positions shed their removal set [deduce-syntax],
    /// projections follow the moved/kept-`Mut` rules, and same-call
    /// ordering applies [deduce-same-call]. An absent contract keeps
    /// everything, so only its `Mut` mutation events fire.
    fn apply_call_contract(
        &mut self,
        args: &[&'p Expr],
        params: &[Ty],
        contract: Option<&[FnParamContract]>,
        span: Span,
    ) {
        let mut consumed_here: Vec<String> = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            if let Some(name) = consumed_here
                .iter()
                .find(|n| expr_mentions(arg, n))
                .cloned()
            {
                self.error(
                    arg.span(),
                    format!(
                        "`{name}` cannot be used here: it was consumed (moved) \
                         by an earlier argument of this call (arguments are \
                         evaluated left to right); use `copy` at the argument \
                         that consumes it"
                    ),
                );
            }
            let param_ty = params.get(i);
            let declared_q: Vec<String> = param_ty
                .map(|t| t.quals().iter().map(|q| q.name.clone()).collect())
                .unwrap_or_default();
            let (mut kept, effect, mutable) = match contract.and_then(|c| c.get(i)) {
                Some(e) => (e.kept, e.effect.clone(), e.mutable),
                None => (
                    true,
                    QualEffect::KeepAll,
                    declared_q.iter().any(|q| q == "Mut"),
                ),
            };
            // [proj-anywhere] A constructor lending into a `proj` arm keeps
            // its argument: the caller receives a borrow of it. Only when
            // the call really builds a constructive qualifier — otherwise
            // the ordinary contract stands and the return check reports.
            if self.lending_ctor == Some(span) && self.constructed_qualifier_at(span).is_some() {
                kept = true;
            }
            let Expr::Ident(id) = arg else {
                if kept && mutable {
                    self.fate_mutation_through(arg, span);
                } else if !kept {
                    let consumed = self.projection_move(arg, span);
                    // Data moved out of the projection: nothing narrowed
                    // about it survives [flow-place-invalidate].
                    if let Some(place) = Place::of_expr(arg) {
                        self.invalidate_place_narrows(&place);
                    }
                    consumed_here.extend(consumed);
                }
                continue;
            };
            let arg_once = self
                .lookup(&id.name)
                .map(|v| v.narrowed.quals().iter().any(|q| q.name == "once"))
                .unwrap_or(false);
            if !kept || arg_once {
                if self.inferred.is_none() {
                    continue;
                }
                let state = self.lookup(&id.name).map(|var| (var.id, var.links.clone()));
                let Some((var_id, links)) = state else {
                    continue;
                };
                if !links.is_empty() {
                    let name = id.name.clone();
                    self.error_derived(id.span, "move", &name, &links);
                    continue;
                }
                let name = id.name.clone();
                if let Some(frame) = self.frame_of_id(var_id) {
                    if self.capture_move_violation(frame, &name, id.span) {
                        continue;
                    }
                }
                if self.consume_kept_lambda_param(&name, id.span) {
                    continue;
                }
                self.poison_derived(var_id, &name, FateEvent::Moved, span, Some(&[]));
                if let Some(var) = self.lookup_mut(&id.name) {
                    var.narrowed = Ty::Nothing;
                    var.consumed_by = Some("an earlier call");
                }
                consumed_here.push(name);
                continue;
            }
            if mutable {
                let name = id.name.clone();
                // [flow-place-invalidate] A `Mut`-keeping call may mutate
                // the whole value, so every fact about its parts falls. A
                // kept *immutable* parameter cannot mutate: narrowings
                // survive such a call (user decision P1b).
                self.fate_mutation_root(&name, span);
            }
            // [deduce-syntax] The removal set is computed against the
            // qualifiers the *argument* actually carries, so an exhaustive
            // list also drops qualifiers the callee never declared (and
            // therefore cannot have preserved through a mutation).
            let have: Vec<String> = self
                .lookup(&id.name)
                .map(|v| v.narrowed.quals().iter().map(|q| q.name.clone()).collect())
                .unwrap_or_default();
            let removed = effect.removal_set(&have, |q| self.is_provenance_qual(q));
            if !removed.is_empty() {
                if let Some(var) = self.lookup_mut(&id.name) {
                    var.narrowed = var.narrowed.clone().remove_quals(&removed);
                }
            }
        }
    }

    /// Guards consumption of a lambda parameter the contract *keeps*
    /// [fn-contract]: the value belongs to the closure's caller, so it
    /// can only be read (a Rust borrow). Reports and returns `true` when
    /// the consumption must be skipped.
    fn consume_kept_lambda_param(&mut self, name: &str, span: Span) -> bool {
        let kept = self.lookup(name).is_some_and(|v| v.lambda_kept);
        if kept {
            self.error(
                span,
                format!(
                    "cannot consume `{name}`: this lambda's contract keeps it \
                     (the caller retains the value); use `copy({name})`"
                ),
            );
        }
        kept
    }

    /// [fate-move-mode] A projection in a *moved* position — a consuming
    /// call argument, a literal store, spread, `return`/`break`/`yield`,
    /// or a `use` constructor argument — moves data out of its
    /// provenance roots. Only projections of *transitively mutable* data
    /// are tracked: for immutable data the backends' clone-vs-alias
    /// difference is unobservable (backend-parity principle). Owned
    /// roots give up the value — they are consumed here, and the
    /// projection is recorded for the emitter (a real partial move); a
    /// written-kept parameter root is an error (moving out of a borrow),
    /// remedy `copy`. Round one records parameter claims only. Returns
    /// the names of the roots consumed (for same-call ordering checks
    /// [deduce-same-call]).
    fn projection_move(&mut self, expr: &Expr, span: Span) -> Vec<String> {
        let mut sources = Vec::new();
        Self::provenance(expr, &mut sources);
        if sources.is_empty() {
            return Vec::new();
        }
        let ty = self
            .out
            .ty_of(self.file_idx, expr.span())
            .cloned()
            .unwrap_or(Ty::Unknown);
        let mut visited = HashSet::new();
        // [linear-group] A projection of *linear* data is a real move
        // whatever its mutability: "immutable projections are free" is a
        // clone-vs-share latitude [fate-partial-move], and duplicating an
        // obligation is exactly what linearity cannot allow. Recording the
        // move is also what lets decomposition settle a container (`return
        // box.item` in its discharger).
        if !self.ty_transitively_mut(&ty, &mut visited) && !self.ty_own_linear(&ty) {
            return Vec::new();
        }
        let links = self.links_for_value(expr, span);
        if links.is_empty() {
            return Vec::new();
        }
        self.record_param_claims(&links);
        if self.inferred.is_none() {
            return Vec::new();
        }
        // A lambda cannot consume data out of a capture [fate-lambda].
        for l in &links {
            let Some(frame) = self.frame_of_id(l.root_id) else {
                continue;
            };
            let name = l.root_name.clone();
            if self.capture_move_violation(frame, &name, span) {
                return Vec::new();
            }
        }
        for l in &links {
            let Some(var) = self.var_by_id(l.root_id) else {
                continue;
            };
            if (var.is_param && !self.param_owned(&l.root_name)) || var.lambda_kept {
                let root = l.root_name.clone();
                self.error(
                    span,
                    format!(
                        "cannot move mutable data out of `{root}`: it is a kept \
                         parameter (the caller keeps it), so its projections can \
                         only be read; use `copy` to pass an independent value"
                    ),
                );
                return Vec::new();
            }
        }
        let mut consumed = Vec::new();
        for l in &links {
            self.poison_derived(
                l.root_id,
                &l.root_name,
                FateEvent::Moved,
                span,
                l.path.as_deref(),
            );
            let file_idx = self.file_idx;
            let mut loop_origin = None;
            // [fate-partial-move] A *proper* projection leaves the root
            // usable: record what left, and reads of disjoint projections
            // keep working. Only a whole-value move (or a derivation the
            // analysis could not place) consumes the variable outright.
            let partial = l.path.as_deref().is_some_and(|p| !p.is_empty());
            if partial {
                let path = l.path.clone().unwrap_or_default();
                if let Some(var) = self.var_by_id_mut(l.root_id) {
                    loop_origin = var.for_origin;
                    var.poison = None;
                    if !var.moved_places.iter().any(|m| m.path == path) {
                        var.moved_places.push(MovedPlace { path, span });
                    }
                }
                // Moving data out of a `for`-loop binding means the loop
                // iterates by value [fate-move-mode].
                if let Some(origin) = loop_origin {
                    self.out.binding_modes.insert((file_idx, origin));
                }
                continue;
            }
            if let Some(var) = self.var_by_id_mut(l.root_id) {
                loop_origin = var.for_origin;
                var.narrowed = Ty::Nothing;
                var.poison = None;
                var.consumed_by = Some(
                    "a move of mutable data projected out of it \
                     (`copy` at that site keeps it usable)",
                );
                consumed.push(l.root_name.clone());
            }
            // Moving data out of a `for`-loop binding means the loop
            // iterates by value [fate-move-mode].
            if let Some(origin) = loop_origin {
                self.out.binding_modes.insert((file_idx, origin));
            }
        }
        self.out
            .moved_projections
            .insert((self.file_idx, expr.span()));
        consumed
    }

    /// Whether a type's data is transitively mutable: `Mut` at any depth
    /// — top-level qualifier, type argument, array/tuple/union
    /// component, or a struct field followed recursively through struct
    /// declarations [type-canbe-mut] [fate-move-mode]. Mirrors the Kotlin
    /// backend's transitive immutability analysis for `copy` lowering.
    fn ty_transitively_mut(&self, ty: &Ty, visited: &mut HashSet<String>) -> bool {
        if ty.quals().iter().any(|q| q.name == "Mut") {
            return true;
        }
        match ty.strip_quals() {
            Ty::Named { name, args } => {
                if args.iter().any(|a| self.ty_transitively_mut(a, visited)) {
                    return true;
                }
                let Some(decl) = self.scope.structs.get(name.as_str()) else {
                    return false;
                };
                if !visited.insert(name.clone()) {
                    return false; // recursive struct: already being checked
                }
                decl.fields
                    .iter()
                    .any(|f| self.ast_type_mut(&f.ty, visited))
            }
            Ty::Array(elem) => self.ty_transitively_mut(elem, visited),
            Ty::Tuple(elems) => elems.iter().any(|e| self.ty_transitively_mut(e, visited)),
            Ty::Union(arms) => arms.iter().any(|a| self.ty_transitively_mut(a, visited)),
            _ => false,
        }
    }

    /// Whether a type *is* linear — it declares `: Linear<self>`
    /// [linear-group], or it is a union one of whose arms does (a union
    /// value is one value: the obligation cannot be lost by narrowing or
    /// by a branch merge), or it is a type parameter opted in with
    /// `<T canbe linear>` [linear-generics].
    ///
    /// **Not** transitive through composites, since R4 part 2: a
    /// composite may not *hold* a linear value at all
    /// [linear-composite], and every way of putting one in is refused at
    /// the store, so "a `List<FileHandle>` is itself linear" describes a
    /// type no accepted program can build. Keeping the obligation out of
    /// composites is also what keeps the diagnostics single: the store is
    /// the one error, with no follow-on leak for the container.
    fn ty_own_linear(&self, ty: &Ty) -> bool {
        self.ty_own_linear_guarded(ty, 0)
    }

    /// [col-key-eligible] Whether a type may be a **key**: a `Set`
    /// element, or a `Map` key. `None` means eligible; `Some(reason)` is
    /// the phrase the diagnostic uses.
    ///
    /// Hashing and equality are what a hash container needs, and this
    /// version answers them from the intrinsic types alone: `Int`, `Long`,
    /// `Str`, `Char` and `Bool` have both on every backend. Two exclusions
    /// are worth stating rather than discovering:
    ///
    /// * `Double`/`Float` are **not** keys. Rust's `f64` implements neither
    ///   `Eq` nor `Hash` (`NaN != NaN`), so a float-keyed map is not
    ///   representable there at all, while Kotlin would take it happily —
    ///   a divergence closed by restriction [backend-parity].
    /// * a struct is not a key *yet*: `canbe hashed` is the opt-in, and it
    ///   is what the diagnostic points at.
    ///
    /// A type *variable* is eligible: a generic fn's `K` is checked where
    /// it is instantiated, which is the same place [linear-generics] checks
    /// its own ban. `Unknown` passes for the usual reason
    /// [type-unknown-lenient] — one mistake, one diagnostic.
    fn key_ineligible(&self, ty: &Ty) -> Option<String> {
        self.hash_ineligible(ty, 0).map(|reason| {
            format!(
                "`{ty}` cannot be a key: {reason}. A `Set` element and a `Map` \
                 key have to be hashable"
            )
        })
    }

    /// [col-literal] An expected element type only helps when it is
    /// *concrete*: `to_set([1, 2])` expects `List<T>` for an unbound `T`, and
    /// taking `T` as the element type would leave nothing to infer it from —
    /// the literal's own elements are what determine it (and then bind `T`).
    fn concrete_elem(ty: Option<Ty>) -> Option<Ty> {
        ty.filter(|t| !matches!(t.strip_quals(), Ty::Var(_)))
    }

    /// [col-literal] The type of a collection literal: the named container
    /// at its element types, carrying a `Mut` when the position asks for one.
    ///
    /// A literal *constructs*, so adopting `Mut` from the expected type is
    /// sound and saves writing it twice (`let ys: Mut List<Int> = [4, 5]`).
    /// Only `Mut`: every other qualifier is a claim about the value that
    /// construction does not establish [qual-constructive].
    fn collection_lit_ty(&self, name: &str, args: Vec<Ty>, expected: Option<&Ty>) -> Ty {
        let ty = Ty::Named {
            name: name.to_string(),
            args,
        };
        let want_mut = expected.is_some_and(|t| t.quals().iter().any(|q| q.name == "Mut"));
        if want_mut {
            ty.qualify(vec![Qual {
                name: "Mut".to_string(),
                args: Vec::new(),
                effect: false,
            }])
        } else {
            ty
        }
    }

    /// [col-equality] Operand rules for the comparison operators (user
    /// decisions 2026-09-12).
    ///
    /// * **Equality works on every struct**, structurally, and ignores
    ///   qualifiers — `Surname Person == Person` is fine, because equality
    ///   is about the data at the moment of the check, not about what is
    ///   claimed of the handle.
    /// * **The two sides must be the same base type.** Comparing different
    ///   struct types is an error rather than a constant `false`: it is
    ///   almost always a mistake, and neither backend agrees on what it
    ///   would mean.
    /// * **A fn-typed field bars a struct from equality**: `Rc<dyn Fn>` has
    ///   none on Rust and Kotlin would compare by reference, so there is no
    ///   answer both backends can give.
    /// * **Ordering needs `canbe ordered`** on a struct, where equality does
    ///   not — the axes are separate.
    ///
    /// Unknown and `Nothing` operands stay lenient [type-unknown-lenient].
    fn check_comparison_operands(
        &mut self,
        op: ast::BinaryOp,
        span: Span,
        lhs: &Expr,
        rhs: &Expr,
        l: &Ty,
        r: &Ty,
    ) {
        let equality = matches!(op, ast::BinaryOp::Eq | ast::BinaryOp::NotEq);
        let (lb, rb) = (l.strip_quals(), r.strip_quals());
        if op_lenient(lb) || op_lenient(rb) {
            return;
        }
        let sym = op_symbol(op);
        // [op-promote] Numeric operands widen within their class for **every**
        // comparison, equality included (user decision 2026-09-18): `Int <
        // Long` compares at `Long` with the narrower side recording a
        // promotion, and so does `Int == Long`. Equality used to be excluded,
        // which made `n == 0` an error for a `Long` `n` while `n < 0` was fine
        // — an inconsistency with no reading behind it, since a widening
        // comparison is exact in both directions. The int↔float mix stays
        // refused the same way. This runs *before* the same-base check, which
        // mixed widths would otherwise trip.
        match (op_numeric(lb), op_numeric(rb)) {
            (Some((lf, lr)), Some((rf, rr))) => {
                if lf != rf {
                    self.error(
                        span,
                        format!(
                            "`{sym}` cannot mix `{l}` and `{r}`: integer and \
                             floating-point operands need an explicit conversion \
                             (`to_double`, `to_long`, …)"
                        ),
                    );
                    return;
                }
                match lr.cmp(&rr) {
                    std::cmp::Ordering::Less => self.record_promotion(lhs.span(), rb),
                    std::cmp::Ordering::Greater => self.record_promotion(rhs.span(), lb),
                    std::cmp::Ordering::Equal => {}
                }
                return;
            }
            (None, None) => {}
            _ => {
                self.error(
                    span,
                    format!(
                        "cannot compare `{l}` with `{r}`: the operands of a \
                         comparison must be the same type"
                    ),
                );
                return;
            }
        }
        // Same base type on both sides — qualifiers ignored, since equality
        // is about the data.
        let mismatch = match (lb, rb) {
            (Ty::Named { name: ln, .. }, Ty::Named { name: rn, .. }) => ln != rn,
            _ => false,
        };
        if mismatch {
            self.error(
                span,
                format!(
                    "cannot compare `{l}` with `{r}`: the operands of a \
                     comparison must be the same type"
                ),
            );
            return;
        }
        let Ty::Named { name, .. } = lb else {
            // [op-order] A non-named operand (tuple, array, fn value) has
            // no ordering either backend defines at the operator.
            if !equality {
                self.error(
                    span,
                    format!(
                        "`{l}` cannot be ordered: `{sym}` works on numeric operands \
                         and on structs declaring `canbe ordered`"
                    ),
                );
            }
            return;
        };
        let Some(decl) = self.scope.structs.get(name.as_str()) else {
            // [op-order] A non-struct, non-numeric base (`Str`, `Char`,
            // `Bool`, `Byte`, containers): equality per [col-equality],
            // ordering refused — the decided surface is numerics and
            // `canbe ordered` structs (user decision 2026-09-14).
            if !equality {
                self.error(
                    span,
                    format!(
                        "`{name}` cannot be ordered: `{sym}` works on numeric \
                         operands and on structs declaring `canbe ordered`"
                    ),
                );
            }
            return;
        };
        if decl.fields.iter().any(|f| matches!(f.ty, ast::Type::Fn { .. })) {
            self.error(
                span,
                format!(
                    "`{name}` cannot be compared: it holds a function-typed \
                     field, and a function value has no equality either \
                     backend can agree on (Rust has none at all, Kotlin would \
                     compare by reference)"
                ),
            );
            return;
        }
        if !equality && !self.has_auto_ordered(name) {
            self.error(
                span,
                format!(
                    "`{name}` cannot be ordered: declare `canbe ordered` on it \
                     to compare its values with `<`, `<=`, `>` and `>=` \
                     (equality needs no opt-in)"
                ),
            );
        }
    }

    /// [op-arith] Arithmetic operand typing (user decision 2026-09-14):
    /// numeric operands only — `Str +` is refused toward `${}`
    /// interpolation — with implicit widening *within* a class
    /// (`Int + Long → Long`, `Float + Double → Double`; the narrower side
    /// records a promotion [op-promote]) and an explicit-conversion error
    /// *across* classes. The result is the promoted operand type.
    /// `Unknown`/`Nothing` and unconstrained generics stay lenient
    /// [type-unknown-lenient].
    fn check_arith(
        &mut self,
        op: ast::BinaryOp,
        span: Span,
        lhs: &Expr,
        rhs: &Expr,
        l: &Ty,
        r: &Ty,
    ) -> Ty {
        let (lb, rb) = (l.strip_quals(), r.strip_quals());
        if op_lenient(lb) || op_lenient(rb) {
            // One mistake, one diagnostic: an unknown side neither errors
            // nor widens — a known numeric side carries the result.
            return match (op_numeric(lb), op_numeric(rb)) {
                (Some(_), _) => lb.clone(),
                (_, Some(_)) => rb.clone(),
                _ => Ty::Unknown,
            };
        }
        let sym = op_symbol(op);
        match (op_numeric(lb), op_numeric(rb)) {
            (Some((lf, lr)), Some((rf, rr))) => {
                if lf != rf {
                    self.error(
                        span,
                        format!(
                            "`{sym}` cannot mix `{l}` and `{r}`: integer and \
                             floating-point operands need an explicit conversion \
                             (`to_double`, `to_long`, …)"
                        ),
                    );
                    return Ty::Unknown;
                }
                match lr.cmp(&rr) {
                    std::cmp::Ordering::Equal => lb.clone(),
                    std::cmp::Ordering::Less => {
                        self.record_promotion(lhs.span(), rb);
                        rb.clone()
                    }
                    std::cmp::Ordering::Greater => {
                        self.record_promotion(rhs.span(), lb);
                        lb.clone()
                    }
                }
            }
            _ => {
                let is_str = |t: &Ty| matches!(t, Ty::Named { name, .. } if name == "Str");
                let message = if matches!(op, ast::BinaryOp::Add) && (is_str(lb) || is_str(rb)) {
                    "`+` does not concatenate strings: build the text with \
                     interpolation (`\"${a}${b}\"`)"
                        .to_string()
                } else {
                    format!(
                        "`{sym}` needs numeric operands (`Int`, `Long`, `Float`, \
                         `Double`); found `{l}` and `{r}`"
                    )
                };
                self.error(span, message);
                Ty::Unknown
            }
        }
    }

    /// [op-bool] A truth-valued operand (`&&`, `||`, `!`): `Bool` only —
    /// the value-position twin of the condition rule (no truthiness).
    fn require_bool_operand(&mut self, sym: &str, operand: &Expr, ty: &Ty) {
        let tb = ty.strip_quals();
        if op_lenient(tb) {
            return;
        }
        if matches!(tb, Ty::Named { name, .. } if name == "Bool") {
            return;
        }
        self.error(
            operand.span(),
            format!(
                "`{sym}` needs `Bool` operands (found `{ty}`): Salvo has no \
                 truthiness — compare explicitly (`n != 0`) or test the type \
                 with `is`"
            ),
        );
    }

    /// [op-promote] Records a numeric widening on an operand, for the
    /// backends whose operators do not mix widths natively (Rust casts;
    /// Kotlin's operator set covers the mixes).
    fn record_promotion(&mut self, span: Span, target: &Ty) {
        self.out.promotions.insert(self.key(span), target.clone());
    }

    /// [col-hashed-ordered] Whether a struct declares `canbe hashed`.
    fn has_auto_hashed(&self, name: &str) -> bool {
        self.scope
            .structs
            .get(name)
            .is_some_and(|s| s.auto_qualifiers.iter().any(|q| q.name.name == "hashed"))
    }

    /// [col-hashed-ordered] Whether a struct declares `canbe ordered`.
    fn has_auto_ordered(&self, name: &str) -> bool {
        self.scope
            .structs
            .get(name)
            .is_some_and(|s| s.auto_qualifiers.iter().any(|q| q.name.name == "ordered"))
    }

    /// [col-hashed-ordered] Whether a type can be **hashed** — the
    /// requirement for a `Set` element or a `Map` key. `None` is eligible.
    fn hash_ineligible(&self, ty: &Ty, depth: usize) -> Option<String> {
        if depth > 8 {
            return None;
        }
        match ty.strip_quals() {
            Ty::Var(_) | Ty::Unknown | Ty::Nothing => None,
            Ty::Named { name, args } => match name.as_str() {
                "Int" | "Long" | "Str" | "Char" | "Bool" => None,
                "Double" | "Float" => Some(format!(
                    "`{name}` is not hashable, because floating-point equality \
                     and hashing disagree between the backends (`NaN` equals \
                     nothing, not even itself). Equality on a value holding one \
                     still works — hashing is what cannot"
                )),
                // A container hashes when its elements do (Rust's `Vec` and
                // Kotlin's `List` both hash structurally).
                "List" | "Set" => args
                    .first()
                    .and_then(|a| self.hash_ineligible(a, depth + 1)),
                "Map" => args
                    .iter()
                    .take(2)
                    .find_map(|a| self.hash_ineligible(a, depth + 1)),
                _ if self.scope.structs.contains_key(name.as_str()) => {
                    if self.has_auto_hashed(name) {
                        None
                    } else {
                        Some(format!(
                            "`{name}` is a struct that does not declare \
                             `canbe hashed`"
                        ))
                    }
                }
                _ => Some(format!("`{name}` is not hashable")),
            },
            Ty::Tuple(elems) => elems
                .iter()
                .find_map(|e| self.hash_ineligible(e, depth + 1)),
            Ty::Array(elem) => self.hash_ineligible(elem, depth + 1),
            Ty::Fn { .. } => Some(
                "a function value is not hashable: neither backend can \
                 compare or hash one meaningfully"
                    .to_string(),
            ),
            Ty::Union(_) => Some(
                "a union is not hashable yet: every arm would have to be, \
                 which this version does not check"
                    .to_string(),
            ),
            other => Some(format!("`{other}` is not hashable")),
        }
    }

    /// [col-hashed-ordered] Whether a type can be **ordered** — the
    /// requirement for a `SortedSet` element or a `SortedMap` key.
    fn order_ineligible(&self, ty: &Ty, depth: usize) -> Option<String> {
        if depth > 8 {
            return None;
        }
        match ty.strip_quals() {
            Ty::Var(_) | Ty::Unknown | Ty::Nothing => None,
            Ty::Named { name, args } => match name.as_str() {
                "Int" | "Long" | "Str" | "Char" | "Bool" => None,
                "Double" | "Float" => Some(format!(
                    "`{name}` is not orderable, because Rust's `f64` has no \
                     total order (`NaN` compares less, greater and equal to \
                     nothing), so a sorted collection of them would not agree \
                     between the backends"
                )),
                // Lists and tuples order lexicographically by their elements
                // (user decision 2026-09-12).
                "List" => args
                    .first()
                    .and_then(|a| self.order_ineligible(a, depth + 1)),
                _ if self.scope.structs.contains_key(name.as_str()) => {
                    if self.has_auto_ordered(name) {
                        None
                    } else {
                        Some(format!(
                            "`{name}` is a struct that does not declare \
                             `canbe ordered`"
                        ))
                    }
                }
                _ => Some(format!("`{name}` is not orderable")),
            },
            Ty::Tuple(elems) => elems
                .iter()
                .find_map(|e| self.order_ineligible(e, depth + 1)),
            Ty::Array(elem) => self.order_ineligible(elem, depth + 1),
            Ty::Fn { .. } => Some(
                "a function value is not orderable: neither backend can \
                 compare one"
                    .to_string(),
            ),
            Ty::Union(_) => Some(
                "a union is not orderable: comparing values of different \
                 types has no obvious meaning (user decision 2026-09-12)"
                    .to_string(),
            ),
            other => Some(format!("`{other}` is not orderable")),
        }
    }

    /// [col-hashed-ordered] Validates a struct's `canbe hashed` /
    /// `canbe ordered` claims where they are written.
    ///
    /// Two conditions, both the user's rule (2026-09-12): the struct must be
    /// **immutable** — a `canbe Mut` struct could change under a hash table
    /// or a sorted tree, which is the classic silent corruption — and every
    /// field must itself be hashable/orderable.
    fn check_key_optins(&mut self, s: &'p ast::StructDecl) {
        let mutable = s.auto_qualifiers.iter().any(|q| q.name.name == "Mut");
        for q in &s.auto_qualifiers {
            let (claim, ordered) = match q.name.name.as_str() {
                "hashed" => ("hashed", false),
                "ordered" => ("ordered", true),
                _ => continue,
            };
            if mutable {
                self.error(
                    q.span,
                    format!(
                        "`{}` cannot be `canbe {claim}`: it is also `canbe Mut`, \
                         and a value that can change while a collection holds \
                         it would corrupt the collection's order or lookup. \
                         Only an immutable struct can be a key",
                        s.name.name
                    ),
                );
                continue;
            }
            for field in &s.fields {
                let empty = HashMap::new();
                let ty = self.lower_type_subst(&field.ty, &empty, 0);
                let bad = if ordered {
                    self.order_ineligible(&ty, 0)
                } else {
                    self.hash_ineligible(&ty, 0)
                };
                if let Some(reason) = bad {
                    self.error(
                        field.ty.span(),
                        format!(
                            "`{}` cannot be `canbe {claim}`: its field `{}` is \
                             not {claim} — {reason}",
                            s.name.name, field.name.name
                        ),
                    );
                }
            }
        }
    }

    /// [col-key-eligible] Reports an ineligible key in a written
    /// `Set<T>` / `Map<K, V>`. The *value* side of a map is unrestricted,
    /// so only the first argument is checked.
    fn check_key_eligibility(&mut self, base: &TypeRef) {
        // [col-sorted] The sorted collections need an *orderable* key, the
        // unordered ones a *hashable* one — different bars, so the container
        // decides which is checked.
        let (arity, ordered) = match base.name.name.as_str() {
            "Set" => (1, false),
            "Map" => (2, false),
            "SortedSet" => (1, true),
            "SortedMap" => (2, true),
            _ => return,
        };
        if base.args.len() != arity {
            return;
        }
        let empty = HashMap::new();
        let key = self.lower_type_subst(&base.args[0], &empty, 0);
        let bad = if ordered {
            self.sorted_key_ineligible(&key)
        } else {
            self.key_ineligible(&key)
        };
        if let Some(reason) = bad {
            let span = base.args[0].span();
            self.error(span, reason);
        }
    }

    /// [col-sorted-list] `Sorted` over a `List<T>` is a claim about the
    /// element order, so `Sorted List<Double>` is as meaningless as a
    /// `SortedSet<Double>` and is refused in the same place, by element.
    fn check_sorted_list_claim(&mut self, qualifiers: &[ast::TypeRef], base: &ast::TypeRef) {
        if base.name.name != "List" || base.args.len() != 1 {
            return;
        }
        if !qualifiers.iter().any(|q| q.name.name == "Sorted") {
            return;
        }
        let empty = HashMap::new();
        let elem = self.lower_type_subst(&base.args[0], &empty, 0);
        if let Some(reason) = self.sorted_list_elem_ineligible(&elem) {
            self.error(base.args[0].span(), reason);
        }
    }

    /// [col-sorted-list] The element rule for a `Sorted List<T>` claim.
    fn sorted_list_elem_ineligible(&self, elem: &Ty) -> Option<String> {
        self.order_ineligible(elem, 0).map(|reason| {
            format!(
                "`{elem}` cannot be the element of a `Sorted List`: {reason}. \
                 Sorting compares the elements, so they have to be orderable"
            )
        })
    }

    /// [col-sorted] The key rule for the sorted collections.
    fn sorted_key_ineligible(&self, ty: &Ty) -> Option<String> {
        self.order_ineligible(ty, 0).map(|reason| {
            format!(
                "`{ty}` cannot be a sorted collection's key: {reason}. A \
                 `SortedSet` element and a `SortedMap` key have to be orderable"
            )
        })
    }

    /// [col-key-eligible] The same rule against an already-lowered type,
    /// for the **inferred** case: `set_of(1.5)` writes no type at all, so
    /// the ineligible key only exists in the substitution the call
    /// resolved. Walks nested positions so `List<Map<Double, Int>>` is
    /// caught too.
    fn check_key_eligibility_ty(&mut self, ty: &Ty, span: Span, depth: usize) {
        if depth > 8 {
            return;
        }
        // [col-sorted-list] An *inferred* `Sorted List<T>` — what `sort` and
        // `mut_sort` return — carries the claim as a qualifier, so it has to
        // be read before `strip_quals` throws it away.
        if ty.quals().iter().any(|q| q.name == "Sorted") {
            if let Ty::Named { name, args } = ty.strip_quals() {
                if name == "List" && args.len() == 1 {
                    if let Some(reason) = self.sorted_list_elem_ineligible(&args[0]) {
                        self.error(span, reason);
                        return;
                    }
                }
            }
        }
        match ty.strip_quals() {
            Ty::Named { name, args } => {
                let (arity, ordered) = match name.as_str() {
                    "Set" => (1, false),
                    "Map" => (2, false),
                    "SortedSet" => (1, true),
                    "SortedMap" => (2, true),
                    _ => (0, false),
                };
                if arity > 0 && args.len() == arity {
                    let bad = if ordered {
                        self.sorted_key_ineligible(&args[0])
                    } else {
                        self.key_ineligible(&args[0])
                    };
                    if let Some(reason) = bad {
                        self.error(span, reason);
                        return;
                    }
                }
                for a in args {
                    self.check_key_eligibility_ty(a, span, depth + 1);
                }
            }
            Ty::Union(arms) => {
                for a in arms {
                    self.check_key_eligibility_ty(a, span, depth + 1);
                }
            }
            Ty::Tuple(elems) => {
                for e in elems {
                    self.check_key_eligibility_ty(e, span, depth + 1);
                }
            }
            Ty::Array(elem) => self.check_key_eligibility_ty(elem, span, depth + 1),
            _ => {}
        }
    }

    /// [linear-generics] The containment half: a **conditional container**
    /// (`struct Box<T canbe linear>`) is linear exactly when the
    /// instantiation puts a linear type where an opted-in parameter
    /// reaches a field (user decision 2026-09-12) — `Box<Lines>` owes,
    /// `Box<Int>` is plain. Depth-guarded: mutually recursive containers
    /// terminate conservatively.
    fn ty_own_linear_guarded(&self, ty: &Ty, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        match ty.strip_quals() {
            Ty::Named { name, args } => {
                if self.has_auto_linear(name) {
                    return true;
                }
                if args.is_empty() {
                    return false;
                }
                // [linear-container] Which parameters hold, then whether the
                // instantiation puts a linear type in one of them.
                self.linear_opt_in_positions(name.as_str())
                    .into_iter()
                    .filter_map(|i| args.get(i))
                    .any(|arg| self.ty_own_linear_guarded(arg, depth + 1))
            }
            Ty::Union(arms) => arms
                .iter()
                .any(|a| self.ty_own_linear_guarded(a, depth + 1)),
            // [linear-generics] An opted-in type parameter is treated as
            // linear inside its fn (worst case), which also makes calls
            // that forward it to other generics require *their* opt-in.
            Ty::Var(name) => self.own_linear_generics.contains(name),
            _ => false,
        }
    }

    /// [linear-group] Whether a declaration is **linear**: it carries the
    /// `linear struct` modifier (user decision 2026-09-12 — replacing the
    /// designated `: Linear<self>` group entry, which itself replaced
    /// `canbe linear`, 2026-09-08). Declaring it obliges the same file to
    /// contain at least one **discharger** — a fn consuming a parameter of
    /// this type — checked at the declaration.
    ///
    /// A `close` alone never makes a type linear: only the modifier does,
    /// and generic code opts in per type parameter with `<T canbe linear>`
    /// [linear-generics]. Attaching an obligation on the strength of a
    /// function name is what [qual-*] keeps the compiler from doing.
    fn has_auto_linear(&self, name: &str) -> bool {
        self.scope.structs.get(name).is_some_and(|s| s.linear)
            || self.opaque_linear(name)
    }

    /// [linear-group] The opaque half of the same rule: `linear intrinsic
    /// type Reply<T>` (user decision 2026-09-15). Nothing else differs — a
    /// linear opaque type owes exactly as a `linear struct` does, and its
    /// discharge set is computed the same way; it simply has no fields for
    /// linearity to reach through.
    fn opaque_linear(&self, name: &str) -> bool {
        self.scope.opaque_types.get(name).is_some_and(|t| t.linear)
    }

    /// [linear-generics] Whether values of this struct *can* owe: the
    /// `linear struct` marker, or a conditional container (`canbe linear`
    /// reaching a field). What gates being a discharger — a same-file
    /// consuming fn of a `Box<T canbe linear>` may `discard` its parameter
    /// whatever the instantiation, since a plain instantiation's discard
    /// is an ordinary drop.
    fn linear_capable(&self, name: &str) -> bool {
        if self.opaque_linear(name) {
            return true;
        }
        // [linear-container] An opaque conditional container (`intrinsic type
        // List<T canbe linear>`) can owe too, so its file's consuming fns —
        // `drain` — are dischargers.
        if !self.linear_opt_in_positions(name).is_empty() {
            return true;
        }
        self.scope.structs.get(name).is_some_and(|s| {
            s.linear
                || s.generic_canbe.iter().any(|(id, q)| {
                    q.name.name == "linear"
                        && s.fields.iter().any(|f| type_mentions_generic(&f.ty, &id.name))
                })
        })
    }

    /// [linear-group] The fields through which linearity reaches a value of
    /// `ty`: each field whose type, under this instantiation, is itself
    /// linear. Empty for a linear *leaf* (`linear struct FileHandle { fd:
    /// Int }`) — its obligation is its own, not its fields'.
    fn linear_fields_of(&self, ty: &Ty) -> Vec<String> {
        let Ty::Named { name, args } = ty.strip_quals() else {
            return Vec::new();
        };
        let Some(s) = self.scope.structs.get(name.as_str()) else {
            return Vec::new();
        };
        let subst: HashMap<String, Ty> = s
            .generics
            .iter()
            .map(|g| g.name.clone())
            .zip(args.iter().cloned())
            .collect();
        s.fields
            .iter()
            .filter(|f| {
                // The field's type under the instantiation: a generic
                // field substitutes; a concrete one stands as declared.
                let field_linear = match &f.ty {
                    ast::Type::Named { base, .. }
                        if base.args.is_empty() && subst.contains_key(&base.name.name) =>
                    {
                        subst
                            .get(&base.name.name)
                            .is_some_and(|t| self.ty_own_linear(t))
                    }
                    other => self.ast_type_own_linear(other).is_some(),
                };
                field_linear
            })
            .map(|f| f.name.name.clone())
            .collect()
    }

    /// [linear-group] The **discharge set** of a linear type: every fn
    /// declared in the *same file* as the type whose effective contract
    /// consumes a parameter of it — **plus every consuming member of an
    /// effect declared in that file** (the amendment the bare-members
    /// decision entailed, user decision 2026-09-14, FILE_SYSTEM.md §5.8):
    /// phase 4's `close(s: InStream)` is an `Fs` member, not a free fn, and
    /// `std/core/fs.sv` declares the token and the effect together.
    ///
    /// Any of these is a legal terminal for the obligation. `discard` is
    /// legal only inside one of them — and for a member that means inside
    /// **every handler's implementing body**, wherever the handler lives,
    /// since discharger status attaches to the *member declaration*
    /// [linear-discard].
    fn discharge_set(&self, type_name: &str) -> Vec<String> {
        // The declaring file is what the rule keys on, and either
        // declaration form can carry the obligation: a `linear struct` or a
        // `linear intrinsic type` [linear-group].
        let struct_file = match self.scope.structs.get(type_name) {
            Some(_) => self.scope.struct_files.get(type_name).copied(),
            // [linear-container] …or a **conditional container**, whose
            // terminal is what a leak diagnostic must name: `drain` for a
            // `List`/`Map` of obligations (user decision 2026-09-16). An
            // opaque type qualifies whether it is linear outright
            // (`linear intrinsic type Reply<T>`) or only when instantiated
            // with one.
            None if self.opaque_linear(type_name)
                || !self.linear_opt_in_positions(type_name).is_empty() =>
            {
                self.scope.opaque_type_files.get(type_name).copied()
            }
            None => return Vec::new(),
        };
        let mut out: Vec<String> = Vec::new();
        for (fn_name, entries) in self.scope.fns.iter() {
            for e in entries {
                if Some(e.key.file) != struct_file {
                    continue;
                }
                let consumes_self_typed = e.decl.params.iter().any(|p| {
                    let base_matches = match &p.ty {
                        ast::Type::Named { base, .. } => base.name.name == type_name,
                        ast::Type::QualifiedGroup { base, .. } => matches!(
                            base.as_ref(),
                            ast::Type::Named { base: b, .. } if b.name.name == type_name
                        ),
                        _ => false,
                    };
                    base_matches && self.param_consumed_by_entry(e, &p.name.name)
                });
                if consumes_self_typed {
                    out.push((*fn_name).to_string());
                    break;
                }
            }
        }
        // [linear-group] Consuming *members* of an effect declared in the
        // type's own file. A member has no body to infer from, so its
        // written clause is the whole contract [decl-explicit].
        for (effect_name, effect) in self.scope.effects.iter() {
            if self.scope.effect_files.get(effect_name).copied() != struct_file {
                continue;
            }
            for m in &effect.fns {
                if member_consumes_type(m, type_name) {
                    out.push(m.name.name.clone());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// [linear-group] The declaration-site check for the *opaque* form,
    /// mirroring `check_obligations`' struct half: a `linear intrinsic type`
    /// must have a legal death in its own file. Reported at the name, with
    /// the remedy spelled with an `intrinsic fn`, since an opaque type's
    /// discharger cannot have a body either.
    fn check_linear_opaque(&mut self, t: &'p ast::TypeDecl) {
        if !t.linear || self.inferred.is_none() {
            return;
        }
        if !self.discharge_set(&t.name.name).is_empty() {
            return;
        }
        self.error(
            t.name.span,
            format!(
                "linear intrinsic type `{0}` has no discharger: nothing in this \
                 file consumes a `{0}` — no fn, and no member of an effect \
                 declared here — so the obligation has no legal death; declare \
                 one (e.g. `intrinsic fn send<T>(r: {0}<T>, value: T) [] -> None \
                 => !r`)",
                t.name.name
            ),
        );
    }

    /// [linear-group] The discharge context a handler member's body runs in:
    /// the linear types the **effect member it implements** consumes. Matched
    /// by name and written parameter types
    /// (`salvo_core::effect_member_index`), so an overloaded member's bodies
    /// each get their own overload's contract — `close(InStream)` discharges
    /// an `InStream`, `close(OutStream)` an `OutStream`.
    /// [effect-handler-multi] Conformance of a handler to the effects it
    /// implements: every member of every face has an implementation, and a
    /// handler member that implements a member of *several* faces is one
    /// signature rather than two.
    ///
    /// The first half is what the target compilers used to report for us (a
    /// missing trait method), which put a Salvo mistake in a foreign
    /// diagnostic; the second is T-4(a)'s refinement — same-named members
    /// across faces are legal when overloading tells them apart (different
    /// written parameter types, so they are two members here) or when one
    /// method implements both (identical signatures), and refused where
    /// overloading *cannot* tell them apart.
    ///
    /// An `intrinsic` or `platform` handler is bodyless by design: its members
    /// are the backend's or the host's, so there is nothing here to match.
    fn check_handler_conformance(&mut self, h: &'p ast::HandlerDecl) {
        if h.intrinsic || h.platform {
            return;
        }
        let faces: Vec<(&'p ast::Type, &'p ast::EffectDecl)> = h
            .of
            .iter()
            .filter_map(|of| {
                let name = match of {
                    ast::Type::Named { base, .. } => base.name.name.as_str(),
                    ast::Type::QualifiedGroup { base, .. } => match base.as_ref() {
                        ast::Type::Named { base, .. } => base.name.name.as_str(),
                        _ => return None,
                    },
                    _ => return None,
                };
                self.scope.effects.get(name).copied().map(|e| (of, e))
            })
            .collect();
        let decls: Vec<&'p ast::EffectDecl> = faces.iter().map(|(_, e)| *e).collect();
        // Every member of every face, implemented.
        for (written, effect) in &faces {
            for (idx, member) in effect.fns.iter().enumerate() {
                let implemented = h.fns.iter().any(|f| {
                    crate::handler_member_faces(&decls, f)
                        .iter()
                        .any(|(e, i)| std::ptr::eq(*e, *effect) && *i == idx)
                });
                if implemented {
                    continue;
                }
                let params: Vec<String> = member
                    .params
                    .iter()
                    .filter(|p| !p.implicit)
                    .map(|p| p.ty.to_string())
                    .collect();
                self.error(
                    written.span(),
                    format!(
                        "handler `{}` does not implement `{}.{}({})`: a handler covers \
                         every member of every effect it implements",
                        h.name.name,
                        effect.name.name,
                        member.name.name,
                        params.join(", ")
                    ),
                );
            }
        }
        // A member that implements several faces at once: one signature, or
        // nothing to choose between them by.
        for f in &h.fns {
            let matched = crate::handler_member_faces(&decls, f);
            let Some((first, first_idx)) = matched.first().copied() else {
                continue;
            };
            let Some(first_member) = first.fns.get(first_idx) else {
                continue;
            };
            for (other, other_idx) in matched.iter().skip(1) {
                let Some(other_member) = other.fns.get(*other_idx) else {
                    continue;
                };
                if self.same_member_signature(first, first_member, other, other_member) {
                    continue;
                }
                self.error(
                    f.name.span,
                    format!(
                        "`{}` implements `{}.{}` and `{}.{}`, whose signatures differ in \
                         what overloading cannot see — the parameters are the same, so \
                         one member cannot answer for both: give them different \
                         parameters, or split the faces across two handlers",
                        f.name.name,
                        first.name.name,
                        first_member.name.name,
                        other.name.name,
                        other_member.name.name
                    ),
                );
            }
        }
    }

    /// [effect-handler-multi] Whether two effect members are the *same*
    /// signature, which is what lets one handler method implement both. Their
    /// written parameter types already agree (that is how they were matched),
    /// so what is left is the return type and the deduction clause — the two
    /// halves of the contract a caller relies on. Compared as lowered types
    /// under each effect's own generics, so an alias on one side is not a
    /// difference.
    fn same_member_signature(
        &mut self,
        a_effect: &'p ast::EffectDecl,
        a: &'p FnDecl,
        b_effect: &'p ast::EffectDecl,
        b: &'p FnDecl,
    ) -> bool {
        let lower = |me: &mut Self, effect: &'p ast::EffectDecl, ty: &Option<ast::Type>| {
            let saved = me.enter_generics(&effect.generics);
            let out = ty.as_ref().map(|t| me.lower_type(t));
            me.generics = saved;
            out
        };
        let a_ret = lower(self, a_effect, &a.return_type);
        let b_ret = lower(self, b_effect, &b.return_type);
        if a_ret != b_ret {
            return false;
        }
        // The deduction clauses, as written: a face that says a parameter is
        // consumed and one that says it is kept are different promises, and the
        // body can only keep one of them.
        let entries = |f: &'p FnDecl| -> Vec<String> {
            f.deductions
                .iter()
                .flatten()
                .map(|d| format!("{d:?}"))
                .collect()
        };
        entries(a) == entries(b)
    }

    /// [effect-handler-multi] The effects a handler implements, as declarations
    /// and in declaration order. A face whose name is not a declared effect in
    /// scope is dropped here and reported where it was written.
    fn handler_faces(&self, h: &'p ast::HandlerDecl) -> Vec<&'p ast::EffectDecl> {
        h.of
            .iter()
            .filter_map(|of| match of {
                ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
                ast::Type::QualifiedGroup { base, .. } => match base.as_ref() {
                    ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
                    _ => None,
                },
                _ => None,
            })
            .filter_map(|name| self.scope.effects.get(name).copied())
            .collect()
    }

    /// [effect-handler-multi] Searched across **every** implemented effect,
    /// since a member may belong to any of the handler's faces (and a member
    /// that implements two of them gets the first face's contract — the
    /// signatures are identical where that is legal, so the contracts are too).
    fn member_discharge_context(
        &self,
        h: &'p ast::HandlerDecl,
        f: &FnDecl,
    ) -> HashSet<String> {
        let faces = self.handler_faces(h);
        for (effect, idx) in crate::handler_member_faces(&faces, f) {
            if let Some(member) = effect.fns.get(idx) {
                return self.member_discharges(&effect.name.name, member);
            }
        }
        HashSet::new()
    }

    /// [linear-group] The linear types a *member* consumes — the discharge
    /// contexts its implementing bodies get. A member consumes a parameter
    /// when its written clause moves it (`=> !s`), and the type joins only
    /// when the effect is declared in that type's own file, which is what
    /// keeps the discharge set a property of the type's own module.
    fn member_discharges(&self, effect_name: &str, member: &FnDecl) -> HashSet<String> {
        let effect_file = self.scope.effect_files.get(effect_name).copied();
        member
            .params
            .iter()
            .filter_map(|p| {
                let base = match &p.ty {
                    ast::Type::Named { base, .. } => &base.name.name,
                    ast::Type::QualifiedGroup { base, .. } => match base.as_ref() {
                        ast::Type::Named { base, .. } => &base.name.name,
                        _ => return None,
                    },
                    _ => return None,
                };
                if !self.linear_capable(base) {
                    return None;
                }
                if self.scope.struct_files.get(base.as_str()).copied() != effect_file {
                    return None;
                }
                let moved = member.deductions.iter().flatten().any(|d| {
                    d.param_name().is_some_and(|n| n.name == p.name.name)
                        && matches!(d.kind, ast::DeductionKind::Moved)
                });
                moved.then(|| base.clone())
            })
            .collect()
    }

    /// [linear-group] A human-readable name for a linear value's legal
    /// terminals: its type's dischargers, backticked and `/`-joined
    /// ("`stop`/`join`"), or a generic phrase where the type is opaque (a
    /// generic parameter) or the set is empty (the declaration check
    /// reports that separately).
    fn linear_discharge_hint(&self, ty: &Ty) -> String {
        let name = match ty.strip_quals() {
            Ty::Named { name, .. } => Some(name.clone()),
            Ty::Union(arms) => arms.iter().find_map(|a| match a.strip_quals() {
                Ty::Named { name, .. } if self.has_auto_linear(name) => Some(name.clone()),
                _ => None,
            }),
            _ => None,
        };
        let set = name.as_deref().map(|n| self.discharge_set(n)).unwrap_or_default();
        if set.is_empty() {
            "its discharger".to_string()
        } else {
            set.iter()
                .map(|f| format!("`{f}`"))
                .collect::<Vec<_>>()
                .join(" / ")
        }
    }

    /// Whether an overload entry's written/inferred contract consumes
    /// `param` — the declaration-side predicate `discharge_set` needs (the
    /// effective contract of an arbitrary decl, not the currently-checked
    /// fn).
    fn param_consumed_by_entry(&self, entry: &crate::resolve::FnEntry<'p>, param: &str) -> bool {
        // A written `!p` decides outright; otherwise consult the inferred
        // facts when available (round two).
        if let Some(list) = &entry.decl.deductions {
            if list.iter().any(|d| {
                d.param_name().is_some_and(|n| n.name == param)
                    && matches!(d.kind, ast::DeductionKind::Moved)
            }) {
                return true;
            }
        }
        if let Some(inferred) = self.inferred {
            if let Some(facts) = inferred.get(&entry.key) {
                return facts.iter().any(|f| f.param == param && !f.kept);
            }
        }
        false
    }

    /// `ty_own_linear` over written (AST) types, without lowering: used by
    /// the composite refusal, which runs at declaration sites
    /// [linear-composite].
    ///
    /// Deliberately blind to `<T canbe linear>` type parameters: a
    /// declaration that *may* be instantiated with a linear type stores
    /// nothing by itself — std's `add(list: Mut List<T>, elem: T)` is a
    /// legal signature — so the generic case is refused at the
    /// instantiation instead (`var_in_composite` at the call site, the
    /// type-argument check at a struct literal).
    fn ast_type_own_linear(&self, ty: &ast::Type) -> Option<String> {
        match ty {
            ast::Type::Named { base, .. } => {
                let name = &base.name.name;
                if self.has_auto_linear(name) {
                    return Some(name.clone());
                }
                // [linear-container] A conditional container written with a
                // linear argument in an opted-in, holding position
                // (`Box<Lines>`, `List<Reply<Int>>`) is linear itself.
                for i in self.linear_opt_in_positions(name.as_str()) {
                    if let Some(arg) = base.args.get(i) {
                        if let Some(found) = self.ast_type_own_linear(arg) {
                            return Some(found);
                        }
                    }
                }
                None
            }
            ast::Type::QualifiedGroup { base, .. } => self.ast_type_own_linear(base),
            // A union value is one value, so an arm's obligation is the
            // union's — which is why writing one is a store.
            ast::Type::Union { arms, .. } => arms.iter().find_map(|a| self.ast_type_own_linear(a)),
            ast::Type::Nullable { inner, .. } => self.ast_type_own_linear(inner),
            _ => None,
        }
    }

    /// [linear-container] Which of `name`'s type parameters opt into holding
    /// **linear** values (`<T canbe linear>`) *and* actually hold one — the
    /// positions that make an instantiation linear when their argument is
    /// (user decisions 2026-09-12 for structs, 2026-09-16 for opaque types).
    ///
    /// The two kinds differ in how "holds" is decided: a struct's parameter
    /// must **reach a field** (`Box<T canbe linear> { item: T }`), because a
    /// parameter the fields never mention stores nothing; an **opaque** type
    /// has no fields to read, so an opted parameter holds by definition —
    /// which is exactly what `intrinsic type List<T canbe linear>` claims
    /// about its elements.
    fn linear_opt_in_positions(&self, name: &str) -> Vec<usize> {
        if let Some(s) = self.scope.structs.get(name) {
            return s
                .generic_canbe
                .iter()
                .filter(|(_, q)| q.name.name == "linear")
                .filter(|(id, _)| s.fields.iter().any(|f| type_mentions_generic(&f.ty, &id.name)))
                .filter_map(|(id, _)| s.generics.iter().position(|g| g.name == id.name))
                .collect();
        }
        if let Some(t) = self.scope.opaque_types.get(name) {
            return t
                .generic_canbe
                .iter()
                .filter(|(_, q)| q.name.name == "linear")
                .filter_map(|(id, _)| t.generics.iter().position(|g| g.name == id.name))
                .collect();
        }
        Vec::new()
    }

    /// [linear-composite] R4 part 2 (interim, user decision 2026-09-08):
    /// a linear value may not be **stored in a composite** — a struct or
    /// handler-state field, a type argument, an array element, a tuple
    /// component, a union arm. Its obligation would have to travel with
    /// the container, and composition plus conditional linearity are one
    /// design question (roadmap L8), so until that is answered a linear
    /// value lives only in a local, a parameter or a return value.
    ///
    /// `position` names where the value would have landed, so the message
    /// reads as a refusal of the *store* rather than of the type.
    fn refuse_linear_composite(&mut self, span: Span, linear: &str, position: String) {
        self.error(
            span,
            format!(
                "`{linear}` is linear, so it cannot be {position}: nothing here would \
                 carry its obligation onward. A linear value lives in a local, a \
                 parameter, a return value, a union arm, a `linear struct`'s field, or \
                 a container that opts in — a `Mut List<{linear}>` or a \
                 `Mut Map<K, {linear}>`, whose terminal is `drain` [linear-container]"
            ),
        );
    }

    /// [linear-container] Why a *particular* container position refuses an
    /// obligation, where the general rule would be unhelpful (LC-3, user
    /// decision 2026-09-15). Both cases are **semantic** — the container drops
    /// a value as part of doing its job — rather than fences the
    /// implementation could remove later, so the diagnostic says which.
    fn linear_position_reason(&self, container: &str, index: usize) -> Option<String> {
        // Std's own containers, not a program's types of the same name.
        if !self.scope.opaque_types.contains_key(container) {
            return None;
        }
        match (container, index) {
            ("Set", 0) | ("SortedSet", 0) => Some(
                "a set **deduplicates**: inserting a value equal to one already there \
                 drops one of the two, and silently dropping an obligation is exactly \
                 what linearity prevents. A `Mut List<T>` holds obligations (its \
                 terminal is `drain`); a set of *identifiers* with the obligations in a \
                 `Mut Map<K, T>` beside it is the other shape"
                    .to_string(),
            ),
            ("Map", 0) | ("SortedMap", 0) => Some(
                "a map's **keys** are compared and retained, and storing under a key \
                 that is already there drops one of the two. Keys are data; the \
                 *values* are where obligations go — `Mut Map<K, V>` opts its values \
                 in, and `remove`/`replace`/`drain` are how they leave"
                    .to_string(),
            ),
            _ => None,
        }
    }

    /// The immediate components of a written composite type, refused when
    /// one of them is linear [linear-composite]. Called from
    /// `validate_type`, which visits every written type once, and checks
    /// only one level: a nested composite reports at its own node, so
    /// `List<List<Lines>>` is one error, at the inner list.
    fn check_linear_components(&mut self, ty: &ast::Type) {
        let mut found: Vec<(Span, String, String)> = Vec::new();
        // [linear-container] LC-3's positions, which explain themselves.
        let mut reasoned: Vec<(Span, String, String)> = Vec::new();
        match ty {
            ast::Type::Named { base, .. } => {
                let opted = self.linear_opt_in_positions(base.name.name.as_str());
                for (i, a) in base.args.iter().enumerate() {
                    if let Some(linear) = self.ast_type_own_linear(a) {
                        // [linear-container] A parameter that declares
                        // `canbe linear` accepts a linear argument: the
                        // container is then *conditionally linear* and the
                        // obligation is checked on the container itself
                        // (user decision 2026-09-12, extended to opaque
                        // containers like `List` 2026-09-16).
                        if opted.contains(&i) {
                            continue;
                        }
                        match self.linear_position_reason(base.name.name.as_str(), i) {
                            // [linear-container] LC-3's two semantic refusals
                            // say why in their own words.
                            Some(reason) => reasoned.push((a.span(), linear, reason)),
                            None => found.push((
                                a.span(),
                                linear,
                                format!("a type argument of `{}`", base.name.name),
                            )),
                        }
                    }
                }
            }
            ast::Type::Array { elem, .. } => {
                if let Some(linear) = self.ast_type_own_linear(elem) {
                    found.push((elem.span(), linear, "an array's element type".to_string()));
                }
            }
            ast::Type::Tuple { elems, .. } => {
                for e in elems {
                    if let Some(linear) = self.ast_type_own_linear(e) {
                        found.push((e.span(), linear, "a tuple component".to_string()));
                    }
                }
            }
            // [linear-union-arm] A union arm may be linear (O-C2, user
            // decision 2026-09-12): a union value is one handle, so the
            // obligation is the union's and narrowing settles it —
            // `Ok InputStream | Err Str` is exactly the fallible-open
            // shape phase 4 needs, and `T?` follows. The value-level
            // rules live in `owes_linear`; nothing to refuse here.
            ast::Type::Union { .. } | ast::Type::Nullable { .. } => {}
            ast::Type::QualifiedGroup { .. } | ast::Type::Fn { .. } => {}
        }
        for (span, linear, position) in found {
            self.refuse_linear_composite(span, &linear, position);
        }
        for (span, linear, reason) in reasoned {
            self.error(
                span,
                format!("`{linear}` is linear, so it cannot go here: {reason}"),
            );
        }
    }

    /// Whether a variable currently *owns* a live linear obligation
    /// [linear-obligation]: its declared type is linear, it holds a live
    /// value (not consumed), it is not an alias (derived variables carry
    /// no obligation), and — for parameters — the fn's effective
    /// contract moves it (a kept parameter leaves the obligation with
    /// the caller). Only checked once inferred contracts exist (round
    /// two onwards).
    fn owes_linear(&self, name: &str, var: &LocalVar) -> bool {
        if self.inferred.is_none() {
            return false;
        }
        if !var.links.is_empty() || matches!(var.narrowed, Ty::Nothing) {
            return false;
        }
        if var.is_param && !self.param_owned(name) {
            return false;
        }
        // [linear-union-arm] O-C2 (user decision 2026-09-12): a union value
        // *is* the value — one handle, not a box holding one — so a written
        // linear arm makes the un-narrowed union owe, and **narrowing
        // decides**: narrowed to the linear arm, the value owes as that
        // arm; narrowed to a non-linear arm, the obligation is discharged —
        // an `Err Str` never held the handle. `T?` falls out (`None` owes
        // nothing). The settling is a flow fact (`linear_settled`) so it
        // survives the branch narrow-restore, and the live narrowed type
        // covers the within-branch case.
        if var.linear_settled {
            return false;
        }
        // [linear-group] [linear-generics] **Decomposition settles a
        // container** (user decision 2026-09-12): when every field through
        // which linearity reaches this value has been moved out — `return
        // box.item` in `unbox`, `end(t.source)` in a wrapper's close —
        // nothing inside owes any more, and the shell dies freely. Only
        // containers qualify: a linear *leaf* (no linear fields) owes as
        // itself and settles only by discharge.
        {
            let ty = if matches!(var.narrowed, Ty::Unknown) {
                &var.declared
            } else {
                &var.narrowed
            };
            let lf = self.linear_fields_of(ty);
            if !lf.is_empty()
                && lf.iter().all(|f| {
                    var.moved_places
                        .iter()
                        .any(|m| matches!(m.path.first(), Some(crate::place::Step::Field(n)) if n == f))
                })
            {
                return false;
            }
        }
        if !matches!(var.narrowed, Ty::Unknown) {
            return self.ty_own_linear(&var.narrowed);
        }
        self.ty_own_linear(&var.declared)
    }

    /// [linear-state] LC-4's second half (user decision 2026-09-15): an
    /// activation must leave **every state field whole**. Taking an
    /// obligation out of handler state is legal — draining a queue of parked
    /// tokens is the point — but a member that returns with a field moved out
    /// has left a hole a later activation would read, and no analysis can see
    /// what the actor holds at an arbitrary future point. So the rule is
    /// per activation, exactly as it is for a `Mut` parameter: put something
    /// back (`waiting = mut_list_of()`), or do not move.
    ///
    /// Called at every exit from a member body, beside the linear-leak checks
    /// it complements: that one asks whether a *value* still owes, this one
    /// whether the *storage* is intact.
    fn check_state_whole(&mut self, span: Span, what: &str) {
        if self.own_handler.is_none() || self.inferred.is_none() {
            return;
        }
        let holes: Vec<String> = self
            .locals
            .iter()
            .flat_map(|frame| {
                frame
                    .iter()
                    .filter(|(_, var)| {
                        var.is_handler_state
                            // Only obligation-holding state: a `Copy` scalar
                            // field is "moved" by every read that hands it
                            // over, and copying it leaves no hole at all —
                            // which is why [effect-state-store] exempts it.
                            && self.ty_own_linear(&var.declared)
                            && (matches!(var.narrowed, Ty::Nothing) || !var.moved_places.is_empty())
                    })
                    .map(|(name, _)| name.clone())
            })
            .collect();
        for name in holes {
            self.error(
                span,
                format!(
                    "cannot {what}: the state field `{name}` was moved out of and \
                     nothing was put back, so this activation would leave the \
                     handler with a hole — assign it a value first (`{name} = …`), \
                     since the obligations an actor holds must survive between \
                     activations"
                ),
            );
            if let Some(var) = self.lookup_mut(&name) {
                // Reported once: treat it as whole again so an enclosing exit
                // does not repeat it.
                var.narrowed = Ty::Unknown;
                var.moved_places.clear();
            }
        }
    }

    /// Reports every live linear obligation in the top scope frame —
    /// called just before the frame pops [linear-obligation]. The
    /// reported variables are marked consumed so enclosing checks do not
    /// re-report them.
    fn check_linear_frame_drop(&mut self) {
        let Some(frame) = self.locals.last() else {
            return;
        };
        let owed: Vec<(String, Span)> = frame
            .iter()
            .filter(|(name, var)| self.owes_linear(name, var))
            .map(|(name, var)| (name.clone(), var.decl_span))
            .collect();
        for (name, decl_span) in owed {
            let hint = self
                .lookup(&name)
                .map(|v| self.linear_discharge_hint(&v.declared))
                .unwrap_or_else(|| "its discharger".to_string());
            self.error(
                decl_span,
                format!(
                    "`{name}` still owns a linear value when it goes out of \
                     scope; move it onward (pass, return, or store it) or \
                     discharge it with {hint}"
                ),
            );
            if let Some(var) = self.lookup_mut(&name) {
                var.narrowed = Ty::Nothing;
            }
        }
    }

    /// Reports live linear obligations in every frame at (or above)
    /// `from_frame` — used at `return` (all frames) and `break`/
    /// `continue` (frames inside the loop) [linear-obligation]. Reported
    /// variables are marked consumed.
    fn check_linear_exit(&mut self, from_frame: usize, span: Span, what: &str) {
        let owed: Vec<String> = self
            .locals
            .iter()
            .skip(from_frame)
            .flat_map(|frame| {
                frame
                    .iter()
                    .filter(|(name, var)| self.owes_linear(name, var))
                    .map(|(name, _)| name.clone())
            })
            .collect();
        for name in owed {
            let hint = self
                .lookup(&name)
                .map(|v| self.linear_discharge_hint(&v.declared))
                .unwrap_or_else(|| "its discharger".to_string());
            self.error(
                span,
                format!(
                    "cannot {what} while `{name}` still owns a linear value; \
                     move it onward, or discharge it with {hint}"
                ),
            );
            if let Some(var) = self.lookup_mut(&name) {
                var.narrowed = Ty::Nothing;
            }
        }
    }

    /// `error`, unless an identical diagnostic was already reported: one
    /// exit can be reached by two walks (a `throw` inside a loop, say), so
    /// the same broken fact can be found twice on one path.
    fn error_once(&mut self, span: Span, message: String) {
        let seen = self
            .out
            .errors
            .iter()
            .any(|e| e.file == self.file_idx && e.span == span && e.message == message);
        if !seen {
            self.error(span, message);
        }
    }

    // ================= fn-type effects [fn-effects] =================

    /// Lowers a fn type's effect list [fn-effects]. `use` is not meaningful
    /// there: registering a handler is a *local* act, so a lambda may do it
    /// exactly when the function containing it may.
    fn lower_fn_effects(&mut self, effects: Option<&[EffectRef]>) -> Vec<Ty> {
        let mut out: Vec<Ty> = Vec::new();
        for eff in effects.into_iter().flatten() {
            match eff {
                EffectRef::Use(span) => self.error(
                    *span,
                    "a fn type cannot declare `use`: registering a handler \
                     is local to a body, so a lambda may `use` exactly when \
                     the function containing it may"
                        .to_string(),
                ),
                // [actor-spawn-effect] Same reasoning: the capability is a
                // property of the body that spawns, not of a value's type.
                EffectRef::Spawn(span) => self.error(
                    *span,
                    "a fn type cannot declare `spawn`: creating an actor is \
                     local to a body, so a lambda may spawn exactly when the \
                     function containing it may"
                        .to_string(),
                ),
                // [waitfor-effect] [actor-no-closure] And the same again: a
                // function value runs wherever it is called, so it cannot
                // carry a claim about *where* — which is the whole content of
                // this capability.
                EffectRef::WaitFor(span) => self.error(
                    *span,
                    "a fn type cannot declare `waitfor`: the capability is a \
                     claim about the thread the body runs on, and a function \
                     value runs wherever it is called"
                        .to_string(),
                ),
                EffectRef::Effect(r) => {
                    if let Some(ty) = self.lower_effect_ref(r) {
                        if !out.contains(&ty) {
                            out.push(ty);                        }
                    }
                }
            }
        }
        out
    }

    /// The effects a parameter's type contributes to the enclosing fn
    /// [fn-effects]: those declared on a fn type, reached through
    /// qualifiers (`once () [Console] -> None`) and optional wrappers
    /// (`((s: Str) [Logger] -> Str)?`).
    fn inherited_fn_effects(&mut self, ty: &ast::Type) -> Vec<Ty> {
        match ty {
            ast::Type::Fn { effects, .. } => self.lower_fn_effects(effects.as_deref()),
            // [fn-effects] An effect claim in qualifier position is
            // inherited from for the same reason a fn type's list is: it
            // says driving the value performs the effect. The spelling is
            // *refused* at declarations now (a pass performs its effects in
            // its `next`), but the refused type still lowers — one mistake,
            // one diagnostic — so the claim is read off the qualified type
            // here, and the *base* is still walked, since a qualified group
            // can wrap a fn type (`once (() [Console] -> None)`).
            ast::Type::QualifiedGroup { base, .. } => {
                let mut out = self.claimed_effects(ty);
                for ty in self.inherited_fn_effects(base) {
                    if !out.contains(&ty) {
                        out.push(ty);
                    }
                }
                out
            }
            ast::Type::Named { .. } => self.claimed_effects(ty),
            ast::Type::Nullable { inner, .. } => self.inherited_fn_effects(inner),
            ast::Type::Union { arms, .. } => arms
                .iter()
                .flat_map(|a| self.inherited_fn_effects(a))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// [fn-effects] The effects a type *claims*: the names in qualifier
    /// position that resolve to effect declarations, lowered to instances.
    /// Reaches through nullability and union arms like [fn-effects]'s
    /// inheritance does, so `Logger MyPass?` claims `Logger`. The spelling
    /// is refused at declarations (a pass performs its effects in its
    /// `next`), so this feeds the refusal diagnostic and the leniency path.
    fn claimed_effects(&mut self, ty: &ast::Type) -> Vec<Ty> {
        let mut out: Vec<Ty> = Vec::new();
        match ty {
            ast::Type::Named { qualifiers, .. } | ast::Type::QualifiedGroup { qualifiers, .. } => {
                for q in qualifiers {
                    if self.scope.qualifiers.contains_key(q.name.name.as_str()) {
                        continue;
                    }
                    if self.scope.effects.contains_key(q.name.name.as_str()) {
                        if let Some(ty) = self.lower_effect_ref(q) {
                            if !out.contains(&ty) {
                                out.push(ty);
                            }
                        }
                    }
                }
            }
            ast::Type::Nullable { inner, .. } => out.extend(self.claimed_effects(inner)),
            ast::Type::Union { arms, .. } => {
                for arm in arms {
                    for ty in self.claimed_effects(arm) {
                        if !out.contains(&ty) {
                            out.push(ty);
                        }
                    }
                }
            }
            _ => {}
        }
        out
    }

    /// Why a *written* qualifier name may not be removed with `^`, if it may
    /// not [qual-widen]. Consults the effect namespace first: an effect claim
    /// on a producer never drops [fn-effects], and the name-based list
    /// cannot know an arbitrary effect's name.
    fn removal_block(&self, name: &str) -> Option<&'static str> {
        if !self.scope.qualifiers.contains_key(name) && self.scope.effects.contains_key(name) {
            return Qual::effect(name, Vec::new()).drop_block();
        }
        crate::types::qual_drop_block(name)
    }

    /// Records that an effect instance was used, for the *inference* of an
    /// enclosing un-annotated lambda's effect set [fn-effects].
    fn note_effect_use(&mut self, ty: &Ty) {
        if let Some(frame) = self.effect_uses.last_mut() {
            if !frame.contains(ty) {
                frame.push(ty.clone());
            }
        }
    }

    /// The effects a fn value's *call* requires, checked against what is
    /// available here [fn-effects]. Records the instances to thread, keyed
    /// by the call span, exactly as a named call's dependencies are.
    fn check_fn_value_effects(&mut self, effects: &[Ty], span: Span) {
        self.check_effects_available(effects, span, true);
    }

    fn check_effects_available(&mut self, effects: &[Ty], span: Span, record: bool) {
        if effects.is_empty() {
            return;
        }
        let mut resolved: Vec<Ty> = Vec::new();
        // [use-no-dup] Innermost first, shadowed duplicates hidden.
        let visible = self.visible_effects();
        for want in effects {
            let found = visible.iter().find(|c| *c == want).cloned();
            let found = found.or_else(|| {
                let compatible: Vec<&Ty> = visible
                    .iter()
                    .filter(|c| unify(want, c, &mut HashMap::new()))
                    .collect();
                match compatible.len() {
                    1 => Some(compatible[0].clone()),
                    _ => None,
                }
            });
            match found {
                Some(instance) => {
                    self.note_effect_use(&instance);
                    resolved.push(instance);
                }
                None => {
                    self.error(
                        span,
                        format!(
                            "no handler for effect `{want}` in scope, required by \
                             this function value (declare it in the function's \
                             effect list or `use` a handler)"
                        ),
                    );
                    resolved.push(want.clone());
                }
            }
        }
        if record {
            self.out.call_effects.insert(self.key(span), resolved);
        }
    }

    // ================= throw and `try` [throw] [try] =================

    /// The enclosing fn's declared throw message type, if it declared
    /// `[Throw<M>]`.
    fn declared_throw_message(&self) -> Option<Ty> {
        self.effect_env.iter().find_map(|e| match e {
            Ty::Named { name, args } if name == THROW_EFFECT => {
                Some(args.first().cloned().unwrap_or(Ty::Unknown))
            }
            _ => None,
        })
    }

    /// Records a site that may throw [throw] and reports the cases where
    /// nothing can receive it. Every such site is also an *exit*: the code
    /// between it and its delimiter does not run, so no linear obligation
    /// may be live [linear-obligation].
    fn record_throw(&mut self, span: Span, message: Ty, performs: bool) {
        match self.try_stack.last_mut() {
            // Inside a `try`: the delimiter collects the message type; the
            // wrap into its `M` is filled in once the body is checked.
            Some(ctx) => {
                let floor = ctx.entry_depth;
                ctx.sites.push((span, message.clone()));
                self.out.may_throw.insert(
                    self.key(span),
                    ThrowSite {
                        performs,
                        message,
                        // Filled in by `check_try` (the innermost `try` is
                        // the one being checked) [try-innermost].
                        delimiter: None,
                        target: Ty::Unknown,
                        arm: None,
                    },
                );
                self.check_linear_throw(floor, span);
            }
            // Outside every `try`: the enclosing fn must declare it.
            None => {
                let Some(target) = self.declared_throw_message() else {
                    self.error(
                        span,
                        format!(
                            "nothing here can receive a throw: wrap the call in a \
                             `try {{ ... }}` block, or declare `[{THROW_EFFECT}<{message}>]` \
                             in this function's effect list to pass it on"
                        ),
                    );
                    return;
                };
                let arm = self.throw_message_arm(span, &message, &target);
                self.out.may_throw.insert(
                    self.key(span),
                    ThrowSite {
                        performs,
                        message,
                        delimiter: None,
                        target,
                        arm,
                    },
                );
                self.check_linear_throw(0, span);
            }
        }
    }

    /// The arm of the target message type a site's message wraps into
    /// [union-arm-identity], reporting a message the target cannot carry.
    fn throw_message_arm(&mut self, span: Span, message: &Ty, target: &Ty) -> Option<usize> {
        if target.is_unknown() || message.is_unknown() {
            return None;
        }
        if target.is_wrapper_union() {
            let arms = target.value_arms();
            match arms.iter().position(|arm| **arm == *message) {
                Some(i) => {
                    self.out.union_sizes.insert(arms.len());
                    return Some(i);
                }
                None => {
                    if !is_subtype(message, target) {
                        self.error(
                            span,
                            format!(
                                "this throw carries a `{message}` message, but the \
                                 throw it lands in carries `{target}`"
                            ),
                        );
                    }
                    return None;
                }
            }
        }
        if !is_subtype(message, target) {
            self.error(
                span,
                format!(
                    "this throw carries a `{message}` message, but the throw it \
                     lands in carries `{target}`"
                ),
            );
        }
        None
    }

    /// The linear-obligation check at a throw [linear-obligation]: the
    /// same walk as an early `return`. Since `defer` was removed
    /// (2026-09-10) there is no way to discharge on a path the author does
    /// not write, so the diagnostic says to release before the call or to
    /// move the value onward.
    fn check_linear_throw(&mut self, from_frame: usize, span: Span) {
        let owed: Vec<String> = self
            .locals
            .iter()
            .skip(from_frame)
            .flat_map(|frame| {
                frame
                    .iter()
                    .filter(|(name, var)| self.owes_linear(name, var))
                    .map(|(name, _)| name.clone())
            })
            .collect();
        for name in owed {
            self.error_once(
                span,
                format!(
                    "`{name}` still owns a linear value across a call that may \
                     throw: the code after it does not run on the throw path, so \
                     release it before the call, or move it onward so the \
                     obligation travels with it"
                ),
            );
            if let Some(var) = self.lookup_mut(&name) {
                var.narrowed = Ty::Nothing;
            }
        }
    }

    /// `throw(message)` [throw]. Not an ordinary effect-member call: there
    /// is no handler to resolve — the delimiter is `try` — and the result
    /// is the bottom type, so the code after it never runs.
    fn check_throw_call(&mut self, member: &'p FnDecl, args: &[&'p Expr], span: Span) -> Ty {
        let message = match args.first() {
            Some(a) => {
                let ty = self.check_expr(a, None);
                // The message is moved into the outcome, like the value
                // passed to `err` [deduce-consume].
                self.fate_move(a, "throw with", "a `throw`", span);
                ty
            }
            None => {
                self.error(
                    span,
                    format!(
                        "`{}` takes the throw message as its only argument",
                        member.name.name
                    ),
                );
                Ty::Unknown
            }
        };
        for extra in args.iter().skip(1) {
            self.check_expr(extra, None);
        }
        self.record_throw(span, message, true);
        Ty::Nothing
    }

    /// `try { ... }` [try]: the delimiter. The body's value becomes the
    /// `Ok T` arm and the throws it performs the `Thrown M` arm, with `M`
    /// the union of their message types (user decision 2026-09-04) — which
    /// is why the outcome is an ordinary union: `is`, `when` and
    /// exhaustiveness need no new rules.
    /// Whether the code being checked is inside a lambda body: a function
    /// value's body runs wherever it is *called*, which is why the
    /// asynchronous forms stop at that boundary [fate-lambda].
    fn in_lambda(&self) -> bool {
        !self.lambda_ctx.is_empty()
    }

    /// [actor-self-send] `self.k(args)` — send a message to **the actor this
    /// member belongs to**: the one thing a member cannot say with an
    /// unqualified call, since that would be self-dispatch (a handler has no
    /// way to reach its own instance) rather than a message.
    ///
    /// Its point is *ordering*, not reach: an unqualified call would run `k`
    /// now, inside this activation; a self-send runs it as its own later one,
    /// which is how "finish this, then continue with `k`" is written. Defined
    /// for both bindings, like every other form: an enqueue on the actor's
    /// own mailbox when the handler is spawned, and the ordinary inline member
    /// call when it is `use`d — which is what a local binding of a `send`
    /// protocol already does.
    fn check_self_send(&mut self, member: &'p Ident, args: &'p [Expr], span: Span) -> Ty {
        let Some(h) = self.own_handler else {
            for a in args {
                self.check_expr(a, None);
            }
            let detail = if self.in_lambda() {
                " — and a lambda is not one: a function value runs wherever it is \
                 called, so `@self` names nothing there"
            } else {
                ""
            };
            self.error(
                span,
                format!(
                    "`{}@self(…)` names a member of the enclosing handler, so it is \
                     legal only inside a handler member{detail}",
                    member.name
                ),
            );
            return Ty::Unknown;
        };
        let Some(target) = h.fns.iter().find(|f| f.name.name == member.name) else {
            for a in args {
                self.check_expr(a, None);
            }
            self.error_unresolved(
                member.span,
                format!(
                    "handler `{}` has no member `{}` to send to",
                    h.name.name, member.name
                ),
                &member.name,
            );
            return Ty::Unknown;
        };
        // A self-send is a *message*, so its target is a send member: an
        // ordinary member would have to answer, and a member cannot wait for
        // itself.
        if !target.is_send {
            self.error(
                span,
                format!(
                    "`{}` is not a `send fn`, so `{}@self(…)` cannot reach it: a \
                     self-send is a message, and a member that answers would have to \
                     wait for itself",
                    member.name, member.name
                ),
            );
        }
        let saved = self.enter_generics(&h.generics);
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        let param_tys: Vec<Ty> = params.iter().map(|p| self.lower_type(&p.ty)).collect();
        self.generics = saved;
        if args.len() != param_tys.len() {
            for a in args {
                self.check_expr(a, None);
            }
            self.error(
                span,
                format!(
                    "`{}` takes {} argument(s), found {}",
                    member.name,
                    param_tys.len(),
                    args.len()
                ),
            );
            return Ty::none();
        }
        for (i, a) in args.iter().enumerate() {
            let want = &param_tys[i];
            let got = self.check_expr(a, Some(want));
            if !got.is_unknown() && !is_subtype(&got, want) {
                self.error(a.span(), format!("expected `{want}`, found `{got}`"));
            }
            let repr = self.repr_of(a, &got);
            self.maybe_coerce(a.span(), &got, &repr, want);
            // A payload crosses the seam even when both ends are the same
            // actor: the message outlives this activation, so the sender
            // gives it up [deduce-consume].
            self.fate_move(a, "send", "a send to this actor", a.span());
        }
        self.record_def_ref(member.span, &member.name);
        self.out
            .self_sends
            .insert(self.key(span), member.name.clone());
        // [actor-send-fn] A send answers nothing.
        Ty::none()
    }

    /// [actor-use-addr] The effect a dot-call's receiver serves, when the
    /// receiver is a **place** whose type is `Addr<E>` — a variable, a field
    /// chain (`registry.child`), a tuple element, or an array element.
    ///
    /// Read without *checking* the receiver, deliberately: the ordinary dot
    /// path checks it as argument zero, and checking it here as well would
    /// duplicate its diagnostics and count a move twice. A place has a type
    /// the checker can look up, which is what makes the peek possible; a
    /// receiver that is a **call** (`get(addrs, 0).bump(1)`) has none until it
    /// is checked, so it needs a `let` first.
    fn addr_receiver(&mut self, base: &Expr) -> Option<Ty> {
        let ty = self.peek_place_ty(base)?;
        addr_effect(&ty)
    }

    /// The type of a place, without checking it: `None` for anything that is
    /// not one, or whose type is not yet known. Errors are never reported
    /// from here — the caller either uses the answer or falls through to the
    /// path that does report.
    fn peek_place_ty(&mut self, expr: &Expr) -> Option<Ty> {
        match expr {
            Expr::Ident(id) => {
                let var = self.lookup(&id.name)?;
                let ty = if var.narrowed.is_unknown() {
                    var.declared.clone()
                } else {
                    var.narrowed.clone()
                };
                Some(ty)
            }
            Expr::Field { base, field, .. } => {
                let base_ty = self.peek_place_ty(base)?;
                self.field_ty(&base_ty, &field.name)
            }
            Expr::TupleIndex { base, index, .. } => match self.peek_place_ty(base)?.strip_quals() {
                Ty::Tuple(elems) => elems.get(*index).cloned(),
                _ => None,
            },
            // [index-resolve] Only arrays are subscriptable, so this is the
            // whole of element access as a place.
            Expr::Index { base, .. } => match self.peek_place_ty(base)?.strip_quals() {
                Ty::Array(elem) => Some((**elem).clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// [actor-use-addr] `addr.member(args)` — a send to an actor, and the
    /// inline form of `use addr` plus an unqualified call. The receiver names
    /// *where* the message goes rather than an argument, so the member's
    /// parameters line up with the written arguments exactly as they do at a
    /// handler.
    #[allow(clippy::too_many_arguments)]
    fn check_addr_call(
        &mut self,
        instance: &Ty,
        receiver: &'p Expr,
        member: &'p Ident,
        type_args: &'p [ast::Type],
        args: &'p [Expr],
        named: &'p [ast::NamedArg],
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        let _ = (type_args, named, expected);
        // The receiver is *read* — it names the target rather than being an
        // argument, but a read is what it is: without checking it here nothing
        // would record the use, and the variable would be reported unused
        // [unused-var]. Checked exactly once, which is why deciding that this
        // *is* an addr call peeks the type instead of checking it.
        self.check_expr(receiver, None);
        let effect_name = match instance.strip_quals() {
            Ty::Named { name, .. } => name.clone(),
            _ => return Ty::Unknown,
        };
        let Some(effect) = self.scope.effects.get(effect_name.as_str()).copied() else {
            for a in args {
                self.check_expr(a, None);
            }
            self.error(
                span,
                format!("effect `{effect_name}` is not in scope here"),
            );
            return Ty::Unknown;
        };
        let members = crate::effect_members_named(effect, &member.name);
        // [effect-member-overload] An overloaded send member is picked by
        // **arity** here, not by argument types: typing the arguments to
        // choose, then typing them again against the winner's parameters,
        // would report every mistake in them twice. Arity settles every
        // overload the first pass can express (nothing in std or the design
        // overloads a send member at all); a genuine tie is refused rather
        // than guessed.
        let target: Option<&'p FnDecl> = match members.as_slice() {
            [] => None,
            [only] => Some(only),
            many => {
                let fits: Vec<&'p FnDecl> = many
                    .iter()
                    .copied()
                    .filter(|f| {
                        let fixed = f
                            .params
                            .iter()
                            .filter(|p| !p.variadic && !p.implicit)
                            .count();
                        f.params.iter().any(|p| p.variadic) && args.len() >= fixed
                            || fixed == args.len()
                    })
                    .collect();
                match fits.as_slice() {
                    [one] => Some(one),
                    _ => {
                        for a in args {
                            self.check_expr(a, None);
                        }
                        self.error(
                            span,
                            format!(
                                "effect `{effect_name}` overloads `{}`, and {} of its \
                                 overloads take {} argument(s): a send through a \
                                 `{ADDR_TYPE}` picks by argument count, so this call \
                                 cannot say which one it means",
                                member.name,
                                fits.len(),
                                args.len()
                            ),
                        );
                        return Ty::Unknown;
                    }
                }
            }
        };
        let Some(target) = target else {
            for a in args {
                self.check_expr(a, None);
            }
            self.error_unresolved(
                member.span,
                format!(
                    "actor `{instance}` serves effect `{effect_name}`, which has no \
                     member named `{}`",
                    member.name
                ),
                &member.name,
            );
            return Ty::Unknown;
        };
        if members.len() > 1 {
            // [effect-member-overload] Which overload a call resolved to, for
            // the emitters: they name an overloaded member positionally.
            if let Some(idx) = crate::effect_member_index(effect, target) {
                self.out.effect_member_calls.insert(self.key(span), idx);
            }
        }
        // [actor-send-fn] Only a send member can be reached through an addr in
        // the first pass: an ordinary member answers, and answering across a
        // actor boundary is the call sugar that comes later (with it, the
        // named question of whether an ordinary member may be actor-backed
        // at all).
        if !target.is_send {
            self.error(
                span,
                format!(
                    "`{}` is not a `send fn`, so it cannot be called through a \
                     `{ADDR_TYPE}`: an actor serves messages, and a member that \
                     answers would have to park its caller",
                    member.name
                ),
            );
        }
        self.record_def_ref(member.span, &member.name);
        let saved = self.enter_generics(&effect.generics);
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        let param_tys: Vec<Ty> = params.iter().map(|p| self.lower_type(&p.ty)).collect();
        self.generics = saved;
        // The instance's arguments bind the effect's own generics.
        let subst: HashMap<String, Ty> = match instance.strip_quals() {
            Ty::Named { args: iargs, .. } => effect
                .generics
                .iter()
                .map(|g| g.name.clone())
                .zip(iargs.iter().cloned())
                .collect(),
            _ => HashMap::new(),
        };
        let generic_set: HashSet<String> =
            effect.generics.iter().map(|g| g.name.clone()).collect();
        if args.len() != param_tys.len() {
            for a in args {
                self.check_expr(a, None);
            }
            self.error(
                span,
                format!(
                    "`{}` takes {} argument(s), found {}",
                    member.name,
                    param_tys.len(),
                    args.len()
                ),
            );
            return Ty::none();
        }
        for (i, a) in args.iter().enumerate() {
            let want = substitute_vars(&param_tys[i], &subst, &generic_set);
            let got = self.check_expr(a, Some(&want));
            if !got.is_unknown() && !is_subtype(&got, &want) {
                self.error(
                    a.span(),
                    format!("expected `{want}`, found `{got}`"),
                );
            }
            let repr = self.repr_of(a, &got);
            self.maybe_coerce(a.span(), &got, &repr, &want);
            // A payload crosses the seam: the sender gives it up
            // [deduce-consume]. This is what makes a reply token's linearity
            // discharge by sending it to an actor.
            self.fate_move(a, "send", "a send to an actor", a.span());
        }
        self.out.addr_calls.insert(self.key(span), instance.clone());
        // [actor-deadlock-cycle] A send from inside a member body is an edge
        // in the wait-for graph: it blocks while the target's bounded mailbox
        // is full. Recorded with the handler doing the sending, so the graph
        // pass can key it on the protocol that handler serves.
        if let (Some(h), Ty::Named { name, .. }) = (self.own_handler, instance.strip_quals()) {
            self.out
                .actor_sends
                .push((h.name.name.clone(), name.clone(), self.key(span)));
        } else if let (Some(task), Ty::Named { name, .. }) =
            (self.own_task.clone(), instance.strip_quals())
        {
            // [task-mint] A send from a *task* body: the graph attributes it to
            // every actor whose mints reach this task (FC-6's conservative
            // tracing), so it is recorded against the `send fn` here.
            self.out
                .task_sends
                .push((task, name.clone(), self.key(span)));
        }
        // [actor-send-fn] A send answers nothing.
        Ty::none()
    }

    /// [actor-spawn-expr] `spawn H(args) use D(...), addr on POOL` — the
    /// asynchronous binding of a handler. Almost every rule here is a rule
    /// `use` already has, moved to the spawn site: the same handler
    /// construction, the same dependency resolution, the same
    /// argument-is-stored consumption. What differs is *where* the
    /// dependencies come from — the spawn's own `use` clause, because a
    /// handler never crosses into an actor (only construction does) — and
    /// that the result is a value: the child's `Addr<E>`.
    ///
    /// [actor-mailbox] The mailbox bound is **not** a clause here: it is the
    /// handler's own slot (user decision 2026-09-16), so a spawn says what to
    /// run, what it depends on and where — and the queue depth is stated once,
    /// by the author who knows the protocol.
    ///
    /// [main-pool] The `on` clause is optional: omitted, the child runs on
    /// the pool current where the spawn was written, which is how a spawn
    /// names the main pool without new vocabulary. [waitfor-dedicated] A
    /// `Dedicated Pool` placement is **consumed** here — one thread, one
    /// occupant, enforced by the move.
    fn check_spawn(
        &mut self,
        handler: &'p Expr,
        uses: &'p [Expr],
        pool: Option<&'p Expr>,
        span: Span,
    ) -> Ty {
        let mut dedicated_placement = false;
        if let Some(pool) = pool {
            let pool_ty = self.check_expr(pool, Some(&Ty::named(POOL_TYPE)));
            if !pool_ty.is_unknown() && !is_subtype(pool_ty.strip_quals(), &Ty::named(POOL_TYPE)) {
                self.error(
                    pool.span(),
                    format!(
                        "a spawn runs on a `{POOL_TYPE}`, found `{pool_ty}`: `on pool(2)` \
                         builds one"
                    ),
                );
            }
            // [waitfor-dedicated] A dedicated thread has exactly one
            // occupant, and the `on` clause is what spends it: the placement
            // is consumed, so a second spawn onto the same `thread()` is the
            // ordinary use-after-move diagnostic rather than a rule of its
            // own.
            if pool_ty.quals().iter().any(|q| q.name == DEDICATED_QUALIFIER) {
                dedicated_placement = true;
                self.fate_move(pool, "spawn on", "this spawn", pool.span());
            }
        }
        // [actor-spawn-effect] The capability gate. Reported once, at the
        // spawn, and the rest of the form is still checked so a program with
        // a missing capability gets its other errors too.
        if !self.can_spawn {
            let msg = if self.in_lambda() {
                "`spawn` cannot be written inside a lambda: a function value's \
                 body runs wherever it is called, and a fn type cannot declare \
                 `spawn` — spawn in the function that holds the capability and \
                 pass the `Addr`"
            } else {
                "`spawn` requires the `spawn` capability in the function's effect \
                 list (`[use, spawn]`)"
            };
            self.error(span, msg);
        }
        let Some((id, args, written_type_args)) = self.handler_construction(handler, "spawn")
        else {
            return Ty::Unknown;
        };
        // [waitfor-dedicated] The grant check, and the reason the placement is
        // typed at all: a handler that may occupy its thread must own one. An
        // omitted `on` is refused here rather than inherited — the current
        // pool is a plain `Pool` (the main pool included), and inheriting a
        // dedicated one would put a second occupant on it.
        if self
            .scope
            .handlers
            .get(id.name.as_str())
            .is_some_and(|h| {
                h.effects
                    .iter()
                    .flatten()
                    .any(|e| matches!(e, EffectRef::WaitFor(_)))
            })
            && !dedicated_placement
        {
            self.error(
                pool.map_or(span, |p| p.span()),
                format!(
                    "`{}` declares `[waitfor]`, so it may occupy its thread until an \
                     answer arrives and must run on a thread of its own: place it \
                     `on thread()`, which answers a `{DEDICATED_QUALIFIER} \
                     {POOL_TYPE}` and is consumed by this spawn. A shared `pool(n)` \
                     would let one wait stall every actor on it, and the pool \
                     current here — `main`'s included — is a shared one",
                    id.name
                ),
            );
        }
        let Some((effects, declared)) =
            self.check_handler_construction(id, args, written_type_args, "spawn", span)
        else {
            for u in uses {
                self.check_spawn_dep(u);
            }
            return Ty::Unknown;
        };
        // [effect-handler-deps] The child's dependencies, supplied here
        // rather than inherited: each clause item is a handler construction
        // (built on the child) or an `Addr` (an effect another actor serves).
        // [actor-effect-kind] [monitor-handler] The spawn form serves two
        // kinds, split by the faces' kind. Every face an actor effect: the
        // actor spawn — a mailbox, a scheduler index, an addr per face. Every
        // face a *plain* effect: the **monitor spawn** (SH-3, user decision
        // 2026-09-19) — one shared instance behind a lock, reached through the
        // same `Addr<E>` type, its members running on the callers' threads.
        // A mixture is refused face-by-face by the actor path's kind check.
        let plain_faces = !effects.is_empty()
            && effects.iter().all(|effect| {
                matches!(effect.strip_quals(), Ty::Named { name, .. }
                    if self.scope.effects.get(name.as_str()).is_some_and(|d| !d.is_actor))
            });
        if plain_faces {
            // [mixed-handler] Send members split the plain-face spawn in two:
            // with them the handler is mixed (a servant + a façade), without
            // them it is a monitor (a lock).
            let mixed = self
                .scope
                .handlers
                .get(id.name.as_str())
                .is_some_and(|h| h.fns.iter().any(|f| f.is_send));
            if mixed {
                self.check_mixed_spawn(id, &effects, span);
            } else {
                self.check_monitor_spawn(id, &effects, pool, span);
            }
        } else {
            for effect in &effects {
                self.require_actor_effect(effect, span, "spawn");
            }
        }
        // The clause item's own index travels with the effect it supplies:
        // the matching below drains this list, so the position in it is not
        // the position the program wrote.
        let mut supplied: Vec<(Ty, Span, usize)> = Vec::new();
        for (at, u) in uses.iter().enumerate() {
            if let Some(eff) = self.check_spawn_dep(u) {
                supplied.push((eff, u.span(), at));
            }
        }
        let mut resolved: Vec<Ty> = Vec::new();
        let mut items: Vec<usize> = Vec::new();
        for want in &declared {
            let found = supplied
                .iter()
                .position(|(eff, _, _)| eff == want)
                .or_else(|| {
                    supplied
                        .iter()
                        .position(|(eff, _, _)| unify(want, eff, &mut HashMap::new()))
                });
            match found {
                Some(i) => {
                    let (eff, _, at) = supplied.remove(i);
                    resolved.push(eff);
                    items.push(at);
                }
                None => self.error(
                    span,
                    format!(
                        "handler `{}` depends on effect `{want}`, which this spawn does \
                         not supply: a spawned handler's dependencies come from its own \
                         `use` clause (`spawn {}(...) use SomeHandler() on \
                         pool(1)`), never from the spawning scope",
                        id.name, id.name
                    ),
                ),
            }
        }
        // Anything left over was supplied for nothing — a mistake worth
        // naming, since the reader believes it is being used.
        for (eff, sp, _) in &supplied {
            self.error(
                *sp,
                format!(
                    "handler `{}` does not depend on effect `{eff}`, so supplying it \
                     here has no effect",
                    id.name
                ),
            );
        }
        if !resolved.is_empty() {
            self.out.spawn_deps.insert(self.key(span), resolved);
            self.out.spawn_dep_items.insert(self.key(span), items);
        }
        self.out.spawn_effects.insert(self.key(span), effects.clone());
        // [effect-handler-multi] One addr per implemented effect: a single face
        // answers the bare `Addr<E>` it always did, and several answer a tuple
        // in declaration order — which is how least authority falls out of the
        // types, since a holder of one face cannot name the other's members.
        let mut addrs: Vec<Ty> = effects
            .into_iter()
            .map(|effect| Ty::Named {
                name: ADDR_TYPE.to_string(),
                args: vec![effect],
            })
            .collect();
        if addrs.len() == 1 {
            addrs.pop().unwrap_or(Ty::Unknown)
        } else {
            Ty::Tuple(addrs)
        }
    }

    /// [monitor-handler] The monitor spawn (SH-3, user decision 2026-09-19):
    /// `spawn H(args)` where every face of `H` is a **plain** effect answers a
    /// shared instance behind a lock — reached through the same `Addr<E>`
    /// type, bound with `use addr` like any handle, its members running on
    /// the callers' threads under mutual exclusion.
    ///
    /// The restriction *is* the design (§4 of the retired working document):
    /// a monitor's members are pure state transformation — no effects, no
    /// waits — which makes the lock innermost by construction, so no thread
    /// ever holds it while wanting anything else. One declaration-level check
    /// carries the whole rule: **no dependency list**. With no dependencies
    /// there is nothing to perform, `use`/`spawn`/`waitfor` are capabilities
    /// the members then cannot hold, and a call to an effectful helper fails
    /// effect resolution — the existing discipline enforces the body
    /// restrictions transitively.
    fn check_monitor_spawn(
        &mut self,
        id: &Ident,
        effects: &[Ty],
        pool: Option<&'p Expr>,
        span: Span,
    ) {
        let Some(decl) = self.scope.handlers.get(id.name.as_str()).copied() else {
            return;
        };
        // One face only, for now: one instance behind several effect types
        // needs a representation neither backend has yet (Rust would want one
        // `Arc<Mutex<dyn E1 + E2>>`, and trait objects have one principal
        // trait). Refused rather than guessed [backend-never-wrong].
        if effects.len() > 1 {
            let faces: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
            self.error(
                span,
                format!(
                    "handler `{}` implements several plain effects ({}), and a shared \
                     instance behind several faces is not supported yet: split the \
                     handler, or give it a single face",
                    id.name,
                    faces.join(", ")
                ),
            );
        }
        // A monitor runs on its callers' threads: there is nothing to place.
        if let Some(pool) = pool {
            self.error(
                pool.span(),
                format!(
                    "`{}` implements a plain effect, so this spawn shares it as a \
                     monitor — its members run on the callers' threads, and there \
                     is nothing to place `on` a pool. Remove the `on` clause",
                    id.name
                ),
            );
        }
        // The restriction: no dependencies, which is "no effects and no
        // waits" stated once at the declaration.
        if decl.effects.iter().flatten().next().is_some() {
            self.error(
                span,
                format!(
                    "`{}` declares dependencies, so it cannot be shared as a monitor: \
                     a shared handler of a plain effect runs its members under a lock, \
                     and they are restricted to state and pure computation — no \
                     effects, no waits. Keep it `use`-bound in one scope, or give it \
                     an `actor effect` face and a mailbox",
                    id.name
                ),
            );
        }
        // A `send fn` member routes the spawn to the mixed path before this
        // check, so reaching one here is an internal inconsistency.
        if let Some(f) = decl.fns.iter().find(|f| f.is_send) {
            self.error(
                span,
                format!(
                    "internal: monitor spawn of `{}` with send member `{}` — the \
                     mixed path should have taken it",
                    id.name, f.name.name
                ),
            );
        }
        // [actor-sendable] The instance crosses to every thread that binds
        // the handle, so what it holds must be sendable: constructor
        // parameters and state fields alike.
        for p in &decl.params {
            let ty = self.lower_type(&p.ty);
            if let Some(why) = self.unsendable_reason(&ty) {
                self.error(
                    span,
                    format!(
                        "`{}` cannot be shared: its constructor parameter `{}` is \
                         `{ty}` — {why}, and a shared handler's state must be \
                         sendable",
                        id.name, p.name.name
                    ),
                );
            }
        }
        for field in &decl.state {
            let ty = self.lower_type(&field.ty);
            if let Some(why) = self.unsendable_reason(&ty) {
                self.error(
                    span,
                    format!(
                        "`{}` cannot be shared: its state field `{}` is `{ty}` — \
                         {why}, and a shared handler's state must be sendable",
                        id.name, field.name.name
                    ),
                );
            }
        }
    }

    /// [mixed-handler] Whether a handler is **mixed** (SH-1, user decision
    /// 2026-09-19): every face a plain effect, and at least one `send fn`
    /// member. The send members and the state form the servant (an ordinary
    /// actor over a handler-local protocol); the sync members form the
    /// façade, running on the caller's thread.
    fn handler_is_mixed(&self, h: &ast::HandlerDecl) -> bool {
        if !h.fns.iter().any(|f| f.is_send) {
            return false;
        }
        !h.of.is_empty()
            && h.of.iter().all(|of| {
                let name = match of {
                    ast::Type::Named { base, .. } => base.name.name.as_str(),
                    ast::Type::QualifiedGroup { base, .. } => match base.as_ref() {
                        ast::Type::Named { base, .. } => base.name.name.as_str(),
                        _ => return false,
                    },
                    _ => return false,
                };
                self.scope
                    .effects
                    .get(name)
                    .is_some_and(|e| !e.is_actor)
            })
    }

    /// [mixed-handler] [free-send-fn] A handler-local `send fn` carries the
    /// free send fn's obligations, because no effect declaration mirrors it:
    /// an explicit all-consumed deduction clause (a scheduled body outlives
    /// the frame that enqueued it, so what it is given is always consumed),
    /// no `Mut` parameter, no generics, and every parameter sendable
    /// [actor-sendable]. And one restriction of its own: no overloading —
    /// the servant's message dispatch is by member name.
    fn check_local_send_member(&mut self, h: &'p ast::HandlerDecl, f: &'p FnDecl) {
        let has_params = f.params.iter().any(|p| !p.implicit);
        if f.deductions.is_none() && has_params {
            self.error(
                f.name.span,
                format!(
                    "`send fn {}` implements no effect member, so its deduction clause \
                     must be written out: a scheduled body outlives the frame that \
                     enqueued it, so every parameter is consumed — `=> {}`",
                    f.name.name,
                    f.params
                        .iter()
                        .filter(|p| !p.implicit)
                        .map(|p| format!("!{}", p.name.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
        for d in f.deductions.iter().flatten() {
            if matches!(d.kind, ast::DeductionKind::Moved) {
                continue;
            }
            let Some(name) = d.param_name() else { continue };
            self.error(
                d.span,
                format!(
                    "`send fn {}` cannot keep `{}`: a scheduled body outlives the \
                     frame that enqueued it, so every parameter is consumed (`=> !{}`)",
                    f.name.name, name.name, name.name
                ),
            );
        }
        if let Some(g) = f.generics.first() {
            self.error(
                f.name.span,
                format!(
                    "`send fn {}` cannot be generic: a message carries payloads and no \
                     type arguments, so `{}` would have nothing to be chosen by",
                    f.name.name, g.name
                ),
            );
        }
        let inner = self.enter_generics(&f.generics);
        for p in &f.params {
            if p.implicit {
                continue;
            }
            if let ast::Type::Named { qualifiers, .. } = &p.ty {
                if let Some(q) = qualifiers.iter().find(|q| q.name.name == "Mut") {
                    self.error(
                        q.span,
                        format!(
                            "`send fn {}` cannot take `Mut` parameter `{}`: mutating a \
                             value across a seam would share what the servant owns alone",
                            f.name.name, p.name.name
                        ),
                    );
                }
            }
            let ty = self.lower_type(&p.ty);
            if let Some(why) = self.unsendable_reason(&ty) {
                self.error(
                    p.ty.span(),
                    format!(
                        "`send fn {}` cannot carry `{ty}`: {why}, and everything crossing \
                         to the servant must be sendable",
                        f.name.name
                    ),
                );
            }
        }
        self.generics = inner;
        if h.fns
            .iter()
            .filter(|other| other.is_send && other.name.name == f.name.name)
            .count()
            > 1
        {
            self.error(
                f.name.span,
                format!(
                    "`send fn {}` is overloaded, which a servant protocol does not \
                     support yet: the message dispatch is by member name — rename one",
                    f.name.name
                ),
            );
        }
    }

    /// [mixed-handler] A bare call inside a façade member that names one of
    /// the handler's own `send fn` members: a **send to the servant**. Typed
    /// like an addr send — every argument consumed (the payload crosses),
    /// answering nothing — and recorded for the emitters, which lower it to
    /// an enqueue on the façade value's addr. Answers `None` when the callee
    /// is not a local send member, so ordinary resolution proceeds.
    fn check_facade_send(&mut self, id: &Ident, args: &'p [Expr], span: Span) -> Option<Ty> {
        let h = self.facade_handler?;
        let target = h
            .fns
            .iter()
            .find(|f| f.is_send && f.name.name == id.name)?;
        self.record_def_ref(id.span, &id.name);
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        let param_tys: Vec<Ty> = params.iter().map(|p| self.lower_type(&p.ty)).collect();
        if args.len() != param_tys.len() {
            for a in args {
                self.check_expr(a, None);
            }
            self.error(
                span,
                format!(
                    "`{}` takes {} argument(s), found {}",
                    id.name,
                    param_tys.len(),
                    args.len()
                ),
            );
            return Some(Ty::none());
        }
        for (i, a) in args.iter().enumerate() {
            let want = &param_tys[i];
            let got = self.check_expr(a, Some(want));
            if !got.is_unknown() && !is_subtype(&got, want) {
                self.error(a.span(), format!("expected `{want}`, found `{got}`"));
            }
            let repr = self.repr_of(a, &got);
            self.maybe_coerce(a.span(), &got, &repr, want);
            // The payload crosses to the servant: the façade gives it up
            // [deduce-consume] — which is also how a reply token minted by
            // the façade's `waitfor` discharges.
            self.fate_move(a, "send", "a send to the servant", a.span());
        }
        self.out.facade_sends.insert(self.key(span), id.name.clone());
        Some(Ty::none())
    }

    /// [mixed-handler] The mixed spawn: `spawn H(args)`, every face plain,
    /// send members present. It answers the same `Addr<E>` a monitor spawn
    /// answers — the façade value: the servant's addr plus the constructor
    /// parameters plus dispatch to the sync bodies. The servant is an
    /// ordinary actor, so the `on` clause stays (optional, inheriting the
    /// current pool), unlike a monitor's.
    fn check_mixed_spawn(&mut self, id: &Ident, effects: &[Ty], span: Span) {
        let Some(decl) = self.scope.handlers.get(id.name.as_str()).copied() else {
            return;
        };
        if effects.len() > 1 {
            let faces: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
            self.error(
                span,
                format!(
                    "handler `{}` implements several plain effects ({}), and a façade \
                     behind several faces is not supported yet: split the handler, or \
                     give it a single face",
                    id.name,
                    faces.join(", ")
                ),
            );
        }
        // First-slice cut: the servant's dependencies want the dependent-
        // member machinery rerouted through the handler-local dispatch,
        // which is its own piece of work. Refused rather than half-built.
        if decl.effects.iter().flatten().next().is_some() {
            self.error(
                span,
                format!(
                    "`{}` declares dependencies, which a mixed handler does not \
                     support yet: the servant's members would carry them, and that \
                     wiring is not built — take an `{ADDR_TYPE}` constructor parameter \
                     and send to it instead",
                    id.name
                ),
            );
        }
        // [actor-sendable] The façade value copies the constructor
        // parameters to every thread that binds the handle.
        for p in &decl.params {
            // A `Mut` parameter would be *shared* between the servant and
            // every façade copy on Kotlin and *cloned apart* on Rust — an
            // observable divergence, so it is refused rather than emitted
            // [backend-never-wrong].
            if let ast::Type::Named { qualifiers, .. } = &p.ty {
                if qualifiers.iter().any(|q| q.name.name == "Mut") {
                    self.error(
                        span,
                        format!(
                            "`{}` cannot be shared: its constructor parameter `{}` is \
                             `Mut`, and the façade value carries a copy of every \
                             parameter — a mutable one would go two ways at once. \
                             Move the mutable state into a field (the servant owns \
                             it), or pass immutable data",
                            id.name, p.name.name
                        ),
                    );
                }
            }
            let ty = self.lower_type(&p.ty);
            if let Some(why) = self.unsendable_reason(&ty) {
                self.error(
                    span,
                    format!(
                        "`{}` cannot be shared: its constructor parameter `{}` is \
                         `{ty}` — {why}, and the façade value carries a copy of it \
                         to every thread that binds the handle",
                        id.name, p.name.name
                    ),
                );
            }
        }
    }

    /// [actor-spawn-expr] One item of a spawn's `use` clause: a handler
    /// construction, whose effect is what it implements, or a value of type
    /// `Addr<E>`, whose effect is `E`. Answers the effect it supplies.
    fn check_spawn_dep(&mut self, item: &'p Expr) -> Option<Ty> {
        // A name that a handler declares is a construction; anything else is
        // an expression, and an `Addr` is the only useful kind.
        let is_handler = match item {
            Expr::Ident(id) => self.scope.handlers.contains_key(id.name.as_str()),
            Expr::Call { callee, .. } => match callee.as_ref() {
                Expr::Ident(id) => self.scope.handlers.contains_key(id.name.as_str()),
                _ => false,
            },
            _ => false,
        };
        if is_handler {
            let (id, args, type_args) = self.handler_construction(item, "spawn")?;
            // A dependency constructed for the child: its dependencies are
            // the child's problem in turn, and a handler with any of its own
            // cannot be written in a spawn clause — there is no scope on the
            // child to resolve them from. Named where it is written.
            let (effects, deps) =
                self.check_handler_construction(id, args, type_args, "spawn", item.span())?;
            let effect = effects.first().cloned().unwrap_or(Ty::Unknown);
            // [effect-handler-multi] A multi-face handler *constructed* in a
            // spawn clause would have to supply two of the child's
            // dependencies from one instance, and the child owns what a clause
            // builds — so there is nothing to share it with. Two addrs of the
            // same actor are the shape that works, and they are two clause
            // items.
            if effects.len() > 1 {
                let faces: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
                self.error(
                    item.span(),
                    format!(
                        "handler `{}` implements several effects ({}), so it cannot be \
                         constructed in a spawn's `use` clause: the child would own it, \
                         and one instance cannot be two of its dependencies — spawn it \
                         separately and pass an `{ADDR_TYPE}` per face",
                        id.name,
                        faces.join(", ")
                    ),
                );
            }
            for dep in &deps {
                self.error(
                    item.span(),
                    format!(
                        "handler `{}` depends on effect `{dep}`, so it cannot be \
                         constructed in a spawn's `use` clause: give the child a \
                         `{ADDR_TYPE}` of an actor serving `{effect}` instead, or \
                         construct it inside the child with `use`",
                        id.name
                    ),
                );
            }
            return Some(effect);
        }
        let ty = self.check_expr(item, None);
        match addr_effect(&ty) {
            Some(effect) => Some(effect),
            None if ty.is_unknown() => None,
            None => {
                self.error(
                    item.span(),
                    format!(
                        "a spawn's `use` clause supplies handlers: write a handler \
                         construction (`SomeHandler(...)`) or a `{ADDR_TYPE}` of the \
                         effect an actor already serves, not a `{ty}`"
                    ),
                );
                None
            }
        }
    }

    /// [actor-replyto] `replyto k(captures)` — mint a continuation targeting
    /// member `k` of the **enclosing handler**. `k`'s parameters are the
    /// captures written here followed by one more: the answer, which is what
    /// the token carries. So `send fn arrived(id: Int, notices: List<Notice>)`
    /// minted as `replyto arrived(7)` yields a `Reply<List<Notice>>`.
    /// [task-mint] The **free `send fn`** a bare name in a mint resolves to,
    /// or `None` when there is none. Resolution is the language's ordinary
    /// scope ladder [fn-overload-scope]: the most specific rung that has a
    /// send-kind candidate wins, so a module's own `parse_row` takes precedence
    /// over an imported one without either being an error. A tie *within* one
    /// rung is refused rather than guessed — a mint has no argument types to
    /// discriminate an overload set by, since its captures are a prefix of the
    /// target's parameters.
    fn free_send_target(&mut self, member: &Ident, form: &str) -> Option<&'p FnDecl> {
        let entries = self.scope.fns.get(member.name.as_str())?;
        let best = entries
            .iter()
            .filter(|e| e.decl.is_send)
            .map(|e| e.rung)
            .max()?;
        let sends: Vec<&'p FnDecl> = entries
            .iter()
            .filter(|e| e.decl.is_send && e.rung == best)
            .map(|e| e.decl)
            .collect();
        if sends.len() > 1 {
            self.error(
                member.span,
                format!(
                    "`{form}` cannot tell which `send fn {}` it means: {} of them are visible \
                     from {}, and a mint carries only the captures — not enough to choose an \
                     overload. Rename one, or bring in only the one this scope means",
                    member.name,
                    sends.len(),
                    best.describe()
                ),
            );
            return None;
        }
        sends.first().copied()
    }

    /// [task-mint] [task-pool-inherit] `replyto k(captures) on POOL` where `k`
    /// is a free `send fn`: the mint that makes a plain function a citizen of
    /// the concurrent world (user decision 2026-09-17, FC-2 + FC-3).
    ///
    /// What differs from a member mint, and why each difference is a
    /// simplification rather than a special case: there is **no mailbox**, so
    /// no capacity is reserved and no deadlock edge is contributed
    /// [actor-deadlock-cycle]; there is **no enclosing handler**, so the mint
    /// is legal in any function; and there **is** a placement, because the
    /// continuation has to run somewhere — the pool current at the mint unless
    /// an `on` clause says otherwise, since whoever creates work pays for it.
    fn check_task_mint(
        &mut self,
        member: &Ident,
        target: &'p FnDecl,
        captures: &'p [Expr],
        gated: bool,
        pool: Option<&'p Expr>,
        span: Span,
    ) -> Ty {
        // A gate is a mailbox policy, and a task has no mailbox: there is
        // nothing to hold back while the answer is outstanding.
        if gated {
            self.error(
                span,
                format!(
                    "`replyto!` gates the minting actor's mailbox until the answer arrives, \
                     and `{}` is a free `send fn` — a task has no mailbox to gate. Use a bare \
                     `replyto`",
                    member.name
                ),
            );
        }
        // [waitfor-dedicated] The typed exception to inheritance: a target that
        // may occupy its thread needs one of its own. Explicit `on thread()`,
        // or inherited from a minting frame that itself carries `[waitfor]` —
        // whose ambient pool is thereby provably dedicated.
        let target_waits = target
            .effects
            .iter()
            .flatten()
            .any(|e| matches!(e, EffectRef::WaitFor(_)));
        let mut dedicated = false;
        if let Some(p) = pool {
            let pool_ty = self.check_expr(p, Some(&Ty::named(POOL_TYPE)));
            if !pool_ty.is_unknown() && !is_subtype(pool_ty.strip_quals(), &Ty::named(POOL_TYPE)) {
                self.error(
                    p.span(),
                    format!(
                        "a continuation runs on a `{POOL_TYPE}`, found `{pool_ty}`: `on \
                         pool(2)` builds one, `on thread()` a dedicated one"
                    ),
                );
            }
            if pool_ty.quals().iter().any(|q| q.name == DEDICATED_QUALIFIER) {
                dedicated = true;
                self.fate_move(p, "run on", "this mint", p.span());
            }
        }
        if target_waits && !dedicated && !self.can_wait {
            self.error(
                span,
                format!(
                    "`{}` declares `[waitfor]`, so it may occupy the thread it runs on and \
                     needs one of its own: write `on thread()`. Inheriting this frame's pool \
                     is allowed only where the frame itself declares `[waitfor]`, which \
                     proves its pool is a `{DEDICATED_QUALIFIER} {POOL_TYPE}`",
                    member.name
                ),
            );
        }
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        if params.len() != captures.len() + 1 {
            for c in captures {
                self.check_expr(c, None);
            }
            self.error(
                span,
                format!(
                    "`{}` takes {} parameter(s): the last is the answer the token carries and \
                     the {} before it are the captures, so `replyto` here wants {} \
                     capture(s), found {}",
                    member.name,
                    params.len(),
                    params.len().saturating_sub(1),
                    params.len().saturating_sub(1),
                    captures.len()
                ),
            );
            return Ty::Unknown;
        }
        let saved = self.enter_generics(&target.generics);
        let param_tys: Vec<Ty> = params.iter().map(|p| self.lower_type(&p.ty)).collect();
        self.generics = saved;
        for (i, c) in captures.iter().enumerate() {
            let want = &param_tys[i];
            let got = self.check_expr(c, Some(want));
            if !got.is_unknown() && !is_subtype(&got, want) {
                self.error(c.span(), format!("expected `{want}`, found `{got}`"));
            }
            // [actor-sendable] A capture crosses to another thread at the
            // crossing site, exactly as a spawn argument does.
            if !got.is_unknown() {
                if let Some(why) = self.unsendable_reason(&got) {
                    self.error(
                        c.span(),
                        format!(
                            "a capture of `{}` cannot be `{got}`: {why}, and a task's \
                             captures cross a thread boundary",
                            member.name
                        ),
                    );
                }
            }
            // Stored in the continuation, so a bare name moves
            // [deduce-consume].
            self.fate_move(c, "capture", "a `replyto`", c.span());
        }
        // [task-mint] Who minted it, for FC-6's tracing: a handler, or another
        // task (a task minting a task chains the attribution).
        if let Some(minter) = self
            .own_handler
            .map(|h| h.name.name.clone())
            .or_else(|| self.own_task.clone())
        {
            self.out.task_mints.push((minter, member.name.clone()));
        }
        if let Some(key) = self.scope.fns.get(member.name.as_str()).and_then(|es| {
            es.iter()
                .find(|e| std::ptr::eq(e.decl, target))
                .map(|e| e.key)
        }) {
            self.out.replyto_tasks.insert(self.key(span), key);
            // [lsp-definition] The mint names a function, so it is a reference
            // to it like any call.
            self.out.fn_refs.insert(self.key(member.span), key);
        }
        let answer = param_tys.last().cloned().unwrap_or(Ty::Unknown);
        Ty::Named {
            name: REPLY_TYPE.to_string(),
            args: vec![answer],
        }
    }

    fn check_replyto(
        &mut self,
        member: &Ident,
        captures: &'p [Expr],
        gated: bool,
        pool: Option<&'p Expr>,
        span: Span,
    ) -> Ty {
        let form = if gated { "replyto!" } else { "replyto" };
        // [mixed-handler] First-slice cut: a mixed handler's servant cannot
        // park continuations yet — the continuation machinery is keyed on
        // effect faces, and rerouting it through the handler-local protocol
        // arrives with the `defer` build (which is also what would *price*
        // the deferral). Refused rather than half-built.
        if self
            .own_handler
            .is_some_and(|h| self.handler_is_mixed(h))
        {
            self.error(
                span,
                format!(
                    "`{form}` inside a mixed handler is not supported yet: a deferred \
                     answer is the `defer` build's business — answer within the \
                     activation instead"
                ),
            );
            for c in captures {
                self.check_expr(c, None);
            }
            return Ty::Unknown;
        }
        // [task-mint] A **free `send fn`** is a legal target, and the mint is
        // then legal in any function — the answer needs no mailbox, because
        // the continuation is a detached task rather than an activation (user
        // decision 2026-09-17, FC-2). Resolved *after* the enclosing handler's
        // members, which keeps the existing rule exactly as it was.
        let member_of_own = self
            .own_handler
            .is_some_and(|h| h.fns.iter().any(|f| f.name.name == member.name));
        if !member_of_own {
            if let Some(target) = self.free_send_target(member, form) {
                return self.check_task_mint(member, target, captures, gated, pool, span);
            }
        }
        if let Some(p) = pool {
            self.check_expr(p, Some(&Ty::named(POOL_TYPE)));
            self.error(
                p.span(),
                format!(
                    "`{form}` targeting a member of this handler takes no `on` clause: the \
                     answer arrives on the actor's own mailbox, which is where the \
                     continuation belongs. A placement is written where the *actor* is \
                     spawned, or on a mint whose target is a free `send fn`"
                ),
            );
        }
        let Some(h) = self.own_handler else {
            for c in captures {
                self.check_expr(c, None);
            }
            let detail = if self.in_lambda() {
                " — and a lambda is not one: a function value runs wherever it is \
                 called, so it has no member for an answer to arrive at"
            } else {
                "; `main` gets its token from `waitfor` instead"
            };
            self.error(
                span,
                format!(
                    "`{form}` mints a continuation into the handler it is written in, \
                     so it is legal only inside a handler member{detail}"
                ),
            );
            return Ty::Unknown;
        };
        let Some(target) = h.fns.iter().find(|f| f.name.name == member.name) else {
            for c in captures {
                self.check_expr(c, None);
            }
            // [actor-replyto] A **remote** mint — `k` is a member of an
            // `actor effect` in scope rather than of this handler — is the
            // generalized form (decided — ROADMAP.md's sugar pass) and a
            // later slice: it makes the mint itself send-like, since capacity
            // has to be reserved in the *target's* queue. Named here rather
            // than reported as an unresolved name, with the workaround that
            // needs nothing new: a token is an ordinary linear value, so the
            // handler that owns `k` mints it and hands it over.
            let remote = self
                .scope
                .effect_members
                .get(member.name.as_str())
                .map(|ms| {
                    ms.iter()
                        .filter(|(e, _)| e.is_actor)
                        .map(|(e, _)| format!("`{}`", e.name.name))
                        .collect::<Vec<_>>()
                })
                .filter(|names| !names.is_empty());
            if let Some(mut names) = remote {
                names.sort();
                names.dedup();
                self.error(
                    member.span,
                    format!(
                        "`{form}` targets a member of the handler it is written in, and \
                         `{}` is a member of {} instead: minting toward another \
                         actor's member is not supported yet. Have the handler that \
                         owns `{}` mint the token and pass it here — a `Reply<T>` is an \
                         ordinary linear value",
                        member.name,
                        names.join(", "),
                        member.name
                    ),
                );
                return Ty::Unknown;
            }
            self.error_unresolved(
                member.span,
                format!(
                    "handler `{}` has no member `{}` for `{form}` to deliver to",
                    h.name.name, member.name
                ),
                &member.name,
            );
            return Ty::Unknown;
        };
        // A continuation is delivered by *sending*, so its target must be a
        // send member: an ordinary member would have to answer, and there is
        // nothing to answer to.
        if !target.is_send {
            self.error(
                member.span,
                format!(
                    "`{}` is not a `send fn`, so `{form}` cannot deliver to it: an \
                     answer arrives as a message",
                    member.name
                ),
            );
        }
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        if params.len() != captures.len() + 1 {
            for c in captures {
                self.check_expr(c, None);
            }
            self.error(
                span,
                format!(
                    "`{}` takes {} parameter(s): the last is the answer the token \
                     carries and the {} before it are the captures, so `{form}` here \
                     wants {} capture(s), found {}",
                    member.name,
                    params.len(),
                    params.len().saturating_sub(1),
                    params.len().saturating_sub(1),
                    captures.len()
                ),
            );
            return Ty::Unknown;
        }
        let saved = self.enter_generics(&h.generics);
        let param_tys: Vec<Ty> = params.iter().map(|p| self.lower_type(&p.ty)).collect();
        self.generics = saved;
        for (i, c) in captures.iter().enumerate() {
            let want = &param_tys[i];
            let got = self.check_expr(c, Some(want));
            if !got.is_unknown() && !is_subtype(&got, want) {
                self.error(
                    c.span(),
                    format!("expected `{want}`, found `{got}`"),
                );
            }
            // A capture travels with the continuation, so it is stored, not
            // borrowed [deduce-consume] — the same rule a `use` argument has.
            let moved_by = if gated {
                "a `replyto!`"
            } else {
                "a `replyto`"
            };
            self.fate_move(c, "capture", moved_by, c.span());
        }
        self.out
            .replyto_members
            .insert(self.key(span), member.name.clone());
        // [actor-replyto] This handler parks, so it may only be spawned: the
        // continuation needs a mailbox to arrive on and a dispatcher to run
        // it, and a synchronously bound instance has neither. Refused at the
        // `use` site (below), which is where the binding is chosen.
        self.parking_handlers.insert(h.name.name.clone());
        // [actor-deadlock-cycle] The gate is the wait-for graph's tail: while
        // this continuation is outstanding the actor serves only its answer,
        // so whatever this handler can send to must be able to answer without
        // waiting on *it*. A bare `replyto` leaves the mailbox open and
        // contributes nothing.
        if gated {
            self.out
                .actor_gates
                .push((h.name.name.clone(), self.key(span)));
        }
        let answer = param_tys
            .last()
            .cloned()
            .unwrap_or(Ty::Unknown);
        Ty::Named {
            name: REPLY_TYPE.to_string(),
            args: vec![answer],
        }
    }

    /// [waitfor-effect] `waitfor out: Reply<T> { ... }` — the bridge into the
    /// asynchronous world, legal wherever the `waitfor` capability was
    /// declared (user decision 2026-09-17, T-5(c)): it was never `main` that
    /// was special, only the fact that `main` owns its thread, and that is
    /// now what the rule says [waitfor-dedicated].
    ///
    /// The token is an ordinary **linear** local, so "the block must consume
    /// it" needs no rule of its own: [linear-obligation] reports a leak at
    /// the block's end. The expression's value is the token's payload.
    fn check_waitfor(
        &mut self,
        binding: &'p Ident,
        ty: &'p ast::Type,
        body: &'p Block,
        span: Span,
    ) -> Ty {
        if !self.can_wait {
            let msg = if self.in_lambda() {
                "`waitfor` cannot be written inside a lambda: it may occupy the \
                 thread it runs on, and a function value runs wherever it is \
                 called — a fn type cannot declare `waitfor`"
                    .to_string()
            } else {
                format!(
                    "`waitfor` may occupy the thread it runs on until an answer \
                     arrives, so the function must declare `[waitfor]` in its effect \
                     list. In an actor's member that is the handler's dependency \
                     list, and its spawn then needs `on thread()` — a \
                     `{DEDICATED_QUALIFIER} {POOL_TYPE}`; the alternative is to mint \
                     the token with `replyto` and let the continuation run"
                )
            };
            self.error(span, msg);
        }
        self.validate_type(ty);
        let token = self.lower_type(ty);
        let payload = match &token {
            Ty::Named { name, args } if name == REPLY_TYPE && args.len() == 1 => {
                args[0].clone()
            }
            other => {
                if !other.is_unknown() {
                    self.error(
                        ty.span(),
                        format!(
                            "`waitfor` mints a reply token, so its binding is a \
                             `{REPLY_TYPE}<T>`, not a `{other}`"
                        ),
                    );
                }
                Ty::Unknown
            }
        };
        // The token lives in a scope of its own: a `waitfor` is an
        // expression, so nothing after it may still hold the token.
        self.locals.push(HashMap::new());
        self.declare(binding, token);
        let (value, _) = self.check_branch_block(body, Vec::new());
        let _ = value;
        // [linear-obligation] The frame dies here, which is what reports a
        // token the block never sent to.
        self.check_linear_frame_drop();
        self.locals.pop();
        payload
    }

    fn check_try(&mut self, body: &'p Block, span: Span) -> Ty {
        let (Some(ok_qual), Some(thrown_qual)) = (
            self.core_qualifier(span, OK_QUALIFIER),
            self.core_qualifier(span, THROWN_QUALIFIER),
        ) else {
            // The diagnostic is reported by `core_qualifier`; check the
            // body so its own errors still surface.
            let _ = self.check_branch_block(body, Vec::new());
            return Ty::Unknown;
        };
        self.try_stack.push(TryCtx {
            entry_depth: self.locals.len(),
            sites: Vec::new(),
        });
        let (value_ty, _) = self.check_branch_block(body, Vec::new());
        let ctx = self.try_stack.pop().expect("try ctx pushed above");
        if ctx.sites.is_empty() {
            self.error(
                span,
                format!(
                    "nothing in this `try` block can throw, so it has no outcome \
                     to produce: drop the `try`, or call something that declares \
                     `[{THROW_EFFECT}<M>]`"
                ),
            );
            return Ty::Unknown;
        }
        // [try] `M` is the union of the message types performed in the
        // body, deduplicated in first-seen order; a single type stays bare.
        let mut messages: Vec<Ty> = Vec::new();
        for (_, ty) in &ctx.sites {
            if !messages.contains(ty) {
                messages.push(ty.clone());
            }
        }
        let message_ty = self.mk_union(messages);
        // Fill in each site's landing information now that `M` is known.
        for (site_span, site_message) in &ctx.sites {
            let arm = self.throw_message_arm(*site_span, site_message, &message_ty);
            if let Some(site) = self.out.may_throw.get_mut(&self.key(*site_span)) {
                site.delimiter = Some(span);
                site.target = message_ty.clone();
                site.arm = arm;
            }
        }
        // The body's value is the `Ok` arm; a body that always exits
        // (`return`, an unconditional throw) contributes `Ok None`.
        let value_ty = if matches!(value_ty, Ty::Nothing) {
            Ty::none()
        } else {
            value_ty
        };
        let ok_arm = value_ty.qualify(vec![ok_qual]);
        let thrown_arm = message_ty.qualify(vec![thrown_qual]);
        self.mk_union(vec![ok_arm, thrown_arm])
    }

    /// Resolves one of the two qualifier names the `try` intrinsic needs
    /// from the implicitly imported core [try]: `Ok` for the value arm,
    /// `Thrown` for the message arm. Absent means std is broken or not on
    /// the source path, which is worth saying out loud rather than
    /// producing a nameless union.
    fn core_qualifier(&mut self, span: Span, name: &str) -> Option<Qual> {
        let decl = self.qualifier_named(name);
        match decl {
            // Written as `Ok Int` / `Thrown Str`, so the qualifier's own
            // type argument stays implicit — exactly as `lower_quals`
            // produces it, which is what makes the outcome type equal to a
            // hand-written `Ok T | Thrown M`.
            Some(_) => Some(Qual {
                effect: false,
                name: name.to_string(),
                args: Vec::new(),
            }),
            None => {
                self.error(
                    span,
                    format!(
                        "`try` needs the `{name}` qualifier from the core library, \
                         which is not in scope (is `std` on the source path?)"
                    ),
                );
                None
            }
        }
    }

    // ================= path analysis =================

    /// Whether a block always leaves the enclosing construct — every path
    /// hits a `return`, `break`, `continue`, or a **diverging expression**
    /// — so its state never reaches the code *after* a branching construct
    /// [deduce-consume].
    ///
    /// [throw] is why this is type-aware: `throw(m)` is an ordinary call
    /// whose type is `Nothing`, so a branch ending in one exits exactly as
    /// a `return` does. Only sound *after* the block has been checked (the
    /// types it reads are the ones just recorded), which is where both
    /// callers stand.
    fn block_exits(&self, block: &Block) -> bool {
        block.stmts.iter().any(|stmt| match stmt {
            Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
            Stmt::Expr(e) => self.expr_exits(e),
            _ => false,
        })
    }

    fn expr_exits(&self, expr: &Expr) -> bool {
        if self.diverges(expr) {
            return true;
        }
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                else_block.as_ref().is_some_and(|b| self.block_exits(b))
                    && branches.iter().all(|(_, b)| self.block_exits(b))
            }
            Expr::When { branches, .. } => {
                !branches.is_empty() && branches.iter().all(|b| self.block_exits(&b.body))
            }
            // [when-condition] Exhaustive by construction: the `else` is
            // mandatory, so "every branch exits" is enough.
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => self.block_exits(else_block) && branches.iter().all(|(_, b)| self.block_exits(b)),
            // Nothing else exits its enclosing block by itself. Loops are
            // deliberately excluded (they may run zero times), a lambda
            // owns its own control flow, and a `try` body's exits are
            // caught by the delimiter. Listed rather than defaulted so a
            // new control-flow form has to be classified here
            // [fn-must-return].
            Expr::While { .. }
            | Expr::For { .. }
            | Expr::Lambda { .. }
            | Expr::Try { .. }
            // [actor-spawn-expr] [actor-replyto] [actor-waitfor] Each has a
            // value and none transfers control out of the enclosing block: a
            // `spawn` yields an `Addr`, a `replyto` a token, and a `waitfor`
            // yields what was sent to its token — whatever its block does.
            // [actor-self-send] `k@self` is a callee, so it is a leaf here.
            | Expr::SelfScoped { .. }
            | Expr::Spawn { .. }
            | Expr::ReplyTo { .. }
            | Expr::WaitFor { .. }
            | Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Str { .. }
            | Expr::Ident(_)
            | Expr::Field { .. }
            | Expr::TupleIndex { .. }
            | Expr::Call { .. }
            | Expr::Index { .. }
            | Expr::ArrayLit { .. }
            | Expr::SetLit { .. }
            | Expr::MapLit { .. }
            | Expr::Tuple { .. }
            | Expr::StructLit { .. }
            | Expr::Unary { .. }
            | Expr::Binary { .. }
            | Expr::Is { .. }
            | Expr::Widen { .. }
            | Expr::NonNull { .. }
            | Expr::IncDec { .. }
            | Expr::Spread { .. }
            | Expr::Scoped { .. }
            | Expr::EffectScoped { .. }
            | Expr::Error { .. } => false,
        }
    }

    /// [fn-must-return] Whether a block always returns from the enclosing
    /// fn. Same shape as `block_exits` without the loop exits — a `break`
    /// leaves a loop, not the function — and with the same [throw]
    /// divergence rule: a path that throws never falls off the end.
    fn block_returns(&self, block: &Block) -> bool {
        block.stmts.iter().any(|stmt| match stmt {
            Stmt::Return { .. } => true,
            Stmt::Expr(e) => self.expr_returns(e),
            _ => false,
        })
    }

    fn expr_returns(&self, expr: &Expr) -> bool {
        if self.diverges(expr) {
            return true;
        }
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                else_block.as_ref().is_some_and(|b| self.block_returns(b))
                    && branches.iter().all(|(_, b)| self.block_returns(b))
            }
            Expr::When { branches, .. } => {
                !branches.is_empty() && branches.iter().all(|b| self.block_returns(&b.body))
            }
            // [when-condition] Mandatory `else`, so the chain covers every
            // path [fn-must-return].
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                self.block_returns(else_block)
                    && branches.iter().all(|(_, b)| self.block_returns(b))
            }
            // Nothing else returns from the enclosing fn by itself — see
            // `expr_exits` for why each is listed rather than defaulted.
            Expr::While { .. }
            | Expr::For { .. }
            | Expr::Lambda { .. }
            | Expr::Try { .. }
            // [actor-spawn-expr] [actor-replyto] [actor-waitfor] Each has a
            // value and none transfers control out of the enclosing block: a
            // `spawn` yields an `Addr`, a `replyto` a token, and a `waitfor`
            // yields what was sent to its token — whatever its block does.
            // [actor-self-send] `k@self` is a callee, so it is a leaf here.
            | Expr::SelfScoped { .. }
            | Expr::Spawn { .. }
            | Expr::ReplyTo { .. }
            | Expr::WaitFor { .. }
            | Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Str { .. }
            | Expr::Ident(_)
            | Expr::Field { .. }
            | Expr::TupleIndex { .. }
            | Expr::Call { .. }
            | Expr::Index { .. }
            | Expr::ArrayLit { .. }
            | Expr::SetLit { .. }
            | Expr::MapLit { .. }
            | Expr::Tuple { .. }
            | Expr::StructLit { .. }
            | Expr::Unary { .. }
            | Expr::Binary { .. }
            | Expr::Is { .. }
            | Expr::Widen { .. }
            | Expr::NonNull { .. }
            | Expr::IncDec { .. }
            | Expr::Spread { .. }
            | Expr::Scoped { .. }
            | Expr::EffectScoped { .. }
            | Expr::Error { .. } => false,
        }
    }

    /// Whether the checker typed this expression as `Nothing` — it
    /// diverges, so nothing after it runs [type-any-nothing].
    fn diverges(&self, expr: &Expr) -> bool {
        matches!(
            self.out.ty_of(self.file_idx, expr.span()),
            Some(Ty::Nothing)
        )
    }

    /// `ty_transitively_mut` over written (AST) types — struct fields
    /// are declared syntactically [fate-move-mode].
    fn ast_type_mut(&self, ty: &ast::Type, visited: &mut HashSet<String>) -> bool {
        match ty {
            ast::Type::Named { qualifiers, base } => {
                if qualifiers.iter().any(|q| q.name.name == "Mut") {
                    return true;
                }
                if base.args.iter().any(|a| self.ast_type_mut(a, visited)) {
                    return true;
                }
                let Some(decl) = self.scope.structs.get(base.name.name.as_str()) else {
                    return false;
                };
                if !visited.insert(base.name.name.clone()) {
                    return false;
                }
                decl.fields
                    .iter()
                    .any(|f| self.ast_type_mut(&f.ty, visited))
            }
            ast::Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                qualifiers.iter().any(|q| q.name.name == "Mut") || self.ast_type_mut(base, visited)
            }
            ast::Type::Union { arms, .. } => arms.iter().any(|a| self.ast_type_mut(a, visited)),
            ast::Type::Tuple { elems, .. } => elems.iter().any(|e| self.ast_type_mut(e, visited)),
            ast::Type::Array { elem, .. } => self.ast_type_mut(elem, visited),
            ast::Type::Nullable { inner, .. } => self.ast_type_mut(inner, visited),
            ast::Type::Fn { .. } => false,
        }
    }

    /// Handles a whole-variable mutation event on `name` (a `Mut` call
    /// argument, a projection assignment through it, or `++`): mutating a
    /// derived variable is an error [fate-derived-readonly]; mutating a
    /// root poisons its derived variables [fate-poison].
    fn fate_mutation(&mut self, name: &str, span: Span) {
        self.fate_mutation_at(name, span, Some(&[]));
    }

    /// [fate-field-disjoint] `fate_mutation`, told *which projection* of
    /// the variable the mutation hits: `[]` is the whole variable (every
    /// derived value falls), `[.tags]` only what overlaps `p.tags`, `None`
    /// the conservative unknown.
    fn fate_mutation_at(&mut self, name: &str, span: Span, event_path: Option<&[Step]>) {
        // [iter-fn] The origin of an open `for` may not be mutated:
        // its machine reads it across suspensions.
        if let Some((_, subject_span)) = self
            .driven_origins
            .iter()
            .find(|(origin, _)| origin == name)
            .cloned()
        {
            let _ = subject_span;
            self.error(
                span,
                format!(
                    "`{name}` is being iterated, so it cannot be mutated here: the \
                     pass reads its origin as it goes, and what it would see has no \
                     answer both backends agree on. Mutate it before or after the \
                     loop, or iterate `copy({name})` to work from a snapshot"
                ),
            );
        }

        // [deduce-syntax] Record the invalidation against the enclosing
        // fn's parameter: mutation can falsify qualifiers the caller has
        // and this signature never mentions, so such a parameter may not
        // keep "everything else". Recorded for written lists too — that
        // is what the written-list validation checks.
        self.record_param_mutation(name);
        let Some(var) = self.lookup(name) else { return };
        let (id, links) = (var.id, var.links.clone());
        if !links.is_empty() {
            // [proj-infer] A variable whose every link is *held* is an owned
            // object that merely projects its roots (a pass over a list):
            // its own fields are its own, and its `proj` fields cannot be
            // written through, so mutating it cannot reach a root. It may be
            // advanced. A wholesale projection or a plain alias cannot
            // [proj-readonly].
            if links.iter().all(|l| l.held) {
                self.poison_derived(id, name, FateEvent::Mutated, span, event_path);
                return;
            }
            self.error_derived(span, "mutate", name, &links);
            return;
        }
        // Mutating a variable captured from outside an enclosing lambda:
        // the closure takes ownership at creation [fate-lambda].
        if !self.lambda_ctx.is_empty() {
            if let Some(frame) = self.frame_of_id(id) {
                self.mark_capture_mutated(frame, id);
            }
        }
        self.poison_derived(id, name, FateEvent::Mutated, span, event_path);
    }

    // ================= place narrowing [flow-place] =================

    /// The recorded narrowing of a projection place, if flow analysis has
    /// one [flow-place].
    fn place_narrow_entry(&self, place: &Place) -> Option<&PlaceNarrow> {
        let var = self.lookup(&place.root)?;
        var.place_narrows.iter().find(|n| n.path == place.path)
    }

    fn place_narrow(&self, place: &Place) -> Option<&Ty> {
        self.place_narrow_entry(place).map(|n| &n.narrowed)
    }

    /// Records a projection narrowing, returning the fact it displaced.
    fn set_place_narrow(&mut self, narrow: &Narrow) -> Option<PlaceNarrow> {
        let entry = PlaceNarrow {
            path: narrow.place.path.clone(),
            narrowed: narrow.narrowed.clone(),
            declared: narrow.declared.clone(),
        };
        let var = self.lookup_mut(&narrow.place.root)?;
        match var.place_narrows.iter().position(|n| n.path == entry.path) {
            Some(i) => Some(std::mem::replace(&mut var.place_narrows[i], entry)),
            None => {
                var.place_narrows.push(entry);
                None
            }
        }
    }

    fn drop_place_narrow(&mut self, place: &Place) {
        if let Some(var) = self.lookup_mut(&place.root) {
            var.place_narrows.retain(|n| n.path != place.path);
        }
    }

    /// Drops every projection narrowing an event on `place` could falsify
    /// [flow-place-invalidate]: the place itself and everything below it
    /// (writing `h.a` replaces the subtree), and everything *above* it
    /// (writing `h.a` mutates `h`, so a fact about `h` as a whole is no
    /// longer known to hold) — i.e. every overlapping place. Sibling
    /// facts (`h.b`) survive. Only *narrowing* facts are dropped here;
    /// fate/poison is `fate_mutation`'s job.
    fn invalidate_place_narrows(&mut self, place: &Place) {
        let path = place.path.clone();
        let root = place.root.clone();
        if let Some(var) = self.lookup_mut(&root) {
            var.place_narrows.retain(|n| {
                let other = Place {
                    root: root.clone(),
                    path: n.path.clone(),
                };
                let event = Place {
                    root: root.clone(),
                    path: path.clone(),
                };
                !event.overlaps(&other)
            });
        }
    }

    /// A mutation *through a projection* [fate-poison] [flow-place]: the
    /// fate event lands on every provenance root, and projection
    /// narrowings are invalidated for exactly the storage the mutation can
    /// reach. Every site that mutates through a non-identifier expression
    /// goes through here, so no invalidation site can be forgotten.
    fn fate_mutation_through(&mut self, expr: &Expr, span: Span) {
        let mut sources = Vec::new();
        Self::provenance(expr, &mut sources);
        let names: Vec<String> = sources.iter().map(|s| s.name.clone()).collect();
        // [fate-field-disjoint] The mutation lands on the projection the
        // argument names (`p.tags` → `[.tags]`), so only values derived
        // from an overlapping projection are poisoned. A chain the analysis
        // cannot spell gives `None` — conservative, as before.
        let place = Place::of_expr(expr);
        let event_path: Option<&[Step]> = place.as_ref().map(|p| p.path.as_slice());
        for name in &names {
            self.fate_mutation_at(name, span, event_path);
        }
        match &place {
            Some(place) => self.invalidate_place_narrows(place),
            // Not a place rooted in a variable (a call result, say): the
            // provenance roots are still mutated, so nothing projected out
            // of them survives.
            None => {
                for name in names {
                    self.invalidate_place_narrows(&Place::root(name));
                }
            }
        }
    }

    /// A mutation of a whole variable: every projection fact under it
    /// falls [flow-place-invalidate].
    fn fate_mutation_root(&mut self, name: &str, span: Span) {
        self.fate_mutation(name, span);
        self.invalidate_place_narrows(&Place::root(name));
    }

    /// Runs `f` with the given narrowings applied, restoring afterwards.
    /// [is-narrow-guard] Applies narrowing facts **permanently** — the
    /// `with_narrows` application half without the restore. Used for the
    /// guard idiom: when every branch of an `if` exits (`return`, `break`,
    /// `continue`, or a diverging call), the code after it is on the
    /// else-path, so the else-narrows of every condition hold there.
    ///
    /// A variable already consumed keeps that fact — a flow fact outranks a
    /// narrowing, exactly as the restore half has it [deduce-consume].
    fn install_narrows(&mut self, narrows: &[Narrow]) {
        for n in narrows {
            if n.place.is_root() {
                if let Some(var) = self.lookup_mut(&n.place.root) {
                    if matches!(var.narrowed, Ty::Nothing) {
                        continue;
                    }
                    var.narrowed = n.narrowed.clone();
                    if n.declared != var.declared {
                        var.widened = Some(n.declared.clone());
                    }
                }
            } else if self.lookup(&n.place.root).is_some() {
                self.set_place_narrow(n);
            }
        }
    }

    fn with_narrows<T>(&mut self, narrows: &[Narrow], f: impl FnOnce(&mut Self) -> T) -> T {
        let mut saved: Vec<(String, Ty)> = Vec::new();
        // Applied place facts, with the fact each one displaced
        // [flow-place].
        let mut saved_places: Vec<(Place, Ty, Option<PlaceNarrow>)> = Vec::new();
        // [qual-widen] Root narrows whose *view* differs from the variable's
        // declared type (a `^` widening) install it for the branch.
        let mut saved_views: Vec<(String, Option<Ty>)> = Vec::new();
        for n in narrows {
            if n.place.is_root() {
                // [linear-union-arm] Narrowing a linear union to a
                // non-linear arm discharges the obligation: an `Err Str`
                // never held the handle. A flow fact — it survives the
                // restore below, and the branch join settles the variable
                // only when every path did this (or consumed the value).
                let settles = !self.ty_own_linear(&n.narrowed)
                    && self
                        .lookup(&n.place.root)
                        .is_some_and(|v| self.ty_own_linear(&v.narrowed));
                if let Some(var) = self.lookup_mut(&n.place.root) {
                    saved.push((n.place.root.clone(), var.narrowed.clone()));
                    var.narrowed = n.narrowed.clone();
                    if settles {
                        var.linear_settled = true;
                    }
                    if n.declared != var.declared {
                        saved_views.push((n.place.root.clone(), var.widened.clone()));
                        var.widened = Some(n.declared.clone());
                    }
                }
            } else if self.lookup(&n.place.root).is_some() {
                let displaced = self.set_place_narrow(n);
                saved_places.push((n.place.clone(), n.narrowed.clone(), displaced));
            }
        }
        let result = f(self);
        for (name, view) in saved_views.into_iter().rev() {
            if let Some(var) = self.lookup_mut(&name) {
                var.widened = view;
            }
        }
        for (name, ty) in saved.into_iter().rev() {
            if let Some(var) = self.lookup_mut(&name) {
                // A value consumed while narrowed stays consumed: the
                // `is`-narrowing restore must not resurrect it — the
                // consumption is a flow fact, not part of the narrowing
                // [deduce-consume].
                if matches!(var.narrowed, Ty::Nothing) {
                    continue;
                }
                var.narrowed = ty;
            }
        }
        for (place, applied, displaced) in saved_places.into_iter().rev() {
            // The dual of the consumed-stays-consumed rule [flow-place]:
            // if the branch invalidated the fact we applied (an
            // assignment or a mutating call through a prefix), the
            // invalidation is a flow fact and must survive the restore.
            if self.place_narrow(&place) != Some(&applied) {
                continue;
            }
            match displaced {
                Some(previous) => {
                    self.set_place_narrow(&Narrow {
                        place,
                        narrowed: previous.narrowed,
                        declared: previous.declared,
                    });
                }
                None => self.drop_place_narrow(&place),
            }
        }
        result
    }

    /// Resets narrowing to the declared type for every variable assigned
    /// inside `block` (called after branching constructs). Projection
    /// facts fall with it [flow-place]: an assignment rebinds the variable,
    /// so nothing projected out of it is known any more.
    fn reset_assigned(&mut self, block: &Block) {
        let mut names = HashSet::new();
        collect_assigned(block, &mut names);
        for name in names {
            if let Some(var) = self.lookup_mut(&name) {
                var.narrowed = var.declared.clone();
                var.place_narrows.clear();
            }
        }
    }

    /// Snapshot of every local's flow state (narrowed type, fate links,
    /// poison), per scope frame — the basis for branch-aware merging of
    /// consumption/qualifier-removal narrowing [deduce-consume]
    /// [fate-link].
    fn snapshot_narrows(&self) -> NarrowSnapshot {
        self.locals
            .iter()
            .map(|frame| {
                frame
                    .iter()
                    .map(|(name, var)| {
                        (
                            name.clone(),
                            VarState {
                                narrowed: var.narrowed.clone(),
                                linear_settled: var.linear_settled,
                                links: var.links.clone(),
                                poison: var.poison.clone(),
                                consumed_by: var.consumed_by,
                                place_narrows: var.place_narrows.clone(),
                                moved_places: var.moved_places.clone(),
                            },
                        )
                    })
                    .collect()
            })
            .collect()
    }

    fn restore_narrows(&mut self, snap: &NarrowSnapshot) {
        for (frame, saved) in self.locals.iter_mut().zip(snap) {
            for (name, state) in saved {
                if let Some(var) = frame.get_mut(name) {
                    var.narrowed = state.narrowed.clone();
                    var.linear_settled = state.linear_settled;
                    var.links = state.links.clone();
                    var.poison = state.poison.clone();
                    var.consumed_by = state.consumed_by;
                    var.place_narrows = state.place_narrows.clone();
                    // [fate-partial-move] Restored like every other flow
                    // fact: this resets to the *entry* state before the
                    // next branch is checked, and the branches' exit
                    // states are union-merged in `merge_fallthrough`.
                    var.moved_places = state.moved_places.clone();
                }
            }
        }
    }

    /// Checks a loop body; when the body's exit state consumed or
    /// weakened any variable, re-checks it once with that exit state as
    /// the entry state, so back-edge flows surface — a use early in the
    /// body errors when a later statement consumed the value in the
    /// previous iteration [deduce-consume]. The re-check's diagnostics
    /// are deduplicated against already-reported ones (checking is
    /// deterministic, so pass one's errors re-derive identically); its
    /// value results are discarded.
    fn check_loop_body(
        &mut self,
        body: &'p Block,
        narrows: &[Narrow],
        bindings: Vec<Binding>,
    ) -> (Ty, Option<TailInfo>) {
        let entry = self.snapshot_narrows();
        let result = self.with_narrows(narrows, |c| c.check_branch_block(body, bindings.clone()));
        if self.snapshot_narrows() != entry {
            let seen: HashSet<(usize, Span, String)> = self
                .out
                .errors
                .iter()
                .map(|e| (e.file, e.span, e.message.clone()))
                .collect();
            let before = self.out.errors.len();
            let _ = self.with_narrows(narrows, |c| c.check_branch_block(body, bindings));
            let second_pass: Vec<FileDiagnostic> = self.out.errors.split_off(before);
            for e in second_pass {
                if !seen.contains(&(e.file, e.span, e.message.clone())) {
                    self.out.errors.push(e);
                }
            }
        }
        result
    }

    /// Merges the fall-through branch states of a branching construct
    /// into the current state [deduce-consume]. For each local: states
    /// that all agree win; a value consumed (`Nothing`) on *any*
    /// fall-through path stays consumed (maybe-moved is unusable, as in
    /// Rust); otherwise disagreeing states conservatively keep only the
    /// qualifiers common to all of them. Fate links union across branches
    /// (may-be-linked is linked [fate-link]); a poison reason from any
    /// consumed branch is kept for diagnostics [fate-poison]. An empty
    /// list (every branch exits) leaves the pre-branch state untouched —
    /// the consumption happened on paths that never reach the code after
    /// the construct.
    fn merge_fallthrough(&mut self, fallthrough: &[NarrowSnapshot], span: Span) {
        if fallthrough.is_empty() {
            return;
        }
        let mut linear_conflicts: Vec<String> = Vec::new();
        for (frame_idx, frame_snap) in fallthrough[0].iter().enumerate() {
            for name in frame_snap.keys() {
                let states: Vec<&VarState> = fallthrough
                    .iter()
                    .filter_map(|s| s.get(frame_idx).and_then(|f| f.get(name)))
                    .collect();
                if states.len() != fallthrough.len() {
                    continue;
                }
                let narrowed_states: Vec<&Ty> = states.iter().map(|s| &s.narrowed).collect();
                // [linear-obligation] The linear rule is the dual of
                // maybe-moved: an owned linear value consumed on *some*
                // fall-through paths but not all is dropped on the
                // remaining ones. [linear-union-arm] A path that narrowed
                // the value to a **non-linear arm** owes nothing there —
                // an `Err Str` never held the handle — so it counts as
                // settled, exactly like a consuming path.
                let all_settled;
                {
                    let settled = states
                        .iter()
                        .filter(|s| {
                            matches!(s.narrowed, Ty::Nothing)
                                || s.linear_settled
                                || !self.ty_own_linear(&s.narrowed)
                        })
                        .count();
                    all_settled = settled == states.len();
                    if settled > 0 && settled < states.len() {
                        let owes = self
                            .locals
                            .get(frame_idx)
                            .and_then(|f| f.get(name))
                            .is_some_and(|var| {
                                // Live-ness differs per path; check the
                                // rest of the ownership conditions.
                                self.inferred.is_some()
                                    && var.links.is_empty()
                                    && (!var.is_param || self.param_owned(name))
                                    && { self.ty_own_linear(&var.declared) }
                            });
                        if owes {
                            linear_conflicts.push(name.clone());
                        }
                    }
                }
                let joined = if narrowed_states.iter().all(|t| **t == *narrowed_states[0]) {
                    narrowed_states[0].clone()
                } else if narrowed_states.iter().any(|t| matches!(t, Ty::Nothing)) {
                    Ty::Nothing
                } else {
                    // Keep only qualifiers every path preserves.
                    let mut common: HashSet<String> = narrowed_states[0]
                        .quals()
                        .iter()
                        .map(|q| q.name.clone())
                        .collect();
                    for t in &narrowed_states[1..] {
                        let names: HashSet<String> =
                            t.quals().iter().map(|q| q.name.clone()).collect();
                        common.retain(|q| names.contains(q));
                    }
                    let removed: HashSet<String> = narrowed_states[0]
                        .quals()
                        .iter()
                        .map(|q| q.name.clone())
                        .filter(|q| !common.contains(q))
                        .collect();
                    narrowed_states[0].clone().remove_quals(&removed)
                };
                // Fate links union across branches [fate-link].
                let mut links: Vec<FateLink> = Vec::new();
                for s in &states {
                    for l in &s.links {
                        if !links.iter().any(|e| e.root_id == l.root_id) {
                            links.push(l.clone());
                        }
                    }
                }
                let (poison, consumed_by) = if matches!(joined, Ty::Nothing) {
                    (
                        states.iter().find_map(|s| s.poison.clone()),
                        states.iter().find_map(|s| s.consumed_by),
                    )
                } else {
                    (None, None)
                };
                // [flow-place] A projection narrowing survives the join
                // only when every fall-through path agrees on it exactly;
                // otherwise the place falls back to its declared type,
                // which is always a supertype. A consumed variable keeps
                // no facts about its parts.
                let place_narrows: Vec<PlaceNarrow> = if matches!(joined, Ty::Nothing) {
                    Vec::new()
                } else {
                    states[0]
                        .place_narrows
                        .iter()
                        .filter(|n| {
                            states[1..]
                                .iter()
                                .all(|s| s.place_narrows.iter().any(|m| *m == **n))
                        })
                        .cloned()
                        .collect()
                };
                // [fate-partial-move] Moved places **union** across the
                // join — the dual of `place_narrows`, and the same
                // direction as consumption: moved on any path is moved,
                // because the paths the compiler cannot distinguish must
                // all be safe.
                let mut moved_places: Vec<MovedPlace> = Vec::new();
                for s in &states {
                    for m in &s.moved_places {
                        if !moved_places.iter().any(|e| e.path == m.path) {
                            moved_places.push(m.clone());
                        }
                    }
                }
                if let Some(var) = self.locals.get_mut(frame_idx).and_then(|f| f.get_mut(name)) {
                    var.narrowed = joined;
                    // [linear-union-arm] Settled on every path means the
                    // obligation is gone after the join, whatever the
                    // joined type says.
                    if all_settled && states.iter().any(|s| s.linear_settled) {
                        var.linear_settled = true;
                    }
                    var.links = links;
                    var.poison = poison;
                    var.consumed_by = consumed_by;
                    var.place_narrows = place_narrows;
                    var.moved_places = moved_places;
                }
            }
        }
        linear_conflicts.sort();
        linear_conflicts.dedup();
        for name in linear_conflicts {
            self.error(
                span,
                format!(
                    "`{name}` owns a linear value that is consumed on some \
                     paths but not others; discharge or move it on every path"
                ),
            );
        }
    }

    // ================= type lowering =================

    fn lower_type(&mut self, ty: &ast::Type) -> Ty {
        let subst = HashMap::new();
        self.lower_type_subst(ty, &subst, 0)
    }

    fn lower_type_subst(
        &mut self,
        ty: &ast::Type,
        subst: &HashMap<String, Ty>,
        depth: usize,
    ) -> Ty {
        if depth > 32 {
            return Ty::Unknown;
        }
        match ty {
            ast::Type::Named { qualifiers, base } => {
                let lowered = self.lower_base_ref(base, subst, depth);
                let quals = self.lower_quals(qualifiers, subst, depth);
                lowered.qualify(quals)
            }
            ast::Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                let lowered = self.lower_type_subst(base, subst, depth);
                let quals = self.lower_quals(qualifiers, subst, depth);
                lowered.qualify(quals)
            }
            ast::Type::Union { arms, .. } => {
                let arms = arms
                    .iter()
                    .map(|a| self.lower_type_subst(a, subst, depth))
                    .collect();
                self.mk_union(arms)
            }
            ast::Type::Nullable { inner, .. } => {
                let inner = self.lower_type_subst(inner, subst, depth);
                self.mk_union(vec![inner, Ty::none()])
            }
            ast::Type::Tuple { elems, .. } => Ty::Tuple(
                elems
                    .iter()
                    .map(|e| self.lower_type_subst(e, subst, depth))
                    .collect(),
            ),
            ast::Type::Array { elem, .. } => {
                Ty::Array(Box::new(self.lower_type_subst(elem, subst, depth)))
            }
            ast::Type::Fn {
                params,
                ret,
                effects,
                ..
            } => Ty::Fn {
                contract: self.lower_fn_contract(ty),
                // [fn-effects] The effects a call of the value performs.
                effects: self.lower_fn_effects(effects.as_deref()),
                params: params
                    .iter()
                    .map(|p| self.lower_type_subst(p, subst, depth))
                    .collect(),
                ret: Box::new(self.lower_type_subst(ret, subst, depth)),
            },
        }
    }

    /// Builds a fn type's contract from its written parameter names and
    /// deduction list [fn-contract]. `None` when nothing is written (the
    /// default: keeps everything). Entries: kept unless the (explicit)
    /// deduction list omits the named parameter; kept quals per the list
    /// (bare name = all declared); `mutable` = the declared param type
    /// carries `Mut`.
    fn lower_fn_contract(&mut self, ty: &ast::Type) -> Option<Vec<FnParamContract>> {
        let ast::Type::Fn {
            params,
            param_names,
            deductions,
            ..
        } = ty
        else {
            return None;
        };
        if param_names.iter().all(|n| n.is_none()) && deductions.is_none() {
            return None;
        }
        let declared_quals = |t: &ast::Type| -> Vec<String> {
            match t {
                ast::Type::Named { qualifiers, .. }
                | ast::Type::QualifiedGroup { qualifiers, .. } => {
                    qualifiers.iter().map(|q| q.name.name.clone()).collect()
                }
                _ => Vec::new(),
            }
        };
        let entries: Vec<FnParamContract> = params
            .iter()
            .zip(param_names)
            .map(|(t, name)| {
                let declared = declared_quals(t);
                let mutable = declared.iter().any(|q| q == "Mut");
                let names = |items: &[ast::TypeRef]| -> Vec<String> {
                    items.iter().map(|q| q.name.name.clone()).collect()
                };
                let (kept, effect) = match (name, deductions) {
                    (Some(id), Some(list)) => {
                        match list
                            .iter()
                            .find(|d| d.param_name().is_some_and(|n| n.name == id.name))
                        {
                            Some(d) => match &d.kind {
                                ast::DeductionKind::KeepAll | ast::DeductionKind::Proj(_) => {
                                    (true, QualEffect::KeepAll)
                                }
                                ast::DeductionKind::Exhaustive(items) => {
                                    (true, QualEffect::Exhaustive(names(items)))
                                }
                                ast::DeductionKind::Remove(items) => {
                                    (true, QualEffect::Remove(names(items)))
                                }
                                ast::DeductionKind::Moved => {
                                    (false, QualEffect::Exhaustive(Vec::new()))
                                }
                            },
                            // [deduce-syntax] Unmentioned in a fn type's
                            // group: kept, the default.
                            None => (true, QualEffect::KeepAll),
                        }
                    }
                    // Named but no group, or unnamed: keeps everything.
                    _ => (true, QualEffect::KeepAll),
                };
                // [proj-infer] `=>[f] proj[from: c]` on a fn type: the only
                // way to say what a bodiless value's result holds.
                let lent = match (name, deductions) {
                    (Some(id), Some(list)) => list
                        .iter()
                        .filter_map(|d| d.proj_sources())
                        .flatten()
                        .any(|src| src.name == id.name),
                    _ => false,
                };
                FnParamContract {
                    name: name.as_ref().map(|id| id.name.clone()),
                    kept,
                    effect,
                    mutable,
                    lent,
                }
            })
            .collect();
        // Validate the group's entries name the fn type's parameters.
        if let Some(list) = deductions {
            for d in list {
                let mut named: Vec<&ast::Ident> = Vec::new();
                if let ast::DeductionTarget::Param { name, .. } = &d.target {
                    named.push(name);
                }
                if let Some(sources) = d.proj_sources() {
                    named.extend(sources.iter());
                }
                for n in named {
                    let known = param_names.iter().flatten().any(|p| p.name == n.name);
                    if !known {
                        self.error(
                            n.span,
                            format!(
                                "`{}` names no parameter of this function type \
                                 (name the parameter in the type: `({}: ...) -> ...`)",
                                n.name, n.name
                            ),
                        );
                    }
                }
            }
        }
        Some(entries)
    }

    fn lower_quals(
        &mut self,
        qualifiers: &[TypeRef],
        subst: &HashMap<String, Ty>,
        depth: usize,
    ) -> Vec<Qual> {
        qualifiers
            .iter()
            // [proj-type] `proj` is part of the lowered type (user decision
            // 2026-09-12): `proj Str` and `Str` are different types, ordered
            // `Str <: proj Str`, so a borrow inside a union arm or a container
            // is visible wherever the value flows. Its `[from: p]` is not: the
            // source is a fact about the value, carried by fate links.
            .map(|q| {
                // [lsp-definition] qualifier name -> its declaration.
                self.record_def_ref(q.name.span, &q.name.name);
                Qual {
                    // [fn-effects] A name in qualifier position that
                    // resolves to an *effect* is an effect claim on a
                    // producer, not a qualifier: same spelling, different
                    // rules (inverted variance, never dropped, never
                    // tested). Qualifier and effect names cannot collide
                    // [mod-collision], so the lookup decides.
                    effect: !self.scope.qualifiers.contains_key(q.name.name.as_str())
                        && self.scope.effects.contains_key(q.name.name.as_str()),
                    name: q.name.name.clone(),
                    args: q
                        .args
                        .iter()
                        .map(|a| self.lower_type_subst(a, subst, depth))
                        .collect(),
                }
            })
            .collect()
    }

    /// Lowers the base of a named type: substitutions, generic parameters,
    /// alias expansion, plain nominals.
    fn lower_base_ref(&mut self, base: &TypeRef, subst: &HashMap<String, Ty>, depth: usize) -> Ty {
        let name = base.name.name.as_str();
        if base.args.is_empty() {
            if let Some(bound) = subst.get(name) {
                return bound.clone();
            }
            if self.generics.contains(name) {
                return Ty::Var(name.to_string());
            }
        }
        // The written name points at its declaration [lsp-definition] —
        // recorded before alias expansion, so an alias use jumps to the
        // alias itself rather than its target.
        let name_span = base.name.span;
        self.record_def_ref(name_span, name);
        // [proj-type-arg] A `proj` type argument makes a *container of
        // borrows* (`List<proj T>` is a view, `Vec<&T>`). That is only
        // meaningful for the intrinsic containers the backends render that
        // way; on a user struct it would smuggle a borrow into a field
        // through the back door of generics, unseen by [proj-infer]'s
        // field-driven inference.
        if !matches!(name, "List") && self.scope.structs.contains_key(name) {
            for arg in &base.args {
                if let Some(span) = first_proj_span(arg) {
                    self.error(
                        span,
                        format!(
                            "`proj` cannot be a type argument of `{name}`: a struct's \
                             fields own their values, so it cannot hold a borrow \
                             through a generic parameter (only the intrinsic \
                             containers — `List`, arrays — may be views)"
                        ),
                    );
                }
            }
        }
        // [type-any-nothing] The written `Nothing` *is* the bottom type,
        // not a nominal type that happens to be called that: `throw`
        // declares `-> [] Nothing` and its callers must see a value that
        // fits everywhere and ends the path.
        if name == "Nothing" && base.args.is_empty() {
            return Ty::Nothing;
        }
        // [type-any-nothing] And the written `Any` *is* the top type, for the
        // same reason: a parameter declared `Any` accepts every value, and
        // overload ranking treats it as the broadest thing a parameter can
        // say [fn-overload-rank]. As a nominal `Named("Any")` it accepted
        // nothing at all, since unification compares names.
        if name == "Any" && base.args.is_empty() {
            return Ty::Any;
        }
        // [group-not-a-value] A `params` group is not a type; the refusal is
        // `reject_group_as_data`'s (validation), and the *lowering* is
        // `Unknown` so the one mistake does not cascade into type-mismatch
        // errors at every use of the annotated value [type-unknown-lenient].
        if !self.type_name_exists(name) && self.scope.param_groups.contains_key(name) {
            return Ty::Unknown;
        }
        let args: Vec<Ty> = base
            .args
            .iter()
            .map(|a| self.lower_type_subst(a, subst, depth))
            .collect();
        // Type aliases expand structurally (with generic substitution).
        if let Some(alias) = self.scope.type_aliases.get(name) {
            if let Some(target) = &alias.alias {
                let mut alias_subst = HashMap::new();
                for (g, arg) in alias.generics.iter().zip(&args) {
                    alias_subst.insert(g.name.clone(), arg.clone());
                }
                let target = target.clone();
                return self.lower_type_subst(&target, &alias_subst, depth + 1);
            }
        }
        Ty::Named {
            name: name.to_string(),
            args,
        }
    }

    /// Builds a union, recording the wrapper size when one is needed.
    fn mk_union(&mut self, arms: Vec<Ty>) -> Ty {
        let ty = Ty::union_of(arms);
        if ty.is_wrapper_union() {
            self.out.union_sizes.insert(ty.value_arms().len());
        }
        ty
    }

    // ================= name resolution =================

    /// Whether `name` is usable as a written *type* name [name-resolve]:
    /// a struct, an `intrinsic type`, a type alias, an effect
    /// (effect lists are written as types), or the language-level `None`
    /// (which has no declaration). Generic parameters are resolved by the
    /// caller, before this is consulted.
    fn type_name_exists(&self, name: &str) -> bool {
        name == "None"
            || self.scope.structs.contains_key(name)
            || self.scope.opaque_types.contains_key(name)
            || self.scope.type_aliases.contains_key(name)
            || self.scope.effects.contains_key(name)
    }

    /// Whether `name` is usable as a written *qualifier* name
    /// [name-resolve]: a declared qualifier, or one of the compiler's
    /// intrinsic ones (which have no declaration to find — their
    /// position-specific rules are enforced by `validate_quals`).
    fn qual_name_exists(&self, name: &str) -> bool {
        matches!(name, "Mut" | "Linear" | "linear" | "once" | "proj")
            || self.scope.is_qualifier(name)
            // [fn-effects] An effect claims what driving a producer
            // performs, and it is written where a qualifier goes. Where it
            // may be written is `validate_quals`' business; that it *is* a
            // name in this position is settled here.
            || self.scope.effects.contains_key(name)
    }

    /// Reports a written name in a type position that resolves to nothing
    /// visible [name-resolve]. `expect_qual` picks the wording and the
    /// namespace searched.
    fn require_name(&mut self, r: &TypeRef, expect_qual: bool) {
        let name = r.name.name.as_str();
        if self.generics.contains(name) {
            return;
        }
        let known = if expect_qual {
            self.qual_name_exists(name)
        } else {
            self.type_name_exists(name)
        };
        if known {
            return;
        }
        // A `params` group in type position is a *position* mistake with its
        // own diagnostic [group-not-a-value] (`reject_group_as_data`); an
        // "unknown type" here would be a second report of the same error.
        if !expect_qual && self.scope.param_groups.contains_key(name) {
            return;
        }
        // A `params` group in type position is a *position* mistake with its
        // own diagnostic [group-not-a-value] (`reject_group_as_data`); an
        // "unknown type" here would be a second report of the same error.
        if !expect_qual && self.scope.param_groups.contains_key(name) {
            return;
        }
        let what = if expect_qual { "qualifier" } else { "type" };
        // A name that exists in the *other* namespace is a position
        // mistake, not a missing declaration: say so instead of
        // suggesting an import that would not help.
        let hint = if expect_qual && self.type_name_exists(name) {
            format!(" (`{name}` is a type, not a qualifier)")
        } else if !expect_qual && self.qual_name_exists(name) {
            format!(" (`{name}` is a qualifier, not a type)")
        } else {
            String::new()
        };
        let name = name.to_string();
        self.error_unresolved(r.span, format!("unknown {what} `{name}`{hint}"), &name);
    }

    /// [effect-handler-deps] The effects a handler declares as dependencies
    /// (`handler Stamped [Logger, Clock] of Logger`), lowered and validated:
    /// they are supplied by the compiler at the `use` site, so the list is
    /// unnamed and every entry must be an effect a `use` could provide.
    /// `use` and `Throw` are refused — a handler registers nothing, and a
    /// throw needs a delimiter rather than a handler [throw].
    fn handler_dep_effects(&mut self, h: &'p ast::HandlerDecl) -> Vec<Ty> {
        let mut out: Vec<Ty> = Vec::new();
        for eff in h.effects.iter().flatten() {
            let r = match eff {
                EffectRef::Use(span) => {
                    self.error(
                        *span,
                        format!(
                            "handler `{}` cannot depend on `use`: a handler registers \
                             no handlers of its own",
                            h.name.name
                        ),
                    );
                    continue;
                }
                // [actor-spawn-effect] A handler *may* depend on `spawn` — a
                // supervisor spawns and re-spawns its children. It names no
                // effect type, so there is nothing to lower here; the gate
                // arrives with the `spawn` expression.
                EffectRef::Spawn(_) => continue,
                // [waitfor-effect] A handler *may* depend on `waitfor` — the
                // motivating customer is a test clock that asks a timer for
                // the time. It names no effect type either; what it costs is
                // paid at the spawn site, where the placement is chosen.
                EffectRef::WaitFor(_) => continue,
                EffectRef::Effect(r) => r,
            };
            if r.name.name == THROW_EFFECT {
                self.error(
                    r.span,
                    format!(
                        "handler `{}` cannot depend on `{THROW_EFFECT}`: a throw is \
                         delimited by a `try {{ ... }}` block, not supplied by a \
                         handler",
                        h.name.name
                    ),
                );
                continue;
            }
            let Some(ty) = self.lower_effect_ref(r) else {
                continue;
            };
            if out.contains(&ty) {
                self.error(
                    r.span,
                    format!("handler `{}` declares `{ty}` twice", h.name.name),
                );
                continue;
            }
            out.push(ty);
        }
        out
    }

    // ================= qualifier validation =================

    /// [effect-not-data] An effect names a *capability*, not a type of
    /// values: it may appear in a fn's effect list, in a handler's effect
    /// list and its `of` clause, and nowhere else. Using one as a struct
    /// field, parameter, return, or `let` annotation is an error — the value
    /// would have to be a handler instance, which only `use` produces, and
    /// neither backend can render it (Rust emits a bare trait, `E0782`).
    ///
    /// **One exception, sanctioned by decision** (user, 2026-09-15):
    /// `Addr<E>`'s type argument [actor-spawn-expr]. An addr is a handle to a
    /// actor, and what a holder may *do* with it is exactly the effect the
    /// actor serves — so the effect is what parameterizes the handle, and
    /// that is what makes an actor and a locally `use`d handler
    /// interchangeable behind one name. The exception is one type argument of
    /// one std type; an addr is still not a handler instance, and every other
    /// data position stays refused (checked in `validate_type`, which knows
    /// the enclosing type).
    /// [actor-spawn-expr] Whether this type argument is the effect argument
    /// of an `Addr` — the one place an effect names something in a type
    /// position. Deliberately narrow: the enclosing base must be std's
    /// `Addr`, the position must be its only type parameter, and the argument
    /// must be a bare name that a visible effect declares. Anything else
    /// (`Addr<Int>`, a second argument, an effect elsewhere) goes down the
    /// ordinary path and is validated — or refused — as before.
    /// [actor-effect-kind] The binding gate: `spawn` and `use addr` are the two
    /// ways an actor is reached, and both need an `actor effect`. This is what
    /// closes the design's carried named question — "may an ordinary member be
    /// actor-backed?" — as *forbid*, with the actor kind as the sanctioned
    /// spelling (user decision 2026-09-15).
    fn require_actor_effect(&mut self, effect: &Ty, span: Span, form: &str) {
        let Ty::Named { name, .. } = effect.strip_quals() else {
            return;
        };
        let name = name.clone();
        let Some(decl) = self.scope.effects.get(name.as_str()).copied() else {
            return;
        };
        if decl.is_actor {
            return;
        }
        self.error(
            span,
            format!(
                "`{form}` cannot bind `{name}` to an actor: it is a plain effect, and \
                 a synchronous protocol has no mailbox — declare `actor effect {name}` \
                 (its members then give up kept and `Mut` parameters, `proj` returns \
                 and non-sendable payloads)"
            ),
        );
    }

    /// [actor-effect-kind] [monitor-handler] The kind gate on an `Addr`'s
    /// argument, since SH-3 an *acceptance* of both kinds: an `Addr` of an
    /// actor effect is a mailbox handle (a spawn produced it), and an `Addr`
    /// of a **plain** effect is a shared monitor's handle (a monitor spawn
    /// produced it) — one type, two backing shapes, chosen by the effect's
    /// declared kind. What remains refused here is only an argument that is
    /// not an effect at all, which the general argument checks report.
    /// [actor-effect-kind] [monitor-handler] Naming `Addr<E>` accepts both
    /// effect kinds since SH-3 (user decision 2026-09-19): an actor effect's
    /// addr is a mailbox handle, and a plain effect's is a shared monitor's
    /// handle. The former per-kind refusal lived here; the walk above now
    /// documents the acceptance at the one call site it had.
    fn is_addr_effect_arg(&self, base: &TypeRef, index: usize, arg: &ast::Type) -> bool {
        if base.name.name != ADDR_TYPE || index != 0 || base.args.len() != 1 {
            return false;
        }
        // Std's `Addr`, not a program's own type of that name.
        if !self.scope.opaque_types.contains_key(ADDR_TYPE) {
            return false;
        }
        match arg {
            ast::Type::Named { qualifiers, base } => {
                qualifiers.is_empty()
                    && base.args.is_empty()
                    && self.scope.effects.contains_key(base.name.name.as_str())
            }
            _ => false,
        }
    }

    fn reject_effect_as_data(&mut self, r: &TypeRef) {
        let name = r.name.name.as_str();
        if self.generics.contains(name) || !self.scope.effects.contains_key(name) {
            return;
        }
        let name = name.to_string();
        self.error(
            r.span,
            format!(
                "`{name}` is an effect, not a data type: effects appear in a \
                 function's effect list (`[{name}]`) or a handler's `of` clause, \
                 and their handlers are reached with `use`"
            ),
        );
    }

    /// [group-not-a-value] A `params` group names an *obligation*, not a
    /// type of values: no value may ever have a group as its type. This is
    /// the single restriction that keeps the mechanism a where-clause
    /// rather than a trait — there is no `dyn`, no erasure, no interface
    /// value; a group constrains a *named* type and is resolved statically
    /// (user decision 2026-09-08). A rule in its own right, not a
    /// consequence of one.
    fn reject_group_as_data(&mut self, r: &TypeRef) {
        let name = r.name.name.as_str();
        if self.generics.contains(name)
            || self.type_name_exists(name)
            || !self.scope.param_groups.contains_key(name)
        {
            return;
        }
        let name = name.to_string();
        self.error(
            r.span,
            format!(
                "`{name}` is a `params` group, not a type: no value may have a \
                 group as its type — a named type satisfies it with `: {name}<...>` \
                 on its declaration, or a signature spreads its members with \
                 `?{name}<...>`"
            ),
        );
    }

    /// Validates a written type: every name resolves [name-resolve], and
    /// every qualifier application is well formed — duplicates,
    /// `of`-type applicability, pairwise `with` compatibility. Called at
    /// declaration sites (fn signatures, `let` annotations, struct
    /// fields, ...) so each error is reported once; type *lowering* runs
    /// repeatedly and stays silent.
    /// [linear-composite] Whether a written type puts `var`-typed values
    /// *inside* a composite: a type argument, an array element, a tuple
    /// component or a union arm. Std's
    /// `add(list: Mut List<T>, elem: T)` is the shape that matters — a
    /// call instantiating such a `T` with a linear type **is** the store,
    /// so it is refused at the call site whatever the callee's
    /// `<T canbe linear>` claims. A bare `T` (or a qualified one, `T as
    /// Ok`) is a value passed along, not stored, and stays legal.
    fn var_in_composite(&self, ty: &ast::Type, var: &str) -> bool {
        let inside =
            |c: &ast::Type| Self::type_mentions_var(c, var) || self.var_in_composite(c, var);
        match ty {
            // [linear-container] A type argument to a *conditional
            // container's* opted-in parameter is not a refused store (user
            // decision 2026-09-12, extended to opaque containers like `List`
            // 2026-09-16): the container carries the obligation itself
            // (`take(source, 2)` building a `Take<It>`, `add(waiting, out)`
            // filling a `Mut List<Reply<Str>>`), and its terminal is checked
            // where the container is.
            ast::Type::Named { base, .. } => {
                let opted = self.linear_opt_in_positions(base.name.name.as_str());
                base.args
                    .iter()
                    .enumerate()
                    .any(|(i, a)| !opted.contains(&i) && inside(a))
            }
            ast::Type::QualifiedGroup { base, .. } => self.var_in_composite(base, var),
            ast::Type::Array { elem, .. } => inside(elem),
            ast::Type::Tuple { elems, .. } => elems.iter().any(|e| inside(e)),
            // [linear-union-arm] A union — and `T?`, which is one — is **not**
            // a composite (O-C2): the value *is* the value, so an arm carries
            // the obligation and narrowing settles it. That is what makes
            // every take-by-move answer (`remove_first(list) -> T?`,
            // `replace(map, k, v) -> V?`) a legal signature rather than a
            // store: the obligation comes back out through it.
            ast::Type::Union { .. } | ast::Type::Nullable { .. } => false,
            // A fn type neither owns nor stores what it is handed.
            ast::Type::Fn { .. } => false,
        }
    }

    /// Whether a written type is the bare type variable `var` — possibly
    /// qualified (`T as Ok`), which is still the value itself and not a
    /// container of it.
    fn type_is_bare_var(ty: &ast::Type, var: &str) -> bool {
        match ty {
            ast::Type::Named { base, .. } => base.name.name == var && base.args.is_empty(),
            ast::Type::QualifiedGroup { base, .. } => Self::type_is_bare_var(base, var),
            _ => false,
        }
    }

    /// Whether a written type mentions the type variable `var` anywhere.
    fn type_mentions_var(ty: &ast::Type, var: &str) -> bool {
        match ty {
            ast::Type::Named { base, .. } => {
                base.name.name == var || base.args.iter().any(|a| Self::type_mentions_var(a, var))
            }
            ast::Type::QualifiedGroup { base, .. } => Self::type_mentions_var(base, var),
            ast::Type::Array { elem, .. } => Self::type_mentions_var(elem, var),
            ast::Type::Tuple { elems, .. } => elems.iter().any(|e| Self::type_mentions_var(e, var)),
            ast::Type::Union { arms, .. } => arms.iter().any(|a| Self::type_mentions_var(a, var)),
            ast::Type::Nullable { inner, .. } => Self::type_mentions_var(inner, var),
            ast::Type::Fn { params, ret, .. } => {
                params.iter().any(|p| Self::type_mentions_var(p, var))
                    || Self::type_mentions_var(ret, var)
            }
        }
    }

    /// [linear-composite] A field may not *hold* a linear value: the
    /// composite's own declaration is where that store is refused, so the
    /// message can name the field. (Composite field *types* —
    /// `xs: List<Lines>` — are refused by `validate_type`; this catches
    /// the bare `h: Lines`.)
    /// [proj-field] A `proj` field is written without a source: the struct
    /// declares *that* it projects, and each literal says *what* — the
    /// source is a property of the value, not the type, and is tracked by
    /// the fate links of whoever holds the struct [proj-infer].
    fn check_proj_field(&mut self, owner: &str, field: &ast::FieldDecl) {
        for r in proj_refs(&field.ty) {
            if let Some(from) = r.from.first() {
                self.error(
                    from.span,
                    format!(
                        "field `{owner}.{}`: a `proj` field names no source — the value \
                         stored in it at each literal decides what it projects; write \
                         `proj` alone",
                        field.name.name
                    ),
                );
            }
        }
    }

    fn check_linear_field(&mut self, owner: &str, field: &ast::FieldDecl) {
        // [linear-composite] [linear-group] A concrete linear field is
        // legal exactly on a `linear struct` (user decision 2026-09-12):
        // the marker is what gives the composite its own obligation and
        // discharge set, so the contents have a legal death through it.
        // Unmarked, the store stays refused — with the marker as the
        // remedy. (A `canbe linear` generic field is the *conditional*
        // case and is not a concrete store.)
        if let Some(linear) = self.ast_type_own_linear(&field.ty) {
            let owner_is_linear = self
                .scope
                .structs
                .get(owner)
                .is_some_and(|s| s.linear);
            if owner_is_linear {
                return;
            }
            if self.scope.structs.contains_key(owner) {
                self.error(
                    field.ty.span(),
                    format!(
                        "`{linear}` is linear, so field `{owner}.{}` makes the \
                         container a resource too: declare `linear struct {owner}` \
                         (and a same-file fn consuming it) to say so",
                        field.name.name
                    ),
                );
                return;
            }
            // [linear-state] Handler state is the exception, and the reason
            // this rule has an exception at all (LC-4, user decision
            // 2026-09-15): an actor *is* its state, and the shapes the
            // concurrency surface is for — a queue of parked reply tokens, a
            // map of gathers — live in a handler field. No marker exists to
            // ask for, and none is wanted: the handler already has a lifetime
            // of its own, so the obligation rests with the **actor** until
            // it ends. What an activation may not do is leave a hole
            // (checked where a member returns), and what death does with
            // parked obligations is `watch`'s answer [actor-watch].
            //
            // One shape is refused: a **bare** obligation (a `linear struct`
            // or a linear opaque type, rather than a container of them). It
            // could be stored and never taken back out — a member consuming
            // it is the hole this rule forbids, and there is nothing to put
            // back — so the obligation would have no reachable discharge at
            // all. Hold it in a container, whose take-by-move operations
            // leave the storage itself intact.
            if self.has_auto_linear(&linear) && self.container_of_linear(&field.ty).is_none() {
                self.error(
                    field.ty.span(),
                    format!(
                        "`{linear}` is linear, so field `{owner}.{}` would hold an \
                         obligation this handler can never discharge: taking it back \
                         out would leave the state with a hole, and nothing could be \
                         put back. Hold it in a container instead — a \
                         `Mut List<{linear}>` or a `Mut Map<K, {linear}>`, whose \
                         `remove_first`/`remove` take one element at a time and whose \
                         `drain` is the terminal",
                        field.name.name
                    ),
                );
            }
        }
    }

    /// [linear-container] The container a field's linearity comes *through*,
    /// if any: `Mut List<Reply<Str>>` answers `List`, a bare `Reply<Str>`
    /// answers `None`. What distinguishes "state holds a queue of
    /// obligations" (legal, LC-4) from "state holds one" (refused: nothing
    /// could take it out again).
    fn container_of_linear(&self, ty: &ast::Type) -> Option<String> {
        match ty {
            ast::Type::Named { base, .. } => {
                let name = base.name.name.clone();
                let opted = self.linear_opt_in_positions(name.as_str());
                if opted
                    .iter()
                    .filter_map(|i| base.args.get(*i))
                    .any(|a| self.ast_type_own_linear(a).is_some())
                {
                    return Some(name);
                }
                None
            }
            ast::Type::QualifiedGroup { base, .. } => self.container_of_linear(base),
            _ => None,
        }
    }

    fn validate_type(&mut self, ty: &ast::Type) {
        // [linear-composite] A linear value may not be stored in a
        // composite: refused at each composite node, one level deep, so a
        // nested one reports at its own node.
        self.check_linear_components(ty);
        match ty {
            ast::Type::Named { qualifiers, base } => {
                self.require_name(base, false);
                self.reject_effect_as_data(base);
                self.reject_group_as_data(base);
                // [col-key-eligible] `Set<Double>` / `Map<Double, V>` are
                // refused where they are written, which covers every
                // declaration site this walk reaches.
                self.check_key_eligibility(base);
                // [col-sorted-list] `Sorted List<T>` claims an order over the
                // elements, so they have to *have* one — the same bar the
                // sorted containers apply to their keys.
                self.check_sorted_list_claim(qualifiers, base);
                for (i, a) in base.args.iter().enumerate() {
                    // [actor-spawn-expr] `Addr<E>`'s argument is the *effect*
                    // the actor serves — the one sanctioned effect-in-a-
                    // type-argument (user decision 2026-09-15). Validated
                    // here rather than in `reject_effect_as_data`, because
                    // only this walk knows what the argument belongs to.
                    if self.is_addr_effect_arg(base, i, a) {
                        // [actor-effect-kind] [monitor-handler] Legal *as* an
                        // effect here, of either kind: an actor effect's addr
                        // is a mailbox handle, a plain effect's a shared
                        // monitor's handle (SH-3, user decision 2026-09-19).
                        continue;
                    }
                    self.validate_type(a);
                }
                if !qualifiers.is_empty() {
                    let empty = HashMap::new();
                    let lowered = self.lower_base_ref(base, &empty, 0);
                    self.validate_quals(qualifiers, &lowered);
                }
            }
            ast::Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                self.validate_type(base);
                let empty = HashMap::new();
                let lowered = self.lower_type_subst(base, &empty, 0);
                self.validate_quals(qualifiers, &lowered);
            }
            ast::Type::Union { arms, .. } => {
                for a in arms {
                    self.validate_type(a);
                }
            }
            ast::Type::Tuple { elems, .. } => {
                for e in elems {
                    self.validate_type(e);
                }
            }
            ast::Type::Array { elem, .. } => self.validate_type(elem),
            ast::Type::Nullable { inner, .. } => self.validate_type(inner),
            ast::Type::Fn { params, ret, .. } => {
                for p in params {
                    self.validate_type(p);
                }
                self.validate_type(ret);
            }
        }
    }

    fn validate_quals(&mut self, qualifiers: &[TypeRef], base: &Ty) {
        // Every applied qualifier names a declaration [name-resolve].
        for q in qualifiers {
            self.require_name(q, true);
            for a in &q.args {
                self.validate_type(a);
            }
        }
        // The same qualifier cannot be applied twice [qual-no-dup].
        for (i, q) in qualifiers.iter().enumerate() {
            if qualifiers[..i].iter().any(|p| p.name.name == q.name.name) {
                self.error(
                    q.span,
                    format!("qualifier `{}` is applied more than once", q.name.name),
                );
            }
        }
        // [qual-overload] The subject picks among same-named qualifiers here:
        // `NonEmpty List<T>` and `NonEmpty Set<T>` are different declarations,
        // and the base type written in front of the qualifier says which.
        let decls: Vec<Option<&'p QualifierDecl>> = qualifiers
            .iter()
            .map(|q| self.qualifier_for(q.name.name.as_str(), Some(base)))
            .collect();
        // Each qualifier must apply to the base type (per its `of` type)
        // [qual-of].
        if !matches!(base, Ty::Unknown | Ty::Var(_)) {
            for (q, decl) in qualifiers.iter().zip(&decls) {
                // `Mut` is the language-level auto-qualifier: valid only
                // on declarations that opt in with `canbe Mut`
                // [struct-mut] [type-canbe-mut].
                if q.name.name == "Mut" {
                    if !self.has_auto_mut(base) {
                        self.error(
                            q.span,
                            format!(
                                "`Mut` does not apply to `{base}` (its declaration \
                                 does not say `canbe Mut`)"
                            ),
                        );
                    }
                    continue;
                }
                // [linear-group] Linearity is declared, not applied: every
                // value of a `canbe linear` type is linear, so writing
                // `Linear` at a use site is meaningless (and forgetting
                // it must not silently drop the protection).
                if q.name.name == "linear" {
                    self.error(
                        q.span,
                        "`Linear` cannot be written in a type: linearity is \
                         declared on the type itself (`canbe linear`) and applies \
                         to every value of it",
                    );
                    continue;
                }
                // [once-fn] `once` is the language-level *use*-multiplicity
                // qualifier. It began as call-multiplicity, on function
                // types only, and was generalized 2026-09-07 (user
                // decision): using a function value means calling it, and
                // using an `Iter<T>` means driving it, so the same
                // "at most once" claim marks a **pass** — a position in a
                // sequence — as against a replayable factory.
                //
                // The position list is deliberately explicit rather than
                // "any type": a general affine qualifier is a vocabulary
                // decision of its own, kept as roadmap D6 rather than
                // shipped as a side effect of this one.
                // [fn-effects] An **effect** name is not a qualifier. It was
                // one for as long as a producer was a *type* (`FileSystem
                // Iter<Str>` claimed what driving it performed); with the
                // reduction to `next` a producer is a struct and its effects
                // are the effects of a *function*, where `[…]` already says
                // them. So the claim-on-a-type spelling is gone, and saying
                // so names the replacement.
                if decl.is_none() && self.scope.effects.contains_key(q.name.name.as_str()) {
                    let fn_type = matches!(base, Ty::Fn { .. });
                    self.error(
                        q.span,
                        if fn_type {
                            format!(
                                "a function type declares its effects in its own list \
                                 (`(…) [{}] -> …`), not in qualifier position",
                                q.name.name
                            )
                        } else {
                            format!(
                                "`{}` is an effect, not a qualifier: a pass performs its \
                                 effects in its `next`, so declare them there \
                                 (`yield fn next(…) [{}] -> …`) rather than on the type",
                                q.name.name, q.name.name
                            )
                        },
                    );
                    continue;
                }
                if q.name.name == "once" {
                    // [once-fn] D6 (user decision 2026-09-12): `once` is
                    // valid on **any** type — the upper bound `[0,1]` is a
                    // self-restriction that demands nothing of the type's
                    // author, so `once FileHandle` or `once Ticket` means
                    // "use at most once" wherever it is written. The old
                    // fn-only/`canbe once` gate is gone; enforcement is the
                    // existing consumption machinery [deduce-consume].
                    continue;
                }
                let Some(decl) = decl else {
                    // [qual-overload] The name is declared, but not over this
                    // subject: with several candidates the resolution simply
                    // found none, which must be the same error as a single
                    // declaration that does not apply rather than silence.
                    if self.qualifier_named(q.name.name.as_str()).is_some() {
                        self.error(
                            q.span,
                            format!("qualifier `{}` does not apply to `{base}`", q.name.name),
                        );
                    }
                    continue;
                };
                if !self.qual_applies(decl, base) {
                    self.error(
                        q.span,
                        format!("qualifier `{}` does not apply to `{base}`", q.name.name),
                    );
                }
            }
        }
        // Multiple qualifiers require declared `with` compatibility
        // [qual-with].
        for i in 0..qualifiers.len() {
            for j in (i + 1)..qualifiers.len() {
                // The `Mut` auto-qualifier composes with everything
                // [type-canbe-mut].
                if qualifiers[i].name.name == "Mut" || qualifiers[j].name.name == "Mut" {
                    continue;
                }
                let (Some(a), Some(b)) = (decls[i], decls[j]) else {
                    continue;
                };
                if a.name.name == b.name.name {
                    continue; // duplicate, already reported
                }
                // One rule, one implementation: a refinement conflict
                // [qual-refn-conflict] is *precisely* "these two could not
                // have been written together", so the two sites must not
                // drift. Provenance composes freely [qual-subject], and so
                // do the compiler's own qualifiers.
                if !crate::refine::quals_compatible(a, b) {
                    self.error(
                        qualifiers[j].span,
                        format!(
                            "qualifiers `{}` and `{}` are not compatible \
                             (neither declares `with` the other)",
                            a.name.name, b.name.name
                        ),
                    );
                }
            }
        }
    }

    /// [qual-overload] The qualifier a use of `name` means. One name may be
    /// declared over several **subject types** (`NonEmpty of List<T>` beside
    /// `NonEmpty of Set<T>`), and the subject decides which — the way a fn
    /// overload is decided by its arguments.
    ///
    /// With a single candidate the subject is not consulted at all, so a use
    /// over an unknown or generic subject still resolves; that is also the
    /// path every program took before overloading existed.
    fn qualifier_for(&mut self, name: &str, subject: Option<&Ty>) -> Option<&'p QualifierDecl> {
        let candidates = self.scope.qualifiers.get(name)?.clone();
        match candidates.as_slice() {
            [] => None,
            [one] => Some(*one),
            many => {
                // Against the *base* type: a qualifier's `of` is about the
                // data, so a `Mut Set<Str>` subject picks the `of Set<T>`
                // declaration exactly as a plain `Set<Str>` does.
                let subject = subject?.strip_quals().clone();
                many.iter().copied().find(|d| self.qual_applies(d, &subject))
            }
        }
    }

    /// [qual-overload] A qualifier with this name, for questions that are
    /// about the *name* rather than about a subject — does one exist, does it
    /// have a body, is it provenance. Where several are declared these
    /// answers agree in practice (they are the same claim over different
    /// containers), and the first is as good as any.
    fn qualifier_named(&self, name: &str) -> Option<&'p QualifierDecl> {
        self.scope.qualifiers.get(name)?.first().copied()
    }

    /// Whether a qualifier's `of` type accepts the given base type.
    fn qual_applies(&mut self, decl: &'p QualifierDecl, base: &Ty) -> bool {
        if matches!(base, Ty::Unknown | Ty::Var(_)) {
            return true;
        }
        let saved = self.enter_generics(&decl.generics);
        let of_ty = self.lower_type(&decl.of);
        self.generics = saved;
        let mut subst = HashMap::new();
        unify(&of_ty, base.strip_quals(), &mut subst)
    }

    /// Whether the base type's declaration opted into the `Mut`
    /// auto-qualifier: `struct S canbe Mut` [struct-mut] or
    /// `intrinsic type List<T> canbe Mut` [type-canbe-mut].
    /// Whether `name` is a *provenance* qualifier [qual-subject]: a claim
    /// about where the handle came from, which no call can invalidate.
    fn is_provenance_qual(&self, name: &str) -> bool {
        self.qualifier_named(name)
            .is_some_and(|d| d.subject == QualSubject::Provenance)
    }

    /// [qual-refn] Applies the refinements in scope to one kept argument of
    /// a resolved call: removals first, then additions.
    ///
    /// The additions are what the feature exists for — `add`'s exhaustive
    /// `[list: Mut]` must drop `NonEmpty` ([deduce-syntax] is sound only
    /// that way), and `NonEmpty`'s own refinement puts it back. Trusted,
    /// like `-> T as Q` [qual-ctor-fn]: no `qualifies` call is emitted.
    ///
    /// Two cases apply nothing. A group whose refinements *conflict*
    /// [qual-refn-conflict] is suppressed, with one warning per (callee,
    /// parameter) — no error, since the caller can still test by hand or
    /// reconcile with a top-level `refn`, but not silence either, or an
    /// imported refinement would appear to do nothing for no visible
    /// reason. And a single addition is skipped when it could not co-apply
    /// with a qualifier the call *preserved* [qual-with]: the value cannot
    /// carry both claims, and knowing less is the safe direction.
    fn apply_refinements(&mut self, callee: FnKey, param: &str, arg: &str, span: Span) {
        let Some(group) = self.refinements.for_param(self.file_idx, callee, param) else {
            return;
        };
        if !group.conflict.is_empty() {
            let key = (callee, param.to_string());
            if self.refn_warned.insert(key) {
                let quals = group.conflict.join("` and `");
                self.warn(
                    span,
                    format!(
                        "the refinements of `{quals}` disagree about `{param}` \
                         here, so none of them apply: `{quals}` cannot be applied \
                         to one value (neither declares `with` the other). Test \
                         the property with `is` after this call, or reconcile them \
                         in a top-level `refn` in this module"
                    ),
                );
            }
            return;
        }
        let remove: HashSet<String> = group.remove.iter().cloned().collect();
        let add = group.add.clone();
        if !remove.is_empty() {
            if let Some(var) = self.lookup_mut(arg) {
                var.narrowed = var.narrowed.clone().remove_quals(&remove);
            }
        }
        for q in add {
            let have: Vec<String> = self
                .lookup(arg)
                .map(|v| v.narrowed.quals().iter().map(|x| x.name.clone()).collect())
                .unwrap_or_default();
            if have.contains(&q) {
                continue;
            }
            let compatible = have.iter().all(|h| {
                // [qual-overload] Compatibility is a relation between *names*:
                // a `with` clause names a qualifier, not a qualifier-over-a-
                // subject, so same-named declarations share their `with` list.
                match (self.qualifier_named(h.as_str()), self.qualifier_named(q.as_str())) {
                    (Some(a), Some(b)) => crate::refine::quals_compatible(a, b),
                    // `Mut` and the other intrinsics compose with
                    // everything; an invisible qualifier cannot be judged.
                    _ => true,
                }
            });
            if !compatible {
                continue;
            }
            if let Some(var) = self.lookup_mut(arg) {
                var.narrowed = var.narrowed.clone().qualify(vec![Qual {
                    effect: false,
                    name: q,
                    args: Vec::new(),
                }]);
            }
        }
    }

    fn has_auto_mut(&self, base: &Ty) -> bool {
        let Ty::Named { name, .. } = base.strip_quals() else {
            return false;
        };
        let has_mut = |quals: &[ast::TypeRef]| quals.iter().any(|q| q.name.name == "Mut");
        self.scope
            .structs
            .get(name.as_str())
            .is_some_and(|s| has_mut(&s.auto_qualifiers))
            || self
                .scope
                .opaque_types
                .get(name.as_str())
                .is_some_and(|t| has_mut(&t.auto_qualifiers))
    }

    /// Validates a qualifier declaration: predicate qualifiers need a
    /// well-formed `qualifies` function [qual-predicate]; field overrides
    /// must refine real fields of the `of` struct [qual-field-override];
    /// bodiless (constructive) qualifiers take neither [qual-constructive].
    fn check_qualifier_decl(&mut self, q: &'p QualifierDecl) {
        self.validate_type(&q.of);
        // [qual-with] Compatibility clauses name other qualifiers
        // [name-resolve].
        for w in &q.with {
            self.require_name(w, true);
        }
        let of_ty = self.lower_type(&q.of);
        match &of_ty {
            Ty::Union(_) => self.error(
                q.of.span(),
                "a qualifier cannot apply to a union type; qualify the arms instead",
            ),
            Ty::Tuple(_) => self.error(
                q.of.span(),
                "a qualifier cannot apply to a tuple type; qualify the parts instead",
            ),
            _ => {}
        }
        // [qual-subject] Provenance is mint-only: no inspection of the
        // bits can establish where a handle came from, so a `qualifies`
        // body (and the field overrides that come with one — those are
        // claims about *contents*) is a contradiction.
        if q.subject == QualSubject::Provenance && q.has_body {
            self.error(
                q.name.span,
                format!(
                    "provenance qualifier `{}` cannot have a body: provenance is not \
                     testable at runtime (no `qualifies`) and describes the handle, \
                     not its contents (no field overrides) — values gain it from \
                     constructor functions",
                    q.name.name
                ),
            );
            return;
        }
        if !q.has_body {
            return;
        }
        let Some(qualifies) = q.fns.iter().find(|f| f.name.name == "qualifies") else {
            self.error(
                q.name.span,
                format!(
                    "predicate qualifier `{}` must define a `qualifies` function",
                    q.name.name
                ),
            );
            return;
        };
        if qualifies.params.len() != 1 {
            self.error(
                qualifies.name.span,
                "`qualifies` must take exactly one parameter (the candidate value)",
            );
        } else {
            let pt = self.lower_type(&qualifies.params[0].ty);
            if !is_subtype(&of_ty, &pt) {
                self.error(
                    qualifies.params[0].span,
                    format!("`qualifies` must accept a `{of_ty}` parameter"),
                );
            }
        }
        let ret = qualifies
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);
        if !matches!(&ret, Ty::Named { name, args } if name == "Bool" && args.is_empty()) {
            self.error(qualifies.name.span, "`qualifies` must return `Bool`");
        }
        if qualifies.deductions.as_ref().is_some_and(|d| !d.is_empty()) {
            self.error(
                qualifies.name.span,
                "`qualifies` does not support deductions (it never moves the value)",
            );
        }
        for fo in &q.field_overrides {
            self.validate_type(&fo.ty);
            let override_ty = self.lower_type(&fo.ty);
            match self.declared_field_ty(&of_ty, &fo.name.name) {
                Some(orig) => {
                    if !is_subtype(&override_ty, &orig) {
                        self.error(
                            fo.span,
                            format!(
                                "field override `{}: {override_ty}` is not a subtype \
                                 of the declared `{orig}`",
                                fo.name.name
                            ),
                        );
                    }
                }
                None => {
                    if let Ty::Named { name, .. } = of_ty.strip_quals() {
                        if self.scope.structs.contains_key(name.as_str()) {
                            let name = name.clone();
                            self.error(
                                fo.name.span,
                                format!("struct `{name}` has no field `{}`", fo.name.name),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validates a qualifier-constructor signature (`fn f(...) -> T as Q`)
    /// [qual-ctor-fn]: same file as the qualifier [qual-ctor-same-file],
    /// simple return type, `of`-type satisfaction [qual-ctor-simple].
    /// Both constructive and predicate qualifiers may have constructors
    /// [qual-ctor-predicate]: a predicate-qualifier constructor asserts
    /// its predicate holds by construction, so callers get the qualified
    /// type without a runtime `is` check.
    fn check_constructor_sig(&mut self, f: &'p FnDecl) {
        let Some(cref) = &f.constructs else { return };
        let name = cref.name.name.as_str();
        let subject = f.return_type.as_ref().map(|t| self.lower_type(t));
        let Some(decl) = self.qualifier_for(name, subject.as_ref()) else {
            self.error(
                cref.span,
                format!("unknown qualifier `{name}` in constructor return type"),
            );
            return;
        };
        if !self.own_qualifiers.contains(name) {
            self.error(
                cref.span,
                format!(
                    "constructor functions for `{name}` must be declared in the \
                     same file as the qualifier"
                ),
            );
        }
        let Some(rt) = &f.return_type else {
            self.error(
                cref.span,
                "a qualifier constructor must declare a return type",
            );
            return;
        };
        let base = self.lower_type(rt);
        match base.strip_quals() {
            Ty::Union(_) => self.error(
                rt.span(),
                "a qualifier constructor must return a simple type, not a union",
            ),
            Ty::Tuple(_) => self.error(
                rt.span(),
                "a qualifier constructor must return a simple type, not a tuple",
            ),
            other => {
                if !self.qual_applies(decl, other) {
                    self.error(
                        rt.span(),
                        format!("qualifier `{name}` does not apply to `{base}`"),
                    );
                }
            }
        }
    }

    /// The return type a *caller* sees for a fn: constructors add their
    /// qualifier on top of the declared return type.
    fn fn_return_ty(&mut self, decl: &'p FnDecl) -> Ty {
        let ret = decl
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);
        match &decl.constructs {
            Some(cref) => {
                let empty = HashMap::new();
                let qual = Qual {
                    effect: false,
                    name: cref.name.name.clone(),
                    args: cref
                        .args
                        .iter()
                        .map(|a| self.lower_type_subst(a, &empty, 0))
                        .collect(),
                };
                ret.qualify(vec![qual])
            }
            None => ret,
        }
    }
}

/// Collects names assigned (or incremented) anywhere in a block, for
/// post-branch narrowing resets.
fn collect_assigned(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Assign { target, .. } => {
                if let Expr::Ident(id) = target {
                    out.insert(id.name.clone());
                }
            }
            Stmt::Expr(e) | Stmt::Let { value: e, .. } => collect_assigned_expr(e, out),
            Stmt::Return { value: Some(e), .. } | Stmt::Break { value: Some(e), .. } => {
                collect_assigned_expr(e, out)
            }
            _ => {}
        }
    }
}

/// Collects the names assigned anywhere inside `expr` [narrow-assign-reset].
///
/// Exhaustive over `Expr` **on purpose**: a wildcard arm here loses
/// assignments inside a new syntactic form silently, and the narrowing that
/// should have been reset stays in place — a wrong-code bug, not a
/// diagnostic. A new variant must be classified, not defaulted.
fn collect_assigned_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::IncDec { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
            collect_assigned_expr(operand, out);
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_assigned_expr(c, out);
                collect_assigned(b, out);
            }
            if let Some(b) = else_block {
                collect_assigned(b, out);
            }
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_assigned_expr(cond, out);
            collect_assigned(body, out);
            if let Some(b) = else_block {
                collect_assigned(b, out);
            }
        }
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_assigned_expr(iterable, out);
            collect_assigned(body, out);
            if let Some(b) = else_block {
                collect_assigned(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_assigned_expr(subject, out);
            for b in branches {
                collect_assigned(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_assigned_expr(c, out);
                collect_assigned(b, out);
            }
            collect_assigned(else_block, out);
        }
        // [try] The delimiter's body is ordinary code.
        Expr::Try { body, .. } => collect_assigned(body, out),
        // [actor-spawn-expr] Every clause is an ordinary expression, and an
        // argument that crosses to the child may itself assign.
        Expr::Spawn {
            handler,
            uses,
            pool,
            ..
        } => {
            collect_assigned_expr(handler, out);
            for handler in uses {
                collect_assigned_expr(handler, out);
            }
            if let Some(pool) = pool {
                collect_assigned_expr(pool, out);
            }
        }
        // [actor-replyto] The captures are ordinary expressions.
        Expr::ReplyTo { captures, .. } => {
            for capture in captures {
                collect_assigned_expr(capture, out);
            }
        }
        // [actor-self-send] A leaf: the selector names a handler member.
        Expr::SelfScoped { .. } => {}
        // [actor-waitfor] The bridge's block is ordinary code.
        Expr::WaitFor { body, .. } => collect_assigned(body, out),
        // A lambda body's assignments happen when the value is called, and
        // the checker cannot see where that is: counted here, so a
        // narrowing an enclosing branch relied on is reset conservatively.
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(e) => collect_assigned_expr(e, out),
            LambdaBody::Block(b) => collect_assigned(b, out),
        },
        Expr::Call { callee, args, .. } => {
            collect_assigned_expr(callee, out);
            for a in args {
                collect_assigned_expr(a, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_assigned_expr(base, out);
            collect_assigned_expr(index, out);
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_assigned_expr(lhs, out);
            collect_assigned_expr(rhs, out);
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_assigned_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_assigned_expr(k, out);
                collect_assigned_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_assigned_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_assigned_expr(e, out),
                }
            }
        }
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_assigned_expr(e, out);
                }
            }
        }
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            collect_assigned_expr(base, out)
        }
        // [fn-overload-at] `xs.add@core.list(..)`: the receiver is an
        // ordinary expression.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(base) = base {
                collect_assigned_expr(base, out);
            }
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::Spread { operand, .. } => collect_assigned_expr(operand, out),
        Expr::Is { subject, .. } | Expr::Widen { subject, .. } => {
            collect_assigned_expr(subject, out)
        }
        // Leaves: no sub-expression, so nothing can be assigned inside.
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Ident(_)
        | Expr::Error { .. } => {}
    }
}

/// Removes the named qualifiers from a type [qual-widen], or `None` when it
/// does not carry all of them. `Qual T` with every qualifier removed is `T`.
fn strip_quals_named(ty: &Ty, names: &[String]) -> Option<Ty> {
    let Ty::Qualified { quals, base } = ty else {
        return None;
    };
    if !names.iter().all(|n| quals.iter().any(|q| q.name == *n)) {
        return None;
    }
    let kept: Vec<Qual> = quals
        .iter()
        .filter(|q| !names.contains(&q.name))
        .cloned()
        .collect();
    Some(if kept.is_empty() {
        (**base).clone()
    } else {
        (**base).clone().qualify(kept)
    })
}

/// Whether any statement of `block` mentions the variable `name` — the
/// block half of [`expr_mentions`].
fn block_mentions_name(block: &Block, name: &str) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Let { value, .. } => expr_mentions(value, name),
        Stmt::Assign { target, value, .. } => {
            expr_mentions(target, name) || expr_mentions(value, name)
        }
        Stmt::Return { value: Some(e), .. } | Stmt::Break { value: Some(e), .. } => {
            expr_mentions(e, name)
        }
        Stmt::Use { handler, .. } => expr_mentions(handler, name),
        Stmt::Expr(e) => expr_mentions(e, name),
        _ => false,
    })
}

/// Whether `expr` mentions the variable `name` anywhere — any read,
/// projection base, interpolation, nested branch/loop body, or lambda
/// capture. Used for same-call ordering [deduce-same-call]: arguments
/// are evaluated left to right, so mentioning a value that an earlier
/// argument of the same call consumed is a use-after-move.
///
/// Exhaustive over `Expr` **on purpose**: a wildcard arm here answers "no"
/// for a form it has never heard of, and a use-after-move goes unreported.
fn expr_mentions(expr: &Expr, name: &str) -> bool {
    let block_mentions = |block: &Block| -> bool { block_mentions_name(block, name) };
    match expr {
        Expr::Ident(id) => id.name == name,
        Expr::Str { parts, .. } => parts.iter().any(|p| match p {
            StrExprPart::Interp(e) => expr_mentions(e, name),
            _ => false,
        }),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => expr_mentions(base, name),
        Expr::Call { callee, args, .. } => {
            expr_mentions(callee, name) || args.iter().any(|a| expr_mentions(a, name))
        }
        Expr::Index { base, index, .. } => expr_mentions(base, name) || expr_mentions(index, name),
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            elems.iter().any(|e| expr_mentions(e, name))
        }
        Expr::MapLit { entries, .. } => entries
            .iter()
            .any(|(k, v)| expr_mentions(k, name) || expr_mentions(v, name)),
        Expr::StructLit { fields, .. } => fields.iter().any(|f| match &f.kind {
            StructLitFieldKind::Named { value, .. } => expr_mentions(value, name),
            StructLitFieldKind::Spread(e) => expr_mentions(e, name),
        }),
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::IncDec { operand, .. }
        | Expr::Spread { operand, .. } => expr_mentions(operand, name),
        Expr::Binary { lhs, rhs, .. } => expr_mentions(lhs, name) || expr_mentions(rhs, name),
        Expr::Is { subject, .. } | Expr::Widen { subject, .. } => expr_mentions(subject, name),
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            branches
                .iter()
                .any(|(c, b)| expr_mentions(c, name) || block_mentions(b))
                || else_block.as_ref().is_some_and(|b| block_mentions(b))
        }
        Expr::When {
            subject, branches, ..
        } => expr_mentions(subject, name) || branches.iter().any(|b| block_mentions(&b.body)),
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            branches
                .iter()
                .any(|(c, b)| expr_mentions(c, name) || block_mentions(b))
                || block_mentions(else_block)
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            expr_mentions(cond, name)
                || block_mentions(body)
                || else_block.as_ref().is_some_and(|b| block_mentions(b))
        }
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => {
            expr_mentions(iterable, name)
                || block_mentions(body)
                || else_block.as_ref().is_some_and(|b| block_mentions(b))
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(e) => expr_mentions(e, name),
            LambdaBody::Block(b) => block_mentions(b),
        },
        // [try] The delimiter's body is ordinary code — a value mentioned
        // only inside it is still mentioned.
        Expr::Try { body, .. } => block_mentions(body),
        // [actor-spawn-expr] A spawn's clauses mention values the same way a
        // call's arguments do — and a value sent to a child is consumed by
        // it, so this must see through every clause.
        Expr::Spawn {
            handler,
            uses,
            pool,
            ..
        } => {
            expr_mentions(handler, name)
                || uses.iter().any(|h| expr_mentions(h, name))
                || pool.as_ref().is_some_and(|p| expr_mentions(p, name))
        }
        // [actor-replyto] A capture is a value the continuation takes.
        Expr::ReplyTo { captures, .. } => captures.iter().any(|c| expr_mentions(c, name)),
        // [actor-self-send] A leaf: it mentions no value of its own.
        Expr::SelfScoped { .. } => false,
        // [actor-waitfor] The bridge's block is ordinary code.
        Expr::WaitFor { body, .. } => block_mentions(body),
        // [fn-overload-at] The name is a *function*, never a value; only the
        // dot-notation receiver can mention anything.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            base.as_ref().is_some_and(|b| expr_mentions(b, name))
        }
        // Leaves: no sub-expression, so nothing to mention.
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => false,
    }
}

/// Value info about a block's trailing expression (for branch-value
/// coercions applied once the join type is known).
struct TailInfo {
    span: Span,
    logical: Ty,
    repr: Ty,
}

/// Value contributions collected while checking one loop body
/// [while-value].
#[derive(Default)]
struct LoopCtx {
    /// `locals.len()` when the loop was entered: frames at or above this
    /// index die when a `break`/`continue` leaves the iteration
    /// [linear-obligation].
    entry_depth: usize,
    /// `break value` contributions (span of the value expression).
    breaks: Vec<TailInfo>,
    /// A bare `break` or a `continue` occurred: an iteration may end
    /// without producing a value, so `None` joins the loop's value type.
    may_skip_value: bool,
    /// Flow-state snapshot at each `break`: the loop's exit merges these
    /// with the fall-through exit state, so a value consumed on a break
    /// path stays consumed after the loop [deduce-consume].
    break_states: Vec<NarrowSnapshot>,
}

/// A parsed `is` check: qualifiers + optional base type (or `None`).
struct CheckPat {
    quals: Vec<String>,
    base: Option<Ty>,
    is_none: bool,
    /// A name in the check resolved to nothing and was reported
    /// [name-resolve]: downstream match diagnostics are suppressed.
    unresolved: bool,
}

/// The outcome of analyzing one `is` expression.
struct IsInfo {
    /// The place the subject denotes, when it is one flow analysis tracks
    /// [flow-place]: a variable or a field chain out of one.
    subject_place: Option<Place>,
    /// The physical (declared) type of the subject's storage — the type the
    /// `is` test was lowered against, and the type a narrowed read unwraps
    /// from [flow-place].
    subject_repr: Ty,
    /// Logical type when the check succeeds.
    matched: Ty,
    /// Logical type when the check fails (union subjects only).
    remaining: Option<Ty>,
    binding: Option<Binding>,
}

impl<'p, 'r> Checker<'p, 'r> {
    // ================= widening checks [qual-widen] =================

    /// Analyzes a `^` check: the **dual of `is`**. The runtime test is the
    /// one `is` of the same qualifiers performs — same arm, same lowering —
    /// but the matched type is *generalized* (the qualifiers removed) rather
    /// than refined, so the subject reads without them inside the branch.
    fn widen_info(&mut self, subject: &'p Expr, quals: &'p [TypeRef], span: Span) -> IsInfo {
        let subj_ty = self.check_expr(subject, None);
        let repr = self.repr_of(subject, &subj_ty);
        let pat = self.parse_check(quals);
        // Same places `is` narrows [flow-place]: a variable or a field chain
        // out of a tracked local (user decision 2026-09-05: consistent with
        // `is`).
        let subject_place =
            Place::of_expr(subject).filter(|p| p.narrowable() && self.lookup(&p.root).is_some());
        let unchanged = IsInfo {
            subject_place: None,
            subject_repr: repr.clone(),
            matched: subj_ty.clone(),
            remaining: None,
            binding: None,
        };
        if pat.unresolved {
            return unchanged;
        }
        // `^` removes qualifiers: a base type in the check would be an `is`
        // question, not a widening one.
        if pat.base.is_some() || pat.is_none {
            self.error(
                span,
                "`^` removes qualifiers, so its right side is qualifier \
                 names only (use `is` to check a type)"
                    .to_string(),
            );
            return unchanged;
        }
        if pat.quals.is_empty() {
            self.error(span, "`^` needs a qualifier to remove".to_string());
            return unchanged;
        }
        // [qual-widen] The single exclusion list, shared with `Qual T <: T`.
        let mut blocked = false;
        for q in &pat.quals {
            if let Some(reason) = self.removal_block(q) {
                self.error(span, format!("`{q}` cannot be removed with `^`: {reason}"));
                blocked = true;
            }
        }
        if blocked {
            return unchanged;
        }
        // The same runtime test `is` would emit; absent for a tautology (the
        // qualifiers are statically present), where the check is `true`.
        if let Some(test) = self.union_test_for(&repr, &pat) {
            self.out.is_tests.insert(self.key(span), test);
        }
        let (matched, remaining) = match &subj_ty {
            Ty::Union(arms) => {
                let (m, r): (Vec<Ty>, Vec<Ty>) = arms
                    .iter()
                    .cloned()
                    .partition(|arm| self.arm_matches(arm, &pat));
                if m.len() > 1 {
                    // Each arm would peel a *different* wrapper position, so
                    // one widened view cannot stand for all of them.
                    self.error(
                        span,
                        format!(
                            "`^ {}` matches more than one arm of \
                             `{subj_ty}`: widening removes a qualifier from \
                             a single arm",
                            pat.quals.join(" ")
                        ),
                    );
                    return unchanged;
                }
                let stripped: Vec<Ty> = m
                    .iter()
                    .filter_map(|arm| strip_quals_named(arm, &pat.quals))
                    .collect();
                if stripped.len() != m.len() || m.is_empty() {
                    self.error(
                        span,
                        format!(
                            "no arm of `{subj_ty}` carries `{}`, so there is \
                             nothing for `^` to remove",
                            pat.quals.join(" ")
                        ),
                    );
                    return unchanged;
                }
                (self.mk_union(stripped), Some(self.mk_union(r)))
            }
            other => match strip_quals_named(other, &pat.quals) {
                // Statically present: the check cannot fail, and the else
                // branch learns nothing.
                Some(stripped) => (stripped, None),
                None => {
                    if !other.is_unknown() && !matches!(other, Ty::Nothing) {
                        self.error(
                            span,
                            format!(
                                "`{other}` does not carry `{}`, so there is \
                                 nothing for `^` to remove",
                                pat.quals.join(" ")
                            ),
                        );
                    }
                    return unchanged;
                }
            },
        };
        // [qual-widen] Inside the branch the value is seen *at* the widened
        // type: the arm test peeled the wrapper, so a further `is`/`when` on
        // the subject must narrow relative to the stripped type, not to the
        // storage it came out of. (The emitters materialize the peel as a
        // branch-local temporary.)
        let subject_repr = if self.out.is_tests.contains_key(&self.key(span)) {
            self.out
                .widen_targets
                .insert(self.key(span), matched.clone());
            matched.clone()
        } else {
            repr
        };
        IsInfo {
            subject_place,
            subject_repr,
            matched,
            remaining,
            binding: None,
        }
    }

    // ================= blocks & statements =================

    /// Checks a block in value position: returns its value type (last
    /// expression, `Nothing` after return/break/continue, else `None`).
    fn check_block_value(&mut self, block: &'p Block) -> Ty {
        self.check_branch_block(block, Vec::new()).0
    }

    /// [unused-var] Warns for locals in the scope about to close that were
    /// never read. A leading `_` opts out — the conventional way to say "I
    /// know, and I mean it" — and so do parameters (a signature often
    /// dictates them, especially an effect or handler member implementing a
    /// declared interface) and handler state. std is exempt: its unused
    /// bindings are the compiler's business, not the user's.
    ///
    /// A *warning*, not an error: an unused variable is a smell, and the
    /// program it appears in is still well-defined.
    fn warn_unused_locals(&mut self) {
        if self.is_std {
            return;
        }
        let Some(frame) = self.locals.last() else {
            return;
        };
        let mut unused: Vec<(String, Span)> = frame
            .iter()
            .filter(|(name, var)| {
                !var.used && !var.is_param && !var.is_handler_state && !name.starts_with('_')
            })
            .map(|(name, var)| (name.clone(), var.decl_span))
            .collect();
        // Deterministic order: frames are hash maps, and a diagnostic list
        // that reorders between runs is not comparable in tests.
        unused.sort_by_key(|(_, span)| span.start);
        for (name, span) in unused {
            self.warn(
                span,
                format!(
                    "`{name}` is never used; prefix it with `_` (`_{name}`) if \
                     that is deliberate"
                ),
            );
        }
    }

    /// Checks a block with `is`-bindings pre-declared in its scope.
    fn check_branch_block(
        &mut self,
        block: &'p Block,
        bindings: Vec<Binding>,
    ) -> (Ty, Option<TailInfo>) {
        self.locals.push(HashMap::new());
        // [effect-scope] `use` registrations expire with the block.
        let effect_depth = self.effect_env.len();
        // [fn-rename] So do renames: a `rename fn` is in force from its line
        // to the end of the block it is written in — including a loop body or
        // a lambda body, which are blocks like any other.
        let rename_depth = self.renames.len();
        for binding in bindings {
            self.declare_binding(binding);
        }
        let mut value = Ty::none();
        let mut tail = None;
        for stmt in &block.stmts {
            value = self.check_stmt(stmt);
            tail = match stmt {
                Stmt::Expr(e) => Some(TailInfo {
                    span: e.span(),
                    logical: value.clone(),
                    repr: self.repr_of(e, &value),
                }),
                _ => None,
            };
        }
        // [linear-obligation] Nothing linear may die with the scope.
        self.check_linear_frame_drop();
        self.warn_unused_locals();
        self.locals.pop();
        self.effect_env.truncate(effect_depth);
        self.renames.truncate(rename_depth);
        (value, tail)
    }

    fn check_stmt(&mut self, stmt: &'p Stmt) -> Ty {
        match stmt {
            // [fn-rename] In force from here to the end of the block.
            Stmt::Rename(decl) => {
                self.declare_rename(decl);
                Ty::none()
            }
            Stmt::Let {
                pattern,
                ty,
                value,
                span,
            } => {
                if let Some(t) = ty {
                    self.validate_type(t);
                }
                let annotated = ty.as_ref().map(|t| self.lower_type(t));
                let value_ty = self.check_expr(value, annotated.as_ref());
                if let Some(ann) = &annotated {
                    if !is_subtype(&value_ty, ann) {
                        self.error(
                            value.span(),
                            format!("expected `{ann}`, found `{value_ty}`"),
                        );
                    }
                }
                let declared = annotated.unwrap_or_else(|| {
                    // [let-infer] Without an annotation the *logical* value
                    // type becomes
                    // the declared type; re-wrap narrowed unions physically.
                    let repr = self.repr_of(value, &value_ty);
                    self.maybe_coerce(value.span(), &value_ty, &repr, &value_ty.clone());
                    value_ty.clone()
                });
                // [proj-anywhere] A view of a temporary cannot be kept.
                self.reject_temp_view(value, "bind");
                // Binding from a bare identifier or projection links the
                // new variable(s) to the source: they share fate
                // [fate-link].
                let links = self.links_for_value(value, *span);
                self.declare_pattern(pattern, declared, links);
                Ty::none()
            }
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let target_ty = match target {
                    Expr::Ident(id) => match self.lookup(&id.name) {
                        Some(var) => var.declared.clone(),
                        None => {
                            self.error(
                                id.span,
                                format!("assignment to undeclared variable `{}`", id.name),
                            );
                            Ty::Unknown
                        }
                    },
                    other => {
                        // [fate-partial-move] A target is written, not read.
                        let saved_target = std::mem::replace(&mut self.assign_target, true);
                        let ty = self.check_expr(other, None);
                        self.assign_target = saved_target;
                        // [struct-mut] Only `Mut`-qualified struct values
                        // may have fields assigned.
                        if let Expr::Field { base, field, .. } = other {
                            let base_ty = self
                                .out
                                .ty_of(self.file_idx, base.span())
                                .cloned()
                                .unwrap_or(Ty::Unknown);
                            let is_struct = matches!(
                                base_ty.strip_quals(),
                                Ty::Named { name, .. }
                                    if self.scope.structs.contains_key(name.as_str())
                            );
                            let has_mut = base_ty.quals().iter().any(|q| q.name == "Mut");
                            if is_struct && !has_mut {
                                self.error(
                                    field.span,
                                    format!(
                                        "cannot assign to field `{}` of an immutable \
                                         `{base_ty}` value: only `Mut`-qualified \
                                         values may have fields assigned",
                                        field.name
                                    ),
                                );
                            }
                        }
                        // [expr-tuple-index] A tuple element is never
                        // assignable: qualifiers — `Mut` among them —
                        // cannot apply to a tuple [qual-union-arm], so no
                        // tuple value can grant mutation permission.
                        if let Expr::TupleIndex { index, span, .. } = other {
                            self.error(
                                *span,
                                format!(
                                    "cannot assign to tuple element `{index}`: `Mut` \
                                     cannot apply to a tuple, so tuple elements are \
                                     read-only — rebuild the tuple instead"
                                ),
                            );
                        }
                        // Assigning through a projection mutates the root
                        // variable [fate-poison]; mutating a derived
                        // variable is an error [fate-derived-readonly].
                        // Narrowings of the overwritten storage fall
                        // [flow-place-invalidate].
                        self.fate_mutation_through(other, *span);
                        // [fate-partial-move] Assigning a place puts data
                        // back: the place and everything under it are
                        // whole again, so their moved-out records go.
                        if let Some(place) = Place::of_expr(other) {
                            self.revive_moved_places(&place);
                        }
                        ty
                    }
                };
                let value_ty = self.check_expr(value, Some(&target_ty));
                // [var-no-widen] assignments must fit the declared type.
                if !is_subtype(&value_ty, &target_ty) {
                    self.error(
                        value.span(),
                        format!(
                            "cannot assign `{value_ty}` to `{target_ty}` \
                             (a variable's type can never widen)"
                        ),
                    );
                } else if let Expr::Ident(id) = target {
                    let value_ty = value_ty.clone();
                    // [linear-obligation] Overwriting a variable that
                    // still owns a linear value drops it.
                    let owes = self
                        .lookup(&id.name)
                        .is_some_and(|var| self.owes_linear(&id.name, var));
                    if owes {
                        self.error(
                            *span,
                            format!(
                                "assigning to `{}` drops the linear value it \
                                 still owns; move it onward or `discard({})` \
                                 first",
                                id.name, id.name
                            ),
                        );
                    }
                    // [effect-state-store] Assigning into a handler *state*
                    // field is a **store**, not a binding: the field
                    // outlives every member call, so the handler takes
                    // ownership — exactly as a struct literal does. A
                    // *link* here would outlive the call it was made in,
                    // which Kotlin can represent (aliasing) and Rust
                    // cannot; before this rule the Rust backend silently
                    // cloned, and mutable data diverged observably between
                    // the backends.
                    let is_state = self
                        .lookup(&id.name)
                        .is_some_and(|var| var.is_handler_state);
                    if is_state {
                        self.fate_move(value, "store", "a handler state assignment", value.span());
                    }
                    // Reassignment: the variable's old value is gone, so
                    // variables derived from it are poisoned
                    // [fate-poison]; the variable itself revives with
                    // fresh links to the new value's sources [fate-link].
                    // An assignment is a bind event: move-mode applies
                    // [fate-move-mode].
                    let links = if is_state {
                        Vec::new()
                    } else {
                        self.links_for_value(value, *span)
                    };
                    let links = self.apply_binding_mode(&id.name, links);
                    let root = self.lookup(&id.name).map(|var| (var.id, id.name.clone()));
                    if let Some((root_id, root_name)) = root {
                        self.poison_derived(
                            root_id,
                            &root_name,
                            FateEvent::Reassigned,
                            *span,
                            // A whole variable: every derived value falls
                            // [fate-field-disjoint].
                            Some(&[]),
                        );
                    }
                    if let Some(var) = self.lookup_mut(&id.name) {
                        var.narrowed = value_ty;
                        var.links = links;
                        var.poison = None;
                        var.consumed_by = None;
                        // The old value is gone: every fact about its
                        // parts falls with it [flow-place-invalidate].
                        var.place_narrows.clear();
                        // [fate-partial-move] And the new value is whole,
                        // so a partially moved variable revives completely.
                        var.moved_places.clear();
                    }
                }
                Ty::none()
            }
            Stmt::Return { value, span } => {
                let expected = self.ret_ty.clone();
                match value {
                    Some(v) => {
                        // [proj-anywhere] A constructor call whose result
                        // goes into a `proj` arm lends its argument: mark
                        // it before checking so the contract does not
                        // consume the value the caller is about to borrow.
                        // The call is resolved during checking, so the arm
                        // test itself happens after; the mark is on the
                        // *shape* (a one-argument call under a union-return
                        // derived fn) and the arm decides below.
                        let saved_lending = self.lending_ctor;
                        if self.own_derived_return.is_some() && !self.own_proj_arms.is_empty() {
                            if let Expr::Call { span, args, .. } = v {
                                if args.len() == 1 {
                                    self.lending_ctor = Some(*span);
                                }
                            }
                        }
                        let vty = self.check_expr(v, Some(&expected));
                        self.lending_ctor = saved_lending;
                        if !is_subtype(&vty, &expected) {
                            self.error(
                                v.span(),
                                format!("expected return type `{expected}`, found `{vty}`"),
                            );
                        }
                        // [readonly-return] A derived-return fn
                        // *borrows* its result out: the value must be
                        // derived from the annotated parameter (or be
                        // `None`), and nothing is consumed.
                        let into_proj_arm = match self.own_derived_return.clone() {
                            None => None,
                            Some(from) if self.own_proj_arms.is_empty() => Some(from),
                            // [proj-anywhere] A union return: only a value
                            // built for a `proj` arm is a borrow. The arm is
                            // read off the constructor the value goes
                            // through (`emitted(x)` builds `Emitted`).
                            Some(from) => self
                                .constructed_qualifier(v)
                                .filter(|q| self.own_proj_arms.contains(q))
                                .map(|_| from),
                        };
                        if let Some(from) = into_proj_arm {
                            // [proj-anywhere] Only a *union* return goes
                            // through a tag constructor whose operand is the
                            // borrowed value (`emitted(x)`); the arm test
                            // above is what established that. A plain
                            // `proj[from: p] T` return is checked as
                            // written — unwrapping any one-argument call
                            // here mistook `list.get(0)` for a constructor
                            // and asked the *index* for its provenance.
                            let inner = if self.own_proj_arms.is_empty() {
                                v
                            } else {
                                Self::constructor_operand(v)
                            };
                            self.check_derived_return_value(inner, &from);
                        } else {
                            // Returning a value moves it: a fate-linked
                            // (derived) variable cannot be moved
                            // [fate-derived-readonly]; returning a root
                            // consumes it [deduce-consume] (terminal
                            // here, but visible to unreachable code and
                            // to derived-variable poison [fate-poison]).
                            self.fate_move(v, "return", "a `return`", *span);
                        }
                        // [linear-obligation] Nothing linear may be
                        // alive anywhere when the fn exits.
                        self.check_linear_exit(0, *span, "return");
                        // [linear-state] …and no state field may be left
                        // with a hole in it.
                        self.check_state_whole(*span, "return");
                    }
                    None => {
                        if !expected.is_none_ty() && !expected.is_unknown() {
                            self.error(
                                *span,
                                format!("bare `return` in a function returning `{expected}`"),
                            );
                        }
                        // [linear-obligation]
                        self.check_linear_exit(0, *span, "return");
                        // [linear-state]
                        self.check_state_whole(*span, "return");
                    }
                }
                Ty::Nothing
            }
            Stmt::Break { value, span } => {
                if self.loop_stack.is_empty() {
                    self.error(*span, "`break` outside of a loop");
                }
                // [while-value] `break value` contributes to the loop's
                // value; a bare `break` may leave the loop without a value.
                match value {
                    Some(v) => {
                        let vty = self.check_expr(v, None);
                        // `break value` moves the value out of the loop:
                        // a derived variable cannot be moved
                        // [fate-derived-readonly]; a root is consumed
                        // [deduce-consume] — the code after the loop sees
                        // it moved.
                        self.fate_move(v, "break with", "a `break`", *span);
                        let repr = self.repr_of(v, &vty);
                        if let Some(ctx) = self.loop_stack.last_mut() {
                            ctx.breaks.push(TailInfo {
                                span: v.span(),
                                logical: vty,
                                repr,
                            });
                        }
                    }
                    None => {
                        if let Some(ctx) = self.loop_stack.last_mut() {
                            ctx.may_skip_value = true;
                        }
                    }
                }
                // [linear-obligation] Frames inside the loop die at a
                // `break`: nothing linear may still be owed in them.
                // The loop's exit is reachable from every `break`: record
                // this path's flow state so the after-loop merge sees
                // values consumed on break paths [deduce-consume] (an
                // always-exiting branch containing the `break` contributes
                // nothing to the merge *inside* the body, but the loop
                // exit is exactly where its state lands).
                match self.loop_stack.last().map(|c| c.entry_depth) {
                    Some(depth) => {
                        self.check_linear_exit(depth, *span, "break");
                        let snap = self.snapshot_narrows();
                        if let Some(ctx) = self.loop_stack.last_mut() {
                            ctx.break_states.push(snap);
                        }
                    }
                    None => {
                        let snap = self.snapshot_narrows();
                        if let Some(ctx) = self.loop_stack.last_mut() {
                            ctx.break_states.push(snap);
                        }
                    }
                }
                Ty::Nothing
            }
            Stmt::Continue { span } => {
                match self.loop_stack.last_mut() {
                    Some(ctx) => ctx.may_skip_value = true,
                    None => self.error(*span, "`continue` outside of a loop"),
                }
                // [linear-obligation] Frames inside the loop iteration
                // die at a `continue`.
                if let Some(depth) = self.loop_stack.last().map(|c| c.entry_depth) {
                    self.check_linear_exit(depth, *span, "continue");
                }
                Ty::Nothing
            }
            Stmt::Use { handler, span } => {
                // [use-requires-use] only `[use]` fns may register handlers.
                if !self.can_use {
                    self.error(
                        *span,
                        "`use` requires the `use` effect in the function's effect list",
                    );
                }
                self.check_use(handler, *span);
                Ty::none()
            }
            Stmt::Expr(e) => {
                let ty = self.check_expr(e, None);
                // [linear-obligation] A linear value in statement
                // position is dropped on the spot.
                if self.inferred.is_some() {
                    if self.ty_own_linear(&ty) {
                        self.error(
                            e.span(),
                            "this expression produces a linear value that is \
                             dropped immediately; bind it, move it onward, or \
                             pass it to `discard`",
                        );
                    }
                }
                ty
            }
        }
    }

    fn declare_pattern(&mut self, pattern: &Pattern, ty: Ty, links: Vec<FateLink>) {
        for binding in self.pattern_bindings(pattern, ty, links, None) {
            self.declare_binding(binding);
        }
    }

    /// Declares one branch/loop binding (name, type, inherited links,
    /// `for`-loop origin) [fate-link] [fate-move-mode].
    fn declare_binding(&mut self, (ident, ty, links, for_origin): Binding) {
        self.declare_var(&ident, ty, links, false, for_origin);
    }

    /// Flattens a binding pattern into the `(name, type, links)` triples
    /// it declares [fate-link]. Loop bindings go through this so each
    /// checking pass of a loop body re-declares them fresh — an iteration
    /// binds a *new* element, so consumption of the binding never
    /// survives the back edge [deduce-consume].
    fn pattern_bindings(
        &mut self,
        pattern: &Pattern,
        ty: Ty,
        links: Vec<FateLink>,
        for_origin: Option<Span>,
    ) -> Vec<Binding> {
        match pattern {
            Pattern::Ident(id) => vec![(id.clone(), ty, links, for_origin)],
            Pattern::Tuple { elems, span } => {
                // A pattern is not an expression, but it *has* a type — the
                // subject's — and both tooling (hover) and the emitters read it
                // by span: a destructuring loop binds the element to a
                // temporary whose declared type this is.
                self.out
                    .expr_ty
                    .entry(self.key(*span))
                    .or_insert_with(|| ty.clone());
                // [let-destructure] The subject has to *be* a tuple of this
                // arity. Until 2026-09-18 a mismatch bound `Unknown`s and said
                // nothing, so `let (a, b) = 7` compiled and the emitters
                // produced target code that could not build — a Salvo mistake
                // reported, at best, by rustc or kotlinc.
                //
                // `Unknown` still passes silently [type-unknown-lenient]: one
                // mistake, one diagnostic.
                let elem_tys: Vec<Ty> = match ty.strip_quals() {
                    Ty::Tuple(ts) if ts.len() == elems.len() => ts.clone(),
                    Ty::Unknown | Ty::Nothing => vec![Ty::Unknown; elems.len()],
                    other => {
                        let what = match other {
                            Ty::Tuple(ts) => format!(
                                "`{ty}` is a tuple of {}, not {}",
                                ts.len(),
                                elems.len()
                            ),
                            _ => format!("`{ty}` is not a tuple"),
                        };
                        self.error(
                            *span,
                            format!(
                                "this pattern destructures a tuple of {}, but {what}",
                                elems.len()
                            ),
                        );
                        vec![Ty::Unknown; elems.len()]
                    }
                };
                elems
                    .iter()
                    .zip(elem_tys)
                    .flat_map(|(p, t)| self.pattern_bindings(p, t, links.clone(), for_origin))
                    .collect()
            }
            Pattern::Struct { fields, span } => {
                self.out
                    .expr_ty
                    .entry(self.key(*span))
                    .or_insert_with(|| ty.clone());
                fields
                .iter()
                .map(|f| {
                    // [let-destructure] Destructuring reads the declared
                    // field type (qualifier overrides only apply to direct
                    // field accesses, which the backend can cast).
                    let fty = self
                        .declared_field_ty(&ty, &f.field.name)
                        .unwrap_or(Ty::Unknown);
                    // Each destructured binding is a projection of the
                    // source: it shares the source's fate [fate-link].
                    (f.binding.clone(), fty, links.clone(), for_origin)
                })
                .collect()
            }
        }
    }

    /// Handles a move-position value — `return x`, `break x`, `yield x`,
    /// a struct/array/tuple literal store, spread `...x`, or a `use`
    /// handler-constructor argument [deduce-consume]. Only bare
    /// identifiers (through spread) are tracked. Moving a fate-linked
    /// (derived) variable is an error [fate-derived-readonly] — `copy`
    /// is the remedy; moving a root consumes it (narrows to `Nothing`,
    /// with `moved_by` naming the event in the use-site diagnostic) and
    /// poisons its derived variables [fate-poison], exactly like a
    /// call-site move.
    /// [linear-container] Records a read that **moves a linear value**, so the
    /// emitters hand it over rather than copy it: Rust's narrowed-read path
    /// clones through a borrow, which duplicates an obligation — and a reply
    /// token is not `Clone` at all. Called from both consumption paths (the
    /// deduction-driven one at a call, and `fate_move`) while the variable's
    /// live type is still readable, i.e. before it is marked consumed.
    fn note_linear_move(&mut self, name: &str, span: Span) {
        let live = self.lookup(name).map(|v| {
            if matches!(v.narrowed, Ty::Unknown) {
                v.declared.clone()
            } else {
                v.narrowed.clone()
            }
        });
        if live.is_some_and(|t| self.ty_own_linear(&t)) {
            self.out.linear_moves.insert(self.key(span));
        }
    }

    fn fate_move(&mut self, value: &Expr, action: &str, moved_by: &'static str, span: Span) {
        // [proj-anywhere] Returning or storing a view of a temporary. Driving
        // one (`for x in iter(list_of(1, 2))`) is fine: the temporary lives for
        // the whole loop statement on both backends.
        if action != "iterate" {
            self.reject_temp_view(value, action);
        }
        let inner = match value {
            Expr::Spread { operand, .. } => operand.as_ref(),
            _ => value,
        };
        let Expr::Ident(id) = inner else {
            // [proj-anywhere] A *value* that borrows — the result of a
            // derived-return call (`iter(bag.items)`, `first(xs)`) or a
            // literal with `proj` fields — cannot be moved out: it is a view
            // of something the caller keeps, and moving it (a `return`, a
            // store) would let the view outlive what it borrows. The remedy
            // is the same as for a derived variable: `copy`.
            if matches!(inner, Expr::Call { .. } | Expr::StructLit { .. }) {
                let links = self.links_for_value(inner, span);
                // [proj-infer] A value that *holds* borrows (a pass minted
                // here) is an owned object and may travel — into a caller,
                // a field, a consuming parameter — as long as it does not
                // outlive its roots: returned, it may be rooted only in the
                // parameters this fn lends.
                if !links.is_empty() && links.iter().all(|l| l.held) {
                    if action == "return" {
                        self.check_returned_view_roots(&links, inner.span());
                    }
                    return;
                }
                if let Some(l) = links.first() {
                    let root = l.root_name.clone();
                    // Inside a fn that *itself* returns a borrow of that root,
                    // returning the view is the declared contract — the
                    // derived-return validation covers it.
                    if action == "return" && self.own_derived_sources.iter().any(|s| s == &root) {
                        return;
                    }
                    self.error(
                        inner.span(),
                        format!(
                            "cannot {action} this value: it is a view of `{root}` (it \
                             borrows from it), so it can only be read here; use `copy` \
                             to make an independent value, or declare the borrow on \
                             this function's return (`proj[from: {root}]`)"
                        ),
                    );
                    return;
                }
            }
            // A projection in a moved position moves data out of its
            // provenance roots [fate-move-mode].
            self.projection_move(inner, span);
            return;
        };
        let Some(var) = self.lookup(&id.name) else {
            return;
        };
        let (var_id, links) = (var.id, var.links.clone());
        let name = id.name.clone();
        // [effect-state-store] A handler's **storage** — a state field or a
        // constructor parameter — outlives every member call, so a member
        // cannot move a value out of it: the handler still owns it after the
        // call returns. The mirror of the store rule, and owed for the same
        // reason it was: the two backends cannot agree otherwise. Rust
        // silently `.clone()`d (so the handler kept a *copy* and mutable data
        // diverged observably — and a `linear` value had its obligation
        // duplicated), Kotlin shared the reference, and where the clone was
        // missing — the consuming *intrinsics*, `send` and `add` — rustc
        // reported a raw E0507 with no Salvo diagnostic at all. Found
        // 2026-09-15; a `Copy` state field hides every part of it.
        //
        // `copy` is the remedy, and the diagnostic says so — except for a
        // linear value, which has none: taking one out of a composite is the
        // interim refusal [linear-composite], and here the composite is the
        // handler.
        if self.consume_of_handler_storage(&name, action, id.span) {
            return;
        }
        if !links.is_empty() {
            // [proj-infer] A held view travels as an owned object (see the
            // value case above); a wholesale projection or alias cannot.
            if links.iter().all(|l| l.held) {
                if action == "return" {
                    self.check_returned_view_roots(&links, id.span);
                }
            } else {
                self.error_derived(id.span, action, &name, &links);
                return;
            }
        }
        // A lambda cannot consume a capture [fate-lambda].
        if let Some(frame) = self.frame_of_id(var_id) {
            if self.capture_move_violation(frame, &name, id.span) {
                return;
            }
        }
        // A kept lambda parameter belongs to the caller [fn-contract].
        if self.consume_kept_lambda_param(&name, id.span) {
            return;
        }
        self.note_linear_move(&name, id.span);
        self.poison_derived(var_id, &name, FateEvent::Moved, span, Some(&[]));
        if let Some(var) = self.lookup_mut(&id.name) {
            var.narrowed = Ty::Nothing;
            var.consumed_by = Some(moved_by);
        }
    }
    /// [effect-state-store] Is `name` one of the *enclosing handler's* stored
    /// values — a state field or a constructor parameter — being consumed?
    /// Reports if so and answers whether it did.
    ///
    /// Both kinds are storage the handler keeps across every activation, so a
    /// member has nothing to move out of: what the caller of the member would
    /// leave behind is a handler with a hole in it. The remedy is `copy`, and
    /// a linear value has none.
    fn consume_of_handler_storage(&mut self, name: &str, action: &str, span: Span) -> bool {
        let Some(h) = self.own_handler else {
            return false;
        };
        let kind = if h.state.iter().any(|f| f.name.name == name) {
            "state field"
        } else if h.params.iter().any(|p| p.name.name == name) {
            "constructor parameter"
        } else {
            return false;
        };
        let ty = self
            .lookup(name)
            .map(|v| v.declared.clone())
            .unwrap_or(Ty::Unknown);
        // [copy-scalar-free] A native scalar's copy is free and
        // indistinguishable from a move, so a member may hand one over: the
        // handler keeps its own, both backends agree, and nothing is cloned
        // that the reader would care about. This is the exemption the
        // deduction rules already grant, and it is why the first asynchronous
        // test (`sum: Int`) never saw the rule below.
        if crate::types::is_copy_scalar(&ty.strip_quals()) {
            return false;
        }
        // [linear-composite] Linearity is deliberately *not* transitive
        // through composites, so the type's own obligation is the whole
        // question here.
        let linear = self.ty_own_linear(&ty);
        let handler = h.name.name.clone();
        // [linear-state] LC-4 (user decision 2026-09-15): a **state field**
        // holding an obligation may be moved out of — that is how a queue of
        // parked tokens is drained — provided the activation puts something
        // back before it returns (`check_state_whole`, at every exit). The
        // actor owns the obligation across activations; what is forbidden is
        // leaving a hole in state a later activation would read. A
        // *constructor parameter* stays refused: it is the handler's own
        // record of how it was built, and nothing can restore it.
        if linear && kind == "state field" {
            // [linear-state] [rs-state-take] Recorded for the emitters: on
            // Rust a field cannot simply be moved out of `&mut self`, so the
            // read becomes a `std::mem::take` — which is also exactly the
            // semantics (the field is empty until the member puts something
            // back, and the checker made it).
            self.out.state_takes.insert(self.key(span));
            return false;
        }
        if linear {
            self.error(
                span,
                format!(
                    "cannot {action} `{name}`: it is a {kind} of handler `{handler}`, \
                     which owns it for its whole lifetime — and a linear value cannot \
                     be copied out of it, so there is nothing to hand over here \
                     [linear-composite]. Discharge it where the handler itself is \
                     settled"
                ),
            );
        } else {
            self.error(
                span,
                format!(
                    "cannot {action} `{name}`: it is a {kind} of handler `{handler}`, \
                     which still owns it after this member returns — so the value \
                     cannot move out. Use `copy({name})` to hand over an independent \
                     value"
                ),
            );
        }
        true
    }

    // ================= expressions =================
    fn check_expr(&mut self, expr: &'p Expr, expected: Option<&Ty>) -> Ty {
        let ty = self.check_expr_inner(expr, expected);
        self.out.expr_ty.insert(self.key(expr.span()), ty.clone());
        if let Some(exp) = expected {
            let repr = self.repr_of(expr, &ty);
            self.maybe_coerce(expr.span(), &ty, &repr, exp);
        }
        ty
    }

    /// The physical representation type of an expression: for identifier
    /// uses this is the declared type (narrowing does not re-wrap values),
    /// and for a narrowed projection place the type its storage keeps
    /// [flow-place].
    fn repr_of(&self, expr: &Expr, logical: &Ty) -> Ty {
        if let Expr::Ident(id) = expr {
            if let Some(var) = self.lookup(&id.name) {
                // [qual-widen] Inside a `^` branch the value is seen at the
                // widened type.
                return var.widened.clone().unwrap_or_else(|| var.declared.clone());
            }
        }
        if matches!(expr, Expr::Field { .. } | Expr::TupleIndex { .. }) {
            if let Some(entry) = Place::of_expr(expr)
                .as_ref()
                .and_then(|p| self.place_narrow_entry(p))
            {
                return entry.declared.clone();
            }
        }
        logical.clone()
    }

    fn check_expr_inner(&mut self, expr: &'p Expr, expected: Option<&Ty>) -> Ty {
        match expr {
            // [lit-adopt] An *unsuffixed* numeric literal adopts the
            // expected numeric type where one exists (`let x: Long = 1`,
            // `f(1)` into a `Long` parameter, `let d: Double = 3`), so
            // `Long` positions need no `to_long(1)` noise (user decision
            // 2026-09-14). A written suffix stays explicit and never
            // adopts; an integer literal never adopts `Int`-ward (a
            // `Float` literal cannot become `Int`).
            Expr::Int { long, .. } => match adopted_numeric(expected, *long, false) {
                Some(name) => Ty::named(name),
                None => Ty::named(if *long { "Long" } else { "Int" }),
            },
            Expr::Float { single, .. } => match adopted_numeric(expected, *single, true) {
                Some(name) => Ty::named(name),
                None => Ty::named(if *single { "Float" } else { "Double" }),
            },
            Expr::Bool { .. } => Ty::named("Bool"),
            Expr::Char { .. } => Ty::named("Char"),
            Expr::Str { parts, .. } => {
                for part in parts {
                    if let StrExprPart::Interp(e) = part {
                        let ty = self.check_expr(e, None);
                        // [str-drop-mut] Interpolation reads the *text* of
                        // a value, so a builder is converted first.
                        self.drop_mut_operand(e, &ty);
                        // [interp-no-none] Interpolating a possibly-absent
                        // value is an error (user decision 2026-09-02):
                        // Kotlin would print `null` while Rust rejects the
                        // `Option` — an observable backend divergence.
                        if !ty.is_unknown() && !matches!(ty, Ty::Nothing) {
                            let stripped = ty.strip_quals();
                            if stripped.has_none_arm() || stripped.is_none_ty() {
                                let message = if stripped.is_none_ty() {
                                    "cannot interpolate `None` into a string: it \
                                     has no text form"
                                        .to_string()
                                } else {
                                    format!(
                                        "cannot interpolate a value that may be \
                                         `None` (its type is `{ty}`): narrow it \
                                         first (`is` / `when`) or assert it with `!`"
                                    )
                                };
                                self.error(e.span(), message);
                                continue;
                            }
                            // [interp-to-str] The value has to have a text
                            // form. Scalars and `Str` render natively on both
                            // backends; anything else needs a `to_str`,
                            // resolved *here* like an implicit parameter
                            // (user decision 2026-09-11). Without one this
                            // used to reach the backend and become rustc's
                            // "doesn't implement Display"
                            // [backend-never-wrong].
                            self.check_interpolable(e, &ty);
                        }
                    }
                }
                Ty::named("Str")
            }
            Expr::Ident(id) => {
                if id.name == "None" {
                    return Ty::none();
                }
                // [unused-var] An identifier *expression* is a read — the
                // only thing that counts as a use. An assignment target
                // looks the variable up directly instead, so writing to a
                // variable without ever reading it leaves it unused.
                if let Some(var) = self.lookup_mut(&id.name) {
                    var.used = true;
                }
                if let Some(var) = self.lookup(&id.name) {
                    let narrowed = var.narrowed.clone();
                    // [qual-widen] Inside a `^` branch the physical view is
                    // the widened type: the wrapper the arm test peeled is
                    // materialized by the emitters, so reads (and any
                    // further narrowing) unwrap relative to *it*.
                    let declared = var.widened.clone().unwrap_or_else(|| var.declared.clone());
                    let poison = var.poison.clone();
                    let consumed_by = var.consumed_by;
                    let links = var.links.clone();
                    let var_id = var.id;
                    // [fate-partial-move] A whole-value use of a variable
                    // something was moved out of: the value is incomplete,
                    // so it cannot be handed on. Suppressed for projection
                    // bases, which do their own precise check.
                    if self.projection_base == 0
                        && self.check_moved_place(&Place::root(id.name.clone()), id.span)
                    {
                        return Ty::Unknown;
                    }
                    // [deduce-consume] `Nothing` marks a consumed (moved)
                    // value: referring to it is an impossibility.
                    if matches!(narrowed, Ty::Nothing) {
                        match poison {
                            // [fate-move-mode] The variable is a *root*
                            // consumed by a move-mode binding:
                            // `root_name` carries the binding's name.
                            Some(p) if p.event == FateEvent::BoundAway => self.error(
                                id.span,
                                format!(
                                    "`{}` cannot be used here: `{binding}` was bound \
                                     from it and later moves the value, so the \
                                     binding took ownership; bind with `copy(...)` \
                                     (e.g. `let {binding} = copy(...)`) to keep \
                                     `{}` usable, or reassign it before use",
                                    id.name,
                                    id.name,
                                    binding = p.root_name,
                                ),
                            ),
                            // [fate-poison] Two-site diagnostic: the value
                            // shared fate with a root that was
                            // mutated/moved/reassigned after the binding.
                            Some(p) => self.error(
                                id.span,
                                format!(
                                    "`{}` cannot be used here: it was bound from \
                                     `{root}` and shares its fate, and `{root}` \
                                     was {event} after the binding; bind it with \
                                     `copy(...)` to keep an independent value, or \
                                     reassign it before use",
                                    id.name,
                                    root = p.root_name,
                                    event = p.event.describe(),
                                ),
                            ),
                            None => self.error(
                                id.span,
                                format!(
                                    "`{}` cannot be used here: it was consumed (moved) \
                                     by {}, so its type is `Nothing`; \
                                     reassign it before use",
                                    id.name,
                                    consumed_by.unwrap_or("an earlier call"),
                                ),
                            ),
                        }
                        return Ty::Unknown;
                    }
                    // A read of a variable from outside an enclosing
                    // lambda is a *capture* [fate-lambda].
                    if !self.lambda_ctx.is_empty() {
                        if let Some(frame) = self.frame_of_id(var_id) {
                            let mut visited = HashSet::new();
                            let mutable = self.ty_transitively_mut(&declared, &mut visited);
                            let name = id.name.clone();
                            self.record_capture(frame, &name, var_id, mutable);
                        }
                    }
                    if narrowed != declared {
                        self.out.repr_ty.insert(self.key(id.span), declared);
                    }
                    // Expose the fate links of derived-variable reads for
                    // tooling (`proj` presentation) [fate-link].
                    if !links.is_empty() {
                        let reads: Vec<FateRead> = links
                            .iter()
                            .map(|l| FateRead {
                                root: l.root_name.clone(),
                                bind_span: l.bind_span,
                                path: l
                                    .path
                                    .as_ref()
                                    .map(|p| p.iter().map(|s| s.to_string()).collect()),
                            })
                            .collect();
                        self.out.fate_reads.insert(self.key(id.span), reads);
                    }
                    return narrowed;
                }
                if self.scope.handlers.contains_key(id.name.as_str()) {
                    return Ty::named(&id.name);
                }
                if self.has_callable(&id.name) {
                    // [fn-value-select] Passing a function *by name*: the
                    // same selection every call goes through, against the
                    // expected fn type instead of an argument list.
                    return self.fn_value_by_name(&id.name, id.span, None, expected);
                }
                // [ident-resolve] Nothing declares this name *as a value*.
                // Salvo assumes it can see everything, so a reference no
                // declaration justifies is an error here rather than a
                // `Ty::Unknown` that reaches a backend and becomes rustc's or
                // kotlinc's problem [backend-never-wrong].
                //
                // [mixed-handler] Inside a mixed handler's sync member, a
                // state field's name is deliberately out of scope — the
                // confinement rule — and the diagnostic names the rule
                // rather than claiming the name does not exist.
                if self.confined_state.iter().any(|n| n == &id.name) {
                    self.error(
                        id.span,
                        format!(
                            "`{}` is the servant's state, which a mixed handler \
                             confines to its `send fn` members: a sync member runs \
                             on the caller's thread, where reading it would race an \
                             activation — send to a member that answers instead",
                            id.name
                        ),
                    );
                    return Ty::Unknown;
                }
                //
                // A name that *is* declared, but as something that is not a
                // value, says so instead — the same courtesy the effect and
                // handler rules already extend [effect-not-a-type].
                let kind = if self.scope.structs.contains_key(id.name.as_str()) {
                    Some("a struct type, not a value: construct one (`Name { … }`)")
                } else if self.scope.effects.contains_key(id.name.as_str()) {
                    Some(
                        "an effect, not a value: call one of its members, \
                         having registered a handler with `use`",
                    )
                } else if self.scope.qualifiers.contains_key(id.name.as_str()) {
                    Some(
                        "a qualifier, not a value: write it on a type \
                         (`Name value`) or test it with `is`",
                    )
                } else if self.scope.param_groups.contains_key(id.name.as_str()) {
                    Some(
                        "a `params` group, not a value: spread it into a \
                         signature (`?Name<…>`)",
                    )
                } else if self.scope.opaque_types.contains_key(id.name.as_str())
                    || self.scope.type_aliases.contains_key(id.name.as_str())
                {
                    Some("a type, not a value")
                } else {
                    None
                };
                match kind {
                    Some(what) => self.error(id.span, format!("`{}` is {what}", id.name)),
                    None => self.error_unresolved(
                        id.span,
                        format!("no variable or function named `{}` is in scope", id.name),
                        &id.name,
                    ),
                }
                Ty::Unknown
            }
            // [effect-at] An effect-selected member is a *call* form: a
            // member is not a value — nothing implements it until a
            // handler is in scope, and neither backend can render one
            // detached from its dispatch.
            Expr::EffectScoped {
                base, name, effect, ..
            } => {
                if let Some(base) = base {
                    self.check_expr(base, None);
                }
                self.error(
                    name.span,
                    format!(
                        "`{}@{}` selects an effect's member, which is not a \
                         value: call it (`{}@{}(…)`)",
                        name.name, effect.name, name.name, effect.name
                    ),
                );
                Ty::Unknown
            }
            // [fn-overload-at] [fn-value-select] A scope-selected fn *value*
            // (`describe@main` passed to a higher-order function). As a
            // *callee* it never reaches here — `check_call` handles it — so a
            // receiver written here is a partially applied call, which Salvo
            // does not have.
            Expr::Scoped {
                base,
                name,
                module,
                span,
            } => {
                if let Some(base) = base {
                    self.check_expr(base, None);
                    self.error(
                        *span,
                        format!(
                            "`{}` here is a *value*, so it takes no receiver: \
                             write `{}@{}` to name the function, or call it",
                            name.name,
                            name.name,
                            module
                                .iter()
                                .map(|m| m.name.as_str())
                                .collect::<Vec<_>>()
                                .join(".")
                        ),
                    );
                    return Ty::Unknown;
                }
                if !self.has_callable(&name.name) {
                    self.error_unresolved(
                        name.span,
                        format!("no function named `{}` is in scope", name.name),
                        &name.name,
                    );
                    return Ty::Unknown;
                }
                self.fn_value_by_name(&name.name, name.span, Some(module), expected)
            }
            Expr::Field { base, field, span } => {
                // [fate-partial-move] The `p` in `p.name` is not a use of
                // the whole value; the place check below is the precise one.
                self.projection_base += 1;
                let base_ty = self.check_expr(base, None);
                self.projection_base -= 1;
                if let Some(place) = Place::of_expr(expr) {
                    if self.check_moved_place(&place, *span) {
                        return Ty::Unknown;
                    }
                }
                // A predicate-qualifier field override refines the type
                // [qual-field-override];
                // the backend casts + asserts at the access site.
                // [lsp-definition] the field name points at its
                // declaration in the struct.
                self.record_field_ref(&base_ty, field);
                if let Some(override_ty) = self.field_override_ty(&base_ty, &field.name) {
                    self.out
                        .field_casts
                        .insert(self.key(*span), override_ty.clone());
                    return override_ty;
                }
                // [flow-place] A narrowed projection place reads at its
                // narrowed type. The storage keeps the declared type — the
                // one the `is` test was lowered against — so the read is
                // recorded in `repr_ty` for the emitters to unwrap from,
                // exactly as a narrowed identifier read is.
                if let Some(place) = Place::of_expr(expr) {
                    if let Some(entry) = self.place_narrow_entry(&place) {
                        let (narrowed, declared) = (entry.narrowed.clone(), entry.declared.clone());
                        if narrowed != declared {
                            self.out.repr_ty.insert(self.key(*span), declared);
                        }
                        return narrowed;
                    }
                }
                self.field_ty_or_error(&base_ty, field)
            }
            Expr::TupleIndex { base, index, span } => {
                // [fate-partial-move] A projection base, not a whole use.
                self.projection_base += 1;
                let base_ty = self.check_expr(base, None);
                self.projection_base -= 1;
                if let Some(place) = Place::of_expr(expr) {
                    if self.check_moved_place(&place, *span) {
                        return Ty::Unknown;
                    }
                }
                // [flow-place] A narrowed element place reads at its
                // narrowed type, exactly as a field does.
                if let Some(place) = Place::of_expr(expr) {
                    if let Some(entry) = self.place_narrow_entry(&place) {
                        let (narrowed, declared) = (entry.narrowed.clone(), entry.declared.clone());
                        if narrowed != declared {
                            self.out.repr_ty.insert(self.key(*span), declared);
                        }
                        return narrowed;
                    }
                }
                self.tuple_elem_ty_or_error(&base_ty, *index, *span)
            }
            Expr::Call {
                callee,
                type_args,
                args,
                named,
                span,
            } => self.check_call(callee, type_args, args, named, expected, *span),
            Expr::Index { base, index, span } => {
                // [fate-partial-move] A projection base, not a whole use.
                self.projection_base += 1;
                let base_ty = self.check_expr(base, None);
                self.projection_base -= 1;
                if let Some(place) = Place::of_expr(expr) {
                    if self.check_moved_place(&place, *span) {
                        self.check_expr(index, Some(&Ty::named("Int")));
                        return Ty::Unknown;
                    }
                }
                self.check_expr(index, Some(&Ty::named("Int")));
                match base_ty.strip_quals() {
                    Ty::Array(elem) => (**elem).clone(),
                    // [index-resolve] Only arrays are subscriptable; other
                    // collections expose element access as declared
                    // functions (`get(list, i)`). Only an un-inferred base
                    // stays lenient [type-unknown-lenient] — a generic `T`
                    // is opaque, not unknown.
                    Ty::Unknown | Ty::Nothing => Ty::Unknown,
                    other => {
                        let other = other.clone();
                        self.error(
                            *span,
                            format!(
                                "`{other}` cannot be indexed with `[]`: only \
                                 arrays can, and other collections expose \
                                 element access as functions (e.g. \
                                 `get(collection, index)`)"
                            ),
                        );
                        Ty::Unknown
                    }
                }
            }
            // [col-literal] `[1, 2, 3]` is a **List** literal. It built an
            // array until 2026-09-13; arrays kept the type syntax (`T[]`)
            // and gained `array_of` as their constructor, because a list is
            // the ordinary sequence and an array is the variadic boundary.
            Expr::ArrayLit { elems, .. } => {
                let expected_elem = Self::concrete_elem(match expected.map(|t| t.strip_quals()) {
                    Some(Ty::Named { name, args }) if name == "List" && args.len() == 1 => {
                        Some(args[0].clone())
                    }
                    // An array is still accepted as the *expected* type so a
                    // literal in a variadic position keeps working.
                    Some(Ty::Array(e)) => Some((**e).clone()),
                    _ => None,
                });
                let want_array = matches!(
                    expected.map(|t| t.strip_quals()),
                    Some(Ty::Array(_))
                );
                let mut tys = Vec::new();
                for e in elems {
                    tys.push(self.check_expr(e, expected_elem.as_ref()));
                }
                // [linear-composite] An array holds many values; a linear
                // one may not be among them.
                for (e, ty) in elems.iter().zip(&tys) {
                    if self.ty_own_linear(ty) {
                        let linear = format!("{ty}");
                        self.refuse_linear_composite(
                            e.span(),
                            &linear,
                            "an array element".to_string(),
                        );
                    }
                }
                // Storing a value in an array literal moves it
                // [deduce-consume]; spread elements move their operand.
                for e in elems {
                    self.fate_move(e, "store", "a literal store", e.span());
                }
                let elem = expected_elem.unwrap_or_else(|| {
                    if tys.is_empty() {
                        Ty::Unknown
                    } else {
                        self.mk_union(tys)
                    }
                });
                if elems.is_empty() && elem.is_unknown() {
                    self.error(
                        expr.span(),
                        concat!(
                            "an empty list literal needs its element type ",
                            "from the position it is in: annotate the binding ",
                            "(`let xs: List<Int> = []`) or pass it where the ",
                            "parameter's type says what it holds",
                        )
                            .to_string(),
                    );
                }
                // [col-literal] A `Mut` on the literal asks for a mutable
                // list (`Mut [1, 2]`); the qualifier arrives through the
                // expected type or the literal's own qualifiers.
                if want_array {
                    Ty::Array(Box::new(elem))
                } else {
                    self.collection_lit_ty("List", vec![elem], expected)
                }
            }
            // [col-literal] `{1, 2}` is a Set literal, `{"a": 1}` a Map
            // literal. Both *construct*, so their elements move
            // [deduce-consume], their key type must be hashable
            // [col-key-eligible], and they adopt a `Mut` the position asks
            // for exactly as a list literal does.
            Expr::SetLit { elems, span } => {
                // [col-literal] An empty `{}` takes its kind from the
                // position: a `Map` there is an empty map, not an empty set.
                if elems.is_empty() {
                    if let Some(Ty::Named { name, args }) =
                        expected.map(|t| t.strip_quals())
                    {
                        if name == "Map" && args.len() == 2 {
                            return self.collection_lit_ty(
                                "Map",
                                vec![args[0].clone(), args[1].clone()],
                                expected,
                            );
                        }
                        if name == "List" && args.len() == 1 {
                            return self.collection_lit_ty(
                                "List",
                                vec![args[0].clone()],
                                expected,
                            );
                        }
                    }
                }
                let expected_elem = Self::concrete_elem(match expected.map(|t| t.strip_quals()) {
                    Some(Ty::Named { name, args }) if name == "Set" && args.len() == 1 => {
                        Some(args[0].clone())
                    }
                    _ => None,
                });
                let mut tys = Vec::new();
                for e in elems {
                    tys.push(self.check_expr(e, expected_elem.as_ref()));
                }
                for e in elems {
                    self.fate_move(e, "store", "a literal store", e.span());
                }
                let elem = expected_elem.unwrap_or_else(|| {
                    if tys.is_empty() {
                        Ty::Unknown
                    } else {
                        self.mk_union(tys)
                    }
                });
                // [col-literal] An empty literal carries no element to infer
                // from, so the position must say what it is.
                if elems.is_empty() && elem.is_unknown() {
                    self.error(
                        *span,
                        concat!(
                            "an empty collection literal needs its type from ",
                            "the position it is in: annotate the binding ",
                            "(`let s: Set<Int> = {}`) or pass it where the ",
                            "parameter's type says which collection it is",
                        )
                            .to_string(),
                    );
                }
                if let Some(reason) = self.key_ineligible(&elem) {
                    self.error(*span, reason);
                }
                self.collection_lit_ty("Set", vec![elem], expected)
            }
            Expr::MapLit { entries, span } => {
                let (expected_key, expected_val) = match expected.map(|t| t.strip_quals()) {
                    Some(Ty::Named { name, args }) if name == "Map" && args.len() == 2 => {
                        (
                            Self::concrete_elem(Some(args[0].clone())),
                            Self::concrete_elem(Some(args[1].clone())),
                        )
                    }
                    _ => (None, None),
                };
                let mut key_tys = Vec::new();
                let mut val_tys = Vec::new();
                for (k, v) in entries {
                    key_tys.push(self.check_expr(k, expected_key.as_ref()));
                    val_tys.push(self.check_expr(v, expected_val.as_ref()));
                }
                for (k, v) in entries {
                    self.fate_move(k, "store", "a literal store", k.span());
                    self.fate_move(v, "store", "a literal store", v.span());
                }
                let key = expected_key.unwrap_or_else(|| {
                    if key_tys.is_empty() {
                        Ty::Unknown
                    } else {
                        self.mk_union(key_tys)
                    }
                });
                let val = expected_val.unwrap_or_else(|| {
                    if val_tys.is_empty() {
                        Ty::Unknown
                    } else {
                        self.mk_union(val_tys)
                    }
                });
                if let Some(reason) = self.key_ineligible(&key) {
                    self.error(*span, reason);
                }
                self.collection_lit_ty("Map", vec![key, val], expected)
            }
            Expr::Tuple { elems, .. } => {
                let expected_elems: Option<&Vec<Ty>> = match expected {
                    Some(Ty::Tuple(ts)) if ts.len() == elems.len() => Some(ts),
                    _ => None,
                };
                let mut tys = Vec::new();
                for (i, e) in elems.iter().enumerate() {
                    tys.push(self.check_expr(e, expected_elems.map(|ts| &ts[i])));
                }
                // [linear-composite] A tuple is a composite too.
                for (e, ty) in elems.iter().zip(&tys) {
                    if self.ty_own_linear(ty) {
                        let linear = format!("{ty}");
                        self.refuse_linear_composite(
                            e.span(),
                            &linear,
                            "a tuple component".to_string(),
                        );
                    }
                }
                // Storing a value in a tuple literal moves it
                // [deduce-consume].
                for e in elems {
                    self.fate_move(e, "store", "a literal store", e.span());
                }
                Ty::Tuple(tys)
            }
            Expr::StructLit { ty, fields, span } => {
                let struct_ty = self.check_struct_lit(ty.as_ref(), fields, expected, *span);
                // Storing a value in a struct literal moves it; spreading
                // a value moves it too (its fields are stored)
                // [deduce-consume].
                let proj_fields = self.proj_fields_of(&struct_ty);
                for f in fields {
                    match &f.kind {
                        // [proj-field] A `proj` field *borrows* what is
                        // stored in it: the literal becomes derived from the
                        // value (see `links_for_value`), and nothing moves.
                        StructLitFieldKind::Named { name, .. }
                            if proj_fields.contains(&name.name) => {}
                        StructLitFieldKind::Named { value, .. } => {
                            self.fate_move(value, "store", "a literal store", value.span());
                        }
                        StructLitFieldKind::Spread(e) => {
                            self.fate_move(e, "spread", "a `...` spread", e.span());
                        }
                    }
                }
                struct_ty
            }
            Expr::Unary { op, operand, .. } => {
                match op {
                    // [op-arith] Negation is numeric. The expectation
                    // passes through, so `let x: Long = -5` adopts
                    // [lit-adopt].
                    UnaryOp::Neg => {
                        let t = self.check_expr(operand, expected);
                        let tb = t.strip_quals();
                        if !op_lenient(tb) && op_numeric(tb).is_none() {
                            self.error(
                                operand.span(),
                                format!(
                                    "unary `-` needs a numeric operand (`Int`, `Long`, \
                                     `Float`, `Double`); found `{t}`"
                                ),
                            );
                        }
                        t
                    }
                    // [op-bool] Negation of a truth value: `Bool` only —
                    // Salvo has no truthiness.
                    UnaryOp::Not => {
                        let t = self.check_expr(operand, None);
                        self.require_bool_operand("!", operand, &t);
                        Ty::named("Bool")
                    }
                }
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                use BinaryOp::*;
                match op {
                    Add | Sub | Mul | Div | Rem => {
                        let l = self.check_expr(lhs, None);
                        let r = self.check_expr(rhs, None);
                        // [op-no-none] Arithmetic never operates on a
                        // possibly-absent value.
                        self.reject_optional_operand(*op, lhs, &l);
                        self.reject_optional_operand(*op, rhs, &r);
                        // [str-drop-mut] An operator works on plain values.
                        self.drop_mut_operand(lhs, &l);
                        self.drop_mut_operand(rhs, &r);
                        // [op-arith] Numeric operands, widening within a
                        // class, promoted result type.
                        self.check_arith(*op, expr.span(), lhs, rhs, &l, &r)
                    }
                    And | Or => {
                        // Value-position boolean: no narrowing propagation.
                        let l = self.check_expr(lhs, None);
                        let r = self.check_expr(rhs, None);
                        // [op-bool] Both sides are truth values.
                        self.require_bool_operand(op_symbol(*op), lhs, &l);
                        self.require_bool_operand(op_symbol(*op), rhs, &r);
                        Ty::named("Bool")
                    }
                    _ => {
                        let l = self.check_expr(lhs, None);
                        let r = self.check_expr(rhs, None);
                        // [op-no-none] Comparison (ordering and equality)
                        // likewise: nullability is tested with `is None`.
                        self.reject_optional_operand(*op, lhs, &l);
                        self.reject_optional_operand(*op, rhs, &r);
                        // [str-drop-mut] Equality especially: a builder
                        // compares by identity where a string compares by
                        // content.
                        self.drop_mut_operand(lhs, &l);
                        self.drop_mut_operand(rhs, &r);
                        // [col-equality] [op-order] Comparison operands must
                        // be compatible, and what may be compared at all
                        // depends on the operator: `==`/`!=` work on any
                        // struct, ordering on numerics (widened) and structs
                        // declaring `canbe ordered`.
                        self.check_comparison_operands(*op, expr.span(), lhs, rhs, &l, &r);
                        Ty::named("Bool")
                    }
                }
            }
            Expr::Is { .. } => {
                self.is_info(expr);
                Ty::named("Bool")
            }
            // [qual-widen] Boolean-valued like `is`; the flow facts are
            // produced by `analyze_cond` in condition position.
            Expr::Widen {
                subject,
                quals,
                span,
            } => {
                self.widen_info(subject, quals, *span);
                Ty::named("Bool")
            }
            Expr::NonNull { operand, .. } => {
                let t = self.check_expr(operand, None);
                t.without_none()
            }
            // [inc-dec] All four forms do the same thing to the operand — a
            // step of one on an `Int` place — so the checker ignores the
            // direction and the fixity; they only decide the expression's
            // *value*, which is the emitters' business.
            Expr::IncDec {
                operand,
                down: _,
                prefix: _,
                span,
            } => {
                let ty = self.check_expr(operand, None);
                // `i++` rebinds a whole variable (revival: severs its own
                // links, poisons variables derived from it [fate-poison]);
                // through a projection it mutates the root
                // [fate-derived-readonly].
                match operand.as_ref() {
                    Expr::Ident(id) => {
                        let root = self.lookup(&id.name).map(|var| (var.id, id.name.clone()));
                        if let Some((root_id, root_name)) = root {
                            self.poison_derived(
                                root_id,
                                &root_name,
                                FateEvent::Reassigned,
                                *span,
                                // A whole variable: every derived value falls
                                // [fate-field-disjoint].
                                Some(&[]),
                            );
                        }
                        if let Some(var) = self.lookup_mut(&id.name) {
                            var.links = Vec::new();
                            var.poison = None;
                            // `i++` rebinds the variable: facts about its
                            // parts fall [flow-place-invalidate].
                            var.place_narrows.clear();
                        }
                    }
                    other => {
                        self.fate_mutation_through(other, *span);
                    }
                }
                ty
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => self.check_if(branches, else_block.as_ref(), expr.span()),
            Expr::When {
                subject,
                branches,
                span,
            } => self.check_when(subject, branches, *span),
            // [when-condition] The subject-less form is a condition chain
            // with a mandatory `else`: the same checking as
            // `if`/`elif`/`else`, and the `else` is what keeps `None` out of
            // the value type.
            Expr::WhenCond {
                branches,
                else_block,
                span,
            } => self.check_if(branches, Some(else_block), *span),
            // [try] The throw delimiter: an intrinsic, not an effect.
            Expr::Try { body, span } => self.check_try(body, *span),
            // [actor-spawn-expr] The asynchronous binding of a handler: its
            // value is the child's `Addr<E>`.
            Expr::Spawn {
                handler,
                uses,
                pool,
                span,
            } => self.check_spawn(handler, uses, pool.as_deref(), *span),
            // [actor-self-send] A selector is a *callee*, never a value: a
            // handler member is not a function value any more than an effect
            // member is [effect-not-data]. Reached only when one is written
            // without a call.
            Expr::SelfScoped { name, span } => {
                self.error(
                    *span,
                    format!(
                        "`{}@self` names a member of the enclosing handler, which is \
                         not a value: call it (`{}@self(…)`) to send it a message",
                        name.name, name.name
                    ),
                );
                Ty::Unknown
            }
            // [actor-replyto] A parked one-shot continuation targeting a
            // member of the enclosing handler; its value is the linear token.
            Expr::ReplyTo {
                member,
                captures,
                gated,
                pool,
                span,
            } => self.check_replyto(member, captures, *gated, pool.as_deref(), *span),
            // [actor-waitfor] `main`'s bridge: its value is what was sent to
            // the token it mints.
            Expr::WaitFor {
                binding,
                ty,
                body,
                span,
            } => self.check_waitfor(binding, ty, body, *span),
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                // [while-value] The loop is an expression: its value is the
                // body's tail (last iteration), a `break value`, or the
                // `else` tail when the loop never ran.
                // [is-bind-once] A binding `is` over a non-place subject is the
                // loop's whole condition or it is refused.
                self.reject_unhoistable_is(cond, true, "inside a larger condition");
                let info = self.analyze_cond(cond);
                self.loop_stack.push(LoopCtx {
                    entry_depth: self.locals.len(),
                    ..LoopCtx::default()
                });
                let (body_ty, body_tail) =
                    self.check_loop_body(body, &info.then_narrows.clone(), info.bindings.clone());
                let mut ctx = self.loop_stack.pop().expect("loop ctx pushed above");
                // The code after the loop is reached from the fall-through
                // exit *and* from every `break`: merge the break-path
                // states in, so a value consumed on a break path stays
                // consumed after the loop [deduce-consume].
                let break_states = std::mem::take(&mut ctx.break_states);
                if !break_states.is_empty() {
                    let mut states = vec![self.snapshot_narrows()];
                    states.extend(break_states);
                    self.merge_fallthrough(&states, expr.span());
                }
                // `else` runs only when the loop never ran, i.e. the
                // condition failed on first evaluation: else-narrows apply.
                let (else_ty, else_tail) = match else_block {
                    Some(b) => {
                        let (t, tail) = self.with_narrows(&info.else_narrows.clone(), |c| {
                            c.check_branch_block(b, Vec::new())
                        });
                        (Some(t), tail)
                    }
                    None => (None, None),
                };
                self.reset_assigned(body);
                if let Some(b) = else_block {
                    self.reset_assigned(b);
                }
                self.finish_loop_value(body_ty, body_tail, ctx, else_ty, else_tail)
            }
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => {
                let iter_ty = self.check_expr(iterable, None);
                let elem = self.iter_elem_ty(&iter_ty, iterable, iterable.span());
                // [once-fn] Driving a **pass** consumes it: `once Iter<T>`
                // is a position in a sequence, not a recipe, so a second
                // `for` over the same value is the ordinary consumed-use
                // error and the elements are owned by the loop rather than
                // derived from a value that is still alive. A plain
                // `Iter<T>` is a factory: `for` mints a pass from it and
                // leaves it usable, exactly as before.
                // [iter-fn] An *origin* is not a pass: each `for`
                // mints a fresh hidden machine from it, so driving neither
                // consumes nor mutates it and the binding stays an ordinary
                // projection. Only a real pass — the value that *holds* the
                // position — is moved into the loop.
                let recorded = self
                    .out
                    .for_drivers
                    .get(&self.key(iterable.span()))
                    .cloned();
                let drives_origin = recorded.as_ref().is_some_and(|d| d.origin);
                // [iter-drive-in-place] A pass the fn *keeps* is advanced where
                // it lives: the loop mutates it rather than taking it over, so
                // the caller sees the position it reached and may drive it on.
                let drives_in_place = recorded.as_ref().is_some_and(|d| d.in_place);
                let drives_pass = !drives_origin
                    && !drives_in_place
                    && (iter_ty.quals().iter().any(|q| q.name == "once")
                        || self.yield_obligation(iter_ty.strip_quals()).is_some()
                        // [iter-generic-drive] A generic pass is a pass: the
                        // implicit `next` in the driver is the declaration.
                        || recorded
                            .as_ref()
                            .is_some_and(|d| matches!(d.next, PassMember::Implicit(_))));
                if drives_in_place {
                    if let Some(place) = crate::place::Place::of_expr(iterable) {
                        let root = place.root.clone();
                        self.fate_mutation_root(&root, iterable.span());
                    }
                }
                // [yield-proj] Whether the pass's `next` emits a *borrow*:
                // then each element shares fate with the pass's source, and
                // driving the pass is a read of that source, not a move.
                let next_borrows = recorded.as_ref().is_some_and(|d| match &d.next {
                    PassMember::Fn(key) => self
                        .scope
                        .fns
                        .values()
                        .flatten()
                        .find(|e| e.key == *key)
                        .is_some_and(|e| {
                            e.decl
                                .return_type
                                .as_ref()
                                .is_some_and(|t| !proj_arm_indices(t).is_empty())
                        }),
                    PassMember::Implicit(_) => false,
                });
                // A pass that is itself a *view* — a local bound from
                // `iter(xs)`, or a call result borrowing its argument — has
                // links of its own; those are the source the elements borrow.
                let view_links: Vec<FateLink> = match iterable.as_ref() {
                    Expr::Ident(id) => self
                        .lookup(&id.name)
                        .map(|v| v.links.clone())
                        .unwrap_or_default(),
                    other => self.links_for_value(other, other.span()),
                };
                let is_view = !view_links.is_empty();
                let links = if drives_pass || drives_in_place {
                    if drives_pass && !is_view {
                        self.fate_move(iterable, "iterate", "a `for` loop", iterable.span());
                    }
                    if next_borrows && is_view {
                        // The element is a borrow of what the pass walks: it
                        // links to the pass's own links (its source), and the
                        // `borrowed` flag keeps it from ever taking ownership
                        // [readonly-return].
                        view_links
                            .into_iter()
                            .map(|mut l| {
                                l.borrowed = true;
                                l
                            })
                            .collect()
                    } else {
                        // A pass **hands the element over**: `next` returns
                        // `Emitted T` by value, so the binding is an owned
                        // value and not a projection of the pass — which is
                        // what lets a combinator move each element into its
                        // output.
                        Vec::new()
                    }
                } else {
                    // The loop binding is a projection of the iterated
                    // collection: it shares the collection's fate
                    // [fate-link]. It goes through the per-pass bindings
                    // channel so every checking pass re-declares it fresh —
                    // each iteration binds a new element [deduce-consume].
                    self.links_for_value(iterable, iterable.span())
                };
                // [proj-type] Elements are projections of what the loop walks
                // — unless the loop *owns* what it walks: a temporary
                // (`for s in list_of("x", "y")`) or a pass moved into the loop
                // dies with it, so its elements are the loop's to give away.
                // Nothing to share fate with means nothing to borrow from.
                let elem = if links.is_empty() {
                    elem.strip_top_proj()
                } else {
                    elem
                };
                // [let-destructure] A loop element may be destructured: the
                // header binds it to a temporary and the body opens with the
                // pattern's own bindings, read off that temporary.
                let bindings = self.pattern_bindings(pattern, elem, links, Some(iterable.span()));
                self.loop_stack.push(LoopCtx {
                    entry_depth: self.locals.len(),
                    ..LoopCtx::default()
                });
                // [iter-fn] While this loop is open, the origin it
                // drives is off limits for mutation. Keyed on the subject's
                // *root* variable, so `add(b.rows, …)` — a write through a
                // projection — reports too: everything reachable from the
                // origin is what the machine may read.
                let driven = if drives_origin {
                    crate::place::Place::of_expr(iterable).map(|p| p.root.clone())
                } else {
                    None
                };
                if let Some(root) = &driven {
                    self.driven_origins.push((root.clone(), iterable.span()));
                }
                let (body_ty, body_tail) = self.check_loop_body(body, &[], bindings);
                if driven.is_some() {
                    self.driven_origins.pop();
                }
                let mut ctx = self.loop_stack.pop().expect("loop ctx pushed above");
                // Merge break-path states into the after-loop state (the
                // loop-binding frame is popped first, so the snapshots'
                // shared outer frames align) [deduce-consume].
                let break_states = std::mem::take(&mut ctx.break_states);
                if !break_states.is_empty() {
                    let mut states = vec![self.snapshot_narrows()];
                    states.extend(break_states);
                    self.merge_fallthrough(&states, expr.span());
                }
                let (else_ty, else_tail) = match else_block {
                    Some(b) => {
                        let (t, tail) = self.check_branch_block(b, Vec::new());
                        (Some(t), tail)
                    }
                    None => (None, None),
                };
                self.reset_assigned(body);
                if let Some(b) = else_block {
                    self.reset_assigned(b);
                }
                self.finish_loop_value(body_ty, body_tail, ctx, else_ty, else_tail)
            }
            Expr::Lambda { params, body, span } => self.check_lambda(params, body, *span, expected),
            Expr::Spread { operand, .. } => self.check_expr(operand, None),
            Expr::Error { .. } => Ty::Unknown,
        }
    }

    /// Records where an accessed field is declared [lsp-definition]. The
    /// struct's own field, even when a predicate qualifier overrides its
    /// type [qual-field-override]: the override refines the field, it does
    /// not replace the declaration.
    fn record_field_ref(&mut self, base_ty: &Ty, field: &Ident) {
        let Ty::Named { name, .. } = base_ty.strip_quals() else {
            return;
        };
        let Some(decl) = self.scope.structs.get(name.as_str()).copied() else {
            return;
        };
        let Some(f) = decl.fields.iter().find(|f| f.name.name == field.name) else {
            return;
        };
        // The struct's `DefSite` names the file it was declared in; the
        // field span comes from that same declaration.
        let Some(site) = self.scope.def_sites.get(name.as_str()).copied() else {
            return;
        };
        self.out.field_refs.insert(
            self.key(field.span),
            DefSite {
                file: site.file,
                span: f.name.span,
            },
        );
    }

    fn field_ty_or_error(&mut self, base_ty: &Ty, field: &Ident) -> Ty {
        match self.field_ty(base_ty, &field.name) {
            Some(t) => t,
            None => {
                // [field-resolve] Only structs have fields, and only the
                // ones they declare. A target-language member is reached by
                // declaring an accessor, not by reading
                // through an opaque type — so an unknown field is an error
                // (user decision 2026-09-03). A generic value exposes
                // nothing either: Salvo has no bounds, so `T` is opaque.
                // Only an *un-inferred* base (`Unknown`) stays lenient
                // [type-unknown-lenient].
                let stripped = base_ty.strip_quals().clone();
                match &stripped {
                    Ty::Named { name, .. } if self.scope.structs.contains_key(name.as_str()) => {
                        let name = name.clone();
                        self.error(
                            field.span,
                            format!("struct `{name}` has no field `{}`", field.name),
                        );
                    }
                    Ty::Unknown | Ty::Nothing => {}
                    // A generic parameter needs its own wording: adding an
                    // accessor cannot help, because nothing is known about
                    // `T` at all.
                    Ty::Var(name) => {
                        let name = name.clone();
                        self.error(
                            field.span,
                            format!(
                                "`{name}` is a type parameter, so nothing is known \
                                 about its values: it has no field `{}`. Take the \
                                 concrete type as a parameter instead",
                                field.name
                            ),
                        );
                    }
                    other => {
                        let other = other.clone();
                        self.error(
                            field.span,
                            format!(
                                "`{other}` has no field `{}`: only structs have \
                                 fields, and a target-language member needs an \
                                 accessor fn",
                                field.name
                            ),
                        );
                    }
                }
                Ty::Unknown
            }
        }
    }

    fn field_ty(&mut self, base_ty: &Ty, field_name: &str) -> Option<Ty> {
        if let Some(t) = self.field_override_ty(base_ty, field_name) {
            return Some(t);
        }
        self.declared_field_ty(base_ty, field_name)
    }

    /// The type of `base.index` [expr-tuple-index]: the element at a
    /// constant position of a tuple. Only tuples have elements, and the
    /// position must exist — both are errors rather than leniency, since
    /// the index is written in the source and cannot be interop-dependent.
    /// A non-tuple `Unknown` base stays lenient
    /// [type-unknown-lenient].
    fn tuple_elem_ty_or_error(&mut self, base_ty: &Ty, index: usize, span: Span) -> Ty {
        match base_ty.strip_quals() {
            Ty::Tuple(elems) => match elems.get(index) {
                Some(t) => t.clone(),
                None => {
                    let len = elems.len();
                    self.error(
                        span,
                        format!(
                            "tuple `{base_ty}` has {len} element(s), so it has no \
                             element `{index}`"
                        ),
                    );
                    Ty::Unknown
                }
            },
            other if other.is_unknown() || matches!(other, Ty::Nothing) => Ty::Unknown,
            _ => {
                self.error(
                    span,
                    format!(
                        "`.{index}` reads a tuple element, but `{base_ty}` is not a \
                         tuple"
                    ),
                );
                Ty::Unknown
            }
        }
    }

    /// A predicate-qualifier field override for this (qualified) base type,
    /// e.g. `surname: Str` under `Surname Person`.
    fn field_override_ty(&mut self, base_ty: &Ty, field_name: &str) -> Option<Ty> {
        for qual in base_ty.quals() {
            let Some(decl) = self.qualifier_for(qual.name.as_str(), Some(base_ty)) else {
                continue;
            };
            let Some(field) = decl
                .field_overrides
                .iter()
                .find(|f| f.name.name == field_name)
            else {
                continue;
            };
            let mut subst = HashMap::new();
            for (i, g) in decl.generics.iter().enumerate() {
                subst.insert(
                    g.name.clone(),
                    qual.args.get(i).cloned().unwrap_or(Ty::Unknown),
                );
            }
            let field_ty = field.ty.clone();
            return Some(self.lower_type_subst(&field_ty, &subst, 0));
        }
        None
    }

    /// The declared struct-field type, ignoring qualifier overrides.
    fn declared_field_ty(&mut self, base_ty: &Ty, field_name: &str) -> Option<Ty> {
        let Ty::Named { name, args } = base_ty.strip_quals() else {
            return None;
        };
        let decl: &'p StructDecl = self.scope.structs.get(name.as_str())?;
        let field = decl.fields.iter().find(|f| f.name.name == field_name)?;
        let mut subst = HashMap::new();
        for (i, g) in decl.generics.iter().enumerate() {
            subst.insert(g.name.clone(), args.get(i).cloned().unwrap_or(Ty::Unknown));
        }
        Some(self.lower_type_subst(&field.ty, &subst, 0))
    }

    /// [iter-protocol] [group-obligation] Whether this type's declaration
    /// states the designated `: Yield<T>` obligation — the one fact that
    /// makes a value a **pass** (roadmap R2, user decisions 2026-09-08).
    /// `for` reads the declaration; it does not scan overloads for a `next`
    /// and guess.
    fn yield_obligation(&self, stripped: &Ty) -> Option<(&'p StructDecl, &'p TypeRef)> {
        let Ty::Named { name, .. } = stripped else {
            return None;
        };
        let decl: &'p StructDecl = self.scope.structs.get(name.as_str()).copied()?;
        let ob = decl.obligations.iter().find(|o| o.name.name == "Yield")?;
        Some((decl, ob))
    }

    /// [implicit-group] [iter-protocol] A `?Yield<It, T>` spread teaches `T`
    /// from the *declaration* of whatever `It` turned out to be: a pass says
    /// what it yields at its `: Yield<self, T>` clause, so a combinator's
    /// element type never has to be written and a bare lambda can be typed
    /// against it. Reaches through an origin mint [iter-fn].
    ///
    /// [proj-type] The pass also decides whether its elements are *borrowed*:
    /// a binding an argument made (`(s: Str) -> …`) is widened to the pass's
    /// `proj Str` when it fits under it — a kept lambda parameter reads the
    /// projection just fine, and the `next` that fills the spread returns it.
    fn learn_yield_elems(
        &mut self,
        decl: &'p FnDecl,
        callee_generics: &HashSet<String>,
        subst: &mut HashMap<String, Ty>,
    ) {
        for (state_var, elem_var) in self.yield_spread_pairs(decl) {
            if !callee_generics.contains(&elem_var) {
                continue;
            }
            let Some(state) = subst.get(&state_var).cloned() else {
                continue;
            };
            let Some(elem) = self.pass_or_origin_elem_ty(&state) else {
                continue;
            };
            if elem.is_unknown() {
                continue;
            }
            match subst.get(&elem_var).cloned() {
                None => {
                    subst.insert(elem_var, elem);
                }
                Some(bound) if bound != elem && is_subtype(&bound, &elem) => {
                    subst.insert(elem_var, elem);
                }
                Some(_) => {}
            }
        }
    }

    /// The `(state, element)` type-variable pairs a signature spreads as
    /// `?Yield<It, T>` — what a combinator's element type can be *inferred*
    /// from [implicit-group].
    fn yield_spread_pairs(&self, decl: &'p FnDecl) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for g in &decl.implicit_groups {
            if g.name.name != "Yield" || g.args.len() < 2 {
                continue;
            }
            let (
                Some(ast::Type::Named { base: state, .. }),
                Some(ast::Type::Named { base: elem, .. }),
            ) = (g.args.first(), g.args.get(1))
            else {
                continue;
            };
            out.push((state.name.name.clone(), elem.name.name.clone()));
        }
        out
    }

    /// The element type of a pass **or** of the origin behind a minted machine
    /// [iter-fn]: read from the `: Yield<self, T>` clause, which is
    /// where a pass declares what it yields [group-obligation].
    fn pass_or_origin_elem_ty(&mut self, state: &Ty) -> Option<Ty> {
        let bare = state.strip_quals().clone();
        if let Some(elem) = self.pass_declared_elem_ty(&bare) {
            return Some(elem);
        }
        // A machine stands for its origin, whose declaration carries the clause.
        let Ty::Named { name, args } = &bare else {
            return None;
        };
        let origin = name.strip_prefix(ORIGIN_PASS_PREFIX)?;
        let origin_ty = Ty::Named {
            name: origin.to_string(),
            args: args.clone(),
        };
        self.pass_declared_elem_ty(&origin_ty)
    }

    /// The `next` overload a subject drives, found by unifying the protocol
    /// shape against the subject's bare type: the overload, the element
    /// type, the arm identity, and the (unsubstituted) state parameter
    /// type. Extracted from `pass_elem_ty` so the not-iterable diagnostic
    /// can use the same scan for its remedy hint.
    fn find_next_driver(&mut self, stripped: &Ty) -> Option<(FnKey, Ty, usize, usize, Ty)> {
        let entries: Vec<crate::resolve::FnEntry<'p>> = self.scope.fns.get("next")?.clone();
        for entry in entries {
            let decl = entry.decl;
            if decl.params.len() != 1 {
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            let pt = self.lower_type(&decl.params[0].ty);
            let ret = decl
                .return_type
                .as_ref()
                .map(|t| self.lower_type(t))
                .unwrap_or_else(Ty::none);
            self.generics = saved;
            let mut subst = HashMap::new();
            // The parameter is `Mut St` [iter-protocol]: advancing a pass
            // mutates its position. Match on the bare types.
            if !unify(pt.strip_quals(), stripped, &mut subst) {
                continue;
            }
            let callee_generics: HashSet<String> =
                decl.generics.iter().map(|g| g.name.clone()).collect();
            let ret = substitute_vars(&ret, &subst, &callee_generics);
            let Some((elem, emitted_arm, arms)) = emitted_arm_ty(&ret) else {
                // A `next` of some other shape is not the protocol; keep
                // looking, and let `iter` have its turn.
                continue;
            };
            return Some((entry.key, elem, emitted_arm, arms, pt));
        }
        None
    }

    /// [iter-protocol] The element type of a **pass**: a value whose type
    /// declares `: Yield<T>` [group-obligation]. Records the overload the
    /// emitters have to drive (there is no call node in the AST for them to
    /// resolve, since the driving loop is synthesized).
    ///
    /// The declaration is the gate; the scan only resolves *which* overload
    /// (and the arm identity). When the declaration is there but no overload
    /// fits, the obligation check has already reported it at the struct —
    /// the declared element type is returned so the one mistake does not
    /// cascade [type-unknown-lenient].
    fn pass_elem_ty(
        &mut self,
        stripped: &Ty,
        iterable: Option<&'p Expr>,
        span: Span,
    ) -> Option<Ty> {
        self.pass_elem_ty_minted(stripped, iterable, span, None)
    }

    /// The same, for a pass the loop **mints** from a container through the
    /// `iter` overload `mint` [iter-pass]: the driving is identical, and the
    /// emitters get told to call `iter` once before the loop.
    fn pass_elem_ty_minted(
        &mut self,
        stripped: &Ty,
        iterable: Option<&'p Expr>,
        span: Span,
        mint: Option<FnKey>,
    ) -> Option<Ty> {
        let (decl, ob) = self.yield_obligation(stripped)?;
        if let Some((key, elem, emitted_arm, arms, pt)) = self.find_next_driver(stripped) {
            // [iter-protocol] The state is taken as `Mut`: advancing a pass
            // mutates its position, and the backends pass a mutable place.
            // A `next` of the right *result* shape whose state is not `Mut`
            // clearly means to be the protocol, so say what is wrong rather
            // than falling through to a puzzling "not iterable".
            if !pt.quals().iter().any(|q| q.name == "Mut") {
                self.error(
                    span,
                    format!(
                        "`next` has to take its state as `Mut {}` — advancing a \
                         pass mutates its position",
                        pt.strip_quals()
                    ),
                );
            }
            // [linear-group] **No implicit discharge sites** (user decision
            // 2026-09-12, replacing the 2026-09-09 loop-splice): a `for`
            // never closes a pass. A linear pass must be *kept* — bound
            // with `let`, driven in place, and explicitly discharged — so
            // a consuming drive of one is refused here rather than
            // leaking on the loop's exits.
            // [iter-drive-in-place] A pass the fn keeps is the caller's: it
            // advances where it lives, and its discharge stays the owner's.
            let in_place = iterable.is_some_and(|e| self.drives_in_place(e));
            if !in_place && self.inferred.is_some() && self.ty_own_linear(stripped) {
                let hint = self.linear_discharge_hint(stripped);
                self.error(
                    span,
                    format!(
                        "a `for` cannot consume a linear pass (`{stripped}`): the \
                         loop never discharges what it drives — bind the pass with \
                         `let`, loop over it, then discharge it with {hint}"
                    ),
                );
            }
            self.out.for_drivers.insert(
                self.key(span),
                PassDriver {
                    next: PassMember::Fn(key),
                    emitted_arm,
                    arms,
                    origin: false,
                    in_place,
                    mint_iter_fn: mint,
                },
            );
            return Some(elem);
        }
        // Declared but not satisfied: already an error at the struct
        // [group-obligation]. Answer with the declared element type.
        let subst: HashMap<String, Ty> = decl
            .generics
            .iter()
            .map(|g| g.name.clone())
            .zip(match stripped {
                Ty::Named { args, .. } => args.clone(),
                _ => Vec::new(),
            })
            .collect();
        let saved = self.enter_generics(&decl.generics);
        let elem = yield_elem_arg(ob)
            .map(|a| self.lower_type_subst(a, &subst, 0))
            .unwrap_or(Ty::Unknown);
        self.generics = saved;
        Some(elem)
    }

    /// [iter-pass] The `iter` overload that turns this container into a pass,
    /// with the pass type it answers (substituted for the subject). Passes are
    /// looked for *first* by the caller: a value that is already a position in
    /// a sequence must not have a second pass minted from it.
    fn iter_pass_for(&mut self, subject: &Ty) -> Option<(FnKey, Ty)> {
        let entries: Vec<crate::resolve::FnEntry<'p>> = self.scope.fns.get("iter")?.clone();
        for entry in entries {
            let decl = entry.decl;
            if decl.params.len() != 1 {
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            let pt = self.lower_type(&decl.params[0].ty);
            let ret = decl
                .return_type
                .as_ref()
                .map(|t| self.lower_type(t))
                .unwrap_or_else(Ty::none);
            self.generics = saved;
            let mut subst = HashMap::new();
            if !unify(&pt, subject, &mut subst) {
                continue;
            }
            let callee_generics: HashSet<String> =
                decl.generics.iter().map(|g| g.name.clone()).collect();
            let ret = substitute_vars(&ret, &subst, &callee_generics);
            if self.yield_obligation(ret.strip_quals()).is_some() {
                return Some((entry.key, ret.strip_quals().clone()));
            }
        }
        None
    }

    /// The element type a pass *declares* it yields, read from the obligation
    /// alone [group-obligation] — no `next` resolved and no driver recorded,
    /// which is what the native container loop needs [iter-for-native].
    fn pass_declared_elem_ty(&mut self, pass: &Ty) -> Option<Ty> {
        let (decl, ob) = self.yield_obligation(pass)?;
        let subst: HashMap<String, Ty> = decl
            .generics
            .iter()
            .map(|g| g.name.clone())
            .zip(match pass {
                Ty::Named { args, .. } => args.clone(),
                _ => Vec::new(),
            })
            .collect();
        let saved = self.enter_generics(&decl.generics);
        let elem = yield_elem_arg(ob).map(|a| self.lower_type_subst(a, &subst, 0));
        self.generics = saved;
        elem
    }

    /// [iter-drive-in-place] Whether a `for` subject is a place the enclosing
    /// fn **keeps**: a parameter its contract hands back, or a projection of
    /// one. Driving such a pass has to be visible to the caller, so the loop
    /// advances it in place rather than binding it into a local.
    fn drives_in_place(&self, iterable: &'p Expr) -> bool {
        // [iter-drive-in-place] Any *named place* is driven where it lives
        // (user decision 2026-09-12, extending the kept-parameter rule to
        // locals and owned parameters): a `for` never consumes a variable,
        // so a linear pass is bound, driven, and explicitly discharged
        // after — the loop is not a discharge site. Only a *temporary*
        // subject (a minted pass, a call result) is consumed by the loop.
        crate::place::Place::of_expr(iterable)
            .map(|p| self.lookup(&p.root).is_some())
            .unwrap_or(false)
    }

    /// [iter-generic-drive] The element type of a **generic** pass: the
    /// subject's type is one of the enclosing fn's type parameters, and a
    /// protocol-shaped `next` for it is in scope as an *implicit parameter* —
    /// the `?Yield<It, T>` spread a combinator declares [implicit-group] (user
    /// decision 2026-09-09).
    ///
    /// The spread is the declaration `for` reads here: there is no struct to
    /// carry a `: Yield<self, T>` clause, but the position that says "this call
    /// supplies a `next` for `It`" says exactly as much, and the element type
    /// falls out of its result. The loop therefore drives by **calling the
    /// implicit parameter** rather than a resolved overload.
    fn generic_pass_elem_ty(&mut self, subject: &Ty, iterable: &'p Expr, span: Span) -> Option<Ty> {
        if !matches!(subject, Ty::Var(_)) {
            return None;
        }
        // The protocol shape, against the implicits this body has: one
        // parameter the subject fits, and an `Emitted T | Finished` result.
        let mut found: Option<(String, Ty, usize, usize, bool)> = None;
        for imp in &self.own_implicits {
            // The protocol's member name, as everywhere else: `for` drives
            // `next`, whatever grouping brought the position in [iter-protocol].
            if imp.name != "next" {
                continue;
            }
            let Ty::Fn { params, ret, .. } = imp.ty.strip_quals() else {
                continue;
            };
            if params.len() != 1 || params[0].strip_quals() != subject {
                continue;
            }
            let Some((elem, emitted_arm, arms)) = emitted_arm_ty(ret) else {
                continue;
            };
            let mutable = params[0].quals().iter().any(|q| q.name == "Mut");
            found = Some((imp.name.clone(), elem, emitted_arm, arms, mutable));
            break;
        }
        let (name, elem, emitted_arm, arms, mutable) = found?;
        // The state is taken as `Mut` [iter-protocol]: advancing a pass mutates
        // its position, and the backends hand over a mutable place. A position
        // of the right *result* shape that does not say so plainly means to be
        // the protocol, so say what is wrong.
        if !mutable {
            self.error(
                span,
                format!(
                    "`{name}` has to take its state as `Mut {subject}` — advancing a \
                     pass mutates its position"
                ),
            );
        }
        // [iter-drive-in-place] A pass the fn keeps advances where it lives, and
        // its release stays the caller's. [linear-group] A consuming drive
        // of a possibly-linear pass is refused — no implicit discharge
        // sites (user decision 2026-09-12) — and the remedy is the same as
        // for a concrete pass: keep it, or take a consuming callback and
        // hand the pass to it.
        let in_place = self.drives_in_place(iterable);
        if !in_place && self.inferred.is_some() && self.ty_own_linear(subject) {
            self.error(
                span,
                format!(
                    "a `for` cannot consume a pass that may be linear \
                     (`{subject}` is `canbe linear`): the loop never discharges \
                     what it drives — keep the pass (drive it in place and let \
                     the caller discharge it), or take a consuming callback \
                     (`end: ({subject}) -> None` with `=>[end] !it`) and hand \
                     the pass to it after the loop"
                ),
            );
        }
        self.out.for_drivers.insert(
            self.key(span),
            PassDriver {
                next: PassMember::Implicit(name),
                emitted_arm,
                arms,
                origin: false,
                in_place,
                mint_iter_fn: None,
            },
        );
        Some(elem)
    }

    /// Whether a `for` subject is one of the containers a backend iterates
    /// natively [iter-for-native]: an `intrinsic type` (a list, a `Str`; an
    /// array is a language-level type and never reaches here). Every other
    /// container is walked through the pass its `iter` mints.
    fn is_intrinsic_container(&self, subject: &Ty) -> bool {
        match subject {
            Ty::Named { name, .. } => self
                .scope
                .opaque_types
                .get(name.as_str())
                .is_some_and(|d| d.intrinsic),
            _ => false,
        }
    }

    fn iter_elem_ty(&mut self, iter_ty: &Ty, iterable: &'p Expr, span: Span) -> Ty {
        match iter_ty.strip_quals() {
            Ty::Array(elem) => (**elem).clone(),
            other => {
                let other = other.clone();
                // [iter-protocol] A **pass** — anything with a `next` — is
                // driven directly, and is looked for *before* `iter`: a type
                // that has both is already a position in a sequence, so
                // minting a second pass from it would be wrong. This is the
                // manual half of the iterator story (`zip`, `merge`), which
                // `yield` cannot express.
                if let Some(elem) = self.pass_elem_ty(&other, Some(iterable), span) {
                    return elem;
                }
                // [iter-pass] `for x in xs` iterates a **container**: its
                // `iter` answers a fresh pass, and the loop drives that. For
                // an *intrinsic* container (a list, an array, a `Str`) no
                // driver is recorded at all — the backends iterate their own
                // data natively, which is both faster and non-consuming
                // [iter-for-native]; for anything else the `iter` call is the
                // mint, made once before the loop.
                // [iter-generic-drive] A **generic** pass: its `next` is an
                // implicit parameter of this body, which is as much of a
                // declaration as a `: Yield<self, T>` clause is.
                if let Some(elem) = self.generic_pass_elem_ty(&other, iterable, span) {
                    return elem;
                }
                if let Some((iter_key, pass_ty)) = self.iter_pass_for(&other) {
                    if self.is_intrinsic_container(&other) {
                        if let Some(elem) = self.pass_declared_elem_ty(&pass_ty) {
                            return elem;
                        }
                    } else if let Some(elem) =
                        // The *minted* pass is the loop's own value, never a
                        // place the caller keeps, so it is never driven in place.
                        self.pass_elem_ty_minted(&pass_ty, None, span, Some(iter_key))
                    {
                        return elem;
                    }
                }
                // [iter-resolve] Nothing makes this value iterable: no
                // `Iter`, no array, and no `iter` overload accepts it. An
                // un-inferred value stays lenient
                // [type-unknown-lenient]; everything else is an error
                // (user decision 2026-09-03).
                if !other.is_unknown() && !matches!(other, Ty::Nothing) {
                    // A matching protocol-shaped `next` without the
                    // declaration is the likeliest near-miss; name the
                    // remedy [group-obligation] rather than leaving a
                    // puzzling "not iterable".
                    let hint = match self.find_next_driver(&other) {
                        Some((_, elem, _, _, _)) => format!(
                            " (`{other}` has a matching `next` — declare \
                             `: Yield<self, {elem}>` on it to make it a pass)"
                        ),
                        None => String::new(),
                    };
                    self.error(
                        span,
                        format!(
                            "`{other}` is not iterable: `for` takes an array, a \
                             pass (a type declaring `: Yield<self, T>`), or a value \
                             some `iter` function accepts{hint}"
                        ),
                    );
                }
                Ty::Unknown
            }
        }
    }

    fn check_lambda(
        &mut self,
        params: &'p [LambdaParam],
        body: &'p LambdaBody,
        span: Span,
        expected: Option<&Ty>,
    ) -> Ty {
        let (exp_params, exp_ret, exp_contract, exp_effects) =
            match expected.map(|t| t.strip_quals()) {
                Some(Ty::Fn {
                    params,
                    ret,
                    contract,
                    effects,
                }) => (
                    Some(params.clone()),
                    Some((**ret).clone()),
                    contract.clone(),
                    Some(effects.clone()),
                ),
                _ => (None, None, None, None),
            };
        // [fate-lambda] Everything below this frame boundary is a
        // capture; the body's reads/mutations of such variables are
        // recorded, and consuming one is an error (the lambda may run
        // any number of times).
        self.lambda_ctx.push(LambdaCtx {
            boundary: self.locals.len(),
            captures: Vec::new(),
        });
        self.locals.push(HashMap::new());
        let mut param_tys = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let ty =
                p.ty.as_ref()
                    .map(|t| self.lower_type(t))
                    .or_else(|| exp_params.as_ref().and_then(|ps| ps.get(i).cloned()))
                    .unwrap_or(Ty::Unknown);
            self.declare(&p.name, ty.clone());
            // [fn-contract] A parameter the expected contract *keeps*
            // belongs to the closure's caller: never consumable inside
            // the body.
            let kept = exp_contract
                .as_ref()
                .and_then(|c| c.get(i))
                .is_some_and(|e| e.kept);
            if kept {
                if let Some(var) = self.lookup_mut(&p.name.name) {
                    var.lambda_kept = true;
                }
            }
            param_tys.push(ty);
        }
        // [fn-contract] Exported for the emitter's parameter-binding
        // modes.
        if let Some(c) = &exp_contract {
            self.out.lambda_contracts.insert(self.key(span), c.clone());
        }
        let saved_ret = std::mem::replace(&mut self.ret_ty, exp_ret.clone().unwrap_or(Ty::Unknown));
        // [fn-effects] The body performs the effects the fn *type* declares
        // — the call site supplies them, so nothing is captured. With no
        // expected type the set is *inferred* from the body, which is what
        // `effect_uses` collects; the enclosing environment stands in for
        // the duration so the calls resolve.
        let saved_effects = match &exp_effects {
            Some(effects) => Some(std::mem::replace(&mut self.effect_env, effects.clone())),
            None => None,
        };
        self.effect_uses.push(Vec::new());
        // A lambda body is a loop barrier: `break`/`continue` inside it
        // never bind a loop enclosing the lambda expression.
        let saved_loops = std::mem::take(&mut self.loop_stack);
        // [actor-spawn-expr] [actor-waitfor] [actor-replyto] …and a barrier
        // for the asynchronous capabilities, for the same reason: a lambda is
        // a *value*, so where its body runs is decided by whoever calls it,
        // and no first-pass form may cross a closure. A fn type cannot
        // declare `spawn` [actor-spawn-effect], `waitfor` needs `main`'s own
        // thread, and a `replyto` continuation belongs to the handler that
        // minted it — none of which travels with a function value.
        let saved_can_spawn = std::mem::replace(&mut self.can_spawn, false);
        // [actor-no-closure] A lambda body carries neither capability: it runs
        // wherever it is called, and a fn type declares neither.
        let saved_can_wait = std::mem::replace(&mut self.can_wait, false);
        let saved_own_handler = self.own_handler.take();
        let ret = match body {
            LambdaBody::Expr(e) => self.check_expr(e, exp_ret.as_ref()),
            LambdaBody::Block(b) => {
                for stmt in &b.stmts {
                    self.check_stmt(stmt);
                }
                exp_ret.unwrap_or(Ty::Unknown)
            }
        };
        self.loop_stack = saved_loops;
        self.can_spawn = saved_can_spawn;
        self.can_wait = saved_can_wait;
        self.own_handler = saved_own_handler;
        self.ret_ty = saved_ret;
        // [linear-obligation] Lambda parameters are owned by the body:
        // a linear one must be discharged before the body ends.
        self.check_linear_frame_drop();
        self.locals.pop();
        let used = self.effect_uses.pop().unwrap_or_default();
        if let Some(saved) = saved_effects {
            self.effect_env = saved;
        }
        let ctx = self.lambda_ctx.pop().expect("lambda ctx pushed above");
        let consumes_captures = ctx.captures.iter().any(|c| c.moved);
        self.finish_lambda_captures(ctx, span);
        // [fn-effects] A declared set is the type's (even where the body
        // uses less of it — the emitted signature has to match what the
        // caller passes); otherwise the inferred set is the type's.
        let effects = exp_effects.unwrap_or(used);
        // What the emitters need: the effect values this lambda takes as
        // leading parameters, in order.
        self.out
            .lambda_effects
            .insert(self.key(span), effects.clone());
        let fn_ty = Ty::Fn {
            params: param_tys,
            ret: Box::new(ret),
            contract: None,
            effects,
        };
        // [once-fn] A lambda that consumes a capture is callable at most
        // once: its type gains `once`, so it only fits `once` fn
        // positions and calling it consumes it.
        if consumes_captures {
            fn_ty.qualify(vec![Qual {
                effect: false,
                name: "once".to_string(),
                args: Vec::new(),
            }])
        } else {
            fn_ty
        }
    }

    /// Applies the capture contract of a checked lambda body
    /// [fate-lambda]:
    /// - a *mutated* capture is consumed at creation — the closure took
    ///   ownership (each call mutates it, and the original observing
    ///   those mutations on one backend but not the other would break
    ///   parity); a written-kept parameter cannot be captured-and-
    ///   mutated (you cannot own what the caller keeps), and an
    ///   inferable parameter is claimed as moved;
    /// - the lambda *value* fate-links to its transitively-mutable
    ///   read captures: the variables stay readable, mutating one
    ///   poisons the closure, and moving the closure follows the
    ///   ordinary derived-value rules [fate-link] [fate-move-mode];
    /// - immutable captures are free (clone-vs-alias is unobservable —
    ///   backend-parity principle);
    /// - the capture list is exported for tooling and emitters
    ///   (`Checked::lambda_captures`).
    fn finish_lambda_captures(&mut self, ctx: LambdaCtx, span: Span) {
        let mut links: Vec<FateLink> = Vec::new();
        let mut exported: Vec<LambdaCapture> = Vec::new();
        for cap in &ctx.captures {
            exported.push(LambdaCapture {
                name: cap.name.clone(),
                consumed: cap.mutated || cap.moved,
                mutable: cap.mutable,
                mutated: cap.mutated,
            });
            if cap.mutated {
                // [linear-lambda] A mutated capture would move the
                // obligation into the closure: forbidden.
                let is_linear = self
                    .var_by_id(cap.var_id)
                    .is_some_and(|v| self.ty_own_linear(&v.declared));
                if is_linear {
                    if self.inferred.is_some() {
                        self.error(
                            span,
                            format!(
                                "this lambda captures and mutates `{}`, which \
                                 holds a linear value: the closure would swallow \
                                 its obligation; restructure so the value is \
                                 passed explicitly",
                                cap.name
                            ),
                        );
                    }
                    continue;
                }
                let is_kept_param = self.var_by_id(cap.var_id).is_some_and(|v| v.is_param)
                    && !self.param_owned(&cap.name);
                if is_kept_param && self.own_written.contains(&cap.name) {
                    if self.inferred.is_some() {
                        self.error(
                            span,
                            format!(
                                "this lambda captures and mutates `{}`, which is a kept \
                                 parameter (the caller keeps it); capture \
                                 `copy({})` instead",
                                cap.name, cap.name
                            ),
                        );
                    }
                    continue;
                }
                if self.var_by_id(cap.var_id).is_some_and(|v| v.is_param) {
                    if let Some(key) = self.own_fn {
                        if !self.own_written.contains(&cap.name) {
                            self.param_claims
                                .entry(key)
                                .or_default()
                                .insert(cap.name.clone());
                        }
                    }
                }
                // Consume the original: the closure owns the value now.
                self.poison_derived(cap.var_id, &cap.name, FateEvent::Moved, span, Some(&[]));
                if let Some(var) = self.var_by_id_mut(cap.var_id) {
                    var.narrowed = Ty::Nothing;
                    var.poison = None;
                    var.consumed_by = Some(
                        "a lambda that captures and mutates it (bind a `copy` first \
                         to keep the original usable)",
                    );
                }
                continue;
            }
            // [lambda-view] A read capture of non-Copy data makes the
            // closure a **view**: it holds a borrow of the captured
            // variable, exactly as a struct holds its `proj` fields
            // [proj-field] — the body can return projections rooted in a
            // capture (`i -> get(words, i)!`), which no fn type can name,
            // so the value itself carries the link. Binding the lambda
            // links it (held, borrowed); a call result linked to the
            // lambda reaches the captured roots transitively; moving or
            // mutating the capture poisons the closure. Two exclusions: a
            // *consumed* capture is owned — the closure swallowed the
            // value at creation (which is what made it `once`
            // [once-fn]), so there is no source left to borrow from — and
            // a Copy scalar capture is the value itself on both backends
            // [copy-scalar-free].
            if cap.moved {
                continue;
            }
            let copy_scalar = self
                .var_by_id(cap.var_id)
                .is_some_and(|v| crate::types::is_copy_scalar(&v.declared));
            if copy_scalar {
                continue;
            }
            if !links.iter().any(|l| l.root_id == cap.var_id) {
                links.push(FateLink {
                    held: true,
                    root_id: cap.var_id,
                    root_name: cap.name.clone(),
                    bind_span: span,
                    // [fate-lambda] A capture is read through whatever
                    // the body does with it; the capture analysis
                    // records the variable, not a projection, so the
                    // link stays whole-variable.
                    path: Some(Vec::new()),
                    borrowed: true,
                });
            }
            if let Some(var) = self.var_by_id(cap.var_id) {
                for l in var.links.clone() {
                    if !links.iter().any(|e| e.root_id == l.root_id) {
                        links.push(FateLink {
                            held: true,
                            borrowed: true,
                            ..l
                        });
                    }
                }
            }
        }
        if !links.is_empty() {
            self.lambda_links.insert(span, links);
        }
        if !exported.is_empty() {
            self.out.lambda_captures.insert(self.key(span), exported);
        }
    }

    fn check_struct_lit(
        &mut self,
        ty: Option<&ast::Type>,
        fields: &'p [StructLitField],
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        let annotated = ty.map(|t| self.lower_type(t)).or_else(|| expected.cloned());
        let Some(struct_ty) = annotated else {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => {
                        self.check_expr(value, None);
                    }
                    StructLitFieldKind::Spread(e) => {
                        self.check_expr(e, None);
                    }
                }
            }
            return Ty::Unknown;
        };
        let (decl, subst) = match struct_ty.strip_quals() {
            Ty::Named { name, args } => match self.scope.structs.get(name.as_str()).copied() {
                Some(decl) => {
                    let mut subst = HashMap::new();
                    for (i, g) in decl.generics.iter().enumerate() {
                        subst.insert(g.name.clone(), args.get(i).cloned().unwrap_or(Ty::Unknown));
                    }
                    (Some(decl), subst)
                }
                None => (None, HashMap::new()),
            },
            _ => (None, HashMap::new()),
        };
        let Some(decl) = decl else {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => {
                        self.check_expr(value, None);
                    }
                    StructLitFieldKind::Spread(e) => {
                        self.check_expr(e, None);
                    }
                }
            }
            return struct_ty;
        };
        // [linear-composite] A generic field type hides the store from the
        // struct's own declaration (`struct Box<T> { item: T }`), so the
        // literal is where it surfaces. A field whose *declared* type is
        // linear was already refused at the struct, so it is not reported
        // twice.
        let mut has_spread = false;
        let mut provided: HashSet<&str> = HashSet::new();
        for f in fields {
            match &f.kind {
                StructLitFieldKind::Named { name, value } => {
                    match decl.fields.iter().find(|df| df.name.name == name.name) {
                        Some(df) => {
                            let fty = self.lower_type_subst(&df.ty, &subst, 0);
                            self.check_expr(value, Some(&fty));
                            let vty = self.out.expr_ty[&self.key(value.span())].clone();
                            // [linear-generics] A field whose declared type
                            // mentions an opted-in parameter (`canbe
                            // linear`) accepts the store: the container is
                            // conditionally linear and owes as a whole
                            // (user decision 2026-09-12).
                            let opted_field = decl.generic_canbe.iter().any(|(id, q)| {
                                q.name.name == "linear"
                                    && type_mentions_generic(&df.ty, &id.name)
                            });
                            if self.ty_own_linear(&vty) && !self.ty_own_linear(&fty) && !opted_field
                            {
                                let linear = format!("{vty}");
                                self.refuse_linear_composite(
                                    value.span(),
                                    &linear,
                                    format!("stored in field `{}.{}`", decl.name.name, name.name),
                                );
                            }
                            if !is_subtype(&vty, &fty) {
                                self.error(
                                    value.span(),
                                    format!("field `{}` expects `{fty}`, found `{vty}`", name.name),
                                );
                            }
                            provided.insert(&name.name);
                        }
                        None => {
                            self.check_expr(value, None);
                            self.error(
                                name.span,
                                format!("struct `{}` has no field `{}`", decl.name.name, name.name),
                            );
                        }
                    }
                }
                StructLitFieldKind::Spread(e) => {
                    has_spread = true;
                    self.check_expr(e, None);
                }
            }
        }
        if !has_spread {
            for df in &decl.fields {
                if df.default.is_none() && !provided.contains(df.name.name.as_str()) {
                    self.error(
                        span,
                        format!(
                            "missing field `{}` in `{}` literal",
                            df.name.name, decl.name.name
                        ),
                    );
                }
            }
        }
        struct_ty
    }
}

impl<'p, 'r> Checker<'p, 'r> {
    // ================= conditions & narrowing =================

    /// Checks a condition expression and derives narrowing facts.
    /// [is-bind-once] Where a binding `is` may take a subject that is **not a
    /// place** (a call, above all): as the *whole* condition of a `while`, or
    /// of an `if`'s first branch. Those two are the shapes the emitters hoist
    /// into a single evaluation — one temporary, read by both the test and the
    /// binding.
    ///
    /// Everywhere else it is refused rather than emitted, because the honest
    /// alternatives are both bad: evaluating the subject twice is what this
    /// rule exists to stop (it silently dropped values — the defect closed
    /// 2026-09-16), and hoisting it out of a `&&` chain or an `else if` would
    /// evaluate it when short-circuiting says it should not run at all.
    fn reject_unhoistable_is(&mut self, cond: &'p Expr, hoistable: bool, what: &str) {
        // The whole condition being the hoistable shape is the legal case.
        if hoistable {
            if let Expr::Is {
                subject,
                binding: Some(_),
                ..
            } = cond
            {
                if !is_place_expr(subject) {
                    return;
                }
            }
        }
        let mut offenders: Vec<Span> = Vec::new();
        collect_binding_is(cond, &mut |subject, span| {
            if !is_place_expr(subject) {
                offenders.push(span);
            }
        });
        for span in offenders {
            self.error(
                span,
                format!(
                    "this `is` binds its subject, and the subject is not a variable or \
                     a field chain — so it has to be evaluated exactly once, which is \
                     only possible when the `is` is the whole condition of a `while` or \
                     of an `if`'s first branch (here it is {what}). Bind the subject \
                     first: `let x = …` and then test `x`"
                ),
            );
        }
    }

    fn analyze_cond(&mut self, cond: &'p Expr) -> CondInfo {
        match cond {
            // [qual-widen] The dual of `is`: same test, generalized type.
            Expr::Widen {
                subject,
                quals,
                span,
            } => {
                let info = self.widen_info(subject, quals, *span);
                self.out.expr_ty.insert(self.key(*span), Ty::named("Bool"));
                let mut out = CondInfo::default();
                if let Some(place) = &info.subject_place {
                    out.then_narrows.push(Narrow {
                        place: place.clone(),
                        narrowed: info.matched.clone(),
                        declared: info.subject_repr.clone(),
                    });
                    if let Some(rem) = &info.remaining {
                        out.else_narrows.push(Narrow {
                            place: place.clone(),
                            narrowed: rem.clone(),
                            declared: info.subject_repr.clone(),
                        });
                    }
                }
                out
            }
            Expr::Is { .. } => {
                let info = self.is_info(cond);
                self.out
                    .expr_ty
                    .insert(self.key(cond.span()), Ty::named("Bool"));
                let mut out = CondInfo::default();
                if let Some(place) = &info.subject_place {
                    out.then_narrows.push(Narrow {
                        place: place.clone(),
                        narrowed: info.matched.clone(),
                        declared: info.subject_repr.clone(),
                    });
                    if let Some(rem) = &info.remaining {
                        out.else_narrows.push(Narrow {
                            place: place.clone(),
                            narrowed: rem.clone(),
                            declared: info.subject_repr.clone(),
                        });
                    }
                }
                if let Some(binding) = info.binding {
                    out.bindings.push(binding);
                }
                out
            }
            Expr::Binary {
                op: BinaryOp::And,
                lhs,
                rhs,
                ..
            } => {
                let l = self.analyze_cond(lhs);
                let r = self.with_narrows(&l.then_narrows.clone(), |c| {
                    c.locals.push(HashMap::new());
                    for (ident, ty, links, _) in &l.bindings {
                        c.declare_with_links(ident, ty.clone(), links.clone());
                    }
                    let r = c.analyze_cond(rhs);
                    c.locals.pop();
                    r
                });
                let mut out = CondInfo::default();
                out.then_narrows.extend(l.then_narrows);
                out.then_narrows.extend(r.then_narrows);
                out.bindings.extend(l.bindings);
                out.bindings.extend(r.bindings);
                out
            }
            Expr::Binary {
                op: BinaryOp::Or,
                lhs,
                rhs,
                ..
            } => {
                let l = self.analyze_cond(lhs);
                let r = self.with_narrows(&l.else_narrows.clone(), |c| c.analyze_cond(rhs));
                let mut out = CondInfo::default();
                out.else_narrows.extend(l.else_narrows);
                out.else_narrows.extend(r.else_narrows);
                out
            }
            Expr::Unary {
                op: UnaryOp::Not,
                operand,
                ..
            } => {
                let inner = self.analyze_cond(operand);
                CondInfo {
                    then_narrows: inner.else_narrows,
                    else_narrows: inner.then_narrows,
                    bindings: Vec::new(),
                }
            }
            other => {
                let ty = self.check_expr(other, None);
                self.require_bool(&ty, other.span());
                CondInfo::default()
            }
        }
    }

    /// [cond-bool] A condition is a boolean expression. `if`, `elif`,
    /// `while` and a subject-less `when`'s branch heads take `Bool` and
    /// nothing else — Salvo has no truthiness, so there is no rule that
    /// could turn another type into a decision. Checked per *leaf* of a
    /// `&&`/`||`/`!` condition, which is where the wrong type was written.
    /// A type the checker could not infer stays lenient
    /// [type-unknown-lenient], and a diverging expression never produces a
    /// value to test [type-any-nothing].
    fn require_bool(&mut self, ty: &Ty, span: Span) {
        if ty.is_unknown() || matches!(ty, Ty::Nothing) || ty.is_bool() {
            return;
        }
        self.error(
            span,
            format!(
                "a condition must be a `Bool` (found `{ty}`): Salvo has no \
                 truthiness — compare explicitly (`n != 0`, `list.size() > 0`) or \
                 test the type with `is`"
            ),
        );
    }

    /// Analyzes an `is` expression: checks the subject, records the runtime
    /// lowering (`is_tests`), and computes matched/remaining types.
    fn is_info(&mut self, is_expr: &'p Expr) -> IsInfo {
        let Expr::Is {
            subject,
            check,
            binding,
            span,
        } = is_expr
        else {
            unreachable!("is_info called on non-is expression")
        };
        let subj_ty = self.check_expr(subject, None);
        let repr = self.repr_of(subject, &subj_ty);
        let pat = self.parse_check(check);

        // A qualifier check on a non-union subject is a predicate test
        // [is-qualifies]: it calls each qualifier's `qualifies` function at
        // runtime.
        // [qual-overload] Resolve each named qualifier against *this* subject
        // once, up front: with several declarations of one name it is the
        // subject that says which is being tested, and every question below
        // (does it have a body, does it apply, what effects does its
        // `qualifies` declare) is about that one.
        let resolved: Vec<Option<&'p QualifierDecl>> = pat
            .quals
            .iter()
            .map(|q| {
                let subj = subj_ty.clone();
                self.qualifier_for(q.as_str(), Some(&subj))
            })
            .collect();
        let is_predicate = !pat.is_none
            && !pat.quals.is_empty()
            && !matches!(subj_ty, Ty::Union(_))
            && !subj_ty.is_unknown()
            && resolved.iter().all(|d| d.is_some_and(|d| d.has_body));
        if is_predicate {
            for (q, decl) in pat.quals.iter().zip(&resolved) {
                let Some(decl) = *decl else { continue };
                if !self.qual_applies(decl, subj_ty.strip_quals()) {
                    self.error(
                        *span,
                        format!("qualifier `{q}` does not apply to `{subj_ty}`"),
                    );
                }
            }
            if let Some(base) = &pat.base {
                if !is_subtype(subj_ty.strip_quals(), base) {
                    self.error(*span, "this check can never succeed".to_string());
                }
            }
            self.out
                .predicate_tests
                .insert(self.key(*span), pat.quals.clone());
            // The `qualifies` call happens here at runtime: its declared
            // effects must be available in this scope
            // [is-qualifies-effects].
            for (q, decl) in pat.quals.iter().zip(&resolved) {
                let Some(decl) = *decl else { continue };
                let Some(qf) = decl.fns.iter().find(|f| f.name.name == "qualifies") else {
                    continue;
                };
                for eff in qf.effects.iter().flatten() {
                    let EffectRef::Effect(r) = eff else { continue };
                    let ename = r.name.name.as_str();
                    let available = self
                        .effect_env
                        .iter()
                        .any(|c| matches!(c, Ty::Named { name, .. } if name == ename));
                    if !available {
                        self.error(
                            *span,
                            format!(
                                "predicate qualifier `{q}` requires effect `{ename}`, \
                                 but no handler for it is in scope (declare it in the \
                                 function's effect list or `use` a handler)"
                            ),
                        );
                    }
                }
            }
        } else if let Some(test) = self.union_test_for(&repr, &pat) {
            // Runtime lowering against the declared union representation
            // [is-narrowing] [is-precise].
            self.out.is_tests.insert(self.key(*span), test);
        } else if !pat.quals.is_empty() && !matches!(subj_ty, Ty::Union(_)) && !pat.unresolved {
            // [qual-constructive] no runtime test exists for constructive
            // qualifiers on non-union values.
            let constructive = pat
                .quals
                .iter()
                .zip(&resolved)
                .find(|(_, d)| d.is_some_and(|d| !d.has_body))
                .map(|(q, _)| q);
            match constructive {
                Some(q) => self.error(
                    *span,
                    format!(
                        "`{q}` is a constructive qualifier; values only gain it \
                         from constructor functions, so it cannot be tested with `is` \
                         on a non-union value"
                    ),
                ),
                None => self.error(
                    *span,
                    "this qualifier check cannot be performed at runtime \
                     (unknown qualifier)",
                ),
            }
        }

        // Narrowing math on the logical (possibly already narrowed) type.
        let (matched, remaining) = match &subj_ty {
            // An unresolved check name was already reported: keep the
            // subject's type and narrow nothing, so nothing cascades
            // [name-resolve].
            _ if pat.unresolved => (subj_ty.clone(), None),
            Ty::Union(arms) => {
                let (m, r): (Vec<Ty>, Vec<Ty>) = arms
                    .iter()
                    .cloned()
                    .partition(|arm| self.arm_matches(arm, &pat));
                if m.is_empty() {
                    self.error(*span, "this check can never succeed".to_string());
                }
                (self.mk_union(m), Some(self.mk_union(r)))
            }
            other => {
                if is_predicate {
                    // A successful predicate check adds the qualifiers; a
                    // failed one proves nothing about the type.
                    let quals: Vec<Qual> = pat
                        .quals
                        .iter()
                        .map(|n| Qual {
                            effect: false,
                            name: n.clone(),
                            args: Vec::new(),
                        })
                        .collect();
                    (other.clone().qualify(quals), None)
                } else {
                    (other.clone(), None)
                }
            }
        };

        let subject_place = Place::of_expr(subject).filter(|p| {
            // Only field chains out of a tracked local narrow [flow-place]
            // (user decision P1a): an element step cannot be told from a
            // sibling, and a place whose root is not a local has no flow
            // state to hang the fact on.
            p.narrowable() && self.lookup(&p.root).is_some()
        });
        let binding = binding.as_ref().map(|b| {
            let bty = if matches!(matched, Ty::Union(_)) {
                self.error(
                    b.span,
                    "an `is` binding requires a check matching a single type",
                );
                matched.clone()
            } else {
                matched.clone()
            };
            self.out.expr_ty.insert(self.key(b.span), bty.clone());
            // [linear-container] A binding whose payload is **linear** takes
            // the obligation *out* of the subject: `remove_at(pending, i) is
            // Reply<Fired> token` is the take-by-move surface's own idiom.
            // Recorded so the emitters move rather than copy — Rust's
            // narrowed-read path clones through a borrow, which duplicates an
            // obligation and does not even compile for a reply token (a
            // `SalvoReply` is not `Clone`), and this is the *binding* site, so
            // `note_linear_move`'s use-site path never sees it.
            if self.ty_own_linear(&bty) {
                self.out.linear_moves.insert(self.key(b.span));
            }
            // The binding aliases the subject: they share fate
            // [fate-link].
            let links = self.links_for_value(subject, b.span);
            (b.clone(), bty, links, None)
        });

        IsInfo {
            subject_place,
            subject_repr: repr,
            matched,
            remaining,
            binding,
        }
    }

    /// Splits an `is` check into qualifier names and an optional base type.
    /// Names that resolve to nothing are reported here [name-resolve] and
    /// mark the pattern `unresolved`, which suppresses the match-arm
    /// diagnostics downstream (an unresolved name matches nothing, and
    /// "this check can never succeed" would only mislead).
    fn parse_check(&mut self, check: &[TypeRef]) -> CheckPat {
        if check.len() == 1 && check[0].name.name == "None" && check[0].args.is_empty() {
            return CheckPat {
                quals: Vec::new(),
                base: None,
                is_none: true,
                unresolved: false,
            };
        }
        let mut quals = Vec::new();
        let mut base = None;
        let mut unresolved = false;
        for (i, r) in check.iter().enumerate() {
            let last = i + 1 == check.len();
            let name = r.name.name.as_str();
            let is_qual = self.qual_name_exists(name) || self.generics.contains(name);
            let is_type = self.type_name_exists(name) || self.generics.contains(name);
            // Every position but the last is a qualifier; the last is a
            // qualifier when it names one, else the checked base type.
            if !last || self.qual_name_exists(name) {
                if !is_qual {
                    self.require_name(r, true);
                    unresolved = true;
                }
                // [lsp-definition] The qualifier name in an `is` check points
                // at its declaration, so hovering `Positive` in
                // `i is Positive` reaches the qualifier rather than falling
                // through to the enclosing expression's `Bool`.
                self.record_def_ref(r.name.span, name);
                quals.push(r.name.name.clone());
            } else if is_type {
                // The same for a *type* check (`x is Str`).
                self.record_def_ref(r.name.span, name);
                let empty = HashMap::new();
                base = Some(self.lower_base_ref(r, &empty, 0));
            } else {
                // Both namespaces are admissible here, so neither wording
                // fits on its own.
                let name = name.to_string();
                self.error_unresolved(r.span, format!("unknown type or qualifier `{name}`"), &name);
                unresolved = true;
            }
        }
        CheckPat {
            quals,
            base,
            is_none: false,
            unresolved,
        }
    }

    /// Whether a union arm matches an `is` check pattern.
    fn arm_matches(&self, arm: &Ty, pat: &CheckPat) -> bool {
        if pat.is_none {
            return arm.is_none_ty();
        }
        if arm.is_none_ty() {
            return false;
        }
        if let Some(base) = &pat.base {
            if !is_subtype(arm.strip_quals(), base) {
                return false;
            }
        }
        let arm_quals: Vec<&str> = arm.quals().iter().map(|q| q.name.as_str()).collect();
        pat.quals.iter().all(|q| arm_quals.contains(&q.as_str()))
    }

    /// The runtime lowering of a check against a union representation.
    fn union_test_for(&mut self, repr: &Ty, pat: &CheckPat) -> Option<UnionTest> {
        let Ty::Union(_) = repr else { return None };
        let value_arms = repr.value_arms();
        let size = value_arms.len();
        let nullable = repr.has_none_arm();
        if size >= 2 {
            self.out.union_sizes.insert(size);
        }
        if pat.is_none {
            return Some(UnionTest {
                size,
                arms: Vec::new(),
                nullable,
                match_none: true,
            });
        }
        let arms: Vec<usize> = value_arms
            .iter()
            .enumerate()
            .filter(|(_, arm)| self.arm_matches(arm, pat))
            .map(|(i, _)| i)
            .collect();
        Some(UnionTest {
            size,
            arms,
            nullable,
            match_none: false,
        })
    }

    // ================= if / when =================

    fn check_if(
        &mut self,
        branches: &'p [(Expr, Block)],
        else_block: Option<&'p Block>,
        span: Span,
    ) -> Ty {
        let mut acc_else: Vec<Narrow> = Vec::new();
        let mut branch_tys = Vec::new();
        let mut tails: Vec<Option<TailInfo>> = Vec::new();
        // Branch-aware consumption merging [deduce-consume]: each branch
        // body's narrowing effects (moves, qualifier removal) are isolated
        // and merged at the join — a branch that always exits never
        // contributes to the state after the `if`.
        let mut fallthrough: Vec<NarrowSnapshot> = Vec::new();
        for (i, (cond, block)) in branches.iter().enumerate() {
            // [is-bind-once] Only the first branch's condition can be hoisted
            // into a single evaluation; a later one would need the temporary
            // declared where Rust has no statement position for it.
            self.reject_unhoistable_is(
                cond,
                i == 0,
                if i == 0 { "inside a larger condition" } else { "an `elif` condition" },
            );
            let info = self.with_narrows(&acc_else.clone(), |c| c.analyze_cond(cond));
            let entry = self.snapshot_narrows();
            let mut narrows = acc_else.clone();
            narrows.extend(info.then_narrows.iter().cloned());
            let (ty, tail) = self.with_narrows(&narrows, |c| {
                c.check_branch_block(block, info.bindings.clone())
            });
            if !self.block_exits(block) {
                fallthrough.push(self.snapshot_narrows());
            }
            self.restore_narrows(&entry);
            branch_tys.push(ty);
            tails.push(tail);
            acc_else.extend(info.else_narrows);
        }
        match else_block {
            Some(block) => {
                let entry = self.snapshot_narrows();
                let (ty, tail) = self.with_narrows(&acc_else.clone(), |c| {
                    c.check_branch_block(block, Vec::new())
                });
                if !self.block_exits(block) {
                    fallthrough.push(self.snapshot_narrows());
                }
                self.restore_narrows(&entry);
                branch_tys.push(ty);
                tails.push(tail);
            }
            None => {
                // No `else`: the no-branch-taken path falls through with
                // the current state. [linear-union-arm] With one linear
                // exception: on this path every condition was false, so
                // the else-narrows hold — and an else-narrow that lands a
                // linear union on a non-linear arm (`if h is InputStream s
                // { close(s) }`: here `h` is `None`) discharges the
                // obligation on this path, exactly as the then-narrow
                // does inside the branch. Only the settling is applied;
                // the general narrowing stays out of the join as before.
                let mut snap = self.snapshot_narrows();
                for n in &acc_else {
                    if !n.place.is_root() || self.ty_own_linear(&n.narrowed) {
                        continue;
                    }
                    let owed = self
                        .lookup(&n.place.root)
                        .is_some_and(|v| self.ty_own_linear(&v.narrowed));
                    if !owed {
                        continue;
                    }
                    for frame in snap.iter_mut() {
                        if let Some(state) = frame.get_mut(&n.place.root) {
                            state.linear_settled = true;
                        }
                    }
                }
                fallthrough.push(snap);
                branch_tys.push(Ty::none());
                tails.push(None);
            }
        }
        self.merge_fallthrough(&fallthrough, span);
        // [is-narrow-guard] The guard idiom: every branch leaves the block
        // (`if e is None { return … }`), so whatever follows the `if` is on
        // the else-path and every condition's else-narrows hold there.
        // Installed *before* the assignment resets below, so a branch that
        // reassigned the subject still wins [narrow-assign-reset].
        if !branches.is_empty() && branches.iter().all(|(_, b)| self.block_exits(b)) {
            let facts = acc_else.clone();
            self.install_narrows(&facts);
        }
        let join = self.mk_union(branch_tys);
        for tail in tails.into_iter().flatten() {
            self.maybe_coerce(tail.span, &tail.logical, &tail.repr, &join);
        }
        for (_, block) in branches {
            // [is-narrow-guard] [narrow-assign-reset] A branch that exits
            // cannot be the path taken to the code below, so its
            // assignments do not reset anything there — the same reason
            // `merge_fallthrough` ignores it.
            if self.block_exits(block) {
                continue;
            }
            self.reset_assigned(block);
        }
        if let Some(block) = else_block {
            if !self.block_exits(block) {
                self.reset_assigned(block);
            }
        }
        join
    }

    /// Checks a `when` expression: union-typed variable subject
    /// [when-union-subject], sequential arm consumption and exhaustiveness
    /// [when-exhaustive], branch values unioning into the result
    /// [when-value].
    fn check_when(&mut self, subject: &'p Expr, branches: &'p [WhenBranch], span: Span) -> Ty {
        let subj_ty = self.check_expr(subject, None);
        let Expr::Ident(subject_id) = subject else {
            self.error(span, "`when` requires a plain variable as its subject");
            for b in branches {
                self.check_branch_block(&b.body, Vec::new());
            }
            return Ty::Unknown;
        };
        let repr = self.repr_of(subject, &subj_ty);
        let Ty::Union(all_arms) = &subj_ty else {
            // [when-union-subject] A *qualified* union (`Ok (A | B)`, or an
            // `Thrown (Str | Int)` message [try]) is a claim **about** a
            // union, not a union: its arms belong to the inner type. The
            // qualifier is droppable ([qual-erasure]'s `Qual T <: T`), so
            // the remedy is a binding at the inner type — which also keeps
            // the level change visible to the reader, since the same
            // qualifier name can appear at both levels.
            let message = match &subj_ty {
                Ty::Qualified { quals, base } if matches!(**base, Ty::Union(_)) => {
                    let qual = quals
                        .iter()
                        .map(|q| q.name.clone())
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!(
                        "`when` requires a union-typed subject, and `{subj_ty}` is the \
                         claim `{qual}` *about* a union: bind the inner union to a \
                         local and match that (`let inner: {base} = ...`)"
                    )
                }
                _ => format!("`when` requires a union-typed subject (found `{subj_ty}`)"),
            };
            self.error(span, message);
            for b in branches {
                self.check_branch_block(&b.body, Vec::new());
            }
            return Ty::Unknown;
        };

        let mut remaining: Vec<Ty> = all_arms.clone();
        let mut branch_tys = Vec::new();
        let mut tails: Vec<Option<TailInfo>> = Vec::new();
        let mut fallthrough: Vec<NarrowSnapshot> = Vec::new();
        // A branch whose check named something unresolved cannot be
        // matched against the arms; its diagnostics — and the
        // exhaustiveness verdict, which the unmatched arms would falsify
        // — are suppressed [name-resolve].
        let mut unresolved_check = false;
        for branch in branches {
            let pat = self.parse_check(&branch.check);
            let matched: Vec<Ty> = remaining
                .iter()
                .filter(|arm| self.arm_matches(arm, &pat))
                .cloned()
                .collect();
            if pat.unresolved {
                unresolved_check = true;
            } else if matched.is_empty() {
                self.error(
                    branch.span,
                    "this `when` branch matches no remaining union arm",
                );
            }
            if let Some(test) = self.union_test_for(&repr, &pat) {
                self.out.is_tests.insert(self.key(branch.span), test);
            }
            // [qual-widen] A `^` branch tests the same arm and then reads the
            // subject *without* the qualifiers: the whole point is the
            // nested case, where the arm's payload is itself a union
            // (`Ok (Ok Int | Err Str)`) that the branch can then `when` on.
            let narrow_ty = if branch.widen {
                let mut ok = !matched.is_empty();
                if pat.base.is_some() || pat.is_none {
                    self.error(
                        branch.span,
                        "a `^` branch removes qualifiers, so its head is \
                         qualifier names only (use `is` to match a type)"
                            .to_string(),
                    );
                    ok = false;
                }
                for q in &pat.quals {
                    if let Some(reason) = self.removal_block(q) {
                        self.error(
                            branch.span,
                            format!("`{q}` cannot be removed with `^`: {reason}"),
                        );
                        ok = false;
                    }
                }
                let stripped: Vec<Ty> = matched
                    .iter()
                    .filter_map(|arm| strip_quals_named(arm, &pat.quals))
                    .collect();
                if ok && stripped.len() == matched.len() {
                    self.mk_union(stripped)
                } else {
                    if ok {
                        self.error(
                            branch.span,
                            format!(
                                "this arm does not carry `{}`, so there is \
                                 nothing for `^` to remove",
                                pat.quals.join(" ")
                            ),
                        );
                    }
                    self.mk_union(matched.clone())
                }
            } else {
                self.mk_union(matched.clone())
            };
            let mut bindings = Vec::new();
            if let Some(b) = &branch.binding {
                if matches!(narrow_ty, Ty::Union(_)) {
                    self.error(
                        b.span,
                        "a `when` binding requires a check matching a single type",
                    );
                }
                self.out.expr_ty.insert(self.key(b.span), narrow_ty.clone());
                // The `when` binding aliases the subject [fate-link].
                let links = self.links_for_value(subject, b.span);
                bindings.push((b.clone(), narrow_ty.clone(), links, None));
            }
            // [qual-widen] A `^` branch sees the subject at the widened
            // type, so nested narrowing composes on that rather than on the
            // storage the arm test peeled it out of.
            let declared = if branch.widen && self.out.is_tests.contains_key(&self.key(branch.span))
            {
                self.out
                    .widen_targets
                    .insert(self.key(branch.span), narrow_ty.clone());
                narrow_ty.clone()
            } else {
                repr.clone()
            };
            let narrows = vec![Narrow {
                place: Place::root(subject_id.name.clone()),
                narrowed: narrow_ty,
                declared,
            }];
            // Isolate this branch's consumption effects; only
            // fall-through branches reach the code after the `when`
            // [deduce-consume].
            let entry = self.snapshot_narrows();
            let (ty, tail) =
                self.with_narrows(&narrows, |c| c.check_branch_block(&branch.body, bindings));
            if !self.block_exits(&branch.body) {
                fallthrough.push(self.snapshot_narrows());
            }
            self.restore_narrows(&entry);
            branch_tys.push(ty);
            tails.push(tail);
            remaining.retain(|arm| !matched.contains(arm));
        }
        self.merge_fallthrough(&fallthrough, span);
        if !remaining.is_empty() && !unresolved_check {
            let missing: Vec<String> = remaining.iter().map(|a| a.to_string()).collect();
            self.error(
                span,
                format!(
                    "non-exhaustive `when`: unhandled union arm{} {}",
                    if missing.len() == 1 { "" } else { "s" },
                    missing.join(", ")
                ),
            );
        }
        let join = self.mk_union(branch_tys);
        for tail in tails.into_iter().flatten() {
            self.maybe_coerce(tail.span, &tail.logical, &tail.repr, &join);
        }
        for branch in branches {
            self.reset_assigned(&branch.body);
        }
        join
    }

    // ================= coercions =================

    /// Joins a loop's value contributions [while-value]: the body tail,
    /// each `break value`, and the `else` tail — or `None` when there is
    /// no `else` (the loop may never run). A bare `break` or a `continue`
    /// also joins `None` (an iteration may end without producing a value).
    /// Tails and break values are coerced to the join like `if` branches.
    fn finish_loop_value(
        &mut self,
        body_ty: Ty,
        body_tail: Option<TailInfo>,
        ctx: LoopCtx,
        else_ty: Option<Ty>,
        else_tail: Option<TailInfo>,
    ) -> Ty {
        let mut arms = vec![body_ty];
        arms.extend(ctx.breaks.iter().map(|b| b.logical.clone()));
        match else_ty {
            Some(t) => arms.push(t),
            None => arms.push(Ty::none()),
        }
        if ctx.may_skip_value {
            arms.push(Ty::none());
        }
        let join = self.mk_union(arms);
        for tail in body_tail.into_iter().chain(ctx.breaks).chain(else_tail) {
            self.maybe_coerce(tail.span, &tail.logical, &tail.repr, &join);
        }
        join
    }

    /// [type-none-unit] Records that a `None` **literal** argument fills a
    /// slot whose type *is* `None`, so the emitters render the target's unit
    /// value (`()`, `Unit`) instead of an absent optional.
    ///
    /// `None` is one spelling for two things — the absent arm of a `T?` and
    /// the sole value of the `None` type — and the targets spell them
    /// differently, so the choice belongs wherever the *slot* is known. A
    /// generic argument the call inferred as `None` is the shape that needs
    /// it: `ok(None)` building an `Ok None | Err E`, `emitted(None)` for a
    /// sequence of optionals. Emitting the optional there is target code
    /// that does not compile, which is how the hole was found (phase 4's
    /// `close(s) -> Ok None | Err FsError`).
    fn note_none_unit(&mut self, arg: &Expr, slot: &Ty) {
        if slot.is_none_ty() && matches!(arg, Expr::Ident(id) if id.name == "None") {
            self.out
                .coerce
                .insert(self.key(arg.span()), Coercion::NoneUnit);
        }
    }

    /// Records the representation change (if any) needed to use a value of
    /// (`logical`, `repr`) where `expected` is required. Arm matching is
    /// positional over the declared type's non-`None` arms
    /// [union-arm-identity]; a qualified union group tries wrapping as
    /// a whole arm before stripping its qualifiers [qual-group].
    ///
    /// [str-drop-mut] A `Mut` qualifier dropped on the way is recorded too,
    /// wrapping whatever the representation math decided.
    ///
    /// [fn-effects] So is a producer *widening* — a pure producer used
    /// where a claiming one is expected — for the same reason: the two have
    /// different representations on both backends. Which positions need the
    /// adapter is therefore not a separate question: it is every position
    /// that funnels through here (call arguments, returns, `let`
    /// annotations, struct fields, union arms, branch joins).
    fn maybe_coerce(&mut self, span: Span, logical: &Ty, repr: &Ty, expected: &Ty) {
        self.coerce_repr(span, logical, repr, expected);
        if expected.is_unknown() || logical.is_unknown() {
            return;
        }
        // A `Mut` arm anywhere in the expected type means the value may
        // stay a builder: `Mut Str`, `Mut Str?`, and a generic position
        // (whose pattern is substituted to the argument's own type) all
        // keep it.
        if expected.arms().iter().any(|a| Self::carries_mut(a)) {
            return;
        }
        if matches!(expected.strip_quals(), Ty::Var(_) | Ty::Any | Ty::Nothing) {
            return;
        }
        let from = if Self::carries_mut(logical) {
            logical.clone()
        } else if Self::carries_mut(repr) {
            repr.clone()
        } else {
            return;
        };
        self.record_mut_drop(span, from);
    }

    /// Whether a type carries the `Mut` qualifier at its top level.
    fn carries_mut(ty: &Ty) -> bool {
        ty.quals().iter().any(|q| q.name == "Mut")
    }

    /// [str-drop-mut] Records a `Mut` drop at `span`, keeping any
    /// representation change already recorded there as the drop's
    /// continuation. Used by the positions that have no *expected type* to
    /// compare against — operator operands and string interpolation — as
    /// well as by `maybe_coerce`.
    fn record_mut_drop(&mut self, span: Span, from: Ty) {
        if !Self::carries_mut(&from) {
            return;
        }
        let key = self.key(span);
        let then = match self.out.coerce.remove(&key) {
            // Already recorded: keep it, but do not nest a drop in a drop.
            Some(Coercion::DropMut { from, then }) => {
                self.out
                    .coerce
                    .insert(key, Coercion::DropMut { from, then });
                return;
            }
            other => other.map(Box::new),
        };
        self.out
            .coerce
            .insert(key, Coercion::DropMut { from, then });
    }

    /// [str-drop-mut] The `Mut`-dropping conversion an *operand* needs: an
    /// operator compares or concatenates plain values, and on a backend
    /// where a builder is its own type the two are not comparable at all
    /// (Kotlin's `StringBuilder == String` is `false`, and `sb1 == sb2` is
    /// *reference* equality against Rust's structural `String == String`).
    /// Rejecting `Mut` operands the way [op-no-none] rejects optionals was
    /// declined: `Mut List` is accepted everywhere else, so a rejection
    /// here would surprise.
    fn drop_mut_operand(&mut self, expr: &'p Expr, ty: &Ty) {
        if Self::carries_mut(ty) {
            self.record_mut_drop(expr.span(), ty.clone());
        }
    }

    /// Records the representation change (if any) needed to use a value of
    /// (`logical`, `repr`) where `expected` is required.
    fn coerce_repr(&mut self, span: Span, logical: &Ty, repr: &Ty, expected: &Ty) {
        if expected.is_unknown() || logical.is_unknown() || matches!(logical, Ty::Nothing) {
            return;
        }
        // The emitter unwraps identifier uses narrowed to a single arm, so
        // the effective representation is the narrowed type in that case.
        // The same holds for a `T?` representation narrowed to its value
        // arm: backends that read optionals physically (Rust) unwrap at
        // the use site, and Kotlin smart-casts to the same effect.
        let effective = if repr.is_wrapper_union() && !matches!(logical, Ty::Union(_)) {
            logical.clone()
        } else if repr.has_none_arm()
            && matches!(repr, Ty::Union(_))
            && !logical.has_none_arm()
            && !logical.is_none_ty()
            && !matches!(logical, Ty::Union(_))
        {
            logical.clone()
        } else {
            repr.clone()
        };
        // A qualified union group (`Ok (A | B)`) wraps as a whole when it is
        // itself an arm of the expected union; otherwise it is physically
        // just the inner union and joins the representation math below.
        let effective = if let Ty::Qualified { base, .. } = &effective {
            if matches!(**base, Ty::Union(_)) {
                if expected.is_wrapper_union() {
                    let arms = expected.value_arms();
                    if let Some(i) = arms.iter().position(|arm| **arm == effective) {
                        self.out.union_sizes.insert(arms.len());
                        self.out.coerce.insert(
                            self.key(span),
                            Coercion::WrapUnion {
                                target: expected.clone(),
                                arm: i,
                                inner: None,
                            },
                        );
                        return;
                    }
                }
                (**base).clone()
            } else {
                effective
            }
        } else {
            effective
        };
        if expected.is_wrapper_union() {
            if effective == *expected {
                return;
            }
            if let Ty::Union(_) = effective {
                if effective.value_arms() == expected.value_arms()
                    && (!effective.has_none_arm() || expected.has_none_arm())
                {
                    return;
                }
                self.out
                    .union_sizes
                    .insert(effective.value_arms().len().max(2));
                self.out.union_sizes.insert(expected.value_arms().len());
                self.out.coerce.insert(
                    self.key(span),
                    Coercion::Rewrap {
                        from: effective,
                        to: expected.clone(),
                    },
                );
                return;
            }
            if effective.is_none_ty() {
                if !expected.has_none_arm() {
                    self.error(span, format!("`None` is not an arm of `{expected}`"));
                }
                return;
            }
            let arms = expected.value_arms();
            let matches: Vec<usize> = arms
                .iter()
                .enumerate()
                .filter(|(_, arm)| is_subtype(&effective, arm))
                .map(|(i, _)| i)
                .collect();
            match matches.len() {
                1 => {
                    self.out.union_sizes.insert(arms.len());
                    // [qual-group] A value whose qualifier list flattened
                    // (`emitted(ok("x"))` for an `Emitted (Ok Str | Err Str)`
                    // arm) is physically the bare inner value: wrap it into
                    // the inner union first.
                    let inner = crate::types::nested_group_arm(&effective, arms[matches[0]]).map(
                        |(inner_ty, inner_arm)| {
                            self.out.union_sizes.insert(inner_ty.value_arms().len());
                            Box::new(Coercion::WrapUnion {
                                target: inner_ty,
                                arm: inner_arm,
                                inner: None,
                            })
                        },
                    );
                    self.out.coerce.insert(
                        self.key(span),
                        Coercion::WrapUnion {
                            target: expected.clone(),
                            arm: matches[0],
                            inner,
                        },
                    );
                }
                0 => {
                    // [qual-group] One shape reads confusingly here: an
                    // expected arm `Q (A | B)` whose inner arms are
                    // themselves qualified, reached by a value carrying
                    // nothing *but* `Q`. Qualifier lists are flat and
                    // deduplicated, so a repeated qualifier vanishes
                    // (`ok(ok(x))` *is* `Ok Str`) and the value cannot say
                    // which reading it is — name the workaround, since the
                    // type in the message looks like it should fit.
                    let deduplicated = arms.iter().any(|arm| match arm {
                        Ty::Qualified { quals, base } => {
                            matches!(**base, Ty::Union(_))
                                && effective
                                    .quals()
                                    .iter()
                                    .all(|v| quals.iter().any(|q| q.name == v.name))
                                && base.value_arms().iter().any(|a| !a.quals().is_empty())
                        }
                        _ => false,
                    });
                    let mut msg =
                        format!("no arm of `{expected}` accepts a value of type `{logical}`");
                    if deduplicated {
                        msg.push_str(
                            "; a qualifier applied to a value that already carries it \
                             deduplicates, so bind the inner union to an annotated local \
                             first (`let inner: A | B = ...`)",
                        );
                    }
                    self.error(span, msg)
                }
                _ => self.error(
                    span,
                    format!(
                        "a value of type `{logical}` matches multiple arms of `{expected}`; \
                         qualify the value to disambiguate"
                    ),
                ),
            }
        }
        // Nullable-style or plain expected types need no union wrapping; a
        // wrapper-union value in a non-union slot is a plain type error
        // reported at the boundary. A bare value used where an optional
        // (`T?`-style) representation is expected records a WrapOption
        // [type-nullable]: physical-optional backends (Rust) wrap it in
        // `Some(...)`; Kotlin nullability ignores it.
        if !expected.is_wrapper_union()
            && matches!(expected, Ty::Union(_))
            && expected.has_none_arm()
            && !effective.is_unknown()
            && !effective.is_none_ty()
            && !effective.has_none_arm()
            && !matches!(effective, Ty::Union(_))
        {
            self.out.coerce.insert(
                self.key(span),
                Coercion::WrapOption {
                    target: expected.clone(),
                },
            );
        }
    }
}

/// [implicit-resolve] Whether a function *value* of type `candidate` fits a
/// position that wants `want`: **parameters contravariant, result
/// covariant**, plus the contract and effect variance [fn-contract]
/// [fn-effects].
///
/// `is_subtype` compares fn parameters *invariantly* — deliberately
/// conservative, since a backend renders a parameter's convention from its
/// declared type and two conventions are two different target types. Here
/// the rule the spec states applies, because the value is only ever *called*
/// by the callee that declared the position: std's
/// `iter(list: List<T>) -> Iter<T>` has to fill an `?Iterable<It, T>` whose
/// `It` turned out to be a `Mut List<Int>`, and reading a list that happens
/// to be mutable is exactly what it does.
fn fn_value_fits(candidate: &Ty, want: &Ty) -> bool {
    match (candidate.strip_quals(), want.strip_quals()) {
        (
            Ty::Fn {
                params: cp,
                ret: cr,
                contract: cc,
                effects: ce,
            },
            Ty::Fn {
                params: wp,
                ret: wr,
                contract: wc,
                effects: we,
            },
        ) => {
            cp.len() == wp.len()
                && cp.iter().zip(wp).all(|(c, w)| is_subtype(w, c))
                && is_subtype(cr, wr)
                && crate::types::contract_fits(cc.as_deref(), wc.as_deref(), cp.len())
                && ce.iter().all(|e| we.contains(e))
        }
        _ => is_subtype(candidate, want),
    }
}

// ================= unification =================

/// Structural unification of a (possibly generic) parameter type pattern
/// against an argument type, binding `Var`s in `subst`. Lenient: `Unknown`
/// matches anything; qualified arguments match unqualified parameters.
fn unify(param: &Ty, arg: &Ty, subst: &mut HashMap<String, Ty>) -> bool {
    if arg.is_unknown() || param.is_unknown() || *arg == Ty::Nothing {
        return true;
    }
    match (param, arg) {
        (Ty::Var(g), _) => {
            match subst.get(g) {
                Some(bound) => {
                    let bound = bound.clone();
                    if is_subtype(arg, &bound) {
                        true
                    } else if is_subtype(&bound, arg) {
                        // A later argument revealed the more general type:
                        // widen the binding (`pick(1, maybe_int)` binds
                        // `T = Int?`, not first-seen `Int`). Keeping the
                        // narrow binding would make the checker believe a
                        // possibly-`None` result is plain `T`.
                        subst.insert(g.clone(), arg.clone());
                        true
                    } else {
                        false
                    }
                }
                None => {
                    // No occurs check, deliberately: `Ty::Var` identity is
                    // name-scoped *per side* — the callee's `T` and a
                    // caller's `T` inside `arg` are different variables
                    // (`fn wrap<T>(x: List<T>)` forwarding to
                    // `inner<T>(x: T)` binds callee-`T` := `List<caller-T>`,
                    // which a name-based occurs check would wrongly
                    // reject). Argument types never contain the callee's
                    // own vars, and `substitute_vars` replaces without
                    // recursing into bindings, so expansion terminates.
                    subst.insert(g.clone(), arg.clone());
                    true
                }
            }
        }
        (
            Ty::Qualified {
                quals: pq,
                base: pb,
            },
            _,
        ) => {
            let arg_quals: Vec<&str> = arg.quals().iter().map(|q| q.name.as_str()).collect();
            // [once-fn] A `once` requirement is satisfied by any fn
            // (inverted subtyping: plain fns may be treated as
            // once-callable). [fn-effects] An effect claim is the same
            // direction: a producer that performs *fewer* effects fits a
            // position expecting more, so the claim need not be present on
            // the argument — while an effect the argument *does* claim must
            // be one the position expects, which `is_subtype` then checks.
            // [proj-type] `proj T` is satisfied by an owned value too
            // (`X <: proj X`), so it need not be on the argument either.
            // The residual the base unifies against keeps the argument's
            // never-drop qualifiers the pattern did not match (`Emitted T`
            // against `Emitted (proj Str)` binds `T = proj Str`), and
            // drops the droppable ones as before.
            let residual = {
                let names: HashSet<String> = pq.iter().map(|q| q.name.clone()).collect();
                arg.clone().remove_quals(&names)
            };
            pq.iter().all(|q| {
                q.name == "once"
                    || q.name == "proj"
                    || q.effect
                    || arg_quals.contains(&q.name.as_str())
            }) && arg
                .quals()
                .iter()
                .filter(|q| q.effect)
                .all(|q| pq.contains(q))
                && unify(pb, &residual, subst)
        }
        // A union parameter tries each arm against the *intact* argument —
        // this must precede the qualifier-stripping arm below, or a
        // qualified argument (`Ok Str`) could never match a union's
        // qualified arm (`Ok Str | Err Str`) [type-union].
        (Ty::Union(parms), _) => match arg {
            Ty::Union(aarms) => aarms.iter().all(|a| {
                parms
                    .iter()
                    .any(|p| unify(p, a, &mut subst.clone()) && unify(p, a, subst))
            }),
            _ => parms.iter().any(|p| unify(p, arg, subst)),
        },
        // `Qual T` can be passed where `T` is expected — except `once`,
        // which may never be dropped [once-fn]. (A top-level `proj` is
        // dropped *here*, for the binding: `size(list: List<T>)` given a
        // `proj List<Str>` binds `T = Str`; whether the position may take
        // the projection is the assignability check's business
        // [proj-type].)
        (_, Ty::Qualified { quals, base }) => {
            // [once-fn] D6 (user decision 2026-09-12): `once` drops on a
            // *data* type — a plain-typed holder consumes at most once
            // anyway — and never on a fn type, whose plain form is
            // callable repeatedly.
            let once_blocked =
                quals.iter().any(|q| q.name == "once") && matches!(**base, Ty::Fn { .. });
            !once_blocked && !quals.iter().any(|q| q.effect) && unify(param, base, subst)
        }
        (Ty::Named { name: pn, args: pa }, Ty::Named { name: an, args: aa }) => {
            pn == an && pa.len() == aa.len() && pa.iter().zip(aa).all(|(p, a)| unify(p, a, subst))
        }
        (Ty::Array(p), Ty::Array(a)) => unify(p, a, subst),
        (Ty::Tuple(ps), Ty::Tuple(as_)) => {
            ps.len() == as_.len() && ps.iter().zip(as_).all(|(p, a)| unify(p, a, subst))
        }
        (
            Ty::Fn {
                params: pp,
                ret: pr,
                contract: pc,
                ..
            },
            Ty::Fn {
                params: ap,
                ret: ar,
                contract: ac,
                ..
            },
        ) => {
            pp.len() == ap.len()
                && pp.iter().zip(ap).all(|(p, a)| unify(p, a, subst))
                && unify(pr, ar, subst)
                // [fn-contract] keeps <: consumes (inverted, like
                // [once-fn]); mutation permission must be granted.
                && crate::types::contract_fits(ac.as_deref(), pc.as_deref(), pp.len())
        }
        (Ty::Any, _) => true,
        _ => param == arg,
    }
}

/// Replaces the callee's generic parameters with their bindings (`Unknown`
/// when unbound). Foreign `Var`s (the caller's generics) are left alone.
/// Does `ty` mention the type variable `g` anywhere? [call-type-args]
fn ty_mentions_var(ty: &Ty, g: &str) -> bool {
    match ty {
        Ty::Var(v) => v == g,
        Ty::Named { args, .. } => args.iter().any(|a| ty_mentions_var(a, g)),
        Ty::Qualified { quals, base } => {
            ty_mentions_var(base, g)
                || quals
                    .iter()
                    .any(|q| q.args.iter().any(|a| ty_mentions_var(a, g)))
        }
        Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().any(|a| ty_mentions_var(a, g)),
        Ty::Array(elem) => ty_mentions_var(elem, g),
        Ty::Fn { params, ret, .. } => {
            params.iter().any(|p| ty_mentions_var(p, g)) || ty_mentions_var(ret, g)
        }
        _ => false,
    }
}

/// Does `ty` mention any of `vars`? [call-type-args]
fn ty_mentions_vars(ty: &Ty, vars: &HashSet<String>) -> bool {
    vars.iter().any(|v| ty_mentions_var(ty, v))
}

/// Does `ty` contain an un-inferred part? [type-unknown-lenient] An
/// argument like this fits *every* candidate, so overload ranking must not
/// turn one un-inferred value into a second diagnostic
/// [fn-overload-rank].
fn ty_mentions_unknown(ty: &Ty) -> bool {
    match ty {
        Ty::Unknown => true,
        Ty::Named { args, .. } => args.iter().any(ty_mentions_unknown),
        Ty::Qualified { quals, base } => {
            ty_mentions_unknown(base)
                || quals.iter().any(|q| q.args.iter().any(ty_mentions_unknown))
        }
        Ty::Union(arms) => arms.iter().any(ty_mentions_unknown),
        Ty::Tuple(elems) => elems.iter().any(ty_mentions_unknown),
        Ty::Array(elem) => ty_mentions_unknown(elem),
        Ty::Fn { params, ret, .. } => {
            params.iter().any(ty_mentions_unknown) || ty_mentions_unknown(ret)
        }
        _ => false,
    }
}

/// [implicit-infer] Like [`substitute_vars`], but a variable the
/// substitution does not mention stays a **variable** instead of becoming
/// `Unknown` — which is what lets a partially known pattern still bind
/// something (`(List<Int>) -> Iter<T>` teaches `T`; `(?) -> ?` teaches
/// nothing).
fn substitute_known(ty: &Ty, subst: &HashMap<String, Ty>, callee_generics: &HashSet<String>) -> Ty {
    match ty {
        Ty::Var(g) if callee_generics.contains(g) => {
            subst.get(g).cloned().unwrap_or_else(|| ty.clone())
        }
        Ty::Named { name, args } => Ty::Named {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| substitute_known(a, subst, callee_generics))
                .collect(),
        },
        Ty::Qualified { quals, base } => {
            let new_base = substitute_known(base, subst, callee_generics);
            new_base.qualify(
                quals
                    .iter()
                    .map(|q| Qual {
                        effect: q.effect,
                        name: q.name.clone(),
                        args: q
                            .args
                            .iter()
                            .map(|a| substitute_known(a, subst, callee_generics))
                            .collect(),
                    })
                    .collect(),
            )
        }
        Ty::Union(arms) => Ty::union_of(
            arms.iter()
                .map(|a| substitute_known(a, subst, callee_generics))
                .collect(),
        ),
        Ty::Tuple(elems) => Ty::Tuple(
            elems
                .iter()
                .map(|e| substitute_known(e, subst, callee_generics))
                .collect(),
        ),
        Ty::Array(elem) => Ty::Array(Box::new(substitute_known(elem, subst, callee_generics))),
        Ty::Fn {
            params,
            ret,
            contract,
            effects,
        } => Ty::Fn {
            params: params
                .iter()
                .map(|p| substitute_known(p, subst, callee_generics))
                .collect(),
            ret: Box::new(substitute_known(ret, subst, callee_generics)),
            contract: contract.clone(),
            effects: effects.clone(),
        },
        other => other.clone(),
    }
}

/// [iter-protocol] The element type argument of a `: Yield<self, T>` clause:
/// the second one, since the state comes first [group-obligation].
fn yield_elem_arg(ob: &TypeRef) -> Option<&ast::Type> {
    ob.args.get(1)
}

/// [group-self] Whether an obligation's type argument is the `self`
/// shorthand — "the type this declaration is". Written at the *obligation*
/// rather than inside the group (user decision 2026-09-08), which is what
/// keeps a group ordinary: its members mention only their own parameters, so
/// one group serves both `: Group<self, …>` and `?Group<…>`.
///
/// Bare and unqualified: `Mut self` or `self<T>` is not the shorthand, and
/// falls through to the ordinary unknown-type error.
fn is_self_ref(ty: &ast::Type) -> bool {
    matches!(
        ty,
        ast::Type::Named { qualifiers, base }
            if qualifiers.is_empty() && base.args.is_empty() && base.name.name == "self"
    )
}

/// [group-obligation] Structural equality of two types up to a *bijective*
/// renaming of type variables — how an obligation's expected member
/// signature (whose variables are the struct's generics) is matched against
/// a candidate overload (whose variables are its own generics). Bijective,
/// so `(A, A)` does not match `(A, B)` in either direction.
fn tys_match_renamed(
    a: &Ty,
    b: &Ty,
    fwd: &mut HashMap<String, String>,
    rev: &mut HashMap<String, String>,
) -> bool {
    let all = |xs: &[Ty],
               ys: &[Ty],
               fwd: &mut HashMap<String, String>,
               rev: &mut HashMap<String, String>| {
        xs.len() == ys.len()
            && xs
                .iter()
                .zip(ys)
                .all(|(x, y)| tys_match_renamed(x, y, fwd, rev))
    };
    match (a, b) {
        (Ty::Var(x), Ty::Var(y)) => {
            let f = fwd.entry(x.clone()).or_insert_with(|| y.clone());
            let r = rev.entry(y.clone()).or_insert_with(|| x.clone());
            f == y && r == x
        }
        (Ty::Named { name: na, args: aa }, Ty::Named { name: nb, args: ab }) => {
            na == nb && all(aa, ab, fwd, rev)
        }
        (
            Ty::Qualified {
                quals: qa,
                base: ba,
            },
            Ty::Qualified {
                quals: qb,
                base: bb,
            },
        ) => {
            qa.len() == qb.len()
                && qa.iter().zip(qb).all(|(x, y)| {
                    x.name == y.name && x.effect == y.effect && all(&x.args, &y.args, fwd, rev)
                })
                && tys_match_renamed(ba, bb, fwd, rev)
        }
        (Ty::Union(aa), Ty::Union(ab)) => all(aa, ab, fwd, rev),
        (Ty::Tuple(aa), Ty::Tuple(ab)) => all(aa, ab, fwd, rev),
        (Ty::Array(ea), Ty::Array(eb)) => tys_match_renamed(ea, eb, fwd, rev),
        (
            Ty::Fn {
                params: pa,
                ret: ra,
                ..
            },
            Ty::Fn {
                params: pb,
                ret: rb,
                ..
            },
        ) => all(pa, pb, fwd, rev) && tys_match_renamed(ra, rb, fwd, rev),
        (Ty::Any, Ty::Any) | (Ty::Nothing, Ty::Nothing) | (Ty::Unknown, Ty::Unknown) => true,
        _ => false,
    }
}

fn substitute_vars(ty: &Ty, subst: &HashMap<String, Ty>, callee_generics: &HashSet<String>) -> Ty {
    match ty {
        Ty::Var(g) if callee_generics.contains(g) => subst.get(g).cloned().unwrap_or(Ty::Unknown),
        Ty::Named { name, args } => Ty::Named {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| substitute_vars(a, subst, callee_generics))
                .collect(),
        },
        Ty::Qualified { quals, base } => {
            // Re-normalize through `qualify`: the substituted base may
            // itself be qualified (e.g. `Ok T` with `T = Ok Str`).
            let new_base = substitute_vars(base, subst, callee_generics);
            new_base.qualify(
                quals
                    .iter()
                    .map(|q| Qual {
                        effect: q.effect,
                        name: q.name.clone(),
                        args: q
                            .args
                            .iter()
                            .map(|a| substitute_vars(a, subst, callee_generics))
                            .collect(),
                    })
                    .collect(),
            )
        }
        Ty::Union(arms) => Ty::union_of(
            arms.iter()
                .map(|a| substitute_vars(a, subst, callee_generics))
                .collect(),
        ),
        Ty::Tuple(elems) => Ty::Tuple(
            elems
                .iter()
                .map(|e| substitute_vars(e, subst, callee_generics))
                .collect(),
        ),
        Ty::Array(elem) => Ty::Array(Box::new(substitute_vars(elem, subst, callee_generics))),
        Ty::Fn {
            params,
            ret,
            contract,
            effects,
        } => Ty::Fn {
            params: params
                .iter()
                .map(|p| substitute_vars(p, subst, callee_generics))
                .collect(),
            contract: contract.clone(),
            effects: effects.clone(),
            ret: Box::new(substitute_vars(ret, subst, callee_generics)),
        },
        other => other.clone(),
    }
}

impl<'p, 'r> Checker<'p, 'r> {
    // ================= calls =================

    fn check_call(
        &mut self,
        callee: &'p Expr,
        type_args: &'p [ast::Type],
        args: &'p [Expr],
        named: &'p [ast::NamedArg],
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        // Dot-notation [fn-dot]: `base.f(args)` == `f(base, args)`. `f`
        // must be a declared function or effect member — reaching
        // a target-language method means declaring it (as a member of a
        // `platform effect`), so an unknown name here is an error, not
        // interop pass-through [call-resolve].
        // [actor-self-send] `k@self(args)` — a message to the actor the
        // enclosing member belongs to. A selector, so it arrives as its own
        // callee shape rather than as a receiver that has to be told apart
        // from a value.
        if let Expr::SelfScoped { name, .. } = callee {
            return self.check_self_send(name, args, span);
        }
        if let Expr::Field { base, field, .. } = callee {
            let name = field.name.as_str();
            // [actor-use-addr] `p.total(out)` where `p` is an `Addr<E>`: the
            // inline form of `use p` — the member is E's, and the receiver is
            // the actor it goes to rather than a first argument. Checked
            // before the ordinary dot rules, because those would make the addr
            // argument zero of a member that never declared it.
            if let Some(effect) = self.addr_receiver(base) {
                return self
                    .check_addr_call(&effect, base, field, type_args, args, named, expected, span);
            }
            let known = self.scope.effect_members.contains_key(name) || self.has_callable(name);
            if known {
                let mut all_args: Vec<&'p Expr> = Vec::with_capacity(args.len() + 1);
                all_args.push(base);
                all_args.extend(args.iter());
                return self.resolve_named_call(
                    name, field.span, type_args, &all_args, named, expected, None, span,
                );
            }
            self.check_expr(base, None);
            for a in args {
                self.check_expr(a, None);
            }
            self.error_unresolved(
                field.span,
                format!(
                    "no function named `{name}` is in scope: dot-notation calls a \
                     function with the receiver as its first argument \
                     ([fn-dot]), so a target-language method is reached by \
                     declaring it as a member of a `platform effect`"
                ),
                name,
            );
            return Ty::Unknown;
        }

        // [effect-at] `close@Fs(s)` / `s.close@Fs()`: the *effect* whose
        // member is meant, written where two effects declare the same
        // member name [effect-member-overload]. It goes straight to that
        // effect's member — same-named fn overloads and locals do not
        // compete — and the call's type arguments keep their
        // [effect-disambiguation] meaning (they pin the instance:
        // `next_random@Random<Int>()`).
        if let Expr::EffectScoped {
            base, name, effect, ..
        } = callee
        {
            let mut all_args: Vec<&'p Expr> = Vec::with_capacity(args.len() + 1);
            if let Some(base) = base {
                all_args.push(base);
            }
            all_args.extend(args.iter());
            let Some(decl) = self.scope.effects.get(effect.name.as_str()).copied() else {
                for a in &all_args {
                    self.check_expr(a, None);
                }
                self.error_unresolved(
                    effect.span,
                    format!("no effect named `{}` is in scope", effect.name),
                    &effect.name,
                );
                return Ty::Unknown;
            };
            self.record_def_ref(effect.span, &effect.name);
            // [effect-member-overload] Every member of that name: the
            // selector picks the *effect*, the argument types still pick the
            // overload.
            let members = crate::effect_members_named(decl, &name.name);
            if members.is_empty() {
                for a in &all_args {
                    self.check_expr(a, None);
                }
                self.error(
                    name.span,
                    format!(
                        "effect `{}` has no member named `{}`",
                        effect.name, name.name
                    ),
                );
                return Ty::Unknown;
            }
            self.record_def_ref(name.span, &name.name);
            return self.check_effect_call(
                decl, &members, type_args, &all_args, named, expected, None, span,
            );
        }

        // [fn-overload-at] `f@core.list(x)` / `xs.f@core.list(y)`: the module
        // whose overload is meant. Written *instead of* letting scope
        // precedence decide, so it goes straight to overload selection — a
        // local of the same name does not shadow it, which is what makes `@`
        // the way out of a shadowed name.
        if let Expr::Scoped {
            base, name, module, ..
        } = callee
        {
            let mut all_args: Vec<&'p Expr> = Vec::with_capacity(args.len() + 1);
            if let Some(base) = base {
                all_args.push(base);
            }
            all_args.extend(args.iter());
            if !self.has_callable(&name.name) {
                for a in &all_args {
                    self.check_expr(a, None);
                }
                self.error_unresolved(
                    name.span,
                    format!("no function named `{}` is in scope", name.name),
                    &name.name,
                );
                return Ty::Unknown;
            }
            return self.resolve_named_call(
                &name.name,
                name.span,
                type_args,
                &all_args,
                named,
                expected,
                Some(module),
                span,
            );
        }

        if let Expr::Ident(id) = callee {
            // A local holding a callable (lambda parameter etc.).
            if let Some(var) = self.lookup(&id.name) {
                let vty = var.narrowed.clone();
                // A consumed callable (e.g. a `once` fn already called
                // [once-fn]) reports the standard consumed-use error.
                if matches!(vty, Ty::Nothing) {
                    self.check_expr(callee, None);
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Ty::Unknown;
                }
                let once = vty.quals().iter().any(|q| q.name == "once");
                if let Ty::Fn {
                    params,
                    ret,
                    contract,
                    effects,
                } = vty.strip_quals().clone()
                {
                    // [call-resolve] The callee is a *local* holding a
                    // function, which outranks any same-named declaration —
                    // an effect member included. Recorded for the emitters,
                    // whose own effect-member test is a program-wide name
                    // map that cannot see locals.
                    self.out.local_calls.insert(self.key(span));
                    // [fn-effects] The call supplies the value's effects.
                    self.check_fn_value_effects(&effects, span);
                    for (i, a) in args.iter().enumerate() {
                        self.check_expr(a, params.get(i));
                    }
                    // [fn-contract] Apply the fn value's contract to the
                    // arguments (default: keeps everything).
                    let arg_refs: Vec<&'p Expr> = args.iter().collect();
                    self.apply_fn_value_contract(&arg_refs, &params, contract.as_deref(), span);
                    // [proj-infer] A fn value has no body to read: its result
                    // holds a borrow of the arguments its type declares
                    // (`[p: proj]`), or conservatively of every kept one.
                    if self.ty_holds_proj(&ret) {
                        let declared: Vec<usize> = contract
                            .as_deref()
                            .map(|c| {
                                c.iter()
                                    .enumerate()
                                    .filter(|(_, e)| e.lent)
                                    .map(|(i, _)| i)
                                    .collect()
                            })
                            .unwrap_or_default();
                        let lent: Vec<usize> = if !declared.is_empty() {
                            declared
                        } else {
                            (0..params.len())
                                .filter(|&i| {
                                    contract
                                        .as_deref()
                                        .and_then(|c| c.get(i))
                                        .is_none_or(|e| e.kept)
                                })
                                .collect()
                        };
                        if !lent.is_empty() {
                            self.out.lending_calls.insert(self.key(span), lent);
                        }
                    }
                    // [once-fn] Calling a `once` fn consumes it: the
                    // existing consumption machinery then enforces the
                    // multiplicity (second call, loop back edge, branch
                    // merges) for free.
                    if once {
                        let state = self.lookup(&id.name).map(|var| (var.id, var.links.clone()));
                        if let Some((var_id, links)) = state {
                            if !links.is_empty() {
                                let name = id.name.clone();
                                self.error_derived(id.span, "call", &name, &links);
                            } else {
                                let name = id.name.clone();
                                self.poison_derived(
                                    var_id,
                                    &name,
                                    FateEvent::Moved,
                                    span,
                                    // A whole variable: every derived value falls
                                    // [fate-field-disjoint].
                                    Some(&[]),
                                );
                                if let Some(var) = self.lookup_mut(&id.name) {
                                    var.narrowed = Ty::Nothing;
                                    var.consumed_by = Some(
                                        "a call (a `once` function is callable \
                                         at most once)",
                                    );
                                }
                            }
                        }
                    }
                    return *ret;
                }
                // [call-resolve] The local's type is known and is not a
                // function, so this call can never succeed — including a
                // generic `T`, which is opaque (no bounds to make it
                // callable). Only an *un-inferred* type stays lenient
                // ([type-unknown-lenient]).
                for a in args {
                    self.check_expr(a, None);
                }
                if !vty.is_unknown() {
                    let detail = if matches!(vty, Ty::Var(_)) {
                        // Adding a declaration cannot make a `T` callable:
                        // there are no bounds to say it is a function.
                        format!(
                            "`{}` is a type parameter (`{vty}`), so nothing is \
                             known about its values — including whether they \
                             are functions. Declare it with a fn type to call \
                             it",
                            id.name
                        )
                    } else {
                        format!("`{}` is not callable: its type is `{vty}`", id.name)
                    };
                    self.error(span, detail);
                }
                return Ty::Unknown;
            }
            let arg_refs: Vec<&'p Expr> = args.iter().collect();
            // [mixed-handler] Inside a façade member, a bare call naming one
            // of the handler's own `send fn` members is a send to the
            // servant — resolved before the general ladder (the handler's
            // members are the innermost declaration scope), after locals
            // (which shadow, as they shadow everything).
            if let Some(ty) = self.check_facade_send(id, args, span) {
                return ty;
            }
            return self.resolve_named_call(
                &id.name, id.span, type_args, &arg_refs, named, expected, None, span,
            );
        }

        // Computed callee (a `once`-typed temporary is called at most
        // once by construction [once-fn]).
        let cty = self.check_expr(callee, None);
        if let Ty::Fn {
            params,
            ret,
            contract: _,
            effects,
        } = cty.strip_quals().clone()
        {
            // [fn-effects]
            self.check_fn_value_effects(&effects, span);
            for (i, a) in args.iter().enumerate() {
                self.check_expr(a, params.get(i));
            }
            return *ret;
        }
        // [call-resolve] As above: a known non-fn type is not callable.
        for a in args {
            self.check_expr(a, None);
        }
        if !cty.is_unknown() && !matches!(cty, Ty::Nothing) {
            self.error(
                span,
                format!("this expression is not callable: its type is `{cty}`"),
            );
        }
        Ty::Unknown
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_named_call(
        &mut self,
        name: &str,
        name_span: Span,
        type_args: &'p [ast::Type],
        args: &[&'p Expr],
        named: &'p [ast::NamedArg],
        expected: Option<&Ty>,
        // [fn-overload-at] The module written after `@`, when the call names
        // one: only that module's overloads compete.
        at: Option<&'p [ast::Ident]>,
        span: Span,
    ) -> Ty {
        // Argument types computed while deciding *which* overload set a
        // colliding name belongs to [effect-available]: the fn path below
        // takes them as they are instead of typing the arguments twice.
        let mut pre_typed: Option<Vec<Ty>> = None;
        // 1. Effect member call: resolve which effect instance in scope
        // provides it (validating availability and disambiguating generic
        // effects). [effect-member-overload] Several effects may declare
        // the member name: the one with an instance available wins; two
        // available (or none) is an error naming the `member@Effect(…)`
        // form [effect-at].
        //
        // [effect-available] A written `@module` selector names a *module's
        // overloads*, which no member ever is, so it skips this block whole:
        // that is how a fn shadowed by a member is called by hand.
        if let Some(members) = self.scope.effect_members.get(name).filter(|_| at.is_none()) {
            let members = members.clone();
            // [effect-available] **Availability decides first** (2026-09-14):
            // a member call needs an instance, so a name that is *also* an
            // ordinary fn is that fn wherever no handler is in scope — and
            // `@` is not needed to say so. Without this rule std's `Fs`
            // claimed `close`, `write`, `read_line` and `position`
            // program-wide, and a program with a `close` of its own stopped
            // compiling the moment the module existed, whether or not it
            // opened a file. The multi-owner branch below already read
            // availability this way; this is the single-owner case of the
            // same rule.
            let any_available = members.iter().any(|(e, _)| {
                self.effect_env
                    .iter()
                    .any(|t| matches!(t, Ty::Named { name: n, .. } if *n == e.name.name))
            });
            if !any_available && !self.overloads_of(name).is_empty() {
                // Fall through to the fn overloads below — and tell the
                // emitters, whose own "is this a member?" map cannot see
                // scopes [effect-available].
                self.out.fn_over_member_calls.insert(self.key(span));
            } else {
            // [lsp-definition] members have no `FnKey`; the def-site table
            // carries their declaration span.
            self.record_def_ref(name_span, name);
            // [effect-member-overload] One candidate per *effect*: within one
            // effect the name may be overloaded, and those entries are one
            // choice, made by the argument types further down — not an
            // ambiguity between effects.
            let mut owners_of: Vec<&'p ast::EffectDecl> = Vec::new();
            for (e, _) in &members {
                if !owners_of.iter().any(|o| o.name.name == e.name.name) {
                    owners_of.push(e);
                }
            }
            let effect = if owners_of.len() == 1 {
                owners_of[0]
            } else {
                let available: Vec<&'p ast::EffectDecl> = owners_of
                    .iter()
                    .filter(|e| {
                        self.effect_env.iter().any(
                            |t| matches!(t, Ty::Named { name: n, .. } if *n == e.name.name),
                        )
                    })
                    .copied()
                    .collect();
                let owners = || {
                    owners_of
                        .iter()
                        .map(|e| format!("`{}`", e.name.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                match available.as_slice() {
                    [one] => *one,
                    [] => {
                        for a in args {
                            self.check_expr(a, None);
                        }
                        self.error(
                            span,
                            format!(
                                "`{name}` is a member of {}, none of which has a \
                                 handler in scope: declare one in the function's \
                                 effect list or `use` a handler",
                                owners()
                            ),
                        );
                        return Ty::Unknown;
                    }
                    more => {
                        let first = more[0].name.name.as_str();
                        for a in args {
                            self.check_expr(a, None);
                        }
                        self.error(
                            span,
                            format!(
                                "`{name}` is a member of {} — more than one is in \
                                 scope, so the call must pick its effect: \
                                 `{name}@{first}(…)` [effect-at]",
                                owners()
                            ),
                        );
                        return Ty::Unknown;
                    }
                }
            };
            // [effect-member-overload] The effect is chosen; its overloads of
            // this name are what the argument types choose between.
            let overloads = crate::effect_members_named(effect, name);
            // [effect-available] **One overload set** (user decision
            // 2026-09-14): where the name is *also* a fn, both sides compete
            // and the more specific signature wins. Only the colliding case
            // takes this route — with candidates on one side only, that
            // side's path runs exactly as it always did.
            let fn_cands = self.overloads_of(name);
            if fn_cands.is_empty() {
                return self.check_effect_call(
                    effect, &overloads, type_args, args, named, expected, None, span,
                );
            }
            match self.route_member_or_fn(name, effect, &overloads, &fn_cands, args, span) {
                Route::Member(arg_tys) => {
                    return self.check_effect_call(
                        effect,
                        &overloads,
                        type_args,
                        args,
                        named,
                        expected,
                        Some(arg_tys),
                        span,
                    );
                }
                Route::Fn(arg_tys) => {
                    // The fn side won: fall through, with the types already
                    // in hand and the emitters told [effect-available].
                    self.out.fn_over_member_calls.insert(self.key(span));
                    pre_typed = Some(arg_tys);
                }
                Route::Neither => return Ty::Unknown,
            }
            }
        }

        // 2. Function overloads [fn-overload] (fn declarations, else
        // signatures).
        // [fn-rename] The overloads that still answer to this name — a
        // renamed one answers only to its new name — and, when the name *is*
        // a rename, exactly the overload it points at.
        let candidates: Vec<crate::resolve::FnEntry<'p>> = self.overloads_of(name);
        if self.renamed(name).is_some() {
            let key = self.key(span);
            self.out.renamed_calls.insert(key);
            if at.is_some() {
                self.error(
                    span,
                    format!(
                        "`{name}` is a renamed overload, so it already names one \
                         declaration: drop the `@module`"
                    ),
                );
            }
        }
        if candidates.is_empty() {
            // 3. Handler constructor.
            for a in args {
                self.check_expr(a, None);
            }
            if self.scope.handlers.contains_key(name) {
                // [handler-not-value] A handler instance is produced by
                // `use` and lives in the effect environment; it is not a
                // value the program can name, store, or pass. `use` never
                // reaches here (it validates its own constructor), so this
                // is always a handler *value* — which neither backend can
                // render (Rust emits `Name()` for a struct with no such
                // constructor, `E0423`).
                self.error(
                    name_span,
                    format!(
                        "`{name}` is a handler, not a value: register it with \
                         `use {name}(...)` and call the effect's members"
                    ),
                );
                return Ty::Unknown;
            }
            // Nothing declares this name [call-resolve]. Reaching a target
            // function means declaring it, so an
            // unresolved callee is an error rather than interop
            // pass-through (user decision 2026-09-03).
            self.error_unresolved(
                name_span,
                format!("no function named `{name}` is in scope"),
                name,
            );
            return Ty::Unknown;
        }

        // [effect-available] The fn path is committed, so tell the emitters —
        // whose own "is this name an effect member?" question is asked of the
        // **program-wide, scope-blind** `Symbols::effect_of_fn`, while the
        // checker has just resolved a fn declaration.
        //
        // The two fall-throughs inside the member block above record the case
        // where the member *was* in this scope and lost. This records the
        // cases where that block never ran at all: the name is a member of an
        // effect not in scope here, or the call wrote an explicit `@module`
        // selector to reach the fn on purpose. The first is a user effect's
        // member name colliding with a std fn — `effect Tally { fn add(n: Int)
        // }` made *std's* `core/seq.sv` emit a member dispatch for its own
        // `add(out, x)`, reported as "no handler for effect `Tally`" from
        // inside std (found 2026-09-15). The checker was right and silent; the
        // record is what makes the emitters agree.
        if self.symbols.effect_of_fn.contains_key(name) {
            self.out.fn_over_member_calls.insert(self.key(span));
        }

        // Type the arguments once, then match candidates against them.
        // The **lead** candidate's parameter types flow into the arguments
        // as expected types — which is what lets lambda literals infer
        // their parameter types and inherit fn-type contracts
        // [fn-contract].
        //
        // [fn-overload-rank] The lead is the single candidate, or — when
        // a name is overloaded — the most specific of those still compatible
        // with the arguments typed *so far*. Both halves matter: specificity
        // picks the candidate the ranking will pick anyway (so a bare lambda
        // has an expected type even for an overloaded name, which is what
        // std's `map(xs, n -> n * 2)` needs once a `List` fast path joins the
        // generic overload), and re-narrowing per argument is what keeps the
        // *subject* deciding — `map(arr, n -> n + 1)` drops the `List`
        // candidate when `arr` turns out to be an array, before the lambda
        // is typed against it.
        //
        // Expected types are only a hint: the scoring loop below re-derives
        // everything from the argument types it ends up with.
        //
        // [call-generic-progressive] Within the lead, its type variables bind
        // *progressively*, left to right: each argument's expected type is
        // the parameter pattern with everything the earlier arguments (and
        // any explicit type arguments) already determined substituted in.
        // Without this an un-annotated lambda would be checked against a
        // pattern still mentioning `T` — its inferred type would then
        // *contain the callee's own variable*, which is exactly what
        // `unify`'s deliberate lack of an occurs check assumes cannot
        // happen [fn-overload], so the call would fail to match itself
        // (`map(xs.iter(), n -> n * 2)` reported as
        // `map(Iter<Int>, (T) -> T)`). The same progressive rule effect
        // member generics already use [effect-member-generics].
        // [effect-available] Already typed, because this name's member and fn
        // candidates were ranked as one overload set: type them again and
        // every argument is checked twice and every mistake reported twice.
        // A colliding call therefore has no lead candidate and no expected
        // types — the recorded cut on that rule.
        let mut pool: Vec<LeadCandidate> = if pre_typed.is_some() {
            Vec::new()
        } else {
            self.lead_pool(&candidates, args.len(), type_args)
        };
        let typed_already = pre_typed.is_some();
        let mut arg_tys: Vec<Ty> = pre_typed.unwrap_or_else(|| Vec::with_capacity(args.len()));
        for (i, a) in args.iter().enumerate() {
            if typed_already {
                break;
            }
            let lead = Self::dominant_lead(&pool);
            let exp: Option<Ty> = match lead {
                Some(pi) => {
                    // [implicit-infer] What the *implicit* parameters
                    // determine counts as progress too, and it has to happen
                    // between the arguments: `map(xs, n -> n * 2)` learns `T`
                    // from resolving `iter` at `(It) -> Iter<T>` with `It`
                    // already bound by `xs`, and without it the lambda is
                    // typed against an unbound `T`.
                    let key = Some(candidates[pool[pi].index].key);
                    let mut subst = std::mem::take(&mut pool[pi].subst);
                    let generics = pool[pi].generics.clone();
                    self.extend_subst_from_implicits(key, &generics, &mut subst);
                    pool[pi].subst = subst;
                    pool[pi]
                        .rank
                        .patterns
                        .get(i)
                        .map(|p| substitute_vars(p, &pool[pi].subst, &pool[pi].generics))
                }
                None => None,
            };
            let lead_generics: HashSet<String> = match lead {
                Some(pi) => pool[pi].generics.clone(),
                None => HashSet::new(),
            };
            // Only lambda literals benefit; other expressions keep
            // the historical untyped probe (expected types can
            // trigger coercion recording).
            let ty = match a {
                Expr::Lambda { .. } => self.check_expr(a, exp.as_ref()),
                // [call-type-args] A *concrete* parameter type flows
                // into a nested call, so that call can infer its own
                // type arguments from where its result is going
                // (`takes_ints(mut_list_of())`). A pattern still
                // mentioning the callee's generics must not: coercion
                // would be recorded against an unsubstituted `T`.
                // [col-literal] A collection literal is in the same
                // position: `takes_set({})` and `takes_ints([])` have
                // nothing *inside* the literal to infer from, so the
                // parameter's type is what says which collection it is and
                // what it holds. Guarded like the call case — a pattern
                // still mentioning the callee's generics would record a
                // coercion against an unsubstituted `T`.
                Expr::Call { .. }
                | Expr::ArrayLit { .. }
                | Expr::SetLit { .. }
                | Expr::MapLit { .. } => {
                    let concrete = exp.filter(|t| !ty_mentions_vars(t, &lead_generics));
                    self.check_expr(a, concrete.as_ref())
                }
                // [fn-effects] A *named fn* passed by value needs the
                // expected fn type too: what the position expects it to
                // accept decides the value's shape (a pure fn passed
                // where effects are expected still takes them).
                Expr::Ident(id)
                    if self.lookup(&id.name).is_none()
                        && self.scope.fns.contains_key(id.name.as_str()) =>
                {
                    self.check_expr(a, exp.as_ref())
                }
                // [lit-adopt] A numeric literal adopts the numeric type its
                // *parameter* expects — the rule's own `f(1)` into a `Long`
                // parameter, which until 2026-09-18 only worked for annotated
                // `let`s and struct fields, so `millis(500)` was refused with
                // "no matching overload for `millis(Int)`" and every call in
                // std's new time surface would have needed `500L`.
                //
                // Read off the *candidates* rather than the lead, so adoption
                // stays a fallback and never re-ranks an overload set: an
                // exact `Int` (or `Double`) candidate at this position blocks
                // it, and the adopted type must be the only numeric one the
                // candidates name there. Generic patterns are not numeric, so
                // they neither block nor supply a target.
                Expr::Int { .. } | Expr::Float { .. } => {
                    let want = Self::adoptable_numeric_param(
                        &pool,
                        i,
                        matches!(a, Expr::Float { .. }),
                    );
                    self.check_expr(a, want.as_ref())
                }
                _ => self.check_expr(a, None),
            };
            // Extend each pool candidate's bindings with what this argument
            // determined, and drop the ones it rules out — the narrowing
            // that keeps the lead honest. A candidate that no longer fits is
            // not an error here: the scoring loop below reports the call as
            // a whole, against the *un-substituted* patterns.
            pool.retain_mut(|c| match c.rank.patterns.get(i) {
                Some(pattern) => {
                    let mut extended = c.subst.clone();
                    if unify(pattern, &ty, &mut extended) {
                        c.subst = extended;
                        true
                    } else {
                        false
                    }
                }
                None => true,
            });
            arg_tys.push(ty);
        }

        let mut viable: Vec<Viable<'p>> = Vec::new();

        let mut proj_blocked: Vec<(String, String, usize, ProjBlock)> = Vec::new();
        for entry in &candidates {
            let (key, decl) = (entry.key, entry.decl);
            // [implicit-param] Implicits are never passed positionally, so
            // they take no part in arity or ranking.
            let fixed: Vec<&Param> = decl
                .params
                .iter()
                .filter(|p| !p.variadic && !p.implicit)
                .collect();
            let variadic = decl.params.iter().find(|p| p.variadic);
            let arity_ok = if variadic.is_some() {
                args.len() >= fixed.len()
            } else {
                args.len() == fixed.len()
            };
            if !arity_ok {
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            // Pattern types for each argument slot.
            let mut patterns: Vec<Ty> = Vec::with_capacity(args.len());
            for (i, _) in args.iter().enumerate() {
                if i < fixed.len() {
                    patterns.push(self.lower_type(&fixed[i].ty));
                } else if let Some(vp) = variadic {
                    let arr = self.lower_type(&vp.ty);
                    let elem = match (&arr, args[i]) {
                        // `...spread` passes the whole array along.
                        (_, Expr::Spread { .. }) => arr.clone(),
                        (Ty::Array(e), _) => (**e).clone(),
                        _ => arr.clone(),
                    };
                    patterns.push(elem);
                }
            }
            self.generics = saved;

            let mut subst = HashMap::new();
            let ok = patterns
                .iter()
                .zip(&arg_tys)
                .all(|(p, a)| unify(p, a, &mut subst));
            if !ok {
                continue;
            }
            let callee_generics: HashSet<String> =
                decl.generics.iter().map(|g| g.name.clone()).collect();
            // Explicit type args pin the substitution.
            for (g, ta) in decl.generics.iter().zip(type_args) {
                let lowered = self.lower_type(ta);
                subst.insert(g.name.clone(), lowered);
            }
            let mut pairings = Vec::new();
            let mut assignable = true;
            // [proj-type] A kept, non-`Mut` position reads its argument, so a
            // *top-level* projection of the parameter type fits it (the
            // borrow of an owned value is a borrow); a consumed or `Mut`
            // position needs the owned value, and a projection *inside* the
            // type (a union arm, a type argument) must match exactly.
            let contract = self.effective_contract(Some(key), decl);
            for (i, p) in patterns.iter().enumerate() {
                let sp = substitute_vars(p, &subst, &callee_generics);
                let kept = contract
                    .as_ref()
                    .and_then(|c| {
                        let pname = fixed.get(i).map(|fp| fp.name.name.as_str());
                        c.iter()
                            .find(|d| Some(d.param.as_str()) == pname)
                            .map(|d| d.kept)
                    })
                    .unwrap_or(true);
                let fits = self.arg_fits_param(&arg_tys[i], &sp, kept);
                if !fits {
                    // Remember when *only* the projection stood in the way,
                    // for the diagnostic below.
                    if is_subtype(&strip_all_proj(&arg_tys[i]), &strip_all_proj(&sp)) {
                        let pname = fixed
                            .get(i)
                            .map(|fp| fp.name.name.clone())
                            .unwrap_or_else(|| format!("argument {}", i + 1));
                        let why = if arg_tys[i].is_proj() && Self::carries_mut(&sp) {
                            ProjBlock::Mutates
                        } else if arg_tys[i].is_proj() {
                            ProjBlock::Consumes
                        } else {
                            ProjBlock::Nested
                        };
                        proj_blocked.push((decl.name.name.clone(), pname, i, why));
                    }
                    assignable = false;
                    break;
                }
                pairings.push((i, sp));
            }
            if !assignable {
                continue;
            }
            // [fn-overload-rank] The slots a *variadic* parameter collected
            // are what make the arity shape matter: with everything else
            // equal, a fixed list wins.
            let collects_variadic = variadic.is_some() && args.len() >= fixed.len();
            viable.push(Viable {
                key: Some(key),
                decl,
                subst,
                pairings,
                rank: crate::types::RankedCandidate {
                    patterns,
                    variadic: collects_variadic,
                },
                rung: entry.rung,
                module: entry.module.to_string(),
            });
        }

        if viable.is_empty() {
            let shown: Vec<String> = arg_tys.iter().map(|t| t.to_string()).collect();
            // [proj-type] When a projection is all that stands between the
            // arguments and a candidate, say so — and say the remedy.
            if let Some((callee, pname, i, why)) = proj_blocked.first().cloned() {
                let arg_shown = arg_tys.get(i).map(|t| t.to_string()).unwrap_or_default();
                let what = match args.get(i) {
                    Some(Expr::Ident(id)) => format!("`{}`", id.name),
                    _ => "this argument".to_string(),
                };
                let msg = match why {
                    ProjBlock::Mutates => format!(
                        "{what} is a projection (`{arg_shown}`), which can only be read — a \
                         `proj` value never satisfies a `Mut` position; `{callee}` mutates \
                         `{pname}`. Use `copy(...)` for a value of your own"
                    ),
                    ProjBlock::Consumes => format!(
                        "{what} is a projection (`{arg_shown}`), and `{callee}` consumes \
                         `{pname}`: a borrowed value cannot be given away. Pass \
                         `copy(...)`, or keep the parameter (`=> {pname}`)"
                    ),
                    ProjBlock::Nested => format!(
                        "{what} holds a borrowed value (`{arg_shown}`) where `{callee}` \
                         expects an owned one for `{pname}`: the two are the same on the \
                         JVM and different in Rust. Write the parameter's type with the \
                         `proj` (as the argument has it), or pass `copy(...)`"
                    ),
                };
                self.error(span, msg);
                return Ty::Unknown;
            }
            self.error(
                span,
                format!("no matching overload for `{name}({})`", shown.join(", ")),
            );
            return Ty::Unknown;
        }
        let Some(best_idx) = self.select_overload(name, &viable, &arg_tys, at, span) else {
            return Ty::Unknown;
        };
        let best = &viable[best_idx];
        if let Some(key) = best.key {
            self.out.call_fn.insert(self.key(span), key);
            // The callee name resolves to this declaration [fn-ref-table].
            self.out.fn_refs.insert(self.key(name_span), key);
        }
        let callee_generics: HashSet<String> =
            best.decl.generics.iter().map(|g| g.name.clone()).collect();
        let subst = best.subst.clone();
        let decl = best.decl;
        let best_key = best.key;
        // Record argument coercions against the selected parameter types.
        for (i, pt) in best.pairings.clone() {
            let logical = arg_tys[i].clone();
            let repr = self.repr_of(args[i], &logical);
            self.maybe_coerce(args[i].span(), &logical, &repr, &pt);
            // [type-none-unit] A generic slot the call inferred as `None`
            // takes the unit value, not an optional.
            self.note_none_unit(args[i], &pt);
        }
        // [linear-discard] `discard` is the obligation's **terminal**, and
        // it is legal only inside a *discharger* of the value's type — a fn
        // declared in the type's own file that consumes a parameter of it
        // (user decision 2026-09-12; before, discard refused linear values
        // outright and the designated `close` parameter was silently
        // exempt). Anywhere else, dropping a handle is precisely the leak
        // the obligation exists to prevent. Keyed on core's `intrinsic fn
        // discard` rather than on the bare name [intrinsic-std-only].
        if best.decl.name.name == "discard" && best.decl.intrinsic && args.len() == 1 {
            let arg_ty = arg_tys[0].clone();
            if self.ty_own_linear(&arg_ty) {
                let allowed = match arg_ty.strip_quals() {
                    Ty::Named { name, .. } => self.own_discharges.contains(name.as_str()),
                    _ => false,
                };
                if !allowed {
                    let set = self.linear_discharge_hint(&arg_ty);
                    self.error(
                        span,
                        format!(
                            "`discard` cannot drop a linear value (`{arg_ty}`) here: \
                             that is the leak the obligation exists to prevent — \
                             discharge it with {set}, or `discard` it inside one of \
                             those (a fn, or an effect member, declared in the \
                             type's own file and consuming it — for a member, \
                             inside any handler's implementation of it)"
                        ),
                    );
                }
            }
        }
        // [iter-mut-param] The callback half of the rule: a callee that
        // **keeps** a fn-typed parameter calls it long after this call returns
        // — a composed pass calls it once per element, for as long as the pass
        // lives — so a lambda carrying mutable state into such a position is
        // the same hazard as a mutable parameter, reached through a capture
        // instead. Rust refuses it structurally (a stored callback arrives as
        // `impl Fn + 'static` [rs-fn-field], so neither a write nor a borrow of
        // outer mutable data compiles) while Kotlin runs it happily,
        // accumulating across passes: a checker-clean program only one backend
        // can build. Owned here rather than left to rustc.
        //
        // The trigger is the *deduction*, not the callee's shape: R5 removed the
        // producer factory, so "held past the call" is exactly "moved into the
        // callee" — which is what a composed combinator's `-> []` says.
        if self.inferred.is_some() {
            let facts: Option<Vec<crate::deduce::ParamDeduction>> =
                self.effective_contract(best_key, decl);
            for (i, arg) in args.iter().enumerate() {
                if !matches!(arg, Expr::Lambda { .. }) {
                    continue;
                }
                let stored = decl.params.get(i).is_some_and(|p| {
                    matches!(p.ty, ast::Type::Fn { .. })
                        && facts
                            .as_ref()
                            .is_some_and(|fs| fs.iter().any(|d| d.param == p.name.name && !d.kept))
                });
                if !stored {
                    continue;
                }
                let Some(captures) = self.out.lambda_captures.get(&self.key(arg.span())) else {
                    continue;
                };
                let offenders: Vec<(String, bool)> = captures
                    .iter()
                    .filter(|c| c.mutable || c.mutated)
                    .map(|c| (c.name.clone(), c.mutated))
                    .collect();
                let param_name = decl
                    .params
                    .get(i)
                    .map(|p| p.name.name.clone())
                    .unwrap_or_else(|| format!("#{}", i + 1));
                for (captured, mutated) in offenders {
                    let what = if mutated {
                        "writes through"
                    } else {
                        "reads mutable data through"
                    };
                    self.error(
                        arg.span(),
                        format!(
                            "this lambda {what} `{captured}`, and it is the `{param_name}` \
                             callback of the iterator function `{}`: a producer's callback \
                             runs once per element in *every* pass, so what the capture \
                             accumulates would depend on how often the producer was \
                             consumed. Capture an immutable snapshot instead (bind \
                             `copy({captured})` to a local at a non-`Mut` type *before* the \
                             lambda), or pass the state in and yield it back",
                            decl.name.name
                        ),
                    );
                }
            }
        }
        // [implicit-infer] A last round of learning, now that every argument is
        // typed. An implicit whose *result* determines a type variable —
        // `?iter: (c: C) -> [] Mut It`, where nothing but the chosen `iter`
        // says what `It` is — can only teach it once `C` is known, and `C` is
        // known only after the arguments. The rounds inside the loop above run
        // *between* arguments, so without this one a container-shaped
        // combinator could never infer its pass type.
        let mut subst = subst;
        self.extend_subst_from_implicits(best_key, &callee_generics, &mut subst);
        // [proj-type] The pass the spread is filled from decides whether the
        // element is borrowed — before the `next` is looked for at that type.
        self.learn_yield_elems(decl, &callee_generics, &mut subst);
        // [implicit-resolve] Fill the callee's implicit parameters, now that
        // its type arguments are known.
        self.resolve_implicits(decl, best_key, &subst, &callee_generics, named, span);
        // [linear-generics] An unconstrained generic parameter cannot be
        // instantiated with a linear type: generic code neither knows
        // nor honors the obligation. `discard` is the one blessed
        // generic (deliberately dropping is its purpose); `copy` refuses
        // with its own message (duplicating an obligation is
        // meaningless).
        if self.inferred.is_some() {
            let opted: HashSet<&str> = decl
                .generic_canbe
                .iter()
                .filter(|(_, q)| q.name.name == "linear")
                .map(|(id, _)| id.name.as_str())
                .collect();
            // Sorted, so which of several offending type arguments is
            // reported does not depend on hash order.
            let mut instantiations: Vec<(&String, &Ty)> = subst.iter().collect();
            instantiations.sort_by(|a, b| a.0.cmp(b.0));
            for (var_name, ty) in instantiations {
                if !callee_generics.contains(var_name) {
                    continue;
                }
                if !self.ty_own_linear(ty) {
                    continue;
                }
                // [linear-composite] The callee takes a bare `T` and puts
                // a `T` *inside* a composite — `add(list: Mut List<T>,
                // elem: T)` is the shape — so this call is the store, and
                // the opt-in does not help: no container can carry the
                // obligation yet. A signature that only *reads* a
                // composite of `T` (`size(list: List<T>) -> Int`) stores
                // nothing and stays legal.
                let takes_bare = decl
                    .params
                    .iter()
                    .any(|p| !p.variadic && Self::type_is_bare_var(&p.ty, var_name));
                let stores = if takes_bare {
                    decl.params
                        .iter()
                        .map(|p| &p.ty)
                        .chain(decl.return_type.as_ref())
                        .find(|t| self.var_in_composite(t, var_name))
                        .map(|t| format!("{t}"))
                } else {
                    None
                };
                if let Some(composite) = stores {
                    let linear = format!("{ty}");
                    self.refuse_linear_composite(
                        span,
                        &linear,
                        format!(
                            "stored in the `{composite}` of `{name}` (type argument \
                             `{var_name}`)"
                        ),
                    );
                    break;
                }
                // [linear-generics] `<T canbe linear>` admits linear
                // instantiation: the callee's body honors the obligation
                // (for an `intrinsic fn`, the backend's lowering does).
                if opted.contains(var_name.as_str()) {
                    continue;
                }
                if decl.intrinsic && decl.name.name == "copy" {
                    self.error(
                        span,
                        format!(
                            "cannot `copy` a value of linear type `{ty}`: \
                             that would duplicate its obligation; every \
                             linear value has exactly one owner"
                        ),
                    );
                } else {
                    self.error(
                        span,
                        format!(
                            "cannot instantiate generic parameter `{var_name}` \
                             of `{name}` with linear type `{ty}`: `{name}` does \
                             not declare `<{var_name} canbe linear>`, so it does \
                             not honor the use obligation"
                        ),
                    );
                }
                break;
            }
        }
        // [col-key-eligible] The inferred half of the key rule: a call that
        // *builds* a keyed collection without the type being written
        // anywhere (`set_of(1.5)`) has its key only in the resolved
        // substitution, so it is checked here rather than in
        // `validate_type`.
        if self.inferred.is_some() {
            if let Some(ret) = decl.return_type.as_ref() {
                if !subst.is_empty() {
                    let lowered = self.lower_type_subst(ret, &subst, 0);
                    // [col-sorted-list] A constructor's `as Q` lives beside the
                    // return type, not in it, so `sort`'s `Sorted` claim would
                    // be invisible here — and `sort(list_of(unorderable))` is
                    // exactly the inferred case this block exists for.
                    let lowered = match &decl.constructs {
                        Some(cref) => lowered.qualify(vec![crate::types::Qual {
                            effect: false,
                            name: cref.name.name.clone(),
                            args: Vec::new(),
                        }]),
                        None => lowered,
                    };
                    self.check_key_eligibility_ty(&lowered, span, 0);
                }
            }
        }
        // [deduce-consume] Deduction lists are a contract, enforced
        // flow-sensitively on bare identifier arguments: parameters *not*
        // kept are consumed (moved) — the variable narrows to `Nothing`
        // and any later use is an error until it is reassigned. Kept
        // parameters shed their removal set — computed from the entry's
        // effect against the qualifiers the argument carries
        // [deduce-syntax] — so e.g.
        // `remove_first(list: NonEmpty Mut List<T>) -> [list: Mut]`
        // leaves the argument un-`NonEmpty` and a second call fails
        // overload resolution. Written lists are enforced directly;
        // unannotated fns are enforced through round one's *inferred*
        // facts (`self.inferred`), so `return list` in a callee consumes
        // the caller's argument just like an explicit `[]`.
        let contract: Option<Vec<crate::deduce::ParamDeduction>> = match &decl.deductions {
            // Shape errors on written lists are reported by the deduce
            // pass; the mapping here is silent.
            Some(_) => self.effective_contract(best.key, decl),
            None => match self.inferred {
                Some(table) => best.key.and_then(|key| table.get(&key).cloned()),
                // Round one has no inferred facts yet: fall back to the
                // optimistic contract (everything kept with its declared
                // qualifiers). It enforces no moves and no qualifier
                // removal — its only effect is the kept-`Mut` *mutation*
                // events, which S2's mode inference needs to see in
                // round one so mutation-driven move-mode candidates are
                // recorded before round two declares the bindings
                // [fate-move-mode].
                None => Some(crate::deduce::optimistic(decl)),
            },
        };
        if let Some(contract) = contract {
            let fixed_count = decl
                .params
                .iter()
                .filter(|p| !p.variadic && !p.implicit)
                .count();
            // [deduce-same-call] Arguments are evaluated left to right:
            // a value moved by an earlier argument of *this* call cannot
            // be mentioned by a later one. (Argument typing runs before
            // this loop, so the ordinary consumed-read error cannot see
            // sibling-argument moves.)
            let mut consumed_here: Vec<String> = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                if let Some(name) = consumed_here
                    .iter()
                    .find(|n| expr_mentions(arg, n))
                    .cloned()
                {
                    self.error(
                        arg.span(),
                        format!(
                            "`{name}` cannot be used here: it was consumed (moved) \
                             by an earlier argument of this call (arguments are \
                             evaluated left to right); use `copy` at the argument \
                             that consumes it"
                        ),
                    );
                }
                if i >= fixed_count {
                    // [linear-generics] Variadic positions are untracked
                    // by the flow analysis, so a linear value passed
                    // there would leave its obligation unresolvable
                    // (physically moved, statically still owed).
                    if self.inferred.is_some() {
                        let ty = self
                            .out
                            .ty_of(self.file_idx, arg.span())
                            .cloned()
                            .unwrap_or(Ty::Unknown);
                        if self.ty_own_linear(&ty) {
                            self.error(
                                arg.span(),
                                "a linear value cannot be passed in a variadic \
                                 position (variadic arguments are not tracked); \
                                 add it to the collection individually instead",
                            );
                        }
                    }
                    continue;
                }
                let param = &decl.params[i];
                let Some(d) = contract.iter().find(|d| d.param == param.name.name) else {
                    continue;
                };
                // [proj-anywhere] A constructor lending into a `proj` arm
                // keeps its argument: the caller receives a borrow of it,
                // so nothing is moved here. Only a real constructive
                // qualifier call (`-> T as Q`) lends — anything else keeps
                // its contract and the return check reports.
                let lends = self.lending_ctor == Some(span) && decl.constructs.is_some();
                let d = if lends {
                    &crate::deduce::ParamDeduction {
                        kept: true,
                        ..d.clone()
                    }
                } else {
                    d
                };
                let Expr::Ident(id) = arg else {
                    // A non-identifier argument in a *kept `Mut`* position
                    // lets the callee mutate through the projection: that
                    // is a mutation of the argument's provenance roots
                    // [fate-poison] — otherwise a Kotlin alias would
                    // observe the mutation while Rust's clone does not
                    // (backend-parity principle).
                    if d.kept
                        && crate::deduce::declared_quals(&param.ty)
                            .iter()
                            .any(|q| q == "Mut")
                    {
                        self.fate_mutation_through(arg, span);
                    } else if !d.kept {
                        // A projection in a *moved* position moves data
                        // out of its provenance roots [fate-move-mode];
                        // narrowings of that storage fall
                        // [flow-place-invalidate].
                        let consumed = self.projection_move(arg, span);
                        if let Some(place) = Place::of_expr(arg) {
                            self.invalidate_place_narrows(&place);
                        }
                        consumed_here.extend(consumed);
                    }
                    continue;
                };
                // [once-fn] A `once` fn value escapes when passed as an
                // argument: fn-value ownership is otherwise untracked,
                // so the pass consumes it regardless of the callee's
                // contract (conservative; relaxable with fn-type
                // contracts).
                let arg_once = self
                    .lookup(&id.name)
                    .map(|v| v.narrowed.quals().iter().any(|q| q.name == "once"))
                    .unwrap_or(false);
                if !d.kept || arg_once {
                    // Moved. A fate-linked (derived) variable cannot be
                    // moved [fate-derived-readonly]; moving a root
                    // poisons the variables derived from it
                    // [fate-poison].
                    let state = self.lookup(&id.name).map(|var| (var.id, var.links.clone()));
                    let Some((var_id, links)) = state else {
                        continue;
                    };
                    if !links.is_empty() {
                        let name = id.name.clone();
                        self.error_derived(id.span, "move", &name, &links);
                        continue;
                    }
                    let name = id.name.clone();
                    // [effect-state-store] A member cannot move a value out
                    // of its handler's storage — the handler still owns it
                    // after the call returns.
                    if self.consume_of_handler_storage(&name, "move", id.span) {
                        continue;
                    }
                    // A lambda cannot consume a capture [fate-lambda].
                    if let Some(frame) = self.frame_of_id(var_id) {
                        if self.capture_move_violation(frame, &name, id.span) {
                            continue;
                        }
                    }
                    // A kept lambda parameter belongs to the caller
                    // [fn-contract].
                    if self.consume_kept_lambda_param(&name, id.span) {
                        continue;
                    }
                    self.note_linear_move(&name, id.span);
                    self.poison_derived(var_id, &name, FateEvent::Moved, span, Some(&[]));
                    if let Some(var) = self.lookup_mut(&id.name) {
                        var.narrowed = Ty::Nothing;
                        var.consumed_by = Some("an earlier call");
                    }
                    consumed_here.push(name);
                    continue;
                }
                // Kept. A parameter declared `Mut` gives the callee
                // mutation permission: mutating a derived variable is an
                // error, mutating a root poisons its derived variables
                // [fate-poison] [fate-derived-readonly], and every
                // narrowing of its parts falls
                // [flow-place-invalidate]. A kept *immutable* parameter
                // cannot mutate, so narrowings survive it (user decision
                // P1b).
                let declared_q = crate::deduce::declared_quals(&param.ty);
                if declared_q.iter().any(|q| q == "Mut") {
                    let name = id.name.clone();
                    self.fate_mutation_root(&name, span);
                }
                // [deduce-syntax] Computed against what the argument
                // actually carries: an exhaustive list drops qualifiers
                // this callee never declared.
                let have: Vec<String> = self
                    .lookup(&id.name)
                    .map(|v| v.narrowed.quals().iter().map(|q| q.name.clone()).collect())
                    .unwrap_or_default();
                let removed = d.effect.removal_set(&have, |q| self.is_provenance_qual(q));
                let name = id.name.clone();
                if !removed.is_empty() {
                    if let Some(var) = self.lookup_mut(&name) {
                        var.narrowed = var.narrowed.clone().remove_quals(&removed);
                    }
                }
                // [qual-refn] Refinements have the last word: the callee's
                // own list said what *it* can promise, and a qualifier's
                // refinement says what the call does to *that qualifier's*
                // claim. Applied after the removal set, so `+NonEmpty`
                // re-establishes what `add`'s exhaustive `[list: Mut]`
                // necessarily dropped.
                if let Some(key) = best.key {
                    self.apply_refinements(key, &param.name.name, &name, span);
                }
            }
        }
        // [readonly-return] Record derived-return calls: the result
        // borrows the annotated argument — the caller links the result
        // to it, and the Rust backend renders the result as a borrow.
        if decl.derived_return.is_some() {
            // Every source of the *first* wholesale `proj` in the return type
            // (`proj[from: a, b] T`: a projection joined across branches is
            // of both) [proj-anywhere].
            let sources: Vec<usize> = decl
                .return_type
                .as_ref()
                .map(|rt| {
                    proj_refs(rt)
                        .into_iter()
                        .find(|r| !r.from.is_empty())
                        .map(|r| {
                            r.from
                                .iter()
                                .filter_map(|f| {
                                    decl.params.iter().position(|p| p.name.name == f.name)
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            if !sources.is_empty() {
                // Whether a borrowed argument is a *temporary* (a call result
                // or literal, not a place): a view of one dies with the
                // statement, so it may be *used* there (`map(iter(list_of(1,
                // 2)), f)`) but not bound, returned or stored — those sites
                // consult this table.
                for &idx in &sources {
                    if let Some(arg) = args.get(idx).copied() {
                        if self.is_temporary(arg) {
                            self.out.temp_views.insert(self.key(span));
                        }
                    }
                }
                self.out.derived_calls.insert(self.key(span), sources);
            }
        }
        // [deduce-syntax] Re-pointing entries — `v.items: proj[from: other]`
        // or `v: proj[from: other]` — say the call makes the argument at `v`
        // hold a borrow of the argument at `other`: the variable passed as
        // `v` gains a held link to `other`'s roots. (The body is trusted for
        // these today; see ROADMAP.)
        if let Some(list) = &decl.deductions {
            let repoints: Vec<(usize, Vec<usize>)> = list
                .iter()
                .filter_map(|d| match (&d.target, &d.kind) {
                    (ast::DeductionTarget::Param { name, .. }, ast::DeductionKind::Proj(srcs)) => {
                        let target = decl.params.iter().position(|p| p.name.name == name.name)?;
                        let sources: Vec<usize> = srcs
                            .iter()
                            .filter_map(|sname| {
                                decl.params.iter().position(|p| p.name.name == sname.name)
                            })
                            .collect();
                        Some((target, sources))
                    }
                    _ => None,
                })
                .collect();
            for (target, sources) in repoints {
                let Some(Expr::Ident(target_id)) = args.get(target).copied() else {
                    continue;
                };
                let mut new_links: Vec<FateLink> = Vec::new();
                for si in sources {
                    let Some(src_arg) = args.get(si).copied() else {
                        continue;
                    };
                    for mut l in self.links_for_value(src_arg, span) {
                        l.borrowed = true;
                        l.held = true;
                        new_links.push(l);
                    }
                }
                let target_name = target_id.name.clone();
                if let Some(var) = self.lookup_mut(&target_name) {
                    for l in new_links {
                        if !var.links.iter().any(|e| e.root_id == l.root_id) {
                            var.links.push(l);
                        }
                    }
                }
            }
        }
        // [proj-infer] A result that *holds* borrows (a struct with `proj`
        // fields) ties itself to the lent arguments; the caller links the
        // result to them, held. A lent temporary is as dangling as a
        // projected one.
        if decl.derived_return.is_none() {
            let mut lent = self.infer_lends(decl);
            // [proj-type] Instantiation can make a result hold borrows the
            // declaration could not see: `keep_all<It, T>(…) -> Mut List<T>`
            // with `T = proj Str` (a `?Yield` filled from a borrowing pass)
            // returns a view. With nothing inferable from the body about
            // *which* argument, the result is linked to every kept one —
            // conservative, exact for the one-source case, and nothing for a
            // generator (`T = Int`).
            if lent.is_empty() {
                let ret_sub = decl.return_type.as_ref().map(|rt| {
                    let saved = self.enter_generics(&decl.generics);
                    let lowered = self.lower_type(rt);
                    self.generics = saved;
                    substitute_vars(&lowered, &subst, &callee_generics)
                });
                let generic_ret_holds_proj = ret_sub
                    .as_ref()
                    .is_some_and(|r| strip_all_proj(r) != *r && !r.is_proj());
                if generic_ret_holds_proj {
                    let contract = self.effective_contract(best_key, decl);
                    lent = decl
                        .params
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| !p.implicit)
                        .filter(|(_, p)| {
                            contract
                                .as_ref()
                                .and_then(|c| c.iter().find(|d| d.param == p.name.name))
                                .is_none_or(|d| d.kept)
                        })
                        .map(|(i, _)| i)
                        .collect();
                }
            }
            if !lent.is_empty() {
                let positional: Vec<usize> = lent
                    .iter()
                    .filter_map(|&pi| {
                        // Implicit params are not positional arguments.
                        let before = decl.params[..pi].iter().filter(|p| p.implicit).count();
                        if decl.params[pi].implicit {
                            None
                        } else {
                            Some(pi - before)
                        }
                    })
                    .collect();
                for &i in &positional {
                    if let Some(arg) = args.get(i).copied() {
                        if self.is_temporary(arg) {
                            self.out.temp_views.insert(self.key(span));
                        }
                    }
                }
                self.out.lending_calls.insert(self.key(span), positional);
            }
        }
        // The callee's declared effect dependencies must be satisfiable
        // here: each must match an instance in the caller's effect
        // environment (declared or `use`d).
        self.check_callee_effects(name, decl, &subst, &callee_generics, span);
        let saved = self.enter_generics(&decl.generics);
        let ret = self.fn_return_ty(decl);
        self.generics = saved;
        let subst = self.settle_type_args(name, decl, &ret, subst, expected, &arg_tys, span);
        substitute_vars(&ret, &subst, &callee_generics)
    }

    /// [call-type-args] Finishes a generic call's substitution and records
    /// it for the emitters.
    ///
    /// A type argument the *arguments* did not determine is taken from the
    /// **expected type** — the annotation on a `let`, the enclosing fn's
    /// return type, or a concrete parameter the call's result flows into.
    /// If it is still unbound and it reaches the result type, the call is an
    /// error: the checker would hand the backends a `T` it never resolved,
    /// which one target language may infer for itself and another may not
    /// (`mutableListOf()` is not valid Kotlin, while rustc infers `vec![]`
    /// backwards from a later use). Requiring the context here keeps the two
    /// backends on the same programs and makes the type known to the
    /// checker, which is what an intrinsic lowering renders.
    fn settle_type_args(
        &mut self,
        name: &str,
        decl: &'p FnDecl,
        ret: &Ty,
        mut subst: HashMap<String, Ty>,
        expected: Option<&Ty>,
        arg_tys: &[Ty],
        span: Span,
    ) -> HashMap<String, Ty> {
        if decl.generics.is_empty() {
            return subst;
        }
        let callee_generics: HashSet<String> =
            decl.generics.iter().map(|g| g.name.clone()).collect();
        if let Some(exp) = expected.filter(|e| !e.is_unknown() && **e != Ty::Nothing) {
            let mut from_expected: HashMap<String, Ty> = HashMap::new();
            if unify(ret, exp, &mut from_expected) {
                for (g, ty) in from_expected {
                    if callee_generics.contains(&g) && !ty.is_unknown() && !subst.contains_key(&g) {
                        subst.insert(g, ty);
                    }
                }
            }
        }
        // [implicit-group] [iter-protocol] A `?Yield<It, T>` spread teaches `T`
        // from the *declaration* of whatever `It` turned out to be: a pass says
        // what it yields at its `: Yield<self, T>` clause, so a combinator's
        // element type never has to be written and a bare lambda can be typed
        // against it. Reaches through an origin mint, whose machine type stands
        // for the origin [iter-fn].
        self.learn_yield_elems(decl, &callee_generics, &mut subst);
        // An `Unknown` argument means an earlier diagnostic already fired
        // (or a type the checker could not determine): stay lenient rather
        // than reporting the same mistake twice [type-unknown-lenient].
        let lenient = arg_tys.iter().any(|t| t.is_unknown());
        let missing: Vec<&str> = decl
            .generics
            .iter()
            .filter(|g| !subst.contains_key(&g.name) && ty_mentions_var(ret, &g.name))
            .map(|g| g.name.as_str())
            .collect();
        if !missing.is_empty() && !lenient {
            let list = missing
                .iter()
                .map(|g| format!("`{g}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let plural = if missing.len() == 1 { "" } else { "s" };
            self.error(
                span,
                format!(
                    "cannot infer type argument{plural} {list} of `{name}`: no \
                     argument determines {} and neither does the context — write \
                     the type argument{plural} (`{name}<...>(...)`) or annotate \
                     where the result goes",
                    if missing.len() == 1 { "it" } else { "them" }
                ),
            );
        }
        let recorded: Vec<Ty> = decl
            .generics
            .iter()
            .map(|g| subst.get(&g.name).cloned().unwrap_or(Ty::Unknown))
            .collect();
        self.out.call_type_args.insert(self.key(span), recorded);
        subst
    }

    /// Resolves each effect dependency of a called fn against the caller's
    /// effect environment [effect-fn-deps] and records the concrete
    /// instances (in declaration order) for the backend to thread as
    /// handler arguments.
    fn check_callee_effects(
        &mut self,
        name: &str,
        decl: &'p FnDecl,
        subst: &HashMap<String, Ty>,
        callee_generics: &HashSet<String>,
        span: Span,
    ) {
        let mut resolved: Vec<Ty> = Vec::new();
        // [actor-spawn-effect] The `spawn` capability **propagates like any
        // other effect**: `pool` and `watch` declare it, and std's own
        // documentation says declaring it is "what makes creating one a
        // capability the caller must hold" — so a caller that has not been
        // given it cannot reach one through a helper either. Checked here
        // rather than beside `can_spawn`'s other use because this is where a
        // callee's requirements meet the caller's environment; there is
        // nothing to *resolve*, since the capability names no instance.
        if !self.can_spawn
            && decl
                .effects
                .iter()
                .flatten()
                .any(|e| matches!(e, EffectRef::Spawn(_)))
        {
            self.error(
                span,
                format!(
                    "`{name}` requires the `spawn` capability, so this function must \
                     declare it too (`[spawn]` in its effect list)"
                ),
            );
        }
        // [free-send-fn] A send-kind function is *scheduled*, never called: a
        // call would run it here, on this thread, inside this frame — which is
        // the callback anti-pattern the kind exists to refuse.
        if decl.is_send {
            self.error(
                span,
                format!(
                    "`{name}` is a `send fn`, so it runs by being scheduled rather than \
                     called: mint a continuation for it with `replyto {name}(…)` and let \
                     whoever answers the token run it"
                ),
            );
        }
        // [waitfor-effect] The `waitfor` capability propagates the same way,
        // and for a sharper reason: a callee that may occupy the thread makes
        // *its caller* a frame that may be occupied, which is what the
        // placement check downstream reads.
        if !self.can_wait
            && decl
                .effects
                .iter()
                .flatten()
                .any(|e| matches!(e, EffectRef::WaitFor(_)))
        {
            self.error(
                span,
                format!(
                    "`{name}` may occupy the thread it runs on until an answer \
                     arrives, so this function must declare that too (`[waitfor]` in \
                     its effect list) — and whatever runs it must be placed on a \
                     `{DEDICATED_QUALIFIER} {POOL_TYPE}` (`thread()`)"
                ),
            );
        }
        // The callee's *effective* effect list: what it declares, plus what
        // it inherited from its fn-typed parameters [fn-effects] — a caller
        // has to supply those too, since they are how the callee calls the
        // value it was given.
        let mut wants: Vec<Ty> = Vec::new();
        {
            let saved = self.enter_generics(&decl.generics);
            for eff in decl.effects.iter().flatten() {
                let EffectRef::Effect(r) = eff else { continue };
                // Unknown effect names are reported at the callee's own
                // declaration; skip them here.
                if !self.scope.effects.contains_key(r.name.name.as_str()) {
                    continue;
                }
                let empty = HashMap::new();
                let lowered = self.lower_base_ref(r, &empty, 0);
                if !wants.contains(&lowered) {
                    wants.push(lowered);
                }
            }
            for p in &decl.params {
                for ty in self.inherited_fn_effects(&p.ty) {
                    if !wants.contains(&ty) {
                        wants.push(ty);
                    }
                }
            }
            self.generics = saved;
        }
        // [use-no-dup] Innermost first, shadowed duplicates hidden.
        let visible = self.visible_effects();
        for lowered in wants {
            let want = substitute_vars(&lowered, subst, callee_generics);
            // [throw] A callee that may throw does not need a handler — it
            // needs a delimiter. The call is an *exit* of everything up to
            // it, which `record_throw` accounts for.
            if let Ty::Named { name: eff, args } = &want {
                if eff == THROW_EFFECT {
                    let message = args.first().cloned().unwrap_or(Ty::Unknown);
                    self.record_throw(span, message, false);
                    continue;
                }
            }
            // Exact instance first, then a unique compatible match (the
            // callee's requirement may still contain unresolved parts).
            let found = visible.iter().find(|c| **c == want).cloned();
            let found = found.or_else(|| {
                let compatible: Vec<&Ty> = visible
                    .iter()
                    .filter(|c| unify(&want, c, &mut HashMap::new()))
                    .collect();
                match compatible.len() {
                    1 => Some(compatible[0].clone()),
                    _ => None,
                }
            });
            match found {
                Some(instance) => {
                    // [fn-effects] Feeds an enclosing lambda's inferred set.
                    self.note_effect_use(&instance);
                    resolved.push(instance);
                }
                None => {
                    self.error(
                        span,
                        format!(
                            "no handler for effect `{want}` in scope, required \
                             by `{name}` (declare it in the function's effect \
                             list or `use` a handler)"
                        ),
                    );
                    resolved.push(want);
                }
            }
        }
        if !resolved.is_empty() {
            self.out.call_effects.insert(self.key(span), resolved);
        }
    }

    /// [effect-member-overload] [effect-available] Which of `members` accept
    /// `arg_tys`, with the patterns that were compared — the ranking material
    /// for both the within-effect overload choice and the member-versus-fn
    /// comparison, so the two cannot disagree about what fits.
    ///
    /// Returned as (index into `members`, patterns).
    fn fitting_members(
        &mut self,
        effect: &'p EffectDecl,
        members: &[&'p FnDecl],
        arg_tys: &[Ty],
    ) -> Vec<(usize, crate::types::RankedCandidate)> {
        let saved = self.enter_generics(&effect.generics);
        let mut viable: Vec<(usize, crate::types::RankedCandidate)> = Vec::new();
        for (i, f) in members.iter().enumerate() {
            let inner = self.enter_generics(&f.generics);
            let patterns: Vec<Ty> = f
                .params
                .iter()
                .filter(|p| !p.variadic && !p.implicit)
                .map(|p| self.lower_type(&p.ty))
                .collect();
            self.generics = inner;
            let generic_set: HashSet<String> = effect
                .generics
                .iter()
                .chain(&f.generics)
                .map(|g| g.name.clone())
                .collect();
            let fits = patterns.len() == arg_tys.len()
                && patterns.iter().zip(arg_tys).all(|(p, a)| {
                    let mut subst: HashMap<String, Ty> = HashMap::new();
                    a.is_unknown()
                        || is_subtype(a, p)
                        || (!generic_set.is_empty() && unify(p, a, &mut subst))
                });
            if fits {
                viable.push((
                    i,
                    crate::types::RankedCandidate {
                        patterns,
                        variadic: false,
                    },
                ));
            }
        }
        self.generics = saved;
        viable
    }

    /// [effect-member-overload] Picks the overload a member call means, out
    /// of every member of `effect` with that name.
    ///
    /// Arity first, then — only when that leaves a choice — the argument
    /// types, ranked by the same specificity order function overloads use
    /// [fn-overload-rank]. The arguments are typed *once* here and handed
    /// back, so nothing is checked twice and one mistake still gets one
    /// diagnostic.
    fn pick_effect_member(
        &mut self,
        effect: &'p EffectDecl,
        members: &[&'p FnDecl],
        args: &[&'p Expr],
        // [effect-available] The argument types, when the caller has already
        // computed them — which it has whenever a name's member and fn
        // candidates were ranked as one set. Typing them again would check
        // every argument twice and report every mistake twice.
        pre_typed: Option<Vec<Ty>>,
        span: Span,
    ) -> Picked<'p> {
        match members {
            [] => return Picked::None,
            [only] => {
                return match pre_typed {
                    Some(tys) => Picked::ByArgs(only, tys),
                    None => Picked::Only(only),
                }
            }
            _ => {}
        }
        let fixed = |f: &FnDecl| -> usize {
            f.params.iter().filter(|p| !p.variadic && !p.implicit).count()
        };
        let by_arity: Vec<&'p FnDecl> = members
            .iter()
            .copied()
            .filter(|f| {
                f.params.iter().any(|p| p.variadic) && args.len() >= fixed(f)
                    || fixed(f) == args.len()
            })
            .collect();
        if let [only] = by_arity.as_slice() {
            return Picked::Only(only);
        }
        let pool: Vec<&'p FnDecl> = if by_arity.is_empty() {
            members.to_vec()
        } else {
            by_arity
        };
        let arg_tys: Vec<Ty> = match pre_typed {
            Some(tys) => tys,
            None => args.iter().map(|a| self.check_expr(a, None)).collect(),
        };
        let viable = self.fitting_members(effect, &pool, &arg_tys);
        let shown = || -> String {
            arg_tys
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        match viable.len() {
            1 => Picked::ByArgs(pool[viable[0].0], arg_tys),
            0 => {
                self.error(
                    span,
                    format!(
                        "no overload of `{}.{}` takes ({})",
                        effect.name.name,
                        members[0].name.name,
                        shown()
                    ),
                );
                Picked::None
            }
            _ => {
                let ranks: Vec<crate::types::RankedCandidate> =
                    viable.iter().map(|(_, r)| r.clone()).collect();
                match crate::types::most_specific(&ranks) {
                    Some(best) => Picked::ByArgs(pool[viable[best].0], arg_tys),
                    None => {
                        self.error(
                            span,
                            format!(
                                "ambiguous call to `{}.{}`: several overloads take \
                                 ({}), and none is more specific",
                                effect.name.name,
                                members[0].name.name,
                                shown()
                            ),
                        );
                        Picked::ByArgs(pool[viable[0].0], arg_tys)
                    }
                }
            }
        }
    }

    /// Checks a call to an effect member fn. The providing effect instance
    /// must be available (declared in the caller's effect list or `use`d)
    /// [effect-available]; generic effects are disambiguated by explicit
    /// type arguments, the argument types, and the expected type, in that
    /// order [effect-disambiguation].
    ///
    /// [effect-member-overload] `members` is every member of `effect` with
    /// the called name — several when the effect *overloads* it
    /// (`close(InStream)` / `close(OutStream)`). The overload is picked here,
    /// before the instance is resolved, since the signature is what the rest
    /// of the check runs on.
    #[allow(clippy::too_many_arguments)]
    fn check_effect_call(
        &mut self,
        effect: &'p EffectDecl,
        members: &[&'p FnDecl],
        type_args: &'p [ast::Type],
        args: &[&'p Expr],
        named: &'p [ast::NamedArg],
        expected: Option<&Ty>,
        // [effect-available] Argument types the caller already computed,
        // because it ranked this effect's members against same-named fns as
        // one overload set.
        pre_typed: Option<Vec<Ty>>,
        span: Span,
    ) -> Ty {
        // Argument types when they had to be computed early — for picking an
        // overload here, or for disambiguating instances below. Recorded once
        // so the arguments are never checked twice.
        let mut typed_args: Option<Vec<Ty>> = None;
        let member: &'p FnDecl = match self.pick_effect_member(effect, members, args, pre_typed, span) {
            Picked::Only(m) => m,
            Picked::ByArgs(m, arg_tys) => {
                typed_args = Some(arg_tys);
                m
            }
            Picked::None => return Ty::Unknown,
        };
        // [effect-member-overload] Which overload the call resolved to, for
        // the emitters: they name an overloaded member positionally, and a
        // name-keyed guess would pick the wrong one.
        if members.len() > 1 {
            if let Some(idx) = crate::effect_member_index(effect, member) {
                self.out.effect_member_calls.insert(self.key(span), idx);
            }
        }
        // [throw] The throw effect has no handler: the delimiter is `try`,
        // so its operation resolves through its own path.
        if effect.name.name == THROW_EFFECT {
            return self.check_throw_call(member, args, span);
        }
        // The member's signature, lowered with the effect's generics as
        // `Var`s — and the member's *own* generics too
        // [effect-member-generics]: they bind per call from the argument
        // types (explicit type args keep their [effect-disambiguation]
        // meaning: they pin the effect instance, not member generics).
        let saved = self.enter_generics(&effect.generics);
        self.enter_generics(&member.generics);
        let member_params: Vec<Ty> = member
            .params
            .iter()
            .map(|p| self.lower_type(&p.ty))
            .collect();
        let member_ret: Ty = member
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);
        // [implicit-param] Collected here, with the effect's and the member's
        // generics still in scope, so `?fmt: (T) -> Str` lowers to a variable
        // rather than a nominal `T`.
        let member_implicits = self.collect_implicits(member);
        self.generics = saved;
        let generic_set: HashSet<String> = effect
            .generics
            .iter()
            .chain(&member.generics)
            .map(|g| g.name.clone())
            .collect();
        let subst_for = |instance: &Ty| -> HashMap<String, Ty> {
            match instance {
                Ty::Named { args, .. } => effect
                    .generics
                    .iter()
                    .map(|g| g.name.clone())
                    .zip(args.iter().cloned())
                    .collect(),
                _ => HashMap::new(),
            }
        };

        // Instances of this effect currently available — innermost first,
        // with a shadowed registration hidden by the one that shadows it
        // [use-no-dup], so interception does not read as ambiguity.
        let candidates: Vec<Ty> = self
            .visible_effects()
            .into_iter()
            .filter(|t| matches!(t, Ty::Named { name, .. } if *name == effect.name.name))
            .collect();

        let resolved: Option<Ty> = if !type_args.is_empty() {
            // Explicit type arguments pin the instance.
            let targs: Vec<Ty> = type_args.iter().map(|t| self.lower_type(t)).collect();
            let want = Ty::Named {
                name: effect.name.name.clone(),
                args: targs,
            };
            if candidates
                .iter()
                .any(|c| *c == want || unify(c, &want, &mut HashMap::new()))
            {
                Some(want)
            } else {
                self.error(
                    span,
                    format!(
                        "no handler for effect `{want}` in scope (declare it in \
                         the function's effect list or `use` a handler)"
                    ),
                );
                None
            }
        } else if candidates.len() == 1 {
            Some(candidates[0].clone())
        } else if candidates.is_empty() {
            // [effect-handler-deps] Inside a handler member, "no handler for
            // its own effect" is not a missing registration: it is
            // *self-dispatch*, which does not exist yet. Neither remedy the
            // general diagnostic names is even available here — a member may
            // not declare effects [effect-member-no-effects] and may not
            // `use` — and declaring the effect as a dependency would bind
            // outward to the handler registered before this one
            // [effect-intercept], not to this one. Recorded as a gap in
            // ROADMAP.md.
            let own = self.handler_ofs.iter().find(
                |of| matches!(of.strip_quals(), Ty::Named { name, .. } if *name == effect.name.name),
            ).cloned();
            match own {
                Some(of) => {
                    // [actor-self-send] For a **send** member the remedy now
                    // exists: `self.k(…)`, a message to this actor, which
                    // runs as its own later activation. For a member that
                    // answers, self-dispatch is still nothing a handler can
                    // do.
                    let remedy = if member.is_send {
                        format!(
                            "send it to this actor instead: `{}@self(…)`, which runs \
                             as its own later activation",
                            member.name.name
                        )
                    } else {
                        "Move the shared logic into a function both members call"
                            .to_string()
                    };
                    self.error(
                        span,
                        format!(
                            "a handler member cannot call `{}`, a member of `{of}` — the \
                             effect its own handler implements: a handler cannot dispatch \
                             to itself, and declaring `{of}` as a dependency would bind to \
                             the handler registered *before* this one. {remedy}",
                            member.name.name
                        ),
                    )
                }
                None => self.error(
                    span,
                    format!(
                        "no handler for effect `{}` in scope (declare it in the \
                         function's effect list or `use` a handler)",
                        effect.name.name
                    ),
                ),
            }
            None
        } else {
            // Multiple instances in scope: disambiguate by the argument
            // types, then by the expected type.
            let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a, None)).collect();
            let viable: Vec<Ty> = candidates
                .iter()
                .filter(|c| {
                    let mut subst = subst_for(c);
                    member_params.iter().zip(&arg_tys).all(|(p, a)| {
                        let sp = substitute_vars(p, &subst, &generic_set);
                        unify(&sp, a, &mut subst)
                    })
                })
                .cloned()
                .collect();
            let narrowed: Vec<Ty> = match expected {
                Some(exp) if !exp.is_unknown() => {
                    let by_ret: Vec<Ty> = viable
                        .iter()
                        .filter(|c| {
                            let subst = subst_for(c);
                            let sr = substitute_vars(&member_ret, &subst, &generic_set);
                            sr.is_unknown() || is_subtype(&sr, exp)
                        })
                        .cloned()
                        .collect();
                    if by_ret.is_empty() {
                        viable.clone()
                    } else {
                        by_ret
                    }
                }
                _ => viable.clone(),
            };
            typed_args = Some(arg_tys);
            match narrowed.len() {
                1 => Some(narrowed[0].clone()),
                0 => {
                    self.error(
                        span,
                        format!(
                            "no handler for effect `{}` in scope matches this \
                             call (declare it in the function's effect list or \
                             `use` a handler)",
                            effect.name.name
                        ),
                    );
                    None
                }
                _ => {
                    self.error(
                        span,
                        format!(
                            "ambiguous effect call: multiple `{}` handlers in \
                             scope; specify the type, e.g. `{}<T>()`",
                            effect.name.name, member.name.name
                        ),
                    );
                    Some(narrowed[0].clone())
                }
            }
        };

        let mut subst = resolved.as_ref().map(&subst_for).unwrap_or_default();
        match typed_args {
            Some(arg_tys) => {
                // Args already typed: bind the member's own generics from
                // them [effect-member-generics], then record coercions
                // against the fully substituted param types.
                for (p, a) in member_params.iter().zip(&arg_tys) {
                    unify(p, a, &mut subst);
                }
                for (i, p) in member_params.iter().enumerate().take(arg_tys.len()) {
                    let sp = substitute_vars(p, &subst, &generic_set);
                    let logical = arg_tys[i].clone();
                    let repr = self.repr_of(args[i], &logical);
                    self.maybe_coerce(args[i].span(), &logical, &repr, &sp);
                    // [type-none-unit] As for a fn call: a `None`-typed slot
                    // takes the unit value.
                    self.note_none_unit(args[i], &sp);
                }
            }
            None => {
                for (i, a) in args.iter().enumerate() {
                    match member_params.get(i) {
                        Some(p) => {
                            let sp = substitute_vars(p, &subst, &generic_set);
                            let ty = self.check_expr(a, Some(&sp));
                            // Progressively bind the member's own generics
                            // [effect-member-generics]: later params and
                            // the return type see earlier bindings.
                            unify(p, &ty, &mut subst);
                        }
                        None => {
                            self.check_expr(a, None);
                        }
                    }
                }
            }
        }
        // [linear-generics] The instantiation ban reaches effect members'
        // *own* generics too (the hole closed 2026-09-12): a member
        // `log<T>(x: T)` binding `T` to a linear type would swallow the
        // obligation — no handler body is checked under a worst-case
        // linear `T` unless the member opts in with `<T canbe linear>`.
        if self.inferred.is_some() {
            let opted: HashSet<&str> = member
                .generic_canbe
                .iter()
                .filter(|(_, q)| q.name.name == "linear")
                .map(|(id, _)| id.name.as_str())
                .collect();
            let mut names: Vec<&String> = member.generics.iter().map(|g| &g.name).collect();
            names.sort();
            for var_name in names {
                if opted.contains(var_name.as_str()) {
                    continue;
                }
                if let Some(bound) = subst.get(var_name) {
                    if self.ty_own_linear(bound) {
                        self.error(
                            span,
                            format!(
                                "cannot instantiate generic parameter `{var_name}` of \
                                 effect member `{}` with linear type `{bound}`: the \
                                 member does not declare `<{var_name} canbe linear>`, \
                                 so its handlers do not honor the use obligation",
                                member.name.name
                            ),
                        );
                        break;
                    }
                }
            }
        }
        // [effect-member-call] A member call is checked against its declared
        // parameters like any other call — arity and types. It used to be
        // checked against *neither*: the member's type only flowed in as an
        // expected type, so `log(true)` on `fn log(message: Str)` was
        // accepted, and the argument reached the handler's implementation as
        // whatever it was.
        //
        // Members do not overload [effect-member-unique], so there is nothing
        // to select and nothing to rank — the name identifies one signature,
        // and these are plain mismatch diagnostics.
        {
            // [implicit-param] Implicits are never passed positionally, and a
            // variadic tail takes what is left — the same arity rule a fn call
            // uses [fn-variadic].
            let positional: Vec<usize> = member
                .params
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.implicit && !p.variadic)
                .map(|(i, _)| i)
                .collect();
            let variadic = member.params.iter().any(|p| p.variadic);
            let fixed = positional.len();
            let arity_ok = if variadic {
                args.len() >= fixed
            } else {
                args.len() == fixed
            };
            if !arity_ok {
                self.error(
                    span,
                    format!(
                        "`{}` takes {fixed} argument(s), found {}",
                        member.name.name,
                        args.len()
                    ),
                );
            }
            for (i, a) in args.iter().enumerate() {
                let Some(p) = positional.get(i).and_then(|&pi| member_params.get(pi)) else {
                    continue;
                };
                let want = substitute_vars(p, &subst, &generic_set);
                let got = self
                    .out
                    .ty_of(self.file_idx, a.span())
                    .cloned()
                    .unwrap_or(Ty::Unknown);
                if !is_subtype(&got, &want) {
                    self.error(
                        a.span(),
                        format!(
                            "`{}` expects `{want}` here, found `{got}`",
                            member.name.name
                        ),
                    );
                }
            }
        }
        // [decl-explicit] The member's declared deduction list is a real
        // contract: apply it exactly like a named call's, so a member that
        // takes ownership consumes its argument. (Validating each
        // *handler* body against the member's contract is the E1-adjacent
        // follow-up; the declaration is trusted here.)
        if let Some(list) = &member.deductions {
            let facts = crate::deduce::from_written(member, list, &HashSet::new(), |_, _| {});
            let contract: Vec<FnParamContract> = member
                .params
                .iter()
                .zip(&member_params)
                .map(|(p, pty)| {
                    let entry = facts.iter().find(|d| d.param == p.name.name);
                    FnParamContract {
                        name: Some(p.name.name.clone()),
                        kept: entry.map(|d| d.kept).unwrap_or(true),
                        effect: entry
                            .map(|d| d.effect.clone())
                            .unwrap_or(QualEffect::KeepAll),
                        mutable: pty.quals().iter().any(|q| q.name == "Mut"),
                        lent: entry.is_some_and(|d| d.lent),
                    }
                })
                .collect();
            self.apply_call_contract(args, &member_params, Some(&contract), span);
        }
        // [proj-infer] An effect member has no body: its result holds a
        // borrow of what its list declares (`[p: proj]`), else of every
        // kept parameter.
        if member.derived_return.is_none()
            && member
                .return_type
                .as_ref()
                .is_some_and(|t| crate::lends::holds_proj(t, &self.scope.structs))
        {
            let lent = crate::lends::declared_lends(member)
                .unwrap_or_else(|| crate::lends::kept_params(member));
            if !lent.is_empty() {
                self.out.lending_calls.insert(self.key(span), lent);
            }
        }
        if let Some(instance) = &resolved {
            // [fn-effects] Feeds an enclosing lambda's inferred set.
            let instance = instance.clone();
            self.note_effect_use(&instance);
            self.out.effect_calls.insert(self.key(span), instance);
        }
        // [waitfor-effect] The one capability an effect *member* may declare,
        // and it propagates to the call like any other: a member that may
        // occupy your thread makes this frame one that may be occupied.
        if !self.can_wait
            && member
                .effects
                .iter()
                .flatten()
                .any(|e| matches!(e, EffectRef::WaitFor(_)))
        {
            self.error(
                span,
                format!(
                    "`{}` may occupy the thread it runs on until an answer arrives, so \
                     this function must declare that too (`[waitfor]` in its effect \
                     list) — and whatever runs it must be placed on a \
                     `{DEDICATED_QUALIFIER} {POOL_TYPE}` (`thread()`)",
                    member.name.name
                ),
            );
        }
        // [implicit-param] An effect member is an ordinary signature, so it
        // may declare implicit parameters: they resolve at *this* call, with
        // the effect instance's type arguments substituted in, and the
        // handler receives them as trailing arguments like any caller would.
        if !member_implicits.is_empty() {
            self.fill_implicits(
                &member_implicits,
                &member.name.name,
                &subst,
                &generic_set,
                named,
                span,
            );
        }
        substitute_vars(&member_ret, &subst, &generic_set)
    }
}

/// Whether a block always leaves the enclosing construct — every path
/// hits a `return`, `break`, or `continue` — so its state never reaches
/// the code *after* a branching construct [deduce-consume]. Same shape as
/// [fn-must-return]'s walker, with loop exits counted too.
/// The source symbol of a binary operator, for diagnostics [op-no-none].
fn op_symbol(op: BinaryOp) -> &'static str {
    use BinaryOp::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Rem => "%",
        Eq => "==",
        NotEq => "!=",
        Lt => "<",
        Gt => ">",
        LtEq => "<=",
        GtEq => ">=",
        And => "&&",
        Or => "||",
    }
}

/// [op-arith] The operator-numeric types, as (is_float_class, width rank):
/// the integer widths `Int` < `Long` and the float widths `Float` <
/// `Double`, promotion widening within a class only. `Byte` is
/// deliberately **not** operator-numeric: it is an octet rather than a
/// number — unsigned on both backends [byte-value] — and its arithmetic
/// goes through `to_int`/`to_byte`, which is one call and states the
/// wrapping instead of implying it.
fn numeric_class(name: &str) -> Option<(bool, u8)> {
    match name {
        "Int" => Some((false, 0)),
        "Long" => Some((false, 1)),
        "Float" => Some((true, 0)),
        "Double" => Some((true, 1)),
        _ => None,
    }
}

/// The operator-numeric class of a (qualifier-stripped) type.
fn op_numeric(ty: &Ty) -> Option<(bool, u8)> {
    match ty {
        Ty::Named { name, .. } => numeric_class(name),
        _ => None,
    }
}

/// [op-arith] Operands the operator rules stay silent about: a type the
/// checker could not infer [type-unknown-lenient], a diverging expression,
/// and an unconstrained generic parameter (whose leniency here is a
/// documented leftover, matching the equality slice).
fn op_lenient(ty: &Ty) -> bool {
    ty.is_unknown() || matches!(ty, Ty::Nothing | Ty::Var(_))
}

/// [lit-adopt] The numeric type an **unsuffixed** literal adopts from its
/// expected type (user decision 2026-09-14): a plain numeric expectation,
/// reached through qualifiers and through a sole non-`None` union arm
/// (`let x: Long? = 1`). A float literal only adopts within the float
/// class; an integer literal adopts any wider numeric type but never
/// re-adopts `Int` (which would be a no-op).
fn adopted_numeric(expected: Option<&Ty>, suffixed: bool, float_lit: bool) -> Option<&'static str> {
    if suffixed {
        return None;
    }
    let target = numeric_expectation(expected?)?;
    let legal = if float_lit {
        matches!(target, "Float" | "Double")
    } else {
        matches!(target, "Long" | "Float" | "Double")
    };
    legal.then_some(target)
}

/// The single numeric type an expected type names, if it names one.
fn numeric_expectation(expected: &Ty) -> Option<&'static str> {
    match expected.strip_quals() {
        Ty::Named { name, .. } => match name.as_str() {
            "Int" => Some("Int"),
            "Long" => Some("Long"),
            "Float" => Some("Float"),
            "Double" => Some("Double"),
            _ => None,
        },
        Ty::Union(arms) => {
            let mut non_none = arms.iter().filter(|a| !a.is_none_ty());
            match (non_none.next(), non_none.next()) {
                (Some(one), None) => numeric_expectation(one),
                _ => None,
            }
        }
        _ => None,
    }
}

// ================= missing-return analysis [fn-must-return] =================

/// Whether a block always exits the enclosing fn (every path hits a
/// `return`). Conservative: loops never count (they may run zero times),
/// `if` needs an `else`, `when` needs every branch to exit (exhaustiveness
/// over union arms is enforced separately [when-exhaustive]).

/// [iter-protocol] The element type carried by a `next` return type — the
/// `Emitted T` arm of `Emitted T | Finished` — or `None` when the type is not
/// that shape, in which case the function is some other `next` and not a
/// driver.
///
/// Exactly two arms, one tagged `Emitted` and one the fieldless `Finished`:
/// the shape is the protocol, so anything else is deliberately not accepted
/// rather than half-understood.
fn emitted_arm_ty(ret: &Ty) -> Option<(Ty, usize, usize)> {
    let Ty::Union(arms) = ret else { return None };
    if arms.len() != 2 {
        return None;
    }
    let value_arms = ret.value_arms();
    let mut found = None;
    let mut finished = false;
    for (i, arm) in value_arms.iter().enumerate() {
        match arm {
            Ty::Qualified { quals, base } if quals.iter().any(|q| q.name == "Emitted") => {
                found = Some(((**base).clone(), i));
            }
            Ty::Named { name, args } if name == "Finished" && args.is_empty() => {
                finished = true;
            }
            _ => return None,
        }
    }
    let (elem, arm) = found?;
    if finished {
        Some((elem, arm, value_arms.len()))
    } else {
        None
    }
}

/// [proj-anywhere] The span of the first `proj` qualifier in a type, if any —
/// searching arms, arguments and elements in order.
fn first_proj_span(ty: &ast::Type) -> Option<Span> {
    fn in_ref(r: &TypeRef) -> Option<Span> {
        if r.name.name == "proj" {
            return Some(r.span);
        }
        r.args.iter().find_map(first_proj_span)
    }
    match ty {
        ast::Type::Named { qualifiers, base } => {
            qualifiers.iter().find_map(in_ref).or_else(|| in_ref(base))
        }
        ast::Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers
            .iter()
            .find_map(in_ref)
            .or_else(|| first_proj_span(base)),
        ast::Type::Nullable { inner, .. } | ast::Type::Array { elem: inner, .. } => {
            first_proj_span(inner)
        }
        ast::Type::Union { arms, .. } | ast::Type::Tuple { elems: arms, .. } => {
            arms.iter().find_map(first_proj_span)
        }
        _ => None,
    }
}

/// [proj-anywhere] Every `proj` qualifier reference in a type, in order.
/// [proj-infer] The `proj` references that sit inside a type *argument*
/// (`List<proj T>`, `Map<K, proj V>`): borrows the value holds, as opposed
/// to wholesale ones on the value itself.
fn proj_refs_in_type_args(ty: &ast::Type) -> Vec<&TypeRef> {
    fn walk<'a>(ty: &'a ast::Type, inside_arg: bool, out: &mut Vec<&'a TypeRef>) {
        fn in_ref<'a>(r: &'a TypeRef, inside_arg: bool, out: &mut Vec<&'a TypeRef>) {
            if r.name.name == "proj" && inside_arg {
                out.push(r);
            }
            for a in &r.args {
                walk(a, true, out);
            }
        }
        match ty {
            ast::Type::Named { qualifiers, base } => {
                for q in qualifiers {
                    in_ref(q, inside_arg, out);
                }
                in_ref(base, inside_arg, out);
            }
            ast::Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                for q in qualifiers {
                    in_ref(q, inside_arg, out);
                }
                walk(base, inside_arg, out);
            }
            ast::Type::Union { arms, .. } | ast::Type::Tuple { elems: arms, .. } => {
                for a in arms {
                    walk(a, inside_arg, out);
                }
            }
            ast::Type::Nullable { inner, .. } | ast::Type::Array { elem: inner, .. } => {
                walk(inner, inside_arg, out)
            }
            ast::Type::Fn { .. } => {}
        }
    }
    let mut out = Vec::new();
    walk(ty, false, &mut out);
    out
}

/// [proj-type] Why a projection alone kept an argument out of a candidate.
#[derive(Clone, Copy, Debug)]
enum ProjBlock {
    /// A top-level projection into a `Mut` position.
    Mutates,
    /// A top-level projection into a consumed position.
    Consumes,
    /// A projection nested inside the type (a union arm, a type argument)
    /// where the position's type has none there.
    Nested,
}

/// [proj-type] The type with every `proj` removed, at every depth — the
/// "same type on the JVM" reading, for diagnostics.
fn strip_all_proj(ty: &Ty) -> Ty {
    match ty {
        Ty::Qualified { quals, base } => {
            let inner = strip_all_proj(base);
            let kept: Vec<Qual> = quals.iter().filter(|q| q.name != "proj").cloned().collect();
            if kept.is_empty() {
                inner
            } else {
                inner.qualify(kept)
            }
        }
        Ty::Named { name, args } => Ty::Named {
            name: name.clone(),
            args: args.iter().map(strip_all_proj).collect(),
        },
        Ty::Union(arms) => Ty::union_of(arms.iter().map(strip_all_proj).collect()),
        Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(strip_all_proj).collect()),
        Ty::Array(elem) => Ty::Array(Box::new(strip_all_proj(elem))),
        other => other.clone(),
    }
}

/// [proj-readonly] The `Mut` qualifier of a parameter type written with a
/// *top-level* `proj Mut` — the self-contradictory spelling refused at the
/// declaration site. Only the parameter's own qualifier list counts:
/// `proj Mut` nested in a type argument (`Mut List<proj Mut Str>`) is the
/// element type a view really holds, and a nullable/union of the pair
/// (`(proj Mut Str)?`) is not a `Mut` position at the call boundary.
fn top_level_proj_mut(ty: &ast::Type) -> Option<&TypeRef> {
    let quals = match ty {
        ast::Type::Named { qualifiers, .. } => qualifiers,
        ast::Type::QualifiedGroup { qualifiers, .. } => qualifiers,
        _ => return None,
    };
    if quals.iter().any(|q| q.name.name == "proj") {
        quals.iter().find(|q| q.name.name == "Mut")
    } else {
        None
    }
}

fn proj_refs(ty: &ast::Type) -> Vec<&TypeRef> {
    fn walk<'a>(ty: &'a ast::Type, out: &mut Vec<&'a TypeRef>) {
        fn in_ref<'a>(r: &'a TypeRef, out: &mut Vec<&'a TypeRef>) {
            if r.name.name == "proj" {
                out.push(r);
            }
            for a in &r.args {
                walk(a, out);
            }
        }
        match ty {
            ast::Type::Named { qualifiers, base } => {
                for q in qualifiers {
                    in_ref(q, out);
                }
                in_ref(base, out);
            }
            ast::Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                for q in qualifiers {
                    in_ref(q, out);
                }
                walk(base, out);
            }
            ast::Type::Nullable { inner, .. } | ast::Type::Array { elem: inner, .. } => {
                walk(inner, out)
            }
            ast::Type::Union { arms, .. } | ast::Type::Tuple { elems: arms, .. } => {
                for a in arms {
                    walk(a, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(ty, &mut out);
    out
}

/// [proj-anywhere] The qualifier names of the union arms that carry a
/// `proj`, for a union return type; empty for a non-union.
fn proj_arm_qualifiers(ty: &ast::Type) -> Vec<String> {
    let ast::Type::Union { arms, .. } = ty else {
        return Vec::new();
    };
    arms.iter()
        .filter(|arm| first_proj_span(arm).is_some())
        .filter_map(|arm| match arm {
            ast::Type::Named { qualifiers, .. } | ast::Type::QualifiedGroup { qualifiers, .. } => {
                qualifiers
                    .iter()
                    .find(|q| q.name.name != "proj")
                    .map(|q| q.name.name.clone())
            }
            _ => None,
        })
        .collect()
}

/// [proj-anywhere] The indices of a union type's arms that carry `proj`;
/// empty for a non-union.
fn proj_arm_indices(ty: &ast::Type) -> Vec<usize> {
    let ast::Type::Union { arms, .. } = ty else {
        return Vec::new();
    };
    arms.iter()
        .enumerate()
        .filter(|(_, arm)| first_proj_span(arm).is_some())
        .map(|(i, _)| i)
        .collect()
}

/// [yield-proj] Whether a written type mentions a generic parameter by name.
fn type_mentions_generic(ty: &ast::Type, name: &str) -> bool {
    fn in_ref(r: &TypeRef, name: &str) -> bool {
        r.name.name == name || r.args.iter().any(|a| type_mentions_generic(a, name))
    }
    match ty {
        ast::Type::Named { qualifiers, base } => {
            qualifiers.iter().any(|q| in_ref(q, name)) || in_ref(base, name)
        }
        ast::Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(|q| in_ref(q, name)) || type_mentions_generic(base, name),
        ast::Type::Nullable { inner, .. } | ast::Type::Array { elem: inner, .. } => {
            type_mentions_generic(inner, name)
        }
        ast::Type::Union { arms, .. } | ast::Type::Tuple { elems: arms, .. } => {
            arms.iter().any(|a| type_mentions_generic(a, name))
        }
        _ => false,
    }
}

/// [linear-group] Whether an effect **member**'s written clause consumes a
/// parameter of `type_name` — the member-side of the discharge-set
/// predicate. Written only, because a member has no body to infer from
/// ([decl-explicit] makes the clause complete).
fn member_consumes_type(member: &FnDecl, type_name: &str) -> bool {
    member.params.iter().any(|p| {
        let base_matches = match &p.ty {
            ast::Type::Named { base, .. } => base.name.name == type_name,
            ast::Type::QualifiedGroup { base, .. } => matches!(
                base.as_ref(),
                ast::Type::Named { base: b, .. } if b.name.name == type_name
            ),
            _ => false,
        };
        base_matches
            && member.deductions.iter().flatten().any(|d| {
                d.param_name().is_some_and(|n| n.name == p.name.name)
                    && matches!(d.kind, ast::DeductionKind::Moved)
            })
    })
}

/// [is-bind-once] Whether an expression is a **place** — a name, or a
/// field/tuple/index chain over one — and so free of side effects to read
/// twice. Everything else has to be evaluated once into a temporary.
fn is_place_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(_) => true,
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => is_place_expr(base),
        Expr::Index { base, index, .. } => is_place_expr(base) && is_place_expr(index),
        _ => false,
    }
}

/// [is-bind-once] Every `is` **with a binding** inside a condition, with its
/// subject: the shapes whose subject is read twice (once by the test, once by
/// the binding) unless the emitters hoist it.
fn collect_binding_is<'a>(cond: &'a Expr, f: &mut impl FnMut(&'a Expr, Span)) {
    match cond {
        Expr::Is {
            subject,
            binding: Some(_),
            span,
            ..
        } => f(subject, *span),
        Expr::Is { subject, .. } => collect_binding_is(subject, f),
        Expr::Unary { operand, .. } => collect_binding_is(operand, f),
        Expr::Binary { lhs, rhs, .. } => {
            collect_binding_is(lhs, f);
            collect_binding_is(rhs, f);
        }
        _ => {}
    }
}
