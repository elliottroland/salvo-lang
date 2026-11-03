// What a call's deductions leave of the argument's qualifiers.

//--- calls_remove_qualifiers_per_declared_deductions
// [deduce-consume] A kept parameter sheds its removal set (declared −
// kept qualifiers) from the argument at the call site: after
// `take_head` (`[list: Mut]` on a `Filled Mut` param) the variable
// is no longer `Filled`, so a second call fails overload resolution.
// The explicit-empty `[list:]` form strips all declared qualifiers.
qualifier Filled<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

fn take_head<T>(list: Filled Mut List<T>) -> T => list: Mut {
    // `get` answers a projection, which a `-> T` cannot return [proj-type].
    //~ error[fn-syntax:return-mismatch]
    return list.get(0) ?: unreachable!()
}

fn main() [use] {
    use StdConsole
    let strings = Mut List("a", "b")
    if strings is Filled {
        let _s = take_head(strings)
        //~ error[call-resolve:no-overload]
        let _t = take_head(strings)
    }
}

//--- an_empty_deduction_strips_every_declared_qualifier
// `[list:]` strips every declared qualifier: after `weaken` the value
// no longer satisfies a `Filled`-requiring overload.
qualifier Filled<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

fn weaken<T>(list: Filled List<T>) -> None => list: None {
}

fn head<T>(list: Filled List<T>) -> T {
    // `get` answers a projection, which a `-> T` cannot return [proj-type].
    //~ error[fn-syntax:return-mismatch]
    return list.get(0) ?: unreachable!()
}

fn main() [use] {
    use StdConsole
    let strings = List("a", "b")
    if strings is Filled {
        weaken(strings)
        //~ error[call-resolve:no-overload]
        let _s = head(strings)
    }
}

//--- partial_qualifier_removal_merges_conservatively
// [deduce-consume] Qualifier removal on only *some* fall-through paths is
// conservative: the join keeps only the qualifiers common to all paths,
// so an overload requiring the maybe-removed qualifier no longer resolves
// (nested inside the `is` branch that established it).
qualifier Filled<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

fn take_head<T>(list: Filled Mut List<T>) -> T => list: Mut {
    // `get` answers a projection, which a `-> T` cannot return [proj-type].
    //~ error[fn-syntax:return-mismatch]
    return list.get(0) ?: unreachable!()
}

fn partial(flag: Bool) {
    let strings = Mut List("a", "b")
    if strings is Filled {
        if flag {
            let _x = take_head(strings)
        }
        //~ error[call-resolve:no-overload]
        let _y = take_head(strings)
    }
}

//--- exhaustive_deductions_drop_undeclared_qualifiers
// ===== D1: exhaustive and delta qualifier deductions =====
// [deduce-syntax] The fix for the qualifier-preservation unsoundness: a
// plain list is *exhaustive* (only those qualifiers survive, including
// ones the callee never declared), `-Q` is a delta, and a body that
// mutates a parameter may use neither keep-all nor a delta.
//
// The reproduction of the unsoundness: `clear` declares only `Mut`,
// so its exhaustive list drops the caller's `Filled`.
qualifier Filled of List<Int> {
    fn qualifies(list: List<Int>) -> Bool {
        return list.size() > 0
    }
}

fn clear(list: Mut List<Int>) [] -> None => list: Mut {
}

fn describe(list: Filled Mut List<Int>) -> Int {
    return list.size()
}

fn main() [use] {
    use StdConsole
    let xs = Mut List(1, 2)
    if xs is Filled {
        clear(xs)
        //~ error[call-resolve:no-overload]
        println("${describe(xs)}")
    }
}

//--- a_delta_deduction_keeps_what_it_does_not_name
// The delta form keeps everything it does not name — legal on a
// parameter nothing can invalidate (no `Mut`): here `Checked` survives
// even though the callee never declares it.
qualifier Filled of List<Int> {
    fn qualifies(list: List<Int>) -> Bool {
        return list.size() > 0
    }
}

qualifier Checked of List<Int> {
    fn qualifies(list: List<Int>) -> Bool {
        return true
    }
}

fn forget_nonempty(list: Filled List<Int>) [] -> None => list: -Filled {
}

fn needs_checked(list: Checked List<Int>) -> Int {
    return list.size()
}

fn main() [use] {
    use StdConsole
    let xs = List(1, 2)
    if xs is Filled Checked {
        forget_nonempty(xs)
        println("${needs_checked(xs)}")
    }
}

//--- a_written_delta_on_a_mut_parameter_is_trusted
// ...including on a fn with a `Mut` parameter whose body does not
// mutate it: a written delta is the author's contract, trusted rather
// than second-guessed (a body that *does* mutate cannot keep
// everything — see the `grow` case below).
qualifier Filled of List<Int> {
    fn qualifies(list: List<Int>) -> Bool {
        return list.size() > 0
    }
}

fn touch(list: Mut List<Int>) [] -> None => list: -Filled {
}

fn describe(list: Checked Mut List<Int>) -> Int {
    return list.size()
}

qualifier Checked of List<Int> {
    fn qualifies(list: List<Int>) -> Bool {
        return true
    }
}

fn main() [use] {
    use StdConsole
    let xs = Mut List(1)
    if xs is Checked {
        touch(xs)
        println("${describe(xs)}")
    }
}

//--- a_mutating_body_may_not_keep_everything
// A mutating body may not keep everything.
//
// NOTE (deduction-defaults change): the contract is now a pure function
// of the signature (user decision 2026-10-11) — `from_written` no longer
// consults body mutations, so the old "mutated by this function, so the
// deduction cannot keep every qualifier" message (raised at the written
// clause itself) can no longer fire from a plain `Mut List<Int>` with no
// qualifier to invalidate. The soundness rule survives, enforced instead
// by the body-validation pass with a more precise message naming the
// qualifier it may lose; reproducing it needs a qualifier on the
// parameter's type for the body to actually invalidate.
qualifier Filled of List<Int> {
    fn qualifies(list: List<Int>) -> Bool {
        return list.size() > 0
    }
}

//~ error[deduce-gained:qualifier-removed]
fn grow(list: Mut Filled List<Int>) -> None => list {
    list.add(1)
}
