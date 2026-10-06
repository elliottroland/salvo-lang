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
    spread: Spread,
    // [rs-loc] The call is a return-path forward inside a lending fn's
    // **locator variant**: the lowering answers *position data* instead
    // of a borrow. Only the lenders with a locator form answer; the rest
    // fall through to `None`, which the caller reports
    // [backend-never-wrong].
    loc_lend: bool,
) -> Option<String> {
    if loc_lend && !matches!((name, recv), ("get", Some("List")) | ("get", Some("[]"))) {
        // [rs-loc] No locator form for this intrinsic yet: `None` makes
        // the reference site report it, never a silently-read lowering.
        return None;
    }
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
        // [op-bits] Rust's operators for the three bitwise folds and `!`; the
        // shifts go through `wrapping_shl`/`wrapping_shr`, which take the
        // count modulo the width as the JVM does (a plain `<<` panics in a
        // debug build past the width). `bit_ushr` shifts the unsigned twin.
        // The function form (`i32::wrapping_shl(a, n)`) because a method on a
        // bare literal is E0689, and the value goes through its signed type
        // before the unsigned cast, which would otherwise type a literal `-16`
        // as `u32` (E0600).
        ("bit_and", Some("Int" | "Long")) => format!("(({}) & ({}))", a(0), a(1)),
        ("bit_or", Some("Int" | "Long")) => format!("(({}) | ({}))", a(0), a(1)),
        ("bit_xor", Some("Int" | "Long")) => format!("(({}) ^ ({}))", a(0), a(1)),
        ("bit_not", Some("Int" | "Long")) => format!("(!({}))", a(0)),
        ("bit_shl", Some(t @ ("Int" | "Long"))) => {
            format!("{}::wrapping_shl({}, ({}) as u32)", rs_int(t), a(0), a(1))
        }
        ("bit_shr", Some(t @ ("Int" | "Long"))) => {
            format!("{}::wrapping_shr({}, ({}) as u32)", rs_int(t), a(0), a(1))
        }
        ("bit_ushr", Some(t @ ("Int" | "Long"))) => format!(
            "({u}::wrapping_shr((({}) as {s}) as {u}, ({}) as u32) as {s})",
            a(0),
            a(1),
            s = rs_int(t),
            u = if t == "Long" { "u64" } else { "u32" }
        ),
        // runtime ------------------------------------------------------
        // [runtime-handles] An addr and a pool are `usize` indices here and
        // `i32` in the Salvo runtime.
        ("addr_index", Some("Addr")) | ("pool_index", Some("Pool")) => format!("((({}).clone()) as i32)", a(0)),
        ("addr_of", Some("Int")) | ("pool_of", Some("Int")) => format!("(({}) as usize)", a(0)),
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
        // [waitfor-dedicated] `thread()` is a pool of one, and the only
        // placement the language types `Dedicated` — the qualifier is erased,
        // so what reaches here is a plain pool id.
        ("thread", None) => "crate::scheduler::salvo_thread()".to_string(),
        // [runtime-handles] The core's token inside a reply token, or `None`
        // for one minted on another node.
        ("reply_token", Some("Reply")) => format!("({}).take_local()", a(0)),
        // time -----------------------------------------------------------
        // [time-ticker] [time-clock] [rs-time] The two clock readings, each a
        // plain `i64` of nanoseconds — the whole of what the host contributes
        // to the time surface. Everything else (`Duration`, `between`, the
        // correlation `DefaultClock` keeps) is ordinary Salvo over these.
        // [stream-handle] One counter for every stream table in the process.
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
        ("array_by", Some("Int")) => {
            format!("(0..({})).map({}).collect::<Vec<_>>()", a(0), a(1))
        }

        // core.set -------------------------------------------------------
        // [col-key-eligible] An owned read of a snapshot element — a clone
        // here, where Kotlin can share the reference.
        ("snapshot_at", Some("List")) => {
            format!("{}.get({}).cloned()", a(0), index(1))
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

/// The Rust integer type of a Salvo one.
fn rs_int(salvo: &str) -> &'static str {
    if salvo == "Long" {
        "i64"
    } else {
        "i32"
    }
}
