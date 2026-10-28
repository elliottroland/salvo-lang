// An `is` binding over a call evaluates its subject once [is-bind-once].

//--- an_is_binding_over_a_call_hoists_its_subject
// [is-bind-once] The defect this rule closed, as a program: an `is` binding
// whose subject is a **call** must evaluate it exactly once. Until 2026-09-16
// both backends emitted the subject twice — once for the test, once for the
// payload — so this loop called `remove_first` twice per turn and *silently
// dropped* every other element (and, for a linear element, its obligation).
//
// Four shapes, because the emitters have four paths: a `while` and an `if` in
// statement position, and both again in value position. The expected output is
// identical on the Kotlin backend.
//
// [is-bind-once] …and the shape it lowers to: one temporary, read by the test
// and by the binding, with the `while` becoming a `loop` so the evaluation
// happens once *per iteration*.
fn drain_ints(xs: Mut List<Int>) [Console] -> None => xs: Mut {
    // statement `while`: one call per turn, so every element is seen
    //~ emits: loop {
    //~|         let mut __subject_1: Option<i32> = crate::std_core_list::remove_first_platform::<i32>(&mut *xs);
    //~|         if !(__subject_1.is_some()) {
    // No subject is emitted inside the loop condition.
    //~ !emits: while ({ let __l
    // The binding reads the temporary, not a second call.
    //~ emits: let mut n = __subject_1.unwrap();
    while remove_first(xs) is Int n {
        println("  took ${n}")
    }
}

fn first_or(xs: Mut List<Int>, fallback: Int) [] -> Int => xs: Mut, fallback {
    // value-position `if`: one call, and the binding is that call's value
    return if remove_first(xs) is Int n { n } else { fallback }
}

fn sum_down(xs: Mut List<Int>) [] -> Int => xs: Mut {
    let total = 0
    // value-position `while`: the loop's value is its last body value
    let last = while remove_first(xs) is Int n {
        total = total + n
        n
    } else { 0 }
    discard(last)
    return total
}

fn main() [use] {
    use StdConsole()
    let a: Mut List<Int> = Mut List(1, 2, 3)
    drain_ints(a)
    println("left ${size(a)}")

    let b: Mut List<Int> = Mut List(7, 8)
    println("first ${first_or(b, 0)} then ${size(b)}")

    let c: Mut List<Int> = Mut List(1, 2, 3, 4)
    println("sum ${sum_down(c)} left ${size(c)}")

    // statement `if`: the call happens once, so the list loses exactly one
    let d: Mut List<Int> = Mut List(5, 6)
    if remove_first(d) is Int n {
        println("one ${n} left ${size(d)}")
    }
}
