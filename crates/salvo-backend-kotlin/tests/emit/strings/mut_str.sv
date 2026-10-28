// Strings [str-drop-mut] [kt-mut-str]: `Mut Str` is a `StringBuilder`.

//--- mut_str_lowers_to_a_string_builder
// [kt-mut-str] `Mut Str` maps to `StringBuilder` through the same
// `mut_type_name` hook `Mut List<T>` uses, and every *drop* of the `Mut`
// renders the conversion — which is what `MutableList` never needed, since
// it really is a `List`.
// The whole string surface in one program, shared with the Rust backend's
// `rustc_compiles_and_runs_strings` (`STRING_DEMO` in codegen_tests.rs).
fn shout(text: Str) -> Str {
    return to_upper(text)
}

fn main() [use] {
    use StdConsole()
    // A builder, and the drop that lets it reach the `Str` surface.
    // Construction is asked for; the parts are joined, which is also what
    // makes a `...spread` work [fn-variadic].
    //~ emits: = salvo.std.core.string.Mut_Str(arrayOf<String>("he", "llo"))
    let b = Mut Str("he", "llo")
    append(b, " world")
    // `setCharAt` throws out of range, so `set` guards — and binds its
    // arguments, so a call argument is evaluated once. [col-bounds] The guard
    // answers `false` rather than falling through silently.
    // [platform-value-type] Through std's host code now.
    //~ emits: setPlatform(b, 0, 'H')
    set(b, 0, 'H')
    // [str-drop-mut] The conversion at a call argument, in interpolation,
    // and at an operator — a bare name takes the suffix without parens.
    //~ emits: shout(b.toString())
    println(shout(b))
    //~ emits: sizePlatform(b.toString())
    println("size: ${size(b)}")
    // `copy` of a builder is a new builder, not an alias.
    // [kt-copy] A builder's copy is a new builder: identity would alias the
    // buffer.
    //~ emits: : salvo.platform.std.core.string.MutStr = StringBuilder(b)
    let dup = copy(b)
    append(dup, "!")
    println("${b} / ${dup}")
    clear(dup)
    println("cleared: [${dup}]")

    let sep = ","
    let dash = "-"
    let parts = split("a,b,,c", sep)
    let joined = join(parts, dash)
    println("${size(parts)} ${joined}")

    let trimmed = trim("  pad  ")
    let pa = "pa"
    let ad = "ad"
    println("[${trimmed}] ${starts_with(trimmed, pa)} ${ends_with(trimmed, ad)} ${contains(trimmed, ad)}")
    println(to_lower(shout(trimmed)))

    let hay = "hello"
    let ll = "ll"
    let lo = "lo"
    let at = index_of(hay, ll)
    let nowhere = index_of(hay, pa)
    println("${at ?: unreachable!()} ${nowhere is None}")
    let sub = substr(hay, 1, 3)
    let oob = substr(hay, 1, 9)
    println("${sub ?: unreachable!()} ${oob is None}")
    let n = parse_int("42")
    let bad = parse_int(pa)
    println("${n ?: unreachable!()} ${bad is None}")
    println("${trim_prefix(hay, hay)}${trim_suffix(hay, lo)}|")

    let count = Mut List<Int>()
    for c in iter(hay) {
        add(count, 1)
    }
    println("chars: ${size(count)}")

    // [type-bytes] [type-byte] The text/byte bridge: UTF-8 out, strict UTF-8
    // back in, and an octet that is unsigned on both targets. The buffer is a
    // `Bytes`, not a list of octets, so nothing here boxes on the JVM.
    let bytes = to_bytes("hé")
    let round = str_of_bytes(bytes)
    let broken = mut_bytes()
    add(broken, to_byte(200))
    println("bytes: ${bytes} ${to_hex(bytes)} ${round ?: unreachable!()} ${str_of_bytes(broken) is None}")

    // Operators drop `Mut` too, so equality is by content on both targets.
    let x = Mut Str(pa)
    let y = Mut Str(pa)
    //~ emits: (x.toString()) == (y.toString())
    println("equal: ${x == y}")
}

//--- a_spread_into_a_variadic_intrinsic_spreads
// [fn-variadic] A `...spread` into a variadic intrinsic uses Kotlin's own
// spread operator: splicing the array as one argument builds a collection
// of one array — which kotlinc catches for `listOf`, but *not* for
// `StringBuilder(...)`, where `append(Any?)` accepts it and prints
// `[Ljava.lang.String;@…` [backend-never-wrong].
export fn main() [use] {
    use StdConsole()
    let parts = array_of("a", "b")
    // [col-of-nonempty] `Set` rather than `List`: the list constructors
    // take a *first* element now, which a spread may not supply
    // [fn-variadic], so a lone spread reaches the purely variadic intrinsics.
    // [platform-value-type] `Mut Str` is Salvo now; the spread is the array.
    //~ emits: = salvo.std.core.string.Mut_Str(parts)
    let sb = Mut Str(...parts)
    let xs = Set(...parts)
    println("${sb} ${size(xs)}")
}
