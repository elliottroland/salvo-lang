# Qualifiers

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

Note that on a qualifier declaration `with` is only ever this compatibility clause between two qualifiers. Declaring that a type or a type parameter _may carry_ a qualifier is a different thing, and uses `canbe` (see auto-qualifiers below, and `canbe linear` in [Linear types](Linear-Types.md)). The same word appears in one other place, where it cannot be confused with this one: after a `use` or `spawn`, `with` names the dependency instances to supply (see "Inheriting the scope, and overriding it with `with`").

## Auto-qualifiers and `Mut`

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

Applying `Mut` to a type whose declaration does not say `canbe Mut` is a compile-time error. How a backend maps a `Mut` type is described in [Backends](Backends.md).

`Mut` is also the one qualifier whose *removal* can cost something. Dropping a qualifier is ordinarily free — it only forgets a claim — and a `Mut List<T>` used as a `List<T>` really is the same value. But a backend may render `Mut T` as a *different type* than `T` (Kotlin's `Mut Str` is a `StringBuilder`, which is not a `String`), and there the drop is a conversion. Salvo hides that: the compiler records where a `Mut` is dropped and the backend supplies whatever conversion it needs, at every such place — arguments, returns, annotations, struct fields, union arms, interpolation and operators. Never in the source changes, and a `Mut Str` behaves like the `Str` it is being used as.

## How a qualifier is established

A qualifier arrives on a value in one of two ways: **by predication** — a
test proves the claim holds — or **by construction**, where a function says
it establishes the claim. The two sections below take them in turn; both
declare their machinery inside the qualifier itself, so a claim and the way
to obtain it live together.

## Predicate qualifiers

A predicate qualifier is one whose claim can be *tested*. In this case, we define the function `qualifies` inside the qualifier, which takes a parameter of the given type and returns a boolean. This function does not support deductions because it can only ever be additive to the qualifiers of the type and can never move the value. It can, however, require effects, in which case the effects must have handlers in the context like any other function call:

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

A claim is tested with the same `is` syntax that narrows a union ([Control flow](Control-Flow.md)). When applied to a non-union type of the relevant kind (in this case `Int`), then it calls the `qualifies` function and allows casting:

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

## Constructive qualifiers

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

## State and provenance

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

* **It is droppable, and it survives storage.** Forgetting where a value came from is always safe, so `Authenticated Request` can be passed wherever a plain `Request` is wanted; and a struct field typed `Authenticated Request` keeps the tag for whoever reads it back.

Both kinds are erased in the generated code — the subject only decides what the compiler knows. If you want a distinct type at runtime (its own identity, its own equality, usable as a distinct map key), use a one-field struct instead; a `Str` wrapped in a provenance qualifier stays a string, which is usually what you want for ids.

`Mut`, `Linear`, `once` and `proj` are also claims about a handle rather than its contents, but they are compiler intrinsics rather than qualifiers you can declare: each one changes how code is generated, or how the ownership analysis treats a value. The rule of thumb is that a permission can be forgotten (`Mut Person` is usable as `Person`) while an obligation cannot (`Linear` and `once` never drop).

## Dependent qualifiers

A claim can be about a value's relationship **to another value** — that this
`Int` indexes *that* list, that this key is present in *that* map — which is
what lets an operation lose its "maybe":

```
if k is KeyOf(m) {
    let v = get(m, k)       // the total read: an Int, not an Int?
}
```

They have a page of their own:
[Dependent qualifiers](Dependent-Qualifiers.md).

## Refinements

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

**When refinements disagree, the call is refused.** Two qualifiers can both claim a call establishes them, and their claims can be mutually exclusive:

```
qualifier Q1<T> of List<T> { ... refn something(list: List<T>) => list: +Q1 }
qualifier Q2<T> of List<T> { ... refn something(list: List<T>) => list: +Q2 }
```

Merging these would ask for `+Q1 +Q2`, which is impossible when neither declares `with` the other. Rather than pick one — and leave you to work out afterwards which claim your value ended up with — the compiler refuses the call and names the place that made each statement:

```
something@lib(xs)    // `lib`'s statement: establishes Q1
something@main(xs)   // this module's: establishes Q2
```

The place you name is the only one that applies, so what the call establishes is what you asked for. Two refinements made by the *same* place are an error where they are written instead, since no selector could separate them.

The other remedies: test the property yourself with `is` after the call (always possible, since these are state qualifiers), or declare `with` on your qualifier so both claims can co-apply and the disagreement disappears.

**A refinement always lives in the qualifier whose claim it is about.** There is no top-level form — a qualifier is the one party entitled to say what happens to its claim, and having one refinement in two possible places bought nothing once ambiguity became a question a call answers. A *constructive* qualifier can hold refinements too, without a `qualifies`: `Sorted` cannot be tested at runtime and still has something to say about an insert that keeps it.

Finally, a refinement reaches **inferred** deductions, so the fact does not die at one frame:

```
// No written list: `NonEmpty` survives the call to `add` because of the
// refinement, so `refill` promises it back to its own callers.
fn refill<T>(list: Mut NonEmpty List<T>, value: T) -> None {
    add(list, value)
}
```

Two limits are worth knowing. A refinement contributes to an inferred deduction only for a qualifier the parameter itself declares — it can put back what a call dropped, never invent a claim the signature never made — and only when the refined call is unconditional in the body, since a call inside an `if` or a loop may not run at all. A refinement's documentation is merged into the refined function's, so the language server shows what `add` establishes *here* even though the statement lives elsewhere.
