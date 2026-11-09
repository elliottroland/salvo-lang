//! [ir-dump] The IR of every example's user module, as a golden snapshot:
//! the checker's own regression test, independent of either backend.

use std::path::Path;

use salvo_core::Program;

fn example_program(dir: &Path) -> Program {
    // [lsp-warm-std] Over the process's warm std cache (TESTS.md T3).
    salvo_testkit::front::expect_clean(salvo_testkit::front::project("kt", dir))
}

#[test]
fn example_ir_is_stable() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        // Examples with dependencies need the CLI's loader; the CLI's IR dump covers them.
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

/// [dump-rows] The checker's facts about one example's bodies — every
/// expression's type, the declaration each call resolved to, coercions and
/// moves — as `salvo dump --phase type-check --rows` prints them.
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

/// [ir-linear-check] D4b: the IR dump dumps even a program the IR linear check
/// rejects, with each leak inlined as `!!! module:line:col: …` and the IR
/// still produced (exit 0). The dump is a debugging view; `salvo build` is what
/// fails. Here a dropped `Checked` arm (noticeboard #2) is a genuine leak, so
/// the dump must contain both the fn's IR and the inline marker.
#[test]
fn dump_inlines_a_linear_leak_and_still_produces_the_ir() {
    let src = "\
export linear struct Problem { code: Int }\n\
export fn close(p: Problem) -> None => !p {}\n\
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

/// [dump-type-check] One example's typed dump (`salvo dump --phase
/// type-check`), as a golden snapshot: every choice the checker made
/// written out, and what Salvo cannot spell marked.
#[test]
fn example_type_check_dump_is_stable() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/iteration");
    let program = example_program(&dir);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    let text: String = (0..program.files.len())
        .filter(|f| !program.files[*f].is_std)
        .map(|f| format!("// {}\n{}", program.files[f].name, salvo_core::round_trip::dump_file(&program, &symbols, &checked, f, false)))
        .collect();
    insta::assert_snapshot!("type_check_iteration", text);
}

/// [dump-type-check] [round-trip] What each kind of choice reads as in the
/// typed dump: a callee with its selector and inferred type arguments, an
/// implicit by name, an operator as its call (a comparison as `cmp(…) < 0`,
/// `!=` as `!eq(…)`, grouping parentheses kept), an interpolated part as its
/// `to_str`, an unannotated binding's and a lambda's types, a generic
/// struct literal's type arguments; and, marked, a coercion, a narrowing,
/// the calls a `for` names and, with `--expr-types`, every type.
#[test]
fn the_type_check_dump_spells_each_kind_of_choice() {
    let main = "struct Box<T> {\n    v: T\n}\n\n\
                fn least<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Bool {\n    return cmp(a, b) < 0\n}\n\n\
                fn f(x: Int?, n: Long) -> Int {\n    let b = Box { v: 1 }\n    let k: (Int) -> Int = (a) -> { return a * 2 }\n    let s = (2 + 3) * b.v\n    if x is Int {\n        return k(x)\n    }\n    for y in range(0, 2) {\n        let z = \"${y + 1}\"\n    }\n    let w = n + 1\n    return s\n}\n\n\
                fn g() -> Bool {\n    return least(1, 2)\n}\n";
    let program = salvo_testkit::front::clean_program("kt", &[("main.sv", main)]);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    let errors: Vec<String> = checked.errors.iter().filter(|d| d.is_error()).map(|d| d.render(&program.files)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let file = program.files.iter().position(|f| f.name == "main.sv").unwrap();
    let text = salvo_core::round_trip::dump_file(&program, &symbols, &checked, file, false);
    for want in [
        "let b: Box@main<Int> = Box<Int> { v: 1 }",
        "return cmp@std.core.basic(cmp(a, b), 0) < 0",
        "= (a: Int) -> Int { return times@std.core.basic(a, 2) }",
        "let s: Int = times@std.core.basic((plus@std.core.basic(2, 3)), b.v)",
        "return k(x#{narrowed from Int?})",
        "for y in range@std.core.range(0, 2) #{next@std.core.range}",
        "let z: Str = \"${to_str@std.core.basic(plus@std.core.basic(y, 1))}\"",
        "let w: Long = plus@std.core.basic(n, 1#{as Long})",
        "return least@main<Int>(1, 2, cmp = cmp@std.core.basic)",
    ] {
        assert!(text.contains(want), "missing `{want}` in:\n{text}");
    }
    let typed = salvo_core::round_trip::dump_file(&program, &symbols, &checked, file, true);
    assert!(typed.contains("return s#{: Int}"), "{typed}");
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

/// A program with a comptime template stamped by a `by` (at a struct with a
/// tuple field, so the stamp makes a comptime call `[to_json(t.[field])]`,
/// which stamps a hidden fn), for the dumps of what the expansion produced.
const STAMPED: &str = "struct Person {
    name: Str
    pair: (Str, Int)

    fn to_json(t: Person) -> Str by to_json
}

comptime fn to_json<T is Struct>(t: T) -> Str {
    let json = Mut Str(\"{\")
    [for field in T.fields] {
        json.append@string(\"\\\"${field.name}\\\": \")
        [when field.type.kind] {
            is Struct { json.append@string([to_json(t.[field])]) }
            is Union { json.append@string(to_json@[site](t.[field])) }
            is Tuple { json.append@string([to_json(t.[field])]) }
            is FnType { refuse!(\"no functions\") }
            is Opaque { json.append@string(to_json@[site](t.[field])) }
        }
        [if !field.last] {
            json.append@string(\", \")
        }
    }
    json.append@string(\"}\")
    return json
}

comptime fn to_json<T is Tuple>(t: T) -> Str {
    let json = Mut Str(\"[\")
    [for index in T.indices] {
        [when index.type.kind] {
            is Struct { json.append@string([to_json(t.[index])]) }
            is Union { json.append@string(to_json@[site](t.[index])) }
            is Tuple { json.append@string([to_json(t.[index])]) }
            is FnType { refuse!(\"no functions\") }
            is Opaque { json.append@string(to_json@[site](t.[index])) }
        }
        [if !index.last] {
            json.append@string(\", \")
        }
    }
    json.append@string(\"]\")
    return json
}

fn to_json(str: Str) -> Str {
    return \"\\\"${str}\\\"\"
}

fn to_json(num: Int) -> Str {
    return num.to_str()
}

fn main() [use] {
    use StdConsole()
    println(to_json(Person { name: \"Roland\", pair: (\"age\", 36) }))
}
";

/// [dump-resolve] [ast-print] The resolve dump prints the expanded tree: a
/// stamped fn's body, with what each name in it resolved to (the stamp's
/// `to_json@[site]` as the site's `to_json@main`), and the hidden fn a
/// comptime call stamped, each under a comment saying what produced it; the
/// templates are gone.
#[test]
fn the_resolve_dump_shows_what_the_expansion_produced() {
    let program = salvo_testkit::front::clean_program("kt", &[("main.sv", STAMPED)]);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let file = program.files.iter().position(|f| f.name == "main.sv").unwrap();
    let text = salvo_core::names::dump::dump_file(&program, &resolution, &symbols, file, false);
    for want in [
        "    // stamped from comptime fn `to_json` (main.sv:8) at `Person`, for the `by` at main.sv:5\n    fn to_json(t: Person@main) -> Str {",
        "json#{7:13}.append@string(to_json@main(t#{6:16}.name))",
        "json#{7:13}.append@string(__salvo_comptime_main_to_json_0@main(t#{6:16}.pair))",
        "// hidden fn for a comptime call `[to_json(…)]` at `(Str, Int)`, made stamping the `by` at main.sv:5: stamped from comptime fn `to_json` (main.sv:27)\nfn __salvo_comptime_main_to_json_0(t: (Str, Int)) -> Str {",
        "json#{33:9}.append@string(to_json@main(t#{32:36}.1))",
    ] {
        assert!(text.contains(want), "missing `{want}` in:\n{text}");
    }
    assert!(!text.contains("comptime fn to_json<"), "the templates are gone: {text}");
}
