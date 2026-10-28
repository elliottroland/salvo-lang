//! [test-data] The checker's test data (TESTS.md T1): every `.sv` file under
//! `tests/check/<area>/` is checked over the real std, and what it reports
//! must be exactly what its `//~` annotations say (`salvo_testkit::data`; the
//! format is in `tests/check/README.md`). One test per area, so a failure
//! names its area and nextest starts one process per area, not per case.
//!
//! A new area is a directory and one `area!(name);` line at the end of this
//! file.

use std::path::Path;

macro_rules! area {
    ($name:ident) => {
        #[test]
        fn $name() {
            salvo_testkit::data::check_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/check").join(stringify!($name)));
        }
    };
}

/// Every directory under `tests/check/` has its `area!` line, so a new one
/// cannot go unchecked.
#[test]
fn every_area_has_a_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dirs: Vec<String> = std::fs::read_dir(root.join("tests/check"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    dirs.sort();
    let me = std::fs::read_to_string(root.join("tests/check_data.rs")).unwrap();
    let mut listed: Vec<String> = me
        .lines()
        .filter_map(|l| l.strip_prefix("area!(")?.strip_suffix(");"))
        .map(str::to_string)
        .collect();
    listed.sort();
    assert_eq!(dirs, listed, "tests/check/'s areas and this file's `area!` lines differ");
}

area!(operators);
