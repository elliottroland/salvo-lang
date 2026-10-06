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
    ordering: Option<&str>,
    keyed: Option<(&str, &str)>,
) -> Option<String> {
    let elem = || type_args.first().map(String::as_str).unwrap_or("Any");
    let a = |i: usize| args.get(i).map(String::as_str).unwrap_or("TODO()");
    // [cmp-carry] The comparison a sorted collection is kept by: the function its
    // type names, or the language's structural comparison when it names none.
    // [cmp-carry] A hash container's constructor: the runtime container when the
    // type names a `hash`/`eq` pair, the JVM's own `LinkedHashMap` otherwise —
    // which is what every Salvo program has always compiled to.
    let set_ctor = |elem: &str| match keyed {
        Some((h, e)) => format!("salvo.SalvoHashSet<{elem}>({h}, {e})"),
        None => format!("linkedSetOf<{elem}>()"),
    };
    let map_ctor = |k: &str, v: &str| match keyed {
        Some((h, e)) => format!("salvo.SalvoHashMap<{k}, {v}>({h}, {e})"),
        None => format!("linkedMapOf<{k}, {v}>()"),
    };
    // [platform-check] The canonical ordering is one object, so a sorted
    // collection can be recognized as sorted the program's way.
    let comparator = || match ordering {
        Some(target) => format!("java.util.Comparator {{ __a, __b -> {target}(__a, __b) }}"),
        None => "salvo.SalvoCanonicalOrder".to_string(),
    };
    Some(match (name, recv) {
        // core.basic -----------------------------------------------------
        // [op-convert] The explicit numeric conversions. Kotlin's `toX()`
        // agrees with Rust's `as` case by case: float→int truncates toward
        // zero and saturates (NaN → 0), Long→Int keeps the low 32 bits,
        // Double→Float rounds.
        ("to_int", Some("Long" | "Double" | "Float")) => format!("({}).toInt()", a(0)),
        ("to_long", Some("Int" | "Double" | "Float")) => format!("({}).toLong()", a(0)),
        ("to_double", Some("Int" | "Long" | "Float")) => format!("({}).toDouble()", a(0)),
        ("to_float", Some("Int" | "Long" | "Double")) => format!("({}).toFloat()", a(0)),
        // [kt-byte-unsigned] [byte-value] A `Byte` is a `UByte` here, so the
        // pair round-trips through the unsigned type: `toUByte()` keeps the
        // low 8 bits exactly as Rust's `as u8` does, and `toInt()` widens
        // into 0..255 exactly as `as i32` does.
        ("to_byte", Some("Int")) => format!("({}).toUByte()", a(0)),
        ("to_int", Some("Byte")) => format!("({}).toInt()", a(0)),
        // core.compare ---------------------------------------------------
        // [cmp-groups] The canonical `cmp`/`eq`/`hash` at each intrinsic type.
        //
        // Every one of these is `Comparable` on the JVM *except* `Str`, whose
        // `compareTo` is UTF-16 code-unit order where Salvo's `Str` order is
        // code point [kt-ordered] — so the string case goes through the same
        // runtime helper the sorted collections use, and the rest compare
        // natively.
        ("cmp", Some("Str")) => format!("salvo.__salvoCompare({}, {})", a(0), a(1)),
        ("cmp", Some("Int" | "Long" | "Byte" | "Char" | "Bool")) => {
            format!("({}).compareTo({})", a(0), a(1))
        }
        // [interp-to-str] The scalars' text form as a function, so a
        // `?ToStr<T>` implicit can resolve one (added 2026-09-23 with the test
        // surface). `toString()` is what interpolation lowers to as well, so
        // `to_str(x)` and `"${x}"` agree by construction. `Double`/`Float`
        // have no `to_str`: the two hosts disagree about printing a whole
        // float.
        // A `Str` is its own text: `toString()` on one is kotlinc's
        // "redundant conversion" warning (0c item 15).
        ("to_str", Some("Str")) => a(0).to_string(),
        // [interp-float] Kotlin's own text is Salvo's rule.
        ("to_str", Some("Int" | "Long" | "Byte" | "Char" | "Bool" | "Double" | "Float")) => {
            format!("({}).toString()", a(0))
        }
        // Structural equality on every intrinsic type: Kotlin's `==` is
        // `equals`, which is value equality for the boxed primitives and for
        // `String`. The float widths are IEEE here as they are on Rust
        // (`NaN != NaN`) because `Double == Double` on *primitive* operands
        // compiles to a numeric comparison — the boxing hazard
        // [kt-float-eq] fixes lives in generated `equals` methods, not here.
        ("eq", Some("Int" | "Long" | "Double" | "Float" | "Byte" | "Char" | "Bool" | "Str")) => {
            format!("(({}) == ({}))", a(0), a(1))
        }
        // [fn-attached] [kt-bytes] A buffer compares **structurally**: the
        // [cmp-hash-values] The host's own digest, widened to the `Long` the
        // signature answers. Values differ from Rust's by design; what holds
        // on both is that equal values hash equal.
        ("hash", Some("Int" | "Long" | "Byte" | "Char" | "Bool" | "Str")) => {
            format!("({}).hashCode().toLong()", a(0))
        }
        // [col-hashed-ordered] **Interim** (see the Rust side): a `List` or a
        // tuple through the host's structural `equals`/`hashCode`, and the
        // runtime comparator for order [kt-ordered].
        ("cmp", Some("List" | "()")) => format!("salvo.__salvoCompare({}, {})", a(0), a(1)),
        ("eq", Some("List" | "()")) => format!("(({}) == ({}))", a(0), a(1)),
        ("hash", Some("List" | "()")) => format!("({}).hashCode().toLong()", a(0)),
        // [cmp-hash-values] The wrapping fold a structural `hash` combines
        // its fields with; the JVM's `Long` arithmetic wraps by itself.
        // [op-bits] Kotlin's infix bit operations; its shifts already take
        // the count modulo the width, which is the language's rule.
        ("bit_and", Some("Int" | "Long")) => format!("(({}) and ({}))", a(0), a(1)),
        ("bit_or", Some("Int" | "Long")) => format!("(({}) or ({}))", a(0), a(1)),
        ("bit_xor", Some("Int" | "Long")) => format!("(({}) xor ({}))", a(0), a(1)),
        ("bit_not", Some("Int" | "Long")) => format!("({}).inv()", a(0)),
        ("bit_shl", Some("Int" | "Long")) => format!("(({}) shl ({}))", a(0), a(1)),
        ("bit_shr", Some("Int" | "Long")) => format!("(({}) shr ({}))", a(0), a(1)),
        ("bit_ushr", Some("Int" | "Long")) => format!("(({}) ushr ({}))", a(0), a(1)),
        // runtime ------------------------------------------------------
        // [runtime-handles] An addr and a pool are `Int` indices already.
        ("addr_index", Some("Addr")) | ("pool_index", Some("Pool")) | ("addr_of", Some("Int"))
        | ("pool_of", Some("Int")) => format!("({})", a(0)),
        // core.actor ---------------------------------------------------
        // [actor-replyto] [kt-actor] Answering a request: the token is
        // consumed and the payload crosses the seam as the runtime's `Any?`.
        ("send", Some("Reply")) => format!("{}.send({})", a(0), a(1)),
        // [actor-spawn-expr] A pool is a scheduler id; `pool(n)` starts its
        // daemon worker threads.
        // [pool-fault-sink] The two-argument overload: the sink's addr plus the
        // builder that turns the host's reason into the language's `Fault`
        // message — the runtime cannot construct one, exactly as with `Exit`
        // [actor-watch]. Dispatched on **arity**, since both overloads take an
        // `Int` first and the table's key is the receiver type.
        ("pool", Some("Int")) if args.len() == 2 => format!(
            "salvo.SalvoSched.poolWithSink({}, {}, {{ __reason -> \
             __Msg_Faults.Faulted(Fault(__reason)) }})",
            a(0),
            a(1)
        ),
        // [waitfor-dedicated] `thread()` is a pool of one, and the only
        // placement the language types `Dedicated` — the qualifier is erased,
        // so what reaches here is a plain pool id.
        ("thread", None) => "salvo.SalvoSched.thread()".to_string(),
        // [runtime-handles] The core's token inside a reply token, or `null`
        // for one minted on another node.
        ("reply_token", Some("Reply")) => format!("({}).takeLocal()", a(0)),
        // time -----------------------------------------------------------
        // [time-ticker] [time-clock] [kt-time] The two clock readings, each a
        // `Long` of nanoseconds — the whole of the host's contribution to the
        // time surface, matching `time.rs` number for number.
        // [stream-handle] One counter for every stream table in the process.
        // core.list ------------------------------------------------------
        // The element type is spelled out: `listOf()` with no arguments
        // leaves kotlinc with nothing to infer from
        // [backend-intrinsic].
        // [fn-variadic] A `...spread` argument arrives as one `Array<T>`,
        // which Kotlin passes on with its own spread operator — the
        // alternative (`listOf(arr)`) is a *list of one array*, and kotlinc
        // says so, but only after the fact [backend-never-wrong].
        // [col-of-nonempty] The empty constructor has no parameters and the
        // element one starts with a `T`, so these match on the *name*: the
        // receiver type no longer identifies them.
        ("list_of", _) => format!("listOf<{}>({})", elem(), args.join(", ")),
        ("mut_list_of", _) => {
            format!("mutableListOf<{}>({})", elem(), args.join(", "))
        }
        // [interp-to-str] `[1, 2, 3]`, the language's format rather than the
        // JVM's — `joinToString` already produces exactly it, but writing it
        // out is what pins the parity with Rust [backend-parity].
        ("to_str", Some("[]")) => {
            format!("{}.joinToString(\", \", \"[\", \"]\")", a(0))
        }


        // [col-by] The generated constructors. The callback is handed to a
        // Kotlin builder (`Array(n, init)`, `MutableList(n, init)`) or to
        // `.map(init)` rather than being invoked inline: an immediately
        // applied lambda literal has no expected type, and kotlinc then
        // demands an explicit parameter type ("an explicit type is required
        // on a value parameter"). Passing it where a `(Int) -> T` is wanted
        // is what types its parameter.
        // [test-recover] [kt-assert-trap] The harness's catch: run the body,
        // answer the trap's message or `null`. `Throwable` rather than a
        // narrower type on purpose — the harness wants *every* death it can
        // survive, ours (`AssertionError`) and the host's (a null dereference,
        // an index out of bounds, a division by zero) alike.
        ("array_by", Some("Int")) => {
            format!("Array<{}>({}, {})", elem(), a(0), a(1))
        }
        ("set_by", Some("Int")) | ("mut_set_by", Some("Int")) => format!(
            "{}.also {{ __s -> __s.addAll((0 until ({})).map({})) }}",
            set_ctor(&elem()),
            a(0),
            a(1)
        ),
        ("map_by", Some("Int")) | ("mut_map_by", Some("Int")) => format!(
            "{}.also {{ __m -> (0 until ({})).map({})\
             .forEach {{ __e -> __m.put(__e.first, __e.second) }} }}",
            map_ctor(
                type_args.first().map(String::as_str).unwrap_or("Any"),
                type_args.get(1).map(String::as_str).unwrap_or("Any"),
            ),
            a(0),
            a(1)
        ),

        // [col-convert] The converters. `LinkedHashSet`/`LinkedHashMap` keep
        // first-appearance order [col-insertion-order].
        ("to_set", Some("List")) => {
            format!("{}.also {{ __s -> __s.addAll({}) }}", set_ctor(&elem()), a(0))
        }
        ("to_map", Some("List")) if args.len() == 1 => format!(
            "{}.also {{ __m -> {}\
             .forEach {{ __e -> __m.put(__e.first, __e.second) }} }}",
            map_ctor(
                type_args.first().map(String::as_str).unwrap_or("Any"),
                type_args.get(1).map(String::as_str).unwrap_or("Any"),
            ),
            a(0)
        ),
        ("to_map", Some("List")) => format!(
            "{}.also {{ __m -> {}.map({})\
             .forEach {{ __e -> __m.put(__e.first, __e.second) }} }}",
            map_ctor(
                type_args.get(1).map(String::as_str).unwrap_or("Any"),
                type_args.get(2).map(String::as_str).unwrap_or("Any"),
            ),
            a(0),
            a(1)
        ),

        // core.set -------------------------------------------------------
        // [col-insertion-order] `linkedSetOf` is a `LinkedHashSet`, whose
        // iteration order is first-insertion — which is the language's rule,
        // not the JVM's default for every set. It is both a `Set` and a
        // `MutableSet`, so the same constructor serves both declarations
        // [type-canbe-mut]; the element type is spelled out for the same
        // reason the list constructors spell it [backend-intrinsic].
        ("set_of", Some("[]")) | ("mut_set_of", Some("[]")) => {
            format!("{}.also {{ __s -> __s.addAll(listOf({})) }}", set_ctor(&elem()), args.join(", "))
        }
        // [col-key-eligible] An owned read of a snapshot element. Identity
        // is the copy here because element/key types are the immutable
        // intrinsic types, which is what `copy` of a generic cannot assume
        // [kt-copy].
        ("snapshot_at", Some("List")) => format!("{}.getOrNull({})", a(0), a(1)),

        // core.sorted ----------------------------------------------------
        // [cmp-carry] The comparator a sorted collection is built with: the
        // ordering its *type* names, or the language's structural comparison when
        // it names none — which is the canonical path and what `__salvoCompare`
        // has always provided.

        // [col-sorted] [kt-ordered] A `TreeSet`/`TreeMap` built with **our**
        // comparator rather than natural ordering: Salvo orders a `List` or a
        // tuple by its elements (neither is `Comparable` on the JVM) and a
        // `Str` by code point (`String.compareTo` is UTF-16 code-unit order),
        // so natural ordering would disagree with Rust [backend-parity].
        ("sorted_set_of", Some("[]")) | ("mut_sorted_set_of", Some("[]")) => format!(
            "java.util.TreeSet<{}>({}).also {{ __s -> \
             __s.addAll(listOf({})) }}",
            elem(),
            comparator(),
            args.join(", ")
        ),

        ("sorted_map_of", Some("[]")) | ("mut_sorted_map_of", Some("[]")) => format!(
            "java.util.TreeMap<{}, {}>({}).also {{ __m -> \
             __m.putAll(listOf({})) }}",
            type_args.first().map(String::as_str).unwrap_or("Any"),
            type_args.get(1).map(String::as_str).unwrap_or("Any"),
            comparator(),
            args.join(", ")
        ),
        ("to_str", Some("SortedMap")) => format!(
            "{}.entries.joinToString(\", \", \"{{\", \"}}\") \
             {{ \"${{it.key}}: ${{it.value}}\" }}",
            a(0)
        ),

        // core.map -------------------------------------------------------
        // [col-insertion-order] `linkedMapOf` is a `LinkedHashMap`: an
        // overwrite keeps the key's position and removal preserves the
        // order of the rest. It takes `Pair`s, which is what a Salvo
        // 2-tuple already is [type-tuple], so the entries pass straight
        // through. A repeated key resolves last-wins, as Salvo's rule says
        // [col-duplicate-keys].
        // [cmp-carry] Through `map_ctor`, like every other keyed constructor:
        // this one built a `linkedMapOf` directly, so a map whose type named a
        // `hash`/`eq` pair silently keyed by the JVM's `hashCode`/`equals`
        // instead — the *set* beside it was routed and the map was not (fixed
        // 2026-09-26, when the constructors began naming an identity and the
        // two disagreed on the same program).
        ("map_of", Some("[]")) | ("mut_map_of", Some("[]")) => format!(
            "{}.also {{ __m -> __m.putAll(listOf({})) }}",
            map_ctor(
                type_args.first().map(String::as_str).unwrap_or("Any"),
                type_args.get(1).map(String::as_str).unwrap_or("Any"),
            ),
            args.join(", ")
        ),
        // [col-to-str] `{a: 1, b: 2}` — the map literal's shape. Kotlin's
        // own `toString` renders `{a=1}`, so the separator is written out.
        ("to_str", Some("Map")) => format!(
            "{}.entries.joinToString(\", \", \"{{\", \"}}\") \
             {{ \"${{it.key}}: ${{it.value}}\" }}",
            a(0)
        ),

        // core.array -----------------------------------------------------
        // [fn-variadic] The variadic tail *is* the array; a `...spread`
        // arrives as `*arr`, which `arrayOf` re-wraps into an array of the
        // same elements.
        ("array_of", Some("[]")) => {
            format!("arrayOf<{}>({})", elem(), args.join(", "))
        }
        // `T[]` maps to `Array<T>`, which is *not* `Iterable`, hence the
        // `asIterable()` the list case does not need [type-array].
        ("get", Some("[]")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("first", Some("[]")) => format!("{}.firstOrNull()", a(0)),
        ("size", Some("[]")) => format!("{}.size", a(0)),


        // core.bytes is platform code since 2026-10-04 [platform-value-type]:
        // `std/platform/core/bytes.{rs,kt}`.

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
        // [kt-byte-unsigned] Unsigned, so the same octet prints and compares
        // the same on both backends: a signed `Byte` would render 255 as
        // `-1` where Rust's `u8` renders `255` [backend-parity].
        "Byte" => "UByte",
        // [actor-types] [kt-actor] The scheduler's handles: an addr and a pool
        // are ids into it, a reply token is its own class. Their Salvo type
        // *arguments* are dropped by `emit_named_parts` — the effect a
        // `Addr<E>` serves is the checker's business, and the generated message
        // classes carry payload types.
        "Addr" => "Int",
        "Pool" => "Int",
        "Reply" => "salvo.SalvoReply",
        "None" => "Unit",
        "Any" => "Any",
        "Never" => "Nothing",
        "List" => "List",
        // [col-deque] One class for `Deque` and `Mut Deque`, as `SalvoBytes`
        // is for `Bytes` [type-canbe-mut].
        "Deque" => "kotlin.collections.ArrayDeque",
        // [col-insertion-order] The immutable views of the ordered
        // implementations the constructors build: a `LinkedHashSet` *is* a
        // `Set` and a `LinkedHashMap` *is* a `Map`, so the declared type
        // stays the interface and the order comes from the instance.
        "Set" => "Set",
        "Map" => "Map",
        // [col-sorted] The JVM's sorted views of the trees the constructors
        // build: a `TreeSet` *is* a `SortedSet`, so dropping `Mut` is free.
        "SortedSet" => "java.util.SortedSet",
        "SortedMap" => "java.util.SortedMap",
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
        // Like `MutableList`, these *are* subtypes of their immutable
        // forms, so dropping the `Mut` is free [str-drop-mut].
        "Set" => Some("MutableSet"),
        "Map" => Some("MutableMap"),
        // [col-sorted] `java.util.SortedSet`/`SortedMap` are already mutable
        // interfaces on the JVM, so `Mut` needs no different type here — and
        // dropping it renders nothing [str-drop-mut].
        "SortedSet" => Some("java.util.SortedSet"),
        "SortedMap" => Some("java.util.SortedMap"),
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
    // std declares no intrinsic handler any more (ROADMAP 0.5): `StdOutConsole`
    // is a platform handler and `DefaultRandom` is Salvo.
    let _ = (handler, member, params);
    None
}
