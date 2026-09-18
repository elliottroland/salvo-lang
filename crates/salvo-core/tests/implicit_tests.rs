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
//! `?Field<T>` spreads a `params` group. It has no binder (user decision
//! 2026-09-05): the members become implicit parameters in their own right,
//! so resolution, forwarding and override all key on a member's own name
//! and type, and a callee can reach the same parameter through a different
//! grouping — or none.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n";

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
            "unexpected parse errors: {parse_errors:?}"
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
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return 0 }
fn zero() -> Int { return 0 }
fn cmp(a: Int, b: Int) -> Int { return 0 }
fn times(a: Int, b: Int) -> Int { return 0 }
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
            .any(|e| e.contains("no `params` group named `Ring`")),
        "expected the unknown-group error, got: {errs:?}"
    );
}

/// Two implicits of the same name cannot both be filled — with no binder
/// there is nothing to tell them apart, and [var-no-shadow] would refuse
/// them in the body. The remedy is to write the clashing ones out.
#[test]
fn two_implicits_of_the_same_name_are_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f<A, B>(a: A, b: B, ?Field<A>, ?Field<B>) -> A => !a, !b {{\n\
         return a\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("is declared as an implicit parameter twice")
                && e.contains("individually under distinct names")
        }),
        "expected the duplicate-name rejection with its remedy, got: {errs:?}"
    );
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

/// A `params` group member is a signature for a parameter; the default comes
/// from a matching top-level fn, so a body here means nothing.
#[test]
fn a_group_member_with_a_body_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         params Ring<T> {{\n\
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
        "params Field<T> {\n\
         fn add(a: T, b: T) -> T\n\
         fn zero() -> T\n\
         }\n\
         fn add(a: Int, b: Int) -> Int { return a }\n\
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
         fn eats(a: Int, b: Int) -> Int {{ return a }}\n\
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

/// Ambiguity says how many matched and offers the override, rather than
/// guessing.
#[test]
fn an_ambiguous_implicit_says_so() {
    let errs = errors(
        "fn cmp<T>(a: T, b: T) -> Int { return 0 }\n\
         fn cmp(a: Int, b: Int) -> Int { return 0 }\n\
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {\n\
         return cmp(a, b)\n\
         }\n\
         fn probe() -> Int {\n\
         return pick(1, 2)\n\
         }\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("is ambiguous for") && e.contains("declarations match")),
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
