//! Overload resolution, finalized (user decisions 2026-09-06/07). One rule,
//! three steps, and two ways for the caller to override it:
//!
//! * [fn-overload-scope] functions arrive from ever more specific scopes —
//!   `core`, then this file's imports, then this module — and only the most
//!   specific scope that *fits the arguments* competes. Scope beats signature,
//!   with a **warning** when it discards a more specific signature;
//! * [fn-overload-rank] within a scope, the most specific signature wins: a
//!   type variable says least, a broader union says less than a narrower one
//!   than an arm, more qualifiers say more (the *kind* of qualifier never
//!   ranking), and a fixed parameter list beats a variadic one. Per argument
//!   slot, so one argument's gain never pays for another's loss;
//! * [fn-overload-ambiguous] no single most specific candidate is an error
//!   naming the remedies — never a pick;
//! * [fn-overload-at] `add@core.list(x)` names the module whose overload is
//!   meant, and [fn-rename] `rename fn add2 = add(...)` gives one overload a
//!   name of its own, taking it *out* of the shared one.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};
use salvo_syntax::diag::Severity;

/// [intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as *std* files (only std may write `intrinsic`), one per `core`
/// module so the tests can name them with `@`.
const STD_LIST: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type Char\nintrinsic type Any\nintrinsic type List<T> canbe Mut\nintrinsic fn of_list<T>(...elems: T[]) [] -> [] Mut List<T>\nintrinsic fn size<T>(list: List<T>) [] -> [list] Int\n";
const STD_STRING: &str = "intrinsic fn size(str: Str) [] -> [str] Int\n";

/// Checks a program of user files (`main.sv` first) against the std prelude.
fn checked_files(files: &[(&str, &str)]) -> salvo_core::Checked {
    let mut sources = SourceSet::default();
    for (name, src) in [
        ("std/core/list.sv", STD_LIST),
        ("std/core/string.sv", STD_STRING),
    ] {
        let rel = name.trim_start_matches("std/");
        sources.add(
            name,
            SourceSet::classify(Path::new(rel)).unwrap(),
            src.to_string(),
            true,
        );
    }
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
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    // `check_program` folds the resolution errors (duplicate fns among them)
    // into its own.
    check_program(&program, &resolution, &symbols)
}

fn checked(src: &str) -> salvo_core::Checked {
    checked_files(&[("main.sv", src)])
}

fn errors(src: &str) -> Vec<FileDiagnostic> {
    checked(src)
        .errors
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

fn warnings(src: &str) -> Vec<String> {
    checked(src)
        .errors
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .map(|d| d.message.clone())
        .collect()
}

fn probe(body: &str) -> String {
    format!("fn probe() -> [] None {{\n{body}\n}}\n")
}

// ===== [fn-overload-rank] the specificity ladder =====

/// A concrete parameter type beats a type variable, in either declaration
/// order — the case the ranking was first written for.
#[test]
fn a_concrete_parameter_beats_a_type_variable() {
    let decls = "fn describe<T>(value: T) [] -> [] Bool { return true }\n\
                 fn describe(value: Int) [] -> [] Str { return \"concrete\" }\n";
    let src = format!("{decls}{}", probe("    let picked: Str = describe(3)"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    let flipped = "fn describe(value: Int) [] -> [] Str { return \"concrete\" }\n\
                   fn describe<T>(value: T) [] -> [] Bool { return true }\n";
    let src = format!("{flipped}{}", probe("    let picked: Str = describe(3)"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A **narrower union** says more, and an arm says more than the union it
/// belongs to. The argument's own type decides what is even viable, so this
/// only ranks candidates the caller could have meant.
#[test]
fn a_narrower_union_beats_a_broader_one() {
    let decls = "fn take(v: Int | Str | Bool) [] -> [] Bool { return true }\n\
                 fn take(v: Int | Str) [] -> [] Char { return 'p' }\n\
                 fn take(v: Int) [] -> [] Str { return \"int\" }\n";
    // A plain `Int` reaches the arm.
    let src = format!("{decls}{}", probe("    let picked: Str = take(3)"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // A value the caller only knows as `Int | Str` cannot: specificity never
    // exceeds what the caller knows.
    let src = format!(
        "{decls}{}",
        probe("    let v: Int | Str = 3\n    let picked: Char = take(v)")
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // ... and once narrowed, it does.
    let src = format!(
        "{decls}{}",
        probe(
            "    let v: Int | Str = 3\n    if v is Int {\n        \
             let picked: Str = take(v)\n    }"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// `T` beats `T?` for a non-`None` argument — the optional is just a union
/// with a `None` arm, so this needs no rule of its own.
#[test]
fn a_plain_type_beats_an_optional() {
    let decls = "fn take(v: Int?) [] -> [] Bool { return true }\n\
                 fn take(v: Int) [] -> [] Str { return \"int\" }\n";
    let src = format!("{decls}{}", probe("    let picked: Str = take(3)"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// `Any` is the broadest type there is, so it is the last resort.
#[test]
fn any_is_the_least_specific_parameter() {
    let decls = "fn take(v: Any) [] -> [] Bool { return true }\n\
                 fn take(v: Int) [] -> [] Str { return \"int\" }\n";
    let src = format!("{decls}{}", probe("    let picked: Str = take(3)"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // And it really does accept everything (it is the top type, not a
    // nominal type that happens to be called `Any`).
    let src = format!(
        "{}{}",
        "fn take(v: Any) [] -> [] Bool { return true }\n",
        probe("    let picked: Bool = take(\"text\")")
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// **More qualifiers say more**, and the *kind* of qualifier never ranks:
/// two candidates with one qualifier each are unrankable, which is an
/// ambiguity the caller settles with a rename.
#[test]
fn qualifier_sets_rank_by_inclusion() {
    let quals = "qualifier Even of Int with Small {\n    \
                 fn qualifies(n: Int) -> Bool { return true }\n}\n\
                 qualifier Small of Int with Even {\n    \
                 fn qualifies(n: Int) -> Bool { return true }\n}\n";
    // A superset wins.
    let decls = format!(
        "{quals}fn label(n: Even Int) [] -> [n] Bool {{ return true }}\n\
         fn label(n: Even Small Int) [] -> [n] Str {{ return \"both\" }}\n"
    );
    let src = format!(
        "{decls}{}",
        probe(
            "    let n = 4\n    if n is Even && n is Small {\n        \
             let picked: Str = label(n)\n    }"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // Different single qualifiers do not.
    let decls = format!(
        "{quals}fn label(n: Even Int) [] -> [n] Bool {{ return true }}\n\
         fn label(n: Small Int) [] -> [n] Str {{ return \"small\" }}\n"
    );
    let src = format!(
        "{decls}{}",
        probe(
            "    let n = 4\n    if n is Even && n is Small {\n        \
             let picked = label(n)\n    }"
        )
    );
    let msgs = messages(&src);
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(
        msgs[0].contains("ambiguous call to `label(Even Small Int)`")
            && msgs[0].contains("rename fn"),
        "{}",
        msgs[0]
    );
}

/// [fn-overload-rank] With the slots otherwise equal, a **fixed** parameter
/// list beats a variadic one — which is what lets an "empty" case be an
/// overload rather than a special form.
#[test]
fn a_fixed_parameter_list_beats_a_variadic_one() {
    let decls = "fn make() [] -> [] Str { return \"empty\" }\n\
                 fn make(...rest: Int[]) [] -> [] Bool { return true }\n";
    let src = format!("{decls}{}", probe("    let picked: Str = make()"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // With arguments, only the variadic one fits.
    let src = format!("{decls}{}", probe("    let picked: Bool = make(1, 2)"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// Ranking is **per argument slot**: two candidates that each win one slot
/// are an ambiguity, not a sum to be totted up.
#[test]
fn one_slot_never_pays_for_another() {
    let decls = "fn mix<T>(a: T, b: Int) [] -> [] Bool { return true }\n\
                 fn mix<T>(a: Int, b: T) [] -> [] Bool { return true }\n";
    let src = format!("{decls}{}", probe("    let x = mix(1, 2)"));
    let msgs = messages(&src);
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(
        msgs[0].contains("ambiguous call to `mix(Int, Int)`")
            && msgs[0].contains("`mix(T, Int)`")
            && msgs[0].contains("`mix(Int, T)`")
            && msgs[0].contains("rename fn"),
        "{}",
        msgs[0]
    );
}

/// [type-unknown-lenient] An un-inferred argument fits every candidate, so
/// it must not produce an ambiguity of its own.
#[test]
fn an_un_inferred_argument_produces_no_ambiguity() {
    let decls = "fn mix<T>(a: T, b: Int) [] -> [] Bool { return true }\n\
                 fn mix<T>(a: Int, b: T) [] -> [] Bool { return true }\n";
    let src = format!("{decls}{}", probe("    let x = mix(nope(), 2)"));
    let msgs = messages(&src);
    assert_eq!(msgs.len(), 1, "expected only the unresolved-call error: {msgs:?}");
    assert!(msgs[0].contains("no function named `nope`"), "{}", msgs[0]);
}

// ===== [fn-overload-scope] the visibility ladder =====

/// A module's own overload beats `core`'s — the defect this decision fixed:
/// a program that declared its own `size(List<T>)` had its calls silently
/// routed to std's.
#[test]
fn this_module_beats_core() {
    let src = format!(
        "{}{}",
        "fn size<T>(list: List<T>) [] -> [list] Str { return \"mine\" }\n",
        probe("    let xs = of_list(1, 2)\n    let picked: Str = size(xs)")
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    assert!(warnings(&src).is_empty(), "no warning: same signature shape");
}

/// An **import** beats `core` too, and this module beats the import.
#[test]
fn the_ladder_runs_core_import_module() {
    let lib = "fn describe(v: Int) [] -> [] Bool { return true }\n";
    let main_import = "import lib.describe\n\n";
    // core has none of these, so the import wins on its own.
    let src = format!(
        "{main_import}{}",
        probe("    let picked: Bool = describe(1)")
    );
    assert!(
        messages_files(&[("main.sv", &src), ("lib.sv", lib)]).is_empty(),
        "{:?}",
        messages_files(&[("main.sv", &src), ("lib.sv", lib)])
    );
    // With an own-module declaration of the same shape, this module wins.
    let src = format!(
        "{main_import}fn describe(v: Int) [] -> [] Str {{ return \"mine\" }}\n{}",
        probe("    let picked: Str = describe(1)")
    );
    let msgs = messages_files(&[("main.sv", &src), ("lib.sv", lib)]);
    assert!(msgs.is_empty(), "{msgs:?}");
}

/// Scope beats signature — deliberately — and the call gets a **warning**
/// naming the more specific candidate it passed over, plus both `@` forms.
#[test]
fn scope_beats_signature_with_a_warning() {
    let src = format!(
        "{}{}",
        "fn size(v: Any) [] -> [] Str { return \"mine\" }\n",
        probe("    let s = \"abc\"\n    let picked: Str = size(s)")
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    let warns = warnings(&src);
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(
        warns[0].contains("resolves to `size(Any)` from `main`")
            && warns[0].contains("`size(Str)` from `core.string`")
            && warns[0].contains("size@main(...)")
            && warns[0].contains("size@core.string(...)"),
        "{}",
        warns[0]
    );
}

// ===== [fn-overload-at] naming the module =====

/// `@module` reaches past scope precedence, and silences the warning.
#[test]
fn at_module_picks_that_modules_overload() {
    let src = format!(
        "{}{}",
        "fn size(v: Any) [] -> [] Str { return \"mine\" }\n",
        probe(
            "    let s = \"abc\"\n    let core: Int = size@core.string(s)\n    \
             let mine: Str = size@main(s)"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    assert!(warnings(&src).is_empty(), "an explicit `@` is the confirmation");
}

/// Naming a module that declares no fitting overload is an error listing the
/// ones that do — not a silent fallback.
#[test]
fn at_module_without_a_fitting_overload_is_an_error() {
    let src = probe("    let s = \"abc\"\n    let n = size@core.list(s)");
    let msgs = messages(&src);
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(
        msgs[0].contains("no overload of `size(Str)` is declared in module `core.list`")
            && msgs[0].contains("core.string"),
        "{}",
        msgs[0]
    );
}

/// `@` is the way out of a **shadowed** name: a local of the same name hides
/// every function, and nothing else could reach one.
#[test]
fn at_module_reaches_past_a_local_of_the_same_name() {
    let src = format!(
        "{}{}",
        "fn describe(v: Int) [] -> [] Str { return \"fn\" }\n",
        probe(
            "    let describe = \"a string\"\n    \
             let picked: Str = describe@main(1)"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// It works in dot form too, since `@` attaches to the *name*.
#[test]
fn at_module_works_in_dot_notation() {
    let src = format!(
        "{}{}",
        "fn size<T>(list: List<T>) [] -> [list] Str { return \"mine\" }\n",
        probe(
            "    let xs = of_list(1, 2)\n    let mine: Str = xs.size()\n    \
             let core: Int = xs.size@core.list()"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

// ===== [fn-value-select] passing a function by name =====

/// The **expected fn type** selects the overload — before this, an
/// overloaded name resolved to whichever was declared first and then failed
/// to match wherever it was going.
#[test]
fn a_fn_value_is_selected_by_the_expected_type() {
    let decls = "fn tag(v: Int) [] -> [v] Str { return \"int\" }\n\
                 fn tag(v: Str) [] -> [v] Str { return \"str\" }\n\
                 fn apply(f: (Str) -> Str, s: Str) [] -> [f, s] Str { return f(s) }\n";
    let src = format!("{decls}{}", probe("    let out = apply(tag, \"x\")"));
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// With nothing to fit, an overloaded name is an ambiguity naming both
/// remedies.
#[test]
fn an_overloaded_fn_value_with_no_expectation_is_an_error() {
    let decls = "fn tag(v: Int) [] -> [v] Str { return \"int\" }\n\
                 fn tag(v: Str) [] -> [v] Str { return \"str\" }\n";
    let src = format!("{decls}{}", probe("    let f = tag"));
    let msgs = messages(&src);
    assert!(
        msgs.iter().any(|m| m.contains("`tag` is overloaded")
            && m.contains("rename fn")),
        "{msgs:?}"
    );
}

// ===== [fn-rename] a name of its own =====

/// A rename settles an ambiguity the ranking cannot: the renamed overload
/// answers *only* to the new name, so the old name is unambiguous again.
#[test]
fn a_rename_settles_an_ambiguity() {
    let decls = "qualifier Even of Int with Small {\n    \
                 fn qualifies(n: Int) -> Bool { return true }\n}\n\
                 qualifier Small of Int with Even {\n    \
                 fn qualifies(n: Int) -> Bool { return true }\n}\n\
                 fn label(n: Even Int) [] -> [n] Bool { return true }\n\
                 fn label(n: Small Int) [] -> [n] Str { return \"small\" }\n\
                 rename fn label_small = label(n: Small Int)\n";
    let src = format!(
        "{decls}{}",
        probe(
            "    let n = 4\n    if n is Even && n is Small {\n        \
             let a: Bool = label(n)\n        let b: Str = label_small(n)\n    }"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A rename is scoped: a statement-level one is in force to the end of its
/// block, and gone after it.
#[test]
fn a_rename_is_scoped_to_its_block() {
    let decls = "fn tag(v: Int) [] -> [] Str { return \"int\" }\n";
    let src = format!(
        "{decls}{}",
        probe(
            "    if true {\n        rename fn tag_int = tag(v: Int)\n        \
             let a: Str = tag_int(1)\n    }\n    let b = tag_int(2)"
        )
    );
    let msgs = messages(&src);
    assert!(
        msgs.iter().any(|m| m.contains("no function named `tag_int`")),
        "{msgs:?}"
    );
}

/// The new name must be free — a rename removes an ambiguity, so adding one
/// would defeat it — and the parameter list must name an overload exactly.
#[test]
fn rename_declaration_errors() {
    let decls = "fn tag(v: Int) [] -> [] Str { return \"int\" }\n";
    let taken = format!("{decls}rename fn tag = tag(v: Int)\n");
    assert!(
        messages(&taken)
            .iter()
            .any(|m| m.contains("already a function in scope")),
        "{:?}",
        messages(&taken)
    );
    let no_match = format!("{decls}rename fn tag2 = tag(v: Str)\n");
    assert!(
        messages(&no_match)
            .iter()
            .any(|m| m.contains("names no `tag` in scope")),
        "{:?}",
        messages(&no_match)
    );
    let unknown = "rename fn nope2 = nope(v: Int)\n".to_string();
    assert!(
        messages(&unknown)
            .iter()
            .any(|m| m.contains("nothing to rename")),
        "{:?}",
        messages(&unknown)
    );
}

/// A renamed name already means one declaration, so `@module` on it is an
/// error rather than a redundancy.
#[test]
fn a_renamed_name_takes_no_module_selector() {
    let src = format!(
        "{}{}",
        "rename fn list_size = size<T>(list: List<T>)\n",
        probe("    let xs = of_list(1, 2)\n    let n = list_size@core.list(xs)")
    );
    assert!(
        messages(&src)
            .iter()
            .any(|m| m.contains("is a renamed overload")),
        "{:?}",
        messages(&src)
    );
}

// ===== [fn-overload-duplicate] =====

/// Two declarations with the same parameter *types* are duplicates, whatever
/// their parameter names or return types say — nothing could tell them
/// apart at a call.
#[test]
fn identical_parameter_types_are_a_duplicate() {
    let src = "fn f(n: Int) [] -> [] Str { return \"a\" }\n\
               fn f(other: Int) [] -> [] Int { return 1 }\n";
    let msgs = messages(src);
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(
        msgs[0].contains("duplicate fn `f(Int)`")
            && msgs[0].contains("Parameter names and return types take no part"),
        "{}",
        msgs[0]
    );
    // Differing types are an ordinary overload set.
    let ok = "fn f(n: Int) [] -> [] Int { return 1 }\n\
              fn f(s: Str) [] -> [] Int { return 2 }\n";
    assert!(messages(ok).is_empty(), "{:?}", messages(ok));
}

fn messages_files(files: &[(&str, &str)]) -> Vec<String> {
    checked_files(files)
        .errors
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}
