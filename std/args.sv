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

// ===== the derive =====
//
// [args-derive] A struct describes its command line, and one clause gives
// it a parser (ARG_PARSE.md §10.1, Q7, Q13):
//
//     import args
//
//     // Serve files over HTTP.
//     struct Serve : FromArgs<self> by args {
//         // Address to bind.
//         host: Str = "127.0.0.1"
//         // Port to listen on.
//         Short('p') port: InRange(1, 65535) Int = 8080
//         // Output format.
//         output: "json" | "text" = "text"
//         // Print more.
//         Short('v') verbose: Bool
//         // Files to serve.
//         Positional files: NonEmpty List<Str>
//     }
//
//     fn main() [use] -> None {
//         use StdConsole()
//         use HostArgs()
//         use HostProcess()
//         let opts: Serve = args_or_exit(args())
//         …
//     }
//
// Each field is an option named after it in kebab case (`--max-workers`),
// its doc comment its help, by type: a `Bool` is a switch; a `List<T>` is
// repeated, every value kept (`NonEmpty` requires one); a `T?` may be
// absent; a literal union takes one of its literals; any other type takes
// one value, required unless it has a default. A value is parsed with
// `from_str` at its type, and a qualified type (`InRange(1, 65535) Int`) is
// tested as well. `Short('p')` adds `-p`; `Positional` takes the field by
// position instead, in field order.

import process

// [args-derive] A field's short name: `Short('p') port: Int` is `-p` too.
export comptime qualifier Short(c: Char) of Field

// [args-derive] A field taken by position rather than by name.
export comptime qualifier Positional of Field

// [args-derive] A command's name, when its type's will not do:
// `Named("rm") struct Remove`. Otherwise a command is named after its type
// [args-runtime] (`command_name`).
export comptime qualifier Named(name: Str) of Struct

// [args-derive] What makes `T` parseable from a command line; stamped with
// `: FromArgs<self> by args`. Both members name `T` only in their return
// type, so the expected type picks a struct's [fn-overload-return].
export params FromArgs<T> {
    // What `T`'s command line accepts.
    fn arg_spec() -> Spec<T>
    // `T` from what a scan of its command line found, or every error in it.
    fn from_found(found: Found) -> Ok T | Errs NonEmpty List<ArgError>
}

// [args-derive] Parses `args` into a `T`: every error at once, or the help
// text when `--help` or `-h` was given (an `Errs` holding one
// `HelpRequested`, whatever else is wrong).
export fn parse_args<T>(args: List<Str>, ?FromArgs<T>) [] -> Ok T | Errs NonEmpty List<ArgError> => args {
    let spec: Spec<T> = arg_spec()
    check_spec(spec)
    let found = scan(spec, args)
    if found.help {
        let help: ArgError = HelpRequested { text: help_text(spec) }
        return errs(help)
    }
    return from_found(found)
}

// [args-derive] Parses `args` into a `T`, or ends the process: help is
// printed to standard output with status 0, and errors to standard error,
// with the usage line, with status 2 (AP-9).
export fn args_or_exit<T>(args: List<Str>, ?FromArgs<T>) [Console, Process] -> T => args {
    let parsed: Ok T | Errs NonEmpty List<ArgError> = parse_args(args)
    when parsed {
        is Ok { return parsed }
        is Errs {
            for e in parsed {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                    exit(0)
                }
            }
            for e in parsed {
                printerr("error: ${message(copy(e))}\n")
            }
            let spec: Spec<T> = arg_spec()
            printerr("\n${usage(spec)}\n\nFor more information, try '--help'.\n")
            exit(2)
        }
    }
}

// [args-derive] What the derive cannot see at compile time, checked as the
// parser starts, so a program's first run fails instead: two options with
// one short name, `-h` or `--help` (help's), a required positional after an optional
// one, and a positional after one that takes every word.
fn check_spec<T>(spec: Spec<T>) [] -> None => spec {
    let shorts: Mut List<Char> = Mut List<Char>()
    for o in spec.opts {
        if o.field == "help" {
            unreachable!("`--help` is the parser's own: name the field of `${spec.name}` something else")
        }
        let s = o.short
        if s !is None {
            if s == 'h' {
                unreachable!("`Short('h')` on `${o.field}` takes help's short name")
            }
            if contains(shorts, s) {
                unreachable!("two options of `${spec.name}` have the short name `-${s}`")
            }
            add(shorts, s)
        }
    }
    let optional = false
    let many = false
    for a in spec.args {
        if many {
            unreachable!("positional `${a.field}` follows one that takes every remaining word")
        }
        if a.required && optional {
            unreachable!("required positional `${a.field}` follows an optional one")
        }
        optional = optional || !a.required
        many = a.many
    }
}

// [args-derive] Runs `f`: a block in a place that takes an expression, which
// is how the derive reads a field with statements inside a struct literal.
export fn eval<Q>(f: once () -> Q) [] -> Q => !f {
    return f()
}

// [args-derive] The value found for `field`, parsed, refined by `refine` to
// the field's own type (its qualifiers tested), or `default`. The derive's
// check pass has already reported a value that fails, so a failure here is
// a bug in the derive.
export fn read_value<B, Q>(found: Found, field: Str, default: Q?, refine: (v: B) -> Q?, ?from_str: (s: Str) -> B?) [] -> Q
=>[refine] !v => found, field, !default, refine {
    let s = value_of(found, field)
    if s !is None {
        let v = from_str(s)
        if v !is None {
            let q = refine(v)
            if q !is None {
                return q
            }
        }
        unreachable!("`${field}`'s value was checked")
    }
    if default !is None {
        return default
    }
    unreachable!("`${field}` was checked as required")
}

// [args-derive] The value found for `field`, parsed, or `None`.
export fn read_optional<E>(found: Found, field: Str, ?from_str: (s: Str) -> E?) [] -> E? => found, field {
    let s = value_of(found, field)
    if s is None {
        return None
    }
    return from_str(s)
}

// [args-derive] Every value found for `field`, parsed.
export fn read_all<E>(found: Found, field: Str, ?from_str: (s: Str) -> E?) [] -> List<E> => found, field {
    let out: Mut List<E> = Mut List<E>()
    for s in values_of(found, field) {
        let v = from_str(copy(s))
        if v !is None {
            add(out, v)
        }
    }
    return out
}

// [args-derive] Reports the value found for `field` that does not parse as
// a `B` (`what` says what one is: "an integer"), or that fails `test`, the
// field type's qualifiers.
export fn check_value<B>(errors: Mut List<ArgError>, found: Found, field: Str, shown: Str, what: Str, test: (v: B) -> Bool, ?from_str: (s: Str) -> B?) [] -> None
=>[test] v => errors: Mut, found, field, shown, what, test {
    for s in values_of(found, field) {
        let v = from_str(copy(s))
        if v is None {
            add(errors, Invalid { what: copy(shown), value: copy(s), reason: "expected ${what}" })
        } else {
            if !test(v) {
                add(errors, Invalid { what: copy(shown), value: copy(s), reason: "the value is not accepted" })
            }
        }
    }
}

// [args-derive] How errors name a field: `--max-workers`, or `<files>`.
export fn shown_name(field: Str, positional: Bool) [] -> Str => field {
    if positional {
        return arg_name(field)
    }
    return long_name(field)
}

// [args-derive] `T`'s command line, from its fields: one option or
// positional per field, as [args-derive]'s mapping says.
export comptime fn arg_spec<T is Struct>() [] -> Spec<T> {
    let opts: Mut List<OptSpec> = Mut List<OptSpec>()
    let args: Mut List<ArgSpec> = Mut List<ArgSpec>()
    [for field in T.fields] {
        [if field.kind is Struct] {
            refuse!("a struct-typed field is a group of options, which `args` does not take yet")
        }
        let required = false
        let many = false
        let switch = false
        let default_text: Str? = None
        let choices: Mut List<Str> = Mut List<Str>()
        [if field.type is Bool] {
            switch = true
        } else {
            [if field.type is List<*>] {
                many = true
                [if field.type is NonEmpty List<*>] {
                    required = true
                }
            } else {
                [if field.type is <E> E?] {
                    [if E.kind is Union] {
                        [for lit in E.literals] {
                            add(choices, lit.name)
                        }
                    }
                } else {
                    [if field.default is None] {
                        required = true
                    } else {
                        default_text = "${field.default}"
                    }
                    [if field.kind is Union] {
                        [for lit in field.type.literals] {
                            add(choices, lit.name)
                        }
                    }
                }
            }
        }
        let short: Char? = None
        [if field is <c> Short(c)] {
            short = c
        }
        let opt = OptSpec {
            field: field.name,
            short: short,
            takes_value: !switch,
            repeats: many,
            required: required,
            help: field.docs,
            default_text: default_text,
            choices: choices
        }
        [if field is Positional] {
            [if field is Short] {
                refuse!("a positional has no short name")
            }
            [if field.type is Bool] {
                refuse!("a positional takes a word, and a `Bool` is a switch")
            }
            add(args, ArgSpec { field: copy(opt.field), help: copy(opt.help), required: opt.required, many: opt.repeats })
        } else {
            add(opts, opt)
        }
    }
    let name = command_name(T.name)
    [if T is <n> Named(n)] {
        name = n
    }
    return Spec<T> { name: name, about: T.docs, opts: opts, args: args }
}

// [args-derive] `T` from a scan: first every value checked against its
// field's type (each failure an error, after the scan's own), then, when
// there are none, `T` built from the values and the defaults.
export comptime fn from_found<T is Struct>(found: Found) [] -> Ok T | Errs NonEmpty List<ArgError> => found {
    let errors: Mut List<ArgError> = Mut List<ArgError>()
    for e in found.errors {
        add(errors, copy(e))
    }
    [for field in T.fields] {
        // A switch has no value, and a literal union's was checked by the
        // scan against its choices.
        [if field.type is Bool || field.kind is Union && field.type !is *?] {} else {
            let positional = false
            [if field is Positional] {
                positional = true
            }
            let shown = shown_name(field.name, positional)
            let what = "a valid value"
            [if field.type is Int || field.type is Long || field.type is List<Int> || field.type is List<Long> || field.type is Int? || field.type is Long?] {
                what = "an integer"
            }
            [if field.type is Double || field.type is List<Double> || field.type is Double?] {
                what = "a number"
            }
            [if field.type is <E> List<E>] {
                check_value<E>(errors, found, field.name, shown, what, (_v: E) -> true)
            } else {
                [if field.type is <E> E?] {
                    // A literal's value was checked by the scan, against
                    // its choices, and every word is a `Str`.
                    [if E.kind is Union] {
                        check_value<Str>(errors, found, field.name, shown, what, (_v: Str) -> true)
                    } else {
                        check_value<E>(errors, found, field.name, shown, what, (_v: E) -> true)
                    }
                } else {
                    [if field.type is Str] {
                        check_value<Str>(errors, found, field.name, shown, what, (v: Str) -> v is [field.type])
                    } else {
                        [if field.type is Int] {
                            check_value<Int>(errors, found, field.name, shown, what, (v: Int) -> v is [field.type])
                        } else {
                            [if field.type is Long] {
                                check_value<Long>(errors, found, field.name, shown, what, (v: Long) -> v is [field.type])
                            } else {
                                [if field.type is Double] {
                                    check_value<Double>(errors, found, field.name, shown, what, (v: Double) -> v is [field.type])
                                } else {
                                    check_value<[field.type]>(errors, found, field.name, shown, what, (_v: [field.type]) -> true)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if errors is NonEmpty {
        return Errs(errors)
    }
    return Ok(T {
        [for field in T.fields] {
            [field]: eval<[field.type]>(() -> {
                [if field.type is Bool] {
                    [if field.default is None] {
                        return given(found, field.name)
                    } else {
                        return given(found, field.name) || field.default
                    }
                } else {
                [if field.type is <E> List<E>] {
                    let all = read_all<E>(found, field.name)
                    if all is [field.type] {
                        return all
                    }
                    unreachable!("`${field.name}` was checked as required")
                } else {
                [if field.type is <E> E?] {
                    [if E.kind is Union] {
                        let s = value_of(found, field.name)
                        if s !is None {
                            [for lit in E.literals] {
                                if s == lit.value {
                                    return lit.value
                                }
                            }
                        }
                        [if field.default is None] {
                            return None
                        } else {
                            return field.default
                        }
                    } else {
                    [if field.default is None] {
                        return read_optional<E>(found, field.name)
                    } else {
                        let v = read_optional<E>(found, field.name)
                        if v !is None {
                            return v
                        }
                        return field.default
                    }
                    }
                } else {
                [if field.kind is Union] {
                    let s = value_of(found, field.name)
                    if s !is None {
                        [for lit in field.type.literals] {
                            if s == lit.value {
                                return lit.value
                            }
                        }
                    }
                    [if field.default is None] {
                        unreachable!("`${field.name}` was checked")
                    } else {
                        return field.default
                    }
                } else {
                [if field.type is Str] {
                    return read_value<Str, [field.type]>(found, field.name, field.default, (v: Str) -> if v is [field.type] { v } else { None })
                } else {
                [if field.type is Int] {
                    return read_value<Int, [field.type]>(found, field.name, field.default, (v: Int) -> if v is [field.type] { v } else { None })
                } else {
                [if field.type is Long] {
                    return read_value<Long, [field.type]>(found, field.name, field.default, (v: Long) -> if v is [field.type] { v } else { None })
                } else {
                [if field.type is Double] {
                    return read_value<Double, [field.type]>(found, field.name, field.default, (v: Double) -> if v is [field.type] { v } else { None })
                } else {
                    return read_value<[field.type], [field.type]>(found, field.name, field.default, (v: [field.type]) -> v)
                }
                }
                }
                }
                }
                }
                }
                }
            })
        }
    })
}
