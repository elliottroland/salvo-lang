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
use crate::program::{Program, Symbols};
use crate::resolve::{FnKey, ModuleScope, Resolution};
use crate::source::SourceKind;
use crate::types::{is_subtype, Qual, Ty};

/// Table key: (file index, expression span).
pub type Key = (usize, Span);

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
    /// Concrete effect instance registered by each `use` statement (keyed
    /// by the statement span), with handler generics inferred from the
    /// constructor arguments (e.g. `Random<Int>` for
    /// `use CyclicRandom([1,2,3])`).
    pub use_effects: HashMap<Key, Ty>,
    /// The concrete effect instance an effect-member call dispatches
    /// through (keyed by the call span), after generic disambiguation.
    pub effect_calls: HashMap<Key, Ty>,
    /// Concrete effect instances threaded as leading handler arguments for
    /// a call to a fn that declares effect dependencies (keyed by the call
    /// span, in the callee's declaration order).
    pub call_effects: HashMap<Key, Vec<Ty>>,
    /// Wrapper union sizes needed by the program (for `unions.kt`).
    pub union_sizes: BTreeSet<usize>,
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
    /// Type errors, structured for CLI/LSP consumption [diag-structured].
    pub errors: Vec<FileDiagnostic>,
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

    // Round one: no inferred facts yet (strict S1 behavior).
    let mut out = check_once(program, resolution, symbols, None, &mut candidates, &mut claims);
    crate::deduce::infer(program, &mut out, &claims);
    let mut inferred = std::mem::take(&mut out.deductions);
    let mut prev_candidates = candidates.clone();
    let mut prev_claims = claims.clone();

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
            &mut candidates,
            &mut claims,
        );
        // Deductions are a whole-program fact (strictest over the call
        // graph), re-inferred against this round's final call
        // resolutions [deduce-infer].
        crate::deduce::infer(program, &mut out, &claims);
        let stable = out.deductions == inferred
            && candidates == prev_candidates
            && claims == prev_claims;
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
    }
}

/// One checking round; `inferred` carries the previous round's deduction
/// facts for fns without a written list [deduce-consume].
fn check_once<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &Symbols<'p>,
    inferred: Option<&HashMap<FnKey, Vec<crate::deduce::ParamDeduction>>>,
    move_candidates: &mut HashSet<Key>,
    param_claims: &mut HashMap<FnKey, HashSet<String>>,
) -> Checked {
    let mut out = Checked::default();
    out.errors.extend(resolution.errors.iter().cloned());
    for (file_idx, (file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        if file.kind != SourceKind::Language {
            continue;
        }
        let mut checker = Checker {
            scope: &resolution.scopes[file_idx],
            resolution,
            symbols,
            inferred,
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
            own_fn: None,
            own_contract: None,
            own_written: false,
            lambda_ctx: Vec::new(),
            lambda_links: HashMap::new(),
        };
        checker.check_module(ast);
    }
    out
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
}

/// One fate link [fate-link]: the derived variable was bound from (a
/// projection of) the root variable at `bind_span`.
#[derive(Clone, Debug, PartialEq)]
struct FateLink {
    root_id: u32,
    root_name: String,
    bind_span: Span,
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
/// [deduce-consume] [fate-link].
#[derive(Clone, PartialEq)]
struct VarState {
    narrowed: Ty,
    links: Vec<FateLink>,
    poison: Option<Poison>,
    consumed_by: Option<&'static str>,
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
}

/// Narrowing facts derived from a condition.
#[derive(Default, Clone)]
struct CondInfo {
    /// Variable narrowings that hold when the condition is true.
    then_narrows: Vec<(String, Ty)>,
    /// Narrowings that hold when the condition is false.
    else_narrows: Vec<(String, Ty)>,
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
                            Some(crate::deduce::from_written(f, list, |_, _| {}))
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
                Item::Handler(h) => {
                    let saved = self.enter_generics(&h.generics);
                    for f in &h.fns {
                        self.reject_member_effects(f, "handler member functions");
                        self.check_fn(f, &h.params, &h.state);
                    }
                    self.generics = saved;
                }
                Item::Effect(e) => {
                    for f in &e.fns {
                        self.reject_member_effects(f, "effect member functions");
                    }
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
                    for field in &s.fields {
                        if let Some(default) = &field.default {
                            let expected = self.lower_type(&field.ty);
                            self.locals.push(HashMap::new());
                            self.check_expr(default, Some(&expected));
                            self.locals.pop();
                        }
                    }
                    self.generics = saved;
                }
                _ => {}
            }
        }
    }

    /// Checks one function body. `extra_params`/`state` provide handler
    /// constructor parameters and state fields as in-scope variables.
    fn check_fn(&mut self, f: &'p FnDecl, extra_params: &'p [Param], state: &'p [FieldDecl]) {
        let saved_generics = self.enter_generics(&f.generics);
        for p in &f.params {
            self.validate_type(&p.ty);
        }
        if let Some(rt) = &f.return_type {
            self.validate_type(rt);
        }
        if f.constructs.is_some() {
            self.check_constructor_sig(f);
        }
        // Validate the declared effect list (unknown effects, duplicates)
        // and build the fn's effect environment.
        let (fn_effects, can_use) = self.check_effect_list(f);
        let Some(body) = &f.body else {
            self.generics = saved_generics;
            return;
        };
        let saved_env = std::mem::replace(&mut self.effect_env, fn_effects);
        let saved_can_use = std::mem::replace(&mut self.can_use, can_use);
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
                },
            );
        }
        for field in state {
            let ty = self.lower_type(&field.ty);
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
                },
            );
        }
        self.locals.push(top);
        self.check_block_value(body);
        self.locals.pop();
        // [fn-must-return] A fn with a non-`None` return type must return
        // on every path. Yield-based iterator fns are exempt: their body
        // produces elements, not a return value.
        if !self.ret_ty.is_none_ty()
            && !matches!(self.ret_ty, Ty::Unknown)
            && !block_contains_yield(body)
            && !block_always_returns(body)
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
        self.generics = saved_generics;
    }

    // ================= effects =================

    /// Validates a fn's declared effect list and lowers it into the
    /// starting effect environment [effect-fn-deps] [effect-no-dup].
    /// Returns `(env, can_use)`.
    fn check_effect_list(&mut self, f: &'p FnDecl) -> (Vec<Ty>, bool) {
        let mut env: Vec<Ty> = Vec::new();
        let mut can_use = false;
        for eff in f.effects.iter().flatten() {
            match eff {
                EffectRef::Use(_) => can_use = true,
                EffectRef::Effect(r) => {
                    let Some(ty) = self.lower_effect_ref(r) else {
                        continue;
                    };
                    if env.contains(&ty) {
                        self.error(
                            r.span,
                            format!(
                                "duplicate effect `{ty}` in the effect list (two effects \
                                 of the same type must differ in their generic arguments)"
                            ),
                        );
                    } else {
                        env.push(ty);
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
        let (id, args): (&Ident, &'p [Expr]) = match handler {
            Expr::Ident(id) => (id, &[]),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => (id, args.as_slice()),
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
        let param_tys: Vec<Ty> = decl.params.iter().map(|p| self.lower_type(&p.ty)).collect();
        let of_ty = self.lower_type(&decl.of);
        self.generics = saved;

        let has_variadic = decl.params.iter().any(|p| p.variadic);
        if !has_variadic && args.len() != decl.params.len() {
            self.error(
                span,
                format!(
                    "handler `{}` expects {} constructor argument(s), found {}",
                    id.name,
                    decl.params.len(),
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
        for (p, a) in param_tys.iter().zip(&arg_tys) {
            unify(p, a, &mut subst);
        }
        let generic_set: HashSet<String> =
            decl.generics.iter().map(|g| g.name.clone()).collect();
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
            Expr::Field { base, .. } => Self::provenance(base, out),
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
            push(
                FateLink {
                    root_id: var.id,
                    root_name: src.name.clone(),
                    bind_span,
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
        let bind_span = links[0].bind_span;
        if !self.move_candidates.contains(&(self.file_idx, bind_span)) {
            return links;
        }
        // Every live ancestor must be owned; a dead root has nothing
        // left to consume and does not block the move.
        for l in &links {
            let Some(var) = self.var_by_id(l.root_id) else { continue };
            if var.is_param && !self.param_owned(&l.root_name) {
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

    /// Guards consumption of a variable inside a lambda body
    /// [fate-lambda]: a lambda may run any number of times, so it cannot
    /// consume a value it captures from the enclosing scope — each run
    /// after the first would use a moved value. Reports the error and
    /// returns `true` when the consumption must be skipped.
    fn capture_move_violation(&mut self, frame: usize, name: &str, span: Span) -> bool {
        let captured = self
            .lambda_ctx
            .iter()
            .any(|ctx| frame < ctx.boundary);
        if captured {
            self.error(
                span,
                format!(
                    "a lambda cannot consume `{name}`: it is captured from the \
                     enclosing scope and the lambda may run any number of times; \
                     use `copy({name})` inside the lambda"
                ),
            );
        }
        captured
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
            if var.is_param && !self.param_owned(&l.root_name) {
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
    /// declarations [type-with-mut] [fate-move-mode]. Mirrors the Kotlin
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

    /// Runs `f` with the given narrowings applied, restoring afterwards.
    fn with_narrows<T>(
        &mut self,
        narrows: &[(String, Ty)],
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let mut saved: Vec<(String, Ty)> = Vec::new();
        for (name, ty) in narrows {
            if let Some(var) = self.lookup_mut(name) {
                saved.push((name.clone(), var.narrowed.clone()));
                var.narrowed = ty.clone();
            }
        }
        let result = f(self);
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
        result
    }

    /// Resets narrowing to the declared type for every variable assigned
    /// inside `block` (called after branching constructs).
    fn reset_assigned(&mut self, block: &Block) {
        let mut names = HashSet::new();
        collect_assigned(block, &mut names);
        for name in names {
            if let Some(var) = self.lookup_mut(&name) {
                var.narrowed = var.declared.clone();
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
        narrows: &[(String, Ty)],
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
    fn merge_fallthrough(&mut self, fallthrough: &[NarrowSnapshot]) {
        if fallthrough.is_empty() {
            return;
        }
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
                if let Some(var) = self.locals.get_mut(frame_idx).and_then(|f| f.get_mut(name))
                {
                    var.narrowed = joined;
                    var.links = links;
                    var.poison = poison;
                    var.consumed_by = consumed_by;
                }
            }
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
            ast::Type::Fn { params, ret, .. } => Ty::Fn {
                params: params
                    .iter()
                    .map(|p| self.lower_type_subst(p, subst, depth))
                    .collect(),
                ret: Box::new(self.lower_type_subst(ret, subst, depth)),
            },
        }
    }

    fn lower_quals(
        &mut self,
        qualifiers: &[TypeRef],
        subst: &HashMap<String, Ty>,
        depth: usize,
    ) -> Vec<Qual> {
        qualifiers
            .iter()
            .map(|q| Qual {
                name: q.name.name.clone(),
                args: q
                    .args
                    .iter()
                    .map(|a| self.lower_type_subst(a, subst, depth))
                    .collect(),
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

    // ================= qualifier validation =================

    /// Validates every qualifier application inside a written type:
    /// duplicates, `of`-type applicability, and pairwise `with`
    /// compatibility. Called at declaration sites (fn signatures, `let`
    /// annotations, struct fields) so each error is reported once.
    fn validate_type(&mut self, ty: &ast::Type) {
        match ty {
            ast::Type::Named { qualifiers, base } => {
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
                // on declarations that opt in with `with Mut`
                // [struct-mut] [type-with-mut].
                if q.name.name == "Mut" {
                    if !self.has_auto_mut(base) {
                        self.error(
                            q.span,
                            format!(
                                "`Mut` does not apply to `{base}` (its declaration \
                                 does not say `with Mut`)"
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
                // [type-with-mut].
                if qualifiers[i].name.name == "Mut" || qualifiers[j].name.name == "Mut" {
                    continue;
                }
                let (Some(a), Some(b)) = (decls[i], decls[j]) else {
                    continue;
                };
                if a.name.name == b.name.name {
                    continue; // duplicate, already reported
                }
                // Internal qualifiers compose with everything.
                if a.backing == Some(BackingMod::Internal)
                    || b.backing == Some(BackingMod::Internal)
                {
                    continue;
                }
                let compat = a.with.iter().any(|w| w.name.name == b.name.name)
                    || b.with.iter().any(|w| w.name.name == a.name.name);
                if !compat {
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
    /// auto-qualifier: `struct S with Mut` [struct-mut] or
    /// `external type List<T> with Mut` [type-with-mut].
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
            _ => {}
        }
    }
}

fn collect_assigned_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::PostIncrement { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
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
        _ => {}
    }
}

/// Whether `expr` mentions the variable `name` anywhere — any read,
/// projection base, interpolation, nested branch/loop body, or lambda
/// capture. Used for same-call ordering [deduce-same-call]: arguments
/// are evaluated left to right, so mentioning a value that an earlier
/// argument of the same call consumed is a use-after-move.
fn expr_mentions(expr: &Expr, name: &str) -> bool {
    let block_mentions = |block: &Block| -> bool {
        block.stmts.iter().any(|stmt| match stmt {
            Stmt::Let { value, .. } => expr_mentions(value, name),
            Stmt::Assign { target, value, .. } => {
                expr_mentions(target, name) || expr_mentions(value, name)
            }
            Stmt::Return { value: Some(e), .. }
            | Stmt::Break { value: Some(e), .. }
            | Stmt::Yield { value: e, .. } => expr_mentions(e, name),
            Stmt::Use { handler, .. } => expr_mentions(handler, name),
            Stmt::Expr(e) => expr_mentions(e, name),
            _ => false,
        })
    };
    match expr {
        Expr::Ident(id) => id.name == name,
        Expr::Str { parts, .. } => parts.iter().any(|p| match p {
            StrExprPart::Interp(e) => expr_mentions(e, name),
            _ => false,
        }),
        Expr::Field { base, .. } => expr_mentions(base, name),
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
        Expr::Is { subject, .. } => expr_mentions(subject, name),
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
        _ => false,
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
}

/// The outcome of analyzing one `is` expression.
struct IsInfo {
    /// Subject variable name when the subject is a plain identifier.
    subject_name: Option<String>,
    /// Logical type when the check succeeds.
    matched: Ty,
    /// Logical type when the check fails (union subjects only).
    remaining: Option<Ty>,
    binding: Option<Binding>,
}

impl<'p, 'r> Checker<'p, 'r> {
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
        self.locals.pop();
        self.effect_env.truncate(effect_depth);
        (value, tail)
    }

    fn check_stmt(&mut self, stmt: &'p Stmt) -> Ty {
        match stmt {
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
                        // Assigning through a projection mutates the root
                        // variable [fate-poison]; mutating a derived
                        // variable is an error [fate-derived-readonly].
                        let mut sources = Vec::new();
                        Self::provenance(other, &mut sources);
                        let names: Vec<String> =
                            sources.iter().map(|s| s.name.clone()).collect();
                        for name in names {
                            self.fate_mutation(&name, *span);
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
                    // Reassignment: the variable's old value is gone, so
                    // variables derived from it are poisoned
                    // [fate-poison]; the variable itself revives with
                    // fresh links to the new value's sources [fate-link].
                    // An assignment is a bind event: move-mode applies
                    // [fate-move-mode].
                    let links = self.links_for_value(value, *span);
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
                        // Returning a value moves it: a fate-linked
                        // (derived) variable cannot be moved
                        // [fate-derived-readonly]; returning a root
                        // consumes it [deduce-consume] (terminal here,
                        // but visible to unreachable code and to
                        // derived-variable poison [fate-poison]).
                        self.fate_move(v, "return", "a `return`", *span);
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
                // The loop's exit is reachable from every `break`: record
                // this path's flow state so the after-loop merge sees
                // values consumed on break paths [deduce-consume] (an
                // always-exiting branch containing the `break` contributes
                // nothing to the merge *inside* the body, but the loop
                // exit is exactly where its state lands).
                let snap = self.snapshot_narrows();
                if let Some(ctx) = self.loop_stack.last_mut() {
                    ctx.break_states.push(snap);
                }
                Ty::Nothing
            }
            Stmt::Continue { span } => {
                match self.loop_stack.last_mut() {
                    Some(ctx) => ctx.may_skip_value = true,
                    None => self.error(*span, "`continue` outside of a loop"),
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
            Stmt::Expr(e) => self.check_expr(e, None),
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
    /// uses this is the declared type (narrowing does not re-wrap values).
    fn repr_of(&self, expr: &Expr, logical: &Ty) -> Ty {
        if let Expr::Ident(id) = expr {
            if let Some(var) = self.lookup(&id.name) {
                return var.declared.clone();
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
                        self.check_expr(e, None);
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
                    let declared = var.declared.clone();
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
                if let Some(entries) = self.scope.fns.get(id.name.as_str()) {
                    // Passing a function by name.
                    let entry = entries[0];
                    // [fn-ref-table]
                    self.out.fn_refs.insert(self.key(id.span), entry.key);
                    let decl = entry.decl;
                    let saved = self.enter_generics(&decl.generics);
                    let params: Vec<Ty> =
                        decl.params.iter().map(|p| self.lower_type(&p.ty)).collect();
                    let ret = self.fn_return_ty(decl);
                    self.generics = saved;
                    return Ty::Fn {
                        params,
                        ret: Box::new(ret),
                    };
                }
                Ty::Unknown
            }
            Expr::Field { base, field, span } => {
                let base_ty = self.check_expr(base, None);
                // A predicate-qualifier field override refines the type
                // [qual-field-override];
                // the backend casts + asserts at the access site.
                if let Some(override_ty) = self.field_override_ty(&base_ty, &field.name) {
                    self.out
                        .field_casts
                        .insert(self.key(*span), override_ty.clone());
                    return override_ty;
                }
                self.field_ty_or_error(&base_ty, field)
            }
            Expr::Call {
                callee,
                type_args,
                args,
                span,
            } => self.check_call(callee, type_args, args, expected, *span),
            Expr::Index { base, index, .. } => {
                let base_ty = self.check_expr(base, None);
                self.check_expr(index, None);
                match base_ty.strip_quals() {
                    Ty::Array(elem) => (**elem).clone(),
                    _ => Ty::Unknown,
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
                let elem = self.lower_base_ref(elem_type, &empty, 0);
                self.check_expr(size, Some(&Ty::named("Int")));
                let init_ty = Ty::Fn {
                    params: vec![Ty::named("Int")],
                    ret: Box::new(elem.clone()),
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
                        self.check_expr(rhs, None);
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
                        self.check_expr(lhs, None);
                        self.check_expr(rhs, None);
                        Ty::named("Bool")
                    }
                }
            }
            Expr::Is { .. } => {
                self.is_info(expr);
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
                        }
                    }
                    other => {
                        let mut sources = Vec::new();
                        Self::provenance(other, &mut sources);
                        let names: Vec<String> =
                            sources.iter().map(|s| s.name.clone()).collect();
                        for name in names {
                            self.fate_mutation(&name, *span);
                        }
                    }
                }
                ty
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => self.check_if(branches, else_block.as_ref()),
            Expr::When {
                subject,
                branches,
                span,
            } => self.check_when(subject, branches, *span),
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
                self.loop_stack.push(LoopCtx::default());
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
                    self.merge_fallthrough(&states);
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
                let elem = self.iter_elem_ty(&iter_ty);
                // The loop binding is a projection of the iterated
                // collection: it shares the collection's fate [fate-link].
                // It goes through the per-pass bindings channel so every
                // checking pass re-declares it fresh — each iteration
                // binds a new element [deduce-consume].
                let links = self.links_for_value(iterable, iterable.span());
                let bindings =
                    self.pattern_bindings(pattern, elem, links, Some(iterable.span()));
                self.loop_stack.push(LoopCtx::default());
                let (body_ty, body_tail) = self.check_loop_body(body, &[], bindings);
                let mut ctx = self.loop_stack.pop().expect("loop ctx pushed above");
                // Merge break-path states into the after-loop state (the
                // loop-binding frame is popped first, so the snapshots'
                // shared outer frames align) [deduce-consume].
                let break_states = std::mem::take(&mut ctx.break_states);
                if !break_states.is_empty() {
                    let mut states = vec![self.snapshot_narrows()];
                    states.extend(break_states);
                    self.merge_fallthrough(&states);
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

    fn field_ty_or_error(&mut self, base_ty: &Ty, field: &Ident) -> Ty {
        match self.field_ty(base_ty, &field.name) {
            Some(t) => t,
            None => {
                // Only report when the base is a known struct (anything else
                // may be backend interop).
                if let Ty::Named { name, .. } = base_ty.strip_quals() {
                    if self.scope.structs.contains_key(name.as_str()) {
                        let name = name.clone();
                        self.error(
                            field.span,
                            format!("struct `{name}` has no field `{}`", field.name),
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

    fn iter_elem_ty(&mut self, iter_ty: &Ty) -> Ty {
        match iter_ty.strip_quals() {
            Ty::Named { name, args } if name == "Iter" && !args.is_empty() => args[0].clone(),
            Ty::Array(elem) => (**elem).clone(),
            other => {
                // `for x in list` implicitly calls `iter(list)`.
                let other = other.clone();
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
        let (exp_params, exp_ret) = match expected {
            Some(Ty::Fn { params, ret }) => (Some(params.clone()), Some((**ret).clone())),
            _ => (None, None),
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
            param_tys.push(ty);
        }
        let saved_ret = std::mem::replace(
            &mut self.ret_ty,
            exp_ret.clone().unwrap_or(Ty::Unknown),
        );
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
        self.locals.pop();
        let ctx = self.lambda_ctx.pop().expect("lambda ctx pushed above");
        self.finish_lambda_captures(ctx, span);
        Ty::Fn {
            params: param_tys,
            ret: Box::new(ret),
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
                consumed: cap.mutated,
            });
            if cap.mutated {
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
            Expr::Is { .. } => {
                let info = self.is_info(cond);
                self.out
                    .expr_ty
                    .insert(self.key(cond.span()), Ty::named("Bool"));
                let mut out = CondInfo::default();
                if let Some(name) = &info.subject_name {
                    out.then_narrows.push((name.clone(), info.matched.clone()));
                    if let Some(rem) = &info.remaining {
                        out.else_narrows.push((name.clone(), rem.clone()));
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
                self.check_expr(other, None);
                CondInfo::default()
            }
        }
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
        } else if !pat.quals.is_empty() && !matches!(subj_ty, Ty::Union(_)) {
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

        let subject_name = match subject.as_ref() {
            Expr::Ident(id) => Some(id.name.clone()),
            _ => None,
        };
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
            subject_name,
            matched,
            remaining,
            binding,
        }
    }

    /// Splits an `is` check into qualifier names and an optional base type.
    fn parse_check(&mut self, check: &[TypeRef]) -> CheckPat {
        if check.len() == 1 && check[0].name.name == "None" && check[0].args.is_empty() {
            return CheckPat {
                quals: Vec::new(),
                base: None,
                is_none: true,
            };
        }
        let mut quals = Vec::new();
        let mut base = None;
        for (i, r) in check.iter().enumerate() {
            let last = i + 1 == check.len();
            if self.scope.is_qualifier(&r.name.name) || !last {
                quals.push(r.name.name.clone());
            } else {
                let empty = HashMap::new();
                base = Some(self.lower_base_ref(r, &empty, 0));
            }
        }
        CheckPat {
            quals,
            base,
            is_none: false,
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
    ) -> Ty {
        let mut acc_else: Vec<(String, Ty)> = Vec::new();
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
            if !block_always_exits(block) {
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
                if !block_always_exits(block) {
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
        self.merge_fallthrough(&fallthrough);
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
            self.error(
                span,
                format!("`when` requires a union-typed subject (found `{subj_ty}`)"),
            );
            for b in branches {
                self.check_branch_block(&b.body, Vec::new());
            }
            return Ty::Unknown;
        };

        let mut remaining: Vec<Ty> = all_arms.clone();
        let mut branch_tys = Vec::new();
        let mut tails: Vec<Option<TailInfo>> = Vec::new();
        let mut fallthrough: Vec<NarrowSnapshot> = Vec::new();
        for branch in branches {
            let pat = self.parse_check(&branch.check);
            let matched: Vec<Ty> = remaining
                .iter()
                .filter(|arm| self.arm_matches(arm, &pat))
                .cloned()
                .collect();
            if matched.is_empty() {
                self.error(
                    branch.span,
                    "this `when` branch matches no remaining union arm",
                );
            }
            if let Some(test) = self.union_test_for(&repr, &pat) {
                self.out.is_tests.insert(self.key(branch.span), test);
            }
            let narrow_ty = self.mk_union(matched.clone());
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
            let narrows = vec![(subject_id.name.clone(), narrow_ty)];
            // Isolate this branch's consumption effects; only
            // fall-through branches reach the code after the `when`
            // [deduce-consume].
            let entry = self.snapshot_narrows();
            let (ty, tail) = self.with_narrows(&narrows, |c| {
                c.check_branch_block(&branch.body, bindings)
            });
            if !block_always_exits(&branch.body) {
                fallthrough.push(self.snapshot_narrows());
            }
            self.restore_narrows(&entry);
            branch_tys.push(ty);
            tails.push(tail);
            remaining.retain(|arm| !matched.contains(arm));
        }
        self.merge_fallthrough(&fallthrough);
        if !remaining.is_empty() {
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
    fn maybe_coerce(&mut self, span: Span, logical: &Ty, repr: &Ty, expected: &Ty) {
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
            if let Some(bound) = subst.get(g) {
                let bound = bound.clone();
                is_subtype(arg, &bound) || is_subtype(&bound, arg)
            } else {
                subst.insert(g.clone(), arg.clone());
                true
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
            pq.iter().all(|q| arg_quals.contains(&q.name.as_str()))
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
        // `Qual T` can be passed where `T` is expected.
        (_, Ty::Qualified { base, .. }) => unify(param, base, subst),
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
            },
            Ty::Fn {
                params: ap,
                ret: ar,
            },
        ) => {
            pp.len() == ap.len()
                && pp.iter().zip(ap).all(|(p, a)| unify(p, a, subst))
                && unify(pr, ar, subst)
        }
        (Ty::Any, _) => true,
        _ => param == arg,
    }
}

/// Replaces the callee's generic parameters with their bindings (`Unknown`
/// when unbound). Foreign `Var`s (the caller's generics) are left alone.
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
        Ty::Fn { params, ret } => Ty::Fn {
            params: params
                .iter()
                .map(|p| substitute_vars(p, subst, callee_generics))
                .collect(),
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
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        // Dot-notation [fn-dot]: `base.f(args)` == `f(base, args)` when `f`
        // is a known function/define/effect member; otherwise backend
        // interop.
        if let Expr::Field { base, field, .. } = callee {
            let name = field.name.as_str();
            let known = self.scope.effect_members.contains_key(name)
                || self.scope.fns.contains_key(name)
                || self.symbols.define_fns.contains_key(name);
            if known {
                let mut all_args: Vec<&'p Expr> = Vec::with_capacity(args.len() + 1);
                all_args.push(base);
                all_args.extend(args.iter());
                return self.resolve_named_call(
                    name, field.span, type_args, &all_args, expected, span,
                );
            }
            self.check_expr(base, None);
            for a in args {
                self.check_expr(a, None);
            }
            return Ty::Unknown;
        }

        if let Expr::Ident(id) = callee {
            // A local holding a callable (lambda parameter etc.).
            if let Some(var) = self.lookup(&id.name) {
                let vty = var.narrowed.clone();
                if let Ty::Fn { params, ret } = vty {
                    for (i, a) in args.iter().enumerate() {
                        self.check_expr(a, params.get(i));
                    }
                    return *ret;
                }
                for a in args {
                    self.check_expr(a, None);
                }
                return Ty::Unknown;
            }
            let arg_refs: Vec<&'p Expr> = args.iter().collect();
            return self.resolve_named_call(
                &id.name, id.span, type_args, &arg_refs, expected, span,
            );
        }

        // Computed callee.
        let cty = self.check_expr(callee, None);
        if let Ty::Fn { params, ret } = cty {
            for (i, a) in args.iter().enumerate() {
                self.check_expr(a, params.get(i));
            }
            return *ret;
        }
        for a in args {
            self.check_expr(a, None);
        }
        Ty::Unknown
    }

    fn resolve_named_call(
        &mut self,
        name: &str,
        name_span: Span,
        type_args: &'p [ast::Type],
        args: &[&'p Expr],
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        // 1. Effect member call: resolve which effect instance in scope
        // provides it (validating availability and disambiguating generic
        // effects).
        if let Some(&(effect, member)) = self.scope.effect_members.get(name) {
            return self.check_effect_call(effect, member, type_args, args, expected, span);
        }

        // 2. Function overloads [fn-overload] (fn declarations, else define
        // signatures).
        let mut candidates: Vec<(Option<FnKey>, &'p FnDecl)> = self
            .scope
            .fns
            .get(name)
            .map(|entries| {
                entries
                    .iter()
                    .map(|e| (Some(e.key), e.decl))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if candidates.is_empty() {
            if let Some(defs) = self.symbols.define_fns.get(name) {
                candidates = defs.iter().map(|d| (None, &d.sig)).collect();
            }
        }
        if candidates.is_empty() {
            // 3. Handler constructor.
            for a in args {
                self.check_expr(a, None);
            }
            if self.scope.handlers.contains_key(name) {
                return Ty::named(name);
            }
            // Unknown callable (backend interop, struct ctor, ...).
            return Ty::Unknown;
        }

        // Type the arguments once, then match candidates against them.
        let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a, None)).collect();

        struct Viable<'p> {
            key: Option<FnKey>,
            decl: &'p FnDecl,
            subst: HashMap<String, Ty>,
            pairings: Vec<(usize, Ty)>, // (arg index, substituted param type)
            score: i64,
        }
        let mut viable: Vec<Viable<'p>> = Vec::new();
        for (key, decl) in &candidates {
            let fixed: Vec<&Param> = decl.params.iter().filter(|p| !p.variadic).collect();
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
            let mut score = 0i64;
            let mut pairings = Vec::new();
            let mut assignable = true;
            for (i, p) in patterns.iter().enumerate() {
                let sp = substitute_vars(p, &subst, &callee_generics);
                if !is_subtype(&arg_tys[i], &sp) {
                    assignable = false;
                    break;
                }
                if arg_tys[i] == sp {
                    score += 2;
                } else {
                    score += 1;
                }
                // Qualified parameters are more specific.
                score += sp.quals().len() as i64 * 4;
                pairings.push((i, sp));
            }
            if !assignable {
                continue;
            }
            viable.push(Viable {
                key: *key,
                decl,
                subst,
                pairings,
                score,
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
        viable.sort_by_key(|v| -v.score);
        let best = &viable[0];
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
        // [deduce-consume] Deduction lists are a contract, enforced
        // flow-sensitively on bare identifier arguments: parameters *not*
        // kept are consumed (moved) — the variable narrows to `Nothing`
        // and any later use is an error until it is reassigned. Kept
        // parameters shed their removal set (declared − kept qualifiers)
        // from the argument's narrowed type, so e.g.
        // `remove_first(list: NonEmpty Mut List<T>) -> [list: Mut]`
        // leaves the argument un-`NonEmpty` and a second call fails
        // overload resolution. Written lists are enforced directly;
        // unannotated fns are enforced through round one's *inferred*
        // facts (`self.inferred`), so `return list` in a callee consumes
        // the caller's argument just like an explicit `[]`.
        let contract: Option<Vec<crate::deduce::ParamDeduction>> = match &decl.deductions {
            // Shape errors on written lists are reported by the deduce
            // pass; the mapping here is silent.
            Some(list) => Some(crate::deduce::from_written(decl, list, |_, _| {})),
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
            let fixed_count = decl.params.iter().filter(|p| !p.variadic).count();
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
                        let mut sources = Vec::new();
                        Self::provenance(arg, &mut sources);
                        let names: Vec<String> =
                            sources.iter().map(|s| s.name.clone()).collect();
                        for name in names {
                            self.fate_mutation(&name, span);
                        }
                    } else if !d.kept {
                        // A projection in a *moved* position moves data
                        // out of its provenance roots [fate-move-mode].
                        let consumed = self.projection_move(arg, span);
                        consumed_here.extend(consumed);
                    }
                    continue;
                };
                if !d.kept {
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
                // [fate-poison] [fate-derived-readonly].
                let declared_q = crate::deduce::declared_quals(&param.ty);
                if declared_q.iter().any(|q| q == "Mut") {
                    let name = id.name.clone();
                    self.fate_mutation(&name, span);
                }
                let kept: HashSet<&str> = d.quals.iter().map(String::as_str).collect();
                let removed: HashSet<String> = declared_q
                    .into_iter()
                    .filter(|q| !kept.contains(q.as_str()))
                    .collect();
                if removed.is_empty() {
                    continue;
                }
                if let Some(var) = self.lookup_mut(&id.name) {
                    var.narrowed = var.narrowed.clone().remove_quals(&removed);
                }
            }
        }
        // The callee's declared effect dependencies must be satisfiable
        // here: each must match an instance in the caller's effect
        // environment (declared or `use`d).
        self.check_callee_effects(name, decl, &subst, &callee_generics, span);
        let saved = self.enter_generics(&decl.generics);
        let ret = self.fn_return_ty(decl);
        self.generics = saved;
        substitute_vars(&ret, &subst, &callee_generics)
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
        for eff in decl.effects.iter().flatten() {
            let EffectRef::Effect(r) = eff else { continue };
            // Unknown effect names are reported at the callee's own
            // declaration; skip them here.
            if !self.scope.effects.contains_key(r.name.name.as_str()) {
                continue;
            }
            let saved = self.enter_generics(&decl.generics);
            let lowered = {
                let empty = HashMap::new();
                self.lower_base_ref(r, &empty, 0)
            };
            self.generics = saved;
            let want = substitute_vars(&lowered, subst, callee_generics);
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
                Some(instance) => resolved.push(instance),
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
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        // The member's signature, lowered with the effect's generics as
        // `Var`s.
        let saved = self.enter_generics(&effect.generics);
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
        self.generics = saved;
        let generic_set: HashSet<String> =
            effect.generics.iter().map(|g| g.name.clone()).collect();
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

        let subst = resolved.as_ref().map(&subst_for).unwrap_or_default();
        match typed_args {
            Some(arg_tys) => {
                // Args already typed: record coercions against the
                // resolved param types.
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
                            self.check_expr(a, Some(&sp));
                        }
                        None => {
                            self.check_expr(a, None);
                        }
                    }
                }
            }
        }
        if let Some(instance) = &resolved {
            self.out
                .effect_calls
                .insert(self.key(span), instance.clone());
        }
        substitute_vars(&member_ret, &subst, &generic_set)
    }
}

/// Whether a block always leaves the enclosing construct — every path
/// hits a `return`, `break`, or `continue` — so its state never reaches
/// the code *after* a branching construct [deduce-consume]. Same shape as
/// [fn-must-return]'s walker, with loop exits counted too.
fn block_always_exits(block: &Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Expr(e) => expr_always_exits(e),
        _ => false,
    })
}

fn expr_always_exits(expr: &Expr) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            else_block.as_ref().is_some_and(block_always_exits)
                && branches.iter().all(|(_, b)| block_always_exits(b))
        }
        Expr::When { branches, .. } => {
            !branches.is_empty() && branches.iter().all(|b| block_always_exits(&b.body))
        }
        _ => false,
    }
}

// ================= missing-return analysis [fn-must-return] =================

/// Whether a block always exits the enclosing fn (every path hits a
/// `return`). Conservative: loops never count (they may run zero times),
/// `if` needs an `else`, `when` needs every branch to exit (exhaustiveness
/// over union arms is enforced separately [when-exhaustive]).
fn block_always_returns(block: &Block) -> bool {
    block.stmts.iter().any(stmt_always_returns)
}

fn stmt_always_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::Expr(e) => expr_always_returns(e),
        _ => false,
    }
}

fn expr_always_returns(expr: &Expr) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            else_block.as_ref().is_some_and(block_always_returns)
                && branches.iter().all(|(_, b)| block_always_returns(b))
        }
        Expr::When { branches, .. } => {
            !branches.is_empty() && branches.iter().all(|b| block_always_returns(&b.body))
        }
        _ => false,
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
