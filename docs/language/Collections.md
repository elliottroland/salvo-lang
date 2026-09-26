# Collections

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

`auto Hashed<self>` generates the `hash` and `eq` that make it a `Set` element and a `Map` key; `auto Ordered<self>` generates the `cmp` that gives it `<`, `<=`, `>` and `>=` and makes it a key of the sorted collections. Both are checked where they are written: the struct may not be `canbe Mut` (a value that changed while a collection held it would corrupt that collection), and every field has to qualify too. A `List` or a tuple qualifies exactly when its elements do, comparing lexicographically. See [Comparison, equality and hashing](Comparison-and-Hashing.md) for the whole story, including hand-written implementations.

**Equality is opt-in, and compares field by field when generated:**

```
let a = Point { x: 1, y: 2 }
let b = Point { x: 1, y: 2 }
let same = a == b        // true
```

Both sides must be the same type — comparing two different struct types is an error rather than a quiet `false` — and qualifiers are ignored, because equality is about the data at the moment of the check, not about what is claimed of the handle. A struct holding a *function* cannot have the generated `eq` (no two backends agree on what equal functions are), which is exactly the case for writing one by hand that ignores the field. Floating-point values compare with semantics Salvo defines itself so that both backends agree (`NaN` equals nothing, including itself).

### Claims a list can carry

std ships three qualifiers over `List<T>`, which is where the qualifier machinery earns its keep over a container: a claim travels in the type, so a function can *demand* it instead of re-checking it.

`NonEmpty` is the one with a predicate, so it can be tested with `is` — and it is what lets `first` drop its optional:

```
let names = list_of("ada", "grace")
let head = first(names)          // a `Str`, not a `Str?`

let xs: Mut List<Int> = mut_list_of()
add(xs, 7)
let seven = first(xs)            // `add` established the claim
```

That second case is a **refinement**: `add` cannot promise `NonEmpty` back (a function that mutates may not promise a qualifier it has never heard of — see "Deductions"), so the qualifier says it on `add`'s behalf. One consequence to know about: if your own qualifier also refines `add`, the two disagree, and a call has to say which statement it means (`add@core.list` or `add@yours`) — or you declare `with NonEmpty` on yours, and both claims survive together.

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

## Arrays

An array (`T[]`) is a fixed-size sequence, and it exists mostly for the *variadic* boundary: `...elems: T[]` is what a variadic parameter receives. Arrays have no literal syntax — `[1, 2]` builds a list — so they are constructed by function:

```
// From its elements
let numbers: Int[] = array_of(1, 2, 3)

// Generate one, size given first
let zeros: Int[] = array_by(5, i -> 0)
```

The array's size can be fetched from `numbers.size()` (see [dot-notation](Functions.md) for calling a function that way) and the array can be 0-indexed using `numbers[index]`. Indexing is an array's alone: a list exposes element access as `get(xs, i)`.

## Any and Never

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
