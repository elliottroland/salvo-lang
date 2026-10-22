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
how to write one of your own, and how to call one from ordinary code.

## `by`: stamping

`by X` on an obligation clause says where the group's members come from. `X`
is a module, and for each member the module's `comptime fn` of that name is
**instantiated at this type** — stamped — and the result is an ordinary
function declared on the type, exactly as if you had written it inside the
struct's body: it travels with the type, `cmp@Point` names it, an implicit
`?cmp` resolves to it, and it carries the struct's own `export`.

`auto` is `core.auto`, which holds `cmp`, `eq`, `hash` and `to_str` for
structs and for unions — named by its suffix, as any module may be
(`size@list` is `size@core.list`). Being in `core` means it can be named
without an import, and nothing more: **nothing is stamped unless you ask**. `X` may also
be a comptime fn named directly (`fn cmp(a: P, b: P) -> Int by by_age`); a name
that is both a module and a comptime fn is refused as ambiguous, and `by @tag`
(the module: a module stands on the right of an `@`, as in `size@list`) or
`by tag@mymod` (the function declared in `mymod`) says which. Hovering the `by`
shows what it stamps, from where. A struct with
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

A `comptime fn` with a bound type parameter is a function over a **kind** of
type rather than a type. Its one type parameter is bound with `is` to an arm
of `Kind` — `<T is Struct>` or `<T is Union>`, the same test a `when` arm
makes — and inside its body the compile-time syntax is legal. A bounded
comptime fn is **stamped**, and is not callable: it exists to be instantiated
by `by`, and by the time the program is checked every use of one has been
stamped and the comptime fn itself is gone. (A comptime fn *without* a bound
is expanded in place and is callable; see below.)

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
  (the type itself), **`field.kind`** (what kind of type it is),
  **`field.index`**, **`field.first`**, **`field.last`** and
  **`field.default`** (below); the bound parameter exposes **`T.name`**.
  `to_str` uses them:

```
export comptime fn to_str<T is Struct>(value: T) [] -> Str => value {
    let out: Mut Str = Mut Str("${T.name} {")
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
`field.last`, `field.name == "value"` or `field.name is "a" | "b"`,
`x.index == y.index` (and the other orderings), `T.mutable`, `field.type is
Str` (a type, up to aliases), and `field.kind is Struct` (a kind — an arm of
`Kind`, below). Each `is` has a `!is`, and conditions combine with `&&`, `||`,
`!` and parentheses, as ordinary conditions do:

```
[if field.kind is Struct && !field.last] { … }
[if field.name !is "id" | "created_at"] { … }
```

`[for …]` and `[if …]` also take a terminal `...` body, as `for` and `if` do:
the rest of the enclosing block becomes the body, so a filter inside a walk is
one line:

```
[for field in T.fields] {
    [if field.name !is "id" | "created_at"] ...
    let c = cmp(a.[field], b.[field])
    if c != 0 { return c }
}
```

`[if cond] ... else { … }` keeps the braced `else` when the condition fails, as
`if cond ... else { … }` does. A `[when …]` takes no `...`: it has several
arms, and the rest of the block is one body.

Each of these bodies is a scope, as the ordinary body it becomes is: what a
`[for …]` copy, an `[if …]` branch or a `[when …]` arm declares is gone after
it, and it may not reuse a name that is visible there.

**Brackets mark a splice.** A bracket is where expansion puts a comptime value
into the code's structure, as something that is not a value: `a.[field]` and
`[field]: …` turn a `Field` into a field name, and `[field.type]` turns it
into a type, wherever a type is written:

```
[for field in T.fields] {
    let value: [field.type] = decode<[field.type]>(text)
    let seen: List<[field.type]>? = None
}
```

and as a struct literal's head, `[arm.type] { … }`. A bare `field.type` in a
type position is an error naming the bracketed form. Declared names need no
bracket, because they already are types: `T`, and a pattern binder such as
`<E>` below, read the same in the body as in any generic fn. Values need none
either: `field.name` and `"${T.name}"` are constants an expression already
accepts. The heads `[for …]`, `[if …]` and `[when …]` are bracketed for a
related reason, that the same text without them is a runtime loop or test;
inside a head, `field.type is Str` and `[when field.type]` stay bare.

**`field.default` is the field's declared default**, `= 8080` in `port: Int =
8080`, and `None` for a field that declares none. It is evaluated where the
struct is declared, so a default that calls a private function of its module
works from a comptime fn anywhere. `[if field.default is None]` tells the two
cases apart:

```
[for field in T.fields] {
    [if field.default is None] {
        append(out, "${field.name} (required)\n")
    } else {
        let d: [field.type]? = field.default
        …
    }
}
```

A field declared `= None` has a default, which is `None`.

A type and its kind are kept apart on purpose. `field.type is Union` compares
the field's type with a type called `Union`; `field.kind is Union` asks
whether the field's type is a union. A program is free to declare a type
called `Union`.

`field.name` is not just any string: it is one of the struct's field names, a
literal of the union `"x" | "y"` for a `Point`. Testing it against a name the
struct does not have is an error, and `[when field.name]` dispatches on it the
way an ordinary `when` dispatches on a literal union — **exhaustively**:

```
[when field.name] {
    is "sub_commands" | "groups" { }
    is "bools" | "strs" | "ints" {
        for arg in args.[field] { … }
    }
}
```

Like any `when` with a subject it takes no `else`: every field has to be
named by some arm. Add a field to the
struct and this stops compiling until you decide what the code does with it,
which is the point: the function stays exhaustive over the struct's fields.

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
comptime type Kind   = Struct | Union | Tuple | FnType | Opaque
comptime struct Type  {}      // a type itself; what kind it is is `.kind`

comptime struct Field { name: Str, kind: Kind, type: Type, index: Int, first: Bool, last: Bool }
comptime struct Arm   { name: Str, kind: Kind, type: Type, index: Int, first: Bool, last: Bool }
```

The bound `T` is a `Struct` or a `Union`; a `[for …]` binder is a `Field` or
an `Arm`; `field.kind` is a `Kind`, and `field.type` the type itself. These exist at compile time only — the
compiler fills them, nothing constructs one, and naming one in an ordinary
type position is an error — but they are ordinary declarations to read, and
the language server shows them: hovering `T`, `field` or `field.index` inside
a comptime fn body answers `T is Struct`, `field: Field`, `field.index: Int`.

Because `Kind` is a union, a comptime fn that treats kinds of field
differently dispatches on it with an ordinary `when`, exhaustive over the arms
and, like any `when` with a subject, with no `else`:

```
[when field.kind] {
    is Struct { … }
    is Union  { … }
    is Tuple  { … }
    is FnType { … }
    is Opaque { … }
}
```

`Opaque` is a type the comptime fn cannot look inside — an `Int`, a type
parameter, or an `opaque struct` from another module — and `is Opaque` matches either of its arms, `Basic` and `Generic`,
when the body does not care which. The exhaustiveness is deliberate: when the
language gains a kind, every comptime fn that did not consider it stops
compiling rather than silently skipping it.

### Type patterns

The type after `field.type is` is a **pattern**. `*` stands for any type, at
any depth, so one test can pick out every field of a shape:

```
[if field.type is List<*>] { … }          // any list
[if field.type is List<Arg<*>>] { … }     // a list of any `Arg`
[if field.type is *?] { … }               // any optional
```

When the body needs the part a `*` stood for, name it: declare a **binder**
before the pattern, the way a function declares its type parameters, and use
it as a type in the branch:

```
[if field.type is <E> List<E>] {
    let first: E? = None
    [if E is Int] { … }
}
```

A binder is the part as written, qualifiers and all, and is in scope only in
the branch the test guards — under `||` or `!` the test proves nothing about
the branch, so a binder there is an error. Writing the same binder twice
constrains: `<K> Pair<K, K>` matches only a pair of one type. Matching ignores
qualifiers (a `List<Int> canbe Mut` field is a `List<*>`), and a union matches
arm by arm, in order.

`[when field.type]` dispatches by pattern, trying the arms in order:

```
[when field.type] {
    is <E> List<E> { … }
    is <K, V> Map<K, V> { … }
    is Str { … }
    is * { … }
}
```

Like any `when` with a subject, it must cover what its subject can be, and it
takes no `else`: `is *` is the arm for every type the ones above it did not
match. No list of patterns covers *every* type, but a walk over a struct's
fields (or a union's arms) meets only the types that struct has, so each one
either finds its arm or is reported by name:

```
// error: no arm of this `[when f.type]` matches `Int?`, which `f.type` is here
[when f.type] {
    is List<*> { … }
    is Str { … }
}
```

`[when field.kind]` is the dispatch that is exhaustive over every type at
once.

That check needs the fields in view, so a **stamped** comptime fn — `<T is
Struct>`, whose `T` is only known where it is stamped — cannot dispatch on
`[when f.type]` or `[when f.name]` over `T`'s fields at all: it dispatches on
`[when f.kind]`, or tests one type at a time with
`[if f.type is …]`.

### A comptime fn expanded in place

A `comptime fn` with no type parameter is **expanded in place**: it is its own
single instantiation, expanded once where it is declared, and from then on an
ordinary function. One use is handling one field by hand while the rest stay
structural — a `Double` that needs a total order, say:

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

The other use is ordinary code that has to cover **every field of a struct**.
Because an expanded-in-place comptime fn is an ordinary function once
expanded, ordinary code calls it — and because its body is unrolled over the
fields the struct has *now*, a field added later is covered without anyone
remembering to add it. When a function must stay exhaustive over a struct's
fields, writing it as a comptime fn is how to guarantee that:

```
struct Totals { apples: Int, pears: Int, plums: Int }

comptime fn sum(t: Totals) [] -> Int => t {
    let n = 0
    [for field in Totals.fields] {
        n = n + t.[field]
    }
    return n
}

fn report(t: Totals) [Console] => t {
    println("${sum(t)} pieces of fruit")
}
```

The type a comptime construct names — `Totals` in `Totals.fields` — is a type
name like any other: it must be declared in the module or imported where it
is written. A mistake in a comptime construct is reported where it is
written, once; an error in one unrolled copy is reported there too, on the
code as written, saying which copy failed: ``when `field` is `groups`
(`Map<Str, ArgGroup>`): no matching overload for `print_help(Map<Str,
ArgGroup>)` ``.

An expanded-in-place comptime fn may be **generic**, and stays generic. A
generic struct's fields are walked at an *instance* — `Arg<T>.fields` with the
function's own `T`, or `Arg<Int>.fields` — so each field's type comes out with
the arguments in place (`default: T?`). The body is then checked like any
generic function's: `T` is opaque, and interpolating a `T` needs a `?to_str`.
Put together with an exhaustive `[when field.name]`, this is a function every
field of a generic struct has to be considered by:

```
struct Arg<T> {
    short_name: Str? = None
    description: Str? = None
    required: Bool = false
    default: T? = None
}

comptime fn describe<T>(arg: Arg<T>, ?to_str: (x: T) -> Str) -> Str => arg {
    let detail = Mut Str("\t")
    [for field in Arg<T>.fields] {
        [when field.name] {
            is "short_name" | "description" {
                if arg.[field] !is None {
                    detail.append("${arg.[field]} ")
                }
            }
            is "required" {
                detail.append(if arg.required { "(Required)" } else { "(Optional)" })
            }
            is "default" {
                if arg.default !is None {
                    detail.append(" (default: ${arg.default})")
                }
            }
        }
    }
    return detail.to_str()
}
```

Add a field to `Arg` and `describe` stops compiling until it says what to do
with it.

### Your own module

`core.auto` is not special. A module of your own holding `comptime fn cmp<struct
T>(…)`, `eq` and `hash` — comparing an `id` field only, say — is reached by
`by ids`, for every entity in a program. A comptime fn is like an `intrinsic fn` in
what it writes down (its effect list, deductions and return type), and one that
a `params` member is fulfilled by must be effect-free, since an implicit
parameter may resolve to it.

## What is not there yet

A `by auto` on a **generic struct** works: `struct Wrapper<T> : Ordered<self>
by auto { value: T, n: Int }` stamps `fn cmp<T>(a: Wrapper<T>, b: Wrapper<T>,
?Ordered<T>)`, since the copy for a field of type `T` has nothing to compare it
by but what the caller hands in. Each type parameter a field mentions gets the
group (`?Eq<T>` for `eq`, `?Hashed<T>` for `hash`, `?ToStr<T>` for `to_str`),
and the `n: Int` beside it still reaches the ordinary `cmp`. Calls, operators
and interpolation fill those implicits as they would any function's. What does
not work yet is passing such a function as a *value*, which a `Set<Wrapper<Str>>`
needs for its `hash`: a function value cannot carry implicits of its own until
recursive implicit resolution lands, and the compiler says so.

Recorded rather than shipped: `eq by auto` at a **call**, supplying an implicit
by stamping, which is how a tuple would get an identity. It is in ROADMAP.md.
