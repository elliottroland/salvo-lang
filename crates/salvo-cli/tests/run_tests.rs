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
    let Some(__stamp) = e2e_stamp("run_forwards_command_line_arguments_to_both_backends", &["kotlinc", "rustc"]) else {
        return;
    };
    // program name excluded, order kept, a value with a space preserved.
    let expected = "count: 3\nalpha\nbeta gamma\ndelta\n";
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("args_{backend}"));
        fs::write(dir.join("main.sv"), ARGS_PROGRAM).unwrap();
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
    let Some(__stamp) = e2e_stamp("build_produces_a_runnable_artifact_on_both_backends", &["kotlinc", "rustc"]) else {
        return;
    };
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("build_{backend}"));
        fs::write(dir.join("main.sv"), ARGS_PROGRAM).unwrap();
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
    let Some(__stamp) = e2e_stamp("init_then_run_works_with_no_flags", &["rustc"]) else {
        return;
    };
    if !have("rustc") {
        eprintln!("skipping: rustc not found on PATH");
        __stamp.verified();
        return;
    }
    let dir = work_dir("init_run");
    let out = salvo_in(&dir, &["init", "."]);
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));
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
    let Some(__stamp) = e2e_stamp("main_flag_selects_among_several_entry_points", &["kotlinc"]) else {
        return;
    };
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
        "fn main() [use] -> None {\n    use StdConsole\n    \
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

// ===== [default-predicate] =====

/// A default whose slot carries a predicate qualifier it does not satisfy is
/// tested where it is applied and traps there, on both backends (user
/// decision 2026-10-20, the CLI design record round 2 Q2): a parameter default that
/// holds runs, and a field default that does not stops the program at the
/// literal with a message naming the field and the claim. Before the fix the
/// default was not checked at all and printed 8080 through an `InRange(1,
/// 10) Int`.
#[test]
fn a_predicate_default_is_tested_where_it_is_applied() {
    let Some(__stamp) = e2e_stamp("a_predicate_default_is_tested_where_it_is_applied", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("default_predicate");
    fs::write(
        dir.join("main.sv"),
        "struct Opts {\n    port: InRange(1, 10) Int = 8080\n    ok: InRange(1, 10) Int = 3\n}\n\n\
         fn needs(p: InRange(1, 10) Int) [] -> Int => p { return p + 0 }\n\
         fn f(p: InRange(1, 10) Int = 5) [] -> Int => p { return needs(p) }\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    println(\"${f()}\")\n    \
         let o = Opts {}\n    println(\"${needs(o.ok)} ${needs(o.port)}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend}: the field default should trap; stdout: {stdout}");
        assert!(stdout.contains('5'), "{backend}: the parameter default holds and runs first; stdout: {stdout}");
        assert!(!stdout.contains("8080"), "{backend}: the bad default must not be read; stdout: {stdout}");
        assert!(
            stderr.contains("the default for field `port` is not `InRange(1, 10) Int`"),
            "{backend}: stderr: {stderr}"
        );
    }
    __stamp.verified();
}

/// [type-literal-qualified] A union of provenance-qualified literals compiles
/// and runs on both backends: the qualified literals collapse into `Str`
/// like bare ones, `is Short` compares against the literals carrying it, and
/// a `when` over the arms dispatches. Before 2026-10-20 both backends emitted
/// a two-arm union and constructed a third arm, and rustc and kotlinc
/// refused the output.
#[test]
fn a_union_of_qualified_literals_runs_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_union_of_qualified_literals_runs_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("qualified_literals");
    fs::write(
        dir.join("main.sv"),
        "provenance qualifier Short of Str\ntype Port = \"port\"\n\
         type FieldName = Short Port | Short \"verbose\" | \"host\"\n\n\
         fn describe(f: FieldName) [] -> Str => f {\n    if f is Short {\n        return \"short ${f}\"\n    }\n    return \"long ${f}\"\n}\n\n\
         fn code(f: FieldName) [] -> Int => f {\n    when f {\n        is Short { return 1 }\n        is \"host\" { return 2 }\n    }\n}\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    let h: FieldName = \"host\"\n    \
         println(describe(h))\n    println(\"${code(h)}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(stdout, "long host\n2\n", "{backend}");
    }
    __stamp.verified();
}

/// [fn-overload-return] Overloads that differ only in their return type,
/// chosen by where each value goes, compile and run on both backends: a
/// `let`, `!`, `?:`, a struct field, an argument and a value `if` branch.
/// Neither target overloads by return type, so each gets a name of its own
/// from [fn-emit-name]'s sixth level (`from_str__Str_IntOpt`; Kotlin camel-
/// cases the part before the `__`).
#[test]
fn return_type_overloads_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("return_type_overloads_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("return_overloads");
    fs::write(
        dir.join("main.sv"),
        "fn read_as(s: Str) [] -> Int? => s { return parse_int(s) }\n\
         fn read_as(s: Str) [] -> Bool? => s {\n    if s == \"true\" { return true }\n    \
         if s == \"false\" { return false }\n    return None\n}\n\n\
         struct Opts { n: Int?, b: Bool? }\n\n\
         fn want_int(n: Int?) [] -> Int => n { return n ?: 0 }\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    \
         let a: Int? = read_as(\"12\")\n    let b: Bool? = read_as(\"true\")\n    \
         let c: Int = read_as(\"7\") ?: unreachable!()\n    let d: Int = read_as(\"x\") ?: 5\n    \
         let o = Opts { n: read_as(\"3\"), b: read_as(\"false\") }\n    \
         let e = want_int(read_as(\"9\"))\n    \
         let f: Bool? = if c > 3 { read_as(\"true\") } else { None }\n    \
         println(\"${a ?: 0} ${b ?: false} ${c} ${d} ${o.n ?: 0} ${o.b ?: true} ${e} ${f ?: false}\")\n}\n",
    )
    .unwrap();
    for (backend, tool, ext) in [("rust", "rustc", "rs"), ("kotlin", "kotlinc", "kt")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(stdout, "12 true 7 5 3 false 9 true\n", "{backend}");
        let emitted: String = fs::read_dir(dir.join(".salvo_tmp_run").join(""))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == ext))
            .map(|e| fs::read_to_string(e.path()).unwrap_or_default())
            .collect();
        if !emitted.is_empty() {
            assert!(
                emitted.contains("__Str_IntOpt(") && emitted.contains("__Str_BoolOpt("),
                "{backend}: the two overloads are named by their return types"
            );
        }
    }
    __stamp.verified();
}

/// [cli-process] [cli-env] [rs-never-call] `exit` ends the process with its
/// status after flushing what was printed without a newline, from statement,
/// value and tail position; `env` answers a set variable and `None` for an
/// unset one. Built once per backend, then the artifact runs under three
/// environments, one per exit path.
#[test]
fn process_exit_and_env_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("process_exit_and_env_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
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
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("process_env_{backend}"));
        fs::write(dir.join("main.sv"), SRC).unwrap();
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

/// [default-scope] [comptime-default] A default is evaluated where it is
/// declared: a struct field's and a parameter's default that call a private
/// fn of their module work when applied in another module (they were an
/// "unresolved identifier" in the IR, which looked the default's facts up
/// under the using file), and a comptime walk in another module reads each
/// field's default with `field.default` — the declared value, tested
/// against its predicate qualifier — or `None` where there is none.
#[test]
fn defaults_are_evaluated_where_they_are_declared() {
    let Some(__stamp) = e2e_stamp("defaults_are_evaluated_where_they_are_declared", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("default_scope");
    fs::create_dir_all(dir.join("lib")).unwrap();
    fs::write(
        dir.join("lib").join("conf.sv"),
        "fn base_port() [] -> Int {\n    return 8000\n}\n\n\
         fn suffix() [] -> Str {\n    return \"!\"\n}\n\n\
         export fn wave(name: Str, punct: Str = suffix()) [] -> Str => name, punct {\n    return \"bye ${name}${punct}\"\n}\n\n\
         export struct Conf {\n    host: Str = \"127.0.0.1\",\n    port: InRange(1, 65535) Int = base_port() + 80,\n    \
         name: Str,\n    tag: Str? = None\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.sv"),
        "import lib.conf\n\n\
         fn show<T>(x: T?, ?to_str: (x: T) -> Str) [] -> Str => x {\n    if x is None {\n        return \"(none)\"\n    }\n    return to_str(x)\n}\n\n\
         comptime fn defaults() [] -> Str {\n    let out: Mut Str = Mut Str(\"\")\n    [for f in Conf.fields] {\n        \
         [if f.default is None] {\n            append(out, \"${f.name}: required\\n\")\n        } else {\n            \
         let d: [f.type]? = f.default\n            append(out, \"${f.name}: ${show(d)}\\n\")\n        }\n    }\n    return out\n}\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    let c = Conf { name: \"x\" }\n    \
         println(\"${c.port} ${wave(c.host)}\")\n    print(defaults())\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "8080 bye 127.0.0.1!\nhost: 127.0.0.1\nport: 8080\nname: required\ntag: (none)\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [qual-const] `Char` and `Str` constants run on both backends: an `is`
/// test passes the constant to `qualifies`, a predicate-qualified default
/// is tested with it where it is applied, and a failing one traps naming
/// the claim with its constant.
#[test]
fn char_and_str_qualifier_constants_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("char_and_str_qualifier_constants_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let src = |default: &str| {
        format!(
            "qualifier Prefix(p: Str) of Str {{\n    fn qualifies(s: Str, p: Str) -> Bool {{\n        return starts_with(s, p)\n    }}\n}}\n\n\
             qualifier Initial(c: Char) of Str {{\n    fn qualifies(s: Str, c: Char) -> Bool {{\n        return char_at(s, 0) == c\n    }}\n}}\n\n\
             fn needs(a: Prefix(\"arn:\") Str) [] -> Str {{\n    return \"ok ${{a}}\"\n}}\n\n\
             struct Opts {{\n    name: Initial('p') Str = \"{default}\",\n    arn: Prefix(\"arn:\") Str = \"arn:aws:s3\"\n}}\n\n\
             fn main() [use] -> None {{\n    use StdConsole()\n    let x = \"arn:aws:sqs\"\n    \
             if x is Prefix(\"arn:\") {{\n        println(needs(x))\n    }}\n    let y = \"http\"\n    \
             println(\"${{y is Prefix(\"arn:\")}} ${{y is Initial('h')}}\")\n    let o = Opts {{}}\n    \
             println(\"${{o.name}} ${{needs(o.arn)}}\")\n}}\n"
        )
    };
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("const_slots_{backend}"));
        fs::write(dir.join("main.sv"), src("port")).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "ok arn:aws:sqs\nfalse true\nport ok arn:aws:s3\n", "{backend}");
        fs::write(dir.join("main.sv"), src("quit")).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend}: a failing default must trap");
        assert!(stderr.contains("the default for field `name` is not `Initial('p') Str`"), "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [comptime-qualifier] Comptime qualifiers end to end: a comptime fn reads
/// a struct's own (`Named`), each field's (`Short('p')`, `Positional`) and a
/// field type's qualifier constants, and the struct itself compiles and runs
/// with the qualifiers erased, on both backends.
#[test]
fn comptime_qualifiers_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("comptime_qualifiers_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("comptime_qualifiers");
    fs::write(
        dir.join("main.sv"),
        "comptime qualifier Short(c: Char) of Field\ncomptime qualifier Positional of Field\n\
         comptime qualifier Named(name: Str) of Struct\n\n\
         Named(\"srv\")\nstruct Serve {\n    Short('p') port: InRange(1, 65535) Int = 8080,\n    host: Str = \"127.0.0.1\",\n    \
         Positional files: List<Str>\n}\n\n\
         comptime fn help() [] -> Str {\n    let out: Mut Str = Mut Str(\"\")\n    \
         [if Serve is <n> Named(n)] {\n        append(out, \"command ${n}\\n\")\n    }\n    \
         [for f in Serve.fields] {\n        [if f is <c> Short(c)] {\n            append(out, \"-${c} \")\n        }\n        \
         [if f is Positional] {\n            append(out, \"<${f.name}>\")\n        } else {\n            append(out, \"--${f.name}\")\n        }\n        \
         [if f.type is <lo, hi> InRange(lo, hi) *] {\n            append(out, \" (${lo}..${hi})\")\n        }\n        \
         append(out, \"\\n\")\n    }\n    return out\n}\n\n\
         fn main() [use] -> None {\n    use StdConsole()\n    print(help())\n    \
         let s = Serve { files: List(\"a\") }\n    println(\"${s.port} ${s.host}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "command srv\n-p --port (1..65535)\n--host\n<files>\n8080 127.0.0.1\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [comptime-arm] A comptime fn builds every kind of union arm — a struct arm
/// field by field, a qualified arm through its claim's constructor, `None` —
/// and the values run identically on both backends.
#[test]
fn comptime_builds_each_arm_on_both_backends() {
    let Some(__stamp) = e2e_stamp("comptime_builds_each_arm_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("comptime_arms");
    fs::write(dir.join("main.sv"), "struct A { n: Int, s: Str }\nstruct B { m: Int }\ntype Pick = A | B | Ok Str | None\n\nfn make() [] -> Int {\n    return 7\n}\n\nfn make() [] -> Str {\n    return \"s\"\n}\n\ncomptime fn build(i: Int) [] -> Pick => i {\n    [for arm in Pick.arms] {\n        if i == arm.index {\n            [if arm.kind is Struct] {\n                return [arm.type] { [for f in arm.type.fields] { [f]: make() } }\n            }\n            [if arm.type is Ok *] {\n                return [arm.type](make())\n            }\n            [if arm.type is None] {\n                return None\n            }\n        }\n    }\n    return None\n}\n\nfn show(p: Pick) [] -> Str => p {\n    when p {\n        is A { return \"A ${p.n} ${p.s}\" }\n        is B { return \"B ${p.m}\" }\n        is Ok { return \"Ok ${p}\" }\n        is None { return \"None\" }\n    }\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    for i in List(0, 1, 2, 3) {\n        println(show(build(i)))\n    }\n}\n").unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "A 7 s\nB 7\nOk s\nNone\n", "{backend}");
    }
    __stamp.verified();
}

/// [comptime-literals] A literal union's members walked in comptime: listed
/// with their positions, and matched to decode a string into the union, on
/// both backends.
#[test]
fn literal_members_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("literal_members_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("literal_members");
    fs::write(dir.join("main.sv"), "type Level = \"debug\" | \"info\" | \"warn\"\n\nstruct Opts {\n    output: \"json\" | \"text\" = \"text\",\n    level: Level = \"info\",\n    n: 1 | 2 | 3 = 1\n}\n\n\ncomptime fn choices() [] -> Str {\n    let out: Mut Str = Mut Str(\"\")\n    [for f in Opts.fields] {\n        append(out, \"${f.name}:\")\n        [for lit in f.type.literals] {\n            append(out, \" ${lit.value}\")\n            [if lit.last] {\n                append(out, \" (${lit.index + 1})\")\n            }\n        }\n        append(out, \"\\n\")\n    }\n    [for lit in Level.literals] {\n        [if lit.first] {\n            append(out, \"first ${lit.value}\\n\")\n        }\n    }\n    return out\n}\n\n\ncomptime fn parse_output(s: Str) [] -> (\"json\" | \"text\")? => s {\n    [for f in Opts.fields] {\n        [if f.name is \"output\"] {\n            [for lit in f.type.literals] {\n                if s == lit.value {\n                    return lit.value\n                }\n            }\n        }\n    }\n    return None\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    print(choices())\n    let picked = parse_output(\"json\")\n    if picked !is None {\n        let o = Opts { output: picked }\n        println(\"${o.output} ${parse_output(\"xml\") is None}\")\n    }\n}\n").unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "output: json text (2)\nlevel: debug info warn (3)\nn: 1 2 3 (3)\nfirst debug\njson true\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [is-union] [type-literal] [type-rewrap] `s is "json" | "text"` narrows a `Str` to
/// the literal union (into a field of that type), `x is Int | Str` and `x !is
/// Bool` narrow to the sub-union (into a parameter of it), and a literal test
/// on a plain `Str` compares the value — it used to pass for every string —
/// on both backends.
#[test]
fn union_is_tests_narrow_on_both_backends() {
    let Some(__stamp) = e2e_stamp("union_is_tests_narrow_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("union_is");
    fs::write(dir.join("main.sv"), "struct Opts {\n    output: \"json\" | \"text\" = \"text\"\n}\n\nfn wants(v: Int | Str) [] -> Str => v {\n    when v {\n        is Int { return \"int ${v}\" }\n        is Str { return \"str ${v}\" }\n    }\n}\n\nfn describe(x: Int | Str | Bool) [] -> Str => x {\n    if x is Int | Str {\n        return wants(x)\n    }\n    return \"bool\"\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    for s in List(\"json\", \"xml\", \"text\") {\n        if s !is \"json\" | \"text\" {\n            println(\"refused ${s}\")\n        }\n        if s is \"json\" | \"text\" {\n            let o = Opts { output: s }\n            println(\"ok ${o.output}\")\n        }\n    }\n    println(\"${describe(3)} ${describe(\"a\")} ${describe(true)} ${rest(4)} ${rest(false)}\")\n    println(\"${lit(\"json\")}${lit(\"text\")}${lit(\"xml\")}\")\n}\n\nfn rest(x: Int | Str | Bool) [] -> Str => x {\n    if x !is Bool {\n        return wants(x)\n    }\n    return \"bool\"\n}\n\nfn lit(s: Str) [] -> Str => s {\n    when {\n        s is \"json\" { return \"J\" }\n        s is \"text\" { return \"T\" }\n        else { return \"?\" }\n    }\n}\n").unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "ok json\nrefused xml\nok text\nint 3 str a bool int 4 bool\nJT?\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [qual-whynot] A predicate qualifier says why a value failed: `whynot?`
/// answers `None` or the first failed check's message — its `@whynot`
/// template filled, or "<value> is not Q(args)" — and an `assert!(x is Q)`
/// with no message of its own traps with the same text. [args-explain] A
/// derived parser reports a field's failed qualifier the same way, a user
/// qualifier as well as std's `InRange`. On both backends.
#[test]
fn whynot_messages_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("whynot_messages_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("whynot");
    const SRC: &str = r#"import std.args

qualifier Even of Int {
    // @whynot [n] is odd
    fn qualifies(n: Int) -> Bool {
        return n % 2 == 0
    }
}

qualifier Small(hi: Int) of Int {
    fn qualifies(n: Int, hi: Int) -> Bool {
        return n < hi
    }
}

qualifier Brief(most: Int) of Str {
    // @whynot "[s]" is longer than [most] characters
    fn qualifies(s: Str, most: Int) -> Bool {
        return size(s) <= most
    }
}

struct P {
    x: Int
}

qualifier Origin of P {
    fn qualifies(p: P) -> Bool {
        return p.x == 0
    }
}

struct Opts : FromArgs<self> by args {
    count: Even Int = 2,
    name: Brief(3) Str = "abc"
}

fn seven() [] -> Int {
    return 7
}

fn show(m: Str?) [Console] -> None => m {
    if m is None {
        println("ok")
    } else {
        println(m)
    }
}

fn main() [use] -> None {
    use StdConsole()
    let n = 70000
    let lim = 5
    show(whynot?(n is InRange(1, 65535) Int))
    show(whynot?(n is Even InRange(1, 65535) Int))
    show(whynot?(seven() is Even Int))
    show(whynot?(n is Small(10) Int))
    show(whynot?(n is Small(lim) Int))
    show(whynot?(n is Int))
    show(whynot?(4 is Even Int))
    let p = P { x: 3 }
    show(whynot?(p is Origin P))
    show(whynot?("long" is Brief(3) Str))
    let r: Ok Opts | Errs NonEmpty List<ArgError> = parse_args(List("--count", "3", "--name", "abcd"))
    when r {
        is Ok { println("parsed") }
        is Errs { for e in r { println(message(copy(e))) } }
    }
    let x = 70000
    assert!(x is InRange(1, 65535))
}
"#;
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend}: the assert must trap: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "70000 is not in 1..65535\n70000 is not in 1..65535\n7 is odd\n70000 is not Small(10)\n70000 is not Small(lim)\nok\nok\n\
             the value is not Origin\n\"long\" is longer than 3 characters\n\
             invalid value '3' for '--count': 3 is odd\ninvalid value 'abcd' for '--name': \"abcd\" is longer than 3 characters\n",
            "{backend}: {stderr}"
        );
        assert!(stderr.contains("salvo: 70000 is not in 1..65535 at main:"), "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [kt-stdlib-qualified] A program may name its own fns and types after the
/// Kotlin stdlib functions the emitter calls — `run` (every block in value
/// position), `error`, `check`, `Pair` (tuples) — without the generated code
/// calling the program's instead: each is written qualified. Before
/// 2026-10-20 the elvis below recursed into the program's `run` and
/// overflowed the stack on Kotlin. On both backends.
#[test]
fn kotlin_stdlib_names_do_not_capture_generated_calls_on_both_backends() {
    let Some(__stamp) = e2e_stamp("kotlin_stdlib_names_do_not_capture_generated_calls_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("ktnames");
    const SRC: &str = r#"struct Pair {
    a: Int
}

struct Triple {
    b: Int
}

fn run(f: (v: Int) -> Str?) [] -> Str => f {
    return f(1) ?: "none"
}

fn error(s: Str) [] -> Str => s {
    return "mine: ${s}"
}

fn check(b: Bool) [] -> Bool {
    return !b
}

fn main() [use] -> None {
    use StdConsole()
    println(run((v: Int) -> None))
    println(error("x"))
    println("${check(true)}")
    let (n, s) = (Pair { a: 1 }.a, "two")
    let (x, y, z) = (Triple { b: 3 }.b, 4, 5)
    println("${n} ${s} ${x + y + z}")
}
"#;
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "none\nmine: x\nfalse\n1 two 12\n", "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [type-identity] [net-wire-format] Two modules declare `Token`: `alpha`'s, which
/// holds the plain key, can cross the wire, and `beta`'s cannot. `beta`'s
/// `Holder { t: Token }` must be judged by `beta`'s own `Token`, so it gets no
/// codec. Before 2026-10-20 the wire predicate read the field by its written
/// name, found `alpha`'s, and gave `Holder` a codec that asked for one
/// `beta.Token` never had: no such `__Codec_Token` on Kotlin, no `__Wire` impl
/// on Rust (ROADMAP §2d item 8). On both backends.
#[test]
fn a_clashing_type_name_is_judged_by_its_own_declaration_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_clashing_type_name_is_judged_by_its_own_declaration_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("wirekeys");
    fs::write(dir.join("alpha.sv"), "export struct Word {\n    w: Str\n}\n\nexport type Token = Word | Int\n").unwrap();
    fs::write(
        dir.join("beta.sv"),
        "export noremote struct Token {\n    n: Int\n}\n\nexport struct Holder {\n    t: Token\n}\n\nexport fn make(n: Int) [] -> Holder {\n    return Holder { t: Token { n: n } }\n}\n\nexport fn read(h: Holder) [] -> Int => h {\n    return h.t.n\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.sv"),
        "import alpha.Token\nimport alpha.Word\nimport beta.make\nimport beta.read\n\nfn show(t: Token) [] -> Str => t {\n    if t is Word {\n        return t.w\n    }\n    return \"${t}\"\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    let t: Token = Word { w: \"hi\" }\n    println(\"${show(t)} ${read(make(7))}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--main", "main.sv", "--clean-target", "before"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "hi 7\n", "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [is-union] [type-literal] `s is "json"` on an un-narrowed `Str?` tests the
/// `Str` arm for that value, and narrows `s` to the literal inside. It was
/// refused as "this check can never succeed" until 2026-10-20 (ROADMAP §2d
/// item 13): the narrowing matched the literal against the arms and found
/// none, though the runtime test was already recorded. On both backends.
#[test]
fn a_literal_test_on_an_optional_str_runs_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_literal_test_on_an_optional_str_runs_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("litopt");
    fs::write(
        dir.join("main.sv"),
        "fn pick(s: Str?) [] -> Str => !s {\n    if s is \"json\" {\n        return s\n    }\n    if s is None {\n        return \"none\"\n    }\n    return \"other ${s}\"\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    println(pick(\"json\"))\n    println(pick(\"x\"))\n    println(pick(None))\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "json\nother x\nnone\n", "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [rs-borrow-locals] [rs-opt-borrow] [proj-local] A local bound from a kept
/// `Str?` parameter and later reassigned owns its value from the start: Rust
/// stored it as the parameter's `Option<&String>`, and the assignment of an
/// owned `Option<String>` was E0308 (ROADMAP §2d item 11). A local whose type
/// is a projection (`let k = get(xs, 0)`) is a borrow for its whole life:
/// assigning it an owned value is refused, with `copy` at the declaration
/// the remedy (user decision 2026-10-20), which then runs on both backends.
#[test]
fn a_reassigned_local_owns_or_borrows_for_its_whole_life_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_reassigned_local_owns_or_borrows_for_its_whole_life_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const UP: &str = "fn up(s: Str) [] -> Str? {\n    if size(s) > 1 {\n        return substr(s, 1, size(s))\n    }\n    return None\n}\n\n";
    const WALK: &str = "fn walk(key: Str?) [] -> Int => key {\n    let k = key\n    let n = 0\n    while k !is None {\n        let here = copy(k)\n        n = n + 1\n        k = up(here)\n    }\n    return n\n}\n\n";
    let walk_list = |bind: &str| format!("fn walk_list(xs: List<Str>) [] -> Int => xs {{\n    let k = {bind}\n    let n = 0\n    while k !is None {{\n        let here = copy(k)\n        n = n + 1\n        k = up(here)\n    }}\n    return n\n}}\n\n");
    const MAIN: &str = "fn main() [use] -> None {\n    use StdConsole()\n    println(\"${walk(\"abcd\")} ${walk_list(List(\"xyz\"))}\")\n}\n";
    let refused = work_dir("projlocal_refused");
    fs::write(refused.join("main.sv"), format!("{UP}{WALK}{}{MAIN}", walk_list("get(xs, 0)"))).unwrap();
    let out = salvo_in(&refused, &["compile", "--backend", "rust", "--src", ".", "--target", "out"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a borrow assigned an owned value must be refused");
    assert!(
        stderr.contains("`k` holds a borrow (`proj Str?`), and this is an owned `Str?`") && stderr.contains("`let k = copy(…)` [proj-local]"),
        "{stderr}"
    );
    let dir = work_dir("projlocal");
    fs::write(dir.join("main.sv"), format!("{UP}{WALK}{}{MAIN}", walk_list("copy(get(xs, 0))"))).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "4 3\n", "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [comptime-hygiene] A stamped body resolves a name the stamping site cannot
/// in the template's own file, private declarations included (user decision
/// 2026-10-20, ROADMAP §2d item 10 (b)): `deriv`'s private `shout` serves
/// `Point`'s stamped `label`, though `main` cannot see it. A name the site
/// does resolve is the site's: with its own `shout`, `main`'s is called, and
/// the two never meet in an ambiguity the site did not write. On both
/// backends.
#[test]
fn a_stamped_body_falls_back_to_its_templates_names_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_stamped_body_falls_back_to_its_templates_names_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const DERIV: &str = "export fns Labelled<T> {\n    fn label(value: T) -> Str\n}\n\nexport comptime fn label<T is Struct>(value: T) [] -> Str => value {\n    return shout(T.name)\n}\n\nfn shout(s: Str) [] -> Str => s {\n    return \"${s}!\"\n}\n";
    const MAIN: &str = "import deriv\n\nstruct Point : Labelled<self> by deriv {\n    x: Int\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    println(label(Point { x: 1 }))\n}\n";
    const OWN: &str = "\nfn shout(s: Str) [] -> Str => s {\n    return \"mine: ${s}\"\n}\n";
    for (case, main, want) in [("fallback", MAIN.to_string(), "Point!\n"), ("site", format!("{MAIN}{OWN}"), "mine: Point\n")] {
        let dir = work_dir(&format!("hygiene_{case}"));
        fs::write(dir.join("deriv.sv"), DERIV).unwrap();
        fs::write(dir.join("main.sv"), main).unwrap();
        for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
            if !have(tool) {
                eprintln!("skipping {backend}: {tool} not found on PATH");
                continue;
            }
            let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--main", "main.sv", "--clean-target", "before"]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(out.status.success(), "{case} on {backend} failed: {stderr}");
            assert_eq!(String::from_utf8_lossy(&out.stdout), want, "{case} on {backend}: {stderr}");
        }
    }
    __stamp.verified();
}

/// [type-result-errs] The accumulating outcome keeps every error: `map2` of two
/// successes, of two failures (both errors, in order), and of one failure,
/// on both backends.
#[test]
fn accumulated_errors_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("accumulated_errors_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("errs");
    fs::write(dir.join("main.sv"), "fn port(s: Str) [] -> Ok Int | Errs NonEmpty List<Str> => s {\n    let n = parse_int(s)\n    if n is None {\n        return errs(\"not a port: ${s}\")\n    }\n    return Ok(n)\n}\n\nfn name(s: Str) [] -> Ok Str | Errs NonEmpty List<Str> => !s {\n    if size(s) == 0 {\n        return errs(\"empty name\")\n    }\n    return Ok(s)\n}\n\nfn show(o: Ok Str | Errs NonEmpty List<Str>) [] -> Str => o {\n    when o {\n        is Ok { return \"ok ${o}\" }\n        is Errs { return \"errors ${o}\" }\n    }\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    println(show(map2(port(\"80\"), name(\"web\"), (p: Int, n: Str) -> \"${n}:${p}\")))\n    println(show(map2(port(\"x\"), name(\"\"), (p: Int, n: Str) -> \"${n}:${p}\")))\n    println(show(map2(port(\"x\"), name(\"ok\"), (p: Int, n: Str) -> \"${n}:${p}\")))\n}\n").unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "ok web:80\nerrors [not a port: x, empty name]\nerrors [not a port: x]\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [obligation-return] A decoder stamped by an obligation clause: two structs
/// each get `from_strs(args: List<Str>) -> S`, which the expected type
/// chooses between (each field read with `from_str`, chosen by the field's
/// type), on both backends.
#[test]
fn a_stamped_decoder_runs_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_stamped_decoder_runs_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("stamped_decoder");
    fs::write(dir.join("decode.sv"), "export fns FromStrs<T> {\n    fn from_strs(args: List<Str>) -> T\n}\n\nexport comptime fn from_strs<T is Struct>(args: List<Str>) [] -> T => args {\n    return T { [for f in T.fields] { [f]: from_str(get(args, f.index) ?: unreachable!()) ?: unreachable!() } }\n}\n").unwrap();
    fs::write(dir.join("main.sv"), "import decode\n\nstruct Serve : FromStrs<self> by decode {\n    port: Int,\n    host: Str\n}\n\nstruct Stop : FromStrs<self> by decode {\n    force: Bool\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    let s: Serve = from_strs(List(\"80\", \"web\"))\n    let t: Stop = from_strs(List(\"true\"))\n    println(\"${s.port} ${s.host} ${t.force}\")\n}\n").unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "80 web true\n", "{backend}");
    }
    __stamp.verified();
}

/// [args-derive] [comptime-is-splice] [comptime-pattern] A struct's command
/// line, derived by `: FromArgs<self> by args`: switches, defaults, `Short`
/// bundles, choices from literal unions (optional too), optional and
/// repeated options, `Double`/`Long` values, a qualified `InRange` port
/// explained with `whynot?(v is [field.type])`, positionals with `NonEmpty`, every
/// error at once, and help with a `Named` command's name, on both backends.
#[test]
fn a_derived_parser_reads_a_command_line_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_derived_parser_reads_a_command_line_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.args

// Serve files.
struct Serve : FromArgs<self> by args {
    // Address to bind.
    host: Str = "127.0.0.1",
    Short('p') port: InRange(1, 65535) Int = 8080,
    output: "json" | "text" = "text",
    level: ("low" | "high")?,
    Short('v') verbose: Bool,
    workers: Int?,
    ratio: Double = 0.5,
    big: Long?,
    tag: List<Str>,
    Positional files: NonEmpty List<Str>
}

// Remove a file.
Named("rm")
struct Remove : FromArgs<self> by args {
    Short('f') force: Bool,
    // What to remove.
    Positional path: Str,
    Positional backup: Str?
}

fn level(l: ("low" | "high")?) [] -> Str => !l {
    if l is None {
        return "-"
    }
    return l
}

fn serve(args: List<Str>) [Console] -> None => args {
    let r: Ok Serve | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok { println("ok ${r.host} ${r.port} ${r.output} ${level(r.level)} ${r.verbose} ${r.workers ?: 0} ${r.ratio} ${r.big ?: 0L} [${join(r.tag, ",")}] [${join(r.files, ",")}]") }
        is Errs {
            for e in r {
                println("error: ${message(copy(e))}")
            }
        }
    }
}

fn remove(args: List<Str>) [Console] -> None => args {
    let r: Ok Remove | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok { println("ok ${r.force} ${r.path} ${r.backup ?: "-"}") }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    serve(List("a"))
    serve(List("-vp80", "--output=json", "--level", "high", "--workers", "4", "--ratio", "-1.5", "--big", "9000000000", "--tag", "x", "--tag", "y", "a", "b"))
    serve(List("-p", "70000", "--ratio", "x", "--level", "mid", "--big", "1.5", "--nope"))
    remove(List("-f", "x"))
    remove(List("x", "y", "z"))
    remove(List("-h"))
}
"#;
    const EXPECTED: &str = r#"ok 127.0.0.1 8080 text - false 0 0.5 0 [] [a]
ok 127.0.0.1 80 json high true 4 -1.5 9000000000 [x,y] [a,b]
error: unknown option '--nope'
error: invalid value 'mid' for '--level': expected one of low, high
error: '<files>' is required
error: invalid value '70000' for '--port': 70000 is not in 1..65535
error: invalid value 'x' for '--ratio': expected a number
error: invalid value '1.5' for '--big': expected an integer
ok true x -
error: unexpected argument 'z'
Remove a file.

Usage: rm [OPTIONS] <path> [<backup>]

Arguments:
  <path>       What to remove.
  <backup>

Options:
  -f, --force
  -h, --help   Print help
"#;
    let dir = work_dir("derived_parser");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), EXPECTED, "{backend}");
    }
    __stamp.verified();
}

/// [args-derive] `args_or_exit` prints help to standard output with status 0,
/// and every error to standard error with the usage line and status 2.
#[test]
fn args_or_exit_ends_the_process_with_help_or_errors_on_both_backends() {
    let Some(__stamp) = e2e_stamp("args_or_exit_ends_the_process_with_help_or_errors_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.args
import std.process

// Copy a file.
struct Copy : FromArgs<self> by args {
    // Overwrite what is there.
    Short('f') force: Bool,
    Positional from: Str,
    Positional to: Str
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    let c: Copy = args_or_exit(args())
    println("${c.force} ${c.from} ${c.to}")
}
"#;
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("args_or_exit_{backend}"));
        fs::write(dir.join("main.sv"), SRC).unwrap();
        let out = salvo_in(&dir, &["build", "--backend", backend, "--src", ".", "--out", "out"]);
        assert!(out.status.success(), "{backend} build failed: {}", String::from_utf8_lossy(&out.stderr));
        let run = |args: &[&str]| Command::new(dir.join("out").join("main")).args(args).output().expect("failed to run the artifact");
        let ok = run(&["-f", "a", "b"]);
        assert_eq!(ok.status.code(), Some(0), "{backend}");
        assert_eq!(String::from_utf8_lossy(&ok.stdout), "true a b\n", "{backend}");
        let help = run(&["--help"]);
        assert_eq!(help.status.code(), Some(0), "{backend}");
        assert_eq!(
            String::from_utf8_lossy(&help.stdout),
            "Copy a file.\n\nUsage: copy [OPTIONS] <from> <to>\n\nArguments:\n  <from>\n  <to>\n\nOptions:\n  -f, --force  Overwrite what is there.\n  -h, --help   Print help\n",
            "{backend}"
        );
        let bad = run(&["-x", "a"]);
        assert_eq!(bad.status.code(), Some(2), "{backend}");
        assert_eq!(String::from_utf8_lossy(&bad.stdout), "", "{backend}");
        assert_eq!(
            String::from_utf8_lossy(&bad.stderr),
            "error: unknown option '-x'\nerror: '<to>' is required\n\nUsage: copy [OPTIONS] <from> <to>\n\nFor more information, try '--help'.\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [ir-narrow] An `else` reads a name its branch's test narrowed as what
/// remains, even inside an enclosing `when` arm: the else of `if one is A`
/// in an `is Errs` arm used that arm's index (a `ClassCastException` on
/// Kotlin, a type error on Rust). And a kept lambda's capture of a borrowed
/// parameter is an owned clone, passed on borrowed (a type error on Rust).
#[test]
fn an_else_narrows_by_its_own_test_inside_a_when_arm_on_both_backends() {
    let Some(__stamp) = e2e_stamp("an_else_narrows_by_its_own_test_inside_a_when_arm_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"struct A { x: Str }
struct B { y: Str }
struct C { z: Str }
type U = A | B | C

fn msg(u: U) [] -> Str => u {
    when u {
        is A { return "a ${u.x}" }
        is B { return "b ${u.y}" }
        is C { return "c ${u.z}" }
    }
}

fn make() [] -> Ok Int | Errs NonEmpty List<U> {
    let b: U = B { y: "1" }
    let c: U = C { z: "2" }
    let a: U = A { x: "3" }
    let all: Mut List<U> = Mut List<U>()
    add(all, b)
    add(all, c)
    add(all, a)
    if all is NonEmpty {
        return Errs(all)
    }
    return Ok(1)
}

struct Found { values: List<Str> }

fn eval<Q>(f: once () -> Q) [] -> Q => !f {
    return f()
}

fn pick(found: Found, i: Int) [] -> Str => found {
    return copy(get(found.values, i) ?: unreachable!())
}

fn both(found: Found) [] -> Str => found {
    let a: Str = eval(() -> {
        return pick(found, 0)
    })
    let b: Str = eval(() -> {
        return pick(found, 1)
    })
    return "${a}${b}"
}

fn main() [use] -> None {
    use StdConsole()
    let r = make()
    when r {
        is Ok { println("ok") }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is A {
                    println(one.x)
                } else {
                    println(msg(one))
                }
            }
        }
    }
    println(both(Found { values: List("x", "y") }))
}
"#;
    let dir = work_dir("else_narrow");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "b 1\nc 2\n3\nxy\n", "{backend}");
    }
    __stamp.verified();
}

/// [args-groups] Groups of options, each a struct with its own `FromArgs`:
/// flattened (`net: Net`, its options the command's own, under a heading),
/// all-or-none (`tls: Tls?`: `--cert` requires `--key`), and exclusive
/// (`source: FromUrl | FromFile | FromStdin`: chosen by which arm's options
/// appear, an empty arm by a switch named after it, one required), on both
/// backends; and a clash between a group's option and the command's is
/// refused as the parser starts.
#[test]
fn derived_option_groups_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("derived_option_groups_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.args

// Network options.
struct Net : FromArgs<self> by args {
    // Address to bind.
    host: Str = "127.0.0.1"
    Short('p') port: Int = 8080
}

// TLS. Give both or neither.
struct Tls : FromArgs<self> by args {
    // Certificate file.
    cert: Str
    // Private key file.
    key: Str
    ca: Str?
}

struct FromUrl : FromArgs<self> by args {
    url: Str
    timeout: Int = 30
}

struct FromFile : FromArgs<self> by args {
    file: Str
}

// Read standard input.
struct FromStdin : FromArgs<self> by args {}

// Fetch something.
struct Fetch : FromArgs<self> by args {
    net: Net
    tls: Tls?
    // Where to read.
    source: FromUrl | FromFile | FromStdin
    output: "json" | "text" = "text"
}

fn show(args: List<Str>) [Console] -> None => args {
    let r: Ok Fetch | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok {
            let src = r.source
            let s = when src {
                is FromUrl { "url ${src.url} ${src.timeout}" }
                is FromFile { "file ${src.file}" }
                is FromStdin { "stdin" }
            }
            let t = r.tls
            let tls = if t is None { "-" } else { "${t.cert}+${t.key}+${t.ca ?: "-"}" }
            println("ok ${r.net.host}:${r.net.port} ${tls} ${s} ${r.output}")
        }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    show(List("--url", "http://x"))
    show(List("-p", "9", "--cert", "c", "--key", "k", "--file", "f", "--output", "json"))
    show(List("--from-stdin", "--timeout", "5"))
    show(List("--cert", "c", "--url", "u", "--file", "f"))
    show(List("--host", "h"))
    show(List("--help"))
}
"#;
    const EXPECTED: &str = r#"ok 127.0.0.1:8080 - url http://x 30 text
ok 127.0.0.1:9 c+k+- file f json
error: '--timeout' requires '--url'
error: '--from-stdin' cannot be used with '--timeout'
error: '--cert' requires '--key'
error: '--file' cannot be used with '--url'
error: one of '--url', '--file', '--from-stdin' is required
Fetch something.

Usage: fetch [OPTIONS]

Options:
      --output <OUTPUT>    [possible values: json, text] [default: text]
  -h, --help               Print help

Network options:
      --host <HOST>        Address to bind. [default: 127.0.0.1]
  -p, --port <PORT>        [default: 8080]

TLS. Give both or neither:
      --cert <CERT>        Certificate file.
      --key <KEY>          Private key file.
      --ca <CA>

Where to read:
      --url <URL>
      --timeout <TIMEOUT>  [default: 30]
      --file <FILE>
      --from-stdin         Read standard input.
"#;
    const CLASH: &str = r#"import std.args

struct Net : FromArgs<self> by args {
    host: Str = "a"
}

struct Cli : FromArgs<self> by args {
    net: Net
    host: Str = "b"
}

fn main() [use] -> None {
    use StdConsole()
    let r: Ok Cli | Errs NonEmpty List<ArgError> = parse_args(List<Str>())
    println("parsed ${r is Ok}")
}
"#;
    let dir = work_dir("option_groups");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    let clash = work_dir("option_group_clash");
    fs::write(clash.join("main.sv"), CLASH).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), EXPECTED, "{backend}");
        let out = salvo_in(&clash, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(!out.status.success(), "{backend}: a clash should trap");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("two options of `cli` are spelled `--host`"), "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [args-commands] Sub-commands: a `Subcommand` field over a union of
/// command structs, chosen by the first word (named by kebab case, the last
/// segment of a namespaced name, or `Named`), nested to two levels, optional
/// (`(A | B)?`), with the parent's options only before the word, an unknown
/// or missing command an error, and help at each level naming the path
/// (`git remote add`), on both backends.
#[test]
fn derived_sub_commands_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("derived_sub_commands_run_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.args

// Manage a repository.
struct Git : FromArgs<self> by args {
    // Print more.
    Short('v') verbose: Bool
    Subcommand command: GitCommand
}

type GitCommand = Clone | Remote

// Copy a repository.
struct Clone : FromArgs<self> by args {
    // Where from.
    Positional url: Str
    depth: Int?
}

// Manage remotes.
struct Remote : FromArgs<self> by args {
    Subcommand command: (Remote.Add | Remote.Remove)?
}

namespace Remote {
    // Add a remote.
    struct Add : FromArgs<self> by args {
        Positional name: Str
        Positional url: Str
    }

    // Remove a remote.
    Named("rm")
    struct Remove : FromArgs<self> by args {
        Positional name: Str
    }
}

fn describe(g: Git) [] -> Str => g {
    let c = g.command
    when c {
        is Clone { return "clone ${c.url} ${c.depth ?: 0}" }
        is Remote {
            let sub = c.command
            when sub {
                is Remote.Add { return "remote add ${sub.name} ${sub.url}" }
                is Remote.Remove { return "remote rm ${sub.name}" }
                is None { return "remote" }
            }
        }
    }
}

fn show(args: List<Str>) [Console] -> None => args {
    let r: Ok Git | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok { println("ok ${r.verbose} ${describe(r)}") }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    show(List("-v", "clone", "URL", "--depth", "1"))
    show(List("remote", "add", "origin", "u"))
    show(List("remote", "rm", "origin"))
    show(List("remote"))
    show(List("clone", "-v"))
    show(List("push"))
    show(List())
    show(List("--help"))
    show(List("remote", "add", "--help"))
}
"#;
    const EXPECTED: &str = r#"ok true clone URL 1
ok false remote add origin u
ok false remote rm origin
ok false remote
error: unknown option '-v'
error: '<url>' is required
error: unrecognized command 'push'
error: '<command>' is required
Manage a repository.

Usage: git [OPTIONS] <COMMAND>

Commands:
  clone          Copy a repository.
  remote         Manage remotes.

Options:
  -v, --verbose  Print more.
  -h, --help     Print help
Add a remote.

Usage: git remote add <name> <url>

Arguments:
  <name>
  <url>

Options:
  -h, --help  Print help
"#;
    let dir = work_dir("sub_commands");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), EXPECTED, "{backend}");
    }
    __stamp.verified();
}

/// [fn-value] A function passed by name and one filling an implicit are one
/// rule: the constructor's gained claim (`-> +Valid Range`) reaches both, a
/// function with implicits of its own (`twice`, `?double`) is passed by name
/// with them filled as it fills an implicit, and so is a stamped generic `eq`
/// for `Box<Str>`, on both backends.
#[test]
fn a_fn_value_is_one_rule_by_name_and_as_an_implicit_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_fn_value_is_one_rule_by_name_and_as_an_implicit_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"struct Range { min: Int, max: Int }

qualifier Valid of Range

fn validate(o: Range) [] -> +Valid Range => !o {
    return o
}

fn by_name<V>(o: Range, f: (o: Range) -> V) [] -> V =>[f] !o => !o, f {
    return f(o)
}

fn by_implicit<V>(o: Range, ?validate: (o: Range) -> V) [] -> V =>[validate] !o => !o {
    return validate(o)
}

fn width(r: Valid Range) [] -> Int => r {
    return r.max - r.min
}

fn double(n: Int) [] -> Int {
    return n * 2
}

fn twice<T>(x: T, ?double: (n: T) -> T) [] -> T => x {
    return double(double(x))
}

fn apply(n: Int, f: (n: Int) -> Int) [] -> Int => f {
    return f(n)
}

fn as_implicit(n: Int, ?twice: (n: Int) -> Int) [] -> Int {
    return twice(n)
}

struct Box<T> : Eq<self> by auto {
    item: T
}

fn same(a: Box<Str>, b: Box<Str>, f: (x: Box<Str>, y: Box<Str>) -> Bool) [] -> Bool => a, b, f {
    return f(a, b)
}

fn main() [use] -> None {
    use StdConsole()
    let a: Valid Range = by_name(Range { min: 1, max: 5 }, validate)
    let b: Valid Range = by_implicit(Range { min: 2, max: 9 })
    println("${width(a)} ${width(b)}")
    println("${apply(3, twice)} ${as_implicit(3)}")
    println("${same(Box { item: "x" }, Box { item: "x" }, eq)} ${same(Box { item: "x" }, Box { item: "y" }, eq)}")
}
"#;
    let dir = work_dir("fn_value_one_rule");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "4 7\n12 12\ntrue false\n", "{backend}");
    }
    __stamp.verified();
}

/// [fn-throw-value] [fn-effects] A fn value that may throw throws to the
/// `try` around its call: a named fn and a lambda passed into a throwing
/// slot, a function that delimits it writing `[]`, one passing it on
/// declaring the throw, one writing no list inheriting it, and an implicit
/// with a throwing type (the shape `args`' validation hook needs), on both
/// backends.
#[test]
fn a_throwing_fn_value_is_delimited_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_throwing_fn_value_is_delimited_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.throw

fn check(n: Int) [Throw<Str>] -> Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

// Delimits the throw itself, so says `[]`.
fn run<T>(n: Int, f: (n: Int) [Throw<Str>] -> T) [] -> Str => f {
    let r = try {
        f(n)
    }
    when r {
        is Ok { return "ok" }
        is Thrown { return "thrown: ${r}" }
    }
}

// Passes the throw on, so says so.
fn pass_on(n: Int, f: (n: Int) [Throw<Str>] -> Int) [Throw<Str>] -> Int => f {
    return f(n) + 1
}

// Writes no list, so inherits `f`'s.
fn inherit(n: Int, f: (n: Int) [Throw<Str>] -> Int) -> Int => f {
    return f(n) * 10
}

// The implicit form the args hook needs.
fn by_implicit<V>(n: Int, ?check: (n: Int) [Throw<Str>] -> V) [] -> Str {
    let r = try {
        check(n)
    }
    when r {
        is Ok { return "implicit ok" }
        is Thrown { return "implicit thrown: ${r}" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(run(3, check))
    println(run(-1, check))
    println(run(5, (n: Int) -> {
        if n > 3 {
            throw("big")
        }
        return n + 0
    }))
    let a = try {
        pass_on(-2, check)
    }
    when a {
        is Ok { println("passed ${a}") }
        is Thrown { println("passed on: ${a}") }
    }
    let b = try {
        inherit(4, check)
    }
    when b {
        is Ok { println("inherited ${b}") }
        is Thrown { println("inherited: ${b}") }
    }
    let i: Str = by_implicit<Int>(-7)
    println(i)
}
"#;
    let dir = work_dir("throwing_fn_value");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "ok\nthrown: negative\nthrown: big\npassed on: negative\ninherited 40\nimplicit thrown: negative\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// [args-validate] [fn-value] [fn-throw-value] The validation hook: a user's
/// `validate(o) [Throw<ArgError>] -> +Valid Range`, found as an implicit, so
/// `parse_valid` answers `Ok Valid Range` or the parse errors or the one
/// `validate` threw, and `valid_args_or_exit` ends the process on an error
/// with status 2, on both backends.
#[test]
fn the_validation_hook_runs_on_both_backends() {
    let Some(__stamp) = e2e_stamp("the_validation_hook_runs_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.args
import std.process
import std.throw

struct Range : FromArgs<self> by args {
    // Smallest.
    min: Int = 0
    // Largest.
    max: Int = 10
}

qualifier Valid of Range

fn validate(o: Range) [Throw<ArgError>] -> +Valid Range => !o {
    if o.min > o.max {
        let e: ArgError = Invalid { what: "--min", value: "${o.min}", reason: "must not exceed --max" }
        throw(e)
    }
    return o
}

fn width(r: Valid Range) [] -> Int => r {
    return r.max - r.min
}

fn show(line: List<Str>) [Console] -> None => line {
    let r: Ok Valid Range | Errs NonEmpty List<ArgError> = parse_valid(line)
    when r {
        is Ok { println("width ${width(r)}") }
        is Errs {
            for e in r {
                println("error: ${message(copy(e))}")
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    show(List("--min", "3"))
    show(List("--min", "30"))
    show(List("--min", "x"))
    let r: Valid Range = valid_args_or_exit(args())
    println("argv width ${width(r)}")
}
"#;
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("validation_hook_{backend}"));
        fs::write(dir.join("main.sv"), SRC).unwrap();
        let out = salvo_in(&dir, &["build", "--backend", backend, "--src", ".", "--out", "out"]);
        assert!(out.status.success(), "{backend} build failed: {}", String::from_utf8_lossy(&out.stderr));
        let lines = "width 7\nerror: invalid value '30' for '--min': must not exceed --max\nerror: invalid value 'x' for '--min': expected an integer\n";
        let ok = Command::new(dir.join("out").join("main")).args(["--max", "4"]).output().expect("failed to run the artifact");
        assert_eq!(ok.status.code(), Some(0), "{backend}");
        assert_eq!(String::from_utf8_lossy(&ok.stdout), format!("{lines}argv width 4\n"), "{backend}");
        let bad = Command::new(dir.join("out").join("main")).args(["--min", "9", "--max", "4"]).output().expect("failed to run the artifact");
        assert_eq!(bad.status.code(), Some(2), "{backend}");
        assert_eq!(String::from_utf8_lossy(&bad.stdout), lines, "{backend}");
        assert!(String::from_utf8_lossy(&bad.stderr).starts_with("error: invalid value '9' for '--min': must not exceed --max\n"), "{backend}: {}", String::from_utf8_lossy(&bad.stderr));
    }
    __stamp.verified();
}

/// [kt-throw-tag] [rs-throw-controlflow] A union-typed message is caught by
/// a `try` whose message is a wider union: on Kotlin it escaped, since the
/// signal was tagged with the sub-union's name, which matched no case.
#[test]
fn a_sub_union_throw_is_caught_on_both_backends() {
    let Some(__stamp) = e2e_stamp("a_sub_union_throw_is_caught_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    const SRC: &str = r#"import std.throw

struct A { x: Int }
struct B { y: Int }
struct C { z: Int }

fn pick(n: Int) [] -> A | B {
    if n > 0 {
        return A { x: n }
    }
    return B { y: n }
}

fn f(n: Int) [Throw<A | B | C>] -> Int {
    if n == 0 {
        throw(C { z: 0 })
    }
    let e: A | B = pick(n)
    throw(e)
}

fn main() [use] -> None {
    use StdConsole()
    let r = try {
        f(1)
    }
    when r {
        is Ok { println("ok") }
        is Thrown { println("caught") }
    }
}
"#;
    let dir = work_dir("sub_union_throw");
    fs::write(dir.join("main.sv"), SRC).unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", ".", "--clean-target", "before"]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "caught\n", "{backend}");
    }
    __stamp.verified();
}

/// [str-parse] The value parsers read the same strings on both backends:
/// `parse_double` checks one decimal grammar before converting (the hosts'
/// own parsers disagree on whitespace, `NaN`, `inf`, hex and `f` suffixes),
/// `parse_bool` takes exactly `true`/`false`, and `from_str` is one name for
/// every scalar, chosen by the expected type and resolvable as an implicit.
#[test]
fn the_value_parsers_agree_on_both_backends() {
    let Some(__stamp) = e2e_stamp("the_value_parsers_agree_on_both_backends", &["rustc", "kotlinc"]) else {
        return;
    };
    let dir = work_dir("value_parsers");
    fs::write(dir.join("main.sv"), "fn show(d: Double?) [] -> Str => d {\n    if d is None { return \"none\" }\n    return \"${d}\"\n}\n\nfn read<T>(s: Str, ?from_str: (s: Str) -> T?) [] -> T? => s {\n    return from_str(s)\n}\n\nfn main() [use] -> None {\n    use StdConsole()\n    for s in List(\"1.5\", \"-2\", \"1e3\", \"2.5E-4\", \".5\", \"1.\", \" 1\", \"1.5f\", \"NaN\", \"inf\", \"0x10\", \"\", \"+\", \"1e\", \"-.5e+2\") {\n        println(\"${s}|${show(parse_double(s))}\")\n    }\n    let l: Long? = from_str(\"9000000000\")\n    let b: Bool? = from_str(\"true\")\n    let bad: Bool? = from_str(\"True\")\n    let i: Int? = read(\"42\")\n    let t: Str? = from_str(\"text\")\n    println(\"${l ?: 0L} ${b ?: false} ${bad is None} ${i ?: 0} ${t ?: \"-\"}\")\n}\n").unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        assert!(out.status.success(), "{backend} failed: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "1.5|1.5\n-2|-2.0\n1e3|1000.0\n2.5E-4|2.5E-4\n.5|0.5\n1.|1.0\n 1|none\n1.5f|none\nNaN|none\ninf|none\n0x10|none\n|none\n+|none\n1e|none\n-.5e+2|-50.0\n9000000000 true true 42 text\n", "{backend}");
    }
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
         use StdConsole\n    println(\"nested ${label(3)}\")\n}\n",
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
        "fn main() [use] {\n    use StdConsole()\n    println(\"defaulted\")\n}\n",
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
        stderr.contains("`std.core.list`") && stderr.contains("std = true") && stderr.contains("[manifest-deps]"),
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
        "export effect Streams {\n    fn next_id() -> Int\n}\n\nexport handler Counter() of Streams {\n    state {\n        n: Int = 0\n    }\n    fn next_id() -> Int {\n        n = n + 1\n        return n\n    }\n}\n",
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
         fn main() [use] {\n    use StdConsole()\n    use Counter()\n    use MemFiles()\n    println(opener(\"x\"))\n    println(work(opener))\n}\n",
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
            "import std.stream\n{imports}\n\nfn main() [use] {{\n    use StdConsole()\n{uses}\
             \n    let s = InStream {{ handle: 987654 }}\n    let line = read_line(s)\n    \
             println(\"read ${{line is None}}\")\n    let closed = close(s)\n    when closed {{\n        \
             is Ok {{ println(\"closed\") }}\n        is Err {{ ignore(closed) }}\n    }}\n}}\n"
        )
    };
    for (label, imports, uses) in [
        ("host", "import std.stream.host", "    use HostRawStreams()\n    use DefaultStreams()"),
        ("mem", "import std.fs.mem", "    use MemFs()"),
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

const PIPE_PROGRAM: &str = r#"import std.fs
import std.fs.path
import std.fs.host
import std.fs.mem
import std.stream
import std.stream.host

fn describe(r: Ok Long | Err Checked<StreamError>) [] -> Str => !r {
    when r {
        is Ok { return "piped ${r} bytes" }
        is Err { return "failed: ${detach(r)}" }
    }
}

// GetObject-to-a-file's shape: a stream from somewhere, into a file, without
// blocking a worker.
fn copy_in(body: InStream, path: Path) [Fs] -> Str => !body, path {
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
    println(copy_in(body, path("piped.txt")))
    let back = read_lines(path("piped.txt"))
    when back {
        is Ok { println("lines: ${back}") }
        is Err { println("read: ${detach(back)}") }
    }
    // A file is a body too: PutObject-from-a-file's shape.
    let file = open_read(path("piped.txt"))
    when file {
        is Ok { println(copy_in(file, path("piped2.txt"))) }
        is Err { println("reopen: ${detach(file)}") }
    }
    let n = read_to_str(path("piped2.txt"))
    when n {
        is Ok { println("copy of the copy: ${byte_size(n)} bytes") }
        is Err { println("read2: ${detach(n)}") }
    }
    let a = delete(path("piped.txt"))
    if a is Err { ignore(a) }
    let b = delete(path("piped2.txt"))
    if b is Err { ignore(b) }
}

fn main() [use] {
    use StdConsole()
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
        "import sc.StorageClass\nimport sc.describe\n\nfn main() [use] {\n    use StdConsole()\n    \
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

/// [type-literal] [type-union-arm-identity] Unions of literals on both backends,
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
    use StdConsole()
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
    fs::write(dir.join("lib/a.sv"), r#"import std.codec
// A `Token` with a wire form and auto equality, and an effect named like
// std's `time.Clock`.
import std.net

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
    let back = decode<Token>(encode(copy(t))) ?: unreachable!()
    let s: Set<Token> = Set(copy(t), back)
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
    state {
        seen: Str = ""
    }
    send fn bump(t: Token) => !t { seen = "${seen}${t.label}" }
    send fn total(out: Reply<Str>) => !out { out.send(copy(seen)) }
}

fn main() [use, spawn] {
    use StdConsole()
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

/// [type-identity] Two effects named `Log` — a plain one declared first,
/// keeping the plain key, and an actor one keyed by its module — and two
/// structs named `View`, one borrowing. Every lookup goes by key or through
/// the file's scope (PIPELINE.md step 5): the deduction tables, the runtime
/// features' actor test, both emitters' interface lookups, which until then
/// fell back to the plain name and could answer the other module's `Log`.
#[test]
fn two_effects_of_one_name_stay_apart_on_both_backends() {
    let Some(__stamp) = e2e_stamp("effect_names_clash", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("effect_clash");
    fs::create_dir_all(dir.join("lib")).unwrap();
    fs::write(dir.join("lib/a.sv"), r#"// A plain `Log`, declared first, so it keeps the plain key.
export effect Log {
    fn note(s: Str) -> None => !s
}

export handler Lines() [Console] of Log {
    state {
        all: Str = ""
    }
    fn note(s: Str) -> None => !s {
        all = "${all}${s};"
        println(all)
    }
}

export struct View { label: Str }
"#).unwrap();
    fs::write(dir.join("lib/b.sv"), r#"// An actor `Log` of the same name, keyed by its module.
export actor effect Log {
    send fn add(n: Int)
    send fn total(out: Reply<Int>) => !out
}

export handler Summing() of Log {
    mailbox { capacity: 4 }
    state {
        sum: Int = 0
    }
    send fn add(n: Int) { sum = sum + n }
    send fn total(out: Reply<Int>) => !out { out.send(copy(sum)) }
}

// A `View` that borrows.
export struct View { items: proj List<Int>, at: Int }

export fn view(xs: List<Int>) [] -> View => xs {
    return View { items: xs, at: 0 }
}

export fn first(v: View) [] -> Int => v {
    return get(v.items, v.at) ?: -1
}
"#).unwrap();
    fs::write(dir.join("main.sv"), r#"import lib.a.Lines
import lib.a.Log
import lib.b.Summing
import lib.b.view
import lib.b.first

fn main() [use, spawn] {
    use StdConsole()
    use Lines()
    note("one")
    note("two")
    let xs = List(7, 8)
    println("first ${first(view(xs))}")
    let c = spawn Summing() on pool(1)
    c.add(2)
    c.add(3)
    let s = waitfor out: Reply<Int> { c.total(out) }
    println("total ${s}")
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
            "one;\none;two;\nfirst 7\ntotal 5\n",
            "{backend}"
        );
    }
    __stamp.verified();
}

/// The emitter gaps of ROADMAP 0c, closed 2026-10-04: one program per item,
/// run on both backends, printing the same lines. Each line is numbered after
/// its item: (1) a second `let` of a name after a loop that declared it, (2) a
/// union argument to a `send fn` through an addr, (3) an effect named `Box`,
/// (4) `f(f(x))`, (6) a `return` in a lambda inside a throwing fn
/// [kt-lambda-return], (9) an `is` binding read later in its `&&` chain
/// [is-and-chain], (11) a fn field with a named parameter, (12) a field default
/// naming a type the literal's file does not import [rs-default-path], (13) a
/// handler named like another module's type [type-identity], (14) writing a
/// field of a `for` element [iter-for-elem-write], (16) a fn-typed local named like
/// a top-level fn [call-resolve].
#[test]
fn the_roadmap_0c_emitter_gaps_build_on_both_backends() {
    let Some(__stamp) = e2e_stamp("roadmap_0c", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("roadmap_0c");
    fs::create_dir_all(dir.join("lib")).unwrap();
    fs::write(dir.join("lib/conf.sv"), r#"import std.time.Duration

// [rs-default-path] A default naming a type the user's file does not import.
export struct Conf { d: Duration = Duration { nanos: 5L } }

// [type-identity] A handler named like a type of another module.
export effect Table {
    fn size() -> Int
}
export handler Circle() of Table {
    fn size() -> Int { return 3 }
}
"#).unwrap();
    fs::write(dir.join("lib/shapes.sv"), r#"export struct Circle { r: Int }
"#).unwrap();
    fs::write(dir.join("main.sv"), r#"import lib.conf
import lib.shapes
import std.throw

struct Node { id: Int, label: Str }
struct A { x: Int }
struct B { y: Int }
struct Rec canbe Mut { dead: Bool }
struct H<T> { skip: (x: T) -> Bool }
struct Tok { f: (x: Int) -> Int }

effect Box {
    fn get() -> Int
}
handler Boxing() of Box {
    fn get() -> Int { return 7 }
}

actor effect Bin {
    send fn go(v: A | B | None, done: Reply<Int>) => !v, !done
}
handler Binning() of Bin {
    mailbox { capacity: 4 }
    send fn go(v: A | B | None, done: Reply<Int>) {
        if v is A {
            done.send(v.x)
        } else {
            done.send(0)
        }
    }
}

fn skip(s: Str) [] -> Str => !s { return s }

// 0c item 1: a second `let n` after a loop that declared one.
fn rebind(m: Mut Map<Int, Node>, k: Int) [] -> Int => m: Mut {
    let seen = 0
    for id in keys(m) {
        let n = get(m, id)
        if !(n is None) { seen = seen + size(n.label) }
    }
    let n = remove(m, k)
    if n is None { return -1 }
    for _i in List(1, 2) { seen = seen + copy(n).id }
    return seen
}

// 0c item 4: a fn value called on its own result.
fn twice(f: (n: Int) -> Int, x: Int) [] -> Int => f {
    return f(f(x))
}

// 0c item 6: a `return` inside a lambda in a fn that may throw.
fn may(n: Int) [Throw<Str>] -> Int {
    if n < 0 { throw("neg") }
    let f = (x: Int) -> {
        if x > 1 { return x * 2 }
        return x
    }
    return f(n)
}

// 0c item 9: an `is` binding read later in its own `&&` chain.
fn same(b: Long?, x: Long) [] -> Bool {
    if b is Long known && known == x { return true }
    return false
}

// 0c item 16: a fn-typed local named like a top-level fn.
fn use_it<T>(h: H<T>, v: T) [] -> Bool => h, v {
    let skip = h.skip
    return skip(v)
}

fn main() [use, spawn] {
    use StdConsole()
    let m: Mut Map<Int, Node> = Mut Map()
    put(m, 1, Node { id: 1, label: "ab" })
    println("1: ${rebind(m, 1)}")
    let b = spawn Binning() on pool(1)
    let got = waitfor out: Reply<Int> { b.go(A { x: 9 }, out) }
    println("2: ${got}")
    use Boxing()
    println("3: ${get()}")
    println("4: ${twice((n: Int) -> n + 1, 1)}")
    let r = try { may(2) }
    when r {
        is Ok { println("6: ${r}") }
        is Thrown { println("6: thrown") }
    }
    println("9: ${same(3L, 3L)} ${same(None, 3L)}")
    let t = Tok { f: x -> x + 1 }
    let tf = t.f
    println("11: ${tf(2)}")
    let c = Conf {}
    println("12: ${c.d.nanos}")
    use Circle()
    let shape = Circle { r: 2 }
    println("13: ${size()} ${shape.r}")
    let xs: Mut List<Mut Rec> = Mut List(Mut Rec { dead: false })
    for a in xs {
        if !a.dead { a.dead = true }
    }
    println("14: ${(get(xs, 0) ?: unreachable!()).dead}")
    println("16: ${use_it(H<Int> { skip: (x: Int) -> x > 2 }, 3)} ${skip("s")}")
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
        assert_eq!(String::from_utf8_lossy(&out.stdout), "1: 4\n2: 9\n3: 7\n4: 3\n6: 4\n9: true false\n11: 3\n12: 5\n13: 3 2\n14: true\n16: true s\n", "{backend}");
    }
    __stamp.verified();
}

/// [rs-interp-sequence] An interpolation whose earlier part reads a place
/// through a call and whose later part mutates it runs in source order on
/// both backends (rustc used to refuse it: E0502).
#[test]
fn an_interpolation_reads_before_a_later_part_mutates() {
    let Some(__stamp) = e2e_stamp("interp_sequence", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("interp_sequence");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] {\n    use StdConsole()\n    let d = Mut List(1, 2, 3)\n    let i = 1\n    \
         if i is Idx(d) {\n        println(\"${get(d, i)} ${replace(d, i, 0)}\")\n    }\n    println(\"${d}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "2 2\n[1, 0, 3]\n", "{backend}");
    }
    __stamp.verified();
}

/// [kt-copy] [copy-plan] A struct that holds a list of itself copies deeply
/// on both backends: Kotlin generates the struct's own copy fn.
#[test]
fn a_struct_holding_itself_copies_on_both_backends() {
    let Some(__stamp) = e2e_stamp("recursive_copy", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("recursive_copy");
    fs::write(
        dir.join("main.sv"),
        "struct Node canbe Mut {\n    label: Str\n    kids: List<Mut Node>\n}\n\n\
         fn relabel(n: Mut Node) [] -> None {\n    n.label = \"changed\"\n}\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let kid: Mut Node = Mut Node { label: \"kid\", kids: List<Mut Node>() }\n    \
         let root: Mut Node = Mut Node { label: \"root\", kids: List(kid) }\n    \
         let twin = copy(root)\n    relabel(root)\n    \
         println(\"${root.label} ${twin.label} ${size(twin.kids)}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "changed root 1\n", "{backend}");
    }
    __stamp.verified();
}

/// [rs-narrow] A scalar arm of a union, narrowed and handed to an intrinsic
/// (`to_str(value)`), reads through the arm rather than the whole union: a
/// generic union with a generic struct arm used to render `*value` and need
/// `Box<A>: Display` (closed 2026-10-06).
#[test]
fn a_generic_union_with_a_scalar_arm_prints_on_both_backends() {
    let Some(__stamp) = e2e_stamp("generic_union_to_str", &["rustc", "kotlinc"]) else { return };
    let dir = work_dir("generic_union_to_str");
    fs::write(
        dir.join("main.sv"),
        "struct Box<A> : ToStr<self> by auto {\n    v: A\n}\n\n\
         type Either<A> = Box<A> | Int : ToStr<self> by auto\n\n\
         fn main() [use] {\n    use StdConsole()\n    \
         let e: Either<Str> = Box { v: \"x\" }\n    let f: Either<Str> = 7\n    \
         println(\"${e} ${f}\")\n}\n",
    )
    .unwrap();
    for (backend, tool) in [("rust", "rustc"), ("kotlin", "kotlinc")] {
        if !have(tool) {
            continue;
        }
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "Box { v: x } 7\n", "{backend}");
    }
    __stamp.verified();
}

/// [expr-terminal-scope] A trailing `...` makes the rest of the enclosing block the
/// construct's body — `if` (with an optional inline `else`), loops, `try` and
/// `waitfor`. Runs on both backends with the same stdout.
const TERMINAL_SCOPE: &str = "\
fn label(x: Str?) [Console] -> None {\n\
\x20   if x is Str ... else {\n\
\x20       println(\"absent\")\n\
\x20   }\n\
\x20   println(\"present ${x}\")\n\
}\n\
fn count(xs: List<Int>) [Console] -> None {\n\
\x20   let total = 0\n\
\x20   for n in xs ...\n\
\x20   total = total + n\n\
\x20   println(\"running ${total}\")\n\
}\n\
fn main() [use] {\n\
\x20   use StdConsole()\n\
\x20   label(\"here\")\n\
\x20   label(None)\n\
\x20   count(List(1, 2, 3))\n\
}\n";

const TERMINAL_SCOPE_STDOUT: &str = "present here\nabsent\nrunning 1\nrunning 3\nrunning 6\n";

#[test]
fn run_compiles_and_runs_terminal_scopes_on_each_backend() {
    let Some(__stamp) = e2e_stamp("run_terminal_scopes", &["kotlinc", "rustc"]) else {
        return;
    };
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("terminal_scope_{backend}"));
        fs::write(dir.join("main.sv"), TERMINAL_SCOPE).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} failed, stderr: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            TERMINAL_SCOPE_STDOUT,
            "{backend} stdout (stderr: {stderr})"
        );
    }
    __stamp.verified();
}

/// [std-console] `Console`'s three streams on both backends: `printerr` writes
/// stderr, and `readln` answers each line without its terminator (a `\r\n`
/// included) and `None` at the end of the input — with the prompt printed
/// before it already flushed.
const CONSOLE_STREAMS: &str = r#"
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

#[test]
fn console_reads_lines_and_writes_stderr_on_each_backend() {
    let Some(stamp) = e2e_stamp("console_streams", &["kotlinc", "rustc"]) else {
        return;
    };
    for (backend, tool) in [("kotlin", "kotlinc"), ("rust", "rustc")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("console_streams_{backend}"));
        fs::write(dir.join("main.sv"), CONSOLE_STREAMS).unwrap();
        for (input, stdout, stderr) in [
            ("Ada\r\n", "name? hello Ada\n", "second: <eof>\n"),
            ("", "name? ", "no input\n"),
        ] {
            let mut child = Command::new(env!("CARGO_BIN_EXE_salvo"))
                .env("SALVO_CACHE_DIR", concat!(env!("CARGO_TARGET_TMPDIR"), "/salvo-cache"))
                .current_dir(&dir)
                .args(["run", "--backend", backend, "--src", "."])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("failed to run salvo");
            use std::io::Write as _;
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            let out = child.wait_with_output().unwrap();
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(out.status.success(), "{backend} failed, stderr: {err}");
            assert_eq!(String::from_utf8_lossy(&out.stdout), stdout, "{backend} stdout (stderr: {err})");
            assert!(err.ends_with(stderr), "{backend} stderr: {err}");
        }
    }
    stamp.verified();
}
