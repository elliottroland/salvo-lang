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

/// [fn-variadic] How a call supplied its variadic tail, which decides the
/// shape a constructor lowering wants.
///
/// The distinction is ownership, and it is the *caller's* to make: a lone
/// `...spread` of a local forwards that local's vector **borrowed**, so the
/// constructor has to clone; a tail that mixes plain arguments with a spread
/// is assembled into a fresh vector by the emitter and arrives **owned**, so
/// cloning it again would copy every element for nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spread {
    /// No `...spread`: `args` are the individual elements.
    None,
    /// One `...spread`, forwarded as a borrowed collection in `args[0]`.
    Borrowed,
    /// The whole tail, assembled by the emitter into an owned collection in
    /// `args[0]` — a mixed `list_of(first, ...rest)` call.
    Owned,
}

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
    spread: Spread,
) -> Option<String> {
    // Unlike Kotlin, rustc infers a `vec![]`'s element type from later
    // use, so pinning it here would churn the output for nothing.
    let _ = type_args;
    let a = |i: usize| args.get(i).map(String::as_str).unwrap_or("todo!()");
    let spread_any = spread != Spread::None;
    // The variadic tail as an **owned** `Vec`: already owned when the emitter
    // assembled it, cloned when it is a borrowed forward.
    let owned_vec = || match spread {
        Spread::Owned => a(0).to_string(),
        _ => format!("{}.clone()", a(0)),
    };
    // …and as an iterator of owned elements, which is what the set/map
    // constructors consume. `into_iter` on an owned vector moves its
    // elements; a borrowed one has to clone each.
    let owned_iter = || match spread {
        Spread::Owned => format!("{}.into_iter()", a(0)),
        _ => format!("{}.iter().cloned()", a(0)),
    };
    Some(match (name, recv) {
        // core.basic -----------------------------------------------------
        // [op-convert] The explicit numeric conversions. `as` matches the
        // Kotlin `toX()` semantics case by case: float→int truncates
        // toward zero and saturates (NaN → 0), i64→i32 keeps the low 32
        // bits, f64→f32 rounds — verified per pair when the set was added
        // (2026-09-14).
        //
        // The operand is parenthesized because `as` binds tighter than unary
        // minus: `-1 as u8` is `-(1 as u8)`, which rustc refuses outright on
        // an unsigned target and would silently mean the wrong thing on a
        // saturating one [backend-never-wrong].
        ("to_int", Some("Long" | "Double" | "Float")) => format!("(({}) as i32)", a(0)),
        ("to_long", Some("Int" | "Double" | "Float")) => format!("(({}) as i64)", a(0)),
        ("to_double", Some("Int" | "Long" | "Float")) => format!("(({}) as f64)", a(0)),
        ("to_float", Some("Int" | "Long" | "Double")) => format!("(({}) as f32)", a(0)),
        // [byte-value] A `Byte` is a `u8` here and a `UByte` on Kotlin, so
        // both conversions mean the same thing on both: `as u8` keeps the low
        // 8 bits (300 → 44, -1 → 255) like `toUByte()`, and `as i32` widens
        // into 0..255 like `toInt()`.
        //
        // The `as i32` in the middle is not redundant: rustc infers an
        // unsuffixed literal's type *from the cast*, so `(-1) as u8` makes
        // the literal a `u8` and is rejected. Naming the source type — which
        // is what the parameter declares — is what keeps `to_byte(-1)`
        // meaning 255 on both backends.
        ("to_byte", Some("Int")) => format!("((({}) as i32) as u8)", a(0)),
        ("to_int", Some("Byte")) => format!("(({}) as i32)", a(0)),
        // core.actor ---------------------------------------------------
        // [actor-replyto] [rs-actor] Answering a request: the token is
        // consumed, and the payload crosses the seam as the runtime's untyped
        // box. `Box::new` is where the sendability the checker proved becomes
        // Rust's `Send` bound on `SalvoMsg`.
        ("send", Some("Reply")) => format!("({}).send(Box::new({}))", a(0), a(1)),
        // [actor-spawn-expr] A pool is a scheduler index; `pool(n)` starts its
        // worker threads.
        ("pool", Some("Int")) => format!("crate::scheduler::salvo_pool((({}) as usize))", a(0)),
        // [waitfor-dedicated] `thread()` is a pool of one, and the only
        // placement the language types `Dedicated` — the qualifier is erased,
        // so what reaches here is a plain pool id.
        ("thread", None) => "crate::scheduler::salvo_thread()".to_string(),
        // [actor-watch] Registering a death watch hands the scheduler the
        // token *and* a builder for the `Exit` it will carry: the runtime
        // holds a reason string and cannot construct a Salvo struct, so the
        // watch site closes over the constructor instead.
        //
        // `Exit` is named unqualified, which is safe rather than lucky: the
        // file glob-imports every module whose names it uses, and a
        // `Reply<Exit>` cannot be *obtained* in a file where `Exit` means
        // something else (the annotation naming std's `Exit` would not
        // resolve), so a shadowing declaration and this emission never meet.
        ("watch", Some("Addr")) => format!(
            "crate::scheduler::salvo_watch({}, {}, |__reason| Box::new(Exit {{ reason: __reason }}))",
            a(0),
            a(1)
        ),
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
        // [col-sorted-list] `sort` uses Rust's own ordering, which for `String`
        // is byte-wise UTF-8 = code-point order — the language's rule
        // [col-sorted]. Kotlin has to be told; here it is free.
        ("sort", Some("List")) | ("mut_sort", Some("List")) => format!(
            "{{ let mut __v = {}.clone(); __v.sort(); __v }}",
            a(0)
        ),
        // A **lower bound**: the count of elements strictly below `elem`, so
        // an equal run is entered from the front and both backends land on the
        // same index.
        ("add_sorted", Some("List")) => format!(
            "{{ let __e = {}; let __at = {}.partition_point(|__x| __x < &__e); \
             {}.insert(__at, __e); }}",
            a(1),
            a(0),
            a(0)
        ),
        // Lower bound, then an equality test — not `Vec::binary_search`, which
        // may answer any index within an equal run [col-sorted-list].
        ("binary_search", Some("List")) => format!(
            "{{ let __e = {}; let __at = {}.partition_point(|__x| __x < &__e); \
             if __at < {}.len() && {}[__at] == __e {{ Some(__at as i32) }} \
             else {{ None }} }}",
            a(1),
            a(0),
            a(0),
            a(0)
        ),
        ("list_of", Some("[]")) | ("mut_list_of", Some("[]")) if spread_any => owned_vec(),
        ("list_of", Some("[]")) | ("mut_list_of", Some("[]")) => {
            format!("vec![{}]", args.join(", "))
        }
        // `T?` is physical here, so an out-of-range index must produce
        // `None` rather than panic. The element is **borrowed**
        // (`Option<&T>`): `get` declares `(proj[from: list] T)?`, and a
        // caller that needs ownership says `copy` [copy-opt-in]. (Until
        // 2026-09-11 every read cloned, even one that only tested `None`.)
        ("get", Some("List")) | ("get", Some("[]")) => {
            format!("{}.get(({}) as usize)", a(0), a(1))
        }
        ("add", Some("List")) => format!("{}.push({})", a(0), a(1)),
        // [linear-container] Take-by-move: the element leaves the list, so
        // nothing is cloned and nothing is left behind — which is what makes
        // these legal for a `List<Reply<T>>` where `get` (a borrow) is not.
        // A block, so the receiver is named once and the emptiness test and
        // the removal cannot disagree about it.
        // Methods on the generated `SalvoTake` trait rather than an inline
        // `&mut` block, for the reason `set(Mut Str)` is one: a `Mut List<T>`
        // parameter *is* a `&mut Vec<T>` and cannot be re-borrowed by an inline
        // `&mut`, and method syntax splices the receiver exactly once
        // [rs-borrows].
        ("remove_first", Some("List")) => format!("{}.salvo_remove_first()", a(0)),
        ("remove_at", Some("List")) => format!("{}.salvo_remove_at({})", a(0), a(1)),
        // [linear-container] The terminal: the list is consumed (so a state
        // field arrives here as a `mem::take`) and every element is handed to
        // the callback, which owns it.
        // `for_each` rather than a `for` loop: the callback's parameter type
        // is then inferred from the iterator's item through the `FnMut` bound,
        // where an immediately-applied closure literal (`(|r| …)(x)`) leaves
        // rustc with nothing to infer from (E0282).
        ("drain", Some("List")) => format!("{}.into_iter().for_each({})", a(0), a(1)),
        // `first` is a derived return (`proj[from: list]`), so it
        // borrows rather than clones [readonly-return].
        ("first", Some("List")) | ("first", Some("[]")) => format!("{}.first()", a(0)),
        ("size", Some("List")) | ("size", Some("[]")) => {
            format!("({}.len() as i32)", a(0))
        }
        // [interp-to-str] `[1, 2, 3]` — the format is fixed by the language,
        // not by the target's own collection formatting, so both backends
        // print the same text [backend-parity]. (Rust `Debug` would quote
        // strings and Kotlin's `toString` would not.)
        ("to_str", Some("List")) | ("to_str", Some("[]")) => format!(
            "format!(\"[{{}}]\", {}.iter().map(|__e| __e.to_string())\
             .collect::<Vec<_>>().join(\", \"))",
            a(0)
        ),

        // core.array -----------------------------------------------------
        // [fn-variadic] The variadic tail *is* the array, so the constructor
        // is its argument: a `...spread` arrives as the whole thing (cloned,
        // since a variadic position is untracked and the source stays
        // usable), and a literal list of arguments builds one.
        ("array_of", Some("[]")) if spread_any => owned_vec(),
        ("array_of", Some("[]")) => format!("vec![{}]", args.join(", ")),

        // [col-by] The generated constructors: the callback is called once
        // per index, in order. `(0..n)` yields `i32`, which is what the
        // callback's parameter is [rs-fn-param-convention].
        ("array_by", Some("Int")) | ("list_by", Some("Int")) | ("mut_list_by", Some("Int")) => {
            format!("(0..({})).map({}).collect::<Vec<_>>()", a(0), a(1))
        }
        ("set_by", Some("Int")) | ("mut_set_by", Some("Int")) => format!(
            "SalvoSet::from_elements((0..({})).map({}))",
            a(0),
            a(1)
        ),
        ("map_by", Some("Int")) | ("mut_map_by", Some("Int")) => format!(
            "SalvoMap::from_entries((0..({})).map({}))",
            a(0),
            a(1)
        ),

        // [col-convert] The converters.
        ("to_set", Some("List")) => {
            format!("SalvoSet::from_elements({}.iter().cloned())", a(0))
        }
        ("to_map", Some("List")) if args.len() == 1 => {
            format!("SalvoMap::from_entries({}.iter().cloned())", a(0))
        }
        ("to_map", Some("List")) => format!(
            "SalvoMap::from_entries({}.iter().map({}))",
            a(0),
            a(1)
        ),

        // core.seq -------------------------------------------------------
        // The `List` fast paths [fn-overload-rank] go through the
        // generated helpers [rs-seq]: a generic parameter is what gives the
        // callback an expected type, which is what closure inference needs
        // and what no inline shape could supply. `&{}[..]` reaches an owned
        // `Vec`, a `&Vec` and a `&mut Vec` alike.
        ("map", Some("List")) => format!("salvo_map(&{}[..], {})", a(0), a(1)),
        ("filter", Some("List")) => format!("salvo_filter(&{}[..], {})", a(0), a(1)),
        ("reduce", Some("List")) => {
            format!("salvo_reduce(&{}[..], {}, {})", a(0), a(1), a(2))
        }

        // core.set -------------------------------------------------------
        // [rs-collections] The ordered set from the runtime file. A
        // `...spread` arrives as the whole `Vec<T>`, so it is the element
        // source itself — cloned, since a variadic position is not tracked
        // by the flow analysis and the array stays usable afterwards (the
        // same reasoning as the list constructors above).
        ("set_of", Some("[]")) | ("mut_set_of", Some("[]")) if spread_any => {
            format!("SalvoSet::from_elements({})", owned_iter())
        }
        ("set_of", Some("[]")) | ("mut_set_of", Some("[]")) => {
            format!("SalvoSet::from_elements(vec![{}])", args.join(", "))
        }
        // The element is *moved* in, so it is spliced owned; `contains` and
        // `remove` only read theirs, so those borrow [rs-borrows].
        ("add", Some("Set")) => format!("{}.insert({})", a(0), a(1)),
        ("remove", Some("Set")) => format!("{}.remove(&{})", a(0), a(1)),
        ("contains", Some("Set")) => format!("{}.contains(&{})", a(0), a(1)),
        ("size", Some("Set")) => format!("({}.len() as i32)", a(0)),
        // [col-insertion-order] The runtime type's `iter` is insertion
        // order, so the list is that order. Cloned: the set keeps its
        // elements, the list gets its own.
        ("to_list", Some("Set")) => {
            format!("{}.iter().cloned().collect::<Vec<_>>()", a(0))
        }
        // [col-key-eligible] An owned read of a snapshot element — a clone
        // here, where Kotlin can share the reference.
        ("snapshot_at", Some("List")) => {
            format!("{}.get(({}) as usize).cloned()", a(0), a(1))
        }
        // [col-to-str] `{1, 2, 3}`, insertion-ordered — the runtime type's
        // `Display` is the language's format, so both backends agree
        // [backend-parity].
        ("to_str", Some("Set")) => format!("{}.to_string()", a(0)),

        // core.sorted ----------------------------------------------------
        // [col-sorted] `BTreeSet`/`BTreeMap` keep their keys in order, and
        // `from_iter` builds one from anything iterable.
        ("sorted_set_of", Some("[]")) | ("mut_sorted_set_of", Some("[]")) if spread_any => {
            format!(
                "{}.collect::<std::collections::BTreeSet<_>>()",
                owned_iter()
            )
        }
        ("sorted_set_of", Some("[]")) | ("mut_sorted_set_of", Some("[]")) => format!(
            "vec![{}].into_iter().collect::<std::collections::BTreeSet<_>>()",
            args.join(", ")
        ),
        ("add", Some("SortedSet")) => format!("{}.insert({})", a(0), a(1)),
        ("remove", Some("SortedSet")) => format!("{}.remove(&{})", a(0), a(1)),
        ("contains", Some("SortedSet")) => format!("{}.contains(&{})", a(0), a(1)),
        ("size", Some("SortedSet")) => format!("({}.len() as i32)", a(0)),
        // Cheap at either end of an ordered tree, which is the reason to use
        // one; cloned because the declaration hands back an owned `T?`.
        ("min", Some("SortedSet")) => format!("{}.iter().next().cloned()", a(0)),
        ("max", Some("SortedSet")) => format!("{}.iter().next_back().cloned()", a(0)),
        ("to_list", Some("SortedSet")) => {
            format!("{}.iter().cloned().collect::<Vec<_>>()", a(0))
        }
        // [col-to-str] The language's format, in key order — written out
        // rather than left to Rust's `Debug` [backend-parity].
        ("to_str", Some("SortedSet")) => format!(
            "format!(\"{{{{{{}}}}}}\", {}.iter().map(|__e| __e.to_string())\
             .collect::<Vec<_>>().join(\", \"))",
            a(0)
        ),

        ("sorted_map_of", Some("[]")) | ("mut_sorted_map_of", Some("[]")) if spread_any => {
            format!(
                "{}.collect::<std::collections::BTreeMap<_, _>>()",
                owned_iter()
            )
        }
        ("sorted_map_of", Some("[]")) | ("mut_sorted_map_of", Some("[]")) => format!(
            "vec![{}].into_iter().collect::<std::collections::BTreeMap<_, _>>()",
            args.join(", ")
        ),
        ("get", Some("SortedMap")) => format!("{}.get(&{})", a(0), a(1)),
        ("put", Some("SortedMap")) => format!("{}.insert({}, {})", a(0), a(1), a(2)),
        ("remove", Some("SortedMap")) => format!("{}.remove(&{})", a(0), a(1)),
        ("contains_key", Some("SortedMap")) => format!("{}.contains_key(&{})", a(0), a(1)),
        ("size", Some("SortedMap")) => format!("({}.len() as i32)", a(0)),
        ("first_key", Some("SortedMap")) => {
            format!("{}.keys().next().cloned()", a(0))
        }
        ("last_key", Some("SortedMap")) => {
            format!("{}.keys().next_back().cloned()", a(0))
        }
        ("keys", Some("SortedMap")) => {
            format!("{}.keys().cloned().collect::<Vec<_>>()", a(0))
        }
        ("to_str", Some("SortedMap")) => format!(
            "format!(\"{{{{{{}}}}}}\", {}.iter()\
             .map(|(__k, __v)| format!(\"{{}}: {{}}\", __k, __v))\
             .collect::<Vec<_>>().join(\", \"))",
            a(0)
        ),

        // core.map -------------------------------------------------------
        // [rs-collections] Entries are native 2-tuples on this backend
        // [type-tuple], which is exactly what `from_entries` consumes.
        ("map_of", Some("[]")) | ("mut_map_of", Some("[]")) if spread_any => {
            format!("SalvoMap::from_entries({})", owned_iter())
        }
        ("map_of", Some("[]")) | ("mut_map_of", Some("[]")) => {
            format!("SalvoMap::from_entries(vec![{}])", args.join(", "))
        }
        // `get` borrows the value out of the map (`Option<&V>`): the
        // declaration is `(proj[from: map] V)?`, so a caller who wants to
        // keep it says `copy` [copy-opt-in].
        ("get", Some("Map")) => format!("{}.get(&{})", a(0), a(1)),
        ("put", Some("Map")) => format!("{}.insert({}, {})", a(0), a(1), a(2)),
        // [linear-container] The displacing write: what was there comes back
        // instead of being dropped.
        ("replace", Some("Map")) => format!("{}.replace({}, {})", a(0), a(1), a(2)),
        // Hands the value back owned, which is what `-> V?` promises.
        ("remove", Some("Map")) => format!("{}.remove(&{})", a(0), a(1)),
        // [linear-container] The terminal: the map is consumed and its values
        // — never its keys, which were not obligations — are handed over one
        // at a time, in insertion order.
        ("drain", Some("Map")) => {
            format!("{}.into_values().into_iter().for_each({})", a(0), a(1))
        }
        ("contains_key", Some("Map")) => format!("{}.contains_key(&{})", a(0), a(1)),
        ("size", Some("Map")) => format!("({}.len() as i32)", a(0)),
        // [col-insertion-order] The runtime type's `keys` is insertion
        // order.
        ("keys", Some("Map")) => {
            format!("{}.keys().cloned().collect::<Vec<_>>()", a(0))
        }
        // [col-to-str] `{a: 1, b: 2}`.
        ("to_str", Some("Map")) => format!("{}.to_string()", a(0)),

        // core.string ----------------------------------------------------
        // [rs-mut-str] `Mut Str` and `Str` are both `String`: mutability
        // lives in the binding and the reference [type-canbe-mut], so
        // dropping `Mut` renders nothing at all [str-drop-mut].
        ("mut_str", Some("[]")) if spread_any => format!("{}.concat()", a(0)),
        ("mut_str", Some("[]")) if args.is_empty() => "String::new()".to_string(),
        // The parts are *read*, not stored, so they are borrowed: an
        // owned `vec![parts].concat()` would move a `Str` variable the
        // caller can still use (a variadic position is not tracked by the
        // flow analysis, so nothing would have warned).
        ("mut_str", Some("[]")) => {
            let parts: Vec<String> = args.iter().map(|p| format!("&{p}[..]")).collect();
            format!("[{}].concat()", parts.join(", "))
        }
        // Characters, not bytes: `Str` is a `String`, whose `len()` counts
        // UTF-8 bytes, which is not what Salvo's `size` means.
        ("size", Some("Str")) => format!("({}.chars().count() as i32)", a(0)),
        // …and `byte_size` *is* that byte count, which is what the
        // filesystem's offsets are in [fs-token].
        ("byte_size", Some("Str")) => format!("({}.len() as i64)", a(0)),
        ("char_at", Some("Str")) => {
            format!("{}.chars().nth(({}) as usize)", a(0), a(1))
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

        // core.bytes -----------------------------------------------------
        // [bytes-type] A `Bytes` is a `Vec<u8>`, so most of these are the
        // vector operation of the same name; the bounds-checked ones answer
        // `None`/do nothing rather than panicking, which is what the Salvo
        // declarations promise.
        ("bytes_of", Some("[]")) if spread_any => owned_vec(),
        ("bytes_of", Some("[]")) => format!("vec![{}]", args.join(", ")),
        ("mut_bytes", Some("[]")) if spread_any => {
            format!("{}.iter().flat_map(|__p| __p.iter().copied()).collect::<Vec<u8>>()", a(0))
        }
        ("mut_bytes", Some("[]")) if args.is_empty() => "Vec::<u8>::new()".to_string(),
        ("mut_bytes", Some("[]")) => format!(
            "[{}].iter().flat_map(|__p| __p.iter().copied()).collect::<Vec<u8>>()",
            args.join(", ")
        ),
        ("to_bytes", Some("Str")) => format!("{}.as_bytes().to_vec()", a(0)),
        // Strict by construction: `from_utf8` rejects, it does not replace.
        ("str_of_bytes", Some("Bytes")) => {
            format!("String::from_utf8({}.clone()).ok()", a(0))
        }
        ("size", Some("Bytes")) => format!("({}.len() as i32)", a(0)),
        // A negative index wraps to a huge `usize`, which `get` answers
        // `None` for — the same answer the declaration gives it.
        ("get", Some("Bytes")) => format!("{}.get(({}) as usize).copied()", a(0), a(1)),
        ("slice", Some("Bytes")) => format!(
            "{{ let __d = &{}; let __i = {}; let __j = {}; \
             if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() \
             {{ Some(__d[(__i as usize)..(__j as usize)].to_vec()) }} else {{ None }} }}",
            a(0),
            a(1),
            a(2)
        ),
        ("index_of", Some("Bytes")) => format!(
            "{}.iter().position(|__x| *__x == {}).map(|__i| __i as i32)",
            a(0),
            a(1)
        ),
        ("add", Some("Bytes")) => format!("{}.push({})", a(0), a(1)),
        ("append", Some("Bytes")) => format!("{}.extend_from_slice(&{}[..])", a(0), a(1)),
        // Out of range does nothing, so this is a statement rather than an
        // indexing assignment (which would panic).
        ("set", Some("Bytes")) => format!(
            "{{ let __i = {}; if __i >= 0 && (__i as usize) < {}.len() \
             {{ {}[(__i as usize)] = {}; }} }}",
            a(1),
            a(0),
            a(0),
            a(2)
        ),
        ("clear", Some("Bytes")) => format!("{}.clear()", a(0)),
        // [interp-to-str] `[0, 255, 200]`, the text a `List<Byte>` printed:
        // the payload type changed, what a program prints did not.
        ("to_str", Some("Bytes")) => format!(
            "format!(\"[{{}}]\", {}.iter().map(|__b| __b.to_string())\
             .collect::<Vec<String>>().join(\", \"))",
            a(0)
        ),
        ("to_hex", Some("Bytes")) => format!(
            "{}.iter().map(|__b| format!(\"{{:02x}}\", __b)).collect::<String>()",
            a(0)
        ),
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
        // [actor-types] [rs-actor] The scheduler's handles: an addr and a pool
        // are indices into it, a reply token is its own type. Their type
        // *arguments* are dropped by `emit_named_parts` — the effect a
        // `Addr<E>` serves is the checker's business, and the message enum is
        // what carries payload types into the runtime.
        "Addr" => "usize",
        "Pool" => "usize",
        "Reply" => "crate::scheduler::SalvoReply",
        // [bytes-type] `Bytes` and `Mut Bytes` are both `Vec<u8>`: mutability
        // lives in the binding and the reference here [type-canbe-mut], and a
        // byte buffer *is* a `Vec<u8>` — no runtime class, no boxing, which is
        // the asymmetry with Kotlin's shipped class [kt-bytes].
        "Bytes" => "Vec<u8>",
        "None" => "()",
        // Only reachable in dead positions.
        "Nothing" => "()",
        "List" => "Vec",
        // [col-insertion-order] Not `HashSet`/`HashMap`: those have no
        // iteration order to speak of (unspecified, and randomly seeded per
        // process), while Salvo's collections iterate in insertion order on
        // every backend. The runtime file supplies the ordered equivalents
        // with `LinkedHashMap` semantics [rs-collections].
        "Set" => "SalvoSet",
        "Map" => "SalvoMap",
        // [col-sorted] The standard library's ordered trees: their iteration
        // order *is* the key order, which is what these types promise, so no
        // runtime helper is needed here.
        "SortedSet" => "std::collections::BTreeSet",
        "SortedMap" => "std::collections::BTreeMap",
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
