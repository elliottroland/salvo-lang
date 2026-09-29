//! [effect-prereq] Effect prerequisites: `effect Fs [Streams]` — wherever
//! `Fs` is, `Streams` is too, bound separately (user decision 2026-09-29, R2:
//! a prerequisite, not inheritance).

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// The base types the checker needs, as a std file [intrinsic-std-only].
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export intrinsic type Long\nexport intrinsic type Any\nexport intrinsic type Never\n\
                           export intrinsic type Addr<E>\nexport struct Mailbox { capacity: Int }\n";

/// Parses, expands, resolves and checks; every diagnostic, both severities.
fn diags(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
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
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let mut out: Vec<FileDiagnostic> = expansion.diagnostics;
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let symbols = Symbols::collect(&program);
    out.extend(resolution.errors.clone());
    out.extend(check_program(&program, &resolution, &symbols).errors);
    out
}

fn errors(files: &[(&str, &str)]) -> Vec<String> {
    diags(files)
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const STREAMS: &str = "\
export effect Streams {
    fn next_id() -> Int
}

export handler Counter() of Streams {
    n: Int = 0
    fn next_id() -> Int {
        n = n + 1
        return n
    }
}
";

const FILES: &str = "\
import streams.Streams

export effect Files [Streams] {
    fn open(path: Str) -> Str => path
}

// A handler of `Files` reaches `Streams` without writing `[Streams]`.
export handler MemFiles() of Files {
    fn open(path: Str) -> Str => path {
        return \"${path}#${next_id()}\"
    }
}
";

// [effect-prereq] A fn declaring `[Files]` may call `Streams` members — in a
// module that never imports `Streams` — and a handler of `Files` reaches it as
// an unwritten dependency. A fn *type* expands the same way, so a fn declared
// `[Files]` passes where `(Str) [Files] -> Str` is expected.
#[test]
fn a_prerequisite_is_in_scope_wherever_its_effect_is() {
    let main = "\
import files.Files
import files.MemFiles
import streams.Counter

fn work(f: (Str) [Files] -> Str) [Files] -> Str {
    return \"${f(\"a\")} ${next_id()}\"
}

fn opener(p: Str) [Files] -> Str => p {
    return open(p)
}

fn main() [use] {
    use Counter()
    use MemFiles()
    let _a = opener(\"x\")
    let _b = work(opener)
}
";
    let errs = errors(&[("streams.sv", STREAMS), ("files.sv", FILES), ("main.sv", main)]);
    assert!(errs.is_empty(), "{errs:?}");
}

// [effect-prereq] Binding a handler of `Files` needs a `Streams` bound first,
// and the error says it is the effect's prerequisite — the reader looking at
// `MemFiles` will not find a dependency written there.
#[test]
fn a_use_without_the_prerequisite_names_it() {
    let main = "\
import files.MemFiles

fn main() [use] {
    use MemFiles()
}
";
    let errs = errors(&[("streams.sv", STREAMS), ("files.sv", FILES), ("main.sv", main)]);
    assert!(
        errs.iter().any(|e| e.contains("implements `Files`, which needs `Streams` in scope")
            && e.contains("[effect-prereq]")),
        "{errs:?}"
    );
}

// [effect-prereq] Transitive: `A [B]`, `B [C]` puts `C` in scope wherever `A`
// is; a handler implementing the prerequisite itself (`of A, B`) depends on
// nothing for it.
#[test]
fn prerequisites_are_transitive_and_a_handler_may_supply_its_own() {
    let src = "\
effect C {
    fn c() -> Int
}
effect B [C] {
    fn b() -> Int
}
effect A [B] {
    fn a() -> Int
}
handler HC() of C {
    fn c() -> Int { return 3 }
}
handler HAB() of A, B {
    fn a() -> Int { return c() + 1 }
    fn b() -> Int { return c() }
}

fn uses_c() [A] -> Int {
    return c()
}

fn main() [use] {
    use HC()
    use HAB()
    let _n = uses_c()
}
";
    let errs = errors(&[("main.sv", src)]);
    assert!(errs.is_empty(), "{errs:?}");
}

// [effect-prereq] The declaration's own rules: no cycle, no actor effect on
// either side, only plain named entries, and an exported effect's prerequisite
// exported too (every module naming the effect gets the prerequisite).
#[test]
fn malformed_prerequisites_are_refused_at_the_declaration() {
    let cycle = "effect A [B] {\n    fn a() -> Int\n}\neffect B [A] {\n    fn b() -> Int\n}\n";
    let errs = errors(&[("main.sv", cycle)]);
    assert!(errs.iter().any(|e| e.contains("form a cycle")), "{errs:?}");

    let actor = "\
actor effect Ping {
    send fn ping(n: Int) => !n
}
effect P {
    fn p() -> Int
}
actor effect Pong [P] {
    send fn pong(n: Int) => !n
}
effect Q [Ping] {
    fn q() -> Int
}
";
    let errs = errors(&[("main.sv", actor)]);
    assert!(errs.iter().any(|e| e.contains("`actor effect Pong` cannot have prerequisites")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("`Ping` is an actor effect")), "{errs:?}");

    let kinds = "effect P {\n    fn p() -> Int\n}\neffect A [use, any P] {\n    fn a() -> Int\n}\n";
    let errs = errors(&[("main.sv", kinds)]);
    assert!(errs.iter().any(|e| e.contains("`use` and `spawn` are capabilities")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("`any` has no meaning in a prerequisite")), "{errs:?}");

    let private = "effect P {\n    fn p() -> Int\n}\nexport effect A [P] {\n    fn a() -> Int\n}\n";
    let errs = errors(&[("main.sv", private)]);
    assert!(errs.iter().any(|e| e.contains("its prerequisite `P` is not")), "{errs:?}");

    let unknown = "effect A [Nowhere] {\n    fn a() -> Int\n}\n";
    let errs = errors(&[("main.sv", unknown)]);
    assert!(errs.iter().any(|e| e.contains("unknown effect `Nowhere`")), "{errs:?}");
}

// [effect-prereq] A generic effect's prerequisite takes the site's type
// arguments: `Log<T> [Sink<T>]` at `Log<Str>` implies `Sink<Str>`, so the
// implied member is typed at `Str`.
#[test]
fn a_generic_prerequisite_is_substituted() {
    let src = "\
effect Sink<T> {
    fn put(v: T) -> None => v
}
effect Log<T> [Sink<T>] {
    fn log(v: T) -> None => v
}
fn f() [Log<Str>] {
    put(\"x\")
}
fn g() [Log<Str>] {
    put(3)
}
";
    let errs = errors(&[("main.sv", src)]);
    assert_eq!(errs.len(), 1, "only `g` is wrong: {errs:?}");
}

// [effect-prereq] The implied name must mean the prerequisite where it is
// written: a module declaring its own `Streams` cannot name `Files` without
// saying which it means.
#[test]
fn a_name_clash_with_the_prerequisite_is_reported() {
    let main = "\
import files.Files

effect Streams {
    fn other() -> Int
}

fn f() [Files] -> Str {
    return open(\"x\")
}
";
    let errs = errors(&[("streams.sv", STREAMS), ("files.sv", FILES), ("main.sv", main)]);
    assert!(
        errs.iter().any(|e| e.contains("needs effect `streams.Streams` in scope")
            && e.contains("`main.Streams`")),
        "{errs:?}"
    );
}

// [effect-handler-deps] A handler's dependency list resolves in the
// handler's own file: `use HY()` in a module that never imports `X` is fine.
// Before 2026-09-29 the list was lowered in the *using* file's scope and this
// was "unknown effect `X`" — found with [effect-prereq], whose implied
// dependencies are never imported at a use site.
#[test]
fn a_handler_dependency_resolves_in_the_handlers_own_file() {
    let x = "export effect X {\n    fn x() -> Int\n}\nexport handler HX() of X {\n    fn x() -> Int { return 1 }\n}\n";
    let m = "import x.X\nexport effect Y {\n    fn y() -> Int\n}\nexport handler HY() [X] of Y {\n    fn y() -> Int { return x() }\n}\n";
    let main = "import m.HY\nimport x.HX\nfn main() [use] {\n    use HX()\n    use HY()\n}\n";
    let errs = errors(&[("x.sv", x), ("m.sv", m), ("main.sv", main)]);
    assert!(errs.is_empty(), "{errs:?}");
}
