//! Structured-diagnostic tests [diag-structured]: resolver and checker
//! errors carry a file index, span, and severity instead of pre-rendered
//! strings, and render at the boundary.

use std::path::Path;

use salvo_core::{check_program, resolve, Checked, Program, SourceSet, Symbols};

/// [intrinsic-std-only] Base types the checker needs, loaded as a std file
/// (module `std.core.prelude`, implicitly imported) rather than pasted into the
/// sources under test. Added *after* the user files so their file indices —
/// which these tests assert on — stay stable.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

/// Parses + resolves + checks a multi-file program. Each entry is
/// `(file_name, source)`; a std prelude supplies the base types.
fn check_files(files: &[(&str, &str)]) -> (Program, Checked) {
    let mut sources = SourceSet::default();
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        modules.push(module);
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
    (program, checked)
}

// [diag-structured] Checker errors are attributed to the declaring file
// with a nonempty span, and render with file:line:col + caret.
#[test]
fn checker_errors_carry_file_and_span() {
    // The declarations are supplied by the std prelude, so the asserted
    // spans in `main.sv` stay put.
    let src = "fn broken() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n";
    let (program, checked) = check_files(&[("main.sv", src)]);
    assert_eq!(checked.errors.len(), 1, "errors: {:?}", checked.errors);
    let diag = &checked.errors[0];
    assert_eq!(diag.file, 0);
    assert!(diag.is_error());
    assert!(diag.message.contains("expected `Int`, found `Str`"));
    // The span points at the string literal on line 2.
    assert_eq!(
        &src[diag.span.start as usize..diag.span.end as usize],
        "\"hello\""
    );
    let rendered = diag.render(&program.files);
    assert!(
        rendered.contains("main.sv:2:18"),
        "unexpected rendering: {rendered}"
    );
    assert!(rendered.contains('^'), "no caret line: {rendered}");
}

// [diag-structured] Errors from different files of one program index the
// right file; resolution errors flow into `Checked::errors` structured.
#[test]
fn errors_index_the_declaring_file() {
    let ok = "export fn fine() -> Int {\n    return 1\n}\n";
    let bad = "import nope.thing\n";
    let (program, checked) = check_files(&[("a.sv", ok), ("b.sv", bad)]);
    assert_eq!(checked.errors.len(), 1, "errors: {:?}", checked.errors);
    let diag = &checked.errors[0];
    assert_eq!(diag.file, 1);
    assert!(diag.message.contains("unresolved import"));
    assert!(
        diag.render(&program.files).contains("b.sv:1:1"),
        "unexpected rendering: {}",
        diag.render(&program.files)
    );
}

// ===== [mod-collision] name collisions error instead of last-win =====

/// Resolves a multi-file program and returns the resolution errors.
fn resolve_errors(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        modules.push(module);
    }
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    resolution
        .errors
        .iter()
        .map(|d| d.render(&program.files))
        .collect()
}

// [mod-collision] Two imports bringing the same visible name collide.
#[test]
fn conflicting_imports_are_an_error() {
    let errors = resolve_errors(&[
        ("a.sv", "export struct Person {\n    name: Str\n}\n"),
        ("b.sv", "export struct Person {\n    age: Int\n}\n"),
        (
            "main.sv",
            "import a.Person\nimport b.Person\n\nfn main() -> None {\n}\n",
        ),
    ]);
    assert!(
        errors.iter().any(|e| e.contains("conflicts with the import from `a`")),
        "unexpected errors: {errors:?}"
    );
}

// [mod-collision] An import colliding with an own-module declaration.
#[test]
fn import_shadowing_own_declaration_is_an_error() {
    let errors = resolve_errors(&[
        ("a.sv", "export struct Person {\n    name: Str\n}\n"),
        (
            "main.sv",
            "import a.Person\n\nstruct Person {\n    age: Int\n}\n\nfn main() -> None {\n}\n",
        ),
    ]);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("conflicts with a declaration in this module")),
        "unexpected errors: {errors:?}"
    );
}

// [mod-collision] Aliasing one of the imports resolves the conflict.
#[test]
fn aliased_import_avoids_the_collision() {
    let errors = resolve_errors(&[
        ("a.sv", "export struct Person {\n    name: Str\n}\n"),
        ("b.sv", "export struct Person {\n    age: Int\n}\n"),
        (
            "main.sv",
            "import a.Person\nimport b.Person as AgedPerson\n\nfn main() -> None {\n}\n",
        ),
    ]);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [mod-collision] A same-kind duplicate within one module is an error.
#[test]
fn duplicate_declaration_in_module_is_an_error() {
    let errors = resolve_errors(&[(
        "main.sv",
        "struct Person {\n    name: Str\n}\n\nstruct Person {\n    age: Int\n}\n",
    )]);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("duplicate struct `Person` in module `main`")),
        "unexpected errors: {errors:?}"
    );
}

// [mod-collision] [fn-overload] Same-name fns are overloads, never
// collisions.
#[test]
fn same_name_fns_do_not_collide() {
    let errors = resolve_errors(&[
        ("a.sv", "export fn describe(x: Int) -> Str {\n    return \"int\"\n}\n"),
        (
            "main.sv",
            "import a.describe\n\nfn describe(x: Str) -> Str {\n    return \"str\"\n}\n",
        ),
    ]);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// ===== generic binding widening in unify =====

// A later argument may reveal the more general type: `pick(1, x)` with
// `x: Int?` binds `T = Int?` regardless of argument order — first-binding
// -wins would let the checker believe the result is plain `Int`.
#[test]
fn generic_binding_widens_to_the_general_type() {
    let src = "fn pick<T>(a: T, b: T) -> T {\n    return a\n}\n\n\
               fn main() -> None {\n    let x: Int? = None\n    let z: Int = pick(1, x)\n}\n";
    let (_, checked) = check_files(&[("main.sv", src)]);
    assert!(
        checked
            .errors
            .iter()
            .any(|e| e.message.contains("expected `Int`, found `Int?`")),
        "errors: {:?}",
        checked.errors
    );
}

#[test]
fn generic_binding_widens_in_either_order() {
    let src = "fn pick<T>(a: T, b: T) -> T {\n    return a\n}\n\n\
               fn main() -> None {\n    let x: Int? = None\n    let z: Int = pick(x, 1)\n}\n";
    let (_, checked) = check_files(&[("main.sv", src)]);
    assert!(
        checked
            .errors
            .iter()
            .any(|e| e.message.contains("expected `Int`, found `Int?`")),
        "errors: {:?}",
        checked.errors
    );
}

// Unrelated bindings still fail the overload (no accidental widening
// across incompatible types).
#[test]
fn incompatible_generic_bindings_still_reject() {
    let src = "fn pick<T>(a: T, b: T) -> T {\n    return a\n}\n\n\
               fn main() -> None {\n    let z = pick(1, \"s\")\n}\n";
    let (_, checked) = check_files(&[("main.sv", src)]);
    assert!(
        !checked.errors.is_empty(),
        "expected an overload-resolution error, got none"
    );
}

// ===== [op-no-none] [interp-no-none] optionals at operators and in
// interpolation are errors (user decisions 2026-09-02) =====

/// Runs the checker on a source *with std loaded* (the new rules only
/// fire on known types) and returns the rendered errors.
fn check_errors(src: &str) -> Vec<String> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let mut sources = SourceSet::default();
    let io_errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(io_errors.is_empty(), "failed to read std: {io_errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: sources.companions,
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and these tests are about which programs are rejected.
    checked
        .errors
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.render(&program.files))
        .collect()
}

// [op-no-none] Arithmetic, ordering, and equality all reject a
// possibly-`None` operand; `None` itself is rejected outright.
#[test]
fn optional_operands_are_rejected() {
    let errors = check_errors(
        "fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   let a = maybe + 1\n\
        \x20   let b = maybe < 2\n\
        \x20   let c = maybe == \"x\"\n\
        \x20   let d = 1 - maybe\n\
        \x20   let e = None * 2\n\
         }\n",
    );
    for (needle, count) in [
        ("operand of `+` may be `None`", 1),
        ("operand of `<` may be `None`", 1),
        ("operand of `==` may be `None`", 1),
        ("operand of `-` may be `None`", 1),
        ("`None` is not a valid operand of `*`", 1),
    ] {
        assert_eq!(
            errors.iter().filter(|e| e.contains(needle)).count(),
            count,
            "expected `{needle}` in: {errors:?}"
        );
    }
}

// [op-no-none] Narrowing or asserting makes the operand legal.
#[test]
fn narrowed_operands_are_accepted() {
    let errors = check_errors(
        "fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   if maybe is Int m {\n\
        \x20       let a = m + 1\n\
        \x20       let b = m < 2\n\
        \x20   }\n\
        \x20   let c = (maybe ?: unreachable!()) + 1\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [op-arm-eq] Equality against a value matching one arm is legal without
// narrowing: the checker reduces it to an arm test plus a comparison.
#[test]
fn equality_against_a_matching_arm_is_accepted() {
    let errors = check_errors(
        "fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   let a = maybe == 3\n\
        \x20   let b = maybe != 3\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [interp-no-none] Interpolating a possibly-`None` value is an error —
// including a struct field, which does not flow-narrow [is-narrowing].
#[test]
fn interpolating_an_optional_is_rejected() {
    let errors = check_errors(
        "struct Person {\n\
        \x20   name: Str,\n\
        \x20   surname: Str? = None\n\
         }\n\
         \n\
         fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   let p = Person {name: \"Ada\"}\n\
        \x20   let a = \"${maybe}\"\n\
        \x20   let b = \"${p.surname}\"\n\
        \x20   let c = \"${None}\"\n\
         }\n",
    );
    assert_eq!(
        errors
            .iter()
            .filter(|e| e.contains("cannot interpolate a value that may be `None`"))
            .count(),
        2,
        "errors: {errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("cannot interpolate `None` into a string")),
        "errors: {errors:?}"
    );
}

// [interp-no-none] The `is`-binding and `!` forms interpolate fine — and
// this is exactly the docs/language/ nullability idiom.
#[test]
fn interpolating_a_narrowed_optional_is_accepted() {
    let errors = check_errors(
        "struct Person {\n\
        \x20   name: Str,\n\
        \x20   surname: Str? = None\n\
         }\n\
         \n\
         fn f() -> None {\n\
        \x20   let p = Person {name: \"Ada\", surname: \"L\"}\n\
        \x20   if p.surname is Str surname {\n\
        \x20       let a = \"${p.name} ${surname}\"\n\
        \x20   }\n\
        \x20   let b = \"${p.surname ?: unreachable!()}\"\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [fn-param-default] A defaulted parameter may be omitted — positionally
// (trailing) or by naming a later one — with no error.
#[test]
fn omitting_a_defaulted_parameter_is_accepted() {
    let errors = check_errors(
        "fn greet(name: Str, greeting: Str = \"Hi\", punct: Str = \"!\") -> Str {\n\
        \x20   return \"${greeting}, ${name}${punct}\"\n\
         }\n\
         fn f() -> None {\n\
        \x20   let a = greet(\"Ada\")\n\
        \x20   let b = greet(\"Ada\", \"Hey\")\n\
        \x20   let c = greet(\"Ada\", punct = \"?\")\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [fn-param-default] A default may read an earlier Copy-scalar parameter.
#[test]
fn a_default_may_reference_an_earlier_scalar_parameter() {
    let errors = check_errors(
        "fn span(lo: Int, hi: Int = lo + 10) -> Int {\n\
        \x20   return hi - lo\n\
         }\n\
         fn f() -> None {\n\
        \x20   let a = span(5)\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [fn-param-default] A default MAY read a non-scalar earlier parameter
// (a `List`, a struct, anything with identity): the IR builder binds the
// parameter once (`Lower::args_for`/`block_expr`) and reads it from both
// its own argument slot and the default, so there is nothing to forbid
// based on the parameter's type.
#[test]
fn a_default_may_reference_a_non_scalar_earlier_parameter() {
    let errors = check_errors(
        "fn f(xs: List<Int>, y: Int = size(xs)) -> Int {\n\
        \x20   return y\n\
         }\n\
         fn g() -> None {\n\
        \x20   let a = f(List(1, 2, 3))\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [fn-param-default] A default cannot read a *later* parameter: only the
// earlier ones are in scope, matching how a struct field default cannot
// read a sibling field.
#[test]
fn a_default_cannot_reference_a_later_parameter() {
    let errors = check_errors(
        "fn f(a: Int = b, b: Int) -> Int {\n\
        \x20   return a + b\n\
         }\n",
    );
    assert!(
        errors.iter().any(|e| e.contains("`b`") && (e.contains("not in scope") || e.contains("is in scope"))),
        "errors: {errors:?}"
    );
}

// [fn-param-default] A default is checked with no effects available, so an
// effectful default is refused rather than silently colouring every caller
// that omits the argument.
#[test]
fn an_effectful_default_is_refused() {
    let errors = check_errors(
        "fn noisy() [Console] -> Int {\n\
        \x20   println(\"hi\")\n\
        \x20   return 1\n\
         }\n\
         fn f(x: Int = noisy()) [Console] -> Int {\n\
        \x20   return x\n\
         }\n",
    );
    assert!(!errors.is_empty(), "expected an error for an effectful default");
}

// [fn-param-default] A named argument may only target a parameter that has
// a default; naming an ordinary required parameter is refused.
#[test]
fn naming_a_parameter_with_no_default_is_refused() {
    let errors = check_errors(
        "fn add(a: Int, b: Int) -> Int {\n\
        \x20   return a + b\n\
         }\n\
         fn f() -> None {\n\
        \x20   let c = add(a = 1, b = 2)\n\
         }\n",
    );
    assert!(
        errors.iter().filter(|e| e.contains("has no default") && e.contains("only a defaulted parameter may be named")).count() >= 2,
        "errors: {errors:?}"
    );
}

// [fn-param-default] A variadic or an implicit parameter cannot carry a
// default — each already fills itself its own way. Parser-level, so parsed
// directly rather than through `check_errors` (which treats a parse error
// as a test failure).
#[test]
fn variadic_and_implicit_parameters_refuse_a_default() {
    let (_module, diags) = salvo_syntax::parse_module(
        "fn f(...xs: Int[] = 5) -> Int {\n\
        \x20   return 0\n\
         }\n",
    );
    let errors: Vec<String> = diags.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect();
    assert!(
        errors.iter().any(|e| e.contains("variadic parameter cannot have a default")),
        "errors: {errors:?}"
    );
    let (_module, diags) = salvo_syntax::parse_module(
        "fn f(?cmp: (Int, Int) -> Int = 5) -> Int {\n\
        \x20   return 0\n\
         }\n",
    );
    let errors: Vec<String> = diags.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect();
    assert!(
        errors.iter().any(|e| e.contains("implicit parameter cannot have a default")),
        "errors: {errors:?}"
    );
}

/// The handler every [fn-param-default] handler-constructor test builds:
/// `lo` required, `hi` defaulted from it, `step` defaulted.
const SPAN_HANDLER: &str = "effect Spanned {\n\
    \x20   fn width() -> Int\n\
     }\n\
     handler Span(lo: Int, hi: Int = lo + 10, step: Int = 1) of Spanned {\n\
    \x20   fn width() -> Int {\n\
    \x20       return (hi - lo) * step\n\
    \x20   }\n\
     }\n";

// [fn-param-default] A handler's constructor parameters take defaults with
// the fn rules: a trailing run omitted positionally, a defaulted parameter
// anywhere filled by name — in a `use`, a `with` item and a `spawn` alike —
// and a default that reads an earlier constructor parameter.
#[test]
fn a_handler_constructor_default_may_be_omitted_or_named() {
    let errors = check_errors(&format!(
        "{SPAN_HANDLER}\
         effect Twice {{\n\
        \x20   fn twice() -> Int\n\
         }}\n\
         handler Doubling [Spanned] of Twice {{\n\
        \x20   fn twice() -> Int {{\n\
        \x20       return width() * 2\n\
        \x20   }}\n\
         }}\n\
         actor effect Desk {{\n\
        \x20   send fn ask(out: Reply<Int>) => !out\n\
         }}\n\
         handler Desking(room: Int = 4, capacity: Int = 2) of Desk {{\n\
        \x20   mailbox {{ capacity: capacity }}\n\
        \x20   send fn ask(out: Reply<Int>) {{\n\
        \x20       send(out, room)\n\
        \x20   }}\n\
         }}\n\
         fn f() [use, spawn] -> None {{\n\
        \x20   use Span(1)\n\
        \x20   use Span(1, 5)\n\
        \x20   use Span(1, step = 2)\n\
        \x20   use Doubling with Span(2, step = 3)\n\
        \x20   let a = spawn Desking() on pool(1)\n\
        \x20   let b = spawn Desking(room = 9) on pool(1)\n\
         }}\n"
    ));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [fn-param-default] Naming a constructor parameter that has no default is
// refused with the fn message; a name matching no parameter is refused too
// (it used to be dropped without a word).
#[test]
fn naming_a_handler_parameter_with_no_default_is_refused() {
    let errors = check_errors(&format!(
        "{SPAN_HANDLER}\
         fn f() [use] -> None {{\n\
        \x20   use Span(lo = 1)\n\
        \x20   use Span(1, size = 2)\n\
         }}\n"
    ));
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`lo` has no default") && e.contains("only a defaulted parameter may be named")),
        "errors: {errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("`Span` has no implicit parameter named `size`")),
        "errors: {errors:?}"
    );
}

// [fn-param-default] Omitting a constructor parameter that has no default
// is still refused, naming it; so is giving one both ways, or too many.
#[test]
fn omitting_a_required_handler_parameter_is_refused() {
    let errors = check_errors(&format!(
        "{SPAN_HANDLER}\
         fn f() [use] -> None {{\n\
        \x20   use Span()\n\
        \x20   use Span(step = 2)\n\
        \x20   use Span(1, 2, hi = 3)\n\
        \x20   use Span(1, 2, 3, 4)\n\
         }}\n"
    ));
    assert_eq!(
        errors.iter().filter(|e| e.contains("handler `Span` is constructed without `lo`, which has no default")).count(),
        2,
        "errors: {errors:?}"
    );
    assert!(errors.iter().any(|e| e.contains("`hi` is already given positionally")), "errors: {errors:?}");
    assert!(
        errors.iter().any(|e| e.contains("handler `Span` takes at most 3 constructor argument(s), found 4")),
        "errors: {errors:?}"
    );
}

// [fn-param-default] A constructor parameter's default sees only the
// earlier parameters, as a fn's does.
#[test]
fn a_handler_default_cannot_reference_a_later_parameter() {
    let errors = check_errors(
        "effect Tell {\n\
        \x20   fn tell() -> Int\n\
         }\n\
         handler Teller(a: Int = b, b: Int) of Tell {\n\
        \x20   fn tell() -> Int {\n\
        \x20       return a + b\n\
        \x20   }\n\
         }\n",
    );
    assert!(errors.iter().any(|e| e.contains("named `b` is in scope")), "errors: {errors:?}");
}

// [fn-param-default] A constructor parameter's default is checked with no
// effects available: an effectful one is refused at the declaration.
#[test]
fn an_effectful_handler_default_is_refused() {
    let errors = check_errors(
        "fn noisy() [Console] -> Int {\n\
        \x20   println(\"hi\")\n\
        \x20   return 1\n\
         }\n\
         effect Tell {\n\
        \x20   fn tell() -> Int\n\
         }\n\
         handler Teller(n: Int = noisy()) [Console] of Tell {\n\
        \x20   fn tell() -> Int {\n\
        \x20       return n\n\
        \x20   }\n\
         }\n",
    );
    assert!(
        errors.iter().any(|e| e.contains("no handler for effect `Console`") && e.contains("`noisy`")),
        "errors: {errors:?}"
    );
}
