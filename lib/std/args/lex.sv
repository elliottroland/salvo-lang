// [args-lex] The bottom layer of command-line parsing: a command line split
// into tokens by the usual conventions, with no knowledge of which flags
// exist (the CLI design record §4.0, Q18). `args` reads the tokens against what a
// command accepts; nothing here knows what a flag takes.
//
// - `--name` and `--name=value` are long flags; the value, when attached, is
//   kept, and whether a flag may have one is decided above.
// - `-abc` is a bundle of short flags. `-p8080` is a bundle too: which
//   letters are switches and where a value starts is decided above, since
//   only the command knows whether `-p` takes one.
// - `--` ends the options: every token after it is a word, `--` included
//   only once.
// - A lone `-` is a word (by convention, standard input), and so is a
//   negative number (`-3`, `-.5`), so `--offset -3` reads its value.
// - There are no abbreviations: `--po` is not `--port`, which keeps adding
//   a flag from breaking a script that used a prefix.

// A long flag: `--port` or `--port=8080`.
export struct LongToken {
    // The name, without the dashes: `port`.
    name: Str,
    // The value after `=`, when one is attached.
    value: Str?
}

// A bundle of short flags: `-jt`, or `-p8080`.
export struct ShortToken {
    // The letters after the dash: `jt`, `p8080`.
    letters: Str
}

// A word: a value or a positional argument.
export struct WordToken {
    value: Str
}

// `--`: the end of the options.
export struct EndToken {}

export type Token = LongToken | ShortToken | WordToken | EndToken

// The tokens of a command line, in order.
export fn lex(args: List<Str>) [] -> List<Token> => args {
    let out: Mut List<Token> = Mut List<Token>()
    let ended = false
    for arg in args {
        let a = copy(arg)
        when {
            ended { add(out, WordToken { value: a }) }
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
            else { add(out, WordToken { value: a }) }
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
