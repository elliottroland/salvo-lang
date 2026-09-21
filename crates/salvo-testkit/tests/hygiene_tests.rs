//! Target-directory hygiene, run as a test so every full `cargo test` /
//! `cargo nextest run` does it without any extra tooling.

use std::path::PathBuf;

/// Deletes unreferenced `*.rcgu.o` debug objects from `target/debug/deps`.
///
/// macOS keeps these files as the debug info of every linked binary
/// (`split-debuginfo=unpacked`), and cargo never garbage-collects the ones
/// stranded by rebuilds — whether the artifact was renamed (a new hash) or
/// relinked in place (incremental rebuilds replace CGUs but keep the name).
/// They once piled up to 790k files / 48.7 GiB here, and again to 650k /
/// 15 GiB three weeks later, slowing everything that touches the directory.
/// An object survives iff a live binary's debug map still references it, so
/// current binaries stay debuggable. See
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

/// Pins the pruner's keep rule on a fixture directory: an object survives
/// iff a binary's debug map (` OSO ` entries) references it. A stale
/// generation of a *live* binary goes, an orphan of a *deleted* binary
/// goes, an rlib-owned loose object goes (the rlib holds its own copy
/// inside the archive) — and every object the binary references stays.
#[test]
fn the_pruner_keeps_exactly_what_debug_maps_reference() {
    if !salvo_testkit::rustc().available {
        eprintln!("skipping: rustc not on PATH (or SALVO_SKIP_E2E is set)");
        return;
    }
    if std::process::Command::new("nm").arg("--version").output().is_err() {
        eprintln!("skipping: nm not on PATH");
        return;
    }
    let dir = salvo_testkit::scratch(env!("CARGO_TARGET_TMPDIR"), "prune-fixture");

    // A real linked binary with unpacked split debuginfo: rustc writes the
    // loose objects next to it and the binary's OSO entries point at them.
    std::fs::write(dir.join("m.rs"), "fn main() { println!(\"x\") }\n").unwrap();
    let status = std::process::Command::new("rustc")
        .args(["-g", "-Csplit-debuginfo=unpacked", "-Ccodegen-units=2"])
        .args(["m.rs", "-o", "prog-1234"])
        .current_dir(&dir)
        .status()
        .expect("rustc failed to run");
    assert!(status.success(), "fixture rustc invocation failed");
    let referenced: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".rcgu.o"))
        .collect();
    assert!(
        !referenced.is_empty(),
        "expected rustc to leave loose .rcgu.o objects"
    );

    // Three kinds of garbage the pruner must remove:
    // a stale generation of the live binary (owned, unreferenced) …
    std::fs::write(dir.join("prog-1234.stalegen.rcgu.o"), b"o").unwrap();
    // … an orphan of a binary that no longer exists …
    std::fs::write(dir.join("gone-9999.x.rcgu.o"), b"o").unwrap();
    // … and a loose object owned by an rlib (the archive has its own copy).
    std::fs::write(dir.join("libfake-1.rlib"), b"a").unwrap();
    std::fs::write(dir.join("fake-1.a.rcgu.o"), b"o").unwrap();

    let removed = salvo_testkit::prune_stale_debug_objects(&dir);

    assert_eq!(removed, 3, "exactly the three garbage objects go");
    for name in &referenced {
        assert!(
            dir.join(name).exists(),
            "referenced object {name} must survive"
        );
    }
    assert!(!dir.join("prog-1234.stalegen.rcgu.o").exists());
    assert!(!dir.join("gone-9999.x.rcgu.o").exists());
    assert!(!dir.join("fake-1.a.rcgu.o").exists());
    assert!(dir.join("prog-1234").exists(), "the binary itself stays");
    assert!(dir.join("libfake-1.rlib").exists(), "the rlib itself stays");
}
