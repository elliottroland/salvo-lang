# Functions

Functions play an important role in Salvo lang:

* They define the contracts of qualifiers.
* They are central point of effects (which we will introduce in this section).
* They define iterators (which we will introduce in this section).

## Syntax

Function syntax largely resembles the Rust function syntax, except for the annotations around the arrow — the effects before it and the deduction clause after the return type — which we will explain shortly:

```
fn function_name<generic_param1, generic_param2, ...>(arg1: type1, arg2: type2, ...) [effect1, effect2, ...] -> return_type => deduction1, deduction2, ...
```

There are no implicit returns of functions (unlike `if`, `while`, and `for` blocks). A function with a return type other than `None` must return on every path: an `if` needs an `else` (or a return after it), and a `when` counts when every branch returns. The `generic_param`s define generics which can be used throughout the rest of the function signature.

Like Koka, we support "dot-notation" for calling functions: the first argument can be pulled forward before the function name, like a method call:

```
let list: List<T> = get_list()

// The following two calls are equivalent, and the second gets reduced to the first
let size_normal: Int = size(list)
let size_dot: Int = list.size()
```

Using dot notation allows us to make method-looking functions without actual support for methods.

If a function return type is not specified, it is assumed to be `None`. When a function's return type is exactly `None`, then you can call `return` without a value to return from the function. Also, returning is not required in this case.

**A generic function's type arguments must be determined at every call.** Usually the arguments settle them (`Mut List(1, 2)` is a `Mut List<Int>`), but when they cannot, the *context* is consulted: the annotation on a `let`, the enclosing function's return type, or the parameter type the result flows into. If nothing determines a type argument that appears in the result type, the call is an error and you name it yourself:

```
let xs = Mut List()                  // ERROR: nothing says what T is
let xs: Mut List<Int> = Mut List()   // fine: the annotation says
let xs = Mut List<Int>()             // fine: written at the call
takes_ints(Mut List())               // fine: the parameter says
fn fresh() -> Mut List<Int> {
    return Mut List()                // fine: the return type says
}
```

Salvo does not look *forward* to a later use to decide a type argument, even where a target language would: what the compiler knows must be visible at the call itself. A type argument that never reaches the result type needs no context — nothing downstream could observe it.

## Overload resolution

Salvo overloads by argument type, so a name may mean several functions. Which one a call means is decided by one rule in three steps, and the rule is designed to be *predictable*: where it cannot decide, it reports an error instead of guessing, and you always have a way to say what you meant.

**1. The most specific *scope* wins.** Functions reach a call site from ever more specific places:

```
core                    // implicitly visible, least specific
explicit imports        // `import lib.describe`
this module's own declarations
the function's own scope    // fn-typed parameters, locals, implicit parameters, effect members
inner scopes                // a `rename` in a block, a local in a block
```

Scope decides what is *visible*, not what a call means. Every overload that fits the arguments competes, wherever it came from — so declaring your own `size(List<T>)` does not quietly take over `size(xs)` in your module: both fit, and the call has to say which:

```
fn size<T>(list: List<T>) -> Int => list {
    return 99
}

size(List(1, 2, 3))          // error: ambiguous — two `size`s fit
size@main(List(1, 2, 3))     // 99 — yours
size@core.list(List(1, 2))   // 2  — std's
size("abcd")                    // 4  — core's, the only one that fits
```

If you mean to work with your own overload throughout a module, take it out of the shared name: `rename fn size2 = size(list: List<T>)`, or `import … as` for an imported one. Both leave the calls selector-free, which is the point.

A local variable is different: it is not an overload but a value, so it hides every function of that name outright, and `@` is how you reach one anyway.

**2. The most specific *signature* wins**, compared per argument:

- a **type variable** says the least: `describe(Int)` beats `describe<T>(T)`;
- a **broader union** says less than a narrower one, which says less than a single arm: `Int` beats `Int | Str` beats `Int | Str | Bool`, and `Int` beats `Int?`. `Any` is the broadest type there is, so it is always last;
- **more qualifiers** say more: `Mut NonEmpty List<T>` beats `Mut List<T>` beats `List<T>`. Which *kind* of qualifier never matters — ranking `Mut` against `NonEmpty` would ask you to know more than what is in front of you;
- a **fixed** parameter list beats a variadic one, so `List()` picks a no-argument overload over `List(...elems)`.

Specificity can never exceed what the caller knows: a value whose type is `Int | Str` does not fit `f(Int)` at all, and once narrowed with `is`, it does.

The comparison is per argument, and a candidate wins only by being at least as specific in *every* argument and more specific in at least one. Nothing else takes part — not effects, not deductions, not implicit parameters — except the return type, among overloads whose parameters are identical (below).

**3. No single winner is an error.** Two candidates that each win one argument — `mix<T>(a: T, b: Int)` against `mix<T>(a: Int, b: T)`, called as `mix(1, 2)` — rank neither way, and so do two that differ only in *which* qualifier they demand. The call is an error naming both candidates, never a coin flip. Two declarations with the same parameter *types* and the same return type are a duplicate rather than an overload set, reported where the second one is written.

### Saying which one you meant

Two ways, both compile-time only and both erased from the output.

**`@module` names the module whose overload you mean**, which overrides scope precedence and reaches past a local of the same name:

```
size@core.list(xs)      // std's, though this module declares its own
size@list(xs)           // the same: a module is named by any unambiguous suffix of its path
size@main(xs)           // this module's, said explicitly (and no warning)
xs.size@core.list()     // dot-notation, since `@` attaches to the name
```

The suffix rule holds for every module *selector* — `@module` and `by` — and a
suffix that fits two modules (`@list` with a `std.core.list` and a `shop.list` in
play) is refused naming both; write more of the path. An `import` is not a
selector: it says where a name comes from, and writes the whole path.

**`rename` gives one overload a name of its own**, which is how an ambiguity is settled:

```
fn label(n: Even Int) -> Str => n { return "even" }
fn label(n: Small Int) -> Str => n { return "small" }

rename fn label_small = label(n: Small Int)

label(n)         // the `Even` overload — the only one still called `label`
label_small(n)   // the other one
```

A rename is not an alias: from that point on the renamed overload answers *only* to the new name. The parameter list repeats one overload's parameters exactly — same names, same types, type parameters positional — and may not mention effects, deductions or a return type. So a rename cannot single out one of a set of overloads that differ only in their return type. A rename at module level applies to the whole module; written inside a function or a block it applies from that line to the end of that scope, loops and lambdas included. It is not importable: taking an overload out of a shared name is the consumer's decision to make.

### Two places the same rule applies

**Passing a function by name** selects an overload from the type the position expects, by the call's rule — every visible overload that fits competes, wherever it came from, and the most specific signature wins:

```
fn tag(v: Int) -> Str => v { return "int" }
fn tag(v: Str) -> Str => v { return "str" }

fn apply(f: (Str) -> Str, s: Str) -> Str => f, s { return f(s) }

apply(tag, "x")      // the `Str` overload: it is what `(Str) -> Str` needs
let f = tag          // ERROR: nothing here says which `tag` — annotate, or rename
```

So an imported `tag(v: Int)` beside a `tag<T>(v: T)` of your own wins by name exactly as `tag(1)` picks it, and a `tag(v: Int)` of your own beside the imported one is ambiguous by name as it is at a call: write `apply(tag@main, 1)` or `apply(tag@lib, 1)`, or rename one.

**Filling an implicit parameter** is the same query against a type rather than an argument list, so it follows the same rule — every visible overload that fits competes, the most specific signature wins, and no single winner is an error — and a renamed overload no longer fills an implicit of its old name:

```
struct Foo { n: Int }
fn to_str(f: Foo) -> Str => f { return "?" }     // Foo's text form, in Foo's file

"${List(1, 2)}"           // [1, 2]: std's `to_str(List<T>)`, then its `to_str(Int)`
"${List(Foo { n: 1 })}"   // [?]: the list's, then Foo's own for each element
```

The chosen function's own implicits are filled after it is chosen; if they cannot be, that is the error, and no other candidate is tried instead.

`to_str` is an operator's function (interpolation calls it), so a second one for a type cannot be declared outside the type's file: a `to_str(List<T>)` of your own, or a catch-all `to_str<T>(x: T)`, is an error where it is written. A rendering of your own takes another name and is passed explicitly — `to_str = as_hex` where the implicit is filled. See [Comparison, equality and hashing](Comparison-and-Hashing.md#one-per-type-where-an-operator-is-declared).

Beyond the declaration itself, two things hang off the arrow: effects and deductions.

## Implicit parameters

A parameter written with `?` is one the caller does not have to pass — the
compiler fills it from what is visible at the call:

```
fn sort<T>(list: List<T>, ?cmp: (T, T) -> Int) -> List<T> => list { ... }

sort(names)                    // `cmp` resolved from scope
sort(names, cmp = descending)  // or supplied by name
```

They are how a type's capabilities travel without a trait system, and they
reach into type declarations and qualifiers too:
[Implicit parameters](Implicit-Parameters.md) is the whole story.

### Overloading by return type

Two overloads may also take the same parameters and differ only in what they return. The call is then chosen by where its result goes:

```
fn from_str(s: Str) -> Int? { … }
fn from_str(s: Str) -> Bool? { … }

let port: Int? = from_str("8080")       // the annotation chooses
let on: Bool = from_str(text) ?: false  // so does the type the `?:` must produce
```

Only overloads with identical parameters are chosen this way: a call its arguments already decide is never moved to another overload by its annotation. Where nothing says which result is wanted (`let x = from_str(text)`), the call is an error asking for an annotation. Two such overloads may not return types one of which contains the other, such as `Int` and `Int?`.

## Default parameter values

An ordinary parameter can give a default — the same `= expr` struct fields
already use:

```
fn greet(name: Str, greeting: Str = "Hello", punct: Str = "!") -> Str {
    return "${greeting}, ${name}${punct}"
}

greet("Ada")                    // "Hello, Ada!"
greet("Ada", "Hi")               // "Hi, Ada!"      — trailing default omitted
greet("Ada", punct = "?")        // "Hello, Ada?"   — the middle one reached by name
```

A call may omit a **trailing run** of defaulted parameters positionally; to
reach a defaulted parameter that is not last, name it with `name = value` —
the same form that overrides an implicit parameter. Naming a parameter that
has no default is an error: a name is for reaching past a default, not a
second way to write an ordinary call.

A default is checked once, where it is written, against the parameter's
type — with only the *earlier* parameters in scope (so order matters, same
as the parameter list reads) and no effects available, so a default cannot
quietly make every caller that omits the argument perform one:

```
fn span(lo: Int, hi: Int = lo + 10) -> Int {
    return hi - lo
}

span(5)        // 10 — hi defaults to lo + 10
span(5, 20)    // 15
```

A default may read an earlier parameter of any type — a `List`, a struct,
anything. Behind the scenes, reading it twice (its own argument, and the
default) does not mean evaluating your expression twice: the compiler binds
the earlier parameter's value once and reads it from both places.

```
fn describe(xs: List<Int>, count: Int = size(xs)) -> Str {
    return "size is ${count}"
}

describe(List(1, 2, 3))   // "size is 3" — xs is read once, shared by both uses
```

A variadic parameter (`...xs`) and an implicit parameter (`?cmp`) each already have
their own way
of filling themselves when the caller says nothing, so neither takes a
default.

A handler's constructor parameters take defaults by the same rules, wherever
the handler is constructed — `use`, `spawn`, and an item of a `with` clause:

```
handler Span(lo: Int, hi: Int = lo + 10, step: Int = 1) of Spanned { … }

use Span(5)                          // hi = 15, step = 1
use Span(5, step = 2)                // hi = 15 — the middle one left to its default
let s = spawn Span(5, hi = 8)        // hi = 8, step = 1
use Scaled with Span(1, step = 3)    // a `with` item's constructor, too
```

