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
         fn move_derived() -> Int {\n    let xs = list(1, 2)\n    let ys = xs\n    consume(ys)\n    return size(xs)\n}\n\n\
         fn mutate_derived() -> Int {\n    let xs = mutable_list(1, 2)\n    let ys = xs\n    add(ys, 3)\n    return size(xs)\n}\n\n\
         fn return_derived(v: Str) -> [v] Str {\n    let w = v\n    return w\n}\n\n\
         fn read_derived(v: Str) -> [v] Int {\n    let w = v\n    return size(w)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [fate-move-mode] S2: moving or mutating a derived variable whose
    // roots are owned is legal — the binding takes ownership and the
    // *root* is consumed at the binding; its later use names the binding.
    assert_eq!(
        stderr
            .matches(
                "`xs` cannot be used here: `ys` was bound from it and later moves the value"
            )
            .count(),
        2,
        "stderr: {stderr}"
    );
    // A *written-kept* parameter root keeps the S1 error at the move
    // site [fate-derived-readonly]: you cannot move out of a borrow.
    assert!(
        stderr.contains("cannot return `w`: it was bound from `v`"),
        "stderr: {stderr}"
    );
    // Exactly the three violations: reading a derived variable is fine.
    assert!(stderr.contains("3 errors"), "stderr: {stderr}");
    assert!(stderr.contains("copy"), "stderr: {stderr}");
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

// [deduce-consume] L2: the remaining move events consume a bare root
// identifier exactly like a call-site move — storing it in a
// struct/array/tuple literal, spreading it (`...n`), `break n` (the code
// after the loop sees it moved, even when the `break` sits inside a
// branch), `yield n` (anew every iteration — the loop re-check surfaces
// the back edge), and passing it to a `use` handler constructor. The
// use-site diagnostic names the consuming event. Derived variables
// cannot be moved by any of these [fate-derived-readonly].
#[test]
fn l2_move_sites_consume_roots() {
    let dir = src_dir("l2_move_sites");
    fs::write(
        dir.join("main.sv"),
        "struct Box {\n    item: Str\n}\n\n\
         effect Greeter {\n    fn greet() -> Str\n}\n\n\
         handler FixedGreeter(text: Str) of Greeter {\n    \
         fn greet() -> Str {\n        return \"hi\"\n    }\n}\n\n\
         fn read(s: Str) -> [s] None {\n}\n\n\
         fn tuple_store() {\n    let s = \"x\"\n    let t = (s, 1)\n    read(s)\n}\n\n\
         fn array_store() {\n    let s = \"x\"\n    let a = [s]\n    read(s)\n}\n\n\
         fn struct_store() {\n    let s = \"x\"\n    let b = Box {item: s}\n    read(s)\n}\n\n\
         fn spread_store() {\n    let b = Box {item: \"x\"}\n    let c = Box {...b}\n    \
         read(b.item)\n}\n\n\
         fn break_in_branch() {\n    let s = \"x\"\n    let r = while true {\n        \
         if true {\n            break s\n        }\n    }\n    read(s)\n}\n\n\
         fn yield_in_loop() -> Iter<Str> {\n    let s = \"x\"\n    \
         for i in [1, 2] {\n        yield s\n    }\n}\n\n\
         fn use_ctor() [use] {\n    let s = \"hi\"\n    use FixedGreeter(s)\n    read(s)\n}\n\n\
         fn store_derived() {\n    let xs = list(1, 2)\n    let ys = xs\n    let t = (ys, 1)\n    \
         read_list(xs)\n}\n\n\
         fn read_list(v: List<Int>) -> [v] None {\n}\n\n\
         fn spread_derived() {\n    let b = Box {item: \"x\"}\n    let d = b\n    \
         let c = Box {...d}\n    read(b.item)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Literal stores (tuple, array, struct) each consume `s`.
    assert_eq!(
        stderr
            .matches("`s` cannot be used here: it was consumed (moved) by a literal store")
            .count(),
        3,
        "stderr: {stderr}"
    );
    // Spread consumes its base.
    assert!(
        stderr.contains("`b` cannot be used here: it was consumed (moved) by a `...` spread"),
        "stderr: {stderr}"
    );
    // `break s` inside an always-exiting branch still reaches the code
    // after the loop (the loop exit merges break-path states).
    assert!(
        stderr.contains("`s` cannot be used here: it was consumed (moved) by a `break`"),
        "stderr: {stderr}"
    );
    // `yield s` in a loop consumes anew every iteration: the back edge
    // errors at the yield itself on the re-check.
    assert!(
        stderr.contains("`s` cannot be used here: it was consumed (moved) by a `yield`"),
        "stderr: {stderr}"
    );
    // Handler-constructor arguments are stored in the handler.
    assert!(
        stderr.contains(
            "`s` cannot be used here: it was consumed (moved) by a `use` handler registration"
        ),
        "stderr: {stderr}"
    );
    // [fate-move-mode] S2: storing/spreading a derived variable of owned
    // roots is legal — the binding takes ownership, so the *root* is
    // consumed at the binding and its later use names the binding.
    assert!(
        stderr.contains(
            "`xs` cannot be used here: `ys` was bound from it and later moves the value"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(
            "`b` cannot be used here: `d` was bound from it and later moves the value"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("9 errors"), "stderr: {stderr}");
}

// [deduce-consume] L2 positive matrix: `copy` at each new move site keeps
// the source usable [copy-fn]; reassignment revives (also within a loop
// body ahead of the back edge); a `break value` consumes only its own
// operand; and a `return` inside a branch poisons derived variables only
// on the exiting path — the fall-through path never saw the move.
#[test]
fn l2_move_sites_remedies_stay_clean() {
    let dir = src_dir("l2_move_remedies");
    fs::write(
        dir.join("main.sv"),
        "struct Box {\n    item: Str\n}\n\n\
         effect Greeter {\n    fn greet() -> Str\n}\n\n\
         handler FixedGreeter(text: Str) of Greeter {\n    \
         fn greet() -> Str {\n        return \"hi\"\n    }\n}\n\n\
         fn read(s: Str) -> [s] None {\n}\n\n\
         fn copy_remedies() [use] {\n    let s = \"x\"\n    \
         let t = (copy(s), 1)\n    let a = [copy(s)]\n    \
         let b = Box {item: copy(s)}\n    let c = Box {...copy(b)}\n    \
         use FixedGreeter(copy(s))\n    read(s)\n    read(b.item)\n}\n\n\
         fn revive() {\n    let s = \"x\"\n    let t = (s, 1)\n    s = \"y\"\n    read(s)\n}\n\n\
         fn yield_then_reassign() -> Iter<Str> {\n    let s = \"x\"\n    \
         for i in [1, 2] {\n        yield s\n        s = \"y\"\n    }\n}\n\n\
         fn break_consumes_only_its_operand() {\n    let s = \"x\"\n    let n = 0\n    \
         let r = while n < 3 {\n        if n == 2 {\n            break n\n        }\n        \
         n++\n    }\n    read(s)\n}\n\n\
         fn return_poisons_only_exiting_path(flag: Bool) -> Str {\n    let s = \"x\"\n    \
         let d = s\n    if flag {\n        return s\n    }\n    return copy(d)\n}\n\n\
         fn main() {\n    revive()\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [fate-move-mode] S2 positive matrix: move-mode bindings make consuming
// pipelines legal with zero copies — moving/mutating a derived variable
// whose ancestors are owned (locals, or parameters the fn's inferred
// contract moves) consumes the ancestors at the binding. Loop bindings
// are fresh per iteration (consuming one is legal), projections of
// *immutable* data in moved positions stay untracked (clone-vs-alias is
// unobservable), and `copy` still severs where the source must stay
// usable.
#[test]
fn s2_move_mode_pipelines_stay_clean() {
    let dir = src_dir("s2_positive");
    fs::write(
        dir.join("main.sv"),
        "struct Person {\n    name: Str,\n    age: Int\n}\n\n\
         struct Label {\n    text: Str\n}\n\n\
         fn consume(text: Str) -> [] None {\n}\n\n\
         fn longest_name(persons: List<Person>) -> Str {\n    let longest = \"\"\n    \
         for person in persons {\n        let name = person.name\n        \
         if size(name) > size(longest) {\n            longest = name\n        }\n    }\n    \
         return longest\n}\n\n\
         fn per_iteration() {\n    for s in list(\"a\", \"b\") {\n        consume(s)\n    }\n}\n\n\
         fn immutable_projection_store(person: Person) -> [person] Label {\n    \
         return Label {text: person.name}\n}\n\n\
         fn copy_keeps_source() -> Int {\n    let xs = list(1, 2)\n    let ys = copy(xs)\n    \
         consume_list(ys)\n    return size(xs)\n}\n\n\
         fn consume_list(v: List<Int>) -> [] None {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         let people = list(Person {name: \"Ada\", age: 36}, Person {name: \"Grace\", age: 45})\n    \
         println(longest_name(people))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// [fate-move-mode] S2 negative matrix: the poison lands at the binding —
// using an ancestor after a move-mode binding errors naming the binding;
// a written-kept parameter ancestor keeps the S1 move-site error; and a
// projection of *mutable* data in a moved position consumes its owned
// roots (closing the 2026-09-02 parity divergence) or errors for a
// kept-parameter root.
#[test]
fn s2_move_mode_ancestors_are_consumed() {
    let dir = src_dir("s2_negative");
    fs::write(
        dir.join("main.sv"),
        "struct Holder {\n    tags: Mut List<Int>\n}\n\n\
         struct Wrapper {\n    item: Mut List<Int>\n}\n\n\
         fn wrap(list: Mut List<Int>) -> [] Wrapper {\n    return Wrapper {item: list}\n}\n\n\
         fn consume_list(v: List<Int>) -> [] None {\n}\n\n\
         fn chain() -> Int {\n    let xs = list(1, 2)\n    let a = xs\n    let b = a\n    \
         consume_list(b)\n    return size(xs)\n}\n\n\
         fn parity_probe() -> Int {\n    let h = Holder {tags: mutable_list(1)}\n    \
         let w = wrap(h.tags)\n    add(h.tags, 9)\n    return size(w.item)\n}\n\n\
         fn kept_leak(h: Holder) -> [h] Wrapper {\n    return wrap(h.tags)\n}\n\n\
         fn kept_remedy(h: Holder) -> [h] Wrapper {\n    return wrap(copy(h.tags))\n}\n\n\
         fn kept_binding(v: Str) -> [v] Str {\n    let w = v\n    return w\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Chain: `xs` was consumed at `a`'s binding (the whole chain is
    // move-mode); the use-site error names the binding.
    assert!(
        stderr.contains(
            "`xs` cannot be used here: `a` was bound from it and later moves the value"
        ),
        "stderr: {stderr}"
    );
    // The parity probe is rejected: `wrap(h.tags)` moved mutable data
    // out of `h`, so the later `add(h.tags, 9)` cannot observe an alias
    // on one backend and a clone on the other.
    assert!(
        stderr.contains(
            "`h` cannot be used here: it was consumed (moved) by a move of \
             mutable data projected out of it"
        ),
        "stderr: {stderr}"
    );
    // A kept parameter's mutable data cannot be moved out; `copy` is the
    // remedy (kept_remedy is clean).
    assert!(
        stderr.contains(
            "cannot move mutable data out of `h`: it is a kept parameter"
        ),
        "stderr: {stderr}"
    );
    // A written-kept parameter binding keeps the S1 error at the move
    // site [fate-derived-readonly].
    assert!(
        stderr.contains("cannot return `w`: it was bound from `v`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("4 errors"), "stderr: {stderr}");
}

// [deduce-same-call] L3: arguments are evaluated left to right, so a
// value moved by an earlier argument of the same call cannot be
// mentioned by a later one — double moves (`f(a, a)`), move-then-read
// (`f(a, size(a))`), and interpolated mentions are all rejected at the
// later argument. `copy` at the consuming argument is the remedy;
// kept-position reads are unaffected.
#[test]
fn l3_same_call_ordering() {
    let dir = src_dir("l3_same_call");
    fs::write(
        dir.join("main.sv"),
        "fn eat_two(a: List<Int>, b: List<Int>) -> [] None {\n}\n\n\
         fn consume_first(a: List<Int>, n: Int) -> [] None {\n}\n\n\
         fn keep_first(a: List<Int>, n: Int) -> [a] None {\n}\n\n\
         fn double_move() {\n    let xs = list(1, 2)\n    eat_two(xs, xs)\n}\n\n\
         fn move_then_read() {\n    let xs = list(1, 2)\n    consume_first(xs, size(xs))\n}\n\n\
         fn remedy() {\n    let xs = list(1, 2)\n    eat_two(copy(xs), xs)\n}\n\n\
         fn kept_then_read() {\n    let ys = list(3)\n    keep_first(ys, size(ys))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert_eq!(
        stderr
            .matches(
                "`xs` cannot be used here: it was consumed (moved) by an earlier \
                 argument of this call"
            )
            .count(),
        2,
        "stderr: {stderr}"
    );
    // remedy and kept_then_read are clean: exactly the two violations.
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
}

// [fate-lambda] L4: lambdas are ordinary values with shared fate.
// Immutable captures are free; a transitively-mutable read capture
// fate-links the closure to the variable (mutating it poisons the
// closure); a mutated capture is consumed at lambda creation (claimed
// when the enclosing fn's contract is inferable, an error when it is a
// written-kept parameter); and a lambda can never consume a capture
// (it may run any number of times) — `copy` is the remedy everywhere.
#[test]
fn l4_lambda_captures() {
    let dir = src_dir("l4_captures");
    fs::write(
        dir.join("main.sv"),
        "fn apply(f: (Int) -> Int, v: Int) -> Int {\n    return f(v)\n}\n\n\
         fn run(f: () -> None) {\n    f()\n}\n\n\
         fn consume_list(v: List<Int>) -> [] None {\n}\n\n\
         fn immutable_free() -> Int {\n    let base = 10\n    let text = \"hi\"\n    \
         let f = (n: Int) -> { return n + base + size(text) }\n    \
         let r = apply(f, 1)\n    return r + base + size(text)\n}\n\n\
         fn poisoned_after_mutation() -> Int {\n    let xs = mutable_list(1, 2)\n    \
         let f = (n: Int) -> { return n + size(xs) }\n    let before = apply(f, 1)\n    \
         add(xs, 9)\n    return apply(f, 1)\n}\n\n\
         fn used_before_mutation() -> Int {\n    let xs = mutable_list(1, 2)\n    \
         let f = (n: Int) -> { return n + size(xs) }\n    let r = apply(f, 1)\n    \
         add(xs, 9)\n    return r + size(xs)\n}\n\n\
         fn mutate_capture_consumes() -> Int {\n    let xs = mutable_list(1, 2)\n    \
         let g = () -> { add(xs, 1) }\n    run(g)\n    return size(xs)\n}\n\n\
         fn mutate_capture_remedy() -> Int {\n    let xs = mutable_list(1, 2)\n    \
         let snapshot = copy(xs)\n    let g = () -> { add(snapshot, 1) }\n    run(g)\n    \
         return size(xs)\n}\n\n\
         fn move_capture_rejected() {\n    let xs = list(1, 2)\n    \
         let h = () -> { consume_list(xs) }\n    run(h)\n}\n\n\
         fn move_capture_remedy() {\n    let xs = list(1, 2)\n    \
         let h = () -> { consume_list(copy(xs)) }\n    run(h)\n}\n\n\
         fn kept_mutates(xs: Mut List<Int>) -> [xs: Mut] None {\n    \
         let g = () -> { add(xs, 1) }\n    run(g)\n}\n\n\
         fn infer_claims(xs: Mut List<Int>) {\n    let g = () -> { add(xs, 1) }\n    run(g)\n}\n\n\
         fn claim_reaches_caller() -> Int {\n    let xs = mutable_list(1)\n    \
         infer_claims(xs)\n    return size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Mutating a mutable read-capture's root poisons the closure.
    assert!(
        stderr.contains(
            "`f` cannot be used here: it was bound from `xs` and shares its fate, \
             and `xs` was mutated after the binding"
        ),
        "stderr: {stderr}"
    );
    // A mutated capture is consumed at creation.
    assert!(
        stderr.contains(
            "`xs` cannot be used here: it was consumed (moved) by a lambda that \
             captures and mutates it"
        ),
        "stderr: {stderr}"
    );
    // A capture-consuming lambda is legal since L7b but `Once`-typed
    // [once-fn]: passing it where a plain fn is expected fails at the
    // boundary.
    assert!(
        stderr.contains("no matching overload for `run(Once () -> "),
        "stderr: {stderr}"
    );
    // A written-kept parameter cannot be captured-and-mutated.
    assert!(
        stderr.contains(
            "this lambda captures and mutates `xs`, which is a kept parameter"
        ),
        "stderr: {stderr}"
    );
    // The capture claim propagates: the caller's argument is consumed.
    assert!(
        stderr.contains(
            "`xs` cannot be used here: it was consumed (moved) by an earlier call"
        ),
        "stderr: {stderr}"
    );
    // The positive matrix (immutable captures, pre-mutation use, `copy`
    // remedies) is clean: exactly the five violations.
    assert!(stderr.contains("5 errors"), "stderr: {stderr}");
}

// [linear-obligation] L6: values of `with Linear` types must be used —
// moved onward or `discard`ed — on every path. The negative matrix:
// scope-exit leak, consumed-on-some-paths-only, dropped expression
// result, overwriting a live value, returning while owing, `copy`
// refused, generic instantiation refused, and a lambda swallowing an
// obligation. The positive matrix (pass onward, discard, return with
// the caller obligated, kept-parameter borrow, derived alias, linear
// composite) is clean.
#[test]
fn l6_linear_obligations() {
    let dir = src_dir("l6_linear");
    fs::write(
        dir.join("main.sv"),
        "struct FileHandle with Linear {\n    fd: Int\n}\n\n\
         struct Box2 with Linear {\n    item: FileHandle\n}\n\n\
         struct Conn with Linear {\n    tags: Mut List<Int>\n}\n\n\
         fn open_file(path: Str) -> [] FileHandle {\n    \
         return FileHandle {fd: size(path)}\n}\n\n\
         fn close_file(h: FileHandle) -> [] None {\n    discard(h)\n}\n\n\
         fn close_box(b: Box2) -> [] None {\n    discard(b)\n}\n\n\
         fn inspect(h: FileHandle) -> [h] Int {\n    return h.fd\n}\n\n\
         fn run(f: () -> None) {\n    f()\n}\n\n\
         fn leak() {\n    let h = open_file(\"data.txt\")\n}\n\n\
         fn maybe_leak(flag: Bool) {\n    let h = open_file(\"data.txt\")\n    \
         if flag {\n        close_file(h)\n    }\n}\n\n\
         fn drop_result() {\n    open_file(\"data.txt\")\n}\n\n\
         fn overwrite() {\n    let h = open_file(\"a.txt\")\n    h = open_file(\"b.txt\")\n    \
         close_file(h)\n}\n\n\
         fn return_while_owing(flag: Bool) -> Int {\n    let h = open_file(\"data.txt\")\n    \
         if flag {\n        return 0\n    }\n    close_file(h)\n    return 1\n}\n\n\
         fn copy_refused() {\n    let h = open_file(\"data.txt\")\n    let c = copy(h)\n    \
         close_file(h)\n    close_file(c)\n}\n\n\
         fn swallow() {\n    let conn = Conn {tags: mutable_list(1)}\n    \
         let g = () -> { add(conn.tags, 2) }\n    run(g)\n    discard(conn)\n}\n\n\
         fn ok_pass() {\n    let h = open_file(\"data.txt\")\n    close_file(h)\n}\n\n\
         fn ok_discard() {\n    let h = open_file(\"data.txt\")\n    discard(h)\n}\n\n\
         fn ok_return() -> FileHandle {\n    let h = open_file(\"data.txt\")\n    return h\n}\n\n\
         fn ok_kept_borrow() {\n    let h = open_file(\"data.txt\")\n    \
         let n = inspect(h)\n    close_file(h)\n}\n\n\
         fn ok_alias() {\n    let h = open_file(\"data.txt\")\n    let alias = h\n    \
         let n = inspect(alias)\n    close_file(h)\n}\n\n\
         fn ok_composite() {\n    let h = open_file(\"data.txt\")\n    \
         let b = Box2 {item: h}\n    close_box(b)\n}\n\n\
         fn caller_obligated() {\n    let h = ok_return()\n    close_file(h)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`h` still owns a linear value when it goes out of scope"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(
            "`h` owns a linear value that is consumed on some paths but not others"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(
            "this expression produces a linear value that is dropped immediately"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("assigning to `h` drops the linear value it still owns"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("cannot return while `h` still owns a linear value"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("cannot `copy` a value of linear type `FileHandle`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(
            "this lambda captures and mutates `conn`, which holds a linear value"
        ),
        "stderr: {stderr}"
    );
    // Exactly the seven violations: the positive matrix is clean.
    assert!(stderr.contains("7 errors"), "stderr: {stderr}");
}

// [linear-generics] Generic instantiation with a linear type is refused
// unless the type parameter declares `<T with Linear>`; variadic
// positions refuse linear values outright (they are untracked).
#[test]
fn l6_generics_refuse_linear_types() {
    let dir = src_dir("l6_generics");
    fs::write(
        dir.join("main.sv"),
        "struct FileHandle with Linear {\n    fd: Int\n}\n\n\
         fn hold<T>(value: T) -> T {\n    return value\n}\n\n\
         fn generic_refused() {\n    let h = FileHandle {fd: 1}\n    \
         let kept = hold(h)\n    discard(kept)\n}\n\n\
         fn variadic_refused() {\n    let h = FileHandle {fd: 1}\n    \
         let xs = list(h)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "cannot instantiate generic parameter `T` of `hold` with linear type \
             `FileHandle`: `hold` does not declare `<T with Linear>`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("a linear value cannot be passed in a variadic position"),
        "stderr: {stderr}"
    );
}

// [linear-generics] L7a: `<T with Linear>` opts a generic fn into
// linear instantiation — its body treats `T` values as linear (a
// written-moved parameter that the body drops is rejected), forwarding
// an opted `T` to an *unopted* generic is rejected, only `Linear` is
// accepted in the clause, and the opted std surface makes the
// `List<FileHandle>` workflow legal (construct empty, `add`
// individually — variadic positions refuse linear values — `size`,
// `discard`).
#[test]
fn l7a_generic_linear_opt_in() {
    let dir = src_dir("l7a_optin");
    fs::write(
        dir.join("main.sv"),
        "struct FileHandle with Linear {\n    fd: Int\n}\n\n\
         fn open_file(n: Int) -> [] FileHandle {\n    return FileHandle {fd: n}\n}\n\n\
         fn hold<T with Linear>(value: T) -> T {\n    return value\n}\n\n\
         fn eat<T with Linear>(value: T) -> [] None {\n}\n\n\
         fn forward<T with Linear>(value: T) {\n    let kept = unopted(value)\n    \
         discard(kept)\n}\n\n\
         fn unopted<T>(value: T) -> T {\n    return value\n}\n\n\
         fn bad_clause<T with Mut>(value: T) -> T {\n    return value\n}\n\n\
         fn variadic_refused() {\n    let h = open_file(1)\n    \
         let xs = mutable_list(h)\n}\n\n\
         fn workflow() -> Int {\n    let handles: Mut List<FileHandle> = mutable_list()\n    \
         add(handles, open_file(1))\n    add(handles, open_file(2))\n    \
         let n = size(handles)\n    discard(handles)\n    return n\n}\n\n\
         fn ok_hold() {\n    let h = hold(open_file(9))\n    discard(h)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // The opted body treats `T` as linear: a written-moved parameter
    // dropped by the body is a leak.
    assert!(
        stderr.contains("`value` still owns a linear value when it goes out of scope"),
        "stderr: {stderr}"
    );
    // Forwarding to an unopted generic is rejected.
    assert!(
        stderr.contains(
            "cannot instantiate generic parameter `T` of `unopted` with linear \
             type `T`: `unopted` does not declare `<T with Linear>`"
        ),
        "stderr: {stderr}"
    );
    // Only `Linear` is supported in the clause.
    assert!(
        stderr.contains("only `Linear` is supported in a type-parameter `with` clause"),
        "stderr: {stderr}"
    );
    // Variadic positions refuse linear values.
    assert!(
        stderr.contains("a linear value cannot be passed in a variadic position"),
        "stderr: {stderr}"
    );
    // workflow and ok_hold are clean. Six errors total: the four above
    // plus the two follow-on leaks in `variadic_refused` (the refused
    // `h` is never discharged, and `xs` — a linear composite — leaks).
    assert!(stderr.contains("6 errors"), "stderr: {stderr}");
}

// [once-fn] L7b: `Once` on fn types means callable at most once,
// enforced by consumption — a second call, a call in a loop, and a
// call after passing the value on all error; a maybe-call is fine
// ("at most once"). Subtyping is inverted (plain fns fit `Once`
// positions, never the reverse), a capture-consuming lambda is legal
// but `Once`-typed, and `Once` applies only to fn types.
#[test]
fn l7b_once_fns() {
    let dir = src_dir("l7b_once");
    fs::write(
        dir.join("main.sv"),
        "fn run_once(f: Once () -> None) {\n    f()\n}\n\n\
         fn run_twice_bad(f: Once () -> None) {\n    f()\n    f()\n}\n\n\
         fn run_in_loop_bad(f: Once () -> None) {\n    for i in list(1, 2) {\n        f()\n    }\n}\n\n\
         fn run_maybe(f: Once () -> None, flag: Bool) {\n    if flag {\n        f()\n    }\n}\n\n\
         fn run_plain(f: () -> None) {\n    f()\n    f()\n}\n\n\
         fn consume_list(v: List<Int>) -> [] None {\n}\n\n\
         fn once_where_plain_bad() {\n    let xs = list(1, 2)\n    \
         let g = () -> { consume_list(xs) }\n    run_plain(g)\n}\n\n\
         fn once_where_once_ok() {\n    let xs = list(1, 2)\n    \
         let g = () -> { consume_list(xs) }\n    run_once(g)\n}\n\n\
         fn plain_where_once_ok() {\n    let n = 5\n    \
         let g = () -> { let m = n + 1 }\n    run_once(g)\n}\n\n\
         fn escape_rule(f: Once () -> None) {\n    run_once(f)\n    f()\n}\n\n\
         fn not_a_fn(x: Once Int) {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Second call and loop back-edge call: consumed by the first call.
    assert_eq!(
        stderr
            .matches(
                "`f` cannot be used here: it was consumed (moved) by a call \
                 (a `Once` function is callable at most once)"
            )
            .count(),
        2,
        "stderr: {stderr}"
    );
    // A `Once` lambda cannot go where a plain fn is expected.
    assert!(
        stderr.contains("no matching overload for `run_plain(Once () -> "),
        "stderr: {stderr}"
    );
    // Passing a `Once` value on consumes it (escape rule).
    assert!(
        stderr.contains(
            "`f` cannot be used here: it was consumed (moved) by an earlier call"
        ),
        "stderr: {stderr}"
    );
    // `Once` applies only to fn types.
    assert!(
        stderr.contains("`Once` applies only to function types, not `Int`"),
        "stderr: {stderr}"
    );
    // run_maybe, once_where_once_ok, plain_where_once_ok are clean.
    assert!(stderr.contains("5 errors"), "stderr: {stderr}");
}

// [readonly-return] L7c: `-> ReadOnly[from: p] T?` marks a derived
// return — the callee may return projections/elements of the kept
// parameter `p` without `copy`, and the caller's result fate-links to
// the argument. Violations: returning an independent value, annotating
// a moved or unknown parameter; and the caller-side discipline holds —
// mutating the argument poisons the result, and the borrowed result
// can never be moved (move-mode refuses borrowed links).
#[test]
fn l7c_derived_returns() {
    let dir = src_dir("l7c_derived");
    fs::write(
        dir.join("main.sv"),
        "struct Person {\n    name: Str,\n    age: Int\n}\n\n\
         fn take(p: Person) -> [] None {\n}\n\n\
         fn find_adult(persons: List<Person>) -> [persons] ReadOnly[from: persons] Person? {\n    \
         for person in persons {\n        if person.age >= 18 {\n            return person\n        }\n    }\n    \
         return None\n}\n\n\
         fn forwarded(persons: List<Person>, tag: Str) -> [persons, tag] ReadOnly[from: persons] Person? {\n    \
         return first(persons)\n}\n\n\
         fn bad_independent(persons: List<Person>) -> [persons] ReadOnly[from: persons] Person? {\n    \
         return Person {name: \"made up\", age: 1}\n}\n\n\
         fn bad_moved(persons: List<Person>) -> [] ReadOnly[from: persons] Person? {\n    \
         return None\n}\n\n\
         fn bad_param(persons: List<Person>) -> [persons] ReadOnly[from: nobody] Person? {\n    \
         return None\n}\n\n\
         fn poison_after_mutation() -> Int {\n    \
         let people = mutable_list(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    add(people, Person {name: \"Grace\", age: 45})\n    \
         if head is Person h {\n        return h.age\n    }\n    return 0\n}\n\n\
         fn escape_borrowed() {\n    let people = list(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        take(h)\n    }\n}\n\n\
         fn ok_use() -> Int {\n    let people = list(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        return h.age\n    }\n    \
         return 0\n}\n\n\
         fn ok_copy_escape() {\n    let people = list(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        take(copy(h))\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "this function returns `ReadOnly[from: persons]`, so every returned \
             value must be derived from `persons`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`ReadOnly[from: persons]` requires `persons` to be kept"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`ReadOnly[from: nobody]` names no parameter"),
        "stderr: {stderr}"
    );
    // Mutating the argument poisons the derived result.
    assert!(
        stderr.contains(
            "`head` cannot be used here: it was bound from `people` and shares \
             its fate, and `people` was mutated after the binding"
        ),
        "stderr: {stderr}"
    );
    // The borrowed result can never be moved; `copy` is the remedy
    // (ok_copy_escape is clean).
    assert!(
        stderr.contains("cannot move `h`: it was bound from `head`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("5 errors"), "stderr: {stderr}");
}
