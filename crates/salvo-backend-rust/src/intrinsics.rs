//! Rust lowerings for the standard library's `intrinsic` declarations
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
//! Argument conventions match what the templates relied on
//! [rs-borrows]: a *place* argument is spliced raw so a method-style
//! lowering borrows it natively (`list.push(..)`), while a variadic tail
//! is spliced owned because it lands inside a constructor (`vec![..]`).
//! The caller decides which is which.

/// The Rust lowering of a call to an `intrinsic fn` [intrinsic-fn].
///
/// `name` is the declaration's name and `recv` the base type name of its
/// first parameter (`type_base_name` conventions, so an array is `[]`) —
/// together they identify the declaration the checker resolved, which is
/// what distinguishes std's overloads: `size(Str)`, `size(List<T>)` and
/// `size(T[])` are three different lowerings.
///
/// `None` means this backend has no lowering, which the caller reports.
/// `copy` and `discard` are absent deliberately: they dispatch on the
/// argument's *type* rather than the declaration, so the emitter lowers
/// them itself ([rs-copy], [linear-discard]).
pub fn fn_call(
    name: &str,
    recv: Option<&str>,
    args: &[String],
    type_args: &[String],
) -> Option<String> {
    // Unlike Kotlin, rustc infers a `vec![]`'s element type from later
    // use, so pinning it here would churn the output for nothing.
    let _ = type_args;
    let a = |i: usize| args.get(i).map(String::as_str).unwrap_or("todo!()");
    Some(match (name, recv) {
        // core.list ------------------------------------------------------
        // `List<T>` and `Mut List<T>` are both `Vec<T>`: Rust expresses
        // mutability through the binding and the reference, not through a
        // second type [type-canbe-mut].
        ("list", Some("[]")) | ("mutable_list", Some("[]")) => {
            format!("vec![{}]", args.join(", "))
        }
        // `T?` is physical here, so an out-of-range index must produce
        // `None` rather than panic — and the element is cloned because no
        // emitted signature returns a reference [rs-borrows].
        ("get", Some("List")) | ("get", Some("[]")) => {
            format!("{}.get(({}) as usize).cloned()", a(0), a(1))
        }
        ("add", Some("List")) => format!("{}.push({})", a(0), a(1)),
        // `first` is a derived return (`ReadOnly[from: list]`), so it
        // borrows rather than clones [readonly-return].
        ("first", Some("List")) | ("first", Some("[]")) => format!("{}.first()", a(0)),
        ("size", Some("List")) | ("size", Some("[]")) => {
            format!("({}.len() as i32)", a(0))
        }
        // Iterators are eager: `Iter<T>` is `Vec<T>` [rs-iter-vec].
        ("iter", Some("List")) | ("iter", Some("[]")) => format!("{}.clone()", a(0)),

        // core.string ----------------------------------------------------
        // Characters, not bytes: `Str` is a `String`, whose `len()` counts
        // UTF-8 bytes, which is not what Salvo's `size` means.
        ("size", Some("Str")) => format!("({}.chars().count() as i32)", a(0)),
        ("char_at", Some("Str")) => {
            format!("{}.chars().nth(({}) as usize)", a(0), a(1))
        }

        _ => return None,
    })
}

/// The Rust type an `intrinsic type` maps to [backend-intrinsic].
///
/// There is deliberately no `Mut` variant, unlike Kotlin's table:
/// mutability lives in the binding and the reference here, so `Mut`
/// erases [type-canbe-mut]. `Any` is absent because the backend has no
/// mapping for it and says so at the reference [backend-never-wrong].
pub fn type_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "Str" => "String",
        "Int" => "i32",
        "Long" => "i64",
        "Float" => "f32",
        "Double" => "f64",
        "Bool" => "bool",
        "Char" => "char",
        "Byte" => "u8",
        "None" => "()",
        // Only reachable in dead positions.
        "Nothing" => "()",
        // Iterators are eager [rs-iter-vec].
        "Iter" => "Vec",
        "List" => "Vec",
        _ => return None,
    })
}

/// The body of an `intrinsic handler`'s member [backend-intrinsic]: the
/// statements (or, for a value-returning member, the expression) that
/// implement it, with the member's own parameter names in scope.
///
/// Paths are absolute so the emitted file needs no `use` items.
pub fn handler_member(handler: &str, member: &str, params: &[String]) -> Option<String> {
    let p = |i: usize| params.get(i).map(String::as_str).unwrap_or("todo!()");
    Some(match (handler, member) {
        ("StdOutConsole", "print") => format!("print!(\"{{}}\", {})", p(0)),
        // No external crates: a splitmix64-style hash of the current time.
        // Not cryptographic — good enough for a default handler; seedable
        // and reproducible generators belong in dedicated handlers.
        ("DefaultRandom", "random") => "{
    let mut x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    (((x >> 11) as f64) / ((1u64 << 53) as f64))
}"
        .to_string(),
        _ => return None,
    })
}
