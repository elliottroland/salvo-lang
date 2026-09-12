//! Target-directory hygiene, run as a test so every full `cargo test` /
//! `cargo nextest run` does it without any extra tooling.

use std::path::PathBuf;

/// Deletes orphaned `*.rcgu.o` debug objects from `target/debug/deps`.
///
/// macOS keeps these files as the debug info of every linked binary
/// (`split-debuginfo=unpacked`), and cargo never garbage-collects the sets
/// left behind by earlier builds — they once piled up to 790k files and
/// tens of GiB here, slowing everything that touches the directory. Only
/// objects whose owning artifact no longer exists are removed, so current
/// binaries stay debuggable. See
/// `salvo_testkit::prune_stale_debug_objects`.
#[test]
fn stale_debug_objects_are_pruned() {
    // This test binary lives in the very directory it cleans.
    let exe = std::env::current_exe().expect("no current exe");
    let deps: PathBuf = exe.parent().expect("exe has no parent").to_path_buf();
    assert!(
        deps.ends_with("deps"),
        "expected to run from target/*/deps, got {}",
        deps.display()
    );
    let removed = salvo_testkit::prune_stale_debug_objects(&deps);
    if removed > 0 {
        eprintln!(
            "pruned {removed} stale debug objects from {}",
            deps.display()
        );
    }
}
