// Strings. `Str` is immutable — every function here returns a new string
// rather than changing its argument — and `canbe Mut` opts it into the
// language-level `Mut` auto-qualifier [type-canbe-mut] for the *building*
// case: a `Mut Str` is a string under construction, which a backend may map
// to a different native type (Kotlin's `StringBuilder`) or to the same one
// (Rust's `String`, where mutability lives in the binding).
//
// A `Mut Str` reaches every function below by *dropping* its `Mut`, which
// on a backend with a separate builder type is a real conversion rather
// than a widening [str-drop-mut] — so the surface is declared once, on
// `Str`, and the three mutators at the bottom are the only functions that
// take a `Mut Str`.
// [platform-value-type] A value platform type (ROADMAP 0.7): the host
// implements it in `lib/platform/std/core/string.{rs,kt}`, and names the mutable
// kind `MutStr`.
export iterable platform type Str canbe Mut : Iter<self, StrYield, Char>

// Builds a mutable string from [parts] (concatenated in order). A string
// *literal* is a `Str`, never a `Mut Str`: mutability is asked for here,
// mirroring [Mut List].
export fn Mut Str(...parts: Str[]) [] -> Mut Str => parts {
    let out = empty_str()
    for part in parts {
        append(out, part)
    }
    return out
}

// An empty string to build: what [Mut Str] starts from.
platform fn empty_str() [] -> Mut Str

// Returns the number of characters in the string
export platform fn size(str: Str) [] -> Int => str

// The number of **bytes** [str] takes in UTF-8 — the unit every byte offset in
// the filesystem surface is counted in [fs-token]: `write` answers one,
// `position` reports one, `open_read_at` takes one. Deliberately a different
// name from [size], which counts characters, because the two differ the
// moment a string leaves ASCII and confusing them silently corrupts an
// offset.
export platform fn byte_size(str: Str) [] -> Long => str

// Returns the character at the given index, or null if it is beyond the length
// of the string.
export platform fn char_at(str: Str, index: Int) [] -> Char? => str, index

// [iter-mint] A fresh iterator over the characters of [str], in order — which is
// what makes every sequence function work on strings.
export fn iter(str: Str) [] -> Mut StrYield => str {
    return Mut StrYield { text: str, at: 0 }
}

// [iter-protocol] The pass a string is walked by: the string plus a position
// in it. A `Str` is immutable, so the iterator holds it and moves the index.
export struct StrYield : Yield<self, Char> canbe Mut {
    // The string being walked.
    text: proj Str,
    // The index of the next character to emit.
    at: Int
}

// Advances the iterator, reporting the character at its position or the end of the
// string.
export fn next(p: Mut StrYield) [] -> Emitted Char | Finished => p: Mut {
    let chr = char_at(p.text, p.at)
    if chr is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(chr)
}

// Splits [str] around every occurrence of [sep]. An empty [str] yields one
// (empty) part; a [sep] that never occurs yields [str] whole. The parts are a
// list of your own to change [str-mut-results].
export platform fn split(str: Str, sep: Str) [] -> Mut List<Str> => str, sep

// The index of the first occurrence of [needle] in [str], or `None` when it
// does not occur. No `-1` sentinel: absence is `None` [type-nullable].
export platform fn index_of(str: Str, needle: Str) [] -> Int? => str, needle

// [str-search] The index of the first occurrence of [needle] at or after
// [from], or `None`. A [from] below 0 searches from the start.
export fn index_of(str: Str, needle: Str, from: Int) [] -> Int? => str, needle, from {
    return index_of_from(str, needle, from)
}

platform fn index_of_from(str: Str, needle: Str, from: Int) [] -> Int? => str, needle, from

// [str-search] The index of the last occurrence of [needle] in [str], or
// `None`.
export platform fn last_index_of(str: Str, needle: Str) [] -> Int? => str, needle

// [str-search] [str] with every occurrence of [from] replaced by [to].
export platform fn replace(str: Str, from: Str, to: Str) [] -> Str => str, from, to

// [str] without leading whitespace
export platform fn trim_start(str: Str) [] -> Str => str

// [str] without trailing whitespace
export platform fn trim_end(str: Str) [] -> Str => str

// Whether [needle] occurs anywhere in [str]
export platform fn contains(str: Str, needle: Str) [] -> Bool => str, needle

// Whether [str] begins with [prefix]
export platform fn starts_with(str: Str, prefix: Str) [] -> Bool => str, prefix

// Whether [str] ends with [suffix]
export platform fn ends_with(str: Str, suffix: Str) [] -> Bool => str, suffix

// [str] without leading and trailing whitespace
export platform fn trim(str: Str) [] -> Str => str

// [str] without a leading [prefix] — unchanged when it does not start with
// one, so the caller needs no `starts_with` test first.
export platform fn trim_prefix(str: Str, prefix: Str) [] -> Str => str, prefix

// [str] without a trailing [suffix] — unchanged when it does not end with
// one.
export platform fn trim_suffix(str: Str, suffix: Str) [] -> Str => str, suffix

// The characters of [str] from [start] (inclusive) to [end] (exclusive), or
// `None` when that range does not lie within the string.
export platform fn substr(str: Str, start: Int, end: Int) [] -> Str? => str, start, end

// [col-span] A half-open range into a sequence: [start] inclusive, [end]
// exclusive. A **struct**, not a tuple — a qualifier cannot apply to a tuple
// [qual-union-arm], and the pair's validity is exactly what [SpanOf] claims.
export struct Span {
    start: Int,
    end: Int
}

// [qual-depend] [col-span] The claim that a span **lies within one
// particular string**: `0 <= start <= end <= size(str)`. Three facts one of
// which relates the pair's own fields — which is why the pair is minted and
// claimed *whole*: parse, don't validate. Tested with the filled block
// (`sp is SpanOf(s)`); the total [substr] below consumes it.
export qualifier SpanOf(str: Str) of Span {
    fn qualifies(span: Span, str: Str) -> Bool {
        return span.start >= 0 && span.start <= span.end && span.end <= size(str)
    }
}

// [col-span] The **total** slice: a span carrying the claim answers the
// substring itself — the optionality was paid once, where the span was
// tested, instead of at every use.
export fn substr(str: Str, at: SpanOf(str) Span) [] -> Str => str, at {
    return substr(str, at.start, at.end) ?: unreachable!()
}

// [str] with every character in upper case
export platform fn to_upper(str: Str) [] -> Str => str

// [str] with every character in lower case
export platform fn to_lower(str: Str) [] -> Str => str

// [parts] concatenated with [sep] between them
export platform fn join(parts: List<Str>, sep: Str) [] -> Str => parts, sep

// The integer [str] spells, or `None` when it does not spell one
export platform fn parse_int(str: Str) [] -> Int? => str

// [str-parse] The 64-bit integer [str] spells, or `None`: an optional sign
// and decimal digits, nothing else (no whitespace, no `_`, no `L`).
export platform fn parse_long(str: Str) [] -> Long? => str

// [str-parse] The number [str] spells in decimal, or `None`. The grammar is
// the same on every backend: an optional sign, digits with an optional
// fraction (`1`, `1.5`, `1.`, `.5`), and an optional exponent (`1e3`,
// `2.5E-4`). No whitespace, no `inf`/`nan`, no hex, no `f`/`d` suffix — the
// hosts' own parsers disagree on all of those, so each host checks this
// grammar before converting.
export platform fn parse_double(str: Str) [] -> Double? => str

// [str-parse] `true` for `"true"`, `false` for `"false"`, `None` otherwise:
// the spellings Salvo writes, and no others (no `yes`, `1`, or other cases).
export fn parse_bool(str: Str) [] -> Bool? => str {
    if str == "true" {
        return true
    }
    if str == "false" {
        return false
    }
    return None
}

// [str-parse] [fn-overload-return] A value of the expected type read from
// [s], or `None`: one name for every scalar, chosen by where the result goes
// (`let port: Int? = from_str(text)`), and the default an implicit
// `?from_str: (s: Str) -> T?` resolves to at each type — the dual of
// `to_str` [interp-to-str].
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

// [str-parse] Types that can be read from a string, as an implicit group:
// `?FromStr<T>` declares `?from_str: (s: Str) -> T?`, the dual of
// `?ToStr<T>`.
export params FromStr<T> {
    fn from_str(s: Str) -> T?
}

// Appends [text] to [str]. The string is mutated, which is why its
// surviving qualifiers are listed exhaustively [deduce-syntax].
export platform fn append(str: Mut Str, text: Str) [] -> None => str: Mut, text

// Replaces the character at [index] with [chr], and answers whether it did.
// Out of range it writes nothing and answers `false` — the string is the
// caller's, and growing it here would make a `set` an `append`.
//
// [col-bounds] Characters, not encoding units, like every other index into a
// string; and the `Bool` is the report every out-of-range write in std makes
// (user decision 2026-09-22).
export platform fn set(str: Mut Str, index: Int, chr: Char) [] -> Bool
=> str: Mut, index, chr

// Removes every character from [str]
export platform fn clear(str: Mut Str) [] -> None => str: Mut

// ===== the string surface written in Salvo [str-salvo] =====

// [str-salvo] Whether [str] has no characters.
export fn is_empty(str: Str) [] -> Bool => str {
    return size(str) == 0
}

// [str-salvo] [str] written [n] times over; empty for `n <= 0`.
export fn repeat(str: Str, n: Int) [] -> Str => str, n {
    let out = Mut Str()
    let i = 0
    while i < n {
        append(out, str)
        i = i + 1
    }
    return out
}

// [str-salvo] The lines of [str]: split at `\n`, with a `\r` before it
// dropped, and no empty last line for a trailing newline [str-mut-results].
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

// [str-salvo] [str] split at the first [sep]: what comes before and after
// it, or `None` when [sep] does not occur.
export fn split_once(str: Str, sep: Str) [] -> (Str, Str)? => str, sep {
    let at = index_of(str, sep)
    if at is Int i {
        let before = substr(str, 0, copy(i)) ?: ""
        let after = substr(str, i + size(sep), size(str)) ?: ""
        return (before, after)
    }
    return None
}

// [str-salvo] [str] split at the last [sep], or `None` when it does not
// occur — `split_last(path, "/")` is a path's directory and name.
export fn split_last(str: Str, sep: Str) [] -> (Str, Str)? => str, sep {
    let at = last_index_of(str, sep)
    if at is Int i {
        let before = substr(str, 0, copy(i)) ?: ""
        let after = substr(str, i + size(sep), size(str)) ?: ""
        return (before, after)
    }
    return None
}
