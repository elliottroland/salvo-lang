# Lambdas and variadic arguments

## Lambdas

Functions can take lambdas as arguments, as `mapper` in the following example (this is what the standard library's `map` does, spelled out):

```
fn transform<S, T>(list: List<S>, mapper: (S) -> T) -> List<T> {
    let result: Mut List<T> = mut_list_of()
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
    let list: List<Int> = list_of(1, 2, 3)

    // The following are all equivalent
    transform(list, to_string) // Pass the function by name
    transform(list, i -> "${i}") // Use an anonymous lambda without {}, doesn't require a return
    transform(list, i -> { return "${i}"}) // Use an anonymous lambda with {}, does require a return
}
```

Lambdas can declare effects ([Effects and handlers](Effects-and-Handlers.md)) and deductions ([Deductions and ownership](Deductions-and-Ownership.md)), just like normal functions.

### Effects on function types

A function *value* that performs an effect says so in its type, and the effect is supplied by whoever **calls** the value:

```
// `f` may log; `run_it` gets `[Logger]` for free — the only reason to take
// `f` is to call it, and calling it needs Logger here.
fn run_it(f: (s: Str) [Logger] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn demo() [Console, Logger] {
    println(run_it(s -> {
        log("in lambda ${s}")      // legal: the fn type declares Logger
        return "done ${s}"
    }, "x"))
}
```

The rules follow from "the caller supplies it":

- **A lambda body performs what its type declares** — not whatever its enclosing scope happens to have. A lambda passed where the fn type declares nothing may not log, even inside a function that can.
- **A function inherits its fn-typed parameters' effects.** `run_it` above needs no effect list of its own, but its *callers* must have `Logger` available, because that is where the value comes from at run time.
- **Fewer effects fit where more are expected.** A pure function passes wherever an effectful one is expected — it is handed the effect and ignores it. The reverse is an error: the value would reach a call site that cannot supply it.
- **An un-annotated lambda's effects are inferred** from its body, so `let f = () -> { println("hi") }` has type `() [Console] -> None` and does not silently fit a pure position.
- **A function value carries no capability**, so storing or returning one is fine; what needs the effect is *calling* it. A stored effectful value called where its effects are unavailable is an error at the call.
- `use` cannot appear in a function type: registering a handler is local to a body, so a lambda may `use` exactly when the function containing it may.

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
