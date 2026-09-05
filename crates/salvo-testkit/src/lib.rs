//! Shared scaffolding for the tests that shell out to a target toolchain
//! (`kotlinc`, `rustc`, the `salvo` binary itself).
//!
//! Those tests are the slow part of the suite by two orders of magnitude —
//! a `kotlinc` invocation costs seconds where a checker test costs
//! microseconds — so this crate exists to make the *default* `cargo test`
//! both complete and quick:
//!
//! - **The toolchain is probed once** per program per test binary. Asking
//!   `kotlinc` for its version starts a JVM and costs about as much as a
//!   small compile, so a per-test probe made the availability check itself
//!   one of the most expensive things in the suite.
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
    static PROBED: OnceLock<Mutex<HashMap<String, Toolchain>>> = OnceLock::new();
    if skip_e2e() {
        return Toolchain {
            available: false,
            version: String::new(),
        };
    }
    let probed = PROBED.get_or_init(|| Mutex::new(HashMap::new()));
    let mut probed = probed.lock().expect("toolchain probe cache poisoned");
    if let Some(found) = probed.get(program) {
        return found.clone();
    }
    let found = match Command::new(program).arg(version_arg).output() {
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
    };
    probed.insert(program.to_string(), found.clone());
    found
}

/// `kotlinc`, probed once.
pub fn kotlinc() -> Toolchain {
    toolchain("kotlinc", "-version")
}

/// `rustc`, probed once.
pub fn rustc() -> Toolchain {
    toolchain("rustc", "--version")
}

/// A toolchain by name, with the flag it actually understands — `rustc`
/// rejects `-version`, and a probe that asks for it "succeeds" with an error
/// message, which would then sit in the cache key *pretending* to be a
/// version and never change when rustc did.
pub fn tool(program: &str) -> Toolchain {
    match program {
        "rustc" => rustc(),
        "kotlinc" => kotlinc(),
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
