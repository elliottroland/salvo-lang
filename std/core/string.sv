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
export intrinsic type Str canbe Mut

// Builds a mutable string from [parts] (concatenated in order). A string
// *literal* is a `Str`, never a `Mut Str`: mutability is asked for here,
// mirroring [mut_list_of].
export intrinsic fn mut_str(...parts: Str[]) [] -> Mut Str => parts

// Returns the number of characters in the string
export intrinsic fn size(str: Str) [] -> Int => str

// The number of **bytes** [str] takes in UTF-8 — the unit every byte offset in
// the filesystem surface is counted in [fs-token]: `write` answers one,
// `position` reports one, `open_read_at` takes one. Deliberately a different
// name from [size], which counts characters, because the two differ the
// moment a string leaves ASCII and confusing them silently corrupts an
// offset.
export intrinsic fn byte_size(str: Str) [] -> Long => str

// Returns the character at the given index, or null if it is beyond the length
// of the string.
export intrinsic fn char_at(str: Str, index: Int) [] -> Char? => str, index

// [iter-pass] A fresh pass over the characters of [str], in order — which is
// what makes every sequence function work on strings.
export fn iter(str: Str) [] -> Mut StrYield => str {
    return Mut StrYield { text: str, at: 0 }
}

// [iter-protocol] The pass a string is walked by: the string plus a position
// in it. A `Str` is immutable, so the pass holds it and moves the index.
export struct StrYield : Yield<self, Char> canbe Mut {
    // The string being walked.
    text: proj Str,
    // The index of the next character to emit.
    at: Int
}

// Advances the pass, reporting the character at its position or the end of the
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
// (empty) part; a [sep] that never occurs yields [str] whole.
export intrinsic fn split(str: Str, sep: Str) [] -> List<Str> => str, sep

// The index of the first occurrence of [needle] in [str], or `None` when it
// does not occur. No `-1` sentinel: absence is `None` [type-nullable].
export intrinsic fn index_of(str: Str, needle: Str) [] -> Int? => str, needle

// Whether [needle] occurs anywhere in [str]
export intrinsic fn contains(str: Str, needle: Str) [] -> Bool => str, needle

// Whether [str] begins with [prefix]
export intrinsic fn starts_with(str: Str, prefix: Str) [] -> Bool => str, prefix

// Whether [str] ends with [suffix]
export intrinsic fn ends_with(str: Str, suffix: Str) [] -> Bool => str, suffix

// [str] without leading and trailing whitespace
export intrinsic fn trim(str: Str) [] -> Str => str

// [str] without a leading [prefix] — unchanged when it does not start with
// one, so the caller needs no `starts_with` test first.
export intrinsic fn trim_prefix(str: Str, prefix: Str) [] -> Str => str, prefix

// [str] without a trailing [suffix] — unchanged when it does not end with
// one.
export intrinsic fn trim_suffix(str: Str, suffix: Str) [] -> Str => str, suffix

// The characters of [str] from [start] (inclusive) to [end] (exclusive), or
// `None` when that range does not lie within the string.
export intrinsic fn substr(str: Str, start: Int, end: Int) [] -> Str? => str, start, end

// [str] with every character in upper case
export intrinsic fn to_upper(str: Str) [] -> Str => str

// [str] with every character in lower case
export intrinsic fn to_lower(str: Str) [] -> Str => str

// [parts] concatenated with [sep] between them
export intrinsic fn join(parts: List<Str>, sep: Str) [] -> Str => parts, sep

// The integer [str] spells, or `None` when it does not spell one
export intrinsic fn parse_int(str: Str) [] -> Int? => str

// Appends [text] to [str]. The string is mutated, which is why its
// surviving qualifiers are listed exhaustively [deduce-syntax].
export intrinsic fn append(str: Mut Str, text: Str) [] -> None => str: Mut, text

// Replaces the character at [index] with [chr], and answers whether it did.
// Out of range it writes nothing and answers `false` — the string is the
// caller's, and growing it here would make a `set` an `append`.
//
// [col-bounds] Characters, not encoding units, like every other index into a
// string; and the `Bool` is the report every out-of-range write in std makes
// (user decision 2026-09-22).
export intrinsic fn set(str: Mut Str, index: Int, chr: Char) [] -> Bool
=> str: Mut, index, chr

// Removes every character from [str]
export intrinsic fn clear(str: Mut Str) [] -> None => str: Mut
