//! [type-name-shadow] A type name is one name whatever kind declares it — a
//! struct, a type alias or an opaque type — so a program's own declaration
//! shadows an implicit `std.core` one of any of the three kinds. Until
//! 2026-10-20 a user `type Field = …` sat beside `std.core.comptime`'s
//! `comptime struct Field`, and a use of `Field` meant one or the other
//! depending on which table a lookup read first: two lines of one fn read it
//! differently (ROADMAP §2d item 4).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// `std.core.comptime`'s shape, enough to collide with: compile-time structs and
/// a compile-time alias over them.
const COMPTIME: &str = concat!(
    "export comptime struct Field { name: Str }\n",
    "export comptime struct Arm { name: Str }\n",
    "export comptime struct Struct { name: Str }\n",
    "export comptime struct Union { name: Str }\n",
    "export comptime type Kind = Struct | Union\n",
);

const BASICS: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add("std/core/basic.sv", SourceSet::classify(Path::new("std/core/basic.sv")).unwrap(), BASICS.to_string(), true);
    sources.add(
        "std/core/comptime.sv",
        SourceSet::classify(Path::new("std/core/comptime.sv")).unwrap(),
        COMPTIME.to_string(),
        true,
    );
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(parse_errors.is_empty(), "parse errors in {}: {parse_errors:?}", file.name);
        modules.push(ast);
    }
    let program = Program { files: sources.files, modules: Program::share(modules), companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    checked.errors.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
}

/// [type-name-shadow] Each `std.core.comptime` name, shadowed by a user alias
/// and by a user struct, reads as the user's everywhere: a parameter, a
/// return type, a `let`, a field.
#[test]
fn a_program_type_shadows_a_core_comptime_name_of_any_kind() {
    for name in ["Field", "Arm", "Struct", "Union", "Kind"] {
        for decl in [format!("type {name} = Int\n"), format!("struct {name} {{ n: Int }}\n")] {
            let alias = decl.starts_with("type");
            let value = if alias { "3".to_string() } else { format!("{name} {{ n: 3 }}") };
            let src = format!(
                "{decl}struct Holder {{ x: {name} }}\n\
                 fn take(x: {name}) [] -> {name} => !x {{ return x }}\n\
                 fn main() [] -> None {{\n    let a: {name} = {value}\n    let b: {name} = take(a)\n    let _h = Holder {{ x: b }}\n}}\n"
            );
            assert_eq!(errors(&src), Vec::<String>::new(), "{src}");
        }
    }
}

/// [comptime-fields] Without a declaration of its own, the program still
/// cannot use a compile-time type as data.
#[test]
fn an_unshadowed_comptime_name_is_still_refused_as_data() {
    let errs = errors("fn take(x: Field) [] -> Int => x { return 0 }\n");
    assert!(errs.iter().any(|e| e.contains("`Field` is a compile-time type")), "{errs:?}");
}
