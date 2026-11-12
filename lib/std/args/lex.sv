// Splits a command line into tokens by the usual Unix conventions, without
// knowing which options a program has. It is the first step of the parsers
// that `std.args` derives; most programs never call it, and read their
// options through those parsers instead.
//
// Each argument becomes one token, in order:
//
// - `--name` and `--name=value` are a [LongToken]. An attached value is
//   kept; whether the option may have one is for the parser to decide.
// - `-abc` is a [ShortToken], a bundle of single-letter options, and so is
//   `-p8080`. Only the parser knows whether `-p` takes a value, so the
//   letters are left together for it to split.
// - `--` is an [EndToken], and every argument after it is a [WordToken],
//   however it is spelled (a second `--` included).
// - Anything else is a [WordToken]: a value or a positional argument. A
//   lone `-` (by convention, standard input) is a word, and so is a
//   negative number (`-3`, `-.5`), so `--offset -3` reads `-3` as the
//   value.
//
// There are no abbreviations: `--po` is the option `po`, not a prefix of
// `--port`, so adding an option never changes what an existing script
// means.
//
// ```
// for t in lex(List("-vp8080", "--out=a.txt", "-", "--", "-x")) {
//     when t {
//         is LongToken { println("long ${t.name} = ${t.value ?: "-"}") }   // long out = a.txt
//         is ShortToken { println("short ${t.letters}") }                  // short vp8080
//         is WordToken { println("word ${t.value}") }                      // word -, then word -x
//         is EndToken { println("end") }                                   // end
//     }
// }
// ```
// @compilerrules [args-lex]

// A long option: `--port`, or `--port=8080` with its value attached.
export struct LongToken {
    // The name, without the dashes: `port`.
    name: Str
    // The text after the first `=`, or `None` when there is no `=`
    // (`--x=` has the value `""`, and `--a=b=c` the value `b=c`).
    value: Str?
}

// A bundle of single-letter options after one dash: `-v`, `-jt`, or
// `-p8080`, which may be `-p` with the value `8080`.
export struct ShortToken {
    // The letters after the dash: `jt`, `p8080`.
    letters: Str
}

// A word: an option's value or a positional argument, including a lone
// `-`, a negative number, and anything after `--`.
export struct WordToken {
    // The argument as it was given.
    value: Str
}

// The first `--` on the line: the end of the options.
export struct EndToken {}

// One argument of a command line, as [lex] reads it.
export type Token = LongToken | ShortToken | WordToken | EndToken

// The tokens of a command line, one per argument, in order.
//
// ```
// let tokens = lex(List("--port", "80", "-v"))
// println("${size(tokens)}")                           // 3
// ```
//
// @param args  the arguments, without the program name (as `args()`
//              answers them)
export fn lex(args: List<Str>) [] -> List<Token> => args {
    let out: Mut List<Token> = Mut List<Token>()
    let ended = false
    for arg in args {
        let a = copy(arg)
        when {
            ended {
                add(out, WordToken { value: a })
            }
            a == "--" {
                ended = true
                add(out, EndToken {})
            }
            starts_with(a, "--") {
                let body = trim_prefix(a, "--")
                let parts = split_once(body, "=")
                if parts is None {
                    add(out, LongToken { name: body, value: None })
                } else {
                    let (name, value) = parts
                    add(out, LongToken { name: name, value: value })
                }
            }
            starts_with(a, "-") && a != "-" && !is_number(a) {
                add(out, ShortToken { letters: trim_prefix(a, "-") })
            }
            else {
                add(out, WordToken { value: a })
            }
        }
    }
    return out
}

// Whether `-…` reads as a negative number rather than as short flags: a
// digit, or a `.` and a digit, after the dash.
fn is_number(a: Str) [] -> Bool => a {
    let first = char_at(a, 1)
    if first is None {
        return false
    }
    if is_digit(first) {
        return true
    }
    let second = char_at(a, 2)
    if first == '.' && second !is None {
        return is_digit(second)
    }
    return false
}

fn is_digit(c: Char) [] -> Bool {
    return c >= '0' && c <= '9'
}
