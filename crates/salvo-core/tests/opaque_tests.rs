//! [struct-opaque] `opaque struct`: an ordinary struct in its declaring
//! module and that module's `*.test.sv` annex; elsewhere the type is usable
//! but its fields are not — no literal, field read or write, spread or
//! destructuring — and a comptime fn stamped there sees kind `opaque`.

use std::path::Path;

use salvo_core::{check_program, resolve, ModulePath, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Bool\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic fn list_of<T>(...elems: T[]) [] -> List<T>\n",
);

const BAG: &str = "export opaque struct Bag {\n    items: List<Int>,\n    total: Int\n}\n\n\
                   export fn bag() [] -> Bag {\n    return Bag { items: list_of<Int>(), total: 0 }\n}\n\n\
                   export fn total(b: Bag) [] -> Int => b {\n    return b.total\n}\n";

fn errors(files: &[(&str, &str)], annex: Option<&str>) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(*name, SourceSet::classify(Path::new(name)).unwrap(), src.to_string(), false);
    }
    if let Some(src) = annex {
        sources.add_test("bag.test.sv", ModulePath::parse("bag.test"), src.to_string());
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}: {diagnostics:?}", file.name);
        modules.push(ast);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program { files: sources.files, modules, companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .chain(expansion.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.clone()))
        .collect()
}

/// Outside the module, every way into the fields is refused, naming the type
/// as opaque and pointing at its functions; the type itself and its
/// functions are fine.
#[test]
fn outside_its_module_the_fields_are_not_there() {
    let main = "import bag.Bag\nimport bag.bag\nimport bag.total\n\n\
                fn fine() [] -> Int {\n    let b: Bag = bag()\n    return total(b)\n}\n\n\
                fn read(b: Bag) [] -> Int => b {\n    return b.total\n}\n\n\
                fn split(b: Bag) [] -> Int => b {\n    let {total} = b\n    return total\n}\n\n\
                fn build() [] -> Bag {\n    return Bag { items: list_of<Int>(), total: 1 }\n}\n";
    let errs = errors(&[("bag.sv", BAG), ("main.sv", main)], None);
    let opaque: Vec<&String> = errs.iter().filter(|e| e.contains("`Bag` is opaque outside `bag`")).collect();
    assert_eq!(opaque.len(), 3, "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("reading its field `total`")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("destructuring it")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("building one with a literal")), "{errs:?}");
    assert!(errs.iter().all(|e| !e.contains("opaque") || e.contains("`total`")), "points at its fns: {errs:?}");
}

/// The module's test annex counts as the module.
#[test]
fn the_test_annex_sees_inside() {
    let annex = "fn peek() [] -> Int {\n    let b = bag()\n    return b.total\n}\n";
    let errs = errors(&[("bag.sv", BAG)], Some(annex));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A comptime fn stamped outside the module sees kind `opaque`, so `by auto`
/// there cannot walk the fields.
#[test]
fn a_stamp_outside_the_module_sees_kind_opaque() {
    let auto = "export comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {\n    return true\n}\n";
    let main = "import bag.Bag\n\nfn same(a: Bag, b: Bag) [] -> Bool by auto\n";
    let errs = errors(&[("bag.sv", BAG), ("auto.sv", auto), ("main.sv", main)], None);
    assert!(errs.iter().any(|e| e.contains("which is opaque outside its module")), "{errs:?}");
}
