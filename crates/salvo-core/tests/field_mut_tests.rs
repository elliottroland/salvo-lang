//! [field-canbe-mut] `name: canbe Mut T` on a struct field: the field is
//! `Mut T` exactly when the struct value is `Mut`, so a plain struct is
//! immutable all the way down. What the checker refuses, and what it types.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`).
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic fn list_of<T>(...elems: T[]) [] -> List<T>\n",
    "export intrinsic fn mut_list_of<T>(...elems: T[]) [] -> Mut List<T>\n",
    "export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n",
    "export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
);

fn messages(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(parse_errors.is_empty(), "parse errors in {}: {parse_errors:?}", file.name);
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    checked
        .errors
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const BAG: &str = "struct Bag canbe Mut {\n    n: Int,\n    items: canbe Mut List<Int>\n}\n\n";

/// A `Mut` struct's field is `Mut`; a plain one's is not.
#[test]
fn the_field_is_mut_exactly_when_the_struct_is() {
    let ok = format!(
        "{BAG}fn fill(b: Mut Bag) [] -> None => b: Mut {{\n    add(b.items, 1)\n}}\n\n\
         fn count(b: Bag) [] -> Int => b {{\n    return size(b.items)\n}}\n\n\
         fn make() [] -> Mut Bag {{\n    return Mut Bag {{ n: 0, items: mut_list_of<Int>() }}\n}}\n\n\
         fn plain() [] -> Bag {{\n    return Bag {{ n: 0, items: list_of(1) }}\n}}\n"
    );
    let errs = messages(&ok);
    assert!(errs.is_empty(), "{errs:?}");
    let bad = format!("{BAG}fn bad(b: Bag) [] -> None => b {{\n    add(b.items, 1)\n}}\n");
    let errs = messages(&bad);
    assert!(errs.iter().any(|e| e.contains("no matching overload for `add(List<Int>, Int)`")), "{errs:?}");
}

/// A `Mut` literal needs `Mut` field values, and may not spread a plain value
/// or rely on a (plain) default for a `canbe Mut` field.
#[test]
fn a_mut_literal_takes_mut_fields_only() {
    let src = format!(
        "{BAG}struct Def canbe Mut {{\n    items: canbe Mut List<Int> = list_of<Int>()\n}}\n\n\
         fn bad(p: Bag) [] -> None => p {{\n    let w = Mut Bag {{ n: 1, items: list_of(1) }}\n    \
         let m = Mut Bag {{ n: 1, ...copy_bag(p) }}\n    let d = Mut Def {{}}\n}}\n\n\
         fn copy_bag(p: Bag) [] -> Bag => p {{\n    return Bag {{ n: p.n, items: list_of<Int>() }}\n}}\n"
    );
    let errs = messages(&src);
    assert!(errs.iter().any(|e| e.contains("field `items` expects `Mut List<Int>`")), "{errs:?}");
    assert!(
        errs.iter().any(|e| e.contains("a `Mut Bag` literal cannot take field `items` (`canbe Mut`) from a plain `Bag`")),
        "{errs:?}"
    );
    assert!(errs.iter().any(|e| e.contains("a `Mut Def` literal must give field `items`")), "{errs:?}");
}

/// Where `canbe Mut` on a field is refused: in a struct that is never `Mut`,
/// over a type that cannot be, and over a type that already is.
#[test]
fn canbe_mut_fields_are_validated_at_the_declaration() {
    let src = "struct Plain {\n    items: canbe Mut List<Int>\n}\n\n\
               struct Bad canbe Mut {\n    a: canbe Mut Int,\n    b: canbe Mut Mut List<Int>\n}\n";
    let errs = messages(src);
    assert!(errs.iter().any(|e| e.contains("but struct `Plain` does not")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("`canbe Mut` does not apply to `Int`")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("field `b` is already `Mut`")), "{errs:?}");
    assert_eq!(errs.len(), 3, "{errs:?}");
}
