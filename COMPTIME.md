# Compile-time field iteration — the option space (working document)

**Status:** DECIDED, rounds 1–7 (2026-09-28) — see §§10, 12–17 for the decided lists and §17.1 for where it stands;
§§3–8 are kept for the argument trail;
where a decision supersedes a recommendation, §10 says so.

**Provenance.** Written 2026-09-28 by a read-only session at the user's request,
after SERDE.md's SD-3 recorded compile-time field iteration as the
shape-changing alternative to growing `auto`, and the user asked whether the
same mechanism could **remove `auto` from the language** by letting std write
the structural functions (`cmp`, `eq`, `hash`, `to_str`, the codecs) in Salvo.
This document takes that feature on its own. It lays out the design in
options/trade-offs/recommendations style, following DESIGN_DOC.md, **and the
calls are the user's.** Another agent was working in the tree while this was
written; only this file and SERDE.md were touched.

**Propagation owed.** Nothing has propagated: no COMPLETED.md decision-log
entry, no ROADMAP.md step (the feature has none; its neighbours are §6
recursive implicit resolution, which it depends on, and the manifest §4, which
it does not), no LANGUAGE_SPEC.md rules. The fresh labels proposed below do
not exist yet: `[comptime-bound]`, `[comptime-fields]`, `[comptime-inline]`,
`[comptime-access]`, `[comptime-refuse]`, `[comptime-instantiate]`,
`[obligation-by]`, `[fn-by]`. `[cmp-auto]` would be **deleted**, and
`[interp-struct]` rewritten. **When the decisions land, this file is
deleted**; its outcomes go to COMPLETED.md's decision log and the specs, and a
ROADMAP.md step carries the build. SERDE.md's SD-3 should then be re-read
against the outcome, since its recommendation (`auto` grows two groups)
assumes this document was *not* adopted.

**Sources.** docs/language/: Comparison-and-Hashing.md (`auto`, attachment,
`@` selection), Implicit-Parameters.md (`params`, obligations, slots, binder),
Generics-and-Aliases.md, Data-and-Types.md (struct interpolation), Modules.md,
Backends.md. LANGUAGE_SPEC.md rules relied on, each verified present:
[cmp-auto] [cmp-groups] [cmp-carry] [cmp-binder] [cmp-hash-values]
[fn-attached] [group-obligation] [group-self] [implicit-param]
[implicit-resolve] [implicit-forward] [implicit-fn-only] [implicit-group]
[interp-struct] [interp-to-str] [decl-explicit] [fn-rename] [fn-overload-at]
[fn-overload-rank] [call-resolve] [struct-decl] [struct-defaults]
[struct-mut] [type-canbe-mut] [let-destructure] [when-exhaustive]
[union-arm-identity] [qual-erasure] [type-unknown-lenient] [linear-generics]
[copy-fn] [mod-export] [mod-import] [name-casing] [wire-format] [noremote]
[protocol-hash] [backend-never-wrong] [effect-generic-decl]. Backend labels
cited as representational facts only: [rs-wire] [kt-wire]. COMPLETED.md: the
ordering round (2026-09-21/22, `auto` as a modifier on the function), the
interpolation derivation (2026-09-11), step ② of the network sequence
(generated codecs beside every struct). ROADMAP.md §6 (recursive implicit
resolution). SERDE.md SD-3 and the conversation that followed it. Zig's
`comptime`, `@typeInfo`, `inline for`, `@field`, `@compileError`, and
`std.json`, checked against the Zig language reference.

---

## 0. The stated intent

The user's sketch, numbered so the decisions below can be judged against it.
Points 1–3 are the user's; 4–6 are what the conversation established and the
user did not object to. All are treated as *direction*, used to guide the
options and price them.

1. **Use compile-time field iteration to remove `auto` from the language.**
   `cmp`, `eq`, `hash` (and by extension `to_str` and the SERDE codecs) become
   std functions written in Salvo, and `auto` has nothing left to do.
2. **`by X` names where the body comes from.** On an obligation clause
   (`struct P : Ordered<self> by structural`) `X` supplies a comptime function
   per group member, stamped out at the type. The user's reading, confirmed:
   the same works per function, `fn cmp(a: P, b: P) -> Int by structural`.
3. **The generated code should not get worse.** Today `auto` emits one
   specialized body beside the struct on both backends; whatever replaces it
   must land there too.
4. **Opt-in must survive.** "Orderable is an ordering being in scope"; a struct
   without a declared `cmp` cannot be compared. A comptime `cmp<T struct>`
   visible everywhere would undo that.
5. **Generic code stays opaque.** A comptime function is instantiated at a
   concrete type only; from a generic body with an opaque `T` the remedy is an
   implicit, as it is for everything else.
6. **Kotlin erases generics**; the design must not require monomorphizing
   arbitrary generic functions on that backend.

How these meet the record: (1) collides with [cmp-auto] ("the compiler can
write `cmp`, `eq` and `hash`, and says so if you write `auto` on anything
else") and with [interp-struct]'s compiler-derived `to_str`; both are what the
feature replaces (CT-5). (2) is new syntax on the obligation clause
[group-obligation] and on a fn declaration (CT-4). (4) and (5) are the rules
that make (1) safe (CT-3, CT-4). (3) and (6) are a backend constraint that the
instantiation rule satisfies (CT-3). The reflection surface itself, what the
program is allowed to know about a type at compile time, is CT-1, and the
constructs that consume it are CT-2.

## 1. Fixed points — already decided, inherited here

- **`auto` is a modifier on the function, and the list is closed** [cmp-auto]
  (user decisions 2026-09-21/22): `auto fn cmp`, `auto fn eq`, `auto fn hash`
  in a struct body, or `: auto Group<self>` as shorthand for one per member.
  Generated bodies are structural in declaration order; a field the compiler
  cannot compare (a `Double`, a fn) and a `canbe Mut` struct are refused **at
  the declaration**. std uses the clause form nine times (`Duration`,
  `Instant`, `Tick`, `NodeEndpoint`, `NodeId`, and comments in `map`, `set`,
  `sorted`); examples once. **§0.1 collides here**; the sweep is small.
- **A function declared on a type travels with it** [fn-attached], via a body
  member or a same-file fulfilment of an obligation; `cmp@Point` names it. A
  detached fulfilment of an exported type must itself be `export`. Whatever
  `by` attaches must obey the same visibility.
- **The obligation clause is checked at the struct** [group-obligation]: each
  member must be satisfied by a visible fn overload, equal positionally up to a
  bijective renaming of type variables, `self` standing for the struct
  [group-self]. `by` is an addition to this check, not a replacement.
- **Generics are opaque** (AGENTS.md; "with no bounds, nothing about a `T` is
  knowable"). There are no bounds today; `canbe linear` and `canbe Mut` on a
  type parameter are the only things written after one. **`T struct` would be
  the first bound** (CT-1).
- **Implicit resolution is per call site and skips a candidate that itself
  needs implicits** [implicit-resolve] (ROADMAP §6). A comptime `to_json` over
  a field of type `List<Address>` needs `to_json<Address>` resolved *inside* a
  generic candidate. **§6 is a prerequisite** for the codec case and not for
  `cmp`/`eq`/`hash` over scalar fields.
- **Implicitly resolved functions are effect-free** [implicit-fn-only]. A
  comptime function stamped out by `by` is not resolved implicitly, but the
  instantiation it produces will be, so it must be effect-free too.
- **An identity lands in the type** [cmp-carry] [cmp-binder]: `Set<Person>(
  hash, eq)`, `Heap(min_by_age)`. A comptime instantiation used as an identity
  has to have a *name* the type can print; `hash@Point` does, and so would
  `hash<Point>@structural`.
- **Struct interpolation is derived by the compiler** [interp-struct] for
  structs whose fields all render natively, in Salvo's literal shape, with an
  explicit `to_str` winning and *no recursion into a field that itself needs
  one*. Applies to **every** struct with no opt-in. **§0.1 collides here** too,
  and the collision is instructive: one derivation is opt-in (`cmp`), one is
  default-on (`to_str`); the replacement must express both (CT-4).
- **Generated codecs sit beside every struct** on both backends [wire-format]
  [rs-wire] [kt-wire], driven by one predicate in `salvo-core`
  [backend-never-wrong]. Their byte-for-byte parity is asserted by tests. This
  document does **not** propose moving them to std (CT-5).
- **Kotlin generics erase; Rust monomorphizes.** A generic Salvo fn is one
  Kotlin body with implicits passed as values; the wire codecs are
  per-struct objects for that reason (BACKEND_SPEC.kotlin.md, [kt-wire]).
  **§0.6 is a constraint on CT-3.**
- **Nothing the compiler cannot see is inferred** [decl-explicit]: a fn with
  no body declares its effects, deductions and return type. A `by` declaration
  has no body of its own, so it is in this class.
- **`rename fn` gives an overload a name and removes it from the old name's
  candidate set** [fn-rename]. Related to, but not the same as, `by`: `rename`
  points at an existing overload; `by` stamps a new one.
- **Unions are positional and exhaustive** [union-arm-identity]
  [when-exhaustive]. A compile-time walk over a union's arms (for a tagged
  codec) would iterate the declared arms in order, so it is consistent with the
  identity rule by construction.

## 2. What other languages teach

### Zig: `comptime`, `@typeInfo`, `inline for`, `@field`

The model this document transplants. Types are first-class values at compile
time; `@typeInfo(T)` answers a tagged union describing `T`, whose `.Struct.fields`
is a slice of `StructField { name, type, default_value, is_comptime, alignment }`;
`inline for (fields) |f|` unrolls the loop so each iteration's body is analyzed
with `f.type` concrete; `@field(v, f.name)` reads or writes a field by a
compile-time string; `@compileError("…")` refuses an instantiation with a
message; `@typeName(T)`. `std.json.stringify` is one function: `switch
(@typeInfo(T))` over kinds, `inline for` over fields in the struct arm,
recursing on each field's type. There is no derive step because there is
nothing to derive: the generic function *is* the implementation for every
type. Two things worth stealing beyond the mechanism: **partial evaluation is
demanded, not inferred** (a function is comptime where it is called with
comptime arguments; `inline` is written), which keeps the analysis local; and
**a struct is built field by field** via `@field(result, name) = …` on an
`undefined` value, which Salvo cannot do and CT-2 must answer differently.
Informs CT-1, CT-2, CT-3.

### D: `static foreach`, `__traits`, `static if`, `static assert`

The same design a decade earlier. `__traits(allMembers, T)` and
`T.tupleof` expose the fields, `static foreach` unrolls, `static if` selects,
`static assert(cond, msg)` refuses. D's experience is the caution: an
unrestricted reflection surface (`allMembers` includes methods, aliases,
nested types) makes generic code that quietly does more than the author
intended. Informs CT-1's recommendation to expose *fields of a struct* and
nothing else in the first cut.

### Nim: `fieldPairs` and macros

`for name, value in obj.fieldPairs:` is an unrolled iteration over the fields
of an object with `name` a compile-time string and `value` typed per
iteration; it is how Nim's `std/json` `%` operator and its `==` for objects
are written. Nim shows the feature can be *spelled as an ordinary loop* with
the unrolling implied by the iterator's kind; Zig makes it explicit with
`inline`. Salvo's [decl-explicit] posture favours the explicit word. Informs
CT-2.

### Scala 3: `inline`, `Mirror`, `summonInline`

Derivation is written in the language: `inline def derived[T](using m:
Mirror.ProductOf[T])` walks `m.MirroredElemTypes` with `inline` matches and
`summonInline[Show[t]]` per element type, so the instance for each field is
resolved at compile time. `Mirror` is the compiler-provided reflection value
(labels, element types), everything else is a library. This is the closest
analogue to Salvo's situation, since the per-field instance is an *implicit*
resolved per unrolled element, which is exactly what `cmp(a.[field],
b.[field])` does in CT-2. Scala also shows the cost: error messages from a
failed `summonInline` deep in an unrolled derivation are notoriously poor, so
the diagnostic rule in CT-3 (name the type, the field, and the missing
capability) is not optional. Informs CT-2, CT-3.

### Rust: derive macros, no reflection

Rust has no compile-time field iteration; `#[derive]` runs a procedural macro
over the *token stream* of the declaration, in a separate crate, before type
checking. It shows the alternative shape: derivation as syntax transformation.
Refused for Salvo on the grounds Salvo has already applied to attributes:
the derivation should be checked Salvo code, not a program over tokens.
Rust's `const fn`/const generics are compile-time evaluation *without* type
reflection, and so do not help here. Informs the shape-changing alternative in
CT-2.

### C++: templates, `constexpr`, and P2996 reflection

Templates instantiate per type and `if constexpr` selects, but there is no
field enumeration without the reflection proposal (P2996, `std::meta`,
`nonstatic_data_members_of(^^T)` with `template for` to expand). Its
decades-long absence is the strongest evidence that per-field iteration is
the missing piece, not per-type instantiation; every serialization library
in C++ (Boost.Serialization, cereal, Boost.PFR's structured-binding trick)
is a workaround for it. Informs CT-1.

### Swift, Kotlin, Java: compiler-owned synthesis

Swift synthesizes `Equatable`/`Hashable`/`Codable` in the compiler;
kotlinx.serialization is a compiler plugin; Java records synthesize `equals`/
`hashCode`/`toString`, and anything else is an annotation processor. These are
`auto` in three dialects: a closed list owned by the compiler, extended only by
the compiler's authors. Each has grown a plugin or macro system to escape the
list. Informs §0.1: the closed list is the thing to remove, and the languages
that kept it all grew an escape hatch later.

### Haskell: `GHC.Generics`

Derivation as a library over a generic *representation* (`Rep a`, a
sum-of-products encoding with metadata) with one type class instance per
representation node. Entirely in the language, no unrolling: the type checker
resolves the structure. Elegant and slow to compile, with error messages one
representation away from the user's type. Informs CT-2's choice of explicit
unrolling over a representation type.

### What Kotlin and Rust do, as backends

Rust monomorphizes every generic instantiation, so a comptime function's per-
type body is what rustc would have produced anyway. Kotlin erases, so a
generic Salvo fn is one JVM method; a body that differs per `T` cannot be one
method. Today the emitters already produce **per-struct** artefacts beside
each declaration: `impl __Wire for S` / `object __Codec_S` [rs-wire] [kt-wire],
and the `auto` bodies. So the backends have the *shape* a comptime
instantiation needs (a named, specialized body beside a concrete type) and lack
only a general rule for when to emit one. CT-3's instantiation rule is what
supplies it, and it is designed so that Kotlin never has to specialize a
function whose type argument is not written in the program.

---

## 3. CT-1 — What a program may know about a type at compile time

**Question.** The reflection surface. What does `fields<T>` answer, what is a
`field`, and what else (if anything) is visible? This is load-bearing: every
other section consumes it, and its size is the feature's blast radius. Folds
§0.1, §0.5.

**Option A — the fields of a struct, and only that.**

- `T struct` is a **bound** on a type parameter: `T` is some struct type,
  concrete at instantiation (CT-3). Written after the parameter like `canbe
  linear` is (`fn cmp<T struct>(a: T, b: T) -> Int`). It is the first bound
  in the language, and it unlocks exactly one thing: `fields<T>`.
- `fields<T>` is a **compile-time sequence** of the struct's declared fields in
  declaration order. It is not a value: it may appear only as the subject of
  `inline for` (CT-2). Each element `field` exposes:
  - `field.name` — a `Str` literal;
  - `field.Type` — a type, usable wherever a type is written inside the loop
    body (`of_json<field.Type>`, `let x: field.Type`);
  - `field.default` — a compile-time optional: the field's `= expr` as a
    value of `field.Type`, or `None`. Reading it is *evaluating the default
    expression* at that point, as a literal would;
  - `field.first`, `field.last` — `Bool` literals, for separators;
  - `T.name` — the struct's declared name as a `Str` literal, for `to_str`.
- Nothing else: no methods (structs have none), no attached functions, no
  qualifiers, no visibility, no documentation.

*Cost.* One bound, one intrinsic sequence, five projections. The checker's
work is in CT-2's unrolling, not here.
*Trade-offs.* Covers every `auto` today and every SERDE codec over structs.
Does not cover a union, so a name-tagged union codec (SERDE SD-4) is still the
compiler's, or is written as a `when` per union by hand.

**Option B — A, plus the arms of a union.** `arms<T>` for `T union`, each
`arm.Type`, `arm.name` (the struct's name for a struct arm, the type text
otherwise), `arm.index`; consumed by an `inline when v { … }` that unrolls a
`when` with one body per arm. Makes `to_json` complete over Salvo's data
model.

*Cost.* A second bound and a second unrolled construct (`inline when`), whose
body binds `v` at `arm.Type` per arm, which is a narrowing the checker already
does for `is`. Arm identity stays positional [union-arm-identity] because the
iteration is in declaration order.
*Trade-offs.* Without it, SERDE's codecs derive struct bodies in Salvo and
union tags in the compiler, two generators again. With it, one.

**Option C — general `typeinfo<T>`** (Zig's whole `@typeInfo`): kind
switching over scalars, containers, functions, effects, qualifiers.

*Cost.* A type-level union the checker must model and keep in step with every
type feature; every later feature grows it. D's lesson.
*Trade-offs.* Would let std write `to_str` for lists and maps generically; but
those are intrinsics with intrinsic `to_str`s already, and the win is small
against the surface.

**Shape-changing alternative — no reflection; a compiler-provided
representation type** (Haskell's `Rep`). Each struct gets an implicit
isomorphism to a nested tuple of its fields with a metadata type; derivation is
ordinary generic code over tuples with per-element implicits. No unrolling
construct, no bound; the type checker does the work through recursive implicit
resolution (§6, made total).

*Cost.* Recursive implicit resolution over deep tuple types on every
derivation, and error messages one encoding away from the source. Kotlin
would still need per-struct specialization since the tuple depth is per type.
*Trade-offs.* Smaller language surface than any option above, larger
compile-time and diagnostic cost. Recorded; recommended against because Salvo
has chosen explicitness over encoding everywhere else.

**Recommendation (the user's call).** Option A for the first cut, with B
scheduled as the second step *if* SERDE's tagged unions are adopted: the two
bounds and two constructs are parallel, and B's cost is mostly the second
construct.

## 4. CT-2 — The constructs that consume it

**Question.** How a body iterates fields, reads them, builds a struct from
them, and refuses an instantiation. Folds §0.1, §0.3.

### The four forms

```
inline for field in fields<T> { … }     // unrolled: one copy of the body per field, checked per copy
inline if <compile-time Bool> { … }     // kept or dropped per instantiation; the dropped branch is not checked
v.[field]                               // read a field by compile-time name; type field.Type
refuse "message with ${T.name}"         // an error at the instantiation site, in the caller's terms
```

and, for construction, a literal whose fields are produced by the loop:

```
exact T {
    inline for field in fields<T> {
        [field]: <expression of type field.Type>
    }
}
```

`inline` is the word because Zig's is, and because [decl-explicit]'s posture is
that partial evaluation is *written*, never inferred from the subject's kind
(Nim's spelling). The loop body is an ordinary block, so `return`, `break`
and `let` work; a `break` in an unrolled loop ends the remaining copies.

### The worked functions

`core.structural`, the module `by structural` names (CT-4):

```
// Lexicographic in declaration order, as `auto fn cmp` is today.
export fn cmp<T struct>(a: T, b: T) -> Int => a, b {
    inline for field in fields<T> {
        let c = cmp(a.[field], b.[field])      // resolves at field.Type, per copy
        if c != 0 { return c }
    }
    return 0
}

export fn eq<T struct>(a: T, b: T) -> Bool => a, b {
    inline for field in fields<T> {
        if !eq(a.[field], b.[field]) { return false }
    }
    return true
}

export fn hash<T struct>(value: T) -> Long => value {
    inline if T canbe Mut {
        refuse "structural hash of ${T.name}: a Mut-capable struct can change while a container holds it"
    }
    let h = 17L
    inline for field in fields<T> {
        h = h * 31L + hash(value.[field])      // per-field hashes are host-specific [cmp-hash-values]
    }
    return h
}
```

`core`'s `to_str`, replacing [interp-struct] and losing its no-recursion limit:

```
export fn to_str<T struct>(value: T) -> Str => value {
    let out: Mut Str = mut_str("${T.name} { ")
    inline for field in fields<T> {
        inline if !field.first { append(out, ", ") }
        append(out, "${field.name}: ${to_str(value.[field])}")
    }
    append(out, " }")
    return out
}
```

SERDE's encoder and decoder (`json` module), showing the construction form and
the default:

```
export fn to_json<T struct>(v: T) -> Json => v {
    let obj: Mut Map<Str, Json> = mut_map_of()
    inline for field in fields<T> {
        inline if field.Type is Extra<Json> {          // SERDE SD-5's bag
            for (k, x) in v.[field] { put(obj, k, x) }
        } else {
            put(obj, field.name, to_json(v.[field]))
        }
    }
    return obj
}

export fn of_json<T struct>(j: Json) -> Ok T | Err DecodeError => j {
    if j !is Map<Str, Json> { return err(NotAnObject {}) }
    return ok(exact T {
        inline for field in fields<T> {
            [field]: when get(j, field.name) {
                is Json { of_json<field.Type>(it) ^Ok?: return _ }
                is None { field.default ?: return err(Missing { key: field.name }) }
            }
        }
    })
}
```

The three `auto` refusals reappear as ordinary outcomes: a `Double` field has
no `cmp`, so the copy for that field fails resolution and CT-3's diagnostic
names `Point.y: Double` and the missing `cmp`; a fn-typed field the same; a
`canbe Mut` struct hits the `refuse`. Nothing in the compiler knows these
rules any more; std states them.

### The decisions inside the section

**Construction.** Zig writes fields into an `undefined` value; Salvo has no
partially initialized struct. Options: (i) the **literal form** above,
`[field]:` inside `inline for` inside a literal, which reads as what it is and
keeps `exact`'s guarantee (every field produced or the literal is refused);
(ii) a **builder intrinsic** `build<T>(each: inline (field) -> field.Type)`,
which needs a lambda whose parameter is a compile-time field and whose return
type varies per call, a new kind of lambda; (iii) **no construction**, decode
answers a `Map<Str, Json>` and the user writes the literal by hand, which
loses `of_json` and half the point. Recommendation: (i).

**Type tests at compile time.** `inline if field.Type is Extra<Json>` and
`inline if T canbe Mut` are compile-time predicates on types. Options: (i)
allow exactly `is <Type>` (equality up to alias expansion, with `Q _` matching
any qualifier-carrying type if needed later) and `canbe Mut` / `canbe linear`;
(ii) a general type-predicate language. Recommendation: (i); anything more is
Option C of CT-1 by another door.

**`refuse`.** Options: (i) a statement legal only inside a comptime body
(`T struct` in scope), reported at the *instantiation site* with the caller's
type substituted and the message as written; (ii) reuse `throw` at compile
time, which conflates two things. Recommendation: (i).

**Shape-changing alternative — a macro layer** (Rust's `derive`, Scala 2
macros): `by X` runs a Salvo program over the *declaration* and emits
declarations. Strictly more powerful (it can add fields, rename, generate
several functions) and strictly worse for the reader, who must run the macro
in their head to know what exists. Refused on the same grounds Salvo refuses
attributes and undeclared members: everything visible, nothing conjured.

**Recommendation (the user's call).** The four forms plus the literal
construction, with the inner decisions as marked.

## 5. CT-3 — When a comptime function is instantiated, and what the backends emit

**Question.** A body that differs per `T` cannot be one erased Kotlin method,
and a body checked per copy needs diagnostics that point somewhere useful.
Where may a comptime function be instantiated, and what does each
instantiation become? Folds §0.3, §0.5, §0.6.

**Definition.** A **comptime function** is one with a `T struct` (or, under
CT-1 B, `T union`) bound. It is checked **once generically** for everything
that does not depend on the bound (syntax, effect list, deductions, the shape
of every statement outside an `inline` construct) and **once per
instantiation** for the unrolled copies.

**Option A — concrete instantiation only, at three kinds of site.**

1. **An obligation clause with `by`** (CT-4): `struct Point : Ordered<self> by
   structural` instantiates `cmp<Point>@structural` at the declaration.
2. **A fn declaration with `by`** (CT-4): `fn cmp(a: Point, b: Point) -> Int
   by structural`.
3. **A concrete call**: `to_json(item)` where `item: PersonItem` resolves the
   comptime overload with `T := PersonItem`; `of_json<PersonItem>(j)` with the
   type argument written [call-type-args]. This is ordinary [call-resolve]
   with one extra rule: the candidate is admissible only when `T` binds to a
   **concrete** struct type.

A call from a generic body with an opaque `T` does **not** instantiate: the
comptime candidate is skipped, and the diagnostic names the remedy, which is
the remedy generic code always has: declare `?Ordered<T>` (or `?to_json: (T)
-> Json`) and let the caller supply it [implicit-forward]. The colouring is
the one implicits and effects already have.

*Cost.* One admissibility rule in resolution; the emitters gain **one
specialization per (comptime fn, concrete type)**, emitted **beside the type**
on both backends exactly where `auto` bodies and `__Codec_S` go today, named
for the pair (`cmp__structural__Point`, or the existing mangling). The set of
instantiations is finite and syntactically visible: every one is at a
declaration or at a call whose type is written or inferred concrete in the
program.
*Trade-offs.* Kotlin never specializes a fn whose type argument is not in the
source [§0.6]. Rust would have monomorphized anyway. Nothing about the generic
calling convention changes: an instantiation is an ordinary fn, so it can be
named (`cmp@Point`), passed as a value, and land in a type as an identity
[cmp-carry].

**Option B — instantiation anywhere, with monomorphization on Kotlin.** Allow
a comptime call from a generic body, and specialize *the caller* too, all the
way up to the concrete root.

*Cost.* A whole-program monomorphization pass for the Kotlin backend that
today has none, applied transitively to every generic fn that transitively
calls a comptime one; a change to the erased calling convention every existing
generic function uses.
*Trade-offs.* Generic code could call `to_json(x)` for an opaque `x`. But that
is exactly what `?to_json` gives without a new lowering, and Salvo already asks
generic code to declare what it needs.

**Shape-changing alternative — comptime functions are a different
declaration kind** (`comptime fn`), not ordinary fns with a bound, and cannot
be called at all: only `by` instantiates them. The simplest rule, and no
resolution change.

*Cost.* `to_json(item)` is not writable; every use needs a `by` declaration
first (`fn to_json(v: PersonItem) -> Json by json`). For codecs over dozens of
record structs that is a line per struct per direction, which is what `auto`
costs today. Loses nothing `auto` has; gains nothing over it but std
authorship.
*Trade-offs.* Attractive as a *first step*: ship `by` and the std module,
defer the concrete-call rule. Recorded as the staging option.

**Diagnostics (any option).** An error inside an unrolled copy is reported
**at the instantiation site** with three names: the comptime fn and where it
is declared, the field the copy was for (`Point.y: Double`), and what failed
in the caller's terms (`no cmp for Double in scope`). A `refuse` message is
reported the same way, verbatim, with `${T.name}` substituted. Scala's
`summonInline` failures are the cautionary example; this rule is not optional.

**Recommendation (the user's call).** Option A, staged through the
shape-changing alternative if the concrete-call rule proves larger than
expected: land `by` and `core.structural` first (that alone deletes `auto`),
then admit comptime candidates at concrete calls.

## 6. CT-4 — `by`: opt-in, attachment, and what `X` is

**Question.** §0.2 and §0.4. `by X` names where a body comes from. What may
`X` be, what does the resulting function attach to, and how does the
opt-in/default-on distinction survive?

### What `X` is

The user's reading is right: **`X` names a scope holding comptime functions,
and `by` looks each required member up there by name**, instantiates it at the
type, and attaches the result. Two forms, and they are one rule applied to one
member or to a group:

```
// Group form: every member of Ordered<self> (just `cmp`) and of Hashed<self>
// (`hash`, `eq`) is looked up in `structural` and stamped out at Point.
struct Point : Ordered<self> by structural, Hashed<self> by structural { x: Int, y: Int }

// Function form: this one overload, from this scope.
struct Person : Hashed<self> {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) -> Int by structural
    fn hash(value: Person) -> Long by structural
    fn eq(a: Person, b: Person) -> Bool { return cmp(a, b) == 0 }     // by hand, as today
}
```

The function form is `auto fn` with the generator named instead of implied.
It keeps its full signature [decl-explicit]: the return type and parameter
types are what the lookup is checked against (the comptime fn's signature with
`T := Person` must equal it, positionally, up to the [group-obligation]
renaming), so a `by` that points at something of the wrong shape is refused at
the declaration and not at a use.

What `X` may be — the options:

- **(i) A module.** `by structural` names `core.structural` (or an imported
  module); the member name is looked up *in that module's exports*. Reads well,
  and a shop's own conventions are a module: `by ids` where `ids` exports
  `cmp<T struct>`, `eq`, `hash` comparing an `id` field only.
- **(ii) A function.** `fn cmp(a: P, b: P) -> Int by by_age` names a comptime
  fn directly, whatever its name. Needed when the generator is not called what
  the member is called. In the group form a fn cannot serve (a group has
  several members), so this is function-form only.
- **(iii) Both**, disambiguated syntactically: a lowercase name that resolves
  to a module is (i), to a fn is (ii); the `@` selector works as everywhere
  (`by cmp@ordering`).

*Recommendation:* (iii), because (i) is the everyday spelling and (ii) is the
escape the function form exists for. A module named like a fn is a
[name-casing] impossibility, so the two never collide.

### Attachment and visibility

The stamped function **is declared on the type** [fn-attached]: it travels with
`Point` to every importer, `cmp@Point` names it, `?cmp` resolves to it by
default, and it lands in a type as an identity (`Set<Point>(hash@Point,
eq@Point)`) [cmp-carry]. This is exactly what `auto` produces, so nothing
downstream changes. Visibility follows the struct's, as an `auto` member's
does; the generator itself must be visible at the declaration (`core.structural`
is in `core`, so nothing is imported for the common case).

### Opt-in versus default-on

Two derivations exist today with opposite postures, and the replacement must
express both without a special case:

- **`cmp`/`eq`/`hash` are opt-in.** Under CT-3's rule a comptime `cmp<T
  struct>` in a module is an *admissible candidate at a concrete call* wherever
  the module is visible. If `core.structural` were in `core`, `cmp(p, q)` would
  resolve for every struct in every file, which §0.4 forbids. Options: (a) put
  the structural functions in a module that is **not implicitly visible**
  (`import structural`), so a file that wants them for ad-hoc calls says so and
  `by structural` reaches it without an import because `by` resolves a module
  path, not a name in scope; (b) mark them **`by`-only** (`comptime fn`, CT-3's
  staging option) so they are never candidates at a call; (c) accept that
  visibility of a module is the opt-in. *Recommendation:* (a), with (b) as the
  staging step. Under (a) a `Set<Person>` built in a file that imported
  `structural` carries `hash<Person>@structural` in its type, which prints and
  compares like any identity and is not `Person`'s own; the type says so.
- **`to_str` is default-on.** [interp-struct] renders every struct with no
  declaration. The comptime `to_str<T struct>` lives in `core` and is
  admissible everywhere; an explicit `to_str` on the type wins by the
  ordinary [fn-overload-rank] (a concrete overload beats a generic one), which
  is the rule [interp-struct] states by hand today. Its no-recursion limit
  disappears because the per-field `to_str` resolves per copy.

So the distinction is **which module the generator lives in**, and the
language has one rule.

### Interaction with `rename fn`

`rename fn` [fn-rename] gives an existing overload a name and removes it from
the old name's set; `by` creates an overload. They compose: `rename fn
by_name = cmp(a: Person, b: Person)` after a `by` declaration works as for any
overload. No new interaction.

### What is deleted

`auto` as a keyword, [cmp-auto] as a rule, the compiler's structural
generators for `cmp`/`eq`/`hash`, and [interp-struct]'s derivation code.
Sweep: nine sites in std, one in examples, the `auto` snippets in
Comparison-and-Hashing.md and Implicit-Parameters.md, LANGUAGE_SPEC.md, the
README's "features at a glance" (`: auto Hashed<self>`), and the syntax
references in COMPLETED.md/ROADMAP.md. Every site rewrites mechanically to
`by structural`.

**Recommendation (the user's call).** `by` in both forms; `X` a module or a
fn; attachment identical to `auto`'s; opt-in by module placement
(`structural` importable, `to_str` in `core`); `auto` deleted in the same
change.

## 7. CT-5 — What moves to std, and what stays the compiler's

**Question.** §0.1 says "remove `auto`". Which derivations become Salvo, and
which stay generated? Folds §0.1, §0.3.

**Moves to std, written in Salvo:**

| Today | Where it goes | Depends on |
|---|---|---|
| `auto fn cmp` / `eq` / `hash` [cmp-auto] | `core.structural` (importable, CT-4) | CT-1 A, CT-2, CT-4 |
| struct `to_str` derivation [interp-struct] | `core` | CT-1 A, CT-2; recursion into fields is free |
| SERDE's `to_json`/`of_json`, `to_attr`/`of_attr` (SD-3) | `json`, `ddb` modules | CT-1 A (+ B for tagged unions), CT-2 construction form, ROADMAP §6 for container fields |
| SERDE's `changes(old, new)` (SD-5 B) | `ddb` | CT-1 A, `eq` per field |
| SERDE's `Extra<V>` handling (SD-5 A) | inside `to_json`/`of_json` | CT-2's compile-time `is` |

**Stays compiler-generated:**

- **The wire codecs** [wire-format] [rs-wire] [kt-wire]. Their byte-for-byte
  parity across backends is asserted by the codec tests, the encoding is stated
  in `salvo-core/wire.rs` once and implemented in each runtime, and the
  `wire_blocker` predicate [noremote] drives both the checker's refusal and the
  emission [backend-never-wrong]. Rewriting them in Salvo would move a tested
  invariant into std for no reader's benefit. *Option:* revisit once CT-1 B
  exists and the std codecs have run for a while; the wire format is a
  fixed-shape codec that comptime could express, and one generator would then
  cover both. Recorded, not recommended now.
- **The protocol hash** [protocol-hash]: a compiler constant over a protocol's
  canonical form; effects are not structs and CT-1 exposes nothing about them.
- **The `noremote` predicate**: a type-graph walk the checker owns.
- **`copy`** [copy-fn], **`eq`/`cmp`/`hash` of intrinsics**: intrinsic already.

**Shape-changing alternative — move the wire codecs too**, and delete the
`__Codec_S` emission: one comptime `encode<T>` in `net` over `fields<T>` and
`arms<T>`. The most uniform outcome and the one with the least compiler.
Priced above; the parity tests would still hold since both backends run the
same Salvo. Deferred rather than refused.

**Recommendation (the user's call).** The table's first five rows move; the
wire codecs stay, revisited after CT-1 B.

## 8. CT-6 — Edges the worked functions run into

**Question.** The small rules the feature needs to be total, gathered so they
are decided once. Each is independent.

- **Recursive implicit resolution (ROADMAP §6).** `to_json(v.[field])` where
  `field.Type = List<Address>` resolves the intrinsic `to_json<T>(List<T>,
  ?to_json: (T) -> Json)`, which itself needs an implicit; [implicit-resolve]
  skips it today. `cmp`/`eq`/`hash`/`to_str` over scalar and struct fields do
  not hit this; codecs over container fields do. *Options:* land §6 first, or
  ship CT with `cmp`/`eq`/`hash`/`to_str` and gate the codecs on §6.
  *Recommendation:* the latter; the feature's first customers do not need §6.
- **Generic structs.** `fields<Wrapper<Int>>` substitutes the argument, so a
  `field.Type` is concrete; `fields<T>` where `T := Wrapper<U>` with `U`
  opaque is not a concrete instantiation (CT-3) and is refused with the
  implicit remedy. `to_str` today excludes generic structs ("their field types
  would need substituting"); under CT they are included at concrete types.
- **Defaults.** `field.default` evaluates the declared `= expr` where it is
  read, exactly as a literal omitting the field would; an effectful default
  is already impossible ([implicit-fn-only]'s reasoning, defaults are
  expressions in a declaration). `None` for a field without one; the
  compile-time optional is consumed by `?:` as any optional is.
- **Linear and `proj` fields.** `a.[field]` reads by the same rules as
  `a.name`: a `proj` field reads as a view, a linear field cannot be moved out
  of a borrowed `a` [linear-generics]. A comptime `hash` over a struct with a
  linear field therefore fails at the copy for that field with the ordinary
  linear diagnostic, which is right: a token has no hash.
- **`Mut` and deductions.** A comptime fn's deduction clause is written once
  (`=> a, b`) and applies to every instantiation; `inline for` bodies cannot
  make it false because a read by name is a read. A comptime fn that *mutates*
  (`v.[field] = …` with `v: Mut T`) is legal under [struct-mut] when `T canbe
  Mut`; `inline if T canbe Mut` is how the body guards it.
- **Effects.** A comptime fn may declare an effect list like any fn; an
  instantiation carries it. One reached by implicit resolution must be
  effect-free [implicit-fn-only], so `by` for a `params` member checks the
  instantiation is effect-free, as the obligation check does for a hand
  fulfilment.
- **Qualifiers on `T`.** `fields<T>` sees the declared field types with their
  declared qualifiers; a value's *current* qualifiers ([qual-field-override]'s
  narrowings) are not part of the type and not visible. Qualifiers on `T`
  itself erase [qual-erasure]: `cmp<NonEmpty Point>` is `cmp<Point>`.
- **`Unknown`.** A `field.Type` is never `Unknown`; a struct with an
  unresolvable field type has already errored at its declaration. An
  instantiation at an `Unknown` `T` is skipped leniently [type-unknown-lenient].
- **Nested and dot-named structs** (`Environment.Id`): `T.name` is the full
  dotted name; the Kotlin emitter places the specialization after the outer
  class as it does the codec (the lesson from step ②).
- **Tests.** Each std comptime fn gets a `.test.sv` annex beside it, which is
  how `heap`, `list`, `map`, `range`, `string` are covered today; the existing
  `auto` tests in the crates become tests of `by structural` with the same
  assertions.

**Recommendation (the user's call).** The bullets as written; the one with a
real choice is the first, and its recommendation is to ship `cmp`/`eq`/`hash`/
`to_str` before §6.

---

## 9. Decisions pending

Load-bearing order: **CT-1** (the surface) and **CT-2** (the constructs) are
one design and come first; **CT-3** (instantiation) is what makes them
implementable on Kotlin and must be decided before any emitter work; **CT-4**
(`by`) is the user-facing spelling and the deletion of `auto`; CT-5 and CT-6
follow. SERDE.md's SD-3 is re-read after CT-4.

| Label | Question | Answers | Recommendation |
|---|---|---|---|
| CT-1 | What is knowable at compile time | §0.1, §0.5 | `T struct` bound; `fields<T>` with `name`, `Type`, `default`, `first`/`last`; `T.name`; nothing else. `arms<T>` + `inline when` as step two if SERDE's tagged unions land |
| CT-2 | The consuming constructs | §0.1, §0.3 | `inline for`, `inline if` (type `is`, `canbe`), `v.[field]`, `refuse`, and `[field]:` inside an `exact` literal for construction; no macro layer |
| CT-3 | Instantiation and backends | §0.3, §0.5, §0.6 | Concrete instantiation only (clause, `by` fn, concrete call); one specialization per (fn, type) beside the type on both backends; generic callers use implicits; errors name fn, field and missing capability at the instantiation site. Stage via `by`-only if needed |
| CT-4 | `by X`, opt-in, attachment | §0.2, §0.4 | `by` on the clause and on a fn; `X` a module (member by name) or a fn; result attached as `auto`'s is; opt-in by module placement (`core.structural` importable, `to_str` in `core`); `auto` deleted |
| CT-5 | What moves to std | §0.1 | `cmp`/`eq`/`hash`, struct `to_str`, SERDE's codecs and `changes`; wire codecs, protocol hash, `noremote` stay compiler-owned, revisited after CT-1 B |
| CT-6 | Edges | — | Ship `cmp`/`eq`/`hash`/`to_str` before ROADMAP §6; codecs over container fields wait on it; the rest as written |

What this document does not claim: that comptime is smaller than `auto`. It is
larger, by the four constructs and the bound; the return is that the next
structural function is a std module rather than a compiler change, and that
the three special-cased refusals in `auto` become ordinary resolution
failures with ordinary diagnostics.

---

## 10. Round 1 — decided (user, 2026-09-28)

1. **Structs and unions from the start** (CT-1: Option B, not A). The
   surface is designed against both so it is not fitted to one.
2. **A new declaration kind: `compfn`.** Deliberately ambiguous between
   "compiled function" and "comptime function". It is the scope in which the
   comptime syntax is legal, and it is **not a callable function**: nothing
   resolves to a `compfn` at a call. Its only consumer is `by` (CT-4). This
   supersedes CT-3 Option A's third site kind (a concrete call instantiating
   the generic): with `compfn` uncallable, *every* instantiation is at a `by`
   site, and the resolution change CT-3 priced is not needed. What survives of
   Option A is its posture: concrete instantiation only, one specialization per
   (compfn, type) beside the type on both backends, and generic code reaches
   the capability through implicits. (The staging option of CT-3 became the
   design.)
3. **Spelling.** `T.fields` and `T.arms` (not `fields<T>`); `v.[field]` to
   read and `[field]:` to build; `T.name`; the projections **lower-case**:
   `field.name`, `field.type`, `field.default`, `field.first`, `field.last`;
   likewise `arm.name`, `arm.type`, `arm.index`.
4. **The worked functions to ship**: `eq`, `hash`, `cmp`, `to_str`,
   `to_json`, `from_json` (renamed from `of_json`). The DynamoDB pair is
   **not** in this slice.
5. **CT-2 as recommended**: `inline for`, `inline if`, `inline` as the written
   word, `refuse`, the literal construction form, and the initial restriction
   of compile-time type tests to `field.type is <Type>` and `canbe`.
6. **CT-4 as recommended, both forms**: `by X` on an obligation clause (every
   member looked up in `X` by name) and on a fn declaration with its full
   signature. `X` is a module, or a `compfn` named directly. Attachment is
   what the language now has — a function declared *inside* the struct or as
   an obligation fulfilment is on the type [fn-attached] — and a `by`
   declaration is one of those two, so nothing about attachment is new.
7. **Nothing auto-applies, `core` included.** Opting in is a line, so a
   struct that wants to interpolate declares `: ToStr<self> by auto`. Being in
   `core` means only that the module can be named without an `import`. So the
   builders live in **`core.auto`**, and the spelling everywhere is **`by
   auto`**. Consequences: [interp-struct]'s default derivation is **deleted**
   (a struct with no `to_str` does not interpolate, and the diagnostic names
   `: ToStr<self> by auto` as the remedy); `core.structural` is not created;
   CT-4's "opt-in by module placement" distinction is moot, since nothing is a
   candidate at a call.

Sweep implied by the round (for the build step, not this document): every
`: auto G<self>` → `: G<self> by auto` (nine in std, one in examples, the docs
and README snippets); every struct interpolation that relied on the derived
`to_str` gains `: ToStr<self> by auto`; the keyword `auto` is removed from the
grammar; [cmp-auto] and [interp-struct] are deleted from LANGUAGE_SPEC.md and
replaced by the `[comptime-*]`, `[obligation-by]`, `[fn-by]` rules.

## 11. What the round left open

Ordered by how much they shape the rest. The first three are language calls;
the remainder are small and could be settled by recommendation.

**R-1 — How a union opts in.** A struct has a declaration to carry `: Ordered
<self> by auto`; a union is usually a `type` alias (`export type FsError =
NotFound | …`) or written inline (`Str | Int`), and neither carries an
obligation clause today. Options: (a) **obligation clauses on `type`
declarations** — `export type Source = Manual | Imported : ToStr<self> by
auto` — so a *named* union opts in like a struct, and an inline union has no
functions of its own (it is spelled at a use, not declared); (b) **unions never
opt in on their own**: a `compfn` over a struct handles a union *field* inline
by iterating `field.type.arms` (so `T.arms` is reached through a field, never
as the root), and a standalone `cmp` for a union is written by hand; (c) both.
*Recommendation:* (a) — it keeps "a capability arrives by declaration" uniform
across the two kinds, and (b)'s reach-through is still available inside a
struct's compfn.

**R-2 — Kind selection in a `compfn`.** A `compfn` is instantiated at a struct
or a union. Options: (a) **the kind is a bound on the parameter** —
`compfn cmp<T struct>(…)` and `compfn cmp<T union>(…)` are two overloads in
`core.auto`, and `by auto` picks the one whose bound the type satisfies; (b) one
`compfn cmp<T>` with `inline when T { is struct { … } is union { … } }` inside;
(c) both. *Recommendation:* (a). It is the shape overloading already has, each
body stays single-purpose, and a module that supports only structs simply has
no union overload, so `by` on a union says so at the declaration.

**R-3 — `by auto` on a generic struct.** `struct Wrapper<T> : Ordered<self> by
auto { value: T }` stamps `cmp<Wrapper<T>>` with `T` opaque, and
`cmp(a.[value], b.[value])` cannot resolve. Options: (a) **refuse for v1**,
naming the field, with a hand-written `cmp<T>(a: Wrapper<T>, b: Wrapper<T>,
?Ordered<T>)` as the remedy; (b) the stamped function **gains an implicit** for
every field whose type mentions a type parameter (`?Ordered<T>` here), which is
what the hand-written version declares — the [group-obligation] match would
then ignore trailing implicits, the same relaxation ROADMAP §16 wants for
`iter fn`. *Recommendation:* (a) now, (b) recorded as the follow-up, since it
shares its prerequisite with an existing item.

**R-4 — The `inline when` construct for arms.** Iterating `T.arms` needs a
way to *dispatch* on a union value per arm with the value typed at `arm.type`
in each copy. Options: (a) `inline when v { [arm] { … } }` — an unrolled
`when` whose one written arm is stamped per declared arm, `v` narrowed to
`arm.type` inside, exhaustive by construction; (b) `inline for arm in T.arms {
if v is [arm] { … } }` — no new construct, but the checker must know the chain
is exhaustive to type the fall-through, and a `when` is the form Salvo already
uses for that. *Recommendation:* (a), as the dual of `[field]:` in a literal.
Building a union value needs nothing: an `arm.type` value coerces into the
union as any arm does.

**R-5 — Where `to_json`/`from_json` live, and the `Json` type.** `core.auto`
holding them would make `core` depend on a `Json` value type; SERDE SD-1 (the
`Json` union in a `json` module) is still undecided and is a prerequisite for
this slice. Options: (a) a `json` module exporting the type *and* the two
compfns, spelled `by json`; (b) `json.auto` as a sub-module, spelled `by
json.auto`, keeping "`by <something>.auto`" as the visible convention; (c) put
them in `core.auto` and the `Json` type in `core`. *Recommendation:* (a) with
SD-1 Option A decided alongside; `by json` reads as well as `by auto` and
keeps `core` free of a format.

**R-6 — Field defaults in `from_json`, and `exact`.** The construction form
in a `compfn` iterates every field, so it is exhaustive by construction and
needs no `exact`. Whether `exact` exists at all is SERDE SD-2 and stays there;
this document should stop writing `exact T { … }` in its examples and write
`T { … }`. *Recommendation:* the literal inside a `compfn` is written `T {
inline for … }`; SD-2 decides the hand-written case independently.

**R-7 — Tag key and arm names in `to_json`.** SERDE SD-4's union
representation (tag under `type`, arm name from the struct for a struct arm,
JSON type for scalars) is what `to_json<T union>` will encode; it is a SERDE
decision, listed here because the union overload cannot be written without it.

**R-8 — Small confirmations**, each with a default: a `compfn` declares its
return type and deduction clause like an `intrinsic fn` does [decl-explicit]
(default: yes); a `compfn` must be effect-free because its instantiations are
resolved implicitly [implicit-fn-only] (default: yes, refused at the
declaration otherwise); a `compfn` may be `export`ed and a user module may hold
them, reached by `by mymodule` (default: yes); a nested struct field's
`to_json(v.[field])` resolves the *stamped* `to_json` on the field's type, so a
nested struct must itself opt in, and the diagnostic at the outer `by` names
the field and the missing declaration (default: yes, this is the opt-in rule
applied at every level); `T.name` is the dotted name for a nested struct
(default: yes).

Not open, because the round closed them by implication: the CT-3 diagnostic
rule (fn, field, missing capability, reported at the `by` site) stands;
CT-5's table stands with the DDB rows removed and `by json` for the codec
rows; CT-6's ordering (ship `cmp`/`eq`/`hash`/`to_str` before ROADMAP §6;
`to_json`/`from_json` over container fields wait on it) stands.

---

## 12. Round 2 (user, 2026-09-28) — decisions and the analyses they asked for

**Decided.** `by` is the binding by default. R-1: named unions opt in through
an obligation clause on a `type` declaration. R-2: kinds as bounds, spelled
**`<struct T>`** and **`<union T>`** (kind before name). R-4: `inline when v {
[arm] { … } }`. R-5: the JSON pair lives in a `json` module, not in `core`;
spelled `by json`. R-6 and R-7 punted to after v1 (see below for what v1 must
still do about unions). R-8 confirmed as defaulted.

### 12.1 A concrete type with one field handled by hand

The case: a twenty-field struct whose `cmp` is structural except for one
`Double` field that needs a total order (or a tolerance). Three mechanisms
answer it at three levels of specificity; none is a new construct beyond a
small extension to `compfn`.

**(a) A `compfn` at a concrete type.** A `compfn` with **no type parameter**
has exactly one instantiation, itself, so it is declared where a fn is (in the
struct body or as a fulfilment), attaches like one, and *is* its stamped
result. The comptime syntax is legal inside because the word says so; the
only change to §10.2 is that "reached only by `by`" applies to *generic*
compfns, and a concrete one is instantiated at its declaration.

```
struct Reading : Ordered<self> {
    sensor: Str, unit: Str, /* … sixteen more … */
    value: Double

    compfn cmp(a: Reading, b: Reading) -> Int => a, b {
        inline for field in Reading.fields {
            inline if field.name == "value" {
                let c = total_cmp(a.value, b.value)     // by hand: NaN sorts last
                if c != 0 { return c }
            } else {
                let c = cmp(a.[field], b.[field])
                if c != 0 { return c }
            }
        }
        return 0
    }
}
```

It needs one addition to CT-2's compile-time predicates: **`field.name ==
"literal"`** (equality of a compile-time `Str` with a literal). A misspelled
name is an error at the declaration ("`Reading` has no field `valeu`"), since
`field.name`'s possible values are known. *Cost:* the predicate, and the rule
that a parameterless `compfn` is its own instantiation. *This is the general
answer*, and the recommended one: it is six lines, every decision is in the
body, and it composes with anything.

**(b) A qualifier on the field.** `value: Total Double` with `qualifier Total
of Double` and `fn cmp(a: Total Double, b: Total Double) -> Int` in the file.
`T.fields` sees declared field types *with their qualifiers* (CT-6), so the
copy for `value` resolves the more specific overload [qual-overload] and the
generic `by auto` stamps the right thing with nothing written in the
struct. *Cost:* the qualifier and its constructor (`total(x)`), which every
construction of `Reading` must now use. Right when the field's *type* is what
differs (it is a total-ordered double everywhere it appears), wrong when only
this struct's comparison differs.

**(c) An overload in scope.** For the `Double` case specifically, the reason
`auto` refuses today is that `Double` has no `cmp` in `core`. Declaring `fn
cmp(a: Double, b: Double) -> Int` in the struct's file makes every `by auto`
in that file resolve it for every `Double` field, because resolution runs at
the `by` site. Zero mechanism; the least specific of the three.

*Recommendation:* (a) as the mechanism the language provides, with (b) and
(c) documented as what already falls out. Not recommended: a per-field
override list on `by` (`by auto(value: total_cmp)`), which is SERDE SD-3's
"partial auto" again — a second way to say what (a) says, with a mini-language
of its own.

### 12.2 Free unions, and why v1 cannot punt on them entirely

A free union (`Str | Int`, and every **`T?`**) has no declaration to opt in
with, so it never has functions of its own. Recursive implicit parameters are
*not* the route: a union of arbitrary arity is not the shape of a generic fn.
The route is **reach-through**: `field.type.arms` is legal wherever
`field.type` is a union, and a struct's compfn dispatches on the field inline:

```
inline if field.type is union {
    inline when v.[field] {
        [arm] { inline if arm.type is None { /* omit */ } else { put(obj, field.name, to_json(v.[field])) } }
    }
}
```

Two consequences for v1, since `nickname: Str? = None` is in every second
struct: `T.arms` **includes the `None` arm** (with `arm.type` `None`), so the
compfn decides what absence means, and the compile-time predicates gain **`is
union`** / **`is struct`** on a type (already implied by R-2's bounds). What
*is* punted (R-7): the JSON *representation* of a union with struct arms
(SERDE SD-4's tag). v1's `to_json` handles `T?` and scalar-only unions and
**refuses** a struct-arm union field at the `by` site, naming SD-4. `cmp`/`eq`
/`hash`/`to_str` over union fields need no representation and are not
affected.

### 12.3 R-3: implicit parameters on compfns from the start

What it takes for `struct Wrapper<T> : Ordered<self> by auto { value: T }`:

1. **Resolution inside a copy records a need instead of failing.** When
   `cmp(a.[value], b.[value])` meets an opaque `T`, the copy notes "needs
   `cmp: (T, T) -> Int`" rather than erroring. This is the same code path as
   the CT-3 diagnostic, taking a different branch when the type is a parameter
   of the struct rather than a concrete type with no overload. Small.
2. **The stamped fn's signature gains the implicits**, deduplicated by (name,
   type): `cmp(a: Wrapper<T>, b: Wrapper<T>, ?cmp: (T, T) -> Int)`. This is
   the signature a hand-written fulfilment declares today, so nothing
   downstream is new: call sites fill it [implicit-resolve], generic callers
   forward it [implicit-forward], and a `Set<Wrapper<Int>>` carries
   `cmp@Wrapper` with the implicit resolved at the construction site
   [cmp-carry]. Kotlin passes the fn value as it does for every generic body.
   Small.
3. **[group-obligation] must ignore trailing implicit parameters** when
   matching the stamped fn against the group member. This is the relaxation
   ROADMAP §16 already needs for `iter fn`'s hidden `next` ("implicits are the
   callee's business, resolved at the call, not part of the member's shape").
   Medium, and shared.

The one principle at stake: (2) infers part of a signature from a body, which
[decl-explicit] resists. The precedent that permits it is that deductions are
inferred from bodies already [deduce-infer]; the implicit is a fact about the
body of the same kind, and it is printed in the language server like an
inferred deduction. std has a customer waiting: `Checked<T>` has no `to_str`
because it "would need the element's own `to_str` as an implicit"
[checked-type].

*Estimate:* roughly a third again on top of the base feature, most of it item
3, which is owed anyway. *Recommendation:* include it from the start; R-3's
"refuse for v1" was recommended when the relaxation looked like extra work
rather than shared work.

### 12.4 Where recursive implicit resolution (ROADMAP §6) still stands

The document covers it in §1 (fixed points), CT-6's first bullet and CT-5's
table; the question deserves a plain answer. It is needed **only for codecs
over generic container fields**: `to_json(v.[field])` with `field.type =
List<Address>` resolves the intrinsic `to_json<T>(items: List<T>, ?to_json: (T)
-> Json)`, a candidate that itself needs an implicit, which [implicit-resolve]
skips today. It is **not** needed by `cmp`, `eq`, `hash` or `to_str`, whose
list/map cases are backend intrinsics [col-hashed-ordered] [col-to-str] with
no implicit to resolve, nor by 12.3, which is about a stamped fn *acquiring*
implicits rather than resolving through one. So the order stands: `core.auto`
ships without §6; `json`'s pair over scalar, struct, `T?` and free-union
fields ships without §6; `List<Struct>` and `Map<Str, Struct>` fields in
`to_json`/`from_json` wait on it.

### 12.5 Left for a next round

- Confirm 12.1(a): a parameterless `compfn` is its own instantiation, and
  `field.name == "literal"` joins the compile-time predicates.
- Confirm 12.2: `T.arms` includes the `None` arm; `is struct`/`is union` as
  type predicates; v1 `to_json` refuses struct-arm unions naming SD-4.
- Confirm 12.3: implicits on stamped fns from the start, including the
  inferred-implicit principle.
- SERDE SD-1 (the `Json` type) must be decided before the `json` module is
  written; SD-4's tag and SD-2's `exact` after v1.

---

## 13. Round 3 (user, 2026-09-28) — the kind `when`, unions, and what §6 needs

### 13.1 Decided: kinds are a `when`, and it is exhaustive

R-1's predicates become a `when` over a type, so that adding a kind to the
language makes every compfn that did not consider it a compile-time error
rather than a silent skip:

```
inline when field.type {
    is struct    { … }
    is union     { … }
    is plain     { … }      // the word is open — see below
}
```

Three things to settle inside it, each with a recommendation:

- **The kind set must be complete for the `when` to be exhaustive.** Salvo's
  types today: intrinsic types (scalars, `None`, `Str`, `Bytes`, `List`, `Set`,
  `Map`, arrays, and whatever `intrinsic type` declares), structs, unions,
  tuples, function types (`(A) -> B`, `iter T`), and, inside a compfn stamped
  at a generic struct (12.3), an opaque **type parameter**. Aliases expand
  before classification. So the arms are **`struct`, `union`, `tuple`, `fn`,
  `param`**, and the one for everything intrinsic. A compfn that has nothing to
  say about tuples writes `is tuple { refuse "…" }`, which is the point.
- **The word for the intrinsic kind.** `plain` is the user's suggestion. The
  language already has a word: `intrinsic type Str canbe Mut` [type-basic]
  [backend-intrinsic], so **`is intrinsic`** names exactly the set (a type whose
  declaration says `intrinsic`) with no new vocabulary, where `plain` would
  need defining and reads as "not qualified" to someone who knows qualifiers.
  *Recommendation:* `intrinsic`; `plain` if the user prefers the shorter word,
  defined as "declared `intrinsic type`".
- **`else`.** A `when` with conditions permits `else` [when-condition]; the kind
  `when` could too, and then a compfn with an `else` is not protected by the
  evolution check. *Recommendation:* permit it (it is a `when`, and a compfn
  that genuinely treats three kinds alike should be able to say so), and
  write std's compfns without it. The protection is then the author's choice,
  stated by the absence of `else`, the same way a `..` opt-out reads on a
  pattern.
- **Spelling: `inline when` or `when`.** The user's sketch wrote `when
  field.type`. A `when` whose subject is a *type* can only be compile-time, so
  `inline` is redundant there; but every other comptime construct is written
  `inline` (§10.5), and one rule ("comptime constructs say `inline`") is
  easier to hold than "except where the subject makes it obvious".
  *Recommendation:* `inline when`, for the one rule; the user's call.

The `is` forms of 12.2 (`inline if field.type is union`) remain as the
one-kind shorthand, exactly as `if x is A` stands beside `when x`.

### 13.2 Unions: what the reach-through gives, and the separate question

Confirmed reading: a union value *is* one of its arms, so a union field's
`to_str`/`cmp`/`to_json` is the arm's, reached by `inline when v.[field] {
[arm] { … } }` with each copy at `arm.type`; **every arm must have the
capability**, and a missing one is reported at the `by` site naming the field
and the arm. No recursion of implicits is involved.

The case the user had in mind is a **different feature**: an ordinary generic
fn taking the capability *per arm* of a generic union —

```
fn say_hello<T, S>(it: T | S, ?to_str: (T) -> Str, ?to_str: (S) -> Str) [Console] -> None {
    println("Hello, ${it}")
}
```

— two implicits with **one name at two types**. It is not needed for
comptime and is recorded here as its own item, with the decisions it would
need: (i) legality of same-named implicits distinguished by type (ROADMAP §16
already anticipates "two implicits named `iter` and two named `next` at
different types" as the first thing to test, so the answer is presumably yes
and the body's `to_str(x)` is ordinary overloading by argument type); (ii) the
**override spelling** at a call, since `to_str = f` no longer says which
(`to_str: (T) -> Str = f`, or positional after the arguments); (iii) the
**interpolation lowering**: `${it}` with `it: T | S` becomes a `when` over the
union's arms choosing the implicit per arm, which is possible without an erased
type test because a union value carries its arm index at runtime
[union-arm-identity]; (iv) whether [interp-to-str]'s rule ("a `to_str` at the
interpolation site, looked up as an implicit is") extends to "one per arm of a
union subject". *Recommendation:* record in ROADMAP beside §16, decide with it.

### 13.3 What recursive implicit resolution (ROADMAP §6) needs decided

§6 today says the resolution half is contained and the emission half is where
the work is; it does not list the language calls. They are:

1. **A candidate whose own implicits cannot be filled: dropped, or an
   error?** `cmp<A, B>((A, B), (A, B), ?Ordered<A>, ?Ordered<B>)` matches
   `(Int, Foo)` by type and fails on `Foo`. Options: (a) it is not a candidate
   (C++'s SFINAE), so another overload may win silently; (b) it is an error
   naming the chain (`cmp for (Int, Foo) needs cmp for Foo: none in scope`)
   unless *another* candidate resolves fully, in which case that one wins and
   the failed one is mentioned only if the two would otherwise be ambiguous.
   *Recommendation:* (b); Salvo's "refuse to choose" and one-mistake-one-
   diagnostic posture both point at naming the chain.
2. **Depth cap and cycles.** A resolution that needs itself (`cmp<T>` for `T`
   requiring `cmp<T>`) is refused as a cycle; a cap (say 8) bounds runaway
   nesting of tuples-of-tuples. Both are diagnostics naming the chain. Small,
   but the numbers and words are the user's.
3. **How a nested identity prints in a type** [cmp-carry]. A `Set<(Int,
   Str)>` carries a `hash`; with recursion the identity is a *tree* (`hash@
   tuple(hash@Int, hash@Str)`). Options: (a) the type prints and compares the
   full tree, so two sets over `(Int, Person)` keyed by different `Person`
   hashes are different types (correct and verbose); (b) the type prints the
   root and the nested arguments are *canonical* by definition (only the
   default resolution is allowed inside a nested identity; a non-default
   nested `hash` is refused in a carried position). *Recommendation:* (a) for
   correctness, with the printer eliding arguments that are the canonical
   default so the common case reads as today.
4. **`with` pairing through nesting** [implicit-with]. `Hashed<(A, B)>` needs
   `Hashed<A>` and `Hashed<B>` as *pairs*; an `eq` for `A` from one place and
   a `hash` for `A` from another must be refused as the top level already
   refuses half a pair. *Recommendation:* the relation propagates: a nested
   group is filled as a group.
5. **Who owns list and tuple identity: std or the backends.** Today `List<T>`
   and tuple `cmp`/`eq`/`hash` are structural *in the backends*, ignoring an
   element's declared identity [col-hashed-ordered]; §6 records the "cheaper
   alternative" of declaring that intended. With recursion available, std can
   declare `cmp<T>(a: List<T>, b: List<T>, ?Ordered<T>)` and the tuple
   versions in Salvo, and then an element's declared `cmp` is honoured inside a
   list key. Options: (a) move them to std and delete the backend structural
   comparison for non-scalar elements; (b) keep the intrinsics for scalar
   element types (fast path, same result) and use the std declaration
   otherwise; (c) leave the backends as they are and accept that a `List<
   Person>` key ignores `Person`'s `cmp`. *Recommendation:* (b); (c) is the
   status quo §6 already calls a lie once declared. This decision also fixes
   what comptime's `to_json` sees for a `List<Address>` field: with std
   owning the list codec, `to_json<T>(items: List<T>, ?to_json: (T) -> Json)` is
   an ordinary Salvo function and the recursion fills it.
6. **Where recursion applies.** At every implicit-resolution site — calls,
   `?:`-picks that resolve, interpolation's `to_str` lookup, and the copies
   inside a compfn — or only at calls. *Recommendation:* everywhere resolution
   runs; one rule.

The emission half (nested `ImplicitArg::Resolved`, adapter closures passing
their own arguments on both backends) is work, not a decision, and §6 already
sizes it as the larger half.

### 13.4 Left for a next round

- 13.1's four calls: kind set as listed; `intrinsic` or `plain`; `else`
  permitted; `inline when` or `when`.
- 13.2's same-name-implicits item: record beside ROADMAP §16, not part of
  comptime.
- 13.3's six §6 decisions, of which 1, 3 and 5 are the substantive ones.

---

## 14. Round 4 (user, 2026-09-28) — the kind words, and §6's examples

**Decided.** `inline when` (the user's sketch omitted the word by mistake).
`else` in the kind `when` is allowed exactly as it is in a subjectless `when`.
§6.1 as recommended (a candidate whose implicits cannot be filled is an error
naming the chain unless another candidate resolves fully). §6.4 as
recommended (`with` pairs propagate). §6.6: recursion applies everywhere
resolution runs. The same-name-implicits item (13.2) goes to ROADMAP §16.

### 14.1 `intrinsic` clashes with `intrinsic fn`; what `param` was

The kind `when` classifies **types**, never functions. An `intrinsic fn` is a
function; the only way it meets the `when` is as a fn-typed *field* (`f: (Int)
-> Str`), whose type is of kind `fn` regardless of how any particular function
was declared. So `is intrinsic` would have been unambiguous in a type position,
but the user is right that the word makes a reader stop and ask exactly this
question.

`param` was the kind of a **type parameter of the struct being stamped**:
`Wrapper<T>`'s `T`, met by `by auto` on a generic struct (12.3), where a copy
records an implicit need instead of resolving. It is opaque to the compfn in
the same way an intrinsic type is: no fields, no arms, nothing to look inside.

That shared property is the better name. The compfn's decision for an
intrinsic type and for a type parameter is the same, **call the capability at
that type**, and 12.3 already makes the parameter case work through resolution
without the compfn saying anything. So the two kinds merge:

```
inline when field.type {
    is struct  { … }
    is union   { … }
    is tuple   { … }
    is fn      { … }
    is opaque  { … }     // an intrinsic type, or a type parameter of the struct being stamped
}
```

*Recommendation:* **`opaque`**, defined as "a type this compfn cannot look
inside". `plain` (the user's word) would do for the same merged set; `opaque`
states the property the compfn is acting on. What is lost by merging is the
ability to `refuse` a generic instantiation specifically, which 12.3 makes
unnecessary; if it is ever wanted, `is param` can be split back out as a new
arm, which is exactly the evolution the exhaustive `when` is designed to
surface.

### 14.2 §6.2 — the depth cap

Three is too low once §6.5 puts list and map identity in std, because the
depth counts *every* implicit-needing candidate on the chain and containers
are on it: `Map<Str, List<(Int, Person)>>` is `Map` → `List` → tuple → the
leaves, depth 3 before reaching `Int` and `Person`, and that is an ordinary
type for a lookup table. The cap is a safety net against runaway resolution,
not a design limit, and its cost is compile time only. *Recommendation:* **8**,
with the wording the user gave: "resolution of `cmp` for `<type>` nests more
than 8 levels deep; pass `cmp = …` explicitly." A cycle (a chain that needs
itself) is refused at once with the chain printed, regardless of depth.

### 14.3 §6.3 — what the two options differ on, by example

```
struct Person : Hashed<self> by auto { id: Str, name: Str }
fn id_hash(p: Person) -> Long => p { return hash(p.id) }
fn same_id(a: Person, b: Person) -> Bool => a, b { return a.id == b.id }

let all: Mut Set<(Int, Person)> = mut_set_of()
```

With recursion, `mut_set_of` resolves `hash` for `(Int, Person)` to std's
tuple hash **with `hash@Int` and `hash@Person` filled underneath it**. The
identity the set carries is a tree, and the question was whether the type
prints and compares the tree or only its root.

Now suppose a set keyed by *id* rather than by `Person`'s own identity:

- Under **(a)**, the type would carry `Set<(Int, Person)>(hash@tuple(hash@Int,
  id_hash), eq@tuple(eq@Int, same_id))`, so it needs a way to *write* a nested
  override at the call: something like `mut_set_of(hash = hash@tuple(hash@Int,
  id_hash), eq = …)`, a partial application of a function's implicits as a
  value. Nothing like it exists today; it is a new spelling, and the type
  printer would have to render the tree.
- Under **(b)**, a nested identity is **canonical by construction**: the only
  thing that can fill a nested implicit is default resolution, so the type
  prints its root only (`Set<(Int, Person)>`, eliding the canonical defaults
  as it does now). To key by id you write the tuple identity as a function,
  which is then a *root* and prints as one:

  ```
  fn id_pair_hash(t: (Int, Person)) -> Long => t { return hash(t.0) * 31L + id_hash(t.1) }
  fn id_pair_eq(a: (Int, Person), b: (Int, Person)) -> Bool => a, b { return a.0 == b.0 && same_id(a.1, b.1) }
  let by_id: Mut Set<(Int, Person)>(id_pair_hash, id_pair_eq) = mut_set_of(hash = id_pair_hash, eq = id_pair_eq)
  ```

So the difference is not correctness, it is whether a **nested override
spelling** exists. Working through the example reverses the earlier
recommendation: **(b)**. Nothing in the language can currently produce a
non-canonical nested identity, so the tree is always the default tree and the
root names it; the hand-written function is the escape and needs no new
syntax. Reopen (a) if a partial-application spelling for implicits is ever
wanted for its own sake.

### 14.4 §6.5 — what "who owns list identity" decides

Yes: the question is **where the default lies**, and the significance is that
the default is what runs when nobody overrides, which is `xs == ys`, a
`Set<List<Person>>`, and a `sort` of a `List<List<Int>>`.

```
struct Person : Hashed<self> {
    id: Str,
    name: Str

    fn hash(p: Person) -> Long by auto
    fn eq(a: Person, b: Person) -> Bool { return a.id == b.id }     // same id, same person
}

let xs = list_of(Person { id: "1", name: "Ann" })
let ys = list_of(Person { id: "1", name: "Anne" })

xs == ys
```

- **Backend-owned (today):** `eq(List<T>, List<T>)` is an intrinsic lowered to
  the host's list equality, which compares elements with the host's
  equality for the generated `Person` (Kotlin data-class `equals`, Rust derived
  `PartialEq`), **structural over every field**. `xs == ys` is `false`, and
  `Person`'s declared `eq` was never consulted. A `Set<List<Person>>` buckets
  the same way. The declared capability is honoured for a `Person` and
  silently ignored for a `List<Person>`, which is the lie §6 says declaring
  the intrinsics would make look intended.
- **std-owned:** `export fn eq<T>(a: List<T>, b: List<T>, ?Eq<T>) -> Bool` in
  `core.list`, written in Salvo, resolving the element's `eq` through recursion.
  `xs == ys` is `true`. The user can still override at any call (`eq = …`), as
  now; what changes is that the *unspoken* default agrees with the declaration.

The hybrid recommended earlier keeps the intrinsic as a fast path only where
the two agree by construction: when `T` is a scalar or `Str`, whose identity is
the host's anyway. For every other element type the std function runs. It
also fixes what comptime's `to_json` meets on a `List<Address>` field: with
std owning the list codec (`to_json<T>(items: List<T>, ?to_json: (T) -> Json)`),
the recursion fills it and nothing is special-cased.

### 14.5 Left for a next round

- Confirm `opaque` (merging intrinsic types and type parameters) or `plain`.
- Confirm the cap of 8 and the wording.
- Confirm §6.3 (b) after the example, and §6.5's hybrid.

---

## 15. Round 5 (user, 2026-09-28) — kind words, intrinsic obligations, `by` at a call

**Decided.** Kind words: **`is opaque`** matches either of two finer kinds,
**`is basic`** (a type declared `intrinsic type`) and **`is generic`** (a type
parameter of the struct being stamped), so a compfn writes `is opaque` when
it does not care and the pair when it does, the way `is Person` matches both
arms of `Surname Person | Person` (Qualifiers.md). The exhaustive set is
therefore `struct`, `union`, `tuple`, `fn`, and `opaque`-or-both-of-its-halves.
§6.2: cap **8**, wording as given. §6.3: **(b)**, nested identities canonical
by construction, root printed.

### 15.1 §6.5 — obligations on `intrinsic type`

The user's shape:

```
intrinsic type List<T> canbe Mut : Hashed<self>, Ordered<self>, ToStr<self>
```

with the fulfilments either `by auto` or declared, and the rule that an
obligation's fulfilment is the type's **canonical** capability, the one a
`Set<List<Person>>(?hash, ?eq)` slot resolves to by default. **Does it work?**
Yes, with three prerequisites, two of which are already owed:

1. **Obligation clauses on `intrinsic type` (and, per R-1, on `type`)
   declarations.** [group-obligation] checks the clause at a struct today; the
   check is the same for any type declaration, and the attached fulfilment
   travels with the type [fn-attached] exactly as a struct's does. New syntax
   position, existing rule.
2. **The trailing-implicit relaxation of the obligation match** (12.3 item 3,
   ROADMAP §16). The fulfilment is `fn eq<T>(a: List<T>, b: List<T>, ?Eq<T>)
   -> Bool`; the member is `eq(a: List<T>, b: List<T>) -> Bool`. The match must
   ignore the trailing implicit. Third customer for the same relaxation.
3. **Recursive resolution (§6).** `?Eq<List<Person>>` at `mut_set_of` resolves
   the attached `eq@List`, which itself needs `?Eq<Person>`, filled with
   `eq@Person`. That is the recursion, and with §6.3(b) the slot prints as
   `Set<List<Person>>`, the root.

Two choices inside it:

- **Salvo or intrinsic bodies.** `eq`, `cmp`, `hash` and `to_str` for a
  `List<T>` are all writable in Salvo in `core.list` with a `for`, taking
  `?Eq<T>` etc.; nothing needs the host. The same holds for `Map` and `Set`
  (over their entries) and tuples (`by auto`, below). The scalars keep their
  `intrinsic fn`s, and now **declare them**: `intrinsic type Int : Ordered<self>,
  Hashed<self>, ToStr<self>` is checked against `intrinsic fn cmp(a: Int, b:
  Int) -> Int` as any obligation is. *Recommendation:* Salvo bodies for the
  containers, intrinsic for the scalars, so every type in std states its
  capabilities in one clause and the backends' host equality on lists is
  simply never reached from Salvo. The scalar fast path (host `==` when the
  resolved element identity is the scalar's own) becomes an emitter
  optimization if a benchmark asks for it, not a semantic rule.
- **`ToStr<self>` on `List<T>`** means `to_str<T>(list: List<T>, ?to_str: (T)
  -> Str)`: `"${people}"` now renders each `Person` with *its* `to_str`, where
  today [col-to-str] renders the host's view. A `List<Pool>` interpolation
  fails at the use ("`Pool` needs a `to_str`"), as a bare `Pool` does today;
  the obligation on `List` is not a claim about its elements.

Net effect on §6: the "cheaper alternative" (declare the structural backend
behaviour intended) is withdrawn; [col-hashed-ordered] and [col-to-str] are
rewritten to say the containers *delegate to their elements' declared
capabilities*; the backend intrinsics for container identity are deleted.

### 15.2 `by` at a call: supplying an implicit by stamping

The user's example, with `compfn eq<tuple T>(a: T, b: T) -> Bool` in
`core.auto`:

```
print_if_equal((1, 2), (4, 5), eq by auto)
```

This is a **third `by` site** beside the clause and the fn declaration: an
**implicit override** (the position `eq = f` fills today) may instead say `eq
by X`, which stamps `X`'s `eq` at the type the call binds for that implicit
and passes the instantiation. It fits §10.2 unchanged: every instantiation is
at a `by` site, and this is one. It is also **the only way a tuple gets an
identity**, since a tuple has no declaration to carry a clause and nothing
auto-applies:

```
let pairs: Mut Set<(Int, Str)> = mut_set_of(hash by auto, eq by auto)   // both, since eq with hash
```

Three details:

- **Where the instantiation is emitted.** A tuple has no declaration to sit
  beside, so a call-site stamping is emitted in the *calling module*, once per
  (compfn, type) it uses; two modules stamping `eq<(Int, Str)>` each get one.
  Same output, no sharing needed.
- **Pairing.** `eq with hash` [implicit-with] applies: `hash by auto` alone
  beside a resolved `eq` is the half-a-pair error, as `hash = f` is.
- **The type it prints as.** The identity in the slot is the stamped root,
  `Set<(Int, Str)>(hash<(Int, Str)>@auto, eq<(Int, Str)>@auto)`, elided in
  the printer as a canonical default is not, since it was written. (Under
  6.3(b) its nested `hash@Int`, `hash@Str` are canonical and unprinted.)

**Punted, and to record in ROADMAP:** *implicit compfn resolution*, where an
implicit that finds no candidate for a tuple would stamp `core.auto`'s compfn
without being asked. It would make `mut_set_of()` over a tuple work bare,
which is what §6's opening example wanted. Deferred because it is the one
place a compfn would apply without a `by`, which round 1 decided against for
everything else; the diagnostic for a missing tuple identity should meanwhile
**name `hash by auto, eq by auto` as the remedy**, so the cost of punting is
one line at the site.

### 15.3 Left for a next round

- Confirm the reading of the kind words (`opaque` matches `basic` and
  `generic`; the exhaustive set is five with `opaque` splittable).
- Confirm 15.1's two choices (Salvo bodies for containers, intrinsic for
  scalars; `ToStr<self>` on `List` delegating to elements).
- Confirm 15.2's three details and the ROADMAP item.

---

## 16. Round 6 (user, 2026-09-28) — supplying an implicit deep in a chain

**Confirmed.** The kind words as read in §15; the three prerequisites of 15.1.

### 16.1 The case

`Person` has no `to_str`. `to_str(set_of(list_of(person)))` resolves the outer
`to_str` at `Set<List<Person>>`, whose `?to_str: (List<Person>) -> Str` resolves
to `to_str@List`, whose `?to_str: (Person) -> Str` finds nothing. The user wants
to hand in the leaf at the outer call. This is exactly the case §6.3(b) does
not cover: (b) says a nested identity is canonical because nothing can be
written that makes it otherwise, and the hand-written escape here is a wrapper
per nesting level (`list_str` calling `to_str(l, to_str = person_str)`, then
`to_str(s, to_str = list_str)`) — re-implementing nothing, but writing a
function whose only content is a forwarded override, twice.

### 16.2 Options

**(a) Declare it in scope.** `fn to_str(p: Person) -> Str` in the calling file.
Recursion resolves each level *in the calling scope*, so the leaf is found.
Zero mechanism, and it is what a file that prints people should do anyway.
But it is a default for the file, not a choice at one call, and it does not
help when `Person` already has an attached `to_str` and this one call wants
another.

**(b) A type-keyed override, pushed down the chain.** The override names the
implicit and **the type it is for**, and resolution carries it down, applying
it at the level whose implicit has that name and that type:

```
to_str(set_of(list_of(person)), to_str: (Person) -> Str = short_name)
to_str(set_of(list_of(person)), to_str: (Person) -> Str by auto)       // or stamp it (15.2)
```

The outer implicit is `to_str: (List<Person>) -> Str`; the written override
does not fit it, so it is not an error but a *pending* override, consulted
before default resolution at every nested level and consumed where it fits.
An override no level consumes is an error ("nothing in this call's resolution
needs a `to_str` at `Person`"). The spelling is the type-ascribed override
13.2 already needs for same-named implicits at different types, so it is one
form serving two cases. Matching is by (name, type), the same pair
[implicit-forward] already matches on when a generic body passes implicits on.

**(c) Partial application of the intermediate.** `to_str(s, to_str =
to_str@List(to_str = short_name))`: name the level and fill its implicit as a
value. Explicit about *where* the leaf goes, at the cost of naming every level
between the call and the leaf, and of a new value form (a function with some
implicits bound). Precise, verbose, and the verbosity grows with depth, which
is the wrapper-per-level cost of (b)'s absence in another spelling.

**Shape-changing alternative — the leaf is the only override there is.** Drop
positional `to_str = f` for the direct implicit and make *every* override
type-keyed: `to_str: (List<Person>) -> Str = f` for the top level too. One
rule, but it makes the common case (override the one implicit a call has)
carry a type it did not need to state; refused for that.

*Recommendation:* **(b)**, with (a) documented as the ordinary way to give a
file its rendering, and `by` accepted on the right side as in 15.2.

### 16.3 What (b) does to §6.3

A pushed-down override can land in a **carried** identity: `mut_set_of(hash:
(Person) -> Long = id_hash, eq: (Person, Person) -> Bool = same_id)` for a
`Set<List<Person>>`. Then the identity in the slot is `hash@List(id_hash)`, not
canonical, and (b) of §6.3 ("nothing can produce a non-canonical nested
identity") no longer holds. So §6.3 moves to **(a) with elision**, as first
proposed: the slot carries the tree; two sets differ in type when their trees
differ; the printer shows a level only where it is not the canonical default,
so `Set<List<Person>>` prints bare in the ordinary case and as
`Set<List<Person>>(hash@List(id_hash), eq@List(same_id))` when the user wrote
the leaf. The emission side is unchanged from §6's own estimate (nested
`ImplicitArg::Resolved` was always the work); what is added is a tree-valued
identity in `Ty` for the checker to compare and print. Pairing: `eq with hash`
applies at the leaf as it does at the root, so `hash: (Person) -> Long =
id_hash` alone is the half-a-pair error.

### 16.4 Left for a next round

- Confirm (b) and the `by` right-hand side; confirm the unused-override
  error.
- Confirm §6.3 moving to (a)-with-elision, and the tree-valued identity in
  the type.

---

## 17. Round 7 (user, 2026-09-28) — (b) adopted, and what it closes

**Decided.** 16.2 **(b)**: a type-keyed override (`name: <fn type> = f`, or
`name: <fn type> by X`) is consulted at every level of a resolution chain and
consumed where its (name, type) fits; one no level consumes is an error.
Consequently §6.3 is **(a) with elision** (16.3): the carried identity is a
tree, compared as one and printed only where a level is not the canonical
default.

**What it closes besides 16.1.** The user's observation: (b) is also the
override spelling for **two implicits of one name at different types** (13.2
(ii), `say_hello<T, S>(it: T | S, ?to_str: (T) -> Str, ?to_str: (S) -> Str)`):
`say_hello(x, to_str: (Person) -> Str = short_name)` names which. And the rule
for the remaining case falls out of one the language already has: two
implicits of the **same name and the same type** are **one binding** and are
overridden together, which is [cmp-binder]'s "all `?name` occurrences in one
signature denote one binding" applied to a written override. So 13.2's four
decisions reduce to two (legality, presumed yes, and the interpolation
lowering over arms), and the item that goes to ROADMAP §16 is those two plus
the note that the override spelling is already decided here.

### 17.1 Where the document stands

Decided across rounds 1–7: the surface (`compfn`, `<struct T>`/`<union T>`,
`T.fields`/`T.arms`/`T.name`, lower-case projections, `v.[field]`, `[field]:`,
`inline for`/`inline if`/`inline when`, `refuse`, the five kinds with `opaque`
splittable into `basic`/`generic`, `field.name == "…"`, `T.arms` including
`None`); instantiation (concrete only, at three `by` sites: clause, fn
declaration, implicit override; a parameterless `compfn` is its own
instantiation; one specialization per (compfn, type), beside the type or in the
calling module); `by X` with `X` a module or a compfn; `core.auto` and `by auto`,
nothing auto-applying, `to_str` opt-in and [interp-struct] deleted; implicits
acquired by stamped fns from the start; obligation clauses on `type` and
`intrinsic type` with std owning container identity in Salvo; the six §6
decisions with the cap at 8 and the tree-valued identity; the JSON pair in a
`json` module after SERDE SD-1; the shipped set `eq`, `hash`, `cmp`, `to_str`,
`to_json`, `from_json`.

Still open, all small: 15.1's two choices (Salvo bodies for containers and
intrinsic for scalars; `ToStr<self>` on `List` delegating), which the round
agreed with in principle and did not object to; 15.2's three details and its
ROADMAP item (implicit compfn resolution for tuples); the wording of the
unused-override error. SERDE SD-1 (the `Json` type) is the one prerequisite
outside this document.

**Propagation owed** (unchanged in kind, grown in content): COMPLETED.md's
decision log gets the seven rounds as one entry; ROADMAP.md gets a comptime
step with the build order of CT-6 (core.auto before §6, `json` over containers
after), §6 rewritten with its six decisions, §16 gaining the same-name
implicits item and the trailing-implicit relaxation's third customer, and a
"Recorded" entry for implicit compfn resolution; LANGUAGE_SPEC.md gains the
`[comptime-*]`, `[obligation-by]`, `[fn-by]`, and an `[implicit-override-typed]`
rule, deletes [cmp-auto], rewrites [interp-struct], [col-hashed-ordered],
[col-to-str], [implicit-resolve] (recursion), [cmp-carry] (tree identity);
SERDE.md's SD-3 is re-read against this document. Then this file is deleted.
