//! Integration tests for source discovery (`SourceSet::add_dir`): the
//! `.svignore` file, hidden-directory and cache-directory skipping
//! [mod-ignore].

use std::fs;
use std::path::PathBuf;

use salvo_core::SourceSet;

fn dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("source_{test}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(sources: &SourceSet) -> Vec<&str> {
    sources.files.iter().map(|f| f.name.as_str()).collect()
}

// [mod-ignore] `.svignore` entries skip files and whole directory subtrees,
// relative to the root; comments and blank lines are ignored.
#[test]
fn svignore_skips_listed_files_and_directories() {
    let root = dir("svignore");
    fs::write(root.join("main.sv"), "fn main() {}\n").unwrap();
    fs::write(root.join("scratch.sv"), "fn broken( {\n").unwrap();
    fs::create_dir_all(root.join("experiments/deep")).unwrap();
    fs::write(root.join("experiments/idea.sv"), "fn broken( {\n").unwrap();
    fs::write(root.join("experiments/deep/x.sv"), "fn broken( {\n").unwrap();
    fs::write(
        root.join(".svignore"),
        "# scratch space\n\nscratch.sv\nexperiments/\n",
    )
    .unwrap();

    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&root, "", false);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(names(&sources), ["main.sv"]);
}

// [mod-ignore] Hidden directories and directories carrying a CACHEDIR.TAG
// marker (e.g. Cargo's `target/`) are skipped; the root itself is exempt.
#[test]
fn hidden_and_cache_directories_are_skipped() {
    let root = dir("hidden_cache");
    fs::write(root.join("main.sv"), "fn main() {}\n").unwrap();
    fs::create_dir_all(root.join(".hidden")).unwrap();
    fs::write(root.join(".hidden/h.sv"), "fn broken( {\n").unwrap();
    fs::create_dir_all(root.join("build_out/tmp")).unwrap();
    fs::write(root.join("build_out/CACHEDIR.TAG"), "Signature: 8a477f597d28d172789f06886806bc55\n").unwrap();
    fs::write(root.join("build_out/tmp/stale.sv"), "fn broken( {\n").unwrap();

    let mut sources = SourceSet::default();
    sources.add_dir(&root, "", false);
    assert_eq!(names(&sources), ["main.sv"]);

    // Pointing the root *at* the tagged directory still loads its files.
    let mut inner = SourceSet::default();
    inner.add_dir(&root.join("build_out"), "", false);
    assert_eq!(names(&inner), ["tmp/stale.sv"]);
}
