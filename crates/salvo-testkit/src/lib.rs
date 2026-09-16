//! Shared scaffolding for the tests that shell out to a target toolchain
//! (`kotlinc`, `rustc`, the `salvo` binary itself).
//!
//! Those tests are the slow part of the suite by two orders of magnitude —
//! a `kotlinc` invocation costs seconds where a checker test costs
//! microseconds — so this crate exists to make the *default* `cargo test`
//! both complete and quick:
//!
//! - **The toolchain is probed once** per program per test binary — and for
//!   `kotlinc`, once per *install*: asking `kotlinc` for its version starts
//!   a JVM and costs about as much as a small compile, so its probe result
//!   is remembered on disk next to the stamps (keyed by the resolved
//!   executable, so upgrading kotlinc re-probes). A per-actor cache alone
//!   made the availability check itself one of the most expensive things in
//!   the suite — and under nextest, which runs every test in its own
//!   process, it made every cached kotlin test pay for a JVM start.
//! - **A verification is remembered by content.** Compiling and running
//!   generated code is a pure function of the code, the expected output and
//!   the toolchain version, so a run that passed leaves a stamp keyed by the
//!   hash of exactly those inputs; the next run with identical inputs skips
//!   the toolchain work. Change the emitter and every affected stamp misses,
//!   which is the point: the cache cannot hide a regression it caused.
//!
//! Two environment variables control it, and both are documented in
//! AGENTS.md:
//!
//! - `SALVO_SKIP_E2E=1` — do not run the toolchain tests at all. The fast
//!   inner loop; never a pre-submit check.
//! - `SALVO_E2E_FRESH=1` — ignore the stamps, so every toolchain test really
//!   compiles and runs. This is what "full" means: the whole suite, nothing
//!   taken on trust. The stamps are still *written*, so the run leaves the
//!   cache warm and correct.
//!
//! `SALVO_E2E_FRESH` deliberately ignores stamps rather than deleting them.
//! Cargo runs test binaries one after another, so a binary that wiped the
//! shared directory would throw away the stamps an earlier binary had just
//! written. To reset the cache outright, delete the directory
//! (`target/tmp/salvo-e2e-cache`).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Bumped when the meaning of a stamp changes — a runner that starts
/// asserting something new must not accept stamps written before it did.
const HARNESS_VERSION: &str = "1";

/// The directory holding the stamps, under the crate's own
/// `CARGO_TARGET_TMPDIR` so nothing lands outside the repository.
const CACHE_DIR: &str = "salvo-e2e-cache";

/// Whether the toolchain tests should be skipped entirely
/// (`SALVO_SKIP_E2E`).
pub fn skip_e2e() -> bool {
    std::env::var_os("SALVO_SKIP_E2E").is_some()
}

/// Whether cached verifications should be ignored (`SALVO_E2E_FRESH`).
pub fn fresh() -> bool {
    std::env::var_os("SALVO_E2E_FRESH").is_some()
}

/// A target toolchain: whether it is usable, and what it calls itself.
///
/// The version string is part of every cache key, so upgrading `kotlinc` or
/// `rustc` re-verifies everything rather than trusting stamps written by the
/// previous one.
#[derive(Clone, Debug)]
pub struct Toolchain {
    pub available: bool,
    pub version: String,
}

/// Probes `program version_arg` **once** per program, for the lifetime of
/// the test binary. Reports unavailability once, too, so a machine without
/// the toolchain gets one line rather than one per test.
pub fn toolchain(program: &str, version_arg: &str) -> Toolchain {
    if skip_e2e() {
        return Toolchain {
            available: false,
            version: String::new(),
        };
    }
    if let Some(found) = probed_this_process(program) {
        return found;
    }
    let found = run_probe(program, version_arg);
    remember_probe(program, &found);
    found
}

/// Like [`toolchain`], but remembers a successful probe *on disk* (in the
/// same directory as the stamps), so it survives across processes. This is
/// for probes that are expensive to run — `kotlinc -version` starts a JVM
/// and costs about as much as a small compile, and a per-actor cache
/// still pays it once per test *binary* under `cargo test` and once per
/// *test* under nextest, where every test is its own process.
///
/// The cache key is the resolved executable: its canonical path, size and
/// mtime. Upgrading the toolchain replaces that file (or points the PATH
/// entry somewhere else), so a stale version string cannot outlive the
/// binary it described — which matters, because the version goes into
/// every verification stamp. Unavailability is never written to disk:
/// installing the toolchain must be noticed by the next run.
///
/// `SALVO_E2E_FRESH=1` bypasses (but still refreshes) this cache, keeping
/// its "nothing taken on trust" meaning.
pub fn toolchain_disk_cached(target_tmpdir: &str, program: &str, version_arg: &str) -> Toolchain {
    if skip_e2e() {
        return Toolchain {
            available: false,
            version: String::new(),
        };
    }
    if let Some(found) = probed_this_process(program) {
        return found;
    }
    let resolved = resolve_on_path(program);
    let cache_path = resolved.as_ref().map(|exe| {
        let print = file_fingerprint(&exe.to_string_lossy());
        let key = digest(&[
            b"probe",
            program.as_bytes(),
            exe.to_string_lossy().as_bytes(),
            print.as_bytes(),
        ]);
        Path::new(target_tmpdir)
            .join(CACHE_DIR)
            .join(format!("probe-{key}.txt"))
    });
    if !fresh() {
        if let Some(path) = &cache_path {
            if let Ok(version) = std::fs::read_to_string(path) {
                let version = version.trim().to_string();
                if !version.is_empty() {
                    let found = Toolchain {
                        available: true,
                        version,
                    };
                    remember_probe(program, &found);
                    return found;
                }
            }
        }
    }
    let found = run_probe(program, version_arg);
    if found.available && !found.version.is_empty() {
        if let Some(path) = &cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, &found.version);
        }
    }
    remember_probe(program, &found);
    found
}

/// The per-actor probe cache, shared by both probe flavors.
fn probe_cache() -> &'static Mutex<HashMap<String, Toolchain>> {
    static PROBED: OnceLock<Mutex<HashMap<String, Toolchain>>> = OnceLock::new();
    PROBED.get_or_init(|| Mutex::new(HashMap::new()))
}

fn probed_this_process(program: &str) -> Option<Toolchain> {
    let probed = probe_cache()
        .lock()
        .expect("toolchain probe cache poisoned");
    probed.get(program).cloned()
}

fn remember_probe(program: &str, found: &Toolchain) {
    let mut probed = probe_cache()
        .lock()
        .expect("toolchain probe cache poisoned");
    probed.insert(program.to_string(), found.clone());
}

/// Actually runs `program version_arg` and reads the version off whichever
/// stream it lands on.
fn run_probe(program: &str, version_arg: &str) -> Toolchain {
    match Command::new(program).arg(version_arg).output() {
        Ok(out) => {
            let mut version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if version.is_empty() {
                // `kotlinc -version` writes to stderr.
                version = String::from_utf8_lossy(&out.stderr).trim().to_string();
            }
            Toolchain {
                available: true,
                version,
            }
        }
        Err(_) => {
            eprintln!("skipping {program} tests: {program} not found on PATH");
            Toolchain {
                available: false,
                version: String::new(),
            }
        }
    }
}

/// The executable `program` resolves to on PATH, canonicalized — the file
/// whose identity keys the disk-cached probe.
fn resolve_on_path(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

/// `kotlinc`, probed once per machine per install (the probe starts a JVM,
/// so it is remembered on disk, not just per process). Pass
/// `env!("CARGO_TARGET_TMPDIR")`.
pub fn kotlinc(target_tmpdir: &str) -> Toolchain {
    toolchain_disk_cached(target_tmpdir, "kotlinc", "-version")
}

/// `rustc`, probed once per process. Deliberately *not* disk-cached: the
/// probe costs ~50ms, and `rustc` on PATH is usually rustup's shim — a file
/// that does not change when `rustup update` changes what it dispatches to,
/// so a disk cache keyed on it could serve a stale version.
pub fn rustc() -> Toolchain {
    toolchain("rustc", "--version")
}

/// A toolchain by name, with the flag it actually understands — `rustc`
/// rejects `-version`, and a probe that asks for it "succeeds" with an error
/// message, which would then sit in the cache key *pretending* to be a
/// version and never change when rustc did. Pass
/// `env!("CARGO_TARGET_TMPDIR")`; it is used for the probes worth
/// remembering on disk.
pub fn tool(target_tmpdir: &str, program: &str) -> Toolchain {
    match program {
        "rustc" => rustc(),
        "kotlinc" => kotlinc(target_tmpdir),
        other => toolchain(other, "--version"),
    }
}

/// A scratch directory for one test, emptied first, under the calling test
/// binary's `CARGO_TARGET_TMPDIR` (pass `env!("CARGO_TARGET_TMPDIR")`).
/// Temporary files belong inside the repository, never in `/tmp`.
pub fn scratch(target_tmpdir: &str, tag: &str) -> PathBuf {
    let dir = Path::new(target_tmpdir).join(format!("e2e-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("failed to create the scratch directory");
    dir
}

/// A verification that has not been done yet. Call [`Stamp::verified`] once
/// the assertions have passed — and only then, so a failure is never
/// remembered as a success.
#[must_use = "a verification that is never marked `verified` will be redone"]
pub struct Stamp {
    path: PathBuf,
    what: String,
}

impl Stamp {
    /// Records that this exact input passed, so an identical later run can
    /// skip the toolchain work.
    pub fn verified(self) {
        let note = format!(
            "{}\nverified by salvo-testkit (harness {HARNESS_VERSION})\n",
            self.what
        );
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.path, note);
    }
}

/// A fingerprint of a *binary* — for keying a cache on the executable under
/// test rather than on generated text. The CLI tests need it: the thing
/// under test there is the `salvo` executable, so what it is decides the
/// outcome.
///
/// Metadata (length and modification time), not content: cargo rewrites a
/// binary whenever it rebuilds it, and hashing tens of megabytes on every
/// test run cost more than the tests it was meant to save (measured: +12s
/// across the CLI suite, since a debug build hashes byte by byte). A rebuild
/// that produced identical bytes merely misses the cache, which is safe; the
/// converse — different bytes with identical length *and* timestamp — is not
/// something cargo can produce.
pub fn file_fingerprint(path: &str) -> String {
    static PRINTS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let prints = PRINTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut prints = prints.lock().expect("fingerprint cache poisoned");
    if let Some(found) = prints.get(path) {
        return found.clone();
    }
    let print = match std::fs::metadata(path) {
        Ok(meta) => {
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            format!("{}:{modified}", meta.len())
        }
        // Unreadable is a distinct key from any file, so nothing is cached
        // under a false assumption.
        Err(e) => format!("unreadable:{e}"),
    };
    prints.insert(path.to_string(), print.clone());
    print
}

/// The fingerprint of the running test binary, which is where a test's own
/// inputs and assertions live: editing a test invalidates its stamps.
pub fn self_fingerprint() -> String {
    static PRINT: OnceLock<String> = OnceLock::new();
    PRINT
        .get_or_init(|| match std::env::current_exe() {
            Ok(path) => file_fingerprint(&path.to_string_lossy()),
            Err(e) => format!("unknown-exe:{e}"),
        })
        .clone()
}

/// The 128-bit digest of a sequence of byte strings: SipHash and FNV-1a side
/// by side, so a collision has to happen in both at once. Length-prefixed,
/// so concatenation cannot be confused with separation.
fn digest(parts: &[&[u8]]) -> String {
    let mut sip = std::collections::hash_map::DefaultHasher::new();
    HARNESS_VERSION.hash(&mut sip);
    let mut fnv: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        part.len().hash(&mut sip);
        sip.write(part);
        for byte in *part {
            fnv ^= u64::from(*byte);
            fnv = fnv.wrapping_mul(0x100_0000_01b3);
        }
        fnv = fnv.wrapping_add(part.len() as u64);
    }
    format!("{:016x}{:016x}", sip.finish(), fnv)
}

/// Asks whether this exact verification has already passed.
///
/// `what` is a human-readable label for the stamp file; `parts` are every
/// input the outcome depends on — the generated sources, the expected
/// output, the toolchain version. Returns `None` when a stamp already
/// covers them (the caller should skip its work) and `Some(stamp)`
/// otherwise.
pub fn cached(target_tmpdir: &str, what: &str, parts: &[&[u8]]) -> Option<Stamp> {
    let path = Path::new(target_tmpdir)
        .join(CACHE_DIR)
        .join(format!("{}.ok", digest(parts)));
    if !fresh() && path.exists() {
        return None;
    }
    Some(Stamp {
        path,
        what: what.to_string(),
    })
}

/// Deletes orphaned debug-info objects from a `target/debug/deps`-style
/// directory, returning how many were removed.
///
/// On macOS, cargo's default `split-debuginfo=unpacked` keeps every
/// `*.rcgu.o` codegen object next to the binary that references it (the
/// binary holds OSO pointers into them; debuggers follow the pointers).
/// Cargo never garbage-collects them, and every rebuild writes a fresh set
/// under a new hash — measured here: 790k files / dozens of GiB after a few
/// weeks, enough to make anything that lists the directory crawl.
///
/// An object is an orphan when the artifact it belongs to is gone: the
/// leading `name-hash` segment of `X.<cgu>.rcgu.o` names the linked
/// artifact, so the object is kept iff `X`, `libX.rlib` or `libX.dylib`
/// still exists. Objects of *current* binaries are never touched, so
/// debugging them keeps working.
pub fn prune_stale_debug_objects(deps_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(deps_dir) else {
        return 0;
    };
    let mut names: Vec<String> = Vec::new();
    let mut live: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in entries.flatten() {
        if let Ok(name) = entry.file_name().into_string() {
            if name.ends_with(".rcgu.o") {
                names.push(name);
            } else if !name.contains('.') || name.ends_with(".rlib") || name.ends_with(".dylib") {
                live.insert(name);
            }
        }
    }
    let mut removed = 0;
    for name in names {
        let Some(owner) = name.split('.').next() else {
            continue;
        };
        let lib = format!("lib{owner}.rlib");
        let dylib = format!("lib{owner}.dylib");
        if live.contains(owner) || live.contains(&lib) || live.contains(&dylib) {
            continue;
        }
        if std::fs::remove_file(deps_dir.join(&name)).is_ok() {
            removed += 1;
        }
    }
    removed
}
