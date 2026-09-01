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
