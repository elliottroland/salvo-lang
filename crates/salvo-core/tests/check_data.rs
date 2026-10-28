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
area!(copy_scalar);
area!(dependent);
area!(compare);
area!(members);
area!(actors);
area!(monitors);
area!(mixed);
area!(once);
area!(widening);
area!(deduce);
area!(refinements);
area!(defaults);
area!(guard);
area!(linear);
area!(with);
area!(elvis);
area!(is_bind);
area!(subject);
area!(literals);
area!(type_args);
area!(when);
area!(bytes);
area!(strings);
area!(iter_fn);
area!(implicits);
area!(seq);
area!(field_mut);
area!(opaque);
area!(collections);
area!(comptime);
area!(overloads);
area!(carry);
area!(places);
area!(diagnostics);
area!(names);
area!(module_imports);
area!(exports);
area!(effect_at);
area!(fn_effects);
area!(multi_effect);
area!(eff_handle);
area!(groups);
area!(prereqs);
area!(aliases);
area!(platform);
area!(elem_distinct);
area!(wire);
area!(assertions);
area!(unique_shape);
area!(throws);
area!(stamp_signature);
area!(qualifiers);
area!(effects);
area!(types);
area!(control);
area!(ownership);
area!(fate);
area!(syntax);
