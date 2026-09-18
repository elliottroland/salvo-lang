//! [mod-export] Module-private by default, `export` to let a declaration out
//! (user decision 2026-09-18).
//!
//! The rule is one line — a declaration is visible outside its own module only
//! if it says `export` — and what the tests here pin is the surface around it:
//! every declaration kind takes the modifier, the *diagnostics* say "declared
//! but not exported" rather than "no such name" (the difference between a rule
//! and a mystery), an import of a private name is refused at the import, and
//! `export` stays an ordinary identifier everywhere else.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// The base types the checker needs, as a std file [intrinsic-std-only] — and
/// exported, since std obeys this rule like anything else.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\n\
                           export intrinsic type Bool\nexport intrinsic type None\n";

/// Parses + resolves + checks a multi-file program over the prelude above,
/// returning every diagnostic message.
fn diags(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let mut out: Vec<String> = resolution.errors.iter().map(|d| d.message.clone()).collect();
    let symbols = Symbols::collect(&program);
    let checked = check_program(&program, &resolution, &symbols);
    out.extend(checked.errors.iter().map(|d| d.message.clone()));
    out
}

// ===================== the rule =====================

/// [mod-export] An exported declaration crosses; the module's own code sees
/// everything either way.
#[test]
fn an_exported_declaration_crosses_and_a_private_one_does_not() {
    let lib = "export fn shown() [] -> Int {\n    return 1\n}\n\n\
               fn hidden() [] -> Int {\n    return 2\n}\n\n\
               export fn both() [] -> Int {\n    return shown() + hidden()\n}\n";
    // Its own module reaches the private one — that is what "module-private"
    // means, and `both` calling `hidden` proves it.
    let user = "import lib.shown\nimport lib.both\n\n\
                fn use_them() [] -> Int {\n    return shown() + both()\n}\n";
    assert!(
        diags(&[("lib.sv", lib), ("main.sv", user)]).is_empty(),
        "{:?}",
        diags(&[("lib.sv", lib), ("main.sv", user)])
    );
}

/// [mod-export] The diagnostic is the feature: a name that exists and is
/// private says so, and names the fix.
#[test]
fn a_private_name_is_reported_as_private_not_missing() {
    let lib = "fn hidden() [] -> Int {\n    return 2\n}\n";
    let user = "fn f() [] -> Int {\n    return hidden()\n}\n";
    let errs = diags(&[("lib.sv", lib), ("main.sv", user)]);
    assert!(
        errs.iter().any(|m| m.contains("no function named `hidden`")
            && m.contains("declared in module `lib` but not exported")
            && m.contains("write `export`")),
        "{errs:?}"
    );
}

/// [mod-export] …and it does not suggest an import that could not work.
#[test]
fn a_private_name_is_not_offered_as_an_import() {
    let lib = "fn hidden() [] -> Int {\n    return 2\n}\n";
    let user = "fn f() [] -> Int {\n    return hidden()\n}\n";
    let errs = diags(&[("lib.sv", lib), ("main.sv", user)]);
    assert!(
        !errs.iter().any(|m| m.contains("import lib.hidden")),
        "a private name must not be suggested as an import: {errs:?}"
    );
}

/// [mod-export] An import of a private name is refused **at the import**,
/// which is where the reader is looking, rather than silently importing
/// nothing and failing at every use.
#[test]
fn importing_a_private_name_is_refused_at_the_import() {
    let lib = "struct Point {\n    x: Int\n}\n";
    let user = "import lib.Point\n\nfn f() [] -> Int {\n    return 1\n}\n";
    let errs = diags(&[("lib.sv", lib), ("main.sv", user)]);
    assert!(
        errs.iter().any(|m| m.contains("module `lib` declares `Point` but does not export it")
            && m.contains("cannot be imported")),
        "{errs:?}"
    );
}

/// [mod-export] A name that *is* in scope under another kind has a different
/// problem, and must not be told about exports — it would send the reader to
/// the wrong file.
#[test]
fn a_wrong_kind_name_in_scope_is_not_reported_as_private() {
    // `Tag` is a qualifier here, used where a type belongs; another module
    // declares a private `Tag` too, which must not colour the diagnostic.
    let other = "struct Tag {\n    v: Int\n}\n";
    let user = "qualifier Tag of Int\n\nfn f(x: Tag) [] -> Int {\n    return 1\n}\n";
    let errs = diags(&[("other.sv", other), ("main.sv", user)]);
    assert!(
        errs.iter().any(|m| m.contains("is a qualifier, not a type")),
        "{errs:?}"
    );
    assert!(
        !errs.iter().any(|m| m.contains("not exported")),
        "a name in scope under another kind is not an export problem: {errs:?}"
    );
}

/// [mod-export] Every declaration kind takes the modifier, and it comes
/// *first* — before `intrinsic` / `linear` / `actor` / `platform` /
/// `provenance`.
#[test]
fn every_declaration_kind_takes_the_modifier() {
    let lib = "export struct Shape {\n    side: Int\n}\n\n\
               export linear struct Ticket {\n    id: Int\n}\n\n\
               export type Alias = Int\n\n\
               export qualifier Tagged of Int\n\n\
               export provenance qualifier Vetted of Shape\n\n\
               export effect Logger {\n    fn log(m: Str) -> None => !m\n}\n\n\
               export actor effect Mailer {\n    send fn mail(m: Str) => !m\n}\n\n\
               export platform effect Host {\n    fn tick() [] -> Int\n}\n\n\
               export handler Loud of Logger {\n    fn log(m: Str) -> None {}\n}\n\n\
               export params ToText<T> {\n    fn text(v: T) -> Str => v\n}\n\n\
               export fn plain() [] -> Int {\n    return 1\n}\n\n\
               export fn close(t: Ticket) [] -> None => !t {}\n";
    let user = "import lib.Shape\nimport lib.Ticket\nimport lib.Alias\n\
                import lib.Tagged\nimport lib.Vetted\nimport lib.Logger\n\
                import lib.Mailer\nimport lib.Host\nimport lib.Loud\n\
                import lib.ToText\nimport lib.plain\n\n\
                fn f() [] -> Int {\n    return plain()\n}\n";
    let errs = diags(&[("lib.sv", lib), ("main.sv", user)]);
    assert!(
        !errs.iter().any(|m| m.contains("not export") || m.contains("unresolved import")),
        "every kind should be exportable and importable: {errs:?}"
    );
}

/// [mod-export] An `iter fn`'s generated pass type inherits the function's
/// visibility: a `for` over the pass needs the *type* in scope, so exporting
/// the function while hiding its pass would make the iterator undrivable from
/// another module.
#[test]
fn an_exported_iter_fn_exports_its_pass_type() {
    let lib = "export struct Countdown {\n    from: Int\n}\n\n\
               export qualifier Emitted<T> of T\n\
               export qualifier Finished of None\n\n\
               export iter fn next(c: Countdown) [] -> Emitted Int | Finished {\n    \
               state {\n        at: Int = c.from\n    }\n    \
               if at <= 0 {\n        return finished()\n    }\n    \
               at = at - 1\n    return emitted(at)\n}\n\n\
               export fn emitted<T>(v: T) [] -> T as Emitted => v {\n    return v\n}\n\
               export fn finished() [] -> None as Finished {\n    return None\n}\n";
    let user = "import lib.Countdown\nimport lib.iter\n\n\
                fn total() [] -> Int {\n    let sum = 0\n    \
                let c = Countdown {from: 3}\n    \
                for x in iter(c) {\n        sum = sum + x\n    }\n    \
                return sum\n}\n";
    let errs = diags(&[("lib.sv", lib), ("main.sv", user)]);
    assert!(
        !errs.iter().any(|m| m.contains("not exported")),
        "the generated pass must travel with its `iter fn`: {errs:?}"
    );
}
