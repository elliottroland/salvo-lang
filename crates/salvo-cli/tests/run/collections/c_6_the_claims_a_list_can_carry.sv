// C-6 the claims a list can carry [col-nonempty] [col-sorted-list]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- list_claims

// A position that demands distinctness needs no duplicate check of its own.
fn count_unique(xs: Distinct List<Int>) [] -> Int => xs {
    return size(xs)
}

// [cmp-carry] An ordering of the program's own, which is nothing like the
// code-point order `cmp(Str, Str)` gives: what `sort` publishes into the
// `Sorted` claim is *this* one, so the search and the insert below use it too.
fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

fn main() [use] {
    use StdConsole()

    // NonEmpty by construction: the element constructor requires a first, so
    // `first` answers with an element, not `Int?` [col-of-nonempty].
    let built = List(10, 20)
    println("built ${first(built)}")

    // NonEmpty by refinement: `add` establishes the claim on the qualifier's
    // behalf, so `first` resolves to the same overload.
    let grown: Mut List<Int> = Mut List()
    add(grown, 7)
    println("grown ${first(grown)}")

    // NonEmpty by test.
    let maybe = List(1, 2)
    if maybe is NonEmpty {
        println("tested ${first(maybe)}")
    }

    // Sorted, minted by `sort`; equal elements answer the lowest index.
    let ordered = sort(List(5, 1, 4, 1, 3))
    println("sorted ${to_str(ordered)}")
    if binary_search(ordered, 1) is Int at {
        println("lowest 1 at ${at}")
    }
    if binary_search(ordered, 9) is None {
        println("9 absent")
    }

    // add_sorted keeps the claim across the mutation.
    let live: Mut Sorted List<Int> = mut_sort(List(10, 40))
    add_sorted(live, 20)
    add_sorted(live, 5)
    println("still sorted ${to_str(live)}")

    // Strings order by code point on both backends.
    let words = sort(List("pear", "Apple", "fig"))
    println("words ${to_str(words)}")

    // [cmp-carry] …unless the claim names another ordering, which is what the
    // slot is for: `by_len` sorts, and `binary_search` and `add_sorted` are
    // computed with it because the list's *type* carries it. Under the
    // code-point order none of these three lines would come out this way.
    let bylen = sort(List("pear", "fig", "Apple"), cmp = by_len)
    println("by length ${to_str(bylen)}")
    if binary_search(bylen, "kiwi") is Int at {
        println("a 4-letter word at ${at}")
    }
    let growing: Mut Sorted List<Str> = mut_sort(List("pear", "fig", "Apple"), cmp = by_len)
    add_sorted(growing, "durian")
    println("still by length ${to_str(growing)}")

    // Distinct, minted by a set.
    let unique = to_list(Set(3, 1, 3, 2))
    println("distinct ${to_str(unique)} of ${count_unique(unique)}")

    // [qual-overload] The same claim over four more subjects: `NonEmpty` is
    // one name whose meaning the subject decides.
    let seen: Mut Set<Str> = Mut Set()
    add(seen, "a")
    if seen is NonEmpty {
        println("set claim ${size(seen)}")
    }
    let tally: Mut Map<Str, Int> = Mut Map()
    put(tally, "k", 1)
    if tally is NonEmpty {
        println("map claim ${size(tally)}")
    }
    let ranked = SortedSet(30, 10, 20)
    if ranked is NonEmpty {
        let lo = min(ranked)
        let hi = max(ranked)
        println("sorted claim ${lo}..${hi}")
    }
    let bykey = SortedMap(("b", 2), ("a", 1))
    if bykey is NonEmpty {
        let fk = first_key(bykey)
        println("sorted map claim ${fk}")
    }
}
//= stdout
//= built 10
//= grown 7
//= tested 1
//= sorted [1, 1, 3, 4, 5]
//= lowest 1 at 0
//= 9 absent
//= still sorted [5, 10, 20, 40]
//= words [Apple, fig, pear]
//= by length [fig, pear, Apple]
//= a 4-letter word at 1
//= still by length [fig, pear, Apple, durian]
//= distinct [3, 1, 2] of 3
//= set claim 1
//= map claim 1
//= sorted claim 10..30
//= sorted map claim a
