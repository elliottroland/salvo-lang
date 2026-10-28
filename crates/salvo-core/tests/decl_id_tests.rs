//! [decl-id] A declaration's id is its module, kind, name path and
//! signature: it does not move when a body is edited, when declarations are
//! reordered, or when another file is added, and it tells overloads apart.

use std::path::Path;

use salvo_core::{resolve, DeclId, DeclKind, Program, SourceSet};

const PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

fn program(files: &[(&str, &str)]) -> Program {
    let mut sources = SourceSet::default();
    sources.add("std/core/prelude.sv", SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(), PRELUDE.to_string(), true);
    for (name, src) in files {
        sources.add(*name, SourceSet::classify(Path::new(name)).unwrap(), src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(diagnostics.iter().all(|d| !d.is_error()), "parse errors in {}: {diagnostics:?}", file.name);
        modules.push(ast);
    }
    let _ = salvo_core::expand(&sources.files, &mut modules);
    Program { files: sources.files, modules: Program::share(modules), companions: Vec::new() }
}

/// Every declaration's id, as `Display` renders it, with its kind.
fn ids(program: &Program) -> Vec<(String, DeclKind, DeclId)> {
    let resolution = resolve(program);
    let mut out = Vec::new();
    for (file, module) in program.modules.iter().enumerate() {
        for item in 0..module.items.len() {
            if let Some(id) = resolution.decls.item(file, item) {
                out.push((id.to_string(), id.kind(), id));
            }
            let mut sub = 1;
            while let Some(id) = resolution.decls.member(file, item, sub) {
                out.push((id.to_string(), id.kind(), id));
                sub += 1;
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn id_of(program: &Program, shown: &str) -> DeclId {
    ids(program).into_iter().find(|(s, _, _)| s == shown).unwrap_or_else(|| panic!("no `{shown}` among {:?}", ids(program).iter().map(|x| x.0.clone()).collect::<Vec<_>>())).2
}

const MAIN: &str = "\
struct Point { x: Int }
effect Log { fn log(line: Str) -> None }
handler Quiet of Log { fn log(line: Str) -> None {} }
fn size(p: Point) -> Int { p.x }
fn size(s: Str) -> Int { 1 }
fn label<T>(v: T) -> Str { \"v\" }
";

#[test]
fn every_kind_has_an_id_named_by_its_module_and_signature() {
    let shown: Vec<(String, DeclKind)> = ids(&program(&[("main.sv", MAIN)])).into_iter().filter(|(s, _, _)| s.ends_with("@main")).map(|(s, k, _)| (s, k)).collect();
    assert_eq!(
        shown,
        vec![
            ("Log.log(Str) -> None@main".to_string(), DeclKind::EffectMember),
            ("Log@main".to_string(), DeclKind::Effect),
            ("Point@main".to_string(), DeclKind::Struct),
            ("Quiet.log(Str) -> None@main".to_string(), DeclKind::HandlerMember),
            ("Quiet@main".to_string(), DeclKind::Handler),
            // [fn-overload-duplicate] Type parameters are positional.
            ("label(#0) -> Str@main".to_string(), DeclKind::Fn),
            ("size(Point) -> Int@main".to_string(), DeclKind::Fn),
            ("size(Str) -> Int@main".to_string(), DeclKind::Fn),
        ]
    );
}

#[test]
fn an_id_survives_body_edits_reordering_and_other_files() {
    let before = program(&[("main.sv", MAIN)]);
    let edited = MAIN.replace("{ p.x }", "{ let y = p.x\n y }").replace("{ 1 }", "{ 2 }");
    // The declarations reordered, and the type parameter renamed.
    let reordered = "\
fn label<U>(v: U) -> Str { \"w\" }
fn size(s: Str) -> Int { 1 }
handler Quiet of Log { fn log(line: Str) -> None {} }
fn size(p: Point) -> Int { p.x }
effect Log { fn log(line: Str) -> None }
struct Point { x: Int }
";
    let other = "fn size(b: Bool) -> Int { 0 }\nstruct Point { y: Int }\n";
    for after in [
        program(&[("main.sv", &edited)]),
        program(&[("main.sv", reordered)]),
        program(&[("lib/other.sv", other), ("main.sv", MAIN)]),
    ] {
        for shown in ["size(Point) -> Int@main", "size(Str) -> Int@main", "label(#0) -> Str@main", "Point@main", "Quiet.log(Str) -> None@main"] {
            assert_eq!(id_of(&before, shown), id_of(&after, shown), "`{shown}` moved");
        }
    }
}

#[test]
fn overloads_by_return_type_are_distinct() {
    // [fn-overload-return]
    let p = program(&[("main.sv", "fn parse(s: Str) -> Int { 1 }\nfn parse(s: Str) -> Bool { true }\n")]);
    assert_ne!(id_of(&p, "parse(Str) -> Int@main"), id_of(&p, "parse(Str) -> Bool@main"));
}

#[test]
fn only_a_duplicate_takes_an_ordinal() {
    // [fn-overload-duplicate] refuses the second; it still has an id of its
    // own, so the checker can go on past the error.
    let p = program(&[("main.sv", "fn f(a: Int) -> Int { a }\nfn f(b: Int) -> Int { b }\n")]);
    let first = id_of(&p, "f(Int) -> Int@main");
    let second = id_of(&p, "f(Int) -> Int@main#1");
    assert_ne!(first, second);
    assert_eq!((first.ordinal(), second.ordinal()), (0, 1));
    let resolution = resolve(&p);
    assert!(resolution.errors.iter().any(|d| d.message.contains("duplicate fn `f(Int) -> Int`")), "{:?}", resolution.errors);
}

/// [decl-id] [type-identity] A type's key is its declaration's alone: a
/// user declaration is `Name§module` whether or not another module declares
/// the name, so adding one re-keys nothing.
#[test]
fn a_type_key_does_not_move_when_another_module_declares_the_name() {
    let key = |p: &Program| {
        let symbols = salvo_core::Symbols::collect(p);
        let point = p.modules.iter().zip(&p.files).filter(|(_, f)| f.module.to_string() == "main").flat_map(|(m, _)| &m.items).find_map(|i| match i {
            salvo_syntax::ast::Item::Struct(s) if s.name.name == "Point" => Some(s),
            _ => None,
        });
        symbols.key_of(point.expect("main's Point")).map(str::to_string)
    };
    let alone = program(&[("main.sv", MAIN)]);
    let beside = program(&[("a/geometry.sv", "export struct Point { y: Int }\n"), ("main.sv", MAIN)]);
    assert_eq!(key(&alone).as_deref(), Some("Point§main"));
    assert_eq!(key(&alone), key(&beside));
    // std's own declarations keep their names.
    let symbols = salvo_core::Symbols::collect(&alone);
    assert!(symbols.key_modules.contains_key("Int"), "std's `Int` is keyed `Int`");
}
