//! [default-fits] A `= default` must fit the slot it fills, at every default
//! site: a struct field, a parameter, a handler's state. Until 2026-10-20 the
//! struct-field and parameter sites checked the expression and discarded the
//! result, so `port: InRange(1, 10) Int = 8080` compiled with a false claim.
//! [default-predicate] What a default may lack: a predicate qualifier with
//! constant arguments, tested where the default is applied (the run test
//! `a_predicate_default_is_tested_where_it_is_applied` covers the trap).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file. `Ranged` is a predicate qualifier with constant slots (the
/// shape of std's `InRange`), `Big` one whose `qualifies` needs an implicit,
/// `Sorted` a constructive one.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str\n",
    "export intrinsic type List<T>\n",
    "export intrinsic fn List<T>(...elems: T[]) [] -> List<T>\n",
    "export qualifier Ranged(lo: Int, hi: Int) of Int {\n",
    "    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool { return true }\n",
    "}\n",
    "export qualifier Big of Int {\n",
    "    fn qualifies(n: Int, ?limit: (n: Int) -> Bool) -> Bool { return limit(n) }\n",
    "}\n",
    "export qualifier Sorted<T> of List<T>\n",
);

fn messages(src: &str) -> Vec<String> {
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
        assert!(parse_errors.is_empty(), "parse errors in {}: {parse_errors:?}", file.name);
        modules.push(ast);
    }
    let program = Program { files: sources.files, modules: Program::share(modules), companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    checked.errors.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
}

/// [default-fits] A default of the wrong type is an error at every site:
/// a struct field and a parameter (both unchecked before the fix) and a
/// handler's state (the one site that was already checked).
#[test]
fn a_default_of_the_wrong_type_is_refused_at_every_site() {
    for src in [
        "struct S { n: Int = \"x\" }\n",
        "fn f(n: Int = \"x\") [] -> Int => n { return n }\n",
        "effect E { fn get() -> Int }\nhandler H() of E {\n    state { n: Int = \"x\" }\n    fn get() -> Int { return n }\n}\n",
    ] {
        let errs = messages(src);
        assert!(
            errs.iter().any(|e| e.contains("this default does not fit its type: expected `Int`, found `Str`")),
            "`{src}`: {errs:?}"
        );
    }
}

/// [default-predicate] A predicate qualifier with constant arguments may be
/// missing from a default: it is tested where the default is applied. At a
/// field, a parameter and a handler's state alike.
#[test]
fn a_default_may_lack_a_constant_predicate_qualifier() {
    for src in [
        "struct S { n: Ranged(1, 10) Int = 8080 }\n",
        "fn f(n: Ranged(1, 10) Int = 8080) [] -> Int => n { return 0 }\n",
        "effect E { fn get() -> Int }\nhandler H() of E {\n    state { n: Ranged(1, 10) Int = 8080 }\n    fn get() -> Int { return 0 }\n}\n",
    ] {
        assert_eq!(messages(src), Vec::<String>::new(), "`{src}`");
    }
}

/// [default-predicate] What cannot be tested where a default is applied is
/// still refused, naming the qualifier: a constructive claim (nothing tests
/// it), and a predicate whose `qualifies` needs an implicit the site may not
/// have.
#[test]
fn a_default_cannot_lack_a_claim_it_cannot_test() {
    for (src, qual) in [
        ("struct S { xs: Sorted List<Int> = List(1, 2) }\n", "`Sorted`"),
        ("struct S { n: Big Int = 3 }\n", "`Big`"),
    ] {
        let errs = messages(src);
        assert!(
            errs.iter().any(|e| e.contains("this default does not fit its type")
                && e.contains("a default can lack only a predicate qualifier with constant arguments")
                && e.contains(qual)),
            "`{src}`: {errs:?}"
        );
    }
}
