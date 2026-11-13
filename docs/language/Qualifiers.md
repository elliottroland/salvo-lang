# Qualifiers

## Overview

A **qualifier** is a claim about a value, written as part of its type.
`NonEmpty List<Int>` is a list the compiler knows has at least one element;
`Positive Int` is an `Int` known to be greater than zero; `Mut Person` is a
`Person` you are allowed to change. The value underneath is the same list,
`Int` or `Person`. The qualifier only records something more that is known
about it, and that extra knowledge decides which functions you may call.

```salvo
qualifier Positive for Int {
    fn qualifies(n: Int) -> Bool {
        return n > 0
    }
}

// Only accepts numbers already known to be positive.
fn reciprocal(n: Positive Int) -> Double {
    return 1.0 / to_double(n)
}

fn show(n: Int) [Console] -> None {
    if n is Positive {
        println("${reciprocal(n)}")    // `n` is a `Positive Int` here
    } else {
        println("no reciprocal for ${n}")
    }
}
```

A qualifier has no behaviour of its own. What it means is entirely defined
by the functions that ask for it, the tests and functions that grant it, and
the operations that take it away. In the example, `qualifies` is the test,
`reciprocal` is a function that asks for the claim, and `n is Positive` grants
it inside the `if`.

If you know other languages, these comparisons help:

* **TypeScript narrowing.** `if (typeof x === "string")` gives `x` a narrower
  type inside the branch. Salvo's `is` does the same for unions, and also for
  claims you declare yourself: `n is Positive` narrows `n` from `Int` to
  `Positive Int`.
* **Rust newtypes.** In Rust you might write `struct NonEmptyVec<T>(Vec<T>)`
  so the type system remembers a check. A qualifier records the same fact
  without a wrapper: a `NonEmpty List<T>` *is* a `List<T>`, every list
  function still works on it, and the claim is erased from the generated
  code.
* **Refinement types.** A qualifier is a named, lightweight refinement: a
  predicate attached to a type, proved once and then carried along.

Claims arrive in two ways. A **predicate** qualifier like `Positive` has a
test, so any value can be checked with `is`. A **constructive** qualifier has
no test, and values only gain it from functions declared to produce it. Claims
also leave: a function that changes a value's contents can make a claim about
those contents false, so the compiler forgets it (see
[Deductions and ownership](Deductions-and-Ownership.md)).

The rest of this page covers declaring qualifiers and overloading on them;
`Mut`, the built-in qualifier for mutable values; the two ways a claim is
established; how qualifiers sit inside unions and tuples; the difference
between claims about a value's contents and claims about its origin; and
refinements, which let a qualifier say what other people's functions do to
its claim. Claims about a value's relation to *another* value, such as "this
index is valid for that list", have their own page,
[Dependent qualifiers](Dependent-Qualifiers.md).

## Declaring a qualifier

A qualifier is declared for the type it applies to:

```salvo
qualifier Surname for Person
```

`Surname` is the name of the qualifier, and `Person` is the type it applies
to. A `Surname Person` then behaves, for overloading, like a type of its own,
so you can write a more specific version of a function for values that carry
the claim:

```salvo
// Does not know there's a surname, so it has to check the optional field.
fn full_name(person: Person) -> Str => person {
    if person.surname is Str {
        return "${person.name} ${person.surname}"
    }
    // Returns are explicit, unlike in Rust.
    return person.name
}

// Elsewhere we define that `Surname Person` means that the surname is
// present, so this version can use it directly.
fn full_name(person: Surname Person) -> Str => person {
    return "${person.name} ${person.surname ?: unreachable!()}"
}
```

The `=> person` after each signature is a *deduction clause*, which says the
function keeps its argument rather than consuming it; it is explained in
[Deductions and ownership](Deductions-and-Ownership.md). A call with a
`Surname Person` picks the second overload, and a call with a plain `Person`
the first ([Functions](Functions.md) describes how overloads are ranked).

Several qualifiers can apply to one value at the same time, in any order.
Qualifiers compose freely: there is no compatibility clause to declare.

```salvo
// Old and Surname both apply, with nothing extra to declare.
let p: Old Surname Person | None = check_old_surname(person)
```

The same qualifier cannot be applied twice to one type: `Old Old Person` is
an error. Nested qualifiers *are* possible, as a qualifier on a parenthesized
union group (see [Qualifiers in unions and tuples](#qualifiers-in-unions-and-tuples)).

Whether a value can genuinely carry two particular claims at once is not
stated by any pairwise relation. It is implicit in which functions mint each
qualifier and in each qualifier's own `qualifies` test.

A qualifier is its declaration, not its name, as a type is. Two modules may
each declare a `Pos`, and they are two different claims: a value that one
module's `Pos` was proved of is no proof of the other's, though both are
written `Pos`, and each `is Pos` test runs its own module's `qualifies`. In
the same way, a program that declares its own `NonEmpty` over `List` has a
claim of its own beside std's: std's functions establish std's `NonEmpty`,
and a function asking for the program's is not satisfied by them.

The word `with` appears elsewhere in Salvo, where it cannot be confused with
a qualifier: after a `use` or `spawn`, `with` names the dependency instances
to supply (see
[Effects and handlers](Effects-and-Handlers.md#inheriting-the-scope-and-overriding-it-with-with)).
Declaring that a type *may carry* a qualifier is `canbe`, described next.

## Auto-qualifiers and `Mut`

Some qualifiers are built into the language rather than declared. These
**auto-qualifiers** are opted into where a type is declared, with `canbe`:
`canbe Q` reads as "values of this type may be `Q`". The declaration opts in,
and individual values gain the qualifier where the language says so. The
auto-qualifier you will use most is `Mut`, which introduces a mutable version
of the type. (A type *parameter* can opt in to linear values the same way,
`T canbe linear`; see [Linear types](Linear-Types.md).)

Values are immutable by default. A `Mut` struct value is one whose fields can
be assigned:

```salvo
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

This is close to Kotlin's `List` and `MutableList`, or Rust's `let` and
`let mut`, except that the permission travels with the value's type: a
function parameter typed `Mut Person` may change the caller's person, and one
typed `Person` may not. Passing a `Mut Person` where a `Person` is expected is
always allowed, since forgetting a permission is safe.

`Mut` is a general language feature, not something a library defines. It
composes with every other qualifier, and the backends give it meaning
(mutable fields in Kotlin, `mut` bindings and `&mut` references in Rust).
Besides structs, other type declarations opt into it with the same
`canbe Mut` syntax; the standard library's list and string types are
declared as:

```salvo
intrinsic type List<T> canbe Mut
intrinsic type Str canbe Mut
```

Applying `Mut` to a type whose declaration does not say `canbe Mut` is a
compile-time error. How a backend maps a `Mut` type is described in
[Backends](Backends.md).

### Fields that follow the struct's `Mut`

A field holding something that can itself be `Mut`, such as a list, can
follow the struct's own `Mut` by writing `canbe Mut` on its type. The field is
then `Mut` exactly when the struct value is:

```salvo
struct Inventory canbe Mut {
    owner: Str,
    items: List<Str> canbe Mut
}

let shop = Mut Inventory {owner: "Ada", items: Mut List<Str>()}
add(shop.items, "lamp")             // `shop.items` is a `Mut List<Str>`

let frozen: Inventory = shop
add(frozen.items, "rope")           // Compile-time error: here it is a `List<Str>`
```

So a plain `Inventory` is immutable all the way down, which a field written
`items: Mut List<Str>` would not be: that list stays mutable inside any
`Inventory`.

The rules for such a field:

* A `Mut` literal gives the field a `Mut` value.
* A `Mut` literal cannot take the field from a plain value by `...spread`, or
  from a default. Both would make something mutable that a plain value still
  shares.
* The struct must itself say `canbe Mut`, and the field's type must be one
  that can be `Mut`.
* `copy` of the struct copies the field with it, so the copy's list is its
  own.

### Dropping `Mut`

Dropping a qualifier is ordinarily free: it only forgets a claim, and a
`Mut List<T>` used as a `List<T>` really is the same value. `Mut` is the one
qualifier whose removal can cost something. A backend may render `Mut T` as a
*different type* from `T` (Kotlin's `Mut Str` is a `StringBuilder`, which is
not a `String`), and there the drop is a conversion.

Salvo hides that. The compiler records every place a `Mut` is dropped, and
the backend supplies whatever conversion it needs at each one: arguments,
returns, annotations, struct fields, union arms, interpolation and operators.
Nothing in the source changes, and a `Mut Str` behaves like the `Str` it is
being used as.

## How a qualifier is established

A qualifier arrives on a value in one of two ways:

* **by predication**: a test proves the claim holds (`n is Positive`);
* **by construction**: a function says it establishes the claim, and its
  callers receive the qualified type.

Both declare their machinery inside the qualifier, or beside it in the same
file, so a claim and the way to obtain it live together. The next two
sections take them in turn.

## Predicate qualifiers

A predicate qualifier is one whose claim can be *tested*. It defines a
function `qualifies` inside the qualifier, which takes a value of the given
type and returns a `Bool`:

```salvo
qualifier Positive for Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

qualifier NonNegative for Int {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

qualifier Negative for Int {
    fn qualifies(int: Int) -> Bool {
        return int < 0
    }
}
```

`qualifies` takes no deduction clause: a test can only ever add to what is
known about a value, and it can never move the value. It can require effects,
and then those effects must have handlers in the context of each test, like
any other function call.

A claim is tested with the same `is` syntax that narrows a union
([Control flow](Control-Flow.md)). It is known statically whether an `is`
names a qualifier or a type. Applied to a value of the qualifier's type that
is not a union (here, an `Int`), `is` calls `qualifies` and, when it answers
`true`, gives the value the qualified type:

```salvo
let number: Int = random_int()
if number is Positive {
    // `number` is of type `Positive Int` in this block
}
```

### Giving a value a qualifier as you write it

`is` tests a value you already have. To attach a qualifier to a value where
it is written, `as!` and `as?` test it for you:

```salvo
let age = 36 as! NonNegative        // a `NonNegative Int`, or a trap
let maybe = input as? NonNegative   // a `NonNegative Int`, or `None`
```

`x as! Q` is `{ let v = x; assert!(v is Q); v }`: it traps, with the same
message `assert!` gives, when `x` does not qualify. `x as? Q` is
`{ let v = x; if v is Q { v } else { None } }`, so it answers a
`NonNegative Int | None` that a `when` or `?:` takes apart. The subject is
evaluated once. Salvo does not check that a literal qualifies while compiling:
what a qualifier's `qualifies` can decide at compile time is not something a
reader could keep track of, so the test runs when the program does.

### Field overrides

A predicate qualifier on a struct can give some of the struct's fields more
specific types, which apply wherever the qualifier does:

```salvo
qualifier Surname for Person {
    surname: Str // More specific type for the `surname` field when `Surname` applies

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

if person is Surname {
    // `person.surname` has type `Str` here because `person` has type `Surname Person`
}
```

These more specific types are not checked by the compiler. They are cast and
asserted where a field is read, so if `qualifies` does not actually guarantee
them, a read can fail at run time. Keep the override and the test in step.

### Explaining a failure: `@whynot`

A predicate qualifier can also say **why** a value fails, in a `@whynot` tag
in the doc comment of its `qualifies`
([documentation comments](Modules.md#documentation-comments)). The message is
plain language around `[name]`s, which name `qualifies`' parameters: the
value tested, and any slot of a scalar type (`Int`, `Long`, `Double`, `Bool`,
`Char`, `Str`). Nothing else can be named, so the message can only say what
the test had in hand.

```salvo
qualifier InRange(lo: Int, hi: Int) for Int {
    // @whynot [n] is not in [lo]..[hi]
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool { return n >= lo && n <= hi }
}
```

(`lo` and `hi` are *slots*, values the claim depends on; they are described
in [Dependent qualifiers](Dependent-Qualifiers.md). std's own version is
[`InRange`](../../lib/std/core/range.sv#qualifier-inrange).)

`whynot?(port is InRange(1, 65535))` asks for the message. It answers a `Str?`
that is `None` when `port` qualifies and "70000 is not in 1..65535" when it
does not.

* With several qualifiers in one test it answers for the first one that
  fails, in the order they are written.
* For a qualifier with no `@whynot` it says, for example,
  "70000 is not Small(10)".
* It is a compiler form, like `assert!`, written in place at each use. The `?`
  says it answers an optional, where `!` would mean it can stop the program.

`assert!(port is InRange(1, 65535))` with no message of its own fails with the
same text. `InRange.qualifies(port, 1, 65535)` is the test as a plain call.
To see which qualifiers a place carries at a given line, rather than whether
a value passes, `type?(port)` reports its type when compiling, and
`type!(port: InRange(1, 65535) Int)` checks it
([Control flow](Control-Flow.md#finding-out-a-type-type-and-type)).

## Constructive qualifiers

Predicate qualifiers apply to an existing value, and need not be present in
the generated Rust and Kotlin code. By contrast, a **constructive** qualifier
can correspond to a new type in the underlying Rust or Kotlin code (as
`MutableList<T>` is different from `List<T>` in Kotlin), or to a different way
of using one (as a mutable `Vec<T>` needs mutable references and variables in
Rust). To support such cases, the only way to obtain a constructive qualifier
is by calling a function that produces it. The qualifier itself has no body.

A **constructor function** is marked by writing `+Qualifier` in front of its
return type, the same establishment marker deductions use. Every return point
returns a plain value of the return type, which gains the qualifier *by
construction*, and callers see the qualified type:

```salvo
// No body: a constructive qualifier
qualifier RandomPositive for Int

// `-> +RandomPositive Int` marks this as a constructor for the qualifier.
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

Two rules keep constructors honest:

* Constructor functions must be defined in the same file as the qualifier
  declaration, so the claim's author is the only one who can vouch for it.
* A constructor must return a simple type, not a union or a tuple. Other
  functions can add more structure on top of the constructors.

The claim is trusted: nothing checks that `random_positive_int` really
returns a positive number.

### Constructors for predicate qualifiers

Constructor functions are not limited to constructive qualifiers: a predicate
qualifier may declare constructors too. The constructor asserts that its
predicate holds *by construction*, so callers get the qualified type without
a runtime `qualifies` check. The same rules apply: constructors live in the
same file as the qualifier and return a simple type.

```salvo
qualifier NonEmpty<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

// Requiring a first element guarantees the predicate by construction.
fn one_of<T>(first: T, ...rest: T[]) -> +NonEmpty List<T> => !first, !rest {
    return List(first, ...rest)
}
```

std's own [`NonEmpty`](../../lib/std/core/nonempty.sv#qualifier-nonempty) is
the generic one in `std.core.nonempty`, over anything with a `size`, and its
list constructors are shaped this way: `List()` for an empty list,
`List(first, ...rest)` for one that is known non-empty
([`List`](../../lib/std/core/list.sv#fn-list)).
[`Sorted`](../../lib/std/core/list.sv#qualifier-sorted) is std's too, and
[`Distinct`](../../lib/std/core/set.sv#qualifier-distinct) in `std.core.set`;
see [Collections](Collections.md).

A claim like std's `NonEmpty`, generic over its subject (`for T`), belongs to
no one type. So the file that declares a type may also establish it about
that type: `std.core.deque` writes `-> +NonEmpty Deque<T>` over a host
function.

### Tagging union arms

Constructors are also how union arms are tagged in practice: a generic
constructor applies the tag, and the tagged value then coerces into the
union.

```salvo
provenance qualifier Ok<T> for T
provenance qualifier Err<T> for T

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

These four declarations are
[`std.core.result`](../../lib/std/core/result.sv), which is implicitly
visible like the rest of `std.core`, so real code writes only the last
function. They are shown here because there is nothing privileged about
them: a domain-specific pair of tags is declared exactly the same way.
(`provenance` is explained in [State and provenance](#state-and-provenance).)

Constructive qualifiers can also be handled entirely by a backend's
implementation; see [Backends](Backends.md).

## Qualifiers in unions and tuples

A qualifier cannot apply to a tuple, only to the types that make it up. In an
un-parenthesized union type, a qualifier binds to the single arm it is
written on, never to the union as a whole. That makes qualifiers a natural
way to *tag* the arms of a union:

```salvo
// Because the qualifier cannot apply to the union as a whole, there is no
// ambiguity in the type annotation here
let p: Surname Person | None = check_surname(person)

// It is known statically whether `is` is being applied to a qualifier or not
if p is Surname {
    // Type of `p` is `Surname Person`
}
```

The arms of a union can overlap, in which case a test narrows to every arm
that matches, and an earlier test removes what it handled:

```salvo
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

Qualifiers can apply to the types of the parts of a tuple:

```salvo
// Type of `p` is `(Person, Surname Person | None)`
let p = (person, check_surname(person))
```

A qualifier *can* apply to an explicitly parenthesized union group, which is
how a qualifier nests over others: `Ok (Ok Str | Err Int)` is an `Ok` whose
payload is itself an `Ok Str` or an `Err Int`. Such types are legal but rarely
what you want.

## Arguments and spacing

Some qualifiers take arguments: constants (`InRange(0, 100) Int`), the
value a claim is about (`KeyOf(m) Str`, `proj(xs) T`), or the function a
claim was made with (`Heap<T>(?cmp) Mut List<T>`). The arguments are in
parentheses that touch the qualifier's name, with nothing between, as a
call's do; the type the qualifier applies to comes after a space. The
space is what tells the two apart:

```salvo
// `Ok` over the tuple type `(1, 2)`: a space, then the qualified type.
fn pair(p: Ok (1, 2)) -> None {
}

// `InRange` given `0` and `100`, qualifying `Int`.
fn ranged(n: InRange(0, 100) Int) -> None {
}
```

So a qualifier written against a type is an error naming the space:
`Mut(Str)` reads as `Mut` given an argument, and `proj(xs)T` lacks the
space before `T`. A space before a qualifier's arguments is an error too
where the parentheses can only be arguments, as in `proj (xs) T` or
`InRange (0, 100) Int`. The same rule holds for every name with an argument
list, a function call or declaration among them, so a Rust or Kotlin reader
writes these the way they already would. A qualifier's type arguments
follow it too: `Heap<T>(?cmp)`, never `Heap <T>`.

The arguments are counted wherever the type is written, as an `is` test
counts them: a qualifier with value slots gets one argument per slot
(`InRange(1) Int` and a bare `InRange Int` are errors naming the filled
form, `InRange(lo, hi)`), a qualifier with none takes no parentheses
(`Mut(1) Str` is an error), and type arguments are written all or none.
A function slot may be left out, since it is found by its name where it
is needed, but not given twice or past the slots there are.

## State and provenance

The sections above answer *how* a value gained a claim: by predication or by
construction. A second, independent question is what the claim is *about*,
and that one changes the semantics:

* A **state** qualifier is a claim about the value's **contents**:
  `NonEmpty`, `Sorted`, `Positive`. This is the default, and it is what
  `qualifier Q for T` declares.
* A **provenance** qualifier is a claim about where the **handle** came from:
  `Authenticated`, `Validated`, an id minted for one particular struct.
  Declare it by prefixing `provenance`.

```salvo
qualifier NonEmpty<T> for List<T>              // about the contents
provenance qualifier Authenticated for Request // about the origin
```

The difference matters because a function that mutates a value invalidates
claims about its contents. A call that takes `Mut` and does not promise to
keep `NonEmpty` strips it, since adding or removing elements could make it
false. Mutating a request's body does not change the fact that the request
was authenticated, so provenance qualifiers survive every call:

```salvo
fn touch(r: Mut Request) [] -> None => r: Mut {
    r.body = ""
}

fn f(r: Mut Authenticated NonEmpty Request) -> None {
    touch(r)
    // `NonEmpty` is gone here -- `touch` may have invalidated it
    // `Authenticated` is still known -- mutation cannot un-authenticate
}
```

Further rules follow from provenance being about the handle rather than the
data:

* **It is mint-only.** No inspection of the bits can tell you where a value
  came from, so a provenance qualifier has no body: no `qualifies` predicate,
  and no field overrides. Values gain it from constructor functions, exactly
  like a constructive qualifier, and `x is Authenticated` on a value that is
  not a union is a compile error.
* **It composes freely**, like every qualifier. An origin is orthogonal to the
  contents and to other origins, so several provenance tags can stack on one
  value: `Authenticated FromCache Request`.
* **It is droppable, and it survives storage.** Forgetting where a value came
  from is always safe, so an `Authenticated Request` can be passed wherever a
  plain `Request` is wanted; and a struct field typed `Authenticated Request`
  keeps the tag for whoever reads it back.

Provenance covers more than authority. Its meaning, "established by what the
value passed through, not by what is in it", fits two families, and std uses
both:

* **Authority**: `Authenticated Request`, an environment id. The value came
  through a checkpoint, and holding the tag *is* the proof.
* **Protocol role**: the tags `Ok`, `Err`, `Thrown` and `Emitted` are
  provenance qualifiers. `ok(x)` is where an `Ok` comes from, and nothing in
  an `Int`'s bits could ever say which arm it is. That classification is what
  you would want anyway: an `Ok Mut List<T>` stays `Ok` through an `add`
  (mutating the payload cannot change which arm it came in), and a tag stacks
  with any content claim: `Ok NonEmpty List<T>`.

What separates a provenance claim from a mint-only state claim is
content-dependence, not the lack of a `qualifies`. `Sorted` is also mint-only,
but it is a claim about *contents*, so mutation strips it; a tag is a claim
about *origin*, so it survives.

Both kinds are erased in the generated code: the kind only decides what the
compiler knows. If you want a distinct type at run time (its own identity, its
own equality, usable as a distinct map key), use a one-field struct instead.
A `Str` wrapped in a provenance qualifier stays a string, which is usually
what you want for ids.

`Mut`, `Linear`, `once`, `proj` and `ref` are also claims about a handle
rather than its contents, but they are compiler intrinsics rather than
qualifiers you can declare: each one changes how code is generated, or how
the ownership analysis treats a value. The rule of thumb is that a
*permission* can be forgotten (`Mut Person` is usable as `Person`) while an
*obligation* cannot (`Linear` and `once` never drop). See
[Linear types](Linear-Types.md), [Deductions and ownership](Deductions-and-Ownership.md)
and [Mutable handles](Mutable-Handles.md).

## One qualifier implying another

A qualifier's `for` type may carry qualifiers of its own, and then the
qualifier **implies** them: a `Positive` is a `NonZero Int` that is also
greater than zero.

```salvo
qualifier NonZero for Int {
    fn qualifies(x: Int) -> Bool { return x != 0 }
}

qualifier Positive for NonZero Int {
    fn qualifies(x: NonZero Int) -> Bool { return x > 0 }
}

fn inverse(x: NonZero Int) -> Int => x { return 100 / x }
fn half(x: Positive Int) -> Int => x { return inverse(x) }   // a Positive is NonZero
```

- **Writing the short form is enough.** `Positive Int` means `Positive
  NonZero Int`, transitively; hover shows the whole set.
- **The test proves the implied qualifiers.** `x is Positive` tests
  `NonZero` first, then calls `Positive`'s `qualifies` with a value that is
  already `NonZero`, so a `true` means both hold. Whatever establishes the
  claim another way (a constructor `as Positive`, a `+Positive` clause, a
  refinement) establishes what it implies, trusted as it always is.
- **Dropping an implied qualifier drops what implies it.** A call that
  takes `NonZero` away (`=> x: -NonZero`) leaves no `Positive` behind.
- **A slot may appear in the `for` type**: `qualifier Small(n: Int) for
  AtMost(n) Int` makes a `Small(10)` an `AtMost(10)`.
- Implication runs one way: a cycle is an error. It also stays within a
  family: a state qualifier cannot imply a provenance one, nor the reverse.
  A provenance qualifier implies another as its author says, with nothing
  to test: `Ordered` implies `ShardKeyOrdered`
  ([Effects and handlers](Effects-and-Handlers.md#claims-ordered-e-and-for-ordered-e)).

## Dependent qualifiers

A claim can be about a value's relationship **to another value**: that this
`Int` indexes *that* list, that this key is present in *that* map. That is
what lets an operation lose its "maybe":

```salvo
if k is KeyOf(m) {
    let v = get(m, k)       // the total read: an Int, not an Int?
}
```

They have a page of their own:
[Dependent qualifiers](Dependent-Qualifiers.md).

## Refinements

A function's deduction clause says what a call does to its arguments, and it
is written by the function's author, so it can only state what that author
knows. `add` mutates its list, so it may not promise a caller's `NonEmpty`
back (see [Deductions and ownership](Deductions-and-Ownership.md)), even
though appending to a list can never empty it. The function is not the party
that can fix this: it has never heard of `NonEmpty`.

The party that can is the qualifier. A **refinement** is a statement about a
function you do not own, written by the qualifier whose claim it is about:

```salvo
qualifier NonEmpty<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}
```

At a call site the refinement has the last word. `add`'s own `=> list: Mut`
drops everything it does not name, and then `+NonEmpty` puts the claim back:

```salvo
let xs: Mut List<Int> = Mut List()
add(xs, 1)
// `xs` is `Mut NonEmpty List<Int>` here, so this resolves:
let n = count(xs)
```

A refinement is **trusted**, exactly as `-> +Q T` is: nothing proves that
`add` establishes `NonEmpty`, and no runtime check is emitted. The qualifier's
author owns the claim's meaning, which is why they are the right party to
ask.

### What a refinement can say

`refn` is deliberately narrower than `fn`:

* It has no body, it cannot declare effects, and it cannot declare a return
  type. A refinement never changes what a function *does*, only what is
  *known* about the arguments afterwards.
* Its deduction entries can only add (`+Q`) and remove (`-Q`) state
  qualifiers. A plain name, which would mean "only this survives", is the
  function's own deduction to make.
* A qualifier may only speak about its *own* claim: `NonEmpty` cannot state
  what a call does to `Sorted`.
* Establishing and invalidating the same qualifier (`+Q -Q`) for one
  parameter is an error at the declaration.

### Which calls a refinement applies to

The parameter list is a **gate**. A refinement refines every in-scope
overload of the name it names whose parameters line up positionally: each
base type (qualifiers, including `Mut`, ignored) must be a super- or subtype
of the other, and a trailing `*` matches the rest. The gate is loose on
purpose. The strict check is at the call site, where a refinement fires only
when every argument is a subtype of the corresponding refinement parameter.

**All matching refinements apply.** At a call, every refinement whose
parameters the arguments fit applies, and their effects compose:

```salvo
qualifier Q1<T> for List<T> { ... refn something(list: List<T>, *) => list: +Q1 }
qualifier Q2<T> for List<T> { ... refn something(list: List<T>, *) => list: +Q2 }
```

Both `Q1` and `Q2` are established, since qualifiers compose freely.

A `@module` selector on the refinement itself (`refn add@core.list(...)`)
scopes *which* functions it matches, when a same-named function in another
module should be left alone.

### Conditional refinements: keeping versus establishing

**A refinement may ask for more than the function does**, and then what it
says is *conditional*:

```salvo
// Swapping two elements of an *already* non-empty list leaves it non-empty.
refn swap(list: NonEmpty Mut List<T>, i: Int, j: Int) => list: +NonEmpty
```

The extra qualifier on the parameter is a precondition: the statement applies
only where the argument already carries it. That is the difference between a
claim a call **establishes** and one it only **keeps**, and the distinction
matters in both directions. Written without the `NonEmpty`, the refinement
above would say that swapping *makes* a list non-empty, which is false for an
empty one. And because a kept claim was there before the call either way, it
survives a call that only *might* happen:

```salvo
if smallest != i {
    list.swap(smallest, i)      // `list` is still NonEmpty after the `if`
}
```

An established claim does not: a call that may not have run cannot have made
a list non-empty.

### Where refinements live

**Refinements travel with their qualifier.** A refinement declared inside
`NonEmpty` applies wherever `NonEmpty` is in scope, and nowhere else: you opt
into the refinements by opting into the qualifier.

**A refinement lives in the qualifier whose claim it is about, or in the type
it refines.** There is no top-level form. A qualifier is entitled to say what
happens to its claim; and for a claim generic over its subject, like std's
`NonEmpty`, the type's own author is the one who knows what its functions do.
So a struct's body, a named union's, or an opaque type's `{ … }` body, may
hold refinements about that type:

```salvo
export iterable platform type List<T canbe linear> canbe Mut : Iter<self, ListIter<T>, T> {
    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}
```

Such a refinement speaks only about its own type's values, and only about a
claim its author is trusted with: a generic one, or one declared in the same
file. It applies wherever that claim is in scope. This is where std's real
`NonEmpty` refinements are: in the declarations of
[`List`](../../lib/std/core/list.sv#type-list), `Set`, `Map`, `Deque` and
`Str`.

A *constructive* qualifier can hold refinements too, without a `qualifies`:
`Sorted` cannot be tested at run time and still has something to say about an
insert that keeps it.

### Refinements and your own deductions

A refinement also counts when the compiler checks a function's own deduction
clause against its body, so the fact does not die at one frame:

```salvo
// `add` strips `NonEmpty` and the refinement puts it back, so `refill`
// may promise it to its own callers.
fn refill<T>(list: Mut NonEmpty List<T>, value: T) -> None => list: NonEmpty Mut, !value {
    add(list, value)
}
```

The promise has to be written. Without a clause, a `Mut` parameter's default
is `=> list: Mut`, which keeps nothing else (see
[Deductions and ownership](Deductions-and-Ownership.md)), so `refill`'s
callers would lose the claim. A written entry may even report a claim the
parameter never declared, when the body's refined calls establish it; the
Deductions page shows this on std's heap.

One limit is worth knowing: a refined call counts toward what a function
promises only when it is unconditional in the body. A call inside an `if` or
a loop may not run at all, so it cannot establish a claim the signature then
promises. (At the call site itself, the refinement still applies inside the
branch.)

Finally, a refinement's documentation is merged into the refined function's,
so the language server shows what `add` establishes *here* even though the
statement lives elsewhere.
