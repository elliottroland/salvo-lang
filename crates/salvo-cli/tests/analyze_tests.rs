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
        // [kt-std-library] The compiled std is cached in the test target.
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
        .args(args)
        .output()
        .expect("failed to run salvo")
}

const CLEAN: &str = "fn main() [use] {\n    use StdConsole()\n    println(\"hello\")\n}\n";

#[test]
fn clean_program_exits_zero() {
    let dir = src_dir("clean");
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

/// [diag-id] A diagnostic's id shows only under `-v`: in the text
/// (`error[id]:`) and as the JSON's `"id"`.
#[test]
fn ids_are_shown_only_under_verbose() {
    let dir = src_dir("ids");
    fs::write(dir.join("main.sv"), "fn main() [] -> None {\n    let x = 1!\n}\n").unwrap();
    let src = dir.to_str().unwrap();
    let id = "assert-op:postfix-bang";
    let plain = String::from_utf8_lossy(&salvo(&["analyze", "--src", src]).stderr).to_string();
    assert!(plain.contains("error: a postfix `!`") && !plain.contains(id), "{plain}");
    let verbose = String::from_utf8_lossy(&salvo(&["-v", "analyze", "--src", src]).stderr).to_string();
    assert!(verbose.contains(&format!("error[{id}]: a postfix `!`")), "{verbose}");
    let json = String::from_utf8_lossy(&salvo(&["analyze", "-v", "--format", "json", "--src", src]).stdout).to_string();
    assert!(json.contains(&format!("\"id\": \"{id}\"")), "{json}");
    let json = String::from_utf8_lossy(&salvo(&["analyze", "--format", "json", "--src", src]).stdout).to_string();
    assert!(!json.contains("\"id\""), "{json}");
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

// [diag-std-hidden] A program whose only errors are located in the embedded
// std. A project cannot reach std any more [std-root], so the construction is
// a `std = true` dependency adding a module under `std.core`: implicitly
// visible to std, it makes std's own `by auto` sites name two modules.
// Neither `analyze` nor a build lists a std position; each says so in one
// line and fails.
#[test]
fn errors_in_the_embedded_std_are_one_line() {
    let dir = src_dir("std_hidden");
    let fork = dir.join("salvo_modules/fork");
    fs::create_dir_all(fork.join("salvo/std/core/extra")).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nmodules = \"salvo_modules\"\n\n\
         [dependencies]\nfork = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        fork.join("salvo.toml"),
        "[project]\nname = \"fork\"\nversion = \"0.1.0\"\nstd = true\n\n[build]\nsrc = \"salvo\"\n",
    )
    .unwrap();
    fs::write(fork.join("salvo/std/core/extra/auto.sv"), "export fn helper() [] -> Int {\n    return 1\n}\n").unwrap();
    fs::write(dir.join("main.sv"), "fn main() [] {\n    let _x = helper()\n}\n").unwrap();
    let line = "the standard library does not check with this program";
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains(line), "stderr: {stderr}");
    assert!(!stderr.contains("--> std/"), "a std position was shown: {stderr}");
    assert!(stderr.contains("1 error, 0 warnings"), "stderr: {stderr}");
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap(), "--format", "json"]);
    assert!(!out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[]");
    assert!(String::from_utf8_lossy(&out.stderr).contains(line));
    let target = dir.with_file_name("analyze_std_hidden_out");
    let _ = fs::remove_dir_all(&target);
    let out = salvo(&["compile", "--backend", "rust", "--src", dir.to_str().unwrap(), "--target", target.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains(line), "stderr: {stderr}");
    assert!(!stderr.contains("--> std/"), "a std position was shown: {stderr}");
}

// [std-root] `std` is a reserved root: a project's module under it is refused
// where it is written, and is not implicitly visible — so std's own expansion
// is untouched, and the program's one mistake is two lines (the misplaced
// module, and the name it would have supplied).
#[test]
fn a_project_module_under_std_is_refused() {
    let dir = src_dir("std_root");
    fs::create_dir_all(dir.join("std/core/extra")).unwrap();
    fs::write(dir.join("std/core/extra/auto.sv"), "export fn helper() [] -> Int {\n    return 1\n}\n").unwrap();
    fs::write(dir.join("main.sv"), "fn main() [] {\n    let _x = helper()\n}\n").unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("module `std.core.extra.auto` is under `std`, which is reserved for the standard library")
            && stderr.contains("move this file out of its `std/` directory --> std/core/extra/auto.sv:1:1"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("[diag-std-hidden]"), "std itself checks: {stderr}");
    assert!(stderr.contains("2 errors, 0 warnings"), "stderr: {stderr}");
}

// [std-root] The bare spelling of a std module is refused with the corrected
// path named, and resolves nothing; a project's own module of that name is
// its own business.
#[test]
fn a_bare_std_import_names_the_corrected_path() {
    let dir = src_dir("std_bare");
    fs::write(
        dir.join("main.sv"),
        "import time\nimport core.list\nimport fs.mem.MemFs\n\nfn main() [] {\n    let _d = seconds(1)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    for corrected in ["import std.time`", "import std.core.list`", "import std.fs.mem.MemFs`"] {
        assert!(
            stderr.contains(&format!("the standard library's modules are under `std`: write `{corrected} --> ")),
            "{corrected}: {stderr}"
        );
    }
    assert!(stderr.contains("no function named `seconds` is in scope"), "not a fallback: {stderr}");

    fs::write(dir.join("time.sv"), "export fn seconds(n: Int) [] -> Int {\n    return n\n}\n").unwrap();
    fs::write(dir.join("main.sv"), "import time\n\nfn main() [] {\n    let _d = seconds(1)\n}\n").unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a project module `time` is the project's: {stderr}");
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
        stderr.contains("help: add `import std.random.DefaultRandom`"),
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
        stdout.contains("\"imports\": [\"std.random.DefaultRandom\"]"),
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
         iter fn iter(n: Nums) -> Emitted Int | Finished {\n    \
         state NumsIter {\n        at: Int = 0\n    }\n    if at >= n.limit {\n        \
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

// [deduce-consume] A written `!p` consumes the identifier argument and later
// uses are errors — the IR flow pass's, naming the move
// [flow-use-after-move]; reassignment revives it; kept
// parameters are unaffected. Consumption on an always-exiting branch does not
// leak past the branch. A Copy scalar is the exception [copy-scalar-free]:
// moving one is a copy, so the same program over `Int`s compiles.
#[test]
fn use_after_consume_is_an_error() {
    let dir = src_dir("consume");
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Str, b: Str) -> Str => !a, !b {\n    return \"${a}${b}\"\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    let a = \"1\"\n    let b = \"2\"\n    \
         let c = a.add(b)\n    println(\"${a}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`a` cannot be used here: it was consumed (moved) at line 9; reassign it before use, or `copy` it where it is moved"),
        "stderr: {stderr}"
    );
    // [copy-scalar-free] The same over `Int`s: moving one is a copy, and the
    // written `!a, !b` only warns that it changes nothing.
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Int, b: Int) -> Int => !a, !b {\n    return a + b\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    let a = 1\n    let b = 2\n    \
         let c = a.add(b)\n    println(\"${a}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("moving an `Int` is a copy, so `!a` changes nothing"), "stderr: {stderr}");

    // Reassignment revives the variable; kept parameters (`[a]`) are
    // never consumed; consumption inside an always-exiting `if` branch
    // never reaches the code after the `if`.
    fs::write(
        dir.join("main.sv"),
        "fn add(a: Int, b: Int) -> Int {\n    return a + b\n}\n\n\
         fn double(a: Int) -> Int => a {\n    return a + a\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    let a = 1\n    let b = 2\n    \
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
         fn main() [use] {\n    use StdConsole\n    let s = \"x\"\n    let flag = true\n    \
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

// [flow-use-after-move] The IR flow pass is a gate: `salvo analyze` (and so
// the LSP), `salvo compile` on both backends, and `salvo ir` (inline) all
// report a use after a move, naming the move's line; a test file's bodies
// are checked too.
#[test]
fn every_gate_reports_a_use_after_move() {
    let dir = src_dir("flow_gate");
    fs::write(
        dir.join("main.sv"),
        "fn eat(s: Str) -> None => !s {\n}\n\n\
         fn twice() {\n    let s = \"a${1}\"\n    eat(s)\n    eat(s)\n}\n",
    )
    .unwrap();
    let message = "`s` cannot be used here: it was consumed (moved) at line 6; reassign it before use, or `copy` it where it is moved";
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains(&format!("{message} --> main.sv:7:9")), "stderr: {stderr}");
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
    for backend in ["rust", "kotlin"] {
        let target = dir.join(format!("out_{backend}"));
        let out = salvo(&["compile", "--backend", backend, "--src", dir.to_str().unwrap(), "--target", target.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend} compiled a use after a move: {stderr}");
        assert!(stderr.contains(message), "{backend}: {stderr}");
    }
    // `salvo ir` dumps the program and shows the use where it happens.
    let out = salvo(&["ir", "--src", dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&format!("!!! main:7:9: {message}")), "stdout: {stdout}");

    // A test file's bodies are checked by the flow pass as well.
    fs::write(dir.join("main.sv"), "fn eat(s: Str) -> None => !s {\n}\n").unwrap();
    fs::write(
        dir.join("main.test.sv"),
        "test \"twice\" {\n    let s = \"a${1}\"\n    eat(s)\n    eat(s)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("`s` cannot be used here: it was consumed (moved) at line 3; reassign it before use"),
        "stderr: {stderr}"
    );
}

// [ir-analyze] [cli-analyze] Ownership errors are per declaration (R8 piece
// 9): a type error in one fn does not hide another fn's use after a move, on
// `salvo analyze` or in a build's error list (which still emits nothing). A
// fn with a type error of its own reports only that; a fn calling one whose
// signature broke is skipped; a stamped fn's error (reported at the struct's
// field) breaks the stamp only, not the struct; a file that does not parse
// is skipped whole.
#[test]
fn ownership_errors_are_reported_beside_type_errors_elsewhere() {
    let dir = src_dir("per_decl_gate");
    fs::write(
        dir.join("main.sv"),
        "fn eat(s: Str) -> None => !s {\n}\n\n\
         fn typo() -> Int {\n    return \"x\"\n}\n\n\
         fn twice() {\n    let s = \"a${1}\"\n    eat(s)\n    eat(s)\n}\n\n\
         fn both() -> Int {\n    let s = \"a${1}\"\n    eat(s)\n    eat(s)\n    return \"y\"\n}\n\n\
         fn odd(s: Str) -> Nope {\n    return s\n}\n\n\
         fn caller() {\n    let s = \"a${1}\"\n    eat(s)\n    eat(s)\n    let _ = odd(\"q\")\n}\n\n\
         struct Sample : Hashed<self> by auto {\n    at: Double\n}\n\n\
         fn gone(p: Sample) -> None => !p {\n}\n\n\
         fn sampled() {\n    let p = Sample { at: 1.0 }\n    gone(p)\n    gone(p)\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("broken.sv"),
        "fn eat2(s: Str) -> None => !s {\n}\n\n\
         fn thrice() {\n    let s = \"a${1}\"\n    eat2(s)\n    eat2(s)\n    let = 3\n}\n",
    )
    .unwrap();
    let moved = |line: u32| format!("cannot be used here: it was consumed (moved) at line {line}");
    let reports = |stderr: &str, what: &str, at: &str| stderr.lines().any(|l| l.contains(what) && l.ends_with(at));
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    // The type errors, each once.
    assert!(stderr.contains("expected return type `Int`, found `Str` --> main.sv:5:12"), "stderr: {stderr}");
    assert!(stderr.contains("expected return type `Int`, found `Str` --> main.sv:18:12"), "stderr: {stderr}");
    assert!(stderr.contains("unknown type `Nope` --> main.sv:21:19"), "stderr: {stderr}");
    assert!(stderr.contains("expected return type `Nope`, found `Str` --> main.sv:22:12"), "stderr: {stderr}");
    assert!(stderr.contains("`s` is kept by default, but the body moves it"), "stderr: {stderr}");
    assert!(stderr.contains("in `hash` from `auto` for `Sample.at: Double`"), "stderr: {stderr}");
    assert!(stderr.contains("expected identifier, found `=` --> broken.sv:8:9"), "stderr: {stderr}");
    // `twice` is clean: its use after a move is reported beside them.
    assert!(reports(&stderr, &format!("`s` {}", moved(10)), "--> main.sv:11:9"), "stderr: {stderr}");
    // So is `sampled`: the stamp's error broke the stamped `hash`, not
    // `Sample`.
    assert!(reports(&stderr, &format!("`p` {}", moved(41)), "--> main.sv:42:10"), "stderr: {stderr}");
    // `both` has a type error of its own, `caller` calls `odd`, whose
    // signature broke, and `broken.sv` does not parse: none of them reports
    // its use after a move.
    assert!(!stderr.contains("main.sv:17:9"), "stderr: {stderr}");
    assert!(!stderr.contains("main.sv:28:9"), "stderr: {stderr}");
    assert!(!stderr.contains("broken.sv:7"), "stderr: {stderr}");
    let errors = stderr.matches("error: ").count();
    assert_eq!(errors, 9, "stderr: {stderr}");
    assert!(stderr.contains("9 errors"), "stderr: {stderr}");

    // A build lists both kinds, and still emits nothing.
    fs::remove_file(dir.join("broken.sv")).unwrap();
    for backend in ["rust", "kotlin"] {
        let target = dir.join(format!("out_{backend}"));
        let out = salvo(&["compile", "--backend", backend, "--src", dir.to_str().unwrap(), "--target", target.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend}: {stderr}");
        assert!(stderr.contains("expected return type `Int`, found `Str`"), "{backend}: {stderr}");
        assert!(stderr.contains(&format!("`s` {}", moved(10))), "{backend}: {stderr}");
        assert!(stderr.contains(&format!("`p` {}", moved(41))), "{backend}: {stderr}");
        assert!(!stderr.contains(&moved(16)), "{backend}: {stderr}");
        assert!(!stderr.contains(&moved(27)), "{backend}: {stderr}");
        assert!(!target.exists() || fs::read_dir(&target).unwrap().next().is_none(), "{backend} emitted code");
    }
}

// [flow-use-after-move] [fate-partial-move] [fate-poison] ROADMAP 0o's
// unsound rules, fixed in the IR flow pass (R8 piece 5): each of these
// checked clean before and failed rustc with E0382 (or, `linked`, printed 0
// on Rust and 1 on Kotlin). Every gate that emits now refuses them, at the
// read: an `if` arm that assigns and then moves (`arm`), a `when` arm that
// assigns and leaves (`leaving`), a move before `continue` read at the top
// of the next iteration (`skips`, at the read and at the move itself), a
// value linked and poisoned in an arm (`linked`), and a field's new value
// reading the moved field (`back`).
#[test]
fn the_gates_refuse_what_the_flow_pass_let_through() {
    let dir = src_dir("flow_0o");
    fs::write(dir.join("main.sv"), r#"struct Rec canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

fn eat(s: Str) -> None => !s {
}

fn look(s: Str) -> Int {
    return size(s)
}

fn mk() -> Str {
    return "a${1}"
}

fn eatl(xs: Mut List<Str>) -> None => !xs {
}

fn relist(xs: Mut List<Str>) -> Mut List<Str> => !xs {
    return xs
}

fn takep(p: Rec) -> None => !p {
}

fn arm(c: Bool) -> Int {
    let s = mk()
    if c {
        s = mk()
        eat(s)
    }
    return look(s)
}

fn leaving(v: Int | Bool) -> Int {
    let s = mk()
    eat(s)
    when v {
        is Int {
            s = mk()
            return look(s)
        }
        is Bool {
        }
    }
    return look(s)
}

fn skips(d: Bool) -> Int {
    let s = mk()
    let n = 0
    let i = 0
    while i < 2 {
        i = i + 1
        n = n + look(s)
        if d {
            eat(s)
            continue
        }
    }
    return n
}

fn linked(c: Bool) -> Int {
    let xs = Mut List<Str>()
    let t = Mut List<Str>()
    if c {
        t = xs
        add(xs, "a")
    }
    return size(t)
}

fn back(p: Mut Rec) -> None => !p {
    eatl(p.tags)
    p.tags = relist(p.tags)
    takep(p)
}

fn main() {
}
"#).unwrap();
    let wanted = [
        "`s` cannot be used here: it was consumed (moved) at line 31; reassign it before use, or `copy` it where it is moved --> main.sv:33:17",
        "`s` cannot be used here: it was consumed (moved) at line 38; reassign it before use, or `copy` it where it is moved --> main.sv:47:17",
        "`s` cannot be used here: it was consumed (moved) at line 58; reassign it before use, or `copy` it where it is moved --> main.sv:56:22",
        "`s` cannot be used here: it was consumed (moved) at line 58; reassign it before use, or `copy` it where it is moved --> main.sv:58:17",
        "`t` cannot be used here: it was bound from `xs` and shares its fate, and `xs` was mutated at line 70 after the binding; bind it with `copy(...)` to keep an independent value, or reassign it before use --> main.sv:72:17",
        "`p.tags` cannot be used here: `p.tags` was moved out of `p` at line 76, and this reads the same data; `copy` at the site that moved it to keep this readable --> main.sv:77:21",
    ];
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    for w in wanted {
        assert!(stderr.contains(w), "missing `{w}` in: {stderr}");
    }
    assert!(stderr.contains(&format!("{} errors", wanted.len())), "stderr: {stderr}");
    for backend in ["rust", "kotlin"] {
        let target = dir.join(format!("out_{backend}"));
        let out = salvo(&["compile", "--backend", backend, "--src", dir.to_str().unwrap(), "--target", target.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend} compiled a use after a move: {stderr}");
        for w in wanted {
            let message = w.split(" --> ").next().unwrap();
            assert!(stderr.contains(message), "{backend}: missing `{message}` in: {stderr}");
        }
    }
}

// [deduce-consume] A *declared* consuming parameter is enforced like an
// inferred one used to be (second checking round): a callee that consumes
// its parameter moves it, so the caller's argument is consumed with the
// annotation in the signature.
//
// NOTE (deduction-defaults change): deductions are no longer inferred from
// the body, so `give_back`'s move (it returns `list`) has to be declared
// with `=> !list` — it can no longer be inferred from the `return list`
// alone. The test's point (a parameter's move at the callee reaches the
// caller's argument with no annotation *at the call site*) still holds.
#[test]
fn inferred_moves_consume_arguments() {
    let dir = src_dir("consume_inferred");
    fs::write(
        dir.join("main.sv"),
        "fn give_back(list: List<Str>) -> List<Str>\n=> !list {\n    return list\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    \
         let strings = List(\"a\", \"b\")\n    give_back(strings)\n    \
         println(\"${strings.size()}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`strings` cannot be used here: it was consumed (moved) at line 9"),
        "stderr: {stderr}"
    );

    // Rebinding through the return value keeps it usable.
    fs::write(
        dir.join("main.sv"),
        "fn give_back(list: List<Str>) -> List<Str>\n=> !list {\n    return list\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    \
         let strings: List<Str> = List(\"a\", \"b\")\n    \
         strings = give_back(strings)\n    \
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
         fn take_head<T>(list: NonEmpty Mut List<T>) -> T => list: Mut {\n    return list.get(0) ?: unreachable!()\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    let strings = Mut List(\"a\", \"b\")\n    \
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
         fn head<T>(list: NonEmpty List<T>) -> T {\n    return list.get(0) ?: unreachable!()\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    let strings = List(\"a\", \"b\")\n    \
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
         fn main() [use] {\n    use StdConsole\n    \
         let strings = List(\"a\", \"b\")\n    if strings is NonEmpty {\n        consume(strings)\n    }\n    println(\"${strings.size()}\")\n    \
         let s = List(\"x\")\n    let i = 0\n    while i < 3 {\n        println(\"${s.size()}\")\n        consume(s)\n        i++\n    }\n}\n",
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
         fn main() [use] {\n    use StdConsole\n    \
         let s = List(\"x\")\n    let i = 0\n    while i < 3 {\n        consume(s)\n        s = List(\"y\")\n        i++\n    }\n    println(\"${s.size()}\")\n}\n",
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
    // `Ok`/`Err` and their constructors come from `std.core.result`.
    let prelude = "fn consume(v: Ok Str) -> None => !v {\n}\n\n";

    // Consumed in one fall-through arm -> unusable after the `when`.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}fn main() [use] {{\n    use StdConsole\n    \
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
            "{prelude}fn main() [use] {{\n    use StdConsole\n    \
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
            "{prelude}fn both(flag: Bool) {{\n    let a = List(\"a\")\n    \
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
            "{prelude}fn one_exits(flag: Bool) {{\n    let b = List(\"b\")\n    \
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
            "{prelude}fn consuming_exits(flag: Bool) {{\n    let c = List(\"c\")\n    \
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
         fn take_head<T>(list: NonEmpty Mut List<T>) -> T => list: Mut {\n    return list.get(0) ?: unreachable!()\n}\n\n\
         fn partial(flag: Bool) {\n    let strings = Mut List(\"a\", \"b\")\n    \
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
         fn main() [use] {\n    use StdConsole\n    let s = List(\"x\")\n    \
         for i in List(1, 2, 3).iter() {\n        println(\"${s.size()}\")\n        consume(s)\n    }\n}\n",
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
         fn main() [use] {\n    use StdConsole\n    println(describe(ok(\"x\")))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "stderr: {stderr}");
}

// [qual-result-tags] [qual-ctor-fn] `std.core.result` supplies the result tags: a program can
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
         fn main() [use] {\n    use StdConsole\n    \
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
         fn main() [use] {\n    use StdConsole\n    \
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
         fn move_derived() -> Int {\n    let xs = List(1, 2)\n    let ys = xs\n    consume(ys)\n    return size(xs)\n}\n\n\
         fn mutate_derived() -> Int {\n    let xs = Mut List(1, 2)\n    let ys = xs\n    add(ys, 3)\n    return size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [fate-move-mode] S2: moving or mutating a derived variable whose
    // roots are owned is legal — the binding takes ownership and the
    // *root* is consumed at the binding; its later use names the binding
    // (the IR flow pass's diagnostic since R8 piece 3c).
    assert_eq!(
        stderr
            .matches("`xs` cannot be used here: `ys` was bound from it at line ")
            .count(),
        2,
        "stderr: {stderr}"
    );
    assert!(stderr.contains("`ys` was bound from it at line 6 and later moves the value"), "stderr: {stderr}");
    assert!(stderr.contains("`ys` was bound from it at line 13 and later moves the value"), "stderr: {stderr}");
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
    assert!(stderr.contains("copy"), "stderr: {stderr}");

    // A *written-kept* parameter root keeps the S1 error at the move
    // site [fate-derived-readonly]: you cannot move out of a borrow.
    let dir = src_dir("fate_readonly_kept");
    fs::write(
        dir.join("main.sv"),
        "fn return_derived(v: Str) -> Str => v {\n    let w = v\n    return w\n}\n\n\
         fn read_derived(v: Str) -> Int => v {\n    let w = v\n    return size(w)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("cannot return `w`: it was bound from `v`"),
        "stderr: {stderr}"
    );
    // Exactly the one violation: reading a derived variable is fine.
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
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
         fn mutated() -> Int {\n    let xs = Mut List(1)\n    let ys = xs\n    \
         add(xs, 2)\n    return size(ys)\n}\n\n\
         fn moved() -> Int {\n    let xs = List(1)\n    let ys = xs\n    \
         consume(xs)\n    return size(ys)\n}\n\n\
         fn reassigned() -> Int {\n    let xs = List(1)\n    let ys = xs\n    \
         xs = List(2, 3)\n    return size(ys)\n}\n\n\
         fn unused_poison_is_fine() {\n    let xs = Mut List(1)\n    let ys = xs\n    \
         add(xs, 2)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // The IR flow pass's diagnostic since R8 piece 3c, naming the event's
    // line.
    assert!(
        stderr.contains(
            "`ys` cannot be used here: it was bound from `xs` and shares its fate, \
             and `xs` was mutated at line 7 after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`xs` was moved at line 14 after the binding"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`xs` was reassigned at line 21 after the binding"),
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
         fn independent() -> Int {\n    let xs = Mut List(1)\n    let ys = copy(xs)\n    \
         add(xs, 2)\n    add(ys, 3)\n    return size(ys) + size(xs)\n}\n\n\
         fn main() [use] {\n    use StdConsole\n    \
         println(longest_name(array_of(Person {name: \"a\"})))\n    println(\"${independent()}\")\n}\n",
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
         let out: List<Int> = List(0)\n    \
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

// [deduce-field] Rung ⑤ v2 (2026-09-25): a contents mutation through a
// projection leaves a *handle to the same field* standing — `t` names the
// very list `add` grew, and both backends see it (Rust re-materializes the
// place, Kotlin holds the object), which is what the historical `diverged`
// case needed. What still refuses: mutating through a *derived* variable,
// which is an error like any mutation of one.
#[test]
fn fate_mut_projection_arguments_keep_a_handle_but_refuse_derived_mutation() {
    let dir = src_dir("fate_mut_proj");
    fs::write(
        dir.join("main.sv"),
        "struct Holder {\n    tags: Mut List<Int>\n}\n\n\
         fn kept() -> Int {\n    let h = Holder {tags: Mut List(1)}\n    \
         let t = h.tags\n    add(h.tags, 2)\n    return size(t)\n}\n\n\
         fn mutate_through_derived(other: Holder) -> None => other {\n    \
         let h = other\n    add(h.tags, 2)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("cannot mutate `h`: it was bound from `other`"),
        "stderr: {stderr}"
    );
    // `kept` is clean: exactly the one violation above.
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
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
         fn list_store() {\n    let s = \"x\"\n    let a = List(s)\n    read(s)\n}\n\n\
         fn struct_store() {\n    let s = \"x\"\n    let b = Box {item: s}\n    read(s)\n}\n\n\
         fn spread_store() {\n    let b = Box {item: \"x\"}\n    let c = Box {...b}\n    \
         read(b.item)\n}\n\n\
         fn break_in_branch() {\n    let s = \"x\"\n    let r = while true {\n        \
         if true {\n            break s\n        }\n    }\n    read(s)\n}\n\n\
         fn use_ctor() [use] {\n    let s = \"hi\"\n    use FixedGreeter(s)\n    read(s)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [flow-use-after-move] The use is the IR flow pass's to report, and it
    // names the line of the move: literal stores (tuple, struct) each
    // consume `s`.
    for line in [20, 32] {
        assert!(
            stderr.contains(&format!("`s` cannot be used here: it was consumed (moved) at line {line};")),
            "line {line}, stderr: {stderr}"
        );
    }
    // [col-no-literal] A list is built by a constructor call now, and
    // `List` consumes its elements like any `!` parameter.
    assert!(stderr.contains("`s` cannot be used here: it was consumed (moved) at line 26;"), "stderr: {stderr}");
    // Spread consumes its base.
    assert!(stderr.contains("`b` cannot be used here: it was consumed (moved) at line 38;"), "stderr: {stderr}");
    // `break s` inside an always-exiting branch still reaches the code
    // after the loop (the loop exit merges break-path states).
    assert!(stderr.contains("`s` cannot be used here: it was consumed (moved) at line 46;"), "stderr: {stderr}");
    // Handler-constructor arguments are stored in the handler.
    assert!(stderr.contains("`s` cannot be used here: it was consumed (moved) at line 54;"), "stderr: {stderr}");
    assert!(stderr.contains("6 errors"), "stderr: {stderr}");

    // [fate-move-mode] S2: storing/spreading a derived variable of owned
    // roots is legal — the binding takes ownership, so the *root* is
    // consumed at the binding and its later use names the binding. The IR
    // flow pass's diagnostic since R8 piece 3c.
    fs::write(
        dir.join("main.sv"),
        "struct Box {\n    item: Str\n}\n\n\
         fn read(s: Str) -> None => s {\n}\n\n\
         fn store_derived() {\n    let xs = List(1, 2)\n    let ys = xs\n    let t = (ys, 1)\n    \
         read_list(xs)\n}\n\n\
         fn read_list(v: List<Int>) -> None => v {\n}\n\n\
         fn spread_derived() {\n    let b = Box {item: \"x\"}\n    let d = b\n    \
         let c = Box {...d}\n    read(b.item)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr
            .contains("`xs` cannot be used here: `ys` was bound from it at line 10 and later moves the value"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`b` cannot be used here: `d` was bound from it at line 20 and later moves the value"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
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
         let t = (copy(s), 1)\n    let a = List(copy(s))\n    \
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
         fn longest_name(persons: List<Person>) -> Str\n=> !persons {\n    let longest = \"\"\n    \
         for person in persons {\n        let name = person.name\n        \
         if size(name) > size(longest) {\n            longest = name\n        }\n    }\n    \
         return longest\n}\n\n\
         fn per_iteration() {\n    for s in List(\"a\", \"b\") {\n        consume(s)\n    }\n}\n\n\
         fn immutable_projection_store(person: Person) -> Label => person {\n    \
         return Label {text: person.name}\n}\n\n\
         fn copy_keeps_source() -> Int {\n    let xs = List(1, 2)\n    let ys = copy(xs)\n    \
         consume_list(ys)\n    return size(xs)\n}\n\n\
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let people = List(Person {name: \"Ada\", age: 36}, Person {name: \"Grace\", age: 45})\n    \
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
// diagnostic name the field rather than the whole variable). The parity probe
// is its own program since R8 piece 3b, when its error became the IR flow
// pass's and a checker error anywhere hid those (per declaration since R8
// piece 9).
#[test]
fn s2_move_mode_ancestors_are_consumed() {
    let dir = src_dir("s2_negative");
    fs::write(
        dir.join("main.sv"),
        "struct Holder {\n    tags: Mut List<Int>\n}\n\n\
         struct Wrapper {\n    item: Mut List<Int>\n}\n\n\
         fn wrap(list: Mut List<Int>) -> Wrapper => !list {\n    return Wrapper {item: list}\n}\n\n\
         fn kept_leak(h: Holder) -> Wrapper => h {\n    return wrap(h.tags)\n}\n\n\
         fn kept_remedy(h: Holder) -> Wrapper => h {\n    return wrap(copy(h.tags))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // A kept parameter's mutable data cannot be moved out; `copy` is the
    // remedy (kept_remedy is clean).
    assert!(
        stderr.contains("cannot move mutable data out of `h`: it is a kept parameter"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");

    // A written-kept parameter binding keeps the S1 error at the move
    // site [fate-derived-readonly] — the IR flow pass's since R8 piece 8.
    let dir = src_dir("s2_negative_kept");
    fs::write(dir.join("main.sv"), "fn kept_binding(v: Str) -> Str => v {\n    let w = v\n    return w\n}\n").unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("cannot return `w`: it was bound from `v`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");

    // Chain: `xs` was consumed at `a`'s binding (the whole chain is
    // move-mode); the use-site error names the binding. The IR flow pass's
    // diagnostic since R8 piece 3c.
    fs::write(
        dir.join("main.sv"),
        "fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn chain() -> Int {\n    let xs = List(1, 2)\n    let a = xs\n    let b = a\n    \
         consume_list(b)\n    return size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`xs` cannot be used here: `a` was bound from it at line 6 and later moves the value"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");

    // The parity probe is rejected: `wrap(h.tags)` moved mutable data out
    // of `h`, so the later `add(h.tags, 9)` cannot observe an alias on one
    // backend and a clone on the other. Since L5 the diagnostic names the
    // *field* that left rather than the whole variable — `h` itself stays
    // usable, and reading `h.tags` back is what fails [fate-partial-move] —
    // and since R8 piece 3b the line it left at.
    let dir = src_dir("s2_negative_parity");
    fs::write(
        dir.join("main.sv"),
        "struct Holder {\n    tags: Mut List<Int>\n}\n\n\
         struct Wrapper {\n    item: Mut List<Int>\n}\n\n\
         fn wrap(list: Mut List<Int>) -> Wrapper => !list {\n    return Wrapper {item: list}\n}\n\n\
         fn parity_probe() -> Int {\n    let h = Holder {tags: Mut List(1)}\n    \
         let w = wrap(h.tags)\n    add(h.tags, 9)\n    return size(w.item)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`h.tags` cannot be used here: `h.tags` was moved out of `h` at line 15, \
             and this reads the same data"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}

// [deduce-same-call] L3: arguments are evaluated left to right, so a
// value moved by an earlier argument of the same call cannot be
// mentioned by a later one (the IR flow pass's diagnostic since R8 piece 2) — double moves (`f(a, a)`), move-then-read
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
         fn double_move() {\n    let xs = List(1, 2)\n    eat_two(xs, xs)\n}\n\n\
         fn move_then_read() {\n    let xs = List(1, 2)\n    consume_first(xs, size(xs))\n}\n\n\
         fn remedy() {\n    let xs = List(1, 2)\n    eat_two(copy(xs), xs)\n}\n\n\
         fn kept_then_read() {\n    let ys = List(3)\n    keep_first(ys, size(ys))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [flow-use-after-move] The IR reads a call's arguments in order, so
    // the flow pass reports the later argument, naming the call's line.
    for (line, at) in [(12, "main.sv:12:17"), (17, "main.sv:17:28")] {
        assert!(
            stderr.contains(&format!("`xs` cannot be used here: it was consumed (moved) at line {line}; reassign it before use, or `copy` it where it is moved --> {at}")),
            "line {line}, stderr: {stderr}"
        );
    }
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
//
// NOTE (deduction-defaults change): `infer_claims`'s consumption of `xs`
// used to be inferred purely from the lambda capturing-and-mutating it;
// deductions are no longer inferred from the body, so the claim now has
// to be written (`=> !xs`). The test's point (the claim, once established,
// propagates to the caller's argument) still holds.
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
         fn used_before_mutation() -> Int {\n    let xs = Mut List(1, 2)\n    \
         let f = (n: Int) -> { return n + size(xs) }\n    let r = apply(f, 1)\n    \
         add(xs, 9)\n    return r + size(xs)\n}\n\n\
         fn mutate_capture_remedy() -> Int {\n    let xs = Mut List(1, 2)\n    \
         let snapshot = copy(xs)\n    let g = () -> { add(snapshot, 1) }\n    run(g)\n    \
         return size(xs)\n}\n\n\
         fn move_capture_rejected() {\n    let xs = List(1, 2)\n    \
         let h = () -> { consume_list(xs) }\n    run(h)\n}\n\n\
         fn move_capture_remedy() {\n    let xs = List(1, 2)\n    \
         let h = () -> { consume_list(copy(xs)) }\n    run(h)\n}\n\n\
         fn kept_mutates(xs: Mut List<Int>) -> None => xs: Mut {\n    \
         let g = () -> { add(xs, 1) }\n    run(g)\n}\n\n\
         fn infer_claims(xs: Mut List<Int>)\n=> !xs {\n    let g = () -> { add(xs, 1) }\n    run(g)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // A capture-consuming lambda is legal since L7b but `once`-typed
    // [fn-once]: passing it where a plain fn is expected fails at the
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
    // The positive matrix (immutable captures, pre-mutation use, `copy`
    // remedies) is clean: exactly the two violations.
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");

    // Mutating a mutable read-capture's root poisons the closure: the IR
    // flow pass's diagnostic since R8 piece 3c.
    fs::write(
        dir.join("main.sv"),
        "fn apply(f: (Int) -> Int, v: Int) -> Int {\n    return f(v)\n}\n\n\
         fn poisoned_after_mutation() -> Int {\n    let xs = Mut List(1, 2)\n    \
         let f = (n: Int) -> { return n + size(xs) }\n    let before = apply(f, 1)\n    \
         add(xs, 9)\n    return before + apply(f, 1)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`f` cannot be used here: it was bound from `xs` and shares its fate, \
             and `xs` was mutated at line 9 after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");

    // [flow-use-after-move] The uses after a move are the IR flow pass's.
    fs::write(
        dir.join("main.sv"),
        "fn run(f: once () -> None) {\n    f()\n}\n\n\
         fn mutate_capture_consumes() -> Int {\n    let xs = Mut List(1, 2)\n    \
         let g = () -> { add(xs, 1) }\n    run(g)\n    return size(xs)\n}\n\n\
         fn infer_claims(xs: Mut List<Int>)\n=> !xs {\n    let g = () -> { add(xs, 1) }\n    run(g)\n}\n\n\
         fn claim_reaches_caller() -> Int {\n    let xs = Mut List(1)\n    \
         infer_claims(xs)\n    return size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // A mutated capture is consumed at creation (the lambda's line).
    assert!(
        stderr.contains("`xs` cannot be used here: it was consumed (moved) at line 7;"),
        "stderr: {stderr}"
    );
    // The capture claim propagates: the caller's argument is consumed.
    assert!(
        stderr.contains("`xs` cannot be used here: it was consumed (moved) at line 20;"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
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
         fn drop_result() {\n    open_file(\"data.txt\")\n}\n\n\
         fn return_while_owing(flag: Bool) -> Int {\n    let h = open_file(\"data.txt\")\n    \
         if flag {\n        return 0\n    }\n    close_file(h)\n    return 1\n}\n\n\
         fn copy_refused() {\n    let h = open_file(\"data.txt\")\n    let c = copy(h)\n    \
         close_file(h)\n    close_file(c)\n}\n\n\
         fn swallow() {\n    let conn = Conn {tags: Mut List(1)}\n    \
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
         fn caller_obligated() {\n    let h = ok_return()\n    close_file(h)\n}\n\n\
         fn maybe_leak(flag: Bool) {\n    let h = open_file(\"data.txt\")\n    \
         if flag {\n        close_file(h)\n    }\n}\n\n\
         fn overwrite() {\n    let h = open_file(\"a.txt\")\n    h = open_file(\"b.txt\")\n    \
         close_file(h)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [ir-analyze] The path-dependent rules are the IR analysis's — the
    // leaks (`leak`, `return_while_owing`), a value consumed on some paths
    // only (`maybe_leak`), an assignment over a live one (`overwrite`, R8
    // piece 7: the last two were the checker's) — reported, per declaration,
    // beside the structural violations in the other fns.
    let leaks = stderr.matches("`h` still owns a linear value when the path leaves").count();
    assert_eq!(leaks, 2, "expected both path-exit leaks, stderr: {stderr}");
    assert!(
        stderr.contains("`h` owns a linear value that is consumed on some paths but not others; discharge or move it on every path --> main.sv:114:5"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("assigning to `h` drops the linear value it still owns; move it onward or `discard(h)` first --> main.sv:121:5"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("this expression produces a linear value that is dropped immediately"),
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
    // Three structural violations and four path-dependent ones.
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
         let h2 = FileHandle {fd: 2}\n    let xs = List(h, h2)\n}\n",
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
         fn hold<T canbe linear>(value: T) -> T\n=> !value {\n    return value\n}\n\n\
         fn eat<T canbe linear>(value: T) -> None => !value {\n}\n\n\
         fn forward<T canbe linear>(value: T) -> T\n=> !value {\n    let kept = unopted(value)\n    \
         return kept\n}\n\n\
         fn unopted<T>(value: T) -> T\n=> !value {\n    return value\n}\n\n\
         fn bad_clause<T canbe Mut>(value: T) -> T\n=> !value {\n    return value\n}\n\n\
         fn variadic_refused() {\n    let h = open_file(1)\n    \
         let h2 = open_file(2)\n    let xs = Mut List(h, h2)\n}\n\n\
         fn ok_hold() {\n    let h = hold(open_file(9))\n    close(h)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // [ir-analyze] The opted body treats `T` as linear: `eat` drops its
    // moved `value`, a path-exit leak the IR reports beside the structural
    // errors in the other fns.
    assert!(
        stderr.contains("`value` still owns a linear value when the path leaves; move it onward (pass, return, or store it) or discharge it --> main.sv:17:52"),
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
    // [linear-composite] A linear value cannot be stored in a variadic `T[]`.
    assert!(
        stderr.contains("cannot be stored in the `T[]` of `Mut List<T>`"),
        "stderr: {stderr}"
    );
    // Four structural violations and the opted body's leak.
    assert!(stderr.contains("5 errors"), "stderr: {stderr}");
}

// [fn-once] L7b: `once` on fn types means callable at most once,
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
         fn run_in_loop_bad(f: once () -> None) {\n    for i in List(1, 2) {\n        f()\n    }\n}\n\n\
         fn run_maybe(f: once () -> None, flag: Bool) {\n    if flag {\n        f()\n    }\n}\n\n\
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn once_where_once_ok() {\n    let xs = List(1, 2)\n    \
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
    // [flow-use-after-move] The uses are the IR flow pass's, naming the
    // move: the second call (moved by the first, line 6), the loop's back
    // edge (line 12), and a call after passing the value on (the escape
    // rule, line 38).
    for line in [6, 12, 38] {
        assert!(
            stderr.contains(&format!("`f` cannot be used here: it was consumed (moved) at line {line};")),
            "line {line}, stderr: {stderr}"
        );
    }
    // [fn-once] D6 (user decision 2026-09-12): `once` is valid on any
    // type now, so `not_a_fn(x: once Int)` draws no error.
    assert!(
        !stderr.contains("`once` applies to function types"),
        "stderr: {stderr}"
    );
    // run_maybe, once_where_once_ok, plain_where_once_ok, not_a_fn are clean.
    assert!(stderr.contains("3 errors"), "stderr: {stderr}");

    // A `once` lambda cannot go where a plain fn is expected — a checker
    // error, and the fn with it reports no flow diagnostic.
    fs::write(
        dir.join("main.sv"),
        "fn run_plain(f: () -> None) {\n    f()\n    f()\n}\n\n\
         fn consume_list(v: List<Int>) -> None => !v {\n}\n\n\
         fn once_where_plain_bad() {\n    let xs = List(1, 2)\n    \
         let g = () -> { consume_list(xs) }\n    run_plain(g)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("no matching overload for `run_plain(once () -> "),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("cannot be used here"), "stderr: {stderr}");
}

// [proj-readonly-return] L7c: `-> proj(p) T?` marks a derived
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
         fn find_adult(persons: List<Person>) -> proj(persons) Person? => persons {\n    \
         for person in persons {\n        if person.age >= 18 {\n            return person\n        }\n    }\n    \
         return None\n}\n\n\
         fn forwarded(persons: List<Person>, tag: Str) -> proj(persons) Person? => persons, tag {\n    \
         return first(persons)\n}\n\n\
         fn bad_independent(persons: List<Person>) -> proj(persons) Person? => persons {\n    \
         return Person {name: \"made up\", age: 1}\n}\n\n\
         fn bad_moved(persons: List<Person>) -> proj(persons) Person? => !persons {\n    \
         return None\n}\n\n\
         fn bad_param(persons: List<Person>) -> proj(nobody) Person? => persons {\n    \
         return None\n}\n\n\
         fn escape_borrowed() {\n    let people = List(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        take(h)\n    }\n}\n\n\
         fn ok_use() -> Int {\n    let people = List(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        return h.age\n    }\n    \
         return 0\n}\n\n\
         fn ok_copy_escape() {\n    let people = List(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    if head is Person h {\n        take(copy(h))\n    }\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "this function returns `proj(persons)`, so every returned \
             value must be derived from `persons`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`proj(persons)` requires `persons` to be kept"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`proj(nobody)` names no parameter"),
        "stderr: {stderr}"
    );
    // The borrowed result can never be moved; `copy` is the remedy
    // (ok_copy_escape is clean). [proj-type] names the projection's type.
    assert!(
        stderr.contains("`h` is a projection (`proj Person`), and `take` consumes `p`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("4 errors"), "stderr: {stderr}");

    // Mutating the argument poisons the derived result: the IR flow pass's
    // diagnostic since R8 piece 3c.
    fs::write(
        dir.join("main.sv"),
        "struct Person {\n    name: Str,\n    age: Int\n}\n\n\
         fn poison_after_mutation() -> Int {\n    \
         let people = Mut List(Person {name: \"Ada\", age: 36})\n    \
         let head = first(people)\n    add(people, Person {name: \"Grace\", age: 45})\n    \
         return head.age\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`head` cannot be used here: it was bound from `people` and shares \
             its fate, and `people` was mutated at line 9 after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
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
        "fn apply_consuming(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v => !data {\n    \
         return f(data)\n}\n\n\
         fn apply_keeping(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] v => data {\n    \
         return f(data) + f(data)\n}\n\n\
         fn eat(v: List<Int>) -> Int => !v {\n    return 0\n}\n\n\
         fn consuming_where_keeping_bad() -> Int {\n    let xs = List(1, 2)\n    \
         return apply_keeping(eat, xs)\n}\n\n\
         fn kept_param_consumed_bad() -> Int {\n    let xs = List(1, 2)\n    \
         return apply_keeping((v: List<Int>) -> {\n        let w = eat(v)\n        \
         return w\n    }, xs)\n}\n\n\
         fn caller_keeps() -> Int {\n    let xs = List(1, 2)\n    \
         let n = apply_keeping((v: List<Int>) -> { return size(v) }, xs)\n    \
         return n + size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
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
    // caller_keeps is clean: exactly the two violations.
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");

    // [flow-use-after-move] The uses after a move through a fn value's
    // contract are the IR flow pass's.
    fs::write(
        dir.join("main.sv"),
        "fn apply_consuming(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v => !data {\n    \
         return f(data)\n}\n\n\
         fn double_use_bad(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v => !data {\n    \
         let a = f(data)\n    return f(data)\n}\n\n\
         fn caller_loses() -> Int {\n    let xs = List(1, 2)\n    \
         let n = apply_consuming((v: List<Int>) -> { return size(v) }, xs)\n    \
         return n + size(xs)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    // Double use through a consuming contract.
    assert!(
        stderr.contains("`data` cannot be used here: it was consumed (moved) at line 6;"),
        "stderr: {stderr}"
    );
    // The consuming contract propagates: the caller's argument dies.
    assert!(
        stderr.contains("`xs` cannot be used here: it was consumed (moved) at line 12;"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");
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
             fn main() [use] {{\n    use StdConsole\n    \
             let xs = Mut List(1, 2)\n    \
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
             fn main() [use] {{\n    use StdConsole\n    \
             let xs = List(1, 2)\n    \
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
             fn main() [use] {{\n    use StdConsole\n    \
             let xs = Mut List(1)\n    \
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
    //
    // NOTE (deduction-defaults change): the contract is now a pure function
    // of the signature (user decision 2026-10-11) — `from_written` no longer
    // consults body mutations, so the old "mutated by this function, so the
    // deduction cannot keep every qualifier" message (raised at the written
    // clause itself) can no longer fire from a plain `Mut List<Int>` with no
    // qualifier to invalidate. The soundness rule survives, enforced instead
    // by the body-validation pass with a more precise message naming the
    // qualifier it may lose; reproducing it needs a qualifier on the
    // parameter's type for the body to actually invalidate.
    fs::write(
        dir.join("main.sv"),
        format!(
            "{prelude}\
             fn grow(list: Mut NonEmpty List<Int>) -> None => list {{\n    list.add(1)\n}}\n"
        ),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("promises qualifier `NonEmpty` on `list`, but the body may remove it"),
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
        stderr.contains("either exhaustive (plain qualifier names"),
        "stderr: {stderr}"
    );

    // [deduce-reapply] `+Qual` says the function **establishes** the claim
    // (user decisions 2026-09-22): legal in the qualifier's own file, whether or
    // not the parameter already declares it, and trusted there.
    fs::write(
        dir.join("main.sv"),
        "qualifier A of Int\n\nfn f(x: Int) -> None\n=> x: +A {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");

    // …but the claim still has to make sense for the parameter's type.
    fs::write(
        dir.join("main.sv"),
        "qualifier A of Int\n\nfn f(s: Str) -> None\n=> s: +A {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("qualifier `A` does not apply to `Str`"),
        "stderr: {stderr}"
    );

    // A type other than `Never` is not supported yet (D1b).
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
// only std may write it [platform-intrinsic-std-only], so it never appears in a
// user source like these.)
#[test]
fn bodyless_declarations_must_be_explicit() {
    let dir = src_dir("decl_explicit");

    // An effect member missing its clause (effects are forbidden there
    // [effect-member-no-effects], so they are not asked for, and a missing
    // return type is `None` [effect-member-none]).
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
        !stderr.contains("effect member `eat` must declare its return type")
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
         fn main() [use] -> None {\n    use StdConsole\n    use Bin\n    \
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
         fn main() [use] -> None {\n    use StdConsole\n    \
         let xs = Mut List(Handle {id: 1})\n    \
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

// [platform-intrinsic-std-only] `intrinsic` marks a declaration the *compiler*
// implements, so only the standard library may write it. A user source
// analyzed here is not std, so an `intrinsic` declaration in it is an
// error naming the one interop path customer code does have — a
// `platform handler`. (The declaration is fully explicit so this is the
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
    assert!(stderr.contains("`platform handler`"), "stderr: {stderr}");
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
    let xs: Mut List<Int> = Mut List()
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

/// [fn-overload-at] [diag-structured] A redundant selector is a **warning**:
/// the program is correct, so it must still compile, and the warning has to
/// reach the CLI's report and its JSON — a severity the emission gate ignores.
#[test]
fn a_redundant_selector_warns_without_failing() {
    let dir = src_dir("redundant_selector");
    let source = "\
fn main() [use] {
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    let _n = size@core.list(xs)
}
";
    fs::write(dir.join("main.sv"), source).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a redundant `@` is not an error: {stderr}");
    assert!(
        stderr.contains("warning: this `@core.list` is not needed"),
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

// [proj-opt-slot] A union arm that borrows (`Emitted (proj Str) | Finished`)
// passes whole into a kept owned union parameter: the parameter reads it, arm
// by arm (user decision 2026-10-10; refused from 2026-09-12 until then). A
// parameter that consumes its argument still refuses it.
#[test]
fn a_kept_parameter_takes_a_borrowed_union_arm() {
    let dir = src_dir("proj_arm_param");
    fs::write(
        dir.join("main.sv"),
        "fn show(step: Emitted Str | Finished) [Console] -> None {\n    \
         when step {\n        is Emitted { println(step) }\n        is Finished { println(\"done\") }\n    }\n}\n\n\
         fn keep(step: Emitted Str | Finished) [] -> None => !step {\n}\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let words = List(\"ann\", \"bo\")\n    let p = iter(words)\n    show(next(p))\n    keep(next(p))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "this argument holds a borrowed value (`proj Emitted Str | Finished`) \
             where `keep` expects an owned one for `step`"
        ),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("where `show` expects"), "stderr: {stderr}");
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
         fn keep(w: proj Str) -> proj(w) Str => w {\n    return w\n}\n\n\
         struct Chars {\n    n: Int\n}\n\n\
         iter fn iter(c: Chars) -> Emitted Str | Finished {\n    \
         state CharsIter {\n        at: Int = 0\n    }\n    \
         if at >= c.n {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(\"x\")\n}\n\n\
         fn keep_all<It, T>(it: Mut It, ?Next<It, T>) -> Mut List<T> => it: Mut {\n    \
         let out = Mut List<T>()\n    for x in it {\n        add(out, x)\n    }\n    return out\n}\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let words = List(\"ann\", \"bo\")\n    \
         let p = iter(words)\n    \
         let kept = map(p, keep)\n    \
         eat(kept)\n    \
         let free = keep_all(iter(Chars {n: 2}))\n    \
         println(\"${size(free)}\")\n    \
         let parts = Mut List(Mut Str(\"a\"))\n    \
         let e = get(parts, 0) ?: unreachable!()\n    \
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
    // `copy(e)` on a `proj Mut Str` is a `Mut Str`: one level un-projected,
    // the `Mut` kept.
    assert!(
        stderr.contains("expected `Int`, found `Mut Str`"),
        "stderr: {stderr}"
    );
    // The generator instantiation is unlinked (`free` draws no error), so
    // exactly the two violations above are reported.
    assert!(stderr.contains("2 errors"), "stderr: {stderr}");

    // The view is fate-linked *through* the temporary pass to the container:
    // consuming the container poisons it. The IR flow pass's diagnostic
    // since R8 piece 3c.
    fs::write(
        dir.join("main.sv"),
        "fn eat(xs: List<Str>) -> None => !xs {}\n\n\
         fn keep_all<It, T>(it: Mut It, ?Next<It, T>) -> Mut List<T> => it: Mut {\n    \
         let out = Mut List<T>()\n    for x in it {\n        add(out, x)\n    }\n    return out\n}\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let words = List(\"ann\", \"bo\")\n    \
         let more = keep_all(iter(words))\n    \
         eat(words)\n    \
         println(\"${size(more)}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "`more` cannot be used here: it was bound from `words` and shares \
             its fate, and `words` was moved at line 15 after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}

// [fate-lambda-view] A capturing lambda is a *view*: it holds borrows of its
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
         fn main() [use] {\n    use StdConsole()\n    \
         // 1. capture-free: the one-liner binds with no links\n    \
         let words = List(\"ann\", \"bo\")\n    \
         let p = iter(words)\n    \
         let kept = map(p, w -> w)\n    \
         println(\"${size(kept)}\")\n    \
         // 2. a capture-rooted projection: legal, linked to the capture\n    \
         let all = List(\"a\", \"b\", \"c\", \"d\")\n    \
         let indices = List(1, 3)\n    \
         let picked = map(iter(indices), i -> (get(all, i) ?: unreachable!()))\n    \
         println(\"${size(picked)}\")\n    \
         // 3. the named-lambda form is linked the same way: consuming\n    \
         // the capture poisons the result derived through the closure\n    \
         let all2 = List(\"a\", \"b\")\n    \
         let f = i -> (get(all2, i) ?: unreachable!())\n    \
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
             its fate, and `all2` was moved at line 20 after the binding"
        ),
        "stderr: {stderr}"
    );
    // Scenarios 1 and 2 are clean: exactly the one violation.
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}

// [proj-infer] A written opaque projection annotation (`-> … holds proj(it)`)
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
         fn keep_all<It, T>(it: Mut It, labels: List<Str>, ?Next<It, T>) \
         -> Mut List<T> holds proj(it) => it: Mut, labels {\n    \
         let out = Mut List<T>()\n    for x in it {\n        add(out, x)\n    }\n    return out\n}\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let words = List(\"ann\", \"bo\")\n    \
         let tags = List(\"x\")\n    \
         let kept = keep_all(iter(words), tags)\n    \
         eat(tags)\n    \
         println(\"${size(kept)}\")\n    \
         let words2 = List(\"ann\", \"bo\")\n    \
         let tags2 = List(\"x\")\n    \
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
             its fate, and `words2` was moved at line 21 after the binding"
        ),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("1 error"), "stderr: {stderr}");
}

// [fs-surface] [type-checked] The filesystem's failures carry the library's one
// obligation: a member answers `Err Checked<FsError>`, so a result that is never
// looked at does not compile, and the ways out are the generic ones — `ignore`,
// `detach`, or narrowing to `Ok`. Against *real* std, because what is being
// tested is the wiring between two std modules rather than the linear rule
// itself (that is `linear_group_tests`').
#[test]
fn an_fs_failure_must_be_acknowledged() {
    let dir = src_dir("fs_checked");
    let program = |body: &str| {
        format!(
            "import std.fs\nimport std.fs.path\nimport std.fs.host\nimport std.stream.host\n\n\
             fn main() [use] -> None {{\n    use StdConsole()\n    \
             use HostRawStreams()\n    use DefaultStreams()\n    \
             use HostRawFs()\n    use DefaultFs()\n    \
             let text = read_to_str(path(\"nope.txt\"))\n    \
             when text {{\n        is Ok {{ println(text) }}\n        \
             is Err {{ {body} }}\n    }}\n}}\n"
        )
    };

    // Looking at the `Err` arm and doing nothing with it leaves the
    // obligation. The `Ok` arm consumes `text` and the `Err` arm drops it, so
    // the surviving fate check reports it as consumed on some paths but not
    // others (the frame-drop message moved to the IR analysis [ir-analyze],
    // but this `when` leaves `text` owned on one path, which the fate-conflict
    // check — kept in the AST checker — catches first).
    fs::write(dir.join("main.sv"), program("println(\"failed\")")).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("`text` owns a linear value that is consumed on some paths but not others"),
        "stderr: {stderr}"
    );

    // `ignore` says the not-looking was deliberate; `detach` hands back the bare
    // `FsError`, which `to_str` renders — the shape that replaced a read of the
    // old wrapper.
    for body in ["ignore(text)", "println(to_str(detach(text)))"] {
        fs::write(dir.join("main.sv"), program(body)).unwrap();
        let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{body}: {stderr}");
        assert!(stderr.contains("no errors"), "{body}: {stderr}");
    }
}

// [proj-infer-fn-type] A fn-typed slot whose return is a bare type parameter
// lends its named kept positions by inference (2026-09-26), so writing the
// clause there says nothing — and an annotation that changes nothing reads as
// evidence that something needs saying, the same reason a no-op `@place`
// selector warns [fn-overload-at]. A **warning**, because the program is
// correct and the clause is merely spare; and silent where the return is
// concrete, since a named type says for itself whether it borrows.
#[test]
fn a_spare_lend_on_an_opaque_slot_warns_once() {
    let dir = src_dir("spare_lend");
    let total = |slot: &str| {
        format!(
            "fn total<C, It, T>(c: C, ?iter: {slot}, ?Next<It, T>) -> Int\n=> c {{\n    \
             let n = 0\n    for _ in iter(c) {{\n        n = n + 1\n    }}\n    return n\n}}\n\n\
             fn main() [use] -> None {{\n    use StdConsole()\n    \
             println(\"${{total(List(\"a\", \"b\"))}}\")\n}}\n"
        )
    };

    // Written where it is inferred: one diagnostic, naming the clause.
    fs::write(
        dir.join("main.sv"),
        total("(c: C) -> Mut It holds proj(c)"),
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a spare clause is legal: {stderr}");
    assert_eq!(
        stderr.matches("says what the signature already infers").count(),
        1,
        "one clause, one diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`holds proj(c)`") && stderr.contains("1 warning"),
        "it names the clause it wants dropped: {stderr}"
    );

    // Dropping it is what the warning asks for, and then there is nothing to say.
    fs::write(dir.join("main.sv"), total("(c: C) -> Mut It")).unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no errors"), "{stderr}");
    assert!(
        !stderr.contains("already infers"),
        "nothing written, nothing to warn about: {stderr}"
    );

    // A **concrete** return still needs the clause, so it stays silent.
    fs::write(
        dir.join("main.sv"),
        "fn walk<T>(xs: List<T>, ?iter: (c: List<T>) -> Mut ListNext<T> holds proj(c)) -> Int\n\
         => xs {\n    let n = 0\n    for _ in iter(xs) {\n        n = n + 1\n    }\n    \
         return n\n}\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    \
         println(\"${walk(List(1, 2, 3))}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no errors"), "{stderr}");
    assert!(
        !stderr.contains("already infers"),
        "a concrete return infers nothing, so the clause is load-bearing: {stderr}"
    );
}

/// [actor-route-stub] `use any g` (user decision R7): binds `any E` for the `E`
/// the group handle names, through a route the IR derives — so the policy it
/// depends on (`RouteSelector<E>`) has to be in scope, and the face is `any`.
/// A policy's waits are its own `config` parameter.
#[test]
fn route_binds_any_and_needs_a_selector() {
    let dir = src_dir("route_ok");
    let program = |tail: &str| {
        format!(
            "import std.net\n\n\
             actor effect Ping {{\n    send fn ping(k: Key Str, out: Reply<Str>) => !k, !out\n}}\n\n\
             fn one() [any Ping] -> Str {{\n    return waitfor out: Reply<Str> {{ ping(\"a\", out) }}\n}}\n\n\
             fn strong() [Ping] -> Str {{\n    return waitfor out: Reply<Str> {{ ping(\"a\", out) }}\n}}\n\n\
             fn main(g: eff[ActorGroup<Ping>]) [use]\n=> !g {{\n{tail}}}\n"
        )
    };
    let analyze = |tail: &str| {
        fs::write(dir.join("main.sv"), program(tail)).unwrap();
        let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
        String::from_utf8_lossy(&out.stderr).to_string()
    };
    let stderr = analyze("    use Sharded<Ping>()\n    use any g\n    let _a = one()\n");
    assert!(stderr.contains("no errors"), "{stderr}");

    // The policy's waits, overridden where it is bound.
    let stderr = analyze("    use Sharded<Ping>(config = RouteConfig {})\n    use any g\n    let _a = one()\n");
    assert!(stderr.contains("no errors"), "{stderr}");

    // The route binds `any Ping`: a bare `[Ping]` is refused [effect-any].
    let stderr = analyze("    use Sharded<Ping>()\n    use any g\n    let a = strong()\n");
    assert!(stderr.contains("`strong` declares `[Ping]`") && stderr.contains("many instances"), "{stderr}");

    // No policy in scope: the route's dependency is missing.
    let stderr = analyze("    use any g\n    let _a = one()\n");
    assert!(
        stderr.contains("`RouteSelector<Ping>` policy") && stderr.contains("use Sharded<Ping>()"),
        "the route depends on `RouteSelector<Ping>`: {stderr}"
    );
}

/// [actor-route-stub] The refusals, and what is no longer one (user decision R7):
/// `use any` of something that is not a group handle; a protocol member
/// marking two `Key` parameters, refused at the effect whether or not
/// anything routes it; and a `use any h` in a module that never spells the
/// protocol, which compiles now that the route is derived from `h`'s type.
#[test]
fn route_refusals_name_the_fix() {
    let dir = src_dir("route_bad");
    fs::write(
        dir.join("main.sv"),
        "import std.net\n\nfn main() [use] {\n    let x = 3\n    use LeastLoaded<NodeGroup>(true)\n    use any x\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`use any` binds a group handle") && stderr.contains("this is `Int`"),
        "{stderr}"
    );

    // Two `Key`s, and nothing routes `Two`.
    fs::write(
        dir.join("main.sv"),
        "import std.net\n\nactor effect Two {\n    send fn go(a: Key Str, b: Key Int) => !a, !b\n}\n\nfn main() {\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("`Two.go` marks 2 parameters `Key`"), "{stderr}");

    // The group's type is spelled only in `lib`: `main` routes it all the same.
    fs::write(
        dir.join("main.sv"),
        "import std.net\nimport lib.Ping\nimport lib.some_group\n\n\
         fn main() [use, spawn] {\n    use LeastLoaded<Ping>(true)\n    let h = some_group()\n    use any h\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("lib.sv"),
        "import std.net\n\nexport actor effect Ping {\n    send fn ping(out: Reply<Str>) => !out\n}\n\n\
         export fn some_group() [spawn] -> eff[ActorGroup<Ping>] {\n    \
         return actor_group<Ping>(spawn StaticNodeGroup(\"x\", List(NodeEndpoint { host: \"a\", port: 1 })) with MemTransport(NodeEndpoint { host: \"a\", port: 1 }, (spawn MemNetwork() on pool(1))) on pool(1))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no errors"), "{stderr}");
}

/// [type-identity] [obligation-by] A clause stamps the declaration it is
/// written on. With another module's `struct Token` in scope through a
/// whole-module import, `beta`'s own union `Token` was stamped as that struct
/// — the comptime expansion looked the name up program-wide, struct first —
/// and reported "`Str | Int` is not a `Union`". It now reads each file's
/// names through the resolver's scope (PIPELINE.md step 6).
#[test]
fn a_clause_stamps_its_own_declaration_beside_a_same_named_struct() {
    let dir = src_dir("comptime_token_clash");
    fs::write(dir.join("alpha.sv"), "export struct Token {\n    a: Int\n}\n").unwrap();
    fs::write(
        dir.join("beta.sv"),
        "import alpha\n\nexport type Token = Str | Int : ToStr<self> by auto\n\n\
         export fn show(t: Token) -> Str {\n    return to_str(t)\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(": 0 errors") || stderr.contains("no errors"), "{stderr}");
}


/// [manifest-src] The project of these tests: `[build] src = "src"`, with
/// `src/main.sv` importing `app.util` — which only resolves when `src/` is
/// the source root.
fn src_root_project(test: &str) -> PathBuf {
    let dir = src_dir(test).canonicalize().unwrap();
    fs::create_dir_all(dir.join("src/app")).unwrap();
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"src\"\n").unwrap();
    fs::write(dir.join("src/main.sv"), "import app.util\n\nfn main() [] -> None {\n    return f()\n}\n").unwrap();
    fs::write(dir.join("src/app/util.sv"), "export fn f() [] -> None {\n    return None\n}\n").unwrap();
    dir
}

/// [manifest-src] `--src` is the source root: one strictly inside the found
/// project's root would re-root `app/util.sv` as `util` (and break the
/// project's own `import app.util`), so every command taking `--src` refuses
/// it with one error naming the root to pass.
#[test]
fn a_src_inside_the_projects_source_root_is_refused() {
    let dir = src_root_project("src_inside_root");
    let inner = dir.join("src/app");
    let root = dir.join("src");
    let expected = format!(
        "error: `{}` is inside the project's source root `{}` (from `{}`): pass `--src {}`, or \
         no `--src`; scope a command to one module with `--module` where it has one [manifest-src]",
        inner.display(),
        root.display(),
        dir.join("salvo.toml").display(),
        root.display()
    );
    let inner = inner.to_str().unwrap();
    let target = dir.join(".out");
    let target = target.to_str().unwrap();
    for args in [
        &["analyze", "--src", inner][..],
        &["test", "--src", inner],
        &["ir", "--src", inner],
        &["facts", "--src", inner],
        &["run", "--src", inner],
        &["compile", "--backend", "rust", "--src", inner, "--target", target],
        &["build", "--src", inner],
        &["platform", "generate", "--backend", "rust", "--src", inner],
    ] {
        let out = salvo(args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{args:?}: {stderr}");
        assert_eq!(stderr.trim(), expected, "{args:?}");
    }
}

/// [manifest-src] The mirror case: a `--src` that contains the found
/// project's root without being it (the manifest's own directory, with
/// `[build] src = "src"`) would re-root `src/main.sv` as `src.main`; it is
/// refused the same way. A relative `--src` is answered in relative paths.
#[test]
fn a_src_around_the_projects_source_root_is_refused() {
    let dir = src_root_project("src_around_root");
    let out = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
        .current_dir(dir.parent().unwrap())
        .args(["analyze", "--src", "analyze_src_around_root"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert_eq!(
        stderr.trim(),
        "error: `analyze_src_around_root` contains the project's source root \
         `analyze_src_around_root/src` (from `analyze_src_around_root/salvo.toml`): pass \
         `--src analyze_src_around_root/src`, or no `--src`; scope a command to one module \
         with `--module` where it has one [manifest-src]"
    );
}

/// [manifest-src] What still works: `--src` equal to the root (spelled with
/// a `..`, compared canonically), no `--src` at all, a `--src` beside the
/// root (neither inside nor around it), and any `--src` with no manifest
/// found.
#[test]
fn a_src_that_is_the_root_or_has_no_manifest_still_works() {
    let dir = src_root_project("src_is_root");
    let spelled = dir.join("src/app/..");
    let out = salvo(&["analyze", "--src", spelled.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && stderr.contains("(2 user,"), "{stderr}");
    let out = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
        .current_dir(&dir)
        .arg("analyze")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && stderr.contains("(2 user,"), "{stderr}");
    fs::create_dir_all(dir.join("other")).unwrap();
    fs::write(dir.join("other/lone.sv"), CLEAN).unwrap();
    let out = salvo(&["analyze", "--src", dir.join("other").to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && stderr.contains("(1 user,"), "beside the root: {stderr}");

    let bare = src_dir("src_no_manifest");
    fs::create_dir_all(bare.join("app")).unwrap();
    fs::write(bare.join("app/main.sv"), CLEAN).unwrap();
    let out = salvo(&["analyze", "--src", bare.join("app").to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && stderr.contains("no errors"), "{stderr}");
}

/// [manifest-src] `--src lib/std` finds `lib/salvo.toml` and is refused at
/// once, naming `lib`. Before the rule it loaded std twice (the tree's files
/// as `core.*`, `time`, … beside the embedded `std.*`) and did not finish
/// (ROADMAP 0t; COMPLETED.md's 2026-10-14 implicit-resolution entry); the
/// deadline guards against that.
#[test]
fn src_lib_std_is_refused_promptly() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
        .current_dir(&repo)
        .args(["analyze", "--src", "lib/std"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("`salvo analyze --src lib/std` did not finish within 60s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains(
            "`lib/std` is inside the project's source root `lib` (from `lib/salvo.toml`): pass `--src lib`"
        ),
        "{stderr}"
    );
}

/// [op-unique-shape] A module's own `to_str(List<T>)` beside std's is refused
/// where it is declared (user decision O1-D, 2026-10-27): `to_str` on `List` is
/// interpolation's function, declared only in `List`'s file, so no module can
/// put a second one beside it — whether or not anything uses it. Before, it
/// sat beside std's and every interpolation and implicit of a list was an
/// ambiguity (PIPELINE.md §II.13 D2, dissolved). The error names the type's
/// file and the remedy.
#[test]
fn a_same_signature_overload_beside_std_is_refused_at_its_declaration() {
    let dir = src_dir("implicit_same_signature");
    fs::write(
        dir.join("main.sv"),
        "fn to_str<T>(list: List<T>, ?to_str: (x: T) -> Str) [] -> Str => list { return \"mine\" }\n\
         \n\
         fn render<T>(x: T, ?to_str: (x: T) -> Str) [] -> Str => x { return to_str(x) }\n\
         \n\
         fn main() [use] {\n    use StdConsole()\n    println(\"${List(1, 2)}\")\n    println(render(List(1, 2)))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("`to_str(List<T>)` has the shape of `to_str` on `List`")
            && stderr.contains("(module `std.core.list`)")
            && stderr.contains("Give this one another name and pass it explicitly")
            && stderr.contains("(`to_str = by_name`) --> main.sv:1:4"),
        "{stderr}"
    );
    // [op-unique-shape] The refused declaration is a candidate nowhere, so its
    // uses report nothing more: one mistake, one diagnostic (ROADMAP 0ag,
    // closed in B3.3c). Before, the interpolation and `render`'s implicit
    // each reported the ambiguity it caused.
    assert!(stderr.contains("1 error,") && !stderr.contains("ambiguous"), "{stderr}");
}

/// [implicit-recursive] The winner's own implicits are filled after it is
/// chosen, and a failure there is an error naming the chain — no other
/// candidate stands in (user decision 2026-10-14). A list nested past the
/// depth cap is that error: std's `List` overload wins at every level, and the
/// cap is the error. (Before 2026-10-14 a module's catch-all `to_str<T>` made
/// the top fall back and print `?`; a catch-all is now refused where it is
/// declared, [op-unique-shape], so it cannot stand in either.) The plain chain
/// names each link down to the type with no text form.
#[test]
fn an_unfillable_winner_is_an_error_naming_the_chain() {
    // [op-unique-shape] The catch-all itself: a `to_str` on a type parameter has
    // no type's file to live in, so it is refused at its declaration.
    let dir = src_dir("implicit_unfillable_catch_all");
    fs::write(
        dir.join("main.sv"),
        "fn to_str<T>(x: T) [] -> Str => x { return \"?\" }\n\
         \n\
         fn main() [use] {\n    use StdConsole()\n    println(\"${List(1)}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("`to_str(T)` has the shape of `to_str` on the type parameter `T`, which has no declaring file")
            && stderr.contains("(`to_str = by_name`) --> main.sv:1:4"),
        "{stderr}"
    );

    let dir = src_dir("implicit_unfillable_winner");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] {\n    use StdConsole()\n    \
         let deep = List(List(List(List(List(List(List(List(List(List(List(1)))))))))))\n    \
         println(\"${deep}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("` for `to_str` cannot be completed: it nests more than 8 levels deep (the `to_str` for `?to_str: (NonEmpty List<NonEmpty List<Int>>) -> Str` needs implicits of its own); chain: ")
            && stderr.contains(" → … 6 more … → `NonEmpty List<NonEmpty List<Int>>`; pass `to_str = …` explicitly"),
        "{stderr}"
    );

    let dir = src_dir("implicit_unfillable_chain");
    fs::write(
        dir.join("main.sv"),
        "struct Foo { n: Int }\n\
         \n\
         fn main() [use] {\n    use StdConsole()\n    println(\"${List(List(Foo { n: 1 }))}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("`?to_str: (NonEmpty List<Foo>) -> Str` for `to_str` cannot be completed: it needs `?to_str: (Foo) -> Str`, and no `to_str` fits (the `to_str` in scope is `(Int) -> Str`, and the position needs `(Foo) -> Str`); pass `to_str = …` explicitly"),
        "{stderr}"
    );
    // Two links: the chain is the headline, and is not repeated.
    assert!(!stderr.contains("chain:"), "{stderr}");

    // A longer chain lists the wanted types between the two ends.
    let dir = src_dir("implicit_unfillable_chain_listed");
    fs::write(
        dir.join("main.sv"),
        "struct Foo { n: Int }\n\
         \n\
         fn main() [use] {\n    use StdConsole()\n    println(\"${List(List(List(List(Foo { n: 1 }))))}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("it needs `?to_str: (Foo) -> Str`, and no `to_str` fits (")
            && stderr.contains("; chain: `NonEmpty List<NonEmpty List<NonEmpty List<Foo>>>` → `NonEmpty List<NonEmpty List<Foo>>` → `NonEmpty List<Foo>` → `Foo`; pass `to_str = …` explicitly"),
        "{stderr}"
    );

    // A candidate that needs itself at the same type is a cycle.
    let dir = src_dir("implicit_unfillable_cycle");
    fs::write(
        dir.join("main.sv"),
        "struct Foo { n: Int }\n\
         \n\
         fn show(x: Foo, ?show: (Foo) -> Str) [] -> Str => x { return \"foo\" }\n\
         \n\
         fn shout(x: Foo, ?show: (Foo) -> Str) [] -> Str => x { return show(x) }\n\
         \n\
         fn main() [use] {\n    use StdConsole()\n    println(shout(Foo { n: 1 }))\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("`?show: (Foo) -> Str` for `shout` cannot be completed: the `show` for `?show: (Foo) -> Str` needs itself, a cycle; pass `show = …` explicitly"),
        "{stderr}"
    );
}

/// [implicit-recursive] The error for a chain that cannot be completed stays
/// a few types long however deep the chain went: before 2026-10-14 every
/// level repeated the whole phrase of the level below, and the 11-deep
/// case reached the depth cap in a 3 KB line. Now the position written and
/// the one that failed are named once, and the chain between is elided in the
/// middle: at most four of its types are printed. (The program had a
/// catch-all `to_str<T>` beside it, refused since [op-unique-shape]; the cap
/// needs none.)
#[test]
fn an_unfillable_chain_error_stays_short_at_the_depth_cap() {
    let dir = src_dir("implicit_unfillable_bounded");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] {\n    use StdConsole()\n    \
         let deep = List(List(List(List(List(List(List(List(List(List(List(1)))))))))))\n    \
         println(\"${deep}\")\n}\n",
    )
    .unwrap();
    let out = salvo(&["analyze", "--src", dir.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    let line = stderr.lines().find(|l| l.starts_with("error:")).unwrap_or_else(|| panic!("{stderr}"));
    // The outermost wanted type, printed once in full: the bound is a few
    // times its length, not the depth times it.
    let outer = "(NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<NonEmpty List<Int>>>>>>>>>>) -> Str";
    assert!(line.contains(&format!("`?to_str: {outer}` for `to_str` cannot be completed")), "{line}");
    assert_eq!(line.matches(outer).count(), 1, "{line}");
    assert!(line.len() < 5 * outer.len(), "{} bytes:\n{line}", line.len());
    assert_eq!(line.matches(" → ").count(), 3, "{line}");
    assert!(line.contains("… 6 more …"), "{line}");
    assert_eq!(line.matches("needs implicits of its own").count(), 1, "{line}");
}

/// [implicit-recursive] The user's doubled-std recipe — a copy of `lib/std`
/// with its own `std = true` manifest inside it — finishes promptly. Before
/// 2026-10-14 an implicit tried every candidate's own implicits before
/// choosing, and the old route to a std loaded twice (`--src lib/std`, before
/// [manifest-src] refused it) never finished: each nested `to_str`/`eq`/`hash`
/// had two candidates per level. Now only the winner's are filled, so the
/// width is one by construction.
#[test]
fn a_doubled_std_analysis_finishes_promptly() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let dir = src_dir("doubled_std");
    let copy = dir.join("std");
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if entry.file_name() != "platform" {
                    copy_tree(&path, &to.join(entry.file_name()));
                }
            } else {
                fs::copy(&path, to.join(entry.file_name())).unwrap();
            }
        }
    }
    copy_tree(&repo.join("lib/std"), &copy);
    fs::write(copy.join("salvo.toml"), "[project]\nname = \"std\"\nversion = \"0.1.0\"\nstd = true\n").unwrap();
    // The diagnostics go to a file: a std checked against another std reports
    // hundreds of errors, more than a pipe holds while nothing reads it.
    let log = dir.join("stderr.txt");
    let mut child = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
        .args(["analyze", "--src", copy.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(fs::File::create(&log).unwrap())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("`salvo analyze` of a doubled std did not finish within 60s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let stderr = fs::read_to_string(&log).unwrap();
    assert!(stderr.contains("analyzed "), "{stderr}");
}
