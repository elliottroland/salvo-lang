# Compile-time functions

## Overview

A struct's structural `cmp` is the same eight lines for every struct: compare
the first field, then the next, then answer zero. Salvo lets the standard
library write those lines *once*, over the fields of any struct, as a
**compile-time function** (`comptime fn`), and lets a type ask for a copy with
one word, `by`:

```salvo
struct Point : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
    x: Int,
    y: Int
}

fn main() [use] -> None {
    use StdConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 3 }
    println("${p < q} ${p == q}")    // true false
    println("${p}")                  // Point { x: 1, y: 2 }
}
```

`: Ordered<self> by auto` says: `Point` promises a `cmp`, and its body comes
from `auto`, the module [`std.core.auto`](../../lib/std/core/auto.sv). The
compiler **stamps** that module's `cmp` at `Point`, and the result is an
ordinary function declared on `Point`, exactly as if you had written it. If
you know Rust, this is the job `#[derive(Ord, Hash, Debug)]` does, and in
Kotlin the job a `data class` does for `equals` and `toString`. The
difference is that the derive is ordinary Salvo code in a library, checked
like any other code, and you can write your own.

A `comptime fn` is code over the *shape* of a type. Inside one, `[for field
in T.fields]` unrolls its body once per field, `[if …]` keeps or drops code
per copy, and `a.[field]` reads the field the walk is at. Here is one that
lists a struct's field names, and a declaration that stamps it for `Point`:

```salvo
comptime fn field_names<T is Struct>(value: T) [] -> Str => value {
    let out: Mut Str = Mut Str("")
    [for field in T.fields] {
        [if !field.first] {
            append@string(out, ", ")
        }
        append@string(out, field.name)
    }
    return out
}

fn names(value: Point) [] -> Str by field_names    // answers "x, y"
```

`<T is Struct>` makes `field_names` a template: it is never called itself,
only stamped by `by`. A `comptime fn` *without* a type parameter is the other
kind: it is expanded once, where it is declared, and is then an ordinary
function you call. That is how ordinary code covers every field of a struct,
including fields added later:

```salvo
struct Totals { apples: Int, pears: Int, plums: Int }

comptime fn sum(t: Totals) [] -> Int => t {
    let n = 0
    [for field in Totals.fields] {
        n = n + t.[field]
    }
    return n
}
```

The rest of this page is the detail: what `by` does and whose names a stamped
body sees ([`by`: stamping](#by-stamping)), what a `comptime fn` body may say
([`comptime fn`: what gets stamped](#comptime-fn-what-gets-stamped)), and the
expanded-in-place kind ([A comptime fn expanded in
place](#a-comptime-fn-expanded-in-place)). What the standard groups mean is
[Comparison, equality and hashing](Comparison-and-Hashing.md).

## `by`: stamping

`by X` on an obligation clause says where the group's members come from. `X`
is a module, and for each member the module's `comptime fn` of that name is
**instantiated at this type** (stamped). The result is an ordinary function
declared on the type, exactly as if you had written it inside the struct's
body: it travels with the type, `cmp@Point` names it, an implicit `?cmp`
resolves to it, and it carries the struct's own `export`.

**A stamped function has the signature it is declared with.** For a clause
that is the group's: `Ordered<self>` on `Point` declares `fn cmp(a: Point, b:
Point) -> Int`, the group member with `self` replaced by the type, with its
parameter names, its defaults, its effect list (none) and its deductions (the
defaults: both parameters kept). The `comptime fn` supplies only the
**body**, and the body is checked against that signature like any other: the
comptime fn's own parameter names, effect list and deduction clause do not
count. A body that prints where the group's member is pure, or moves a
parameter the member keeps, is an error at the clause, in the terms of the
stamp ("in `take` from `derive` for `P`: `v` is kept by default, but the
body moves it …").

**What `X` may be.** `auto` is `std.core.auto`, which holds `cmp`, `eq`,
`hash` and `to_str` for structs and for unions, named by its suffix, as any
module may be (`size@list` is `size@core.list`). Being in `std.core` means it
can be named without an import, and nothing more: **nothing is stamped unless
you ask**. A struct with no clause has no `cmp` and cannot be compared, and
one with no `to_str` does not interpolate. `X` may also be a comptime fn named
directly (`fn cmp(a: P, b: P) -> Int by by_age`). A name that is both a
module and a comptime fn is refused as ambiguous, and `by @tag` (the module:
a module stands on the right of an `@`, as in `size@list`) or `by tag@mymod`
(the function declared in `mymod`) says which. Hovering the `by` shows what
it stamps, from where.

**`X` names only what the file can see**: its own module, the modules it
imports (whole, or an item of them), and what is implicitly visible
(`std.core`, and in a test file the module it tests). A derive module of your
own is imported like any other, `import derive` beside `by derive`, and a
module you do not import is no reading of a `by` at all. So a project module
that happens to be called `auto` does not touch the standard library's
`by auto`, nor a file of yours that does not import it. A file that does
import it has two `auto`s in sight and is told so; `by std.core.auto` settles
it.

**Named unions.** The clause works on a named union too, written after the
alias:

```salvo
struct Manual : ToStr<self> by auto { note: Str }
struct Imported : ToStr<self> by auto { feed: Str, n: Int }
type Source = Manual | Imported : ToStr<self> by auto
```

Each arm needs the function itself, here also from `by auto`, since a value
in an arm is handled by that arm's own one (see [Unions](#unions)). A free union, `Str | Int` or any
`T?`, has no declaration to carry a clause, so it has no functions of its
own; a comptime fn over a struct that holds one reaches its arms through the
field (below).

**One function at a time.** `by` also works on one function, keeping its full
signature, which is how a type mixes stamped and hand-written members. Here
`Person` is equal by all its fields, and hashes by its name alone, which still
agrees with that equality:

```salvo
struct Person : Hashed<self> {
    name: Str,
    age: Int

    fn eq(a: Person, b: Person) -> Bool by auto
    fn hash(value: Person) -> Long => value { return hash(value.name) }
}
```

The type stamped at is the first parameter's, and the written signature is
the function's: its effect list and deduction clause, as written or
defaulted. It has to have the parameter types and return type the comptime fn
has at that type (a mismatch names which), and a body beside `by` is an
error, since the body is what `by` supplies. So a declaration may say more
than the comptime fn does: `fn eq(a: Person, b: Person) -> Bool => !a by
auto` consumes its first argument, and `fn loud(v: Person) [Console] -> Int by
derive` grants a body that prints. A clause is exactly this, once per member,
with the group's signature as the declaration.

**Parameter names.** The parameter names are the comptime fn's: the body is
its body as written, so the declaration spells its parameters the same way.
`std.core.auto`'s `hash` names its parameter `value`, so `fn hash(h: Person)
-> Long by auto` is an error at `h`:

```text
error: `fn hash`'s parameter `h` must be named `value`, as `auto`'s `hash`
       names it
```

Through a clause the declaration is the group's member, which you do not
write, so a comptime fn meant for a group names its parameters as the group's
member does; one that does not is refused once, at the clause.
`std.core.auto`'s templates use the names of `Ordered`, `Eq`, `Hashed` and
`ToStr`.

**A member need not take the type.** A clause names its type itself, so its
members need not take one. A decoder returns the type and takes something
else:

```salvo
import decode

fns FromStrs<T> {
    fn from_strs(args: List<Str>) -> T
}

struct Serve : FromStrs<self> by decode { port: Int, host: Str }
struct Stop : FromStrs<self> by decode { force: Bool }

let s: Serve = from_strs(List("80", "web"))
```

Each struct gets its own `from_strs`, and since they differ only in what they
return, the type the result goes to chooses ([Functions](Functions.md#overloading-by-return-type)).
The standard library's command-line parser works this way:
[`FromArgs<self> by args`](../../lib/std/args.sv#fns-fromargs) in
[`std.args`](../../lib/std/args.sv).

### Stamping for one call

There is no `by` at a call site: an override such as `hash = …` names a
function, and stamping happens where a function is declared. Declare the one
you want under a name of your own, in your module, and pass that:

```salvo
fn point_hash(value: Point) -> Long by hash@auto
fn point_eq(a: Point, b: Point) -> Bool by eq@auto

let seen: Mut Set<Point> = Mut Set(hash = point_hash, eq = point_eq)
```

`by hash@auto` names the comptime fn, because a `by` path names one after the
function it stamps, and `point_hash` is not its name; `fn hash(value: Point)
-> Long by auto` needs no more. The parameters still carry the template's
names (`value`, and `a` and `b`).

### Whose names a stamped body sees

A template's names mean what they mean **where it is written**, private
declarations included, so a derive keeps its helpers to itself and you
import nothing its body uses. A type, an effect or a qualifier the template
names is the template's: one of yours with the same name does not change it.

Functions are different, because some of a template's calls are meant to
reach yours: the `cmp` of a field's type is declared beside that type, in
your module, where the template's file cannot see it. So a function name in a
template says which place it means:

- **`name@module`**: the one the template's file sees in that module, as
  anywhere else ([Functions](Functions.md#saying-which-one-you-meant)):
  `mix_hash@compare(h, x)`, `append@string(out, " ")`.
- **`name@[site]`**: the one the code it is stamped into sees, as a bare
  `name` written there would reach it, the stamped fn's own implicits
  included and its declared parameters not: `cmp@[site](a.[field],
  b.[field])`.
- **A bare `name` the template's file cannot see** is the site's: there is
  nothing else it could mean.

A bare `name` the template's file *can* see is an error at the template,
whether or not the site sees one too, because the template has not said
which it means:

```text
error: `mix_hash` is visible where `hash` is written, so a template has to say
       which `mix_hash` it means: `mix_hash@compare` for the one it sees, or
       `mix_hash@[site]` for the one the stamping site sees
```

Positions where the template writes no name (an interpolation's `to_str`,
an operator's `cmp` or `eq`, a `for`'s `next`) are the site's, as they would
be in code written there. A type's name called as a constructor
(`Mut Str("…")`) is the type's, so it needs no selector. A comptime fn
expanded in place (below) is stamped nowhere: its names are ordinary, and
`@[site]` is an error in it.

### Where the errors land

A stamped body resolves a `hash` per field, so a field with no `hash` (a
`Double`, a function value, a struct that never opted in) fails in the copy
for that field, and the error is reported **at that field**, prefixed with
what was being stamped:

```text
error: in `hash` from `auto` for `Reading.value: Double`: no matching overload
       for `hash(Double)`
  23 |     value: Double
           ^^^^^^^^^^^^^
```

A stamping can also be refused outright: `std.core.auto`'s `hash` refuses a
`canbe Mut` struct, because a key that can change while a collection holds it
corrupts the collection, and that refusal lands at the clause.

## `comptime fn`: what gets stamped

A `comptime fn` with a bound type parameter is a function over a **kind** of
type rather than a type. Its one type parameter is bound with `is` to an arm
of `Kind`, `<T is Struct>` or `<T is Union>` (the same test a `when` arm
makes), and inside its body the compile-time syntax is legal. A bounded
comptime fn is **stamped**, and is not callable: it exists to be instantiated
by `by`, and by the time the program is checked every use of one has been
stamped and the comptime fn itself is gone. (A comptime fn *without* a bound
is expanded in place and is callable; see [A comptime fn expanded in
place](#a-comptime-fn-expanded-in-place).)

Here is `std.core.auto`'s [`cmp`](../../lib/std/core/auto.sv#fn-cmp) for
structs, in full:

```salvo
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    [for field in T.fields] {
        let c = cmp@[site](a.[field], b.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}
```

Its signature is checked against the declaration it is stamped into, by
position: at `Point`, `(a: T, b: T) -> Int` has to be the declaration's
`(Point, Point) -> Int`. Its names, its `[]` and its `=> a, b` describe this
body only; the stamped function's are the declaration's.

### Walking the fields: `[for …]`

Three things in that body are compile-time:

- **`T.fields`** is the struct's fields, in declaration order, and `[for …]`
  unrolls the loop: one copy of the body per field, each checked with that
  field's type concrete. So `cmp@[site](a.[field], b.[field])` in the copy
  for `x` is an ordinary call `cmp(a.x, b.x)` at your site, resolved like any
  other, and a field with no `cmp` fails there. (`@[site]` says the `cmp`
  meant is the one your code sees, not one `std.core.auto` sees; see
  [Whose names a stamped body sees](#whose-names-a-stamped-body-sees).)
- **`a.[field]`** reads the field the enclosing `[for …]` is at. Inside a
  struct literal, `[field]: value` is the same entry.
- The binder exposes **`field.name`** (a string literal), **`field.type`**
  (the type itself), **`field.kind`** (what kind of type it is),
  **`field.index`**, **`field.first`**, **`field.last`**,
  **`field.default`** and **`field.docs`** (below); the bound parameter
  exposes **`T.name`**, **`T.docs`** and **`T.mutable`**. `to_str` uses
  them:

```salvo
export comptime fn to_str<T is Struct>(value: T) [] -> Str => value {
    let out: Mut Str = Mut Str("${T.name} {")
    [for field in T.fields] {
        [if field.first] {
            append@string(out, " ")
        } else {
            append@string(out, ", ")
        }
        append@string(out, "${field.name}: ${value.[field]}")
    }
    append@string(out, " }")
    return out
}
```

### Conditions: `[if …]`

`[if …]` keeps or drops a branch per instantiation, and the dropped branch is
never checked. Its conditions are compile-time facts: `field.first`,
`field.last`, `field.name == "value"` or `field.name is "a" | "b"`,
`x.index == y.index` (and the other orderings), `T.mutable`, `field.type is
Str` (a type, up to aliases), and `field.kind is Struct` (a kind: an arm of
`Kind`, below). Each `is` has a `!is`, and conditions combine with `&&`, `||`,
`!` and parentheses, as ordinary conditions do:

```salvo
[if field.kind is Struct && !field.last] { … }
[if field.name !is "id" | "created_at"] { … }
[if a.name == b.name] { … }               // binders of two walks
```

`[for …]` and `[if …]` also take a terminal `...` body, as `for` and `if` do:
the rest of the enclosing block becomes the body, so a filter inside a walk is
one line:

```salvo
[for field in T.fields] {
    [if field.name !is "id" | "created_at"] ...
    let c = cmp@[site](a.[field], b.[field])
    if c != 0 { return c }
}
```

`[if cond] ... else { … }` keeps the braced `else` when the condition fails, as
`if cond ... else { … }` does. A `[when …]` takes no `...`: it has several
arms, and the rest of the block is one body.

**Scopes.** Each of these bodies is a scope, as the ordinary body it becomes
is: what a `[for …]` copy, an `[if …]` branch or a `[when …]` arm declares is
gone after it, and it may not reuse a name that is visible there.

### Splices: brackets

**Brackets mark a splice.** A bracket is where expansion puts a comptime
value into the code's structure, as something that is not a value:
`a.[field]` and `[field]: …` turn a `Field` into a field name, and
`[field.type]` turns it into a type, wherever a type is written:

```salvo
[for field in T.fields] {
    let value: [field.type] = decode<[field.type]>(text)
    let seen: List<[field.type]>? = None
}
```

and as a struct literal's head, `[arm.type] { … }`. After `is`, it tests what
the type says, its qualifiers and then its base, so a value parsed as an
`Int` can be checked against the field's own `InRange(1, 65535) Int` and is
narrowed to it:

```salvo
[if field.type is Int] {
    let v = parse_int@string(text) ?: unreachable!()
    if v is [field.type] {
        return v                    // an `InRange(1, 65535) Int`
    }
}
```

A bare `field.type` in a type position is an error naming the bracketed
form. Declared names need no bracket, because they already are types: `T`,
and a pattern binder such as `<E>` below, read the same in the body as in any
generic fn. Values need none either: `field.name` and `"${T.name}"` are
constants an expression already accepts. The heads `[for …]`, `[if …]` and
`[when …]` are bracketed for a related reason, that the same text without
them is a runtime loop or test; inside a head, `field.type is Str` and
`[when field.type]` stay bare.

### Defaults and doc comments

**`field.default` is the field's declared default**, `= 8080` in `port: Int =
8080`, and `None` for a field that declares none. It is evaluated where the
struct is declared, so a default that calls a private function of its module
works from a comptime fn anywhere. `[if field.default is None]` tells the two
cases apart:

```salvo
[for field in T.fields] {
    [if field.default is None] {
        append@string(out, "${field.name} (required)\n")
    } else {
        let d: [field.type]? = field.default
        …
    }
}
```

A field declared `= None` has a default, which is `None`.

**`field.docs` is the field's doc comment**, the `//` lines above it joined as
hover shows them (the prose, without its tags), and `None` where there is
none; `arm.docs` is the comment of the declaration an arm names, and `T.docs`
the type's own. Both `.default` and `.docs` are one thing or the other in
each copy, so test them in a head (`[if field.docs !is None]`) rather than
with `?:`:

```salvo
[for field in T.fields] {
    [if field.docs !is None] {
        append@string(out, "  --${field.name}  ${field.docs}\n")
    }
}
```

### A type and its kind

A type and its kind are kept apart on purpose. `field.type is Union` compares
the field's type with a type called `Union`; `field.kind is Union` asks
whether the field's type is a union. A program is free to declare a type
called `Union`.

### Field names

`field.name` is not just any string: it is one of the struct's field names, a
literal of the union `"x" | "y"` for a `Point`. Testing it against a name the
struct does not have is an error, and `[when field.name]` dispatches on it the
way an ordinary `when` dispatches on a literal union, **exhaustively**:

```salvo
[when field.name] {
    is "sub_commands" | "groups" { }
    is "bools" | "strs" | "ints" {
        for arg in args.[field] { … }
    }
}
```

Like any `when` with a subject it takes no `else`: every field has to be
named by some arm. Add a field to the struct and this stops compiling until
you decide what the code does with it, which is the point: the function stays
exhaustive over the struct's fields. (A stamped comptime fn cannot do this
over `T`'s fields; see [Type patterns](#type-patterns).)

### Refusing an instantiation

A comptime fn can **refuse** an instantiation, with a message in the caller's
terms. `std.core.auto`'s [`hash`](../../lib/std/core/auto.sv#fn-hash):

```salvo
export comptime fn hash<T is Struct>(value: T) [] -> Long => value {
    [if T.mutable] {
        refuse!("a `Mut`-capable struct can change while a collection holds it, so it cannot be a key: drop `canbe Mut`, or hash by hand")
    }
    let h = 17L
    [for field in T.fields] {
        h = mix_hash@compare(h, hash@[site](value.[field]))
    }
    return h
}
```

### Unions

A comptime fn over a union walks its arms. `[when value] { [arm] { … } }`
dispatches on a union value: the one written arm is stamped once per declared
arm, with `value` narrowed to that arm inside, producing an ordinary
exhaustive `when`. `None` is an arm like any other, with `arm.type` `None`:

```salvo
export comptime fn to_str<T is Union>(value: T) [] -> Str => value {
    [when value] {
        [arm] {
            [if arm.type is None] {
                return "None"
            } else {
                return to_str@[site](value)
            }
        }
    }
}
```

`T.arms` walks them as a sequence, and `arm.index` is the arm's declared
position, which is how the union `cmp` orders across arms. A union field of a
struct is reached the same way, through `value.[field]`.

**Building an arm.** In the copy for each arm, a struct arm is a struct
literal headed by the arm's type, a qualified arm is a call to its claim's
constructor, and `None` is `None`:

```salvo
comptime fn first() [] -> Pick {                 // type Pick = A | Ok Str | None
    [for arm in Pick.arms] {
        [if arm.kind is Struct] {
            return [arm.type] { [for f in arm.type.fields] { [f]: make() } }
        }
        [if arm.type is Ok *] {
            return [arm.type]("text")            // `Ok<Str>("text")`
        }
    }
    return None
}
```

**Literal unions.** A literal union's members are one arm at run time, but
comptime can walk them: `[for lit in field.type.literals]` makes one copy per
member, in declaration order, with `lit.value` the member itself:

```salvo
// output: "json" | "text"
[for lit in field.type.literals] {
    if s == lit.value {
        return lit.value                     // a `"json" | "text"`
    }
}
```

`lit.name` is its text, and `lit.index`, `lit.first` and `lit.last` are as
for fields. A named alias walks the same way: `[for lit in Level.literals]`.

### The model: `std.core.comptime`

Everything a comptime fn may know about a type is declared, as compile-time
structs in [`std.core.comptime`](../../lib/std/core/comptime.sv) (abridged
here; the module page documents each member):

```salvo
comptime struct Struct  { name: Str, fields: List<Field>, mutable: Bool, docs: Str? }
comptime struct Union   { name: Str, arms: List<Arm>, docs: Str? }
comptime struct Tuple   { elems: List<Type> }
comptime struct FnType  {}
comptime struct Basic   { name: Str }      // declared `intrinsic type`
comptime struct Generic { name: Str }      // a type parameter of the struct being stamped
comptime type Opaque = Basic | Generic
comptime type Kind   = Struct | Union | Tuple | FnType | Opaque
comptime struct Type  {}      // a type itself; what kind it is is `.kind`
comptime struct Value {}      // a declared value, such as a field's default

comptime struct Field   { name: Str, kind: Kind, type: Type, index: Int, first: Bool, last: Bool,
                          default: Value?, docs: Str? }
comptime struct Arm     { name: Str, kind: Kind, type: Type, index: Int, first: Bool, last: Bool,
                          docs: Str? }
comptime struct Literal { value: Value, name: Str, index: Int, first: Bool, last: Bool }
```

The bound `T` is a [`Struct`](../../lib/std/core/comptime.sv#struct-struct)
or a [`Union`](../../lib/std/core/comptime.sv#struct-union); a `[for …]`
binder is a [`Field`](../../lib/std/core/comptime.sv#struct-field), an
[`Arm`](../../lib/std/core/comptime.sv#struct-arm) or a
[`Literal`](../../lib/std/core/comptime.sv#struct-literal); `field.kind` is a
[`Kind`](../../lib/std/core/comptime.sv#type-kind), and `field.type` the type
itself. These exist at compile time only: the compiler fills them, nothing
constructs one, and naming one in an ordinary type position is an error. But
they are ordinary declarations to read, and the language server shows them:
hovering `T`, `field` or `field.index` inside a comptime fn body answers
`T is Struct`, `field: Field`, `field.index: Int`.

Because `Kind` is a union, a comptime fn that treats kinds of field
differently dispatches on it with an ordinary `when`, exhaustive over the
arms and, like any `when` with a subject, with no `else`:

```salvo
[when field.kind] {
    is Struct { … }
    is Union  { … }
    is Tuple  { … }
    is FnType { … }
    is Opaque { … }
}
```

`Opaque` is a type the comptime fn cannot look inside (an `Int`, a type
parameter, or an `opaque struct` from another module), and `is Opaque`
matches either of its arms, `Basic` and `Generic`, when the body does not
care which. The exhaustiveness is deliberate: when the language gains a kind,
every comptime fn that did not consider it stops compiling rather than
silently skipping it.

### Type patterns

The type after `field.type is` is a **pattern**. `*` stands for any type, at
any depth, so one test can pick out every field of a shape:

```salvo
[if field.type is List<*>] { … }          // any list
[if field.type is List<Arg<*>>] { … }     // a list of any `Arg`
[if field.type is *?] { … }               // any optional
```

`X?` matches any union with a `None` arm, `X` standing for the others, so
`*?` is every optional, `("low" | "high")?` included.

**Binders.** When the body needs the part a `*` stood for, name it: declare a
**binder** before the pattern, the way a function declares its type
parameters, and use it as a type in the branch:

```salvo
[if field.type is <E> List<E>] {
    let first: E? = None
    [if E is Int] { … }
}
```

A binder is the part as written, qualifiers and all; it has a `.kind` like a
field (`[if E.kind is Union]`), and it is in scope only in the branch the
test guards. Under `||` or `!` the test proves nothing about the branch, so a
binder there is an error. Writing the same binder twice constrains:
`<K> Pair<K, K>` matches only a pair of one type. On the right of `&&` a
binder from its left is in scope: `[if field.type is <S> S? && S.kind is
Struct]`. Matching ignores the type's qualifiers (a `List<Int> canbe Mut`
field is a `List<*>`) unless the pattern writes some, and a union matches arm
by arm, in order.

**Qualifiers in a pattern.** A pattern that writes a qualifier requires it,
and a lowercase binder reads its constant:

```salvo
[if field.type is <lo, hi> InRange(lo, hi) *] {
    append@string(out, " (${lo}..${hi})")   // " (1..65535)" for `InRange(1, 65535) Int`
}
```

**Dispatching by pattern.** `[when field.type]` tries the arms in order:

```salvo
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

```salvo
// error: no arm of this `[when f.type]` matches `Int?`, which `f.type` is here
[when f.type] {
    is List<*> { … }
    is Str { … }
}
```

`[when field.kind]` is the dispatch that is exhaustive over every type at
once.

That check needs the fields in view, so a **stamped** comptime fn (`<T is
Struct>`, whose `T` is only known where it is stamped) cannot dispatch on
`[when f.type]` or `[when f.name]` over `T`'s fields at all: it dispatches on
`[when f.kind]`, or tests one type at a time with `[if f.type is …]`.

### Describing a declaration: comptime qualifiers

A comptime fn sometimes needs a fact about a field that its type does not
carry: the short flag a parser accepts for it, or that it is positional. A
**comptime qualifier** says it, written before the field's name:

```salvo
export comptime qualifier Short(c: Char) for Field
export comptime qualifier Positional for Field
export comptime qualifier Named(name: Str) for Struct

Named("srv")
struct Serve {
    Short('p') port: InRange(1, 65535) Int = 8080
    Positional files: List<Str>
}
```

Its subject is `std.core.comptime`'s `Field` (written before a field's name)
or `Struct` (written before a struct, ahead of `export`, on its line or the
line above). It has no body and claims nothing about a value: `port` is an
ordinary `InRange(1, 65535) Int`, and `Short` exists only where it is written
and for the comptime code that reads it. Its slots hold constants: an `Int`,
a `Char` or a `Str`. [`std.args`](../../lib/std/args.sv) declares these
three, and `Subcommand`, for its command-line parser; see
[`Short`](../../lib/std/args.sv#qualifier-short) and the others there.

Comptime code tests for one with `is`, and reads its constants through a
lowercase binder declared before it:

```salvo
[for field in T.fields] {
    [if field is <c> Short(c)] {
        append@string(out, "-${c} ")    // "-p "
    }
    [if field !is Positional] {
        append@string(out, "--${field.name}")
    }
}
[if T is <n> Named(n)] { … }            // a struct's own
```

### Computing in a head: `[let]` and the compile-time functions

A head is a small expression language over the model, so a comptime fn can
state a rule about a declaration as a whole: that a struct marks exactly one
field as its key, say. `[let name = …]` holds a compile-time value for the
rest of the block, and a head may call the functions
[`std.core.comptime`](../../lib/std/core/comptime.sv) declares for it:
[`size`](../../lib/std/core/comptime.sv#fn-size),
[`get`](../../lib/std/core/comptime.sv#fn-get),
[`filter`](../../lib/std/core/comptime.sv#fn-filter),
[`count`](../../lib/std/core/comptime.sv#fn-count),
[`any`](../../lib/std/core/comptime.sv#fn-any) and
[`all`](../../lib/std/core/comptime.sv#fn-all), with lambdas over the
elements:

```salvo
export comptime qualifier Key for Field

comptime fn key_name<T is Struct>(value: T) [] -> Str => value {
    [let keys = filter(T.fields, (f) -> f is Key)]
    [if size(keys) != 1] {
        refuse!("`${T.name}` marks ${size(keys)} fields as `Key`; mark exactly one")
    }
    [for f in keys] ...
    return f.name
}
```

`[for x in …]` walks any list of fields, arms or literals, so the walk over
`keys` is a walk over the marked fields only. An element keeps its declared
`.index`, `.first` and `.last`: they describe where it is declared, not
where it is in the list. A head has the ordinary operators (`&&`, `||`, `!`,
comparisons, and arithmetic on `Int`s and `Long`s), and a `refuse!` message
may interpolate what a head computes.

The values exist only while the compiler stamps. A `[let]` holding a field
is a binder from then on (`m.[f]`, `f.name`), and one holding a number, a
`Bool` or a `Str` reads as that literal in code; a list or a type is for
heads only. A head sees only these functions: an ordinary `size` in scope is
an error there, so the two are never confused. Mistakes in a head (comparing
a list with a number, say) are reported where the comptime fn is stamped, as
every mistake in a template is. Evaluation always ends: a head has no loops,
and these functions walk lists the compiler built.

### A comptime fn expanded in place

A `comptime fn` with no type parameter is **expanded in place**: it is its own
single instantiation, expanded once where it is declared, and from then on an
ordinary function. One use is handling one field by hand while the rest stay
structural, such as a `Double` that needs a total order:

```salvo
struct Reading : Ordered<self> {
    sensor: Str,
    unit: Str,
    value: Double

    comptime fn cmp(a: Reading, b: Reading) -> Int => a, b {
        [for field in Reading.fields] {
            [if field.name == "value"] {
                let c = total_cmp(a.value, b.value)    // a total order of your own
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

**Covering every field.** The other use is ordinary code that has to cover
**every field of a struct**. Because an expanded-in-place comptime fn is an
ordinary function once expanded, ordinary code calls it, and because its body
is unrolled over the fields the struct has *now*, a field added later is
covered without anyone remembering to add it. When a function must stay
exhaustive over a struct's fields, writing it as a comptime fn is how to
guarantee that:

```salvo
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

The type a comptime construct names (`Totals` in `Totals.fields`) is a type
name like any other: it must be declared in the module or imported where it
is written. A mistake in a comptime construct is reported where it is
written, once; an error in one unrolled copy is reported there too, on the
code as written, saying which copy failed: ``when `field` is `groups`
(`Map<Str, ArgGroup>`): no matching overload for `print_help(Map<Str,
ArgGroup>)` ``.

**Generic comptime fns.** An expanded-in-place comptime fn may be
**generic**, and stays generic. A generic struct's fields are walked at an
*instance* (`Arg<T>.fields` with the function's own `T`, or
`Arg<Int>.fields`), so each field's type comes out with the arguments in
place (`default: T?`). The body is then checked like any generic function's:
`T` is opaque, and interpolating a `T` needs a `?to_str`. Put together with
an exhaustive `[when field.name]`, this is a function every field of a
generic struct has to be considered by:

```salvo
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

`std.core.auto` is not special. A module of your own holding
`comptime fn cmp<T is Struct>(…)`, `eq` and `hash`, comparing an `id` field
only, say, is reached by `import ids` and `by ids`, for every entity in a
program. A comptime fn is like an `intrinsic fn` in what it writes down (its
effect list, deductions and return type), and one that a `fns` member is
fulfilled by must be effect-free, since an implicit parameter may resolve to
it.

## Generic structs

A `by auto` on a **generic struct** works: `struct Wrapper<T> : Ordered<self>
by auto { value: T, n: Int }` stamps `fn cmp<T>(a: Wrapper<T>, b: Wrapper<T>,
?Ordered<T>)`, since the copy for a field of type `T` has nothing to compare
it by but what the caller hands in. Each type parameter a field mentions gets
the group (`?Eq<T>` for `eq`, `?Hashed<T>` for `hash`, `?ToStr<T>` for
`to_str`), and the `n: Int` beside it still reaches the ordinary `cmp`.
Calls, operators and interpolation fill those implicits as they would any
function's, and so does passing such a function as a *value*: `takes(eq)`
with `eq` stamped for `Wrapper<T>` is a function of two `Wrapper<Str>`s, its
`?Eq<Str>` filled where it is passed, as a `Set<Wrapper<Str>>` needs for its
`hash`.

## Not there yet: stamping at a call

Recorded rather than shipped: `eq by auto` at a **call**, supplying an
implicit by stamping, which is how a tuple would get an identity. It is in
ROADMAP.md.
