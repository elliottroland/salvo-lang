//! Integration tests for `salvo run` [cli-run]: compile with a backend and
//! run the result through that backend's toolchain.
//!
//! The command's exit code is the *program's*, so `salvo run` stands in for
//! running the binary. Tests that need a toolchain (`kotlinc`, `rustc`) skip
//! gracefully when it is missing; the argument-validation tests need none,
//! since validation happens before anything is built.
//!
//! What a program does — its output, exit code, trap and streams — is run
//! data (`tests/run/`, [test-run-data]); the tests here are about the
//! command: what it accepts, builds and forwards, and what stays out of the
//! data's reach (environment variables, an argument holding a space, a
//! carriage return on stdin, a host runtime's own panic).

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
        // [kt-std-library] The compiled std is cached in the test target.
        .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
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

/// [test-cli-stamp] A stamp keyed on the binary, then on what it emits for
/// `projects` (`salvo_testkit::cli`): write the projects first.
fn cli_stamp(test: &str, tools: &[&str], projects: &[salvo_testkit::cli::CliProject]) -> Option<salvo_testkit::cli::CliStamp> {
    let t = salvo_testkit::cli::CliTest {
        salvo: env!("CARGO_BIN_EXE_salvo"),
        tmpdir: env!("CARGO_TARGET_TMPDIR"),
        name: test,
        source: include_str!("run_tests.rs"),
        tools,
    };
    salvo_testkit::cli::cli_stamp(&t, projects)
}

/// A project of a CLI test, for [`cli_stamp`].
fn project<'a>(dir: &'a Path, args: &'a [&'a str], backends: &'a [&'a str]) -> salvo_testkit::cli::CliProject<'a> {
    salvo_testkit::cli::CliProject { dir, args, backends }
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
    use StdConsole
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
    let dirs: Vec<PathBuf> = ["kotlin", "rust"].iter().map(|b| work_dir(&format!("hello_{b}"))).collect();
    for dir in &dirs {
        fs::write(dir.join("main.sv"), HELLO).unwrap();
    }
    let Some(__stamp) = cli_stamp(
        "run_compiles_and_runs_with_each_backend",
        &["kotlinc", "rustc"],
        &[project(&dirs[0], &["--src", "."], &["kotlin"]), project(&dirs[1], &["--src", "."], &["rust"])],
    ) else {
        return;
    };
    for ((backend, tool), dir) in [("kotlin", "kotlinc"), ("rust", "rustc")].into_iter().zip(&dirs) {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
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

/// [cli-args] A program that reads the command line through the `Args`
/// effect: the arguments after `--` reach `args()`, program name excluded,
/// and the *same* list prints on both backends — the parity the two hosts'
/// different argv conventions are reconciled to.
const ARGS_PROGRAM: &str = r#"
import std.args

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    let a = args()
    println("count: ${size(a)}")
    for arg in a {
        println(arg)
    }
}
"#;

#[test]
fn run_forwards_command_line_arguments_to_both_backends() {
    let dirs: Vec<PathBuf> = ["kotlin", "rust"].iter().map(|b| work_dir(&format!("args_{b}"))).collect();
    for dir in &dirs {
        fs::write(dir.join("main.sv"), ARGS_PROGRAM).unwrap();
    }
    let Some(__stamp) = cli_stamp(
        "run_forwards_command_line_arguments_to_both_backends",
        &["kotlinc", "rustc"],
        &[project(&dirs[0], &["--src", "."], &["kotlin"]), project(&dirs[1], &["--src", "."], &["rust"])],
    ) else {
        return;
    };
    // program name excluded, order kept, a value with a space preserved.
    let expected = "count: 3\nalpha\nbeta gamma\ndelta\n";
    for ((backend, tool), dir) in [("kotlin", "kotlinc"), ("rust", "rustc")].into_iter().zip(&dirs) {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(
            &dir,
            &["run", "--backend", backend, "--src", ".", "--", "alpha", "beta gamma", "delta"],
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed, stderr: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            expected,
            "{backend} stdout (stderr: {stderr})"
        );
    }
    __stamp.verified();
}

// ===== salvo build: persisted artifacts =====

/// [cli-build] `salvo build` produces a persisted, runnable artifact — a
/// native binary for Rust, a jar + `sh` launcher for Kotlin — that runs to
/// the *same* stdout on both backends, with argv forwarded. The artifact
/// outlives the command (unlike `run`'s scratch build), and the launcher
/// works from any directory.
#[test]
fn build_produces_a_runnable_artifact_on_both_backends() {
    // No annex, so the tests `build` runs first are an empty suite: nothing
    // `compile` leaves out.
    let dirs: Vec<PathBuf> = ["kotlin", "rust"].iter().map(|b| work_dir(&format!("build_{b}"))).collect();
    for dir in &dirs {
        fs::write(dir.join("main.sv"), ARGS_PROGRAM).unwrap();
    }
    let Some(__stamp) = cli_stamp(
        "build_produces_a_runnable_artifact_on_both_backends",
        &["kotlinc", "rustc"],
        &[project(&dirs[0], &["--src", "."], &["kotlin"]), project(&dirs[1], &["--src", "."], &["rust"])],
    ) else {
        return;
    };
    for ((backend, tool), dir) in [("kotlin", "kotlinc"), ("rust", "rustc")].into_iter().zip(&dirs) {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["build", "--backend", backend, "--src", ".", "--out", "out"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} build failed, stderr: {stderr}");
        // The artifact is `out/main` (a binary or a launcher) and persists.
        let artifact = dir.join("out").join("main");
        assert!(artifact.is_file(), "{backend}: no artifact at {}", artifact.display());
        // Run it directly, from an unrelated working directory, with args.
        let ran = Command::new(&artifact)
            .current_dir(env!("CARGO_TARGET_TMPDIR"))
            .args(["alpha", "beta gamma", "delta"])
            .output()
            .expect("failed to run the artifact");
        assert!(ran.status.success(), "{backend} artifact failed: {}", String::from_utf8_lossy(&ran.stderr));
        assert_eq!(
            String::from_utf8_lossy(&ran.stdout),
            "count: 3\nalpha\nbeta gamma\ndelta\n",
            "{backend} artifact stdout"
        );
    }
    __stamp.verified();
}

/// [cli-build] A failing test fails the build and writes no artifact
/// (decision 3). `--skip-tests` builds anyway.
#[test]
fn build_runs_tests_first_and_a_failure_blocks_the_artifact() {
    // Keyed on the binary: the build runs an annex, which `compile` does not emit.
    let Some(__stamp) = e2e_stamp("build_runs_tests_first_and_a_failure_blocks_the_artifact", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        __stamp.verified();
        return;
    }
    let dir = work_dir("build_failing_test");
    fs::write(
        dir.join("main.sv"),
        "fn two() -> Int { return 2 }\nfn main() [use] -> None { use StdConsole()\n println(\"${two()}\") }\n",
    )
    .unwrap();
    fs::write(dir.join("main.test.sv"), "test \"wrong\" { assert!(two() == 3) }\n").unwrap();

    // Tests run, fail, and block the build.
    let out = salvo_in(&dir, &["build", "--backend", "rust", "--src", ".", "--out", "out"]);
    assert!(!out.status.success(), "a failing test should fail the build");
    assert!(!dir.join("out").join("main").exists(), "no artifact on a failed build");

    // `--skip-tests` builds the artifact despite the failing test.
    let out = salvo_in(&dir, &["build", "--backend", "rust", "--src", ".", "--out", "out", "--skip-tests"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "--skip-tests build failed: {stderr}");
    assert!(dir.join("out").join("main").is_file(), "--skip-tests should build the artifact");
    __stamp.verified();
}

/// [cli-build] Under `backend = "*"` each backend must name its own `out`;
/// with none, the command refuses before building anything. No toolchain
/// needed — the check is argument validation.
#[test]
fn build_under_star_requires_an_explicit_out_per_backend() {
    let dir = work_dir("build_star_no_out");
    fs::create_dir_all(dir.join("salvo")).unwrap();
    fs::write(dir.join("salvo").join("main.sv"), "fn main() [use] -> None { use StdConsole()\n println(\"x\") }\n").unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"s\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"*\"\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["build"]);
    assert!(!out.status.success(), "`*` without out should be refused");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("each needs its own artifact directory"), "stderr: {stderr}");
}

// ===== salvo init: scaffolding a project =====

/// [cli-init] `salvo init PATH` lays down a `salvo.toml`, a `src/main.sv`,
/// and the `target/`/`build/` directories, with the project named after the
/// directory. No toolchain needed — this is filesystem scaffolding.
#[test]
fn init_scaffolds_a_project() {
    let dir = work_dir("init_scaffold");
    let project = dir.join("proj");
    let out = salvo_in(&dir, &["init", "proj"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "init failed: {stderr}");

    assert!(project.join("salvo.toml").is_file(), "no manifest");
    assert!(project.join("src").join("main.sv").is_file(), "no starter main.sv");
    assert!(project.join("target").is_dir(), "no target/");
    assert!(project.join("build").is_dir(), "no build/");

    let manifest = fs::read_to_string(project.join("salvo.toml")).unwrap();
    // Named after the directory, builds with Rust out of `src`, and the
    // Kotlin / extra options are present but commented.
    assert!(manifest.contains("name = \"proj\""), "manifest:\n{manifest}");
    assert!(manifest.contains("backend = \"rust\""), "manifest:\n{manifest}");
    assert!(manifest.contains("src = \"src\""), "manifest:\n{manifest}");
    assert!(manifest.contains("# [kotlin]"), "kotlin example should be commented:\n{manifest}");
    assert!(manifest.contains("# [dependencies]"), "deps example should be commented:\n{manifest}");
}

/// [cli-init] `init` refuses to overwrite an existing project rather than
/// clobbering a manifest.
#[test]
fn init_refuses_to_overwrite_an_existing_manifest() {
    let dir = work_dir("init_overwrite");
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"mine\"\nversion = \"9.9.9\"\n").unwrap();
    let out = salvo_in(&dir, &["init", "."]);
    assert!(!out.status.success(), "init should refuse an existing manifest");
    // The original is untouched.
    assert!(fs::read_to_string(dir.join("salvo.toml")).unwrap().contains("9.9.9"));
}

/// [cli-init] The scaffolded project runs as-is: `init` then `run` prints the
/// starter program's output, reading the generated manifest with no flags.
#[test]
fn init_then_run_works_with_no_flags() {
    let dir = work_dir("init_run");
    let out = salvo_in(&dir, &["init", "."]);
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));
    // The scaffold is the project: written before the stamp, which keys on it.
    let Some(__stamp) = cli_stamp("init_then_run_works_with_no_flags", &["rustc"], &[project(&dir, &[], &["rust"])]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        __stamp.verified();
        return;
    }
    // No flags: the manifest supplies the backend (rust) and src.
    let out = salvo_in(&dir, &["run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "run failed: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello from Salvo!\n");
    __stamp.verified();
}

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
                "fn main() [use] -> None {{\n    use StdConsole\n    \
                 println(\"{text}\")\n}}\n"
            ),
        )
        .unwrap();
    }
    let Some(__stamp) = cli_stamp(
        "main_flag_selects_among_several_entry_points",
        &["kotlinc", "rustc"],
        &[project(&dir, &["--main", "main.sv"], &[]), project(&dir, &["--main", "other.sv"], &[])],
    ) else {
        return;
    };
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
    let dir = work_dir("exit_code");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] -> None {\n    use StdConsole\n    \
         let numbers = array_of(1, 2, 3)\n    let i = 10\n    \
         println(\"value ${numbers[i]}\")\n}\n",
    )
    .unwrap();
    let Some(__stamp) = cli_stamp("the_programs_exit_code_is_the_commands", &["rustc"], &[project(&dir, &["--src", "."], &[])]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
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

/// [cli-process] [cli-env] [rs-never-call] `exit` ends the process with its
/// status after flushing what was printed without a newline, from statement,
/// value and tail position; `env` answers a set variable and `None` for an
/// unset one. Built once per backend, then the artifact runs under three
/// environments, one per exit path.
#[test]
fn process_exit_and_env_run_on_both_backends() {
    const SRC: &str = r#"import std.process
import std.env

fn pick(c: Bool) [Process] -> Int {
    let x: Int = if c { exit(9) } else { 3 }
    return x
}

fn give_up() [Process] -> Int {
    exit(7)
}

fn main() [use] -> None {
    use StdConsole()
    use HostProcess()
    use HostEnv()
    let probe = env("SALVO_PROBE") ?: "unset"
    println("${probe} ${env("SALVO_PROBE_UNSET") ?: "unset"} ${pick(false)}")
    print("no newline")
    if probe == "value" {
        println("${pick(true)}")
    }
    if probe == "tail" {
        println("${give_up()}")
    }
    exit(5)
}
"#;
    // No annex, so the tests `build` runs first are an empty suite.
    let dirs: Vec<PathBuf> = ["rust", "kotlin"].iter().map(|b| work_dir(&format!("process_env_{b}"))).collect();
    for dir in &dirs {
        fs::write(dir.join("main.sv"), SRC).unwrap();
    }
    let Some(__stamp) = cli_stamp(
        "process_exit_and_env_run_on_both_backends",
        &["rustc", "kotlinc"],
        &[project(&dirs[0], &["--src", "."], &["rust"]), project(&dirs[1], &["--src", "."], &["kotlin"])],
    ) else {
        return;
    };
    for ((backend, tool), dir) in [("rust", "rustc"), ("kotlin", "kotlinc")].into_iter().zip(&dirs) {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["build", "--backend", backend, "--src", ".", "--out", "out"]);
        assert!(out.status.success(), "{backend} build failed: {}", String::from_utf8_lossy(&out.stderr));
        for (probe, code) in [("plain", 5), ("value", 9), ("tail", 7)] {
            let ran = Command::new(dir.join("out").join("main"))
                .env("SALVO_PROBE", probe)
                .env_remove("SALVO_PROBE_UNSET")
                .output()
                .expect("failed to run the artifact");
            let stderr = String::from_utf8_lossy(&ran.stderr);
            assert_eq!(ran.status.code(), Some(code), "{backend} {probe}: {stderr}");
            assert_eq!(String::from_utf8_lossy(&ran.stdout), format!("{probe} unset 3\nno newline"), "{backend} {probe}: {stderr}");
        }
    }
    __stamp.verified();
}

// ===== --clean-target =====

/// `both` (the default) leaves nothing behind; `before` keeps the output for
/// inspection. Both clear the target first, so a run never sees the previous
/// run's files.
#[test]
fn clean_target_decides_what_survives_the_run() {
    // Keyed on the binary: the point is what `run` does with its target, and
    // the test writes into it between runs.
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
    // Keyed on the binary: the point is which targets `run` accepts, which
    // `compile` into a scratch target would not show.
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
         use StdConsole\n    println(\"nested ${label(3)}\")\n}\n",
    )
    .unwrap();
    let Some(__stamp) = cli_stamp(
        "src_and_main_together_allow_a_nested_entry_point",
        &["kotlinc", "rustc"],
        &[project(&dir, &["--src", ".", "--main", "bin/tool.sv"], &[]), project(&dir, &["--main", "bin/tool.sv"], &["rust"])],
    ) else {
        return;
    };

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
        "fn main() [use] -> None {\n    use StdConsole\n    \
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

/// A program whose refinements disagree [qual-refn]: legal, so it
/// runs, and the checker's *warning* has to reach the builder.
const REDUNDANT_SELECTOR: &str = r#"
fn main() [use] {
    use StdConsole()
    let xs: Mut List<Int> = Mut List()
    // The selector changes nothing here, which is a warning — and a warning
    // must reach the builder without stopping it [fn-overload-at].
    add(xs, 1)
    println("checked by hand: ${size@core.list(xs)}")
}
"#;

/// [qual-refn] [diag-structured] Non-fatal diagnostics reach the
/// builder: `Backend::emit` returns them alongside the files, and the driver
/// prints them without failing. Both halves are asserted, because either one
/// alone is a bug — an abort would reject a legal program, and silence would
/// leave the warning visible only in `salvo analyze`.
///
/// Per backend: the gate and the channel are duplicated in each emitter, so
/// each backend's `compile` must print it — which needs no toolchain. The
/// run, which the warning must not fail, is the driver's and the same for
/// both, so it runs on Rust only (a Kotlin run was a `kotlinc` for nothing
/// the compile does not show).
#[test]
fn a_warning_reaches_the_builder_without_failing_the_run() {
    let warned = |stderr: &str| stderr.contains("warning: this `@core.list` is not needed") && stderr.contains("main.sv:");
    for backend in ["kotlin", "rust"] {
        let dir = work_dir(&format!("warn_{backend}"));
        fs::write(dir.join("main.sv"), REDUNDANT_SELECTOR).unwrap();
        let out = salvo_in(&dir, &["compile", "--backend", backend, "--src", ".", "--target", "out"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "a warning must not fail the compile ({backend}), stderr: {stderr}");
        assert!(warned(&stderr), "the warning should reach the builder ({backend}), stderr: {stderr}");
    }
    // Keyed on the binary: the warning is the checker's, and `compile`'s
    // emitted files would not change if it went missing.
    let Some(__stamp) = e2e_stamp("a_warning_reaches_the_builder", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping the run: rustc not found on PATH");
        return;
    }
    let dir = work_dir("warn_run");
    fs::write(dir.join("main.sv"), REDUNDANT_SELECTOR).unwrap();
    let out = salvo_in(&dir, &["run", "--backend", "rust", "--src", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a warning must not fail the run, stderr: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "checked by hand: 1\n", "stdout (stderr: {stderr})");
    assert!(warned(&stderr), "the warning should reach the builder, stderr: {stderr}");
    __stamp.verified();
}

/// [cli-run] `--backend` defaults to **rust** (user decision 2026-09-26), the
/// default `salvo test` already had: the two commands agree, and the cheapest
/// toolchain to start is the one you get without saying so.
#[test]
fn run_defaults_to_the_rust_backend() {
    let dir = work_dir("default_backend");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] {\n    use StdConsole()\n    println(\"defaulted\")\n}\n",
    )
    .unwrap();
    let Some(__stamp) = cli_stamp("run_defaults_to_rust", &["rustc"], &[project(&dir, &["--src", "."], &[])]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
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

/// [actor-protocol-lock] A project's actor protocols are locked at its version: a
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
             fn main() [use] {{\n    use StdConsole()\n    println(\"ok\")\n}}\n"
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
    let Some(__stamp) = e2e_stamp("dependency", &["rustc"]) else { return }; // the test rewrites the manifest and adds files
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
         fn main() [use] {\n    use StdConsole()\n    println(\"the dependency's own main\")\n}\n",
    )
    .unwrap();
    fs::write(
        sources.join("main.sv"),
        "import aws.ProfileCredentials\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
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
    fs::create_dir_all(aws.join("std/core")).unwrap();
    fs::write(aws.join("std/core/list.sv"), "export fn size2() -> Int { return 0 }\n").unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`std.core.list`") && stderr.contains("manifest says `[project] std = true`"),
        "{stderr}"
    );
    fs::remove_dir_all(aws.join("std")).unwrap();

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


/// [platform-host-deps] A dependency's host libraries join a build only when
/// its platform code is reached: here the dependency declares a crate that
/// does not exist, and a program that imports only its surface still builds
/// with bare `rustc`; importing its host module brings the crate in, which
/// `compile` shows by writing the `Cargo.toml`.
#[test]
fn a_dependency_brings_its_host_libraries_only_when_reached() {
    let Some(__stamp) = e2e_stamp("host_deps_reached", &["rustc"]) else { return }; // runs `platform generate` and rewrites main.sv
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
    fs::write(dir.join("salvo/main.sv"), "import lib\n\nfn main() [use] {\n    use StdConsole()\n    \
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
         use StdConsole()\n    use HostGreet()\n    println(greet())\n}\n").unwrap();
    let out = salvo_in(&dir, &["compile", "--target", "out"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let cargo = fs::read_to_string(dir.join("out/Cargo.toml")).unwrap();
    assert!(cargo.contains("no-such-crate-salvo-test"), "{cargo}");
    __stamp.verified();
}

/// [platform-fn] [platform-handler] Without its implementation file a
/// program with `platform fn`s and a platform handler is refused, naming
/// `salvo platform generate`. What it does with one is run data
/// (`tests/run/platform/boundary.sv#platform_fns_and_handlers`).
#[test]
fn a_missing_implementation_file_names_the_generate_command() {
    let dir = work_dir("platform_impls");
    fs::create_dir_all(dir.join("salvo/platform")).unwrap();
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"impls\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"*\"\nplatform = \"salvo/platform\"\n").unwrap();
    fs::write(dir.join("salvo/main.sv"), IMPL_SV).unwrap();
    let Some(__stamp) = cli_stamp("platform_impls", &["rustc", "kotlinc"], &[project(&dir, &[], &[])]) else { return };
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && stderr.contains("salvo platform generate"), "{backend}: {stderr}");
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
    use StdConsole()
    use HostCounter(10)
    println(shout(make("hello")))
    println(shout(Greeting { text: "quiet", loud: false }))
    println(describe(parse("42")))
    println(describe(parse("x")))
    println("${next(5)} ${next(1)}")
    println(describe_attrs(List(Attr { data_type: "String", string_value: "hi" }, Attr { data_type: "Number" })))
}
"#;

/// [fn-overload-return] Overloads that differ only in their return type:
/// neither target overloads by return type, so each gets a name of its own
/// from [fn-emit-name]'s sixth level (`read_as__Str_IntOpt`; Kotlin
/// camel-cases the part before the `__`). No toolchain needed: what the
/// program prints is run data (`tests/run/functions/overloads_and_fn_values.sv`).
#[test]
fn return_type_overloads_are_named_by_their_return_types() {
    let dir = work_dir("return_overloads");
    fs::write(
        dir.join("main.sv"),
        "fn read_as(s: Str) [] -> Int? => s { return parse_int(s) }\n\
         fn read_as(s: Str) [] -> Bool? => s {\n    if s == \"true\" { return true }\n    \
         if s == \"false\" { return false }\n    return None\n}\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    \
         let a: Int? = read_as(\"12\")\n    let b: Bool? = read_as(\"true\")\n    \
         println(\"${a ?: 0} ${b ?: false}\")\n}\n",
    )
    .unwrap();
    for (backend, ext) in [("rust", "rs"), ("kotlin", "kt")] {
        let target = format!("out_{backend}");
        let out = salvo_in(&dir, &["compile", "--backend", backend, "--src", ".", "--target", &target]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        let emitted = fs::read_to_string(dir.join(&target).join(format!("main.{ext}"))).unwrap();
        assert!(
            emitted.contains("__Str_IntOpt(") && emitted.contains("__Str_BoolOpt("),
            "{backend}: the two overloads are named by their return types:\n{emitted}"
        );
    }
}

/// [rs-borrow-locals] [proj-local] A local whose type is a projection (`let k
/// = get(xs, 0)`) is a borrow for its whole life: assigning it an owned value
/// is refused, with `copy` at the declaration the remedy (user decision
/// 2026-10-20). The remedied program runs as data
/// (`tests/run/deductions/locals_and_interpolation_order.sv`).
#[test]
fn a_projection_local_assigned_an_owned_value_is_refused() {
    const UP: &str = "fn up(s: Str) [] -> Str? {\n    if size(s) > 1 {\n        return substr(s, 1, size(s))\n    }\n    return None\n}\n\n";
    const WALK: &str = "fn walk(key: Str?) [] -> Int => key {\n    let k = key\n    let n = 0\n    while k !is None {\n        let here = copy(k)\n        n = n + 1\n        k = up(here)\n    }\n    return n\n}\n\n";
    const WALK_LIST: &str = "fn walk_list(xs: List<Str>) [] -> Int => xs {\n    let k = get(xs, 0)\n    let n = 0\n    while k !is None {\n        let here = copy(k)\n        n = n + 1\n        k = up(here)\n    }\n    return n\n}\n\n";
    const MAIN: &str = "fn main() [use] -> None {\n    use StdConsole()\n    println(\"${walk(\"abcd\")} ${walk_list(List(\"xyz\"))}\")\n}\n";
    let dir = work_dir("projlocal_refused");
    fs::write(dir.join("main.sv"), format!("{UP}{WALK}{WALK_LIST}{MAIN}")).unwrap();
    let out = salvo_in(&dir, &["compile", "--backend", "rust", "--src", ".", "--target", "out"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a borrow assigned an owned value must be refused");
    assert!(
        stderr.contains("`k` holds a borrow (`proj Str?`), and this is an owned `Str?`") && stderr.contains("`let k = copy(…)` --> "),
        "{stderr}"
    );
}

/// [std-console] `readln` answers a line without its terminator, a `\r\n`
/// included, on both backends. The rest of `Console`'s three streams is run
/// data (`tests/run/pilot/outcomes.sv#console_streams`); a carriage return
/// cannot be written there.
#[test]
fn console_reads_a_crlf_line_on_each_backend() {
    const SRC: &str = r#"
fn main() [use] -> None {
    use StdConsole()
    print("name? ")
    let name = readln()
    if name is None {
        printerr("no input\n")
        return
    }
    println("hello ${name}")
    let second = readln()
    printerr("second: ${second ?: "<eof>"}\n")
}
"#;
    let dirs: Vec<PathBuf> = ["kotlin", "rust"].iter().map(|b| work_dir(&format!("console_streams_{b}"))).collect();
    for dir in &dirs {
        fs::write(dir.join("main.sv"), SRC).unwrap();
    }
    let Some(stamp) = cli_stamp(
        "console_streams",
        &["kotlinc", "rustc"],
        &[project(&dirs[0], &["--src", "."], &["kotlin"]), project(&dirs[1], &["--src", "."], &["rust"])],
    ) else {
        return;
    };
    for ((backend, tool), dir) in [("kotlin", "kotlinc"), ("rust", "rustc")].into_iter().zip(&dirs) {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_salvo"))
            .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
            .current_dir(dir)
            .args(["run", "--backend", backend, "--src", "."])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to run salvo");
        use std::io::Write as _;
        child.stdin.take().unwrap().write_all(b"Ada\r\n").unwrap();
        let out = child.wait_with_output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed, stderr: {err}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "name? hello Ada\n", "{backend} stdout (stderr: {err})");
        assert!(err.ends_with("second: <eof>\n"), "{backend} stderr: {err}");
    }
    stamp.verified();
}
