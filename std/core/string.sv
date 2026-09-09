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
intrinsic type Str canbe Mut

// Builds a mutable string from [parts] (concatenated in order). A string
// *literal* is a `Str`, never a `Mut Str`: mutability is asked for here,
// mirroring [mutable_list].
intrinsic fn mutable_str(...parts: Str[]) [] -> [parts] Mut Str

// Returns the number of characters in the string
intrinsic fn size(str: Str) [] -> [str] Int

// Returns the character at the given index, or null if it is beyond the length
// of the string.
intrinsic fn char_at(str: Str, index: Int) [] -> [str, index] Char?

// [iter-pass] A fresh pass over the characters of [str], in order — which is
// what makes every sequence function work on strings.
fn iter(str: Str) [] -> [] Mut StrPass {
    return Mut StrPass { text: str, at: 0 }
}

// [iter-protocol] The pass a string is walked by: the string plus a position
// in it. A `Str` is immutable, so the pass holds it and moves the index.
struct StrPass : Yield<self, Char> canbe Mut {
    // The string being walked.
    text: Str,
    // The index of the next character to emit.
    at: Int
}

// Advances the pass, reporting the character at its position or the end of the
// string.
fn next(pass: Mut StrPass) [] -> [pass: Mut] Emitted Char | Finished {
    let chr = char_at(pass.text, pass.at)
    if chr is None {
        return finished()
    }
    pass.at = pass.at + 1
    return emitted(chr)
}

// Splits [str] around every occurrence of [sep]. An empty [str] yields one
// (empty) part; a [sep] that never occurs yields [str] whole.
intrinsic fn split(str: Str, sep: Str) [] -> [str, sep] List<Str>

// The index of the first occurrence of [needle] in [str], or `None` when it
// does not occur. No `-1` sentinel: absence is `None` [type-nullable].
intrinsic fn index_of(str: Str, needle: Str) [] -> [str, needle] Int?

// Whether [needle] occurs anywhere in [str]
intrinsic fn contains(str: Str, needle: Str) [] -> [str, needle] Bool

// Whether [str] begins with [prefix]
intrinsic fn starts_with(str: Str, prefix: Str) [] -> [str, prefix] Bool

// Whether [str] ends with [suffix]
intrinsic fn ends_with(str: Str, suffix: Str) [] -> [str, suffix] Bool

// [str] without leading and trailing whitespace
intrinsic fn trim(str: Str) [] -> [str] Str

// [str] without a leading [prefix] — unchanged when it does not start with
// one, so the caller needs no `starts_with` test first.
intrinsic fn trim_prefix(str: Str, prefix: Str) [] -> [str, prefix] Str

// [str] without a trailing [suffix] — unchanged when it does not end with
// one.
intrinsic fn trim_suffix(str: Str, suffix: Str) [] -> [str, suffix] Str

// The characters of [str] from [start] (inclusive) to [end] (exclusive), or
// `None` when that range does not lie within the string.
intrinsic fn substr(str: Str, start: Int, end: Int) [] -> [str, start, end] Str?

// [str] with every character in upper case
intrinsic fn to_upper(str: Str) [] -> [str] Str

// [str] with every character in lower case
intrinsic fn to_lower(str: Str) [] -> [str] Str

// [parts] concatenated with [sep] between them
intrinsic fn join(parts: List<Str>, sep: Str) [] -> [parts, sep] Str

// The integer [str] spells, or `None` when it does not spell one
intrinsic fn parse_int(str: Str) [] -> [str] Int?

// Appends [text] to [str]. The string is mutated, which is why its
// surviving qualifiers are listed exhaustively [deduce-syntax].
intrinsic fn append(str: Mut Str, text: Str) [] -> [str: Mut, text] None

// Replaces the character at [index] with [chr]. Out of range, it does
// nothing — the string is the caller's, and growing it here would make a
// `set` an `append`.
intrinsic fn set(str: Mut Str, index: Int, chr: Char) [] -> [str: Mut, index, chr] None

// Removes every character from [str]
intrinsic fn clear(str: Mut Str) [] -> [str: Mut] None
