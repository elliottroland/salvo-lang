//! [wire-format] [noremote] [protocol-hash] What may cross a machine, as the
//! checker sees it — step ② of the network sequence (user decisions
//! 2026-09-26). Every type is serializable unless it is, or holds, something
//! `noremote`; `encode`/`decode` are refused at the call, naming what stops
//! them; and an actor protocol's canonical hash is a property of its shape,
//! not its names.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] A std-shaped prelude: the scalars, the containers the
/// predicate names, and `net`'s two intrinsics in a module of their own.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type Bytes canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn set_of<T>(...elems: T[]) [] -> Set<T>\n",
    "export noremote intrinsic type Pool\n",
    "export intrinsic fn pool(size: Int) [] -> Pool => size\n",
);

const STD_NET: &str = concat!(
    "export intrinsic fn encode<T>(value: T) [] -> Bytes => !value\n",
    "export intrinsic fn decode<T>(data: Bytes) [] -> T? => data\n",
);

fn program(src: &str) -> (Program, Vec<FileDiagnostic>) {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/net.sv",
        SourceSet::classify(Path::new("net.sv")).unwrap(),
        STD_NET.to_string(),
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
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    errors.extend(check_program(&program, &resolution, &symbols).errors);
    (program, errors)
}

fn messages(src: &str) -> Vec<String> {
    program(src).1.iter().map(|d| d.message.clone()).collect()
}

const IMPORT: &str = "import net\n\n";

/// [wire-format] The default: a struct of scalars, strings, lists, options
/// and unions has a wire form, and so do `encode` and `decode` of it.
#[test]
fn ordinary_data_encodes() {
    let errs = messages(&format!(
        "{IMPORT}struct Tag {{ name: Str }}\n\
         struct Pin {{ x: Int, label: Str?, tags: List<Tag>, kind: Tag | Int | None, raw: Bytes }}\n\
         fn main() -> None {{\n    \
         let p = Pin {{ x: 1, label: None, tags: [], kind: 2, raw: encode(3) }}\n    \
         let b = encode(p)\n    let _back = decode<Pin>(b)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [noremote] A `noremote` struct is refused at the `encode`, by name.
#[test]
fn a_noremote_struct_is_refused_at_encode() {
    let errs = messages(&format!(
        "{IMPORT}noremote struct Canvas {{ n: Int }}\n\
         fn main() -> None {{\n    let b = encode(Canvas {{ n: 1 }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`encode` needs a type with a wire form")
            && e.contains("`Canvas` is declared `noremote`")
            && e.contains("[noremote]")),
        "expected the noremote refusal, got {errs:?}"
    );
}

/// [noremote] Transitive: a struct holding a `noremote` field is refused, and
/// the message names the field that stops it.
#[test]
fn noremote_is_transitive_through_fields() {
    let errs = messages(&format!(
        "{IMPORT}noremote struct Canvas {{ n: Int }}\n\
         struct Frame {{ title: Str, canvas: Canvas }}\n\
         fn main() -> None {{\n    let b = encode(Frame {{ title: \"t\", canvas: Canvas {{ n: 1 }} }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Frame.canvas (Canvas)` is declared `noremote`")),
        "expected the field to be named, got {errs:?}"
    );
}

/// [noremote] std's process-local handles are `noremote` by declaration
/// (`Pool` here), so a struct holding one has no wire form.
#[test]
fn a_pool_has_no_wire_form() {
    let errs = messages(&format!(
        "{IMPORT}struct Placement {{ where: Pool }}\n\
         fn main() -> None {{\n    let b = encode(Placement {{ where: pool(2) }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Placement.where (Pool)` is declared `noremote`")),
        "expected Pool to block, got {errs:?}"
    );
}

/// [noremote] A function value has no wire form, by construction.
#[test]
fn a_function_value_has_no_wire_form() {
    let errs = messages(&format!(
        "{IMPORT}struct Rule {{ test: (Int) -> Bool }}\n\
         fn main() -> None {{\n    let b = encode(Rule {{ test: n -> n > 0 }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("holds a function value")),
        "expected the fn-value block, got {errs:?}"
    );
}

/// [wire-format] Step ②'s recorded cuts, each refused with its reason: a keyed
/// container (identity capabilities not yet carried), and a generic `T`
/// (the form is the instantiation's).
#[test]
fn keyed_containers_and_generic_parameters_are_refused_for_now() {
    let errs = messages(&format!(
        "{IMPORT}fn ship<T>(v: T) -> Bytes => !v {{\n    return encode(v)\n}}\n\
         fn main() -> None {{\n    let b = decode<Set<Int>>(encode(1))\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Set` is a keyed container")),
        "expected the keyed-container cut, got {errs:?}"
    );
    assert!(
        errs.iter().any(|e| e.contains("`T` is a generic parameter")),
        "expected the generic cut, got {errs:?}"
    );
}

/// [protocol-hash] The hash is over the protocol's *shape*: a struct renamed
/// hashes the same (names are not in the canonical form), a field reordered
/// does not (the encoding is positional), and a member added does not.
#[test]
fn protocol_hash_follows_shape_not_names() {
    fn hash_of(src: &str) -> String {
        let (prog, errs) = program(src);
        assert!(errs.is_empty(), "expected no errors, got {errs:?}");
        let symbols = Symbols::collect(&prog);
        let resolution = resolve(&prog);
        let checked = check_program(&prog, &resolution, &symbols);
        checked.protocol_hashes["Ledger"].clone()
    }
    let base = hash_of(
        "struct Entry { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close() }\n",
    );
    let renamed = hash_of(
        "struct Row { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Row) => !e\n send fn close() }\n",
    );
    let reordered = hash_of(
        "struct Entry { amount: Long, who: Str }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close() }\n",
    );
    let grown = hash_of(
        "struct Entry { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close()\n send fn audit() }\n",
    );
    assert_eq!(base.len(), 16, "sixteen hex digits: {base}");
    assert_eq!(base, renamed, "a renamed struct is the same protocol");
    assert_ne!(base, reordered, "a reordered field is a different protocol");
    assert_ne!(base, grown, "a new member is a different protocol");
    // And the constant is stable across runs and machines: FNV-1a 64 over a
    // fixed canonical string.
    assert_eq!(
        salvo_core::protocol_hash("record({Str,Long});close();"),
        base,
        "the canonical form is `member(types);` with structs expanded"
    );
}
