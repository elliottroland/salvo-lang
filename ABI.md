# The platform ABI — design and build sequence

Working document for replacing platform templates with a **generated ABI**
(user direction 2026-10-01). It holds the proposal, the decisions as they are
made, the questions still open, and the order of work. When a step is built,
its record moves to COMPLETED.md as usual; when the design settles, the rules
move to LANGUAGE_SPEC.md and the backend specs, and this file shrinks to what
is still open.

## Why

Platform templates (`<m>.sv.kt` / `.sv.rs`, [host-splice], built 2026-09-30)
put Salvo inside host code. Reading the aws glue written that way, the user
found two problems:

1. **Two languages at once.** Every line mixes Kotlin or Rust with Salvo in
   backticks, so the reader has to switch constantly.
2. **No host tooling.** A template is not a Kotlin or Rust file to the host's
   compiler, IDE or linter, so none of them can help.

The approach before templates, hand-written host files (`<m>.kt` / `<m>.rs`),
avoided both but had a problem of its own: the files float free. They refer to
types (`Union7<…>`, `U7_1(…)`, `salvo.core.checked.Checked`) that exist only in
the compiler's output, so the host's tooling cannot see them and a change in
emission breaks them silently until the next build. `std/platform/fs/host.kt`
shows it:

```kotlin
private typealias Kind = Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory,
    PathEscapes, IoError, Streaming>

private fun notFound(path: String): Kind = U7_1(NotFound(path))
```

Now that the manifest names a platform root per backend ([platform-root]), the
compiler knows where host code lives, so it can **generate the types host code
needs next to it**. The implementation then compiles against real files in
its own tree, with full host tooling, and refers to nothing that appears only
at build time.

## The proposal (user, 2026-10-01)

Three kinds of file per backend, all under the backend's platform root:

1. **The ABI**: the types every platform implementation may use, reachable as
   `salvo.X`, and the same types the compiled program uses at run time
   (unions, tuples, `Checked`, `Reply`, streams, `Bytes`, the collections). It
   is generated and freely regenerated, never edited, and named `*.sv.kt` /
   `*.sv.rs` to say so.
2. **The platform interface file**: one per Salvo module that declares
   `platform fn`s or `platform handler`s. It holds:
   - the interface each handler implements;
   - for each fn, a wrapper the compiled program calls, which calls the
     implementation and does any validation or translation needed, for example
     checking that a `Str` returned for `"A" | "B" | Other Str` is one of the
     literals, or wrapping it as `Other`;
   - the Salvo structs that appear in those signatures.

   It is generated and freely regenerated (`*.sv.kt` / `*.sv.rs`). Its wrappers
   are not for host code, so they carry mangled names (perhaps a `_platform`
   suffix), leaving the real names to the implementation.
3. **The platform implementation file**: mirrors the interface file, imports
   the ABI, and is the host developer's. It is written once if missing and
   never regenerated.

Also decided in the same message:

- **Platform effects go.** Only `platform fn` and `platform handler` remain
  until a need for platform effects appears. No shipped source uses one:
  `std`, the examples and `modules/aws` use platform handlers only, and the
  effects are exercised by tests alone.
- **Templates go**, with everything built for them.
- **Union naming** is to be revisited.
- **Kotlin names** should follow Kotlin's camel case rather than Salvo's snake
  case.

## Decisions and open questions

Each item says whether it is **decided** (with the date) or still a
**DECISION**, which is the user's call.

### D1. `platform fn` as syntax — decided 2026-10-01

`platform fn name(…) -> T`, a modifier like `platform handler`'s. A bodiless
`fn` without it stays an error, so every interop point can be found by
searching for one word.

### D2. A host project in each platform root — decided 2026-10-01

Each platform root holds a generated, regenerated host project, so the host
tools open it as an ordinary project: `build.gradle.kts` +
`settings.gradle.kts` for Kotlin (the ABI and interface files as sources, the
resolved `[kotlin] artifacts` as dependencies), `Cargo.toml` for Rust. The
generated files are checked in, so a checkout opens in an IDE before the first
build.

Built 2026-10-01 (steps 6–7): the runtime under `<root>/salvo/*.sv.<ext>`,
a module's declarations at `<root>/<module path>.sv.<ext>`, implementation
files at `<root>/<module path>.<ext>`, the project files at the top. The
build does not consume the project (D4). On Rust the root is a library crate
whose `lib.sv.rs` mounts every file at the path the build gives it, so
`crate::…` agrees between the two.

### D3. Every build regenerates — decided 2026-10-01

`compile`, `run` and `test` generate the ABI and interface files for their own
output anyway, so they also rewrite the copies in the platform root. An error
in the host code then appears both in the build and in the platform project.
`analyze` and the language server do not write files.

### D4. One definition of each type — decided 2026-10-01: (a)

The problem, by example. `fs.sv` declares the types and `fs/host.sv` the
platform handler that uses them:

```
// fs.sv
export struct NotFound { path: Str }
export type FsError = NotFound | PermissionDenied | …

// fs/host.sv
export platform handler HostRawFs of RawFs     // members return `… | FsError`
```

Today `NotFound` is defined once, in the emitted `fs.kt` (`package salvo.fs`,
`data class NotFound(val path: String)` plus its wire codec and comparisons),
inside the build output. The implementation file in the platform root needs
`NotFound` to compile, but the build output does not exist when the IDE opens
the root, and it is not part of the host project anyway.

So the definition has to be somewhere the host project can see, and **only
there**: if the platform root defines `salvo.fs.NotFound` and the emitted
`fs.kt` does too, Kotlin reports a redeclaration (both are `package salvo.fs`),
and Rust has two unrelated types. "Defined in the generated host tree" means:

- the platform root gets a generated file holding `data class NotFound(…)`,
  with everything the emitter attaches to it (codec, `equals`/`compareTo`,
  Rust derives and trait impls);
- the emitted `fs.kt` / `fs.rs` leaves it out, and the rest of the program
  uses the platform root's definition (Kotlin: same package, so no import
  changes; Rust: the emitted `fs` module re-exports it, `pub use
  <host crate>::fs::NotFound;`).

Rust adds a constraint: with the host project as a separate crate, the emitted
crate may not implement foreign traits (`Hash`, `Clone`) for a type defined
there (the orphan rule), so the derives and impls must be generated with the
definition, never added later by the emitted code.

The open choices:
- (a) **The closure of the platform signatures moves**: `NotFound`, the other
  arms of `FsError`, every struct their fields name, and so on. Everything
  else stays in the emitted modules.
- (b) **Every type definition moves** into the platform root's generated
  files, so the split is "types in the host tree, code in the output". It is
  simpler to state and never needs a closure computed, but every program with
  a platform root carries all its types there.

Also open: which generated file holds a moved type. The proposal puts the
structs "in the platform interface file", but `NotFound` belongs to `fs`,
which declares nothing platform. A per-declaring-module types file
(`<root>/fs.sv.kt` for module `fs`) keeps one file per Salvo module.

**Decided: (a)**, with the user's framing (2026-10-01): **no `*.sv.kt` /
`*.sv.rs` file in a platform root is taken into the build output as it is.**
They exist so the host project compiles and its tooling works. The build
generates its own copies, and in the output every type is in the one crate (or
the one Kotlin compilation), so the platform project may "pretend" that is
already the case. The duplicate-definition problem above therefore does not
arise in the output, and the emitted modules need no re-exports. What remains
is that the platform project's generated files define the closure of the
platform signatures, with the same names, packages and derives the output
uses, so code written against them compiles unchanged in the build. A types
file per declaring module (`<root>/fs.sv.kt` for module `fs`) holds them.

**Recorded alternative** (user, 2026-10-01; ROADMAP "Recorded, not
scheduled"): generate the *whole* emitted program into the root as `*.sv.*`,
so the host project is the build's compilation exactly and no closure is
computed. Not taken for now: it carries every function body into the root.

### D5. Union naming — decided 2026-10-01

Decided:
- **Positional unions stay** in the ABI, written clearly: Kotlin moves from
  `Union7<…>` with top-level arms `U7_1` to arms nested in the union,
  `Union7.U1(…)`, which Rust already has (`Union7::U1`). This is a rename in
  all generated Kotlin, not only the ABI.
- **The interface file adds factory functions** as a convenience at the
  boundary (`FsError.notFound(path)` or similar), so Salvo's own
  representation keeps its flexibility.
- **Factories are namespaced, and compose** (2026-10-01): a named union's
  factories hang off its type (`FsError.notFound(…)`); the anonymous union of a
  signature gets an object named after the fn or member (`ReadToStr.ok(…)`).
  Both apply at once when one union nests a named one: `Ok Str | Err FsError`
  is built as `ReadToStr.err(FsError.notFound(path))`. Each module is its own
  Kotlin package and Rust module, so two interface files cannot collide.
- **Two platform fns may not overload each other** (2026-10-01), which keeps
  each fn's factory object unique. A platform fn may still share its name with
  ordinary Salvo fns.

**The remaining cases — decided 2026-10-01.** A factory exists per runtime
arm [union-arm-identity]:

| Arm | Factory (Kotlin; Rust in snake case) |
|---|---|
| a struct `NotFound` | `notFound(value: NotFound)`: **always the struct**, never its fields, so a struct gaining a field breaks only the code that builds it |
| a qualified arm `Ok Int`, `Err E` | named after the **outermost qualifier**: `ok(n)`, `err(e)`, as Salvo's own constructors are |
| a base type `Int`, `Str`, `List<T>` | `int(…)`, `str(…)`, `list(…)` |
| literals with their base (`"STANDARD" \| "GLACIER" \| Other Str`) | **one `str(value)`**, checked at the boundary (D7): the check is what tells a listed literal from an `Other` value, so per-literal factories add nothing |
| `None` | no factory: the host writes `null` / `None` |
| two arms that would get one name (`List<Int> \| List<Str>`) | **no factory** for those arms, and a note in the generated file; the positional `UnionN.Uk` always exists |

Also decided:
- **`Checked` gets no special treatment.** It is an ordinary Salvo struct, so
  `Err Checked<SqsFailure>` is built as `GetQueueUrl.err(Checked(failure))`.
- **A named union's factories on Kotlin live in a plural object,
  `FsErrors.notFound(…)`** (option (a) below). The user asked whether
  `FsError` could instead be a sealed interface extending `Union7`. Checked
  with kotlinc on 2026-10-01: it cannot. Kotlin allows a subtype of a sealed
  interface only in the sealed interface's package, and `Union7` is in package
  `salvo` while `FsError` would be in `salvo.fs`. Even in the same package, a
  `Union7.U1` value is not an `FsError` (the cast fails at run time), so every
  value Salvo builds would need per-name arm classes, which is the per-name
  representation D5 set aside.

The options weighed for the Kotlin factory home were:
- (a) an object with another name, `FsErrors.notFound(…)` — chosen;
- (b) extensions on the union's companion, `fun Union7.Companion.notFound(…)`:
  `FsError.notFound(…)` works through the alias, but the function appears on
  every 7-arm union and two named unions in one package with a same-named arm
  clash on the JVM signature;
- (c) top-level functions, `fsErrorNotFound(…)`.
Rust needs none of this: `impl FsError { pub fn not_found(…) }` on the alias
is an inherent impl on a local type.

Also checked on 2026-10-01: Kotlin accepts the nested-arm shape
`sealed interface Union2<out A, out B> { data class U1<A>(val value: A) :
Union2<A, Nothing> … }`, so `Union2.U1(x)` needs no type arguments at the use
site.

### D6. Kotlin camel case everywhere — decided 2026-10-01

All generated Kotlin uses Kotlin's conventions, not only the ABI: camel case
for functions, members, parameters, fields and locals (`read_to_str` →
`readToStr`, `data_type` → `dataType`). Rust stays snake case. The aim is
that a Salvo project can switch target language without its host code looking
foreign in either.

**Two Salvo names that map to one Kotlin name are an error in all Salvo code**,
on every backend, so a project's validity does not depend on its target. To be
loosened later if it proves painful. Left for implementation: the scope of a
clash (one module, a struct's fields, an effect's members, one fn's locals) and
how std's intrinsics and the runtime's own names map.

### D7. What the wrappers check — decided 2026-10-01

Decided: a wrapper validates what Salvo's type promises but the host type
cannot express. Literal unions: a returned base value must be a listed
literal, or becomes `Other` when the union is open; otherwise it is a fault
naming the declaration.

**Platform handlers — decided 2026-10-01.** A handler is called through its
effect's interface, so there is no fn call to wrap. The interface file
generates an **adapter**: a class (Kotlin) or
struct (Rust) that implements the effect's Salvo-side interface, holds the
implementation, forwards each member call, and validates the result. The
`use` site constructs the adapter, which constructs the implementation with
the handler's arguments.

The values that need checking are the ones that cross **from host to Salvo**:

| Crossing | Direction | Check needed |
|---|---|---|
| handler constructor arguments | Salvo → host | none (already Salvo values) |
| member arguments | Salvo → host | none |
| member return values | host → Salvo | yes, in the adapter |
| a `Reply<T>` sent from host code, possibly later and on another thread | host → Salvo | yes, but no call returns: the check has to sit in the host-facing reply handed out by `hosted()`, typed and validating on `send` |
| a callback the host calls back into Salvo, if one is ever allowed | host → Salvo | its arguments |

**Qualifiers — decided 2026-10-01:**
- a **state** qualifier (one with a `qualifies`) is checked by running its
  `qualifies`;
- a **provenance** qualifier is trusted;
- a **constructive** qualifier is trusted when declared in the same module as
  the platform declaration, as its constructors are, and refused in the
  signature otherwise. There is no way to prove to the compiler that it
  applies.

**A failed check panics** (Rust) or **throws** (Kotlin), the way Kotlin
throws a `NullPointerException` at a Java value that broke its nullability
(2026-10-01). On a reply sent from host code, it fails on the host's thread,
in `send`.

### D8. A platform handler declares arguments, not state — decided 2026-10-01

A `platform handler` keeps its constructor parameters (`platform handler
HostCounter(start: Int) of Counter`) and declares no Salvo state; the
implementation class owns its fields. Declaring state is an error, as before
templates. For

```
platform fn shout(g: Greeting) -> Str
platform handler HostCounter(start: Int) of Counter
```

the Kotlin implementation would look like (names not final):

```kotlin
// <root>/main.kt — implementation, the developer's
package salvo.platform.main
import salvo.main.*
fun shout(g: Greeting): String = if (g.loud) g.text.uppercase() else g.text
class HostCounter(start: Int) : CounterPlatform {
    private var at = start
    override fun next(step: Int): Int { at += step; return at }
}
```

The `threadsafe` contract and `Reply` keep their rules ([threadsafe-platform],
[platform-reply]), now visible in the generated interface: Rust's `&self` or
`&mut self` trait, documentation on the Kotlin one. A Rust member's parameters
follow [rs-borrows] (`&T`, `&mut T`, `T`).

### D9. std and dependencies — decided 2026-10-01

Decided: std's host files become implementation files over generated
interfaces, written by hand like a customer's, with std's generated files in
std's root. A dependency runs its own `salvo platform generate` and checks in
the generated files, so it builds as its author tested it.

**ABI compatibility — decided 2026-10-01: (b)**, an ABI stamp.
The consumer may build with a different compiler, whose ABI or interface
files differ from the ones the dependency's implementation was written
against. Options:

- (a) **The consumer's build regenerates the dependency's ABI and interface
  files into its own output** and compiles the dependency's implementation
  files against them. That is what D3 already does for the project, so an
  incompatibility is a host compile error. That error is accurate, but it
  points into the dependency's code without saying why.
- (b) (a) plus an **ABI version** in every generated file's header (the
  compiler's ABI revision, plus a hash of the module's platform signatures,
  like [protocol-hash] for actor protocols). A mismatch with the dependency's
  checked-in copy is reported first: "dependency `aws` was generated for ABI
  N, this compiler is M; regenerate it".
- (c) Refuse to build a dependency whose checked-in files differ from what this
  compiler generates, without trying to compile.

Recommendation: (b). The build stays correct as in (a), and the stamp turns
the confusing case into a message that names the cause.

### D10. Tuples and collections at the boundary — decided 2026-10-01

Decided (user, 2026-10-01): C1 runtime types, with the ABI conveniences
below; C2 identity-keyed collections refused, sorted ones under the default
ordering allowed through ABI constructors; C3 elements checked, **with a
warning on the platform declaration** naming the cost (a walk of the returned
value) wherever a check has to look inside a collection; C4 a documented
contract; C5 borrowed results refused; C6 Kotlin keeps `Pair`/`Triple` and
renames `SalvoTupleN` to `TupleN` (all generated Kotlin, not only the ABI).

The analysis as it was put:

What the program uses today ([kt-host-abi], [rs-host-abi]):

| Salvo | Kotlin | Rust |
|---|---|---|
| `(A, B)`, `(A, B, C)`, larger | `Pair`, `Triple`, `SalvoTupleN` | native tuples |
| `List<T>` / `Mut List<T>` | `List<T>` / `MutableList<T>` | `Vec<T>` (`&mut Vec<T>` when kept `Mut`) |
| `Bytes` | `salvo.SalvoBytes` | `Vec<u8>` |
| `Map`, `Set` (the host's own hash and equality) | `LinkedHashMap` / `LinkedHashSet` as `Map` / `Set` | `SalvoMap` / `SalvoSet`, built with `from_entries::<HostHash, HostEq, _>` |
| `Map`, `Set` keyed by a Salvo identity ([cmp-carry]) | `SalvoHashMap` / `SalvoHashSet` over a pair of functions | `SalvoMap` / `SalvoSet` over generated markers |
| `SortedMap`, `SortedSet` | `java.util.TreeMap` / `TreeSet`, typed `java.util.SortedMap` / `SortedSet`, always built with a `Comparator` (even under the canonical ordering: `String.compareTo` is UTF-16 order where Salvo's is code-point order) | `SalvoSortedMap` / `SalvoSortedSet` (runtime `collections.rs`): a boxed store over a `BTreeMap`, built with `new::<C>()` for an ordering marker `C` |

Questions:

- **C1. Runtime types or host-idiomatic ones.** Exposing the program's own
  representation costs nothing at the boundary; converting to `HashMap`,
  `java.util.*` and the like costs a copy per crossing. Recommendation:
  expose the runtime types, and make them pleasant to build in the ABI. Rust:
  `FromIterator` and `From<Vec<_>>` on `SalvoMap` / `SalvoSet`, so `.collect()`
  works. Kotlin: `Map` and `Set` are interfaces, so a host could return a
  `HashMap`, whose order would make the program's output differ between
  backends. The wrapper would copy a returned map or set that is not
  insertion-ordered into one.
- **C2. Identity-keyed and sorted collections.** A host cannot easily build a
  container keyed by a Salvo hash or ordering, because it would need to supply
  that identity. Recommendation: refuse them in platform signatures for now.
  Sorted collections under the canonical ordering are allowed, but they still
  need Salvo's ordering, not the host's: a Kotlin `TreeSet<String>` with
  natural ordering sorts differently from Salvo. So the ABI provides the
  constructors, a `TreeSet` built with Salvo's canonical `Comparator` (Kotlin)
  and `SalvoSortedSet::new::<HostOrd>()` with `FromIterator` (Rust), and the
  Kotlin wrapper checks that a returned sorted collection uses that
  comparator, copying it otherwise.
- **C3. Checking elements.** By D7, a `List<"A" | "B">` or a `List<NonEmpty
  Str>` returned by the host needs every element checked, which is one walk of
  the value. Recommendation: walk, consistent with D7. The cost is
  proportional to what the host returned.
- **C4. Aliasing.** A Kotlin host receiving a `Mut List<T>` gets the
  program's own `MutableList` and could keep it after the call returns. A host
  returning a list could keep mutating it after Salvo has it. Rust's borrows
  rule out the first, and ownership the second. Options: a documented
  contract (like `threadsafe`, trusted on Kotlin), or a defensive copy in the
  Kotlin wrapper. Recommendation: the contract, since a copy changes the cost
  of every crossing.
- **C5. Borrowed results.** A platform fn whose result borrows from a
  parameter (`holds proj(p)`) would need Rust lifetimes in the interface.
  Recommendation: refuse it in platform signatures for now.
- **C6. Tuple names on Kotlin.** `Pair` and `Triple`, then `SalvoTuple4`
  upwards. Keep these, or name every arity alike (`Tuple2`…), the way unions
  are `UnionN`? The first is what Kotlin code expects for the common sizes.

### D11. Platform types and fn values — decided 2026-10-02

Opaque `platform type`s in three kinds, and effect-free fn values lent for
the call, as RUNTIME.md §12 sets out; the rules are LANGUAGE_SPEC.md's
[platform-type] and [platform-fn-value]. Generic platform types are still
to come (RUNTIME.md E1).

## What goes away

Recorded so nothing is left half-removed (no compatibility, per AGENTS.md):

- **Platform effects**: syntax, checker rules, both emitters' interfaces and
  host-owned `main` ([platform-effect], [platform-tree]'s entry rules), and
  the tests that use them (about 60 sites across 9 test files).
- **Platform templates**: `salvo_core::template`, `Hole` and the marker
  lexer/parser, `check_host_blocks`, both emitters' `render_host`,
  `HandlerDecl.{spliced, host, host_fields}`, `FnDecl.host`, the template
  grammars and the language server's template support (built 2026-09-30),
  place ascription, the `[host-splice]` rule, and the template parts of
  `salvo platform generate`.
- **The aws generator's template output.** It writes implementation files
  instead, so this cannot be removed before that port is done.

## Build sequence (complete 2026-10-01)

To be reordered once the decisions are made. Steps marked *independent* can
go in any order.

1. ✅ **Decide D1–D10** (2026-10-01).
2. ✅ **Remove platform effects** (2026-10-01).
3. ✅ **`platform fn`** (2026-10-01, [platform-fn]).
4. ✅ **Kotlin camel case** (2026-10-01, [name-camel] [kt-camel]).
5. ✅ **Kotlin union arms as `UnionN.Uk`, and `TupleN`** (2026-10-01).
5b. ✅ **Remove templates first** (2026-10-01). Done ahead of the ABI files,
    so the `*.sv.kt` / `*.sv.rs` names are free before anything generated uses
    them: implementation files are plain host code again (`<m>.kt` / `<m>.rs`),
    a `platform fn` compiles to a `<name>Platform` / `<name>_platform` wrapper
    calling the implementation's real name, `salvo platform generate` writes
    implementation skeletons for handlers and platform fns, the aws generator
    is back at its pre-splice shape (updated for camel case and `Union2.U1`),
    and a root's `*.sv.*` files are skipped by the build. This took steps 9,
    11 (partly) and 13 out of the sequence below; the aws glue still spells
    emitted names until step 8 gives it factories.
6. ✅ **The host project and ABI files** (D2, D3; 2026-10-01): written into
   the project's root by `compile`, `run`, `test` and `platform generate`,
   stale ones removed; std's and aws's checked in and checked current.
7. ✅ **Types in the host tree** (D4(a); 2026-10-01): the closure of the
   platform signatures, declarations only. Under D4 nothing moves and nothing
   is re-exported: the output keeps its own definitions. std's and aws's
   implementation files compile against the host projects alone.
8. ✅ **Interface files** (D7, D8): handler interfaces and adapters, fn wrappers
   with validation, validating host replies, factory functions (D5).
   - 8a. ✅ Host-facing interfaces and adapters (2026-10-01): `EPlatform`
     (Rust also `EPlatformSync` for `threadsafe`) and `__Platform_E` beside
     the effect, `__Platform_H` per handler; std's host files and the aws
     generator ported (this was step 10's and part of 11's work).
   - 8b. ✅ Validation in the adapters and wrappers (D7, D10 C3), host replies
     (2026-10-01, [platform-check]); D10 C5 (a platform fn's borrowed result
     refused), C1 and C2 (sets and maps in Salvo's order, identity-keyed ones
     refused) the same day.
   - 8c. ✅ Factories (D5; 2026-10-01, [platform-factory]). The aws generator
     onto them is step 11.
9. ✅ **Implementation skeletons** (done in 5b).
10. ✅ **Port std's host files** onto the generated interfaces (done in 8a).
11. ✅ **Port the aws generator** onto the generated interface files and their
    factories (2026-10-01): the glue builds every answer as
    `GetQueueUrl.ok(…)` / `GetQueueUrl.err(Checked(SqsFailures.sqsError(…)))`
    (Rust `GetQueueUrl::err(… SqsFailure::sqs_error(…))`); it reads unions
    positionally still, which the factories do not cover. Its drift test keeps
    checking against both SDKs.
12. ✅ **ABI stamps and the dependency check** (D9; 2026-10-01,
    [platform-stamp]).
13. ✅ **Remove templates** (done in 5b).
14. ✅ **Docs** (with each step, 2026-10-01): LANGUAGE_SPEC rules ([platform-abi] rewritten, new rules for the
    file kinds), both backend specs' host-ABI tables, `docs/language/Backends.md`,
    README, vscode README.

## Decision log

- 2026-10-01 (user): replace platform templates with a generated ABI,
  interface and implementation files; drop platform effects for now; revisit
  union naming and Kotlin case.
- 2026-10-01 (user): D1 `platform fn`; D2 a generated host project per root;
  D3 every build regenerates the root's files; D5 positional unions kept with
  Kotlin `UnionN.Uk`, factory functions in the interface file, namespaced so
  they cannot collide; D6 camel case in all generated Kotlin, a name clash an
  error in all Salvo code; D7 wrappers validate literal unions; D8 platform
  handlers take arguments, not state; D9 std hand-writes implementations over
  generated interfaces, a dependency checks in its own generated files.
- 2026-10-01 (user): D4 (a), with the platform root's generated files never
  copied into the output; D5 factories compose (`ReadToStr.err(FsError.…)`),
  and two platform fns may not overload each other; D7 adapters for platform
  handlers, state qualifiers checked, provenance trusted, constructive trusted
  from the same module and refused otherwise, a failed check a panic or
  exception; D9 an ABI stamp.
- 2026-10-01 (user): D10 decided (runtime types; identity-keyed collections
  refused; element checks with a cost warning on the declaration; aliasing by
  contract; borrowed results refused; `TupleN` replaces `SalvoTupleN`, `Pair`
  and `Triple` kept).
- 2026-10-01 (user): D5's remaining cases: a struct arm's factory takes the
  struct; literal arms get one checked `str(value)`; arms that would share a
  name get no factory; `Checked` is an ordinary struct; Kotlin named-union
  factories in a plural object (`FsErrors`), since a sealed `FsError` cannot
  extend `Union7` from another package.
