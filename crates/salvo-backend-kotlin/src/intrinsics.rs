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
    let a = |i: usize| args.get(i).map(String::as_str).unwrap_or("kotlin.TODO()");
    Some(match (name, recv) {
        // std.core.basic -----------------------------------------------------
        // [op-convert] The explicit numeric conversions. Kotlin's `toX()`
        // agrees with Rust's `as` case by case: float→int truncates toward
        // zero and saturates (NaN → 0), Long→Int keeps the low 32 bits,
        // Double→Float rounds.
        // [ref-notsame] Same storage: the same object.
        ("same", _) => format!("({} === {})", a(0), a(1)),
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
        // std.core.basic and std.core.string (`cmp`, `eq`), std.core.compare
        // (`hash`) ------------------------------------------------------------
        // [cmp-groups] The canonical `cmp`/`eq`/`hash` at each intrinsic type;
        // `cmp` and `eq` are declared beside their types [unique-shape], and
        // the table keys on name and receiver, not module.
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
        // compiles to a numeric comparison.
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
        // [cmp-hash-values] The wrapping fold a structural `hash` combines
        // its fields with; the JVM's `Long` arithmetic wraps by itself.
        // [op-arith] [kt-op-arith] The arithmetic operators' functions, for a
        // call written out or a function value: an operator resolved to one of
        // these is the IR's own operator, and these are the same text. The
        // JVM's integer arithmetic wraps [op-wrap].
        ("plus", Some("Int" | "Long" | "Float" | "Double")) => format!("(({}) + ({}))", a(0), a(1)),
        ("minus", Some("Int" | "Long" | "Float" | "Double")) => format!("(({}) - ({}))", a(0), a(1)),
        ("times", Some("Int" | "Long" | "Float" | "Double")) => format!("(({}) * ({}))", a(0), a(1)),
        ("divide", Some("Int" | "Long" | "Float" | "Double")) => format!("(({}) / ({}))", a(0), a(1)),
        // [op-pow] [kt-pow] `**`, which is always this call. The receiver key
        // is `base^exponent`, since two overloads share a `Double` base. An
        // immediately invoked lambda takes the operands as its parameters, so
        // each is evaluated once, in order, and a nested `**` in an operand
        // shadows nothing. The integer powers are repeated squaring in the
        // wrapping arithmetic (the low bits of the exact power, as Rust's
        // `wrapping_pow`), and a negative exponent throws, as a zero divisor
        // does; `Double ** Int` is the same squaring, in the order the Rust
        // spelling uses, so the same bits; `Double ** Double` is the host's
        // `Math.pow`.
        ("pow", Some(k @ ("Int^Int" | "Long^Int"))) => {
            let (t, one) = if k.starts_with("Long") { ("Long", "1L") } else { ("Int", "1") };
            format!(
                "({{ __b: {t}, __e: Int -> if (__e < 0) throw ArithmeticException(\"negative exponent in `**`: \" + __e); \
                 var __x = __b; var __n = __e; var __r = {one}; while (__n > 0) {{ if ((__n and 1) == 1) __r *= __x; \
                 __x *= __x; __n = __n shr 1 }}; __r }})({}, {})",
                a(0),
                a(1)
            )
        }
        ("pow", Some("Double^Int")) => format!(
            "({{ __b: Double, __e: Int -> var __x = __b; var __n = kotlin.math.abs(__e.toLong()); var __r = 1.0; \
             while (__n > 0L) {{ if ((__n and 1L) == 1L) __r *= __x; __x *= __x; __n = __n shr 1 }}; \
             if (__e < 0) 1.0 / __r else __r }})({}, {})",
            a(0),
            a(1)
        ),
        ("pow", Some("Double^Double")) => format!("java.lang.Math.pow({}, {})", a(0), a(1)),
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
        // std.core.actor ---------------------------------------------------
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
        // std.core.list ------------------------------------------------------
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
        ("List", _) => format!("listOf<{}>({})", elem(), args.join(", ")),
        ("Mut_List", _) => {
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
        // std.core.set -------------------------------------------------------
        // [col-key-eligible] An owned read of a snapshot element. Identity
        // is the copy here because element/key types are the immutable
        // intrinsic types, which is what `copy` of a generic cannot assume
        // [kt-copy].
        ("snapshot_at", Some("List")) => format!("{}.getOrNull({})", a(0), a(1)),

        // std.core.array -----------------------------------------------------
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


        // std.core.bytes is platform code since 2026-10-04 [platform-value-type]:
        // `lib/platform/std/core/bytes.{rs,kt}`.

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
