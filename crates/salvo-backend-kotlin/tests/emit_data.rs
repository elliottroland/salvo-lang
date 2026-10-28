//! [test-emit-data] The Kotlin backend's emit data: every program under
//! `tests/emit/<area>/*.sv`, emitted over the process's std front, must hold
//! the code its `//~ emits` annotations name, or be refused with exactly the
//! ids its `//~ refused[…]` annotations name (the format:
//! `salvo_testkit::data`; one test per area).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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

areas!(pilot, unions, qualifiers, iteration, types, effects, handles, operators, control, modules, copy, narrowing, collections, names, throwing, platform, wire, functions, implicits, strings, actors);

fn emit_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/emit")
}

const EMITTER: salvo_testkit::data::Emitter = salvo_testkit::data::Emitter { ext: "kt", emit: &emit, own: &own };

fn emit(program: &salvo_core::Program) -> Result<Vec<(PathBuf, String)>, Vec<String>> {
    salvo_backend_kotlin::emit_program_over(program, salvo_testkit::front::std_front("kt"))
        .map(|files| files.into_iter().map(|f| (f.rel_path, f.content)).collect())
}

/// [kt-std-library] The program's own files: the ones std's library build
/// does not emit, and not the union arities.
fn own(rel: &Path) -> bool {
    static STD: OnceLock<BTreeSet<PathBuf>> = OnceLock::new();
    let std = STD.get_or_init(|| {
        let program = salvo_testkit::front::clean_program("kt", &[]);
        salvo_backend_kotlin::stdlib::std_library_files(&program)
            .unwrap_or_else(|e| panic!("std library emission failed:\n{}", e.join("\n")))
            .into_iter()
            .map(|f| f.rel_path)
            .collect()
    });
    !std.contains(rel) && !rel.starts_with("unions")
}
