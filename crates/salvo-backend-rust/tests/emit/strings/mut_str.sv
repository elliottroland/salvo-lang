// [str-drop-mut] [rs-mut-str] `Mut Str` is a `String` on the Rust backend.

//--- mut_str_is_a_plain_string
// The same source and the same expected stdout as the Kotlin backend's
// `kotlinc_compiles_and_runs_strings`: `Mut Str` is a *different type*
// there (`StringBuilder`) and the same one here, so what the pair asserts
// is that dropping `Mut` costs the program nothing on either target.
//
// [rs-mut-str] `Mut` erases here — `Mut Str` and `Str` are both `String` —
// so a drop renders *nothing*, and the string surface is a set of `&str`
// methods with the char-vs-byte corrections Salvo's semantics need. (That
// `strings.rs` is not emitted for a program that never mutates a string
// stays in `codegen_tests.rs`: a file's absence.)
fn shout(text: Str) -> Str {
    return to_upper(text)
}

fn main() [use] {
    use StdConsole()
    // A builder, and the drop that lets it reach the `Str` surface.
    let b = Mut Str("he", "llo")
    append(b, " world")
    // [platform-value-type] The operations are std's host code
    // (`lib/platform/std/core/string.rs`), called through their wrappers; a
    // `Mut Str` goes as `&mut`.
    //~ emits: crate::std_core_string::set_platform(&mut b, 0i32, 'H')
    set(b, 0, 'H')
    // [str-drop-mut] No conversion anywhere: the argument is borrowed as it
    // stands, and interpolation prints the `String` itself.
    //~ emits: crate::shout(&b)
    println(shout(b))
    println("size: ${size(b)}")
    // `copy` of a builder is a new builder, not an alias.
    let dup = copy(b)
    append(dup, "!")
    //~ emits: format!("{} / {}", b, dup)
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
    //~ emits: crate::std_core_string::index_of_platform(&hay, &ll)
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
    // The parts are borrowed, not moved: a variadic position is untracked by
    // the flow analysis, so an owned `vec![pa]` would move a variable the
    // checker still considers live.
    //~ emits: let mut x: String = crate::std_core_string::Mut_Str(vec![pa.clone()]);
    let x = Mut Str(pa)
    let y = Mut Str(pa)
    println("equal: ${x == y}")
}

//--- a_spread_into_a_variadic_intrinsic_is_the_collection
// [fn-variadic] A `...spread` into a variadic intrinsic *is* the whole
// collection: `vec![parts]` would be a vector of one vector. Cloned rather
// than moved, matching Kotlin's `listOf(*arr)` — and the string parts are
// borrowed, since `mut_str` reads them.
export fn main() [use] {
    use StdConsole()
    let parts = array_of("a", "b")
    //~ emits: let mut sb: String = crate::std_core_string::Mut_Str(parts.clone());
    let sb = Mut Str(...parts)
    // [col-of-nonempty] `Set` rather than `List`: the list constructors
    // now have a *first* parameter, which a spread may not supply
    // [fn-variadic], so a lone spread reaches the purely variadic intrinsics —
    // which is where this lowering lives anyway.
    //~ emits: Set_platform::<String>(parts.clone(), 
    let xs = Set(...parts)
    println("${sb} ${size(xs)}")
}

//--- mut_str_places
// The case the generated trait exists for: `set` through a `&mut String`
// *parameter* and through a **field** projection, next to an owned local.
// An inline `&mut` re-borrow fails on the parameter (E0596), and method
// syntax works for all three. Same source and stdout as the Kotlin
// backend's `kotlinc_compiles_and_runs_mut_str_places`.
struct Buf canbe Mut {
    text: Mut Str
}

// A `Mut Str` parameter is a `&mut String`, reborrowed for the helper.
//~ emits: pub fn grow(s: &mut String)
fn grow(s: Mut Str) -> None => s: Mut {
    append(s, "!")
    //~ emits: crate::std_core_string::set_platform(&mut *s, 0i32, 'G')
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
    //~ emits: crate::std_core_string::set_platform(&mut buf.text, 0i32, 'I')
    set(buf.text, 0, 'I')
    println("${buf.text}")
}
