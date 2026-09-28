# Serialization — the option space (working document)

**Status:** OPEN. Nothing here is decided.

**Provenance.** Written 2026-09-28 by a read-only session (the only file it may
write is this one) while another agent was working in the tree, at the user's
request to explore *flexible and safe serialization and deserialization* between
domain models and external shapes (DynamoDB items, JSON objects) and which
language features would support it. It lays out the design in
options/trade-offs/recommendations style, following DESIGN_DOC.md, **and the
calls are the user's.**

**Propagation owed.** Because this session is read-only, none of the following
has happened: no decision-log entry in COMPLETED.md, no ROADMAP.md step (this
topic has no step there today; the closest are §4's manifest **DECISION** and
§17's recursive types), no LANGUAGE_SPEC.md rules (the fresh labels proposed
below are `[serde-value]`, `[destructure-exhaustive]`, `[literal-exact]`,
`[codec-auto]`, `[codec-extra]`, `[record-hash]` — none exists yet), and no
backend-spec bullets. Until the user decides, nothing has propagated. **When the
decisions land, this file is deleted**: its outcomes go to COMPLETED.md's
decision log and the specs, and a ROADMAP.md step carries the build.

**Sources.** docs/language/ pages: Data-and-Types.md (structs, defaults, spread,
destructuring, unions), Qualifiers.md (constructive and provenance qualifiers,
erasure, refinements), Implicit-Parameters.md (`params`, obligations, slots),
Comparison-and-Hashing.md (`auto fn`, the derivation mechanism), Modules.md,
Linear-Types.md. LANGUAGE_SPEC.md rules relied on, each verified present:
[struct-decl] [struct-defaults] [struct-spread] [struct-lit-infer]
[let-destructure] [placeholder] [type-no-cycle] [type-union] [type-nullable]
[union-arm-identity] [when-exhaustive] [qual-erasure] [qual-union-arm]
[qual-constructive] [qual-ctor-fn] [qual-field-override] [cmp-auto] [cmp-groups]
[fn-attached] [group-obligation] [implicit-group] [implicit-resolve]
[implicit-fn-only] [interp-struct] [interp-to-str] [checked-type]
[linear-obligation] [linear-discard] [throw] [try] [effect-not-data]
[platform-effect] [platform-handler] [intrinsic-std-only] [wire-format]
[noremote] [protocol-hash] [backend-never-wrong] [mod-export] [name-casing]
[col-hashed-ordered]. Backend labels cited as representational facts only:
[rs-wire] [kt-wire]. COMPLETED.md: "The network sequence, step ② — codecs and
the protocol hash" and decision 1 and 9 of "Actors across machines — the network
round" (schema evolution "recorded, not built"). ROADMAP.md: §4 (the manifest
DECISION, already with a second customer in the protocol lock file), §6
(recursive implicit resolution), §17 (recursive types; JSON named as a
List-mediated customer). And the user's stated intent, below.

---

## 0. The stated intent

The user's sketch, numbered so the decisions below can be judged against it.
It is a statement of *problems from practice* rather than a proposed design, so
it is used here to guide the options and price them, not to fix answers.

1. **Domain model ↔ external shape.** The everyday task is converting between a
   domain struct and a DynamoDB item or a JSON object, in both directions.
2. **Injection.** The external shape carries data the domain model does not:
   a source type modeled as an enum, key attributes, versions. A serializer is
   not a mirror of the struct.
3. **Defaults on read.** The domain model evolves and its new fields should
   not be nullable, but historical items lack them, so deserialization must
   populate a default.
4. **The mixed fleet.** While two versions run, the older one reads and writes
   items and thereby *removes* fields the newer one added, which forces
   two-stage commits (deploy readers that tolerate the field, then writers that
   produce it). The question is whether any approach mitigates or removes this.
5. **Exhaustiveness.** Can Salvo model a struct's fields exhaustively, so the
   compiler checks that a hand-written serializer has *considered* every field
   (passed it on or explicitly ignored it), and that a deserializer handles a
   newly added optional field rather than silently leaving it out?
6. **Safety with flexibility.** Whatever the answer, it must keep both: the
   author decides the external shape field by field, and the compiler catches
   the omission.

How these meet the record: (1), (2) and (3) fit the language as it stands and
are mostly a question of *what std provides* (SD-1, SD-3, SD-4). (5) collides
with two present rules, [let-destructure] (a struct pattern may list a subset)
and [struct-defaults] (a literal may omit a defaulted field), and is the
load-bearing language change (SD-2). (4) is not a language problem the compiler
can solve for binaries already deployed, but there are two mechanisms that make
the *next* evolution single-stage, and one of them wants a language feature
(SD-5). Where the store client lives is settled by [platform-effect] and only
needs stating (SD-6).

## 1. Fixed points — already decided, inherited here

- **Salvo owns its wire format, and it is positional.** [wire-format] is a
  compiler-defined binary encoding: struct fields in declaration order, union
  arms by index over the declared arms [union-arm-identity], qualifiers erased
  [qual-erasure]. It is generated for every type unless `noremote` [noremote],
  compatibility is a hash compared at the handshake [protocol-hash], and
  **decode never runs against a different schema than encode**. This is the
  right design for messages between nodes of one build and the wrong one for a
  persisted item: an item outlives the build that wrote it, is read by a
  different one, and is keyed by *names*, not positions. So the persisted
  codecs are a second thing, not an extension of `encode`/`decode`. **This is
  where §0.1 and §0.4 collide with the record**: the network round recorded
  schema evolution as "recorded, not built: unions cannot evolve, and a
  protocol gains a message as a new arm". A persisted format has to evolve.
- **One predicate, two consumers** [backend-never-wrong] [wire-format]: "it
  encodes" and "it has a codec on both backends" are one predicate in
  `salvo-core`, so the two backends cannot disagree. Any derived codec here
  inherits that shape.
- **Derivation exists and is a modifier on the function**: `auto fn cmp`,
  `auto fn eq`, `auto fn hash` [cmp-auto], and only those three ("the compiler
  can write `cmp`, `eq` and `hash`, and says so if you write `auto` on anything
  else"). Struct interpolation is derived the same way [interp-struct], with
  the explicit `to_str` winning. So "the compiler writes a structural function
  from the fields" is an accepted mechanism with a known shape; what SD-3 asks
  is whether two more members join the list.
- **Struct patterns list a subset** [let-destructure]: `let {name, age: a} = p`
  binds two fields of a three-field struct today. Used **once** in the whole
  repository (`crates/salvo-syntax/tests/corpus/structs.sv:17`), so changing
  its rule costs one rewrite. **§0.5 collides here.**
- **A literal omits a defaulted field silently** [struct-defaults]. This is
  the deserialization half of §0.5's hazard: add `created: Long = 0L` to a
  struct and every existing literal still compiles, including the one in the
  deserializer that should have read it from the item.
- **Defaults are the language's answer to §0.3 already**: a field `= expr` is
  optional at construction. What is missing is not the mechanism but the
  *check* that a codec chose to rely on it.
- **Unions are positional and exhaustive** [union-arm-identity]
  [when-exhaustive]: an old binary has no value to construct for an arm it does
  not know. Any external representation of a union has to be by *name*, and a
  decoder meeting an unknown name has to fail as an ordinary result, never a
  trap (SD-4).
- **`Checked<T>` is "you must look at this"** [checked-type] and `throw`/`try`
  is the non-resuming exit [throw] [try]. A decode failure is a value in one of
  those two shapes; this document does not propose a third.
- **Generics are opaque** ("with no bounds, nothing about a `T` is knowable",
  AGENTS.md) and a generic `T` is refused at `encode` (recorded cut, step ②).
  A generic `to_json<T>` written in Salvo therefore needs an implicit
  (`?ToJson<T>`) per element type, and [implicit-resolve] skips a candidate
  that itself needs implicits (ROADMAP §6). Nested containers in a derived
  codec are the compiler's to write, not std's, until §6 lands.
- **Host capabilities are declared as `platform effect`s** [platform-effect]
  [platform-handler]; std's primitives are `intrinsic` [intrinsic-std-only].
  A DynamoDB client is the host's capability; a JSON text parser can be either
  a runtime companion (as `wire.rs`/`wire.kt` are) or a platform effect, and
  SD-1 prices both.
- **Recursive types indirect through containers** [type-no-cycle]; ROADMAP §17
  names JSON as a customer covered by `List`-mediated recursion. A `Json` value
  type is writable today as a union over `List<Json>` and `Map<Str, Json>`.
- **Names are cased by rule** [name-casing]: fields are lowercase-initial, so a
  field cannot be spelled `createdAt`. An external key that is not a legal
  field name needs a renaming mechanism (SD-4).
- **Keyed containers have no wire form yet** (step ② cut) and a keyed container
  over a tuple is refused (§6). A JSON object is a `Map<Str, Json>`, keyed by
  `Str`, whose identity is intrinsic, so this cut does not block a `Json` type;
  it blocks deriving a codec for a struct field of type `Map<K, V>` with a
  user-defined `K` until the decoder can be handed the identity.
- **The manifest DECISION** (ROADMAP §4) already has a lock-file customer:
  effect → (declared version, protocol hash). SD-5's version lock for persisted
  shapes is a third customer of the same file, so it should be decided
  alongside rather than separately.

## 2. What other languages teach

### Rust: serde

The pattern is a derived `Serialize`/`Deserialize` with per-field attributes.
The ones that map to §0: `#[serde(default)]` and `#[serde(default = "f")]`
fill a missing field on read (§0.3); `#[serde(rename = "createdAt")]` and
`rename_all = "camelCase"` decouple the key from the field name;
`#[serde(skip)]`, `skip_serializing_if = "Option::is_none"` decide what is
written; `#[serde(flatten)] extra: HashMap<String, Value>` **collects every
key no field claims and writes them back**, which is the unknown-field
preservation §0.4 needs (and it is documented as incompatible with
`deny_unknown_fields`, which is the other posture: refuse rather than keep).
Injection (§0.2) is done with a second struct or with `#[serde(tag = "type")]`
for enums. `serde_json::Value` is the untyped tree. What to steal: the
*distinction between the two postures toward unknown keys* (keep, or refuse)
as a per-struct choice, and the observation that everything serde does with
attributes Salvo would rather do with a *second struct* whose fields are the
external shape, because Salvo has no attributes and wants none. Informs SD-3,
SD-4, SD-5.

### Kotlin: kotlinx.serialization and Jackson

`kotlinx.serialization` is a compiler plugin deriving from the class. Its
configuration is what a fleet-safety discussion turns on: `ignoreUnknownKeys`
(default *false*, so a newer writer breaks an older reader unless it is set),
`encodeDefaults` (default *false*: a property equal to its default is
**omitted**, so a reader without that default cannot tell "absent" from
"default", and `@EncodeDefault` forces it per property), `@Required` (a
property with a default that must nevertheless be present), `explicitNulls`
(whether `null` is written or the key omitted). Jackson's
`@JsonAnySetter`/`@JsonAnyGetter` pair is the unknown-key bag, and
`@JsonIgnoreProperties(ignoreUnknown = true)` the tolerant reader. What to
steal: the *four independent switches* — absent-vs-null, write-defaults or
omit, tolerate-or-refuse unknown keys, required-despite-default — are exactly
the representation decisions SD-4 has to make once, as language rules rather
than per-call configuration, because a Salvo program's output must not depend
on a configuration object. Informs SD-4, SD-5.

### Swift: Codable and `CodingKeys`

Codable synthesizes the codec; a hand-written `CodingKeys` enum lists which
properties are coded and under which names. **If a property is omitted from
`CodingKeys` and has no default value, synthesis fails to compile.** That is
§0.5 in production: the list of coded fields is checked against the type, and
the way to leave one out is to give it a default, which is visible at the
declaration. Informs SD-2 and SD-3 (the derivation may be steered by an
explicit list, and the list is checked for completeness).

### Java: MapStruct, records and record patterns

MapStruct generates mapping code between two bean types and has
`unmappedTargetPolicy = ERROR`: a target property no source property or
explicit mapping covers fails the build. This is the *two-struct* approach
with exhaustiveness on the target side, and it is how §0.2's "inject extra
data" is done there (a constant or expression per target property). Java
record patterns (`if (o instanceof Person(var name, var age))`) are
exhaustive over the components. Informs SD-2, SD-3 (the record-layer option).

### Exhaustive record patterns elsewhere

Rust struct patterns are exhaustive unless `..` is written; OCaml allows a
partial record pattern but has warning 9 ("missing fields in record pattern")
that teams turn on for exactly this reason; Haskell needs `RecordWildCards`
to be lenient; Elm and PureScript row types let a function state which fields
it needs and no more. TypeScript has no exhaustive destructuring, so the
idiom is `satisfies Record<keyof T, …>` to force an object to mention every
key. The lesson is consistent: **exhaustive is the safe default and the
opt-out is one token.** Informs SD-2.

### Zig: `inline for` over `@typeInfo`

A generic serializer is written once as a compile-time loop over the fields
of any struct, each iteration monomorphic. It is the strongest form of "the
compiler knows the fields", and it is why Zig's std has one JSON codec written
in the language. Its cost is a compile-time evaluation facility. It is the
shape-changing alternative in SD-2 and SD-3, priced there.

### Protobuf, Avro, Thrift, Cap'n Proto: schema evolution

Protobuf: fields are numbered, unknown fields are **preserved and re-emitted**
(proto3 dropped them until 3.5, then restored proto2's behaviour because the
mixed-fleet round trip broke), a missing field reads as its default, and
`reserved` keeps a retired number from being reused. Avro: the *reader's*
schema and the *writer's* schema are both present at decode and resolved
field-by-field by name; a field added to the reader needs a default, a field
missing from the reader is skipped, and the writer's schema travels with the
data or is looked up by fingerprint. Thrift: unknown fields are skipped on
read and lost on write. Buf's `breaking` check compares the schema against the
last committed one and fails the build on an incompatible change. What to
steal for §0.4: **preservation is what makes the mixed fleet safe**, and it
has to be present in *every* running version before it helps, so the cost of
introducing it is paid once; and a **schema fingerprint in a lock file** is the
mechanism that turns "did I break compatibility" into a build error, which
Salvo already does for protocols [protocol-hash]. Informs SD-5.

### DynamoDB in practice

The item is a `Map<String, AttributeValue>` where `AttributeValue` is a tagged
union (`S`, `N` as a decimal string, `B`, `BOOL`, `NULL`, `L`, `M`, `SS`,
`NS`, `BS`). Two SDK-level facts bear on §0.4: `PutItem` **replaces the whole
item**, so a writer that maps a struct to an item and puts it removes every
attribute its struct lacks, whereas `UpdateItem` with a `SET` expression
touches only the named attributes and leaves the rest; the Java Enhanced
Client's `ignoreNulls`/`ignoreNullsMode` exists because a mapped object's
`null` field would otherwise `REMOVE` the attribute. The idiomatic mitigation
in shops is therefore "write with `UpdateItem` and only the attributes you
own", and the *expand/contract* (parallel change) deployment pattern for the
rest. Informs SD-5 (partial writes as a derived change set) and SD-6.

### What Kotlin and Rust do, as backends

Neither backend needs a library. The wire runtime already exists twice
(`wire.rs`, `wire.kt`) and generated codecs sit beside every struct
([rs-wire] [kt-wire]); a JSON text printer/parser is a few hundred lines in
each, of the same kind, and the DynamoDB attribute-value model is a Salvo
union the emitters already know how to render. The only host-side piece is the
store client, which is a `platform effect` whose skeleton `salvo platform
generate` writes, and whose host body converts the Salvo `Attr` union to the
SDK's `AttributeValue` (a `when` over nine arms, in Kotlin or Rust). Nothing
in the options below requires Jackson, kotlinx.serialization or serde, and
depending on them would break the byte-for-byte parity the codec tests
assert.

---

## 3. SD-1 — What a serializer produces: the value model

**Question.** A hand-written serializer has to produce *something*. Is it a
std value type (`Json`, `Attr`) the program builds and reads like any union, a
stream of writer calls, or the host's own object? This is load-bearing: SD-2
through SD-6 assume the answer. Folds §0.1, §0.2, §0.6.

**Option A — std value types, one per external model, as ordinary unions.**

```
// std `json`
export type Json = Null | Bool | Long | Double | Str | List<Json> | Map<Str, Json>
export struct Null {}
export fn parse(text: Str) -> Ok Json | Err JsonError
export fn print(value: Json) -> Str            // canonical: sorted keys, no whitespace
export fn pretty(value: Json) -> Str

// std `ddb` (the model only; the client is SD-6)
export type Attr = S Str | N Str | B Bytes | Bool | Null
                 | List<Attr> | Map<Str, Attr> | SS List<Str> | NS List<Str> | BS List<Bytes>
export provenance qualifier S<T> of T     // and N, B, SS, NS, BS: tags, as Ok/Err are
export type Item = Map<Str, Attr>
```

A serializer is a function to `Json` or `Item`; a deserializer is a function
from one, answering `Ok T | Err DecodeError`. Recursion is through `List` and
`Map`, which [type-no-cycle] permits. The DynamoDB tags are provenance
qualifiers on `Str` because that is what they are on the wire (`N` is a decimal
string), and [qual-union-arm] keeps `S Str | N Str` two arms.

- *Cost.* Two modules of std written in Salvo plus a JSON text runtime per
  backend (parse/print, of the `wire.rs`/`wire.kt` kind). Building a `Map<Str,
  Json>` by hand is verbose without a literal shortcut; `{"name": name}` is
  already a map literal, so `{"name": s(name)}`-style constructors keep it
  short.
- *Trade-offs.* Everything is ordinary Salvo: `when` over a `Json` is
  exhaustive, `is Str` narrows, a test compares two `Json` values with `eq`.
  Injection (§0.2) is a `put`. Nothing is hidden, which is Salvo's posture.
  Number handling needs a rule: a JSON number parses to `Long` when it is an
  integer that fits and `Double` otherwise, and a decoder into an `Int` field
  range-checks. Double text follows [interp-float] so both backends print one
  string.

**Option B — a writer/reader effect (`[JsonWriter]`) with streaming members.**
`begin_object()`, `key("name")`, `str(v)`, `end_object()`; the reader side is
a pull parser. No tree is built.

- *Cost.* Two effects and handlers per format; hand-written serializers are
  longer and their *shape* is not checkable (an unbalanced `end_object` is a
  runtime error).
- *Trade-offs.* Streams large payloads without a tree. But §0's payloads are
  items and objects of tens of fields; the tree is the natural size, and a
  streaming form can be added under the tree later (a `Json` → writer walk is
  twenty lines). Refused on the same grounds `wire` chose values: a value can
  be compared, stored, tested and printed; a stream of calls cannot.

**Option C — the host's object (Jackson `JsonNode`, `serde_json::Value`,
the SDK's `AttributeValue`) reached through a `platform effect`.**

- *Cost.* Two host implementations of every member; the byte-for-byte parity
  the codec tests assert becomes impossible (Jackson and serde_json disagree
  on float formatting, key order, escaping); the customer's serializer is
  written against an effect whose members are the host library's API.
- *Trade-offs.* Zero std code. Refused for the reason decision 1 of the
  network round refused handing bytes to the host: two backends would produce
  two outputs for one program.

**Shape-changing alternative — no value model; codecs only.** Skip `Json` as a
type: `to_json(p) -> Str` and `of_json<T>(text) -> Ok T | Err …` are derived
(SD-3) and a hand-written serializer does not exist; customization is by the
record layer (a second struct). Cheapest std, and it removes the whole class
of "I built the tree wrong" mistakes. But §0.2's injection and §0.5's
exhaustiveness then live entirely in struct-to-struct conversion, and anything
the derivation cannot express (a field encoded as two keys, a legacy shape) has
no escape hatch. The value type is that hatch, and it costs little to have
both.

**Recommendation (the user's call).** Option A: `Json` and `Attr` as std
unions, with the JSON text runtime as a backend companion, and codecs (SD-3)
derived *to and from those values*, never straight to text. This is the
`wire` decision applied to a named format, and it is what makes every later
section ordinary Salvo.

## 4. SD-2 — Exhaustiveness over a struct's fields

**Question.** §0.5: can the compiler check that a codec has considered every
field? The two halves are the *pattern* side (reading a struct field by field
to serialize it) and the *literal* side (building a struct field by field when
deserializing). Folds §0.5, §0.6; collides with [let-destructure] and
[struct-defaults].

The observation that makes this small: **a serializer that destructures its
input is exhaustive if destructuring is**, and **a deserializer that builds
its output with a literal is exhaustive if the literal refuses to fill a
default silently.** No new declaration kind is needed; two existing forms
change rule.

```
struct Person {
    name: Str,
    age: Int,
    nickname: Str? = None,
    created: Long = 0L          // added later; historical items lack it
}

fn to_item(p: Person, source: Source) -> Item => !p, !source {
    let {name, age, nickname, created} = p        // (a) every field, or an error naming the missing one
    let item: Mut Item = mut_map_of()
    put(item, "pk", s("PERSON#${name}"))
    put(item, "source", to_attr(source))         // §0.2: injected, not a field of Person
    put(item, "name", s(name))
    put(item, "age", n(age))
    if nickname is Str { put(item, "nickname", s(nickname)) }
    put(item, "created", n(created))
    return item
}

fn of_item(item: Item) -> Ok Person | Err DecodeError => item {
    let name = str_at(item, "name") ^Ok?: return _
    let age = int_at(item, "age") ^Ok?: return _
    let nickname = opt_str_at(item, "nickname") ^Ok?: return _
    let created = opt_long_at(item, "created") ^Ok?: return _
    return ok(exact Person {                     // (b) every field, defaults notwithstanding
        name: name, age: age, nickname: nickname,
        created: created ?: 0L                   // the default is *written here*, where it is a choice
    })
}
```

Add `email: Str? = None` to `Person` and both functions stop compiling: (a)
names `email` as unmentioned, (b) names it as unset. Ignoring is explicit:
`let {name, age, ..} = p` on the pattern side, and on the literal side there
is nothing to ignore, since every field must be given.

### (a) The pattern side

**Option A1 — exhaustive by default, `..` opts out.** `let {name, age} = p`
is an error when `p` has other fields; `let {name, age, ..} = p` is today's
meaning. Same rule for `for {name, score} in rows` [let-destructure].

- *Cost.* One rewrite in the repository. A new token in patterns (`..`; or
  `...` to match spread, at the price of reading like a spread; or `_` as a
  bare trailing element, which [placeholder] currently reserves for "the
  value the construct left unnamed" and would then mean "the fields left
  unnamed", a defensible reading).
- *Trade-offs.* Matches Rust and the OCaml warning teams enable. A pattern
  that lists every field is also better documentation of what a function
  reads.

**Option A2 — opt-in keyword** (`let all {name, age} = p`).

- *Cost.* Nothing to rewrite. But the safe form is the one the author must
  remember, which is the mistake §0.5 wants the compiler to catch.

**Option A3 — a field-set `when`.** A new construct that iterates the fields
with a typed body per field, `when fields(p) { name: Str => …, age: Int => …
}`, exhaustive like `when` over a union. Shape-changing.

- *Cost.* A new expression form whose arms are fields; each arm is a
  different type, so the body is checked per arm. It is a destructuring with
  a body per binding, and the pattern already gives that with less syntax.
  Refused as duplicating (a).

### (b) The literal side

**Option B1 — a site-level modifier: `exact Person { … }`.** Every field
must be written, default or not; spread is refused inside it (a spread would
fill fields the author never looked at). Contextual word, like `noremote`,
`threadsafe`, `auto`.

- *Cost.* One contextual word; no sweep. The risk is the author forgetting
  it in the one place it matters, mitigated by SD-3's derived decoder being
  the common path and a hand-written one being the exception.
- *Trade-offs.* Ordinary construction keeps its defaults, which is what
  defaults are for; only a codec asks for the strict form.

**Option B2 — literals never fill defaults silently; `..` fills them.**
`Person {name: n, age: a, ..}` is the only way to take defaults; today's
`Person {name: n, age: a}` is an error naming the unset fields. Symmetric with
A1.

- *Cost.* A wide sweep: every literal in std, tests, examples and docs that
  relies on a default gains `..`. Every ordinary construction site then
  carries a token whose only purpose is the codec case.
- *Trade-offs.* Symmetry is attractive and the hazard disappears everywhere,
  not only where someone wrote `exact`. But defaults were adopted so that
  callers need not mention what they do not care about; B2 makes them mention
  that they do not care.

**Option B3 — declaration-level: the struct says its literals are exact.**
`exact struct PersonItem { … }`: a persisted-shape struct (SD-3's record
layer) opts its literals and patterns into strictness once, at the
declaration.

- *Cost.* A struct modifier. Only helps when the strict struct is the one
  being built; `of_item` above builds the *domain* struct, which the domain
  would not mark, so B3 covers the record layer and not the general case.
  Useful as sugar over B1, not instead of it.

**Recommendation (the user's call).** A1 for patterns (exhaustive by default,
`..` opts out; spelling of the opt-out the user's) and B1 for literals
(`exact` at the site), with B3 as optional sugar if the record layer lands in
SD-3. B2 is the principled alternative and the user may prefer its symmetry;
the price is the sweep and a token on every ordinary construction.

## 5. SD-3 — The derived codec, and how a hand-written one takes over

> **Superseded in part (2026-09-28).** The comptime rounds (COMPLETED.md's log, "Comptime, first slice") adopted this section's *shape-changing alternative*:
> `auto` is gone, the structural functions are `compfn`s in `core.auto` asked
> for with `by auto`, and the JSON pair is to be `compfn to_json<struct T>` /
> `from_json` in a `json` module spelled `by json` (ROADMAP §2c). Option A's
> "grow `auto`" is no longer available; Option B (the record layer) stands as
> the customization pattern, and the `Json` value type of SD-1 is the
> prerequisite the module waits on. Read the options below as the argument
> trail.


**Question.** Most fields map one-to-one; writing `put(item, "age", n(age))`
twenty times is the part serde and kotlinx remove. Should the compiler derive
the codec, and if so how does the author customize it without attributes?
Folds §0.1, §0.2, §0.3, §0.6.

**Option A — `auto fn` gains two members: `to_json`/`of_json`, `to_attr`/`of_attr`.**

```
struct PersonItem : auto Json<self> {        // declares `to_json` and `of_json`
    pk: Str, sk: Str,
    source: Source,
    name: Str, age: Int,
    nickname: Str? = None,
    created: Long? = None,
}
```

with `params Json<T> { fn to_json(v: T) -> Json; fn of_json(j: Json) -> Ok T |
Err DecodeError }` in std `json`, and `auto` writing both from the fields, as
[cmp-auto] does for `cmp`. The derived encoder writes every field under its
declared name; the derived decoder reads every field by name, **fills a
declared default when the key is absent** (§0.3, the mechanism Avro and
serde use), and answers `Err` naming the key and the reason for a missing
required key or a wrong shape.

- *Cost.* Two more `auto` members ([cmp-auto]'s list is closed by rule; the
  rule changes). The derivation recurses into field types that themselves have
  a codec (intrinsics, `List`, `Map<Str, _>`, `T?`, unions of structs per SD-4,
  structs with the obligation) and refuses at the declaration otherwise,
  naming the field, which is [interp-struct]'s posture.
- *Trade-offs.* The whole-struct case is one line. Customization has no
  attributes to hang on, which leads to the next option.

**Option B — the record layer: derive on a second struct, hand-write the
conversion.** This is not a feature; it is the pattern Option A makes
natural, and it answers §0.2 and §0.3 without any per-field mechanism:

```
struct Person { name: Str, age: Int, nickname: Str? = None, created: Long = 0L }

struct PersonItem : auto Attr<self> {      // the persisted shape, and nothing else
    pk: Str, sk: Str, source: Source,
    name: Str, age: Int, nickname: Str? = None, created: Long? = None
}

fn to_item(p: Person, source: Source) -> PersonItem => !p, !source {
    let {name, age, nickname, created} = p                        // SD-2 (a)
    return exact PersonItem {                                     // SD-2 (b)
        pk: "PERSON#${name}", sk: "PROFILE", source: source,
        name: name, age: age, nickname: nickname, created: created
    }
}

fn of_item(i: PersonItem) -> Person => !i {
    let {name, age, nickname, created, pk: _, sk: _, source: _} = i
    return exact Person { name: name, age: age, nickname: nickname, created: created ?: 0L }
}
```

Injection is a field of the record. A default is written where it is chosen.
Renaming is the record field's name (SD-4 for keys that are not legal field
names). A field encoded as two keys is two record fields. Both conversions
are exhaustive by SD-2, so adding a field to either struct breaks exactly the
two functions that must decide about it.

- *Cost.* A struct per persisted shape and two conversion functions, which
  is what MapStruct users write and what DDD shops call the persistence model.
  For a struct that *is* its own persisted shape, Option A alone suffices and
  no record struct is written.
- *Trade-offs.* Every decision is visible in ordinary code. The duplication
  is the point: the record's fields are the schema, and the domain's are not.

**Option C — `auto` with a partial body.** `auto fn to_json(p: Person) ->
Json { created: n(p.created), "schema": 2 }`: the listed fields are by hand,
the rest derived, extra keys injected inline. Exhaustive by construction.

- *Cost.* A new body form (field overrides plus extra keys) for `auto`,
  mirrored on the decoder side with a harder question (an override for a field
  the input lacks needs a default expression, which is the struct default
  again). Two ways to say one thing once Option B exists.
- *Trade-offs.* Shorter than B for a one-field exception. Recorded as the
  attribute-free equivalent of serde's per-field attributes; recommended
  against until B proves too verbose in a real program.

**Shape-changing alternative — compile-time field iteration, so std writes
the codec in Salvo.** `for field in fields<T>()` unrolled per instantiation,
each iteration monomorphic, `field.name`, `field.get(v)`, `field.default`,
with the body resolving `to_json(field.get(v))` as an implicit per field. Zig's
model. One generic `to_json<T>` in std would cover every struct and every
customization would be a Salvo function.

- *Cost.* A compile-time evaluation facility: types as values in a restricted
  context, unrolling, per-iteration implicit resolution, and [implicit-resolve]
  made recursive (ROADMAP §6) since the element's `to_json` itself needs
  implicits for its fields. The largest feature in this document by an order of
  magnitude, and it introduces a second way to know a struct's fields beside
  `auto`.
- *Trade-offs.* Removes the closed `auto` list for good and makes SD-1's
  runtime smaller. Worth recording as the direction `auto` generalizes to if a
  fourth and fifth derived member appear; not worth building for two.

**Recommendation (the user's call).** Option A, with Option B as the documented
pattern and the shape the std module's own examples take. `auto` stays a
closed list the compiler owns, grown by two groups (`Json<T>`, `Attr<T>`).

## 6. SD-4 — Representation rules for the derived codec

**Question.** The switches kotlinx and serde expose per call have to be
language rules here, since one program prints one output. Each is small; they
are gathered so they are decided once. Folds §0.2, §0.3.

**Unions.** Positional arms [union-arm-identity] cannot be persisted: reorder
the declaration and every stored item changes meaning. Options: (i) **by arm
name**: a union of structs encodes as an object with the struct's name as a
tag key (`{"type": "Imported", "feed": "…"}` in JSON, an `M` with a `type` `S`
in DynamoDB), a union of scalars by the JSON type itself (`Str | Long` needs
no tag), and a union that mixes two arms the format cannot tell apart (`S Str |
N Str` is fine, `Ok Str | Err Str` is not) is refused at the declaration; (ii)
**always tagged**, including scalars, with the arm's *type text* as the tag.
Recommendation: (i); the tag key name is fixed by the language (`type`) and a
struct with its own field named `type` is refused for the obligation, naming
the clash. An unknown tag on read is `Err UnknownArm { tag }`, never a trap.

**Optionals.** `T?` with `None`: (i) key omitted on write, absent-or-null both
read as `None`; (ii) `null` written explicitly. DynamoDB has a `NULL` type and
also refuses empty string sets, so omission is the safer default there.
Recommendation: (i) on both formats, and a decoder reads `null` as `None` for
a `T?` and as `Err` for a `T`.

**Defaults.** A field with a default is (i) always written, or (ii) omitted
when equal to its default (kotlinx's `encodeDefaults = false`). Omitting saves
bytes and destroys the distinction between "absent in old data" and
"explicitly the default", which §0.4's reader needs. Recommendation: (i),
always written. A field whose absence should be tolerated on read is a
defaulted field; nothing else is optional.

**Unknown keys on read.** (i) ignored; (ii) refused; (iii) collected into a
field declared for them (SD-5). Recommendation: ignored by default, collected
when the struct declares the bag, never refused, because refusal is what makes
the mixed fleet two-stage in the first place.

**Key names.** The declared field name, exactly. [name-casing] forbids
`createdAt` as a field, so an external schema with camelCase keys needs a
renaming: (i) a `rename` clause on the obligation (`: auto Json<self> rename
camel`), a single convention per struct; (ii) a per-field spelling (`created:
Long as "createdAt"`), the only attribute-like form in this document; (iii)
nothing, and the record layer names the key in a `Map` by hand for the rare
legacy shape. Recommendation: (i) for the convention and (iii) for the
exception, so no per-field syntax is added; the user may find (ii) worth its
weight if legacy schemas dominate.

**Numbers.** `Int`/`Long` write as JSON integers and DynamoDB `N`; `Float`/
`Double` per [interp-float] text (so `NaN` is refused at encode, as JSON has no
spelling for it); a JSON integer wider than the field is `Err OutOfRange`.
`Byte` writes as an integer; `Bytes` as base64 in JSON and `B` in DynamoDB.

**Qualifiers.** Erased [qual-erasure]: a `NonEmpty List<T>` field writes as a
list, and the decoder *re-tests* a predicate qualifier on read (the claim is
about contents, so it can be checked) and refuses to derive a decoder for a
field carrying a constructive or provenance claim, since nothing in the data
can establish it, naming the field. A `Mut` field is refused likewise; a
persisted shape is immutable data.

**Linear and `noremote`.** A struct with a linear field, a `proj` view, a fn
type, a `Pool`, a stream or an `Addr` has no persisted form, by the same
predicate `wire_blocker` already applies, extended with "and no keyed
container over a user-defined key until its identity can travel".

**Recommendation (the user's call).** The bolded choices above, as one bundle:
name-tagged unions, omitted `None`, defaults always written, unknown keys
ignored unless collected, declared field names with a per-struct casing
convention. Each is independently reversible; together they are the posture
"the item says exactly what the struct says, by name."

## 7. SD-5 — Evolution and the mixed fleet

**Question.** §0.4: version N+1 adds a field; version N, still running, reads
an item, rebuilds it from its own struct and writes it back, and the field is
gone. Can the language remove the two-stage commit? Folds §0.3, §0.4.

The honest frame first. **No compiler can protect a field from a binary that
was built before the field existed.** What a language can do is make every
binary built *from now on* behave so that the *next* addition is one stage.
Two mechanisms do that, they are independent, and both are known to work in
the fleets §2 surveys (Protobuf preserves; DynamoDB shops write partially).
The two-stage cost is paid once, to introduce either mechanism, and never
again for adding a field.

**Option A — unknown-field preservation: a declared bag.** A struct that is
a persisted shape declares one field of a std type the codec recognizes:

```
struct PersonItem : auto Attr<self> {
    pk: Str, sk: Str, name: Str, age: Int,
    extra: Extra<Attr> = extra_of()          // every attribute no field claims
}
```

The derived decoder puts each key it did not consume into `extra`; the derived
encoder writes `extra`'s entries back after the fields; a declared field
shadows an `extra` entry of the same name (the field wins, and the encoder
does not write the stale entry). `Extra<V>` is a `Map<Str, V>` wrapper so the
codec can tell it from an ordinary map field. Version N built with this reads
N+1's item, carries `created` in `extra`, and writes it back untouched.

- *Cost.* One std type; the derived codec learns one field kind; a hand-written
  codec has to do the same by hand, which is exactly the kind of omission SD-2
  cannot catch (the bag is not a field of the domain), so this is a reason to
  prefer the derived codec for the record layer. Every item read costs a map
  of the unclaimed attributes, usually empty.
- *Trade-offs.* This is serde's `flatten` bag and Protobuf's unknown field set,
  and it is what made proto3 reverse its decision. It also composes with
  `PutItem`: the whole item is written and nothing is lost. Its limit is
  semantic: version N preserves bytes it does not understand, so if N also
  changes a field N+1 derives `created` from, no mechanism saves that; that
  is a real schema change and stays two-stage.

**Option B — partial writes: a derived change set.** Instead of writing the
item, write the attributes that changed:

```
auto fn changes(old: PersonItem, new: PersonItem) -> Map<Str, Attr?>
// present with a value: SET; present with None: REMOVE; absent: untouched
```

derived from the fields with their `eq`, and the store's `update(key,
changes)` member (SD-6) turns it into an `UpdateItem` expression. Version N
touches `age` and never mentions `created`, so `created` survives.

- *Cost.* A third derived member and an `update` member on the store effect.
  It requires the program to have the *old* item in hand (it read it), which
  the read-modify-write cycle §0.4 describes already does. A create is a
  `put`; a modify is an `update`.
- *Trade-offs.* The DynamoDB-native answer, and it also removes the lost-update
  hazard between two *concurrent* writers of different fields, which Option A
  does not. It does not apply to a JSON blob stored whole. Both A and B can be
  adopted; they answer different write paths.

**Option C — a version lock for persisted shapes.** Reuse [protocol-hash]:
each struct with a `Json<self>`/`Attr<self>` obligation gets a canonical form
(field names and types, sorted by name, since this format is by name) and the
manifest's lock file records it per struct. The check is not equality but
**compatibility**: a build fails if, against the locked form, a field was
*added without a default*, a field was *removed* (rather than kept with a
default and marked as retired in a comment), a field's type changed, or a
union lost an arm. Buf's `breaking`, in the compiler.

- *Cost.* Waits on the manifest DECISION (ROADMAP §4) and adds a third
  customer to its lock file. A rule set for "compatible change" that the user
  owns.
- *Trade-offs.* Turns §0.4's most common mistake (a required field added,
  which no old item has, so every read fails after deploy) into a build
  error at the desk. It does not preserve anything at runtime; it is the
  guard that makes A and B sufficient.

**Shape-changing alternative — the format carries its schema (Avro).** Every
item holds a schema fingerprint; the decoder resolves writer-vs-reader by
name with defaults; the compiler emits the writer schema per version and a
registry of past schemas travels with the program. Removes the need for
defaults to be *in the struct*, since the resolution supplies them.

- *Cost.* A schema registry in std, a resolver in the runtime, a fingerprint
  per item, and old schemas kept in the source tree. The defaults live in two
  places (the struct's and the resolution's).
- *Trade-offs.* The strongest evolution model for data at rest, and a poor fit
  for a DynamoDB item, which is read by attribute name by other tools; also
  strictly more than §0 asked for. Recorded, recommended against.

**Marking evolution in the declaration.** A field could carry the version it
appeared in (`created: Long = 0L since 3`), letting Option C check that a
`since` field has a default and letting the lock file compare by version
rather than by hash. Cheap syntax, but it duplicates what the lock file
knows; recommended only if the user wants the history readable in the source.

**Recommendation (the user's call).** A (`Extra<V>` bag, declared per struct)
as the general answer because it protects both formats and `PutItem`; B
(derived `changes`) for DynamoDB read-modify-write paths, since it is the
idiom the platform rewards; C when the manifest lands, as the guard. The
two-stage commit is then paid once per table, to deploy the first version that
preserves.

## 8. SD-6 — Where the store client lives

**Question.** DynamoDB is the host's. How does a Salvo program reach it, and
what crosses the boundary? Folds §0.1; settled in principle by
[platform-effect], stated here so the model and the client are not confused.

**Option A — a `platform effect Ddb` over the `Attr` model.**

```
export platform effect Ddb {
    fn get(table: Str, key: Item) -> Ok Item? | Err Checked<DdbError>
    fn put(table: Str, item: Item) -> Ok None | Err Checked<DdbError>
    fn update(table: Str, key: Item, changes: Map<Str, Attr?>) -> Ok None | Err Checked<DdbError>
    fn delete(table: Str, key: Item) -> Ok None | Err Checked<DdbError>
    fn query(table: Str, key_condition: Str, values: Item) -> Ok List<Item> | Err Checked<DdbError>
}
```

The host body, in Kotlin or Rust from `salvo platform generate`, converts the
`Attr` union to the SDK's `AttributeValue` and back, a `when` over nine arms
each way. A `MemDdb` handler in Salvo (a `Map<Str, Map<Str, Item>>` behind
the same effect) is the test double, as `MemFs` is for `Fs`.

- *Cost.* One effect, two host skeletons the customer fills (or std ships a
  reference host under `std/platform/`, as it does for `net`), one double.
- *Trade-offs.* The codec never touches the SDK; the program is testable
  without a table; the errors are `Checked` [checked-type] so a failed write
  cannot be dropped in silence. Conditional expressions and transactions are
  members added when a program needs them.

**Option B — the platform effect speaks the *domain* type.** `fn
put<T>(table, value: T)` with the host doing the mapping.

- *Cost.* Generic members over an erased `T` the host cannot see; the mapping
  moves into Kotlin and Rust, twice, unchecked by Salvo.
- *Trade-offs.* Refused: it is the host-owns-serialization option SD-1 refused,
  reached from the other side.

**Shape-changing alternative — a `Store<T>` effect over records, in std, with
`Ddb` beneath it.** `effect Store<T> { get(key) -> T?; put(T); update(old,
new) }` implemented once in Salvo over `Ddb` and the derived codec, so a
program says `[Store<PersonItem>]` and never sees an `Item`.

- *Cost.* An effect-typed generic handler ([effect-generic-decl] exists for
  effects) whose members use the `Attr<T>` obligation, which is the recursive
  implicit case of §6 unless the handler is per-type. Worth writing as a
  library once SD-3 lands, not as a language feature.

**Recommendation (the user's call).** Option A, with `MemDdb` beside it and a
reference host in `std/platform/`, and the `Store<T>` layer recorded as the
library that grows on top.

---

## 9. Decisions pending

Load-bearing order: **SD-1 first** (everything else produces or consumes its
value types), then **SD-2** (the language change; independent of SD-1 but the
reason the hand-written path is safe), then SD-3 and SD-4 together (the derived
codec and its rules are one design), then SD-5, then SD-6. SD-5's Option C
waits on ROADMAP §4's manifest DECISION and should be folded into it.

| Label | Question | Answers | Recommendation |
|---|---|---|---|
| SD-1 | What a serializer produces | §0.1, §0.2, §0.6 | std unions `Json` and `Attr`/`Item`, a JSON text runtime per backend, codecs to and from the values, never straight to text |
| SD-2 | Exhaustiveness over fields | §0.5, §0.6; collides with [let-destructure] [struct-defaults] | Struct patterns exhaustive by default with a one-token opt-out (`..`); `exact Name { … }` literals refuse to fill defaults; optional `exact struct` sugar for the record layer |
| SD-3 | Derived codec and customization | §0.1, §0.2, §0.3 | `auto` grows `Json<T>` and `Attr<T>` (`to_json`/`of_json`, `to_attr`/`of_attr`); the record layer (a second struct plus two exhaustive conversions) is the documented customization; no attributes, no partial-auto |
| SD-4 | Representation rules | §0.2, §0.3 | Unions tagged by arm name under `type`; `None` omitted; defaults always written; unknown keys ignored unless collected; declared field names with a per-struct casing convention; erased qualifiers re-tested on read |
| SD-5 | Evolution and the mixed fleet | §0.3, §0.4 | `Extra<V>` bag per persisted struct (preservation); derived `changes(old, new)` for partial `UpdateItem` writes; compatibility lock in the manifest when it lands. Two-stage paid once, to deploy the first preserving version |
| SD-6 | The store client | §0.1 | `platform effect Ddb` over `Item`/`Attr` with `Checked` errors, `MemDdb` double, reference host in `std/platform/`; a `Store<T>` library on top later |

What this document does not claim: that any of it removes the two-stage commit
for binaries already deployed (§7's first paragraph), or that a hand-written
codec can be made exhaustive over data that is *not* a field of the struct (the
`Extra` bag, injected keys), which is why the derived codec is the recommended
path for the persisted shape and the hand-written conversion is recommended
between two structs, where SD-2 checks both ends.
