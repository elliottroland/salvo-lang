//! Integration tests for `salvo platform generate` and the `platform/` tree
//! [cli-platform] [platform-tree].
//!
//! The arc these cover is the one a developer actually walks: write a
//! `platform handler`, discover from the error that the host is missing, run
//! the command, implement the stub, run the program. Tests needing a
//! toolchain (`kotlinc`, `rustc`) skip gracefully when it is missing.
//!
//! What a program does across the boundary — a platform fn's result, a
//! callback, a host thread's reply, a check that fails — is run data
//! (`tests/run/platform/`, [test-run-data]); the tests here check what
//! `generate` writes, what the build refuses or warns about, and the host
//! projects and tools.

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
    use StdConsole()
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
        "fn main() [use] -> None {\n    use StdConsole\n    \
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
    // Keyed on the binary: `platform generate` writes host files mid-test.
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

/// Runs `salvo platform generate` for `backend` in `dir` and answers the
/// implementation file it wrote for `module` (`platform/<module>.<ext>`).
fn generate(dir: &Path, backend: &str, module: &str) -> String {
    let out = salvo_in(dir, &["platform", "generate", "--backend", backend, "--src", "."]);
    assert!(out.status.success(), "{backend}: {}", String::from_utf8_lossy(&out.stderr));
    let ext = if backend == "rust" { "rs" } else { "kt" };
    fs::read_to_string(dir.join("platform").join(format!("{module}.{ext}"))).unwrap()
}

/// [platform-never] A platform fn and a platform handler member answering
/// `Never` are `-> !` / `: Nothing` on the host side, so a host body that
/// returns is the host compiler's error, not a silent fall-through: the
/// skeletons say so. That a host which exits ends the program is run data
/// (`tests/run/platform/boundary.sv`).
#[test]
fn a_platform_member_answering_never_cannot_return() {
    const SRC: &str = r#"
effect Stop {
    fn halt(why: Str) [] -> Never => why
}

platform handler HostStop() of Stop

platform fn bail(why: Str) [] -> Never => why

fn main() [use] {
    use StdConsole()
    use HostStop()
    println("before")
    if size("x") > 5 {
        halt("never")
    }
    bail("stop")
}
"#;
    for (backend, sigs) in [
        ("kotlin", ["fun bail(why: String): Nothing {", "override fun halt(why: String): Nothing {"]),
        ("rust", ["pub fn bail(why: &String) -> ! {", "fn halt(&mut self, why: &String) -> ! {"]),
    ] {
        let dir = work_dir(&format!("never_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), SRC).unwrap();
        let skeleton = generate(&dir, backend, "main");
        for sig in sigs {
            assert!(skeleton.contains(sig), "{backend} skeleton lacks `{sig}`:\n{skeleton}");
        }
    }
}

/// [platform-type] Platform types, declared in a module of their own:
/// `generate` writes a class/struct per type with the contract its kind
/// promises. On Rust a host type breaking the contract (not `Clone`) is
/// rustc's error at the declaration's assertion, and a handle in an
/// `encode` is the checker's. What a program does with the types — a shared
/// copy, a linear handle mutated and closed, a handle sent to an actor — is
/// run data (`tests/run/platform/boundary.sv#platform_types`).
#[test]
fn platform_types_keep_their_contracts() {
    // Keyed on the binary: the host files come from `platform generate`.
    let Some(__stamp) = e2e_stamp("platform_types_keep_their_contracts", &["rustc"]) else {
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
    use StdConsole()
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
    // The Rust implementation, with the contract kept.
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
    for (backend, skeleton_line) in [("kotlin", "class Cursor {"), ("rust", "#[derive(Clone)]\npub struct Tally {")] {
        let dir = work_dir(&format!("ptype_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let skeleton = generate(&dir, backend, "host");
        assert!(skeleton.contains(skeleton_line), "{backend} skeleton:\n{skeleton}");
        if backend == "rust" && have("rustc") {
            let path = dir.join("platform/host.rs");
            fs::write(&path, HOST_RS.replacen("#[derive(Clone)]\npub struct Tally", "pub struct Tally", 1)).unwrap();
            let out = salvo_in(&dir, &["run", "--backend", backend, "--src", "."]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(!out.status.success() && stderr.contains("__contract"), "{stderr}");
        }
    }
    // [net-noremote] A host handle has no wire form.
    let dir = work_dir("ptype_wire");
    project(&dir);
    fs::write(dir.join("host.sv"), HOST_SV).unwrap();
    fs::write(dir.join("main.sv"), "import host\nimport std.codec\n\nfn main() [] {\n    let _b = encode(new_tally(1))\n}\n").unwrap();
    let out = salvo_in(&dir, &["analyze", "--src", "."]);
    let all = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(all.contains("`Tally` is declared `noremote`"), "{all}");
    __stamp.verified();
}

/// [platform-fn-value] Function values at the boundary: the skeletons give
/// the host its own closure types — for a platform fn's callback, and for a
/// platform handler member's, which on Rust is `&mut dyn FnMut`, since an
/// effect trait is used as `dyn`. The callbacks crossing (called twice,
/// consumed, faulting) are run data (`tests/run/platform/boundary.sv`).
#[test]
fn function_values_cross_the_platform_boundary() {
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
    use StdConsole()
    let k = 5
    println("${twice(n -> n + k, 1)}")
    let label = "late"
    println(call(() -> "${label} ran"))
    let why = run_guarded(() -> {
        let xs: List<Int> = List<Int>()
        let _x = get(xs, 3) ?: unreachable!()
    })
    println("faulted: ${!(why is None)}")
    println("${run_guarded(() -> {}) is None}")
    use HostApply()
    println("${apply(n -> n * k, 2)}")
}
"#;
    for (backend, sigs) in [
        ("kotlin", ["fun call(body: () -> String): String", "override fun apply(f: (Int) -> Int, x: Int): Int"]),
        ("rust", ["pub fn call(body: impl FnOnce() -> String) -> String", "fn apply(&mut self, f: &mut dyn FnMut(i32) -> i32, x: i32) -> i32"]),
    ] {
        let dir = work_dir(&format!("pfn_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let skeleton = generate(&dir, backend, "host");
        for sig in sigs {
            assert!(skeleton.contains(sig), "{backend} skeleton lacks `{sig}`:\n{skeleton}");
        }
    }
}

/// [platform-fn-value] A platform fn that **keeps** a fn value (`=> !hook`
/// on a plain fn type) takes a named top-level fn: Rust's host gets a `fn`
/// pointer, Kotlin's its own fn type, and a lambda in that position is
/// refused. The host storing it and calling it later is run data
/// (`tests/run/platform/boundary.sv#a_platform_fn_keeps_a_named_fn`).
#[test]
fn a_platform_fn_keeps_a_named_fn() {
    const HOST_SV: &str = r#"export platform fn keep(hook: (n: Int, label: Str) -> Str) [] -> None => !hook
export platform fn fire(n: Int) [] -> Str => n
"#;
    const MAIN_SV: &str = r#"import host

fn shout(n: Int, label: Str) [] -> Str => n, label {
    return "${label} ${n * 2}"
}

fn main() [use] {
    use StdConsole()
    keep(shout)
    println(fire(4))
    println(fire(5))
}
"#;
    for (backend, sig) in [
        ("kotlin", "fun keep(hook: (Int, String) -> String)"),
        ("rust", "pub fn keep(hook: fn(i32, &String) -> String)"),
    ] {
        let dir = work_dir(&format!("pkeep_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let skeleton = generate(&dir, backend, "host");
        assert!(skeleton.contains(sig), "{backend} skeleton lacks `{sig}`:\n{skeleton}");
    }
    // A lambda (or a local) cannot be kept.
    let dir = work_dir("pkeep_lambda");
    project(&dir);
    fs::write(dir.join("host.sv"), HOST_SV).unwrap();
    fs::write(
        dir.join("main.sv"),
        "import host\n\nfn main() [] {\n    let k = 3\n    keep((n, label) -> \"${label} ${n + k}\")\n}\n",
    )
    .unwrap();
    let out = salvo_in(&dir, &["analyze", "--src", "."]);
    let all = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(all.contains("takes a named top-level fn"), "{all}");
}

/// [platform-iterable] A customer's `iterable platform type`: `salvo platform
/// generate` writes the `each` stub beside the type. A `for` over a value
/// looping over the host's `each`, and the Salvo pass agreeing with it, are
/// run data (`tests/run/platform/boundary.sv`).
#[test]
fn an_iterable_platform_type_loops_over_its_hosts_each() {
    const HOST_SV: &str = r#"export iterable platform type Bag : Iter<self, BagIter, Int>
export platform fn bag_of(a: Int, b: Int) [] -> Bag
export platform fn bag_size(b: Bag) [] -> Int => b
export platform fn bag_at(b: Bag, i: Int) [] -> Int => b

export iter fn iter(b: Bag) -> Emitted Int | Finished {
    state BagIter {
        at: Int = 0
    }
    if at >= bag_size(b) {
        return finished()
    }
    at = at + 1
    return emitted(bag_at(b, at - 1))
}
"#;
    const MAIN_SV: &str = r#"import host

fn main() [use] {
    use StdConsole()
    let b = bag_of(3, 4)
    for x in b {
        println("each ${x}")
    }
    println("pass ${to_list(iter(b))}")
}
"#;
    for (backend, sig) in [
        ("kotlin", "fun each(x: Bag): Iterable<Int>"),
        ("rust", "pub fn each(x: &Bag) -> impl Iterator<Item = i32> + '_"),
    ] {
        let dir = work_dir(&format!("iterable_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let skeleton = generate(&dir, backend, "host");
        assert!(skeleton.contains(sig), "{backend} skeleton lacks `{sig}`:\n{skeleton}");
    }
}

/// [platform-generic] Generic platform types and fns: the host's `T` is
/// opaque — Rust bounds it `Send + 'static`, never `Clone` — as the
/// skeletons say. The host storing, moving and handing back a `T` (an
/// erased payload, a linear `T`) is run data
/// (`tests/run/platform/boundary.sv#generic_platform_types_and_fns`).
#[test]
fn generic_platform_skeletons_keep_their_t_opaque() {
    const HOST_SV: &str = r#"export linear platform type Cell<T canbe linear>
export linear platform type Dyn

export platform fn cell_of<T canbe linear>(v: T) [] -> Cell<T> => !v
export platform fn take<T canbe linear>(c: Cell<T>) [] -> T => !c
export platform fn erase<T canbe linear>(v: T) [] -> Dyn => !v
export platform fn unerase<T canbe linear>(d: Dyn) [] -> T => !d
"#;
    const MAIN_SV: &str = r#"import host

struct Point { x: Int, y: Int }

fn main() [use] {
    use StdConsole()
    let c = cell_of("hi")
    println(take(c))
    let d = erase(Point { x: 1, y: 2 })
    let p: Point = unerase(d)
    println("${p.x} ${p.y}")
    let n: Int = unerase(erase(7))
    println("${n}")
    tickets()
}

linear struct Ticket { id: Int }

fn redeem(t: Ticket) [Console] -> None => !t {
    println("redeemed ${t.id}")
    discard(t)
}

fn tickets() [Console] -> None {
    let back: Ticket = take(cell_of(unerase<Ticket>(erase(Ticket { id: 9 }))))
    redeem(back)
}
"#;
    for (backend, sig) in [
        ("kotlin", "fun<T> unerase(d: salvo.platform.host.Dyn): T {"),
        ("rust", "pub fn unerase<T: Send + 'static>(d: crate::host::Dyn) -> T {"),
    ] {
        let dir = work_dir(&format!("pgen_{backend}"));
        project(&dir);
        fs::write(dir.join("host.sv"), HOST_SV).unwrap();
        fs::write(dir.join("main.sv"), MAIN_SV).unwrap();
        let skeleton = generate(&dir, backend, "host");
        assert!(skeleton.contains(sig), "{backend} skeleton:\n{skeleton}");
    }
}

/// [platform-tree] The tree mirrors the sources, per module: the implementation
/// of a handler declared in `telemetry.sv` is `platform/telemetry.<ext>`, and
/// a `use` of it from another module (`bin/tool.sv`, chosen with `--main`)
/// constructs it across modules. Run end to end on both backends, because
/// the cross-module path is where a mistake would be a target-language error
/// rather than anything Salvo would notice.
#[test]
fn the_platform_tree_mirrors_the_source_tree() {
    // Keyed on the binary: `platform generate` writes host files mid-test.
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

/// [platform-host-deps] [rs-cargo] [kt-classpath] [kt-gradle] [backend-host-tool] A platform companion that
/// uses a host library: the manifest declares it — a `path` crate under
/// `[rust] crates`, a jar directory under `[kotlin] libs` — and `salvo run`
/// builds with it: Rust through an emitted `Cargo.toml` and `cargo build`,
/// Kotlin with the jars on both classpaths. Same source, same stdout. The
/// Rust half also checks that `compile` writes the manifest and the hint
/// names cargo, and that dropping the crates removes the stale manifest.
#[test]
fn a_companion_can_use_a_declared_host_library() {
    // Keyed on the binary: `platform generate` writes host files, and the
    // manifest is rewritten between commands.
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
    use StdConsole()
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
        // [backend-host-tool] `[rust] cargo` names the cargo to run: here a wrapper
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

        // [kt-gradle] [backend-host-tool] `artifacts` are resolved by the Gradle
        // `[kotlin] gradle` names — here a stand-in speaking the same
        // protocol (the real one runs in `modules/aws/check.sh`): it is given the
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

/// [backend-host-tool] A build tool the manifest names that is not there is reported
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
        fs::write(dir.join("main.sv"), "fn main() [use] {\n    use StdConsole()\n    println(\"x\")\n}\n").unwrap();
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
    fs::write(dir.join("salvo/main.sv"), "fn main() [use] {\n    use StdConsole()\n    println(\"x\")\n}\n").unwrap();
    let out = salvo_in(&dir, &["analyze"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        stderr.contains("`serde`") && stderr.contains("`1.0`") && stderr.contains("`2.0`") && stderr.contains("one version per crate"),
        "{stderr}"
    );
}

/// [platform-reply] A platform member that takes a `Reply` returns at once
/// and completes the reply later from a host thread: the skeleton states
/// that contract (`.hosted()` takes the token, `send` completes it). The
/// host thread completing it, and on Rust a reply dropped unsent reported
/// as such, are run data (`tests/run/platform/boundary.sv`).
#[test]
fn a_host_thread_completes_a_reply() {
    const PROGRAM: &str = "\
effect Slow {
    fn later(n: Int, done: Reply<Int>) [] -> None => !n, !done
}

threadsafe platform handler HostSlow of Slow

send fn labelled(label: Str, out: Reply<Str>, n: Int) => !label, !out, !n {
    out.send(\"${label} ${n}\")
}

fn main() [use] {
    use StdConsole()
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
    for backend in ["rust", "kotlin"] {
        let dir = work_dir(&format!("reply_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), PROGRAM).unwrap();
        let src = generate(&dir, backend, "main");
        assert!(
            src.contains("`done`: a continuation [platform-reply]") && src.contains(".hosted()"),
            "{backend} skeleton states no contract:\n{src}"
        );
    }
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
    use StdConsole()
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
    use StdConsole()
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
    // Keyed on the binary: the host project is `platform generate`'s output.
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
        if rel.ends_with("Cargo.lock") {
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

/// [platform-abi] The host project checked in beside std's implementation
/// files is what the compiler generates now (a dependency checks in its own,
/// ABI D9). A dependency's own host project — `modules/aws`'s — is that
/// module's to check (`modules/aws/check.sh`), not the compiler's.
#[test]
fn the_checked_in_host_project_is_current() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let src = repo.join("lib");
    let copy = tmp.join("host_project_std");
    copy_project(&src, &copy);
    let out = salvo_in(&copy, &["platform", "generate", "--backend", "*"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let fresh = generated_files(&copy.join("platform"));
    assert!(!fresh.is_empty(), "no host project generated");
    let checked_in = generated_files(&src.join("platform"));
    let names = |t: &[(String, Vec<u8>)]| t.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();
    assert_eq!(names(&checked_in), names(&fresh), "regenerate with `salvo platform generate --backend '*'` in {}", src.display());
    for ((path, a), (_, b)) in checked_in.iter().zip(&fresh) {
        assert!(a == b, "`{path}` is stale: regenerate with `salvo platform generate --backend '*'` in {}", src.display());
    }
}

/// [platform-check] The adapters and wrappers check what the host hands
/// back (ABI D7, D10 C3): the skeleton has a stub for each member, and a
/// literal field of a struct inside a list is checked element by element,
/// which the build warns about. Good values passing on both backends, and a
/// bad one failing where it crosses, are run data
/// (`tests/run/platform/host_values.sv`).
#[test]
fn the_boundary_checks_what_the_host_returns() {
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
    use StdConsole()
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
    for (backend, stubs) in [
        (
            "rust",
            [
                "todo!(\"implement tier\")",
                "todo!(\"implement items\")",
                "todo!(\"implement names\")",
                "todo!(\"implement open\")",
                "todo!(\"implement Lookup.find\")",
                "todo!(\"implement Lookup.parity\")",
            ],
        ),
        (
            "kotlin",
            [
                "TODO(\"implement tier\")",
                "TODO(\"implement items\")",
                "TODO(\"implement names\")",
                "TODO(\"implement open\")",
                "TODO(\"implement Lookup.find\")",
                "TODO(\"implement Lookup.parity\")",
            ],
        ),
    ] {
        let dir = work_dir(&format!("boundary_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), PROGRAM.replace("PARITY", "3").replace("TIER", "1")).unwrap();
        let src = generate(&dir, backend, "main");
        for stub in stubs {
            assert!(src.contains(stub), "{backend}: `{stub}` not in the skeleton:\n{src}");
        }
        let out = salvo_in(&dir, &["compile", "--backend", backend, "--target", "out"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{backend}: {stderr}");
        assert!(
            stderr.contains("`platform fn items`'s result is checked element by element"),
            "{backend}: the walk is not warned about: {stderr}"
        );
    }
}

/// [platform-stamp] A dependency checks in its host project, stamped with the
/// ABI revision and a hash of its platform signatures (ABI D9 (b)); a build
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
    fs::write(dir.join("salvo/main.sv"), "import lib\n\nfn main() [use] {\n    use StdConsole()\n    use Plain()\n    println(greet())\n}\n").unwrap();
    let (ok, stderr) = compile();
    assert!(ok, "{stderr}");
    // Reached, with no host project.
    fs::write(
        dir.join("salvo/main.sv"),
        "import lib\nimport lib.host\n\nfn main() [use] {\n    use StdConsole()\n    use HostGreet()\n    println(greet())\n}\n",
    )
    .unwrap();
    let (ok, stderr) = compile();
    assert!(!ok && stderr.contains("dependency `lib` has platform code but no generated host project"), "{stderr}");
    assert!(stderr.contains("salvo platform generate --backend rust") && stderr.contains("and check in what it writes"), "{stderr}");
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

/// [platform-value-type] A set or map the host returns is std's own host
/// type (ROADMAP §0j step 7): the skeleton has a stub for each. The hosts
/// building them with the canonical builders, and Salvo finding the
/// elements by its own `hash`/`eq`/`cmp`, are run data
/// (`tests/run/platform/host_values.sv`).
#[test]
fn sets_and_maps_from_the_host_are_built_canonically() {
    const PROGRAM: &str = "\
struct Bag {
    tags: Set<Str>
}

platform fn names() [] -> Set<Str>

platform fn sorted() [] -> SortedSet<Str>

platform fn ages() [] -> Map<Str, Int>

platform fn bag() [] -> Bag

fn main() [use] {
    use StdConsole()
    let ns = names()
    let ag = ages()
    let has = contains(ns, \"a\")
    let y = get(ag, \"y\") ?: unreachable!()
    println(\"${size(ns)} ${has} ${sorted()} ${y} ${size(bag().tags)}\")
}
";
    for (backend, stubs) in [
        ("kotlin", ["TODO(\"implement names\")", "TODO(\"implement sorted\")", "TODO(\"implement ages\")", "TODO(\"implement bag\")"]),
        ("rust", ["todo!(\"implement names\")", "todo!(\"implement sorted\")", "todo!(\"implement ages\")", "todo!(\"implement bag\")"]),
    ] {
        let dir = work_dir(&format!("shapes_{backend}"));
        project(&dir);
        fs::write(dir.join("main.sv"), PROGRAM).unwrap();
        let src = generate(&dir, backend, "main");
        for stub in stubs {
            assert!(src.contains(stub), "{backend}: `{stub}` not in:\n{src}");
        }
    }
}

/// [platform-check] D10 C2: a host cannot build a collection keyed by a
/// Salvo-defined identity — a written one (`SortedSet<Str>(by_len)`) or one
/// filled by name from a hand-written `hash`, or a `by auto` stamped one,
/// whose `hash` folds fields its own way — so a result of one is refused.
/// Only an intrinsic identity is the host's own (ROADMAP §0j step 7).
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
    assert!(errors.len() == 3, "{stderr}");
    assert!(errors.iter().any(|l| l.contains("`platform fn people`'s result is `Set<Person>`")), "{stderr}");
    assert!(
        errors.iter().any(|l| l.contains("`platform fn sorted`'s result is `SortedSet<Str>(by_len)`, a collection keyed by a Salvo-defined identity")),
        "{stderr}"
    );
    assert!(errors.iter().any(|l| l.contains("`platform fn tags`'s result is `Set<Tag>`")), "{stderr}");
}
