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
    ordering: Option<&str>,
    // [cmp-carry] The identity as **function values** rather than marker
    // types: what a keyed container built inside a *generic* function is
    // given, where the identity is a capability the function was handed and
    // no type names it (2026-09-26). One string, already
    // comma-separated — `hash, eq` or a single `cmp` — spliced into the
    // runtime's `with_fns`/`with_cmp` entry points.
    keyed_values: Option<&str>,
    // [rs-loc] The call is a return-path forward inside a lending fn's
    // **locator variant**: the lowering answers *position data* instead
    // of a borrow. Only the lenders with a locator form answer; the rest
    // fall through to `None`, which the caller reports
    // [backend-never-wrong].
    loc_lend: bool,
) -> Option<String> {
    // Unlike Kotlin, rustc infers a `vec![]`'s element type from later
    // use, so pinning it here would churn the output for nothing.
    let _ = type_args;
    if loc_lend && !matches!((name, recv), ("get", Some("List")) | ("get", Some("[]"))) {
        // [rs-loc] No locator form for this intrinsic yet: `None` makes
        // the reference site report it, never a silently-read lowering.
        return None;
    }
    // [cmp-carry] The marker type naming the ordering a keyed container is kept
    // by, when this call *constructs* one: the emitter reads it off the call's
    // own type, since that is where the identity lives.
    let ord = || ordering.unwrap_or("HostOrd");
    let keyed = || ordering.unwrap_or("HostHash, HostEq");
    // [cmp-carry] One place per family that knows how a keyed container is
    // built, so the marker form and the function-value form cannot drift —
    // the shape the Kotlin intrinsics have always had (`set_ctor`/`map_ctor`).
    // The value form fills by insertion, which is what the runtime's
    // `from_elements` does anyway.
    let set_ctor = |iter: String| match keyed_values {
        Some(fns) => format!(
            "{{ let mut __c = SalvoSet::with_fns({fns}); for __e in {iter} {{ __c.insert(__e); }} __c }}"
        ),
        None => format!("SalvoSet::from_elements::<{}, _>({iter})", keyed()),
    };
    let map_ctor = |iter: String| match keyed_values {
        Some(fns) => format!(
            "{{ let mut __c = SalvoMap::with_fns({fns}); for (__k, __v) in {iter} {{ __c.insert(__k, __v); }} __c }}"
        ),
        None => format!("SalvoMap::from_entries::<{}, _>({iter})", keyed()),
    };
    let sorted_set_ctor = |iter: String| match keyed_values {
        Some(fns) => format!(
            "{{ let mut __c = SalvoSortedSet::with_cmp({fns}); for __e in {iter} {{ __c.insert(__e); }} __c }}"
        ),
        None => format!("SalvoSortedSet::from_elements::<{}, _>({iter})", ord()),
    };
    let sorted_map_ctor = |iter: String| match keyed_values {
        Some(fns) => format!(
            "{{ let mut __c = SalvoSortedMap::with_cmp({fns}); for (__k, __v) in {iter} {{ __c.insert(__k, __v); }} __c }}"
        ),
        None => format!("SalvoSortedMap::from_entries::<{}, _>({iter})", ord()),
    };
    // [cmp-carry] …and the hash/equality **pair** a keyed container is kept by:
    // one marker each, the host's own when the type names none. The emitter hands
    // them over as one rendered argument list, since they always travel together
    // [col-membership].
    let a = |i: usize| args.get(i).map(String::as_str).unwrap_or("todo!()");
    // [col-bounds] An `Int` argument in an **index** position. The cast goes
    // through `i64` rather than straight to `usize`, because a *literal* takes
    // its type from the cast target: `(-1) as usize` is rustc's E0600, so
    // `get(xs, -1)` type-checked in Salvo and then failed to build — a
    // [backend-never-wrong] violation, found and fixed 2026-09-22 while adding
    // `swap`. Through a signed type the negative value wraps to a huge `usize`,
    // which every length test rejects, which is the answer std promises.
    let index = |i: usize| format!("({}) as i64 as usize", a(i));
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
        // core.compare ---------------------------------------------------
        // [cmp-groups] The canonical `cmp`/`eq`/`hash` at each intrinsic type.
        //
        // `Ord::cmp` answers an `Ordering`, which is a fieldless `#[repr(i8)]`
        // enum whose discriminants are exactly the sign convention Salvo's
        // `cmp` answers (-1/0/1), so the cast *is* the lowering. It is written
        // as a path call rather than a method call so it works whether the
        // argument arrives as a value or as a borrow: `&T` has its own `Ord`
        // that delegates to `T`'s [rs-borrows].
        //
        // A `Str` is compared as `str` — byte-wise UTF-8, which is code-point
        // order, matching what the Kotlin backend has to arrange by hand
        // [kt-ordered].
        ("cmp", Some("Str")) => {
            format!("(Ord::cmp(&{}[..], &{}[..]) as i32)", a(0), a(1))
        }
        ("cmp", Some("Int" | "Long" | "Byte" | "Char" | "Bool")) => {
            format!("(Ord::cmp(&({}), &({})) as i32)", a(0), a(1))
        }
        // [interp-to-str] The scalars' text form as a function, so a
        // `?ToStr<T>` implicit can resolve one (added 2026-09-23 with the test
        // surface). `Display` is what interpolation itself uses
        // [rs-interp-to-str], so `to_str(x)` and `"${x}"` are the same string
        // by construction; a float's text is Salvo's rule [interp-float].
        ("to_str", Some("Double")) => format!("crate::strings::salvo_f64_text({})", a(0)),
        ("to_str", Some("Float")) => format!("crate::strings::salvo_f32_text({})", a(0)),
        ("to_str", Some("Int" | "Long" | "Byte" | "Char" | "Bool" | "Str")) => {
            format!("format!(\"{{}}\", {})", a(0))
        }
        ("eq", Some("Str")) => format!("(&{}[..] == &{}[..])", a(0), a(1)),
        // [fn-attached] A buffer compares **structurally**: `Vec<u8> ==
        // Vec<u8>` is element-wise, which is what the Kotlin runtime's
        // `SalvoBytes.equals` also does [kt-bytes].
        // [actor-types] Same actor: the same routable identity, so a proxy
        // and the actor it stands for compare equal.
        ("eq", Some("Addr")) => format!(
            "(crate::scheduler::salvo_addr_identity(({}).clone()) == crate::scheduler::salvo_addr_identity(({}).clone()))",
            a(0),
            a(1)
        ),
        ("eq", Some("Int" | "Long" | "Double" | "Float" | "Byte" | "Char" | "Bool")) => {
            format!("(({}) == ({}))", a(0), a(1))
        }
        // [cmp-hash-values] The host's own digest, which is `DefaultHasher`
        // here and `hashCode()` on Kotlin: the values differ between the
        // backends by design, and only the agreement with `eq` is promised.
        // A block expression keeps the hasher local, so a `hash` nested inside
        // another one is still one hasher per call.
        ("hash", Some("Str")) => format!(
            "{{ let mut __h = std::hash::DefaultHasher::new(); \
             std::hash::Hash::hash(&{}[..], &mut __h); \
             (std::hash::Hasher::finish(&__h) as i64) }}",
            a(0)
        ),
        ("hash", Some("Int" | "Long" | "Byte" | "Char" | "Bool")) => format!(
            "{{ let mut __h = std::hash::DefaultHasher::new(); \
             std::hash::Hash::hash(&({}), &mut __h); \
             (std::hash::Hasher::finish(&__h) as i64) }}",
            a(0)
        ),
        // [col-hashed-ordered] **Interim**: a `List` or a tuple compares, hashes
        // and orders through the host's structural implementations, which
        // reach an element struct's derive (kept for exactly this, see
        // `emit_struct`) rather than its Salvo fn. Until std owns container
        // identity through recursive implicit resolution (ROADMAP §2c and
        // §6), so an element's *declared* `cmp` is not consulted inside
        // a list — the limitation §6 records.
        ("cmp", Some("List" | "()")) => {
            format!("(Ord::cmp(&({}), &({})) as i32)", a(0), a(1))
        }
        ("eq", Some("List" | "()")) => format!("(({}) == ({}))", a(0), a(1)),
        ("hash", Some("List" | "()")) => format!(
            "{{ let mut __h = std::hash::DefaultHasher::new(); \
             std::hash::Hash::hash(&({}), &mut __h); \
             (std::hash::Hasher::finish(&__h) as i64) }}",
            a(0)
        ),
        // [cmp-hash-values] Folding one digest into another, wrapping: what a
        // structural `hash` combines its fields with. Written arithmetic
        // would trap here in a debug build, where the JVM wraps.
        ("mix_hash", Some("Long")) => format!(
            "(({}).wrapping_mul(31).wrapping_add({}))",
            a(0),
            a(1)
        ),
        // core.actor ---------------------------------------------------
        // [actor-replyto] [rs-actor] Answering a request: the token is
        // consumed, and the payload crosses the seam as the runtime's untyped
        // box. `Box::new` is where the sendability the checker proved becomes
        // Rust's `Send` bound on `SalvoMsg`.
        ("send", Some("Reply")) => format!("({}).send(std::boxed::Box::new({}))", a(0), a(1)),
        // [actor-spawn-expr] A pool is a scheduler index; `pool(n)` starts its
        // worker threads.
        // [pool-fault-sink] The two-argument overload: the sink's addr plus the
        // builder that turns the host's reason into the language's `Fault`
        // message — the runtime cannot construct one, exactly as with `Exit`
        // [actor-watch]. Dispatched on **arity**, since both overloads take an
        // `Int` first and the table's key is the receiver type.
        ("pool", Some("Int")) if args.len() == 2 => format!(
            "crate::scheduler::salvo_pool_with_sink((({}) as usize), Some(((({}) as usize), \
             |__reason| std::boxed::Box::new(__Msg_Faults::Faulted(Fault {{ reason: __reason }})))))",
            a(0),
            a(1)
        ),
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
            "crate::scheduler::salvo_watch({}, {}, |__reason| std::boxed::Box::new(Exit {{ reason: __reason }}))",
            a(0),
            a(1)
        ),
        // [actor-on-idle] The quiescence hook, registered the same way and for
        // the same reason: the runtime holds two counts and cannot construct
        // the language's `Idle`, so the registration site closes over the
        // constructor. `Idle` is named unqualified on `Exit`'s precedent above.
        ("on_idle", Some("Pool")) => format!(
            "crate::scheduler::salvo_on_idle({}, {}, |__gates, __tokens| std::boxed::Box::new(Idle {{ \
             parked_gates: __gates, parked_tokens: __tokens }}))",
            a(0),
            a(1)
        ),
        // time -----------------------------------------------------------
        // [time-ticker] [time-clock] [rs-time] The two clock readings, each a
        // plain `i64` of nanoseconds — the whole of what the host contributes
        // to the time surface. Everything else (`Duration`, `between`, the
        // correlation `DefaultClock` keeps) is ordinary Salvo over these.
        // [stream-handle] One counter for every stream table in the process.
        ("epoch_nanos", None) => "crate::hosttime::salvo_epoch_nanos()".to_string(),
        // [col-of-nonempty] The constructors match on the *name*, because the
        // first parameter no longer identifies them: the empty one has none and
        // the element one starts with a `T`. Three call shapes reach here.
        //
        // A lone `...spread` fills `first` [fn-variadic], so it arrives as the
        // only argument and the spread *is* the list.
        ("list_of" | "mut_list_of", _) if spread_any && args.len() == 1 => owned_vec(),
        // `list_of(a, b, ...rest)`: the leading elements, then the tail's
        // elements. `vec![…]` and `extend` rather than one expression because
        // the tail is a collection, not an element.
        ("list_of" | "mut_list_of", _) if spread_any => {
            let leading: Vec<&str> = args[..args.len() - 1].iter().map(|s| s.as_str()).collect();
            let tail = &args[args.len() - 1];
            let extend = match spread {
                Spread::Owned => format!("{tail}.into_iter()"),
                _ => format!("{tail}.iter().cloned()"),
            };
            format!(
                "{{ let mut __v = vec![{}]; __v.extend({extend}); __v }}",
                leading.join(", ")
            )
        }
        // No spread: the arguments are the elements. `vec![]` needs no element
        // type — rustc infers it from later use.
        ("list_of" | "mut_list_of", _) => {
            format!("vec![{}]", args.join(", "))
        }
        // `T?` is physical here, so an out-of-range index must produce
        // `None` rather than panic. The element is **borrowed**
        // (`Option<&T>`): `get` declares `(proj(list) T)?`, and a
        // caller that needs ownership says `copy` [copy-opt-in]. (Until
        // 2026-09-11 every read cloned, even one that only tested `None`.)
        ("get", Some("[]")) => {
            if loc_lend {
                // [rs-loc] The locator form: the position when present,
                // `None` where the read would have answered `None` — same
                // absence semantics, no borrow taken.
                format!(
                    "{{ let __i = {}; if __i < {}.len() {{ Some(__i) }} else {{ None }} }}",
                    index(1),
                    a(0)
                )
            } else {
                format!("{}.get({})", a(0), index(1))
            }
        }
        // `first` is a derived return (`proj(list)`), so it
        // borrows rather than clones [readonly-return].
        ("first", Some("[]")) => format!("{}.first()", a(0)),
        ("size", Some("[]")) => {
            format!("({}.len() as i32)", a(0))
        }
        // [interp-to-str] `[1, 2, 3]` — the format is fixed by the language,
        // not by the target's own collection formatting, so both backends
        // print the same text [backend-parity]. (Rust `Debug` would quote
        // strings and Kotlin's `toString` would not.)
        ("to_str", Some("[]")) => format!(
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
        ("set_by", Some("Int")) | ("mut_set_by", Some("Int")) => {
            set_ctor(format!("(0..({})).map({})", a(0), a(1)))
        }
        ("map_by", Some("Int")) | ("mut_map_by", Some("Int")) => {
            map_ctor(format!("(0..({})).map({})", a(0), a(1)))
        }

        // [col-convert] The converters.
        ("to_set", Some("List")) => set_ctor(format!("{}.iter().cloned()", a(0))),
        ("to_map", Some("List")) if args.len() == 1 => {
            map_ctor(format!("{}.iter().cloned()", a(0)))
        }
        ("to_map", Some("List")) => map_ctor(format!("{}.iter().map({})", a(0), a(1))),


        // core.set -------------------------------------------------------
        // [rs-collections] The ordered set from the runtime file. A
        // `...spread` arrives as the whole `Vec<T>`, so it is the element
        // source itself — cloned, since a variadic position is not tracked
        // by the flow analysis and the array stays usable afterwards (the
        // same reasoning as the list constructors above).
        ("set_of", Some("[]")) | ("mut_set_of", Some("[]")) if spread_any => {
            set_ctor(owned_iter())
        }
        ("set_of", Some("[]")) | ("mut_set_of", Some("[]")) => {
            set_ctor(format!("vec![{}]", args.join(", ")))
        }
        // [col-key-eligible] An owned read of a snapshot element — a clone
        // here, where Kotlin can share the reference.
        ("snapshot_at", Some("List")) => {
            format!("{}.get({}).cloned()", a(0), index(1))
        }

        // core.sorted ----------------------------------------------------
        // [col-sorted] `BTreeSet`/`BTreeMap` keep their keys in order, and
        // `from_iter` builds one from anything iterable.
        ("sorted_set_of", Some("[]")) | ("mut_sorted_set_of", Some("[]")) if spread_any => {
            sorted_set_ctor(owned_iter())
        }
        ("sorted_set_of", Some("[]")) | ("mut_sorted_set_of", Some("[]")) => {
            sorted_set_ctor(format!("vec![{}]", args.join(", ")))
        }
        ("add", Some("SortedSet")) => format!("{}.insert({})", a(0), a(1)),
        ("remove", Some("SortedSet")) => format!("{}.remove(&{})", a(0), a(1)),
        ("contains", Some("SortedSet")) => format!("{}.contains(&{})", a(0), a(1)),
        ("size", Some("SortedSet")) => format!("({}.len() as i32)", a(0)),
        // Cheap at either end of an ordered tree, which is the reason to use
        // one; cloned because the declaration hands back an owned `T?`.
        ("min", Some("SortedSet")) => format!("{}.min().cloned()", a(0)),
        ("max", Some("SortedSet")) => format!("{}.max().cloned()", a(0)),
        ("to_list", Some("SortedSet")) => format!("{}.to_vec()", a(0)),
        // [col-to-str] The language's format, in key order — written out
        // rather than left to Rust's `Debug` [backend-parity].
        ("to_str", Some("SortedSet")) => format!(
            "format!(\"{{{{{{}}}}}}\", {}.to_vec().iter().map(|__e| __e.to_string())\
             .collect::<Vec<_>>().join(\", \"))",
            a(0)
        ),

        ("sorted_map_of", Some("[]")) | ("mut_sorted_map_of", Some("[]")) if spread_any => {
            sorted_map_ctor(owned_iter())
        }
        ("sorted_map_of", Some("[]")) | ("mut_sorted_map_of", Some("[]")) => {
            sorted_map_ctor(format!("vec![{}]", args.join(", ")))
        }
        ("get", Some("SortedMap")) => format!("{}.get(&{})", a(0), a(1)),
        ("put", Some("SortedMap")) => format!("{}.insert({}, {})", a(0), a(1), a(2)),
        ("remove", Some("SortedMap")) => format!("{}.remove(&{})", a(0), a(1)),
        ("contains_key", Some("SortedMap")) => format!("{}.contains_key(&{})", a(0), a(1)),
        ("size", Some("SortedMap")) => format!("({}.len() as i32)", a(0)),
        ("first_key", Some("SortedMap")) => format!("{}.first_key().cloned()", a(0)),
        ("last_key", Some("SortedMap")) => format!("{}.last_key().cloned()", a(0)),
        ("keys", Some("SortedMap")) => format!("{}.keys()", a(0)),
        ("to_str", Some("SortedMap")) => format!(
            "format!(\"{{{{{{}}}}}}\", {}.entries().iter()\
             .map(|(__k, __v)| format!(\"{{}}: {{}}\", __k, __v))\
             .collect::<Vec<_>>().join(\", \"))",
            a(0)
        ),

        // core.map -------------------------------------------------------
        // [rs-collections] Entries are native 2-tuples on this backend
        // [type-tuple], which is exactly what `from_entries` consumes.
        ("map_of", Some("[]")) | ("mut_map_of", Some("[]")) if spread_any => {
            map_ctor(owned_iter())
        }
        ("map_of", Some("[]")) | ("mut_map_of", Some("[]")) => {
            map_ctor(format!("vec![{}]", args.join(", ")))
        }
        // `get` borrows the value out of the map (`Option<&V>`): the
        // declaration is `(proj(map) V)?`, so a caller who wants to
        // keep it says `copy` [copy-opt-in].
        ("get", Some("Map")) => format!("{}.get(&{})", a(0), a(1)),
        // [qual-depend] The total read behind `get(map, key: KeyOf(map) K)`:
        // the claim proved presence, so the unwrap cannot fire.
        ("get_present", Some("Map")) => format!("{}.get(&{}).unwrap()", a(0), a(1)),
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
        "None" => "()",
        // Only reachable in dead positions.
        "Never" => "()",
        "List" => "Vec",
        // [col-deque] Both ends O(1); `Deque` and `Mut Deque` are one type,
        // mutability living in the binding as for `Vec` [type-canbe-mut].
        "Deque" => "std::collections::VecDeque",
        // [col-insertion-order] Not `HashSet`/`HashMap`: those have no
        // iteration order to speak of (unspecified, and randomly seeded per
        // process), while Salvo's collections iterate in insertion order on
        // every backend. The runtime file supplies the ordered equivalents
        // with `LinkedHashMap` semantics [rs-collections].
        "Set" => "SalvoSet",
        "Map" => "SalvoMap",
        // [col-sorted] [cmp-carry] The runtime's ordered collections: a B-tree
        // behind a store that carries the ordering the *type* names, so two sets
        // ordered differently are one Rust type and no signature grows a
        // parameter for the difference [rs-collections].
        "SortedSet" => "SalvoSortedSet",
        "SortedMap" => "SalvoSortedMap",
        _ => return None,
    })
}

/// The body of an `intrinsic handler`'s member [backend-intrinsic]: the
/// statements (or, for a value-returning member, the expression) that
/// implement it, with the member's own parameter names in scope.
///
/// Paths are absolute so the emitted file needs no `use` items.
pub fn handler_member(handler: &str, member: &str, params: &[String]) -> Option<String> {
    // std declares no intrinsic handler any more (ROADMAP 0.5): `StdOutConsole`
    // is a platform handler and `DefaultRandom` is Salvo.
    let _ = (handler, member, params);
    None
}
