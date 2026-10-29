//! [test-cli-stamp] Stamps for the tests of the `salvo` binary itself
//! (TESTS.md C1): what such a test verifies is a function of the binary, so
//! its plain key is the binary's fingerprint — which every compiler rebuild
//! changes, re-running every CLI test after a change that left their output
//! alone. [`cli_stamp`] keys a test twice:
//!
//! - **On the binary** (plus the test file and the toolchains): a re-run with
//!   nothing rebuilt hits at once, as before.
//! - **On what the binary produces**: the code `salvo compile` emits for the
//!   test's projects on each backend it runs, every input file of those
//!   projects, the sources of what the binary does *after* emitting (the CLI
//!   driver, the backends' build and run steps, the runtime files, the
//!   manifest), the test file and the toolchains. A rebuild that changes none
//!   of that — a checker change that emits the same code — hits too, and
//!   writes the binary key so the next re-run is instant.
//!
//! A test is stamped by content only when its projects are written before
//! the stamp is asked for and not rewritten afterwards: the key is what
//! those projects compile to. `SALVO_E2E_FRESH=1` ignores both keys.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// A CLI test, for its stamp.
pub struct CliTest<'a> {
    /// `env!("CARGO_BIN_EXE_salvo")`.
    pub salvo: &'a str,
    /// `env!("CARGO_TARGET_TMPDIR")`.
    pub tmpdir: &'a str,
    /// The test's name, unique in its file.
    pub name: &'a str,
    /// The test file's text (`include_str!` of it): editing the file
    /// re-runs its tests.
    pub source: &'a str,
    /// The toolchains the test runs (`rustc`, `kotlinc`, `cargo`, …); `rustc`
    /// and `kotlinc` also name the backends its projects are compiled for.
    pub tools: &'a [&'a str],
}

/// One project a test runs: its directory (the working directory of the
/// compile) and the `salvo compile` arguments that emit what the test's
/// commands build (`--src .`, `--main app/main.sv`, …), `--backend` and
/// `--target` aside.
pub struct CliProject<'a> {
    pub dir: &'a Path,
    pub args: &'a [&'a str],
    /// The backends this project is built for; empty: every backend the
    /// test's tools name.
    pub backends: &'a [&'a str],
}

/// Both keys of one CLI test; [`CliStamp::verified`] writes both.
#[must_use = "a verification that is never marked `verified` will be redone"]
pub struct CliStamp {
    binary: crate::Stamp,
    content: Option<crate::Stamp>,
}

impl CliStamp {
    pub fn verified(self) {
        self.binary.verified();
        if let Some(c) = self.content {
            c.verified();
        }
    }
}

/// The digest of every file under the directories of the workspace that
/// decide what the binary does once code is emitted.
fn driver_print() -> &'static str {
    static PRINT: OnceLock<String> = OnceLock::new();
    PRINT.get_or_init(|| {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut files: Vec<PathBuf> = Vec::new();
        for dir in [
            "salvo-cli/src",
            "salvo-backend/src",
            "salvo-backend-rust/src",
            "salvo-backend-rust/runtime",
            "salvo-backend-kotlin/src",
            "salvo-backend-kotlin/runtime",
            "salvo-test/src",
        ] {
            files.extend(walk(&crates.join(dir)));
        }
        files.push(crates.join("salvo-core/src/manifest.rs"));
        files.sort();
        let mut parts: Vec<Vec<u8>> = Vec::new();
        for f in &files {
            parts.push(f.to_string_lossy().as_bytes().to_vec());
            parts.push(std::fs::read(f).unwrap_or_default());
        }
        let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
        crate::fingerprint(&refs)
    })
}

/// Every file under `dir`, recursively, skipping hidden directories.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Asks whether `test` has already passed, first by the binary, then by
/// what the binary emits for `projects`. `None`: skip the test's work.
pub fn cli_stamp(test: &CliTest, projects: &[CliProject]) -> Option<CliStamp> {
    let versions: Vec<String> = test.tools.iter().map(|t| crate::tool(test.tmpdir, t).version).collect();
    let source = crate::fingerprint(&[test.source.as_bytes()]);
    let mut parts: Vec<Vec<u8>> = vec![
        b"cli".to_vec(),
        test.name.as_bytes().to_vec(),
        crate::file_fingerprint(test.salvo).into_bytes(),
        source.clone().into_bytes(),
    ];
    parts.extend(versions.iter().map(|v| v.clone().into_bytes()));
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    let binary = crate::cached(test.tmpdir, &format!("cli {}", test.name), &refs)?;
    let backends: Vec<&str> = test
        .tools
        .iter()
        .filter_map(|t| match *t {
            "rustc" => Some("rust"),
            "kotlinc" => Some("kotlin"),
            _ => None,
        })
        .collect();
    // `SALVO_E2E_FRESH` runs the test whatever the content key says, so the
    // `salvo compile` per project and backend that computes it would be work
    // for nothing (a CLI test's run was one more build); the content stamp
    // is written by the next run that is not fresh.
    if projects.is_empty() || backends.is_empty() || crate::fresh() {
        return Some(CliStamp { binary, content: None });
    }
    let mut parts: Vec<Vec<u8>> = vec![
        b"cli-content".to_vec(),
        test.name.as_bytes().to_vec(),
        source.into_bytes(),
        driver_print().as_bytes().to_vec(),
    ];
    parts.extend(versions.iter().map(|v| v.clone().into_bytes()));
    for (i, project) in projects.iter().enumerate() {
        for f in walk(project.dir) {
            parts.push(f.strip_prefix(project.dir).unwrap_or(&f).to_string_lossy().as_bytes().to_vec());
            parts.push(std::fs::read(&f).unwrap_or_default());
        }
        for backend in backends.iter().filter(|b| project.backends.is_empty() || project.backends.contains(b)) {
            let target = Path::new(test.tmpdir).join("cli-key").join(format!("{}-{i}-{backend}", test.name.replace(' ', "_")));
            let _ = std::fs::remove_dir_all(&target);
            let out = Command::new(test.salvo)
                .env("SALVO_CACHE_DIR", Path::new(test.tmpdir).join("salvo-cache"))
                .current_dir(project.dir)
                .arg("compile")
                .args(["--backend", backend])
                .args(project.args)
                .arg("--target")
                .arg(&target)
                .output()
                .expect("failed to run salvo compile");
            parts.push(format!("{backend} {:?}", out.status.code()).into_bytes());
            // The emitted files, not the log: it names the scratch target.
            if !out.status.success() {
                parts.push(out.stderr);
            }
            for f in walk(&target) {
                parts.push(f.strip_prefix(&target).unwrap_or(&f).to_string_lossy().as_bytes().to_vec());
                parts.push(std::fs::read(&f).unwrap_or_default());
            }
            let _ = std::fs::remove_dir_all(&target);
        }
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    match crate::cached(test.tmpdir, &format!("cli-content {}", test.name), &refs) {
        None => {
            // Produced the same as when it last passed: remember that for
            // this binary too.
            binary.verified();
            None
        }
        content => Some(CliStamp { binary, content }),
    }
}
