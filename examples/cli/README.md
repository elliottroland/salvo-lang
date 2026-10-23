# cli

A command-line tool whose command line is its types. Every struct says `:
FromArgs<self> by args`, and `args` derives a parser from its fields at
compile time, so the parser's answer is the struct: the program reads
`c.port` as an `InRange(1, 65535) Int` and dispatches on `t.command` with an
exhaustive `when`.

```bash
cargo run -- run --backend rust   --src examples/cli/salvo
cargo run -- run --backend kotlin --src examples/cli/salvo
cargo run -- run --backend rust   --src examples/cli/salvo -- serve --help
```

The program parses eight command lines with `parse_args`, which never exits,
and prints what each one says; then it parses its real argument list with
`args_or_exit`, which prints help or errors and ends the process. The test
runs it with no arguments, so the last line is `argv: nothing to do`.

## What to look for

**1 — a field's type is what it takes.** `verbose: Bool` is a switch,
`port: … Int = 8080` a value with a default, `workers: Int?` one that may be
absent, `output: "json" | "text"` one of two literals, `header: List<Str>` a
repeatable option, and `Positional files: NonEmpty List<Str>` at least one
word. `Short('p')` and `Positional` are *comptime qualifiers*: written
before a field's name, they describe the field to the derive and say nothing
about its value.

**2 — a qualified type is checked.** `--port 0` fails with `0 is not in
1..65535`, which is `InRange`'s own `explain`. What reaches the program has
passed the test, so the type carries the claim.

**3 — a struct-typed field is a group.** `net: Net` adds `--host` and
`--workers` to `serve` (under `Net`'s doc comment in help); `tls: Tls?` is
all or none, so `--cert` alone is `'--cert' requires '--key'`; `source:
FromUrl | FromFile | FromStdin` is exactly one arm, chosen by which arm's
options appear, so `--url` with `--file` is refused, and the empty
`FromStdin` is the switch `--from-stdin`.

**4 — a sub-command is a union.** `Subcommand command: (Serve | Fetch)?`
takes the first word as the command, and the rest of the line is the
command's. `tool serve --help` is `serve`'s help, its usage naming the path.

**5 — every error at once.** `tool serve -p 0 --output xml --cert c.pem`
reports four problems, not the first.

**6 — doc comments are help.** The `//` lines directly above a field are its
help text. The comments written for a reader of this file are kept apart by
a blank line, which ends a doc comment.
