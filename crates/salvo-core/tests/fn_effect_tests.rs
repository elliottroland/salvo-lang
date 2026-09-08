//! Effects on fn types [fn-effects].
//!
//! An effect list on a fn type is a *requirement the caller of the value
//! supplies*, not a capability the value carries: `(s: Str) [Logger] -> Str`
//! means "call me with Logger available". So a lambda body performs only the
//! effects its type declares, a higher-order fn *inherits* its fn-typed
//! parameters' effects (user decision 2026-09-04 — the only reason to take
//! `f` is to call it), and a fn value can be stored or passed freely because
//! it holds no handler: only *calling* it needs one.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str =
    "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type Iter<T>\n";

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

effect Logger {
    fn log(message: Str) -> [message] None
}

effect Counter {
    fn bump() -> [] None
}

handler QuietLogger of Logger {
    fn log(message: Str) -> [message] None {}
}
handler ZeroCounter of Counter {
    fn bump() -> [] None {}
}

fn note(text: Str) [] -> [text] None {}
"#;

// ===== a lambda body performs what its type declares =====

/// [fn-effects] A lambda passed where a fn type declares the effect may
/// perform it — the call site supplies it.
#[test]
fn a_declared_effect_is_available_in_the_body() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn probe() [Logger] -> [] None {{\n\
         run(s -> {{\n\
         log(\"in lambda ${{s}}\")\n\
         return \"seen ${{s}}\"\n\
         }})\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-effects] The dual: a lambda checked against a fn type that declares
/// *nothing* may not perform an effect, even though the enclosing scope has
/// one. Lexical leakage is what made a fn value's capabilities invisible in
/// its type.
#[test]
fn an_undeclared_effect_in_a_lambda_body_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn probe() [Logger] -> [] None {{\n\
         run(s -> {{\n\
         log(\"in lambda ${{s}}\")\n\
         return \"seen ${{s}}\"\n\
         }})\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Logger`")),
        "expected the lambda body to be rejected, got: {errs:?}"
    );
}

// ===== inheritance (user decision 2026-09-04) =====

/// [fn-effects] A fn taking an effectful fn value *inherits* the effect: it
/// needs no list of its own to call `f`, and its callers supply it.
#[test]
fn a_fn_inherits_its_fn_typed_parameters_effects() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-effects] The other half of inheritance: a caller that *cannot*
/// supply the inherited effect is rejected, since that is where the value
/// comes from at run time.
#[test]
fn a_caller_must_supply_an_inherited_effect() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [] -> [] None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no handler for effect `Logger`") && e.contains("run")),
        "expected the caller to be rejected, got: {errs:?}"
    );
}

/// [fn-effects] Inheritance reaches through a qualifier: `Once` fn types
/// carry effects the same way.
#[test]
fn inheritance_reaches_through_a_qualifier() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run_once(f: Once () [Logger] -> None) -> [] None {{\n\
         f()\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run_once(() -> {{ log(\"once\") }})\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== variance =====

/// [fn-effects] A value performing *fewer* effects fits where more are
/// expected: the caller supplies what it declared and the value ignores it.
#[test]
fn fewer_effects_fit_where_more_are_expected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn pure_fn(s: Str) [] -> [s] Str {{\n\
         return \"plain ${{s}}\"\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(pure_fn)\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-effects] Never the reverse: a value that performs an effect its
/// position does not declare would reach a call site that cannot supply it.
#[test]
fn more_effects_do_not_fit_where_fewer_are_expected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn shout(s: Str) [Logger] -> [s] Str {{\n\
         log(s)\n\
         return \"loud ${{s}}\"\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(shout)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no matching overload")
            || e.contains("expected")
            || e.contains("Logger")),
        "expected the effectful fn to be rejected, got: {errs:?}"
    );
}

// ===== inference =====

/// [fn-effects] An un-annotated lambda's effect set is *inferred* from its
/// body (inference from a visible body is what [decl-explicit] permits), so
/// it does not silently fit a pure position.
#[test]
fn an_unannotated_lambdas_effects_are_inferred() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         let f = (s: Str) -> {{\n\
         log(s)\n\
         return \"logged ${{s}}\"\n\
         }}\n\
         run(f)\n\
         }}\n"
    ));
    assert!(
        !errs.is_empty(),
        "expected the inferred `[Logger]` to be rejected by the pure position"
    );
}

// ===== a fn value carries no capability =====

/// [fn-effects] Storing an effectful fn value is fine — it holds no handler.
/// What needs the effect is *calling* it, which is why "forbid escape" was
/// dropped from the design (user decision 2026-09-04).
#[test]
fn calling_is_what_needs_the_effect_not_holding() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe() [Logger] -> [] None {{\n\
         let f = (s: Str) -> {{\n\
         log(s)\n\
         return \"logged ${{s}}\"\n\
         }}\n\
         let g: (s: Str) -> [s] Str = (s: Str) -> {{\n\
         return f(s)\n\
         }}\n\
         note(g(\"x\"))\n\
         }}\n"
    ));
    // `g` declares no effects, so the `f(s)` inside it has no Logger.
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Logger`")
            && e.contains("function value")),
        "expected the inner call to be rejected, got: {errs:?}"
    );
}

/// [fn-effects] Two effects on one fn type both have to be supplied.
#[test]
fn every_declared_effect_must_be_available_at_the_call() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger, Counter] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Counter`")),
        "expected the missing second effect to be reported, got: {errs:?}"
    );
}

// ===== `use` is not part of a fn type =====

/// [fn-effects] Registering a handler is local to a body, so `use` in a fn
/// type's effect list is meaningless — a lambda may `use` exactly when the
/// function containing it may.
#[test]
fn use_in_a_fn_type_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [use] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("a fn type cannot declare `use`")),
        "expected the `use` rejection, got: {errs:?}"
    );
}

// ===== [iter-effects] a producer declares its effects on its return type =====
//
// None of a `yield` fn's body runs when it is called: the effects happen
// while the *consumer* drives the pass. So the list does not belong on the
// function, where [fn-effects] would make every call site supply a handler
// for something calling it never does — it belongs on the return type, in
// qualifier position (user decision 2026-09-07, D8).

/// [iter-effects] An effect in the fn's *own* list is rejected, and the
/// diagnostic names the remedy: write it on the return type.
#[test]
fn an_iterator_fn_declares_its_effects_on_its_return_type() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn counted(n: Int) [Logger] -> Iter<Int> {{\n\
         log(\"start\")\n\
         yield n\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("declares its effects on its return type")
                && e.contains("-> Logger Iter<…>")
        }),
        "expected the relocation diagnostic with its remedy, got: {errs:?}"
    );
}

/// And with the claim in the right place, the body may perform it.
#[test]
fn a_producer_may_perform_what_its_return_type_claims() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn counted(n: Int) -> Logger Iter<Int> {{\n\
         log(\"start\")\n\
         yield n\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A claim it does not make is still an error — the relocation moves the
/// declaration, it does not remove it.
#[test]
fn a_producer_may_not_perform_what_it_does_not_claim() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn counted(n: Int) -> Iter<Int> {{\n\
         log(\"start\")\n\
         yield n\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("Logger")),
        "expected the undeclared-effect error, got: {errs:?}"
    );
}

/// `use` is a hole of its own: it would let the body register a handler the
/// pass then has to carry across every suspension.
#[test]
fn an_iterator_fn_cannot_declare_use() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn counted(n: Int) [use] -> Iter<Int> {{\n\
         use QuietLogger()\n\
         yield n\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("cannot `use` a handler of its own")),
        "expected the `use` rejection, got: {errs:?}"
    );
}

/// [iter-mut-param] A producer's parameters are *captured* — by the factory
/// and again by each pass — so a mutable one is state a suspended body shares
/// with its caller, and the backends did not agree on what a write through it
/// meant: Rust captured by clone (a private copy, invisible to the caller,
/// fresh per pass), Kotlin handed the reference over (the caller's collection
/// mutated, and successive passes accumulating). Refused rather than sided
/// with (user decision 2026-09-08); the reproduction that forced it is under
/// "Open defects" in PROGRESS.md.
#[test]
fn an_iterator_fn_cannot_take_a_mutable_parameter() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         struct Buffer canbe Mut {{\n\
         at: Int\n\
         }}\n\
         fn counted(n: Int, sink: Mut Buffer) -> Iter<Int> {{\n\
         yield n\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("cannot take a mutable parameter")),
        "expected the mutable-parameter rejection, got: {errs:?}"
    );
}

/// And it is the **transitive** test [fate-move-mode] uses, not a look at the
/// written qualifiers: `Mut` reachable through a struct field counts, since
/// that is equally what makes clone-vs-alias observable.
#[test]
fn a_producers_mutable_parameter_is_caught_through_a_field() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         struct Buffer canbe Mut {{\n\
         at: Int\n\
         }}\n\
         struct Holder {{\n\
         buffer: Mut Buffer\n\
         }}\n\
         fn counted(n: Int, holder: Holder) -> Iter<Int> {{\n\
         yield n\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("cannot take a mutable parameter")),
        "expected the transitive rejection, got: {errs:?}"
    );
}

/// The control, so the rule is not "a producer may take nothing": an
/// immutable parameter of the same shape is fine, and so is a mutable
/// parameter on an ordinary fn.
#[test]
fn an_immutable_producer_parameter_is_fine() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         struct Buffer canbe Mut {{\n\
         at: Int\n\
         }}\n\
         fn counted(n: Int, buffer: Buffer) -> Iter<Int> {{\n\
         yield n\n\
         }}\n\
         fn bump(buffer: Mut Buffer) -> [buffer: Mut] None {{\n\
         buffer.at = buffer.at + 1\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// [iter-mut-param] The **callback** half of the rule: a producer holds its
/// fn-typed parameters for as long as it can mint a pass, and calls one once
/// per element in *every* pass — so a lambda carrying mutable state in is the
/// same hazard as a mutable parameter, reached through a capture. Rust already
/// refused it (`impl Fn + 'static`) while Kotlin ran it and accumulated across
/// passes: a checker-clean program only one backend could build.
#[test]
fn a_producers_callback_cannot_write_through_a_capture() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         struct Buffer canbe Mut {{\n\
         at: Int\n\
         }}\n\
         fn raise(b: Mut Buffer) -> [b: Mut] Int {{\n\
         b.at = b.at + 1\n\
         return b.at\n\
         }}\n\
         fn tagged(xs: Iter<Int>, f: (Int) -> Int) -> Iter<Int> {{\n\
         for x in xs {{\n\
         yield f(x)\n\
         }}\n\
         }}\n\
         fn drive(xs: Iter<Int>, buf: Mut Buffer) -> [] None {{\n\
         let source = tagged(xs, (v) -> raise(buf))\n\
         return None\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("writes through `buf`")),
        "expected the write-capture rejection, got: {errs:?}"
    );
}

/// And a *read* of mutable data is refused too, for [iter-mut-param]'s own
/// reason rather than for `FnMut`: whether the closure holds a snapshot or an
/// alias is observable, so the two backends would not agree on what the
/// callback sees on a later pass.
#[test]
fn a_producers_callback_cannot_read_mutable_data_through_a_capture() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         struct Buffer canbe Mut {{\n\
         at: Int\n\
         }}\n\
         fn peek(b: Buffer) -> [b] Int {{\n\
         return b.at\n\
         }}\n\
         fn tagged(xs: Iter<Int>, f: (Int) -> Int) -> Iter<Int> {{\n\
         for x in xs {{\n\
         yield f(x)\n\
         }}\n\
         }}\n\
         fn drive(xs: Iter<Int>, buf: Mut Buffer) -> [buf] None {{\n\
         let source = tagged(xs, (v) -> peek(buf))\n\
         return None\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("reads mutable data through `buf`")),
        "expected the read-capture rejection, got: {errs:?}"
    );
}

/// The control, and the remedy the diagnostic names: an **immutable** capture
/// is free — clone-vs-alias is unobservable, which is the standing parity
/// argument — so a snapshot at a non-`Mut` type goes through. The same lambda
/// against a *non*-producer is fine too, since only a pass keeps its callback
/// past the call.
#[test]
fn a_producers_callback_may_capture_immutable_data() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         struct Buffer canbe Mut {{\n\
         at: Int\n\
         }}\n\
         fn peek(b: Buffer) -> [b] Int {{\n\
         return b.at\n\
         }}\n\
         fn tagged(xs: Iter<Int>, f: (Int) -> Int) -> Iter<Int> {{\n\
         for x in xs {{\n\
         yield f(x)\n\
         }}\n\
         }}\n\
         fn eager(xs: Iter<Int>, f: (Int) -> Int) -> Int {{\n\
         return f(1)\n\
         }}\n\
         fn drive(xs: Iter<Int>, snapshot: Buffer) -> [snapshot] None {{\n\
         let lazy = tagged(xs, (v) -> peek(snapshot))\n\
         let now = eager(xs, (v) -> peek(snapshot))\n\
         return None\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// The restriction is on *producing*, not consuming: a `for` loop over an
/// iterator sits in an ordinary fn and may perform whatever it declares.
#[test]
fn consuming_an_iterator_may_perform_effects() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn counted(n: Int) -> Iter<Int> {{\n\
         yield n\n\
         }}\n\
         fn report(n: Int) [Logger] -> [] None {{\n\
         for v in counted(n) {{\n\
         log(\"one\")\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}
