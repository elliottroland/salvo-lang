# The IR

Between the checker and the backends, the compiler rewrites a program into
its **IR** (intermediate representation): a small language in which every
question the checker answered is written down. Both backends, Kotlin and Rust,
read the IR and nothing else. This page explains how to read it. You need it
when a backend's output surprises you, when the two backends disagree, or when
you work on the compiler.

`salvo ir` prints it:

```bash
salvo ir --src ./my_project                  # every module of the project
salvo ir --src ./my_project --module main    # one module
salvo ir --src ./my_project --all            # with the std modules it reaches
```

The printed form is for reading and for golden tests. Nothing parses it back.
The Rust types behind it are in `crates/salvo-ir/src/ir.rs`.

## What the IR decides

The surface language leaves a lot for the compiler to work out: which
overload a call means, which handler a member call goes to, what a variable's
type is after `is`, whether a value moves or is borrowed, which union arm a
value goes into. The checker works all of that out. The IR is where those
answers are stored, as fields of the nodes that need them. A backend never
infers, and when it meets a node it cannot render it reports an error rather
than guessing [backend-never-wrong].

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
is a small program, and below it what `salvo ir` prints for it.

```
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
    use StdOutConsole
    let b = Mut Box { n: 1, tag: "t" }
    println("${bumped(b)} ${b.tag}")
    let xs = [3, 2, 1]
    println("${total(xs)}")
}
```

```
module main

struct Box canbe Mut wire {
  n: Int
  tag: Str
}

fn main::bumped(lent_mut b: Mut Box) -> Int {
  assign b.n = op Add(read b.n, 1)
  return read b.n
}

fn main::total(moved items: List<Int>) -> Int {
  bind#1 let sum: Int = 0
  foreach#2 i: Int in read items {
    assign sum = op Add(read sum, read i)
  }
  return !read sum
}

fn main::main() -> None {
  bind#1 let __use~1: StdOutConsole = construct StdOutConsole { }
  bind#2 let __handle~2: Console = read __use~1
  bind#5 let b: Mut Box = construct Mut Box { n: 1, tag: "t" }
  call core.console::println(read __handle~2, concat(call core.basic::to_str(Int)(call main::bumped(read b)), " ", read b.tag))
  bind#6 let xs: List<Int> = list[List<Int>](3, 2, 1)
  call core.console::println(read __handle~2, concat(call core.basic::to_str(Int)(call main::total(!read xs))))
}
```

The things to notice:

* **References are `module::name`**, with the parameter types added when the
  module overloads the name: `core.basic::to_str(Int)`. A declaration with
  no recorded name prints as its module and position, such as `main::#4.0`.
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
  a call to the `to_str` the checker chose for it.
* **The `wire` flag** on `Box` means every field has a wire form, so the
  struct can travel between actors on different nodes.

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

A union's arms are numbered in order, skipping the `None` arm
[union-arm-identity]. `make_union[Int | Str | None]#0(4)` puts `4` into arm 0.
An optional `T?` is the union `T | None`.

## Declarations

| Dump | What it is |
|---|---|
| `struct Name<T> canbe Mut wire { field: Ty }` | a struct. It may also be `export`, `opaque` or `linear`, and a field may be `canbe_mut` or have a default |
| `union Name = A \| B` | a named union type |
| `interface Name [Prereq] { fn m(…) -> T }` | an effect. An `actor interface` is an actor effect, and its `send fn` members are messages |
| `impl H(ctor params) of Face deps [Dep] stateful { … }` | a handler: constructor parameters, the effects it depends on, its `state` fields, an optional `mailbox`, `init` and member bodies. It may also be `platform`, `threadsafe` or `intrinsic` |
| `fn module::name<T>(effects; params; ?implicits) -> T` | a function. `;` separates the leading effect parameters, the declared ones and the trailing implicit ones |
| `intrinsic fn …`, `platform fn …` | a function with no body. A backend lowers an intrinsic from its own table, and the host implements a platform function ([Backends](Backends.md)) |
| `qualifies fn …` | the test of a predicate qualifier |
| `platform type Name` | a type the host defines |
| `static name: Ty { … }` | a module-level `use`: an instance built once, which every function in the module reads |
| `enum Name { Variant(field: Ty) }` | a generated enum with named variants (see "Actors" below) |

After the return type, a function signature may list:

* **`borrows [p]`**: the result holds a view of parameter `p` (a projection,
  `proj`).
* **`holds [p <- q]`**: after the call, parameter `p` holds a view of `q`
  (the deduction `p.f: proj(q)`).
* **`throws M`**: the function may throw a message of type `M`
  ([Throwing](Throwing.md)).
* **`=> a canbe b`, `=> a canbe in lib.tracks`**: the parameters that may
  name the same object ([Mutable Handles](Mutable-Handles.md)).

## Statements

A block is a list of statements, optionally followed by `value e`, the
block's value.

| Dump | Meaning |
|---|---|
| `bind#n let x: T = e` | a new local |
| `assign place = e` | a write to a local, a field or an element |
| `narrow#n[reason] x~1: T = x` | a new local holding `x` at the narrower type `T`, justified by `reason` |
| `alias#n x: T = place` | a name for a place. Every read and write of `x` is one of the place |
| `unpack#n (a: A, b: B) = m as variant i` | the payload of an enum variant, in the arm that tested for it |
| `loop#n { … }` | a loop with no condition, left by `break` |
| `foreach#n x: T in e { … }` | a loop over a list, an array or a `Str` |
| `return e`, `break`, `continue` | as in Salvo |
| an expression | evaluated for its effect |

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
| `member E#i(instance, args)` | a call of member `i` of effect `E`, through an instance |
| `op Add(a, b)` | the host's operator on basic types: `Add`, `Sub`, `Mul`, `Div`, `Rem`, `And`, `Or`, `Not`, `Neg`, `Lt`, `Gt`, `LtEq`, `GtEq`, `Eq`, `NotEq` (see "Comparisons and operators") |
| `construct T { f: e }` | a struct or handler value |
| `make_union[U]#i(e)` | `e` placed into arm `i` of the union `U` |
| `present(e)` | `e` placed into the present arm of an optional. Rust writes `Some(e)`, and Kotlin writes `e` |
| `rewrap[A -> B](e)` | a union value moved to another union's arm numbering |
| `widen[Long](e)` | a number widened within its class (`Int` to `Long`, `Float` to `Double`) |
| `drop_mut(e)` | a `Mut` value used where the plain type is needed |
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

A `switch` arm's test is one of: `arm i`, `arms [i, j]`, `none`, `else`, or
`lit` for an arm limited to some literal values (a `"GET" | "PUT"` collapsed
into one `Str` arm).

## Comparisons and operators

The rule turns on the **basic types**: `Byte`, `Int`, `Long`, `Float`,
`Double`, `Bool` and `Char` ([Data and Types](Data-and-Types.md)).

* **On basic types, an operator is an `op`.** `i < 4` is `op Lt(read i, 4)`
  and `i != 2` is `op NotEq(read i, 2)`. No basic-type operator goes through
  `cmp` or `eq`. Those intrinsics exist only so a scalar's ordering can be
  passed to generic code, where they appear as a value:
  `call main::bigger<Int>(2, 7, fn core.compare::cmp(Int, Int))`.
* **On every other type, `Str` included, an operator is a call** of the
  function the checker resolved, whether it is declared in Salvo or is an
  intrinsic. `a == b` is `call core.compare::eq(Str, Str)(read a, read b)`,
  and `a != b` is `op Not` of that call. An ordering is the `cmp` call
  followed by a sign test of its `Int` result, which is an `op` because `Int`
  is basic:

  ```
  op Lt(call core.compare::cmp(Str, Str)(read s, "c"), 0)
  op Lt(call main::cmp(read p, read q), 0)
  op Gt(call local cmp(read a, read b), 0)     // a forwarded ?cmp
  ```

Arithmetic (`+`, `-`, `*`, `/`, `%`) is defined on numbers only, so it is
always an `op`.

## Control flow

**`if`** is a `branch`. A `when` with no subject is a `branch` too.

**`is` and narrowing.** An `if x is Int` tests with a `check`, and the branch
begins with a `narrow` that names it:

```
branch#1 cond#0 check#2 (read x is arm 0) {
  narrow#3[check#2] x~1: Int = x
  return concat("int ", call core.basic::to_str(Int)(read x~1))
}
```

Inside the branch, `x` is read as `x~1`, an `Int`. Each backend binds the
narrowed value to a typed local where the test held: Kotlin takes the arm's
payload out of the union wrapper, and Rust unwraps the enum variant. The reason in brackets is one of:

| Reason | Proof |
|---|---|
| `check#n` | the `check` node held |
| `switch#n.arm#i` | inside arm `i` of a `switch` |
| `branch#n.cond#i` | inside a branch whose condition tested it |
| `after branch#n.cond#i` | after a branch that left (returned, broke or threw), so the test failed |
| `loop#n value` | a loop assigned its value on every way out |
| `claim` | a qualifier the value carries proves it |

**`when x`, `?:` and `!`** become a `switch`. `name!` is a switch whose `none`
arm traps:

```
switch#1 read __nn~1 {
  arm#0 (none) {
    unreachable("value is absent") at main:14:12
  }
  arm#1 (else) {
    narrow#3[switch#1.arm#1] __some~2: Str = __nn~1
    value !read __some~2
  }
}
```

**`branch … cond#0 true { … }`** is a block in value position: statements
that set up a value, then the value. The IR has no separate block
expression, so a `?:`, a `?.` or a `!` that needs a temporary is wrapped this
way:

```
return branch#4 cond#0 true {
  bind#1 let __safe~1: P? = read p
  value switch#2 read __safe~1 { … }
}
```

**Loops.** The IR has one general loop, `loop`, with no condition, and one
special case, `foreach`.

* `while c { … }` is a `loop` that begins by breaking when `c` fails:

  ```
  loop#2 {
    branch#3 cond#0 op Not(op Gt(read at, 0)) {
      break
    }
    assign at = op Sub(read at, member main::Clock#0(read clock))
  }
  ```

* `for` over a list, an array or a `Str` is a `foreach`, which each backend
  writes as its own native loop.
* `for` over an iterator is a `loop` that calls `next` and switches on the
  step. The iterator is minted first:

  ```
  bind#4 let __pass~3: Mut __Iter_halving_Int = call main::halving(8)
  loop#3 {
    bind#5 let __step~4: Int | Finished = call main::next(read __pass~3)
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
  uses them.

Both backends print a `loop` as their own unconditional loop with the
`break` inside it (Rust `loop { if !(c) { break; } … }`, Kotlin
`while (true)`).

## Effects and handlers

A function's effects are its leading parameters, before the `;`:

```
fn main::countdown(lent clock: Clock; lent from: Int) -> Int
```

A call to a function that needs an effect passes the instance the checker
chose, and a call of an effect member goes through it:
`member main::Clock#0(read clock)` is member 0 (`now`) of `Clock`. A
`use` becomes a `construct` of the handler and a binding of the instance. A
stateful handler is wrapped in a `handle`, which lets only one member call in
at a time:

```
bind#3 let __use~3: TickingClock = construct TickingClock { }
bind#4 let __handle~4: Clock = handle[Clock](read __use~3)
```

A handler is an `impl`. Its state fields are `state` lines, and its members
are ordinary function bodies:

```
impl TickingClock() of Clock stateful {
  state tick: Int = 0
  fn now() -> Int {
    assign tick = op Add(read tick, 5)
    return !read tick
  }
}
```

## Actors

`spawn`, a send, `waitfor` and `replyto` are nodes of their own
([Concurrency](Concurrency.md)):

```
bind#3 let c: Addr<Counter> = spawn[Addr<Counter>](construct Tally { }) serves [Counter]
send main::Counter#0(read c, 2)
bind#4 let t: Int = waitfor r: Reply<Int> {
  send main::Counter#1(read c, !read r)
}
```

For each actor effect and handler, the IR also contains declarations no one
wrote:

* **`enum __Msg_E`**: one variant per `send` member of the effect, holding its
  arguments. A message enum with a wire form carries its protocol hash,
  `[proto …]`, which two nodes compare before they talk.
* **`enum __Priv_H`** and **`enum __Cont_H`**: a handler's private messages
  (its `init`, and `send fn`s no effect declares), and the continuations of
  members that wait for a reply.
* **`fn __dispatch_H_E`**: delivers one message to the handler. It is a
  `switch` over the message enum, an `unpack` of the payload, and a
  `handler_call` of the member:

  ```
  fn __dispatch_Tally_Counter(lent_mut __handler: Tally, moved __msg: __Msg_Counter) -> None {
    switch#1 read __msg {
      arm#0 (arm 0) {
        unpack#2 (by: Int) = __msg as variant 0
        handler_call main::Counter#0(read __handler, !read by)
      }
      …
    }
  }
  ```

* **`impl __Stub_E(moved addr: Addr<E>) of E`**: the effect implemented by
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
