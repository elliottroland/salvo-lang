//! Kotlin lowerings for the standard library's `intrinsic` declarations
//! [backend-intrinsic].
//!
//! Every `intrinsic type`, `intrinsic fn` and `intrinsic handler` std
//! declares must be lowered here; anything missing is a codegen error at
//! the reference site, never a pass-through [backend-never-wrong]. This
//! table replaced the per-backend `define` template files that used to live
//! next to the std modules: the emitted text is the same, but it is now code
//! the backend owns, dispatched on the *checker-resolved* declaration rather
//! than matched by name and arity.
//!
//! Interpolation conventions match what the templates relied on: `args`
//! holds the already-rendered argument code in declaration order (a
//! variadic tail is pre-joined by the caller), and `type_args` holds the
//! call's resolved type arguments ([call-type-args]) — Kotlin needs them
//! spelled out for the list constructors, which kotlinc cannot infer from
//! an empty argument list.

/// The Kotlin lowering of a call to an `intrinsic fn` [intrinsic-fn].
///
/// `name` is the declaration's name and `recv` the base type name of its
/// first parameter ([`crate::emit::type_base_name`] conventions, so an
/// array is `[]`) — together they identify the declaration the checker
/// resolved, which is what distinguishes std's overloads: `size(Str)`,
/// `size(List<T>)` and `size(T[])` are three different lowerings.
///
/// `None` means this backend has no lowering, which the caller reports.
/// `copy` and `discard` are absent deliberately: they dispatch on the
/// argument's *type* rather than the declaration, so the emitter lowers
/// them itself ([kt-copy], [linear-discard]).
pub fn fn_call(
    name: &str,
    recv: Option<&str>,
    args: &[String],
    type_args: &[String],
) -> Option<String> {
    let elem = || type_args.first().map(String::as_str).unwrap_or("Any");
    let a = |i: usize| args.get(i).map(String::as_str).unwrap_or("TODO()");
    Some(match (name, recv) {
        // core.list ------------------------------------------------------
        // The element type is spelled out: `listOf()` with no arguments
        // leaves kotlinc with nothing to infer from
        // [backend-intrinsic].
        // [fn-variadic] A `...spread` argument arrives as one `Array<T>`,
        // which Kotlin passes on with its own spread operator — the
        // alternative (`listOf(arr)`) is a *list of one array*, and kotlinc
        // says so, but only after the fact [backend-never-wrong].
        ("list", Some("[]")) => format!("listOf<{}>({})", elem(), args.join(", ")),
        ("mutable_list", Some("[]")) => {
            format!("mutableListOf<{}>({})", elem(), args.join(", "))
        }
        ("get", Some("List")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("add", Some("List")) => format!("{}.add({})", a(0), a(1)),
        ("first", Some("List")) => format!("{}.firstOrNull()", a(0)),
        ("size", Some("List")) => format!("{}.size", a(0)),
        // [interp-to-str] `[1, 2, 3]`, the language's format rather than the
        // JVM's — `joinToString` already produces exactly it, but writing it
        // out is what pins the parity with Rust [backend-parity].
        ("to_str", Some("List")) | ("to_str", Some("[]")) => {
            format!("{}.joinToString(\", \", \"[\", \"]\")", a(0))
        }

        // core.seq -------------------------------------------------------
        // [kt-seq] The `List` fast paths [fn-overload-rank]: Kotlin's own
        // collection operations, with the eager result made mutable because
        // `map`/`filter` return `Mut List<U>`.
        ("map", Some("List")) => format!("{}.map({}).toMutableList()", a(0), a(1)),
        ("filter", Some("List")) => {
            format!("{}.filter({}).toMutableList()", a(0), a(1))
        }
        ("reduce", Some("List")) => format!("{}.fold({}, {})", a(0), a(1), a(2)),

        // core.array -----------------------------------------------------
        // `T[]` maps to `Array<T>`, which is *not* `Iterable`, hence the
        // `asIterable()` the list case does not need [type-array].
        ("get", Some("[]")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("first", Some("[]")) => format!("{}.firstOrNull()", a(0)),
        ("size", Some("[]")) => format!("{}.size", a(0)),

        // core.string ----------------------------------------------------
        // [kt-mut-str] `Mut Str` is a `StringBuilder`, so construction is
        // asked for explicitly and a literal stays a `String`.
        ("mutable_str", Some("[]")) if args.is_empty() => "StringBuilder()".to_string(),
        // The parts are joined rather than appended one by one, so the same
        // lowering serves a `...spread` (which arrives as `*arr`).
        ("mutable_str", Some("[]")) => {
            format!("StringBuilder(listOf({}).joinToString(\"\"))", args.join(", "))
        }
        ("size", Some("Str")) => format!("{}.length", a(0)),
        ("char_at", Some("Str")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("split", Some("Str")) => format!("{}.split({})", a(0), a(1)),
        // No `-1` sentinel: absence is `null` [type-nullable].
        ("index_of", Some("Str")) => {
            format!("{}.indexOf({}).takeIf {{ it >= 0 }}", a(0), a(1))
        }
        ("contains", Some("Str")) => format!("{}.contains({})", a(0), a(1)),
        ("starts_with", Some("Str")) => format!("{}.startsWith({})", a(0), a(1)),
        ("ends_with", Some("Str")) => format!("{}.endsWith({})", a(0), a(1)),
        ("trim", Some("Str")) => format!("{}.trim()", a(0)),
        // `removePrefix`/`removeSuffix` are already the unchanged-when-absent
        // shape Salvo declares.
        ("trim_prefix", Some("Str")) => format!("{}.removePrefix({})", a(0), a(1)),
        ("trim_suffix", Some("Str")) => format!("{}.removeSuffix({})", a(0), a(1)),
        // Out of range is `null`, not an exception — and the arguments are
        // bound first so a call argument is evaluated once.
        ("substr", Some("Str")) => format!(
            "run {{ val __s = {}; val __i = {}; val __j = {}; \
             if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) \
             else null }}",
            a(0),
            a(1),
            a(2)
        ),
        ("to_upper", Some("Str")) => format!("{}.uppercase()", a(0)),
        ("to_lower", Some("Str")) => format!("{}.lowercase()", a(0)),
        ("join", Some("List")) => format!("{}.joinToString({})", a(0), a(1)),
        ("parse_int", Some("Str")) => format!("{}.toIntOrNull()", a(0)),
        // [kt-mut-str] The mutators take a `StringBuilder`.
        ("append", Some("Str")) => format!("{}.append({})", a(0), a(1)),
        // `setCharAt` throws out of range; Salvo's `set` does nothing.
        ("set", Some("Str")) => format!(
            "run {{ val __s = {}; val __i = {}; \
             if (__i >= 0 && __i < __s.length) __s.setCharAt(__i, {}) }}",
            a(0),
            a(1),
            a(2)
        ),
        ("clear", Some("Str")) => format!("{}.clear()", a(0)),

        _ => return None,
    })
}

/// The Kotlin type an `intrinsic type` maps to [backend-intrinsic], to
/// which the caller appends the rendered generic arguments.
pub fn type_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "Str" => "String",
        "Int" => "Int",
        "Long" => "Long",
        "Float" => "Float",
        "Double" => "Double",
        "Bool" => "Boolean",
        "Char" => "Char",
        "Byte" => "Byte",
        "None" => "Unit",
        "Any" => "Any",
        "Nothing" => "Nothing",
        "List" => "List",
        _ => return None,
    })
}

/// The Kotlin type a `Mut`-qualified use of an `intrinsic type` maps to,
/// where that is a *different type* rather than an erased qualifier
/// [type-canbe-mut] — the one place a qualifier survives erasure on this
/// backend. `None` means the plain mapping applies.
pub fn mut_type_name(name: &str) -> Option<&'static str> {
    match name {
        "List" => Some("MutableList"),
        // [kt-mut-str] A string under construction. Unlike `MutableList`,
        // this is *not* a subtype of its immutable form, which is what
        // makes dropping `Mut` a conversion [str-drop-mut].
        "Str" => Some("StringBuilder"),
        _ => None,
    }
}

/// [str-drop-mut] [kt-mut-str] The conversion a value needs when its `Mut`
/// qualifier is dropped — i.e. when a `Mut T` is used where `T` is
/// required. `None` means none is needed: `MutableList<T>` *is* a
/// `List<T>`, so that drop is free, and every qualifier other than `Mut`
/// erases entirely [qual-erasure]. `StringBuilder` is the exception the
/// mechanism exists for: it is not a `String`.
///
/// Suffix rather than a wrapper call, so the caller appends it to the
/// already-emitted code.
pub fn drop_mut_suffix(name: &str) -> Option<&'static str> {
    match name {
        "Str" => Some(".toString()"),
        _ => None,
    }
}

/// The body of an `intrinsic handler`'s member [backend-intrinsic]: the
/// statements (or, for a value-returning member, the expression) that
/// implement it, with the member's own parameter names in scope.
///
/// Names are fully qualified so the emitted file needs no imports, and so
/// a member implementing `print` does not recurse into itself — it would
/// otherwise resolve to std's own `println`.
pub fn handler_member(handler: &str, member: &str, params: &[String]) -> Option<String> {
    let p = |i: usize| params.get(i).map(String::as_str).unwrap_or("TODO()");
    Some(match (handler, member) {
        ("StdOutConsole", "print") => format!("kotlin.io.print({})", p(0)),
        ("DefaultRandom", "random") => "kotlin.random.Random.nextDouble()".to_string(),
        _ => return None,
    })
}
