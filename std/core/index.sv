// [col-idx] `core.index`: the claims about positions in a container — `Idx`,
// an index proven in range, and `NotEq`, an index proven different from
// another. Their own module (user decision 2026-10-05, ROADMAP §0j 6g0), since
// one `Idx` serves every container with a `size`: `List`, `Deque` and the
// buffers. `core.*` is visible everywhere, so nothing imports it.

// [qual-depend] [col-idx] The claim that an `Int` is a **valid index of one
// particular container**: `0 <= index < size(c)`, bound to that container's
// identity, so `Idx(xs) Int` and `Idx(ys) Int` are different facts. Tested
// with the filled block (`i is Idx(xs)`), stripped by any mutation of the
// container; the total `get`, `swap` and `replace` overloads consume it.
//
// Any container with a `size` (user decision 2026-10-05): `qualifies` takes the
// `size` as an implicit, resolved where the claim is tested at the concrete
// container — `i is Idx(xs)` over a `List<Int>` passes `size(List<T>)`, over a
// `Mut IntBuffer` the buffer's — so nothing about it prints in the type.
// `<C canbe linear>`: a list of obligations keeps its claims too.
export qualifier Idx<C canbe linear>(c: C) of Int {
    fn qualifies(index: Int, c: C, ?size: (c: C) -> Int) -> Bool {
        return index >= 0 && index < size(c)
    }

    // [qual-preserve] Growth keeps every existing index valid, and an exchange
    // or a write in place moves no boundary: `Idx` claims survive them — the
    // opt-back from the conservative rule that any mutation of the container
    // strips them. Each container's own; the refinement lives with the claim
    // [qual-refn-scope], so they are all here.
    refn add<T>(list: Mut List<T>, elem: T) => list: preserve Idx
    refn swap<T>(list: Mut List<T>, i: Int, j: Int) => list: preserve Idx
    // [col-replace] The total write.
    refn replace<T>(list: Mut List<T>, index: Idx(list) Int, value: T) => list: preserve Idx
    // [col-deque] Growth at either end keeps every index in range (it is the
    // *position* an index names that moves at the front, not the bound).
    refn add_last<T>(d: Mut Deque<T>, elem: T) => d: preserve Idx
    refn add_first<T>(d: Mut Deque<T>, elem: T) => d: preserve Idx
    refn replace<T>(d: Mut Deque<T>, index: Idx(d) Int, value: T) => d: preserve Idx
    // [buffer-type] A buffer's length never changes.
    refn replace(buf: Mut IntBuffer, index: Idx(buf) Int, value: Int) => buf: preserve Idx
    refn clear(buf: Mut IntBuffer, fill: Int) => buf: preserve Idx
    refn replace(buf: Mut LongBuffer, index: Idx(buf) Int, value: Long) => buf: preserve Idx
    refn clear(buf: Mut LongBuffer, fill: Long) => buf: preserve Idx
}

// [qual-depend] [col-noteq] The claim that an `Int` **differs from one
// particular other `Int`**: `j is NotEq(i)` proves `j != i`, bound to
// `i`'s identity. For two element handles of one list it is the proof that
// they cannot alias — mutation through one leaves the other standing
// [elem-distinct], and a call may take both at once — which is what the
// `update2` family stands on. Reassigning either side strips it
// [qual-depend], like any dependent claim.
export qualifier NotEq(i: Int) of Int with Idx {
    fn qualifies(j: Int, i: Int) -> Bool {
        return j != i
    }
}

// [col-idx] Every index of [list], front to back — and each emitted `Int`
// carries the claim: `for i in indices(xs)` makes `get(xs, i)` total. Sound
// because the iterator borrows the list [proj-field], so nothing can shrink it
// while the loop runs [proj-infer].
//
// The `+Idx` is [deduce-reapply]'s establishment in a return-type arm, trusted
// in the file that declares `Idx` and only there — which is why the indices
// live here rather than in `core.list`; the bounds test above the emit is the
// proof the trust rests on. [qual-depend] The claim names the parameter; in
// the generated struct's obligation it names the struct's own borrowed field,
// and a `for` binds it to the *source* list's identity, so the claim reads
// `Idx(xs)` in the loop body.
export iter fn indices<T>(list: List<T>) -> Emitted (+Idx(list) Int) | Finished {
    state {
        // The index the next turn emits.
        at: Int = 0
    }
    if at >= size(list) {
        return finished()
    }
    let index = at.copy()
    at = at + 1
    return emitted(index)
}

// [col-idx] The same indices, back to front: `size(list) - 1` down to `0` —
// **the founding example of the refinement-types design**: the descending
// loop whose body needs no `!`.
export iter fn rev_indices<T>(list: List<T>) -> Emitted (+Idx(list) Int) | Finished {
    state {
        at: Int = size(list) - 1
    }
    if at < 0 {
        return finished()
    }
    let index = at.copy()
    at = at - 1
    return emitted(index)
}
