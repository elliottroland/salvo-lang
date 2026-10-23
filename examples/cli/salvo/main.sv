// A command-line tool, described by its types. `: FromArgs<self> by args`
// gives each struct a parser, derived from its fields at compile time: the
// parser's result is the struct itself, so the program reads its options as
// typed fields, and a field added later is parsed, checked and in the help
// without anything else to update.
//
// A doc comment, the `//` lines directly above a declaration, is its help
// text. The comments for the reader below are kept apart from them by a
// blank line, which ends a doc comment.

import args
import process

// A toy file server.
struct Tool : FromArgs<self> by args {
    // Print more.
    Short('v') verbose: Bool

    // A sub-command, or none: the first word picks it, and the rest of the
    // line is its own.

    Subcommand command: (Serve | Fetch)?
}

// Serve files over HTTP.
struct Serve : FromArgs<self> by args {
    // A struct-typed field is a group: `Net`'s options are `serve`'s own,
    // shown under `Net`'s doc comment.

    net: Net

    // A qualified type is re-tested: `--port 0` is an error, and in the
    // program `port` is an `InRange(1, 65535) Int`.

    // Port to listen on.
    Short('p') port: InRange(1, 65535) Int = 8080
    // Output format.
    output: "json" | "text" = "text"

    // An optional struct is all or none: `--cert` without `--key` is an
    // error.

    // TLS (give both or neither).
    tls: Tls?

    // Taken by position; `NonEmpty` requires at least one.

    // Files to serve.
    Positional files: NonEmpty List<Str>
}

// Network options.
struct Net : FromArgs<self> by args {
    // Address to bind.
    host: Str = "127.0.0.1"
    // Connections served at once.
    workers: Int?
}

struct Tls : FromArgs<self> by args {
    // Certificate file.
    cert: Str
    // Private key file.
    key: Str
}

// Fetch a document.
struct Fetch : FromArgs<self> by args {
    // A union of structs is exclusive: exactly one arm, chosen by which of
    // its options appear.

    // Where to read.
    source: FromUrl | FromFile | FromStdin
    // A header line to send; repeatable.
    header: List<Str>
}

struct FromUrl : FromArgs<self> by args {
    // Where to fetch from.
    url: Str
    // Seconds to wait.
    timeout: Int = 30
}

struct FromFile : FromArgs<self> by args {
    // A file to read instead.
    file: Str
}

// Read standard input.
struct FromStdin : FromArgs<self> by args {}

// What a parsed command line says, in one line.
fn describe(t: Tool) [] -> Str => t {
    let c = t.command
    when c {
        is Serve {
            let tls = c.tls
            let secure = if tls is None { "plain" } else { "tls ${tls.cert}" }
            return "serve ${join(c.files, ",")} on ${c.net.host}:${c.port} as ${c.output}, ${secure}, ${c.net.workers ?: 1} worker(s)"
        }
        is Fetch {
            let s = c.source
            let from = when s {
                is FromUrl { "${s.url} (timeout ${s.timeout}s)" }
                is FromFile { "file ${s.file}" }
                is FromStdin { "standard input" }
            }
            return "fetch ${from} with ${size(c.header)} header(s)"
        }
        is None { return "nothing to do" }
    }
}

// Parses one command line and prints what it says: the value, every error,
// or the help text. `parse_args` is the pure form; it never exits.
fn show(line: List<Str>) [Console] -> None => line {
    println("$ tool ${join(line, " ")}")
    let r: Ok Tool | Errs NonEmpty List<ArgError> = parse_args(line)
    when r {
        is Ok {
            let loud = if r.verbose { " (verbose)" } else { "" }
            println("  ${describe(r)}${loud}")
        }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is HelpRequested {
                    print(one.text)
                } else {
                    println("  error: ${message(one)}")
                }
            }
        }
    }
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    show(List("serve", "-p", "80", "index.html", "app.js"))
    show(List("-v", "serve", "--host=0.0.0.0", "--workers", "4", "--cert", "c.pem", "--key", "k.pem", "--output", "json", "site"))
    show(List("fetch", "--url", "https://example.com", "--header", "Accept: text/html", "--header", "X-Id: 7"))
    show(List("fetch", "--from-stdin"))
    show(List("serve", "-p", "0", "--output", "xml", "--cert", "c.pem"))
    show(List("fetch", "--url", "u", "--file", "f"))
    show(List("deploy"))
    show(List("--help"))
    show(List("serve", "--help"))
    // The real command line: help and errors end the process (status 0 and
    // 2), so a tool's `main` needs only this line. Run with no arguments, as
    // the example's test runs it, there is no command.
    let tool: Tool = args_or_exit(args())
    println("argv: ${describe(tool)}")
}
