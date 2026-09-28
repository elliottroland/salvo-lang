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

This page is the other side of that line: what `by` does, what a `compfn` is,
and how to write one of your own.

## `by`: stamping

`by X` on an obligation clause says where the group's members come from. `X`
is a module, and for each member the module's `compfn` of that name is
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
clause, so it has no functions of its own; a compfn over a struct that holds
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

## `compfn`: what gets stamped

A `compfn` is a function over a **kind** of type rather than a type. Its one
type parameter carries the kind as a bound, `<struct T>` or `<union T>`, and
inside its body the compile-time syntax is legal. It is not callable: it exists
to be instantiated by `by`, and by the time the program is checked every use
of one has been stamped and the compfn itself is gone.

Here is `core.auto`'s `cmp` for structs, in full:

```
export compfn cmp<struct T>(a: T, b: T) [] -> Int => a, b {
    inline for field in T.fields {
        let c = cmp(a.[field], b.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}
```

Three things in it are compile-time:

- **`T.fields`** is the struct's fields, in declaration order, and `inline for`
  unrolls the loop: one copy of the body per field, each checked with that
  field's type concrete. So `cmp(a.[field], b.[field])` in the copy for `x` is
  an ordinary call `cmp(a.x, b.x)`, resolved like any other, and a field with
  no `cmp` fails there.
- **`a.[field]`** reads the field the enclosing `inline for` is at. Inside a
  struct literal, `[field]: value` is the same entry.
- The binder exposes **`field.name`** (a string literal), **`field.type`**
  (usable wherever a type is written), **`field.index`**, **`field.first`** and
  **`field.last`**; the bound parameter exposes **`T.name`**. `to_str` uses
  them:

```
export compfn to_str<struct T>(value: T) [] -> Str => value {
    let out: Mut Str = mut_str("${T.name} {")
    inline for field in T.fields {
        inline if field.first {
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

`inline if` keeps or drops a branch per instantiation, and the dropped branch is
never checked. Its conditions are compile-time facts: `field.first`,
`field.last`, `field.name == "value"`, `x.index == y.index` (and the other
orderings), `T canbe Mut`, `field.type is Str` (a type, up to aliases), and
`field.type is struct` (a kind).

A compfn can **refuse** an instantiation, with a message in the caller's terms:

```
export compfn hash<struct T>(value: T) [] -> Long => value {
    inline if T canbe Mut {
        refuse "a `Mut`-capable struct can change while a collection holds it, so it cannot be a key"
    }
    let h = 17L
    inline for field in T.fields {
        h = mix_hash(h, hash(value.[field]))
    }
    return h
}
```

### Unions

A compfn over a union walks its arms. `inline when value { [arm] { … } }`
dispatches on a union value: the one written arm is stamped once per declared
arm, with `value` narrowed to that arm inside, producing an ordinary exhaustive
`when`. `None` is an arm like any other, with `arm.type` `None`:

```
export compfn to_str<union T>(value: T) [] -> Str => value {
    inline when value {
        [arm] {
            inline if arm.type is None {
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

### Kinds

A compfn that treats field types differently dispatches on their **kind** with
`inline when`, which is exhaustive over the kinds unless an `else` closes it:

```
inline when field.type {
    is struct { … }
    is union  { … }
    is tuple  { … }
    is fn     { … }
    is opaque { … }
}
```

`opaque` is a type the compfn cannot look inside — an intrinsic type such as
`Int`, or a type parameter — and it matches either of its two halves, `basic`
and `generic`, when a compfn does not care which. The exhaustiveness is
deliberate: when the language gains a kind, every compfn that did not consider
it stops compiling rather than silently skipping it.

### A concrete compfn

A `compfn` with no type parameter is its own single instantiation, declared
where a function is. It is how one field gets handled by hand while the rest
stay structural — a `Double` that needs a total order, say:

```
struct Reading : Ordered<self> {
    sensor: Str,
    unit: Str,
    value: Double

    compfn cmp(a: Reading, b: Reading) -> Int => a, b {
        inline for field in Reading.fields {
            inline if field.name == "value" {
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

`core.auto` is not special. A module of your own holding `compfn cmp<struct
T>(…)`, `eq` and `hash` — comparing an `id` field only, say — is reached by
`by ids`, for every entity in a program. A compfn is like an `intrinsic fn` in
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
