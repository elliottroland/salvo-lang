//! Member-resolution rules [call-resolve] [field-resolve] [index-resolve]
//! [iter-resolve]: Salvo assumes it can see everything, so a call, field
//! read, subscript, or `for` over something the compiler cannot justify is
//! an error. Reaching a target-language feature means *declaring* it
//! (`external fn` and friends) — user decision 2026-09-03, which replaced
//! the interop pass-through that used to make these silent.
//!
//! The narrow leniency that remains is [type-unknown-lenient]: a type the
//! checker could not *infer* (`Ty::Unknown`) still passes through, so one
//! mistake never cascades.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type Opaque\n";

fn check_errors(src: &str) -> Vec<FileDiagnostic> {
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
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    errors.extend(check_program(&program, &resolution, &symbols).errors);
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and this file is about which references are rejected.
    errors.retain(|d| d.is_error());
    errors
}

fn messages(src: &str) -> Vec<String> {
    check_errors(src)
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

struct Person {
    name: Str
}

fn shout(s: Str) [] -> [s] Str { return "" }
"#;

fn body(src: &str) -> String {
    format!("{PRELUDE}\nfn probe(p: Person) -> [p] Int {{\n{src}\n    return 1\n}}\n")
}

// ===== calls [call-resolve] =====

/// [call-resolve] A name nothing declares is not a call into the target
/// language any more — it is a mistake.
#[test]
fn unresolved_call_is_an_error() {
    let errs = messages(&body("    nowhere()"));
    assert!(
        errs.iter()
            .any(|m| m == "no function named `nowhere` is in scope"),
        "got {errs:?}"
    );
}

/// [call-resolve] [diag-import-suggest] Unresolved calls carry import
/// suggestions, like every other unresolved name.
#[test]
fn unresolved_call_suggests_imports() {
    let lib = "fn helper() -> Int {\n    return 1\n}\n";
    let main = "fn f() -> Int {\n    return helper()\n}\n";
    let mut sources = SourceSet::default();
    for (name, src) in [("lib.sv", lib), ("main.sv", main)] {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(name, module, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(
            !diagnostics.iter().any(|d| d.is_error()),
            "parse errors: {diagnostics:?}"
        );
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let errors = check_program(&program, &resolution, &symbols).errors;
    let diag = errors
        .iter()
        .find(|d| d.message.contains("no function named `helper`"))
        .unwrap_or_else(|| panic!("got {errors:?}"));
    assert_eq!(diag.suggested_imports, vec!["lib.helper".to_string()]);
}

/// [call-resolve] Calling a value whose type is known and is not a
/// function: the old leniency emitted `n()` and let kotlinc/rustc reject
/// it.
#[test]
fn calling_a_non_fn_value_is_an_error() {
    let errs = messages(&body("    let n = 5\n    n()"));
    assert!(
        errs.iter()
            .any(|m| m == "`n` is not callable: its type is `Int`"),
        "got {errs:?}"
    );
}

/// [call-resolve] A generic parameter is opaque: Salvo has no bounds, so
/// nothing makes a `T` callable. The diagnostic says *that*, rather than
/// pointing at a declaration that could not help.
#[test]
fn calling_a_generic_value_is_an_error() {
    let src = format!(
        "{PRELUDE}\nfn probe<T>(f: T) -> [f] Int {{\n    f()\n    return 1\n}}\n"
    );
    let errs = messages(&src);
    let diag = errs
        .iter()
        .find(|m| m.contains("`f` is a type parameter"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("Declare it with a fn type"),
        "the diagnostic should name the remedy: {diag}"
    );
}

/// [call-resolve] [fn-dot] Dot-notation is sugar for a function call, so an
/// undeclared method name is unresolved — the diagnostic says how to reach
/// a target-language method.
#[test]
fn unresolved_dot_call_is_an_error() {
    let errs = messages(&body("    p.missing_method()"));
    let diag = errs
        .iter()
        .find(|m| m.contains("no function named `missing_method`"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("`platform effect`"),
        "the diagnostic should name the remedy: {diag}"
    );
}

/// [call-resolve] A *declared* function is reachable by dot-notation, which
/// is how a member-looking call is spelled.
#[test]
fn a_declared_fn_is_callable_by_dot_notation() {
    let errs = messages(&body("    let s = \"x\".shout()"));
    assert!(errs.is_empty(), "got {errs:?}");
}

// ===== fields [field-resolve] =====

/// [field-resolve] Only structs have fields.
#[test]
fn field_on_a_non_struct_is_an_error() {
    let errs = messages(&body("    let n = 5\n    let x = n.whatever"));
    assert!(
        errs.iter().any(|m| m.starts_with("`Int` has no field `whatever`")),
        "got {errs:?}"
    );
}

/// [field-resolve] An opaque (interop) type exposes nothing by itself: its
/// members are reached through declared accessors.
#[test]
fn field_on_an_opaque_type_is_an_error() {
    let src = format!(
        "{PRELUDE}\nfn probe(o: Opaque) -> [o] Int {{\n    let x = o.member\n    return 1\n}}\n"
    );
    let errs = messages(&src);
    assert!(
        errs.iter().any(|m| m.starts_with("`Opaque` has no field `member`")),
        "got {errs:?}"
    );
}

/// [field-resolve] A generic value is opaque too — and its diagnostic must
/// not suggest a declared accessor, which cannot help for a `T`.
#[test]
fn field_on_a_generic_is_an_error() {
    let src = format!(
        "{PRELUDE}\nfn probe<T>(v: T) -> [v] Int {{\n    let x = v.name\n    return 1\n}}\n"
    );
    let errs = messages(&src);
    let diag = errs
        .iter()
        .find(|m| m.contains("`T` is a type parameter"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("no field `name`") && diag.contains("concrete type"),
        "the diagnostic should name the remedy: {diag}"
    );
    assert!(
        !diag.contains("external fn"),
        "an accessor cannot help for a type parameter: {diag}"
    );
}

// ===== subscripts [index-resolve] =====

/// [index-resolve] `[]` is for arrays; other collections expose element
/// access as functions.
#[test]
fn indexing_a_non_array_is_an_error() {
    let errs = messages(&body("    let n = 5\n    let x = n[0]"));
    assert!(
        errs.iter()
            .any(|m| m.starts_with("`Int` cannot be indexed with `[]`")),
        "got {errs:?}"
    );
}

/// [index-resolve] Arrays still work, of course.
#[test]
fn indexing_an_array_is_fine() {
    let errs = messages(&body("    let xs: Int[] = [1, 2]\n    let x = xs[0]"));
    assert!(errs.is_empty(), "got {errs:?}");
}

// ===== iteration [iter-resolve] =====

/// [iter-resolve] `for` needs an array, a pass with a `next`, or a value
/// some `iter` function accepts.
#[test]
fn iterating_a_non_iterable_is_an_error() {
    let errs = messages(&body("    let n = 5\n    for x in n {\n        let y = x\n    }"));
    assert!(
        errs.iter().any(|m| m.starts_with("`Int` is not iterable")),
        "got {errs:?}"
    );
}

/// [iter-resolve] Arrays iterate.
#[test]
fn iterating_an_array_is_fine() {
    let errs = messages(&body(
        "    let xs: Int[] = [1, 2]\n    for x in xs {\n        let y = x\n    }",
    ));
    assert!(errs.is_empty(), "got {errs:?}");
}

// ===== effects and handlers are not data [effect-not-data]
// ===== [handler-not-value] =====

const EFFECT_PRELUDE: &str = r#"

effect Counter {
    fn bump() -> [] Int
}

handler MemCounter of Counter {
    n: Int = 0

    fn bump() -> [] Int {
        n = n + 1
        return n
    }
}
"#;

fn effect_messages(src: &str) -> Vec<String> {
    messages(&format!("{EFFECT_PRELUDE}\n{src}"))
}

/// [effect-handler] A state field's initializer is checked against its
/// declared type, like a struct field's default. Until 2026-09-04 it was not
/// checked *at all*, which both hid type errors and left the emitters with
/// no types for the expression, which an intrinsic lowering may need in
/// order to spell a type out [backend-intrinsic].
#[test]
fn handler_state_initializers_are_checked() {
    let errs = messages(
        "\n\
         effect Sink {\n    fn kept() -> [] Int\n}\n\n\
         handler Bin of Sink {\n    held: Int = \"not an int\"\n\n    \
         fn kept() -> [] Int {\n        return held\n    }\n}\n",
    );
    let diag = errs
        .iter()
        .find(|m| m.contains("expected `Int`, found `Str`"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(diag.contains("expected `Int`"), "{diag}");
}

#[test]
fn a_well_typed_handler_state_initializer_is_accepted() {
    let errs = effect_messages("fn main() [use] -> [] Int {\n    use MemCounter\n    return bump()\n}\n");
    assert!(errs.is_empty(), "got {errs:?}");
}

/// [effect-not-data] An effect names a capability, not a type of values.
/// Every data position rejects it; the diagnostic names the two positions
/// that *do* accept an effect, and `use`.
#[test]
fn effect_types_are_rejected_in_data_positions() {
    let cases = [
        ("struct S {\n    c: Counter\n}\n", "struct field"),
        ("fn f(c: Counter) -> [c] Int {\n    return 1\n}\n", "parameter"),
        ("fn f() -> Counter {\n    return 1\n}\n", "return type"),
        (
            "fn f() -> Int {\n    let c: Counter = 1\n    return 1\n}\n",
            "`let` annotation",
        ),
        ("type Alias = Counter\n", "type alias"),
    ];
    for (src, what) in cases {
        let errs = effect_messages(src);
        let diag = errs
            .iter()
            .find(|m| m.starts_with("`Counter` is an effect, not a data type"))
            .unwrap_or_else(|| panic!("no rejection for a {what}: {errs:?}"));
        assert!(
            diag.contains("`use`"),
            "the diagnostic should point at `use`: {diag}"
        );
    }
}

/// [effect-not-data] The two positions that legitimately name an effect —
/// a fn's effect list and a handler's `of` clause — keep working.
#[test]
fn effect_lists_and_of_clauses_still_accept_effects() {
    let errs = effect_messages("fn f() [Counter] -> Int {\n    return bump()\n}\n");
    assert!(errs.is_empty(), "got {errs:?}");
}

/// [handler-not-value] A handler instance comes from `use`; it is not a
/// value to bind, return, or pass. Rust could not render one anyway
/// (`Name()` is not a constructor — `E0423`).
#[test]
fn handler_constructors_are_not_values() {
    let errs = effect_messages(
        "fn f() -> Int {\n    let c = MemCounter()\n    return 1\n}\n",
    );
    let diag = errs
        .iter()
        .find(|m| m.starts_with("`MemCounter` is a handler, not a value"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("use MemCounter(...)"),
        "the diagnostic should name the remedy: {diag}"
    );
}

/// [handler-not-value] `use` itself is unaffected: it validates its own
/// constructor call and never goes through the value path.
#[test]
fn use_still_registers_handlers() {
    let errs = effect_messages(
        "fn main() [use] -> [] Int {\n    use MemCounter()\n    return bump()\n}\n",
    );
    assert!(errs.is_empty(), "got {errs:?}");
}

/// [effect-handler-deps] A constructor parameter of effect type is the one
/// position where an effect names something a handler may hold: it is a
/// *dependency*, and the member bodies may use that effect.
#[test]
fn handler_dependencies_are_accepted() {
    let src = "effect Logger {\n    fn log(m: Str) -> [m] None\n}\n\n\
               handler CountingLogger(counter: Counter) of Logger {\n    \
               fn log(m: Str) -> [m] None {\n        let n = bump()\n    }\n}\n";
    let errs = effect_messages(src);
    assert!(errs.is_empty(), "got {errs:?}");
}

/// [effect-handler-deps] A handler cannot depend on the effect it
/// implements: registering it would require itself.
#[test]
fn handler_cannot_depend_on_its_own_effect() {
    let errs = effect_messages(
        "handler Wrapper(inner: Counter) of Counter {\n    \
         fn bump() -> [] Int {\n        return 0\n    }\n}\n",
    );
    assert!(
        errs.iter().any(|m| m
            == "handler `Wrapper` cannot depend on `Counter`, the effect it implements"),
        "got {errs:?}"
    );
}

/// [effect-handler-deps] Dependency *cycles* need no separate check: a
/// dependency must already be registered, so a cycle cannot be constructed
/// in either order. Verified both ways round, since "it happens to fail"
/// and "it cannot be written" are different claims.
#[test]
fn dependency_cycles_cannot_be_registered() {
    let cyclic = "\n\
        effect Alpha {\n    fn a(m: Str) -> [m] None\n}\n\n\
        effect Beta {\n    fn b(m: Str) -> [m] None\n}\n\n\
        handler AlphaViaBeta(beta: Beta) of Alpha {\n    \
        fn a(m: Str) -> [m] None {\n        b(m)\n    }\n}\n\n\
        handler BetaViaAlpha(alpha: Alpha) of Beta {\n    \
        fn b(m: Str) -> [m] None {\n        a(m)\n    }\n}\n";
    for (first, second, blamed) in [
        ("AlphaViaBeta", "BetaViaAlpha", "Beta"),
        ("BetaViaAlpha", "AlphaViaBeta", "Alpha"),
    ] {
        let src = format!(
            "{cyclic}\nfn main() [use] -> [] None {{\n    use {first}()\n    \
             use {second}()\n}}\n"
        );
        let errs = messages(&src);
        assert!(
            errs.iter().any(|m| m.contains(&format!(
                "handler `{first}` depends on effect `{blamed}`, which has no handler"
            ))),
            "registering `{first}` first should fail: {errs:?}"
        );
    }
}

/// [effect-handler-deps] Dependencies are supplied by the compiler from the
/// enclosing scope, so they are not written at the `use` site — and one must
/// be available there.
#[test]
fn handler_dependencies_come_from_the_use_scope() {
    let prelude = "effect Logger {\n    fn log(m: Str) -> [m] None\n}\n\n\
                   handler CountingLogger(counter: Counter) of Logger {\n    \
                   fn log(m: Str) -> [m] None {\n        let n = bump()\n    }\n}\n";
    // Registered *after* its dependency: fine, and no argument is written.
    let ok = effect_messages(&format!(
        "{prelude}\nfn main() [use] -> [] None {{\n    use MemCounter()\n    \
         use CountingLogger()\n    log(\"x\")\n}}\n"
    ));
    assert!(ok.is_empty(), "got {ok:?}");

    // Without the dependency in scope: rejected at the registration.
    let missing = effect_messages(&format!(
        "{prelude}\nfn main() [use] -> [] None {{\n    use CountingLogger()\n    \
         log(\"x\")\n}}\n"
    ));
    assert!(
        missing.iter().any(|m| m.contains(
            "handler `CountingLogger` depends on effect `Counter`, which has no handler"
        )),
        "got {missing:?}"
    );
}


// ===== the leniency that remains =====

/// [type-unknown-lenient] A type the checker could not *infer* still passes
/// through: one mistake must not cascade. Here the unresolved call is
/// reported once, and reading a member of its `Unknown` result adds
/// nothing.
#[test]
fn uninferred_types_still_pass_through() {
    let errs = messages(&body("    let v = nowhere()\n    let x = v.member"));
    assert_eq!(
        errs.len(),
        1,
        "expected only the unresolved call to be reported: {errs:?}"
    );
    assert!(errs[0].contains("no function named `nowhere`"), "got {errs:?}");
}

// ===== identifier references [ident-resolve] =====

/// [ident-resolve] [backend-never-wrong] A bare name nothing declares used to
/// pass the checker as `Ty::Unknown` and reach the emitters, which spelled it
/// straight into the output — so the *target* compiler reported it
/// (`E0425` / "unresolved reference"). It is a Salvo error now: the same rule
/// as a call, a field or a subscript, which this file exists to pin.
#[test]
fn an_undeclared_identifier_is_an_error() {
    let errs = messages(&body("    let n = undeclared_thing"));
    assert!(
        errs.iter()
            .any(|m| m == "no variable or function named `undeclared_thing` is in scope"),
        "got {errs:?}"
    );
}

/// [ident-resolve] Reading a variable *before* its declaration is the same
/// mistake: scopes are not hoisted.
#[test]
fn using_a_variable_before_its_declaration_is_an_error() {
    let errs = messages(&body("    let a = later\n    let later = 1"));
    assert!(
        errs.iter()
            .any(|m| m == "no variable or function named `later` is in scope"),
        "got {errs:?}"
    );
}

/// [ident-resolve] A name that *is* declared but is not a value says what it
/// is, rather than claiming nothing declares it — the courtesy the effect and
/// handler rules already extend [effect-not-a-type].
#[test]
fn a_declared_non_value_name_says_what_it_is() {
    let errs = messages(&body("    let s = Person"));
    assert!(
        errs.iter().any(|m| m.contains("`Person` is a struct type, not a value")),
        "got {errs:?}"
    );
}

/// [ident-resolve] [type-unknown-lenient] One mistake, one diagnostic: the
/// unresolved name's `Unknown` type does not cascade into the read below it.
#[test]
fn an_undeclared_identifier_does_not_cascade() {
    let errs = messages(&body("    let v = undeclared_thing\n    let x = v.member"));
    assert_eq!(
        errs.len(),
        1,
        "expected only the unresolved name to be reported: {errs:?}"
    );
}

// ===== unused variables [unused-var] =====

/// All diagnostics, warnings included — `check_errors` filters to errors, and
/// the point here is the warning.
fn all_messages(src: &str) -> Vec<String> {
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
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(
            diagnostics.iter().all(|d| !d.is_error()),
            "parse errors: {diagnostics:?}"
        );
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    check_program(&program, &resolution, &symbols)
        .errors
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// [unused-var] A local that is never read warns — and says how to opt out.
#[test]
fn an_unused_local_warns() {
    let msgs = all_messages(&body("    let spare = 1"));
    assert!(
        msgs.iter().any(|m| m
            == "`spare` is never used; prefix it with `_` (`_spare`) if that is deliberate"),
        "got {msgs:?}"
    );
}

/// [unused-var] A leading `_` is the opt-out, and a read is a use.
#[test]
fn an_underscore_prefix_and_a_read_are_both_silent() {
    let msgs = all_messages(&body(
        "    let _spare = 1\n    let used = 2\n    let echo = used",
    ));
    assert!(
        !msgs.iter().any(|m| m.contains("`_spare`") || m.contains("`used`")),
        "expected no warning for `_spare` or `used`: {msgs:?}"
    );
    // `echo` itself is never read, so it *does* warn — the rule is uniform.
    assert!(
        msgs.iter().any(|m| m.contains("`echo` is never used")),
        "got {msgs:?}"
    );
}

/// [unused-var] Assignment is not a use: a variable only ever written to has
/// no reader, which is exactly the mistake worth reporting.
#[test]
fn a_variable_that_is_only_assigned_warns() {
    let msgs = all_messages(&body("    let counter = 1\n    counter = 2"));
    assert!(
        msgs.iter().any(|m| m.contains("`counter` is never used")),
        "got {msgs:?}"
    );
}

/// [unused-var] A *parameter* never warns: a signature often dictates it —
/// an effect or handler member implementing a declared interface cannot drop
/// one — so the warning would fire where nothing can be done about it.
#[test]
fn an_unused_parameter_does_not_warn() {
    let msgs = all_messages("fn takes(a: Int, b: Int) -> [] Int {\n    return a\n}\n");
    assert!(
        !msgs.iter().any(|m| m.contains("never used")),
        "expected no parameter warning: {msgs:?}"
    );
}
