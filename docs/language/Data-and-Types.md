# Data and Types

## Basic data types

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

All numbers include the usual arithmetic operations, on **numeric operands only**: `+`, `-`, `*`, `/` and `%` work on `Int`, `Long`, `Float` and `Double` (and unary `-` on the same). `/` between integers is integer division on every backend. `+` does not concatenate strings — `${}` interpolation is how text is built — and `&&`, `||` and `!` take `Bool` operands only: Salvo has no truthiness in value position any more than in conditions. Ordering (`<`, `<=`, `>`, `>=`) and equality (`==`, `!=`) are **capabilities**, not built-in operations: `a < b` means `cmp(a, b) < 0` and `a == b` means `eq(a, b)`, so they work wherever those functions exist. Numbers, and the other intrinsic types, come with theirs (see [Comparison, equality and hashing](Comparison-and-Hashing.md)).

`x += e` and its three siblings (`-=`, `*=`, `/=`) are shorthand for `x = x + e`, so they follow the same operand rules — `+=` on a `Str` is refused like `+` is — and they are statements rather than expressions, so they do not chain. `x++` and `x--` remain for a step of one, where the value of the expression matters. An assignment's left side must be a *place*: a variable, a field path, or a subscript.

Mixed widths widen implicitly **within** a class: `Int + Long` computes at `Long`, `Float < Double` compares at `Double` — the compiler records the promotion and each backend renders it its own way. Mixing the integer and float classes never happens implicitly: `1 + 2.5` is a type error naming the explicit conversions — `to_int`, `to_long`, `to_float` and `to_double`, declared in `core.basic` for every source width, truncating toward zero and saturating at the target's bounds identically on both backends (`to_int(Long)` keeps the low 32 bits).

```
let count = 42          // Int, the default for a whole number
let total = 42L         // Long
let ratio = 1.5         // Double, the default for a fraction
let small = 1.5f        // Float
let ok = true           // Bool
let initial = 'S'       // Char

let widened = count + total          // computes at Long: 84
let half = 7 / 2                     // integer division: 3
```

Numeric literals default to `Int` and `Double`: `1` is an `Int` and `1.2` is a `Double`. Suffixes select the other widths: `1L` is a `Long`, and `1.2f` is a `Float` (the `f` suffix requires a decimal point — write `1.0f`, not `1f`). Underscores may separate digits (`1_000_000L`). An **unsuffixed** literal also *adopts* the numeric type its position expects — `let x: Long = 1`, `let d: Double = 3` and passing `1` to a `Long` parameter all work, and `x + 1` needs no `1L` because the operator widening covers it. Adoption is for literals only: an `Int` *variable* never becomes a `Long` implicitly — write `to_long(n)`.

## Strings

Salvo strings work much the same way as in Kotlin: they are immutable, and can be defined using literals:

```
let str: Str = "hello, world!"
```

There is no equivalent to Rust's string literal type `str`.

A string being immutable does not mean building one has to be quadratic: `Str` opts into the `Mut` auto-qualifier ([Qualifiers](Qualifiers.md)), so `Mut Str` is *a string under construction*. It is asked for explicitly — a literal is never a `Mut Str` — and it is the only place the string functions come in a mutating flavour:

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

## Tuples and Unions

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

It makes no sense to have duplicate types in a union (unless they are qualified — see [Qualifiers](Qualifiers.md)): `Str | Str` is equivalent and simplified to `Str`.

The type of a variable can never get more general (although its qualifiers can change — see [Qualifiers](Qualifiers.md)), and we don't support variable shadowing. In the above example, it is only because the type of `string_or_number` _started_ as a union, it could move between `Str` and `Int`.

## Structs

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

## Nullability via "| None"

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

On a value that is **not** a union, a pick of a predicate qualifier is the runtime test — the same duality `is` has, reached from `?:` instead of a condition. The claim is *applied* when it holds, which is what makes a bounds-checked loop read without a single `!`:

```
let i_child = i * 2 + 1 Idx(heap)?: break
// i_child is an Idx(heap) Int here: get(heap, i_child) answers the element
```

The right side runs when the claim does not hold — `_` there is the plain subject — and a right side that yields a value drops the claim (`5 Idx(xs)?: 0` is an `Int`): a claim and its absence are one representation, never two union arms.

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
