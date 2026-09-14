# Salvo Lang

Salvo lang is an experimental high level programming language with C-like syntax which can transpile into Rust and Kotlin. The aim of the language is to support the transition from the JVM to a non-garbage collected language by using a language we can work with both of them. This has the following goals:

* No explicit pointer or references.
* No explicit ownership or borrowing, except possibly in type signatures.
* An imperative style rather than a purely-functional style.
* A strong type system, with algebraic effects.
* Support for "qualities" which allow us to annotate data in the type system.

The compiler is written in Rust, and translates code into an intermediate representation. From there, each backend language (initially Kotlin and Rust) writes this out to the respective language.

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

All numbers include the usual arithmetic operations, on **numeric operands only**: `+`, `-`, `*`, `/` and `%` work on `Int`, `Long`, `Float` and `Double` (and unary `-` on the same). `/` between integers is integer division on every backend. `+` does not concatenate strings — `${}` interpolation is how text is built — and `&&`, `||` and `!` take `Bool` operands only: Salvo has no truthiness in value position any more than in conditions. Ordering (`<`, `<=`, `>`, `>=`) works on numbers and on structs declaring `canbe ordered`; equality is its own, broader story (see Collections).

Mixed widths widen implicitly **within** a class: `Int + Long` computes at `Long`, `Float < Double` compares at `Double` — the compiler records the promotion and each backend renders it its own way. Mixing the integer and float classes never happens implicitly: `1 + 2.5` is a type error naming the explicit conversions — `to_int`, `to_long`, `to_float` and `to_double`, declared in `core.basic` for every source width, truncating toward zero and saturating at the target's bounds identically on both backends (`to_int(Long)` keeps the low 32 bits).

Numeric literals default to `Int` and `Double`: `1` is an `Int` and `1.2` is a `Double`. Suffixes select the other widths: `1L` is a `Long`, and `1.2f` is a `Float` (the `f` suffix requires a decimal point — write `1.0f`, not `1f`). Underscores may separate digits (`1_000_000L`). An **unsuffixed** literal also *adopts* the numeric type its position expects — `let x: Long = 1`, `let d: Double = 3` and passing `1` to a `Long` parameter all work, and `x + 1` needs no `1L` because the operator widening covers it. Adoption is for literals only: an `Int` *variable* never becomes a `Long` implicitly — write `to_long(n)`.

### Strings

Salvo strings work much the same way as in Kotlin: they are immutable, and can be defined using literals:

```
let str: Str = "hello, world!"
```

There is no equivalent to Rust's string literal type `str`.

A string being immutable does not mean building one has to be quadratic: `Str` opts into the `Mut` auto-qualifier (see below), so `Mut Str` is *a string under construction*. It is asked for explicitly — a literal is never a `Mut Str` — and it is the only place the string functions come in a mutating flavour:

```
let text: Mut Str = mut_str("hello")
append(text, ", world")
set(text, 0, 'H')
println(text)              // Hello, world
```

Everything else in `core.string` takes a plain `Str`, and a `Mut Str` reaches all of it by *dropping* its `Mut` like any other qualifier — `size(text)`, `trim(text)`, `text == other` all work. The difference from every other qualifier is invisible in Salvo and matters to the backends: `Mut` is the one qualifier a target may render as a different type (a `StringBuilder` on the JVM), so dropping it there is a real conversion rather than a widening. The compiler records the drop and each backend renders what it needs, which is what keeps `text == other` a comparison of *characters* on both targets.

**Interpolation needs a text form.** `"${value}"` works directly for the scalars, `Str`, and a union whose every arm is one of those. Anything else needs a `to_str` — a function taking the value and returning `Str` — which the compiler looks for *at the interpolation site*, exactly as it fills an implicit parameter:

```
struct Point { x: Int, y: Int }

fn to_str(p: Point) -> Str => p {
    return "(${p.x}, ${p.y})"
}

println("at ${point}")     // uses the `to_str` above
```

Without one, the compiler says so rather than letting the backend fail. The standard library renders a list this way — `"${list_of(1, 2, 3)}"` gives `[1, 2, 3]` — and a `Mut Str` needs nothing special, since it drops its `Mut` first.

**A struct interpolates by default when every field does.** With no `to_str` of its own, a struct whose fields are all scalars or strings renders in the shape its literal has:

```
struct Person { name: Str, age: Int }
println("${Person {name: "ann", age: 3}}")    // Person { name: ann, age: 3 }
```

An explicit `to_str` always wins, and a struct with a field that itself needs one is not derived — write the `to_str` instead. The format is the language's own on purpose: leaving it to each target would print different text on the JVM than in Rust.

**Declaring the obligation is optional.** A type may say `: ToStr<self>`, which does not change how interpolation works — it checks at the declaration that a matching `to_str` exists, so the mistake surfaces where the type is defined rather than where it is printed.

The behavior of Strings are governed by the module `core.string`.

### Tuples and Unions

Salvo supports algebraic data types in the form of tuples (for AND) and unions (for OR). The tuple of types `A`,`B`, `C` is written `(A, B, C)` while the union of them is written `A | B | C`. Tuples can be destructured:

```
// The type of `a` is Str
// The type of `b` is Int
// The type of `c` is Bool
let (a, b, c) = ("String", -1, true)
```

A single element can also be read by its position, written like a field with
the index in place of the name:

```
let point: (Int, Int) = (3, 4)
let x = point.0             // Int
let y = point.1             // Int

// Nesting reads left to right, so `.0.1` is "element 1 of element 0".
let nested: ((Int, Int), Str) = ((3, 4), "label")
let inner_y = nested.0.1    // Int
```

Positions are counted from zero, and an index that the tuple does not have
is a compile-time error — unlike an array subscript, the position is part of
the program text rather than a runtime value. Tuple elements are read-only:
qualifiers never apply to a tuple, so no tuple value can be `Mut`, and there
is nothing to assign through. Build a new tuple instead.

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
// Because `Str?` is just `Str | None` we can use normal union type checking.
// The check narrows the field itself: inside the block, `person.surname`
// reads as `Str`.
if (person.surname is Str) {
    println("Surname is ${person.surname}") // String interpolation like in Kotlin
}
```

Narrowing works for any chain of fields and tuple positions out of a
variable (`a.b.0.c`), and the fact is dropped as soon as anything could have
changed the value: an assignment to that place or one containing it, a
reassignment of the variable, or a call that takes it as `Mut`. Array
elements do not narrow — `numbers[i]` cannot be told apart from
`numbers[j]` — so for those, bind the checked value with `is Str name` as
shown for unions above.

A narrowing also survives a **guard**: when every branch of an `if` leaves the
block — `return`, `break`, `continue`, or a call that never comes back — what
follows is on the else-path, so it reads with the remaining arms.

```
fn next(p: Mut ListYield<T>) -> Emitted T | Finished => p: Mut {
    let element = get(p.items, p.at)
    if element is None {
        return finished()       // the only way out for the empty case
    }
    p.at = p.at + 1
    return emitted(element)     // `element` is `T` here, not `T?`
}
```

An `elif` chain accumulates the same way, so two exiting branches leave the
third arm. One branch exiting is not enough: if any branch can fall through,
either path may have been taken and nothing is narrowed.

When a value is known to be non-null but this can't be proven by the compiler, then you can use `!` to get the non-null value out or panic (equivalent to `unwrap` in Rust):

```
let person: Person = Person {name: "Roland", surname: "Elliott", age: 36}

// Haven't done an explicit check, but we know `surname` is `Str` because we just constructed it
println("Surname is ${person.surname!}")
```

### Collections

`List<T>`, `Set<T>` and `Map<K, V>` are the everyday collections, and each has a literal:

```
let xs = [1, 2, 3]                       // List<Int>
let names = {"ada", "grace"}             // Set<Str>
let ages = {"ada": 36, "grace": 45}      // Map<Str, Int>
```

A literal builds an immutable collection; write `Mut` on the position to ask for a mutable one, exactly as with a struct literal:

```
let seen: Mut Set<Str> = {}
add(seen, "ada")
```

Besides the literals, each collection can be **generated** from a size and a rule, or **converted** from another:

```
let squares = list_by(4, i -> i * i)        // [0, 1, 4, 9]
let unique = to_set(xs)                     // duplicates collapse
let lengths = to_map(names, n -> (n, size(n)))
```

There are two `to_map` forms — from a list of pairs, and from a list of anything plus a rule saying what key and value each element becomes. Duplicate keys are last-wins everywhere.

Each literal is sugar for the corresponding constructor — `list_of`, `set_of`, `map_of`, with `mut_list_of`, `mut_set_of`, `mut_map_of` for the mutable forms — so anything a literal does can be written as a call.

An **empty** literal has nothing to infer from, so its type comes from the position it is in: the annotation on a `let`, or the parameter it is passed to. Where nothing supplies one, that is an error naming both remedies.

A map literal's keys are expressions, which is why `{x: 1}` is *not* a map: a brace whose first entry is `identifier:` is a bare struct literal, which came first and stays. Write `{"x": 1}`, or `map_of((x, 1))` when the key really is a variable.

For a collection kept in order rather than in insertion order, there are `SortedSet<T>` and `SortedMap<K, V>`:

```
let names: Mut SortedSet<Str> = mut_sorted_set_of("pear", "apple")
add(names, "fig")
println("${to_str(names)}")        // {apple, fig, pear}
let smallest = min(names)          // and max, first_key, last_key on a map
```

They are separate types rather than a qualifier on `Set`/`Map`, because sortedness changes how a collection behaves and a qualifier could be dropped on the way into a function that relied on it. Their keys have to be **orderable** rather than hashable, which is a slightly different bar: a union can be hashed but not ordered, since comparing values of different types has no obvious meaning. Strings order by code point, the same reading Salvo takes everywhere else.

Sets and maps **iterate in insertion order**, on every backend. A `Map` iterates its *keys*, and a value is reached with `get`:

```
for name in iter(ages) {
    let age = get(ages, name)
    if age is Int {
        println("${name} is ${age}")
    }
}
```

Not every type can be a key. A key has to be hashable, which for now means one of `Int`, `Long`, `Str`, `Char` and `Bool` — `Double` and `Float` are deliberately excluded, since floating-point equality would not mean the same thing on both backends. A struct opts in:

```
struct Point canbe hashed, ordered {
    x: Int,
    y: Int
}
```

`canbe hashed` makes it a `Set` element and a `Map` key; `canbe ordered` additionally allows `<`, `<=`, `>` and `>=`, and makes it a key of the sorted collections. Both are checked where they are written: the struct may not be `canbe Mut` (a value that changed while a collection held it would corrupt that collection), and every field has to qualify too. A `List` or a tuple qualifies exactly when its elements do, comparing lexicographically.

**Equality needs no opt-in.** Every struct supports `==` and `!=`, comparing field by field:

```
let a = Point { x: 1, y: 2 }
let b = Point { x: 1, y: 2 }
let same = a == b        // true
```

Both sides must be the same type — comparing two different struct types is an error rather than a quiet `false` — and qualifiers are ignored, because equality is about the data at the moment of the check, not about what is claimed of the handle. Two things cannot be compared: a struct holding a function (no two backends agree on what equal functions are), and floating-point values, which *can* be compared but whose semantics Salvo defines itself so that both backends agree (`NaN` equals nothing, including itself).

#### Claims a list can carry

std ships three qualifiers over `List<T>`, which is where the qualifier machinery earns its keep over a container: a claim travels in the type, so a function can *demand* it instead of re-checking it.

`NonEmpty` is the one with a predicate, so it can be tested with `is` — and it is what lets `first` drop its optional:

```
let names = non_empty_list("ada", "grace")
let head = first(names)          // a `Str`, not a `Str?`

let xs: Mut List<Int> = mut_list_of()
add(xs, 7)
let seven = first(xs)            // `add` established the claim
```

That second case is a **refinement**: `add` cannot promise `NonEmpty` back (a function that mutates may not promise a qualifier it has never heard of — see "Deductions"), so the qualifier says it on `add`'s behalf. One consequence to know about: if your own qualifier also refines `add`, the two disagree and neither applies — declare `with NonEmpty` on yours and both survive.

`Sorted` is established by construction only. There is no `is Sorted`, because deciding whether a list happens to be sorted means comparing its elements, which nothing can do over an unconstrained `T` at the Salvo level:

```
let ordered = sort(list_of(40, 10, 30))     // a Sorted List<Int>
let at = binary_search(ordered, 30)         // honest only because it is Sorted

let live: Mut Sorted List<Int> = mut_sort(list_of(10, 30))
add_sorted(live, 20)                        // inserts in order, claim survives
```

`add_sorted` is the insert that *keeps* the claim: it places the element where the order survives, and says so in its own deduction clause — which it may do, unlike `add`, because it genuinely knows. Note that this `Sorted` is a different mechanic from the `SortedSet`/`SortedMap` **types**: those are a representation, this is an erased claim about an otherwise ordinary list, which is why a `Sorted List` still reaches the whole list surface.

These three are about a *list*. `NonEmpty` also means what it says about the other containers — `Set`, `Map`, `SortedSet`, `SortedMap` — because **a qualifier name may be declared over several subject types**, and the subject decides which one a use means, just as a function overload is decided by its arguments:

```
let seen: Mut Set<Str> = mut_set_of()
add(seen, "a")                       // establishes NonEmpty of Set
if seen is NonEmpty { … }

let ranked = sorted_set_of(30, 10)
if ranked is NonEmpty {
    let lo = min(ranked)             // an element, not an optional
}
```

Same name plus the *same* subject is not an overload but a replacement: your own `NonEmpty of List<T>` shadows std's, and two over one subject are a duplicate, since nothing at a use site could tell them apart.

`Distinct` says a list holds no duplicates, and comes from the one thing that can honestly promise it — a set:

```
let unique = to_list({3, 1, 3})    // a Distinct List<Int>
```

Elements have to be orderable for a `Sorted List` claim, on the same terms a `SortedSet` key does — so `Sorted List<Double>` is refused where it is written.

### Arrays

An array (`T[]`) is a fixed-size sequence, and it exists mostly for the *variadic* boundary: `...elems: T[]` is what a variadic parameter receives. Arrays have no literal syntax — `[1, 2]` builds a list — so they are constructed by function:

```
// From its elements
let numbers: Int[] = array_of(1, 2, 3)

// Generate one, size given first
let zeros: Int[] = array_by(5, i -> 0)
```

The array's size can be fetched from `numbers.size()` (see below for dot-notation of functions) and the array can be 0-indexed using `numbers[index]`. Indexing is an array's alone: a list exposes element access as `get(xs, i)`.

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
fn full_name(person: Person) -> Str => person {
    if (person.surname is Str) {
        return "${person.name} ${person.surname}"
    }
    // Explicit returns, unlike in Rust
    return person.name
}

// Elsewhere we define that `Surname Person` means that the surname is non-null, so we know we can safely extract the non-null value here.
fn full_name(person: Surname Person) -> Str => person {
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

Note that `with` is only ever this compatibility clause between two qualifiers. Declaring that a type or a type parameter _may carry_ a qualifier is a different thing, and uses `canbe` (see auto-qualifiers below, and `canbe linear` in the linear types section).

### Auto-qualifiers and `Mut`

When they're defined, structs can specify "auto-qualifiers", which are precanned qualifiers supported at the language level. `canbe Q` reads as "values of this type may be `Q`" — the declaration opts in, and individual values gain the qualifier where the language says so. At the moment, the only auto-qualifier is `Mut`, which introduces support for a mutable version of the struct, wherein each field can be modified:

```
struct Person canbe Mut {
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

`Mut` is a general language feature, not something a library defines: it composes with every other qualifier, and backends give it meaning (mutable fields in Kotlin, `mut` bindings and `&mut` references in Rust). Besides structs, other type declarations can opt into it with the same `canbe Mut` syntax — for example, the standard library's list and string types are declared as:

```
intrinsic type List<T> canbe Mut
intrinsic type Str canbe Mut
```

Applying `Mut` to a type whose declaration does not say `canbe Mut` is a compile-time error. How a backend maps a `Mut` type is described in the backends section.

`Mut` is also the one qualifier whose *removal* can cost something. Dropping a qualifier is ordinarily free — it only forgets a claim — and a `Mut List<T>` used as a `List<T>` really is the same value. But a backend may render `Mut T` as a *different type* than `T` (Kotlin's `Mut Str` is a `StringBuilder`, which is not a `String`), and there the drop is a conversion. Salvo hides that: the compiler records where a `Mut` is dropped and the backend supplies whatever conversion it needs, at every such place — arguments, returns, annotations, struct fields, union arms, interpolation and operators. Nothing in the source changes, and a `Mut Str` behaves like the `Str` it is being used as.

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

Using qualifiers and generics we can implement the equivalent of a `Result` type from Rust. `core` ships exactly this pair, so you do not have to declare it — but nothing about it is built in, and the declarations are just:

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

Conditions are boolean expressions -- `if`, `elif` and `while` accept `Bool` and nothing else. There is no truthiness: a number, a string or a possibly-absent `Bool?` is not a decision, so compare explicitly (`n != 0`, `name.size() > 0`, `flag!`). The `is` expressions we've been using for unions *are* booleans, although there is some special syntax to bind the casted value in the scope of the if block.

### When expressions

A `when` expression is always exhaustive. That is its purpose, and it happens in one of two ways depending on whether a subject is given.

**With a subject**, `when` branches on the arms of a union type. Since the arms are known at compile time, the compiler validates that every one of them is handled:

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

There is no `else` branch in this form -- the arms *are* the cases, and covering them is what the compiler checks. The subject must be a union-typed variable. The subject together with a check should form a valid boolean expression that could work for an if-expression when concatenated (i.e. `[subject] [check]` should be the boolean expression). As with if-expressions, any type/qualifier checking proven in the condition allows us to refer to the subject within that block by the more specific type. The branches of the when expression each resolve to a value like the `if`-`elif`-`else` chain. To reuse an example from earlier:

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

**Without a subject**, `when` is a chain of conditions -- an `if`-`elif`-`else` chain in `when`'s shape. The branch heads are ordinary boolean expressions, and the `else` is mandatory, since with no arms to cover it is the only thing that can make the chain exhaustive:

```
// `label` is a `Str`, not a `Str?`: every path produces a value
let label = when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    else { "positive" }
}
```

That mandatory `else` is the reason the form exists. An `if` chain without an `else` folds `None` into its value, so a chain of conditions that always produces something has to be written with a trailing `else` and read carefully to see that it does; a subject-less `when` says so in its grammar. Everything else follows from the heads being plain conditions: `is` and `^` work in them and narrow their branch, and a later branch sees the earlier ones ruled out.

```
when {
    value is Str s { println("a string: ${s}") }
    else { println("an int: ${value}") }   // `value` is an `Int` here
}
```

Note that a subject-less `when` gets no arm-exhaustiveness: writing `is` heads that happen to cover a union does not remove the need for the `else`. Use the subject form when that is what you mean.

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
let last = while i++ < numbers.size() { // ++ and -- work in both fixities
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

### Widening with `^`

`is` narrows: a successful check means the value is *more* specific than its declared type. `^` is its dual — a successful check means the value may be read as *less* specific, with the named qualifiers removed:

```
fn describe(p: Mut Person) [Console] {
    if p ^ Mut {
        read_only_report(p)      // `p` reads as `Person` here
    }
}
```

Its reason to exist is the qualified union. `Ok (Ok Int | Err Str)` is a claim *about* a union, so `when` cannot take its arms apart — they belong to the inner type. A `^` branch head tests the arm and removes the claim in one step:

```
let nested = try { wrapped(7) }        // Ok (Ok Int | Err Str) | Thrown Str
when nested {
    ^ Ok {
        // `nested` reads as `Ok Int | Err Str` here
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

- **Boolean-valued, like `is`**, and usable in the same places: `if`/`elif`, `&&`/`||`/`!`, and as a `when` branch head. A `^` branch consumes the arms it matched, so exhaustiveness works unchanged.
- **The qualifiers must be there.** Nothing to remove is an error, not a false test — `^` removes a known claim, it does not test for one (that is `is`).
- **Several at once** is allowed: `v ^ Mut NonEmpty`.
- **Some qualifiers can never be dropped**: `once` (it restricts rather than refines), `Linear` (it carries a use obligation) and `proj` (the value is derived from another). Everything else can, since dropping a claim loses only knowledge and dropping a permission loses only permission.
- **No binding form.** The subject itself reads widened, so `is Type name`'s counterpart would be redundant.

## Functions

Functions play an important role in Salvo lang:

* They define the contracts of qualifiers.
* They are central point of effects (which we will introduce in this section).
* They define iterators (which we will introduce in this section).

### Syntax

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

**A generic function's type arguments must be determined at every call.** Usually the arguments settle them (`mut_list_of(1, 2)` is a `Mut List<Int>`), but when they cannot, the *context* is consulted: the annotation on a `let`, the enclosing function's return type, or the parameter type the result flows into. If nothing determines a type argument that appears in the result type, the call is an error and you name it yourself:

```
let xs = mut_list_of()                  // ERROR: nothing says what T is
let xs: Mut List<Int> = mut_list_of()   // fine: the annotation says
let xs = mut_list_of<Int>()             // fine: written at the call
takes_ints(mut_list_of())               // fine: the parameter says
fn fresh() -> Mut List<Int> {
    return mut_list_of()                // fine: the return type says
}
```

Salvo does not look *forward* to a later use to decide a type argument, even where a target language would: what the compiler knows must be visible at the call itself. A type argument that never reaches the result type needs no context — nothing downstream could observe it.

### Overload resolution

Salvo overloads by argument type, so a name may mean several functions. Which one a call means is decided by one rule in three steps, and the rule is designed to be *predictable*: where it cannot decide, it reports an error instead of guessing, and you always have a way to say what you meant.

**1. The most specific *scope* wins.** Functions reach a call site from ever more specific places:

```
core                    // implicitly visible, least specific
explicit imports        // `import lib.describe`
this module's own declarations
the function's own scope    // fn-typed parameters, locals, implicit parameters, effect members
inner scopes                // a `rename` in a block, a local in a block
```

Only the most specific scope that has an overload *fitting the arguments* competes. So declaring your own `size(List<T>)` means calls in your module get yours — whatever the standard library declares — while `size("text")` still reaches std's, because yours does not fit:

```
fn size<T>(list: List<T>) -> Int => list {
    return 99
}

size(list_of(1, 2, 3))   // 99 — this module's
size("abcd")          // 4  — core's, the only one that fits
```

A local variable is at the function's scope, which is the most specific of all: it hides every function of that name outright.

Scope beats *signature*, deliberately — the alternative is a rule you cannot predict without knowing std's whole surface. When a more specific signature is passed over because it sits in a less specific scope, the call gets a **warning** naming both, which you silence by saying which you meant (below).

**2. Then the most specific *signature* wins**, compared per argument:

- a **type variable** says the least: `describe(Int)` beats `describe<T>(T)`;
- a **broader union** says less than a narrower one, which says less than a single arm: `Int` beats `Int | Str` beats `Int | Str | Bool`, and `Int` beats `Int?`. `Any` is the broadest type there is, so it is always last;
- **more qualifiers** say more: `Mut NonEmpty List<T>` beats `Mut List<T>` beats `List<T>`. Which *kind* of qualifier never matters — ranking `Mut` against `NonEmpty` would ask you to know more than what is in front of you;
- a **fixed** parameter list beats a variadic one, so `list_of()` picks a no-argument overload over `list_of(...elems)`.

Specificity can never exceed what the caller knows: a value whose type is `Int | Str` does not fit `f(Int)` at all, and once narrowed with `is`, it does.

The comparison is per argument, and a candidate wins only by being at least as specific in *every* argument and more specific in at least one. Nothing else takes part: not the return type, not effects, not deductions, not implicit parameters.

**3. No single winner is an error.** Two candidates that each win one argument — `mix<T>(a: T, b: Int)` against `mix<T>(a: Int, b: T)`, called as `mix(1, 2)` — rank neither way, and so do two that differ only in *which* qualifier they demand. The call is an error naming both candidates, never a coin flip. Two declarations with the same parameter *types* are a duplicate rather than an overload set, reported where the second one is written.

#### Saying which one you meant

Two ways, both compile-time only and both erased from the output.

**`@module` names the module whose overload you mean**, which overrides scope precedence and reaches past a local of the same name:

```
size@core.list(xs)      // std's, though this module declares its own
size@main(xs)           // this module's, said explicitly (and no warning)
xs.size@core.list()     // dot-notation, since `@` attaches to the name
```

**`rename` gives one overload a name of its own**, which is how an ambiguity is settled:

```
fn label(n: Even Int) -> Str => n { return "even" }
fn label(n: Small Int) -> Str => n { return "small" }

rename fn label_small = label(n: Small Int)

label(n)         // the `Even` overload — the only one still called `label`
label_small(n)   // the other one
```

A rename is not an alias: from that point on the renamed overload answers *only* to the new name. The parameter list repeats one overload's parameters exactly — same names, same types, type parameters positional — and may not mention effects, deductions or a return type, since none of them takes part in choosing an overload. A rename at module level applies to the whole module; written inside a function or a block it applies from that line to the end of that scope, loops and lambdas included. It is not importable: taking an overload out of a shared name is the consumer's decision to make.

#### Two places the same rule applies

**Passing a function by name** selects an overload from the type the position expects:

```
fn tag(v: Int) -> Str => v { return "int" }
fn tag(v: Str) -> Str => v { return "str" }

fn apply(f: (Str) -> Str, s: Str) -> Str => f, s { return f(s) }

apply(tag, "x")      // the `Str` overload: it is what `(Str) -> Str` needs
let f = tag          // ERROR: nothing here says which `tag` — annotate, or rename
```

**Filling an implicit parameter** is the same query against a type rather than an argument list, so it obeys the ladder and the ranking too — and a renamed overload no longer fills an implicit of its old name:

```
params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

// `map(p, f)` fills `next` with whichever `next` fits `p` — yours, if you
// declared one for a pass of your own, since this module beats core.
```

We have already seen some examples of functions, so now we will move to the extra bits around the arrow: effects and deductions.

### Implicit parameters

A parameter written with a `?` is one the caller does not have to pass:

```
fn sort<T>(list: List<T>, ?cmp: (T, T) -> Int) -> List<T> => list { ... }
```

At the call site the compiler fills `cmp` by looking for a function *named*
`cmp` whose type fits `(T, T) -> Int` with `T` as this call binds it. So
declaring the default for a type is just declaring a function:

```
fn cmp(a: Str, b: Str) -> Int { ... }

sort(names)                    // cmp resolved
sort(names, cmp = descending)  // or supply your own, by name
```

Nothing ties `cmp` to `Str`. There is no declaration saying "`Str` has an
ordering" — the overload whose parameters accept `Str` *is* the ordering, and
a different one is a lambda or a function reference away. Overriding is by
the parameter's own name, and the value can be either:

```
sort(names, cmp = (a: Str, b: Str) -> size(a) - size(b))
```

A bundle of related functions is declared once with `params` and spread with
`?`:

```
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn total<T>(xs: List<T>, ?Field<T>) -> T => xs {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

total(list_of(1, 2, 3))                          // 6
total(list_of(2, 3, 4), add = times, zero = one) // 24 — override one or both
```

The spread has **no name of its own**, deliberately: its members become
implicit parameters in their own right, so they are called unqualified inside
the body and overridden by their own names outside it. A `params` group is
never a value — it exists only to keep a signature short.

Details worth knowing:

- **An implicit parameter's type must be a function type.** What fills it is
  resolved as a function of that name.
- **They come last.** A normal parameter written after one could not be
  passed positionally.
- **Generic code passes its own implicits on.** Inside `total`, `T` is
  opaque, so a call to another function needing `?Field<T>` is filled from
  *this* function's implicits — matched by name and type, whatever grouping
  either side used. A generic function that declares none cannot call one
  that needs one: there is nothing to resolve and nothing to forward, and the
  error says which to add. This is the same colouring effects have, for the
  same reason.
- **Resolution is local.** Which default a call gets depends on what is
  visible where the call is written, exactly as with `use` and handlers. Two
  matching declarations make the call ambiguous, which is an error naming the
  override as the remedy.
- **An implicitly resolved function is effect-free.** A fn type without an
  effect list means "performs nothing", and an effectful function does not
  fit there — so resolution can never quietly add an effect to a caller.
- **Effect members have them too.** A member is an ordinary signature, so
  `fn show(v: T, ?fmt: (T) -> Str) -> Str` works: the call site resolves
  `fmt`, and every handler implementing `show` receives it. Handler
  *constructors* do not — their instance is built by `use`, which resolves
  nothing — and neither do lambdas, whose types have no room to declare one.
- **A mismatch that types cannot show is explained.** What a call does to
  each argument is part of whether a function fits, but not part of how a
  type prints, so a function that *consumes* an argument where the position
  keeps it is reported in words: which argument, which direction, and the two
  ways to fix it.
- **What the implicit resolves to can determine the call's type arguments.**
  Resolution runs *between* the arguments, not after them, so a variable that
  appears only in the implicit's type is still inferred. A `?Yield<It, T>`
  spread goes one step further: `T` is read off `It`'s own declaration (see
  [Iteration](#iteration-passes-and-next)), so the element type of a
  combinator is never written.

### Iterating anything: `Yield`

`params` groups are how Salvo says what a Rust programmer would say with a
trait bound. The standard library's own example is iteration:

```
params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) -> Mut List<U> => it: Mut, f {
    let out = mut_list_of<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}
```

A `for` over a **type parameter** works because of the spread: the position says
"this call supplies a `next` for `It`", which is as much of a declaration as a
`: Yield<self, T>` clause is, so the loop drives by calling that parameter and
takes the element type from its result. Nothing about `for` is special-cased for
std — this is how any combinator of your own reads.

There is no `Yield` *type* and nothing implements it: `map` needs a `next` for
whatever `it` is, and the call site supplies one. The subject is a **pass** — a
position in a sequence — so a container is iterated by writing its `iter`:

```
let doubled = map(iter(xs), double)         // a list
let letters = filter(iter("hello"), keep)   // a string's characters
let capped = map(iter(counter(3)), double)   // a pass of your own
```

A type of your own joins in by declaring an `iter` that hands back a pass:

```
struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListYield<Int> => bag {
    return iter(bag.items)
}

let total = reduce(iter(bag), 0, (acc, n) -> acc + n)
```

Inference runs *through* the group: `It` comes from the subject, and `T` — the
element type — is read from `It`'s `: Yield<self, T>` clause. That is what lets
the lambda be written bare (`n -> n * 2`) with no annotation anywhere.

`map`, `filter` and `reduce` are **eager**: they return a `Mut List<U>`.
Chaining works because a list has an `iter` like anything else. Each also has a
`List` overload, so `map(xs, f)` — no `iter` — keeps the short spelling for the
type people map most. One named variant covers the rest:

- `map_to` and `filter_to` put their results in a collection you provide, given
  first because it is what the call is about, and **hand it back** so a chain
  can carry on from it. Appending goes through an `?add` implicit parameter, so
  the destination is anything with an `add` — not just a `List`.

```
let doubled = map(xs, double)                       // Mut List<Int>
let out = map_to(mut_list_of<Int>(), iter(xs), double)
let kept = filter_to(map_to(mut_list_of<Int>(), iter(xs), double), iter(ys), is_even)
```

The destination is moved in and returned, which is what makes the nested form
work. If you want to keep hold of one across the call, rebind it:
`let sink = map_to(sink, iter(xs), double)`.

**Iterating a container consumes it**, because the pass holds it: `iter(xs)`
moves `xs` into the pass it builds. A `for` straight over the container does
not — that is data, and the backends walk it in place — so the copy is only
needed where you walk the same container twice through `iter`:
`map(iter(copy(xs)), f)`.

### Obligations: `params` groups on a type

A `params` group can also be stated as an **obligation** on a struct, with a
`:` clause between the generics and `canbe`:

```
params Step<It, T> {
    fn advance(it: Mut It) -> Emitted T | Finished => it: Mut
}

struct Countdown : Step<self, Int> canbe Mut {
    at: Int
}

fn advance(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 { return finished() }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}
```

Where `?Step<It, Int>` in a signature asks the *call site* to supply the
group's members, `: Step<self, Int>` on a declaration promises that the
members exist for this type — and the promise is checked **at the struct**:
declaring it without a visible `advance` matching
`fn advance(it: Mut Countdown) -> Emitted Int | Finished` is an error naming
that signature, where a misspelled member would otherwise surface as some
puzzling failure at a distant use site.

`self` is the shorthand for "the type being declared", written where a type
argument goes. The checker substitutes `Countdown` for it and looks for a
matching overload — by parameter types and return type, positionally, up to a
consistent renaming of type variables, so a generic struct satisfies a group
through its own type parameters. The member's parameter *names* belong to the
group; an implementation picks its own.

Note what `self` being an *argument* buys: the group itself is ordinary, so
**one declaration serves both uses**. The same `Step` spreads as
`?Step<It, Int>`, which is how a generic function reaches the member of a type
it does not know — a magic `Self` inside the group would have ruled that out,
since nothing would bind it in a signature. Outside an obligation `self` is
simply an unknown type.

Two things keep this a where-clause rather than a trait:

- **No value may have a group as its type.** `let p: Step<Countdown, Int>` is an error
  wherever a type can be written — parameter, return, field, `let`
  annotation, type argument, union arm. There is no erasure and no interface
  value; a group constrains a *named* type, and everything resolves
  statically.
- **A group is satisfied by functions, not by membership.** The obligation
  adds no scope and no dispatch: `advance` is an ordinary function, found and
  overloaded like any other. The clause only moves the check to the
  declaration.

One group is **designated**: the compiler knows `Yield<self, T>` by name and
reads the clause itself — it is what makes a type a pass, so `for` resolves
its `next` from the declaration ([Iteration](#iteration-passes-and-next)).
It is an ordinary `params` group otherwise — declared in std, spreadable
with `?`. (Linearity used to be the second designated group; it is a
declaration modifier now, `linear struct` — see [Linearity](#linearity).)

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

Lambdas can declare effects and deductions (to be discussed below), just like normal functions.

#### Effects on function types

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

### Iteration: passes and `next`

Iteration is ordinary Salvo, not a built-in protocol. A **pass** is a value
that holds a position in a sequence, and a pass is advanced by a `next`
returning either an element or the end:

```
qualifier Emitted<T> of T
struct Finished {}

params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}
```

`Emitted` is a qualifier so the element keeps its own type, which is also what
keeps the end of a sequence of optionals distinguishable: `Emitted None |
Finished` has two arms where `None | None` would have one. `Finished` is a
fieldless struct because it has nothing to qualify.

**`for` is sugar for calling `next` until `Finished`.** There is one protocol
and one lowering. A `for` over *data* — a list, an array, a string — is walked
natively by the backend; anything else is a pass, driven by its `next`.

```
for x in xs { ... }             // a list: walked in place
for c in "hi" { ... }           // its characters
for x in iter(xs) { ... }       // the pass an `iter` hands back
for pair in zip(names, ages) { ... }   // a pass of your own
```

A pass is **consumed by driving it**: the position it holds has moved, so a
second `for` over the same value is the ordinary consumed-use error. Data is
not — a `for` over a list leaves the list alone.

#### Writing a pass by hand

Declaring a `next` is what makes iteration writable
— `zip`, `merge`, anything reading two sources at once. The method alone does
not make a struct a pass: the tie is stated as an obligation, `: Yield<self,
T>`, and checked at the struct (declaring it without a matching `next` is an
error naming the missing signature). `for` reads the declaration — it does not
scan overloads for a `next` and guess:

```
struct Zip<A, B> : Yield<self, (A, B)> canbe Mut {
    left: List<A>,
    right: List<B>,
    at: Int
}

fn next<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }
```

Leave the clause off `Zip` and the `for` reports it:
``` `Zip<(A, B)>` is not iterable … (`Zip` has a matching `next` — declare
`: Yield<self, (A, B)>` on it to make it a pass) ```.

A pass that owns something declares itself `linear struct` and names its
death in its own file ([Linearity](#linearity)) — and the `for` loop is
**never** the discharge. Any *named* pass is advanced **where it lives**: the
position the loop reaches is what the owner sees next, so a function can take
two elements and leave the rest — and a linear pass is bound with `let`,
driven, and explicitly discharged after the loop (the ordinary all-paths
checking forces the `close` before every exit, `break` and early `return`
included).

```
fn take(p: Mut Slice<Int>, count: Int) -> Int => p: Mut { ... }   // keeps it

let p = slice(list_of(1, 2, 3, 4))
let first = take(p, 2)     // 1 + 2
let rest = take(p, 9)      // 3 + 4 — the same pass, carried on
```

Only a *temporary* subject (`for x in iter(xs)`) is consumed by the loop — and
a linear temporary is refused there, since the loop discharges nothing: bind
it first. A **generic** function that owns a pass which may be linear
(`<It canbe linear>`) takes its discharge as an ordinary consuming callback:

```
fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    end(it)                // the explicit terminal, on the one path out
    return sum
}
```

A linear caller passes the type's own discharger (`drain(handle, close)`); a
plain caller passes std's `drop`, the consuming no-op. Nothing is implicit:
leave the `end(it)` off and the obligation reports the leak at the exit.

#### Letting the compiler write the struct: `iter fn`

Writing the pass out is the right thing when it needs a *name* — to store it, to
hand it to a function, to zip two of them. Most passes need none of that: they
are only ever driven by a `for`. A **`iter fn`** is the same hand-written `next`
with the boilerplate removed:

```
struct Countdown {
    from: Int
}

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

let c = Countdown {from: 3}
for n in c { ... }        // 3, 2, 1
for n in c { ... }        // again: driving copied the subject
let p = iter(c)           // or hold the pass and drive it yourself
```

One declaration makes `Countdown` iterable. What the compiler writes from it is
the pass struct — the subject plus the `state` fields — and the `iter` that mints
one; both are ordinary declarations, which is why everything that works on a
hand-written pass works here.

- **The `state { ... }` block is the pass's own data**, declared exactly as a
  struct's fields are, and each initializer is evaluated **once, when the pass is
  minted**. It may read the subject and call ordinary functions; it may not
  perform effects, because minting is not where a producer's work belongs. The
  block is declarations only — it is the pass's shape, not code that runs.
- **The subject is ordinary data, and read-only inside the body.** The pass
  *borrows* it (a `proj` field, see "Projections"), so nothing is copied at the
  mint, the subject cannot be moved or mutated while a pass over it lives, and
  writing through it is the same error as writing through any immutable value.
  A pass that wants a snapshot writes one: `state { rows: List<Int> = copy(c.rows) }`.
- **The pass has no name.** No variable may be annotated with it, no field may
  store it — a pass you need to name is the written-out form above. `iter(c)`
  still hands one to you, and inference carries it, so holding one in a local
  and driving it in stages works.
- **Nothing suspends.** The body *is* the `next`: it returns on every turn, so
  there is no state machine, and effects are ordinary effects on an ordinary
  function.
- An `iter fn` must be called `next`, must take exactly one parameter, and must
  return `Emitted T | Finished` — or `Emitted (proj[from: c] T) | Finished` when
  it emits borrowed elements of its subject `c` — it is the obligation's member,
  so the subject needs no `: Yield<self, T>` clause of its own.

#### What a container hands you

A container is iterated through its `iter`, which builds a fresh pass:

```
fn iter<T>(list: List<T>) [] -> Mut ListYield<T>
fn iter<T>(array: T[]) [] -> Mut ArrayYield<T>
fn iter(str: Str) [] -> Mut StrYield
```

Each is an ordinary struct with an ordinary `next` — `ListYield` *borrows* the
list (`items: proj List<T>`) and keeps an index — so nothing about container
iteration is special-cased in the language, and nothing is copied to walk a
container. The backends keep their native loop as a fast path for a `for`
straight over a list, an array or a string, which is why *that* form neither
allocates a pass nor consumes the container.

A type of your own becomes iterable by declaring any one of three things: a
`iter fn next` (the compiler writes the pass *and* the `iter`), an `iter` that
hands back a pass (so combinators reach it), or its own `: Yield<self, T>` plus
`next` (so it *is* a pass). `for x in bag` works as soon as `iter(bag)` does —
the loop calls it once and drives what it answers.

#### There is no iterator *type*

Every pass is its own struct, so two producers have unrelated types. A position that has to hold either of two
different producers is therefore a union — `when` reads it like any other — or
a re-wrap: drive the one you have from an `iter fn` of your own and emit its
elements. Nothing is boxed behind your back, and nothing is dynamically
dispatched: an element costs an inlined call.

```
// Two producers, two types.
let ys = if fast { counter(100) } else { primes(100) }   // Counter | Primes

when ys {
    is Counter { for n in ys { ... } }
    is Primes { for n in ys { ... } }
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
fn main() [use] -> None {
    // Register the CyclicRandom as the implementation of Random<Int> for the rest of this function.
    // Attempts to register another Random<Int> will fail.
    use CyclicRandom(array_of(1,2,3,4))

    // Here we can call the function next_random()
    let num = next_random()
}
```

The effects a function depends on are declared in the square brackets before its arrow. In the above example, the `main` function (which is also the entry point to any Salvo program) starts with the special `use` effect, which is what allows it to use the `use` keyword. If a function does not declare a dependency on this, then `use` is not available to it. If the initial square brackets are not present in a function declaration, then it is assumed to be empty and that function is "pure".

An effect names a *capability*, not a type of values. It can appear in a
function's effect list and in a handler's `of` clause, and nowhere else: a
struct field, parameter, return type or `let` annotation of effect type is
a compile-time error. Handlers are not values either — `use` is the only
thing that produces one, and you reach it by calling the effect's members
rather than by holding the handler:

```
let c: Console = StdOutConsole()   // error: `Console` is an effect, not a data type
let h = StdOutConsole()            // error: `StdOutConsole` is a handler, not a value
use StdOutConsole()                // this is how you register it
println("...")                     // and this is how you use it
```

A generic handler can be registered at a type by writing it on the `use`:

```
handler Plain<T> of Show<T> { ... }

use Plain<Int>()        // registers `Show<Int>`
```

For a handler with a constructor argument the type is usually implied by it
(`use Prefixed("p")`), and writing it as well is fine as long as the two
agree — a disagreement is an error rather than one quietly winning. A handler
with *no* argument has nothing to imply it, so there the written form is the
only way.

Almost every action other than simple data transformation needs to be encoded in an effect. For example, printing to the console is managed by an effect:

```
effect Console {
    fn println(message: Str) -> None => message
}
```

Suppose we want to write a function which uses the Random and Console effects. Then we can write the following:

```
fn age_prediction(person: Surname Person) [Random<Int>, Console] -> None => person {
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

### Two effects, one member name

Different effects may declare the same member name — `close` on a file system and `close` on a network effect is the natural spelling, not a collision. A bare call resolves through whichever effect actually has a handler in scope; when more than one does, the call picks its effect with `@`, the same selector that picks a module's overload:

```
fn shut(h: Int) [Fs] -> Str {
    return close(h)          // only Fs is available here: unambiguous
}

fn both(h: Int) [Fs, Net] -> None {
    close@Fs(h)              // explicit: the Fs member
    h.close@Net()            // dot form, like any member call
}
```

The name after `@` is capitalized, which is what distinguishes an effect selector from a module path (`size@core.list(xs)`). A generic effect's instance is pinned by the call's type arguments, exactly as without the selector: `next_random@Random<Int>()`. Within a *single* effect, member names are still unique — the selector distinguishes effects, not overloads. A selected member is a call form, not a value.

### Throwing: leaving early with a message

Handlers so far always *resume*: an effect operation runs and control comes back. `throw` is the other option — it does not come back. It is declared in the core library as an ordinary effect:

```
effect Throw<M> {
    fn throw(message: M) -> Nothing => !message
}
```

A function that may throw says so in its effect list, and keeps its own return type:

```
fn parse(line: Str) [Throw<Str>] -> Int {
    if size(line) == 0 {
        throw("empty line")        // does not return
    }
    return size(line)
}
```

`throw` returns `Nothing`, the bottom type, so the code after it never runs — which is what keeps the frames in between silent. `parse` returns `Int`, not an outcome union: no `Result` plumbing, no unwrapping at each call. A function that calls `parse` either declares `[Throw<Str>]` too, passing the throw on, or delimits it.

The delimiter is `try`, a **compiler intrinsic** rather than an effect — there is no `Try` handler to register, just as there is nothing to register for loops or `if`. It evaluates to an outcome:

```
let outcome = try {
    let n = parse(line)
    n * 2
}
// outcome: Ok Int | Thrown Str

when outcome {
    is Ok {
        println("length doubled: ${outcome}")
    }
    is Thrown {
        println("could not parse: ${outcome}")
    }
}
```

Both arms are qualified, and the outcome is an ordinary union, so `is`, `when` and exhaustiveness work exactly as they do on `Ok Int | Err Str`. `Ok` is the same tag `core.result` uses; `Err` is deliberately *not* reused, because a throw is not an error value.

Details worth knowing:

- **`M` is the union of the message types the body can throw with.** A body that throws with a `Str` in one place and an `Int` in another yields `Ok T | Thrown (Str | Int)`, the same way an `if` with two branch types yields their union. With one message type it stays bare.
- **A throw lands in the innermost `try`.** There are no labelled throws; a nested delimiter takes its own body's throws and lets an outer one pass through.
- **A `try` whose body cannot throw is an error.** Nothing can produce the `Thrown` arm, so the `try` is dead scaffolding; the diagnostic says to drop it.
- **`main` cannot declare `[Throw<M>]`**: there is no caller to receive the throw, so the delimiter has to be inside.
- **`Thrown M` is forgeable, deliberately.** The qualifier carries no authority — `core.throw`'s `thrown(message)` constructor produces a value in the thrown arm without transferring control. The authority to throw is `[Throw<M>]` availability alone.
- **Nothing linear may be live across a call that may throw**: the code after the call does not run on the throw path, so the obligation would be owed on a path with no code left to discharge it. Release before the call, or move the value onward so the obligation travels with it. (`defer` used to be the third option — a block spliced at every exit, including the throw path — and it was removed 2026-09-10: linearity is what *checks* the obligation, so the construct that discharged it out of sight was the partial solution to a problem already solved.)

Both backends implement this without colouring any function the author did not annotate: Rust returns `ControlFlow<M, T>` from a function that declares `[Throw<M>]` (a throw is a plain `return`, propagation is `?`), and Kotlin throws a generated signal the innermost `try` catches.

### Deductions

A function's signature ends in a **deduction clause**: after the return type, `=>` introduces a comma-separated list of facts about what the call does to its arguments — and, for the ownership analysis behind the Rust backend, what the result holds of them. The clause may sit on the same line as the signature or start on the next; the body's `{` follows its last entry.

Suppose that we have the standard library's mutable list (`intrinsic type List<T> canbe Mut`) and a qualifier that tracks non-emptiness:

```
// Tells us that there is at least one element in the list
qualifier NonEmpty<T> of List<T>
```

If we remove an element from the list, then we don't know if it's non-empty any more. We can capture this as follows:

```
fn remove_first<T>(list: Mut NonEmpty List<T>) -> T => list: Mut
```

The entries, by shape:

| entry | meaning |
|---|---|
| `=> list` | kept, and every qualifier the argument had survives |
| `=> !list` | consumed — the caller loses the value (`list: Nothing` says the same) |
| `=> list: Mut` | **exhaustive**: afterwards `Mut` is the *only* thing still known about the argument |
| `=> list: None` | exhaustive and empty: every qualifier stripped |
| `=> list: -NonEmpty` | **delta**: drops `NonEmpty`, leaves everything else intact |
| `=> .items: proj[from: list]` | the result's field `items` projects `list` (see "Projections") |
| `=> v.items: proj[from: other]` | the call re-points the parameter `v`'s field to project `other` |
| `=> proj[from: list]` | opaque: the result *holds* a borrow of `list` somewhere inside |
| `=>[keep] !t` | a group: these entries are about the fn-typed parameter `keep` (its own parameter `t` named in its type, `keep: (t: T) -> Bool`) |

Why does an exhaustive entry drop "qualifiers this function never mentions"? Because a function that _mutates_ a value can invalidate any claim about its contents, whether or not that claim appears in its signature. A `clear` that empties a list cannot honestly promise a caller's `NonEmpty` back, even though `clear` has never heard of `NonEmpty`. So a parameter the body mutates must state exactly what survives: the bare and `-` forms are rejected there, and the compiler names the exhaustive form you want. Mutation is the only operation that invalidates a kept value — reading it cannot change its contents, and moving it ends the caller's access.

The flip side is deliberate over-strictness: `add` cannot promise to preserve `NonEmpty` either, even though appending to a list can never empty it. The function is the wrong party to ask — it has never heard of `NonEmpty` — so the claim's *owner* states it instead, in a **refinement** (see "Refinements" below). Without one, re-test with `is NonEmpty` after a mutating call.

These are enforced at each call site: passing a variable to `remove_first` above removes `NonEmpty` from what the compiler knows about it, so a second `remove_first(list)` without an intervening `is NonEmpty` check fails overload resolution:

```
let list: Mut List<Int> = mut_list_of(1, 2, 3)

// We will discuss this "predicate qualifier" later
if list is NonEmpty {
    // Type of `list` is `Mut NonEmpty List<T>`
    let first = remove_first(list) // can call this because we have `list: NonEmpty Mut`

    // At _this_ point, `list` is no longer NonEmpty, but only Mut
    let second = remove_first(list) // Invalid: there is no function for this
    let size = list.size() // Still valid, because `list` is a List<T>
}
```

In Rust, the `NonEmpty` state was not captured: this is a Salvo compile-time inference. But the fact that the function "gave back" the `list` value is captured too — a kept parameter is passed as a reference rather than moved. By contrast, a consumed parameter transfers ownership:

```
fn consume<T>(list: List<T>) -> None => !list
```

Calling `consume(list)` _moves_ the variable to the function: `list`'s type narrows to `Nothing` (a value that no longer exists is an impossibility), and any future reference to it in the calling function is a compile-time error until the variable is reassigned. This holds whether the consumption is written or inferred — a function that returns its parameter moves it, and callers are checked against that inferred contract just the same. It also holds uniformly across all types: for basic value types the underlying backends copy the value and the generated code would remain valid, but the Salvo-level contract is enforced consistently regardless of the type. The analysis is branch-aware: consuming a value in a branch that always exits (via `return`, `break`, or `continue`) does not affect the code after the branch, while a value consumed on only some fall-through paths is conservatively unusable afterwards. Loops account for the back edge too: a value read early in a loop body and consumed later in the same body is an error, since the read happens after the consumption from the second iteration onwards (reassigning before the body ends keeps it valid).

Consuming calls are not the only way a value moves. Every other escape route consumes a bare variable the same way, and the error at a later use names the event: storing it in a struct, array, or tuple literal (the literal owns it now), spreading it (`...n` reads all of its fields into a new value and consumes the source), returning it, `break`-ing with it, and passing it to a `use` handler constructor (the handler stores it for the rest of the scope). A `break` with a value reaches the code after the loop on every exit path, so a variable consumed by `break` is unusable after the loop even when the `break` sits inside a branch. Reads, by contrast, never consume anything — in particular, string interpolation is a read: `"${n}"` formats the value and retains nothing, so `n` stays usable. As always, `copy(...)` at the move site keeps the original usable, and reassignment revives it.

**Write what inference cannot reach; the rest is inferred.** A parameter the clause does not mention gets the contract the compiler reads off the body — kept with the qualifiers that survive every call the body makes, or consumed when the body moves it — so most functions write no clause at all, and a clause may be *partial*: `=> list: Mut` on a three-parameter function says nothing about the other two. What is written is checked against the body (a promise to keep what the body moves is an error) and is otherwise fixed. Two places have no body to infer from and must therefore say everything: an `effect` member (including a `platform effect`'s) and an `intrinsic fn` must mention every parameter — except Copy scalars (`Int`, `Bool`, …), whose fate is nothing to deduce. A function *type* is bodiless too but keeps the default of keeping everything; `=>[f] …` on the enclosing declaration is how to say otherwise, and it needs the fn type's parameters named (`f: (v: List<Int>) -> Int`).

```
// Nothing written: `list` is inferred `Mut` (because `remove_first` might be
// called on it) — the hover shows `=> list: Mut`.
fn maybe_remove_first<T>(list: Mut NonEmpty List<T>) [Random<Int>] -> T? {
    if next_random() > 0 {
        return remove_first(list)
    }
    return None
}
```

Inference is the strictest deduction over every function the body hands the value to (moves included), computed to a fixpoint across the program. The price of inferring is that a body edit can change a contract callers depend on with no signature change; the hover always shows the effective clause, and `!p` can be written where a move is meant to be part of the contract.

### Why returning a parameter is a move

A parameter that is kept compiles to a *borrow* in Rust: the caller retains its value. A function's return value, by contrast, is *owned* by the caller unless the signature says otherwise. If a function returns one of its parameters as an owned value, these two facts collide, and Salvo resolves it without a hidden clone: returning a parameter transfers ownership out through the return channel, and the parameter is deduced as _moved_ — the caller that passed it in loses it. The same applies to the other escape routes — storing a parameter in a struct, array, or tuple literal, or passing it to a consuming call. Consequently, a written clause cannot promise a parameter back when the body returns it owned: `=> x` with `return x` is a compile-time error.

The way to give a caller access to a parameter's data *without* moving it is to return a **projection** of it — `-> proj[from: x] T` — which is a borrow, described next. This is purely a constraint of the Rust backend — the Kotlin backend ignores deductions, since everything is a garbage-collected reference on the JVM — but one Salvo codebase must compile to both, so the checker enforces the stricter contract everywhere.

Binding a parameter with `let` is *not* on the move list: it creates a *shared fate* link instead (see "Shared fate and `copy`").

### Projections: borrowing without copying

Salvo has no references in the source, but it has one qualifier that means "this value is borrowed from somewhere": **`proj`**. It is how the standard library reads an element out of a list, walks a list, or filters one without copying anything — and how you write such a thing yourself. The principle behind it (user decision 2026-09-11) is that **a copy never happens without the program opting in**: `copy(x)` where you want one, a `_to` function that fills a destination you provide, and nothing else.

**`proj` is part of the type.** A projected value's type says so — `proj Str`, `Mut List<proj Str>`, `Emitted (proj Str) | Finished` — in diagnostics, on hover, and through generics: matching `Emitted T` against an `Emitted (proj Str)` binds `T = proj Str`. An owned value satisfies a projected position (it can do strictly more), never the reverse — and the projection is never dropped silently. What a projection may be *passed to* follows from what the callee does with the parameter: a callee that only reads a kept, non-`Mut` parameter accepts a top-level projection even where the parameter is written owned (a borrow read in place is indistinguishable from the value), while a callee that **consumes** the parameter, **mutates** it, or expects the projection **nested** inside the type (a union arm, a type argument — where the two are genuinely different types in Rust) refuses it with an error naming which of the three stood in the way and the remedies (write the `proj`, or pass `copy(...)`). The one exception is Copy scalars: `proj Int` *is* `Int` — the number is the value itself on both backends. In overloading, an owned position beats a projected one, the way one arm beats its union: `proj` accepts more, so it says less.

**A projected value.** `proj[from: p] T` on a result says the value *is* a borrow of the parameter `p` — an element of it, a field, the whole of it. It may appear wherever a type does: the whole result (`-> proj[from: xs] Person`), a nullable (`-> (proj[from: list] T)?`), a union arm (`-> Emitted (proj[from: p] T) | Finished`), a tuple element. Several sources are written together, and a projection joined across branches is of all of them:

```
fn either(a: List<Int>, b: List<Int>, flag: Bool) -> proj[from: a, b] List<Int> {
    if flag { return a }
    return b
}
```

Three rules follow from "it is a borrow":

- **It is read-only, whatever its `Mut` says.** `proj Mut X` is a legal type — the value came out of a mutable slot — but a `proj` value never satisfies a `Mut` position: `Mut X` is usable where `proj Mut X` is expected, not the reverse. `copy(x)` is the way out, and yields a `Mut X` of your own. For the same reason a *parameter* cannot be written `proj Mut X` — the `proj` promises to accept borrows and the `Mut` refuses every one of them — and the compiler says so at the declaration; write `proj X` (which accepts `proj Mut X` arguments) or `Mut X`.
- **It shares fate with its source.** The caller's result is linked to the argument: mutating or moving the source poisons the projection, and moving the projection itself needs `copy` (a Copy scalar excepted: an `Int` read out of a list is the number itself on both backends, so it moves for free).
- **The body must deliver it.** Every value the function returns must derive from a named source (a projection, element or alias of it) or be `None`; a source must be a kept parameter.

**A view: an owned value that holds borrows.** A struct may declare fields as `proj`, written without a source — the struct says *that* it projects, each literal says *what*:

```
struct ListYield<T> : Yield<self, proj T> canbe Mut {
    items: proj List<T>,   // borrows the list it walks
    at: Int                // its own position
}

fn iter<T>(list: List<T>) -> Mut ListYield<T> {
    return Mut ListYield<T> { items: list, at: 0 }
}
```

Such a struct is an ordinary owned object: its `Mut` is real (a pass is advanced in place), its non-`proj` fields are its own, and it may be moved, stored, or passed on. What it may not do is outlive what it borrows. The compiler tracks this as shared fate too: `let p = iter(xs)` links `p` to `xs`, so `xs` cannot be moved or mutated while `p` is alive — and nothing had to be written on `iter`, because **which parameters a result holds borrows of is inferred from the body**: the literal stores `list` in a `proj` field, so `iter` lends `list`. Where there is no body — an effect member, an intrinsic, a fn-typed parameter — the clause says it: `=> proj[from: list]` ("the result holds a borrow of `list`"), `=> .items: proj[from: list]` (this field does), or on a fn type `=>[iter] proj[from: c]`. A written entry must name every lend the body performs; it may name more (a generic body lends through opacity the analysis cannot see). Returning a view rooted in a *local* is an error: the local dies with the call.

A container can hold borrows too: `List<proj T>` is a list of projected elements, and it is what `filter` returns:

```
fn filter<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) -> Mut List<proj T> => it: Mut, proj[from: it], keep
```

The result holds borrows of whatever the pass walks — nothing is copied — and it lives no longer than the source. For a list of your own, `filter_to(dest, it, keep)` copies each kept element into `dest`, and it says so twice: in its `_to` name, and in the `?copy` implicit it takes so that the copy is the element type's own.

**A view of a temporary.** A view's source must outlive it, so binding, returning or storing a view of a temporary is an error: `let p = iter(list_of(1, 2))` dies at the end of its statement (the diagnostic says to `let` the list first). *Using* one within the statement is fine — `map(iter(list_of(1, 2)), f)`, `for x in iter(list_of(1, 2))` — the temporary lives that long on both backends.

**A capturing lambda is a view.** A lambda's body can hand out projections rooted in a *capture* — `indices.map(i -> all.get(i)!)` returns elements of `all`, and no function type can say so (`proj[from: …]` names parameters; captures have no name). So the closure itself carries the fact: it holds a borrow of every non-Copy variable it reads from the enclosing scope, exactly as a struct holds its `proj` fields. Binding the lambda links it to those variables, a call result built from the lambda is linked through it, and moving or mutating a captured variable poisons the closure — the same discipline every view lives under. A lambda that captures nothing holds nothing (`map(p, w -> w)` binds freely), a Copy scalar capture is the value itself, and a capture the body *consumes* is owned by the closure rather than borrowed — that is the lambda that becomes `once`.

**Passes borrow.** A pass that walks data declares `: Yield<self, proj T>` and its `next` returns `Emitted (proj[from: p] T) | Finished` — the element is a borrow of the pass, which borrows the source — while a generator (a countdown, a random stream) declares `: Yield<self, T>` and emits owned values. The two must agree: an obligation at `proj T` with an owning `next`, or the reverse, is an error. Reading combinators accept both. An `iter fn`'s generated pass borrows its subject the same way (`__subject: proj Subject`), so nothing is copied at the mint; an `iter fn` that wants a snapshot writes `copy(...)` in a `state` initializer.

**Re-pointing.** A mutable view may be made to project something else: a function that does so says which field and from what, `=> v: Mut, v.items: proj[from: other]`, and the caller's variable at `v` becomes linked to `other` from the call on.

**What Rust makes of it.** A projected value is `&T` (or `Option<&T>`, `Union2<&T, Finished>`); a view struct carries a lifetime (`ListYield<'s, T>`), a lent parameter ties it (`iter<'a, T>(list: &'a Vec<T>) -> ListYield<'a, T>` when elision cannot); `List<proj T>` is `Vec<&T>`. Kotlin, where everything is a reference already, changes nothing — the rules are what keep the two backends printing the same thing.

### Shared fate and `copy`

Salvo has no references, but variables can still overlap: `let m = n` and `let name = person.name` both make a new name for data that another variable already owns. Salvo tracks this as **shared fate**: when one variable is bound to the value or a projection of another — by `let`, assignment, destructuring, a `for`-loop binding, or an `is`/`when` binding — the new variable becomes *derived from* its source, transitively down to the ultimate root. Reading either variable is always fine; reads never consume anything. Operations that need *ownership* of the data are governed by the binding's **mode**, which the compiler infers from how the derived variable is used later:

- **Borrow-mode** (a derived variable that is only ever read): reads flow freely and every ancestor stays usable. Mutating, moving, or reassigning a **root** poisons every variable derived from it — the derived values may no longer exist, so using one afterwards is an error naming both the link and the event. Reassigning a poisoned variable revives it.
- **Move-mode** (a derived variable that is later moved or mutated): the binding *takes ownership* — every ancestor is consumed at the binding itself, and using an ancestor afterwards is an error naming the binding. From the binding on, the variable is the value's independent owner. This is what makes zero-copy consuming pipelines legal: the whole chain of bindings hands the value along, and the Rust backend emits real moves with no clones.
- Move-mode needs every ancestor to be *owned* by the function. Locals always are. A parameter is owned when the function's deductions move it — and when its entry is inferred, a move-mode binding reaching a parameter *claims* it: the parameter becomes moved, and callers hand over ownership. A **written** entry that keeps the parameter pins it as borrowed instead: moving or mutating anything derived from it stays a compile-time error — you cannot move out of a borrow — and the remedy is `copy`.
- The same ownership rule applies to a **projection in a moved position** — passing `h.tags` to a call that consumes it, or storing it in a literal. If the projected data is mutable, the move consumes the owner (`h` is unusable afterwards) or, for a kept parameter, is an error with the `copy` remedy. Projections of immutable data are free: whether a backend copies or shares immutable data is unobservable.

The escape hatch is one word: the standard library's `copy` duplicates a value, leaving the source untouched and producing a fresh value with no links. Its parameter says `proj T` because a projection is exactly what `copy` is *for* — and an owned value satisfies a projected position too, so `copy` of anything works. The result is un-projected one level with the rest kept: `copy` of a `proj Mut Str` is a `Mut Str` of your own.

```
fn copy<T>(value: proj T) -> T => value   // intrinsic: each backend implements it
```

Here is the discipline at work, together with the deduction contract. With a *written* entry that keeps `persons`, moving a derived value out is an error:

```
fn longest_name(persons: Person[]) -> Str => persons {
    let longest = ""
    for person in persons {                       // `person` derived from `persons`
        if longest.size() < person.name.size() {
            longest = person.name                 // `longest` derived from `person` (and `persons`)
        }
    }
    return longest        // ERROR: `longest` shares its fate with `persons`,
                          // which this function promised to keep (`=> persons`)
}
```

One fix is a single copy at the escape point — one copy for the whole function instead of one per iteration:

```
    return copy(longest)
```

The other fix is to *not* promise the parameter back: drop the written entry, and the compiler infers that the pipeline consumes `persons` — the bindings become move-mode, the function demands ownership from its callers, and the whole thing compiles with **zero copies** (in Rust: the argument moves in, the loop iterates by value, the field moves out, the result moves up):

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
let xs = mut_list_of(1, 2, 3)
let ys = xs          // ys derived from xs (borrow-mode: ys is never moved/mutated)
add(xs, 4)           // mutates the root...
size(ys)             // ERROR: ys shared xs's fate and xs was mutated

// Move-mode: `ys` is mutated later, so the binding takes ownership.
let xs = mut_list_of(1, 2, 3)
let ys = xs          // ys takes ownership: xs is consumed here
add(ys, 5)           // fine: ys owns the value
add(xs, 4)           // ERROR: ys was bound from xs and later moves the value

let zs = copy(ys)    // an independent duplicate
add(zs, 6)           // fine, and ys is untouched
```

**Lambdas follow the same discipline.** A lambda's relationship to the variables it captures is read off its body, and binds when the closure is created (a closure may run any number of times, so its contract cannot wait for the call): captured immutable values are free; a captured mutable value that the body only *reads* links the closure to it — the variable stays usable, but mutating it poisons the closure; a captured mutable value that the body *mutates* is consumed at creation — the closure owns it now (`copy` first to keep the original); and a lambda can never *consume* a capture, since every run after the first would use a moved value (`copy` inside the lambda instead).

```
let xs = mut_list_of(1, 2)
let f = (n: Int) -> { return n + size(xs) }   // reads xs: closure linked to it
apply(f, 1)          // fine
add(xs, 9)           // mutates the root: f is poisoned
apply(f, 1)          // ERROR: f shared xs's fate and xs was mutated

let g = () -> { add(xs, 1) }   // mutates xs: g takes ownership at creation
size(xs)             // ERROR: xs was consumed by the lambda; copy first
```

**Function types carry contracts.** A higher-order function can state what the function it receives does to its arguments, using the same deduction entries as ordinary signatures, scoped to the parameter as a group on the enclosing declaration — name the fn type's parameters in the type, then write `=>[f] …`: `fn apply(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v` demands a function that *consumes* its argument (so `f(data)` consumes `data`, and calling it twice with the same value is an error), while no group — or `=>[f] v` — demands one that *keeps* it (call it as often as you like; the caller keeps the argument). An unannotated function type keeps everything, and a group may not be written inline inside the parameter list. A lambda checked against a keeping contract cannot consume its parameters (`copy` if needed), and a named function passed by value is checked with its real deductions — a consuming function never sneaks into a keeping position (the reverse is fine: keeping more than required never hurts). On the Rust backend this decides the physical calling convention — borrowed argument types for keeping contracts, owned for consuming, `&mut impl FnMut` for the function value itself — while Kotlin's aliases need no change.

A lambda that goes further and *consumes* a capture is allowed, but its type changes: it becomes a **`once` function** — callable at most once. `once` says a value may be *used* at most once, and it may be written on **any** type: the bound is a restriction the holder imposes on itself, demanding nothing of the type's author. What using means depends on the type. A function (`fn run(f: once () -> None)`) is used by calling it: the compiler rejects a second call, a call inside a loop, or a call after the value has been passed along, and a `once` function never fits a plain fn position (which could call it repeatedly). A data value (`once Ticket`) is used by consuming it — and since a plain-typed holder can consume at most once anyway, `once Ticket` may be handed to an ordinary consuming `redeem(t: Ticket)`: that is its one use. Any ordinary value can be used where a `once` one is expected (you may always promise to use something less often). On the Rust backend a `once` parameter compiles to `FnOnce`; on the JVM the restriction is enforced by the compiler alone.

One more ordering rule: **arguments are evaluated left to right**, and within a single call a later argument cannot mention a value an earlier argument consumed — `f(a, a)` where both parameters move, or `f(a, size(a))`, are errors at the second argument (`copy` at the consuming argument is the remedy).

Some consequences worth knowing:

- **Values from calls are independent — unless they project.** `copy(x)` and most function results carry no links. A function that returns a *projection* of a kept parameter — `fn first<T>(list: List<T>) -> proj[from: list] T?` — or a value that *holds* one (a pass over a list) hands the caller something that shares fate with the argument: mutating the collection poisons it, moving it out needs `copy`. See "Projections" below; on the Rust backend these are real borrows, which is what makes the standard library's `first`, `get`, `iter` and `filter` zero-copy.
- **The analysis is flow-aware** like consumption: links merge across branches (linked on any path means linked), survive loop back edges, and reassignment severs a variable's own links while poisoning its previous derivatives. A `for`-loop binding is fresh each iteration: consuming it inside the body is fine.
- **It is uniform across all types** — an `Int` derived from an `Int` follows the same rules — and **purely static**: on the JVM nothing physically prevents the rejected programs. The discipline is what lets each backend choose the cheapest correct representation with no observable difference: Kotlin shares references throughout; Rust emits real moves for move-mode bindings and clones for borrow-mode ones (real borrows are a later stage).
- **Fields are tracked apart.** A link records *which projection* of the value it came from, and an event only reaches what it could actually have changed: reading `p.name` while `p.tags` is mutated is fine, and so is the reverse. What overlaps still poisons — the same field, a *prefix* of it (mutating `o.inner.tags` invalidates a value derived from `o.inner`), the whole variable (a `Mut` argument or a reassignment reaches every field), and an array element reached by a computed index, since `xs[i]` and `xs[j]` cannot be told apart. A derivation the compiler cannot spell as a projection chain is treated as the whole value.

```
let p = Person {name: "ann", tags: mut_list_of("x")}
let n = p.name           // derived from p.name
add(p.tags, "y")         // mutates p.tags — a different field
println(n)               // fine: the mutation could not have touched p.name

let t = p.tags
add(p.tags, "z")         // mutates the very field `t` came from
size(t)                  // ERROR: t shared p.tags's fate and p.tags was mutated
```

  Moving a field out is tracked the same way: the field leaves, the rest of the value stays. What left is remembered, so reading *that* field back is an error, and the value can no longer be handed on whole — but its other fields are still readable, and putting the field back with an assignment makes the value complete again.

```
let p = Person {name: "ann", tags: mut_list_of("x")}
eat(p.tags)              // consumes the field
println(p.name)          // fine: a different field
size(p.tags)             // ERROR: `p.tags` was moved out of `p`
take(p)                  // ERROR: `p` cannot be used as a whole — part of it is gone

p.tags = mut_list_of()  // puts it back
take(p)                  // fine again
```

  A parameter the function promised to keep is the exception, and it is the same rule as everywhere: you cannot take something out of a value you do not own. `copy` is the remedy.

### Copy semantics per backend

`copy` is an `intrinsic fn` (see the Backends chapter): its declaration gives the checker everything it needs — the argument is kept with all its qualifiers, the result is independent — and each backend lowers calls to it against the argument's *actual type*. Where no Salvo operation could mutate the value anyway, a copy is free: Kotlin emits the argument unchanged (duplicating a reference to immutable data is a copy), and Rust clones. Where mutation is possible, the copy is real on every backend: `Mut List<Int>` becomes `xs.toMutableList()` in Kotlin and `xs.clone()` in Rust; a `Mut` struct with immutable fields becomes `p.copy()` / `p.clone()`. Where a backend cannot yet produce a correct copy (for example, nested mutability like `Mut List<Mut Person>` on the JVM, where a shallow copy would share the inner values), the compiler reports an error rather than emit code that behaves differently across backends.

### Linear types: values that must be used

Everything above makes values *affine*: they can be used at most once. Resource types want the other half too — a file handle that is never closed, a transaction that is never committed or rolled back, is a bug. A type declares **linearity** with a modifier, and its file supplies the death:

```
linear struct FileHandle {
    fd: Int
}

fn close(handle: FileHandle) -> None => !handle {
    // release the resource
    discard(handle)
}
```

The **discharge set** is every function declared in the *type's own file* that consumes a parameter of the type — `close` for a file, `stop` *or* `join` for a thread handle, `remove(cache, entry)` for a pooled one (the extra parameters are ordinary parameters). A `linear struct` whose file has no such function is an error at the struct: the obligation would have no legal death. Leak diagnostics name the whole set. `discard(handle)` is the obligation's terminal, legal **only** inside a discharger — and a discharger gets no exemption: its own body must terminate the obligation on every path, by `discard` or by forwarding into another discharger (`fn shutdown(t: Thread) => !t { stop(t) }`).

Every value of a linear type carries an **obligation**: on every path, it must be *moved onward* before it goes out of scope. Moving is anything the ownership system already recognizes — passing it to a consuming call (`close(handle)`), returning it, spreading it, a move-mode binding handing it to a new owner. Each move transfers the obligation with the value: a function that receives a linear value by move must discharge it in turn; a function that *keeps* (borrows) a linear parameter leaves the obligation with its caller; a derived (fate-linked) variable is an alias and carries no obligation of its own.

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

There is no escape hatch: `discard(x)`, which deliberately drops an ordinary value, refuses a linear one and names its `close` — dropping a handle is exactly the leak the obligation exists to prevent.

```
fn deliberate() {
    let h = open("data.txt")
    discard(h)                  // ERROR: `discard` cannot drop a linear value; call `close(h)`
}
```

The `maybe_leak` shape above — a value that must be released however the block ends — is written out: `close(h)` on each path. The compiler names the path you missed, which is the whole point; there is no construct that discharges an obligation implicitly (`defer` did, and was removed 2026-09-10 for exactly that reason).

```
fn no_leak(flag: Bool) {
    let h = open("data.txt")
    if flag {
        close(h)
        return                  // fine: this path releases it
    }
    close(h)                    // and so does this one
}
```

Rules that keep the obligation sound:

- **Linearity is declared, not applied**: `linear` cannot be written in a use-site type — every value of a `linear struct` type is linear, always. (A spelling you could forget would defeat the point.) `canbe linear` on a declaration is an error naming the modifier; on a *type parameter* it keeps its spelling, where it means something else entirely (below).
- **A composite may not hold a linear value** — for now. Storing a handle in a struct field, an array, a tuple, a union arm or a type argument (`List<FileHandle>`) is an error *at the store*, rather than moving the obligation into the container: a linear value lives only in a local, a parameter or a return value. This is an interim rule; carrying an obligation through a container is one design question together with conditional linearity ("a `Box<T>` is linear exactly when `T` is"), and until that is answered the compiler refuses rather than guesses. The store is refused wherever it is written — the field's declaration, the literal, or a call like `add(list, handle)` that would put a bare value into a container.
- **Generics opt in per type parameter**: an unconstrained `T` cannot be instantiated with a linear type, but a function may declare `fn hold<T canbe linear>(value: T) -> T` — the same `canbe linear` phrase as on type declarations, now opting the *function's handling* in. Inside the body, `T` values are treated as linear (they must be discharged on every path); in exchange, callers may instantiate `T` with linear types, and an opted `T` forwarded to another generic requires that one to be opted too. The standard library's collection surface is audited and opted where sound (`list`, `mut_list_of`, `add`, `size`), but that means only that those functions may be *called* with a linear `T` — putting one *into* a `List<T>` is refused by the composite rule above. `get` stays out (it returns an alias of an element, which would duplicate the obligation) and `copy` refuses linear values outright. `discard`'s declaration is simply `intrinsic fn discard<T canbe linear>(value: T) -> None => !value`. One extra rule: a linear value cannot be passed in a *variadic* position (those are untracked).
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
    return list_of(first, ...rest)
}
```

**This one is real.** `NonEmpty`, and the constructor above, are declared in std's `core.list` — so is `Sorted`, and `Distinct` in `core.set`; see "Collections".

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

These four declarations are `core.result`, which is implicitly visible like the rest of `core`, so real code writes only the last function. They are shown here because there is nothing privileged about them: a domain-specific pair of tags is declared exactly the same way.

Constructive qualifiers can also be entirely handled by the backend implementation. We will discuss this more in the section on backends.

### State and provenance

The qualifiers above answer *how* a value gained a tag — by predication or by construction. A second, independent question is what the tag is a claim *about*, and that one changes the semantics:

* A **state** qualifier is a claim about the value's **contents**: `NonEmpty`, `Sorted`, `Positive`. This is the default, and it is what `qualifier Q of T` declares.
* A **provenance** qualifier is a claim about where the **handle** came from: `Authenticated`, `Validated`, an id minted for one particular struct. Declare it by prefixing `provenance`.

```
qualifier NonEmpty<T> of List<T>              // about the contents
provenance qualifier Authenticated of Request // about the origin
```

The difference matters because a function that mutates a value invalidates claims about its contents. A call that takes `Mut` and does not promise to keep `NonEmpty` strips it, since adding or removing elements could make it false. But mutating a request's body does not change the fact that the request was authenticated, so provenance qualifiers survive every call:

```
fn touch(r: Mut Request) [] -> None => r: Mut {
    r.body = ""
}

fn f(r: Mut Authenticated NonEmpty Request) -> None {
    touch(r)
    // `NonEmpty` is gone here -- `touch` may have invalidated it
    // `Authenticated` is still known -- mutation cannot un-authenticate
}
```

Three further rules follow from provenance being about the handle rather than the data:

* **It is mint-only.** No inspection of the bits can tell you where a value came from, so a provenance qualifier has no body: no `qualifies` predicate, and no field overrides. Values gain it from constructor functions, exactly like a constructive qualifier, and `x is Authenticated` on a non-union value is a compile error.
* **It composes freely.** Two state qualifiers must declare `with` compatibility, because both constrain the same contents. An origin is orthogonal to contents and to other origins, so provenance tags stack without any declaration, including several at once: `Authenticated FromCache Request`.
* **It is droppable, and it survives storage.** Forgetting where a value came from is always safe, so `Authenticated Request` can be passed wherever a plain `Request` is wanted; and a struct field typed `Authenticated Request` keeps the tag for whoever reads it back.

Both kinds are erased in the generated code — the subject only decides what the compiler knows. If you want a distinct type at runtime (its own identity, its own equality, usable as a distinct map key), use a one-field struct instead; a `Str` wrapped in a provenance qualifier stays a string, which is usually what you want for ids.

`Mut`, `Linear`, `once` and `proj` are also claims about a handle rather than its contents, but they are compiler intrinsics rather than qualifiers you can declare: each one changes how code is generated, or how the ownership analysis treats a value. The rule of thumb is that a permission can be forgotten (`Mut Person` is usable as `Person`) while an obligation cannot (`Linear` and `once` never drop).

### Refinements

A deduction clause is written by the function's author, so it can only state what that author knows. `add` mutates its list, so it may not promise a caller's `NonEmpty` back (see "Deductions") — even though appending to a list can never empty it. The function is not the party that can fix this: it has never heard of `NonEmpty`.

The party that can is the qualifier. A **refinement** is a statement about a function you do not own, written by the qualifier whose claim it is about:

```
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}
```

`refn` is deliberately narrower than `fn`. It has no body, it cannot declare effects, and it cannot declare a return type — a refinement never changes what a function *does*, only what is *known* about the arguments afterwards. Its deduction entries can only add (`+Q`) and remove (`-Q`) state qualifiers; a plain name (which would mean "only this survives") is the function's own deduction to make. The parameter list is there to pick one overload, so it repeats the parameters exactly: same names, same types, with type parameters matched by position.

At a call site the refinement has the last word. `add`'s own `=> list: Mut` drops everything it does not name, and then `+NonEmpty` puts the claim back:

```
let xs: Mut List<Int> = mut_list_of()
add(xs, 1)
// `xs` is `Mut NonEmpty List<Int>` here, so this resolves:
let n = count(xs)
```

A refinement is **trusted**, exactly as `-> T as Q` is: nothing proves that `add` establishes `NonEmpty`, and no runtime check is emitted. The qualifier's author owns the claim's meaning, which is why they are the right party to ask.

**Refinements travel with their qualifier.** A refinement declared inside `NonEmpty` applies wherever `NonEmpty` is in scope, and nowhere else — you opt into the refinements by opting into the qualifier. A qualifier may only speak about its *own* claim: `NonEmpty` cannot state what a call does to `Sorted`.

**When refinements disagree, none of them apply.** Two qualifiers can both claim a call establishes them, and their claims can be mutually exclusive:

```
qualifier Q1<T> of List<T> { ... refn something(list: List<T>) => list: +Q1 }
qualifier Q2<T> of List<T> { ... refn something(list: List<T>) => list: +Q2 }
```

Merging these would ask for `+Q1 +Q2`, which is impossible when neither declares `with` the other. That is not an error — the program still compiles, and `something` is simply a less useful function — but the compiler warns, because a refinement you imported silently doing nothing would be impossible to diagnose otherwise. You then have two remedies: test the property yourself with `is` after the call (always possible, since these are state qualifiers), or state the reconciled result in your own module with a top-level `refn`:

```
// In your own module: replaces the qualifiers' refinements for `list`.
refn something<T>(list: List<T>) => list: +Q1
```

A top-level refinement is module-scoped and not importable. Reconciling is the consumer's call — a library shipping its own reconciliation would just move the disagreement one level up.

Finally, a refinement reaches **inferred** deductions, so the fact does not die at one frame:

```
// No written list: `NonEmpty` survives the call to `add` because of the
// refinement, so `refill` promises it back to its own callers.
fn refill<T>(list: Mut NonEmpty List<T>, value: T) -> None {
    add(list, value)
}
```

Two limits are worth knowing. A refinement contributes to an inferred deduction only for a qualifier the parameter itself declares — it can put back what a call dropped, never invent a claim the signature never made — and only when the refined call is unconditional in the body, since a call inside an `if` or a loop may not run at all. A refinement's documentation is merged into the refined function's, so the language server shows what `add` establishes *here* even though the statement lives elsewhere.

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

Every name written in a type position must resolve to a declaration in scope — base types (structs, `type` declarations and aliases, `intrinsic type`s, effects) and qualifiers alike. A name that resolves to nothing is an error naming the name, wherever it is written: a signature, a struct field, a `let` annotation, an `of` type, a `canbe` or `with` clause, or an `is` / `when` check. The diagnostic lists the modules that would bring the name into scope, if any.

This matters most for qualifiers, because a qualifier and a base type sit next to each other in the same syntax. `Ok Int` with no `Ok` in scope is not a type built from an unknown claim — it is a typo or a missing import, and it would otherwise fail much later and much less obviously, as a check that "can never succeed" against arms tagged with a qualifier the compiler never heard of. A name that *does* exist but in the other namespace says so, since no import can fix it:

```
qualifier Tag of Int

fn f(x: Tag) -> Int { ... }  // error: unknown type `Tag` (`Tag` is a qualifier, not a type)
```

Nothing is taken on trust. Besides written names, every *member* you reach
for must be justified by a declaration the compiler can see: a call to a
function nobody declared, a field on something that is not a struct (or a
field a struct does not have), `[]` on something that is not an array, and
`for` over something not iterable are all compile-time errors. There is no
"pass it through and let the target language sort it out" — a value's type
tells you exactly what you can do with it.

This is what makes interop itself a matter of *declaration*: a Salvo
program reaches its target language through a `platform effect`, whose
member functions you declare and the compiler turns into an interface for
the host to implement (see the backends section) — there is no way to name
a Kotlin method or a Rust function that some declaration in scope does not
already stand for. Dot-notation still reads like a method call
(`text.shout()` is `shout(text)`), but the function has to exist. The same
applies to generics: a type parameter has no bounds, so nothing is known
about a `T` — reading `value.name` inside `fn f<T>(value: T)` is an error,
not a promise about the values you will pass in.

The one thing that *is* lenient is a type the compiler could not work out
for itself: it stays unknown, compatible with everything, so a single
mistake produces a single error instead of a cascade of follow-on
complaints.

### Naming rules

Casing is part of the language, not a convention:

* **Types start with an uppercase letter** — structs, qualifiers, type declarations and aliases, effects, handlers, and generic parameters.
* **Values do not** — functions, parameters, fields, variables, bindings, and lambda parameters.
* **Module paths are lowercase.** Since a module path is its file path, that applies to file and directory names too: `Utils.sv` is a compile-time error telling you to rename the file.
* **A file name may not contain a dot.** A module's path comes from where the file sits in the directory tree, and a dot in the name would read as a path separator that no directory backs — so `string.sv` is the module `string`, while `string.kotlin.sv` is rejected. There are no per-backend companion files; both targets are served from the one source.

This is what lets the compiler tell a type from a value at the start of a dotted name, which the next section relies on.

**Every name must be declared.** A reference to something nothing declares is an error, not a passthrough to the target language — the same rule as calls, field reads and subscripts. Scopes are not hoisted either, so reading a variable above its `let` is that same error. A name that *is* declared but is not a value says which it is:

```
let s = Person       // error: `Person` is a struct type, not a value:
                     //        construct one (`Name { … }`)
```

**A variable that is never read is a warning.** Assignment does not count as a read — a variable only ever written to has no reader, which is the mistake worth reporting. Prefix the name with `_` to say the omission is deliberate:

```
let spare = compute()    // warning: `spare` is never used;
                         //          prefix it with `_` (`_spare`) if that is deliberate
let _ignored = compute() // no warning
```

Parameters are exempt: a signature often dictates them, and a handler member implementing an effect cannot drop one.

### Namespaced names

Structs and qualifiers can be declared with a *dot-name* `Ns.Name`, where `Ns` is a struct in the same file. This gives you the wrapper-type pattern — distinct types for the strings and ids hanging off a struct, so that an incomplete refactor is a type error instead of a silently mis-wired value — without nesting declarations:

```
struct Environment {
    id: Environment.Id,
    name: Environment.Name
}

struct Environment.Id {
    value: Str
}

struct Environment.Name {
    value: Str
}

// Qualifiers can be namespaced too
qualifier Environment.Tag of Str
```

Dot-names work in every type position (annotations, `of` types, `is` checks, `as` constructors, deduction clauses) and in struct literals:

```
let env = Environment {
    id: Environment.Id {value: "prod"},
    name: Environment.Name {value: "Production"}
}
```

Three rules apply:

* `Ns` must be a struct **in the same file**, and it must not be generic.
* A dot-name has exactly two segments: `A.B.C` is an error, and a dot-named struct cannot itself be a namespace.
* Nothing else visible in the file may be called `NsName` — the concatenation. The Rust backend renders `Environment.Id` as `EnvironmentId`, so that spelling has to stay free.

Importing works either way: import the member directly, or import the namespace struct and get its members with it.

```
import env.types.Environment.Id     // just the member
import env.types.Environment        // the struct *and* Environment.Id
```

Backends differ, deliberately. Kotlin emits a nested class, so `Environment.Id` is the same name in the generated code. Rust concatenates, because Rust modules and structs share one namespace and a `mod Environment` next to a `struct Environment` would not compile.

### Documentation comments

Salvo has no separate doc-comment syntax. A `//` comment block sitting directly above a declaration *is* that declaration's documentation: the comment on the line immediately above it, plus every consecutive comment line above that. One blank line ends the block, which is how you keep an unrelated remark unrelated. A comment sharing its line with code documents nothing.

Docs are **markdown**. The `//` and one following space come off; everything after that is passed through, so emphasis, inline code, lists and fenced blocks work as written, and a bare `//` line is a paragraph break.

`[symbol]` in a doc comment references a name: a parameter, generic or field of the declaration being documented, or a type, qualifier, effect, handler or function declared in the program. A reference that resolves becomes a link to the declaration; one that does not is left exactly as written, so brackets in prose are safe.

Structs document their fields individually — the comment above a field belongs to that field, and tooling shows the struct's own docs followed by a list of its fields. The same goes for anything else declared inside a declaration: an effect's or handler's member functions, and a handler's state.

```
// A person we know about.
//
// Only [name] is required; [surname] may be absent, and a
// [Person] is never partially built.
struct Person {
    // Their given name.
    name: Str,
    // Their family name, when we know it.
    //
    // Absent for people who go by one name.
    surname: Str? = None,
    age: Int
}

// Describes a [person] in one line.
//
// Reads [Person]'s fields directly:
//
// - `name` always
// - `surname` when present
fn describe(person: Person) -> Str {
    return person.name
}
```

The language server shows these on hover — for a declaration, for a *use* of it, and for anything nested inside one: hovering a field, wherever it is written, shows that field's own documentation and says which struct declares it. It also shows a variable's type as it is *known at the position you hover* — narrowed by any `is` test or `when` arm you are inside, qualifiers included, with the declared type named below when the two differ.

Three things it adds beyond the declaration text. A **`params` group** hovers with its members, since those are the point of it. A **predicate qualifier** shows the condition it holds under when its `qualifies` is a single `return` — just the expression, so `Positive` reads as "Holds when `int > 0`." A longer body is hidden, and its doc comment explains it instead. And a variable that **shares fate** with another says so, naming what it was derived from and where, down to the field (`p.name`, not all of `p`) — with the reminder that reads are free, that moving or mutating it is rejected, and that `copy` makes an independent value. Names reached through an `import` hover like local ones, on the import line itself as well as at each use — as do the group name in a struct's obligation clause (`: Show<self>`) and the qualifier in an `is` check (`i is Positive`). A function's hover also says **where it came from** — the module of the overload that actually won, named the way an `@module` selector would spell it, since with scope-based overloading the signature alone does not tell you which `size` you are looking at.

## Backends

One of the aims of Salvo is to make it easy to integrate Salvo code with the backend code. To achieve this, the Salvo compiler builds an internal representation (in Rust), and passes this on to the configured backend implementation to write out the relevant target source code. In order to support this, we distinguish between two layers: the `intrinsic` layer, which is the compiler's, and the `platform` layer, which is yours. The core library is entirely intrinsic; everything an application needs from its target language is a platform effect.

### Intrinsic

The `intrinsic` layer sits in a backend specific module inside the compiler. This handles complex language-specific logic, and core functionality: how to encode union types, what the `None` type transpiles to in different cases, how to pass parameters to functions, how function naming works, how imports are handled, and more. These can only be changed by making changes to the compiler itself. Anything involving syntax will appear here, and all `intrinsic` backend definitions are declared as part of the standard library (defined in `std`).

`intrinsic` is the standard library's alone. Customer code cannot declare one, because there would be no lowering in any backend to give it meaning — an `intrinsic` with no compiler support behind it is a promise nothing keeps. Application code reaches the target language the other way, through a `platform effect`; that is the single interop path. This is also the one exception to a plain structural rule: a top-level `fn` must have a body and a `type` must have a definition (`= ...`). There is no bodyless declaration form for customer code — `intrinsic` (and the bodyless `intrinsic handler`) is precisely what lets the standard library state a contract the compiler fulfils in place of one.

For example, the basic types (`Int`, `Str`, `List<T>`, ...) are declared as `intrinsic type`s, and each backend maps them natively:

```
intrinsic type Str
```

When building the compiler, _all_ `intrinsic` declarations must be handled by _every_ backend module.

Functions can be intrinsic too. An `intrinsic fn` carries the signature and deductions the checker uses and has no body; each backend lowers calls to it directly, seeing the resolved argument type at every call site. That is what makes type-directed lowering possible where one generic template could not express it — the standard library's `copy` is the canonical example:

```
intrinsic fn copy<T>(value: proj T) -> T => value
```

A backend that does not implement an intrinsic fn, or cannot lower it for a particular argument type, reports a compile-time error — never wrong code.

The standard library's collection and string surface (`list`, `mut_list_of`, `add`, `get`, `first`, `size`, `iter`, and the string functions from `char_at` to `split`, `trim`, `join` and `parse_int`) is intrinsic for the same reason, which is what keeps `size(xs)` compiling to `xs.size` in Kotlin and `(xs.len() as i32)` in Rust rather than to a wrapper function nobody wants. Because the lowering sees the *resolved* declaration, the three `size` overloads — on `Str`, on `List<T>`, and on an array — are three separate lowerings rather than one template guessing from arity.

Handlers can be intrinsic as well: `intrinsic handler StdOutConsole of Console` is bodyless in Salvo, and each backend emits a real class or trait impl for it.

The `Mut` auto-qualifier is also handled at this level: a type declaration can opt into it with `canbe Mut` (`intrinsic type List<T> canbe Mut`), and each backend decides what `Mut` means. Kotlin maps `Mut List<T>` to `MutableList<T>` and `Mut Str` to `StringBuilder` — the one place a qualifier survives erasure — while Rust maps both `List<T>` and `Mut List<T>` to `Vec<T>`, and both `Str` and `Mut Str` to `String`, because there mutability shows up in bindings and references instead. Where the two types really differ, a *drop* of the `Mut` is a conversion (`.toString()` for a Kotlin builder) and the compiler records the drop for the backend to render; where they do not — `MutableList<T>` is a `List<T>` — it renders nothing.

### Platform

The `platform` layer is where an application reaches its target language. Where `intrinsic` is the compiler's, `platform` is yours: you declare what you need from the host, and the compiler generates an interface for the host to implement.

A platform declaration is an *effect*:

```
platform effect Telemetry {
    fn record(name: Str, value: Int) [] -> None => name, value
}
```

That is an ordinary effect in every respect except where its implementation comes from. A function that records telemetry declares `[Telemetry]`, its callers declare it too, and the value threads through exactly as any handler would:

```
fn work(n: Int) [Telemetry] -> Int {
    record("work", n)
    return n + 1
}
```

Grouping the functions under an effect, rather than declaring them one at a time, is what makes the interop boundary something you choose: one effect for telemetry, another for storage, each generating its own interface. It also gives the implementation somewhere to keep state and dependencies, because the host constructs it.

The compiler generates the interface next to the rest of the emitted code — `interface Telemetry` in Kotlin, `pub trait Telemetry` in Rust — and nothing else. There is no handler to write in Salvo, and writing one is an error: the host's implementation *is* the handler.

Because the instance is constructed outside the Salvo program, it cannot be registered with `use`. It arrives as a parameter instead, and that changes who owns the entry point: a `main` that declares a platform effect is emitted as `salvoMain` (Kotlin) or `salvo_main` (Rust), taking one parameter per platform effect it declares, and the *host's* `main` constructs the implementations and calls it.

You do not write that file from scratch. `salvo platform generate` writes it for you:

```bash
salvo platform generate --backend kotlin --src ./my_project
```

The host code lives in a `platform/` directory at the root of your sources, mirroring the source layout: `platform/main.kt` implements the platform effects declared in `main.sv`, `platform/app/entry.rs` those of `app/entry.sv`. Both languages can sit side by side in the same tree — a build only ever picks up the extension of the backend it is compiling for — so one source tree stays buildable for both targets.

The generated file is a skeleton: one class per platform effect, implementing the generated interface, with every member stubbed, plus the `main` the toolchain will run:

```kotlin
// platform/main.kt, as generated
package salvo.platform.main

import salvo.main.*

class TelemetryHost : Telemetry {
    override fun record(name: String, value: Int) {
        TODO("implement Telemetry.record")
    }
}

fun main() {
    salvoMain(TelemetryHost())
}
```

Fill in the bodies and `salvo run` works. The file is generated **once**: run the command again and it reports that the file exists and leaves it alone, because from that point on it is yours. Forgetting to run it at all is an ordinary compile error that names the command — a program whose `main` needs a platform effect has no entry point without a host.

This is the point of the design: because the interface is generated and the implementation is real target-language code, the target's own compiler checks the two against each other. Add a member and the implementation fails to compile until you write it; remove one and the leftover override fails; change a signature and the mismatch is a type error. Nothing needs to be validated by Salvo, and nothing can drift silently — which is also why the generator never has to touch the file twice.

A Salvo handler may *depend* on a platform effect, which is how a handler written in Salvo reaches the host:

```
handler AuditLogger(telemetry: Telemetry) of Logger {
    fn log(message: Str) -> None => message {
        record(message)
    }
}
```

Two restrictions follow from the host implementing one concrete interface: neither a platform effect nor its members may be generic. Member names may be shared with other effects like any effect's (`close@Fs(…)` picks — see "Two effects, one member name"), but within the platform effect itself each member name appears once.

## Specific backend details

### Kotlin

* When `None` is the only return type of a function, it should be translated to `Unit`.
* The backend should define generic union type wrappers using a sealed interface. If the larger union type is of size N, then the backend should define union types for each number from 1 to N. The qualifier checks then reduce down to checking which of the sealed types a value results in.
* Effects and handlers can map to interfaces and implementations of those interfaces. The effects are passed to a function as the first arguments of that function, and all uses of those effects is mapped to the relevant parameter name.
* `Mut Str` maps to `StringBuilder`, which — unlike `MutableList<T>` — is *not* a subtype of the immutable form, so dropping the `Mut` emits `.toString()`. `copy` of a `Mut Str` is `StringBuilder(sb)`, not the identity.

### Rust

* When `None` is the only return type of a function, the return type is omitted (`()`).
* `T?` maps to a physical `Option<T>`; union types map to generated enums (`Union2<T1, T2>` with one variant per non-`None` arm).
* Deductions determine ownership: a parameter that appears in a function's deductions is passed by reference (`&T`, or `&mut T` when its declared type carries `Mut`), while a parameter omitted from the deductions is moved (passed by value) — the calling code no longer has access to it in Salvo, so the move is always legal. Copy scalar types are always passed by value.
* Effects map to traits with `&mut self` methods; effect dependencies become leading `&mut dyn` parameters, and `use` instantiates a handler into a local that is threaded as `&mut local`.
* `Str` and `Mut Str` are both `String`, so dropping a `Mut` emits nothing. String indexes are *characters*, not bytes, on both backends, so the lowerings convert where Rust counts bytes.
* See BACKEND_SPEC.rust.md for the full rules.