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

* `Byte`: an 8-bit unsigned integer (0..255), equivalent to `UByte` in Kotlin and `u8` in Rust. A `Byte` is an octet rather than a number: the operators do not take it, and `to_int(b)` / `to_byte(n)` convert (`to_byte` keeps the low 8 bits).
* `Bytes`: a **byte buffer**, and the currency of every byte-shaped API in `std` — deliberately not a `List<Byte>`, which would box every octet on the JVM and carry a list's surface rather than a buffer's. Like `Str` it comes in two shapes: `Bytes` is read (`size`, `get`, `slice`, `index_of`, `to_str`, `to_hex`, `str_of_bytes`, and `for b in data`) and `Mut Bytes` is built (`add`, `append`, `set`, `clear`), reaching the read surface by dropping its `Mut`. `bytes_of(...)` and `mut_bytes(...)` construct one; `to_bytes(str)` and `str_of_bytes(data)` bridge to text, in UTF-8 and decoding strictly.
* `Int`: a 32-bit signed integer, equivalent to `Int` in Kotlin and `i32` in Rust.
* `Long`: a 64-bit signed integer, equivalent to `Long` in Kotlin and `i64` in Rust.
* `Float`: a 32-bit floating point number, equivalent to `Float` in Kotlin and `f32` in Rust.
* `Double`: a 64-bit floating point number, equivalent to `Double` in Kotlin and `f64` in Rust.
* `Bool`: a boolean, equivalent to `Boolean` in Kotlin and `bool` in Rust.
* `Char`: a character, equivalent to `Char` in Kotlin and `char` in Rust.
* `None`: a singleton type, used to represent expressions with no response.

All numbers include the usual arithmetic operations, on **numeric operands only**: `+`, `-`, `*`, `/` and `%` work on `Int`, `Long`, `Float` and `Double` (and unary `-` on the same). `/` between integers is integer division on every backend. `+` does not concatenate strings — `${}` interpolation is how text is built — and `&&`, `||` and `!` take `Bool` operands only: Salvo has no truthiness in value position any more than in conditions. Ordering (`<`, `<=`, `>`, `>=`) and equality (`==`, `!=`) are **capabilities**, not built-in operations: `a < b` means `cmp(a, b) < 0` and `a == b` means `eq(a, b)`, so they work wherever those functions exist. Numbers, and the other intrinsic types, come with theirs (see [Comparison, equality and hashing](#comparison-equality-and-hashing)).

`x += e` and its three siblings (`-=`, `*=`, `/=`) are shorthand for `x = x + e`, so they follow the same operand rules — `+=` on a `Str` is refused like `+` is — and they are statements rather than expressions, so they do not chain. `x++` and `x--` remain for a step of one, where the value of the expression matters. An assignment's left side must be a *place*: a variable, a field path, or a subscript.

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
set(text, 0, 'H')          // answers whether it wrote; ignorable
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

A pattern has to match what it destructures: the value must be a tuple, and a tuple of exactly that many elements. `let (a, b) = 7` and `let (a, b) = (1, 2, 3)` are both errors that say which. A loop binding is a pattern too, checked against the element type:

```
for (name, score) in iter(rows) {
    println("${name}: ${score}")
}
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

The subject can be a call rather than a variable, and then it is evaluated **exactly once** — per iteration, for a `while`, which is what makes the take-until-empty loop the natural way to drain a collection:

```
while remove_first(queue) is Ticket next {
    redeem(next)                    // one `remove_first` per turn
}
```

Because the binding and the test are two reads of one subject, a call subject is only allowed where that single evaluation has somewhere to live: as the whole condition of a `while`, or of an `if`'s first branch. Inside a `&&` chain or an `elif` condition it is an error naming the remedy — bind it with `let` first — since hoisting it there would run it when short-circuiting says it should not. A variable or field chain is unrestricted, because reading one twice costs nothing.

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

A guard usually wants the *negative* test, and that is `!is`:

```
fn describe(s: Str?) -> Str {
    if s !is Str {
        return "nothing"
    }
    return s                    // `s` is `Str` here
}
```

`x !is Q` is exactly `!(x is Q)` — the same check, spelled the way a guard reads
— so it works wherever an `is` does, `&&`/`||` and `when` heads included. It
binds nothing and takes no `^`: a failed test tells you nothing about the value
on the branch it guards, so there is nothing to name or to lift.

`?:` is `!` with a path instead of a panic. It picks the non-`None` side of its
subject, and its right side runs only when the subject is `None`:

```
fn greet(p: Person) -> Str {
    let nick: Str = p.nickname ?: "friend"
    return "hello ${nick}"
}
```

The right side is an ordinary expression, so it can also *leave*, which is the
shape that removes the most code:

```
fn shout(p: Person) -> Str? {
    let nick: Str = p.nickname ?: return None
    return "${nick}!"
}
```

A right side that leaves contributes nothing to the type, so `nick` above is a
plain `Str`.

`?.` is the other half: it reaches a field, or a dot-notation function, on the
non-`None` side, and hands back an optional of its own.

```
struct Address { city: Str, zip: Str? = None }
struct Person  { home: Address? = None }

let city: Str? = p.home?.city        // `Str` field, so `Str?`
let zip: Str?  = p.home?.zip         // already `Str?`, so still `Str?`
let shout: Str? = p.home?.city?.to_upper()
let shown: Str = p.home?.city ?: "-"
```

A qualifier is picked by naming it. `Ok?:` picks the `Ok` arm keeping its tag;
`^Ok?:` picks it and lifts the tag, exactly as `is ^Ok` does:

```
fn doubled(text: Str) -> Ok Int | Err Str {
    let n: Int = parse(text) ^Ok?: return _
    return ok(n * 2)
}
```

`_` is the **unpicked** side, tags and all — `Err Str` here, so the right side
can hand it to something that expects an error. That is the one place the
placeholder is needed: a plain `?:` leaves `None`, which is already writable.
There is no way to strip a tag here, so turning a `Thrown` into an `Err` is a
`when`.

A `?:` whose right side leaves also **narrows its subject** below, because the
value must have been there for the code to be running:

```
let v: Str = t ?: return        // `t` reads as `Str` from here on
let n: Int = r ^Ok?: return     // `r` reads as `Ok Int` — the arm, tag and all
```

The pick narrows to the arm rather than to the lifted value: the lift applies to
what the expression produced, while the variable still holds the tagged arm. A
right side that yields a value narrows nothing, since that is the path where the
subject was absent.

The subject is evaluated once, so it can be a call. A pick that matches no arm,
or that matches every arm and leaves the right side unreachable, is an error.

Because the result carries a `None` arm, each link re-tests and the chain reads
left to right. The receiver has to be a variable or a field of one: the form
reads it twice, once to ask and once to reach the member, so a call belongs in a
`let` first.

The operator keys on a `None` arm rather than on the `?` spelling, so it works on
any union with one: `Str | Int | None` picks `Str | Int`. Its precedence is
Kotlin's — tighter than `is` and comparison, looser than arithmetic, and
right-associative — so `count ?: 0 > 3` reads as `(count ?: 0) > 3` and
`a ?: b ?: 0` tries each in turn. A subject with no `None` arm is an error:
nothing could take the right side.

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

Taking something back **out** of a collection is a move, not a copy: `remove_first(list)` and `remove_at(list, i)` answer `T?` — the element, or `None` when there is nothing at that position — and `remove(map, key)` does the same for a map's value. `replace(map, key, value)` is the write that hands back what it displaced. None of them leaves a hole or duplicates anything, which is what lets a collection hold values that must be used exactly once (see "Linear types"); for ordinary data they are simply the operations you would expect. `drain(list, each)` consumes a collection and hands every element to a function, in order.

A list has no positional *write*, on purpose: when the index misses, the value written has nowhere to go. What it has instead is `swap(list, i, j)`, which exchanges two elements — nothing enters, nothing leaves, so nothing can be dropped. Every operation that takes an index **answers rather than failing**, and identically on both targets: a read is an optional, and a *write* — `swap`, or `set` on a `Mut Str` or `Mut Bytes` — is a `Bool` that is `false` when the index is out of range, in which case nothing was written. Ignore the answer where the index is known good. A negative index is out of range rather than counted from the back, on every surface.

For a collection kept in order rather than in insertion order, there are `SortedSet<T>` and `SortedMap<K, V>`:

```
let names: Mut SortedSet<Str> = mut_sorted_set_of("pear", "apple")
add(names, "fig")
println("${to_str(names)}")        // {apple, fig, pear}
let smallest = min(names)          // and max, first_key, last_key on a map
```

They are separate types rather than a qualifier on `Set`/`Map`, because sortedness changes how a collection behaves and a qualifier could be dropped on the way into a function that relied on it. Their keys have to be **orderable** rather than hashable, which is a slightly different bar: a union can be hashed but not ordered, since comparing values of different types has no obvious meaning. Strings order by code point, the same reading Salvo takes everywhere else. **What counts as one member** is named per container, and the two answers differ. A `Set` and a `Map` are **`eq`-distinct**: they bucket by `hash` and confirm the bucket hit by `eq`, which is why those two travel together. A `SortedSet` and a `SortedMap` are **`cmp`-distinct** — two keys are one member when `cmp(a, b) == 0`, and equality plays no part. That is not a shortcut: it is what both target platforms do, and it is the only rule a sorted container can implement. So a set sorted by age keeps one person per age, by definition; if you want "same rank" and "same person" in one program, hold both containers.

The four keyed containers name the ordering or the hash they keep their keys by — `SortedSet<T>(?cmp)`, `Set<T>(?hash, ?eq)` — and a slot nobody writes is resolved by its own name, the way an implicit parameter already is, so `Set<Str>` means what it always did. Naming a *different* one is accepted by the language and not yet by either runtime: a keyed container has to carry the function at run time, and neither host's container has a slot for one, so that is refused with a message saying so rather than compiled into something that ignores it.

Sets and maps **iterate in insertion order**, on every backend. A `Map` iterates its *keys*, and a value is reached with `get`:

```
for name in iter(ages) {
    let age = get(ages, name)
    if age is Int {
        println("${name} is ${age}")
    }
}
```

Not every type can be a key. A key has to be hashable, which for now means one of `Int`, `Long`, `Str`, `Char` and `Bool` — `Double` and `Float` are deliberately excluded, since floating-point equality would not mean the same thing on both backends. A struct joins in by *having the functions*:

```
struct Point : auto Ordered<self>, auto Hashed<self> {
    x: Int,
    y: Int
}
```

`auto Hashed<self>` generates the `hash` and `eq` that make it a `Set` element and a `Map` key; `auto Ordered<self>` generates the `cmp` that gives it `<`, `<=`, `>` and `>=` and makes it a key of the sorted collections. Both are checked where they are written: the struct may not be `canbe Mut` (a value that changed while a collection held it would corrupt that collection), and every field has to qualify too. A `List` or a tuple qualifies exactly when its elements do, comparing lexicographically. See [Comparison, equality and hashing](#comparison-equality-and-hashing) for the whole story, including hand-written implementations.

**Equality is opt-in, and compares field by field when generated:**

```
let a = Point { x: 1, y: 2 }
let b = Point { x: 1, y: 2 }
let same = a == b        // true
```

Both sides must be the same type — comparing two different struct types is an error rather than a quiet `false` — and qualifiers are ignored, because equality is about the data at the moment of the check, not about what is claimed of the handle. A struct holding a *function* cannot have the generated `eq` (no two backends agree on what equal functions are), which is exactly the case for writing one by hand that ignores the field. Floating-point values compare with semantics Salvo defines itself so that both backends agree (`NaN` equals nothing, including itself).

#### Claims a list can carry

std ships three qualifiers over `List<T>`, which is where the qualifier machinery earns its keep over a container: a claim travels in the type, so a function can *demand* it instead of re-checking it.

`NonEmpty` is the one with a predicate, so it can be tested with `is` — and it is what lets `first` drop its optional:

```
let names = list_of("ada", "grace")
let head = first(names)          // a `Str`, not a `Str?`

let xs: Mut List<Int> = mut_list_of()
add(xs, 7)
let seven = first(xs)            // `add` established the claim
```

That second case is a **refinement**: `add` cannot promise `NonEmpty` back (a function that mutates may not promise a qualifier it has never heard of — see "Deductions"), so the qualifier says it on `add`'s behalf. One consequence to know about: if your own qualifier also refines `add`, the two disagree and neither applies — declare `with NonEmpty` on yours and both survive.

`Sorted` is established by construction only. There is no `is Sorted`, because deciding whether a list happens to be sorted means comparing its elements, which nothing can do over an unconstrained `T` at the Salvo level:

```
let ordered = sort(list_of(40, 10, 30))     // a Sorted<Int>(cmp) List<Int>
let at = binary_search(ordered, 30)         // honest only because it is Sorted

let live: Mut Sorted List<Int> = mut_sort(list_of(10, 30))
add_sorted(live, 20)                        // inserts in order, claim survives
```

`add_sorted` is the insert that *keeps* the claim: it places the element where the order survives, and says so in its own deduction clause — which it may do, unlike `add`, because it genuinely knows. Note that this `Sorted` is a different mechanic from the `SortedSet`/`SortedMap` **types**: those are a representation, this is an erased claim about an otherwise ordinary list, which is why a `Sorted List` still reaches the whole list surface.

The claim **names the ordering it was sorted by**, the way any structure that holds an ordering does (see "A structure that holds an ordering"): orderings are plural, so "sorted" alone would not say enough — an `add_sorted` under a different `cmp` than the sort used would insert at a position that means nothing in the order the list is actually in. `sort` publishes what it resolved, and the two operations that read the claim capture it:

```
fn by_len(a: Str, b: Str) -> Int { return cmp(size(a), size(b)) }

let words = sort(list_of("pear", "fig", "Apple"), cmp = by_len)   // Sorted<Str>(by_len)
let found = binary_search(words, "kiwi")                          // asks by length: 1
```

Two lists sorted differently are two types and refuse to mix, and a signature that wants either writes the claim without an ordering (`xs: Sorted List<T>`) — an argument nobody wrote is *unconstrained*, so the value keeps carrying its own. A `let` annotation reads the same way, which is why the `Mut Sorted List<Int>` above still knows what sorted it.

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

### Any and Never

Like Kotlin, there is an `Any` type which is the superclass of all other types. There is also a `Never` type which represents unreachable code (similar to `!` in Rust or `never` in TypeScript). In the type system, it is treated as the sub-type of every other type so that the types of branching code work out nicely.

In the following example, the `return` "evaluates" to `Never` while the `Ok` branch evaluates to type `Str`. Thus, the type of `value` is compatible with `Str` (technically, the type of the when expression is `Ok Str`, but the code explicitly elides the `Ok` qualifier).

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

`Never` is the type of `return`, `break` and `continue`, which are
**expressions**, not statements. Usually that makes no visible difference — an
escape sits on a line of its own, as it does above — but it means one can stand
wherever a value is expected, which is what lets a guard be written inline:

```
let step: Int = if i < limit { i } else { break }
```

A value follows an escape only on the same line, so `return` before a newline or
a `}` is still the value-less form. `break` and `continue` leave a *loop* rather
than the function, so neither satisfies "this function always returns" the way a
`return` does.

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

Note that on a qualifier declaration `with` is only ever this compatibility clause between two qualifiers. Declaring that a type or a type parameter _may carry_ a qualifier is a different thing, and uses `canbe` (see auto-qualifiers below, and `canbe linear` in the linear types section). The same word appears in one other place, where it cannot be confused with this one: after a `use` or `spawn`, `with` names the dependency instances to supply (see "Inheriting the scope, and overriding it with `with`").

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

`Mut` is also the one qualifier whose *removal* can cost something. Dropping a qualifier is ordinarily free — it only forgets a claim — and a `Mut List<T>` used as a `List<T>` really is the same value. But a backend may render `Mut T` as a *different type* than `T` (Kotlin's `Mut Str` is a `StringBuilder`, which is not a `String`), and there the drop is a conversion. Salvo hides that: the compiler records where a `Mut` is dropped and the backend supplies whatever conversion it needs, at every such place — arguments, returns, annotations, struct fields, union arms, interpolation and operators. Never in the source changes, and a `Mut Str` behaves like the `Str` it is being used as.

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
provenance qualifier Ok<T> of T
provenance qualifier Err<T> of T

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

### Lifting a qualifier: `is ^Q`

`is` narrows: a successful check means the value is *more* specific than its declared type. Marking a qualifier with `^` inside the check makes it do the dual as well — the arm is still tested, but the claim is **lifted**, so the value reads as *less* specific afterwards:

```
fn describe(p: Mut Person) [Console] {
    if p is ^Mut {
        read_only_report(p)      // `p` reads as `Person` here
    }
}
```

Its reason to exist is the qualified union. `Ok (Ok Int | Err Str)` is a claim *about* a union, so `when` cannot take its arms apart — they belong to the inner type. A `^` branch head tests the arm and removes the claim in one step:

```
let nested = try { wrapped(7) }        // Ok (Ok Int | Err Str) | Thrown Str
when nested {
    is ^Ok {
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

- **It is an ordinary `is`**, so it works everywhere one does: `if`/`elif`, `&&`/`||`/`!`, and as a `when` branch head, where it consumes the arms it matched so exhaustiveness is unchanged. The runtime test is the same; `^` decides only whether the claim survives into the branch.
- **The qualifiers must be there.** Nothing to lift is an error, not a false test — `^Q` lifts a known claim, it does not test for one (that is a plain `is Q`).
- **All or none.** `is ^Mut ^NonEmpty` lifts both; a check mixing lifted and kept qualifiers (`is ^Ok NonEmpty`) is an error, since the two say opposite things about one check.
- **Some qualifiers can never be lifted**: `once` (it restricts rather than refines), `Linear` (it carries a use obligation) and `proj` (the value is derived from another). Everything else can, since dropping a claim loses only knowledge and dropping a permission loses only permission.
- **A binding is allowed**, and it is what makes a lift of *several arms at once* possible:

  ```
  let outcome: Ok Int | Ok Str | Err Str = classify(input)
  if outcome is ^Ok value {
      // `value` is `Int | Str`; `outcome` keeps its declared type here
      print_either(value)
  }
  ```

  Without a binding the lifted value has to be re-read out of the subject, and one such read cannot stand for two arms that live in different wrapper positions — so a multi-arm lift without a binding is an error naming this remedy. With a single arm, a binding is optional: the subject itself reads lifted, as `p is ^Mut` does above.

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

### Comparison, equality and hashing

Three capabilities are declared exactly this way, in `core.compare`:

```
params Ordered<T> { fn cmp(a: T, b: T) -> Int }
params Eq<T>      { fn eq(a: T, b: T) -> Bool }
params Hashed<T>  { fn hash(value: T) -> Long
                    fn eq(a: T, b: T) -> Bool }
```

`Hashed` is a **pair**, and says so: a hash container buckets by `hash` and
confirms the bucket hit by `eq`, so a `hash` without its `eq` is useless and a
pair that disagrees is a container that loses values. `Ordered` is deliberately
not a pair — no sorted collection consults equality, since it decides membership
by `cmp` — which is what lets one program hold a `Set<Person>` keyed by every
field beside a heap ranked by age, without either being a lie.

A group that spreads two overlapping capabilities asks for the shared member
once: `?Eq<T>, ?Hashed<T>` brings one `eq`. And a spread asks for *every* member
it declares, whether the body uses it or not — so a function that only needs an
ordering writes `?cmp: (T, T) -> Int` rather than a group.

So "orderable" is not a property of a type — it is *an ordering being in
scope*. `cmp` answers a negative number when `a` sorts first, zero when the two
tie and a positive number otherwise; `eq` decides equality; `hash` digests.
Each is an ordinary overloadable function, which is the whole mechanism: no
trait, no interface value, nothing either backend knows about.

The intrinsic types come with theirs — `cmp` for `Int`, `Long`, `Byte`, `Char`,
`Bool` and `Str`, `eq` for those plus `Double` and `Float`, `hash` for the ones
`cmp` covers:

```
let order = cmp("ab", "b")      // negative: strings compare by code point
let yes = eq(1.5, 1.5)          // true
```

`Double` and `Float` have an `eq` and no `cmp`: `NaN` ties with nothing, so
there is no total order to promise — the same reason a sorted collection of
them is refused.

A type of your own joins in by declaring the function — and the *canonical* one
for a type is written `@`-scoped to it, in the type's own file:

```
struct Person { name: Str, age: Int }

fn cmp@Person(a: Person, b: Person) -> Int {
    return cmp(a.age, b.age)
}
```

`cmp@Person` is an ordinary function: `cmp(p, q)` and `p.cmp(q)` both reach it.
The `@` adds two things. It **travels with the type** — a module that imports
`Person` gets its canonical too, so "orderable" arrives with the type instead of
depending on which of its file's names you happened to import — and it is what
an implicit `?cmp` resolves to by default. `export` on it is explicit and must
match the type's: a public type with a private canonical is a compile error, not
a silent hole.

Because a canonical is *the* implementation for its type, an ambiguity around
one is never settled by scope. If another `cmp` also fits, the call says which
it means, with the same selector the declaration was written with:

```
let by_age = cmp@Person(ada, bob)      // the canonical
let by_name = cmp@main(ada, bob)       // this module's own
let younger = min_of(ada, bob, cmp = cmp@Person)
```

A type can also promise the capability at its declaration,
`struct Person : Ordered<self>`, which checks there that a matching `cmp`
exists rather than failing at some distant use.

For the everyday case — compare the fields, in order — you do not write the
bodies at all. `auto` asks the compiler for them, on the function:

```
struct Point { x: Int, y: Int }

auto fn cmp@Point(a: Point, b: Point) -> Int
auto fn eq@Point(a: Point, b: Point) -> Bool
auto fn hash@Point(value: Point) -> Long
```

An `auto fn` has no body: the compiler writes one from the fields. Ordering is
lexicographic by field declaration order (so field order is significant), and
everything generated is structural, so a generated `cmp`, `eq` and `hash` agree
with each other by construction.

On an obligation clause, `auto` is shorthand for exactly those declarations —
one per member of the group:

```
struct Point : auto Ordered<self>, auto Hashed<self> { x: Int, y: Int }
```

The point of `auto` being a modifier on the function is that a type can mix.
Suppose `Person` should be ordered by name, with equality meaning "same rank":

```
struct Person : Ordered<self>, Hashed<self> { name: Str, age: Int }

auto fn cmp@Person(a: Person, b: Person) -> Int
auto fn hash@Person(value: Person) -> Long

fn eq@Person(a: Person, b: Person) -> Bool {
    return cmp(a, b) == 0
}
```

`auto Hashed<self>` is not available here, because it would generate the `eq`
this type writes by hand — and a bare `: Hashed<self>` is still the promise,
checked at the declaration, that a `hash` and an `eq` exist.

The compiler can write `cmp`, `eq` and `hash`, and says so if you write `auto`
on anything else. An `auto fn` names its type with `@`, because that is where
the fields come from. A field it cannot compare (a `Double`, a function) is an
error at the declaration, where the mistake is, not at a distant
`SortedSet<Point>`; so is a `canbe Mut` struct, which could change while a
collection holds it.

Writing a member by hand *and* asking for it with `auto` is a duplicate: keep
one.

A *generic* function has to ask, because nothing about an opaque `T` is
knowable:

```
fn min_of<T>(a: T, b: T, ?Ordered<T>) -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

let smaller = min_of("pear", "apple")   // the call site supplies `cmp`
```

The group spread is what makes `cmp(a, b)` legal inside, the call site fills it
with the canonical overload for the type it instantiates, and a function that
forwards to another needing the same capability declares it too — the colouring
implicit parameters already have.

**A hash value means something within one execution and nowhere else.** Each
backend hashes with its host's own algorithm, so the same value digests
differently on Kotlin and on Rust — deliberately, the way the two backends
generate different random numbers. What holds everywhere is that equal values
hash equal. So a hash is a bucket, not data: do not print it, persist it, or
compare it across runs.

#### A structure that holds an ordering

`sort(list, cmp = …)` picks an ordering per call, which is right for an
algorithm that does its work and hands the result back. A structure that *stays*
ordered — a heap, a ranked list — cannot work that way: building it under one
ordering and reading it under another is a corrupt structure, and "which
ordering" is a property of the value, not of each call.

So a structure names the ordering it holds as a **type argument**, fixed where
the value is constructed:

```
// The claim, with a slot for the ordering it is kept by. `?Ordered<T>` would
// say the same thing by naming the group.
qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T>

// Building one: an ordinary implicit parameter, and the return type publishes
// what the call resolved.
fn empty_heap<T>(?cmp: (T, T) -> +Heap<T>(?cmp) Int) -> Mut List<T> {
    return mut_list_of()
}

// Using one: `?cmp` is *captured* from the argument's type. Nothing says what
// it is — the slot above does that.
fn heap_push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> +Heap<T>(?cmp) Mut List<T> => !heap, !elem {
    // `cmp` is an ordinary implicit parameter in here: the one this heap was
    // built with, whatever is visible at the call.
    ...
}
```

Type arguments come first and slots after. A slot is spelled like the implicit
parameter it is resolved as, and a use site fills it with a **function
identity** — a bare name (`Heap<Person>(min_by_age)`) or a canonical
(`Heap<Person>(cmp@Person)`) — or with the signature's own binder
(`Heap<T>(?cmp)`).

Three things follow, and they are the point:

- **`Heap(min_by_age)` and `Heap(max_by_age)` are different types.** Passing one
  where the other is expected is an ordinary type error naming both.
- **All the `?cmp` in one signature are one binding.** A
  `merge(a: Heap<T>(?cmp) Mut List<T>, b: Heap<T>(?cmp) Mut List<T>)` accepts two
  heaps only if they carry the same ordering. Two *different* orderings need two
  names, which is what an alias is for: `b: Heap<T>(?cmp: cmp2)` fills the same
  slot under the name `cmp2`. A slot a signature never mentions is not
  constrained at all, so a function that does not care writes the claim bare
  (`Heap List<T>`) and accepts any of them.
- **Two orderings in scope refuse the operator.** In that `merge`, `x < y` could
  mean either `cmp` or `cmp2`, so it is an error naming both and the body calls
  the one it means. The alias is how the two get into one scope, not a way to
  make the choice implicit.
- **A function bound into a type must be named, top-level and capture-free.** A
  lambda has no identity a type could carry, and the error says so instead of
  losing the claim quietly. Everything a type can *print* it can carry.

Dropping the claim is safe rather than silently wrong: the operations demand it,
and a plain value never gets it back by subtyping — you lose access, never
correctness.

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
provenance qualifier Emitted<T> of T
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
  return `Emitted T | Finished` — or `Emitted (proj(c) T) | Finished` when
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
    // A later `use` for the same effect shadows this one from that point on.
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

### Handlers with dependencies, and interception

A handler may need an effect of its own to do its job. It declares that the way a function does — an effect list before the `of` clause — and the compiler supplies it where the handler is registered. Callers never mention it:

```
handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")      // Console reaches the body
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use ConsoleLogger                   // the Console is not written here
    log("hello")
}
```

The dependencies have no names, because there would be nothing to do with one: code inside a handler reaches an effect the way all Salvo code does, by calling its members. So the list says only *what the handler needs to run*, which is exactly what a function's effect list says.

The dependency is declared on the *handler*, not on the effect: two implementations of one effect need different things, and a `Logger` that writes to a file has no business making every `Logger` mention a console. A `use` whose dependency has no handler in scope is an error that names it, so a dependency is always registered before its dependent — which is also why dependency cycles need no separate check: `A` needing `B` and `B` needing `A` fails at whichever is registered first.

The list may name **the very effect the handler implements**, which is *interception*: a handler that wraps the one already in scope.

```
handler Loud [Greeter] of Greeter {
    fn greet(name: Str) -> Str => name {
        return "${greet(name)}!"        // the *wrapped* greeter
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use Plain                           // greet -> "hello world"
    use Loud                            // greet -> "hello world!"
    println(greet("world"))
}
```

The rule that makes this well-defined is that such a dependency **binds strictly outward**: it is the instance registered *before* this `use`, never the handler being registered. So a call inside `Loud.greet` goes one layer out rather than back to itself, interceptors stack (`use Loud` twice gives `"hello world!!"`), and the acyclicity argument is untouched — every dependency still points at an earlier registration. Registering an interceptor with nothing to intercept is an error that says so.

Interception is what makes a *policy* handler writable in Salvo: a restricting file system that checks paths and then delegates, a logging or retrying handler over whatever was there before, a test double wrapped around a real implementation. It is per instance of a generic effect, so intercepting `Store<Int>` leaves `Store<Str>` alone.

Both of these rest on shadowing, which is worth stating on its own: a `use` for an effect instance already in scope is not an error — it takes over for the rest of the block, and the handler it shadowed answers again when that block ends. Shadowing needs no dependency (registering a second, unrelated `Greeter` simply replaces the first), and the innermost registration always wins, exactly as a shadowing local variable does.

One thing a handler cannot do yet: call **its own** effect's other members. `Twice.greet` cannot call `greet` on itself — a handler does not dispatch to itself, and declaring the effect as a dependency means the handler *outside*, not this one. Move the shared logic into a function both members call; the diagnostic says as much.

### One handler, several effects

A handler may implement more than one effect, and the effects are simply listed: `handler H() of Public, Admin`. It stays one handler with one piece of state — what it gains is a *face* per effect, which is how a public protocol and an administrative one share an implementation without a second handler forwarding into the first.

```
effect Tally { fn bump(n: Int) -> None => !n }
effect Stats { fn total() -> Int }

handler Counting() of Tally, Stats {
    sum: Int = 0

    fn bump(n: Int) { sum = sum + n }
    fn total() -> Int { return sum }
}

fn main() [use] {
    use local Counting()    // registers *both* effects — `local` because one
    bump(2)                 // lock behind several faces has no shared form yet
    println("${total()}")
}
```

One `use` binds every face, so a function declaring `[Tally]` and one declaring `[Stats]` both reach this instance. For an actor the same declaration is what makes least authority ordinary: `spawn` answers **one addr per face**, in declaration order, so who holds which face decides what they may do.

```
let (timer, ctl) = spawn ManualTime() on pool(1)
//   ^Addr<Timer>  ^Addr<TimerCtl> — code holding `timer` cannot name `advance`
```

A single face answers a bare `Addr<E>` as it always did; several answer a tuple. The rules the list brings with it are the ones you would guess: every member of every effect must be implemented, each effect may be named once, and the effects must be of one kind — all `actor effect`s or none, because a handler is bound one way or the other. Where two of the effects declare the same member *name*, one handler method may implement both when their signatures are identical, or two methods may implement them when the parameters differ (ordinary overloading); what is refused is the case overloading cannot see — same parameters, a different return type — since no single method could answer for both. Callers still pick the face with `@` where two effects in scope declare the name, exactly as below.

### Monitors — sharing a plain-effect handler

A handler of a *plain* effect can also be spawned. That does not make it an actor — there is no mailbox and no messages — it makes it a **monitor**: one shared instance behind a lock, whose members run on the callers' threads under mutual exclusion. The spawn answers the same `Addr<E>` an actor spawn answers, the handle is freely copyable and sendable, and `use` binds it like any handle — so a caller of `next()` never learns whether the `Random` in scope is a scope-local handler or a monitor three threads share.

```
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    cursor: Int = 0
    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(12345)   // one shared instance, no `on` —
    use rng                               // a monitor runs on its callers' threads
    let n = next()                        // lock, advance, unlock
    let child = spawn Worker() with rng    // the same instance, from another thread
}
```

The price of sharing used to be "a monitor declares no dependencies"; since the shareable-by-default round (2026-09-20) a monitor **may** declare dependencies, provided every one is the shareable default (`[E]` — no `local E`, no `use`, no `spawn` capability). They are captured as **owned handles at construction**, resolved from the enclosing scope exactly as a `use` resolves them, so the bindings are fixed for the instance's life. The availability rule keeps the capture acyclic — a dependency was bound before this handler, so no lock order can cycle by construction — and where a member *can* wait (a dependency with mixed handlers, a `waitfor` in a member), the wait is **priced, not refused**: the handler gets a node in the same deadlock graph actors use (`H's lock`), and a cycle through held locks is reported before it runs. A handler that cannot be shared at all — unsendable state, the `use`/`spawn` capabilities, a `local` dependency — is refused with the opt-out named.

A monitor serializes with a lock where an actor serializes with a mailbox, and one piece of state can be under only one of them — so a handler is one or the other, read off its shape: mutable state with `send fn` members is an actor's, mutable state with only plain members shares as a monitor. Fit: passive shared state — counters, caches, configuration, cursors, `Random`. A generic effect instance shares like any other: `handler CyclicRandom of Random<Int>` gets a monitor of `Random<Int>`, so genericity costs nothing here.

### Shareable by default: `use`, `use local`, and `local E`

`use H(args)` binds **shareable by default** (user decision 2026-09-20). A stateless handler binds *bare* — shareable without a lock, so `StdOutConsole` and friends pay nothing — and a stateful one binds as a **monitor**: lock-shaped from birth, effectively `let h = spawn H(args); use h`. A `use` of an addr or of a spawn expression is already a handle and needs no words. The motivating goal is *spawn-inheritance*: for `spawn H on pool(2)` to pick up the scope's effects without re-declaration, a bare `[E]` in a signature has to guarantee something that may cross a seam.

That is what `[E]` now means: **a shareable `E`** — the function may pass it across seams. The opt-outs are spelled:

- **`use local H(args)`** binds scope-local and lock-free — the pre-2026-09-20 meaning. It is *required*, by an error naming it, for handlers that cannot be shared: unsendable state (a stored lambda), the `use`/`spawn` capabilities, a `local E` dependency, an actor-effect or generic-instance dependency (both pin the fusion form).
- **`[local E]`** in an effect list accepts a scope-local binding and disclaims seam rights for `E`. The call-site rule: a `use local` binding satisfies only `[local E]` requirements; a shareable binding satisfies both, since `local` is the weaker claim. The annotation is viral down call chains that traffic in local bindings — an accepted cost, to be lifted later by inference — and std's own effect-forwarding functions (`println`, the `Fs` surface, `elapsed`) declare `[local E]`, being pure forwarders that never cross a seam.
- A **fn type's** effects are always call-only — a function value cannot spawn — so writing `local` there is refused as redundant, and a lambda's availabilities are local: a function called from inside a lambda declares `[local E]`.

A dependent handler bound shareable captures its dependencies as owned handles **at construction** — from a binding in the same function, or from an effect the function received through its own signature, in which case the handle is threaded in by the caller (see the next section). A `platform handler` is **assumed thread-safe by its design** (user decision 2026-09-20): it classifies bare, so nothing that depends on a platform-backed effect ever writes `local` — `DefaultFs [RawFs]` stays annotation-free, as does the production interceptor chain, which was the point of lifting monitor dependencies. The assumption is unvalidated for now; a way for a host to state (and the compiler to check) its thread-safety is future work.

### Inheriting the scope, and overriding it with `with`

A spawned handler **inherits its dependencies from the spawning scope** (user decision 2026-09-20). A handler declares what it needs, the spawn says where it runs, and the wiring in between is the compiler's:

```
handler Drawing() [Random] of Drawer { … }

fn main() [use, spawn] {
    use CyclicRandom(7)                   // one shared instance
    let d = spawn Drawing() on pool(1)    // …inherited, with nothing written
}
```

This is what the shareable-by-default round bought. A bare `[Random]` guarantees a shareable instance, so the scope's registration can be captured as a handle and travel with the child — checked one function at a time, with no whole-program analysis and no runtime check. Before it, every dependency had to be written at every spawn.

Only a shareable binding can be inherited: a `use local` one exists precisely so that it does not cross a seam, and the error names the remedy (bind it shareable, or give the child its own). Two candidate instances in scope is an ambiguity the compiler will not guess at, and nothing in scope is still an error — now naming both ways to fix it.

Where the scope's instance is *not* what a handler should get, **`with` names what it should**:

```
use Plain
use Loud with Formal()        // Loud wraps this fresh Formal, not the Plain in scope
let d = spawn Drawing() with FixedRandom() on pool(1)
```

A `with` item is a **private instance**: constructed at the clause, owned by the handler or child it is given to, shared with nothing. The clause is *partial* — items satisfy the dependencies they match, and the rest still inherit — and it may supply the self-dependency, which is the one case where an interceptor wraps something other than what it shadows. It works on both binding forms, and `use local H with …` is fine too: the clause chooses which instance, which has nothing to do with locality.

The same lift applies to `use` inside a function that *received* the effects it wires:

```
fn interception() [Logger, Clock, use] -> None {
    use Stamped                 // captures handles for Logger and Clock —
    work("stamped")             // both arrived through this signature
}
```

Nothing in that function says `local`. On the Kotlin backend an object reference already is a handle, so this costs nothing; on the Rust backend the caller passes one extra hidden parameter — a small bundle of the handles the callee has to capture — and only on call chains that actually capture one. A function may only have such a parameter if its effect list carries `use` or `spawn`, so the possibility is visible in the signature even though the parameter is not.

One shape still refuses: a **platform effect** cannot be captured this way, because the host owns that instance and hands it to `main` as a borrow — there is no handle to make. Put an ordinary Salvo handler over it (`DefaultFs [RawFs]` is exactly this) or declare the dependency `local`.


### Mixed handlers — a servant behind a plain effect

The other way to share mutable state keeps a mailbox: a handler of a plain effect may declare `send fn` members beside its plain ones, and the two halves divide the work. The send members and the state form the **servant** — an ordinary actor over a handler-local protocol, with a `mailbox` like any other — and the plain members form the **façade**, running on the caller's thread: a façade member sends to its own servant and waits for the answer.

```
handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    cursor: Int = 0                          // the servant's, and only the servant's

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {                       // the façade: the caller's thread
        return waitfor got {                 // type read off `advance`'s parameter
            advance(got)                     // a send to its own servant
        }
    }
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(12345)      // one servant; the handle is the façade
    use rng
    let n = next()                           // send, wait, answer — no [waitfor] anywhere
}
```

A `waitfor` names its token and infers the token's type from the send the block
makes: `advance` declares `out: Reply<Int>`, so `got` is a `Reply<Int>` and the
wait yields an `Int`. Write the type out — `waitfor got: Reply<Int> { … }` —
where several members of that name would make it ambiguous, which is an error
naming that remedy.

State is **confined**: only send members touch it, so every access is a serialized activation, and a sync member that reads a field is refused by name — its environment is its own parameters, the constructor parameters, and sends to its own servant. Nothing here declares `waitfor`: a call occupying its thread until it returns is what a call is, and the façade's wait serves its pool while it waits. What a caller of `next()` can never learn is whether the `Random` in scope is a scope-local handler, a monitor, or three threads' shared servant — which is the point.

Send members reach their siblings the same way: a bare call naming another of the handler's `send fn` members enqueues on the servant's own mailbox — "finish this activation, then that one" — so a request can travel member to member without leaving the actor. `k@self(…)` is the explicit spelling of the same send, available in either member kind, and it is how the call says what it means where a bare name would be ambiguous with an effect member in scope.

A mixed handler is spawn-only (`use` would leave its sends nowhere to arrive). By default its servant answers every request within the activation that received it, which is what makes a façade's wait end after one straight-line activation rather than after an event that may never come. A send member that declares `defer` on a reply parameter opts out of that default: the answer may outlive the activation — forwarded to another actor whose discharge ends the caller's wait, or parked in a continuation on one of the servant's own members with `replyto`, exactly as an actor parks — and the deadlock graph prices the deferral.

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

The name after `@` is capitalized, which is what distinguishes an effect selector from a module path (`size@core.list(xs)`). A generic effect's instance is pinned by the call's type arguments, exactly as without the selector: `next_random@Random<Int>()`. A selected member is a call form, not a value.

Within a *single* effect a member name may recur too, as an ordinary **overload**: one name, different parameter types, picked by the argument types like any function overload.

```
effect Fs {
    fn close(s: InStream) -> Ok None | Err FsError => !s
    fn close(s: OutStream) -> Ok None | Err FsError => !s
}
```

Two members with the same name *and* the same parameter types are the error they look like — no call could tell them apart. The selector and the overload compose: `@Fs` picks the effect, the arguments pick the member.

A member and an ordinary **function** may also share a name, and they are one overload set too. std's own filesystem needs it: `close` is an `Fs` member per stream token *and* the function that closes a `Lines` pass.

```
fn close(p: Lines) [local Fs] -> Ok None | Err FsError => !p {   // a function
    return close(p.s)                                            // ...calling the member
}
```

The argument types decide, ranked exactly as two function overloads are: the more specific signature wins, so a concrete function beats a generic member. Two rules keep it predictable:

* **Availability first.** A member is a candidate only where its effect has a handler in scope. Without one, the name is the function — no `@` needed. That is what lets you write a `close` of your own in a program that never opens a file.
* **A tie is an error.** If both sides fit and neither is more specific, the call must say which it means: `close@Fs(…)` for the member, `close@my.module(…)` for the function. Nothing is preferred silently.

### Throwing: leaving early with a message

Handlers so far always *resume*: an effect operation runs and control comes back. `throw` is the other option — it does not come back. It is declared in the core library as an ordinary effect:

```
effect Throw<M> {
    fn throw(message: M) -> Never => !message
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

`throw` returns `Never`, the bottom type, so the code after it never runs — which is what keeps the frames in between silent. `parse` returns `Int`, not an outcome union: no `Result` plumbing, no unwrapping at each call. A function that calls `parse` either declares `[Throw<Str>]` too, passing the throw on, or delimits it.

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
- **A `try` whose body cannot throw is an error.** Never can produce the `Thrown` arm, so the `try` is dead scaffolding; the diagnostic says to drop it.
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
fn take_head<T>(list: Mut NonEmpty List<T>) -> T => list: Mut
```

The entries, by shape:

| entry | meaning |
|---|---|
| `=> list` | kept, and every qualifier the argument had survives |
| `=> !list` | consumed — the caller loses the value (`list: Never` says the same) |
| `=> list: Mut` | **exhaustive**: afterwards `Mut` is the *only* thing still known about the argument |
| `=> list: None` | exhaustive and empty: every qualifier stripped |
| `=> list: -NonEmpty` | **delta**: drops `NonEmpty`, leaves everything else intact |
| `=> list: +Sorted Mut` | exhaustive, and `Sorted` is **re-established by this function** — only in the file that declares it |
| `=> .items: proj(list)` | the result's field `items` projects `list` (see "Projections") |
| `=> v.items: proj(other)` | the call re-points the parameter `v`'s field to project `other` |
| `=> proj(list)` | opaque: the result *holds* a borrow of `list` somewhere inside |
| `=>[keep] !t` | a group: these entries are about the fn-typed parameter `keep` (its own parameter `t` named in its type, `keep: (t: T) -> Bool`) |

Why does an exhaustive entry drop "qualifiers this function never mentions"? Because a function that _mutates_ a value can invalidate any claim about its contents, whether or not that claim appears in its signature. A `clear` that empties a list cannot honestly promise a caller's `NonEmpty` back, even though `clear` has never heard of `NonEmpty`. So a parameter the body mutates must state exactly what survives: the bare and `-` forms are rejected there, and the compiler names the exhaustive form you want. Mutation is the only operation that invalidates a kept value — reading it cannot change its contents, and moving it ends the caller's access.

The flip side is deliberate over-strictness: `add` cannot promise to preserve `NonEmpty` either, even though appending to a list can never empty it. The function is the wrong party to ask — it has never heard of `NonEmpty` — so the claim's *owner* states it instead, in a **refinement** (see "Refinements" below). Without one, re-test with `is NonEmpty` after a mutating call.

Sometimes the mutating function *is* the right party: it knows the claim survives because it is the code that re-establishes it. A heap's `push` breaks the heap order by appending and then restores it by sifting, and nothing outside it can say so. That is written with a `+`:

```
// In the file that declares `Heap`:
fn heap_push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
=> heap: +Heap<T>(?cmp) Mut, !elem {
    add(heap, elem)         // strips the claim, as any mutating call must
    …                       // sift it back into order
}
```

`+Q` is a claim about what *this* function does, so it is **trusted** rather than checked — and for that reason it is allowed only in the file that declares `Q`, exactly like a constructor (`-> +Q T`) and a refinement. Two spellings, because they are two different statements: a plain `Heap` says the body preserved the claim and is checked against the body, while `+Heap` says the body put it there.

A plain entry may also name a qualifier the parameter does **not** carry, and then it *reports* what the body left behind:

```
export fn push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
=> heap: +Heap NonEmpty Mut, !elem { ... }
```

`push`'s parameter says nothing about `NonEmpty`, but its body calls `add` — which the qualifier's owner has refined to establish the claim — and then only swaps, which keeps it. So the heap really does come back non-empty, and the caller is told: after `push(h, 3)`, `pop(h)` answers an element rather than an optional. The promise is *checked* against the body, which is why it needs no permission from `NonEmpty`'s file, unlike `+NonEmpty`. Two rules go with it: only a written clause reports a gain (an inferred one keeps quiet — handing a caller a claim is something a signature should say out loud), and a bodiless declaration cannot report one at all, since there is nothing to check it against.

It does not matter whether the parameter already had the claim. Putting back what a mutation stripped and *minting* one on a value that arrived without it are the same sentence — "after this call, this is a `Heap`" — so the same form says both:

```
// An ordinary list on the way in, a heap on the way out.
fn heapify<T>(list: Mut List<T>, ?Ordered<T>) -> None
=> list: +Heap<T>(?cmp) Mut {
    …
}
```

A minted claim answers to the rules a constructor's `+Q` answers to, because that is what it is: the qualifier has to apply to the parameter's type, and one that holds a function has to be given one (`+Heap` alone would be a heap whose ordering nothing named). The identity is the *call's*: `heapify(xs)` and `heapify(xs, cmp = by_name)` hand back two different types.

These are enforced at each call site: passing a variable to `take_head` above removes `NonEmpty` from what the compiler knows about it, so a second `take_head(list)` without an intervening `is NonEmpty` check fails overload resolution:

```
let list: Mut List<Int> = mut_list_of(1, 2, 3)

// We will discuss this "predicate qualifier" later
if list is NonEmpty {
    // Type of `list` is `Mut NonEmpty List<T>`
    let first = take_head(list) // can call this because we have `list: NonEmpty Mut`

    // At _this_ point, `list` is no longer NonEmpty, but only Mut
    let second = take_head(list) // Invalid: there is no function for this
    let size = list.size() // Still valid, because `list` is a List<T>
}
```

In Rust, the `NonEmpty` state was not captured: this is a Salvo compile-time inference. But the fact that the function "gave back" the `list` value is captured too — a kept parameter is passed as a reference rather than moved. By contrast, a consumed parameter transfers ownership:

```
fn consume<T>(list: List<T>) -> None => !list
```

Calling `consume(list)` _moves_ the variable to the function: `list`'s type narrows to `Never` (a value that no longer exists is an impossibility), and any future reference to it in the calling function is a compile-time error until the variable is reassigned. This holds whether the consumption is written or inferred — a function that returns its parameter moves it, and callers are checked against that inferred contract just the same. It also holds uniformly across all types: for basic value types the underlying backends copy the value and the generated code would remain valid, but the Salvo-level contract is enforced consistently regardless of the type. The analysis is branch-aware: consuming a value in a branch that always exits (via `return`, `break`, or `continue`) does not affect the code after the branch, while a value consumed on only some fall-through paths is conservatively unusable afterwards. Loops account for the back edge too: a value read early in a loop body and consumed later in the same body is an error, since the read happens after the consumption from the second iteration onwards (reassigning before the body ends keeps it valid).

Consuming calls are not the only way a value moves. Every other escape route consumes a bare variable the same way, and the error at a later use names the event: storing it in a struct, array, or tuple literal (the literal owns it now), spreading it (`...n` reads all of its fields into a new value and consumes the source), returning it, `break`-ing with it, and passing it to a `use` handler constructor (the handler stores it for the rest of the scope). A `break` with a value reaches the code after the loop on every exit path, so a variable consumed by `break` is unusable after the loop even when the `break` sits inside a branch. Reads, by contrast, never consume anything — in particular, string interpolation is a read: `"${n}"` formats the value and retains nothing, so `n` stays usable. As always, `copy(...)` at the move site keeps the original usable, and reassignment revives it.

**Write what inference cannot reach; the rest is inferred.** A parameter the clause does not mention gets the contract the compiler reads off the body — kept with the qualifiers that survive every call the body makes, or consumed when the body moves it — so most functions write no clause at all, and a clause may be *partial*: `=> list: Mut` on a three-parameter function says nothing about the other two. What is written is checked against the body (a promise to keep what the body moves is an error) and is otherwise fixed. Two places have no body to infer from and must therefore say everything: an `effect` member (including a `platform effect`'s) and an `intrinsic fn` must mention every parameter — except Copy scalars (`Int`, `Bool`, …), whose fate is nothing to deduce. A function *type* is bodiless too but keeps the default of keeping everything; `=>[f] …` on the enclosing declaration is how to say otherwise, and it needs the fn type's parameters named (`f: (v: List<Int>) -> Int`).

```
// Nothing written: `list` is inferred `Mut` (because `take_head` might be
// called on it) — the hover shows `=> list: Mut`.
fn maybe_take_head<T>(list: Mut NonEmpty List<T>) [Random<Int>] -> T? {
    if next_random() > 0 {
        return take_head(list)
    }
    return None
}
```

Inference is the strictest deduction over every function the body hands the value to (moves included), computed to a fixpoint across the program. The price of inferring is that a body edit can change a contract callers depend on with no signature change; the hover always shows the effective clause, and `!p` can be written where a move is meant to be part of the contract.

### Why returning a parameter is a move

A parameter that is kept compiles to a *borrow* in Rust: the caller retains its value. A function's return value, by contrast, is *owned* by the caller unless the signature says otherwise. If a function returns one of its parameters as an owned value, these two facts collide, and Salvo resolves it without a hidden clone: returning a parameter transfers ownership out through the return channel, and the parameter is deduced as _moved_ — the caller that passed it in loses it. The same applies to the other escape routes — storing a parameter in a struct, array, or tuple literal, or passing it to a consuming call. Consequently, a written clause cannot promise a parameter back when the body returns it owned: `=> x` with `return x` is a compile-time error.

The way to give a caller access to a parameter's data *without* moving it is to return a **projection** of it — `-> proj(x) T` — which is a borrow, described next. This is purely a constraint of the Rust backend — the Kotlin backend ignores deductions, since everything is a garbage-collected reference on the JVM — but one Salvo codebase must compile to both, so the checker enforces the stricter contract everywhere.

Binding a parameter with `let` is *not* on the move list: it creates a *shared fate* link instead (see "Shared fate and `copy`").

### Projections: borrowing without copying

Salvo has no references in the source, but it has one qualifier that means "this value is borrowed from somewhere": **`proj`**. It is how the standard library reads an element out of a list, walks a list, or filters one without copying anything — and how you write such a thing yourself. The principle behind it (user decision 2026-09-11) is that **a copy never happens without the program opting in**: `copy(x)` where you want one, a `_to` function that fills a destination you provide, and nothing else.

**`proj` is part of the type.** A projected value's type says so — `proj Str`, `Mut List<proj Str>`, `Emitted (proj Str) | Finished` — in diagnostics, on hover, and through generics: matching `Emitted T` against an `Emitted (proj Str)` binds `T = proj Str`. An owned value satisfies a projected position (it can do strictly more), never the reverse — and the projection is never dropped silently. What a projection may be *passed to* follows from what the callee does with the parameter: a callee that only reads a kept, non-`Mut` parameter accepts a top-level projection even where the parameter is written owned (a borrow read in place is indistinguishable from the value), while a callee that **consumes** the parameter, **mutates** it, or expects the projection **nested** inside the type (a union arm, a type argument — where the two are genuinely different types in Rust) refuses it with an error naming which of the three stood in the way and the remedies (write the `proj`, or pass `copy(...)`). The one exception is Copy scalars: `proj Int` *is* `Int` — the number is the value itself on both backends. In overloading, an owned position beats a projected one, the way one arm beats its union: `proj` accepts more, so it says less.

**A projected value.** `proj(p) T` on a result says the value *is* a borrow of the parameter `p` — an element of it, a field, the whole of it. It may appear wherever a type does: the whole result (`-> proj(xs) Person`), a nullable (`-> (proj(list) T)?`), a union arm (`-> Emitted (proj(p) T) | Finished`), a tuple element. Several sources are written together, and a projection joined across branches is of all of them:

```
fn either(a: List<Int>, b: List<Int>, flag: Bool) -> proj(a, b) List<Int> {
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

Such a struct is an ordinary owned object: its `Mut` is real (a pass is advanced in place), its non-`proj` fields are its own, and it may be moved, stored, or passed on. What it may not do is outlive what it borrows. The compiler tracks this as shared fate too: `let p = iter(xs)` links `p` to `xs`, so `xs` cannot be moved or mutated while `p` is alive — and nothing had to be written on `iter`, because **which parameters a result holds borrows of is inferred from the body**: the literal stores `list` in a `proj` field, so `iter` lends `list`. Where there is no body — an effect member, an intrinsic, a fn-typed parameter — the clause says it: `=> proj(list)` ("the result holds a borrow of `list`"), `=> .items: proj(list)` (this field does), or on a fn type `=>[iter] proj(c)`. A written entry must name every lend the body performs; it may name more (a generic body lends through opacity the analysis cannot see). Returning a view rooted in a *local* is an error: the local dies with the call.

A container can hold borrows too: `List<proj T>` is a list of projected elements, and it is what `filter` returns:

```
fn filter<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) -> Mut List<proj T> => it: Mut, proj(it), keep
```

The result holds borrows of whatever the pass walks — nothing is copied — and it lives no longer than the source. For a list of your own, `filter_to(dest, it, keep)` copies each kept element into `dest`, and it says so twice: in its `_to` name, and in the `?copy` implicit it takes so that the copy is the element type's own.

**A view of a temporary.** A view's source must outlive it, so binding, returning or storing a view of a temporary is an error: `let p = iter(list_of(1, 2))` dies at the end of its statement (the diagnostic says to `let` the list first). *Using* one within the statement is fine — `map(iter(list_of(1, 2)), f)`, `for x in iter(list_of(1, 2))` — the temporary lives that long on both backends.

**A capturing lambda is a view.** A lambda's body can hand out projections rooted in a *capture* — `indices.map(i -> all.get(i)!)` returns elements of `all`, and no function type can say so (`proj(…)` names parameters; captures have no name). So the closure itself carries the fact: it holds a borrow of every non-Copy variable it reads from the enclosing scope, exactly as a struct holds its `proj` fields. Binding the lambda links it to those variables, a call result built from the lambda is linked through it, and moving or mutating a captured variable poisons the closure — the same discipline every view lives under. A lambda that captures nothing holds nothing (`map(p, w -> w)` binds freely), a Copy scalar capture is the value itself, and a capture the body *consumes* is owned by the closure rather than borrowed — that is the lambda that becomes `once`.

**Passes borrow.** A pass that walks data declares `: Yield<self, proj T>` and its `next` returns `Emitted (proj(p) T) | Finished` — the element is a borrow of the pass, which borrows the source — while a generator (a countdown, a random stream) declares `: Yield<self, T>` and emits owned values. The two must agree: an obligation at `proj T` with an owning `next`, or the reverse, is an error. Reading combinators accept both. An `iter fn`'s generated pass borrows its subject the same way (`__subject: proj Subject`), so nothing is copied at the mint; an `iter fn` that wants a snapshot writes `copy(...)` in a `state` initializer.

**Re-pointing.** A mutable view may be made to project something else: a function that does so says which field and from what, `=> v: Mut, v.items: proj(other)`, and the caller's variable at `v` becomes linked to `other` from the call on.

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

- **Values from calls are independent — unless they project.** `copy(x)` and most function results carry no links. A function that returns a *projection* of a kept parameter — `fn first<T>(list: List<T>) -> proj(list) T?` — or a value that *holds* one (a pass over a list) hands the caller something that shares fate with the argument: mutating the collection poisons it, moving it out needs `copy`. See "Projections" below; on the Rust backend these are real borrows, which is what makes the standard library's `first`, `get`, `iter` and `filter` zero-copy.
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

The **discharge set** is every function — and every *effect member* — declared in the *type's own file* that consumes a parameter of the type — `close` for a file, `stop` *or* `join` for a thread handle, `remove(cache, entry)` for a pooled one (the extra parameters are ordinary parameters). A `linear struct` whose file has no such function is an error at the struct: the obligation would have no legal death. Leak diagnostics name the whole set. `discard(handle)` is the obligation's terminal, legal **only** inside a discharger — and a discharger gets no exemption: its own body must terminate the obligation on every path, by `discard` or by forwarding into another discharger (`fn shutdown(t: Thread) => !t { stop(t) }`).

When the discharger is an effect member, the declaration is what carries that status, so **every handler's implementation of it** is a discharge context — the real one, a test double in another module, an interceptor that discharges by forwarding into the handler it wraps. That is how a stream token stays linear while `close` is a member of `Fs`: one declaration, many implementations, all of them allowed to end the obligation and none of them able to end anybody else's (a member consuming an `InStream` may not `discard` an `OutStream`).

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
- **A container is linear exactly when its element is.** A `List<FileHandle>` owes; a `List<Int>` does not; nothing at a use site says which — the element's declaration is the source, and the container opts a parameter in on *its* declaration (`intrinsic type List<T canbe linear>`, `struct Box<T canbe linear>`). Obligations enter with `add`/`replace`, leave one at a time with `remove_first`/`remove_at`/`remove` (each answering `T?`, so the emptiness check is the ordinary narrow), and the container's **terminal** is `drain(container, each)`: it consumes the container and hands every element to a callback that consumes it. A container that is neither drained nor moved onward is an ordinary leak, and the diagnostic names `drain`.

  What a container may *not* do is drop an element on your behalf, so the surface has no `clear`, no positional list write, and no `get` for obligations (that would hand out an alias). `Set`, `SortedSet` and map **keys** refuse obligations outright: insertion deduplicates, and dedup *is* dropping — an equal element or a repeated key discards one of the two values, which no API reshaping can fix. A struct field holding either an obligation or a container of them makes the struct a resource too, so it takes the `linear struct` marker: contagion is spelled, never inferred. Arrays and tuples stay out for now, and a linear value still cannot travel through a *variadic* position (those are untracked).

- **Handler state may hold obligations, and the actor owes until it ends.** A queue of parked reply tokens (`waiting: Mut List<Reply<Str>>`) is what the concurrency surface is for, so a handler field may hold a container of obligations. Within one activation the discipline is unchanged — take one out, and either discharge it or put something back — and a member that *returns* with a state field moved out is an error, because it would leave the actor with a hole a later activation would read. This is the one place the promise weakens: static analysis cannot know what an actor holds at an arbitrary future point, so the guarantee becomes "the actor owes until it ends", and what a *death* does with parked obligations is what `watch` reports. A **bare** obligation in a field is refused, naming the container: taking it out would leave that hole and nothing could be put back, so its obligation would have no reachable discharge at all.
- **Generics opt in per type parameter**: an unconstrained `T` cannot be instantiated with a linear type, but a function may declare `fn hold<T canbe linear>(value: T) -> T` — the same `canbe linear` phrase as on type declarations, now opting the *function's handling* in. Inside the body, `T` values are treated as linear (they must be discharged on every path); in exchange, callers may instantiate `T` with linear types, and an opted `T` forwarded to another generic requires that one to be opted too. The standard library's collection surface is audited and opted where sound (`list_of`, `mut_list_of`, `add`, `size`, `remove_first`, `remove_at`, `drain`, and a map's `remove`/`replace`/`drain`), and since containers carry obligations those opt-ins are permissions to *store* as well as to call. `get` stays out (it returns an alias of an element, which would duplicate the obligation) and `copy` refuses linear values outright. `discard`'s declaration is simply `intrinsic fn discard<T canbe linear>(value: T) -> None => !value`. One extra rule: a linear value cannot be passed in a *variadic* position (those are untracked).
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

Predicate qualifiers apply to an existing value, and need not be present in the generated Rust and Kotlin code. By constrast, _constructive_ qualifiers can correspond to a new type in the underlying Rust or Kotlin code (like `MutableList<T>` is different from `List<T>` in Kotlin) or correspond to a different way of using it (like mutable `Vec<T>` requires mutable references and variables in Rust). To support such cases, the only way to build these sorts of types is by calling a function. The qualifier itself has no body, and the constructor functions must all be defined in the same file as the qualifier declaration. A constructor function is marked by writing `+Qualifier` in front of its return type — the same establishment marker deductions use: every return point returns a plain instance of the return type, which is assumed to gain the qualifier _by construction_. Callers of the function see the qualified type. Constructor functions must return a simple type (not a union or tuple); other functions can add more complexity on top of the constructors:

```
// No body -- we're using a constructive qualifier
qualifier RandomPositive of Int

// `-> Int as RandomPositive` marks this as a constructor for the qualifier.
// Callers see the return type `RandomPositive Int`.
fn random_positive_int() [Random<Int>] -> +RandomPositive Int {
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

// Requiring a first element guarantees the predicate by construction — which
// is how std's own list constructor is shaped: `list_of()` for an empty list,
// `list_of(first, ...rest)` for one that is known non-empty.
intrinsic fn list_of<T>(first: T, ...rest: T[]) -> +NonEmpty List<T>
```

**This one is real.** `NonEmpty`, and the constructor above, are declared in std's `core.list` — so is `Sorted`, and `Distinct` in `core.set`; see "Collections".

This is also how union arms are tagged in practice: a generic constructor applies the tag, and the tagged value then coerces into the union:

```
provenance qualifier Ok<T> of T
provenance qualifier Err<T> of T

fn ok<T>(value: T) -> +Ok T {
    return value
}

fn err<T>(value: T) -> +Err T {
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

Provenance covers more than authority. The claim's semantics — "established by what the value passed through, not by what is in it" — fits two families, and std uses both:

* **Authority**: `Authenticated Request`, an environment id — the value came through a checkpoint, and holding the tag *is* the proof.
* **Protocol role**: the tags `Ok`, `Err`, `Thrown` and `Emitted` are provenance qualifiers — `ok(x)` is where an `Ok` comes from, and nothing in an `Int`'s bits could ever say which arm it is. That classification is what you would want anyway: an `Ok Mut List<T>` stays `Ok` through an `add` (mutating the payload cannot change which arm it came in), and a tag stacks with any content claim without a `with` declaration — `Ok NonEmpty List<T>` needs no ceremony.

What separates a provenance claim from a mint-only state claim is content-dependence, not the lack of a `qualifies`: `Sorted` is also mint-only, but it is a claim about *contents*, so mutation strips it; a tag is a claim about *origin*, so it survives.

### Dependent qualifiers

A claim can be about a value's relationship **to another value**. The qualifier declares the values it depends on as **value slots** — unprefixed entries in its block — and its `qualifies` takes them as parameters after the subject:

```
// core.map — the claim that a key is present in one particular map.
qualifier KeyOf<K, V>(map: Map<K, V>) of K {
    fn qualifies(key: K, map: Map<K, V>) -> Bool {
        return contains_key(map, key)
    }
}
```

A use fills the slot with a **place** — a variable or a field chain — and the test is the ordinary `is`, with the block filled:

```
if k is KeyOf(m) {
    // `k` is a `KeyOf(m) Str` here
}
assert!(k is KeyOf(m))     // or hoisted: narrows the rest of the scope
```

`KeyOf(m)` and `KeyOf(m2)` are different facts — the claim is bound to the *identity* of the value that filled the slot (its fate roots, so an alias of `m` is still `m`). And because the claim is about the **map's** contents rather than the key's, it is invalidated from the other side: any mutation of `m` strips `KeyOf(m)` from every value holding it, conservatively — reads keep it, and mutating some other map keeps it. This is the refinement-types machinery at its smallest: prove a fact once, carry it in the type, and let mutation of what it depends on take it away.

The dependent claims std ships, and the total overloads that consume them (`get` answering an element rather than an optional), arrive with the rest of the sequence — see ROADMAP.md.
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

`refn` is deliberately narrower than `fn`. It has no body, it cannot declare effects, and it cannot declare a return type — a refinement never changes what a function *does*, only what is *known* about the arguments afterwards. Its deduction entries can only add (`+Q`) and remove (`-Q`) state qualifiers; a plain name (which would mean "only this survives") is the function's own deduction to make. The parameter list is there to pick one overload, so it repeats that overload's parameters: same names, same types, with type parameters matched by position.

**A refinement may ask for more than the function does**, and then what it says is *conditional*:

```
// Swapping two elements of an *already* non-empty list leaves it non-empty.
refn swap(list: NonEmpty Mut List<T>, i: Int, j: Int) => list: +NonEmpty
```

The extra qualifier on the parameter is a precondition: the statement applies only where the argument already carries it. That is the difference between a claim a call **establishes** and one it only **keeps**, and the distinction is load-bearing in both directions. Written without the `NonEmpty`, the refinement above would say that swapping *makes* a list non-empty — false for an empty one. And because a kept claim was there before the call either way, it survives a call that only *might* happen:

```
if smallest != i {
    list.swap(smallest, i)      // `list` is still NonEmpty after the `if`
}
```

whereas an established one does not — a call that may not have run cannot have made a list non-empty.

At a call site the refinement has the last word. `add`'s own `=> list: Mut` drops everything it does not name, and then `+NonEmpty` puts the claim back:

```
let xs: Mut List<Int> = mut_list_of()
add(xs, 1)
// `xs` is `Mut NonEmpty List<Int>` here, so this resolves:
let n = count(xs)
```

A refinement is **trusted**, exactly as `-> +Q T` is: nothing proves that `add` establishes `NonEmpty`, and no runtime check is emitted. The qualifier's author owns the claim's meaning, which is why they are the right party to ask.

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

A declaration is only importable if its own module lets it out. **Declarations are module-private by default, and `export` is how one becomes visible elsewhere:**

```
export fn label(n: Int) -> Str {   // callers in other modules may use this
    return "${n}"
}

fn digits(n: Int) -> Int {         // this module's business, and nobody else's
    return 1
}
```

The modifier comes first, before `intrinsic`, `linear`, `actor`, `platform`, `provenance`, `iter` and `send`, so a declaration reads *who can see it, what kind it is, what it is called*. It is an ordinary word rather than a reserved one, so a variable may still be called `export`; only the start of a declaration makes it a modifier.

It applies to the whole declaration and not its parts: an exported struct exports its fields, an exported effect its members, an exported handler its state. There is no field- or member-level visibility. What that leaves is one useful consequence worth knowing deliberately — a type you do not export may still appear in a signature you do, and callers can then hold values of it without being able to write its name. That is an opaque type, and it is the only way Salvo has to say it.

Privacy is per *module*, which is per file. Splitting a file in two is therefore a visibility decision: what the halves share has to be exported. The standard library is written this way, which is what it is for — `fire_after` and `earliest_due` in `time`, the `mem_*` helpers in the in-memory filesystem, are reachable from their own file and nowhere else.

The diagnostics are meant to send you to the right file. Using a private name says it is declared in module `m` and not exported, and names the fix; it is not offered as an import, since importing it could not work. An `import` of a private name is refused at the import line rather than silently importing nothing. And a name that is in scope but of the wrong *kind* — a qualifier where a type belongs — keeps its own diagnostic, because that is not a visibility problem.

A module can also be imported *whole*, which brings every name in it — and in every module beneath it — into scope with one line:

```
import time // Instant, Tick, Duration, Clock, Ticker, Timer, millis, between, ...
```

This is how a std surface that is not implicitly visible stays cheap to use: `time` is one import rather than a dozen. A whole-module import is deliberately the weakest way for a name to arrive. Your own module's declarations win over it, and so does a named import, both silently — so a file that declares its own `Duration` keeps it, and `import other.Duration` beside `import time` picks the other one. Functions do not compete at all: overloads from a bulk import simply sit lower on the scope ladder, and `between@time(a, b)` names the module explicitly. The one case that cannot be resolved by hand at the use site is two whole-module imports carrying the same *type* name, since a type reference has no module selector; there the first module wins and the compiler warns, naming the import that would settle it. A module import takes no `as` — there is nothing to qualify a renamed module with — and `import core` is redundant, since core is always visible.

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
program reaches its target language through a `platform effect` — or a
`platform handler`, a host implementation of an ordinary Salvo effect —
whose member functions you declare and the compiler turns into an interface
for the host to implement (see the backends section) — there is no way to
name a Kotlin method or a Rust function that some declaration in scope does
not already stand for. Dot-notation still reads like a method call
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

## Testing

A test is a declaration:

```
test "an empty heap pops nothing" {
    let heap = empty_heap<Int>()
    expect(pop(heap) is None, "popping an empty heap answers None")
}
```

Named by a string, because a test name is prose and there is nothing to call it by. It takes no parameters, declares no effects, returns nothing, and cannot be exported — a test is run, never referenced. `salvo test` finds them, runs them, and prints what happened:

```
test heap :: an empty heap pops nothing ... ok (2 ms)
test heap :: pops come out in order ... FAILED
    expected 9, got 5

5 tests: 4 passed, 1 failed
```

### Where tests live

**In a companion file, always.** The tests of module `heap` live in `heap.test.sv` beside `heap.sv`, and a `test` block written in a production source file is an error naming the companion. `.test` is the one dot a file name may carry; the file is module `heap.test`, the **test annex** of module `heap`.

The annex is the module's whitebox: it sees every declaration of `heap`, private ones included, exactly as another file of the module would. Nothing sees the annex. That is not a rule to remember but a consequence of two facts — no production file imports it, and `salvo compile` and `salvo run` do not load `.test.sv` files at all. There is nothing to strip from a production build, and no way for shipped code to come to depend on a test.

An annex may declare whatever its tests need: helper functions, structs, qualifiers of its own. They are invisible to the module it tests, so a test vocabulary never leaks into the shipped surface.

An annex with no module to be the annex of is an error: `heap.test.sv` needs a `heap.sv` beside it.

### What a test may do

A test body is an ordinary block with an entry point's powers. It may `use` handlers, which is how a test supplies fakes:

```
test "a missing file reports its path" {
    use MemFs()
    let outcome = try {
        read_to_str("/nothing")
    }
    expect(outcome is Thrown, "reading a missing file fails")
}
```

Everything else about a body is the language as it is everywhere else — narrowing, linearity, deductions, effects. A linear value a test opens must still be closed on every path, and a test that leaks one does not compile.

### How a test fails

`std.test` is **implicitly available in every annex**, which is why no test file imports anything to assert:

- `expect(condition, label)` — the general assertion. The label says what was expected.
- `expect_eq(actual, expected)` — for a type with an `eq` and a `to_str`, which is what the report prints.

An assertion that does not hold *throws*: assertions declare `[Throw<Failure>]`, and the harness reads the outcome off a `try` [throw]. Two consequences follow from that, and they are the whole model:

**A test stops at its first failing assertion**, because that is what `throw` does. Several independent facts are several tests.

**An assertion vocabulary of your own is an ordinary function.** A helper that asserts declares `[Throw<Failure>]` and composes with the built-in ones; there is nothing to register and no framework to extend:

```
// In an annex, beside the tests that use it.
fn expect_sorted(list: List<Int>) [Throw<Failure>] -> None => list {
    let i = 1
    while i < list.size() {
        expect(list.get(i - 1)! <= list.get(i)!, "sorted at ${i}")
        i += 1
    }
}
```

`Failure` is a small struct carrying the message, so a failure can be built and inspected like any other value.

### Running them

```
salvo test --src ./my_project                    # everything, on the rust backend
salvo test --backend kotlin --src ./my_project    # the same tests, the same report
salvo test --src . "empty"                        # only tests whose id contains "empty"
salvo test --src . --list                         # enumerate, run nothing
```

A test's **id** is the module a program would import, then the name as written: `heap :: an empty heap pops nothing`. The filter is a plain substring of that id, so one word selects a module, a test, or a family of tests. The command's exit code is what a build reads: nonzero when anything failed.

**A test that fails an assertion is a failed test, not a dead run.** An assertion traps, and a trap would ordinarily end the program — so the generated harness catches it and reports it like any other failure, with the trap's own message:

```
test calc :: first passes ... ok (0 ms)
test calc :: this one traps ... FAILED
    salvo: n should exceed 100, was 6 at calc.test:7:5
test calc :: and the run goes on ... ok (0 ms)

3 tests: 2 passed, 1 failed
```

Only the harness can do this: the catch is `std.test`'s own, so it exists in test files and nowhere else — production code still cannot catch a trap. A death the harness *cannot* catch (a process killed outright) is still handled, by naming the test that was running and re-running the rest.

**A test can also be about a trap.** The same catch is available to you:

```
test "halving an odd number traps" {
    expect_trap(() -> { halve(3) }, "halving an odd number")
}

test "the trap says which number" {
    expect_trap_with(() -> { halve(7) }, "got 7", "halving an odd number")
}

test "the message is available" {
    let trap = trap_of(() -> { halve(5) })
    expect(trap is Str, "a trap message came back")
}
```

The body is a plain fn value, so it does not inherit the test's effects — a body that needs one registers it itself (`() -> { use StdOutConsole(); … }`).

The runner works by writing a Salvo program. It synthesizes a module that calls each test inside its own `try`, compiles it with the rest of the sources exactly as `salvo run` would, and renders what it prints. So the two backends run the same tests the same way, and the report is identical on both — a test suite is not a place where a target language should show through.

## Assertions

Four ways exist to deal with a fact that might not hold, and they are in order of
preference — the ladder is the point of this section, and assertions are its
bottom rung:

1. **Prove it.** A qualifier carries the fact in the type, established by
   construction (`list_of(1, 2, 3)` is `NonEmpty`) or by a refinement, so nothing
   checks it twice and nothing can fail.
2. **Require it.** A linear type makes the caller *act* rather than merely know:
   forgetting is a compile error, not a run-time one.
3. **Handle it.** `?:` picks a fallback, `is` narrows, `when` covers the cases,
   `throw`/`try` carries a failure to a delimiter. Use these when the absence is
   a *case* rather than a bug.
4. **Assert it.** Last, and only where the fact is true but unprovable here.

An assertion says something the checker cannot prove, and fails if it is wrong.
Three forms, each carrying a `!` because a bang in Salvo marks a place that can
fail:

```
let head = first(xs)!                    // asserts presence
assert!(n > 0, "n must be positive, was ${n}")
unreachable!("an Int is negative, zero or positive")
```

**`expr!` asserts presence** and answers the value without its `None` arms. It
needs an operand that *can* be absent: `3!` is an error, because it states
something false. Where a value can legitimately be missing, `?:` and `is None`
are the answers — the diagnostic says so.

**`assert!(cond)` asserts a condition**, with an optional message, and it also
**narrows**:

```
fn describe(value: Int | Str) [Console] -> None {
    assert!(value is Str, "expected a string, got ${value}")
    println("len ${size(value)}")       // `value` is a `Str` here
}
```

That is what makes it more than a check: the same `is` test that would narrow
inside an `if` narrows for the rest of the scope, so an assertion buys a fact the
type system then carries. `assert!(xs is NonEmpty)` makes `first(xs)` answer an
element.

**`unreachable!()` asserts that a path is not taken.** Its type is `Never`, so it
stands wherever a value is expected and ends the path, exactly as `throw` and
`return` do:

```
return when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    n > 0 { "positive" }
    else { unreachable!("an Int compares one way or the other") }
}
```

The two named forms look like function calls and are not: the message is built
*only when the assertion fails*, the condition's narrowing reaches the enclosing
scope, and neither name can be shadowed. `assert` and `unreachable` stay ordinary
identifiers — only `assert!(` and `unreachable!(` are the forms.

**A failed assertion reports in Salvo's words.** The message names the Salvo
module, line and column, and reads identically on both backends:

```
salvo: n must be positive, was 7 at main:5:5
salvo: value is absent at core.list:153:12
```

The mechanism underneath is each target's own trap — a panic on Rust, an
`AssertionError` on the JVM — because neither program is meant to continue.
**Assertions are always on.** There is no build mode that removes them; a check
you cannot rely on is not worth writing.

## Backends

One of the aims of Salvo is to make it easy to integrate Salvo code with the backend code. To achieve this, the Salvo compiler builds an internal representation (in Rust), and passes this on to the configured backend implementation to write out the relevant target source code. In order to support this, we distinguish between two layers: the `intrinsic` layer, which is the compiler's, and the `platform` layer, which is yours. The core library is entirely intrinsic; everything an application needs from its target language is a platform effect.

### Intrinsic

The `intrinsic` layer sits in a backend specific module inside the compiler. This handles complex language-specific logic, and core functionality: how to encode union types, what the `None` type transpiles to in different cases, how to pass parameters to functions, how function naming works, how imports are handled, and more. These can only be changed by making changes to the compiler itself. Anything involving syntax will appear here, and all `intrinsic` backend definitions are declared as part of the standard library (defined in `std`).

`intrinsic` is the standard library's alone. Customer code cannot declare one, because there would be no lowering in any backend to give it meaning — an `intrinsic` with no compiler support behind it is a promise nothing keeps. Application code reaches the target language the other way, through the `platform` declarations — a `platform effect`, or a `platform handler` implementing an ordinary effect; that is the single interop path. This is also the one exception to a plain structural rule: a top-level `fn` must have a body and a `type` must have a definition (`= ...`). The bodyless declaration forms customer code does have are the `platform` ones, whose contract the *build* fulfils; `intrinsic` (and the bodyless `intrinsic handler`) is what lets the standard library state a contract the compiler fulfils in place of one.

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
handler AuditLogger [Telemetry] of Logger {
    fn log(message: Str) -> None => message {
        record(message)
    }
}
```

Two restrictions follow from the host implementing one concrete interface: neither a platform effect nor its members may be generic. Member names may be shared with other effects like any effect's, and overloaded within the effect like any effect's (see "Two effects, one member name").

### A host implementation of an ordinary effect

A `platform effect` says *the whole effect is the host's*. Sometimes the effect is Salvo's own — declared here, handled here, with several handlers — and only one of those handlers is host code: the one that actually touches the outside world. That handler is a `platform handler`:

```
effect RawClock {
    fn raw_now() [] -> Int
}

platform handler HostRawClock of RawClock
```

It is bodyless, because its members live in the target language, and it is otherwise an ordinary handler: registered with `use`, one instance per registration, constructor parameters passed through to the host class.

```
fn main() [use] {
    use HostRawClock()       // constructs the host's class
    use DefaultClock()       // ordinary Salvo, depends on RawClock
    ...
}
```

The implementation goes in the same `platform/` tree as a platform effect's, as a class named after the *handler* — the `use` site constructs that name, so it is not the host's to choose — and `salvo platform generate` writes the skeleton for it too:

```kotlin
// platform/main.kt, as generated
class HostRawClock : RawClock {
    override fun raw_now(): Int {
        TODO("implement RawClock.raw_now")
    }
}
```

Nothing else moves: `main` stays the program's entry point, because the instance is constructed *inside* the program rather than handed to it. Constructor parameters are how a host implementation is configured — `platform handler HostS3(bucket: Str) of Store`, registered as `use HostS3("my-bucket")`, becomes a class with a `bucket` parameter.

Three restrictions, each following from the implementation not being Salvo's:

* **No body in Salvo** — no members, no state. The host class holds both.
* **No effect dependencies.** A handler's dependencies are supplied to its *members*, and these members are host code, which performs no Salvo effect: the host reaches the outside world directly. Write an ordinary Salvo handler that depends on this one's effect when something has to sit in between — `handler DefaultClock [RawClock] of Clock` is exactly that.
* **Not generic**, for the reason a platform effect is not: the host writes one concrete class.

The two forms answer different questions. Use a `platform effect` when the *capability* is the host's and the program is a guest in the host's process — the host constructs everything and owns `main`. Use a `platform handler` when the capability is the language's, several implementations exist, and one of them is host code: a real filesystem beside an in-memory one, a host clock beside a fake, an S3-backed store beside a local directory. The standard library uses the second form itself, and ships its host classes the same way — under `std`'s own `platform/` tree, one file per backend.

## Files

The filesystem is the first place all of this meets: an effect for the capability, linear tokens for the streams, a linear error that cannot be dropped in silence, a pass for the lines, and a `platform handler` at the very bottom.

A program that reads a file declares `[Fs]` and nothing else:

```
fn first_line(path: Str) [Fs] -> Ok Str | Err FsError => path {
    let opened = open_read(path)
    if opened is Err {
        return opened                    // the error travels; it still owes
    }
    let s: InStream = opened             // s owes: a stream must be closed
    let line = read_line(s)
    let closed = close(s)                // the discharger, and it reports
    if closed is Err {
        return closed
    }
    when line {
        is Str { return ok(line) }
        is None { return err(FsError { kind: IoError {path: copy(path), message: "empty"} }) }
    }
}
```

Four things in that function are the language's, not the library's:

* **`open_read` returns a union with a linear arm**, so the result *is* the stream: forget to look at it and the program does not compile; narrow it to `Err` and the stream was never opened.
* **`InStream` is linear**, so the `close` is not politeness. Its only field is a handle — the position, the buffer and the resource live in the handler — which is why no stream operation needs `Mut`.
* **`FsError` is linear too.** An error you do not care about takes one call to say so: `ignore(e)`. One you want to keep costs `detach(e)`, which hands back the plain `FsErrorKind` (a linear value may not be stored, so this is the way into a `List<FsErrorKind>`). Narrowing a result to its `Ok` arm discharges the error that was never there.
* **Failures are returned, never thrown.** An effect member may declare no effects, `Throw` included, so every fallible member answers `Ok T | Err FsError`.

Reading the lines is ordinary iteration, over a pass that owns the stream:

```
fn print_file(path: Str) [Fs, Console] -> None => path {
    let opened = open_read(path)
    if opened is Err {
        println("cannot read ${path}: ${to_str(opened)}")
        ignore(opened)
        return None
    }
    let p = lines(opened)                // the stream's obligation moves in
    for line in p {
        println(line)
    }
    let closed = close(p)          // closing the pass closes the stream
    if closed is Err {
        ignore(closed)
    }
}
```

And the 90% case needs none of it — `read_to_str(path)`, `read_lines(path)`, `write_str(path, text)` open, work and close, so no token ever reaches the caller.

The composition root is where the filesystem is chosen:

```
fn main() [use] {
    use StdOutConsole()
    use HostRawFs()      // the host's: real files, plain handles
    use DefaultFs()      // Salvo: mints the tokens, maps the errors
    let text = read_to_str("notes.txt")
    when text {
        is Ok { println(text) }
        is Err { println(to_str(text)) ignore(text) }
    }
}
```

`DefaultFs` depends on `RawFs` and says so on its declaration, so nothing above it mentions the raw layer; `RawFs` trades in `Long` handles and droppable error kinds, so the host class never holds a Salvo obligation — the `close` that discharges a token is Salvo code, checked. Swapping the bottom swaps the filesystem: a handler of your own that implements `Fs` fakes the whole surface, streams included, because the stream operations are *members* rather than free functions.

Byte offsets are exact and usable: `write` and `write_line` answer how many bytes they took, `position` reports the consumed byte offset of a stream, and `open_read_at(path, offset)` reopens at one. Byte counts are `byte_size(str)`, deliberately a different function from `size(str)`, which counts characters. There is no seek — streams are forward-only.

Bytes are readable and writable as themselves: `read_bytes(s, max)` answers up to `max` bytes as a `Bytes` and `write_bytes(s, data)` writes them back, with nothing encoded or decoded on the way, so a file that is not text is handled by the same surface. Text and byte operations share one stream and one position, both counted in bytes, so a text read continues exactly where a byte read stopped. Text is decoded **strictly**: an offset that lands mid-codepoint is a legal seek — it is bytes, and bytes have no characters — and it is the *decode* that fails, as `Err InvalidUtf8` rather than as mojibake. That failure is recorded too, so `close` reports it a second time.

```
let s: InStream = opened
let head = read_bytes(s, 4)      // Ok Bytes | Err FsError
let rest = read_all(s)           // continues after those four bytes
```

Every read has a **fill-a-buffer** form, for the loop where allocating a payload per step is the cost: `read_to(s, buf, max)` appends up to `max` bytes to a `Mut Bytes` of yours and answers how many, `read_to(s, builder)` appends the rest of the stream to a `Mut Str`, and `read_line_to(s, builder)` appends the next line and answers whether there was one. They *append* rather than overwrite, so `size(buf)` is the data — there is no "only the first n are meaningful" convention — and `clear(buf)` between steps is what makes one buffer serve a whole loop.

```
let buf = mut_bytes()
let reading = true
while reading {
    clear(buf)
    let got = read_to(s, buf, 65536)     // Ok Int | Err FsError
    when got {
        is Ok { if got == 0 { reading = false } else { consume(buf) } }
        is Err { ignore(got) reading = false }
    }
}
```

Or hand the loop over: `chunks(s, size)` is a pass over a stream's bytes (a fresh buffer per step) as `lines(s)` is over its lines, and the one-shots keep the buffer out of sight entirely — `copy_file(from, to)`, `copy_stream(s, w)`, `read_to_bytes(path)`, `write_bytes_to(path, data)`.

Two more handlers come with the surface, and neither is a special case of anything:

```
fn main() [use] {
    use StdOutConsole()
    use MemFs()                      // a filesystem in memory: no host, no disk
    if true {
        use RestrictedFs("notes")    // ...scoped to one directory, for this block
        // code in here writes "a.txt" and cannot reach "../secret.txt"
    }
}
```

`MemFs` fakes the *whole* of `Fs`, streams included — which is what putting the stream operations on the effect bought — so a test needs no filesystem at all. Its files are **bytes**, as a real one's are, and it counts byte offsets exactly as the host does, because a fake that stored text and counted characters would let tests pass while production broke.

`RestrictedFs(root)` is an **interceptor**: it declares the effect it implements, so it wraps whichever filesystem is already registered, and the same handler restricts the host's files in production and a `MemFs` in a test of the restriction itself. Paths are rebased — code under it never learns where it is really running — and one that resolves outside the root comes back as `Err PathEscapes` rather than pretending not to exist. The check is lexical, so it is not symlink-safe; that hardening belongs to the host layer and is not pretended away here.

## Where work runs

Concurrency in Salvo is built out of two things that run on **pools**. A pool is a set of worker threads (`pool(4)`), or exactly one (`thread()`), and it is an ordinary value: one pool can host many actors.

The two kinds of work are:

* an **activation** — one member invocation of an actor, which is a handler bound with `spawn` instead of `use`. An actor's activations run one at a time, in the order their invocations arrived, on some worker of the actor's pool. That serialization *is* its mutual exclusion.
* a **task** — the body of a free `send fn`, scheduled rather than called. A task has no mailbox, no state and no identity; it exists because `replyto` may target a free `send fn`, so an ordinary synchronous function can wire future work and return:

```
send fn finish(label: Str, out: Reply<Str>, row: Int) => !label, !out, !row {
    out.send("${label}=${row}")
}

fn fetch(id: Int, out: Reply<Str>) [Db] -> None => !out {
    db.query(id, replyto finish("row", out))   // wires the work, then returns
}
```

**A mint schedules nothing.** `replyto finish(…)` allocates a token and decides *where* the continuation will run; the body runs only when someone sends to that token. Sending to it queues the task on that pool, and a worker of that pool runs it.

**Placement is inherited unless you write it.** `on POOL` is optional at a mint, and omitted means the pool current where the mint was written — inside an actor's member, the actor's own pool; in `main`, main's own pool. Whoever creates work pays for it, so a client cannot spend a shared service's threads by accident, and an actor's continuations stay on the threads its author budgeted. Write `on p` when the work belongs somewhere else.

**One serving rule, and one ordering.** A worker serves its own pool. It takes a queued task **before** a pending activation, and each queue is served in arrival order. Both backends do exactly this, so an interleaving does not depend on which one you compiled for.

That leaves one question — *which* thread serves the pool, and what else that thread might be doing. The cases:

* **A pool with several workers.** The task runs as soon as any worker is free. Nothing more to know.
* **A one-thread pool whose thread is idle.** The task runs immediately.
* **A one-thread pool whose thread is inside a blocked activation.** A `waitfor` **serves its own pool while it waits**, so the task still runs — nested on that thread, while the waiting actor's mailbox stays stalled. This is not an optimisation, it is what makes the inherit-by-default rule safe: an actor that mints a task on its own pool and then waits for that task's answer would deadlock against itself if a wait merely blocked.
* **The main pool.** `main` is the single worker of a pool of its own, and it never gets a thread besides its own — so main-pool work runs **only while `main` waits**, on main's own thread, nested inside the `waitfor`. This is what makes `main` and an actor indistinguishable to a function that mints: a mint from `main` has somewhere to land.

The main pool has one consequence worth stating on its own. **If the answer arrives after `main`'s last `waitfor`, the task never runs.** It is not lost work that will be picked up later: nothing else serves that pool, and everything still queued dies when `main` returns. From `main`, a task is *reached* by a subsequent wait, so "wire it and forget it" is the one shape that quietly does nothing. Place the work `on` a pool with workers of its own if it must proceed regardless of what `main` does next.

What a wait serves is worth being precise about, because it is asymmetric:

* An **actor's** wait serves its pool's tasks and other actors' activations, but never its own — re-entering an actor mid-activation is exactly what serialization exists to prevent. On a dedicated thread (`thread()`), which by linearity has exactly one occupant, that means it serves tasks only.
* A **task's** wait excludes nothing, because a task belongs to no actor. It cannot re-enter a running actor regardless: an actor with an activation in progress has nothing deliverable.
* Blocking is just the degenerate case of an empty queue. There is no separate "blocking" and "pumping" semantics to reason about.

Two more cases complete the picture:

* **A token that is never sent to.** The task never runs — but you cannot get there by forgetting, because a `Reply<T>` is linear: whoever holds it must send to it or pass it on. What can still happen is that its holder *dies* first (a faulted actor loses what it owed), and then the runtime's idle report names the waiter that can no longer be answered instead of hanging.
* **A task that faults.** It has no identity, so there is nothing to `watch`. The fault goes to its pool's **fault sink** if the pool was given one — `pool(4, sink)`, where `sink` is the addr of an actor serving `Faults` — and otherwise is named on stderr. Either way the program carries on: a task's death is not the program's.

**Asking when the work is done.** A near relative of that detection is available to a program (near, not the same: the hook reads the stricter condition — it does not fire while any frame is parked in a wait, where the report counts a parked frame out of the running ones, since a parked frame cannot get anywhere on its own): `on_idle(p, notify)` registers a one-shot for the moment nothing anywhere can run, and answers an `Idle` saying what pool `p` is still owed — `parked_gates`, the actors placed there whose mailbox is gated on a reply, and `parked_tokens`, the reply tokens aimed at work there that nobody has discharged. Both zero means the program is *finished*, not merely quiet.

```
let p = pool(2)
counter.bump(2)                                  // … place work on p …
let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
println("gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
```

The token is minted like any other and **consumed** by the registration, so a hook you forget to register is the ordinary linearity error rather than a request that quietly never answers. The answer is edge-triggered and one-shot, because delivering it is itself work and ends the idleness that produced it: hearing about the next one means registering again. And it says what it says only while nothing outside the scheduler injects work — a platform handler with a thread of its own can make "idle" stale.

Finally, what a task body may *do*. It is ordinary Salvo, with one restriction: it declares no effects. A task runs detached from the frame that minted it — that frame may have returned by the time it runs — so there is no scope left to supply its handlers from. Reaching an actor needs no effect declaration, so the way to give a task a capability is to hand it an `Addr` as a capture and send to it; anything else belongs in the function that mints. A task may wait (`waitfor` needs no declaration anywhere), and a wait serves the pool it runs on.

## Time

Time is a std module rather than a language feature, and it is not part of `core`, so it is imported. The surface is a dozen names, so one line brings all of it:

```
import time
```

**Three types, one representation.** A `Duration` is a span, an `Instant` is a point on the wall clock, and a `Tick` is a point on the monotonic clock. Each is an ordinary struct holding a single `nanos: Long`, which is what makes every value canonical: a two-field `{secs, nanos}` form would let the same span be written two ways, and struct equality is structural, so the two would compare unequal.

**Why there are two kinds of point, and not one.** The wall clock jumps — NTP steps and slews it, and a suspend advances it while the monotonic clock stops — so a deadline measured against it would move under the program. The monotonic clock never jumps, but its origin is arbitrary and no API reveals it, so a `Tick` on its own cannot say what day it is. Keeping them apart makes the mistake a *type* error rather than a plausible wrong answer: `between` takes two points of one timeline, and comparing an `Instant` with a `Tick` is refused where it is written. Measure with ticks; record and report with instants.

The arithmetic is named functions, because operators are numeric-only: `millis(1500)` and its siblings build a span, `to_millis` and its siblings read one back, `plus`/`minus`/`times`/`abs` compute, and `between(start, end)` answers the span from one point to another — signed, so the argument order *is* the direction of the answer. Interpolating a `Duration` prints the largest unit that divides it exactly (`1500ms`, `120s`, `37ns`).

**Reading a clock is a capability.** A function whose answer depends on when it was called has a dependency, and Salvo's dependencies live in signatures:

```
effect Ticker { fn tick() -> Tick }               // the monotonic clock
effect Clock  { fn now() -> Instant  … }           // the wall clock
```

`DefaultTicker` and `DefaultClock` are the machine's. `Clock` also carries the bridge between the timelines — `to_instant(at)` and `to_tick(at)` — and those are *members* rather than free functions because the answer is an estimate: nothing exposes the monotonic origin, so relating the two means reading both clocks at nearly the same moment and keeping the difference, which then drifts. A handler owns that correlation, which is what makes the conversion available at all, and exact in a test.

**A deadline is a message.** Sleeping is an `actor effect`, and it could not be anything else: a handler runs to completion and cannot block mid-body, so "in two seconds" can only mean "park a continuation and resume me".

```
struct Fired { at: Tick }

actor effect Timer {
    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done
}
```

`after` takes the continuation and answers nothing; the caller mints one with `replyto` inside a handler, or `waitfor` anywhere that may occupy its thread. The fire carries a `Tick`, because a deadline that moved when the wall clock was adjusted would not be a deadline. `DefaultTimer` is spawned like any actor (`spawn DefaultTimer() on pool(1)`) and keeps one deadline structure and one thread for the whole program, however many deadlines are outstanding. There is no cancellation: a timer nobody wants any more fires into a continuation that finds its work already done.

### Time in a test

Because every clock is an effect, a test replaces it — and `ManualTime` is the replacement std ships, in pure Salvo: virtual time starts at zero and moves only when told, so a program that would wait two seconds runs in microseconds and prints the same thing every time. It wears two faces, so a spawn answers one addr per face and least authority falls out of the types — the code under test is handed the `Timer` and *cannot* reach `advance`:

```
let (timer, ctl) = spawn ManualTime() on p
let sessions = spawn Sessions() with timer on p

sessions.open(order, answer)
waitfor settled: Reply<Idle> { on_idle(p, settled) }   // let it register first
ctl.advance(millis(2500))
```

The middle line is not optional in spirit: `advance` is a message like any other, so without it the advance races the `after` the code under test has not registered yet. Quiescence is the sequencing tool, which is what `on_idle` is for.

**The posture the module is built around is to pass time rather than read it.** A `Fired` carries the `at` it came due at; a request can be stamped where it enters the system. A function that takes its times as parameters declares no effect, needs no handler, and is tested by being called:

```
fn verdict(started: Tick, at: Tick, budget: Duration) [] -> Str { … }
```

That is a stance rather than a mechanism, and the reason for it is not only testability: a function that reads an ambient clock in the middle of its body has an answer that depends on when the scheduler ran it, which inside an actor is a race with its own mailbox. Passing the time in removes the dependency instead of faking it.

Where a reading genuinely cannot be passed in — how long a handler's *own* work took is not something its caller could have stamped — the reading and the deadlines must agree, and they are made to agree by writing a clock over the timer the test advances. A reading is a deadline of zero:

```
handler TestTicker(timer: Addr<Timer>) of Ticker {
    fn tick() -> Tick {
        let fired = waitfor answer: Reply<Fired> { timer.after(nanos(0), answer) }
        return fired.at
    }
}
```

One virtual clock is then behind both, so a measurement taken across a two-second virtual nap is exactly two seconds. Two existing rules shape this handler and are worth reading off it. The timer arrives as a **value** — an `Addr<Timer>` constructor parameter — because a handler with dependencies of its own cannot be *constructed* in a spawn's `with` clause: there is no scope on the child to resolve them from. And the wait is declared nowhere: occupancy is inferred, the wait serves its pool while it waits, and the deadlock graph prices any cycle it could close. The round trip per reading is why this is the posture of last resort rather than the default.

## Specific backend details

### Kotlin

* When `None` is the only return type of a function, it should be translated to `Unit`.
* The backend should define generic union type wrappers using a sealed interface. If the larger union type is of size N, then the backend should define union types for each number from 1 to N. The qualifier checks then reduce down to checking which of the sealed types a value results in.
* Effects and handlers can map to interfaces and implementations of those interfaces. The effects are passed to a function as the first arguments of that function, and all uses of those effects is mapped to the relevant parameter name.
* `Mut Str` maps to `StringBuilder`, which — unlike `MutableList<T>` — is *not* a subtype of the immutable form, so dropping the `Mut` emits `.toString()`. `copy` of a `Mut Str` is `StringBuilder(sb)`, not the identity.
* `Byte` maps to `UByte`, not to Kotlin's signed `Byte`: an octet has to print and compare the same on both backends, and a signed byte would render 255 as `-1` where Rust's `u8` renders `255`.
* `Bytes` and `Mut Bytes` both map to one **shipped runtime class** (`salvo.SalvoBytes`, emitted per program that names the type): a growable byte array with structural `equals`/`hashCode` and an `iterator()`. Neither stdlib shape would do — `List<UByte>` boxes every element, and `UByteArray` is fixed-size *and* is not a `List<T>`, so generic code could not take one. Rust needs no such class: `Bytes` is a `Vec<u8>`.

### Rust

* When `None` is the only return type of a function, the return type is omitted (`()`).
* `T?` maps to a physical `Option<T>`; union types map to generated enums (`Union2<T1, T2>` with one variant per non-`None` arm).
* Deductions determine ownership: a parameter that appears in a function's deductions is passed by reference (`&T`, or `&mut T` when its declared type carries `Mut`), while a parameter omitted from the deductions is moved (passed by value) — the calling code no longer has access to it in Salvo, so the move is always legal. Copy scalar types are always passed by value.
* Effects map to traits with `&mut self` methods; effect dependencies become leading `&mut dyn` parameters, and `use` instantiates a handler into a local that is threaded as `&mut local`.
* `Str` and `Mut Str` are both `String`, so dropping a `Mut` emits nothing. String indexes are *characters*, not bytes, on both backends, so the lowerings convert where Rust counts bytes.
* See BACKEND_SPEC.rust.md for the full rules.