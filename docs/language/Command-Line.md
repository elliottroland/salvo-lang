# Command-line tools

`examples/cli` is a complete tool built this way, with sub-commands, groups
and every kind of field below.

A command line is described by a struct. One clause gives the struct a
parser, and the parser's result is the struct itself, so the program reads
its options as typed fields and the compiler checks every use:

```
import args
import process

// Serve files over HTTP.
struct Serve : FromArgs<self> by args {
    // Address to bind.
    host: Str = "127.0.0.1"
    // Port to listen on.
    Short('p') port: InRange(1, 65535) Int = 8080
    // Output format.
    output: "json" | "text" = "text"
    // Print more.
    Short('v') verbose: Bool
    // Files to serve.
    Positional files: NonEmpty List<Str>
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    use HostProcess()
    let opts: Serve = args_or_exit(args())
    for f in opts.files {
        println("serving ${f} on ${opts.host}:${opts.port}")
    }
}
```

`serve -vp 80 --output json a.txt b.txt` gives `Serve { host: "127.0.0.1",
port: 80, output: "json", verbose: true, files: List("a.txt", "b.txt") }`.
`serve --help` prints:

```
Serve files over HTTP.

Usage: serve [OPTIONS] <files>...

Arguments:
  <files>...             Files to serve.

Options:
      --host <HOST>      Address to bind. [default: 127.0.0.1]
  -p, --port <PORT>      Port to listen on. [default: 8080]
      --output <OUTPUT>  Output format. [possible values: json, text] [default: text]
  -v, --verbose          Print more.
  -h, --help             Print help
```

## What each field becomes

Each field is an option named after it in kebab case: `max_workers` is
`--max-workers`. Its doc comment, the `//` lines directly above it, is its
help; a blank line ends a doc comment, so a remark meant for the reader of
the source can sit above it, apart. What a field takes depends on its type:

| Field type | On the command line |
|---|---|
| `Bool` | a switch: `--verbose` sets it |
| `T` with a default | `--name VALUE`, optional |
| `T` without one | `--name VALUE`, required |
| `T?` | `--name VALUE`, `None` when absent |
| `List<T>` | `--name V --name W`, every value kept |
| `NonEmpty List<T>` | the same, at least once |
| a literal union `"json" \| "text"` | `--name json`, one of its literals |

A value is parsed with `from_str` at the field's type, so `Int`, `Long`,
`Double`, `Str` and a type of your own with a `from_str` all work. A field
whose type has a predicate qualifier, `InRange(1, 65535) Int`, is tested as
well: `--port 0` is an error (`invalid value '0' for '--port': 0 is not in
1..65535`), and `opts.port` is an `InRange(1, 65535) Int` with nothing more
to check.

Three comptime qualifiers say what the type cannot:

- `Short('p')` before a field's name adds `-p`.
- `Positional` takes the field by position instead of by name, in field
  order: `Str` one word, `Str?` zero or one, `List<Str>` the rest.
- `Named("rm")` before a struct names the command; otherwise it is named
  after the struct, `Serve` as `serve`.

## Groups of options

A field whose type is a struct is a group of that struct's options. The
struct says `: FromArgs<self> by args` too, and its doc comment, or the
field's, is the group's heading in help.

```
// Network options.
struct Net : FromArgs<self> by args {
    host: Str = "127.0.0.1"
    Short('p') port: Int = 8080
}

struct Tls : FromArgs<self> by args {
    cert: Str
    key: Str
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

struct Fetch : FromArgs<self> by args {
    net: Net
    // TLS. Give both or neither.
    tls: Tls?
    // Where to read.
    source: FromUrl | FromFile | FromStdin
}
```

- `net: Net` is **flattened**: `--host` and `--port` are `fetch`'s own
  options. A name that both `Net` and `Fetch` use is an error.
- `tls: Tls?` is **all or none**: `None` when neither `--cert` nor `--key`
  is given, and `--cert` alone is an error, `'--cert' requires '--key'`.
- `source: FromUrl | FromFile | FromStdin` is **exactly one** of its arms,
  chosen by whose options appear: `--url` picks `FromUrl`, after which
  `--timeout` is allowed and `--file` is an error. An arm with no fields is a
  switch named after it, `--from-stdin`. With no `None` in the union one arm
  is required: `one of '--url', '--file', '--from-stdin' is required`.

Groups nest: an exclusive arm may hold an all-or-none group, and so on.

## Sub-commands

A field marked `Subcommand`, whose type is a union of command structs, takes
a sub-command: the first word names it, and the rest of the line is its own.

```
// Manage a repository.
struct Git : FromArgs<self> by args {
    Short('v') verbose: Bool
    Subcommand command: Clone | Remote
}

// Copy a repository.
struct Clone : FromArgs<self> by args {
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
```

`git -v clone URL --depth 1` gives `Git { verbose: true, command: Clone {
url: "URL", depth: 1 } }`, and the program dispatches with an exhaustive
`when`, so a command added later breaks every dispatch that does not handle
it:

```
when c = git.command {
    is Clone { clone(c) }
    is Remote { remote(c) }
}
```

- A command is named after its struct in kebab case, the last segment of a
  namespaced name (`Remote.Add` is `add`), or by `Named`.
- The parent's options come before the command's name: `git clone -v` is an
  error, since `-v` is `git`'s.
- With `?` the command may be left out; otherwise `'<command>' is required`.
- `git remote add --help` is `add`'s help, its usage naming the path:
  `Usage: git remote add <name> <url>`.
- A struct with a `Subcommand` field takes no positionals: whether a word
  named a command would depend on its spelling.

## The conventions

- `--port 80` and `--port=80`; `-p 80` and `-p80`.
- Short switches bundle: `-vf` is `-v -f`, and `-vp80` is `-v -p 80`.
- `--` ends the options; everything after it is positional.
- A lone `-` is a positional, and so is a negative number, so `--offset -3`
  reads `-3`.
- There are no abbreviations: `--po` is not `--port`.
- `--help` and `-h` are the parser's own.

## Errors

Parsing reports every problem at once, not the first:

```
error: unknown option '--prot'
error: invalid value 'many' for '--workers': expected an integer
error: '<files>' is required

Usage: serve [OPTIONS] <files>...

For more information, try '--help'.
```

`args_or_exit` prints help to standard output and exits with status 0, and
errors to standard error and exits with status 2. `parse_args` is the same
parse without the exit: it answers `Ok T | Errs NonEmpty List<ArgError>`,
help included as a `HelpRequested` error, so a test can call it with a list:

```
let r: Ok Serve | Errs NonEmpty List<ArgError> = parse_args(List("-p", "0", "a.txt"))
```

`message(e)` is an error as the line above prints it.

## Validating the whole command line

A rule between options that no type says, `--min` at most `--max`, is a
`validate` function that answers a qualified type the rest of the program
can demand, and throws an `ArgError` when the rule is broken:

```
import throw

struct Range : FromArgs<self> by args {
    min: Int = 0
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
```

`valid_args_or_exit` parses and then calls the `validate` in scope, so what
it answers is the validated type:

```
let r: Valid Range = valid_args_or_exit(args())
println("${width(r)}")
```

`range --min 9 --max 4` prints `error: invalid value '9' for '--min': must
not exceed --max` and exits with status 2. `parse_valid` is the same without
the exit: `Ok Valid Range | Errs NonEmpty List<ArgError>`. Since `width`
takes a `Valid Range`, a program that parses with `args_or_exit` and forgets
to validate does not compile.

## What is checked where

A mistake the derive can see is reported where the struct says `by args`: a
positional group, `Short` on a positional, a `Bool` positional. The rest is
checked as the parser starts, so the first run of the program fails rather
than a user's: two options with one name (a group's and the command's) or
one short name, `Short('h')`, a field named `help`, an exclusive arm whose
options are all optional, a required positional after an optional one, a
positional after one that takes every word.
