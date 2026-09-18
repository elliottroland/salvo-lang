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
    let out = salvo(&[
        "analyze",
        "--src",
        dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success());
    assert!(stdout.trim().starts_with('['), "stdout: {stdout}");
    assert!(stdout.trim().ends_with(']'), "stdout: {stdout}");
    assert!(stdout.contains("\"file\": \"bad.sv\""), "stdout: {stdout}");
    assert!(stdout.contains("\"line\": 2"), "stdout: {stdout}");
    assert!(stdout.contains("\"col\": 18"), "stdout: {stdout}");
    assert!(
        stdout.contains("\"severity\": \"error\""),
        "stdout: {stdout}"
    );
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
    let out = salvo(&[
        "analyze",
        "--src",
        dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
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

    let out = salvo(&[
        "analyze",
        "--src",
        dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
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
        "export effect Audit {\n    fn audit(message: Str)\n}\n",
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
    assert!(
        stderr.contains("unknown effect `Audit`"),
        "stderr: {stderr}"
    );
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

    // All-paths-return via if/else, a `yield fn` (its body produces elements,
    // not a return value) and None-returning fns are fine.
    fs::write(
        dir.join("bad.sv"),
        "fn sign(x: Int) -> Int {\n    if x < 0 {\n        return -1\n    } else {\n        return 1\n    }\n}\n\
         struct Nums {\n    limit: Int\n}\n\
         iter fn next(n: Nums) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    if at >= n.limit {\n        \
         return finished()\n    }\n    at = at + 1\n    return emitted(copy(at))\n}\n\
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

// [deduce-consume] A written `!p` consumes the identifier argument
// uniformly across all types: its type narrows to `Nothing` and later uses
// are errors; reassignment revives it; kept parameters are unaffected.
// Consumption on an always-exiting branch does not leak past the branch.
#[test]
fn use_after_consume_is_an_error() {
    let dir = src_dir("consume");
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Int, b: Int) -> Int => !a, !b {\n    return a + b\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let a = 1\n    let b = 2\n    \
         let c = a.add(b)\n    println(\"${a}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`a` cannot be used here: it was consumed (moved) by an earlier call"),
        "stderr: {stderr}"
    );

    // Reassignment revives the variable; kept parameters (`[a]`) are
    // never consumed; consumption inside an always-exiting `if` branch
    // never reaches the code after the `if`.
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Int, b: Int) -> Int {\n    return a + b\n}\n\n\
         fn double(a: Int) -> Int => a {\n    return a + a\n}\n\n\
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
        "fn consume(text: Str) -> None => !text {\n}\n\n\
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
         let strings = list_of(\"a\", \"b\")\n    give_back(strings)\n    \
         println(\"${strings.size()}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr
            .contains("`strings` cannot be used here: it was consumed (moved) by an earlier call"),
        "stderr: {stderr}"
    );

    // Rebinding through the return value keeps it usable.
    fs::write(
        dir.join("main.sv"),
        "fn give_back(list: List<Str>) -> List<Str> {\n    return list\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let strings = list_of(\"a\", \"b\")\n    strings = give_back(strings)\n    \
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
// `take_head` (`[list: Mut]` on a `NonEmpty Mut` param) the variable
// is no longer `NonEmpty`, so a second call fails overload resolution.
// The explicit-empty `[list:]` form strips all declared qualifiers.
#[test]
fn calls_remove_qualifiers_per_declared_deductions() {
    let dir = src_dir("dedu_quals");
    fs::write(
        dir.join("main.sv"),
        "qualifier NonEmpty<T> of List<T> {\n    fn qualifies(list: List<T>) -> Bool {\n        return list.size() > 0\n    }\n}\n\n\
         fn take_head<T>(list: NonEmpty Mut List<T>) -> T => list: Mut {\n    return list.get(0)!\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let strings = mut_list_of(\"a\", \"b\")\n    \
         if strings is NonEmpty {\n        let s = take_head(strings)\n        let t = take_head(strings)\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `take_head(Mut List<Str>)`"),
        "stderr: {stderr}"
    );

    // `[list:]` strips every declared qualifier: after `weaken` the value
    // no longer satisfies a `NonEmpty`-requiring overload.
    fs::write(
        dir.join("main.sv"),
        "qualifier NonEmpty<T> of List<T> {\n    fn qualifies(list: List<T>) -> Bool {\n        return list.size() > 0\n    }\n}\n\n\
         fn weaken<T>(list: NonEmpty List<T>) -> None => list: None {\n}\n\n\
         fn head<T>(list: NonEmpty List<T>) -> T {\n    return list.get(0)!\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let strings = list_of(\"a\", \"b\")\n    \
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
         fn consume(strings: List<Str>) -> None => !strings {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let strings = list_of(\"a\", \"b\")\n    if strings is NonEmpty {\n        consume(strings)\n    }\n    println(\"${strings.size()}\")\n    \
         let s = list_of(\"x\")\n    let i = 0\n    while i < 3 {\n        println(\"${s.size()}\")\n        consume(s)\n        i++\n    }\n}\n",
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
    assert!(
        stderr.contains("`s` cannot be used here"),
        "stderr: {stderr}"
    );

    // Consume-then-revive inside the body is clean across iterations.
    fs::write(
        dir.join("main.sv"),
        "fn consume(strings: List<Str>) -> None => !strings {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let s = list_of(\"x\")\n    let i = 0\n    while i < 3 {\n        consume(s)\n        s = list_of(\"y\")\n        i++\n    }\n    println(\"${s.size()}\")\n}\n",
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
    // `Ok`/`Err` and their constructors come from `core.result`.
    let prelude = "fn consume(v: Ok Str) -> None => !v {\n}\n\n";

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
    let prelude = "fn consume(strings: List<Str>) -> None => !strings {\n}\n\n";

    // Both branches consume -> consumed after.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn both(flag: Bool) {{\n    let a = list_of(\"a\")\n    \
             if flag {{\n        consume(a)\n    }} else {{\n        consume(a)\n    }}\n    \
             let n = a.size()\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`a` cannot be used here"),
        "stderr: {stderr}"
    );

    // One branch consumes, the other exits: the only fall-through path
    // consumed it -> consumed after.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn one_exits(flag: Bool) {{\n    let b = list_of(\"b\")\n    \
             if flag {{\n        consume(b)\n    }} else {{\n        return\n    }}\n    \
             let n = b.size()\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`b` cannot be used here"),
        "stderr: {stderr}"
    );

    // The consuming branch always exits -> clean after.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn consuming_exits(flag: Bool) {{\n    let c = list_of(\"c\")\n    \
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
         fn take_head<T>(list: NonEmpty Mut List<T>) -> T => list: Mut {\n    return list.get(0)!\n}\n\n\
         fn partial(flag: Bool) {\n    let strings = mut_list_of(\"a\", \"b\")\n    \
         if strings is NonEmpty {\n        if flag {\n            let x = take_head(strings)\n        }\n        let y = take_head(strings)\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `take_head(Mut List<Str>)`"),
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
        "fn consume(strings: List<Str>) -> None => !strings {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    let s = list_of(\"x\")\n    \
         for i in list_of(1, 2, 3).iter() {\n        println(\"${s.size()}\")\n        consume(s)\n    }\n}\n",
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
        "fn describe(v: Ok Str | Err Str) -> Str {\n    when v {\n        is Ok {\n            return \"ok\"\n        }\n        is Err {\n            return \"err\"\n        }\n    }\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    println(describe(ok(\"x\")))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

// [qual-result-tags] [qual-ctor-fn] `core.result` supplies the result tags: a program can
// write `Ok Int | Err Str` and call `ok`/`err` without declaring anything,
// and `is`/`when` narrow the arms as they do for local qualifiers.
#[test]
fn std_supplies_the_result_tags() {
    let dir = src_dir("std_result");
    fs::write(
        dir.join("main.sv"),
        "fn parse_age(input: Int) -> Ok Int | Err Str | None {\n    \
         if input >= 0 {\n        return ok(input)\n    }\n    \
         return err(\"negative age\")\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let result = parse_age(36)\n    \
         when result {\n        is Ok {\n            println(\"age ${result}\")\n        }\n        \
         is Err {\n            println(\"error: ${result}\")\n        }\n        \
         is None {\n            println(\"none\")\n        }\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

// [name-resolve] An undeclared qualifier is reported as the unresolved
// name it is. Regression: `is Ok` with no `Ok` in scope used to report
// "this check can never succeed" (the name was read as a base type that
// no union arm matched) and then cascade onto the subject.
#[test]
fn undeclared_qualifier_reports_the_name() {
    let dir = src_dir("unknown_qual");
    fs::write(
        dir.join("main.sv"),
        "fn f() -> Yes Int | No Str {\n    return 1\n}\n\n\
         fn main() [use] {\n    use StdOutConsole\n    \
         let v = f()\n    if v is Yes {\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("unknown qualifier `Yes`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("unknown qualifier `No`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("unknown type or qualifier `Yes`"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("can never succeed"), "stderr: {stderr}");
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
        "fn consume(v: List<Int>) -> None => !v {\n}\n\n\
         fn move_derived() -> Int {\n    let xs = list_of(1, 2)\n    let ys = xs\n    consume(ys)\n    return size(xs)\n}\n\n\
         fn mutate_derived() -> Int {\n    let xs = mut_list_of(1, 2)\n    let ys = xs\n    add(ys, 3)\n    return size(xs)\n}\n\n\
         fn return_derived(v: Str) -> Str => v {\n    let w = v\n    return w\n}\n\n\
         fn read_derived(v: Str) -> Int => v {\n    let w = v\n    return size(w)\n}\n",
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
            .matches("`xs` cannot be used here: `ys` was bound from it and later moves the value")
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
        "fn consume(v: List<Int>) -> None => !v {\n}\n\n\
         fn mutated() -> Int {\n    let xs = mut_list_of(1)\n    let ys = xs\n    \
         add(xs, 2)\n    return size(ys)\n}\n\n\
         fn moved() -> Int {\n    let xs = list_of(1)\n    let ys = xs\n    \
         consume(xs)\n    return size(ys)\n}\n\n\
         fn reassigned() -> Int {\n    let xs = list_of(1)\n    let ys = xs\n    \
         xs = list_of(2, 3)\n    return size(ys)\n}\n\n\
         fn unused_poison_is_fine() {\n    let xs = mut_list_of(1)\n    let ys = xs\n    \
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
    assert!(
        stderr.contains("`xs` was moved after the binding"),
        "stderr: {stderr}"
    );
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
         fn longest_name(persons: Person[]) -> Str => persons {\n    let longest = \"\"\n    \
         for person in persons {\n        if size(longest) < size(person.name) {\n            \
         longest = person.name\n        }\n    }\n    return longest\n}\n\n\
         fn is_binding(v: Str | Int) -> Str => v {\n    if v is Str s {\n        return s\n    }\n    \
         return \"other\"\n}\n\n\
         fn revived(p: Person) -> Str => p {\n    let n = p.name\n    n = \"fresh\"\n    return n\n}\n",
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
    assert!(
        stderr.contains("cannot return `s`: it was bound from `v`"),
        "stderr: {stderr}"
    );
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
         fn longest_name(persons: Person[]) -> Str => persons {\n    let longest = \"\"\n    \
         for person in persons {\n        if size(longest) < size(person.name) {\n            \
         longest = person.name\n        }\n    }\n    return copy(longest)\n}\n\n\
         fn independent() -> Int {\n    let xs = mut_list_of(1)\n    let ys = copy(xs)\n    \
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
        "fn pick(a: List<Int>, cond: Bool) -> List<Int> => a {\n    \
         let out = list_of(0)\n    \
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
        "struct Person canbe Mut {\n    name: Str\n}\n\n\
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
         fn diverged() -> Int {\n    let h = Holder {tags: mut_list_of(1)}\n    \
         let t = h.tags\n    add(h.tags, 2)\n    return size(t)\n}\n\n\
         fn mutate_through_derived(other: Holder) -> None => other {\n    \
         let h = other\n    add(h.tags, 2)\n}\n\n\
         fn remedy() -> Int {\n    let h = Holder {tags: mut_list_of(1)}\n    \
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
// branch), and the remedies that keep each of them clean
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
         fn read(s: Str) -> None => s {\n}\n\n\
         fn tuple_store() {\n    let s = \"x\"\n    let t = (s, 1)\n    read(s)\n}\n\n\
         fn array_store() {\n    let s = \"x\"\n    let a = [s]\n    read(s)\n}\n\n\
         fn struct_store() {\n    let s = \"x\"\n    let b = Box {item: s}\n    read(s)\n}\n\n\
         fn spread_store() {\n    let b = Box {item: \"x\"}\n    let c = Box {...b}\n    \
         read(b.item)\n}\n\n\
         fn break_in_branch() {\n    let s = \"x\"\n    let r = while true {\n        \
         if true {\n            break s\n        }\n    }\n    read(s)\n}\n\n\
         fn use_ctor() [use] {\n    let s = \"hi\"\n    use FixedGreeter(s)\n    read(s)\n}\n\n\
         fn store_derived() {\n    let xs = list_of(1, 2)\n    let ys = xs\n    let t = (ys, 1)\n    \
         read_list(xs)\n}\n\n\
         fn read_list(v: List<Int>) -> None => v {\n}\n\n\
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
        stderr
            .contains("`xs` cannot be used here: `ys` was bound from it and later moves the value"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`b` cannot be used here: `d` was bound from it and later moves the value"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("8 errors"), "stderr: {stderr}");
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
         fn read(s: Str) -> None => s {\n}\n\n\
         fn copy_remedies() [use] {\n    let s = \"x\"\n    \
         let t = (copy(s), 1)\n    let a = [copy(s)]\n    \
         let b = Box {item: copy(s)}\n    let c = Box {...copy(b)}\n    \
         use FixedGreeter(copy(s))\n    read(s)\n    read(b.item)\n}\n\n\
         fn revive() {\n    let s = \"x\"\n    let t = (s, 1)\n    s = \"y\"\n    read(s)\n}\n\n\
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
         fn consume(text: Str) -> None => !text {\n}\n\n\
         fn longest_name(persons: List<Person>) -> Str {\n    let longest = \"\"\n    \
         for person in persons {\n        let name = person.name\n        \
         if size(name) > size(longest) {\n            longest = name\n        }\n    }\n    \
         return longest\n}\n\n\
         fn per_iteration() {\n    for s in list_of(\"a\", \"b\") {\n        consume(s)\n    }\n}\n\n\
         fn immutable_projection_store(person: Person) -> Label => person {\n    \
         return Label {text: person.name}\n}\n\n\
         fn copy_keeps_source() -> Int {\n    let xs = list_of(1, 2)\n    let ys = copy(xs)\n    \
         consume_list(ys)\n    return size(xs)\n}\n\n\
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         let people = list_of(Person {name: \"Ada\", age: 36}, Person {name: \"Grace\", age: 45})\n    \
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

// [fate-move-mode] [fate-partial-move] S2 negative matrix: the poison lands
// at the binding — using an ancestor after a move-mode binding errors naming
// the binding; a written-kept parameter ancestor keeps the S1 move-site
// error; and a projection of *mutable* data in a moved position leaves the
// root usable but records the moved place, so reading *that* projection back
// is the error (closing the 2026-09-02 parity divergence — L5 made the
// diagnostic name the field rather than the whole variable).
#[test]
fn s2_move_mode_ancestors_are_consumed() {
    let dir = src_dir("s2_negative");
    fs::write(
        dir.join("main.sv"),
        "struct Holder {\n    tags: Mut List<Int>\n}\n\n\
         struct Wrapper {\n    item: Mut List<Int>\n}\n\n\
         fn wrap(list: Mut List<Int>) -> Wrapper => !list {\n    return Wrapper {item: list}\n}\n\n\
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn chain() -> Int {\n    let xs = list_of(1, 2)\n    let a = xs\n    let b = a\n    \
         consume_list(b)\n    return size(xs)\n}\n\n\
         fn parity_probe() -> Int {\n    let h = Holder {tags: mut_list_of(1)}\n    \
         let w = wrap(h.tags)\n    add(h.tags, 9)\n    return size(w.item)\n}\n\n\
         fn kept_leak(h: Holder) -> Wrapper => h {\n    return wrap(h.tags)\n}\n\n\
         fn kept_remedy(h: Holder) -> Wrapper => h {\n    return wrap(copy(h.tags))\n}\n\n\
         fn kept_binding(v: Str) -> Str => v {\n    let w = v\n    return w\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Chain: `xs` was consumed at `a`'s binding (the whole chain is
    // move-mode); the use-site error names the binding.
    assert!(
        stderr
            .contains("`xs` cannot be used here: `a` was bound from it and later moves the value"),
        "stderr: {stderr}"
    );
    // The parity probe is rejected: `wrap(h.tags)` moved mutable data
    // out of `h`, so the later `add(h.tags, 9)` cannot observe an alias
    // on one backend and a clone on the other. Since L5 the diagnostic
    // names the *field* that left rather than the whole variable — `h`
    // itself stays usable, and reading `h.tags` back is what fails
    // [fate-partial-move].
    assert!(
        stderr.contains(
            "`h.tags` cannot be used here: `h.tags` was moved out of `h`, \
             and this reads the same data"
        ),
        "stderr: {stderr}"
    );
    // A kept parameter's mutable data cannot be moved out; `copy` is the
    // remedy (kept_remedy is clean).
    assert!(
        stderr.contains("cannot move mutable data out of `h`: it is a kept parameter"),
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
        "fn eat_two(a: List<Int>, b: List<Int>) -> None => !a, !b {\n}\n\n\
         fn consume_first(a: List<Int>, n: Int) -> None => !a {\n}\n\n\
         fn keep_first(a: List<Int>, n: Int) -> None => a {\n}\n\n\
         fn double_move() {\n    let xs = list_of(1, 2)\n    eat_two(xs, xs)\n}\n\n\
         fn move_then_read() {\n    let xs = list_of(1, 2)\n    consume_first(xs, size(xs))\n}\n\n\
         fn remedy() {\n    let xs = list_of(1, 2)\n    eat_two(copy(xs), xs)\n}\n\n\
         fn kept_then_read() {\n    let ys = list_of(3)\n    keep_first(ys, size(ys))\n}\n",
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
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn immutable_free() -> Int {\n    let base = 10\n    let text = \"hi\"\n    \
         let f = (n: Int) -> { return n + base + size(text) }\n    \
         let r = apply(f, 1)\n    return r + base + size(text)\n}\n\n\
         fn poisoned_after_mutation() -> Int {\n    let xs = mut_list_of(1, 2)\n    \
         let f = (n: Int) -> { return n + size(xs) }\n    let before = apply(f, 1)\n    \
         add(xs, 9)\n    return apply(f, 1)\n}\n\n\
         fn used_before_mutation() -> Int {\n    let xs = mut_list_of(1, 2)\n    \
         let f = (n: Int) -> { return n + size(xs) }\n    let r = apply(f, 1)\n    \
         add(xs, 9)\n    return r + size(xs)\n}\n\n\
         fn mutate_capture_consumes() -> Int {\n    let xs = mut_list_of(1, 2)\n    \
         let g = () -> { add(xs, 1) }\n    run(g)\n    return size(xs)\n}\n\n\
         fn mutate_capture_remedy() -> Int {\n    let xs = mut_list_of(1, 2)\n    \
         let snapshot = copy(xs)\n    let g = () -> { add(snapshot, 1) }\n    run(g)\n    \
         return size(xs)\n}\n\n\
         fn move_capture_rejected() {\n    let xs = list_of(1, 2)\n    \
         let h = () -> { consume_list(xs) }\n    run(h)\n}\n\n\
         fn move_capture_remedy() {\n    let xs = list_of(1, 2)\n    \
         let h = () -> { consume_list(copy(xs)) }\n    run(h)\n}\n\n\
         fn kept_mutates(xs: Mut List<Int>) -> None => xs: Mut {\n    \
         let g = () -> { add(xs, 1) }\n    run(g)\n}\n\n\
         fn infer_claims(xs: Mut List<Int>) {\n    let g = () -> { add(xs, 1) }\n    run(g)\n}\n\n\
         fn claim_reaches_caller() -> Int {\n    let xs = mut_list_of(1)\n    \
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
    // A capture-consuming lambda is legal since L7b but `once`-typed
    // [once-fn]: passing it where a plain fn is expected fails at the
    // boundary.
    assert!(
        stderr.contains("no matching overload for `run(once () -> "),
        "stderr: {stderr}"
    );
    // A written-kept parameter cannot be captured-and-mutated.
    assert!(
        stderr.contains("this lambda captures and mutates `xs`, which is a kept parameter"),
        "stderr: {stderr}"
    );
    // The capture claim propagates: the caller's argument is consumed.
    assert!(
        stderr.contains("`xs` cannot be used here: it was consumed (moved) by an earlier call"),
        "stderr: {stderr}"
    );
    // The positive matrix (immutable captures, pre-mutation use, `copy`
    // remedies) is clean: exactly the five violations.
    assert!(stderr.contains("5 errors"), "stderr: {stderr}");
}

// [linear-obligation] L6: values of a `: Linear<self>` type must be used —
// moved onward or `close`d — on every path. The negative matrix:
// scope-exit leak, consumed-on-some-paths-only, dropped expression
// result, overwriting a live value, returning while owing, `copy`
// refused, a lambda swallowing an obligation, and — since R4 part 2 —
// storing one in a struct field [linear-composite]. The positive matrix
// (pass onward, `close`, return with the caller obligated, kept-parameter
// borrow, derived alias) is clean.
#[test]
fn l6_linear_obligations() {
    let dir = src_dir("l6_linear");
    fs::write(
        dir.join("main.sv"),
        "linear struct FileHandle {\n    fd: Int\n}

fn close(x: FileHandle) -> None => !x { discard(x) }
\n\n\
         linear struct Box2 {\n    item: FileHandle\n}

fn close(x: Box2) -> None => !x { discard(x) }
\n\n\
         linear struct Conn {\n    tags: Mut List<Int>\n}

fn close(x: Conn) -> None => !x { discard(x) }
\n\n\
         fn open_file(path: Str) -> FileHandle => !path {\n    \
         return FileHandle {fd: size(path)}\n}\n\n\
         fn close_file(h: FileHandle) -> None => !h {\n    close(h)\n}\n\n\
         fn close_box(b: Box2) -> None => !b {\n    close(b)\n}\n\n\
         fn inspect(h: FileHandle) -> Int => h {\n    return h.fd\n}\n\n\
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
         fn swallow() {\n    let conn = Conn {tags: mut_list_of(1)}\n    \
         let g = () -> { add(conn.tags, 2) }\n    run(g)\n    close(conn)\n}\n\n\
         fn ok_pass() {\n    let h = open_file(\"data.txt\")\n    close_file(h)\n}\n\n\
         fn ok_discard() {\n    let h = open_file(\"data.txt\")\n    close(h)\n}\n\n\
         fn ok_return() -> FileHandle {\n    let h = open_file(\"data.txt\")\n    return h\n}\n\n\
         fn ok_kept_borrow() {\n    let h = open_file(\"data.txt\")\n    \
         let n = inspect(h)\n    close_file(h)\n}\n\n\
         fn ok_alias() {\n    let h = open_file(\"data.txt\")\n    let alias = h\n    \
         let n = inspect(alias)\n    close_file(h)\n}\n\n\
         fn composite_refused() {\n    let h = open_file(\"data.txt\")\n    \
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
        stderr.contains("`h` owns a linear value that is consumed on some paths but not others"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("this expression produces a linear value that is dropped immediately"),
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
        stderr.contains("this lambda captures and mutates `conn`, which holds a linear value"),
        "stderr: {stderr}"
    );
    // [linear-composite] [linear-group] `Box2` is a `linear struct` with a
    // concrete linear field — legal since 2026-09-12: the container carries
    // the obligation, `close_box` discharges it by forwarding, and the
    // whole shape draws no error.
    assert!(!stderr.contains("Box2.item"), "stderr: {stderr}");
    // Exactly the seven violations: the positive matrix is clean.
    assert!(stderr.contains("7 errors"), "stderr: {stderr}");
}

// [linear-generics] Generic instantiation with a linear type is refused
// unless the type parameter declares `<T canbe linear>`; variadic
// positions refuse linear values outright (they are untracked).
#[test]
fn l6_generics_refuse_linear_types() {
    let dir = src_dir("l6_generics");
    fs::write(
        dir.join("main.sv"),
        "linear struct FileHandle {\n    fd: Int\n}

fn close(x: FileHandle) -> None => !x { discard(x) }
\n\n\
         fn hold<T>(value: T) -> T {\n    return value\n}\n\n\
         fn generic_refused() {\n    let h = FileHandle {fd: 1}\n    \
         let kept = hold(h)\n    discard(kept)\n}\n\n\
         fn variadic_refused() {\n    let h = FileHandle {fd: 1}\n    \
         let xs = list_of(h)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "cannot instantiate generic parameter `T` of `hold` with linear type \
             `FileHandle`: `hold` does not declare `<T canbe linear>`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("a linear value cannot be passed in a variadic position"),
        "stderr: {stderr}"
    );
}

// [linear-generics] L7a: `<T canbe linear>` opts a generic fn into
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
        "linear struct FileHandle {\n    fd: Int\n}

fn close(x: FileHandle) -> None => !x { discard(x) }
\n\n\
         fn open_file(n: Int) -> FileHandle {\n    return FileHandle {fd: n}\n}\n\n\
         fn hold<T canbe linear>(value: T) -> T {\n    return value\n}\n\n\
         fn eat<T canbe linear>(value: T) -> None => !value {\n}\n\n\
         fn forward<T canbe linear>(value: T) -> T {\n    let kept = unopted(value)\n    \
         return kept\n}\n\n\
         fn unopted<T>(value: T) -> T {\n    return value\n}\n\n\
         fn bad_clause<T canbe Mut>(value: T) -> T {\n    return value\n}\n\n\
         fn variadic_refused() {\n    let h = open_file(1)\n    \
         let xs = mut_list_of(h)\n}\n\n\
         fn ok_hold() {\n    let h = hold(open_file(9))\n    close(h)\n}\n",
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
             type `T`: `unopted` does not declare `<T canbe linear>`"
        ),
        "stderr: {stderr}"
    );
    // Only `Linear` is supported in the clause.
    assert!(
        stderr.contains("only `linear` is supported in a type-parameter `with` clause"),
        "stderr: {stderr}"
    );
    // Variadic positions refuse linear values.
    assert!(
        stderr.contains("a linear value cannot be passed in a variadic position"),
        "stderr: {stderr}"
    );
    // [linear-container] The container *itself* owes now (LC-1, 2026-09-16):
    // `Mut List<FileHandle>` is a linear type, so the `xs` that was built —
    // however badly — has to be drained or moved onward, and the diagnostic
    // names the terminal rather than the element's discharger.
    assert!(
        stderr.contains("`xs` still owns a linear value when it goes out of scope")
            && stderr.contains("discharge it with `drain`"),
        "stderr: {stderr}"
    );
    // ok_hold is clean (its discharge is `close`, not `discard`
    // [linear-group]). Six errors total: the four above, the follow-on leak
    // in `variadic_refused` (the refused `h` is never discharged), and the
    // container's own obligation.
    //
    // The variadic refusal stays what it was: a variadic position is not
    // tracked, so a linear value may not travel through one — `add` is how an
    // obligation enters a list [linear-container].
    assert!(stderr.contains("6 errors"), "stderr: {stderr}");
}

// [once-fn] L7b: `once` on fn types means callable at most once,
// enforced by consumption — a second call, a call in a loop, and a
// call after passing the value on all error; a maybe-call is fine
// ("at most once"). Subtyping is inverted (plain fns fit `once`
// positions, never the reverse), a capture-consuming lambda is legal
// but `once`-typed, and `once` applies only to fn types.
#[test]
fn l7b_once_fns() {
    let dir = src_dir("l7b_once");
    fs::write(
        dir.join("main.sv"),
        "fn run_once(f: once () -> None) {\n    f()\n}\n\n\
         fn run_twice_bad(f: once () -> None) {\n    f()\n    f()\n}\n\n\
         fn run_in_loop_bad(f: once () -> None) {\n    for i in list_of(1, 2) {\n        f()\n    }\n}\n\n\
         fn run_maybe(f: once () -> None, flag: Bool) {\n    if flag {\n        f()\n    }\n}\n\n\
         fn run_plain(f: () -> None) {\n    f()\n    f()\n}\n\n\
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn once_where_plain_bad() {\n    let xs = list_of(1, 2)\n    \
         let g = () -> { consume_list(xs) }\n    run_plain(g)\n}\n\n\
         fn once_where_once_ok() {\n    let xs = list_of(1, 2)\n    \
         let g = () -> { consume_list(xs) }\n    run_once(g)\n}\n\n\
         fn plain_where_once_ok() {\n    let n = 5\n    \
         let g = () -> { let m = n + 1 }\n    run_once(g)\n}\n\n\
         fn escape_rule(f: once () -> None) {\n    run_once(f)\n    f()\n}\n\n\
         fn not_a_fn(x: once Int) {\n}\n",
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
                 (a `once` function is callable at most once)"
            )
            .count(),
        2,
        "stderr: {stderr}"
    );
    // A `once` lambda cannot go where a plain fn is expected.
    assert!(
        stderr.contains("no matching overload for `run_plain(once () -> "),
        "stderr: {stderr}"
    );
    // Passing a `once` value on consumes it (escape rule).
    assert!(
        stderr.contains("`f` cannot be used here: it was consumed (moved) by an earlier call"),
        "stderr: {stderr}"
    );
    // [once-fn] D6 (user decision 2026-09-12): `once` is valid on any
    // type now, so `not_a_fn(x: once Int)` draws no error.
    assert!(
        !stderr.contains("`once` applies to function types"),
        "stderr: {stderr}"
    );
    // run_maybe, once_where_once_ok, plain_where_once_ok, not_a_fn are clean.
    assert!(stderr.contains("4 errors"), "stderr: {stderr}");
}

// [readonly-return] L7c: `-> proj[from: p] T?` marks a derived
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
         fn take(p: Person) -> None => !p {\n}\n\n\
         fn find_adult(persons: List<Person>) -> proj[from: persons] Person? => persons {\n    \
         for person in persons {\n        if person.age >= 18 {\n            return person\n        }\n    }\n    \
         return None\n}\n\n\
         fn forwarded(persons: List<Person>, tag: Str) -> proj[from: persons] Person? => persons, tag {\n    \
         return first(persons)\n}\n\n\
         fn bad_independent(persons: List<Person>) -> proj[from: persons] Person? => persons {\n    \
         return Person {name: \"made up\", age: 1}\n}\n\n\
         fn bad_moved(persons: List<Person>) -> proj[from: persons] Person? => !persons {\n    \
         return None\n}\n\n\
         fn bad_param(persons: List<Person>) -> proj[from: nobody] Person? => persons {\n    \
         return None\n}\n\n\
         fn poison_after_mutation() -> Int {\n    \
         let people = mut_list_of(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    add(people, Person {name: \"Grace\", age: 45})\n    \
         if head is Person h {\n        return h.age\n    }\n    return 0\n}\n\n\
         fn escape_borrowed() {\n    let people = list_of(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        take(h)\n    }\n}\n\n\
         fn ok_use() -> Int {\n    let people = list_of(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        return h.age\n    }\n    \
         return 0\n}\n\n\
         fn ok_copy_escape() {\n    let people = list_of(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        take(copy(h))\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "this function returns `proj[from: persons]`, so every returned \
             value must be derived from `persons`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`proj[from: persons]` requires `persons` to be kept"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`proj[from: nobody]` names no parameter"),
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
    // (ok_copy_escape is clean). [proj-type] names the projection's type.
    assert!(
        stderr.contains("`h` is a projection (`proj Person`), and `take` consumes `p`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("5 errors"), "stderr: {stderr}");
}

// [fn-contract] L7d: fn types carry contracts — named parameters plus a
// standard deduction clause (`=>[f] !v` consumes, unannotated keeps
// everything). Calls through fn
// values apply the contract (consumption, double-use, interprocedural
// propagation); lambdas checked against a keeping contract may not
// consume their kept parameters; a consuming fn cannot be passed where
// a keeping one is expected (checked via the named-fn contract).
#[test]
fn l7d_fn_type_contracts() {
    let dir = src_dir("l7d_contracts");
    fs::write(
        dir.join("main.sv"),
        "fn apply_consuming(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v {\n    \
         return f(data)\n}\n\n\
         fn apply_keeping(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] v => data {\n    \
         return f(data) + f(data)\n}\n\n\
         fn double_use_bad(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v {\n    \
         let a = f(data)\n    return f(data)\n}\n\n\
         fn eat(v: List<Int>) -> Int => !v {\n    return 0\n}\n\n\
         fn consuming_where_keeping_bad() -> Int {\n    let xs = list_of(1, 2)\n    \
         return apply_keeping(eat, xs)\n}\n\n\
         fn kept_param_consumed_bad() -> Int {\n    let xs = list_of(1, 2)\n    \
         return apply_keeping((v: List<Int>) -> {\n        let w = eat(v)\n        \
         return w\n    }, xs)\n}\n\n\
         fn caller_loses() -> Int {\n    let xs = list_of(1, 2)\n    \
         let n = apply_consuming((v: List<Int>) -> { return size(v) }, xs)\n    \
         return n + size(xs)\n}\n\n\
         fn caller_keeps() -> Int {\n    let xs = list_of(1, 2)\n    \
         let n = apply_keeping((v: List<Int>) -> { return size(v) }, xs)\n    \
         return n + size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Double use through a consuming contract.
    assert!(
        stderr.contains("`data` cannot be used here: it was consumed (moved) by an earlier call"),
        "stderr: {stderr}"
    );
    // A consuming named fn cannot fit a keeping contract.
    assert!(
        stderr.contains("no matching overload for `apply_keeping("),
        "stderr: {stderr}"
    );
    // A kept lambda parameter cannot be consumed.
    assert!(
        stderr.contains("cannot consume `v`: this lambda's contract keeps it"),
        "stderr: {stderr}"
    );
    // The consuming contract propagates: the caller's argument dies.
    assert!(
        stderr.contains("`xs` cannot be used here: it was consumed (moved) by an earlier call"),
        "stderr: {stderr}"
    );
    // caller_keeps is clean: exactly the four violations.
    assert!(stderr.contains("4 errors"), "stderr: {stderr}");
}

// ===== D1: exhaustive and delta qualifier deductions =====
// [deduce-syntax] The fix for the qualifier-preservation unsoundness: a
// plain list is *exhaustive* (only those qualifiers survive, including
// ones the callee never declared), `-Q` is a delta, and a body that
// mutates a parameter may use neither keep-all nor a delta.
#[test]
fn exhaustive_deductions_drop_undeclared_qualifiers() {
    let dir = src_dir("dedu_exhaustive");
    let prelude = "qualifier NonEmpty of List<Int> {\n    \
                   fn qualifies(list: List<Int>) -> Bool {\n        \
                   return list.size() > 0\n    }\n}\n\n";

    // The reproduction of the unsoundness: `clear` declares only `Mut`,
    // so its exhaustive list drops the caller's `NonEmpty`.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}\
             fn clear(list: Mut List<Int>) [] -> None => list: Mut {{\n}}\n\n\
             fn describe(list: NonEmpty Mut List<Int>) -> Int {{\n    \
             return list.size()\n}}\n\n\
             fn main() [use] {{\n    use StdOutConsole\n    \
             let xs = mut_list_of(1, 2)\n    \
             if xs is NonEmpty {{\n        clear(xs)\n        \
             println(\"${{describe(xs)}}\")\n    }}\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `describe(Mut List<Int>)`"),
        "stderr: {stderr}"
    );

    // The delta form keeps everything it does not name — legal on a
    // parameter nothing can invalidate (no `Mut`): here `Checked` survives
    // even though the callee never declares it.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}\
             qualifier Checked of List<Int> {{\n    \
             fn qualifies(list: List<Int>) -> Bool {{\n        \
             return true\n    }}\n}}\n\n\
             fn forget_nonempty(list: NonEmpty List<Int>) [] -> None \
             => list: -NonEmpty {{\n}}\n\n\
             fn needs_checked(list: Checked List<Int>) -> Int {{\n    \
             return list.size()\n}}\n\n\
             fn main() [use] {{\n    use StdOutConsole\n    \
             let xs = list_of(1, 2)\n    \
             if xs is NonEmpty Checked {{\n        forget_nonempty(xs)\n        \
             println(\"${{needs_checked(xs)}}\")\n    }}\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "the delta form must preserve `Checked`: {stderr}"
    );

    // ...including on a fn with a `Mut` parameter whose body does not
    // mutate it: a written delta is the author's contract, trusted rather
    // than second-guessed (a body that *does* mutate cannot keep
    // everything — see the `grow` case below).
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}\
             fn touch(list: Mut List<Int>) [] -> None => list: -NonEmpty {{\n}}\n\n\
             fn describe(list: Checked Mut List<Int>) -> Int {{\n    \
             return list.size()\n}}\n\n\
             qualifier Checked of List<Int> {{\n    \
             fn qualifies(list: List<Int>) -> Bool {{\n        \
             return true\n    }}\n}}\n\n\
             fn main() [use] {{\n    use StdOutConsole\n    \
             let xs = mut_list_of(1)\n    \
             if xs is Checked {{\n        touch(xs)\n        \
             println(\"${{describe(xs)}}\")\n    }}\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "a declared delta is trusted: {stderr}"
    );

    // A mutating body may not keep everything.
    fs::write(
        dir.join("main.sv"),
        "fn grow(list: Mut List<Int>) -> None => list {\n    list.add(1)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr
            .contains("is mutated by this function, so the deduction cannot keep every qualifier"),
        "stderr: {stderr}"
    );
}

// [deduce-syntax] Syntax rules for the new entry forms.
#[test]
fn deduction_entry_forms_are_validated() {
    let dir = src_dir("dedu_forms");

    // Mixing exhaustive and delta in one entry.
    fs::write(
        dir.join("main.sv"),
        "qualifier A of Int\n\nfn f(x: A Int) -> None => x: A -A {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("either exhaustive (plain qualifier names) or a delta"),
        "stderr: {stderr}"
    );

    // `+Qual` is not supported yet (D2).
    fs::write(
        dir.join("main.sv"),
        "qualifier A of Int\n\nfn f(x: Int) -> None => x: +A {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("adding qualifiers in a deduction (`+Qual`) is not supported yet"),
        "stderr: {stderr}"
    );

    // A type other than `Nothing` is not supported yet (D1b).
    fs::write(
        dir.join("main.sv"),
        "fn f(x: List<Int>) -> None => x: List<Int> {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("type narrowing in deductions is not supported yet"),
        "stderr: {stderr}"
    );
}

// [decl-explicit] Bodyless declarations carry no inference: an effect
// member must state its deductions and return type (effects are forbidden
// there [effect-member-no-effects]). The declaration is then a real
// contract — a member that takes ownership consumes its argument at the
// call site. (The other bodyless form, `intrinsic`, is the compiler's and
// only std may write it [intrinsic-std-only], so it never appears in a
// user source like these.)
#[test]
fn bodyless_declarations_must_be_explicit() {
    let dir = src_dir("decl_explicit");

    // An effect member missing the two that apply to it (effects are
    // forbidden there [effect-member-no-effects], so they are not asked
    // for).
    fs::write(
        dir.join("main.sv"),
        "effect Sink {\n    fn eat(x: Str)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [deduce-syntax] The clause is required per parameter (Copy scalars
    // exempt), not as a whole.
    assert!(
        stderr.contains("effect member `eat` must declare its return type")
            && stderr.contains(
                "an effect member has no body to infer from, so its deduction clause must say \
                 what happens to `x`"
            ),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("effect member `eat` must declare its effect list"),
        "members may not declare effects at all: {stderr}"
    );

    // The member's declared deductions are enforced: `[]` moves `h`, so
    // the later read is an error (this was silently accepted before).
    fs::write(
        dir.join("main.sv"),
        "struct Handle {\n    id: Int\n}\n\n\
         effect Sink {\n    fn eat(h: Handle) -> None => !h\n}\n\n\
         handler Bin of Sink {\n    fn eat(h: Handle) -> None => !h {\n        \
         discard(h)\n    }\n}\n\n\
         fn main() [use] -> None {\n    use StdOutConsole\n    use Bin\n    \
         let h = Handle {id: 1}\n    eat(h)\n    println(\"${h.id}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`h` cannot be used here: it was consumed (moved)"),
        "stderr: {stderr}"
    );
}

// [decl-explicit] std's `add` takes ownership of the element: the list
// owns it now. Before bodyless declarations became explicit this was
// inferred as *kept*, so the program below compiled on Kotlin and was
// rejected by rustc — a backend divergence the checker now catches.
#[test]
fn std_add_consumes_its_element() {
    let dir = src_dir("decl_add_moves");
    fs::write(
        dir.join("main.sv"),
        "struct Handle {\n    id: Int\n}\n\n\
         fn main() [use] -> None {\n    use StdOutConsole\n    \
         let xs = mut_list_of(Handle {id: 1})\n    \
         let h = Handle {id: 2}\n    add(xs, h)\n    \
         println(\"${h.id}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`h` cannot be used here: it was consumed (moved)"),
        "stderr: {stderr}"
    );
}

// [intrinsic-std-only] `intrinsic` marks a declaration the *compiler*
// implements, so only the standard library may write it. A user source
// analyzed here is not std, so an `intrinsic` declaration in it is an
// error naming the one interop path customer code does have — a
// `platform effect`. (The declaration is fully explicit so this is the
// only diagnostic, not a decl-explicit complaint.)
#[test]
fn intrinsic_in_a_user_file_is_an_error() {
    let dir = src_dir("intrinsic_user");
    fs::write(
        dir.join("main.sv"),
        "intrinsic fn secret<T>(value: T) [] -> T => value\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`intrinsic fn secret` is the compiler's to declare, not yours"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("`platform effect`"), "stderr: {stderr}");
}

// --- Qualifier refinements [qual-refn] ---

/// The refined program end to end through the binary, over the *real* std:
/// `add`'s exhaustive `[list: Mut]` drops `NonEmpty` [deduce-syntax], and
/// `NonEmpty`'s own refinement puts it back, so the `NonEmpty` overload still
/// resolves after the mutation.
#[test]
fn a_refinement_recovers_a_qualifier_stripped_by_a_mutating_call() {
    let dir = src_dir("refn");
    let source = "\
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

fn count<T canbe linear>(list: NonEmpty List<T>) -> Int => list {
    return size(list)
}

fn main() [use] {
    let xs: Mut List<Int> = mut_list_of()
    add(xs, 1)
    let _n = count(xs)
}
";
    fs::write(dir.join("main.sv"), source).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");

    // The control: without the refinement, std's `add` strips the claim.
    let dir = src_dir("refn_without");
    fs::write(
        dir.join("main.sv"),
        source
            .replace(
                "    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty\n",
                "",
            )
            .replace("    // Adding an element makes the list non-empty.\n", ""),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("no matching overload for `count(Mut List<Int>)`"),
        "stderr: {stderr}"
    );
}

/// [qual-refn-conflict] Two qualifiers whose refinements cannot both hold
/// leave the function unrefined — deliberately *not* an error, so the
/// program still compiles (exit 0), but a warning, so the silence is
/// discoverable. Reported once per callee and parameter, not per call.
#[test]
fn a_refinement_conflict_warns_without_failing() {
    let dir = src_dir("refn_conflict");
    let source = "\
qualifier Q1<T> of List<T> with NonEmpty {
    fn qualifies(list: List<T>) -> Bool { return list.size() > 0 }
    refn add(list: Mut List<T>, elem: T) => list: +Q1
}

qualifier Q2<T> of List<T> with NonEmpty {
    fn qualifies(list: List<T>) -> Bool { return list.size() > 0 }
    refn add(list: Mut List<T>, elem: T) => list: +Q2
}

fn main() [use] {
    let xs: Mut List<Int> = mut_list_of()
    add(xs, 1)
    add(xs, 2)
}
";
    fs::write(dir.join("main.sv"), source).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a conflict is not an error: {stderr}");
    assert!(
        stderr.contains("warning: the refinements of `Q1` and `Q2` disagree about `list`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("0 errors, 1 warning"), "stderr: {stderr}");

    // …and the warning is a warning in the JSON too [diag-structured].
    let out = salvo(&[
        "analyze",
        "--src",
        dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(
        stdout.contains("\"severity\": \"warning\""),
        "stdout: {stdout}"
    );
}

// [proj-type] A union arm that borrows (`Emitted (proj Str) | Finished`)
// passes whole into a parameter only when the parameter's type *writes*
// the projection; an owned parameter refuses it, naming the type and the
// two remedies. The phase-2b cut's negative half, closed 2026-09-12.
#[test]
fn an_owned_parameter_refuses_a_borrowed_union_arm() {
    let dir = src_dir("proj_arm_param");
    fs::write(
        dir.join("main.sv"),
        "fn show(step: Emitted Str | Finished) [Console] -> None {\n    \
         when step {\n        is Emitted { println(step) }\n        is Finished { println(\"done\") }\n    }\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         let words = list_of(\"ann\", \"bo\")\n    let p = iter(words)\n    show(next(p))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "this argument holds a borrowed value (`proj Emitted Str | Finished`) \
             where `show` expects an owned one for `step`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("Write the parameter's type with the `proj`"),
        "stderr: {stderr}"
    );
}

// [proj-type] What a pass combinator's result *is*: `map`/`keep_all` over a
// container's pass yield a `Mut List<proj Str>` — a view, fate-linked to
// the container — so consuming either the view or the container is
// refused; an instantiation over a *generator* pass has no container and
// stays free. And `copy` un-projects exactly one level: `copy(e)` on a
// `proj Mut Str` is a `Mut Str` (observed via the mismatch against `Int`).
// The phase-2b regression set, closed 2026-09-12.
#[test]
fn a_combinator_result_is_a_view_of_its_container() {
    let dir = src_dir("proj_combinator_view");
    fs::write(
        dir.join("main.sv"),
        "fn eat(xs: List<Str>) -> None => !xs {}\n\n\
         fn keep(w: proj Str) -> proj[from: w] Str => w {\n    return w\n}\n\n\
         struct Chars {\n    n: Int\n}\n\n\
         iter fn next(c: Chars) -> Emitted Str | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    \
         if at >= c.n {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(\"x\")\n}\n\n\
         fn keep_all<It, T>(it: Mut It, ?Yield<It, T>) -> Mut List<T> => it: Mut {\n    \
         let out = mut_list_of<T>()\n    for x in it {\n        add(out, x)\n    }\n    return out\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         let words = list_of(\"ann\", \"bo\")\n    \
         let p = iter(words)\n    \
         let kept = map(p, keep)\n    \
         eat(kept)\n    \
         let more = keep_all(iter(words))\n    \
         eat(words)\n    \
         println(\"${size(more)}\")\n    \
         let free = keep_all(iter(Chars {n: 2}))\n    \
         println(\"${size(free)}\")\n    \
         let parts = mut_list_of(mut_str(\"a\"))\n    \
         let e = get(parts, 0)!\n    \
         let n: Int = copy(e)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // The combinator's type, named whole by the refusal to consume it.
    assert!(
        stderr.contains(
            "`kept` holds a borrowed value (`Mut List<proj Str>`) where `eat` \
             expects an owned one for `xs`"
        ),
        "stderr: {stderr}"
    );
    // The view is fate-linked *through* the temporary pass to the container:
    // consuming the container poisons it.
    assert!(
        stderr.contains(
            "`more` cannot be used here: it was bound from `words` and shares \
             its fate, and `words` was moved after the binding"
        ),
        "stderr: {stderr}"
    );
    // `copy(e)` on a `proj Mut Str` is a `Mut Str`: one level un-projected,
    // the `Mut` kept.
    assert!(
        stderr.contains("expected `Int`, found `Mut Str`"),
        "stderr: {stderr}"
    );
    // The generator instantiation is unlinked (`free` draws no error), so
    // exactly the three violations above are reported.
    assert!(stderr.contains("3 errors"), "stderr: {stderr}");
}

// [lambda-view] A capturing lambda is a *view*: it holds borrows of its
// non-Copy read captures, so binding it links it to the captured roots,
// a call result linked to the lambda reaches them transitively, and
// consuming a capture while the closure lives poisons it. A capture-free
// lambda holds nothing — `map(p, w -> w)` needs no ceremony. (User
// decision 2026-09-12; closed the hole where naming the lambda evaded
// the discipline.)
#[test]
fn a_capturing_lambda_is_a_view_of_its_captures() {
    let dir = src_dir("lambda_view");
    fs::write(
        dir.join("main.sv"),
        "fn eat(xs: List<Str>) -> None => !xs {}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         // 1. capture-free: the one-liner binds with no links\n    \
         let words = list_of(\"ann\", \"bo\")\n    \
         let p = iter(words)\n    \
         let kept = map(p, w -> w)\n    \
         println(\"${size(kept)}\")\n    \
         // 2. a capture-rooted projection: legal, linked to the capture\n    \
         let all = list_of(\"a\", \"b\", \"c\", \"d\")\n    \
         let indices = list_of(1, 3)\n    \
         let picked = map(iter(indices), i -> get(all, i)!)\n    \
         println(\"${size(picked)}\")\n    \
         // 3. the named-lambda form is linked the same way: consuming\n    \
         // the capture poisons the result derived through the closure\n    \
         let all2 = list_of(\"a\", \"b\")\n    \
         let f = i -> get(all2, i)!\n    \
         let picked2 = map(iter(indices), f)\n    \
         eat(all2)\n    \
         println(\"${size(picked2)}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`picked2` cannot be used here: it was bound from `all2` and shares \
             its fate, and `all2` was moved after the binding"
        ),
        "stderr: {stderr}"
    );
    // Scenarios 1 and 2 are clean: exactly the one violation.
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}

// [proj-infer] A written opaque projection entry (`=> proj[from: it]`)
// names the lends exactly and takes precedence over the instantiation
// fallback that links a `proj`-holding result to every kept argument —
// even where the *written* return type shows no projection (`Mut List<T>`
// with `T = proj Str`). The link still flows through the named source.
// (Closed 2026-09-12; before, the entry was consulted only behind a
// written-return-type gate the instantiation case never passed.)
#[test]
fn a_written_proj_entry_narrows_the_instantiation_link() {
    let dir = src_dir("proj_written_entry");
    fs::write(
        dir.join("main.sv"),
        "fn eat(xs: List<Str>) -> None => !xs {}\n\n\
         fn keep_all<It, T>(it: Mut It, labels: List<Str>, ?Yield<It, T>) \
         -> Mut List<T> => it: Mut, proj[from: it], labels {\n    \
         let out = mut_list_of<T>()\n    for x in it {\n        add(out, x)\n    }\n    return out\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         let words = list_of(\"ann\", \"bo\")\n    \
         let tags = list_of(\"x\")\n    \
         let kept = keep_all(iter(words), tags)\n    \
         eat(tags)\n    \
         println(\"${size(kept)}\")\n    \
         let words2 = list_of(\"ann\", \"bo\")\n    \
         let tags2 = list_of(\"x\")\n    \
         let kept2 = keep_all(iter(words2), tags2)\n    \
         eat(words2)\n    \
         println(\"${size(kept2)}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Consuming the co-argument the entry does *not* name is free…
    assert!(!stderr.contains("`kept` cannot"), "stderr: {stderr}");
    // …while the named source still links through the temporary pass.
    assert!(
        stderr.contains(
            "`kept2` cannot be used here: it was bound from `words2` and shares \
             its fate, and `words2` was moved after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}
