# The IR

## Overview

Between the checker and the backends, the compiler rewrites a program into
its **IR** (intermediate representation): a small language in which every
question the checker answered is written down. Both backends, Kotlin and Rust,
read the IR and nothing else.

Why a separate form at all? The surface language leaves a lot for the
compiler to work out, and two backends that each worked it out again could
work it out differently. With the IR, the answers are made once and stored,
and a backend only decides how to say them in its own language. If you know
a compiler with a typed middle layer (Kotlin's IR, Rust's HIR and MIR), this
is the same idea, kept small: a backend never infers anything, it renders
nodes.

Here is a function that tests a union:

```salvo
fn show(x: Int | Str) -> Str {
    if x is Int {
        return "int ${x}"
    }
    return "str"
}
```

and its IR:

```text
fn show@main(lent x: Int | Str) -> Str {
  branch#1 cond#0 check#2 (read x is arm 0) {
    narrow#3[check#2] x~1: Int = x
    return concat("int ", call to_str(Int)@std.core.basic(read x~1))
  }
  return "str"
}
```

Everything the source left implicit is spelled out. The parameter is `lent`
(borrowed, not moved). `x is Int` became a test of union arm 0. Inside the
branch, `x` is a new local `x~1` of type `Int`, and the `narrow` line says
which test (`check#2`) proved it. The string template is a `concat` of
`Str` parts, and the `Int` became text through a call of one particular
`to_str`, in `std.core.basic`.

`salvo dump --phase generate-ir` prints the IR ([Dumps](Dumps.md) covers the
other phases):

```sh
salvo dump --phase generate-ir --src ./my_project                  # every module of the project
salvo dump --phase generate-ir --src ./my_project --module main    # one module
salvo dump --phase generate-ir --src ./my_project --all            # with the std modules it reaches
```

This page explains how to read it. You need it when a backend's output
surprises you, when the two backends disagree, or when you work on the
compiler. The IR is built exactly as the backends build it, so it is the
point to bisect at: if the IR is right, the fault is in a backend; if it is
wrong, it is in the checker or the builder.

The printed form is for reading and for golden tests. Nothing parses it back.
It names declarations as every dump does
([How a dump names things](Dumps.md#how-a-dump-names-things)).
The Rust types behind it are in `crates/salvo-core/src/ir/nodes.rs`, and the
[tour of the compiler](../../TOUR.md) follows a sample program through them.

## What the IR decides

The surface language leaves a lot for the compiler to work out: which
overload a call means, which handler a member call goes to, what a variable's
type is after `is`, whether a value moves or is borrowed, which union arm a
value goes into. The checker works all of that out. The IR is where those
answers are stored, as fields of the nodes that need them. A backend never
infers, and when it meets a node it cannot render it reports an error rather
than guessing.

What the IR leaves to a backend is **idiom**. The IR says "switch on `x`: if
`None`, this; otherwise bind the value". Kotlin turns that into `x ?: …`, and
Rust into a `match`. The IR says "this read consumes the value". Rust writes a
move, and Kotlin writes nothing. The IR never contains `clone`, `import`,
`&mut`, `?:`, or an operator a backend has to look up.

A few rules hold everywhere:

* **Every expression carries its type**, already resolved.
* **Every reference is absolute.** A call names one declaration, in one
  module, so the IR has no imports. Each backend works out what its target
  language needs to import from the references.
* **Narrowing is a new variable.** After a successful test, the narrowed
  value is a fresh local with its own type and a note of which test proved it.
* **Effects are parameters.** A function's effects are its first parameters,
  and a member call goes through one of them.
* **Generics stay generic.** A generic function is one declaration with type
  parameters, and each call passes its type arguments.

## Reading a dump

A dump is one module: `module main`, then one declaration per paragraph. Here
is a small program, and below it what the dump prints for it.

```salvo
struct Box canbe Mut { n: Int, tag: Str }

fn bumped(b: Mut Box) -> Int {
    b.n = b.n + 1
    return b.n
}

fn total(items: List<Int>) -> Int => !items {
    let sum = 0
    for i in items {
        sum = sum + i
    }
    return sum
}

fn main() [use] -> None {
    use StdConsole
    let b = Mut Box { n: 1, tag: "t" }
    println("${bumped(b)} ${b.tag}")
    let xs = List(3, 2, 1)
    println("${total(xs)}")
}
```

```text
module main

struct Box canbe Mut wire {
  n: Int
  tag: Str
}

fn bumped@main(lent_mut b: Mut Box@main) -> Int {
  assign b.n = (read b.n + 1)
  return read b.n
}

fn total@main(moved items: List<Int>) -> Int {
  bind#1 let sum: Int = 0
  foreach#2 i: Int in read items {
    assign sum = (read sum + read i)
  }
  return !read sum
}

fn main@main() -> None {
  bind#1 let __use~1: StdConsole = construct StdConsole { }
  bind#2 let __handle~2: Console = read __use~1
  bind#3 let b: Mut Box@main = construct Mut Box@main { n: 1, tag: "t" }
  call println(Str)@std.core.console(read __handle~2, concat(call to_str(Int)@std.core.basic(call bumped@main(read b)), " ", read b.tag))
  bind#4 let xs: List<Int> = call List(T, T[])@std.core.list<Int>(3, array[Int[]](2, 1))
  call println(Str)@std.core.console(read __handle~2, concat(call to_str(Int)@std.core.basic(call total@main(!read xs))))
}
```

The things to notice:

* **References are `name@module`**, Salvo's own selector, with the parameter
  types added when the module overloads the name: `to_str(Int)@std.core.basic`,
  and `List(T, T[])@std.core.list`, std's variadic `List`, whose trailing
  arguments arrive as an array. An effect member is `member@Effect`
  (`now@Clock`). A type another module could also declare carries its module
  (`Box@main`); std's own names stay bare. A declaration with no recorded
  name prints its path and module, such as `#4.0@main`.
* **Parameters say how they are passed.** `lent` means borrowed, `lent_mut`
  means borrowed and mutated, `moved` means handed over. These come from the
  deduction clause (see [Deductions and Ownership](Deductions-and-Ownership.md)).
* **`!read` is a read that consumes the value**: `total(!read xs)` hands `xs`
  over, and `xs` is not used afterwards. A plain `read` leaves the value
  where it is.
* **`#n` is a node id**, unique within one function. A narrowing names the
  node that proved it by this id. The ids have gaps, because the builder
  numbers nodes it later folds away.
* **`~n` marks a local the compiler made**, or a source name reused for a
  different value (`x~1` is the narrowed `x`). Names starting `__` are
  generated.
* **`"${…}"` is a `concat`** of `Str` parts. Each interpolated value becomes
  the call `to_str(value)`, resolved as a written call is; a `Str` whose
  `to_str` is std's own is the value itself.
* **The `wire` flag** on `Box` means every field has a wire form, so the
  struct can travel between actors on different nodes.

## Errors in an IR dump

A program with type errors has no IR: `generate-ir` prints nothing, the
errors go to stderr, and the command fails. (The other phases print the
program anyway.)

One kind of error is found *on* the IR rather than before it: a linear value
that some path neither moves on nor settles ([Linear Types](Linear-Types.md)).
Such a program still has its IR, and the dump prints each leak after `!!!`:
at the end of the line of the statement that produced it, or, as here, on a
line of its own after the module's declarations:

```text
fn drop_it@main(moved out: Reply<Int>) -> None {
  obligation gain-params [out]
  bind#1 let x: Int = 1
}

!!! main:1:45: `out` still owns a linear value when the path leaves; move it onward (pass, return, or store it) or discharge it
```

The `obligation` lines are the ledger that check reads; see
[Statements](#statements).

## Types

A type in the IR is the checker's type with the qualifiers erased
([Qualifiers](Qualifiers.md)). Two qualifiers survive, because a backend may
represent them differently:

* **`Mut`**: Kotlin represents `Mut Str` as a `StringBuilder` and `Mut List<T>`
  as a `MutableList<T>`.
* **`proj`**: Rust represents a projection as a borrow.

Two more rules keep union arms apart once qualifiers are gone:

* **A qualifier stays on an arm when erasing it would merge two arms.**
  `Err Str | Thrown Str` keeps both tags. `Emitted Int | Finished` becomes
  `Int | Finished`, because nothing clashes.
* **Literal types become their base type.** `"a" | "b"` is a `Str`.

A union's arms are numbered in order, skipping the `None` arm.
`make_union[Int | Str | None]#0(4)` puts `4` into arm 0.
An optional `T?` is the union `T | None`.

## Declarations

| Dump | What it is |
|---|---|
| `struct Name<T> canbe Mut wire { field: Ty }` | a struct. It may also be `export`, `opaque` or `linear`, and a field may be `canbe_mut` or have a default |
| `union Name = A \| B` | a named union type |
| `interface Name [Prereq] { fn m(…) -> T }` | an effect. An `actor interface` is an actor effect, and its `send fn` members are messages |
| `impl H(ctor params) for Face deps [Dep] stateful { … }` | a handler: constructor parameters, the effects it depends on, its `state` fields, an optional `mailbox`, `init` and member bodies. It may also be `platform`, `threadsafe` or `intrinsic` |
| `fn name@module<T>(effects; params; ?implicits) -> T` | a function. `;` separates the leading effect parameters, the declared ones and the trailing implicit ones |
| `intrinsic fn …`, `platform fn …` | a function with no body. A backend lowers an intrinsic from its own table, and the host implements a platform function ([Backends](Backends.md)) |
| `qualifies fn …` | the test of a predicate qualifier |
| `platform type Name` | a type the host defines |
| `static name: Ty { … }` | a module-level `use`: an instance built once, which every function in the module reads |
| `enum Name { Variant(field: Ty) }` | a generated enum with named variants (see [Actors](#actors)) |

A generic function with an implicit parameter shows all three groups:

```text
fn bigger@main<T>(moved a: T, moved b: T; ?lent cmp: (T, T) -> Int) -> T {
```

After the return type, a function signature may list:

* **`borrows [p]`**: the result holds a view of parameter `p` (a projection,
  `proj`): `fn first_of@main(lent xs: List<Str>) -> proj Str? borrows [xs]`.
* **`holds [p <- q]`**: after the call, parameter `p` holds a view of `q`
  (the deduction `p.f: proj(q)`):
  `fn repoint@main(lent_mut v: Mut View@main, lent other: List<Int>) -> None holds [v <- other]`.
* **`throws M`**: the function may throw a message of type `M`
  ([Throwing](Throwing.md)). A `[Throw<Str>]` in the effect list is not an
  effect parameter in the IR; it becomes `throws Str`.
* **`anchors a in c, d in c`**: parameters typed `ref(c)`, handles into
  the container parameter `c`, which may therefore name the same element
  (`FnDecl::ref_anchors`; [Mutable Handles](Mutable-Handles.md)). A handle
  parameter's own type dumps as `ref<C> Mut T`, naming its container's type
  (what a backend builds the handle's storage path from):
  `fn strike@main(lent_mut c: List<Mut Fighter@main>, lent_mut a: ref<List<Mut Fighter@main>> Mut Fighter@main, lent_mut d: ref<List<Mut Fighter@main>> Mut Fighter@main) -> None anchors a in c, d in c`.

## Statements

A block is a list of statements, optionally followed by `value e`, the
block's value.

| Dump | Meaning |
|---|---|
| `bind#n let x: T = e` | a new local |
| `assign place = e` | a write to a local, a field or an element |
| `narrow#n[reason] x~1: T = x` | a new local holding `x` at the narrower type `T`, justified by `reason` ([Control flow](#control-flow)) |
| `alias#n x: T = place` | a name for a place. Every read and write of `x` is one of the place |
| `unpack#n (a: A, b: B) = m as variant i` | the payload of an enum variant, in the arm that tested for it |
| `loop#n { … }` | a loop with no condition, left by `break` |
| `foreach#n x: T in e { … }` | a loop over a list, an array or a `Str` |
| `return e`, `break`, `continue` | as in Salvo |
| an expression | evaluated for its effect |
| `obligation gain [x]`, `gain-params`, `release`, `discharge` | the linear-obligation ledger: where a linear value is gained (a binding, or the linear parameters on entry), moved on, or settled. These lines are added for the dump and the linear check only; the IR a backend reads has none |

A place is a local followed by field, tuple and index steps: `b.n`,
`pair.0`, `xs[i]`. Reading a place is the only way to use a variable.

## Expressions

| Dump | Meaning |
|---|---|
| `1`, `5L`, `2.0`, `true`, `'c'`, `"s"` | literals |
| `unit` | the `None` value where the type is `None` |
| `none[T?]` | the absent arm of an optional |
| `read p`, `!read p` | a read of a place. `!` means the read consumes the value |
| `call f<T>(args)` | a call of a declaration |
| `call local f(args)` | a call through a function-typed local |
| `member now@Clock(instance, args)` | a call of a member of an effect, through an instance. A member the program does not declare prints by its effect and index, `Clock#0` |
| `(a + b)`, `!(a)`, `-(a)` | the host's operator on basic types, written inline: `+ - * / %`, `&& \|\| !`, unary `-`, `< > <= >=`, `== !=` (see [Comparisons and operators](#comparisons-and-operators)). A binary operation is always parenthesized, and so is a unary operand, so `!(read x)` cannot be mistaken for the consuming `!read x` |
| `construct T { f: e }` | a struct or handler value |
| `make_union[U]#i(e)` | `e` placed into arm `i` of the union `U` |
| `present(e)` | `e` placed into the present arm of an optional. Rust writes `Some(e)`, and Kotlin writes `e` |
| `rewrap[A -> B](e)` | a union value moved to another union's arm numbering: `rewrap[Int \| Str -> Int \| Str \| Bool](!read x)` |
| `widen[Long](e)` | a number widened within its class (`Int` to `Long`, `Float` to `Double`) |
| `drop_mut(e)` | a `Mut` value used where the plain type is needed |
| `into_mut(e)` | the reverse: a plain value stored into `Mut` storage |
| `borrow_as[proj Mut A -> A, B -> B](e)` | a borrow read at a position that takes the owned type (a kept parameter), arm by arm: for each arm of the target, the argument's arm that lands there. `ref Mut A -> Mut A` is a handle read as its element, `proj Mut A -> A` a borrow read as the value (dropping `Mut`), `B -> B` an owned arm lent. A single value is the one-arm case, `borrow_as[proj Str -> Str](e)` |
| `tuple(…)`, `list[T](…)`, `array[T](…)`, `...xs` | literals. A spread appears only inside an array |
| `concat(…)` | string concatenation. Every part is a `Str` |
| `union_to_str(u, f0, f1, …)` | the text of a union value: the function for the arm `u` holds, called on its payload |
| `branch#n cond#0 c { … } else { … }` | `if`/`elif`/`else`: ordered conditions |
| `switch#n e { arm#0 (test) { … } }` | a choice on the arms of a union |
| `check#n (e is test)` | a union test used as a `Bool` condition |
| `lambda(params) -> T captures [x, !y, mut z] { … }` | a function value. A capture marked `!` is consumed, and one marked `mut` is mutated |
| `fn f` | a declared function used as a value |
| `try[T] { … }`, `throw(m)` | [Throwing](Throwing.md) |
| `assert(c, m) at main:3:5`, `unreachable(m) at …` | traps. `at` is the Salvo source location the failure message names |
| `handle[E](h)` | a handler instance bound as the effect `E`: the shared handle a `use` makes |
| `spawn[…](…) serves [E]`, `send m@E(addr, args)`, `waitfor r: T { … }`, `replyto k(captures)`, `handler_call`, `instance_at(addr)`, `self_addr`, `self_send m(args)` | actors (see [Actors](#actors)) |
| `UNSUPPORTED(what)` | a construct the builder does not lower yet. It always comes with an error, so no backend renders it |

A `switch` arm's test is one of: `arm i`, `arms [i, j]`, `none`, `else`, or
`lit` for an arm limited to some literal values (a `"GET" | "PUT"` collapsed
into one `Str` arm).

## Comparisons and operators

A comparison is the call it names (`eq(a, b)`, `!eq(a, b)`, `cmp(a, b)`
against `0`), and the IR writes it as what that call resolved to
([Comparison, equality and hashing](Comparison-and-Hashing.md#what-an-operator-calls)):

* **std's own `eq` or `cmp` for a basic type is written inline.** The basic
  types are `Byte`, `Int`, `Long`, `Float`, `Double`, `Bool` and `Char`
  ([Data and Types](Data-and-Types.md)), and their `eq` and `cmp` *are* the
  host's operators, so `i < 4` is `(read i < 4)` and `i != 2` is
  `(read i != 2)`. A `Float` or `Double` ordering, which has no `cmp`, is
  inline too. The intrinsics also appear as values, where a scalar's ordering
  is passed to generic code:
  `call bigger@main<Int>(2, 7, fn cmp(Int, Int)@std.core.basic)`.
* **Anything else is a call** of the function the checker resolved, whether
  it is declared in Salvo or is an intrinsic: `a == b` on `Str`s is
  `call eq@std.core.string(read a, read b)`, and `a != b` is `!` of that
  call, `!(call eq@std.core.string(read a, read b))`. An ordering is the
  `cmp` call followed by a sign test of its `Int` result, which is inline
  because `Int` is basic. An enclosing implicit takes the operator on any
  type, a basic one included, and an effect's member is a member call:

  ```text
  (call cmp@std.core.string(read s, "c") < 0)
  (call cmp@main(read p, read q) < 0)
  (call local cmp(read a, read b) > 0)     // a forwarded ?cmp, on a T or an Int
  ```

Arithmetic is the call it names in the same way: `a + b` on two numbers is
std's `plus`, so it is inline (`(read a + read b)`), and on anything else it
is the call: a struct's `plus`, a forwarded `?plus`, an effect's member.
`**` is always a call, `pow`, since no target has a power operator; `%` names
no function and is always inline:

```text
call plus@main(read v, read w)
call local plus(read a, read b)          // a forwarded ?plus
call pow(Int, Int)@std.core.basic(2, 10)
```

## Control flow

**`if`** is a `branch`. A `when` with no subject is a `branch` too.

### `is` and narrowing

An `if x is Int` tests with a `check`, and the branch begins with a `narrow`
that names it (the [overview](#overview)'s `show`):

```text
branch#1 cond#0 check#2 (read x is arm 0) {
  narrow#3[check#2] x~1: Int = x
  return concat("int ", call to_str(Int)@std.core.basic(read x~1))
}
```

Inside the branch, `x` is read as `x~1`, an `Int`. Each backend binds the
narrowed value to a typed local where the test held: Kotlin takes the arm's
payload out of the union wrapper, and Rust unwraps the enum variant. The
reason in brackets is one of:

| Reason | Proof |
|---|---|
| `check#n` | the `check` node held |
| `switch#n.arm#i` | inside arm `i` of a `switch` |
| `switch#n.?.` | inside the present arm of the `switch` a `?.` makes (a backend may write it as its own optional chain) |
| `branch#n.cond#i` | inside a branch whose condition tested it |
| `after branch#n.cond#i` | after a branch that left (returned, broke or threw), so the test failed |
| `loop#n value` | a loop assigned its value on every way out |
| `claim` | a qualifier the value carries proves it |

### `when`, `?:` and `?.`

**`when x`** becomes a `switch`, with a `narrow` at the top of each arm that
binds a value. An exhaustive `when` gets an `else` arm anyway, a trap, so the
case no arm can reach is in the IR rather than worked out by a backend:

```text
return switch#1 read x {
  arm#0 (arm 0) {
    narrow#2[switch#1.arm#0] x~1: Int = x
    value "int"
  }
  arm#1 (arm 1) {
    narrow#3[switch#1.arm#1] x~2: Str = x
    value "str"
  }
  arm#2 (none) {
    value "none"
  }
  arm#3 (else) {
    unreachable("unhandled union arm") at main:25:12
  }
}
```

**`?:`** is a `switch` too: its first arm is the subject's present arm
(arm 0), narrowed, and `else` takes the right-hand side. A subject that is
not a variable is bound to a temporary first. Here `name_of(i) ?:
unreachable!()` binds the call's result, narrows it on the present side,
and traps on the other:

```text
return branch#4 cond#0 true {
  bind#2 let __pick1: Str? = call name_of@main(read i)
  value switch#1 read __pick1 {
    arm#0 (arm 0) {
      narrow#3[switch#1.arm#0] __pick1~1: Str = __pick1
      value !read __pick1~1
    }
    arm#1 (else) {
      value unreachable() at main:21:26
    }
  }
}
```

With a variable as the subject (`name ?: unreachable!()`), there is no
temporary: the dump is the `switch#1 read name { … }` alone.

**`?.`** is the same shape, with the `?.` reason on its narrowing and the
absent arm answering `none`. `p?.v`, with `p: P?`:

```text
return branch#4 cond#0 true {
  bind#2 let __nav1: P@main? = read p
  value switch#1 read __nav1 {
    arm#0 (arm 0) {
      narrow#3[switch#1.?.] __nav1~1: P@main = __nav1
      value present(read __nav1~1.v)
    }
    arm#1 (else) {
      value none[None]
    }
  }
}
```

### Blocks in value position

**`branch … cond#0 true { … }`** is a block in value position: statements
that set up a value, then the value. The IR has no separate block
expression, so a `?:` or a `?.` that needs a temporary is wrapped this
way, as in both samples above. Both backends write this exact shape as a
plain block (Rust's `{ … }`, Kotlin's `run { … }` where a value is needed),
not as an `if (true)`.

### Loops

The IR has one general loop, `loop`, with no condition, and one special
case, `foreach`.

* `while c { … }` is a `loop` that begins by breaking when `c` fails:

  ```text
  loop#2 {
    branch#3 cond#0 !(read at > 0) {
      break
    }
    assign at = (read at - member now@Clock(read clock))
  }
  ```

* `for` over a list, an array or a `Str` is a `foreach`, which each backend
  writes as its own native loop.
* `for` over an iterator is a `loop` that calls `next` and switches on the
  step. The iterator is minted first:

  ```text
  bind#4 let __pass~3: Mut Halving@main = call halving@main(8)
  loop#3 {
    bind#5 let __step~4: Int | Finished = call next@main(read __pass~3)
    switch#6 read __step~4 {
      arm#0 (arm 0) {
        narrow#7[switch#6.arm#0] __emitted~5: Int = __step~4
        bind#8 let x: Int = !read __emitted~5
        …
      }
      arm#1 (else) {
        break
      }
    }
  }
  ```

  An `iter fn` reaches the IR already rewritten as a struct, a function that
  builds it, and a `next` ([Iterators](Iterators.md)).
* A loop with a value or an `else` gets explicit locals for the value and
  for whether the loop ran, and a `branch` after it, only when the program
  uses them. A `while` with a value and an `else`:

  ```text
  bind#2 let __loop~1: Int? = none[Int]
  bind#3 let __ran~2: Bool = false
  loop#4 {
    …
    assign __ran~2 = true
    assign __loop~1 = read numbers[read i]
  }
  branch#7 cond#0 !(read __ran~2) {
    assign __loop~1 = -(1)
  }
  narrow#8[loop#4 value] __loop_value~3: Int = __loop~1
  ```

Both backends print a `loop` as their own unconditional loop with the
`break` inside it (Rust `loop { if !(c) { break; } … }`, Kotlin
`while (true)`).

## Effects and handlers

A function's effects are its leading parameters, before the `;`:

```text
fn countdown@main(lent clock: Clock@main; lent from: Int) -> Int
```

A call to a function that needs an effect passes the instance the checker
chose, and a call of an effect member goes through it:
`member now@Clock(read clock)` is the member `now` of `Clock`. A
`use` becomes a `construct` of the handler and a binding of the instance. A
stateful handler is wrapped in a `handle`, which lets only one member call in
at a time:

```text
bind#3 let __use~3: TickingClock@main = construct TickingClock@main { }
bind#4 let __handle~4: Clock@main = handle[Clock@main](read __use~3)
```

A stateless one, such as `StdConsole`, is bound as it is:
`bind#2 let __handle~2: Console = read __use~1`.

A handler is an `impl`. Its state fields are `state` lines, and its members
are ordinary function bodies:

```text
impl TickingClock() for Clock@main stateful {
  state tick: Int = 0
  fn now() -> Int {
    assign tick = (read tick + 5)
    return !read tick
  }
}
```

## Actors

`spawn`, a send, `waitfor` and `replyto` are nodes of their own
([Concurrency](Concurrency.md)). A `spawn` lists the pool after `on:` and,
for `spawn … in group`, the group after `in:`; `serves` names the effects
the actor answers:

```text
bind#4 let c: eff[Counter@main] = spawn[eff[Counter@main]](construct Tally@main { }, on: read workers) serves [Counter@main]
send bump@Counter(read c, 2)
bind#5 let t: Int = waitfor r: Reply<Int> {
  obligation gain [r]
  obligation release [r]
  send total@Counter(read c, !read r)
}
```

(A `Reply` is linear, hence the `obligation` lines; see
[Statements](#statements).) Inside an actor, `self_addr` is its own address
and `self_send m(args)` a message to itself. `instance_at(addr)` is the
effect behind an address, every member a send: what `use addr` binds.

For each actor effect and handler, the IR also contains declarations no one
wrote:

* **`enum __Msg_E`**: one variant per `send` member of the effect, holding its
  arguments. A message enum with a wire form carries its protocol hash,
  `[proto …]`, which two nodes compare before they talk:

  ```text
  enum __Msg_Counter [proto f39d50f9ee8f9923] {
    Bump(by: Int)
    Total(out: Reply<Int>)
  }
  ```

* **`enum __Priv_H`** and **`enum __Cont_H`**: a handler's private messages
  (its `init`, and `send fn`s no effect declares), and the continuations of
  members that wait for a reply.
* **`fn __dispatch_H_E`**: delivers one message to the handler. It is a
  `switch` over the message enum, an `unpack` of the payload, and a
  `handler_call` of the member, a call on the handler instance itself rather
  than through a handle:

  ```text
  fn __dispatch_Tally_Counter(lent_mut __handler: Tally@main, moved __msg: __Msg_Counter@main) -> None {
    switch#1 read __msg {
      arm#0 (arm 0) {
        unpack#2 (by: Int) = __msg as variant 0
        handler_call bump@Counter(read __handler, !read by)
      }
      …
    }
  }
  ```

* **`impl __Stub_E(moved addr: eff[E]) for E`**: the effect implemented by
  sending each member to an address. This is what `use addr` binds.

The wire codecs, the scheduler's parking of a member that waits, and the
glue between a handler and its mailbox are not in the IR. Each backend
generates them.

## What a backend adds

Everything in the IR is a fact about the Salvo program. A backend adds how its
target language says it:

* the representation of unions and optionals (Rust enums, Kotlin sealed
  classes),
* imports and names, worked out from the references,
* the host's rendering of an intrinsic call, such as `eq(Str, Str)` as a
  string comparison,
* copies where the target needs them (Rust adds `.clone()` for a read that
  does not consume a value it only borrows; the IR records moves, never
  copies),
* evaluation-order temporaries where the target's borrow rules need them.

The details per target are in `BACKEND_SPEC.kotlin.md` and
`BACKEND_SPEC.rust.md`.
