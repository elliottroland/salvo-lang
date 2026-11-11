// Text: the `Str` type and everything a program does with one. Part of
// `std.core`, so every file sees it without an import.
//
// A `Str` is an **immutable** sequence of characters. A function here that
// "changes" a string ([replace], [trim], [to_upper]) answers a new string and
// leaves its argument as it was. To build a string piece by piece, ask for a
// `Mut Str`: `Mut Str(...)` makes one, and [append], [set] and [clear] change it in
// place. A `Mut Str` can be passed wherever a `Str` is expected, so every
// reading function works on it too.
//
// ```
// let line = "  name = Salvo  "
// let kv = split_once(trim(line), " = ")                 // ("name", "Salvo")
// let shout = to_upper("salvo")                          // "SALVO"
// let words = split("a,b,c", ",")                        // [a, b, c]
// let csv = join(words, ";")                             // "a;b;c"
// let out = Mut Str("Hello")
// append(out, ", world")
// println(out)                                           // Hello, world
// ```
//
// **Indexes count characters, not bytes.** [size], [char_at], [substr],
// [index_of] and [set] all count characters, and `for c in text` walks one
// `Char` at a time. [byte_size] is the one function that counts bytes: the
// length of the text in UTF-8, which is the unit file offsets are in. The
// two differ as soon as the text leaves ASCII:
//
// ```
// let s = "héllo"
// println("${size(s)} characters, ${byte_size(s)} bytes")   // 5 characters, 6 bytes
// ```
//
// On the Rust backend a character is a Unicode code point. On the Kotlin
// backend indexes count UTF-16 units, which is the same thing except for
// characters outside the Basic Multilingual Plane (most emoji): there the
// two backends give different sizes and indexes for the same text.
//
// **Absence is `None`.** A search that finds nothing, a slice that does not
// fit and a parse of text that does not spell a number all answer `None`
// rather than `-1`, an empty string or an exception, so the caller has to
// look before using the result:
//
// ```
// let at = index_of("key=value", "=")                    // 3
// if at is Int i {
//     println(substr("key=value", 0, i) ?: "")           // key
// }
// let port: Int? = from_str("8080")                      // 8080
// let bad = parse_int("80x")                             // None
// ```
//
// **Comparing.** `==` compares the text, and `<` orders strings by code
// point, the same order on every backend.

// A value platform type (ROADMAP 0.7): the host implements it in
// `lib/platform/std/core/string.{rs,kt}`, and names the mutable kind
// `MutStr`. `canbe Mut` opts it into the language-level `Mut` auto-qualifier
// for the building case: a backend may map `Mut Str` to a different native
// type (Kotlin's `StringBuilder`) or to the same one (Rust's `String`, where
// mutability lives in the binding). A `Mut Str` reaches every function below
// by dropping its `Mut`, which on a backend with a separate builder type is
// a real conversion rather than a widening, so the surface is declared once,
// on `Str`, and only the mutators take a `Mut Str`.

// A string of characters, immutable once made. A `Mut Str` is a string
// under construction: change it with [append], [set] and [clear], and pass
// it anywhere a `Str` is read. On the Kotlin backend that passing converts
// the builder to a string, which copies its text.
//
// A string literal is a `Str`, never a `Mut Str`; ask for one with
// `Mut Str(...)`.
// `for c in text` walks the characters in order.
//
// ```
// let vowels = 0
// for c in "salvo" {
//     if c == 'a' || c == 'o' {
//         vowels = vowels + 1
//     }
// }
// println("${vowels}")                                   // 2
// ```
// @compilerrules [type-canbe-mut, str-drop-mut, platform-value-type, qual-refn-type]
export iterable platform type Str canbe Mut : Iter<self, StrIter, Char> {
    // Appending to a non-empty string keeps it non-empty, and writing one
    // character in place changes no length. Appending to an empty string
    // establishes nothing: the text appended may be empty too. Appending a
    // character establishes it, as a list's `add` does.
    // @compilerrules [qual-refn-narrow]
    refn append(str: NonEmpty Mut Str, text: Str) => str: +NonEmpty
    refn append(str: Mut Str, char: Char) => str: +NonEmpty
    refn set(str: NonEmpty Mut Str, index: Int, chr: Char) => str: +NonEmpty
}

// Makes a mutable string holding [parts], concatenated in order. `Mut
// Str()` is the empty one.
//
// ```
// let greeting = Mut Str("Hello", ", ", "world")
// append(greeting, "!")
// println(greeting)                                      // Hello, world!
// ```
export fn Mut Str(...parts: Str[]) [] -> Mut Str => parts {
    let out = empty_str()
    for part in parts {
        append(out, part)
    }
    return out
}

// An empty string to build: what [Mut Str] starts from.
platform fn empty_str() [] -> Mut Str

// The number of characters in [str].
//
// ```
// let n = size("héllo")                                  // 5
// ```
export platform fn size(str: Str) [] -> Int => str

// The number of **bytes** [str] takes in UTF-8.
//
// This is the unit every byte offset in the filesystem is counted in: a
// stream's `write` answers one, `position` reports one and `open_read_at`
// takes one. It has a different name from [size], which counts characters,
// because the two differ as soon as the text leaves ASCII, and using one
// where the other was meant puts an offset in the wrong place.
//
// ```
// let n = byte_size("héllo")                             // 6
// ```
// @compilerrules [fs-token, str-byte-size]
export platform fn byte_size(str: Str) [] -> Long => str

// The character at [index], or `None` when [index] is negative or not less
// than [size].
//
// ```
// let c = char_at("abc", 1)                              // 'b'
// let none = char_at("abc", 3)                           // None
// ```
export platform fn char_at(str: Str, index: Int) [] -> Char? => str, index

// The character at [index], with no `None` to handle: [index] is proven to
// lie within [str].
//
// An `Idx(str) Int` is an `Int` the program has tested against this string,
// with `i is Idx(str)`, `assert!(i is Idx(str))` or a loop condition such as
// `while ++i is Idx(str)`, so `0 <= index < size(str)` is known where the
// call is made. An index that has not been tested calls the [char_at] above,
// which answers `Char?`. The read still checks the bound, as that one does.
//
// ```
// let text = "salvo"
// let out = Mut Str()
// let i = -1
// while ++i is Idx(text) {
//     append(out, char_at(text, i))
//     append(out, '.')
// }
// println(out)                                           // s.a.l.v.o.
// ```
// @compilerrules [str-char, col-idx, is-narrow-step]
export fn char_at(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    let char = str.char_at((index: Int))
    assert!(char is Char)
    return char
}

// A fresh iterator over the characters of [str], in order. This is what
// lets a `for` loop and every function of `std.core.seq` walk a string.
//
// ```
// let upper = count(filtering(iter("Hello World"), (c: Char) -> c >= 'A' && c <= 'Z'))   // 2
// ```
// @compilerrules [iter-mint]
export fn iter(str: Str) [] -> Mut StrIter => str {
    return Mut StrIter { text: str, at: 0 }
}

// The iterator [iter] answers: a string and a position in it. It borrows the
// string rather than copying it.
// @compilerrules [iter-protocol]
export struct StrIter : Next<self, Char> canbe Mut {
    // The string being walked.
    text: proj Str,
    // The index of the next character to emit.
    at: Int
}

// Answers the character at the iterator's position and moves past it, or
// answers the end once every character has been emitted.
export fn next(p: Mut StrIter) [] -> Emitted Char | Finished => p: Mut {
    let chr = char_at(p.text, p.at)
    if chr is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(chr)
}

// Splits [str] around every occurrence of [sep], and answers the parts in
// order, without the separators.
//
// An empty [str] answers one empty part, and a [sep] that never occurs
// answers [str] whole. Two separators in a row give an empty part between
// them. The list is a new one, yours to change.
//
// ```
// let parts = split("a,b,,c", ",")                       // [a, b, , c]
// let whole = split("abc", ";")                          // [abc]
// ```
// @compilerrules [str-mut-results]
export platform fn split(str: Str, sep: Str) [] -> Mut List<Str> => str, sep

// The index of the first occurrence of [needle] in [str], or `None` when it
// does not occur.
//
// ```
// let at = index_of("banana", "an")                      // 1
// let none = index_of("banana", "x")                     // None
// ```
// @compilerrules [type-nullable]
export platform fn index_of(str: Str, needle: Str) [] -> Int? => str, needle

// [index_of] from a starting point: the index of the first occurrence of
// [needle] at or after [from], or `None`. A [from] below 0 searches from the
// start.
//
// ```
// let second = index_of("banana", "an", 2)               // 3
// ```
// @compilerrules [str-search]
export fn index_of(str: Str, needle: Str, from: Int) [] -> Int? => str, needle, from {
    return index_of_from(str, needle, from)
}

platform fn index_of_from(str: Str, needle: Str, from: Int) [] -> Int? => str, needle, from

// The index of the last occurrence of [needle] in [str], or `None` when it
// does not occur.
//
// ```
// let at = last_index_of("banana", "an")                 // 3
// ```
// @compilerrules [str-search]
export platform fn last_index_of(str: Str, needle: Str) [] -> Int? => str, needle

// [str] with every occurrence of [from] replaced by [to].
//
// ```
// let s = replace("a-b-c", "-", "+")                     // "a+b+c"
// ```
// @compilerrules [str-search]
export platform fn replace(str: Str, from: Str, to: Str) [] -> Str => str, from, to

// [str] without its leading whitespace.
export platform fn trim_start(str: Str) [] -> Str => str

// [str] without its trailing whitespace.
export platform fn trim_end(str: Str) [] -> Str => str

// Whether [needle] occurs anywhere in [str].
export platform fn contains(str: Str, needle: Str) [] -> Bool => str, needle

// Whether [str] begins with [prefix].
export platform fn starts_with(str: Str, prefix: Str) [] -> Bool => str, prefix

// Whether [str] ends with [suffix].
export platform fn ends_with(str: Str, suffix: Str) [] -> Bool => str, suffix

// [str] without its leading and trailing whitespace.
//
// ```
// let s = trim("  hello \n")                             // "hello"
// ```
export platform fn trim(str: Str) [] -> Str => str

// [str] without a leading [prefix]. When [str] does not start with
// [prefix] it is answered unchanged, so no [starts_with] test is needed
// first.
//
// ```
// let name = trim_prefix("--verbose", "--")              // "verbose"
// let same = trim_prefix("verbose", "--")                // "verbose"
// ```
export platform fn trim_prefix(str: Str, prefix: Str) [] -> Str => str, prefix

// [str] without a trailing [suffix], or unchanged when it does not end
// with one.
//
// ```
// let stem = trim_suffix("notes.txt", ".txt")            // "notes"
// ```
export platform fn trim_suffix(str: Str, suffix: Str) [] -> Str => str, suffix

// The characters of [str] from [start] (inclusive) to [end] (exclusive), or
// `None` when that range does not lie within the string.
//
// `start == end` is an empty string. A negative [start], an [end] past the
// string's [size] or an [end] before [start] is `None`. To slice without
// the optional, prove the range first with [SpanOf] and call the other
// [substr].
//
// ```
// let mid = substr("salvo", 1, 4)                        // "alv"
// let out = substr("salvo", 2, 9)                        // None
// ```
export platform fn substr(str: Str, start: Int, end: Int) [] -> Str? => str, start, end

// A half-open range of indexes: [start] inclusive, [end] exclusive.
//
// On its own a `Span` claims nothing; [SpanOf] is the claim that it fits a
// particular string. It is a struct rather than a pair because a qualifier
// can apply to a struct and not to a tuple.
// @compilerrules [col-span, qual-union-arm]
export struct Span {
    // The first index in the range.
    start: Int,
    // The index just past the range.
    end: Int
}

// The claim that a [Span] lies within one particular string: `0 <= start
// <= end <= size(str)`.
//
// Test it once, with `sp is SpanOf(text)`, and the span then slices that
// string through the [substr] that answers a plain `Str`, with no `None` to
// handle at each use. The span is checked as a whole, because one of the
// facts relates its two fields to each other.
//
// ```
// let text = "salvo"
// let sp = Span { start: 1, end: 4 }
// if sp is SpanOf(text) {
//     println(substr(text, sp))                          // alv
// }
// ```
// @compilerrules [qual-depend, col-span]
export qualifier SpanOf(str: Str) for Span {
    fn qualifies(span: Span, str: Str) -> Bool {
        return span.start >= 0 && span.start <= span.end && span.end <= size(str)
    }
}

// The characters of [str] that a proven span covers. The [SpanOf] claim
// already says the range fits, so the answer is a `Str` rather than a
// `Str?`.
// @compilerrules [col-span]
export fn substr(str: Str, at: SpanOf(str) Span) [] -> Str => str, at {
    return substr(str, at.start, at.end) ?: unreachable!()
}

// [str] with every character in upper case.
export platform fn to_upper(str: Str) [] -> Str => str

// [str] with every character in lower case.
export platform fn to_lower(str: Str) [] -> Str => str

// [parts] concatenated in order, with [sep] between each two.
//
// ```
// let s = join(List("a", "b", "c"), ", ")                // "a, b, c"
// ```
export platform fn join(parts: List<Str>, sep: Str) [] -> Str => parts, sep

// The `Int` [str] spells in decimal, or `None` when it does not spell one:
// an optional sign and digits, nothing else, within `Int`'s range.
//
// ```
// let n = parse_int("-42")                               // -42
// let none = parse_int(" 42")                            // None
// ```
export platform fn parse_int(str: Str) [] -> Int? => str

// The `Long` [str] spells in decimal, or `None`: an optional sign and
// digits, nothing else (no whitespace, no `_`, no `L` suffix).
// @compilerrules [str-parse]
export platform fn parse_long(str: Str) [] -> Long? => str

// The `Double` [str] spells in decimal, or `None`.
//
// The grammar is the same on every backend: an optional sign, digits with
// an optional fraction (`1`, `1.5`, `1.`, `.5`), and an optional exponent
// (`1e3`, `2.5E-4`). No whitespace, no `inf` or `nan`, no hex and no `f` or
// `d` suffix.
//
// ```
// let x = parse_double("2.5e3")                          // 2500.0
// let none = parse_double("NaN")                         // None
// ```
// @compilerrules [str-parse]
export platform fn parse_double(str: Str) [] -> Double? => str

// `true` for `"true"`, `false` for `"false"`, and `None` for anything else:
// the two spellings Salvo writes, and no others (no `yes`, no `1`, no
// `True`).
// @compilerrules [str-parse]
export fn parse_bool(str: Str) [] -> Bool? => str {
    if str == "true" {
        return true
    }
    if str == "false" {
        return false
    }
    return None
}

// The value [s] spells, of the type the result is wanted as, or `None`.
//
// One name reads every scalar: the overload is chosen by where the result
// goes, and each one is the matching `parse_` function (the `Str` one
// answers a copy of [s]). It is also what an implicit `?from_str` parameter
// (the [FromStr] group) finds at each of these types, so generic code can
// read a value of its type parameter. It is the reverse of `to_str`.
//
// ```
// let port: Int? = from_str("8080")                      // 8080
// let ratio: Double? = from_str("0.5")                   // 0.5
// let on: Bool? = from_str("yes")                        // None
// ```
// @compilerrules [str-parse, fn-overload-return, interp-to-str]
export fn from_str(s: Str) [] -> Str? => s {
    return copy(s)
}

export fn from_str(s: Str) [] -> Int? => s {
    return parse_int(s)
}

export fn from_str(s: Str) [] -> Long? => s {
    return parse_long(s)
}

export fn from_str(s: Str) [] -> Double? => s {
    return parse_double(s)
}

export fn from_str(s: Str) [] -> Bool? => s {
    return parse_bool(s)
}

// The types that can be read from a string, as an implicit group:
// `?FromStr<T>` on a function declares `?from_str: (s: Str) -> T?`, found
// at the call site. It is the reverse of `?ToStr<T>`.
//
// ```
// fn read_or<T>(text: Str, fallback: T, ?FromStr<T>) [] -> T => !fallback {
//     return from_str(text) ?: fallback
// }
//
// fn use_it() [] -> Int {
//     return read_or("12", 0)                            // 12
// }
// ```
// @compilerrules [str-parse]
export fns FromStr<T> {
    fn from_str(s: Str) -> T?
}

// Appends [text] to the end of [str], in place.
//
// ```
// let path = Mut Str("/var")
// append(path, "/log")
// println(path)                                          // /var/log
// ```
// @compilerrules [deduce-syntax]
export platform fn append(str: Mut Str, text: Str) [] -> None => str: Mut, text

// Appends the one character [char] to the end of [str], in place, without
// making a string of it first. Afterwards [str] is non-empty, whatever it
// held before.
//
// ```
// let word = Mut Str("ca")
// append(word, 't')
// println(word)                                          // cat
// ```
// @compilerrules [str-char, deduce-syntax]
export fn append(str: Mut Str, char: Char) [] -> None => str: Mut, char {
    // A Salvo overload over a private platform fn, as `index_of` from an
    // offset is: a host module's platform fns have one name each, since
    // Rust has no overloading.
    append_char(str, char)
}

platform fn append_char(str: Mut Str, char: Char) [] -> None => str: Mut, char

// Replaces the character at [index] with [chr], and answers whether it did.
//
// Out of range it writes nothing and answers `false`: the string never
// grows here, which is what [append] is for. Ignore the answer where the
// index is known to be good.
//
// ```
// let word = Mut Str("cat")
// let done = set(word, 0, 'b')                           // true: "bat"
// let missed = set(word, 3, 's')                         // false: still "bat"
// ```
// @compilerrules [col-bounds]
export platform fn set(str: Mut Str, index: Int, chr: Char) [] -> Bool
=> str: Mut, index, chr

// Removes every character from [str], so the same builder can be filled
// again.
export platform fn clear(str: Mut Str) [] -> None => str: Mut

// ===== the string surface written in Salvo =====
// @compilerrules [str-salvo]

// Whether [str] has no characters.
// @compilerrules [str-salvo]
export fn is_empty(str: Str) [] -> Bool => str {
    return size(str) == 0
}

// [str] written [n] times over, or an empty string when [n] is 0 or less.
//
// ```
// let rule = repeat("-", 5)                              // "-----"
// ```
// @compilerrules [str-salvo]
export fn repeat(str: Str, n: Int) [] -> Str => str, n {
    let out = Mut Str()
    let i = 0
    while i < n {
        append(out, str)
        i = i + 1
    }
    return out
}

// The lines of [str], without their line endings.
//
// Lines end at `\n`, and a `\r` just before it is dropped too, so text with
// Windows line endings splits the same way. A trailing newline does not
// make an empty last line. The list is a new one, yours to change.
//
// ```
// let ls = lines("one\r\ntwo\nthree\n")                  // [one, two, three]
// ```
// @compilerrules [str-salvo, str-mut-results]
export fn lines(str: Str) [] -> Mut List<Str> => str {
    let parts = split(str, "\n")
    if size(parts) > 1 && ends_with(str, "\n") {
        let _end = remove_back(parts, 1)
    }
    let out = Mut List<Str>()
    for p in parts {
        add(out, trim_suffix(p, "\r"))
    }
    return out
}

// [str] split at the first [sep]: what comes before it and what comes
// after it, or `None` when [sep] does not occur.
//
// ```
// let kv = split_once("a=b=c", "=")                      // ("a", "b=c")
// if kv is None {
//     println("no separator")
// } else {
//     let (key, value) = kv
//     println("${key} is ${value}")                      // a is b=c
// }
// ```
// @compilerrules [str-salvo]
export fn split_once(str: Str, sep: Str) [] -> (Str, Str)? => str, sep {
    let at = index_of(str, sep)
    if at is Int i {
        let before = substr(str, 0, copy(i)) ?: ""
        let after = substr(str, i + size(sep), size(str)) ?: ""
        return (before, after)
    }
    return None
}

// [str] split at the last [sep]: what comes before it and what comes after
// it, or `None` when [sep] does not occur.
//
// ```
// let cut = split_last("a/b/c.txt", "/")                 // ("a/b", "c.txt")
// ```
// @compilerrules [str-salvo]
export fn split_last(str: Str, sep: Str) [] -> (Str, Str)? => str, sep {
    let at = last_index_of(str, sep)
    if at is Int i {
        let before = substr(str, 0, copy(i)) ?: ""
        let after = substr(str, i + size(sep), size(str)) ?: ""
        return (before, after)
    }
    return None
}

// `Str`'s operators are declared beside the type, as every type's are
// (user decision O1-D, 2026-10-27), and are `intrinsic`: each backend lowers
// them to the host's comparison. `cmp` keeps the code-point contract both
// backends owe: Rust's `String: Ord` is byte-wise UTF-8, which is code-point
// order, while the JVM's `String.compareTo` is UTF-16 code-unit order and
// has to be corrected.
// @compilerrules [op-unique-shape, cmp-groups, backend-intrinsic, col-sorted, kt-ordered]

// The order of two strings, which is what `<`, `>` and sorting use:
// negative when [a] comes first, zero when they are equal, positive when [b]
// comes first. Strings compare character by character, by code point, and a
// string comes before every longer string it starts.
//
// ```
// let before = "apple" < "banana"                        // true
// let upper_first = "Zebra" < "apple"                    // true: 'Z' is a smaller code point
// ```
// @compilerrules [op-unique-shape, cmp-groups, backend-intrinsic, col-sorted, kt-ordered]
export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b
// Whether two strings hold the same characters, which is what `==` means on
// strings.
// @compilerrules [op-unique-shape, backend-intrinsic]
export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b
// A string's text form: the string itself. It is what interpolating a
// string uses, and what an implicit `?ToStr<Str>` finds (the other scalars'
// are in `std.core.basic`).
// @compilerrules [interp-to-str]
export intrinsic fn to_str(value: Str) [] -> Str => value
