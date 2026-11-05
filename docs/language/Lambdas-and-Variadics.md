# Lambdas and variadic arguments

## Overview

Functions are values in Salvo. A parameter can have a **function type**,
written like a signature without a name, `(S) -> T`, and the function is
called like any other. The caller passes either an existing function, by
name, or a **lambda**, an anonymous function written in place.

This is what the standard library's `map` does, spelled out, with `mapper`
as the function-typed parameter:

```salvo
fn transform<S, T>(list: List<S>, mapper: (S) -> T) -> List<T> {
    let result: Mut List<T> = Mut List()
    for s in list {
        // Call `mapper` like a normal function
        add(result, mapper(s))
    }
    // The return does not expose the `Mut` qualifier we know about internally
    return result
}
```

It can be called by passing it an existing function, or by passing it an
anonymous lambda:

```salvo
fn to_string(int: Int) -> Str {
    return "${int}"
}

fn do_something() {
    let list: List<Int> = List(1, 2, 3)

    // The following are all equivalent
    transform(list, to_string)                      // Pass the function by name
    transform(list, (i) -> "${i}")                  // A lambda without {}: no `return` needed
    transform(list, (i) -> Str { return "${i}" })   // A lambda with {}: it needs a `return`
}
```

A function type can say more than its parameters and result. Like a
declared function, it may list the **effects** calling it performs, and a
function generic over *which* effects its callback performs uses an
**effect variable**, `[E]`. Both are explained below
([Effects on function types](#effects-on-function-types),
[Effect variables](#effect-variables-e)). The page ends with
**variadic arguments**, `...xs: T[]`, which collect the remaining arguments
of a call into an array ([Variadic arguments](#variadic-arguments)).

## Lambdas

A lambda's parameters are always in parentheses, `(i) -> …`, even when there
is one. A lambda with an expression body answers that expression. A lambda
with a block body answers the type written before the block, or else the
return type of the function type it is passed as (or assigned to), or else
`None`, exactly as a function with no `->` returns `None`:

```salvo
let double = (n: Int) -> Int { return n * 2 }   // (n: Int) -> Int
label_all(list, (i) -> { return "${i}" })       // `f: (Int) -> Str` says `Str`
let say = (s: Str) -> { println(s) }            // (s: Str) -> None
let wrong = (n: Int) -> { return n * 2 }        // error: returns an `Int` where `None` is expected
```

Here `label_all` stands for any function with a parameter `f: (Int) ->
Str`. When the function type's return is a type parameter the call is still
inferring, as `transform`'s `T` above, it says nothing, and a block-bodied
lambda writes its type (`(i) -> Str { … }`).

A type before a brace is the return type unless the brace opens a struct
literal, so `(n: Int) -> Point { x: n }` still answers a `Point` built from
`n`; a fieldless struct literal is written in parentheses, `() ->
(Finished {})`.

### Effects and deductions on a lambda

Lambdas can declare effects ([Effects and handlers](Effects-and-Handlers.md))
and deductions ([Deductions and ownership](Deductions-and-Ownership.md)),
just like named functions. Both go between the parameters and `->`, effects
first:

```salvo
let say = (s: Str) [Console] -> println(s)      // (s: Str) [Console] -> None: callers supply Console
let consume = (b: Box) => !b -> eat(b)          // calling it consumes the argument
let keep = (b: Box) => b -> b.v                 // `b` is the caller's: the body may not consume it
let both = (b: Box) [Console] => !b -> { println(eat(b)) }
```

Both are optional:

* **Effects.** Without a written list, a lambda performs what the function
  type it is passed as declares, or else what its body calls (see
  [Effects on function types](#effects-on-function-types)). A written list
  is the set its body may perform, and where a function type is expected it
  may name fewer effects than the type but not more.
* **Deductions.** A lambda's clause says only which parameters it consumes
  (`!p`) and which it keeps (`p`, the default, so a parameter the clause
  leaves out is kept too). A qualifier entry such as `xs: -Sorted` is
  written on a declared function. A lambda that consumes a parameter does
  not fit a function type that keeps it, for the same reason a function
  that does is refused there (below).

### Which functions fit a function type

A function passed where a function type is expected must take the parameter
types it names, with one allowance: it may ask *less* of an argument by a
qualifier that is only a compile-time claim, such as `NonEmpty`, `Sorted`,
`Ok` or any qualifier a program declares:

```salvo
fn render_row(xs: List<Int>) -> Str => xs { return "${size(xs)} items" }

fn apply(f: (xs: NonEmpty List<Int>) -> Str, xs: NonEmpty List<Int>) -> Str => f, xs {
    return f(xs)
}

apply(render_row, List(1, 2))        // fine: `render_row` reads any list
to_str(xss, to_str = render_row)     // fine, and the same rule for an implicit
```

A second allowance is about borrows. A function whose parameter is an owned
type it **keeps** (the default; see
[Deductions and ownership](Deductions-and-Ownership.md)) fits a slot that
hands it a `proj` of that type: it only reads its argument, and a borrowed
value can be read. So a function over strings reads the elements of a view
list:

```salvo
fn longer(s: Str) -> Bool { return size(s) > 2 }

let vs = filter(iter(ws), (s) -> size(s) > 1)   // a Mut List<proj Str>
any(vs, longer)                                 // fine: `longer` keeps `s`
let short = filter(vs, longer)
let m = to_map(vs, (v) -> (v, size(v)))         // std's `hash` and `eq` of `Str` read the keys
```

A function that **consumes** its parameter (`=> !s`) does not fit: it would
give away a value it was only lent, and the error says so. Pass a lambda
that copies (`(s) -> eat(copy(s))`) if that is what you mean.

A qualifier that changes how the argument is passed is different: a
`count(xs: List<Int>)` does not fit `(xs: Mut List<Int>) -> Int`, and a
function reading a list does not fit a slot handing it a `proj Mut` one.
Wrap it in a lambda (`(xs) -> count(xs)`) where you mean it. Both
allowances apply to the parameter's outer qualifiers only, not to those
inside a type argument (`List<NonEmpty List<Int>>`, `List<proj Str>`) or on
a union's arms.

## Effects on function types

A function *value* that performs an effect says so in its type, and the
effect is supplied by whoever **calls** the value:

```salvo
// `f` may log; `run_it` gets `[Logger]` for free: the only reason to take
// `f` is to call it, and calling it needs Logger here.
fn run_it(f: (s: Str) [Logger] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn demo() [Console, Logger] {
    println(run_it((s) -> {
        log("in lambda ${s}")      // legal: the fn type declares Logger
        return "done ${s}"
    }, "x"))
}
```

The rules follow from "the caller supplies it":

- **A lambda body performs what its type declares**, not whatever its
  enclosing scope happens to have. A lambda passed where the fn type
  declares nothing may not log, even inside a function that can.
- **A function that writes no effect list inherits its fn-typed parameters'
  effects.** `run_it` above needs no effect list of its own, but its
  *callers* must have `Logger` available, because that is where the value
  comes from at run time. A written list is the whole set, `[]` included: a
  function that handles the effect itself writes the list without it, which
  is how a `try` around a throwing parameter's call keeps the throw from its
  callers:

  ```salvo
  fn attempt(f: (n: Int) [Throw<Str>] -> Int, n: Int) [] -> Str => f {
      let r = try { f(n) }
      when r {
          is Ok { return "ok ${r}" }
          is Thrown { return "failed: ${r}" }
      }
  }
  ```

- **Fewer effects fit where more are expected.** A pure function passes
  wherever an effectful one is expected: it is handed the effect and ignores
  it. The reverse is an error, since the value would reach a call site that
  cannot supply it.
- **An un-annotated lambda's effects are inferred** from its body, so `let
  f = () -> { println("hi") }` has type `() [Console] -> None` and does not
  silently fit a pure position. A lambda that writes its list, `() [Console]
  -> …`, has that set instead.
- **A function value that may throw** (`[Throw<M>]` in its type) throws to
  the `try` around its call, or out of a function that declares the throw,
  as a call of a named function does ([Throwing](Throwing.md)).
- **A function value carries no capability**, so storing or returning one
  is fine; what needs the effect is *calling* it. A stored effectful value
  called where its effects are unavailable is an error at the call.
- **`use` cannot appear in a function type**: registering a handler is local
  to a body, so a lambda may `use` exactly when the function containing it
  may.

## Effect variables: `[E]`

Naming the effects a function value performs works when the function knows
them. A function that only *calls* the value it is given, such as `map` or
`drain`, does not care what they are, and naming any set would make it
useless for every other. A generic written in brackets, `[E]`, is an
**effect variable**: it ranges over a *set* of effects, and a call binds it
to whatever the value it is handed performs.

```salvo
fn each<T, [E]>(xs: List<T>, f: (x: T) [E] -> None) => xs, f {
    for x in xs {
        f(x)
    }
}

fn main() [use] {
    use StdConsole()
    each(List(1, 2), (x) -> println("${x}"))   // E is [Console]
    each(List(1, 2), (x) -> {})                // E is [], nothing is needed
}
```

How a variable is bound and used:

- **`each` inherits `[E]`** from `f`, as it would inherit a named effect, so
  its callers supply whatever `E` stands for.
- **A call binds `E`** to what the value performs beyond the effects the
  parameter's type names itself: the effects a lambda's body uses, or the
  list a named function declares. Two parameters sharing `E` bind what
  either performs, and each value fits by the rule above.
- **Inside the body `E` is opaque.** The function can pass it on, to `f` or
  to another function taking a variable, but cannot call a member of it,
  handle it with `try`, or name it in a written list that leaves it out.
  `[E, Console]` is a variable beside a named effect: the function performs
  `Console` itself and hands `E` on.
- **Its handle is `eff[E]`.** A parameter `fx: eff[E]` is bound like a
  function value's effects are, and `use fx` makes the effects `E` stands
  for available in the body, which is how a function can hold a capability
  it cannot name. `eff[]` is the handle over no effects, as `()` is the
  tuple of no values.
- **An actor's address passed as `eff[E]` becomes its instance in `E`'s
  pack**, so `later(counter, () -> bump(2))` works. What does not work yet
  is the way back: a function returning `eff[E]`, or a struct's `eff[E]`
  field read, where `E` holds an actor effect, is refused, since nothing
  turns the instance back into an address.

What a variable may not be:

- **A variable is a set of effects, not a type of values.** `x: E` is an
  error, and so is a plain type parameter in an effect list: `[T]` must be
  declared `<[T]>` to mean one. `any E` is refused too: each effect a
  variable stands for keeps its own identity strength.
- **One variable per function type.** `(x: T) [E, F] -> None` is refused,
  since a call could not tell which of the value's effects belong to which
  variable.
- **A throw is not an effect a variable carries.** A function value that may
  throw cannot bind `E`; a parameter whose type lists `Throw<…>` itself
  takes one, and the function either catches it with `try` or passes it on
  in its own list. The message may be generic, `f: () [E, Throw<X>] ->
  None`: a lambda's throws bind `X`, and one that throws nothing binds it to
  `Never`.

### Effect variables on structs and groups

- **A struct may have one too**, for a function it stores: `struct Later<[E]>
  { run: () [E] -> None }`. Its type names what the variable is bound to as
  a list, `Later<[Console]>`, `Later<[]>`, or `Later<[E]>` inside a function
  generic over `E`, and a function that calls the stored value declares
  `[E]` itself, since it has no fn-typed parameter to inherit it from:

  ```salvo
  fn fire<[E]>(l: Later<[E]>) [E] => l {
      let f = l.run
      f()
  }
  ```

- **So may a `fns` group**, for what its members perform: `fns Next<It, T,
  [E]> { fn next(it: Mut It) [E] -> … }`. A spread names the function's own
  variable, `?Next<It, T, [E]>`, so the members' effects are the caller's,
  inherited as a fn-typed parameter's are
  ([Implicit parameters](Implicit-Parameters.md#effects-in-a-group)).
- **A trailing effect variable may be left out, and then it is `[]`**, in
  any list of generics: `Later` is `Later<[]>`, and `?Next<It, T>` asks for
  an iterator whose `next` performs nothing.

### Effect variables in std

std's functions that take callbacks have an effect variable, so a callback
passed to them may log, count or print:

* `map`, `reduce`, `filter` over an iterator, and the lazy adaptors
  (`mapping`, `filtering`, `taking_while`, `skipping_while`), whose callbacks
  run as the adaptor is driven ([std.core.seq](../../lib/std/core/seq.sv));
* `any`, `all`, `count`, `find_first`, `find_last`, `partition`,
  `remove_front_while`, `remove_back_while`, `update`, `drain`, and the `_by`
  builders written in Salvo (`list_by`, `deque_by`)
  ([std.core.list](../../lib/std/core/list.sv),
  [std.core.deque](../../lib/std/core/deque.sv));
* `to_map` ([std.core.map](../../lib/std/core/map.sv)) and `map2`
  ([std.core.result](../../lib/std/core/result.sv));
* the ones std implements on the platform (`filter` over a list, `set_by`,
  `map_by`): the host takes the callback pure, and std's wrapper hands it
  one that passes the pack. `array_by` takes one as well.

The iterator functions share their variable with the iterator's `next`
(`?Next<It, T, [E]>`), so the iterator may perform effects too:
`to_list(mapping(lines, f))` drives a stream's lines through an `f` that
logs.

## Variadic arguments

A function can take a variable number of arguments. The variadic parameter
is written `...name: T[]`, and inside the function it is an array of the
remaining arguments:

```salvo
fn max(...ints: Int[]) -> Int? {
    let max: Int? = None
    for i in ints {
        if max is None || i > max {
            max = i
        }
    }
    return max
}
```

Variadic arguments are assigned the remainder left after the non-variadic
arguments are assigned. For example, we might write the following overload
for the above function:

```salvo
// Guaranteed to have a maximum, because the first argument is given
fn max(first: Int, ...rest: Int[]) -> Int {
    // `...rest` spreads the array into the call. A spread can only supply a
    // variadic tail, never the required `first`, so this calls the overload above.
    let rest_max: Int? = max(...rest)
    if rest_max is Int && rest_max > first {
        // Because of the `is Int` check, `rest_max` is an `Int` in this block
        return rest_max
    }
    return first
}
```

A variadic parameter takes a deduction entry like any other. Kept (the
default, or `=> ints`), it lends its arguments to the call: a spread
`max(...xs)` leaves `xs` usable. Consumed (`=> !rest`), it moves them in, a
spread's array with them, so `List(a, ...xs)` spends `xs` just as it spends
`a`. Every standard-library constructor consumes its tail, which is what
lets a tail hold linear values (see [Linear types](Linear-Types.md)).

A variadic parameter does not take a default value
([Default parameter values](Functions.md#default-parameter-values)).
