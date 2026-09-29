# Compile-time functions

A struct's structural `cmp` is the same eight lines for every struct: compare
the first field, then the next, then answer zero. Salvo lets the standard
library write those lines *once*, over the fields of any struct, as a
**compile-time function** — and lets a type ask for a copy with one word.

```
struct Point : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
    x: Int,
    y: Int
}
```

This page is the other side of that line: what `by` does, what a `comptime fn` is,
and how to write one of your own.

## `by`: stamping

`by X` on an obligation clause says where the group's members come from. `X`
is a module, and for each member the module's `comptime fn` of that name is
**instantiated at this type** — stamped — and the result is an ordinary
function declared on the type, exactly as if you had written it inside the
struct's body: it travels with the type, `cmp@Point` names it, an implicit
`?cmp` resolves to it, and it carries the struct's own `export`.

`auto` is `core.auto`, which holds `cmp`, `eq`, `hash` and `to_str` for
structs and for unions. Being in `core` means it can be named without an
import, and nothing more: **nothing is stamped unless you ask**. A struct with
no clause has no `cmp` and cannot be compared, and one with no `to_str` does
not interpolate.

The clause works on a **named union** too, written after the alias:

```
struct Manual { note: Str }
struct Imported { feed: Str, n: Int }
type Source = Manual | Imported : ToStr<self> by auto
```

A free union — `Str | Int`, or any `T?` — has no declaration to carry a
clause, so it has no functions of its own; a comptime fn over a struct that holds
one reaches its arms through the field (below).

`by` also works on one function, keeping its full signature, which is how a
type mixes stamped and hand-written members:

```
struct Person : Hashed<self> {
    name: Str,
    age: Int

    fn hash(value: Person) -> Long by auto
    fn eq(a: Person, b: Person) -> Bool => a, b { return a.age == b.age }
}
```

The type stamped at is the first parameter's, and the written signature has to
be the instantiation's; a body beside `by` is an error, since the body is what
`by` supplies.

### Where the errors land

A stamped body resolves a `cmp` per field, so a field with no `cmp` — a
`Double`, a function value, a struct that never opted in — fails in the copy
for that field, and the error is reported **at that field**, prefixed with
what was being stamped:

```
error: in `cmp` from `auto` for `Reading.value: Double`: no matching overload
       for `cmp(Double, Double)`
  23 |     value: Double
           ^^^^^^^^^^^^^
```

A stamping can also be refused outright: `core.auto`'s `hash` refuses a
`canbe Mut` struct, because a key that can change while a collection holds it
corrupts the collection, and that refusal lands at the clause.

## `comptime fn`: what gets stamped

A `comptime fn` is a function over a **kind** of type rather than a type. Its one
type parameter is bound with `is` to an arm of `Type` — `<T is Struct>` or
`<T is Union>`, the same test a `when` arm makes — and inside its body the
compile-time syntax is legal. It is not callable: it exists to be instantiated
by `by`, and by the time the program is checked every use of one has been
stamped and the comptime fn itself is gone.

Here is `core.auto`'s `cmp` for structs, in full:

```
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    [for field in T.fields] {
        let c = cmp(a.[field], b.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}
```

Three things in it are compile-time:

- **`T.fields`** is the struct's fields, in declaration order, and `[for …]`
  unrolls the loop: one copy of the body per field, each checked with that
  field's type concrete. So `cmp(a.[field], b.[field])` in the copy for `x` is
  an ordinary call `cmp(a.x, b.x)`, resolved like any other, and a field with
  no `cmp` fails there.
- **`a.[field]`** reads the field the enclosing `[for …]` is at. Inside a
  struct literal, `[field]: value` is the same entry.
- The binder exposes **`field.name`** (a string literal), **`field.type`**
  (usable wherever a type is written), **`field.index`**, **`field.first`** and
  **`field.last`**; the bound parameter exposes **`T.name`**. `to_str` uses
  them:

```
export comptime fn to_str<T is Struct>(value: T) [] -> Str => value {
    let out: Mut Str = mut_str("${T.name} {")
    [for field in T.fields] {
        [if field.first] {
            append(out, " ")
        } else {
            append(out, ", ")
        }
        append(out, "${field.name}: ${value.[field]}")
    }
    append(out, " }")
    return out
}
```

`[if …]` keeps or drops a branch per instantiation, and the dropped branch is
never checked. Its conditions are compile-time facts: `field.first`,
`field.last`, `field.name == "value"`, `x.index == y.index` (and the other
orderings), `T.mutable`, `field.type is Str` (a type, up to aliases), and
`field.type is Struct` (a kind — an arm of `Type`, below).

A comptime fn can **refuse** an instantiation, with a message in the caller's terms:

```
export comptime fn hash<T is Struct>(value: T) [] -> Long => value {
    [if T.mutable] {
        refuse!("a `Mut`-capable struct can change while a collection holds it, so it cannot be a key")
    }
    let h = 17L
    [for field in T.fields] {
        h = mix_hash(h, hash(value.[field]))
    }
    return h
}
```

### Unions

A comptime fn over a union walks its arms. `[when value] { [arm] { … } }`
dispatches on a union value: the one written arm is stamped once per declared
arm, with `value` narrowed to that arm inside, producing an ordinary exhaustive
`when`. `None` is an arm like any other, with `arm.type` `None`:

```
export comptime fn to_str<T is Union>(value: T) [] -> Str => value {
    [when value] {
        [arm] {
            [if arm.type is None] {
                return "None"
            } else {
                return to_str(value)
            }
        }
    }
    return ""
}
```

`T.arms` walks them as a sequence, and `arm.index` is the arm's declared
position, which is how the union `cmp` orders across arms. A union field of a
struct is reached the same way, through `value.[field]`.

### The model: `core.comptime`

Everything a comptime fn may know about a type is declared, as compile-time
structs in `core.comptime`:

```
comptime struct Struct  { name: Str, fields: List<Field>, mutable: Bool }
comptime struct Union   { name: Str, arms: List<Arm> }
comptime struct Tuple   { elems: List<Type> }
comptime struct FnType  {}
comptime struct Basic   { name: Str }      // declared `intrinsic type`
comptime struct Generic { name: Str }      // a type parameter of the struct being stamped
comptime type Opaque = Basic | Generic
comptime type Type   = Struct | Union | Tuple | FnType | Opaque

comptime struct Field { name: Str, type: Type, index: Int, first: Bool, last: Bool }
comptime struct Arm   { name: Str, type: Type, index: Int, first: Bool, last: Bool }
```

The bound `T` is a `Struct` or a `Union`; a `[for …]` binder is a `Field` or
an `Arm`; `field.type` is a `Type`. These exist at compile time only — the
compiler fills them, nothing constructs one, and naming one in an ordinary
type position is an error — but they are ordinary declarations to read, and
the language server shows them: hovering `T`, `field` or `field.index` inside
a comptime fn body answers `T is Struct`, `field: Field`, `field.index: Int`.

Because `Type` is a union, a comptime fn that treats field types differently
dispatches on it with an ordinary `when`, exhaustive over the arms unless an
`else` closes it:

```
[when field.type] {
    is Struct { … }
    is Union  { … }
    is Tuple  { … }
    is FnType { … }
    is Opaque { … }
}
```

`Opaque` is a type the comptime fn cannot look inside — an `Int`, or a type
parameter — and `is Opaque` matches either of its arms, `Basic` and `Generic`,
when the body does not care which. The exhaustiveness is deliberate: when the
language gains a kind, every comptime fn that did not consider it stops
compiling rather than silently skipping it.

### A concrete comptime fn

A `comptime fn` with no type parameter is its own single instantiation, declared
where a function is. It is how one field gets handled by hand while the rest
stay structural — a `Double` that needs a total order, say:

```
struct Reading : Ordered<self> {
    sensor: Str,
    unit: Str,
    value: Double

    comptime fn cmp(a: Reading, b: Reading) -> Int => a, b {
        [for field in Reading.fields] {
            [if field.name == "value"] {
                let c = total_cmp(a.value, b.value)
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

A field name no field has is an error at the declaration.

### Your own module

`core.auto` is not special. A module of your own holding `comptime fn cmp<struct
T>(…)`, `eq` and `hash` — comparing an `id` field only, say — is reached by
`by ids`, for every entity in a program. A comptime fn is like an `intrinsic fn` in
what it writes down (its effect list, deductions and return type), and one that
a `params` member is fulfilled by must be effect-free, since an implicit
parameter may resolve to it.

## What is not there yet

Two things this page's machinery is built for are recorded rather than shipped:
a `by auto` on a **generic struct** (`Wrapper<T>`), whose copy for a field of
type `T` would need an implicit (`?Ordered<T>`) the stamped function acquires —
refused today with the hand-written form named — and `eq by auto` at a **call**,
supplying an implicit by stamping, which is how a tuple would get an identity.
Both are in ROADMAP.md.
