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
    assert!(!stderr.contains("the standard library does not check"), "std itself checks: {stderr}");
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

/// [fn-overload-at] [diag-structured] A redundant selector is a **warning**:
/// the program is correct, so it must still compile, and the warning has to
/// reach the CLI's report and its JSON — a severity the emission gate ignores.
/// (The warning itself is checker data: `overloads/selector.sv`; what stays
/// here is the exit status, the summary line and the JSON.)
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
         no `--src`; scope a command to one module with `--module` where it has one",
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
         with `--module` where it has one"
    );
}

/// [diag-id] Under `-v`, an error the project loader reports as a line of
/// text leads with its id, and without it no id shows: a malformed manifest
/// [manifest], and a `--src` inside the project's source root [manifest-src].
#[test]
fn a_manifest_error_shows_its_id_under_v() {
    let dir = src_dir("manifest_error_id").canonicalize().unwrap();
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"p\"\n\n[build]\nbackend = \"wasm\"\n").unwrap();
    fs::write(dir.join("main.sv"), CLEAN).unwrap();
    let src = dir.to_str().unwrap();
    let text = format!(
        "`{}`: `[build] backend` is `wasm`; it names a backend (`rust`, `kotlin`) or `*` for every backend",
        dir.join("salvo.toml").display()
    );
    let plain = salvo(&["analyze", "--src", src]);
    assert!(!plain.status.success());
    assert_eq!(String::from_utf8_lossy(&plain.stderr).trim(), format!("error: {text}"));
    let verbose = salvo(&["-v", "analyze", "--src", src]);
    assert!(!verbose.status.success());
    assert_eq!(
        String::from_utf8_lossy(&verbose.stderr).trim(),
        format!("error: [manifest:unknown-backend] {text}")
    );

    let dir = src_root_project("src_inside_root_id");
    let inner = dir.join("src/app");
    let out = salvo(&["-v", "analyze", "--src", inner.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.trim().starts_with("error: [manifest-src:overlaps-root] `"), "{stderr}");
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
