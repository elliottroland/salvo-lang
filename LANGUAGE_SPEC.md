# Salvo Language Spec — Labeled Rules

A companion to [LANGUAGE.md](LANGUAGE.md) (the narrative spec). This file
states every language feature as a short, labeled rule, with the compiler's
implementation decisions as sub-bullets. It assumes familiarity with the
language; its purpose is precision and greppability.

Conventions:

* Rule labels are stable identifiers: `[area-topic]`. Compiler code and
  tests reference them in comments (`grep -rn '\[qual-erasure\]'` finds the
  rule, its implementation, and its tests).
* Top-level bullets are *language rules* (what LANGUAGE.md means).
  Sub-bullets are *compiler decisions* (how the implementation realizes the
  rule, including deliberate cuts). When behavior is ambiguous, LANGUAGE.md
  decides; when they conflict, fix one and note it in PROGRESS.md.
* This file is backend-neutral. Each backend has its own
  `BACKEND_SPEC.<backend>.md` (e.g. [BACKEND_SPEC.kotlin.md](BACKEND_SPEC.kotlin.md)),
  loaded only when working on that backend. A backend spec may *repeat*
  rules from this file to add backend interpretation details, and may add
  new rules whose labels are prefixed with the backend's tag (`kt-` for
  Kotlin, `rs-` for Rust).
* "Checker" = `salvo-core/src/check.rs`, "resolver" =
  `salvo-core/src/resolve.rs`; each backend's emitter lives in its own
  crate (`salvo-backend-<backend>`).

## Types

* [type-basic] Basic types: `Byte`, `Int`, `Long`, `Float`, `Double`,
  `Bool`, `Char`, `None` (unit/no-value singleton), `Str`.
  * Declared as `internal type` in `std/core/basic.sv` /
    `std/core/string.sv`; each backend maps them natively.
* [type-str] Strings are immutable, with `${...}` interpolation in
  literals.
  * The lexer captures each `${...}` fragment as raw source + offset; the
    parser re-lexes fragments with spans shifted back into the file.
* [type-tuple] `(A, B, C)` is a tuple type; tuples can be destructured in
  `let`.
  * Backends may support only small sizes; unsupported sizes are codegen
    errors ([backend-never-wrong]).
* [type-union] `A | B | C` is a union type. Duplicate arms collapse
  (`Str | Str` ≡ `Str`) — but *qualified* duplicates are distinct arms.
  * `Ty::Union` invariants: ≥ 2 arms, no nested unions (flattened), arms
    deduped, declaration order preserved.
* [union-arm-identity] Union arm identity is *positional over the declared
  type's non-`None` arms, in declaration order* (qualifiers erased).
  * The checker's side tables (`coerce`, `is_tests`) are expressed in
    these arm indices; every backend's runtime encoding must preserve
    them. Never reorder or dedupe in ways that change arm indices, and
    keep the checker and emitters agreeing on the ident-unwrap predicate.
* [type-nullable] There is no null value: `T?` is shorthand for
  `T | None`. `x!` asserts non-`None` (panics otherwise).
* [type-array] `T[]` is an array; literals `[1, 2, 3]`; generator form
  `Int[5] { i: Int -> 0 }`; `arr[i]` is 0-indexed; size via `size()`.
  * The generator form is an `ArrayInit` AST node (speculative parse).
  * std currently defines `size` for `Str` and `List<T>`, not arrays.
* [type-any-nothing] `Any` is the top type; `Nothing` is the bottom type
  (the type of `return`/`break`/`continue`), subtype of everything.
* [type-alias] `type Name<G> = ...` declares a type alias; aliases can be
  generic and must be imported like other declarations.
  * Aliases expand *structurally* at use sites (with generic
    substitution), in both the checker (`lower_base_ref`) and the
    emitters; no nominal identity.
* [type-unknown-lenient] Anything the checker cannot type is `Ty::Unknown`,
  compatible in both directions, and never an error by itself.
  * This is the backbone of backend interop: unknown
    names/fields/methods pass through to the backend untouched, and
    unchecked code must not cascade errors. Coercions/unwraps fire only
    where checker side tables say so.

## Variables and scoping

* [var-no-shadow] Variable shadowing is not allowed: declaring a name that
  is already visible is an error.
* [var-no-widen] A variable's type can never widen: assignments must be
  subtypes of the *declared* type (qualifiers/narrowing may vary within
  it).
* [var-block-scope] Every block (`if`, `while`, `for`, `when` branches) is
  its own scope; declarations do not escape.
* [let-infer] `let` without an annotation takes the value's logical type
  as the declared type.
  * A narrowed union value is physically re-wrapped to its own logical
    type at the `let` boundary (coercion recorded by the checker).
* [let-destructure] `let (a, b) = ...` and `let {field, field: name} = ...`
  destructure tuples and structs.
  * Struct destructuring binds the *declared* field types, deliberately
    ignoring predicate-qualifier field overrides (see
    [qual-field-override], which applies to direct accesses only).

## Structs

* [struct-decl] `struct Name<G> { field: Type, ... }` — public data
  fields, no methods. Trailing commas allowed.
* [struct-defaults] Fields with `= expr` defaults are optional at
  construction; all other fields are required.
* [struct-spread] `Name {...base, field: v}` copies `base` and overrides
  listed fields.
  * Multiple spreads in one literal are a codegen error (deliberate cut).
* [struct-lit-infer] The struct literal's type annotation can be dropped
  when the expected type is known (`let p: Person = {name: ...}`).
  * A struct literal with no inferable type is a codegen error, never a
    guess.
* [struct-mut] `struct Name with Mut { ... }` opts a struct into the `Mut`
  auto-qualifier; only `Mut Name` values may have fields assigned.
  * `Mut` is the only auto-qualifier.
* [type-with-mut] `Mut` is a language-level qualifier, not a library
  declaration: any type declaration may opt into it with `with Mut`
  (`external type List<T> with Mut`), and applying `Mut` to a type whose
  declaration does not say `with Mut` is an error. `Mut` composes with
  every other qualifier (no `with` compatibility needed).
  * Validated at declaration sites (`validate_quals` in the checker)
    against struct and opaque-type `auto_qualifiers`.
  * Backends decide what `Mut` means. A `define type` block may provide
    a `Mut inline:` template used when the type is `Mut`-qualified
    (Kotlin maps `Mut List<T>` to `MutableList<T>`); without one, `Mut`
    erases for that backend (Rust expresses it as `mut` bindings and
    `&mut` references instead).

## Qualifiers

* [qual-of] `qualifier Q<G> of T` declares a qualifier applying to `T`
  (its `of` type). `Q T` behaves like a distinct type for overloading.
  * Applicability is checked by unification against the `of` type at
    *declaration sites* (fn signatures, `let` annotations, struct fields)
    — not during type lowering, which runs repeatedly.
* [qual-no-dup] The same qualifier cannot be applied twice to one type
  (`Old Old Person` is invalid).
* [qual-with] Two qualifiers may stack on one type only if one declares
  `with` the other; the `Mut` auto-qualifier composes with everything
  [type-with-mut].
  * Pairwise `with` compatibility is validated at declaration sites.
* [qual-union-arm] In an un-parenthesized union, a qualifier binds to the
  single arm it is written on, never the whole union; qualifiers cannot
  apply to tuples.
* [qual-group] A qualifier *can* apply to an explicitly parenthesized
  union group: `Ok (Ok Str | Err Int)`.
  * Lowered as `Ty::Qualified { base: Union }`. A group wraps as a whole
    when it is itself an arm of the expected union, otherwise it coerces
    as the bare inner union (physically identical after erasure). The
    group subtype rule (exact-arm equality, then drop-quals) must be
    tried *before* the generic any-arm union rule; `maybe_coerce`
    mirrors that order.
* [qual-generic] Qualifiers can be generic, and as generic as their `of`
  type or less (`qualifier Ok<T> of T`, `qualifier Ints of Pair<Int,
  Int>`). Nested qualified types (`Ok (Ok Str | Err Int)`) are legal but
  discouraged.
  * `substitute_vars` re-normalizes `Ty::Qualified` through `qualify()` so
    `Ok T` with `T = Ok Str` never nests `Qualified` in `Qualified`.
  * Constructing a nested qualified union group in a single expression
    needs an annotated intermediate `let` (single-level coercion only;
    errors, never mis-emits).
* [qual-predicate] A qualifier with a body is a *predicate qualifier*: it
  declares `fn qualifies(x: OfType) -> Bool`.
  * The `qualifies` signature is validated: exactly one param accepting
    the `of` type, returns `Bool`, no deductions; effects are allowed
    (see [is-qualifies-effects]).
* [is-qualifies] The `is` keyword is backed by the `qualifies` function
  for predicate qualifiers: `x is Positive` on a non-union subject calls
  `Positive.qualifies(x)` at runtime and narrows the subject's type by
  adding the qualifier in the then-branch (no else information).
  * Recorded in the checker's `predicate_tests` table (qualifier names,
    conjunction); backends lower the check to `qualifies` calls.
* [is-qualifies-effects] `qualifies` may declare effects; at each
  predicate `is` site those effects must be available in the caller's
  scope like any call.
* [qual-field-override] A predicate qualifier on a struct may re-declare
  fields with more specific types; these are *not* proven by the checker
  but cast-and-asserted at direct access sites (runtime exception risk is
  the user's).
  * Overrides must refine the real field's type (subtype check); access
    sites are recorded in the checker's `field_casts` table.
* [qual-constructive] A qualifier without a body is *constructive*: values
  gain it only through constructor functions. Plain values never subtype
  `Q T`, and `is Q` on a non-union value is a compile error.
* [qual-ctor-fn] `fn f(...) -> T as Q { ... }` marks a constructor: every
  return point returns plain `T`; the qualifier is applied *by
  construction*, and callers see `Q T`. This replaced value-level `as`
  expressions and the `as` effect (removed from the language).
  * Union tagging goes through generic constructors
    (`fn ok<T>(value: T) -> T as Ok`).
* [qual-ctor-same-file] Constructor functions must be declared in the same
  file as their qualifier.
* [qual-ctor-simple] Constructor return types must be simple (no
  union/tuple); predicate qualifiers cannot have constructors; the
  constructed qualifier must satisfy the `of` type.
* [qual-erasure] Qualifiers are erased in generated code; only their
  compile-time consequences (overload choice, casts, predicate calls,
  union arm choice) survive.
  * Backends must resolve name collisions that erasure creates between
    overloads (see the backend specs).

## Control flow and expressions

* [expr-everything] Every control-flow construct is an expression; a
  branch's value/type is its last expression, and the construct's type is
  the union of branch types.
* [if-bool] `if`/`elif` conditions must be boolean expressions; there is
  no truthiness. `is` checks evaluate to `Bool`.
  * The parser disables struct-literal speculation in condition position
    (`no_struct`) so `if x is Person { ... }` parses.
* [if-else-none] A missing `else` contributes `None` to an
  if-expression's type (`Str` + no else → `Str?`).
* [is-narrowing] `is` checks flow-narrow identifier subjects: matched type
  in the then-branch, remaining arms in the else-branch; `elif` chains
  accumulate exclusions; `&&`/`||`/`!` propagate facts.
  * Only *identifier* subjects narrow (struct-field union subjects are a
    known leftover).
  * Narrowing resets to the declared type for any variable assigned
    inside a branch ([narrow-assign-reset]).
* [is-binding] `is Type name` binds the narrowed value to a fresh
  variable in the matched branch (and per-iteration in `while`).
  * Parse heuristic: uppercase idents in the check are type refs; a
    trailing lowercase ident is the binding.
* [is-precise] Checks may include qualifiers and generics:
  `is Err Str` matches only the `Err Str` arm; `is Err` matches every
  `Err`-qualified arm; overlapping matches infer the smaller union.
* [when-union-subject] `when` requires a union-typed *variable* subject;
  there is no default branch.
* [when-exhaustive] `when` must be exhaustive over the subject's arms;
  arms are consumed sequentially (each branch matches what previous
  branches left), and a branch that can match nothing is an error. A
  `None` arm is handled via the subject's nullability.
* [when-value] `when` is an expression; branches ending in `Nothing`
  (e.g. `return`) drop out of the value type.
* [while-value] `while` evaluates to the last evaluated body expression,
  or a `break value`; `else` runs (and provides the value) only if the
  loop never ran. Same for `for`.
  * The value type joins: the body's tail type, every `break value` type,
    and the `else` tail type — or `None` when there is no `else` (the
    loop may never run). A bare `break` or a `continue` also joins `None`
    (an iteration may end without producing a value); `!` recovers the
    non-optional type when the user knows better.
  * Tails and break values coerce to the join like `if` branch values;
    `break`/`continue` outside a loop are errors; a lambda body is a
    loop barrier.
* [loop-while-is] `while x is T (name)?` re-tests in the loop condition
  and re-binds per iteration at the top of the body.
* [for-iter] `for x in e` iterates `iter(e)` implicitly when `e` is not
  already an `Iter<T>`; missing/ambiguous `iter` resolution is an error.

## Functions

* [fn-syntax] `fn name<G>(params) [effects] -> [deductions] ReturnType`;
  no implicit returns from functions (unlike blocks); omitted return type
  means `None`; omitted effect list means pure (`[]`).
* [fn-return-none] Functions returning `None` may `return` bare or not
  return at all.
* [fn-overload] Functions overload by parameter types (including
  qualifiers: `full_name(Person)` vs `full_name(Surname Person)`).
  * The checker scores viable candidates (exact type match > subtype;
    qualified params more specific) and records the winner per call site
    (`call_fn`). Emitters fall back to arity-based resolution in
    unchecked contexts.
* [fn-dot] Dot-notation: `x.f(a)` ≡ `f(x, a)` whenever `f` resolves to a
  known fn/define/effect member; otherwise it stays a backend method call
  ([type-unknown-lenient] interop).
* [fn-variadic] `...xs: T[]` collects remaining arguments as an array;
  a spread argument `...xs` forwards an array whole; variadics bind after
  fixed params.
* [fn-lambda] Lambdas: `x -> expr`, `(a, b) -> expr`, and block bodies
  `{ x: T -> ... }` (blocks require `return`). Lambdas may declare
  effects/deductions.
  * Param types come from annotation or the expected fn type; early
    `return` inside expression-position lambdas is a codegen error
    (deliberate cut).
* [fn-iterator] A function returning `Iter<T>` and using `yield` is an
  iterator function: `yield` produces elements; `return` only
  short-circuits (no value). `Iter<T>` is an `internal type` each backend
  maps to its native iterable.

## Effects

* [effect-decl] `effect E<G> { fn member(...) -> T }` declares an effect:
  a set of functions available to code that depends on `E`.
* [effect-member-no-effects] Effect member fns (and handler member fns)
  cannot declare their own effect dependencies (compile error "…yet"):
  dispatch call sites go through the handler instance and cannot thread
  extra handler args.
* [effect-handler] `handler H<G>(ctor params) of E<G> { state fns }`
  implements every member of its effect; state fields have initializers
  and persist for the handler's lifetime.
* [effect-fn-deps] A fn's `[E1, E2<T>]` list declares its effect
  dependencies. Calling a fn requires each of its effects to be available
  in the caller (declared or `use`d) — validated by the checker at every
  call site, recorded per-call (`call_effects`) in declaration order.
* [effect-no-dup] Two effects of the same type in one list are an error
  unless their generic arguments differ (`[Random<Int>, Random<Double>]`
  is fine, `[Console, Console]` is not).
* [effect-use] `use Handler(...)` registers a handler instance for the
  rest of the current scope; `use Handler` is sugar for `use Handler()`.
  * The checker infers the handler's generics from the constructor
    arguments and registers the *concrete* effect instance
    (`use CyclicRandom(list(1,2,3))` registers `Random<Int>`), recorded
    in `use_effects`.
* [use-requires-use] `use` is only legal in functions declaring the
  special `use` effect (`main() [use]` is the conventional entry point).
* [use-no-dup] Registering a second handler for an effect instance already
  in scope is an error.
* [effect-available] Calling an effect member requires an instance of its
  effect in scope; otherwise "no handler for effect" (a spanned checker
  error since M5).
* [effect-disambiguation] With multiple instances of a generic effect in
  scope, a member call disambiguates by (in order): explicit type args
  (`next_random<Int>()`), argument types, the expected type
  (`let i: Int = next_random()`). Still >1 → "ambiguous effect call"
  error; 0 → "no handler" error.
  * The resolved instance is recorded per call site (`effect_calls`).
* [effect-scope] `use` registrations are block-scoped: they expire at the
  end of the enclosing block.
  * Checker `effect_env` and emitter environments truncate at block
    boundaries identically.

## Deductions

* [deduce-syntax] The `-> [param: Quals, ...]` list states what a call
  does to each parameter: listed = returned to the caller (borrowed) with
  exactly the listed qualifiers still known; omitted from a specified
  list = moved (caller loses access).
  * Deductions are interpreted relative to the qualifiers *declared on
    the parameter*: a call removes exactly `declared − kept` from the
    argument's known qualifiers. Qualifiers the argument carries beyond
    the declared ones are unaffected (`fn f(list: A B List<T>) ->
    [list: B]` applied to an `A B C List<T>` leaves `B C List<T>`).
  * Written lists are shape-checked: entries must name a parameter
    (once), and may only keep qualifiers declared on that parameter.
* [deduce-infer] An unspecified deduction list is inferred as the
  strictest deduction over all uses of each parameter in the body;
  deductions never depend on the return value. If inference is impossible,
  they must be written.
  * Inference runs as a whole-program fixpoint after checking
    (`deduce.rs`), starting optimistic (everything kept with its declared
    qualifiers); constraints only remove facts, so it terminates. Results
    are stored in the typed IR (`Checked::deductions`); Kotlin ignores
    them — they are the Rust backend's ownership/borrow contract.
  * Moves are inferred when a bare parameter is: passed to a call whose
    deduction omits it, bound by `let`/assignment, returned, `break`- or
    `yield`-ed, stored in a struct/array/tuple literal, or passed to a
    `use` handler constructor. Unresolved callees (backend interop,
    effect members) borrow leniently and preserve all qualifiers; value
    flow out of a branch/loop tail is not tracked as a move yet.
  * A written list is validated against the same body facts: it may be
    *stricter* than the body (drop qualifiers, move parameters the body
    gives back), but promising a parameter back that the body moves, or
    a qualifier the body may remove, is an error.

## Modules, imports, files

* [mod-file] `.sv` files are modules; the module path is the file path (no
  in-file module declaration).
* [mod-ignore] Source discovery walks the source root recursively but
  skips: hidden directories (`.git`, ...), cache directories carrying a
  `CACHEDIR.TAG` marker (Cargo's `target/`), and anything listed in
  `<root>/.svignore` — one path per line, relative to the root, naming a
  file or a directory subtree; blank lines and `#` comments ignored.
  The root itself is exempt from the hidden/cache rules.
* [mod-visibility] Code sees: everything declared in its own module (all
  files of the module, including backend define files), everything in
  `core.*` (implicit), and whatever it imports.
* [mod-import] `import path.Name` / `import path.Name as Alias`; aliasing
  resolves ambiguity. Unresolved/ambiguous imports are errors.
  * Import prefixes match module paths exactly or as a leading path
    (`import core.Str` finds `core.string`). Non-fn name collisions
    across visible modules currently last-win silently (known leftover).
* [mod-used-only] Only modules used by the program are transpiled.
  * Roots are the user modules declaring `fn main` (all user modules for
    a library compile without one). Reachability follows *name usage*:
    every identifier/type name a file mentions is looked up in its
    resolved scope (`ModuleScope::name_origins`), and each declaring
    module becomes reachable (`reach.rs`). Deliberately conservative:
    shadowed and overloaded names pull in every declaring module.
  * A module's backend define files travel with it; a reachable module is
    emitted only if it produces code.

## Backends

* [backend-internal] `internal` declarations (types) are
  mapped inside the compiler; every backend must handle all of them
  (`Str`, numeric types, `Iter<T>`, ...). The `Mut` auto-qualifier is
  mapped per backend via `Mut inline:` define sections [type-with-mut].
* [backend-external] `external` declarations (fns, types, handlers) carry
  only signatures; each backend that needs them provides `define`
  templates in a sibling `<module>.<backend>.sv` file. Coverage is
  checked at compile time: everything external in `core.*` must have a
  define (core is implicitly imported); outside core, a missing define
  is an error at every reference (call to an external fn, use of an
  external type, emission of an external handler) — never a silent
  pass-through.
  * `SourceSet::classify` filters define files per selected backend at
    load time.
* [backend-companion] A backend-native source file next to a module's
  sources (`complicated.kt` beside `complicated.sv`, using the backend's
  native extension) is a *companion*: it is copied verbatim into the
  output whenever its module is reachable, letting `define` templates
  delegate to hand-written native code. A companion module should
  declare only `external` items; a companion that collides with a
  generated file is an error.
* [backend-define-inline] `define fn` bodies hold an `inline:`
  \`\` template \`\` interpolated at each call site: `${param}` splices the
  argument's code, `${...variadic}` splices remaining arguments.
  Template output is written as-is (correctness not validated by Salvo).
  * Templates lex as raw dedented `Template` tokens. A template calling a
    same-named native fn must qualify it (`kotlin.io.print`) to avoid
    self-recursion.
  * Overloaded externals share a define name; templates are matched to
    the checker-resolved declaration by parameter base types, then
    arity/shape (`define_for_decl`; the arity-only fallback can still
    mis-pick in unchecked contexts — known leftover).
* [backend-define-imports] `imports:` sections list target-language
  imports, hoisted (deduped) to the top of any file whose code used the
  template.
* [backend-define-type] `define type` templates map external types
  (`${T}` interpolates generic args); `internal type`s map natively in
  the compiler.
* [backend-define-handler] `define handler H of E { define fn ... }`
  provides template bodies for an external handler's members; external
  handlers support constructor params like any other handler.
* [backend-companion] A `<name>.<ext>` companion source file next to a
  define file is copied into the output when used (not yet implemented,
  M7).
* [backend-never-wrong] A backend must never emit silently wrong code:
  unsupported constructs are codegen/checker errors. Each backend spec
  lists its current deliberate cuts.

## Tooling

* [diag-structured] The resolver, checker, and deduction pass report
  errors as structured diagnostics — `(file index, span, severity,
  message)` (`salvo_core::FileDiagnostic`) — never as pre-rendered
  strings. Human-readable rendering (file:line:col + caret) happens at
  the consuming boundary: the backends render before returning
  `BackendError`, the CLI renders for terminal output. Parser
  diagnostics stay per-file (`salvo_syntax::Diagnostic`); the CLI
  attributes them to files the same way. This is the contract a future
  language server builds on.
* [diag-import-suggest] Diagnostics for unresolved names carry structured
  import suggestions: the modules elsewhere in the program that declare
  the name, as `module.Item` paths (`FileDiagnostic::suggested_imports`).
  * Computed from a whole-program declaration index
    (`Resolution::declared_in`); effect members map to their owning
    *effect* (importing the effect brings its members). `core.*` modules
    are never suggested — they are implicitly visible, so an unknown
    name cannot be fixed by importing from core.
  * Attached at: unknown handler in `use`, unknown effect in an effect
    list, and unresolved imports (which suggest the correct module path
    for the item, e.g. `import std.random.Random` -> `import
    random.Random`).
  * Rendering: text mode appends one ``help: add `import …` `` line per
    suggestion; `--format json` adds an `"imports"` array (only when
    non-empty); the LSP carries them on `Diagnostic.data` and serves
    `textDocument/codeAction` quickfixes ("Add `import …`") that insert
    the import line after the file's last import (or at the top).
* [cli-analyze] `salvo analyze --src DIR [--backend NAME]
  [--format text|json]` runs the front half of the pipeline — parse,
  resolve, type-check — and reports every diagnostic without generating
  code. Exit code is nonzero iff any diagnostic is an error.
  * Analysis is backend-neutral: checking never consults define files.
    `--backend` only opts that backend's `*.<backend>.sv` define files
    into loading (so they get parse checking); without it only language
    files are loaded.
  * Resolve/check always run, even with parse errors — a broken file
    must not suppress diagnostics elsewhere. Parse-broken files
    participate with their recovered ASTs (their parsed declarations
    still resolve for other files) but contribute only their parse
    diagnostics; their resolution/checker diagnostics are dropped
    (recovered ASTs cascade nonsense).
  * `--format json` prints a JSON array of
    `{file, line, col, start, end, severity, message}` objects to
    stdout (line/col 1-based, start/end byte offsets); text mode
    renders to stderr with a summary line.
* [cli-lsp] `salvo lsp [--backend NAME]` starts a language server
  speaking LSP over stdio. The workspace root comes from the client's
  `initialize` request; `--backend` selects define files exactly like
  `analyze --backend`.
  * No incremental state: every document event re-runs the [cli-analyze]
    pipeline over the whole workspace, with open-editor buffers as a
    content overlay (unsaved files under the root participate).
  * Diagnostics are pushed on open/change/close/save; every open
    document gets a publish (an empty list marks it clean), and files
    whose diagnostics disappeared get an explicit clearing publish.
    Diagnostics are published against the URI the client opened the
    document under (clients compare URIs exactly).
  * Hover returns the checker's type (`Checked::expr_ty`) for the
    smallest expression under the cursor; `Unknown`-typed expressions
    yield no hover.
  * `textDocument/codeAction` serves import quickfixes from the
    suggestions on published diagnostics [diag-import-suggest].
  * Positions convert between byte offsets (Salvo spans) and UTF-16
    line/character pairs (the LSP default encoding).
* [cli-lang] `salvo lang tm-grammar [--out PATH]` emits the TextMate
  grammar consumed by the VS Code extension (`vscode/syntaxes/`);
  without `--out` it prints to stdout.
  * Keyword alternations are derived from the lexer's keyword table
    (`salvo_syntax::token::KEYWORDS` — the same table
    `TokenKind::keyword` consults), partitioned into highlighting
    categories (control / declaration / other / boolean). A test
    asserts the partition covers the table exactly, so adding a keyword
    without categorizing it fails `cargo test`.
  * The checked-in extension grammar must byte-equal the generated one
    (`vscode_extension_grammar_is_up_to_date`); regenerate with
    `cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json`.
