// ===== [fn-overload-duplicate] =====
//
// Two declarations with the same parameter *types* are duplicates, whatever
// their parameter names or return types say — nothing could tell them
// apart at a call. Here the same parameters and the same return type: the
// diagnostic is `duplicate fn `f(Int) -> Str``, "Parameter names take no
// part", once. A program of its own (duplicate declarations).
fn f(n: Int) [] -> Str { return "a" }
//~ error[fn-overload-return:duplicate-fn]
fn f(other: Int) [] -> Str { return "b" }

//--- differing types are an overload set
fn f(n: Int) [] -> Int { return 1 }
fn f(s: Str) [] -> Int => !s { return 2 }

//--- return-type overloads
// ===== [fn-overload-return] overloading by return type =====
//
// `read` is the old prelude's `from_str`: std declares its own `from_str`
// for `Str?`, `Int?`, `Long?`, `Double?` and `Bool?`, which these cases
// would then compete with.
fn read(s: Str) [] -> Int? => s { return 1 }
fn read(s: Str) [] -> Bool? => s { return true }

// [fn-overload-return] The same parameters with different return types are
// an overload set, chosen by where the value goes: a `let`, a `return`, an
// argument, a struct field, an assignment, through `!` and `?:`, and into
// the branches of a value `if` (R-3).
struct Holder { b: Bool? }

fn takes_int(n: Int?) [] -> Int => n { return 0 }

fn give() [] -> Bool? { return read("t") }

fn chosen_by_where_it_goes() -> None {
    let _a: Int? = read("1")
    let _b: Bool? = read("t")
    let _c: Int = read("1") ?: unreachable!()
    let d: Bool = read("t") ?: false
    let _e = takes_int(read("1"))
    let _h = Holder { b: read("t") }
    let _g: Int? = None
    _g = read("2")
    let _i: Bool? = if d { read("t") } else { None }
}

// [fn-overload-return] With nothing saying which result is wanted, the call
// is ambiguous, naming the return types (`Int?`, `Bool?`) and that nothing
// here says which result is wanted — never a pick.
fn nothing_says_which() -> None {
    //~ error[fn-overload-return:ambiguous]
    let _x = read("1")
}

// …and an expected type none of them is says so, as the same diagnostic
// ("none of them is a `Str?`").
fn none_is_the_expected() -> None {
    //~ error[fn-overload-return:ambiguous]
    let _x: Str? = read("1")
}

// [fn-overload-return] R-1 (a): a call its parameters already decide is
// decided the same way whatever its expected type, so an annotation never
// moves a call to another overload: `pf(3)` is `pf(Int) -> Str`, and a `let`
// of `Int` is a mismatch. (`pf` was a second `f`.)
fn pf(x: Int) [] -> Str => x { return "s" }
fn pf(x: Int?) [] -> Int => x { return 1 }

fn annotation_never_overrides() -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _c: Int = pf(3)
}

fn annotation_agrees() -> None {
    let _s: Str = pf(3)
}

// [fn-overload-return] R-2: return types one of which is a subtype of the
// other are refused at the declaration, since an expected supertype would
// fit both ("overloaded on its return type alone, and `Int` and `Int?`
// overlap"). (`ov` was a second `f`.)
fn ov(s: Str) [] -> Int => s { return 1 }
//~ error[fn-overload-return:overlap]
fn ov(s: Str) [] -> Int? => s { return None }

// [fn-overload-return] [fn-value-select] A fn passed by name and an
// implicit already compare the whole fn type, result included, so a
// return-type overload is chosen there by the slot it fills.
fn apply(f: (s: Str) -> Bool?, s: Str) [] -> Bool? => f, s { return f(s) }
fn parse<E>(s: Str, ?read: (s: Str) -> E?) [] -> E? => s { return read(s) }

fn by_the_slot() -> None {
    let _a = apply(read, "t")
    let _b: Int? = parse("1")
}
