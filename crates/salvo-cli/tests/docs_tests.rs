//! Hygiene for the language documentation.
//!
//! `docs/language/` is the narrative specification and the source of truth for
//! behaviour; `wiki/` is a **generated** copy of it for reading on GitHub
//! (`tools/sync-wiki.sh`). Both facts are only true if something checks them,
//! which is what these tests do — the same reasoning that makes
//! `every_examples_checked_in_*_is_current` a test rather than a habit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is crates/salvo-cli.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

/// Every page in `docs/language/` is listed in the index, and every page the
/// index lists exists. Without this a new page is invisible — the index is
/// what gives the pages a reading order, and what the wiki sidebar is built
/// from.
#[test]
fn every_language_page_is_indexed() {
    let dir = repo_root().join("docs/language");
    let index = std::fs::read_to_string(dir.join("README.md")).expect("docs/language/README.md");

    let mut pages: Vec<String> = std::fs::read_dir(&dir)
        .expect("docs/language")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .collect();
    pages.sort();

    for page in &pages {
        assert!(
            index.contains(&format!("]({page})")),
            "docs/language/{page} is not linked from README.md — add it to the \
             reading order, or the wiki sidebar will not carry it"
        );
    }
    // …and nothing is listed that does not exist.
    for line in index.lines() {
        let Some(start) = line.find("](") else { continue };
        let rest = &line[start + 2..];
        let Some(end) = rest.find(')') else { continue };
        let target = &rest[..end];
        if !target.ends_with(".md") || target.contains('/') {
            continue;
        }
        assert!(
            dir.join(target).exists(),
            "docs/language/README.md links {target}, which does not exist"
        );
    }
}

/// When a `wiki/` checkout is present it must be **current**: it is generated,
/// so a stale page is a page that lies about the language. Skips when the
/// checkout is absent — the wiki is a separate repository and a clone of this
/// one need not have it (the toolchain tests skip the same way).
#[test]
fn the_wiki_is_current_when_present() {
    let root = repo_root();
    if !root.join("wiki").is_dir() {
        eprintln!("skipping: no wiki/ checkout");
        return;
    }
    let out = Command::new("bash")
        .arg(root.join("tools/sync-wiki.sh"))
        .arg("--check")
        .current_dir(&root)
        .output()
        .expect("run tools/sync-wiki.sh");
    assert!(
        out.status.success(),
        "wiki/ is out of date — run tools/sync-wiki.sh\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
