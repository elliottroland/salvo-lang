//! [cmp-groups] Comparison, equality and hashing as **fns groups**: what is
//! left in Rust after the move to test data (TESTS.md T1). The diagnostics
//! are `tests/check/compare/`, over the real std; what stays here is
//!
//! - tests whose old diagnostic set holds a second diagnostic for one mistake
//!   (a `by` clause that fails also reports its obligation unmet; a group
//!   argument that is not `self` is reported twice; checking comptime syntax
//!   outside a `comptime fn` past its parse error reports an internal error),
//!   kept as they were, over this file's prelude, until that is settled;
//! - tests whose point is a std file of their own (an `intrinsic type`, which
//!   only std may write), over the prelude;
//! - [node-origin] tests of what a stamp records, which annotations cannot
//!   say, over the real std (`salvo_testkit::front`).
//!
//! The prelude mirrors `lib/std/core/compare.sv` and `auto.sv`'s struct half.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] Loaded as a *std* file, since only std writes
/// `intrinsic` — and `std.core.*` is implicitly visible, which is how a canonical
/// `cmp` is in scope without an import.
const STD_PRELUDE: &str = concat!(
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Double\n",
    "export intrinsic type Str canbe Mut\n",
    "export fns Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export fns Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export fns Hashed<T> {\n    fn hash(value: T) -> Long\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn hash(value: Str) [] -> Long => value\n",
    "export intrinsic fn to_long(value: Int) [] -> Long => value\n",
),
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b\n",
);

/// [obligation-by] The structural `comptime fn`s `by auto` stamps, mirroring
/// `lib/std/core/auto.sv`'s struct half: the tests here are about what a `by`
/// clause does to a type, not about the bodies.
const AUTO_PRELUDE: &str = concat!(
    "export intrinsic fn mix_hash(seed: Long, value: Long) [] -> Long => seed, value\n",
    "export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {\n",
    "    [for field in T.fields] {\n",
    "        let c = cmp(a.[field], b.[field])\n",
    "        if c != 0 {\n            return c\n        }\n",
    "    }\n    return 0\n}\n",
    "export comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {\n",
    "    [for field in T.fields] {\n",
    "        if !eq(a.[field], b.[field]) {\n            return false\n        }\n",
    "    }\n    return true\n}\n",
    "export comptime fn hash<T is Struct>(value: T) [] -> Long => value {\n",
    "    [if T.mutable] {\n",
    "        refuse!(\"a `Mut`-capable struct can change while a collection holds it, so it cannot be a key\")\n",
    "    }\n",
    "    let h = 17L\n",
    "    [for field in T.fields] {\n",
    "        h = mix_hash(h, hash(value.[field]))\n",
    "    }\n    return h\n}\n",
);

fn errors(src: &str) -> Vec<&'static str> {
    errors_in(&[("main.sv", src)])
}

/// The same over several user files, which is what the canonical rules need:
/// "the type's own file" and "imported with the type" are both about *files*.
///
/// [diag-id] Each error by its id.
fn errors_in(files: &[(&str, &str)]) -> Vec<&'static str> {
    checked_in(files, |_, _, diagnostics| diagnostics.iter().filter(|d| d.is_error()).map(|d| d.id.unwrap_or("?")).collect())
}

/// The files with this file's preludes, expanded, resolved and checked: `f`
/// reads the program, its resolution and every diagnostic (the expansion's,
/// the resolution's, the checker's).
fn checked_in<R>(files: &[(&str, &str)], f: impl FnOnce(&Program, &salvo_core::Resolution, &[salvo_core::FileDiagnostic]) -> R) -> R {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/compare.sv",
        SourceSet::classify(Path::new("std/core/compare.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/auto.sv",
        SourceSet::classify(Path::new("std/core/auto.sv")).unwrap(),
        AUTO_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            src.to_string(),
            false,
        );
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "unexpected parse errors: {parse_errors:?}"
        );
        modules.push(ast);
    }
    // [comptime-instantiate] The `by` sites stamp here, before resolution.
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let diagnostics: Vec<salvo_core::FileDiagnostic> = expansion
        .diagnostics
        .iter()
        .chain(resolution.errors.iter())
        .chain(checked.errors.iter())
        .cloned()
        .collect();
    f(&program, &resolution, &diagnostics)
}

// ===== kept over the prelude: a second diagnostic for one mistake =====

/// [cmp-auto] `default` is legal only where the compiler has a generator: on
/// any other group the word would promise an implementation nothing provides.
#[test]
fn default_is_refused_on_a_group_with_no_generator() {
    let errs = errors(
        r#"
fns Step<It, T> {
    fn advance(it: Mut It) -> T
}

struct Counter : Step<self, Int> by auto canbe Mut {
    at: Int
}
"#,
    );
    // `auto` has no `advance`, and the clause's argument is not `self` either.
    assert!(
        errs.contains(&"obligation-by:not-self") || errs.contains(&"obligation-by:module-lacks-member"),
        "expected the stamping to be refused, got: {errs:?}"
    );
}

/// The generator writes the signature over the declaring type, so the argument
/// is `self` [group-self].
#[test]
fn a_default_obligation_takes_self() {
    let errs = errors(
        r#"
struct Point : Ordered<Int> by auto {
    x: Int
}
"#,
    );
    assert!(
        errs.contains(&"obligation-by:not-self"),
        "expected the `self` rule to be named, got: {errs:?}"
    );
}

/// [obligation-by] [comptime-instantiate] [comptime-refuse] What a stamped body
/// needs of the type is checked where the type is declared: a float field has no
/// `hash` (reported at the field, in the copy's terms), and `std.core.auto`'s `hash`
/// refuses a mutable struct (reported at the clause).
#[test]
fn a_default_obligation_validates_the_fields() {
    let float_field = errors(
        r#"
struct Sample : Hashed<self> by auto {
    at: Double
}
"#,
    );
    assert!(
        float_field.contains(&"call-resolve:no-overload"),
        "expected the float field to be refused, got: {float_field:?}"
    );

    let mutable = errors(
        r#"
struct Counter : Hashed<self> by auto canbe Mut {
    at: Int
}
"#,
    );
    assert!(
        mutable.contains(&"comptime-refuse:refused"),
        "expected the mutable struct to be refused, got: {mutable:?}"
    );
}

/// [comptime-refuse] A `refuse` is an error at the `by` site, in the caller's
/// terms, naming the type.
#[test]
fn a_refuse_lands_at_the_by_site() {
    let errs = errors(
        r#"
comptime fn tag<T is Struct>(v: T) [] -> Int => v {
    [if T.mutable] {
        refuse!("no tags for a mutable ${T.name}")
    }
    return 1
}

fns Tagged<T> {
    fn tag(v: T) -> Int
}

struct P : Tagged<self> by tag canbe Mut {
    x: Int
}
"#,
    );
    assert!(
        errs.contains(&"comptime-refuse:refused"),
        "expected the refusal with the name substituted: {errs:?}"
    );
}

/// [obligation-by] A name that is both a module and a `comptime fn` is refused
/// as ambiguous rather than defaulted, and each reading has a spelling: `by
/// @tag` for the module (a module stands right of `@`), `by tag@<module>` for
/// the function (user decisions 2026-09-29). Only a module the file can see is
/// a reading at all (user decision 2026-10-14): without `import tag` the
/// module is not one, and `by tag` is the fn.
#[test]
fn a_by_name_that_is_both_a_module_and_a_fn_is_ambiguous() {
    let files: &[(&str, &str)] = &[
        (
            "tag.sv",
            r#"
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    return 0
}
"#,
        ),
        (
            "main.sv",
            r#"
import tag

comptime fn tag<T is Struct>(a: T, b: T) [] -> Int => a, b {
    return 1
}

fns Tagged<T> {
    fn cmp(a: T, b: T) -> Int
}

struct P : Tagged<self> by tag {
    x: Int
}
"#,
        ),
    ];
    let errs = errors_in(files);
    assert!(
        errs.contains(&"obligation-by:ambiguous-by"),
        "expected the two spellings: {errs:?}"
    );

    // Each selector settles it.
    let module_src = files[1].1.replace("by tag {", "by @tag {");
    let module_form = errors_in(&[files[0], ("main.sv", &module_src)]);
    assert!(module_form.is_empty(), "unexpected errors: {module_form:?} for {module_src}");
    let fn_form = errors_in(&[files[0], ("main.sv", &files[1].1.replace("by tag {", "by tag@main {"))]);
    assert!(fn_form.is_empty(), "unexpected errors: {fn_form:?}");
    // Not imported, the module is no reading: `by tag` is the fn.
    let unimported = errors_in(&[files[0], ("main.sv", &files[1].1.replace("import tag\n", ""))]);
    assert!(unimported.is_empty(), "unexpected errors: {unimported:?}");
}

/// [obligation-by] [mod-suffix] A `by` path names only the modules its file
/// can see (user decision 2026-10-14, ROADMAP 0s): a project module `auto` is
/// no second reading of `by auto` in a file that does not import it — std's
/// own sites included — and is one in a file that does, where more of the
/// path settles it.
#[test]
fn a_by_path_names_only_the_modules_its_file_can_see() {
    let auto = ("auto.sv", "export fn helper() [] -> Int {\n    return 1\n}\n");
    let point = "struct P : Ordered<self> by auto {\n    x: Int\n}\n";
    let alone = errors_in(&[auto, ("main.sv", point)]);
    assert!(alone.is_empty(), "`auto` is not imported here: {alone:?}");
    let imported = format!("import auto\n\n{point}\nfn main() [] {{\n    let x = helper()\n}}\n");
    let errs = errors_in(&[auto, ("main.sv", &imported)]);
    assert!(
        errs.contains(&"obligation-by:several-modules"),
        "the file's own ambiguity: {errs:?}"
    );
    let settled = errors_in(&[auto, ("main.sv", &imported.replace("by auto", "by core.auto"))]);
    assert!(settled.is_empty(), "`by core.auto` settles it: {settled:?}");
    // A module the file does not see is named in the help, with the import.
    let derive = ("derive.sv", "export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {\n    return 0\n}\n");
    // Kept on the text: the point is the import the message suggests.
    let errs: Vec<String> = checked_in(&[derive, ("main.sv", "struct P : Ordered<self> by derive {\n    x: Int\n}\n")], |_, _, diagnostics| {
        diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
    });
    assert!(errs.iter().any(|e| e.contains("add `import derive`")), "{errs:?}");
}

/// [comptime-inline] Every comptime construct belongs inside a `comptime fn`; in an
/// ordinary fn each is a parse error naming the scope.
#[test]
fn comptime_syntax_outside_a_comptime_fn_is_refused() {
    for src in [
        "fn f(p: Int) -> Int => p {\n    [for x in Int.fields] { }\n    return p\n}\n",
        "fn f(p: Int) -> Int => p {\n    refuse!(\"no\")\n    return p\n}\n",
        "struct P { x: Int }\nfn f(p: P) -> Int => p {\n    return p.[x]\n}\n",
    ] {
        let (_, diags) = salvo_syntax::parse_module(src);
        assert!(
            diags.iter().any(|d| d.id == Some("comptime-inline:outside-compfn")),
            "expected the scope to be named for {src:?}: {diags:?}"
        );
    }
}

// ===== [obligation-by] obligations on an `intrinsic type` (GB Step 0) =====
//
// Over the prelude: the point is a std file of the test's own.

/// Like `errors_in`, but the test source is loaded as a **std** file, so it
/// may write `intrinsic` — the context an obligation on `Int`/`Str`/etc.
/// actually lives in (`lib/std/core/basic.sv`).
fn errors_in_std_file(src: &str) -> Vec<&'static str> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/compare.sv",
        SourceSet::classify(Path::new("std/core/compare.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/auto.sv",
        SourceSet::classify(Path::new("std/core/auto.sv")).unwrap(),
        AUTO_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/widget.sv",
        SourceSet::classify(Path::new("std/core/widget.sv")).unwrap(),
        src.to_string(),
        true,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "unexpected parse errors: {parse_errors:?}"
        );
        modules.push(ast);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    expansion
        .diagnostics
        .iter()
        .chain(resolution.errors.iter())
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.id.unwrap_or("?"))
        .collect()
}

/// [obligation-by] An `intrinsic type` may carry a **bare** obligation — a
/// checked promise that the hand-written `intrinsic fn`s exist — exactly as
/// a user struct's `: Ordered<self>` without `by auto`. `Int` already has
/// `cmp`/`eq`/`hash` in the prelude, so the claim is satisfied, clean.
#[test]
fn an_intrinsic_type_may_carry_a_bare_obligation() {
    let errs = errors_in_std_file(
        "export intrinsic type Widget : Ordered<self>, Eq<self>, Hashed<self>\n\
         export intrinsic fn cmp(a: Widget, b: Widget) [] -> Int => a, b\n\
         export intrinsic fn eq(a: Widget, b: Widget) [] -> Bool => a, b\n\
         export intrinsic fn hash(value: Widget) [] -> Long => value\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [obligation-by] …but `by auto` on an `intrinsic type` is rejected: there
/// is no body to stamp (the implementation is a hand-written `intrinsic
/// fn`), so the clause names the real reason rather than failing later with
/// a misleading "no visible `cmp` matches" (GB Step 0).
#[test]
fn by_auto_on_an_intrinsic_type_is_rejected() {
    let errs = errors_in_std_file("export intrinsic type Widget : Ordered<self> by auto\n");
    assert!(
        errs.contains(&"obligation-by:intrinsic-type"),
        "expected the by-auto rejection, got {errs:?}"
    );
}

/// [obligation-by] A bare obligation whose `intrinsic fn` is missing still
/// errors at the declaration, naming the signature — the guard narrows
/// nothing about the existing checked-promise path.
#[test]
fn an_intrinsic_type_with_a_missing_member_still_errors() {
    let errs = errors_in_std_file("export intrinsic type Widget : Ordered<self>\n");
    assert!(
        errs.contains(&"group-obligation:no-member"),
        "expected the missing-member error, got {errs:?}"
    );
}

// ===== [node-origin] a stamped body keeps its template's spans =====

/// Every span a stamped fn's body holds, with the stamped fn's stamp.
fn stamped_spans(program: &Program, file: usize, name: &str) -> (Vec<salvo_syntax::Span>, salvo_syntax::ast::Stamp) {
    use salvo_syntax::ast::Item;
    use salvo_syntax::visit::{self as v, Visitor};
    struct Spans(Vec<salvo_syntax::Span>);
    impl Visitor for Spans {
        fn visit_span(&mut self, span: &salvo_syntax::Span) {
            self.0.push(*span);
        }
    }
    let module: &salvo_syntax::ast::Module = &program.modules[file];
    let f = module
        .items
        .iter()
        .find_map(|i| match i {
            Item::Fn(f) if f.name.name == name && f.stamped.is_some() => Some(f),
            _ => None,
        })
        .expect("the stamped fn");
    let mut spans = Spans(Vec::new());
    v::walk_block(&mut spans, f.body.as_ref().expect("a stamped body"));
    (spans.0, f.stamped.clone().unwrap())
}

/// `main` checked over the real std, with the index of its file.
fn checked_main(main: &str) -> (salvo_testkit::front::Checked, usize) {
    let out = salvo_testkit::front::check_main(main);
    let file = out.program.files.iter().position(|f| f.name == "main.sv").expect("main.sv");
    (out, file)
}

/// [node-origin] [comptime-instantiate] A stamp copies the template's nodes with
/// their template spans — inside the template's file, never past the end of
/// it — under expansions the `Stamp` records: the template's whole copy, and
/// one copy per field, each for its field's declaration and made inside the
/// whole copy. Two copies of one statement are told apart by their expansion
/// alone.
#[test]
fn a_stamped_body_keeps_the_template_spans() {
    let main = "struct Point : Ordered<self> by auto {\n    x: Int,\n    y: Str\n}\n";
    let (out, file) = checked_main(main);
    let errors = out.errors();
    assert!(errors.is_empty(), "{errors:#?}");
    let auto = &out.program.files.iter().find(|f| f.is_std && f.name.ends_with("core/auto.sv")).expect("std.core.auto").content;
    let (spans, stamp) = stamped_spans(&out.program, file, "cmp");
    use salvo_syntax::ast::ExpansionKind;
    let template = stamp.expansions.iter().find(|e| e.kind == ExpansionKind::Template).expect("the template's copy");
    let copies: Vec<_> = stamp.expansions.iter().filter(|e| matches!(e.kind, ExpansionKind::Copy { .. })).collect();
    assert_eq!(copies.len(), 2, "{:?}", stamp.expansions);
    for (copy, field) in copies.iter().zip(["x", "y"]) {
        assert_eq!(copy.parent, template.id);
        let ExpansionKind::Copy { target, label, written } = &copy.kind else { unreachable!() };
        assert_eq!(&main[target.start as usize..target.end as usize].split(':').next().unwrap().trim(), &field);
        assert!(label.starts_with(&format!("Point.{field}")), "{label}");
        assert!(written.is_none(), "a stamp's copy is not of written text");
    }
    let template_len = auto.len() as u32;
    // The body's own nodes are all copies; what a copy substitutes (the
    // field's type) keeps its declaration's written span.
    let copied: Vec<_> = spans.iter().filter(|s| s.is_expanded()).collect();
    assert!(!copied.is_empty());
    for s in &copied {
        assert!(s.end <= template_len, "{s:?} is not a template span");
        assert!(stamp.expansion(s.expn).is_some(), "{s:?} names no recorded expansion");
    }
    // The `let c = cmp(…)` of each copy: one template range, two expansions.
    // The struct `cmp` is the first in `std.core.auto`.
    let let_at = auto.find("let c").unwrap() as u32;
    let lets: Vec<_> = copied.iter().filter(|s| s.start == let_at).map(|s| s.expn).collect();
    assert_eq!(lets.len(), 2, "{copied:?}");
    assert_ne!(lets[0], lets[1]);
}

/// [node-origin] [ir-analyze] An error at a copied node renders at the field its
/// copy was for, prefixed with the stamp, and names the stamped fn — the
/// declaration it breaks (`ir::gate`), whatever its span says.
#[test]
fn a_stamped_error_names_the_stamped_fn() {
    let main = "struct Sample : Hashed<self> by auto {\n    at: Double\n}\n";
    let (out, file) = checked_main(main);
    let program = &out.program;
    let diagnostics: Vec<&salvo_core::FileDiagnostic> = out.parse.iter().chain(&out.checked.errors).collect();
    // Kept on the text: the stamp's prefix is what this test renders.
    let d = diagnostics
        .iter()
        .find(|d| d.message.starts_with("in `hash` from `auto` for `Sample.at: Double`"))
        .unwrap_or_else(|| panic!("{diagnostics:?}"));
    assert!(d.span.is_written());
    assert_eq!(&main[d.span.start as usize..d.span.end as usize], "at: Double");
    let resolution = salvo_core::resolve(program);
    let loc = resolution.decls.loc(d.decl.expect("the stamped fn")).unwrap();
    assert_eq!(loc.file, file);
    let salvo_syntax::ast::Item::Fn(f) = &program.modules[file].items[loc.item] else { panic!("not a fn") };
    assert_eq!(f.name.name, "hash");
    assert!(f.stamped.is_some());
    let gate = salvo_core::ir::gate::Gate::new(program, &resolution.decls, diagnostics.iter().copied());
    assert!(gate.bodies.contains(&(file, loc.item)), "{gate:?}");
    assert!(gate.interfaces.is_empty() && gate.files.is_empty(), "{gate:?}");
}
