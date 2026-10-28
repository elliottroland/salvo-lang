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
    assert!(text.contains("fn main::drops_it"), "the IR body is dumped:\n{text}");
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
