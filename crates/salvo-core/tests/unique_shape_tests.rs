//! [unique-shape] A function of a unique shape on a type is declared in the
//! type's own file (user decision O1-D, 2026-10-27; OPERATOR_OVERLOAD.md
//! "D's details"): the class is `eq` (2 explicit parameters, `-> Bool`), `cmp`
//! (2, `-> Int`), `plus`, `minus`, `times`, `divide`, `pow` (2), `to_str` (1),
//! `iter` (1) and `next` (1, `-> Emitted T | Finished`); "on `T`" is the first
//! explicit parameter's base type, qualifiers stripped; a second one anywhere
//! else is an error at its declaration, used or not. These tests run the real
//! std, since the homes of the built-in and structural types are std's files,
//! and read only the resolution's diagnostics: the rule is a `resolve`-phase
//! fact.

use std::path::Path;

use salvo_core::source::ModulePath;
use salvo_core::{resolve, Program, SourceSet};

/// A user file: a plain one, or a `.test.sv` annex of the module it names.
enum File<'a> {
    Plain(&'a str, &'a str),
    Annex(&'a str, &'a str),
}

/// The `[unique-shape]` errors of std plus `files`, as `file:line: message`.
fn refusals(files: &[File]) -> Vec<String> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let mut sources = SourceSet::default();
    let io_errors = sources.add_dir(&std_dir, "rs", true, false);
    assert!(io_errors.is_empty(), "failed to read std: {io_errors:?}");
    for file in files {
        match file {
            File::Plain(name, src) => {
                sources.add(*name, SourceSet::classify(Path::new(name)).unwrap(), src.to_string(), false)
            }
            File::Annex(name, src) => {
                let module = name.trim_end_matches(".sv");
                sources.add_test(*name, ModulePath::parse(module), src.to_string())
            }
        }
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(ast);
    }
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
        .filter(|d| d.message.contains("[unique-shape]"))
        .map(|d| {
            let file = &program.files[d.file];
            let line = file.content[..(d.span.start as usize).min(file.content.len())].matches('\n').count() + 1;
            format!("{}:{line}: {}", file.name, d.message)
        })
        .collect()
}

fn plain(files: &[(&'static str, &'static str)]) -> Vec<String> {
    let files: Vec<File> = files.iter().map(|(n, s)| File::Plain(n, s)).collect();
    refusals(&files)
}

/// Every row of the class on a struct: its own file and its `next`'s pass
/// struct.
const PT: &str = "export struct Pt { x: Int }\n\nexport struct PtNext canbe Mut { at: Int }\n";

/// One fn of each shape on `Pt` (and `PtNext`), in declaration order.
const SHAPES_ON_PT: &str = "\
export fn eq(a: Pt, b: Pt) [] -> Bool => a, b { return true }
export fn cmp(a: Pt, b: Pt) [] -> Int => a, b { return 0 }
export fn plus(a: Pt, b: Pt) [] -> Pt => a, b { return Pt { x: 0 } }
export fn minus(a: Pt, b: Pt) [] -> Pt => a, b { return Pt { x: 0 } }
export fn times(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
export fn divide(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
export fn pow(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
export fn to_str(p: Pt) [] -> Str => p { return \"pt\" }
export fn iter(p: Pt) [] -> Mut PtNext => p { return Mut PtNext { at: 0 } }
export fn next(p: Mut PtNext) [] -> Emitted Int | Finished => p: Mut { return finished() }
";

#[test]
fn each_shape_is_refused_outside_its_types_file() {
    let other = format!("import pt\n\n{SHAPES_ON_PT}");
    let other: &'static str = Box::leak(other.into_boxed_str());
    let errs = plain(&[("pt.sv", PT), ("other.sv", other)]);
    let shapes = ["eq", "cmp", "plus", "minus", "times", "divide", "pow", "to_str", "iter", "next"];
    assert_eq!(errs.len(), shapes.len(), "{errs:#?}");
    for (i, (shape, err)) in shapes.iter().zip(&errs).enumerate() {
        let on = if *shape == "next" { "`PtNext`" } else { "`Pt`" };
        assert!(
            err.starts_with(&format!("other.sv:{}: ", i + 3))
                && err.contains(&format!("has the shape of `{shape}` on {on}"))
                && err.contains("`pt.sv` (module `pt`)")
                && err.contains("Move this one into that file, or give it another name and pass it explicitly"),
            "{shape}: {err}"
        );
    }
}

#[test]
fn each_shape_is_accepted_in_its_types_file() {
    let own = format!("{PT}\n{SHAPES_ON_PT}");
    let own: &'static str = Box::leak(own.into_boxed_str());
    let errs = plain(&[("pt.sv", own)]);
    assert!(errs.is_empty(), "{errs:#?}");
}

/// D-2: the first explicit parameter's *base* type, qualifiers stripped.
#[test]
fn a_qualified_first_parameter_is_on_its_base() {
    let errs = plain(&[(
        "main.sv",
        "fn eq<T>(a: NonEmpty List<T>, b: List<T>) [] -> Bool => a, b { return true }\n\
         fn next(s: Mut Str) [] -> Emitted Char | Finished => s: Mut { return finished() }\n",
    )]);
    assert_eq!(errs.len(), 2, "{errs:#?}");
    assert!(
        errs[0].starts_with("main.sv:1: ")
            && errs[0].contains("`eq(NonEmpty List<T>, List<T>)` has the shape of `eq` on `List`")
            && errs[0].contains("(module `std.core.list`)")
            // std's file is not the program's to edit: the remedy is the name.
            && errs[0].contains("Give this one another name and pass it explicitly")
            && !errs[0].contains("Move this one"),
        "{}",
        errs[0]
    );
    assert!(
        errs[1].contains("has the shape of `next` on `Str`") && errs[1].contains("(module `std.core.string`)"),
        "{}",
        errs[1]
    );
}

/// D-3: a name with another arity, or `eq`/`cmp`/`next` with another return,
/// is an ordinary fn; implicit parameters are not counted.
#[test]
fn a_wrong_arity_or_return_is_not_a_shape() {
    let errs = plain(&[(
        "main.sv",
        "struct Stepped canbe Mut { at: Int }\n\
         fn iter(list: List<Str>, step: Int) [] -> Mut Stepped => list, step { return Mut Stepped { at: step } }\n\
         fn eq(a: Int, b: Int) [] -> Int => a, b { return 0 }\n\
         fn cmp(a: Str, b: Str) [] -> Bool => a, b { return true }\n\
         fn next(n: Mut List<Int>) [] -> Int => n: Mut { return 0 }\n\
         fn to_str(a: Int, b: Int) [] -> Str => a, b { return \"\" }\n\
         fn plus(a: Int) [] -> Int => a { return a }\n\
         effect Counter {\n    fn next(step: Int) -> Int => step\n}\n\
         fn to_str(x: Long, ?f: (x: Long) -> Str) [] -> Str => x { return f(x) }\n",
    )]);
    // Only the last: one explicit parameter, the implicit not counted.
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert!(
        errs[0].starts_with("main.sv:11: ") && errs[0].contains("`to_str(Long)` has the shape of `to_str` on `Long`"),
        "{}",
        errs[0]
    );
}

/// D-4: a `fns` group's member signatures and comptime templates are exempt;
/// a stamp is a declaration like a written one, refused outside the type's
/// file. Effect members are covered; a handler's members implement theirs.
#[test]
fn groups_and_templates_are_exempt_and_stamps_and_effect_members_are_not() {
    let errs = plain(&[
        ("pt.sv", "export struct Pt { x: Int }\n\nexport effect Show {\n    fn to_str(p: Pt) -> Str => p\n}\n"),
        (
            "main.sv",
            "import pt\n\
             fns Shown<T> {\n    fn to_str(x: T) -> Str\n}\n\
             comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {\n    return true\n}\n\
             fn eq(a: Pt, b: Pt) [] -> Bool => a, b by auto\n\
             effect Fmt {\n    fn to_str(x: Int) -> Str => x\n}\n\
             handler Plain of Show {\n    fn to_str(p: Pt) -> Str { return \"pt\" }\n}\n",
        ),
    ]);
    assert_eq!(errs.len(), 2, "{errs:#?}");
    assert!(
        errs[0].starts_with("main.sv:8: ") && errs[0].contains("`eq(Pt, Pt)` has the shape of `eq` on `Pt`"),
        "{}",
        errs[0]
    );
    assert!(
        errs[1].starts_with("main.sv:10: ")
            && errs[1].contains("`to_str(Int)` has the shape of `to_str` on `Int`")
            && errs[1].contains("(module `std.core.basic`)"),
        "{}",
        errs[1]
    );
}

/// D-5: a bare type parameter has no home, so a shape on one is refused —
/// the catch-all `fn to_str<T>(x: T)`.
#[test]
fn a_bare_type_parameter_is_refused() {
    let errs = plain(&[(
        "main.sv",
        "fn to_str<T>(x: T) [] -> Str => x { return \"?\" }\n\
         fn eq<T>(a: T, b: T) [] -> Bool => a, b { return true }\n",
    )]);
    assert_eq!(errs.len(), 2, "{errs:#?}");
    for (err, shape) in errs.iter().zip(["to_str", "eq"]) {
        assert!(
            err.contains(&format!("has the shape of `{shape}` on the type parameter `T`, which has no declaring file:"))
                && err.contains("give this one another name and pass it explicitly"),
            "{err}"
        );
    }
}

/// D-5: the structural types' homes in std — tuples, fn types and anonymous
/// unions in `std.core.basic`, arrays in `std.core.array`, an `eff[E]` handle
/// where `Addr` is declared; a literal type is on its base.
#[test]
fn structural_types_have_a_std_home() {
    let errs = plain(&[(
        "main.sv",
        "fn eq<A, B>(a: (A, B), b: (A, B)) [] -> Bool => a, b { return true }\n\
         fn to_str(f: (x: Int) -> Int) [] -> Str => f { return \"fn\" }\n\
         fn to_str(x: Int | Str) [] -> Str => x { return \"u\" }\n\
         fn to_str(x: Bool?) [] -> Str => x { return \"o\" }\n\
         fn to_str(xs: Int[]) [] -> Str => xs { return \"a\" }\n\
         fn to_str(c: eff[Console]) [] -> Str => c { return \"h\" }\n\
         fn to_str(x: \"A\") [] -> Str => x { return \"A\" }\n",
    )]);
    let homes = [
        ("tuples", "std.core.basic"),
        ("fn types", "std.core.basic"),
        ("anonymous unions", "std.core.basic"),
        ("anonymous unions", "std.core.basic"),
        ("arrays (`T[]`)", "std.core.array"),
        ("`eff[…]` handles", "std.core.actor"),
        ("`Str`", "std.core.string"),
    ];
    assert_eq!(errs.len(), homes.len(), "{errs:#?}");
    for (err, (on, module)) in errs.iter().zip(homes) {
        assert!(err.contains(&format!(" on {on}, which")) && err.contains(&format!("(module `{module}`)")), "{on}: {err}");
    }
}

/// O4: a declared union is its own home; a non-union alias names the type it
/// expands to [type-alias], so its shapes live with that type.
#[test]
fn a_union_alias_is_a_home_and_another_alias_names_its_target() {
    let errs = plain(&[(
        "main.sv",
        "struct A {}\nstruct B {}\ntype Shape = A | B\ntype Grid = List<Int>\n\
         fn to_str(s: Shape) [] -> Str => s { return \"shape\" }\n\
         fn to_str(g: Grid) [] -> Str => g { return \"grid\" }\n",
    )]);
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert!(
        errs[0].starts_with("main.sv:6: ") && errs[0].contains("on `List`") && errs[0].contains("(module `std.core.list`)"),
        "{}",
        errs[0]
    );
}

/// D-1: a `.test.sv` annex is a different file — it may declare shapes only
/// for the types it declares itself.
#[test]
fn an_annex_declares_shapes_only_for_its_own_types() {
    let errs = refusals(&[
        File::Plain("pt.sv", "export struct Pt { x: Int }\n"),
        File::Annex(
            "pt.test.sv",
            "fn to_str(p: Pt) [] -> Str => p { return \"pt\" }\n\
             struct Probe { n: Int }\n\
             fn to_str(p: Probe) [] -> Str => p { return \"probe\" }\n",
        ),
    ]);
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert!(
        errs[0].starts_with("pt.test.sv:1: ") && errs[0].contains("on `Pt`") && errs[0].contains("`pt.sv` (module `pt`)"),
        "{}",
        errs[0]
    );
}

/// Several overloads of one shape in the type's own file stay legal:
/// uniqueness is across files, and inside the file the author controls the
/// set.
#[test]
fn several_overloads_in_the_types_own_file_are_allowed() {
    let errs = plain(&[(
        "boxes.sv",
        "export struct Box<T> { value: T }\n\
         export fn eq<T>(a: Box<T>, b: Box<T>, ?eq: (x: T, y: T) -> Bool) [] -> Bool => a, b { return eq(a.value, b.value) }\n\
         export fn eq(a: Box<Int>, b: Box<Int>) [] -> Bool => a, b { return a.value == b.value }\n\
         export fn to_str<T>(b: Box<T>, ?to_str: (x: T) -> Str) [] -> Str => b { return to_str(b.value) }\n\
         export fn to_str(b: Box<Str>) [] -> Str => b { return b.value }\n",
    )]);
    assert!(errs.is_empty(), "{errs:#?}");
}

/// std itself follows the rule: every shape it declares is in its type's
/// file (D-7's adoption, checked rather than assumed).
#[test]
fn std_declares_every_shape_in_its_types_file() {
    assert!(refusals(&[]).is_empty());
}
