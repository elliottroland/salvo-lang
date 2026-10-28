// ===== the `Sorted` claim [col-sorted-list] =====
//
// Against **std's own** declarations: what is under test is the shape
// `std.core.list` gives `sort`/`add_sorted`/`binary_search`.

fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

// The ordering a program of its own: `sort` publishes it into the claim, and
// the two operations that read the claim capture it [cmp-binder] — so the
// search and the insert are computed with the ordering the list actually
// carries rather than with whatever is visible at the call.
fn carries_its_ordering() [] -> Int? {
    let ordered = sort(List("pear", "fig"), cmp = by_len)
    let growing = mut_sort(List("pear", "fig"), cmp = by_len)
    add_sorted(growing, "durian")
    return binary_search(ordered, "kiwi")
}

// [cmp-carry] Two orderings are two types here too: a list sorted by `by_len`
// does not fit a position demanding the canonical `cmp`, and the diagnostic
// reads the ordering out of the type ("Sorted<Str>(by_len)") — which is the
// whole reason the claim carries it. Before step 5 the claim said only
// "sorted", so this call was accepted and the search ran under the wrong
// ordering.
fn needs_canonical(xs: Sorted<Str>(cmp) List<Str>) [] -> Int? => xs {
    return binary_search(xs, "x")
}

fn sorted_by_another() [] -> Int? {
    let by = sort(List("pear"), cmp = by_len)
    //~ error[call-resolve:no-overload]
    return needs_canonical(by)
}

// [cmp-binder] One binder twice is one binding, so two sorted lists in one
// signature must have been sorted the same way (the diagnostic names
// "Sorted<Str>(cmp…)" and "Sorted<Str>(by_len)").
fn merge<T>(a: Sorted<T>(?cmp) List<T>, b: Sorted<T>(?cmp) List<T>, probe: T) [] -> Int?
=> a, b, probe {
    return binary_search(a, probe)
}

fn merge_mixed() [] -> Int? {
    let one = sort(List("pear"))
    let other = sort(List("fig"), cmp = by_len)
    //~ error[call-resolve:no-overload]
    return merge(one, other, "x")
}

// The same source with both lists sorted the same way needs no annotation
// anywhere: the binder is filled from the argument types.
fn merge_agreeing() [] -> Int? {
    let one = sort(List("pear"))
    let other = sort(List("fig"), cmp = cmp)
    return merge(one, other, "x")
}

// [cmp-carry] A **written type is a pattern for its slots**, an annotation
// included: `Mut Sorted List<Int>` names the claim without naming its
// ordering, and the value keeps carrying the one `mut_sort` published — which
// is what lets `add_sorted` capture it.
fn annotation_keeps_it() [] -> None {
    let live: Mut Sorted List<Int> = mut_sort(List(10, 40))
    add_sorted(live, 20)
}

// An annotation that *does* name an ordering is a demand like any other, and
// the wrong one is refused, naming both: "Sorted<Int>(by_size)" and
// "Sorted<Int>(cmp@Int)" ([fn-attached] std's `cmp(Int, Int)` is declared
// beside `Int` and fulfils its `Ordered<self>`, so it is shown as declared on
// it, since [op-unique-shape] put it in `Int`'s file, 2026-10-27).
fn by_size(a: Int, b: Int) [] -> Int => a, b {
    return cmp(b, a)
}

fn annotation_names_another() [] -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let live: Mut Sorted<Int>(by_size) List<Int> = mut_sort(List(10, 40))
    add_sorted(live, 20)
}
