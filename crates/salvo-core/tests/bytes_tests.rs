//! [bytes-type] [byte-value] `Bytes` — std's byte buffer — as the checker sees
//! it: an `intrinsic type` that opts into `Mut`, so it behaves like `Str`
//! rather than like a container of `Byte`.
//!
//! What is worth pinning here is the *rules*, not the lowerings (the backends'
//! e2e cases cover those): a buffer is written only through `Mut`, dropping
//! `Mut` reaches the read surface, a `Byte` is not a number to the operators,
//! and a buffer is not a hash key — the last one deliberately, since the
//! Kotlin buffer is a mutable object and a key that can change under a map is
//! a bug nobody sees.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`) — the shape of the real
/// `core.bytes`, trimmed to what the cases below name.
const STD_PRELUDE: &str = concat!(
    "intrinsic type Int\n",
    "intrinsic type Bool\n",
    "intrinsic type Byte\n",
    "intrinsic type Str canbe Mut\n",
    "intrinsic type Bytes canbe Mut\n",
    "intrinsic type Map<K, V> canbe Mut\n",
    "intrinsic fn to_byte(value: Int) [] -> Byte => value\n",
    "intrinsic fn to_int(value: Byte) [] -> Int => value\n",
    "intrinsic fn bytes_of(...elems: Byte[]) [] -> Bytes\n",
    "intrinsic fn mut_bytes(...parts: Bytes[]) [] -> Mut Bytes\n",
    "intrinsic fn to_bytes(str: Str) [] -> Bytes => str\n",
    "intrinsic fn str_of_bytes(data: Bytes) [] -> Str? => data\n",
    "intrinsic fn size(data: Bytes) [] -> Int => data\n",
    "intrinsic fn get(data: Bytes, index: Int) [] -> Byte? => data, index\n",
    "intrinsic fn slice(data: Bytes, start: Int, end: Int) [] -> Bytes? => data, start, end\n",
    "intrinsic fn add(data: Mut Bytes, byte: Byte) [] -> None => data: Mut, byte\n",
    "intrinsic fn append(data: Mut Bytes, more: Bytes) [] -> None => data: Mut, more\n",
    "intrinsic fn clear(data: Mut Bytes) [] -> None => data: Mut\n",
    "intrinsic fn to_hex(data: Bytes) [] -> Str => data\n",
    "intrinsic fn mut_map_of<K, V>() [] -> Mut Map<K, V>\n",
    "intrinsic fn put<K, V>(map: Mut Map<K, V>, key: K, value: V) [] -> None => map: Mut, !key, !value\n",
);

fn errors_of(src: &str) -> Vec<FileDiagnostic> {
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
        let (ast, parse_diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = parse_diagnostics.iter().filter(|d| d.is_error()).collect();
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
    let mut diags = check_program(&program, &resolution, &symbols).errors;
    diags.retain(|d| d.is_error());
    diags
}

fn messages(src: &str) -> Vec<String> {
    errors_of(src).iter().map(|d| d.message.clone()).collect()
}

fn assert_ok(src: &str) {
    let msgs = messages(src);
    assert!(msgs.is_empty(), "expected no errors, got {msgs:?}");
}

fn body(stmts: &str) -> String {
    format!("fn main() [] -> None {{\n{stmts}\n}}\n")
}

/// [bytes-type] The read surface takes a `Bytes`, and a `Mut Bytes` reaches it
/// by dropping its `Mut` — the `Str`/`Mut Str` rule, on the buffer
/// [str-drop-mut].
#[test]
fn a_buffer_reaches_the_read_surface_by_dropping_mut() {
    assert_ok(&body(
        "    let buf = mut_bytes()\n\
         \x20   add(buf, to_byte(1))\n\
         \x20   let n = size(buf)\n\
         \x20   let hex = to_hex(buf)\n\
         \x20   let first = get(buf, 0)",
    ));
}

/// [bytes-type] [type-canbe-mut] Writing needs the `Mut`: a plain `Bytes` is
/// read-only, which is what makes `copy` and the fs payloads safe to hand out.
#[test]
fn writing_a_plain_buffer_is_refused() {
    let msgs = messages(&body(
        "    let data = bytes_of(to_byte(1))\n\x20   add(data, to_byte(2))",
    ));
    assert!(
        msgs.iter().any(|m| m.contains("add") || m.contains("Mut")),
        "{msgs:?}"
    );
    let msgs = messages(&body(
        "    let data = bytes_of(to_byte(1))\n\x20   clear(data)",
    ));
    assert!(
        msgs.iter().any(|m| m.contains("clear") || m.contains("Mut")),
        "{msgs:?}"
    );
}

/// [bytes-type] The text bridge is strict, so it answers an optional; the
/// element reads are optional too, since an index may be outside the buffer
/// [type-nullable].
#[test]
fn the_optional_reads_must_be_narrowed() {
    assert_ok(&body(
        "    let data = to_bytes(\"hi\")\n\
         \x20   let text = str_of_bytes(data)\n\
         \x20   let n = 0\n\
         \x20   if text is Str {\n\
         \x20       n = 1\n\
         \x20   }\n\
         \x20   let part = slice(data, 0, 1)!\n\
         \x20   let b = get(data, 0)!",
    ));
    // `Str?` is not a `Str`: the surface says so rather than coercing.
    let msgs = messages(&body(
        "    let data = to_bytes(\"hi\")\n\x20   let text: Str = str_of_bytes(data)",
    ));
    assert!(!msgs.is_empty(), "expected the optional to be refused");
}

/// [byte-value] [op-arith] A `Byte` read out of a buffer is an octet, not a
/// number: arithmetic goes through `to_int`.
#[test]
fn bytes_hold_octets_not_numbers() {
    let msgs = messages(&body(
        "    let data = bytes_of(to_byte(1))\n\
         \x20   let b = get(data, 0)!\n\
         \x20   let sum = b + b",
    ));
    assert!(
        msgs.iter().any(|m| m.contains("needs numeric operands")),
        "{msgs:?}"
    );
    assert_ok(&body(
        "    let data = bytes_of(to_byte(1))\n\
         \x20   let b = get(data, 0)!\n\
         \x20   let sum = to_int(b) + 1",
    ));
}

/// [col-hashed-ordered] A buffer is **not** a hash key, deliberately: the
/// Kotlin `Bytes` is one mutable object [kt-bytes], so a key could change
/// under the map it is filed in. Refused where it is written, as every
/// unhashable key is.
#[test]
fn a_buffer_is_not_a_map_key() {
    let msgs = messages(&body(
        "    let m: Mut Map<Bytes, Int> = mut_map_of()\n\
         \x20   put(m, bytes_of(to_byte(1)), 1)",
    ));
    assert!(
        msgs.iter().any(|m| m.contains("hashable")),
        "expected the key to be refused as unhashable, got {msgs:?}"
    );
}
