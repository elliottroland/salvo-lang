//! The type checker.
//!
//! Walks every function body of every language file, inferring a type for
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
use crate::place::{Place, Proj};
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

/// [fn-overload-rank] One candidate that *fits* a call: the declaration, the
/// bindings and coercion targets its match produced, and everything the
/// selection needs — the ranking view of its parameters, and the rung of the
/// visibility ladder it came in on [fn-overload-scope].
struct Viable<'p> {
    key: Option<FnKey>,
    decl: &'p FnDecl,
    subst: HashMap<String, Ty>,
    /// (argument index, substituted parameter type).
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

/// A representation change the emitter must apply to an expression.
#[derive(Clone, Debug)]
pub enum Coercion {
    /// Wrap a plain value into arm `arm` (index into non-`None` arms) of
    /// the `target` union.
    WrapUnion { target: Ty, arm: usize },
    /// Re-wrap a value between two union representations (matching arms by
    /// type equality; unmatched source arms are unreachable at runtime).
    Rewrap { from: Ty, to: Ty },
    /// Wrap a bare value into an optional (`T?`-style) representation
    /// [type-nullable]. Backends whose optionals are physical wrap
    /// (`Some(...)` in Rust); backends with transparent nullability
    /// (Kotlin) treat this as a no-op.
    WrapOption { target: Ty },
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
/// The value arm of a `try` outcome, from `core.result` [try].
pub const OK_QUALIFIER: &str = "Ok";
/// The message arm of a `try` outcome, from `core.throw` [try].
pub const THROWN_QUALIFIER: &str = "Thrown";

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
    /// [iter-protocol] The `next` overload a `for` loop drives, keyed by the
    /// span of its *subject*. Present only when the subject is a **pass** —
    /// something with a `next` — rather than an array, an `Iter<T>`, or a
    /// value with an `iter`. There is no call node in the AST for the
    /// emitters to look at (the driving loop is synthesized), so the choice
    /// of overload has to be handed over here.
    pub for_drivers: HashMap<Key, FnKey>,
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
    pub use_effects: HashMap<Key, Ty>,
    /// Effect instances resolved for a `use`d handler's *dependencies*
    /// [effect-handler-deps], in the handler's declaration order (keyed by
    /// the `use` statement span). Only present when the handler declares
    /// dependencies: the emitters supply these at construction.
    pub use_deps: HashMap<Key, Vec<Ty>>,
    /// The concrete effect instance an effect-member call dispatches
    /// through (keyed by the call span), after generic disambiguation.
    pub effect_calls: HashMap<Key, Ty>,
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
    /// variable's type with a bare `ReadOnly` compiler qualifier and
    /// serves the parameters (roots, binding sites) as on-request detail
    /// (user decision 2026-09-02, progressive disclosure).
    pub fate_reads: HashMap<Key, Vec<FateRead>>,
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
    /// Calls to fns with a derived return (`ReadOnly[from: param]`
    /// [readonly-return]), keyed by the call span: the index of the
    /// argument the result borrows. The checker links the result to that
    /// argument; the Rust backend renders the result as a borrow.
    pub derived_calls: HashMap<Key, usize>,
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
    Resolved { name: String, key: FnKey },
}

/// One fate link of a derived variable, exposed for tooling
/// [fate-link]: the root variable's name and the span of the binding
/// that created the link.
#[derive(Clone, Debug, PartialEq)]
pub struct FateRead {
    pub root: String,
    pub bind_span: Span,
}

/// One captured variable of a lambda [fate-lambda]: the outer local the
/// body mentions, and whether the capture *consumed* it (the body
/// mutates it, so the closure took ownership at creation).
#[derive(Clone, Debug, PartialEq)]
pub struct LambdaCapture {
    pub name: String,
    pub consumed: bool,
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
            can_use: false,
            loop_stack: Vec::new(),
            next_var_id: 0,
            move_candidates,
            param_claims,
            param_mutations,
            own_fn: None,
            own_contract: None,
            own_written: false,
            lambda_ctx: Vec::new(),
            lambda_links: HashMap::new(),
            own_linear_generics: HashSet::new(),
            own_derived_return: None,
            own_implicits: Vec::new(),
            renames: Vec::new(),
            defers: Vec::new(),
            try_stack: Vec::new(),
            in_defer_body: false,
            effect_uses: Vec::new(),
        };
        checker.check_module(ast);
    }
    check_effect_member_names(program, &mut out);
    check_intrinsic_is_std_only(program, &mut out);
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
                Item::Qualifier(q) if q.intrinsic => {
                    ("qualifier", &q.name.name, q.name.span)
                }
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

/// [effect-member-unique] A member name identifies its effect
/// program-wide (`Symbols::effect_of_fn` maps a name to *one* effect), so
/// two effects declaring the same member name make every call to it
/// resolve to whichever was collected last — and there is no syntax to say
/// which one was meant (`emit<Logger>(…)` parses as *member* type
/// arguments). Reported at declaration rather than left to produce a
/// baffling "no handler for effect" downstream (user decision 2026-09-05).
///
/// Walks files and items in source order and reports at the *second*
/// declaration, so the diagnostic is deterministic and fires exactly once
/// per collision — the `Symbols` maps cannot be used for this, since they
/// are last-wins and hash-ordered.
fn check_effect_member_names(program: &Program, out: &mut Checked) {
    // member name -> (effect name, file index, name span)
    let mut seen: HashMap<&str, (&str, usize, Span)> = HashMap::new();
    for (file_idx, (_file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        for item in &ast.items {
            let Item::Effect(e) = item else { continue };
            for f in &e.fns {
                match seen.get(f.name.name.as_str()) {
                    Some((other, _, _)) if *other == e.name.name.as_str() => {
                        out.errors.push(FileDiagnostic::error(
                            file_idx,
                            f.name.span,
                            format!(
                                "effect `{}` already declares a member named `{}`",
                                e.name.name, f.name.name
                            ),
                        ));
                    }
                    Some((other, _, _)) => {
                        out.errors.push(FileDiagnostic::error(
                            file_idx,
                            f.name.span,
                            format!(
                                "effect `{other}` already declares a member named \
                                 `{}`: a member name identifies its effect, and there \
                                 is no syntax to say which effect a call to `{}` means \
                                 — rename one of them",
                                f.name.name, f.name.name
                            ),
                        ));
                    }
                    None => {
                        seen.insert(
                            &f.name.name,
                            (&e.name.name, file_idx, f.name.span),
                        );
                    }
                }
            }
        }
    }
}

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
}

/// One narrowed projection place [flow-place]: the path out of the
/// variable, the type reads of that place have while the fact holds, and
/// the *declared* (physical) type the storage keeps — narrowing never
/// re-wraps storage, so reads unwrap from `declared` to `narrowed` and
/// both the checker (`repr_ty`) and the emitters need it.
#[derive(Clone, PartialEq)]
struct PlaceNarrow {
    path: Vec<Proj>,
    narrowed: Ty,
    declared: Ty,
}

/// One fate link [fate-link]: the derived variable was bound from (a
/// projection of) the root variable at `bind_span`.
#[derive(Clone, Debug, PartialEq)]
struct FateLink {
    root_id: u32,
    root_name: String,
    bind_span: Span,
    /// The link passes through a derived-return call [readonly-return]:
    /// the value is *physically borrowed*, so move-mode can never take
    /// ownership through it [fate-move-mode].
    borrowed: bool,
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
    links: Vec<FateLink>,
    poison: Option<Poison>,
    consumed_by: Option<&'static str>,
    place_narrows: Vec<PlaceNarrow>,
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
    /// Whether the current fn declared the special `use` effect.
    can_use: bool,
    /// Enclosing loops of the code being checked; `break`/`continue`
    /// statements record their value contributions into the innermost
    /// entry [while-value]. Lambda bodies are a barrier.
    loop_stack: Vec<LoopCtx>,
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
    /// The fn currently being checked, when it is a top-level `fn` item
    /// (member fns have no key).
    own_fn: Option<FnKey>,
    /// The current fn's effective deduction contract (written list, else
    /// the previous round's inferred facts): decides whether a parameter
    /// root is *owned* (moved by the contract) for move-mode bindings
    /// [fate-move-mode]. `None` for member fns and in round one.
    own_contract: Option<Vec<crate::deduce::ParamDeduction>>,
    /// Whether the current fn has a *written* deduction list: written
    /// contracts never gain claims — a kept parameter stays kept and
    /// derived moves stay errors [fate-derived-readonly].
    own_written: bool,
    /// Enclosing lambdas of the code being checked [fate-lambda]: the
    /// scope-frame boundary of each (locals below it are *captures*)
    /// plus the captures recorded so far. Innermost last.
    lambda_ctx: Vec<LambdaCtx>,
    /// Fate links carried by lambda *values* [fate-lambda]: a lambda is
    /// derived from the transitively-mutable variables it reads (keyed
    /// by the lambda expression's span, this file only).
    lambda_links: HashMap<Span, Vec<FateLink>>,
    /// Type parameters of the current fn that opted into linearity with
    /// `<T canbe Linear>` [linear-generics]: `T`-typed values are treated
    /// as linear in the body, and callers may instantiate them with
    /// linear types.
    own_linear_generics: HashSet<String>,
    /// The current fn's derived-return parameter
    /// (`-> ReadOnly[from: p] T` [readonly-return]): every returned
    /// value must be derived from `p`, and the return is a *borrow*, not
    /// a move.
    own_derived_return: Option<String>,
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
    /// Deferred blocks registered but not yet run [defer], in
    /// registration order (innermost/latest last). Each is applied at
    /// every exit of the frame it belongs to.
    defers: Vec<PendingDefer>,
    /// Enclosing `try` delimiters [try], innermost last: each collects the
    /// message types of the throws performed in its body.
    try_stack: Vec<TryCtx>,
    /// Whether a *deferred* block's body is being checked [defer-no-escape]:
    /// a throw there would unwind out of an unwind path.
    in_defer_body: bool,
    /// [fn-effects] Effect instances used inside each enclosing lambda
    /// body, innermost last: an un-annotated lambda's effect set is
    /// *inferred* from what its body performs.
    effect_uses: Vec<Vec<Ty>>,
}

/// One enclosing `try` delimiter while its body is checked [try].
struct TryCtx {
    /// `locals.len()` at entry: a throw leaves every frame above this
    /// one, which is the floor for the linear-obligation check
    /// [linear-obligation] and for running deferred blocks [defer].
    entry_depth: usize,
    /// The sites that throw into this delimiter, in first-seen order:
    /// their span and message type. The outcome's `Thrown M` is the union
    /// of those types (user decision 2026-09-04), which is only known once
    /// the body is checked — so each site's wrap is filled in afterwards.
    sites: Vec<(Span, Ty)>,
}

/// One `defer { ... }` awaiting the exits of the block it was written in
/// [defer]. `defer` means *splice at exit*: the body is type-checked
/// once, where the `defer` statement stands, and what running it does to
/// the flow state is applied at each exit of that block — the end of the
/// block, and every `return`/`break`/`continue` that leaves it.
#[derive(Clone)]
struct PendingDefer {
    /// `locals.len()` at the `defer` statement: the body runs at the
    /// exits of that frame.
    depth: usize,
    /// The `defer` statement's span — where its exit-time diagnostics land.
    span: Span,
    /// The flow facts the body was checked against: for every local it
    /// mentions, the narrowed type and the narrowed projections it saw.
    /// A fact that no longer holds at an exit would make the recorded
    /// lowering wrong *there* [backend-never-wrong], so it is an error.
    expects: Vec<(String, Ty, Vec<PlaceNarrow>)>,
    /// Locals the body consumes: the obligation is discharged at each
    /// exit [linear-obligation], and consuming one twice is an error.
    consumes: Vec<String>,
    /// Locals the body only weakens: `Some(ty)` when it invalidated the
    /// narrowing (a mutating call [flow-place-invalidate]), plus a
    /// poison reason when it poisoned a derived variable [fate-poison].
    weakens: Vec<(String, Option<Ty>, Option<Poison>)>,
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
    /// `Once` — callable at most once [once-fn].
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
    fn error_unresolved(&mut self, span: Span, msg: impl Into<String>, name: &str) {
        let imports = self.resolution.import_candidates(name);
        self.out.errors.push(
            FileDiagnostic::error(self.file_idx, span, msg).with_imports(imports),
        );
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
                    self.own_written = f.deductions.is_some();
                    self.own_contract = match &f.deductions {
                        Some(list) => {
                            // Shape errors are reported by the deduce
                            // pass; the mapping here is silent.
                            Some(crate::deduce::from_written(f, list, &HashSet::new(), |_, _| {}))
                        }
                        None => self
                            .inferred
                            .and_then(|table| table.get(&key).cloned()),
                    };
                    self.check_fn(f, &[], &[]);
                    self.own_fn = None;
                    self.own_written = false;
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
                                    "`{}.{}` is a signature, not an implementation: a                                      `params` group declares what a caller must supply,                                      and the default comes from a matching top-level fn",
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
                    for p in &h.params {
                        if p.implicit {
                            self.error(
                                p.span,
                                "a handler constructor cannot take implicit parameters:                                  the instance is built by `use`, which resolves nothing                                  [implicit-fn-only]"
                                    .to_string(),
                            );
                        }
                    }
                    let of_ty = self.lower_type(&h.of);
                    // [throw] There is no handler for throwing: the
                    // delimiter is `try`, and a handler would have to
                    // *resume* the operation, which `Nothing` forbids.
                    if matches!(
                        of_ty.strip_quals(),
                        Ty::Named { name, .. } if name == THROW_EFFECT
                    ) {
                        self.error(
                            h.of.span(),
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
                                h.of.span(),
                                format!(
                                    "`{name}` is a platform effect: the host implements \
                                     it in the target language and supplies it to the \
                                     entry point, so it has no Salvo handler — declare \
                                     an ordinary `effect` if you mean to handle it here"
                                ),
                            );
                        }
                    }
                    for p in &h.params {
                        // [effect-handler-deps] A constructor parameter of
                        // effect type is a *dependency*, not data: the one
                        // position where an effect names something a handler
                        // may hold ([effect-not-data]).
                        match self.handler_dep_effect(&p.ty) {
                            Some(dep) => {
                                // A handler cannot depend on the effect it
                                // implements: registering it would need
                                // itself.
                                if dep.strip_quals() == of_ty.strip_quals() {
                                    self.error(
                                        p.name.span,
                                        format!(
                                            "handler `{}` cannot depend on `{dep}`, \
                                             the effect it implements",
                                            h.name.name
                                        ),
                                    );
                                }
                            }
                            None => self.validate_type(&p.ty),
                        }
                    }
                    for field in &h.state {
                        self.validate_type(&field.ty);
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
                        self.check_fn(f, &h.params, &h.state);
                    }
                    self.generics = saved;
                }
                Item::Effect(e) => {
                    self.check_platform_effect(e);
                    let saved = self.enter_generics(&e.generics);
                    for f in &e.fns {
                        self.reject_member_effects(f, "effect member functions");
                        self.require_explicit_member(f);
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
                    for field in &s.fields {
                        self.validate_type(&field.ty);
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
                    self.generics = saved;
                }
                _ => {}
            }
        }
    }

    /// [canbe-optin] A `canbe` clause names one of the compiler's
    /// permission/obligation qualifiers; user qualifiers are applied in
    /// types, not granted by opt-in.
    ///
    /// `Once` joined the list 2026-09-07 (user decision) so a hand-written
    /// **pass** — a type with a `next` [iter-protocol] — can say that
    /// driving it uses it up. Opting in is the author's call for the same
    /// reason `Linear` is declared rather than applied [linear-canbe]: an
    /// obligation should not attach to someone's type on the strength of a
    /// method name.
    fn validate_auto_quals(&mut self, quals: &[TypeRef]) {
        for q in quals {
            if matches!(q.name.name.as_str(), "Mut" | "Linear" | "Once") {
                continue;
            }
            self.error(
                q.span,
                format!(
                    "only `Mut`, `Linear` and `Once` can be opted into with \
                     `canbe` (found `{}`)",
                    q.name.name
                ),
            );
        }
    }

    /// [once-fn] [canbe-optin] Whether this type opted into `Once` with a
    /// `canbe` clause, which is what makes `Once T` writable for a type of
    /// one's own. Mirrors [`Self::has_auto_mut`].
    fn has_auto_once(&self, base: &Ty) -> bool {
        let Ty::Named { name, .. } = base.strip_quals() else {
            return false;
        };
        let has_once = |quals: &[ast::TypeRef]| quals.iter().any(|q| q.name.name == "Once");
        self.scope
            .structs
            .get(name.as_str())
            .is_some_and(|s| has_once(&s.auto_qualifiers))
            || self
                .scope
                .opaque_types
                .get(name.as_str())
                .is_some_and(|t| has_once(&t.auto_qualifiers))
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
        if f.return_type.is_none() {
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
        if f.deductions.is_none() {
            self.error(
                f.name.span,
                format!(
                    "{kind} `{name}` must declare its deduction list (write \
                     `[]` to move every parameter, or list what it gives \
                     back): with no body to infer from, the signature is the \
                     whole contract"
                ),
            );
        }
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
        let (Ty::Fn { params: wp, contract: wc, effects: we, .. }, Ty::Fn { params: gp, contract: gc, effects: ge, .. }) =
            (want.strip_quals(), got.strip_quals())
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
                     `{got_name}` a deduction list that gives it back \
                     (`-> [{}] ...`), or declare the position as consuming \
                     (`-> []` on the parameter's own signature)",
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
            // The spread's type arguments bind the group's generics.
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
                out.push(ImplicitParam {
                    name: member.name.name.clone(),
                    ty,
                    span: g.span,
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
        let facts: Option<Vec<crate::deduce::ParamDeduction>> = match &decl.deductions {
            Some(list) => Some(crate::deduce::from_written(decl, list, &HashSet::new(), |_, _| {})),
            None => self.inferred.and_then(|table| table.get(&key).cloned()),
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
        let params: Vec<Ty> = member.params.iter().map(|p| self.lower_type(&p.ty)).collect();
        let ret = member
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or_else(Ty::none);
        Ty::Fn {
            params,
            ret: Box::new(ret),
            contract: None,
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
        let implicits = self.out.implicit_params.get(&key).cloned().unwrap_or_default();
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
                    format!("`{callee}` has no implicit parameter named `{}`", arg.name.name),
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
                    format!("`{callee}` has no implicit parameter named `{}`", arg.name.name),
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
        let mut hits: Vec<FnKey> = Vec::new();
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
                    format!(
                        "the `{name}` in scope is `{shown}`, and the position needs `{want}`"
                    ),
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
                hits.push(entry.key);
            } else {
                let reason = self.fn_fit_reason(want, &candidate, name).unwrap_or_else(|| {
                    format!("the `{name}` in scope is `{candidate}`, and the position needs `{want}`")
                });
                note(0, reason, &mut near);
            }
        }
        match hits.len() {
            0 => Err(match near {
                Some((_, reason)) => ImplicitMiss::NearMiss(reason),
                None => ImplicitMiss::Unknown,
            }),
            1 => Ok(hits[0]),
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
            Some(list) => Some(crate::deduce::from_written(
                decl,
                list,
                &HashSet::new(),
                |_, _| {},
            )),
            None => self.inferred.and_then(|table| table.get(&entry.key).cloned()),
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
        Ty::Fn {
            params,
            ret: Box::new(ret),
            contract,
            effects,
        }
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
                let mut modules: Vec<String> =
                    viable.iter().map(|v| v.module.clone()).collect();
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
            let overridden = shadowed.into_iter().find(|&i| {
                crate::types::spec_dominates(&viable[i].rank, &viable[winner].rank)
            });
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

    /// [implicit-infer] Extends a call's progressive substitution with what
    /// its callee's **implicit parameters** determine (user design 2026-09-06,
    /// built with S-Seq).
    ///
    /// `params Iterable<It, T> { fn iter(it: It) -> Iter<T> }` is how Salvo
    /// says "anything iterable": `map<It, T, U>(xs: It, f: (T) -> U,
    /// ?Iterable<It, T>)` binds `It` from its first argument, and `T` is
    /// determined by *which `iter` fills the implicit* — resolving it at
    /// `(List<Int>) -> Iter<T>` finds std's `iter(List<T'>) -> Iter<T'>` and
    /// reads `T = Int` back off it. Without this step the lambda would be
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
        for imp in &implicits {
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
            let entries: Vec<crate::resolve::FnEntry<'p>> =
                self.overloads_of(&imp.name);
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
                    ty_mentions_vars(want, callee_generics)
                        || unify(have, want, &mut binding)
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
        for p in &f.params {
            self.validate_type(&p.ty);
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
        // [readonly-return] `-> ReadOnly[from: p] T`: `p` must be a
        // parameter and must be *kept* — a moved parameter's data needs
        // no annotation (the callee owns it), and a borrow of a moved
        // value could not outlive the call.
        self.own_derived_return = f.derived_return.as_ref().map(|id| id.name.clone());
        if let Some(id) = &f.derived_return {
            let is_param = f.params.iter().any(|p| p.name.name == id.name)
                || extra_params.iter().any(|p| p.name.name == id.name);
            if !is_param {
                self.error(
                    id.span,
                    format!(
                        "`ReadOnly[from: {}]` names no parameter of this function",
                        id.name
                    ),
                );
            } else if self.inferred.is_some() {
                let kept = match &f.deductions {
                    Some(list) => {
                        crate::deduce::from_written(f, list, &HashSet::new(), |_, _| {})
                            .iter()
                            .find(|d| d.param == id.name)
                            .is_some_and(|d| d.kept)
                    }
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
                            "`ReadOnly[from: {}]` requires `{}` to be kept: a \
                             moved parameter is owned by this function, so its \
                             data is returned by ordinary moves",
                            id.name, id.name
                        ),
                    );
                }
            }
        }
        // [linear-generics] Type-parameter opt-ins: `<T canbe Linear>`
        // treats `T`-typed values as linear in this body and admits
        // linear instantiation at call sites. Only `Linear` is
        // supported in a type-parameter `with` clause.
        self.own_linear_generics = f
            .generic_canbe
            .iter()
            .filter(|(_, q)| q.name.name == "Linear")
            .map(|(id, _)| id.name.clone())
            .collect();
        for (_, q) in &f.generic_canbe {
            if q.name.name != "Linear" {
                self.error(
                    q.span,
                    format!(
                        "only `Linear` is supported in a type-parameter `with` \
                         clause (found `{}`)",
                        q.name.name
                    ),
                );
            }
        }
        // Validate the declared effect list (unknown effects, duplicates)
        // and build the fn's effect environment.
        let (mut fn_effects, can_use) = self.check_effect_list(f);
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
        // its handler declares as constructor dependencies, exactly as if
        // the member had declared them — which it may not
        // ([effect-member-no-effects]): the dependency belongs to the
        // implementation, so it is declared once on the handler.
        for p in extra_params {
            if let Some(dep) = self.handler_dep_effect(&p.ty) {
                if !fn_effects.contains(&dep) {
                    fn_effects.push(dep);
                }
            }
        }
        if let Some(key) = self.own_fn {
            self.out.fn_effects.insert(key, fn_effects.clone());
        }
        let Some(body) = &f.body else {
            self.generics = saved_generics;
            return;
        };
        // [iter-effect-free] An iterator fn is *lazy* [fn-iterator]: its
        // body runs in pieces, driven by whoever consumes the elements,
        // long after the call that created it returned. An effect it
        // performed would therefore have to reach a handler that is no
        // longer in scope — on Rust literally so, since a handler arrives
        // as a borrow that cannot outlive the call. So an iterator fn
        // declares no effects at all, `use` included (which would let it
        // register its own handler and perform effects undeclared). The
        // consumer is where effects belong: a `for` loop in an effectful
        // fn can do anything it likes with the elements.
        if block_contains_yield(body) {
            if let Some(span) = f.effects.as_ref().and_then(|list| list.first()).map(|e| match e {
                EffectRef::Use(span) => *span,
                EffectRef::Effect(r) => r.span,
            }) {
                self.error(
                    span,
                    format!(
                        "an iterator function cannot declare effects: `{}` produces its \
                         elements lazily, so its body would run after the call that \
                         supplied the handlers returned — perform the effects where the \
                         elements are consumed, in the `for` loop's own function",
                        f.name.name
                    ),
                );
            }
        }
        let saved_env = std::mem::replace(&mut self.effect_env, fn_effects);
        let saved_can_use = std::mem::replace(&mut self.can_use, can_use);
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
                    is_param: true,
                    for_origin: None,
                    decl_span: p.name.span,
                    lambda_kept: false,
                    is_handler_state: false,
                    widened: None,
            place_narrows: Vec::new(),
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
                    is_param: true,
                    for_origin: None,
                    decl_span: imp.span,
                    lambda_kept: true,
                    is_handler_state: false,
                    widened: None,
                    place_narrows: Vec::new(),
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
                    is_param: true,
                    for_origin: None,
                    decl_span: field.name.span,
                    lambda_kept: false,
                    is_handler_state: true,
                    widened: None,
            place_narrows: Vec::new(),
                },
            );
        }
        self.locals.push(top);
        self.check_block_value(body);
        // [linear-obligation] A moved-in linear parameter must be
        // discharged by the body.
        self.check_linear_frame_drop();
        self.locals.pop();
        // [fn-must-return] A fn with a non-`None` return type must return
        // on every path. Yield-based iterator fns are exempt: their body
        // produces elements, not a return value.
        if !self.ret_ty.is_none_ty()
            && !matches!(self.ret_ty, Ty::Unknown)
            && !block_contains_yield(body)
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
        self.own_implicits = saved_implicits;
        self.generics = saved_generics;
    }

    // ================= effects =================

    /// Validates a fn's declared effect list and lowers it into the
    /// starting effect environment [effect-fn-deps] [effect-no-dup].
    /// Returns `(env, can_use)`.
    fn check_effect_list(&mut self, f: &'p FnDecl) -> (Vec<Ty>, bool) {
        let mut env: Vec<Ty> = Vec::new();
        let mut can_use = false;
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
        (env, can_use)
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
    /// the current scope. Registering two handlers for the same effect
    /// instance is an error [use-no-dup].
    fn check_use(&mut self, handler: &'p Expr, span: Span) {
        let (id, args, written_type_args): (&Ident, &'p [Expr], &'p [ast::Type]) = match handler {
            Expr::Ident(id) => (id, &[], &[]),
            Expr::Call {
                callee,
                args,
                type_args,
                ..
            } => match callee.as_ref() {
                Expr::Ident(id) => (id, args.as_slice(), type_args.as_slice()),
                _ => {
                    self.error(span, "`use` expects a handler name or constructor call");
                    self.check_expr(handler, None);
                    return;
                }
            },
            _ => {
                self.error(span, "`use` expects a handler name or constructor call");
                self.check_expr(handler, None);
                return;
            }
        };
        let Some(decl) = self.scope.handlers.get(id.name.as_str()).copied() else {
            self.error_unresolved(
                id.span,
                format!("unknown handler `{}` in `use`", id.name),
                &id.name,
            );
            for a in args {
                self.check_expr(a, None);
            }
            return;
        };
        let saved = self.enter_generics(&decl.generics);
        // [effect-handler-deps] Split the constructor parameters: those of
        // effect type are *dependencies* the compiler supplies from the
        // enclosing scope, and are not written at the `use` site; the rest
        // are ordinary arguments.
        let deps: Vec<(usize, Ty)> = decl
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| self.handler_dep_effect(&p.ty).map(|d| (i, d)))
            .collect();
        let value_params: Vec<&Param> = decl
            .params
            .iter()
            .enumerate()
            .filter(|(i, _)| !deps.iter().any(|(d, _)| d == i))
            .map(|(_, p)| p)
            .collect();
        let param_tys: Vec<Ty> =
            value_params.iter().map(|p| self.lower_type(&p.ty)).collect();
        let of_ty = self.lower_type(&decl.of);
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
        let generic_set: HashSet<String> =
            decl.generics.iter().map(|g| g.name.clone()).collect();
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
        let concrete = substitute_vars(&of_ty, &subst, &generic_set);
        if self.effect_env.contains(&concrete) {
            self.error(
                span,
                format!("a handler for `{concrete}` is already registered in this scope"),
            );
            return;
        }
        // [effect-handler-deps] Every dependency must already have a handler
        // here: the registration is what wires them together, so this is the
        // point where "who provides it" is decided. Resolved instances are
        // recorded for the emitters, in declaration order.
        let mut resolved_deps: Vec<Ty> = Vec::new();
        for (_, dep) in &deps {
            let want = substitute_vars(dep, &subst, &generic_set);
            let found = self
                .effect_env
                .iter()
                .find(|c| **c == want)
                .cloned()
                .or_else(|| {
                    let compatible: Vec<&Ty> = self
                        .effect_env
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
            self.out
                .use_deps
                .insert(self.key(span), resolved_deps);
        }
        self.out.use_effects.insert(self.key(span), concrete.clone());
        self.effect_env.push(concrete);
    }

    // ================= scopes, locals, narrowing =================

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
                format!("`{}` is already declared (shadowing is not allowed)", name.name),
            );
        }
        // [fate-move-mode] A move-mode binding takes ownership: its
        // ancestors are consumed here and the binding carries no links.
        let links = self.apply_binding_mode(&name.name, links);
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
                is_param,
                for_origin,
                decl_span: name.span,
                lambda_kept: false,
                is_handler_state: false,
                widened: None,
            place_narrows: Vec::new(),
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
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
                Self::provenance(base, out)
            }
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
        // [readonly-return] The result of a derived-return call borrows
        // the annotated argument: it carries that argument's links
        // (dot-notation receivers are argument 0).
        if let Expr::Call { callee, args, span, .. } = value {
            if let Some(&idx) = self.out.derived_calls.get(&(self.file_idx, *span)) {
                let arg: Option<&Expr> = if let Expr::Field { base, .. } = callee.as_ref()
                {
                    if idx == 0 {
                        Some(base)
                    } else {
                        args.get(idx - 1)
                    }
                } else {
                    args.get(idx)
                };
                if let Some(arg) = arg {
                    return self
                        .links_for_value(arg, bind_span)
                        .into_iter()
                        .map(|mut l| {
                            l.borrowed = true;
                            l
                        })
                        .collect();
                }
                return Vec::new();
            }
        }
        // A lambda value is derived from its transitively-mutable read
        // captures [fate-lambda]: binding it carries those links (the
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
                            borrowed: l.borrowed,
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
        let mut sources = Vec::new();
        Self::provenance(value, &mut sources);
        let mut links: Vec<FateLink> = Vec::new();
        let push = |link: FateLink, links: &mut Vec<FateLink>| {
            if !links.iter().any(|l| l.root_id == link.root_id) {
                links.push(link);
            }
        };
        for src in sources {
            let Some(var) = self.lookup(&src.name) else { continue };
            // A source that is itself physically borrowed propagates the
            // flag to its direct link [readonly-return].
            let src_borrowed = var.links.iter().any(|l| l.borrowed);
            push(
                FateLink {
                    root_id: var.id,
                    root_name: src.name.clone(),
                    bind_span,
                    borrowed: src_borrowed,
                },
                &mut links,
            );
            for l in var.links.clone() {
                push(l, &mut links);
            }
        }
        links
    }

    /// Poisons every live variable fate-linked to `root_id` [fate-poison]:
    /// the root was mutated, moved, or reassigned, so derived values may
    /// no longer exist. They narrow to `Nothing` (error at a later use,
    /// revival by reassignment — the standard possibly-consumed
    /// machinery [deduce-consume]).
    fn poison_derived(&mut self, root_id: u32, root_name: &str, event: FateEvent, span: Span) {
        for frame in &mut self.locals {
            for var in frame.values_mut() {
                if var.id != root_id && var.links.iter().any(|l| l.root_id == root_id) {
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
        self.record_move_candidates(links);
        self.record_param_claims(links);
        let root = &links[0].root_name;
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
    /// fn [fate-move-mode] — only when the fn's contract is *inferable*
    /// (no written deduction list): a binding or projection that takes
    /// ownership of data reached through a parameter makes the fn demand
    /// ownership from its callers. Claims are seeded into deduction
    /// inference between the checking rounds.
    fn record_param_claims(&mut self, links: &[FateLink]) {
        let Some(key) = self.own_fn else { return };
        if self.own_written {
            return;
        }
        for l in links {
            let is_param = self
                .var_by_id(l.root_id)
                .is_some_and(|var| var.is_param);
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
            let Some(var) = self.var_by_id(l.root_id) else { continue };
            if (var.is_param && !self.param_owned(&l.root_name)) || var.lambda_kept {
                return links;
            }
        }
        // A lambda cannot consume a capture [fate-lambda]: a move-mode
        // binding inside a lambda whose ancestors live outside it stays
        // borrow-mode, and the move site reports the violation.
        for l in &links {
            let Some(frame) = self.frame_of_id(l.root_id) else { continue };
            if self.lambda_ctx.iter().any(|ctx| frame < ctx.boundary) {
                return links;
            }
        }
        self.record_param_claims(&links);
        for l in &links {
            // Other variables derived from this root lose their value
            // [fate-poison].
            self.poison_derived(l.root_id, &l.root_name, FateEvent::Moved, bind_span);
            let file_idx = self.file_idx;
            let mut loop_origin = None;
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
            if frame < ctx.boundary
                && !ctx.captures.iter().any(|c| c.var_id == var_id)
            {
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
    /// `Once` — callable at most once — so the consumption is legal and
    /// proceeds (the value is owned by the closure from creation on).
    /// Exception: a *linear* capture may not be swallowed (the closure
    /// would inherit an exactly-once obligation, a future feature) —
    /// that reports an error and returns `true` so the consumption is
    /// skipped.
    fn capture_move_violation(&mut self, frame: usize, name: &str, span: Span) -> bool {
        let captured = self
            .lambda_ctx
            .iter()
            .any(|ctx| frame < ctx.boundary);
        if !captured {
            return false;
        }
        let is_linear = self.lookup(name).is_some_and(|v| {
            let mut visited = HashSet::new();
            self.ty_transitively_linear(&v.declared, &mut visited)
        });
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
        // Mark every enclosing lambda `Once` (an outer lambda re-creating
        // an inner consuming closure would re-consume per run).
        let var_id = self.lookup(name).map(|v| v.id);
        if let Some(var_id) = var_id {
            for ctx in &mut self.lambda_ctx {
                if frame < ctx.boundary {
                    if let Some(c) =
                        ctx.captures.iter_mut().find(|c| c.var_id == var_id)
                    {
                        c.moved = true;
                    }
                }
            }
        }
        false
    }

    /// Validates a returned value against the fn's derived-return
    /// annotation [readonly-return]: `None` is fine (no borrow), and
    /// everything else must be derived from the annotated parameter —
    /// its provenance chain must terminate at `from` and nowhere else.
    fn check_derived_return_value(&mut self, value: &Expr, from: &str) {
        if matches!(value, Expr::Ident(id) if id.name == "None") {
            return;
        }
        let from_id = match self.lookup(from) {
            Some(var) => var.id,
            None => return,
        };
        let links = self.links_for_value(value, value.span());
        let ok = !links.is_empty()
            && links.iter().all(|l| {
                l.root_id == from_id
                    || self
                        .var_by_id(l.root_id)
                        .is_some_and(|v| v.links.iter().any(|x| x.root_id == from_id))
            });
        if !ok {
            self.error(
                value.span(),
                format!(
                    "this function returns `ReadOnly[from: {from}]`, so every \
                     returned value must be derived from `{from}` (a projection, \
                     element, or alias of it) or be `None`"
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
            let (kept, effect, mutable) = match contract.and_then(|c| c.get(i)) {
                Some(e) => (e.kept, e.effect.clone(), e.mutable),
                None => (
                    true,
                    QualEffect::KeepAll,
                    declared_q.iter().any(|q| q == "Mut"),
                ),
            };
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
                .map(|v| v.narrowed.quals().iter().any(|q| q.name == "Once"))
                .unwrap_or(false);
            if !kept || arg_once {
                if self.inferred.is_none() {
                    continue;
                }
                let state = self
                    .lookup(&id.name)
                    .map(|var| (var.id, var.links.clone()));
                let Some((var_id, links)) = state else { continue };
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
                self.poison_derived(var_id, &name, FateEvent::Moved, span);
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
                .map(|v| {
                    v.narrowed
                        .quals()
                        .iter()
                        .map(|q| q.name.clone())
                        .collect()
                })
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
        if !self.ty_transitively_mut(&ty, &mut visited) {
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
            let Some(frame) = self.frame_of_id(l.root_id) else { continue };
            let name = l.root_name.clone();
            if self.capture_move_violation(frame, &name, span) {
                return Vec::new();
            }
        }
        for l in &links {
            let Some(var) = self.var_by_id(l.root_id) else { continue };
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
            self.poison_derived(l.root_id, &l.root_name, FateEvent::Moved, span);
            let file_idx = self.file_idx;
            let mut loop_origin = None;
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
                decl.fields.iter().any(|f| self.ast_type_mut(&f.ty, visited))
            }
            Ty::Array(elem) => self.ty_transitively_mut(elem, visited),
            Ty::Tuple(elems) => {
                elems.iter().any(|e| self.ty_transitively_mut(e, visited))
            }
            Ty::Union(arms) => arms.iter().any(|a| self.ty_transitively_mut(a, visited)),
            _ => false,
        }
    }

    /// Whether a type is *linear* — declared `canbe Linear`, or a
    /// composite containing a linear component at any depth (type
    /// argument, array/tuple/union component, or struct field, followed
    /// recursively) [linear-canbe] [linear-composite].
    fn ty_transitively_linear(&self, ty: &Ty, visited: &mut HashSet<String>) -> bool {
        match ty.strip_quals() {
            Ty::Named { name, args } => {
                if self.has_auto_linear(name) {
                    return true;
                }
                if args.iter().any(|a| self.ty_transitively_linear(a, visited)) {
                    return true;
                }
                let Some(decl) = self.scope.structs.get(name.as_str()) else {
                    return false;
                };
                if !visited.insert(name.clone()) {
                    return false;
                }
                decl.fields
                    .iter()
                    .any(|f| self.ast_type_linear(&f.ty, visited))
            }
            Ty::Array(elem) => self.ty_transitively_linear(elem, visited),
            Ty::Tuple(elems) => {
                elems.iter().any(|e| self.ty_transitively_linear(e, visited))
            }
            Ty::Union(arms) => {
                arms.iter().any(|a| self.ty_transitively_linear(a, visited))
            }
            // [linear-generics] An opted-in type parameter is treated as
            // linear inside its fn (worst case), which also makes calls
            // that forward it to other generics require *their* opt-in.
            Ty::Var(name) => self.own_linear_generics.contains(name),
            _ => false,
        }
    }

    /// Whether a declaration opted into linearity with `canbe Linear`
    /// [linear-canbe].
    fn has_auto_linear(&self, name: &str) -> bool {
        let has = |quals: &[ast::TypeRef]| quals.iter().any(|q| q.name.name == "Linear");
        self.scope
            .structs
            .get(name)
            .is_some_and(|s| has(&s.auto_qualifiers))
            || self
                .scope
                .opaque_types
                .get(name)
                .is_some_and(|t| has(&t.auto_qualifiers))
    }

    /// `ty_transitively_linear` over written (AST) types
    /// [linear-composite].
    fn ast_type_linear(&self, ty: &ast::Type, visited: &mut HashSet<String>) -> bool {
        match ty {
            ast::Type::Named { base, .. } => {
                if self.has_auto_linear(&base.name.name) {
                    return true;
                }
                if base.args.iter().any(|a| self.ast_type_linear(a, visited)) {
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
                    .any(|f| self.ast_type_linear(&f.ty, visited))
            }
            ast::Type::QualifiedGroup { base, .. } => self.ast_type_linear(base, visited),
            ast::Type::Union { arms, .. } => {
                arms.iter().any(|a| self.ast_type_linear(a, visited))
            }
            ast::Type::Tuple { elems, .. } => {
                elems.iter().any(|e| self.ast_type_linear(e, visited))
            }
            ast::Type::Array { elem, .. } => self.ast_type_linear(elem, visited),
            ast::Type::Nullable { inner, .. } => self.ast_type_linear(inner, visited),
            ast::Type::Fn { .. } => false,
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
        let mut visited = HashSet::new();
        self.ty_transitively_linear(&var.declared, &mut visited)
    }

    /// Reports every live linear obligation in the top scope frame —
    /// called just before the frame pops [linear-obligation]. The
    /// reported variables are marked consumed so enclosing checks do not
    /// re-report them.
    fn check_linear_frame_drop(&mut self) {
        let Some(frame) = self.locals.last() else { return };
        let owed: Vec<(String, Span)> = frame
            .iter()
            .filter(|(name, var)| self.owes_linear(name, var))
            .map(|(name, var)| (name.clone(), var.decl_span))
            .collect();
        for (name, decl_span) in owed {
            self.error(
                decl_span,
                format!(
                    "`{name}` still owns a linear value when it goes out of \
                     scope; move it onward (pass, return, or store it) or \
                     `discard({name})`"
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
            self.error(
                span,
                format!(
                    "cannot {what} while `{name}` still owns a linear value; \
                     move it onward or `discard({name})` first"
                ),
            );
            if let Some(var) = self.lookup_mut(&name) {
                var.narrowed = Ty::Nothing;
            }
        }
    }

    // ================= deferred blocks [defer] =================

    /// Registers a `defer { ... }` [defer]. The body is type-checked
    /// *here*, in the flow state at the `defer` statement, and the state
    /// is restored afterwards — the code does not run at this point, so
    /// nothing it consumes is consumed yet. What running it does is
    /// summarized for the enclosing block's exits, where
    /// `apply_defer` replays it.
    ///
    /// Checking once (rather than at every exit) keeps one recording of
    /// the body's types for the emitters; the facts the check relied on
    /// are recorded with it and verified at each exit.
    fn check_defer(&mut self, body: &'p Block, span: Span) {
        // [defer-no-escape] The body runs on the way out of the enclosing
        // block: there is no path for it to leave through.
        if let Some((what, at)) = block_defer_escape(body, 0) {
            self.error(
                at,
                format!(
                    "`{what}` is not allowed in a deferred block: the block runs \
                     when the enclosing block exits, so there is nothing to \
                     `{what}` out of"
                ),
            );
        }
        // Facts the body's check is about to rely on, for the locals it
        // mentions.
        let mentioned: Vec<String> = self
            .locals
            .iter()
            .flat_map(|frame| frame.keys().cloned())
            .filter(|name| block_mentions_name(body, name))
            .collect();
        let mut expects: Vec<(String, Ty, Vec<PlaceNarrow>)> = Vec::new();
        for name in mentioned {
            if let Some(var) = self.lookup(&name) {
                expects.push((name, var.narrowed.clone(), var.place_narrows.clone()));
            }
        }
        let entry = self.snapshot_narrows();
        let saved_in_defer = std::mem::replace(&mut self.in_defer_body, true);
        let _ = self.check_branch_block(body, Vec::new());
        self.in_defer_body = saved_in_defer;
        // What the body did to the enclosing state, summarized.
        let mut consumes: Vec<String> = Vec::new();
        let mut weakens: Vec<(String, Option<Ty>, Option<Poison>)> = Vec::new();
        for (frame, saved) in self.locals.iter().zip(&entry) {
            for (name, before) in saved {
                let Some(var) = frame.get(name) else { continue };
                let consumed =
                    matches!(var.narrowed, Ty::Nothing) && !matches!(before.narrowed, Ty::Nothing);
                if consumed {
                    consumes.push(name.clone());
                    continue;
                }
                let narrowing_changed = before.narrowed != var.narrowed;
                let places_changed = before.place_narrows != var.place_narrows;
                let poisoned = before.poison != var.poison;
                if narrowing_changed || places_changed || poisoned {
                    weakens.push((
                        name.clone(),
                        narrowing_changed.then(|| var.narrowed.clone()),
                        var.poison.clone(),
                    ));
                }
            }
        }
        self.restore_narrows(&entry);
        self.defers.push(PendingDefer {
            depth: self.locals.len(),
            span,
            expects,
            consumes,
            weakens,
        });
    }

    /// Replays one deferred block's effect on the current flow state
    /// [defer]: the facts it was checked against must still hold here,
    /// and what it consumes is consumed here.
    fn apply_defer(&mut self, d: &PendingDefer) {
        for (name, expect, expect_places) in &d.expects {
            let Some(var) = self.lookup(name) else { continue };
            if matches!(var.narrowed, Ty::Nothing) {
                let reason = var
                    .consumed_by
                    .map(|by| format!(" (consumed by {by})"))
                    .unwrap_or_default();
                self.error_once(
                    d.span,
                    format!(
                        "the deferred block uses `{name}`, but `{name}` no longer \
                         holds a value at this exit{reason}: the deferred block \
                         runs *after* it was given away"
                    ),
                );
                continue;
            }
            // A fact the body relied on that no longer holds would make
            // the lowering recorded for it wrong here
            // [backend-never-wrong].
            if !is_subtype(&var.narrowed, expect) {
                let found = var.narrowed.clone();
                self.error_once(
                    d.span,
                    format!(
                        "the deferred block was checked where `{name}` is \
                         `{expect}`, but at this exit it is `{found}`: bind the \
                         narrowed value to a local (`if {name} is ... {name}2`) \
                         and defer that instead"
                    ),
                );
                continue;
            }
            let stale = expect_places
                .iter()
                .find(|want| {
                    !var.place_narrows.iter().any(|have| {
                        have.path == want.path && is_subtype(&have.narrowed, &want.narrowed)
                    })
                })
                .cloned();
            if let Some(want) = stale {
                let ty = want.narrowed;
                self.error_once(
                    d.span,
                    format!(
                        "the deferred block was checked where a place in `{name}` \
                         is `{ty}`, but that no longer holds at this exit: bind \
                         the narrowed value to a local and defer that instead"
                    ),
                );
            }
        }
        for name in &d.consumes {
            if let Some(var) = self.lookup_mut(name) {
                if matches!(var.narrowed, Ty::Nothing) {
                    // Already gone — reported above.
                    continue;
                }
                var.narrowed = Ty::Nothing;
                var.consumed_by = Some("a deferred block");
                var.place_narrows.clear();
            }
        }
        for (name, narrowed, poison) in &d.weakens {
            let Some(var) = self.lookup(name) else { continue };
            if matches!(var.narrowed, Ty::Nothing) {
                continue;
            }
            // Only ever *lose* facts here: the exit state may be narrower
            // than the one the summary was computed in (flow narrowing
            // after the `defer`), and re-imposing the recorded type would
            // resurrect a fact this path does not have.
            let widen = narrowed
                .as_ref()
                .filter(|after| is_subtype(&var.narrowed, after))
                .cloned();
            let poison = poison.clone();
            if let Some(var) = self.lookup_mut(name) {
                if let Some(after) = widen {
                    var.narrowed = after;
                }
                var.place_narrows.clear();
                if let Some(p) = poison {
                    var.poison = Some(p);
                    var.narrowed = Ty::Nothing;
                }
            }
        }
    }

    /// Runs — and drops — every deferred block belonging to the frame that
    /// is about to end, latest first [defer].
    fn run_defers_at_frame_end(&mut self) {
        let depth = self.locals.len();
        let mut pending: Vec<PendingDefer> = Vec::new();
        while self.defers.last().is_some_and(|d| d.depth >= depth) {
            pending.push(self.defers.pop().expect("checked by the loop condition"));
        }
        for d in pending {
            self.apply_defer(&d);
        }
    }

    /// Applies the deferred blocks an early exit *leaves* — those in
    /// frames at or above `from_frame` — runs `f` with their effects in
    /// place (so the linear-obligation check sees the obligations they
    /// discharge), then restores the state of the variables they touched:
    /// the frame's normal exit runs the same `defer`s on its own path
    /// [defer].
    fn with_exit_defers<R>(&mut self, from_frame: usize, f: impl FnOnce(&mut Self) -> R) -> R {
        let pending: Vec<PendingDefer> = self
            .defers
            .iter()
            .filter(|d| d.depth > from_frame)
            .rev()
            .cloned()
            .collect();
        if pending.is_empty() {
            return f(self);
        }
        let snap = self.snapshot_narrows();
        let touched: Vec<String> = pending
            .iter()
            .flat_map(|d| {
                d.consumes
                    .iter()
                    .cloned()
                    .chain(d.weakens.iter().map(|(name, _, _)| name.clone()))
            })
            .collect();
        for d in &pending {
            self.apply_defer(d);
        }
        let out = f(self);
        for (frame, saved) in self.locals.iter_mut().zip(&snap) {
            for name in &touched {
                if let (Some(var), Some(state)) = (frame.get_mut(name), saved.get(name)) {
                    var.narrowed = state.narrowed.clone();
                    var.links = state.links.clone();
                    var.poison = state.poison.clone();
                    var.consumed_by = state.consumed_by;
                    var.place_narrows = state.place_narrows.clone();
                }
            }
        }
        out
    }

    /// The frame an early exit unwinds to for the purpose of deferred
    /// blocks: a `return` inside a lambda leaves the lambda, not the
    /// enclosing fn, so outer `defer`s are none of its business
    /// [fate-lambda].
    fn defer_exit_floor(&self) -> usize {
        self.lambda_ctx.last().map(|c| c.boundary).unwrap_or(0)
    }

    /// `error`, unless an identical diagnostic was already reported: a
    /// deferred block is applied at every exit of its block, so one
    /// broken fact can be found twice on the same path [defer].
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
                    "a fn type cannot declare `use`: registering a handler is                      local to a body, so a lambda may `use` exactly when the                      function containing it may"
                        .to_string(),
                ),
                EffectRef::Effect(r) => {
                    if let Some(ty) = self.lower_effect_ref(r) {
                        if !out.contains(&ty) {
                            out.push(ty);
                        }
                    }
                }
            }
        }
        out
    }

    /// The effects a parameter's type contributes to the enclosing fn
    /// [fn-effects]: those declared on a fn type, reached through
    /// qualifiers (`Once () [Console] -> None`) and optional wrappers
    /// (`((s: Str) [Logger] -> Str)?`).
    fn inherited_fn_effects(&mut self, ty: &ast::Type) -> Vec<Ty> {
        match ty {
            ast::Type::Fn { effects, .. } => self.lower_fn_effects(effects.as_deref()),
            ast::Type::QualifiedGroup { base, .. } => self.inherited_fn_effects(base),
            ast::Type::Nullable { inner, .. } => self.inherited_fn_effects(inner),
            ast::Type::Union { arms, .. } => arms
                .iter()
                .flat_map(|a| self.inherited_fn_effects(a))
                .collect(),
            _ => Vec::new(),
        }
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
        if effects.is_empty() {
            return;
        }
        let mut resolved: Vec<Ty> = Vec::new();
        for want in effects {
            let found = self.effect_env.iter().find(|c| *c == want).cloned();
            let found = found.or_else(|| {
                let compatible: Vec<&Ty> = self
                    .effect_env
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
        self.out.call_effects.insert(self.key(span), resolved);
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
    /// between it and its delimiter does not run, so deferred blocks run
    /// and no linear obligation may be live [linear-obligation].
    fn record_throw(&mut self, span: Span, message: Ty, performs: bool) {
        // [defer-no-escape] Unwinding out of an unwind path is a hole
        // neither backend's lowering wants.
        if self.in_defer_body {
            let what = if performs {
                "a `throw`"
            } else {
                "a call that may throw"
            };
            self.error(
                span,
                format!(
                    "{what} is not allowed in a deferred block: the block runs \
                     while the enclosing block is being left, so there is no \
                     delimiter left to throw to"
                ),
            );
            return;
        }
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
                self.with_exit_defers(floor, |c| c.check_linear_throw(floor, span));
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
                self.with_exit_defers(0, |c| c.check_linear_throw(0, span));
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
    /// same walk as an early `return`, with a diagnostic that names
    /// `defer` — the only way to discharge on a path the author does not
    /// write.
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
                     release it in a `defer {{ ... }}` block (which runs on every \
                     path) or move it onward first"
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
        let decl = self.scope.qualifiers.get(name).copied();
        match decl {
            // Written as `Ok Int` / `Thrown Str`, so the qualifier's own
            // type argument stays implicit — exactly as `lower_quals`
            // produces it, which is what makes the outcome type equal to a
            // hand-written `Ok T | Thrown M`.
            Some(_) => Some(Qual {
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
            } => {
                self.block_exits(else_block)
                    && branches.iter().all(|(_, b)| self.block_exits(b))
            }
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
            | Expr::ArrayInit { .. }
            | Expr::Tuple { .. }
            | Expr::StructLit { .. }
            | Expr::Unary { .. }
            | Expr::Binary { .. }
            | Expr::Is { .. }
            | Expr::Widen { .. }
            | Expr::NonNull { .. }
            | Expr::PostIncrement { .. }
            | Expr::Spread { .. }
            | Expr::Scoped { .. }
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
            | Expr::ArrayInit { .. }
            | Expr::Tuple { .. }
            | Expr::StructLit { .. }
            | Expr::Unary { .. }
            | Expr::Binary { .. }
            | Expr::Is { .. }
            | Expr::Widen { .. }
            | Expr::NonNull { .. }
            | Expr::PostIncrement { .. }
            | Expr::Spread { .. }
            | Expr::Scoped { .. }
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
                decl.fields.iter().any(|f| self.ast_type_mut(&f.ty, visited))
            }
            ast::Type::QualifiedGroup { qualifiers, base, .. } => {
                qualifiers.iter().any(|q| q.name.name == "Mut")
                    || self.ast_type_mut(base, visited)
            }
            ast::Type::Union { arms, .. } => {
                arms.iter().any(|a| self.ast_type_mut(a, visited))
            }
            ast::Type::Tuple { elems, .. } => {
                elems.iter().any(|e| self.ast_type_mut(e, visited))
            }
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
        // [deduce-syntax] Record the invalidation against the enclosing
        // fn's parameter: mutation can falsify qualifiers the caller has
        // and this signature never mentions, so such a parameter may not
        // keep "everything else". Recorded for written lists too — that
        // is what the written-list validation checks.
        self.record_param_mutation(name);
        let Some(var) = self.lookup(name) else { return };
        let (id, links) = (var.id, var.links.clone());
        if !links.is_empty() {
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
        self.poison_derived(id, name, FateEvent::Mutated, span);
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
        for name in &names {
            self.fate_mutation(name, span);
        }
        match Place::of_expr(expr) {
            Some(place) => self.invalidate_place_narrows(&place),
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
                if let Some(var) = self.lookup_mut(&n.place.root) {
                    saved.push((n.place.root.clone(), var.narrowed.clone()));
                    var.narrowed = n.narrowed.clone();
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
                                links: var.links.clone(),
                                poison: var.poison.clone(),
                                consumed_by: var.consumed_by,
                                place_narrows: var.place_narrows.clone(),
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
                    var.links = state.links.clone();
                    var.poison = state.poison.clone();
                    var.consumed_by = state.consumed_by;
                    var.place_narrows = state.place_narrows.clone();
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
        let result =
            self.with_narrows(narrows, |c| c.check_branch_block(body, bindings.clone()));
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
                // remaining ones.
                {
                    let consumed = narrowed_states
                        .iter()
                        .filter(|t| matches!(t, Ty::Nothing))
                        .count();
                    if consumed > 0 && consumed < narrowed_states.len() {
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
                                    && {
                                        let mut visited = HashSet::new();
                                        self.ty_transitively_linear(
                                            &var.declared,
                                            &mut visited,
                                        )
                                    }
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
                if let Some(var) = self.locals.get_mut(frame_idx).and_then(|f| f.get_mut(name))
                {
                    var.narrowed = joined;
                    var.links = links;
                    var.poison = poison;
                    var.consumed_by = consumed_by;
                    var.place_narrows = place_narrows;
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
                     paths but not others; consume it on every path, or \
                     `discard({name})` on the paths that keep it"
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
            ast::Type::QualifiedGroup { qualifiers, base, .. } => {
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
                params, ret, effects, ..
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
                        match list.iter().find(|d| d.param.name == id.name) {
                            Some(d) => match &d.kind {
                                ast::DeductionKind::KeepAll => {
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
                            None => (false, QualEffect::Exhaustive(Vec::new())),
                        }
                    }
                    // Named but no list, or unnamed: keeps everything.
                    _ => (true, QualEffect::KeepAll),
                };
                FnParamContract {
                    name: name.as_ref().map(|id| id.name.clone()),
                    kept,
                    effect,
                    mutable,
                }
            })
            .collect();
        // Validate the deduction list references named parameters.
        if let Some(list) = deductions {
            for d in list {
                let known = param_names
                    .iter()
                    .flatten()
                    .any(|n| n.name == d.param.name);
                if !known {
                    self.error(
                        d.param.span,
                        format!(
                            "`{}` names no parameter of this function type \
                             (name the parameter: `({}: ...) -> [...] ...`)",
                            d.param.name, d.param.name
                        ),
                    );
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
            .map(|q| {
                // [lsp-definition] qualifier name -> its declaration.
                self.record_def_ref(q.name.span, &q.name.name);
                Qual {
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
    fn lower_base_ref(
        &mut self,
        base: &TypeRef,
        subst: &HashMap<String, Ty>,
        depth: usize,
    ) -> Ty {
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
        matches!(name, "Mut" | "Linear" | "Once") || self.scope.is_qualifier(name)
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

    /// [effect-handler-deps] The effect a handler constructor parameter
    /// declares as a *dependency*, or `None` when the parameter is ordinary
    /// data. A dependency is a bare effect name (arity-validated here);
    /// a *qualified* effect type falls through to [effect-not-data], since
    /// qualifiers describe values and an effect is not one.
    fn handler_dep_effect(&mut self, ty: &ast::Type) -> Option<Ty> {
        let ast::Type::Named { qualifiers, base } = ty else {
            return None;
        };
        if !qualifiers.is_empty() || !self.scope.effects.contains_key(base.name.name.as_str())
        {
            return None;
        }
        self.lower_effect_ref(base)
    }

    // ================= qualifier validation =================

    /// [effect-not-data] An effect names a *capability*, not a type of
    /// values: it may appear in a fn's effect list, in a handler's `of`
    /// clause, and nowhere else. Using one as a struct field, parameter,
    /// return, or `let` annotation is an error — the value would have to be
    /// a handler instance, which only `use` produces, and neither backend
    /// can render it (Rust emits a bare trait, `E0782`).
    ///
    /// Handler *dependencies* — a handler constructor parameter of effect
    /// type — are the one exception, handled before this runs
    /// ([effect-handler-deps]).
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

    /// Validates a written type: every name resolves [name-resolve], and
    /// every qualifier application is well formed — duplicates,
    /// `of`-type applicability, pairwise `with` compatibility. Called at
    /// declaration sites (fn signatures, `let` annotations, struct
    /// fields, ...) so each error is reported once; type *lowering* runs
    /// repeatedly and stays silent.
    fn validate_type(&mut self, ty: &ast::Type) {
        match ty {
            ast::Type::Named { qualifiers, base } => {
                self.require_name(base, false);
                self.reject_effect_as_data(base);
                for a in &base.args {
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
        let decls: Vec<Option<&'p QualifierDecl>> = qualifiers
            .iter()
            .map(|q| self.scope.qualifiers.get(q.name.name.as_str()).copied())
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
                // [linear-canbe] Linearity is declared, not applied: every
                // value of a `canbe Linear` type is linear, so writing
                // `Linear` at a use site is meaningless (and forgetting
                // it must not silently drop the protection).
                if q.name.name == "Linear" {
                    self.error(
                        q.span,
                        "`Linear` cannot be written in a type: linearity is \
                         declared on the type itself (`canbe Linear`) and applies \
                         to every value of it",
                    );
                    continue;
                }
                // [once-fn] `Once` is the language-level *use*-multiplicity
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
                if q.name.name == "Once" {
                    if !crate::types::once_position(base) && !self.has_auto_once(base) {
                        self.error(
                            q.span,
                            format!(
                                "`Once` applies to function types and `Iter<T>`; a \
                                 type of your own opts in with `canbe Once` (found \
                                 `{base}`)"
                            ),
                        );
                    }
                    continue;
                }
                let Some(decl) = decl else { continue };
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
        self.scope
            .qualifiers
            .get(name)
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
                match (self.scope.qualifiers.get(h.as_str()), self.scope.qualifiers.get(q.as_str()))
                {
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
        if qualifies
            .deductions
            .as_ref()
            .is_some_and(|d| !d.is_empty())
        {
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
        let Some(decl) = self.scope.qualifiers.get(name).copied() else {
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
            self.error(cref.span, "a qualifier constructor must declare a return type");
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
            Stmt::Return { value: Some(e), .. }
            | Stmt::Break { value: Some(e), .. }
            | Stmt::Yield { value: e, .. } => collect_assigned_expr(e, out),
            // [defer] Assignments in a deferred body happen at the block's
            // exits.
            Stmt::Defer { body, .. } => collect_assigned(body, out),
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
        Expr::PostIncrement { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
            collect_assigned_expr(operand, out);
        }
        Expr::If { branches, else_block, .. } => {
            for (c, b) in branches {
                collect_assigned_expr(c, out);
                collect_assigned(b, out);
            }
            if let Some(b) = else_block {
                collect_assigned(b, out);
            }
        }
        Expr::While { cond, body, else_block, .. } => {
            collect_assigned_expr(cond, out);
            collect_assigned(body, out);
            if let Some(b) = else_block {
                collect_assigned(b, out);
            }
        }
        Expr::For { iterable, body, else_block, .. } => {
            collect_assigned_expr(iterable, out);
            collect_assigned(body, out);
            if let Some(b) = else_block {
                collect_assigned(b, out);
            }
        }
        Expr::When { subject, branches, .. } => {
            collect_assigned_expr(subject, out);
            for b in branches {
                collect_assigned(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond { branches, else_block, .. } => {
            for (c, b) in branches {
                collect_assigned_expr(c, out);
                collect_assigned(b, out);
            }
            collect_assigned(else_block, out);
        }
        // [try] The delimiter's body is ordinary code.
        Expr::Try { body, .. } => collect_assigned(body, out),
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
        Expr::ArrayInit { size, init, .. } => {
            collect_assigned_expr(size, out);
            collect_assigned_expr(init, out);
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_assigned_expr(lhs, out);
            collect_assigned_expr(rhs, out);
        }
        Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_assigned_expr(e, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => {
                        collect_assigned_expr(value, out)
                    }
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
        Expr::Scoped { base, .. } => {
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
    if !names
        .iter()
        .all(|n| quals.iter().any(|q| q.name == *n))
    {
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
        Stmt::Return { value: Some(e), .. }
        | Stmt::Break { value: Some(e), .. }
        | Stmt::Yield { value: e, .. } => expr_mentions(e, name),
        Stmt::Use { handler, .. } => expr_mentions(handler, name),
        // [defer] The body's code runs at the block's exits.
        Stmt::Defer { body, .. } => block_mentions_name(body, name),
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
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            expr_mentions(base, name)
        }
        Expr::Call { callee, args, .. } => {
            expr_mentions(callee, name) || args.iter().any(|a| expr_mentions(a, name))
        }
        Expr::Index { base, index, .. } => {
            expr_mentions(base, name) || expr_mentions(index, name)
        }
        Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
            elems.iter().any(|e| expr_mentions(e, name))
        }
        Expr::ArrayInit { size, init, .. } => {
            expr_mentions(size, name) || expr_mentions(init, name)
        }
        Expr::StructLit { fields, .. } => fields.iter().any(|f| match &f.kind {
            StructLitFieldKind::Named { value, .. } => expr_mentions(value, name),
            StructLitFieldKind::Spread(e) => expr_mentions(e, name),
        }),
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::PostIncrement { operand, .. }
        | Expr::Spread { operand, .. } => expr_mentions(operand, name),
        Expr::Binary { lhs, rhs, .. } => {
            expr_mentions(lhs, name) || expr_mentions(rhs, name)
        }
        Expr::Is { subject, .. } | Expr::Widen { subject, .. } => {
            expr_mentions(subject, name)
        }
        Expr::If { branches, else_block, .. } => {
            branches
                .iter()
                .any(|(c, b)| expr_mentions(c, name) || block_mentions(b))
                || else_block.as_ref().is_some_and(|b| block_mentions(b))
        }
        Expr::When { subject, branches, .. } => {
            expr_mentions(subject, name)
                || branches.iter().any(|b| block_mentions(&b.body))
        }
        // [when-condition]
        Expr::WhenCond { branches, else_block, .. } => {
            branches
                .iter()
                .any(|(c, b)| expr_mentions(c, name) || block_mentions(b))
                || block_mentions(else_block)
        }
        Expr::While { cond, body, else_block, .. } => {
            expr_mentions(cond, name)
                || block_mentions(body)
                || else_block.as_ref().is_some_and(|b| block_mentions(b))
        }
        Expr::For { iterable, body, else_block, .. } => {
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
        // [fn-overload-at] The name is a *function*, never a value; only the
        // dot-notation receiver can mention anything.
        Expr::Scoped { base, .. } => {
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
        let subject_place = Place::of_expr(subject)
            .filter(|p| p.narrowable() && self.lookup(&p.root).is_some());
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
                "`^` removes qualifiers, so its right side is qualifier names only                  (use `is` to check a type)"
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
            if let Some(reason) = crate::types::qual_drop_block(q) {
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
                            "`^ {}` matches more than one arm of `{subj_ty}`: widening                              removes a qualifier from a single arm",
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
                            "no arm of `{subj_ty}` carries `{}`, so there is nothing                              for `^` to remove",
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
                                "`{other}` does not carry `{}`, so there is nothing                                  for `^` to remove",
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
            // [defer] A `defer` neither produces nor consumes the block's
            // value: registering one leaves the tail where it was, which
            // is what both emitters do with it too.
            if matches!(stmt, Stmt::Defer { .. }) {
                self.check_stmt(stmt);
                continue;
            }
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
        // [defer] Deferred blocks registered in this frame run as it ends.
        self.run_defers_at_frame_end();
        // [linear-obligation] Nothing linear may die with the scope.
        self.check_linear_frame_drop();
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
            Stmt::Let { pattern, ty, value, span } => {
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
                // Binding from a bare identifier or projection links the
                // new variable(s) to the source: they share fate
                // [fate-link].
                let links = self.links_for_value(value, *span);
                self.declare_pattern(pattern, declared, links);
                Ty::none()
            }
            Stmt::Assign { target, value, span } => {
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
                        let ty = self.check_expr(other, None);
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
                            let has_mut =
                                base_ty.quals().iter().any(|q| q.name == "Mut");
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
                        self.fate_move(
                            value,
                            "store",
                            "a handler state assignment",
                            value.span(),
                        );
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
                    let root = self
                        .lookup(&id.name)
                        .map(|var| (var.id, id.name.clone()));
                    if let Some((root_id, root_name)) = root {
                        self.poison_derived(
                            root_id,
                            &root_name,
                            FateEvent::Reassigned,
                            *span,
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
                    }
                }
                Ty::none()
            }
            Stmt::Return { value, span } => {
                let expected = self.ret_ty.clone();
                match value {
                    Some(v) => {
                        let vty = self.check_expr(v, Some(&expected));
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
                        if let Some(from) = self.own_derived_return.clone() {
                            self.check_derived_return_value(v, &from);
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
                        // [defer] The deferred blocks this exit leaves run
                        // first: what they discharge is discharged here.
                        let floor = self.defer_exit_floor();
                        self.with_exit_defers(floor, |c| {
                            c.check_linear_exit(0, *span, "return")
                        });
                    }
                    None => {
                        if !expected.is_none_ty()
                            && !expected.is_unknown()
                            && !matches!(expected, Ty::Named { ref name, .. } if name == "Iter")
                        {
                            self.error(
                                *span,
                                format!("bare `return` in a function returning `{expected}`"),
                            );
                        }
                        // [linear-obligation] [defer]
                        let floor = self.defer_exit_floor();
                        self.with_exit_defers(floor, |c| {
                            c.check_linear_exit(0, *span, "return")
                        });
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
                // [defer] The loop body's deferred blocks run on the way
                // out, so what they discharge is discharged here.
                // The loop's exit is reachable from every `break`: record
                // this path's flow state so the after-loop merge sees
                // values consumed on break paths [deduce-consume] (an
                // always-exiting branch containing the `break` contributes
                // nothing to the merge *inside* the body, but the loop
                // exit is exactly where its state lands).
                match self.loop_stack.last().map(|c| c.entry_depth) {
                    Some(depth) => {
                        self.with_exit_defers(depth, |c| {
                            c.check_linear_exit(depth, *span, "break");
                            let snap = c.snapshot_narrows();
                            if let Some(ctx) = c.loop_stack.last_mut() {
                                ctx.break_states.push(snap);
                            }
                        });
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
                // die at a `continue`. [defer] Their deferred blocks run
                // on the way out.
                if let Some(depth) = self.loop_stack.last().map(|c| c.entry_depth) {
                    self.with_exit_defers(depth, |c| {
                        c.check_linear_exit(depth, *span, "continue")
                    });
                }
                Ty::Nothing
            }
            Stmt::Yield { value, span } => {
                let elem = match &self.ret_ty {
                    Ty::Named { name, args } if name == "Iter" && !args.is_empty() => {
                        args[0].clone()
                    }
                    _ => Ty::Unknown,
                };
                self.check_expr(value, Some(&elem));
                // Yielding a value moves it into the produced iterator:
                // a derived variable cannot be moved
                // [fate-derived-readonly]; a root is consumed
                // [deduce-consume] — a yield in a loop body consumes
                // anew every iteration, which the loop re-check surfaces
                // on the back edge.
                self.fate_move(value, "yield", "a `yield`", *span);
                Ty::none()
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
            // [defer] Registered here, run at every exit of this block.
            Stmt::Defer { body, span } => {
                self.check_defer(body, *span);
                Ty::none()
            }
            Stmt::Expr(e) => {
                let ty = self.check_expr(e, None);
                // [linear-obligation] A linear value in statement
                // position is dropped on the spot.
                if self.inferred.is_some() {
                    let mut visited = HashSet::new();
                    if self.ty_transitively_linear(&ty, &mut visited) {
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
            Pattern::Tuple { elems, .. } => {
                let elem_tys: Vec<Ty> = match &ty {
                    Ty::Tuple(ts) if ts.len() == elems.len() => ts.clone(),
                    _ => vec![Ty::Unknown; elems.len()],
                };
                elems
                    .iter()
                    .zip(elem_tys)
                    .flat_map(|(p, t)| self.pattern_bindings(p, t, links.clone(), for_origin))
                    .collect()
            }
            Pattern::Struct { fields, .. } => fields
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
                .collect(),
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
    fn fate_move(&mut self, value: &Expr, action: &str, moved_by: &'static str, span: Span) {
        let inner = match value {
            Expr::Spread { operand, .. } => operand.as_ref(),
            _ => value,
        };
        let Expr::Ident(id) = inner else {
            // A projection in a moved position moves data out of its
            // provenance roots [fate-move-mode].
            self.projection_move(inner, span);
            return;
        };
        let Some(var) = self.lookup(&id.name) else { return };
        let (var_id, links) = (var.id, var.links.clone());
        let name = id.name.clone();
        if !links.is_empty() {
            self.error_derived(id.span, action, &name, &links);
            return;
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
        self.poison_derived(var_id, &name, FateEvent::Moved, span);
        if let Some(var) = self.lookup_mut(&id.name) {
            var.narrowed = Ty::Nothing;
            var.consumed_by = Some(moved_by);
        }
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
            Expr::Int { long, .. } => Ty::named(if *long { "Long" } else { "Int" }),
            Expr::Float { single, .. } => {
                Ty::named(if *single { "Float" } else { "Double" })
            }
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
                            }
                        }
                    }
                }
                Ty::named("Str")
            }
            Expr::Ident(id) => {
                if id.name == "None" {
                    return Ty::none();
                }
                if let Some(var) = self.lookup(&id.name) {
                    let narrowed = var.narrowed.clone();
                    // [qual-widen] Inside a `^` branch the physical view is
                    // the widened type: the wrapper the arm test peeled is
                    // materialized by the emitters, so reads (and any
                    // further narrowing) unwrap relative to *it*.
                    let declared = var
                        .widened
                        .clone()
                        .unwrap_or_else(|| var.declared.clone());
                    let poison = var.poison.clone();
                    let consumed_by = var.consumed_by;
                    let links = var.links.clone();
                    let var_id = var.id;
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
                            let mutable =
                                self.ty_transitively_mut(&declared, &mut visited);
                            let name = id.name.clone();
                            self.record_capture(frame, &name, var_id, mutable);
                        }
                    }
                    if narrowed != declared {
                        self.out.repr_ty.insert(self.key(id.span), declared);
                    }
                    // Expose the fate links of derived-variable reads for
                    // tooling (`ReadOnly` presentation) [fate-link].
                    if !links.is_empty() {
                        let reads: Vec<FateRead> = links
                            .iter()
                            .map(|l| FateRead {
                                root: l.root_name.clone(),
                                bind_span: l.bind_span,
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
                let base_ty = self.check_expr(base, None);
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
                        let (narrowed, declared) =
                            (entry.narrowed.clone(), entry.declared.clone());
                        if narrowed != declared {
                            self.out.repr_ty.insert(self.key(*span), declared);
                        }
                        return narrowed;
                    }
                }
                self.field_ty_or_error(&base_ty, field)
            }
            Expr::TupleIndex { base, index, span } => {
                let base_ty = self.check_expr(base, None);
                // [flow-place] A narrowed element place reads at its
                // narrowed type, exactly as a field does.
                if let Some(place) = Place::of_expr(expr) {
                    if let Some(entry) = self.place_narrow_entry(&place) {
                        let (narrowed, declared) =
                            (entry.narrowed.clone(), entry.declared.clone());
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
                let base_ty = self.check_expr(base, None);
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
            Expr::ArrayLit { elems, .. } => {
                let expected_elem = match expected.map(|t| t.strip_quals()) {
                    Some(Ty::Array(e)) => Some((**e).clone()),
                    _ => None,
                };
                let mut tys = Vec::new();
                for e in elems {
                    tys.push(self.check_expr(e, expected_elem.as_ref()));
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
                Ty::Array(Box::new(elem))
            }
            Expr::ArrayInit {
                elem_type,
                size,
                init,
                ..
            } => {
                let empty = HashMap::new();
                self.require_name(elem_type, false);
                let elem = self.lower_base_ref(elem_type, &empty, 0);
                self.check_expr(size, Some(&Ty::named("Int")));
                // [type-array] The initializer is inlined at the
                // construction site, so it performs whatever the enclosing
                // scope allows: its expected type declares the effects in
                // scope rather than none [fn-effects].
                let init_ty = Ty::Fn {
                    params: vec![Ty::named("Int")],
                    ret: Box::new(elem.clone()),
                    contract: None,
                    effects: self.effect_env.clone(),
                };
                self.check_expr(init, Some(&init_ty));
                Ty::Array(Box::new(elem))
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
                for f in fields {
                    match &f.kind {
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
                let t = self.check_expr(operand, None);
                match op {
                    UnaryOp::Neg => t,
                    UnaryOp::Not => Ty::named("Bool"),
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
                        if l.is_unknown() {
                            Ty::Unknown
                        } else {
                            l.strip_quals().clone()
                        }
                    }
                    And | Or => {
                        // Value-position boolean: no narrowing propagation.
                        self.check_expr(lhs, None);
                        self.check_expr(rhs, None);
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
            Expr::Widen { subject, quals, span } => {
                self.widen_info(subject, quals, *span);
                Ty::named("Bool")
            }
            Expr::NonNull { operand, .. } => {
                let t = self.check_expr(operand, None);
                t.without_none()
            }
            Expr::PostIncrement { operand, span } => {
                let ty = self.check_expr(operand, None);
                // `i++` rebinds a whole variable (revival: severs its own
                // links, poisons variables derived from it [fate-poison]);
                // through a projection it mutates the root
                // [fate-derived-readonly].
                match operand.as_ref() {
                    Expr::Ident(id) => {
                        let root = self
                            .lookup(&id.name)
                            .map(|var| (var.id, id.name.clone()));
                        if let Some((root_id, root_name)) = root {
                            self.poison_derived(
                                root_id,
                                &root_name,
                                FateEvent::Reassigned,
                                *span,
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
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                // [while-value] The loop is an expression: its value is the
                // body's tail (last iteration), a `break value`, or the
                // `else` tail when the loop never ran.
                let info = self.analyze_cond(cond);
                self.loop_stack.push(LoopCtx {
                    entry_depth: self.locals.len(),
                    ..LoopCtx::default()
                });
                let (body_ty, body_tail) = self.check_loop_body(
                    body,
                    &info.then_narrows.clone(),
                    info.bindings.clone(),
                );
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
                let elem = self.iter_elem_ty(&iter_ty, iterable.span());
                // [once-fn] Driving a **pass** consumes it: `Once Iter<T>`
                // is a position in a sequence, not a recipe, so a second
                // `for` over the same value is the ordinary consumed-use
                // error and the elements are owned by the loop rather than
                // derived from a value that is still alive. A plain
                // `Iter<T>` is a factory: `for` mints a pass from it and
                // leaves it usable, exactly as before.
                let drives_pass = iter_ty.quals().iter().any(|q| q.name == "Once");
                let links = if drives_pass {
                    self.fate_move(iterable, "iterate", "a `for` loop", iterable.span());
                    Vec::new()
                } else {
                    // The loop binding is a projection of the iterated
                    // collection: it shares the collection's fate
                    // [fate-link]. It goes through the per-pass bindings
                    // channel so every checking pass re-declares it fresh —
                    // each iteration binds a new element [deduce-consume].
                    self.links_for_value(iterable, iterable.span())
                };
                let bindings =
                    self.pattern_bindings(pattern, elem, links, Some(iterable.span()));
                self.loop_stack.push(LoopCtx {
                    entry_depth: self.locals.len(),
                    ..LoopCtx::default()
                });
                let (body_ty, body_tail) = self.check_loop_body(body, &[], bindings);
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
            Expr::Lambda { params, body, span } => {
                self.check_lambda(params, body, *span, expected)
            }
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
            let Some(decl) = self.scope.qualifiers.get(qual.name.as_str()).copied() else {
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
            subst.insert(
                g.name.clone(),
                args.get(i).cloned().unwrap_or(Ty::Unknown),
            );
        }
        Some(self.lower_type_subst(&field.ty, &subst, 0))
    }

    /// [iter-protocol] The element type of a **pass**: a value some `next`
    /// accepts, returning `Emitted T | Finished`. Records the overload the
    /// emitters have to drive (there is no call node in the AST for them to
    /// resolve, since the driving loop is synthesized) and reports a value
    /// that has a `next` but does not declare itself `Once`.
    ///
    /// `stripped` selects the overload — a subject's own `Once`/`Mut` say
    /// nothing about which `next` fits — while `full` is what carries the
    /// qualifiers the rule is about.
    fn pass_elem_ty(&mut self, stripped: &Ty, full: &Ty, span: Span) -> Option<Ty> {
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
            let Some(elem) = emitted_arm_ty(&ret) else {
                // A `next` of some other shape is not the protocol; keep
                // looking, and let `iter` have its turn.
                continue;
            };
            // [once-fn] A pass must say it is one. `next` says the value can
            // be advanced; `Once` says advancing uses it up, and only the
            // author knows whether that is true — inferring it from a method
            // name would attach an obligation to someone's type on the
            // strength of a name (user decision 2026-09-07).
            if !full.quals().iter().any(|q| q.name == "Once") {
                self.error(
                    span,
                    format!(
                        "`{stripped}` has a `next` but is not a pass: driving it \
                         uses it up, so it has to be declared `Once {stripped}` — \
                         annotate the return type of the function that builds it"
                    ),
                );
            }
            self.out.for_drivers.insert(self.key(span), entry.key);
            return Some(elem);
        }
        None
    }

    fn iter_elem_ty(&mut self, iter_ty: &Ty, span: Span) -> Ty {
        match iter_ty.strip_quals() {
            Ty::Named { name, args } if name == "Iter" && !args.is_empty() => args[0].clone(),
            Ty::Array(elem) => (**elem).clone(),
            other => {
                let other = other.clone();
                // [iter-protocol] A **pass** — anything with a `next` — is
                // driven directly, and is looked for *before* `iter`: a type
                // that has both is already a position in a sequence, so
                // minting a second pass from it would be wrong. This is the
                // manual half of the iterator story (`zip`, `merge`), which
                // `yield` cannot express.
                if let Some(elem) = self.pass_elem_ty(&other, iter_ty, span) {
                    return elem;
                }
                // `for x in list` implicitly calls `iter(list)`.
                if let Some(entries) = self.scope.fns.get("iter") {
                    let entries: Vec<crate::resolve::FnEntry<'p>> = entries.clone();
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
                        if unify(&pt, &other, &mut subst) {
                            let callee_generics: HashSet<String> =
                                decl.generics.iter().map(|g| g.name.clone()).collect();
                            let ret = substitute_vars(&ret, &subst, &callee_generics);
                            if let Ty::Named { name, args } = ret.strip_quals() {
                                if name == "Iter" && !args.is_empty() {
                                    return args[0].clone();
                                }
                            }
                        }
                    }
                }
                // [iter-resolve] Nothing makes this value iterable: no
                // `Iter`, no array, and no `iter` overload accepts it. An
                // un-inferred value stays lenient
                // [type-unknown-lenient]; everything else is an error
                // (user decision 2026-09-03).
                if !other.is_unknown() && !matches!(other, Ty::Nothing) {
                    self.error(
                        span,
                        format!(
                            "`{other}` is not iterable: `for` takes an array, an \
                             `Iter<T>`, or a value some `iter` function accepts"
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
            let ty = p
                .ty
                .as_ref()
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
        let saved_ret = std::mem::replace(
            &mut self.ret_ty,
            exp_ret.clone().unwrap_or(Ty::Unknown),
        );
        // [fn-effects] The body performs the effects the fn *type* declares
        // — the call site supplies them, so nothing is captured. With no
        // expected type the set is *inferred* from the body, which is what
        // `effect_uses` collects; the enclosing environment stands in for
        // the duration so the calls resolve.
        let saved_effects = match &exp_effects {
            Some(effects) => {
                Some(std::mem::replace(&mut self.effect_env, effects.clone()))
            }
            None => None,
        };
        self.effect_uses.push(Vec::new());
        // A lambda body is a loop barrier: `break`/`continue` inside it
        // never bind a loop enclosing the lambda expression.
        let saved_loops = std::mem::take(&mut self.loop_stack);
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
        self.ret_ty = saved_ret;
        // [linear-obligation] Lambda parameters are owned by the body:
        // a linear one must be discharged before the body ends.
        // [defer] A lambda block body is its own frame: its deferred
        // blocks run as it ends.
        self.run_defers_at_frame_end();
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
        // once: its type gains `Once`, so it only fits `Once` fn
        // positions and calling it consumes it.
        if consumes_captures {
            fn_ty.qualify(vec![Qual {
                name: "Once".to_string(),
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
            });
            if cap.mutated {
                // [linear-lambda] A mutated capture would move the
                // obligation into the closure: forbidden.
                let is_linear = self.var_by_id(cap.var_id).is_some_and(|v| {
                    let mut visited = HashSet::new();
                    self.ty_transitively_linear(&v.declared, &mut visited)
                });
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
                let is_kept_param = self
                    .var_by_id(cap.var_id)
                    .is_some_and(|v| v.is_param)
                    && !self.param_owned(&cap.name);
                if is_kept_param && self.own_written {
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
                if self
                    .var_by_id(cap.var_id)
                    .is_some_and(|v| v.is_param)
                {
                    if let Some(key) = self.own_fn {
                        if !self.own_written {
                            self.param_claims
                                .entry(key)
                                .or_default()
                                .insert(cap.name.clone());
                        }
                    }
                }
                // Consume the original: the closure owns the value now.
                self.poison_derived(cap.var_id, &cap.name, FateEvent::Moved, span);
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
            if cap.mutable {
                // The closure shares fate with its mutable read-captures:
                // link it (transitively through the capture's own links).
                if !links.iter().any(|l| l.root_id == cap.var_id) {
                    links.push(FateLink {
                        root_id: cap.var_id,
                        root_name: cap.name.clone(),
                        bind_span: span,
                        borrowed: false,
                    });
                }
                if let Some(var) = self.var_by_id(cap.var_id) {
                    for l in var.links.clone() {
                        if !links.iter().any(|e| e.root_id == l.root_id) {
                            links.push(l);
                        }
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
        let annotated = ty
            .map(|t| self.lower_type(t))
            .or_else(|| expected.cloned());
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
            Ty::Named { name, args } => {
                match self.scope.structs.get(name.as_str()).copied() {
                    Some(decl) => {
                        let mut subst = HashMap::new();
                        for (i, g) in decl.generics.iter().enumerate() {
                            subst.insert(
                                g.name.clone(),
                                args.get(i).cloned().unwrap_or(Ty::Unknown),
                            );
                        }
                        (Some(decl), subst)
                    }
                    None => (None, HashMap::new()),
                }
            }
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
                            if !is_subtype(&vty, &fty) {
                                self.error(
                                    value.span(),
                                    format!(
                                        "field `{}` expects `{fty}`, found `{vty}`",
                                        name.name
                                    ),
                                );
                            }
                            provided.insert(&name.name);
                        }
                        None => {
                            self.check_expr(value, None);
                            self.error(
                                name.span,
                                format!(
                                    "struct `{}` has no field `{}`",
                                    decl.name.name, name.name
                                ),
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
    fn analyze_cond(&mut self, cond: &'p Expr) -> CondInfo {
        match cond {
            // [qual-widen] The dual of `is`: same test, generalized type.
            Expr::Widen { subject, quals, span } => {
                let info = self.widen_info(subject, quals, *span);
                self.out
                    .expr_ty
                    .insert(self.key(*span), Ty::named("Bool"));
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
        let is_predicate = !pat.is_none
            && !pat.quals.is_empty()
            && !matches!(subj_ty, Ty::Union(_))
            && !subj_ty.is_unknown()
            && pat.quals.iter().all(|q| {
                self.scope
                    .qualifiers
                    .get(q.as_str())
                    .is_some_and(|d| d.has_body)
            });
        if is_predicate {
            for q in &pat.quals {
                let decl = self.scope.qualifiers[q.as_str()];
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
            for q in &pat.quals {
                let decl = self.scope.qualifiers[q.as_str()];
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
            let constructive = pat.quals.iter().find(|q| {
                self.scope
                    .qualifiers
                    .get(q.as_str())
                    .is_some_and(|d| !d.has_body)
            });
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
                quals.push(r.name.name.clone());
            } else if is_type {
                let empty = HashMap::new();
                base = Some(self.lower_base_ref(r, &empty, 0));
            } else {
                // Both namespaces are admissible here, so neither wording
                // fits on its own.
                let name = name.to_string();
                self.error_unresolved(
                    r.span,
                    format!("unknown type or qualifier `{name}`"),
                    &name,
                );
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
        for (cond, block) in branches {
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
                let (ty, tail) =
                    self.with_narrows(&acc_else.clone(), |c| c.check_branch_block(block, Vec::new()));
                if !self.block_exits(block) {
                    fallthrough.push(self.snapshot_narrows());
                }
                self.restore_narrows(&entry);
                branch_tys.push(ty);
                tails.push(tail);
            }
            None => {
                // No `else`: the no-branch-taken path falls through with
                // the current state.
                fallthrough.push(self.snapshot_narrows());
                branch_tys.push(Ty::none());
                tails.push(None);
            }
        }
        self.merge_fallthrough(&fallthrough, span);
        let join = self.mk_union(branch_tys);
        for tail in tails.into_iter().flatten() {
            self.maybe_coerce(tail.span, &tail.logical, &tail.repr, &join);
        }
        for (_, block) in branches {
            self.reset_assigned(block);
        }
        if let Some(block) = else_block {
            self.reset_assigned(block);
        }
        join
    }

    /// Checks a `when` expression: union-typed variable subject
    /// [when-union-subject], sequential arm consumption and exhaustiveness
    /// [when-exhaustive], branch values unioning into the result
    /// [when-value].
    fn check_when(
        &mut self,
        subject: &'p Expr,
        branches: &'p [WhenBranch],
        span: Span,
    ) -> Ty {
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
                        "a `^` branch removes qualifiers, so its head is qualifier                          names only (use `is` to match a type)"
                            .to_string(),
                    );
                    ok = false;
                }
                for q in &pat.quals {
                    if let Some(reason) = crate::types::qual_drop_block(q) {
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
                                "this arm does not carry `{}`, so there is nothing for                                  `^` to remove",
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
                self.out
                    .expr_ty
                    .insert(self.key(b.span), narrow_ty.clone());
                // The `when` binding aliases the subject [fate-link].
                let links = self.links_for_value(subject, b.span);
                bindings.push((b.clone(), narrow_ty.clone(), links, None));
            }
            // [qual-widen] A `^` branch sees the subject at the widened
            // type, so nested narrowing composes on that rather than on the
            // storage the arm test peeled it out of.
            let declared = if branch.widen
                && self.out.is_tests.contains_key(&self.key(branch.span))
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
            let (ty, tail) = self.with_narrows(&narrows, |c| {
                c.check_branch_block(&branch.body, bindings)
            });
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
        for tail in body_tail
            .into_iter()
            .chain(ctx.breaks)
            .chain(else_tail)
        {
            self.maybe_coerce(tail.span, &tail.logical, &tail.repr, &join);
        }
        join
    }

    /// Records the representation change (if any) needed to use a value of
    /// (`logical`, `repr`) where `expected` is required. Arm matching is
    /// positional over the declared type's non-`None` arms
    /// [union-arm-identity]; a qualified union group tries wrapping as
    /// a whole arm before stripping its qualifiers [qual-group].
    ///
    /// [str-drop-mut] A `Mut` qualifier dropped on the way is recorded too,
    /// wrapping whatever the representation math decided.
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
                self.out.coerce.insert(key, Coercion::DropMut { from, then });
                return;
            }
            other => other.map(Box::new),
        };
        self.out.coerce.insert(key, Coercion::DropMut { from, then });
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
                self.out.union_sizes.insert(effective.value_arms().len().max(2));
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
                    self.out.coerce.insert(
                        self.key(span),
                        Coercion::WrapUnion {
                            target: expected.clone(),
                            arm: matches[0],
                        },
                    );
                }
                0 => self.error(
                    span,
                    format!("no arm of `{expected}` accepts a value of type `{logical}`"),
                ),
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
            // [once-fn] A `Once` requirement is satisfied by any fn
            // (inverted subtyping: plain fns may be treated as
            // once-callable).
            pq.iter()
                .all(|q| q.name == "Once" || arg_quals.contains(&q.name.as_str()))
                && unify(pb, arg.strip_quals(), subst)
        }
        // A union parameter tries each arm against the *intact* argument —
        // this must precede the qualifier-stripping arm below, or a
        // qualified argument (`Ok Str`) could never match a union's
        // qualified arm (`Ok Str | Err Str`) [type-union].
        (Ty::Union(parms), _) => match arg {
            Ty::Union(aarms) => aarms
                .iter()
                .all(|a| parms.iter().any(|p| unify(p, a, &mut subst.clone()) && unify(p, a, subst))),
            _ => parms.iter().any(|p| unify(p, arg, subst)),
        },
        // `Qual T` can be passed where `T` is expected — except `Once`,
        // which may never be dropped [once-fn].
        (_, Ty::Qualified { quals, base }) => {
            !quals.iter().any(|q| q.name == "Once") && unify(param, base, subst)
        }
        (Ty::Named { name: pn, args: pa }, Ty::Named { name: an, args: aa }) => {
            pn == an
                && pa.len() == aa.len()
                && pa.iter().zip(aa).all(|(p, a)| unify(p, a, subst))
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
fn substitute_known(
    ty: &Ty,
    subst: &HashMap<String, Ty>,
    callee_generics: &HashSet<String>,
) -> Ty {
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

fn substitute_vars(ty: &Ty, subst: &HashMap<String, Ty>, callee_generics: &HashSet<String>) -> Ty {
    match ty {
        Ty::Var(g) if callee_generics.contains(g) => {
            subst.get(g).cloned().unwrap_or(Ty::Unknown)
        }
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
        if let Expr::Field { base, field, .. } = callee {
            let name = field.name.as_str();
            let known =
                self.scope.effect_members.contains_key(name) || self.has_callable(name);
            if known {
                let mut all_args: Vec<&'p Expr> = Vec::with_capacity(args.len() + 1);
                all_args.push(base);
                all_args.extend(args.iter());
                return self.resolve_named_call(
                    name, field.span, type_args, &all_args, named, expected, None,
                    span,
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

        // [fn-overload-at] `f@core.list(x)` / `xs.f@core.list(y)`: the module
        // whose overload is meant. Written *instead of* letting scope
        // precedence decide, so it goes straight to overload selection — a
        // local of the same name does not shadow it, which is what makes `@`
        // the way out of a shadowed name.
        if let Expr::Scoped {
            base,
            name,
            module,
            ..
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
                // A consumed callable (e.g. a `Once` fn already called
                // [once-fn]) reports the standard consumed-use error.
                if matches!(vty, Ty::Nothing) {
                    self.check_expr(callee, None);
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Ty::Unknown;
                }
                let once = vty.quals().iter().any(|q| q.name == "Once");
                if let Ty::Fn { params, ret, contract, effects } = vty.strip_quals().clone() {
                    // [fn-effects] The call supplies the value's effects.
                    self.check_fn_value_effects(&effects, span);
                    for (i, a) in args.iter().enumerate() {
                        self.check_expr(a, params.get(i));
                    }
                    // [fn-contract] Apply the fn value's contract to the
                    // arguments (default: keeps everything).
                    let arg_refs: Vec<&'p Expr> = args.iter().collect();
                    self.apply_fn_value_contract(
                        &arg_refs,
                        &params,
                        contract.as_deref(),
                        span,
                    );
                    // [once-fn] Calling a `Once` fn consumes it: the
                    // existing consumption machinery then enforces the
                    // multiplicity (second call, loop back edge, branch
                    // merges) for free.
                    if once {
                        let state = self
                            .lookup(&id.name)
                            .map(|var| (var.id, var.links.clone()));
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
                                );
                                if let Some(var) = self.lookup_mut(&id.name) {
                                    var.narrowed = Ty::Nothing;
                                    var.consumed_by = Some(
                                        "a call (a `Once` function is callable \
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
            return self.resolve_named_call(
                &id.name, id.span, type_args, &arg_refs, named, expected, None, span,
            );
        }

        // Computed callee (a `Once`-typed temporary is called at most
        // once by construction [once-fn]).
        let cty = self.check_expr(callee, None);
        if let Ty::Fn { params, ret, contract: _, effects } = cty.strip_quals().clone() {
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
            self.error(span, format!("this expression is not callable: its type is `{cty}`"));
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
        // 1. Effect member call: resolve which effect instance in scope
        // provides it (validating availability and disambiguating generic
        // effects).
        if let Some(&(effect, member)) = self.scope.effect_members.get(name) {
            // [lsp-definition] members have no `FnKey`; the def-site table
            // carries their declaration span.
            self.record_def_ref(name_span, name);
            return self
                .check_effect_call(effect, member, type_args, args, named, expected, span);
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
        let mut pool: Vec<LeadCandidate> = self.lead_pool(&candidates, args.len(), type_args);
        let mut arg_tys: Vec<Ty> = Vec::with_capacity(args.len());
        for (i, a) in args.iter().enumerate() {
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
                // (`takes_ints(mutable_list())`). A pattern still
                // mentioning the callee's generics must not: coercion
                // would be recorded against an unsubstituted `T`.
                Expr::Call { .. } => {
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
        for entry in &candidates {
            let (key, decl) = (entry.key, entry.decl);
            // [implicit-param] Implicits are never passed positionally, so
            // they take no part in arity or ranking.
            let fixed: Vec<&Param> =
                decl.params.iter().filter(|p| !p.variadic && !p.implicit).collect();
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
            for (i, p) in patterns.iter().enumerate() {
                let sp = substitute_vars(p, &subst, &callee_generics);
                if !is_subtype(&arg_tys[i], &sp) {
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
        // Record argument coercions against the selected parameter types.
        for (i, pt) in &best.pairings {
            let logical = arg_tys[*i].clone();
            let repr = self.repr_of(args[*i], &logical);
            self.maybe_coerce(args[*i].span(), &logical, &repr, pt);
        }
        let callee_generics: HashSet<String> =
            best.decl.generics.iter().map(|g| g.name.clone()).collect();
        let subst = best.subst.clone();
        let decl = best.decl;
        let best_key = best.key;
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
                .filter(|(_, q)| q.name.name == "Linear")
                .map(|(id, _)| id.name.as_str())
                .collect();
            for (var_name, ty) in &subst {
                if !callee_generics.contains(var_name) {
                    continue;
                }
                // [linear-generics] `<T canbe Linear>` admits linear
                // instantiation: the callee's body honors the obligation
                // (for an `intrinsic fn`, the backend's lowering does).
                if opted.contains(var_name.as_str()) {
                    continue;
                }
                let mut visited = HashSet::new();
                if self.ty_transitively_linear(ty, &mut visited) {
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
                                 not declare `<{var_name} canbe Linear>`, so it does \
                                 not honor the use obligation"
                            ),
                        );
                    }
                    break;
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
            Some(list) => Some(crate::deduce::from_written(
                                    decl,
                                    list,
                                    &HashSet::new(),
                                    |_, _| {},
                                )),
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
                        let mut visited = HashSet::new();
                        if self.ty_transitively_linear(&ty, &mut visited) {
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
                let Some(d) = contract.iter().find(|d| d.param == param.name.name)
                else {
                    continue;
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
                // [once-fn] A `Once` fn value escapes when passed as an
                // argument: fn-value ownership is otherwise untracked,
                // so the pass consumes it regardless of the callee's
                // contract (conservative; relaxable with fn-type
                // contracts).
                let arg_once = self
                    .lookup(&id.name)
                    .map(|v| v.narrowed.quals().iter().any(|q| q.name == "Once"))
                    .unwrap_or(false);
                if !d.kept || arg_once {
                    // Moved. A fate-linked (derived) variable cannot be
                    // moved [fate-derived-readonly]; moving a root
                    // poisons the variables derived from it
                    // [fate-poison].
                    let state = self
                        .lookup(&id.name)
                        .map(|var| (var.id, var.links.clone()));
                    let Some((var_id, links)) = state else { continue };
                    if !links.is_empty() {
                        let name = id.name.clone();
                        self.error_derived(id.span, "move", &name, &links);
                        continue;
                    }
                    let name = id.name.clone();
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
                    self.poison_derived(var_id, &name, FateEvent::Moved, span);
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
                    .map(|v| {
                        v.narrowed
                            .quals()
                            .iter()
                            .map(|q| q.name.clone())
                            .collect()
                    })
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
        if let Some(from) = &decl.derived_return {
            if let Some(idx) = decl
                .params
                .iter()
                .position(|p| p.name.name == from.name)
            {
                self.out.derived_calls.insert(self.key(span), idx);
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
                    if callee_generics.contains(&g)
                        && !ty.is_unknown()
                        && !subst.contains_key(&g)
                    {
                        subst.insert(g, ty);
                    }
                }
            }
        }
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
            let found = self.effect_env.iter().find(|c| **c == want).cloned();
            let found = found.or_else(|| {
                let compatible: Vec<&Ty> = self
                    .effect_env
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

    /// Checks a call to an effect member fn. The providing effect instance
    /// must be available (declared in the caller's effect list or `use`d)
    /// [effect-available]; generic effects are disambiguated by explicit
    /// type arguments, the argument types, and the expected type, in that
    /// order [effect-disambiguation].
    fn check_effect_call(
        &mut self,
        effect: &'p EffectDecl,
        member: &'p FnDecl,
        type_args: &'p [ast::Type],
        args: &[&'p Expr],
        named: &'p [ast::NamedArg],
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
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

        // Instances of this effect currently available.
        let candidates: Vec<Ty> = self
            .effect_env
            .iter()
            .filter(|t| matches!(t, Ty::Named { name, .. } if *name == effect.name.name))
            .cloned()
            .collect();

        // Argument types when they had to be computed for disambiguation
        // (in that case coercions are recorded afterwards; otherwise the
        // args are checked below with the resolved param types expected).
        let mut typed_args: Option<Vec<Ty>> = None;

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
            self.error(
                span,
                format!(
                    "no handler for effect `{}` in scope (declare it in the \
                     function's effect list or `use` a handler)",
                    effect.name.name
                ),
            );
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
            let facts =
                crate::deduce::from_written(member, list, &HashSet::new(), |_, _| {});
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
                    }
                })
                .collect();
            self.apply_call_contract(args, &member_params, Some(&contract), span);
        }
        if let Some(instance) = &resolved {
            // [fn-effects] Feeds an enclosing lambda's inferred set.
            let instance = instance.clone();
            self.note_effect_use(&instance);
            self.out.effect_calls.insert(self.key(span), instance);
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
fn emitted_arm_ty(ret: &Ty) -> Option<Ty> {
    let Ty::Union(arms) = ret else { return None };
    if arms.len() != 2 {
        return None;
    }
    let mut elem = None;
    let mut finished = false;
    for arm in arms {
        match arm {
            Ty::Qualified { quals, base } if quals.iter().any(|q| q.name == "Emitted") => {
                elem = Some((**base).clone());
            }
            Ty::Named { name, args } if name == "Finished" && args.is_empty() => {
                finished = true;
            }
            _ => return None,
        }
    }
    if finished {
        elem
    } else {
        None
    }
}

/// Whether the fn body contains a `yield` statement — iterator fns build
/// their `Iter` return value from yields and are exempt from
/// [fn-must-return]. Lambdas are their own fns and are not descended into.
fn block_contains_yield(block: &Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Yield { .. } => true,
        Stmt::Expr(e) => expr_contains_yield(e),
        _ => false,
    })
}

fn expr_contains_yield(expr: &Expr) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            branches.iter().any(|(_, b)| block_contains_yield(b))
                || else_block.as_ref().is_some_and(block_contains_yield)
        }
        Expr::When { branches, .. } => {
            branches.iter().any(|b| block_contains_yield(&b.body))
        }
        Expr::While {
            body, else_block, ..
        }
        | Expr::For {
            body, else_block, ..
        } => block_contains_yield(body) || else_block.as_ref().is_some_and(block_contains_yield),
        _ => false,
    }
}

// ================= deferred blocks [defer] =================

/// The first control-flow statement in a deferred block that would leave
/// it [defer-no-escape]: `return`/`yield` anywhere, and
/// `break`/`continue` outside a loop *inside* the body (a loop written in
/// the body owns its own). Lambda bodies are their own functions and are
/// not descended into.
fn block_defer_escape(block: &Block, loop_depth: usize) -> Option<(&'static str, Span)> {
    block.stmts.iter().find_map(|stmt| match stmt {
        Stmt::Return { span, .. } => Some(("return", *span)),
        Stmt::Yield { span, .. } => Some(("yield", *span)),
        Stmt::Break { span, .. } if loop_depth == 0 => Some(("break", *span)),
        Stmt::Continue { span } if loop_depth == 0 => Some(("continue", *span)),
        Stmt::Break { .. } | Stmt::Continue { .. } => None,
        // [fn-rename] A declaration, not control flow.
        Stmt::Rename(_) => None,
        Stmt::Let { value, .. } => expr_defer_escape(value, loop_depth),
        Stmt::Assign { target, value, .. } => expr_defer_escape(target, loop_depth)
            .or_else(|| expr_defer_escape(value, loop_depth)),
        Stmt::Use { handler, .. } => expr_defer_escape(handler, loop_depth),
        // A nested `defer` runs at the end of *this* body: its own
        // registration checks it.
        Stmt::Defer { .. } => None,
        Stmt::Expr(e) => expr_defer_escape(e, loop_depth),
    })
}

fn expr_defer_escape(expr: &Expr, loop_depth: usize) -> Option<(&'static str, Span)> {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => branches
            .iter()
            .find_map(|(c, b)| {
                expr_defer_escape(c, loop_depth).or_else(|| block_defer_escape(b, loop_depth))
            })
            .or_else(|| {
                else_block
                    .as_ref()
                    .and_then(|b| block_defer_escape(b, loop_depth))
            }),
        Expr::When {
            subject, branches, ..
        } => expr_defer_escape(subject, loop_depth).or_else(|| {
            branches
                .iter()
                .find_map(|b| block_defer_escape(&b.body, loop_depth))
        }),
        // [when-condition] The subject-less form is a condition chain, so
        // it escapes exactly where an `if`/`elif`/`else` chain does.
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => branches
            .iter()
            .find_map(|(c, b)| {
                expr_defer_escape(c, loop_depth).or_else(|| block_defer_escape(b, loop_depth))
            })
            .or_else(|| block_defer_escape(else_block, loop_depth)),
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => expr_defer_escape(cond, loop_depth)
            .or_else(|| block_defer_escape(body, loop_depth + 1))
            .or_else(|| {
                else_block
                    .as_ref()
                    .and_then(|b| block_defer_escape(b, loop_depth))
            }),
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => expr_defer_escape(iterable, loop_depth)
            .or_else(|| block_defer_escape(body, loop_depth + 1))
            .or_else(|| {
                else_block
                    .as_ref()
                    .and_then(|b| block_defer_escape(b, loop_depth))
            }),
        // A lambda is its own function: its `return` is not an escape.
        Expr::Lambda { .. } => None,
        // [try] A `try` body is ordinary code; a `return` inside it still
        // leaves the deferred block.
        Expr::Try { body, .. } => block_defer_escape(body, loop_depth),
        Expr::Call { callee, args, .. } => expr_defer_escape(callee, loop_depth)
            .or_else(|| args.iter().find_map(|a| expr_defer_escape(a, loop_depth))),
        // [fn-overload-at]
        Expr::Scoped { base, .. } => base
            .as_ref()
            .and_then(|b| expr_defer_escape(b, loop_depth)),
        Expr::Field { base, .. }
        | Expr::TupleIndex { base, .. }
        | Expr::Unary { operand: base, .. }
        | Expr::NonNull { operand: base, .. }
        | Expr::PostIncrement { operand: base, .. }
        | Expr::Spread { operand: base, .. }
        | Expr::Is { subject: base, .. }
        | Expr::Widen { subject: base, .. } => expr_defer_escape(base, loop_depth),
        Expr::Index { base, index, .. } => expr_defer_escape(base, loop_depth)
            .or_else(|| expr_defer_escape(index, loop_depth)),
        Expr::Binary { lhs, rhs, .. } => expr_defer_escape(lhs, loop_depth)
            .or_else(|| expr_defer_escape(rhs, loop_depth)),
        Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => elems
            .iter()
            .find_map(|e| expr_defer_escape(e, loop_depth)),
        Expr::ArrayInit { size, init, .. } => expr_defer_escape(size, loop_depth)
            .or_else(|| expr_defer_escape(init, loop_depth)),
        Expr::StructLit { fields, .. } => fields.iter().find_map(|f| match &f.kind {
            StructLitFieldKind::Named { value, .. } => expr_defer_escape(value, loop_depth),
            StructLitFieldKind::Spread(e) => expr_defer_escape(e, loop_depth),
        }),
        Expr::Str { parts, .. } => parts.iter().find_map(|p| match p {
            StrExprPart::Interp(e) => expr_defer_escape(e, loop_depth),
            StrExprPart::Text(_) => None,
        }),
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Ident(_)
        | Expr::Error { .. } => None,
    }
}
