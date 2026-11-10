# Control flow

## Overview

Salvo has few control-flow constructs: `if`/`elif`/`else`, `when`, `while` and
`for`, plus `return`, `break` and `continue`. Every one of them is an
**expression**: an `if` or a `when` has a value, as in Kotlin or Rust, and so
does a loop. Every block (`if`, `while`, `for`) is its own scope, so variables
declared in it cannot be accessed from outside.

```salvo
fn grades(scores: List<Int>) [Console] -> None {
    for score in scores {
        let grade = when {
            score >= 90 { "A" }
            score >= 70 { "B" }
            else { "C" }
        }
        let note = if score == 100 { " (perfect)" } else { "" }
        println("${score}: ${grade}${note}")
    }
}
```

A few things work differently from the languages you may know:

* **Conditions are `Bool`.** There is no truthiness: a number, a string or an
  optional is not a decision, so compare explicitly.
* **`is` narrows.** A condition `x is T` tests which arm of a union `x` holds,
  and inside the branch `x` reads as a `T`
  ([Data and types](Data-and-Types.md#unions)).
* **`when` is exhaustive.** It either covers every arm of a union, or ends in
  a mandatory `else`.
* **`for` only walks iterators.** There is no C-style
  `for (int i = 0; …)`; a counting loop is `for i in range(0, n)`.
* **A body can be the rest of the block.** A trailing `...` instead of `{ … }`
  makes everything after it the body, which keeps guard-then-continue code
  flat ([Terminal `...` scopes](#terminal--scopes)).

The sections below take each construct in turn, then the operators that join
expressions (`|>`, `?:`, `?>`), and finally re-typing, which drops a
qualifier from a value.

## If/elif/else blocks

As in Ruby, `if` blocks are expressions which evaluate to values. Each branch
of an `if`-`elif`-`else` chain is a possible value that the chain might resolve
to, and the chain's type is the union of them all. The value and type of a
branch are those of the last expression in that branch:

```salvo
// Type of `full_name` is `Str | Str` which simplifies to `Str`
let full_name = if person.surname is Str {
    "${person.name} ${person.surname}"
} else {
    person.name
}
```

The type of an unspecified `else` branch is `None`:

```salvo
// Type of `full_name` is `Str | None` or `Str?`
let full_name = if person.surname is Str {
    "${person.name} ${person.surname}"
}
```

### Conditions

Conditions are boolean expressions: `if`, `elif` and `while` accept `Bool` and
nothing else. There is no truthiness: a number, a string or a possibly-absent
`Bool?` is not a decision, so compare explicitly (`n != 0`,
`name.size() > 0`, `flag ?: unreachable!()`). The `is` tests used for unions
*are* booleans, with the extra syntax `is T name` to bind the narrowed value in
the scope of the block.

## When expressions

A `when` expression is always exhaustive. That is its purpose, and it is
achieved in one of two ways, depending on whether a subject is given.

### With a subject

**With a subject**, `when` branches on the arms of a union type. Since the
arms are known at compile time, the compiler checks that every one of them is
handled:

```text
when [subject variable] {
    [check1] {
        [code]
    }
    [check2] {
        [code]
    }
    ...
}
```

There is no `else` branch in this form: the arms *are* the cases, and covering
them is what the compiler checks. The subject must be a union-typed variable
(for an expression, see the binding form below). The subject followed by a
check should form a valid boolean expression that could work in an `if`
(that is, `[subject] [check]` should be the boolean expression). As with `if`,
any type or qualifier test proven by the check lets the block refer to the
subject by the more specific type. The branches each resolve to a value, as in
an `if`-`elif`-`else` chain, and a branch that ends in an escape such as
`return` contributes nothing to the type. For example:

```salvo
let result: Ok Str | Err Str | Err Bool = some_function()

// `value` of type `Str` because the `Err` branch does not resolve to a value
let value = when result {
    is Ok {
        // `result` is of type `Ok Str`
        result
    }
    is Err {
        // `result` is of type `Err Str | Err Bool`
        println("Error: ${result}")
        return
    }
}
```

### Several cases in one arm

An arm may cover several cases at once: `is A | B` matches either, as
`x is A | B` does in a condition, and inside it the subject is the union of the
two. It takes no binding, since there is no single type to name:

```salvo
let label = when v {           // v: Point | Circle | None
    is Point { "point at ${v.x}" }
    is Circle | None { "something else" }
}
```

### Binding the subject

When the subject is an expression rather than a variable, the **binding form**
`when name = subject { ... }` introduces `name` for it, so no outer `let` is
needed and the arms refer to `name`:

```salvo
let nanos = when last_updated = meta.last_updated {
    is Long { last_updated }   // `last_updated` is `Long` here
    is None { -1 }
}
```

The binding is scoped to the `when`: a `last_updated` after it is an unrelated
name.

### A borrowed subject

A subject may be a *borrowed* union, `proj (A | B)`: a `proj` parameter, an
element of a list of unions met in a `for`, a map entry's value (see
[Projections](Deductions-and-Ownership.md#projections-borrowing-without-copying)).
The `when` matches the union's arms, and each arm stays borrowed: inside
`is A` the subject is a `proj A`, so its fields can be read without copying
anything, and they read as borrows too.

```salvo
namespace Json {
    struct Str { value: Str }
    struct Number { value: Int }
    struct Null {}
}

type Json = Json.Str | Json.Number | Json.Null

fn to_str(json: proj Json) -> Str {
    return when json {
        is .Str { "\"${json.value}\"" }   // `json` is a `proj Json.Str`
        is .Number { "${json.value}" }
        is .Null { "null" }
    }
}
```

The same rules as for any `when` apply: it must cover every arm, an arm may
cover several (`is .Str | .Null`, a borrowed union of the two), and a binding
binds the borrowed arm. What a borrow cannot do, a narrowed borrow cannot do
either: assigning a field of `json` in `is .Str`, or passing it where the
value is consumed or mutated, needs `copy(json)` first. A borrow of a union is
a borrow of whichever arm it holds, so `proj (A | B)` is usable as `(proj A) |
(proj B)`.

Only a read-only borrow reads through to the arms this way. A mutable handle
to a union (what `at` gives over a `List<Mut A | Mut B>`) cannot be matched
arm by arm yet. Any other qualifier on a union is a claim *about* the union,
and `when` refuses it: for `Ok (A | B)`, drop the claim first (`v := -Ok`)
and match the `A | B` it leaves.

### Without a subject

**Without a subject**, `when` is a chain of conditions: an `if`-`elif`-`else`
chain in `when`'s shape. The branch heads are ordinary boolean expressions, and
the `else` is mandatory, since with no arms to cover it is the only thing that
can make the chain exhaustive:

```salvo
// `label` is a `Str`, not a `Str?`: every path produces a value
let label = when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    else { "positive" }
}
```

That mandatory `else` is the reason the form exists. An `if` chain without an
`else` folds `None` into its value, so a chain of conditions that always
produces something has to be written with a trailing `else`, and read carefully
to see that it does; a subject-less `when` says so in its grammar. Everything
else follows from the heads being plain conditions: `is` works in them and
narrows its branch, and a later branch sees the earlier ones ruled out.

```salvo
when {
    value is Str s { println("a string: ${s}") }
    else { println("an int: ${value}") }   // `value` is an `Int` here
}
```

A subject-less `when` gets no arm-exhaustiveness: writing `is` heads that
happen to cover a union does not remove the need for the `else`. Use the
subject form when that is what you mean.

## While loops

`while` loops can be thought of as repeated `if` blocks:

* A `while` loop repeats until its condition evaluates to false.
* It supports the same `is` test as `if` and `elif`, and the value is rebound
  on each iteration of the loop.
* It evaluates to a value, like an `if` block: the last evaluated body
  expression, or the value of a `break` (e.g. `break "hello, world!"`).
* It supports `continue`, which skips to the next iteration.
* It supports an `else` block, which runs only if the loop never ran. This
  works like the `else` of an `if`: its last expression is the value.

```salvo
let numbers: Int[] = get_numbers()
let i = -1
let last = while ++i < numbers.size() { // ++ and -- work in both fixities
    numbers[i] // the last number evaluated will be the value of the while loop
} else {
    -1 // defaults to -1 if never looped
}
```

Without an `else`, a loop that may never run has an optional value: the value
type joins the body's last expression, every `break` value and the `else`, or
`None` when there is no `else`. A bare `break` or a `continue` also joins
`None`, since an iteration may end without producing a value.

## For loops

`for` loops only work with iterators, and do not support the Java- or C-like
`for (int i = 0; ...)` form. An iterator is a standard type, made by functions
such as `range`, and every collection can be walked
([Iterators](Iterators.md) describes how `for` works underneath and how to
write your own). For example:

```salvo
// Print out a message for each age strictly less than the person's age.
// The `range` function returns an iterator which is exclusive of the end.
for i in range(0, person.age) {
    println("Person is older than ${i}...")
}
```

`for` loops evaluate to values just as `while` loops do, and support `break`,
`continue` and `else` in the same way.

## Terminal `...` scopes

A body-taking construct (`if`, `while`, `for`, `try`, `waitfor`) can write its
body as a trailing `...` instead of a `{ … }` block. The **rest of the
enclosing block** then becomes the body. It is for the guard-then-continue
shape, where braces would indent everything that follows:

```salvo
fn greet(x: Str?) [Console] -> None {
    if x is Str ...
    println("hello ${x}")      // runs only when x is a Str; x reads as Str here
}
```

A terminal `...` is **statement-only**: it is the whole statement, never a
value or a nested expression. `let v = if cond ...` and `f(if cond ...)` are
errors; use a `{ … }` block in a value position. And since the body runs to the
end of the block, nothing can follow it at the same level.

The `if` form takes an optional `else`, written as an ordinary braced block
**inline right after the `...`**. It runs when the condition fails, while the
`...` body (the rest of the block) runs when it holds:

```salvo
fn label(x: Str?) [Console] -> None {
    if x is Str ... else {
        println("absent")
    }
    println("present ${x}")    // the rest of the block: the then-body
}
```

There is no `elif` and no `else ...` on a terminal `if`: only the `if` branch
is the terminal scope. Anything more complex is written with braces
throughout.

Loops take `...` the same way: `for n in xs ...` and `while cond ...` make the
rest of the block the loop body (and take no `else`). So do `try ...` and
`waitfor out ...`, and in a comptime fn `[for …] ...` and `[if …] ...`
([Compile-Time Functions](Compile-Time-Functions.md)). A `when` does not: it has
several arms, and the rest of the block is one body.

## Terminal lambda arguments

A call's **last argument** may be a lambda whose body is written as a trailing
`...`, and then the rest of the enclosing block becomes that lambda's body.
This is the trailing-lambda form: a call that takes a function reads as a block
without nesting it in braces:

```salvo
fn doubled(xs: List<Int>) -> Mut List<Int> => xs {
    return map(xs, (e) -> ...)
    return e * 2                 // the lambda body: `e` is in scope here
}
```

This is the same terminal-scope idea applied to a lambda argument:
`(e) -> ...` is `(e) -> { <the rest of the block> }`. The lambda's parameters
(`e`) are in scope for the rest of the block, and the rest runs as the lambda's
body, so the `return e * 2` above is the mapper, while the outer
`return map(...)` returns the mapped list. Only one `...` lambda is allowed per
statement (the rest of the block cannot be two bodies), and as with every
terminal scope, nothing may follow it at the same level.

## Operators and the pipe

### Precedence

Every infix operator binds by one precedence table. From loosest to tightest:

| Tier | Operators |
|---|---|
| pipe | `\|>`, `?>` |
| or | `\|\|` |
| and | `&&` |
| equality | `==`, `!=` |
| comparison | `<`, `>`, `<=`, `>=`, and the `is`/`!is` tests |
| pick | `?:` |
| additive | `+`, `-` |
| multiplicative | `*`, `/`, `%` |
| power | `**` |

All of them are left-associative except `**`, which is right-associative and
also binds tighter than a `-` on its left
([Power](Data-and-Types.md#power)).

Statements end at a newline, so most operators must stay on their line: a line
that *begins* with `+` or `==` is a new statement, not a continuation.
Explicit parentheses group across lines, which is how a precedence other than
the default is written.

A handful of **flattening** operators are the exception: they may begin a
continuation line, and when they do, the newline brackets everything before it
on the left. These are the pick (`?:`), the postfix `.`/method chain, the
`replyto` of a send, and the **pipe**:

```salvo
calculate_age(person) |> list.add(_)

calculate_age(person)
    |> list.add(_)
```

### The pipe: `|>`

`value |> expr` evaluates `value` once, binds it to the placeholder `_`, and
runs `expr` with `_` standing for the value: the same `_` a pick's right side
binds. It flattens a nest of calls into a left-to-right pipeline:
`a() |> f(_) |> g(_)` is `g(f(a()))`, read top to bottom.

The `_` is required (a right side that ignores the piped value is dead
scaffolding), and it may appear more than once only where an ordinary local
could. Because the pipe is the loosest operator, `2 + 3 |> double(_)` pipes the
whole sum, and `x |> f(_) ?: g(_)` pipes `f(_) ?: g(_)` (the tighter pick is
part of the right side).

### The pick: `?:`

The pick `?:` keeps the subject union's **first arm** as the value when the
subject holds it; otherwise the right side runs, with `_` bound to the arms
left behind. `maybe_int() ?: 0` keeps the `Int` and defaults to `0` for the
`None`; `(Ok Int | Err Str) ?: return _` keeps the `Ok Int` and hands the
`Err Str` to the escape. A diverging right side narrows the subject to the
first arm for the code below. [Defaults and early exits](Data-and-Types.md#defaults-and-early-exits-)
has the whole story.

### The null-safe pipe: `?>`

The **null-safe pipe** `?>` is the pipe analogue of safe navigation `?.`. It
picks the left side's first arm, binds `_` to it, runs the right side, and
replaces that arm with the result; the other arms pass through unchanged.

```salvo
let number: Int? = parse(text)
let doubled: Int? = number ?> double(_)
```

`doubled` is `double(number)` as an `Int?` when `number` is present, and
`None` otherwise. On a general union, `(A | B | C) ?> f(_)` with `f: A -> X`
is `X | B | C`. A diverging right side (`x ?> return _`) leaves only the
pass-through arms. It shares the pipe's loosest tier and its
left-associativity, so `?>` and `|>` mix and chain freely across continuation
lines.

## Dropping a qualifier: `x: -Q` and `x := -Q`

`is` narrows: a successful test means the value is *more* specific than its
declared type. A **re-typing** does the opposite: it drops qualifiers, so the
value reads as *less* specific. Its list is the one a deduction clause writes
([Deductions and ownership](Deductions-and-Ownership.md)): `-A -B` drops those,
and `A B` keeps only those, optionally ending in the value's own type.

Written after a value, `e: -Q` re-types it for that one use:

```salvo
fn describe(p: Mut Person) [Console] {
    read_only_report((p: -Mut))     // reaches the `Person` overload
    let plain = p: Person           // the same, keeping no qualifier
}
```

At the top of a call's arguments, `k: v` is a key-value pair (`Map("a": 1)`),
so a re-typing argument goes in parentheses, as above. Written as a statement,
`place := -Q` re-types a variable or field for the rest of its flow, and
`place = place: -Q` is the same statement:

```salvo
let list: Ok Mut List<Str> = f()
if list is NonEmpty {
    list := -Ok                     // `list` reads as `NonEmpty Mut List<Str>` from here
    g((list: -Mut))                 // this call only: the non-`Mut` overload
    g((list: NonEmpty))             // this call only: keep `NonEmpty` alone
}
```

Its main use is the qualified union. `Ok (Ok Int | Err Str)` is a claim
*about* a union, so `when` cannot take its arms apart: they belong to the inner
type. Test the arm, then drop the claim:

```salvo
let nested = try { wrapped(7) }        // Ok (Ok Int | Err Str) | Thrown Str
when nested {
    is Ok {
        nested := -Ok                  // `nested` reads as `Ok Int | Err Str`
        when nested {
            is Ok { println("value ${nested}") }
            is Err { println("error ${nested}") }
        }
    }
    is Thrown {
        println("thrown ${nested}")
    }
}
```

The rules:

- **It never adds a qualifier.** Naming one the value does not carry is an
  error, not a false test: a re-typing drops a known claim, it does not test
  for one (that is `is Q`). It tests nothing at all; the arm test is `is`'s.
- **On a union**, every arm must carry what it drops, and it drops it from
  each. So after `if outcome is Ok`, with `outcome: Ok Int | Ok Str | Err Str`,
  `let value = outcome: -Ok` is an `Int | Str`. Before the test it would be an
  error, since the `Err` arm has no `Ok`.
- **Some qualifiers can never be dropped**: `once` (it restricts rather than
  refines), `Linear` (it carries a use obligation) and `proj` (the value is
  derived from another). A kept list leaves them on without naming them.
  Everything else can be dropped, since dropping a claim loses only knowledge
  and dropping a permission loses only permission.
- **A dropped claim stays dropped** until a value that has it is assigned.
  After an `if`, a variable keeps only what every path kept.
- **Assignment drops qualifiers too**: a variable declared
  `NonEmpty List<Int>` takes an empty list, and reads without `NonEmpty`
  afterwards. `Mut` is no exception: a `Mut Str` variable takes a plain `Str`,
  and reads as a `Str` until a `Mut Str` is assigned.
