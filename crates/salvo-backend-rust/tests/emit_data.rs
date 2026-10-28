//! [test-emit-data] The Rust backend's emit data: every program under
//! `tests/emit/<area>/*.sv`, emitted over the process's std front, must hold
//! the code its `//~ emits` annotations name, or be refused with exactly the
//! ids its `//~ refused[…]` annotations name (the format:
//! `salvo_testkit::data`; one test per area).

use std::path::Path;

macro_rules! areas {
    ($($area:ident),* $(,)?) => {
        $(
            #[test]
            fn $area() {
                salvo_testkit::data::emit_dir(&emit_root().join(stringify!($area)), &EMITTER);
            }
        )*
        /// Every directory under `tests/emit/` has its test.
        #[test]
        fn every_area_has_a_test() {
            let listed: Vec<&str> = vec![$(stringify!($area)),*];
            for e in std::fs::read_dir(emit_root()).unwrap().flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                assert!(!e.path().is_dir() || listed.contains(&name.as_str()), "tests/emit/{name} has no `areas!` entry");
            }
        }
    };
}

areas!(
    pilot, actors, collections, control, deductions, effects, functions, handles, implicits, iteration, linear, modules, network, operators,
    platform, qualifiers, throwing, types, unions, strings, wire,
);

fn emit_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/emit")
}

const EMITTER: salvo_testkit::data::Emitter = salvo_testkit::data::Emitter { ext: "rs", emit: &emit, own: &own };

fn emit(program: &salvo_core::Program) -> Result<Vec<(std::path::PathBuf, String)>, Vec<String>> {
    salvo_backend_rust::emit_program_over(program, salvo_testkit::front::std_front("rs"))
        .map(|files| files.into_iter().map(|f| (f.rel_path, f.content)).collect())
}

/// The program's own files: not std's modules, std's platform files or the
/// runtime's (`unions/`, `wire.rs`, …).
fn own(rel: &Path) -> bool {
    const RUNTIME: &[&str] = &["wire.rs", "strings.rs", "seq.rs", "scheduler.rs", "hoststreams.rs", "hosttime.rs"];
    !(rel.starts_with("std") || rel.starts_with("platform/std") || rel.starts_with("unions") || RUNTIME.iter().any(|r| rel == Path::new(r)))
}
