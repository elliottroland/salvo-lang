// Collections: the literals, the two orderings, keys, and equality.
//
// The program is chosen to show the parts that are *decisions* rather than
// mechanics — the things you would otherwise have to discover:
//
//   1. three literal forms, and what a brace means where it is ambiguous
//   2. insertion order for `Set`/`Map`, key order for the sorted pair
//   3. what may be a key, and how a struct joins in
//   4. equality and ordering as capabilities, opted into per type
//   5. generated constructors and converters
//   6. claims about a list: `NonEmpty`, `Sorted`, `Distinct`
//   7. iterating a set and a map
//
// Everything here prints identically on both backends, which is the point:
// the orderings are the language's, not the target's.

// [cmp-default] Comparison, equality and hashing are **capabilities**: having
// one is having the function. `default` asks the compiler for the structural
// implementation — `cmp@Point`, `hash@Point` and `eq@Point`, generated from the
// fields in declaration order — which is what makes a `Point` orderable with
// `<`, usable as a key, and comparable with `==`. Both clauses are checked
// here, at the declaration: a `canbe Mut` struct or a float field is refused.
struct Point : default Ordered<self>, default Hashed<self> {
    x: Int,
    y: Int
}

// Equality alone: comparable with `==`, and not a key (no `hash`).
struct Note : default Eq<self> {
    text: Str
}

// [col-distinct] A position that *demands* the claim: only a list that came
// from a set (or was otherwise minted) can be passed here, so the body needs
// no duplicate check.
fn count_unique(xs: Distinct List<Int>) [] -> Int => xs {
    return size(xs)
}

fn main() [use] -> None {
    use StdOutConsole()

    // ===== 1. the literals =====
    //
    // Brackets build a *list*; braces build a set, or a map when the entries
    // have values. `Mut` on the position asks for a mutable one, and an empty
    // literal takes its type from where it is going.
    let primes = [2, 3, 5, 7]
    let vowels = {"a", "e", "i", "o", "u"}
    let ages = {"ada": 36, "grace": 45}
    println("1. list ${to_str(primes)}")
    println("1. set ${to_str(vowels)} of ${size(vowels)}")
    println("1. map ${to_str(ages)}")

    // A brace whose first entry is `name:` is a *struct* literal — which is
    // why a map key is an expression, not a bare name.
    let note: Note = {text: "still a struct literal"}
    println("1. struct ${note.text}")

    let seen: Mut Set<Str> = {}
    add(seen, "first")
    println("1. empty then filled ${to_str(seen)}")

    // ===== 2. the two orderings =====
    //
    // A `Set`/`Map` iterates in *insertion* order, on both backends — Rust's
    // own hash containers have no order at all, so the backend ships one.
    // Overwriting a key keeps its position; removing one leaves the rest.
    let tally: Mut Map<Str, Int> = mut_map_of(("pear", 1), ("apple", 2))
    put(tally, "fig", 3)
    put(tally, "pear", 99)
    println("2. insertion order kept ${to_str(tally)}")

    // The sorted pair is ordered by its keys instead. Separate types, not a
    // qualifier: sortedness changes behaviour, so it must not be droppable.
    let ranked: Mut SortedSet<Str> = mut_sorted_set_of("pear", "apple", "fig")
    println("2. key order ${to_str(ranked)}")
    let smallest = min(ranked)
    if smallest is Str {
        println("2. min is cheap here ${smallest}")
    }

    // ===== 3. keys =====
    //
    // A struct with a `hash` and an `eq` is a set element and a map key by
    // value: two
    // equal points are the same key.
    let corners: Mut Set<Point> = {}
    add(corners, Point { x: 0, y: 0 })
    let again = add(corners, Point { x: 0, y: 0 })
    println("3. struct key: size ${size(corners)}, second add ${again}")

    let labels: Mut Map<Point, Str> = {}
    put(labels, Point { x: 1, y: 1 }, "diagonal")
    let found = get(labels, Point { x: 1, y: 1 })
    if found is Str {
        println("3. looked up by value ${found}")
    }

    // ===== 4. equality and ordering =====
    let a = Point { x: 1, y: 2 }
    let b = Point { x: 1, y: 2 }
    let c = Point { x: 1, y: 9 }
    let same = a == b
    let before = a < c
    println("4. equal ${same}, ordered ${before}")

    // `default Eq<self>` alone: `==` without an order and without a hash.
    let n1 = Note { text: "same" }
    let n2 = Note { text: "same" }
    let notes_equal = n1 == n2
    println("4. plain struct equality ${notes_equal}")

    // ===== 5. generating and converting =====
    let squares = list_by(4, i -> i * i)
    println("5. generated ${to_str(squares)}")
    let deduped = to_set(primes)
    println("5. to_set ${to_str(deduped)}")
    let words = ["alpha", "be"]
    let lengths = to_map(words, w -> (w, size(w)))
    println("5. to_map with a rule ${to_str(lengths)}")

    // ===== 6. claims about a list =====
    //
    // The qualifier machinery pays off over a container: a *claim* about a
    // list travels in its type, so a function can demand it instead of
    // re-checking. `NonEmpty` is the one with a predicate, so it can be
    // tested; `Sorted` and `Distinct` are established by construction only,
    // because deciding them means comparing elements.
    //
    // `NonEmpty` earns its keep on `first`, which drops the optional:
    let filled = non_empty_list("ada", "grace")
    println("6. first is ${first(filled)}, no optional")

    // …and `add` establishes the claim, which `add` itself cannot promise —
    // the qualifier says so on its behalf (a *refinement*).
    let growing: Mut List<Int> = mut_list_of()
    add(growing, 7)
    println("6. after add, first is ${first(growing)}")

    // `sort` mints `Sorted`, which is what makes `binary_search` honest: over
    // an unordered list the answer would be meaningless, not merely absent.
    let ordered = sort(list_of(40, 10, 30, 20))
    println("6. sorted ${to_str(ordered)}")
    if binary_search(ordered, 30) is Int at {
        println("6. found 30 at ${at}")
    }

    // `add_sorted` is the insert that *keeps* the claim: it places the element
    // where the order survives, so the list is still a `Sorted List` after it.
    let live: Mut Sorted List<Int> = mut_sort(list_of(10, 30))
    add_sorted(live, 20)
    add_sorted(live, 5)
    println("6. still sorted ${to_str(live)}")

    // `Distinct` comes from a set: it is the one thing a set can honestly
    // promise about the list it converts to.
    let unique = to_list(deduped)
    println("6. distinct ${to_str(unique)} of ${count_unique(unique)}")

    // ===== 7. iterating =====
    //
    // A set yields its elements; a *map* yields its keys, and a value is
    // reached with `get` — which borrows it rather than copying.
    for v in iter(vowels) {
        print(v)
    }
    println("")
    for name in iter(ages) {
        let age = get(ages, name)
        if age is Int {
            println("7. ${name} is ${age}")
        }
    }
}
