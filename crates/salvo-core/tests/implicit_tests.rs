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
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n";

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
         fn total<T>(a: T, ?Field<T>) -> [] T {{\n\
         return add(a, zero())\n\
         }}\n\
         fn probe() -> [] Int {{\n\
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
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> [] Int {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> [] Int {{\n\
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
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> [] Int {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe(s: Str) -> [] Int {{\n\
         return pick(s, s)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("no `cmp` for this call")
                && e.contains("declare a matching `fn cmp`")
                && e.contains("cmp = ...")
        }),
        "expected the unresolved-implicit error with both remedies, got: {errs:?}"
    );
}

// ===== [implicit-override] `name = value` at the call site =====

/// The caller can replace one member of a group by its own name — the point
/// of dropping the binder.
#[test]
fn a_call_can_override_one_member_by_name() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn total<T>(a: T, ?Field<T>) -> [] T {{\n\
         return add(a, zero())\n\
         }}\n\
         fn probe() -> [] Int {{\n\
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
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> [] Int {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> [] Int {{\n\
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
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> [] Int {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> [] Int {{\n\
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
         fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> [] Int {{\n\
         return cmp(a, b)\n\
         }}\n\
         fn probe() -> [] Int {{\n\
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
         fn total<T>(a: T, ?Field<T>) -> [] T {{\n\
         return add(a, zero())\n\
         }}\n\
         fn total_twice<T>(a: T, b: T, ?Field<T>) -> [] T {{\n\
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
         fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> [] T {{\n\
         return add(a, b)\n\
         }}\n\
         fn total<T>(a: T, ?Field<T>) -> [] T {{\n\
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
         fn total<T>(a: T, ?Field<T>) -> [] T {{\n\
         return add(a, zero())\n\
         }}\n\
         fn careless<T>(a: T) -> [] T {{\n\
         return total(a)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no `add` for this call")),
        "expected the missing-implicit error, got: {errs:?}"
    );
}

// ===== declaration-site rules =====

/// [implicit-param] Only a function can be resolved by name and type.
#[test]
fn an_implicit_of_non_fn_type_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(a: Int, ?limit: Int) -> [] Int {{\n\
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
         fn f<T>(a: T, ?Ring<T>) -> [] T {{\n\
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
         fn f<A, B>(a: A, b: B, ?Field<A>, ?Field<B>) -> [] A {{\n\
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
         fn f<A, B>(a: A, ?Field<A, B>) -> [] A {{\n\
         return a\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("takes 1 type argument(s), found 2")),
        "expected the arity error, got: {errs:?}"
    );
}

/// [implicit-fn-only] An effect member has no call site that could resolve
/// an implicit — it is reached through a handler.
#[test]
fn an_effect_member_cannot_take_implicits() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         effect Sorter {{\n\
         fn sorted(a: Int, ?cmp: (Int, Int) -> Int) -> [] Int\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("an effect member cannot take implicit parameters")),
        "expected the fn-only rejection, got: {errs:?}"
    );
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
