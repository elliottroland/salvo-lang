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

/// The rule labels the specs define: LANGUAGE_SPEC.md and the backend specs,
/// one `* [label]` (or `- [label]`) opening a rule.
fn defined_labels() -> std::collections::HashSet<String> {
    let root = repo_root();
    let mut out = std::collections::HashSet::new();
    for spec in ["LANGUAGE_SPEC.md", "BACKEND_SPEC.kotlin.md", "BACKEND_SPEC.rust.md"] {
        let text = std::fs::read_to_string(root.join(spec)).expect(spec);
        for line in text.lines() {
            let t = line.trim_start();
            let Some(rest) = t.strip_prefix("* [").or_else(|| t.strip_prefix("- [")) else { continue };
            let Some(end) = rest.find(']') else { continue };
            let label = &rest[..end];
            if !label.is_empty() && label.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                out.insert(label.to_string());
            }
        }
    }
    out
}

fn std_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("lib/").filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            std_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "sv") {
            out.push(path);
        }
    }
}

/// [doc-compilerrules] std's documentation cites compiler rules only in an
/// `@compilerrules` tag, which hover leaves out, and every rule it cites
/// exists. A documentation block is the run of comment lines directly above
/// a declaration [doc-comment]; a cited rule is a bracketed label the specs
/// define (`[symbol]` references are not labels, so they are left alone).
#[test]
fn std_docs_cite_rules_only_in_compilerrules() {
    let labels = defined_labels();
    assert!(labels.len() > 500, "found only {} rule labels", labels.len());
    let declaration = |line: &str| {
        let t = line.trim_start();
        let word: String = t.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        const STARTS: &[&str] = &[
            "export", "fn", "struct", "effect", "handler", "qualifier", "type", "platform", "intrinsic", "linear", "iter",
            "comptime", "send", "actor", "fns", "namespace", "rename", "opaque", "threadsafe", "state", "test", "init",
            "monitor", "mixed", "alias", "any",
        ];
        !word.is_empty()
            && (STARTS.contains(&word.as_str())
                || (word.starts_with(|c: char| c.is_ascii_lowercase()) && t[word.len()..].trim_start().starts_with(':')))
    };
    let mut files = Vec::new();
    std_sources(&repo_root().join("lib"), &mut files);
    files.sort();
    let mut inline = Vec::new();
    let mut unknown = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if !lines[i].trim_start().starts_with("//") {
                i += 1;
                continue;
            }
            let start = i;
            while i < lines.len() && lines[i].trim_start().starts_with("//") {
                i += 1;
            }
            let documents = i < lines.len() && declaration(lines[i]);
            for (n, line) in lines[start..i].iter().enumerate() {
                let comment = line.trim_start().trim_start_matches('/').trim_start();
                let tag = comment.starts_with("@compilerrules");
                let mut rest = comment;
                while let Some(open) = rest.find('[') {
                    let Some(close) = rest[open..].find(']') else { break };
                    for label in rest[open + 1..open + close].split(',').map(str::trim) {
                        let at = format!("{}:{}", file.strip_prefix(repo_root()).unwrap().display(), start + n + 1);
                        if tag && !labels.contains(label) {
                            unknown.push(format!("{at}: `{label}`"));
                        } else if !tag && documents && labels.contains(label) {
                            inline.push(format!("{at}: [{label}]"));
                        }
                    }
                    rest = &rest[open + close + 1..];
                }
            }
        }
    }
    assert!(inline.is_empty(), "std documentation cites rules in its prose, where hover shows them; move them to `@compilerrules`:\n{}", inline.join("\n"));
    assert!(unknown.is_empty(), "`@compilerrules` cites rules no spec defines:\n{}", unknown.join("\n"));
}
