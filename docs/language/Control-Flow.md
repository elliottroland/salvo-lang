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
* **One statement per line.** There is no `;`: a statement ends at the end
  of its line, and the next one starts on a new line
  ([Statements and lines](#statements-and-lines)).

The sections below start with how statements are laid out, take each
construct in turn, then the operators that join expressions (`|>`, `?:`,
`?>`), then re-typing, which drops a qualifier from a value, and finally
`type?` and `type!`, which show and check what a place's type is at a given
line.

## Statements and lines

A statement ends at the end of its line, and Salvo has no separator to put a
second one after it: `let a = 1 let b = 2` and `print(a) print(b)` are
errors, each asking for the second statement on a line of its own. The same
holds after a block's closing `}`: `if c { a } b` is two statements and is
refused, since nothing continues an `if` but `elif` and `else`. A terminal
`...` head is a statement of its own line too, so the body it takes starts on
the next line: `for x in xs ... println(x)` is refused, as is a statement after
the inline `else { … }` of an `if c ...`, and the error names the braces form
that keeps the body on the line, `for x in xs { println(x) }`. A `;` is an
error too, one that names the fix.

What the grammar itself continues stays on one line: a one-line block holding
one statement (`fn one() -> Int { return 1 }`, `if c { return }`, an arm's
`is Int { 0 }`, a lambda's `(x: Int) -> Int { return x }`), `} else {` and
`} elif … {`, a `when`'s arms, a struct's fields and a literal's, and an
operator written on the line it continues.

```salvo
fn sign(n: Int) -> Int {
    if n < 0 { return -1 } elif n == 0 { return 0 } else { return 1 }
}

fn report(n: Int) [Console] -> None {
    let s = sign(n)
    println("${n}: ${s}")    // not `let s = sign(n) println(…)`
}
```

Top-level declarations follow the same rule: `struct A {} struct B {}` on
one line is refused. The rule does not reach a declaration's members: two
`fn`s of an effect or a handler written on one line are still accepted.

Kotlin, Swift and Go accept `;` as an optional separator; Salvo does not, so
the end of a line is the only thing that ends a statement.

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

### Stepping in the condition

A step is an assignment: `++i` stores `i + 1` in `i`, and its value is the
new `i`. So `++i is Q` tests the variable's new value, and when the test
holds, `i` itself is a `Q` in the body. That makes the common "advance, then
check I am still in range" loop one line:

```salvo
fn first_negative(xs: List<Int>) -> Int? {
    let i = -1
    while ++i is Idx(xs) {
        let x = get(xs, i)          // the total `get`: `i` is an `Idx(xs) Int` here
        if x < 0 {
            return x
        }
    }
    return None                     // `i` is a plain `Int` here
}
```

The step happens whether or not the test holds. Where it failed (the
`else`, after the loop) `i` has still moved on, and is the plain `Int`
the step left. Each iteration's test makes the claim again: what the
previous iteration knew is not carried round the loop. `if`, `elif` and a
subject-less `when` read a stepping condition the same way, and so does the
left side of `&&`, whose right side and branch see the claim (the right
side of `||` runs only when the left test failed, so it does not). A write
later in the condition ends the claim: in `++i is Idx(xs) && ++i < n`, and
in an `elif` that steps `i` after an earlier condition narrowed it, `i` is a
plain `Int` again.

The postfix step is different: `i++ is Q` tests the *old* value, which `i`
no longer holds, so nothing is narrowed. The same is true of any test whose
subject is not a variable or a field path: the test runs, and only a binding
(`is Q name`) names what it proved.

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

Every infix operator binds by one precedence table. From loosest to tightest,
with where each may break a line:

| Level | Operators | Line breaks |
|---|---|---|
| pipe | `\|>`, `?>` | may begin or end a line |
| pick | `?:` | may begin or end a line |
| or | `\|\|` | may begin or end a line |
| and | `&&` | may begin or end a line |
| equality | `==`, `!=` | none, outside brackets |
| comparison | `<`, `>`, `<=`, `>=`, the `is`/`!is` tests, `as!`/`as?` | none, outside brackets |
| additive | `+`, `-` | none, outside brackets |
| multiplicative | `*`, `/`, `%` | none, outside brackets |
| power | `**` | none, outside brackets |

All of them are left-associative except `**`, which is right-associative and
also binds tighter than a `-` on its left
([Power](Data-and-Types.md#power)). A union's `|` (`x is A | B`, a union
type) binds tighter than `is`: `x is A | B` tests `x` against `A | B`.

Each operator is written whole: no space or line break may come inside
`?:`, `?.`, `?>`, `!is`, `as!`, `as?` or any other, so `a ? : 0` and `b !
is Q` are errors naming the operator to write. The spaces around an
operator are free (`a?:0` is `a ?: 0`).

The pick sits just inside the pipe, looser than `||` and `&&` (where Kotlin's
`?:` is tighter than comparison), so its right side is a whole condition or
sum: `n ?: 0 > 5` is `n ?: (0 > 5)`, `ok && done ?: false` is
`(ok && done) ?: false`, and `a ?: b + 1` is `a ?: (b + 1)`. To compare a
pick, write `(n ?: 0) > 5`.

Statements end at a newline, except where the next line begins with an
operator that continues it. The operators come in three tiers by where a
line may break:

* **The pipes and the pick** (`|>`, `?>`, `?:`) may begin a continuation
  line, after a `}` too, or end the line they continue. A line that begins
  with one applies it to everything before it (below), a pipe included:
  `x |> f(_)` with `?: d` on the next line is `(x |> f(_)) ?: d`.
* **`||` and `&&`** may begin a continuation line or end the line they
  continue, and bind there by the table, as on one line:

  ```salvo
  let ready = loaded
      && checked || forced  // (loaded && checked) || forced
  let picked = cached ?: loaded
      || fetched            // cached ?: (loaded || fetched)
  ```

* **Every tighter operator**, and a union type's `|`, neither begins nor
  ends a line outside brackets. A line that begins with `+` or `==` after
  an expression is an error, and so is one that ends with `+`; a `-` that
  begins a line is a negation, so `a` then `-1` on the next line is two
  statements. To break such an expression, put it in parentheses, inside
  which a newline is nothing:

  ```salvo
  let total = (base
      + rate * hours)       // base + (rate * hours)
  type Json = (
      Null
      | Bool
      | Number
      | Text
  )
  ```

  The formatter never breaks these for the width: a line too long with one
  stays long, as a line holding a long string does.

Some places are not continued:

* **After a `}`**: a line that begins with `||` or `&&` directly after a
  `}` (a block's, a `when`'s, a struct literal's) is an error. The `}` may
  end a statement (`if c { … }`), so the line would read as a continuation
  to some readers and as a new statement to others; write the operator at
  the end of the line above, or put the expression in parentheses
  (`(Point { x: 1 })`), whose `)` is continued.
* **A line that begins with `--` or `++`** is a prefix step, a statement of
  its own: those are one token each.

A line after a `return`, `break` or `continue` with no value is a statement
of its own, and so is the line after a `(x) -> ...` argument, which begins
the lambda's body. After a line that begins with `.` (which applies to
everything before it), a line that begins with `||` or `&&` continues the
whole: `xs` / `.is_empty()` / `|| ok` is `xs.is_empty() || ok`.

Explicit parentheses group across lines, which is how a precedence other than
the default is written: inside `(…)`, `[…]` and a struct literal's `{…}` a
newline is nothing.

A handful of **flattening** operators work differently: they may begin a
continuation line, after a `}` too, and when they do, the newline brackets
everything before it on the left. These are the pick (`?:`), the postfix
`.`/method chain, the `replyto` of a send, and the **pipe**:

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

## Finding out a type: `type?` and `type!`

Narrowing, re-typing, mutation and loops all change what a place's type is
from one line to the next. Two compiler forms let you see it. `type?(place)`
asks: the compiler reports the place's type at that point, as a warning that
fails nothing, and the program does not change. `type!(place: T)` states
it: the program does not compile unless the place's type is exactly `T`. It is
TypeScript's hover type, or a `static_assert` on the type, written in the
program. Both are checked when compiling, never at run time: the `?` asks,
and the `!` says the build can break.

```salvo
struct Parsing canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

fn step(parsing: Mut Parsing) [] -> None => parsing: Mut {
    let { str, pos } = parsing
    type?(pos)                          // warning: type of `pos`: Idx(str) Int
    type?(parsing.pos)                  // warning: type of `parsing.pos`: Idx(parsing.str) Int
    type!(pos: Idx(parsing.str) Int)    // holds: `str` is `parsing.str`
    pos++
    type!(pos: Int)                     // the step left the claim behind
    if pos is Idx(str) {
        parsing.pos = pos
    }
}
```

- **The argument is a place**: a variable, a field path such as
  `parsing.pos`, or a destructured binding, borrowed or not. A call, a
  literal or a subscript is refused: bind the value to a local first.
- **Neither form does anything at run time.** Both are `None`, nothing is
  evaluated, and no code is generated for them. The place is not read: it is
  not moved, borrowed or narrowed, a linear value is not ended, and a local
  named only in them is still reported as unused.
- **The type is the whole type**, every qualifier included (`proj`, `Mut`,
  claims such as `Idx(str)`), written with a borrow first and the rest in name
  order. The order you write them in `type!` does not matter.
- **A claim is compared by the value it is about**, not by its spelling: in
  the example, `Idx(str)` and `Idx(parsing.str)` are one claim, because the
  destructured `str` is `parsing.str`. A claim about another cursor's `str`
  fails the check even when it prints the same.
- **Exact means exact**: `type!` checks the type, it does not test or
  convert (`is` tests; `x := -Q` drops). A `-Q` or `+Q` list is refused, and
  so is a message, since the check never fails at run time.
- **`assert!` is not a type check.** `assert!(cond)` is a run-time
  assertion ([Testing](Testing.md#assertions)), so `assert!(x: Int)` asserts
  the re-typing `x: Int` as a condition, and is refused because an `Int` is
  not a `Bool`.
- **Inside a loop**, the compiler may walk the body twice, the second time
  with what the first walk left at the end of the body. `type?` reports what
  the last walk saw, once, and `type!` is judged on that walk.
