//! [ir-dump] The IR of every example's user module, as a golden snapshot:
//! the checker's own regression test, independent of either backend.

use std::path::Path;

use salvo_core::Program;

fn example_program(dir: &Path) -> Program {
    // [std-cache] Over the process's std front (TESTS.md T3).
    salvo_testkit::front::expect_clean(salvo_testkit::front::project("kt", dir))
}

#[test]
fn example_ir_is_stable() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        // Examples with dependencies need the CLI's loader; `salvo ir` covers them.
        .filter(|p| p.join("salvo.toml").exists() && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]"))
        .collect();
    dirs.sort();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let program = example_program(&dir);
        let (text, errors) = salvo_testkit::front::dump_ir("kt", &program, false, None).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(errors.is_empty(), "{name}: {errors:?}");
        insta::assert_snapshot!(format!("ir_{name}"), text);
    }
}

/// [cli-facts] The checker's facts about one example's bodies — every
/// expression's type, the declaration each call resolved to, coercions and
/// moves — as the pre-IR dump (`salvo facts`, PIPELINE.md D1) prints them.
#[test]
fn example_facts_are_stable() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/iteration");
    let program = example_program(&dir);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    assert!(!checked.errors.iter().any(|d| d.is_error()));
    let text = salvo_core::facts::dump(&program, &symbols, &checked, &|f| !program.files[f].is_std);
    assert!(text.contains("-> std.core.list::size"), "{text}");
    insta::assert_snapshot!("facts_iteration", text);
}

/// [ir-linear-check] D4b: `salvo ir` dumps even a program the IR linear check
/// rejects, with each leak inlined as `!!! module:line:col: …` and the IR
/// still produced (exit 0). The dump is a debugging view; `salvo build` is what
/// fails. Here a dropped `Checked` arm (noticeboard #2) is a genuine leak, so
/// the dump must contain both the fn's IR and the inline marker.
#[test]
fn dump_inlines_a_linear_leak_and_still_produces_the_ir() {
    let src = "\
export linear struct Problem { code: Int }\n\
export fn close(p: Problem) -> None => !p { discard(p) }\n\
export fn attempt() -> Ok Int | Err Problem { return Err Problem { code: 1 } }\n\
export fn drops_it() -> Int { return attempt() ?: return 0 }\n";
    let program = salvo_testkit::front::clean_program("kt", &[("main.sv", src)]);
    // Dumps rather than erroring (the leak is an IR-reliant check, not a type
    // error): returns `Ok` with the IR text.
    let (text, _errors) = salvo_testkit::front::dump_ir("kt", &program, false, None).expect("dump should succeed despite the leak");
    assert!(text.contains("fn drops_it@main"), "the IR body is dumped:\n{text}");
    assert!(text.contains("!!! main:"), "the leak is inlined with a location:\n{text}");
    assert!(text.contains("still owns a linear value"), "the leak message is inlined:\n{text}");
    // The arm is named by its type, not a bare index.
    assert!(text.contains("the `Problem` arm"), "the leaked arm is named:\n{text}");
    assert!(!text.contains("(arm 1)"), "the leak should not show a bare arm index:\n{text}");
    // The note lands on the line that produced the leak (the bind of the
    // picked value), not in a block after the fn.
    let on_line = text
        .lines()
        .any(|l| l.contains("let __pick") && l.contains("!!! main:"));
    assert!(on_line, "the leak note is on the line that produced it:\n{text}");
}

/// [dump-resolve] One example's resolve dump (`salvo dump --phase resolve`),
/// as a golden snapshot: every name with what resolution bound it to.
#[test]
fn example_resolve_dump_is_stable() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/iteration");
    let program = example_program(&dir);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let text: String = (0..program.files.len())
        .filter(|f| !program.files[*f].is_std)
        .map(|f| format!("// {}\n{}", program.files[f].name, salvo_core::names::dump::dump_file(&program, &resolution, &symbols, f, false)))
        .collect();
    insta::assert_snapshot!("resolve_iteration", text);
}

/// [dump-resolve] [dump-spelling] What each kind of name reads as in the
/// resolve dump: another module's declaration with its module (an import
/// alias as the declaration's own name), a callee by its selector, a name of
/// several modules as the set, a variable by where its binding is written
/// (an implicit group's member at the spread), std's own names bare, and a
/// name nothing answers as `#{???}`.
#[test]
fn the_resolve_dump_spells_each_kind_of_name() {
    let shapes = "export struct Point {\n    x: Int\n}\n\nexport fn show(p: Point) -> Str {\n    return \"${p.x}\"\n}\n\nexport fn describe(n: Int) -> Str {\n    return \"${n}\"\n}\n";
    let main = "import shapes.Point as P\nimport shapes.show\nimport shapes.describe\n\n\
                fn describe(s: Str) -> Str => !s {\n    return s\n}\n\n\
                fn least<T>(a: T, b: T, ?Ordered<T>) -> Bool {\n    return cmp(a, b) < 0\n}\n\n\
                fn main() [use] {\n    use StdConsole()\n    let p: P = P { x: 1 }\n    println(show(p))\n    println(describe(1))\n    println(describe@main(\"a\"))\n    let n = size(List(1, 2))\n    println(\"${n}\")\n}\n";
    let program = salvo_testkit::front::clean_program("kt", &[("shapes.sv", shapes), ("main.sv", main)]);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let file = program.files.iter().position(|f| f.name == "main.sv").unwrap();
    let text = salvo_core::names::dump::dump_file(&program, &resolution, &symbols, file, false);
    for want in [
        "let p: Point@shapes = Point@shapes { x: 1 }",
        "println(show@shapes(p#{15:9}))",
        "println(describe#{@main, @shapes}(1))",
        "println(describe@main(\"a\"))",
        "return cmp#{9:26}(a#{9:13}, b#{9:19}) < 0",
        "let n = size(List(1, 2))",
        "println(\"${n#{19:9}}\")",
        "use StdConsole()",
    ] {
        assert!(text.contains(want), "missing `{want}` in:\n{text}");
    }
    // [qual-depend] A value block's place is a variable read and its
    // constant nothing; [effect-available] a member reached only through
    // the body's effect list (an alias of another module) is that effect's;
    // and inside a lambda, whose fn type says what it performs, a name
    // nothing written answers is the type phase's.
    let svc = "export effect Clock {\n    fn now() -> Int\n}\n\nexport effects Time = [Clock]\n";
    let main = "import svc.Time\n\n\
                fn f(xs: List<Int>, i: Idx(xs) Int) [Time] -> Bool {\n    let k = () -> later()\n    return i is InRange(0, 9) && now() > 0\n}\n";
    let program = salvo_testkit::front::program("kt", &[("svc.sv", svc), ("main.sv", main)]).program;
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let file = program.files.iter().position(|f| f.name == "main.sv").unwrap();
    let text = salvo_core::names::dump::dump_file(&program, &resolution, &symbols, file, false);
    for want in [
        "i: Idx(xs#{3:6}) Int",
        "let k = () -> later#{deferred}()",
        "return i#{3:21} is InRange(0, 9) && now@Clock() > 0",
    ] {
        assert!(text.contains(want), "missing `{want}` in:\n{text}");
    }
    // A name nothing answers, in a program the checker refuses.
    let broken = salvo_testkit::front::program("kt", &[("main.sv", "fn f() -> Int {\n    let q: Pont = 1\n    return missing(q)\n}\n")]).program;
    let symbols = salvo_core::Symbols::collect(&broken);
    let resolution = salvo_core::resolve(&broken);
    let file = broken.files.iter().position(|f| f.name == "main.sv").unwrap();
    let text = salvo_core::names::dump::dump_file(&broken, &resolution, &symbols, file, false);
    assert!(text.contains("let q: Pont#{???} = 1") && text.contains("return missing#{???}(q#{2:9})"), "{text}");
}
