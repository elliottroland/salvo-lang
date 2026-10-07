//! The Kotlin backend's entry points and spelling helpers. The emitter
//! itself reads the IR (`ir_emit/`, [kt-ir]); this file is what the rest of
//! the crate shares: Kotlin names and packages, literal spelling, and the
//! runtime files a build ships [kt-runtime].

use std::collections::BTreeSet;

use salvo_core::{ModulePath, Program};

pub use salvo_backend::emit_util::*;

/// Emits Kotlin for every *reachable* module that produces code
/// [mod-used-only]. Returns the files or the accumulated codegen/type
/// errors, **dropping** any warnings: this is the shape the golden tests
/// want. The driver calls [`emit_program_reporting`], which hands them back
/// [qual-refn-ambiguous].
pub fn emit_program(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_reporting(program).map(|(files, _warnings)| files)
}

/// [`emit_program`] with the non-fatal diagnostics: `(files, warnings)`,
/// each warning rendered with its own `warning:` prefix and location
/// [diag-structured]. Two returns rather than one, because a warning must
/// not stop emission and must not be swallowed either.
pub fn emit_program_reporting(program: &Program) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    crate::ir_emit::emit_program_ir(program, false)
}

/// [platform-abi] [kt-abi] The declaration files of a platform root's host
/// project: every declaration the platform surface reaches, emitted exactly
/// as the build emits it, with the runtime under `salvo/`.
pub fn emit_abi(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    crate::ir_emit::emit_program_ir(program, true).map(|(files, _)| files)
}

/// [platform-tree] [kt-platform-host] `salvo platform generate`'s output:
/// the host implementation skeleton of every project module declaring
/// platform items.
pub fn platform_skeletons(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    crate::ir_emit::skeletons::platform_skeletons_ir(program)
}

fn kt_package_part(name: &str) -> String {
    if KOTLIN_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

pub(crate) fn kotlin_package(module: &ModulePath) -> String {
    let mut out = String::from("salvo");
    for part in &module.0 {
        out.push('.');
        out.push_str(&kt_package_part(part));
    }
    out
}

/// The generated throw signal [kt-throw-signal]: the JVM's unwinding *is*
/// the propagation, so `throw` throws and the innermost `try` catches. No
/// stack trace and no suppression bookkeeping — it is a control transfer,
/// not an error — and it is never visible in Salvo source, so nothing can
/// catch it by accident: `try` compiles to the only `catch` for it.
///
/// The `tag` is the Salvo type of the message as written at the throw site.
/// Rust wraps a message into the delimiter's arm at the *propagation* site,
/// which it has (`?` unwrapping a `ControlFlow`); the JVM has no such site
/// — the exception flies through untouched — so the arm is chosen at the
/// `catch`, and the tag is what tells it which one. Erasure-proof by
/// construction: it compares Salvo type names, not JVM classes.
/// Source in `runtime/throwsignal.kt`, included verbatim and compiled directly
/// by `runtime_tests.rs`.
pub(crate) fn generate_throw_file() -> String {
    include_str!("../runtime/throwsignal.kt").to_string()
}

/// Source in `runtime/compare.kt`, included verbatim and compiled directly
/// by `runtime_tests.rs`.
pub(crate) fn generate_compare_file() -> String {
    include_str!("../runtime/compare.kt").to_string()
}

/// [kt-bytes] The `Bytes` buffer class, which is `Bytes` *and* `Mut Bytes`
/// [bytes-type]: neither `List<UByte>` (a box per element) nor `UByteArray`
/// (fixed-size, and not a `List<T>`, so generic code cannot take one) is a
/// growable byte buffer on this backend.
///
/// Source in `runtime/bytes.kt`, included verbatim and compiled directly by
/// `runtime_tests.rs`.
/// [kt-actor] The actor scheduler asynchronous effect handlers run on —
/// the mirror of the Rust backend's, with identical decided semantics and
/// identical observable behaviour (asserted by the runtime tests). Emitted
/// only when a program spawns.
/// Source in `runtime/scheduler.kt`.
pub(crate) fn generate_scheduler_file() -> String {
    include_str!("../runtime/scheduler.kt").to_string()
}

/// [time-types] [kt-time] The two clock readings `time`'s effects are built
/// on, mirroring the Rust backend's `time.rs` number for number. Emitted only
/// when a program reads time. Source in `runtime/hosttime.kt`.
pub(crate) fn generate_time_file() -> String {
    include_str!("../runtime/hosttime.kt").to_string()
}

pub(crate) fn generate_bytes_file() -> String {
    include_str!("../runtime/bytes.kt").to_string()
}

/// [kt-host-abi] `UnionN` and its `UN_k` arms are part of the host ABI: host
/// code builds and tests them, so their shape is a contract.
pub(crate) fn generate_unions_file(sizes: &BTreeSet<usize>, wire: bool) -> String {
    let mut out = String::from(
        "// Generated by the Salvo compiler: sealed wrappers for union types.\npackage salvo\n",
    );
    for &n in sizes {
        let params: Vec<String> = (1..=n).map(|i| format!("out T{i}")).collect();
        let args: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
        // [kt-union-wrappers] The arms nest in the sealed interface
        // (`Union2.U1`), as Rust's are variants of its enum (`Union2::U1`),
        // each over the union's full parameter list.
        out.push_str(&format!(
            "\nsealed interface Union{n}<{}> {{\n    val value: Any?\n",
            params.join(", ")
        ));
        for i in 1..=n {
            out.push_str(&format!(
                "    data class U{i}<{}>(override val value: T{i}) : Union{n}<{}>\n",
                params.join(", "),
                args.join(", ")
            ));
        }
        out.push_str("}\n");
        // [wire-format] [kt-wire] The union's codec: one tag byte holding
        // the arm's declared index, then the arm — the Rust `impl __Wire for
        // UnionN`, as a class over the arms' codecs.
        if wire {
            let ctor: Vec<String> = (1..=n)
                .map(|i| format!("private val c{i}: WireCodec<T{i}>"))
                .collect();
            let mut enc = String::new();
            let mut dec = String::new();
            for i in 1..=n {
                enc.push_str(&format!(
                    "            is Union{n}.U{i} -> {{ out.u8({}); c{i}.enc(v.value, out) }}\n",
                    i - 1
                ));
                dec.push_str(&format!(
                    "            {} -> Union{n}.U{i}(c{i}.dec(inp))\n",
                    i - 1
                ));
            }
            out.push_str(&format!(
                "\nclass Union{n}Codec<{a}>({c}) : WireCodec<Union{n}<{a}>> {{\n    \
                 override fun enc(v: Union{n}<{a}>, out: WireOut) {{\n        when (v) {{\n{enc}        }}\n    }}\n    \
                 override fun dec(inp: WireIn): Union{n}<{a}> = when (inp.u8()) {{\n{dec}            \
                 else -> throw WireError()\n        }}\n}}\n",
                a = args.join(", "),
                c = ctor.join(", ")
            ));
        }
    }
    out
}

/// [kt-wire] The codec of `UnionN`, alone in its file: what
/// `generate_unions_file` appends when asked for the wire.
pub(crate) fn generate_union_codec_file(n: usize) -> String {
    let both = generate_unions_file(&BTreeSet::from([n]), true);
    let codec = &both[both.find(&format!("\nclass Union{n}Codec<")).unwrap_or(both.len())..];
    format!("// Generated by the Salvo compiler: the codec of `Union{n}` [kt-wire].\npackage salvo\n{codec}")
}

/// [kt-tuple-class] [type-tuple] The tuple classes Kotlin lacks: `Pair` and
/// `Triple` cover two and three, and everything past them is a **generated
/// data class** — one per arity the program actually names, in the `salvo`
/// package beside the union wrappers.
///
/// A `data class` is what makes them behave like tuples rather than like
/// objects: structural `equals`/`hashCode` (so a big tuple is a `Set` element
/// or a `Map` key on the same terms as a `Pair`) and `componentN` (so
/// `let (a, b, c, d) = t` destructures). Ordering goes through
/// Salvo's `eq`/`cmp`/`hash` for them are Salvo code
/// (`core.compare`, ROADMAP §0j step 8), so the classes carry nothing more.
///
/// The property names are `first`, `second`, `third` and then `v3`, `v4`, …:
/// keeping `Pair`/`Triple`'s three names is what lets one index rule serve
/// every arity [kt-tuple-component].
pub(crate) fn generate_tuples_file(sizes: &BTreeSet<usize>) -> String {
    let mut out = String::from(
        "// Generated by the Salvo compiler: the tuple types Kotlin does not \
         have [kt-tuple-class].\npackage salvo\n",
    );
    for &n in sizes {
        let params: Vec<String> = (1..=n).map(|i| format!("out T{i}")).collect();
        let args: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
        let fields: Vec<String> = (0..n)
            .map(|i| format!("val {}: T{}", tuple_field(i), i + 1))
            .collect();
        out.push_str(&format!(
            "\ndata class Tuple{n}<{}>(\n    {},\n)\n",
            params.join(", "),
            fields.join(",\n    ")
        ));
        let _ = &args;
    }
    out
}

/// [kt-tuple-component] The property a generated tuple gives position `i`.
pub(crate) fn tuple_field(i: usize) -> String {
    match i {
        0 => "first".to_string(),
        1 => "second".to_string(),
        2 => "third".to_string(),
        n => format!("v{n}"),
    }
}

/// [platform-tree] [kt-platform-host] The Kotlin package of a module's host
/// file: `salvo.platform.` plus the module path. A package of its own is
/// what keeps the host's facade class distinct from the generated module's
/// — both files are named after the module, so sharing a package would put
/// two `MainKt` classes on the classpath.
pub fn host_package(module: &ModulePath) -> String {
    let mut out = String::from("salvo.platform");
    for part in &module.0 {
        out.push('.');
        out.push_str(&kt_package_part(part));
    }
    out
}

/// Kotlin reserved words that need backtick-escaping as identifiers.
const KOTLIN_KEYWORDS: &[&str] = &[
    "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in",
    "interface", "is", "null", "object", "package", "return", "super", "this", "throw", "true",
    "try", "typealias", "typeof", "val", "var", "when", "while",
];

pub(crate) fn kt_ident(name: &str) -> String {
    let name = salvo_core::case::camel(salvo_core::typekey::plain(name));
    if KOTLIN_KEYWORDS.contains(&name.as_str()) {
        format!("`{name}`")
    } else {
        name
    }
}

pub(crate) fn escape_string(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '$' => out.push_str("\\$"),
            c => out.push(c),
        }
    }
    out
}

/// [type-literal] A literal type's value as a Kotlin literal.
pub(crate) fn kt_literal(lit: &salvo_syntax::ast::TypeLit) -> String {
    use salvo_syntax::ast::TypeLit;
    match lit {
        TypeLit::Str(v) => format!("{v:?}").replace('$', "\\$"),
        TypeLit::Int(v) => v.to_string(),
        TypeLit::Long(v) => format!("{v}L"),
        TypeLit::Bool(v) => v.to_string(),
    }
}
