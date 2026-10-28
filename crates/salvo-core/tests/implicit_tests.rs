//! Implicit parameters [implicit-param] [implicit-group]
//! [implicit-resolve] [implicit-forward] [implicit-override] and
//! [effect-handler-generics] are test data, `tests/check/implicits/`
//! (TESTS.md T1). What is left here is a case whose diagnostics over the
//! real std are not yet the ones it should be: `use Echo<Str>(1)` reports
//! the handler-argument mismatch beside the type-argument conflict, two
//! errors for one mistake (ROADMAP §0ar). It stays as it was, over its
//! prelude, until that is fixed.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// [op-unique-shape] `cmp` and `times` are operators on `Int`, so they are
/// declared in `Int`'s own file, here, and reach the tests' programs as any
/// std overload does.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export fn cmp(a: Int, b: Int) -> Int { return 0 }\n\
                           export fn times(a: Int, b: Int) -> Int { return 0 }\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
);

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
