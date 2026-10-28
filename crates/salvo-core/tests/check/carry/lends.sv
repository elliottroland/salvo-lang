// ===== the written opaque lend [proj-infer] =====

// [proj-infer] What `filter`'s written lend **buys**, over std's own
// declaration: `-> Mut List<proj T> holds proj(it)` names the pass as the
// only source, so a later mutation of something the *predicate* captured
// leaves the filtered view standing. Without the annotation the
// conservative fallback links the result to every kept argument — the
// callback included — and this program is refused.
//
// The spelling is the 2026-09-25 respelling (`T holds proj(x)` after the
// type, reading "is a `T`, holds a borrow of `x`"); the fact it pins is why
// the annotation exists at all, which is what made the respelling worth
// doing rather than deleting the annotation.
fn narrows() [use] -> None {
    use StdConsole()
    let words: List<Str> = List("a", "bb")
    let limits: Mut List<Int> = Mut List(1)
    let p = iter(words)
    let long = filter(p, (s: Str) -> { return size(s) > (first(limits) ?: unreachable!()) })
    add(limits, 2)
    println("${size(long)}")
}

// [proj-infer] …and the lend it *does* name still links: moving the pass's
// source poisons the view ("shares its fate"), which is the half no
// annotation could remove.
fn eat(words: List<Str>) [] -> None => !words {
    return None
}

fn still_links() [use] -> None {
    use StdConsole()
    let words: List<Str> = List("a", "bb")
    let p = iter(words)
    let long = filter(p, (s: Str) -> { return size(s) > 1 })
    eat(words)
    //~ error[fate-poison:shared-root]
    println("${size(long)}")
}

// ===== [col-no-literal] a bare literal determines a type argument =====

// [col-no-literal] A bare collection constructor as an argument determines the
// callee's type parameter (defect closed 2026-09-25). The cause was subtle
// enough to pin: substituting an **unbound** variable yields `Unknown`, so the
// parameter pattern `List<T>` arrived at the literal looking concrete
// (`List<Unknown>`), the literal adopted `Unknown` as its element type and
// discarded what its own elements said — and the call then had nothing to
// infer `T` from. Run against std's own `to_set`/`to_list`, which is where it
// was found.
fn bare_literal_arguments() [use] -> None {
    use StdConsole()
    let a = to_set(List(1, 2))
    let b = to_list(Set(3, 4))
    let c = to_set(List("x", "y"))
    println("${size(a)} ${size(b)} ${size(c)}")
}

// …and the diagnostic still fires where nothing *can* determine the argument
// (an empty `List()`), which is the rule the fix had to leave standing
// [col-no-literal]. That case stays a Rust test, `carry_tests.rs`: over the
// real std it reports `cannot infer` twice, for `List`'s `T` and for
// `to_set`'s.
