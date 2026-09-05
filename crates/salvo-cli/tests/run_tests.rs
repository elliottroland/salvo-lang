//! Integration tests for `salvo run` [cli-run]: compile with a backend and
//! run the result through that backend's toolchain.
//!
//! The command's exit code is the *program's*, so `salvo run` stands in for
//! running the binary. Tests that need a toolchain (`kotlinc`, `rustc`) skip
//! gracefully when it is missing; the argument-validation tests need none,
//! since validation happens before anything is built.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh working directory under the target tmp dir, with `main.sv` in it.
/// Everything a `run` test needs lives here: sources, and (unless the test
/// says otherwise) the default target the command creates.
fn work_dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("run_{test}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `salvo` with the working directory set, so relative paths and the
/// default `--target` resolve inside the test's own sandbox.
fn salvo_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_salvo"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run salvo")
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("-version").output().is_ok()
}

/// Prints three lines, one per branch of a subject-less `when`
/// [when-condition] — enough to prove the program really ran.
const HELLO: &str = r#"
fn classify(n: Int) -> [] Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

fn main() [use] -> [] None {
    use StdOutConsole
    println(classify(-5))
    println(classify(0))
    println(classify(7))
}
"#;

const HELLO_STDOUT: &str = "negative\nzero\npositive\n";

// ===== running, on both backends =====

/// The whole point: one command from `.sv` source to program output. Asserted
/// per backend with the *same* expected stdout, which is what parity means
/// here.
#[test]
fn run_compiles_and_runs_with_each_backend() {
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("hello_{backend}"));
        fs::write(dir.join("main.sv"), HELLO).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed, stderr: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            HELLO_STDOUT,
            "{backend} stdout (stderr: {stderr})"
        );
    }
}

/// `--main` names the entry file and, on its own, implies its directory as
/// the source directory (user decision 2026-09-05), so the command works
/// from anywhere.
#[test]
fn main_flag_implies_the_source_directory() {
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("main_flag");
    let sources = dir.join("program");
    fs::create_dir_all(&sources).unwrap();
    fs::write(sources.join("main.sv"), HELLO).unwrap();
    // Run from `dir`, pointing at a file one level down.
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--main", "program/main.sv"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), HELLO_STDOUT);
}

/// Two entry points in one directory: `--main` is how you say which.
/// Historically this was the Rust backend's blind spot — the crate root is
/// the `main`-declaring module [rs-crate], and the emitter picked the first
/// one it found regardless of the choice, so the build produced a module
/// without the `mod` declarations and rustc rejected it.
#[test]
fn main_flag_selects_among_several_entry_points() {
    let dir = work_dir("several_mains");
    for (file, text) in [("main.sv", "from main.sv"), ("other.sv", "from other.sv")] {
        fs::write(
            dir.join(file),
            format!(
                "fn main() [use] -> [] None {{\n    use StdOutConsole\n    \
                 println(\"{text}\")\n}}\n"
            ),
        )
        .unwrap();
    }
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        for (file, expected) in [
            ("main.sv", "from main.sv\n"),
            ("other.sv", "from other.sv\n"),
        ] {
            let out = salvo_in(&dir, &["run", "--backend", backend, "--main", file]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                out.status.success(),
                "{backend} --main {file} failed, stderr: {stderr}"
            );
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                expected,
                "{backend} --main {file} (stderr: {stderr})"
            );
        }
    }
}

/// The command's exit code is the program's, so `salvo run` can replace
/// running the binary in a script. A crashing program is a *nonzero* exit,
/// not a compiler error — the two backends' runtimes pick different codes
/// (rustc panics with 101, the JVM exits 1), so only "nonzero" is asserted.
#[test]
fn the_programs_exit_code_is_the_commands() {
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("exit_code");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] -> [] None {\n    use StdOutConsole\n    \
         let numbers = [1, 2, 3]\n    let i = 10\n    \
         println(\"value ${numbers[i]}\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "."]);
    assert!(
        !out.status.success(),
        "expected the panic to propagate; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("index out of bounds"),
        "the program's own stderr should reach the terminal: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ===== --clean-target =====

/// `both` (the default) leaves nothing behind; `before` keeps the output for
/// inspection. Both clear the target first, so a run never sees the previous
/// run's files.
#[test]
fn clean_target_decides_what_survives_the_run() {
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("clean_target");
    fs::write(dir.join("main.sv"), HELLO).unwrap();

    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "."]);
    assert!(out.status.success());
    assert!(
        !dir.join(".salvo_tmp_run").exists(),
        "the default `both` should delete the target"
    );

    let out = salvo_in(
        &dir,
        &["run", "--backend", "rust", "--src", ".", "--clean-target", "before"],
    );
    assert!(out.status.success());
    assert!(
        dir.join(".salvo_tmp_run").join("main.rs").is_file(),
        "`before` should leave the output in place"
    );

    // A stale file in the target is gone after the next run: `before`
    // clears it too.
    let stale = dir.join(".salvo_tmp_run").join("stale.rs");
    fs::write(&stale, "// left over\n").unwrap();
    let out = salvo_in(
        &dir,
        &["run", "--backend", "rust", "--src", ".", "--clean-target", "before"],
    );
    assert!(out.status.success());
    assert!(!stale.exists(), "the target is cleared before the build");
}

// ===== argument validation (no toolchain needed) =====

/// [cli-run] The target is deleted before the build, so a target that
/// *contains* the sources would delete the program. Rejected before anything
/// runs.
#[test]
fn a_target_containing_the_sources_is_rejected() {
    let dir = work_dir("target_contains_src");
    let sources = dir.join("program");
    fs::create_dir_all(&sources).unwrap();
    fs::write(sources.join("main.sv"), HELLO).unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "program", "--target", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("contains the sources"), "stderr: {stderr}");
    assert!(
        sources.join("main.sv").is_file(),
        "the sources must still be there"
    );
}

#[test]
fn a_target_equal_to_the_source_directory_is_rejected() {
    let dir = work_dir("target_is_src");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", ".", "--target", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("is the source directory itself"),
        "stderr: {stderr}"
    );
    assert!(dir.join("main.sv").is_file(), "the source must still be there");
}

/// [cli-run] A *visible* target inside the sources would be read back on the
/// next build: emitted files carry the backend's native extension, which is
/// exactly what a hand-written companion file looks like
/// [backend-companion]. A dot-prefixed one is skipped by discovery
/// [mod-ignore], which is why the default target is `.salvo_tmp_run`.
#[test]
fn a_visible_target_inside_the_sources_is_rejected() {
    let dir = work_dir("target_inside_src");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", ".", "--target", "out"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("read the emitted files back as sources"),
        "stderr: {stderr}"
    );
    // The dot-prefixed spelling of the same nesting is accepted (the
    // default target is exactly this shape).
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", ".", "--target", ".out"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("read the emitted files back"),
        "a hidden nested target should be allowed: {stderr}"
    );
}

/// `--src` and `--main` are independent, each supplying a default for the
/// other (user decision 2026-09-05). Giving both is the only way to say
/// "compile this tree, start at this file" when the entry sits in a
/// *subdirectory* — which is also when it matters, because a shared module
/// one level up is out of reach for `--main` alone.
#[test]
fn src_and_main_together_allow_a_nested_entry_point() {
    let dir = work_dir("src_and_main");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(
        dir.join("helper.sv"),
        "fn label(n: Int) -> [] Str {\n    return when {\n        \
         n < 0 { \"neg\" }\n        else { \"nonneg\" }\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        bin.join("tool.sv"),
        "import helper.label\n\nfn main() [use] -> [] None {\n    \
         use StdOutConsole\n    println(\"nested ${label(3)}\")\n}\n",
    )
    .unwrap();

    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(
            &dir,
            &["run", "--backend", backend, "--src", ".", "--main", "bin/tool.sv"],
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} stderr: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "nested nonneg\n",
            "{backend} (stderr: {stderr})"
        );
    }

    // `--main` alone cannot express this layout: it would take `bin/` as the
    // source directory, leaving `helper` outside the compilation.
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--main", "bin/tool.sv"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unresolved import"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The entry file has to be one of the compiled sources, so a `--main`
/// outside `--src` is an error rather than a silently ignored flag.
#[test]
fn a_main_outside_the_source_directory_is_rejected() {
    let dir = work_dir("main_outside_src");
    let sources = dir.join("program");
    fs::create_dir_all(&sources).unwrap();
    fs::write(sources.join("main.sv"), HELLO).unwrap();
    fs::write(dir.join("stray.sv"), HELLO).unwrap();
    let out = salvo_in(
        &dir,
        &["run", "--backend", "rust", "--src", "program", "--main", "stray.sv"],
    );
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("is not inside"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// At least one of `--src` / `--main` is required — with neither there is
/// nothing to compile — and clap names both so the message says what to do.
#[test]
fn src_or_main_is_required() {
    let dir = work_dir("src_or_main");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--src") && stderr.contains("--main"),
        "both options should be named: {stderr}"
    );
}

/// `--backend` is required for `run`: there is no sensible default when the
/// answer decides which toolchain has to be installed.
#[test]
fn backend_is_required_and_validated() {
    let dir = work_dir("backend_required");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    let out = salvo_in(&dir, &["run", "--src", "."]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--backend"));

    let out = salvo_in(&dir, &["run", "--backend", "cobol", "--src", "."]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown backend `cobol`") && stderr.contains("kotlin, rust"),
        "stderr: {stderr}"
    );
}

/// A `--main` that declares no entry point, and a directory with none: both
/// are errors naming the file that was searched, since there is nothing to
/// run.
#[test]
fn a_missing_main_is_an_error() {
    let dir = work_dir("no_main");
    fs::write(
        dir.join("lib.sv"),
        "fn helper(n: Int) -> [] Int {\n    return n + 1\n}\n",
    )
    .unwrap();

    let out = salvo_in(&dir, &["run", "--backend", "rust", "--main", "lib.sv"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("declares no `main` function"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "."]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("nothing to run"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A define file carries native templates, not code, so it can never be the
/// entry point [backend-define-inline]. Caught by its double extension
/// rather than by failing to find `main`, so the message can say why.
#[test]
fn a_define_file_cannot_be_the_entry_point() {
    let dir = work_dir("define_entry");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    fs::write(dir.join("main.rust.sv"), "\n").unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--main", "main.rust.sv"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("backend define file"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A checker error stops the run, and the diagnostic is the compiler's own
/// (rendered once, with its location) rather than a wrapped restatement.
#[test]
fn a_check_error_stops_the_run() {
    let dir = work_dir("check_error");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] -> [] None {\n    use StdOutConsole\n    \
         let n: Int = \"not an int\"\n    println(\"${n}\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "."]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("expected `Int`, found `Str`") && stderr.contains("main.sv:3"),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("error: error:"),
        "the diagnostic should not be double-prefixed: {stderr}"
    );
}

/// [cli-run] The belt to the overlap check's braces: `--clean-target` never
/// deletes a directory holding `.sv` files, whatever the paths say.
#[test]
fn a_target_holding_sources_is_never_deleted() {
    let dir = work_dir("target_with_sources");
    let sources = dir.join("program");
    fs::create_dir_all(&sources).unwrap();
    fs::write(sources.join("main.sv"), HELLO).unwrap();
    // A sibling target that (wrongly) holds a source file. The paths do not
    // overlap, so only the deletion guard can catch this.
    let target = dir.join("precious");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("keep.sv"), "// not output\n").unwrap();

    let out = salvo_in(
        &dir,
        &["run", "--backend", "rust", "--src", "program", "--target", "precious"],
    );
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("refusing to delete"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(target.join("keep.sv").is_file(), "the file must survive");
}
