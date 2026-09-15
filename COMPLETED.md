# Salvo Compiler — Completed work

The record: what is built, the decisions already actioned, the options explored
and abandoned, the defects found and closed, the tests that exist, and the
lessons the work left behind. Read it to find out whether a question has already
been answered — and how, and why — before answering it again.

**Work still to do lives in [ROADMAP.md](ROADMAP.md)**: open defects, the phases
not yet built, and the decisions and plans already made about them. The two
documents replaced PROGRESS.md (2026-09-09); nothing was dropped in the split.

What is built, in one paragraph: both backends (Kotlin, Rust) work end to end,
verified by compiling and running the emitted code with `kotlinc` and `rustc` to
byte-identical stdout. The language has structs, tuples, arrays, unions with
flow-sensitive narrowing, qualifiers (state and provenance, with predicates,
constructors, refinements and deductions), everything-is-an-expression control
flow, algebraic effects with handler dependencies, non-resumption
(`throw`/`try`), implicit parameters and obligation groups, linear types with a
designated `close`, and pull iteration reduced to a `next` that `for` drives.
Ownership on the Rust side is derived mechanically from deductions — no
lifetimes in emitted signatures except the one deliberate exception
([readonly-return]). Around it: `salvo analyze`, a language server, a VS Code
extension, and worked [examples/](examples/).

Companion documents: [LANGUAGE.md](LANGUAGE.md) is the narrative spec (source of
truth); [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) states every feature as a labeled
rule (`[qual-erasure]` style) with the compiler decisions under it;
`BACKEND_SPEC.<backend>.md` ([kotlin](BACKEND_SPEC.kotlin.md),
[rust](BACKEND_SPEC.rust.md)) repeats rules with backend interpretation details
and adds backend-prefixed rules (`kt-…`, `rs-…`) — load it only when working on
that backend. Labels are referenced from compiler code and tests
(`grep -rn '[rule-name]'`); backend-prefixed labels may only be referenced from
that backend's crate. Keep all of these in sync when adding or changing
features. Detailed feature mechanics live in the specs; these two documents keep
the decision log, the plan, and the hard-won operational knowledge.

How this document is ordered: the **decision log** comes first, newest entry at
the top — it is the running narrative of what landed and what each thing cost.
After it, the **milestone history**, the **closed defects**, and then one section
per feature *arc* (linear types, effects, iterators, places, deductions, names,
overloads, std). Those arc sections keep the headings they were written under —
several still say "Roadmap:", because the prose in both documents refers to them
by name; each is the plan *as executed*, and anything still open under it moved
to ROADMAP.md with a one-line pointer left behind. The **test inventory** and **gotchas** are last.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 1012 tests, complete: the toolchain tests are
                            # content-cached, so an unchanged one is not
                            # recompiled — ~5s warm, ~1min cold
SALVO_E2E_FRESH=1 cargo nextest run --no-fail-fast
                            # FULL: every test, nothing taken from the cache
                            # (~55-70s), with per-test timings; the pre-commit /
                            # handover check (fall back to `cargo test` if
                            # nextest is not installed)
SALVO_SKIP_E2E=1 cargo test # inner loop: ~4s, by skipping every test that shells
                            # out to kotlinc/rustc. Those tests still report as
                            # *passing*, so this is never the pre-submit check.
INSTA_UPDATE=always cargo test   # accept/update insta snapshots after intended changes

# End-to-end:
cargo run -- compile --src ./some_dir --target ./out        # --backend defaults to kotlin
cargo run -- compile --backend rust --src ./some_dir --target ./out_rs
cargo run -- compile --src ./some_dir --target ./out --emit-ast             # user-module AST dump
cargo run -- compile --src ./some_dir --target ./out --emit-ast=core.list   # one module's AST

# Type-check without generating code [cli-analyze]:
cargo run -- analyze --src ./some_dir                       # text diagnostics, exit 1 on errors
cargo run -- analyze --src ./some_dir --format json         # machine-readable diagnostics
cargo run -- analyze --src ./some_dir --backend kotlin      # also parse kotlin define files

# Generate the host side of `platform effect` declarations [cli-platform]:
cargo run -- platform generate --backend kotlin --src ./some_dir
# Writes ./some_dir/platform/<module>.kt once per module that declares
# platform effects; never overwrites, so implement the stubs and re-run
# `salvo run`.

# Language server over stdio [cli-lsp] (point your editor's LSP client at it):
cargo run -- lsp

# Regenerate the VS Code extension's TextMate grammar [cli-lang]
# (a test fails if the checked-in copy is stale):
cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json

# Verify generated Kotlin manually (the CLI prints the entry point):
kotlinc $(find out -name '*.kt') -d classes && kotlin -cp classes salvo.main.MainKt
# Verify generated Rust manually (the CLI prints the exact command):
rustc --edition 2021 out_rs/main.rs -o program && ./program
```

## Workspace layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir,
│                         #   analysis pipeline (analysis.rs), LSP server (lsp.rs), tm-grammar (lang.rs)
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols + resolve.rs/types.rs/check.rs/deduce.rs/reach.rs
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
├── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
├── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests
└── salvo-testkit/        # dev-dependency for the test crates: toolchain probing
                          #   (once per binary) + the e2e content-hash cache
std/                      # stdlib: core/ (basic, string, list, set, map, sorted,
                          #   console, …) + random.sv
vscode/                   # VS Code extension: LSP client + generated TextMate grammar
```

Adding another backend = new crate implementing `salvo_backend::Backend`,
register it in `salvo-cli/src/main.rs`, write `*.<name>.sv` define files
next to the std modules, and add a `BACKEND_SPEC.<name>.md`. Std embedding
already filters define files per backend at load time
(`SourceSet::classify`).

## Decision log — newest first

Each entry is one piece of work: what was decided, by whom, what it took, and
what fell out of building it. Entries marked "(user decision …)" record a
language-design call, which is the user's to make (AGENTS.md's first
invariant).

**Phase 5, sequence item 4: dependent-handler spawns (2026-09-15).** A spawned
child can now have effect dependencies, supplied by its own `use` clause as
handler constructions, as addrs, or a mix — on **both backends, with identical
output**. It was the biggest remaining piece of the phase and it landed exactly
as the survey predicted, which is the useful part of the record: reading both
backends first turned a refactor into three additive changes. Tests: **1006
(+3)**, and the two "not emitted yet" refusals in `emit_spawn` are gone —
`emit_spawn` now refuses only a generic handler.

**One checker table, then one shape per backend.** `spawn_dep_items` records
*which clause item satisfied which declared dependency* — a clause index per
declared dependency, in the handler's declaration order, beside `spawn_deps`.
`check_spawn` already computed the matching while draining its `supplied` list;
without the table both emitters would have re-derived it, which is two chances
to disagree about an ordering that differs routinely (`use tally, Recording()`
and `use Recording(), tally` are one program, and the child's slots are in
neither of those orders on Kotlin, where the fused class sorts them
canonically).

**Rust: the child owns a flat provider.** `__Prov_H<__D0, …>`, one `pub` field
per dependency, with a Has-accessor impl per dependency, emitted beside the
handler; `__Proc_H<__D0, …>` holds `handler` **and** `prov`; and `handle`
builds the *existing* `__Deps_H` view over the provider and calls through
`__Impl_H`. `__Deps_H` needed no change at all — it is a view over **one**
provider, and a flat struct is a provider. What the generic version did cost is
one thing the survey did not name: `SalvoProcess: Send`, so the impl has to
*say* `__D0: Log + Send + 'static` where a non-generic body gets `Send` from
the auto trait.

**Kotlin: nothing new is generated.** A dependent handler already stores its
carrier as a bounded type parameter, so the process repeats it
(`class __Proc_Counting<__Fx>(private val handler: Counting<__Fx>) :
salvo.SalvoProcess where __Fx : __Has_Log, __Fx : __Has_Tally`) and the *spawn
site* calls `emit_fx_class` exactly as a `use` site does. Kotlin infers `__Fx`
from the constructor argument, so unlike the `use` path there is no
type-argument list to append. The asymmetry between the backends is the same
one the fusion has everywhere: Rust must own and rebuild a view per activation,
Kotlin's objects alias.

**Item 3's refactor is what made a clause item uniform.** A construction is
`D::new(args)` / `D(args)`, an addr is the forwarding stub
`__Stub_D::new(addr)` / `__Stub_D(addr)` — and both go into the same slot, so
the child cannot tell which of its dependencies is a process. That is Example
6's binding swap, executed: the compile-and-run case per backend gives
`Counting [Log, Tally]` a constructed `Recording()` for one and an
`Addr<Tally>` for the other, and prints `last bumped 3` / `sum 5` on both.
The test's ordering is deterministic without synchronisation, which is worth
copying: everything is chained through **one** process's arrival order, and
`report` forwards `main`'s own reply token to the child's local `Log` instance,
so the answer comes from inside the child.

**Two defects surfaced while writing that test, and both were fixed the same
day** (records with repros under "Defects found and closed"; both pre-existing
and unrelated to concurrency, and each had a *silently wrong output* half that
the loud half hid). An **effect member whose name is also a std fn** broke
std's own emission — the checker's [effect-available] rule is import-scoped, the
emitters' `effect_of_fn` map is program-wide, and `fn_over_member_calls`, the set
that bridges them, was only filled when the member lost a contest *in scope*;
filling it wherever the fn path commits also fixed `to_upper@core.string("hi")`,
which had been running the member and printing `hi!`. And **consuming a
handler's stored values** — a state field or a constructor parameter — was a
silent clone on Rust and a share on Kotlin (`eaten 3 kept 2` versus
`kept 3`), up to and including duplicating a *linear* obligation; it is now the
read direction of [effect-state-store], with `copy` as the remedy, linear values
refused outright and Copy scalars exempt. The first defect cost the test its
`add` member (renamed `tick`), the second its `copy(last)` — which the checker
now requires rather than rustc.

**Phase 5, sequence item 3: the `use addr` forwarding stub (2026-09-15).**
Both backends now bind an effect to a process: `__Stub_E` holds an addr and
implements the effect by sending, and `use addr` registers it exactly as a
handler instance is registered. A function declaring `[Log]` therefore never
learns that its capability is a process — verified by a compile-and-run case
per backend printing the same `noted 2`. Tests: **1003 (+3)**.

**The refactor is the interesting part, and item 4 needs it.** Each backend's
`use` path used to be shaped around a *handler declaration*: it built
`H::new(args)` itself and reached into the decl for dependencies and generics.
The stub has no declaration, so the path now takes the **expression that builds
an instance** — `emit_fusion_instance` in Rust (the old `emit_fusion_use` is a
thin wrapper over a shared `emit_fusion_inner` with an optional decl),
`bind_effect_instance` in Kotlin (extracted from `emit_use`'s tail). Everything
downstream is indifferent to which kind of instance it got, which is precisely
the property Example 6's binding swap rests on and precisely what a spawned
child will need when a dependency arrives as an addr rather than as a
construction.

**Why the stub belongs to the effect, not the use site**: it depends on nothing
else — the protocol decides its members and the message type — so one per async
effect is emitted beside the message type, and every `use addr` of that effect
shares it.

**Phase 5, sequence item 2: the `async effect` kind and sendability
(2026-09-15).** The kind marker, its refusal list, and the binding gate that
closes the design's carried named question. Tests: **1000 (+5)**.

**What the kind buys is a *declaration-time* answer.** Before it, "can this
protocol be a process?" was answerable only by trying to spawn it; now
`async effect` says so, and everything a seam cannot carry is refused where the
author is choosing: a **kept** parameter (a send payload always crosses, so the
clause must say `=> !p`), a **`Mut`** parameter, a **`proj` return**, and a
**non-sendable** payload [async-sendable]. `send fn` requires the kind, and
`spawn`, `use addr` and even *naming* `Addr<E>` refuse a plain effect — the
last reported where the type is written rather than at a spawn that could never
have produced it. What stays legal is the binding swap: `use H()` on an
async-effect handler still runs its members inline, which is why the kind sits
on the effect and not the handler.

**Sendability, as decided**: a payload may not transitively hold a **function
value** (shared, and `Rc` in Rust — not `Send` [rs-fn-field]) or a **`proj`
view** (a borrow of the sender's value), checked structurally through fields,
type arguments, arrays, tuples and unions with a depth guard. `Arc`-where-sent
inference stays C-4(c)'s growth point.

**The lesson of the slice was not about effects at all: never let a script
write a Rust string literal.** Several diagnostics in this session were written
through Python heredocs, and Python ate the `\`-continuations inside its own
triple-quoted strings — so the *Rust* source got one long line with runs of
indentation baked into the message ("a message is enqueued on&nbsp;&nbsp;…&nbsp;&nbsp;a
process"). Repairing that mechanically then went wrong twice more: adding a
space before every continuation corrupted **code templates** in the emitters
(`{{\n\` became `{{\n \`, and `{own_path}::\` grew a space), and a
"revert" that dropped the character before the space deleted `)` and `;` from
three generated-code templates — caught only because the golden and
compile-and-run tests failed. The way out was to stop patching and restore the
damaged lines from `HEAD` by diffing against it. Three rules for next time:
**write diagnostics by hand or with a real editor tool, never through a shell
heredoc**; **never apply a blanket regex to string literals in the emitters**
(their strings are *code*); and when a repair script's second attempt is also
wrong, **restore from git rather than patching a third time**. The one good
side effect: the sweep found and fixed six *pre-existing* diagnostics that had
been damaged the same way in earlier sessions (`^`-on-a-projection, three
qualifier-widening messages, two `once`/`linear` ones).

**Phase 5, sequence item 1: the respelling sweep — `Addr` and `@self`
(2026-09-15).** Both surface changes EFFECT_UNIFICATION.md decided, landed
before any more emission was built on the old spellings. Tests: **995 (+2)**.

**`Pid<E>` → `Addr<E>`**, mechanically, across 21 files: the `core.process`
declaration, the checker's constant and its five side tables and predicates
(`addr_calls`, `use_addrs`, `addr_effect`, `addr_receiver`, `check_addr_call`),
both backends' type maps and lowerings, every test, the spec rules (with
[async-use-pid] refreshed to **[async-use-addr]**) and the prose. Two things a
mechanical rename teaches: **it breaks articles** — "a addr", "a `Addr`" —
which needed a second pass and is worth doing in the same commit; and **it
edits its own record**, turning ROADMAP's and COMPLETED's "`Pid<E>` →
`Addr<E>`" into "`Addr<E>` → `Addr<E>`", so the *decision* text has to be
restored by hand afterwards. **The shipped runtime was renamed too**, after the
first pass at this slice tried to keep `pid` there on the grounds that it *is*
an index into the scheduler's table (user, 2026-09-15: "why keep it pid in the
runtime?"). The argument against keeping it is the rename's own: the runtime is
emitted **into the user's program**, so it is precisely where a real OS pid — a
`platform effect` wrapping process management — would sit beside the
scheduler's, and where a second word for one thing costs a reader following a
send across the seam. `salvo_send(addr, …)`, `SalvoCtx::addr` and every
internal index now say `addr`; "process" stays the noun for the thing an addr
names, so `SalvoProcess`/`procs` are untouched. Both backends' four behaviour
scenarios passed unchanged, which is what made a 56-site rename a two-minute
job.

**`self.k(…)` → `k@self(…)`**, and the selector spelling paid for itself
immediately. It joins `k@E` and `k@module` as one family — "the call says which
it means" — and it **deleted a rule**: the receiver form needed a check
refusing a variable named `self` inside a handler member, because a local would
shadow the form silently; a selector cannot be shadowed, so the check and its
test are gone and `self` is an ordinary name again everywhere. Implementation:
a new `Expr::SelfScoped` leaf (nine exhaustive matches again, all trivial this
time — a callee has no sub-expressions), `SELF_SELECTOR` recognised
contextually after `@` in `parse_postfix`, and the old dot form made a **plain
parse error** naming the replacement. A selector written *without* a call is
its own diagnostic ("names a member of the enclosing handler, which is not a
value"), since a handler member is no more a function value than an effect
member is.

**The rest of phase 5, sequenced — and the sugar pass deferred out of it
(user decisions 2026-09-15, on EFFECT_UNIFICATION.md's plan).** That document
(a parallel read-only session's, decided across six rounds with the user) hands
implementation four items; sequencing them against the phase-5 build produced
an agreed eight-item order, in ROADMAP.md. The three calls:

1. **The surface changes go first**: `Pid<E>` → **`Addr<E>`** (the token is a
   many-shot *address*; `Pid` is the OS's word, which a `platform effect`
   wrapping process management would want) and `self.k(…)` → **`k@self(…)`**
   (one selector family — `k@E` picks an effect's member, `k@self` the
   enclosing handler's — instead of a second, dot-shaped mechanism). Both touch
   landed surface, and both are cheapest now: `Pid` is in ~124 places already
   and one emitter slice added a third of them, while the self-send is checked
   but **not yet emitted**, so no lowering has to be rewritten. Then
   `async effect` (EU-5), because it changes the gate the emitters key on and
   its blast radius is smallest before the remaining emission lands.
2. **Sendability is part of `async effect`'s refusal list, not a slice of its
   own.** A payload that transitively holds a **fn-typed field** ([rs-fn-field]
   lowers one to `Rc`, which is not `Send`) or a **`proj` view** is refused —
   checked at the *declaration* for member payloads, which is where the author
   is choosing, and at the site for `replyto` captures and spawn arguments.
   `Arc`-where-sent inference stays C-4(c)'s recorded growth point.
3. **The sugar pass leaves phase 5** — call syntax, `then`/`then!`, `defer`,
   merge/join and the gate generalization become later items with their own
   decision surfaces, with EU-6/EU-7b's decided content folded into ROADMAP.md
   so nothing open lives in a working document. **Linearity in collections
   moves after `watch` and the deadlock baseline.**

**The question the sequencing answered, worth keeping.** The user asked whether
the `use addr` forwarding stub — item 3 of the sequence — is "the thing that
makes sync → async calls possible", and if so whether it fits the *dependency*
case at all. It is not: the stub is an **address-to-interface adapter**, and
its bodies enqueue, which is legal from a synchronous frame and from inside an
activation alike (the sync→async bridge is `waitfor`, confined to `main`). The
dependency case does not merely tolerate it but *requires* one, because a
handler is compiled **once** and bound many ways: whether `Counting`'s `[Log]`
is a local construction or a remote process is decided per *spawn site*, so
the child receives something implementing the effect (`__Fx: __Has_Log` under
the fusion) and the stub is what an addr becomes to satisfy that. The stub is
the runtime witness of Example 6's binding swap. Where the instinct was right
is one pass later: an **answering** member cannot have one implementation for
both bindings — from a process the call parks, from synchronous code it must
block, which `main` may do and a process may not — so the two stub readings
(EU-2, and point 2 of the unification's stated intent) arrive with the sugar
pass. The first pass has one stub precisely because nothing answers.

**Phase 5 step two, slice five: the emitters — a process runs (2026-09-15).**
Both backends now compile and run the first asynchronous program, and print
**the same thing**: a counter handler spawned on a pool, two sends through its
addr, then `main`'s `waitfor` asking for the total — `sum 5` from Kotlin and
Rust alike. Tests: **993 (+4)**, including one compile-and-run case per
backend with identical expected output (the parity assertion at the *surface*,
where until now it was only at the scheduler library) and one
generated-text assertion per backend so a lowering regression names itself.

**Three generated pieces per program, and the shapes are mirrors.** The
protocol's **message type** (`__Msg_E`: a Rust enum, a Kotlin sealed class with
nested classes) sits beside the *effect*, not the handler — a sender holds a
`Addr` and knows only the effect it serves, which is the same fact that makes a
process and a locally `use`d handler interchangeable. The **process body**
(`__Proc_H`) sits beside the handler, owns the handler instance (a process's
state *is* the handler's) and dispatches messages onto its members. The three
types **erase to scheduler handles**: `Addr<E>` and `Pool` to `usize`/`Int`,
`Reply<T>` to `SalvoReply` — with their Salvo type arguments dropped, since the
runtime is untyped and the message type is what carries payloads across.
[rs-process], [kt-process].

**What the slice taught, worth keeping.**

- **The receiver of an addr send is a *read*, and forgetting it showed up as a
  warning.** The first running program reported `counter` as never used: the
  checker peeks the receiver's type to *decide* the call is an addr send and
  then, in the original code, never checked the expression at all. Peeking is
  right for deciding, but the receiver still has to be checked exactly once —
  which is now what `check_pid_call` does first. A "never used" warning about
  a variable used twice is the kind of tell worth chasing rather than
  suppressing.
- **`intrinsic fn` lowerings are keyed on the *first parameter's* type**, not
  on the absence of a receiver: `pool(size: Int)` is `("pool", Some("Int"))`,
  which cost one wrong guess (`None`) and one clear diagnostic to find.
- **A generated `run { }` is how Kotlin gets a block expression**, and a
  destructuring `val (out, __wid) = waiter()` is how the token and the waiter
  id arrive together — the Rust side is a plain block with a tuple `let`.
- **Both backends compile the output warning-free**, which is the bar for
  generated code (a warning in emitted code is noise the user cannot fix):
  checked by hand this slice, and by the existing warning-free runtime tests
  for the library underneath.

**Deliberately still refused, each a named diagnostic** (the compiler's output
is the next slice's work list) — *as of this slice*: spawning a handler with
effect **dependencies** — its members take a fused value the child would have
to hold and thread, which is the biggest remaining piece — a **generic**
handler, a **generic effect** as a protocol, `replyto` (needs the
parked-continuation table `resume` dispatches on), a **self-send** (needs the
activation's own addr), and `use addr` (needs the forwarding stub). Sequence
item 3 closed `use addr` and item 4 the dependencies, both later the same day;
the current list is in [rs-process] / [kt-process].

**Phase 5 step two, slice four's leftovers closed the same day
(2026-09-15).** Three of the four gaps the checker slice recorded are gone;
the fourth needs a language-design call and is in ROADMAP.md's table.
Tests: **989 (+9)** — three for the mechanical leftovers, six for the
self-send.

- **An addr receiver may be any place.** `registry.child.bump(1)` and
  `many[0].bump(1)` now resolve, through a `peek_place_ty` that reads a
  place's type *without checking the expression* — a variable's from the
  locals, a field's through `field_ty`, a tuple element's and an array
  element's structurally. That restriction was never about addrs: checking the
  receiver here as well as on the ordinary dot path would duplicate its
  diagnostics and count a move twice, and a place is exactly the shape whose
  type can be read without doing either. A receiver that is a *call* still
  needs a `let`, recorded in the rule.
- **The forms stop at a closure** [async-no-closure]. `can_spawn`, `in_main`
  and `own_handler` are saved and cleared around a lambda body, beside the
  loop-stack barrier that was already there — so `spawn`, `waitfor` and
  `replyto` inside a lambda are errors. The wording matters as much as the
  rule: inside a closure the general remedies ("add `spawn` to the effect
  list") do not exist, so each diagnostic is written in the closure's terms.
  This is [fate-lambda]'s deferral made *checkable* instead of assumed.
- **An overloaded send member through an addr picks by arity.** The alternative
  — the ordinary overload machinery — types the arguments to choose and then
  types them again against the winner, which double-reports every mistake in
  them; arity settles every overload the first pass can express, and a
  same-arity tie is refused rather than guessed.

**Self-sends decided and built the same day: `k@self(args)`** (user decision
2026-09-15, option (a) of three — the spelling the sugar tower's merge/join
form already assumes; (b) relaxing the self-dispatch refusal was rejected for
reading identically to a call on a same-named *dependency*, and (c) waiting
for the sugar pass for leaving finish-then-continue unwritable). The form is a
message to the process the enclosing member belongs to, and its point is
**ordering**, not reach: an unqualified call runs `k` inside this activation,
a self-send runs it as its own later one — which extracting a function cannot
express, since a call runs the work now. Defined for both bindings, like
everything else here: an enqueue when the handler was spawned, the ordinary
inline member call when it was `use`d.

**It needed no syntax at all.** `k@self(args)` already parses as a dot-call,
so the whole form is a checker rule — the third time this surface has cost
nothing in the parser (`use addr` and dot-call through an addr were the others),
which is what "the effect surface is the model" keeps buying. Two details
worth keeping: a handler member **may not declare a variable named `self`**,
because it would shadow the form silently (refused there, and an ordinary name
everywhere else — contextual, not reserved); and the **self-dispatch
diagnostic now names the remedy** when the member it refused was a `send fn`,
which turns the error that used to say "move the shared logic into a function"
into one that says "send it to this process instead: `bump@self(…)`".

**Phase 5 step two, slice four: the checker rules — the whole first-pass
surface now type-checks (2026-09-15).** A program can be written and analyzed
end to end; only emission is still refused. Tests: **980 (+19, −3)** — a new
`async_tests.rs` whose first case is the *whole* surface checking clean (two
spawns, one supplying a dependency with the other's addr; a send through a
addr; `waitfor` with the token consumed in its block; `use addr` then an
unqualified call), and the three pending-refusal tests deleted as promised.

**The finding that shaped the slice: a spawn is a `use` whose dependencies
come from elsewhere.** `check_use` was 250 lines that did two things —
construct a handler, then resolve its dependencies from the enclosing scope —
and only the second differs for a spawn. So the construction half became
`check_handler_construction` (arguments typed and *stored*, generics inferred
from arguments and any written type list, constructor implicits filled),
answering the concrete effect and its dependencies *already substituted*; `use`
keeps `finish_use` (resolve from scope, register, push onto `effect_env`) and
`spawn` matches the declared dependencies against its own clause instead. The
refactor was behaviour-preserving except for one message — "unknown handler
`X`" lost its "in `use`" until the form name was threaded through, which a CLI
test caught, and which is the argument for the shared helper taking the form
name rather than guessing.

**Rules that fell out of existing machinery rather than being written.**
`waitfor`'s "the block must consume the token" is not a rule at all: the
binding is an ordinary linear local, so [linear-obligation] reports the leak
and names `send` as its discharge. A payload crossing a seam is
`fate_move` — the same consumption a `use` argument gets — which is what makes
`counter.total(out)` discharge the token's obligation. And a spawn's value
being `Addr<E>` needed no new typing: `Addr`'s argument is the effect
[async-types], so the handler's own `of` clause answers it.

**Three rules that did need stating, each with a reading worth recording.**
(1) **`replyto k(captures)` takes `k`'s parameters as captures-then-answer** —
the *trailing* parameter is what the token carries, matching the
trailing-token convention the sugar tower's `-> T` will use, so
`arrived(id: Int, sum: Int)` minted as `replyto arrived(7)` is a `Reply<Int>`.
(2) **Only a `send fn` is reachable through an addr**, because a member that
answers would have to park its caller — which is exactly the call-sugar pass's
named question, so refusing now commits to nothing. (3) **A dependency
*constructed* in a spawn's `use` clause may not have dependencies of its own**:
there is no scope on the child to resolve them from, and the remedy the
diagnostic names is an addr of a process already serving the effect.

**What the emitters do meanwhile is mark their own seams.** Rather than
letting a correct program reach the handler path and be reported as an
"unknown handler `counter`", both backends now refuse `use addr` and a
send-through-a-addr with diagnostics that name the missing piece — so the next
slice's work list is readable from the compiler's own output.

**Phase 5 step two, slice three: `core.process` and the linear opaque type
(user decisions 2026-09-15, three calls presented and answered in one
round).** The types the asynchronous forms produce and consume now exist:
`Addr<E>`, `linear intrinsic type Reply<T>` with `send` beside it, and `Pool`
with `pool(size)`. Tests: **964 (+5)**.

**The three calls, each a rule that said "no" before.**

1. **`Addr<E>`'s argument is an effect, and that is the one exception to
   [effect-not-data]** (option (a) of three). An addr is a handle to a process,
   and what a holder may *do* with it is exactly the effect the process
   serves — which is also what lets a process and a locally `use`d handler
   stand behind one name (the binding swap). Implemented as narrowly as it
   was granted: the check lives in `validate_type`, which is the only walk
   that knows what a type argument *belongs to*, and it fires only for std's
   `Addr` in its only parameter position with a bare effect name in it.
   `List<Counter>` is still refused, and so is `Counter` anywhere else — both
   asserted.
2. **`linear intrinsic type`** [linear-opaque], over a `linear struct`
   wrapping an opaque field. A token is a scheduler handle, so its
   representation belongs to the backend, and a wrapper struct would buy a
   layer and nothing else; the user's own reason for wanting the modifier
   rather than the wrapper is that unifying the synchronous and asynchronous
   effect surfaces will want this control in the compiler's hands. It cost
   less than expected because [linear-group]'s machinery is keyed on *names*,
   not on struct declarations: `has_auto_linear`, `linear_capable` and
   `discharge_set` each gained an opaque case (the last taking the declaring
   file from `opaque_type_files`, new, mirroring `struct_files`), and
   everything downstream — leaks, `discard`, moves, the diagnostics that name
   the discharger — worked untouched.
3. **The discharger is an `intrinsic fn` beside the declaration**:
   `intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply,
   !value`. This makes "a `Reply` is a one-shot `Addr` with a single send
   member" true in the type system, and the same-file rule then does the rest.
   Note `!value`: the payload crosses the seam, so it is consumed, not kept —
   the same reasoning that makes a send member's own clause `=> !c`.

**A [backend-never-wrong] hole closed on the way, and it was not the new
code's.** Naming an `intrinsic type` that a backend has no mapping for used
to fall through to "emit the Salvo name verbatim" — only `Any` was refused,
by name, in the Rust backend. So `fn hold(p: Addr<Counter>)` emitted
`Addr<Counter>` and left kotlinc/rustc to report a dangling reference: wrong
output, deferred to the target compiler, which is exactly what the invariant
forbids. Both emitters now consult `symbols.intrinsic_types` and refuse with
"the `Addr` type is not supported by the … backend yet". `Addr`, `Reply` and
`Pool` are deliberately left unmapped: what they should map to is a
consequence of the process classes' shape, and guessing now would be a
commitment made in the wrong slice.

**One process lesson, recorded because it nearly hid two failures.**
Summing `cargo test`'s "test result:" lines with awk reported *0 failed*
while two insta snapshots were in fact failing — a `FAILED.` line does not
parse like an `ok.` line. `cargo nextest run`'s single summary line
(`964 tests run: 964 passed`) is the number to trust. The snapshots
themselves were the intended kind of churn: `TypeDecl` gained a `linear`
field, so 14 dumps gained one `linear: false` line and nothing else.

**Two calls closed on the way (user decisions 2026-09-15).** **`on POOL`
stays required** — the frozen grammar's reading confirmed, so every spawn
says where it runs and there is no ambient default pool. **A `send fn` still
writes its deduction clause** — `send fn go(c: Config)` in an effect needs
`=> !c` — deferred rather than denied: a send payload always crosses the
seam, so implying consumption would be sound, but the clause stays until the
surface is real enough to judge it noise or documentation. Both are in
LANGUAGE_SPEC.md under [async-spawn-expr] and [async-send-fn].

**Phase 5 step two, slices one and two: the asynchronous surface's syntax
(2026-09-15).** The declaration forms, then the expression forms — both
**contextual**, so phase 5 reserved *not one word*. What checks: `send fn`
members of effects and handlers [async-send-fn] and `[spawn]` in effect lists
[async-spawn-effect]. What parses and is then refused: `spawn H(args) use …
capacity N on POOL` [async-spawn-expr], `replyto k(c)` / `replyto! k(c)`
[async-replyto], `waitfor out: Reply<T> { … }` [async-waitfor], and `use addr`
[async-use-addr] — the last needing no syntax at all, since the `use`
statement already takes an expression. Tests: **959 (+13)** — 3 parser tests
and 3 checker tests in this slice, on top of the declaration slice's 2 and 5.
The rules are now written down: LANGUAGE_SPEC.md gained an "Asynchronous
effect handlers" section ([async-process] … [async-use-addr]), which also
closed the dangling-label bug the declaration slice left (two labels lived in
code with no rule behind them). LANGUAGE.md is deliberately untouched until
the feature runs.

**The half-built form is refused on both sides, and that is the point.** A
parsed-but-unimplemented form has two honest states and one dishonest one:
the checker rejects it (honest), the emitter rejects it (honest), or it
type-checks clean and emits nothing (silently wrong output — the class of bug
[backend-never-wrong] exists to prevent). So `check_expr` answers each of the
three forms with one diagnostic naming it, `Ty::Unknown` as its type so
nothing cascades [type-unknown-lenient], and both emitters carry a refusal
arm for the case where emission runs on an already-rejected program. The
three checker tests asserting those refusals are written to be **deleted** by
the slice that implements them, and ROADMAP.md says so.

Five things worth keeping from the build:

- **Contextual recognition needs a shape, not a word.** Each form is
  recognised from its word *plus what follows*: `spawn` before a **name**
  (`spawn(x)` is still a call), `replyto` before a member name or `!`,
  `waitfor` before `name:`. That is what keeps `let spawn = 1`,
  `fn replyto(n: Int)` and a field named `send` legal, and it is the `iter
  fn` precedent applied five times over (`capacity` and `on` are ordinary
  identifiers too).
- **The clause keywords *are* the named arguments.** `capacity N` and
  `on POOL` are required with no default, and the `use` clause is optional —
  the reading the frozen grammar's brackets give. Making `spawn` a function
  instead was rejected in the design round for two reasons that also
  constrain the parser: Salvo has no named arguments, and a handler
  construction is not a value.
- **The handler construction is parsed, not checked.** `spawn H(args)` and
  the `use` clause items are kept as expressions — the shape `Stmt::Use`
  already keeps — and deliberately *not* type-checked in this slice: checking
  `Counting()` as an expression would report the constructor as an
  unresolved function, since handlers are not values. Only `capacity` and
  `on` are checked, being ordinary expressions.
- **Every exhaustive `Expr` match is a classification decision, and there
  are nine.** `collect_assigned_expr`, `expr_mentions`, `expr_exits`,
  `expr_returns` (core), `lends_of_expr`, the deduce walker, `expr_names`
  (reach), `collect_mutated`/`collect_declared`/`block_terminates` (both
  emitters) — each carries a comment saying why a wildcard arm would be a
  wrong-code bug rather than a missing diagnostic, and each needed a real
  answer for the new forms: the clauses are ordinary reads, a `waitfor`
  binder is a declared name, none of the three diverges, and none yields a
  view (nothing crossing a process boundary can borrow a local).
- **A `waitfor` block is left unchecked on purpose.** Its whole content is
  the token the binder introduces, and nothing types that binder yet, so
  checking the block would report the token as an unresolved name *on top of*
  the pending diagnostic — two errors for one unbuilt feature.

**Phase 5 step one built: the scheduler library (2026-09-15).** The first
piece of asynchronous effect handlers, and it needed **no compiler
cooperation** — as designed, it is a library in each backend's runtime
files: `salvo-backend-rust/runtime/scheduler.rs` and
`salvo-backend-kotlin/runtime/scheduler.kt`, mirrored APIs (`salvo_pool` /
`SalvoSched.pool`, `salvo_spawn` / `spawn`, `salvo_send` / `send`,
`salvo_mint`+`salvo_mint_gated` / `mint`+`mintGated`, `SalvoReply::send` /
`SalvoReply.send`, `salvo_watch` / `watch`, `salvo_waiter`+`salvo_wait` /
`waiter`+`awaitReply`). Both implement the decided semantics: run-to-completion
activations on pool threads (Kotlin's are daemon threads — the program ends
when `main` returns), one arrival-order queue per process with an explicit
required bound that blocks a full sender, replies with reserved capacity (a
reply never blocks and never counts against the bound), the gate (at most one
outstanding; while gated only the awaited reply is delivered), death by
faulted activation caught at the dispatch boundary with watcher notification
and silent no-op sends to the corpse, and the idle-with-parked-gates report
(stderr + exit 1) instead of a hang. Tests: **946 (+5)** — four behaviour
scenarios per backend, *with identical expected output on both*, which is the
parity assertion made executable, plus the modules joining the
compile-warning-free registries (`bytes.kt` was missing from Kotlin's list
and is now in it).

Three things worth keeping. **Kotlin's `Object()` monitor is a warning**
("this class is not recommended for use in Kotlin"), and the runtime tests
reject warnings because they land in user output — so the lock is a
`ReentrantLock` + `Condition` (`withLock`/`await`/`signalAll`), which mirrors
Rust's `Mutex` + `Condvar` one-for-one anyway. **A Kotlin file's *name*
decides its facade class**, so batched driver programs live as `main.kt` in
per-case directories to be `<pkg>.MainKt`. And the scheduler method that
blocks `main` is `awaitReply`, not `wait`: an object's `wait(Int)` sits
beside `java.lang.Object.wait`, which is a trap for generated call sites.
Both behaviour harnesses kill a child that overruns 30s (60s on the JVM) and
report its partial output, so a lost wakeup fails a test instead of hanging
the suite.

**Supervision and process death decided (user decision 2026-09-15, same day
as the design; nothing built yet).** The last phase-5 design prerequisite
(SUPERVISION.md, S-1…S-4, accepted as recommended): **death is a faulted
activation** — and only that, since [effect-member-no-effects] makes
Salvo-level uncaught throws impossible by construction (the fault sources
are backend runtime faults and platform-handler failures); no `kill`
(ask-to-stop plus stop-forwarding covers its useful half). **The monitor
surface is one member**: `watch(addr, on_exit: Reply<Exit>)` — the death
notification is itself a linear token, so the kernel's four pieces still
suffice and an unhandled watch is a leak diagnostic. **The corpse's
obligations are lost** (the monitor is the recovery mechanism, at the domain
level — LC-4's guarantee worded as decided); sends and fulfils to a dead
target are silent no-ops (Erlang's rule); the scheduler reports
**idle-with-parked-gates** as a named runtime error, catching orphaned
requesters and a hung `waitfor` in `main`. **Supervision is a pattern, not a
construct**: interceptor + `watch` + `spawn`, clients hold the supervisor's
addr and never observe the death; strategies are handler logic; a std
`Supervisor` handler waits for real usage. With this, **the phase-5 decision
space ahead of implementation is empty**; the plan is ROADMAP.md's
"Threading and concurrency" step 2.

**Phase 5 designed: asynchronous effect handlers (user decisions
2026-09-14/15, across one extended session; nothing built yet).** The design
pass ROADMAP.md's phase 5 called for ran ahead of schedule and went further
than its four questions: the user took a **direction** — *the effect surface
is the concurrency model* — and then decided the first implementation pass in
full. The working documents carry the detail and stay alive until
implementation lands (the FILE_SYSTEM.md pattern): **CONCURRENCY.md** (the
option space, the direction, the first pass, the remaining opens),
**CONCURRENCY_EXAMPLES.md** / **CONCURRENCY_EXAMPLES.effects.md** (worked
examples: message-passing form and effect form, the kernel and sugar tower,
the deadlock example and its statics), **LINEARITY_COLLECTIONS.md** (the
first prerequisite, decided — below), **DESIGN_DOC.md** (the reusable
working-document template these follow). The shortest honest summary:

- **The model** ("asynchronous effect handlers", after Ahman & Pretnar's Æff,
  its closest formal relative): a process is an effect handler bound
  asynchronously — state struct + one function per member, run-to-completion
  on a small scheduler library (no runtime in generated code, full backend
  parity). The kernel is four pieces — process state; `send` (enqueue a
  member invocation, the only send); `replyto k(captures)` (allocate a parked
  one-shot continuation, yielding a **linear** `Reply<T>` [linear-obligation]
  — statically enforced where OCaml checks dynamically); and the gate
  (`replyto!` — bounded selective receive, at most one outstanding per
  process). Everything else — call syntax, `then`, `defer`, futures,
  merge/join — is a strict sugar tower over the kernel, deferred to later
  passes each with its own decision surface.
- **The four ROADMAP DECISIONs, answered**: sendability = structural rule +
  diagnostic (C-4(a); `Arc`-where-sent inference recorded as the growth
  point); effects of a spawned process = handler dependencies
  [effect-handler-deps] supplied at the spawn site; substrate =
  run-to-completion on pools (an `on pool(n)` clause; one arrival-order
  queue per process, **bound explicit and required at spawn**; a blocking
  send from inside a handler is allowed and its load-conditioned edges join
  the deadlock graph); async **dissolves** — no colouring, honouring the
  2026-09-04 record by making the question moot.
- **First-pass surface (grammar frozen)**: `send fn` members with explicit
  `Reply<T>` parameters; `replyto`/`replyto!`; `r.send(v)` discharges (no
  `fulfil` verb — consumption says it); `Addr<T>`; `spawn H(args) use
  Handler(...), addr, ... on pool(n)` (dependencies as constructor params or
  spawn-site `use` clause — handlers never cross, *construction* crosses; no
  `fork`); lowercase `[use, spawn]`; `use addr` binds an effect to a
  forwarding stub; **`waitfor`** is main's explicit blocking bridge and the
  program ends when `main` returns (no run-to-quiescence ambience).
  Supervision = interception (a policy handler wrapping a process — the
  existing mechanism across the scheduler boundary). The deadlock baseline
  is the effect-graph cycle check (awaits *are* effect calls); stratification
  and fallbacks wait for observed false positives.
- **Linearity in collections decided** (LC-1…5, LINEARITY_COLLECTIONS.md,
  closing the intrinsic-containers deferral of 2026-09-12 and retiring
  [linear-composite]'s interim refusal for collections *when built*):
  phase 3's conditional-container machinery (`Box<T canbe linear>`,
  settle-by-decomposition) **extends to the intrinsic collections** —
  an instantiation is linear iff an opted argument is; take-by-move
  APIs returning the [linear-union-arm] `T?` shape; `drain`+`for` as the
  discharge terminal; `Set` and map keys refused (dedup *is* dropping);
  struct fields require declared `linear struct`, destructuring discharges;
  handler state owns obligations across activations with the guarantee
  honestly weakened to "the process owes until it ends" — end-of-life handed
  to the supervision design as its opening requirement.
- **Sequencing** (user): supervision/monitors is designed *before*
  implementation (SUPERVISION.md); **[fate-lambda] is deferred to the
  call-sugar pass** — every first-pass form crosses values, not closures
  (spawn-`use` args, `replyto` captures, `waitfor`'s token), so the hole is
  not on the critical path. The scheduler library lives in **backend runtime
  files** (the emitted-support precedent), with anything needing compiler
  cooperation flagged to the user. Spec rules and examples-file respelling
  land with implementation.

**`Bytes`, `read_to` and the copy one-shots — the last of phase 4
(user decision 2026-09-15, built the same day).** The previous entry left one
question open (Kotlin boxes a `List<Byte>` payload and cannot render
`UByteArray` instead) and the user answered it with the largest of the three
options: **a `Bytes` type of std's own**, plus the **fill-a-buffer reads** they
had asked for and the ergonomic layer over them. Naming was theirs too:
`read_to`, following `read_to_str`, with the *parameter type* picking the
overload.

**`core.bytes`** [bytes-type]: `intrinsic type Bytes canbe Mut`, and the pair
is exactly `Str`/`Mut Str` — `Bytes` is read (`size`, `get`, `slice`,
`index_of`, `to_str`, `to_hex`, `str_of_bytes`, `for b in data`), `Mut Bytes`
is built (`add`, `append`, `set`, `clear`), and dropping the `Mut` reaches the
read surface. Constructors `bytes_of(...)`/`mut_bytes(...)` mirror
`mut_str`/`mut_list_of`. `slice` copies (nothing in the surface borrows without
saying so), out-of-range reads answer `None`, `set` out of range does nothing,
`==` is structural, `copy` really copies, and a buffer is deliberately **not
hashable** — the Kotlin buffer is one mutable object, and a key that can change
under its map is a bug no later diagnostic would catch. `to_bytes` and
`str_of_bytes` moved here from `core.string`.

**The fill-a-buffer reads** [fs-read-to]: `read_to(s, buf: Mut Bytes, max) ->
Ok Int`, `read_to(s, buf: Mut Str) -> Ok Long` (the `read_all` parallel) and
`read_line_to(s, buf: Mut Str) -> Bool` (the `read_line` parallel — `false`
where `read_line` answers `None`). They **append** rather than overwrite, which
is the decision that carries the design: `size(buf)` is then the data, so there
is no "only the first n bytes are meaningful" convention, writing a chunk back
is `write_bytes(w, buf)` with no ranged write, and `MemFs` can implement all
three with `append` alone. `clear(buf)` between steps is what makes one buffer
serve a loop. On Rust that is genuinely allocation-free after warmup (the `Vec`
keeps its room); on Kotlin it saves the payload object per step, and — now that
the payload is a `Bytes` rather than a `List<UByte>` — the per-byte box as well.

**The ergonomics on top** (option D of the four presented): `chunks(s, size)` /
`open_chunks` — a pass over a stream's bytes as `Lines` is over its lines, with
a **fresh** buffer per step, since a pass recycling its own would overwrite what
the caller is still holding — and the one-shots that keep the buffer inside std:
`copy_file`, `copy_stream`, `read_to_bytes`, `write_bytes_to`, `fill_from`.

**What it took.** `Bytes` is `Vec<u8>` on Rust for both shapes, so that backend
needed a type mapping and a dozen lowerings and nothing else. Kotlin needed a
**shipped runtime class** [kt-bytes]: `runtime/bytes.kt`'s `SalvoBytes`, emitted
as `bytes.kt` whenever a program names the type (the `compare.kt` mechanism),
because neither stdlib shape works — `List<UByte>` boxes and `UByteArray` is
fixed-size *and* not a `List<T>`. One class serves `Bytes` and `Mut Bytes`, the
`SortedSet` shape rather than the `StringBuilder` one, so dropping `Mut` renders
nothing; it carries structural `equals`/`hashCode` (which is what makes `==`
agree with `Vec<u8>`), an `iterator()` so `for b in data` stays Kotlin's own
loop, `asString()`, `toHex()` and a `toString()` that prints `[0, 255, 200]` —
the text a `List<Byte>` printed, kept on purpose so no program's output changed
when the payload type did.

**Two things the build taught us.** A `vararg UByte` parameter *is* a
`UByteArray` under the hood, so the first version of the constructor made every
generated call site warn about `@ExperimentalUnsignedTypes`; it takes an
`Array<UByte>` now. And `MemFs` met the recorded self-dispatch gap again: a
handler member may not call a member of its own effect, so each `read_to` could
not call its returning sibling — the three reads moved into module functions
(`mem_read_line`, `mem_read_all`, `mem_read_bytes`) that both the plain and the
filling members call. That is the second time the documented workaround held,
and it is still the reason the DECISION stays unscheduled.

Tests: **941 passing** (from 932), fresh. New: `crates/salvo-core/tests/bytes_tests.rs`
(5 checker tests — the read surface through a dropped `Mut`, writing a plain
buffer refused, the optional reads, `Byte` not operator-numeric, a buffer
refused as a map key), a `BYTES_PROGRAM` compile-and-run case per backend over
the whole surface plus one structural test each (`Vec<u8>` and no class on Rust;
one class for both shapes, and the native byte loop, on Kotlin), the fs and
memfs programs extended with `read_to`, the chunk pass, `read_line_to`,
`copy_file` and `read_to_bytes` — printing identically for the host and the fake
— and `snapshot_std_bytes` in the parser corpus. Churn worth knowing about: std
now declares four `close` overloads (the `Chunks` discharger) and two more
`iter`/`next` pairs, so mangled-name assertions moved (`close__2` → `close__3`,
`next__9` → `next__11`).

`examples/files/` grew steps 5–9 around the new surface and still prints its two
blocks — disk and `MemFs` — identically. Rules: LANGUAGE_SPEC.md gained
[bytes-type] and [fs-read-to] ([fs-bytes], [fs-surface] and [type-basic]
rewritten around them); BACKEND_SPEC.kotlin.md gained [kt-bytes];
BACKEND_SPEC.rust.md's [type-basic] states the `Vec<u8>` mapping; LANGUAGE.md
carries the type in its basic-types list and the new reads in "Files".

**Bytes, and the worked example — phase 4 is complete (2026-09-14).** The
last two items of the filesystem: the byte payload deliverable (§5.10.2 E of
the retired FILE_SYSTEM.md) and `examples/files/`. With them, **phase 4 of
the sequence is done** and FILE_SYSTEM.md is deleted, its decided outcomes
having moved into LANGUAGE.md / LANGUAGE_SPEC.md and this log. Its `FS-`/`O-`
labels and `§`-references survive in code comments and test docs as the
attribution of a user decision; they now read against **this log**, exactly as
OBLIGATIONS.md's did after phase 3.

**What the byte surface is.** `read_bytes(s, max) -> Ok List<Byte> | Err
FsError` and `write_bytes(s, data) -> Long` on `Fs` (mirrored as
`raw_read_bytes`/`raw_write_bytes` on `RawFs`, implemented in both host
files, delegated by `DefaultFs`, passed through by `RestrictedFs`, faked by
`MemFs`) [fs-bytes]. Nothing is encoded or decoded: text and byte operations
share one stream and one position, both counted in bytes, so a `read_all`
continues exactly where a three-byte `read_bytes` stopped — which is only
possible because both host handlers already buffer *bytes* below the decoder.
Four std conversions came with it, and they are the whole of `Byte`'s
surface: `to_byte(Int)`/`to_int(Byte)` and `to_bytes(Str)`/`str_of_bytes(List<Byte>) -> Str?`
(strict UTF-8, `None` on invalid bytes) [byte-value].

**`Byte` is now unsigned on both backends** ([kt-byte-unsigned]) — the other
half of the decided deliverable, and a parity fix rather than a
beautification: `Byte` mapped to the JVM's *signed* `Byte`, so the same octet
printed `-1` on Kotlin where Rust's `u8` printed `255`. It maps to `UByte`
now, which interpolates and compares unsigned, and LANGUAGE.md's claim that
Salvo's unsigned `Byte` "is `Byte` in Kotlin" was a spec bug, fixed with it.

**What the deliverable could *not* deliver, and why (a DECISION is now open).**
The second half was to be a specialized `UByteArray`/`Vec<u8>` payload
rendering. Rust needs nothing: `List<Byte>` *is* `Vec<u8>`, unboxed. Kotlin
**cannot have it as a rendering**: a `UByteArray` is not a `List<T>`, so it
cannot reach std's generic list surface on an erased-generics backend —
verified with kotlinc 2.4, which answers `argument type mismatch: actual type
is 'UByteArray', but 'List<T>' was expected` for `sizeOf(ubyteArrayOf(...))`.
So `List<Byte>` stays a boxed `List<UByte>` there, and closing the gap needs
either monomorphization or a distinct non-generic `Bytes` type in std — a
language decision, recorded in ROADMAP.md rather than guessed at. (The
`@ExperimentalUnsignedTypes` worry about `UByteArray` turned out to be moot in
Kotlin 2.4: the probe compiled without an opt-in.)

**`MemFs` was rewritten to store bytes** (`Mut Map<Str, List<Byte>>`, a byte
read cursor, `mem_slice`/`mem_join`/`mem_find_newline` helpers). It had stored
`Str` and converted offsets through `byte_size`, which was correct arithmetic
over a representation that could not hold a non-text file at all — and
`write_bytes` is exactly that file. Two behaviors became *more* faithful as a
result: a ranged open landing mid-codepoint now succeeds (a byte offset has no
characters to land between) and the strict decode afterwards fails, as on the
host; and a decode failure is *recorded*, so `close` reports it a second time
[fs-errors-at-close]. `position` is now the cursor itself.

**One latent Rust-emission defect fixed on the way**: the conversion
intrinsics emitted `({} as T)`, and `as` binds tighter than unary minus, so
`to_byte(-1)` became `-(1 as u8)` — which rustc rejects outright (E0600) and
which would have silently meant the wrong thing on a saturating target. The
lowering now names the source type: `(((x) as i32) as u8)`.

**The example.** `examples/files/` runs the *same* `workflow()` — one-shots,
the `Lines` pass, an append with `position`, a resume through
`open_read_at`, a byte file, a mid-codepoint decode failure, failures
collected through `detach`, cleanup — against the real filesystem under
`RestrictedFs`, and then against `MemFs`, and **prints the two blocks
identically**. That equality is the example's argument: putting the stream
operations on the effect is what lets a double fake reading and writing, so a
test of file code needs no files. A `sandbox_edges()` fn shows what the
restriction refuses (`sub/../probe.txt` through, `../secret.txt` and
`/etc/hosts` refused as `PathEscapes`). It works in `tmp/files-example/` and
removes everything it made.

Tests: **932 passing** (from 931), fresh. The byte cases ride along in the
existing per-backend fs and memfs programs (both extended identically: a
non-text file written and read back, `[0, 255, 200]` on both backends, the
mid-codepoint decode reported twice) and in the string-surface program
(`to_bytes`/`str_of_bytes`, including invalid bytes answering `None`); a new
checker test `bytes_compute_through_int` pins that `Byte` is not
operator-numeric and that the conversions are the way through. All six older
examples were **regenerated** — their checked-in output predated the fs
surface, so they were stale (the outputs still matched; the numbering of
generated helpers had moved).

Rules: LANGUAGE_SPEC.md gained [byte-value] and [fs-bytes], and [fs-double],
[fs-v1-cuts] and [op-arith]'s `Byte` bullet were rewritten;
BACKEND_SPEC.kotlin.md gained [kt-byte-unsigned] (with the `UByteArray`
finding); BACKEND_SPEC.rust.md's [type-basic] gained the cast-parenthesization
rule; LANGUAGE.md's basic-types list and "Files" section carry bytes.

**`MemFs` and `RestrictedFs` — the fs doubles (phase 4 item 6.4,
2026-09-14).** std can now run a filesystem in memory and scope one to a
directory, both in pure Salvo: `core.memfs`'s `MemFs of Fs` (no dependency, no
host — a test registering it touches no disk) and `core.restrictedfs`'s
`RestrictedFs(root: Str) [Fs] of Fs`, the interception customer O-R2 was built
for. Each is its own module, so a program links neither unless it names it
[mod-used-only], and `RestrictedFs` *must* be separate anyway: a dependent
handler switches the program to the fused emission [fs-host-split].

**What they are.** `MemFs` keeps `Mut Map`s of files and open streams and
fakes the whole surface, streams included — which is what putting the stream
operations on the effect bought [fs-double]. Directories are implicit (a path
is a key; a directory exists while something under it does), and a fresh
`MemFs` is empty, because seeding it from a constructor argument would need an
immutable `Map` to become a `Mut Map` and std has no route for that.
`RestrictedFs` rebases every path under its root, refuses an escape
*distinguishably* as `Err PathEscapes`, and resolves `..` right to left so
`a/../b` stays inside while `../b` does not — lexically, and documented as not
symlink-safe [fs-restricted]. Its stream members are pass-throughs: the policy
is in the opens, and a token it forwarded goes back to the handler that minted
it.

**One std addition, forced by §5.5.1's hazard**: `byte_size(str) -> Long`, the
UTF-8 byte count [str-byte-size]. Every offset in the fs surface is in bytes,
so a fake that counted *characters* would let unit tests pass and production
break; `MemFs` slices by characters and converts through `byte_size`, and a
ranged open landing between the bytes of a character answers
`Err InvalidUtf8` exactly as the host's strict decode does. Both backends
report `position: 11` for the same read — the memory case and the real-files
case assert the same number. (`MemFs`'s *representation* was superseded the
same day: the byte deliverable made its files `List<Byte>`, so the conversion
is gone and the mid-codepoint open now succeeds with the decode failing after
it, as on the host — see the bytes entry above.)

**Three defects found on the way, two fixed:**

* **`for k in map` was broken on both backends** — and *silently* on one,
  which is the class [backend-never-wrong] exists to prevent. Salvo iterates a
  map's **keys** (`iter(map)` answers a `MapKeyYield`); Rust emitted
  `for k in map.clone()`, which does not compile (`SalvoMap` is no
  `IntoIterator`), while Kotlin compiled and iterated **entries**, binding
  `a=1` where the program asked for `a`. Both now ask for the keys
  (`.keys().cloned().collect()`, `.keys`), through one helper per backend.
* **`copy(get(list, i)!)` did not compile on Rust**: a call answering a
  *projection* renders as a borrow, so the "a call result is already owned"
  reading in the `copy` lowering pushed an `&String` where a `String` was
  wanted (E0308). It now clones when the argument's type is a projection
  [rs-copy]. Found writing `RestrictedFs`'s path resolver.
* **`size(Str)` disagrees between the backends outside ASCII** — Kotlin's
  UTF-16 units against Rust's code points, so `size("a😀b")` prints 4 and 3.
  Left open with its repro in ROADMAP.md: what a `Str` index *means* is a
  language decision, not a lowering bug, and `byte_size` gave the filesystem
  the one length that cannot drift.

Tests: 931 passing (from 930), fresh. Per backend one compile-and-run case
over a program with **no host handler at all** — `MemFs` written, read,
listed, a ranged open with its byte position, then `RestrictedFs("notes")`
intercepting it inside an `if true` block (inside, through `..`, an escape and
an absolute path) and the unrestricted filesystem answering again after it.
Rules: LANGUAGE_SPEC.md gained [fs-double], [fs-restricted] and
[str-byte-size]; LANGUAGE.md's "Files" section gained both handlers;
ROADMAP.md's S-IO list is down to bytes and an example.

**One overload set: an effect member and a fn of the same name compete
(user decision 2026-09-14, option (a) of three).** A name that is both a
member of an *available* effect and an ordinary fn now resolves as one
overload set, ranked by signature specificity exactly as two fn overloads are
[effect-available]. std is the customer that forced the question: `close` is
an `Fs` member per stream token *and* the `Lines` pass's discharger, so it
shipped as `close_lines` for one afternoon and is `close` again.

**What the rule is.** The chosen effect's members and the fn overloads are
compared as one pool; the more specific signature wins, so a concrete fn beats
a generic member. A tie — both fitting, neither dominating — is an error
naming both remedies (`close@Fs(…)` / `close@module(…)`), never a silent
preference; a call fitting neither side is *one* diagnostic listing both
sides' signatures. `@module` now skips the member set whole, which is how a
shadowed fn is called by hand (before, it was hijacked by the member too —
the bug that made option (c) look worse than it was).

**How it is built, and why the blast radius is small.** Where a name has
candidates on one side only, that side's path runs untouched, so only
colliding names take the new route. There, the arguments are typed **once**
and handed to whichever path wins, which needed three things: a shared
`arg_fits_param` predicate (a call routed by a *different* fit test than the
path applies would report "no overload" for something that fits), a
`fitting_members`/`fitting_fns` pair returning the patterns that were
compared, and `pre_typed` parameters on `check_effect_call` and the fn path so
neither types an argument twice. Recorded cut: a colliding call has no lead
candidate, so a lambda argument needing its parameter type from the position
falls back to [type-unknown-lenient] — it still ran identically on both
backends in the cases tried, but the checker is not proving it. ROADMAP.md
carries it.

**A pre-existing Rust defect fell out of testing it**: a fn-typed *parameter*
whose name matched a program-wide fn was emitted as a module call —
`self::keep(x)` inside `core/seq.rs` when the program declared any
`fn keep(…)`, which does not resolve there. The member branch already
consulted `local_calls` for exactly this hazard; the fn branch now does too.
It was reachable before any of this work (a program with `fn f(…)` broke
`map`), and Kotlin was unaffected because a local shadows a top-level name
there.

Tests: 930 passing (from 923), fresh. Seven checker tests in
`effect_at_tests.rs` — the two sides coexisting, the fn reachable while the
effect is available, a concrete fn beating a generic member, the ambiguous
tie, both selectors, the fits-neither diagnostic, and availability still
deciding first — plus each backend's fs case now calling `close(p)` (the fn)
and `close(s)` (the member) in one program, with the emitted fn asserted in
the golden test. Three golden assertions moved to `close__2`, because a user's
`close` is now the *second* overload of that name program-wide (std declares
the first). Rules: LANGUAGE_SPEC.md [effect-available] rewritten; LANGUAGE.md
"Two effects, one member name" gained the member-versus-fn half.

**The filesystem surface — `core.fs` and `core.hostfs` (phase 4 item 6.3;
user decision on the token shape, 2026-09-14).** std has a filesystem: an
`Fs` effect carrying path *and* stream operations, linear stream tokens, a
linear `FsError`, a `Lines` pass, the one-shots, and a host seam at the
bottom. Both backends compile and run the same program against real files to
byte-identical output, warning-free.

**The decision: tokens are never `Mut`** (option (b) of ROADMAP's open
question). `Mut` is minted at construction and needs `canbe Mut`, so
`open_read` returning a plain `InStream` could not feed a
`read_line(s: Mut InStream)`. The call went the other way instead: the token
is an opaque handle and *all* mutable state — position, buffer, resource —
lives in handler state, where the decided architecture already put it, so
`Mut` claims a mutation that does not happen. `Mut` is now absent from the
whole fs surface: stream members keep their token (`=> s`), `close` consumes
it (`=> !s`), and a token in a field (`Lines`) is read without projecting a
`Mut` out of it [fs-token].

**Two modules, and the split is load-bearing.** `core.fs` is the surface;
`core.hostfs` holds `RawFs`, `platform handler HostRawFs` and
`handler DefaultFs [RawFs] of Fs`. Reachability is name-based, and `core.fs`
declares a `next` and a `to_str`, so ordinary programs drag the surface in —
harmless — while a *dependent handler* switches the whole program to the
fused effect emission, which would have fused every Salvo program ever
compiled. The fusion gate also became reachable-only, in both backends
[fs-host-split].

**Four defects fell out of being the first real customer**, all of them
invisible until now and all fixed here:

* **`ok(None)` did not compile on either backend** [type-none-unit]. `None`
  is one spelling for the absent arm of a `T?` *and* for the sole value of
  the `None` type; the targets spell those differently (`None`/`()`,
  `null`/`Unit`). The checker now records a `NoneUnit` coercion where a
  `None` literal fills a slot whose type *is* `None` — a generic argument the
  call inferred as `None` is the shape — and the emitters render the unit
  value. `Ok None | Err FsError` is six members of `Fs`, so nothing worked
  before this.
* **A Kotlin handler with dependencies could not be constructed from another
  module** [kt-effect-fusion]: its carrier parameter was one of the per-file
  `__Fx_N` classes, so `DefaultFs` in std and the `use` site in the program
  disagreed about a type with the same name. The carrier is a bounded type
  parameter now, as it already was for fns; a generic dependent handler's
  `use` site appends the carrier to its written type arguments.
* **`salvo platform generate` wrote skeletons that did not compile** when a
  member's signature mentioned more than primitives: no `use crate::…`/
  `import` lines for the union wrappers, the declaring module's items, or the
  effect's module. Fixed in both backends — a skeleton that does not compile
  fails at its one job [rs-platform-handler] [kt-platform-handler].
* **std's emitted Kotlin was not warning-free**: narrowed reads of a nested
  union drew `UNCHECKED_CAST` from a site that noted nothing, and concrete
  arms drew `USELESS_CAST` (kotlinc's smart cast had already typed them). One
  annotation now covers both, on any function containing a payload cast
  [kt-suppress-cast].

**One rule changed, and it had to**: **availability decides a bare member
call** [effect-available]. A name that is both an effect member and an
ordinary fn now resolves to the *fn* wherever no instance of the owning
effect is in scope. Without it, std declaring `Fs` claimed `close`, `write`,
`read_line` and `position` program-wide: a program with a `close` of its own
stopped compiling whether or not it touched a file — which is exactly what
happened to a dozen tests the moment `core.fs` existed. The multi-owner case
already read availability this way; this is the single-owner case of the same
rule, recorded for the emitters as `fn_over_member_calls` for the same reason
`local_calls` exists.

**One thing was left open here and answered the same day**: where the effect
*is* available, a fitting free fn did not compete with the member set, so
`close(p: Lines)` could not be called while an `Fs` was in scope. The user
chose one overload set (option (a)) — see the entry above.

Smaller findings, recorded rather than fixed: `rename` is a keyword, so the
member is `rename_path`; `core.fs` is emitted (dead) in programs that never
open a file, until reachability resolves calls through `call_fn`; and a
pass's *type* must be visible for `for` to drive it, which reads as a missing
import of something the value already has. All three are in ROADMAP.md.

Tests: 923 passing (from 920), fresh in 69s. Per backend a golden test of the
emitted layering and **one compile-and-run case over the same program**
(create_dirs → write_str → read_lines → the `Lines` pass → `open_read_at` +
`position` → a missing file, error acknowledged → list_dir → delete), plus
the Kotlin fusion-carrier assertion and the `USELESS_CAST` reversal. Four
test programs were renamed off std's new names (`Fs`, `InStream`, `Lines`),
which is what a fresh std module costs. Rules: LANGUAGE_SPEC.md gained
[fs-surface] [fs-token] [fs-errors-at-close] [fs-host-split] [fs-v1-cuts]
[type-none-unit] and the [effect-available] amendment; LANGUAGE.md gained a
"Files" section; both backend specs carry the fusion, skeleton and cast
rules.

**Linear tokens can be closed by an effect member, and Rust member
parameters follow their clause (S-IO item 6's second prerequisite plus the
gap it exposed — user request 2026-09-14).** Two changes that only matter
together, and the second is why the first is worth having.

**The [linear-group] amendment.** A linear type's discharge set was
same-file consuming *fns*; it is now same-file consuming fns **and effect
members** — phase 4's `close` is an `Fs` member, and `std/core/fs.sv`
declares the token and the effect together. Discharger status attaches to
the **member declaration**, so *every* handler's implementation of a
consuming member is a discharge context where `discard` terminates the
obligation: the real handler, a `MemFs` double in another module, an
interceptor that discharges by forwarding into the handler it wraps. The
same-file rule still binds — it keys on the *effect's* file, so no module can
declare a member that disposes of another module's linear values — and the
context is **per overload**: the `close(InStream)` body may discard an
`InStream` and not an `OutStream`, while a *keeping* member's body may
discard nothing at all. `ModuleScope` gained `effect_files` for the same
reason it has `struct_files`. Both diagnostics were reworded (the
declaration's "no legal death" and `discard`'s refusal now say that a member
counts, and where its bodies are).

**The Rust member-mode fix.** Effect member parameters had followed the
default kept rule regardless of the member's declared deductions — recorded
since 2026-09-04 as sound-but-unoptimized, because the checker had already
consumed the caller's value. Phase 4 made it not merely unoptimized: a
member consuming a **linear token** rendered `&InStream` and the handler
cloned the thing it was supposed to consume. Modes now come from the
member's own written clause (`member_param_mode`) — consumed by value, kept
`Mut` as `&mut T`, kept plain as `&T` — through *one* function, because the
trait method, every handler's impl, the generated `__Impl_H` trait, the
fusion's forwarding impls and the argument rendering at call sites must
agree or rustc refuses. The gap's entry in BACKEND_SPEC.rust.md is replaced
by the rule.

Tests: 920 passing (from 910), fresh. Six checker tests for the amendment (a
member as discharger; a handler in another *file* discharging; a keeping
member refused; a member in another file than the type refused; per-overload
scoping; and a leak whose hint now names the member), plus per backend a
golden test of the emitted modes and two compile-and-run cases: a
consuming-plus-`Mut` pair forwarded through a *dependent* handler (the
hardest agreement case — four renderers), and **phase 4's token shape in
miniature** — `linear struct InStream canbe Mut` and `OutStream`, an
overloaded `close` per token, `read_line`/`write` mutating through them, a
`MemFs` discharging both — printing the same four lines on both backends.
Rules: LANGUAGE_SPEC.md [linear-group] (members join, bodies are contexts),
BACKEND_SPEC.rust.md [rs-effects] (the mode rule), LANGUAGE.md's linearity
section.

**One finding from proving the shape is now an open question, not a
record**: a `Mut` member parameter needs its token declared `canbe Mut`, and
`Mut` is minted at construction (`Mut InStream { … }`), so §5.10.2's plain
`open_read(path) -> Ok InStream | Err FsError` cannot feed
`read_line(s: Mut InStream)`. It is a **DECISION** in ROADMAP.md (S-IO item
6.3, with options and a recommendation) and flagged at FILE_SYSTEM.md
§5.10.2's signature block; it is not restated here.

**Effect members overload within their own effect (S-IO item 6's first
prerequisite, §5.10.2 sub-question A — 2026-09-14).** Phase 4's signed-off
`Fs` declares `close` once per stream token and `position` twice, so the
2026-09-05 within-effect uniqueness rule had to go: a member name may now
recur inside its own effect as an ordinary **overload**
[effect-member-overload], and what stays an error is a *duplicate signature*
— same name, same parameter types, which no call could tell apart
[effect-member-unique]. Signatures are compared as lowered types, so two
spellings of one type are still the duplicate they are.

**Selection is the checker's, in two stages that had to stop competing.**
Which *effect* is chosen first (availability, or `@Effect`), and several
same-named members of one effect are **one candidate** at that stage — the
cross-effect ambiguity error used to fire on them, which was the first thing
to fix. Then the overload is picked by arity, and only if that leaves a
choice by the argument types, ranked with the same `most_specific` order
function overloads use [fn-overload-rank]. Arguments are typed once and the
types handed on, so nothing is checked twice and one mistake still gets one
diagnostic. No overload fitting, or several with none most specific, are
errors naming the member and the argument types — never a guess
[backend-never-wrong].

**The emitted names are one rule in one place** (`salvo-core/src/effects.rs`,
new): every overload after the first is suffixed — `close`, `close__2` — and
`salvo_core::effect_member_name` is called by *both* backends, because five
renderers have to agree on the name (the interface/trait, every handler's
override/impl, Rust's fusion forwarding impls, the `platform generate`
skeletons, and the call sites) and the two backends have to agree with each
other. Rust has no trait-method overloading at all; Kotlin does, and that is
worse — it would resolve by **Kotlin's** type lattice rather than Salvo's,
the [kt-fn-mangling] hazard one level down. Positional suffixes need no
qualifier pass here: every overload but the first is renamed regardless, so
no erasure collision can survive. A handler's implementing member is matched
to its effect member by name and written parameter types
(`effect_member_index`), which is what tells two overloads apart; call sites
read the checker's `effect_member_calls` (the member's index), and an
overloaded name with no recorded resolution is a codegen error.

Found while proving it end to end, recorded rather than fixed: on Rust an
effect member's parameter modes still ignore its *declared* deductions, so a
consuming member (`close(f: InFile) => !f`) takes `&InFile` and the body
clones. Sound — the checker consumed the caller's value, so nothing can
observe the copy — and already noted as a missed optimization under
[rs-effects]; phase 4's tokens are fine with it, since the obligation is a
checker notion and the handler discharges in Salvo.

Tests: 910 passing (from 901), fresh. Six checker tests (overload by argument
type, the `@Effect` selector composing with overloads, arity-only overloads,
no-fitting-overload, duplicate signatures, and the two rewritten pins of the
old rule — `close(Int)`/`close(Str)` is now *legal*, which is exactly the
reversal), plus per backend a golden test of the three emitted name sites and
a compile-and-run case over one shared program (an effect with `close(InFile)`,
`close(OutFile)` and an un-overloaded `describe`, called both bare and through
`@Fs`) printing the same three lines on both. Rules: LANGUAGE_SPEC.md
[effect-member-overload] rewritten (and [effect-member-unique],
[effect-member-call] restated), BACKEND_SPEC.{kotlin,rust}.md gained the
naming rule; LANGUAGE.md's "Two effects, one member name" now covers
overloading within one effect too.

**`platform handler`, both backends (S-IO item 5, FS-1 resolved as O-M2 —
2026-09-14).** The interop surface gains its second declaration: a **host
implementation of an ordinary Salvo effect**, registered with `use` like any
handler [platform-handler]. Where a `platform effect` hands the whole effect
to the host and takes the entry point with it, this hands it one *handler* —
the effect stays Salvo's, with as many other handlers as it likes, and `main`
stays `main` because the instance is constructed *inside* the program. It is
the proposal deferred on 2026-09-05 ("until a need arises"), un-deferred by
phase 4's `HostRawFs of RawFs` sitting under an `Fs` whose other handlers are
all Salvo.

**Where the class comes from is the whole design, and the answer removed
work**: it is a companion in the `platform/` tree of the module that
*declared* the handler — the same tree, loader, mounting and
`salvo platform generate` that platform effects already had. So a `use` emits
`salvo.platform.<M>.H(args)` (Kotlin) or
`crate::platform_<M>::H::new(args)` (Rust), named through the declaring
module rather than the using one, which is what lets std declare a handler a
customer's `main` registers. **std's route is the same route**: std ships its
own host files under `std/platform/…`, and the embedded loader now picks up
the active backend's extension there exactly as it does in a source
directory — so FS-1's "the runtime file" became "std's own companion", and
no backend registry keyed by handler name was needed. Nothing is emitted for
the declaration itself; the interface/trait is the effect's, as always.

Three restrictions, each a consequence rather than a choice: **no body** (no
members, no state — the host class holds both), **no effect dependencies** (a
dependency is supplied to *members*, and these members are host code, which
performs no Salvo effect — the remedy the diagnostic names is an ordinary
Salvo handler in between, which is exactly `handler DefaultFs [RawFs] of Fs`),
and **not generic** (one concrete host class, as for a platform effect). A
`use` whose declaring module has no host companion is a codegen error naming
the command [backend-never-wrong] — the platform-entry error's sibling, with
the `use` as its trigger instead of `main`.

**One pre-existing assumption broke and is fixed**: Kotlin's `entry_hint`
treated *an emitted `platform/…` file* as proof that the host owns `main`. A
platform handler puts a host companion beside a module whose `main` is still
the entry point, so `salvo run` launched the wrong class ("could not find or
load main class"). The evidence is now the generated module declaring
`salvoMain` — the emitter's own marker for the entry having moved — rather
than the presence of customer-written text [kt-platform-handler].

Tests: 901 passing (from 881), fresh. Four parser tests, eight checker tests
in `salvo-core`'s `platform_tests.rs`, and per backend: no-class/host-ctor
emission, the skeleton, the missing-host error, and a compile-and-run case —
one shared program (`HostRawClock` under a `DefaultClock [RawClock] of
Clock`, the phase-4 shape in miniature, which also puts the `use` on the
*fused* path) printing `boot@42` byte-identically on both. Plus a Kotlin test
of std's route (a `platform handler` in a std module, a shipped companion, and
`platform generate` correctly writing nothing for it) and a CLI test walking
the whole arc — run fails naming the `use`, generate, implement, run — on
both backends. Rules: LANGUAGE_SPEC.md [platform-handler] (with
[platform-tree], [backend-companion], [cli-platform] and [platform-effect]
updated around it), BACKEND_SPEC.kotlin.md [kt-platform-handler],
BACKEND_SPEC.rust.md [rs-platform-handler]; LANGUAGE.md gained "A host
implementation of an ordinary effect" under the platform layer.

**std's route was verified end to end by hand**, since it has no shipped
customer yet: a throwaway `std/core/testclock.sv` declaring
`platform handler HostRawTestClock`, with `std/platform/core/testclock.kt`
and `.rs` beside it, ran `now=7` through `salvo run` on both backends before
being deleted again. That is what makes the remaining phase-4 work on this
seam *only* its first customer: `HostRawFs` needs the declaration and the two
shipped files, nothing more. (It also turned up the `include_dir!` gotcha
below — a new file under `std/` is invisible until a `salvo-cli` source is
touched.)

**Handler dependencies become an effect list, and the fusion stops
duplicating itself (user decisions 2026-09-14, from reading the generated
Kotlin).** Four calls, all four built:

1. **The surface**: `handler Stamped [Logger, Clock] of Logger` replaces
   constructor parameters of effect type [effect-handler-deps]. The old form
   made you *name* something you could never use — a member body reaches an
   effect by calling its members — so the names are gone and the list reads
   like a fn's. It also removes the last exception from [effect-not-data]:
   an effect in a data position is now always that error, with no special
   case anywhere. `use` and `Throw` in the list, and a repeated effect, are
   refused where they are written. Parser + AST + checker + both emitters,
   swept across every `.sv` source, test program and spec snippet.
2. **Kotlin stores one fused value per handler instance**, built at the
   `use` site (`class Stamped(private val __fx: __Fx_2)`, registered as
   `Stamped(__Fx_2(__fx.__fx_Clock, __fx.__fx_Logger))`), replacing a
   combiner rebuilt on **every member call**. A handler holds its
   environment for its lifetime, so there was nothing to rebuild.
   **Rust deliberately does not follow**: a stored `&mut` would borrow the
   fusion for the handler's lifetime — the `E0499` the whole strategy exists
   to avoid — and the handler is constructed inside the very struct literal
   that borrows the provider; its per-call `__Deps_H` is one reference, so
   there is nothing to amortize. Stated in both backend specs.
3. **Self-dispatch is a recorded gap, now with an honest diagnostic.** A
   handler member calling another member of its *own* effect used to report
   "no handler for effect `E` in scope (declare it… or `use` a handler)" —
   both remedies impossible inside a member. It now says a handler cannot
   dispatch to itself, that declaring the effect as a dependency would bind
   to the handler registered *before* this one [effect-intercept], and to
   move the shared logic into a fn. ROADMAP.md carries the gap, the repro and
   the DECISION it will force: phase 4's `MemFs.read_all` wants to call its
   own `read_line`.
4. **Fused values are deduplicated.** Kotlin sorts each effect set into a
   canonical order and reuses an identical class (the effects example went
   from 15 fused classes to 9); Rust builds each fusion struct under a
   placeholder name and reuses an identical one, so two fns registering the
   same handler over the same inherited set share one struct and one set of
   impls. Property names derive from the effect instance rather than from
   position, so only constructor-argument order follows the canonical order.

Two things fell out. The Kotlin **fusion gate** was still the old syntactic
test (a ctor param of effect type), so after (1) it silently switched the
whole program out of fusion mode while handlers kept their carriers — caught
by reading the generated code, and the reason both gates are now the same
one-line predicate over the effect list. And a handler whose **dependency
mentions its own generics** (`handler Twice<T> [Store<T>] of Store<T>`) was a
kotlinc "unresolved reference 'T'" leak: a fused class has no type parameters,
so Kotlin now reports it in Salvo's words, which is the cut Rust already
stated. Tests: 881 passing (from 876), fresh; `examples/effects/` rewritten to
the new surface with its generated code regenerated.

**Effect interception, both backends (S-IO item 4, from the phase-4
decisions — 2026-09-14).** A handler may now depend on **the effect it
implements** [effect-intercept], and a `use` may **shadow** an earlier
registration of the same effect instance [use-no-dup] — the two halves of
FILE_SYSTEM.md's O-R2, and what makes `RestrictedFs(root, fs: Fs) of Fs`
writable. The binding rule is the user's from §5.1: a self-dependency binds
**strictly outward**, to the instance in scope *before* this `use`, which
leaves the acyclicity argument [effect-handler-deps] rests on untouched
(every edge still points at an earlier registration) without any new
creation restriction — [handler-not-value] already makes "created where
used" a theorem. Interceptors therefore stack, an interceptor's own member
calls go one layer out rather than recursing, and interception is per
*instance* of a generic effect.

What it cost was small in the checker and almost entirely about **reading
the effect environment as a scope rather than as a set**. The declaration
ban and the duplicate-registration error were deleted; a new
`visible_effects()` (innermost-first, shadowed duplicates hidden) now feeds
every lookup — `use`-dependency resolution, `check_effects_available`,
per-call effect resolution, and `check_effect_call`'s candidate list, where
the dedup is also what keeps a repeat from reading as "ambiguous effect
call" (that error survives for *different* instances of a generic effect,
tested). Dependency resolution already ran before the new instance was
pushed, so the outward binding needed no new code — only its own
diagnostic, since "nothing in scope" now means "nothing to intercept":
*handler `H` intercepts `E` … an intercepting handler wraps the instance
already in scope*.

**Rust** needed one emission rule, verified by hand with rustc first (the
precedent [rs-effect-fusion] has followed since E1): a fusion must carry
**exactly one `__Has_E` impl per effect**, so the inherited accessor for the
instance the new handler shadows is suppressed while the shadowed instance
stays in `__outer`'s provider conjunction — which is precisely what
`__Deps_H{ __p: &mut **__outer }` binds the interceptor's dependency to.
Two impls is `E0119`, and that is exactly what a shadowing `use` produced
before. Everything else — chaining, the adapter, `dyn` in `__outer` — was
already the right shape.

**Kotlin** needed no new machinery (a dependency is a constructor field, so
an interceptor is constructed with the previous fused value's property —
`Loud(__fx4.__fx_Greeter)` — which *is* the outward binding, and the fused
class already carried one property per instance) but it had the real
divergence: its lookups read the environment as a set, so under shadowing
the **outer** handler answered calls the inner one owned. The same program
printed `hello world` twice on Kotlin and `hello world` / `Good day, world.`
on Rust — silently, not as a diagnostic. All five resolution paths now read
an innermost-first `visible_effects()`.

**A pre-existing defect fell out on the way** and is fixed: a *dependent*
handler of a **generic** effect instance whose member parameter substitutes
to a Copy scalar (`handler Reporting(n: Note) of Store<Int>` with
`keep(value: Int)`) emitted a raw rustc `E0308`. The forwarding impl renders
its signature from the *effect's* declaration, where a `T` parameter is
borrowed because nothing is known about a `T`, while `__Impl_H` renders from
the *handler's* declaration, where `Int` passes by value; the forward now
derefs. Loud rather than wrong, so not a [backend-never-wrong] breach — but
interception is what made the shape easy to reach.

Both backends compile and run one shared `INTERCEPTION_DEMO` — stateless and
stateful interceptors, an interceptor over an interceptor, a plain shadowing
`use` with no dependency at all, block-scoped expiry restoring the handler
that was shadowed, and one instance of a generic effect intercepted while
its sibling keeps its handler — to identical stdout. Tests: 876 passing
(from 870), fresh. The rules are LANGUAGE_SPEC.md [effect-intercept] and the
restated [use-no-dup]; LANGUAGE.md gained a "Handlers with dependencies, and
interception" section, which is also where handler dependencies get their
first narrative treatment outside the platform-interop section.

**`examples/effects/` is new** (same day, user request): the effects surface
run *together*, which is where the interesting behavior is. Seven sections —
handler state, several effects in one signature with a subset callee, a
handler depending on another effect, interception (two interceptors stacking,
one of them depending on a second effect as well), shadowing shown as
distinct from wrapping, one member name on two effects with the `@` selector,
and two instances of one generic effect — plus a "composition root" note,
since `main` is the only function in the program that names a handler. It
also documents the two lowerings, because effects are the feature whose
generated code is least obvious: Kotlin's interception is object references
(`Stamped(__fx.__fx_Logger, __fx.__fx_Clock)`), Rust's is the chained fusion
with exactly one `__Has_Logger` impl per struct. Filling the last obvious hole
in `examples/` — effects had no worked example at all — and it needed no
compiler change to write, which was the point of writing it.

**Operator typing and the `@Effect` selector, built (2026-09-14, from the
phase-4 decisions).** The two work items between the fusion milestone and the
platform handler, closing both former due-before DECISIONs as *code*.

**Operator typing** ([op-arith] [op-order] [op-bool] [op-promote]
[op-convert] [lit-adopt]): arithmetic and unary `-` are numeric-only (`Int`,
`Long`, `Float`, `Double` — `Byte` deliberately excluded until its `UByte`
lowering lands, since it is signed on one backend and unsigned on the other
today); `Str +` is refused pointing at `${}` interpolation; `&&`/`||`/`!`
take `Bool` in value position (conditions already did, and report once, not
twice); ordering works on numerics and `canbe ordered` structs and refuses
the rest — including `Double` *operands* being fine (IEEE partial comparison
agrees on both backends) while sorted-collection keys stay refused (no total
order), a deliberate split. Mixed widths widen within a class with the
promotion recorded in a new `Checked::promotions` table: Kotlin's operator
set covers the mixes natively, Rust casts — `((n * 2) as i64)`, where the
inner parentheses were the session's near-miss (below). Cross-class mixes
error naming std's new explicit conversions: twelve `to_int`/`to_long`/
`to_float`/`to_double` intrinsics whose truncating semantics were verified
pairwise (Kotlin `toX()` ≡ Rust `as`: saturating float→int, low-32-bits
`Long`→`Int`). Unsuffixed literals **adopt** the expected numeric type
(`let x: Long = 1`, optionals through the sole value arm; suffixed literals
and variables never adopt), replacing LANGUAGE.md's "no implicit widenings —
write `1L`" rule; both emitters render adopted literals at their checked
type (`1i64`/`1L`, `3f64`/`3.0`), since Kotlin refuses a bare `1` for a
`Long` *parameter*. Found while testing, pre-existing, now an open defect in
ROADMAP.md: whole-valued `Double` interpolation prints `2` on Rust and `2.0`
on Kotlin.

**The `@Effect` selector** ([effect-at] [effect-member-overload], revising
[effect-member-unique]): member names recur across effects — `close` on `Fs`
and on `Net` — with `Symbols::effect_of_fn` and the scope member table now
multimaps and the 2026-09-05 program-wide ban reduced to per-effect
uniqueness. A bare call resolves through the one candidate effect with a
handler *available*; none or several is an error naming the selector:
`close@Fs(h)`, dot form `h.close@Net()`, parsed by case (capitalized after
`@` is an effect, lowercase a module path [fn-overload-at]) into a new
`Expr::EffectScoped` node. The call's type arguments keep their
[effect-disambiguation] meaning under the selector
(`next_random@Random<Int>()` — no new AST field needed, the generic-call
parse already carries them). A selected member is a call form, not a value.
Emission is erased on both backends: the checker records the resolved effect
per call (`effect_calls`), and the emitters' name-keyed fallback only
answers for sole owners. Known leftover: the LSP def-site table is
name-keyed, so go-to-definition on a *shared* member name lands on the last
collected declaration.

Tests: 870 passing (from 849) — a new `op_tests.rs` (nine checker tests), a
new `effect_at_tests.rs` (seven), parser tests for the selector grammar,
rewritten pins of the two replaced rules (`operators_drop_mut`, the two
shared-member-ban tests), and paired e2e cases compiling and running
`Long`-arithmetic/conversion and shared-member programs to identical stdout
on both backends.

**Has-accessor effect fusion, both backends (user decision 2026-09-14, built
the same day).** The user proposed the design during the phase-4 decision
rounds (below) and pulled it forward as its own milestone, delivered before
the filesystem work: the value fused calls thread does **not** implement the
effect traits/interfaces — it implements one generated *accessor* per effect
(`__Has_Console { fn __get_Console(&mut self) -> &mut dyn Console }` on Rust,
`interface __Has_Random_Int { val __fx_Random_Int: Random<Int> }` on Kotlin),
and member calls go through the accessor. What it buys: effect members can
never collide on a fused value (the `close@Fs` vs `close@Net` future), two
instances of a generic effect disambiguate structurally (Rust: the generic
Has trait's turbofish; Kotlin: per-instance interfaces — which **retires
[kt-effect-facets] unbuilt**, since the instance now lands in the
interface/property name instead of mangled member names), and the fn-ABI is
uniform — a *single* effect fuses too (user steering call: consistency over
the `&mut dyn E` special case). The gate is unchanged and now shared: any
handler declaring a dependency switches the whole program on either backend;
plain-mode output is untouched. Kotlin's side is the "uniformity fusion"
[kt-effect-fusion] had recorded as planned, now real: multi-bounded generics
(`fun<__Fx> f(__fx: __Fx) where __Fx : __Has_A`), flat-rebuilt fused classes
per `use`, per-instance Has interfaces merged program-wide into `fx.kt`.
Rust's side is the [rs-effect-fusion] reshape: Has traits emitted beside each
effect (generic, so cross-module identity rides the existing globs),
provider traits over Has supertraits for the one nameable `__outer`/fn-value
type, accessor-then-method dispatch replacing UFCS-on-the-effect, and the
`__Deps_H` adapter made unconditional. The dyn boundaries — platform `main`
(host ABI) and fn values (aliasing/borrow limits) — keep per-effect
parameters and open with a generated combiner. Every Rust shape was
rustc-verified before the emitter learned it (including two instances of one
generic effect inherited through a single `dyn` provider — supertrait
elaboration carries the UFCS bound, which made a feared cut unnecessary).
Both backends compile and run every fusion program end-to-end; suite
849/849, fresh. Full mechanism: BACKEND_SPEC.rust.md [rs-effect-fusion],
BACKEND_SPEC.kotlin.md [kt-effect-fusion].

**Phase 4 (the filesystem) fully decided (user decisions 2026-09-14, six
rounds).** FILE_SYSTEM.md carries the complete option record and the decided
architecture; the calls, in brief: **effects may intercept** — a handler may
depend on the effect it implements, binding strictly outward, with shadowing
allowed and [use-no-dup] reduced to the duplicate-instance error (O-R2);
**stream ops are effect members** (O-P1, after two reversals — free fns lost
to the `[Fs, RawFs]` double declaration their test seam required), with
**bare member names disambiguated by `@Effect` syntax** (`read_line@Fs(s)`,
generics in the effect signature when ambiguous — this *made* the ROADMAP
"shared member names" due-before decision); **linearity stays above the
platform** — `RawFs` (plain `Long` handles, `platform handler HostRawFs`,
FS-1 resolved as O-M2) sits under an `Fs` whose handlers are all Salvo
(`DefaultFs(raw: RawFs)`, `MemFs`, `RestrictedFs(root, fs: Fs)`), so linear
tokens are minted and discharged only in checked code; **errors** are O-L2
(`linear struct FsError { kind: FsErrorKind }`, `ignore`/`detach`
dischargers, errors-at-close on both stream sides, 8-arm taxonomy);
**buffered, per-operation Str/Byte streams** with `open_read_at` ranged
opens, byte-count-returning writes, `position` on both stream types, `Byte`
→ Kotlin `UByte`, bytes in v1; **restriction** is lexical, rebased,
distinguishable (`PathEscapes`), root as `Str`; and the **operator-typing**
due-before DECISION is closed: numeric-only arithmetic (no `Str +`),
`Bool`-only logicals, implicit widening within integer types, explicit
int↔float conversion, literals adopting the expected numeric type, promoted
result types. The [linear-group] discharge set gains consuming effect
members declared in the type's file (entailed by O-P1 + member dischargers).
FILE_SYSTEM.md remains the plan of record until phase 4 lands, then retires
into this log as OBLIGATIONS.md did.

**Mixed variadic spreads, and qualifier overloading (user decisions
2026-09-13).** Two follow-ups the user asked for after C-6, each of which
turned out to be an unimplemented case rather than a target limitation.

**`list_of(first, ...rest)` works.** The user's question was whether something
in Rust prevented it; nothing did. Kotlin had always accepted it — its spread
is an operator on an argument (`listOf(first, *rest)`) — while Rust wants the
tail as one `Vec<T>`, and that assembly was simply never written: the ordinary
path refused the mixture outright and the *intrinsic* path had no guard at all,
so it read the spread as the whole tail and silently dropped the leading
elements. Both paths now assemble the tail in **written order** (`push` each
plain element, `extend` from each spread), which also allows a spread anywhere
in the tail rather than only last. A constructor lowering is told which shape
it received — a new `Spread { None, Borrowed, Owned }` replaces the `bool` —
because an assembled vector is fresh and must not be cloned again while a
borrowed forward must be; `owned_vec()`/`owned_iter()` helpers keep the seven
storing lowerings honest, and `mut_str`'s read-only tail keeps borrowing. std's
`non_empty_list` went back to being ordinary Salvo (`return list_of(first,
...rest)`), its two intrinsic lowerings deleted.

**A qualifier name may now be declared over several subject types**, and the
subject decides which one a use means — the way a fn overload is decided by its
arguments. This was the DECISION C-6 left open, and it is what `NonEmpty` over
the containers needed. The subject is keyed **syntactically, by the `of` type's
base name**, because resolution, the [mod-collision] checks, the refinement
matcher and both backends all need the same answer and only the checker can
unify; a generic `of` (`qualifier Ok<T> of T`) has no base name, accepts
everything, and so still collides with its namesakes. Same name *and* same
subject stays a **replacement**, not an overload — which is what lets a module
shadow std's `NonEmpty of List<T>`, and a plain `push` into the new overload
set broke exactly that until the insert became a subject-aware upsert.

Three layers had to learn the subject. `ModuleScope.qualifiers` and
`Symbols.qualifiers` became overload sets; the checker gained
`qualifier_for(name, subject)` beside `qualifier_named(name)` for the questions
that are about the *name* (existence, body, provenance, `with`-compatibility —
`with` names a qualifier, so same-named declarations share it); the `is` path
resolves each named qualifier against its subject **once**, up front, and every
later question reads that. `AddCtx::admit` needed the subject in its provenance
key too — without it the second core module declaring `NonEmpty` was silently
refused, which is why the first attempt found only the `List` one. And a use
whose subject *no* declaration accepts is now the same error as a single
inapplicable declaration rather than silence.

**The Rust backend needed real work**, which [qual-erasure] had predicted:
qualifiers are erased and Rust has no overloading, so two same-named
`qualifies` functions both emitted `Q_qualifies` and collided with E0428. The
subject's base name now disambiguates (`Filled__List_qualifies`) and only when
the name is actually overloaded, following [rs-fn-mangling]'s
mangle-only-on-collision precedent; the predicate call site resolves the same
declaration from the subject in hand, so the two agree by construction. Kotlin
needed nothing — the JVM overloads on the parameter type.

**What std gained**: `core.nonempty`, declaring `NonEmpty` over `Set`, `Map`,
`SortedSet` and `SortedMap` beside `core.list`'s `of List<T>`, with the
refinements that establish it (`add` for the sets, `put` for the maps) and the
overloads that drop the optional — `min`/`max` on a `NonEmpty SortedSet`,
`first_key`/`last_key` on a `NonEmpty SortedMap`. They live in a module of
their own for a mechanical reason worth remembering: an accessor overload has
to **delegate** to the plain version, a scope selector names a module
(`min@core.sorted`), and declared next to their targets the selector would
re-pick the overload — a `NonEmpty` argument still ranks it first — and recurse
forever. `core.list`'s `first` dodges that by delegating to `get` instead;
`min` has no such alternative.

That change also made an existing test *better*: the sorted-collections demo
did `add(s, …)` and then tested `min(s) is Str`, and with the refinement in
place `min` answers with an element, so the test now reads the value directly.

**And a third fix the user asked for next**: a user-declared variadic of a
*primitive* element type had been broken on Kotlin, and the two directions I
had recorded turned out to be one — `vararg ns: Int` is an `IntArray`, and
Kotlin has **no** way to write a `vararg` of boxed `Int` (verified against
kotlinc), so both directions were really "stop using `vararg`", differing only
in scope. Taken uniformly: a variadic parameter is now an ordinary `Array<T>`
parameter on that backend [kt-variadic], with the call site building the array
— and since `arrayOf` takes Kotlin's own spread, a plain tail, a lone spread
and a mixture all build the same way. `vararg` bought nothing, both sides of
every call being generated, and it cost a representation split the Rust backend
never had (it has always passed a `Vec<T>`). Reference and generic element types
were unaffected, which is why std never hit this.
Writing the test for it exposed **another** instance of the E0382 class: the
*ordinary* call path moved a lone spread of a place into the callee's owned
variadic parameter, so `f(...rest)` followed by any use of `rest` was a raw
rustc error. The intrinsic path had learned to clone with the sorted
collections; this one had not.

**Tests**: 849 (from 845). Three qualifier-overloading tests with locally
declared qualifiers (the checker harness builds its own prelude, so std's own
declarations are asserted end to end instead), `a_mixed_variadic_tail_is_
assembled_once` on the emitted shape, and mixed-spread plus container-claim
cases in each backend's compile-and-run registry, sharing source and expected
stdout verbatim. Specs: `[qual-overload]` in LANGUAGE_SPEC.md, a
`[fn-variadic]` sub-bullet for mixed tails, `[rs-fn-mangling]` extended in
BACKEND_SPEC.rust.md, and LANGUAGE.md's Collections section. **Recorded, not
fixed**: a user-declared variadic of a *primitive* element type breaks on
Kotlin (`vararg ns: Int` is an `IntArray`, not an `Array<Int>`), found while
testing mixed spread but reproducible without one.

**S-Col C-6 built, and the array generator deleted (user decisions
2026-09-13).** Two things the user asked for after the collections work
landed: delete the broken `Int[n] { i -> … }` form, and give std the qualifier
surface over containers — plus `add_sorted`, which was the user's own
addition ("inserts the element in such a way that it preserves the sorted
qualifier").

**The generator form is gone**, node and all. Deleting `Expr::ArrayInit` from
the AST first and letting `cargo build` report every `E0599` found all 20
sites with no grepping — parser, desugar, checker (4), deduce, lends, reach,
and both emitters (3 and 4). `array_by(n, init)` already said the same thing
through the ordinary intrinsic path, which is why deleting beat fixing; the
spelling now fails to parse with "`Int` is a type, not a value". The corpus
file it appeared in also had a stale array *literal*, fixed in passing.

**Three claims over `List<T>`.** `NonEmpty` (in `core.list`) has a
`qualifies`, so it can be tested — and it is the one that earns the machinery:
`first(list: NonEmpty List<T>) -> proj[from: list] T` drops the optional.
It arrives three ways: by construction (`non_empty_list`), by *refinement*
(`refn add(...) => list: +NonEmpty`, since `add` may not promise it itself),
and by `is`. `Sorted` (also `core.list`) has **no** `qualifies` — deciding
whether a list happens to be sorted compares its elements, which nothing can
do over an unconstrained `T` — so it is minted by `sort`/`mut_sort` only;
`add_sorted` inserts at the order-preserving position and names `Sorted` in
its **own** exhaustive deduction list, which it may do because it genuinely
knows the claim survives, and `binary_search` is honest only because its
parameter carries the claim. `Distinct` lives in **`core.set`**, not
`core.list`: a constructor must sit beside its qualifier
[qual-ctor-same-file], and a set is what can honestly promise it, so
`to_list(set)` mints it. Both backends answer the **lowest** index within an
equal run (explicit lower bounds — Rust `partition_point` plus an equality
test rather than `Vec::binary_search`, Kotlin an `indexOfFirst` over
`__salvoCompare`), and `sort` on Kotlin uses Salvo's comparator, never natural
ordering, because JVM `String.compareTo` is UTF-16 code-unit order where
Rust's is code points.

**No language change was needed**, which was the surprise. The plan had been
to ask for `canbe ordered` on a type parameter; but the compiler already
dispatches key eligibility on std *names* (`Set`, `SortedMap`, …), so adding
the `Sorted` **qualifier** to that same mechanism is consistent rather than
new coupling. The check fires on written claims (`check_sorted_list_claim` in
`validate_type`) and on inferred ones — the latter needed
`resolve_named_call` to re-apply `decl.constructs`, because a constructor's
`as Q` lives *beside* the return type rather than in it, so `sort`'s claim was
invisible there.

**Five bugs fixed on the way, all pre-existing.** (1) A `refinement` inside
any `core` module could never match: `add_items` ran twice for a core module's
own files (`Level::Core` *and* `Level::Own`), putting every fn in the overload
set twice, which ordinary resolution tolerated and the refn matcher — which
counts matches — did not. (2) `list_of(first, ...rest)` **silently emitted
wrong Rust**: the ordinary variadic path had always refused a mixed
plain-plus-spread call, but the intrinsic path had no such guard and its
lowerings read the spread as the whole variadic, so only `first.clone()` came
out. Now refused on both paths, and recorded as a feature gap — it is
LANGUAGE.md's own spelling for `non_empty_list`, which is therefore an
`intrinsic` with a native lowering per backend. (3) `=>[f]` rejected a
`once`-qualified fn-typed parameter, locking the contract out of exactly the
parameter that most wants one: a callback that *consumes* what it is given can
only be called once. (4) A projected return over a bare generic
(`-> proj[from: list] T`) rendered as `T` rather than `&T` on Rust — the
documented "borrowedness is the instantiation's fact" exception, which is
right at a definition site but wrong in a return, where elision ties the
borrow to a named parameter. (5) A diagnostic hardcoded "discharge it with its
`close`" where the value's discharger was named something else.

**One design consequence, accepted.** std's `refn add` means a *user*
qualifier that also refines `add` over a `List` now disagrees with std's, so
neither applies and the call warns. The remedy is one word in the user's own
code — `with NonEmpty` — which the diagnostic names, and four tests carrying a
Q1/Q2 conflict demo needed exactly that edit.

**Two new examples' worth of ground truth.** `examples/linearity/` is new:
the obligation and its discharge, that it moves, that a keeping call borrows
instead, that it cannot hide in a composite, and a generic carrying one
(`<T canbe linear>` with a `once` consuming callback). Writing it forced a
retraction — a first draft asserted L5 partial moves (move a field out, keep
reading a sibling, whole-value use refused) and **none of that reproduces**,
neither for a `Str` field nor a `List<Int>` one; the section was rewritten
around what is true. `examples/collections/` gained a section on the three
claims. **Tests**: 843 (from 841) — a prelude-safe checker test plus a
compile-and-run case per backend, sharing source and expected stdout
*verbatim* (asserted identical), since ordering and the equal-run answer are
exactly where the two could drift. Specs: `[col-nonempty]`,
`[col-sorted-list]`, `[col-distinct]` in LANGUAGE_SPEC.md, a `[fn-contract]`
sub-bullet for the qualifier-group fix, LANGUAGE.md's Collections and
constructor sections (its `non_empty_list` example had a body that no longer
compiles), and `[type-array]` in both LANGUAGE_SPEC.md and
BACKEND_SPEC.rust.md. **Left for the user in ROADMAP.md**: whether two
qualifiers may share a name over different subject types, which is what
`NonEmpty` over `Set`/`Map`/the sorted pair needs.

**S-Col built: collections, in six increments (2026-09-12/13, the same day
they were decided).** Every decision in the round below this entry is now in
the language, on both backends, and COLLECTIONS.md is deleted per its charter
(its reasoning — the language survey, the rejected options, the option space —
is summarized in that entry). Three pre-existing bugs fell out on the way and
were fixed; three more were found and recorded. What it took, by increment:

**(A) The rename sweep.** `list` → `list_of`, `mutable_list` →
`mut_list_of`, `mutable_str` → `mut_str` across std, every corpus and inline
test source, the examples and their checked-in output, and the specs; four
parser snapshots re-accepted. No compatibility shims (AGENTS.md's
backwards-compatibility invariant), so this is simply what the functions are
called now.

**(B) `Set` and `Map`.** Two intrinsic types, a Rust runtime file
(`collections.rs`: `SalvoMap`/`SalvoSet`, insertion-ordered to match
LinkedHashMap exactly — position kept on overwrite, O(1) order-preserving
remove), and key eligibility checked where the type is *instantiated*
(`key_ineligible`/`sorted_key_ineligible` in check.rs). Kotlin gets
`LinkedHashMap`/`LinkedHashSet` for free, which is why they were the model.

**(B2) Iteration, and C-7 answered by prototype rather than on paper.** A
Set/Map pass is a **snapshot owning a `List<T>`**, yielding owned elements
through a set-local `snapshot_at` intrinsic. Two designs were built first and
failed, which is what the prototype was for: an intrinsic *borrowing* pass
([proj-field] lifetimes over the native iterators) breaks because the emitter
renders `proj[from: p] T` as plain `T` for a non-borrowing struct, so the
yield type mismatches; and a `copy()`-based pass breaks because Kotlin cannot
copy a generic `V`. **A Map pass yields keys**, not entries — an entries pass
needs an owned `(K, V)`, and with no generic copy on Kotlin the pair would
share identity with the stored value and alias mutable values on one backend
only, a parity break. Python's `for k in d` is the precedent. `values` and
`entries` passes are recorded in ROADMAP.md.

**(C) Collection literals, and array literals removed.** `[1, 2, 3]` is a
List, `{"a"}` a Set, `{"k": "v"}` a Map, `Mut` prefixing like a struct
literal, empty literals typed by expectation or an error. The parser
speculates with snapshot/rollback (`parse_brace_collection`,
`brace_is_struct_lit`): **`{}` is always an empty collection**, never an empty
bare struct literal — `Finished {}` (named) still works. Arrays keep their
constructors (`array_of`/`array_by`) and lose their literal.

**(D) Universal `==`, and the two opt-ins.** Structs compare structurally on
same-base-type operands with qualifiers ignored — state, provenance and `Mut`
alike, so `Surname Person == Person` — and a fn-typed field bars a struct from
`==` (and so from either opt-in). `canbe hashed` and `canbe ordered` are
declaration-site opt-ins validated where they are written
(`hash_ineligible`/`order_ineligible`, with the error saying which field and
why); Rust adds derives, Kotlin gets generated `equals`/`hashCode`/
`compareTo` plus a `compare.kt` runtime (`__salvoCompare` for lists and
tuples). **Float equality is Salvo-emitted** on both backends rather than
delegated — IEEE today, deliberately chosen as the seam for the
precision-specified comparison the user asked for later — which also closed a
real divergence: `nan == nan` used to disagree between the backends. Float
fields bar `canbe hashed`.

**(E) `SortedSet` and `SortedMap`.** Separate types, not a qualifier. String
ordering had to be fixed to code-point order to make the two backends agree.

**(F) The remaining conventions.** `*_by(size, i -> value)` constructors,
`to_set`, and both `to_map` forms (a `List<(K, V)>` and a
`List<T>` + entry function), duplicate keys last-wins. Container equality fell
out of (D) for free and is order-insensitive on both backends for Set and Map
— the one C-10 item that had been punted to the operator-typing decision.

**Bugs fixed on the way, all pre-existing.** (1) A tuple-array type
`(Str, Int)[]` failed to parse — "expected `->` after effect list in function
type", the parser having taken `[]` for an effect list. (2) An **effect member
hijacked a same-named fn-typed local**: the emitters resolved calls through
the program-wide `symbols.effect_of_fn`, so `std/core/seq.sv` reported "no
handler for effect `Sink`" for a call to a local. Fixed with
`Checked::local_calls` plus tightened member visibility in resolve.rs — the
checker knows which it is, and the emitters now ask. (3) **Variadic arguments
moved out of their caller's locals**, so `list_of(a); use(a)` failed with a
raw rustc E0382. Fixed by cloning in the emitter rather than by tracking the
move in the checker: tracking it would have been more precise and would have
turned working programs into errors.

**Recorded, not fixed** (all in ROADMAP.md with repros): a bare inline
collection literal does not determine a callee's type parameter
(`to_set([1, 2])` cannot infer `T`; binding it first works) — probably one fix
with the recorded bare-generic-struct-literal gap; and the
`Int[3] { i: Int -> … }` **array-generator form is broken on Rust** (yields
`()`, and splices the enclosing handler into the closure), which
`array_by(3, i -> …)` now expresses properly — the recommendation is to delete
the form, a user decision.

**The accepted cost.** Name-based reachability now pulls `core.set`,
`core.map` and `collections.rs` into every program, because the constructors
and converters name each other across modules: hello-world went from ~300 to
569 lines of allowed dead code. Both backends tolerate unused items, and the
real fix — walking the checker's *resolved* call targets instead of matching
names — needs the checker to record them, so it is a separate pass over
`reach.rs` (ROADMAP.md).

**Tests**: 841, all green (from 807). New: `collection_tests.rs` (26) and an
`examples/collections/` worked example — chosen to show the *decisions* rather
than the mechanics (the three literal forms and what a brace means where it is
ambiguous, the two orderings, what may be a key and how a struct opts in,
equality everywhere versus ordering only where declared) — byte-identical
output on both backends. All ten golden snapshots re-accepted; every example
regenerated. Specs: `[col-literal]`, `[col-equality]`,
`[col-hashed-ordered]`, `[col-sorted]`, `[col-by]`, `[col-convert]` in
LANGUAGE_SPEC.md; `[kt-float-eq]`, `[kt-ordered]` in BACKEND_SPEC.kotlin.md;
LANGUAGE.md, BACKEND_SPEC.rust.md and `examples/README.md` updated. A
label audit at the end closed four spec gaps: `[col-insertion-order]` and
`[col-to-str]` were referenced from 30-odd sites in code and had no rule, so
both are now written (the second says the `to_str` format is the language's,
not the target's — Rust `Debug` quotes map keys and Kotlin writes `a=1`); the
two backend specs gained the **runtime module** sections
`[rs-runtime-source]`/`[rs-collections]` and `[kt-runtime-source]` they had
been missing; and a dangling `[fn-value-ty]` in check.rs was dropped, its
sentence already citing `[fn-contract]`. Every hyphenated label in code now
resolves to a rule. Two *test* lists were also stale — the runtime-module
lists that make `runtime_tests.rs` complete rather than a sample did not
include `collections.rs` or `compare.kt`, so neither was being compiled on
its own; both are now listed and both compile warning-free standalone.
README's feature list gained a Collections bullet (its front-door sample was
re-run on both backends and still prints correctly). **Left
undecided and moved to ROADMAP.md**: C-6, the std qualifier surface over
containers (`NonEmpty`, `Sorted of List` + `binary_search`, `Distinct`) —
std authorship only, since the machinery it needs is already built and
tested — and `Deque<T>` as the next container when a customer appears.

**Design pass: collections decided in outline; the filesystem option space
laid out; recursive types investigated (user decisions 2026-09-12, evening
into 2026-09-13).** A documentation-only session (run read-only alongside
the phase-3 build, made writable at the end) that produced three artifacts.
**(1) FILE_SYSTEM.md** — the phase-4 option space in OBLIGATIONS.md style:
nine decisions (FS-1…FS-9) with recommendations, from a survey of Java
classic/nio, Kotlin/Okio, Rust, Go, Python and the capability systems
(WASI, cap-std, Deno). Notable findings: the user's restricted-handler
sketch collides with [effect-handler-deps]'s self-dependency ban and
[use-no-dup] (no effect shadowing), so the recommended shape is two-effect
layering (`RawFs` + `Fs`, both public handlers depending on the raw seam);
and the FS-3 recommendation (stream ops as free fns on linear stream
values, object-capability style) mostly defuses the shared-member-name
blocker. Decisions not yet made; the document dies into this log when they
are. **(2) COLLECTIONS.md** (since deleted per its charter — built the same
day; see the entry above this one) — the collections option space, then
*decided*
across three same-day rounds (all user): separate `Set`/`Map`/`SortedSet`/
`SortedMap` types — the `Sorted`-qualifier idea was examined and rejected
because a qualifier is droppable by design and sortedness changes behavior;
insertion-ordered iteration on both backends with LinkedHashMap's exact
semantics (position kept on `put`-overwrite, O(1) order-preserving
`remove`; Rust ships a linked ordmap/ordset runtime file); keys by
intrinsic ordering only, structs opting in via `canbe hashed` /
`canbe ordered` validated at the declaration, unions hashable-never-
orderable, lists/tuples orderable via Kotlin runtime comparators; universal
struct `==`/`!=` (same base type, qualifiers ignored — equality is on the
data at check time; fn-typed fields bar it; float equality Salvo-emitted on
both backends, deliberately, as the future hook for precision-specified
comparison; float fields bar hashing, revisit later); collection literals
`[…]`/`{…}`/`{k: v}` as sugar for the `*_of` constructors, array literals
removed, empty literals typed by expectation or error, `Mut` prefixing like
struct literals; `Mut T[]` (element-assignable arrays) added; the
`*_of`/`mut_*`/`*_by`/`to_*` conventions with `list`→`list_of`,
`mutable_list`→`mut_list_of`, `mutable_str`→`mut_str` renames; duplicate
keys last-wins. This settles the `==`/`!=` slice of the operator-typing
DECISION (recorded in ROADMAP.md); one design item stayed open (C-7, the
Set/Map pass) and was answered by prototype in the build entry above.
S-Col rode before phase 4; FS-6's
in-memory test filesystem is its first customer. **(3) Recursive types**
— probed against the evening's debug binary: no rule anywhere admits or
refuses them; Kotlin compiles and runs a recursive struct while Rust dies
downstream with E0072 and no Salvo diagnostic (recorded as an open defect
with repro, ROADMAP.md), while recursion through `List<T>` already works
end to end on both backends. The write-up (ROADMAP.md "Recursive types",
unscheduled at the end of the queue by user decision) splits the work into
the cycle diagnostic (owed regardless) and the Rust boxing rule, with the
two DECISION points (constructibility, depth semantics) and the
refuse-together rules (non-regular recursion; recursion + linearity).

**A `proj Mut` parameter errors at the declaration (user decision
2026-09-12, late evening).** Follow-up to [proj-readonly]: a parameter
written with a *top-level* `proj Mut X` was accepted at the declaration but
unsatisfiable by any projection — the `Mut` makes it a `Mut` position, so
every projection argument was refused at the call site with "cannot mutate"
(and the body could never use the permission either, the parameter being a
projection to it). The user's call: report it at the declaration instead.
`check_fn` now refuses the spelling per parameter (`top_level_proj_mut` in
check.rs), naming both remedies — drop the `Mut` (a `proj X` position
accepts `proj Mut X` arguments, `Mut` being droppable) or drop the `proj`.
Top-level only: `Mut List<proj Mut Str>` (a view's real element type),
locals, fields and returns keep the type legal. The same walk-through
re-confirmed *why* kept-aware argument acceptance ([proj-type]) is sound in
Rust: a kept non-`Mut` parameter is `&T` in the emitted code whatever the
Salvo spelling ([rs-borrows]), so a top-level projection into it is a plain
reborrow — verified by compiling and running the probe under rustc. Test:
`a_proj_mut_parameter_is_refused_at_the_declaration` (widen_tests); spec:
[proj-readonly] sub-bullet in LANGUAGE_SPEC.md, one sentence in
LANGUAGE.md's read-only rule. Nothing in std/examples wrote the spelling.
The same review reworded the over-broad prose (user-approved): LANGUAGE.md's
"passing a projection where an owned value is expected is an error" and the
[proj-type] subtyping bullet now state the kept-aware rule the checker
actually implements — a top-level projection fits a kept, non-`Mut`
position even written owned (kept is `&T` in Rust either way [rs-borrows]);
the refusals are exactly `ProjBlock::{Consumes, Mutates, Nested}`, where
the representations genuinely clash.
The inventory's per-crate figures were re-measured with `cargo test -p`
while updating the count (806 → 807): salvo-core 409 → 428, salvo-syntax
74 → 76, salvo-backend-rust 128 → 131 — stale since earlier same-day
entries updated only the header total.

**Phase 3 built (2026-09-12, same day as decided).** Every step of the
plan landed green, and the phase is closed. What it took, by step:
**(1) Keywords**: `proj`/`once`/`linear` reserved in the lexer;
`type_ref_name`, the type-atom loop, `parse_is_check`, deduction entries
and `canbe` lists accept them; internal qualifier strings lowercased
repo-wide; six std parser snapshots re-accepted; the VS Code grammar
regenerated. **(2) O-C2**: the union/nullable composite refusals deleted;
`owes_linear` made flow-sensitive with a `linear_settled` flow flag
(set where a narrow lands a linear union on a non-linear arm, surviving
the branch narrow-restore like consumption, joined across paths, applied
to the if-without-else fall-through from the else-narrows); the
fallible-open shape e2e on both backends. **(3+4) The discharge model**:
`linear struct` in the AST/parser; `has_auto_linear` reads the modifier;
`discharge_set` scans the declaring file (a new `struct_files` map in
resolve); `own_discharges` replaces the `close` designation and its
silent exemption — a discharger terminates on every path, `discard`
gated to dischargers as the terminal, forwarding legal; the
no-discharger error at the declaration; leak diagnostics enumerate the
set (`stop`/`join`); the `params Linear` group deleted from std;
**no implicit discharge sites** — the loop close-splice deleted from the
checker and both emitters (`PassDriver.close` gone), `drives_in_place`
extended to every named place (driving an exhausted pass again is now
legal, zero iterations), linear temporaries refused as loop subjects,
and the consuming-callback pattern (`drain(h, close)` / `drain(c,
drop)`) replacing `?Linear<It>`, with std's `drop` in `core.seq` (in
`basic` it made every `Int` user import the module — the multi-module
golden caught it). **(5) Conditional containers**: struct generics take
`canbe` (`StructDecl.generic_canbe`); `ty_own_linear` computes
containment per instantiation (depth-guarded); concrete linear fields
require the marker with the error naming it; the literal-store,
type-argument and `var_in_composite` refusals all learned the opt-in;
`linear_capable` gates dischargers and the declaration check;
**settle-by-decomposition** (all linear-reaching fields moved out — the
`projection_move` gate widened so linear projections are real moves
regardless of mutability, since the immutable-free latitude would
duplicate obligations); the wrapper pass (lazy `take` over a linear
source) e2e on both backends. **(6) D6**: the position gate deleted;
`once T <: T` on non-fn bases in both `is_subtype` and `unify` (fn types
keep never-drop — the `run_plain` refusal stands). **(7) The smuggling
hole**: `check_effect_call` applies the instantiation ban to members'
own generics, opt-in respected. **(8) Generic fn values instantiate**
(the close-out the user asked for): `fn_value_by_name` unifies a generic
candidate against the position's expected fn type and substitutes, with
the linear ban applied there too — `drain(c, drop)` works, `drain(h,
drop)` is refused, e2e on both backends. Tests: +23 checker/parse tests,
4 new e2e cases per backend pair, and every rewritten expectation.
Leftovers in ROADMAP ("Linear types"): the generic wrapper-pass loop
emission gap, bare generic struct literal inference, intrinsic
containers. OBLIGATIONS.md deleted per its charter.

**Phase 3 decided: obligations, lowercase (user decisions 2026-09-12,
evening).** The full OBLIGATIONS.md option space was walked and every call
made — the recommendations accepted, with the user's own respelling and a
simpler discharge model replacing §1-1/§3-1/§3-2:

- **Obligations are lowercase keywords**: `proj`, `once`, `linear` —
  reserved words, written in qualifier position but visually distinct from
  qualifiers (`proj NonEmpty List<T>`), signalling compiler-owned behavior
  the way `provenance` does. `proj[from: p]` keeps its source list. Bounds
  follow: `T canbe linear`, `q canbe once` stays available for the future
  multiplicity-polymorphism reserve; `canbe Mut` (a permission) stays
  uppercase.
- **`linear struct FileHandle`** replaces `: Linear<self>`, and the
  `params Linear<It>` group loses its designated meaning (deleted — no
  compiler-known group, no designated `close`). Hover shows linearity from
  the declaration and, for containers, from the instantiation.
- **Discharge = same-file consumption + `discard`.** Any fn declared in
  the same file as a linear type that consumes a parameter of it is a
  discharger; at least one must exist (checked at the declaration — "no
  legal death" otherwise). `discard(x)` on a linear value is legal *only*
  inside such fns, and inside them the obligation must terminate on all
  paths — `discard` as the terminal, or forwarding into another consuming
  fn (`shutdown(t) { stop(t) }`). This subsumes O-D1's `by stop, join`
  lists (alternatives are just two same-file consuming fns), the cache's
  context parameters (ordinary parameters), and generic discharges, with
  zero new syntax at the type.
- **No implicit discharge sites anywhere.** The `for`-loop's close
  splicing is removed: a linear pass must be let-bound, the loop drives
  it kept/in-place, and the ordinary all-paths analysis forces the
  explicit discharge after the loop and before early exits; an inline
  linear-pass subject (`for l in open_lines(…)`) is refused like any
  view of a temporary. Generic combinators that consume a pass take a
  consuming-callback parameter; std gains
  `fn drop<T>(value: T) -> None => !value` so non-linear callers have a
  uniform thing to pass (`close` for a handle, `drop` for a `Str`); a
  linear argument to `drop` is refused by the existing instantiation ban
  (no `canbe linear` bound).
- **O-C2 accepted**: written linear union arms are legal; a value of
  `Ok InputStream | Err Str` owes while un-narrowed; narrowing to a
  non-linear arm discharges; `T?` falls out. Unblocks phase 4's
  fallible-open shape.
- **Conditional containers**: `struct Box<T canbe linear>` with `T`
  reaching a field is conditionally linear — the written bound is the
  declaration, no extra marker — and needs a same-file discharger generic
  over the same bound. A **concrete** linear field
  (`struct Session { file: InputStream }`) requires the `linear struct`
  marker; unmarked stays refused. Intrinsic containers (`List<linear T>`)
  stay deferred.
- **D6 yes** (`once` on any type; position gates removed), **D7 no
  surface** (covered by conditional containers; revisit on a customer),
  **O-C4 skipped** (unforced), typestate/session-types boundary kept.

Build order: keyword/rename sweep → O-C2 → same-file discharge model →
splice removal + `drop` → conditional containers → D6 → the
effect-member-generics smuggling hole → spec sweep, with OBLIGATIONS.md §6's
acceptance shapes (plus a lambda-view-of-linear sibling) as tests on both
backends before the phase closes. OBLIGATIONS.md is scaffolding and is
deleted when the phase lands.

**Capturing lambdas are views; written lend entries win (user decision
2026-09-12, evening).** Walking the projection leftovers surfaced two
gaps, decided and closed together. (1) *Written-entry precedence*
([proj-infer]): a bodied generic's `=> proj[from: it]` was consulted only
behind a written-return-type gate that an instantiation-borne projection
(`-> Mut List<T>`, `T = proj Str`) never passed, so the every-kept-argument
fallback linked the result to arguments the author had excluded — the fix
is one reorder in `lends.rs::lends_of` (declared entries before the
`holds_proj` gate), and the entry now narrows the link while still flowing
through a temporary pass to its roots. (2) *[lambda-view]*: the user's
`pick_list` example — `indices.map(i -> list.get(i)!)` — disproved the
claim that a callback's projections always derive from its arguments: a
body can root a projection in a **capture**, which no fn type can name,
and naming the lambda (`let f = i -> get(words, i)!`) evaded fate
entirely — `eat(words)` was accepted with the view live (the Rust emitter
masked it with a silent clone, and then failed on an unrelated cast bug).
The decision: a lambda holds borrows of every non-Copy *read* capture,
like a struct holds its `proj` fields — binding links it (held, borrowed,
transitively through the capture's own links), results link through it, a
capture-free lambda holds nothing (`map(p, w -> w)` binds freely), a
lambda expression is never a "temporary" a view could dangle from, a
consumed capture stays owned (the `once` case — the first cut of the
implementation wrongly linked those and broke `run_once(g)`), and Copy
scalars link nothing. Emitter halves, closing both open defects: an
unwrap whose checker type is `proj` stays the reference (no clone — the
lambda tail returns `&String` into a `Vec<&String>`, and as a bonus an
interpolated `first(names)!` stopped cloning in the demo golden), and a
Ref-bound Copy scalar renders as a value (`*i`) in intrinsic arguments,
fixing `(i) as usize` (E0606). Tests: `a_capturing_lambda_is_a_view_of_
its_captures` and `a_written_proj_entry_narrows_the_instantiation_link`
(analyze), `rustc_compiles_and_runs_a_capture_rooted_projection` (with
the emitted-lambda shape pinned) and the `pick-list` Kotlin case —
byte-identical output. The map-links-to-every-container over-linking
remains, softened by the entry precedence; the type-mention-tracing
refinement is recorded in ROADMAP.

**`proj` in the type (option A, user decision 2026-09-12).** Two phase-2b
cuts had one cause: `proj` lived only on fate links and was stripped from
the lowered `Ty`, so two values of one Salvo type could have two Rust
representations — a borrowed non-Copy union arm handed to a position
written as the owned union (Rust codegen error, Kotlin ran it), and a
generic body storing an element of an opaque pass (`Vec<&String>` under a
`List<Str>` type; rustc loud, Kotlin silently permissive). Chosen over a
call-site borrowness check and over a body-only rule: the projection is
part of the type. **Rules** ([proj-type]): `X <: proj X`, never the
reverse; `proj` is never-drop (passing a projection to an owned position
is an error naming the type and both remedies — annotate or `copy`, with
the blocker distinguished: `ProjBlock::{Mutates, Consumes, Nested}`);
`proj` on a Copy scalar erases; unification treats a pattern's `proj` as
optional and binds the base minus the matched qualifiers (`proj T` vs
`Mut Str` gives `T = Mut Str`); overload assignability is kept-aware, and
an owned position **outranks** a projected one as its own inverted
specificity dimension (replacing "`proj` never affects overloading");
generic bindings show it (`map` over a container pass is a
`Mut List<proj Str>`), with instantiation-driven linking: a substituted
return holding `proj` with no inferable lend links the result to every
kept argument, held. std: `copy<T>(value: proj T) -> T` (un-projects one
level, keeps the rest — `copy` of a `proj Mut Str` is a `Mut Str`);
`ArrayYield.items: proj (T[])`. **Rust emitter**: one rendering rule —
`proj X` is `&X` at any depth, under the named lifetime when one is in
scope (`'s` in borrowing structs and their `next`, `'a` on tied returns)
— with one carve-out found by the e2e suite: `proj T` over a *bare
generic at its definition site* renders owned `T` (the generic body is
uniform; the instantiation carries the borrow via the turbofish retag,
which now defers to types already carrying `proj`). The old AST-based
`&`-adding sites (`Option<&T>`, union arms, `&T` returns) now just render
the written type under the lifetime. **What closed**: both cuts are
regression tests on both backends (`proj-arm-param` e2e cases; the
checker halves in `analyze_tests` — an owned parameter refusing a
borrowed union arm, and the combinator-view set: `Mut List<proj Str>`
named in the consume refusal, fate poisoning through the temporary pass
to the container, a generator instantiation staying free, and `copy`'s
un-projection observed through a type mismatch), plus the `spec_cmp`
ladder rung. Two diagnostics-expectation updates (widen_tests earlier,
`l7c_derived_returns` now) moved to the new type-level messages. One
question surfaced during the closing run and left as a **DECISION** in
ROADMAP: instantiation linking holds *fn-typed* arguments too, so
`let kept = map(p, w -> w)` is refused as a view of a temporary (the
lambda), and the idiomatic spelling needs a named callback.

**Test-running discipline (user decisions 2026-09-12).** Three calls, all
encoded in AGENTS.md's "Build, test, verify": (1) *Runner split* — `cargo
test` stays the default for warm runs (one process per test binary
amortizes ~700 spawns: 4.6s vs nextest's 9.3s warm) and
`SALVO_E2E_FRESH=1 cargo nextest run --no-fail-fast` is the full/fresh
pre-commit check (cross-binary scheduling wins fresh: 48.6s vs 52s, plus
per-test timings and no first-failing-binary blind spot). (2) *Fix before
re-running* — a small number of failures is fixed completely and only then
re-run, never one suite run per fix; a large number is grouped by root
cause and fixed in batches. (3) *Watch the clock* — test runs are timed
against the documented budget, and overshoots are flagged to the user
rather than silently waited out; every slow-run incident so far had a
diagnosable cause.

**Test-suite speed: batched kotlinc, no doctest passes, disk-cached probe
(2026-09-12).** The suite had drifted to ~54s warm / ~2min fresh, and the
diagnosis found four independent causes. (1) *Doctest collection*: six
library crates each paid ~7s of uncached rustdoc per `cargo test` to find
zero doctests — `[lib] doctest = false` in every library crate (delete it if
a doc example should ever run). (2) *The kotlinc probe*: `kotlinc -version`
starts a JVM (~1.5s); the per-process `OnceLock` cache meant once per test
binary under `cargo test` and once per *test* under nextest — now
disk-cached next to the stamps, keyed on the resolved executable
(canonical path + size + mtime) so upgrading kotlinc re-probes; `rustc`
deliberately stays process-cached (50ms probe, and rustup's shim does not
change on `rustup update`, so a disk key on it could serve a stale
version into every stamp). (3) *One kotlinc per test*: 67 compile-and-run
tests each paid the ~2.5s JVM+compiler startup — ~800 CPU-seconds fresh.
They are now data: each is a fn returning a `KotlinCase`, listed in
`KOTLIN_CASES`, and one driver test (`kotlinc_compiles_and_runs_every_case`)
compiles every stamp-missing case in a few parallel batched kotlinc
invocations — each case's generated code rewritten into its own package
namespace (`k_<tag>.salvo…`), since every program declares
`salvo.main.MainKt` — then runs the programs in parallel and stamps each
case individually with the same keys as before. Content assertions inside
case builders still run without kotlinc (cases are built before the
toolchain gate). (4) *Stale debug objects*: macOS `split-debuginfo=unpacked`
keeps every `.rcgu.o` next to the binary and cargo never collects the sets
orphaned by rebuilds — 790k files / 48.7 GiB had accumulated, slowing
everything that touched `deps/`; `salvo_testkit::prune_stale_debug_objects`
deletes objects whose owning artifact is gone (current binaries stay
debuggable), run as a hygiene test in `salvo-testkit` on every full suite
run. Measured after: warm `cargo test` 54s → ~4.5s; fresh 777 tests ~50s
(was ~127s under nextest). The remaining fresh cost is the CLI suites
(~16s max single test, both-backend runs through the `salvo` binary) and
rust codegen — batching those is open in ROADMAP. Also learned: recurring
`SIGKILL (signal 9)` on freshly built test binaries is AMFI (macOS
code-signing) rejecting a stale kernel signature cache — see gotchas.

**Deductions respelled: the `=>` clause (user proposal and decisions
2026-09-11, late).** The bracket list after `->` is gone; a signature ends in
`-> T => entries`, on the same line or the next. Entries: `p` (kept), `!p`
(consumed; `p: Nothing` says the same), `p: Qual` (exhaustive), `p: None`
(strips every qualifier — was `[p:]`), `p: -Q` (delta), `.f: proj[from: a]`
(the result's field projects `a`), `v.f: proj[from: a]` (a parameter's field is
re-pointed), bare `proj[from: a, b]` (opaque: the result holds a borrow of
both), and `=>[f] …` groups scoping entries to a fn-typed parameter (its own
parameters named in its type; inline lists inside a parameter list are a parse
error — ambiguous with the next parameter, user chose the one clean option).
**Unmentioned means inferred**: a clause is partial and most fns write none;
a bodiless declaration (effect member, platform member, intrinsic) must
mention every parameter except Copy scalars [copy-scalar-free]; fn types keep
today's default. The contract-stability cost — a body edit can change what
callers may do with no signature change — was raised, accepted, and recorded
in ROADMAP as a revisit. `proj[from: a, b]` names several sources at once
and is what a projection joined across branches reads as. **What it took:**
a `FatArrow` token; `Deduction { target: Param{name, path} | Result{path} |
Opaque, kind: … | proj(sources) }` with `TypeRef.from: Vec<Ident>`;
`parse_deduction_clause`/`parse_deduction_entry` (fn-type groups attach to
the parameter's `Type::Fn`); `deduce.rs` running the fixpoint for *every*
bodied fn and overlaying the written entries (`ParamDeduction.written`),
validating only what was written, and treating `return x` under a wholesale
`proj[from: x]` as a borrow; `check.rs` preferring the effective table
(`effective_contract`), per-parameter `own_written` for move-mode claims,
`require_full_clause` replacing the whole-list requirement, multi-source
`derived_calls`, and re-pointing entries adding held links at the call; the
LSP hover rendering the effective clause (`!p`, `p: Q`, `proj[from: …]`,
`=>[f] …`; Copy scalars and keep-all entries omitted); the Rust emitter
tagging every source with `'a`, borrowing the RHS of a `proj`-field
assignment, and tying `'r` for re-pointing. The sweep was mechanical
(a throwaway script, then a pass restoring the consumptions the old
exhaustive lists implied on bodied test helpers — `consume(x) {}` infers
*kept*, which the tests did not mean): std, corpus, examples (regenerated),
every inline test source, LANGUAGE.md/LANGUAGE_SPEC.md/README. Std shrank:
intrinsics keep their full clauses, bodied fns mostly lost theirs. **Tests**:
842 (from 828), all green with `--no-fail-fast`. **Gotcha found on the
way**: plain `cargo test` stops at the first failing test *binary*, so "one
failure left" can hide others — check with `--no-fail-fast`.

**Phase 2b "copies only by opt-in" — built, and the view model settled (user
decisions 2026-09-11).** The five steps from ROADMAP landed: `[rs-opt-borrow]`
(the `Option<&T>` local), `get → (proj[from: list] T)?` and std's `next →
Emitted (proj[from: p] T) | Finished` [yield-proj], `filter → Mut List<proj
T>` as a view, `?copy` as a general implicit on `filter_to` and on handler
constructors [copy-implicit] (the former [implicit-fn-only] restriction on
constructors lifted), and the std audit (no element-storing combinator clones
silently; `Str` operations produce new strings by nature). Along the way
three questions were settled by the user: (1) **`proj Mut X` is legal but
read-only** [proj-readonly] — `Mut X <: proj Mut X`, never the reverse, `copy`
the way out — which is what refuses `take(s, 2)` on `s = get(ps, 0)!` while
`take(p, 2)` on a constructed `p = iter(xs)` advances; a view is an **owned
object holding borrows**, so `iter` returns a plain `Mut ListYield<T>`, no
`proj` on the whole; (2) **which parameters a result holds borrows of is
inferred** [proj-infer] (`lends.rs`: per field, through calls, locals, `!`,
branches; conservative fallback to every kept parameter where there is no
body), with the explicit form for fn types and effect members — the
principle the user stated for the language: "infer as much as it can for
free, and let the user state what inference cannot reach"; (3) **any struct
may hold `proj` fields** [proj-field], written without a source, replacing
the pass-only exemption that first landed (the option-(2) review item is
closed by it). Two soundness rules came from rustc catching what the checker
had not: **a view of a temporary** may be used within its statement but not
bound, returned or stored [proj-anywhere] (`let p = slice(list(1, 2))` is
E0716), and a `Yield<self, proj T>` obligation must agree with its `next`.
**`iter fn` passes borrow their subject** (user decision): `__subject: proj
Subject`, `proj` snapshot fields, nothing copied at the mint, a snapshot
written with `copy(...)` in a `state` initializer — which fixed the
self-referential generated struct rustc had refused and **lifted the Kotlin
generic-`iter fn` cut** (nothing copies a `T` any more; e2e on both backends).
Rust: borrowing structs carry `'s` (transitively through owned view fields),
`[rs-proj-lends]` ties `'a`/`'c`, `[rs-proj-arm]` retags the element generic
and peels one reference in callbacks and adapters, borrowed-arm payload reads
bind as `Ref`, a borrowed Copy arm flowing into an owned position is adapted
arm by arm (non-Copy is a codegen error, not a hidden clone). **Option
explored and rejected**: struct-level *link parameters* (`Pair<T, U, a, b>`)
as the required spelling — the user preferred inference with the clause as
fallback; the form is reserved in ROADMAP for per-field precision on
bodiless callees. **Documented gap**: a generic body cannot see that an
element of an opaque pass is borrowed (`filter`'s `add(out, x)`), so the
signature says it (`=> proj[from: it]`); a user fn storing such an element
without writing so is caught by rustc, not the checker.

**`ReadOnly` is renamed `proj`, and copying becomes opt-in-only (user decisions
2026-09-11).** Found while making `get` return a borrow: std hides two copies
the language would never let user code make silently — `filter` on a list
clones every kept element, `get` clones every read — and the attempt exposed
that fixing them needs a design, not a patch (the first try failed 41 tests;
what it took is in ROADMAP "Copies only by opt-in"). The decisions: the
provenance qualifier `ReadOnly` becomes **`proj`** everywhere and
**user-writable** in any type position except a struct field; `[from: p]` stays
**attached** to each occurrence (a detached form was rejected — a tuple
borrowing from two sources has no single "the return borrows from p");
`List<proj T>` is a *view* and `(proj T)?` an optional borrow, each spelling
being exactly its Rust representation; and **`?copy: (T) -> T`** becomes a
general implicit, the `?to_str` mechanism reused, which is what lifts Kotlin's
generic-`copy` refusal. The rename landed at once (17 files, no compatibility
form); the internal place-step enum also called `proj` became `Step`. A live
emitter bug was found on the way — `narrow_unwrap` clones an `Option<&T>` as
`&&T` — and is step one of the plan. Only the sequencing relative to phase 3 is
still open.

**Seven requested items: two silent-failure defects, a warning, an operator
family, and three hover gaps (user requests 2026-09-11).** All landed
together; the two defects are the important part, because both were the class
AGENTS.md calls worst — a Salvo program the checker accepted and the *target*
compiler rejected.

**[ident-resolve] — an undeclared name was not an error.** The `Expr::Ident`
arm fell through to `Ty::Unknown` when a name was neither a local, a handler,
nor a callable; the emitters then spelled it verbatim, so rustc reported
`E0425` and kotlinc "unresolved reference". Now an error, which also covers
use-before-declaration (scopes are not hoisted). A name that *is* declared but
is not a value says which it is — struct type, effect, qualifier, `params`
group, type — following the courtesy [effect-not-a-type] already extended. The
whole suite passed unchanged, which is the evidence that nothing legitimate had
been relying on the silence.

**[interp-to-str] — interpolation was unchecked.** `"${list}"` and `"${p}"`
reached rustc as "doesn't implement `Display`". Now: scalars, `Str` and unions
of those render natively; anything else resolves a `to_str` **at the
interpolation site** (user decision — the group was explicitly *not* to be the
mechanism), recorded in `Checked::interp_to_str` for the emitters. std gained
`intrinsic fn to_str<T>(list: List<T>)` rendering `[1, 2, 3]`, and
`params ToStr<T>` exists purely as declaration-site validation, per the same
decision. Two corrections to the original sketch fell out of building it:
`Mut Str` needed **no** intrinsic (the existing [str-drop-mut] coercion already
converts a builder — verified running), and a union of renderable arms had to
be *allowed* — the first version rejected an un-narrowed `Ok Str | Err Str`,
which a test caught, and both backends do render it (Kotlin through the
wrapper's `.value`, Rust through the arm accessor). Recorded limitation: a
generic `List<T>` cannot interpolate, since an opaque `T` has no text form;
composing an element `?to_str` is blocked because `resolve_implicit_fn` skips
candidates that take implicits and `implicit_args` is keyed by call spans,
which an interpolation has none of.

**[interp-struct] — structs interpolate by default.** With no `to_str` of its
own, a struct whose every field is natively renderable renders as
`Person { name: ann, age: 3 }`. The format is the *language's* deliberately:
Rust's `Debug` prints `name: "ann"` and a Kotlin data class prints
`Person(name=ann)`, so deferring to either target would have made one program
print two different things [backend-parity]. An explicit `to_str` wins; a field
that itself needs one is not followed.

**[unused-var] — a warning, with `_` to opt out.** Only *reads* count, so a
write-only variable warns too; parameters, handler state and std are exempt.
Its side effect was the instructive part: **~14 test helpers conflated errors
and warnings**, because `Checked::errors` holds both severities and nothing had
previously warned routinely. They were filtered to `is_error()` — except
`refine_tests`' helper, deliberately named `diagnostics`, which *needs*
warnings; filtering it broke a test and the filter was reverted. Worth knowing
before the next warning is added.

**[inc-dec] — all four step forms.** Only `i++` existed. Rather than add three
near-identical AST variants, `Expr::PostIncrement` became
`Expr::IncDec { down, prefix }`: about twenty consumers treated the old node
uniformly, so a mechanical rename covered them and only three sites needed real
logic. The checker ignores both new fields on purpose — all four forms do the
same thing to the operand, and only the *value* differs. Kotlin renders them
directly; Rust, having neither operator, emits compound assignment in statement
position and a block in value position. All eight combinations were run on both
backends and print identically.

**Three hover gaps.** A `params` group had no hover at all (`DeclAt` had no
variant for it). A predicate qualifier now shows the *condition* it holds under
when its `qualifies` is a single `return <expression>` — the expression alone
[doc-qualifies-body]. That is narrower than the five-line rule first built:
the user cut it the same day, on the grounds that a one-line predicate is the
rule while a longer body is an implementation the reader did not ask for, and
the qualifier's own doc comment is the place for that. And **fate links** were already shown
at a *use* — the gap was the **declaration**, where a reader looks first:
`fate_reads` was keyed only by read spans, so `declare_var` now records it too,
after the move-mode decision so a binding that took ownership honestly shows no
links. It also names the projection (`p.name`, not `p`), which L5 made
necessary — saying "shares fate with `p`" would overstate the link. Hovering a
name inside an `import` line works now as well.

Hover on a **fn** also names the module its resolved overload came from
[lsp-fn-origin], which with scope-ladder overloading is load-bearing: the
signature alone does not say whether `size` was std's, an import's or the
module's own. Other declarations get the same section only when they come from
another file.

Two more name positions were reported the same day and fixed
[lsp-name-positions]: a struct's **obligation clause** (`: Linear<self>`) and
the type or qualifier name in an **`is` check** (`i is Positive`, which used
to fall through to the enclosing expression and report only its `Bool`). Both
were missing `def_refs` records; `parse_check` turned out to be the single
site where every `is`-check name is classified, so one call covers type and
qualifier checks alike.

Tests: 822 (797 at the start of the batch). Examples regenerated: the only diff
is `use crate::core_string::*;` in `core/list.rs` and its Kotlin twin, because
`to_str` returns `Str`.

**L5: fields are tracked apart (phase 2, 2026-09-10).** Shared fate poisoned
at whole-variable granularity: mutating any part of a value invalidated
everything derived from any other part. The roadmap had this as "a refinement
with no current use case — revisit only if whole-variable poison proves too
coarse in practice", so the first step was to test that claim, and it did not
survive: **four out of four** ordinary field-disjoint programs were rejected —
read `p.name` while mutating `p.tags`, hold `q.left` while mutating `q.right`,
read a field while handing a *different* field to a mutating fn, and assign to
a disjoint field. Each demanded `copy`, which is a real clone on the Rust
backend. That evidence is what justified building it.

**What it took: less than "the largest analysis change".** The rule is one
overlap test. A `FateLink` gained the projection path out of its root
(`let n = p.name` → `[.name]`, `[]` for a whole variable, `None` for a
derivation that is not a projection chain), a poison event gained the path it
hit, and `poison_derived` fires only where the two overlap — `Place::overlaps`,
the relation P1 built for *narrowing*, reused without modification. The
sequencing decision (P1 before L5) paid off exactly as it was argued it would.
Transitive links compose their paths (`let p = q.inner` then `let n = p.name`
links `n` to `q` at `[.inner, .name]`), and `fate_mutation_through` already
computed the `Place` it needed for narrowing invalidation, so the mutation path
was free.

**Where precision stops, deliberately.** Overlap keeps every case that could
name the same storage: the same projection, a prefix in either direction
(mutating `o.inner.tags` still poisons a value from `o.inner`), the whole
variable (a `Mut` argument, a reassignment, `++` — `[]` is a prefix of
everything), and a computed index, since `proj::Element` may-aliases any
element. An unknown path overlaps everything. All four are tested as *reject*
cases alongside the three accept cases, because the risk in a precision change
is silently losing a rule.

**It needed no emitter change, and that is the interesting part.** The Rust
backend already emits a borrow-mode binding from a pure place as a real borrow,
and rustc permits that borrow to live across a `&mut` of a *disjoint field of
the same local* — so `let mut n = &p.name;` held across `p.tags.push(…)` is
accepted by rustc for the same reason Salvo now accepts it. The two analyses
draw the same line, and the newly legal programs compile **clone-free**. Kotlin
was already aliasing. Verified by running the four shapes on both backends with
identical stdout.

**What is left, recorded rather than built**: the *move* half. A move-mode
binding of a projection still consumes its whole owner, so taking `p.tags` out
leaves `p` unusable rather than leaving `p.name` readable — Salvo stricter than
rustc here. That is the piece that genuinely needs the place lattice
(partial-move state per root, merged across branches and back edges); the poison
half needed none of it, since a link already names its projection. Nothing in
std or the examples wants it. See ROADMAP "L5".

Tests: 797 (was 788). Seven checker tests (three accept, four reject — each
accept verified to fail without the change), plus a field-disjoint program run
end to end on both backends, the Rust one asserting the borrow shape rather
than trusting it. LANGUAGE.md's "whole-variable granularity" bullet is replaced
by the new rule; `[fate-field-disjoint]` is the label.

**L5's move half: partial moves (2026-09-10, same day).** Built immediately
after, on the user's call, once Rust's model settled the question the poison
half had left open. `[fate-partial-move]`.

**The question first, because it shaped the work.** The open item was what a
*deduction list* says when a body moves one field out of a parameter — and the
answer is that it says nothing, because the situation never crosses a call
boundary. Rust's rule is all-or-nothing per parameter: a **borrowed** parameter
refuses a move out of it entirely (E0507), and an **owned** one may be
partially moved because the caller has already surrendered the whole value and
can never observe the partial state. Partial-move tracking is therefore purely
function-local, and the return channel is the only way to hand part of something
back. Checking Salvo against that found it *already* implemented both halves —
a written kept deduction refuses the move ("cannot move mutable data out of
`p`: it is a kept parameter"), an inferred one claims the parameter as moved —
so the binary kept/moved deduction needed no change and no new qualifier. A
place-parameterized deduction (`[p: -tags]`) was considered and rejected: it is
viral (every caller tracking which fields survived, merged across branches),
and `f(p.tags)` — passing the field rather than the struct — says the same
thing with no notation and is strictly more precise. `ReadOnly[from: p]` is a
different axis: it describes the *return* channel, where the borrowed place
escapes and the caller must know which argument it came from.

**What it took.** A `moved_places: Vec<MovedPlace>` per local, sitting beside
`place_narrows` and carried by the same snapshot/restore/merge machinery — the
second time that substrate paid for itself in one day. A move of a *proper*
projection records the path instead of setting the root to `Nothing` (both
routes: the consuming-call path in `projection_move` and the move-mode binding
path in `apply_binding_mode`). Reads check the place against the record, and the
merge is **union** — moved on any path is moved — the dual of `place_narrows`,
which intersects. Reassignment drops every record the assigned place covers, so
`p.tags = …` revives that field and `p = …` revives everything.

**Two things the implementation turned up.** A read of a projection *base* is
not a use of the whole value, so the whole-value refusal had to be suppressed
inside `p.name`'s `p` — a `projection_base` depth counter, with the enclosing
projection doing the precise check. And an assignment *target* is a write, not a
read: without an `assign_target` flag, `p.tags = …` reported reading the very
field it was putting back.

**Emission needed no change here either**, and rustc does the work: a moved
projection already rendered as a raw place (`eat(p.tags);`) through
`moved_projections`, the sibling read still clones from the surviving field, and
a revival is rustc's reinitialization of a moved field. Verified by compiling
and running the three shapes on both backends with identical stdout.

**One existing test changed meaning, deliberately.** `s2_move_mode_ancestors_are_consumed`
asserted the old whole-variable diagnostic for a projection move
(``h` cannot be used here: it was consumed…`). The program is still rejected —
`add(h.tags, 9)` after `wrap(h.tags)` reads a field that left, which is the
parity divergence that test exists to pin — but the diagnostic now names the
field. The assertion was updated to the more precise message; nothing was
weakened.

Tests: 806 (was 797). Seven more checker tests (six verified to fail without
the change; the seventh, the kept-parameter refusal, holds either way and is
there to pin Rust's rule), plus a partial-move program run end to end on both
backends, the Rust one asserting the raw-place move and the live sibling read.
LANGUAGE.md gained the worked example; the previous entry's "what is left"
paragraph is superseded by this one.

**Phase 1 closes: the riding-along polish, and option (e)'s residue
(2026-09-10).** Four small items, done together as the last of "Finish the
iterators"; no language-design calls, all under decisions already made.

- **[kt-suppress-cast]** — emitted Kotlin functions whose bodies read a union
  payload through an erased cast (a generic loop element, a `when`-arm or
  `is` binding, a `^` widening bind) now carry `@Suppress("UNCHECKED_CAST")`:
  the emitter notes warning-worthy casts (a generic parameter, or any
  parameterized type — concrete casts are run-time checked and stay bare) as
  it renders them, and `emit_fn_inner` emits the body before assembling the
  signature so the annotation can go on the function. std's `seq.kt` compiles
  warning-free again (the baseline drew five warnings). New rule in
  BACKEND_SPEC.kotlin.md.
- **The same-name-pass collision is fixed by the ladder, not by nominal
  identity.** `resolve_implicit_fn` now prefers the most specific scope rung
  among fitting candidates before declaring ambiguity — the same
  [fn-overload-scope] rule every named call walks — so a program declaring its
  own `ListYield` plus `next` resolves the `?Yield` spread to its own `next`
  instead of erroring. Consistent with the flat name-keyed scope (the user's
  struct declaration already wins the name in their file); true
  module-qualified nominal identity remains future resolver work.
- **The stale `Iter<T>` comments are gone** — more than the roadmap's three:
  check.rs ([iter-protocol] table doc, the [implicit-infer] example now
  spelled with `Yield`/`next`, both [fn-effects] claim docs), types.rs (the
  `Qual.effect` doc, the `once`-widen example, and `once_position`, where the
  sweep found **live dead code**: a `name == "Iter"` arm that would have given
  a user struct named `Iter` the `once` position without opt-in — removed),
  the Rust emitter (an orphaned `[rs-iter-pass]` flag comment), and
  BACKEND_SPEC.rust.md, whose output-layout section still promised the
  deleted `iter.rs` runtime.
- **Option (e) closed: the per-field snapshot is program-wide.** The `iter fn`
  expansion moved from parse time to a program-level pass in the CLI and LSP
  (`parse_module_deferred` + `desugar::expand_iter_fns_with`, local
  declarations winning a name), so a subject declared in *another file* now
  gets the per-field snapshot instead of the whole-value fallback — verified
  end to end with a two-file program whose `__Pass_Countdown` holds
  `{from, at}` rather than a cloned struct with its `Str`. `parse_module`
  keeps its single-file behavior for direct consumers, and a survivor is
  loud: resolve reports an unexpanded `iter fn` as an internal integration
  error rather than checking it as an ordinary fn. Generic subjects and
  assignment-through keep the recorded fallback.

Fallout worth noting: regenerating the examples turned up **stale checked-in
output in `examples/qualifiers`** (a `u1_mut` accessor and `next__N`
renumbering) that pre-dated this work — verified by regenerating with the
pre-change compiler — now refreshed alongside the `@Suppress` lines. Tests:
788 (was 784) — the suppression demo, the collision repro (fails without the
one-line fix, with the exact roadmap message), the cross-file snapshot
assertion, and the loud-guard test.

**Regions designed: an effect with an intrinsic handler, regional by default
(user decisions 2026-09-10).** Raised by the user as "model Vale-style regions
as effects, the way `try`/`throw` works"; designed across one session, build
scheduled into phase 5 — ROADMAP "Regions" holds the full design. The calls,
each the user's: **(1)** `Region` is a real effect whose members are `reg`
(move a value in) and `unreg` (copy a value out — always a copy, built into
the function, eliding on fresh constructions); the `region { }` delimiter
registers an *intrinsic handler*, which keeps "an effect is a capability with
a handler" true and leaves `Throw` the single handler-less exception. **(2)**
Membership is an intrinsic provenance qualifier `Reg`, transitive through
projections (inner tags rejected); whether propagation-through-projection
becomes a general per-qualifier property is deferred until more examples
exist. **(3)** Defaults are inverted: everything constructed in region
context is `Reg`; `unreg` is the opt-out (the original sketch had an explicit
`pool()` opt-in, rejected as tag noise). **(4)** R1 only — region-scoped data
plus scope immutability. R2 (bulk-discharging obligations at region close) is
rejected: discharge can be a choice (`stop` vs `join` on a thread handle),
can need context the scope does not hold (`remove(cache, handle)`), and
cleanup functions can use effects; linear values are exempt from regions
entirely. The same two examples broke "an obligation group is a `close()`" —
recorded under L8 for phase 3. **(5)** Sequencing: with phase 5, where a
process *is* a region (per-process heap, sendability as the escape rule) and
`region { }` is the sequential special case — the null hypothesis for that
design session. Names: `region { }`, `Region`, `Reg`, `reg`/`unreg` (the
register/region double reading is intentional). A first write-up the same day
recommended folding the cleanup half into phase 3 and spelling regions
without an effect; it was superseded by (1), (4) and (5), and its analysis of
what transfers from Vale and the "second axis, not a simplification" framing
were kept.

**`defer` is deleted (user decision 2026-09-10).** The reason given, and it is the
whole argument: *`defer` is the partial solution to which linearity is the full
solution already, and it introduces its own complexity.* A linear obligation has
to be discharged on every path or the compiler says which path leaks
[linear-obligation]; `defer` discharged it out of sight, which made the release
invisible at the point it happens and bought a feature whose rules were
genuinely intricate — a body type-checked once but *applied* at every exit, every
flow fact it relied on having to survive to each of those exits, a ban on
`return`/`break`/`continue`/`throw` leaving it, and two unrelated lowerings.

**What went.** Syntax: the `defer` keyword (an ordinary identifier again),
`Stmt::Defer`, the parser arm. Checker: `check_defer`, `apply_defer`,
`run_defers_at_frame_end`, `with_exit_defers`, `defer_exit_floor`,
`PendingDefer`, the `defers` and `in_defer_body` fields, and
`block_defer_escape`/`expr_defer_escape` — the escape-analysis pair that existed
only to reject control flow out of a deferred body. Rules `[defer]` and
`[defer-no-escape]`. The throw and early-exit paths that wrapped themselves in
`with_exit_defers` are now direct calls, which is a genuine simplification of the
hairiest part of the linear check.

**What stayed, under an honest name.** Both emitters keep the machinery, because
the release a `for` owes a pass it owns needs exactly it: Rust splices code at
each exit ([rs-exit-splice] — was `[rs-defer-splice]`; `defers` → `exit_splices`,
`splice_defers` → `splice_exits`, `__deferred_valueN` → `__exit_valueN`), Kotlin
wraps the rest of the block in `try`/`finally` ([kt-exit-finally] — was
`[kt-defer-finally]`). Renaming rather than keeping the old names was the point:
`defer` no longer exists, so code and specs saying "deferred block" would have
been describing a language feature that is gone. The line the rename draws is
also the right one — the compiler owns a minted pass, so it may own its
lifetime; an author's handle is the author's to release.

**What it costs, stated in the example rather than in prose.**
`examples/defer-and-throw` became `examples/throw-and-release`, and the diff is
the argument: `read_size` now writes `close(handle)` on both of its paths, and
`port_from_file` — a handle *and* a throwing call — had to be **reordered**,
because holding the handle across `parse_port` is now rejected. That reordering
is the honest cost and the honest benefit in one place: repetition, in exchange
for the release being where it happens. The checker's diagnostic was reworded to
match, since it named `defer` as the remedy: it now says to release before the
call or move the value onward.

**Verified rather than assumed**, because the decision rests on a claim about
what linearity can express. `crates/salvo-core/tests/linear_release_tests.rs`
(the old `defer_tests.rs`, rewritten) pins the four cases: a path that leaves
without releasing is reported, releasing on every path is accepted, a `break`
must release what the iteration owns, and — the case `defer` was most useful for
— a call that may throw must release first, with the diagnostic naming the
remedy. 784 tests pass (from 800: 20 defer tests and their two backend demo
sections went, 4 arrived).

**Laziness is removed from std and reconsidered after concurrency (user decision
2026-09-10).** `map_lazy`/`filter_lazy` and their composed passes
(`MapYield`/`FilterYield`) are gone. The reasoning is a design position, recorded
because it decides what replaces them: *standard laziness couples data and
functions, and the two should stay separate. What is wanted is a good way to
compose functions — `iter fn`s included — into pipeline functions, which then
mint a pass from data supplied independently.* So a lazy chain should build a
**function**, not a wrapped data structure.

Removing rather than parking it was the right call for a reason the roadmap makes
visible: laziness was touching three unsettled decisions at once — L8 (a composed
pass stores its source, so a linear source is refused), sendability (`Rc<dyn Fn…>`
in a fn-typed field is not `Send`, phase 5), and the shape of the combinator
surface — while being the least settled of the four. Carrying it as a constraint
through four phases would have let it shape decisions it cannot yet justify. See
ROADMAP.md's "Laziness, after concurrency" for the direction and the four
questions it has to answer, one of which is worth stating here: a pipeline that
holds only *functions* stores no source, so "can you lazily `map` over a file's
lines?" may become yes without widening the composite rule at all.

What the removal cost in coverage, and what was done about it: the two lazy tests
per backend became one *unbounded producer with a `break`* (the property that
actually mattered — an `iter fn` computes one element per turn, so an endless
source costs nothing until driven — and, pleasingly, the same expected output),
the combinator-surface demos now reach the generic `map`/`filter` bodies by
handing a pass in explicitly (`map(iter(copy(xs)), f)`, since the `List` overload
takes the fast path), and `[rs-fn-field]`'s stored-callback convention is now
asserted only where it belongs, on the hand-written composed pass — std no longer
has one. `examples/iteration` lost its section 6 chain.

**The `?close` implicit is answered rather than built (2026-09-10).** The last
phase-1 item, struck after checking it: an early-stopping combinator already
releases its source. `[iter-drive-in-place]` plus the `?Linear<It>` spread —
option (a) of the four R0 sketched, chosen over `?close` (option (d)) because it
puts the release on the *type that owns the resource* rather than on every
combinator — means a `for` over a pass the function **owns** releases it on every
exit. Verified on both backends for all four shapes: generic with `break`
(already checked in as `GENERIC_CLOSE_DEMO`), generic with an early `return`
(`close(__loop1_pass)` emitted on both the return path and the fall-through),
concrete through the resolved `close`, and a hand-written `while` driver — the one
shape no loop can help with — caught by linearity itself ("`h` still owns a linear
value when it goes out of scope"). What the item pointed at that *is* still open
is a lazy `take`, and that belongs to L8: a wrapper pass stores its source.

**Mutating through a narrowed place emitted a borrow of a clone on Rust
(defect closed 2026-09-10) — the one [backend-never-wrong] violation this
compiler has shipped.** Found while checking whether ROADMAP's "a suspending
loop driving a pass" item still meant anything. It did not: the item described a
`for` inside a `yield fn` body driving a nested pass out of a machine slot, and
`yield fn`, the planner the bullet named and the slot machinery were all deleted
earlier the same day. Nothing suspends now — a `for` inside an `iter fn`'s
`next` is an ordinary loop, verified on both backends — so the bullet was
retired rather than built.

**What the check turned up instead.** The capability that cut actually cost is
*lazy* interleaving with an inner pass, which under `iter fn` is written by
holding that pass in `state`. A flatten written that way type-checked, ran
correctly on Kotlin, and on Rust printed the first element of the first row for
ever. Reduced to three lines with no iterators in sight:

```
let p: Mut ListYield<Int>? = iter(list(1, 2))
if p is Mut ListYield<Int> { show(next(p)); show(next(p)) }   // Rust: 1, 1
```

**Root cause.** `narrow_unwrap` reads a narrowed value *out of* its declared
representation, and its result is an owned temporary
(`p.as_ref().unwrap().clone()`) — correct for a read, which is all it was
written for. `borrowed_mut_arg`'s fast path skipped any ident needing an unwrap
and fell through to `&mut (<expression>)`, so the mutable borrow was taken of
the clone. It compiles, and the mutation lands on the temporary. Kotlin's smart
cast *is* the storage and its objects are references, so Kotlin was right by
construction — which is exactly why the parity principle catches this class of
bug and a single-backend test never would.

**The fix is a mutable twin of the whole read path** [rs-narrow-mut]:
`narrow_unwrap_mut` unwraps through `Option::as_mut` and a new
`u{i}_mut(&mut self) -> &mut T{i}` on the generated union enums, so the result
is a `&mut` into the storage; `emit_place_mut`/`place_storage_mut` render a
*chain* mutably, so no link becomes a temporary. Both mutable sites now go
through it — an argument in a `&mut` position, and the **base** of an
assignment target, which had the same fault with a louder symptom (`r.at = 2` on
a narrowed `Mut ListYield<Int>?` reached for a field of the `Option`: E0609).
The outermost node of an assignment target deliberately keeps the read rule,
since assigning to a narrowed *variable* writes its storage.

**Two things worth keeping from how it was built:**

- **The read and mutable unwraps share one classification** (`Narrowing`: arm
  index, whether an `Option` sits in front, whether the payload is `Copy`).
  Duplicating the arm lookup would have put [union-arm-identity] in two places
  in one file, which is the drift the checker/emitter invariant exists to
  prevent.
- **The gate is the unwrap producing something, not the presence of a recorded
  representation.** The first attempt keyed on `repr_ty` containing the span,
  which is also true for a `Mut` drop — so a plain `Mut` argument lost its
  `&mut` and five golden tests plus a refinement e2e failed. A recorded
  representation is not a narrowing.

**A second, independent defect had to be fixed to finish it**: the `iter fn`
desugaring gave the synthesized `__p` base of a state-field read *the read's own
span*, so the base answered to the field's narrowing and the emitter unwrapped
`__p` itself (`__p.as_mut().unwrap().inner…`, E0599). This is precisely the
gotcha already recorded from `iter fn`'s construction — "a desugaring must give
every synthesized node its own span" — and the `Spans` allocator written for it
was already there; `pass_field` simply had not been wired to it. The `Field`
node keeps the original span, since that is what a diagnostic about the read
should point at.

**What it cost**: 3 tests (797 → 800) — one program per backend that mutates
through all four narrowing shapes (optional, union arm, assignment base,
`iter fn` state slot) with output asserted identical, plus a generated-source
assertion so the regression is caught with no toolchain on PATH. Two snapshot
groups moved for reasons worth checking before accepting: the five Rust goldens
gained the `u{i}_mut` accessors (additions only, nothing changed), and two
parser AST snapshots changed exactly the `__p` base spans.

**A qualifier applied to an already-qualified value now builds a group arm
(defect closed 2026-09-10).** The last open defect, and the reason it was first
in phase 1: `emitted(ok("x"))` matched no arm of
`Emitted (Ok Str | Err Str) | Finished`, which made the *shape* unwritable
without a workaround — and phase 4's `Ok InputStream | Err Str` is exactly that
shape. The workaround was one `let`, so nothing was blocked; what was blocked
was writing the obvious thing.

**Root cause, restated because it decided the fix**: `Ty::Qualified` holds a
flat, sorted, deduplicated qualifier list whose base is never itself
`Qualified`, and `qualify` appends. So `emitted(ok("x"))` is
`Qualified { quals: [Emitted, Ok], base: Str }` — shape-identical to "two
qualifiers on a `Str`", and *not* `Emitted` applied to `Ok Str`. Matching it
against the arm `Qualified { quals: [Emitted], base: Union[Ok Str, Err Str] }`
compared `Str` against the union, and a plain value never subtypes a
constructive qualifier [qual-constructive].

**The fix is the targeted rule, not the representation change.** ROADMAP.md
offered both: nest `Ty::Qualified` (wide reach — `strip_quals`, `quals()`,
`qualify`, the sort/dedup invariant and every emitter that reads them assume
flatness) or read the value the other way round where the *expected* arm is
`Q (A | B)`. The second is 40 lines: `types::nested_group_remainder` takes the
group's qualifiers off the value's list and asks whether the remainder fits
exactly one arm of the inner union; `is_subtype` consults it as a fallback, and
`nested_group_arm` hands the same answer to the coercion. Two fitting arms is a
*no* — ambiguity is not resolvable from a flat list — so the existing no-arm
diagnostic reports it.

**It turned up a second, latent bug on the same path, and that one was already
mis-shaping code**: a group over *plain* arms (`Emitted (Str | Int)`, one of the
two probes ROADMAP recorded as "fine") type-checked and emitted a **single**
wrap. The inner union is a physical wrapper of its own, so two are needed, and
rustc rejected the output with E0308 — [backend-never-wrong] holding rather than
the compiler being right. `Coercion::WrapUnion` gained an `inner` coercion
applied before the wrap, and both emitters (which already render coercions
recursively, for `DropMut`'s `then`) apply it. The rule and the coercion are
deliberately gated on the same predicate — the inner union must be a *wrapper*
union — so a shape the emitters could not wrap stays an error.

**What flatness still cannot express**, and no rule can fix: the *same*
qualifier twice. `quals` is deduplicated, so `ok(ok(x))` **is** `Ok Str`; the
nesting is gone before any check sees it. That case keeps the annotated
intermediate `let`, and the no-arm error now says so instead of printing a type
that looks like it should fit. The inherent ambiguity is accepted the other way
too: `emitted(ok(x))` and `ok(emitted(x))` have the same type, so either reading
is admitted wherever the expected type picks one — harmless, because every
qualifier erases and the representation is identical.

**What it cost**: 8 tests (789 → 797) — four checker tests in `widen_tests.rs`,
which is where the qualified union group already lived (`^` *reads* a group,
these *build* one), a generated-source assertion per backend that the inner arm
is wrapped first, and a running program per backend for the plain-arms case that
rustc used to reject. Both backends' `FALLIBLE_PASS_DEMO` lost its two
workaround `let`s, so the regression is now carried by a demo that reads the way
the language should. The inventory totals are corrected here as well: they
claimed 817 and 812 by two different routes while `cargo test` and a `#[test]`
count both said 789 — the per-crate figures had not been swept after the `yield
fn` deletion removed 59 tests. They now read 378 / 80 / 73 / 146 / 120 = **797**,
each measured with `cargo test -p` (and 147 / 122 for the two backends after the
entry above).

**std takes a pass and never asks for an `iter` (user decision 2026-09-10).** The
boundary drawn around the previous entry, and the reason given is stronger than
the inference argument R5 used: *a source is not guaranteed to have a container
behind it at all*. A `yield fn`'s machine, a composed pass out of `map_lazy`, a
hand-written `zip` — the pass is all there is in each case, so a std function
declaring `?iter` would exclude them by construction. So the container-shaped
combinator stays something a **program** may write, and `std/` keeps taking
passes; verified as already true (`grep`: every `?`-spread in `std/` is
`?Yield<It, T>`, and the `List` fast paths are overloads on a concrete intrinsic
type that ask for no `iter`). Recorded under [seq-pass] so the next combinator
author, and S-IO, inherit it.

**`yield fn` is deleted and `pass fn` is now `iter fn` (user decisions
2026-09-10).** With both forms in the language side by side, the user picked:
`iter fn` covers the same ground without a state machine, so the sugar goes —
and the name moves to `iter`, which ties the form to the `iter()` function it
generates and says what the pass is *for*.

**`iter fn` is a contextual keyword, and it has to be.** `iter` is the name of
the function the form generates, so it must stay callable and declarable; at item
level a bare identifier is otherwise a parse error, which makes `iter` followed by
`fn` unambiguous with one token of lookahead. `pass` stopped being a keyword with
the rename (`state` stays one).

**What the deletion removed**, and it is the largest subtraction the compiler has
had: the `yield` keyword and the `yield` *statement*; `generator.rs` (1,339
lines) with its plan, numbered states, per-`defer` flags and release path; both
backends' machine renderers, their `Option`-slot bindings, the origin-mint paths
at `for` and at call arguments, and the Rust `SalvoPass`/`SalvoWalk` runtime
module; the checker's `check_yield_fn_decl`, `origin_mints`, `origin_pass_ty`,
`origin_pass_next`, `in_yield_fn`, the mutation-during-drive rule the origin
needed, and `PassDriver::origin`; and the rules `[yield-fn-origin]`,
`[iter-generator]`, `[fn-iterator]`, `[rs-generator]`, `[kt-generator]` and
`[rs-iter-lazy]`. The test count went from 848 to **789** — 59 tests deleted with
the machinery whose behaviour they pinned, and nothing else lost.

**What replaced the affordances the machine had.** Three things a `yield fn`
could do needed a plain answer rather than a port:

- **Effects while driving.** A machine took its handlers per resume; an `iter
  fn`'s `next` is an ordinary call, so `[Console]` is an ordinary effect and the
  `for` supplies the handler once per turn. (Which is why the "effectful `next`"
  codegen cut had to be lifted first — it is what made the deletion possible.)
- **Replay.** A `for` over an origin minted a fresh machine each time; an `iter
  fn` copies the subject at the mint, so a second drive starts over for the same
  reason.
- **Passing a subject straight to a combinator.** `origin_mints` let `map(fibs(6),
  f)` work. It is now `map(iter(fibs(6)), f)` — the container convention, and the
  same rule R5 chose for everything else: a combinator's subject *is* the pass.

**What it costs, stated plainly**: a body whose control flow you would rather not
invert by hand. `while … { yield a; … }` becomes "one turn, then return", which
for a nested loop or a tree walk means writing the state out. And a `defer`
bracketing a whole drive is gone — nothing suspends, so cleanup is a `close` on
the pass rather than a `defer` inside the producer. The examples show the trade:
`examples/iteration`'s Fibs went from an eight-line generator with an
`opening`/`closing` `defer` to an eleven-line step function with no machine
behind it — 81 lines of generated Rust down to 24.

**Two things the deletion turned up in passing**, both worth knowing because they
are the shape of every large deletion in this compiler:

- **A cut that spans a section marker takes general helpers with it.** Removing
  the Rust emitter's "iterator functions" section swallowed `rust_fn_name`,
  `enter_generics` and four more that happened to live under the same heading;
  the compiler caught it immediately, but the lesson is to cut by *function*, not
  by region.
- **Deleting a struct field is a behaviour change when a table is keyed by it.**
  `Viable::pairings` looked dead once the mint bookkeeping beside it was gone —
  and it carried the *argument coercions*, so dropping it silently stopped
  `[str-drop-mut]` firing at call arguments. The test that caught it was a
  Kotlin string-builder assertion, four files away from the edit.

**A generic function may now take a *container* and infer its pass type (user
decision 2026-09-10).** The shape:

```
fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>) -> Int =>[iter] !c {
    let sum = 0
    let p = iter(c)
    for n in p { sum = sum + n }
    return sum
}

total(countdown)      // an `iter fn` subject — its pass has no name
total(bag)            // a container with a written `iter`
total(list(1, 2, 3))  // std's own `iter`
```

`?iter` was always *declarable* — there is nothing special about the name — but
`It` could not be inferred, so `next` was reported ambiguous with `It` still `?`
and the caller had to write both type arguments. **For an `iter fn` subject that
was not a workaround at all**: the generated pass is unnameable, so there is no
type argument to write. That is what made this worth doing rather than recording.

- **The mechanism already existed and was called in the wrong place.**
  `extend_subst_from_implicits` ([implicit-infer], built with S-Seq) learns a
  caller's type variables from whichever fn fills an implicit — but it ran only
  *between* arguments, where `C` is not yet bound, so every `iter` matched
  `(?) -> Mut ?` and it learned nothing. It now also runs once after the
  arguments are typed, and repeats until it stops learning, so one implicit can
  determine another's type and declaration order stops mattering.
- **Ambiguity is the accepted price** (user decision): with the container type
  itself undetermined, several `iter`s match and the choice would be a guess. The
  user's reasoning is that the remedies are already in the language — a `rename`,
  or passing the member by name — so a guess is the wrong trade.
- **No emission change**: implicits are ordinary trailing parameters, so the
  generated Rust is `total<C, It>(c: C, iter: &mut dyn FnMut(C) -> It, next:
  &mut dyn FnMut(&mut It) -> Union2<i32, Finished>)`. Verified on both backends
  with one program covering all three container kinds, plus three checker tests
  (including the accepted ambiguity).
- **What it re-opens, deliberately**: R5 chose "a combinator's subject *is* the
  pass" precisely to avoid cross-implicit inference, and this brings it back as an
  *option* rather than as std's convention. std still takes passes; a container
  combinator is now something a program can write.

**`iter fn` landed (user decision 2026-09-09): a hand-written `next` whose pass
struct is generated.** The question behind it was the user's: the generated state
machine for a `yield fn` is a lot of code per producer, so what if the *other*
form — writing `next` by hand — lost its boilerplate instead? The hurdle was
having to declare a pass struct for the position. Now:

```
struct Countdown { from: Int }

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 { return finished() }
    at = at - 1
    return emitted(at + 1)
}
```

One declaration makes `Countdown` iterable — `for n in c`, `iter(c)` to hold a
pass, and the combinators — with **no state machine anywhere in the output**.
The design decisions, all the user's: the `state` block groups the fields and
mirrors a struct/handler body (so it is *declarations only*, and the annotations
are required for now in all three places at once), the subject is read-only, and
the generated pass stays unnameable.

**It is a desugaring in the syntax crate, and that is the whole reason it was
cheap** [iter-fn]. `desugar::expand_pass_fns` runs inside `parse_module` and
expands one `iter fn` into three ordinary declarations: a hidden
`struct __Pass_<Subject> : Yield<self, T> canbe Mut` holding the subject and the
`state` fields, an `iter` whose body is the struct literal (so the initializers
land where they can read the subject), and the author's body as an ordinary
`next` with the fields written out. Nothing downstream knows the form exists —
resolve, the checker, deductions, narrowing, the LSP and both emitters see the
shape they already supported — so `for`, `iter(c)`, combinators, `?Yield` spreads
and `map_lazy` all worked on the first run. Three rules fell out rather than
being written:

- **"No effects in a `state` initializer"** is not a check: the initializers
  become the body of an `iter` declared `[]`, so an effectful call there is an
  ordinary effect error pointing at the call.
- **"The subject is read-only"** is not a check either: it is a field of the pass
  typed as the subject, so writing through it is the standing [struct-mut]
  refusal, with its existing message.
- **`state` fields get every rule for free** — types, `Mut`, narrowing,
  deductions — because they *are* struct fields.

**Two defects fell out of building it, both in the path the feature needs.**

- **`for` over a container of one's own emitted code the target compiler
  rejected.** The checker has recorded the `iter` to mint with since R5
  (`PassDriver::mint_iter_fn`) and **neither emitter read it**: the loop bound the
  *container* to its pass local and called `next` on it. `salvo analyze` was
  clean, so LANGUAGE.md's promise that "`for x in bag` works as soon as
  `iter(bag)` does" was false on both backends — a [backend-never-wrong]
  violation that had been invisible because every checked-in example wrote
  `iter(bag)` at the call site instead. Both emitters now call it; the seq demo
  drives a `Bag` directly to pin it.
- **Synthesized AST needs unique spans.** The checker's side tables are keyed by
  span — `fn_refs` → `fn_effects`, `expr_ty`, `coerce`, `call_fn` — so the
  generated `iter` and `next` sharing one name span made `iter` inherit the
  `iter fn`'s `[Console]`, and a shared expression span typed a `copy` argument
  as the struct literal that shared it. Fixed with a span allocator that hands
  each synthesized node its own byte *inside* the declaration, so every span is
  still real. Recorded as a gotcha: it is the first desugaring in the compiler,
  and it will not be the last.

**And one recorded cut is gone**: an **effectful `next` driven by a `for`** was a
codegen error on both backends ("the handlers would have to be threaded into
every turn of the loop"). It is threading, and the loop site has the handlers, so
both emitters now pass them per turn — which is what makes an effectful `iter fn`
work, and it lifts the same cut for hand-written passes. A *generic* `next` with
effects is still refused (its effects live on a fn value the caller supplied).

**What it cost, and what it did not.** Two new keywords (`pass`, `state`), which
took std's `next(pass: Mut ListYield<T>)` parameters to `p`; a leading-underscore
type reference is now a parse error, which is what keeps the generated pass
unnameable; and no new machinery in the checker or either emitter beyond the mint
call and the handler threading. Verified with one program on both backends to
byte-identical stdout — a `for` over a subject, the subject replayed, a pass held
and driven by hand then finished by a `for`, three `state` fields, a subject field
read per turn, and an effectful `next` — plus 22 new tests (17 checker, a parser
snapshot of the expansion itself, 2 per backend) and an `iter fn` producer added to
`examples/iteration`.

**A follow-up the same day: the pass holds as little of the subject as the body
needs.** Reading the generated code, the user asked why the pass keeps the
subject at all — and for a plain counter it does not need to: `Halving` reads
`h.start` once, in a `state` initializer, so the field and its per-mint clone were
pure waste. The desugaring now *scans* the body first and picks one of three
tiers: **nothing** when the body never reads the subject; **one snapshot field
per field read** when it only ever reads plain fields (types taken from the
subject's declaration, the field's own name kept unless a `state` field has it);
and the **whole subject** otherwise — handed on as a value, assigned through, a
generic subject, or a declaration this file cannot see. (The visibility limit
was lifted 2026-09-10 — the expansion runs program-wide now; see the
decision-log entry above.)

- **It is free of new rules**, because the mint already copies: the pass can
  never observe a later write to the subject, so snapshotting fields at the mint
  says exactly what a whole copy says. That is what made this cheap here and
  expensive for the `yield fn` machine, where the same idea (roadmap "Mutable
  origins", option (e)) has to interact with the freeze rule.
- **The predicate it needs was already there.** The blocker recorded for option
  (e) was that deciding "every use of the origin is a plain field read" needs a
  *complete* expression walk, and `collect_bindings_expr` is not one. The
  desugarer's rewriter **is** one — exhaustive over every `Expr` and `Stmt` by
  construction — so the scan is that same traversal in a second mode rather than
  a walk to keep in step by hand.
- **Assignment through the subject falls back to tier 3 deliberately**: with a
  snapshot the write would land on a field of the (mutable) pass and *succeed*,
  which would quietly undo the read-only rule. Keeping the subject whole makes
  the standing [struct-mut] refusal fire with its own message.
- What it saves, measured on the checked-in examples: `__Pass_Halving` went from
  `{ __subject: Halving, at: i32 }` with `h.clone()` at every mint to
  `{ at: i32 }`, and `__Pass_Fibs` from holding a `Fibs` to holding the one
  `Int` it reads. Four more tests, and each backend's assertions now pin all
  three tiers.

**Known limitation, inherited rather than introduced**: a *generic* subject
(`iter fn next<T>(w: Window<T>)`) reaching tier 3 is refused by the Kotlin `copy`
lowering [kt-copy], which cannot decide mutability through a type variable. Rust
handles it; Kotlin says so loudly. `yield fn` is untouched and still the answer
when a body's control flow should not be inverted by hand.

**The order of the remaining work is fixed (user decision 2026-09-09), and one
roadmap item turned out to be dead.** With the plan and the record separated, the
open items could be ranked, and the user set the sequence: **1** finish the
iterators (including the flat-qualifier defect that blocks
`Emitted (Ok T | Err E)`), **2** finish shared fate with places and partial moves
(L5), **3** finish linearity with composition and conditionality, **4** build the
filesystem on an IO stream design that uses both, **5** start the threading model
— Erlang/Gleam/OTP. It is recorded as "The sequence" at the top of ROADMAP.md,
with each themed section tagged by phase, so a session picks from the current
phase rather than from a flat list.

What the ranking exercise found, all of it now in ROADMAP.md:

- **Nothing needed to go before phase 1**, and the phase-1 defect is
  load-bearing for phase 4 rather than merely first: a qualifier applied to an
  already-qualified value flattens, which is exactly the shape a stream of
  results needs.
- **Two decisions were re-dated to "before phase 4"** rather than left
  unscheduled, because streams make them concrete: **operator typing and numeric
  promotion** (offsets and sizes are `Long`, and deciding promotion after std has
  a numeric surface means churning that surface) and **two effects sharing a
  member name** (`read`/`write`/`close` collide across `Fs`, the stream surface
  and `Console`, so the `println@Console(...)` question is due there instead of
  being dodged with prefixed names).
- **Phase 3 is three questions, not one.** L8 (obligations through containers) is
  the same machinery question as D7 (linearity conditioned on a use-site
  qualifier) along a different axis, and an earlier decision (2026-09-07) already
  binds D7 to D6 (`once` on any type). D7's *stated* motivation is stale — it was
  written for `once Iter<T>` versus plain `Iter<T>`, and `Iter<T>` no longer
  exists — so the case has to be re-derived from L8. Also folded in: the linear
  instantiation ban does not cover an **effect member's own generics**, which is a
  hole in what already shipped.
- **Phase 5 has a prerequisite the roadmap only hinted at.** `Sendable` was on
  the intrinsic-capability watch list ("the moment concurrency lands"), but the
  blocker is representational: generated Rust holds a fn-typed field as
  `Rc<dyn Fn…>` [rs-fn-field] — every composed pass, so `map_lazy`'s result — and
  `Rc` is not `Send`. A new "Threading and concurrency" section states the four
  questions the existing implementation forces (sendability, what effects a
  spawned process has, OS threads versus a runtime, whether async survives) and
  what is already in place: message ownership transfer is ordinary consumption,
  and linearity survives a send because moves transfer the obligation.
- **`Cell` moves to *after* phase 5**, deliberately. It exists to make shared
  mutable state expressible, and the OTP answer is that processes own their state
  and message-pass — so phase 5 may remove its motivation, and deciding it
  earlier would spend the same language-design call twice.
- **E2 is deleted, not deferred.** It proposed heuristics that inspect "the
  backend define template" for mutations, moves and I/O under a wrong contract —
  and `external`/`define` were deleted in the 2026-09-05 interop redesign, so
  there is no template to inspect. Its job is already done twice over: an
  intrinsic's lowering is Rust code covered by tests, and a `platform effect`'s
  host implementation is checked by the *target's* compiler, which was the point
  of the redesign. Recorded here rather than left as an item nobody could act on.

**PROGRESS.md became two documents (user decision 2026-09-09).** It had grown to
10,400 lines in which the plan and the record were interleaved — the same section
often holding a decision, its implementation notes and the part of it still
unbuilt — so "what is left to do?" could not be answered without reading the
history, and the history could not be read without stepping over the plan. The
split is by *tense*: **COMPLETED.md** (this document) is what happened —
features built, decisions actioned, options explored and abandoned, defects found
and closed, the test inventory and the gotchas — and **ROADMAP.md** is what has
not happened yet, carrying each open item together with the decisions and plans
already made about it.

- **Nothing was dropped**, and that was checked mechanically rather than by
  reading: every line of PROGRESS.md was assigned to exactly one destination, the
  assignment verified to be a partition, and each moved block verified to appear
  *verbatim* in its destination. The only lines not carried over are the old
  title, the companion-documents paragraph (rewritten in both intros) and two
  section headers that became differently-named sections here and in ROADMAP.md.
- **Mixed sections were split at the seam rather than paraphrased.** An arc whose
  early phases landed and whose late ones did not — the linear-types arc, the
  effects arc, the deduction arc — keeps its completed phases here, in the
  headings they were written under, while the open ones moved to ROADMAP.md
  whole, with a one-line pointer left where they used to sit.
- **Arc headings still say "Roadmap:"** because the prose around them, in both
  documents, refers to them by name. Each is the plan *as executed*; the open
  remainder is in ROADMAP.md.
- **One defect turned out to be genuinely open and had lost its heading**: the
  flat-qualifier-list bug that makes `emitted(ok("x"))` match no arm of
  `Emitted (Ok Str | Err Str) | Finished` sat under the *closed* defects with no
  `###` of its own, so it read as part of the entry above it. It now has a
  heading, in ROADMAP.md.
- **Six leftovers were pruned rather than moved**, because later work had
  already closed them and a roadmap carrying dead items is worse than one that
  is short: fn-type effect lists "parse but are not enforced" (E3 step 3
  enforces them [fn-effects]); an overloaded fn passed by name resolving to the
  first overload, and expected fn-type contracts reaching only single-candidate
  callees (both closed by the overload finalization — [fn-value-select] and the
  *lead candidate*); `yield` in a value-position loop being a kotlinc
  "restricted suspending function" error (the `iterator {}` builder it named was
  deleted when the generator lowering landed); and the whole I4-era iterator
  refusal list, which described `Iter<T>` machinery the reduction deleted —
  ROADMAP.md quotes R5's own cut list instead. Their text stays here, in the
  entries that recorded them.
- **What the split makes visible** is the shape of what remains: one reproduced
  defect, nine questions waiting on a language-design call, and a few dozen
  small engineering remainders — against nine thousand lines of record.


**The pass structs are `*Yield`, and `experiments/` became `examples/`
(user decisions 2026-09-09).** Two housekeeping changes with one theme: the
repository should say what it means, and it should show the language that
exists.

- **`ListPass`/`ArrayPass`/`StrPass`/`MapPass`/`FilterPass` →
  `ListYield`/`ArrayYield`/`StrYield`/`MapYield`/`FilterYield`.** "Pass" was
  too general a word for a type name (the user's call); `Yield` names the
  obligation the struct declares (`: Yield<self, T>`), so the type and the
  group now read as one thing. The *concept* keeps its name — a pass is still
  what these are — and nothing else moved: the Rust runtime's `SalvoPass<T>`
  trait and the generated `__Pass_<fn>` machines are internal to a backend and
  untouched. The user named three structs; the other two came with them, since
  half a rename is worse than none. Pure rename, no semantics: `std/`, the
  specs, the tests and every snapshot swept in one pass, the only snapshot
  churn being spans shifting by one character per name.
- **`experiments/` is deleted; `examples/` replaces it.** The two directories
  are not the same thing. `experiments/` held hand-written *prototypes of
  designs not yet built* — target-language code an emitter would eventually
  produce, beside Salvo that deliberately did not compile — and their value was
  the findings, which are recorded in this file. `examples/<name>/` holds
  programs that compile and run **today**: `salvo/` source, the `rust/` and
  `kotlin/` trees exactly as `salvo compile` wrote them, `expected.txt` (the
  same stdout on both backends), and a README saying what to look for. Three to
  start with, chosen by the user: `iteration/`, `defer-and-throw/`,
  `qualifiers/`. Conventions live in `examples/README.md`, including the one
  worth stating out loud: **an example that no longer works is deleted, not
  preserved** — now also an invariant in AGENTS.md, since it is a thing a
  future session needs permission for. Entries *below* this one cite
  `experiments/…` paths as the evidence for what was verified at the time;
  those files are gone, and the citations stay because what they record is the
  verification, not the code.
- **The generator-plan tests were modernized with it** (2026-09-09). Their 13
  inline sources still wrote `fn name(args) -> Once Iter<Int>`, a producer shape
  R5 deleted; they only had to *parse*, so nothing failed and the staleness
  survived the flip. Each is now `struct X : Yield<self, Int>` plus
  `yield fn next(x: X) -> Int` [yield-fn-origin], which made the plans change
  exactly as the model says they should — the origin is field 0, and what was a
  parameter is a field read off it (`i < u.limit`) — and made every positive
  source *check*, verified by running the seven of them as one program through
  `salvo analyze`. That is how three of them turned out to be wrong in a second
  way: `yield i` **moves** `i`, so a body that yields a counter and then
  increments it needs `yield copy(i)`, which no plan-only test would ever have
  told us. The helper now finds the function by its `is_yield` flag rather than
  by name, since every producer's sugar is called `next`.
- **One prototype had a live consumer and moved rather than died**: the I3
  acceptance test reads the gnarly `yield` body from disk. It is now
  `crates/salvo-core/tests/fixtures/gnarly.sv`, **restated in the current
  language** (an origin struct plus `yield fn next`, and a hand-written
  `Countdown` for the nested pass) — and the plan it produces is the
  prototype's eight states unchanged but for `g (param)` where `limit (param)`
  was, which is the evidence the restatement is faithful. The test now finds
  the `yield` fn by its `is_yield` flag rather than by name, because a fixture
  with a nested pass declares two functions called `next`.

**Three examples, and building them was a small survey of the rough edges.**
Each is one `main.sv` verified by `salvo run` on both backends and `diff`-ed:

- `examples/iteration/` — native container loops, a hand-written pass driven in
  two stages, two `yield fn` generators (one effectful with a `defer`, one
  unbounded), a generic combinator of one's own over `?Yield<It, Int>`, the
  eager sequence functions, and the lazy pair plus `map_to`.
- `examples/defer-and-throw/` — `defer` LIFO at a block end and per loop
  iteration (`continue` and `break` included), a `: Linear<self>` handle
  released by one `defer` across two exits *and* across a throw, `[Throw<Str>]`
  with a silent intermediate frame, a union message, and nested `try`.
- `examples/qualifiers/` — a state qualifier with its `qualifies` predicate, a
  `refn` supplying the claim through `add`, an `is` check establishing one at
  run time, a constructive qualifier with its constructor, `^` widening to
  reach the less specific overload, an inferred deduction list keeping a claim
  across a call, an exhaustive one dropping it, and the state/provenance
  contrast — where the generated code names the winner
  (`handle__Authenticated` before *and* after a mutating call, `handle__Fresh`
  before it and plain `handle` after).

What writing them turned up, none of it new but all of it worth having written
down where a newcomer meets it:

- **A lambda in a lazy combinator needs an annotated parameter.**
  `map_lazy(naturals(1), n -> n * n)` cannot infer `U`: `T` comes from which
  `next` filled the implicit, and the un-annotated lambda's result is what `U`
  would have to come from. `(n: Int) -> n * n` fixes it, and the diagnostic
  already names both remedies. The eager `map(xs, n -> n * 2)` is fine, because
  the `List` fast path binds `T` from the argument.
- **A nested string literal inside an interpolation does not lex**
  (`"read ${read_size("a.txt", 3)}"`). Bind the call to a local first. Not a
  defect — the interpolation lexer re-lexes the fragment — but it is the first
  thing an example author trips over.
- **`core.list` has no removal function**, so "a call that may empty the list"
  had to be written as a declaration whose body does nothing. The example says
  so rather than pretending.
- **`map_to`'s destination is moved and handed back** (`[it: Mut, f] Mut D`),
  so the result is the collection to read afterwards, not the argument.
- **Two known Kotlin warnings show up in example output**, both in code the
  author cannot touch: the generic combinators' `unchecked cast of 'Any?' to
  'T'` (recorded under R5) and the same on a nested-union bind
  (`let why: Str | Int = mixed`, dropping a `Thrown` claim). Each example's
  README names its own.

**The iterator reduction is complete (2026-09-09): all six phases are done and
`Iter<T>` is gone.** The user decided a wholesale simplification — a pass is a
user struct, `for` is sugar for `next` until `Finished`, `Iter<T>` goes away —
and it landed in order: **R0**, the hand-written prototype on
both backends (`experiments/next-reduction/`, six shapes, one byte-identical
stdout), which settled unknown 2 (composition renders the group member as an
*implicit*, not a bound — an effectful `next` cannot implement a fixed trait
method) and reshaped unknown 1 into the **origin-struct model** (the user's:
`struct Counter : Yield<Int>` + `yield fn next(c: Counter) -> Int`, the state
machine hidden and unnameable); **R1**, obligation groups as a mechanism
([group-obligation] [group-self] [group-not-a-value]: `: Group<Args>` checked
at the struct, the declaring type written `self` in the argument list, and *no
value may have a group as its type* — the rule that keeps this a where-clause,
not a trait);
**R2**, `Yield<T>` designated in std and `for` reading the declaration —
the `once`-on-passes requirement is gone, a matching `next` without the
clause gets a remedy hint, and driving still consumes (drive-in-place waits
for R5); **R3**, the origin-struct sugar [yield-fn-origin] on both
backends — `yield fn next(c: Counter) [Console] -> Int` discharges the
obligation, the machine is hidden and uncallable, driving mints a fresh one
per loop so an origin replays, and one program plus one stdout proves the
parity; **R4**, `Linear` as a designated obligation group
([linear-group]: `: Linear<self>` with its mandatory `close`, `discard` no
longer discharging) plus the interim composite refusal ([linear-composite]:
a struct field, array, tuple, union arm or type argument may not *hold* a
linear value — refused at the store, which also turned two diagnostics per
mistake into one); and **R5**, the flip itself — `iter` returns a pass
[iter-pass], std's combinators take a pass through a `?Yield<It, T>` spread
[seq-pass], and the whole `Iter<T>` apparatus is deleted: the intrinsic type,
`SalvoIter`/`Iterable` as its representation, `once` on producers, the
effect-claim-on-a-type ([iter-effects] is now a *deleted* rule), the variance
adapter, the per-effect-set traits, `params Iterable`, and the factory tail of
both generators. Decisions taken along the way, all the user's: a `close`
never
implies `Linear` (roadmap L8 records the composition casualty), `canbe
Linear` keeps its spelling beside `: Linear`, the Rust fn-typed-field
refusal is lifted ([rs-fn-field]: `Rc<dyn Fn…>` fields, so a
hand-written composed pass builds on both targets), and a combinator's subject
**is** the pass (so a container is iterated by writing `iter(xs)`, which is
what keeps the inference ordinary). See "Roadmap: iterators — the reduction
to `next`" for the phase notes, "R5 part 2 as built" for what the flip forced,
and the known cuts listed there.

**Two follow-ons landed the same day (user decisions 2026-09-09), and they close
the flip's own leftovers.** First, **`for` drives a generic pass**
[iter-generic-drive]: where a body has a `?Yield<It, T>` spread, that position
*is* the declaration the loop needs, so a combinator is written with `for` — and
std's `map`/`filter`/`reduce`/`map_to`/`filter_to` now are, which is the
dogfooding the flip left undone. When the fn **owns** a possibly-linear pass
(`<It canbe linear>` and no deduction handing it back), it must declare the
release it will need — `?Linear<It>`, whose member the loop calls on every exit —
and a body without one is an error naming that remedy. Second, the **generic
effect** refusal on a `yield fn` is lifted: `[Random<Int>]` works, because the
machine's handlers are ordinary parameters rendered from the effect *type*.
Building the first turned up a **parity defect** worth the whole exercise —
`for` over a pass a fn *keeps* cloned it on Rust and aliased it on Kotlin, so the
same program printed different things — fixed as [iter-drive-in-place]: a kept
pass is advanced where it lives, on both backends. See "The generic drive as
built".

**The iterator design was re-evaluated and reduced (user decisions 2026-09-08);
the next work is that reduction, not the old roadmap.** `Iter<T>` was a
structural interface in a language with none — a factory, i.e. a function, which
is why `once` and effects both fit on it — and it left two protocols (`next` and
`iter`) with `for` arbitrating, plus a combinator surface that could not see a
hand-written pass at all. The replacement: **a pass is a user struct, tied to
iteration by a `next`, and `for` is sugar for calling it until `Finished`.** The
ties between functions and structs are declared with **compiler-known `params`
obligation groups** (`: Yield<Str>`, `: Linear`), which also gives linearity a
named discharge (`close`). See "Roadmap: iterators — **the reduction to
`next`**" for the decisions, the six-phase plan, the unknowns to settle first,
and what it deletes (most of I4, the boxing question, `once` on producers).
Everything below this line is the design it supersedes, kept because its
prototypes and defect findings are what the reduction stands on.

**I5 landed (2026-09-08): the combinator surface, and four defects behind it.**
`map`/`filter`/`reduce` stay eager; `map_lazy`/`filter_lazy` return a producer;
`map_to`/`filter_to` map into a collection the caller provides, reached through
an `?add` **implicit parameter** — so the destination is anything with an `add`,
not a `List` (user decision). Verified on both backends with identical stdout,
including a lazy chain over an **unbounded** producer that terminates only
because the consumer breaks. Writing it was mostly *finding* things: the
generic `?Iterable` body had never run (every caller took a `List` fast path),
so a producer holding implicits, an implicit with a `Mut` parameter, and
`return None` in a `None`-returning fn were all first-time paths — four
defects, three of them pre-existing and backend-general. Details under "I5 as
built".

**The release path is plumbed everywhere (2026-09-08), which is I2c's first
part.** A generated pass is no longer a target-language iterator — Rust gets a
`SalvoPass<T>` trait with `advance`/`close` and Kotlin a `SalvoClosable`
interface on its pass base — so every `for` over an `Iter<T>` mints, advances
and closes, claiming or pure, and a nested pass is released with its outer one.
Collections keep native loops (decision 8). **No observable behaviour change
today**, deliberately recorded as such: after [iter-mut-param] a pure producer
has no way to make an abandoned `defer` visible, and every route that would is a
shape still refused — this is the protocol the un-boxing half of the split needs,
correct in advance. Details under "I2c: the release path, everywhere".

**A producer may not carry mutable state — parameter or capture (2026-09-08).**
[iter-mut-param] grew its second half the same day: a lambda handed to a
producer's fn-typed parameter may not capture mutable state either, since a
producer keeps its callback for as long as it can mint a pass and calls it once
per element in *every* pass. Both kinds are refused — a capture the closure
writes through, and one it merely reads mutable data through, because
snapshot-vs-alias is what makes the second observable. Rust already rejected
both (`impl Fn + 'static`) while Kotlin ran them and accumulated across passes:
a checker-clean program only one backend could build. **And the remedy turned
out to be broken too**: a producer's callback was emitted as a *borrowing*
closure despite its declared `impl Fn + 'static`, so E0373 hit every capturing
lambda — immutable ones included. One `move`, and the first test in the suite
that hands a capturing lambda to a producer. **I5's surface is decided**
(user): eager by default, `map_lazy` for the lazy pair, `map_to` mapping into a
caller-provided collection with an `?add` implicit.

**A producer may not take a mutable parameter (user decision 2026-09-08).**
Asking whether the *pure* producer's injected `close` is observable — the I2c
leftover, and the natural next item after I4 — turned up a `[backend-parity]`
defect with nothing to do with `close`: a `yield` fn taking `sink: Mut
List<Int>` mutated a **private copy** on Rust (parameters are captured by
clone, which is what makes the factory's state `'static`) and the **caller's
own list** on Kotlin (a `MutableList` is a reference), so successive passes
accumulated on one target and started fresh on the other — same source,
different output, silently. Which backend is "wrong" is a language question, so
the shape is **refused** [iter-mut-param] rather than sided with: transitively,
by the same `Mut`-at-any-depth test [fate-move-mode] uses, with a diagnostic
naming the two remedies that work everywhere (yield the values and let the
consumer collect them, or reach the outside through an effect). Nothing in
`std/` or the corpus did this, so it landed without a sweep. Two follow-ons
recorded rather than decided: **`once Iter<T>`** may be able to take one after
all — one pass exists, so what remains is shared fate ([fate-link]), which
needs I2c's representation split — and **sharing it properly on both backends**
is queued as the sharpest customer of the `Cell` roadmap ("Producers: the
`Mut`-parameter case (option C)").

**I4 is complete (2026-09-07): an effectful producer compiles and runs on both
backends.** The language half landed earlier the same day — a producer's effects
live on its type, `FileSystem Iter<Str>` [iter-effects] — and the emission half
is now built: a **generated trait/interface per effect set**
(`iter_effects.rs` / `iter_effects.kt`, one per set the way `UnionN` is one per
arity), the handlers threaded into the machine's `advance`/`close`/`__run_dN` as
*parameters* in the checker's canonical order, a `for` over a claiming producer
lowered to **mint/advance/close** — which is what finally gives the injected
`close` a caller — and the **variance adapter** the prototype turned up, at the
positions the checker records it. Verified with `experiments/pull-iterators/
effectful.sv` compiled and run by `rustc` *and* `kotlinc`: byte-identical
stdout, no warnings from either toolchain, and the same source in both backends'
test suites (the parity claim). The two "producer that performs effects"
refusals are gone; four narrower ones took their place
[backend-never-wrong]. Details under "I4 emission as built".

**D8 decided and I4's language half built (2026-09-07): a producer's effects
live on its type.** `FileSystem Iter<Str>` is a producer whose *driving*
performs `FileSystem` [iter-effects] — the effect written in **qualifier
position** (user decision: a type with no arrow has nowhere to put a `[…]`
list, and a prefixed one reads as a deduction list in return position), on the
**return type** rather than in the `yield` fn's own list (none of its body runs
when it is called, so a list there would demand a handler at every call site
for something calling it never does), and legal exactly where `once` is legal
(`Iter<T>` and a `canbe once` type of one's own). Every other rule came free
from [fn-effects]: a fn **inherits** a producer parameter's claim, *fewer*
effects fit where more are expected, the claim never drops, and **holding** a
producer needs nothing — only driving it does. [iter-effect-free] is gone as a
rule. Emission followed the same day, and its **shape was fixed by a
prototype** (`experiments/pull-iterators/effectful.{sv,rs,kt}`, byte-identical
output on both toolchains, no warnings): a trait/interface per effect set, the
factory still a factory, and — the finding worth having early — a *variance
adapter*, because a pure producer used where a claiming one is expected has a
different representation. Details under "D8 decided", "I4 emission prototyped"
and "I4 emission as built".

**The `yield` lowering is built (2026-09-07): one state machine, planned in
`salvo-core`, rendered by both backends.** `generator.rs` turns a `yield` fn
body into a `GeneratorPlan` [iter-generator] — numbered states of steps, the
body's locals as fields, a flag per `defer` site, a release path — and its
acceptance test is the I3 prototype itself: the plan for the checked-in gnarly
body (now `crates/salvo-core/tests/fixtures/gnarly.sv`) must be the eight states
of the hand-written prototype machine, in its order. It is, on the first run.

**Both emitters now render it, and the borrowed coroutine machinery is gone**:
`SalvoGen`/`SalvoYield`/`async`/`Pin`/`Waker` on Rust [rs-generator], the
`iterator { … }` builder and `return@iterator` on Kotlin [kt-generator]. A
pass is a struct (a class) the compiler writes, with the body's locals as
fields and `__advance` as one flat `match`/`when` on a state number. `Iter<T>`
keeps its representation — a factory that mints a pass per `for` — so nothing
about the *semantics* moved; what moved is who builds the state machine, which
is what lets `next` take effect handlers per resume later (I4). Verified end to
end on both backends with one shared program and one shared expected stdout: a
`defer` inside a suspending loop body writing a local read after the loop, a
bare `return` out of the middle of the nest, a nested producer, and a second
pass that starts over.

Details and the four places the prototype's shape overruled the sketched plan
are under "I3 step 1 as built"; the emission half, including the two things the
real emitters forced (an `Option` slot for a field whose type has no zero, and
a **defect** the migration exposed: a `return` inside a value-position loop was
leaking into the machine as a target-language `return`), is under "I3 steps 2–3
as built". What remains of the `yield` half is the representation split (I2c's
second half) and threading effects (I4).

**Overload resolution finalized 2026-09-07 (user decisions), and it is now
one rule with two escape hatches.** The agenda was drawn up from probing the
old implementation — twelve small programs, most of which answered in ways
nobody had chosen — and the user settled every item. The governing principle
was stated first and decides most of the details: *the rule for which
function is selected should be simple, we should refuse to guess but rather
raise an error, and we should give the user options for addressing the
error.*

**The rule.** A call is decided in three steps [fn-overload]:

1. `f@module(args)` names the module whose overload is meant, if written
   [fn-overload-at].
2. **The most specific scope that fits wins** [fn-overload-scope]: `core`,
   then this file's imports, then this module, then the fn's own scope
   (fn-typed locals, parameters, implicits, effect members), then inner
   scopes. Only the most specific rung with a candidate *fitting the
   arguments* competes.
3. **Then the most specific signature** [fn-overload-rank], per argument
   slot: a type variable says least; fewer union arms says more (`Int` >
   `Int | Str` > `Int | Str | Bool`, and `Any` is the broadest type there
   is); more qualifiers says more, with the *kind* never ranking; and a
   fixed parameter list beats a variadic one. No single winner is an error
   [fn-overload-ambiguous].

**What that fixed.** The open defect from yesterday — an own-module fn losing
to an identically shaped std one, *silently* — is closed by step 2: a
program's own `size(List<T>)` now means its own, while `size("text")` still
reaches core's, because the shadowing overload does not fit. Scope beats
signature deliberately (the alternative needs the reader to know std's whole
surface), and where it discards a more specific signature the call gets a
**warning** naming both candidates and both `@module` forms.

**The two escape hatches, both erased before emission:**

- **`f@module(...)`** — a module *path*, not a rung keyword (`@mod`/`@import`
  would ask the reader to know which rung a name came in on). It works in dot
  form (`xs.size@core.list()`), as a value (`describe@main`), and it is the
  only way to reach a function a **local of the same name** shadows.
- **`rename fn label_small = label(n: Small Int)`** — a scope-local name for
  one overload, which from that point answers *only* to it. Not an alias:
  that is what makes the old name unambiguous again, and it is the remedy the
  ambiguity diagnostic names. Module-scoped (whole module, not importable) or
  block-scoped from its line, matched by the same overload matcher `refn`
  uses [qual-refn-match], and it may not mention effects, deductions or a
  return type — none of them takes part in selection [fn-rename].

**Three defects came out with it**, all found by probing rather than by
report: an own-module fn losing to std's (above); **two identical parameter
lists** being declarable, with the second silently unreachable — now a
declaration error [fn-overload-duplicate]; and **effect member calls not
checking their arguments at all** (`log(true)` against
`fn log(message: Str)`) — now checked like any other call
[effect-member-call]. A fourth was already recorded and is fixed here too: an
overloaded function *passed by name* resolved to whichever overload was
declared first; it is now selected by the expected fn type
[fn-value-select].

**Two things the implementation forced:**

- **`Any` had to become the top type it always claimed to be.** Written as
  the `intrinsic type Any`, it lowered to a *nominal* `Named("Any")`, so
  `f(v: Any)` accepted nothing at all — unification compares names. The
  ranking rule "`Any` is the broadest thing a parameter can say" would have
  been theory. `Any` now lowers to `Ty::Any`, like `Nothing` already lowered
  to `Ty::Nothing` [type-any-nothing].
- **Rust needed a path for a shadowed call** [rs-shadowed-call]: functions
  and locals share one value namespace there, so `describe(7)` beside
  `let describe = "…"` is E0618 — the emitted call is
  `crate::describe(7)`. Kotlin needs nothing, since functions and properties
  are separate namespaces. This is the first mechanism whose *only* reason to
  exist is that `@` made a shadowed function reachable.

Deferred, and recorded rather than dropped: **`@Effect` for member
disambiguation** (`println@Console(...)`). Two effects declaring one member
name is currently a declaration error whose message already promises this
syntax; lifting it means a multimap plus every member path
(`check_effect_call`, deduction inference, refinements, LSP), so it is a
milestone of its own rather than an easy win.

**S-Str landed 2026-09-06 (user design): `Str canbe Mut`, and the string
function surface.** A string is still immutable; what `Mut Str` adds is a
string *under construction*, asked for explicitly (`mutable_str("he",
"llo")` — a literal is never a builder) and mutated by exactly three
functions (`append`, `set`, `clear`). Everything else in `core.string` takes
a plain `Str`, and a builder reaches all of it by dropping its `Mut`.

**That drop is the feature.** Every other qualifier erases, so widening only
forgets a claim — and `Mut List<T> <: List<T>` has always been free because
`MutableList<T>` *is* a Kotlin `List<T>`. A `StringBuilder` is not a
`String`, so this is the first `Mut` whose removal costs an instruction.
Rule [str-drop-mut]:

- **The checker records the drop** (`Coercion::DropMut`), rather than each
  emitter guessing from types — the standing checker/emitter agreement. The
  record carries the qualified type, so a backend decides from its base:
  Kotlin renders `.toString()` for `Str` and nothing for `List`, Rust
  nothing at all (there `Mut` erases, and `coercion_of` unwraps a `DropMut`
  so even the "is this argument a fresh temporary?" tests see that nothing
  happens).
- **It fires at every drop site**, which is the part worth testing rather
  than believing: call arguments (intrinsic lowerings included — a
  `StringBuilder` must not reach `String.getOrNull`), returns, `let`
  annotations, struct fields, union arms, interpolation, and operator
  operands. Operators are *coerced, not rejected*: Kotlin's `StringBuilder
  == String` is `false` and `sb1 == sb2` is reference equality, against
  Rust's structural `String == String` — a live parity divergence, and
  rejecting `Mut` there (the [op-no-none] treatment) would surprise, since
  `Mut List` is accepted everywhere else.
- **A drop carries the coercion it displaces.** One expression has one
  coercion slot, and a `Mut Str` flowing into a `Str | Int` needs both the
  conversion *and* the union wrap, so `DropMut` has a `then` field. Found by
  writing the union-arm test, not by design.
- **`copy` had to learn about it**: a `Mut Str`'s copy is
  `StringBuilder(sb)` on Kotlin [kt-copy]. Identity would alias the buffer —
  the S1 transitive-mutability trap, and `ty_immutable` already answered
  correctly (`Mut` anywhere means mutable), so this was one arm in the
  copy lowering rather than an analysis change.

**Two things fell out of implementation.** First, the surface is
*char-indexed* on both targets, which Rust needs conversions for (`find`
answers in bytes) — and Kotlin still counts UTF-16 code units, the
divergence `size`/`char_at` already had, now shared by `substr`/`index_of`/
`set`. Astral-plane text is where the two differ; a `Char`-exact `Str` is a
decision, not a patch. Second, **`set` needed a generated support module on
Rust** (`strings.rs`, mounted and imported like `iter.rs`): replacing a
character reads *and* writes the string, and an inline `let s: &mut String =
&mut place;` does not compile for a `&mut String` parameter (E0596 — the
binding is not `mut`). A trait method auto-refs an owned local, a `&mut`
parameter and a field projection alike, and mentions the receiver once, so a
call argument is never evaluated twice.

**And a pre-existing hazard surfaced: `...spread` into a variadic
intrinsic.** `mutable_str(...parts)` spliced the array as one argument,
which Kotlin's `StringBuilder.append(Any?)` cheerfully accepted — printing
`[Ljava.lang.String;@37bba400`, i.e. silently wrong output
[backend-never-wrong]. The same shape in `list(...arr)` was merely a
kotlinc/rustc error, which is why it had gone unnoticed. Fixed for all of
them [fn-variadic]: Kotlin uses its own spread operator (`listOf(*arr)`),
Rust takes the vector itself (cloned — Salvo does not track a variadic
position, so the array stays usable), and `mutable_str` *borrows* its parts,
since it reads them rather than storing them.

Verified end to end by compiling and running one 40-line program covering
the whole surface under both `kotlinc` and `rustc` with byte-identical
stdout, plus a second one for `set` through a parameter and a field.

**S-Seq landed the same day too — `map`/`filter`/`reduce` over anything with
an `iter`** — and it is the answer to "what did Salvo get instead of traits"
applied to collections: `params Iterable<It, T>` is a bundle of implicit
parameters, so a customer type becomes iterable by declaring one function.
The design was decided in advance and needed **four mechanisms that did not
exist** — implicit resolution feeding back into type inference
[implicit-infer], a *lead* candidate for expected types
[fn-overload-rank], intrinsics passed as adapter closures
[implicit-intrinsic], and parameter contravariance in implicit resolution —
plus generated Rust helpers, because Rust closure inference rules out every
inline shape. Details under "Roadmap: standard library surface".

**O1 (overload specificity) landed the same day**, and was superseded the
next: a concrete parameter type beating a type variable is now one rung of
the finalized ranking [fn-overload-rank] — see "Overload resolution" and the
entry at the top of this file. What O1 got right and the finalized rule kept:
comparing the *declared* patterns, the partial order with an error for "no
winner", and the leniency guard for un-inferred arguments.

**Qualifier refinements (`refn`) landed 2026-09-06 (user design), closing
roadmap D3.** D1 made deductions sound by forbidding a mutating function
from promising a qualifier it never declared, and accepted the
over-strictness that follows: `add` cannot promise `NonEmpty` back even
though appending to a list can never empty it. The insight that fixes it is
that **the function was never the party to ask** — it has never heard of
`NonEmpty`. The qualifier that owns the claim states it instead:

```
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool { return list.size() > 0 }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty, !elem
}
```

The user's decisions (all 2026-09-06):

- **A qualifier may only refine its own claim**; a *top-level* `refn` may
  name any state qualifier in scope. This was the load-bearing choice: it
  collapses the conflict taxonomy to one case. The two failure modes the
  design memo listed — refinements removing each other, and a refinement
  declaring exhaustively — become unreachable, the second by grammar and
  the first because only `Q` can speak about `Q`. What remains is
  add/add: two qualifiers that cannot co-apply [qual-with].
- **A top-level `refn` is module-scoped and not importable.** Reconciling
  is the consumer's call; a library shipping its own reconciliation would
  move the conflict one level up.
- **Overload matching is by types *and* names** [qual-refn-match], with
  type parameters positional. Requiring names means a std rename surfaces
  as a diagnostic instead of a refinement that silently stops firing.
- **Conflicts are per (callee, parameter)**, not per call: a disagreement
  about one parameter must not cost the refinements of another.
- **A suppressed conflict warns.** No error — the program compiles and the
  function is simply less useful, exactly as designed — but silence would
  make an imported refinement's doing nothing undiagnosable, so a
  `Severity::Warning` lands at the call site, once per (callee, parameter).
- **Effect members are not refinable** (deferred): a member has no `FnKey`,
  and naming one needs an effect-qualified form.
- **Inference is included.** A refinement reaches the inferred contract, so
  the fact survives one frame outward.

**Two things fell out of implementation that the memo did not predict.**

First, *reconciliation only works if a top-level `refn` **replaces** the
qualifiers' refinements* for the parameters it names [qual-refn-reconcile].
Joining them would keep the disagreement — the user's stated remedy ("
redeclare the reconciled qualifier in their scope") is only a remedy under
replacement. It is the same precedence own-module declarations already have
over imported ones [mod-collision].

Second, *additions and the deduction fixpoint do not mix freely*. The walk
in `deduce.rs` is a meet over all uses, not a flow analysis: removals only
ever accumulate, which is what makes it order-insensitive and terminating.
An addition is the opposite direction, so `if c { add(list, x) }` would
have let a signature promise a fact that holds on one path. Two
restrictions keep it sound [qual-refn-infer], and both were worth having
for independent reasons:

- **Only at `cond_depth == 0`** — the addition must be unconditional in the
  body (a new depth counter bumped by `if`/`when`/loop/lambda/`defer`/`try`
  bodies). The *call site* stays flow-sensitive and still narrows inside
  the branch; it is only the exported *contract* that is conservative.
- **Only for a qualifier the parameter declares.** A refinement can cancel
  a removal, never invent a claim — inventing one is `+Q` in a function's
  own deduction list, which is D2 and stays deferred. This also bounds the
  lattice (the keep set stays a subset of the declared set), so the
  fixpoint still only shrinks and still terminates.

**`+Q` now has exactly one home, and D2 is untouched.** In a `refn` it is
the qualifier author's claim about someone else's call — trusted, like
`-> T as Q` [qual-ctor-fn]. In a `fn` it remains a parse error naming D2,
because there it would be a claim about your *own* body, which needs an
establishment rule.

**No backend work at all**, and that is a consequence of the "state
qualifiers only" rule rather than luck: `Mut` is the one qualifier that is
*not* erased, so admitting `+Mut` would have made this an emission feature.
Verified by compiling and running one refined program under both `kotlinc`
and `rustc` — `after add: 1` / `after refill: 2`, byte-identical — with the
emitted `main` asserted to contain no `qualifies` call, since a refinement
is trusted rather than checked.

**The warning had to be made non-fatal, and then given somewhere to go.**
Both backends' emission gates aborted on *any* diagnostic, so the first
conflicting program printed the warning and then refused to compile — the
exact opposite of "no compiler error, just a less useful function". The gate
is errors-only now; and since a warning nobody sees is the same as no
warning, `Backend::emit` gained a channel for it (user request 2026-09-06):
it returns `Emitted { files, warnings }`, and the CLI's one build path
prints the warnings unconditionally — not under `--verbose` — before
carrying on. Two things worth knowing about the shape:

- **The warning-dropping entry point stayed.** `emit_program` is what 108
  golden tests call, so it keeps its signature and delegates to the new
  `emit_program_reporting`, where the drop is one visible `map`. Changing
  the public shape would have churned every one of those call sites to say
  nothing new.
- **The failure path renders *every* diagnostic**, warnings included. Each
  carries its own severity prefix, so they read correctly next to the
  errors — and nothing is lost while the author fixes the errors, which is
  what the old all-diagnostics behaviour got right before it also aborted.

`salvo platform generate` deliberately reports nothing: it writes host stubs
once, and the program's diagnostics belong to the compile path.

**One rule, one implementation**: the refinement-conflict test and the
declaration-site [qual-with] check now share `refine::quals_compatible`. A
conflict *is* "these two could not have been written together", so letting
the two drift would have been a bug in waiting.

The feature is `crates/salvo-core/src/refine.rs` (~700 lines): resolve each
refinement to its overload, validate it, compute per-file visibility, merge
and detect conflicts. The checker applies groups after the removal set in
the named-call contract loop; `deduce.rs` applies the same table through
`apply_callee`; the LSP merges the docs [qual-refn-docs].

**`use Handler<T>()` now binds the handler's generics (fixed 2026-09-06).**
Found while recording it as a defect, which is the only reason it was found at
all: the written type arguments were *parsed* and then read by nobody, so the
checker left the effect instance generic and both emitters constructed the
handler with no type argument. Both backends emitted code their own compiler
rejected — Kotlin "cannot infer type for type parameter 'T'", Rust `E0283`
plus `E0392` — with `salvo analyze` reporting nothing, which is a
[backend-never-wrong] violation rather than a documented cut. A generic
handler could therefore only be instantiated when a *constructor argument*
happened to bind its parameter, which a stateless one never has.

The fix follows the data: `check_use` binds the handler's generics from the
written list first (a constructor argument that disagrees is now an error, and
the wrong number of arguments is reported), records them as
`use_handler_args`, and both emitters write them at the constructor —
`Plain<Int>()`, `Plain::<i32>::new()`. Rust additionally emits a
`PhantomData` field for a type parameter no field mentions, since a handler is
a *behaviour* and a generic one need hold nothing, where Rust insists every
parameter be used (`E0392`).

Two things this corrected in the specs, both under [rs-effect-fusion]'s cut
list. Its "Reported" promise held only on the *fusion* path, so a
single-effect program emitted invalid Rust instead; and its claim that
"Kotlin accepts these (erasure)" was false — erasure removes a type argument
from the JVM, not from the source, so Kotlin needs it written exactly as Rust
does. The type arguments are now written **even where the target could have
inferred them**: the emitter does not reason about the target's inference, and
the case that motivated the fix leaves nothing to infer from. That made two
golden snapshots and three assertions more explicit.

**Implicit parameters landed 2026-09-05 (user decisions), and they are what
Salvo got instead of traits.** `?cmp: (T, T) -> Int` is a parameter the caller
need not pass: the call site fills it by resolving the parameter's *name* at
the parameter's *type*. The whole feature rests on one observation — Salvo
already overloads by parameter type, so **the default for a type is just a
function**:

```
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn total<T>(xs: List<T>, ?Field<T>) -> T => xs { ... add(acc, x) ... zero() ... }

total(list(1, 2, 3))                          // 6, defaults resolved
total(list(2, 3, 4), add = times, zero = one)  // 24, overridden by name
```

Koka spells the defaults `Str/cmp` because it does not overload on argument
types; here the `cmp` whose parameters accept `Str` already *is* the ordering
for `Str`, so no qualified-name syntax was needed and nothing is tied to a
type's declaration. The decisions (all user, 2026-09-05):

- **Resolution is name + type, at the call site**, with no global coherence:
  which default a call gets depends on what is visible where the call is
  written, exactly as `use` and handlers already work. Ambiguity is an error
  naming the override as the remedy.
- **Generic code forwards its own implicits automatically** — matched by name
  and type. It is the only thing that *can* fill an inner call there, since
  nothing about an opaque `T` is knowable [call-resolve]. A generic fn that
  declares none cannot call one that needs one; the error says which to add.
  Approved as colouring in the same shape effects have.
- **A group has no binder** (the refinement that shaped the design):
  `?Field<T>`, not `?ops: Field<T>`. The binder referenced nothing (members
  are called unqualified), prevented no collision, and forced the caller to
  know it just to override one member. Without it the *individual* implicit
  parameter is the only unit in the system — resolution, forwarding and
  override all key on a member's own name — so an inner fn may declare `?add`
  directly, or reach the same parameter through a different grouping.
- **A group is declaration-side sugar, never a value.** That is what keeps it
  free of any runtime representation: neither backend knows groups exist.
  Verified before choosing: a struct of fn-typed fields ran on Kotlin but
  **did not compile on Rust** (`E0562: impl Trait is not allowed in field
  types`), so a bundle-as-value would have needed `Rc<dyn Fn>` fields and a
  way to call a fn-typed field — which dot-notation already spells otherwise
  (`ops.add(a, b)` *is* `add(ops, a, b)` in Salvo, and even `(ops.add)(x)`
  routes there). R5 later lifted the *field* half [rs-fn-field]; the calling
  half is a language rule, so the decision stands.
- **Overrides are named arguments**, `cmp = f`, scoped to implicit
  parameters — Salvo has no general named-argument form, and a general one
  stays a separate decision. Unambiguous because assignment is a statement
  here, never an expression.
- **Implicits trail**, and are declared on a **fn or an effect member** (the
  member half added 2026-09-06: "they're just normal functions"). A member's
  implicits belong to its signature — the interface takes them, every handler
  takes them, the call fills them. Handler *constructors* and lambdas still
  may not have them: `use` resolves nothing, and a lambda's type has no room
  to declare one.
  * On Rust an implicit parameter is `&mut dyn FnMut(..)` — **`dyn`
    everywhere**, decided by two failures in a row. First the trait and its
    impl disagreed (`E0053`), because a member's implicits have to be `dyn`
    for `&mut dyn E` to stay object-safe. Then a member *forwarding* to a
    plain fn handed a `dyn` value to an `impl` (`Sized`) parameter, which
    rustc refuses — so a per-position convention could not compose, and one
    convention everywhere is both simpler and the only sound choice. The cost
    is an indirect call, which every effect member call already pays.

`[name-casing]` pays off unexpectedly: `?cmp:` versus `?Field<T>` is decided
by the case of the first token after `?`, so the grammar needs no lookahead.

**No new backend machinery**, which is the sharpest contrast with the traits
design this replaced (that needed generated interfaces on Kotlin and real
traits plus impl blocks on Rust). An implicit is an ordinary trailing
parameter of fn type; the call site passes a function reference (`::add`) on
Kotlin, an adapter closure on Rust. Two Rust wrinkles were worth the trouble:
a resolved fn is a fn *item*, not a closure, so it is wrapped mechanically;
and an argument that reborrows an implicit the same call passes has to be
hoisted into a `let` first, or the borrows overlap (`E0499`) — the same rule
[effect-args-hoisted] already applies to threaded effect values.

**The contract is part of fitting, and it does not print** (2026-09-06). What
a call does to each argument decides whether a function fits a fn-typed
position, but `Ty`'s Display shows parameters, effects and the result — not
the contract. So a mismatch there used to read "expects `(Int, Int) -> Int`,
found `(Int, Int) -> Int`". The checker now explains it instead: which
argument, in which direction, and both fixes (give the candidate a deduction
list that returns the argument, or declare the position as consuming). Two
further touches came out of the same work: a *near-miss* is reported as one
("no `cmp` fits … the `cmp` in scope is …") rather than as "nothing of that
name", and candidates are **ranked**, so a same-shape wrong-contract
declaration is what gets explained rather than whichever unrelated overload
of the name came first — the first version dutifully reported std's
`add(Mut List<T>, T)` when the user's own `add(Int, Int)` was the near-miss.

Verified end to end on both backends with one shared program and one shared
expected stdout: group defaults resolved, one member overridden by name, a
lambda override, forwarding through an opaque `T`, an individually declared
`?add` reached by the same name, and (a second shared program) an effect
member's implicit resolved at the call and received by its handler.

**The struct-field-of-fn-type hole is closed as part of it**: that shape is
now a Rust codegen *error* naming the implicit-parameter remedy, rather than
invalid output rustc rejects [backend-never-wrong].

**The suite got its iteration cost back, 2026-09-05.** The toolchain tests
had grown to ~75 of the ~80 seconds a `cargo test` took, which was starting
to shape how often it got run. Three findings, in order of how much they
returned:

1. **The availability *probe* was nearly as expensive as the work.** Each of
   39 Kotlin tests ran `kotlinc -version` as its guard, and that starts a
   JVM: 1.4s, against 2.4s for the compile it guarded. Probing once per test
   binary took the Kotlin crate from 44s to 32s without touching a test.
2. **A verification is worth remembering.** Compiling and running generated
   code is a pure function of the code, the expected output and the compiler
   doing it, so `salvo-testkit` writes a stamp keyed by the content hash of
   exactly those inputs and skips the work when one already exists. A plain
   `cargo test` is therefore still complete but now costs **~8s warm** (80s
   cold). Touch the emitter and every affected stamp misses — verified by
   adding one comment line to the emitted output and watching the suite go
   straight back to 15s of real toolchain work. A stamp is written only after
   every assertion passes, so a failure is never remembered as a success.
   `SALVO_E2E_FRESH=1` ignores stamps: that is what **full** means, ~70s with
   nothing taken on trust.
3. **`cargo nextest` did not help, and is kept for diagnosis.** It runs each
   test in its own process, which is a loss when 531 of them are mostly
   microseconds: 18s warm against `cargo test`'s 8s, and a wash on a fresh
   run (74s vs 71s). Thread count does not change the picture (`-j6` 22s,
   `-j20` 18s). What it *is* good for is per-test timings, which stock
   libtest will not print on stable — that is how the CLI's five long poles
   were found. Config in `.config/nextest.toml`.

Two smaller things fell out. The Kotlin and Rust runners were writing their
scratch trees to the *system* temp dir, against the repository's own rule
that temporary files stay inside it; they now use `CARGO_TARGET_TMPDIR` like
everything else. And three Kotlin tests had no toolchain guard at all, so
they would have *failed* rather than skipped on a machine without `kotlinc`
— found by moving the gate inside the runner helpers, where a test cannot
forget it.

**`Iter<T>` is now lazy on both backends (user decision 2026-09-05, option 3
of the four costed below).** The divergence that prompted it: the same
program printed different things, because Kotlin's `Iterable { iterator { …
} }` produced elements on demand and re-ran its producer per pass while
Rust's `Vec` materialised everything at creation — so *when* a producer's
effects happened, *whether* they interleaved with the consumer, and
*whether* a second `for` re-ran them all differed. The user chose lazy on
both **with iterator functions restricted to being effect-free**, on the
grounds that "other functions can just use them with effects via
for-loops".

What that took:

- **[iter-effect-free]** An iterator fn declares no effects, `use`
  included. Checked once at the declaration, which is enough: performing an
  effect requires declaring it, and `use` is what would otherwise let a
  body register its own handler. The diagnostic names the remedy (perform
  it where the elements are consumed).
- **[rs-iter-lazy]** `Iter<T>` is a generated `SalvoIter<T>` — a *factory*
  of passes, `Rc<dyn Fn() -> Box<dyn Iterator<Item = T>>>`, which is
  exactly what Kotlin's `Iterable` already was. Keeping it a factory rather
  than a one-shot iterator is what avoided the second decision the costing
  flagged: `for` still does not consume its subject, and linearity is
  untouched.
- **Stable Rust has no generators, so the state machine came from
  `async`**: rustc builds one for an `async` block, and the generated
  `SalvoGen` drives it with `Waker::noop()`. `yield v` becomes
  `__slot.replace(Some(v)); SalvoYield::once().await;`. No `unsafe`, no
  crates, ~120 generated lines in `iter.rs` (mounted like `unions.rs`, only
  when the program touches `Iter<T>`). Hand-written and verified before
  being taught to the emitter, per the gotcha that says to do exactly that.
- **Parameters are captured once into the factory and cloned per pass**, so
  each pass starts from the beginning and the captured state is `'static`.
  Sound *because* of [iter-effect-free]: a handler arrives as `&mut dyn E`
  borrowed for the call and could not live that long. Generic parameters
  gained `'static` next to the blanket `Clone` bound for the same reason.
- **One convention exception**: an iterator fn's fn-typed parameter arrives
  owned as `impl Fn(…) + 'static` (shared across passes via `Rc`) rather
  than `&mut impl FnMut(…)`, because it is called in every pass rather than
  during the call. `Fn` rather than `FnMut` follows from repeatability —
  callback state would depend on how many times the iterator was consumed,
  which is the same divergence [iter-effect-free] rules out for effects.
  The checker does not reject a state-mutating callback up front; rustc
  does. Known gap, recorded in [fn-iterator].

Verified end to end on both backends with one shared program and one shared
expected stdout: an *unbounded* producer (`while true { yield … }`) that
terminates because the consumer `break`s — it would previously have hung
forever on Rust — a filter over it, and a factory consumed twice that
starts over each time.

**Three defects found by writing `map`/`filter`/`reduce` in Salvo, fixed
2026-09-05.** Testing the combinators — rather than reasoning about them —
turned up one checker bug and two silent-wrong-code bugs, all independent of
the traits question that prompted the exercise:

- **[call-generic-progressive]** A callee's type variables now bind
  *progressively*, left to right, so an un-annotated lambda argument is
  checked against the pattern its *siblings* already determined
  (`map(xs.iter(), n -> n * 2)`). Before, the lambda was checked against an
  unsubstituted `(T) -> U`, so its inferred type mentioned the callee's own
  variable (`(T) -> T`) — exactly what `unify`'s deliberate lack of an
  occurs check assumes cannot happen [fn-overload] — and the call failed to
  match itself. An explicit type-argument list did not help either; it is
  now what seeds the substitution. Left-to-right only: a lambda written
  before the argument that would bind its parameter type still needs an
  annotation, because Salvo does not look forward [call-type-args] and a
  fixed-point pass would reorder the fate events a call records.
- **[rs-fn-param-convention]** A lambda in a fn-typed parameter position
  now binds its parameters the way the *callee's declared* fn type renders
  them. The two sides used to test `Copy`-ness on different types — the
  declaration on `T` (never known to be `Copy`, hence `FnMut(&T)`), the
  lambda on its own annotation (`Int`, hence `|n: i32|`) — so rustc
  rejected the call with `E0631`. Invisible until a lambda was *annotated*:
  the un-annotated form compiled, because rustc inferred the parameter from
  the bound.
- **[kt-fn-mangling]** Overload dispatch is the checker's, and Kotlin no
  longer gets a second opinion: every emitted overload of a name gets a
  unique Kotlin name, by the same rule the Rust backend already used
  [rs-fn-mangling]. Sharing the name let Kotlin resolve by *Kotlin's*
  lattice, and two Salvo types with no subtype relation can map onto Kotlin
  types that have one (`Iter<T>` → `Iterable<T>`, `List<T>` → `List<T>`,
  and Kotlin's `List` *is* an `Iterable`). A `List` overload delegating to
  its `Iter` sibling emitted a call that re-resolved to itself: infinite
  recursion, no diagnostic anywhere [backend-never-wrong].

**The `Iter<T>` laziness divergence is closed** — see the lead entry: option
3 was chosen and built on 2026-09-05. The divergence as measured (Kotlin's
`Iterable { iterator { … } }` lazy and re-running its producer per pass,
Rust's `Vec` materialising at creation) is gone, and the obstacle that
shaped the decision — a lazy Rust iterator would have to hold the fn's
effect handlers *across* the suspension, where emitted Rust borrows them
only for the call — is what [iter-effect-free] removes rather than works
around.

**Non-resumption was renamed 2026-09-05 (user decision): `Abort` → `Throw`,
`abort` → `throw`, `Aborted` → `Thrown`.** A pure rename of the E3 step-2
feature, taken under the no-backwards-compatibility invariant: the old
spelling simply stopped being the language, with no transitional
diagnostic. What moved: the std module `std/core/abort.sv` →
`std/core/throw.sv` (module `core.abort` → `core.throw`), the compiler's
known names (`ABORT_EFFECT`/`ABORTED_QUALIFIER` → `THROW_EFFECT`/
`THROWN_QUALIFIER`, with `OK_QUALIFIER` untouched), `Checked::may_abort` →
`may_throw` with `AbortSite` → `ThrowSite`, the Kotlin signal
`salvo.AbortSignal` → `salvo.ThrowSignal` in `abort.kt` → `throw.kt`, and
the rule labels `[abort]` → `[throw]`,
`[abort-not-main]` → `[throw-not-main]`, `[abort-linear]` →
`[throw-linear]`, `[kt-abort-signal]` → `[kt-throw-signal]`,
`[rs-abort-controlflow]` → `[rs-throw-controlflow]`. `try` is untouched:
it was never named after the operation. Nothing about the *semantics*
changed — `throw` is still an ordinary effect operation returning
`Nothing`, still handler-less, still delimited by the intrinsic `try`
yielding `Ok T | Thrown M`; in particular it is **not** a keyword, so
`throw(message)` is a plain call. The test suite kept its 514 tests
(`crates/salvo-core/tests/abort_tests.rs` → `throw_tests.rs`), and both
backends still compile and run the demo byte-identically under `kotlinc`
and `rustc`. Two sites were deliberately left alone because their `abort`
is the English verb, not the feature: `salvo-cli`'s "does not abort the
analysis" / "aborting due to N parse errors", and the roadmap's
"panic/abort semantics out of scope" (Rust's process abort).

**Interop was redesigned 2026-09-05 (user decisions): string-template
interop was replaced by `platform effect`, and the redesign is complete.**
The old model — `external` declarations resolved by `define` templates that
interpolate `${param}` into target syntax — was, in the user's words,
"difficult to validate, and only really there so that certain std types and
functions (like those related to the list) don't get unnecessarily wrapped in
backend functions". The replacement follows Roc's platform idea: the compiler
*generates an interface* and the host implements it, so the target
language's own compiler checks the two against each other. There is now
exactly one interop path for customer code (`platform effect`) and one for
std (`intrinsic`, which only std may declare).

The decisions (all user, 2026-09-05):

- **`internal` → `intrinsic`**, and it is the *compiler's* modifier:
  declared by std, never by customer code, because "I don't see why a
  customer would be able to declare `intrinsic` if the compiler doesn't
  already declare it".
- **`external` → `platform`, and `define` goes away entirely** — "one
  interop path is good". So every std lowering moved into the backends as
  code, and the unvalidatable template text is gone from the language.
- **A platform declaration is an `effect`**, not a free function
  (agreed after the alternative — top-level `platform fn` collected into
  one synthetic effect per module — was costed): the author picks the
  interop boundary, the name is theirs, and the implementation has
  somewhere to keep state and dependencies.
- **The host owns `main`.** A platform effect's instance is constructed
  outside Salvo, so it cannot be `use`d; it is a *parameter*. A `main`
  declaring one is emitted as `salvoMain`/`salvo_main` and the host's
  `main` calls it. The user accepted this consequence explicitly, noting
  it can be skipped when `main` needs no platform effect — which is how it
  works.
- **Deferred**: `platform handler` (a host handler of an ordinary Salvo
  effect, constructed by `use`) and `platform type` ("let's leave platform
  type until a need arises" — the type-aliasing gap the user was willing
  to accept, with platform structs sketched as a future answer).
- **Rejected**: generic platform effects and generic platform members.
  **Made errors**: member-name collisions.
- The CLI command is **`salvo platform generate`** (the `-api` suffix was
  dropped).

**The interface framing removed work rather than adding it, in two places
worth recording.** First, a platform effect *is* an effect, so the existing
interface/trait emission, `&mut dyn` threading and handler fusion all
applied unchanged — the only emitter surgery was the `if !is_main` guard on
effect-parameter emission. Second, it made the whole
"don't overwrite the customer's implementation" problem disappear: the
first design needed marker-delimited regions keyed by a canonical mangled
name, with signature-change detection, because generation would run
repeatedly over a file the customer edits. With an interface, the
implementation file is generated **once** and never touched again, and every
kind of drift is a target-language compile error — a member added is
"does not implement abstract member" / `E0046`, one removed is "overrides
nothing" / `E0407`, a changed signature is an ordinary type error, and a new
platform effect breaks the entry-point call. No markers, no keys, no merge.

Landed so far (each step left the tree green):

1. **`internal` → `intrinsic` everywhere.** `TokenKind::KwIntrinsic`,
   `BackingMod::Intrinsic`, `emit_intrinsic_call`,
   `Symbols::intrinsic_types`, rules `[backend-intrinsic]` and
   `[intrinsic-fn]`. Incidentally deleted the dead duplicate keyword list
   in `TokenKind::symbol()` — the `KEYWORDS` lookup precedes it, so those
   arms were unreachable, and they are exactly the drift hazard the gotcha
   below records as having once panicked the compiler.
2. **std migrated off templates.** New `intrinsics.rs` per backend
   (`type_name`, `mut_type_name`, `fn_call`, `handler_member`); all ten
   `std/**/*.{kotlin,rust}.sv` files deleted; `emit_define_handler` became
   `emit_intrinsic_handler`, taking member signatures from the *effect*
   declaration. Dispatch is on the checker-resolved declaration (name +
   first parameter's base type name), which is what separates
   `size(Str)`/`size(List)`/`size([])` without the define era's arity
   guessing. **No backend golden snapshot changed at all** — the emitted
   output is byte-identical to the template era, which is the strongest
   evidence the migration is faithful.
3. **`platform effect`.** Parser (the modifier takes nothing but `effect`),
   five checker rules, both emitters, entry-point restructuring. Verified
   end to end: the same source, with a hand-written host implementation,
   prints `[telemetry] work=41` / `result=42` under both `kotlinc` and
   `rustc`.
4. **`salvo platform generate` + the `platform/` tree wiring**, which
   together close the loop: the compiler now writes the host skeleton and
   builds it back in, so a platform program goes from `.sv` to program
   output with two commands and no hand-written glue.

   *The tree* is the source root's `platform/`, mirroring the source
   layout (`platform/app/entry.kt` for `app/entry.sv`). It reuses the
   companion mechanism [backend-companion] wholesale — `CompanionFile`
   gained a `platform: bool`, and `classify_companion` strips a leading
   `platform/` segment so the file is attributed to the module it
   implements for. The strip is load-bearing, not cosmetic: a companion is
   only copied when its *module* is reachable, and no Salvo module is ever
   called `platform.app.entry`, so without it the host file would be
   discovered and then silently dropped. Only the root's `platform/` is
   special, so a module named `platform` keeps its own companions. Both
   backends' hosts coexist in one tree, since discovery filters by the
   active backend's extension.

   *Kotlin* puts the host in package `salvo.platform.<module>`, not
   `salvo.<module>`. That was forced: Kotlin names a facade class after the
   **file**, so `platform/main.kt` sharing `salvo.main` would put a second
   `MainKt` on the classpath. The consequence is that the launch class
   changes when the host owns `main`, which is why `Backend::entry_hint`
   now takes the emitted file list — the presence of the host file is the
   evidence, and it is exactly the right one, since a platform `main`
   cannot be emitted without it.

   *Rust* mounts the host as `platform_<module>` (prefixed so it can never
   collide with the module it implements for) and appends
   `fn main() { crate::platform_<module>::main() }` to the crate root,
   because `fn main` must live there. The `rustc` invocation is unchanged.
   `module_mod_names` was extracted so `emit_program` and the skeleton
   renderer cannot disagree about what a module is called — a skeleton
   naming `crate::core_console::…` where the crate calls it something else
   would be generated code that does not compile.

   *The skeletons* are rendered by the same `Emitter` and the same
   signature renderers the interface emission uses (`emit_param_list` /
   `emit_member_param_list`, `emit_return_type`), on the same checked
   program, and the entry-point arguments come from the same
   `checked.fn_effects` table the entry's *parameters* come from. That is
   the point: a skeleton that does not match the interface it implements is
   impossible by construction rather than by test coverage. `TODO(…)` /
   `todo!(…)` bodies typecheck in return position, so a value-returning
   member stubs without a cast.

   *The command* never overwrites — an existing file is reported and left
   alone — which is the whole dividend of the interface framing recorded
   above. It shares the front end with `compile`/`run`: `build` was split
   into `assemble` (load, parse, resolve the entry) + emission, so
   `platform generate` resolves `--src`/`--main` identically.

   *The missing-host gap is closed by an error, not by hope*: any reachable
   module whose `main` needs a platform effect and has no host file is a
   codegen error naming `salvo platform generate` and the exact path. The
   rule is per module and identical on both backends, so a two-entry
   directory cannot compile on one backend and fail on the other. The old
   symptoms (`'main' method not found in class salvo.main.MainKt`, rustc
   `E0601`) are unreachable now.

   Both backends' `platform` tests were rewritten around the *generated*
   skeleton with only its stub body replaced, so what they prove is that
   the skeleton is complete and correct everywhere else — package, imports,
   trait path, member signature, entry-point call. Same stdout on both:
   `[telemetry] work=41` / `result=42`.
5. **`external`/`define` deleted, and `intrinsic` enforced as std-only** —
   the last item, and the one that makes the redesign real: there is now
   exactly one interop path.

   *Deleted from the language*: the `external` and `define` keywords, the
   `define fn`/`define type`/`define handler` items, the double-backtick
   `Template` token and its lexing, the `imports:`/`inline:`/`Mut inline:`
   sections, and the `<module>.<backend>.sv` define file as a concept. With
   them went `Symbols::{define_fns, define_types, define_handlers,
   external_types}`, `check_define_pairing`, both backends'
   `check_core_define_coverage`, `define_for_decl`, `emit_define_call`,
   `define_type_args` and the whole template-expansion machinery —
   about 900 lines net.

   *Two enums collapsed into flags.* `BackingMod` had one variant left, so
   `backing: Option<BackingMod>` became `intrinsic: bool` on `FnDecl`,
   `TypeDecl`, `HandlerDecl` and `QualifierDecl` — the same shape
   `EffectDecl::platform` already had, and for the same reason the earlier
   gotcha records: a one-variant enum is an invariant expressed as a
   runtime check. `SourceKind` went the same way: with no define files
   there is only one kind of source, so the field is gone rather than
   always `Language`, and every `if file.kind != Language { continue }` in
   the checker, both emitters and reachability went with it.

   *Three new rules fell out of the deletion, each replacing something that
   used to be silent or meaningless.*
   - **[decl-body]**: a bodiless top-level `fn`, and a `type` with neither
     `= alias` nor `intrinsic`, are parse errors. Those were `external`'s
     shape; without it there is nothing for them to mean, so the parser
     says so and names the surviving forms rather than reporting a bare
     "expected `{`".
   - **[intrinsic-std-only]**: `intrinsic` is the compiler's, so only std
     may write it. It is checked in `check_intrinsic_is_std_only` rather
     than the parser, which sees one file's tokens and cannot know where
     the file came from; the checker has `is_std` at hand. Not cosmetic —
     the backends dispatch intrinsics from a table keyed by *name*, so a
     customer `intrinsic fn` has no lowering anywhere, and the diagnostic
     names the `platform effect` path instead.
   - **[mod-file-name]**: a `.sv` file name may not contain a dot. This is
     exactly the spelling that used to select a backend's define file, and
     `classify` used to *skip it in silence* — so a leftover
     `main.kotlin.sv` would have vanished from the build. It now errors,
     naming both the file and the nested path it is ambiguous with. That
     is why `SourceSet::classify` returns `Result` and `add_dir` returns
     rendered messages rather than `(PathBuf, io::Error)` pairs.

   *CLI surface shrank.* `salvo analyze --backend` and `salvo lsp
   --backend` are **gone**: their only documented purpose was loading a
   backend's define files, and an option that does nothing is worse than
   no option. Both commands are unconditionally backend-neutral now.

   *The test sweep was the bulk of it* — 108 `external` declarations across
   14 files. Most became ordinary fns with the *same* signature and a
   minimal body, which is what keeps the tests testing what they tested: a
   written deduction list stays authoritative over anything inferred from a
   body, so the contract under test is unchanged and the call stays on the
   named-call path rather than moving to `check_effect_call`. The
   `intrinsic` type stubs those tests relied on moved into a std-loaded
   `core/prelude.sv` per harness (module `core.prelude`, implicitly
   imported), which is precisely the `is_std` split this document predicted
   would be needed. Tests whose *subject* was the define machinery were
   deleted outright — missing-define errors, core coverage, the
   define/external pairing, template generics — along with the
   `defines.kotlin.sv` corpus file and its snapshot. **No golden snapshot
   of emitted code changed**, again: the second time in this redesign that
   byte-identical output has been the evidence a mechanism swap was
   faithful.

   *Two things the suite caught that a human sweep would have missed.* The
   TextMate grammar still listed `external` and `define` as keywords and
   still defined a double-backtick template rule — caught by
   `keywords_are_fully_categorized`, which asserts the grammar's categories
   partition the lexer's keyword table exactly. And `run_tests` still
   asserted the old "backend define file" message for a dotted `--main`,
   which is now the [mod-file-name] error.

Still to do: nothing from the interop redesign. Its deferred pieces remain
deferred by decision — `platform handler` (a host handler of an ordinary
Salvo effect, constructed by `use`) and `platform type` — and are recorded
under the decisions above rather than as leftovers.

**One cut taken beyond the decisions, flagged to the user**: generic
platform *effects* are rejected as well as generic members. A generic
platform effect would need the host to implement one interface per
instantiation — Kotlin's facets exist for that and Rust has no equivalent,
and the fusion already refuses generic effect instances there.

Status snapshot as of 2026-09-03: both backends (Kotlin, Rust) work
end-to-end; the post-M8 phase added developer tooling (`salvo analyze`,
the `salvo lsp` language server, a VS Code extension) and a flow-sensitive
ownership analysis (use-after-consume from declared *and* inferred
deductions, uniform across types, branch- and loop-aware). The
shared-fate arc (S1, L2, S2, L3, L4, S3) **and L6 must-use linearity**
are complete: move-mode bindings emit real moves, borrow-mode bindings
and loops emit real borrows, read-only pipelines over kept parameters
are clone-free — and `canbe linear` types carry a use obligation
(forgetting to close/commit is a compile error, `discard` is the
explicit escape hatch, enforcement is purely static and identical on
both backends). This document is the handoff point for continuing
development: it records what is built, the key design decisions, known
limitations, and the plan for what's next. **The L7 milestone is
complete** — L7a (`<T canbe linear>`), L7b (`once` fn types), L7c
(derived returns `ReadOnly[from: p]`), and L7d (fn-type contracts:
named fn-type parameters with standard deduction lists, applied at
fn-value calls, inherited by lambdas, carried by named fns, emitted as
real Rust modes). Small recorded remainders: fn-type *effect* lists
parse but are not yet enforced, `once` inference, the internal
qualifier unification (nothing forces it), and L5 field precision only
if whole-variable granularity proves too coarse. (It did: L5's poison half
landed 2026-09-10 — see the decision log.)

**E3 step 1 landed 2026-09-04: `defer { ... }` on both backends — and was
deleted from the language 2026-09-10** (user decision; see the decision log, and
note that the syntax below no longer parses). **The
first slice of handler control beyond "always resumes" starts with the
piece the rest of it depends on: a way to run code on the way out of a
block. Four user decisions shaped it (all 2026-09-04): **block-only
syntax** (`defer { ... }`, consistent with every other body form),
**block scope** (not function scope — a `defer` in a loop body runs per
iteration), **splice-at-exit semantics**, and **no control flow out of a
deferred body**.

Splice-at-exit is the load-bearing one, and it *dissolved* the open
question the roadmap recorded ("does a deferred body capture by move or by
reference?"). `defer S` means: `S` is checked and emitted as if written at
every exit of the enclosing block — its end, and each
`return`/`break`/`continue` leaving it. There is no closure, so there is
nothing to capture, and `defer { close(f) }` discharges the linear
obligation exactly as writing `close(f)` at each exit would
[linear-obligation]. The proposed Rust lowering in the roadmap sketch (a
`Drop` guard) was dropped for a concrete reason: `close(f)` consumes the
handle, so the guard must own `f` from the `defer` onward — which makes
`f` unusable for the rest of the block, i.e. exactly the code the feature
exists to enable (a `&mut` capture trades that for `E0499` at the next
use).

Lowerings: Kotlin wraps the rest of the block in `try { … } finally {
body }`, where nesting gives LIFO for free and `try` being an *expression*
keeps value-position blocks working [kt-defer-finally]; Rust splices the
body — rendered once at the `defer`, re-indented at each exit
[rs-defer-splice]. Verified end to end: the same demo (LIFO at a block
end, an early `return`, `continue`/`break` out of a loop body, a linear
handle released on both paths of a two-exit fn) prints byte-identical
output under `kotlinc` and `rustc`.

Checker design worth remembering: the body is checked **once**, in the
flow state at the `defer` statement (snapshot/restored, so nothing is
consumed *there*), and the *effect* of running it is replayed at each
exit — the values it consumes are consumed there, so a manual `close(h)`
plus a deferred one is a use-after-move, reported once per `defer`. The
facts the single check relied on are recorded with it and verified at every
exit: if a call in between takes the value away or invalidates a narrowing
the body used, that exit is an error naming the remedy. Without that check
a deferred `!!`/unwrap could be emitted for a fact that no longer holds
[backend-never-wrong].

One divergence is accepted and recorded for future work: Kotlin's
`finally` also runs while an *unexpected* exception unwinds (a panic out of
a std intrinsic), where the Rust splice does not. Salvo has no `catch`, so
side effects during a crash are not part of a program's meaning; tightening
it means catching the throw signal specifically once `throw` exists.

Probed by hand on both backends beyond the test demo (identical output
except where noted): a `defer` in a value-position block, a `defer` nested
inside a deferred body, a deferred call through an *effect* member with the
handler registered by `use`, a `defer` inside a lambda block body passed to
a fn-typed parameter — and a `defer` in an *iterator* body, which differed
at the time: Rust's eager collection ran the deferred prints before the
consumer saw any element while Kotlin's lazy sequence interleaved them.
That cut predated `defer` (a plain `println` after a `yield` diverged the
same way) and is **closed** as of 2026-09-05, when `Iter<T>` was made lazy
on both backends [rs-iter-lazy].

**The widening check `^` landed 2026-09-05 (user design).** The dual of
`is`: boolean-valued, same places, same runtime test, but a successful check
reads the subject with the named qualifiers **removed** rather than added —
`list ^ Mut` without `Mut`, `outcome ^ Ok` without `Ok`. A `when` branch head
may be `^ Qual` too, which is the form that motivated it: `Ok (Ok Int | Err
Str)` is a claim *about* a union, and `when o { ^ Ok { when o { … } } }`
reaches the inner union with no intermediate binding and no repeated type.

Decisions taken with it (all user, 2026-09-05): nothing to remove is an
**error** (`^` removes a known claim; testing for one is `is`), qualifier
**sequences** are allowed, there is **no binding form** (the subject itself
reads widened), and droppability comes from **one exclusion list** —
`types::qual_drop_block`, which `Qual T <: T` now reads as well, so the two
cannot drift as intrinsics are added. That list is `once` (restricts rather
than refines), `Linear` (carries a use obligation) and `ReadOnly` (the value
is derived from another); `ReadOnly` cannot be *written* in source today, so
it is unreachable from a `^` and the list carries it for the day that
changes.

**The lowering needed a materialization the design did not anticipate, and
finding it was the whole value of running the demo.** Qualifiers are erased,
so widening looked like a pure typing act: emit `is`'s test and change the
type. The emitted code compiled — and was **wrong**. Where the check peels a
wrapper arm (`Ok (A | B)` → `A | B`), a nested `when` on the subject still
scrutinized the *outer* wrapper, whose arm 0 is the one the outer test had
already taken, so the second inner branch was dead code: `wrapped(0)`
returning `err("zero")` printed `value zero` instead of `error zero`. The
generated union's `Display` impl had been masking it, since printing either
arm produced plausible output.

The fix is to materialize the peel: the widened value is bound to a
**shadowing local** at the top of the branch — `let mut nested =
nested.u1().clone();` in Rust, `val nested = nested.value as Union2<Int,
String>` in Kotlin — so reads and nested `when`s see the inner value.
Shadowing (rather than a fresh name) is what avoids rewriting every read;
Kotlin warns about it, which is the price. Rust also saves and restores the
binding *kind* around the branch, since the shadow is owned where the outer
binding may be a borrow. Checker-side this needed a physical-view override on
the variable (`LocalVar::widened`), because `repr_of` reads the variable's
declared type and a root narrow could not previously change it.

Two cuts, both reported: `^` on a *projection* (`p.result ^ Ok`) needs a
plain variable to shadow, and a `^` matching **more than one arm** cannot be
one widened view (each arm peels a different wrapper position) — the latter
rejected in the checker, so it reads as a language rule rather than a codegen
failure.

**`salvo run` landed 2026-09-05 (user design).** One command from `.sv`
source to program output: compile with a backend, then build and run the
result with that backend's own toolchain [cli-run].

```bash
salvo run --backend kotlin --src ./my_project
salvo run --backend rust --main ./my_project/main.sv
salvo run --backend rust --src ./my_project --main ./my_project/bin/tool.sv
```

`--backend` is required (it decides which toolchain must be installed, so
there is no defensible default). **`--src` and `--main` are independent, and
each supplies a reasonable default for the other** (user decision
2026-09-05): `--src` alone uses the unique `main` the directory declares,
`--main` alone additionally implies `--src $(dirname FILE)`, and **both** is
the only way to say "compile this tree, start at this file" when the entry
sits in a subdirectory. That last case is the one that earns the
combination: `--main bin/tool.sv` alone would take `bin/` as the source
directory, leaving a module shared from above it outside the compilation.
Either way `--main` is how you pick between several entry points.

The first sketch had `--main` walk imports and include only the modules it
transitively reached; the user cut that as premature (2026-09-05), and
rightly — the whole directory is compiled either way, and reachability
already prunes the *output* to the modules actually used.

`--target` defaults to `.salvo_tmp_run` in the working directory, and
`--clean-target` (`before` | `both`, default `both`) says what survives:
both modes clear the target *before* the build, so a run never picks up the
previous run's output. **The command's exit code is the program's** and its
stdio is inherited, so `salvo run` can stand in for running the binary
(verified: a Rust panic propagates 101, a JVM exception 1).

Two design points worth keeping:

- **The target may not overlap the sources**, for two *separate* reasons.
  It may not be or contain the source directory, because the target is
  deleted before the build. And it may not sit *visibly* inside the sources,
  because the next build would read the emitted files back — output carrying
  the backend's native extension is indistinguishable from a hand-written
  companion file [backend-companion]. Nesting under a dot-prefixed
  directory is fine, since discovery skips those [mod-ignore]; that is
  exactly why the default target is `.salvo_tmp_run` and why the literal
  reading of "no overlap" had to be rejected — it would have made the
  default illegal for the most natural invocation (`cd project && salvo run
  --main main.sv`). Independently, clearing a target that holds any `.sv`
  file is refused: a guard on the deletion itself, not on the paths.
- **The entry choice had to reach the backend**, not just the launch
  command. Running is not purely a post-processing step: Rust gives the
  `main`-declaring module the crate root [rs-crate], so with two entry
  points the emitter picked the first it found and `--main other.sv` built a
  module with no `mod` declarations — rustc reported unresolved imports for
  every module. `Backend::emit` now takes the selected entry; Kotlin ignores
  it (a `MainKt`-style facade per module means the choice only picks the
  launch class). This also moved the `entry point:` hint out of the CLI's
  `match backend.name()` and into the trait, where that knowledge belongs —
  and immediately exposed the hint's own bug (see the gotchas: the Kotlin
  facade class is named after the *file*, so it is not always `MainKt`).

**The subject-less `when` landed 2026-09-05 (user design).** `when` was
already the exhaustive construct; it is now exhaustive in *two* ways.
With a subject it branches on a union's arms and takes **no `else`**
(unchanged, but now stated that way and enforced with a diagnostic that
names the other form instead of reporting a missing `is`). Without a
subject it is a condition chain — bare boolean branch heads — whose
**`else` is mandatory** [when-condition]:

```
let label = when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    else { "positive" }
}
```

**The mandatory `else` is the entire reason the form exists**, and it is
worth being explicit about why, because the feature otherwise looks like
`if`/`elif` with different punctuation. An `if` chain without an `else`
folds `None` into its value [if-else-none], so "a chain of conditions that
always produces a value" is a property the reader has to verify by
inspection; this form is the one the grammar guarantees. That framing is
also what kept the implementation small: `Expr::WhenCond` is checked by
`check_if` itself, so narrowing, accumulated exclusions, the value join,
`Nothing`-tail dropping and [fn-must-return] all came for free and cannot
drift from `if`'s behaviour.

Decisions taken with it (all user, 2026-09-05): **bare branch heads** (no
`->`; consistent with `if cond { }` and with every other body form),
**`is`/`^` allowed in the heads** (they are boolean-valued, so they narrow
their branch — note this buys *no* arm-exhaustiveness, so `is` heads that
visibly cover a union still need the `else`; the subject form is how you
ask for that check), an **`else`-only `when` is a parse error** (it decides
nothing — write the block), and conditions are **required to be `Bool`**
everywhere, not just here.

That last one closed a standing gap rather than adding strictness for this
feature: `[if-bool]` had said "no truthiness" since M0 and *nothing
enforced it*. It is now `[cond-bool]`, checked on `if`/`elif`, `while` and
the new branch heads, per **leaf** of a `&&`/`||`/`!` condition so the
diagnostic lands on the operand that is wrong, and lenient on `Unknown`
and `Nothing` [type-unknown-lenient]. `Bool?` is rejected too — which way
`None` should decide is exactly the guess the rule refuses to make, and
guessing is how Kotlin/Rust divergence gets in. Nothing in `std/`, the
corpus, the inline test sources or the specs had to change: the whole
suite went green on the new rule unmodified, which says the language was
already being written as if the rule existed.

Lowerings: Kotlin has the same construct, so the shape survives —
`cond -> { … }` arms closed by `else -> { … }`, an *expression* in value
position with no `else null` filler [kt-when-cond]. Rust has no
subject-less `match`, so it emits the `if`/`else if`/`else` chain it is,
reusing `if`'s statement and value paths [rs-when-cond]; a
`match () { () if cond => … }` was considered and rejected (a scrutinee
that means nothing, reading worse than the chain the source already is).
Verified end to end: one demo — value and statement position, `is` heads
narrowing their branch and the `else`, a chain returning from every
branch, and one nested inside a subject `when`'s arm — printing
byte-identical output under `kotlinc` and `rustc`.

**E3 step 3 landed 2026-09-04: effects on fn types, threaded into the
value.** Fn-type effect lists were parsed and dropped; now they are part of
the type and mean **a requirement the caller of the value supplies**, not a
capability the value carries (user decision). So a lambda body performs the
effects *its type* declares rather than whatever its enclosing scope has,
and both backends pass the effect *into* the closure as a leading parameter
instead of capturing it.

**The fusion's structural cut is lifted.** The one program shape Rust lost
to Kotlin — an effect-using fn value passed to a callee that needs an effect
too — compiles and runs: with nothing captured, the closure holds no borrow
to overlap the call's own. Its codegen-error test is now a compile-and-run
test, and the same source runs identically on both backends.

**The user added an inference rule that removed the annotation tax:** a fn
*inherits* the effects of its fn-typed parameters ("we only give the
parameter `f` to `twice` so that it can call it"). So
`fn run_it(f: (s: Str) [Logger] -> Str)` needs no list of its own — while its
*callers* must still supply `Logger`, which is mechanically necessary, since
that is where the value comes from at run time. Inheritance reaches through
qualifiers (`once () [Logger] -> None`), optionals and unions, and it is part
of the callee's contract at call sites, so it had to be added to
`check_callee_effects` as well as to the fn's own environment.

**Two sub-items dissolved rather than shipped.** "Forbid escape" is
unnecessary: a fn value carries no capability, so storing or returning one is
safe and the error surfaces where it is *called* without its effects — the
same shape as `defer`'s capture question in step 1. And a `use` in a fn
type's effect list is rejected instead of represented: registering a handler
is local to a body, so a lambda may `use` exactly when the function
containing it may.

Decided with it: **variance** (fewer effects fit where more are expected — a
pure lambda is handed the effect and ignores it, never the reverse) and
**inference** for un-annotated lambdas (from a visible body, which
[decl-explicit] permits). Where a fn type *is* written, its list drives
emission too: a pure lambda in an effectful position must still take the
parameters the caller passes, which a hand-written Rust probe made obvious
before any code was generated.

The sweep cost four test programs (`ONCE_DEMO` and `FUSION_MIXED_DEMO` on
both backends) and nothing in `std` — no std function takes a fn-typed
parameter. Two bugs fell out: a bare *identifier* argument was checked
without its expected type, so a named fn passed by value never saw the fn
type it had to adapt to; and the emitters' effect lookups searched
outermost-first, so a lambda's own effect parameter was shadowed *by* the
enclosing fn's value instead of the other way round.

**E3 step 2 landed 2026-09-04: `throw` + the intrinsic `try` on both
backends.** Non-resumption works end to end: a fn that may throw declares
`[Throw<M>]` and keeps its own return type, `throw(m)` returns `Nothing` so
the frames in between stay silent, and `try { ... }` — a compiler
intrinsic, not an effect — yields `Ok T | Thrown M`. Four user decisions
shaped the open questions (all 2026-09-04): **`M` is the union** of the
body's message types (chosen for consistency with `if`/`when` branch types,
with generated code wrapping only when a union is present), a **`try` whose
body cannot throw is an error** rather than `Thrown None`, **`main` may not
declare `[Throw<M>]`**, and `Throw`/`Thrown` are **declared in std**
(`std/core/throw.sv`) with the compiler knowing only their names.

The roadmap said to hand-write and run the three deciding Rust shapes first;
that paid for itself twice. It confirmed `?` on `ControlFlow` is stable and
that a may-throw call inside a loop stays a loop (no trampoline — the reason
CPS-splitting was rejected for this rung). And it showed the sketch's
closure lowering for `try` is the wrong shape: a closure would capture the
fn's effect parameters, so **`try` is a labelled block** instead
([rs-try-label]) — no captures at all. The price is that `?` cannot be used
inside a `try` body, so a may-throw call there becomes an inline `match`
that breaks the label. The same `match` form is what lets a deferred release
run on the throw path, since `?` returns without running the splice — which
is the third shape, and the concrete payoff of building `defer` first.

Kotlin diverges in mechanism, as expected: the JVM's unwinding *is* the
propagation, so `throw` throws a generated stack-trace-less
`salvo.ThrowSignal` and an intermediate frame does nothing at all. One
consequence was not obvious and is worth remembering: **the thrown arm has
to be chosen at the `catch`, not at the throw.** Rust wraps a message into
the delimiter's union arm at the propagation site; the JVM has no such site,
and a throwing frame cannot know which `try` will catch it. So the signal
carries the Salvo type name of the message as a `tag` and the delimiter
dispatches on it, with an `else -> throw __signal` rethrow for a signal from
outside its set ([kt-throw-signal]). Comparing Salvo type *names* rather
than JVM classes keeps it erasure-proof.

Verified by compiling and running the same program on both: the value path,
the throw path with a linear handle released by `defer` *on it*, two message
types meeting at one delimiter (`Thrown (Str | Int)`), a may-throw call
inside a loop, and a nested delimiter that does not swallow the outer throw
— byte-identical stdout under `rustc` and `kotlinc`.

Two latent bugs fell out of the work, both fixed. `TokenKind::symbol()`
carried a hand-maintained copy of the keyword list and `unreachable!()`d on
anything missing from it, so *any diagnostic* mentioning `defer` or `try`
panicked the compiler; it now resolves through `KEYWORDS`, the single source
of truth. And the Rust backend only hoisted effect-reaching call arguments
under the *fusion*, so two effectful calls in one expression (`outer(inner(1))`,
or the natural `report(try { … })`) emitted `E0499` — a live parity
divergence, since Kotlin accepts it. Hoisting is now per-call and general
([effect-args-hoisted]).

**Type arguments joined the "no trust" list (2026-09-04).** A generic
call's type arguments must be *determined* — by the arguments, an explicit
list, or the expected type ([call-type-args], user decision A). What the
compiler knows must be visible at the call, so Salvo does not look forward
to a later use the way rustc does; and because the checker now always has
the arguments, a backend can spell an element type out where its own
compiler cannot infer it [backend-intrinsic], which is how Kotlin's list
constructors get theirs.

**The checker no longer takes anything on trust (2026-09-03).** Members
must be declared, not assumed: unresolved calls and dot-calls, calls on
non-fn values, fields on non-structs, `[]` on non-arrays, and `for` over
non-iterables are all errors ([call-resolve], [field-resolve],
[index-resolve], [iter-resolve]). Target-language features are reached by
declaring them; generics are opaque for want of bounds. The remaining
leniency is inference alone ([type-unknown-lenient]).

**General-leftover sweep (2026-09-02, this session).** Eleven of the
non-ownership leftovers were worked through; four turned out to be
*stale* (the capability was already there, just untested — now pinned
with tests), the rest were implemented. Landed: precedence-aware binary
rendering in the Kotlin emitter; iterator-body `return` retargeting
through value-position lowerings (`StmtCtx` is now emitter state, with
lambda bodies as the barrier); aliased imports of mangled qualified
overloads; the new `[mod-collision]` rule (non-fn name collisions are
errors, not last-win); binding widening in `unify` (with the deliberate
no-occurs-check rationale recorded); type-directed dispatch for
unchecked call sites, ambiguity now a codegen error; the new
`[effect-member-generics]` rule (member generics bind per call; Kotlin
renders them on the interface, Rust rejects them loudly); effect
environments keyed by checker `Ty` on both backends (new
`Checked::fn_effects`); and the new `[lsp-definition]` rule
(go-to-definition via `ModuleScope::def_sites` + `Checked::def_refs`).
Two real bugs fell out of the probes: a stray `eprintln!` debug print in
the checker, and the Rust emitter rendering fn-type `let` annotations as
`impl FnMut(...)` (invalid Rust). Four items were escalated as
**DECISION**s for the user; three came back decided and are now
implemented (std array functions; optionals rejected at operators and in
interpolation), leaving `when` on non-identifier subjects and the wider
operator-typing rules open.

**E1 handler dependencies landed on both backends 2026-09-04.** A handler
may now declare a dependency as a **constructor parameter of effect type**
([effect-handler-deps]) — declared on the *handler*, per the user's
interface-design objection to putting it on the effect. What the checker
does: member bodies get the dependency's effect in their environment (so
`ConsoleLogger.log` may call `println`), the `use` site resolves each
dependency from the enclosing scope and reports one that is missing
("register one before it"), dependencies are excluded from the constructor
*argument* count (the compiler supplies them, so `use ConsoleLogger()`
takes none), and a handler depending on the effect it implements is
rejected outright. Resolved instances land in a new `Checked::use_deps`
table.

Kotlin emits it by **injection**: the dependency stays the `private val` it
already was, member bodies resolve that effect to the *field* rather than a
leading parameter (an `override` signature must match the interface), and
the `use` site passes the handler from scope —
`ConsoleLogger(console)` — with callers of the outer effect never
mentioning it. Verified end to end with kotlinc.

The Rust backend fuses instead (same session, see below): capturing the
dependency in the handler is exactly B1's exclusivity trap (`E0499`) where
Kotlin shares freely, so the handler keeps no reference and the fusion hands
the dependency in per call ([rs-effect-fusion]).

**Parity bug found and fixed 2026-09-04: handler state stores.** Chasing
the last E1 item (validating handler bodies against their member contracts)
turned up a *demonstrated* backend divergence, not just a missing check.
This program was accepted with no errors and printed **2 on Kotlin, 1 on
Rust**:

```
effect Sink {
    fn keep(list: Mut List<Int>) -> None    // promises it back => list: Mut
    fn size_kept() -> Int
}
handler Bin of Sink {
    held: Mut List<Int> = mutable_list()
    fn keep(list: Mut List<Int>) -> None => list: Mut { held = list }
    fn size_kept() -> Int { return size(held) }
}
// caller: keep(xs); add(xs, 2); size_kept()
```

Two defects, both needed for the divergence. First, `held = list` was
treated as a *fate link* rather than a store, so the link outlived the call
that made it — Kotlin represents that (aliasing), Rust cannot, so it
silently cloned mutable data, which the parity principle explicitly forbids
as a strategy. Second, handler member bodies were **never validated**
against their declared lists, because the deduction pass collected only
`Item::Fn`; the contract said "kept" while the body moved.

Fixed on both sides: a handler state field is now marked as such on its
`LocalVar`, and assigning into one is a store in both the checker and the
deduction walker ([effect-state-store]); and handler members are validated
against their written lists ([deduce-infer]), outside the fixpoint, which is
sound because members' *bodies* never feed other fns' contracts. The
program above is now rejected at the member's own declaration ("deduction
promises `list` back to the caller, but the body moves it"), and the
honest version — the member declaring `=> !…`, moving the list — prints 2
on both backends.

Known remaining gap, recorded in BACKEND_SPEC.rust.md under [rs-effects]:
the Rust backend derives effect *member* parameter modes from the default
kept rule rather than from the member's deductions, so a moved member
parameter still emits as `&mut` plus a clone. That is sound — the checker
consumed the caller's value, so nothing can observe the copy — but it is a
missed optimization and the natural companion to the fusion work.

**Dependency cycles need no check (verified 2026-09-04).** Chasing the
prerequisite the fusion's soundness rests on — a DAG — turned up that the
availability rule already provides it: a dependency must be registered
*before* its dependent, so a cycle cannot be constructed in any order
(tested both ways round; each order fails on the first `use`). One less
pass to write, and the reason is worth remembering: an ordering requirement
on registration is an acyclicity guarantee.

**E1 is complete: the Rust fusion landed 2026-09-04.** Both backends now
run handler dependencies to the same output. Rust builds one *fusion* per
`use` scope which owns the registered handler, implements every effect in
scope, and hands a dependent handler's member its dependency from a
disjoint field borrow. The full emission — and the reasoning that each
shape rests on — is under [rs-effect-fusion]; the parts worth carrying
away, because they cost the most to re-derive:

- **A fused fn parameter must be a Sized generic, not `dyn`.** A fn needing
  `[Console, Logger]` takes `__fx: &mut __Fx` with `__Fx: Console + Logger`;
  only a fn needing exactly one effect keeps `&mut dyn E`. The reason is
  *subset forwarding*: a fn must be able to hand its fused value to a callee
  needing fewer effects, and `&mut dyn Conj_A_B_C` → `&mut dyn Conj_A_B` is
  not expressible — trait upcasting reaches supertraits only, and a
  conjunction of two is not a supertrait of a conjunction of three. The
  originally recorded plan preferred the `dyn` form for code size; it does
  not work, and this was found by compiling it.
- **Nested scopes chain, they do not rebuild flat** (revises the
  2026-09-04 decision, which was recorded before implementation). Flat
  rebuilding over the outer scope's *handler locals* fails twice: effects
  inherited from a fn's parameter are not locals at all (there is only one
  fused value), and an inner fusion re-borrowing the same locals makes the
  **outer** fusion unusable after the inner block — the very case nesting
  exists to allow. Chaining through a single
  `__outer: &'a mut dyn <E | Conj>` field keeps what the decision wanted
  (dependency threading is identical at every depth) and makes lexical
  nesting equal borrow nesting. It also yields a small theorem: a
  dependency is *always* in `__outer` and never a sibling field, because it
  had to be registered first — the acyclicity guarantee doing a second job.
- **`dyn` in that one field is load-bearing**: it keeps monomorphization
  finite when a recursive fn registers a handler and recurses (the fusion
  type maps to itself instead of nesting forever).
- **The fusion is generic over the handler it owns** (`__H`), which is what
  lets a *generic* handler be fused without re-deriving its type arguments
  from the effect instance.
- **Dependent handler bodies move into a generated `__Impl_H` trait**
  (`&mut self` kept, so `self.state` still works). It is only ever a bound,
  never `dyn`, so its methods may be generic — which is how a two-dependency
  member gets a Sized fused value, via a per-handler `__Deps_H` adapter over
  the provider.
- **Two new hazards appear only under the fusion**, both because one value
  now carries every effect: member calls need UFCS
  (`Random::<i32>::next_random(recv)`) or they are ambiguous, and an argument
  that itself reaches the fused value must be hoisted into a temporary or
  the call borrows it twice (`E0499`).

The gate is program-wide and narrow: a program where no handler declares a
dependency keeps the per-effect `&mut dyn` parameters untouched, which is
why none of the existing goldens or e2e tests moved.

**Two pre-existing defects surfaced while probing the fusion** (both
reproduced with the fusion *off*, so neither was caused by it). The first is
fixed; the second needs a language call.

**Fixed 2026-09-04 — type arguments must be determined ([call-type-args],
user decision A).** The symptom was a backend divergence:
`let xs = mutable_list()` emitted `val xs = mutableListOf()`, which kotlinc
rejects, while rustc infers `vec![]` from a *later* `add`. Diagnosis showed
the template was not the problem — the **checker** never determined the
element type either (an unbound callee generic became `Ty::Unknown` in the
result), so no `define fn` could have interpolated one. Worse, the same hole
swallowed real mistakes: `add(xs, 1)` followed by `add(xs, "two")` on that
list was accepted with no error at all.

Now a generic call's type arguments must be determined — by an explicit
list, by the arguments, or by the **expected type** (a `let` annotation, the
enclosing fn's return type, or a concrete parameter the result flows into,
which now reaches nested calls). One that appears in the *result* type and
that nothing determines is an error naming both remedies. Salvo deliberately
does not look forward to a later use the way rustc does: what the compiler
knows must be visible at the call. Nothing in the repository needed
rewriting — every existing `mutable_list()`/`list()` was already annotated
or given elements.

**Also landed with it: the era's `define fn` templates gained type-argument
interpolation** (user request "I want the templating to be consistent").
`${T}` resolved against the define's own type parameters using the checker's
new `call_type_args` table, exactly as `define type` templates already did.
The templates are gone (2026-09-05), but `call_type_args` outlived them: it
is what the backends' intrinsic lowerings read. std's Kotlin list defines
used it
(`mutableListOf<${T}>(${...elems})`), so the element type is *always*
spelled out rather than left to kotlinc's context. Rust's templates keep
`vec![]` deliberately — rustc infers there, and pinning it would churn
output for nothing.

**And a third hole fell out of the work: handler state initializers were
never type-checked.** `held: Mut List<Int> = "definitely not a list"` was
accepted, because the handler item only validated the field's *type* and
skipped its default expression (struct fields have always been checked).
That is also why the emitters had no types for it, which is how the gap
surfaced: the `${T}` in a state field's `mutable_list()` had nothing to
interpolate. State initializers are now checked against the declared type
like struct defaults [effect-handler].

**Next effects slice: E3 steps 1–3 landed 2026-09-04; step 4 (effect
transformers) is what remains.** Handler *control* beyond "always resumes":
`defer`, then `throw` returning `Nothing` with a compiler-intrinsic `try`
yielding `Ok T | Thrown M`, then effects on fn types threaded into the
value (which lifted the fusion's structural cut) — all three **done**, see
the entries above. Step 4 is transformers: an effect member that runs a
fn-typed parameter with *additional* effects available. The gate step 3 was
supposed to build for it is in place, so the remaining question is the
surface, not the mechanism. Async is explicitly *not* part of this arc: no
silent `async`/`suspend` colouring, and async arrives later as an explicit
effect.

- **Still open: two effects cannot share a member name.**
  `Symbols::effect_of_fn` maps a member name to *one* effect, so declaring
  `emit` on both `Logger` and `Metrics` resolves every `emit` call to
  whichever was collected last (`no handler for effect Metrics<Logger>`),
  and there is no syntax to disambiguate — `emit<Logger>(…)` parses as
  *member* type arguments, not as an effect selection. Both backends fail
  identically and loudly. Deciding this needs a language call: reject the
  collision at declaration ([mod-collision]'s reasoning), or add a
  disambiguation form.

**Two** cuts remain inside the fusion, both reported
([backend-never-wrong]), and both about type arguments the fusion does not
derive: a dependent handler using its own generic parameters in a member
signature, and a `use` whose effect instance is still generic. Kotlin
accepts both (generics erase), so each is a live divergence, documented
under [rs-effect-fusion].

The third — structural, and the only program shape the fusion *lost* to
Kotlin — was **an effect-using function value passed to a callee that needs
one too**, where the closure's captured borrow overlapped the call's own.
**Lifted 2026-09-04 by E3 step 3**, exactly as predicted here: the fused
value is passed *into* the closure instead of captured ([fn-effects]).

**E1 prerequisite landed 2026-09-04: effects and handlers are not data.**
The first buildable slice of the effects arc, and a live
[backend-never-wrong] violation on its own: an effect type in a data
position (struct field, parameter, return, `let` annotation, alias) and a
handler constructor call outside `use` were both accepted by the checker
and emitted **invalid Rust** — a bare trait (`E0782`) and a non-existent
constructor (`E0423`) — while Kotlin happened to render both correctly.
Now rejected with diagnostics naming the legitimate positions and the
`use` remedy ([effect-not-data], [handler-not-value]). The two positions
that legitimately name an effect (a fn's effect list, a handler's `of`
clause) resolve through their own path and are untouched, verified by
compiling and running an ordinary effect program on both backends. E1 will
*re-admit* exactly one of the rejected shapes — a handler constructor
parameter of effect type, its dependency — together with the fusion
emission that can render it.

**Effects arc: E1 strategy settled (user decisions 2026-09-03/04).**
Effect-to-effect dependencies will use **handler fusion (B9)**: effect
interfaces stay dependency-free so user fns are emitted once; a handler's
dependencies are its constructor parameters of effect type (already
parseable today); one *fusion* per `use` scope holds the handlers and
hides dependencies by passing them from its own fields — in Rust via
disjoint field borrows, which is what lets a *shared* dependency work at
all; nested scopes rebuild flat on Kotlin and chain on Rust (revised
during implementation — see E1a); facets are Kotlin-only, since erasure
forbids one class implementing `Random<Int>` and `Random<Double>`.
Mechanism and rationale: [rs-effect-fusion], [kt-effect-fusion]; six
explored alternatives and why they failed: E1a. Consequences worth
knowing: no exclusivity, no runtime failure mode, no duplication of user
code — and neither an immutable/mutable effect distinction nor `Cell` is
needed for testability, since a handler member may mutate a dependency it
receives as a parameter. `Cell` (shared mutable state as an intrinsic
capability qualifier) therefore stands or falls on its own cases —
accumulators across lambdas, memoization, counters — written up in its own
roadmap section.

**Next arc: place-based flow analysis.** **P1 is done** (see the decision
log): flow state is keyed by *places*, `is` narrows field chains *and*
tuple positions (`t.0`, new syntax added in the same session), and
invalidation rides on the fate analysis's event set. **P2** (`when` on
field subjects) was decided *against* — `when` stays variable-only. What
still rides on the substrate: **L5**'s field-disjoint ownership.

**Effect-member deductions reach inference too (2026-09-03).** A follow-up
to the above, from a user question about why the Rust output was still
sound: the checker enforces an effect member's declared deduction list at
the call site (`check_effect_call` builds a contract and runs
`apply_call_contract`), but `deduce.rs` did not — it resolves callees
through `call_fn`, a span→`FnKey` map, and a member has no `FnKey` because
the handler is chosen at run time. So the two disagreed about the same
call: the body could not use an argument a member had taken, while the
*inferred* contract still told callers it was kept.

Nothing miscompiled, for an instructive reason: both under-claimed at once.
Deduce said "kept" and the Rust backend emitted `&String` for the member
too (member deductions do not drive its parameter modes yet), so the
emitted program borrowed end to end — and rustc only rejects
*over*-claiming (use-after-move, moving out of a borrow), never
under-claiming. Fixing one side alone would have produced the over-claim it
does catch (`E0507`), which is why the two halves are worth keeping in
mind together.

The fix is a second lookup path in `deduce.rs`: `effect_member_contract`
finds the member through the checker's recorded `effect_calls` instance
plus the callee name, then applies its written list through the same
`apply_callee` loop resolved calls use ([call-resolve]). Verified: the
example now errors at the caller ("`text` ... was consumed (moved)"), and
with the caller corrected both backends compile and run — Rust emits
`fn forward(sink, s: String)` with `sink.take(&s)`, an owned parameter lent
to a borrowing member. Still open (E1-adjacent): the Rust backend deriving
*member* parameter modes from their deductions, and validating each handler
body against the member's contract.

**Salvo assumes it can see everything (user decision 2026-09-03).** The
checker's interop leniency is gone: a call, field read, subscript or `for`
subject that no declaration justifies is now an error
([call-resolve], [field-resolve], [index-resolve], [iter-resolve]).
Target-language features are reached by *declaring* them — as of
2026-09-05 that means a member of a `platform effect` [platform-effect]
(then `external type` for the type and `external fn` + `define` for
anything you did with it) — and dot-notation still reads like a method call
because it *is* a call to a declared function. Generics fall under the same rule: with no bounds,
nothing about a `T` is knowable, so `value.name` inside `fn f<T>(value: T)`
is an error rather than a promise about future call sites.

Six holes closed, all of which the compiler used to accept silently and
hand to the target compiler: an unresolved bare call (`nowhere()`), an
unresolved dot-call (`text.shout()`), calling a value of known non-fn type
(`n()` on an `Int`), a field on a non-struct (an opaque type, a generic, an
`Int`), `[]` on a non-array, and `for` over a non-iterable.
Each produced code the target compiler rejected — `E0618: expected
function` from rustc, `unresolved reference 'n'` from kotlinc — which was
loud but pointed at generated code the author never wrote, and in Kotlin's
case named a symbol that *does* exist in the Salvo source. That is not what
[backend-never-wrong] asks for.

What remains lenient is **inference, not visibility**
([type-unknown-lenient], rewritten): a type the checker could not work out
stays `Ty::Unknown` so one mistake yields one diagnostic. The one
unresolved *callee* kind left is an effect member, whose handler is chosen
at run time — [deduce-infer]'s lenient borrow now names only that.

Cost of the change: nothing. **No test needed the leniency** — the only
failure in 331 was a deduction test whose premise was a call to
`unknown_interop`, rewritten to pin the effect-member case it actually
covers. Making generics opaque broke nothing either.

**Why (user rationale, 2026-09-03):** the rules the compiler imposes should
be *easy to understand and predictable*. Strictness is welcome on that
basis — one rule for dot-calls beats a rule plus a silent interop
exception — provided the diagnostic makes the problem obvious and, where
possible, the language offers a straightforward remedy (the precedents are
`copy` for shared fate and `discard` for linear obligations). That is the
standard the new diagnostics are held to: each names the remedy (a
declared fn — since 2026-09-05 a `platform effect` member — for a missing
member, `get(collection, index)` for a subscript, "rebuild the tuple" for an
element write) — and where no remedy
exists, it says *that* instead of suggesting a useless one: a type
parameter's diagnostic explains that nothing is known about a `T` rather
than pointing at an accessor that could not help.

The emitters' method-call fallbacks went with it (user decision, same
session): both `emit_call`s used to render an unresolved dot-call as a
native method call, which was the *mechanism* for interop and is now
unreachable from valid source. Rather than leave a path that silently
guesses, each is a codegen error naming the internal inconsistency — the
checker guarantees resolution, so arriving there means a table lost an
entry without reporting it. Fallbacks stay only where a table may
legitimately have no entry (an `Unknown`-typed expression still has to
render).

**Tuple indexing added 2026-09-03 (user decision).** `t.0` reads a tuple
element by constant position ([expr-tuple-index]) — the projection P1a had
asked to narrow but which the language could not express. It is a
`proj::Index` place, so it narrows, invalidates and merges exactly like a
field, in any combination (`p.pair.0`, `t.1.0`). Decisions inside it:
elements are **read-only** (qualifiers, `Mut` among them, cannot apply to a
tuple [qual-union-arm], so there is nothing to assign through — the error
names the rebuild remedy), out-of-range and non-tuple bases are **errors**
rather than lenient `Unknown` (the index is program text, so it cannot be
interop-dependent), and no numeric suffixes.

The subtle part was lexical: numbers own their decimal point, so `t.0.1`
would lex as `t` and the float `0.1`. Rather than un-parse a float in the
parser — which cannot recover `.0.10` from an `f64` — the *lexer* now
refuses a fraction directly after `.`, where nothing else in the grammar
can put a numeric literal (paths and dot-calls take identifiers, spread is
one `...` token). `t.0.1` is then two ordinary index tokens. Kotlin maps
the projection onto `Pair`/`Triple` components ([kt-tuple-component]);
Rust uses its own `t.0` ([rs-tuple-index]).

**P1 landed 2026-09-03: flow analysis is keyed by places, and fields
narrow.** `is` now narrows a *place* — a variable or a field chain out of
one — so after `if p.surname is Str` the field itself reads as `Str`
([flow-place]), which is what the optional-strictness rules
([interp-no-none], [op-no-none]) had been forcing into the `is Str name`
binding form. Three user decisions shaped it:

- **P1a — which projections narrow: field chains** (`h.a.b`). Array
  elements never narrow: an unknown index may alias any element, and
  restricting to constant indices would invite the expectation that
  `arr[i]` narrows too. The `Place` type carries `Field`, `Index` and
  `Element` projections from the start so place-based *ownership* (L5)
  reuses it. The user asked for constant *tuple* indices as well —
  which turned up a gap: Salvo had no tuple element access at all (`.`
  required an ident, so `t.0` was a parse error; tuples were
  destructure-only). **The user added it the same day** — see the
  tuple-indexing entry above — so constant indices narrow too.
- **P1b — kept-immutable calls preserve narrowing.** Only a call that
  keeps the value *mutably* (a `Mut` parameter) invalidates
  ([flow-place-invalidate]); a read-only call cannot mutate, so a
  logging call no longer costs you the fact. This reads the same
  deduction facts D1 established, so it is precision without new
  machinery — and it keeps the rule consistent with D5's reason for
  exempting provenance qualifiers.
- **P2 — `when` stays variable-only** (against the recommendation): field
  subjects keep the [when-union-subject] error, since `if … is` covers
  them now. That dropped the arc's top-ranked motivation; the
  strictness-gap customer and L5's shared substrate carried it.

Design notes worth keeping: place facts live *inside* the root's
`LocalVar` (a `place_narrows` list keyed by projection path), which is
why `snapshot_narrows`/`restore_narrows`/`merge_fallthrough` stayed the
single source of truth (the S1 gotcha) and why an event on a root
invalidates everything below it for free. A fact survives a join only if
every fall-through path agrees on it exactly — falling back to the
declared type is always sound. Restoring a branch's narrows must *not*
resurrect a fact the branch invalidated, the dual of the
consumed-stays-consumed rule. The four duplicated "mutation through a
projection" provenance loops collapsed into one `fate_mutation_through`,
so no invalidation site can be forgotten.

The implementation also found a real **backend-parity bug** the feature
would have shipped: Kotlin refuses to smart-cast a *property*, and
`canbe Mut` struct fields emit as `var`, so a narrowed nullable field
read produced Kotlin that did not compile ("smart cast to 'String' is
impossible"). Narrowed nullable field reads now emit `!!`
([kt-narrow-field-assert]) — the assert can never fire, since the
checker invalidates the fact on any mutation.

**Bodyless declarations are explicit (user decision 2026-09-03).** No
inference for anything the compiler cannot see: a bodyless fn must declare
effects, deductions, *and* return type; effect members must declare return
type and deductions [decl-explicit]. (At the time that covered
`external`/`intrinsic` fns and required every `define fn` to match an
external one-to-one; since 2026-09-05 `intrinsic fn` and effect members are
the only bodyless forms, and a bodyless `fn` anywhere else is a parse error
[decl-body].) This removed the last inference-from-nothing guess (and
with it the `Mut`-parameter proxy D1 needed), and fixed a real ownership
bug it had been hiding: std's `add` did not consume its element, so
`add(xs, h)` then using `h` compiled on Kotlin and was rejected by rustc.
New roadmap sections: **E1** effect-to-effect dependencies (the effect
declares them, handlers mirror exactly) and **E2** heuristics for
validating external declarations against their define templates.

**Backwards compatibility is not a requirement (user decision
2026-09-03).** The language is experimental and its features are still
being worked out, so compatibility machinery would only get in the way:
syntax and rules change outright, with no deprecation periods, no
grammars that accept both spellings, and no version gates. The
obligation that replaces it is a *sweep*: every affected example gets
rewritten in the same change (`std/`, test corpora, inline `.sv` sources
in Rust tests, the spec documents, README, and the syntax references in
this file — it is a handoff document, not an archive, so stale syntax in
prose is a bug and the change belongs in this decision log instead).
An example that no longer works is *deleted*, not preserved — history
belongs in this decision log, not in a directory of dead programs — and
anything that cannot be rewritten confidently (ambiguous intent under the
new rules, or a site that merely *looks* like the changed construct) is
flagged for the user to update by hand. A transitional *error* naming the replacement is
permitted but not expected (it is a diagnostic, not compatibility);
still accepting the old form never is — and the `canbe` rename's own
transitional error was removed on the user's call, so a plain parse
error is the default. Recorded as an invariant in AGENTS.md with a step
in the task workflow.

**Qualifier subjects landed 2026-09-03 (roadmap D5).** Qualifiers now say
what their claim is *about*: `qualifier Q of T` is a **state** claim about
the value's contents (the default, unchanged), `provenance qualifier Q of
T` is a claim about where the handle came from ([qual-subject]).
Provenance is exempt from D1's stripping — the rule that a mutating call
invalidates unlisted qualifiers is sound only for claims about contents —
which is the whole point: an `Authenticated Request` no longer loses its
tag to a logger that takes `Mut`. Provenance is mint-only (no body, so no
`qualifies` and no field overrides), droppable, survives storage, and
composes without `with`. It erases like every qualifier, so no backend
changed; the visible consequence is which overload the checker picks. The
compiler's own capability qualifiers (`Mut`, `Linear`, `once`,
`ReadOnly`) stay intrinsic — each needs a representation choice, a flow
rule, a subtyping direction or a restricted position that no user
declaration could supply.

**Dot-names landed 2026-09-03 (roadmap N1).** Structs and qualifiers can
be declared `Ns.Name` where `Ns` is a struct in the same file
([name-dot]), giving the Kotlin wrapper-type idiom (`Environment.Id`)
without nested declarations — Kotlin emits a nested class, Rust flattens
to `EnvironmentId`. Casing became a language rule ([name-casing]): types
uppercase, values lowercase, module paths lowercase (so an uppercase
`.sv` file or directory name is a compile-time error). That rule is what
makes `Environment.Id { … }` decidable against `person.name`, and it
retired the parser's old "lowercase after `is` means a binding"
heuristic. Nothing visible may carry the concatenated spelling, checked
scope-wide because Rust flattening and overload mangling share it. No
existing Salvo source violated the casing rule.

**`canbe` replaces `with` at opt-in sites (user decision 2026-09-03).**
Auto-qualifiers on struct and type declarations and the per-type-parameter
linear opt-in are now spelled `canbe` (`struct Person canbe Mut`,
`external type List<T> canbe Mut`, `struct FileHandle canbe linear`,
`fn hold<T canbe linear>`), which states the optionality the clause
actually carries [canbe-optin]. `with` keeps exactly one job: qualifier
*compatibility* (`qualifier Old of Person with Surname` [qual-with]) —
co-application of two qualifiers, which `canbe` would misdescribe; no
better word was found, so the overload is gone but `with` stays. The two
clauses are simply independent syntax now — a transitional rename
diagnostic (and the test pinning it) was implemented and then removed on
the user's call, since nothing outside this repository writes Salvo yet.
Labels renamed with the
syntax: `[type-with-mut]` → `[type-canbe-mut]`, `[linear-with]` →
`[linear-canbe]`; AST field `FnDecl.generic_with` → `generic_canbe`
(uniform snapshot churn). The same analysis produced roadmap **D4**:
`is` on a union subject always means arm identity, so a *predicate*
qualifier can never be tested against a union-typed value — fixing that
needs qualifiers over unions, and `is` itself stays as-is (user decision
2026-09-03).

**D1 landed 2026-09-02: deductions are exhaustive by default.** A
confirmed unsoundness — qualifiers surviving calls that invalidate them,
reproduced with a `clear` that emptied a list while the caller kept
believing `NonEmpty` — is fixed. `[list: NonEmpty Mut]` now means *only*
those apply afterwards (including dropping qualifiers the callee never
declared), `[list: -Q]` is the delta form for "everything else
preserved", `[list: Nothing]` is moved, and a parameter the body mutates
may use neither keep-all nor a delta. D2 holds `+Q` asserts and their
possible unity with constructive qualifiers; D3 covers refinements — the
general answer to D1's accepted over-strictness, after analysis showed
blanket qualifier *polymorphism* cannot be sound.

## History (condensed)

The milestone-by-milestone detail that used to live here has been folded
into the spec documents; what follows is the decision log — the choices
that still shape the code, and where to look for the mechanics.

- **M0+M1 — CLI + parser.** Hand-written lexer + recursive-descent parser
  (deliberate: newline-terminated statements, template/interpolation
  lexer modes, struct-literal-vs-block ambiguity, and speculative parses
  make grammar generators a poor fit). Tokens carry `newline_before`;
  parser recovers at item/statement level. String interpolation re-lexes
  `${...}` fragments with spans shifted back into the file.
- **M2 — Kotlin codegen, end-to-end verified** (kotlinc compiles the
  output; tests assert exact stdout). Founding invariant
  [backend-never-wrong]: unsupported constructs are codegen *errors*,
  never silently wrong code. Remaining deliberate cuts: multi-spread
  struct literals, early `return` inside lambdas, tuples beyond
  Pair/Triple.
- **M3 — Typechecker + unions.** `Ty` model (`Qualified` with sorted qual
  sets, `Union` flattened/deduped in declaration order), per-file scopes,
  and the side-table architecture (`Checked`: `expr_ty`, `repr_ty`,
  `coerce`, `is_tests`, `call_fn`, …) the emitters consult. Two founding
  decisions: the checker is *lenient* (anything untypable is
  `Ty::Unknown` and passes through — then justified by Kotlin interop,
  narrowed twice since: to inferred types only, and finally to inference
  alone once members had to be declared [call-resolve]), and union arm
  identity is *positional over the declared type's non-`None` arms*
  [union-arm-identity]. Kotlin unions lower to generated sealed wrappers
  (`UnionN`), `None` arms to outer nullability.
- **M4 — Qualifiers.** User decision replacing the old spec: the `as`
  effect and value-level `as` expressions were removed; constructive
  qualifiers are built exclusively through constructor functions
  (`fn ok<T>(value: T) -> T as Ok`), same-file rule, simple return types
  [qual-ctor-fn] [qual-ctor-same-file] [qual-ctor-simple]. Predicate
  qualifiers lower to `{Q}_qualifies` calls at `is` sites; struct-field
  overrides cast+assert. Overloads identical after erasure get
  deterministic `__Qual` name mangling. Qualified union groups
  (`Ok (A | B)`) wrap whole-arm first, then coerce as the bare union.
- **M5 — Effects.** The checker owns effect semantics: per-fn effect
  environments seeded from declared lists, grown by `use`, truncated at
  block boundaries; handler generics *inferred by unification* from `use`
  constructor args; effect-member disambiguation via explicit type args →
  argument types → expected type. The emitters prefer the checker's
  effect tables and fall back to string-keyed environments only in
  unchecked contexts (see "Current architectural facts").
- **M6 — Deductions + loops-as-values.** Loops are expressions
  [while-value] (body tail / `break value` / `else` join; no `else` means
  optional). Deductions (`deduce.rs`): user decision — a deduction list
  is interpreted *relative to the callee's declared parameter qualifiers*
  (a call removes exactly `declared − kept`); inference is a
  whole-program fixpoint from an optimistic start (facts only removed →
  terminates); written lists may be stricter than the body but never
  looser [deduce-syntax] [deduce-infer].
- **M7 — Polish.** Only reachable modules are emitted [mod-used-only]
  (name-usage reachability, deliberately conservative); per-module Kotlin
  packages + generated imports [kt-package] [kt-imports]; interop coverage
  checked upfront for `core.*` and at reference sites (the `define` era's
  rule, since deleted); backend-native companion files copy verbatim
  [backend-companion]. Decision under [type-array]: `T[]` stays
  `Array<T>` in Kotlin (no primitive-array specialization without
  profiling data).
- **M8 — Rust backend.** The point of the design: deductions drive
  ownership [rs-borrows] — omitted parameter = moved (by value), kept =
  borrowed (`&mut` when `Mut`); expressions emit owned by default
  (borrowed reads clone); no emitted signature returns a reference, so no
  lifetimes exist anywhere. User decision: `Mut` generalized to a
  language-level qualifier any type opts into with `canbe Mut`
  [type-canbe-mut]; backends map it per type (`Mut inline:` define
  sections). Unions are generated enums; effects are traits with
  `&mut dyn` threading; iterators are lazy on both backends (`Iter<T>` is
  a generated factory type [rs-iter-lazy]; it was `Vec<T>` and eager
  until 2026-09-05); `WrapOption` coercion added
  because optionals are physical in Rust and transparent in Kotlin
  [type-nullable]. Crate layout: main-declaring module is the crate root
  with `#[path]` mounts [rs-crate].

### Post-M8 — tooling and flow analysis (decision log)

- **A producer may not take a mutable parameter (user decision 2026-09-08)** —
  `[iter-mut-param]`. A parity hole rather than a strictness gap: a producer's
  parameters are captured by the pass, and Rust captured by clone (a private
  copy — the caller saw nothing and every pass started fresh) while Kotlin
  handed the reference over (the caller's collection mutated, and successive
  passes accumulated). The checker now rejects a `yield` fn parameter whose
  type is transitively mutable, by the same `Mut`-at-any-depth test
  `[fate-move-mode]` uses, with a diagnostic naming the remedies that work
  everywhere: yield the values and let the consumer collect them, or reach the
  outside through an effect. The two alternatives were copying per pass on both
  backends (rejected as the trap: it agrees everywhere and quietly discards the
  writes the caller expected) and sharing on both (what the author expects, and
  on Rust it *is* the `Cell` work — queued there as that roadmap's sharpest
  customer). Raised by the user alongside it and recorded rather than decided:
  a producer returning `once Iter<T>` may be able to take one after all, since
  exactly one pass exists and the rest is shared fate `[fate-link]` — which
  wants I2c's representation split first.
- **The same rule covers a producer's *callbacks* (2026-09-08)** — a lambda
  handed to a producer's fn-typed parameter may not capture mutable state, for
  the same reason and with the same remedies. Both flavours are refused (the
  closure writing through the capture, and merely reading mutable data through
  it), which needed two flags exported on `Checked::lambda_captures`. Writing
  the diagnostic exposed a second defect: a producer's callback is declared
  `impl Fn + 'static` and was emitted as a *borrowing* closure, so every
  capturing lambda handed to a producer was E0373 on Rust — immutable captures
  included. Fixed with `move`, and covered by the first test in the suite that
  passes a capturing lambda to a producer.
- **I5's combinator surface (user decision 2026-09-08)** — eager by default;
  `map_lazy`/`filter_lazy` for the lazy pair; `map_to`, which maps into a
  caller-provided collection passed as the first argument with an `?add`
  implicit parameter, so the destination is anything with an `add` rather than a
  `List`.

- **Optionals never reach operators or interpolation (user decisions
  2026-09-02)** — `[op-no-none]`, `[interp-no-none]`. Both were parity
  holes, not just strictness gaps: Kotlin compares against `null` and
  prints `null`, Rust rejects the `Option` (`Display` unimplemented, type
  mismatch on comparison). The checker now errors on a possibly-`None`
  operand of `+ - * / %` and `< > <= >= == !=`, and on interpolating a
  possibly-`None` value; `None` itself is rejected in both positions.
  Remedy is narrowing (`is` / `when`, taking the binding for a
  non-variable place) or `!`. Deliberately *not* covered: `&&`/`||`,
  because operand typing beyond `None` (numeric towers, promotion, `Bool`
  requirements) is a separate open decision, and value-position `&&` goes
  through a different path than `analyze_cond`. Two pieces of evidence
  that the leniency was costing us: the loops demo's Kotlin and Rust
  sources had silently diverged (`${capped}` vs `${capped!}`) because
  only Rust complained, and three LANGUAGE.md nullability examples
  interpolated `${person.surname}` after `person.surname is Str`,
  relying on field narrowing Salvo does not do — all now use the
  spec's own `is Str surname` binding idiom.
- **Doc comments and richer hover (user request 2026-09-03)** —
  `[doc-comment]`, `[doc-markdown]`, `[doc-symbol-ref]`,
  `[doc-struct-fields]`, `[doc-hover-narrowed]`. A declaration's docs are
  the run of own-line `//` comments *directly* above it — no `///`, no
  attribute, no separate syntax; one blank line ends the run. Decisions
  made along the way, all reversible:
  - **Comments are collected, not tokenized.** The lexer already knew
    exactly what a comment was and threw it away; it now records
    `LexResult::comments` (span, text, `own_line`) and the parser attaches
    the block above each declaration *by line number*. Keeping comments
    out of the token stream means the grammar is untouched — no skip logic
    in a hundred parse functions. `own_line` is what makes
    `a: Int, // note` document nothing.
  - **Docs live on the AST** (`docs: Vec<String>` on fn, struct, field,
    qualifier, effect, handler and type declarations), which is what the
    parser snapshots now show. The alternative — extracting them textually
    in the LSP — would misread a `//` inside a string literal on the
    preceding line.
  - **Hover is markdown throughout** (`MarkupContent`, not the deprecated
    `MarkedString`): a fenced `salvo` block with the signature or type,
    then doc sections separated by rules. The `ReadOnly` presentation
    moved into the same shape.
  - **`[symbol]` resolves locally first**, then against any declaration in
    the program, and renders as a *link* to the declaration; unresolved
    references are left verbatim so bracketed prose is never mangled.
    Deliberate simplification: the search is name-based over the AST
    rather than import-visibility-exact, which [mod-collision] makes
    almost always equivalent — `Resolution` borrows `Program`, so the
    analysis result cannot carry it.
  - **Hover now answers on declarations, not just uses.** Declared names
    (parameters, handler state, `let` patterns, `is`/`for` bindings) record
    their type in `expr_ty` at their own name span. Before this, hovering
    the `items` in `fn f(items: Mut List<Int>)` said nothing.
  - **The narrowed/declared pair needed no new table**: `Checked::repr_ty`
    already held the declared type for narrowed identifier uses (the
    emitters' re-wrapping fact), which is exactly the "declared as X"
    line. Hovering inside `if items is NonEmpty` shows
    `Mut NonEmpty List<Int>` over `Mut List<Int>`.
  - Struct hover lists *every* field with type and default-as-written, not
    only documented ones, so it shows the shape of the struct.
  - **Follow-up, same day: fields and members hover too.** The two gaps
    left above are closed. Hover (and go-to-definition) now reach *nested*
    declarations through one search over a module's items (`decl_at`,
    matching by name span): struct fields, handler state fields, qualifier
    field overrides, and the member fns of effects, handlers and
    qualifiers. Each renders its own declaration line, its docs, and what
    declares it ("Field of struct `Person`.", "Member of effect `Log`.").
    - Field *accesses* needed a new table: `Checked::field_refs` maps the
      field-name span in `base.field` to the field's declaration, built
      from the struct's `DefSite` (for the file) plus the `FieldDecl`'s own
      name span. It feeds go-to-definition as well, which fields never
      had. It points at the struct's field even under a qualifier field
      override — the override refines the field, it does not replace it.
    - `fn_signature` split into `fn_decl_signature(decl, inferred)` so
      members can render from the declaration; they have no `FnKey`, so
      they show their *declared* deduction list, which [decl-explicit]
      requires of them anyway.
    - A nested declaration's docs see its owner's names (`own_names`), so a
      handler member can write `[count]` for the handler's state
      [doc-symbol-ref].
    - Fallout worth noting: the feature immediately caught a real
      mis-attachment in `std/core/result.sv`, where an edit had turned the
      blank line between the module header and the qualifier's docs into a
      `//` line — so the whole header had become `qualifier Ok`'s
      documentation. Every std file's attachment was then checked.
- **Written names in type positions must resolve; `Ok`/`Err` move into
  std (user decisions 2026-09-03)** — `[name-resolve]`,
  `[qual-result-tags]`. Found through a TODO in
  `experiments/refinements.sv`: `if n is Ok` on `Ok Int | Err Str | None`
  reported "this check can never succeed", and the real problem was that
  `Ok` was never declared. Nothing checked qualifier or type *names*, so
  `Ok Int` in the return type quietly became a qualified type with an
  unheard-of qualifier, while `parse_check` — asking `is_qualifier`,
  getting no — read the same `Ok` as a *base type* that no arm matched.
  Then the empty match set narrowed the subject to `Nothing` and a bogus
  "consumed (moved)" error landed on the next use. Three errors' worth of
  noise, none of them the missing declaration.
  - The checker now rejects any written name in a type position that
    resolves to nothing, in either namespace (base types vs qualifiers),
    with import suggestions [diag-import-suggest] and a wording hint when
    the name exists in the *other* namespace. Reported from
    `validate_type` / `validate_quals` / `parse_check` at declaration
    sites, which is where [qual-of] already put applicability checking —
    lowering runs repeatedly and stays silent.
  - This narrowed [type-unknown-lenient] a first time: leniency is about
    types the checker cannot *infer*, not about names the author wrote.
    (It was narrowed again on 2026-09-03, when members stopped being
    lenient too — see the "Salvo assumes it can see everything" entry.
    At the time of this milestone, an interop type's *members* were still
    pass-through.)
  - An unresolved `is`/`when` check marks the pattern and suppresses every
    verdict that follows from the failed match — "can never succeed", "no
    remaining union arm", non-exhaustiveness, the `Nothing` cascade. One
    error per mistake.
  - Wiring the check up exposed genuinely missing declaration sites:
    struct fields, type-alias targets, effect-member signatures, handler
    ctor params and state fields were never validated at all (the
    [qual-of] rule *claimed* struct fields were). `canbe` clauses now
    reject user qualifiers, which is the `with` confusion [canbe-optin]
    already warns about.
  - Two real bugs fell out: `core.string`'s `char_at(index: Positive Int)`
    used an undeclared qualifier lifted from a LANGUAGE.md example — no
    caller could ever have satisfied it — now plain `Int`; and four
    std-less test preludes never declared `Int`/`Str`.
  - `Ok`/`Err`/`ok`/`err` now live in **`core.result`**, not
    `core.basic` (the user's initial suggestion; deviation raised and
    approved on the dead-code grounds): `core.basic` declares `Int`, so
    every program reaches it, and putting code there emits a dead
    `core/basic.{kt,rs}` into
    every output [mod-used-only]. Deliberately no `Result` alias — the
    union *is* the result. Every demo that hand-rolled the tags now uses
    std's, which is also what verifies the module end-to-end (the unions,
    qualifiers, and loops demos compile and run under `kotlinc`/`rustc`
    with `ok`/`err` imported from `salvo.core.result`).
    `crates/salvo-syntax/tests/corpus/qualifiers.sv` deliberately keeps
    its own `Ok`/`Err` and `type Result<S, T>` (user decision): it is a
    *parser* corpus — it also names an undeclared `Person` and `Pair` —
    so std's tags would buy it nothing.
- **Arrays get a std function surface (user decision 2026-09-02)** —
  `[type-array]`. Arrays already had literals, indexing, and native
  `for` iteration on both backends; only functions were missing, which
  is why LANGUAGE.md's `CyclicRandom` example (`values.size()` on a
  `T[]`) did not compile. New `core.array` module mirrors `core.list`
  minus construction (literals are the constructor) and mutation
  (fixed size): `size`, `get`, `first`, `iter`, with the same
  `<T canbe linear>` opt-in pattern (measuring/iterating a linear array
  is fine; taking an element out is not). The example now compiles and
  runs verbatim on both backends.

- **Structured diagnostics [diag-structured] + `salvo analyze`
  [cli-analyze]**: errors are `FileDiagnostic` (file index, span,
  severity, message) rendered only at the consuming boundary; `analyze`
  runs the front half of the pipeline with text or JSON output. Analysis
  is *backend-neutral* (`--backend` only opts define files into parsing).
  Parse-broken files participate with recovered ASTs but contribute only
  their parse diagnostics — one broken file never suppresses diagnostics
  elsewhere.
- **`salvo lsp` [cli-lsp]**: LSP over stdio (`lsp-server`/`lsp-types`,
  sync); no incremental state — every document event re-runs
  whole-workspace analysis with open buffers as an overlay. Diagnostics
  (with clearing publishes), markdown hover [doc-markdown] — doc comments
  for fns and structs (with a per-field section) [doc-comment]
  [doc-struct-fields], `[symbol]` references linked to their declarations
  [doc-symbol-ref], fn signatures with effective (inferred) deductions
  [fn-ref-table], and a variable's flow-narrowed type
  [doc-hover-narrowed] — and import-fix code actions
  [diag-import-suggest].
- **VS Code extension + `salvo lang tm-grammar` [cli-lang]**: `vscode/`
  bundles a grammar *generated by the compiler* from the lexer's keyword
  table (tests fail if the checked-in grammar or keyword categories
  drift); `salvo.serverPath` points at a locally built binary.
- **Source discovery [mod-ignore]**: the walk skips hidden directories,
  `CACHEDIR.TAG` directories (Cargo's `target/`), and `.svignore`
  entries; the root itself is exempt.
- **Import suggestions [diag-import-suggest]**: unresolved
  handler/effect/import diagnostics carry `module.Item` suggestions from
  a whole-program declaration index; rendered as `help:` lines, JSON
  `imports`, and LSP quickfixes. std gained `random`
  (`DefaultRandom`), the first non-`core` module — and exposed
  [kt-handler-template-return]: value-returning handler-member templates
  emit `return run { … }`.
- **Numeric literal suffixes [lit-numeric]**: `1` Int, `1L` Long, `1.2`
  Double, `1.2f` Float; decision: `f` requires a decimal point (`1f` is
  a lex error). Kotlin renders native suffixes; Rust renders explicit
  types (`1i64`, `1.2f32`).
- **Missing-return [fn-must-return]**: non-`None` fns must return on
  every path (syntactic; loops never count; yield-fns exempt).
- **Predicate-qualifier constructors [qual-ctor-predicate]**: the
  constructive-only restriction was lifted; a predicate constructor
  asserts its predicate by construction.
- **Deduction entry forms [deduce-syntax]**: bare `[list]` keeps *all*
  declared qualifiers (semantics change from "keep none"),
  `[list: Mut]` keeps exactly the listed, `[list:]` strips all
  (`Deduction.explicit` flag).
- **Use-after-consume [deduce-consume]** — the largest post-M8 feature,
  built up across several user decisions:
  - Consumed values narrow to `Ty::Nothing` (decision: `Nothing` *is*
    the marker — "a value that no longer exists is an impossibility");
    referencing one is an error; assignment revives.
  - `check_program` runs *two rounds* so inferred deductions are
    enforced at call sites exactly like declared ones (round one checks
    + infers; round two re-checks with the inferred facts injected,
    then re-infers). Round one's diagnostics are discarded (checking is
    deterministic). Known non-convergence: round two's narrowing can
    change overload resolution, whose re-inferred deductions are not
    fed back again (no third round); acceptable at current scale.
  - Decision: consumption is *uniform across all types* (a rejected
    alternative exempted backend-copyable scalars; consistency of the
    abstract contract won). Made livable by a **branch-aware** analysis:
    per-branch snapshot/isolate/merge (`snapshot_narrows`/
    `restore_narrows`/`merge_fallthrough` + `block_always_exits`) —
    always-exiting branches contribute nothing, a value consumed on any
    fall-through path stays consumed (maybe-moved, as in Rust),
    disagreeing states keep only common qualifiers.
  - Kept parameters shed their removal set (declared − kept) from the
    argument's narrowed type, so a second `remove_first` after
    `[list: Mut]` stripped `NonEmpty` fails overload resolution.
  - Consumption survives `is`-narrowing restores (`Nothing` is skipped
    on restore), and loop bodies are re-checked once with their exit
    state as entry when the first pass changed anything
    (`check_loop_body`) — back-edge use-after-move surfaces like
    rustc's "moved in previous iteration" (second-pass duplicates
    deduplicated by file/span/message; value results discarded).
- **Union-arm arguments [type-union]**: fixed `unify`'s match-arm order
  so `describe(ok("x"))` resolves against `Ok Str | Err Str` (see
  Gotchas).

### S1 — shared fate, strict checker-only (completed 2026-09-01)

The first stage of the shared-fate roadmap (see the L1 section below for
the decided model). What landed:

- **`intrinsic fn copy<T>(value: T) -> T`** in `std/core/basic.sv` => value
  [intrinsic-fn] [copy-fn]: parser already accepted `intrinsic fn`
  (body-less like `external`); the declaration flows through
  Symbols/resolve/checker unchanged — the `[value]` deduction is the
  whole checker contract. Both emitters intercept
  `backing == Internal` before define-template lookup and lower the
  call from the checker's resolved argument type: Kotlin [kt-copy]
  identity for transitively immutable types, `.toMutableList()` /
  `.copy()` / `.copyOf()` for `Mut List` / `Mut` structs / arrays,
  codegen error for nested mutability and unknown/generic types; Rust
  [rs-copy] `.clone()` on the argument's place.
- **Fate links in the checker** [fate-link]: `LocalVar` gained
  `id`/`links`/`poison`; links are directed, flattened to roots at the
  binding, whole-variable. Creators: `let`/assignment from a bare
  identifier or projection chain, destructuring, `for` bindings,
  `is`/`when` bindings. Snapshot/restore/merge and the loop re-check
  carry the full `VarState`; links union across branch merges.
- **Poison rules** [fate-poison]: a `Mut`-kept call argument, projection
  assignment, or `++` on a root — and moves and whole-variable
  reassignment of it — poison its derived variables (`Nothing` + a
  recorded reason; the use-site error names the root, the event, and
  the `copy` remedy). Derived variables are read-only
  [fate-derived-readonly]: moving (consuming call, `return`, `break
  value`, `yield`) or mutating one errors at the site.
- **deduce.rs**: `let`/assignment of a bare parameter is no longer
  inferred as a move — it links; sound because every escape of the
  derived variable is a checker error until `copy` intervenes.
- **[struct-mut] is now enforced** at field-assignment sites (it was
  spec'd but unchecked, and became load-bearing: Kotlin's identity-copy
  is only correct if non-`Mut` values really are immutable). Arrays
  stay index-assignable without `Mut` (status quo; `copy` does a real
  array copy). Fixed a LANGUAGE.md spec bug the enforcement exposed:
  the `Mut` example mutated `person` instead of `mutable_person`.
- **Rust emission**: fate-linked bindings clone — `let`/assignment
  values and `for` iterables that are bare identifiers of owned
  non-Copy locals emit `.clone()` instead of moving (the checker keeps
  both sides readable). This also closed the old "`let a = b` moves
  local `b`" rustc-rejection leftover. Kotlin emission unchanged
  (aliasing is unobservable because mutation-after-link is rejected).
- Deliberately not tracked yet (later stages): non-identifier call
  arguments in *moved* positions (physically a clone today;
  kept-`Mut` positions *are* tracked since the 2026-09-02 parity fix —
  they mutate their provenance roots), lambda captures, and S2's
  move-mode relaxation. (Literal stores, spread, and `use`
  handler-constructor arguments landed in L2 — see the next section.)

### L2 — remaining consuming sites (completed 2026-09-02)

Every remaining move event now feeds the same consumption lattice as
call-site moves (mirroring the [deduce-infer] move list): storing a bare
identifier in a struct/array/tuple literal, spread `...n` (struct-literal
spreads and any `Expr::Spread`), `return n`, `break n`, `yield n`, and
`use Handler(n)` constructor arguments. One helper (`fate_move`) handles
all of them: a derived variable errors at the site
[fate-derived-readonly], a root is consumed (`Nothing`) and poisons its
derived variables [fate-poison], exactly like a call. What's worth
knowing:

- **Loop exits merge break-path states.** `LoopCtx` captures a
  `NarrowSnapshot` at every `break`; the `while`/`for` checking merges
  them with the fall-through exit state via `merge_fallthrough`. Without
  this, a `break s` inside an `if` was invisible after the loop (the
  always-exiting branch contributes nothing to the merge *inside* the
  body — correct there, but the loop exit is precisely where break-path
  state lands). Merge order puts the current (fall-through) snapshot
  first so the shorter frame stack drives the merge (break snapshots
  carry extra inner frames; for `for` loops the merge runs after the
  binding frame is popped).
- **Diagnostics name the event**: `LocalVar`/`VarState` carry
  `consumed_by: Option<&'static str>` ("a literal store", "a `...`
  spread", "a `break`", "a `yield`", "a `use` handler registration",
  "an earlier call"), threaded through snapshot/restore/merge like
  poison, cleared by reassignment revival.
- **`yield` + back edge works for free**: the existing loop re-check
  reports the second-iteration use at the `yield` itself.
- **L2a decided (user, 2026-09-02)**: string interpolation is a *read*
  — `"${n}"` never consumes. Spec'd under [type-str], cross-referenced
  from [deduce-consume]. Parity-sound because both emitters render the
  interpolated value as an owned copy purely for formatting.
- **Parity audit** (per the backend-parity principle): all new sites
  render through `emit_expr` = owned rendering in the Rust backend, and
  an owned non-Copy local emits as a bare place — a *physical move* —
  so bare-ident consumption is faithful-emission parity and closed real
  rustc-rejection gaps (`let t = (s, 1)` then `read(s)` was
  checker-clean but rustc-rejected before L2). Struct-literal spread was
  a latent *mutable-data* parity hole: Kotlin emits shallow `.copy()`
  (aliases `Mut` fields) while Rust deep-clones (`..base.clone()`) —
  `let p2 = Person {...p}; add(p.tags, 2)` would print different values
  per backend. Closed by restriction: the spread consumes `p`. The one
  finding left open — *projection* values in moved positions (literal
  stores `Box {item: h.tags}`, moved-position call arguments), where
  Rust cloned and Kotlin aliased, observable for mutable data and *not*
  caught by rustc — was accepted as a known live divergence (user
  decision 2026-09-02) and closed by S2 the same day [fate-move-mode].

### S2 — move-mode bindings (completed 2026-09-02)

The relaxation stage of shared fate: bindings have *modes* inferred
from downstream flow. What landed (rule [fate-move-mode]):

- **Mode inference rides the two-round architecture.** Round one is
  strict S1; `error_derived` — the single choke point for every derived
  move/mutation — records the *whole bind chain* as move-mode
  candidates (links now keep their original bind spans when flattened,
  so a variable's links describe the full derivation chain even after
  intermediate variables die) plus parameter *claims*. Round two
  applies the modes at every bind event (`declare_var` and assignment
  re-links): all live ancestors owned → consume them at the binding
  (poison names the binding, `FateEvent::BoundAway`), the binding
  carries no links, and the event is recorded in
  `Checked::binding_modes`.
- **Claims make the flagship work.** A move-mode binding reaching a
  parameter of an *inferable* fn claims it as moved;
  `deduce::infer(program, checked, claims)` seeds claims after every
  body inference (monotone). Written-kept parameters block claims: the
  binding stays borrow-mode and the S1 error stands at the move site.
- **Moved-position projections closed the accepted parity divergence**
  (the S2 obligation): a projection of *transitively mutable* data
  (`ty_transitively_mut`, following struct fields with a visited set)
  in a moved position consumes its owned roots
  (`Checked::moved_projections`; for-binding roots flip the loop to
  by-value) or errors for a written-kept parameter root ("cannot move
  mutable data out of `h`", remedy `copy`). The 2026-09-02 probe
  (`wrap(h.tags)` then `add(h.tags, 9)`) is now *rejected* — verified.
  Immutable projections stay untracked by design (unobservable).
- **Rust emission is faithful**: move-mode bind events emit raw places
  (real moves, partial for projections), move-mode loops iterate by
  value, tracked projections render without clones. The flagship
  pipeline (`longest_name` with inferred deductions) emits **zero
  clones**, rustc-compiles, and prints identically on both backends
  (verified end to end). Kotlin emission unchanged. `is`/`when`
  move-mode bindings still clone on Rust (restriction-valid).
- **Pre-existing false positive fixed**: a `for`-loop binding consumed
  in the body (`for s in xs { consume(s) }`) errored on the back-edge
  re-check — bindings now go through `check_loop_body`'s per-pass
  bindings channel (`pattern_bindings`), so each pass re-declares them
  fresh (an iteration binds a new element).

### L3 + L4 — convergence, same-call ordering, lambda captures (completed 2026-09-02)

- **L3 same-call ordering [deduce-same-call]:** within one call, a later
  argument may not mention a value an earlier argument consumed
  (`f(a, a)`, `f(a, size(a))`) — argument typing precedes contract
  enforcement, so the contract loop now tracks what this call consumed
  (`consumed_here`, fed by bare-ident moves and `projection_move`'s
  returned root names) and mention-checks every argument
  (`expr_mentions`, a full expression/block walk). Nested calls were
  already ordered (consumption applies during argument typing).
- **L3a decided (user, 2026-09-02): iterate to a capped fixpoint
  [deduce-fixpoint]** — option (iii): extra checking rounds run only
  when the driving facts changed (inferred deductions, move-mode
  candidates, claims), capped at four; stable programs stay at two
  rounds and identical cost; a program unstable at the cap gets a
  deterministic error naming the oscillating fns with the
  write-the-list remedy. Candidates/claims grow monotonically, so late
  discoveries converge — a move-mode candidate first seen under
  round-two narrowing is now *applied* in round three (the diagnostic
  moves from the raw derived-move error to the true site). The cap
  error is direct code but untested: constructing a genuine overload
  oscillator is an open exercise.
- **L4a decided (user, 2026-09-02): lambdas are ordinary values under
  shared fate [fate-lambda]** — superseding the recorded
  captures-copy recommendation after the emitter audit (plain borrow
  closures on Rust; Kotlin aliases; mutate-after-capture was a rustc
  E0502, not a silent divergence). Per-capture classification from the
  body, bound at creation: immutable reads free; mutable reads link
  the closure to the variable (root mutation poisons it — the E0502
  class becomes a Salvo diagnostic); mutated captures consumed at
  creation (kept-param → error, inferable param → claim); consuming a
  capture is always an error (multiplicity untracked). **No emitter
  changes**: borrow-captures alias on both backends, so parity is
  direct, and checker-legal programs pass NLL (verified end to end —
  identical stdout). Implementation rides the existing event
  machinery: a `lambda_ctx` boundary stack, capture recording in the
  Ident read arm, mutation marking in `fate_mutation`, consumption
  guards at the four consuming sites, and `finish_lambda_captures`
  applying the contract; lambda values get links via `links_for_value`
  and `Checked::lambda_captures` is exported for tooling/emitters.
  Discovered en route: PROGRESS previously overstated "captures are
  completely untracked" — body *consumption* already applied inline at
  creation; the new guard turns that into the multiplicity error.
- Known loud leftover [fate-lambda]: returning/storing a
  capture-carrying closure is a rustc lifetime error the checker does
  not reject; the recorded refinement is `move`-closure emission with
  hoisted clones, pending a treatment for captured effect-handler
  locals. Fn-type contracts (deductions/effects on `Ty::Fn`, the
  named-fn mode mismatch, closure double-use) remain deferred to the
  L7 parameterized-qualifier work.

### S3 — borrow emission (completed 2026-09-02)

The final shared-fate stage: borrow-mode bindings become real Rust
borrows [rs-borrow-locals]. What landed:

- **`&T` locals via `BindKind::Ref` reuse**: a borrow-mode `let` from a
  *pure place* (bare ident / field / index chain; no coercion,
  narrowing unwrap, or field cast; name never reassigned; bind event
  not move-mode) emits `let mut n = &person.name;` and registers as a
  reference binding — the entire existing kept-parameter rendering
  (owned reads clone, borrow positions pass bare, Copy derefs) then
  applies unchanged. Already-`&` roots pass the reference through;
  `&mut` roots reborrow (`&*x`).
- **By-reference loops**: a borrow-mode `for` over a pure-place
  iterable with a plain ident binding iterates without cloning the
  collection (`for person in persons` where `persons: &Vec<Person>`),
  the loop variable itself a reference binding. Guarded to concrete
  non-union element types — union/optional elements go through
  `matches!`/unwrap lowering that expects owned subjects and keep the
  clone path.
- **Decision S3a resolved as emission, not semantics**: a mixed join
  (linked on one path, independent on another) simply keeps today's
  owned/clone emission — since S1, links union across branches and
  poison covers every observation, so the clone is restriction-sound;
  forbidding would have added errors with no parity need, and `Cow`
  buys nothing. No program's legality changed anywhere in S3.
- **Borrowck alignment**: checker-legal programs pass NLL because
  poison forbids using a derived value after its root is mutated,
  moved, or reassigned — so every borrow's last use precedes the
  conflicting event. Known loud exception (documented, rare shape): a
  single call that passes a borrow-emitted local *and* moves its root
  (rustc E0505; the checker's left-to-right argument model accepts
  it). Loud, never wrong.
- **Exit criterion verified**: the moved-position parity probe is
  still rejected after the emission change, and the read-only pipeline
  demo compiles and prints identically on both backends with *zero*
  clones in the Rust output (`count_long`: borrowed param, borrowed
  loop, borrowed field binding).

### L6 — must-use linearity (completed 2026-09-02)

All five decisions (L6a–e) approved by the user as recommended; rules
[linear-canbe] [linear-obligation] [linear-discard] [linear-composite]
[linear-generics] [linear-lambda] [linear-static]. What landed:

- **`canbe linear`** on type declarations (the auto-qualifier clause
  already parsed arbitrary auto-qualifiers; `has_auto_linear` mirrors
  `has_auto_mut`); `Linear` in a use-site type is an error — linearity
  is declared, not applied. `ty_transitively_linear` /
  `ast_type_linear` mirror the `Mut` transitive analysis (composites
  are contagious).
- **Obligation checks ride the existing flow machinery**:
  `owes_linear` (declared-linear + live + no links + owned-param rule
  via `own_contract`), scanned at frame pops (`check_linear_frame_drop`
  in `check_branch_block`/`check_fn`/`check_lambda`), at
  `return` (all frames) and `break`/`continue` (frames above the
  loop's `entry_depth`, new `LoopCtx` field) via `check_linear_exit`,
  at linear-typed expression statements, and at assignment over a live
  linear value. The all-paths rule lives in `merge_fallthrough` (which
  gained a `span` parameter): consumed on some fall-through paths but
  not all = error — the exact dual of maybe-moved. Reported variables
  are marked consumed (one error per obligation). Gated to round two+
  (`inferred.is_some()`), like the other contract-dependent checks.
- **`intrinsic fn discard<T>(value: T) -> None`** in std; the `[]` => !value
  deduction makes the discharge just another move. Rust lowers to
  `drop(value)`, Kotlin to `(value).let {}` [intrinsic-fn]. Both
  verified end to end with identical stdout on the open/use/close
  resource demo.
- **Generic ban** in `resolve_named_call` on the resolved substitution:
  linear instantiation of an unconstrained `T` errors; `copy` refuses
  with its own message; `discard` (intrinsic, by name) is blessed.
  Known leftover: effect members with their own generics are not
  covered by the ban.
- **Lambda guard**: capture-and-mutate of a linear value errors in
  `finish_lambda_captures` (the closure would swallow the obligation);
  read captures are aliases and fine.
- `LocalVar` gained `decl_span`, so obligation diagnostics point at the
  variable's declaration.
- Practical consequence (documented): a Salvo-bodied consumer
  (`fn close_file(h: FileHandle) -> `) must itself end the chain with => !h
  `discard(h)` — real resource release lives in external fns, which
  have no body to check. `List<FileHandle>` is expressible but not
  constructible until a generic opt-in exists (L7).

### L7a — generic linear opt-in `<T canbe linear>` (completed 2026-09-02)

Syntax decision (user, 2026-09-02, option C of the explored set): the
opt-in reuses the `canbe linear` phrase on *type parameters* — one
qualifier per `canbe`, comma separates parameters; struct-side syntax
deferred. (Both sites were spelled `with` until the 2026-09-03 rename
[canbe-optin].) What landed (rule [linear-generics] rewritten):

- **Parser**: `parse_generics_canbe` parses `<T canbe Q, U>` into
  `FnDecl.generic_canbe: Vec<(Ident, TypeRef)>` (fn declarations and
  define signatures); non-fn declarations report "`canbe` on a type
  parameter is only supported on functions". Uniform snapshot churn
  (new FnDecl field) accepted.
- **Checker**: `own_linear_generics` set per fn; `Ty::Var(name)` joined
  the transitive linearity analysis — so opted bodies are checked with
  `T` linear (a written-moved parameter the body drops is a leak), and
  forwarding an opted `T` to an unopted generic fails the ban
  compositionally. The ban lift replaced the discard-by-name blessing;
  `copy` keeps its dedicated refusal. Only `Linear` is accepted in the
  clause.
- **Variadic guard**: linear values are refused in variadic positions
  (untracked — the value would be physically moved but statically still
  owed); the audit found this while opting in `list`/`mutable_list`,
  whose variadic constructors would otherwise have leaked obligations.
  Empty construction + `add` is the supported pattern.
- **std audit**: `add`, `list`, `mutable_list`, `size` opted in;
  `discard` re-declared as
  `intrinsic fn discard<T canbe linear>(value: T) -> None`; `get` => !value
  deliberately *not* opted (returns an alias of an element — a clone of
  a linear value would duplicate the obligation); `copy` refused.
- Verified end to end: the `List<FileHandle>` workflow (construct
  empty, `add` individually, `size`, `discard`) compiles and prints
  identically on both backends.

### L7b — `once` fn types (completed 2026-09-02)

The call-multiplicity qualifier, landed as a *fn-type qualifier* rather
than the originally-sketched deduction-list surface (user decision
2026-09-02 — calling once is consuming, which contradicts a deduction
entry's kept-ness; the type-qualifier form rides the existing
machinery). Rule [once-fn]:

- **Enforcement is consumption**: calling a `once` value consumes it —
  double calls, loop back-edge calls, and call-after-escape are the
  ordinary consumed-use errors; maybe-calls are conservative; zero
  calls fine. One fix en route: the Ident-callable path in `check_call`
  bypassed the standard consumed-read error (it never `check_expr`s the
  callee ident), so calling an already-consumed callable reported
  nothing — it now routes through the standard error.
- **Inverted subtyping, flagged for future review** (user request):
  plain fn <: `once` fn, and `once` may never be dropped — the
  opposite direction of every other qualifier, special-cased in
  `is_subtype` and `unify` with loud comments.
- **Consuming-capture lambdas legalized**: [fate-lambda]'s always-error
  became "legal but `once`-typed" — the capture is consumed at
  creation, the closure fits only `once` positions (boundary error
  otherwise), and every *enclosing* lambda is marked too (an outer
  closure re-creating an inner consuming one re-consumes per run).
  Linear captures still refuse (exactly-once closures are future
  work). The escape rule consumes a `once` value passed as any
  argument (fn-value ownership is otherwise untracked).
- **Emission nearly free**: Rust emits `impl FnOnce(…)` for `once`
  params (`QualifiedGroup` over `Fn` in `emit_type` + the
  fn-param-Owned mode extended to qualified groups); lambda emission
  unchanged (rustc's capture inference produces `FnOnce` closures
  itself). Kotlin erases `once` entirely. Verified end to end with
  identical stdout.
- No inference in v1 (written `once` only); the parser needed nothing —
  `once () -> None` already parsed as a qualified group over a fn type.

### L7c — derived returns `ReadOnly[from: p]` (completed 2026-09-02)

The relaxation of S1a: zero-copy accessors across fn boundaries. Rule
[readonly-return]; surface decided by the user (square brackets — angle
reads as generics, round collides with qualified groups, square is
where annotations already name parameters). What landed:

- **Parser**: `parse_derived_return` recognizes
  `ReadOnly[from: param]` between the deduction list and the return
  type in both fn parse paths; stored as `FnDecl.derived_return`
  (never enters the type AST — mirroring the checker design, where the
  fact becomes links at the boundary and the result's *type* stays
  plain, so overloading is untouched).
- **Checker**: validation (parameter exists, *kept* — written or
  inferred), return-provenance validation (every returned value's link
  chain terminates at `p`, `None` free, forwarded derived calls
  validate through the same links), and returns in derived fns do not
  consume. Caller side: `Checked::derived_calls` (call span → argument
  index) makes `links_for_value` link results to arguments.
- **Borrowed links close the S2 interaction**: probing found move-mode
  would have "taken ownership" of a *physically borrowed* result
  (`take(h)` after narrowing `first(...)`'s result compiled to moving
  out of a `&`). `FateLink` gained a `borrowed` flag, set through
  derived calls and propagated through derivation chains;
  `apply_binding_mode` refuses move-mode over borrowed links, so the
  S1 error with the `copy` remedy stands.
- **Rust emission**: `&T` / `Option<&T>` returns; lifetime elision for
  a single reference parameter, mechanical `'a` generation onto the
  annotated parameter and return when there are more — the first
  deliberate exception to the no-lifetimes invariant. Return values
  render as borrows (`Some(&place)`, bare for `&` bindings,
  pass-through for forwarded derived calls). Caller-side narrowing
  works without emitter changes (`Option<&T>` is `Copy`; the
  `.clone()` on a `&&T` copies the reference — the
  `suspicious_double_ref_op` lint joined the generated allow list).
  std's `first` dropped its `.cloned()` — clone-free — and gained the
  annotation. Kotlin: zero changes.
- **v1 scope recorded**: plain `T` and `T?` shapes; fn declarations
  only; accumulator bodies (`best = person; …; return best`) rejected
  by the provenance validation — reassignable borrowed locals are the
  recorded refinement.

### L7d — fn-type contracts (completed 2026-09-02)

The user-directed redesign of the original "piece 3": instead of a
fixed all-moved convention, fn types carry *declared* contracts —
default keeps-everything, so nothing broke and the parity hole closed
by faithful emission. Rule [fn-contract]:

- **Surface**: fn-type parameters may be named and a standard deduction
  list may follow the arrow (`(v: List<Int>) -> Int => !…`). `Type::Fn`
  gained `param_names`/`deductions`; `Ty::Fn` gained
  `contract: Option<Vec<FnParamContract>>` (a types.rs struct — kept
  out of `Display` to avoid message churn).
- **Checker**: `apply_fn_value_contract` mirrors the named-call
  contract loop (consumption, kept-`Mut` mutation events, qualifier
  shedding, same-call ordering, Once escape, capture/kept guards) at
  fn-value call sites; lambdas checked against a contract mark kept
  parameters `lambda_kept` (never consumable — guarded at all four
  consuming sites plus binding modes); named fns passed by value build
  their contract from written/inferred deductions; `contract_fits`
  (keeps <: consumes, inverted like [once-fn] and flagged with it)
  joined `is_subtype` and `unify`. One enabling fix: single-candidate
  named calls now type *lambda literal* arguments against the callee's
  parameter types upfront, so expected fn-type contracts actually
  reach `check_lambda` (multi-candidate calls keep the untyped probe).
- **deduce.rs**: calls through fn-typed *parameters* of the walking fn
  apply that parameter's written contract, so consuming contracts
  propagate interprocedurally (`caller_loses` sees its argument die).
- **Rust emission**: fn params render `&mut impl FnMut(…)` (closure
  double-use fixed by faithful emission; `FnMut` accepts
  handler-mutating closures; `once` stays `impl FnOnce`), argument
  types per contract, call-site arguments per
  `Checked::fn_value_calls`, lambda bindings/annotations per
  `Checked::lambda_contracts`, and named fns wrap in mechanical
  adapter closures. **Kotlin**: contracts erase; named fns emit
  `::name` function references (that pass was silently broken on
  Kotlin too — `count` bare emitted a call-less identifier kotlinc
  rejects).
- Deferred, recorded: fn-type *effect* lists (parse, lexical env
  meanwhile), `once` inference, per-parameter written contracts
  beyond kept/moved/quals.

### Current architectural facts worth knowing

- **`ReadOnly` presentation (landed 2026-09-02)**: reads of fate-linked
  variables record their links into `Checked::fate_reads` (root name +
  bind span per link [fate-link]); the LSP hover renders such a
  variable as `ReadOnly T` with the qualifier's parameters (roots,
  binding sites, `copy` remedy) as detail below the type line —
  progressive disclosure per user decision. Presentation-only:
  `ReadOnly` is not in the type system and cannot be written. It is
  phase 1 of the parameterized-compiler-qualifier design recorded
  under L7.
- **Resolution/checking pipeline**: `emit_program` runs
  `Symbols::collect` (flat, still used for define templates and arity
  fallbacks) → `salvo_core::resolve` (per-file scopes) →
  `salvo_core::check_program` (two rounds + deduction inference, see
  [deduce-consume]). Type errors are structured `FileDiagnostic`s
  [diag-structured]; they abort emission and are rendered at the backend
  boundary into `BackendError::Codegen` strings (the CLI `analyze`
  command consumes them structured instead).
- The checker assumes it can **see everything** (2026-09-03): calls,
  fields, subscripts and `for` subjects must be justified by declarations
  ([call-resolve], [field-resolve], [index-resolve], [iter-resolve]).
  What stays lenient is *inference*: a type it could not work out is
  `Ty::Unknown`, compatible with everything, so one mistake yields one
  diagnostic. Coercions/unwraps only fire where the tables say so — the
  emitter's syntactic paths remain the fallback everywhere else, which is
  what keeps a checker regression degraded rather than wrong.
- Wrapper-union arm identity is positional over the **declared** type's
  non-`None` arms; narrowing never re-wraps a variable in place (uses are
  unwrapped/re-wrapped at expression sites instead).
- Modules are emitted to `<module/path>.kt` / `<module/path>.rs`.
  `unions.kt` / `unions.rs` is emitted whenever any wrapper size is used
  by an *emitted* file.
- Only reachable modules that produce code are emitted [mod-used-only]
  (`reach.rs`: name-usage edges over `ModuleScope::name_origins`); each
  module gets its own Kotlin package `salvo.<module.path>` with generated
  imports [kt-package] [kt-imports]; companions copy verbatim
  [backend-companion].
- Deductions (`-> T => list: Mut`) are inferred/validated by the
  `deduce.rs` post-pass and stored in `Checked::deductions`; the Kotlin
  backend ignores them, the Rust backend derives its parameter modes from
  them (kept = borrow, omitted = move [rs-borrows]); the checker enforces
  them flow-sensitively at call sites [deduce-consume].
- **Emitter effect-environment fallback (deliberate, revisit later)**:
  the emitters' effect environments are string-keyed; at each site they
  first consult the checker's `use_effects`/`effect_calls`/`call_effects`
  tables (rendered through `kotlin_ty`, which must agree with `emit_type`
  on the same source type) and fall back to string/base-name matching
  only when the table has no entry or the type contains `Unknown`. The
  fallback keeps the lenient-checker contract: a checker regression
  degrades to string matching rather than wrong code. Cost: double
  bookkeeping. When the emitters key their environments by checker `Ty`
  directly, the string env can be deleted.
- The two emitters deliberately share their architecture (side-table
  access, `emit_expr` = base + coercion, fallback paths, is-binding and
  loop lowering shape). When a lowering rule changes, check both crates —
  and the checker, which must agree with them on the ident-unwrap
  predicates (`maybe_coerce`'s "effective repr").

## Defects found and closed

Each was reproduced before it was fixed, and the repro is kept: it is the
argument for the rule that closed it. Defects still open are in
[ROADMAP.md](ROADMAP.md).

### ~~An effect member named like a std fn breaks std's emission — and `@module` runs the wrong one~~ — found and closed 2026-09-15

**Was reproduced** by an ordinary effect, no concurrency involved — found while
writing the dependent-spawn test, whose child wanted a member called `add`:

```
effect Tally {
    fn add(n: Int) -> None => !n
}
handler Summing() of Tally {
    sum: Int = 0
    fn add(n: Int) -> None { sum = sum + n }
}
fn double(x: Int) [] -> Int { return x * 2 }
fn main() [use] {
    use StdOutConsole()
    use Summing()
    add(4)
    let ys = map(iter([1, 2, 3]), double)
    println("size ${size(ys)}")
}
```

Both backends, twice: `std/core/seq.sv: no handler for effect 'Tally' in scope`.
`salvo analyze` was **clean** — the checker was right and silent.

**The second half was silently wrong output**, which the first half's noise hid:

```
effect Shout { fn to_upper(s: Str) -> Str => !s }
handler Excited() of Shout { fn to_upper(s: Str) -> Str { return "${s}!" } }
fn main() [use] {
    use StdOutConsole()
    use Excited()
    let mine = to_upper@Shout("hi")
    let theirs = to_upper@core.string("hi")     // ran the *member*
    println("member ${mine} std ${theirs}")     // "member hi! std hi!"
}
```

`std HI` was expected; both backends printed `std hi!` — a
[backend-never-wrong] violation, and the reason the fix is verified by a
compile-and-run case rather than a text assertion.

**Root cause: the checker and the emitters asked different questions.** The
checker's availability rule [effect-available] consults `scope.effect_members`,
which is **import-scoped** — inside `std/core/seq.sv` a program's `Tally` is not
there at all, so `add(out, x)` resolved to std's own `add` and the fall-through
that records `fn_over_member_calls` was never reached. The emitters consult
`Symbols::effect_of_fn`, which is **program-wide**, and take the member path
unless that set says otherwise. `@module` was the same hole from the other side:
it skips the member block *deliberately*, so nothing was recorded there either.

**The fix is one condition, in the place the fn path commits**: record
`fn_over_member_calls` whenever a call resolves to a fn declaration and the name
is a member *anywhere in the program*. That covers all three routes — member
not in scope, `@module` written, and the two existing contests — with one rule
instead of three, and it makes the emitters' scope-blind map harmless by
construction. The alternative (giving the emitters the checker's scope) was not
tried: the record already exists for exactly this hazard, it was just
underfilled.

**The lesson**, which is the one `local_calls` taught and this repeated: when
two passes answer one question from *different tables*, the narrower table must
publish its answer for **every** case, not only the interesting one. A
"resolved to a fn" record that fires only on collisions-in-scope is a record
with a hole exactly where the tables disagree most.

### ~~Consuming a handler's stored values is a silent clone on Rust and a share on Kotlin~~ — found and closed 2026-09-15

**Was reproduced** three ways, each worse than it looks. The loud one, which is
how it was found (writing the dependent-spawn test):

```
handler Holding() of Sink {
    last: Str = "abc"
    send fn peek(out: Reply<Str>) { out.send(last) }
}
```

Checker-clean; rustc reports `E0507: cannot move out of 'self.last' which is
behind a mutable reference`, naming `.clone()`. Kotlin runs it. The same shape
through `add(out, last)`. A `Copy` state field hides it entirely, which is why
the first process test (`sum: Int`) never met it.

**The silent ones were the real defect.** An ordinary consuming call and a
`return` both *did* have Rust's clone inserted, and it diverges:

```
fn eat(xs: Mut List<Int>) [] -> Int => !xs { add(xs, 99) return size(xs) }
handler Holding() of Bag {
    items: Mut List<Int> = mut_list_of(1, 2)
    fn go() -> Int { return eat(items) }
    fn count() -> Int { return size(items) }
}
// println("eaten ${go()} kept ${count()}")
//   Rust:   eaten 3 kept 2      (the handler kept a *copy*)
//   Kotlin: eaten 3 kept 3      (the handler kept the same list)
```

The same with a getter (`fn label() -> Mut List<Int> { return items }`), and
worst of all with a **linear** value: `close(t)` on a `linear struct` handler
constructor parameter emitted `close__3(self.t.clone())` — an obligation
*duplicated*, silently, by the backend whose whole job is to refuse that.

**Root cause: handler storage was invisible to the flow analysis as a
lifetime.** A state field and a handler constructor parameter are registered as
ordinary locals, so a move out of one looked like a move out of any local —
legal, since nothing read it afterwards *in that member*. But the handler still
owns the value when the member returns, so there is nothing to move: Rust
cloned to make the borrow check pass, Kotlin shared, and neither is what the
program said.

**The fix is the mirror of a rule already decided.** [effect-state-store]
already said that assigning *into* a state field is a **store** rather than a
link, for this exact reason and on this exact evidence (a 2-versus-1 divergence
found 2026-09-14). The read direction is now the same rule: a member may not
move a value out of the handler's storage, and the diagnostic names
`copy(held)`. A **linear** stored value is refused outright (there is no `copy`,
and taking one out of a composite is [linear-composite]'s interim refusal); a
**Copy scalar** is exempt ([copy-scalar-free]), which is what keeps `out.send(sum)`
on an `Int` field working and what hid the whole thing.

Beside it, the Rust backend's `intrinsic_arg_code` now renders a **consumed**
intrinsic argument owned rather than as a bare place, so the intrinsic path
matches the ordinary-call path instead of being the one consuming position that
emitted a place into a moving one.

**This tightened the language**: `return held` and `eat(held)` used to compile.
Under phase 2b's "copies only by opt-in" they were a copy nobody wrote, which is
what makes the tightening a fix rather than a new rule — but it is a visible
change, and two checker tests had to write `copy` (or stop returning storage) to
keep passing.

**The lesson**: a rule about one direction of a lifetime asymmetry is owed in
the other. "Assigning in is a store" and "moving out is impossible" are one
fact about handler storage, and shipping half of it left the half that produces
wrong output.

### ~~Mutation through a narrowed place borrows a clone on Rust~~ — found and closed 2026-09-10

**Was reproduced** by *running* both backends — `analyze` is silent, and the
Rust output is wrong rather than absent, which is what makes this the one
[backend-never-wrong] violation the compiler has shipped:

```
let p: Mut ListYield<Int>? = iter(list(1, 2))
if p is Mut ListYield<Int> {
    show(next(p))      // Rust: got 1     Kotlin: got 1
    show(next(p))      // Rust: got 1     Kotlin: got 2
    show(next(p))      // Rust: got 1     Kotlin: end
}
```

The emitted call was `next__2(&mut (p.as_ref().unwrap().clone()))`: a mutable
borrow of a fresh clone, so every call advanced a different copy. Three more
shapes had the same fault — a narrowed *union arm* (silently wrong the same
way), an assignment whose **base** is narrowed (`r.at = 2` reached for a field
of the `Option`: E0609), and a narrowed `state` slot of an `iter fn`, which is
where it was found: a lazy flatten holding its inner pass.

**Fixed as [rs-narrow-mut]** — the decision-log entry at the top of this
document has the reasoning, the two false starts, and the second defect it
uncovered (the desugaring gave the synthesized `__p` base the read's span, so
the base answered to the field's narrowing).

### ~~A qualifier applied to an already-qualified value flattens~~ — found 2026-09-07, closed 2026-09-10

**Was reproduced** (`analyze`), found while answering whether a fallible
producer needs `Throw` support [iter-protocol]:

```
fn b(flag: Bool) -> Emitted (Ok Str | Err Str) | Finished {
    if flag {
        return emitted(ok("x"))    // ERROR
        // no arm of `Emitted (Ok Str | Err Str) | Finished` accepts a value
        // of type `Emitted Ok Str`
    }
    return finished()
}
```

Two probes localized it, and *both passed*, so the fault was in the combination:
`fn d(flag: Bool) -> Emitted (Str | Int) | Finished { … return emitted("x") }`
and `fn c() -> Ok Str | Err Str { return ok("x") }`. The rendering
("Emitted Ok Str", no parentheses) was the ambiguity showing through: a flat
qualifier list cannot say whether `Emitted` is applied to `Ok Str` or sits
beside it.

**Fixed under [qual-group]** — the decision-log entry at the top of this
document has the reasoning, the rejected representation change, and the
second bug it uncovered (probe `d` emitted one union wrap where two were
needed, which rustc rejected). The workaround it replaced, kept because it is
still the answer for a *repeated* qualifier:

```
let good: Ok Str | Err Str = ok("x")
return emitted(good)
```

### ~~Narrowing does not survive an early-returning guard~~ — found and closed 2026-09-09

**Was reproduced** (`analyze`, both a concrete and a generic element type):

```
fn head(xs: List<Int>) -> Int => xs {
    let e = get(xs, 0)
    if e is None {
        return 0
    }
    let n: Int = e      // ERROR: expected `Int`, found `Int?`
    return n
}
```

The then-branch cannot fall through, so the statements after the `if` are on the
else-path, where [is-narrowing] already says the remaining arms hold. The
equivalent `when e { is None { return 0 } is Int { return e } }` was accepted, so
the fact existed and was computed — what was missing was *carrying* a branch's
negative fact past an `if` whose branches diverge.

**Fixed as [is-narrow-guard]** (user decision 2026-09-09: "early-returning
guards should produce narrowing properly"). `check_if` already computed
`acc_else` — the facts of every condition being false — and already knew which
branches exit (`block_exits`, the same predicate the consumption merge uses). It
threw the facts away at the join. Now: if every branch exits, `acc_else` is
*installed* (a new `install_narrows`, the application half of `with_narrows`
without the restore) after the fall-through merge and before the assignment
resets.

- **Two lines of it are the whole feature**; the interesting part was that all
  the machinery was already there, keyed on the same predicate.
- **A second imprecision fell out with it**: `reset_assigned` ran for *every*
  branch, so an assignment inside a branch that exits reset the narrowing for
  code that branch can never reach. It is now skipped for exiting branches — the
  same reason `merge_fallthrough` ignores them.
- **Consumed stays consumed**: `install_narrows` skips a variable narrowed to
  `Nothing`, exactly as the restore half does — a flow fact outranks a
  narrowing [deduce-consume].
- **One existing test asserted the old behavior** and became the new rule's
  test: `place_tests`' "outside the branch the fact does not hold" is now
  `a_guard_narrows_the_fall_through_path`, and the interesting part is *which*
  error it gets — the read after the guard is no longer a maybe-`None`
  interpolation but a known-`None` one, so the diagnostic changes from "may be
  `None`" to "cannot interpolate `None`". Both are refusals; the fact is what
  moved.
- **Tests**: `guard_tests.rs` (10) — `return`/`break`/`continue`/diverging-call
  guards, an `elif` chain leaving the third arm, an explicit non-exiting `else`,
  and the negatives (a branch that falls through, a mixed `if`, an assignment on
  the surviving path resetting vs one in the exiting branch not resetting).

### ~~A `yield fn`'s origin may be transitively mutable: checker-clean, and the two backends disagree~~ — found and closed 2026-09-08

**Reproduced.** A `yield fn`'s origin parameter is refused when it is written
`Mut` [yield-fn-origin], but **not** when it is transitively mutable, and the
`Iter<T>` form's [iter-mut-param] refusal does not reach it (it sits in the
`!f.is_yield` branch). So this checks clean and prints different things:

```
struct B : Yield<self, Int> {
    rows: Mut List<Int>
}

yield fn next(b: B) -> Int {
    let before = size(b.rows)
    yield copy(before)
    let after = size(b.rows)     // read across a suspension
    yield copy(after)
}

fn main() [use] {
    use StdOutConsole()
    let b = make_b()             // rows = [1, 2]
    for n in b {
        println("saw ${n}")
        add(b.rows, 9)           // the consumer mutates the origin
    }
}
```

`saw 2 / saw 2` under rustc, `saw 2 / saw 3` under kotlinc — exactly the
clone-vs-alias divergence [iter-mut-param] exists to prevent: Rust clones the
origin into the machine, Kotlin shares the reference. A silently
target-dependent program, so [backend-parity] and the spirit of
[backend-never-wrong].

**Root cause**: R3 added a *shallow* check (top-level `Mut` on the parameter)
and the backend specs then justified the differing capture conventions by
claiming the transitive rule covered the rest. It does not. The claim has been
corrected; the hole is open.

**Not fixed by "extend the transitive refusal"** — that would refuse
`struct B { rows: Mut List<Int> }` as an origin at all, and iterating a
mutable buffer you hold is a reasonable thing to want (more so than under the
`Iter<T>` form, since an origin is ordinary data the caller keeps).

**Closed by option (d)**, the same day: the origin of an **open `for`** may not
be mutated [yield-fn-origin]. The insight the user supplied is that the problem
was never mutability but *mutation during a drive* — a mutable-origin **type**
is fine, and what has no target-independent answer is the overlap of a live
pass with a write. Hooked into `fate_mutation`, the single choke point every
whole-variable mutation already goes through, and keyed on the subject's
**root**, so a write through a projection (`add(b.rows, 9)`) reports too, as
does passing the origin to a `Mut` parameter. Scoped to the loop, so a
*different* loop's body may mutate it. The repro above is now an error naming
both remedies (move the write out, or iterate `copy(b)`); four tests in
`yield_origin_tests.rs`, including that a transitively mutable origin mutated
*before and after* its drives stays clean — the case that had to keep working.

**Closed 2026-09-10, as built**: option (e), under "Mutable origins" below.

### ~~A producer's callback may capture mutable state: checker-clean, Kotlin runs it, rustc rejects it~~ — closed 2026-09-08

Found 2026-09-08 while sequencing I5, and it is the **sibling** of
[iter-mut-param]: that rule closed mutable state reaching a pass through a
*parameter*, and this is the same state reaching it through a *callback* the
producer holds. LANGUAGE_SPEC had it as a known gap ("the checker does not
reject it up front"); this is the reproduction that showed it was reachable,
kept because it is the argument for the rule.

```
fn tagged(xs: Iter<Int>, f: (Int) -> Int) -> Iter<Int> {
    for x in xs { yield f(x) }
}

fn bump(seen: Mut List<Int>, v: Int) -> Int => seen: Mut {
    add(seen, copy(v))
    return v
}

fn main() [use] -> None {
    use StdOutConsole()
    let seen = mutable_list<Int>()
    let source = tagged(upto(2), (v) -> bump(seen, v))
    for s in source { println("${s}") }
    for s in source { println("${s}") }
}
```

`salvo analyze`: **no errors**. Kotlin: compiled, ran, printed `0 1 0 1` — and
`seen` had accumulated **four** entries, because the callback runs once per
element in *every* pass. Rust: `rustc` rejected the emitted code (E0373, E0596),
because a producer's fn-typed parameter arrives as `impl Fn + 'static`
[rs-iter-lazy] — `Fn` and not `FnMut` precisely so a callback cannot carry state
across passes.

**Closed by extending [iter-mut-param] to a producer's callbacks** (2026-09-08),
covering both a capture the closure *writes* through and one it merely *reads*
mutable data through — the second for the rule's own reason rather than
`FnMut`'s: snapshot-vs-alias is observable, so the backends would disagree about
what a later pass sees. `Checked::lambda_captures` grew the two flags the rule
reads (`mutable`, `mutated`), which are worth having exported anyway.

**A second defect fell out of writing the remedy**, and it is the more
embarrassing one: the diagnostic's advice ("capture an immutable snapshot") *did
not compile on Rust either*. A producer's callback is declared
`impl Fn(…) + 'static` and was emitted as a **borrowing** closure, so E0373 hit
every capturing lambda handed to a producer — including the immutable captures
the parity rules call free. One word (`move`) in
`emit_args_for_params`/`emit_lambda`, gated on the callee being an iterator fn.
Nothing in the suite passed a capturing lambda to a producer, which is why a
plainly broken path stayed green; `{rustc,kotlinc}_compiles_and_runs_a_producer
_with_a_capturing_callback` covers it now, one source and one stdout.

### ~~A producer's `Mut` parameter is copied on Rust and shared on Kotlin~~ — closed 2026-09-08 by refusing the shape


Kept because the reproduction is the argument for the rule, and because option
C revives the question under `Cell`.

Found 2026-09-08, immediately after I4 landed, while asking whether the
*pure* producer's injected `close` (the I2c leftover) is observable. It is —
and finding out turned up something worse and unrelated to `close`: the
**capture convention for a producer's parameters diverges between the
backends** [backend-parity]. Minimal repro, which compiles clean today:

```
fn tally(limit: Int, sink: Mut List<Int>) -> Iter<Int> {
    let i = 0
    while i < limit {
        add(sink, copy(i))
        yield copy(i)
        i = i + 1
    }
}

fn main() [use] -> None {
    use StdOutConsole()
    let seen = mutable_list<Int>()
    let source = tally(2, seen)
    for v in source { println("a ${v}") }
    println("after first: ${seen.size()}")
    for v in source { println("b ${v}") }
    println("after second: ${seen.size()}")
}
```

| | after first | after second |
|---|---|---|
| Rust | `0` | `0` |
| Kotlin | `2` | `4` |

**Root cause**, one line of emitter each and both defensible on their own:

- Rust captures every parameter into the factory **by clone** and clones again
  per pass (`let __c_sink = sink.clone();` … `__Pass_tally::new(…,
  __c_sink.clone())`), which is what makes the captured state `'static` and
  each pass start from the beginning. The pass therefore mutates a *private
  copy*: the caller sees nothing, and two passes cannot interfere.
- Kotlin hands the parameter to the pass **as it is** (`Iterable<Int> {
  __Pass_tally(limit, sink) }`), and a `MutableList` is a reference — so the
  caller's list receives the writes and successive passes *accumulate* into
  it.

`defer` is not involved (this repro has none), and neither is I4: an effect
claim changes nothing here. The same shape with a `defer` shows it too, which
is how it was found — and there the abandoned path hides it on both backends,
because nothing calls the pure `close` yet.

**This was a language question before it was a fix**, so it was not decided
here: what a `Mut` parameter of a producer means — a private copy, the caller's
own collection, or nothing at all because it is refused — decides which backend
is wrong. **Closed by refusing the shape** [iter-mut-param] (user decision
2026-09-08, option A); see "Producer parameters: `Mut` refused" for the two
options it did not take, and the `Cell` roadmap for where option C is queued.

**Still open behind it**: the pure producer's `close` (I2c's leftover). Giving
it a caller makes an abandoned producer's deferred blocks run, and those are
observable only through state the pass shares with someone else — which, after
this rule, a *pure* producer has none of. The hole is real but no longer
reachable by the route that found it.

## Roadmap: toward full linear types


Where we are: an *affine* analysis ("use at most once") with solid
underpinnings: interprocedural contracts (inferred + validated
deductions), `Nothing`-narrowing with revival, and branch-/loop-aware
state merging. Since L2, the local flow analysis watches *every* move
event the deduction inference knows — call-site moves, literal stores,
spread, `return`/`break`/`yield`, `use` constructor arguments — for bare
identifiers; the remaining gaps are projection values in moved positions
(a parity question, see the L2 audit), lambda captures (L4), and the two
genuinely new mechanisms (places, must-use). The safety net holds:
holes surface as rustc errors on the generated code (loud, never
silently wrong); the one known exception — moved-position projections
of mutable data — was closed by S2 (see the backend-parity principle
note).

Each phase below is independently shippable, in rough dependency order.
Items marked **DECISION** need a language-design call before or during
implementation — everything else is analysis engineering under decisions
already made (uniform-across-types consumption, `Nothing` as the marker,
maybe-moved-is-unusable).

### Backend-parity principle (user decision 2026-09-01)

The operational semantics must be the *same* on both backends; divergent
representations are acceptable only where the difference is
unobservable. This principle governs every move/copy/borrow decision in
the stages below:

- **Immutable data**: clone vs shared reference is purely a performance
  question — free to address later, in any direction, at any time.
- **Mutable data**: a clone where Kotlin shares a reference is a
  *semantics* change, never just a cost. Parity must come from one of
  exactly two strategies:
  1. **Restriction** — the checker rejects every program that could
     observe the difference. This is what deductions and shared fate do
     today: S1's clone-emission is sound *because*
     mutate-after-link and mutate-through-derived are compile errors.
  2. **Faithful emission** — the Rust backend works harder, including
     emitting code shaped *differently from the source* when a
     mechanical equivalence justifies it. Recorded technique, **read
     redirection**: after `let name = person.name`, a later read of
     `person.name` with no intervening mutation is guaranteed equal to
     `name`, so Rust may emit the binding as a real (partial) move and
     redirect subsequent reads of the moved path to the surviving
     variable — no clone, no borrow, no lifetime; the fate-link table
     already knows the equivalence. The same idea generalizes to any
     place the checker can prove holds the same data as a live local.
  Silent clones of mutable data are *not* a valid parity strategy.
- **Parity bug closed (verified 2026-09-01, fixed 2026-09-02):** a
  *projection* passed directly to a kept `Mut` parameter is a mutation
  of its provenance roots — before the fix, `let t = h.tags;
  add(h.tags, 2); size(t)` compiled clean and printed 2 on Kotlin
  (alias) but 1 on Rust (clone). The call-site contract loop now runs
  `fate_mutation` on the provenance roots of non-identifier arguments
  in kept-`Mut` positions (mirroring projection assignment), so the
  derived variable is poisoned and the program is rejected; `copy` at
  the binding is the remedy. Audit the remaining untracked events
  (literal stores, `use` ctor args, interpolation, *moved*-position
  projection args) against this principle when L2 lands them.
- **Moved-position projection divergence — CLOSED by S2 (2026-09-02):**
  a *projection* of transitively-`Mut` data in a *moved* position (a
  consuming call argument, a literal store, a `use` ctor argument)
  cloned in Rust but aliased in Kotlin, and rustc did not catch it (the
  clone is valid Rust) — verified live with `wrap(h.tags)` then
  `add(h.tags, 9)`: Kotlin printed 2, Rust printed 1. S2 closed it
  [fate-move-mode]: such projections now consume their owned roots (the
  probe program is *rejected* — re-verified) or error for written-kept
  parameter roots with the `copy` remedy; tracked projections also emit
  as real partial moves in Rust. Projections of immutable data remain
  deliberately untracked (clone-vs-alias unobservable).

### L1 — Shared fate: links, poison, and `copy` (user decisions 2026-09-01)

Redesigned: replaces the original "aliasing bindings are moves" +
"reads are copies" plan. Shared fate is a lifetime-free borrow
discipline: a variable bound to the value or projection of another
*links* to it, reads flow freely through links, and ownership-requiring
operations (moves, `Mut` ops) consume the rest of the link group. The
motivating example: `longest = person.name` inside a loop over a *kept*
parameter `persons`, then `return longest` — invalid (moving a value
derived from a borrow); remedy `return copy(longest)` — one copy at the
escape instead of a clone per iteration.

The decided model:

- **L1a — `intrinsic fn copy` (decided).** New `intrinsic` item keyword
  for compiler-intrinsic fns:
  `intrinsic fn copy<T>(value: T) -> T` is declared in std (the => value
  signature + `[value]` deduction are all the checker needs:
  non-consuming, result independent), has *no define files*, and each
  emitter lowers calls to it type-directedly — Kotlin: identity for
  immutable data, real copies for `Mut`-capable types; Rust:
  `.clone()` (scalar identity as a later refinement). Unsupported
  types are codegen errors [backend-never-wrong]. Rationale: a define
  template is type-blind text per signature and cannot dispatch on the
  instantiated `T`; an intrinsic sees the checker's resolved argument
  type at each call site.
- **Shared fate (decided; supersedes L1b).** Links are *directed*
  (derived → root), *transitive* (`longest` → `person` → `persons`),
  and — as decided here — at *whole-variable granularity* (no place
  lattice). Link
  creators: `let`/assignment from a bare identifier or a projection,
  loop bindings, destructuring. (Superseded by L5, 2026-09-10: a link
  carries its projection path, poison needs an overlap
  [fate-field-disjoint], and a projection move records a moved place
  rather than consuming the root [fate-partial-move].)
  - Reads never consume and never poison, on any member, any time.
  - Mutation events are already defined by the deduction system:
    Mut-kept call args and projection assignments (effect-handler
    capture deferred to L4).
  - A `Mut` op or move on the *root* poisons every derived member
    (narrows to `Nothing`, error at later *use*, revival by
    reassignment — the existing possibly-consumed machinery). The
    root stays usable after mutation. Poison is not retroactive:
    derivatives created after a mutation are fine. This is NLL-like
    precision without a liveness analysis (no later use → nothing
    fires).
  - **Bindings have modes**, decided by downstream flow:
    *borrow-mode* (derived value only ever read; ancestors stay
    usable) vs *move-mode* (derived value eventually moved/mutated;
    the *binding itself* consumes the ancestors — Rust partial-move
    semantics, emission-faithful, no hidden clones). Poison-at-move
    was rejected: it cannot be emitted faithfully without a hidden
    clone.
  - Moving or mutating a value derived from a *kept* parameter is a
    deduction-contract violation regardless of mode (you cannot move
    out of a borrow); remedy `copy`.
  - Uniform across all types (standing decision). On Kotlin the whole
    discipline is a purely static protocol (no runtime component); it
    rejects some JVM-fine programs by design, remedy `copy`.

Stages (each independently shippable):

- **S1 — strict, checker-only. ✅ Done 2026-09-01** (see the S1 section
  in the decision log above). Links + poison rules with derived
  members *read-only* (any move or `Mut` op on a derived member is an
  error at that site; remedy `copy`). Emission unchanged
  (clone-by-default), so nothing can physically break — the protocol
  lands before the performance. Includes `intrinsic fn copy`
  end-to-end and diagnostics naming the link and the event.
  - **S1a — the function boundary (decided 2026-09-01):** call
    results never link to arguments — returned values are always
    independent; a fn returning a projection of a kept param must
    `copy` internally; links stay intraprocedural. Derived-return
    annotations are reconsidered in a dedicated late milestone (L7).
- **S2 — move-mode bindings. ✅ Done 2026-09-02** (see the S2 section
  in the decision log above; rule [fate-move-mode]). Binding modes
  inferred from downstream flow; move-mode bindings consume their
  owned ancestors at the binding and emit as real moves (zero-clone
  pipelines verified end to end); parameters are claimed into the
  deduction fixpoint; written-kept parameters keep the S1 errors.
  - **Obligation discharged**: the moved-position projection
    divergence is closed — projections of transitively-`Mut` data in
    moved positions consume owned roots / error for written-kept
    parameter roots (`copy` remedy); the parity probe is rejected.
  - Refinement (recorded, optional, still open): *read redirection*
    (see the backend-parity principle) can soften poison-at-binding —
    a read of exactly the moved path (`person.name` after a move-mode
    `let name = person.name`, before any mutation) is provably equal
    to the surviving variable, so the checker may allow it and the
    Rust emitter substitutes `name`. Keeps more source shapes legal
    without clones.
- **S3 — borrow emission (Rust). ✅ Done 2026-09-02** (see the S3
  section in the decision log above; rule [rs-borrow-locals]).
  Borrow-mode bindings from pure places emit `&T` locals; borrow-mode
  loops iterate by reference; Kotlin unchanged; clone fallback
  everywhere else (restriction-sound).
  - **DECISION S3a — resolved 2026-09-02 as emission, not semantics:**
    mixed joins keep the owned/clone emission — link-union + poison
    already reject every observation, so no forbid and no `Cow`; no
    program's legality changed. Read redirection remains a recorded
    future refinement.
  - Exit criterion verified: the parity probe is still rejected after
    the emission change, and the borrow demo prints identically on
    both backends with zero clones in the Rust pipeline.

### L2 — Remaining consuming sites. ✅ Done 2026-09-02

(See the L2 section in the decision log above.) All root-consuming move
events are tracked: literal stores, spread, `return`/`break`/`yield`
(with break-path states merged into loop exits, and the yield/back-edge
interaction handled by the existing two-pass loop analysis), and `use`
constructor arguments. The parity audit found bare-ident consumption is
faithful-emission parity (Rust already moved at these sites) and closed
the struct-spread shallow-copy-vs-deep-clone hole by restriction.
Remaining audit finding: projection values in moved positions cloned
in Rust and aliased in Kotlin — observable for mutable data only, and
*not* caught by rustc (the clone is valid Rust). Accepted as a known
live divergence (user decision 2026-09-02) and closed by S2 the same
day [fate-move-mode].

- **Priority item — done 2026-09-02**: the verified parity bug (see the
  backend-parity principle above) is fixed — non-identifier arguments
  in kept-`Mut` positions run `fate_mutation` on their provenance
  roots.
- **DECISION L2a — interpolation (decided 2026-09-02).** `"${n}"` is a
  *read*: reads never consume. Both emitters render interpolated values
  owned-by-clone purely for formatting (no reference retained), so the
  decision is parity-sound. Spec'd under [type-str] and cross-referenced
  from [deduce-consume].

### L3 — Same-call and convergence tightening. ✅ Done 2026-09-02

(See the L3+L4 section in the decision log above.) Same-call argument
ordering enforced [deduce-same-call]; checking iterates to a capped
fixpoint [deduce-fixpoint].

- **DECISION L3a — decided (user, 2026-09-02):** option (iii) — iterate
  only while facts changed, cap four rounds, deterministic instability
  error with the write-the-list remedy.

### L4 — Lambda captures. ✅ Done 2026-09-02

(See the L3+L4 section in the decision log above; rule [fate-lambda].)

- **DECISION L4a — decided (user, 2026-09-02):** lambdas are ordinary
  values under shared fate — per-capture classification from the body
  (read/mutate/move), contract bound at creation; immutable reads
  free, mutable reads fate-link the closure, mutated captures consumed
  at creation, consuming a capture always an error. The emitter audit
  superseded the recorded (b) recommendation: plain borrow-closures
  already alias on both backends, so no emitter change was needed.
  Option (c) — full capture/deduction contracts on fn types — remains
  the long-term design, folded into the L7 parameterized-qualifier
  work.

### L5 — Places and partial moves

**Open** — see ROADMAP.md. Largely subsumed by shared fate; what is left is
field-disjoint precision.

### L6 — Must-use: true linearity. ✅ Done 2026-09-02

(See the L6 section in the decision log above; rules [linear-*].) All
five decisions approved by the user as recommended on 2026-09-02:

- **L6a**: `canbe linear` on the type declaration; `Linear` is not
  writable at use sites (every value of the type is linear, always).
- **L6b**: consumption = any move, as the deduction system defines it;
  moves transfer the obligation (compositional across calls, returns,
  stores, and move-mode bindings).
- **L6c**: `intrinsic fn discard<T>(value: T) -> None` is the => !value
  explicit escape hatch; early-exit paths are checked by the existing
  path machinery; panic/abort semantics out of scope until they exist.
- **L6d**: composites containing linear components are linear
  (transitive); unconstrained generics refuse linear instantiation
  (`copy` refused with a dedicated message, `discard` blessed);
  `where T: Linear`-style opt-in deferred to the L7 qualifier work.
- **L6e**: purely static protocol, identical on both backends, no
  runtime component.

### L7 — Reconsider derived-return annotations

Revisit S1a's "returned values are always independent" rule once
shared fate (S1–S3) and linearity (L6) have real usage. The question:
should deductions grow a derived-return dimension ("the result is
derived from parameter X" — lifetime elision by another name), so
zero-copy accessors (`first(persons)` returning a linked element)
work across call boundaries? Adding it later is purely a relaxation
(more programs expressible, nothing breaks). Evaluate against real
std/user code: if the intra-function `copy` costs never show up in
practice, independent returns may be the permanently right answer.

- **DECISION L7a** — whether to add it at all, and the annotation
  surface if so (e.g. `-> persons.T`-style (a projected-element type) vs a marker on
  the return type); every std external returning a projection would
  need auditing.
- **Leading design for L7a (user decision 2026-09-02): parameterized
  compiler qualifiers.** Supersedes the `-> persons.T`
  strawman. Compiler-inserted qualifiers form a distinct class — never
  affecting overload resolution/`unify`/mangling/erasure, not testable
  with `is`, not constructible, strippable only by blessed fns
  (`copy`, later `discard`) — and may carry *parameters* the checker
  uses for checking and diagnostics. `ReadOnly(from: root)` is the
  fate link as a type; `-> ReadOnly(from: param) T` in a signature is
  the derived-return annotation, giving exact root-naming (the caller
  links the result to that argument; Rust lifetime annotations are
  generated mechanically from the parameter — elision already covers
  the single-kept-param case). The same vehicle can later carry
  deduction contracts on `Ty::Fn` values (closing the
  named-fn-as-lambda mode-mismatch leftover) and L4 capture contracts.
  Representational discipline: parameters are var identities in flow
  state and param *names* in signatures, substituted at call
  boundaries (same shape as generic substitution). Adoption is phased
  so each step pays for itself:
  1. **Presentation (✅ done 2026-09-02)**: derived variables hover as
     bare `ReadOnly T`, with the parameters (roots, binding sites,
     `copy` remedy) as on-request detail — progressive disclosure per
     user decision; `Checked::fate_reads` carries the data. No
     type-system change.
  2. **Internal unification (when L7 starts)**: fold
     links/poison/consumed_by into parameterized qualifiers on the
     narrowed type with per-qualifier join direction (`ReadOnly`
     params union across branches; user qualifiers intersect);
     diagnostics gain LSP related-information spans from the
     parameters. Same programs accepted/rejected — done at L7 to avoid
     refactoring twice.
  3. **Signature transport (L7 proper)**: `ReadOnly(from: param)` in
     return position, call-site substitution, mechanical Rust lifetime
     generation; std externals returning projections audited then.
  Open decision points for when L7 starts: the writability boundary
  (readable everywhere; writable only in return position first?),
  per-qualifier join declarations, and whether `Nothing` gains
  parameters (recommended: yes — strictly more informative, revival
  unchanged).
- **L7a delivered 2026-09-02**: the generic linear opt-in
  `<T canbe linear>` (see the decision-log section; syntax option C —
  `where` clauses and qualifier-prefix forms were explored and
  declined; body-inference recorded as a possible later complement,
  mirroring the deduction precedent: written validates, unwritten
  infers).
- **L7c delivered 2026-09-02**: derived returns `ReadOnly[from: p]`
  (see the decision-log section) — the L7a-recorded bare-`ReadOnly`
  idea matured into the parameterized square-bracket surface; the
  internal qualifier unification was *not* needed (links carry the
  fact across the one boundary it crosses).
- **L7b delivered 2026-09-02**: the `once` call-multiplicity qualifier
  (see the decision-log section) — landed on *fn types* rather than in
  deduction lists (the surface originally sketched here; user approved
  the change): `fn run(f: once () -> None)`, enforcement by
  consumption, inverted subtyping (flagged for review),
  consuming-capture lambdas legal and `once`-typed, Rust `impl FnOnce`.
  Inference (a callee auto-promoting a once-called fn param) remains
  open for the fn-type-contracts sub-phase.

Sequencing note: L1 lands in stages (S1 strict checker-only → S2
move-mode bindings → S3 borrow emission); S1+L2 closed real
rustc-rejection gaps; L3 and L4 are done (2026-09-02);
L5 is largely subsumed by shared fate (field-disjoint precision only);
L6 landed 2026-09-02 with its own LANGUAGE.md section and the
[linear-*] rule family. Per AGENTS.md, each phase lands with
LANGUAGE_SPEC.md rules and tests at every affected layer.

### L8 — Composition and conditional linearity

**Open** — see ROADMAP.md. How an obligation travels through a container, and
when a container is linear at all.

## Roadmap: effects

### E1 — Effect-to-effect dependencies (✅ complete 2026-09-04)

A handler may depend on another effect. [effect-member-no-effects] still
forbids *members* declaring effects, because dispatch goes through the
handler instance and the call site has no way to thread extra handler
arguments; a handler's dependency is declared on the handler instead.

**Where the dependency is declared changed during design** (user decision
2026-09-04). The first sketch put it on the *effect* (every handler
mirroring it exactly, as a `define fn` mirrors an external). The user's
objection stands: dependencies are a property of *implementations* —
`ConsoleLogger` needs a Console, a null logger needs nothing — and an
effect declaring them forces every handler to pay for the union. So the
dependency is a handler **constructor parameter of effect type**:

```
handler ConsoleLogger(console: Console) of Logger {
    fn log(message: Str) -> None => message { print(message) }
}
```

This already parses and type-checks (handler ctor params exist; only the
Rust emission of handler-typed values was broken — see the prerequisites in
E1a), so the declaration side of E1 needed almost no new syntax. What it
needed was: an effect-typed ctor param resolved from the ambient `use` set,
those effects placed in the member bodies' effect environment, and the
fusion emission ([rs-effect-fusion] / [kt-effect-fusion]) — all of which
landed 2026-09-04, on both backends, verified by running the same programs
under kotlinc and rustc to the same stdout.

- The mirroring principle still applies where the compiler cannot see an
  implementation: a handler must implement every member of its effect, and
  an `external handler`'s templates are trusted exactly like a `define
  fn`'s — declaration is the contract, no inference.
- Effect *member* deduction contracts (declared on the member, applied at
  call sites) landed with [decl-explicit], the deduce pass reads them
  ([call-resolve]), and handler bodies are validated against them
  ([deduce-infer], [effect-state-store]).
- Three cuts remain on the Rust side, all reported: a dependent handler
  using its own generic parameters in a member signature, a `use` whose
  effect instance is still generic, and a fn *value* that uses an effect
  passed to a callee that needs one ([rs-effect-fusion]). Kotlin accepts
  all three.

### E1a — Ownership strategy: explored options (✅ settled 2026-09-04)

Kept as the record of *why* B9 was chosen; skip to B9 for the plan.

E1's mechanics are all present (handlers already travel as leading
arguments, resolved per call site from `call_effects`); what is *not*
settled is how a dependent handler reaches its dependency in **Rust**,
where mutable state has exactly one usable path at a time and every
handler member is `&mut self` today. Kotlin has no problem here — objects
alias — so this is a one-backend constraint that nevertheless decides the
language rule, because the rule must hold on both.

Six strategies were explored, each verified by compiling the emitted shape
with `rustc` (and `kotlinc` where Kotlin was the constraint) rather than by
reasoning. **B9 (handler fusion) was chosen**; the others are kept because
their failure modes are the argument for it:

- **B1 — capture as a borrow** (`struct LoudLogger<'a> { console: &'a mut dyn Console }`;
  the lifetime stays inside generated Rust, invisible in Salvo).
  Compiles, but `&mut` exclusivity means registering the logger *locks*
  the console: a later direct `println` is `E0499`. Kotlin accepts the
  same program, so parity requires Salvo to adopt the restriction as a
  rule — expressible in existing vocabulary (capture = fate link, member
  call = mutation, so the existing poison rules produce Rust's answer),
  and block-scoped `use` is the remedy (`{ use LoudLogger(); … }`, then
  direct use after the block — verified).
- **B2 — shared ownership** (`Rc<RefCell<dyn Console>>`). Rejected: it
  turns a compile-time question into a **runtime panic**, which breaks
  parity by construction. Cycle detection is *not* sufficient, which was
  the surprise: `println("${bump()} ${bump()}")` — legal Salvo today —
  panics with "RefCell already borrowed" because Rust temporaries live to
  the end of the *statement*, so two guards coexist with no cycle
  anywhere. Reentrancy also arrives through ordinary functions, not just
  declared dependency edges. Three separate guarantees would be needed
  (cycle rejection, a handler-reachability rule, and a statement-hoisting
  invariant in the emitter), and a gap in any of them is a crash in a
  user's program instead of a diagnostic.
- **B3 — ownership at construction** (`use LoudLogger(StdOutConsole())`,
  handler owns a `Box<dyn Console>` or a monomorphized `C: Console`). No
  lifetimes, no runtime checks, and Kotlin already emits this shape
  correctly. Cost: the dependency comes from an explicit argument rather
  than the ambient environment, and a *stateful* dependency cannot be
  shared with the surrounding scope — you get two instances.
- **B7 — "B semantics, A mechanics"**: store nothing; thread the
  dependency closure through emitted signatures, checking the requirement
  at the `use` site instead of the call site. Verified with a three-level
  chain (`Audit` needing both `Logger` and `Console`, `Logger` needing
  `Console`, `main` interleaving direct use): one console, state intact,
  no exclusivity, no lifetimes. It works because a `&mut` passed as an
  argument is a *reborrow* whose duration is the call — the lender is
  suspended, so five frames can reach the value while only the innermost
  uses it. The failure of B1 is the same fact seen from the other side:
  a *held* borrow lasts as long as the holder, so it overlaps.
  **B7's constraint** is that the dependency closure must be a static
  property of the *effect type*, since a fn declaring `[Logger]` has one
  emitted signature. That forces dependencies to be declared on the
  effect, so every handler pays for the union (`NullLogger` receives a
  Console it ignores).
- **B8 — per-handler dependencies + specialization** (costed below).

**The user's objection to B7** (2026-09-03): dependencies are a property
of *handlers*, not effects — `LoudLogger` needs a Console, `FileLogger` a
filesystem, `NullLogger` nothing. Correct as interface design, and it
rules out B7's static closure. The tempting middle road (declare on
handlers, thread the per-effect *union*) collapses: if the union for
`Logger` includes Console, `use NullLogger()` would have to supply one,
which is exactly the unpredictable rule to avoid.

Framing that drove the exploration: **Rust appeared to demand one of
exclusivity (B1), duplication (B3 / B8), or a runtime check (B2)** — every
option a different concession. B9 escaped the trilemma by changing *what
holds the dependency*: a fusion that owns the handlers as **distinct
fields** can lend each one separately, so a shared dependency is threaded
per call instead of held for a scope. The lesson worth keeping is that the
binding constraint was never "who may reach this value" but **how long
each borrow lasts**.

#### B8 — per-handler dependencies with specialized consumers (superseded)

Costed and then superseded by B9, but two findings from the costing carry
over and are worth keeping:

- **Handler selection is lexically static.** `check_use` accepts only a
  handler *name* or a *constructor call*, resolved against
  `scope.handlers` rather than locals, so a variable holding a
  runtime-chosen handler cannot be registered. Every call site sits in a
  statically known set of `use` scopes. Any strategy that resolves
  handlers at compile time depends on this.
- **Nothing in *checking* depends on which handler serves an effect.**
  `[effect-disambiguation]` resolves by effect *instance type*, and a
  member call's contract is the *member's* declared deduction list
  ([decl-explicit]). So handler-aware work belongs to emission; type
  checking stays single-pass and handler-agnostic.

B8's own mechanism — one emitted copy of a *user function* per handler
binding it is reachable under — was rejected because it duplicates
arbitrarily large functions. B9 keeps per-handler dependencies without any
duplication of user code.

#### B9 — handler fusion ✅ chosen (user decisions 2026-09-03/04), shipped 2026-09-04

The strategy of record, now implemented on both backends. **Reshaped
2026-09-14 to the Has-accessor design** (user decision — see the decision
log's "Has-accessor effect fusion" entry): the fused value now implements
generated per-effect accessor traits/interfaces instead of the effect
traits themselves, the single-effect case fuses too, and Kotlin's planned
uniformity fusion became real. Everything below about *why* fusion beat
B1–B8 stands unchanged; the specs hold the current shapes. Full mechanism,
with the borrow reasoning and the
verified shapes, is in **BACKEND_SPEC.rust.md [rs-effect-fusion]** (the
constraint is Rust's) and **BACKEND_SPEC.kotlin.md [kt-effect-fusion]**.
In brief:

- Effect traits/interfaces stay **dependency-free**, so a user fn is
  emitted **once** no matter which handlers flow in.
- A handler's dependencies are its **constructor parameters of effect
  type** — `handler ConsoleLogger(console: Console) of Logger`, which
  already parses and type-checks today, so E1 needed almost no new
  declaration syntax. The dependency reaches the member body as an extra
  parameter — on Rust through a generated `__Impl_H` trait carrying the
  bodies, since `impl Logger for H` has no room for it.
- One **fusion** per `use` scope holds the registered handlers and exposes
  each member, hiding dependencies by passing them from its own fields.
  In Rust the forwarding impl destructures `&mut self` into **disjoint
  field borrows**, which is what makes a *shared* dependency work — every
  earlier option needed two `&mut` to the same place.
- A fn needing several effects takes **one fusion value**: a multi-bounded
  generic on both backends. Rust cannot use `dyn` here — a `dyn` fused value
  cannot be forwarded to a callee needing a *subset* of the effects — so the
  blanket-impl conjunction trait survives only as the type of the fusion's
  provider field ([rs-effect-fusion]).
- Nested `use` scopes **rebuild flat** (the inner fusion holds the outer
  scope's *handlers*, not the outer *fusion*): depth-independent
  threading, no forwarding hops, resolution explicit in the construction.
  **Revised at implementation time on Rust** (2026-09-04): flatness is not
  achievable there — effects inherited from a fn's fused *parameter* are not
  handler locals, and an inner fusion re-borrowing the outer scope's locals
  makes the outer fusion unusable after the inner block. Rust chains through
  one `__outer` field instead, which preserves the property this bullet was
  really about (threading is identical at every depth) at the cost of one
  forwarding hop per level; see [rs-effect-fusion]. Kotlin still rebuilds
  flat [kt-effect-fusion].
- **Facets are Kotlin-only**: erasure forbids one class implementing
  `Random<Int>` and `Random<Double>`, so Kotlin generates a non-generic
  facet interface per instance with the type argument in the member name.
  Rust needs none of it. Each backend leverages its own language rather
  than sharing one lowest-common-denominator shape.

Why this beats everything above it: dependencies live on handlers (the
user's interface-design objection to B7 is honoured), nothing is captured
for a scope so there is **no exclusivity**, no `Rc`/`RefCell` so **no
runtime failure mode**, no user-function duplication, no lifetimes visible
in Salvo — and, because a handler member can mutate a dependency it
receives as a parameter, **neither the immutable/mutable effect
distinction nor `Cell` is needed** to keep effects testable. It is the
only option whose supporting features turned out to be unnecessary rather
than merely deferred.

Prerequisites: **handler values** ✅ done 2026-09-04 (see the decision log
entry below); **effect dependency cycles** must still be rejected at
declaration time, which only becomes reachable once dependencies can be
declared.

Open sub-decision carried forward: **effectful fn values**. A named fn
passed by value needs its fusion baked in (a closure). Fn-type effect
lists parse but are unenforced today, so this is a pre-existing hole that
B9 turns into a decision point rather than creating.

### E2 — Heuristics for validating external declarations (deleted 2026-09-09)

**Obsolete, not deferred.** E2 proposed heuristics that read a *define template*
and warned when it contradicted the `external fn`'s declared contract — a
template mutating a kept-immutable parameter, moving a kept argument, or doing
I/O under `[]`. `external`/`define` were deleted in the 2026-09-05 interop
redesign, so there is no template to read, and the job is now done twice over
without heuristics: an intrinsic's lowering is Rust code inside a backend, with
tests, and a `platform effect`'s host implementation is checked against the
generated interface by the *target's own compiler* — which was the point of the
redesign. See the decision-log entry at the top of this file.

### E3 — Non-resumption: `defer`, `throw`, and an intrinsic `try` (user decisions 2026-09-04)

> **`defer` was deleted from the language 2026-09-10** (user decision; see the
> decision log). What follows is the record of building it, kept because the
> reasoning — why splice-at-exit rather than a queue, why it discharges a linear
> obligation on every path, and the two lowerings — is what the deletion was
> weighed against. Its syntax no longer parses.

The first slice of *handler control* beyond "always resumes at the tail",
which is all E1 supports. The exploration ran through four rungs of handler
power — tail-resumptive (today, free), throw (resume zero or one time),
suspend (resume later), multi-shot (resume repeatedly) — and settled on
building the second, with the third deferred to an *explicit* async effect
and the fourth ruled out.

**Multi-shot is closed on principle, not for want of a mechanism**:
resuming twice duplicates a use obligation, so it cannot coexist with
`canbe linear`. (It is also unavailable on both targets: a Kotlin
`Continuation` throws on a second resume, and a Rust `Future` cannot be
cloned.)

**No silent function colouring** (user decision 2026-09-04). Some colouring
is *inherent* to non-resumption — the code between the operation and the
delimiter must not run, so either the return shape changes, the stack
unwinds, or the function is split. The rule that keeps it honest: the
ability to not resume is **declared on the effect member**, never
discovered from the handler. That is forced anyway by the E1 principle that
a user fn is emitted once whatever handlers flow in — the shape of `work()`
cannot depend on which `Logger` is registered — and it means the colouring
is exactly the effect annotation the author already writes. The Rust
backend's `async`/`suspend` transform is *not* how this is built; async
arrives later as an explicit effect (likely a compiler intrinsic).

#### The design as decided

```
qualifier Thrown<M> of M            // mirrors `Err<T> of T` in core.result

effect Throw<M> {                    // intrinsic; message is moved, like `err`
    fn throw(message: M) [] -> Nothing => !message
}

// `try` is a compiler intrinsic, not an effect:
try { body } : Ok T | Thrown M
```

- **`try` is an intrinsic, not an effect** (user decision 2026-09-04):
  "there's not much value in a function declaring the `Try` effect in its
  signature any more than there is in declaring that it uses loops or
  if-expressions". So no `Try` handler to register, no `[Try]` in
  signatures, and — see below — no collision with the fusion.
- **Both arms are qualified: `Ok T | Thrown M`** (user decision
  2026-09-04), reusing `Ok` from `core.result` so ordinary `is` checks and
  exhaustive `when` work on the outcome exactly as they do on a result.
  `Err` is deliberately *not* reused: a throw is not an error value.
- **`Thrown M` is parameterized by a message type** (user decision
  2026-09-04), mirroring `Err`. It follows that the outcome union is
  structurally an `Ok T | Err M`, so union arm identity, narrowing and
  exhaustiveness need no new rules.
- **`Thrown M` is forgeable, deliberately** (user decision 2026-09-04):
  the qualifier carries no *authority* — a hand-written `-> M as Thrown`
  produces a value in the thrown arm but transfers no control. The
  authority is `[Throw<M>]` availability alone, which is why the original
  sketch's "only `throw()` may construct it" rule turned out to be
  unnecessary. No provenance semantics, no intrinsic qualifier.
- **`throw` returns `Nothing`**, which is what keeps intermediate frames
  silent: a fn that may throw declares `[Throw<Str>]` and returns `Int`.
  It does *not* also return `Thrown` — that would be `Result` plumbing
  with extra steps and would defeat throw being an effect. `Thrown M`
  appears in exactly one place: the `try` outcome.

#### Lowering

Rust: the message type *is* `ControlFlow`'s `Break` type, so the
propagation falls out of the design rather than being imposed on it.
`throw(m)` is `return ControlFlow::Break(m)` — no handler, no dispatch, no
allocation — every call in a fn with `[Throw<M>]` is `f(..)?`, and the
intrinsic converts at the delimiter:

```rust
pub fn parse(line: &String) -> ControlFlow<String, i32> { … }

// try { … }
match (|| -> ControlFlow<String, i32> { … })() {
    ControlFlow::Continue(v) => Union2::U1(v),
    ControlFlow::Break(m)    => Union2::U2(m),
}
```

Kotlin: a private stack-trace-less signal, with the delimiter's identity as
an unforgeable token — nesting must not let an inner `try` swallow an outer
throw:

```kotlin
private class Throw_Signal(val message: Any?, val token: Any) :
    RuntimeException(null, null, false, false)
```

Mechanism divergence with behavioural parity, the same reasoning as facets
and unions: `?` returns through each Rust frame running `Drop`, the JVM
unwinds running `finally`, and nothing user-visible happens on the way out
either way — *provided* `defer` is what puts code on that path.

#### `defer` comes first (user decision 2026-09-04) — **done 2026-09-04, deleted 2026-09-10**

Not tidiness; three reasons:

1. **It turns a prohibition into a pattern.** Without it, a linear value
   live across a may-throw call cannot discharge its obligation on the
   throw path, so the checker would have to forbid the combination. With
   `defer close(f)` the author discharges on every path and the existing
   flow analysis can count it.
2. **It proves both backends can run code on a throw path** before
   anything depends on that. The two lowerings are exactly the two throw
   mechanisms' unwind paths: a `Drop` guard in Rust (whose reverse
   declaration order gives LIFO for free) and nested `try/finally` in
   Kotlin.
3. **It exercises the capture machinery** the `try` body needs, on a
   smaller independently testable feature. Its own open question is
   whether a deferred body captures by move or by reference — the
   fn-boundary contract question again.

Doing throw first would mean revisiting its lowering to add guards and
`finally` afterwards: the interesting part, twice.

**Built 2026-09-04** — see the decision-log entry at the top for the four
user decisions and the shape it landed in. What the sketch above got wrong:
the `Drop` guard is not a usable Rust lowering (it must own the value from
the `defer` onward, killing the very pattern), and the capture question
does not arise at all under splice-at-exit. What it got right: it does turn
the linear-across-an-exit prohibition into a pattern, and both backends
demonstrably run code on the way out of a block — the guarantee `throw`
now builds on. Reason 3 (exercising the capture machinery the `try` body
needs) is *not* discharged: nothing was captured, so `try`'s body closure
is still unexercised ground.

#### Consequences to settle before building — **all settled 2026-09-04**

- **`M` inference collides with [call-type-args].** ✅ Settled by the
  generalization: `M` is the **union** of the message types the body
  performs (user decision), and a body that cannot throw at all is an
  *error* rather than `Thrown None` (user decision) — so nothing has to be
  inferred from an empty set. The union costs a wrap at each propagation
  site on Rust (`?` needs identical `Break` types) and a tag dispatch at the
  catch on Kotlin; both are in the backend specs.
- **A `Nothing`-typed expression statement must count as terminating.** ✅
  Done, and it went further than "small": both path analyses
  (`block_returns` for [fn-must-return], `block_exits` for branch merging)
  became *type-aware* Checker methods reading the recorded types, and a
  written `Nothing` now lowers to the bottom type rather than a nominal
  type spelled that way. Without the second half, `throw(n)` in a branch
  leaked its consumption of `n` to the fall-through path.
- **The intrinsic couples the compiler to two core qualifier names.** ✅
  `Ok` and `Thrown` are resolved by name from the implicitly imported core,
  with a diagnostic naming the missing one; the compiler knows three names
  in total (`Throw` the effect, `Ok` and `Thrown` the arms) and nothing
  else about them.
- **Nested qualification is the honest consequence of wrapping in `Ok`.**
  ✅ Confirmed, both shapes tested: `Ok None`, and `Ok (Ok Int | Err Str)`
  taken apart through a binding at the inner type. `when` does reject a
  qualified-group subject, but that is not a dead end — the droppable
  qualifier rule unwraps it; see "Nested qualification" below for the two
  alternatives that were rejected.
- **Throw targets the innermost `try`.** ✅ [try-innermost]. Rust needs no
  token at all (the block label decides where a `break` lands); Kotlin's
  catch-all is innermost by construction, and its `else -> throw` rethrow is
  what an escaped function value would hit.

#### Nested qualification: resolved without new surface (2026-09-04)

`try`'s outcome makes two qualified-union shapes reachable — a
result-returning body (`Ok (Ok Int | Err Str) | Thrown Str`) and a union
message (`Thrown (Str | Int)`) — and `when` rejects a qualified-group
subject. That looked like a dead end and was reported as one; it is not.
The **droppable-qualifier rule already unwraps**: `Qual T <: T`, so a
binding at the inner type takes the level off, and both backends emit it
correctly (Rust a plain unwrap, Kotlin a cast the narrowing makes
unfailable — verified end to end, same stdout).

```
when nested {
    is Ok {
        let inner: Ok Int | Err Str = nested     // outer claim dropped
        when inner { is Ok { … } is Err { … } }
    }
    is Thrown {
        let message: Str | Int = mixed           // union message, same idiom
        when message { is Str { … } is Int { … } }
    }
}
```

Two alternatives were considered and rejected (user discussion
2026-09-04):

- **Merging nested qualifiers** (`Ok Err Str`) collides with an existing
  meaning: a multi-qualifier type is a *set of claims* (`Mut NonEmpty
  List<T>`), so `Ok Err Str` reads as both applying — a contradiction, not
  nesting. It would also flatten the outcome union's arms, changing
  [union-arm-identity] and the wrapper representation, and cost `try` its
  uniform two-arm shape.
- **A dedicated "check and unwrap" keyword.** Two objections. It conflates
  a test with a *static* strip (inside `is Ok` the type is already known,
  so nothing needs checking), and — decisively — the exclusions it would
  need already exist: `Qual T <: T` is written "except `once`", and
  `Linear` is never a use-site qualifier, so the annotation path gets the
  intrinsic-qualifier cases right for free. A keyword would re-implement
  that list and have to keep it in sync as intrinsics are added (`Cell` is
  next).

What *was* wrong is discoverability: the diagnostic dead-ended. It now
names the remedy — "`Ok (…)` is the claim `Ok` *about* a union: bind the
inner union to a local and match that" — and the binding keeps which level
is meant visible to a reader, which matters when the same qualifier name
appears twice. Sugar (a stripping form on `is`) stays open, but it must
reuse the subtype rule's exclusions rather than inventing its own.

#### Why the intrinsic dodges a gate the library form would need

An earlier sketch had `Try` as an ordinary effect whose member takes the
body as a lambda. That form needs work this one does not:

- Fn-type **effect lists are parsed and dropped** today, and lambda bodies
  check under the enclosing fn's effect environment (lexically). "The
  lambda gets an effect its enclosing scope does not have" — the core of an
  *effect transformer* — is therefore not expressible yet, and neither is
  the dual guarantee that a body carrying `[Throw<M>]` cannot outlive its
  delimiter. [effect-not-data] already stops the capability escaping as a
  *value*; the lambda case is what remains.
- It collides head-on with the fusion cut landed 2026-09-04: a realistic
  body performs other effects (`try { println("x"); throw("bad") }`), so
  the lambda would capture the fused value while the call to the `try`
  *member* borrows it too — exactly the reported `E0499` shape.

Both point at one piece of work: enforce fn-type effect lists, pass effects
*into* fn values instead of capturing them, and forbid their escape. That
gate is also what rung 3 needs and what would lift the fusion cut, so three
motivations converge on it — but the intrinsic `try` needs none of it,
because the compiler generates the body closure and can pass the fused
value in.

**Effect transformers** — an effect member that runs a fn-typed parameter
with *additional* effects available, its body having registered handlers
for them — remain the general prize (`Retry`, `Timeout`, and async itself
are the same shape). Worth building once the gate exists and there are two
customers rather than one; `try` could then be re-expressed as a library
transformer if that reads better.

#### Sequencing

1. ~~`defer` — standalone, testable, settles linear-on-throw first.~~
   **Done 2026-09-04** ([defer], [defer-no-escape], [kt-defer-finally],
   [rs-defer-splice]).
2. ~~`throw` returning `Nothing` + intrinsic `try`, with `Thrown<M>` in
   std.~~ **Done 2026-09-04** ([throw], [try], [try-innermost],
   [throw-not-main], [throw-linear], [rs-throw-controlflow],
   [rs-try-label], [kt-throw-signal]). What the design got right and wrong
   is in the decision-log entry at the top; the short version is that the
   `ControlFlow` propagation and the `defer`-first ordering both held, and
   the closure lowering for `try` did not.
3. ~~Enforce fn-type effect lists; pass effects into fn values; forbid
   escape. (Lifts the fusion cut, opens transformers.)~~ **Done
   2026-09-04** ([fn-effects], [rs-fn-effect-params],
   [kt-fn-effect-params]) — with "forbid escape" dropped as unnecessary (a
   fn value carries no capability) and the fusion cut duly lifted.
   Transformers are now unblocked.
4. Transformers, and async as the second one.

Before (2), hand-write and run the Rust shapes for the three compositions
that decide whether propagation stays clean: a may-throw call inside a
loop, one inside a nested `try`, and one with a live linear value plus a
`defer` across it. Loops are the specific reason CPS-splitting into
continuation legs was rejected for this rung — a `perform` inside a loop
becomes recursion through the continuation, and neither target guarantees
tail calls, so ten thousand iterations means ten thousand frames unless you
hand-build a trampoline, which is the async machinery under another name.
The legs idea is right for rung 3, where the continuation must be reified
anyway; there its cost (a boxed closure per call, answer-type erasure) buys
something.

#### Step 4 — effect transformers

**Open** — see ROADMAP.md. The gate step 3 built is in place, so what remains is
the surface; async arrives later as an explicit effect.

## Roadmap: iterators — `Iter<T>` laziness (**DECIDED and built 2026-09-05**)

`Iter<T>` mapped to Kotlin `Iterable<T>` (lazy, re-iterable) and Rust
`Vec<T>` (eager, materialised). That was a [backend-parity] violation, not a
representation detail: the same program printed different things. Measured
2026-09-05 with a `yield` fn that prints per element —

| | Kotlin | Rust |
|---|---|---|
| created, never consumed | nothing | producer runs |
| consumed once | producer/consumer interleave | producer fully, then consumer |
| consumed twice | producer re-runs | second pass replays the buffer |

The obstacle was never the `Iter` mapping, which is easy on both sides, but
the body of a Salvo `yield` fn: on the JVM the emitted `iterator { … }`
builder suspends and resumes for free, while stable Rust has no generators,
and — the part that decided the design — a suspended iterator must hold the
fn's *effect handlers* across the suspension. Emitted Rust takes them as
`&mut dyn E` borrowed for the call, which a returned iterator cannot
outlive.

The four options as costed, with the user's choice marked:

1. **Eager on both** — Kotlin materialises. Parity by restriction, small
   change, no lifetimes. Loses lazy chains *and* infinite generators.
2. **Lazy where free, eager where not** — `Iter<T>` a repeatable factory,
   std combinators as intrinsics over each target's lazy adapters; a
   user-written `yield` fn stays eager on both. Parity exact, no new rule,
   moderate cost. Loses user-written lazy generators. (This was the
   recommendation.)
3. ✅ **Lazy on both, effects forbidden in `yield` fns** — a checker
   restriction buys `'static` generators, so no lifetimes; Rust lowers the
   body to a state machine. Infinite and lazy generators work; effectful
   ones are rejected on *both* backends. **Chosen** (user, 2026-09-05): "I'm
   happy with iterators needing to be effect-free, since other functions can
   just use them with effects via for-loops."
4. **Lazy on both, lifetimes in the Rust output** — nothing lost, nothing
   restricted, but lifetimes propagate through returns, locals, struct
   fields and unions. Largest change; not taken.

Built as [iter-effect-free] + [rs-iter-lazy]; the lead entry records what it
took. The second decision the costing flagged — whether a lazy `Iter` is
**one-shot**, which would make `for` consume its subject and interact with
linear types — did **not** have to be taken: keeping `SalvoIter` a factory
preserves Kotlin's repeatable semantics exactly. It is back on the table
under the next section, which supersedes this one's *implementation*
(the semantics it chose — lazy, both backends — are kept).

## Roadmap: iterators — **the reduction to `next`** (user decisions 2026-09-08; **all six phases done 2026-09-09**)

**Standing back from what was built.** After I4/I5 landed, the user asked for an
evaluation of the whole iterator design rather than the next increment, and the
outcome is a simplification that deletes most of it. The complaint was precise:
there were multiple ways to define an iterator (`next` or `iter`, with `for`
arbitrating), and `Iter<T>` had become "effectively a function in its own
right" — which is why `once` and effects both fit on it.

**The diagnosis, confirmed.** `Iter<T>` *is* a factory:
`Rc<dyn Fn() -> Box<dyn SalvoPass<T>>>` on Rust, `Iterable<T>` on Kotlin. So

```
once Iter<T>  ≅  a pass  ≅  (state St, next: (Mut St) -> Emitted T | Finished)
Iter<T>       ≅  () -> once Iter<T>
```

`Iter<T>` was a structural interface in a language with none — and, as the user
put it, putting one in a struct field smuggled a *stateful method* onto data,
against the separation of functions and data. `params Iterator<St, T>` was
already the un-smuggled version of the same thing; keeping both was the mistake.

**Two costs of the old design, found by probing rather than by report:**

- A hand-written pass (struct + `next`) and a `once Iter<T>` are drivable by
  `for` but **invisible to every combinator**: `for` speaks `next` while
  `map`/`filter`/`map_lazy` speak `?Iterable` → `iter` → `Iter<T>`. So the
  manual form that exists to express `zip`/`merge` cannot be mapped over.
  Verified: `map_lazy(Countdown {at: 3}, double)` fails with "no `iter` fits".
- `once Iter<T>` was written as a factory type, reasoned about as a position,
  and *represented* as a factory — the mismatch I2c's remaining half existed to
  fix.

### Decided (user, 2026-09-08)

1. **A pass is a user struct**, tied to iteration by a `next` function. No
   universal `Iter<T>` type. Getting a new pass is constructing a new instance.
2. **`for` is sugar for calling `next` until `Finished`.** One protocol, one
   lowering.
3. **`iter` converts a container into a fresh pass** (`fn iter(c: Counter) ->
   Countdown`), and stays available so `for x in list` keeps working for
   intrinsic containers (lists, arrays, later sets and maps) — declared in std
   as a pass struct plus an (intrinsic) `next`, with the backends keeping the
   native loop as a fast path. The language gets no special case; the emitters
   keep the one they have.
4. **A `yield` fn is sugar** that generates the pass struct and its `next`.
   *Refined 2026-09-08 (user): the generated struct is **hidden**.* The author
   declares an *origin* struct carrying `: Yield<T>` and writes
   `yield fn next(origin) -> T` — the return type is the element type, and the
   machine is not nameable. Whoever wants the state struct writes the raw
   `next`. See "Unknown 1: the origin-struct model".
5. **The element type is declared, not encoded in a name**, so `for` resolves
   `next` nominally and the reader sees what a pass yields where it is declared.
   Now folded into the `params`-obligation proposal below.
6. **`Iter<T>` in a struct field stops being expressible**, and that is
   accepted: erasure is what the language does not have.
7. **The `Defer`-effect question is deferred**, recorded below.

**What this deletes**: `[iter-effects]` and the whole claim-on-the-type
apparatus (`producer_effects`, `pass_effect_sets`, the generated
trait/interface per effect set, the variance adapter — effects go on `next`,
where `[fn-effects]` already handles them); `once` on producers (driving
mutates, so `Mut` carries it, and a second drive continues rather than
restarting); `SalvoIter`/`Iterable` as `Iter<T>`'s representation, with the
boxing and the whole un-boxing question; and `[iter-mut-param]` as an
*iterator* rule — a `yield` fn's parameters become fields of the generated
struct, so a `Mut` one is a struct-field question with pre-existing answers.

**What survives**: the parts that were about the machine rather than the type —
`generator.rs`'s plan, both `__advance` renderings, the release path, and the
two capture rules.

### Abandonment, under the reduction

The close problem does not go away; it **relocates**, from "a suspended body's
pending blocks" to "a value the driver holds", where the language already has
answers:

- **`close` is a second function overloaded on the pass type**, resolved
  exactly like `next`, called by the `for` sugar on every exit if one exists.
  Effects on it as usual, because it is a call.
- **Linearity makes it checked**: L6's must-use machinery (built 2026-09-02)
  turns draining-or-closing into the ordinary all-paths obligation. The
  abandonment write-up above already predicted this ("the injection is a
  *convenience* … it can be relaxed to a plain obligation whenever we want the
  user to see it") and left one thing undecided — *whether a hand-written
  iterator's state must be `canbe linear` by rule*. Under the reduction every
  pass is a named struct, so that is now the only question, and it is
  **decided (user, 2026-09-08): a `close` does not make a pass linear.** Only
  `: Linear` on the type does, and generic code that may carry one opts in with
  `<T canbe linear>`; R0's finding 4 is why (the alternative made most
  generated passes uncomposable). So `for` calls a `close` when one exists,
  while a hand-written `while` around `next` may abandon a non-linear pass
  unchecked — accepted, since `for` covers the path people write.

### Open: declaring the tie between functions and structs (user proposal 2026-09-08)

Instead of a bespoke `yields T` clause, a general mechanism: a **declaration-site
obligation** reusing `params`, with the declaring type written `self` in the
argument list.

```
struct Lines canbe Mut : Linear, Yield<Str>

params Yield<T> {
    fn next(s: Mut Self) -> Emitted T | Finished => s: Mut
}
```

Unlike a qualifier, the obligation *always* applies: declaring `: Yield<Str>`
without a matching `next` is an error at the **struct**, where today a
misspelled `next` surfaces as "not iterable" at the loop. It also makes `for`
resolution read a declaration instead of scanning overloads.

**The rule that keeps this from being an interface** (accepted by the user
2026-09-08): **no value may have a group as its type.** `items: Yield<Int>` is a
syntax error — there is no `dyn`, no erasure, no interface value; a group
constrains a *named* type and is resolved statically. This is the single
restriction that makes the mechanism a where-clause rather than a trait, so it
is a rule in its own right and not a consequence of one.

**Open sub-questions**: how a per-type effect set reaches the group's members
(`Lines`'s `next` performs `FileSystem`) — recommended and prototyped: each
implementation declares its own, the group declares none. **Answered
2026-09-08, revised the same day**: the state-as-parameter shape is the *only*
shape, and the declaring type is written `self` **at the obligation**
(`: Yield<self, Int>` against `params Yield<It, T>`). A magic `Self` *inside*
the group was built first and then withdrawn by the user, for a reason worth
recording: it made the group unusable with a `?` spread, since nothing binds
`Self` in a signature — and the spread is exactly the composition rendering
unknown 2 settled on. One group now serves both, so a designated group puts the
**state first, element second**. Also open:
whether obligations may ever be conditional (`Wrapper<T> : Yield<T>` only when
`T` yields) — recommended: unconditional only, to start; and `canbe linear`
stays the *type-parameter* opt-in beside `: Linear` as the *declaration*
clause, which is what makes the change additive.

### Decided (user, 2026-09-08): `Linear` becomes a compiler-known obligation group

Following `Yield<T>`'s precedent, **`Linear` is a designated `params` group with
a single member**, and declaring it is declaring how the obligation is
discharged:

```
struct Lines : Linear, Yield<Str> canbe Mut { handle: File }

params Linear {
    fn close(s: Self) -> None      // consumes: `s` is not kept => !s
}
```

- **The declarer must supply the `close`.** A type that says `: Linear` without
  a matching `close(Lines) -> None => !…` is an error at the *struct*.
- **`close` is the discharge.** `discard` no longer satisfies a linear
  obligation — it is refused, naming `close` — which closes the hole that made
  the separate-groups version wrong: `discard(lines)` satisfied linearity and
  leaked the handle. Moving the value onward still *transfers* the obligation,
  since the callee's body is checked for discharging it.
- **How the compiler knows which function**: the group is designated (one
  candidate group), at most one per type (one candidate declaration), and it has
  a single member (one candidate function). No scanning; every failure is a
  declaration-site error.
- **Linearity is therefore non-optional per type** — every `Linear` type has a
  `close` — and the user accepted that trade explicitly. **Conditional
  linearity is deferred**, and it is where `<T canbe linear>` reappears.

**Rules this rewrites** (none of them shipped to anyone — no compatibility
concern, user note 2026-09-08 "no-one is using this language yet"):

- `[linear-group]` (was `[linear-canbe]`) — the spelling moves from `canbe linear` to `: Linear<self>`, which
  also tidies `canbe`: it goes back to meaning only "may be qualified thus"
  (`canbe Mut`, `canbe once`), while `:` means "must provide these".
- `[linear-discard]` — `discard` stays for non-linear values; for a linear one it
  becomes a refusal naming `close`.
- `[linear-generics]` — `<T canbe linear>` **keeps its spelling** (user
  decision 2026-09-08, superseding this plan's earlier `<T: Linear>`
  suggestion). The two clauses say different things and should not be spelled
  alike: `canbe linear` on a *type parameter* is permission — "this generic may
  be instantiated with a linear type, and its body is checked under that worst
  case" — while `: Linear` on a *type declaration* is obligation — "this type
  provides a `close`". A bound is still not an interface: there is no value
  whose type is a group, so no erasure and no `dyn`.

**Decided (user, 2026-09-08): a `close` does not make a type linear.** R0's
finding 4 (below) forced this: the proposed answer to abandonment — *a pass
declaring a `close` must be linear* — collides with the interim composite rule,
because a composed pass stores its source and every `yield` fn with a `defer`
gets a `close`. Together they would have made most generated passes
unmappable. The rule instead:

- **`: Linear` on the type is the only thing that makes a value linear.** A
  bare `close` function is an ordinary function; the `for` sugar still calls it
  as the release path, and nothing is obliged.
- **Generic code opts in with `<T canbe linear>`**, as it does today, and
  **the concrete type must say `: Linear`** for the obligation to exist at all.
  So both halves are explicit: the function admits it may carry a linear value,
  and the type admits it is one. Neither is inferred from a member's presence —
  attaching an obligation on the strength of a function name is what [qual-*]
  keeps the compiler from doing, and it is the same argument that made
  hand-written passes declare `once` rather than have it inferred from `next`.
- **What it costs**: a pass with a `defer` (so, most generated ones) is *not*
  linear, so a hand-written `while` driver may abandon one without closing it
  and the checker will not complain. Accepted — the `for` sugar covers the
  path people write, and the alternative was worse.

**Open, and the first thing a user will hit**: `[linear-composite]` said a
composite containing a linear component is itself linear. **Interim decision
(user, 2026-09-08): storing a linear value in a composite is an *error*
for now** — struct field, type argument, array/tuple/union component alike —
rather than making the composite linear. Composition and conditional linearity
will be tackled together, and until then a linear value lives only as a local,
a parameter or a return value, which is all the iterator use case needs (open a
`Lines`, drive it, close it). **✅ Built as R4 part 2 (2026-09-08)** — see "R4
part 2 as built" for the six refusal sites and the one place the plan was
wrong.

- **What it costs to implement**: `ty_transitively_linear` is exactly the
  predicate that *detects* a linear part, so it survives — what changes is its
  consumers: the places that today propagate the obligation into the container
  become a refusal at the store (or at the composite's declaration, where the
  message can name the field).
- **Known casualty, worth deciding before S-IO resumes**: the fallible-open
  shape. S-IO's outline settled on `Ok InputStream | Err Str` because an effect
  member may not declare `[Throw<M>]`, and it flagged "a linear value inside a
  union arm" as the interaction to verify first. Under this refusal that shape is
  unavailable, so S-IO needs either an exception for union arms or a different
  result shape. Not a reason to change the interim rule, but it should not be a
  surprise when the filesystem work restarts.
- **Second known casualty (user accepted 2026-09-08, recorded for
  reconsideration): a linear pass cannot be composed.** `map_lazy(open_lines("a.txt"), f)`
  is refused, because a composed pass stores its source and `Lines` is
  `: Linear` — so mapping, filtering or zipping over a file's lines, which is
  the canonical reason to want lazy sequences at all, is unavailable. The
  interim shape that *is* available is to drive the linear pass by hand and
  yield from inside a `yield` fn, which re-wraps it rather than composing it.
  The user accepted the interim uncomposability rather than widening the rule
  now; it should be first in the queue when composition and conditional
  linearity are tackled together, and it is the concrete test case to design
  against — "can you `map` over a file?" answers whether the composite rule
  landed usefully.


### Implementation plan: the reduction, in six phases (written 2026-09-08)

Every phase ends **green** (`cargo build` warning-free, `SALVO_E2E_FRESH=1 cargo
test` passing) and is worth committing on its own. The order is chosen so that
`Iter<T>` keeps working until the thing that replaces it is already carrying
load — the same sequencing I2a/I2b/I4 used, which is what let each of those land
without a half-migration.

**R0 — prototype the target shape on both backends first.** The discipline that
paid off in I1b, I3 and I4 (each time turning up something the design did not
predict: the variance adapter, the `Option`-slot rule, the `close`-is-idempotent
economy). Hand-write, in `experiments/next-reduction/`, one Salvo source plus
the Rust and Kotlin a correct emitter would produce, and run both to
byte-identical stdout with no toolchain warnings:

- a hand-written pass (`Countdown` + `next`), driven by `for`;
- a `yield`-generated pass, including one with a `defer`;
- a **composed** pass (`map_lazy`-shaped: a pass holding another pass), which is
  where the type-parameter cascade shows up;
- a **linear** pass (`Lines` + `close`) driven to exhaustion and abandoned early;
- a `for` over a `List` (native loop) beside a `for` over a pass.

#### R0 as built (2026-09-08): six shapes, one stdout, and five findings

Done. `experiments/next-reduction/` holds `passes.sv` (the planned language),
`passes.rs`, `passes.kt` and `expected.txt`; both toolchains build them and
print the same 30 lines, verified with `diff`. `kotlinc` says nothing at all
and `rustc` says nothing under `#![allow(dead_code, non_snake_case)]`, both of
which are in the emitter's own header. The program covers all five required
shapes plus a sixth — the composed pass is prototyped **twice**, because that
is where the design had a fork nobody had named. The README records what each
shape proves; the findings, in the order they matter:

1. **Composition only breaks when the source's type is a *parameter*.** A
   non-generic composed pass already works in today's language — a struct with
   a `Mut Countdown` field whose `next` calls `next(d.src)`, compiled and run
   on the Rust backend. What is refused is the generic form:
   `next(Mut P)` on an unbounded `P` is `no matching overload`, exactly as
   [call-resolve] promises. So **unknown 2 reduces to one question**: how does
   `next` become reachable through a type parameter?
2. **It has two answers, both prototyped, and they are not equivalent.**
   **(A)** the group as a *generic bound* — a trait/interface with one `impl`
   per declaring type, `P: SalvoYield<T>`, static dispatch, and no value ever
   typed as the group. **(B)** the group's member as an *implicit parameter*,
   stored as a function value — which is not new machinery, since the existing
   generated `map_lazy` already stores its `?Iterable` member that way on both
   backends. (B) wins on two counts the type rules do not hint at: an
   **effectful** `next` takes handlers as parameters and therefore cannot
   implement a fixed trait method — the "one trait per effect set" problem
   [iter-effects] had, resurfacing at the bound — while a stored fn's *type*
   carries the effects and [fn-effects] inheritance already handles them; and
   `close` is **optional at the call site** under (B), where under (A) it is a
   `P: Linear` bound a pass without a `close` cannot satisfy. The prototype's
   section 4 composes over the effectful `chatty`, which only (B) expresses.
3. **A consuming `close` moves the idempotency rather than removing it.**
   `fn close(s: Self)` cannot be called twice, so I1b/I4's "one call after the
   loop covers `break` and exhaustion" reasoning is gone. Two things survive:
   the per-`defer`-site **flag guard inside** `close` (the body's own
   exhaustion path may already have discharged it), and — unpredicted — a
   composed pass needs its source in an **`Option`/nullable slot**, because
   the release path only holds `&mut self` while closing the source consumes
   it. Same shape as I3's no-zero-value slot rule, arriving from ownership
   instead of initialization.
4. **Unpredicted collision: `close`-implies-`Linear` would make most generated
   passes uncomposable.** The proposed abandonment answer (*a pass declaring a
   `close` must be linear*) meets the interim rule (*storing a linear value in
   a composite is an error*), and a composed pass stores its source — so every
   `yield` fn with a `defer` would be unmappable. **Decided (user,
   2026-09-08): the two are independent** — `: Linear` on the type is the only
   thing that makes a value linear, generic code opts in with
   `<T canbe linear>`, and a `close` implies nothing. `Chatty` has a `close`
   and is not `Linear`; `Lines` declares `: Linear`. Accepted cost and
   casualty are recorded under the `Linear`-group decision and as roadmap L8.
5. **Smaller, all verified.** `Mut` on a user struct is only reachable through
   a qualified struct literal (`Mut Countdown { at: 3 }`) — a plain literal
   cannot be coerced and `for x in Mut P { … }` is a parse error, so a pass
   always arrives from a call or a local. A **fn-typed struct field was refused
   on Rust and accepted on Kotlin** — a live backend divergence, and the
   reason (B) was a *generated*-pass mechanism; ✅ lifted in R5 as predicted
   ([rs-fn-field]: `Rc<dyn Fn…>` was all it took). Calling a
   fn-typed field needs a local first (`h.f(e)` is `f(h, e)` [fn-dot]). A
   generic struct literal needs written type arguments. And Rust's overload
   mangling reaches `close` too (`close__2`), so the `for` sugar needs **two**
   resolved functions handed over per subject where `for_drivers` carries one.

Also confirmed, on the credit side: Kotlin needs **no runtime file at all**
under the reduction (`SalvoPass<T>` existed for the `hasNext`/`next` lookahead,
which `Emitted T | Finished` makes unnecessary), and Rust's `Clone`/`'static`
bounds survive only on *stored callbacks*, not on passes — they came from the
factory.

**R1 — obligation groups, as a mechanism.** `: Group<Args>` on a struct
declaration; `Self` inside a `params` group; declaration-site checking (declared
without a matching function is an error naming the missing signature); the
no-value-of-group-type rule. Designate nothing yet — prove it with a group
declared in a test. Nothing else in the language changes.

#### R1 as built (2026-09-08): the mechanism, designating nothing

Done, green (`cargo build` warning-free, full suite passing). The mechanism
exists and is proven with groups declared in tests; the compiler designates
nothing yet.

- **Syntax**: `StructDecl.obligations`, parsed from `struct Name<G> : A,
  B<Args> canbe … { … }` — obligations before `canbe`, because `:` states
  what the type must *provide* while `canbe` states what it may be qualified
  as. Two parser tests.
- **`self`** [group-self]: the declaring type, written as a **type argument at
  the obligation** (`: Step<self, Int>`), bare and unqualified. First built as
  a magic `Self` *inside* the group and withdrawn the same day by the user, who
  spotted what it cost: a group whose members mention `Self` cannot be spread
  as `?Group<…>` implicits, and that spread is the composition rendering
  (unknown 2). With `self` as an argument the group is ordinary, so **one
  declaration serves both** — which is now a test
  (`the_same_group_serves_an_obligation_and_a_spread`). `self` anywhere else,
  including in a spread, is an unknown type; spread arguments are validated
  like any other written type, which is what reports it.
- **Declaration-site checking** [group-obligation] (`check_obligations`, in
  the `Item::Struct` arm): unknown group (with is-a-type/is-a-qualifier
  position hints), arity, duplicate obligation, and member satisfaction —
  the expected signature is the member's `Ty::Fn` with group generics bound
  to the written args — `self` among them standing for the declaring type with
  its own generics as variables — matched against every visible overload of the
  member's name
  **up to a bijective variable renaming** (`tys_match_renamed`): a generic
  `Zip<A, B>` satisfies through its own parameters, and `(A, A)` never
  matches `(A, B)`. Parameter names are the group's own, deliberately
  *unlike* [qual-refn-match]; effects are not compared (the group declares
  none, each implementation declares its own — R0 unknown 4's answer).
  Failures name the missing signature at the struct.
- **[group-not-a-value]**: a group name is refused in every type position
  (`reject_group_as_data`, mirroring [effect-not-data]'s shape). Two details
  worth keeping: `require_name` *suppresses* its unknown-type error for
  group names so one mistake gets one diagnostic, and the lowering of a
  group-typed annotation is `Ty::Unknown` so no mismatch cascade follows
  [type-unknown-lenient] — found by the return-position test, which saw
  "expected return type `Pair<Int>`, found `Int`" trailing the refusal.
- **Tests**: `salvo-core/tests/group_tests.rs` (12 — satisfaction incl.
  `Mut It` and the generic/bijective cases, wrong-shape, unknown/arity/
  duplicate, both `self` rules — one group serving an obligation *and* a
  spread, and `self` unknown outside an obligation — and the six-position
  not-a-value sweep) plus
  2 parser tests. One insta snapshot changed, by the new empty
  `obligations: []` field only.

**R2 — designate `Yield<T>`, and make `for` read the declaration.** A pass type
declares `: Yield<T>`; `for` resolves `next` from that declaration instead of
scanning overloads, and takes the element type from it. `Iter<T>` is untouched,
so both old paths still work; `iter_elem_ty`'s scan-for-`next`-then-`iter`
becomes read-the-declaration-then-`iter`. The driving *emission* already exists
(the `for_drivers` `while let` loop from I2b/I2c), so this phase is checker work
with almost no emitter work.

#### R2 as built (2026-09-08): `for` reads the declaration

Done, green (full suite 767, both toolchains' e2e recompiling the rewritten
sources to the same stdout). `std/core/iterator.sv` now declares the
designated group — `params Yield<T> { fn next(s: Mut Self) => s: Mut
Emitted T | Finished }`, replacing `params Iterator<St, T>` — and a pass is
**a type declaring `: Yield<T>`**, full stop.

- **`pass_elem_ty` is gated on the declaration** (`yield_obligation`): no
  clause, not a pass — the overload scan survives only to resolve *which*
  `next` drives (`for_drivers` still hands the emitters the overload + arm
  identity) and was extracted as `find_next_driver` so the not-iterable
  error can use it for a remedy hint: a subject with a matching `next` and
  no clause gets "declare `: Yield<Int>` on it to make it a pass".
- **The `once` requirement on hand-written passes is gone** (the I2b error
  and its remedy), replaced by the clause; the *consume* behavior stays —
  a declared-pass subject is `fate_move`d exactly as `once` subjects were,
  so driving twice is still the ordinary consumed-use error. Drive-in-place
  ("a second drive continues", `Mut` carrying single-use-ness) is recorded
  as the eventual semantics and deferred to R5 with `once`-on-producers'
  deletion — it changes deductions and emission, and nothing needs it yet.
- **Unsatisfied-but-declared stays lenient**: the obligation error already
  landed at the struct [group-obligation], so the loop answers the
  *declared* element type and adds nothing [type-unknown-lenient].
- **Builders return the plain type now** (`fn zip(…) -> Zip`), or `Mut` —
  both drive; `Once Zip` returns are gone from every test. The sweep was
  five failing tests: both backends' `PASS_DEMO`/`FALLIBLE_PASS_DEMO`
  (structs gain the clause — including `Yield<Ok Str | Err Str>`, pinning
  that a union element matches through the obligation checker — and
  builders drop `once`) and `once_tests`' hand-written-pass trio, plus a
  new `Mut`-built-pass test. `iter_effect_tests`' `canbe once` fixtures
  are [iter-effects] position tests, untouched until R5 deletes that
  apparatus.
- LANGUAGE.md's hand-written-iterator section and [iter-resolve]/
  [iter-protocol] in LANGUAGE_SPEC.md rewritten to the declaration form;
  the full [iter-*] rewrite stays R6's.

**R3 — retarget the `yield` sugar** (shape decided by the user 2026-09-08 —
see "Unknown 1: the origin-struct model" below). A `yield fn next(c: Counter)
-> Int` is the second way to satisfy a `: Yield<Int>` clause: the fn's subject
is the *origin* struct, its return type is the **element** type, and the
compiler generates a **hidden** state struct — the origin's captured fields
plus the plan's fields, a `next` that is the plan's `__advance`, and a `close`
when the body has deferred work. The generated struct is not nameable;
`for` over the origin mints one and drives it. `generator.rs`'s plan survives
unchanged — this changes what the emitters *render around* it and what the
checker accepts as a group member. `Iter<T>`-returning `yield` fns stop being
legal here, so this is where the corpus and most tests move.

#### R3 as built (2026-09-08): the origin-struct sugar, both backends

Done, green. `yield fn next(c: Counter) [Console] -> Int` is the second way to
discharge a `: Yield<Int>` obligation [yield-fn-origin], and the state machine
is **hidden**: not nameable, not constructible, not callable.

- **Syntax**: `FnDecl.is_yield`, a `yield fn` item form (`parse_fn_flavored`).
  Declared, not inferred — which is what lets the compiler tell this form from
  the `Iter<T>` one, and the reason the old form stays legal until R5.
- **Checker**: `yield_fn_for` finds the sugar member; `check_obligations`
  accepts it for `Yield` and refuses **both** forms on one type (the sugar
  generates the struct a hand-written `next` would be);
  `check_yield_fn_decl` validates the six declaration rules (named `next`, one
  parameter, origin not `Mut`, origin carries the clause, return type is the
  clause's `T`, body yields); `pass_elem_ty` records an *origin* `PassDriver`
  and checks the drive-site effects through `check_fn_value_effects` — which
  also records `call_effects` at the subject span, so the emitters thread the
  same handlers; `Expr::For` skips the `fate_move` for an origin, so driving
  consumes nothing and a second loop replays; and a **direct call** is refused,
  naming the remedy.
- **Emitters, both**: `emit_generator` split into a shared machine builder
  plus the factory tail, so the origin form is the *same* machine with nothing
  around it — no factory, no `SalvoIter`, no boxing, no trait. Named after the
  **origin** type (`__Pass_Counter`), because every `yield fn` is `next`. Rust:
  `pub struct` + `pub fn new/__advance/__close`, `for` constructing and
  splicing `__close` (plus the deferred entry, so a `return` releases it), a
  *place* subject **cloned**. Kotlin: a public class with no supertype and its
  own `__current`, `try`/`finally` for the close, and the origin shared by
  reference — the two agree because [iter-mut-param] refuses a transitively
  mutable origin, so there is nothing a copy could hide. Value-position origin
  loops are refused on both [backend-never-wrong], the same cut a claiming
  producer already takes.
- **Effects arrive by the ordinary route**: the `yield fn` declares them, and
  they are performed while driving — so `fn_effects` (minus `Throw`) is the
  handler list for the machine *and* the drive site, and [iter-effects]'
  claim-on-the-type apparatus is not involved. That is R0's unknown 4 answered
  by construction.
- **What the origin model bought, beyond the ownership cleanup**: a generated
  pass **cannot be abandoned**, because only the `for` sugar can hold one and
  the sugar always closes. The accepted "hand-written `while` driver abandons a
  `close`-bearing pass unchecked" cost now applies to *raw* passes only.
- **Tests**: `salvo-core/tests/yield_origin_tests.rs` (11) plus one e2e per
  backend over **one** source and **one** expected stdout — the R0 prototype's
  awkward shape (`Console` performed while the consumer drives, a `defer` that
  prints, the first loop abandoning after two elements, a second loop over the
  *same origin* replaying) — and a Kotlin emission-shape test. Both toolchains
  agree byte for byte.
- **Two fixture gotchas worth keeping**: an effect member is declared
  `fn println(message: Str) [Console]`, *not* taking the effect as a data
  parameter ([effect-not-data] catches it, but the message is about the
  parameter, so it reads like a different mistake); and a **struct literal
  cannot be a `for` subject** (`for n in Counter { start: 2 }` — the brace is
  read as the loop body), so a builder fn is needed. The second is a
  pre-existing grammar limitation, first hit in R0.
- **One standing rule to be aware of, unchanged**: `yield num` *moves* `num`
  [deduce-consume], so a body that decrements after yielding writes
  `yield copy(num)` — the same idiom the `Iter<T>` form has always used. Worth
  a decision at some point (a `Copy`-like exemption), but not R3's to take.

#### Mutable origins: the question, and the options (opened 2026-09-08)

Raised by the user: *the origin does not need to be mutable, but why should it
be required to be immutable?* Right on both counts, and checking it turned up
the defect above — the requirement is not actually enforced past the top level.

**What is in place**: a `yield fn`'s origin parameter may not be written `Mut`
(the sugar never advances it — the machine holds the position), and nothing
else. A non-`Mut` origin holding a `Mut List<T>` is accepted, and that is where
the two backends part company.

**What is actually problematic is not mutability — it is mutation *during* a
drive.** A pass reads the origin across suspensions, so if anyone writes to it
while the loop runs, "what does the pass see" has two defensible answers and
the two targets each pick one: Rust's machine holds a clone (the write is
invisible; the *next* loop sees it), Kotlin's holds the reference (the write
lands mid-iteration). Nothing about the origin's *type* decides that; the
overlap of a live pass and a write does. Which is why immutability is the wrong
thing to require: it is sufficient, and much stronger than necessary.

The options:

- **(a) Extend the transitive refusal** (what the specs wrongly claimed). Makes
  the docs true for one line of code. Cost: a struct with a mutable field can
  never be an origin, so "iterate the buffer I am holding" is unavailable — and
  an origin is *ordinary data the caller keeps*, which makes the shape far more
  natural here than it was for `Iter<T>`.
- **(b) Make the capture identical: Kotlin copies too.** A pass becomes a
  *snapshot* of the origin, on both backends, which is easy to state and needs
  no new checking — the emitters already generate deep copies for `copy()`
  ([kt-copy]: `MutableList` → a new list, `Mut Str` → `StringBuilder(sb)`).
  Cost: a real copy per loop, and the snapshot semantics are a choice the
  author cannot opt out of.
- **(c) Share on both.** Rust would need `Rc`/`RefCell` or a borrow with a
  lifetime — reintroducing exactly the machinery the reduction deleted.
  Rejected.
- **(d) Refuse mutation for the duration of the loop** — shared fate
  [fate-link] already expresses "the caller may not touch this while the pass
  is alive". Mutating the origin inside the loop is a *checker error*,
  clone-vs-alias stops being observable, and each backend keeps whatever is
  cheapest (Rust clone, Kotlin reference). A mutable-origin *type* stays
  perfectly legal — only the overlap is refused. This is also the answer the
  roadmap had in reserve: [iter-mut-param]'s open question says a producer
  taking a mutable parameter "needs a pass that *borrows* … which is what
  shared fate expresses", and the origin form is that pass.
  **✅ Built 2026-09-08** (user decision), closing the defect: hooked into
  `fate_mutation`, keyed on the subject's root so projections and `Mut`
  arguments both report, scoped to the loop. Four tests.
- **(e) Don't keep the origin at all** — the machine snapshots the origin
  fields it *uses* at construction, so the origin is read exactly once, before
  any suspension exists, and Rust's clone disappears.
  **✅ Closed 2026-09-10, in two steps**: the `iter fn` `state`-block work
  (2026-09-09) built exactly this field-level snapshot — the desugarer's
  rewriter is the complete expression walk the blocker asked for — and the
  same-file restriction on it was lifted 2026-09-10 (the CLI and LSP expand
  `iter fn`s program-wide, so a subject declared in another file snapshots
  per field too). The *machine* variant this entry was written against went
  with the `yield fn` deletion. Recorded as it stood, because two
  cheaper-looking routes to the same guarantee were blocked:
  * *Deep-copy the whole origin on both backends* — blocked on Kotlin, whose
    `copy()` lowering deliberately handles only the all-immutable struct case
    ([kt-copy]: a shallow `.copy()` would alias a `Mut List` field), so the
    interesting case is exactly the one it cannot express.
  * *Hold the origin by reference on Rust* — needs a lifetime on the machine
    struct (`__Pass_O<'a>`), which a temporary subject (`for n in make()`) has
    nothing to borrow from, so it would need a hoisted `let` plus a naming
    ripple.
  So (e) means a **field-level** snapshot: `FieldKind::OriginField { param,
  field }` in the plan, one per field the body reads, initialized in `new`,
  with both emitters rewriting `o.field` → the machine field. What makes it
  more than an afternoon is that the predicate ("every use of the origin is a
  plain field read") needs a *complete* expression walk: `collect_bindings_expr`
  is not one — it follows blocks and binding positions only — and a walk that
  misses a bare use would snapshot when it must not, emitting a reference to a
  field that does not exist. Wrong output rather than an error, so it wants the
  walk written deliberately [backend-never-wrong].
  * **What it is worth, now that (d) is in**: not soundness — (d) covers that —
    but defence in depth (a rule gap could not produce divergent output) and
    removing a clone per loop. An optimization, and priced as one.

Sequencing note: (d) landed as a patch because it is pure checker work at an
existing choke point. (e) touches the plan and both emitters, so it belongs with
R5's std rewrite rather than in front of R4.

**R4 — designate `Linear`.** `: Linear` with its `close`; `discard` refused for a
linear value, naming `close`; storing a linear value in a composite refused
(interim). `<T canbe linear>` keeps its spelling (user decision 2026-09-08):
permission on a parameter is not obligation on a declaration, and a `close`
alone implies neither.

#### R4 part 1 as built (2026-09-08): `Linear` designated

Done and green. `Linear` is a designated obligation group in
`std/core/basic.sv` — `params Linear<It> { fn close(it: It) -> None }` — and => !it
a type declares `: Linear<self>`.

- **The mandatory `close` came free** from [group-obligation]: declaring
  `: Linear<self>` without one is already an error at the struct, naming the
  signature. That is R1 paying for itself.
- **`has_auto_linear` reads the obligation** instead of `canbe linear`, so
  linearity is one fact from one place. `canbe linear` on a *declaration* is now
  an error naming `: Linear<self>`; on a **type parameter** it keeps its
  spelling [linear-generics], as decided.
- **`discard` no longer discharges** [linear-discard]: refused for a linear
  value, naming its `close`. Keyed on core's `intrinsic fn discard` rather than
  the bare name, since only std may write `intrinsic`.
- **The `close` implementation is exempt**, which the design memo had not
  noticed: `close`'s own parameter owes nothing, because that is where the
  value legitimately dies. Without it a `close` would have been the one thing
  linearity makes impossible to write — found by writing the first one.
- **The leak diagnostic now names `close`** instead of `discard`.
- **Tests**: `linear_group_tests.rs` (8), including that a `close` alone does
  *not* make a type linear (the decision that keeps a generated pass
  composable) and that `discard` still drops a plain value.

**The sweep found the interim composite rule's absence.** Three tests
discharged a **linear composite** (`Mut List<FileHandle>`) with `discard`, and
under the new rule that shape has *no* discharge at all — a `List<FileHandle>`
has no `close`. That is exactly what R4's remaining piece says outright
(storing a linear value in a composite is an error), so those cases were
removed rather than contorted, with a note at each site. Coverage restored by
part 2 (below).

#### R4 part 2 as built (2026-09-08): a composite may not hold a linear value

Done and green. [linear-composite] is now a **refusal at the store** instead of
contagion: a linear value lives only in a local, a parameter or a return value,
and every way of putting one into a container is an error where it is written.

- **The predicate split, and the deep one turned out to be unnecessary.** The
  plan said `ty_transitively_linear` survives as the detector; in fact it
  *dissolved*. `ty_own_linear` is the whole predicate now — the type declares
  `: Linear<self>`, or it is an opted-in `T`, or it is a **union with a linear
  arm** — because a struct with a linear field can no longer exist, so there is
  nothing for depth to find. That the union case stays is the one subtlety
  worth keeping: a union value *is* the linear value (`Lines | Int` is one
  handle), so narrowing and branch merges must not lose the obligation, while a
  *written* union arm is refused as a store.
- **One mistake, one diagnostic** — the payoff that made the change worth
  doing beyond the rule itself. Under contagion a refused store produced two
  errors: the store, and then a leak for the container that was never built
  (the l7a fixture asserted exactly that, "6 errors"). It is now 5.
- **Six refusal sites**, chosen so that each store is caught where the author
  wrote it: a struct or handler-state **field** (at the field, naming it); a
  **written composite** via `validate_type`, one level per node so
  `List<List<Lines>>` is a single error at the inner list; an **array or tuple
  literal** (nothing written to refuse); a **struct literal** field whose
  declared type is generic (`struct Box<T> { item: T }` — the store its own
  declaration cannot see, and skipped when the field type is concretely linear
  so the struct's error is not doubled); and a **call** that takes a bare `T`
  and puts a `T` in a composite.
- **The call-site rule is what took the thinking.** The first cut refused any
  call whose signature mentions `T` inside a composite — and broke the
  refinement demo, where `count<T canbe linear>(list: List<T>)` calls
  `size(list)`. Reading a container of `T` is not storing one. The rule that
  holds: the callee must take a **bare** `T` parameter *and* mention `T` inside
  a composite (`add(list: Mut List<T>, elem: T)`). `size`, `iter` and `get`
  read; `add` and a `-> List<T>` return store. Variadic `list(...elems: T[])`
  is already covered by the older variadic refusal.
- **Std needed no change**: an opted `<T canbe linear>` signature is not a
  store, so `add`'s declaration stays legal and the refusal lands on the *call*
  that instantiates it. That is why the declaration-site predicate is
  deliberately blind to type parameters while the call-site one is not.
- **Tests**: `linear_group_tests.rs` grew to 18 (ten new, one per site plus the
  positives: reading a composite of `T`, a container of plain values, and the
  nested-composite single-error case), and the two CLI fixtures were rewritten
  — `l6_linear`'s `ok_composite` positive became `composite_refused` (8 errors,
  the new one at `Box2.item`), `l7a`'s count dropped to 5.
- **Known gap, deliberate**: `let b = Box { item: h }` on a struct with *no*
  type arguments inferred still types as `Box<Unknown>`, so the store is caught
  by the field-value check rather than by any type-argument reasoning — there is
  no struct-literal type-argument inference to hook. Fine as it stands (the
  store is refused), but it is the reason the check looks at the value's type.

**R4 part 2, as planned (for the record)**: the interim composite refusal —
storing a linear value in a struct field, type argument, or array/tuple/union
component becomes an error at the store, rather than propagating the obligation
into the container. It was sequenced before R5 because std's `List` is where the
shape shows up; the one thing the plan got wrong is noted above (the transitive
predicate did not survive — it became unnecessary).

**R5 — demolish `Iter<T>`. ✅ Built 2026-09-09** (see "R5 part 2 as built").
Remove the type and everything that existed to
support it: the `SalvoIter`/`Iterable` representations, `once` on producers, the
effect claim with `producer_effects`/`pass_effect_sets`/the per-effect-set
traits/the variance adapter, and `params Iterable`. Rewrite std: a pass struct
plus `next` per intrinsic container, `iter` returning a fresh pass, `seq.sv`'s
combinators over passes (`map_lazy` becomes a composed origin struct with its
own `yield fn next`). The emitters keep their native `for` for a list, array
or `Str` subject.

#### R5 part 1 as built (2026-09-08): fn-typed fields on Rust [rs-fn-field]

Done and green — the one piece of R5 that is independent of the `Iter<T>` flip,
taken first because everything composed depends on it. A composed pass holds its
callback as a field (`f: (T) -> U`), which Kotlin has always accepted and Rust
refused; the refusal is gone.

- **The representation was already in the build**: a *generated* pass stores a
  callback as `Rc<dyn Fn…>` [rs-iter-lazy], so a user struct field renders the
  same way — `emit_type` under the iterator-fn convention (`impl Fn(..) +
  'static`) put through the existing `rc_fn_type` surgery. Not a second
  fn-type renderer.
- **`Fn`, not `FnMut`** — the first attempt rendered `Rc<dyn FnMut…>` (the
  default parameter convention) and rustc refused the call: `cannot borrow data
  in an `Rc` as mutable`. A shared `Rc` can only offer `Fn`, which is also what
  a stored callback should be [iter-mut-param].
- **Three edits, not one**: the field type, the *store* (`Rc::new(…)` in a
  struct literal and in an inlined default), and `Debug` — a `dyn Fn` has none,
  so a struct with a callback gets `#[derive(Clone)]` plus a hand-written
  `Debug` printing `<fn>`. The derive was the thing that would have failed at
  rustc rather than in the emitter.
- **Wrapping an already-`Rc` value is fine**: `Rc::new(rc)` re-coerces to
  `Rc<dyn Fn…>` at one more indirection, so the store's rendering does not have
  to know where the value came from.
- **Tests**: one program, one stdout, both backends
  (`rustc_compiles_and_runs_a_composed_pass_with_a_stored_callback` /
  `kotlinc_…`) — a hand-written `Doubling` pass storing a `Mut Countdown` source
  and a `(Int) -> Int` callback, driven by `for` — plus a Rust shape test for
  the field type, the `Debug` impl and the `Rc::new` store. The former codegen
  refusal test is gone; the `params`-group-in-a-field guard stays.
- **What it unblocks**: R5's composed combinators (rendering (B) stores the
  source's `next` as a function value), and R0 finding 5's "the difference
  between std may compose passes and anyone may".

#### R5 part 2 as built (2026-09-09): the flip

Done and green: 799 tests, `cargo build` warning-free, both toolchains running
the same programs to the same stdout. `iter` hands back a pass, std's
combinators take one, and the `Iter<T>` world is deleted. The plan below was
followed in its six steps; what it did not predict is recorded here.

**std as built.** `ListYield<T>`/`ArrayYield<T>`/`StrYield` are ordinary structs
(`items`/`text` plus `at`) declaring `: Yield<self, T> canbe Mut`, each with a
Salvo `next` — the `is None` guard shape [is-narrow-guard] is what made them
writable in Salvo at all. `iter` returns `Mut <C>Pass<T>` with `=> !…`
(constructing the pass *moves* the container in, probe finding 4), and
`iterable.sv` is gone. `seq.sv`'s `map`/`filter`/`reduce`/`map_to`/`filter_to`
drive `it: Mut It` through a `?Yield<It, T>` spread — with `while` + `next` +
`when` as first built, and with an ordinary `for` since the same day
[iter-generic-drive]; `map_lazy`/`filter_lazy` return **composed passes** (`MapYield`,
`FilterYield`) holding the source, the callback and the source's `next`.

**What the flip forced, in the order it turned up:**

- **`for` over a container is mint-then-drive, except for the intrinsic ones.**
  The checker records the `iter` to call beside the `next` to drive
  (`PassDriver::mint_iter_fn`), and records *nothing* for a list, array or
  `Str` — those keep the native loop [iter-for-native], which is also why a
  `for` over a list still does not consume it. The predicate is "the subject is
  an `intrinsic type`", not a name list.
- **Rendering (B) needed three things on Rust.** A composed pass stores its
  callback, so: a fn-typed parameter arrives **owned** when the callee keeps it
  past the call; `copy(f)` on a function value lowers to the value itself (an
  owned `impl Fn` has no `clone`); and the hand-written `Debug` for a struct
  with a fn field bounds its generics by `Debug`, as a derive would.
  * The *trigger* for "owned" took two tries. The deduction list looked right —
    a moved fn-typed parameter is one the body keeps — but a callback is
    commonly moved without being stored (`apply(f, data)` calls it and drops
    it), and that convention change broke `apply_keeping`. What holds: a
    fn-typed parameter **and** a return type that is a struct with a fn-typed
    field. That is the composed-pass shape, and nothing else in the language
    stores a callback yet.
- **A `move` closure has to clone its captures.** Two lazy combinators reading
  one local is legal Salvo (an immutable read) and E0382 in Rust, so a stored
  lambda renders as `{ let mut x = x.clone(); move |…| … }` — shadowing, so the
  body needs no rewriting.
- **A minted origin is released by whoever drives it.** The mint at an argument
  position used to close after the call; a *lazy* combinator stores the machine
  and drives it later, so closing there handed the pass a finished machine and
  the loop saw nothing. The rule both backends now use: close after the call
  only when the callee **keeps** the pass (Rust: a kept-`Mut` position, so
  `&mut __mintN`; Kotlin: the callee's deduction list). A moved mint is the
  callee's, and is not closed here.
- **The synthesized code needs imports the source does not.** A driving loop
  spells `Finished`; Kotlin adds the protocol module when a file drives *or
  mints* a pass, and Rust adds the glob unconditionally (harmless under the
  crate's `unused_imports` allowance).
- **Both backends needed the adapter's parameter type written.** Kotlin cannot
  infer a callee's element type from two `U2_n` branches, so the origin-mint
  adapter is an anonymous *function* with a declared return type; rustc cannot
  infer the closure parameter through the `&mut dyn FnMut` coercion, so it is
  annotated with the machine type. The turbofish substitutes the machine type
  for the origin type, since that is what the value is.
- **One new inference rule, and it earns its keep**: a `?Yield<It, T>` spread
  teaches `T` from `It`'s own `: Yield<self, T>` clause, reaching through the
  machine an origin mints. Without it a *bare* lambda over an origin subject
  could not be typed (there is no declared `next` to read `T` off) and the
  turbofish above was missing. `yield_spread_pairs` + `pass_or_origin_elem_ty`.
- **[iter-mut-param] survived by changing its trigger** from "the callee is a
  producer" (there are no producer factories any more) to "the callee moves a
  fn-typed parameter" — which is exactly "keeps it past the call".

**Known cuts and gaps, all with a diagnostic rather than wrong output:**

- **A *suspending* loop may not iterate a pass**: inside a `yield fn`, a `for`
  takes an array, a `List<T>` or a `Str`. The nested-pass slot machinery was
  built for `Iter<T>`, and driving a pass from a slot is unbuilt. This is the
  one capability the flip *lost* (a nested producer used to work), and the
  workaround is to collect first (`map(upto(3), f)` into a list) — the
  generator-defer demo does exactly that.
  * **Retired 2026-09-10, not built**: `yield fn` and the slot machinery are
    gone, so nothing suspends and the cut has no construct to apply to. A `for`
    inside an `iter fn`'s `next` is an ordinary loop; the capability this
    describes — lazily interleaving with an inner pass — is written by holding
    that pass in `state`, which is what turned up [rs-narrow-mut].
- **A machine holding a nested pass slot is not `Clone` on Rust**, so such an
  origin cannot be a type argument: a producer that drives a suspending loop
  cannot be composed there. rustc reports the missing bound.
  * Retired with the entry above — there are no machines and no slots.
- **A callback handed *onward*** to another storing fn stays borrowed on Rust
  (the predicate is deliberately non-transitive); rustc reports the lifetime.
- **Value-position `for` over an origin** is still unsupported on both backends
  (the machine has nowhere to be closed).
- **Kotlin's generic union arm reads warn** (`unchecked cast of 'Any?' to 'T'`)
  in std's `seq.kt`: a `when` arm over a generic union must use star
  projections, so the element read casts. Cosmetic, in code the user cannot
  edit — a `@Suppress("UNCHECKED_CAST")` on the emitted function is the fix,
  and it is the first thing to do in a polish pass.
- **A user type named like one of std's passes collides**: implicit resolution
  matches by name, so a program declaring its own `ListYield` plus `next` makes
  the `next` ambiguous. Nominal types are not module-qualified in that match;
  worth fixing when someone hits it (the demos were renamed instead).

#### The generic drive as built (2026-09-09): `for` over a `?Yield` spread

Two user decisions, one afternoon, and a defect found on the way.

**What was decided.** A `for` may drive a subject whose type is a *type
parameter* when the enclosing fn declares a protocol-shaped `?Yield<It, T>`
spread for it: the position says "this call supplies a `next` for `It`", which is
as much of a declaration as a struct's `: Yield<self, T>` clause. And when the fn
**owns** a pass whose parameter says `canbe linear`, it must declare the release
it will need (`?Linear<It>`, or a `?close` by hand), which the loop then calls on
every exit.

**As built.** `PassDriver` now names a `PassMember` — `Fn(FnKey)` or
`Implicit(String)` — for both `next` and `close`, so the emitters call an implicit
parameter by its own name (which shadows the fns of that name in the body anyway
[implicit-param]). The gate is deliberately two-part: the implicit must be
**named `next`** (shape alone would let any advancing function drive, and `for`
drives `next` everywhere else) and must take its state as **`Mut It`** (the same
diagnostic a declared `next` gets). Without a spread the subject reports the
ordinary not-iterable error — a bare type parameter says nothing.

**The parity defect the work turned up.** `fn take_two(p: Mut ListYield<Int>) ->
[p: Mut] Int` driving `p` with `for`, called twice, printed `first 3 rest 3` on
Rust and `first 3 rest 7` on Kotlin: Rust bound the subject into a local, which
for a kept parameter is a *clone* of the `&mut`, so the caller's pass never
advanced — while Kotlin aliased it and the caller saw the new position. It was
pre-existing (concrete passes had it too) and would have been inherited by every
std combinator. Fixed as [iter-drive-in-place]: when the subject is a place the
fn keeps, the loop advances it where it lives — no local on either backend — the
checker treats the drive as a *mutation* rather than a move, and no `close` is
resolved (the release stayed the caller's). A second, quieter one came with it: a
pass the fn *owns* is now **moved** into the loop's local rather than cloned,
since a clone left the original unreleased.

**One rule had to change for std to compile at all.** A pass drive gives the loop
binding **no fate links**: `next` hands the element over by value, so the binding
is an owned value and not a projection of the pass. Without that, `filter`'s
`add(out, x)` was refused ("cannot move `x`: it was bound from `it` and shares
its fate"). A native container loop still links — there the binding really is a
projection.

**The payoff.** `std/core/seq.sv` lost 44 lines and reads as ordinary Salvo:

```
fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) [] -> Mut List<U> => it: Mut, f {
    let out = mutable_list<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}
```

The lazy `next`s (`MapYield`, `FilterYield`) stay hand-written: they produce one
element per call, so they have no loop to write.

#### The generic-effect refusal, lifted (2026-09-09)

A `yield fn` declaring `[Random<Int>]` was refused with "the generated pass
interface is named per effect set, and the arguments would have to be part of
that name" — a reason that stopped existing when R5 deleted the per-effect-set
traits. The machine's handlers are now rendered exactly as any fn's: the type
through `rust_ty`/`kotlin_ty`, the parameter name from
`unique_name(effect_param_name(&rendered))`. `claim_names` is gone from both
emitters.

Lifting it exposed a second bug immediately: the drive-site effect check recorded
`call_effects` at the *subject's* span, so the emitters threaded the handlers into
the subject **call** — `rolls(random_int, 3)`, a builder that declares none.
`check_fn_value_effects` (records) and `check_drive_effects` (checks only) now
split over one `check_effects_available(effects, span, record)`; a drive site
derives its own handler list from the `yield fn`'s effects, so it never needed the
record.

##### R5 part 2 probe (2026-09-08): what the flip needs, from a real program

Two throwaway programs (a `ListYield<T>` container pass plus a generic
combinator; and a narrowing probe) were run through `analyze` before touching
std, and they turn the sequencing question into five facts:

1. **A combinator cannot take `xs: It` plus `?iter: (It) -> P` today** — which
   is what the surface decision below routes around, so this stays unbuilt. The
   implicit resolver fills each implicit against the *declared* signature, so
   `P` is still unbound when it reaches `?Yield<P, Int>`: `no next fits ?next:
   (Mut ?) -> Emitted Int | Finished`, while the `next` in scope is
   `(Mut ListYield<Int>) -> …`. Filling `?iter` first and **feeding the chosen
   overload's return type back into the substitution** before resolving the
   next implicit is the missing step — a targeted change in `resolve_implicits`,
   and the enabling work R5 part 2 rests on. (Where a type parameter is bound by
   an ordinary *argument*, the spread already works — `group_tests`'
   `the_same_group_serves_an_obligation_and_a_spread` is that case.)
   * The alternative surface, if that inference is not wanted: a combinator
     takes the **pass** (`fn total<P>(p: Mut P, ?Yield<P, Int>)`) and the call
     site writes `total(iter(xs))`. Cheap, works today — and a different
     language surface, so it is the user's call, not an implementation detail.
2. **Driving through an implicit needs no `for` machinery.** An explicit
   `while` + `next(p)` + `when` over the two arms resolves through the implicit
   already. `for` over a type-parameter subject (nicer, and it injects the
   `close`) remains optional: it needs `pass_elem_ty` to accept a `Ty::Var`
   whose `next` is a fn-typed *parameter*, and a `PassDriver` that can name one.
3. **A generic struct literal needs its type arguments written**
   (`Mut ListYield<T> { … }`) — R0 finding 5, met again immediately.
4. **A pass that stores its source consumes it**: `fn iter<T>(items: List<T>)
   -> Mut ListYield<T> => items` is refused ("deduction promises `items` back to
   the caller, but the body moves it"), so std's `iter` will be `=> !…`. Worth
   knowing before the rewrite: iterating a list *consumes* the list unless the
   pass borrows, which is drive-in-place (R5's own open question) territory.
5. **A container `next` cannot be written in Salvo yet** — narrowing did not
   survive an early-returning guard, so `let e = get(p.items, p.at)`, `if e is
   None { return finished() }`, then `emitted(e)` failed with `Emitted (T?)`.
   **Closed 2026-09-09** as [is-narrow-guard] (see the defect entry): the guard
   shape works, so std's container passes can be ordinary Salvo code.

##### The combinator surface — DECIDED (user, 2026-09-09), and validated

`?Iterable` goes away entirely and a combinator's subject **is** the pass:

```
fn map<It, T, U>(it: Mut It, mapper: (T) -> U, ?Yield<It, T>) -> Mut List<U> => it: Mut, mapper
```

A container is iterated by writing the `iter` call — `map(iter(xs), double)` —
and a custom pass type arrives the same way, from whatever call produces it. Two
things this settles at once:

- **No cross-implicit inference is needed** (probe finding 1): `It` is bound by
  an ordinary argument, so the `?Yield<It, T>` spread resolves with today's
  machinery. The gap found in the probe stays a gap, and nothing depends on it.
- **`params Iterable` and every `iter`-returning-`Iter<T>` overload disappear**
  rather than being rewritten, which is a large part of R5's demolition list
  turning into deletion.
- The cost, accepted: `map(xs, f)` over a `List` becomes `map(iter(xs), f)` —
  one call more at every use site, and the eager `List` fast paths
  ([seq-iterable]'s `intrinsic` overloads) are the ones that keep the short
  spelling for the common case.
- Only the parameter mode had to be pinned down beyond the sketch: the pass is
  `Mut It` with `[it: Mut]`, since advancing it is a mutation.

**Validated end to end on both backends before touching std** (2026-09-09): a
`ListYield<T>` with a hand-written `next`, a `map2` over the `?Yield` spread
driving with `while` + `next` + `when`, `map2(iter2(xs), double)` printing
`d 2 / d 4 / d 6` under rustc and kotlinc. Doing that first was worth it — the
surface works, but **nothing had ever exercised a spread member that mutates
its subject or returns a union**, and each backend was broken in a different
way:

- **Checker: a group member's fn type was built without its contract**
  (`member_fn_ty` passed `contract: None`), so every parameter of an implicit
  position read as kept-and-immutable and a real `next` could not fill it —
  *"`next` mutates `p`, which this position does not permit"*. Now built from
  the member's own deduction list, exactly as a declared fn's is. This was the
  blocker for the whole composition rendering, and it is one field.
- **Rust: the adapter closure borrowed a `&mut` parameter twice.** A kept-`Mut`
  implicit position renders `&mut T`, so `&mut |__i0| next(&mut __i0)` is
  `E0596`; the adapter now passes such a parameter through (and clones for an
  owned callee). [rs-implicit-adapter half of [implicit-param]]
- **Kotlin: a union-returning member printed its Salvo type text** into the
  Kotlin source (`next: (It) -> Emitted T | Finished`) — the `Ty`-to-Kotlin
  renderer used for implicit positions had no union case and fell through to
  `Display`. Invalid output rather than an error, so a
  [backend-never-wrong] defect in its own right.
- **Tests**: `group_tests`' `a_mutating_member_fills_a_mut_spread_position`
  (checker), plus one shared demo per backend
  (`{rustc,kotlinc}_compiles_and_runs_a_combinator_over_a_yield_spread`) with a
  shape assertion each — the adapter's pass-through on Rust, the wrapper type
  on Kotlin.

##### Open defect (found 2026-09-08): narrowing does not survive a guard

```
fn head(xs: List<Int>) -> Int => xs {
    let e = get(xs, 0)
    if e is None {
        return 0
    }
    let n: Int = e      // ERROR: expected `Int`, found `Int?`
    return n
}
```
The then-branch **diverges**, so everything after the `if` is the else-path and
`e` is `Int` there — [is-narrowing] says "remaining arms in the else-branch",
and this is that branch, reached by falling through instead of by an `else`.
The equivalent `when e { is None { return 0 } is Int { return e } }` is
accepted, so the fact is available; what is missing is carrying the negative
fact past a statement whose branch cannot fall through. Not generic-specific
(`Int?` behaves the same as `T?`). This is the early-return guard shape, which
is the ordinary way to write a `next`, so it will be hit constantly.

##### Driving an *origin* from a combinator — the prerequisite (analysed 2026-09-09)

Raised by the user: a combinator has to accept a type that satisfies
`: Yield<T>` through the **sugar**, because the author of
`yield fn next(c: Counter) -> Int` has nothing `Mut` to hand over — the machine
is hidden and unnameable [yield-fn-origin]. Confirmed unrepresentable today:
`map2(counter(2), double)` fails at selection (`no matching overload for
map2(Counter, (Int) -> Int)`), since `Counter` is immutable and no
`next(Mut Counter)` exists.

The user's observation that **selection is unambiguous** holds — a type
satisfies its `Yield` obligation by a raw `next` *or* by the sugar, never both
[yield-fn-origin] — so the compiler can always tell which mechanism a type
needs. What is missing is not the decision but the *representation*.

**Every route needs the same thing: the hidden machine has to become a real
type in the checker, inferred and unwritable.** Four were worked through and
they all reduce to it:

- **Mint at the argument** (`map(counter, f)`, the compiler constructs the
  machine at the pass position): the type argument `It` binds to the *machine*,
  so the machine needs a `Ty`, and the implicit `next` needs something to
  resolve to.
- **Mint by a written call** (`map(iter(counter), f)`): the mint function has to
  have a return *type*. Same requirement, reached from the other side.
- **A second group for origins** (`params Origin<O, T>` + a `for` over a
  type-parameter subject in the combinator body): the body would have to mint
  the machine, whose type a generic body cannot know — on Kotlin generics are
  erased, so the minting has to arrive as a *value*, which means either the
  machine type as an inferred type argument or two implicits
  (`(O) -> M`, `(Mut M) -> …`) and therefore the cross-implicit inference gap
  (probe finding 1). No cheaper.
- **Desugar the sugar into the raw form at the source level** — the user's own
  premise ("the yield form lowers to the raw form"). This *is* the prerequisite,
  stated positively: for each `yield fn next(o: O) -> T`, the compiler declares
  a pass type `__Pass_O : Yield<self, T> canbe Mut`, a
  `fn next(p: Mut __Pass_O) -> Emitted T | Finished`, and a mint => p: Mut
  `fn iter(o: O) -> Mut __Pass_O`. The emitters already generate the machine => !o
  and know how to advance and close it, so the two functions are lowerings, not
  new code. Everything else then follows from rules that already exist:
  overload selection, implicit resolution, `for` over the machine, and a
  composed pass holding one in a field ([rs-fn-field] made that possible).

With that in place the two call surfaces are one small step apart, which is why
the prerequisite is the thing to build first:

- **(B) written mint** — `map(iter(counter), f)`: free once the declarations
  exist, and uniform with a container (`map(iter(xs), f)`).
- **(A) implicit mint** — `map(counter, f)`: (B) plus an argument-level sugar —
  "an origin in a position that wants a pass of the same element type inserts
  the mint" — which is the same thing `for` already does per loop, so it is a
  coercion, not a new semantics.

**Open for the user**: whether (A) is wanted as the surface (recommended: yes,
with (B) always available — the sugar's point is that the machine is invisible,
and an origin *is* declared `: Yield<T>`, so a pass position should take it),
and whether `iter` is the mint's name for both origins and containers.

##### DECIDED (user, 2026-09-09): the mint is implicit, at the call site

The rule: **anything whose declaration says `: Yield<self, T>` can be passed as
is.** If it is a pass, it is taken as it stands; if it is an origin, the argument
is rewritten to a fresh instance of its pass. And the half that makes it
tractable — *generic* signatures always write the **pass**:

```
fn map<It, T, U>(it: Mut It, mapper: (T) -> U, ?Yield<It, T>) -> Mut List<U> => it: Mut, mapper
```

"Generic functions don't need to be desugared, since that happens before they
are called" (the user). So `Mut It` is not a question the body has to answer: by
the time the body runs, `It` *is* a pass, and a pass is always mutable. A
non-generic signature needs nothing new either — it names a concrete type, so a
parameter of an origin type is driven by `for`, which R3 already lowers to
minting a machine per loop [yield-fn-origin]. The new rewrite therefore applies
in exactly one place: **an origin argument in a generic pass position**.

One wrinkle found while planning it, worth stating because it decides *where*
the rewrite goes: the pattern `Mut It` substitutes to `Mut <machine>`, and a
`Mut` position requires the argument to carry `Mut` — so the rewritten argument
type has to be `Mut <machine>`, and the rewrite must happen **per candidate,
before unification**, not as a coercion afterwards. That is also what keeps
selection honest: a candidate whose parameter is the concrete origin type still
matches the origin, and only a `?Yield`-bound generic slot mints.

##### As built (2026-09-09): the mint, both backends

Done and green. `map2(counter(2), double)` and
`map2(Mut Zip { … }, double)` run in one program with one stdout on both
backends — an origin minted at the argument, a raw pass taken as it stands.

- **Checker, three small pieces.** `origin_pass_ty` turns an origin into
  `Mut __Pass_<Origin>` (a name no declaration can spell, so the machine stays
  unnameable while being an ordinary inferred type argument);
  `yield_spread_vars` says which parameters want a pass; the candidate loop
  rewrites those argument *types* before unifying, and the winner's rewrites are
  recorded in `Checked::origin_mints`, keyed by the argument's span.
- **Where it had to go, and why.** Not a coercion after selection: the pattern
  `Mut It` substitutes to `Mut <machine>` and a `Mut` position requires the
  argument to *carry* `Mut`, which an origin does not — so selection itself has
  to see the machine type. Per candidate, so a signature naming the concrete
  origin type still matches the origin.
- **The implicit has no fn to resolve to**, since the machine is generated: a
  new `ImplicitArg::OriginNext { next_fn }` carries the `yield fn`'s key, and
  each emitter wraps the machine's advance into the protocol's two arms —
  Rust `match __p.__advance(h) { Some(v) => U1(v), None => U2(Finished{}) }`,
  Kotlin `if (__p.__advance(h)) U2_1(__p.__current()) else U2_2(finished())`.
  Handlers come from the call site's effect environment, which is where the
  adapter is built, so an effectful origin threads them exactly as a `for` does.
- **One thing rustc found**: the machine is now a *type argument*, and every
  generic parameter in the Rust backend carries `Clone + 'static`, so the
  generated origin machine needed `#[derive(Clone)]`. Its fields are Salvo
  values and `Rc`-held callbacks, both `Clone`.
- **Tests**: `yield_origin_tests` gains the checker pair (an origin fits a
  generic pass position; a plain struct still does not), and each backend gets
  the shared demo plus a shape assertion — the mint at the argument, the advance
  adapter, `Clone` on the machine, and `__Pass_Zip` *not* appearing, which is
  what proves a raw pass is not minted.
- **The `close` gap is closed** (same day, user request): the mint is hoisted
  into a local and released *after the call*, so a combinator that abandons the
  pass early cannot leak it. This is the answer R0's `?close` implicit was
  sketched for, and it needs no optional group member at all — the machine is
  the compiler's value, so the compiler owns its lifetime. The release
  threads the same handler list the advance adapter does (one helper per
  backend, so driving and releasing cannot disagree), and `__close` is
  idempotent, so a drained pass pays nothing.
  * Verified by an origin whose body has a `defer` printing `close`, driven by
    a combinator that returns after **one** element: `open / close / got 1` on
    both backends. Rust splices the release after the call inside a block
    expression (`{ let mut __mint1 = …; let __call = …; __mint1.__close(); __call }`),
    Kotlin uses `run { … }`.
  * Nested calls keep their own mints (the pending list is swapped around
    argument rendering), so a mint inside an argument of another minted call
    releases at the right boundary.
- **Two mints in one call are independent**, including two of the *same origin
  value*: `sum_two(c, c)` mints two machines, both replay from the beginning,
  both are released — `two 32 / same 22` byte-identical on both backends. Four
  tests (two shape, two e2e) cover it.
- **Still not covered**: a *raw* pass abandoned by a combinator — the caller's
  value, moved into the call, nobody to close it. Pre-existing
  unchecked-driver hole, not a new one; a *minted* pass can no longer be
  abandoned. Options and the probe that constrains them below.

##### DECIDED and built (user, 2026-09-09): option (a) — the driver releases

`: Linear<self>` beside `: Yield<self, T>` declares "this pass owns something",
[group-obligation] then requires the `close`, and the **`for` sugar calls it** on
every exit. Built on both backends, one program, one stdout: a drained loop
prints `n 2 / n 1 / closed / after drain`, one that `break`s prints
`m 5 / closed / after break`.

- **`PassDriver.close_fn`**: the checker resolves the `close` beside the `next`
  (`pass_close_fn`, a single parameter unifying with the subject), so both
  halves of the protocol come from one table and cannot disagree.
- **Emission reuses the shape that already existed** for a producer and for an
  origin: Rust splices the call after the loop *and* registers it as a deferred
  entry, so a `return` out of the body reaches it; Kotlin wraps in
  `try`/`finally`. One Kotlin wrinkle: the pass local has to be declared
  *outside* the `try` or the `finally` cannot see it, so the header's first line
  is hoisted out.
- **Nothing is implied.** A `close` alone still means nothing [linear-group];
  the release is driven by the *clause*, and a pass owning nothing is untouched
  — which is what keeps generated machines composable.
- **A combinator keeps its pass**, so the obligation stays with the caller and
  the existing [linear-obligation] rule does the rest; a generic one admits it
  may carry a resource with `<It canbe linear>`. Verified: the opt-in is
  required, the leak is reported without a `close`, and the whole thing is
  clean with it.
- **Diagnostic fixed on the way**: the return-while-owing message still said
  "or `discard(x)` first", which R4 made wrong for a linear value — it now names
  the value's own `close`, like the scope-exit message already did.
- **Tests**: one shared demo per backend (drained *and* abandoned in one
  program) plus a shape assertion each — the release after the loop on Rust, the
  `finally` on Kotlin.
- **Unchanged casualty**: a linear pass still cannot be *composed* (R4's interim
  composite refusal), which is L8's to answer.

##### The raw-pass release: the options as presented (probe 2026-09-09)

Asked by the user: could linearity handle it? **Probed, and the answer is not on
its own.** This is accepted today and prints `n 2 / n 1 / after` — no `closed`:

```
struct Lines : Yield<self, Int>, Linear<self> canbe Mut { at: Int }

fn next(l: Mut Lines) -> Emitted Int | Finished => l: Mut { … }
fn close(l: Lines) [Console] -> None { println("closed") }

for n in lines { println("n ${n}") }        // drives, consumes, never closes
```

Two obligations on one struct check out fine, and the linear obligation *is*
discharged — by the **move into the loop** [iter-resolve]. So linearity is
bookkeeping about ownership, and what is missing is that driving does not run
the release. R0 finding 5 predicted exactly this: the `for` sugar needs **two**
resolved functions per raw subject where `for_drivers` carries one.

The options, with what each costs:

- **(a) Linearity as the declaration, plus `for` calling `close`** — the pass
  type says `: Linear<self>` when it owns something, and the `for` lowering
  gains the second resolved function so driving a raw pass releases it, exactly
  as driving a minted one now does. A combinator *keeps* its pass parameter
  (`[it: Mut]`), so the obligation stays with the caller, who must close it —
  already enforced, no new rule. Cost: `for_drivers` carries a second `FnKey`,
  and both emitters splice the call (Rust after the loop + as a deferred entry,
  Kotlin in the `finally`) — the same shape they already emit for a mint and
  for a producer. **Recommended.**
- **(b) Auto-release at the call boundary, like a mint.** Rejected on
  intuition: a minted pass has no name and can never be referenced again, which
  is why the compiler may close it; a *raw* pass is the author's named,
  resumable value, and closing it behind their back at an arbitrary call would
  be surprising — they may drive it further afterwards.
- **(c) A `close` implies an obligation** (any pass with a `close` must be
  released). Already rejected as a rule (2026-09-08): attaching an obligation
  on the strength of a function name is what [qual-*] keeps the compiler from
  doing, and it would make most generated passes uncomposable.
- **(d) `?close` as an optional implicit on every combinator** (R0's sketch).
  Needs optional group members — new surface — and puts the burden on each
  combinator rather than on the type that owns the resource. The mint's fix
  already showed this machinery is avoidable where the compiler owns the value;
  for a raw pass, (a) puts it where the *author* owns it.

Under (a) the remaining hole is a hand-written `while` driver over a raw linear
pass: the checker sees the moves, so it will report the obligation if the value
is never passed to `close`, which is the honest answer — the driver author is
the owner, and linearity is what tells them.

##### Implementation plan as written before building (machine APIs verified 2026-09-09)

The emitted machine already has everything needed — Rust
`__Pass_O::new(origin)`, `.__advance(handlers) -> Option<T>`,
`.__close(handlers)`; Kotlin `__Pass_O(origin)`, `.__advance(handlers) -> Boolean`,
`.__current()`, `.__close(handlers)` — so this is plumbing, not new codegen.

1. **Checker, candidate loop** (`check_call`, where `patterns` unify against
   `arg_tys`): per candidate, for each fixed slot whose pattern is `Mut <var>`
   with `<var>` a callee generic that the callee spreads as `?Yield<var, _>`,
   and whose argument type is an origin (`yield_obligation` + `yield_fn_for`),
   substitute the argument type with `Mut __Pass_<Origin>` and remember the
   slot. Unify and rank with the substituted types.
2. **Checker, after selection**: record the winner's mints —
   `origin_mints: HashMap<Key /* arg span */, Ty /* origin type */>` in
   `Checked`.
3. **Checker, implicit fill**: a `want` of
   `(Mut __Pass_X) -> Emitted T | Finished` resolves to no declared fn, so
   `fill_implicits` recognizes the machine type and pushes a new
   `ImplicitArg::OriginNext { origin }` instead of erroring.
4. **Both emitters, the argument**: where `origin_mints` has the span, emit the
   construction rather than the ordinary rendering — Rust
   `&mut __Pass_X::new(<origin, cloned if a place>)`, Kotlin `__Pass_X(<origin>)`.
   The machine item is already emitted for every `yield fn`.
5. **Both emitters, the implicit**: `OriginNext` becomes the adapter that wraps
   `__advance` into the protocol's union — Rust
   `&mut |__i0| match __i0.__advance(h) { Some(v) => Union2::U1(v), None => Union2::U2(Finished{}) }`,
   Kotlin `{ p -> if (p.__advance(h)) U2_1(p.__current()) else U2_2(finished()) }`.
   Handlers come from the call site's effect environment, which is where the
   adapter is built, so an effectful origin threads them the same way a `for`
   over one does.
6. **Tests**: the checker rule (origin accepted in a pass position, a raw pass
   still taken as is, a non-`Yield` argument still refused) plus one shared demo
   per backend — `map`-shaped combinator over an origin *and* over a raw pass in
   one program, one stdout.
7. **Not covered, deliberately**: `close` on early abandonment (see the
   consequence below) and a *second* mint of the same origin inside one call
   (each mint is fresh, so replay works, but nothing tests it yet).

**Known consequence to keep in view**: a combinator that abandons its source
early cannot close it — the mint's machine has a `close` when the body defers,
and nothing in a `while`-driven combinator calls it. `map`/`filter`/`reduce`
drain, and a drained body discharges its own defers (R0 finding 3), so the
shipping set is safe; an early-stopping combinator (`take`, `first`) needs the
`?close` implicit that R0 sketched and nothing has built.

**R6 — sweep.** ~48 test fns and the `Iter`/`yield` snapshots; rewrite
`[fn-iterator]`, `[iter-protocol]`, `[iter-generator]`, `[rs-iter-lazy]`,
`[kt-generator]`, `[seq-iterable]`, `[seq-lazy]`, `[seq-into]`,
`[linear-canbe]`, `[linear-discard]`, `[linear-composite]`,
`[linear-generics]`; fold the superseded iterator sections of this file into
history; update LANGUAGE.md and README.

### Unknowns to settle before writing much

Each of these is cheaper to answer with a probe or a prototype than to discover
mid-phase — R0 exists to answer the first two.

1. **What does a `yield` fn look like? — SHAPE DECIDED (user, 2026-09-08): the
   origin-struct model.** The fn never names a state struct at all; see
   "Unknown 1: the origin-struct model" below for the worked example, what it
   dissolves, and the three sub-questions still open (consumption of the
   origin, the `next` naming requirement, composition's fn-field dependency).
   Settle those before R3.
2. **Composition — DECIDED (user, 2026-09-08): rendering (B), the group's
   member as an implicit parameter.** R0 prototyped both and they are not
   equivalent (see "R0 as built", findings 1–2): a bound cannot carry the
   source's effects, because an effectful `next` takes its handlers as
   parameters and so cannot implement a fixed trait method — the "one generated
   trait per effect set" problem [iter-effects] had, resurfacing at the bound —
   while a stored fn's *type* carries them and [fn-effects] inheritance already
   applies. A bound also cannot express an *optional* member, so one combinator
   could not serve sources with and without a `close`.
   * The rendering is **not new machinery**: the existing generated `map_lazy`
     already stores its `?Iterable` member as a function value on both backends
     (`iter: Rc<dyn Fn(It) -> SalvoIter<T>>` / a Kotlin function type).
   * **The state-as-parameter shape is the only shape** (user decision
     2026-09-08, revising the `Self` form built earlier the same day): `self`
     names the declaring type at the *obligation*, so the group stays ordinary
     and the very same declaration also spreads as implicits — which is what
     this rendering needs, and what a `Self` inside the group would have ruled
     out.
   * **The Rust limitation this rides on, recorded for future review.** A
     *generated* pass struct may store a function; a **user-written** struct
     may not, on the Rust backend only. The refusal is real today:
     ```
     struct Holder<T, U> canbe Mut {
         src: List<T>,
         f: (T) -> U,          // <-- refused
         at: Int
     }
     ```
     ```
     main.sv: the rust backend cannot store a function in a struct field
     (`Holder.f`): pass it as a parameter instead — an implicit parameter
     (`?f: ...`) or a `params` group is how a bundle of functions travels
     ```
     **Kotlin accepts the same program**, so this was a live backend divergence
     rather than a language rule — a program that built on one target and not
     the other. **✅ Lifted 2026-09-08 as R5's first piece** [rs-fn-field]: the
     field is an `Rc<dyn Fn…>`, exactly the representation the generated pass
     already used, so "std may compose passes" became "anyone may" — verified
     by a hand-written composed pass (a struct storing its source *and* its
     callback) printing identically under rustc and kotlinc.

3. **`for x in list`.** Intrinsic containers reach iteration through an `iter`
   returning a pass, or through an intrinsic `next` on the container itself.
   Either way the emitters keep the native loop, so this is a std-surface choice.
4. **Effects on group members.** Recommended: the group declares none, each
   implementation declares its own, and a generic driver *inherits* per call
   site from the implicit it was filled with — the existing "a fn inherits its
   fn-typed parameters' effects" rule applied to implicits. **Known gap**:
   implicits are not in `f.params`, so `inherited_fn_effects` does not walk them
   today (the same gap I5 hit when a producer's implicits were not pass fields).
   The alternative — an effect parameter on the group (`params Yield<T> [E]`) —
   is bounded row polymorphism and is being held in reserve.

### Unknown 1: the origin-struct model (user direction, 2026-09-08)

Three spellings were worked out on `Countdown` and presented; the user replied
with a fourth that supersedes them, and it is better than all three: **the
`yield` fn never mentions the state struct at all.** The struct the author
declares is the *origin* — the starting data — and the machine the compiler
builds from the body is **hidden**: not nameable, not constructible, not
readable. Whoever wants the state struct writes the raw `next` instead. In the
user's words: `yield` indicates that this is doing more work; the return type
is the yield type.

```
struct Counter : Yield<Int> {
    start: Int
}

// The second way to satisfy `: Yield<Int>`. The subject is the origin; the
// return type is the ELEMENT type — no `Emitted`, no `Finished`, no machine.
yield fn next(c: Counter) [] -> Int {
    let num = copy(c.start)
    while num >= 0 {
        yield num
        num = num - 1
    }
}
```

So a `: Yield<T>` clause is dischargeable **two ways**, distinguished by the
`yield` keyword on the fn:

- **raw**: `fn next(c: Mut Counter) -> Emitted Int | Finished` — the => c: Mut
  subject *is* the state, `for` drives it in place, a second drive continues.
  This is the form for `zip`, `merge`, holding an iteration in a variable,
  passing one to a function.
- **`yield`**: `yield fn next(c: Counter) -> Int` — the subject is consumed
  into a hidden machine (origin fields + hoisted locals + `__state` + `defer`
  flags), which only the `for` sugar ever holds. `generator.rs`'s plan is
  unchanged; what moved is that its struct stopped having a name.

**What this dissolves — the reason it wins.** The presented options all made
the generated pass a *nameable* type with compiler-owned fields, which dragged
in four rules (literal construction refused, field reads refused, name
collisions, `copy` of a machine) and left "may the author add fields?"
hanging. All four evaporate: `Counter` is fully the author's, the machine is
fully the compiler's, nothing is half-owned. Two more things fall out:

- **Abandonment outside `for` becomes impossible for generated passes.** No
  expression can hold the hidden machine, so the only driver is the sugar, and
  the sugar always closes. The accepted "hand-written `while` abandons a
  `close`-bearing pass unchecked" cost now applies only to raw passes, whose
  author wrote the state and can manage it.
- **The effect list lands where the group rule already wanted it.**
  `yield fn next(c: Counter) [Console] -> Int` is a member declaring its own
  effects, and "driving performs them" stops being a relocation rule — nothing
  ever calls this fn directly, so there is no call site to wrongly demand a
  handler at. [iter-effects]'s claim-on-the-type apparatus stays deleted.
- **`yield`-ness is declared, not inferred**: the keyword on the fn replaces
  [fn-iterator]'s "returns `Iter<T>` and uses `yield`" test, and a `yield`
  statement in a non-`yield` fn becomes a plain error naming the keyword.

**Sub-questions, as settled (user, 2026-09-08):**

1. **Driving does not consume the origin.** The origin is not a pass; it is
   the data the hidden *state* struct is initialized from, per iteration —
   each `for` mints a fresh machine, and only the machine is consumed. So a
   `for` over an origin is repeatable by design, and "getting a new pass is
   constructing a new instance" refers to the machine (which the sugar
   constructs), not the origin. What replay-by-origin must not become is the
   old factory ambiguity, and the standing capture rules cover it: the yield
   fn's origin parameter is non-`Mut`, and an immutable capture is the case
   both backends agree on ([iter-mut-param]'s parity argument). *R3 note:*
   whether the machine holds the origin by clone or by reference is
   observable only if the origin can be mutated while a pass is alive —
   decide it deliberately, with a test, when the emission is built.
2. **The fn is spelled `next` against the clause — yes.** The declaration
   check is one rule with two accepted shapes (raw signature, or `yield fn`
   whose return type matches the clause's `T`), and there is no free-standing
   `yield fn`: one without an origin struct would reintroduce an anonymous
   generator type.
3. **The fn-field question is not about `yield` at all** — explored at the
   user's request, and the user's own premise ("the yield form lowers to the
   raw form") is what settles it. A customer writing a map-shaped combinator
   with a **raw** `next` hits the identical refusal today, no `yield`
   involved: between `map_lazy(xs, f)` returning and the `for` driving, the
   callback has to live somewhere, and the origin value is the only carrier —
   so `Mapped { src: P, f: (T) -> U }` needs a fn-typed field in the *raw*
   form too. (The R0 probe that found the refusal was exactly this shape: a
   plain struct with a `(T) -> U` field, refused by the Rust backend, accepted
   by Kotlin.) It looked yield-specific only because the pre-origin shape had
   the combinator *be* the yield fn, so the callback went straight from
   parameter into the hidden machine, where the Rust emitter writes
   `Rc<dyn Fn…>` freely. Callback-less composition (`zip`, `merge`, `chain`)
   is untouched either way. The alternatives, presented 2026-09-08:
   * **(a) lift the refusal as `Rc<dyn Fn…>`/function-object fields** — what
     generated structs already do on both backends; uniform in every
     position; costs an allocation at construction and dynamic dispatch per
     call, which is what Kotlin pays for every function value anyway.
     **DECIDED (user, 2026-09-08): this one.** Lands with R5, where std's
     combinators become origin structs and need it.
   * **(b) lift it by hidden monomorphization** — the field becomes a hidden
     type parameter on Rust (`Mapped<P, T, U, F: Fn(&T) -> U>`, literally
     `std::iter::Map<I, F>`); zero-cost but the Rust type becomes unnameable,
     so such a struct cannot sit in another struct's field and a function
     returning one is `impl Trait` (one concrete closure per fn — two return
     sites with different lambdas are an error). Restrictions would mirror
     the interim linear rule's "locals, params, returns only". Viable as a
     later invisible optimization of (a); not worth its rule surface now.
   * **(c) keep the refusal and make std's combinators intrinsic** —
     permanent divergence, customers can never write their own combinator on
     Rust, and it contradicts R5's plan of `map_lazy` as an ordinary origin
     struct in `seq.sv`.
   Decided: (a); (b) stays available as a later invisible optimization.

### Gotchas that will bite, from the work that just landed

- **A fast path hides the slow path.** `map`/`filter`/`reduce` have `List`
  overloads that overload specificity always picks, so the generic body went
  unemitted for months and three defects surfaced at once when `map_to` reached
  it. Any new std function with a fast path needs a test that reaches the
  generic one deliberately.
- **kotlinc will not smart-cast a mutable property.** `if (slot is X) slot.m()`
  on a field is rejected; bind to a local first.
- **`rc_fn_type`-style chained `unwrap_or` bugs**: each strip must fall back to
  its own input, not to the original.
- **An owned parameter whose type says `Mut` needs a `mut` binder** even when the
  body never reassigns it — passing it to a `Mut` position is not an assignment,
  so a scan for assignments cannot see it.
- **`return None` in a `None`-returning fn** must emit a bare `return` on both
  backends; the test is the *fn's* return type, not the value's.
- **Generated code must be warning-free**, and both backends' `runtime_tests`
  compile the runtime modules on their own to enforce it.

### Deferred: `defer` as an effect with a `defers` block — **closed 2026-09-10**

Never built, and now moot: `defer` itself was deleted (see the decision log), so
generalizing it into a `defers { … }` block with a `Defer` effect is no longer a
generalization of anything. ROADMAP.md keeps the one argument worth carrying
forward if the *capability* is ever wanted — registering cleanup on a caller's
scope — together with the objection that killed it: `defer` was cheap because it
had zero runtime representation, and a dynamic queue costs an allocation and
brings the capture question back.

## Roadmap: iterators — Salvo-level pull iterators (built 2026-09-07/08, **largely superseded 2026-09-08**)

**Read the section above first.** What follows is the design that was decided
and built on 2026-09-07/08 — the protocol, `once Iter<T>`, the `yield`
lowering, the effect claim on producer types, and the combinator surface. The
reduction to `next` supersedes the *type* half of it and keeps the machine half;
the record stays because the reasoning, the prototypes and the defects found are
what the reduction is standing on.

The design above works, and its cost is concentrated in one place: the
Rust body is lowered through `async` because stable Rust has no
generators, and *that* is what forces [iter-effect-free]. An `async`
closure captures its environment `'static`, while a handler arrives as
`&mut dyn E` borrowed for one call, so a suspended producer cannot hold
one. Every other compromise follows from the same root: the `Rc<dyn Fn()
-> Box<dyn Iterator>>` factory, the `T: Clone + 'static` bounds, the
clone-params-per-pass, the `Fn`-not-`FnMut` convention exception, and
eager `map`/`filter` in `seq.sv`.

The exit is to stop borrowing the target language's coroutine transform
and materialise a pull iterator as a **state struct the compiler writes**,
whose `next` takes the effect handlers as parameters like any other Salvo
function. Then effects thread in per resume, nothing is captured, and
[iter-effect-free] dissolves — without going push, so `zip`, `merge` and
lookahead stay expressible.

### How this was decided

A push design was costed first (`yield` as a tail-resumptive effect, à la
Koka: producer runs inside the consumer's dynamic scope). It was
**rejected as the iteration model** (user, 2026-09-07) once it became
clear that its sole real advantage — effectful producers — is a property
of *owning the suspension*, not of pushing, and that push can never zip or
merge two lazy sequences (inverting a push producer needs the `ctl`-clause
continuation capture Koka has and the Rust backend cannot). Pull → push is
free, so a yielding *consumer* can be layered on later if some feature
turns out to need it; nothing in this design forecloses it.

### Decisions taken (user, 2026-09-07)

1. **Pull stays the model.** Push revisitable as an addition, never as a
   replacement.
2. **The protocol is Salvo-level, not intrinsic**: `fn next(st: Mut St)
   -> Emitted T | Finished => st: Mut`, gathered in a `params Iterator<St, T>`
   group beside the existing `params Iterable<It, T>`. `has_next`/`next`
   retire. Consequences: a user can write an iterator *by hand* (which is
   how `zip`/`merge` become ordinary structs — no materialising one side),
   and the state type binds per call site through the `params` group, so
   it never has to be named in a signature.
   * Tagged `Emitted T | Finished`, **not** `T?`: Salvo's unions are flat, so
     `Iter<Int?>` would collapse `Int??` and lose the end signal. It
     lowers to the generated `Union2`; the emitter may special-case
     `Option` where the element type is provably non-nullable.
3. **Effects thread into `next`** like any other function, so an iterator
   function may perform them and [iter-effect-free] goes away. An
   effectful iterator is consequently not a `std::iter::Iterator` (its
   `next` takes extra parameters), which costs nothing — the `for` lowering
   is ours. Kotlin *could* instead capture the handler at creation (a JVM
   handler is just a reference), and must not: creation-time vs
   consumption-time binding is observable when an iterator outlives the
   `use` scope that made it, i.e. exactly the class of [backend-parity]
   divergence the laziness episode was about.
4. **`close` is mandatory and compiler-injected.** See "Abandonment"
   below.
5. **Boxing at the meeting point.** Each iterator function has its own
   state type, so a position holding either of two producers gets one
   boxed value there and nowhere else. Recorded with an example in
   LANGUAGE.md ("Planned change: Salvo-level pull iterators") for further
   thought. The alternative — rejecting such positions
   [backend-never-wrong] — would have kept Rust output free of `dyn`
   entirely at the price of banning an `Iter<T>` struct field.
6. **One uniform lowering on both backends**, hidden behind hand-written
   runtime modules so user-facing generated code stays concise. Those
   modules move out of the emitters' string literals into real source
   files under each backend crate (`runtime/iter.rs`, `runtime/iter.kt`,
   pulled in with `include_str!`) — which makes them reviewable as code
   and, more importantly, lets a test hand them straight to `rustc` /
   `kotlinc`, turning the "hand-verify the generated shape first" gotcha
   into a standing test. `[mod-used-only]` still applies, and a test must
   compile *emitter output against the checked-in runtime*, since the two
   can now drift in a way a string literal made impossible.
   * The same treatment is wanted for `defer` on Kotlin: an
     `inline fun <R> deferScope(f: DeferScope.() -> R): R` in the runtime
     module instead of the current try/finally splices. **`inline` is
     mandatory** — Salvo's `defer` bans control flow out of the *deferred*
     body, but the enclosing block's `return`/`break`/`continue` must
     still cross the scope function, which Kotlin permits only through an
     inline lambda. It trades a zero-allocation splice for one scope
     object per block.
7. **Two tiers over one protocol**: `yield` functions (compiler-generated
   state machine) and hand-written struct + `next`. Both satisfy
   `params Iterator<St, T>`, so `for`/`map`/`filter` accept either — which
   lets tier 1 stay deliberately incomplete without blocking anyone.
   * ~~**Simple-generator fast path**~~ — **dropped 2026-09-07** (user,
    following the I3 prototype). One lowering, not two: the fast path would
    have covered less than it looked (a body with statements *after* its
    yield, or any `break`/`continue`/`defer`, does not qualify — including
    `naturals`, the repo's own demo producer), while the flat machine reads
    acceptably for simple bodies. Two lowerings is two things to keep in
    agreement, which is the divergence risk the laziness episode taught.
8. **Consumer side lowers to `while let`**; a `for` over an obvious
   backend iterable (list, array, `Str`) keeps emitting a native loop, so
   the common case pays nothing.
9. **Factory and pass are distinguished at the type level, by `once`** —
   see the next subsection. This retired the open DECISION rather than
   answering it: both exist, and the author of an iterator function picks.

### Abandonment, and why `close` is mandatory

A consumer that `break`s stops driving an iterator before it reports
`Finished`, leaving the body suspended at a `yield` forever. Today that is
harmless *only* because of [iter-effect-free]: a producer that cannot
perform effects holds nothing worth releasing, and its pending `defer`
blocks are skipped invisibly (dropping the future / abandoning the
coroutine never runs the emitted exit code). Threading effects in creates
the problem — `let f = open(path); defer { close(f) }` inside a producer
leaks the handle on every `break`.

Because the machine is ours, each state knows which defers are pending, so
it gets a **close path** that jumps to the unwind states and runs them
latest-first. The compiler injects the call on every exit out of the `for`
— exhaustion, `break`, `return`, a `Throw` transfer. Deterministic on both
backends, and it needs no destructors: Kotlin has none, and a Rust `Drop`
could not take the effect parameters a deferred block may need.

**On the linear-types angle** (user asked whether the injection could
later be removed): must-use linearity is already *built* — L6, done
2026-09-02, rules [linear-obligation] / [linear-discard] / [linear-canbe].
So the obligation is expressible today: declare the pass type
`canbe linear` and draining-or-closing becomes the ordinary all-paths
obligation check, with `close` as its `discard`. The injection is
therefore a *convenience* (the `for` lowering is the one place the
compiler always knows every exit path) rather than a workaround for a
missing feature, and it can be relaxed to a plain obligation whenever we
want the user to see it. The umbrella roadmap already exists — "Roadmap:
toward full linear types" above, remaining phases L5 (places and partial
moves) and L7 (derived-return annotations) — so no new roadmap item was
added; what is *not* yet decided is whether a hand-written iterator's
state must be `canbe linear` by rule.

### Factory and pass: `Iter<T>` vs `once Iter<T>` (user decision 2026-09-07)

The 2026-09-05 costing deferred a second question — whether a lazy `Iter`
is **one-shot** — and a state-struct materialisation brings it back, now
sharper: with producers allowed to perform effects, replaying a pass
replays its I/O. Both branches were costed (factory: two loops over one
value both replay, silently re-reading a file; pass: the second use is a
consumption error, and re-reading means calling the producer again, where
the cost is visible).

Neither branch was taken. **The distinction moves into the type**, and
`once` already means exactly what a pass needs — with no new rule to
write:

```
Iter<T>        // factory: replayable; a fresh pass is minted per use
once Iter<T>   // pass: a position in a sequence, consumed by driving it
```

Why `once` fits with nothing invented:

- It is already specified as **never droppable** ("it restricts rather
  than refines", LANGUAGE.md; [qual-*]: the compiler owns permissions,
  which drop, and obligations, which do not). A pass must never be
  forgettable into a replayable recipe, and it cannot be.
- Its **variance is already inverted and already the direction needed**:
  "any ordinary function value can be used where a `once` one is
  expected — never the reverse" generalizes to "a factory fits where a
  pass is wanted, never the reverse".
- The **conversion in the permitted direction is already a call**: the
  implicit `iter()` that `[iter-resolve]` inserts *is* factory → pass.
- **Enforcement is the existing consumption machinery**
  [deduce-consume] / [once-fn]; no new analysis.
- The backend mapping follows the existing pattern (`once` fn params
  already compile to `FnOnce`): `once Iter<T>` is the owned state struct,
  plain `Iter<T>` the argument bundle it re-mints from. Boxing at the
  meeting point (decision 5) applies to each form independently.

Rejected alternative: a second nominal type (`Pass<T>` / `Cursor<T>`). It
would need its own variance rule, its own never-drop rule and its own
name in every signature — three things `once` already has written down.
The one objection to `once` is that a qualifier would gate the *operation
set* (`iter` versus `next`), and `Mut` is the precedent for exactly that
(it gates the mutators, overloads select on qualifiers, and a backend may
render `Mut T` as a different type with the compiler inserting the
conversion).

Consequences recorded with it:

- **`once` generalizes from call-multiplicity to use-multiplicity**
  (user, 2026-09-07), with the fn case as the instance where using means
  calling. Valid positions stay explicit — fn types and `Iter<T>` — and
  whether it applies to *any* type is roadmap **D6**.
- **An effectful producer may return a factory** (user, 2026-09-07).
  Mechanically fine, since handlers arrive per `next`; semantically,
  replay re-does the I/O, and `Iter<T>` visibly means replayable, so the
  type carries the warning. Forbidding it would outlaw the legitimate
  read-a-file-twice case. Each minted pass is closed at its own loop
  exit, so nothing leaks either way.
- **The close obligation stays injected, not declared.** `once` covers
  no-replay; the injected `close` covers no-leak. Making the obligation
  visible in the language needs linearity conditioned on a use-site
  qualifier — roadmap **D7**, which records the relation to this change.

### Still open

**See ROADMAP.md** for the I4-era leftovers and refusals, and for the `once`- and
linearity-shaped questions (D6, D7) this section forwarded to.

### Producer parameters: `Mut` refused (user decision 2026-09-08)

**The decision: option A — refuse it.** An iterator function may not take a
parameter whose type is transitively mutable [iter-mut-param], by the same test
[fate-move-mode] uses (`Mut` at any depth: type argument, array/tuple/union
component, struct field). The diagnostic names the two remedies that work
everywhere — yield the values and let the consumer collect them, or reach the
outside through an effect.

Forced by the parity defect recorded under "Open defects": a `yield` fn taking
`sink: Mut List<Int>` mutated a private copy on Rust and the caller's own list
on Kotlin, with successive passes accumulating on one target and starting fresh
on the other. It is a language question rather than an emitter bug because a
producer's parameters are captured for the lifetime of a **factory**, which can
mint a pass at any later time — so "who owns the argument" is semantics.

**Why A over the alternatives.** B (copy per pass on both backends) is the
trap: it compiles, agrees on both targets, and silently discards the writes the
caller was expecting. C (share on both backends) is what someone writing
`sink: Mut List<Int>` actually expects — and doing it properly on Rust *is* the
`Cell` work, since a captured `Mut` parameter needs `Rc<RefCell<…>>` with its
run-time panic surface. So A now, C reconsidered under "Roadmap: shared mutable
state (`Cell`)", where it is recorded as a customer of that machinery.

**Nothing in `std/` or the test corpus does this**, so the rule landed with no
sweep: three tests in `fn_effect_tests.rs` (direct, transitive through a struct
field, and a control that an *immutable* parameter of the same shape is fine —
and that a `Mut` parameter on an ordinary fn is untouched).

**Open, raised by the user with the decision: should `once Iter<T>` be allowed
one after all?** A pass is minted once and driven once, so the ambiguity that
sank the factory case — do two passes share the collection or each get their
own — does not arise. What remains is that the caller must not touch the
collection while the pass is alive, which is exactly **shared fate**
([fate-link], the S1/L-series machinery: links, poison, and the
consumed-use error). The blocker is representational, not semantic: a pass that
*borrows* its argument needs to be the state machine itself rather than a boxed
factory value, i.e. I2c's remaining half. Recorded there as a follow-on, not
decided.

### D8 decided (user, 2026-09-07): a producer's effects live on its type

**The decision: option B, spelled as a qualifier.** `FileSystem Iter<Str>` is a
producer whose *driving* performs `FileSystem` [iter-effects]. Three calls, all
the user's:

1. **The spelling is qualifier position**, not a bracket list. Prefix
   (`[FileSystem] Iter<T>`) was ruled out because it reads as a deduction list
   in return position; postfix was viable but disliked; the qualifier form
   costs no new grammar (a type is already a sequence of qualifier names
   before a base) and lands on `once`'s precedents for every rule it needs.
2. **A `yield` fn declares them on its return type**, not in its own effect
   list — because none of its body runs when it is called, so a list on the
   function would make [fn-effects] demand a handler at every call site for
   something calling it never does. Its body is checked against the claim.
3. **Position rule 2**: the claim may be written where `once` may — `Iter<T>`
   and a `canbe once` type of one's own — with fn types excluded, since they
   have the bracket spelling. One predicate for both qualifiers, which is what
   I2b's collision argued for.

What made B the recommendation, after D was proposed first and withdrawn: D
answers the *representation* question (which struct an indirect call dispatches
to) and leaves the *declaration* question open, and effects are not inferred
per call site in this language. The rule that settles it already exists for the
value kind that had the problem first — **"a fn inherits its fn-typed
parameters' effects"** (2026-09-04), with inverted variance, a walk through
qualifiers/optionals/unions in `check.rs::inherited_fn_effects`, and the
sentence beside it: *a fn value carries no capability, so it may be stored and
passed freely — only calling it needs its effects in scope.* A producer is the
same kind of thing, and D8 is what makes driving one the call that needs them.

**Asked and answered along the way**: is any of this a consequence of having
both `iter()` (factory) and `next()` (pass)? No. The problem is *abstracting
over producers at all* — a position that can hold either of two producers is
an indirect call with one signature — and a pass-only design still needs the
abstraction for the *return* type (a generated state struct has no name) while
making the parameter case worse: the effects of the `next` it was handed vary
per instantiation, which needs an effect variable (row polymorphism) where the
nominal claim needs one more arm in an existing walk. Dropping the factory is
still a fair question on its own merits — one representation instead of two —
but it is not a way out of D8. The only two things that would remove D8 are
giving up polymorphic combinators, or inferring effects through driving
positions, which is the silent colouring the effect system exists to prevent.

### The costing that led there (kept for the reasoning)


I4 removes [iter-effect-free] by giving the generated machine's `advance` the
effect handlers as *parameters* (decision 3): nothing is captured, so a
producer may perform effects while the consumer drives it. The direct cases
need nothing new — a `for` over a producer *call* is a direct call, and a
producer nested inside another producer is a **field** of the outer pass, so
its concrete type (and therefore its handler list) is known at both.

What is not decided is the **indirect** case. A position whose *declared* type
is `Iter<T>` or `once Iter<T>` can hold either of two producers, so the value
there is behind a trait object (Rust) or an interface (Kotlin) — and an
indirect call has **one fixed signature**. Which handlers does it take? The
positions that force the question: an `Iter<T>` **parameter**
(`fn tagged(xs: Iter<Int>) -> Iter<Str>` — every std combinator), a **struct
field**, a **union arm**, and a local that can hold either of two producers.
This is decision 5's "boxing at the meeting point" meeting decision 3.

Salvo has already answered the same question once, for function *values*: the
type carries the effects (`(s: Str) [Logger] -> Str`) and whoever calls the
value supplies them [fn-effects]. That is the precedent option B rests on.

**A — effect-free at the boundary.** `Iter<T>` means an effect-free producer;
an effectful one may be created, consumed, and nested inside another producer,
but may not flow into an `Iter<T>`-typed position (an error naming the
remedy). No new syntax, smallest change. Cost: `map`/`filter`/`zip` — anything
taking a producer as a *parameter* — would only accept effect-free producers,
which is most of the composition story.

**B — `Iter<T>` carries its effects, like a fn type** (spelling TBD, e.g.
`Iter<Int> [Console]`). A position states what its producer may perform; the
emitters generate one interface per effect set, exactly as they generate one
`UnionN` per arity; fewer effects is usable where more are expected, which is
the variance fn types already have. Cost: new type syntax, effect lists in
more signatures (including std's), and the subtyping rule to go with them.

**C — one erased handler bundle** (`advance(&mut self, env: &mut SalvoEnv)`).
Fixed signature, no syntax. Rejected on inspection: the consumer would have to
supply handlers for effects it may not declare, and at an indirect drive site
the checker cannot tell which those are — a verifiability hole, so this is
recorded as considered rather than offered.

**D — specialize the parameter case.** An `Iter<T>` *parameter* becomes
generic in the emitted code (Rust `impl SalvoPass<T>`, Kotlin a type parameter
plus one generated interface per effect set, inferred per call site), so
composition keeps working with effectful producers and no syntax is added.
Storage positions (struct field, union arm) have no call site to infer from,
so they still need A's restriction or B's annotation.

**Recommendation as it stood: B** — revised 2026-09-07 after writing the examples out (see
below), which showed D cannot carry it. D fixes the *representation* (which
struct the indirect call dispatches to) and leaves the *declaration* open: a
combinator that drives its subject performs whatever the subject performs, so
`fn take<T>(xs: Iter<T>, n: Int)` has to end up declaring `FileSystem` when it
is handed `lines_of(...)` and nothing when it is handed `naturals(0)`. Effects
are not inferred per call site in this language; they are read off the type.

And that rule already exists, for the value kind that had the same problem
first: **"a fn inherits its fn-typed parameters' effects"** (user decision
2026-09-04, [fn-effects]) — `fn run(f: (s: Str) [Logger] -> Str)` needs no list
of its own, its callers must supply `Logger`, inheritance reaches through
qualifiers, optionals and unions, and *fewer* effects fits where more are
expected. It is implemented in `check.rs::inherited_fn_effects`, which walks a
parameter's type for exactly this. Teaching that walk about `Iter<T> [E]` is a
one-arm extension rather than a new mechanism.

The clincher is the sentence next to it: **"a fn value carries no capability,
so it may be stored and passed freely — only *calling* it needs its effects in
scope."** A producer is the same kind of thing: storing one is harmless, and
*driving* it is the call that needs the handlers. Under B the storage case —
the one A would have forbidden and D could not describe — reads correctly
instead: `struct Feed { name: Str, items: Iter<Str> [FileSystem] }` says what
driving the field will perform, so the loop that drives it must sit somewhere a
`FileSystem` handler exists.

What is left to decide under B is the **spelling** (a type with no arrow has
nowhere obvious to put the list — `Iter<Str> [FileSystem]`,
`[FileSystem] Iter<Str>`, …) and whether the annotation is valid on any type or
only where a *driving* operation exists, which is the same question `once`'s
position rule answers (`types::once_position`), and is best answered the same
way.

D is still worth having *inside* B, as an optimization rather than a
substitute: where the concrete pass type is known at a parameter's call site,
the emitted code can specialize instead of boxing (decision 5's "boxing at the
meeting point, and nowhere else").

**The examples that decided it** (the three positions, smallest first):

```
fn naturals(from: Int) -> Iter<Int> { … }                    // no effects
fn lines_of(path: Str) [FileSystem] -> Iter<Str> {           // effects, per I4
    let handle = open(path)
    while has_more(handle) { yield read_line(handle) }
    close(handle)
}

// 1. a parameter — every combinator
fn take<T>(xs: Iter<T>, n: Int) -> Iter<T> { … for x in xs { … yield x … } }
take(naturals(0), 3)         // xs is a __Pass_naturals
take(lines_of("a.txt"), 3)   // xs is a __Pass_lines_of, and driving it needs
                             // a FileSystem handler `take` never mentions

// 2. one local, two sources
let src = if from_file { lines_of("data.txt") } else { canned() }
for line in src { println(line) }

// 3. storage, where the drive site is nowhere near the creation site
struct Feed { name: Str, items: Iter<Str> }
for f in feeds { for item in f.items { println("${f.name}: ${item}") } }
```

Case 1 is what killed D: the representation question ("which struct does the
indirect call dispatch to") is answerable by specializing, and it leaves the
declaration question untouched. Case 3 is what killed A: with the effects in
the type the field is fine, and A would have had to forbid it.

Consequences of the decision, whichever way it goes:

- **The injected `close` lands with I4, not before.** It is built into the
  plan (`__close` is emitted) but nothing calls it, and while producers are
  effect-free nothing it does is *observable*: a producer's deferred blocks can
  only touch its own state, which dies with the pass. A test for it could not
  fail today, so it waits for the release path to have something to release.
  *(It landed with I4's emission half the same day — the `for` over a claiming
  producer is its caller, and the abandoned-consumer `close` prints.)*
- **I2c's representation split follows from the same choice**: what
  `once Iter<T>` renders as (a concrete pass, a boxed one, or a generic
  parameter) is exactly what D8 decides.
- **I5 (lazy `map`/`filter` in std) follows too**: a lazy combinator calls its
  callback *inside* a producer, so a callback that performs effects is legal
  only once producers are.

### I2b as built (2026-09-07): the protocol, and the collision it found

Built and verified so far:

- **`std/core/iterator.sv`** declares the protocol in Salvo rather than in
  the compiler: `qualifier Emitted<T> of T` with its constructor, a fieldless
  `struct Finished {}`, and
  `params Iterator<St, T> { fn next(st: Mut St) -> Emitted T | Finished }`. => st: Mut
  Names are the user's (2026-09-07), renamed from `Next`/`Stopped` before
  any of it was written.
  * `Emitted` is a *qualifier* so the element keeps its own type and a
    sequence of optionals still has a distinguishable end (`Emitted None |
    Finished` has two arms where `None | None` would have one). `Finished`
    is a *struct* because it has nothing to qualify, and reusing `None`
    would say "absent" where the claim is "the sequence ended".
- **`Checked::for_drivers`**, a side table keyed by the subject's span,
  holding the `next` overload a `for` drives. The driving loop is
  synthesized, so there is no call node for the emitters to resolve — the
  choice has to be handed over.
- **`pass_elem_ty`** resolves `next` *before* `iter` in `iter_elem_ty` (a
  type with both is already a position in a sequence, so minting a second
  pass from it would be wrong), matching on the bare types since a
  subject's own `once`/`Mut` say nothing about which `next` fits, and
  accepting only the exact `Emitted T | Finished` shape — anything else is
  some other `next`, not a driver.
- **The "not a pass" error** fires and reads well:
  ``` `Countdown` has a `next` but is not a pass: driving it uses it up, so
  it has to be declared `Once Countdown` — annotate the return type of the
  function that builds it ```

**The collision, and how it was resolved.** The error's remedy was at first
impossible: I2a restricted `once` to function types and `Iter<T>`
deliberately, to keep a general affine qualifier as roadmap D6 — but a
hand-written pass is a *user type*, so "hand-written iterators must say
`once`" requires `once` on user types. Three ways out were costed (`canbe
Once`; take D6 now; or make `once` valid on any type that has a `next`,
which would make a *type*'s legality depend on which functions are in
scope). **User decision 2026-09-07: `canbe once`**, with the note that D6
and D7 should be designed together rather than piecemeal.

- **`canbe once`** joins `canbe Mut` and `canbe linear` [canbe-optin]: a
  type opts into being a pass the way it opts into mutability and
  linearity, and `once` stays inapplicable to types that never asked. The
  author declaring it is the same argument the "not a pass" error rests on
  — an obligation should not attach on the strength of a method name.
- The position rule is now in **two halves, on purpose**:
  `types::once_position` (fn types, `Iter<T>`) and the checker's
  `has_auto_once` (the opt-in, which needs the declaration).
  `is_subtype`'s inverted `once` rule checks *neither* — where a qualifier
  may be **written** is a different question from what it means once
  present, and an unauthorized `once` has already been reported, so
  accepting it in the weakening direction keeps one mistake to one
  diagnostic. That is a deliberate reversal of I2a's "one predicate, no
  drift" arrangement, which stopped being possible once the answer depended
  on a declaration.

Tests: `once_tests.rs` grew to 15 — the opt-in as a valid position, the
`canbe` allowlist rejection, a hand-written pass driving a `for`, driving it
twice, and the not-a-pass error.

### I2c first half as built (2026-09-07): hand-written passes run

**`zip` works.** The driving-loop emission landed on both backends, which is
the point at which the manual half of the iterator story stops being a
declaration and becomes a feature: a struct, a `next`, and `for` drives it —
no state machine, no compiler support beyond the loop.

- **What the checker hands over** grew from a `FnKey` to a `PassDriver`: the
  overload *and* the arm identity (`emitted_arm`, `arms`). Deriving the arm
  index in each emitter instead would be precisely the checker/emitter
  disagreement the invariants forbid, and the checker already has the
  lowered return type in `pass_elem_ty`.
- **Rust**: `let mut __loop1_pass = countdown(3);` then
  `while let Union2::U1(mut n) = next(&mut __loop1_pass) {`. A `while let`
  re-evaluates its condition per turn, so `Finished` needs no arm.
- **Kotlin**: `while (true)` plus `if (step !is U2_1<Int, Finished>) { break }`,
  since Kotlin has no pattern-matching loop condition. The arm is spelled
  with its **real type arguments** when `next` is non-generic, which is what
  keeps the element read cast-free: a star-projected arm leaves `value` at
  `Any?`, and casting back warns ("unchecked cast") in code the user cannot
  edit. A generic `next` has type arguments the loop cannot see — there is no
  call node — and falls back to stars plus a cast.
- **`next` must take its state as `Mut`**, checked: the right result shape
  with a non-`Mut` state gets a diagnostic saying so, rather than falling
  through to a puzzling "not iterable".
- **An effectful `next` is a codegen error** on both backends. This is the
  *hand-written* pass path (a `once` type with its own `next` fn, I2b), which is
  a different mechanism from a generated producer's claim: I4's emission half
  threads handlers into a machine the compiler wrote, and has nothing to say
  about a `next` the author wrote. Threading them into every turn of a
  `for_drivers` loop is still open [backend-never-wrong] — see the leftovers.

**Defect found and fixed: an empty struct emitted invalid Kotlin.**
`struct Finished {}` became `data class Finished()`, which kotlinc rejects
("data class must have at least one primary constructor parameter"). A
fieldless struct now emits a plain `class` [kt-struct-empty] — nothing is
lost, since with no fields there is no state for `equals`/`copy` to compare
or clone. Latent since structs were implemented; nothing had ever declared
an empty one until std's iterator protocol did.

Verified end to end with one shared program and one shared expected stdout on
both backends: a `Countdown` pass, and a `Zip` reading two lists at once
(`n 3 / n 2 / n 1 / ada is 36 / grace is 45 / done`), plus emission-shape
tests on each side and the `kt-struct-empty` regression test.

**What is left of I2c**: the representation split — a pass becoming the
generated state struct, a factory keeping the arguments it re-mints from, and
the async machinery going away. That is the `yield` half; the manual half is
done.

### I3 prototyped (2026-09-07): the general lowering is mechanical

Before writing any state-machine lowering, the gnarly shape was hand-written
and checked against an **oracle**, per the standing caution. Everything lives
in `experiments/pull-iterators/` (`gnarly.sv`, `gnarly.rs`, `gnarly.kt`,
`gnarly_oracle.rs`), with the README recording how to run it.

The body puts everything resumable in one place: a fn-level `defer`; a
`defer` **inside a loop body**, registered anew each iteration and reading a
per-iteration local; a nested loop over *another pass*, alive across the outer
body's suspensions; a `continue`; two yields per outer iteration and a third
after the loop (four resume points); effects performed by the producer *and*
by a deferred block; and a consumer that stops early, so the release path runs
with the body suspended mid-nest.

**The oracle is the part worth copying.** A hand-written machine is only as
trustworthy as whatever decides the expected answer, and hand-tracing when a
*suspended* body's deferred blocks run is exactly the reasoning most likely to
be wrong. So the same body is written in *push* style — where `yield` is a
callback and no bookkeeping exists — with each `defer` expressed as a Rust
`Drop` impl, making **Rust's own scope discipline** the authority on ordering.
Salvo's `[defer]` rule is precisely Rust's drop order for locals. Both
machines then had to match it, on both exit paths (break after four elements,
and drain — the only path that reaches the yield after the loop and the normal
end of the body). All four programs agree, byte for byte.

What it established:

- **The lowering is mechanical**: numbered resume points, the body's locals as
  fields, a flat `loop { match state }` dispatch. Nesting needed no special
  case — an inner loop is just more states.
- **`continue` is staying in the same state**, and a **yield in the middle of
  a loop body** is free: the state *after* the yield is "the statements after
  it", and the back edge is a transition.
- **A nested pass becomes a field**, which is also exactly why a *recursive*
  producer needs a `Box`: that field would have the struct's own type.
- **One slot per `defer` site is enough — and now the reason is known rather
  than assumed**: a loop-body `defer` is discharged before the back edge, so
  it cannot outlive its iteration and no stack is needed. What it needs
  alongside the flag is the local the deferred block reads, which is already a
  field because every local is.
- **The release path is unchanged from I1b**: flags, latest-first, idempotent,
  one `close` after the loop covering `break` and exhaustion alike.

### I4 emission prototyped (2026-09-07): the shape, and the adapter nobody asked for

Before teaching the emitters to thread handlers, the target shape was
hand-written on both backends and run —
`experiments/pull-iterators/effectful.{sv,rs,kt}`, one source, one expected
stdout, verified byte-identical with `diff` and warning-free from both
toolchains. The same discipline as I1b/I3, and it paid the same way: it turned
up a piece of machinery the type rules do not hint at.

The program is the awkward one on purpose: a producer that performs `Console`
while the consumer drives it, holding a `defer` that *itself* performs an
effect; a consumer that `break`s after two elements; a second consumer that
drains; a fn inheriting the claim from a producer parameter; and a pure
producer passed into a claiming position.

**What it fixes:**

- **A trait (interface) per effect set**, generated like `UnionN` per arity: a
  claiming pass cannot be a `std::iter::Iterator` or a Kotlin `Iterator<T>`,
  because its `advance` takes a handler. `Console Iter<T>` becomes
  `Rc<dyn Fn() -> Box<dyn SalvoPassConsole<T>>>` on Rust and a
  `fun interface { fun mint(): SalvoPassConsole<T> }` on Kotlin. The pure path
  is untouched, and the boxing costs nothing new — it already boxed.
- **The factory stays a factory**, so replayability is unaffected by the claim.
- **Variance needs an adapter.** [iter-effects] says a producer performing
  *fewer* effects fits where more are expected; on both backends that is a
  **representation** change, so the compiler has to wrap a pure producer at
  that boundary with an `advance` that ignores the handler. Nothing in the type
  rules says so, and discovering it mid-emitter would have been expensive.
- **The injected `close` is observable at last**, which is why it waited: the
  breaking consumer's `close` prints, the draining one's runs exactly once from
  the body's own exit, and one call after the loop covers both because the
  flags make it idempotent.
- **Kotlin's `advance` returns a Boolean and leaves the element in
  `current()`** rather than returning `T?` — the same reason the protocol tags
  its end: a `T?` return cannot tell "no more" from "the element is null".

**The table between the halves landed with it.** The handler *order* has to be
the same in three places — the generated trait per effect set, the pass's own
`advance`/`close`, and every `for` that drives one — so it is derived once, by
the checker: `Checked::producer_effects` (per producer `FnKey`) and
`Checked::pass_effect_sets` (the distinct sets, one generated trait each, the
way `union_sizes` drives one `UnionN` per arity). Both backends read that table
rather than re-reading the written type — first for the refusal, now for the
emission itself, which is what keeps the handler order one decision instead of
three.

Writing it turned up the ordering trap: `Ty::qualify` **normalizes** qualifier
order, so `Counter Logger Iter<Int>` and `Logger Counter Iter<Int>` are the same
type — two producers written the two ways would have disagreed on their
machine's parameter order, and a trait generated per *set* would have fitted
neither. The recorded order is therefore sorted, not written, with a test that
pins exactly that.

What remains for the emitters, in order: generate a trait (interface) per entry
of `pass_effect_sets`; render a claiming producer's `advance`/`close` with the
handlers as leading parameters in the recorded order (the emitters already
thread handlers this way for declared effects); lower a `for` over a claiming
subject to mint/advance/close; insert the variance adapter at a pure→claiming
boundary; and drop the two refusals.

### I4 emission as built (2026-09-07): both backends, one program, one stdout

All five steps landed, in that order, Rust end to end first and Kotlin after.
The acceptance test is the prototype itself: `effectful.sv` is now a test source
in *both* backends' suites, with the same expected stdout, compiled and run by
`rustc` and `kotlinc` (`rustc_compiles_and_runs_an_effectful_producer`,
`kotlinc_compiles_and_runs_an_effectful_producer`). Byte-identical, no warnings
from either toolchain, and it worked on the first run on each side — which is
what a fixed shape buys.

**The two unknowns, answered before writing much.**

- **Where the generated trait file lives, and how it names effect traits across
  modules.** The pure runtime files (`iter.rs`, `iter.kt`) have no cross-module
  reference at all, so there was no import machinery to reuse. There is now
  still none: the file is `iter_effects.rs` at the crate root (mounted like
  `unions.rs`) / `iter_effects.kt` in the root `salvo` package, and it names
  effect traits by **absolute path** — `crate::core_console::Console`,
  `salvo.core.console.Console`. Both languages accept a fully-qualified type
  anywhere a name goes, so the file imports nothing and cannot collide with
  anything. Modules that *mention* a pass trait pick it up through the glob they
  already have (`use crate::iter_effects::*;`, `import salvo.*`), gated on a
  per-file flag exactly like `union_sizes`.
- **Which expression positions need the variance adapter.** Not a separate
  question, as it turned out: it is exactly what `maybe_coerce` sees. The
  widening is recorded as `Coercion::WidenProducer { from, to, then }` beside
  [str-drop-mut]'s `DropMut` — same funnel, same `then` chaining for the change
  it displaces — so call arguments, returns, `let` annotations, struct fields,
  union arms and branch joins are all covered by one insertion, and the emitters
  do not guess from types. `a_producer_widening_is_recorded_as_a_coercion` pins
  it: two widenings in a program with three producer positions, and *not* at the
  argument whose value already claims the set.

**What the checker's own tables forced on the emitters** — both of them, and
neither obvious from the prototype:

- **A producer fn's `fn_effects` contains its claim**, because that is the
  environment its *body* is checked in. Read naively for the signature, that
  gives the factory a leading `console` parameter no call site passes. So a
  `yield` fn takes **no** effect parameters — which is [iter-effects] restated:
  none of the body runs when it is called.
- **`call_effects` at a `for` subject's span holds the drive site's handlers**
  (the checker records the claim there, since a synthesized driving loop has no
  call node of its own). When the subject *is* a call, that is the same span the
  call-argument logic reads, so `chatty(3)` was handed a handler it does not
  take. Fixed by skipping effect arguments for a producer call: the entry
  belongs to the drive site, and the drive site is where it is read.

**The release path, at last observable.** The `for` lowering is where the
injected `close` finally gets a caller, and the two backends reach the same
behaviour by different mechanisms — which is worth recording, because it is the
first time the parity principle was satisfied by *different* code rather than
the same shape twice:

- **Rust** splices `p.close(h);` after the loop *and* registers it as a deferred
  entry of the driving loop, pushed **before** `loop_defer_floors.push` so a
  `return` out of the body runs it while `break`/`continue` do not (they land on
  the call after the loop anyway).
- **Kotlin** wraps the loop in `try { … } finally { p.close(h) }`, which is how
  `defer` is lowered there already [kt-defer-finally] — so `break`, `return` and
  exhaustion all reach it for free.

Both are idempotent, so landing twice costs nothing; `rustc_releases_a_claiming
_producer_on_a_return` and its Kotlin twin assert the same stdout for a `return`
out of the middle of a driving loop.

**What is still refused** [backend-never-wrong], on both backends with the same
wording:

- a **claiming producer nested inside another producer** — its handlers would
  have to thread through the outer machine, and its slot be typed as the
  generated trait rather than a plain iterator;
- a **generic effect claim** (`Bucket<Int> Iter<Int>`) — the set identity would
  be the backend's rendering of the arguments while the checker's is Salvo's,
  and two spellings of one set would generate two traits that fit neither;
- a **`for` over a claiming producer in value position** — the `close` would
  have to sit in a block that is also producing a value;
- a **widening between two non-empty claim sets** — only a `from_pure` adapter
  is generated, and an adapter per *pair* is work nothing asks for yet.

**Naming, for whoever reads the output next.** `pass_suffix(set)` is the effect
names sanitized and concatenated in the canonical order, giving
`SalvoPassConsole` / `SalvoIterConsole` / `SalvoPureAsConsole`. It is a function
of the *set*, so the trait a machine implements and the trait a drive site calls
are the same trait by construction rather than by agreement.

### The `Throw` question, answered without new machinery (2026-09-07)

The last unknown was a **`Throw` transfer out of a suspended body**. Rather
than prototype the machinery (`next` returning
`ControlFlow<M, Emitted T | Finished>`, pending defers running on the `Break`
path, and a `for` that propagates it), the *alternative* was tested against
the real compiler first: **a fallible producer yields a result, and the
consumer throws.**

It works, today, on both backends with identical output — a `Reader` pass
whose `next` returns `Emitted (Ok Str | Err Str) | Finished`, driven by a
`read_all` that declares `[Throw<Str>]` and throws on the `Err` arm, wrapped
in a `try` (`a_fallible_pass_yields_a_result` in both backends' codegen
tests):

```
line alpha / line beta / read 2 / line alpha / failed: stopped: bad line at 2
```

So the protocol needs nothing: `Emitted T | Finished` stays exactly two arms,
and no machinery has to cross a suspension. **User decision 2026-09-07: a
`yield` fn may not declare `[Throw<M>]`** — a producer that can fail yields a
result. The reasoning is
that `Throw` exists so *intermediate* frames stay silent, and a suspended
generator is not an intermediate frame: it is a value the consumer drives, so
its failure belongs in the value it hands over. The consumer's own `throw`
inside the loop body already works and needed nothing from the producer.

The one thing this turned up is a defect — an inner arm not wrapping into a
union *under a qualifier*, so the element union had to be bound to a local
first. Closed 2026-09-10 under [qual-group]; see "Defects found and closed".

### I4/I2c implementation plan: the `yield` lowering (written 2026-09-07)

The prototypes have fixed the target shape exactly (see
`experiments/pull-iterators/`), so what remains is mechanical rather than
exploratory. The plan, in the order it should be built:

**1. A shared pass in `salvo-core` (`generator.rs`), not two lowerings.**
✅ **Done 2026-09-07** — see "I3 step 1 as built" below for the shape that
shipped and where it deviates from the sketch. Given a `yield` fn body it
produces a plan the emitters *render*; neither backend re-derives control
flow. Shape, as sketched off the prototypes:

```
struct GeneratorPlan<'p> {
    fields: Vec<Field<'p>>,        // params, hoisted locals, nested pass slots
    defers: Vec<DeferSite<'p>>,    // one flag each; the locals they read are fields
    states: Vec<State<'p>>,        // numbered resume points
}

enum Step<'p> {
    Plain(&'p Stmt),                                    // no control flow, no yield
    Register(usize), Discharge(usize),                  // defer flag set / run
    Goto(usize),
    Branch { cond: &'p Expr, then_state: usize, else_state: usize },
    Emit { value: &'p Expr, resume: usize },            // `yield`
    OpenPass { slot: usize, subject: &'p Expr },        // entering a nested `for`
    Drive { slot: usize, pattern: &'p Pattern, body: usize, done: usize },
    Finish,
}
```

The prototype's numbering is the acceptance test: building the plan for
`gnarly.sv` must produce the eight states of `gnarly.rs`, in that order.

**2. Locals become fields, which needs a name-resolution mode.** ✅ Done
2026-09-07 (Rust reuses `BindKind::SelfField`; Kotlin needs it only for `let`). Every body
local reads and writes through `self`. Both emitters already have the concept
for handler state ("a handler constructor param or state field: accessed as
`self.x`"), so this is a binding kind, not new machinery.

**3. Emission per backend** — ✅ Done 2026-09-07, see "I3 steps 2–3 as built" —
rendering the same plan: a struct with the
fields plus `state`, a `next` whose body is `loop { match state { … } }`, and
a `close` running pending defers latest-first. Rust and Kotlin differ only
where they already differ (`while let` vs a guard, `Option<Box<…>>` for a
recursive pass slot).

**4. The representation split.** `once Iter<T>` *is* the state struct; plain
`Iter<T>` is the arguments plus a mint operation, so a second `for` re-runs
the producer. Boxing appears only at a meeting point (decision 5).

**5. Effects thread into `next`** (I4), removing [iter-effect-free] — except
`[Throw<M>]`, which stays rejected on a `yield` fn (user decision
2026-09-07).

**6. Delete** — *partly done 2026-09-07*: `SalvoGen`/`SalvoYield` are gone from
`runtime/iter.rs` and the Kotlin `iterator { … }` builder with them. Still to
go with the representation split: `SalvoIter` itself, the `'static`+`Clone` bounds and the
`Fn`-not-`FnMut` convention exception, then sweep std (`seq.sv` can go lazy)
and the ~48 tests and 10 snapshots that mention `Iter`/`yield`.

Restrictions worth shipping v1 with, reported rather than guessed: a `when`
containing a `yield` (the prototypes used `if`, and guessing the arm/binding
interaction is what the invariants forbid), and a `yield` inside a `defer`
body (already an error [defer-no-escape]).

### I3 step 1 as built (2026-09-07): the plan, and what the prototype forced

`crates/salvo-core/src/generator.rs` turns a `yield` fn body into a
`GeneratorPlan` [iter-generator], and the acceptance test is the one the plan
named: `tests/generator_tests.rs` reads the checked-in gnarly body (now
`crates/salvo-core/tests/fixtures/gnarly.sv`), renders the plan as text, and
compares it with the **eight states of the hand-written machine, in its
order** —
the machine an oracle had already vouched for. It passed on the first run,
which is the whole return on having prototyped: the numbering was predictable
enough to write down before the code ran. Negative-tested by disabling the
simplification pass (three tests fail, including the acceptance test).

Four things the prototype forced that the sketch did not have:

- **A branch's arm nests inside the state it is written in**, rather than
  `Branch { then_state, else_state }`. That is what reproduces the
  prototype's state count: `if col == 1 { continue }` is
  `if cond { state = 2; continue; }` *inside* state 2 with the rest of the
  body falling through after it, and no join state exists. The same step
  shape renders a loop head's exit test (`negate: true`). Where an arm
  suspends — or does not end in a jump — a join state is unavoidable and the
  plan allocates one; the predicate that decides is `stmt_inlinable`.
- **States are numbered at commit time, through labels.** Steps refer to
  labels while the body is walked and a label is bound to a number when its
  steps are complete, so states come out in the order their code is
  *written* rather than the order the walker needed to reserve them (a loop's
  exit is reserved before its body but written after it). Reading the
  emitted machine against the source is the reason to care.
- **A simplification pass earns two of the prototype's economies.** States
  whose only step is a `Goto` are collapsed and the unreachable are dropped,
  which is what makes the resume point of a `yield` at the *end* of a loop
  body be the loop head itself — the naive walk allocates a state there
  every time, and 3 of the 8 gnarly states would have been forwarding stubs.
  State 0 is exempt: it is the entry, and the machine starts there.
- **A `for`'s element binding is a field**, where `gnarly.rs` kept it a
  per-turn local. It can be live across a suspension
  (`for x in p { yield x; println(x) }`), and the emitted machine is the same
  either way; the plan takes the safe one uniformly rather than analysing
  liveness.

Beyond that: `ClosePass` joined the step set (the release path closes a nested
pass as well as running deferred blocks, in one reverse-declaration order), and
the plan carries its `close` sequence rather than leaving each emitter to
derive it.

The refusals are the v1 list plus three the implementation found, all naming
their remedy: a `yield` in a **value position** (`let x = if c { yield 1 }`),
a **destructuring** `let`/`for` binding in a suspending block, and a local
**shadowing** another local or a parameter — the body's locals become fields
of one struct, so two of a name would collide, and the emitters decide "field
or local" by name. Renaming behind the author's back is what
[backend-never-wrong] rules out; a loop with an `else` block is refused for a
different reason (the `else` runs only if the loop never ran, and the pass has
nowhere to record that).

### I3 steps 2–3 as built (2026-09-07): both emitters render the plan

The machine is emitted, the borrowed coroutine transforms are deleted, and the
two backends print the same bytes for a producer with `defer`s, an early
`return` and a nested producer (`{rustc,kotlinc}_compiles_and_runs_a_generator_with_defers`,
one source and one expected stdout).

What each backend does, in one line: Rust emits
`struct __Pass_f { <params>, <locals>, __state: u32, __d0: bool }` with
`__advance(&mut self) -> Option<T>` plus `impl Iterator`, and the fn body
becomes `SalvoIter::from_factory(Rc::new(move || Box::new(__Pass_f::new(…))))`
[rs-generator]; Kotlin emits
`private class __Pass_f(…) : SalvoPass<T>()` with
`override fun __advance(): Boolean`, and the fn body becomes
`return Iterable<T> { __Pass_f(args) }` [kt-generator]. `Iter<T>`'s
representation is untouched: it is still the factory, so a second `for` starts
from the beginning, and the *semantics* the laziness decision fixed do not move
at all. That ordering was deliberate — the plan's step 3 before its step 4 —
because it makes the representation split a change to types alone.

**Three things the emitters forced, none of them visible in the prototype:**

- **A field whose type has no zero value has to be a slot.** A pass field is
  initialized when the pass is *created*, while the body's own initializer
  belongs where it was written, so a hoisted local starts at its type's zero —
  and `map`'s `for x in it { yield f(x) }` has an element of type `T`, which
  has no zero. Both backends now split: a zero-able type gets a plain field
  (`i: i32 = 0`, `var i: Int = 0`), anything else an `Option`/nullable one
  where reads unwrap (`BindKind::SelfSlot` on Rust — reads clone out,
  `&`/`&mut` borrow through `as_ref`/`as_mut`, assignments re-wrap; `!!` on
  Kotlin). Uniform `Option` everywhere would have been simpler to explain and
  worse to read, and demanding a zero everywhere would have refused a generic
  producer.
- **Kotlin needed a runtime base class, and the reason is null.** `SalvoPass<T>`
  (`runtime/iter.kt`) turns "advance and report" into `hasNext`/`next` with one
  element of lookahead, holding the element as `Any?` — because `T` may itself
  be nullable, so absence cannot mean "the end". That is the same argument the
  protocol's `Emitted T | Finished` tagging rests on [iter-protocol], arriving
  from the other direction. Rust needs no such thing: `Option<Option<T>>` is
  unambiguous, so `__advance` *is* the `Iterator::next`.
- **Kotlin needed no name rewriting for reads**, only for `let`: a property is
  in scope in its own class's methods. Rust needs `self.` everywhere, which
  `BindKind::SelfField` — the handler-state binding kind — already did.

**One defect, found by migrating rather than by report**: a `return` inside a
**value-position** loop (`let last = while … { … return … }`) leaked into the
generated machine as a *target-language* `return`, because the plan's
"does this statement jump out of itself?" predicate did not look inside a
`let`'s value. On Kotlin that produced code kotlinc rejected (`return type
mismatch`); on Rust it would have returned from `__advance` with the wrong
type — [backend-never-wrong] either way. The predicate now descends into
`let`/assignment/`use` values, and the shape is *refused* with a message naming
the remedy (write the loop as a statement): flattening it properly would mean
the machine producing a loop's value from a state it jumped out of. The test
that covered it (`iterator_bare_return_in_value_loop_retargets`, which asserted
`return@iterator`) is now `iterator_bare_return_finishes_the_pass` over the
statement form, which is the feature it was really about.

Smaller notes: `StmtCtx::IteratorBody` is gone from both emitters (a `yield`
never reaches the statement emitter now — reaching it is an internal error);
`simplify` gained one more peephole (a state whose only step is `Finish`
forwards to the terminal state, so a fn-block end and a loop exit with nothing
after it are the same state); and `runtime/iter.rs` lost two thirds of its
lines. The Kotlin `defer`-as-`deferScope` idea from decision 6 is still
unbuilt — the machine's defers are flags, and a `defer` inside a *plain*
statement still uses the try/finally splice.



Decision 7 planned two lowerings — a readable direct form for bodies whose
yields sit in the tail of a single loop nest, with the flat machine as a
backstop. The prototype argues against it, and this is worth deciding before
I2c's second half is built:

- **The fast path's coverage is thinner than it looked.** `naturals`
  (`while true { yield copy(i); i = i + 1 }`) — the producer in the repo's own
  lazy-iterator demo — has statements *after* its yield, so it needs two
  states and does not qualify. Neither does anything containing a `break`, a
  `continue`, or a `defer`.
- **The flat machine is not much worse for simple bodies.** `range` becomes
  three states and reads fine; the dispatch loop is the same shape either way.
- **Two lowerings is two things to keep correct**, and the divergence risk is
  the one the laziness episode already taught.

Recommendation: emit the flat machine always. **Accepted by the user
2026-09-07**, superseding decision 7's second bullet: there is one lowering,
and the "simple-generator fast path" is not to be built.

### Hand-written iterators must say `once` (user decision 2026-09-07)

A hand-written state type — the point of decision 2, since `zip`/`merge`
read two sources and `yield` cannot express them — has to be drivable by
`for` like a generated one. It is **not** inferred: a value whose type has
a `next` but no `once` qualifier is an **error** at the driving site, and
the diagnostic names the remedy — annotate the return type `Once X`.

```
struct Zip<A, B> { ... }
fn next<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }

fn zip<A, B>(xs: Once Iter<A>, ys: Once Iter<B>) -> Zip<A, B> { ... }
for pair in zip(as, bs) { ... }   // ERROR: `Zip<A, B>` has a `next` but is
                                  // not a pass — return `Once Zip<A, B>`
```

Why an error rather than an inference: a struct with a `next` is not
self-evidently single-use — `next` says it can be advanced, `once` says
advancing it uses it up, and only the author knows whether the second is
true. Inferring `once` from the presence of `next` would attach an
obligation to someone's type on the strength of a name, and attaching
obligations silently is what [qual-*] keeps the compiler from doing. The
error is also the cheap half of the feature: it is a check at the driving
site plus a diagnostic, with the LSP surfacing the same message.

### Phases

- **I1** — ✅ Done 2026-09-07. Runtime modules extracted to real source files
  and the motivating program hand-verified on both backends; see "I1 as
  built" below.
- **I2** — Salvo-level protocol (`Emitted T | Finished`, `params Iterator`),
  `for` lowering to `while let`, simple-generator lowering. Effect-free
  producers only: pure simplification, `SalvoGen`/`SalvoYield`/`SalvoIter`
  deleted.
  - **I2a** ✅ Done 2026-09-07 — `once Iter<T>` is a real type and the
    factory/pass distinction is checked. See "I2a as built" below.
  - **I2b** ✅ Done 2026-09-07 — the protocol in std (`Emitted`/`Finished`,
    `params Iterator<St, T>`, `next`), `for` resolving `next` before `iter`,
    the "has a `next` but is not `once`" error, and `canbe once`. Emission
    is not part of it: both backends reject a `for` over a pass for now.
    See "I2b as built" below.
  - **I2c** — ✅ *two of three parts done*. The driving-loop emission
    (2026-09-07), so hand-written passes (`zip`, `merge`) run on both backends;
    the async machinery gone with I3; and **the release path plumbed**
    (2026-09-08) — see "I2c: the release path, everywhere" below. Remaining:
    the **un-boxing** half of the representation split — `once Iter<T>` *being*
    the generated struct and a factory keeping the arguments it re-mints from,
    so a `for` drives the machine directly instead of through a
    `Box<dyn SalvoPass<T>>`. With it go `SalvoIter` itself, the `'static` +
    `Clone` bounds, and the `Fn`-not-`FnMut` convention exception.
    **What it is actually worth is written up under "The un-boxing question,
    costed" — it is smaller than it looks, and it needs a decision.**
    Also unlocked by it: whether a producer returning `once Iter<T>`
    may take a **mutable parameter** after all. One pass exists, so the
    factory case's ambiguity is gone and what remains — the caller may not
    touch the collection while the pass lives — is what shared fate
    ([fate-link]) expresses. It needs a pass that *borrows*, i.e. the state
    machine itself rather than a boxed factory value. Raised by the user
    2026-09-08 with the decision that refused the factory case.
- **I3** — ✅ **Done 2026-09-07**: prototyped against an oracle (see "I3
  prototyped"), then the shared plan (`salvo-core/src/generator.rs`
  [iter-generator], accepted against the prototype's eight states — "I3 step 1
  as built"), then both emitters rendering it with the `async`/`iterator { … }`
  machinery deleted ("I3 steps 2–3 as built"). What is left of the `yield`
  half belongs to I2c (the representation split) and I4 (effects).
- **I4** — ✅ **Done 2026-09-07**, both halves. The **language surface**
  [iter-effects] — the claim on a producer type, the relocation to the return
  type, position rule 2, inverted variance, never-drop, inheritance, and
  "driving is what needs the handler"; [iter-effect-free] is gone as a rule.
  Then **emission**: a generated trait/interface per effect set
  ([rs-pass-effects] / [kt-pass-effects]), the handlers as parameters of
  `advance`/`close`/`__run_dN` in the checker's order, a `for` over a claiming
  producer lowered to mint/advance/close — which gives the injected `close` its
  first caller — and the variance adapter at every position the checker records
  a `Coercion::WidenProducer`. Verified by compiling and running
  `experiments/pull-iterators/effectful.sv` under both toolchains with identical
  stdout. See "I4 emission as built" for the two unknowns it had to answer and
  the four shapes still refused. I2c's representation split and I5 follow.
- **I5** — ✅ **Done 2026-09-08**: the combinator surface, to the user's
  decision — *eager by default, with two named variants*. See "I5 as built"
  below for the four defects it uncovered.
  - **`map` / `filter` stay eager**, returning `Mut List<U>`. The default is
    the one that surprises least, and chaining already works because a list is
    iterable.
  - **`map_lazy` (and `filter_lazy`) are the lazy pair**, returning `Iter<U>` —
    now that a producer may perform effects [iter-effects], a stored callback
    is no longer a reason they cannot exist. Laziness is *asked for* rather
    than inherited, which also keeps the "how many times was it consumed"
    question visible at the call site: `[iter-mut-param]` refuses a callback
    carrying mutable state precisely because a lazy combinator calls it once
    per element in every pass.
  - **`map_to` maps into a collection the caller provides**, given as the
    **first argument**, with an **`?add` implicit parameter** for appending to
    it [implicit-param]. So the destination need not be a `List` — anything
    with an `add` qualifies, which is the same "a bundle of implicit
    parameters, not a trait" move `params Iterable` already makes.
  - Open when it is built: whether `reduce` needs variants at all (it is
    already a fold to one value), and what `filter_to`/`map_to`'s deduction
    lists say about the destination — it is mutated, so `[dest: Mut]`, and
    `[iter-mut-param]` does *not* apply because `map_to` is not a producer:
    it returns when it is done.
- **I6** — Sweep: ~48 test fns and 10 of 23 insta snapshots mention
  `Iter`/`yield`; `[fn-iterator]`, `[iter-effect-free]`, `[rs-iter-lazy]`,
  `[seq-iterable]` rewritten; the LANGUAGE.md planned-change subsection
  folded into the section proper.

### I1 as built (2026-09-07)

**I1a — the runtime modules are source files now.** The four *static*
generated modules moved out of Rust string literals in the emitters into
`crates/salvo-backend-rust/runtime/{iter.rs,strings.rs,seq.rs}` and
`crates/salvo-backend-kotlin/runtime/throw.kt`, pulled in with
`include_str!`. The parameterized generators stay generated — `unions.rs` /
`unions.kt` are a function of the arities a program needs, so there is no
static text to extract.

- **Extracted byte-for-byte, deliberately**: the files were captured from
  the compiler's own output (one scratch program touching all four),
  so emitted bytes did not change, no insta snapshot moved, and no e2e
  content stamp missed. Verified by compiling that program before and
  after and diffing the whole output tree — identical on both backends.
- **New tests** `crates/salvo-backend-{rust,kotlin}/tests/runtime_tests.rs`
  compile each module *on its own* (`rustc --crate-type lib`, `kotlinc`)
  and assert the toolchain said nothing at all: this code is spliced into
  user output, where a warning is noise the user cannot fix. They skip
  without the toolchain like every other e2e test, and they are
  content-cached the same way.
- **Negative-tested**, since a test that cannot fail is decoration: an
  unused local added to `seq.rs` made
  `every_runtime_module_compiles_warning_free` fail with the warning
  quoted, and reverting restored green.
- The list of modules is a `const` in each test, so a new runtime module
  that is not registered is a visible omission rather than an untested
  file.

**I1b — the design is hand-verified end to end.** The prototype lives in
`experiments/pull-iterators/` (with a README recording what it proves and
how to run it), since `tmp/` is scratch and these are evidence.
`lines.sv` is the motivating program, written in the *planned* language: an
effectful iterator function (`[FileSystem, Console]`) holding a resource,
releasing it with a `defer` **that itself performs effects**, with two
distinct resume points (a header `yield`, then a `yield` inside
`while true`), consumed by a `for` that `break`s after three elements.
`lines.rs` and `lines.kt` are what the emitters would produce. Both compile
warning-free and print **byte-identical stdout**:

```
opening data.txt
line -- data.txt --
line alpha
line beta
closing data.txt
done
```

What that establishes, in order of how much it was in doubt:

- **`close` can be idempotent, and that collapses the injection.** Guard
  the unwind path on the per-site `defer` flags and *one* call after the
  loop covers `break` and exhaustion alike, because both land there — no
  per-exit-path duplication, and no double-run. Verified with a second
  variant that drains instead of breaking: exactly one `closing` line,
  identical on both backends. Only `return`/`throw` out of the loop body
  need their own splice, which is machinery `defer` already has.
- **Effects thread in cleanly, and that is the whole design.** `next(&mut
  self, fs: &mut dyn FileSystem, console: &mut dyn Console)` holds no
  handler, so there is no lifetime, no `'static` bound, and nothing
  captured — which is exactly what [iter-effect-free] existed to avoid.
  A deferred block performing effects on the close path works for the same
  reason, and it is why a Rust `Drop` impl could never have been the
  mechanism (it takes no parameters).
- **The Rust output contains no `Pin`, `Future`, `Waker`,
  `Box<dyn Iterator>`, `Rc`, or `async`.** The state machine is a struct
  with a `u32` state, the body's locals as fields, and one `bool` per
  `defer` site.
- **The consumer lowering is a `while let`** on Rust
  (`while let SalvoStep::Next(l) = pass.next(fs, console)`) and the
  obvious `while (true) { … if (step !is Next) break }` on Kotlin.
- **The two backends can share one lowering** (decision 6) with the JVM
  losing nothing: Kotlin's `iterator { … }` builder could only ever have
  *captured* the handlers, so the uniform machine is not a concession on
  that side, it is the only shape that threads them.

Not yet prototyped, and still the riskiest thing in the plan: a body where
`defer`, `when`, labelled loops and a `Throw` transfer all have to be
resumable at once. I1's program has one `defer` site and one loop; **I3
should open with the gnarly shape** (`rangeIncl` with a `defer` inside a
nested `for`) before the general lowering is written.

### I5 as built (2026-09-08): the combinator surface, and what it uncovered

The surface is the user's decision, in `std/core/seq.sv`:

- `map` / `filter` / `reduce` — **eager**, unchanged, returning `Mut List<U>`.
- `map_lazy` / `filter_lazy` — return `Iter<U>`. Laziness is *asked for*, which
  also keeps "how many times was this consumed?" visible at the call site.
- `map_to` / `filter_to` — the destination is the **first argument**, and
  appending goes through an **`?add` implicit parameter**
  (`(dest: Mut D, elem: U) -> None => dest: Mut`), so the destination is
  anything with an `add` rather than a `List` — the same "a function, not a
  trait" move `?Iterable` makes for the subject. Nothing is returned: the
  caller already holds the destination.

Verified on both backends with one source and one stdout, plus a second program
that is the point of laziness: `map_lazy(filter_lazy(naturals(), is_even),
triple)` over an **unbounded** producer, terminating only because the consumer
breaks. Eager `map` would not return.

**Four defects, and only one of them was I5's own.** The cause of the cluster is
worth recording: `map`/`filter`/`reduce` over a list take their `List` fast
path [fn-overload-rank], so the **generic `?Iterable` body had never been
emitted** for a real call. `map_to` has no fast path, and `map_lazy` is the
first *producer* with implicits — so several long-standing paths ran for the
first time at once.

1. **A producer's implicits were not pass fields.** The body called `iter(xs)`
   inside the machine, where the implicit is not in scope ("unresolved
   reference"/E0425 on both backends). Fixed in the *plan*
   (`FieldKind::Implicit`, `plan_generator_with_implicits`) rather than twice in
   the emitters, since "the body's locals become fields" is a plan concept.
   Rust holds them `Rc`-shared and calls them through `self`; Kotlin makes them
   constructor properties.
2. **An implicit with a `Mut` parameter lost its `&mut`** on Rust:
   `implicit_param_type` rendered parameter *types*, and `Mut` erases there, so
   `?add` became `FnMut(Vec<i32>, i32)` and the adapter could not append to
   what it was handed (E0596). Now the fn type's **contract** decides, as it
   already did for a written `Type::Fn`. Deliberately narrowed to kept-`Mut`
   positions: making every implicit borrow would be more uniform and would
   touch every existing `?Iterable`/`?cmp` call site.
3. **A generic call filling implicits could not be inferred.** Each implicit
   arrives as an adapter *closure* whose parameter types Rust takes from the
   callee's bound — so with the callee's generics open there is nothing to infer
   them from, and inference stalls on closures waiting for the answer (E0282).
   The call now states the instantiation from `Checked::call_type_args`
   [rs-implicit-turbofish]. Also `rc_fn_type` had a latent chained-`unwrap_or`
   bug that undid its own prefix strip, emitting `Rc<dyn impl Fn(..)>`.
4. **`return None` in a `None`-returning fn was wrong on *both* backends** —
   `return null` against Kotlin's `Unit`, `return None;` against Rust's `()`,
   each a target-language type error. The same defect in both, found by the same
   std line, and neither is iterator-related: any program writing an explicit
   `return None` hit it. The test asks the **fn**, not the returned value: in a
   `Str?`/`Option`-returning fn `return None` is exactly right. The literal is
   dropped rather than evaluated, since emitting `null` as a statement warns
   ("expression is unused") in code the user cannot edit.

**`map_to` returns its destination** (user decision 2026-09-08, taken right
after the surface landed), so a chain carries on from it:

```
let out = map_to(mutable_list<Int>(), xs, double)
let kept = filter_to(map_to(mutable_list<Int>(), xs, double), xs, is_even)
```

The destination is therefore **moved in and handed back** (`-> Mut D => xs, f`,
with `dest` absent from the deduction list) rather than kept — which is what
makes the nested form above legal, since a kept parameter could not be the
value of the enclosing expression. Holding a destination across the call means
rebinding it (`let sink = map_to(sink, …)`), which is the ordinary move
discipline.

One emitter fix came with it: an owned parameter binds `mut` when its **type**
says `Mut`, not only when the body reassigns it. A moved-in `Mut D` handed to a
`Mut` position needs `&mut dest`, which a non-`mut` binder refuses (E0596), and
passing it on is not an assignment so `collect_mutated` could not see it. `Mut`
in the type *is* the claim that the value may be mutated through this binding,
so the binder now follows the type.

**Still open on the surface**: whether `reduce` wants variants at all (it
already folds to one value).

### The un-boxing question, costed (2026-09-08)

What I2c's remaining half would buy, and where it stops. The example is one
program, and every line of Rust below is the emitter's actual output today:

```
struct Feed { name: Str, items: Iter<Int> }

fn naturals() -> Iter<Int> { let i = 0  while true { yield copy(i)  i = i + 1 } }
fn evens(it: Iter<Int>) -> Iter<Int> => it { for x in it { if x % 2 == 0 { yield copy(x) } } }
fn total(xs: Iter<Int>, limit: Int) -> Int => xs { … for v in xs { … } … }
```

**Today** — three kinds of indirection, all of them from one decision (`Iter<T>`
is one type whatever produced it):

```rust
pub struct Feed { pub name: String, pub items: SalvoIter<i32> }

pub fn naturals() -> SalvoIter<i32> {
    SalvoIter::from_factory(std::rc::Rc::new(move || Box::new(__Pass_naturals::new())))
}
pub fn evens(it: &SalvoIter<i32>) -> SalvoIter<i32> { … }
pub fn total(xs: &SalvoIter<i32>, limit: i32) -> i32 { … }

struct __Pass_evens { it: SalvoIter<i32>, x__pass: Option<Box<dyn SalvoPass<i32>>>, … }
//   SalvoIter<T> = Rc<dyn Fn() -> Box<dyn SalvoPass<T>>>
```

**Un-boxed**, each producer's factory and pass would be its own named struct.
`naturals()` returns `__Factory_naturals` and mints a `__Pass_naturals` — no
`Rc`, no `Box`, no `dyn`, one less allocation per loop. That is the whole prize,
and it is real but narrow. What it costs, in the same program:

- **`evens` has to become generic in its input.** Its pass holds the inner one
  *by value*, so the type is `__Pass_evens<P>` where `P` is whatever
  `naturals()` returned — and the parameter is `impl SalvoFactory<i32>` rather
  than a named type. The type parameter then propagates to anything holding an
  `evens(...)`, so composition (`map_lazy`, `filter_lazy`, `zip`, `merge`, every
  combinator) cascades type parameters through the chain.
- **`Feed.items` cannot be un-boxed at all.** Its type is written `Iter<Int>`
  and its value is `evens(naturals())`, whose un-boxed type is
  `__Factory_evens<__Factory_naturals>` — a name the *Salvo* declaration has no
  way to say. A struct field, a union arm and a `List<Iter<Int>>` element are
  all in this position: they must erase, i.e. stay `Box<dyn …>`.
- **`total`'s parameter** is the choice that has to be made: generic
  (`impl SalvoFactory<i32>`, monomorphised per call — fast, but every producer
  that *stores* one grows a parameter, see above) or boxed (`&dyn` — no
  cascade, no saving). Decision 5 said "boxing at the meeting point"; this is
  the meeting point, and which of the two it gets is a language-visible choice,
  because the generic form changes what can be written where.

So the honest summary: un-boxing removes an `Rc` and a `Box` where a producer
is **built and driven without being stored or passed** — a `for` over
`naturals()`, which is the common shape in small code — and keeps them
everywhere the value crosses a declaration a user wrote. It is not a
correctness change: nothing about it is observable except allocation counts.

**What is genuinely gated behind it** is the other half: a pass that *is* a
struct can hold a **borrow**, which is what would let a `once Iter<T>` take a
mutable parameter under shared fate [fate-link] — the question raised with the
[iter-mut-param] decision. That is a language capability, not an optimisation,
and it is the reason to do the work.

**Recommendation**: treat the un-boxing as a *means* to the `once`-borrowing
feature rather than as a performance item, and decide the parameter question
(generic vs boxed) before starting — because the answer decides whether the
cascade exists at all.

### I2c: the release path, everywhere (2026-09-08)


The first part of the representation split, done on its own because it is what
the rest is built on: **a generated pass stops being a target-language
iterator**, and every `for` over an `Iter<T>` — claiming or pure — mints,
advances and closes.

- **Rust**: `runtime/iter.rs` gains `trait SalvoPass<T> { fn advance(&mut self)
  -> Option<T>; fn close(&mut self) {} }` and `SalvoWalk<I>` (a walk over
  elements that already exist, whose default `close` is the honest no-op).
  `SalvoIter<T>`'s payload becomes `Rc<dyn Fn() -> Box<dyn SalvoPass<T>>>` with
  a `mint()`, and `SalvoPassIter<T>` adapts a pass *to* a Rust iterator for the
  positions that want one (`to_vec`, `IntoIterator`). A generated pass now
  `impl SalvoPass<T>` instead of `impl Iterator`.
- **Kotlin**: `runtime/iter.kt` gains `interface SalvoClosable { fun
  __close() }`, which `SalvoPass<T>` implements, so a drive site can release
  the pass it is holding without knowing which producer wrote it — a
  collection's iterator is an `Iterator` too and has nothing to release, which
  is what the `is SalvoClosable` test asks.
- **Nested passes are released too**: the plan's `ClosePass` step closes the
  slot before clearing it, so an inner producer abandoned along with its outer
  one runs its deferred blocks. It was a plain `= None` / `= null` before.
- **Collections keep native loops** (decision 8): the gate is the subject's
  type being `Iter<T>`, so `for x in xs` over a list, an array or a `Str` emits
  exactly what it did.

**What it does *not* buy, and this is worth being exact about: no observable
behaviour change today.** The hole it closes is that an abandoned *pure*
producer skipped its deferred blocks — and after [iter-mut-param] a pure
producer has no way to make that observable: its `defer` can only touch its own
locals, which die with the pass, and a `defer` that performs an effect makes the
producer *claiming*, which I4 already handled. Every route to observing it runs
through a shape that is currently refused (a claiming producer nested in
another, the value-position `for`). So this is plumbing that is *correct in
advance* of the cases that will need it, plus the protocol the un-boxing half
requires — not a bug fix, and it should not be recorded as one.

**Two things it cost, both small and both instructive:**

- **Rust: `mint()` already boxes.** `Box::new(x.mint())` is a
  `Box<Box<dyn SalvoPass<T>>>`, which does not implement `SalvoPass<T>` —
  E0277 at the nested-pass slot. Caught immediately by the e2e tests.
- **Kotlin refuses to smart-cast a mutable property.** `if (x__pass is
  SalvoClosable) x__pass.__close()` on a slot is *"smart cast is impossible,
  because it is a mutable property that could be mutated concurrently"* — the
  trap already in the gotchas from the field-narrowing work, arriving from a new
  direction. The slot is bound to a local first.

**The shape tests that changed** are the interesting record of the diff:
`a_for_loop_borrows_an_iter_subject` asserted `for mut v in &twice` and now
asserts `twice.mint()` + `.advance()` + `close()`; the state-machine test
asserted `impl Iterator for __Pass_naturals` and now asserts
`impl SalvoPass<i32>`; and both backends' loop-numbering assertions shifted by
one, because driving a producer costs a fresh loop name for the pass.

### I2a as built (2026-09-07): the factory/pass distinction ships first


`once Iter<T>` is now a type the compiler accepts, and the one-shot rule is
enforced — *before* the representation splits. That order is deliberate:
the semantics are the part the user decided, they are checkable today, and
landing them first makes I2c's representation change a non-event
semantically (nothing legal before it becomes illegal after).

What it took was small, because `once` was already the right shape:

- **`types::once_position`** is the single predicate for where `once` may
  be written — `Ty::Fn` or `Iter<T>` — read by *both* the checker's
  position check and `is_subtype`'s inverted rule, so the two cannot drift
  as D6 widens the list.
- **The position check** (check.rs, `once` branch of the qualifier
  validation) now consults it, and its message names both positions:
  ``` `once` applies to function types and `Iter<T>`, not `Int` ```.
- **The inverted subtyping rule** lost its `Ty::Fn` gate: `(_, Qualified
  { Once, base })` with `once_position(base)`, so plain `Iter<T>` <:
  `once Iter<T>` exactly as plain `(A) -> B` <: `once (A) -> B`. The
  never-drop direction needed nothing — `qual_drop_block` was already
  type-agnostic.
- **The one new rule**: a `for` whose subject type carries `once` calls
  `fate_move(iterable, "iterate", "a `for` loop", …)` and gives the loop
  binding *no* links — the elements are owned by the loop rather than
  derived from a subject that is still alive [fate-link]. A factory keeps
  the existing behaviour (links, no consumption). The diagnostic falls out
  of the existing machinery: "`p` cannot be used here: it was consumed
  (moved) by a `for` loop".
- **The emitters needed no change at all**: `once` erases
  [qual-erasure], and `iter_elem_ty` already went through `strip_quals`.
  Both backends compile and run a pass-returning producer, verified with
  one shared program and one shared expected stdout
  (`{rustc,kotlinc}_compiles_and_runs_a_once_iterator`,
  `pass 6 / factory 6 6 / total 6 / total 6`).

Tests: `crates/salvo-core/tests/once_tests.rs` (10) covers the position
list, both variance directions, driving a pass once, driving it twice
(consumed-use error), driving a factory twice (fine), and two calls giving
two passes; plus the two e2e tests above.

**Two diagnostic leftovers**, both quality rather than correctness:

- A pass in a factory position reports "no matching overload for
  `twice(Once Iter<Int>)`" rather than saying `once` never drops. The
  `qual_drop_block` message exists and is used for `^` widening; the
  overload-failure path does not reach for it. This will be a common
  mistake, so it is worth a near-miss hint.
- An invalid `once` position cascades: `let bad: Once Int = 3` reports the
  position error *and* "expected `Once Int`, found `Int`", because the
  rejected qualifier stays on the lowered type. One mistake should be one
  diagnostic — the fix is to fall back to the base type when the position
  check fails.

### Costs recorded up front

- **Recursive producers need boxing on Rust.** A nested `for` keeps the
  inner pass alive across the outer's suspensions, so it becomes a
  *field*; for a recursive producer that field has the struct's own type
  (`E0072`), hence `Option<Box<…>>`. One allocation per level per pass and
  O(depth) per element — the cost profile of chained `flatten`. Kotlin is
  unaffected. This is the one durable advantage push kept.
- **The flat machine is the one lowering whose output stops resembling its
  input**, and the interaction to distrust is `defer` + `when` + labelled
  loops + `Throw` transfers all having to be resumable in the same body.
  Hand-prototype the gnarly shape (`rangeIncl` with a `defer` inside a
  nested `for`) before committing.
- **`yield x` stops being a move.** Today it consumes ([deduce-consume],
  "a yield in a loop consumes anew every iteration"); under a pass the
  element is handed over per `next`, so the loop binding becomes a
  kept-or-moved decision the deduction engine has to make. A class of
  current errors disappears; sizing the analysis that replaces it is the
  one item that could not be bounded from reading the code.

## Roadmap: place-based flow analysis

A second flow-analysis arc, independent of linearity. Flow facts are keyed
by *place* — a local root plus a projection path (`h`, `h.field`,
`h.a.b`) — rather than by variable name. The place facts live inside the
root's `LocalVar`, so `snapshot_narrows`/`restore_narrows`/
`merge_fallthrough` remain the single source of truth (the S1 gotcha) and
an event on a root reaches everything below it.

### P1 — Field smart-casting. ✅ Done 2026-09-03

`is` narrows places, so a checked field or tuple element reads at its
narrowed type ([flow-place]), and invalidation ([flow-place-invalidate])
rides on the event set the fate analysis already watches. Landed:
`place.rs` (the `Place`/`proj` types and the prefix/overlap relations),
place-keyed narrowing through the branch machinery, narrowed-read
unwrapping in both emitters, tuple element access as new syntax
([expr-tuple-index]), and — from the Kotlin parity bug it surfaced —
[kt-narrow-field-assert]. Decisions P1a/P1b/P2 are in the decision log.

Deliberately out: array elements (an unknown index may alias any element,
so they are tested and bound but never narrowed) and `when` on field
subjects (P2, decided against — `when` stays variable-only).

**Tuple element access** ([expr-tuple-index]) was added in the same
session to close P1a: `t.0` is a `proj::Index` place, so it narrows,
invalidates and merges like a field.

### L5 — field-disjoint ownership

**Open** — see ROADMAP.md; it rides on this substrate.

## Roadmap: deductions and qualifier reasoning

Motivated by a confirmed unsoundness (see "Remaining leftovers"): the
removal-set rule (`removal = declared − kept`) assumes a function can only
invalidate qualifiers it *declares*, which is false for any function that
mutates. Fixing it needs a way to say "and nothing else survives", which a
delta-only model cannot express.

### D1 — Exhaustive and delta qualifier deductions. ✅ Done 2026-09-02

Keep the `:` syntax; give the qualifier list a *polarity* per entry:

| Form | Meaning |
|---|---|
| `[list]` | keep the parameter, all qualifiers preserved (unchanged) |
| `[list:]` | keep it, strip every qualifier (unchanged) |
| `[list: NonEmpty Mut]` | **exhaustive**: afterwards *only* these apply |
| `[list: -NonEmpty]` | **delta**: drop `NonEmpty`, everything else preserved |
| `[list: Nothing]` | moved (equivalently: omit the entry) |
| `[]` | no promises about any parameter — everything moved |
| *(absent)* | inferred [deduce-infer] |

`+Q` (add a qualifier) is **not part of the language** — it is an assert,
see D2 (user decision 2026-09-02). The parser recognizes the `+` only to
report "adding qualifiers in a deduction (`+Qual`) is not supported yet"
instead of a bare parse error.

The plain (exhaustive) form is what closes the hole: `clear`'s
`[list: Mut]` now drops a caller's `NonEmpty` because it was not listed.

- **Soundness rule (the load-bearing part).** For a parameter the body
  *mutates*, only the exhaustive form (or `[list:]`) may be written:
  keep-all `[list]` and `-` deltas both claim "everything else survives",
  which is exactly the unsound claim. Inference emits the declared set
  for mutated parameters and keep-all for read-only ones — so pure reads
  still never strip a caller's qualifiers, which is the property the
  original removal-set rule existed to protect.
- **Is mutation the only invalidating operation?** (user question
  2026-09-02) Within the current language, yes — and it follows from the
  model rather than being a stipulation. Everything a callee can do to a
  *kept* parameter is: read it (projections, interpolation, passing it on
  to other readers) — observably state-preserving, so no predicate can
  break; mutate it — the invalidating case; or move it — after which the
  caller has no access, so preservation is moot. There is no way for a
  kept value to escape observably (fate links do not outlive the call;
  `use` constructor arguments are moves). So "mutation forces the
  exhaustive form" is the complete rule for today's language, and a new
  invalidating operation could only arrive with a new capability
  (out-parameters, escaping references).
- **But the *granularity* is wrong, and that is worth marking on
  qualifiers.** Qualifiers split into two kinds:
  * **State predicates** (`NonEmpty`, `Sorted`, `Validated`): claims
    about content. Mutation may falsify them.
  * **Capabilities** (`Mut`, and the usage disciplines `Linear`,
    `once`): claims about what the *holder* may do. Mutation cannot
    falsify "you may mutate this".
  Only state predicates need dropping when a body mutates; capabilities
  survive trivially. Today this distinction is **degenerate**: every
  capability qualifier is built into the language and every *user*
  qualifier is a state predicate (or provenance, which behaves like one —
  `Validated` is falsified by mutation as surely as `NonEmpty`). So D1
  hard-codes the built-ins and does **not** add a marker; revisit if a
  user ever needs to declare a capability qualifier.
  * Consequence for D1's syntax: `Mut` is written explicitly in
    exhaustive lists (`[list: Mut]`), and `[list:]` keeps stripping
    everything including `Mut`, exactly as today. The alternative
    (auto-preserving capabilities) would silently change `[list:]`'s
    meaning for no present benefit.
- **Accepted cost (user decision 2026-09-02): over-strictness.** A
  mutating function cannot promise to preserve a qualifier it does not
  declare, so `add(list: Mut List<T>, elem: T)` drops a caller's
  `NonEmpty` even though appending cannot empty a list. Remedy is a
  re-test (`if list is NonEmpty`); the general answer is D3, **built
  2026-09-06** as `refn` [qual-refn] — the claim's owner states what the
  call does to it, since the function cannot.
- **Mixing polarities is an error.** An entry is either all-plain
  (exhaustive) or all-delta (`-`, and later `+`): `[list: Mut -NonEmpty]`
  is redundant under the exhaustive reading and contradictory otherwise.
- **`-Q` may name a qualifier the parameter does not declare** (a
  function that knows it invalidates a specific property). It is a
  convenience only — the exhaustive form remains the sound default, and
  inference never relies on `-`.

**Implementation notes (2026-09-02).** `QualEffect { KeepAll,
Exhaustive(Vec<String>), Remove(Vec<String>) }` lives in `types.rs` and is
carried by both `ParamDeduction` and `FnParamContract`; all three forms
reduce to one operation, `removal_set(have)`, computed against the
qualifiers the *argument* carries (`have`) — that is where the soundness
comes from. Mutation is tracked like move-mode claims: `fate_mutation`
records the enclosing fn's parameter in a new `param_mutations` map (for
*written* lists too — that is what the validation reads), threaded through
`check_once` and the fixpoint's convergence check. Inference gained
`restrict_to` for the contagion rule, and the lattice `KeepAll` →
`Remove` (growing) → `Exhaustive` (shrinking) is what keeps the fixpoint
terminating; `bodyless_mutations` supplies the `Mut`-parameter proxy for
externals. Exactly one test had to change meaning:
`undeclared_qualifiers_pass_through_calls` *encoded* the unsoundness, and
is now `exhaustive_lists_drop_undeclared_qualifiers`, with
`delta_lists_pass_undeclared_qualifiers_through` covering the opt-in.
- **Type in the entry.** An entry's items are qualifiers plus *at most
  one* type — structurally identical to an `is` check pattern
  (`CheckPat { quals, base }`), so the parser and resolver can share that
  path: uppercase idents parse as type refs, and qualifier-vs-type is
  settled by resolution.
  * `[list: Nothing]` is sound *because `Nothing` is uninhabited*: it
    asserts nothing about runtime content, it only withdraws use. That is
    why "moved" falls out of the same rule rather than being a special
    case.
  * **Type narrowing beyond `Nothing`: its own step (user decision
    2026-09-02, following the recommendation).** `[x: Str]` on a `Str?`
    parameter is a *content* claim, so the callee must make it true.
    Out-parameters would do it, but Salvo has none (reassigning a
    parameter is a rebind, invisible to the caller). The implementable
    reading is a **verified postcondition**: "if this call returns
    normally, the parameter is a `Str`" — checked for Salvo bodies by
    requiring the promised narrowed type at every normal exit (the
    machinery `check_linear_exit` already walks), trusted for externals.
    D1 lands with `Nothing` only; this follows as **D1b**.

### D3 — Refinements ✅ Done 2026-09-06

D1's over-strictness had two candidate general answers. Analysis
2026-09-02 said they are *not* both needed, and the more obvious one does
not work:

- **Polymorphism — "preserves whatever predicates it received" — is
  unsound as a blanket promise.** Different predicates react differently
  to the *same* mutation: `add(list, elem)` preserves `NonEmpty` but can
  break `Sorted`. So a mutating function cannot honestly promise to
  preserve an unknown set. Restricting it to *non*-mutating functions
  makes it vacuous (keep-all `[list]` already preserves everything
  there). The only sound version is an explicit per-qualifier set, which
  is refinements by another name.
- **Refinements — a qualifier annotates existing functions with
  additional deductions, without changing their implementation.** This is
  the same information as "which operations invalidate me", stated from
  the qualifier's side, and it is per-qualifier-per-function, which is the
  granularity soundness actually requires. It also inverts the dependency
  in the useful direction: a user's `NonEmpty` can declare that std's
  `add` preserves it, without std knowing the qualifier exists.

**Built 2026-09-06 as `refn`** — see the decision-log entry at the top for
the six user decisions and the two things implementation revised. Every
open question this section recorded is answered:

- *Where a refinement may be declared*: in the qualifier that owns the
  claim (in scope wherever the qualifier is), or as a **top-level `refn`**,
  module-scoped and not importable [qual-refn-scope].
- *Trusted or checked*: **trusted**, like `-> T as Q` — no runtime check is
  emitted where it applies [qual-refn].
- *How refinements from several qualifiers on one function compose*: they
  merge; when they **disagree none of them apply**, with a warning
  [qual-refn-conflict], and a top-level `refn` **replaces** them for the
  parameters it names [qual-refn-reconcile].
- *Whether a refinement can strengthen a contract for callers who do not
  import the qualifier*: **no.** Visibility is per file, and a fn can only
  re-promise a qualifier its own signature names [qual-refn-infer].

Rules: [qual-refn], [qual-refn-match], [qual-refn-scope],
[qual-refn-conflict], [qual-refn-reconcile], [qual-refn-infer],
[qual-refn-docs].

### D2, D4, D6, D7

**Open** — see ROADMAP.md: qualifier asserts (`+Q`), predicate `is` on union
subjects, `once` on any type, and qualifier-conditional linearity.

## Roadmap: names and namespacing

### N1 — Dot-names for structs and qualifiers ✅ Done 2026-09-03

Motivated by the wrapper/newtype pattern (see D5): the Kotlin habit of
nesting `Id` inside `Environment` so signatures can demand
`Environment.Id`, without Salvo adopting nested *declarations* — the
user prefers flat code, so the namespace is in the *name*, not the
layout.

Proposed surface: a struct or qualifier may be declared as
`<ns>.<name>`, where `<ns>` names a struct in the same file.
`<ns>` must be a struct in that file, and no name in that file may be
`<ns><name>` when a dot-name with that `<ns>` exists (the Rust
flattening would collide). Backends: Kotlin nests the type inside
`<ns>`'s class; Rust concatenates (`<ns><name>`), since Rust modules and
structs do not relate the way Kotlin classes do. Qualifiers are erased,
so for them the dot is compile-time only on both backends — which is why
qualifiers are the easy half.

Import disambiguation rests on a new casing rule: modules always start
lowercase; types, structs and qualifiers always start uppercase. That
turns `import a.b.Environment.Id` into a decidable split (leading
lowercase segments are the module path, trailing uppercase segments the
item name) where today the split is purely positional — `resolve_import`
takes the *last* segment as the item name and everything before it as
the module prefix.

Decided in review (user decisions 2026-09-03):

- **N1a — the casing rule covers values too.** An uppercase-initial
  *head* identifier starts a *type path*; variables, parameters, fields
  and fn names start lowercase. Needed because in expression position
  `Environment.Id { value: "x" }` is otherwise indistinguishable from a
  field access on a variable named `Environment` (or a dot-notation
  call). Side benefit: `parse_is_check` currently guesses "lowercase
  means a binding" by convention (the `is Str surname` form) — the rule
  makes that principled.
- **N1b — uppercase module segments are a compile-time error, reported
  at source discovery.** Module paths *are* file paths [mod-file], so
  the rule reaches into the filesystem: `src/Utils.sv` stops being a
  legal program, and the diagnostic must name the file rather than
  surfacing later as a baffling unresolved import.
- **N1c — importing `Ns` brings `Ns.X` into scope.** Precedent:
  importing an *effect* brings its members (`ModuleScope` maps members
  to the owning effect). Dot-names carry their namespace at every use
  site, so auto-importing them cannot introduce an ambiguity — and
  without it every newtype costs an import line.
- The **collision ban is module-wide**, not file-wide: a module is all
  of its files, including backend define files (`list.sv` +
  `list.kotlin.sv`) [mod-visibility].
- The **collision ban also covers mangled names and imported names.**
  Overload mangling embeds qualifier names (`full_name__Surname`
  [kt-qual-mangling] [rs-fn-mangling]), so a dot-name must canonicalize
  its dot, and the resulting suffix can collide with a plain qualifier
  of the concatenated name imported from another module — which a
  file-scoped ban never sees. No encoding escapes this (Rust identifiers
  are `[A-Za-z0-9_]`, so every encoding is also a legal name), which is
  why a ban is the right mechanism; it just has to be scope-wide and
  applied to mangled forms.
- **Kotlin emits a *nested* class, never `inner`.** An `inner class`
  captures an outer instance and cannot be constructed without one.
  Consequence: a nested class of a *generic* struct cannot reference the
  outer type parameters (only `inner` can), so either `<ns>` must be
  non-generic or the member may not mention the outer generics.
- **Rust concatenates (`<ns><name>`) — the nested-module alternative is
  rejected.** Rust puts modules and structs in one *type namespace*, so
  `pub mod Environment` collides with `pub struct Environment`
  (E0428, verified with rustc: "`Environment` must be defined only once
  in the type namespace of this module") — and since `<ns>` is required
  to be a struct in the same file, that collision is guaranteed, not
  incidental. A mangled module (`Environment__ns::Id`) compiles but
  reads worse than the flat name; a lowercase module (`environment::Id`)
  also compiles but impersonates a Salvo module and adds a collision
  surface against real ones; Rust allows no item declarations in `impl`
  blocks and inherent associated types are unstable. Flattening is
  therefore the only clean rendering, which is what makes the collision
  ban load-bearing rather than a nuisance.
- Minor, but pin them: names cap at two segments (no `A.B.C`, and a
  dot-named struct may not itself be an `<ns>`); and `ty_base_name` /
  `type_base_name` plus the string-keyed define-template environments
  must agree on the canonical spelling of a dot-name — the documented
  "touch one, touch all three" trap.

Independent of D5: dot-names work the same for state qualifiers, and
apply to structs too, so this is a naming feature rather than part of the
subject axis.

What landed (rules [name-casing] [name-dot] [name-dot-import]
[kt-nested-dot-name]):

- **Parser**: `ident_decl_dotted` for struct/qualifier declarations and
  `type_ref_name` for every type position (annotations, `of` types, `is`
  checks, `as Q`, `canbe`, deduction lists, struct literals). A dot is
  only taken when *both* segments are uppercase, so `person.name` and
  `list.size()` are untouched. The name is carried as one dotted string —
  `Ident { name: "Environment.Id" }` — which is what keeps scope keys,
  checker types and the emitters' `ty_base_name`/`type_base_name`
  conventions in agreement without a second representation.
- **Casing rule**: `ident_type` / `ident_value` at every declaration site
  (structs, qualifiers, types, effects, handlers, generic parameters;
  fns, parameters, fields, `let`/`for`/destructuring bindings, lambda
  parameters). No existing Salvo source violated it — std and the
  corpora were already consistent.
- **Resolve**: casing-aware import splitting (trailing uppercase segments
  are the item, so `import env.types.Environment.Id` works while
  `import core.list.size` keeps its old positional meaning); an unaliased
  namespace import brings dot-named members (`want` matches the `Ns.`
  prefix); `ModuleItems::name_ref` exchanges a synthesized dotted lookup
  key for the declaration's own `&'p str`; `check_dot_names` enforces the
  same-file non-generic namespace, the scope-wide concatenation ban, and
  module-path casing.
- **Kotlin**: `emit_struct_with_members` nests members inside the
  namespace class (declared under their member segment, referenced with
  the dotted name); `qual_suffix` flattens dots so mangled overloads stay
  single identifiers (`label__EnvironmentTag`).
- **Rust**: `rs_ident` flattens dots — one funnel covers declarations,
  references and mangling, and no other Salvo name can carry a dot
  because dots are invalid in Rust identifiers.
- Verified end to end on both backends with two programs — a namespace
  struct with two members plus a dot-named qualifier driving an overload,
  and dot-named types as *union arms* (`when` narrowing, a dot-named
  qualifier in a nullable union, `canbe Mut` on a dot-named struct):
  identical output under `kotlinc` and `rustc`. Union arm identity needed
  no special handling — the wrapper/enum machinery keys off arm position,
  and the flattened Rust names fall out of `rs_ident`.

## Overload resolution — **FINALIZED 2026-09-07**

Salvo overloads by argument type, and that decision reaches further than any
other single rule: `size(Str)`/`size(List)`/`size([])` are three
declarations, qualifiers make `full_name(Surname Person)` a distinct
overload, mangling exists to keep the target from re-resolving them
[kt-fn-mangling], and the checker's choice is authoritative everywhere
downstream. What had never been *designed* was the ranking; it is now, along
with the scope ladder and the two ways a caller overrides both. The rule is
at the top of this file and in LANGUAGE.md; the labelled rules are
[fn-overload] [fn-overload-scope] [fn-overload-rank]
[fn-overload-ambiguous] [fn-overload-at] [fn-rename]
[fn-overload-duplicate] [fn-value-select] [effect-member-call].

**Where the decisions came from.** Twelve probe programs against the old
implementation, each answering a question nobody had chosen: `describe(3)`
resolving to `describe<T>` (fixed as O1 on 2026-09-06); an own-module `size`
losing to std's *silently*; two identical parameter lists both declarable;
fixed-vs-variadic decided by declaration order; an unavailable-effect
candidate winning and then erroring; an overloaded fn passed by name
resolving to `entries[0]`; and effect member calls checking nothing about
their arguments. The user's answers, in the order asked:

- **Scope first, then signature** — with a *warning* where scope discarded
  the more specific signature, silenced by an explicit `@`.
- **`@` takes a module path**, not a rung keyword: `@mod` vs `@import` asks
  the reader to know which rung a name came in on.
- **The fn rung is everything a function scope holds**: fn-typed parameters,
  locals, implicit parameters, effect members.
- **Qualifier sets rank by inclusion**, kind ignored; unrankable pairs are
  settled by renaming.
- **Unions rank by arm inclusion**; broader is less specific.
- **Fixed beats variadic**, and an *empty* parameter list counts as fixed —
  so `list()` picks the no-argument overload over `list(...elems)`, which is
  what lets an "empty" case be an overload rather than a special form.
- **Identical parameter types are a declaration error**, whatever the names
  or return type say.
- **Effect availability does not filter candidates**: selection is by types,
  and a missing handler is its own diagnostic.
- **Renames apply everywhere a name resolves**, including fn values and
  implicit parameters — documented in LANGUAGE.md with examples, as asked.
- **Implicit parameters take no part in ranking**; an unresolvable one is a
  failure at the winner, not a demotion.
- **`@Effect` for members is deferred** and recorded (see the top of this
  file).

**Implementation notes worth keeping:**

- The old *sum of per-slot scores* is gone. A sum lets one argument's gain
  pay for another's loss, which is the definition of a guess; the ranking is
  a per-slot partial order (`types::spec_cmp` / `rank_cmp` /
  `most_specific`), and "no winner" is the ambiguity error.
- `FnEntry` carries its **rung** (`Core`/`Import`/`Own`) and its declaring
  module, filled where the scope is built — the rungs above `Own` are not
  overload sets, so they need no entry (a local shadows outright, a member
  takes the name first, a rename introduces a new one).
- The **lead candidate** (the source of expected types for arguments) uses
  the same rung filter and the same ranking, re-narrowed per argument. That
  is what keeps `map(arr, n -> n + 1)` working with a `List` fast path in
  scope: the `List` candidate leads until `arr` turns out to be an array.
- `Ty::Any` is now produced by lowering the written `Any` — see the top of
  this file.
- Verified end to end on both backends with one program that overrides
  resolution both ways (`@core.list`, `@main`, a dot-form `@`, a rename, and
  a call past a shadowing local): byte-identical stdout under `kotlinc` and
  `rustc`, with the emitted sources asserted to contain no `@` and no renamed
  name.

## Roadmap: standard library surface

Decisions taken 2026-09-06 (user). **S-Str and S-Seq are built** (see their
entries below and the narrative at the top of this file); **S-IO** is the one
still open, deferred by the user until IO streams are designed properly. Each
item was independently shippable, in the order given, because each fed the
next.

### S-Str — a mutable string, and the string function surface — **LANDED 2026-09-06**

See the entry at the top of this file for what the drop coercion turned out
to need. The surface as built, all in `std/core/string.sv` with lowerings in
each backend's `intrinsics.rs`:

- `intrinsic type Str canbe Mut`; `intrinsic fn mutable_str(...parts: Str[])
  [] -> Mut Str => parts` (the parts are *kept*, since they are read rather
  than stored — which is what makes the Rust lowering borrow them).
- On `Str`: `size`, `char_at`, `iter` (→ `Iter<Char>`, which is what makes
  the S-Seq functions work over strings for nothing), `split`, `index_of`
  (`Int?`, no `-1` sentinel), `contains`, `starts_with`, `ends_with`,
  `trim`, `trim_prefix`, `trim_suffix` (unchanged when the affix is absent),
  `substr` (`Str?`), `to_upper`, `to_lower`, `join(List<Str>, Str)`,
  `parse_int` (`Int?`).
- On `Mut Str`: `append`, `set` (out of range does nothing — growing here
  would make a `set` an `append`), `clear`.
- Rule [str-drop-mut] in LANGUAGE_SPEC.md, with [kt-mut-str] and
  [rs-mut-str] in the backend specs.

Deferred deliberately, because nothing needs them yet: a `Char` → `Str`
conversion (so `set` is the only way to place a character), `replace`,
`repeat`, `pad`, and a `Char`-exact `Str` on Kotlin (see the UTF-16 note
above).

### S-Seq — `map`, `filter`, `reduce` over anything iterable — **LANDED 2026-09-06**

Built as decided (`std/core/iterable.sv` + `std/core/seq.sv`), rule
[seq-iterable]:

```
params Iterable<It, T> { fn iter(it: It) -> Iter<T> }

fn map<It, T, U>(xs: It, f: (T) -> U, ?Iterable<It, T>) -> Mut List<U> => xs, f
fn filter<It, T>(xs: It, keep: (T) -> Bool, ?Iterable<It, T>) -> Mut List<T> => xs, keep
fn reduce<It, T, A>(xs: It, init: A, f: (A, T) -> A, ?Iterable<It, T>) -> A => xs, f, !init

intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> Mut List<U>   // + filter, reduce => list, f
```

Verified on both backends with one program covering a `List` (the fast
path), an array, a `Str`, an `Iter` from an iterator function, a chain, a
named fn as the callback, non-`Copy` elements, and a customer struct made
iterable by declaring `fn iter` — byte-identical stdout under `kotlinc` and
`rustc`.

**The decided design needed four mechanisms that did not exist**, and the
memo's claim that "the generic half needs no new mechanism" was wrong on
every one of them:

- **[implicit-infer] — implicit resolution has to feed back into the call's
  type arguments.** `T` appears *only* in the implicit's type, so nothing
  bound it: the lambda was typed against an unbound `T` and `U` came out
  undeterminable [call-type-args]. Resolution now runs *between* the
  arguments, two-sided: the candidate's generics bind from the known part of
  the pattern, then the caller's variables bind from the instantiated
  candidate. This is also what fixed the recorded array gap — the `List`
  case only ever worked because `List<T>`'s own `T` did the binding.
- **[fn-overload-rank] — a *lead* candidate, re-narrowed per argument.**
  With a `List` fast path beside the generic overload, a bare lambda had no
  expected type at all (multiple candidates kept the untyped probe), which
  is the second gap the memo recorded. Expected types now come from the most
  specific candidate *still compatible with the arguments typed so far* —
  and the per-argument re-narrowing is what makes `map(arr, …)` work: the
  `List` candidate leads until `arr` turns out to be an array, and it has to
  be dropped before the lambda is typed. The lead is a hint only; the
  scoring still re-derives everything.
- **[implicit-intrinsic] — an intrinsic cannot be passed by name.** std's
  `iter` overloads *are* lowerings, so `::iter` (Kotlin) and `iter(__i0)`
  (Rust, where `iter` is the generated *module* — E0423) were both
  nonsense. The adapter closure's body is now the intrinsic's own lowering.
  While there: a resolved *declared* fn's adapter forwards each argument in
  that fn's parameter mode, or a kept struct parameter is passed by value
  (E0308) — reachable as soon as a customer type declares `fn iter`.
- **Parameter contravariance in implicit resolution.** `filter(map(xs, …),
  …)` wants `(Mut List<Int>) -> Iter<Int>` and std has `(List<Int>) ->
  Iter<Int>`; `is_subtype` compares fn parameters *invariantly*, so it did
  not fit. [implicit-resolve] already documented contravariance — the
  implementation just did not do it. Fixed there rather than in `is_subtype`,
  deliberately: a backend renders a parameter's convention from its declared
  type, so general fn-value contravariance would let a `&Vec` callback reach
  a `&mut Vec` position.

**And the Rust `List` fast path could not be an inline expression.** Every
shape that splices the callback into an expression hits Rust closure
inference: a closure bound to a `let` cannot infer its parameter types, and
neither can one nested inside another closure's argument
(`filter(|__x| (|n| *n > 1)(*__x))` — E0282). The fast paths therefore lower
to generated helpers in `seq.rs` ([rs-seq], gated and mounted exactly like
`iter.rs` and `strings.rs`): a generic parameter *is* an expected type, and
it also pins the callback convention `FnMut(&T)` that a declared `(T) -> U`
renders as. `salvo_reduce` is a loop rather than `Iterator::fold`, because
that convention borrows the accumulator and `fold` passes it by value.

Everything else landed as decided: eager `Mut List` results, the group in
its own implicitly visible `core.iterable`, and the identity
`intrinsic fn iter<T>(it: Iter<T>)` that makes an `Iter` iterable and chains
compose.

Deferred (nothing needs them yet): `any`/`all`/`find`/`count`/`zip`/`flat_map`
— the same shape, one more overload pair each — and a lazy `map` for the
effect-free case, which would need a way to say "this callback performs
nothing" at the type level rather than by convention.

### S-IO — streams, then the filesystem

**Deferred by decision** — see ROADMAP.md.

## Test inventory (all green: 1012)

The kotlinc/rustc tests are **content-cached** (`salvo-testkit`): a plain
`cargo test` still runs every one of them, but only recompiles the ones whose
generated code, expected output or toolchain actually changed. Use
`SALVO_E2E_FRESH=1 cargo nextest run` for a run that takes nothing from the
cache, with per-test timings.

- `salvo-core`: 561 - 19 unit tests (file classification, including the
  `platform/` strip [platform-tree]; `types.rs` union
  normalization, subtyping, display, wrapper detection; `place.rs`
  [flow-place]: the prefix relation reflexive and downward-closed,
  different roots never relating, overlap symmetric, an unknown array
  index aliasing every element while constant indices stay distinct, and
  `narrowable` accepting field chains only) + 2 source
  discovery tests (`tests/source_tests.rs` [mod-ignore]: `.svignore`
  skips listed files/subtrees; hidden and `CACHEDIR.TAG` directories
  skipped with the root exempt) + 24 deduction
  tests (`tests/deduce_tests.rs`: exhaustive lists dropping *undeclared*
  qualifiers and delta lists passing them through, mutating bodies
  requiring the exhaustive form, `Nothing` meaning moved
  [deduce-syntax], move inference, call-graph fixpoint
  transitivity, effect-member contracts reaching inference [call-resolve]
  and a keeping member still borrowing, handler *state* stores counting as
  moves so a keeping member that stores its parameter is rejected while a
  moving one is accepted [effect-state-store] — plus its **read** direction
  (added 2026-09-15): a member consuming or returning the handler's storage
  (a state field, a constructor parameter) refused naming `copy`, `copy`
  settling it, and a Copy scalar exempt [copy-scalar-free] — written-list body
  validation,
  written-list shape validation, stricter-than-body lists,
  `let`-bindings linking instead of moving — the parameter stays kept,
  reads through the alias are free, `copy` severs [fate-link] — and
  move-mode bindings *claiming* parameters as moved, through binding
  chains and propagated through the call graph [fate-move-mode], a
  lambda capture-mutation claiming its parameter while a read capture
  keeps it [fate-lambda], and late move-mode candidates converging in a
  third round with the refined diagnostic superseding the raw
  derived-move error [deduce-fixpoint]) + 2
  structured-diagnostic tests (`tests/diag_tests.rs`: checker errors
  carry file index/span/severity and render with file:line:col + caret;
  multi-file programs index the declaring file [diag-structured]) + 5
  name-collision tests ([mod-collision]: conflicting imports, an import
  shadowing an own-module declaration, `as` resolving the conflict, a
  duplicate declaration within one module, and same-name fns staying
  overloads) + 3 generic-binding tests ([fn-overload]: bindings widen to
  the more general type in either argument order; unrelated bindings
  still reject) + 4 optional-strictness tests ([op-no-none]
  [interp-no-none]: possibly-`None` operands rejected for arithmetic,
  ordering, and equality — and `None` itself — while narrowed/asserted
  operands pass; interpolating an optional or a struct field rejected,
  the `is T name` binding and `!` forms accepted) + 7 name tests
  (`tests/name_tests.rs` [name-dot] [name-dot-import] [name-casing]:
  dot-names resolve, the namespace must be a same-file non-generic
  struct, the concatenated-name ban fires for own-module *and* imported
  names, importing a namespace brings its members and a member imports
  directly, an uppercase module path is an error naming the file; and
  7 `[name-resolve]` tests: an undeclared qualifier in an `is` check
  reported as the unresolved name with no "can never succeed" and no
  consumed-value cascade, an unresolved `when` branch not reported as
  non-exhaustive, unknown base types and qualifiers reported at ten
  declaration sites (params, returns, struct fields, `let`, aliases,
  effect members, `of`, `with`), the wrong-namespace wording hint both
  ways, import suggestions on an unknown type [diag-import-suggest], the
  intrinsic qualifiers staying known names, and `canbe` refusing a user
  qualifier [canbe-optin]) + 6
  subject tests (`tests/subject_tests.rs` [qual-subject]: a mutating call
  strips a state qualifier but not a provenance one, provenance composes
  without `with` while two state claims still need it, a provenance body
  is rejected, `is` on a non-union provenance value is rejected,
  provenance is droppable and survives being stored in a struct field)
  + 20 place-narrowing tests (`tests/place_tests.rs` [flow-place]
  [flow-place-invalidate], using `[interp-no-none]` acceptance as the
  observable: a checked field narrows inside the branch, and after a
  branch that **exits** carries the else fact to the fall-through path
  [is-narrow-guard] (the read there is a known-`None` one, not a
  maybe-`None` one) while a branch that falls through carries nothing;
  siblings stay independent, field *chains* narrow, `&&` accumulates place
  facts, the `else` branch carries the negative fact, array elements do
  not narrow (P1a); and for invalidation — assignment to the place, to a
  prefix (dropping the subtree) but *not* to a sibling, a `Mut`-keeping
  call, a `Mut` call through a projection hitting only that subtree, a
  kept-*immutable* call preserving the fact (P1b), and a fact only some
  paths agree on not surviving the join; and for tuple elements
  [expr-tuple-index] — a constant index narrowing, siblings staying
  independent, an out-of-range index and a non-tuple base erroring, and
  assignment to an element rejected)
  + 10 guard-narrowing tests (`tests/guard_tests.rs` [is-narrow-guard]:
  `return`/`break`/`continue`/diverging-call guards narrowing the rest of
  the block, an `elif` chain leaving the third arm, an explicit
  non-exiting `else`, and the negatives — a branch that falls through, a
  mixed `if`, and assignment resetting on the surviving path but not in
  the exiting branch)
  + 21 member-resolution tests (`tests/member_tests.rs` [call-resolve]
  [field-resolve] [index-resolve] [iter-resolve]: unresolved bare and
  dot-calls rejected — the latter naming `external fn` as the remedy, both
  carrying import suggestions — calling a non-fn value and a generic
  rejected, a *declared* external still callable by dot-notation, fields
  on a non-struct/opaque/generic base rejected, `[]` on a non-array
  rejected while arrays still work, `for` over a non-iterable rejected
  while arrays still iterate, and — the leniency that remains — a member
  read off an un-inferred value adding *no* second diagnostic
  [type-unknown-lenient]; and 7 effect/handler-as-data tests
  [effect-not-data] [handler-not-value]: an effect rejected in five data
  positions with the diagnostic naming `use`, effect lists and `of` clauses
  still accepting effects, a handler constructor rejected as a value while
  `use` still registers it, and a handler dependency accepted with its
  effect available in the member body, dependencies resolved from the
  `use` scope — absent one, an error at the registration — and a mutual
  dependency unregisterable in *either* order, so cycles need no check
  [effect-handler-deps]; plus interception [effect-intercept]
  [use-no-dup]: a handler depending on the effect it implements accepted at
  the declaration, an intercepting `use` binding strictly outward — refused
  with nothing to wrap, accepted over an instance in scope, stacking over
  another interceptor — and a plain `use` shadowing an earlier registration
  with no dependency anywhere; and 2 handler-state tests [effect-handler]: a
  state field initializer checked against its declared type, a well-typed
  one accepted; plus 2 [async-types] tests: `Addr<Counter>` accepted as a
  field, a parameter and a return — the sanctioned effect-as-type-argument —
  and `List<Counter>` still refused, so nothing else widened; plus 5
  asynchronous-declaration tests [async-send-fn] [async-spawn-effect]: a
  `send fn` refused a return type on an effect and on a handler, the legal
  no-return shape checking clean, `[spawn]` accepted on a fn and a handler
  and refused on a fn type. The three pending-refusal tests for
  `spawn`/`replyto`/`waitfor` were **deleted** by the slice that implemented
  them, as they were written to be)
  + 28 asynchronous-checker tests (`tests/async_tests.rs` [async-spawn-expr]
  [async-replyto] [async-waitfor] [async-use-addr]: the whole first-pass
  surface checking clean in one `main` — the canary for the feature — then the
  capability gate; a spawned handler refused the spawning scope's
  registrations, an unused clause item, and a clause construction with
  dependencies of its own; `capacity`/`on` typed; the spawn's value being
  `Addr<E>`; an addr call resolved against the served effect, its arguments
  typed, and `use addr` binding the effect while keeping the addr; `use` of a
  plain value refused; `replyto` outside a handler, at an unknown member, at
  a non-`send` member, and with the wrong capture count; the token's payload
  taken from the *trailing* parameter; and `waitfor` outside `main`, with a
  non-token binding, leaking its token, and answering that token's payload;
  plus the three leftovers closed the same day — an addr place of any shape as a
  receiver, all three forms refused inside a lambda [async-no-closure] with
  closure-specific wording, and an overloaded send member picked by arity with
  a same-arity tie refused; plus 6 [async-self-send] cases: a member sending
  to its own process, an unknown and a non-`send` target, the form outside a
  handler member, its arguments typed, `self` refused as a variable in a member
  while staying legal in an ordinary fn, and the self-dispatch diagnostic
  naming `bump@self(…)`)
  + 15 type-argument tests (`tests/type_arg_tests.rs` [call-type-args]:
  an undetermined type argument reported with both remedies; determined by
  the arguments, by an explicit list, by a `let` annotation, by the
  enclosing return type, and by a *concrete* parameter of a nested call;
  a *generic* parameter determining nothing, so the nested call is still
  reported; a type argument confined to the parameters needing no context;
  an unknown-typed argument keeping the call lenient so one mistake yields
  one diagnostic [type-unknown-lenient]; and the resolved bindings recorded
  per call, which is what a backend's intrinsic lowering renders
  [backend-intrinsic]; plus 5 progressive-binding tests
  [call-generic-progressive]: an earlier argument typing a later
  un-annotated lambda, the lambda's *body* determining the result type
  argument, an explicit type-argument list doing the same, an annotated
  parameter needing no binding in any position, and the deliberate
  left-to-right limit — a lambda before its binding argument is not
  inferred)
  + 10 widening tests (`tests/widen_tests.rs` [qual-widen]: a `^` branch head
  opening a nested union, `^ Mut` stripping in an `if` with the mutation it
  then rejects, the intrinsic qualifiers refused with their reasons from the
  shared exclusion list, nothing-to-remove rejected, a *type* on the right
  rejected, more than one arm rejected, the no-binding parse error, and a
  `^` branch consuming its arms so exhaustiveness still reports the rest;
  plus [proj-readonly]: a top-level `proj Mut` parameter refused at the
  declaration, with the nested spelling staying legal)
  + 25 implicit-parameter tests (`tests/implicit_tests.rs` [implicit-param]
  [implicit-group] [implicit-resolve] [implicit-forward] [implicit-override]
  [implicit-fn-only]: a group's members and a written `?cmp` resolved from the
  visible overloads; an unresolvable one reporting *both* remedies; one member
  overridden by name and by lambda; a named argument matching nothing, and one
  of the wrong type, rejected; forwarding through a generic fn, and forwarding
  *across groupings* — a `?Field<T>` spread filling an individually declared
  `?add`; a generic fn without the implicit unable to call one that needs it,
  which is the colouring cost; and the declaration-site rules — non-fn type,
  unknown group, wrong group arity, two implicits of one name, and a group
  member with a body; an effect member *may* declare them, and its call may
  override one, while a handler constructor may not [implicit-fn-only]; and
  the three diagnostics a printed type cannot carry — a contract mismatch
  explained for a resolved default and for a call-site override, and an
  ambiguity that says how many matched); plus 4 handler-generics tests
  [effect-handler-generics]: a `use` writing its handler's type arguments, a
  constructor argument still binding them, the two disagreeing reported, and
  the wrong number of them rejected)
  + 16 fn-type-effect tests (`tests/fn_effect_tests.rs` [fn-effects]: a
  declared effect available in a lambda body while an undeclared one is
  rejected even with the effect in lexical scope; a fn *inheriting* its
  fn-typed parameters' effects, through a qualifier too, with its caller
  required to supply them; variance both ways (a pure fn and a pure lambda
  fitting an effectful position, an effectful fn rejected by a pure one); an
  un-annotated lambda's effects inferred from its body; each declared effect
  required at the call, and a fn value called where its effect is
  unavailable rejected — the reason "forbid escape" was unnecessary; and
  `use` in a fn type rejected; plus 5 iteration tests: an **effect in
  qualifier position** refused with the `yield fn next(…) [E]` remedy and a fn
  type redirected to its own bracket list (the two halves of [iter-effects]'
  deletion), a `yield` outside a `yield fn` reported at the declaration
  [fn-iterator], a `yield fn` declaring its effects normally accepted, and a
  `for` loop performing effects freely — the restriction is on producing, not
  consuming;
  plus 3 callback-capture tests, the same rule's other half: a lambda handed to
  a producer rejected for writing through a capture and for merely reading
  mutable data through one, with the control that an *immutable* capture is
  free and that the same lambda into a non-producer is untouched)
  + 20 throw/`try` tests (`tests/throw_tests.rs` [throw] [try]: the
  outcome type read off an annotation mismatch (`Ok Int | Thrown Str`),
  several message types unioning (`Thrown (Str | Int)`), an always-leaving
  body still carrying `Ok None`, a `try` that cannot throw rejected, an
  throw with nowhere to land rejected while declaring the effect
  propagates, a message the target cannot carry rejected, `main` declaring
  `Throw` rejected [throw-not-main], a handler *for* `Throw` rejected, an
  throwing branch counting as returning [fn-must-return] and not leaking
  its consumption to the fall-through path [type-any-nothing], a linear
  value across a may-throw call rejected with the `defer` remedy accepted
  [throw-linear], `throw` *and* a may-throw call inside a deferred block
  rejected [defer-no-escape], an inner delimiter taking only its own
  throws [try-innermost]; plus the two nested-qualification shapes the
  design asked to test rather than assume — a nested result outcome and a
  union message, both taken apart through a binding at the inner type — and
  the diagnostic that names that remedy when a qualified union is matched
  directly [when-union-subject]; and an assignment inside a `try` body
  resetting an earlier `is` narrowing, with the no-assignment control —
  behavioural, since flow-sensitive checking is what achieves it rather
  than the syntactic assigned-name scan)
  + 12 deferred-block tests (`tests/defer_tests.rs` [defer]
  [defer-no-escape]: a linear obligation discharged on both paths of a fn
  with an early `return` — with the no-`defer` control proving the
  acceptance means something — and per iteration on a loop body's
  `continue` path; a manual consume plus a deferred one reported as a
  use-after-move, exactly *once* even though the block is applied at two
  exits; a read after a deferred consume staying legal (the deferred code
  runs later); a narrowing the body relied on rejected when a `Mut` call
  invalidates it before the exit, with the surviving-fact control accepted;
  `return`/`break`/`continue` in a deferred body rejected while a loop
  written *inside* it keeps its own; and the body's own linear value owed
  inside the body).
  + 12 `when`-condition tests (`tests/when_tests.rs` [when-condition]
  [cond-bool]: the subject-less chain accepted; the mandatory `else`
  keeping `None` out of the value, with the `if`-without-`else` control
  showing the `Str?` it replaces [if-else-none]; a missing `else`, an
  `else`-only `when`, and a branch after the `else` rejected; an `else` in
  the *subject* form rejected with the diagnostic naming the other form;
  `is` heads narrowing their branch *and* the `else`; a chain returning
  from every branch counting as the fn's return [fn-must-return]; and the
  boolean rule on all four condition positions, on the offending *leaf* of
  a compound condition only, on `Bool?` (with `!` accepted), and staying
  quiet on an un-inferred type [type-unknown-lenient]).
  + 9 platform-effect tests (`tests/platform_tests.rs` [platform-effect]
  [effect-member-unique]: a platform effect performed like any other effect
  with no handler anywhere; a Salvo handler *depending* on one (which is how
  Salvo-written handlers reach the host); and the restrictions — a Salvo
  `handler ... of` a platform effect rejected with the ordinary-`effect`
  remedy, a generic platform effect and a generic member each rejected, two
  members of one effect sharing a name, the same name across two effects
  (reported *once*, at the second declaration), and distinct names across
  effects staying legal).
  + 17 refinement tests (`tests/refine_tests.rs` [qual-refn]
  [qual-refn-match] [qual-refn-scope] [qual-refn-conflict]
  [qual-refn-reconcile] [qual-refn-infer], with *overload resolution* as
  the observable — a `NonEmpty` overload resolves only while the checker
  still believes the claim): a refinement re-establishing what a mutating
  call's exhaustive list dropped, with the no-refinement control proving
  the acceptance is the refinement's doing; `-Q` invalidating a claim a
  call would otherwise have kept; conflicting refinements all standing
  down *with a warning* while compatible ones (`with`-declared) both
  apply; a top-level `refn` reconciling the conflict by replacing them; a
  refinement in scope only with its qualifier, and a top-level one not
  leaving its module (same module, second file: applies; another module
  importing everything it can: does not); and the declaration rules —
  four ways to miss an overload (wrong parameter name, wrong type, no such
  function, no such parameter), an unbound type parameter reported as
  itself with the `refn add<T>` remedy, a qualifier refining someone
  else's claim, provenance and intrinsic qualifiers refused, a *moved*
  parameter having nothing to refine, and a qualifier that does not apply
  to the parameter's type; plus the three inference facts — a refinement
  reaching an inferred deduction, a written list allowed to promise the
  refined qualifier (with the no-refinement control rejected by
  [deduce-infer]), and a *conditional* refined call **not** reaching the
  contract).
- **13 obligation-group tests** (`tests/group_tests.rs` [group-obligation]
  [group-self] [group-not-a-value], roadmap R1 — groups declared in the
  tests, nothing designated: a satisfied `Mut It` protocol clean; the
  missing-member error at the *struct* naming the substituted signature; a
  wrong shape (non-`Mut` state) not satisfying; a generic `Zip<A, B>`
  satisfying through its own parameters and the bijectivity refusal
  (`(A, A)` vs `(A, B)`); parameter names not part of matching; unknown
  group with is-a-type hint; arity and duplicate as declaration errors;
  one group serving an obligation *and* a `?` spread — the point of writing
  `self` as an argument — with `self` outside an obligation unknown, and a
  group with no obligation still spreading; a **mutating** implementation
  filling a `Mut` spread position and driving it to exhaustion
  [fn-contract], which is every pass's `next` and was impossible until the
  member's contract reached its fn type; and the six-position
  not-a-value sweep, each with exactly one diagnostic).
- **15 yield-origin tests** (`tests/yield_origin_tests.rs` [yield-fn-origin],
  roadmap R3: the sugar discharging the obligation; **driving an origin twice**
  clean, which is the model's whole point; drive-site effects required at the
  loop and clean when declared; the six declaration rules (named `next`, one
  parameter, non-`Mut` origin, origin carries the clause, return type is the
  clause's `T` reported *once* at the return type, body must yield); both forms
  on one type refused; a direct call refused, naming the remedy; and the
  **driven-origin** rule — mutating one mid-loop refused (through a projection
  *and* through a `Mut` parameter), a transitively mutable origin mutated before
  and after its drives clean, and the refusal ending with its loop).
- **25 linear-group tests** (`tests/linear_group_tests.rs` [linear-group]
  [linear-discard] [linear-composite], roadmap R4: declaring
  `: Linear<self>` with its `close` clean and without one an error at the
  struct; `discard` refused for a linear value and still dropping a plain
  one; the leak diagnostic naming `close`; a `close` alone *not* making a
  type linear; `canbe linear` refused on a declaration and kept on a type
  parameter; and the ten composite-refusal cases — a struct field, a field
  of composite type, handler state, the five written positions (array,
  tuple, union, `T?`, type argument), array and tuple literals, a generic
  struct literal, a storing generic call (`add`) — against the positives
  that must stay legal: reading a composite of `T` (`size`), a container of
  plain values, and one error for a nested composite; plus the four
  [iter-generic-drive] release cases — a fn *owning* a possibly-linear pass and
  driving it without a `close` reported with the `?Linear<It>` remedy, the spread
  supplying it accepted, a pass the fn *keeps* needing nothing
  [iter-drive-in-place], and a pass that never opted in needing nothing either;
  plus 3 [linear-opaque] cases: a `linear intrinsic type` with no consuming fn
  in its own file reported at the declaration, the same type with an
  `intrinsic fn` discharger checking clean while a *dropped* value is still a
  leak naming that discharger, and `linear` on an **alias** refused in the
  parser).
- **22 overload-resolution tests** (`tests/overload_tests.rs` [fn-overload]
  [fn-overload-scope] [fn-overload-rank] [fn-overload-ambiguous]
  [fn-overload-at] [fn-rename] [fn-overload-duplicate] [fn-value-select]:
  the ranking rung by rung — concrete over a type variable in *both*
  declaration orders, a narrower union (and the caller-knowledge limit: an
  `Int | Str` value not fitting `f(Int)` until it is narrowed), `T` over
  `T?`, `Any` last *and* accepting everything, qualifier sets by inclusion
  with the unrankable pair reported, fixed over variadic including the
  no-argument case, and per-slot dominance so one slot never pays for
  another; the ladder — this module over core, core→import→module in order,
  and the scope-override *warning* naming both `@` forms; `@module` picking
  a module's overload, erroring when that module has none, reaching past a
  local of the same name, and working in dot form; fn values selected by the
  expected type and reported as ambiguous without one; renames settling an
  ambiguity, scoped to their block, refusing a taken name, a mismatched
  parameter list and a `@module` on top; and a duplicate parameter list
  reported as a duplicate while differing types stay an overload set).
- **12 `Mut Str` tests** (`tests/str_tests.rs` [str-drop-mut]
  [type-canbe-mut]: `Str canbe Mut` while `Mut Int` is still an error, and a
  literal is not a builder; a recorded drop at every site — call argument
  (a declared fn and an intrinsic), `let` annotation, `return`, struct
  field, union arm (with the displaced `WrapUnion` asserted to survive as
  the drop's continuation), interpolation, `==` and `+`; and no drop where
  the target keeps `Mut` — a `Mut Str` parameter, an optional `Mut Str?`, a
  generic position (which is what makes `copy(builder)` a builder) — nor
  for a plain `Str`).
- **8 sequence-function tests** (`tests/seq_tests.rs` [seq-pass]
  [implicit-infer] [fn-overload-rank]: everything inferred for a list pass and
  for a `Str` pass — with a `Char` element proved by rejecting a `Str`
  operation on it; chains composing through `iter` over the previous result; a
  customer struct joining in by declaring an `iter` that answers a pass; a
  subject that is not a pass reported as *no matching overload* (the `Mut It`
  position rules an `Int` out before its spread is resolved); and the
  *selection* facts — a `List` subject resolving to the intrinsic fast path, a
  pass subject to the generic body, and the lead candidate narrowing before the
  lambda is typed; plus 4 [iter-generic-drive] tests: a generic pass driven by
  `for`, the driven element being *owned* rather than derived (which is what lets
  a combinator move it into its output), a type parameter without the spread
  reported as not iterable, and a non-`Mut` position refused).
- **14 generator-plan tests** (`tests/generator_tests.rs` [iter-generator]:
  the I3 acceptance test — the plan for the checked-in
  `tests/fixtures/gnarly.sv`, rendered as text and compared with the eight
  states of the hand-written prototype machine, in its order — plus one
  shape at a time: a bare loop resuming at its own head, a `yield` in the
  middle of a body, a `break` and a `return` discharging what they leave, a
  nested `for` becoming a pass field, a suspending `if` arm getting states of
  its own, and a yield-free loop staying one `Plain` step; and the
  refusals — a `when` containing a `yield`, a `yield` in a value position, a
  shadowing local and a shadowed parameter, a suspending loop with an `else`,
  and a destructuring `let`).
- **30 collection tests** (`tests/collection_tests.rs`, the S-Col rules:
  the three literal forms and their inferred types, `{}` as an empty
  collection rather than a bare struct literal while a *named* `Finished {}`
  still parses, an empty literal typed by its expected type and an error
  without one; key eligibility at the instantiation — a `canbe Mut` struct,
  a float-bearing struct, an un-opted-in struct and a union as a
  `SortedMap` key all refused with the reason, a union accepted as a plain
  `Map` key; the opt-ins validated at the *declaration*, with a fn-typed
  field barring `==` and therefore both; `==` on same-base-type operands
  only, qualifiers ignored (`Surname Person == Person` accepted, two
  different struct types refused); `canbe ordered` gating `<`; and
  [linear-generics] refusing a linear element without a new rule, since the
  constructors declare no `<T canbe linear>` — which is what makes linear
  keys permanently impossible rather than merely deferred; plus
  [col-sorted-list]: a `Sorted List<Double>` refused by element, and
  [qual-overload]: one name over two subjects accepted, a subject no
  declaration accepts refused, and one name twice over one subject still a
  duplicate. The rest of
  C-6 needs std's own declarations, so it is asserted end to end in each
  backend's `compiles_and_runs_list_claims` case instead of here — this
  harness builds its own prelude).
- `salvo-cli`: 87 - 51 `analyze` integration tests running the built
  binary (`tests/analyze_tests.rs` [cli-analyze]: clean program exits 0,
  type errors render with location and exit 1, JSON diagnostics
  (populated + empty array), parse errors reported, a parse error in one
  file not suppressing checker diagnostics in others, `.svignore`
  exclusions [mod-ignore], import suggestions rendered as help lines +
  JSON `imports` for std and user modules [diag-import-suggest],
  missing-return analysis [fn-must-return], use-after-consume from
  declared *and inferred* deductions, uniform across types, incl.
  reassignment revival, consumption surviving `is`-narrowing restores,
  `while`/`for` loop back-edge re-checking with consume-then-revive
  staying clean, the `if`/`else` merge matrix (consumed on every
  fall-through path / on the only fall-through path / only on an
  always-exiting path), `when`-arm merging incl. subject consumption,
  partial qualifier removal joining conservatively, and call-site
  qualifier removal for kept params incl. `[list:]` [deduce-consume],
  bodyless-declaration explicitness ([decl-explicit]: an effect member
  missing the two parts that apply to it and *not* asked for effects, an
  effect member's declared deductions enforced at the call site, and std's
  `add` consuming its element),
  the D1 deduction forms ([deduce-syntax]: the `clear`/`NonEmpty`
  unsoundness now rejected, a delta preserving an undeclared qualifier, a
  bodyless `Mut` parameter refusing the delta form, a mutating body
  refusing keep-all, mixed polarities, `+Qual`, and non-`Nothing` types),
  union-arm arguments resolving against union params [type-union],
  std's result tags working with no local declarations
  (`Ok Int | Err Str | None` narrowed by `when`, [qual-result-tags]) and
  an undeclared qualifier reporting the *name* rather than the arm
  mismatch it causes [name-resolve],
  the shared-fate matrix (derived variables read-only for
  moves/mutations/returns with the `copy` remedy [fate-derived-readonly],
  root mutation/move/reassignment poisoning derived variables with
  use-site errors naming the event — and unobserved poison staying
  silent [fate-poison], links flowing through projections, `for` and
  `is` bindings transitively to the root, reassignment revival, links
  unioning across branch merges [fate-link], `copy` producing
  independent values that make the whole matrix pass [copy-fn],
  and the backend-parity fix: a projection argument in a kept-`Mut`
  position mutating its provenance roots, poisoning derived variables
  and rejecting mutation through derived ones),
  `[struct-mut]` field assignment requiring a `Mut`-qualified struct
  value,
  the L2 move-site matrix (struct/array/tuple literal stores, spread,
  `break` inside an always-exiting branch consuming after the loop,
  `yield`-in-loop back-edge, `use` constructor arguments — each with the
  event named in the diagnostic, derived variables rejected at the new
  sites, and the positive side: `copy` at every site, reassignment
  revival incl. ahead of the loop back edge, `break n` consuming only
  its operand, and in-branch `return` poison not leaking to the
  fall-through path [deduce-consume] [fate-derived-readonly]),
  the S2 move-mode matrix ([fate-move-mode]: the zero-clone pipeline
  and mutation-driven move-mode staying clean, per-iteration loop
  bindings consumable, immutable projections in moved positions free,
  `copy` keeping sources usable; and the negative side: ancestors
  consumed at the binding with the use-site error naming the binding
  through whole chains, the moved-position parity probe rejected,
  kept-parameter projections erroring with the `copy` remedy, and
  written-kept bindings keeping the S1 move-site error),
  same-call argument ordering ([deduce-same-call]: double moves and
  move-then-read within one call rejected at the later argument, `copy`
  at the consuming argument and kept-position reads clean),
  the L4 capture matrix ([fate-lambda]: immutable captures free,
  closure poisoned by a mutable read-capture's root mutation, mutated
  captures consumed at creation with claims reaching callers,
  written-kept parameter capture-mutation rejected, capture consumption
  always rejected, `copy` remedies clean),
  the L6 linearity matrix ([linear-obligation]: scope-exit leak,
  consumed-on-some-paths-only, dropped expression result, overwrite of
  a live value, return-while-owing, `copy` refused, lambda swallow
  rejected, and a **struct field holding a linear value** refused
  [linear-composite] — with pass/`close`/return/kept-borrow/alias all
  clean — and the generic instantiation ban [linear-generics]),
  the L7a opt-in matrix ([linear-generics]: opted bodies checked with
  `T` linear, unopted forwarding rejected, non-`Linear` clauses
  rejected, variadic positions refusing linear values with their
  follow-on leak),
  the L7b `once` matrix ([once-fn]: double call and loop back-edge
  call consumed, `once` lambdas rejected at plain-fn boundaries,
  escape-then-call rejected, `once` on non-fn types rejected, with
  maybe-call and both subtyping directions' positives clean),
  the L7c derived-return matrix ([readonly-return]: independent
  returns rejected, moved/unknown parameters rejected, argument
  mutation poisoning the result, borrowed results immovable with the
  `copy` remedy clean, direct element returns and forwarded derived
  calls clean),
  the L7d contract matrix ([fn-contract]: double use through a
  consuming contract rejected, a consuming named fn rejected at a
  keeping boundary, kept lambda parameters unconsumable, consuming
  contracts propagating to callers, keeping contracts clean),
  `--backend` opting
  define files into the analysis, unknown backend rejected;
  and refinements over the *real* std [qual-refn]: a refinement recovering
  the `NonEmpty` std's `add` necessarily strips, with the no-refinement
  control failing overload resolution, and a conflict warning that leaves
  the exit code 0 and renders as `"severity": "warning"` in the JSON
  [qual-refn-conflict]) + 2 UTF-16
  position-mapping unit tests (`src/lsp.rs` [cli-lsp]: multi-byte and
  supplementary-plane round-trips, clamping) + 3 LSP integration tests
  (`tests/lsp_tests.rs` [cli-lsp]: speaks framed JSON-RPC to the binary —
  initialize, didOpen of an unsaved broken buffer -> publishDiagnostics
  with UTF-16 range, didChange fix -> clearing publish, hover -> checked
  type, fn-name hover -> full signature with inferred deductions at both
  the declaration and a call site [fn-ref-table], derived-variable
  hover -> bare `ReadOnly T` type line with root/binding-site detail
  below [fate-link],
  shutdown/exit -> clean process exit; codeAction import quickfix
  round-trip [diag-import-suggest]; go-to-definition for a call-site
  callee, a cross-file struct, an effect member, a handler in `use`, and
  an effect in `of`, plus a no-name position yielding nothing
  [lsp-definition]; and doc-comment hover [doc-comment]: fn docs after the
  signature block, markdown verbatim, a resolvable `[symbol]` becoming a
  link while an unresolvable one stays literal [doc-symbol-ref], struct
  docs with the blank-line-separated comment above them excluded and a
  **Fields** section listing typed/defaulted fields with their own docs
  [doc-struct-fields], the same docs at a *use* of the struct name, and a
  variable hovering as its narrowed type with the declared type named
  below — and as its plain declared type on the parameter itself
  [doc-hover-narrowed]; and nested-declaration hover: a struct field at its
  declaration and at an access (identical contents, via
  `Checked::field_refs`) with its default as written and its owner named,
  a refinement's docs merged into the refined fn's hover as a
  **Refinements** section naming the effective entry and the qualifier it
  came from [qual-refn-docs],
  an effect member at its declaration and at a call, handler state, and a
  handler member whose `[symbol]` references reach the handler's own state
  — plus go-to-definition on a field access [lsp-definition])
  + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one)
  + 17 `run` tests (`tests/run_tests.rs` [cli-run], each in its own
  working directory so the default `--target` lands in the sandbox): the
  same program compiled and run on *both* backends with identical asserted
  stdout; `--main` alone implying its directory as the source root;
  `--src` and `--main` *together* reaching a nested entry point on both
  backends, with the `--main`-alone control failing on the unresolved
  import that proves the combination is not redundant; an entry outside
  `--src` rejected; `--main` choosing between two entry points in one
  directory on both backends —
  which is the [rs-crate] regression, since the emitter used to pick the
  first `main` it found and the other choice built a module without the
  `mod` declarations; the program's exit code and stderr reaching the
  caller (a Rust panic, so nonzero rather than a fixed code); the two
  `--clean-target` modes and the fact that both clear the target first;
  and the validation that needs no toolchain — a target that is, contains,
  or sits visibly inside the sources (each with the sources asserted
  intact afterwards), at least one of `--src`/`--main` required, a
  required and validated `--backend`, a missing `main` from either
  direction, a define file rejected as an entry point, a check error
  stopping the run with a single un-double-prefixed diagnostic, and the
  deletion guard refusing a target that holds a `.sv` file;
  and — per backend — a *warning* reaching the builder without failing the
  run [qual-refn-conflict] [diag-structured]: the program's stdout asserted
  (so it really ran), the exit code 0, and the rendered warning with its
  location asserted on stderr. Either half alone would be a bug — an abort
  rejects a legal program, and silence leaves the diagnostic visible only in
  `salvo analyze`).
  + 5 `platform generate` tests (`tests/platform_tests.rs` [cli-platform]
  [platform-tree]): the whole arc per backend — the run failing with an
  error that names the command and the path, the command writing
  `platform/main.<ext>`, the stub implemented, and the *same* `salvo run`
  then printing identical stdout on both backends; a second generate
  leaving an edited host byte-identical and saying so; both backends'
  hosts coexisting in one tree; a program without platform effects
  generating nothing; and a nested layout where the effect's module gets
  the implementation and the entry's module (chosen with `--main`) gets the
  `main`, each mirroring its own source path, with the cross-module
  reference qualified as `crate::platform_telemetry::TelemetryHost`.
- `salvo-syntax`: 87 (three parser tests for the scope selector and
  `rename` [fn-overload-at] [fn-rename]: `@` on a name, a dot call and a
  value, the placement error, module- and statement-level renames, and the
  four things a rename may not repeat; two std snapshots for `core.iterable`
  and
  `core.seq` [implicit-group]; three refinement parser tests [qual-refn]:
  a refinement in a qualifier body with its docs and its `+`/`-` entries, a
  top-level `refn` as an item of its own, and the four things a refinement
  may not say — effects, a return type, an unsigned qualifier, a missing
  deduction list — each a diagnostic naming the reason. Six
  implicit-parameter parser tests were added
  [implicit-param] [implicit-group] [implicit-override]: `?cmp:` and
  `?Field<T>` parsing side by side — the group spread is *not* a parameter —
  a `params` group of bodiless members, a `name = value` argument recognised
  by the `=` that follows an identifier, and the two ordering rejections, an
  ordinary parameter after an implicit and a positional argument after a
  named one) (the std *define-file* snapshot tests were deleted
  with the define files themselves, as were the `defines.kotlin.sv` corpus
  file and its snapshot, and `imports_externals.sv` became `imports.sv`;
  six declaration-form tests were added — [decl-body]: a bodiless top-level
  `fn` erroring with `platform effect` named, a bodiless non-alias `type`,
  a `canbe` clause not counting as a definition, and the two defining forms
  (`= alias`, `intrinsic`) *not* erroring; plus `external` and `define`
  asserted to be ordinary identifiers now, so each of their three old forms
  fails with a plain "expected item"; three `platform effect` parser tests
  were added [platform-effect]: the flag is set by the modifier, a plain
  `effect` leaves it clear so nothing existing changed meaning, and
  `platform type` / `platform fn` are parse errors naming the form) (the
  corpus grew three LANGUAGE.md examples with E3: a
  `defer` in `control_flow.sv`, `throw`/`try` in `effects.sv`, an effectful
  fn type in `functions.sv`) - std +
  LANGUAGE.md-corpus parse-clean assertions with
  insta AST snapshots (`tests/corpus/*.sv`, plus `std/core/result.sv`),
  error-reporting tests,
  lexer unit tests for numeric literal suffixes [lit-numeric] (`1L`,
  `1.2f`, invalid suffix/juxtaposition errors, `1.size()` stays an int),
  5 tuple-index tests ([expr-tuple-index]: `t.0` parses as a projection,
  `t.0.1` as *two* projections rather than a float, floats still lexing as
  floats where an index cannot appear, a tuple element as a dot-call
  receiver, and numeric suffixes rejected),
  3 `canbe` opt-in tests ([canbe-optin]: `canbe Mut` on a struct and
  on an `external type`, `<T canbe linear>` on a fn, and `canbe` on a
  non-fn type parameter rejected [linear-generics]), and 5 name tests
  ([name-dot] [name-casing]: dot-names in declarations and type
  positions, a dot-name struct literal distinguished from a field read
  and a dot-call, three-segment names rejected, the casing rule enforced
  across ten declaration forms, generic parameters uppercase), 5 doc-comment
  tests ([doc-comment] [doc-struct-fields]: the block directly above a
  declaration with a blank line ending it and a bare `//` kept, trailing
  comments documenting nothing, struct and field docs captured separately,
  docs surviving `external`/`intrinsic`/`provenance` modifiers, and
  indentation kept after the marker), and 2
  subject tests ([qual-subject]: `provenance qualifier` parses with the
  provenance subject while a plain declaration defaults to state;
  `provenance` must precede `qualifier`), and 2 `try` tests ([try]: `try { ... }`
  parses as a block *expression*; a bodyless `try` is a parse error naming
  the form), and 6 subject-less `when` tests ([when-condition]: the
  condition chain parsing into `Expr::WhenCond` with its branches and
  `else`, a subject still parsing as the arm form, and the four parse
  errors — missing `else`, `else`-only, a branch after the `else`, and an
  `else` in the subject form), and 5 asynchronous-surface tests
  ([async-send-fn] [async-spawn-effect] [async-spawn-expr] [async-replyto]
  [async-waitfor]: the declaration forms with a state field named `send`
  beside them; the expression forms in one program covering every clause —
  a spawn with and without a `use` clause, `replyto` and `replyto!` with
  their captures, `waitfor`'s binder and written type, and `use addr` as a
  plain `Stmt::Use` over a name; the two missing-clause parse errors; and
  all five new words still usable as ordinary identifiers, since not one is
  reserved).
- `salvo-backend-kotlin`: 99 - **the compile-and-run programs are one
  test now**: each is a fn returning a `KotlinCase` listed in
  `KOTLIN_CASES`, and `kotlinc_compiles_and_runs_every_case` batch-compiles
  the stamp-missing ones in a few parallel kotlinc invocations (per-case
  package prefix `k_<tag>.salvo…`), runs them in parallel, and stamps each
  case separately — so the count fell from 151 with no coverage change
  (2026-09-12; 86 cases as of the member-name-collision case, which pins both
  halves of that defect by *running* — a member named `add` beside std's own,
  and `to_upper@core.string` versus `to_upper@Shout` [effect-available]. Beside
  it, the dependent-spawn case runs a child whose `[Log, Tally]` arrive as a
  construction and an addr, asserting the same `last bumped 3` / `sum 5` the
  Rust backend does, and the plain process case asserts `sum 5`
  [kt-process]). The remaining tests: golden snapshots of the M2 demo, the M3
  unions demo, the M4 qualifiers demo, the M5 effects demo, and the M6
  loops demo;
  M7 assertions (only-used-modules + companion copying, per-module
  packages + generated imports, alias imports, effect-param collision
  avoidance, unique destructure temps);
  M8 `Mut` assertions (`Mut List<T>` maps through the `Mut inline:`
  template; `Mut` on a non-`canbe Mut` type is an error [type-canbe-mut]);
  wrapper/wrap/`is`-lowering assertions (`unions_emit_sealed_wrappers`),
  predicate/mangling/field-cast assertions
  (`qualifiers_lower_to_predicates_and_mangled_overloads`), checker-driven
  effect-resolution assertions (`effects_resolve_through_checker_tables`),
  loop-lowering assertions (`loops_lower_to_run_blocks`);
  union-arm argument wrapping at call sites [type-union]; numeric literal
  suffixes; handler-member template returns
  [kt-handler-template-return]; predicate-qualifier constructors
  [qual-ctor-predicate];
  negative tests (non-exhaustive `when`, non-union `when` subject,
  no-matching-arm wrap, missing effect handler at a fn call site and at an
  effect-member call site, `use` without the `use` effect, duplicate effect
  in an effect list, duplicate `use` registration, unknown effect,
  ambiguous generic effect call, handler-member effect deps, duplicate
  qualifier, incompatible qualifiers, `of`-type mismatch, constructor
  same-file rule, non-simple constructor
  return, `is` on constructive qualifiers, `qualifies` signature,
  constructive values only from constructors, `break` outside a loop,
  missing defines for used external fns/types, uncovered core externals,
  companion/generated-file collision); `copy` intrinsic lowering
  assertions (identity / `.toMutableList()` / `.copy()` / `.copyOf()`
  [kt-copy] [intrinsic-fn]) and a negative test (`copy` of nested
  mutability is a codegen error);
  general-sweep assertions (precedence-preserving binary rendering,
  iterator-body `return` retargeting through a value-position loop,
  alias imports of mangled qualified overloads keeping the `__Qual`
  suffix [kt-imports] [kt-qual-mangling], field-subject `is` lowering
  [is-narrowing] with `when` still rejecting field subjects
  [when-union-subject], place narrowing [flow-place] (a narrowed nullable
  field read asserting [kt-narrow-field-assert], a narrowed field *chain*
  read, a narrowed wrapper-union field read taking its arm payload — plus
  the kotlinc run, whose stdout matches the Rust backend's byte for
  byte); a narrowed `val` field *not* asserted (kotlinc smart-casts it, so
  the assert would only warn) and a narrowed field used as an operator
  operand [op-no-none], union coercion inside array/tuple literals and
  lambda tail returns, type-directed dispatch for unchecked define
  overloads plus the ambiguity error [backend-never-wrong], effect
  member generics rendered on the interface and bound per call
  [effect-member-generics], and an aliased effect type resolving to the
  handler registered under the canonical type
  [effect-disambiguation], and the std array functions with the
  LANGUAGE.md `CyclicRandom` handler [type-array]);
  handler-dependency assertions
  ([effect-handler-deps]: the dependency as a constructor field, the
  member signature still matching the interface, the `use` site supplying
  it, callers not mentioning it);
  type-argument assertions ([call-type-args] [backend-intrinsic]: std's
  list constructors carrying their element type from a `let` annotation and
  from an explicit type argument — `mutableListOf<Int>()`, which is the form
  kotlinc requires);
  and twenty-seven kotlinc compile+run tests
  with exact stdout assertions (including the M7 multi-module program
  with packages, generated imports, and a companion file, the S1
  copy demo, the S2/S3 move-mode and borrow demos, the L6 linear
  resource demo, and the L7a–L7d linear-generics, `once`,
  derived-returns, and fn-contracts demos —
  emission aliases throughout, stdout identical to the Rust runs
  [fate-move-mode] [fate-link] [linear-static] [once-fn];
  the last four are the handler-dependency programs the Rust fusion runs —
  the same sources, the same asserted stdout, which is what parity means
  here [effect-handler-deps] [rs-effect-fusion]);
  and 2 fn-type-effect tests ([fn-effects] [kt-fn-effect-params]:
  `fn_type_effects_thread_into_lambdas` asserting the inherited effect in
  the signature, the effect as a leading *lambda parameter* rather than a
  capture, a matching named fn passing as `::name` and a pure one wrapped in
  an adapter; plus the kotlinc run of the program the Rust fusion used to
  reject, with the same stdout);
  and 2 throw tests ([throw] [try] [kt-throw-signal]:
  `throw_lowers_to_a_signal_and_try_to_a_catch` asserting the generated
  stack-trace-less signal, a tagged `throw`, a *plain* call in the
  propagating frame (no colouring), the delimiter's tag dispatch with its
  rethrow fallback, and no interface emitted for the effect; plus the
  kotlinc run of the throw demo, whose stdout matches the Rust run byte for
  byte);
  and 2 `defer` tests ([defer] [kt-defer-finally]:
  `defer_lowers_to_try_finally` asserting one `try` per `defer`, nested
  latest-first, and a single `finally` covering both `return`s of a
  two-exit fn; plus the kotlinc run of the defer demo — LIFO at a block
  end, an early `return`, `continue`/`break` out of a loop body, and a
  linear handle released on both paths — whose stdout matches the Rust
  run byte for byte);
  and 2 overload-dispatch tests ([kt-fn-mangling] [fn-overload]:
  `every_emitted_overload_gets_its_own_kotlin_name` asserting the second
  overload is renamed and the delegation reaches its sibling; plus the
  kotlinc run of a `List`→pass delegation, which before the rule
  recursed until the stack ran out);
  and 2 implicit-parameter tests ([implicit-param] [implicit-group]:
  `implicit_parameters_lower_to_trailing_fn_parameters` asserting the trailing
  fn-typed parameters, `::add`/`::times` references at the call sites and *no*
  class for the group; plus the kotlinc run of the same source and stdout the
  Rust backend asserts);
  and 2 effect-member-implicit tests ([implicit-param]: the interface method,
  every handler's `override` and the member call all carrying the member's
  implicits; plus the kotlinc run of the shared demo);
  and 4 generic-drive tests ([iter-generic-drive] [iter-drive-in-place]
  [fn-effects]: `a_kept_pass_is_driven_in_place` (no local, `next(it)`),
  `an_owned_generic_pass_is_closed_by_the_loop` (the implicit `close` in the
  `finally`), `a_machine_takes_a_generic_effect_handler` (`Random<Int>` as an
  ordinary machine parameter, and the *builder* called without handlers), plus
  the kotlinc run of the shared demo — a combinator driving a kept pass twice so
  the caller sees the position it reached, and a `yield fn` performing a generic
  effect, with the stdout the Rust backend asserts byte for byte);
  and 2 laziness tests ([fn-iterator] [yield-fn-origin]:
  `an_origin_lowers_to_a_lazy_machine` pinning the generated class — the
  body's locals as properties, the dispatch loop, the resume point written
  back before the element is handed over — and asserting the `iterator { … }`
  builder and every `Iterable<Int> {` factory are *gone*; plus the kotlinc run
  of the *same source* the Rust backend runs (an unbounded origin, `filter_lazy`
  over it, and an origin driven twice to prove it replays), asserting the *same
  stdout* — which is the parity claim itself, not a Kotlin property)
  and 2 subject-less `when` tests ([when-condition] [kt-when-cond]:
  `a_subjectless_when_emits_a_subjectless_kotlin_when` asserting the
  Kotlin `when {` with `cond ->` arms, a plain `else ->` with no optional
  filler, and the `is` binding declared inside its arm; plus the kotlinc
  run of the demo — value and statement position, `is` heads, a chain
  returning from every branch, and one nested in a subject `when`'s arm —
  whose stdout matches the Rust run byte for byte)
  and 2 `try`-body tests ([try]: a variable assigned *only* inside a `try`
  body declared `var` — the traversal gap that emitted `val` and had
  kotlinc reject the output — plus the kotlinc run of the same program).
  and 2 refinement tests ([qual-refn] [qual-erasure]
  [qual-refn-conflict]: `kotlinc_compiles_and_runs_a_refined_program` — the
  refined program runs, and the emitted `main` contains no `qualifies` call,
  since a refinement is trusted rather than checked; same source and same
  asserted stdout as the Rust backend, which is the parity claim itself; and
  a suppressed conflict still *emitting*, since a warning may not stop
  codegen);
  and 4 platform tests ([platform-effect] [kt-platform-entry]
  [platform-tree] [kt-platform-host]:
  `platform_effect_emits_an_interface_and_a_host_entry` asserting the
  generated `interface`, the *absence* of a handler class, `salvoMain`
  taking the instance, no generated `fun main(`, and the effect threaded
  into an intermediate frame; `platform_generate_renders_a_host_skeleton`
  asserting the skeleton's path, its own `salvo.platform.main` package, the
  import, the `TelemetryHost : Telemetry` class, the stubbed `override` and
  the host `main`; `a_missing_host_file_names_the_command` asserting that a
  platform program without a host does not emit and that the error carries
  both the path and the command; plus a kotlinc compile+run of the
  *generated* skeleton with only its `TODO` body replaced — so the test
  proves the skeleton is right everywhere else — whose stdout matches the
  Rust run byte for byte. The `run_kotlin_entry` helper exists because
  `run_kotlin_files` hardcodes `salvo.main.MainKt`, and the entry here is
  the host's `salvo.platform.main.MainKt`); and 1 overload-specificity test
  ([fn-overload-rank]: `kotlinc_runs_the_most_specific_overload` —
  `concrete` then `generic`, with the generic overload declared first); and
  4 string tests ([kt-mut-str] [str-drop-mut] [fn-variadic]:
  `mut_str_lowers_to_a_string_builder` asserting the `StringBuilder`
  construction, `.toString()` at a call argument, in interpolation and at
  `==`, `StringBuilder(b)` for `copy`, and `set`'s guarded `setCharAt`;
  `a_spread_into_a_variadic_intrinsic_spreads` asserting Kotlin's own spread
  operator — the case that printed `[Ljava.lang.String;@…` before; plus
  kotlinc runs of the whole string surface and of `set` through a parameter
  and a field, both asserting the stdout the Rust backend asserts); and 2
  overload-override tests ([fn-overload-at] [fn-rename]:
  `scope_selectors_and_renames_are_erased` asserting that no `@` and no
  renamed name reaches Kotlin, that `@core.list` emits std's lowering, that
  the renamed overload is called by its declaration's mangled name, and that
  a call past a shadowing local needs nothing here (separate namespaces);
  plus the kotlinc run of that program); and 2 sequence tests ([kt-seq] [implicit-group] [implicit-intrinsic]:
  `sequence_functions_lower_to_collection_operations` asserting
  `.map{}.toMutableList()`, `.filter{}.toMutableList()`, `.fold(init, op)`,
  the resolved `next` passed as `::next` at a pass subject, the origin mint and
  its advance adapter, and that nothing *declares* `Yield`; plus the kotlinc run
  of the seven-subject demo).
- `salvo-backend-rust`: 176 - including five [rs-process] tests (the first
  asynchronous program compiled and run, printing the `sum 5` the Kotlin
  backend prints; the message enum, process body and mounted scheduler
  asserted on the generated text; a **dependent spawn** compiled and run —
  one dependency a construction, the other an addr — printing the
  `last bumped 3` / `sum 5` Kotlin prints; its flat provider, `__Deps_H`
  view and `__Impl_H` dispatch asserted on the generated text; and the
  generic-handler cut reported as a codegen error) - and the
  member-name-collision case [effect-available], which is a compile-and-run
  test precisely because one half of the defect it pins was silently wrong
  *output* - golden snapshots of the same five demos
  emitted as Rust; deduction-mode assertions
  (`deductions_drive_parameter_modes`: kept -> `&`, kept+Mut -> `&mut`,
  omitted -> move, matching call-site argument shapes [rs-borrows]);
  union-enum assertions (`unions_emit_enums`), predicate/mangling
  assertions (`qualifiers_lower_to_predicates_and_mangled_fns`),
  effect-trait assertions (`effects_lower_to_traits_and_mut_dyn_params`),
  loop-lowering assertions (`loops_lower_to_block_expressions`),
  crate-layout assertions (`crate_layout_mounts_only_used_modules`
  [rs-crate] [rs-imports]); numeric literal suffixes emit explicit types;
  predicate-qualifier constructors emit plain fns [qual-ctor-predicate];
  negative tests (missing defines for external
  fns/types, uncovered core externals, generic effect members
  [rs-effects]); `copy`-lowering assertions (`.clone()` on the
  argument's place, and fate-linked `let`s cloning instead of moving
  [rs-copy] [fate-link]); move-mode emission assertions
  (`move_mode_bindings_emit_real_moves` [fate-move-mode]: claimed
  parameter taken by value, loop by value, partial field move, move-mode
  `let` moving, pipeline clone-free); borrow emission assertions
  (`borrow_mode_bindings_emit_borrows` [rs-borrow-locals]: `&Vec`
  parameter iterated bare by reference, `&T` field binding, borrow
  alias of an owned local, read-only pipeline clone-free); `discard`
  lowering assertions (`drop(...)` [linear-discard]);
  general-sweep assertions (field-subject `is` lowering
  [is-narrowing], place narrowing [flow-place] (narrowed nullable field
  and field-chain reads unwrapping the `Option`, a narrowed wrapper-union
  field read using the arm accessor — plus the rustc run against the same
  expected stdout as Kotlin) and a narrowed field as an operator operand
  [op-no-none], union coercion inside array/tuple literals and lambda
  tail returns with fn-type `let` annotations dropped [fn-contract],
  type-directed dispatch for unchecked define overloads plus the
  ambiguity error [backend-never-wrong], and an aliased effect type
  resolving to the handler registered under the canonical type
  [effect-disambiguation], and the std array functions with the
  LANGUAGE.md `CyclicRandom` handler [type-array]); and twenty-five rustc
  compile+run tests with exact stdout assertions mirroring the kotlinc
  set (demo, unions, qualifiers, effects, loops, multi-module, copy,
  the S2 zero-clone move-mode demo, the S3 borrow demo, the L6 linear
  resource demo, the L7a linear-generics workflow demo, the L7b
  `once` demo — with `once_fn_params_emit_fnonce` asserting the
  `impl FnOnce` rendering [once-fn] — the L7c derived-returns demo,
  with `derived_returns_emit_borrows` asserting the elided and
  generated-lifetime signatures and the borrow returns
  [readonly-return], and the L7d contracts demo, with
  `fn_type_contracts_emit_modes` asserting `&mut impl FnMut`
  signatures with contract-mode argument types and the named-fn
  adapter [fn-contract]);
  type-argument assertions ([call-type-args] [backend-intrinsic]:
  `Vec::<i32>::new()` from an annotation and from an explicit type argument,
  with a rustc run);
  fusion assertions ([rs-effect-fusion]: the dependency absent from the
  struct and from `new`, member bodies in the generated `__Impl_H` trait,
  the fusion chaining through `__outer` and owning the handler, the
  disjoint-field-borrow destructuring, a generic fused parameter forwarded
  to a smaller callee, the two-dependency `__Deps_H` adapter, a
  conjunction trait with its blanket impl, UFCS dispatch for a generic
  effect, nested effect calls hoisted — plus
  `programs_without_handler_dependencies_do_not_fuse`, which pins the
  gate: no dependency anywhere means the per-effect `&mut dyn` parameters
  are untouched; and `generic_dependent_handler_is_a_codegen_error`
  [backend-never-wrong]); and five further rustc compile+run tests for
  the fusion (the handler-deps program Kotlin also runs, a stress program
  with handler state behind a dependency, a two-dependency handler,
  nested `use` scopes with the outer one reused, a `use` inside a fn that
  already has effects and inside a loop body, a dependency chain, a
  qualifier's `qualifies` effects, the `use` site in a different module
  from the handler, constructor parameters mixing a dependency with plain
  data, a dependent member calling a fn that does its own `use`, an
  effect-using lambda passed to an effect-free higher-order fn, three
  effects in one signature, a `use` in a `while` body, and an effect member
  taking a `Mut` parameter); and
  `effect_using_fn_value_at_an_effectful_call_is_a_codegen_error`
  [backend-never-wrong]; and 2 `defer` tests ([defer] [rs-defer-splice]:
  `defer_splices_at_every_exit` asserting LIFO order at a block end with no
  scaffolding, the hoisted `return` value, and the loop body's deferred
  code appearing at its `continue`, its `break` and the block end; plus the
  rustc run of the same demo Kotlin runs, with the same stdout); and 3
  throw tests ([throw] [try] [rs-throw-controlflow] [rs-try-label]:
  `throw_lowers_to_controlflow` asserting the `ControlFlow<M, T>` return
  shape, `throw` as a `Break` return, `Continue`-wrapped returns, no trait
  for the effect, and the deferred release on the throw path of a
  propagating call; `try_lowers_to_a_labelled_block` asserting the label,
  the *absence* of a closure, and the message wrapped into its union arm;
  plus the rustc run of the demo Kotlin also runs); and 3 fn-type-effect
  tests ([fn-effects] [rs-fn-effect-params]:
  `fn_type_effects_thread_into_closures` asserting the `&mut dyn` effect in
  the closure *type* and as a leading closure parameter, and the named-fn
  adapter forwarding or ignoring it;
  `rustc_compiles_and_runs_effect_using_fn_value`, which is the **lifted
  fusion cut** — the program this test used to assert *could not* be
  emitted; and the shared demo Kotlin also runs); and 2 subject-less
  `when` tests ([when-condition] [rs-when-cond]:
  `a_subjectless_when_emits_an_if_chain` asserting the
  `if`/`else if`/`else` chain, no `else { None }` filler in value position
  and no `unreachable!()` arm (the mandatory `else` makes it total), and
  the `is` binding declared inside its branch; plus the rustc run of the
  demo Kotlin also runs, with the same stdout); and 1 `try`-body test
  ([try]: the rustc run of the program whose `try` body assigns an outer
  variable and declares a local — the Rust half of the same traversal
  gap); and 2 generic-higher-order tests ([rs-fn-param-convention]
  [fn-contract]: `a_lambda_binds_a_generic_fn_parameter_by_reference`
  asserting the declared `FnMut(&T)` convention and the `|n: &i32|` /
  `|s: &String|` annotations that now follow it; plus the rustc run of a
  generic `map` in every argument form — bare lambda, annotated lambda and
  named fn, over a `Copy` and a non-`Copy` element type — which before the
  fix failed with `E0631` for the annotated forms only); and 3
  implicit-parameter tests ([implicit-param] [implicit-group]
  [backend-never-wrong]: `implicit_parameters_lower_to_trailing_fn_arguments`
  asserting the expanded trailing parameters, the adapter closure a resolved
  default is wrapped in, the `&mut *add` reborrow forwarding uses and no
  struct for the group; and the rustc run of the same source and stdout Kotlin
  asserts) + 2 fn-field tests ([rs-fn-field], roadmap R5: the `Rc<dyn Fn…>`
  field, the hand-written `Debug` printing `<fn>` and the `Rc::new` store, plus
  the rustc run of a hand-written composed pass storing its source *and* its
  callback — the same source and stdout the Kotlin backend asserts, which is
  what closed the divergence); and 2 effect-member-implicit tests ([implicit-param]: the trait
  method, its implementation and the call site rendered from one helper, with
  `dyn` for object safety; plus the rustc run of the shared demo); and 4 generic-drive tests
  ([iter-generic-drive] [iter-drive-in-place] [fn-effects]:
  `a_kept_pass_is_driven_in_place` (no local, no clone — the parity fix),
  `an_owned_generic_pass_is_closed_by_the_loop` (the pass *moved* into the local
  and the implicit `close` spliced after the loop),
  `a_machine_takes_a_generic_effect_handler` (`&mut dyn Random<i32>` as an
  ordinary machine parameter, the builder called without handlers), plus the
  rustc run of the shared demo for the stdout Kotlin asserts); and 3
  laziness tests ([rs-iter-lazy] [fn-iterator] [rs-generator] [yield-fn-origin]:
  `an_origin_lowers_to_a_state_machine` asserting the generated struct, the flat
  state dispatch and the *absence* of `SalvoIter`, `SalvoGen`, `async move` and
  `.await`; `a_stored_mint_is_not_closed_at_the_call`, the rule a lazy
  combinator forced — the hoisted mint with no `__close` after the call, because
  the callee drives it later; plus the rustc run of an *unbounded* origin that
  terminates because the consumer `break`s and an origin driven twice to prove
  it replays — the same source and stdout the Kotlin backend asserts);
  and 2 refinement tests  `unsupported_claiming_producer_shapes_are_refused` — nested claiming
  producer, value-position `for`, generic effect claim); and 2 refinement tests
  ([qual-refn] [qual-erasure] [qual-refn-conflict]:
  `rustc_compiles_and_runs_a_refined_program`, the same source and stdout the
  Kotlin backend asserts, with no `qualifies` call in the emitted `main`; and
  a suppressed conflict still emitting and returning its rendered warning
  through `emit_program_reporting`); and 4 platform tests
  ([platform-effect] [rs-platform-entry] [platform-tree]
  [rs-platform-host]:
  `platform_effect_emits_a_trait_and_a_host_entry` asserting the generated
  `trait`, the *absence* of a handler struct, `salvo_main` taking
  `&mut dyn`, the effect threaded into an intermediate frame, and the
  crate-root wiring — the `#[path]` mount of `platform/main.rs` as
  `platform_main` plus the `fn main()` that delegates to it;
  `platform_generate_renders_a_host_skeleton` asserting the skeleton's
  path, the unit struct, `impl crate::Telemetry`, the stubbed member and
  the `main` that calls `crate::salvo_main`;
  `a_missing_host_file_names_the_command` asserting that a platform program
  without a host does not emit and that the error carries both the path and
  the command; plus the rustc compile+run of the *generated* skeleton with
  only its `todo!` body replaced, asserting the same stdout Kotlin does);
  and 1 overload-specificity test ([fn-overload-rank]:
  `rustc_runs_the_most_specific_overload`, the generic overload declared
  first and the concrete one still chosen — same source and stdout as the
  Kotlin backend's `kotlinc_runs_the_most_specific_overload`, since the
  winner is the *checker's* choice and the two targets must agree on it);
  and 4 string tests ([rs-mut-str] [str-drop-mut] [fn-variadic]:
  `mut_str_is_a_plain_string` asserting that a drop renders *nothing*, that
  `mutable_str` borrows its parts rather than moving them, the
  byte-to-character correction in `index_of`, `set` reaching the generated
  trait, the `strings.rs` mount and import, and that the support file is
  absent when nothing needs it;
  `a_spread_into_a_variadic_intrinsic_is_the_collection`; plus rustc runs of
  the whole string surface and of `set` through a `&mut String` parameter
  and a field projection — the case the trait exists for — each asserting
  the stdout Kotlin asserts); and 2 overload-override tests
  ([fn-overload-at] [fn-rename] [rs-shadowed-call]: the same program as
  Kotlin's, asserting the erasure, the std lowering behind `@core.list`, the
  renamed overload's mangled name, and `crate::describe(7)` for the call
  past a shadowing local — E0618 without it; plus the rustc run asserting
  the stdout Kotlin asserts); and 2 sequence tests ([rs-seq]
  [implicit-intrinsic]: `sequence_functions_lower_to_helpers` asserting the
  `salvo_map`/`salvo_filter`/`salvo_reduce` calls with their `&place[..]`
  receivers, the adapter a *named fn* callback wraps in, the intrinsic
  lowering inside the implicit's adapter closure, and `seq.rs` present only
  when something needs it; plus the rustc run of the same seven-subject demo,
  asserting the stdout Kotlin asserts).
- `salvo-testkit`: 1 - the hygiene test
  (`stale_debug_objects_are_pruned`), which deletes orphaned `.rcgu.o`
  debug objects from `target/debug/deps` on every full run (see gotchas:
  macOS never collects them).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

- **When two passes answer one question from different tables, the narrower
  table must publish for *every* case.** The checker resolves a call with an
  **import-scoped** member table; the emitters ask a **program-wide** one, and
  `fn_over_member_calls` exists to bridge them — but it was only written where
  the member lost a contest *in scope*, which is the one case where the two
  tables already agree. Every case where they disagree (a member of an effect
  not in scope, an explicit `@module`) went unrecorded, and one of them emitted
  the wrong function silently. A "which did this resolve to?" record with a
  condition on it is a record with a hole exactly where it is needed
  (2026-09-15).
- **A rule about one direction of a lifetime asymmetry is owed in the other.**
  "Assigning into handler state is a store, not a link" shipped 2026-09-14 with
  a 2-versus-1 divergence as its evidence. The *read* direction — moving a
  value out of that same storage — was left alone for a day, and it produced
  the same divergence in the same shape, plus a duplicated linear obligation.
  When a rule is justified by "this thing outlives the call", grep for the
  other direction before closing it (2026-09-15).
- **A loud error can be the least of a defect.** The reported symptom was a
  raw rustc E0507 from one intrinsic; the same root cause was *silently*
  cloning through every ordinary consuming call and every `return`, where the
  clone existed and made the backends disagree. Before fixing the loud
  instance, look for the shape where the compiler already inserted the thing
  it was missing — that is where the wrong output lives (2026-09-15).
- **The exemption is part of the rule, and its absence is why nobody saw the
  bug.** `copy`-or-refuse on handler storage would have made `out.send(sum)`
  on an `Int` field an error, which is noise: a Copy scalar's copy is
  indistinguishable from a move. That same exemption is why the rule went
  unnoticed for a day — the first process handler's state was
  `sum: Int`. Write the exemption and the rule together, and treat "all our
  tests use scalars" as a coverage gap (2026-09-15).

- **A generic type parameter has to *say* what an auto trait gives a concrete
  one.** `SalvoProcess: Send`, and a non-generic `impl SalvoProcess for
  __Proc_H` proves it for free — the compiler derives `Send` from the fields.
  Making the process generic in its dependency instances
  (`__Proc_H<__D0>`) moved that proof out of reach: rustc wants
  `__D0: Send` written on the impl, and `'static` too once the value is
  boxed as `Box<dyn SalvoProcess>`. Any time a concrete generated type
  becomes generic, re-derive the *bounds it was getting silently*
  (2026-09-15).
- **The pair of names for one concept is where a program's own names can
  collide with the compiler's.** `__Prov_H` (the process's flat provider,
  named after a handler) and `__Prov_A_B` (the fusion's provider trait,
  named after an effect set) share a prefix and a Rust namespace, so a
  handler named like a sanitized effect set would define the type twice.
  It is *safe* only because a duplicate definition is a rustc error rather
  than a silent reuse — which is exactly the distinction the existing
  provider-trait collision check is built on, and worth checking whenever a
  generated name is derived from user text (2026-09-15).
- **Write the ordering test, not just the feature test.** A dependent spawn
  has two orders — the handler's declaration order and the clause's written
  order — and a matching table between them; a test that writes them in the
  same order proves nothing about the table. `use tally, Recording()` versus
  `use Recording(), tally` is the one-line check, and on Kotlin there is a
  *third* order (the fused class sorts its effects canonically) that only
  shows up if the first two differ (2026-09-15).
- **A deterministic concurrency test chains through one process, not
  several.** The temptation is to send to two processes and then ask both;
  arrival order is only a guarantee *per process*, so that races. What works:
  make everything travel through one mailbox, and get the second fact out by
  **forwarding the caller's own reply token** to whatever holds it — the
  token is a value, so a handler can pass it to a dependency and let the
  answer come from inside the child. Every later query is then ordered by the
  activation that already finished (2026-09-15).
- **A `Copy` state field hides a whole class of bug in a process test.** The
  first process test's handler state was `sum: Int`, so `out.send(sum)`
  copied; the first test with a `Str` field found E0507 immediately. When a
  surface's tests are all built on scalars, one non-scalar field is the
  cheapest coverage there is (2026-09-15).

- **Never write a Rust string literal through a shell heredoc.** Python eats
  `\`-continuations inside its own triple-quoted strings, so a diagnostic
  written that way reaches the *Rust* source as one long line with the
  indentation baked into the message. Worse, repairing it mechanically is a
  trap: a blanket "space before every continuation" corrupts the emitters'
  **code templates** (their strings are generated code), and a revert that
  drops the character before the space deletes `)` and `;` from them — caught
  only by the golden and compile-and-run tests. Write diagnostics with a real
  editor tool; if a repair script is wrong twice, restore the lines from `HEAD`
  by diffing rather than patching a third time (2026-09-15).

- **Trust nextest's summary line, not a hand-rolled count.** Summing
  `cargo test`'s `test result:` lines with awk once reported "0 failed" while
  two insta snapshots were failing, because a `test result: FAILED. 91
  passed; 2 failed` line does not parse like an `ok.` one. `cargo nextest
  run` prints one authoritative line (`964 tests run: 964 passed`), and it
  is also the run that shows which test failed without scrolling
  (2026-09-15).
- **Adding a field to an AST node is a snapshot sweep.** `TypeDecl` gaining
  `linear` changed 14 checked-in parser snapshots by exactly one
  `linear: false` line each. Accept with `INSTA_UPDATE=always` *after*
  reading a diff, then confirm the whole diff is only what you expected
  (`git diff <snapshots> | grep '^[+-]' | sort | uniq -c` makes that one
  glance).

- **A new `Expr` variant is nine classification decisions, not one.** Adding
  the asynchronous expression forms (2026-09-15) made the compiler stop
  building in nine exhaustive matches, and each is deliberately exhaustive
  because a wildcard there is a *wrong-code* bug rather than a missing
  diagnostic: `collect_assigned_expr` (narrowing reset),
  `expr_mentions` (same-call use-after-move), `expr_exits` / `expr_returns`
  (must-return), `lends_of_expr` (what a value borrows), the deduce walker
  (consuming calls), `expr_names` (unused-import/unused-fn census), and each
  emitter's `collect_mutated` / `collect_declared` / `block_terminates`.
  Answer them from the form's meaning — does it transfer control, can it
  yield a view, can it declare a name — and let the build tell you where
  they are; grepping for an existing simple variant (`Expr::Try`) finds the
  same set.
- **A half-built form must be refused on both sides.** A form that parses,
  type-checks clean and emits nothing is exactly the failure
  [backend-never-wrong] names. The pattern that worked: one checker
  diagnostic naming the form with `Ty::Unknown` as its type, plus a refusal
  arm in both emitters for the case where emission runs on an
  already-rejected program — and tests asserting the refusals, written to be
  deleted by the slice that implements the form (2026-09-15).

- **A Kotlin `vararg` of an unsigned type is an experimental array.** The
  `SalvoBytes.of(vararg elems: UByte)` constructor compiled fine, and then
  every *generated call site* warned that `UByteArray` "needs opt-in"
  (`@ExperimentalUnsignedTypes`) — because a vararg parameter of an unsigned
  type *is* a `UByteArray` under the hood. A shipped runtime class must be
  written so its callers need no annotations: the parameter is an
  `Array<UByte>` now (2026-09-15). The general rule: check what a runtime
  helper's signature forces on the code the compiler *emits*, not just on the
  helper.
- **A pass may not recycle its own buffer.** `chunks(s, size)` was tempting to
  write with one reused buffer in the pass struct — and it would have handed
  every step an alias of that buffer, so the next `next` would overwrite what
  the caller was still holding. A pass yields a **fresh** buffer per step and
  the allocation-free shape is `read_to` into a buffer of the caller's own
  (which is what `copy_stream` does internally). Recycling is safe only where
  the recycler owns the loop (2026-09-15).
- **Fill-a-buffer APIs should append, not overwrite.** The C shape — pre-size a
  buffer, fill `0..n`, return `n` — needs a `set`-by-index surface, forces
  every caller to carry `n` beside the buffer, and would have made `MemFs`
  rebuild its store to fake it. Appending makes `size(buf)` the truth,
  `clear(buf)` the reuse, and `write_bytes(w, buf)` the write-back with no
  ranged write, and it is implementable in pure Salvo with `append` alone
  (2026-09-15).

- **A cast in generated Rust needs its *source* type named, not just
  parentheses.** The conversion intrinsics emitted `({} as u8)`, and `as`
  binds tighter than unary minus, so `to_byte(-1)` became `-(1 as u8)`;
  adding parentheses around the operand did not help, because rustc then
  infers the unsuffixed literal's type *from the cast* and reports "cannot
  apply unary operator `-` to type `u8`" (E0600). The fix is
  `(((x) as i32) as u8)` — spell the parameter's own type in the middle.
  A signed target hides this: `-1 as i64` compiles and even means the right
  thing, so the whole family was latently wrong and only the unsigned
  addition made it visible (2026-09-14).
- **A specialized representation cannot coexist with erased generics.**
  Kotlin's `UByteArray` was to be the "honest" lowering of `List<Byte>`, and
  it cannot be one: std's list surface is generic (`size<T>(List<T>)`,
  `get`, `add`, `iter`, `map`), a `UByteArray` is not a `List<T>`, and this
  backend does not monomorphize — so the specialized value could not be
  passed to any of it (kotlinc 2.4: *argument type mismatch: actual type is
  'UByteArray', but 'List<T>' was expected*). Before promising a
  representation change as "just a rendering", check whether the value has
  to flow through generic code on that backend. Rust needed nothing at all
  for the same rule, which is what made the asymmetry easy to miss
  (2026-09-14).
- **A fake's *representation* is part of its fidelity, not an
  implementation detail.** `MemFs` stored files as `Str` and converted
  offsets through `byte_size` — arithmetically correct, and still unable to
  hold the first file `write_bytes` was asked to write. Storing bytes made
  two behaviors match the host that had been *approximated* before (a
  mid-codepoint ranged open succeeds, with the decode failing afterwards;
  a decode failure is recorded and re-reported by `close`). When a double
  exists to be byte-exact, store bytes: converting at the boundary is a
  smaller promise than it looks (2026-09-14).
- **`include_dir!` does not notice a *new* file in `std/`.** The embedded
  standard library is `include_dir!("…/std")`, and cargo re-runs it only when
  a crate source changes — so adding `std/core/x.sv` (or a
  `std/platform/…` host companion) and rebuilding produces a binary that has
  never heard of it, and the failure reads like a language bug ("no function
  named … is in scope"). Touch a file in `salvo-cli` to force the rebuild.
  Found 2026-09-14 while verifying the std route of [platform-handler];
  whoever ships `std/platform/core/fs.*` meets it first.
- **"A file exists" is not evidence of what is *in* it.** Kotlin's
  `entry_hint` decided the JVM launch class from the presence of an emitted
  `platform/<M>.kt`, which was equivalent to "the host owns `main`" only
  while a host file existed for one reason. `platform handler` gave it a
  second reason, and `salvo run` launched a class with no `main`
  (2026-09-14). A predicate standing in for a fact stays correct only as long
  as nothing else can make it true: prefer asking the thing itself — here the
  *generated* module declaring `salvoMain`, which is the emitter's own marker
  for the entry having moved, and not customer-written text.
- **An effect environment is a scope, not a set — and "same effect twice"
  is the case that proves it.** Both emitters and the checker had lookups
  that took the *first* matching entry, which was indistinguishable from
  the last while only one registration per instance existed. The moment
  shadowing became legal ([use-no-dup], interception), the Kotlin emitter
  answered member calls with the **outer** handler — silently, and only on
  that backend (2026-09-14). Rust was already innermost-first, so the
  divergence was invisible until the same program was run on both. When a
  rule relaxes uniqueness anywhere, grep every lookup of the thing that was
  unique and check its *direction*: a set-shaped read of a stack is a bug
  the tests cannot see until a duplicate exists.
- **Two impls of one generated trait on one generated type is the fusion's
  failure mode.** Rust's fusion emits an accessor impl per effect in scope;
  a shadowing `use` made two `__Has_E` impls for one struct (`E0119`).
  Suppressing the *inherited* one is right, but only because the shadowed
  instance stays reachable through `__outer` — which is what the
  intercepting handler's own dependency binds to. Emission for a shadowing
  construct has to keep the shadowed thing reachable *somewhere*, or the
  wrapper has nothing to wrap (2026-09-14).
- **A generated trait impl and a generated forwarding body can disagree
  about parameter modes.** The fusion's `impl Effect for <fusion>` renders
  its signature from the *effect's* member declaration, where a parameter
  of the effect's own generic type is borrowed; `__Impl_H` renders from the
  *handler's*, where the same parameter may be a Copy scalar passed by
  value. Any place two independently-rendered signatures meet needs the
  bridge written explicitly (here a deref) — it had been an `E0308` since
  the fusion landed, reachable by any dependent handler of a generic effect
  instance (2026-09-14).
- **A green e2e test can hide a misparenthesized emission — shape-assert
  the exact text when an emitter inserts casts.** The first promotion cast
  emitted `(n * 2 as i64)`, which Rust parses as `n * (2 as i64)` because
  `as` binds tighter than every arithmetic operator — and the e2e test
  *passed*, because `n` was un-annotated and inference bent it to `i64`.
  The compile-and-run test proved "some program with this text runs", not
  "the cast means what the checker recorded"; the shape assertion
  (`((n * 2) as i64)`) is what caught it (2026-09-14). Emit
  `((operand) as T)` with the operand parenthesized, always.
- **A scope's effect environment must be restored by *clone*, not by depth
  truncation, once anything mutates entries in place.** The Kotlin emitter
  truncated `effect_env` back to its depth at block exit, which was
  sufficient while scopes only *pushed*; the Has-fusion `use` sites also
  **rebase** the enclosing entries onto the new fused value, and a
  truncate leaves that mutation behind — the enclosing scope then
  dispatches through a variable that no longer exists (`unresolved
  reference '__fx4'`, found by kotlinc in the e2e batch, 2026-09-14). The
  Rust emitter never hit it because it had always saved and restored full
  clones. If an emitter grows any in-place mutation of scoped state, audit
  every save/restore of that state for the truncate pattern the same day.
- **A new runtime module must be added to `runtime_tests.rs`'s list.** That
  list is what makes the test complete rather than a sample, and nothing
  fails if you forget: the module is still spliced into user output, just
  never compiled on its own, so a warning or syntax error in it surfaces as
  a failure in some unrelated end-to-end test. `collections.rs` and
  `compare.kt` were both missing until an audit caught them
  (2026-09-12). The same audit is worth running for **rule labels**:
  `grep`ping every `[label]` in code against the specs found two rules
  referenced 30-odd times that had never been written, which AGENTS.md
  calls a bug.
- **Kotlin cannot copy a value behind a type parameter.** There is no
  generic `copy()`, so any design that needs to *own* a `T`/`V` it read out
  of a container is dead on arrival for backend parity — Rust would clone
  and Kotlin would share identity, aliasing mutable values on one backend
  only. This is what forced Map passes to yield **keys** and Set/Map passes
  to be snapshots owning a `List<T>` (2026-09-12). Reach for a snapshot
  before reaching for a copy.
- **An intrinsic type cannot host a borrowing pass today.** The emitters
  render a `proj[from: p] T` field as plain `T` when the owning struct does
  not otherwise borrow, so an intrinsic pass built on [proj-field]
  lifetimes over the native iterators fails on its own yield type. Worth
  knowing before designing the next container's iteration.
- **Kotlin's linked containers have no `(size, init)` constructor.**
  `List`/`Array` do, which is why `list_by`/`array_by` lower to one
  expression; `set_by`/`map_by` must lower to an `also`-scoped *builder*
  over `(0 until n).map(f)` instead. Expect the shape of a `*_by` lowering
  to differ per container, not per backend.
- **A "test result" sum does not include insta failures.** The
  `^test result` lines are what a pass/fail tally is usually scraped from,
  and a snapshot mismatch does not appear in them; also
  `grep -cE "FAILED|panicked at"` before believing a green count.
- **Two concurrent `cargo run`s deadlock on the build lock.** Comparing the
  backends with process substitution (`diff <(cargo run … rust) <(cargo run
  … kotlin)`) hangs; run them one after the other.
- **The std module names are load-bearing.** `core.list` appears in user
  syntax (`f@core.list(x)`), so a module rename is a language change, not a
  file rename. Renaming the *functions* inside it was the whole of
  increment A; renaming the module was never on the table.
- **Salvo has no `\u` escape, and no string literal nested inside
  `${…}`.** Both bite when writing test programs that print exotic
  characters or compute a message inline; hoist the inner string into a
  `let`.
- **macOS `grep` has no `-P`, and zsh does not word-split unquoted
  variables.** Reach for `perl -ne` for lookarounds, and
  `find … -print0 | xargs -0` rather than building a command string.
- **`SIGKILL (signal 9)` on a freshly built test binary is macOS, not the
  test.** AMFI (the kernel's code-signing enforcement) sometimes rejects a
  just-linked binary — the log says `has no CMS blob? … Unrecoverable CT
  signature issue` (`log show --last 10m --predicate 'eventMessage CONTAINS
  "AMFI"'`) — and once an inode is flagged, retries on it keep dying until
  the file is replaced. Under nextest it surfaces as `creating test list
  failed … aborted with signal 9`. Re-touch the affected crate (or `cargo
  clean`) to force a re-link; re-running alone may pick a different victim
  (2026-09-12: three different binaries in one afternoon).
- **`target/debug/deps` accumulates `.rcgu.o` files forever on macOS.**
  `split-debuginfo=unpacked` (the platform default) keeps every codegen
  object as the debug info of its binary, and cargo never garbage-collects
  the sets orphaned by rebuilds: 790k files / 48.7 GiB after a few weeks,
  slowing every directory scan and possibly implicated in the AMFI kills
  above. `salvo-testkit`'s hygiene test now prunes orphans on every full
  run; if `deps/` is somehow huge again, `cargo clean` resets it.
- **An empty doctest pass is not free.** `cargo test` runs rustdoc over
  every library crate to *collect* doctests even when there are none —
  ~7s per crate here, uncached, every run; it was 38s of a 54s warm suite.
  The library crates set `[lib] doctest = false`; remove that if a doc
  example should ever run as a test.
- **`kotlinc -version` costs a JVM start (~1.5s), and process-level caches
  do not help nextest.** nextest runs every test in its own process, so a
  per-process probe cache re-pays the JVM per *test* — the probe is
  disk-cached (keyed on the resolved kotlinc binary). The same JVM cost is
  why the compile-and-run tests batch: one `kotlinc` invocation per test
  was ~800 CPU-seconds of mostly startup, fresh.
- **`cargo test` stops at the first failing test binary.** A run that shows
  "one failure left" may be hiding failures in every crate after it; the
  binaries run in alphabetical order, so a red `salvo-backend-rust` hides
  `salvo-cli`, `salvo-core` and `salvo-syntax` entirely. Check with
  `cargo test --no-fail-fast` before calling anything green (2026-09-11: a
  respelling that "left one test" had 35 more behind it).
- **A lending iterator cannot own its source.** If a pass *owns* its list,
  `next(&mut p) -> &T` ties the element to the `&mut` of the pass, and storing
  an element (what a view must do) is E0499. The pass has to *borrow* the
  source (`items: &'s Vec<T>`) so elements are `&'s T` — which is why
  [proj-field] exists and why an `iter fn` pass borrows its subject. Found by
  writing the Rust by hand and compiling it before touching the emitter.
- **"proj stripped at lowering" has a price.** Keeping `proj` out of `Ty`
  keeps generics from binding `T = proj Int`, but two Salvo values of the same
  type can then have different Rust types (`Union2<&T, F>` vs `Union2<T, F>`).
  Every place a borrowed value flows into an owned-typed position needs an
  adapter or a refusal [rs-proj-arm]; the fate link's `held`/`borrowed` flags
  are the checker's only record of the difference.
- **A generic body cannot see that an element it stores is borrowed**, because
  the pass is opaque (`it: Mut It`). Std says it in the signature
  (`=> proj[from: it]`); a user fn that does not is caught by rustc, not the
  checker. Recorded in ROADMAP.
- **Inference changes what a migration means.** Converting `-> [] T` (consume
  everything) to a clause that mentions nothing turns a `consume(x) {}` test
  helper into a *keeping* fn: every test that asserted a move went quiet. A
  spelling change that also changes a default needs a second pass restoring
  the old facts explicitly (a second throwaway script read them off the git
  diff).

- (rs-narrow-mut) **A read helper reused at a write site is a correctness bug,
  not an inefficiency.** `narrow_unwrap` produces an owned temporary, which is
  right for every read; borrowing it `&mut` compiles and mutates the temporary.
  Rust makes this class silent because `&mut <rvalue>` is legal. When a helper's
  doc comment says "the result is owned", every mutable caller needs a twin —
  and the twin must render the whole *chain* mutably, or a middle link becomes
  the temporary instead.
- (rs-narrow-mut) **"Is a representation recorded for this span" is not "is this
  span narrowed".** Keying the new mutable path on `repr_ty.contains_key` caught
  `Mut` drops too, so an ordinary `Mut` argument lost its `&mut` — five golden
  snapshots and a refinement e2e test failed at once. Gate on the unwrap
  *returning* something.
- (rs-narrow-mut) **A one-backend bug is found by running both, not by reading
  either.** The checker accepted the program, rustc accepted the output, and the
  Rust program printed a plausible answer; only the Kotlin run disagreed. The
  parity principle is a *test procedure* as much as a design rule — a demo whose
  output is asserted identical on both backends is the only thing that catches
  this class.
- (rs-narrow-mut) **`awk`-summing `cargo test` output hides failures**: a
  `test result: FAILED. 68 passed; 2 failed;` line shifts the fields relative to
  an `ok.` line, so a script that sums "the sixth field" reports zero failures
  while two tests are red. Grep for `FAILED` explicitly.
- (iter-fn) **The span-allocator gotcha below has a second instance, so it is a
  pattern rather than an anecdote**: `pass_field` gave the synthesized `__p`
  base the read's own span, and the side table found the field's narrowing for
  it. The allocator to fix it already existed — written for the first instance —
  and simply had not been wired to this constructor. When a desugaring helper
  takes a `span` parameter, ask which of the nodes it builds may legitimately
  share it.

- (qual-group) **A subtype rule and the coercion beside it must be gated on the
  same predicate.** `is_subtype` decides *whether* a value may enter a union
  arm; `coerce_repr` decides *how* it is wrapped. Widen one without the other
  and the checker accepts a shape the emitter wraps wrongly — which is how the
  plain-arms group (`Emitted (Str | Int)`) came to type-check and emit one wrap
  where two were needed. Both now go through
  `types::nested_group_remainder`/`nested_group_arm`, restricted to a *wrapper*
  inner union, so a shape without a physical inner arm is an error rather than
  output.
- (qual-group) **"Both halves pass, so the fault is in the combination" can be
  wrong about which combination.** The defect's two probes were
  `Emitted (Str | Int)` and `Ok Str | Err Str`, and the second was indeed fine —
  but the *first* was quietly broken too, at emission rather than in the checker,
  so rustc was the only thing reporting it. When a probe is declared fine
  because `analyze` is silent, run it end to end before building a theory on it.
- (qual-group) **A normalizing representation deletes information you may later
  want.** `Ty::Qualified` sorts and deduplicates its qualifier list, which makes
  `Ok Ok Str` *equal* to `Ok Str` — not "hard to distinguish", gone. Rules can
  recover a flattened *distinct* qualifier from the expected type; nothing can
  recover a deduplicated one. Worth knowing before adding a normalization: the
  cheap invariant costs a case you cannot get back.

- (iter-fn) **Cut by function, not by region.** Deleting the Rust emitter's
  "iterator functions" section took `rust_fn_name`, `enter_generics`,
  `emit_return_type` and three more with it — they happened to live under that
  heading. The compiler caught it, but a section marker is not a scope, and the
  restore cost more than the delete.
- (iter-fn) **A struct field that looks dead may be the only path a side table
  has.** `Viable::pairings` was reported as never-read once the code beside it
  was gone; it carried the per-argument *coercions*, so removing it silently
  stopped `[str-drop-mut]` from firing at call arguments — caught four files away
  by a Kotlin string-builder assertion. When the compiler says a field is unread,
  check what stopped reading it.
- (iter-fn) **An assertion on a substring of generated code can start matching
  something else.** `!src.contains("_pass = it")` was pinning "a kept pass is not
  bound into a local"; once a mint appeared it matched `__loop2_pass = iter__4(…)`
  and failed for the wrong reason. Name the loop.

- (implicit-infer) **A learning pass that only runs *between* arguments cannot
  learn from the arguments.** `extend_subst_from_implicits` existed, was correct,
  and was called in the one place where the variable it needed (`C` in
  `?iter: (c: C) -> Mut It`) was still unbound — so it silently learned nothing
  and the failure surfaced two implicits later as "`next` is ambiguous". When an
  inference step reports nothing, check *when* it runs before doubting *what* it
  does; the fix here was one extra call after the argument loop, plus repeating
  the sweep to a fixpoint.

- (iter-fn) **A desugaring must give every synthesized node its own span.** The
  checker's side tables are keyed by span, so two generated declarations sharing
  a name span silently share their `fn_effects` entry, and two generated
  expressions sharing a span share their type. Both happened within an hour of
  each other while building [iter-fn] — the generated `iter` inheriting a
  `[Console]` it never declared, then a `copy` argument typed as the struct
  literal beside it. The fix that scales is a span allocator over the original
  declaration's byte range: unique by construction, and every span still points
  at real source, so an escaped diagnostic lands in the right place.
- (iter-fn) **A table the checker fills and nobody reads is a defect waiting.**
  `PassDriver::mint_iter_fn` was recorded from R5 onward and read by neither
  emitter, so `for x in bag` over a container of one's own emitted a drive of the
  container. `grep` for a field's readers when adding one; a `Checked` field with
  no consumer is either dead or a hole.
- (iter-fn) **Desugaring into declarations the checker already supports is the
  cheapest way to add a form.** `iter fn` needed no checker rule, no emitter
  rule, and no `Checked` field: expanding it in `parse_module` bought `for`,
  `iter`, the combinators, deductions and narrowing at once. The contrast with
  `yield fn` — whose machine needed a planner, two renderers and a per-effect-set
  protocol — is the argument for trying the desugaring first.

- **(generic drive) A convention that reads a *place* has to ask who owns it.**
  Binding a `for` subject into a local is right for a pass the body owns and
  wrong for one it keeps: for a `&mut` parameter the "bind" is a clone, so the
  loop advanced a copy and the caller's pass stood still — on Rust only, because
  Kotlin's local aliased the same object. Two backends, two answers, no
  diagnostic. The general lesson: whenever a lowering *copies* something to work
  on it, the question "would the caller notice?" has to be asked in the checker,
  where ownership is known, not in the emitter.
- **(generic drive) A "fate link" is about projection, not about provenance.**
  The loop binding of a pass drive comes *out of* `next` by value, so linking it
  to the pass made std's own `filter` illegal ("cannot move `x`"). A binding
  links when it is a *view* of something still alive (a list element), not merely
  because the thing it came from is still alive.
- **(lifting a refusal) A refusal outlives its reason.** The generic-effect
  refusal named the per-effect-set trait's naming scheme; that scheme had been
  deleted two hours earlier and the refusal stayed, with its message explaining
  a mechanism that no longer existed. When a mechanism goes, grep for the
  diagnostics that *justified themselves by it* — they are as much a part of the
  deletion as the code.
- **(lifting a refusal) The thing behind a refusal is often a second bug.**
  Removing the generic-effect check surfaced handlers being threaded into the
  *builder* call (`rolls(random_int, 3)`), because the drive site's effect check
  had always recorded `call_effects` at the subject's span — invisible while the
  only subjects that claimed effects were producer calls, which were suppressed
  for another reason.

- **(R5) A convention flag read from the deduction list is not the same as
  "the callee stores it".** Rust renders a callback owned (`impl Fn + 'static`)
  when the callee keeps it past the call, and "moved by the deduction list"
  looked like the right trigger — a moved value is one the body keeps. It is
  not: `apply(f: (v: List<P>) -> Int, data: List<P>) -> Int => data` moves `f`
  and merely *calls* it, so the whole convention changed under a passing test.
  What holds is the *shape of the result*: a fn-typed parameter plus a return
  type that is a struct with a fn-typed field. When a backend convention needs
  "does this outlive the call", look at what the fn *builds*, not at what it
  consumes.
- **(R5) Release belongs where the driving ends, not where the call returns.**
  A mint hoisted at an argument position was closed after the call — right for
  an eager combinator that drains the pass, silently wrong for a lazy one that
  stores it: the loop then drove a *finished* machine and printed nothing. The
  symptom is the worst kind (no output, no error), and the rule that fixes it
  is the callee's deduction: close only what the callee **keeps**.
- **(R5) Two `move` closures reading one local is legal Salvo and E0382 in
  Rust.** An immutable read is free by the parity rules, so nothing in the
  checker objects; ownership is Rust's problem, and the fix is the emitter's —
  clone the captures into a block around the closure. A rule that is "free" in
  one language is a rendering obligation in the other.
- **(R5) Kotlin cannot infer a callee's type argument from two union-arm
  branches.** `{ __p -> if (…) U2_1<Int, Finished>(…) else U2_2<Int, Finished>(…) }`
  leaves `T` unresolved ("cannot infer type for type parameter 'T'"); an
  anonymous *function* with a declared return type pins it. Where an adapter's
  type has to be known, declare it rather than build it out of arms.
- **(R5) Synthesized code needs imports the source cannot ask for.** A driving
  loop spells `Finished`, which the user's file need never mention, so
  reachability-by-name misses it. Any lowering that names a std type has to add
  that module itself — and the test that catches it is an e2e compile, not a
  shape assertion.
- **(R5) "Iterating consumes the container" is the flip's one user-visible
  cost.** `iter(xs)` moves `xs` into the pass, so walking the same container
  twice needs `copy`. It surfaced first in the *test demos* (`reduce(iter(arr),
  …)` then `map(iter(arr), …)`), which is a good sign the diagnostic is clear —
  but it is worth stating in the language docs, since the `for` form does not
  consume and the asymmetry is otherwise surprising.

- **(R4 part 2) "Refuse the store" needs two different predicates, and mixing
  them up breaks std.** A declaration-site check must be *blind* to
  `<T canbe linear>` type parameters — `add(list: Mut List<T>, elem: T)` is a
  legal signature, generic over a `T` that may be linear — while the call-site
  check must not be, since instantiating that `T` with a linear type *is* the
  store. Treating an opted `T` as linear at declaration sites made std itself
  illegal (nine errors inside `core.list` and `core.array`), which is a fast
  signal: if a new rule fires in std, the rule is being asked at the wrong
  place.
- **(R4 part 2) Reading a composite is not storing into one.** The first
  call-site rule refused any callee mentioning `T` inside a composite, and it
  rejected `size(list)` inside `count<T canbe linear>(list: List<T>)` — a
  read. The rule that holds needs *both* halves: the callee takes a **bare**
  `T` parameter and mentions `T` inside a composite. Signature shape is the
  only evidence available at a call site, so distinguish "hands a value in"
  from "looks at a container".
- **(R4 part 2) A contagious property costs a second diagnostic per mistake.**
  While a composite containing a linear value was itself linear, one refused
  store produced two errors: the store, then a leak for a container that was
  never built. Dropping contagion when the store became illegal removed the
  follow-on error everywhere (a CLI fixture went from six errors to five). When
  a rule turns "X may contain Y" into "X may not contain Y", delete the
  propagation as well or the diagnostics double.

- **A fast path can hide the slow path for months.** `map`/`filter`/`reduce`
  each have a `List` overload that overload specificity always picked, so the
  *generic* spread body — the one the whole "iterable is a function, not a
  trait" design rests on — had never been emitted for a real call. Adding
  `map_to` (no fast path) ran it for the first time and three long-standing
  defects surfaced at once. When a declaration exists in two forms and one is
  always chosen, the other is untested by construction: write the test that
  reaches it deliberately, or add a shape that has no fast path.
- **The same bug in both backends is a signal about *where* it lives.**
  `return None` emitted `return null` on Kotlin and `return None;` on Rust —
  both target-language type errors, both found by one line of std. Two
  independent emitters getting the same thing wrong the same way means the
  question ("does this fn return anything?") was never asked at the right
  level; each had the answer locally (the rendered return type is empty) and
  neither consulted it.

- **Plumbing that fixes nothing observable is still worth doing — and worth
  labelling.** The release path for a *pure* producer closes a hole with no
  reachable symptom: after [iter-mut-param] a pure producer's `defer` can only
  touch state that dies with the pass, and a `defer` that performs an effect
  makes the producer claiming, which was already handled. Every route to
  observing it goes through a shape that is refused. That is a reason to build
  it (it is the protocol the un-boxing half needs, and it is correct in advance
  of the cases that will need it) and *not* a reason to write it up as a bug
  fix. Recording "no observable change" is what stops the next reader looking
  for the test that proves it.

- **A test suite cannot catch a shape nobody writes.** A producer's callback is
  declared `impl Fn + 'static` and was emitted as a *borrowing* closure, so
  every capturing lambda handed to an iterator fn failed to compile on Rust
  (E0373) — for months, with 743 tests green, because not one of them passed a
  capturing lambda to a producer. The `'static` in the rendered *type* had been
  there since [rs-iter-lazy] was written; the `move` on the closure never was.
  Two lessons, and the second is the useful one: a hand-written prototype
  proves the shape you thought of, and the gap was in a shape nobody thought
  of — so when a rule says "this position is `'static`", the test that earns it
  is the one where something is actually *captured*.
- **Write the remedy the diagnostic names, then compile it.** The
  callback-capture rule's first message said "capture `copy(x)` for a
  snapshot", which is wrong twice over: the copy happens inside the closure so
  the capture is still there, and a copy of a `Mut List` is still `Mut`. The
  advice that works is "bind a snapshot at a non-`Mut` type *before* the
  lambda" — and finding that out is what exposed the missing `move`, because
  the working remedy did not compile either. A remedy is a claim about the
  compiler; check it like one.

- **A checker table's *purpose* is not its shape.** Two tables I4's emission
  read said what they were for and had to be read against the grain anyway.
  `fn_effects` of a producer contains its claim — because that is the
  environment its *body* is checked in — so rendering the signature from it
  gave the factory a handler parameter no call site passes. And `call_effects`
  at a `for` subject's span holds the *drive* site's handlers (a synthesized
  loop has no call node of its own), which is the same span the argument logic
  reads when the subject is a call, so a producer call was handed a handler it
  does not take. Both cost one guard, and both looked like the emitter's bug
  until the table's comment was read. When a table is keyed by a span that two
  constructs share, say which construct owns it.
- **When a claim is a representation, it is not a qualifier any more.** Every
  qualifier before this one *erased* on both backends, so both emitters had one
  line each saying so (`let _ = quals;`). An effect claim on a producer is
  written where a qualifier goes and behaves like one everywhere except the one
  place that matters: `Console Iter<Int>` is a different type from `Iter<Int>`.
  The tell that this was going to cost something was in the prototype, in the
  form of the variance adapter — a "widening" that has to emit code is a
  representation change wearing a qualifier's clothes.
- **Two backends reaching one behaviour by different mechanisms is a feature,
  not a smell.** The injected `close` had to run on `break`, `return` and
  exhaustion. Rust splices the call at each exit and registers it as a deferred
  entry so a `return` picks it up; Kotlin wraps the loop in `try`/`finally` and
  gets all three for free — because that is how `defer` is already lowered
  there. Forcing one shape would have meant either a `finally` Rust does not
  have or a splice list Kotlin does not need. What must match is the *output*,
  which is what the shared program and shared expected stdout test.
- **A generated support file that mentions other modules wants no imports at
  all.** `iter_effects.rs`/`iter_effects.kt` are the first generated files with
  cross-module references, and the question "how do they import the effect
  traits" had an answer that removed the question: name them in full
  (`crate::core_console::Console`, `salvo.core.console.Console`). Both
  languages accept a fully-qualified type anywhere, so there is no import
  ordering, no alias collision, and nothing to keep in sync with the module
  emitter.

- **Adding a file to `std/` does not always reach the CLI.** The embedded
  standard library is `include_dir!("$CARGO_MANIFEST_DIR/../../std")` in
  `salvo-cli/src/analysis.rs`, and there is no `build.rs` emitting
  `rerun-if-changed` for that tree — so a *new* `.sv` file can leave the
  binary compiled against the old set. The tell is the file count in the
  CLI's own output ("analyzed 12 file(s) (1 user, 11 std)"): if it did not
  go up, touch `analysis.rs` and rebuild. Found while adding
  `std/core/iterator.sv` (2026-09-07); editing an *existing* std file is
  fine, since its content is part of the macro's input.

- **Probe the *current* behaviour before designing the rule.** The overload
  agenda was worth more than the design discussion that followed it: twelve
  five-line programs answered questions nobody had chosen an answer to —
  fixed-vs-variadic by declaration order, an own-module fn losing to std's
  *silently*, effect member calls checking nothing. Writing the options down
  from the code would have missed all three, because the code looked
  reasonable; only running it showed what it did.
- **A ranking that sums per-argument scores is a guess in disguise.** The old
  scoring added +2/+1/+4 per argument and picked the maximum, so a candidate
  could win by being much better in one argument and worse in another. Nobody
  noticed until the rule was written down as "at least as specific in every
  argument", at which point the sum was obviously the wrong shape. When a
  comparison is a *lattice*, implement the lattice, not a scalar projection
  of it.
- **A type declared in std is not the same as a type the compiler knows.**
  `Any` was `intrinsic type Any` and lowered to a nominal `Named("Any")`, so
  `f(v: Any)` accepted *nothing*: unification compares names, and no argument
  is named `Any`. It had been that way for as long as `Any` existed, hidden
  because nobody wrote an `Any` parameter. If a type has language-level
  meaning ([type-any-nothing]), the lowering has to say so — the std
  declaration only gives it a name.
- **Making something reachable creates new emission cases.** `@module` let a
  call reach a function shadowed by a local, which had been *unreachable*
  before — and immediately produced E0618 on Rust, where functions and locals
  share a namespace. A feature that removes a restriction should be followed
  by the question "what did the restriction make impossible in the output?"

- **A "free" widening can stop being free when a backend disagrees.**
  `Mut T <: T` had been one line in `is_subtype` because the only
  `canbe Mut` type mapped to a Kotlin *subtype*. `Str canbe Mut` broke that
  in the direction that shows least: equality. `sb1 == sb2` compiles, runs,
  and answers `false` where Rust answers `true`. When adding a `canbe Mut`
  type, ask what the *drop* costs on each backend before asking what the
  mutators cost [str-drop-mut].
- **One expression, one coercion slot — so a new coercion has to say what
  it displaces.** `DropMut` was first recorded by *removing* whatever was
  already at that span, which quietly discarded the union wrap a `Mut Str`
  needs on its way into a `Str | Int`. The `then` field is not extra
  generality: without it the emitters would have to re-derive the wrap.
- **Rust closure inference decides the shape of a lowering.** A closure
  bound to a `let` cannot infer its parameter types, and neither can one
  nested inside another closure's argument — so of all the ways to splice a
  callback into an expression, *none* works without an annotation the
  emitter does not have. A generic function parameter is an expected type,
  which is why the sequence fast paths are generated helpers [rs-seq]. The
  general rule: when a lowering has to *call* a value the program supplied,
  give it a typed home rather than an inline one.
- **"No new mechanism needed" is a hypothesis, not a finding.** The S-Seq
  memo said the generic half worked with today's machinery, having probed
  the shape by hand; building it needed four new mechanisms (see the
  roadmap entry). What the probe had actually shown was that the *syntax*
  parsed and one hand-written case checked. Probe with the case you intend
  to ship — here, a bare lambda over a non-`List` subject with the fast
  path present.
- **A generated support module beats a clever inline lowering.** Rust's
  `set` wants the string read and written; every inline form either
  splices the receiver twice (evaluating a call argument twice) or fails
  for one place shape — `&mut place` is E0596 when the place is a `&mut
  String` parameter, since that binding is not `mut`. A trait method in a
  generated file (`strings.rs`, gated like `iter.rs`) auto-refs every
  shape and mentions the receiver once. The precedent is worth reusing:
  when a lowering needs a *statement*, generate a helper instead of
  building an expression that pretends otherwise.
- **A new tie-break needs a leniency audit before it needs tests.**
  Overload specificity [fn-overload-rank] was correct on the case it
  was written for and immediately wrong on `size(xs)` where `xs` had *no
  inferred type*: an un-inferred argument fits every candidate, so a
  perfectly ordinary fate error grew a second, bogus "ambiguous call"
  beside it. Any rule that turns "several candidates match" into an error
  has to ask first whether they match *because the checker knows nothing*
  [type-unknown-lenient]. The suite caught it — the two CLI `analyze`
  tests that assert whole stderr text — which is the argument for keeping
  full-output assertions somewhere.
- **Rank the declared patterns, not the substituted ones.** The reason
  `describe<T>(T)` beat `describe(Int)` for `describe(3)` is that by the
  time the candidates are scored, `T` has been substituted to `Int` and
  the two candidates look *identical* (both "exact match", both zero
  qualifiers). Specificity is a property of the signature as written, so
  it must be read from `patterns`, before `substitute_vars`.

- **A remedy the user names has to be *checked* against the mechanism.**
  Refinements were designed with "reconcile it in a top-level `refn`" as
  the escape from a conflict, and the first implementation *merged* every
  refinement in scope — under which the reconciliation joins the
  disagreement it was supposed to settle, and changes nothing. The remedy
  only works because a top-level `refn` **replaces** the qualifiers'
  refinements [qual-refn-reconcile]. When a design records an escape
  hatch, write the program that uses it before believing the mechanism
  provides it.
- **A fact-*adding* rule cannot be dropped into a meet-shaped analysis.**
  `deduce.rs` is order-insensitive and terminating *because* removals only
  accumulate. A refinement's `+Q` is the opposite direction, so
  `if c { add(list, x) }` would have let a signature promise a fact that
  holds on one path — sound-looking, because the *call site* really does
  narrow there, and the call site is flow-sensitive while the walk is not.
  Two restrictions fixed it [qual-refn-infer], and the second (additions
  only for qualifiers the parameter declares) also keeps the lattice
  bounded, so the fixpoint still terminates. Before adding a monotone
  fact, ask which direction the existing analysis is monotone *in*.
- **A restriction can be what makes a feature backend-free.** "State
  qualifiers only" reads like a scoping decision about which claims a
  refinement may make. It is the reason the whole feature needed *no*
  emitter work: `Mut` is the one qualifier that is not erased, so `+Mut`
  would have turned a compile-time statement into an emission feature. The
  test that pins it asserts the emitted `main` contains no `qualifies`
  call — a refinement is trusted, so nothing may be emitted for it.
- **"None of them apply" needs a granularity, and per-call is the wrong
  one.** Suppressing every refinement of a *call* because two qualifiers
  disagree about one parameter costs refinements that never disagreed. Per
  (callee, parameter) is the unit, which is also the unit the warning
  deduplicates on.

- **A silent skip is a bug waiting for a deletion.** `SourceSet::classify`
  returned `None` for a file belonging to another backend, and `add_dir`
  dropped it. That was right while define files existed. The moment they
  were deleted, the same code silently ignored any `x.y.sv` — so a leftover
  `main.kotlin.sv` in a source tree would have contributed nothing, with no
  message. Deleting a feature means auditing the *tolerances* it justified,
  not just its code: every "skip this, it isn't for us" branch is a claim
  that something else handles the file, and the deletion may have removed
  the something else.
- **Delete the enum when it loses its second variant.** `BackingMod` was
  down to `Intrinsic` and `SourceKind` to `Language`. Keeping either would
  have left an invariant as a runtime check — `if file.kind != Language`
  in a dozen places, all of them dead — so both became a flag or nothing at
  all (`intrinsic: bool`, no `kind` field). This is the same lesson the
  `platform: bool` decision recorded, arriving from the opposite direction:
  there, an enum was refused before it existed; here, one was removed after
  it emptied out.
- **Keep the signature, add the body.** The 108 `external fn` declarations
  in the tests were not obstacles to route around: each became an ordinary
  fn with the *same* signature and a minimal body. That preserved what each
  test tested, because a written deduction list stays authoritative over
  anything inferred from a body — so the contract under test was unchanged
  and calls stayed on the named-call path. Rewriting them as `platform
  effect` members would have compiled just as well and quietly moved the
  tests onto `check_effect_call`, testing something else.
- **A partition test earns its keep during a deletion.** The TextMate
  grammar's keyword list is asserted to partition the lexer's keyword table
  *exactly*, and that assertion is what caught `external` and `define` still
  being highlighted after they stopped being keywords. Nothing else in the
  suite would have noticed. When a generated artifact mirrors a table, assert
  the mirroring rather than the artifact's contents.

- **Reuse the mechanism, and check what the mechanism keys on.** Mounting
  the `platform/` tree looked like "companions already do this", and it
  was — but companions are gated on their *module* being reachable, and
  `platform/main.kt` classifies as module `platform.main`, which no Salvo
  program declares. Discovery would have found the file and then dropped it
  silently. Stripping the leading segment is one line and the whole reason
  the reuse works; when adopting an existing mechanism, find the key it
  filters on before assuming the fit.
- **A generated file's name can collide in the target's namespace, not
  just the filesystem's.** `platform/main.kt` and the generated `main.kt`
  are different paths, so nothing looked wrong — but Kotlin names a facade
  class after the *file*, so both would have produced `salvo.main.MainKt`
  and the classpath would have carried two. The fix (host files live in
  `salvo.platform.<module>`) then propagated: the launch class differs when
  the host owns `main`, which is why `Backend::entry_hint` had to learn
  what was emitted. Check the target language's naming rules, not just the
  output paths.
- **Two backends' checks should fire on the same condition.** The
  missing-host error was first written per module for Kotlin (no crate
  root, so any module's `main` can be launched) and only for the crate root
  on Rust (only its `main` is reachable) — each locally correct, and
  together a program that compiles on one backend and fails on the other.
  Made uniform: any reachable module whose `main` needs a platform effect
  must have a host. When a rule's natural scope differs per backend, pick
  the stricter one rather than shipping the divergence.
- **Render generated glue with the emitter that generates what it glues
  to.** The host skeleton must match the interface member for member. It
  does, because `host_impl` calls the same `emit_param_list` /
  `emit_member_param_list` and `emit_return_type` that `emit_effect` does,
  on the same checked program, and takes the entry's arguments from the
  same `checked.fn_effects` table its parameters come from. Likewise
  `module_mod_names` is shared, so a skeleton cannot name a Rust module
  something the crate root calls otherwise. None of that needed a test to
  hold — which is the point, since drift here would emit code that does not
  compile, and a test only tells you afterwards.

- **An unchanged golden snapshot is the best evidence a refactor is
  faithful.** Moving std's interop out of `define` templates and into
  backend code could have changed emitted output in a hundred small ways;
  the check that settled it was that *no* backend snapshot moved at all,
  with both toolchain suites passing unmodified. When replacing a
  mechanism rather than a behaviour, make "the output is byte-identical"
  the acceptance criterion and you get a regression test for free.
- **Let the type system refuse a bad shape.** Adding
  `BackingMod::Platform` compiled the parser fine and immediately produced
  three `non-exhaustive patterns` errors at sites that classify *type* and
  *fn* backings — where `Platform` can never occur. That was the signal the
  shape was wrong: `platform` applies only to effects and
  `intrinsic`/`external` never do, so the flag belongs on `EffectDecl`
  (`platform: bool`), which makes the invariant structural instead of three
  unreachable arms someone would later have to reason about. A compiler
  error asking you to handle an impossible case is usually asking you to
  restructure, not to write the arm.
- **A feature framing can delete a subsystem.** The template design needed
  marker regions, canonical keys and signature-change detection so
  regeneration would not clobber hand-written code. Choosing an interface
  the host implements made all of it unnecessary — generate once, and let
  the target compiler catch every kind of drift. Before building
  machinery to protect user edits, check whether a different boundary makes
  the edits unreachable.
- **Reuse the existing injection mechanism instead of adding a second
  one.** A platform effect needed no new emission path *because* effects
  already lower to interfaces/traits threaded as parameters — the only
  change was which effects `main` receives. The alternative (a
  registration/wiring mechanism for host objects) would have duplicated
  what `use` and handler dependencies already do.
- **A program-wide uniqueness check must not read the `Symbols` maps.**
  They are hash-ordered and last-wins, which is how the
  "two effects cannot share a member name" bug hid for so long: the
  collision *was* the map overwriting an entry. The check walks files and
  items in source order and reports at the second declaration, so it is
  deterministic and fires exactly once — the same lesson as the
  `core_modules` sorting fix below.
- **When a test fails, read the test source before the compiler.** The
  first platform-effect positive test failed with "deduction promises `n`
  back to the caller, but the body moves it" — and the compiler was right:
  `return n` moves `n`, so `-> Int => n` was a contradiction I had written.
  The checker's diagnostics have been load-bearing enough for long enough
  that a fresh test source is the more likely culprit.

- **`try` was invisible to four traversals, and one of them emitted wrong
  code.** `Expr::Try`'s body is ordinary code, but the wildcard arms of the
  scanning traversals never looked into it. The mutability census was the
  fatal one: a variable assigned *only* inside a `try` body was declared
  `val`, and kotlinc rejected the output — a [backend-never-wrong] miss that
  nothing but the toolchain would have caught. (Proven by reverting the arm:
  `'val' cannot be reassigned`.) Also missing: `collect_declared` (generated
  locals could collide with a name declared in a `try` body),
  `expr_mentions` (a use-after-move mentioned only inside a `try` would go
  unreported), and the deduction walk (a consuming call inside a `try` did
  not reach the inferred contract). Rust's mutability census was missing
  `Expr::Widen` too.
- **New AST variants are only half-caught by the compiler.** Adding
  `Expr::WhenCond` produced five `non-exhaustive patterns` errors (the two
  `check_expr` dispatches, `expr_defer_escape`, and each emitter's
  expression dispatch); the *dozen* other traversals that needed an arm
  ended in `_ => {}` or `_ => false` and compiled silently. All of them are
  now exhaustive, so the next variant is a compile error at every site —
  which is how the `try` gaps above were found, since making the match
  exhaustive forces every variant to be *classified* rather than defaulted.
  The ones that matter fail quietly: `reach.rs`'s `expr_names` (a module
  used only inside the new construct has its import pruned), `block_exits` /
  `block_returns` / `expr_terminates` (a total chain not counting as
  returning), `collect_assigned_expr` / `expr_mentions`, `deduce.rs`'s walk,
  `collect_mutated` / `collect_declared` in both emitters, and each
  emitter's `emit_operand` parenthesization list.
- **A rule can sit in the spec for eight milestones without being
  enforced.** `[if-bool]` (now `[cond-bool]`, renamed when the rule grew to
  cover `while` and subject-less `when` heads) said "`if`/`elif` conditions
  must be boolean expressions; there is no truthiness" from M0 and was never
  checked: `analyze_cond`'s fallback arm typed the condition and threw the
  type away. Nothing caught it because nobody *wrote* a truthy condition —
  the spec was describing a convention the authors were already following.
  When adding enforcement to a stated-but-unchecked rule, expect the sweep
  to come back empty and do not read that as evidence the rule was
  redundant; the next contributor is who it is for. Worth auditing the
  other "must"s in LANGUAGE_SPEC.md the same way.
- **The Kotlin entry class is named after the file, not the function.** The
  `entry point:` hint `salvo compile` prints read `salvo.<module>.MainKt`
  unconditionally — right only because every entry file so far was
  `main.sv`. Kotlin puts a file's top-level declarations in a facade class
  named for the *file*, so `other.sv` gives `salvo.other.OtherKt`. Latent
  as long as the hint was only ever read by a human who then typed the
  right thing; `salvo run` executes it, so it had to be correct
  [kt-run].


- **"Erased at runtime" does not mean "no lowering".** `^` removes a
  qualifier, and qualifiers are erased, so it looked like a typing-only
  feature — but where the qualifier sat on a *union arm*, the value's
  physical view moved inside a wrapper, and reads had to peel it. The
  emitted code compiled and ran with plausible output while taking the wrong
  branch. Two lessons: a wrapper-arm change is a representation change even
  when the *type* change is pure, and a generated `Display` that prints
  "whichever arm I hold" can hide exactly this class of bug — so test a
  branch that distinguishes the arms (`err("zero")`, not just the happy
  path).
- **An expected type that only flows to *some* argument shapes will bite.**
  `resolve_named_call` passed the parameter type down for lambda and call
  arguments but not for a bare identifier — a deliberate old choice ("other
  expressions keep the historical untyped probe"). With [fn-effects] that
  silently broke a *named fn passed by value*: it never saw the fn type it
  had to adapt to, so the Kotlin emitter kept `::plain` where a
  `(Console, String) -> String` was wanted. When a feature depends on the
  expectation reaching an argument, check *which* argument shapes get it.
- **Innermost-first is the right search order for a scoped environment.**
  Both emitters looked up effect values from the front of `effect_env`, so a
  lambda's own effect *parameter* lost to the enclosing fn's value and the
  closure captured after all. The same reversal was needed for the
  base-name fallback, where "the same effect twice" (an inner scope
  shadowing an outer) had to stop counting as ambiguity while two genuinely
  different generic instances still do.
- **Check what the *other* backend does before designing a representation.**
  A bundle of functions as a struct value read perfectly and ran on Kotlin;
  on Rust it does not compile at all (`impl Trait` is illegal in a field
  type). Writing the five-line program first turned a plausible design into
  a rejected one, and made the group declaration-side sugar — which is why
  neither backend needs to know groups exist.
- **A feature that adds no lowering is worth looking for.** Implicit
  parameters gave `sort(?cmp)`, numeric abstractions and overridable defaults
  with *no* new backend machinery, because they reduce to something both
  emitters already do: pass a function. The traits design that preceded them
  needed generated interfaces on Kotlin and generated traits plus impl blocks
  on Rust. When a feature seems to need new runtime shapes, check whether an
  existing one already carries it.
- **A cache is only safe if its key is the whole input.** The e2e stamps hash
  the generated files, the expected output and the toolchain version — so
  changing the emitter invalidates exactly the tests whose output changed,
  and nothing else. Where the thing under test is a *subprocess* (the CLI
  tests), the key has to include the binary instead, which is why those
  stamps miss on every rebuild: correct, if less rewarding. Fingerprint a
  binary by length and mtime, not by content — hashing tens of megabytes in
  a debug build cost more than the tests it saved (+12s, measured).
- **The availability *probe* was one of the most expensive things in the
  suite.** Each of 39 Kotlin tests ran `kotlinc -version` as its guard, and
  that starts a JVM: 1.4s a time, against 2.4s for the compile it was
  guarding. Probing once per test binary (`OnceLock`) took the Kotlin crate
  from 44s to 32s without touching a single test. When a test suite is slow,
  measure the scaffolding before the work.
- **A gate belongs at the point of use, not in every caller.** Three tests
  called the Kotlin runner with no toolchain guard at all, so they would
  have *failed* rather than skipped on a machine without `kotlinc` — found
  only by putting the check inside `run_kotlin_files`/`run_kotlin_entry`/
  `run_rust_files`, where it cannot be forgotten.
- **An `async` block is a state machine you are allowed to borrow.** Rust
  has no stable generators, which is why `Iter<T>` was eager for eight
  milestones — but rustc *will* build a resumable state machine for an
  `async` block, and driving one by hand needs nothing but `Box::pin` and
  `Waker::noop()`. No `unsafe`, no crates, no CPS transformation of the
  body. The lesson generalises: when the target lacks a feature, check
  whether it lacks a *mechanism* or only the syntax.
- **Restricting the feature was cheaper than plumbing lifetimes, and it
  came from the same place the divergence did.** A lazy iterator has to
  hold whatever its body needs across the suspension; effects are the only
  thing emitted Rust holds as a borrow. Forbidding them
  ([iter-effect-free]) bought `'static` captures — which is why the whole
  change needed no lifetimes anywhere in the Rust output. Both parity
  strategies were available; the restriction was an order of magnitude
  less work than faithful emission would have been.
- **Write the feature in the language before designing around it.** The
  traits discussion assumed `map`/`filter`/`reduce` needed an `Iterable`
  bound. Actually writing them found the opposite: monomorphic combinators
  over `Iter<T>` already worked on both backends, and the three things in
  the way were a checker inference gap and two emitter bugs
  ([call-generic-progressive], [rs-fn-param-convention],
  [kt-fn-mangling]) — none of which a trait would have fixed, and two of
  which emitted silently wrong code. The trait question survived the
  exercise, but its *justification* changed from "needed for combinators"
  to "needed for `for` and for one body over many collections".
- **An emitted-name collision is only safe if the target agrees with the
  checker about which overload it is.** Kotlin's overload resolution
  follows Kotlin's type lattice, and Salvo types that are unrelated can map
  onto Kotlin types that are not (`Iter`/`List` both reach
  `Iterable`). Mangling every overload [kt-fn-mangling] is cheaper than
  reasoning about the target's subtyping — and the Rust backend had it
  right for a different reason all along.
- **Two renderers of the same contract must share one function.** The
  fn-type declaration and the lambda that fills it decided `Copy`-ness
  independently, on different types, and agreed everywhere except generics
  — where the failure was an `E0631` in *generated* code and only for
  *annotated* lambdas. The fix that sticks is one function computing the
  convention, called by both sides [rs-fn-param-convention].
- **A new file under `std/` needs a touch to be seen.** `std/` is embedded
  into the CLI with `include_dir`, which has no rerun-if-changed trigger for
  *added* files: `cargo run -- analyze` kept reporting "7 std files" after
  `std/core/throw.sv` appeared. `touch crates/salvo-cli/src/main.rs` (or any
  edit to the crate) picks it up.
- **A std module may be named after a target-language keyword, but only
  because the emitters already escape identifiers.** `core.throw` emits
  ``package salvo.core.`throw` `` on Kotlin — backticks are legal in a
  package declaration *and* in the matching import, verified with `kotlinc`
  before committing to the name — and a plain `mod`/`#[path]` pair on Rust,
  where `throw` is not a keyword at all. The escape comes free from
  `kt_ident`/the Rust equivalent; a module name that needed escaping in a
  position those functions do not cover would not have worked.
- **A hand-maintained copy of a keyword list will drift, and this one
  panicked the compiler.** `TokenKind::symbol()` matched every keyword
  explicitly and `unreachable!()`d otherwise, so *any diagnostic* mentioning
  the new `defer`/`try` tokens crashed instead of reporting — and the crash
  looked like a parser bug, not a diagnostic bug. It now resolves through
  `KEYWORDS`. When adding a token, grep for the enum name: a second match
  arm somewhere is a liability.
- **Write the target-language shape by hand before choosing a lowering.**
  The roadmap's `try` sketch used a closure returning `ControlFlow`; hand-
  writing it showed the closure has to capture the fn's effect parameters
  (the same exclusivity trap the fusion hits), while a *labelled block*
  captures nothing. Ten minutes of `rustc` saved a rewrite — and the same
  probe confirmed `?` on `ControlFlow` is stable and that a may-throw call
  in a loop stays a loop.
- **`?` is not usable where deferred code must run**: it returns without
  running the splice. That is why a may-throw call with pending `defer`s
  becomes an inline `match` — and it is the concrete reason `defer` had to
  be built before `throw` rather than after.
- **Where a union arm gets wrapped is a backend-shaped decision.** Rust
  wraps at the propagation site (it has one); the JVM does not have one, so
  Kotlin must choose the arm at the `catch` — which the throwing frame
  cannot know. The fix is a type-*name* tag on the signal, not an `is` test
  on the payload: erasure makes `List<Int>` and `List<Str>` the same class,
  and the wrapper encoding exists precisely to avoid such tests.
- **A lowering sketched in a roadmap can be unbuildable — check the
  motivating program against it.** E3's `defer` sketch called for a Rust
  `Drop` guard ("reverse declaration order gives LIFO for free"). It cannot
  work: the deferred call *consumes* the handle, so the guard has to own it
  from the `defer` statement onward, which makes the handle unusable for
  the rest of the block — the exact code `defer` exists to enable. Writing
  the three-line target program by hand before implementing would have
  shown it immediately (and did, once asked).
- **Splicing at exits means the block-end splice can be dead code — and
  rustc borrow-checks dead code.** A fn whose last statement is `return`
  got the deferred body twice: once before the `return`, once after it. The
  second copy used a value the first had moved. `unreachable_code` is only
  a lint, but a use-after-move there is an error, so blocks whose own
  statements always exit skip the trailing splice (`block_terminates`).
- **Kotlin's `try` is an *expression*, which is what makes the `finally`
  lowering fit everywhere.** Wrapping "the rest of the block" in
  `try { … } finally { … }` keeps working in value position — the block's
  value is the `try` block's tail — so `defer` needed no result-local
  machinery on that side, unlike the Rust splice, which has to hoist the
  tail into a temporary.
- **Check a deferred body once, replay its effect at each exit.** The
  tempting alternative (re-check the body at every exit) writes the
  checker's type/coercion side tables repeatedly, and if two exits disagree
  on a narrowing the emitters get whichever recording came last — a
  silently wrong `!!` or unwrap. Checking once (in the flow state at the
  `defer`, snapshot/restored) and replaying a *summary* — what it consumes,
  what it weakens, and the facts it relied on — keeps one recording and
  makes the disagreement a diagnostic instead of a miscompile.
- **When replaying flow effects, only ever lose facts.** The summary is
  computed in the state at the `defer`; at an exit the state may be
  *narrower* (a later `if x is None { return }`), so re-imposing the
  recorded narrowing would resurrect a fact that path does not have. The
  replay widens only when the body genuinely invalidated the narrowing, and
  never re-adds place facts.

- **A codegen symptom can be a checker hole.** "Kotlin emits
  `mutableListOf()`" looked like a one-line template fix
  (`mutableListOf<${T}>()`). It was not: the *checker* had never determined
  the element type either, so there was nothing to interpolate — and the
  same hole was silently accepting `add(xs, 1); add(xs, "two")` on one list.
  The template change only became possible *after* the checker learned to
  demand the type. When a backend cannot render something, ask what the
  checker knows about it before reaching for the emitter.
- **The absence of a diagnostic is evidence.** Both holes found in that
  session were found the same way: writing a program that *should* be an
  error and watching it pass. `let xs = mutable_list(); add(xs, "two")` and
  `held: Mut List<Int> = "definitely not a list"` were each accepted with no
  errors at all — the second because handler state initializers were never
  checked, only their declared types validated. A cheap habit with real
  yield: for any construct, write the obviously-wrong version and confirm it
  is rejected.

- **A design recorded before implementation is a hypothesis.** The fusion
  strategy was written up in detail, with rustc-verified snippets, *before*
  being built — and two of its load-bearing choices turned out to be wrong
  in ways the snippets could not show, because a snippet exercises one
  shape and a program exercises their composition. `dyn` fused parameters
  cannot forward to a callee needing a *subset* of the effects (trait
  upcasting reaches supertraits only), and flat rebuilding over the outer
  scope's handler locals makes the *outer* fusion unusable after an inner
  block. Both were found within an hour of writing the whole shape out as
  one compiling program. The write-up still paid for itself many times
  over: the reasoning it recorded is what made the fixes obvious. Record
  the design *and* expect to revise it — and prototype the composition,
  not the pieces.
- **Under a fusion, "one value with every capability" creates aliasing
  where there was none.** Two habits that were safe with one parameter per
  effect become `E0499`: an effect call inside another call's arguments
  (`fx.a(&fx.b())`), and a method call that is ambiguous because two
  effects in scope share a member name. The fixes are mechanical (hoist
  arguments into a temporary; dispatch by UFCS), but they are only
  *discoverable* by compiling a program that does both — a single-effect
  test program never trips either.
- **Environment entries that get rewritten cannot be restored by
  truncation.** The emitter's effect environment was scoped by
  `truncate(depth)`, which is correct only while entries are immutable. The
  fusion *rewrites* outer entries (every effect now threads through the
  inner fusion), so leaving a block had to restore a saved clone —
  otherwise the outer scope kept pointing at a fusion local that had gone
  out of scope. Whenever a scope-restore mechanism is a depth counter, ask
  whether anything mutates the entries below the mark.

- **"Loud" is not the same as "an error".** The interop leniency did not
  emit *wrong* code — kotlinc and rustc both rejected what it produced —
  which is why it survived so long. But the diagnostic pointed at
  generated code the author never wrote, and Kotlin's version
  (`unresolved reference 'n'`) named a symbol that plainly exists in the
  Salvo source. When judging a leniency against
  [backend-never-wrong], ask *where the error surfaces*, not just whether
  one does.
- **Leniency with no customer is pure risk.** Removing the interop
  pass-through that dated from M3 (unresolved calls, dot-calls, non-fn
  callees, fields on non-structs, non-array subscripts, non-iterable
  `for`) broke exactly **one** test — and that test's premise *was* the
  leniency. If a permissive path has no test that needs it, it is not a
  feature.
- **Lexer rules can block a syntax before the parser sees it.** `t.0.1`
  cannot be parsed as two tuple indices while the lexer still owns the
  decimal point: it hands over one `Float` token, and no parser trick
  recovers the digits (`.0.10` and `.0.1` are the same `f64`). Fixing it
  in the lexer — no fraction directly after `.`, the one position where
  the grammar cannot hold a numeric literal — made the parser side
  trivial. Where two layers disagree about who owns a character, the
  earlier layer is usually the cheaper place to fix it.
- **Kotlin does not smart-cast properties.** A narrowed nullable *field*
  read cannot rely on the smart cast a narrowed local gets: kotlinc
  rejects it ("smart cast to 'String' is impossible, because 'surname' is
  a mutable property that could be mutated concurrently"), and every
  `canbe Mut` struct field emits as `var`, so the hazard is the common
  case, not a corner. Narrowed nullable field reads emit `!!`
  ([kt-narrow-field-assert]). The lesson generalizes: when a checker fact
  is discharged by the *target language's* own analysis, check that the
  target's rules are at least as permissive — here Rust (explicit
  `unwrap`) was fine and Kotlin was not, which is exactly the asymmetry
  the backend-parity principle exists to catch. It only surfaced because
  the e2e test compiles the output; a golden-string test would have
  passed.
- **Place facts belong on the root, not in a new map.** Keying narrowing
  by `Place` looked like it needed a place-keyed frame map — a second
  structure for `snapshot_narrows`/`restore_narrows`/`merge_fallthrough`
  to carry, against the S1 gotcha below. Storing the projection facts
  *inside* the root's `LocalVar` instead kept those three functions the
  single source of truth, and made "an event on the root invalidates
  everything below it" fall out of clearing one list.
- **Restores must not resurrect invalidated facts.** `with_narrows`
  reinstates the pre-branch narrowing on exit, and the variable version
  already had to skip that when the branch *consumed* the value. Place
  facts need the same exception for invalidation (an assignment or a
  mutating call inside the branch): the dual rule is "only put the old
  fact back if the fact you applied is still standing".
- **A feature can be blocked by syntax that was never written.** P1a's
  decision to narrow constant tuple indices could not be implemented at
  first: Salvo had no tuple element access at all (`.` required an
  identifier, and `t[0]` on a tuple typed as `Unknown`). Worth checking
  that the construct a rule talks about is actually *expressible* before
  designing the rule around it — the gap here was one session wide, but
  it was invisible from the rule's wording.

- Trivia the lexer discards is expensive to get back. Comments were
  dropped outright, and the cheap-looking recovery — re-scan the text
  above a declaration in the LSP — would misread a `//` inside a string
  literal. Collecting them at the one place that already knows what a
  comment is (`LexResult::comments`) cost less than the workaround and is
  correct by construction. The trick that kept it small: comments stay
  *out* of the token stream, so no parse function had to learn to skip
  them; the parser matches them to declarations by line number.
- Look for the fact you need before adding a table. The
  narrowed-vs-declared hover line wanted "the declared type of a narrowed
  identifier use" — which `Checked::repr_ty` had been recording all along
  for the emitters' re-wrapping. A second table would have been a second
  thing to keep in sync.
- Declaration *names* are not expressions, and tooling does not care.
  Hover, which reads `expr_ty`, said nothing on the `items` in
  `fn f(items: List<Int>)` or on the `x` in `let x = …`. Recording a type
  at binding-name spans (in `declare_var`, the single funnel) fixed
  parameters, `let`, `is` and `for` bindings at once — with
  `entry().or_insert` so a more specific entry from an earlier pass wins.

- A misleading diagnostic is usually a *missing* one further up. `if n is
  Ok` reporting "this check can never succeed" was correct on its own
  terms — no arm matched — but the arm never matched because `Ok` was
  undeclared and nothing said so. When a diagnostic is technically true
  and practically useless, look for the check that should have fired
  first, rather than softening the message.
- Two namespaces sharing one syntax need explicit resolution rules.
  `Ok Int` is a qualifier and a base type side by side, and the checker
  had two independent guesses about which was which: `lower_quals` took
  any name as a qualifier, `parse_check` asked `is_qualifier` and fell
  back to *base type*. Same source token, opposite classification, no
  error either way. Any position where a name's meaning depends on which
  table it is in should reject "in neither".
- Cascade suppression needs a flag, not a heuristic. Once an `is` check
  names something unresolved, four downstream verdicts become garbage
  (can-never-succeed, no-remaining-arm, non-exhaustive, and the
  `Nothing`-narrowing that poisons the subject). Carrying an `unresolved`
  bit on the parsed pattern kills all four at once; trying to recognize
  the situation at each site would have missed some.
- Where a *lenient* checker draws the line matters more than how lenient
  it is. [type-unknown-lenient] read as "unknown names pass through",
  which is what let undeclared qualifiers survive; the useful reading is
  "types I cannot *infer* pass through". Interop needs the second, not
  the first — you reach a foreign type by declaring it.
- Validation that runs only at "declaration sites" needs its site list
  audited when it grows. `[qual-of]` documented struct fields as a
  declaration site; `validate_type` was never called on them, and the
  same held for type-alias targets, effect-member signatures, and handler
  state. Nothing failed, because the only check there was
  applicability — which silently skips undeclared qualifiers.
- std is checked by the same rules as user code, so an aspirational
  signature there rots quietly: `char_at(index: Positive Int)` had been
  copied out of a LANGUAGE.md example with no `Positive` qualifier
  anywhere, and no call could have satisfied it if there had been one.
- Where a std declaration *lives* is a code-size decision, not just
  taste. `ok`/`err` in `core.basic` would have emitted a dead
  `core/basic.{kt,rs}` into every program, because `core.basic` declares
  `Int` and [mod-used-only] reachability is name-based: every file
  reaches it. Put anything that *emits code* in a module whose names only
  its users mention.

- (sweep) Four "known leftovers" were already fixed by earlier work and
  only *looked* open because nothing tested them (field-subject `is`
  lowering, union coercion inside arrays/tuples/lambda returns, and both
  halves of the unchecked-mangling item). Probe before implementing: the
  cheapest step in closing a leftover is a program that demonstrates it.
  Two of those probes instead found *new* bugs (a stray `eprintln!`
  debug print left in `check.rs`, and Rust rendering fn-type `let`
  annotations as `impl FnMut(...)` — invalid Rust).
- (sweep) Emitter *state* beats threaded parameters for context that must
  survive nested lowerings: `StmtCtx` was passed down through
  `emit_stmt`/`emit_block_stmts`, so every value-position path
  (`emit_value_block`, loop lowering, `when` expressions) silently reset
  it to `Normal` and iterator-body `return`s stopped retargeting. As a
  field with explicit save/restore at the two real boundaries (fn bodies,
  lambda bodies) the default becomes "inherit", which is what the
  language means.
- (sweep) Kotlin and Rust do *not* share an operator precedence table:
  Kotlin gives comparison a tighter level than equality, Rust puts them
  on one non-associative level. A ported precedence table must be
  re-derived per backend, or `(a == b) < c` re-renders flat and
  re-associates.
- (sweep) Emitter dispatch in unchecked contexts must be *arity plus
  types*: `resolve_define_fn`'s arity-only fallback silently picked the
  first same-arity template. The fix pattern is
  `<candidates> → filter by checked argument base names → exactly one or
  error` ([backend-never-wrong]); `ty_base_name` (checker types) and
  `type_base_name` (AST types) must agree on conventions (`T?` compares
  as its value arm, arrays as `[]`).
- (sweep) `HashMap` iteration order leaked into user-visible behavior:
  implicit `core.*` modules were added to each scope in hash order, which
  ordered overload candidates. Anything that feeds resolution or
  diagnostics must iterate deterministically (`core_modules` is sorted
  now).
- (sweep) Generic bindings in `unify` must *widen*: first-binding-wins
  made `pick(1, maybe_int)` bind `T = Int` and hide the optional. There
  is deliberately no occurs check — `Ty::Var` identity is name-scoped per
  side, so a callee's `T` legitimately binds to `List<caller-T>`; a
  name-based occurs check would reject generic forwarding.
- (sweep) Handler state fields are declared bare (`i: Int = 0`) — there
  is no `state` keyword, despite the prose in some notes.
- (D1) A test can *encode* the bug. `undeclared_qualifiers_pass_through_
  calls` asserted the exact behavior that made the checker unsound, with a
  comment explaining why it was right. When a soundness fix makes a test
  fail, read the test as evidence about the old model before assuming the
  new code is wrong — and rewrite it to state the new rule rather than
  deleting it (the delta form now carries the old behavior as an opt-in).
- (D1) The unsoundness survived because the *inference* direction hid it:
  facts only shrink, so "preserve everything not mentioned" looked like a
  safe optimistic start. Optimism is only safe if the pessimistic end of
  the lattice is reachable for every fn that needs it — here mutating fns
  could never reach it, because no constraint knew about qualifiers the
  signature never named.
- (decl-explicit) The `add` bug is the cautionary tale for
  inference-from-nothing: a bodyless fn has no body to constrain
  inference, so the "optimistic start" of the deduction fixpoint *was* the
  answer — keep everything, including an element the list had taken
  ownership of. It survived because the two backends disagreed quietly:
  Kotlin aliased and printed, rustc rejected with E0382. When a rule's
  inputs are absent, ask what the optimistic default *claims*, not whether
  the algorithm terminates.
- (decl-explicit) Trust boundaries should be one-directional. Once
  externals must declare their contracts, the checker must *stop*
  second-guessing them — the `Mut`-parameter proxy added hours earlier had
  to come back out, because inferring mutation from a declaration and
  overriding the author is the opposite of "the declaration is the
  contract". Catching wrong declarations belongs in a separate, opt-out
  validation pass, not in the semantics — which was roadmap E2, deleted as
  obsolete in 2026-09-09 once `define` templates no longer existed to inspect.
- (decl-explicit) Requiring declarations exposed a second silent hole for
  free: effect-member deductions were *parsed* and ignored, so a member
  taking ownership never consumed its argument. The fix was to extract the
  flow half of the fn-value contract loop (`apply_call_contract`) and
  reuse it — when two call paths are documented as "keep the two in
  sync", that is a sign they should share code instead.
- (D1) Deriving a capability from a *declaration* rather than a body is
  the only option for `external`s, and it is often the right call anyway:
  `Mut` on a bodyless parameter means "I may mutate this", which is
  exactly the fact the soundness rule needs. Applying the same proxy to
  bodied fns would have been strictly worse (a non-`Mut` parameter can
  still have `Mut` *contents* mutated through it — the `h.tags` case the
  fate analysis already tracks).
- (std) Analyzing `--src std` double-loads the standard library (the CLI
  always loads its embedded copy) and trips every [mod-collision] check.
  Expected artifact of pointing the tool at its own std — use an ordinary
  project directory to see std diagnostics.
- (arrays) The CLI embeds std with `include_dir` at *build* time: adding a
  file under `std/` does not invalidate the crate, so the new module is
  silently invisible to `cargo run -- compile` (the tell is the file count
  in "parsed N file(s)"). Touch a `salvo-cli` source file to force the
  rebuild. The backend test crates read `std/` from disk and see new
  files immediately, which makes the discrepancy easy to misread.
- (optionals) A new strictness rule is also a *documentation* audit: the
  interpolation ban immediately flagged three LANGUAGE.md examples that
  read `${person.surname}` after `person.surname is Str` — the spec was
  quietly assuming Kotlin-style field smart-casts. Fix the examples to
  the binding form (`is Str surname`), which the spec already used
  elsewhere. Also worth reading twice: when two backend demo *sources*
  differ (here `${capped}` vs `${capped!}`), that divergence is usually
  evidence of a missing checker rule, not a backend quirk.

- (L7d) Expected types only reach lambda arguments when the arg-typing
  pass supplies them: named calls typed all arguments with `None`
  before overload matching, so fn-type contracts silently never
  arrived at `check_lambda`. Single-candidate callees now pre-lower
  their parameter types as expecteds for lambda literals; overloaded
  callees still probe untyped (an expected could bias resolution) —
  contracted lambdas passed to *overloaded* fns are a known gap.
- (L7d) The named-fn-by-value pass was broken on BOTH backends in
  different ways (Rust: signature-mode mismatch; Kotlin: a bare
  identifier where `::name` is required) — when a feature is
  "loud-but-unsupported", verify each backend's failure mode
  separately; they rarely match.
- (L7c) A borrow crossing a fn boundary interacts with *every* later
  relaxation: S2's move-mode would happily have "taken ownership" of a
  physically borrowed call result (moving out of a `&` — rustc
  rejects, checker accepted). Flow facts that change physical
  representation must be carried on the links themselves
  (`FateLink.borrowed`) so every downstream rule can refuse; when
  adding a new emission regime, probe its interaction with move-mode
  before shipping.
- (L7b) The Ident-callable path in `check_call` never `check_expr`s the
  callee identifier, so consumed-value reads did not error there —
  calling an already-consumed callable was silently accepted until the
  path got an explicit `Ty::Nothing` branch. When adding a consumption
  rule, audit every place an identifier is *used* without going
  through the standard read path.
- (L7b) `once` is the first qualifier with *inverted* subtyping (a
  restriction, not a refinement): plain fn <: `once` fn and `once` may
  never drop. Both `is_subtype` and `unify` carry special cases —
  flagged for review; if a second restricting qualifier ever appears,
  generalize direction into the qualifier model instead of a third
  special case.
- (L7a) Opting a variadic constructor into linearity is a trap:
  variadic positions are untracked by the flow analysis, so a linear
  argument would be physically moved while statically still owed —
  contradictory requirements. The variadic guard refuses linear values
  there outright; collection construction with linear elements is
  empty-then-`add`. When opting in an external, audit *every* position,
  not just the contract shape.
- (L6) A Salvo-bodied consuming fn must end the obligation chain
  itself: `fn close(h: FileHandle) -> None {}` leaks `h` by its own
  rules — the body owns the moved-in value and must `discard` it (real
  release lives in external fns with no body to check). Tests and
  examples that stub consumers with empty bodies will all fail the
  frame-drop check; stub with `discard`.
- (L6) The obligation checks mark reported variables as consumed so
  each obligation errors exactly once (a `return`-site error would
  otherwise repeat at the frame pop). Any new exit-shaped check should
  follow the same report-then-consume pattern.
- (S3) Decide the borrow path *before* emitting the value: `emit_let`
  computed `value_code` eagerly, and the discarded emission would have
  recorded coercions/union sizes for code that never lands. Emission
  helpers are not side-effect free — order decisions first, render
  second.
- (S3) By-reference loops hand `&T` to every lowering that touches the
  loop variable: `matches!`/unwrap lowering for union and optional
  elements expects owned subjects and breaks on references — hence the
  concrete-non-union element guard. If later refinements widen the
  guard, extend the union-test rendering to ref patterns first.
- (S3) Borrowck alignment is a consequence of poison, not a separate
  analysis: a checker-legal program never uses a derived value after
  its root's mutation/move, so NLL sees every borrow die before the
  conflict. The one mismatch is *within a single call* (pass a
  borrowed local and move its root in one argument list — rustc E0505,
  checker-legal): loud, documented, rare.
- (L4) "Lambda bodies are a barrier" was only ever true of
  `loop_stack`: bodies are checked *inline*, so flow events (consuming
  calls, mutations) always fired against outer variables — captures
  were never fully untracked, they were tracked with the wrong
  multiplicity (once, at creation). The L4 model rides those existing
  events (a boundary stack + guards at the consuming sites) instead of
  adding a separate capture walk; when auditing "untracked" claims,
  check what the inline checking already does.
- (L3) Sibling arguments of one call are the *only* place where a read
  can see a value after its move without the standard consumed-read
  error firing: argument typing runs before contract enforcement, and
  nested calls consume during typing. Hence the dedicated
  mention-scan (`expr_mentions`) rather than a flow-state fix.
- (L3a) The fixpoint converges because move-mode candidates and claims
  only grow; inferred deductions are the one non-monotone axis
  (overload resolution can flip with narrowing), which is why the
  round cap exists. The instability error path is untested — nobody
  has constructed a genuine oscillator yet; if you find one, turn it
  into a test.
- (S2) Round one must see *every* fate event, or mode inference goes
  blind: kept-`Mut` mutation events only fired when a call contract
  existed, and round one had no inferred facts — so mutation-driven
  move-mode candidates were recorded one round too late. The fix:
  round one falls back to the *optimistic* contract
  (`deduce::optimistic`) for unwritten callees — it enforces no moves
  and strips nothing, so it is behavior-neutral except for surfacing
  the `Mut` mutation events.
- (S2) Flattened fate links now keep their *original* bind spans
  (only the direct source link carries the current event's span). This
  makes a variable's links describe its whole derivation chain, which
  move-mode candidate recording needs — intermediate variables of a
  chain (loop bindings especially) are often dead by the time the move
  is seen. If links are ever restamped again, zero-clone chains break
  silently (one clone reappears per dead intermediate).
- (S2) A `for`-loop binding must be re-declared *fresh on every
  checking pass* of the body (it binds a new element each iteration).
  Declaring it outside `check_loop_body` let its consumed state leak
  into the back-edge re-check — a live false positive
  (`for s in xs { consume(s) }` errored) that predated S2 and only
  surfaced when move-mode made consuming loop bindings routine. Loop
  bindings go through the per-pass `bindings` channel
  (`pattern_bindings`); `is`-bindings always did.
- (L2) An always-exiting branch contributes nothing to the merge after
  the construct — which is correct *inside* a loop body but silently
  drops `break`-path consumption for the code *after the loop*. The
  loop exit is a join point of its own: `LoopCtx` captures a state
  snapshot at every `break` and the loop merges them with the
  fall-through exit state. When adding a new control-flow event, ask
  *where its state lands*, not just whether the branch exits.
- (L2) When merging snapshots with different frame depths
  (`merge_fallthrough`), put the shallowest (current) snapshot first —
  it drives the key iteration, and deeper break-time frames align as a
  prefix. For `for` loops, merge only after the loop-binding frame is
  popped.
- (L2) Kotlin's `.copy()` for struct spread is *shallow* while Rust's
  `..base.clone()` is *deep* — a mutable-data parity divergence that
  had been sitting unobserved in [struct-spread] since M8. Consuming
  the spread base closed it by restriction. Same audit lens as the S1
  lesson: for every emission difference, ask "could the two backends
  disagree observably?" — the remaining known case is projection values
  in moved positions (documented in [fate-poison]).
- (S1) `[struct-mut]` ("only `Mut Name` values may have fields
  assigned") was in both specs but *unenforced* — and became
  load-bearing: Kotlin's identity lowering of `copy` is only correct if
  non-`Mut` values really are immutable. When a backend decision leans
  on a checker rule, verify the rule is actually enforced, not just
  written down. (Enforcing it also exposed a LANGUAGE.md example bug:
  the `Mut` struct example mutated `person` instead of
  `mutable_person`.)
- (S1) Poison rides the existing consumed-state machinery: a poisoned
  variable is just `narrowed = Nothing` plus a `Poison` reason on the
  `LocalVar` — branch merging, revival-by-reassignment, `is`-restore
  survival, and the loop re-check all carry it with no new lattice.
  Keep new flow facts inside `VarState` (narrowed/links/poison) so
  `snapshot_narrows`/`restore_narrows`/`merge_fallthrough` stay the
  single source of flow-state truth.
- (S1) Variable *names* are not stable identities across sibling scopes
  even with [var-no-shadow] (two sequential `for person in …` loops);
  fate links use per-binding numeric ids (`Checker.next_var_id`), and a
  link may outlive its root (the loop binding dies, the flattened link
  to the collection survives) — flatten to all transitive roots at the
  binding, not lazily.
- (S1) `is`/`when`/`for` bindings alias their subject, so they must
  inherit its fate links — the binding tuples are `(Ident, Ty,
  Vec<FateLink>)` threaded through `CondInfo`/`IsInfo`/
  `check_branch_block`. A new binding form must decide its provenance.
- (S1) The checker now keeps *both* sides of `let a = b` readable, so
  the Rust emitter had to stop moving bare-ident sources
  (`emit_linked_value`: owned non-Copy locals clone in `let`/assignment
  values and `for` iterables). Checker-permissiveness changes and
  emission must move together, or the gap surfaces as rustc errors on
  generated code.
- (S1) `i++` and whole-variable reassignment are *rebinds* (revival:
  sever the variable's own links, poison its previous derivatives), not
  mutations — only projection assignment and `Mut`-kept call arguments
  are mutation events. Getting this wrong makes ordinary loop counters
  (`let i = start; while … { i++ }`) uncompilable.
- (S1) Kotlin's `copy` immutability analysis must be *transitive* (a
  non-`Mut` struct with a `Mut List` field is not immutable — identity
  would alias the mutable part and diverge from Rust's deep clone), and
  generic struct fields must be checked under the instantiation's
  substitution (`approx_ty` bridges written field types to checker
  `Ty`s just far enough for that).
- (S1) Leniency gaps in the fate analysis are not always mere
  strictness gaps — some are *backend-parity* holes. Untracked
  mutation events let a Kotlin alias observe a change a Rust clone
  does not (`let t = h.tags; add(h.tags, 2); size(t)` prints 2 vs 1).
  When auditing an unhandled event, always ask "could the two backends
  disagree?", not just "should this be an error?" — the parity
  principle in the roadmap is the checklist.
- `unify` (overload resolution) is *order-sensitive across its match
  arms*: the argument-qualifier-stripping arm
  (`(_, Ty::Qualified { .. })`) must come *after* the union-parameter
  arm, or a qualified argument (`Ok Str`) loses its qualifier before the
  union's qualified arms are tried and `describe(ok("x"))` against
  `Ok Str | Err Str` never resolves. Found passing a union-arm value in
  argument position; `is_subtype` had the arm→union rule all along, but
  candidates were rejected by `unify` before the subtype check ran.
- `SourceSet::classify` with a backend name that matches no define suffix
  (the CLI passes `""` for backend-neutral `analyze`) loads language
  files only — every `*.<something>.sv` is treated as another backend's
  define file and skipped. Cheap way to get a language-only load; don't
  name a real backend the empty string.
- CLI integration tests use `env!("CARGO_BIN_EXE_salvo")` +
  `env!("CARGO_TARGET_TMPDIR")` (both provided by Cargo for integration
  tests of a crate with a binary) — no `assert_cmd`/`tempfile`
  dependencies needed.
- (lsp) `lsp_server::Connection` must be *dropped before*
  `io_threads.join()`: the writer thread only exits when the
  connection's channel sender is dropped — joining first deadlocks the
  server on shutdown (symptom: clean shutdown/exit exchange, then the
  process never terminates).
- (lsp) `Path::canonicalize` fails for files that don't exist on disk
  (unsaved editor buffers): canonicalize the *parent* and re-append the
  file name, or symlinked roots (macOS `/tmp` -> `/private/tmp`) make
  overlay paths miss the workspace root and the buffer silently drops
  out of the analysis.
- (lsp) Publish diagnostics against the URI the client opened the
  document under, not one rebuilt from the canonicalized path — clients
  match URIs textually, and a `/var` vs `/private/var` rewrite makes
  them ignore the publish.
- Kotlin smart casts make some emitted `as` casts redundant (kotlinc warns
  "no cast needed") — harmless. The `(x.value as T)` unwrap casts are
  *required* though: narrowing may come from `elif` exclusion where Kotlin
  has no smart cast, and `value` is typed `Any?` on the sealed interface.
- `is UN_i` checks need star projections (`is U2_1<*, *>`) — kotlinc
  rejects bare generic classes in `is`. Sealed exhaustiveness still works.
- A subject-less Kotlin `when` used as an expression demands an `else`
  branch — the emitter converts the last branch of a `T?`-subject `when`
  to `else` (sound because the checker proved exhaustiveness).
- The checker and emitter must agree on the ident-unwrap rule
  (`repr is wrapper && logical is a single non-None arm`); `maybe_coerce`
  computes the "effective repr" with the same predicate the emitter uses.
- `define` templates that call something with the same name as the effect
  member they implement must qualify it (hence `kotlin.io.print`).
- Overload resolution now happens in the checker; `size(Str)` vs
  `size(List<T>)` style collisions are resolved by argument type. The
  emitter still falls back to arity in unchecked contexts.
- insta snapshot tests fail on first run by design; accept with
  `INSTA_UPDATE=always`.
- The `Number` type alias in std is a general union — fine: aliases expand
  on use (now with generic substitution in both checker and emitter), and
  nothing uses `Number` yet.
- Subtype-rule *order* matters for qualified union groups: the
  `(_, Union)` any-arm rule would otherwise compare `Ok (A | B)` against
  single arms and always fail; the group rule (exact-arm equality, then
  drop-quals) must come before it. Symmetrically, `maybe_coerce` must try
  wrapping the group as a whole arm *before* stripping its qualifiers.
- `substitute_vars` must re-normalize `Ty::Qualified` through `qualify()`:
  a generic constructor's `Ok T` with `T = Ok Str` would otherwise nest
  `Qualified` inside `Qualified` (breaking the type invariant).
- Qualifier validation happens at *declaration sites* (`validate_type` in
  check_fn / let / struct fields), not inside `lower_type` — lowering runs
  repeatedly (e.g. per overload candidate), which would duplicate errors.
- Overload mangling compares *emitted* Kotlin parameter strings, so the
  `Mut List` → `MutableList` mapping naturally avoids false collisions.
- Predicate `is` on a subject already narrowed out of a wrapper union works
  because `emit_expr_base` (the unwrap) is used for the `qualifies` call
  argument.
- All handlers — external ones included — emit as Kotlin *classes* and are
  instantiated at their `use` site (user decision: `object` was an artifact
  of StdOutConsole being stateless). Bare `use Handler` is sugar for
  `use Handler()`; external handlers support constructor params like any
  other handler.
- `kotlin_ty(Ty)` (checker type → Kotlin) must agree with `emit_type`
  (AST type → Kotlin) on the same source type: checker-resolved effect
  types are looked up in an effect environment keyed by `emit_type`
  renderings. Both expand type aliases and map internal names through
  `emit_named_parts`, which is what keeps them aligned. If they drift, the
  lookup degrades to base-name fallback matching (or a codegen error) —
  never silently wrong dispatch, but worth knowing when adding type forms.
- The checker types effect-member call *args* only when it must
  disambiguate between multiple instances (multi-candidate path types them
  with `None` expected, then records coercions afterwards); the
  single-candidate path checks args directly against the substituted param
  types, preserving expected-type-driven inference for lambda/struct-lit
  arguments. Keep the two paths in sync when touching `check_effect_call`.
- `Stmt::Use` no longer runs `check_expr` on the whole handler expression
  (ctor args are typed individually in `check_use`), so the handler `Call`
  expr itself has no `expr_ty` entry — the emitter doesn't need one, but
  don't add a table lookup keyed on it.
- Same-name defines for overloaded externals (`size(Str)` vs
  `size(List<T>)`) are disambiguated in `define_for_decl` by comparing
  parameter *base type names* against the checker-resolved declaration —
  the arity-only fallback (`resolve_define_fn`) can still pick the wrong
  one in unchecked contexts.
- The `use` duplicate-registration check compares checker `Ty`s, so it
  catches `Random<Int>` twice while allowing `Random<Int>` +
  `Random<Str>`; the effect-list duplicate check is separate
  (`check_effect_list`) because declared effects never pass through
  `check_use`.
- Kotlin loops are *statements*: any block whose value is a trailing
  `while`/`for` (if-expr branches, `when` branches) must route the loop
  through `emit_loop_value` — a plain emission would silently value the
  block as `Unit`. `emit_value_block` special-cases the trailing loop.
- The loop result local must be a *nullable* temp (`var __loopN: T? =
  null`) even for non-optional joins: Kotlin cannot prove definite
  assignment through a loop, so the block ends `__loopN!!` instead. Only
  loops with `else` produce non-optional joins, and then every path
  assigns — except the spec-silent corner where every iteration
  `continue`s before the tail; the checker closes it by joining `None`
  into the type whenever a bare `break`/`continue` exists.
- `break`/`continue` attribution needs matching stacks on both sides
  (checker `loop_stack`, emitter `loop_results`), and both must treat
  lambda bodies as barriers and *pop before checking/emitting the `else`
  block* (a `break` in `else` belongs to the outer loop).
- Deduction inference must interpret a callee's deduction list relative
  to the callee's *declared* parameter qualifiers (removal set =
  declared − kept, subtracted from the argument), not as the absolute
  set of remaining qualifiers — otherwise qualifiers the callee never
  declared would be wrongly stripped from the caller's argument.
- Deduction fixpoint direction matters: start optimistic (all kept, all
  quals) and only remove facts — starting pessimistic would not converge
  to the least-strict sound answer and recursive fns would infer
  everything as moved.
- Kotlin wildcard imports of *nonexistent* packages are compile errors:
  generated `import salvo.<mod>.*` lines must be filtered to modules that
  are actually emitted (reachable *and* produce code) — hence
  `emitted_modules` is computed before any file is emitted.
- A companion file with the same stem as a code-producing module would
  silently overwrite the generated `.kt` at write time (both map to the
  same path); `emit_program` errors on the collision instead. The
  LANGUAGE.md pattern keeps companion modules externals-only.
- Aliased Salvo imports need Kotlin alias imports only for items that
  exist as Kotlin symbols; alias-importing an *inlined* external (define
  template) would reference a nonexistent symbol and fail kotlinc.
  `emit_fn_call` must emit the source (alias) name when it differs from
  the declaration name, or the alias import is dead and the call
  ambiguous.
- `unions.kt` is now driven by the emitters' tracked sizes only (the
  checker's `union_sizes` may include wraps in unreachable modules);
  every emission path that renders a wrapper inserts its size
  (`emit_ty`, `emit_union_type`, `apply_coercion`, union tests) — keep
  that invariant when adding forms.
- The `--emit-ast` flag takes an optional `=MODULE` value
  (`num_args 0..=1` + `require_equals` in clap); a bare flag must not
  swallow the next positional argument.
- (M8) `matches!(subj, Pat)` and `match subj` on a place do not move it
  when the patterns bind nothing - union tests and `when` lowering rely
  on this to test borrowed subjects without clones.
- (M8) Rust implicitly reborrows `&mut` *place* expressions at call
  sites, and coerces `&mut T` to `&T` - which is why parameter bindings
  thread through calls as bare names while locals need explicit
  `&`/`&mut`. Unsized coercion turns `&mut ConcreteHandler` into
  `&mut dyn Effect` at the argument position for free.
- (M8) The Rust ident-unwrap rule is a superset of Kotlin's: it also
  unwraps `T?`-repr idents narrowed to their value arm (Kotlin smart
  casts those). `maybe_coerce`'s "effective repr" was extended to match
  both - if you touch one of the three places, touch all three.
- (M8) `WrapOption` must be recorded even though Kotlin ignores it:
  the checker cannot know which backend will consume the tables.
  Backends must treat unknown-to-them coercions as no-ops only when the
  representation really is transparent (Kotlin nullability), never by
  default.
- (M8) Everything the Rust emitter renders in a *moved* position must be
  owned; the easy mistake is a field read (`person.name`) - moving out
  of a borrow is illegal, hence the clone-by-default owned rendering.
  The place/owned/raw rendering split (`emit_place`/`emit_owned`/
  `emit_raw`) exists to keep assign targets and `matches!` subjects
  clone-free.
- (M8) Define templates receive raw *places* for non-variadic args so
  method-style templates (`${list}.push(..)`) borrow natively; variadic
  parts splice owned because they land inside constructors
  (`vec![${...elems}]`). Getting this backwards either double-clones or
  moves out of borrows.
- (M8) rustc needs `match` exhaustiveness over the *representation*:
  a `when` over a narrowed subject covers only the logical arms, so the
  emitter appends `_ => unreachable!()` when repr arms are left over
  (the checker proved them impossible).
- (M8) Enum-variant wraps need turbofish (`Union2::<A, B>::U1(x)`): the
  other type parameters are not inferable from one arm's payload.
- (M8) Eager iterators changed side-effect *timing* vs Kotlin's lazy
  `Iterable`, and infinite ones hung. **Closed 2026-09-05** without waiting
  for generators to stabilize: an `async` block *is* a state machine rustc
  will build, so the lowering drives one with a no-op waker
  [rs-iter-lazy].
- (M8) The crate root must carry `#![allow(...)]` *before* any item, and
  `#[path]` mounts resolve relative to the file containing them - the
  root-file header is prepended after all files are emitted, when the
  full mount list (unions, companions) is known.
- (rename) A keyword rename touches more than the lexer: `KEYWORDS`
  feeds `salvo lang tm-grammar`, and a test compares the generated
  grammar against the checked-in `vscode/syntaxes/salvo.tmLanguage.json`
  byte for byte — add the word to a category list in `cli/src/lang.rs`
  (`keywords_are_fully_categorized` asserts the partition is exact) and
  regenerate the file in the same change. A longer keyword also shifts
  every span in the parser snapshots, so the insta diffs are large but
  should contain *only* span deltas and the renamed field.
- (rename) Sweeping a keyword with `sed` needs case sensitivity and a
  pass over the hits first: `with Linear` (syntax) and `with linear
  type` (an error message's prose) differ only in case, and one site
  that looked like the same clause — `qualifier NonEmpty<T> of List<T>
  with Mut<T>` in a prototype sketch — was the *other* meaning of `with`
  ([qual-with]) and had to stay. Grep with context, exclude the
  exceptions explicitly, then re-grep for leftovers.
- (N1) **Rust puts modules and structs in one type namespace.** The
  attractive idea of emitting a dot-name as a real nested module
  (`pub mod Environment { pub struct Id }`) dies on E0428 the moment the
  namespace struct exists — which the rule requires. Verified with rustc
  before writing any emitter code; a lowercase module (`environment::Id`)
  does compile but impersonates a Salvo module. Flattening plus a
  language-level collision ban was the answer.
- (N1) **One name representation beats two.** Carrying a dot-name as a
  single dotted string in `Ident.name` meant scope keys, checker types,
  `ty_base_name`/`type_base_name` and define-template environments needed
  *no* changes — the "touch one, touch all three" trap never fired.
  Translation happens where a Salvo name becomes target syntax: Kotlin
  renders it verbatim (nested access is spelled the same), Rust flattens
  in `rs_ident`, which is the single funnel for all 56 identifier
  renderings. The safety argument for that: a dot is invalid in a Rust
  identifier, so any dotted string reaching `rs_ident` can only be a
  dot-name.
- (N1) **A casing rule pays for itself.** Making types-uppercase /
  values-lowercase normative was needed for dotted *expressions*
  (`Environment.Id { … }` vs `person.name`), but it also turned an
  existing heuristic into a consequence: `parse_is_check` had been
  guessing that a lowercase word after `is` was a binding. Rules that
  retire guesses are cheaper than they look — and no existing source
  violated it, so the sweep cost nothing.
- (N1) Import paths split *positionally* (last segment = item), so
  two-segment item names needed the split to become casing-aware:
  trailing uppercase segments are the item, and a lowercase last segment
  stays a value (`import core.list.size`). The synthesized dotted key is
  short-lived, so `ModuleItems::name_ref` exchanges it for the
  declaration's own `&'p str` before it enters the scope maps.
