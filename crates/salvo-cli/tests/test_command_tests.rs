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
        // [kt-std-library] The compiled std is cached in the test target.
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
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
    // A wall-clock limit: std's tests now drive the runtime's own scheduler,
    // and a scheduler bug is a hang — which has to fail the suite rather than
    // stall it (AGENTS.md: watch the clock).
    let child = Command::new(env!("CARGO_BIN_EXE_salvo"))
        // [kt-std-library] The compiled std is cached in the test target.
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
        .current_dir(&repo)
        .args(["test", "--src", "std", "--target"])
        .arg(&target)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run salvo");
    let out = wait_with_limit(child, 240);
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

/// [test-recover] A test that traps — a failed `assert!` [assert-trap], a
/// subscript out of range — is **that test's failure**, not the end of the run:
/// the generated harness catches it and the report names it like any other
/// failure (user decision 2026-09-23, A-5, revised the same day to catch in the
/// harness rather than restart the process).
#[test]
fn a_trapping_test_fails_and_the_run_continues() {
    let Some(stamp) = e2e_stamp("trap_recovery", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("trap_recovery");
    fs::write(dir.join("calc.sv"), CALC).unwrap();
    fs::write(
        dir.join("calc.test.sv"),
        "test \"before\" {\n    expect_eq(double(2), 4)\n}\n\n\
         test \"traps\" {\n    let n = double(3)\n    \
         assert!(n > 100, \"n should exceed 100, was ${n}\")\n}\n\n\
         test \"after\" {\n    expect_eq(double(5), 10)\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["test", "--src", "."]);
    let stdout = normalize_ms(&String::from_utf8_lossy(&out.stdout));
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The trap's own words, under the test that caused it — and reported as a
    // failure, in the one process.
    assert_eq!(
        stdout,
        "test calc :: before ... ok (N ms)\n\
         test calc :: traps ... FAILED\n\
         \x20   salvo: n should exceed 100, was 6 at calc.test:7:5\n\
         test calc :: after ... ok (N ms)\n\
         \n\
         3 tests: 2 passed, 1 failed\n",
        "stderr: {stderr}"
    );
    // Nothing died, so nothing was re-run and no host trace reached the user.
    assert!(!stderr.contains("re-running"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(!out.status.success());
    stamp.verified();
}

/// [test-filter] `salvo test --clean-target both` deletes the generated harness
/// after the report, exactly as `run` deletes a program's sources (user decision
/// 2026-09-26). The default stays `before`, so a failure can be read.
#[test]
fn test_honours_clean_target() {
    let Some(__stamp) = e2e_stamp("test_clean_target", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("clean_target");
    fs::write(dir.join("m.sv"), "export fn two() -> Int {\n    return 2\n}\n").unwrap();
    fs::write(
        dir.join("m.test.sv"),
        "test \"two is two\" {\n    expect(two() == 2, \"two\")\n}\n",
    )
    .unwrap();
    let target = dir.join(".out");

    // The default leaves the harness in place.
    let out = salvo_in(
        &dir,
        &["test", "--src", ".", "--target", ".out"],
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(target.exists(), "the default keeps the generated harness");

    // `both` deletes it.
    let out = salvo_in(
        &dir,
        &["test", "--src", ".", "--target", ".out", "--clean-target", "both"],
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!target.exists(), "`both` should delete the target after the report");
    __stamp.verified();
}

/// Waits for [child], killing it and failing after [secs] seconds. Both pipes
/// are drained on threads of their own, so a chatty child cannot fill one and
/// block while the limit is being watched.
fn wait_with_limit(mut child: std::process::Child, secs: u64) -> std::process::Output {
    use std::io::Read;
    let mut out_pipe = child.stdout.take().expect("piped stdout");
    let mut err_pipe = child.stderr.take().expect("piped stderr");
    let out_thread = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out_pipe.read_to_end(&mut b);
        b
    });
    let err_thread = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err_pipe.read_to_end(&mut b);
        b
    });
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("failed to poll salvo") {
            break status;
        }
        if start.elapsed() > std::time::Duration::from_secs(secs) {
            let _ = child.kill();
            let _ = child.wait();
            let stdout = out_thread.join().unwrap_or_default();
            panic!(
                "`salvo test --src std` ran past {secs}s and was killed — a hang\nstdout: {}",
                String::from_utf8_lossy(&stdout)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    std::process::Output {
        status,
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
    }
}

/// [test-actor] [test-kind] Actor tests run on the virtual runtime, in a
/// program of their own after the plain tests: a timer fires without the wait,
/// a wait nothing can answer is the deadlock report (the test's death, and the
/// rest re-run in a fresh process), and the seed is what the declaration says.
#[test]
fn actor_tests_run_on_the_virtual_runtime() {
    let Some(stamp) = e2e_stamp("actor_tests", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("actor_tests");
    fs::write(dir.join("calc.sv"), CALC).unwrap();
    fs::write(
        dir.join("calc.test.sv"),
        r#"import time

actor effect Hoard {
    send fn ask(out: Reply<Int>) => !out
}

// Keeps every reply, answering none.
handler Hoarding() of Hoard {
    mailbox { capacity: 2 }
    kept: Mut List<Reply<Int>> = mut_list_of()

    send fn ask(out: Reply<Int>) {
        add(kept, out)
    }
}

test actor "a day passes at once" {
    let timer = spawn DefaultTimer() on pool(1)
    use DefaultTicker()
    let fired = waitfor answer: Reply<Fired> {
        timer.after(hours(24), answer)
    }
    expect_eq(fired.at.nanos, hours(24).nanos)
}

test "a plain test" {
    expect_eq(double(2), 4)
}

test actor(seed: 3) "nobody answers" {
    let hoard = spawn Hoarding() on pool(1)
    let n = waitfor answer: Reply<Int> {
        hoard.ask(answer)
    }
    expect_eq(n, 1)
}

test actor "after the death" {
    use DefaultTicker()
    expect_eq(tick().nanos, 0L)
}
"#,
    )
    .unwrap();
    let out = salvo_in(&dir, &["test", "--src", "."]);
    let stdout = normalize_ms(&String::from_utf8_lossy(&out.stdout));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("test calc :: a plain test ... ok"), "{stdout}\n{stderr}");
    assert!(stdout.contains("test calc :: a day passes at once ... ok"), "{stdout}\n{stderr}");
    assert!(stdout.contains("test calc :: after the death ... ok"), "{stdout}\n{stderr}");
    // The plain test runs first, in the threaded program.
    assert!(
        stdout.find("a plain test").unwrap() < stdout.find("a day passes").unwrap(),
        "{stdout}"
    );
    assert!(stdout.contains("salvo: deadlock"), "{stdout}\n{stderr}");
    assert!(!out.status.success());
    stamp.verified();
}
