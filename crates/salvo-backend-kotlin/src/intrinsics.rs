//! Kotlin lowerings for the standard library's `intrinsic` declarations
//! [backend-intrinsic].
//!
//! Every `intrinsic type`, `intrinsic fn` and `intrinsic handler` std
//! declares must be lowered here; anything missing is a codegen error at
//! the reference site, never a pass-through [backend-never-wrong]. This
//! table replaced the `define` template files that used to live next to
//! the std modules: the text is the same, but it is now code the backend
//! owns, dispatched on the *checker-resolved* declaration rather than
//! matched by name and arity.
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
        // [backend-define-generics].
        ("list", Some("[]")) => format!("listOf<{}>({})", elem(), args.join(", ")),
        ("mutable_list", Some("[]")) => {
            format!("mutableListOf<{}>({})", elem(), args.join(", "))
        }
        ("get", Some("List")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("add", Some("List")) => format!("{}.add({})", a(0), a(1)),
        ("first", Some("List")) => format!("{}.firstOrNull()", a(0)),
        ("size", Some("List")) => format!("{}.size", a(0)),
        // `List<T>` is already `Iterable<T>`, which is what `Iter<T>` maps
        // to, so iterating a list is the identity.
        ("iter", Some("List")) => a(0).to_string(),

        // core.array -----------------------------------------------------
        // `T[]` maps to `Array<T>`, which is *not* `Iterable`, hence the
        // `asIterable()` the list case does not need [type-array].
        ("get", Some("[]")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("first", Some("[]")) => format!("{}.firstOrNull()", a(0)),
        ("size", Some("[]")) => format!("{}.size", a(0)),
        ("iter", Some("[]")) => format!("{}.asIterable()", a(0)),

        // core.string ----------------------------------------------------
        ("size", Some("Str")) => format!("{}.length", a(0)),
        ("char_at", Some("Str")) => format!("{}.getOrNull({})", a(0), a(1)),

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
        "Iter" => "Iterable",
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
        _ => None,
    }
}

/// The body of an `intrinsic handler`'s member [backend-intrinsic]: the
/// statements (or, for a value-returning member, the expression) that
/// implement it, with the member's own parameter names in scope.
///
/// Names are fully qualified so the emitted file needs no imports, and so
/// a member implementing `print` does not recurse into itself — the trap
/// the define templates documented as needing `kotlin.io.print`.
pub fn handler_member(handler: &str, member: &str, params: &[String]) -> Option<String> {
    let p = |i: usize| params.get(i).map(String::as_str).unwrap_or("TODO()");
    Some(match (handler, member) {
        ("StdOutConsole", "print") => format!("kotlin.io.print({})", p(0)),
        ("DefaultRandom", "random") => "kotlin.random.Random.nextDouble()".to_string(),
        _ => return None,
    })
}
