//! Implicit parameters [implicit-param] [implicit-group]
//! [implicit-resolve] [implicit-forward] [implicit-override].
//!
//! `?cmp: (T, T) -> Int` is a parameter the caller need not pass: the call
//! site fills it by resolving the parameter's *name* at the parameter's
//! *type*. That is the overload query the language already runs, only
//! against a type instead of an argument list — which is why Salvo needs no
//! qualified-name syntax for the defaults (Koka's `Str/cmp`): the `cmp`
//! whose parameters accept `Str` *is* the default for `Str`.
//!
//! `?Field<T>` spreads a `fns` group. It has no binder (user decision
//! 2026-09-05): the members become implicit parameters in their own right,
//! so resolution, forwarding and override all key on a member's own name
//! and type, and a callee can reach the same parameter through a different
//! grouping — or none.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// [unique-shape] `cmp` and `times` are operators on `Int`, so they are
/// declared in `Int`'s own file, here, and reach the tests' programs as any
/// std overload does.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export fn cmp(a: Int, b: Int) -> Int { return 0 }\n\
                           export fn times(a: Int, b: Int) -> Int { return 0 }\n";

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
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
            "unexpected parse errors: {parse_errors:?}"
        );
        modules.push(ast);
    }
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"
fns Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return 0 }
fn zero() -> Int { return 0 }
"#;

// ===== [implicit-resolve] the default is an ordinary overload =====

/// The `add`/`zero` whose parameters accept `Int` are what a call with
/// `T = Int` gets — no declaration ties them to the type.
#[test]
fn a_group_resolves_its_members_from_the_visible_overloads() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {{\n\
         return add(a, zero())\n\
         }}\n\
         fn probe() -> Int {{\n\
         return total(1)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// An individually written implicit needs no group at all.
#[test]
fn a_written_implicit_resolves_the_same_way() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> Int {{\n\
         return pick(1, 2)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// Nothing resolves for a type with no matching declaration, and the error
/// names both remedies — declare one, or pass one here.
#[test]
fn an_unresolvable_implicit_names_both_remedies() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe(s: Str) -> Int => !s {{\n\
         return pick(s, s)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            // A `cmp` *is* declared — for `Int` — so the diagnostic says which
            // one it looked at and what the position needed, rather than
            // claiming nothing of that name exists.
            e.contains("no `cmp` fits")
                && e.contains("the `cmp` in scope is")
                && e.contains("the position needs")
        }),
        "expected the near-miss explanation, got: {errs:?}"
    );
}

// ===== [implicit-override] `name = value` at the call site =====

/// The caller can replace one member of a group by its own name — the point
/// of dropping the binder.
#[test]
fn a_call_can_override_one_member_by_name() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {{\n\
         return add(a, zero())\n\
         }}\n\
         fn probe() -> Int {{\n\
         return total(1, add = times)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A lambda works as well as a fn name.
#[test]
fn an_override_can_be_a_lambda() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> Int {{\n\
         return pick(1, 2, cmp = (x: Int, y: Int) -> x)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A name that matches no implicit parameter is a mistake, not a no-op.
#[test]
fn a_named_argument_matching_nothing_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> Int {{\n\
         return pick(1, 2, order = times)\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("has no implicit parameter named `order`")),
        "expected the unknown-name rejection, got: {errs:?}"
    );
}

/// The value has to fit the parameter's type.
#[test]
fn an_override_of_the_wrong_type_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> Int {{\n\
         return pick(1, 2, cmp = (x: Int) -> x)\n\
         }}\n"
    ));
    assert!(!errs.is_empty(), "expected the mismatched override to be reported");
}

// ===== [implicit-forward] generic code passes its own implicits on =====

/// Inside a generic fn nothing about `T` is knowable [call-resolve], so the
/// only thing that can fill an inner call's implicit is the enclosing fn's
/// own — matched by name and type, and passed on without being named.
#[test]
fn a_generic_fn_forwards_its_own_implicits() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {{\n\
         return add(a, zero())\n\
         }}\n\
         fn total_twice<T>(a: T, b: T, ?Field<T>) -> T => !a, !b {{\n\
         return add(total(a), total(b))\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// Forwarding matches on name and type, not on how the parameters were
/// declared: a group here fills an individually-declared implicit there.
#[test]
fn forwarding_ignores_how_the_parameters_were_grouped() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> T => !a, !b {{\n\
         return add(a, b)\n\
         }}\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {{\n\
         return sum_pair(a, zero())\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A generic fn that declares *nothing* cannot call one that needs an
/// implicit: there is no default for an opaque `T` and nothing to forward.
/// This is the colouring the feature costs, and the error says what to add.
#[test]
fn a_generic_fn_without_the_implicit_cannot_call_one_that_needs_it() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {{\n\
         return add(a, zero())\n\
         }}\n\
         fn careless<T>(a: T) -> T => !a {{\n\
         return total(a)\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no `add`") && e.contains("for `total`")),
        "expected the missing-implicit error, got: {errs:?}"
    );
}

// ===== declaration-site rules =====

/// [implicit-param] Only a function can be resolved by name and type.
#[test]
fn an_implicit_of_non_fn_type_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(a: Int, ?limit: Int) -> Int {{\n\
         return a\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("an implicit parameter must have a function type")),
        "expected the fn-type requirement, got: {errs:?}"
    );
}

/// [implicit-group] A spread of an unknown group names the mistake.
#[test]
fn an_unknown_group_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f<T>(a: T, ?Ring<T>) -> T => !a {{\n\
         return a\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no `fns` group named `Ring`")),
        "expected the unknown-group error, got: {errs:?}"
    );
}

/// [implicit-same-name] Two implicits of one name at different types are two
/// parameters (user decision 2026-09-28): `?Field<A>, ?Field<B>` brings one
/// `field` per type, and nothing is refused.
#[test]
fn two_implicits_of_the_same_name_at_different_types_are_two_parameters() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f<A, B>(a: A, b: B, ?Field<A>, ?Field<B>) -> A => !a, !b {{\n\
         return a\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

/// A group's type arguments have to match its generics.
#[test]
fn a_group_spread_with_the_wrong_arity_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f<A, B>(a: A, ?Field<A, B>) -> A => !a {{\n\
         return a\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("takes 1 type argument(s), found 2")),
        "expected the arity error, got: {errs:?}"
    );
}

/// [implicit-param] An effect member is an ordinary signature, so it may
/// declare implicit parameters (user decision 2026-09-06). They resolve at
/// the *call*, and the handler receives them like any other argument.
#[test]
fn an_effect_member_may_take_implicits() {
    let errs = errors(
        "fn fmt(n: Int) -> Str => n { return \"\" }\n\
         effect Show {\n\
         fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v\n\
         }\n\
         handler Angle of Show {\n\
         fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v { return fmt(v) }\n\
         }\n\
         fn probe() [use] -> Str {\n\
         use Angle()\n\
         return show(7)\n\
         }\n",
    );
    assert!(errs.is_empty(), "{errs:?}");
}

/// And the call site can override one, exactly as for a fn.
#[test]
fn an_effect_member_call_can_override_an_implicit() {
    let errs = errors(
        "fn fmt(n: Int) -> Str => n { return \"\" }\n\
         fn loud(n: Int) -> Str => n { return \"\" }\n\
         effect Show {\n\
         fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v\n\
         }\n\
         handler Angle of Show {\n\
         fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v { return fmt(v) }\n\
         }\n\
         fn probe() [use] -> Str {\n\
         use Angle()\n\
         return show(7, fmt = loud)\n\
         }\n",
    );
    assert!(errs.is_empty(), "{errs:?}");
}

/// [copy-implicit] A handler *constructor* may take implicit parameters
/// (lifted 2026-09-11): the `use` site is where the handler's type arguments
/// are known, so it resolves them as a call resolves a fn's. The forcing case
/// is a generic handler that must `copy` a `T` it cannot see through.
#[test]
fn a_handler_constructor_resolves_implicits_at_use() {
    let errs = errors(
        "fn fmt(n: Int) -> Str => n { return \"\" }\n\
         effect Show {\n\
         fn show(v: Int) -> Str => v\n\
         }\n\
         handler Angle(?fmt: (n: Int) -> Str) of Show {\n\
         fn show(v: Int) -> Str => v { return fmt(v) }\n\
         }\n\
         fn main() [use] {\n\
         \x20   use Angle()\n\
         \x20   let _s = show(1)\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected `?fmt` to resolve at `use`, got: {errs:?}");
}

/// A `fns` group member is a signature for a parameter; the default comes
/// from a matching top-level fn, so a body here means nothing.
#[test]
fn a_group_member_with_a_body_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fns Ring<T> {{\n\
         fn mul(a: T, b: T) -> T {{ return a }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("is a signature, not an implementation")),
        "expected the body rejection, got: {errs:?}"
    );
}

// ===== the contract is part of fitting, and does not print =====

/// A fn whose *types* match but whose contract does not cannot fill the
/// position — and `Ty`'s Display shows no contract, so the bare mismatch
/// would read "expects `(Int, Int) -> Int`, found `(Int, Int) -> Int`". The
/// diagnostic has to name the argument, the direction, and the fix.
#[test]
fn a_contract_mismatch_is_explained_rather_than_printed() {
    let errs = errors(
        "fns Field<T> {\n\
         fn add(a: T, b: T) -> T\n\
         fn zero() -> T\n\
         }\n\
         fn add(a: Int, b: Int) -> Int\n=> !a {\n return a\n}\n\
         fn zero() -> Int { return 0 }\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {\n\
         return add(a, zero())\n\
         }\n\
         fn probe() -> Int {\n\
         return total(1)\n\
         }\n",
    );
    assert!(
        errs.iter().any(|e| {
            e.contains("*consumes* `a` while this position keeps it")
                && e.contains("the types match, the contracts do not")
                && e.contains("=>[")
        }),
        "expected the contract explanation with both fixes, got: {errs:?}"
    );
}

/// The same explanation for a value written at the call site: there the
/// mismatch is between the *given* fn and the parameter.
#[test]
fn an_override_with_the_wrong_contract_is_explained() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn eats(a: Int, b: Int) -> Int\n=> !a {{ return a }}\n\
         fn total<T>(a: T, ?Field<T>) -> T => !a {{\n\
         return add(a, zero())\n\
         }}\n\
         fn probe() -> Int {{\n\
         return total(1, add = eats)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("`add` does not fit here")
                && e.contains("`eats` *consumes* `a`")
        }),
        "expected the override contract explanation, got: {errs:?}"
    );
}

/// [implicit-resolve] [fn-overload-rank] An implicit is chosen as a call is
/// (user decision 2026-10-14): of a generic `order<T>` and a concrete
/// `order(Int, Int)`, both fitting `(Int, Int) -> Int`, the more specific wins.
/// Under the old rule — the most specific *scope*, then "two hits are
/// ambiguous" — this was an ambiguity. (`order`, not `cmp`: a `cmp` on a type
/// parameter or on `Int` outside `Int`'s file is refused [unique-shape].)
#[test]
fn a_more_specific_candidate_wins_at_an_implicit() {
    let errs = errors(
        "fn order<T>(a: T, b: T) -> Int { return 0 }\n\
         fn order(a: Int, b: Int) -> Int { return 0 }\n\
         fn pick<T>(a: T, b: T, ?order: (T, T) -> Int) -> Int => !a, !b {\n\
         return order(a, b)\n\
         }\n\
         fn probe() -> Int {\n\
         return pick(1, 2)\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected the concrete `order` to win, got: {errs:?}");
}

/// [implicit-resolve] [fn-overload-ambiguous] No single most specific
/// candidate is an ambiguity naming the candidates and the remedies an
/// implicit has — the override naming the place, or `rename fn` — rather than
/// a guess.
#[test]
fn an_ambiguous_implicit_says_so() {
    let errs = errors(
        "fn order(a: Int | Bool, b: Int) -> Int { return 0 }\n\
         fn order(a: Int, b: Int | Bool) -> Int { return 0 }\n\
         fn pick<T>(a: T, b: T, ?order: (T, T) -> Int) -> Int => !a, !b {\n\
         return order(a, b)\n\
         }\n\
         fn probe() -> Int {\n\
         return pick(1, 2)\n\
         }\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("`order` is ambiguous for `pick` at `?order: (Int, Int) -> Int`")
            && e.contains("`order(Int | Bool, Int)` from `main`")
            && e.contains("`order(Int, Int | Bool)` from `main`")
            && e.contains("none is more specific")
            && e.contains("rename fn <new> = order(...)")),
        "expected the ambiguity error, got: {errs:?}"
    );
}

// ===== [effect-handler-generics] a `use` may write its handler's types =====
//
// A handler with no constructor argument has nothing to infer its generics
// from, so the `use` site's written type arguments are the only source. They
// used to be discarded silently, which both backends then turned into invalid
// target code — see COMPLETED.md's closed-defect entry, fixed 2026-09-06.

const HANDLER_PRELUDE: &str = r#"
effect Show<T> {
    fn show(v: T) -> Str => v
}

handler Plain<T> of Show<T> {
    fn show(v: T) -> Str => v { return "plain" }
}

// [effect-state-store] The member does not *return* `prefix`: a handler's own
// storage outlives every member call, so handing it out is `copy(prefix)` —
// which these tests cannot write (their prelude has no std). What they are
// about is the type argument a constructor argument binds, so a literal answer
// says the same thing.
handler Prefixed<T>(prefix: Str) of Show<T> {
    fn show(v: T) -> Str => v { return "p" }
}
"#;

/// The written arguments make the instance concrete, which is what a member
/// call resolves against.
#[test]
fn a_use_can_write_its_handlers_type_arguments() {
    let errs = errors(&format!(
        "{HANDLER_PRELUDE}\n\
         fn probe() [use] -> Str {{\n\
         use Plain<Int>()\n\
         return show(1)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A constructor argument still binds them, as it always did.
#[test]
fn a_constructor_argument_still_binds_them() {
    let errs = errors(&format!(
        "{HANDLER_PRELUDE}\n\
         fn probe() [use] -> Str {{\n\
         use Prefixed<Int>(\"p\")\n\
         return show(1)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}

/// Written arguments that disagree with what an argument implies are an
/// error, not a silent winner either way.
#[test]
fn written_type_arguments_must_agree_with_the_constructor() {
    let errs = errors(&format!(
        "{HANDLER_PRELUDE}\n\
         handler Echo<T>(seed: T) of Show<T> {{\n\
         fn show(v: T) -> Str => v {{ return \"e\" }}\n\
         }}\n\
         fn probe() [use] -> Str {{\n\
         use Echo<Str>(1)\n\
         return show(\"x\")\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("but an argument makes it")),
        "expected the disagreement to be reported, got: {errs:?}"
    );
}

/// And the wrong *number* of them is reported against the handler.
#[test]
fn the_wrong_number_of_type_arguments_is_rejected() {
    let errs = errors(&format!(
        "{HANDLER_PRELUDE}\n\
         fn probe() [use] -> Str {{\n\
         use Plain<Int, Str>()\n\
         return show(1)\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("takes 1 type argument(s), found 2")),
        "expected the arity error, got: {errs:?}"
    );
}


// ===== B3 piece 1: implicits selected as a call selects (R10) =====

/// [implicit-infer] Inference from an implicit runs selection and reads the
/// variable off the winner (R10 D22): of `f(x: Int) -> Str` and `f<T>(x: T)
/// -> Str`, both fitting `?f: (x: Int) -> B`, the concrete one is more specific
/// and teaches `B = Str`. Before, the matcher wanted exactly one candidate to
/// fit, so an un-annotated `conv(1)` could not infer `B`.
#[test]
fn inference_from_an_implicit_reads_the_most_specific_candidate() {
    let errs = errors(
        "fn f(x: Int) [] -> Str => x { return \"int\" }\n\
         fn f<T>(x: T) [] -> Str => x { return \"any\" }\n\
         fn conv<B>(x: Int, ?f: (x: Int) -> B) [] -> B => x { return f(x) }\n\
         fn takes(s: Str) [] -> Str => !s { return s }\n\
         fn probe() [] -> Str {\n\
         let s = conv(1)\n\
         return takes(s)\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected `B = Str` to be inferred, got: {errs:?}");
}

/// [implicit-infer] Two candidates neither of which is more specific teach
/// nothing: the ambiguity is the fill's to report, as one diagnostic.
#[test]
fn inference_from_an_ambiguous_implicit_teaches_nothing() {
    let errs = errors(
        "fn f(x: Int | Bool) [] -> Str => x { return \"a\" }\n\
         fn f(x: Int | Str) [] -> Int => x { return 1 }\n\
         fn conv<B>(x: Int, ?f: (x: Int) -> B) [] -> B => x { return f(x) }\n\
         fn probe() [] -> Str {\n\
         let s = conv(1)\n\
         return \"\"\n\
         }\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("cannot infer type argument `B` of `conv`")),
        "expected `B` to stay unknown, got: {errs:?}"
    );
}

/// [implicit-same-name] Two implicits of one name are ranked like overloads
/// (R10 D21): `tag(n)` with `n: Int` fits both `?tag: (x: Int) -> Str` and
/// `?tag: (x: Int | Str) -> Str`, and the `Int` one is more specific. Before,
/// "could go through more than one implicit of that name".
#[test]
fn two_implicits_of_one_name_are_ranked_like_overloads() {
    let errs = errors(
        "fn both(n: Int, ?tag: (x: Int) -> Str, ?tag: (x: Int | Str) -> Str) [] -> Str => n {\n\
         return tag(n)\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected the `Int` implicit to win, got: {errs:?}");
}

/// [implicit-same-name] [fn-overload-ambiguous] …and a tie is still the
/// ordinary ambiguity.
#[test]
fn two_implicits_of_one_name_neither_more_specific_are_ambiguous() {
    let errs = errors(
        "fn both(n: Int, ?tag: (x: Int, y: Int | Str) -> Str, ?tag: (x: Int | Str, y: Int) -> Str) [] -> Str => n {\n\
         return tag(n, n)\n\
         }\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("could go through more than one implicit of that name")
            && e.contains("none is more specific")),
        "expected the ambiguity, got: {errs:?}"
    );
}

const FMT_EFFECT: &str = "struct Point { x: Int }\n\
     effect Fmt {\n\
     fn to_str(p: Point) -> Str => p\n\
     }\n\
     handler Plain of Fmt {\n\
     fn to_str(p: Point) -> Str { return \"p\" }\n\
     }\n\
     fn render(p: Point, ?to_str: (p: Point) -> Str) [] -> Str => p { return to_str(p) }\n";

/// [effect-member-overload] [effect-available] An available effect's member
/// fills an implicit position (R10 D3: an effect's members fill every
/// position a fn does), passed as a value that carries its handler — so the
/// pure `render` takes it. Before, "no `to_str` fits".
#[test]
fn an_available_effect_member_fills_an_implicit() {
    let errs = errors(&format!(
        "{FMT_EFFECT}\
         fn show(p: Point) [Fmt] -> Str => p {{ return render(p) }}\n\
         fn probe() [use] -> Str {{\n\
         use Plain()\n\
         return render(Point {{ x: 1 }})\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected the member to fill `?to_str`, got: {errs:?}");
}

/// [effect-available] With no instance in scope the member is no candidate.
#[test]
fn an_unavailable_effect_member_fills_nothing() {
    let errs = errors(&format!("{FMT_EFFECT}fn probe() [] -> Str {{ return render(Point {{ x: 1 }}) }}\n"));
    assert!(errs.iter().any(|e| e.contains("no `to_str` for this call")), "got: {errs:?}");
}

/// [effect-member-overload] [fn-overload-ambiguous] A member and a fn of one
/// signature tie, and the remedies name both — the member as a value,
/// `to_str@Fmt`, included — which then settle it.
#[test]
fn a_member_and_a_fn_of_one_signature_are_ambiguous_at_an_implicit() {
    let fn_too = "fn to_str(p: Point) [] -> Str => p { return \"fn\" }\n";
    let errs = errors(&format!(
        "{FMT_EFFECT}{fn_too}fn show(p: Point) [Fmt] -> Str => p {{ return render(p) }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`to_str` is ambiguous for `render`")
            && e.contains("`to_str(Point)` from `Fmt`")
            && e.contains("`to_str = to_str@Fmt` or `to_str = to_str@main`")),
        "expected the ambiguity naming both remedies, got: {errs:?}"
    );
    let errs = errors(&format!(
        "{FMT_EFFECT}{fn_too}fn show(p: Point) [Fmt] -> Str => p {{\n\
         let a = render(p, to_str = to_str@Fmt)\n\
         return render(p, to_str = to_str@main)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected both remedies to work, got: {errs:?}");
}

/// [effect-member-overload] [implicit-infer] A member teaches as it fills:
/// `?f: (x: Int) -> B` filled by an available member `f(x: Int) -> Str`
/// infers `B = Str`.
#[test]
fn inference_reads_an_available_member() {
    let errs = errors(
        "effect Conv {\n\
         fn f(x: Int) -> Str => x\n\
         }\n\
         fn conv<B>(x: Int, ?f: (x: Int) -> B) [] -> B => x { return f(x) }\n\
         fn takes(s: Str) [] -> Str => !s { return s }\n\
         fn probe() [Conv] -> Str {\n\
         let s = conv(1)\n\
         return takes(s)\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected `B = Str` from the member, got: {errs:?}");
}
