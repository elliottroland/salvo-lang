//! Integration tests for `salvo test` [test-run]: discovery, the report, and
//! std's own suite.
//!
//! The command compiles a generated harness and runs it with the backend's
//! toolchain, so the running tests need one (`rustc` by default, `kotlinc` for
//! the parity check) and skip gracefully without it. Discovery and the
//! refusals need none: they happen before anything is built.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh working directory under the target tmp dir.
fn work_dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("test_{test}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn salvo_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_salvo"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run salvo")
}

fn have(tool: &str) -> bool {
    salvo_testkit::tool(env!("CARGO_TARGET_TMPDIR"), tool).available
}

/// The stamp shape `run_tests.rs` uses, for the same reason: what decides the
/// outcome is the `salvo` binary, this test binary, and the toolchain.
fn e2e_stamp(test: &str, tools: &[&str]) -> Option<salvo_testkit::Stamp> {
    let mut parts: Vec<Vec<u8>> = vec![
        test.as_bytes().to_vec(),
        salvo_testkit::file_fingerprint(env!("CARGO_BIN_EXE_salvo")).into_bytes(),
        salvo_testkit::self_fingerprint().into_bytes(),
    ];
    for tool in tools {
        parts.push(
            salvo_testkit::tool(env!("CARGO_TARGET_TMPDIR"), tool)
                .version
                .into_bytes(),
        );
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    salvo_testkit::cached(env!("CARGO_TARGET_TMPDIR"), &format!("test {test}"), &refs)
}

/// A module and its annex: one passing test, one failing, and one reaching a
/// *private* declaration — which is the whole point of the companion
/// [test-visibility].
const CALC: &str = r#"
export fn double(n: Int) -> Int {
    return n * 2
}

fn secret(n: Int) -> Int {
    return n + 1
}
"#;

const CALC_TESTS: &str = r#"
test "doubling works" {
    expect_eq(double(3), 6)
}

test "the annex sees a private helper" {
    expect_eq(secret(1), 2)
}

test "a failure names both values" {
    expect_eq(double(2), 5)
}
"#;

fn calc_dir(name: &str) -> PathBuf {
    let dir = work_dir(name);
    fs::write(dir.join("calc.sv"), CALC).unwrap();
    fs::write(dir.join("calc.test.sv"), CALC_TESTS).unwrap();
    dir
}

// ===== discovery, which needs no toolchain =====

/// [test-report] `--list` enumerates without running: the id is the module a
/// program would `import`, then the name as written.
#[test]
fn list_enumerates_every_test() {
    let dir = calc_dir("list");
    let out = salvo_in(&dir, &["test", "--src", ".", "--list"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "calc :: doubling works\n\
         calc :: the annex sees a private helper\n\
         calc :: a failure names both values\n"
    );
}

/// [test-filter] The filter is a substring of the whole id.
#[test]
fn a_filter_selects_by_substring() {
    let dir = calc_dir("filter");
    let out = salvo_in(&dir, &["test", "--src", ".", "--list", "private"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "calc :: the annex sees a private helper\n"
    );
}

/// [test-file] An annex with no module to be the annex *of* is a mistake —
/// a renamed or deleted production file with its tests left behind.
#[test]
fn an_orphan_annex_is_refused() {
    let dir = work_dir("orphan");
    fs::write(dir.join("calc.test.sv"), CALC_TESTS).unwrap();
    let out = salvo_in(&dir, &["test", "--src", ".", "--list"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stdout: {:?}", out.stdout);
    assert!(
        stderr.contains("no module `calc` to test") && stderr.contains("[test-file]"),
        "{stderr}"
    );
}

/// [test-file] A `test` block in a production file names the annex as the fix
/// rather than being quietly compiled into the program.
#[test]
fn a_test_in_a_production_file_is_refused() {
    let dir = work_dir("in_production");
    fs::write(
        dir.join("calc.sv"),
        "export fn double(n: Int) -> Int {\n    return n * 2\n}\n\n\
         test \"nope\" {\n    expect(true, \"never runs\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["test", "--src", ".", "--list"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("calc.test.sv") && stderr.contains("[test-file]"),
        "{stderr}"
    );
}

/// [test-file] `compile` does not walk annexes at all, which is the whole of
/// how tests stay out of a production build: nothing to strip.
#[test]
fn a_production_build_ignores_the_annex() {
    let dir = calc_dir("production_build");
    let out = salvo_in(&dir, &["compile", "--backend", "rust", "--src", ".", "--target", "out"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        !stderr.contains("calc.test.sv"),
        "the annex took part in the build: {stderr}"
    );
    assert!(!dir.join("out").join("calc").join("test.rs").exists());
}

/// [test-decl] Two tests cannot share a name: the name is what identifies one
/// to the runner.
#[test]
fn duplicate_names_are_refused() {
    let dir = work_dir("duplicate");
    fs::write(dir.join("calc.sv"), CALC).unwrap();
    fs::write(
        dir.join("calc.test.sv"),
        "test \"same\" {\n    expect(true, \"a\")\n}\n\n\
         test \"same\" {\n    expect(true, \"b\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["test", "--src", ".", "--list"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("[test-unique]"), "{stderr}");
}

// ===== running =====

/// [test-report] The report: a pass with its milliseconds, a failure with its
/// message indented under it, and a closing count. Asserted with the timings
/// normalized away, since those are the one part that cannot be identical.
#[test]
fn the_report_reads_the_same_on_both_backends() {
    let Some(stamp) = e2e_stamp("report_parity", &["rustc", "kotlinc"]) else {
        return;
    };
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = calc_dir(&format!("report_{backend}"));
        let out = salvo_in(&dir, &["test", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = normalize_ms(&String::from_utf8_lossy(&out.stdout));
        assert_eq!(
            stdout,
            "test calc :: doubling works ... ok (N ms)\n\
             test calc :: the annex sees a private helper ... ok (N ms)\n\
             test calc :: a failure names both values ... FAILED\n\
             \x20   expected 5, got 4\n\
             \n\
             3 tests: 2 passed, 1 failed\n",
            "{backend} (stderr: {stderr})"
        );
        // A failing test fails the command: exit code is what a CI reads.
        assert!(!out.status.success(), "{backend} reported success");
    }
    stamp.verified();
}

/// A run where everything passes succeeds, and says so.
#[test]
fn a_green_run_succeeds() {
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = calc_dir("green");
    let out = salvo_in(&dir, &["test", "--src", ".", "doubling"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert_eq!(
        normalize_ms(&String::from_utf8_lossy(&out.stdout)),
        "test calc :: doubling works ... ok (N ms)\n\n1 test: 1 passed\n"
    );
}

/// [std-shadow] std's own suite, run the way a session runs it: the checkout's
/// `std/` replaces the copy compiled into the binary, so these tests exercise
/// the working tree.
#[test]
fn std_tests_pass() {
    let Some(stamp) = e2e_stamp("std_tests", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("std_suite");
    let out = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .current_dir(&repo)
        .args(["test", "--src", "std", "--target"])
        .arg(&target)
        .output()
        .expect("failed to run salvo");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("heap :: "), "no heap tests ran: {stdout}");
    assert!(
        !stdout.contains("FAILED") && !stdout.contains("failed"),
        "{stdout}"
    );
    stamp.verified();
}

/// Timings are the one part of the report that cannot be asserted.
fn normalize_ms(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(" ms)") {
        let head = &rest[..at];
        let cut = head.rfind('(').expect("a timing has its parenthesis");
        out.push_str(&head[..=cut]);
        out.push_str("N ms)");
        rest = &rest[at + " ms)".len()..];
    }
    out.push_str(rest);
    out
}
