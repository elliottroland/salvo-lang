# Deductions and ownership

## Deductions

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
| `=>[keep] !t` | a group: these entries are about the fn-typed parameter `keep` (its own parameter `t` named in its type, `keep: (t: T) -> Bool`) |

One projection statement is *not* a clause entry: a result that is owned but **holds** borrows somewhere inside writes them on the return type — `-> proj(list) in (T)`, the parentheses mandatory (see "Projections").

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

## Why returning a parameter is a move

A parameter that is kept compiles to a *borrow* in Rust: the caller retains its value. A function's return value, by contrast, is *owned* by the caller unless the signature says otherwise. If a function returns one of its parameters as an owned value, these two facts collide, and Salvo resolves it without a hidden clone: returning a parameter transfers ownership out through the return channel, and the parameter is deduced as _moved_ — the caller that passed it in loses it. The same applies to the other escape routes — storing a parameter in a struct, array, or tuple literal, or passing it to a consuming call. Consequently, a written clause cannot promise a parameter back when the body returns it owned: `=> x` with `return x` is a compile-time error.

The way to give a caller access to a parameter's data *without* moving it is to return a **projection** of it — `-> proj(x) T` — which is a borrow, described next. This is purely a constraint of the Rust backend — the Kotlin backend ignores deductions, since everything is a garbage-collected reference on the JVM — but one Salvo codebase must compile to both, so the checker enforces the stricter contract everywhere.

Binding a parameter with `let` is *not* on the move list: it creates a *shared fate* link instead (see "Shared fate and `copy`").

## Projections: borrowing without copying

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

Such a struct is an ordinary owned object: its `Mut` is real (a pass is advanced in place), its non-`proj` fields are its own, and it may be moved, stored, or passed on. What it may not do is outlive what it borrows. The compiler tracks this as shared fate too: `let p = iter(xs)` links `p` to `xs`, so `xs` cannot be moved or mutated while `p` is alive — and nothing had to be written on `iter`, because **which parameters a result holds borrows of is inferred from the body**: the literal stores `list` in a `proj` field, so `iter` lends `list`. Where there is no body — an effect member, an intrinsic, a fn-typed parameter — the signature says it: `-> proj(list) in (T)` on the return type ("the result holds a borrow of `list` somewhere inside `T`" — the parentheses are mandatory, and the same form sits on a fn type's own return, `?iter: (c: C) -> proj(c) in (Mut It)`), or per field in the clause, `=> .items: proj(list)`. A written entry must name every lend the body performs; it may name more (a generic body lends through opacity the analysis cannot see). Returning a view rooted in a *local* is an error: the local dies with the call.

A parameter is a **constant binding**: `n = n + 1` inside a function is an error, whatever `n`'s type — bind a local (`let next = n + 1`) instead. To change what the caller holds, assign a *field* of the parameter (`h.tags = …`), which is what the clause below describes.

A deduction clause can also say *which field* a call mutates, and whether it **changes** that field or **replaces** it. `=> e: Mut` means "mutated somewhere in `e`", so everything derived from `e` is invalidated; `=> e.hp: Mut` says the mutation lands on that field alone, and a value derived from a different field — `let rings = e.rings` — survives the call. `=> !e.rings` is the other half: the field itself is **replaced**, so whatever was derived from it is gone. The difference is storage identity, and it decides what survives: after a call that only changes the *contents* of `e.rings`, a handle to that list is still a handle to that list (`let rings = e.rings` keeps working, and sees the change), while a value read *out* of it — an element — does not, because an element may be gone. A written field entry is checked against the body, since every caller's precision rests on it.

Where a function writes no clause, the field set is **inferred** from its body, so ordinary code gets this precision without saying anything — and the inference is conservative: a body that replaces a field, or hands the whole value to another mutator, narrows nothing. Writing `=> h: Mut` explicitly still means *anywhere*.

This is also what lets a **qualifier hold about a field**: `if h.tags is NonEmpty { … }` narrows that field, the claim is used inside the branch, it survives a call that only touches `h.n`, and it falls the moment `h.tags` itself is mutated.

A container can hold **mutable** elements too — and that mutability is the element type's, not the container handle's: container `Mut` permits reshaping (`add`, `remove_at`, `set`, `swap`), while `List<Mut T>` elements hand out **mutable handles**. `get(squad, i)!` over a `List<Mut Entity>` answers a `proj(squad) Mut Entity`, and mutating through it — `hero.hp = hero.hp - 3`, or passing it to a `Mut Entity` parameter — writes the element in place. Every such write counts as a mutation of the container: values derived from it are invalidated, exactly as a mutating call would invalidate them, while the acting handle itself stays live. A handle that is only ever read imposes nothing, so any number can coexist; a projection whose element type has no `Mut` stays read-only, and `copy` remains the way to a value of your own.

A function of your own can hand back a mutable handle too: `fn front(es: List<Mut Entity>) -> (proj(es) Mut Entity)?` lends an element, and the handle it returns may be mutated, bound across statements, or found by a search loop (`for e in es { if e.hp < 10 { return e } }`). Where the algorithm should not know what a *position* is, `params Locate<C, L, T>` is the bundle to take: one `at` function the caller supplies, the way `Ordered` supplies `cmp` — which is how a generic function hands out mutable handles into a container it has never heard of.

For the common cases std wraps the proofs into an **update family**: `update(squad, i, hero -> { hero.hp = hero.hp - 3 })` writes one element in place through a callback, and `update2(squad, i, j, (a, d) -> { … })` is the two-element transaction — its `j` parameter declares `NotEq(i)`, so the proof is demanded where the call is made and the family's bodies are ordinary Salvo. Both preserve `Idx` claims: an in-place write moves no boundary, so a sequence of updates stays total end to end.

Two mutable handles at once take a **proof** — or a **declaration**. A function that means to accept two handles which might be the same object says so: `fn attack(a: Mut Entity, d: Mut Entity) -> None => a canbe d` declares that its two parameters may name one entity, and a caller may then hand it `get(squad, i)` and `get(squad, j)` with nothing proven about `i` and `j` — self-attack included, behaving the same on both backends. `canbe` is symmetric and non-transitive, `a canbe b|c` relates `a` to each of the two, and the anchored form (`track canbe in lib.tracks`) says the parameter may be an element of a named container, so two parameters anchored in the same one may coincide.

Where no such declaration exists, two handles at once take a proof. The analysis cannot tell `squad[i]` from `squad[j]` — `i` might equal `j` — so a write through one invalidates the other, and a call taking both (`attack(get(squad, i)!, get(squad, j)!)`) is refused. `core.list`'s `NotEq` qualifier is the proof: after `j is NotEq(i)`, the two indices are known to differ, the handles are known to name different elements — the write through one leaves the other standing, and the two-handle call is accepted. The claim is a fact about the indices' *current values*: reassigning either side takes it away, like any dependent claim.

A container can hold borrows too: `List<proj T>` is a list of projected elements, and it is what `filter` returns:

```
fn filter<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) -> proj(it) in (Mut List<proj T>) => it: Mut, keep
```

The result holds borrows of whatever the pass walks — nothing is copied — and it lives no longer than the source. For a list of your own, `filter_to(dest, it, keep)` copies each kept element into `dest`, and it says so twice: in its `_to` name, and in the `?copy` implicit it takes so that the copy is the element type's own.

**A view of a temporary.** A view's source must outlive it, so binding, returning or storing a view of a temporary is an error: `let p = iter(list_of(1, 2))` dies at the end of its statement (the diagnostic says to `let` the list first). *Using* one within the statement is fine — `map(iter(list_of(1, 2)), f)`, `for x in iter(list_of(1, 2))` — the temporary lives that long on both backends.

**A capturing lambda is a view.** A lambda's body can hand out projections rooted in a *capture* — `indices.map(i -> all.get(i)!)` returns elements of `all`, and no function type can say so (`proj(…)` names parameters; captures have no name). So the closure itself carries the fact: it holds a borrow of every non-Copy variable it reads from the enclosing scope, exactly as a struct holds its `proj` fields. Binding the lambda links it to those variables, a call result built from the lambda is linked through it, and moving or mutating a captured variable poisons the closure — the same discipline every view lives under. A lambda that captures nothing holds nothing (`map(p, w -> w)` binds freely), a Copy scalar capture is the value itself, and a capture the body *consumes* is owned by the closure rather than borrowed — that is the lambda that becomes `once`.

**Passes borrow.** A pass that walks data declares `: Yield<self, proj T>` and its `next` returns `Emitted (proj(p) T) | Finished` — the element is a borrow of the pass, which borrows the source — while a generator (a countdown, a random stream) declares `: Yield<self, T>` and emits owned values. The two must agree: an obligation at `proj T` with an owning `next`, or the reverse, is an error. Reading combinators accept both. An `iter fn`'s generated pass borrows its subject the same way (`__subject: proj Subject`), so nothing is copied at the mint; an `iter fn` that wants a snapshot writes `copy(...)` in a `state` initializer.

**Re-pointing.** A mutable view may be made to project something else: a function that does so says which field and from what, `=> v: Mut, v.items: proj(other)`, and the caller's variable at `v` becomes linked to `other` from the call on.

**What Rust makes of it.** A projected value is `&T` (or `Option<&T>`, `Union2<&T, Finished>`); a view struct carries a lifetime (`ListYield<'s, T>`), a lent parameter ties it (`iter<'a, T>(list: &'a Vec<T>) -> ListYield<'a, T>` when elision cannot); `List<proj T>` is `Vec<&T>`. Kotlin, where everything is a reference already, changes nothing — the rules are what keep the two backends printing the same thing.

## Shared fate and `copy`

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

## Copy semantics per backend

`copy` is an `intrinsic fn` (see the Backends chapter): its declaration gives the checker everything it needs — the argument is kept with all its qualifiers, the result is independent — and each backend lowers calls to it against the argument's *actual type*. Where no Salvo operation could mutate the value anyway, a copy is free: Kotlin emits the argument unchanged (duplicating a reference to immutable data is a copy), and Rust clones. Where mutation is possible, the copy is real on every backend: `Mut List<Int>` becomes `xs.toMutableList()` in Kotlin and `xs.clone()` in Rust; a `Mut` struct with immutable fields becomes `p.copy()` / `p.clone()`. Where a backend cannot yet produce a correct copy (for example, nested mutability like `Mut List<Mut Person>` on the JVM, where a shallow copy would share the inner values), the compiler reports an error rather than emit code that behaves differently across backends.
