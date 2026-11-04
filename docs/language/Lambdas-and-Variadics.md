# Lambdas and variadic arguments

## Lambdas

Functions can take lambdas as arguments, as `mapper` in the following example (this is what the standard library's `map` does, spelled out):

```
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

This can be called either by passing it an existing function, or by passing it an anonymous lambda:

```
fn to_string(int: Int) -> Str {
    return "${int}"
}

fn do_something() {
    let list: List<Int> = List(1, 2, 3)

    // The following are all equivalent
    transform(list, to_string) // Pass the function by name
    transform(list, (i) -> "${i}") // Use an anonymous lambda without {}, doesn't require a return
    transform(list, (i) -> { return "${i}"}) // Use an anonymous lambda with {}, does require a return
}
```

A lambda's parameters are always in parentheses, `(i) -> …`, even when there is one. A lambda with an expression body answers that expression. A lambda with a block body answers the type written before the block, or else the return type of the function type it is passed as (or assigned to), or else `None` — exactly as a function with no `->` returns `None`:

```
let double = (n: Int) -> Int { return n * 2 }   // (n: Int) -> Int
transform(list, (i) -> { return "${i}" })        // the parameter's type says `Str`
let say = (s: Str) -> { println(s) }             // (s: Str) -> None
let wrong = (n: Int) -> { return n * 2 }         // error: returns an `Int` where `None` is expected
```

A type before a brace is the return type unless the brace opens a struct literal, so `(n: Int) -> Point { x: n }` still answers a `Point` built from `n`; a fieldless struct literal is written in parentheses, `() -> (Finished {})`.

Lambdas can declare effects ([Effects and handlers](Effects-and-Handlers.md)) and deductions ([Deductions and ownership](Deductions-and-Ownership.md)), just like normal functions. Both go between the parameters and `->`, effects first:

```
let say = (s: Str) [Console] -> println(s)      // (s: Str) [Console] -> None: callers supply Console
let consume = (b: Box) => !b -> eat(b)          // calling it consumes the argument
let keep = (b: Box) => b -> b.v                 // `b` is the caller's: the body may not consume it
let both = (b: Box) [Console] => !b -> { println(eat(b)) }
```

Both are optional. Without a written list, a lambda performs what the function type it is passed as declares, or else what its body calls (see [Effects on function types](#effects-on-function-types)); a written list is the set its body may perform, and where a function type is expected it may name fewer effects than the type but not more. A lambda's clause says only which parameters it consumes (`!p`) and which it keeps (`p`, the default, so a parameter the clause leaves out is kept too); a qualifier entry such as `xs: -Sorted` is written on a declared function. A lambda that consumes a parameter does not fit a function type that keeps it, for the same reason a function that does is refused there (below).

### Which functions fit a function type

A function passed where a function type is expected must take the parameter types it names, with one allowance: it may ask *less* of an argument by a qualifier that is only a compile-time claim — `NonEmpty`, `Sorted`, `Ok`, any qualifier a program declares:

```
fn render_row(xs: List<Int>) -> Str => xs { return "${size(xs)} items" }

fn apply(f: (xs: NonEmpty List<Int>) -> Str, xs: NonEmpty List<Int>) -> Str => f, xs {
    return f(xs)
}

apply(render_row, List(1, 2))        // fine: `render_row` reads any list
to_str(xss, to_str = render_row)     // fine, and the same rule for an implicit
```

A second allowance is about borrows. A function whose parameter is an owned type it **keeps** (the default; see [Deductions and ownership](Deductions-and-Ownership.md)) fits a slot that hands it a `proj` of that type: it only reads its argument, and a borrowed value can be read. So a function over strings reads the elements of a view list:

```
fn longer(s: Str) -> Bool { return size(s) > 2 }

let vs = filter(iter(ws), (s) -> size(s) > 1)   // a Mut List<proj Str>
any(vs, longer)                              // fine: `longer` keeps `s`
let short = filter(vs, longer)
let m = to_map(vs, (v) -> (v, size(v)))        // std's `hash` and `eq` of `Str` read the keys
```

A function that **consumes** its parameter (`=> !s`) does not fit: it would give away a value it was only lent, and the error says so. Pass a lambda that copies (`(s) -> eat(copy(s))`) if that is what you mean.

A qualifier that changes how the argument is passed is different: a `count(xs: List<Int>)` does not fit `(xs: Mut List<Int>) -> Int`, and a function reading a list does not fit a slot handing it a `proj Mut` one. Wrap it in a lambda (`(xs) -> count(xs)`) where you mean it. Both allowances apply to the parameter's outer qualifiers only, not to those inside a type argument (`List<NonEmpty List<Int>>`, `List<proj Str>`) or on a union's arms.

### Effects on function types

A function *value* that performs an effect says so in its type, and the effect is supplied by whoever **calls** the value:

```
// `f` may log; `run_it` gets `[Logger]` for free — the only reason to take
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

- **A lambda body performs what its type declares** — not whatever its enclosing scope happens to have. A lambda passed where the fn type declares nothing may not log, even inside a function that can.
- **A function that writes no effect list inherits its fn-typed parameters' effects.** `run_it` above needs no effect list of its own, but its *callers* must have `Logger` available, because that is where the value comes from at run time. A written list is the whole set, `[]` included: a function that handles the effect itself writes the list without it, which is how a `try` around a throwing parameter's call keeps the throw from its callers:

```
fn attempt(f: (n: Int) [Throw<Str>] -> Int, n: Int) [] -> Str => f {
    let r = try { f(n) }
    when r {
        is Ok { return "ok ${r}" }
        is Thrown { return "failed: ${r}" }
    }
}
```
- **Fewer effects fit where more are expected.** A pure function passes wherever an effectful one is expected — it is handed the effect and ignores it. The reverse is an error: the value would reach a call site that cannot supply it.
- **An un-annotated lambda's effects are inferred** from its body, so `let f = () -> { println("hi") }` has type `() [Console] -> None` and does not silently fit a pure position. A lambda that writes its list, `() [Console] -> …`, has that set instead.
- **A function value that may throw** (`[Throw<M>]` in its type) throws to the `try` around its call, or out of a function that declares the throw, as a call of a named function does.
- **A function value carries no capability**, so storing or returning one is fine; what needs the effect is *calling* it. A stored effectful value called where its effects are unavailable is an error at the call.
- `use` cannot appear in a function type: registering a handler is local to a body, so a lambda may `use` exactly when the function containing it may.

### Effect variables: `[E]`

Naming the effects a function value performs works when the function knows them. A function that only *calls* the value it is given, such as `map` or `drain`, does not care what they are, and naming any set would make it useless for every other. A generic written in brackets, `[E]`, is an **effect variable**: it ranges over a *set* of effects, and a call binds it to whatever the value it is handed performs.

```
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

- **`each` inherits `[E]`** from `f`, as it would inherit a named effect, so its callers supply whatever `E` stands for.
- **A call binds `E`** to what the value performs beyond the effects the parameter's type names itself: the effects a lambda's body uses, or the list a named function declares. Two parameters sharing `E` bind what either performs, and each value fits by the rule above.
- **Inside the body `E` is opaque.** The function can pass it on, to `f` or to another function taking a variable, but cannot call a member of it, handle it with `try`, or name it in a written list that leaves it out. `[E, Console]` is a variable beside a named effect: the function performs `Console` itself and hands `E` on.
- **Its handle is `eff[E]`.** A parameter `fx: eff[E]` is bound like a function value's effects are, and `use fx` makes the effects `E` stands for available in the body, which is how a function can hold a capability it cannot name. `eff[]` is the handle over no effects, as `()` is the tuple of no values.
- **A variable is a set of effects, not a type of values.** `x: E` is an error, and so is a plain type parameter in an effect list: `[T]` must be declared `<[T]>` to mean one. `any E` is refused too: each effect a variable stands for keeps its own identity strength.
- **One variable per function type.** `(x: T) [E, F] -> None` is refused, since a call could not tell which of the value's effects belong to which variable.
- **A throw is not an effect a variable carries.** A function value that may throw cannot bind `E`; a parameter whose type lists `Throw<…>` itself takes one, and the function either catches it with `try` or passes it on in its own list.
- **An actor effect does not reach a variable through `eff[E]` yet.** It may through a function value's effects: a callback that sends to an actor works.

std's eager functions that take callbacks have an effect variable: `map`, `reduce`, `filter` over an iterator, `any`, `all`, `count`, `find_first`, `find_last`, `partition`, `remove_front_while`, `remove_back_while`, `update`, `drain`, `to_map`, `map2`, and the `_by` builders written in Salvo (`list_by`, `deque_by`). So a callback passed to them may log, count or print. The lazy adaptors (`mapping`, `filtering`, …) and the functions std implements on the platform (`sort_by`, `filter` over a list, `set_by`, `map_by`, `array_by`) still take pure callbacks.

## Variadic arguments

Functions support variadic arguments. These get interpreted as an Array of the relevant type:

```
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

A variadic parameter takes a deduction entry like any other. Kept (the default, or `=> ints`), it lends its arguments to the call: a spread `max(...xs)` leaves `xs` usable. Consumed (`=> !rest`), it moves them in, a spread's array with them, so `List(a, ...xs)` spends `xs` just as it spends `a`. Every standard-library constructor consumes its tail, which is what lets a tail hold linear values (see "Linear types").

Variadic arguments are assigned with the remainder left after the non-variadic arguments are assigned. For example, we might write the following overload for the above function:

```
// Guaranteed to have a maximum, because the first argument is given
fn max(first: Int, ...rest: Int[]) -> Int {
    // This calls the above function, because we are not giving the first argument explicitly
    // We could equally call `max(...rest)`, wherein we also don't give a first argument explicitly
    let rest_max: Int? = max(rest)
    if rest_max is Int && rest_max > first {
        // Because of the `is Int` check, `rest_max` is an `Int` in this block
        return rest_max
    }
    return first
}
```
