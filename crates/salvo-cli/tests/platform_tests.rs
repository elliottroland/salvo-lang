//! Integration tests for `salvo platform generate` and the `platform/` tree
//! [cli-platform] [platform-tree].
//!
//! The arc these cover is the one a developer actually walks: write a
//! `platform effect`, discover from the error that the host is missing, run
//! the command, implement the stub, run the program. Tests needing a
//! toolchain (`kotlinc`, `rustc`) skip gracefully when it is missing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn work_dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("platform_{test}"));
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

/// A platform effect, an intermediate frame that performs it, and a `main`
/// that needs it — so the host owns the entry point and both the interface
/// and the threading are exercised.
const DEMO: &str = r#"
platform effect Telemetry {
    fn record(name: Str, value: Int) [] -> None => name, value
}

fn work(n: Int) [Telemetry] -> Int {
    record("work", n)
    return n + 1
}

fn main() [use, Telemetry] {
    use StdOutConsole()
    println("result=${work(41)}")
}
"#;

/// The stub body each backend's skeleton carries, and what to replace it
/// with to make the demo print. Only the body changes: everything else in
/// the generated file is used as written, which is what makes the run a test
/// of the *skeleton* and not of a hand-written host.
fn implement(backend: &str, skeleton: &str) -> String {
    let (stub, body) = match backend {
        "kotlin" => (
            "TODO(\"implement Telemetry.record\")",
            "kotlin.io.println(\"[telemetry] $name=$value\")",
        ),
        "rust" => (
            "todo!(\"implement Telemetry.record\")",
            "println!(\"[telemetry] {}={}\", name, value);",
        ),
        other => panic!("unknown backend {other}"),
    };
    assert!(
        skeleton.contains(stub),
        "expected the {backend} stub `{stub}` in:\n{skeleton}"
    );
    skeleton.replace(stub, body)
}

const EXPECTED: &str = "[telemetry] work=41\nresult=42\n";

/// [cli-platform] [platform-tree] The whole arc, per backend: the run fails
/// naming the command, the command writes the skeleton into `platform/`, the
/// stub is implemented, and the same `salvo run` now prints the program's
/// output. The asserted stdout is identical for both backends, which is what
/// parity means here.
#[test]
fn generate_then_run_works_for_each_backend() {
    let Some(__stamp) = e2e_stamp("generate_then_run_works_for_each_backend", &["kotlinc", "rustc"]) else {
        return;
    };
    for (backend, tool, ext) in [("kotlin", "kotlinc", "kt"), ("rust", "rustc", "rs")] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("arc_{backend}"));
        fs::write(dir.join("main.sv"), DEMO).unwrap();

        // Without a host there is no entry point, and the error says so by
        // name — this is the diagnostic that replaced a toolchain failure
        // against generated code.
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend} should not have run");
        assert!(
            stderr.contains("salvo platform generate")
                && stderr.contains(&format!("platform/main.{ext}")),
            "{backend} stderr: {stderr}"
        );

        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} generate failed: {stderr}");
        let host = dir.join("platform").join(format!("main.{ext}"));
        assert!(host.is_file(), "{backend}: {} was not written", host.display());

        let implemented = implement(backend, &fs::read_to_string(&host).unwrap());
        fs::write(&host, implemented).unwrap();

        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} run failed: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            EXPECTED,
            "{backend} stdout (stderr: {stderr})"
        );
    }
    __stamp.verified();
}

/// [cli-platform] The command never overwrites. With an interface between
/// Salvo and the host there is nothing to merge — every later divergence is
/// a target-language compile error — so a second run must leave an edited
/// file exactly as it was, and say that it did.
#[test]
fn generate_never_overwrites_an_existing_host() {
    let dir = work_dir("no_overwrite");
    fs::write(dir.join("main.sv"), DEMO).unwrap();
    let out = salvo_in(&dir, &["platform", "generate", "--backend", "rust", "--src", "."]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let host = dir.join("platform/main.rs");
    let edited = implement("rust", &fs::read_to_string(&host).unwrap());
    fs::write(&host, &edited).unwrap();

    let out = salvo_in(&dir, &["platform", "generate", "--backend", "rust", "--src", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("already exists"), "stderr: {stderr}");
    assert_eq!(
        fs::read_to_string(&host).unwrap(),
        edited,
        "the edited host must be untouched"
    );
}

/// [cli-platform] Each backend writes its own file, and they coexist in one
/// `platform/` tree: discovery only ever picks up the active backend's
/// native extension, so the same sources build for both.
#[test]
fn each_backend_writes_its_own_host_file() {
    let dir = work_dir("both_backends");
    fs::write(dir.join("main.sv"), DEMO).unwrap();
    for backend in ["kotlin", "rust"] {
        let out =
            salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        assert!(
            out.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(dir.join("platform/main.kt").is_file());
    assert!(dir.join("platform/main.rs").is_file());
}

/// [cli-platform] A program with no `platform effect` has nothing to
/// generate, which is a success with a message rather than an empty tree.
#[test]
fn a_program_without_platform_effects_generates_nothing() {
    let dir = work_dir("nothing");
    fs::write(
        dir.join("main.sv"),
        "fn main() [use] -> None {\n    use StdOutConsole\n    \
         println(\"hi\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["platform", "generate", "--backend", "rust", "--src", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("nothing to generate"), "stderr: {stderr}");
    assert!(!dir.join("platform").exists(), "no tree should be created");
}

/// [platform-handler] A host handler of an ordinary effect, with a Salvo
/// handler depending on it — the shape phase 4's `HostRawFs`/`DefaultFs` pair
/// has (FILE_SYSTEM.md §5.8). Application code registers both in `main` and
/// names the host class nowhere else.
const HANDLER_DEMO: &str = r#"
effect RawClock {
    fn raw_now() [] -> Int
}

platform handler HostRawClock(offset: Int) of RawClock

effect Clock {
    fn stamp(label: Str) -> Str => label
}

handler DefaultClock [RawClock] of Clock {
    fn stamp(label: Str) -> Str => label {
        return "${label}@${raw_now()}"
    }
}

fn main() [use] {
    use StdOutConsole()
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
"#;

/// [cli-platform] [platform-handler] The same arc for a `platform handler`:
/// the run fails naming the command *and the `use` that needs a host*, the
/// command writes a class named after the handler, the stub is implemented,
/// and the program runs — with `main` staying in generated code, since
/// nothing arrives from outside here. Identical stdout on both backends.
#[test]
fn generate_then_run_works_for_a_platform_handler() {
    let Some(__stamp) =
        e2e_stamp("generate_then_run_works_for_a_platform_handler", &["kotlinc", "rustc"])
    else {
        return;
    };
    for (backend, tool, ext, stub, body) in [
        (
            "kotlin",
            "kotlinc",
            "kt",
            "TODO(\"implement RawClock.raw_now\")",
            "return offset + 7",
        ),
        (
            "rust",
            "rustc",
            "rs",
            "todo!(\"implement RawClock.raw_now\")",
            "self.offset + 7",
        ),
    ] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("handler_{backend}"));
        fs::write(dir.join("main.sv"), HANDLER_DEMO).unwrap();

        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend} should not have run");
        assert!(
            stderr.contains("use HostRawClock")
                && stderr.contains("salvo platform generate")
                && stderr.contains(&format!("platform/main.{ext}")),
            "{backend} stderr: {stderr}"
        );

        let out =
            salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} generate failed: {stderr}");
        let host = dir.join("platform").join(format!("main.{ext}"));
        let src = fs::read_to_string(&host).unwrap();
        assert!(
            src.contains("HostRawClock") && src.contains(stub),
            "{backend} host:\n{src}"
        );
        fs::write(&host, src.replace(stub, body)).unwrap();

        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} run failed: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "boot@42\n",
            "{backend} stdout (stderr: {stderr})"
        );
    }
    __stamp.verified();
}

/// [platform-tree] The tree mirrors the sources, per module: the effect's
/// module gets the implementation and the entry's module gets the `main`,
/// each at its own path. Run end to end on both backends because this is
/// where the cross-module qualification lives — Kotlin has to import the
/// other host's package, Rust has to name it `crate::platform_telemetry`,
/// and a mistake in either is a target-language error rather than anything
/// Salvo would notice.
#[test]
fn the_platform_tree_mirrors_the_source_tree() {
    let Some(__stamp) = e2e_stamp("the_platform_tree_mirrors_the_source_tree", &["kotlinc", "rustc"]) else {
        return;
    };
    for (backend, tool, ext, stub, body) in [
        (
            "kotlin",
            "kotlinc",
            "kt",
            "TODO(\"implement Telemetry.record\")",
            "kotlin.io.println(\"[t] $name=$value\")",
        ),
        (
            "rust",
            "rustc",
            "rs",
            "todo!(\"implement Telemetry.record\")",
            "println!(\"[t] {}={}\", name, value);",
        ),
    ] {
        let dir = work_dir(&format!("nested_{backend}"));
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            dir.join("telemetry.sv"),
            "export platform effect Telemetry {\n    \
             fn record(name: Str, value: Int) [] -> None => name, value\n}\n",
        )
        .unwrap();
        fs::write(
            bin.join("tool.sv"),
            "import telemetry.Telemetry\n\nfn main() [Telemetry] {\n    \
             record(\"tool\", 7)\n}\n",
        )
        .unwrap();

        let out = salvo_in(
            &dir,
            &[
                "platform", "generate", "--backend", backend, "--src", ".", "--main",
                "bin/tool.sv",
            ],
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} stderr: {stderr}");

        // The effect's module gets the implementation, the entry's module the
        // `main` — each mirroring its own source path.
        let effect_host = dir.join(format!("platform/telemetry.{ext}"));
        let entry_host = dir.join(format!("platform/bin/tool.{ext}"));
        let effect_src = fs::read_to_string(&effect_host).unwrap();
        let entry_src = fs::read_to_string(&entry_host).unwrap();
        assert!(
            effect_src.contains("TelemetryHost") && effect_src.contains(stub),
            "{backend} effect host:\n{effect_src}"
        );
        let cross = match backend {
            "kotlin" => "import salvo.platform.telemetry.*",
            _ => "crate::platform_telemetry::TelemetryHost",
        };
        assert!(
            entry_src.contains("main()") && entry_src.contains(cross),
            "{backend} entry host:\n{entry_src}"
        );

        if !have(tool) {
            eprintln!("skipping {backend} run: {tool} not found on PATH");
            continue;
        }
        fs::write(&effect_host, effect_src.replace(stub, body)).unwrap();
        let out = salvo_in(
            &dir,
            &["run", "--backend", backend, "--src", ".", "--main", "bin/tool.sv"],
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} run failed: {stderr}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "[t] tool=7\n",
            "{backend} stdout (stderr: {stderr})"
        );
    }
    __stamp.verified();
}

/// [platform-host-deps] [rs-cargo] [kt-classpath] A platform companion that
/// uses a host library: the manifest declares it — a `path` crate under
/// `[rust] crates`, a jar directory under `[kotlin] libs` — and `salvo run`
/// builds with it: Rust through an emitted `Cargo.toml` and `cargo build`,
/// Kotlin with the jars on both classpaths. Same source, same stdout. The
/// Rust half also checks that `compile` writes the manifest and the hint
/// names cargo, and that dropping the crates removes the stale manifest.
#[test]
fn a_companion_can_use_a_declared_host_library() {
    let Some(__stamp) =
        e2e_stamp("a_companion_can_use_a_declared_host_library", &["kotlinc", "rustc", "cargo"])
    else {
        return;
    };
    const PROGRAM: &str = "\
platform effect Greeter {
    fn greet(n: Int) [] -> Str => n
}

fn main() [use, Greeter] {
    use StdOutConsole()
    println(greet(7))
}
";
    // ---- Rust: a local path crate.
    if have("rustc") && have("cargo") {
        let dir = work_dir("hostdeps_rust");
        fs::create_dir_all(dir.join("salvo")).unwrap();
        fs::create_dir_all(dir.join("vendor/greeter/src")).unwrap();
        fs::write(
            dir.join("vendor/greeter/Cargo.toml"),
            "[package]\nname = \"greeter\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n",
        )
        .unwrap();
        fs::write(
            dir.join("vendor/greeter/src/lib.rs"),
            "pub fn greet(n: i64) -> String { format!(\"hello #{n}\") }\n",
        )
        .unwrap();
        let manifest = |crates: &str| {
            format!(
                "[project]\nname = \"hd\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n\n[rust]\n{crates}"
            )
        };
        fs::write(dir.join("salvo.toml"), manifest("crates = { greeter = { path = \"vendor/greeter\" } }\n")).unwrap();
        fs::write(dir.join("salvo/main.sv"), PROGRAM).unwrap();
        let out = salvo_in(&dir, &["platform", "generate"]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let host = dir.join("salvo/platform/main.rs");
        let src = fs::read_to_string(&host).unwrap();
        fs::write(&host, src.replace("todo!(\"implement Greeter.greet\")", "greeter::greet(n as i64)")).unwrap();

        let out = salvo_in(&dir, &["run"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "rust run failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "hello #7\n");

        // `compile` writes the manifest beside the crate root, absolute path
        // inside, and the hint names cargo.
        let out = salvo_in(&dir, &["compile", "--target", "out"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{stderr}");
        let cargo_toml = fs::read_to_string(dir.join("out/Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("[dependencies.greeter]"), "{cargo_toml}");
        assert!(cargo_toml.contains(&dir.join("vendor/greeter").display().to_string()), "{cargo_toml}");
        assert!(cargo_toml.contains("[workspace]"), "{cargo_toml}");
        assert!(stderr.contains("cargo build --manifest-path"), "{stderr}");

        // Crates gone: the manifest we wrote goes with them, the hint is rustc's.
        fs::write(dir.join("salvo.toml"), manifest("")).unwrap();
        fs::write(&host, &src).unwrap();
        let out = salvo_in(&dir, &["compile", "--target", "out"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{stderr}");
        assert!(!dir.join("out/Cargo.toml").exists(), "stale Cargo.toml kept");
        assert!(stderr.contains("rustc --edition"), "{stderr}");
    } else {
        eprintln!("skipping rust: rustc or cargo not found on PATH");
    }

    // ---- Kotlin: a jar built with kotlinc, in a `libs` directory.
    if have("kotlinc") {
        let dir = work_dir("hostdeps_kotlin");
        fs::create_dir_all(dir.join("salvo")).unwrap();
        fs::create_dir_all(dir.join("lib/kotlin")).unwrap();
        fs::create_dir_all(dir.join("ktsrc")).unwrap();
        fs::write(
            dir.join("ktsrc/Greeter.kt"),
            "package greeter\nfun greet(n: Int): String = \"hello #$n\"\n",
        )
        .unwrap();
        let status = Command::new("kotlinc")
            .current_dir(&dir)
            .args(["ktsrc/Greeter.kt", "-d", "lib/kotlin/greeter.jar"])
            .status()
            .expect("kotlinc");
        assert!(status.success(), "building the library jar failed");
        fs::write(
            dir.join("salvo.toml"),
            "[project]\nname = \"hd\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"kotlin\"\n\n\
             [kotlin]\nartifacts = [\"example:greeter:0.1.0\"]\nlibs = \"lib/kotlin\"\n",
        )
        .unwrap();
        fs::write(dir.join("salvo/main.sv"), PROGRAM).unwrap();
        let out = salvo_in(&dir, &["platform", "generate"]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let host = dir.join("salvo/platform/main.kt");
        let src = fs::read_to_string(&host).unwrap();
        fs::write(&host, src.replace("TODO(\"implement Greeter.greet\")", "return greeter.greet(n)")).unwrap();
        let out = salvo_in(&dir, &["run"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "kotlin run failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "hello #7\n");
    } else {
        eprintln!("skipping kotlin: kotlinc not found on PATH");
    }
    __stamp.verified();
}

/// [platform-host-deps] The same crate at two versions across a project and
/// its dependency is refused before anything is built, naming both.
#[test]
fn conflicting_host_libraries_are_refused() {
    let dir = work_dir("hostdeps_conflict");
    fs::create_dir_all(dir.join("salvo")).unwrap();
    fs::create_dir_all(dir.join("salvo_modules/lib/salvo")).unwrap();
    fs::write(
        dir.join("salvo_modules/lib/salvo.toml"),
        "[project]\nname = \"lib\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\n\n[rust]\ncrates = { serde = \"1.0\" }\n",
    )
    .unwrap();
    fs::write(dir.join("salvo_modules/lib/salvo/lib.sv"), "export struct L { n: Int }\n").unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\nmodules = \"salvo_modules\"\n\n\
         [dependencies]\nlib = \"0.1.0\"\n\n[rust]\ncrates = { serde = \"2.0\" }\n",
    )
    .unwrap();
    fs::write(dir.join("salvo/main.sv"), "fn main() [use] {\n    use StdOutConsole()\n    println(\"x\")\n}\n").unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`serde`") && stderr.contains("`1.0`") && stderr.contains("`2.0`") && stderr.contains("[platform-host-deps]"),
        "{stderr}"
    );
}
