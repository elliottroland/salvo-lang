//! Integration tests for `salvo platform generate` and the `platform/` tree
//! [cli-platform] [platform-tree].
//!
//! The arc these cover is the one a developer actually walks: write a
//! `platform handler`, discover from the error that the host is missing, run
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

/// [platform-root] A project whose platform files live in `platform/` beside
/// its sources — the root has no default, so every test with platform files
/// names it.
fn project(dir: &Path) {
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nplatform = \"platform\"\n",
    )
    .unwrap();
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

/// [cli-platform] The command never overwrites: a second run leaves an
/// edited file exactly as it was, and says that it did.
#[test]
fn generate_never_overwrites_an_existing_host() {
    let dir = work_dir("no_overwrite");
    project(&dir);
    fs::write(dir.join("main.sv"), HANDLER_DEMO).unwrap();
    let out = salvo_in(&dir, &["platform", "generate", "--backend", "rust", "--src", "."]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let host = dir.join("platform/main.rs");
    let edited = fs::read_to_string(&host)
        .unwrap()
        .replace("todo!(\"implement RawClock.raw_now\")", "self.offset + 7");
    fs::write(&host, &edited).unwrap();

    let out = salvo_in(&dir, &["platform", "generate", "--backend", "rust", "--src", "."]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(!stderr.contains("wrote"), "stderr: {stderr}");
    assert_eq!(
        fs::read_to_string(&host).unwrap(),
        edited,
        "the edited host must be untouched"
    );
}

/// [cli-platform] Each backend writes its own file, and they coexist in one
/// platform root: discovery only ever picks up the active backend's
/// extensions, so the same sources build for both.
#[test]
fn each_backend_writes_its_own_host_file() {
    let dir = work_dir("both_backends");
    project(&dir);
    fs::write(dir.join("main.sv"), HANDLER_DEMO).unwrap();
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

/// [cli-platform] A program with no platform declaration has nothing to
/// generate, which is a success with a message rather than an empty tree.
#[test]
fn a_program_without_platform_declarations_generates_nothing() {
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
        project(&dir);
        fs::write(dir.join("main.sv"), HANDLER_DEMO).unwrap();

        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{backend} should not have run");
        assert!(
            stderr.contains("`HostRawClock`")
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

/// [platform-never] A platform fn and a platform handler member answering
/// `Never` are `-> !` / `: Nothing` on the host side, so a host body that
/// returns is the host compiler's error, not a silent fall-through: the
/// skeletons say so, and a host that exits is the program's end on both
/// backends.
#[test]
fn a_platform_member_answering_never_cannot_return() {
    let Some(__stamp) = e2e_stamp("a_platform_member_answering_never_cannot_return", &["kotlinc", "rustc"]) else {
        return;
    };
    const SRC: &str = r#"
effect Stop {
    fn halt(why: Str) [] -> Never => why
}

platform handler HostStop() of Stop

platform fn bail(why: Str) [] -> Never => why

fn main() [use] {
    use StdOutConsole()
    use HostStop()
    println("before")
    if size("x") > 5 {
        halt("never")
    }
    bail("stop")
}
"#;
    for (backend, tool, ext, sigs, impl_) in [
        (
            "kotlin",
            "kotlinc",
            "kt",
            ["fun bail(why: String): Nothing {", "override fun halt(why: String): Nothing {"],
            "package salvo.platform.main\n\nimport salvo.main.*\n\n\
             fun bail(why: String): Nothing { System.err.println(why); kotlin.system.exitProcess(3) }\n\n\
             class HostStop : StopPlatform {\n    override fun halt(why: String): Nothing = kotlin.system.exitProcess(4)\n}\n",
        ),
        (
            "rust",
            "rustc",
            "rs",
            ["pub fn bail(why: &String) -> ! {", "fn halt(&mut self, why: &String) -> ! {"],
            "use crate::*;\n\npub fn bail(why: &String) -> ! { eprintln!(\"{why}\"); std::process::exit(3) }\n\n\
             pub struct HostStop;\nimpl HostStop { pub fn new() -> Self { HostStop } }\n\
             impl crate::StopPlatform for HostStop {\n    fn halt(&mut self, _why: &String) -> ! { std::process::exit(4) }\n}\n",
        ),
    ] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("never_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), SRC).unwrap();
        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        assert!(out.status.success(), "{backend}: {}", String::from_utf8_lossy(&out.stderr));
        let host = dir.join("platform").join(format!("main.{ext}"));
        let skeleton = fs::read_to_string(&host).unwrap();
        for sig in sigs {
            assert!(skeleton.contains(sig), "{backend} skeleton lacks `{sig}`:\n{skeleton}");
        }
        fs::write(&host, impl_).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(3), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "before\n", "{backend}: {stderr}");
        assert!(stderr.contains("stop"), "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [platform-type] Platform types end to end on both backends, declared in
/// a module of their own: `generate` writes a class/struct per type with the
/// contract its kind promises, a copy of a plain handle shares the host
/// object, a linear `canbe Mut` one is mutated through a `Mut` parameter and
/// consumed by its closer, and a handle travels to an actor. On Rust a host
/// type breaking the contract (not `Clone`) is rustc's error at the
/// declaration's assertion, and a handle in an `encode` is the checker's.
#[test]
fn platform_types_run_on_both_backends() {
    let Some(__stamp) = e2e_stamp("platform_types_run_on_both_backends", &["kotlinc", "rustc"]) else {
        return;
    };
    const HOST_SV: &str = r#"// A counter living in the host.
export platform type Tally
export threadsafe platform type Shared
export linear platform type Cursor canbe Mut

export platform fn new_tally(start: Int) [] -> Tally => start
export platform fn bump(t: Tally) [] -> Int => t
export platform fn new_shared() [] -> Shared
export platform fn label(s: Shared) [] -> Str => s
export platform fn open_cursor(n: Int) [] -> Mut Cursor => n
export platform fn step(c: Mut Cursor) [] -> Int => c: Mut
export platform fn close(c: Cursor) [] -> None => !c

"#;
    const MAIN_SV: &str = r#"import host

actor effect Holder {
    send fn hold(t: Tally, done: Reply<Int>) => !t, !done
}

handler Holding() of Holder {
    mailbox { capacity: 4 }
    send fn hold(t: Tally, done: Reply<Int>) {
        done.send(bump(t))
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let t = new_tally(10)
    let u = copy(t)
    println("${bump(t)} ${bump(u)}")
    println(label(new_shared()))
    let c = open_cursor(3)
    println("${step(c)} ${step(c)}")
    close(c)
    let h = spawn Holding() on pool(1)
    let got = waitfor done: Reply<Int> { h.hold(t, done) }
    println("held ${got}")
}
"#;
    const HOST_RS: &str = r#"use crate::*;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Tally {
    n: Arc<Mutex<i32>>,
}

#[derive(Clone)]
pub struct Shared {}

pub struct Cursor {
    at: i32,
}

pub fn new_tally(start: i32) -> Tally {
    Tally { n: Arc::new(Mutex::new(start)) }
}

pub fn bump(t: &Tally) -> i32 {
    let mut n = t.n.lock().unwrap();
    *n += 1;
    *n
}

pub fn new_shared() -> Shared {
    Shared {}
}

pub fn label(_s: &Shared) -> String {
    "shared".to_string()
}

pub fn open_cursor(n: i32) -> Cursor {
    Cursor { at: n }
}

pub fn step(c: &mut Cursor) -> i32 {
    c.at += 1;
    c.at
}

pub fn close(c: Cursor) {
    let _ = c;
}
"#;
    const HOST_KT: &str = r#"package salvo.platform.host

import salvo.host.*

class Tally(var n: Int)
class Shared
class Cursor(var at: Int)

fun newTally(start: Int): Tally = Tally(start)
fun bump(t: Tally): Int = synchronized(t) { t.n += 1; t.n }
fun newShared(): Shared = Shared()
fun label(s: Shared): String = "shared"
fun openCursor(n: Int): Cursor = Cursor(n)
fun step(c: Cursor): Int { c.at += 1; return c.at }
fun close(c: Cursor) {}
"#;
    for (backend, tool, ext, skeleton_line, implementation) in [
        ("kotlin", "kotlinc", "kt", "class Cursor {", HOST_KT),
        ("rust", "rustc", "rs", "#[derive(Clone)]\npub struct Tally {", HOST_RS),
    ] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("ptype_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        assert!(out.status.success(), "{backend}: {}", String::from_utf8_lossy(&out.stderr));
        let path = dir.join("platform").join(format!("host.{ext}"));
        let skeleton = fs::read_to_string(&path).unwrap();
        assert!(skeleton.contains(skeleton_line), "{backend} skeleton:\n{skeleton}");
        fs::write(&path, implementation).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "11 12\nshared\n4 5\nheld 13\n", "{backend}: {stderr}");
        if backend == "rust" {
            fs::write(&path, implementation.replacen("#[derive(Clone)]\npub struct Tally", "pub struct Tally", 1)).unwrap();
            let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(!out.status.success() && stderr.contains("__contract"), "{stderr}");
        }
    }
    // [noremote] A host handle has no wire form.
    let dir = work_dir("ptype_wire");
    project(&dir);
    fs::write(dir.join("host.sv"), HOST_SV).unwrap();
    fs::write(dir.join("main.sv"), "import host\nimport net\n\nfn main() [] {\n    let _b = encode(new_tally(1))\n}\n").unwrap();
    let out = salvo_in(&dir, &["analyze", "--src", "."]);
    let all = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(all.contains("`Tally` is declared `noremote`"), "{all}");
    __stamp.verified();
}

/// [platform-fn-value] Function values at the boundary, on both backends: a
/// platform fn taking an effect-free callback (a plain one, called twice,
/// and a `once` one, consumed), one catching a fault the callback raises,
/// and a platform handler member taking a callback — which on Rust is
/// `&mut dyn FnMut`, since an effect trait is used as `dyn`. The skeletons
/// give the host its own closure types.
#[test]
fn function_values_cross_the_platform_boundary() {
    let Some(__stamp) = e2e_stamp("function_values_cross_the_platform_boundary", &["kotlinc", "rustc"]) else {
        return;
    };
    const HOST_SV: &str = r#"export platform fn call(body: once () -> Str) [] -> Str => !body
export platform fn twice(f: (n: Int) -> Int, x: Int) [] -> Int => f, x
export platform fn run_guarded(body: once () -> None) [] -> Str? => !body

export effect Apply {
    fn apply(f: (n: Int) -> Int, x: Int) -> Int => f, x
}

export platform handler HostApply() of Apply
"#;
    const MAIN_SV: &str = r#"import host

fn main() [use] {
    use StdOutConsole()
    let k = 5
    println("${twice(n -> n + k, 1)}")
    let label = "late"
    println(call(() -> "${label} ran"))
    let why = run_guarded(() -> {
        let xs: List<Int> = []
        let _x = get(xs, 3)!
    })
    println("faulted: ${!(why is None)}")
    println("${run_guarded(() -> {}) is None}")
    use HostApply()
    println("${apply(n -> n * k, 2)}")
}
"#;
    const HOST_RS: &str = r#"// Host implementation of the platform declarations of Salvo module `host`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: rustc
// checks it, against the traits the backend generates from the
// `platform handler` declarations.

use crate::host::*;

pub fn call(body: impl FnOnce() -> String) -> String {
    body()
}

pub fn twice(f: &mut impl FnMut(i32) -> i32, x: i32) -> i32 {
    let y = f(x);
    f(y)
}

pub fn run_guarded(body: impl FnOnce()) -> Option<String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(()) => None,
        Err(e) => Some(e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "fault".into())),
    }
}

// `platform handler HostApply` — the compiler SERIALIZES this instance: every
// member runs under one lock on both backends, so the receivers are
// `&mut self` and plain fields are fine; the struct must be `Send`.
// If the host synchronizes internally, declare it `threadsafe platform
// handler` in Salvo to say so [threadsafe-platform].
pub struct HostApply {
}

impl HostApply {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::host::ApplyPlatform for HostApply {
    fn apply(&mut self, f: &mut dyn FnMut(i32) -> i32, x: i32) -> i32 {
        f(x)
    }
}
"#;
    const HOST_KT: &str = r#"// Host implementation of the platform declarations of Salvo module `host`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: the
// Kotlin compiler checks it, against the interfaces the backend generates
// from the `platform handler` declarations.
package salvo.platform.host

import salvo.host.*

fun call(body: () -> String): String = body()
fun twice(f: (Int) -> Int, x: Int): Int = f(f(x))
fun runGuarded(body: () -> Unit): String? = try { body(); null } catch (t: Throwable) { t.message ?: "fault" }

class HostApply : ApplyPlatform {
    override fun apply(f: (Int) -> Int, x: Int): Int {
        return f(x)
    }
}
"#;
    for (backend, tool, ext, sigs, implementation) in [
        ("kotlin", "kotlinc", "kt", ["fun call(body: () -> String): String", "override fun apply(f: (Int) -> Int, x: Int): Int"], HOST_KT),
        ("rust", "rustc", "rs", ["pub fn call(body: impl FnOnce() -> String) -> String", "fn apply(&mut self, f: &mut dyn FnMut(i32) -> i32, x: i32) -> i32"], HOST_RS),
    ] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("pfn_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        assert!(out.status.success(), "{backend}: {}", String::from_utf8_lossy(&out.stderr));
        let path = dir.join("platform").join(format!("host.{ext}"));
        let skeleton = fs::read_to_string(&path).unwrap();
        for sig in sigs {
            assert!(skeleton.contains(sig), "{backend} skeleton lacks `{sig}`:\n{skeleton}");
        }
        fs::write(&path, implementation).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "11\nlate ran\nfaulted: true\ntrue\n10\n", "{backend}: {stderr}");
    }
    __stamp.verified();
}

/// [platform-tree] The tree mirrors the sources, per module: the implementation
/// of a handler declared in `telemetry.sv` is `platform/telemetry.<ext>`, and
/// a `use` of it from another module (`bin/tool.sv`, chosen with `--main`)
/// constructs it across modules. Run end to end on both backends, because
/// the cross-module path is where a mistake would be a target-language error
/// rather than anything Salvo would notice.
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
        project(&dir);
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            dir.join("telemetry.sv"),
            "export effect Telemetry {\n    \
             fn record(name: Str, value: Int) [] -> None => name, value\n}\n\n\
             export platform handler HostTelemetry of Telemetry\n",
        )
        .unwrap();
        fs::write(
            bin.join("tool.sv"),
            "import telemetry.Telemetry\nimport telemetry.HostTelemetry\n\n\
             fn main() [use] {\n    use HostTelemetry()\n    record(\"tool\", 7)\n}\n",
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

        // The declaring module gets the implementation file, mirroring its
        // source path; the entry's module gets nothing.
        let template = dir.join(format!("platform/telemetry.{ext}"));
        let src = fs::read_to_string(&template).unwrap();
        assert!(src.contains("HostTelemetry") && src.contains(stub), "{backend} host:\n{src}");
        assert!(!dir.join("platform/bin").exists(), "{backend}: nothing for the entry module");

        if !have(tool) {
            eprintln!("skipping {backend} run: {tool} not found on PATH");
            continue;
        }
        fs::write(&template, src.replace(stub, body)).unwrap();
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

/// [platform-host-deps] [rs-cargo] [kt-classpath] [kt-gradle] [host-tool] A platform companion that
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
effect Greeter {
    fn greet(n: Int) [] -> Str => n
}

platform handler HostGreeter of Greeter

fn main() [use] {
    use StdOutConsole()
    use HostGreeter()
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
                "[project]\nname = \"hd\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\nplatform = \"salvo/platform\"\n\n[rust]\n{crates}"
            )
        };
        // [host-tool] `[rust] cargo` names the cargo to run: here a wrapper
        // announcing itself on stdout, which must not reach the program's.
        fake_tool(&dir.join("tools/cargo"), "echo 'wrapped cargo'\nexec cargo \"$@\"\n");
        fs::write(
            dir.join("salvo.toml"),
            manifest("crates = { greeter = { path = \"vendor/greeter\" } }\ncargo = \"tools/cargo\"\n"),
        )
        .unwrap();
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
        assert!(stderr.contains("wrapped cargo"), "the named cargo ran: {stderr}");

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
        // Following that hint puts cargo's `target/` inside the output tree,
        // with build scripts' `.rs` output in it; the next `compile` leaves a
        // cache directory alone (found 2026-09-30: it removed them, and the
        // rebuild failed on a missing `build_env.rs`).
        let generated = dir.join("out/target/debug/build/x/out/build_env.rs");
        fs::create_dir_all(generated.parent().unwrap()).unwrap();
        fs::write(dir.join("out/target/CACHEDIR.TAG"), "Signature: 8a477f597d28d172789f06886806bc55\n").unwrap();
        fs::write(&generated, "// cargo's\n").unwrap();
        let out = salvo_in(&dir, &["compile", "--target", "out"]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(generated.exists(), "compile removed a file in cargo's target/");

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
        let manifest = |kotlin: &str| {
            format!(
                "[project]\nname = \"hd\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"kotlin\"\nplatform = \"salvo/platform\"\n\n[kotlin]\n{kotlin}"
            )
        };
        fs::write(dir.join("salvo.toml"), manifest("libs = \"lib/kotlin\"\n")).unwrap();
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

        // [kt-gradle] [host-tool] `artifacts` are resolved by the Gradle
        // `[kotlin] gradle` names — here a stand-in speaking the same
        // protocol (the real one runs in the aws glue test): it is given the
        // target as `--project-dir`, finds the coordinate in the build salvo
        // wrote, and lists the jar. It announces a download on stdout, as a
        // wrapper's first run does, which must not reach the program's.
        let jar = dir.join("lib/kotlin/greeter.jar");
        fake_tool(
            &dir.join("tools/gradle"),
            &format!(
                "echo 'Downloading https://services.gradle.org/distributions/fake.zip'\n\
                 while [ $# -gt 0 ]; do [ \"$1\" = --project-dir ] && dir=\"$2\"; shift; done\n\
                 grep -q 'salvoHost(\"example:greeter:0.1.0\")' \"$dir/build.gradle.kts\" || exit 3\n\
                 grep -q 'rootProject.name' \"$dir/settings.gradle.kts\" || exit 4\n\
                 mkdir -p \"$dir/.salvo_gradle\"\n\
                 printf '%s\\n' '{}' > \"$dir/.salvo_gradle/classpath.txt\"\n",
                jar.display()
            ),
        );
        fs::write(
            dir.join("salvo.toml"),
            manifest("artifacts = [\"example:greeter:0.1.0\"]\ngradle = \"tools/gradle\"\n"),
        )
        .unwrap();
        let out = salvo_in(&dir, &["run"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "kotlin run through Gradle failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "hello #7\n");
        assert!(stderr.contains("Downloading"), "{stderr}");
        // `compile` writes the build; dropping the artifacts removes it.
        let out = salvo_in(&dir, &["compile", "--target", "out"]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(dir.join("out/build.gradle.kts").is_file() && dir.join("out/settings.gradle.kts").is_file());
        fs::write(dir.join("salvo.toml"), manifest("libs = \"lib/kotlin\"\n")).unwrap();
        let out = salvo_in(&dir, &["compile", "--target", "out"]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(!dir.join("out/build.gradle.kts").exists(), "stale Gradle build kept");
    } else {
        eprintln!("skipping kotlin: kotlinc not found on PATH");
    }
    __stamp.verified();
}

/// A shell script at `path`, executable: a stand-in for a host build tool.
fn fake_tool(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// [host-tool] A build tool the manifest names that is not there is reported
/// naming the file and the key, for both backends alike — and a program that
/// declares no host libraries never runs either tool. No toolchain is
/// needed: the tool is looked for before anything is compiled.
#[test]
fn a_missing_host_tool_names_its_setting() {
    for (backend, section) in [
        ("rust", "[rust]\ncrates = { serde = \"1.0\" }\ncargo = \"tools/cargo\"\n"),
        ("kotlin", "[kotlin]\nartifacts = [\"a:b:1.0\"]\ngradle = \"tools/gradlew\"\n"),
    ] {
        let dir = work_dir(&format!("missing_tool_{backend}"));
        fs::write(
            dir.join("salvo.toml"),
            format!("[project]\nname = \"m\"\nversion = \"0.1.0\"\n\n[build]\nbackend = \"{backend}\"\n\n{section}"),
        )
        .unwrap();
        fs::write(dir.join("main.sv"), "fn main() [use] {\n    use StdOutConsole()\n    println(\"x\")\n}\n").unwrap();
        let out = salvo_in(&dir, &["run"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let key = if backend == "rust" { "[rust] cargo" } else { "[kotlin] gradle" };
        assert!(!out.status.success(), "{backend}: {stderr}");
        assert!(stderr.contains("does not exist") && stderr.contains(key), "{backend}: {stderr}");
    }
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

/// [platform-reply] A platform member that takes a `Reply` returns at once and
/// completes the reply later from a host thread: `.hosted()` takes the token,
/// `send` completes it. Checked on both backends with two continuations — a
/// `waitfor` in `main`, and a free `send fn` whose own answer `main` waits on —
/// and the skeleton states the contract. On Rust, a host reply dropped unsent
/// reaches the fault sink and the waiter reports its deadlock instead of
/// hanging (the JVM cannot see a drop, so Kotlin is not asked to).
#[test]
fn a_host_thread_completes_a_reply() {
    let Some(__stamp) = e2e_stamp("a_host_thread_completes_a_reply", &["kotlinc", "rustc"]) else {
        return;
    };
    const PROGRAM: &str = "\
effect Slow {
    fn later(n: Int, done: Reply<Int>) [] -> None => !n, !done
}

threadsafe platform handler HostSlow of Slow

send fn labelled(label: Str, out: Reply<Str>, n: Int) => !label, !out, !n {
    out.send(\"${label} ${n}\")
}

fn main() [use] {
    use StdOutConsole()
    use HostSlow()
    let n = waitfor done: Reply<Int> {
        later(41, done)
    }
    println(\"waited for ${n}\")
    let line = waitfor out: Reply<Str> {
        later(1, replyto labelled(\"continued with\", out))
    }
    println(line)
}
";
    let expected = "waited for 42\ncontinued with 2\n";
    for (backend, tool, ext, stub, body) in [
        (
            "rust",
            "rustc",
            "rs",
            "todo!(\"implement Slow.later\")",
            "let done = done.hosted();\n        std::thread::spawn(move || {\n            std::thread::sleep(std::time::Duration::from_millis(50));\n            if n >= 0 { done.send(n + 1) } else { drop(done) }\n        });",
        ),
        (
            "kotlin",
            "kotlinc",
            "kt",
            "TODO(\"implement Slow.later\")",
            "val host = done.hosted()\n        Thread { Thread.sleep(50); host.send(n + 1) }.start()",
        ),
    ] {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("reply_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), PROGRAM).unwrap();
        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend, "--src", "."]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let host = dir.join("platform").join(format!("main.{ext}"));
        let src = fs::read_to_string(&host).unwrap();
        assert!(
            src.contains("`done`: a continuation [platform-reply]") && src.contains(".hosted()"),
            "{backend} skeleton states no contract:\n{src}"
        );
        fs::write(&host, src.replace(stub, body)).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend} run failed: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), expected, "{backend} (stderr: {stderr})");

        if backend == "rust" {
            // Dropped unsent: reported, and the waiter's deadlock is named.
            fs::write(
                dir.join("main.sv"),
                PROGRAM.replace("later(41, done)", "later(-1, done)"),
            )
            .unwrap();
            let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                stderr.contains("a host reply was dropped without being sent")
                    && stderr.contains("deadlock"),
                "{stderr}"
            );
        }
    }
    __stamp.verified();
}

/// Every file under [dir] as (relative path, contents), sorted, skipping
/// hidden directories and cargo's `target/` — the generated tree, as input to
/// a content stamp.
fn tree_contents(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !name.starts_with('.') && name != "target" {
                    stack.push(path);
                }
            } else {
                let rel = path.strip_prefix(dir).unwrap().display().to_string();
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// A stamp keyed on the *generated code* (and the toolchains), not on the
/// `salvo` binary: the glue has to be recompiled only when what it is
/// compiled with changed.
fn content_stamp(what: &str, tree: &[(String, Vec<u8>)], tools: &[&str]) -> Option<salvo_testkit::Stamp> {
    let mut parts: Vec<Vec<u8>> = vec![what.as_bytes().to_vec()];
    for (path, bytes) in tree {
        parts.push(path.as_bytes().to_vec());
        parts.push(bytes.clone());
    }
    for tool in tools {
        parts.push(salvo_testkit::tool(env!("CARGO_TARGET_TMPDIR"), tool).version.into_bytes());
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    salvo_testkit::cached(env!("CARGO_TARGET_TMPDIR"), what, &refs)
}

/// [platform-abi] [platform-tree] [platform-host-deps] [kt-gradle] The `aws`
/// module's **generated** host glue (`modules/aws/salvo/platform/aws/**`)
/// compiles against the real SDKs, through the two live demos that reach it.
/// The generator writes host code that names what the Salvo emitters produce
/// (`UnionN`, `Checked`, a module's Rust path, a Kotlin data class), so a
/// change to emission breaks it — loudly, but until now only when someone ran
/// a demo by hand. The Kotlin SDK is resolved the way a build resolves it:
/// through the `build.gradle.kts` `salvo compile` writes, run by the Gradle
/// the demo's manifest names (the generator's wrapper). Skips, saying so, when
/// the SDKs are not available locally: the crates not in cargo's cache
/// (checked `--offline`), or Gradle unable to resolve the artifacts (offline,
/// nothing cached). Keyed on the generated trees, so it reruns only when
/// emission or the glue changed.
#[test]
fn the_aws_glue_compiles_against_both_sdks() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let gradlew = repo.join("modules/aws/codegen/gradlew");
    for demo in ["sqs_live", "s3_live"] {
        let src = repo.join("modules/aws/demo").join(demo);

        // ---- Rust: `cargo check`, offline, sharing one target directory.
        if have("rustc") && have("cargo") {
            let out = tmp.join(format!("aws_glue_rs_{demo}"));
            let _ = fs::remove_dir_all(&out);
            let done = salvo_in(&src, &["compile", "--backend", "rust", "--target", out.to_str().unwrap()]);
            assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
            let tree = tree_contents(&out);
            if let Some(stamp) = content_stamp(&format!("aws glue rust {demo}"), &tree, &["rustc", "cargo"]) {
                let check = Command::new("cargo")
                    .args(["check", "--offline", "--quiet", "--manifest-path"])
                    .arg(out.join("Cargo.toml"))
                    .env("CARGO_TARGET_DIR", tmp.join("aws_glue_cargo"))
                    .output()
                    .expect("failed to run cargo");
                let stderr = String::from_utf8_lossy(&check.stderr);
                if !check.status.success() && (stderr.contains("offline") || stderr.contains("no matching package")) {
                    eprintln!("skipping the Rust glue of {demo}: the SDK crates are not in cargo's cache");
                } else {
                    assert!(check.status.success(), "the Rust glue of {demo} does not compile:\n{stderr}");
                    stamp.verified();
                }
            }
        }

        // ---- Kotlin: kotlinc over the tree, the SDK resolved by Gradle.
        if have("kotlinc") && have("java") {
            let out = tmp.join(format!("aws_glue_kt_{demo}"));
            let _ = fs::remove_dir_all(&out);
            let done = salvo_in(&src, &["compile", "--backend", "kotlin", "--target", out.to_str().unwrap()]);
            assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
            let build = fs::read_to_string(out.join("build.gradle.kts"))
                .expect("reaching the aws glue writes the Gradle build");
            assert!(build.contains("salvoHost(\"aws.sdk.kotlin:sqs:1.9.11\")"), "{build}");
            let tree = tree_contents(&out);
            if let Some(stamp) = content_stamp(&format!("aws glue kotlin {demo}"), &tree, &["kotlinc"]) {
                let resolved = Command::new(&gradlew)
                    .args(["--quiet", "--project-dir"])
                    .arg(&out)
                    .arg("salvoClasspath")
                    .output()
                    .expect("failed to run the aws Gradle wrapper");
                if !resolved.status.success() {
                    eprintln!(
                        "skipping the Kotlin glue of {demo}: Gradle could not resolve the SDK:\n{}",
                        String::from_utf8_lossy(&resolved.stderr)
                    );
                    continue;
                }
                let classpath = fs::read_to_string(out.join(".salvo_gradle/classpath.txt")).unwrap();
                assert!(classpath.lines().any(|l| l.ends_with("sqs-jvm-1.9.11.jar")), "{classpath}");
                let sources: Vec<PathBuf> = tree
                    .iter()
                    .filter(|(p, _)| p.ends_with(".kt"))
                    .map(|(p, _)| out.join(p))
                    .collect();
                let classpath = classpath.lines().collect::<Vec<_>>().join(":");
                let compiled = Command::new("kotlinc")
                    .args(&sources)
                    .args(["-nowarn", "-cp", &classpath, "-d"])
                    .arg(out.join(".classes"))
                    .output()
                    .expect("failed to run kotlinc");
                assert!(
                    compiled.status.success(),
                    "the Kotlin glue of {demo} does not compile:\n{}",
                    String::from_utf8_lossy(&compiled.stderr)
                );
                stamp.verified();
            }
        }
    }
}

/// [platform-root] [cli-platform] [platform-fn] Each backend has its own
/// platform root when the manifest says so, and `generate` fills each: a
/// implementation file (`main.<ext>`) for the platform handler and the platform fn.
/// The skeletons type-check as written, a second run keeps every file, and
/// without a root for a backend the program is refused naming the key.
#[test]
fn generate_writes_implementations_into_each_backends_root() {
    let dir = work_dir("split_roots");
    fs::create_dir_all(dir.join("salvo")).unwrap();
    let manifest = |roots: &str| {
        format!("[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"*\"\n\n{roots}")
    };
    fs::write(dir.join("salvo.toml"), manifest("[kotlin]\nplatform = \"kotlin\"\n\n[rust]\nplatform = \"rust\"\n")).unwrap();
    fs::write(
        dir.join("salvo/main.sv"),
        "\
effect Clock {
    fn now() [] -> Int
}

platform handler HostClock(offset: Int) of Clock

platform fn shout(s: Str) [] -> Str => s

fn main() [use] {
    use StdOutConsole()
    use HostClock(1)
    println(shout(\"${now()}\"))
}
",
    )
    .unwrap();
    let out = salvo_in(&dir, &["platform", "generate"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    for (root, ext, todo) in [("kotlin", "kt", "TODO(\"implement shout\")"), ("rust", "rs", "todo!(\"implement shout\")")] {
        let host = fs::read_to_string(dir.join(root).join(format!("main.{ext}")))
            .unwrap_or_else(|e| panic!("{root}: no implementation file ({e}): {stderr}"));
        let shout = if ext == "kt" { "fun shout(s: String): String {" } else { "pub fn shout(s: &String) -> String {" };
        assert!(host.contains(shout) && host.contains(todo), "{host}");
        assert!(host.contains("HostClock") && host.contains("implement Clock.now"), "{host}");
        assert!(!dir.join("salvo/platform").exists(), "nothing under the source root");
    }
    let out = salvo_in(&dir, &["analyze"]);
    let all = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success() && all.contains("no errors"), "{all}");

    let out = salvo_in(&dir, &["platform", "generate"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && !stderr.contains("wrote"), "{stderr}");

    // No root for Rust: refused, naming the key.
    fs::write(dir.join("salvo.toml"), manifest("[kotlin]\nplatform = \"kotlin\"\n")).unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let all = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!out.status.success(), "{all}");
    assert!(all.contains("names no platform root for rust") && all.contains("[rust] platform"), "{all}");
    // A Rust file under the Kotlin root is not read, and says so.
    fs::write(dir.join("salvo.toml"), manifest("[build]\n")).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"*\"\n\n[kotlin]\nplatform = \"kotlin\"\n\n[rust]\nplatform = \"rust\"\n",
    )
    .unwrap();
    fs::write(dir.join("kotlin/stray.rs"), "pub fn stray() {}\n").unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let all = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(all.contains("is a Rust file under the Kotlin platform root"), "{all}");
}

/// [platform-abi] Every build writes the platform root's host project — the
/// declarations its platform surface reaches, the runtime they need, and the
/// project files — so the root's implementation files compile **on their
/// own**, without the build output: `kotlinc` over the root's `.kt` files,
/// `rustc` over its `lib.sv.rs`. `analyze` writes nothing; a generated file
/// the program no longer produces is removed, a hand-written one never.
#[test]
fn the_host_project_compiles_on_its_own() {
    let dir = work_dir("host_project");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nbackend = \"*\"\n\n[kotlin]\nplatform = \"kt\"\n\n[rust]\nplatform = \"rs\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.sv"),
        "\
struct Greeting {
    text: Str,
    loud: Bool
}

struct Unused {
    n: Int
}

effect Greeter {
    fn greet(g: Greeting) [] -> Str => g
}

platform handler HostGreeter() of Greeter

platform fn shout(s: Str) [] -> Str => s

fn main() [use] {
    use StdOutConsole()
    use HostGreeter()
    println(shout(greet(Greeting { text: \"hi\", loud: true })))
}
",
    )
    .unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!dir.join("kt").exists() && !dir.join("rs").exists(), "analyze writes nothing");

    fs::create_dir_all(dir.join("kt")).unwrap();
    let stale = "// GENERATED by salvo for the host project of this platform root [platform-abi]:\n";
    fs::write(dir.join("kt/gone.sv.kt"), stale).unwrap();
    fs::write(dir.join("kt/mine.sv.kt"), "// hand-written\n").unwrap();
    let out = salvo_in(&dir, &["platform", "generate"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!dir.join("kt/gone.sv.kt").exists(), "a stale generated file is removed");
    assert!(dir.join("kt/mine.sv.kt").exists(), "a hand-written one is kept");
    fs::remove_file(dir.join("kt/mine.sv.kt")).unwrap();

    let kt = fs::read_to_string(dir.join("kt/main.sv.kt")).unwrap();
    assert!(kt.contains("data class Greeting(") && kt.contains("interface Greeter"), "{kt}");
    assert!(!kt.contains("Unused") && !kt.contains("fun main"), "declarations only, and only the reached: {kt}");
    let gradle = fs::read_to_string(dir.join("kt/build.gradle.kts")).unwrap();
    assert!(gradle.contains("kotlin(\"jvm\")"), "{gradle}");
    let rs = fs::read_to_string(dir.join("rs/lib.sv.rs")).unwrap();
    assert!(rs.contains("pub struct Greeting") && rs.contains("pub mod platform_main;"), "{rs}");
    assert!(!rs.contains("Unused") && !rs.contains("fn main"), "{rs}");
    let cargo = fs::read_to_string(dir.join("rs/Cargo.toml")).unwrap();
    assert!(cargo.contains("path = \"lib.sv.rs\""), "{cargo}");

    // Implement the skeletons, then compile each root alone.
    let edit = |path: PathBuf, pairs: &[(&str, &str)]| {
        let mut s = fs::read_to_string(&path).unwrap();
        for (a, b) in pairs {
            assert!(s.contains(a), "{a} not in {s}");
            s = s.replace(a, b);
        }
        fs::write(path, s).unwrap();
    };
    edit(dir.join("kt/main.kt"), &[("TODO(\"implement shout\")", "return s.uppercase()"), ("TODO(\"implement Greeter.greet\")", "return g.text")]);
    edit(dir.join("rs/main.rs"), &[("todo!(\"implement shout\")", "s.to_uppercase()"), ("todo!(\"implement Greeter.greet\")", "g.text.clone()")]);
    let Some(stamp) = e2e_stamp("host project compiles alone", &["kotlinc", "rustc"]) else { return };
    if have("kotlinc") {
        let mut sources = Vec::new();
        let mut stack = vec![dir.join("kt")];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "kt") {
                    sources.push(p);
                }
            }
        }
        let compiled = Command::new("kotlinc")
            .args(&sources)
            .args(["-nowarn", "-d"])
            .arg(dir.join(".kt_classes"))
            .output()
            .expect("failed to run kotlinc");
        assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    }
    if have("rustc") {
        let compiled = Command::new("rustc")
            .args(["--edition", "2021", "--crate-type", "lib", "--crate-name", "host", "lib.sv.rs", "-o"])
            .arg(dir.join("libhost.rlib"))
            .current_dir(dir.join("rs"))
            .output()
            .expect("failed to run rustc");
        assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    }
    stamp.verified();
}

/// Copies a project's `salvo.toml` and source tree to `to`, leaving out
/// hidden directories and host-tool output.
fn copy_project(from: &Path, to: &Path) {
    let _ = fs::remove_dir_all(to);
    for (rel, bytes) in tree_contents(from) {
        if rel.starts_with("codegen/") || rel.starts_with("demo/") || rel.ends_with("Cargo.lock") {
            continue;
        }
        let path = to.join(&rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

/// The generated files of a platform root, by their header.
fn generated_files(root: &Path) -> Vec<(String, Vec<u8>)> {
    tree_contents(root)
        .into_iter()
        .filter(|(_, b)| b.starts_with(b"// GENERATED by salvo") || b.starts_with(b"# GENERATED by salvo"))
        .collect()
}

/// [platform-abi] The host projects checked in beside std's and aws's
/// implementation files are what the compiler generates now (a dependency
/// checks in its own, ABI.md D9), and aws's implementation files compile
/// against them alone — `cargo check` on the root's `Cargo.toml`, Gradle's
/// `compileKotlin` on its `build.gradle.kts` — which is the setup an IDE
/// opening the root gets. Skips the compiles, saying so, when the SDKs or
/// the Kotlin Gradle plugin are not available offline.
#[test]
fn the_checked_in_host_projects_are_current_and_compile() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    for (name, root) in [("std", "platform"), ("aws", "salvo/platform")] {
        let src = if name == "std" { repo.join("std") } else { repo.join("modules/aws") };
        let copy = tmp.join(format!("host_project_{name}"));
        copy_project(&src, &copy);
        let out = salvo_in(&copy, &["platform", "generate", "--backend", "*"]);
        assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
        let fresh = generated_files(&copy.join(root));
        assert!(!fresh.is_empty(), "{name}: no host project generated");
        let checked_in = generated_files(&src.join(root));
        let names = |t: &[(String, Vec<u8>)]| t.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();
        assert_eq!(names(&checked_in), names(&fresh), "{name}: regenerate with `salvo platform generate --backend '*'` in {}", src.display());
        for ((path, a), (_, b)) in checked_in.iter().zip(&fresh) {
            assert!(a == b, "{name}: `{path}` is stale: regenerate with `salvo platform generate --backend '*'` in {}", src.display());
        }
        if name != "aws" {
            continue;
        }
        let tree = tree_contents(&copy.join(root));
        if have("cargo") && have("rustc") {
            if let Some(stamp) = content_stamp("aws host crate", &tree, &["rustc", "cargo"]) {
                let check = Command::new("cargo")
                    .args(["check", "--offline", "--quiet", "--manifest-path"])
                    .arg(copy.join(root).join("Cargo.toml"))
                    .env("CARGO_TARGET_DIR", tmp.join("aws_glue_cargo"))
                    .output()
                    .expect("failed to run cargo");
                let stderr = String::from_utf8_lossy(&check.stderr);
                if !check.status.success() && (stderr.contains("offline") || stderr.contains("no matching package")) {
                    eprintln!("skipping aws's host crate: the SDK crates are not in cargo's cache");
                } else {
                    assert!(check.status.success(), "aws's host crate does not compile:\n{stderr}");
                    stamp.verified();
                }
            }
        }
        if have("java") {
            if let Some(stamp) = content_stamp("aws host gradle project", &tree, &["java"]) {
                let built = Command::new(repo.join("modules/aws/codegen/gradlew"))
                    .args(["--offline", "--quiet", "--project-dir"])
                    .arg(copy.join(root))
                    .arg("compileKotlin")
                    .output()
                    .expect("failed to run the aws Gradle wrapper");
                let stderr = String::from_utf8_lossy(&built.stderr);
                if !built.status.success() && (stderr.contains("was not found in any of the following sources") || stderr.contains("Could not resolve")) {
                    eprintln!("skipping aws's Gradle project: the Kotlin plugin or the SDK is not cached:\n{stderr}");
                } else {
                    assert!(built.status.success(), "aws's Gradle project does not compile:\n{stderr}");
                    stamp.verified();
                }
            }
        }
    }
}

/// [platform-check] The adapters and wrappers check what the host hands
/// back (ABI.md D7, D10 C3): a closed literal union, a literal field of a
/// struct inside a list (element by element, which warns), a state qualifier
/// (`NonEmpty`, by running its `qualifies`), a literal arm of a positional
/// union, and a `Reply<T>` the host completes from its own thread — the hosts
/// building their unions with the factories [platform-factory] (a named
/// union's `Problems.notFound` / `Problem::not_found`, a signature's
/// `Open.err`, a literal arm's checked `Find.str`). Good values
/// pass on both backends; a bad one fails where it crosses — Rust panics,
/// Kotlin throws — naming the declaration and the value.
#[test]
fn the_boundary_checks_what_the_host_returns() {
    let Some(stamp) = e2e_stamp("the_boundary_checks_what_the_host_returns", &["kotlinc", "rustc"]) else {
        return;
    };
    const PROGRAM: &str = "\
struct Item {
    tier: \"gold\" | \"silver\",
    tags: List<\"a\" | \"b\">
}

platform fn tier(n: Int) [] -> \"gold\" | \"silver\" | None

platform fn items(n: Int) [] -> List<Item>

platform fn names(n: Int) [] -> NonEmpty List<Str>

struct NotFound { path: Str }
struct Denied { path: Str }
type Problem = NotFound | Denied

platform fn open(p: Str) [] -> Ok Str | Err Problem => p

effect Lookup {
    fn find(key: Str) [] -> Int | \"missing\" => key
    fn parity(n: Int, done: Reply<\"even\" | \"odd\">) [] -> None => !n, !done
}

threadsafe platform handler HostLookup() of Lookup

fn show(t: \"gold\" | \"silver\" | None) -> Str {
    when t {
        is None { return \"none\" }
        is \"gold\" { return \"gold\" }
        is \"silver\" { return \"silver\" }
    }
}

fn main() [use] {
    use StdOutConsole()
    use HostLookup()
    println(\"${show(tier(1))} ${show(tier(2))}\")
    println(\"${items(1).size()} ${names(1)}\")
    let o = open(\"x\")
    when o {
        is Ok { println(\"ok\") }
        is ^Err {
            when o {
                is NotFound { println(\"not found ${o.path}\") }
                is Denied { println(\"denied\") }
            }
        }
    }
    let found = find(\"x\")
    when found {
        is Int { println(\"int\") }
        is \"missing\" { println(\"missing\") }
    }
    let p = waitfor done: Reply<\"even\" | \"odd\"> { parity(PARITY, done) }
    println(\"${p}\")
    println(show(tier(TIER)))
}
";
    let hosts = [
        (
            "rust",
            "rustc",
            "rs",
            vec![
                ("todo!(\"implement tier\")", "match n { 1 => Some(\"gold\".to_string()), 2 => None, _ => Some(\"bronze\".to_string()) }"),
                ("todo!(\"implement items\")", "vec![Item { tier: \"gold\".to_string(), tags: vec![\"a\".to_string(), \"b\".to_string()] }]"),
                ("todo!(\"implement names\")", "if n > 0 { vec![\"x\".to_string()] } else { vec![] }"),
                ("todo!(\"implement open\")", "Open::err(Problem::not_found(NotFound { path: p.clone() }))"),
                ("todo!(\"implement Lookup.find\")", "Find::str(\"missing\".to_string())"),
                (
                    "todo!(\"implement Lookup.parity\")",
                    "let done = done.hosted();\n        std::thread::spawn(move || {\n            \
                     let v = if n < 0 { \"neither\" } else if n % 2 == 0 { \"even\" } else { \"odd\" };\n            \
                     done.send(v.to_string())\n        });",
                ),
            ],
        ),
        (
            "kotlin",
            "kotlinc",
            "kt",
            vec![
                ("TODO(\"implement tier\")", "return when (n) { 1 -> \"gold\"; 2 -> null; else -> \"bronze\" }"),
                ("TODO(\"implement items\")", "return listOf(Item(tier = \"gold\", tags = listOf(\"a\", \"b\")))"),
                ("TODO(\"implement names\")", "return if (n > 0) listOf(\"x\") else listOf()"),
                ("TODO(\"implement open\")", "return Open.err(Problems.notFound(NotFound(p)))"),
                ("TODO(\"implement Lookup.find\")", "return Find.str(\"missing\")"),
                (
                    "TODO(\"implement Lookup.parity\")",
                    "val host = done.hosted()\n        Thread {\n            \
                     val v = if (n < 0) \"neither\" else if (n % 2 == 0) \"even\" else \"odd\"\n            \
                     try { host.send(v) } catch (e: IllegalStateException) { System.err.println(e.message); System.exit(3) }\n        }.start()",
                ),
            ],
        ),
    ];
    for (backend, tool, ext, edits) in hosts {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("boundary_{backend}"));
        project(&dir);
        let program = |parity: &str, tier: &str| PROGRAM.replace("PARITY", parity).replace("TIER", tier);
        fs::write(dir.join("main.sv"), program("3", "1")).unwrap();
        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let host = dir.join("platform").join(format!("main.{ext}"));
        let mut src = fs::read_to_string(&host).unwrap();
        for (a, b) in &edits {
            assert!(src.contains(a), "{backend}: `{a}` not in the skeleton:\n{src}");
            src = src.replace(a, b);
        }
        fs::write(&host, src).unwrap();

        let out = salvo_in(&dir, &["run", "--backend", backend]);
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "gold none\n1 [x]\nnot found x\nmissing\nodd\ngold\n", "{backend}: {stderr}");
        assert!(
            stderr.contains("`platform fn items`'s result is checked element by element"),
            "{backend}: the walk is not warned about: {stderr}"
        );

        for (parity, tier, says) in [
            ("3", "3", "`platform fn tier`'s result was"),
            ("-1", "1", "what the host sent on `Lookup.parity` was"),
        ] {
            fs::write(dir.join("main.sv"), program(parity, tier)).unwrap();
            let out = salvo_in(&dir, &["run", "--backend", backend]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(!out.status.success(), "{backend}: a bad value passed: {stderr}");
            assert!(stderr.contains(says) && stderr.contains("[platform-check]"), "{backend}: {stderr}");
        }
    }
    stamp.verified();
}

/// [platform-stamp] A dependency checks in its host project, stamped with the
/// ABI revision and a hash of its platform signatures (ABI.md D9 (b)); a build
/// that compiles the dependency's implementation files compares the stamps
/// first and names what to do: no host project, signatures changed since it
/// was generated, or another ABI revision. A build that does not reach the
/// dependency's platform code does not look.
#[test]
fn a_dependencys_host_project_is_checked_before_its_code() {
    let dir = work_dir("stamps");
    let lib = dir.join("salvo_modules/lib");
    fs::create_dir_all(lib.join("salvo/lib")).unwrap();
    fs::create_dir_all(lib.join("salvo/platform/lib")).unwrap();
    fs::write(
        lib.join("salvo.toml"),
        "[project]\nname = \"lib\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nplatform = \"salvo/platform\"\n",
    )
    .unwrap();
    fs::write(
        lib.join("salvo/lib.sv"),
        "export effect Greet {\n    fn greet() -> Str\n}\n\nexport handler Plain() of Greet {\n    \
         fn greet() -> Str { return \"plain\" }\n}\n",
    )
    .unwrap();
    fs::write(lib.join("salvo/lib/host.sv"), "import lib\n\nexport platform handler HostGreet of Greet\n").unwrap();
    fs::write(
        lib.join("salvo/platform/lib/host.rs"),
        "use crate::lib::*;\npub struct HostGreet;\nimpl HostGreet { pub fn new() -> Self { HostGreet } }\n\
         impl crate::lib::GreetPlatform for HostGreet { fn greet(&mut self) -> String { \"host\".into() } }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("salvo")).unwrap();
    fs::write(
        dir.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"rust\"\n\
         modules = \"salvo_modules\"\n\n[dependencies]\nlib = \"0.1.0\"\n",
    )
    .unwrap();
    let compile = || {
        let out = salvo_in(&dir, &["compile", "--target", "out"]);
        (out.status.success(), String::from_utf8_lossy(&out.stderr).to_string())
    };
    // Not reached: nothing is checked.
    fs::write(dir.join("salvo/main.sv"), "import lib\n\nfn main() [use] {\n    use StdOutConsole()\n    use Plain()\n    println(greet())\n}\n").unwrap();
    let (ok, stderr) = compile();
    assert!(ok, "{stderr}");
    // Reached, with no host project.
    fs::write(
        dir.join("salvo/main.sv"),
        "import lib\nimport lib.host\n\nfn main() [use] {\n    use StdOutConsole()\n    use HostGreet()\n    println(greet())\n}\n",
    )
    .unwrap();
    let (ok, stderr) = compile();
    assert!(!ok && stderr.contains("dependency `lib` has platform code but no generated host project"), "{stderr}");
    assert!(stderr.contains("salvo platform generate --backend rust") && stderr.contains("[platform-stamp]"), "{stderr}");
    // Generated: accepted.
    let out = salvo_in(&lib, &["platform", "generate", "--backend", "rust"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let (ok, stderr) = compile();
    assert!(ok, "{stderr}");
    // The signatures changed and nobody regenerated.
    let host_sv = "import lib\n\nexport platform handler HostGreet of Greet\n";
    fs::write(lib.join("salvo/lib/host.sv"), format!("{host_sv}\nexport platform fn shout(s: Str) [] -> Str => s\n")).unwrap();
    let (ok, stderr) = compile();
    assert!(!ok && stderr.contains("generated from platform signatures that have changed since"), "{stderr}");
    // Another compiler's ABI revision.
    fs::write(lib.join("salvo/lib/host.sv"), host_sv).unwrap();
    let out = salvo_in(&lib, &["platform", "generate", "--backend", "rust"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stamped = lib.join("salvo/platform/lib.sv.rs");
    let text = fs::read_to_string(&stamped).unwrap();
    assert!(text.contains("// salvo-abi 1 "), "{text}");
    fs::write(&stamped, text.replace("// salvo-abi 1 ", "// salvo-abi 0 ")).unwrap();
    let (ok, stderr) = compile();
    assert!(!ok && stderr.contains("was generated for ABI 0, and this compiler's is 1"), "{stderr}");
}

/// [platform-check] D10 C1/C2: a set or map the host returns keeps Salvo's
/// order. On Kotlin, a top-level result built the host's way (`hashSetOf`, a
/// `TreeSet` under natural ordering, which sorts by UTF-16 code unit) is copied
/// into Salvo's shape; one inside another value cannot be replaced, so it is
/// refused naming the constructor to use. Rust's types promise the shape, and
/// `collect()` builds them under the canonical identity.
#[test]
fn sets_and_maps_from_the_host_keep_salvos_order() {
    let Some(stamp) = e2e_stamp("sets_and_maps_from_the_host", &["kotlinc", "rustc"]) else { return };
    const PROGRAM: &str = "\
struct Bag {
    tags: Set<Str>
}

platform fn names() [] -> Set<Str>

platform fn sorted() [] -> SortedSet<Str>

platform fn ages() [] -> Map<Str, Int>

platform fn bag(ok: Bool) [] -> Bag

fn main() [use] {
    use StdOutConsole()
    println(\"${size(names())} ${sorted()} ${size(ages())} ${size(bag(true).tags)}\")
    LAST
}
";
    let hosts = [
        (
            "kotlin",
            "kotlinc",
            "kt",
            vec![
                ("TODO(\"implement names\")", "return hashSetOf(\"b\", \"a\", \"c\")"),
                ("TODO(\"implement sorted\")", "return java.util.TreeSet(listOf(\"\\uFF61\", \"\\uD83D\\uDE00\"))"),
                ("TODO(\"implement ages\")", "return hashMapOf(\"x\" to 1, \"y\" to 2)"),
                ("TODO(\"implement bag\")", "return Bag(tags = if (ok) linkedSetOf(\"a\", \"b\") else hashSetOf(\"a\", \"b\", \"c\"))"),
            ],
        ),
        (
            "rust",
            "rustc",
            "rs",
            vec![
                ("todo!(\"implement names\")", "[\"b\", \"a\", \"c\"].iter().map(|s| s.to_string()).collect()"),
                ("todo!(\"implement sorted\")", "[\"\\u{FF61}\", \"\\u{1F600}\"].iter().map(|s| s.to_string()).collect()"),
                ("todo!(\"implement ages\")", "[(\"x\".to_string(), 1), (\"y\".to_string(), 2)].into_iter().collect()"),
                ("todo!(\"implement bag\")", "let _ = ok;\n    Bag { tags: [\"a\", \"b\"].iter().map(|s| s.to_string()).collect() }"),
            ],
        ),
    ];
    for (backend, tool, ext, edits) in hosts {
        if !have(tool) {
            eprintln!("skipping {backend}: {tool} not found on PATH");
            continue;
        }
        let dir = work_dir(&format!("shapes_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), PROGRAM.replace("LAST", "")).unwrap();
        let out = salvo_in(&dir, &["platform", "generate", "--backend", backend]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let host = dir.join("platform").join(format!("main.{ext}"));
        let mut src = fs::read_to_string(&host).unwrap();
        for (a, b) in &edits {
            assert!(src.contains(a), "{backend}: `{a}` not in:\n{src}");
            src = src.replace(a, b);
        }
        fs::write(&host, src).unwrap();
        let out = salvo_in(&dir, &["run", "--backend", backend]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "3 {\u{FF61}, \u{1F600}} 2 2\n", "{backend}: {stderr}");
        if backend == "kotlin" {
            fs::write(dir.join("main.sv"), PROGRAM.replace("LAST", "println(\"${size(bag(false).tags)}\")")).unwrap();
            let out = salvo_in(&dir, &["run", "--backend", backend]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                !out.status.success() && stderr.contains("is a set that keeps no insertion order: build it with `linkedSetOf(…)`"),
                "{stderr}"
            );
        }
    }
    stamp.verified();
}

/// [platform-check] D10 C2: a host cannot build a collection keyed by a
/// Salvo-defined identity — a written one (`SortedSet<Str>(by_len)`) or one
/// filled by name from a hand-written `hash` — so a result of one is refused.
/// The canonical identity passes, and so does one `by auto` stamped.
#[test]
fn a_collection_keyed_by_a_salvo_identity_is_refused_at_the_boundary() {
    let dir = work_dir("keyed");
    project(&dir);
    fs::write(
        dir.join("main.sv"),
        "\
struct Person : Hashed<self> by auto {
    name: Str
}

struct Tag {
    name: Str
}

fn hash(t: Tag) [] -> Long => t {
    return to_long(size(t.name))
}

fn eq(a: Tag, b: Tag) [] -> Bool => a, b {
    return a.name == b.name
}

fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return size(a) - size(b)
}

platform fn sorted() [] -> SortedSet<Str>(by_len)

platform fn plain() [] -> SortedSet<Str>

platform fn people() [] -> Set<Person>

platform fn tags() [] -> Set<Tag>
",
    )
    .unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let errors: Vec<&str> = stderr.lines().filter(|l| l.starts_with("error:")).collect();
    assert!(errors.len() == 2, "{stderr}");
    assert!(
        errors.iter().any(|l| l.contains("`platform fn sorted`'s result is `SortedSet<Str>(by_len)`, a collection keyed by a Salvo-defined identity")),
        "{stderr}"
    );
    assert!(errors.iter().any(|l| l.contains("`platform fn tags`'s result is `Set<Tag>`")), "{stderr}");
}
