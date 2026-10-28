// [str-drop-mut] [rs-mut-str] `Mut Str` is a `String`
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- mut_str_places

struct Buf canbe Mut {
    text: Mut Str
}

fn grow(s: Mut Str) -> None => s: Mut {
    append(s, "!")
    set(s, 0, 'G')
    clear(s)
    append(s, "grown")
}

fn main() [use] {
    use StdConsole()
    let b = Mut Str("seed")
    grow(b)
    println("${b} ${size(b)}")
    let buf = Mut Buf {text: Mut Str("in-struct")}
    append(buf.text, "!")
    set(buf.text, 0, 'I')
    println("${buf.text}")
}
//= stdout
//= grown 5
//= In-struct!

//--- strings

fn shout(text: Str) -> Str {
    return to_upper(text)
}

fn main() [use] {
    use StdConsole()
    // A builder, and the drop that lets it reach the `Str` surface.
    let b = Mut Str("he", "llo")
    append(b, " world")
    set(b, 0, 'H')
    println(shout(b))
    println("size: ${size(b)}")
    // `copy` of a builder is a new builder, not an alias.
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
    println("equal: ${x == y}")
}
//= stdout
//= HELLO WORLD
//= size: 11
//= Hello world / Hello world!
//= cleared: []
//= 4 a-b--c
//= [pad] true true true
//= pad
//= 2 true
//= el true
//= 42 true
//= hel|
//= chars: 5
//= bytes: [104, 195, 169] 68c3a9 hé true
//= equal: true

// [fn-variadic] A `...spread` into a variadic intrinsic *is* the whole
// collection: `vec![parts]` would be a vector of one vector. Cloned rather
// than moved, matching Kotlin's `listOf(*arr)` — and the string parts are
// borrowed, since `mut_str` reads them.
//--- a_spread_into_a_variadic_intrinsic_is_the_collection

export fn main() [use] {
    use StdConsole()
    let parts = array_of("a", "b")
    let sb = Mut Str(...parts)
    let xs = Set(...parts)
    println("${sb} ${size(xs)}")
}
//= stdout
//= ab 2
