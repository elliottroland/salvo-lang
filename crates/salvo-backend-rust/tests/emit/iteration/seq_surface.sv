// The combinator surface [seq-into].

//--- the_combinator_surface
// [seq-into] The combinator surface: **eager**, over a pass, with `map_to`
// for mapping into a collection the caller provides. The lazy pair went with
// the 2026-09-10 removal, so the generic bodies are reached by passing a pass
// explicitly — which is what `map(iter(copy(xs)), …)` is for here.
//
// Same source and stdout as the Kotlin backend's twin. What it exercises that
// nothing did before: the *generic* `?Next` body — `map`/`filter` over a
// list take their `List` fast path, so the polymorphic one had never run —
// and an implicit with a `Mut` parameter (`?add`).

// The generic body drives its subject through the `next` the call site
// resolved, which arrives as an ordinary borrowed callback [implicit-group].
//~ emits(std/core/seq.rs): next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>
// [fn-contract] A kept `Mut` position of an implicit's fn type borrows
// mutably: `add` cannot append to a destination handed over by value.
//~ emits(std/core/seq.rs): add: &mut dyn FnMut(&mut D, U)
// [proj-infer-fn-type] The lend inference is scoped to an opaque return and
// a lend names a source, so a slot with no parameter name (std's
// `?copy: (T) -> T`) gets no lifetime: `std.core.seq`'s combinators carry
// exactly that slot.
//~ !emits(std/core/seq.rs): &'c T
fn double(n: Int) -> Int {
    return n * 2
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdConsole()
    let xs = List(1, 2, 3, 4)
    let doubled = map(xs, double)
    println("eager ${doubled.size()}")
    let generic = map(iter(copy(xs)), double)
    println("generic ${generic.size()}")
    let snapshot = copy(xs)
    let kept = filter(iter(snapshot), is_even)
    println("kept ${kept.size()}")
    // [rs-implicit-turbofish] The instantiation is spelled out, or the adapter
    // closures have nothing to infer their parameter types from.
    //~ emits(main.rs): crate::std_core_seq::map_to::<Vec<i32>, crate::std_core_list::ListNext<'_, i32>, i32, i32>(
    let out = map_to(Mut List<Int>(), iter(copy(xs)), double)
    println("sink ${out.size()}")
    let chained = filter_to(map_to(Mut List<Int>(), iter(copy(xs)), double), iter(xs), is_even)
    println("chained ${chained.size()}")
}
