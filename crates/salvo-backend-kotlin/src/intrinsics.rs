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
        // core.process ---------------------------------------------------
        // [async-replyto] [kt-process] Answering a request: the token is
        // consumed and the payload crosses the seam as the runtime's `Any?`.
        ("send", Some("Reply")) => format!("{}.send({})", a(0), a(1)),
        // [async-spawn-expr] A pool is a scheduler id; `pool(n)` starts its
        // daemon worker threads.
        ("pool", Some("Int")) => format!("salvo.SalvoSched.pool({})", a(0)),
        // [async-watch] Registering a death watch hands the scheduler the
        // token *and* a builder for the `Exit` it will carry: the runtime
        // holds a reason string and cannot construct a Salvo class, so the
        // watch site closes over the constructor instead.
        //
        // `Exit` is named unqualified, which is safe rather than lucky: the
        // file star-imports every module whose names it uses, and a
        // `Reply<Exit>` cannot be *obtained* in a file where `Exit` means
        // something else (the annotation naming std's `Exit` would not
        // resolve), so a shadowing declaration and this emission never meet.
        ("watch", Some("Addr")) => format!(
            "salvo.SalvoSched.watch({}, {}, {{ __reason -> Exit(__reason) }})",
            a(0),
            a(1)
        ),
        // core.list ------------------------------------------------------
        // The element type is spelled out: `listOf()` with no arguments
        // leaves kotlinc with nothing to infer from
        // [backend-intrinsic].
        // [fn-variadic] A `...spread` argument arrives as one `Array<T>`,
        // which Kotlin passes on with its own spread operator — the
        // alternative (`listOf(arr)`) is a *list of one array*, and kotlinc
        // says so, but only after the fact [backend-never-wrong].
        ("list_of", Some("[]")) => format!("listOf<{}>({})", elem(), args.join(", ")),
        ("mut_list_of", Some("[]")) => {
            format!("mutableListOf<{}>({})", elem(), args.join(", "))
        }
        // [col-sorted-list] `sortedWith` over Salvo's own comparator, never
        // natural ordering: `String.compareTo` is UTF-16 code-unit order,
        // where Rust's `String: Ord` is code-point order [kt-ordered].
        ("sort", Some("List")) => format!(
            "{}.sortedWith(Comparator {{ __a, __b -> salvo.__salvoCompare(__a, __b) }})",
            a(0)
        ),
        ("mut_sort", Some("List")) => format!(
            "{}.sortedWith(Comparator {{ __a, __b -> salvo.__salvoCompare(__a, __b) }})\
             .toMutableList()",
            a(0)
        ),
        // A **lower bound**, by scan: the first index whose element is not
        // below `elem`. `binarySearch` would answer an arbitrary index within
        // an equal run, which would diverge from Rust [col-sorted-list].
        ("add_sorted", Some("List")) => format!(
            "{}.let {{ __l -> {}.let {{ __e -> __l.add(\
             __l.indexOfFirst {{ salvo.__salvoCompare(it, __e) >= 0 }}\
             .let {{ if (it < 0) __l.size else it }}, __e) }} }}",
            a(0),
            a(1)
        ),
        ("binary_search", Some("List")) => format!(
            "{}.let {{ __l -> {}.let {{ __e -> __l.indexOfFirst \
             {{ salvo.__salvoCompare(it, __e) >= 0 }}\
             .let {{ if (it >= 0 && __l[it] == __e) it else null }} }} }}",
            a(0),
            a(1)
        ),
        ("get", Some("List")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("add", Some("List")) => format!("{}.add({})", a(0), a(1)),
        // [linear-container] Take-by-move: the element leaves the list. A
        // `let` so the receiver is evaluated once, and `removeAt` answers the
        // element it removed.
        ("remove_first", Some("List")) => format!(
            "({}).let {{ __l -> if (__l.isEmpty()) null else __l.removeAt(0) }}",
            a(0)
        ),
        ("remove_at", Some("List")) => format!(
            "({}).let {{ __l -> ({}).let {{ __i -> \
             if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null }} }}",
            a(0),
            a(1)
        ),
        // [linear-container] The terminal: every element is handed to the
        // callback, which owns it. Over a snapshot, so the callback may touch
        // the collection the list came from; the list itself is spent — the
        // checker made the caller put a fresh one back.
        // `forEach` rather than a `for` loop: the callback's parameter type is
        // then inferred from the receiver through `forEach`'s own signature,
        // where an immediately-applied lambda literal leaves kotlinc asking
        // for an explicit type ("an explicit type is required on a value
        // parameter"). Over a snapshot, so the callback may touch the
        // collection the list came from.
        ("drain", Some("List")) => format!("({}).toList().forEach({})", a(0), a(1)),
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

        // [col-by] The generated constructors. The callback is handed to a
        // Kotlin builder (`Array(n, init)`, `MutableList(n, init)`) or to
        // `.map(init)` rather than being invoked inline: an immediately
        // applied lambda literal has no expected type, and kotlinc then
        // demands an explicit parameter type ("an explicit type is required
        // on a value parameter"). Passing it where a `(Int) -> T` is wanted
        // is what types its parameter.
        ("array_by", Some("Int")) => {
            format!("Array<{}>({}, {})", elem(), a(0), a(1))
        }
        ("list_by", Some("Int")) | ("mut_list_by", Some("Int")) => {
            format!("MutableList<{}>({}, {})", elem(), a(0), a(1))
        }
        ("set_by", Some("Int")) | ("mut_set_by", Some("Int")) => format!(
            "linkedSetOf<{}>().also {{ __s -> __s.addAll((0 until ({})).map({})) }}",
            elem(),
            a(0),
            a(1)
        ),
        ("map_by", Some("Int")) | ("mut_map_by", Some("Int")) => format!(
            "linkedMapOf<{}, {}>().also {{ __m -> (0 until ({})).map({})\
             .forEach {{ __e -> __m.put(__e.first, __e.second) }} }}",
            type_args.first().map(String::as_str).unwrap_or("Any"),
            type_args.get(1).map(String::as_str).unwrap_or("Any"),
            a(0),
            a(1)
        ),

        // [col-convert] The converters. `LinkedHashSet`/`LinkedHashMap` keep
        // first-appearance order [col-insertion-order].
        ("to_set", Some("List")) => {
            format!("linkedSetOf<{}>().also {{ __s -> __s.addAll({}) }}", elem(), a(0))
        }
        ("to_map", Some("List")) if args.len() == 1 => format!(
            "linkedMapOf<{}, {}>().also {{ __m -> {}\
             .forEach {{ __e -> __m.put(__e.first, __e.second) }} }}",
            type_args.first().map(String::as_str).unwrap_or("Any"),
            type_args.get(1).map(String::as_str).unwrap_or("Any"),
            a(0)
        ),
        ("to_map", Some("List")) => format!(
            "linkedMapOf<{}, {}>().also {{ __m -> {}.map({})\
             .forEach {{ __e -> __m.put(__e.first, __e.second) }} }}",
            type_args.get(1).map(String::as_str).unwrap_or("Any"),
            type_args.get(2).map(String::as_str).unwrap_or("Any"),
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
            format!("linkedSetOf<{}>({})", elem(), args.join(", "))
        }
        // `add`/`remove` already report whether the set changed, which is
        // what Salvo's `Bool` returns mean.
        ("add", Some("Set")) => format!("{}.add({})", a(0), a(1)),
        ("remove", Some("Set")) => format!("{}.remove({})", a(0), a(1)),
        ("contains", Some("Set")) => format!("{}.contains({})", a(0), a(1)),
        ("size", Some("Set")) => format!("{}.size", a(0)),
        // [col-insertion-order] A `LinkedHashSet` iterates in insertion
        // order, so the list is that order.
        ("to_list", Some("Set")) => format!("{}.toMutableList()", a(0)),
        // [col-key-eligible] An owned read of a snapshot element. Identity
        // is the copy here because element/key types are the immutable
        // intrinsic types, which is what `copy` of a generic cannot assume
        // [kt-copy].
        ("snapshot_at", Some("List")) => format!("{}.getOrNull({})", a(0), a(1)),
        // [col-to-str] `{1, 2, 3}` — the set literal's own shape, written
        // out rather than left to the JVM's `toString` so both backends
        // print the same text [backend-parity].
        ("to_str", Some("Set")) => {
            format!("{}.joinToString(\", \", \"{{\", \"}}\")", a(0))
        }

        // core.sorted ----------------------------------------------------
        // [col-sorted] [kt-ordered] A `TreeSet`/`TreeMap` built with **our**
        // comparator rather than natural ordering: Salvo orders a `List` or a
        // tuple by its elements (neither is `Comparable` on the JVM) and a
        // `Str` by code point (`String.compareTo` is UTF-16 code-unit order),
        // so natural ordering would disagree with Rust [backend-parity].
        ("sorted_set_of", Some("[]")) | ("mut_sorted_set_of", Some("[]")) => format!(
            "java.util.TreeSet<{}>(java.util.Comparator {{ __a, __b -> \
             salvo.__salvoCompare(__a, __b) }}).also {{ __s -> \
             __s.addAll(listOf({})) }}",
            elem(),
            args.join(", ")
        ),
        ("add", Some("SortedSet")) => format!("{}.add({})", a(0), a(1)),
        ("remove", Some("SortedSet")) => format!("{}.remove({})", a(0), a(1)),
        ("contains", Some("SortedSet")) => format!("{}.contains({})", a(0), a(1)),
        ("size", Some("SortedSet")) => format!("{}.size", a(0)),
        // `first()`/`last()` throw on an empty set; Salvo answers `None`.
        ("min", Some("SortedSet")) => format!("{}.firstOrNull()", a(0)),
        ("max", Some("SortedSet")) => format!("{}.lastOrNull()", a(0)),
        ("to_list", Some("SortedSet")) => format!("{}.toMutableList()", a(0)),
        ("to_str", Some("SortedSet")) => {
            format!("{}.joinToString(\", \", \"{{\", \"}}\")", a(0))
        }

        ("sorted_map_of", Some("[]")) | ("mut_sorted_map_of", Some("[]")) => format!(
            "java.util.TreeMap<{}, {}>(java.util.Comparator {{ __a, __b -> \
             salvo.__salvoCompare(__a, __b) }}).also {{ __m -> \
             __m.putAll(listOf({})) }}",
            type_args.first().map(String::as_str).unwrap_or("Any"),
            type_args.get(1).map(String::as_str).unwrap_or("Any"),
            args.join(", ")
        ),
        ("get", Some("SortedMap")) => format!("{}[{}]", a(0), a(1)),
        ("put", Some("SortedMap")) => format!("{}.put({}, {})", a(0), a(1), a(2)),
        ("remove", Some("SortedMap")) => format!("{}.remove({})", a(0), a(1)),
        ("contains_key", Some("SortedMap")) => format!("{}.containsKey({})", a(0), a(1)),
        ("size", Some("SortedMap")) => format!("{}.size", a(0)),
        ("first_key", Some("SortedMap")) => format!("{}.keys.firstOrNull()", a(0)),
        ("last_key", Some("SortedMap")) => format!("{}.keys.lastOrNull()", a(0)),
        ("keys", Some("SortedMap")) => format!("{}.keys.toMutableList()", a(0)),
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
        ("map_of", Some("[]")) | ("mut_map_of", Some("[]")) => format!(
            "linkedMapOf<{}, {}>({})",
            type_args.first().map(String::as_str).unwrap_or("Any"),
            type_args.get(1).map(String::as_str).unwrap_or("Any"),
            args.join(", ")
        ),
        // Absence is `null`, never a default [type-nullable].
        ("get", Some("Map")) => format!("{}[{}]", a(0), a(1)),
        ("put", Some("Map")) => format!("{}.put({}, {})", a(0), a(1), a(2)),
        // [linear-container] `put` answers the previous value on the JVM
        // already, which is exactly `replace`'s contract; the two differ only
        // in what Salvo lets you *do* with the answer.
        ("replace", Some("Map")) => format!("{}.put({}, {})", a(0), a(1), a(2)),
        // `remove` already answers the removed value or `null`.
        ("remove", Some("Map")) => format!("{}.remove({})", a(0), a(1)),
        // [linear-container] The terminal: the values, in insertion order,
        // each handed to the callback. Over a snapshot, as a list's drain is.
        ("drain", Some("Map")) => format!("({}).values.toList().forEach({})", a(0), a(1)),
        ("contains_key", Some("Map")) => format!("{}.containsKey({})", a(0), a(1)),
        ("size", Some("Map")) => format!("{}.size", a(0)),
        // [col-insertion-order] A `LinkedHashMap`'s keys are in insertion
        // order, so the list is that order.
        ("keys", Some("Map")) => format!("{}.keys.toMutableList()", a(0)),
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

        // core.string ----------------------------------------------------
        // [kt-mut-str] `Mut Str` is a `StringBuilder`, so construction is
        // asked for explicitly and a literal stays a `String`.
        ("mut_str", Some("[]")) if args.is_empty() => "StringBuilder()".to_string(),
        // The parts are joined rather than appended one by one, so the same
        // lowering serves a `...spread` (which arrives as `*arr`).
        ("mut_str", Some("[]")) => {
            format!("StringBuilder(listOf({}).joinToString(\"\"))", args.join(", "))
        }
        ("size", Some("Str")) => format!("{}.length", a(0)),
        // The UTF-8 byte count — encoded explicitly, since the JVM's own
        // string length is neither bytes nor code points [fs-token].
        ("byte_size", Some("Str")) => {
            format!("{}.toByteArray(Charsets.UTF_8).size.toLong()", a(0))
        }
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

        // core.bytes -----------------------------------------------------
        // [kt-bytes] [bytes-type] Every call goes to the shipped buffer
        // class, which is `Bytes` and `Mut Bytes` both — so a dropped `Mut`
        // renders nothing and no conversion happens anywhere here.
        ("bytes_of", Some("[]")) => {
            format!("salvo.SalvoBytes.of(arrayOf<UByte>({}))", args.join(", "))
        }
        // `joined()` with no parts is the empty buffer, so one lowering
        // serves `mut_bytes()` and `mut_bytes(a, b)` alike.
        ("mut_bytes", Some("[]")) => format!("salvo.SalvoBytes.joined({})", args.join(", ")),
        ("to_bytes", Some("Str")) => format!("salvo.SalvoBytes.ofUtf8({})", a(0)),
        ("str_of_bytes", Some("Bytes")) => format!("{}.asString()", a(0)),
        ("size", Some("Bytes")) => format!("{}.size", a(0)),
        ("get", Some("Bytes")) => format!("{}.getOrNull({})", a(0), a(1)),
        ("slice", Some("Bytes")) => format!("{}.slice({}, {})", a(0), a(1), a(2)),
        ("index_of", Some("Bytes")) => format!("{}.indexOf({})", a(0), a(1)),
        ("add", Some("Bytes")) => format!("{}.add({})", a(0), a(1)),
        ("append", Some("Bytes")) => format!("{}.append({})", a(0), a(1)),
        // `setAt`, not `set`: the class is Kotlin's, and `set` there is the
        // indexing operator, which would take an `Int` receiver position.
        ("set", Some("Bytes")) => format!("{}.setAt({}, {})", a(0), a(1), a(2)),
        ("clear", Some("Bytes")) => format!("{}.clear()", a(0)),
        ("to_str", Some("Bytes")) => format!("{}.toString()", a(0)),
        ("to_hex", Some("Bytes")) => format!("{}.toHex()", a(0)),

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
        // [async-types] [kt-process] The scheduler's handles: an addr and a pool
        // are ids into it, a reply token is its own class. Their Salvo type
        // *arguments* are dropped by `emit_named_parts` — the effect a
        // `Addr<E>` serves is the checker's business, and the generated message
        // classes carry payload types.
        "Addr" => "Int",
        "Pool" => "Int",
        "Reply" => "salvo.SalvoReply",
        // [kt-bytes] [bytes-type] One class for `Bytes` and `Mut Bytes`, in
        // the root `salvo` package so no emitted file needs an import — see
        // `runtime/bytes.kt` for why neither `List<UByte>` nor `UByteArray`
        // could do the job.
        "Bytes" => "salvo.SalvoBytes",
        "None" => "Unit",
        "Any" => "Any",
        "Nothing" => "Nothing",
        "List" => "List",
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
    let p = |i: usize| params.get(i).map(String::as_str).unwrap_or("TODO()");
    Some(match (handler, member) {
        ("StdOutConsole", "print") => format!("kotlin.io.print({})", p(0)),
        ("DefaultRandom", "random") => "kotlin.random.Random.nextDouble()".to_string(),
        _ => return None,
    })
}
