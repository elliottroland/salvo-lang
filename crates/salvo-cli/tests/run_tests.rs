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

/// Whether a toolchain should be exercised. Probed once per tool per test
/// binary by `salvo-testkit`, which also owns the `SALVO_SKIP_E2E` gate.
fn have(tool: &str) -> bool {
    salvo_testkit::tool(env!("CARGO_TARGET_TMPDIR"), tool).available
}

/// A stamp for one toolchain test, so a re-run that cannot have a different
/// outcome does not pay for it again. Three things decide that outcome: the
/// `salvo` binary (the thing under test), this test binary (where the test's
/// inputs and assertions live, so editing a test invalidates it), and the
/// toolchains the command shells out to. `SALVO_E2E_FRESH=1` ignores stamps;
/// a stamp is written only after every assertion has passed.
///
/// Unlike the backends' stamps, these miss on *every* compiler rebuild —
/// the binary is an input — which is exactly right: they pay off in the
/// re-run and test-editing loops, not when the compiler itself changed.
fn e2e_stamp(test: &str, tools: &[&str]) -> Option<salvo_testkit::Stamp> {
    let mut parts: Vec<Vec<u8>> = vec![
        test.as_bytes().to_vec(),
        salvo_testkit::file_fingerprint(env!("CARGO_BIN_EXE_salvo")).into_bytes(),
        salvo_testkit::self_fingerprint().into_bytes(),
    ];
    for tool in tools {
        parts.push(
            salvo_testkit::tool(env!("CARGO_TARGET_TMPDIR"), tool).version.into_bytes(),
        );
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    salvo_testkit::cached(env!("CARGO_TARGET_TMPDIR"), &format!("cli {test}"), &refs)
}

/// Prints three lines, one per branch of a subject-less `when`
/// [when-condition] — enough to prove the program really ran.
const HELLO: &str = r#"
fn classify(n: Int) -> Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

fn main() [use] -> None {
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
    let Some(__stamp) = e2e_stamp("run_compiles_and_runs_with_each_backend", &["kotlinc", "rustc"]) else {
        return;
    };
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
    __stamp.verified();
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
    let Some(__stamp) = e2e_stamp("main_flag_selects_among_several_entry_points", &["kotlinc"]) else {
        return;
    };
    let dir = work_dir("several_mains");
    for (file, text) in [("main.sv", "from main.sv"), ("other.sv", "from other.sv")] {
        fs::write(
            dir.join(file),
            format!(
                "fn main() [use] -> None {{\n    use StdOutConsole\n    \
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
    __stamp.verified();
}

/// The command's exit code is the program's, so `salvo run` can replace
/// running the binary in a script. A crashing program is a *nonzero* exit,
/// not a compiler error — the two backends' runtimes pick different codes
/// (rustc panics with 101, the JVM exits 1), so only "nonzero" is asserted.
#[test]
fn the_programs_exit_code_is_the_commands() {
    let Some(__stamp) = e2e_stamp("the_programs_exit_code_is_the_commands", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("exit_code");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] -> None {\n    use StdOutConsole\n    \
         let numbers = array_of(1, 2, 3)\n    let i = 10\n    \
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
    __stamp.verified();
}

// ===== --clean-target =====

/// `both` (the default) leaves nothing behind; `before` keeps the output for
/// inspection. Both clear the target first, so a run never sees the previous
/// run's files.
#[test]
fn clean_target_decides_what_survives_the_run() {
    let Some(__stamp) = e2e_stamp("clean_target_decides_what_survives_the_run", &["rustc"]) else {
        return;
    };
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
    __stamp.verified();
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
    let Some(__stamp) = e2e_stamp("a_visible_target_inside_the_sources_is_rejected", &["rustc"]) else {
        return;
    };
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
    __stamp.verified();
}

/// `--src` and `--main` are independent, each supplying a default for the
/// other (user decision 2026-09-05). Giving both is the only way to say
/// "compile this tree, start at this file" when the entry sits in a
/// *subdirectory* — which is also when it matters, because a shared module
/// one level up is out of reach for `--main` alone.
#[test]
fn src_and_main_together_allow_a_nested_entry_point() {
    let Some(__stamp) = e2e_stamp("src_and_main_together_allow_a_nested_entry_point", &["kotlinc"]) else {
        return;
    };
    let dir = work_dir("src_and_main");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(
        dir.join("helper.sv"),
        "export fn label(n: Int) -> Str {\n    return when {\n        \
         n < 0 { \"neg\" }\n        else { \"nonneg\" }\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        bin.join("tool.sv"),
        "import helper.label\n\nfn main() [use] -> None {\n    \
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
    __stamp.verified();
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
fn an_unknown_backend_is_validated() {
    let dir = work_dir("backend_required");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    // `--backend` is optional now [cli-run] — see
    // `run_defaults_to_the_rust_backend` — but a name that is not a backend is
    // still an error listing the ones that are.
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
        "fn helper(n: Int) -> Int {\n    return n + 1\n}\n",
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

/// [mod-file-name] A `.sv` file whose name contains a dot cannot be a module
/// at all — module paths come from the directory layout — so it can hardly be
/// the entry point. Caught by the name rather than by failing to find `main`,
/// so the message says why. This is the spelling that used to select a
/// backend's `define` file, and the one place a leftover would show up.
#[test]
fn a_dotted_file_name_cannot_be_the_entry_point() {
    let dir = work_dir("dotted_entry");
    fs::write(dir.join("main.sv"), HELLO).unwrap();
    fs::write(dir.join("main.rust.sv"), "\n").unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--main", "main.rust.sv"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("may not contain a dot"),
        "stderr: {stderr}"
    );

    // And discovery says the same thing rather than skipping it: a stale
    // define file left in a source tree must not vanish from the build.
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("main.rust.sv") && stderr.contains("may not contain a dot"),
        "stderr: {stderr}"
    );
}

/// A checker error stops the run, and the diagnostic is the compiler's own
/// (rendered once, with its location) rather than a wrapped restatement.
#[test]
fn a_check_error_stops_the_run() {
    let dir = work_dir("check_error");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] -> None {\n    use StdOutConsole\n    \
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

/// A program whose refinements disagree [qual-refn-ambiguous]: legal, so it
/// runs, and the checker's *warning* has to reach the builder.
const REDUNDANT_SELECTOR: &str = r#"
fn main() [use] {
    use StdOutConsole()
    let xs: Mut List<Int> = mut_list_of()
    // The selector changes nothing here, which is a warning — and a warning
    // must reach the builder without stopping it [fn-overload-at].
    add@core.list(xs, 1)
    println("checked by hand: ${size(xs)}")
}
"#;

/// [qual-refn-ambiguous] [diag-structured] Non-fatal diagnostics reach the
/// builder: `Backend::emit` returns them alongside the files, and the driver
/// prints them without failing. Both halves are asserted, because either one
/// alone is a bug — an abort would reject a legal program, and silence would
/// leave the warning visible only in `salvo analyze`.
///
/// Per backend: the gate and the channel are duplicated in each emitter.
#[test]
fn a_warning_reaches_the_builder_without_failing_the_run() {
    let Some(__stamp) =
        e2e_stamp("a_warning_reaches_the_builder", &["kotlinc", "rustc"])
    else {
        return;
    };
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("warn_{backend}"));
        fs::write(dir.join("main.sv"), REDUNDANT_SELECTOR).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "a warning must not fail the run ({backend}), stderr: {stderr}"
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "checked by hand: 1\n",
            "{backend} stdout (stderr: {stderr})"
        );
        assert!(
            stderr.contains("warning: this `@core.list` is not needed")
                && stderr.contains("main.sv:"),
            "the warning should reach the builder ({backend}), stderr: {stderr}"
        );
    }
    __stamp.verified();
}

/// [cli-run] `--backend` defaults to **rust** (user decision 2026-09-26), the
/// default `salvo test` already had: the two commands agree, and the cheapest
/// toolchain to start is the one you get without saying so.
#[test]
fn run_defaults_to_the_rust_backend() {
    let Some(__stamp) = e2e_stamp("run_defaults_to_rust", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("default_backend");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] {\n    use StdOutConsole()\n    println(\"defaulted\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["run", "--src", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "defaulted\n",
        "stderr: {stderr}"
    );
    __stamp.verified();
}

/// [manifest] [manifest-discovery] With a `salvo.toml` above the working
/// directory, `salvo run` needs no flags: the manifest supplies the source
/// root, the entry point and the backend. A flag still wins over it.
#[test]
fn a_manifest_supplies_what_the_flags_would() {
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let dir = work_dir("manifest");
    let sources = dir.join("salvo");
    fs::create_dir_all(&sources).unwrap();
    fs::write(sources.join("main.sv"), HELLO).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"hello\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), HELLO_STDOUT);

    // From a subdirectory too: discovery walks up.
    let out = salvo_in(&sources, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("no errors"), "{stderr}");

    // A wrong `backend` value is refused at the manifest, naming the file.
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"hello\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"jvm\"\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("salvo.toml") && stderr.contains("[build] backend"), "{stderr}");
}

/// [protocol-lock] A project's actor protocols are locked at its version: a
/// changed protocol at the same version fails the build naming the effect,
/// and a version bump relocks.
#[test]
fn a_changed_protocol_needs_a_version_bump() {
    let dir = work_dir("lock");
    let sources = dir.join("salvo");
    fs::create_dir_all(&sources).unwrap();
    let program = |member: &str| {
        format!(
            "actor effect Counter {{\n    send fn bump(n: {member}) => !n\n}}\n\
             handler Counting() of Counter {{\n    mailbox {{ capacity: 4 }}\n    send fn bump(n: {member}) {{ }}\n}}\n\
             fn main() [use] {{\n    use StdOutConsole()\n    println(\"ok\")\n}}\n"
        )
    };
    let manifest = |version: &str| {
        format!("[project]\nname = \"lock\"\nversion = \"{version}\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n")
    };
    fs::write(sources.join("main.sv"), program("Int")).unwrap();
    fs::write(dir.join("salvo.toml"), manifest("0.1.0")).unwrap();
    let out = salvo_in(&dir, &["compile", "--target", "out"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let lock = fs::read_to_string(dir.join("salvo.lock")).unwrap();
    assert!(lock.contains("version = \"0.1.0\"") && lock.contains("Counter = "), "{lock}");

    // The protocol changes, the version does not: refused.
    fs::write(sources.join("main.sv"), program("Str")).unwrap();
    let out = salvo_in(&dir, &["compile", "--target", "out"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("protocol changed without a version bump") && stderr.contains("`Counter`"),
        "{stderr}"
    );

    // Bumped: relocked.
    fs::write(dir.join("salvo.toml"), manifest("0.2.0")).unwrap();
    let out = salvo_in(&dir, &["compile", "--target", "out"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let lock = fs::read_to_string(dir.join("salvo.lock")).unwrap();
    assert!(lock.contains("version = \"0.2.0\""), "{lock}");
}

/// [manifest-deps] A project's `[dependencies]` are loaded from `[build]
/// modules` by name and compile into the program like any module: the
/// dependency's `main` is not an entry point, its actor protocols do not
/// enter this project's lock, a dependency declaring a std module is refused
/// unless its own manifest says `std = true`, and a missing dependency is an
/// error naming the path.
#[test]
fn a_dependency_is_loaded_from_the_modules_directory() {
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let Some(__stamp) = e2e_stamp("dependency", &["rustc"]) else { return };
    let dir = work_dir("dependency");
    let sources = dir.join("salvo");
    let aws = dir.join("salvo_modules/aws/salvo");
    fs::create_dir_all(&sources).unwrap();
    fs::create_dir_all(&aws).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n\
         modules = \"salvo_modules\"\n\n[dependencies]\naws = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("salvo_modules/aws/salvo.toml"),
        "[project]\nname = \"aws\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nmain = \"salvo/aws.sv\"\n",
    )
    .unwrap();
    // The dependency has a `main` of its own and an actor protocol: neither
    // is the project's business.
    fs::write(
        aws.join("aws.sv"),
        "// AWS services, modelled as actors.\n\n\
         export struct ProfileCredentials {\n    profile: Str = \"default\",\n    path: Str\n}\n\n\
         export actor effect Bucket {\n    send fn put(key: Str) => !key\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    println(\"the dependency's own main\")\n}\n",
    )
    .unwrap();
    fs::write(
        sources.join("main.sv"),
        "import aws.ProfileCredentials\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    \
         let creds = ProfileCredentials {path: \"~/.aws/credentials\"}\n    \
         println(\"${creds.profile}: ${creds.path}\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "default: ~/.aws/credentials\n");
    // Not the project's protocol: no lock entry for `Bucket`, and the lock
    // exists only when there is something to lock.
    let lock = fs::read_to_string(dir.join("salvo.lock")).unwrap_or_default();
    assert!(!lock.contains("Bucket"), "{lock}");

    // A dependency may not redefine std unless its manifest says it is std.
    fs::create_dir_all(aws.join("core")).unwrap();
    fs::write(aws.join("core/list.sv"), "export fn size2() -> Int { return 0 }\n").unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`core.list`") && stderr.contains("std = true") && stderr.contains("[manifest-deps]"),
        "{stderr}"
    );
    fs::remove_dir_all(aws.join("core")).unwrap();

    // The declared version has to be the dependency's own.
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n\
         modules = \"salvo_modules\"\n\n[dependencies]\naws = \"0.2.0\"\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("`0.2.0`") && stderr.contains("`0.1.0`"), "{stderr}");

    // A dependency nothing on disk backs is an error naming the manifest it
    // looked for.
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n\
         modules = \"salvo_modules\"\n\n[dependencies]\ngcp = \"0.1.0\"\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("gcp/salvo.toml") && stderr.contains("does not exist"), "{stderr}");
    __stamp.verified();
}

/// [effect-prereq] End to end on both backends: `effect Files [Streams]`, a
/// handler of `Files` reaching `Streams` without writing it, a fn declaring
/// `[Files]` calling a `Streams` member from a module that never imports it,
/// and a fn value of type `(Str) [Files] -> Str`. Same stdout on both.
#[test]
fn effect_prerequisites_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("effect_prerequisites", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("prereq");
    fs::write(
        dir.join("streams.sv"),
        "export effect Streams {\n    fn next_id() -> Int\n}\n\nexport handler Counter() of Streams {\n    n: Int = 0\n    fn next_id() -> Int {\n        n = n + 1\n        return n\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("files.sv"),
        "import streams.Streams\n\nexport effect Files [Streams] {\n    fn open(path: Str) -> Str => path\n}\n\nexport handler MemFiles() of Files {\n    fn open(path: Str) -> Str => path {\n        return \"${path}#${next_id()}\"\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.sv"),
        "import files.Files\nimport files.MemFiles\nimport streams.Counter\n\n\
         fn work(f: (Str) [Files] -> Str) [Files] -> Str {\n    return \"${f(\"a\")} ${next_id()}\"\n}\n\n\
         fn opener(p: Str) [Files] -> Str => p {\n    return open(p)\n}\n\n\
         fn main() [use] {\n    use StdOutConsole()\n    use Counter()\n    use MemFiles()\n    println(opener(\"x\"))\n    println(work(opener))\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "x#1\na#2 3\n", "{backend}");
    }
    __stamp.verified();
}

/// [stream-provider] A stream belongs to the table that minted it: a handle
/// the host's table never minted traps on both backends, naming the rule, and
/// so does one handed to `MemFs`. The program never gets to print.
#[test]
fn a_stream_from_another_provider_traps() {
    let Some(__stamp) = e2e_stamp("stream_provider", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("stream_provider");
    let program = |imports: &str, uses: &str| {
        format!(
            "import stream\n{imports}\n\nfn main() [use] {{\n    use StdOutConsole()\n{uses}\
             \n    let s = InStream {{ handle: 987654 }}\n    let line = read_line(s)\n    \
             println(\"read ${{line is None}}\")\n    let closed = close(s)\n    when closed {{\n        \
             is Ok {{ println(\"closed\") }}\n        is Err {{ ignore(closed) }}\n    }}\n}}\n"
        )
    };
    for (label, imports, uses) in [
        ("host", "import stream.host", "    use HostRawStreams()\n    use DefaultStreams()"),
        ("mem", "import fs.mem", "    use MemFs()"),
    ] {
        fs::write(dir.join("main.sv"), program(imports, uses)).unwrap();
        for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
            if !have(tool) {
                continue;
            }
            let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(!out.status.success(), "{label}/{backend} should trap");
            assert!(stderr.contains("987654") && stderr.contains("[stream-provider]"), "{label}/{backend}: {stderr}");
            assert!(!String::from_utf8_lossy(&out.stdout).contains("read"), "{label}/{backend}");
        }
    }
    __stamp.verified();
}

/// [stream-receive] [stream-pipe] [stream-from-bytes] Non-blocking copies, end
/// to end on both backends and against both worlds: a buffer piped into a file
/// (GetObject-to-a-file's shape), and a file piped into another (a file is a
/// body: PutObject-from-a-file's). On the host, `receive` reads on a host
/// thread and completes its reply from there [platform-reply]; in memory it
/// answers at once. Same stdout everywhere.
#[test]
fn streams_pipe_without_blocking_on_both_backends() {
    let Some(__stamp) = e2e_stamp("stream_pipe", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("stream_pipe");
    fs::write(dir.join("main.sv"), PIPE_PROGRAM).unwrap();
    let expected = "-- host --\npiped 27 bytes\nlines: [hello, streams, second line]\npiped 27 bytes\n\
                    copy of the copy: 27 bytes\n-- mem --\npiped 27 bytes\nlines: [hello, streams, second line]\n\
                    piped 27 bytes\ncopy of the copy: 27 bytes\n";
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), expected, "{backend}");
    }
    __stamp.verified();
}

const PIPE_PROGRAM: &str = r#"import fs
import fs.host
import fs.mem
import stream
import stream.host

fn describe(r: Ok Long | Err Checked<StreamError>) [] -> Str => !r {
    when r {
        is Ok { return "piped ${r} bytes" }
        is Err { return "failed: ${detach(r)}" }
    }
}

// GetObject-to-a-file's shape: a stream from somewhere, into a file, without
// blocking a worker.
fn copy_in(body: InStream, path: Str) [Fs] -> Str => !body, path {
    let out = open_write(path)
    when out {
        is Ok {
            let result = waitfor done: Reply<Ok Long | Err Checked<StreamError>> {
                pipe(body, out, done)
            }
            return describe(result)
        }
        is Err {
            let closed = close(body)
            if closed is Err { ignore(closed) }
            return "open failed: ${detach(out)}"
        }
    }
}

fn run() [Fs, Console] {
    let body = from_bytes(to_bytes("hello, streams\nsecond line\n"))
    println(copy_in(body, "piped.txt"))
    let back = read_lines("piped.txt")
    when back {
        is Ok { println("lines: ${back}") }
        is Err { println("read: ${detach(back)}") }
    }
    // A file is a body too: PutObject-from-a-file's shape.
    let file = open_read("piped.txt")
    when file {
        is Ok { println(copy_in(file, "piped2.txt")) }
        is Err { println("reopen: ${detach(file)}") }
    }
    let n = read_to_str("piped2.txt")
    when n {
        is Ok { println("copy of the copy: ${byte_size(n)} bytes") }
        is Err { println("read2: ${detach(n)}") }
    }
    let a = delete("piped.txt")
    if a is Err { ignore(a) }
    let b = delete("piped2.txt")
    if b is Err { ignore(b) }
}

fn main() [use] {
    use StdOutConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()
    println("-- host --")
    run()
    println("-- mem --")
    if true {
        use MemFs()
        run()
    }
}
"#;

/// [name-dot] [kt-nested-dot-name] A `type` namespacing its members — the
/// shape the aws generator uses for Smithy enums — runs on both backends:
/// Rust concatenates the names, Kotlin nests the members in an `object`
/// named for the type.
#[test]
fn a_type_namespace_runs_on_both_backends() {
    let Some(__stamp) = e2e_stamp("type_namespace", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("type_namespace");
    fs::write(
        dir.join("sc.sv"),
        "export type StorageClass = StorageClass.Standard | StorageClass.Glacier | StorageClass.Unknown\n\
         export struct StorageClass.Standard {}\nexport struct StorageClass.Glacier {}\n\
         export struct StorageClass.Unknown { value: Str }\n\n\
         export fn describe(c: StorageClass) [] -> Str => c {\n    when c {\n        \
         is StorageClass.Standard { return \"standard\" }\n        is StorageClass.Glacier { return \"glacier\" }\n        \
         is StorageClass.Unknown { return \"unknown: ${c.value}\" }\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.sv"),
        "import sc.StorageClass\nimport sc.describe\n\nfn main() [use] {\n    use StdOutConsole()\n    \
         let one: StorageClass = StorageClass.Glacier {}\n    println(describe(one))\n    \
         println(describe(StorageClass.Standard {}))\n    println(describe(StorageClass.Unknown { value: \"DEEP\" }))\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "glacier\nstandard\nunknown: DEEP\n", "{backend}");
    }
    __stamp.verified();
}

/// [platform-host-deps] A dependency's host libraries join a build only when
/// its platform code is reached: here the dependency declares a crate that
/// does not exist, and a program that imports only its surface still builds
/// with bare `rustc`; importing its host module brings the crate in, which
/// `compile` shows by writing the `Cargo.toml`.
#[test]
fn a_dependency_brings_its_host_libraries_only_when_reached() {
    let Some(__stamp) = e2e_stamp("host_deps_reached", &["rustc"]) else { return };
    let dir = work_dir("host_deps_reached");
    let lib = dir.join("salvo_modules/lib");
    fs::create_dir_all(lib.join("salvo/lib")).unwrap();
    fs::create_dir_all(lib.join("salvo/platform/lib")).unwrap();
    fs::write(
        lib.join("salvo.toml"),
        "[project]\nname = \"lib\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nplatform = \"salvo/platform\"\n\n\
         [rust]\ncrates = { no-such-crate-salvo-test = \"9.9.9\" }\n",
    )
    .unwrap();
    fs::write(lib.join("salvo/lib.sv"), "export effect Greet {\n    fn greet() -> Str\n}\n\n\
         export handler Plain() of Greet {\n    fn greet() -> Str { return \"plain\" }\n}\n").unwrap();
    fs::write(lib.join("salvo/lib/host.sv"), "import lib\n\nexport platform handler HostGreet of Greet\n").unwrap();
    fs::write(lib.join("salvo/platform/lib/host.rs"), "use crate::lib::*;\npub struct HostGreet;\n\
         impl HostGreet { pub fn new() -> Self { HostGreet } }\n\
         impl crate::lib::GreetPlatform for HostGreet { fn greet(&mut self) -> String { \"host\".into() } }\n").unwrap();
    fs::create_dir_all(dir.join("salvo")).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n\
         modules = \"salvo_modules\"\n\n[dependencies]\nlib = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(dir.join("salvo/main.sv"), "import lib\n\nfn main() [use] {\n    use StdOutConsole()\n    \
         use Plain()\n    println(greet())\n}\n").unwrap();
    let out = salvo_in(&dir, &["run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the unreached crate must not be fetched: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "plain\n");

    // Reaching the host module brings the crate: the manifest is written. The
    // dependency checks in its host project first [platform-stamp].
    let out = salvo_in(&lib, &["platform", "generate", "--backend", "rust"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    fs::write(dir.join("salvo/main.sv"), "import lib\nimport lib.host\n\nfn main() [use] {\n    \
         use StdOutConsole()\n    use HostGreet()\n    println(greet())\n}\n").unwrap();
    let out = salvo_in(&dir, &["compile", "--target", "out"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let cargo = fs::read_to_string(dir.join("out/Cargo.toml")).unwrap();
    assert!(cargo.contains("no-such-crate-salvo-test"), "{cargo}");
    __stamp.verified();
}

/// [type-literal] [union-arm-identity] Unions of literals on both backends,
/// same stdout: an open union (`Other Str`) that is a plain string at run time,
/// `is "A"` and `is Other` in a `when`, a `?`-typed literal union assigned
/// later and narrowed, a named literal sub-union tested with `is`, widening to
/// `Str` for a call and for `==`, and a mixed union whose two `Str` literals
/// share one arm beside an `Int` and a struct.
#[test]
fn unions_of_literals_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("literal_unions", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("literal_unions");
    fs::write(dir.join("main.sv"), LITERAL_PROGRAM).unwrap();
    let expected = "hot\ncold\nunknown: DEEP\ncold\nkms\nmissing\nother Throttling\nKmsDisabled!\nequal\nthe name\nthree\nthe other\nperson Ada\nmissing 400\n";
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), expected, "{backend}");
    }
    __stamp.verified();
}

const LITERAL_PROGRAM: &str = r#"type Class = "STANDARD" | "GLACIER" | Other Str

fn describe_class(c: Class) [] -> Str => c {
    when c {
        is "STANDARD" { return "hot" }
        is "GLACIER" { return "cold" }
        is Other { return "unknown: ${c}" }
    }
}

fn part_one() [Console] {
    let a: Class = "STANDARD"
    println(describe_class(a))
    println(describe_class("GLACIER"))
    println(describe_class(other("DEEP")))
    let later: Class? = None
    later = "GLACIER"
    if later is None {
        return None
    }
    println(describe_class(later))
}

struct Person { name: Str }

type Kms = "KmsDisabled" | "KmsThrottled"
type Code = Kms | "QueueDoesNotExist" | Other Str
type Mixed = "name" | 3 | "other" | Person

// An exported struct has a wire codec, and a field whose type is an alias
// holding an alias of literals must collapse to its base there too.
export struct Failure { code: Code, status: Int }

fn kind(c: Code) [] -> Str => c {
    if c is Kms {
        return "kms"
    }
    when c {
        is "QueueDoesNotExist" { return "missing" }
        is Other { return "other ${c}" }
    }
}

fn shout(s: Str) [] -> Str => s {
    return "${s}!"
}

fn mixed(m: Mixed) [] -> Str => m {
    when m {
        is "name" { return "the name" }
        is 3 { return "three" }
        is "other" { return "the other" }
        is Person { return "person ${m.name}" }
    }
}

fn main() [use] {
    use StdOutConsole()
    part_one()
    println(kind("KmsThrottled"))
    println(kind("QueueDoesNotExist"))
    println(kind(other("Throttling")))
    let c: Code = "KmsDisabled"
    println(shout(c))
    if c == "KmsDisabled" {
        println("equal")
    }
    println(mixed("name"))
    println(mixed(3))
    println(mixed("other"))
    println(mixed(Person { name: "Ada" }))
    let f = Failure { code: "QueueDoesNotExist", status: 400 }
    println("${kind(f.code)} ${f.status}")
}
"#;


/// [platform-fn] [platform-handler] Implementation files on both backends,
/// same stdout: `platform fn`s (a field read, a struct built from host
/// values, a union answered through its arm, a list of structs) and a platform
/// handler with constructor parameters whose class keeps its own state. The
/// program calls each fn's wrapper (`shoutPlatform` / `shout_platform`), which
/// calls the implementation by its real name. Without the implementation
/// file the build fails naming `salvo platform generate`.
#[test]
fn platform_fns_and_handlers_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("platform_impls", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("platform_impls");
    fs::create_dir_all(dir.join("salvo/platform")).unwrap();
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"impls\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"*\"\nplatform = \"salvo/platform\"\n").unwrap();
    fs::write(dir.join("salvo/main.sv"), IMPL_SV).unwrap();
    let expected = "HELLO!\nquiet\nok 42\nerr not a number: x\n15 16\nString=hi, Number=-\n";
    for (backend, tool, ext, implementation) in [("rust", "rustc", "rs", IMPL_RS), ("kotlin", "kotlinc", "kt", IMPL_KT)] {
        if !have(tool) {
            continue;
        }
        let host = dir.join(format!("salvo/platform/main.{ext}"));
        let _ = fs::remove_file(&host);
        let out = salvo_in(&dir, &["run", "--backend", backend]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && stderr.contains("salvo platform generate"), "{backend}: {stderr}");
        fs::write(&host, implementation).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), expected, "{backend}");
    }
    __stamp.verified();
}

const IMPL_SV: &str = r#"struct Greeting { text: Str, loud: Bool }
struct Attr { data_type: Str, string_value: Str? = None }

platform fn shout(g: Greeting) [] -> Str => g
platform fn make(text: Str) [] -> Greeting => text
platform fn parse(s: Str) [] -> Ok Int | Err Str => s
platform fn describe_attrs(attrs: List<Attr>) [] -> Str => attrs

effect Counter {
    fn next(step: Int) -> Int => step
}

platform handler HostCounter(start: Int) of Counter

fn describe(r: Ok Int | Err Str) [] -> Str => r {
    when r {
        is Ok { return "ok ${r}" }
        is Err { return "err ${r}" }
    }
}

fn main() [use] {
    use StdOutConsole()
    use HostCounter(10)
    println(shout(make("hello")))
    println(shout(Greeting { text: "quiet", loud: false }))
    println(describe(parse("42")))
    println(describe(parse("x")))
    println("${next(5)} ${next(1)}")
    println(describe_attrs([Attr { data_type: "String", string_value: "hi" }, Attr { data_type: "Number" }]))
}
"#;

const IMPL_KT: &str = r#"package salvo.platform.main

import salvo.*
import salvo.main.*

fun shout(g: Greeting): String = if (g.loud) g.text.uppercase() + "!" else g.text

fun make(text: String): Greeting = Greeting(text = text, loud = true)

fun parse(s: String): Union2<Int, String> {
    val n = s.toIntOrNull() ?: return Union2.U2("not a number: $s")
    return Union2.U1(n)
}

fun describeAttrs(attrs: List<Attr>): String =
    attrs.joinToString(", ") { "${it.dataType}=${it.stringValue ?: "-"}" }

class HostCounter(start: Int) : CounterPlatform {
    private var at = start
    override fun next(step: Int): Int {
        at += step
        return at
    }
}
"#;

const IMPL_RS: &str = r#"use crate::*;
use crate::unions::*;

pub fn shout(g: &Greeting) -> String {
    if g.loud { format!("{}!", g.text.to_uppercase()) } else { g.text.clone() }
}

pub fn make(text: &String) -> Greeting {
    Greeting { text: text.clone(), loud: true }
}

pub fn parse(s: &String) -> Union2<i32, String> {
    match s.parse::<i32>() {
        Ok(n) => Union2::U1(n),
        Err(_) => Union2::U2(format!("not a number: {s}")),
    }
}

pub fn describe_attrs(attrs: &Vec<Attr>) -> String {
    attrs
        .iter()
        .map(|a| format!("{}={}", a.data_type, a.string_value.clone().unwrap_or("-".to_string())))
        .collect::<Vec<_>>()
        .join(", ")
}

pub struct HostCounter {
    at: i32,
}

impl HostCounter {
    pub fn new(start: i32) -> Self {
        Self { at: start }
    }
}

impl crate::CounterPlatform for HostCounter {
    fn next(&mut self, step: i32) -> i32 {
        self.at += step;
        self.at
    }
}
"#;

/// [type-identity] One type name in three modules — a program's own `Token`,
/// two libraries' — with an effect named like std's `time.Clock`, an aliased
/// import (`import lib.b.Token as BToken`), a union over one library's
/// `Token`, a wire codec and auto equality on the other's, and an actor
/// protocol over the program's own. Every stage after resolution knows which
/// declaration each use means; same stdout on both backends.
#[test]
fn type_names_clash_across_modules_on_both_backends() {
    let Some(__stamp) = e2e_stamp("type_names_clash", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("type_clash");
    fs::create_dir_all(dir.join("lib")).unwrap();
    fs::write(dir.join("lib/a.sv"), r#"// A `Token` with a wire form and auto equality, and an effect named like
// std's `time.Clock`.
import net

export struct Token : Hashed<self> by auto { id: Int }

export effect Clock {
    fn now() -> Long
}

export handler Fixed(at: Long) of Clock {
    fn now() -> Long { return copy(at) }
}

export fn make_a() [] -> Token {
    return Token { id: 1 }
}

export fn show_a(t: Token) [Clock] -> Str {
    let back = decode<Token>(encode(copy(t)))!
    let s: Set<Token> = {copy(t), back}
    return "a.Token ${t.id}, set of ${size(s)}, at ${now()}"
}
"#).unwrap();
    fs::write(dir.join("lib/b.sv"), r#"// A different `Token`: a fn field and a union over it.
export struct Token { name: Str, f: (Int) -> Int }
export struct Circle { r: Int }
export type Shape = Token | Circle

export fn make_b() [] -> Token {
    return Token { name: "bee", f: x -> x + 1 }
}

export fn show_b(t: Token) [] -> Str {
    let g = t.f
    return "b.Token ${t.name} ${g(1)}"
}

export fn describe(s: Shape) [] -> Str {
    when s {
        is Token { return "a token named ${s.name}" }
        is Circle { return "a circle of ${s.r}" }
    }
}
"#).unwrap();
    fs::write(dir.join("main.sv"), r#"import lib.a.make_a
import lib.a.show_a
import lib.a.Fixed
import lib.b.Token as BToken
import lib.b.Circle
import lib.b.make_b
import lib.b.show_b
import lib.b.describe

// This module's own `Token`, beside two imported ones.
struct Token { label: Str }

actor effect Counter {
    send fn bump(t: Token) => !t
    send fn total(out: Reply<Str>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 4 }
    seen: Str = ""
    send fn bump(t: Token) => !t { seen = "${seen}${t.label}" }
    send fn total(out: Reply<Str>) => !out { out.send(copy(seen)) }
}

fn main() [use, spawn] {
    use StdOutConsole()
    use Fixed(42L)
    let mine = Token { label: "mine" }
    println(mine.label)
    println(show_a(make_a()))
    let b: BToken = make_b()
    println(b.name)
    println(show_b(b))
    println(describe(BToken { name: "two", f: x -> x }))
    println(describe(Circle { r: 3 }))
    let c = spawn Counting() on pool(1)
    c.bump(Token { label: "x" })
    c.bump(copy(mine))
    let s = waitfor out: Reply<Str> { c.total(out) }
    println(s)
}
"#).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "mine\na.Token 1, set of 1, at 42\nbee\nb.Token bee 2\na token named two\na circle of 3\nxmine\n",
            "{backend}"
        );
    }
    __stamp.verified();
}
