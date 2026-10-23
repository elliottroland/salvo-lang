# Command-line parsing — the option space (working document)

**Status:** DECIDED (rounds 1 to 4, 2026-10-20), not built. Earlier sections
keep their options for the record; the rounds below say which were chosen. The decision points are **AP-1** to
**AP-12** in §9, each with options, trade-offs and a recommendation; §7 lists
extensions to existing Salvo features (**X1** to **X17**), each justified by
uses beyond argument parsing. §10 designs the derive's names, positionals,
sub-commands and groups, with its own design calls **D-1** to **D-6**. The
calls are the user's.

**Round 1 (user, 2026-10-20).**
- **AP-1:** the derive approach (Option C), with comptime extended as needed.
- **D-1:** per-field metadata is a **comptime qualifier** written before the
  field's name, declared against `core.comptime`'s `Field`:
  `export comptime qualifier Short(c: Char) of Field`, used as `Short('p')
  port: InRange(1, 65535) Int = 8080`. The `comptime` modifier is what lets a
  qualifier target a comptime type. Erased; not carried on `FieldOf<T>` at
  run time.
- **Extensions:** build the recommended ones alongside the derive (§7; X15's
  marker aliases and X17's annotation kind are superseded by comptime
  qualifiers; X8, X13 and X14 are not part of this).
- **ROADMAP §2d:** the defects found while designing are fixed in general,
  alongside this work.

**Round 2 (user, 2026-10-20).** Every recommendation in §11 (Q0–Q19)
accepted, with two changes:
- **Q0 / Q19 / AP-12:** `std/args/parse.sv` and its test were the user's
  scratch space before this design, and are deleted (done 2026-10-20). The
  `args` module starts from the design here.
- **Q4: overloading by return type**, added to the language (option b):
  "we aren't really going to support wildcards, etc. so I think we can go
  ahead and include return-type overloading", making calls consistent with
  how a function passed by name is already chosen by its expected type,
  result included [fn-value-select]. Per-type value parsers are then plain
  `from_str(s: Str) -> Int?` overloads, and the witness parameter goes.
  The rule is designed in §12; its open details are R-1 to R-3 there.

**Round 3 (user, 2026-10-20).** R-2: return types related by subtyping are
refused at the declaration. R-3: the expected type reaches the branches of a
value `if` and the arms of a value `when`. R-1 waits on the example now in
§12. New question: whether comptime output used as a type is bracketed
(§13).

**Round 4 (user, 2026-10-20).** R-1: (a), the return type decides only among
candidates with identical parameter lists ("less action at a distance").
§13: (B), a bracket marks where expansion splices a comptime value into the
code's structure: `[field.type]`, `[arm.type] { … }`, `v.[field]`,
`[field]: …`; declared names (`T`, pattern binders) and values
(`field.name`) stay bare. **No design question is open; building can
start** (§8's order, as refined by §11's phases).

**Provenance.** Written 2026-10-20 at the user's request: "how to implement CLI
parsing in Salvo … as type-safe as is reasonable, as well as allow
inter-argument validation." It surveys how functional (Haskell, OCaml, Scala,
F#), procedural (C, Go, Python) and the two backend languages (Rust, Kotlin)
do it, plus Swift and Zig where they add something, then lays out what Salvo
could do with what it has and what it would need.

**Lifecycle.** As with the other option-space documents (SERDE.md,
FILE_SYSTEM.md): when the decisions land, the outcomes go to COMPLETED.md's
decision log and the specs, anything still open moves to ROADMAP.md, and this
file is deleted.

**Salvo code in this document** is marked one of two ways. *Today* means it
uses only built features (it may still need std functions that do not exist
yet). *Sketch* means it relies on something listed in §1's gaps, and is there
to show a shape, not a syntax decision.

---

## Build progress (resume here after a context reset)

**How to use this section.** The user asked (2026-10-20) for the work to be
tracked here so a fresh agent can carry on. Rules for whoever continues:
- Work the steps **in order**; mark a step `[x]` when it lands, and add a
  line to the log below naming its commit message (`git log --oneline
  --grep` finds the hash).
- **One commit per step** (AGENTS.md "Commits"): code, tests, spec rules,
  docs, the COMPLETED.md decision-log entry and the ROADMAP.md edits land
  together; a short imperative message.
- Before each commit: `cargo build` warning-free, `cargo test --no-fail-fast`,
  and a warm `cargo nextest run --no-fail-fast`.
- Proceed without stopping unless something needs the user (a new language
  decision not covered by rounds 1 to 4 above, or a result that contradicts
  this design). Record such a question here under "Blocked on the user".
- The design sections below are the specification: §10 (names, positionals,
  sub-commands, groups), §10.1c and round 1 (comptime qualifiers), §11
  (Q0 to Q19, all accepted), §12 (return-type overloading), §13 (brackets).

**Phase 0: fixes and std basics**
- [x] 0.1 Defaults checked against their declared type, at every default site
  (ROADMAP §2d item 1; round-2 Q2: a predicate-qualified default is tested
  with `qualifies` where it is applied, and traps on failure).
- [x] 0.2 Qualified literals in unions: supported, collapsing into the base arm;
  `is Q` compares against the literals carrying `Q`; a literal listed with
  and without `Q` refused; `Q "lit"` parses (§2d items 2, 3; Q1).
- [x] 0.3 A user type alias named like a `core.comptime` type resolves like a
  struct does (§2d item 4).
- [x] 0.4 Warnings in comptime copies: redirected to written code, reported
  once, only if every copy would (§2d item 5).
- [x] 0.5 Return-type overloading (§12, R-1 (a), R-2 (b), R-3).
- [x] 0.6 `parse_double`, `parse_long`, `parse_bool` and per-type `from_str`
  overloads; `params FromStr<T>` (X11, Q4; `Bool` accepts `true`/`false`).
- [x] 0.7 `Process` (`exit`, flushing first) and `Env` effects with host
  handlers (X10, Q3).

**Phase 1: language extensions**
- [x] 1.1 Bracketed splices: `[field.type]` in type positions and `[arm.type]`
  as a literal head; bare projections as types become a parse error (§13).
- [x] 1.2 `field.default` (X1).
- [x] 1.3 `field.docs` (and on `Struct`, `Arm`, `Union`) (X2).
- [x] 1.4 `Char`/`Str` constant slots in qualifiers (Q5c).
- [x] 1.5 Comptime qualifiers `of Field` / `of Struct`: declaration, placement
  before a field's name or a declaration, comptime `field is Q` and
  `[if field is <c> Q(c)]`; the same binder form for qualifiers on a field's
  type, `[if field.type is <lo, hi> InRange(lo, hi) *]` (round 1, Q5, X7).
- [x] 1.6 Comptime construction of a union arm (X5, Q6).
- [x] 1.7 Literal members in comptime: `[for lit in field.type.literals]`
  (X3, Q8).
- [x] 1.8 Multi-literal / multi-type `is` in ordinary code (X4, Q9).
- [x] 1.9 `explain` on predicate qualifiers (X9, Q10).
- [x] 1.10 An accumulating outcome `Ok T | Errs NonEmpty List<E>` (X12, Q11).
- [x] 1.11 Comparing two binders' names in comptime (X16).
- [ ] 1.12 Stamping a decoder: `struct S : FromArgs<self> by args` (X6, Q7).

**Phase 2: `std/args`**
- [ ] 2.1 Lexer, `ArgError`, internal runtime layer (§4.0, Q12, Q17, Q18).
- [ ] 2.2 The derive: flags, defaults, `Short`, `Positional`, help (§10.1).
- [ ] 2.3 Groups: flattened, co-occurring, exclusive (§10.3, Q15).
- [ ] 2.4 Sub-commands (§10.2, Q14).
- [ ] 2.5 Validation hook and qualifier re-testing (§5.2, Q16).
- [ ] 2.6 An example in `examples/`, the docs page, and retiring this
  document per its lifecycle.

**Blocked on the user:** nothing.

**Log** (newest first):
- 2026-10-20: 1.11 done ("Compare two binders' names in comptime"):
  [comptime-name]; 1989 tests. Next: 1.12 (`FromArgs<self> by args`).
- 2026-10-20: 1.10 done ("Add an accumulating outcome to core.result"):
  [result-errs]; 1988 tests. `errs`, `Errs`, `both`, `map2` (copying errors);
  no `all` yet. Recorded ROADMAP §2d item 7 (generic-name implicit
  misinference). Next: 1.11 (comparing binder names).
- 2026-10-20: 1.9 done ("Let a predicate qualifier explain a failure"):
  [qual-explain]; 1987 tests. `Q.explain(…)`/`Q.qualifies(…)` callable;
  `InRange` explains. Next: 1.10 (the accumulating outcome).
- 2026-10-20: 1.8 done ("Test several literals or types in one is"): [is-union];
  1985 tests. Fixed on the way: a literal `is` on a plain `Str` was always
  true; `||` narrowed nothing; a sub-union narrowing was rewrapped twice.
  Next: 1.9 (`explain`).
- 2026-10-20: 1.7 done ("Walk a literal union's members in comptime"):
  [comptime-literals]; 1983 tests. Fixed: a narrowed `(A | B)?` passed as
  `A | B` was rewrapped on Rust. Recorded (ROADMAP §2d item 6, DECISION): `?:`
  on an optional literal union; use `!is None` in the derive. Next: 1.8.
- 2026-10-20: 1.6 done ("Build union arms in comptime"): [comptime-arm]; 1981
  tests. `[arm.type](value)` calls the claim's constructor, given the subject
  as a type argument when the claim is generic. Next: 1.7 (literal members).
- 2026-10-20: 1.5 done ("Add comptime qualifiers"): [comptime-qualifier]; 1978
  tests. `Field` and `Struct` subjects; constant binders are lowercase and
  read as the constant. Next: 1.6 (building a union arm in comptime).
- 2026-10-20: 1.4 done ("Allow Char and Str constants in qualifier slots"):
  [qual-const]; 1973 tests. `Ty::Const(Const)`; constants are now checked
  against their slot's type. Next: 1.5 (comptime qualifiers).
- 2026-10-20: 1.3 done ("Read doc comments in comptime"): [comptime-docs]; 1971
  tests. A copy's `.docs`/`.default` is one value, so it is tested in a head
  (`[if field.docs !is None]`), not with `?:`. Next: 1.4 (`Char`/`Str` slots).
- 2026-10-20: 1.2 done ("Add field.default, and evaluate defaults where they are
  declared"): [comptime-default], [default-scope]; 1969 tests. `field.default`
  is `None` where a field declares none, so `take(t, field.default)` works
  either way. Fixed first: a default calling a private fn failed when
  applied in another module. Next: 1.3 (`field.docs`).
- 2026-10-20: 1.1 done ("Splice comptime types in brackets"): [comptime-inline]
  restated; 1965 tests. `[arm.type] { … }` already builds a struct arm;
  qualified and literal arms are 1.6. Also fixed a parser hang (a stray `is`
  in a handler body). Next: 1.2 (`field.default`).
- 2026-10-20: 0.7 done ("Add the Process and Env effects"): [cli-process],
  [cli-env], [rs-never-call] (a `Never` call in value position on Rust);
  1960 tests. Phase 0 is complete. Next: 1.1 (bracketed splices).
- 2026-10-20: 0.6 done ("Add parse_long, parse_double, parse_bool and
  from_str"): [str-parse]; 1958 tests. All `from_str` overloads live in
  `core.string`; `parse_double` checks one grammar on both hosts. Next: 0.7.
- 2026-10-20: 0.5 done ("Overload by return type"): [fn-overload-return];
  1957 tests. Emitted names are `from_str__Str_IntOpt` (level 6). Not
  covered (ROADMAP "Recorded"): effect members, `rename fn`. Next: 0.6.
- 2026-10-20: 0.4 done ("Report warnings in comptime copies once, on the
  written code"): [comptime-copy-warn]; 1951 tests. ROADMAP §2d is now all
  done. Next: 0.5 (return-type overloading).
- 2026-10-20: 0.3 done ("Let a program's type name shadow core whatever kind
  declares it"): [type-name-shadow]; 1950 tests. Next: 0.4.
- 2026-10-20: 0.2 done ("Collapse qualified literals like bare ones and parse
  them directly"): [type-literal-qualified]; 1948 tests. Next: 0.3 (the
  `Field` alias bug reproduced again while testing 0.2).
- 2026-10-20: 0.1 done ("Check defaults against their type and test predicate
  defaults where applied"): [default-fits], [default-predicate]; 1943 tests.
  Next: 0.2.
- 2026-10-20: rounds 1 to 4 decided; `std/args/parse.sv` deleted; the
  session's earlier work committed as one commit before phase 0.

---

## 0. What "type-safe" and "inter-argument validation" mean here

A command line is untyped input: a list of strings. A parser is a decoder from
`List<Str>` to a value the program uses. The requirements, so the options can
be scored against them:

- **T1, typed reads.** The program reads `opts.port` as an `Int`, never
  `get(parsed, "--port")` returning a string or an optional it must unwrap
  again.
- **T2, names checked at compile time.** A flag referenced anywhere (in a
  constraint, in a read, in a help override) is one that was declared. A typo
  is a compile error, not a runtime panic or silent miss.
- **T3, optionality in the type.** Required is `T`, optional is `T?`,
  defaulted is `T` with the default applied. The program never sees "required
  but absent".
- **T4, validation produces evidence.** A port in range is an
  `InRange(1, 65535) Int`, so a function that needs a valid port says so and
  cannot be called with an unchecked one ("parse, don't validate").
- **V, inter-argument rules.** The common cases, with names this document uses
  throughout:
  - *exclusive*: at most one of `--json`, `--text`;
  - *one-of*: exactly one of `--url`, `--file`;
  - *co-occurring*: `--cert` and `--key` both or neither;
  - *requires*: `--key` needs `--cert` (the one-directional form);
  - *conditional*: `--level` is required when `--mode=strict`;
  - *relational*: `--min-workers <= --max-workers`.
- **H, one source of truth.** Help, usage and error messages come from the
  same declaration that parses, so they cannot drift.
- **E, errors in the user's terms.** An error names the flag as typed
  (`--port`), says what was wrong, and the program exits non-zero.
- **P, purity and tests.** Parsing is a pure function of `List<Str>`; only
  `main` reads argv (through `Args`), so a test passes a list.

**The running example**, used in every language below so they compare:

```
serve [--host HOST] [-p|--port PORT] [--json | --text] [--cert FILE --key FILE] FILE...
```

`--host` defaults to `127.0.0.1`; `--port` is 1..65535 and defaults to 8080;
`--json`/`--text` are exclusive and default to text; `--cert`/`--key` are
co-occurring; one or more positional files.

---

## 1. What Salvo has today

| Need | What exists | Where |
|---|---|---|
| argv | `effect Args { fn args() -> List<Str> }`, `HostArgs()` handler; program name excluded | std/args.sv, [cli-args] |
| A start on the module | `Arg<T>`, `ArgGroup`, `SubCommand` and a comptime `print_help`; no parsing yet. The user's work in progress | std/args/parse.sv |
| Walking a struct | `[for field in T.fields]`, `field.name` (a literal union of the names), `field.type`, `field.kind`, `index/first/last`, type patterns, exhaustive `[when field.name]` | [comptime-fields], [comptime-pattern], [comptime-name] |
| Building a struct field by field | `T { [for field in T.fields] { [field]: expr } }` parses and expands; **no customer and no tests yet** | [comptime-access] |
| Two comptime forms | Stamped `<T is Struct>` via `by`; expanded in place (ordinary, callable, may be generic) | [comptime-bound], [comptime-generic-fn] |
| Value predicates as types | Predicate qualifiers established by `is`: `n is InRange(1, 65535)`; constructors `-> +Q T` | [qual-predicate], [is-qualifies], [qual-ctor-fn] |
| Sum types | Unions with exhaustive `when` and no `else`; literal unions `"json" \| "text"`; provenance tags (`Ok Int \| Err Str`) | [type-literal], [when-exhaustive], [qual-result-tags] |
| Failure | `[Throw<M>]` and `try { … }` giving `Ok T \| Thrown M`; `main` may not throw | [throw], [throw-not-main] |
| Per-type behaviour | Implicit parameters resolved by type (`?to_str: (T) -> Str`) | [implicit-resolve] |
| Value parsing | `parse_int(str) -> Int?` only | std/core/string.sv |

### Gaps that matter for a parser

1. **`field.default` is recorded, unbuilt** (ROADMAP §2c, for the codec slice).
   A derived parser cannot apply a default, or print one in help, without it.
   (Built 2026-10-20, step 1.2.)
2. **No per-field metadata.** Salvo has no attributes. Doc comments are on the
   AST but not exposed to comptime. So short names (`-p`), help text and
   metavariables have nowhere to live on a field.
3. **Comptime cannot construct a union arm**, only dispatch on an existing
   value (`[when value]`). A derived parser that produces an exclusive group or
   a subcommand as a union needs it.
4. **A decoder does not fit `by`.** `by` stamps at the first parameter's type;
   `from_strs(args: List<Str>) -> T` has `T` only in its return type. This is
   the same open question as SERDE.md's `from_json`.
5. **No exit status.** `exit_process` is private to the runtime. A program can
   end non-zero only by trapping (`assert!`, `x!`), so one that rejects its
   arguments, prints an error and returns from `main` exits 0, which breaks
   every shell script that checks `$?`.
6. **Value parsers.** No `parse_double`, `parse_long` or `parse_bool`; and
   [fn-overload-duplicate] refuses two same-module functions that differ only by
   return type, so `parse_arg(s: Str) -> Int?` beside `parse_arg(s: Str) ->
   Str?` cannot coexist in one module. (Round 2: return-type overloading is
   added, §12.)
7. **Comptime erases qualifiers when matching**, and cannot read a qualifier's
   arguments off a field type. A derived parser could not discover
   `InRange(1, 65535)` on `port` to re-test it. (SERDE.md SD-4 wants the same
   re-test for decoders.)
8. **[comptime-scope]**: a `[for]` copy's locals vanish after the copy, so a
   derived parser cannot declare one local per field and assemble the struct
   afterwards. Each field's value must be an expression inside the literal's
   entry, failing through `throw`. This is workable, and worth knowing early.

---

## 2. How other languages do it

Each entry shows the running example (trimmed where it repeats) and notes how
it scores on the requirements. The snippets are written for this document from
each library's documented API; they are illustrations, not tested programs.

### 2.1 Procedural

**C: `getopt_long`.** A loop over a `switch`. The library only lexes; every
type conversion and every rule is the programmer's.

```c
static struct option longopts[] = {
    {"host", required_argument, NULL, 'H'}, {"port", required_argument, NULL, 'p'},
    {"json", no_argument, NULL, 'j'},       {"text", no_argument, NULL, 't'},
    {"cert", required_argument, NULL, 'c'}, {"key",  required_argument, NULL, 'k'},
    {0, 0, 0, 0}
};
int c;
while ((c = getopt_long(argc, argv, "H:p:jtc:k:", longopts, NULL)) != -1) {
    switch (c) {
    case 'p': port = strtol(optarg, &end, 10);
              if (*end || port < 1 || port > 65535) usage(2);
              break;
    case 'j': json = 1; break;
    case 't': text = 1; break;
    /* ... */
    default:  usage(2);
    }
}
if (json && text) usage(2);           /* exclusive: by hand */
if (!cert != !key) usage(2);          /* co-occurring: by hand */
```

Scores: nothing typed, the letters in the option string and the `case`s must
agree by hand, help is a separate string. It does settle the **lexing
conventions** everyone else copies: bundled short flags (`-jt`), attached values
(`-p8080`), `--name=value`, and `--` ending options.

**Go: `flag`, and cobra.** The standard `flag` package returns a typed pointer
per flag, which is the simplest form of a *typed handle* (§4.1):

```go
host := flag.String("host", "127.0.0.1", "address to bind")
port := flag.Int("port", 8080, "port to listen on")
json := flag.Bool("json", false, "JSON output")
flag.Parse()
// *port is an int. Range, exclusivity and co-occurrence are checked by hand.
```

cobra (built on pflag) adds declarative flag groups, referenced **by name as
strings**:

```go
cmd.Flags().IntVarP(&port, "port", "p", 8080, "port to listen on")
cmd.Flags().BoolVar(&json, "json", false, "JSON output")
cmd.Flags().BoolVar(&text, "text", false, "text output")
cmd.MarkFlagsMutuallyExclusive("json", "text")
cmd.MarkFlagsRequiredTogether("cert", "key")
cmd.MarkFlagsOneRequired("url", "file")
```

Scores: T1 yes (through the pointer), T2 no (the group calls take strings,
checked when the command runs), V covers exclusive, co-occurring and one-of
declaratively, and the result is still two independent bools rather than one
value that is either JSON or text.

**Python: `argparse`.** A runtime specification producing a dynamic namespace.

```python
def port(s):
    p = int(s)
    if not 1 <= p <= 65535:
        raise argparse.ArgumentTypeError("expected 1-65535")
    return p

ap = argparse.ArgumentParser(prog="serve")
ap.add_argument("--host", default="127.0.0.1")
ap.add_argument("-p", "--port", type=port, default=8080)
out = ap.add_mutually_exclusive_group()
out.add_argument("--json", action="store_true")
out.add_argument("--text", action="store_true")
ap.add_argument("--cert"); ap.add_argument("--key")
ap.add_argument("files", nargs="+")
args = ap.parse_args()
if (args.cert is None) != (args.key is None):
    ap.error("--cert and --key go together")      # co-occurring: by hand
```

Scores: per-value validation through `type=` is neat (a function from string to
value, which is exactly the shape a typed parser wants), exclusive groups are
built in, everything else is a post-parse check. `args.port` is dynamically
typed. Typer and Click move the declaration into type hints and decorators,
which is the derive idea (§2.2) in a dynamic language.

### 2.2 Rust

**clap, derive.** The struct is the declaration; attributes carry metadata;
**doc comments become help text**.

```rust
#[derive(Parser)]
struct Serve {
    /// Address to bind
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080,
          value_parser = clap::value_parser!(u16).range(1..))]
    port: u16,
    #[arg(long, conflicts_with = "text")]
    json: bool,
    #[arg(long)]
    text: bool,
    #[arg(long, requires = "key")]
    cert: Option<PathBuf>,
    #[arg(long, requires = "cert")]
    key: Option<PathBuf>,
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

#[derive(Subcommand)]
enum Command { Serve(Serve), Stop { #[arg(long)] force: bool } }
```

Scores: T1 and T3 yes (`Option<T>` is optional, `Vec<T>` is repeated, an enum
is subcommands). T2 is only half: `conflicts_with = "text"` is a string, checked
by clap's debug assertions when the program starts in a debug build, not by the
compiler. V is declarative (`conflicts_with`, `requires`, and `ArgGroup` via
`#[group(required = true, multiple = false)]`), but the result is still
`json: bool, text: bool`. Deriving a group *as an enum* (so exclusivity is
in the type) is a long-standing open request
([clap#2621](https://github.com/clap-rs/clap/issues/2621)). The builder API
underneath is the same model with values instead of attributes.

**bpaf.** Both a derive and a **combinator** API, the combinators being the
applicative style of §2.4 written in Rust. Alternatives produce one value:

```rust
let json   = long("json").help("JSON output").req_flag(Output::Json);
let text   = long("text").help("text output").req_flag(Output::Text);
let output = construct!([json, text]).fallback(Output::Text);   // exclusive, as a value
let port   = short('p').long("port").argument::<u16>("PORT")
                .guard(|p| *p >= 1, "port 0 is reserved").fallback(8080);
let opts   = construct!(Serve { port, output }).to_options();
```

Scores: exclusivity by construction (an `Output`, not two bools), per-value
`guard`, help from the same values. It is the clearest evidence that the
combinator style works without typeclasses or higher-kinded types, which Salvo
also lacks.

**lexopt** and **pico-args** are the minimal end: a pull lexer (`Short('p')`,
`Long("port")`, `Value(...)`) and nothing else. Worth naming because whatever
Salvo builds wants this layer underneath (§4.0).

### 2.3 Kotlin

**kotlinx-cli.** Delegated properties registered on a parser object; typed
through `ArgType`.

```kotlin
val parser = ArgParser("serve")
val host by parser.option(ArgType.String, description = "Address to bind").default("127.0.0.1")
val port by parser.option(ArgType.Int, shortName = "p", description = "Port").default(8080)
val json by parser.option(ArgType.Boolean, description = "JSON output").default(false)
parser.parse(args)
// No group constraints: exclusivity, co-occurrence and range are checked by hand.
```

**Clikt.** The richest of the declarative libraries, and the one whose groups
map most directly onto types:

```kotlin
sealed class Output {
  object Json : Output()
  object Text : Output()
}

class Tls : OptionGroup() {
    val cert by option().required()
    val key by option().required()
}

class Serve : CliktCommand() {
    val host by option(help = "Address to bind").default("127.0.0.1")
    val port by option("-p", "--port").int().restrictTo(1..65535).default(8080)
    val output by option().switch("--json" to Output.Json, "--text" to Output.Text)
        .default(Output.Text)
    val tls by Tls().cooccurring()          // Tls? : both or neither
    val files by argument().multiple(required = true)
    override fun run() { /* ... */ }
}
```

Scores: T1 and T3 yes; per-value validation (`restrictTo`, `validate { }`); V
by construction for the main cases: `switch` and `mutuallyExclusiveOptions`
give one value of a sealed class, `cooccurring()` gives a nullable group, and
`groupChoice` picks a group by an option's value (the *conditional* case). T2
is good because options are referenced as Kotlin values, not strings. The
catch is the shape: the parsed result is a command object's properties, not a
plain data class the program passes around.
([Clikt options](https://ajalt.github.io/clikt/options/))

### 2.4 Functional

**Haskell: optparse-applicative.** The reference design for typed parsing. A
`Parser a` is a value; records are built with `<$>`/`<*>`; **`<|>` is
alternatives**, so exclusivity is a sum type by construction; `optional` of a
whole sub-parser is co-occurrence.

```haskell
data Output = Json | Text
data Tls    = Tls { cert :: FilePath, key :: FilePath }
data Serve  = Serve { host :: String, port :: Int, output :: Output
                    , tls :: Maybe Tls, files :: [FilePath] }

serve :: Parser Serve
serve = Serve
  <$> strOption (long "host" <> value "127.0.0.1" <> showDefault <> help "Address to bind")
  <*> option (eitherReader portR) (long "port" <> short 'p' <> value 8080 <> metavar "PORT")
  <*> (flag' Json (long "json") <|> flag' Text (long "text") <|> pure Text)
  <*> optional (Tls <$> strOption (long "cert") <*> strOption (long "key"))
  <*> some (argument str (metavar "FILE"))

portR :: String -> Either String Int
portR s = case readMaybe s of
  Just p | p >= 1 && p <= 65535 -> Right p
  _                             -> Left "expected a port, 1-65535"
```

Subcommands are another alternative:
`hsubparser (command "serve" (info (CServe <$> serve) (progDesc "Run")) <> …)`.

The important design point is that `Parser` is **Applicative and deliberately
not a Monad**. An applicative's structure is fixed before any input is seen, so
the library can walk it to print help and generate shell completions without
running it. A monadic parser could choose the next option from the value of the
previous one, and then no help text could describe it. This is the trade-off
behind requirement H: static structure buys help and completion; dynamic
dependence costs them.
([optparse-applicative](https://github.com/pcapriotti/optparse-applicative))

Scores: T1, T3 by types; T2 by construction (options are values; there is no
name to misspell in a constraint because constraints are the shape); V
exclusive, one-of and co-occurring by construction; relational and conditional
by a post-parse function. Per-value validation through `eitherReader` is the
"`type=` function" again, typed.

**Haskell: optparse-generic** derives the parser from a record through
`Generic`, and carries help **in the type**: `port :: Int <?> "Port to listen
on"`, where `<?>` wraps a field's type with a type-level string. This is the
precedent for putting metadata on the field's type rather than in an attribute
(Option C4 in §4.3).

**OCaml: cmdliner.** The same applicative model with different spelling. A
`Term.t` is built by applying a pure function to argument terms; `vflag` makes
an exclusive group a variant directly.

```ocaml
let port =
  let doc = "Port to listen on." in
  Arg.(value & opt int 8080 & info ["p"; "port"] ~docv:"PORT" ~doc)
let output = Arg.(value & vflag `Text [`Json, info ["json"]; `Text, info ["text"]])
let serve host port output tls files = (* ... *)
let cmd = Cmd.v (Cmd.info "serve") Term.(const serve $ host $ port $ output $ tls $ files)
let () = exit (Cmd.eval cmd)
```

Validation goes in a converter (`Arg.conv`, string to `result`) or in
`Term.ret`, which lets the final function answer `` `Error `` with a message.
`Cmd.eval` returns the exit code, so the "help exits 0, error exits 2" policy
lives in the library.

**Scala: decline.** optparse-applicative's model on cats, with two things worth
copying: **`validate`/`mapValidated`** attach a check (and a message) to one
option, and the parse result is a `Validated`, so **every error is reported at
once** rather than the first.

```scala
val port = Opts.option[Int]("port", "Port to listen on", short = "p")
  .withDefault(8080)
  .validate("port must be 1-65535")(p => p >= 1 && p <= 65535)
val output = Opts.flag("json", "JSON output").as(Json)
  .orElse(Opts.flag("text", "Text output").as(Text))
  .withDefault(Text)
val tls = (Opts.option[String]("cert", "TLS certificate"),
           Opts.option[String]("key", "TLS key")).mapN(Tls).orNone
val serve = (port, output, tls).mapN(Serve)
```

**F#: Argu.** The dual of everything above: **each argument is an arm of a
discriminated union**, and the parse result is queried by arm. The help text is
an exhaustive `match` over the arms, so the compiler warns when an argument has
no description.

```fsharp
type ServeArgs =
    | [<AltCommandLine("-p")>] Port of int
    | Json
    | Text
    | Cert of path: string
    | Key of path: string
    interface IArgParserTemplate with
        member s.Usage =
            match s with
            | Port _ -> "port to listen on"
            | Json   -> "JSON output"
            | Text   -> "text output"
            | Cert _ -> "TLS certificate"
            | Key _  -> "TLS key"

let results = ArgumentParser.Create<ServeArgs>().Parse argv
let port = results.GetResult(Port, defaultValue = 8080)
```

Scores: T2 strong (arms, not strings), help exhaustive by the compiler, but T1
and T3 are weaker: reading a value is a query (`GetResult`, `TryGetResult`) the
program makes after parsing, so "required" is enforced by an attribute
(`[<Mandatory>]`) rather than by the result's type. The exhaustive
description `match` is the precedent for Option C5 in §4.3.
([Argu](https://fsprojects.github.io/Argu/))

### 2.5 Two more worth a look

**Swift: ArgumentParser.** Property wrappers on a struct, plus a
**`validate()` hook** that runs after parsing and before `run()`. Exclusivity
of flags comes from an `EnumerableFlag` enum; everything else inter-argument
goes in `validate()` (the maintainers recommend exactly that for
mutually-exclusive options).

```swift
enum Output: EnumerableFlag { case json, text }   // --json | --text, one value

struct Serve: ParsableCommand {
    @Option(help: "Address to bind") var host = "127.0.0.1"
    @Option(name: .shortAndLong, help: "Port to listen on") var port = 8080
    @Flag var output: Output = .text
    @Option var cert: String?
    @Option var key: String?
    @Argument var files: [String]

    mutating func validate() throws {
        guard (1...65535).contains(port) else { throw ValidationError("port out of range") }
        guard (cert == nil) == (key == nil) else { throw ValidationError("--cert and --key go together") }
    }
}
```

**Zig: zig-clap.** The parameter list is written **as help text**, and a
comptime function parses that string into a parameter array whose result type
has one typed field per parameter:

```zig
const params = comptime clap.parseParamsComptime(
    \\-h, --help             Display this help and exit.
    \\-p, --port <usize>     Port to listen on.
    \\<str>...
    \\
);
var res = try clap.parse(clap.Help, &params, clap.parsers.default, args, .{ .allocator = gpa });
if (res.args.port) |p| { _ = p; }   // typed: ?usize
```

This is the comptime precedent closest to Salvo's model: comptime builds the
parser from a declaration and the program reads typed fields. It needs comptime
to *generate a type* from data, which Salvo's comptime does not do (it walks
declared types; it never declares one), so §4 does not pursue the string form.
([zig-clap](https://github.com/Hejsil/zig-clap))

### 2.6 Summary

| Library | Declared as | Program reads | Exclusive / co-occurring | Names checked by compiler | Help from same source | All errors at once |
|---|---|---|---|---|---|---|
| C getopt | option string + loop | variables set by hand | by hand | no | no | by hand |
| Go flag / cobra | runtime calls | typed pointers | cobra: string groups | no | yes | no |
| Python argparse | runtime calls | dynamic namespace | exclusive group; rest by hand | no | yes | no |
| Rust clap derive | struct + attributes | the struct | string `conflicts_with`/`requires`, groups; still bools | partly (debug assert) | yes | no |
| Rust bpaf | combinators or derive | the struct | alternatives give one value | yes | yes | no |
| Kotlin kotlinx-cli | delegates | properties | by hand | n/a | yes | no |
| Kotlin Clikt | delegates + groups | properties | sealed-class switch, `cooccurring` | yes | yes | no |
| Swift ArgumentParser | property wrappers | the struct | `EnumerableFlag`; rest in `validate()` | yes | yes | no |
| Haskell optparse-applicative | applicative values | the record | `<\|>` sum types, `optional` group | yes (no names) | yes | no |
| Haskell optparse-generic | record + type-level help | the record | by the record's types | yes | yes | no |
| OCaml cmdliner | applicative terms | function arguments | `vflag` variants | yes | yes | no |
| Scala decline | applicative values | the case class | `orElse`, `orNone` | yes | yes | **yes** |
| F# Argu | union of arguments | queries by arm | by hand | yes (arms) | yes, exhaustive | no |
| Zig zig-clap | help string, at comptime | generated struct | by hand | yes | yes | no |

---

## 3. What the survey teaches

**Four places the shape can be declared.**
1. *Runtime specification values* (getopt, argparse, Go, kotlinx-cli, clap's
   builder). Flexible (a plugin can add flags), but reading results is a lookup.
2. *Combinator values* (optparse-applicative, cmdliner, decline, bpaf). The
   parser is a typed value; composition builds the result type.
3. *A type, derived* (clap derive, optparse-generic, Swift, Typer). The result
   type is the declaration; metadata needs a home (attributes, wrappers, doc
   comments).
4. *A string* (zig-clap, docopt). The help text is the spec; needs type
   generation.

**Three ways inter-argument rules are enforced**, from strongest to weakest:
1. **By construction in the result type.** Exclusive and one-of become a sum
   type; co-occurring becomes an optional group; "at least one" becomes a sum
   with no empty arm or a non-empty list. The program cannot even *observe*
   both `--json` and `--text` because there is no value that holds both.
   optparse-applicative, cmdliner, bpaf, Clikt, Swift's `EnumerableFlag`.
2. **Declaratively, by reference.** `conflicts_with`, `requires`,
   `MarkFlagsMutuallyExclusive`. Good for help text ("cannot be used with")
   and generated errors; type-safe only if the references are values, not
   strings.
3. **A post-parse function.** Swift's `validate()`, decline's `validate`,
   cmdliner's `Term.ret`, and the "check by hand" of every procedural library.
   The only way to express relational and most conditional rules.

**The applicative/monadic line** (optparse-applicative's design note): keep the
parser's structure static and help, usage and completion fall out of it. Allow a
later option to depend on an earlier option's *value* and they don't. Every
library that generates good help keeps the structure static and pushes value
dependence into rule 3.

**Per-value validation is a function from string to value-or-message**
(`type=`, `eitherReader`, `Arg.conv`, `value_parser`, `guard`). Salvo's version of
that function can return a *qualified* type, which is what makes T4 possible.

**Errors.** Most libraries stop at the first error; decline accumulates. Every
mature library treats `--help` and `--version` as a successful outcome
(stdout, exit 0) distinct from an error (stderr plus usage, exit 2 by
convention).

---

## 4. Options for Salvo

### 4.0 The common foundation: a lexer

Whatever sits on top, the bottom layer is the same: split `List<Str>` into
tokens by the conventions in AP-11, with no knowledge of which flags exist
(lexopt's design). *Today*, apart from the function bodies:

```
export type Token = LongFlag | ShortFlags | Positional | EndOfOptions

export struct LongFlag   { name: Str, value: Str? }   // --port=8080 or --port
export struct ShortFlags { letters: Str, rest: Str? }  // -jt, or -p8080
export struct Positional { value: Str }
export struct EndOfOptions {}                          // --

export fn lex(args: List<Str>) [] -> List<Token> => args { … }
```

A short flag's attached value (`-p8080`) is ambiguous until the parser knows
whether `-p` takes a value, so the lexer keeps `rest` and the layer above
decides. This layer is pure Salvo, needs no language work, and is shared by
every option below.

### 4.1 Option A — a runtime specification with typed handles

Extend the shape `std/args/parse.sv` started: build a spec at runtime, parse
into a result, read through **handles** that carry the value type (Go's typed
pointer, made safe). Defining a flag answers a handle; the handle is the only
way to read the value, so a read cannot name an undeclared flag (T2 for reads).

*Today*, apart from the function bodies:

```
let spec = Mut Spec("serve")
let host = spec.str_option(Arg { long_name: "--host", default: "127.0.0.1" })
let port = spec.int_option(Arg { short_name: "-p", long_name: "--port", default: 8080 })
let json = spec.flag(Arg { long_name: "--json" })
let text = spec.flag(Arg { long_name: "--text" })
let cert = spec.str_option(Arg { long_name: "--cert" })
let key  = spec.str_option(Arg { long_name: "--key" })
spec.exclusive(json, text)          // constraints take handles, not strings
spec.together(cert, key)

let parsed = spec.parse(argv)       // Ok Parsed | Err ArgError
// ... after narrowing to Ok:
let p: Int = parsed.get(port)       // Handle<Int> in, Int out
```

A Salvo-specific strengthening: a **dependent qualifier** ties a handle to its
spec and a result to the spec it came from, the way `Idx(list) Int` ties an
index to its list ([qual-depend]): `port: ArgOf(spec) Handle<Int>` and
`parsed: ParsedFrom(spec) Parsed`, so `get` cannot read one spec's handle from
another spec's result. This would need refinements so that adding a flag to a
`Mut Spec` keeps the claims on earlier handles (as `refn add` keeps `NonEmpty`
on a list), so it is a follow-on, not a requirement.

- **For:** no language work. Runtime-extensible (flags from a plugin, a config
  file). Constraints reference handles, which are values, so they are
  typo-proof without comptime. Help renders from the spec.
- **Against:** the program assembles its own typed struct by hand (`ServeOpts
  { host: parsed.get(host), … }`), and nothing makes that exhaustive. Exclusive
  and co-occurring stay runtime rules producing two bools, not a union. Values
  are typed through a closed set of methods (`str_option`, `int_option`) or a
  generic `option<T>(arg, ?from_str: (Str) -> T?)`. Per-value evidence (T4) is
  possible only by reading and testing again.

### 4.2 Option B — applicative combinators

optparse-applicative's model in Salvo: a `Parser<T>` is a struct holding the
static description (for help) and a function that runs over the tokens.
Composition builds `T`. *Today*, apart from the bodies and the fn-typed field:

```
export struct Parser<T> {
    help: List<HelpLine>                                  // static: what --help prints
    run: (found: Found) -> Ok T | Err List<ArgError>       // dynamic: what parsing does
}

export fn option<T>(long: Str, ?from_str: (s: Str) -> T?) -> Parser<T?> { … }
export fn flag(long: Str) -> Parser<Bool> { … }
export fn default<T>(p: Parser<T?>, value: T) -> Parser<T> { … }
export fn required<T>(p: Parser<T?>) -> Parser<T> { … }
export fn together<T>(p: Parser<T>) -> Parser<T?> { … }      // all or none (co-occurring)
export fn either<A, B>(a: Parser<A>, b: Parser<B>) -> Parser<A | B> { … }
export fn map2<A, B, R>(a: Parser<A>, b: Parser<B>, f: (a: A, b: B) -> R) -> Parser<R> { … }
export fn map3<A, B, C, R>(…) -> Parser<R> { … }             // and so on to some N
```

The running example (*sketch*: the explicit type arguments on `option<Str>`
and the helper names are illustrative):

```
let host   = option<Str>("--host").default("127.0.0.1")
let port   = option<Int>("--port").short("-p").check_range(1, 65535).default(8080)
let output = either(flag_as("--json", "json"), flag_as("--text", "text")).default("text")
let tls    = together(map2(option<Str>("--cert").required(), option<Str>("--key").required(),
                           (c, k) -> Tls { cert: c, key: k }))
let serve  = map4(host, port, output, tls,
                  (h, p, o, t) -> ServeOpts { host: h, port: p, output: o, tls: t })
```

Salvo's unions do something Haskell cannot: **`either` returns `A | B`
directly**, with no sum type to declare first, and an exhaustive `when` over
the result. `either(flag_as("--json", "json"), flag_as("--text", "text"))` is a
`Parser<"json" | "text">`.

- **For:** no language work. Help, usage and (later) completion come from the
  static half. Errors accumulate naturally (decline's lesson). Exclusive,
  one-of and co-occurring are by construction. Per-value checks can answer
  qualified types: `check_range` could produce `Parser<InRange(lo, hi) Int>`,
  if the constructor lives with the qualifier ([qual-ctor-same-file]) or the
  check goes through `is`.
- **Against:** `mapN` boilerplate (no variadic generics), and the lambda
  assembling the struct is positional: two `Str` options swapped in
  `(h, c) -> …` type-check. Nothing makes it exhaustive over the struct's
  fields. Verbose for a wide struct. A `Parser<T>` holds a function, so it is a
  value Salvo has to carry through generics; lambdas and fn-typed fields exist
  ([rs-fn-field]), and the static-identity rule ([cmp-carry]) applies only to
  fns carried in *types*, so this should work, but no std code does it yet.

### 4.3 Option C — derive from a struct (and a union)

The clap-derive / optparse-generic / Swift route, with comptime as the
mechanism. The program declares the type it wants; a comptime fn walks it and
builds the parser and the help. *Sketch*:

```
import args

struct Tls { cert: Str, key: Str }

struct ServeOpts {
    // Address to bind.
    host: Str = "127.0.0.1"
    // Port to listen on.
    port: InRange(1, 65535) Int = 8080
    // Output format.
    output: "json" | "text" = "text"
    // TLS; give both or neither.
    tls: Tls?
    // Files to serve.
    files: NonEmpty List<Str>
}

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    let parsed = from_strs<ServeOpts>(args())     // Ok ServeOpts | Err ArgError
    when parsed {
        is Ok { serve(parsed) }
        is Err { report(parsed) }                 // prints, and should exit 2 (AP-9)
    }
}
```

**The mapping, by type** (each line is a candidate rule for AP-6):

| Field type | Command line | Rule it enforces |
|---|---|---|
| `Bool` (default `false`) | `--name` switch | |
| `T` with a default | `--name VALUE`, optional | |
| `T` without a default | `--name VALUE`, required | T3 |
| `T?` | `--name VALUE`, optional, `None` when absent | T3 |
| `List<T>` | repeatable `--name V --name V` | |
| a literal union `"json" \| "text"` | `--json` \| `--text`, or `--output json` (AP-6) | exclusive |
| a qualified type `InRange(1, 65535) Int` | value re-tested on parse | T4 |
| a struct `S` | its fields, flattened | |
| an optional struct `S?` | its fields, all or none | co-occurring |
| a union of tagged arms `FromUrl Str \| FromFile Str` | `--from-url` \| `--from-file`, exactly one | one-of |
| the top-level type is a union of structs | subcommands, by arm name | |
| a `List<T>` or `NonEmpty List<T>` marked as positional (AP-6) | positionals | at least one, if `NonEmpty` |

Names follow the field: `host` → `--host`, `max_workers` → `--max-workers`. `field.name` is a literal of the union of the struct's field
names ([comptime-name]), so the derive knows every flag name at compile time
and a rule that names one can be checked (§5.3).

**The comptime body, in outline** (*sketch*: needs `field.default`, union-arm
construction, and a home for `from_strs` in `by`):

```
comptime fn from_strs<T is Struct>(found: Found) [Throw<ArgError>] -> T {
    return T {
        [for field in T.fields] {
            [field]: take<[field.type]>(found, flag_name(field.name), field.default)
        }
    }
}
```

Each entry is one expression, because a `[for]` copy's locals do not survive
the copy ([comptime-scope]); `take` throws an `ArgError` for a missing or
malformed value, and resolves `?from_str: (Str) -> E?` at the field's concrete
type, the way `core.auto`'s stamps resolve `cmp` per field. The help renderer is
a second comptime walk over the same fields, which is what keeps H true.

**Metadata.** The mapping covers long names, optionality and types by
convention. Short names, help text and metavariables need a home, and this is
the main language-design question of the option (AP-2):

- **C1, convention only.** No short names, no help text. Too weak on its own.
- **C2, doc comments as help.** Expose a field's doc comment to comptime
  (`field.docs: Str?`). clap and Swift do this. A small, general addition (a
  derived JSON schema or a derived `describe` would use it too).
- **C3, field attributes.** `@short("p") port: Int`. Salvo has none, and
  SERDE.md's survey leans away from them ("Salvo has no attributes and wants
  none" is its working stance, not a decision). Largest change.
- **C4, metadata in the type.** optparse-generic's `Int <?> "help"`. Salvo's
  closest tool is a qualifier with constant slots, `port: Short("p") Int`, but
  qualifiers are claims about *values*, and `Short("p")` claims nothing about
  the value. The honest reading would be a provenance qualifier ("this value
  came from `-p`"), which is a stretch. It would also need comptime to read a
  field type's qualifiers and their arguments (gap 7). Recorded, not
  recommended.
- **C5, an exhaustive companion function.** Argu's exhaustive `Usage` match,
  in Salvo: an expanded-in-place comptime fn over the struct's field names,
  which the compiler refuses to accept until every field is described. Works
  *today*, given an `ArgInfo` struct whose fields all have defaults:

  ```
  comptime fn arg_info(name: Str) -> ArgInfo => name {
      [for field in ServeOpts.fields] {
          [when field.name] {
              is "port" { if name == field.name { return ArgInfo { short: "-p" } } }
              is "host" | "output" | "tls" | "files" {}
          }
      }
      return ArgInfo {}
  }
  ```

  The exhaustiveness holds today (a new field is an error until an arm names
  it); the shape is clumsy, and a parameter typed as the struct's field names
  (`name: ServeOpts.field_name`, *sketch*) would make it a plain `when`. It
  has to be a per-struct, expanded-in-place fn: the derive itself is stamped,
  and a stamped fn may not dispatch on `[when field.name]` ([comptime-name]).
  The cost is a second declaration per struct, kept honest by exhaustiveness
  rather than by proximity.

- **For:** the declaration is the type the program uses: no hand assembly, no
  `mapN`, exhaustive by construction (a new field is parsed, documented by C2
  or forced by C5, and in help). T1 to T4 and V's by-construction cases all come
  from Salvo's existing types: unions, `T?`, qualifiers, `NonEmpty`. Flag names
  are field names, known at compile time.
- **Against:** the most new work: `field.default`, union-arm construction, a
  `by` site for a decoder, qualifier re-testing in comptime, and a metadata
  decision. Relational rules still need §5.2's validating constructor. Field
  names fix the spelling of flags (`--max-workers` from `max_workers`), so a
  legacy interface with odd names needs the metadata mechanism too.

### 4.4 Option D — a usage string (docopt, zig-clap)

Comptime parses `"serve [--host HOST] [-p|--port PORT] …"` into a parser and a
result type. Salvo's comptime walks declared types and never declares one, so
the result type would have to be written separately and checked against the
string, losing the point. Recorded for completeness; not pursued.

### 4.5 Option E — derive on top of combinators

bpaf's and clap's actual architecture: one runtime model underneath, with a
derive that generates calls into it. In Salvo: the lexer (§4.0), then Option B's
`Parser<T>` (or Option A's spec) as the std foundation that anyone can use by
hand, then Option C's comptime derive that *expands into* those combinators.
Hand-written and derived parsers then share lexing, value parsers, help
rendering and error messages, and the derive can be built after the foundation,
when its language prerequisites land.

---

## 5. Inter-argument validation in Salvo

The menu, matched to the cases in §0:

### 5.1 By construction: the result type cannot hold an invalid state

*Today* (the types; deriving a parser for them is Option C):

```
provenance qualifier FromUrl of Str
provenance qualifier FromFile of Str

struct Tls { cert: Str, key: Str }

struct FetchOpts {
    output: "json" | "text" = "text"     // exclusive: one value, never both
    tls: Tls?                             // co-occurring: both or neither
    source: FromUrl Str | FromFile Str    // one-of: exactly one; no None arm, so required
    tags: NonEmpty List<Str>              // at least one
}
```

and downstream code is an exhaustive `when` that the compiler checks:

```
when source = opts.source {
    is FromUrl { fetch_url(source) }
    is FromFile { read_file(source) }
}
```

| Rule | Type |
|---|---|
| exclusive (at most one) | `A \| B \| None`, or a literal union with a default |
| one-of (exactly one) | `A \| B` |
| co-occurring | `S?` for a struct `S` of the group |
| requires (`--key` needs `--cert`) | `S?` where `S { cert: Str, key: Str? }` |
| at least one (repeated) | `NonEmpty List<T>` |
| conditional by value (`--level` when `--mode=strict`) | a union of structs keyed by the mode: `Strict { level: Int } \| Lenient` (Clikt's `groupChoice`) |
| relational (`min <= max`) | not expressible by shape: §5.2 |

The parser's job for these is mechanical: a union field accepts exactly one of
its arms' flags, an optional struct accepts all of its required fields or none.
Errors are generated: "`--cert` requires `--key`", "`--url` cannot be used with
`--file`".

### 5.2 By evidence: a qualifier on the parsed value

For what shape cannot say, Salvo can still make the check visible in types. A
predicate qualifier over the whole options struct (*today*):

```
qualifier Consistent of ServeOpts {
    fn qualifies(o: ServeOpts) -> Bool {
        return o.min_workers <= o.max_workers
    }
}

fn run(opts: Consistent ServeOpts) [Console] => opts { … }

// in main, after parsing:
if opts is Consistent ... else {
    printerr("--min-workers must not exceed --max-workers")
    return
}
run(opts)
```

`run` cannot be called with unchecked options. The weakness is that `qualifies`
answers `Bool`, so the message is written separately from the check. The
alternative is a **validating constructor** in the qualifier's file, which
throws with a message and answers the claim (*today*, [qual-ctor-fn] and
[throw]):

```
qualifier Valid of ServeOpts

fn validate(o: ServeOpts) [Throw<ArgError>] -> +Valid ServeOpts => o {
    if o.min_workers > o.max_workers {
        throw(conflict("--min-workers", "--max-workers", "must not exceed"))
    }
    return o
}
```

(`conflict` stands for an `ArgError` constructor, §6.) This is Swift's
`validate()` hook, typed: the result is a different type from
the unvalidated struct, so forgetting to validate is a compile error at the
first function that needs `Valid ServeOpts`. The derive could call a
user-supplied `validate` if one exists for `T` (resolved like any implicit),
so the parse result is `Valid ServeOpts` directly.

Per-field values work the same way: a field typed `InRange(1, 65535) Int` is
tested with `is` as it is parsed (gap 7: the derive needs to see the
qualifier), and a value that fails is an `ArgError` naming the flag.

### 5.3 By declaration: rules that reference flags

clap's `conflicts_with`, cobra's `MarkFlags…`. Useful mainly for help ("`--json`
cannot be used with `--text`") and for rules between flags in *different*
groups that the shape does not relate. In Option A they take handles (values,
so typo-proof today). In Option C they would take field names, which are a
literal union the compiler knows, but only inside comptime: naming that type
in ordinary code (`ServeOpts.field_name` above) would be a new feature. Since
§5.1 covers the common cases by type and §5.2 the rest with messages, this
layer is optional.

### 5.4 Recommendation for validation

§5.1 as the primary mechanism (the parser derives the rule from the type),
§5.2's validating constructor for relational and conditional rules, and §5.3
only if help output turns out to need it. This is the functional languages'
answer (shape first, function second), with Salvo's qualifiers turning "a
function after parsing" into a type the rest of the program can demand.

---

## 6. Cross-cutting details

**Errors.** One union for every way parsing fails, so a caller can match it and
the renderer can word it:

```
export type ArgError = Unknown | MissingValue | Missing | Invalid | Conflict
                     | HelpRequested | VersionRequested
```

with `Invalid { flag: Str, value: Str, reason: Str }` and so on. Help and
version are outcomes in this union, not side effects, so a parser stays pure;
the caller (or a std `run_cli` helper) prints them to stdout and exits 0, and
prints the others to stderr with usage and exits 2.

**First error or all.** decline reports every problem in one run. Option B
gets that for free; Option C can collect if `take` returns per-field results
instead of throwing, at the cost of the "one expression per entry" shape.
AP-8.

**Exit status.** Gap 5. A CLI parser is the first std feature that needs an
exit code. Options in AP-9.

**Value parsers.** Gap 6. Each field type needs `(Str) -> T?`: the derive and
the combinators would resolve it as an implicit (`?from_str`). Because two
same-module functions cannot differ by return type alone, each type's
`from_str` needs either its own module (`Int`'s in `core.basic`, `Str`'s in
`core.string`, which [fn-overload-duplicate] permits since it is per-module),
or a distinct name. Separate-module resolution through [implicit-resolve]'s
step 3 is unverified; AP-10. `parse_double`, `parse_long` and `parse_bool` are
owed either way.

**Where argv is read.** Only `main`, through `Args`. The parser takes
`List<Str>`, so tests pass a list directly and need no fake handler.

**Subcommands.** The top-level type is a union of structs, one per command,
named by arm (`type Command = ServeCmd | StopCmd`, `serve`/`stop` by AP-6's
naming rule). Global options before the command are a struct wrapping the union
(`struct Cli { verbose: Bool = false, command: Command }`).

**Help layout.** `std/args/parse.sv`'s `print_help` already walks `ArgGroup`
with an exhaustive `[when field.name]`; whichever option is chosen, its
rendering (columns, required/optional, defaults) is reusable as the help layer.

**Out of scope for a first version:** environment-variable fallback
(clap's `env`), config files, shell completion. All three are easier with
static structure (Options B, C, E) than without.

---

## 7. Extensions to existing Salvo features

Each extension below makes command-line parsing fit Salvo more naturally, and
each is **justified on its own**: the test applied is "would this be worth
building if `std/args` never shipped?" Each lists what it is, what it does
for parsing, at least two other uses, what it costs, and a recommendation.
Several are language-design calls (AGENTS.md's first invariant); none is
decided.

| | Extension | Closes gap (§1) | Language or std | Other main users |
|---|---|---|---|---|
| X1 | `field.default` in the comptime model | 1 | language (comptime model) | codecs, derived `to_str`, builders |
| X2 | `field.docs`: doc comments in the comptime model | 2 | language (comptime model) | schemas, config templates |
| X3 | A literal union's literals visible to comptime | — | language (comptime model) | string enums in codecs, `all_values` |
| X4 | `s is "a" \| "b"` in ordinary code | — | language (syntax) | every string dispatch |
| X5 | Comptime construction of a union arm | 3 | language (comptime) | union decoders, generators |
| X6 | A `by` site for a type in return position | 4 | language (`by`) | `from_json`, `default()`, generators |
| X7 | Comptime patterns over qualifiers | 7 | language (comptime) | decoders, redaction, schemas |
| X8 | A type naming a struct's field names | — | language (types) | patches, sort keys, form errors |
| X9 | Predicate qualifiers that can say why | — | language (qualifiers) | `assert!`, `expect`, decoders |
| X10 | `exit` and environment variables as effects | 5 | std (+ runtime) | every tool, tests, config |
| X11 | A `FromStr<T>` group, the dual of `ToStr<T>` | 6 | std (+ a resolution check) | config, CSV, text protocols |
| X12 | An accumulating outcome | — | std | form and config validation |
| X13 | Field attributes | 2 | language (syntax) | serde-style renames, ORMs |
| X14 | Comptime-declared types (`Partial<T>`) | — | language (comptime) | patches, layered config |
| X15 | Marker aliases: a generic alias as an annotation | 2 | language (a rule + literal binders) | codec renames, schema hints, units |
| X16 | Comparing two binders' names | — | language (comptime) | merging derives, view structs |
| X17 | Declared annotations (`Short('p') T`), with `Char`/`Str` constant slots | 2 | language (a declaration kind) | codec renames, units, schema formats |

X13 and X14 are recorded for completeness and not recommended now.

### X1. `field.default` in the comptime model

**What.** `Field` gains `default`, the field's declared `= expr` as a
compile-time optional: usable as an expression in a copy where the field has
one, and testable (`[if field.default !is None]`) where it may not. Already
recorded in ROADMAP §2c for the codec slice.

**For parsing.** A defaulted field is an optional flag, its default is applied
when the flag is absent, and help prints "(default: 8080)". Without it the
derive must treat every non-`T?` field as required.

**Elsewhere.**
- *Decoders* (SERDE.md SD-4): a key absent from old data reads as the field's
  default, which is how a struct gains a field without breaking readers.
- *A derived `to_str` or diff* that prints only the fields that differ from
  their defaults, which keeps logs of wide config structs readable.
- *Builders and resets*: a comptime `reset(s: Mut S)` that restores every
  defaulted field, exhaustive over the struct.
- *Schemas*: a JSON Schema or OpenAPI `default` from the declaration.

**Cost.** The default is an expression checked in the struct's file; in a copy
it is evaluated where the copy is, so it must be evaluated in the declaring
file's scope (the same scoping care [comptime-root-visible] took for type
names). A default that reads a private name of its module needs that name
reachable from the expansion site. **Recommendation:** build (AP-3).

### X2. `field.docs`: doc comments in the comptime model

**What.** `Field` (and `Arm`, `Struct`, `Union`) gains `docs: Str?`, the doc
comment already carried on the AST for hover [doc-comment].

**For parsing.** Help text written once, next to the field it describes, as in
clap and Swift.

**Elsewhere.**
- *Schemas and API descriptions*: JSON Schema `description`, OpenAPI fields,
  generated from the same comment the IDE shows.
- *Config templates*: a derived "write a commented default config file" for a
  config struct.
- *Wire documentation*: describing an actor protocol's message fields.
- *Derived `describe`* for diagnostics or admin pages.

**Cost.** Small: a projection of data the parser already keeps. The one
question is the comment's text form (raw lines, or joined and trimmed), which
should match what hover prints. Doc comments become load-bearing, so editing
one changes generated output; that is the intent, but it should be stated.
**Recommendation:** build (AP-2's recommendation rests on it).

### X3. A literal union's literals visible to comptime

**What.** Today a base type's literals collapse into one runtime arm
([union-arm-identity], [type-literal]), so `"json" | "text"` walks as one arm.
The extension exposes the literals to comptime, for example `[for lit in
field.type.literals]` with `lit.value` a literal of the union, or the
literals as arms of their own with `arm.literal`. The runtime representation
does not change.

**For parsing.** A field `output: "json" | "text"` needs its choices to accept
`--output json`, refuse `--output xml` with "expected one of json, text", and
list them in help.

**Elsewhere.**
- *String enums in codecs*: a JSON or DynamoDB attribute holding one of a
  fixed set of strings decodes into a literal union, and the decoder needs the
  set.
- *`all_values()`*: the list of a literal union's members, which Kotlin's
  `entries` and Rust's strum `EnumIter` provide; used for menus, validation
  messages and exhaustive tests.
- *Derived `cmp`* by declaration order for an ordered literal union (severity
  levels `"debug" | "info" | "warn"`).

**Cost.** A new projection; the comptime model must describe a literal
(`comptime struct Literal { value: …, index: Int, … }`). No backend change.
**Recommendation:** build alongside the derive; it is the cheapest way to
make literal unions first-class in generated code.

### X4. `s is "a" | "b"` in ordinary code

**What.** Comptime already accepts `field.name is "a" | "b"` [comptime-name].
Ordinary code does not: `s is "json" | "text"` is a parse error, and the
working spelling is `s is "json" || s is "text"`, which does narrow a plain
`Str` to `"json" | "text"` (checked 2026-10-20). (Corrected when building it,
step 1.8: it type-checked but did not narrow, and the literal test itself
passed for every string.) The extension accepts the
union form in an ordinary `is`, meaning the disjunction, and narrowing to the
union of the literals.

**For parsing.** Turning a value string into a literal-union field is one
test, and a hand-written parser reads as the comptime one does.

**Elsewhere.**
- *Any string dispatch*: HTTP methods, log levels, REPL commands, config keys.
- *Consistency*: one spelling for one test in both comptime and ordinary code.
- The same form for type arms (`x is Int | Str` on a three-arm union) would
  narrow to a sub-union; worth deciding together.

**Cost.** Parser and the narrowing rule; `|` after `is` must stop at `{` and
at `&&`/`||`. **Recommendation:** build; small and generally useful.

### X5. Comptime construction of a union arm

**What.** Comptime can dispatch on a union value it has (`[when value] {
[arm] { … } }`) but cannot produce one. The extension lets a copy over
`T.arms` build an arm's value: for a struct arm, a literal of `arm.type`
(`[arm.type] { [for f in arm.type.fields] { [f]: … } }`); for a qualified arm,
the arm's constructor; for `None`, `None`.

**For parsing.** Subcommands (`type Command = Serve | Stop`) and one-of groups
(`FromUrl Str | FromFile Str`) are unions the derive must produce.

**Elsewhere.**
- *Union decoders*: SERDE.md's name-tagged unions decode by reading the tag
  and building that arm.
- *Generators*: a derived `arbitrary()` for property-based tests picks an arm
  and builds it; a derived `examples()` builds one value per arm for
  exhaustive tests.
- *A derived `default()`* for a union (its first arm, if that arm has one).

**Cost.** The struct-literal form exists for `T` already ([comptime-access],
untested); this extends it to an arm's type and adds the per-arm return. The
checker sees ordinary code after expansion, so no new checking rule.
**Recommendation:** build (AP-5).

### X6. A `by` site for a type in return position

**What.** `by` stamps a comptime fn at the type of the stamped fn's first
parameter. A decoder (`from_strs(args: List<Str>) -> T`) or a constructor
(`default() -> T`) has `T` only in its result. Three shapes, as AP-4 lists:
stamp at the return type when there is no `T` parameter; an obligation clause
whose member returns `self` (`struct ServeOpts : FromArgs<self> by args`); or
a written type argument.

**For parsing.** The derive becomes `fn parse_serve(args: List<Str>) [] ->
ServeOpts by args`, or one clause on the struct.

**Elsewhere.**
- *`from_json`, and `decode<T>` as an implicit* (ROADMAP §2c): the same shape,
  today blocked on the same question.
- *`default() -> T by auto`*: a derived default instance from the fields'
  defaults (X1); useful for builders, test fixtures and config.
- *`arbitrary() -> T by auto`* for property tests (with X5 for unions).
- *Row and environment mappers*: `from_row(row: Row) -> T`, `from_env() -> T`.

**Cost.** A rule in the expansion's target selection and in obligation
clauses; the interaction with implicit resolution (a `?decode: (Bytes) -> T?`
slot filled by a stamp) is the part to design. **Recommendation:** decide once
for decoders generally, not for `args` alone.

### X7. Comptime patterns over qualifiers

**What.** Type patterns erase qualifiers today ([comptime-pattern]). The
extension lets a pattern test and bind them: `[if field.type is <lo, hi>
InRange(lo, hi) *]` binds the constant arguments; `[if field.type has
Secret]` (spelling open) tests a provenance qualifier.

**For parsing.** A field `port: InRange(1, 65535) Int` is re-tested as it is
parsed, the error names the range, and help prints it.

**Elsewhere.**
- *Decoders* (SERDE.md SD-4): re-test a predicate qualifier on read, and refuse
  to derive a decoder for a constructive or provenance claim nothing in the
  data can establish. SD-4 needs exactly this.
- *Redaction*: a derived `to_str` that prints `***` for a `Secret Str` field,
  so a log line never leaks it.
- *Schemas*: `minimum`/`maximum` from `InRange`, `minItems: 1` from
  `NonEmpty`.
- *Generators*: an `arbitrary()` that respects `InRange`.

**Cost.** Constant arguments are values, not types, so binders over them are a
new kind of comptime binder; the model needs a `Qualifier` description. Medium.
**Recommendation:** build when the codec slice does; parsing can start without
it by making range checks explicit in the combinators.

### X8. A type naming a struct's field names

**What.** The literal union of a struct's field names, which comptime already
computes for `field.name` [comptime-name], made nameable in ordinary code:
`FieldOf<ServeOpts>` is `"host" | "port" | "output" | "tls" | "files"`
(spelling open; TypeScript calls it `keyof`).

**For parsing.** Metadata companions (Option C5) become a plain exhaustive
`when` over `FieldOf<ServeOpts>`; declarative rules (§5.3) and error values
(`Missing { field: FieldOf<T> }`) name fields checked by the compiler.

**Elsewhere.**
- *Patches and change sets*: SERDE.md SD-5's derived change set is a
  `Map<Str, Attr?>`; keyed by `FieldOf<PersonItem>` it cannot name a field
  that does not exist.
- *Validation errors keyed by field*: `Map<FieldOf<SignupForm>, Str>` for a
  form, so a renamed field breaks the error table at compile time.
- *Sort and query keys*: `sort_by(people, "age")` where the key is a
  `FieldOf<Person>`.

**Cost.** A type former whose meaning depends on another declaration; adding
a field widens it, which breaks exhaustive `when`s over it (the point, and a
compatibility consideration for library structs). Reading a field *by* such a
name (`person.[key]` at runtime) is a separate, larger feature and not part of
this. **Recommendation:** worth a decision on its own merits; parsing does not
need it first.

### X9. Predicate qualifiers that can say why

**What.** `fn qualifies(x) -> Bool` answers only yes or no. The extension adds
an optional member, `fn explain(x, …) -> Str`, with the same parameters,
called only when a test fails and a message is wanted:

```
export qualifier InRange(lo: Int, hi: Int) of Int {
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool { return n >= lo && n <= hi }
    fn explain(n: Int, lo: Int, hi: Int) -> Str { return "${n} is not in ${lo}..${hi}" }
}
```

**For parsing.** "invalid value for `--port`: 70000 is not in 1..65535" comes
from the qualifier the field already declares, and §5.2's `Consistent`
qualifier can say which rule failed.

**Elsewhere.**
- *`assert!(x is Q)`* traps with the explanation instead of a bare failure.
- *Tests*: `expect(n is InRange(0, 65535))` fails with the reason when no
  message is written.
- *Decoders*: SD-4's re-test reports why a stored value no longer qualifies.
- *API input validation* in a service, where the message goes back to the
  caller.

**Cost.** One optional, signature-checked member ([qual-predicate] validates
`qualifies` the same way), and a std function to reach it
(`explain(x, Q)`-style spelling open). **Recommendation:** build; cheap and
useful wherever predicates fail visibly.

### X10. `exit` and environment variables as effects

**What.** Two capabilities in the [cli-args] mould, separate so a function can
have one without the other:
- `effect Process { fn exit(code: Int) -> Never }`, with a host handler;
- `effect Env { fn env(name: Str) -> Str? }`, with a host handler.

**For parsing.** Errors exit 2 and help exits 0 (AP-9); an option can fall
back to an environment variable (clap's `env`).

**Elsewhere.**
- *Every tool and script*: a non-zero exit status is how a shell, CI or a
  supervisor (systemd, Kubernetes) learns a program failed. Salvo has no way to
  set one today short of a trap.
- *Tests*: a fake `Process` records the code instead of exiting; a fake `Env`
  supplies variables without touching the real environment.
- *Configuration*: twelve-factor services read their settings from the
  environment.

**Cost.** `exit` must flush buffered console output first, and must have a
stated rule for live linear values (it is a `Never` path like a trap, so the
rule [throw] gives for throw paths is the precedent to follow). Both backends
have direct host calls. **Recommendation:** build both; `Process.exit` is the
one prerequisite for a usable CLI that no option avoids.

### X11. A `FromStr<T>` group, the dual of `ToStr<T>`

**What.** std has `ToStr<T>` and per-type `to_str`. The dual is `params
FromStr<T> { fn from_str(s: Str) -> T? }` with one `from_str` per type in that
type's own module (`Int` in `core.basic`, `Double` likewise, `Bool`, `Str`),
plus the missing `parse_double`, `parse_long`, `parse_bool` they wrap.

**For parsing.** Every value parser resolves as `?from_str` at the field's
type, in a hand-written parser, in the combinators and in the derive, with no
table of names.

**Elsewhere.**
- *Config files, CSV columns, environment variables* (X10), *REPLs* and *text
  protocols*: all turn a string into a typed value.
- *A derived `from_str` for structs and literal unions* (with X3) for
  user-defined types, the dual of the derived `to_str` std already ships.

**Cost.** [fn-overload-duplicate] refuses two same-module functions that
differ only in return type, so the per-type `from_str`s must live in separate
modules, and [implicit-resolve]'s step 3 must pick the candidate by the slot's
return type. That is unverified and is the first thing to test. A bare
`from_str("3")` with no expected type stays ambiguous, which is acceptable
since the call is normally reached through the implicit. **Recommendation:**
add the three parsers now; adopt the group once the resolution is confirmed.

### X12. An accumulating outcome

**What.** A std shape for "every error, not the first": `Ok T | Errs
NonEmpty List<E>`, with `both`/`map2` that keep collecting after a failure.
std only.

**For parsing.** Report every bad flag in one run (AP-8, decline's behaviour).

**Elsewhere.** Form and config validation, batch imports ("rows 3, 17 and 40
failed"), linters, and any check where the user fixes all problems at once.

**Cost.** Small; overlaps with `throw` (which stops at the first) and with
Option B's own error list, so it should be one std shape, not one per
library. **Recommendation:** build with the combinators, in std rather than in
`args`.

### X13. Field attributes (recorded, not recommended now)

**What.** `@short("p") port: Int`: arbitrary metadata on a declaration,
readable by comptime.

**Elsewhere.** The widest reach of any item here: serde-style renames, ORM
column names, test markers, deprecation. That breadth is also the cost: an
open-ended annotation language is hard to keep checked, and SERDE.md's survey
takes "Salvo has no attributes and wants none" as its working stance. X1, X2
and an exhaustive companion (C5, or X8) cover the parsing need without it.

### X14. Comptime-declared types (recorded, not recommended now)

**What.** Comptime that *declares* a type from another: `Partial<T>` (every
field optional) or `Parsers<T>` (every field a `Parser<[field.type]>`), the
"higher-kinded data" pattern of Haskell's barbies library and TypeScript's
mapped types.

**Elsewhere.** Patches (a `Partial<Person>` holds only the fields to change),
layered configuration (defaults, then file, then environment, then flags,
merged field by field), and form state.

**Why not now.** Today's comptime walks declared types and never makes one;
types that exist only after expansion change what resolution, hover and both
emitters can assume, which is a much larger step than any item above.

### X15. Marker aliases: a generic alias as a comptime-visible annotation

**What.** Found while designing §10 (probed 2026-10-20): a generic alias such
as `type Short<C, T> = T` is already **transparent to ordinary code** (a
`Short<"p", Int>` field reads, defaults, adds, stamps through `by auto` and
emits on both backends as an `Int`) and **visible to comptime patterns**
(`[if field.type is <C, E> Short<C, E>]` matches, and `[if C is "p"]` tests the
literal type argument). That is an annotation mechanism with no new syntax.
Two things would make it a feature rather than an accident:
1. **State the rule.** [comptime-pattern] says matching expands an
   *argument-less* alias; that an alias with arguments stays visible is
   implied, not stated, and nothing tests it. Make it a rule with tests, so
   std can build on it.
2. **Read a literal binder as its value.** A binder bound to a literal type
   (`C` = `"p"`) can be tested but not used as a value: `"${C}"` is "no
   variable named `C`". The extension lets a binder whose type is a single
   literal stand for that literal in an expression.

**For parsing.** Short names and positionals on the field they describe:
`port: Short<"p", Int>`, `files: Positional<List<Str>>` (§10.1).

**Elsewhere.**
- *Codec renames*: SERDE.md SD-4 needs a per-field key spelling for legacy
  schemas and rejected per-field syntax; `created: Key<"createdAt", Long>`
  is that spelling with none.
- *Wire and schema hints*: `Fixed<8, Bytes>` for a fixed-width field, or
  `Format<"email", Str>` for a JSON Schema `format`.
- *Units in generated output*: `Unit<"ms", Long>` printed by a derived
  `to_str` or a metrics exporter.
- *Any derive*: one convention for per-field metadata across std, instead of
  each derive inventing its own.

**Cost.** Small: a rule, tests, and the value use of a literal binder. The
convention it creates needs one discipline: a marker is meaningful only at
the top of a field's type (the derive refuses `List<Short<"p", Int>>`), and
two markers nest in a fixed order the derive documents. Unlike a qualifier
(C4), a marker claims nothing about the value, so there is no soundness
question. **Recommendation:** build; it is what §10 rests on, and it answers
SERDE.md's rename question without attributes.

### X16. Comparing two binders' names in comptime

**What.** `[if a.name == b.name]` for binders from two different walks
(today only `.index` comparisons relate two binders).

**For parsing.** A flattened group (§10.3) must not reuse a parent's flag
name; the derive reports the clash at the declaration instead of at run time.

**Elsewhere.** Any derive that merges two structs (a patch applied to a
record, a view struct checked against its source) needs "same field name";
SERDE.md's second-struct shapes (an item type beside a domain type) are
checked the same way.

**Cost.** Small, inside the expansion's own condition evaluation.
**Recommendation:** build with the derive; without it the clash is a run-time
check in the generated parser, caught by its first test.

### X17. Declared annotations

**What.** A declaration kind for field metadata written in qualifier position,
`port: Short('p') Int`: typed slots (with `Char` and `Str` constants, which
qualifiers lack today), no claim on the value, dropped when the field is
read, visible to comptime. Designed, with the probes behind it, in §10.1a.

**Elsewhere.** The same uses as X15 (codec renames, units, schema formats), in
a declared, typed vocabulary rather than a convention over aliases; `Char`
and `Str` constant slots also serve ordinary qualifiers (`Prefix("arn:")
Str`).

**Cost.** Larger than X15: a declaration kind, constant slots of two new
types, placement rules, and comptime access. **Recommendation:** the user's
call under D-1; X15 is the cheaper path to the same derive.

---

## 8. A plausible build order

Independent of the recommendations, this is the order the dependencies impose:

1. The lexer (§4.0), `ArgError`, and the missing value parsers. No language
   work; useful to any option.
2. An exit-status mechanism (AP-9, X10), and X11's value parsers.
3. The hand-usable layer: Option B's combinators (or Option A's spec), with
   help rendering adapted from `print_help`.
4. Language prerequisites for the derive, each justified on its own in §7:
   X1 (`field.default`), X2 (`field.docs`, if C2), X3 (literal members), X5
   (union-arm construction), X6 (the decoder `by` site, shared with SERDE.md),
   X15 (marker aliases), X16 (name clashes), and later X7 (qualifier
   patterns). X4, X8 and X9 are independent and can land whenever they are
   decided.
5. The derive (Option C) over the layer from step 3.

---

## 9. Decisions pending

**AP-1. The overall approach.**
(A) runtime spec with typed handles; (B) applicative combinators; (C) derive
from types; (E) derive on top of combinators. (D, a usage string, is set aside
in §4.4.)
*Recommendation:* **E**, built in §8's order. B alone is a complete, typed
parser with no language work and gives the derive something to expand into; C
on top removes the hand assembly and makes the parser exhaustive over the
struct. Choose A over B for the foundation only if runtime-extensible flag
sets matter more than by-construction exclusivity.

**AP-2. Where a derived field's metadata lives** (short names, help, metavar).
Extensions: X2, X8, X13, X15. §10.1 proposes marker aliases (D-1).
(1) convention only; (2) doc comments as `field.docs`; (3) field attributes;
(4) qualifiers on the field type; (5) an exhaustive companion function.
*Recommendation:* **(2) for help text, plus (5) for short names and
overrides**, so no new syntax and an exhaustiveness guarantee for what is not
derived. Revisit (3) only if (5) proves too noisy in practice.

**AP-3. Build `field.default`.** (X1.) Needed by the derive (to apply and print
defaults) and by SERDE.md's codecs. *Recommendation:* yes, as recorded in
ROADMAP §2c.

**AP-4. How a derived decoder is instantiated.** (X6.) `from_strs<T>` has `T` only in
its return type. (1) `by` stamps at the return type when there is no `T`
parameter; (2) an obligation clause on the struct (`struct ServeOpts :
FromArgs<self> by args`) whose member returns `self`; (3) a one-line
expanded-in-place wrapper per struct. *Recommendation:* decide together with
`from_json`; (2) is the most uniform with `by auto` today.

**AP-5. Comptime construction of a union arm.** (X5.) Needed for exclusive groups of
tagged arms and for subcommands. *Recommendation:* build it; literal-union
fields (`"json" | "text"`) work without it, so it can follow the first derive.

**AP-6. The derive's mapping rules** (§4.3's table). §10 refines
the rules for positionals, sub-commands and groups. Open sub-questions:
literal-union field as one flag per literal (`--json`/`--text`) or as one
valued option (`--output json`); how positionals are marked (a field-name
convention such as `args`/`files`, a wrapper type `Positional<T>`, or
metadata); kebab-case flag names from snake-case fields. *Recommendation:* a
literal-union field is **one valued option** (`--output json`), which keeps
the flag's name the field's name and lists the choices in help; metadata
(AP-2) can ask for switches instead. Positionals by a wrapper type, since a
name convention is invisible where the struct is declared. Kebab-case names,
yes.

**AP-7. Inter-argument validation.** §5.1 by type, §5.2 by validating
constructor, §5.3 by declaration. *Recommendation:* §5.4: by type first,
the validating constructor for the rest, declarations only if help needs them.
Sub-question: whether the derive calls a user `validate` automatically
(resolved as an implicit) so the parse result is already `Valid T`.

**AP-8. Errors.** (X12.) First error or all of them; the shape of `ArgError`; help
and version as outcomes. *Recommendation:* all errors (the user fixes a
command line once, not once per mistake); help and version as `ArgError` arms,
so parsing stays pure.

**AP-9. Exit status.** (X10.) (1) export `exit(code: Int) -> Never` as a member of a
new `Process` effect, a capability like `Args` [cli-args]; (2) let `main`
return an `Int` status; (3) a std `run_cli` helper that owns the exit, with
the mechanism private. *Recommendation:* **(1)**: exiting is something a
function does to the process, which is what effects are for, and it keeps
`main`'s signature (the `main(args)` form was rejected for the same reason).
(3) can sit on top.

**AP-10. Per-type value parsers.** (X11.) One implicit name (`from_str`) defined per
type in each type's own module, or distinct names (`parse_int`,
`parse_double`, …) with the derive mapping types to names. *Recommendation:*
one implicit name, if [implicit-resolve] picks the right module's candidate by
return type (to verify first); otherwise distinct names, chosen in the derive by a chain of
`[if field.type is …]` tests (a stamped fn may not use `[when field.type]`
[comptime-pattern]).

**AP-11. Lexing conventions.** Bundled short flags (`-jt`); attached short
values (`-p8080`); `--name=value` and `--name value`; `--` ends options; a lone
`-` is a positional; negative numbers as values (`--offset -3`) accepted when
the option takes a value; `--no-name` for negating a defaulted `Bool`
(optional); no prefix abbreviation (`--po` for `--port`), since it makes adding
a flag a breaking change. *Recommendation:* all of the above, with `--no-`
deferred.

**AP-12. What happens to `std/args/parse.sv`.** It is the user's work in
progress: a builder-style `ArgGroup` and a comptime help printer. *Options:*
keep it as the help-rendering layer under whichever option is chosen; evolve
it into Option A; or replace it. *Recommendation:* the user's call; its
`print_help` walk is reusable under every option.

---

## 10. Derive design: names, positionals, subcommands and groups

The direction (2026-10-20, user): the derive (Option C), with comptime
extended as needed. This section designs the three parts the user was unsure
of. Each part states the model, the alternatives considered, the rules the
derive enforces at the declaration, and what it needs from §7. The design
calls are marked **D-1** to **D-6** and collected in §10.4.

The whole running example under this design, as one declaration (*sketch*:
needs X1, X2, X3, X5, X6 and X15):

```
import args

// Serve files over HTTP.
struct Serve {
    // Address to bind.
    host: Str = "127.0.0.1"
    // Port to listen on.
    port: Short<"p", InRange(1, 65535) Int> = 8080
    // Output format.
    output: "json" | "text" = "text"
    // TLS. Give both or neither.
    tls: Tls?
    // Files to serve.
    files: Positional<NonEmpty List<Str>>
}

struct Tls {
    // Certificate file.
    cert: Str
    // Private key file.
    key: Str
}
```

### 10.1 Short names and positional arguments

**Model: marker aliases (X15).** std's `args` declares

```
export type Short<C, T> = T          // C is a one-character literal: Short<"p", Int>
export type Positional<T> = T        // not a flag: taken by position
```

and the derive recognizes them at the top of a field's type. Everything else
about the field is unchanged: `opts.port` is an `Int` (with its `InRange`
claim), `opts.files` a `NonEmpty List<Str>`.

**Short names.**
- `port: Short<"p", Int>` accepts `-p 8080`, `-p8080` and `--port 8080`.
  The long name is always the field's name in kebab case; a field cannot opt
  out of its long name, so every flag has one spelling that is grep-able in
  scripts.
- Bundling follows AP-11: `-jt` is two switches; `-vp8080` is a switch and a
  valued short flag.
- **Refused at the declaration**: two fields with the same short name (X16,
  or a run-time check until it lands), a short name on a `Positional`, a
  marker below the top of a type.
- **Not checked at compile time**: that `C` is one character. Comptime has no
  string operations; the generated parser checks it when it is built, so the
  first test of the parser fails. A std `Letter` literal union with
  membership testing in comptime would close it; not worth a feature yet.

**Positionals.**
- `Positional<T>` fields are taken in **field order**. `T` decides arity:
  `Str` exactly one, `Str?` zero or one, `List<Str>` zero or more,
  `NonEmpty List<Str>` one or more, and any `FromStr` type for the element.
- **Refused at the declaration**: a required positional after an optional
  one, a positional after a list positional (only the last may be a list),
  a defaulted `Positional` before a required one.
- Usage and help name a positional by its field name in angle brackets
  (`<files>...`), with its doc comment as help.
- After `--`, every token is positional, so `serve -- -weird-name.txt` works.

**Alternatives considered.**
- *Exhaustive companion function* (Option C5) for short names: works today
  but puts the short name away from its field, and needs a per-struct
  declaration. Kept as a fallback if X15 is refused.
- *Positionals by convention* (a field named `args`, or the last `List<Str>`):
  invisible at the declaration, and a rename silently changes the command
  line. Refused.
- *Positionals as a separate struct* (`struct Cli { opts: Serve, args:
  ServeArgs }`): two declarations where one suffices, and the order rules
  still apply inside the second.
- *Attributes* (X13): the widest change, for the same information.

**Overrides deferred.** A long-name override (`Long<"output-format", T>`), a
metavariable (`Meta<"PATH", T>`) and an environment fallback (`Env<"PORT",
T>`, with X10) are the same mechanism and can be added when wanted. With
several markers on one field the order is fixed and documented (D-2).

### 10.1a The qualifier spelling: `Short('p') Int`

Suggested by the user (2026-10-20): `port: Short('p') InRange(1, 65535) Int =
8080`, a qualifier with a `Char` constant slot instead of a generic marker.
It reads better: the short name sits in the same position as the range,
qualifiers compose in any order ([qual-no-with]), so D-2 disappears, and
`Char` makes "one character" a type error rather than a run-time check.
Probed against today's compiler (2026-10-20):

1. **Constant slots are `Int`-only.** [qual-const] lowers a constant to
   `Ty::ConstInt`; `Short('p')` is a parse error ("expected identifier,
   found character literal"). `Char` (and `Str`) constants are an extension,
   with their own uses (`Prefix("arn:") Str`, `Unit("ms") Long`).
2. **A qualifier is a claim the value must carry.** With `qualifier
   Short(c: Int) of Int` whose `qualifies` always answers `true`, the struct
   declares fine, but `Opts { port: 3 }` is refused: "field `port` expects
   `Short(112) Int`, found `Int`". Every literal of the options struct (a
   test fixture, options built from a config file) would have to establish
   the claim with `is Short('p')` first. The marker alias imposes nothing,
   because it is not a claim.
3. **Comptime cannot read it.** Type patterns erase qualifiers, so the derive
   could not find `Short` or its argument without X7.
4. **It flows into values.** Reading `opts.port` gives a `Short(112)
   InRange(1, 65535) Int`, carrying "came from `-p`" into every function the
   port reaches. The metadata belongs to the field declaration, not to the
   value.
5. **It does not extend to the other markers.** A qualifier over a union is
   "the claim `Sub` *about* a union", and `when` refuses it until the union
   is bound to a local (probed with `command: Sub GitCommand`). So
   `Subcommand` would stay a marker, or wait on qualifiers over unions
   (ROADMAP D4).

(Probing this also found that a struct field's **default is not checked
against a qualified field type**: `port: InRange(1, 10) Int = 8080` compiles,
and `needs(o.port)` with `needs(p: InRange(1, 10) Int)` prints 8080 on both
backends. Recorded in ROADMAP §2d, to be fixed generally alongside this work.)

**What would make it work: declared annotations.** Points 2 and 4 say the
spelling needs a qualifier that *claims nothing*: a value fits it without
being established, and reading the field drops it. That is a new declaration
kind rather than a qualifier, for example:

```
export annotation Short(c: Char) of *       // sketch: spelling open
export annotation Positional of *
```

- **Semantics.** Written in qualifier position on a struct field's type (and
  nowhere else); ignored by type checking, so `Opts { port: 3 }` and `= 8080`
  fit; dropped when the field is read; visible to comptime as
  `field.annotations` with typed arguments (`[if field has <c> Short(c)]`,
  spelling open).
- **Why it is not attributes (X13) again.** SERDE.md's objection to
  attributes is an open-ended, unchecked annotation language. Here each
  annotation is a **declaration** with **typed slots**: `Short("pp")` is a
  type error, an unknown annotation is an unresolved name, hover and
  go-to-definition work, and an annotation is imported like anything else.
  That is attributes with a declared vocabulary, in a position Salvo already
  has.
- **Elsewhere.** Every per-field metadata use X15 lists: `Key("createdAt")
  Long` for codec renames, `Unit("ms") Long`, `Format("email") Str`, and
  `Doc(…)` if doc comments (X2) are not enough. A redaction marker (`Secret`)
  is *not* one: whether a value may be printed is a fact about the value, so
  it stays a provenance qualifier.
- **Cost.** A declaration kind, `Char`/`Str` constant slots, a rule for where
  an annotation may be written, and comptime access to annotations (the
  annotation counterpart of X7, simpler because annotations never reach the
  checker's subtyping). Larger than X15, smaller than open attributes.
- **Sub-commands.** `Subcommand` as an annotation has no union problem
  (point 5), since an annotation is not a claim about the value: `command:
  Subcommand GitCommand`. So annotations would replace all three markers.

**Comparison.**

| | Marker alias `Short<"p", Int>` | Annotation `Short('p') Int` |
|---|---|---|
| Works today | yes, apart from reading the literal (X15) | no: needs a declaration kind, `Char` slots, comptime access |
| One-character check | run time (first test) | type |
| Several on one field | nesting order matters (D-2) | any order |
| Reads like | a wrapper type | the qualifiers next to it |
| What a read of the field gives | the value's type | the value's type (annotation dropped) |
| Covers `Positional`, `Subcommand` | yes | yes |
| General mechanism | a convention over aliases | a declared, typed vocabulary |

### 10.1b Field qualifiers: provenance on `FieldOf<T>`

Proposed by the user (2026-10-20), following from 10.1a's conclusion that
the short name describes the field, not the value. A field's *name* is
already a value in this design: X8's `FieldOf<T>` is the literal union of
`T`'s field names. So an annotation can be an ordinary qualifier whose subject
is that name:

```
export provenance qualifier Short<T>(c: Char) of FieldOf<T>

struct Serve {
    Short('p') port: InRange(1, 65535) Int = 8080
    host: Str = "127.0.0.1"
}
```

- **Written before the field's name**, as a qualifier is written before the
  type it qualifies: `Short('p') port` qualifies the name, `InRange(1, 65535)
  Int` the value. Several stack freely, as provenance qualifiers already do
  ([qual-no-with]): `Short('p') Env("PORT") port: …`.
- **The struct declaration is the mint.** Provenance claims are mint-only;
  here the declaration mints the claim on its own field name, and nothing
  else can. "Provenance" is the right subject: whether `"port"` carries
  `Short('p')` is not a fact about the string, which another struct may use
  without it, but about where the name came from.
- **`FieldOf<Serve>`'s arms carry it**: `Short('p') "port" | "host"`.
- **Checked by the qualifier's subject** [qual-of]: `Short` applies to field
  names and nowhere else, without a new declaration kind. A qualifier
  declared `of FieldOf<Serve>` would apply to that struct's fields only.
- **Erased** like every qualifier; **comptime** reads it on `field.name`
  (`[if field.name is Short]`, and its constant with a binder, spelling
  open), which is all the derive needs.
- **Fields read unchanged**: `serve.port` is an `InRange(1, 65535) Int`.

Against declared annotations (10.1a): the declaration-site syntax and the
comptime access are the same, so the derive does not care which is chosen.
This form needs no new declaration kind and gives the metadata a runtime
presence; it depends on X8 and on qualified literal arms, which annotations
do not.

**Would `field is Short` at run time be useful?** Sometimes, and mostly
through the constant rather than the test:
- **Where a field name is a runtime value.** An `ArgError { field:
  FieldOf<Serve> }` rendered as "missing `-p`/`--port`"; help written as
  ordinary code over the field names (with X3's `all_values`) instead of a
  comptime walk; SERDE.md's change set keyed by `FieldOf<PersonItem>`, whose
  writer needs each field's `Key("createdAt")`. Each wants the *value* of the
  constant (`'p'`, `"createdAt"`), so the useful form is a binding test such
  as `if field is Short(c)` with `c: Char`, or an accessor. Since the arms are
  literals, the compiler lowers either to a comparison against the literals
  that carry the qualifier (or a table lookup), so nothing is stored at run
  time.
- **Not in generic code.** Which literals carry `Short` depends on `T`, so in
  `fn render<T>(e: ArgError<T>)` the test cannot be compiled without
  something per `T` (an implicit `?short_of: (FieldOf<T>) -> Char?` the
  derive stamps). The renderers that would use it are usually generic, so
  the test helps less than it first looks: concrete code and comptime cover
  most of it.
- **Not for the derive.** Comptime knows every field's qualifiers statically.

**User call (2026-10-20):** the runtime annotation of `FieldOf<T>`'s arms
is not needed; the defects these probes found are fixed generally anyway
(ROADMAP §2d).

So the runtime visibility is a real but modest bonus. It is free once the
qualifier is on the arm, since `is` on a qualified arm is existing
machinery; the binding form for constants is the one new piece, and it can
wait for a use.

**What exists today** (probed 2026-10-20):
- A qualifier cannot be written directly on a literal (`Short "port"` is a
  parse error); through aliases (`type Port = "port"`, `Short Port`) the
  union `Short Port | Short Verbose | "host"` is accepted by the checker.
- **It does not compile**: both backends emit a two-arm union and refer to a
  third arm (`Union2::U3`), so rustc and kotlinc fail. The checker and the
  emitters disagree on the arm identity of qualified literals
  [union-arm-identity]. Recorded in ROADMAP §2d, to be fixed generally alongside this work.
- A bare `"port"` does not fit `Short "port"` ("not one of the values …
  lists"): provenance is mint-only, and [arm-infer] does not cover literal
  arms. Here only the declaration mints, so this is the wanted behaviour for
  `FieldOf`; user code obtains field names from `FieldOf` values and from
  comptime, not from bare literals. (Whether `let f: FieldOf<Serve> =
  "port"` should be accepted, by inferring the one arm it can be, is a
  sub-question.)

**A side effect worth deciding: nominal field names.** `FieldOf<A>` and
`FieldOf<B>` for two structs with the same field names are the same
structural literal union, so one struct's field name passes where the
other's is expected. If every arm carried an implicit provenance tag naming
its struct (the same mechanism, applied by the compiler), `FieldOf<A>` would
be nominal and X8's "a field name the struct does not have is a compile
error" would hold across structs too. Not needed for parsing.

**Needs:** X8 (`FieldOf<T>`); `Char` and `Str` constant slots; qualifiers on
literal arms (syntax, and the emission defect above fixed); a field
declaration that starts with qualifiers; comptime access to `field.name`'s
qualifiers and constants.

### 10.1c Declaring a field qualifier: the spelling

With the runtime part dropped (§10.1b, user call), a field qualifier is
compile-time only: written before a field's name, erased, read by comptime.
The question is how its **declaration** is spelled. The use site is the same
in every option below: `Short('p') port: InRange(1, 65535) Int = 8080`.

**(A) `provenance qualifier Short<T>(c: Char) of FieldOf<T>`** (§10.1b).
Three concepts at once: `provenance`, which describes a value's origin when
nothing is a value any more; a type parameter `<T>` whose only job is "any
struct"; and `FieldOf<T>` (X8), which nothing else in the plan needs yet. It
is the only option that keeps the runtime reading open, which is no longer
wanted.

**(B) `qualifier Short(c: Char) of Field`**, where `Field` is
`core.comptime`'s own `Field`, the type a `[for field in T.fields]` binder
already has. The qualifier's subject is exactly what it describes, as
comptime already models it:

```
// std/args
export qualifier Short(c: Char) of Field
export qualifier Positional of Field
export qualifier Subcommand of Field

// a user's struct
struct Serve {
    Short('p') port: InRange(1, 65535) Int = 8080
    Positional files: NonEmpty List<Str>
}

// the derive
[for field in T.fields] {
    [if field is <c> Short(c)] { … c … }       // c: Char, spelling open
    [if field is Positional] { … }
}
```

- **Erased by construction**: `Field` is compile-time only
  [comptime-fields], so a qualifier on it can never reach a value or a
  backend, and there is nothing to lower.
- **Reads in comptime as qualifiers read anywhere**: `field is Short`, the
  same `is` as `n is InRange(1, 65535)`, on the binder that stands for the
  field. No new projection such as `field.annotations`.
- **Extends to the rest of the model with no new idea**: `of Struct` for a
  struct-level fact (`Named("rm") struct Remove { … }`, a command's name),
  `of Arm` for one arm of a union, `of Union` for a whole union.
- **Rules it needs**: a qualifier `of Field` (or `Arm`, `Struct`, `Union`)
  has no body (a `qualifies` would have to run at compile time) and is
  minted only by writing it on the declaration; `of` accepts the
  `core.comptime` types, which is the one new position where they may be
  named; the constant slot takes `Char`/`Str` (X17's slot types).
- **No `<T>`**, since `Field` already ranges over every struct's fields. A
  qualifier meant for one struct's fields only has no spelling here, and no
  use has been found for one.

**(C) `annotation Short(c: Char)`**, a declaration kind of its own (§10.1a).
The plainest to read, and it says outright that this is not a claim about a
value. But it is a new declaration kind beside `qualifier`, it needs its own
"applies to fields / structs / arms" rule (which (B) gets from `of`), and
comptime needs a new projection to read it.

**(D) `qualifier Short(c: Char) of field`**, with a keyword subject rather than
a type. (B) without the link to the comptime model: the same rules, but
`field` names nothing a reader can hover or go to, and `struct` and `arm`
would need keywords of their own.

**(E) Annotations as comptime structs**, the Java, C# and Kotlin model, where
an annotation is a class:

```
export comptime struct Option {
    short: Char? = None
    env: Str? = None
    value_name: Str? = None
}

struct Serve {
    Option { short: 'p', env: "PORT" } port: Int = 8080
}
// the derive: [if field has Option] { … field.meta(Option).short … }   (sketch)
```

Its strength is several related properties in one annotation, with names
and defaults, where clap writes `#[arg(short, long, env = "PORT")]`; (B)
writes `Short('p') Env("PORT") port`. Its cost is a value written at the
declaration and read by comptime, which is the evaluation of ordinary
expressions at compile time that comptime has never done; and `comptime
struct` today means "a description the compiler fills", not "something a
user writes".

| | (A) provenance of `FieldOf<T>` | **(B) qualifier of `Field`** | (C) `annotation` | (D) of `field` | (E) comptime struct |
|---|---|---|---|---|---|
| New declaration kind | no | **no** | yes | no | no (new use of one) |
| Depends on | X8, qualified literals | **`Char`/`Str` slots** | `Char`/`Str` slots | `Char`/`Str` slots | compile-time values |
| Comptime reads it as | `field.name is Short` | **`field is Short`** | a new projection | `field is Short` | a new projection |
| Covers structs and arms | no | **`of Struct`, `of Arm`** | needs a rule | needs keywords | needs a rule |
| Several properties at once | stack qualifiers | **stack qualifiers** | stack | stack | one value, named fields |
| Runtime reading possible | yes | no | no | no | no |

**Recommendation: (B).** It is the shortest declaration, it needs no new
declaration kind and no `FieldOf<T>`, and it reuses the comptime model's own
`Field` and the `is` test comptime code already writes. If several
properties per field turn out to be common (clap's `short` + `long` + `env` +
`value_name`), (E) is the one to revisit; stacked qualifiers cover that case
until then.

### 10.2 Sub-commands

**Model: a command is a struct; a sub-command slot is a field whose type is
a union of command structs, marked `Subcommand<U>`.**

```
// Manage a repository.
struct Git {
    // Print more.
    verbose: Short<"v", Bool> = false
    command: Subcommand<GitCommand>
}

type GitCommand = Clone | Remote

// Copy a repository.
struct Clone {
    url: Positional<Str>
    depth: Int?
}

// Manage remotes.
struct Remote {
    command: Subcommand<RemoteCommand>
}

namespace Remote {
    // Add a remote.
    struct Add { name: Positional<Str>, url: Positional<Str> }
    // Remove a remote.
    struct Remove { name: Positional<Str> }
}
type RemoteCommand = Remote.Add | Remote.Remove
```

`git -v clone URL --depth 1` parses to `Git { verbose: true, command: Clone {
url: "URL", depth: 1 } }`, and the program dispatches with an ordinary,
exhaustive `when`, so adding a command breaks every dispatch that does not
handle it:

```
when command = git.command {
    is Clone { clone(command) }
    is Remote { remote(command) }
}
```

**Names.** A command's name is the last segment of its arm's name, in kebab
case: `Clone` → `clone`, `Remote.Add` → `add` (`arm.name` is `"Remote.Add"`
for a struct in `namespace Remote`, checked 2026-10-20). Namespaces are how a
nested command set avoids a `RemoteAdd`-style prefix without the derive
guessing one (D-4). The struct's doc comment (X2) is the command's
description in help.

**Parsing rules.**
- The parent's options come **before** the sub-command's name; after it, only
  the sub-command's options and positionals. This is git's rule and keeps
  `-v` meaning one thing per position. (Alternative: clap's `global`
  options, accepted anywhere below; deferrable.)
- `Subcommand<GitCommand>` is required; `Subcommand<GitCommand?>` allows no
  command (the program then prints help or does a default action).
- Nesting is a struct arm with its own `Subcommand` field, to any depth.

**Refused at the declaration.**
- More than one `Subcommand` field in a struct.
- A `Subcommand` field beside `Positional` fields: whether a token is a
  command or a positional would depend on its spelling. (clap allows it with
  precedence rules; the restriction can be lifted later.)
- An arm that is not a struct (a `Str` arm has no options and no name).
- Two arms with the same command name after kebab-casing.

**Why a marker, when a union of structs could be recognized by shape.** The
same shape also models an exclusive group of options (§10.3,
`source: FromUrl | FromFile`). The two parse differently: a sub-command is
chosen by a positional word and takes the rest of the line; an exclusive group
is chosen by which of its flags appear, anywhere among the parent's options.
Telling them apart by shape would make one of them unrepresentable, so one of
them is marked, and the sub-command is the one with the visible syntax.

**Alternatives considered.**
- *The top-level type is the union* (`type Git = Clone | Remote`), clap's
  `Subcommand` enum. Leaves no place for options before the command (`git
  -v clone`), so a wrapper struct is needed anyway; the field form covers both.
- *Commands as functions* (`fn clone(url: Str, depth: Int?)` with dispatch
  derived from a module, Python's Fire or Nim's cligen). Attractive for small
  tools, but the parsed value never exists as data, so it cannot be tested or
  passed on, and comptime has no walk over a module's functions.

**Needs.** X5 (building the chosen arm) and X6 (the decoder's `by` site), X2
for descriptions.

### 10.3 Groups of options

Four kinds, each a type the derive already sees; no marker is needed for any.

| Kind | Declared as | Command line |
|---|---|---|
| Flattened group (reuse, help section) | a struct-typed field `net: NetOpts` | the struct's flags, at the parent's level |
| Co-occurring group | an optional struct `tls: Tls?` | all of its required flags, or none of them |
| Exclusive choice of one value | a literal union `output: "json" \| "text"` | `--output json` (AP-6) |
| Exclusive group of options | a union of structs `source: FromUrl \| FromFile` | the flags of exactly one arm |

**Flattened groups.** `net: NetOpts` contributes `NetOpts`'s fields as the
parent's own flags (`--host`, `--port`), so a set of options is declared once
and reused across commands. The flags are **not prefixed** (clap's `flatten`),
and a clash with a parent flag is refused at the declaration (X16). The
field's doc comment, or the struct's, becomes a **section heading in help**
("Network options:"), which is what `std/args/parse.sv`'s `groups` map is for
today. (Alternative: a `Prefixed<"net", NetOpts>` marker giving `--net-host`;
deferrable, D-5.)

**Co-occurring groups.** `tls: Tls?` is `None` when none of `Tls`'s flags
appear, and otherwise every required field of `Tls` must be given ("`--cert`
requires `--key`"). Optional fields inside give the one-directional
*requires*: `struct Tls { cert: Str, key: Str?, ca: Str? }` lets `--key` and
`--ca` appear only with `--cert`. This is Clikt's `cooccurring()` with the
type doing the work.

**Exclusive choice of one value.** A literal union: one flag, its value one of
the literals (needs X3 to list them in help and in the error). With a default,
the flag is optional; with `?`, it may be absent; with neither, required.

**Exclusive groups of options.** A union of structs that is *not* marked
`Subcommand`:

```
struct FromUrl { url: Str, timeout: Int = 30 }   // --url URL [--timeout N]
struct FromFile { file: Str }                    // --file PATH
struct FromStdin {}                              // --from-stdin

struct Fetch {
    source: FromUrl | FromFile | FromStdin
}
```

- The arm is chosen by **which flags appear**: `--url` selects `FromUrl`,
  after which `--timeout` is allowed and `--file` is an error ("`--file`
  cannot be used with `--url`"). No selector flag is needed.
- An empty arm is chosen by a switch named after it (`--from-stdin`).
- `source: … | None` (or `?`) makes the whole group optional; without it, one
  arm is required ("one of `--url`, `--file`, `--from-stdin` is required").
- **Refused at the declaration**: two arms sharing a flag name (the flag
  would not say which arm it selects), and an arm whose flags are all
  optional and that is not empty (nothing would select it).
- For single-value arms, provenance-tagged scalars work too:
  `source: FromUrl Str | FromFile Str` gives `--from-url VALUE` and
  `--from-file VALUE`, the arm named by its qualifier.

Clikt's `groupChoice` (a valued selector, `--source url --url …`) is the
alternative selection rule; selection by presence is simpler to write and to
read, and the selector form can be added later as a marker if a tool needs it
(D-6).

**Nesting.** Groups compose: a co-occurring group may contain a flattened
group, an exclusive arm may contain a co-occurring group. The derive walks the
types recursively through struct fields, so the rules above apply at every
level, and every rule is checked where the struct is declared.

**Needs.** X1 (defaults), X3 (literal members), X5 (building a union arm),
X16 (flag-name clashes).

### 10.4 Design calls

**D-1. How per-field metadata is spelled.** (1) Marker aliases
(`Short<"p", T>`, `Positional<T>`, `Subcommand<U>`; X15). (2) Declared
annotations in qualifier position (`Short('p') T`; §10.1a). (2b) Provenance
qualifiers on the field's name, `Short('p') port: T` with `provenance
qualifier Short<T>(c: Char) of FieldOf<T>` (§10.1b). (3) A plain
qualifier with a constant slot, as suggested: not workable as it stands,
since a qualifier is a claim the field's values must carry (§10.1a, points
2 and 4). (4) The exhaustive companion function (C5). (5) Open attributes
(X13). Doc comments (X2) carry help text in every option.
*Recommendation:* **field qualifiers written before the field's name,
without the runtime part** (user, 2026-10-20: the `FieldOf<T>` arms need not
carry the qualifiers at run time), **declared `qualifier Short(c: Char) of
Field`** against `core.comptime`'s `Field` (§10.1c, option B): erased by
construction, read in comptime as `field is Short`, and extended to `of
Struct` and `of Arm` with no new idea. **(1) as the interim**, since it works
in today's compiler apart from X15. The derive is built the same in every case: only the comptime test that
finds the metadata differs, so the derive can start before this is settled.

**D-2. Marker order when several apply.** Moot under D-1 (2). Under (1): a
fixed outer-to-inner order (`Short<"p", Env<"PORT", Int>>`) or order-free (the
derive peels any marker in any order). *Recommendation:* order-free if the
derive can peel with a fixed number of nested `[if]`s per marker; otherwise a
documented order. Only `Short` exists at first, so this can wait.

**D-3. Where parent options may appear.** Before the sub-command only (git),
or anywhere for flags marked global (clap). *Recommendation:* before only;
global flags later if wanted.

**D-4. Sub-command names.** The last segment of the arm's name in kebab case
(`Remote.Add` → `add`, with namespaces for nested sets), the full name
(`remote-add`), or dropping a prefix shared with the parent (`RemoteAdd`
under `Remote` → `add`). *Recommendation:* last segment: it is predictable,
uses an existing feature, and needs no guessing; a `Named<"rm", …>` marker on
the arm for a name the struct cannot have.

**D-5. Flattened groups.** Unprefixed with clashes refused, or prefixed.
*Recommendation:* unprefixed; a `Prefixed` marker later.

**D-6. Selecting an exclusive group's arm.** By which flags appear, or by a
valued selector. *Recommendation:* by presence, with the declaration rules
above; a selector marker later.

---

## 11. Decisions needed before building (round 2)

In build order: a phase can start once the questions above it are answered.
Each has a recommendation; "accept the recommendations, except …" is a
complete answer.

Also found while preparing this list (probed 2026-10-20, both backends):
building a struct field by field already works in an expanded-in-place
comptime fn, with the per-field value parser found as an implicit:

```
fn from_tokens(t: Tokens, into: Int?) [] -> Int => t, into { … }
fn take<E>(t: Tokens, ?from_tokens: (t: Tokens, into: E?) -> E) [] -> E => t { … }
comptime fn serve_from(t: Tokens) [] -> Serve => t {
    return Serve { [for field in Serve.fields] { [field]: take<[field.type]>(t) } }
}
```

So X11 has a working shape today (a witness parameter, Q4), and the derive's
core loop needs no new mechanism; what is missing is around it.

### Phase 0: fixes and std basics

**Q0. `std/args/parse.sv`.** It still uses the retired `field: canbe Mut T`
spelling, so the embedded std does not parse and nothing in the main tree
compiles. (a) The user updates it; (b) the agent updates the six fields and
leaves the rest; (c) it is replaced by the derive (AP-12).
*Recommendation:* (b) now, so the tree builds; its fate (c) or not is AP-12.

**Q1. Qualified literals in unions** (ROADMAP §2d item 2). (a) Support:
they collapse into the base arm like every literal, `is Q` compares against
the literals carrying `Q`, and a union listing one literal both with and
without `Q` is refused. (b) Refuse them with a diagnostic.
*Recommendation:* (a).

**Q2. Qualified defaults** (§2d item 1). Fixing the unchecked default makes
`port: InRange(1, 65535) Int = 8080` an error, since a plain `8080` does not
carry the claim, and that is the derive's running example. (a) Refuse until
compile-time literal establishment exists (a separate, larger feature). (b)
Check it at run time: a default of a predicate-qualified type is tested with
`qualifies` where it is applied, and a failure traps, as `x!` does. (c) Both:
(b) now, replaced by compile-time establishment when it lands.
*Recommendation:* (c). The claim is then always backed by a check, and the
example compiles.

**Q3. `Process` and `Env` (X10).** `effect Process { fn exit(code: Int) ->
Never }` with `HostProcess()`, and `effect Env { fn env(name: Str) -> Str? }`
with `HostEnv()`, each in its own module as `args` is. Sub-questions: (a)
`exit` flushes the host's stdout and stderr first; (b) linear values live at
an `exit` are treated as at a trap (obligations are not run), stated in the
spec. *Recommendation:* both as written.

**Q4. Value parsers (X11).** (a) A **witness parameter**: one `from_str(s:
Str, into: E?) -> E?` per type, in each type's module, found as an implicit
(works today, above), with a user-facing `parse<E>(s: Str) -> E?` on top;
(b) a language change letting overloads differ by return type only, so
`from_str(s: Str) -> Int?` and `-> Double?` coexist; (c) distinct names
(`parse_int`, `parse_double`) mapped by the derive. Sub-question: which
strings a `Bool` accepts. *Recommendation:* (a), and `true`/`false` only.

### Phase 1: language extensions

**Q5. Comptime qualifiers, the rules** (D-1 decided the spelling).
- (a) *Subjects now*: `Field` and `Struct`; `Arm` and `Union` when a use
  appears. *Recommendation:* as written.
- (b) *Placement*: before a field's name; before a whole declaration for
  `Struct`, ahead of its modifiers (`Named("rm") export struct Remove`), as
  Kotlin places annotations. *Recommendation:* as written.
- (c) *Constant slots*: `Char` and `Str`, for ordinary qualifiers too
  (`Prefix("arn:") Str`). *Recommendation:* both kinds.
- (d) *Reading a constant in comptime*: a binder declared before the
  pattern, as type patterns do: `[if field is <c> Short(c)]`, with `c` usable
  as a value in the branch; and the same form for qualifiers on a field's
  type (X7, needed to re-test `InRange`): `[if field.type is <lo, hi>
  InRange(lo, hi) *]`. *Recommendation:* one syntax for both.
- (e) No body (`qualifies`) on a comptime qualifier, and minted only by
  being written. *Recommendation:* as written.

**Q6. Building a union arm in comptime (X5).** Inside `[for arm in
U.arms]`: `return [arm.type] { [for f in arm.type.fields] { [f]: … } }` for a
struct arm; the arm's constructor for a qualified arm; `None` for `None`.
*Recommendation:* as written.

**Q7. How a struct gets its parser (X6).** (a) An obligation clause, `struct
Serve : FromArgs<self> by args`, stamping at `self`; (b) `by` stamps at the
return type when the fn has no `T` parameter; (c) only expanded-in-place
comptime fns (works today, but each struct writes its own). This also
settles `from_json` (SERDE.md). *Recommendation:* (a), matching `:
Ordered<self> by auto`.

**Q8. Literal members in comptime (X3).** `[for lit in field.type.literals]`
with `lit.value` (a literal of the union), `lit.index`, `lit.first`,
`lit.last`. *Recommendation:* as written.

**Q9. `is` with several literals or types in ordinary code (X4).** `s is
"json" | "text"` narrows to the literal union; and `x is Int | Str` on a
wider union narrows to the sub-union. *Recommendation:* both.

**Q10. Why a predicate failed (X9).** An optional member `fn explain(x, …) ->
Str` beside `qualifies`, reached by `assert!`/`expect` failure messages and
the derive, and by user code as `InRange.explain(n, 1, 65535)`.
*Recommendation:* as written; the user-facing call spelling is the part to
confirm.

**Q11. An accumulating outcome (X12).** `Ok T | Errs NonEmpty List<E>` in
`core.result`, with `both`/`map2` collecting. *Recommendation:* as written;
the arm's name (`Errs`) is the part to confirm.

*No decision needed* (engineering under the calls above): X1
(`field.default`, evaluated in the declaring file's scope), X2 (`field.docs`,
text as hover prints it), X16 (`[if a.name == b.name]`).

### Phase 2: `std/args`

**Q12. The runtime layer under the derive.** (a) Public combinators (Option
E), usable by hand; (b) an internal layer only the derive uses (Option C).
*Recommendation:* (b) first, kept clean enough to publish later; the derive is
what the user asked for, and a public combinator API is a second surface to
design.

**Q13. Mapping rules** (AP-6). A literal-union field is one valued option
(`--output json`); `Bool` is a switch with no `--no-x` form yet; positionals
and sub-commands are the comptime qualifiers `Positional` and `Subcommand`;
flags are kebab-cased field names. *Recommendation:* as written.

**Q14. Sub-commands** (D-3, D-4). Parent options come before the command
word; a command's name is the last segment of its namespaced struct name,
kebab-cased (`Remote.Add` → `add`), with `Named("rm")` (a comptime qualifier
`of Struct`) to rename. *Recommendation:* as written.

**Q15. Groups** (D-5, D-6). Flattened groups unprefixed, a clash refused at
the declaration; an exclusive group's arm chosen by which flags appear.
*Recommendation:* as written.

**Q16. Validation hook** (AP-7). The derive calls a user `validate(o: T)
[Throw<ArgError>] -> +Valid T` when one is declared (found as an implicit),
so the parse result is the validated type. *Recommendation:* yes.

**Q17. Errors, help and version** (AP-8). All errors reported in one run;
help and version are outcomes, not side effects. Sub-question: where
`--version`'s text comes from: (a) the manifest's `version`, made available
to the program; (b) a struct-level comptime qualifier, `Version("1.2.0")`;
(c) no `--version` until asked for. *Recommendation:* all errors, outcomes,
and (c) for version.

**Q18. Lexing conventions** (AP-11). As listed there: bundling, attached
values, `=`, `--`, lone `-`, negative numbers as values, no abbreviations.
*Recommendation:* as listed.

**Q19. `std/args/parse.sv` and the module layout** (AP-12). Keep its
`print_help` walk as the help renderer, or replace it; and whether the
derive, its qualifiers and the `Args` effect share one `args` module.
*Recommendation:* replace the builder types with the derive, adapt
`print_help`'s layout, and keep one `args` module with an internal lexer
submodule. This is the user's file, so the call is theirs.

## 12. Overloading by return type

Decided in round 2. Today [fn-overload] says nothing but the parameters takes
part in selection, and [fn-overload-duplicate] makes two declarations with
the same parameter types a duplicate "since nothing at a call site could
tell them apart". The precedent is [fn-value-select]: a function passed by
name is chosen by the expected fn type, parameters and **result** alike. The
extension gives calls the same information.

**The rule (proposed):**
1. **Declarations.** Same name, same parameter types, different return types:
   an overload set. Same parameter types and the same return type: still a
   duplicate.
2. **Selection.** The three steps are unchanged (`@module`, scope, signature
   by parameters). The return type decides only **among candidates whose
   parameter types are identical**, which is exactly the case parameters
   cannot decide: the expected type at the call picks the candidate whose
   return type fits it (covariantly, as [fn-value-select] does).
3. **Where the expected type comes from.** The places [call-type-args]
   already lists: a `let` annotation, the enclosing fn's return type at a
   `return`, a parameter the result flows into, plus a struct literal's
   field type, an assignment to a declared variable, and an implicit's slot
   type (which [implicit-resolve] step 3 already matches in full). Through
   `!` (an expected `T` asks for `T?`) and `?:` (the whole expression's
   expected type).
4. **No expected type, or none that decides:** an ambiguity error naming the
   candidates and the three remedies (annotate, `f@module`, `rename`), like
   every other unresolved overload [fn-overload-ambiguous]. Never a pick.
5. **Emitted names.** Neither backend can overload by return type (Rust has
   no overloading; Kotlin refuses two signatures differing only in result),
   so [fn-emit-name] gains a sixth level: the return type, appended to the
   suffix (`from_str__Str_IntOpt`, `from_str__Str_DoubleOpt`). Nothing else in the
   backends changes; they already call the overload the checker recorded.

**Example** (std, after the change):

```
// core.basic
export fn from_str(s: Str) [] -> Int? => s { return parse_int(s) }
export fn from_str(s: Str) [] -> Double? => s { return parse_double(s) }
export fn from_str(s: Str) [] -> Bool? => s { … }   // "true" / "false" only

let port: Int? = from_str("8080")          // the annotation picks
let ratio: Double = from_str(text) ?: 1.0  // ?: passes Double? down
let n = from_str(text)                      // error: ambiguous; annotate it
```

and the derive's per-field call is `[field]: take<[field.type]>(t)` with `take`
holding `?from_str: (s: Str) -> E?`, resolved per field type as today.

**What it simplifies elsewhere.** X11's group becomes the obvious `params
FromStr<T> { fn from_str(s: Str) -> T? }`, the dual of `ToStr<T>`; Q4's
witness parameter and `parse<E>` wrapper are not needed; a decoder for any
type is spelled the same way (`from_json(j: Json) -> Person?` beside `->
Order?`), which also bears on X6.

**Details for the user (R-1 to R-3):**

**R-1. Does the return type decide only among identical parameter lists, or
filter every candidate?** (a) Only among identical parameter lists (rule 2):
a call that parameters decide today is decided the same way after the
change, whatever its expected type. (b) Filter first, as [fn-value-select]
does for fn values: the expected type removes candidates whose result does
not fit, then parameters rank the rest, so `let n: Int = f(3)` with `f(x:
Int) -> Str` and `f(x: Int?) -> Int` picks the second. *Recommendation:* (a).
(b) is more uniform with fn values, but an annotation could then change which
overload a call reaches, which is the kind of action at a distance the
2026-09-26 scope decision removed.

*Example.* Two overloads whose parameters differ:

```
fn f(x: Int) [] -> Str => x { … }
fn f(x: Int?) [] -> Int => x { … }

let a = f(3)            // (a) and (b): f(Int), the more specific parameter
let b: Str = f(3)       // (a) and (b): f(Int)
let c: Int = f(3)       // (a): f(Int), then "expected `Int`, found `Str`"
                        // (b): f(Int?): the annotation removed f(Int) first
let d: Int | Str = f(3) // (a) and (b): f(Int), since both results fit
```

Under (b) the same call `f(3)` reaches two different functions depending on
the annotation beside it, and editing `c`'s annotation from `Int` to `Int |
Str` (as `d`) silently moves it from `f(Int?)` to `f(Int)`. Under (a) a call
whose parameters already decide is never affected by its expected type, and
the expected type only ever chooses between declarations like `from_str(s:
Str) -> Int?` and `from_str(s: Str) -> Double?`, whose parameter lists are
identical so nothing else could choose.

**Decided (round 4): (a).**

**R-2. Return types related by subtyping** (`-> Int` beside `-> Int?`, or
`-> Int?` beside `-> Str?` at an expected `Int | Str | None`). (a) Allowed;
when several fit, the most specific return type wins, else ambiguous. (b)
Refused at the declaration when one return type is a subtype of another; an
expected type that fits several unrelated ones is still ambiguous.
*Recommendation:* (b): the sets std needs (`Int?`, `Double?`, `Bool?`) are
unrelated, and a pair like `Int` / `Int?` reads as a mistake.
**Decided (round 3): (b).**

**R-3. Does the expected type travel further** than rule 3 (into an `if`
expression's branches, a `when`'s arms, a lambda body's result)? *Recommendation:*
the branches and arms of a value `if`/`when` yes, since each has the whole
expression's expected type already; lambda bodies only as far as
[call-generic-progressive] already types them.
**Decided (round 3): as recommended.**

## 13. Comptime output used as a type: bracketed or bare?

Raised by the user (2026-10-20): in `return T { … }` and in X5's `return
arm.type { … }`, a type that exists only after expansion sits in the code
bare, while comptime output elsewhere is bracketed (`v.[field]`,
`[field]: …`). The question is whether such a type should be written
`[T]` / `[arm.type]`.

**Today's rule** ([comptime-inline], user decision 2026-09-29): a bracket
appears exactly where the same text without it would be runtime Salvo with
another meaning: `[for …]` against a runtime loop, `v.[field]` against a field
literally called `field`. Where no runtime reading competes, nothing is
bracketed (`field.name`, `T.fields`, `is Struct`). By that rule a type stays
bare: `type` is a keyword, so `field.type` can never be a namespaced type
path, and `T` is a type parameter in either reading. So today's rule is about
**ambiguity**, and the question is whether it should also be about
**visibility**: marking where expansion puts something into the code.

**Where comptime output meets ordinary code**, as the cases to decide:

| | Example | Today |
|---|---|---|
| 1 | a type parameter as a type: `a: T`, `let x: T`, `Mut List<T>()` | bare |
| 2 | a type parameter as a struct literal's head: `return T { … }` | bare |
| 3 | a pattern binder as a type: `let first: E? = None` | bare |
| 4 | a projection as a type: `let x: field.type`, `take<field.type>(t)` | bare |
| 5 | a projection as a struct literal's head: `return arm.type { … }` (X5) | not built |
| 6 | a binder as a field name: `v.[field]`, `[field]: …` | bracketed |
| 7 | a projection as a value: `field.name`, `"${T.name}"` | bare |
| 8 | a constant binder as a value: `c` from `[if field is <c> Short(c)]` | not built |

**(A) Keep today's rule.** Brackets only for ambiguity: cases 6 only.
Least to write; every type reads as a type. A reader cannot tell from
`arm.type { … }` that the head is filled in per arm, and a projection used as
a type (`field.type`) looks like an expression in a type position.

**(B) Bracket what expansion computes into syntax; leave declared names
bare.** A bracket marks where a comptime *value* (a `Field`, an `Arm`, a
`Type` projection) becomes something that is not a value: a field name
(case 6, as today) or a type (cases 4 and 5): `let x: [field.type]`,
`take<[field.type]>(t)`, `return [arm.type] { … }`. Declared names stay
bare, because they are already types: a type parameter is a type the way it
is in any generic fn (cases 1 to 3), and a pattern binder is declared like
one (`<E>`). Values stay bare (cases 7 and 8), because an expression accepts
a comptime value as a constant, as `"${T.name}"` already does.
- *The principle:* brackets are where expansion **splices** a comptime value
  into the code's structure; names and values need none.
- `return T { … }` stays bare under (B): `T` is declared by the signature and
  used bare there (`a: T`), and the bracket would make `T` in the body differ
  from `T` in the signature.

**(C) Bracket everything expansion fills in**, names included: `[T] { … }`,
`let first: [E]? = None`, `[field.type]`, `[arm.type] { … }`. Every
substitution is visible. The signature then either brackets too (`fn
cmp<T is Struct>(a: [T], b: [T])`, noisy across every stamped fn in
`core.auto`) or stays bare, and then `T` has two spellings in one fn. Values
(`field.name`) would follow for consistency, which `"${field.name}"`
already writes without.

**(B′) (B), plus a type parameter as a struct literal's head (case 2):
`return [T] { … }`.** The one place a type parameter does something no
generic fn can: an ordinary generic fn cannot construct a `T`, and a stamped
one can only because expansion knows its fields. Marks exactly the
construction, nothing else.

| | (A) | (B) | (B′) | (C) |
|---|---|---|---|---|
| `a: T` (signature) | `T` | `T` | `T` | `T` or `[T]` |
| `return T { … }` | `T { … }` | `T { … }` | `[T] { … }` | `[T] { … }` |
| `let first: E? = None` | `E?` | `E?` | `E?` | `[E]?` |
| `take<field.type>(t)` | `field.type` | `[field.type]` | `[field.type]` | `[field.type]` |
| `return arm.type { … }` | `arm.type { … }` | `[arm.type] { … }` | `[arm.type] { … }` | `[arm.type] { … }` |
| `v.[field]` | `v.[field]` | `v.[field]` | `v.[field]` | `v.[field]` |
| `"${field.name}"` | bare | bare | bare | bare or `[field.name]` |

**Decided (round 4): (B).**

**Recommendation: (B).** It gives the bracket one meaning that explains the
cases it already has (`v.[field]`: a `Field` becomes a field name) and the
new ones (`[field.type]`: a `Field` becomes a type), while every declared name
reads as it does in any generic fn. (B′) is the choice if construction should
stand out too; it adds one case to the rule without changing the principle.

**Cost.** Small under (B): the parser accepts `[projection]` in a type
position and as a struct literal's head; a bare projection in a type position
becomes a plain parse error naming the bracketed form (no compatibility
shim, AGENTS.md). Sweep: `std/core/comptime.sv`'s comment, the
Compile-Time-Functions page and [comptime-fields]/[comptime-inline] in
LANGUAGE_SPEC.md (no std body or test uses a projection as a type today; the
only uses are a doc comment and probes). [comptime-inline]'s rule text is
rewritten to the splice principle.

## References

- [optparse-applicative](https://github.com/pcapriotti/optparse-applicative), Haskell
- [Clikt: options, mutually exclusive and co-occurring groups](https://ajalt.github.io/clikt/options/), Kotlin
- [clap derive reference](https://github.com/clap-rs/clap/blob/v3.1.18/examples/derive_ref/README.md), and [clap#2621, ArgGroup as enum](https://github.com/clap-rs/clap/issues/2621), Rust
- [bpaf](https://docs.rs/bpaf/latest/bpaf/), Rust
- [cobra: working with flags](https://cobra.dev/docs/how-to-guides/working-with-flags/), and [the one-required group](https://github.com/spf13/cobra/pull/1952), Go
- [Argu](https://fsprojects.github.io/Argu/), F#
- [cmdliner cheatsheet](https://github.com/mjambon/cmdliner-cheatsheet), OCaml
- [decline](https://github.com/bkirwi/decline), Scala
- [Swift ArgumentParser announcement](https://www.swift.org/blog/argument-parser/), and [mutually exclusive parameters](https://forums.swift.org/t/mutually-exclusive-parameters/71633/2)
- [zig-clap](https://github.com/Hejsil/zig-clap), Zig
- Salvo: [cli-args], [comptime-*], [qual-predicate], [qual-ctor-fn], [type-literal], [throw] in LANGUAGE_SPEC.md; SERDE.md SD-3/SD-4 (the decoder questions this shares); ROADMAP §2c (`field.default`, `by` at a call).
