//! Overload resolution [fn-overload-scope] [fn-overload-rank]
//! [fn-overload-ambiguous] [fn-overload-at] [fn-rename] [fn-overload-return]
//! [fn-value-select] is test data, `tests/check/overloads/` (TESTS.md T1).
//! What is left here is a case whose diagnostics over the real std are not
//! yet the ones it should be: `list_size@core.list(xs)` on a renamed
//! overload reports the "`@` not needed" warning beside the error that says
//! to drop the `@` (ROADMAP §0ar). It stays as it was, over its prelude,
//! until that is fixed.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};
use salvo_syntax::diag::Severity;

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as *std* files (only std may write `intrinsic`), one per `std.core`
/// module so the tests can name them with `@`.
const STD_LIST: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type Char\nexport intrinsic type Any\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn of_list<T>(...elems: T[]) [] -> Mut List<T>\nexport intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);
const STD_STRING: &str = "export intrinsic fn size(str: Str) [] -> Int => str\n";

/// Checks a program of user files (`main.sv` first) against the std prelude.
fn checked_files(files: &[(&str, &str)]) -> salvo_core::Checked {
    let mut sources = SourceSet::default();
    for (name, src) in [
        ("std/core/list.sv", STD_LIST),
        ("std/core/string.sv", STD_STRING),
    ] {
        let rel = name;
        sources.add(
            name,
            SourceSet::classify(Path::new(rel)).unwrap(),
            src.to_string(),
            true,
        );
    }
    for (name, src) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            src.to_string(),
            false,
        );
    }
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    // `check_program` folds the resolution errors (duplicate fns among them)
    // into its own.
    check_program(&program, &resolution, &symbols)
}

fn checked(src: &str) -> salvo_core::Checked {
    checked_files(&[("main.sv", src)])
}

fn messages(src: &str) -> Vec<String> {
    checked(src).errors.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn probe(body: &str) -> String {
    format!("fn probe() -> None {{\n{body}\n}}\n")
}

/// A renamed name already means one declaration, so `@module` on it is an
/// error rather than a redundancy.
#[test]
fn a_renamed_name_takes_no_module_selector() {
    let src = format!(
        "{}{}",
        "rename fn list_size = size<T>(list: List<T>)\n",
        probe("    let xs = of_list(1, 2)\n    let n = list_size@core.list(xs)")
    );
    assert!(
        messages(&src)
            .iter()
            .any(|m| m.contains("is a renamed overload")),
        "{:?}",
        messages(&src)
    );
}
