# Collections

## Overview

The standard library has a small set of collections, all generic over their
element type and all part of `std.core`, so every file sees them without an
import:

| Type | What it is | Module |
|---|---|---|
| `List<T>` | an ordered sequence, read by index and grown at the end | [std.core.list](../../lib/std/core/list.sv) |
| `Set<T>` | distinct elements, in insertion order | [std.core.set](../../lib/std/core/set.sv) |
| `Map<K, V>` | values by key, in insertion order | [std.core.map](../../lib/std/core/map.sv) |
| `SortedSet<T>`, `SortedMap<K, V>` | a set and a map kept in order | [std.core.sorted](../../lib/std/core/sorted.sv) |
| `Deque<T>` | a queue with two ends | [std.core.deque](../../lib/std/core/deque.sv) |
| `IntBuffer`, `LongBuffer` | fixed-length runs of numbers | [std.core.buffer](../../lib/std/core/buffer.sv) |
| `T[]` | an array, the fixed-size sequence a variadic parameter receives | [std.core.array](../../lib/std/core/array.sv) |

`List`, `Set` and `Map` are the everyday ones. Each is built by a constructor
named after its type, is immutable unless it is asked for as `Mut`, and is
worked on by ordinary functions, much like Kotlin's `listOf` and
`mutableListOf` with the methods turned into functions:

```salvo
let primes = List(2, 3, 5, 7)                 // List<Int>, immutable
let squares: Mut List<Int> = Mut List()       // an empty mutable list
for p in primes {
    add(squares, p * p)
}
let ages = Map("ada": 36, "grace": 45)        // Map<Str, Int>
let age = get(ages, "ada")                    // an Int?: the key may be missing
let third = get(primes, 2)                    // an Int?: the index may be out of range
println("${squares} ${age ?: 0} ${third ?: 0}")   // [4, 9, 25, 49] 36 5
```

A few rules hold across all of them, and the sections below come back to each:

* **Reads answer rather than fail.** A read by index or by key answers an
  optional, and a write by index answers whether it wrote. Nothing throws an
  out-of-range error.
* **Taking out is a move.** Removing an element hands it to you; nothing is
  copied and no hole is left, which is what lets a collection hold values that
  must be used exactly once ([Linear types](Linear-Types.md)).
* **Sets and maps keep insertion order** on every backend, and their keys must
  be hashable; the sorted kinds keep their keys in order, and need them
  orderable.
* **Claims travel in the type.** A list can be known to be non-empty, sorted
  or free of duplicates, and a function can ask for that in its signature
  (see [Claims a list can carry](#claims-a-list-can-carry)).

The module pages document every function; this page is about how the
collections behave.

## Building collections

### Constructors

`List<T>`, `Set<T>` and `Map<K, V>` are each built by a constructor named
after the type:

```salvo
let xs = List(1, 2, 3)                   // List<Int>
let names = Set("ada", "grace")          // Set<Str>
let ages = Map("ada": 36, "grace": 45)   // Map<Str, Int>
```

A collection is built immutable. Write `Mut` in the constructor's name to ask
for a mutable one, exactly as `Mut` reads on a type:

```salvo
let seen: Mut Set<Str> = Mut Set<Str>()
add(seen, "ada")
```

Each of these constructors **is** a type-named function (`List`, `Set`, `Map`,
with `Mut List`, `Mut Set` and `Mut Map` for the mutable forms):
`List(1, 2, 3)` resolves directly to the `fn List<T>(...)` declaration in std,
which is the one and only constructor. There is no separate lowercase builder.

A map's entries are written as `key: value` *arguments*: the same `k: v` pair
sugar any `(K, V)[]` variadic accepts. A `key: value` with an *identifier* key
is fine here too, since there is no struct-literal ambiguity inside a call.

An **empty** collection is the constructor with no arguments. It has nothing to
infer its type from, so that type comes from the position it is in (the
annotation on a `let`, or the parameter it is passed to) or is written on the
constructor itself (`List<Int>()`, `Mut Set<Str>()`). Where nothing supplies
one, that is an error naming the remedies.

A custom hash and equality (or `cmp`, for the sorted containers) is passed by
name: `Mut Set("x", hash = my_hash, eq = my_eq)`.

### No collection literals

There are no bracket or brace *literals* for collections: `[1, 2]` and
`{1, 2}` are retired. A `{ name: value }` brace with an identifier key is a
**struct** literal, which is why a collection takes its entries through a
constructor call.

### Ownership of the elements

A constructor takes ownership of what it stores, a spread array included:
`List(0, ...xs)` spends `xs` as it spends `0`, so copy the array first if you
need it again. An array's elements also move onto an existing list with
`append(list, items)`, and `drain(items, each)` hands each one to a callback;
both spend the array.

### Generating and converting

Besides the constructors, each collection can be **generated** from a size and
a rule, or **converted** from another:

```salvo
let squares = list_by(4, (i) -> i * i)        // [0, 1, 4, 9]
let unique = to_set(xs)                       // duplicates collapse
let lengths = to_map(List("ada", "grace"), (n) -> (n, size(n)))   // {ada: 3, grace: 5}
```

There are two [`to_map`](../../lib/std/core/map.sv#fn-to_map) forms: one from a
list of pairs, and one from a list of anything plus a rule saying what key and
value each element becomes. Duplicate keys are last-wins everywhere.

## Lists

A [`List`](../../lib/std/core/list.sv#type-list) is read by index with `get`
and grown at the end with `add` (on a `Mut List`). `for x in xs` walks it in
order. The whole surface is on the
[std.core.list](../../lib/std/core/list.sv) page; what follows is how its
operations behave.

### Taking elements out

Taking something back **out** of a collection is a move, not a copy.
`remove_first(list)` and `remove_at(list, i)` answer `T?`: the element, or
`None` when there is nothing at that position. `remove(map, key)` does the same
for a map's value, and `replace(map, key, value)` is the write that hands back
what it displaced. None of them leaves a hole or duplicates anything, which is
what lets a collection hold values that must be used exactly once (see
[Linear types](Linear-Types.md)); for ordinary data they are simply the
operations you would expect. `drain(list, each)` consumes a collection and
hands every element to a function, in order.

The same holds for taking several at once. `remove_front(list, n)` and
`remove_back(list, n)` take `n` elements off either end, and
`remove_front_while(list, keep)` and `remove_back_while(list, keep)` take them
for as long as `keep` accepts them. Each answers the removed elements as a list
of their own, in order, and the list keeps the rest.

`insert_at(list, i, x)` puts `x` at `i`, shifting the rest up, and hands `x`
back when `i` is out of range. `replace(list, i, x)` writes `x` at an index
proven in range (`i is Idx(list)`, or an index from `indices(list)`; see
[Dependent qualifiers](Dependent-Qualifiers.md)) and answers the element it
displaced.

### Searching and reshaping

A list also has the searches you would expect. `find_first(list, pick)` and
`find_last(list, pick)` answer the index of the first or last element `pick`
accepts; `index_of`, `last_index_of` and `contains` compare by `==`; and
`any`, `all` and `count` ask a predicate about the elements. `partition(list,
pick)` splits a list in two, `sub_list(list, from, to)` copies a range, and
`reverse(list)` reverses a `Mut List` in place. All of these are ordinary
Salvo in std, written over a handful of primitives.

### Indexes answer rather than fail

A list has no positional *write*, on purpose: when the index misses, the value
written has nowhere to go. What it has instead is `swap(list, i, j)`, which
exchanges two elements: nothing enters, nothing leaves, so nothing can be
dropped.

Every operation that takes an index **answers rather than failing**, and
identically on both targets. A read is an optional. A *write* answers whether
it happened: `set` on a `Mut Str` or a `Mut Bytes` is a `Bool` that is `false`
when the index is out of range, in which case nothing was written, and which
may be ignored where the index is known good. A list's `swap` answers a
[`Checked<Bool>`](../../lib/std/core/checked.sv), an answer that must be looked
at (`detach`) or explicitly dropped (`ignore`); with both indexes proven in
range (`Idx(list)`) it answers nothing.

A negative index is out of range rather than counted from the back, on every
surface.

## Sets and maps

A [`Set`](../../lib/std/core/set.sv#type-set) holds each element once, and a
[`Map`](../../lib/std/core/map.sv#type-map) holds a value per key. Sets and
maps **iterate in insertion order**, on every backend. A `Map` iterates its
*keys*, and a value is reached with `get`:

```salvo
for name in iter(ages) {
    let age = get(ages, name)
    if age is Int {
        println("${name} is ${age}")
    }
}
```

### What can be a key

Not every type can be a key. A `Set` element or a `Map` key has to be
hashable. Among the built-in types that means `Byte`, `Int`, `Long`, `Char`,
`Bool` and `Str`; `Double` and `Float` are deliberately excluded, since
floating-point equality would not mean the same thing on both backends. A
`List` or a tuple qualifies when its elements do. A union cannot be a key for
now: it would need every arm to be hashable, which the compiler does not yet
check.

A struct joins in by *having the functions*:

```salvo
struct Point : Ordered<self> by auto, Hashed<self> by auto {
    x: Int,
    y: Int
}
```

[`Hashed<self> by auto`](../../lib/std/core/compare.sv#fns-hashed) writes the
`hash` and `eq` that make it a `Set` element and a `Map` key;
[`Ordered<self> by auto`](../../lib/std/core/compare.sv#fns-ordered) writes the
`cmp` that gives it `<`, `<=`, `>` and `>=` and makes it a key of the sorted
collections. Both are checked where they are written: a `canbe Mut` struct is
refused (a value that changed while a collection held it would corrupt that
collection), and every field has to have the capability too, reported at the
field. A `List` or a tuple compares lexicographically. See
[Comparison, equality and hashing](Comparison-and-Hashing.md) for the whole
story, including hand-written implementations, and
[Compile-time functions](Compile-Time-Functions.md) for what `by auto` writes.

### The identity a container is keyed by

The four keyed containers name the ordering or the hash they keep their keys
by: `SortedSet<T>(?cmp)`, `Set<T>(?hash, ?eq)`. A slot nobody writes is
resolved by its own name, the way an implicit parameter is, so `Set<Str>`
means the set keyed by `Str`'s own `hash` and `eq`. Naming a *different* one
keys the container by it: the identity travels in the type, and every
operation is handed the function it names (see
[Comparison, equality and hashing](Comparison-and-Hashing.md)).

### Equality

**Equality is opt-in, and compares field by field when generated:**

```salvo
let a = Point { x: 1, y: 2 }
let b = Point { x: 1, y: 2 }
let same = a == b        // true
```

`a == b` is the call `eq(a, b)`, so comparing two different struct types is an
error rather than a quiet `false`, unless an `eq` taking the pair is declared.
A qualified value compares through its plain type's `eq` unless one is
declared for the qualifier
([What an operator calls](Comparison-and-Hashing.md#what-an-operator-calls)).
A struct holding a *function* cannot have the generated `eq`, since no two
backends agree on what equal functions are; that is exactly the case for
writing one by hand that ignores the field. Floating-point values compare with
semantics Salvo defines itself so that both backends agree (`NaN` equals
nothing, including itself).

## Sorted sets and maps

For a collection kept in order rather than in insertion order, there are
[`SortedSet<T>`](../../lib/std/core/sorted.sv#type-sortedset) and
[`SortedMap<K, V>`](../../lib/std/core/sorted.sv#type-sortedmap):

```salvo
let names: Mut SortedSet<Str> = Mut SortedSet("pear", "apple")
add(names, "fig")
println("${to_str(names)}")        // {apple, fig, pear}
let smallest = min(names)          // and max, first_key, last_key on a map
```

They are separate types rather than a qualifier on `Set` or `Map`, because
sortedness changes how a collection behaves, and a qualifier could be dropped
on the way into a function that relied on it.

Their keys have to be **orderable** rather than hashable, which is a slightly
different bar: a union is never orderable, since comparing values of different
types has no obvious meaning. Strings order by code point, the same reading
Salvo takes everywhere else.

**What counts as one member** is named per container, and the two answers
differ. A `Set` and a `Map` are **`eq`-distinct**: they bucket by `hash` and
confirm the bucket hit by `eq`, which is why those two travel together. A
`SortedSet` and a `SortedMap` are **`cmp`-distinct**: two keys are one member
when `cmp(a, b) == 0`, and equality plays no part. That is not a shortcut: it
is what both target platforms do, and it is the only rule a sorted container
can implement. So a set sorted by age keeps one person per age, by definition;
if you want "same rank" and "same person" in one program, hold both
containers.

## Claims a list can carry

std ships three qualifiers a list can carry, and this is where the qualifier
machinery ([Qualifiers](Qualifiers.md)) earns its keep over a container: a
claim travels in the type, so a function can *demand* it instead of
re-checking it.

| Qualifier | Claim | How a list gets it |
|---|---|---|
| [`NonEmpty`](../../lib/std/core/nonempty.sv#qualifier-nonempty) | it has at least one element | a test (`is NonEmpty`), or an operation that establishes it |
| [`Sorted`](../../lib/std/core/list.sv#qualifier-sorted) | it is in order, by a named ordering | `sort`, `mut_sort`, and kept by `add_sorted` |
| [`Distinct`](../../lib/std/core/set.sv#qualifier-distinct) | it has no duplicates | `to_list` of a set |

### `NonEmpty`

`NonEmpty` is the one with a predicate, so it can be tested with `is`, and it
is what lets `first` drop its optional:

```salvo
let names = List("ada", "grace")
let head = first(names)          // a `Str`, not a `Str?`

let xs: Mut List<Int> = Mut List()
add(xs, 7)
let seven = first(xs)            // `add` established the claim
```

That second case is a **refinement**. `add` cannot promise `NonEmpty` back,
since a function that mutates may not promise a qualifier it has never heard
of (see [Deductions and ownership](Deductions-and-Ownership.md)), so `List`
says it on `add`'s behalf, in its own body (see
[Refinements](Qualifiers.md#refinements)).

`NonEmpty` is not only about lists. It is about anything with a `size`
(`List`, `Set`, `Map`, `SortedSet`, `SortedMap`, `Deque`, `Str`), because it is
**one generic qualifier**, in `std.core.nonempty`:

```salvo
export qualifier NonEmpty<T canbe linear> for T {
    fn qualifies(value: T, ?size: (value: T) -> Int) -> Bool {
        return size(value) > 0
    }
}
```

The `size` is an implicit parameter of `qualifies`, found where the claim is
tested at the value's own type, so nothing about it shows in
`NonEmpty Set<Str>`. What *establishes* the claim is each type's to say, in its
own body (appending makes a list non-empty, adding makes a set non-empty), so
the same name works the same way over every container:

```salvo
let seen: Mut Set<Str> = Mut Set()
add(seen, "a")                       // Set's `add` establishes NonEmpty
let ranked = SortedSet(30, 10)
if ranked is NonEmpty {
    let lo = min(ranked)             // an element, not an optional
}
```

### `Sorted`

`Sorted` is established by construction only. There is no `is Sorted`, because
deciding whether a list happens to be sorted means comparing its elements,
which nothing can do over an unconstrained `T` at the Salvo level:

```salvo
let ordered = sort(List(40, 10, 30))     // a Sorted<Int>(cmp) List<Int>
let at = binary_search(ordered, 30)         // honest only because it is Sorted

let live: Mut Sorted List<Int> = mut_sort(List(10, 30))
add_sorted(live, 20)                        // inserts in order, claim survives
```

`add_sorted` is the insert that *keeps* the claim: it places the element where
the order survives, and says so in its own deduction clause, which it may do,
unlike `add`, because it genuinely knows.

This `Sorted` is a different mechanism from the `SortedSet` and `SortedMap`
**types**: those are a representation, while this is an erased claim about an
otherwise ordinary list, which is why a `Sorted List` still reaches the whole
list surface.

The claim **names the ordering it was sorted by**, the way any structure that
holds an ordering does (see
[A structure that holds an ordering](Comparison-and-Hashing.md#a-structure-that-holds-an-ordering)).
Orderings are plural, so "sorted" alone would not say enough: an `add_sorted` under a different `cmp`
than the sort used would insert at a position that means nothing in the order
the list is actually in. `sort` publishes what it resolved, and the two
operations that read the claim capture it:

```salvo
fn by_len(a: Str, b: Str) -> Int { return cmp(size(a), size(b)) }

let words = sort(List("pear", "fig", "Apple"), cmp = by_len)   // Sorted<Str>(by_len)
let found = binary_search(words, "kiwi")                          // asks by length: 1
```

Two lists sorted differently are two types and refuse to mix. A signature that
accepts either writes the claim without an ordering (`xs: Sorted List<T>`): an
argument nobody wrote is *unconstrained*, so the value keeps carrying its own.
A `let` annotation reads the same way, which is why the `Mut Sorted List<Int>`
above still knows what sorted it.

Elements have to be orderable for a `Sorted List` claim, on the same terms a
`SortedSet` key does, so `Sorted List<Double>` is refused where it is written.

### `Distinct`

`Sorted` and `Distinct` are about a *list*. `Distinct` says a list holds no
duplicates, and it comes from the one thing that can honestly promise it, a
set:

```salvo
let unique = to_list(Set(3, 1, 3))    // a Distinct List<Int>
```

## Deques

A [`Deque<T>`](../../lib/std/core/deque.sv#type-deque) is a queue with two ends:
adding or removing at either end costs the same small amount however long it
is, where a `List`'s `remove_first` shifts every element after it. It is the
collection for a mailbox, a work queue or requests waiting to be answered.

```salvo
let jobs: Mut Deque<Str> = mut_deque_of("b")
add_last(jobs, "c")
add_first(jobs, "a")
let next = remove_first(jobs)      // "a", as a Str?
let back = remove_last(jobs)       // "c"
println("${jobs}")                 // [b]
```

`get(d, i)` and `remove_at(d, i)` count from the front and answer `None` out of
range, as a list's do. With `i is Idx(d)` proven, `get(d, i)` answers the
element itself and `replace(d, i, x)` writes in place. `for x in d` and
`reversed(d)` walk it front to back and back to front. `to_list` and
`to_deque` convert. Like a list it may hold linear values: a
`Mut Deque<Reply<Str>>` owes, and `drain` is how it ends.

A deque has no type-named constructor (it is built by `deque_of` and
`mut_deque_of`) and no `==`, hash or ordering: it is a queue to work through,
not a value to compare or use as a key. It also has no wire form yet, so a
message that would carry one to another node is refused.

## Buffers

[std.core.buffer](../../lib/std/core/buffer.sv) has two fixed-length runs of
numbers, [`IntBuffer`](../../lib/std/core/buffer.sv#type-intbuffer) and
[`LongBuffer`](../../lib/std/core/buffer.sv#type-longbuffer): the index tables
a hash structure probes, where a `List<Int>` would box every slot on the JVM.
A buffer is made full and never changes length. There is no `add` or `remove`,
and growing one means building a bigger one.

```salvo
let table = int_buffer(8, -1)      // eight slots, every one -1
let slot = 3
if slot is Idx(table) {
    let was = replace(table, slot, 42)   // -1: writes in place, answers what it held
    let now = get(table, slot)           // 42, an Int: the index is proven
}
clear(table, -1)                   // every slot back to -1
println("${size(table)} ${table}") // 8 [-1, -1, …]
```

`get(buf, i)` without a proof answers an `Int?`. Since the length is fixed,
every `Idx` claim on a buffer survives `replace` and `clear`. `for x in buf`
walks the slots, `copy` copies the array, and a buffer has no `==` and no wire
form: it stays on its node.

## Arrays

An array (`T[]`) is a fixed-size sequence, and it exists mostly for the
*variadic* boundary: `...elems: T[]` is what a variadic parameter receives
([Lambdas and variadics](Lambdas-and-Variadics.md)). Arrays have no literal
syntax, and `List(1, 2)` builds a list, not an array, so arrays are constructed
by function ([std.core.array](../../lib/std/core/array.sv)):

```salvo
// From its elements
let numbers: Int[] = array_of(1, 2, 3)

// Generate one, size given first
let zeros: Int[] = array_by(5, (i) -> 0)
```

The array's size can be fetched with `numbers.size()` (see
[dot calls](Functions.md#dot-calls) for calling a function that way), and the array
can be 0-indexed with `numbers[index]`. Indexing is an array's alone: a list
exposes element access as `get(xs, i)`.

## Any and Never

Like Kotlin, Salvo has an [`Any`](../../lib/std/core/basic.sv#type-any) type,
which is the supertype of all other types. There is also a
[`Never`](../../lib/std/core/basic.sv#type-never) type, which represents
unreachable code (similar to `!` in Rust or `never` in TypeScript). In the type
system it is treated as a subtype of every other type, so that the types of
branching code work out.

In the following example, the `return` "evaluates" to `Never` while the `Ok`
branch evaluates to type `Str`. Thus the type of `value` is compatible with
`Str` (technically, the type of the `when` expression is `Ok Str`, but the code
explicitly elides the `Ok` qualifier).

```salvo
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
**expressions**, not statements. Usually that makes no visible difference,
since an escape sits on a line of its own, as it does above. But it means one
can stand wherever a value is expected, which is what lets a guard be written
inline:

```salvo
let step: Int = if i < limit { i } else { break }
```

A value follows an escape only on the same line, so `return` before a newline
or a `}` is still the value-less form. `break` and `continue` leave a *loop*
rather than the function, so neither satisfies "this function always returns"
the way a `return` does.
