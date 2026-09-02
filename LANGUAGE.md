# Salvo Lang

Salvo lang is an experimental high level programming language with C-like syntax which can transpile into Rust and Kotlin. The aim of the language is to support the transition from the JVM to a non-garbage collected language by using a language we can work with both of them. This has the following goals:

* No explicit pointer or references.
* No explicit ownership or borrowing, except possibly in type signatures.
* An imperative style rather than a purely-functional style.
* A strong type system, with algebraic effects.
* Support for "qualities" which allow us to annotate data in the type system.

The compiler is written in Rust, and translates code into an intermediate representation. From there, each backend language (initially Kotlin and Rust) writes this to the respective language, filling in the necessary parts which are marked with `expect`.

## Data and Types

### Basic data types

Salvo has the following basic data types, similar to the JVM:

* `Byte`: an 8-bit unsigned integer, equivalent to `Byte` in Kotlin and `u8` in Rust.
* `Int`: a 32-bit signed integer, equivalent to `Int` in Kotlin and `i32` in Rust.
* `Long`: a 64-bit signed integer, equivalent to `Long` in Kotlin and `i64` in Rust.
* `Float`: a 32-bit floating point number, equivalent to `Float` in Kotlin and `f32` in Rust.
* `Double`: a 64-bit floating point number, equivalent to `Double` in Kotlin and `f64` in Rust.
* `Bool`: a boolean, equivalent to `Boolean` in Kotlin and `bool` in Rust.
* `Char`: a character, equivalent to `Char` in Kotlin and `char` in Rust.
* `None`: a singleton type, used to represent expressions with no response.

All numbers include the usual arithmetic operations.

Numeric literals default to `Int` and `Double`: `1` is an `Int` and `1.2` is a `Double`. Suffixes select the other widths: `1L` is a `Long`, and `1.2f` is a `Float` (the `f` suffix requires a decimal point — write `1.0f`, not `1f`). Underscores may separate digits (`1_000_000L`). There are no implicit numeric widenings: assigning `1` to a `Long` variable is a type error; write `1L`.

### Strings

Salvo strings work much the same way as in Kotlin: they are immutable, and can be defined using literals:

```
let str: Str = "hello, world!"
```

There is no equivalent to Rust's string literal type `str`.

The behavior of Strings are governed by the module `core.string`.

### Tuples and Unions

Salvo supports algebraic data types in the form of tuples (for AND) and unions (for OR). The tuple of types `A`,`B`, `C` is written `(A, B, C)` while the union of them is written `A | B | C`. Tuples can be destructured:

```
// The type of `a` is Str
// The type of `b` is Int
// The type of `c` is Bool
let (a, b, c) = ("String", -1, true)
```

Unions can be checked in if expressions and while loops using `is`. The syntax of these checks is similar to type checking in C#:

```
let string_or_number: Str | Int = func()
if string_or_number is Str s {
    // The type of `string_or_number` is `Str` in this block
}

while string_or_number is Int i {
    // The type of `string_or_number` is `Int`
    string_or_number = func()
}
```

It makes no sense to have duplicate types in a union (unless they are qualified, see below): `Str | Str` is equivalent and simplified to `Str`.

The type of a variable can never get more general (although its qualifiers can change, more on this later), and we don't support variable shadowing. In the above example, it is only because the type of `string_or_number` _started_ as a union, it could move between `Str` and `Int`.

### Structs

Data types can be combined into structs which are like Kotlin class objects where all the fields are public and there are no methods:

```
struct Person {
    name: Str,
    age: Int, // trailing comma is allowed but not required
}
```

This can be instantiated like:

```
let person: Person = Person {name: "Roland", age: 36}
```

By default, all the fields of a struct must be defined. If you want to make them optional, then you can give them default values:

```
struct Person {
    name: Str,
    surname: Str | None = None, // surname is optional and nullable (see next section)
    age: Int
}
```

Structs can be destructured. If the variable we destructure it into is the same as the field name, then we just mention it directly, otherwise we use the syntax `field: variable_name`

```
// `name` is type `Str`
// `their_age` is type `Int`
let {name, age: their_age} = person
```

Structs can be built from other structs using a spread operator `...`. This allows us to re-use most of the values from one struct, but then overwrite the ones we want:

```
let person = Person {name: "Roland", age: 36}

// `person2` has the same name, but different surname and age
let person2 = Person {...person, surname: "Elliott", age: 25}
```

Spreading a variable consumes it — its fields now live in the new struct — so `person` can no longer be used after building `person2`; spread `copy(person)` instead to keep both (see the ownership section for the full rules).

When the type of a struct is known, then the type annotation can be dropped:

```
let person: Person = {name: "Roland", age: 36}
```

### Nullability via "| None"

Salvo does not have a null value, but emulates it using union types with "| None". Because unions between a single type `T` and None is common, we have the shorthand `T?` for `T | None`. For example:

```
let person: Person = Person {name: "Roland", age: 36}

// Type of `surname` is `Str | None`
let surname: Str? = person.surname
```

We say that `surname` is _nullable_, because this syntax resembles the nullability in Kotlin. The nullability of a field can be checked using the same approach as in Kotlin:

```
// Because `Str?` is just `Str | None` we can use normal union type checking
if (person.surname is Str) {
    println("Surname is ${person.surname}") // String interpolation like in Kotlin
}
```

When a value is known to be non-null but this can't be proven by the compiler, then you can use `!` to get the non-null value out or panic (equivalent to `unwrap` in Rust):

```
let person: Person = Person {name: "Roland", surname: "Elliott", age: 36}

// Haven't done an explicit check, but we know `surname` is `Str` because we just constructed it
println("Surname is ${person.surname!}")
```

### Arrays

Values can be put into arrays, with similar syntax to Java but with support for building them without null values from the beginning inspired by Rust:

```
// Create an array with literal values - size is inferred from the literal expression
let numbers: Int[] = [1, 2, 3]

// Generate an array with an anonymous function, size is given earlier
let numbers: Int[] = Int[5] { i: Int -> 0 }
```

The array's size can be fetched from `numbers.size()` (see below for dot-notation of functions) and the array can be 0-indexed using `numbers[index]`.

### Any and Nothing

Like Kotlin, there is an `Any` type which is the superclass of all other types. There is also a `Nothing` type which represents unreachable code (similar to `!` in Rust or `never` in TypeScript). In the type system, it is treated as the sub-type of every other type so that the types of branching code work out nicely.

In the following example, the `return` "evaluates" to `Nothing` while the `Ok` branch evaluates to type `Str`. Thus, the type of `value` is compatible with `Str` (technically, the type of the when expression is `Ok Str`, but the code explicitly elides the `Ok` qualifier).

```
let result: Ok Str | Err Int = some_func()
let value: Str = when result {
    is Ok {
        result
    }
    is Err {
        return
    }
}
```

`Nothing` is the "evaluated type" of operators like `return`, `break`, and `continue`.

### Qualifiers

Salvo introduces the notion of _qualifiers_, which act like annotations on data which can expose more or less functionality for them. Qualifiers do not generally have their own definition: their behavior is entirely defined by which functions act on them and how they change them.

A qualifier is defined by being attached to another type. We can then use this quality to define more specific functions on types. So, for our `Person` type, we could define a qualifier of having a surname:

```
qualifier Surname of Person
```

Here, `Surname` is the name of the qualifier, and `Person` is the type it applies to. When we show the full syntax for functions, we will show how they can be used to define semantics around this. For now, it suffices to point out that when we refer to a `Surname Person` it in some sense represents a new type, allowing to overload functions as follows:

```
// Does not know there's a surname, so we have to check it's nullability. Ignore the `[person]` part for now.
fn full_name(person: Person) -> [person] Str {
    if (person.surname is Str surname) {
        return "${person.name} ${surname}"
    }
    // Explicit returns, unlike in Rust
    return person.name
}

// Elsewhere we define that `Surname Person` means that the surname is non-null, so we know we can safely extract the non-null value here.
fn full_name(person: Surname Person) -> [person] Str {
    return "${person.name} ${person.surname!}"
}
```

Qualifiers CANNOT apply to tuples, only to the types which make them up. In an un-parenthesized union type, a qualifier binds to the single arm it is written on — never the union as a whole — so qualifiers can be used to "tag" the elements of unions (a qualifier _can_ be applied to an explicitly parenthesized union group, like `Ok (Ok Str | Err Int)`, as shown later):

```
// Because the qualifier cannot apply to the union as a whole, there is no ambiguity in the type annotation here
let p: Surname Person | None = check_surname(person)

// It is known statically whether `is` is being applied to a qualifier or not
if p is Surname {
    // Type of `p` is `Surname Person`
}
```

There can be overlap between the branches of a union, in which case a smaller union type may be inferred:

```
let p: Surname Person | Person | None = check_surname2(person)

if p is Person {
    // Type of `p` is `Surname Person | Person` because both branches match the check
}

if p is Surname {
    // Type of `p` is `Surname Person` because only one branch matches
} elif p is Person {
    // Type of `p` is `Person` because the `Surname Person` branch was already handled
}
```

Qualifiers can also apply to the types of parts of tuples:

```
// Type of `p` is `(Person, Surname Person | None)`
let p = (person, check_surname(person))
```

The final thing to note about qualifiers is that multiple qualifiers can apply to the same type simultaneously, and the order doesn't matter. In order for two qualifiers to be applied to the same instance, one of the qualifiers must explicitly state that it is compatible with the other:

```
// The `with` tells us that `Old` can be used with `Surname`
qualifier Old of Person with Surname

// We can apply both Old and Surname to the person
let p: Old Surname Person | None = check_old_surname(person)
```

The same qualifier CANNOT be applied multiple times to the same type (i.e. `Old Old Person` is invalid). However, we will see that nested qualifiers _are_ possible.

### Auto-qualifiers and `Mut`

When they're defined, structs can specify "auto-qualifiers", which are precanned qualifiers supported at the language level. At the moment, the only auto-qualifier is `Mut`, which introduces support for a mutable version of the struct, wherein each field can be modified:

```
struct Person with Mut {
    name: Str,
    surname: Str? = None,
    age: Int
}

// By default, the struct is immutable
let person = Person {name: "Roland", age: 36}
person.name = "Someone else" // Compile-time error

// The `Mut` qualifier shows up in the type annotation when building the struct
let mutable_person = Mut Person {...person}
mutable_person.name = "Someone else" // No problem
```

`Mut` is a general language feature, not something a library defines: it composes with every other qualifier, and backends give it meaning (mutable fields in Kotlin, `mut` bindings and `&mut` references in Rust). Besides structs, other type declarations can opt into it with the same `with Mut` syntax — for example, the standard library's list type is declared as:

```
external type List<T> with Mut
```

Applying `Mut` to a type whose declaration does not say `with Mut` is a compile-time error. How a backend maps a `Mut` type is described in the backends section (`Mut inline`).

### Generic types

Structs, qualifiers, and arrays can all refer to generic types, using the following syntax:

```
// Struct with generic types S and  T
struct Pair<S, T> {
    first: S,
    second: T
}

// Qualifier can be as generic as type or less. This governs which `Pair`s the qualifier can apply to.
qualifier Something<S, T> of Pair<S, T>
qualifier FirstStr<T> of Pair<Str, T>
qualifier Ints of Pair<Int, Int>
```

Using qualifiers and generics we can implement the equivalent of a `Result` type from Rust:

```
qualifier Ok<T> of T
qualifier Err<T> of T

let result: Ok Int | Err Str = get_age()

if result is Ok {
    // `result` is `Ok Int` here
    println("Age is ${result}")
} else { // No need to check that it's Err, because all other options are excluded by the other branches
    // `result` is `Err Str` here
    println("Something went wrong calculating age: ${result}")
}
```

Because neither of these are `with` the other, it follows that nothing can be both at the same time. However, because the type of the qualifier is generic, it's possible for _it_ to be a qualified type:

```
let result: Ok (Ok Str | Err Int) | Err Bool = some_function()
```

This nesting is generally discouraged. There is nothing wrong with having multiple qualifiers over various types, such as:

```
let result: Ok Str | Err Str | Err Bool = some_function()
```

This allows us to accumulate failure modes and handle them at a higher level of the code. Because of generic qualifiers, Salvo also supports more precise checks for unions:

```
let result: Ok Str | Err Str | Err Bool = some_function()

if result is Err Str {
    // `result` is of type `Err Str`
}

if result is Err {
    // `result` is of type `Err Str | Err Bool`
}
```

### Type aliases

Type aliases can be convenient for making union and tuple types more readable. You can use generics when declaring a type alias:

```
type Result<S, T> = Ok S | Err T
```

Type aliases need to be imported like everything else (see below for details).

## Control flow

There are minimal control flow options, and every option is an expression. All blocks (if, while, for) represent their own scopes, so that variables declared in them cannot be access from outside.

### If/elif/else blocks

As in Ruby, `if` blocks are expressions which evaluate to values. Each branch of an `if`-`elif`-`else` chain represents a possible value that the chain might resolve to, and the final type is the union of them all. The value and type of a branch are equal to the last expression in that branch:

```
// Type of `full_name` is `Str | Str` which simplifies to `Str`
let full_name = if person.surname is Str {
    "${person.name} ${person.surname}"
} else {
    person.name
}
```

The type of an unspecified `else` branch is `None`:

```
// Type of `full_name` is `Str | None` or `Str?`
let full_name = if person.surname is Str {
    "${person.name} ${person.surname}"
}
```

Salvo only supports evaluating boolean expressions for `if` and `elif` -- there is no default evaluation of other types. Even the `is` expressions we've been using for unions evaluate to booleans, although there is some special syntax to bind the casted value in the scope of the if block.

### When expressions

For exhaustive checking over union types, when expressions are recommended. Since the union type options are known at compile time, the compiler can validate that every type is handled by a branch of the expression. When expressions are of the form:

```
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

There is no default branch, and `when` cannot be used without a subject that has a union type. The subject together with the checks should form a valid boolean expression that could work for an if-expression when concatenated (i.e. `[subject] [check]` should be the boolean expression). As with if-expressions, any type/qualifier checking proven in the condition allows us to refer to the subject within that block by the more specific type. The branches of the when expression each resolve to a value like the `if`-`elif`-`else` chain. To reuse an example from earlier:

```
let result: Ok Str | Err Str | Err Bool = some_function()

// `value` of type `Str` because the `Err` branch does not resolve to a value
let value = when result {
    is Ok {
        // `result` is of type `Ok Str`
        result
    }
    is Err {
        // `result` is of type `Err Str | Err Bool`
        println("Error: {result}")
        return
    }
}
```

### While loops

`while` loops can be thought of as repeated `if` blocks:

* `while` loops repeat until their expression evaluates to false.
* `while` loops support the same `is` expression as `if` and `elif`, where the value is rebound for each iteration of the loop.
* `while` loops evaluate to a value like `if` blocks, which is determined by either the last evaluated expression or one of the `break` statements (e.g. `break "hello, world!"`)
* `while` loops support the `continue` statement, which skips to the next iteration of the loop.
* `while` loops support an `else` block, which runs only if the loop never ran. This works the same as the `else` of an `if` block in that the last expression determines its value.

```
let numbers: Int[] = get_numbers()
let i = 0
let last = while i++ < numbers.size() { // post-fix increment is supported
    numbers[i] // the last number evaluated will be the value of the while loop
} else {
    -1 // defaults to -1 if never looped
}
```

### For loops

`for` loops only work with iterators, and do not support the Java/C like `for (int i = 0; ...)` format. An iterator is a standard type and can be specified using functions (which we will see in detail in the functions section). For example:

```
// Print out a message for each age strictly less than the person's age.
// The `range` function returns an iterator which is exclusive of the end.
for i in range(0, person.age) {
    println("Person is older than ${i}...")
}
```

`for` loops evaluate to values just like `while` loops. They support `break`, `continue` and `else`.

## Functions

Functions play an important role in Salvo lang:

* They define the contracts of qualifiers.
* They are central point of effects (which we will introduce in this section).
* They define iterators (which we will introduce in this section).

### Syntax

Function syntax largely resembles the Rust function syntax, except for the extra annotations around the arrow, which we will explain shortly:

```
fn function_name<generic_param1, generic_param2, ...>(arg1: type1, arg2: type2, ...) [effect1, effect2, ...] -> [deduction1, deduction2, ...] return_type
```

There are no implicit returns of functions (unlike `if`, `while`, and `for` blocks). A function with a return type other than `None` must return on every path: an `if` needs an `else` (or a return after it), and a `when` counts when every branch returns. Iterator functions built from `yield` are exempt. The `generic_param`s define generics which can be used throughout the rest of the function signature.

Like Koka, we support "dot-notation" for calling functions: the first argument can be pulled forward before the function name, like a method call:

```
let list: List<T> = get_list()

// The following two calls are equivalent, and the second gets reduced to the first
let size_normal: Int = size(list)
let size_dot: Int = list.size()
```

Using dot notation allows us to make method-looking functions without actual support for methods.

If a function return type is not specified, it is assumed to be `None`. When a function's return type is exactly `None`, then you can call `return` without a value to return from the function. Also, returning is not required in this case.

We have already seen some examples of functions, so now we will move to the extra bits around the arrow: effects and deductions.

### Variadic arguments

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

### Lambdas

Functions can take lambdas as arguments, as `mapper` in the following example:

```
fn map<S, T>(list: List<S>, mapper: (S) -> T) -> List<T> {
    let result: Mut List<T> = mutable_list()
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
    let list: List<Int> = list(1, 2, 3)

    // The following are all equivalent
    map(list, to_string) // Pass the function by name
    map(list, i -> "${i}") // Use an anonymous lambda without {}, doesn't require a return
    map(list, i -> { return "#{it}"}) // Use an anonymous lambda with {}, does require a return
}
```

Lambdas can declare effects and deductions (to be discussed below), just like normal functions.

### Iterator functions

A special kind of function called an "iterator" allows us to define a sequence of values rather than returning a single value. An iterator supports two functions:

```
type Iter<T>
fn has_next<T>(iter: Iter<T>) -> Bool
fn next<T>(iter: Iter<T>) -> T
```

These are used internally when iterating in `for`-loops. The way to build iterators is by defining a function which returns the `Iter<T>` type. Such a function is called an "iterator function". In an iterator function, the `return` keyword can be used only for short-circuiting, and cannot be provided a value. Additionally, there is a `yield` keyword, which "returns" the next value for the `Iter<T>` returned. Thus, every `yield` must be given a value of type `T`. This is used to implement the standard range 

```
// Exclusive iterator
fn range(start: Int, end: Int) -> Iter<Int> {
    let i = start
    while i++ < end {
        yield i
    }
}

// Inclusive iterator
fn rangeIncl(start: Int, end: Int) -> Iter<Int> {
    // Empty range will not yield anything
    if start > end {
        return
    }
    for i in range(start, end) {
        yield i
    }
    yield end
}
```

When a `for`-loop is used over a data type `T`, then this is assumed to be using a function `iter(T)`. If no such function exists, or the resolution is ambiguous, then it is a compile-time error. For example, the `List<T>` type has an `iter` function:

```
fn iter<T>(list: List<T>) -> Iter<T>
```

So, when we loop over the list the following are equivalent:
§
```
let list: List<Str> = get_list()

// Implicit call to iter(list)
for str in list {
    // Do something
}

// Dot-notation call to iter(list)
for str in list.iter() {
    // Do something
}

// Explicit call to iter(list)
for str in iter(list) {
    // Do something
}
```

### Effects

Some modern languages have been playing with algebraic _effects_. At this early stage, we are just using them as a way of passing concrete implementations of "interfaces" around. An effect defines a set of functions which will be available to functions which declare a dependency on it:

```
// An effect which can generate range T's
effect Random<T> {
    fn next_random() -> T
}
```

Effects have _handlers_, which define their concrete implementations:

```
// A handler which returns successive values in an array, and loops back when done.
// Handlers have their own state, which can be used through their lifetime. Here `numbers` are passed in and `i` is defined at construction time with a default value.
handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    // Every function must be defined. In this case just `next_random`
    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}
```

A handler can be "registered" in the current context using the `use` keyword. This operates similarly to `with` in Koka and `use` in Gleam:

```
fn main() [use] -> [] None {
    // Register the CyclicRandom as the implementation of Random<Int> for the rest of this function.
    // Attempts to register another Random<Int> will fail.
    use CyclicRandom([1,2,3,4])

    // Here we can call the function next_random()
    let num = next_random()
}
```

The effects a function depends on are declared in the square brackets before its arrow. In the above example, the `main` function (which is also the entry point to any Salvo program) starts with the special `use` effect, which is what allows it to use the `use` keyword. If a function does not declare a dependency on this, then `use` is not available to it. If the initial square brackets are not present in a function declaration, then it is assumed to be empty and that function is "pure".

Almost every action other than simple data transformation needs to be encoded in an effect. For example, printing to the console is managed by an effect:

```
effect Console {
    fn println(message: Str) -> [message] None
}
```

Suppose we want to write a function which uses the Random and Console effects. Then we can write the following:

```
fn age_prediction(person: Surname Person) [Random<Int>, Console] -> [person] None {
    // next_random() is accessible because of Random<Int> effect
    let years: Int = next_random()

    // println() is accessible because of Console effect
    println("In ${years} years, ${full_name(person)} will be ${person.age + years}")
}
```

When calling a function, all its effects either need to be `use`d or be declared in the calling function's effect dependencies. Two effects of the same type can be in the dependencies, so long as they have different generic values:

```
fn random_numbers() [Random<Int>, Random<Double>] -> None {
    // Ambiguous: will result in compile-time error
    let number = next_random()

    // Will use Random<Int> handler
    let int: Int = next_random()

    // Will use Random<Double> handler
    let double: Double = next_random()

    // The type can also be specified in the function call
    let number = next_random<Int>()
}
```

### Deductions

The second square bracket includes what we call "deductions". These tell the compiler what happens to the parameters given to a function, which in turn helps us to compile functions with concrete ownership/borrowing rules in Rust.

Suppose that we have the standard library's mutable list (`external type List<T> with Mut`) and a qualifier that tracks non-emptiness:

```
// Tells us that there is at least one element in the list
qualifier NonEmpty<T> of List<T>
```

If we remove an element from the list, then we don't know if it's non-empty any more. We can capture this as follows:

```
fn remove_first<T>(list: Mut NonEmpty List<T>) -> [list: Mut] T
```

A deduction entry takes one of three forms: a bare `[list]` keeps the parameter with _all_ of its declared qualifiers; `[list: Mut]` keeps it with exactly the listed qualifiers; and the explicit-empty `[list:]` keeps the parameter but strips every declared qualifier. These are enforced at each call site: passing a variable to `remove_first` above removes `NonEmpty` from what the compiler knows about it, so a second `remove_first(list)` without an intervening `is NonEmpty` check fails overload resolution.

This tells us that after the function has returned, _we no longer know that the list is NonEmpty_. From the calling context, then, we have the following:

```
let list: Mut List<Int> = mutable_list(1, 2, 3)

// We will discuss this "predicate qualifier" later
if list is NonEmpty {
    // Type of `list` is `Mut NonEmpty List<T>`
    let first = remove_first(list) // can call this because we have `list: NonEmpty Mut`

    // At _this_ point, `list` is no longer NonEmpty, but only Mut
    let second = remove_first(list) // Invalid: there is no function for this
    let size = list.size() // Still valid, because `list` is a List<T>
}
```

In Rust, the `NonEmpty` state was not captured: this is a Salvo compile-time inference. But the fact that the function "gave back" the `list` value is captured by the fact that `list` appears in the deductions of the function -- this means that it was passed as a reference rather than moved. By contrast, if the deduction list was specified by _did not_ include `list`, then the calling function no longer has ownership:

```
fn consume<T>(list: List<T>) -> [] Unit
```

In this case calling `consume(list)` would _move_ the variable to the function: `list`'s type narrows to `Nothing` (a value that no longer exists is an impossibility), and any future reference to it in the calling function is a compile-time error until the variable is reassigned. This holds whether the deduction list is written out or inferred — a function that returns its parameter moves it, and callers are checked against that inferred contract just the same. It also holds uniformly across all types: for basic value types the underlying backends copy the value and the generated code would remain valid, but the Salvo-level contract is enforced consistently regardless of the type. The analysis is branch-aware: consuming a value in a branch that always exits (via `return`, `break`, or `continue`) does not affect the code after the branch, while a value consumed on only some fall-through paths is conservatively unusable afterwards. Loops account for the back edge too: a value read early in a loop body and consumed later in the same body is an error, since the read happens after the consumption from the second iteration onwards (reassigning before the body ends keeps it valid).

Consuming calls are not the only way a value moves. Every other escape route consumes a bare variable the same way, and the error at a later use names the event: storing it in a struct, array, or tuple literal (the literal owns it now), spreading it (`...n` reads all of its fields into a new value and consumes the source), returning it, `break`-ing with it, `yield`-ing it (an iterator function that yields the same variable inside a loop consumes it anew every iteration — an error the loop analysis reports on the second iteration), and passing it to a `use` handler constructor (the handler stores it for the rest of the scope). A `break` with a value reaches the code after the loop on every exit path, so a variable consumed by `break` is unusable after the loop even when the `break` sits inside a branch. Reads, by contrast, never consume anything — in particular, string interpolation is a read: `"${n}"` formats the value and retains nothing, so `n` stays usable. As always, `copy(...)` at the move site keeps the original usable, and reassignment revives it.

When the deduction list is not specified, then it is implied that all parameters are included, with the qualifiers that are inferred from their usage in the function. For example:

```
// The inferred deductions are `[list: Mut]`, because `remove_first` makes this deduction and might be called.
fn maybe_remove_first<T>(list: Mut NonEmpty List<T>) [Random<Int>] -> T? {
    if next_random() > 0 {
        return remove_first(list)
    }
    return None
}
```

Deductions must be statically computable, and so do not depend on the return type of the function. The deductions are automatically made as the strictest deductions of all functions interacting with the respective variable (including moves). If the compiler cannot determine the deductions for a functions, perhaps because it is too complicated, then they must be provided manually.

### Why returning a parameter is a move

A parameter that is kept (listed in the deductions) compiles to a *borrow* in Rust: the caller retains its value. A function's return value, by contrast, is always *owned* by the caller. If a function returns one of its parameters, these two facts collide: returning a borrowed parameter would tie the return value's lifetime to the argument, and Salvo deliberately has no lifetimes to express that — the alternative, an implicit clone, is a hidden cost the compiler never inserts. So returning a parameter transfers ownership out through the return channel, and the value is deduced as _moved_: the caller that passed it in loses it. The same applies to the other escape routes — storing a parameter in a struct, array, or tuple literal, `yield`-ing it, or passing it to a consuming call. Consequently, a written deduction list cannot promise a parameter back when the body returns it: `-> [x] T` with `return x` is a compile-time error.

This is purely a constraint of the Rust backend — the Kotlin backend ignores deductions, since everything is a garbage-collected reference on the JVM — but one Salvo codebase must compile to both, so the checker enforces the stricter contract everywhere. When the caller should keep access to a value, keep the parameter and return something derived from it instead (an element copy, an index, a new value).

Binding a parameter with `let` is *not* on the move list: it creates a *shared fate* link instead, described next.

### Shared fate and `copy`

Salvo has no references, but variables can still overlap: `let m = n` and `let name = person.name` both make a new name for data that another variable already owns. Salvo tracks this as **shared fate**: when one variable is bound to the value or a projection of another — by `let`, assignment, destructuring, a `for`-loop binding, or an `is`/`when` binding — the new variable becomes *derived from* its source, transitively down to the ultimate root. Reading either variable is always fine; reads never consume anything. Operations that need *ownership* of the data are governed by the binding's **mode**, which the compiler infers from how the derived variable is used later:

- **Borrow-mode** (a derived variable that is only ever read): reads flow freely and every ancestor stays usable. Mutating, moving, or reassigning a **root** poisons every variable derived from it — the derived values may no longer exist, so using one afterwards is an error naming both the link and the event. Reassigning a poisoned variable revives it.
- **Move-mode** (a derived variable that is later moved or mutated): the binding *takes ownership* — every ancestor is consumed at the binding itself, and using an ancestor afterwards is an error naming the binding. From the binding on, the variable is the value's independent owner. This is what makes zero-copy consuming pipelines legal: the whole chain of bindings hands the value along, and the Rust backend emits real moves with no clones.
- Move-mode needs every ancestor to be *owned* by the function. Locals always are. A parameter is owned when the function's deductions move it — and when the deduction list is inferred, a move-mode binding reaching a parameter *claims* it: the parameter becomes moved, and callers hand over ownership. A **written** deduction list that keeps the parameter pins it as borrowed instead: moving or mutating anything derived from it stays a compile-time error — you cannot move out of a borrow — and the remedy is `copy`.
- The same ownership rule applies to a **projection in a moved position** — passing `h.tags` to a call that consumes it, or storing it in a literal. If the projected data is mutable, the move consumes the owner (`h` is unusable afterwards) or, for a kept parameter, is an error with the `copy` remedy. Projections of immutable data are free: whether a backend copies or shares immutable data is unobservable.

The escape hatch is one word: the standard library's `copy` duplicates a value, leaving the source untouched and producing a fresh value with no links.

```
fn copy<T>(value: T) -> [value] T   // internal: each backend implements it
```

Here is the discipline at work, together with the deduction contract. With a *written* deduction list that keeps `persons`, moving a derived value out is an error:

```
fn longest_name(persons: Person[]) -> [persons] Str {
    let longest = ""
    for person in persons {                       // `person` derived from `persons`
        if longest.size() < person.name.size() {
            longest = person.name                 // `longest` derived from `person` (and `persons`)
        }
    }
    return longest        // ERROR: `longest` shares its fate with `persons`,
                          // which this function promised to keep ([persons])
}
```

One fix is a single copy at the escape point — one copy for the whole function instead of one per iteration:

```
    return copy(longest)
```

The other fix is to *not* promise the parameter back: drop the written deduction list, and the compiler infers that the pipeline consumes `persons` — the bindings become move-mode, the function demands ownership from its callers, and the whole thing compiles with **zero copies** (in Rust: the argument moves in, the loop iterates by value, the field moves out, the result moves up):

```
fn longest_name(persons: Person[]) -> Str {   // inferred: persons is moved
    let longest = ""
    for person in persons {
        if longest.size() < person.name.size() {
            longest = person.name
        }
    }
    return longest        // fine: the chain owns the value all the way
}
```

Both modes in action on locals:

```
// Borrow-mode: `ys` is only read, so `xs` stays usable — but mutating
// the root poisons the derived variable.
let xs = mutable_list(1, 2, 3)
let ys = xs          // ys derived from xs (borrow-mode: ys is never moved/mutated)
add(xs, 4)           // mutates the root...
size(ys)             // ERROR: ys shared xs's fate and xs was mutated

// Move-mode: `ys` is mutated later, so the binding takes ownership.
let xs = mutable_list(1, 2, 3)
let ys = xs          // ys takes ownership: xs is consumed here
add(ys, 5)           // fine: ys owns the value
add(xs, 4)           // ERROR: ys was bound from xs and later moves the value

let zs = copy(ys)    // an independent duplicate
add(zs, 6)           // fine, and ys is untouched
```

**Lambdas follow the same discipline.** A lambda's relationship to the variables it captures is read off its body, and binds when the closure is created (a closure may run any number of times, so its contract cannot wait for the call): captured immutable values are free; a captured mutable value that the body only *reads* links the closure to it — the variable stays usable, but mutating it poisons the closure; a captured mutable value that the body *mutates* is consumed at creation — the closure owns it now (`copy` first to keep the original); and a lambda can never *consume* a capture, since every run after the first would use a moved value (`copy` inside the lambda instead).

```
let xs = mutable_list(1, 2)
let f = (n: Int) -> { return n + size(xs) }   // reads xs: closure linked to it
apply(f, 1)          // fine
add(xs, 9)           // mutates the root: f is poisoned
apply(f, 1)          // ERROR: f shared xs's fate and xs was mutated

let g = () -> { add(xs, 1) }   // mutates xs: g takes ownership at creation
size(xs)             // ERROR: xs was consumed by the lambda; copy first
```

One more ordering rule: **arguments are evaluated left to right**, and within a single call a later argument cannot mention a value an earlier argument consumed — `f(a, a)` where both parameters move, or `f(a, size(a))`, are errors at the second argument (`copy` at the consuming argument is the remedy).

Some consequences worth knowing:

- **Values from calls are always independent.** `copy(x)`, `get(xs, 0)`, and every other function result carries no links — a function that wants to return a projection of a *kept* parameter must `copy` internally.
- **The analysis is flow-aware** like consumption: links merge across branches (linked on any path means linked), survive loop back edges, and reassignment severs a variable's own links while poisoning its previous derivatives. A `for`-loop binding is fresh each iteration: consuming it inside the body is fine.
- **It is uniform across all types** — an `Int` derived from an `Int` follows the same rules — and **purely static**: on the JVM nothing physically prevents the rejected programs. The discipline is what lets each backend choose the cheapest correct representation with no observable difference: Kotlin shares references throughout; Rust emits real moves for move-mode bindings and clones for borrow-mode ones (real borrows are a later stage).
- Whole-variable granularity: mutating a struct value poisons variables derived from *any* of its fields, and a move-mode binding consumes its ancestors wholly; field-precise tracking may come later.

### Copy semantics per backend

`copy` is an `internal fn` (see the Backends chapter): its declaration gives the checker everything it needs — the argument is kept with all its qualifiers, the result is independent — and each backend lowers calls to it against the argument's *actual type*. Where no Salvo operation could mutate the value anyway, a copy is free: Kotlin emits the argument unchanged (duplicating a reference to immutable data is a copy), and Rust clones. Where mutation is possible, the copy is real on every backend: `Mut List<Int>` becomes `xs.toMutableList()` in Kotlin and `xs.clone()` in Rust; a `Mut` struct with immutable fields becomes `p.copy()` / `p.clone()`. Where a backend cannot yet produce a correct copy (for example, nested mutability like `Mut List<Mut Person>` on the JVM, where a shallow copy would share the inner values), the compiler reports an error rather than emit code that behaves differently across backends.

### Linear types: values that must be used

Everything above makes values *affine*: they can be used at most once. Resource types want the other half too — a file handle that is never closed, a transaction that is never committed or rolled back, is a bug. A type can opt into **linearity** at its declaration:

```
struct FileHandle with Linear {
    fd: Int
}
```

Every value of a linear type carries an **obligation**: on every path, it must be *moved onward* before it goes out of scope. Moving is anything the ownership system already recognizes — passing it to a consuming call (`close(handle)`), returning it, storing it in a struct/array/tuple, spreading it, a move-mode binding handing it to a new owner. Each move transfers the obligation with the value: a function that receives a linear value by move must discharge it in turn; a function that *keeps* (borrows) a linear parameter leaves the obligation with its caller; a derived (fate-linked) variable is an alias and carries no obligation of its own.

Dropping the obligation is a compile-time error, wherever it would happen:

```
fn leak() {
    let h = open("data.txt")
}                               // ERROR: `h` still owns a linear value at scope exit

fn maybe_leak(flag: Bool) {
    let h = open("data.txt")
    if flag {
        close(h)
    }
}                               // ERROR: `h` is consumed on some paths only

fn drop_result() {
    open("data.txt")            // ERROR: a linear value is dropped immediately
}

fn overwrite() {
    let h = open("a.txt")
    h = open("b.txt")           // ERROR: overwriting drops the first handle
    close(h)
}
```

The deliberate escape hatch is one word: `discard(x)` in the standard library consumes a linear value and drops it on purpose — the code says out loud that the resource dies here.

```
fn deliberate() {
    let h = open("data.txt")
    discard(h)                  // fine: an explicit drop
}
```

Rules that keep the obligation sound:

- **Linearity is declared, not applied**: `Linear` cannot be written in a use-site type — every value of a `with Linear` type is linear, always. (A qualifier you could forget to write would defeat the point.)
- **Composites are contagious**: a struct with a linear field, a tuple/array/union with a linear component, is itself linear — storing a handle in a box moves the obligation into the box, and the box must now be passed on.
- **Generics refuse linear values for now**: an unconstrained `T` cannot be instantiated with a linear type — in particular `copy` refuses them (duplicating an obligation is meaningless); `discard` is the one blessed generic. A `where T: Linear`-style opt-in can come later.
- **Lambdas may read but not swallow**: a lambda can read-capture a linear value (an alias), but a capture the body mutates would move the obligation into the closure — an error.
- **Purely static, on both backends**: like the rest of the ownership system, linearity is a protocol the compiler enforces; there is no runtime component and no destructor on either backend, and the discipline is identical on the JVM and in Rust.

## Qualifiers continued

Now that we know how functions work, we can return to the topic of qualifiers and discuss how they are defined.

### Predicate qualifiers

We have not really described how to _add_ qualifiers to a type. There are basically two ways: by construction or by predication. A predicate qualifier is one in which we can write the predicate which allows us to see that the qualifier applies to a type. In this case, we define the function `qualifies` inside the qualifier, which takes a parameter of the given type and returns a boolean. This function does not support deductions because it can only ever be additive to the qualifiers of the type and can never move the value. It can, however, require effects, in which case the effects must have handlers in the context like any other function call:

```
qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

qualifier NonNegative of Int with Positive {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

// Incompatible with Positive, NonNegative
qualifier Negative of Int {
    fn qualifies(int: Int) -> Bool {
        return int < 0
    }
}
```

To access this, you can use the standard `is` syntax we have already seen. When applied to a non-union type of the relevant kind (in this case `Int`), then it calls the `qualifies` function and allows casting:

```
let number: Int = random_int()
if number is Positive {
    // `number` is of type `Positive Int` in this block
}
```

When a predicate qualifier is applied to a struct, it can specify more specific versions of the types of that struct's fields. These more specific types are not checked by the compiler, but they are cast and asserted when used in the code. Thus, if not carefully managed, these can result in runtime exceptions.

```
qualifier Surname of Person {
    surname: Str // More specific type for the `surname` field when `Surname` applies

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

if person is Surname {
    // `person.surname` has type `Str` here because `person` has type `Surname Person`
}
```

### Constructive qualifiers

Predicate qualifiers apply to an existing value, and need not be present in the generated Rust and Kotlin code. By constrast, _constructive_ qualifiers can correspond to a new type in the underlying Rust or Kotlin code (like `MutableList<T>` is different from `List<T>` in Kotlin) or correspond to a different way of using it (like mutable `Vec<T>` requires mutable references and variables in Rust). To support such cases, the only way to build these sorts of types is by calling a function. The qualifier itself has no body, and the constructor functions must all be defined in the same file as the qualifier declaration. A constructor function is marked by writing `as Qualifier` after its return type: every return point returns a plain instance of the return type, which is assumed to gain the qualifier _by construction_. Callers of the function see the qualified type. Constructor functions must return a simple type (not a union or tuple); other functions can add more complexity on top of the constructors:

```
// No body -- we're using a constructive qualifier
qualifier RandomPositive of Int

// `-> Int as RandomPositive` marks this as a constructor for the qualifier.
// Callers see the return type `RandomPositive Int`.
fn random_positive_int() [Random<Int>] -> Int as RandomPositive {
    let num = next_random()
    while num <= 0 {
        num = next_random()
    }

    // Returns a plain Int; it qualifies as RandomPositive by construction.
    return num
}
```

Constructor functions are not limited to constructive qualifiers: a predicate qualifier may declare constructors too. The constructor asserts that its predicate holds _by construction_, so callers get the qualified type without a runtime `qualifies` check. The same rules apply — constructors must live in the same file as the qualifier and return a simple type:

```
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

// Requiring a first element guarantees the predicate by construction.
fn non_empty_list<T>(first: T, ...rest: T[]) -> List<T> as NonEmpty {
    return list(first, ...rest)
}
```

This is also how union arms are tagged in practice: a generic constructor applies the tag, and the tagged value then coerces into the union:

```
qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

fn parse_age(input: Int) -> Ok Int | Err Str {
    if input >= 0 {
        return ok(input)
    }
    return err("negative age")
}
```

Constructive qualifiers can also be entirely handled by the backend implementation. We will discuss this more in the section on backends.

## Modules and files

The file extension for Salvo source code is `.sv`. Modules correspond to files, so that there is no need to specify the module name or path at the top of the file (as in Java and Kotlin).

The compiler discovers sources by walking the source directory recursively. Hidden directories and cache directories (those carrying a `CACHEDIR.TAG` marker, such as Cargo's `target/`) are skipped. A `.svignore` file at the source root excludes further paths: one path per line, relative to the root, naming a file or a whole directory subtree; blank lines and `#` comments are ignored.

Everything must be imported explicitly unless it is defined in the `core` module. This includes qualifiers, types, effects, handlers, and functions. When there is ambiguity in a reference (e.g. functions which have the same signature, qualifiers which apply to the same type), then they can be aliased using `as` in the import statement.

```
import random.Random // import Random<T> effect from random.sv
import list.add // Import that list's add function from list.sv
import list.ext.add as ext_add // Import the list extensions add function from list/ext.sv, which is now aliased
import core.Str // Not strictly necessary -- all of core is imported by default
```

Only the modules which are used in the code are transpiled to the relevant backend equivalent (modules in Rust, packages in Kotlin).

## Backends

One of the aims of Salvo is to make it easy to integrate Salvo code with the backend code. To achieve this, the Salvo compiler builds an internal representation (in Rust), and passes this on to the configured backend implementation to write out the relevant target source code. In order to support this, we distinguish between the `internal` and `external` backend layers. The core library uses both of these.

### Internal 

The `internal` layer sits in a backend specific module inside the compiler. This handles complex language-specific logic, and core functionality: how to encode union types, what the `None` type transpiles to in different cases, how to pass parameters to functions, how function naming works, how imports are handled, and more. These can only be changed by making changes to the compiler itself. Anything involving syntax will appear here, and all `internal` backend definitions are declared as part of the standard library (defined in `std`).

For example, the basic types (`Int`, `Str`, `Iter<T>`, ...) are declared as `internal type`s, and each backend maps them natively:

```
internal type Str
```

When building the compiler, _all_ `internal` declarations must be handled by _every_ backend module.

Functions can be internal too. An `internal fn` is a compiler intrinsic: the declaration carries the signature and deductions the checker uses, but there are no templates — each backend lowers calls to it directly, seeing the resolved argument type at every call site. This is what makes type-directed lowering possible where a single generic template could not express it; the standard library's `copy` is the canonical example:

```
internal fn copy<T>(value: T) -> [value] T
```

A backend that does not implement an internal fn, or cannot lower it for a particular argument type, reports a compile-time error — never wrong code.

The `Mut` auto-qualifier is also handled at this level: a type declaration can opt into it with `with Mut` (`external type List<T> with Mut`), and each backend decides what `Mut` means. For external types, the `define type` block may provide a `Mut inline` section giving the target type used when the type is `Mut`-qualified:

```
// In file list.kotlin.sv
define type List<T> {
    inline: ``
    List<${T}>
    ``

    Mut inline: ``
    MutableList<${T}>
    ``
}
```

When no `Mut inline` section is given, `Mut` simply erases for that backend (Rust, for example, maps both `List<T>` and `Mut List<T>` to `Vec<T>` — mutability shows up in bindings and references instead).

### External

The `external` layer sits outside the compiler, and uses a simple templating system to make it possible to write new backend-accessible modules as part of the codebase. Most of the standard library is defined using the external layer, and users can add their own external definitions to support interop with their target language. Unlike internal definitions, not every declaration needs to be handled by every backend: everything in the core library must (`core.*`) because these are all implicitly imported, but outside of this only those things which are explicitly imported need to be handled. This is checked at compile time.

To start, you prefix a definition with `external` and leave out any implementation details besides the signature:

```
// In file string.sv

// Returns an array of characters in the [str]
external fn chars(str: Str) -> Char[]
```

Alongside the file where this is defined, you define files for each target backend, and use the `define` syntax to tell Salvo how to resolve the function call to something. The `define` keyword exposes the "``" character, which is used to create the code which will be interpolated at that call site in the target code:

```
// In file string.kotlin.sv

define fn chars(str: Str) -> Char[] {
    inline: ``
    ${str}.toCharArray()
    ``
}
```

As can be seen above, inside the `define` scope we have access to an `inline` section, which defines how the definition will be inlined. We use something similar to string interpolation, but it works a bit differently here: when `chars(...)` is called in Salvo, the expression is _replaced_ with the inlined code defined in the `define.inline` section, perhaps after being converted as need be to the relevant language paradigms following normal backend rules.

The `define` scope also provides an `imports` section, which can be used to define the imports that need to appear in the generated target source. For example, supposing that we needed to import the `toCharArray` function, we could have written the above as:

```
// In file string.sv

define fn chars(str: Str) -> Char[] {
    imports: ``
    import kotlin.string.toCharArray
    ``

    inline: ``
    ${str}.toCharArray()
    ``
}
```

These imports will now be included at the top of the file whenever we have to interpolate this inline definition The same logic applies to _type_ definitions, which tell Salvo how to refer to a specific type in the backend language as well as what imports are required to do so. Here we see that we can interpolate generics as well:

```
// In file linked_list.sv
external type LinkedList<T>

// In file linked_list.kotlin.sv
define type LinkedList<T> {
    imports: ``
    import java.util.LinkedList
    ``

    inline: ``
    LinkedList<${T}>
    ``
}
```

If you would like to provide custom function definitions, then you can do so as well. Suppose we want a complicated function implementation using Kotlin-specific language features:

```
// In file complicated.sv
external fn complicated_func<T>(list: List<T>) -> Str

// In file complicated.kotlin.sv
define fn complicated_func<T>(list: List<T>) -> Str {
    imports: ``
    import salvo.complicatedFunc
    ``

    inline: ``
    complicatedFunc(${list})
    ``
}

// In file complicated.kt
package salvo

fun complicatedFunc(list: List<T>): String {
    // Normal Kotlin code
}
```

The relevant Kotlin source file (`complicated.kt`) is then copied into the results if this file is ever needed.

Outside of validating that the interpolated variables refer to declared variables, the resulting code is written as-is into the target source files of the backend language. Their correctness is not guaranteed or validated by Salvo.

## Specific backend details

### Kotlin

* When `None` is the only return type of a function, it should be translated to `Unit`.
* The backend should define generic union type wrappers using a sealed interface. If the larger union type is of size N, then the backend should define union types for each number from 1 to N. The qualifier checks then reduce down to checking which of the sealed types a value results in.
* Effects and handlers can map to interfaces and implementations of those interfaces. The effects are passed to a function as the first arguments of that function, and all uses of those effects is mapped to the relevant parameter name.
* The `Iter<T>` type should map to the `Iterable<T>` type in Kotlin, since this is what can be looped over in for-loops. A custom iterable type can be defined for dynamic `iterator {}` blocks in Kotlin.

### Rust

* When `None` is the only return type of a function, the return type is omitted (`()`).
* `T?` maps to a physical `Option<T>`; union types map to generated enums (`Union2<T1, T2>` with one variant per non-`None` arm).
* Deductions determine ownership: a parameter that appears in a function's deductions is passed by reference (`&T`, or `&mut T` when its declared type carries `Mut`), while a parameter omitted from the deductions is moved (passed by value) — the calling code no longer has access to it in Salvo, so the move is always legal. Copy scalar types are always passed by value.
* Effects map to traits with `&mut self` methods; effect dependencies become leading `&mut dyn` parameters, and `use` instantiates a handler into a local that is threaded as `&mut local`.
* The `Iter<T>` type maps to `Vec<T>`: iterator functions collect eagerly (`yield` pushes into a result vector).
* See BACKEND_SPEC.rust.md for the full rules.