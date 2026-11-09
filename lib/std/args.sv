// The program's command-line arguments, and parsers for them derived from a
// struct.
//
// The arguments are an effect, [Args], rather than a global: a function
// whose answer depends on how the process was launched says so in its
// signature, as one that reads the clock or the console does. `main`
// provides the host's arguments with `use HostArgs()`, and `args()` answers
// them as a `List<Str>`, in order, without the program name. Not part of
// `std.core`, so it is imported: `import std.args`.
//
// Most programs do not walk that list themselves. A struct describes the
// command line, one field per option or positional argument, and the
// clause `: FromArgs<self> by args` derives a parser for it at compile
// time. The parser answers the struct itself, so the program reads its
// options as typed fields, and an option added later is parsed, checked
// and listed in the help with nothing else to update. A field's doc comment
// is its help text, and the struct's is the command's description.
//
// ```
// import std.args
// import std.process
//
// // Count the lines of files.
// struct Lines : FromArgs<self> by args {
//     // Skip lines that are empty.
//     Short('b') skip_blank: Bool
//     // Stop after this many lines of each file.
//     Short('n') max: InRange(1, 1000000) Int = 1000
//     // How to print the counts.
//     format: "plain" | "json" = "plain"
//     // A pattern a line must contain; repeatable.
//     grep: List<Str>
//     // The files to read.
//     Positional files: NonEmpty List<Str>
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostArgs()
//     use HostProcess()
//     let opts: Lines = args_or_exit(args())
//     for f in opts.files {
//         println("${f}: at most ${opts.max} lines, as ${opts.format}")
//     }
// }
// ```
//
// `lines -b -n 20 --format json a.txt b.txt` gives a `Lines` whose
// `skip_blank` is `true`, `max` 20, `format` `"json"`, `grep` empty and
// `files` the two names. `lines --help` prints this and exits with
// status 0:
//
// ```text
// Count the lines of files.
//
// Usage: lines [OPTIONS] <files>...
//
// Arguments:
//   <files>...             The files to read.
//
// Options:
//   -b, --skip-blank       Skip lines that are empty.
//   -n, --max <MAX>        Stop after this many lines of each file. [default: 1000]
//       --format <FORMAT>  How to print the counts. [possible values: plain, json] [default: plain]
//       --grep <GREP>      A pattern a line must contain; repeatable.
//   -h, --help             Print help
// ```
//
// **What each field becomes.** Each field is an option named after it in
// kebab case (`skip_blank` is `--skip-blank`), and its type says what it
// takes:
//
// - `Bool` is a switch: `true` when given, otherwise the field's default
//   (`false` when it has none).
// - `T` with a default takes one value, `--max 20`, and may be left out.
//   Without a default it is required.
// - `T?` takes one value and is `None` when left out.
// - `List<T>` may be given any number of times, every value kept in order.
//   `NonEmpty List<T>` must be given at least once.
// - A union of string literals, `"plain" | "json"`, takes one of them. Any
//   other value is an error, and the help lists the choices.
// - A struct that derives `FromArgs` too is a group of options (see
//   [FromArgs]).
//
// A value is read with the `from_str` of the field's type, so `Str`,
// `Int`, `Long`, `Double` and a type of your own with a `from_str` all
// work. A qualified type such as `InRange(1, 1000000) Int` is tested as
// well, the error saying why a value fails (`0 is not in 1..1000000`), and
// in the program the field has that type, with nothing left to check.
// Three qualifiers say what a type cannot: [Short] adds a one-letter name,
// [Positional] takes a field by position instead of by name, and [Named]
// names a command when its struct's name will not do.
//
// **The command line.** The conventions are the common Unix ones (the
// splitting is [std.args.lex]'s):
//
// - `--max 20` and `--max=20`; `-n 20` and `-n20`.
// - One-letter switches bundle: `-bv` is `-b -v`, and `-bn20` is
//   `-b -n 20`.
// - Options and positionals may come in any order. `--` ends the options:
//   every argument after it is positional, however it is spelled.
// - A lone `-` and a negative number are values, so `--offset -3` reads
//   `-3`.
// - Long names are not abbreviated: `--ma` is an unknown option, not
//   `--max`.
// - An option that is not a list may be given once: `-bb` is an error.
// - `--help` and `-h` are the parser's own.
//
// **Errors and help.** A parse reports every problem it finds, not only the
// first: unknown options, missing or unwanted values, repeated options,
// values outside their choices, missing required options and positionals,
// words no positional takes, and values that do not parse or fail their
// qualifiers. [args_or_exit] is what a `main` calls. On `--help` it prints
// the help to standard output and exits with status 0, whatever else is
// wrong; on errors it prints each to standard error, then the usage line,
// and exits with status 2:
//
// ```text
// error: invalid value 'xml' for '--format': expected one of plain, json
// error: '<files>' is required
// error: invalid value '0' for '--max': 0 is not in 1..1000000
//
// Usage: lines [OPTIONS] <files>...
//
// For more information, try '--help'.
// ```
//
// [parse_args] is the same parse without the exit, for a test or for a
// program that reports errors itself: it answers `Ok` with the struct, or
// `Errs` with every [ArgError], a request for help as a [HelpRequested].
// [message] is an error as one line, as above.
//
// **Groups, sub-commands and validation.** A struct-typed field is a group
// of options: flattened into the command's own, all or none, or exactly
// one of several (see [FromArgs]). A field marked [Subcommand] takes a
// sub-command named by the first word, as `git clone` does. A rule between
// options that no type states, such as `--min` at most `--max`, is a
// `validate` function that [parse_valid] and [valid_args_or_exit] call
// after the parse. The Command-line tools topic tells the whole story.
//
// **Mistakes in the declaration.** What the derive can see is wrong is an
// error where the struct says `by args`: a `Bool` positional, `Short` on a
// positional, a struct-typed positional. The rest is checked each time the
// parser starts, so the program's first run stops with a message rather
// than a user's run misbehaving: two options with one long or short name,
// `Short('h')` or a field named `help`, a required positional after an
// optional one, a positional after one that takes every remaining word.
//
// **Below the derive.** [Spec], [OptSpec], [ArgSpec], [GroupSpec],
// [CmdSpec], [Found] and [Hit] are exported because the derived code names
// them; [usage] and [help_text] render a [Spec]. They describe what a
// command accepts and what a command line held, and are not meant for
// writing parsers by hand.
// @compilerrules [cli-args, mod-import-module, net-addr-capability, args-derive]
import std.args.lex

// Reading the program's command-line arguments.
//
// A function that reads them declares `[Args]`; `main` provides the host's
// with `use HostArgs()`, and a test provides a handler that answers the
// list it wants.
//
// ```
// // A greeting for the names on the command line.
// fn greeting() [Args] -> Str {
//     let names = args()
//     if is_empty(names) {
//         return "hello, world"
//     }
//     return "hello, ${join(names, " and ")}"
// }
//
// // Answers a fixed list, as a test would.
// handler FixedArgs(list: List<Str>) for Args {
//     fn args() -> List<Str> {
//         return copy(list)
//     }
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use FixedArgs(List("Ada", "Alan"))
//     println(greeting())                                  // hello, Ada and Alan
// }
// ```
// @compilerknown
export effect Args {
    // The arguments the process was started with, in order, without the
    // program name: `prog a b` answers `["a", "b"]`.
    fn args() -> List<Str>
}

// How each backend reads argv: the JVM entry point's array, captured as the
// program starts, which already leaves out the program name; and Rust's
// `std::env::args`, whose first element, the program name, the handler
// drops. Both then answer the same list.

// The host's command-line arguments, as an [Args] handler: `use HostArgs()`
// in `main`.
//
// It answers the same list on every backend, the program name left out.
// It holds no state, so threads share it without synchronization. During
// development, `salvo run -- a b` passes `a` and `b` to the program.
//
// ```
// fn main() [use] -> None {
//     use StdConsole()
//     use HostArgs()
//     for a in args() {
//         println(a)
//     }
// }
// ```
// @compilerrules [cli-args, platform-threadsafe]
// @compilerknown
export threadsafe platform handler HostArgs() for Args

// ===== parsing a command line =====
//
// What the derive (`: FromArgs<self> by args`) builds on: a command's
// description (`Spec`), a scan of the command line against it (`scan`,
// answering a `Found`), the readers the derived code takes values with, the
// errors, and the help text. The helpers are private: a stamped body
// reaches them through this file. It is the derive's layer, not a surface
// to build parsers by hand.
// @compilerrules [args-runtime, comptime-hygiene]

// ----- what can go wrong -----

// Every way a command line can fail to parse, and the request for help.
//
// [parse_args] answers a list of these, every problem the command line has
// rather than only the first. A request for help is one of them, a
// [HelpRequested] holding the text, so that parsing itself never prints or
// exits: the caller decides what to do, and [args_or_exit] prints and exits
// for it. [message] turns an error into the line a user reads.
//
// ```
// let e: ArgError = Invalid { what: "--port", value: "x", reason: "expected an integer" }
// if e is HelpRequested {
//     print(e.text)
// } else {
//     // error: invalid value 'x' for '--port': expected an integer
//     println("error: ${message(e)}")
// }
// ```
//
// A `validate` function (see [parse_valid]) throws one of these, usually
// an [Invalid], for a rule between options.
// @compilerrules [args-errors]
export type ArgError = UnknownOption | MissingValue | Missing | Invalid | Unexpected | Repeated | Requires | Conflict | MissingOne | UnknownCommand | HelpRequested

// An option the command does not have: `--prot`, or `-x`. [message] reads
// `unknown option '--prot'`.
export struct UnknownOption {
    // The option as given, with its dashes: `--prot`, `-x`.
    option: Str
}

// An option that takes a value, given none: `--port` at the end of the
// line, or followed by another option or `--`. [message] reads `option
// '--port' needs a value`.
export struct MissingValue {
    // The option as given: `--port`, `-p`.
    option: Str
}

// A required option, positional argument or sub-command that is absent.
// [message] reads `'--host' is required`.
export struct Missing {
    // What is missing: `--host`, `<files>`, or `<command>`.
    what: Str
}

// A value an option or positional argument does not accept, and why: it
// does not parse as the field's type, is not one of its choices, fails a
// qualifier, or was given to a switch. [message] reads `invalid value 'x'
// for '--port': expected an integer`.
//
// It is also the error a `validate` function usually throws (see
// [parse_valid]).
export struct Invalid {
    // The option or positional: `--port`, `<files>`.
    what: Str,
    // The value as given.
    value: Str,
    // Why it was refused: `expected an integer`, `0 is not in 1..65535`.
    reason: Str
}

// A word no positional argument takes: one too many, or any word for a
// command with no positionals. [message] reads `unexpected argument 'z'`.
export struct Unexpected {
    // The word.
    value: Str
}

// An option given more than once that is not a list, a switch included:
// `-vv`. [message] reads `option '--verbose' was given more than once`.
export struct Repeated {
    // The option, by its long name: `--verbose`.
    option: Str
}

// An option of a group given without another the group requires:
// `--cert` without `--key`, for a field `tls: Tls?`. [message] reads
// `'--cert' requires '--key'`.
// @compilerrules [args-groups]
export struct Requires {
    // The option that was given: `--cert`.
    option: Str,
    // The required option that was not: `--key`.
    needs: Str
}

// Options of two arms of an exclusive group given together: `--file` with
// `--url`, for a field `source: FromUrl | FromFile`. [message] reads
// `'--file' cannot be used with '--url'`.
// @compilerrules [args-groups]
export struct Conflict {
    // The later option: `--file`.
    option: Str,
    // The option of the arm given first: `--url`.
    other: Str
}

// No arm of an exclusive group given, when the group's union has no
// `None`. [message] reads `one of '--url', '--file' is required`.
// @compilerrules [args-groups]
export struct MissingOne {
    // One option for each arm, the arm's first required one where it has
    // one.
    options: List<Str>
}

// A word where a sub-command goes that names none of them. [message]
// reads `unrecognized command 'push'`.
// @compilerrules [args-commands]
export struct UnknownCommand {
    // The word.
    name: Str
}

// A request for help, `--help` or `-h`, with the help text to print.
//
// When help is asked for, it is the only error a parse answers, whatever
// else is wrong with the line, so a program prints [text] and exits
// successfully. A sub-command's `--help` answers that command's help.
//
// ```
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     Positional from: Str
//     Positional to: Str
// }
//
// fn show_help() [Console] -> None {
//     // `<to>` is missing, but help was asked for, so help is the answer.
//     let r: Ok Copy | Errs NonEmpty List<ArgError> = parse_args(List("a", "--help"))
//     if r is Errs {
//         for e in r {
//             let one = copy(e)
//             if one is HelpRequested {
//                 print(one.text)                      // Copy a file. ...
//             }
//         }
//     }
// }
// ```
export struct HelpRequested {
    // The command's help, as [help_text] writes it, ending in a newline.
    text: Str
}

// An error as one line, in the user's terms, without the `error: `
// prefix [args_or_exit] puts in front of it.
//
// ```
// println(message(Missing { what: "--host" }))         // '--host' is required
// println(message(UnknownOption { option: "-x" }))     // unknown option '-x'
// ```
//
// A [HelpRequested] reads `help was requested`; print its text instead.
// @compilerrules [args-errors]
export fn message(e: ArgError) [] -> Str => e {
    when e {
        is UnknownOption { return "unknown option '${e.option}'" }
        is MissingValue { return "option '${e.option}' needs a value" }
        is Missing { return "'${e.what}' is required" }
        is Invalid { return "invalid value '${e.value}' for '${e.what}': ${e.reason}" }
        is Unexpected { return "unexpected argument '${e.value}'" }
        is Repeated { return "option '${e.option}' was given more than once" }
        is Requires { return "'${e.option}' requires '${e.needs}'" }
        is Conflict { return "'${e.option}' cannot be used with '${e.other}'" }
        is MissingOne { return "one of '${join(e.options, "', '")}' is required" }
        is UnknownCommand { return "unrecognized command '${e.name}'" }
        is HelpRequested { return "help was requested" }
    }
}

// ----- what a command accepts -----

// One option of a command, as the derive describes it: an option field of
// the command's own or of one of its groups.
// @compilerrules [args-runtime]
export struct OptSpec {
    // The field it fills, which is its key in a [Found]: `max_workers`. Its
    // long name is this in kebab case, `--max-workers`.
    field: Str,
    // Its short name: `-p`.
    short: Char? = None,
    // Whether it takes a value (`--port 8080`) or is a switch (`--verbose`).
    takes_value: Bool = true,
    // Whether it may be given more than once, each value kept: a `List`
    // field.
    repeats: Bool = false,
    // Whether it must be given.
    required: Bool = false,
    // Its help text: the field's doc comment.
    help: Str? = None,
    // Its default, as help shows it.
    default_text: Str? = None,
    // The values it accepts, when they are a fixed set.
    choices: List<Str> = List<Str>(),
    // The key of the group it belongs to, a [GroupSpec]'s, or `None` for
    // one of the command's own.
    // @compilerrules [args-groups]
    group: Str? = None
}

// A group of options: a struct-typed field of a command, and how its
// options are taken.
// @compilerrules [args-groups]
export struct GroupSpec {
    // The group's name: the field's (`tls`, `source`), an exclusive arm's
    // after its union's (`source/FromUrl`), and a nested group's after its
    // parent's (`outer/tls`).
    key: Str,
    // How its options are taken: `flat` (`net: Net`, the command's own),
    // `together` (`tls: Tls?`, all or none), `one` (`source: FromUrl |
    // FromFile`, exactly one arm) or `arm` (one arm of a `one`).
    kind: Str,
    // Whether a `one` group must have an arm given: its union has no
    // `None`.
    required: Bool = false,
    // Its heading in help: the field's doc comment, or the struct's.
    heading: Str? = None
}

// One positional argument of a command. Positionals take the words of the
// line in the order they are declared.
// @compilerrules [args-runtime]
export struct ArgSpec {
    // The field it fills; usage shows it as `<files>`.
    field: Str,
    // Its help text: the field's doc comment.
    help: Str? = None,
    // Whether at least one word must fill it.
    required: Bool = true,
    // Whether it takes every remaining word (only the last may).
    many: Bool = false
}

// What a command accepts: its name and description, its options,
// positionals, groups and sub-commands. `T` is the struct it parses into.
//
// The derive writes one for each struct that declares `: FromArgs<self> by
// args`, and the struct's `arg_spec()` answers it. A program reads it to
// print the help or usage itself:
//
// ```
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     // Overwrite what is there.
//     Short('f') force: Bool
//     Positional from: Str
//     Positional to: Str
// }
//
// fn describe() [Console] -> None {
//     let spec: Spec<Copy> = arg_spec()
//     println("${spec.name}: ${size(spec.opts)} option(s), ${size(spec.args)} positional(s)")
//     // copy: 1 option(s), 2 positional(s)
// }
// ```
// @compilerrules [args-runtime]
export struct Spec<T> {
    // The command's name, as usage shows it: `serve`.
    name: Str,
    // What the command does: the struct's doc comment.
    about: Str? = None,
    // Its options, in field order, a group's where the group's field is.
    opts: List<OptSpec>,
    // Its positionals, in field order.
    args: List<ArgSpec>,
    // Its groups of options, nested ones included.
    groups: List<GroupSpec> = List<GroupSpec>(),
    // The sub-commands, when a field takes one.
    // @compilerrules [args-commands]
    commands: List<CmdSpec> = List<CmdSpec>(),
    // Whether a sub-command must be given: the field's union has no `None`.
    command_required: Bool = false,
    // How many fields take a sub-command. More than one stops the parser as
    // it starts.
    command_fields: Int = 0
}

// A sub-command of a command: its name and what it does, as help lists it
// under `Commands:`.
// @compilerrules [args-commands]
export struct CmdSpec {
    // The word that names it: `clone`, its struct's name in kebab case, or
    // the one [Named] gives.
    name: Str,
    // What it does: its struct's doc comment.
    about: Str? = None
}

// Adds a group's options to a command's: `child`, the spec of
// the struct a field of kind `kind` holds, under `key`. Its own groups nest
// under `key`; an empty arm (`struct FromStdin {}`) is chosen by a switch
// named after it, `--from-stdin`.
// @compilerrules [args-groups]
fn merge_group<C>(opts: Mut List<OptSpec>, groups: Mut List<GroupSpec>, child: Spec<C>, key: Str, kind: Str, heading: Str?) [] -> None
=> opts: Mut, groups: Mut, child, key, kind, !heading {
    if !is_empty(child.args) {
        unreachable!("`${key}` is a group of options, and `${child.name}` takes positionals")
    }
    add(groups, GroupSpec { key: copy(key), kind: copy(kind), heading: heading ?: copy(child.about) })
    for g in child.groups {
        add(groups, GroupSpec { key: "${key}/${g.key}", kind: copy(g.kind), required: g.required, heading: copy(g.heading) })
    }
    for o in child.opts {
        let group = if o.group is None { copy(key) } else { "${key}/${o.group}" }
        add(opts, OptSpec {
            field: copy(o.field),
            short: o.short,
            takes_value: o.takes_value,
            repeats: o.repeats,
            required: o.required,
            help: copy(o.help),
            default_text: copy(o.default_text),
            choices: copy(o.choices),
            group: group
        })
    }
    if kind == "arm" && is_empty(child.opts) {
        add(opts, OptSpec { field: replace(command_name(child.name), "-", "_"), takes_value: false, help: copy(child.about), group: copy(key) })
    }
}

// The exclusive group for a union-typed field: its key, and
// whether an arm is required (no `None` arm).
// @compilerrules [args-groups]
fn add_one(groups: Mut List<GroupSpec>, key: Str, required: Bool, heading: Str?) [] -> None => groups: Mut, key, !heading {
    add(groups, GroupSpec { key: copy(key), kind: "one", required: required, heading: heading })
}

// Whether the scan found any of `spec`'s options: whether a
// group (`tls: Tls?`) or an exclusive arm was given.
// @compilerrules [args-groups]
fn any_given<C>(found: Found, spec: Spec<C>) [] -> Bool => found, spec {
    if is_empty(spec.opts) {
        return given(found, replace(command_name(spec.name), "-", "_"))
    }
    for o in spec.opts {
        if given(found, o.field) {
            return true
        }
    }
    return false
}

// The long spelling of a field's option: `max_workers` is `--max-workers`.
fn long_name(field: Str) [] -> Str => field {
    return "--${replace(field, "_", "-")}"
}

// How usage names a positional: `<files>`.
fn arg_name(field: Str) [] -> Str => field {
    return "<${replace(field, "_", "-")}>"
}

// A command's name from its type's: `Serve` is `serve`, `RemoteAdd`
// `remote-add`, and a namespaced `Remote.Add` the last segment, `add`.
fn command_name(type_name: Str) [] -> Str => type_name {
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

// One value a scan of a command line found, under the field it fills. A
// switch that was given is a hit whose value is `""`.
// @compilerrules [args-runtime]
export struct Hit {
    // The field: `port`.
    field: Str,
    // The value as given: `8080`.
    value: Str
}

// What a scan of a command line against a [Spec] found: the values by
// field, the errors, and whether help was asked for. The derived
// `from_found` reads it to build the struct; the values are still text
// here, parsed only then.
// @compilerrules [args-runtime]
export struct Found {
    // Every value found: the options' in the order of the line, then the
    // positionals'.
    hits: List<Hit>,
    // The errors the scan could see, in the order it found them.
    errors: List<ArgError>,
    // Whether `--help` or `-h` was given.
    help: Bool,
    // The sub-command the first word named, if any.
    // @compilerrules [args-commands]
    command: Str? = None,
    // The arguments after the sub-command's name, which are its own.
    rest: List<Str> = List<Str>(),
    // How usage names this command: `git remote`.
    // @compilerrules [args-commands]
    path: Str = ""
}

// Reads `args` against `spec`. Every problem the scan can
// see is collected: an unknown option, a missing or unwanted value, an
// option repeated that takes one value, a value outside its choices, a
// required option or argument absent, a word with nowhere to go. Whether a
// value parses as its field's type is the derived code's check, after.
// @compilerrules [args-runtime]
fn scan<T>(spec: Spec<T>, args: List<Str>) [] -> Found => spec, args {
    let tokens = lex(args)
    let hits: Mut List<Hit> = Mut List<Hit>()
    let errors: Mut List<ArgError> = Mut List<ArgError>()
    let words: Mut List<Str> = Mut List<Str>()
    let help = false
    let command: Str? = None
    let failed_command = false
    let rest: Mut List<Str> = Mut List<Str>()
    let n = size(tokens)
    let i = 0
    while i < n {
        let t = copy(get(tokens, i) ?: unreachable!())
        i = i + 1
        when t {
            is EndToken {}
            is WordToken {
                // [args-commands] The first word names the sub-command, and
                // everything after it is the sub-command's.
                if !is_empty(spec.commands) && command is None && !failed_command {
                    if has_command(spec, t.value) {
                        command = copy(t.value)
                        while i < n {
                            add(rest, copy(get(args, i) ?: unreachable!()))
                            i = i + 1
                        }
                    } else {
                        add(errors, UnknownCommand { name: copy(t.value) })
                        failed_command = true
                        i = n
                    }
                } else {
                    add(words, t.value)
                }
            }
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
                    let c = char_at(letters, k) ?: unreachable!()
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
                                add(hits, Hit { field: copy(o.field), value: substr(letters, k, len) ?: unreachable!() })
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
            // [args-groups] Inside a group that is all-or-none (or one arm
            // of an exclusive group), required only once the group is used.
            let why = unmet_group(spec, hits, o.group)
            when {
                why is Str { add(errors, Requires { option: why, needs: copy(flag) }) }
                live(spec, hits, o.group) { add(errors, Missing { what: copy(flag) }) }
                else {}
            }
        }
    }
    // [args-groups] Exclusive groups: at most one arm, and one when required.
    for g in spec.groups {
        if g.kind == "one" && live(spec, hits, g.key) {
            let first: Str? = None
            let names: Mut List<Str> = Mut List<Str>()
            for a in spec.groups {
                if a.kind == "arm" && parent_is(a.key, g.key) {
                    add(names, first_option(spec, a.key))
                    let here = given_in(spec, hits, a.key)
                    if here !is None {
                        if first is None {
                            first = here
                        } else {
                            add(errors, Conflict { option: here, other: copy(first) })
                        }
                    }
                }
            }
            if first is None && g.required {
                add(errors, MissingOne { options: names })
            }
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
                add(hits, Hit { field: copy(a.field), value: copy(get(words, w) ?: unreachable!()) })
                w = w + 1
            }
        } else {
            when {
                w < nw {
                    add(hits, Hit { field: copy(a.field), value: copy(get(words, w) ?: unreachable!()) })
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
        add(errors, Unexpected { value: copy(get(words, w) ?: unreachable!()) })
        w = w + 1
    }
    if !is_empty(spec.commands) && command is None && !failed_command && spec.command_required {
        add(errors, Missing { what: "<command>" })
    }
    return Found { hits: hits, errors: errors, help: help, command: command, rest: rest, path: copy(spec.name) }
}

// The group a key nests in: `source` for `source/FromUrl`.
// @compilerrules [args-groups]
fn parent_key(key: Str) [] -> Str? => key {
    let cut = split_last(key, "/")
    if cut is None {
        return None
    }
    let (parent, _) = cut
    return parent
}

// Whether `key` is `parent`'s direct member.
// @compilerrules [args-groups]
fn parent_is(key: Str, parent: Str) [] -> Bool => key, parent {
    let p = parent_key(key)
    if p is None {
        return false
    }
    return p == parent
}

// Whether an option's group is exactly `key`.
fn in_group(group: Str?, key: Str) [] -> Bool => group, key {
    if group is None {
        return false
    }
    return group == key
}

// The first option given in the group `key` (or nested in
// it), as it was spelled long.
// @compilerrules [args-groups]
fn given_in<T>(spec: Spec<T>, hits: List<Hit>, key: Str) [] -> Str? => spec, hits, key {
    for h in hits {
        for o in spec.opts {
            if o.field == h.field && o.group !is None {
                let g = copy(o.group)
                if g == key || starts_with(g, "${key}/") {
                    return long_name(o.field)
                }
            }
        }
    }
    return None
}

// How an arm is named when none was given: its first
// required option, or its first.
// @compilerrules [args-groups]
fn first_option<T>(spec: Spec<T>, key: Str) [] -> Str => spec, key {
    for o in spec.opts {
        if o.required && in_group(o.group, key) {
            return long_name(o.field)
        }
    }
    for o in spec.opts {
        if in_group(o.group, key) {
            return long_name(o.field)
        }
    }
    return copy(key)
}

// Whether options in group `key` count: every all-or-none
// group and exclusive arm it is in was used. The command's own always do.
// @compilerrules [args-groups]
fn live<T>(spec: Spec<T>, hits: List<Hit>, key: Str?) [] -> Bool => spec, hits, key {
    let k = key
    while k !is None {
        let here = copy(k)
        for g in spec.groups {
            if g.key == here && (g.kind == "together" || g.kind == "arm") && given_in(spec, hits, here) is None {
                return false
            }
        }
        k = parent_key(here)
    }
    return true
}

// For a required option of group `key` that is missing: the
// option given in its innermost conditional group, which is what requires
// it, when that group is used and every group around it counts.
// @compilerrules [args-groups]
fn unmet_group<T>(spec: Spec<T>, hits: List<Hit>, key: Str?) [] -> Str? => spec, hits, key {
    let k = key
    while k !is None {
        let here = copy(k)
        for g in spec.groups {
            if g.key == here && (g.kind == "together" || g.kind == "arm") {
                let why = given_in(spec, hits, here)
                if why !is None && live(spec, hits, parent_key(here)) {
                    return why
                }
                return None
            }
        }
        k = parent_key(here)
    }
    return None
}

// Whether `spec` has a sub-command named `name`.
// @compilerrules [args-commands]
fn has_command<T>(spec: Spec<T>, name: Str) [] -> Bool => spec, name {
    for c in spec.commands {
        if c.name == name {
            return true
        }
    }
    return false
}

// Whether the scan found the sub-command `name`.
// @compilerrules [args-commands]
fn is_command(found: Found, name: Str) [] -> Bool => found, name {
    let c = found.command
    if c is None {
        return false
    }
    return c == name
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
fn word_at(tokens: List<Token>, i: Int) [] -> Str? => tokens {
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

// Whether the scan found `field` at all.
// @compilerrules [args-runtime]
fn given(found: Found, field: Str) [] -> Bool => found, field {
    for h in found.hits {
        if h.field == field {
            return true
        }
    }
    return false
}

// The value found for `field`, when there was one.
// @compilerrules [args-runtime]
fn value_of(found: Found, field: Str) [] -> Str? => found, field {
    for h in found.hits {
        if h.field == field {
            return copy(h.value)
        }
    }
    return None
}

// Every value found for `field`, in order.
// @compilerrules [args-runtime]
fn values_of(found: Found, field: Str) [] -> List<Str> => found, field {
    let out: Mut List<Str> = Mut List<Str>()
    for h in found.hits {
        if h.field == field {
            add(out, copy(h.value))
        }
    }
    return out
}

// ----- help -----

// A command's usage line: `Usage: serve [OPTIONS] --port <PORT> <files>...`.
//
// It names the command, `[OPTIONS]` when it has any, each required option
// with its value, and the positionals in order: `<name>` for a required
// one, `[<name>]` for an optional one, and `...` after one that takes every
// remaining word; then `<COMMAND>` or `[COMMAND]` for a sub-command. An
// option required only inside a group is not listed. [args_or_exit] prints
// it after the errors, without a newline at the end.
//
// ```
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     Short('f') force: Bool
//     Positional from: Str
//     Positional to: Str
// }
//
// fn show_usage() [Console] -> None {
//     let spec: Spec<Copy> = arg_spec()
//     println(usage(spec))                             // Usage: copy [OPTIONS] <from> <to>
// }
// ```
// @compilerrules [args-help]
export fn usage<T>(spec: Spec<T>) [] -> Str => spec {
    let out: Mut Str = Mut Str("Usage: ${spec.name}")
    if !is_empty(spec.opts) {
        append(out, " [OPTIONS]")
    }
    let none: List<Hit> = List<Hit>()
    for o in spec.opts {
        // [args-groups] An option a group requires is not required of the
        // command.
        if o.required && live(spec, none, o.group) {
            append(out, " ${long_name(o.field)} <${value_name(o.field)}>")
        }
    }
    for a in spec.args {
        let name = arg_name(a.field)
        let shown = if a.many { "${name}..." } else { name }
        append(out, if a.required { " ${shown}" } else { " [${shown}]" })
    }
    if !is_empty(spec.commands) {
        append(out, if spec.command_required { " <COMMAND>" } else { " [COMMAND]" })
    }
    return out
}

// The placeholder for an option's value: `--port <PORT>`.
fn value_name(field: Str) [] -> Str => field {
    return to_upper(replace(field, "_", "-"))
}

// A command's help, as `--help` prints it: what the command does, its
// [usage] line, then its positionals under `Arguments:`, its sub-commands
// under `Commands:`, and its options under `Options:`, in two aligned
// columns.
//
// Each row shows the doc comment of its field (several lines joined into
// one), the choices of a literal union (`[possible values: json, text]`)
// and the default (`[default: 8080]`). `-h, --help` closes the command's
// own options; each group's options follow under the group's heading. The
// text ends in a newline.
//
// ```
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     // Overwrite what is there.
//     Short('f') force: Bool
//     Positional from: Str
//     Positional to: Str
// }
//
// fn show_help() [Console] -> None {
//     let spec: Spec<Copy> = arg_spec()
//     print(help_text(spec))
// }
// ```
//
// prints
//
// ```text
// Copy a file.
//
// Usage: copy [OPTIONS] <from> <to>
//
// Arguments:
//   <from>
//   <to>
//
// Options:
//   -f, --force  Overwrite what is there.
//   -h, --help   Print help
// ```
// @compilerrules [args-help]
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
        add(rights, one_line(a.help))
    }
    let nargs = size(lefts)
    // [args-commands] The sub-commands, each with what it does.
    for c in spec.commands {
        add(lefts, copy(c.name))
        add(rights, one_line(c.about))
    }
    let ncommands = size(lefts) - nargs
    // [args-groups] Each option under its section: the command's own under
    // `Options:`, a group's with a heading under it, in field order.
    let sections: Mut List<Str> = Mut List<Str>()
    let section_of: Mut List<Str> = Mut List<Str>()
    for o in spec.opts {
        let heading = section(spec, o.group)
        if heading != "" && !contains(sections, heading) {
            add(sections, copy(heading))
        }
        add(section_of, heading)
    }
    for o in spec.opts {
        let left: Mut Str = Mut Str(if o.short is None { "    " } else { "-${o.short}, " })
        append(left, long_name(o.field))
        if o.takes_value {
            append(left, " <${value_name(o.field)}>")
        }
        add(lefts, copy(left))
        let right: Mut Str = Mut Str(one_line(o.help))
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
    let help_row = size(lefts) - 1
    let rows: Mut List<Int> = Mut List<Int>()
    if nargs > 0 {
        append(out, "\nArguments:\n")
        let i: Int = 0
        while i < nargs {
            add(rows, i)
            i = i + 1
        }
        help_rows(out, lefts, rights, rows, width)
    }
    if ncommands > 0 {
        append(out, "\nCommands:\n")
        let commands: Mut List<Int> = Mut List<Int>()
        let c: Int = 0
        while c < ncommands {
            add(commands, nargs + c)
            c = c + 1
        }
        help_rows(out, lefts, rights, commands, width)
    }
    let first_opt = nargs + ncommands
    let mine: Mut List<Int> = Mut List<Int>()
    let k = 0
    for s in section_of {
        if s == "" {
            add(mine, first_opt + k)
        }
        k = k + 1
    }
    add(mine, help_row)
    append(out, "\nOptions:\n")
    help_rows(out, lefts, rights, mine, width)
    for heading in sections {
        let these: Mut List<Int> = Mut List<Int>()
        let j = 0
        for s in section_of {
            if s == heading {
                add(these, first_opt + j)
            }
            j = j + 1
        }
        append(out, "\n${heading}:\n")
        help_rows(out, lefts, rights, these, width)
    }
    return out
}

// A doc comment as one row of help: its lines joined by spaces.
// @compilerrules [args-help]
fn one_line(text: Str?) [] -> Str => text {
    if text is None {
        return ""
    }
    return replace(text, "\n", " ")
}

// The rows `rows` of help's two columns, the left padded to `width`.
fn help_rows(out: Mut Str, lefts: List<Str>, rights: List<Str>, rows: List<Int>, width: Int) [] -> None => out: Mut, lefts, rights, rows {
    for i in rows {
        let l = get(lefts, i) ?: unreachable!()
        let r = get(rights, i) ?: unreachable!()
        if is_empty(r) {
            append(out, "  ${l}\n")
        } else {
            append(out, "  ${l}${repeat(" ", width - size(l))}  ${r}\n")
        }
    }
}

// The heading an option's group is shown under: its
// outermost group's doc comment, its last `.` dropped, or `""` for the
// command's own options.
// @compilerrules [args-groups]
fn section<T>(spec: Spec<T>, group: Str?) [] -> Str => spec, group {
    if group is None {
        return ""
    }
    let top = copy(group)
    let cut = split_once(top, "/")
    if cut !is None {
        let (head, _) = cut
        top = head
    }
    for g in spec.groups {
        if g.key == top && g.heading !is None {
            return trim_suffix(one_line(g.heading), ".")
        }
    }
    return ""
}

// ===== the derive =====
//
// `: FromArgs<self> by args` stamps the two comptime fns at the end of this
// file, `arg_spec` and `from_found`, for a struct: one walks its fields
// into a `Spec`, the other reads a `Found` into the struct. The functions
// below are what a program calls with the result. The module doc describes
// what each field becomes; LANGUAGE_SPEC.md has the rules.
// @compilerrules [args-derive]

import std.process
import std.throw

// Gives an option a one-letter name besides its long one.
//
// With `Short('p') port: Int`, `-p 80`, `-p80` and `--port 80` all set
// `port`. One-letter switches bundle (`-vf` is `-v -f`), and a letter that
// takes a value takes the rest of the bundle or the next word (`-vp80`).
// Two options with one letter, or `Short('h')` (help's), stop the parser
// as it starts; a positional cannot have one.
//
// ```
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     // Overwrite what is there.
//     Short('f') force: Bool
//     // Copies of the old file to keep.
//     Short('k') keep: Int = 0
//     Positional from: Str
//     Positional to: Str
// }
// ```
// @compilerrules [args-derive]
export comptime qualifier Short(c: Char) for Field

// Takes a field by position rather than by name.
//
// The words of the line that are not options or their values fill the
// positional fields in the order they are declared. A `Str` (or any type
// with a `from_str`) takes one word, and is required unless it has a
// default; `T?` takes one word if there is one; `List<T>` takes every word
// left, and `NonEmpty List<T>` at least one. A required positional may not follow
// an optional one, and nothing may follow one that takes every word. A
// positional is neither a `Bool` nor a group, and has no [Short] name.
// Usage shows positionals as `<from>`, `[<backup>]` and `<files>...`.
//
// ```
// // Remove files.
// struct Remove : FromArgs<self> by args {
//     // What to remove.
//     Positional first: Str
//     // More to remove.
//     Positional more: List<Str>
// }
// ```
//
// `remove a b c` gives `first` `"a"` and `more` `["b", "c"]`; `remove` alone
// is the error `'<first>' is required`.
// @compilerrules [args-derive]
export comptime qualifier Positional for Field

// Makes a field a sub-command: its type is a union of command structs,
// each deriving `FromArgs`, and the first word of the line names the one
// it holds.
//
// Everything after that word is the sub-command's own, so the parent's
// options come before it (`git -v clone`, not `git clone -v`). With `?` on
// the union the sub-command may be left out; otherwise none is the error
// `'<command>' is required`, and a word that names none is an
// [UnknownCommand]. A command is named after its struct in kebab case
// (`RemoteAdd` is `remote-add`, and a namespaced `Remote.Add` is `add`) or
// by [Named]. Help lists the commands with their structs' doc comments, and
// `git clone --help` is `clone`'s help, its usage naming the path
// (`Usage: git clone …`). Sub-commands nest.
//
// ```
// // Manage a repository.
// struct Git : FromArgs<self> by args {
//     // Print more.
//     Short('v') verbose: Bool
//     Subcommand command: Clone | Init
// }
//
// // Copy a repository.
// struct Clone : FromArgs<self> by args {
//     // Where from.
//     Positional url: Str
//     // How many commits to fetch.
//     depth: Int?
// }
//
// // Create a repository.
// struct Init : FromArgs<self> by args {
//     // Where; the current directory if left out.
//     Positional dir: Str?
// }
//
// fn run(git: Git) [Console] -> None => git {
//     let c = git.command
//     when c {
//         is Clone { println("clone ${c.url}, depth ${c.depth ?: 0}") }
//         is Init { println("init ${c.dir ?: "."}") }
//     }
// }
// ```
//
// `git -v clone URL --depth 1` gives a `Git` whose `command` is a `Clone`.
// Since the dispatch is an exhaustive `when`, a command added later is a
// compile-time error at every dispatch that does not handle it. A struct
// with a sub-command takes no positionals, and has one `Subcommand` field
// at most.
// @compilerrules [args-commands]
export comptime qualifier Subcommand for Field

// Names a command, when its struct's name will not do. Otherwise a command
// is named after its struct in kebab case: `Serve` is `serve`, `RemoteAdd`
// is `remote-add`, and a namespaced `Remote.Add` is `add`.
//
// The name is what usage and help show, and the word that chooses a
// sub-command.
//
// ```
// // Remove a file.
// Named("rm")
// struct Remove : FromArgs<self> by args {
//     Positional path: Str
// }
// ```
//
// Help for `Remove` reads `Usage: rm <path>`.
// @compilerrules [args-derive, args-runtime]
export comptime qualifier Named(name: Str) for Struct

// What makes a struct `T` parseable from a command line. A struct gets it
// by declaring `: FromArgs<self> by args`, which derives both functions
// from its fields at compile time; [parse_args] and the other parse
// functions find them at the call site, picked by the type the call is
// expected to answer.
//
// The module doc lists what each field becomes. A field whose type is a
// struct that derives `FromArgs` too is a **group** of that struct's
// options, shown in help under the field's doc comment or the struct's:
//
// ```
// // Network options.
// struct Net : FromArgs<self> by args {
//     // Address to bind.
//     host: Str = "127.0.0.1"
//     Short('p') port: Int = 8080
// }
//
// struct Tls : FromArgs<self> by args {
//     // Certificate file.
//     cert: Str
//     // Private key file.
//     key: Str
// }
//
// struct FromUrl : FromArgs<self> by args {
//     url: Str
// }
//
// struct FromFile : FromArgs<self> by args {
//     file: Str
// }
//
// // Read standard input.
// struct FromStdin : FromArgs<self> by args {}
//
// // Fetch a document.
// struct Fetch : FromArgs<self> by args {
//     net: Net
//     // TLS (give both or neither).
//     tls: Tls?
//     // Where to read.
//     source: FromUrl | FromFile | FromStdin
// }
// ```
//
// - `net: Net` is **flattened**: `--host` and `--port` are `fetch`'s own
//   options. A name both structs use stops the parser as it starts.
// - `tls: Tls?` is **all or none**: `None` when neither `--cert` nor `--key`
//   is given, and `--cert` alone is the error `'--cert' requires '--key'`.
// - `source: FromUrl | FromFile | FromStdin` is **exactly one** arm, chosen
//   by whose options appear: `--url u --file f` is the error `'--file'
//   cannot be used with '--url'`. An arm with no fields is a switch named
//   after it, `--from-stdin`. With no `None` in the union an arm is
//   required: `one of '--url', '--file', '--from-stdin' is required`. An arm
//   whose options are all optional could never be chosen, and stops the
//   parser as it starts.
//
// Groups nest. A field marked [Subcommand] takes a sub-command instead.
// @compilerrules [args-derive, fn-overload-return]
export fns FromArgs<T> {
    // What `T`'s command line accepts.
    fn arg_spec() -> Spec<T>
    // `T` from what a scan of its command line found, or every error in it.
    fn from_found(found: Found) -> Ok T | Errs NonEmpty List<ArgError>
}

// Parses a command line into a `T`, a struct that derives `FromArgs`, and
// answers it or every error in the line. It never prints and never exits
// (only a mistake in `T`'s declaration stops it, as the module doc says).
//
// `T` is the type the call is expected to answer, so the result is bound
// with its type written out. When `--help` or `-h` was given the answer is
// an `Errs` holding one [HelpRequested], whatever else is wrong. Otherwise
// the errors the scan sees come first (unknown, repeated and missing
// options and positionals), then values that do not parse as their field's
// type or fail its qualifiers.
//
// ```
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     // Overwrite what is there.
//     Short('f') force: Bool
//     Positional from: Str
//     Positional to: Str
// }
//
// fn try_it() [Console] -> None {
//     let r: Ok Copy | Errs NonEmpty List<ArgError> = parse_args(List("-f", "a.txt", "b.txt"))
//     when r {
//         is Ok { println("${r.from} -> ${r.to}, force ${r.force}") }   // a.txt -> b.txt, force true
//         is Errs {
//             for e in r {
//                 println("error: ${message(copy(e))}")
//             }
//         }
//     }
// }
// ```
//
// @param args  the arguments, without the program name: `args()`, or a
//              list a test writes
// @compilerrules [args-derive]
export fn parse_args<T>(args: List<Str>, ?FromArgs<T>) [] -> Ok T | Errs NonEmpty List<ArgError> => args {
    return parse_named(args, None)
}

// [parse_args] for a command whose usage and help name it after a prefix:
// with the prefix `"git"`, a command `clone` reads `Usage: git clone …`.
//
// The derived parser calls it for a sub-command, with the parent's path,
// so a program rarely does.
//
// ```
// // Copy a repository.
// struct Clone : FromArgs<self> by args {
//     Positional url: Str
// }
//
// fn clone_help() [Console] -> None {
//     let r: Ok Clone | Errs NonEmpty List<ArgError> = parse_named(List("--help"), "git")
//     if r is Errs {
//         for e in r {
//             let one = copy(e)
//             if one is HelpRequested {
//                 print(one.text)                      // ... Usage: git clone <url> ...
//             }
//         }
//     }
// }
// ```
//
// @param prefix  what usage shows before the command's name, or `None`
// @compilerrules [args-commands]
export fn parse_named<T>(args: List<Str>, prefix: Str?, ?FromArgs<T>) [] -> Ok T | Errs NonEmpty List<ArgError> => args, !prefix {
    let base: Spec<T> = arg_spec()
    check_spec(base)
    let spec = if prefix is None { base } else { Spec<T> { ...base, name: "${prefix} ${base.name}" } }
    let found = scan(spec, args)
    if found.help {
        let help: ArgError = HelpRequested { text: help_text(spec) }
        return errs(help)
    }
    let r: Ok T | Errs NonEmpty List<ArgError> = from_found(found)
    // A sub-command's help is the answer, whatever else is wrong.
    if r is Errs {
        for e in r {
            let one: ArgError = copy(e)
            if one is HelpRequested {
                let help: ArgError = one
                return errs(help)
            }
        }
    }
    return r
}

// Parses a command line into a `T`, a struct that derives `FromArgs`, or
// ends the process: what a tool's `main` calls.
//
// On a good command line it answers the struct. When `--help` or `-h` was
// given it prints the help to standard output and exits with status 0. On
// errors it prints each to standard error as `error: …`, then the usage
// line and a pointer to `--help`, and exits with status 2.
//
// ```
// import std.process
//
// // Copy a file.
// struct Copy : FromArgs<self> by args {
//     // Overwrite what is there.
//     Short('f') force: Bool
//     Positional from: Str
//     Positional to: Str
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostArgs()
//     use HostProcess()
//     let c: Copy = args_or_exit(args())
//     println("${c.from} -> ${c.to}")
// }
// ```
//
// `copy -x a` prints this to standard error and exits with status 2:
//
// ```text
// error: unknown option '-x'
// error: '<to>' is required
//
// Usage: copy [OPTIONS] <from> <to>
//
// For more information, try '--help'.
// ```
//
// [parse_args] is the same parse without printing or exiting.
//
// @param args  the arguments, without the program name: usually `args()`
// @compilerrules [args-derive]
export fn args_or_exit<T>(args: List<Str>, ?FromArgs<T>) [Console, Process] -> T => args {
    let parsed: Ok T | Errs NonEmpty List<ArgError> = parse_args(args)
    when parsed {
        is Ok { return parsed }
        is Errs { exit_with<T>(parsed) }
    }
}

// Parses a command line into a `T` and checks it with a `validate`
// function, answering what `validate` answers: usually a qualified type
// that the rest of the program can demand, such as `Valid Range`.
//
// A rule between options that no field's type states, such as `--min` at
// most `--max`, belongs in `validate`. It takes the parsed struct, throws
// an [ArgError] (usually an [Invalid]) when the rule is broken, and is
// found at the call site, so `T` comes from its parameter. It runs only on
// a command line that parsed. The answer is `Ok` with what it returned,
// every parse error, or the one error it threw; help is answered as
// [parse_args] answers it.
//
// ```
// import std.throw
//
// struct Range : FromArgs<self> by args {
//     // Smallest.
//     min: Int = 0
//     // Largest.
//     max: Int = 10
// }
//
// qualifier Valid for Range
//
// fn validate(o: Range) [Throw<ArgError>] -> +Valid Range => !o {
//     if o.min > o.max {
//         let e: ArgError = Invalid { what: "--min", value: "${o.min}", reason: "must not exceed --max" }
//         throw(e)
//     }
//     return o
// }
//
// fn width(r: Valid Range) [] -> Int => r {
//     return r.max - r.min
// }
//
// fn show(line: List<Str>) [Console] -> None => line {
//     let r: Ok Valid Range | Errs NonEmpty List<ArgError> = parse_valid(line)
//     when r {
//         is Ok { println("width ${width(r)}") }        // width 7, for --min 3
//         is Errs {
//             for e in r {
//                 // invalid value '30' for '--min': must not exceed --max, for --min 30
//                 println("error: ${message(copy(e))}")
//             }
//         }
//     }
// }
// ```
//
// Since `width` takes a `Valid Range`, a program that parses with
// [parse_args] and forgets to validate does not compile.
//
// @param validate  found at the call site: checks the parsed struct
// @compilerrules [args-validate]
export fn parse_valid<T, V>(args: List<Str>, ?FromArgs<T>, ?validate: (o: T) [Throw<ArgError>] -> V) [] -> Ok V | Errs NonEmpty List<ArgError>
=>[validate] !o => args {
    let parsed: Ok T | Errs NonEmpty List<ArgError> = parse_args(args)
    when parsed {
        is Errs { return parsed }
        is Ok {
            let checked = try {
                validate(parsed)
            }
            when checked {
                is Ok { return Ok(checked) }
                is Thrown {
                    let e: ArgError = checked
                    return errs(e)
                }
            }
        }
    }
}

// [parse_valid], ending the process on help or errors as [args_or_exit]
// does: what the `main` of a tool with a `validate` calls.
//
// ```
// import std.process
// import std.throw
//
// struct Range : FromArgs<self> by args {
//     min: Int = 0
//     max: Int = 10
// }
//
// qualifier Valid for Range
//
// fn validate(o: Range) [Throw<ArgError>] -> +Valid Range => !o {
//     if o.min > o.max {
//         let e: ArgError = Invalid { what: "--min", value: "${o.min}", reason: "must not exceed --max" }
//         throw(e)
//     }
//     return o
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostArgs()
//     use HostProcess()
//     let r: Valid Range = valid_args_or_exit(args())
//     println("${r.max - r.min}")
// }
// ```
//
// `range --min 9 --max 4` prints `error: invalid value '9' for '--min':
// must not exceed --max` and the usage line to standard error, and exits
// with status 2.
//
// @param validate  found at the call site: checks the parsed struct
// @compilerrules [args-validate]
export fn valid_args_or_exit<T, V>(args: List<Str>, ?FromArgs<T>, ?validate: (o: T) [Throw<ArgError>] -> V) [Console, Process] -> V
=>[validate] !o => args {
    let parsed: Ok V | Errs NonEmpty List<ArgError> = parse_valid<T, V>(args)
    when parsed {
        is Ok { return parsed }
        is Errs { exit_with<T>(parsed) }
    }
}

// Ends the process for a parse that did not answer a value:
// help to standard output with status 0, or every error to standard error,
// then the usage line, with status 2.
// @compilerrules [args-derive]
fn exit_with<T>(errors: NonEmpty List<ArgError>, ?FromArgs<T>) [Console, Process] -> Never => errors {
    for e in errors {
        let one = copy(e)
        if one is HelpRequested {
            print(one.text)
            exit(0)
        }
    }
    for e in errors {
        printerr("error: ${message(copy(e))}\n")
    }
    let spec: Spec<T> = arg_spec()
    printerr("\n${usage(spec)}\n\nFor more information, try '--help'.\n")
    exit(2)
}

// What the derive cannot see at compile time, checked as the
// parser starts, so a program's first run fails instead: two options with
// one short name, `-h` or `--help` (help's), a required positional after an optional
// one, and a positional after one that takes every word.
// @compilerrules [args-derive]
fn check_spec<T>(spec: Spec<T>) [] -> None => spec {
    let shorts: Mut List<Char> = Mut List<Char>()
    let fields: Mut List<Str> = Mut List<Str>()
    for o in spec.opts {
        // [args-groups] A flattened group's options are the command's own,
        // unprefixed, so one name twice is a clash.
        if contains(fields, o.field) {
            unreachable!("two options of `${spec.name}` are spelled `${long_name(o.field)}`")
        }
        add(fields, copy(o.field))
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
    // [args-groups] An exclusive arm is chosen by its options, so one whose
    // options are all optional is chosen by nothing when none is given.
    for g in spec.groups {
        if g.kind == "arm" {
            let count = 0
            let required = false
            for o in spec.opts {
                if in_group(o.group, g.key) {
                    count = count + 1
                    required = required || o.required
                }
            }
            if count > 0 && !required && !(count == 1 && !opt_takes_value(spec, g.key)) {
                unreachable!("every option of `${g.key}` is optional, so nothing chooses that arm: make one required")
            }
        }
    }
    // [args-commands] A word is a sub-command or a positional, not either.
    if !is_empty(spec.commands) && !is_empty(spec.args) {
        unreachable!("`${spec.name}` takes a sub-command and positionals: whether a word is one or the other would depend on its spelling")
    }
    if spec.command_fields > 1 {
        unreachable!("`${spec.name}` has more than one `Subcommand` field")
    }
    let names: Mut List<Str> = Mut List<Str>()
    for c in spec.commands {
        if contains(names, c.name) {
            unreachable!("two sub-commands of `${spec.name}` are named `${c.name}`")
        }
        add(names, copy(c.name))
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

// Whether the one option of group `key` takes a value: an empty arm's
// switch does not.
fn opt_takes_value<T>(spec: Spec<T>, key: Str) [] -> Bool => spec, key {
    for o in spec.opts {
        if in_group(o.group, key) {
            return o.takes_value
        }
    }
    return true
}

// Runs `f`: a block in a place that takes an expression, which
// is how the derive reads a field with statements inside a struct literal.
// @compilerrules [args-derive]
fn eval<Q>(f: once () -> Q) [] -> Q => !f {
    return f()
}

// The value found for `field`, parsed, refined by `refine` to
// the field's own type (its qualifiers tested), or `default`. The derive's
// check pass has already reported a value that fails, so a failure here is
// a bug in the derive.
// @compilerrules [args-derive]
fn read_value<B, Q>(found: Found, field: Str, default: Q?, refine: (v: B) -> Q?, ?from_str: (s: Str) -> B?) [] -> Q
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

// The value found for `field`, parsed, or `None`.
// @compilerrules [args-derive]
fn read_optional<E>(found: Found, field: Str, ?from_str: (s: Str) -> E?) [] -> E? => found, field {
    let s = value_of(found, field)
    if s is None {
        return None
    }
    return from_str(s)
}

// Every value found for `field`, parsed.
// @compilerrules [args-derive]
fn read_all<E>(found: Found, field: Str, ?from_str: (s: Str) -> E?) [] -> List<E> => found, field {
    let out: Mut List<E> = Mut List<E>()
    for s in values_of(found, field) {
        let v = from_str(copy(s))
        if v !is None {
            add(out, v)
        }
    }
    return out
}

// Reports the value found for `field` that does
// not parse as a `B` (`what` says what one is: "an integer"), or that fails
// `test`, the field type's qualifiers, which answers why (`None` when the
// value is accepted).
// @compilerrules [args-derive, args-explain]
fn check_value<B>(errors: Mut List<ArgError>, found: Found, field: Str, shown: Str, what: Str, test: (v: B) -> Str?, ?from_str: (s: Str) -> B?) [] -> None
=>[test] v => errors: Mut, found, field, shown, what, test {
    for s in values_of(found, field) {
        let v = from_str(copy(s))
        if v is None {
            add(errors, Invalid { what: copy(shown), value: copy(s), reason: "expected ${what}" })
        } else {
            let why = test(v)
            if why !is None {
                add(errors, Invalid { what: copy(shown), value: copy(s), reason: why })
            }
        }
    }
}

// How errors name a field: `--max-workers`, or `<files>`.
// @compilerrules [args-derive]
fn shown_name(field: Str, positional: Bool) [] -> Str => field {
    if positional {
        return arg_name(field)
    }
    return long_name(field)
}

// The text of a literal arm's name: `json` for `"json"`.
// @compilerrules [args-derive]
fn literal_text(name: Str) [] -> Str => name {
    return trim_suffix(trim_prefix(name, "\""), "\"")
}

// A group's value from its own `from_found`, adding the
// errors it found beyond the scan's (the first `skip`, which the caller
// already has).
// @compilerrules [args-groups]
fn group_errors<C>(errors: Mut List<ArgError>, r: Ok C | Errs NonEmpty List<ArgError>, skip: Int) [] -> None
=> errors: Mut, !r {
    if r is Errs {
        let i = 0
        for e in r {
            if i >= skip {
                add(errors, copy(e))
            }
            i = i + 1
        }
    }
}

// A group's value, once every check has passed.
// @compilerrules [args-groups]
fn built<C>(r: Ok C | Errs NonEmpty List<ArgError>) [] -> C => !r {
    when r {
        is Ok { return r }
        is Errs { unreachable!("a group failed after its checks passed") }
    }
}

// The [Spec] of a struct `T`, built from its fields: what `: FromArgs<self>
// by args` stamps as the struct's `arg_spec`. Each field is one option or
// positional, as the module doc describes, a struct-typed field adds its
// group's options, and a `Subcommand` field the commands of its union.
// @compilerrules [args-derive, args-groups]
export comptime fn arg_spec<T is Struct>() [] -> Spec<T> {
    let opts: Mut List<OptSpec> = Mut List<OptSpec>()
    let args: Mut List<ArgSpec> = Mut List<ArgSpec>()
    let groups: Mut List<GroupSpec> = Mut List<GroupSpec>()
    let commands: Mut List<CmdSpec> = Mut List<CmdSpec>()
    let command_required = false
    let command_fields = 0
    [for field in T.fields] {
        [if field is Subcommand] {
            [if field is Positional || field is Short] {
                refuse!("a sub-command is chosen by a word of its own, so it is neither positional nor an option")
            }
            command_fields = command_fields + 1
            [if field.type is *?] {} else {
                command_required = true
            }
            [if field.type.kind is Union] {
                [for arm in field.type.arms] {
                    [if arm.type.kind is Struct] {
                        let child: Spec<[arm.type]> = arg_spec()
                        add@list(commands, CmdSpec { name: copy@basic(child.name), about: copy@basic(child.about) })
                    } else {
                        [if arm.type is None] {} else {
                            refuse!("a sub-command is a struct, declared `: FromArgs<self> by args`")
                        }
                    }
                }
            } else {
                refuse!("a `Subcommand` field's type is a union of command structs")
            }
        } else {
        [if field.type is <S> S? && S.kind is Struct] {
            [if field is Positional] {
                refuse!("a positional takes words, and a group of options does not")
            }
            // All or none.
            let child: Spec<S> = arg_spec()
            merge_group@args(opts, groups, child, field.name, "together", field.docs)
        } else {
        [if field.type.kind is Struct] {
            [if field is Positional] {
                refuse!("a positional takes words, and a group of options does not")
            }
            // A flattened group: the struct's options, as this command's.
            let child: Spec<[field.type]> = arg_spec()
            merge_group@args(opts, groups, child, field.name, "flat", field.docs)
        } else {
            let required = false
            let many = false
            let switch = false
            let default_text: Str? = None
            let choices: Mut List<Str> = Mut List<Str>()
            let exclusive = false
            [if field.type is Bool] {
                switch = true
            } else {
                [if field.type is List<*>] {
                    many = true
                    [if field.type is NonEmpty List<*>] {
                        required = true
                    }
                } else {
                    [if field.default is None] {
                        [if field.type is *?] {} else {
                            required = true
                        }
                    } else {
                        [if field.type is *?] {} else {
                            default_text = "${field.default}"
                        }
                    }
                    // A union: its literals are choices, its struct arms an
                    // exclusive group of options.
                    [if field.type is <E> E?] {
                        [if E.kind is Union] {
                            [for arm in field.type.arms] {
                                [if arm.type.kind is Struct] {
                                    let child: Spec<[arm.type]> = arg_spec()
                                    merge_group@args(opts, groups, child, "${field.name}/${arm.name}", "arm", None)
                                    exclusive = true
                                } else {
                                    [if arm.type is None] {} else {
                                        add@list(choices, literal_text@args(arm.name))
                                    }
                                }
                            }
                        }
                    } else {
                        [if field.type.kind is Union] {
                            [for arm in field.type.arms] {
                                [if arm.type.kind is Struct] {
                                    let child: Spec<[arm.type]> = arg_spec()
                                    merge_group@args(opts, groups, child, "${field.name}/${arm.name}", "arm", None)
                                    exclusive = true
                                } else {
                                    [if arm.type is None] {} else {
                                        add@list(choices, literal_text@args(arm.name))
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if exclusive {
                add_one@args(groups, field.name, required, field.docs)
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
                required: required && !exclusive,
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
                if exclusive {
                    unreachable!("positional `${field.name}` is a choice of option groups")
                }
                add@list(args, ArgSpec { field: copy@basic(opt.field), help: copy@basic(opt.help), required: opt.required, many: opt.repeats })
            } else {
                // An exclusive group is chosen by its arms' options, and has
                // none of its own unless it mixes in literals.
                if !exclusive || !is_empty@list(opt.choices) {
                    add@list(opts, opt)
                }
            }
        }
        }
        }
    }
    let name = command_name@args(T.name)
    [if T is <n> Named(n)] {
        name = n
    }
    return Spec<T> {
        name: name,
        about: T.docs,
        opts: opts,
        args: args,
        groups: groups,
        commands: commands,
        command_required: command_required,
        command_fields: command_fields
    }
}

// A struct `T` from what a scan of its command line found: what `:
// FromArgs<self> by args` stamps as the struct's `from_found`.
//
// Every value is first checked against its field's type, each failure an
// error after the scan's own. When there are no errors, `T` is built from
// the values and the defaults. A group's value comes from its own struct's
// `from_found`, and a sub-command's from parsing the rest of the line.
// @compilerrules [args-derive, args-groups]
export comptime fn from_found<T is Struct>(found: Found) [] -> Ok T | Errs NonEmpty List<ArgError> => found {
    let errors: Mut List<ArgError> = Mut List<ArgError>()
    for e in found.errors {
        add@list(errors, copy@basic(e))
    }
    [for field in T.fields] {
        [if field is Subcommand] {
            [if field.type.kind is Union] {
                [for arm in field.type.arms] {
                    [if arm.type.kind is Struct] {
                        let s: Spec<[arm.type]> = arg_spec()
                        if is_command@args(found, s.name) {
                            let r: Ok [arm.type] | Errs NonEmpty List<ArgError> = parse_named@args(found.rest, copy@basic(found.path))
                            group_errors@args(errors, r, 0)
                        }
                    }
                }
            }
        } else {
        [if field.type is <S> S? && S.kind is Struct] {
            let s: Spec<S> = arg_spec()
            if any_given@args(found, s) {
                let r: Ok S | Errs NonEmpty List<ArgError> = from_found(found)
                group_errors@args(errors, r, size@list(found.errors))
            }
        } else {
        [if field.type.kind is Struct] {
            let r: Ok [field.type] | Errs NonEmpty List<ArgError> = from_found(found)
            group_errors@args(errors, r, size@list(found.errors))
        } else {
        [if field.type is <E> E? && E.kind is Union] {
            // A literal's value was checked by the scan, against its choices.
            [for arm in field.type.arms] {
                [if arm.type.kind is Struct] {
                    let s: Spec<[arm.type]> = arg_spec()
                    if any_given@args(found, s) {
                        let r: Ok [arm.type] | Errs NonEmpty List<ArgError> = from_found(found)
                        group_errors@args(errors, r, size@list(found.errors))
                    }
                }
            }
        } else {
        [if field.type.kind is Union && field.type !is *?] {
            // A literal's value was checked by the scan, against its choices.
            [for arm in field.type.arms] {
                [if arm.type.kind is Struct] {
                    let s: Spec<[arm.type]> = arg_spec()
                    if any_given@args(found, s) {
                        let r: Ok [arm.type] | Errs NonEmpty List<ArgError> = from_found(found)
                        group_errors@args(errors, r, size@list(found.errors))
                    }
                }
            }
        } else {
        // A switch has no value to check.
        [if field.type is Bool] {} else {
            let positional = false
            [if field is Positional] {
                positional = true
            }
            let shown = shown_name@args(field.name, positional)
            let what = "a valid value"
            [if field.type is Int || field.type is Long || field.type is List<Int> || field.type is List<Long> || field.type is Int? || field.type is Long?] {
                what = "an integer"
            }
            [if field.type is Double || field.type is List<Double> || field.type is Double?] {
                what = "a number"
            }
            [if field.type is <E> List<E>] {
                check_value@args<E>(errors, found, field.name, shown, what, (_v: E) -> None)
            } else {
                [if field.type is <E> E?] {
                    check_value@args<E>(errors, found, field.name, shown, what, (_v: E) -> None)
                } else {
                    [if field.type is Str] {
                        // [args-explain] The field's qualifiers say why a
                        // value fails them: "0 is not in 1..65535".
                        check_value@args<Str>(errors, found, field.name, shown, what, (v: Str) -> whynot?(v is [field.type]))
                    } else {
                        [if field.type is Int] {
                            check_value@args<Int>(errors, found, field.name, shown, what, (v: Int) -> whynot?(v is [field.type]))
                        } else {
                            [if field.type is Long] {
                                check_value@args<Long>(errors, found, field.name, shown, what, (v: Long) -> whynot?(v is [field.type]))
                            } else {
                                [if field.type is Double] {
                                    check_value@args<Double>(errors, found, field.name, shown, what, (v: Double) -> whynot?(v is [field.type]))
                                } else {
                                    check_value@args<[field.type]>(errors, found, field.name, shown, what, (_v: [field.type]) -> None)
                                }
                            }
                        }
                    }
                }
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
        [...for field in T.fields] {
            [field]: eval@args<[field.type]>(() -> {
                [if field is Subcommand] {
                    [if field.type.kind is Union] {
                        [for arm in field.type.arms] {
                            [if arm.type.kind is Struct] {
                                let s: Spec<[arm.type]> = arg_spec()
                                if is_command@args(found, s.name) {
                                    return built@args<[arm.type]>(parse_named@args(found.rest, copy@basic(found.path)))
                                }
                            }
                        }
                    }
                    [if field.type is *?] {
                        return None
                    } else {
                        unreachable!("the sub-command was checked")
                    }
                } else {
                [if field.type is <S> S? && S.kind is Struct] {
                    let s: Spec<S> = arg_spec()
                    if any_given@args(found, s) {
                        return built@args<S>(from_found(found))
                    }
                    return None
                } else {
                [if field.type.kind is Struct] {
                    return built@args<[field.type]>(from_found(found))
                } else {
                [if field.type is <E> E? && E.kind is Union] {
                    // The arm whose options were given, or the literal the value names.
                    [for arm in field.type.arms] {
                        [if arm.type.kind is Struct] {
                            let s: Spec<[arm.type]> = arg_spec()
                            if any_given@args(found, s) {
                                return built@args<[arm.type]>(from_found(found))
                            }
                        }
                    }
                    let v = value_of@args(found, field.name)
                    if v !is None {
                        [for arm in field.type.arms] {
                            [if arm.type.kind is Struct || arm.type is None] {} else {
                                if v is [arm.type] {
                                    return v
                                }
                            }
                        }
                    }
                    [if field.default is None] {
                        [if field.type is *?] {
                            return None
                        } else {
                            unreachable!("`${field.name}` was checked as required")
                        }
                    } else {
                        return field.default
                    }
                } else {
                [if field.type.kind is Union && field.type !is *?] {
                    // The arm whose options were given, or the literal the value names.
                    [for arm in field.type.arms] {
                        [if arm.type.kind is Struct] {
                            let s: Spec<[arm.type]> = arg_spec()
                            if any_given@args(found, s) {
                                return built@args<[arm.type]>(from_found(found))
                            }
                        }
                    }
                    let v = value_of@args(found, field.name)
                    if v !is None {
                        [for arm in field.type.arms] {
                            [if arm.type.kind is Struct || arm.type is None] {} else {
                                if v is [arm.type] {
                                    return v
                                }
                            }
                        }
                    }
                    [if field.default is None] {
                        [if field.type is *?] {
                            return None
                        } else {
                            unreachable!("`${field.name}` was checked as required")
                        }
                    } else {
                        return field.default
                    }
                } else {
                [if field.type is Bool] {
                    [if field.default is None] {
                        return given@args(found, field.name)
                    } else {
                        return given@args(found, field.name) || field.default
                    }
                } else {
                [if field.type is <E> List<E>] {
                    let all = read_all@args<E>(found, field.name)
                    if all is [field.type] {
                        return all
                    }
                    unreachable!("`${field.name}` was checked as required")
                } else {
                [if field.type is <E> E?] {
                    [if field.default is None] {
                        return read_optional@args<E>(found, field.name)
                    } else {
                        let v = read_optional@args<E>(found, field.name)
                        if v !is None {
                            return v
                        }
                        return field.default
                    }
                } else {
                [if field.type is Str] {
                    return read_value@args<Str, [field.type]>(found, field.name, field.default, (v: Str) -> if v is [field.type] { v } else { None })
                } else {
                [if field.type is Int] {
                    return read_value@args<Int, [field.type]>(found, field.name, field.default, (v: Int) -> if v is [field.type] { v } else { None })
                } else {
                [if field.type is Long] {
                    return read_value@args<Long, [field.type]>(found, field.name, field.default, (v: Long) -> if v is [field.type] { v } else { None })
                } else {
                [if field.type is Double] {
                    return read_value@args<Double, [field.type]>(found, field.name, field.default, (v: Double) -> if v is [field.type] { v } else { None })
                } else {
                    return read_value@args<[field.type], [field.type]>(found, field.name, field.default, (v: [field.type]) -> v)
                }
                }
                }
                }
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
