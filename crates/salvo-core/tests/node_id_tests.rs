//! [node-id] A fact is keyed by the declaration its node is in and the
//! node's id within it: a key does not move when code above its declaration
//! changes, two nodes sharing a span are two keys, and a member of a handler
//! is its own declaration.

use std::path::Path;

use salvo_core::check::Key;
use salvo_core::{check_program, resolve, Checked, DeclId, DeclKind, Program, SourceSet, Symbols};

/// What the checks below need of std: the scalars, the groups `by auto`
/// stamps against, and the canonical overloads for `Int`.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export fns Hashed<T> {\n    fn hash(value: T) -> Long\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
);

/// `lib/std/core/auto.sv`'s `eq` and `hash`, as `compare_tests` has them.
const AUTO_PRELUDE: &str = concat!(
    "export intrinsic fn mix_hash(seed: Long, value: Long) [] -> Long => seed, value\n",
    "export comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {\n",
    "    [for field in T.fields] {\n",
    "        if !eq(a.[field], b.[field]) {\n            return false\n        }\n",
    "    }\n    return true\n}\n",
    "export comptime fn hash<T is Struct>(value: T) [] -> Long => value {\n",
    "    let h = 17L\n",
    "    [for field in T.fields] {\n",
    "        h = mix_hash(h, hash(value.[field]))\n",
    "    }\n    return h\n}\n",
);

fn checked(main: &str) -> (Program, Checked) {
    let (program, checked) = checked_with_errors(main);
    assert!(checked.errors.iter().all(|d| !d.is_error()), "{:?}", checked.errors.iter().map(|d| &d.message).collect::<Vec<_>>());
    (program, checked)
}

fn checked_with_errors(main: &str) -> (Program, Checked) {
    let mut sources = SourceSet::default();
    for (name, src) in [("std/core/compare.sv", STD_PRELUDE), ("std/core/auto.sv", AUTO_PRELUDE)] {
        sources.add(name, SourceSet::classify(Path::new(name)).unwrap(), src.to_string(), true);
    }
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), main.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(diagnostics.iter().all(|d| !d.is_error()), "parse errors in {}: {diagnostics:?}", file.name);
        modules.push(ast);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(expansion.diagnostics.iter().all(|d| !d.is_error()), "{:?}", expansion.diagnostics);
    let program = Program { files: sources.files, modules: Program::share(modules), companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    (program, checked)
}

fn main_file(program: &Program) -> usize {
    program.files.iter().position(|f| f.name == "main.sv").unwrap()
}

/// The id of `main`'s top-level declaration named `name`.
fn decl(program: &Program, checked: &Checked, name: &str) -> DeclId {
    let file = main_file(program);
    (0..program.modules[file].items.len())
        .filter_map(|i| checked.decls.item(file, i))
        .find(|id| id.name() == name)
        .unwrap_or_else(|| panic!("no declaration `{name}`"))
}

/// The types `checked` recorded in declaration `id`, by node, with each
/// node's text.
fn types_in(program: &Program, checked: &Checked, id: DeclId) -> Vec<(Key, String, String)> {
    let content = &program.files[main_file(program)].content;
    let mut rows: Vec<(Key, String, String)> = checked
        .expr_ty
        .iter()
        .filter(|(k, _)| k.decl == id)
        .map(|(k, ty)| (*k, content[k.span.start as usize..k.span.end as usize].to_string(), ty.to_string()))
        .collect();
    rows.sort_by_key(|(k, _, _)| k.node);
    rows
}

/// [node-id] An edit above a declaration moves its spans, not its keys: the
/// facts of `twice` are at the same keys, with the same types, whatever
/// `first`'s body holds — what a per-declaration cache stands on.
#[test]
fn a_key_does_not_move_when_code_above_its_declaration_changes() {
    let tail = "fn twice(n: Int) -> Bool {\n    return eq(n, n)\n}\n";
    let (p1, c1) = checked(&format!("fn first() -> Bool {{\n    return eq(1, 2)\n}}\n{tail}"));
    let (p2, c2) = checked(&format!("fn first() -> Bool {{\n    let a = 1\n    let b = 2\n    return eq(a, b)\n}}\n{tail}"));
    let (t1, t2) = (decl(&p1, &c1, "twice"), decl(&p2, &c2, "twice"));
    assert_eq!(t1, t2, "a declaration's id is its text's");
    let (r1, r2) = (types_in(&p1, &c1, t1), types_in(&p2, &c2, t2));
    assert!(!r1.is_empty());
    assert_eq!(r1.len(), r2.len());
    for ((k1, text1, ty1), (k2, text2, ty2)) in r1.iter().zip(&r2) {
        assert_eq!(k1, k2, "`{text1}` moved its key");
        assert_eq!((text1, ty1), (text2, ty2));
        assert_ne!(k1.span, k2.span, "the span is origin data, and it moved");
    }
}

/// [node-id] [obligation-by] The `eq` and `hash` a `by` clause stamps are
/// named at the one `by` site. Keyed by span, the later's reference
/// overwrote the earlier's; keyed by declaration and node, each keeps its
/// own (found by the A3 differential: 5,965 such keys in the suite).
#[test]
fn two_stamps_at_one_site_keep_their_own_references() {
    let main = "struct Pair : Hashed<self> by auto {\n    a: Int,\n    b: Int\n}\n";
    let (program, checked) = checked(main);
    let file = main_file(&program);
    let at = main.find("by auto").unwrap() as u32;
    let mut named: Vec<(DeclId, DeclId)> = checked
        .fn_refs
        .iter()
        .filter(|(k, _)| k.file() == file && k.span.start == at)
        .map(|(k, target)| (k.decl, *target))
        .collect();
    named.sort_by_key(|(_, t)| t.name());
    let names: Vec<&str> = named.iter().map(|(_, t)| t.name()).collect();
    assert_eq!(names, ["eq", "hash"], "{named:?}");
    // Each reference is its own stamped fn's name.
    for (owner, target) in &named {
        assert_eq!(owner, target);
    }
}

/// [node-id] [decl-id] A handler's member is a declaration of its own: the
/// facts of its body are keyed by its member id, the handler's own nodes by
/// the handler's.
#[test]
fn a_handler_member_is_keyed_by_its_own_id() {
    let main = "effect Count {\n    fn bump(by: Int) -> Bool\n}\nhandler Counter() of Count {\n    fn bump(by: Int) -> Bool {\n        return eq(by, 1)\n    }\n}\n";
    let (program, checked) = checked(main);
    let file = main_file(&program);
    let item = program.modules[file].items.iter().position(|i| matches!(i, salvo_syntax::ast::Item::Handler(_))).unwrap();
    let handler = checked.decls.item(file, item).unwrap();
    let member = checked.decls.member(file, item, 1).unwrap();
    assert_eq!(member.kind(), DeclKind::HandlerMember);
    let call = main.find("eq(by, 1)").unwrap() as u32;
    let key = checked.expr_ty.keys().find(|k| k.file() == file && k.span.start == call && k.span.len() == 9).expect("the call's type");
    assert_eq!(key.decl, member);
    assert_ne!(key.decl, handler);
}

/// [node-id] [origin] A row of `salvo facts` for a node a stamp copied is
/// listed where a diagnostic about it would land — the field its copy was
/// for, or the `by` site — not at the end of the file (ROADMAP 0ac).
#[test]
fn a_copied_node_is_listed_where_it_lands() {
    let main = "struct Pair : Hashed<self> by auto {\n    a: Int\n}\n";
    let (program, checked) = checked(main);
    let file = main_file(&program);
    let symbols = Symbols::collect(&program);
    let dump = salvo_core::facts::dump(&program, &symbols, &checked, &|f| f == file);
    let hash = dump.split("\nhash(").nth(1).expect("the stamped `hash`");
    assert!(hash.contains("`a: Int`") && hash.contains("`by auto`"), "{dump}");
    assert!(!hash.contains("`…`"), "a copied row has no text of its own only when nothing places it:\n{dump}");
}

/// [decl-rows] `Checked::split_decls` puts every row in exactly one
/// declaration's part: a node-keyed row under its key's declaration, a
/// declaration-keyed row under that declaration, a handler member's rows
/// under the member, a diagnostic under the declaration its node is in.
#[test]
fn every_row_belongs_to_one_declaration() {
    let main = "\
effect Counter {\n    fn get() -> Int\n}\n\
handler Count() of Counter {\n    fn get() -> Int {\n        let one = 1\n        return one\n    }\n}\n\
fn twice(n: Int) -> Int {\n    let m = n\n    return m\n}\n\
fn broken() -> Int {\n    return nope\n}\n";
    let (program, checked) = checked_with_errors(main);
    let file = main_file(&program);
    let symbols = Symbols::collect(&program);
    let rows = checked.split_decls(&program, &symbols);
    let total: usize = rows.parts.values().map(|p| p.expr_ty.len()).sum();
    assert_eq!(total, checked.expr_ty.len(), "every expression row in one part");
    for (id, part) in &rows.parts {
        assert!(part.expr_ty.keys().all(|k| k.decl == *id), "{id}: a row of another declaration");
        assert!(part.fn_effects.keys().all(|k| k == id), "{id}: another declaration's effects");
    }
    let errors: usize = rows.parts.values().map(|p| p.errors.len()).sum::<usize>() + rows.rest.errors.len();
    assert_eq!(errors, checked.errors.len());
    let item = |name: &str| {
        program.modules[file]
            .items
            .iter()
            .position(|i| matches!(i, salvo_syntax::ast::Item::Fn(f) if f.name.name == name))
            .unwrap()
    };
    let broken = checked.decls.item(file, item("broken")).unwrap();
    assert!(rows.parts[&broken].errors.iter().any(|d| d.message.contains("nope")), "the error is `broken`'s");
    let twice = checked.decls.item(file, item("twice")).unwrap();
    assert!(rows.parts[&twice].fn_effects.contains_key(&twice));
    let handler = program.modules[file].items.iter().position(|i| matches!(i, salvo_syntax::ast::Item::Handler(_))).unwrap();
    let member = checked.decls.member(file, handler, 1).unwrap();
    assert!(rows.parts.get(&member).is_some_and(|p| !p.expr_ty.is_empty()), "the member's body rows are the member's");
}
