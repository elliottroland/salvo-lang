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

Of these, `Byte`, `Int`, `Long`, `Float`, `Double`, `Bool` and `Char` are the **basic types**: scalars whose operators are the host language's own (an operator is the call it names — `eq(a, b)`, `cmp(a, b)`, `plus(a, b)` — and std's are the host's operators; see [Comparison, equality and hashing](Comparison-and-Hashing.md#what-an-operator-calls)). `Bytes`, `None` and `Str` (below) are not, so every operation on them is a function call.

### Arithmetic

All numbers include the usual arithmetic operations: `+`, `-`, `*`, `/`, `%` and `**` work on `Int`, `Long`, `Float` and `Double` (and unary `-` on the same; `**` as below). `/` between integers is integer division on every backend. `+` does not concatenate strings — `${}` interpolation is how text is built — and `&&`, `||` and `!` take `Bool` operands only: Salvo has no truthiness in value position any more than in conditions.

Like ordering and equality, the arithmetic operators are **the calls they name**: `a + b` is `plus(a, b)`, `a - b` is `minus(a, b)`, `a * b` is `times(a, b)`, `a / b` is `divide(a, b)` and `a ** b` is `pow(a, b)`, chosen exactly as the written call would be. The numbers' functions are std's, declared beside the types, and the backends write the host's own operator for them. A type of your own gets an operator by declaring the function **in its own file** — any right operand, any result:

```
struct V { x: Int, y: Int }

fn plus(a: V, b: V) -> V { return V { x: a.x + b.x, y: a.y + b.y } }
fn times(a: V, k: Int) -> V { return V { x: a.x * k, y: a.y * k } }

let w = (a + b) * 3
```

An implicit of the function you are in comes first, as for every call: inside `fn total<T>(a: T, b: T, ?plus: (a: T, b: T) -> T)`, `a + b` is that `?plus` — the only way generic code can add, and the error at a `T` without one says to ask for it. An available effect's member `plus` is a candidate like a function. Ordering (`<`, `<=`, `>`, `>=`) and equality (`==`, `!=`) work the same way: `a < b` means `cmp(a, b) < 0` and `a == b` means `eq(a, b)` (see [Comparison, equality and hashing](Comparison-and-Hashing.md)).

`%`, unary `-` and `++`/`--` name no function: they work on the numbers alone.

**`**` is the power operator.** It binds tighter than `*` and than a `-` on its left, and it is right-associative: `-2 ** 2` is `-4`, `2 ** 3 ** 2` is `2 ** 9`, and the exponent may be negative (`2.0 ** -1`). Nothing is widened, since an exponent is not a base: std has `Int ** Int`, `Long ** Int`, `Double ** Int` and `Double ** Double`. An integer power wraps like the rest of integer arithmetic, and a negative integer exponent is an error at run time, as a division by zero is; `Double ** Int` is computed by repeated multiplication, identically on every backend, and `Double ** Double` is the host's `pow`.

`x += e` and its three siblings (`-=`, `*=`, `/=`) are shorthand for `x = x + e`, so they follow the same operand rules — `+=` on a `Str` is refused like `+` is, and `v += w` on a type with a `plus` calls it — and they are statements rather than expressions, so they do not chain. `x++` and `x--` remain for a step of one, where the value of the expression matters. An assignment's left side must be a *place*: a variable, a field path, or a subscript.

Mixed widths widen implicitly **within** a class: `Int + Long` computes at `Long`, `Float < Double` compares at `Double` — the compiler records the promotion and each backend renders it its own way. An operator widens before it calls its function, so `i + l` is `plus` at `Long` and `i < l` compares at `Long`. Mixing the integer and float classes never happens implicitly: `1 + 2.5` is a type error naming the explicit conversions — `to_int`, `to_long`, `to_float` and `to_double`, declared in `std.core.basic` for every source width, truncating toward zero and saturating at the target's bounds identically on both backends (`to_int(Long)` keeps the low 32 bits).

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

A float's **text** is the same on every backend: the shortest digits that
round-trip, always a decimal point, and scientific notation outside
`[10^-3, 10^7)` — so `${2.0}` is `2.0`, `${0.0001}` is `1.0E-4`, `${1234567.0}`
is `1234567.0` and `${12345678.0}` is `1.2345678E7`, with `NaN`, `Infinity` and
`-Infinity` for the specials. That holds wherever a float becomes text: on its
own, inside a list or a map value, and as a struct field.

Integer arithmetic **wraps** on overflow: `Int` and `Long` `+`, `-`, `*`, `**`, unary `-` and `++`/`--` keep the low 32 or 64 bits, the same on both backends, so `2147483647 + 1` is `-2147483648`. Bitwise operations are functions rather than operators, since `&`, `|`, `^` and `>>` already mean other things in the grammar: `bit_and`, `bit_or`, `bit_xor`, `bit_not`, `bit_shl`, `bit_shr` (keeps the sign) and `bit_ushr` (fills with zeros), each for `Int` and `Long`. A shift count is taken modulo the width, so `bit_shl(1, 33)` is `2`.

Numeric literals default to `Int` and `Double`: `1` is an `Int` and `1.2` is a `Double`. Suffixes select the other widths: `1L` is a `Long`, and `1.2f` is a `Float` (the `f` suffix requires a decimal point — write `1.0f`, not `1f`). Underscores may separate digits (`1_000_000L`). An **unsuffixed** literal also *adopts* the numeric type its position expects — `let x: Long = 1`, `let d: Double = 3` and passing `1` to a `Long` parameter all work, and `x + 1` needs no `1L` because the operator widening covers it. Adoption is for literals only: an `Int` *variable* never becomes a `Long` implicitly — write `to_long(n)`.

## Strings

Salvo strings work much the same way as in Kotlin: they are immutable, and can be defined using literals:

```
let str: Str = "hello, world!"
```

There is no equivalent to Rust's string literal type `str`.

A string being immutable does not mean building one has to be quadratic: `Str` opts into the `Mut` auto-qualifier ([Qualifiers](Qualifiers.md)), so `Mut Str` is *a string under construction*. It is asked for explicitly — a literal is never a `Mut Str` — and it is the only place the string functions come in a mutating flavour:

```
let text: Mut Str = Mut Str("hello")
append(text, ", world")
set(text, 0, 'H')          // answers whether it wrote; ignorable
println(text)              // Hello, world
```

Everything else in `std.core.string` takes a plain `Str`, and a `Mut Str` reaches all of it by *dropping* its `Mut` like any other qualifier — `size(text)`, `trim(text)`, `text == other` all work. The difference from every other qualifier is invisible in Salvo and matters to the backends: `Mut` is the one qualifier a target may render as a different type (a `StringBuilder` on the JVM), so dropping it there is a real conversion rather than a widening. The compiler records the drop and each backend renders what it needs, which is what keeps `text == other` a comparison of *characters* on both targets.

**An interpolation may hold a string literal**, and that literal may interpolate in turn: `"${name ?: "unknown"}"` is an ordinary thing to write, and the inner quotes belong to the inner literal. Escapes work inside it as anywhere else.

**Interpolation needs a text form.** Each `${value}` is the call `to_str(value)`, and it resolves exactly as that call would if you wrote it there: the standard library has a `to_str` for every scalar and for `Str`, a type may declare its own, an effect in scope may provide one, and a function's own `?to_str` parameter takes it first. A union is the text of the arm it holds: it interpolates when every arm has a text form of its own (a scalar, `Str`, or a type with a `to_str`), and a `to_str` that takes the union itself takes precedence:

```
struct Point { x: Int, y: Int }

fn to_str(p: Point) -> Str => p {
    return "(${p.x}, ${p.y})"
}

println("at ${point}")     // uses the `to_str` above
```

Without one, the compiler says so rather than letting the backend fail. The standard library renders a list this way — `"${List(1, 2, 3)}"` gives `[1, 2, 3]` — and a `Mut Str` needs nothing special, since it drops its `Mut` first.

**A type's `to_str` lives in the type's own file.** `to_str` is what interpolation calls, so a `to_str(p: Point)` may be declared only in the file that declares `Point` — anywhere else it is an error where it is written, used or not, and the same goes for a catch-all `to_str<T>(x: T)`, which would be every type's at once. Another rendering is a function of another name, passed where it is wanted (`to_str = as_hex`). The rule is the same for every operator; [Comparison, equality and hashing](Comparison-and-Hashing.md#one-per-type-where-an-operator-is-declared) has it whole.

**A struct interpolates by opting in.** `: ToStr<self> by auto` stamps a
`to_str` that renders the struct in the shape its literal has, identical on
both backends:

```
struct Person : ToStr<self> by auto { name: Str, age: Int }
println("${Person {name: "ann", age: 3}}")    // Person { name: ann, age: 3 }
```

Each field is interpolated in turn, so a nested struct renders with its own
`to_str` — and needs one, or the error lands on that field. A `to_str` written
by hand is the other way in. The format is the language's own on purpose:
leaving it to each target would print different text on the JVM than in Rust.
How `by auto` works is [Compile-time functions](Compile-Time-Functions.md).

**Declaring the obligation is optional.** A type may say `: ToStr<self>`, which does not change how interpolation works — it checks at the declaration that a matching `to_str` exists, so the mistake surfaces where the type is defined rather than where it is printed.

**Reading a value back out of text** is `from_str`, the dual of `to_str`. It
returns `None` for text that does not spell a value, and it is overloaded by
its return type ([Functions](Functions.md#overloading-by-return-type)), so the
type the result goes to chooses the reader:

```
let port: Int? = from_str("8080")       // 8080
let ratio: Double? = from_str("2.5e-1") // 0.25
let on: Bool? = from_str("yes")         // None: only `true` and `false`
```

`std.core.string` has it for `Str`, `Int`, `Long`, `Double` and `Bool`, under the
named functions `parse_int`, `parse_long`, `parse_double` and `parse_bool`.
They accept the same text on every backend: an optional sign, digits, and for
a `Double` a fraction and an exponent. Whitespace, `NaN`, `inf`, hex and type
suffixes are refused everywhere, rather than accepted by whichever host
happens to. A generic function that needs a reader asks for `?FromStr<T>`.

The behavior of Strings are governed by the module `std.core.string`.

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

### Unions of literals

A literal is a type too — the type whose one value is that literal — and a union of them names a set of values:

```
type Direction = "north" | "south" | "east" | "west"
type Retries = 0 | 1 | 3
```

Strings, integers (negative ones too), `Long`s (`3L`) and `Bool`s can be literal types. A float cannot: equality on floats does not name a value.

A literal takes its literal type from where it is used and nowhere else. `let d: Direction = "north"` is a `Direction`, a `"north"` passed to a `Direction` parameter is one, and `let s = "north"` is a plain `Str`. A literal the union does not list is an error that names the listed values, so a typo is caught where it is written. A plain `Str` is not a `Direction` either, since it could hold anything: narrow it first. `is` with several literals does that in one test, comparing the value:

```
fn direction(s: Str) [] -> Direction? => s {
    if s is "north" | "south" | "east" | "west" {
        return s                         // a `Direction` here
    }
    return None
}
```

The same form tests several arms of any union: on an `Int | Str | Bool`, `x is Int | Str` narrows `x` to `Int | Str`. It means the tests joined with `||`, which narrows the same way, so it is written over a variable or a field (bind a call's result first), and binds nothing.

`is` tests a literal, and `when` over a union of literals is exhaustive, as over any union:

```
fn arrow(d: Direction) [] -> Str => d {
    when d {
        is "north" { return "↑" }
        is "south" { return "↓" }
        is "east" { return "→" }
        is "west" { return "←" }
    }
}
```

A union of literals widens to their base type wherever the base is expected: a `Direction` passes where a `Str` is wanted, interpolates, and compares with `==` against a literal. Unions compose, so a named sub-union is a group you can test in one go:

```
type Kms = "KmsDisabled" | "KmsThrottled"
type Code = Kms | "QueueDoesNotExist" | Other Str

if code is Kms { … }
```

**Open sets.** Many sets of values are open: a service may add a code this program has never heard of. `Other`, a qualifier in `std.core`, is the open arm: `"A" | "B" | Other Str` is the listed values and any other string. A value reaches the `Other` arm only by being built with `other("…")`, so everything else still has to be one of the listed literals; `is Other` narrows to the rest.

**At run time a literal is its base.** A union of one base type's literals is that base type on both backends — `Direction` is a `String` — and the `Other` arm, being a `Str` too, collapses with them. In a union that mixes bases, each base's literals share one arm: `"name" | 3 | "other" | Person` is a `Str | Int | Person` at run time, its arms in the order they first appear. Since two literals of one base are the same host type, two overloads that differ only in literal types (`fn pick(x: "A")` and `fn pick(x: "B")`) are refused; take the union in one function and `when` over it. `"a" | Str` is a warning — every `Str` is already an arm — pointing at `Other Str`.

**Qualified literals.** A literal can carry a qualifier like any other type: `Short "port" | Short "verbose" | "host"`. The qualifier erases, so at run time this is still one `Str`, and `is Short` is the test "the value is `"port"` or `"verbose"`". Because the value alone decides, a union may not list one literal twice under different qualifiers (`Short "port" | "port"`); that is an error naming both arms.

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

Function parameters can give a default the same way — see
[Default parameter values](Functions.md#default-parameter-values).

A default must have the field's type. The one thing it may lack is a predicate qualifier with constant arguments, such as `InRange(1, 65535)` ([Qualifiers](Qualifiers.md)): `port: InRange(1, 65535) Int = 8080` is accepted, and the claim is tested each time the default is used, so a default that does not satisfy it stops the program at that literal with a message naming the field. Parameter defaults and a handler's state initialisers follow the same rule.

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

A field whose value is a variable of the same name can be written by the name
alone — the inverse of destructuring. The type name is required for this, since
a bare `{ name }` is a set:

```
let name = "Roland"
let age = 36
let person = Person {name, age}     // Person {name: name, age: age}
```

When the type of a struct is known, then the type annotation can be dropped:

```
let person: Person = {name: "Roland", age: 36}
```

A struct may not **contain itself**, directly or through other structs: a value
of it would be infinitely large, so the declaration is an error naming the
field that closes the cycle.

```
struct Node {
    value: Int,
    next: Node?      // error: `Node` contains itself through field `next`
}
```

Indirect through a container instead, which is how a tree or a linked structure
is written — a `List` holds any number of them, including none:

```
struct Tree {
    value: Int,
    kids: List<Tree>     // fine, and so is `Tree[]`
}
```

Only edges that lay a value out *inline* count: a field of struct type, a tuple
element, a union arm, a nullable's inner type, an alias expansion, and a type
argument a generic struct stores inline (`Wrapper<Looped>` where `Wrapper<T>`
has a `value: T`). A `List`, `Set`, `Map`, array, function type or `proj` field
indirects already, so recursion through one is legal — including a mutually
recursive pair (`Dir` holding `List<File>` and `File` holding `List<Dir>`).

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
fn next(p: Mut ListNext<T>) -> Emitted T | Finished => p: Mut {
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

`?:` is `!` with a path instead of a panic. It is the **positional pick**: a run
of `?`s names how many arms of the subject's union it keeps, counted from the
front, and the picked value is the subject when it holds one of them — otherwise
the right side runs. On an optional, `?:` keeps the one value arm:

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

The pick selects by **position** over the subject's declared arm order, not by
name. `?:` keeps the first arm; `:?` keeps the last; a run of `?`s keeps that
many. On `Ok Int | Err Str`:

```
fn tail(text: Str) -> Ok Int | Err Str {
    let e: Err Str = parse(text) :? return _   // keep the last arm, Err Str
    return e
}
```

`_` is **the arms the pick left behind**, tags and all — the sub-union on the
opposite side from the pick's direction. In `?: return _` on `Ok Int | Err Str`,
`_` is the `Err Str`, so the right side can hand it to something that expects an
error. That is the one place the placeholder is needed, and it reads only inside
a pick's right-hand side.

A pick whose right side leaves also **narrows its subject** below, because the
subject must have been one of the picked arms for the code to be running:

```
let v: Str = t ?: return        // `t` reads as `Str` from here on
let r2: Ok Int = r ?: return    // `r` reads as `Ok Int` — the picked arm
```

A right side that yields a value narrows nothing, since that is the path where
the subject was not picked.

The subject is evaluated once, so it can be a call. A pick that keeps **every**
arm (so the right side is unreachable) is an error — the same dead-scaffolding
refusal a throw-free `try` gets.

**Widening is allowed**: a right side whose type is outside the subject's arms
joins into the result, so `n ?: "none"` on an `Int?` is an `Int | Str`.

The operator works on any union: `??:` on `Str | Int | None` keeps the first two
arms, `Str | Int`. Its precedence is Kotlin's — tighter than `is` and comparison,
looser than arithmetic — and it is **left-associative**, so `count ?: 0 > 3`
reads as `(count ?: 0) > 3` and `a ?: b ?: 0` is `(a ?: b) ?: 0`, each pick
handling the arms the last left behind. Either form may start a continuation
line, so a long chain can wrap (the newline brackets everything before it on the
left):

```
let value = read_value()
    ?: fallback()
    ?: default()
```


The safe-navigation operator `?.` reaches through the **first arm** of a
union: `a?.b` performs `.b` on that arm and replaces it with the result,
passing the other arms through. For a `T?` that is the familiar reading —
`a?.b` is `None` when `a` is, otherwise `a.b` lifted back into an optional —
and `a?.m(args)` is the dot call `a.m(args)` on that arm (a member of the
arm if it has one, otherwise a function taking it first; see
[dot calls](Functions.md)). On a general union `(A | B | C)?.field` where
`A.field: F` is `F | B | C`. It guards **one** link, so a chain spells every
hop: `notice.metadata.last_updated?.nanos` is a plain read of a non-optional
`last_updated`, while `o?.inner?.nanos` guards both. It pairs with `?:` for a
default — `last_updated?.nanos ?: -1L` — the two staying separate operators.
A method-call link needs a variable or field as its receiver (it is read
twice); bind a longer chain to a local first.

To feed the first arm through an arbitrary expression the same way, the
null-safe pipe `?>` is the pipe analogue of `?.`: `number ?> to_str(_)` runs
`to_str` with `_` bound to the first-arm value and replaces that arm with the
result, passing the rest through — see "Operators and the pipe" in
[Control Flow](Control-Flow.md).

To get a non-null value out when the compiler cannot prove it (equivalent to
Rust's `unwrap`):

```
let person: Person = Person {name: "Roland", surname: "Elliott", age: 36}

// Haven't done an explicit check, but we know `surname` is `Str` because we just constructed it
println("Surname is ${person.surname ?: unreachable!()}")
```

An optional (or any union) may also be compared with `==`/`!=` against a value
whose type matches one of its arms: `x == -1L` on a `Long?` asks "is `x` present
and equal to -1", with a `None` -- or any other arm -- comparing unequal. The
value's type picks the arm, so this works for any union, not just optionals; it
is `==`/`!=` only (ordering still needs the value narrowed first).
