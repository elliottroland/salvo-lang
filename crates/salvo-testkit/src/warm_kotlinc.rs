//! [test-warm-kotlinc] The warm Kotlin compiler (user decision 2026-10-29).
//!
//! A cold `kotlinc` is a JVM start and an un-JIT-ted compiler on every call:
//! 2.6s for an empty file, 7s for a hello-world program, and the suite made
//! 130 calls, 2,496s summed — most of all the time the tests took. This puts
//! a `kotlinc` of its own first on the tests' PATH: a small client
//! (`kotlinc/client.rs`) that hands each compile to a long-lived compiler
//! JVM (`kotlinc/KotlincServer.kt`), started on first use and shared by every
//! test process, the `salvo` subprocesses included, since they inherit the
//! PATH. The same hello-world compiles in about a second there.
//!
//! It is not a cache: every compile is done, by the same compiler the real
//! `kotlinc` runs, on the same JVM. What a cold start would decide differently
//! — nothing it compiles is reused between requests — it cannot. The
//! differences are in how messages are printed (full paths, where the real
//! one prints them relative to its working directory) and in the JVM's heap,
//! which the server sets once. Anything the client does not understand runs
//! the real `kotlinc`, as does `SALVO_WARM_KOTLINC=0`.
//!
//! Installed per content key under `target/tmp/kotlinc-warm/<key>/`: the
//! client (built with `rustc`), the server's classes (built with the real
//! `kotlinc`), and what the client reads to start a server. The servers'
//! sockets and locks live in its `run/`, a 0700 directory: Unix-domain
//! sockets only, nothing listens on the network.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const CLIENT: &str = include_str!("../kotlinc/client.rs");
const SERVER: &str = include_str!("../kotlinc/KotlincServer.kt");

/// Puts the warm `kotlinc` first on this process's PATH (once), when it can
/// be installed; otherwise leaves the PATH alone, and the real `kotlinc` is
/// what runs. Called by [`crate::kotlinc`] after the real one is found.
pub(crate) fn activate(target_tmpdir: &str) {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        if std::env::var_os("SALVO_WARM_KOTLINC").is_some_and(|v| v == "0") {
            return;
        }
        match install(target_tmpdir) {
            Ok(bin) => {
                let path = std::env::var_os("PATH").unwrap_or_default();
                let mut dirs = vec![bin];
                dirs.extend(std::env::split_paths(&path));
                if let Ok(joined) = std::env::join_paths(dirs) {
                    // Rust's own environment accesses (and `Command`'s
                    // capture of it) are serialized by std; nothing here
                    // reads the environment through libc.
                    std::env::set_var("PATH", joined);
                }
            }
            Err(why) => eprintln!("the warm kotlinc is not available, using the real one: {why}"),
        }
    });
}

/// The `bin/` directory holding the warm `kotlinc`, installed if it is not.
fn install(target_tmpdir: &str) -> Result<PathBuf, String> {
    let real = find_on_path("kotlinc").ok_or("no kotlinc on PATH")?;
    let real = real.canonicalize().unwrap_or(real);
    // <home>/bin/kotlinc, with the compiler in <home>/lib — or, for a
    // Homebrew-style wrapper, in <install>/libexec/lib.
    let install_dir = real.parent().and_then(Path::parent).ok_or("kotlinc has no install directory")?;
    let lib = ["libexec/lib", "lib"]
        .iter()
        .map(|l| install_dir.join(l))
        .find(|l| l.join("kotlin-compiler.jar").is_file())
        .ok_or_else(|| format!("no kotlin-compiler.jar under {}", install_dir.display()))?;
    let compiler = lib.join("kotlin-compiler.jar");
    let stamp = std::fs::metadata(&compiler)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = crate::digest(&[
        b"warm-kotlinc",
        CLIENT.as_bytes(),
        SERVER.as_bytes(),
        real.to_string_lossy().as_bytes(),
        compiler.to_string_lossy().as_bytes(),
        stamp.to_string().as_bytes(),
        slots().to_string().as_bytes(),
    ]);
    let base = Path::new(target_tmpdir).join("kotlinc-warm");
    let root = base.join(&key);
    // [test-warm-kotlinc] The socket path must fit `sun_path` (104 bytes on
    // macOS).
    if root.join("run/slot-00.sock").as_os_str().len() > 100 {
        return Err(format!("{} is too long a path for a socket", root.display()));
    }
    if root.join("done").is_file() {
        return Ok(root.join("bin"));
    }
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let staging = base.join(format!(".building-{}-{key}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    let built = build(&staging, &real, &lib, &compiler, &root);
    if let Err(e) = built {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    if std::fs::rename(&staging, &root).is_err() {
        // Another process installed it first; theirs is the same.
        let _ = std::fs::remove_dir_all(&staging);
    }
    if root.join("done").is_file() {
        Ok(root.join("bin"))
    } else {
        Err(format!("could not install the warm kotlinc at {}", root.display()))
    }
}

/// Builds an installation into `dir`, which will be renamed to `root`.
fn build(dir: &Path, real: &Path, lib: &Path, compiler: &Path, root: &Path) -> Result<(), String> {
    let run = |cmd: &mut Command, what: &str| -> Result<std::process::Output, String> {
        let out = cmd.output().map_err(|e| format!("{what}: {e}"))?;
        if out.status.success() {
            Ok(out)
        } else {
            Err(format!("{what} failed:\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
        }
    };
    let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).map_err(|e| format!("{name}: {e}"));
    std::fs::create_dir_all(dir.join("bin")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    write("src/client.rs", CLIENT)?;
    write("src/KotlincServer.kt", SERVER)?;
    run(
        Command::new("rustc").args(["-O", "--edition", "2021", "-o"]).arg(dir.join("bin/kotlinc")).arg(dir.join("src/client.rs")),
        "building the warm kotlinc's client",
    )?;
    run(
        Command::new(real)
            .arg(dir.join("src/KotlincServer.kt"))
            .arg("-nowarn")
            .arg("-cp")
            .arg(compiler)
            .arg("-d")
            .arg(dir.join("server")),
        "building the warm kotlinc's server",
    )?;
    // The JVM the real kotlinc runs on: its `kotlin` runner, beside it, has
    // the same launcher and so the same JVM.
    let classpath = format!("{}:{}", root.join("server").display(), compiler.display());
    let staged_cp = format!("{}:{}", dir.join("server").display(), compiler.display());
    let runner = real.with_file_name("kotlin");
    let out = run(
        Command::new(&runner).args(["-cp", &staged_cp, "salvo.testkit.KotlincServerKt", "--java"]),
        "asking the real kotlinc's JVM where it lives",
    )?;
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    let mut lines = said.lines();
    let java_home = lines.next().ok_or("no java.home")?.trim().to_string();
    let spec: u32 = lines.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    let java = Path::new(&java_home).join("bin/java");
    if !java.is_file() {
        return Err(format!("no java at {}", java.display()));
    }
    // What the real launcher passes, and what a compiler kept alive wants.
    let mut jvm_args = vec!["-Xmx3g".to_string(), "-Dkotlin.environment.keepalive=true".to_string()];
    if spec >= 24 {
        jvm_args.push("--enable-native-access=ALL-UNNAMED".to_string());
        jvm_args.push("--sun-misc-unsafe-memory-access=allow".to_string());
    }
    let slots = slots();
    write("real", &real.to_string_lossy())?;
    write("java", &java.to_string_lossy())?;
    write("jvm-args", &jvm_args.join("\n"))?;
    write("classpath", &classpath)?;
    write("slots", &slots.to_string())?;
    // `<install>/lib` beside `bin/kotlinc`: what a build looks for next to the
    // kotlinc on PATH (`kotlin-stdlib.jar`, [kt-build]).
    std::os::unix::fs::symlink(lib, dir.join("lib")).map_err(|e| format!("lib: {e}"))?;
    std::fs::create_dir_all(dir.join("run")).map_err(|e| e.to_string())?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.join("run"), std::fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    write("done", "")?;
    Ok(())
}

/// How many servers may run: one compile each at a time, each with a heap
/// of up to 3 GiB.
fn slots() -> usize {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Half the cores (measured 2026-10-29 on 10 cores: 5 servers, a fresh
    // suite in 178s; 8 servers, 247s — the JVMs' own threads and heaps
    // crowd out the tests).
    (cores / 2).clamp(2, 6)
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(program)).find(|c| c.is_file())
}
