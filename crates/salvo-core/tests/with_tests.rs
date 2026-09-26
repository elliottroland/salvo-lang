//! [implicit-with] Implicits that are **filled together**: `=> eq with hash` on
//! a function or on a `params` group, and the one-source rule a call site obeys.
//!
//! The user's design (2026-09-26), from a defect: filling one slot of
//! `?Hashed<T>` and leaving the other to resolution builds a pair nobody
//! checked, and because a hash container buckets by `hash` first, the written
//! slot quietly does nothing rather than failing. `with` says the two are one
//! decision — symmetric, because it states that they must *agree* and agreement
//! has no direction, and transitive by the check, so a chain is one class.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] Enough of `std/core/compare.sv` to have a bound group,
/// plus a set to build with it.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export params Hashed<T> => eq with hash {\n",
    "    fn hash(value: T) -> Long\n",
    "    fn eq(a: T, b: T) -> Bool\n",
    "}\n",
    "export params Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic type Set<T> canbe Mut\n",
    "export intrinsic fn mut_set_of<T>(...elems: T[], ?Hashed<T>) [] -> Mut Set<T>(?hash, ?eq)\n",
    "export intrinsic fn add<T>(set: Mut Set<T>, elem: T) [] -> None => set: Mut, !elem\n",
    "export intrinsic fn size<T>(set: Set<T>) [] -> Int => set\n",
);

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/compare.sv",
        SourceSet::classify(Path::new("core/compare.sv")).unwrap(),
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
    let mut parse_errors: Vec<String> = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        parse_errors.extend(
            diagnostics
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.message.clone()),
        );
        modules.push(ast);
    }
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    check_program(&program, &resolution, &symbols)
        .errors
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const OWN_EQ: &str = "export fn all_same(a: Int, b: Int) -> Bool => a, b {\n    return true\n}\n";

/// A **written** member beside a **resolved** one: the defect this exists for.
#[test]
fn writing_one_member_of_a_bound_pair_is_refused() {
    let errs = errors(&format!(
        "{OWN_EQ}\n\
         fn main() -> None {{\n    \
         let s: Mut Set<Int> = mut_set_of(eq = all_same)\n    \
         add(s, 1)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("filled together")
            && e.contains("`eq` is written here")
            && e.contains("resolved by name")),
        "the mix must be refused, naming both sources: {errs:?}"
    );
    // …and the remedy is *writing them out*, which is what the author does next.
    assert!(
        errs.iter()
            .any(|e| e.contains("Write them out at the call") && e.contains("hash = …, eq = …")),
        "the diagnostic names the fix: {errs:?}"
    );
}

/// Writing **both** is the legal spelling, and the one the diagnostic asks for.
#[test]
fn writing_both_members_is_accepted() {
    let errs = errors(&format!(
        "{OWN_EQ}\n\
         fn main() -> None {{\n    \
         let s: Mut Set<Int> = mut_set_of(hash = hash, eq = all_same)\n    \
         add(s, 1)\n}}\n"
    ));
    assert!(errs.is_empty(), "a written pair is one source: {errs:?}");
}

/// Leaving the **whole** pair to resolution is untouched by the rule: the author
/// wrote nothing, so there is nothing to disagree.
#[test]
fn leaving_the_whole_pair_alone_is_accepted() {
    let errs = errors(
        "fn main() -> None {\n    \
         let s: Mut Set<Int> = mut_set_of()\n    \
         add(s, 1)\n}\n",
    );
    assert!(errs.is_empty(), "an unfilled group still resolves: {errs:?}");
}

/// **Forwarded** beside **resolved**, which arrives with nothing written at the
/// call at all — the case that decided the rule reads sources rather than
/// writes (user decision 2026-09-26).
#[test]
fn holding_half_the_pair_is_refused() {
    let errs = errors(
        "fn collect(a: Int, ?hash: (Int) -> Long) -> Set<Int> => !a {\n    \
         let s: Mut Set<Int> = mut_set_of()\n    \
         add(s, a)\n    \
         return s\n}\n\
         fn main() -> None {\n    \
         let n = size(collect(1, hash = hash))\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("filled together")
            && e.contains("forwarded from this function")
            && e.contains("resolved by name")),
        "a half-held pair must be refused: {errs:?}"
    );
    // Here writing them out is *impossible* — a container's identity must be a
    // name a type can carry [cmp-carry] — so the diagnostic offers the other
    // remedy first.
    assert!(
        errs.iter()
            .any(|e| e.contains("declare the rest of them too")
                && e.contains("cannot be written out")),
        "the diagnostic names the remedy that applies: {errs:?}"
    );
}

/// Holding the **whole** pair is the fix, and then every member is forwarded.
#[test]
fn holding_the_whole_pair_is_accepted() {
    let errs = errors(
        "fn collect(a: Int, ?Hashed<Int>) -> Set<Int> => !a {\n    \
         let s: Mut Set<Int> = mut_set_of()\n    \
         add(s, a)\n    \
         return s\n}\n\
         fn main() -> None {\n    \
         let n = size(collect(1))\n}\n",
    );
    assert!(errs.is_empty(), "a held pair is one source: {errs:?}");
}

/// Declaring the members **individually** takes them unbound: the binding is a
/// property of the *spread*, which is what makes "a function may add pairs and
/// never remove them" cost nothing (user decision 2026-09-26).
#[test]
fn members_declared_individually_are_not_bound() {
    let errs = errors(
        "fn pick(a: Int, ?hash: (Int) -> Long, ?eq: (Int, Int) -> Bool) -> Long => a {\n    \
         return hash(a)\n}\n\
         fn main() -> None {\n    \
         let n = pick(1, hash = hash)\n}\n",
    );
    assert!(
        errs.is_empty(),
        "individually declared members impose nothing: {errs:?}"
    );
}

/// A fn may **add** a pair of its own, over implicits nothing else binds.
#[test]
fn a_function_may_bind_a_pair_of_its_own() {
    let errs = errors(
        "fn pick(a: Int, ?hash: (Int) -> Long, ?eq: (Int, Int) -> Bool) -> Long \
         => a, hash with eq {\n    \
         return hash(a)\n}\n\
         fn main() -> None {\n    \
         let n = pick(1, hash = hash)\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("filled together") && e.contains("`hash` and `eq`")),
        "a fn's own clause binds the pair: {errs:?}"
    );
}

/// A **chain** is one component: `a with b with c` binds all three, so filling
/// one of them names the other two (user decision 2026-09-26).
#[test]
fn a_chain_binds_every_member() {
    let errs = errors(
        "fn pick(\n    \
         a: Int,\n    \
         ?hash: (Int) -> Long,\n    \
         ?eq: (Int, Int) -> Bool,\n    \
         ?cmp: (Int, Int) -> Int\n\
         ) -> Long => a, hash with eq with cmp {\n    \
         return hash(a)\n}\n\
         fn main() -> None {\n    \
         let n = pick(1, hash = hash)\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("filled together")
            && e.contains("`hash`")
            && e.contains("`eq`")
            && e.contains("`cmp`")),
        "a chain is one class, and the diagnostic names all of it: {errs:?}"
    );
}

/// `with` relates the `?name` positions a call fills, so a name that is not an
/// implicit parameter is a mistake — reported where it is written.
#[test]
fn with_over_a_non_implicit_is_refused() {
    let errs = errors(
        "fn pick(a: Int, ?hash: (Int) -> Long) -> Long => a, hash with a {\n    \
         return hash(a)\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("is not an implicit parameter of this signature")),
        "a `with` over an ordinary parameter must be refused: {errs:?}"
    );
}
