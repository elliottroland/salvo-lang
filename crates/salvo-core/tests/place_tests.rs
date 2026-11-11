//! Place-based flow narrowing [flow-place] [flow-place-invalidate].
//!
//! `is` narrows *places* — a variable or a field chain out of one — not
//! just variables, so a checked field reads at its narrowed type. The
//! observable used here is `[interp-no-none]`: interpolating a
//! possibly-`None` value is an error, so an accepted interpolation of
//! `p.surname` is proof the field narrowed, and a rejected one is proof
//! the fact was invalidated.
//!
//! The rest of these cases are test data, `tests/check/places/` (TESTS.md
//! T1). This one stays, over its prelude, until a defect is fixed (ROADMAP
//! §0ar): assigning `None` to a narrowed `Str?` field also reports
//! `var-no-widen:assign-mismatch` ("cannot assign `None` to `Str`"), which
//! [var-no-widen] says it must not; the test below filters it out. The
//! last test is about how a join *spells* a claim, which an annotation does
//! not see (it judges a claim by its value).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `std.core.prelude`: `std.core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
/// [iter-for] [iter-for-native] What a `for` over an array calls, in the
/// arrays' std home [op-unique-shape]: `for x in xs` is `next(iter(xs))`, which
/// the backends walk natively when these are the calls.
const STD_ARRAY: &str = "export qualifier Emitted<T> for T\nexport struct Finished {}\n\
     export struct ArrayIter<T> canbe Mut {\n    at: Int\n}\n\
     export intrinsic fn iter<T>(array: T[]) [] -> Mut ArrayIter<T> => array\n\
     export intrinsic fn next<T>(p: Mut ArrayIter<T>) [] -> Emitted T | Finished => p: Mut\n";

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

/// Parses + resolves + checks one file (no std) and returns the checker's
/// and resolver's error messages.
/// [diag-id] The id of every error `src` reports.
fn errors(src: &str) -> Vec<&'static str> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/array.sv",
        SourceSet::classify(Path::new("std/core/array.sv")).unwrap(),
        STD_ARRAY.to_string(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
        interfaces: None,
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.id.unwrap_or("?"))
        .collect()
}

const PRELUDE: &str = r#"

struct Address canbe Mut {
    city: Str? = None
}

struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    nick: Str? = None,
    tags: Str?[],
    pair: (Str?, Str?),
    address: Mut Address
}

// Opaque observers. Deduction lists are *written*, which is what these
// tests turn on [decl-explicit]: the declared contract is authoritative, so
// the body only has to exist.
fn touch(p: Mut Person) [] -> None => p: Mut {
    p.name = p.name
}

fn read(p: Person) [] -> None => p {}

fn touch_address(a: Mut Address) [] -> None => a: Mut {
    a.city = a.city
}
"#;

fn check(body: &str) -> Vec<&'static str> {
    errors(&format!("{PRELUDE}\nfn probe(p: Mut Person) -> Str => p: Mut {{\n{body}\n}}\n"))
}

fn interp_errors(body: &str) -> Vec<&'static str> {
    check(body)
        .into_iter()
        .filter(|id| *id == "interp-no-none:optional-value")
        .collect()
}

/// [flow-place-invalidate] Assigning to the checked field drops the fact.
#[test]
fn assignment_to_the_place_invalidates() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        p.surname = None
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after assignment to fail: {errs:?}");
}

/// [qual-depend-field] [type-query] A join spells a claim as it names its
/// value **after** the branches: the `str` a branch destructured ended with
/// it, so the join is `Idx(a.str)`, whichever spelling each branch read, and
/// a later `str` (another cursor's) is not taken for it. What a `//~ type[T]`
/// annotation cannot see, since it is judged by the value a claim is about,
/// not by its spelling (`tests/check/flow/joins.sv` has the claims).
#[test]
fn a_join_spells_its_claims_where_it_is() {
    let src = "struct Cursor canbe Mut {\n    str: proj Str\n    pos: Idx(str) Int\n}\n\n\
               fn f(a: Cursor, b: Cursor, c: Bool) [] -> Int => a, b {\n    \
               let p = if c {\n        let { str, pos } = a\n        let _n = size(str)\n        pos\n    } else {\n        a.pos\n    }\n    \
               type?(p)\n    \
               let q = if c {\n        let { str, pos } = a\n        let _n = size(str)\n        pos\n    } else {\n        let { str, pos } = a\n        let _n = size(str)\n        pos\n    }\n    \
               let { str, pos } = b\n    \
               type?(q)\n    \
               let r = pos\n    \
               if c {\n        r = b.pos\n    }\n    \
               type?(r)\n    \
               return p + q + r + size(str)\n}\n\nfn main() {}\n";
    let out = salvo_testkit::front::check_main(src);
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    let shown: Vec<&str> = out.diagnostics().filter(|d| d.file == user).map(|d| d.message.as_str()).collect();
    assert_eq!(
        shown,
        [
            "type of `p`: Idx(a.str) Int",
            "type of `q`: Idx(a.str) Int",
            // `str` and `b.str` both name it here: the first path's spelling
            // is kept.
            "type of `r`: Idx(b.str) Int",
        ],
        "{shown:#?}"
    );
}
