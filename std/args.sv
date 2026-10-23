// [cli-args] The program's command-line arguments, as a capability.
//
// Not part of `core`, so it is imported — one line brings it in
// [mod-import-module]:
//
//     import args
//
// A function that reads argv is a function whose answer depends on how the
// process was launched, so — like reading a clock or the console — it is an
// effect, not a global. `Args` is bound once in `main` and threaded to
// whoever needs it [addr-capability]; nothing reads argv behind the back of
// the signature.
//
//     import args
//
//     fn main() [use] -> None {
//         use HostArgs()
//         for a in args() {
//             println(a)
//         }
//     }
//
// `args()` answers the arguments **after the program name** — the one form
// both backends produce identically. On the JVM the entry point's array
// already excludes argv[0]; on Rust `std::env::args()` includes it and the
// host handler drops it. A tool that wants the program name reads it from
// neither here: it is not an argument.
import args.lex

export effect Args {
    // The command-line arguments the process was launched with, program name
    // excluded, in order.
    fn args() -> List<Str>
}

// [cli-args] The host's argv, read from the process the one way each backend
// offers: the JVM `main`'s array (captured as the program starts), and
// Rust's `std::env::args`. Stateless, so sharing it across threads needs no
// synchronization [threadsafe-platform].
export threadsafe platform handler HostArgs() of Args

// ===== parsing a command line =====
//
// [args-runtime] What the derive (`: FromArgs<self> by args`) builds on: a
// command's description (`Spec`), a scan of the command line against it
// (`scan`, answering a `Found`), the readers the derived code takes values
// with, the errors, and the help text. Exported because a stamped body
// resolves its names where it is stamped; it is the derive's layer, not yet
// a surface to build parsers by hand (ARG_PARSE.md Q12 (b)).

// ----- what can go wrong -----

// [args-errors] Every way a command line can fail to parse, and the request
// for help, which is an outcome rather than a side effect so parsing stays
// pure (Q17). A parse reports all of its errors at once.
export type ArgError = UnknownOption | MissingValue | Missing | Invalid | Unexpected | Repeated | HelpRequested

// An option the command does not have: `--prot`, `-x`.
export struct UnknownOption {
    option: Str
}

// An option that takes a value, given none: `--port` at the end of the line.
export struct MissingValue {
    option: Str
}

// A required option or argument that is absent: `--host`, `<files>`.
export struct Missing {
    what: Str
}

// A value the option or argument does not accept, and why.
export struct Invalid {
    what: Str,
    value: Str,
    reason: Str
}

// A word no positional argument takes.
export struct Unexpected {
    value: Str
}

// An option given twice that takes one value.
export struct Repeated {
    option: Str
}

// `--help` or `-h`: the help text, to print and exit successfully.
export struct HelpRequested {
    text: Str
}

// [args-errors] An error as one line, in the user's terms.
export fn message(e: ArgError) [] -> Str => e {
    when e {
        is UnknownOption { return "unknown option '${e.option}'" }
        is MissingValue { return "option '${e.option}' needs a value" }
        is Missing { return "'${e.what}' is required" }
        is Invalid { return "invalid value '${e.value}' for '${e.what}': ${e.reason}" }
        is Unexpected { return "unexpected argument '${e.value}'" }
        is Repeated { return "option '${e.option}' was given more than once" }
        is HelpRequested { return "help was requested" }
    }
}

// ----- what a command accepts -----

// [args-runtime] One option of a command.
export struct OptSpec {
    // The field it fills, which is its key in a `Found`: `max_workers`.
    field: Str,
    // Its short name: `-p`.
    short: Char? = None,
    // Whether it takes a value (`--port 8080`) or is a switch (`--verbose`).
    takes_value: Bool = true,
    // Whether it may be given more than once, each value kept.
    repeats: Bool = false,
    // Whether it must be given.
    required: Bool = false,
    // Its help text: the field's doc comment.
    help: Str? = None,
    // Its default, as help shows it.
    default_text: Str? = None,
    // The values it accepts, when they are a fixed set.
    choices: List<Str> = List<Str>()
}

// [args-runtime] One positional argument of a command, taken in order.
export struct ArgSpec {
    field: Str,
    help: Str? = None,
    // Whether at least one word must fill it.
    required: Bool = true,
    // Whether it takes every remaining word (only the last may).
    many: Bool = false
}

// [args-runtime] What a command accepts, for the type it parses into.
export struct Spec<T> {
    // The command's name, as usage shows it: `serve`.
    name: Str,
    // What the command does: the struct's doc comment.
    about: Str? = None,
    opts: List<OptSpec>,
    args: List<ArgSpec>
}

// The long spelling of a field's option: `max_workers` is `--max-workers`.
export fn long_name(field: Str) [] -> Str => field {
    return "--${replace(field, "_", "-")}"
}

// How usage names a positional: `<files>`.
fn arg_name(field: Str) [] -> Str => field {
    return "<${replace(field, "_", "-")}>"
}

// A command's name from its type's: `Serve` is `serve`, `RemoteAdd`
// `remote-add`, and a namespaced `Remote.Add` the last segment, `add`.
export fn command_name(type_name: Str) [] -> Str => type_name {
    let last = type_name
    let cut = split_last(type_name, ".")
    if cut !is None {
        let (_, tail) = cut
        last = tail
    }
    let out: Mut Str = Mut Str("")
    let first = true
    for c in last {
        let lower = to_lower("${c}")
        if !first && lower != "${c}" {
            append(out, "-")
        }
        append(out, lower)
        first = false
    }
    return out
}

// ----- scanning -----

// [args-runtime] One value a scan found, under the field it fills. A switch
// found is a hit whose value is `""`.
export struct Hit {
    field: Str,
    value: Str
}

// [args-runtime] What a scan of a command line found: the values by field,
// the errors in order, and whether help was asked for.
export struct Found {
    hits: List<Hit>,
    errors: List<ArgError>,
    help: Bool
}

// [args-runtime] Reads `args` against `spec`. Every problem the scan can
// see is collected: an unknown option, a missing or unwanted value, an
// option repeated that takes one value, a value outside its choices, a
// required option or argument absent, a word with nowhere to go. Whether a
// value parses as its field's type is the derived code's check, after.
export fn scan<T>(spec: Spec<T>, args: List<Str>) [] -> Found => spec, args {
    let tokens = lex(args)
    let hits: Mut List<Hit> = Mut List<Hit>()
    let errors: Mut List<ArgError> = Mut List<ArgError>()
    let words: Mut List<Str> = Mut List<Str>()
    let help = false
    let n = size(tokens)
    let i = 0
    while i < n {
        let t = copy(get(tokens, i)!)
        i = i + 1
        when t {
            is EndToken {}
            is WordToken { add(words, t.value) }
            is LongToken {
                if t.name == "help" {
                    help = true
                } else {
                    let o = long_opt(spec, t.name)
                    if o is None {
                        add(errors, UnknownOption { option: "--${t.name}" })
                    } else {
                        let flag = long_name(o.field)
                        let attached = t.value
                        when {
                            !o.takes_value && attached is None {
                                add(hits, Hit { field: copy(o.field), value: "" })
                            }
                            !o.takes_value {
                                add(errors, Invalid { what: flag, value: attached ?: "", reason: "the option takes no value" })
                            }
                            attached !is None {
                                add(hits, Hit { field: copy(o.field), value: attached })
                            }
                            else {
                                let v = word_at(tokens, i)
                                if v is None {
                                    add(errors, MissingValue { option: flag })
                                } else {
                                    add(hits, Hit { field: copy(o.field), value: v })
                                    i = i + 1
                                }
                            }
                        }
                    }
                }
            }
            is ShortToken {
                let letters = t.letters
                let k = 0
                let len = size(letters)
                while k < len {
                    let c = char_at(letters, k)!
                    k = k + 1
                    if c == 'h' {
                        help = true
                    } else {
                        let o = short_opt(spec, c)
                        when {
                            o is None {
                                add(errors, UnknownOption { option: "-${c}" })
                            }
                            !o.takes_value {
                                add(hits, Hit { field: copy(o.field), value: "" })
                            }
                            // The rest of the bundle is the value (`-p8080`),
                            // or the next word is (`-p 8080`).
                            k < len {
                                add(hits, Hit { field: copy(o.field), value: substr(letters, k, len)! })
                                k = len
                            }
                            else {
                                let v = word_at(tokens, i)
                                if v is None {
                                    add(errors, MissingValue { option: "-${c}" })
                                } else {
                                    add(hits, Hit { field: copy(o.field), value: v })
                                    i = i + 1
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Options: repeated, outside their choices, or absent.
    for o in spec.opts {
        let flag = long_name(o.field)
        let count = 0
        for h in hits {
            if h.field == o.field {
                count = count + 1
                if !is_empty(o.choices) && !contains(o.choices, h.value) {
                    add(errors, Invalid { what: copy(flag), value: copy(h.value), reason: "expected one of ${join(o.choices, ", ")}" })
                }
            }
        }
        if count > 1 && !o.repeats {
            add(errors, Repeated { option: copy(flag) })
        }
        if count == 0 && o.required {
            add(errors, Missing { what: copy(flag) })
        }
    }
    // Positionals: one word each, the last taking what is left.
    let w = 0
    let nw = size(words)
    for a in spec.args {
        if a.many {
            if w == nw && a.required {
                add(errors, Missing { what: arg_name(a.field) })
            }
            while w < nw {
                add(hits, Hit { field: copy(a.field), value: copy(get(words, w)!) })
                w = w + 1
            }
        } else {
            when {
                w < nw {
                    add(hits, Hit { field: copy(a.field), value: copy(get(words, w)!) })
                    w = w + 1
                }
                a.required {
                    add(errors, Missing { what: arg_name(a.field) })
                }
                else {}
            }
        }
    }
    while w < nw {
        add(errors, Unexpected { value: copy(get(words, w)!) })
        w = w + 1
    }
    return Found { hits: hits, errors: errors, help: help }
}

// The option spelled `--name`.
fn long_opt<T>(spec: Spec<T>, name: Str) [] -> OptSpec? => spec, name {
    for o in spec.opts {
        if long_name(o.field) == "--${name}" {
            return copy(o)
        }
    }
    return None
}

// The option spelled `-c`.
fn short_opt<T>(spec: Spec<T>, c: Char) [] -> OptSpec? => spec {
    for o in spec.opts {
        if o.short == c {
            return copy(o)
        }
    }
    return None
}

// The word at `i`, which an option's value may be: a word, or nothing
// (an option, `--`, or the end of the line).
fn word_at(tokens: List<ArgToken>, i: Int) [] -> Str? => tokens {
    let t = get(tokens, i)
    if t is None {
        return None
    }
    let w = copy(t)
    if w is WordToken {
        return w.value
    }
    return None
}

// ----- reading what a scan found -----

// [args-runtime] Whether the scan found `field` at all.
export fn given(found: Found, field: Str) [] -> Bool => found, field {
    for h in found.hits {
        if h.field == field {
            return true
        }
    }
    return false
}

// [args-runtime] The value found for `field`, when there was one.
export fn value_of(found: Found, field: Str) [] -> Str? => found, field {
    for h in found.hits {
        if h.field == field {
            return copy(h.value)
        }
    }
    return None
}

// [args-runtime] Every value found for `field`, in order.
export fn values_of(found: Found, field: Str) [] -> List<Str> => found, field {
    let out: Mut List<Str> = Mut List<Str>()
    for h in found.hits {
        if h.field == field {
            add(out, copy(h.value))
        }
    }
    return out
}

// ----- help -----

// [args-help] The usage line: `Usage: serve [OPTIONS] <files>...`.
export fn usage<T>(spec: Spec<T>) [] -> Str => spec {
    let out: Mut Str = Mut Str("Usage: ${spec.name}")
    if !is_empty(spec.opts) {
        append(out, " [OPTIONS]")
    }
    for o in spec.opts {
        if o.required {
            append(out, " ${long_name(o.field)} <${value_name(o.field)}>")
        }
    }
    for a in spec.args {
        let name = arg_name(a.field)
        let shown = if a.many { "${name}..." } else { name }
        append(out, if a.required { " ${shown}" } else { " [${shown}]" })
    }
    return out
}

// The placeholder for an option's value: `--port <PORT>`.
fn value_name(field: Str) [] -> Str => field {
    return to_upper(replace(field, "_", "-"))
}

// [args-help] The help text: what the command does, its usage, its
// arguments and its options, each with its doc comment, choices and
// default, in two aligned columns.
export fn help_text<T>(spec: Spec<T>) [] -> Str => spec {
    let out: Mut Str = Mut Str("")
    if spec.about !is None {
        append(out, "${spec.about}\n\n")
    }
    append(out, "${usage(spec)}\n")
    let lefts: Mut List<Str> = Mut List<Str>()
    let rights: Mut List<Str> = Mut List<Str>()
    for a in spec.args {
        add(lefts, if a.many { "${arg_name(a.field)}..." } else { arg_name(a.field) })
        add(rights, copy(a.help ?: ""))
    }
    let nargs = size(lefts)
    for o in spec.opts {
        let left: Mut Str = Mut Str(if o.short is None { "    " } else { "-${o.short}, " })
        append(left, long_name(o.field))
        if o.takes_value {
            append(left, " <${value_name(o.field)}>")
        }
        add(lefts, copy(left))
        let right: Mut Str = Mut Str(copy(o.help ?: ""))
        if !is_empty(o.choices) {
            append(right, " [possible values: ${join(o.choices, ", ")}]")
        }
        if o.default_text !is None {
            append(right, " [default: ${o.default_text}]")
        }
        add(rights, trim(right))
    }
    add(lefts, "-h, --help")
    add(rights, "Print help")
    let width = 0
    for l in lefts {
        if size(l) > width {
            width = size(l)
        }
    }
    let row = 0
    for l in lefts {
        if row == 0 && nargs > 0 {
            append(out, "\nArguments:\n")
        }
        if row == nargs {
            append(out, "\nOptions:\n")
        }
        let r = get(rights, row)!
        if is_empty(r) {
            append(out, "  ${l}\n")
        } else {
            append(out, "  ${l}${repeat(" ", width - size(l))}  ${r}\n")
        }
        row = row + 1
    }
    return out
}
