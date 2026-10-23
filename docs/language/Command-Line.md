# Command-line tools

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
`--max-workers`. Its doc comment is its help. What it takes depends on its
type:

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
well: `--port 0` is an error, and `opts.port` is an `InRange(1, 65535) Int`
with nothing more to check.

Three comptime qualifiers say what the type cannot:

- `Short('p')` before a field's name adds `-p`.
- `Positional` takes the field by position instead of by name, in field
  order: `Str` one word, `Str?` zero or one, `List<Str>` the rest.
- `Named("rm")` before a struct names the command; otherwise it is named
  after the struct, `Serve` as `serve`.

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

## What is checked where

A mistake the derive can see is reported where the struct says `by args`: a
struct-typed field (option groups are not built yet), `Short` on a
positional, a `Bool` positional. The rest is checked as the parser starts,
so the first run of the program fails rather than a user's: two options with
one short name, `Short('h')`, a field named `help`, a required positional
after an optional one, a positional after one that takes every word.
