//! Rust lowerings for the standard library's `intrinsic` declarations
//! [backend-intrinsic].
//!
//! Every `intrinsic type`, `intrinsic fn` and `intrinsic handler` std
//! declares must be lowered here; anything missing is a codegen error at
//! the reference site, never a pass-through [backend-never-wrong]. This
//! table replaced the per-backend `define` template files that used to live
//! next to
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
    spread: bool,
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
        // [fn-variadic] A `...spread` argument arrives as the whole
        // `Vec<T>`, so the constructor *is* that vector — wrapping it in a
        // `vec![]` would build a vector of one vector (which rustc rejects,
        // but only after the fact [backend-never-wrong]).
        // Cloned, not moved: Salvo does not track a variadic position, so
        // the spread array stays usable afterwards — which is what Kotlin's
        // `listOf(*arr)` does too.
        ("list", Some("[]")) | ("mutable_list", Some("[]")) if spread => {
            format!("{}.clone()", a(0))
        }
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
        // Iterators are lazy: `Iter<T>` is the generated factory type
        // [rs-iter-lazy].
        ("iter", Some("List")) | ("iter", Some("[]")) => {
            // [rs-iter-lazy] The elements are already there, so a pass is a
            // walk over a copy of them.
            format!("SalvoIter::from_vec({}.clone())", a(0))
        }

        // core.seq -------------------------------------------------------
        // The `List` fast paths [fn-overload-specific] go through the
        // generated helpers [rs-seq]: a generic parameter is what gives the
        // callback an expected type, which is what closure inference needs
        // and what no inline shape could supply. `&{}[..]` reaches an owned
        // `Vec`, a `&Vec` and a `&mut Vec` alike.
        ("map", Some("List")) => format!("salvo_map(&{}[..], {})", a(0), a(1)),
        ("filter", Some("List")) => format!("salvo_filter(&{}[..], {})", a(0), a(1)),
        ("reduce", Some("List")) => {
            format!("salvo_reduce(&{}[..], {}, {})", a(0), a(1), a(2))
        }
        // core.iterable --------------------------------------------------
        // [rs-iter-lazy] `Iter<T>` is a factory of passes, and the identity
        // that makes it satisfy `Iterable` clones the factory (a borrowed
        // one cannot be returned).
        ("iter", Some("Iter")) => format!("{}.clone()", a(0)),

        // core.string ----------------------------------------------------
        // [rs-mut-str] `Mut Str` and `Str` are both `String`: mutability
        // lives in the binding and the reference [type-canbe-mut], so
        // dropping `Mut` renders nothing at all [str-drop-mut].
        ("mutable_str", Some("[]")) if spread => format!("{}.concat()", a(0)),
        ("mutable_str", Some("[]")) if args.is_empty() => "String::new()".to_string(),
        // The parts are *read*, not stored, so they are borrowed: an
        // owned `vec![parts].concat()` would move a `Str` variable the
        // caller can still use (a variadic position is not tracked by the
        // flow analysis, so nothing would have warned).
        ("mutable_str", Some("[]")) => {
            let parts: Vec<String> = args.iter().map(|p| format!("&{p}[..]")).collect();
            format!("[{}].concat()", parts.join(", "))
        }
        // Characters, not bytes: `Str` is a `String`, whose `len()` counts
        // UTF-8 bytes, which is not what Salvo's `size` means.
        ("size", Some("Str")) => format!("({}.chars().count() as i32)", a(0)),
        ("char_at", Some("Str")) => {
            format!("{}.chars().nth(({}) as usize)", a(0), a(1))
        }
        // [rs-iter-lazy] The characters are already there, so a pass is a
        // walk over a copy of them.
        ("iter", Some("Str")) => {
            format!("SalvoIter::from_vec({}.chars().collect::<Vec<char>>())", a(0))
        }
        ("split", Some("Str")) => format!(
            "{}.split(&{}[..]).map(|__p| __p.to_string()).collect::<Vec<String>>()",
            a(0),
            a(1)
        ),
        // In *characters*, like every other index here — `find` answers in
        // bytes, so the prefix is re-counted.
        ("index_of", Some("Str")) => format!(
            "{{ let __s = &{}[..]; __s.find(&{}[..]).map(|__b| __s[..__b].chars().count() as i32) }}",
            a(0),
            a(1)
        ),
        ("contains", Some("Str")) => format!("{}.contains(&{}[..])", a(0), a(1)),
        ("starts_with", Some("Str")) => format!("{}.starts_with(&{}[..])", a(0), a(1)),
        ("ends_with", Some("Str")) => format!("{}.ends_with(&{}[..])", a(0), a(1)),
        ("trim", Some("Str")) => format!("{}.trim().to_string()", a(0)),
        // `strip_prefix` answers `None` when the affix is absent, where
        // Salvo's `trim_prefix` answers the string unchanged. The receiver
        // is bound once so a call argument is evaluated once.
        ("trim_prefix", Some("Str")) => format!(
            "{{ let __s = &{}[..]; __s.strip_prefix(&{}[..]).unwrap_or(__s).to_string() }}",
            a(0),
            a(1)
        ),
        ("trim_suffix", Some("Str")) => format!(
            "{{ let __s = &{}[..]; __s.strip_suffix(&{}[..]).unwrap_or(__s).to_string() }}",
            a(0),
            a(1)
        ),
        // Out of range is `None`, not a panic — and the bounds are compared
        // as `i32` first, since a negative index cast to `usize` is huge.
        ("substr", Some("Str")) => format!(
            "{{ let __s = &{}[..]; let __i = {}; let __j = {}; \
             let __n = __s.chars().count() as i32; \
             if __i >= 0 && __j >= __i && __j <= __n {{ \
             Some(__s.chars().skip(__i as usize).take((__j - __i) as usize)\
             .collect::<String>()) }} else {{ None }} }}",
            a(0),
            a(1),
            a(2)
        ),
        ("to_upper", Some("Str")) => format!("{}.to_uppercase()", a(0)),
        ("to_lower", Some("Str")) => format!("{}.to_lowercase()", a(0)),
        ("join", Some("List")) => format!("{}.join(&{}[..])", a(0), a(1)),
        ("parse_int", Some("Str")) => format!("{}.parse::<i32>().ok()", a(0)),
        ("append", Some("Str")) => format!("{}.push_str(&{}[..])", a(0), a(1)),
        // [rs-mut-str] A method on a generated trait, not an inline block:
        // replacing a character needs the string both read and written, and
        // a `&mut String` *parameter* cannot be re-borrowed by an inline
        // `&mut` (it is not a `mut` binding). Method syntax auto-refs both
        // shapes and splices the receiver once.
        ("set", Some("Str")) => {
            format!("{}.salvo_set({}, {})", a(0), a(1), a(2))
        }
        ("clear", Some("Str")) => format!("{}.clear()", a(0)),

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
        // Iterators are lazy [rs-iter-lazy]: a factory of passes, like
        // Kotlin's `Iterable<T>`.
        "Iter" => "SalvoIter",
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
