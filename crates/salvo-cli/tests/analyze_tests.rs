//! Integration tests for `salvo analyze` [cli-analyze]: parse + resolve +
//! check without code generation, text/JSON diagnostics, exit codes.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Creates a fresh source directory under the target tmp dir.
fn src_dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("analyze_{test}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn salvo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_salvo"))
        .args(args)
        .output()
        .expect("failed to run salvo")
}

const CLEAN: &str = "fn main() [use] {\n    use StdOutConsole()\n    println(\"hello\")\n}\n";

#[test]
fn clean_program_exits_zero() {
    let dir = src_dir("clean");
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

#[test]
fn type_errors_render_with_location_and_exit_nonzero() {
    let dir = src_dir("type_error");
    fs::write(
        dir.join("bad.sv"),
        "fn broken() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("expected `Int`, found `Str` --> bad.sv:2:18"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error, 0 warnings"), "stderr: {stderr}");
}

#[test]
fn json_output_lists_structured_diagnostics() {
    let dir = src_dir("json");
    fs::write(
        dir.join("bad.sv"),
        "fn broken() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap(), "--format", "json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success());
    assert!(stdout.trim().starts_with('['), "stdout: {stdout}");
    assert!(stdout.trim().ends_with(']'), "stdout: {stdout}");
    assert!(stdout.contains("\"file\": \"bad.sv\""), "stdout: {stdout}");
    assert!(stdout.contains("\"line\": 2"), "stdout: {stdout}");
    assert!(stdout.contains("\"col\": 18"), "stdout: {stdout}");
    assert!(stdout.contains("\"severity\": \"error\""), "stdout: {stdout}");
    // Message quotes (`Int`) survive; embedded double quotes are escaped.
    assert!(
        stdout.contains("\"message\": \"expected `Int`, found `Str`\""),
        "stdout: {stdout}"
    );
}

#[test]
fn json_output_is_empty_array_for_clean_program() {
    let dir = src_dir("json_clean");
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap(), "--format", "json"]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[]");
}

// Parse errors are reported with location; exit is nonzero.
#[test]
fn parse_errors_are_reported() {
    let dir = src_dir("parse_error");
    fs::write(dir.join("bad.sv"), "fn broken( {\n").unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("bad.sv:1:"), "stderr: {stderr}");
}

// A parse error in one file must not suppress checker diagnostics in
// other files: the broken file reports only its parse error, the clean
// file still gets type-checked.
#[test]
fn parse_error_in_one_file_does_not_suppress_other_files() {
    let dir = src_dir("mixed_errors");
    fs::write(dir.join("broken.sv"), "fn broken( {\n").unwrap();
    fs::write(
        dir.join("typed.sv"),
        "fn wrong() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("broken.sv:1:"), "stderr: {stderr}");
    assert!(
        stderr.contains("expected `Int`, found `Str` --> typed.sv:2:18"),
        "stderr: {stderr}"
    );
}

// [mod-ignore] `.svignore` entries exclude files and directories from
// analysis.
#[test]
fn svignore_excludes_sources() {
    let dir = src_dir("svignore");
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    fs::create_dir_all(dir.join("scratch")).unwrap();
    fs::write(dir.join("scratch/broken.sv"), "fn broken( {\n").unwrap();

    // Without .svignore the broken scratch file fails the analysis...
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(!out.status.success());

    // ...with it, the analysis is clean.
    fs::write(dir.join(".svignore"), "scratch/\n").unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

// `--backend` opts that backend's define files into the analysis; without
// it they are skipped entirely (backend-neutral analysis).
#[test]
fn backend_flag_selects_define_files() {
    let dir = src_dir("defines");
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    // A define file with a parse error: only seen with --backend kotlin.
    fs::write(dir.join("main.kotlin.sv"), "define fn broken( {\n").unwrap();

    let neutral = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        neutral.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&neutral.stderr)
    );

    let kotlin = salvo(&[
        "analyze",
        "--src",
        dir.to_str().unwrap(),
        "--backend",
        "kotlin",
    ]);
    let stderr = String::from_utf8_lossy(&kotlin.stderr);
    assert!(!kotlin.status.success());
    assert!(stderr.contains("main.kotlin.sv:1:"), "stderr: {stderr}");
}

#[test]
fn unknown_backend_is_an_error() {
    let dir = src_dir("unknown_backend");
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    let out = salvo(&[
        "analyze",
        "--src",
        dir.to_str().unwrap(),
        "--backend",
        "cobol",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown backend `cobol`"));
}

// [diag-import-suggest] Unresolved names suggest imports from modules that
// declare them: `use DefaultRandom` without the import gets a rendered
// help line (std `random` module) and a JSON `imports` array.
#[test]
fn unresolved_names_suggest_imports() {
    let dir = src_dir("import_suggest");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] {\n    use DefaultRandom\n}\n",
    )
    .unwrap();

    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("unknown handler `DefaultRandom` in `use`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("help: add `import random.DefaultRandom`"),
        "stderr: {stderr}"
    );

    let out = salvo(&["analyze", "--src", dir.to_str().unwrap(), "--format", "json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"imports\": [\"random.DefaultRandom\"]"),
        "stdout: {stdout}"
    );
}

// [diag-import-suggest] Suggestions also cover user modules (an effect
// declared in a sibling module) and misspelled import paths.
#[test]
fn import_suggestions_cover_user_modules_and_bad_imports() {
    let dir = src_dir("import_suggest_user");
    fs::write(
        dir.join("audit.sv"),
        "effect Audit {\n    fn audit(message: Str)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.sv"),
        "import wrong.path.Audit\n\nfn log_it() [Audit] {\n    audit(\"x\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // The bad import suggests the right path...
    assert!(
        stderr.contains("unresolved import: no module matching `wrong.path`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("help: add `import audit.Audit`"),
        "stderr: {stderr}"
    );
    // ...and the effect list's unknown `Audit` gets the same suggestion.
    assert!(stderr.contains("unknown effect `Audit`"), "stderr: {stderr}");
}

// [fn-must-return] A fn with a non-`None` return type must return on all
// paths; `if/else` and `when` where every branch returns count, yield-based
// iterator fns are exempt.
#[test]
fn missing_return_is_an_error() {
    let dir = src_dir("missing_return");
    fs::write(
        dir.join("bad.sv"),
        "fn sign(x: Int) -> Int {\n    if x < 0 {\n        return -1\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("missing return: not all paths in `sign` return a value"),
        "stderr: {stderr}"
    );

    // All-paths-return via if/else, iterator fns, and None-returning fns
    // are fine.
    fs::write(
        dir.join("bad.sv"),
        "fn sign(x: Int) -> Int {\n    if x < 0 {\n        return -1\n    } else {\n        return 1\n    }\n}\n\
         fn nums() -> Iter<Int> {\n    yield 1\n    yield 2\n}\n\
         fn nothing(x: Int) {\n    let y = x\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [deduce-consume] A deduction list consumes unlisted identifier
// arguments uniformly across all types: their type narrows to `Nothing`
// and later uses are errors; reassignment revives them; listed (kept)
// parameters are unaffected. Consumption on an always-exiting branch
// does not leak past the branch.
#[test]
fn use_after_consume_is_an_error() {
    let dir = src_dir("consume");
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Int, b: Int) -> [] Int {\n    return a + b\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let a = 1\n    let b = 2\n    \
         let c = a.add(b)\n    println(\"${a}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`a` cannot be used here: it was consumed (moved) by an earlier call"
        ),
        "stderr: {stderr}"
    );

    // Reassignment revives the variable; kept parameters (`[a]`) are
    // never consumed; consumption inside an always-exiting `if` branch
    // never reaches the code after the `if`.
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Int, b: Int) -> [] Int {\n    return a + b\n}\n\n\
         fn double(a: Int) -> [a] Int {\n    return a + a\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let a = 1\n    let b = 2\n    \
         let c = a.add(b)\n    a = 5\n    println(\"${a}\")\n    \
         let d = double(a)\n    println(\"${a}\")\n    \
         let n = 0\n    let found = while n < 5 {\n        if n == 3 {\n            break add(n, 10)\n        }\n        n++\n        n\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Consumption in a *fall-through* branch is conservative: the value
    // is maybe-moved after the `if`, so using it is an error.
    fs::write(
        dir.join("main.sv"),
        "fn consume(text: Str) -> [] None {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let s = \"x\"\n    let flag = true\n    \
         if flag {\n        consume(s)\n    }\n    println(\"${s}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`s` cannot be used here"),
        "stderr: {stderr}"
    );
}

// [deduce-consume] *Inferred* deductions are enforced like declared ones
// (second checking round): a callee that returns its parameter moves it,
// so the caller's argument is consumed with no annotation in sight.
#[test]
fn inferred_moves_consume_arguments() {
    let dir = src_dir("consume_inferred");
    fs::write(
        dir.join("main.sv"),
        "fn give_back(list: List<Str>) -> List<Str> {\n    return list\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let strings = list(\"a\", \"b\")\n    give_back(strings)\n    \
         println(\"${strings.size()}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`strings` cannot be used here: it was consumed (moved) by an earlier call"
        ),
        "stderr: {stderr}"
    );

    // Rebinding through the return value keeps it usable.
    fs::write(
        dir.join("main.sv"),
        "fn give_back(list: List<Str>) -> List<Str> {\n    return list\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let strings = list(\"a\", \"b\")\n    strings = give_back(strings)\n    \
         println(\"${strings.size()}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [deduce-consume] A kept parameter sheds its removal set (declared −
// kept qualifiers) from the argument at the call site: after
// `remove_first` (`[list: Mut]` on a `NonEmpty Mut` param) the variable
// is no longer `NonEmpty`, so a second call fails overload resolution.
// The explicit-empty `[list:]` form strips all declared qualifiers.
#[test]
fn calls_remove_qualifiers_per_declared_deductions() {
    let dir = src_dir("dedu_quals");
    fs::write(
        dir.join("main.sv"),
        "qualifier NonEmpty<T> of List<T> {\n    fn qualifies(list: List<T>) -> Bool {\n        return list.size() > 0\n    }\n}\n\n\
         fn remove_first<T>(list: NonEmpty Mut List<T>) -> [list: Mut] T {\n    return list.get(0)!\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let strings = mutable_list(\"a\", \"b\")\n    \
         if strings is NonEmpty {\n        let s = remove_first(strings)\n        let t = remove_first(strings)\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `remove_first(Mut List<Str>)`"),
        "stderr: {stderr}"
    );

    // `[list:]` strips every declared qualifier: after `weaken` the value
    // no longer satisfies a `NonEmpty`-requiring overload.
    fs::write(
        dir.join("main.sv"),
        "qualifier NonEmpty<T> of List<T> {\n    fn qualifies(list: List<T>) -> Bool {\n        return list.size() > 0\n    }\n}\n\n\
         fn weaken<T>(list: NonEmpty List<T>) -> [list:] None {\n}\n\n\
         fn head<T>(list: NonEmpty List<T>) -> T {\n    return list.get(0)!\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let strings = list(\"a\", \"b\")\n    \
         if strings is NonEmpty {\n        weaken(strings)\n        let s = head(strings)\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `head(List<Str>)`"),
        "stderr: {stderr}"
    );
}

// [deduce-consume] Flow-analysis edge cases: consuming an `is`-narrowed
// variable inside its own narrowed branch survives the narrowing restore,
// and loop bodies are re-checked with their exit state so back-edge flows
// (use early, consume late) surface. Consume-then-revive per iteration
// stays clean.
#[test]
fn consumption_survives_narrowing_and_loop_back_edges() {
    let dir = src_dir("consume_flow");
    fs::write(
        dir.join("main.sv"),
        "qualifier NonEmpty<T> of List<T> {\n    fn qualifies(list: List<T>) -> Bool {\n        return list.size() > 0\n    }\n}\n\n\
         fn consume(strings: List<Str>) -> [] None {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let strings = list(\"a\", \"b\")\n    if strings is NonEmpty {\n        consume(strings)\n    }\n    println(\"${strings.size()}\")\n    \
         let s = list(\"x\")\n    let i = 0\n    while i < 3 {\n        println(\"${s.size()}\")\n        consume(s)\n        i++\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // The consumption in the `is`-narrowed branch reaches the code after
    // the `if`...
    assert!(
        stderr.contains("`strings` cannot be used here"),
        "stderr: {stderr}"
    );
    // ...and the loop's second pass flags the use-before-consume on the
    // back edge.
    assert!(stderr.contains("`s` cannot be used here"), "stderr: {stderr}");

    // Consume-then-revive inside the body is clean across iterations.
    fs::write(
        dir.join("main.sv"),
        "fn consume(strings: List<Str>) -> [] None {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let s = list(\"x\")\n    let i = 0\n    while i < 3 {\n        consume(s)\n        s = list(\"y\")\n        i++\n    }\n    println(\"${s.size()}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [deduce-consume] `when` branches merge like `if` branches: consumption
// of the *subject* inside an arm survives the arm's narrowing restore and
// reaches the code after the `when` (any-fall-through-path rule), while a
// consuming arm that always exits contributes nothing.
#[test]
fn when_branches_merge_consumption() {
    let dir = src_dir("consume_when");
    let prelude = "qualifier Ok<T> of T\nqualifier Err<T> of T\n\n\
         fn ok<T>(value: T) -> T as Ok {\n    return value\n}\n\n\
         fn err<T>(value: T) -> T as Err {\n    return value\n}\n\n\
         fn consume(v: Ok Str) -> [] None {\n}\n\n";

    // Consumed in one fall-through arm -> unusable after the `when`.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn main() [use] {{\n    use StdOutConsole\n    \
             let result: Ok Str | Err Str = ok(\"x\")\n    \
             when result {{\n        is Ok {{\n            consume(result)\n        }}\n        is Err {{\n        }}\n    }}\n    \
             println(\"${{result}}\")\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`result` cannot be used here"),
        "stderr: {stderr}"
    );

    // The consuming arm always exits -> usable after the `when`.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn main() [use] {{\n    use StdOutConsole\n    \
             let result: Ok Str | Err Str = ok(\"x\")\n    \
             when result {{\n        is Ok {{\n            consume(result)\n            return\n        }}\n        is Err {{\n        }}\n    }}\n    \
             println(\"${{result}}\")\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [deduce-consume] The `if`/`else` merge matrix: consumed on every
// fall-through path, consumed on the only fall-through path (other exits),
// and consumed on an always-exiting path only (clean after).
#[test]
fn if_branch_merge_matrix() {
    let dir = src_dir("consume_if_matrix");
    let prelude = "fn consume(strings: List<Str>) -> [] None {\n}\n\n";

    // Both branches consume -> consumed after.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn both(flag: Bool) {{\n    let a = list(\"a\")\n    \
             if flag {{\n        consume(a)\n    }} else {{\n        consume(a)\n    }}\n    \
             let n = a.size()\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("`a` cannot be used here"), "stderr: {stderr}");

    // One branch consumes, the other exits: the only fall-through path
    // consumed it -> consumed after.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn one_exits(flag: Bool) {{\n    let b = list(\"b\")\n    \
             if flag {{\n        consume(b)\n    }} else {{\n        return\n    }}\n    \
             let n = b.size()\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("`b` cannot be used here"), "stderr: {stderr}");

    // The consuming branch always exits -> clean after.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn consuming_exits(flag: Bool) {{\n    let c = list(\"c\")\n    \
             if flag {{\n        consume(c)\n        return\n    }}\n    \
             let n = c.size()\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [deduce-consume] Qualifier removal on only *some* fall-through paths is
// conservative: the join keeps only the qualifiers common to all paths,
// so an overload requiring the maybe-removed qualifier no longer resolves
// (nested inside the `is` branch that established it).
#[test]
fn partial_qualifier_removal_merges_conservatively() {
    let dir = src_dir("consume_partial_qual");
    fs::write(
        dir.join("main.sv"),
        "qualifier NonEmpty<T> of List<T> {\n    fn qualifies(list: List<T>) -> Bool {\n        return list.size() > 0\n    }\n}\n\n\
         fn remove_first<T>(list: NonEmpty Mut List<T>) -> [list: Mut] T {\n    return list.get(0)!\n}\n\n\
         fn partial(flag: Bool) {\n    let strings = mutable_list(\"a\", \"b\")\n    \
         if strings is NonEmpty {\n        if flag {\n            let x = remove_first(strings)\n        }\n        let y = remove_first(strings)\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `remove_first(Mut List<Str>)`"),
        "stderr: {stderr}"
    );
}

// [deduce-consume] `for` loops re-check their body with the exit state
// like `while` loops do: the back edge surfaces use-early/consume-late.
#[test]
fn for_loop_back_edge() {
    let dir = src_dir("consume_for");
    fs::write(
        dir.join("main.sv"),
        "fn consume(strings: List<Str>) -> [] None {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let s = list(\"x\")\n    \
         for i in list(1, 2, 3).iter() {\n        println(\"${s.size()}\")\n        consume(s)\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("`s` cannot be used here"), "stderr: {stderr}");
}

// [type-union] A union-arm value passes where the union is expected, in
// argument position too: `describe(ok("x"))` against
// `describe(v: Ok Str | Err Str)` resolves and wraps (regression: the
// qualifier-stripping unify arm used to precede the union arm, so a
// qualified argument could never match a union's qualified arm).
#[test]
fn union_arm_arguments_resolve_against_union_params() {
    let dir = src_dir("union_arm_arg");
    fs::write(
        dir.join("main.sv"),
        "qualifier Ok<T> of T\nqualifier Err<T> of T\n\n\
         fn ok<T>(value: T) -> T as Ok {\n    return value\n}\n\n\
         fn err<T>(value: T) -> T as Err {\n    return value\n}\n\n\
         fn describe(v: Ok Str | Err Str) -> Str {\n    when v {\n        is Ok {\n            return \"ok\"\n        }\n        is Err {\n            return \"err\"\n        }\n    }\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    println(describe(ok(\"x\")))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

// ===== Shared fate [fate-link] [fate-poison] [fate-derived-readonly] =====

// [fate-derived-readonly] A fate-linked (derived) variable is read-only:
// moving it (consuming call, `return`) or mutating it (`Mut` argument)
// errors at the site with `copy` as the remedy. Reads stay legal.
#[test]
fn fate_derived_variables_are_read_only() {
    let dir = src_dir("fate_readonly");
    fs::write(
        dir.join("main.sv"),
        "fn consume(v: List<Int>) -> [] None {\n}\n\n\
         fn move_derived() {\n    let xs = list(1, 2)\n    let ys = xs\n    consume(ys)\n}\n\n\
         fn mutate_derived() {\n    let xs = mutable_list(1, 2)\n    let ys = xs\n    add(ys, 3)\n}\n\n\
         fn return_derived(v: Str) -> [v] Str {\n    let w = v\n    return w\n}\n\n\
         fn read_derived(v: Str) -> [v] Int {\n    let w = v\n    return size(w)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("cannot move `ys`: it was bound from `xs` and shares its fate"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("cannot mutate `ys`: it was bound from `xs`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("cannot return `w`: it was bound from `v`"),
        "stderr: {stderr}"
    );
    // Exactly the three violations: reading a derived variable is fine.
    assert!(stderr.contains("3 errors"), "stderr: {stderr}");
    assert!(stderr.contains("use `copy`"), "stderr: {stderr}");
}

// [fate-poison] Mutating, moving, or reassigning a root poisons every
// variable derived from it: the later *use* errors, naming the link and
// the event; a poison never observed never fires.
#[test]
fn fate_root_events_poison_derived_variables() {
    let dir = src_dir("fate_poison");
    fs::write(
        dir.join("main.sv"),
        "fn consume(v: List<Int>) -> [] None {\n}\n\n\
         fn mutated() -> Int {\n    let xs = mutable_list(1)\n    let ys = xs\n    \
         add(xs, 2)\n    return size(ys)\n}\n\n\
         fn moved() -> Int {\n    let xs = list(1)\n    let ys = xs\n    \
         consume(xs)\n    return size(ys)\n}\n\n\
         fn reassigned() -> Int {\n    let xs = list(1)\n    let ys = xs\n    \
         xs = list(2, 3)\n    return size(ys)\n}\n\n\
         fn unused_poison_is_fine() {\n    let xs = mutable_list(1)\n    let ys = xs\n    \
         add(xs, 2)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`ys` cannot be used here: it was bound from `xs` and shares its fate, \
             and `xs` was mutated after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("`xs` was moved after the binding"), "stderr: {stderr}");
    assert!(
        stderr.contains("`xs` was reassigned after the binding"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("3 errors"), "stderr: {stderr}");
}

// [fate-link] Links flow through projections (transitively to the root),
// loop bindings, and `is` bindings; reassignment from a fresh value
// severs them (revival).
#[test]
fn fate_links_flow_through_projections_loops_and_bindings() {
    let dir = src_dir("fate_links");
    fs::write(
        dir.join("main.sv"),
        "struct Person {\n    name: Str\n}\n\n\
         fn longest_name(persons: Person[]) -> [persons] Str {\n    let longest = \"\"\n    \
         for person in persons {\n        if size(longest) < size(person.name) {\n            \
         longest = person.name\n        }\n    }\n    return longest\n}\n\n\
         fn is_binding(v: Str | Int) -> [v] Str {\n    if v is Str s {\n        return s\n    }\n    \
         return \"other\"\n}\n\n\
         fn revived(p: Person) -> [p] Str {\n    let n = p.name\n    n = \"fresh\"\n    return n\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // The loop assignment links `longest` through `person` to `persons`.
    assert!(
        stderr.contains("cannot return `longest`: it was bound from `person`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("cannot return `s`: it was bound from `v`"), "stderr: {stderr}");
    // `revived` is clean: reassignment severed the link.
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
}

// [copy-fn] `copy` keeps its argument (with all qualifiers) and returns
// an independent value: every fate error above has a one-word remedy,
// and copies are unaffected by later root events.
#[test]
fn fate_copy_produces_independent_values() {
    let dir = src_dir("fate_copy");
    fs::write(
        dir.join("main.sv"),
        "struct Person {\n    name: Str\n}\n\n\
         fn longest_name(persons: Person[]) -> [persons] Str {\n    let longest = \"\"\n    \
         for person in persons {\n        if size(longest) < size(person.name) {\n            \
         longest = person.name\n        }\n    }\n    return copy(longest)\n}\n\n\
         fn independent() -> Int {\n    let xs = mutable_list(1)\n    let ys = copy(xs)\n    \
         add(xs, 2)\n    add(ys, 3)\n    return size(ys) + size(xs)\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         println(longest_name([Person {name: \"a\"}]))\n    println(\"${independent()}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

// [fate-link] Links union across branch merges: a variable linked on any
// fall-through path is linked after the join.
#[test]
fn fate_links_merge_across_branches() {
    let dir = src_dir("fate_branch");
    fs::write(
        dir.join("main.sv"),
        "fn pick(a: List<Int>, cond: Bool) -> [a] List<Int> {\n    \
         let out = list(0)\n    \
         if cond {\n        out = a\n    }\n    \
         return out\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("cannot return `out`: it was bound from `a`"),
        "stderr: {stderr}"
    );
}

// [struct-mut] Only `Mut`-qualified struct values may have fields
// assigned; the checker enforces it at the assignment site.
#[test]
fn struct_field_assignment_requires_mut() {
    let dir = src_dir("struct_mut");
    fs::write(
        dir.join("main.sv"),
        "struct Person with Mut {\n    name: Str\n}\n\n\
         fn bad() {\n    let p = Person {name: \"a\"}\n    p.name = \"b\"\n}\n\n\
         fn good() -> Str {\n    let p = Mut Person {name: \"a\"}\n    p.name = \"b\"\n    \
         return copy(p.name)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("cannot assign to field `name` of an immutable `Person` value"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}

// [fate-poison] Backend-parity fix: a non-identifier argument in a kept
// `Mut` position mutates through the projection, so it poisons variables
// derived from the provenance root (before this, `size(t)` compiled
// clean and printed 2 on Kotlin vs 1 on Rust). Mutating through a
// projection of a *derived* variable errors like any other mutation of
// it; `copy` at the binding is the remedy.
#[test]
fn fate_mut_projection_arguments_poison_derived() {
    let dir = src_dir("fate_mut_proj");
    fs::write(
        dir.join("main.sv"),
        "struct Holder {\n    tags: Mut List<Int>\n}\n\n\
         fn diverged() -> Int {\n    let h = Holder {tags: mutable_list(1)}\n    \
         let t = h.tags\n    add(h.tags, 2)\n    return size(t)\n}\n\n\
         fn mutate_through_derived(other: Holder) -> [other] None {\n    \
         let h = other\n    add(h.tags, 2)\n}\n\n\
         fn remedy() -> Int {\n    let h = Holder {tags: mutable_list(1)}\n    \
         let t = copy(h.tags)\n    add(h.tags, 2)\n    return size(t)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`t` cannot be used here: it was bound from `h` and shares its fate, \
             and `h` was mutated after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("cannot mutate `h`: it was bound from `other`"),
        "stderr: {stderr}"
    );
    // `remedy` is clean: exactly the two violations above.
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
}
