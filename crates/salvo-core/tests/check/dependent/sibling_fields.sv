// [qual-depend-field] A struct field's claim about a sibling field: `pos:
// Idx(str) Int` is an index of the `str` of the same value. A read of the
// field, a destructured local and a literal's value are each rooted to the
// sibling read the same way, so the total overload is chosen exactly when the
// index and the string are one value's pair — and never for another string.

// A total read beside an optional one, ranked by the claim as std's
// `char_at` pair is.
fn read_char(str: Str, index: Int) [] -> Char? => str, index {
    return char_at(str, index)
}

fn read_char(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    let c = read_char(str, (index: Int))
    assert!(c is Char)
    return c
}

struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

// A field read: `cursor.pos` is an `Idx(cursor.str) Int`.
fn a_field_read_roots_to_the_sibling(cursor: Mut Cursor) [] -> Char => cursor {
    return cursor.str.read_char(cursor.pos)
}

// Destructuring: `pos` is an `Idx(str) Int` about the local `str` bound
// beside it, which is `cursor.str`.
fn a_destructured_field_roots_to_the_sibling_local(cursor: Mut Cursor) [] -> Char => cursor {
    let { str, pos } = cursor
    return str.read_char(pos)
}

// Half and half: the destructured `str` is `cursor.str`, so the field read's
// claim is about it too — and the other way round.
fn a_destructured_sibling_is_the_field(cursor: Mut Cursor) [] -> Char => cursor {
    let { str, pos } = cursor
    let a = str.read_char(cursor.pos)
    let _b: Char = cursor.str.read_char(pos)
    return a
}

// A `pos` destructured alone still names `cursor.str`.
fn only_the_index_destructured(cursor: Cursor) [] -> Char => cursor {
    let { pos } = cursor
    return cursor.str.read_char(pos)
}

// An index of one cursor's string is not an index of another's: the
// optional overload takes the call.
fn another_cursors_string_refuses(a: Cursor, b: Cursor) [] -> Char => a, b {
    //~ error[fn-syntax:return-mismatch]
    return b.str.read_char(a.pos)
}

// Nor of an unrelated string.
fn an_unrelated_string_refuses(cursor: Cursor, other: Str) [] -> Char => cursor, other {
    //~ error[fn-syntax:return-mismatch]
    return other.read_char(cursor.pos)
}

// A literal's index must be about the string stored beside it.
fn a_literal_needs_the_stored_strings_index(s: Str) [] -> Char => s {
    let i = 0 as! Idx(s)
    let cursor = Cursor { str: s, pos: i }
    // The literal's `proj` field *is* `s`, so its claim serves `s` too.
    let _also: Char = s.read_char(cursor.pos)
    return cursor.str.read_char(cursor.pos)
}

fn a_literal_refuses_another_strings_index(s: Str, t: Str) [] -> None => s, t {
    let i = 0 as! Idx(t)
    //~ error[struct-decl:field-mismatch]
    let _c = Cursor { str: s, pos: i }
}

// A spread supplies the sibling: the new index must be about its string.
fn a_spread_supplies_the_sibling(c: Cursor, t: Str) [] -> Char => c, t {
    let d = copy(c)
    let e = copy(c)
    // Two copies are two values, as far as a claim can tell.
    //~ error[struct-decl:field-mismatch]
    let _same = Cursor { ...d, pos: e.pos }
    let i = 0 as! Idx(t)
    //~ error[struct-decl:field-mismatch]
    let _other = Cursor { ...e, pos: i }
    let f = copy(c)
    let g = Cursor { ...f, pos: f.pos }
    return g.str.read_char(g.pos)
}

// Assigning the index: it must be about this cursor's string.
fn assigning_the_index(cursor: Mut Cursor, other: Cursor) [] -> None => cursor: Mut, other {
    let j = cursor.pos
    cursor.pos = j
    //~ error[var-no-widen:assign-mismatch]
    cursor.pos = other.pos
}

// Assigning the string another field's claim is about is refused: the stored
// index would describe a string that is gone.
fn assigning_the_depended_on_field(cursor: Mut Cursor, s: Str) [] -> None => cursor: Mut, s {
    //~ error[qual-depend-field:assign-depended]
    cursor.str = s
}

// [var-drop-assign] A step leaves the claim behind, as `pos = pos + 1`
// would; an `is` test gives it back. The field itself cannot be stepped out
// of its claim.
fn a_step_drops_the_claim(cursor: Mut Cursor) [] -> Char? => cursor: Mut {
    let { str, pos } = cursor
    pos++
    //~ error[var-let-infer:annotation-mismatch]
    let _c: Char = str.read_char(pos)
    if pos is Idx(str) {
        return str.read_char(pos)
    }
    //~ error[var-no-widen:assign-mismatch]
    cursor.pos++
    return None
}

// A spread's index is about the spread's string: replacing the string alone
// would leave it describing the old one.
fn a_spread_cannot_replace_the_sibling_alone(c: Cursor, t: Str) [] -> None => c, t {
    let d = copy(c)
    //~ error[qual-depend-field:spread-pair]
    let _e = Cursor { ...d, str: t }
}

// A temporary has no identity: its field reads with no claim, and cannot
// stand where one is needed.
fn a_temporarys_index_has_no_claim(cursor: Mut Cursor, other: Cursor, t: Str) [] -> None => cursor: Mut, other, t {
    //~ error[var-no-widen:assign-mismatch]
    cursor.pos = copy(other).pos
    //~ error[var-let-infer:annotation-mismatch]
    let _i: Idx(t) Int = copy(other).pos
}

// A prefix step's value is the new one, without the old one's claim.
fn a_prefix_steps_value_has_no_claim(s: Str) [] -> Char => s {
    let i = 0 as! Idx(s)
    //~ error[fn-syntax:return-mismatch]
    return s.read_char(++i)
}

// A branch that steps the tested variable does not hand the test's fact on.
fn a_branch_that_steps_keeps_no_fact(s: Str) [] -> Char => s {
    let i = 0 as! Idx(s)
    if i is Idx(s) {
        i++
    } else {
        return 'x'
    }
    //~ error[fn-syntax:return-mismatch]
    return s.read_char(i)
}

// A cursor built here holds `s` in its one `proj` field: destructured, its
// `str` is `s`, and its `pos` an index of `s`.
fn a_local_cursor_destructured(s: Str) [] -> Char => s {
    let cursor = Cursor { str: s, pos: 0 as! Idx(s) }
    let { str, pos } = cursor
    let _a: Char = s.read_char(pos)
    let _b: Char = str.read_char(cursor.pos)
    return cursor.str.read_char(pos)
}

// Two views of one string the analysis cannot place are two values.
fn two_unplaced_views_are_two_values(xs: List<Str>) [] -> Char => xs {
    let a = get(xs, 0) ?: unreachable!()
    let b = get(xs, 1) ?: unreachable!()
    let i = 0
    assert!(i is Idx(a))
    let _c: Char = a.read_char(i)
    //~ error[fn-syntax:return-mismatch]
    return b.read_char(i)
}

// A claimed field written through an element at an unknown index: there is
// no telling which cursor's string the claim is about, so no value fits.
fn writing_through_an_element(s: Str) [] -> None => s {
    let cs = array_of(Mut Cursor { str: s, pos: 0 as! Idx(s) })
    let j = 0 as! Idx(s)
    //~ error[var-no-widen:assign-mismatch]
    cs[0].pos = j
    //~ error[var-no-widen:assign-mismatch]
    cs[0].pos++
}

// A branch that strips the claim by a mutation does not hand the test's fact
// on either.
fn a_branch_that_mutates_keeps_no_fact(xs: Mut List<Int>) [] -> Int? => xs: Mut {
    let i = 0
    if i is Idx(xs) {
        let _gone = remove_last(xs)
    } else {
        return None
    }
    //~ error[var-let-infer:annotation-mismatch]
    let _n: Int = get(xs, i)
    return None
}

// A branch that ends with another claim of the same name hands that one on,
// not the pre-branch one.
fn a_branch_that_ends_with_another_claim(s: Str, t: Str) [] -> Char => s, t {
    let i = 0 as! Idx(t)
    if i is Idx(s) {
        i++
        assert!(i is Idx(s))
    }
    //~ error[fn-syntax:return-mismatch]
    return t.read_char(i)
}

// Two paths with claims of one name about different strings keep neither.
fn paths_with_different_claims(s: Str, t: Str, c: Bool) [] -> Char => s, t {
    let i = 0 as! Idx(t)
    let k = 0 as! Idx(s)
    if c {
        i = k
    }
    //~ error[fn-syntax:return-mismatch]
    return s.read_char(i)
}

// A loop whose condition narrows the index leaves it stepped, not claimed:
// after the loop it is what the body left or what came in.
fn after_a_loop_that_steps(s: Str) [] -> Char => s {
    let i = 0 as! Idx(s)
    while i is Idx(s) {
        i++
    }
    //~ error[fn-syntax:return-mismatch]
    return s.read_char(i)
}

// A `when` arm that steps the subject hands on the stepped value.
fn after_a_when_arm_that_steps(s: Str, j: (Idx(s) Int)?) [] -> Char => s, j {
    let i = j
    when i {
        is None { return 'x' }
        is Int { i++ }
    }
    //~ error[call-resolve:no-overload]
    return s.read_char(i)
}

// An `if` that picked an arm and stepped it leaves no claim on the arm.
fn after_an_arm_step(s: Str, j: (Idx(s) Int)?) [] -> Char => s, j {
    let i = j
    if i is Int {
        i++
    }
    if i is Int {
        //~ error[fn-syntax:return-mismatch]
        return s.read_char(i)
    }
    return 'x'
}

// A sibling reached through a path must not be mutable anywhere along it,
// and a write anywhere along it is a write to the sibling.
struct Inner { text: Str }
struct Holder canbe Mut {
    inner: Inner
    at: Idx(inner.text) Int
}

fn a_path_sibling(h: Mut Holder, other: Inner) [] -> Char => h: Mut, other {
    let c = h.inner.text.read_char(h.at)
    //~ error[qual-depend-field:assign-depended]
    h.inner = other
    return c
}

struct MutInner { items: Mut List<Int> }
struct Deep {
    inner: MutInner
    //~ error[qual-depend-field:mutable-sibling]
    at: Idx(inner.items) Int
}

// A generic sibling's claim is trusted only where the type argument cannot
// change in place.
struct Pair<C> {
    items: C
    at: Idx(items) Int
}

fn a_generic_sibling(p: Pair<List<Int>>, q: Pair<Mut List<Int>>) [] -> Int => p, q {
    let a = get(p.items, p.at)
    //~ error[var-let-infer:annotation-mismatch]
    let _b: Int = get(q.items, q.at)
    return a
}

// A claim about an `Int` sibling: stepping the sibling is a write to it.
qualifier Below(n: Int) for Int {
    fn qualifies(k: Int, n: Int) -> Bool {
        return k < n
    }
}

struct Bounded canbe Mut {
    n: Int
    k: Below(n) Int
}

fn stepping_the_sibling(b: Mut Bounded) [] -> None => b: Mut {
    //~ error[qual-depend-field:assign-depended]
    b.n--
}

// A pass bound to a name still mints claims about what it lends from, while
// its own fields, kept by a call, are about nothing the analysis can name.
fn a_named_pass_mints_for_its_source(xs: List<Int>) [] -> Int => xs {
    let it = indices(xs)
    let total = 0
    for i in it {
        total = total + get(xs, i)
    }
    return total
}

// [qual-depend-field] Two fields of one value are two values: an index of
// `p.a` is not an index of `p.b`, however it was reached.
struct Two { a: Str, b: Str }

fn a_field_is_not_its_sibling(p: Two) [] -> Char => p {
    let i = 0
    assert!(i is Idx(p.a))
    //~ error[fn-syntax:return-mismatch]
    return p.b.read_char(i)
}

fn destructured_fields_are_not_each_other(p: Two) [] -> Char => p {
    let { a, b } = p
    let i = 0
    assert!(i is Idx(a))
    let _c: Char = a.read_char(i)
    let _d: Char = p.a.read_char(i)
    //~ error[fn-syntax:return-mismatch]
    return b.read_char(i)
}

fn bound_fields_are_not_each_other(p: Two) [] -> Char => p {
    let a = p.a
    let b = p.b
    let i = 0
    assert!(i is Idx(a))
    //~ error[fn-syntax:return-mismatch]
    return b.read_char(i)
}

// An index destructured from one cursor is not an index of another's string.
fn a_destructured_index_of_another_cursor(a: Cursor, b: Cursor) [] -> Char => a, b {
    let { pos } = a
    let _ok: Char = a.str.read_char(pos)
    //~ error[fn-syntax:return-mismatch]
    return b.str.read_char(pos)
}

// A literal's index serves the string stored in it, and no other.
fn a_literals_index_is_not_anothers(s: Str, t: Str) [] -> Char => s, t {
    let cursor = Cursor { str: s, pos: 0 as! Idx(s) }
    //~ error[fn-syntax:return-mismatch]
    return t.read_char(cursor.pos)
}

// A cursor variable given another value: an index read from the old one is
// not kept, since the variable's `str` is a different string now.
fn the_cursor_reassigned(a: Cursor, b: Cursor) [] -> Char => a, b {
    let cursor = copy(a)
    let i = cursor.pos
    let _ok: Char = cursor.str.read_char(i)
    cursor = copy(b)
    // What the new value's own field says still holds.
    let _also: Char = cursor.str.read_char(cursor.pos)
    //~ error[fn-syntax:return-mismatch]
    return cursor.str.read_char(i)
}

// [qual-depend] A call that may mutate the cursor strips every claim about
// it, as it strips a claim about a map it may change.
fn touch(cursor: Mut Cursor) [] -> None => cursor: Mut {
    let _ = cursor
}

fn the_cursor_mutated_by_a_call(cursor: Mut Cursor) [] -> Char => cursor: Mut {
    let i = cursor.pos
    let _ok: Char = cursor.str.read_char(i)
    touch(cursor)
    //~ error[fn-syntax:return-mismatch]
    return cursor.str.read_char(i)
}

// [qual-depend-field] An element is not what the loop walks, nor another
// loop's element: a claim about one is about that element.
struct Plain { str: Str, pos: Idx(str) Int }

fn two_elements_of_one_list(cs: List<Plain>) [] -> Char? => cs {
    for a in cs {
        let _own: Char = a.str.read_char(a.pos)
        for b in cs {
            //~ error[var-let-infer:annotation-mismatch]
            let _c: Char = b.str.read_char(a.pos)
        }
    }
    for { str, pos } in cs {
        let _own: Char = str.read_char(pos)
    }
    return None
}

fn an_elements_index_is_not_the_lists(strs: List<Str>) [] -> None => strs {
    let i = 0
    for s1 in strs {
        if i is Idx(s1) {
            let _own: Char = s1.read_char(i)
            //~ error[var-let-infer:annotation-mismatch]
            let _n: Str = get(strs, i)
            for s2 in strs {
                //~ error[var-let-infer:annotation-mismatch]
                let _c: Char = s2.read_char(i)
            }
        }
    }
}

// Two fields bound out of a value that holds a borrow are two values, as
// its two field reads are.
struct Doc { str: proj Str, a: Str, b: Str }

fn fields_beside_a_held_borrow(s: Str) [] -> None => s {
    let d = Doc { str: s, a: "abcdef", b: "" }
    let a = d.a
    let b = d.b
    let { a: a2, b: b2 } = d
    let i = 5
    assert!(i is Idx(a))
    let _same: Char = d.a.read_char(i)
    //~ error[var-let-infer:annotation-mismatch]
    let _x: Char = b.read_char(i)
    //~ error[var-let-infer:annotation-mismatch]
    let _y: Char = b2.read_char(i)
    //~ error[var-let-infer:annotation-mismatch]
    let _z: Char = d.b.read_char(i)
    let _w = a2
}

// Fields destructured out of an element: likewise two values.
fn fields_of_an_element(ds: List<Two>) [] -> None => ds {
    for { a, b } in ds {
        let i = 0
        assert!(i is Idx(a))
        let _own: Char = a.read_char(i)
        //~ error[var-let-infer:annotation-mismatch]
        let _c: Char = b.read_char(i)
    }
}

// [is-narrow-assign-reset] A `continue` path reaches the back edge too: a
// step on it is not undone by the end of the body.
fn a_step_before_continue(s: Str, xs: List<Int>) [] -> None => s, xs {
    let i = 0 as! Idx(s)
    for x in xs {
        //~ error[var-let-infer:annotation-mismatch]
        let _c: Char = s.read_char(i)
        if x > 0 {
            i++
            continue
        }
    }
}

fn a_step_before_continue_in_a_while(s: Str, c: Bool) [] -> Char => s, c {
    let i = 0 as! Idx(s)
    let go = true
    while go {
        go = false
        if c {
            i++
            continue
        }
    }
    //~ error[fn-syntax:return-mismatch]
    return s.read_char(i)
}

fn a_claim_given_back_before_continue(s: Str, n: Int) [] -> Char => s, n {
    let i = 0 as! Idx(s)
    let k = 0
    while k < n {
        k++
        let _c: Char = s.read_char(i)
        if k > 2 {
            i++
            assert!(i is Idx(s))
            continue
        }
    }
    return s.read_char(i)
}

// A `for` may run no times: what its body asserted does not hold after it.
fn after_a_for_that_may_not_run(s: Str, xs: List<Int>) [] -> Char => s, xs {
    let i = 5
    for _x in xs {
        assert!(i is Idx(s))
    }
    //~ error[fn-syntax:return-mismatch]
    return s.read_char(i)
}

// [qual-depend] A mutation strips a claim on a union's arm too.
fn a_mutation_strips_a_claim_on_an_arm(xs: Mut List<Int>) [] -> Int? => xs: Mut {
    let i = index_of(xs, 3)
    let _gone = remove_last(xs)
    if i is Int {
        //~ error[var-let-infer:annotation-mismatch]
        let _n: Int = get(xs, i)
    }
    return None
}

// [flow-place] And on a place's narrowing.
struct Slot { i: Int }

fn a_mutation_strips_a_place_claim(xs: Mut List<Int>, p: Slot) [] -> Int? => xs: Mut, p {
    if p.i is Idx(xs) {
        let _ok: Int = get(xs, p.i)
        let _gone = remove_last(xs)
        //~ error[var-let-infer:annotation-mismatch]
        let _n: Int = get(xs, p.i)
    }
    return None
}

// The declaration: a claim names another field, one that cannot change in
// place.
struct Lost {
    //~ error[qual-depend-field:not-a-sibling]
    pos: Idx(nowhere) Int
}

struct Selfish {
    //~ error[qual-depend-field:not-a-sibling]
    pos: Idx(pos) Int
}

struct Moving canbe Mut {
    items: List<Int> canbe Mut
    //~ error[qual-depend-field:mutable-sibling]
    at: Idx(items) Int
}

fn main() {}
