// derived command-line parsers (std.args)
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [args-derive] [comptime-is-splice] [comptime-pattern] A struct's command
// line, derived by `: FromArgs<self> by args`: switches, defaults, `Short`
// bundles, choices from literal unions (optional too), optional and
// repeated options, `Double`/`Long` values, a qualified `InRange` port
// explained with `whynot?(v is [field.type])`, positionals with `NonEmpty`,
// every error at once, and help with a `Named` command's name.
//--- a_derived_parser_reads_a_command_line
import std.args

// Serve files.
struct Serve : FromArgs<self> by args {
    // Address to bind.
    host: Str = "127.0.0.1",
    Short('p') port: InRange(1, 65535) Int = 8080,
    output: "json" | "text" = "text",
    level: ("low" | "high")?,
    Short('v') verbose: Bool,
    workers: Int?,
    ratio: Double = 0.5,
    big: Long?,
    tag: List<Str>,
    Positional files: NonEmpty List<Str>
}

// Remove a file.
Named("rm")
struct Remove : FromArgs<self> by args {
    Short('f') force: Bool,
    // What to remove.
    Positional path: Str,
    Positional backup: Str?
}

fn level(l: ("low" | "high")?) [] -> Str => !l {
    if l is None {
        return "-"
    }
    return l
}

fn serve(args: List<Str>) [Console] -> None => args {
    let r: Ok Serve | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok { println("ok ${r.host} ${r.port} ${r.output} ${level(r.level)} ${r.verbose} ${r.workers ?: 0} ${r.ratio} ${r.big ?: 0L} [${join(r.tag, ",")}] [${join(r.files, ",")}]") }
        is Errs {
            for e in r {
                println("error: ${message(copy(e))}")
            }
        }
    }
}

fn remove(args: List<Str>) [Console] -> None => args {
    let r: Ok Remove | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok { println("ok ${r.force} ${r.path} ${r.backup ?: "-"}") }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    serve(List("a"))
    serve(List("-vp80", "--output=json", "--level", "high", "--workers", "4", "--ratio", "-1.5", "--big", "9000000000", "--tag", "x", "--tag", "y", "a", "b"))
    serve(List("-p", "70000", "--ratio", "x", "--level", "mid", "--big", "1.5", "--nope"))
    remove(List("-f", "x"))
    remove(List("x", "y", "z"))
    remove(List("-h"))
}
//= stdout
//= ok 127.0.0.1 8080 text - false 0 0.5 0 [] [a]
//= ok 127.0.0.1 80 json high true 4 -1.5 9000000000 [x,y] [a,b]
//= error: unknown option '--nope'
//= error: invalid value 'mid' for '--level': expected one of low, high
//= error: '<files>' is required
//= error: invalid value '70000' for '--port': 70000 is not in 1..65535
//= error: invalid value 'x' for '--ratio': expected a number
//= error: invalid value '1.5' for '--big': expected an integer
//= ok true x -
//= error: unexpected argument 'z'
//= Remove a file.
//=
//= Usage: rm [OPTIONS] <path> [<backup>]
//=
//= Arguments:
//=   <path>       What to remove.
//=   <backup>
//=
//= Options:
//=   -f, --force
//=   -h, --help   Print help

// [args-derive] `args_or_exit` prints help to standard output with status 0,
// and every error to standard error with the usage line and status 2. Three
// command lines over one program: a good one, `--help`, and a bad one.
//--- args_or_exit_answers_a_good_line
// ARGS: -f a b
import std.args
import std.process

// Copy a file.
struct Copy : FromArgs<self> by args {
    // Overwrite what is there.
    Short('f') force: Bool,
    Positional from: Str,
    Positional to: Str
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    let c: Copy = args_or_exit(args())
    println("${c.force} ${c.from} ${c.to}")
}
//= stdout
//= true a b

// [args-derive] `--help`: the help on standard output, status 0.
//--- args_or_exit_prints_help
// ARGS: --help
import std.args
import std.process

// Copy a file.
struct Copy : FromArgs<self> by args {
    // Overwrite what is there.
    Short('f') force: Bool,
    Positional from: Str,
    Positional to: Str
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    let c: Copy = args_or_exit(args())
    println("${c.force} ${c.from} ${c.to}")
}
//= stdout
//= Copy a file.
//=
//= Usage: copy [OPTIONS] <from> <to>
//=
//= Arguments:
//=   <from>
//=   <to>
//=
//= Options:
//=   -f, --force  Overwrite what is there.
//=   -h, --help   Print help

// [args-derive] Errors: every one on standard error with the usage line,
// nothing on standard output, status 2.
//--- args_or_exit_reports_errors
// ARGS: -x a
import std.args
import std.process

// Copy a file.
struct Copy : FromArgs<self> by args {
    // Overwrite what is there.
    Short('f') force: Bool,
    Positional from: Str,
    Positional to: Str
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    let c: Copy = args_or_exit(args())
    println("${c.force} ${c.from} ${c.to}")
}
//= exit 2
//= stderr
//= error: unknown option '-x'
//= error: '<to>' is required
//=
//= Usage: copy [OPTIONS] <from> <to>
//=
//= For more information, try '--help'.

// [args-groups] Groups of options, each a struct with its own `FromArgs`:
// flattened (`net: Net`, its options the command's own, under a heading),
// all-or-none (`tls: Tls?`: `--cert` requires `--key`), and exclusive
// (`source: FromUrl | FromFile | FromStdin`: chosen by which arm's options
// appear, an empty arm by a switch named after it, one required).
//--- derived_option_groups
import std.args

// Network options.
struct Net : FromArgs<self> by args {
    // Address to bind.
    host: Str = "127.0.0.1"
    Short('p') port: Int = 8080
}

// TLS. Give both or neither.
struct Tls : FromArgs<self> by args {
    // Certificate file.
    cert: Str
    // Private key file.
    key: Str
    ca: Str?
}

struct FromUrl : FromArgs<self> by args {
    url: Str
    timeout: Int = 30
}

struct FromFile : FromArgs<self> by args {
    file: Str
}

// Read standard input.
struct FromStdin : FromArgs<self> by args {}

// Fetch something.
struct Fetch : FromArgs<self> by args {
    net: Net
    tls: Tls?
    // Where to read.
    source: FromUrl | FromFile | FromStdin
    output: "json" | "text" = "text"
}

fn show(args: List<Str>) [Console] -> None => args {
    let r: Ok Fetch | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok {
            let src = r.source
            let s = when src {
                is FromUrl { "url ${src.url} ${src.timeout}" }
                is FromFile { "file ${src.file}" }
                is FromStdin { "stdin" }
            }
            let t = r.tls
            let tls = if t is None { "-" } else { "${t.cert}+${t.key}+${t.ca ?: "-"}" }
            println("ok ${r.net.host}:${r.net.port} ${tls} ${s} ${r.output}")
        }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    show(List("--url", "http://x"))
    show(List("-p", "9", "--cert", "c", "--key", "k", "--file", "f", "--output", "json"))
    show(List("--from-stdin", "--timeout", "5"))
    show(List("--cert", "c", "--url", "u", "--file", "f"))
    show(List("--host", "h"))
    show(List("--help"))
}
//= stdout
//= ok 127.0.0.1:8080 - url http://x 30 text
//= ok 127.0.0.1:9 c+k+- file f json
//= error: '--timeout' requires '--url'
//= error: '--from-stdin' cannot be used with '--timeout'
//= error: '--cert' requires '--key'
//= error: '--file' cannot be used with '--url'
//= error: one of '--url', '--file', '--from-stdin' is required
//= Fetch something.
//=
//= Usage: fetch [OPTIONS]
//=
//= Options:
//=       --output <OUTPUT>    [possible values: json, text] [default: text]
//=   -h, --help               Print help
//=
//= Network options:
//=       --host <HOST>        Address to bind. [default: 127.0.0.1]
//=   -p, --port <PORT>        [default: 8080]
//=
//= TLS. Give both or neither:
//=       --cert <CERT>        Certificate file.
//=       --key <KEY>          Private key file.
//=       --ca <CA>
//=
//= Where to read:
//=       --url <URL>
//=       --timeout <TIMEOUT>  [default: 30]
//=       --file <FILE>
//=       --from-stdin         Read standard input.

// [args-groups] A clash between a group's option and the command's is
// refused as the parser starts.
//--- a_clash_between_option_groups_traps
import std.args

struct Net : FromArgs<self> by args {
    host: Str = "a"
}

struct Cli : FromArgs<self> by args {
    net: Net
    host: Str = "b"
}

fn main() [use] -> None {
    use StdConsole()
    let r: Ok Cli | Errs NonEmpty List<ArgError> = parse_args(List<Str>())
    println("parsed ${r is Ok}")
}
//= trap two options of `cli` are spelled `--host`

// [args-commands] Sub-commands: a `Subcommand` field over a union of
// command structs, chosen by the first word (named by kebab case, the last
// segment of a namespaced name, or `Named`), nested to two levels, optional
// (`(A | B)?`), with the parent's options only before the word, an unknown
// or missing command an error, and help at each level naming the path
// (`git remote add`).
//--- derived_sub_commands
import std.args

// Manage a repository.
struct Git : FromArgs<self> by args {
    // Print more.
    Short('v') verbose: Bool
    Subcommand command: GitCommand
}

type GitCommand = Clone | Remote

// Copy a repository.
struct Clone : FromArgs<self> by args {
    // Where from.
    Positional url: Str
    depth: Int?
}

// Manage remotes.
struct Remote : FromArgs<self> by args {
    Subcommand command: (Remote.Add | Remote.Remove)?
}

namespace Remote {
    // Add a remote.
    struct Add : FromArgs<self> by args {
        Positional name: Str
        Positional url: Str
    }

    // Remove a remote.
    Named("rm")
    struct Remove : FromArgs<self> by args {
        Positional name: Str
    }
}

fn describe(g: Git) [] -> Str => g {
    let c = g.command
    when c {
        is Clone { return "clone ${c.url} ${c.depth ?: 0}" }
        is Remote {
            let sub = c.command
            when sub {
                is Remote.Add { return "remote add ${sub.name} ${sub.url}" }
                is Remote.Remove { return "remote rm ${sub.name}" }
                is None { return "remote" }
            }
        }
    }
}

fn show(args: List<Str>) [Console] -> None => args {
    let r: Ok Git | Errs NonEmpty List<ArgError> = parse_args(args)
    when r {
        is Ok { println("ok ${r.verbose} ${describe(r)}") }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    show(List("-v", "clone", "URL", "--depth", "1"))
    show(List("remote", "add", "origin", "u"))
    show(List("remote", "rm", "origin"))
    show(List("remote"))
    show(List("clone", "-v"))
    show(List("push"))
    show(List())
    show(List("--help"))
    show(List("remote", "add", "--help"))
}
//= stdout
//= ok true clone URL 1
//= ok false remote add origin u
//= ok false remote rm origin
//= ok false remote
//= error: unknown option '-v'
//= error: '<url>' is required
//= error: unrecognized command 'push'
//= error: '<command>' is required
//= Manage a repository.
//=
//= Usage: git [OPTIONS] <COMMAND>
//=
//= Commands:
//=   clone          Copy a repository.
//=   remote         Manage remotes.
//=
//= Options:
//=   -v, --verbose  Print more.
//=   -h, --help     Print help
//= Add a remote.
//=
//= Usage: git remote add <name> <url>
//=
//= Arguments:
//=   <name>
//=   <url>
//=
//= Options:
//=   -h, --help  Print help

// [args-validate] [fn-value] [fn-throw-value] The validation hook: a user's
// `validate(o) [Throw<ArgError>] -> +Valid Range`, found as an implicit, so
// `parse_valid` answers `Ok Valid Range` or the parse errors or the one
// `validate` threw, and `valid_args_or_exit` ends the process on an error
// with status 2. Two command lines over one program: a valid one and an
// invalid one.
//--- the_validation_hook_accepts_a_valid_line
// ARGS: --max 4
import std.args
import std.process
import std.throw

struct Range : FromArgs<self> by args {
    // Smallest.
    min: Int = 0
    // Largest.
    max: Int = 10
}

qualifier Valid of Range

fn validate(o: Range) [Throw<ArgError>] -> +Valid Range => !o {
    if o.min > o.max {
        let e: ArgError = Invalid { what: "--min", value: "${o.min}", reason: "must not exceed --max" }
        throw(e)
    }
    return o
}

fn width(r: Valid Range) [] -> Int => r {
    return r.max - r.min
}

fn show(line: List<Str>) [Console] -> None => line {
    let r: Ok Valid Range | Errs NonEmpty List<ArgError> = parse_valid(line)
    when r {
        is Ok { println("width ${width(r)}") }
        is Errs {
            for e in r {
                println("error: ${message(copy(e))}")
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    show(List("--min", "3"))
    show(List("--min", "30"))
    show(List("--min", "x"))
    let r: Valid Range = valid_args_or_exit(args())
    println("argv width ${width(r)}")
}
//= stdout
//= width 7
//= error: invalid value '30' for '--min': must not exceed --max
//= error: invalid value 'x' for '--min': expected an integer
//= argv width 4

// [args-validate] The invalid line: `valid_args_or_exit` reports what
// `validate` threw on standard error and ends the process with status 2.
//--- the_validation_hook_refuses_an_invalid_line
// ARGS: --min 9 --max 4
import std.args
import std.process
import std.throw

struct Range : FromArgs<self> by args {
    // Smallest.
    min: Int = 0
    // Largest.
    max: Int = 10
}

qualifier Valid of Range

fn validate(o: Range) [Throw<ArgError>] -> +Valid Range => !o {
    if o.min > o.max {
        let e: ArgError = Invalid { what: "--min", value: "${o.min}", reason: "must not exceed --max" }
        throw(e)
    }
    return o
}

fn width(r: Valid Range) [] -> Int => r {
    return r.max - r.min
}

fn show(line: List<Str>) [Console] -> None => line {
    let r: Ok Valid Range | Errs NonEmpty List<ArgError> = parse_valid(line)
    when r {
        is Ok { println("width ${width(r)}") }
        is Errs {
            for e in r {
                println("error: ${message(copy(e))}")
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    show(List("--min", "3"))
    show(List("--min", "30"))
    show(List("--min", "x"))
    let r: Valid Range = valid_args_or_exit(args())
    println("argv width ${width(r)}")
}
//= exit 2
//= stderr
//= error: invalid value '9' for '--min': must not exceed --max
//=
//= Usage: range [OPTIONS]
//=
//= For more information, try '--help'.
//= stdout
//= width 7
//= error: invalid value '30' for '--min': must not exceed --max
//= error: invalid value 'x' for '--min': expected an integer
