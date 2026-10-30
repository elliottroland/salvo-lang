//! [proj-opt-slot] [ir-dump] A borrow read at a kept owned position is a
//! `BorrowAs` in the IR, arm by arm — the one-arm case (`proj X` -> `X`)
//! included — so what an arm does is visible, even where a backend ignores it.



const SRC: &str = r#"
struct A canbe Mut { n: Int }
struct B { s: Str }

fn one(a: A) -> Int => a {
    return a.n
}

fn bump(a: Mut A) -> None => a: Mut {
    a.n = a.n + 1
}

fn either(x: A | B) -> Int => x {
    if x is A {
        return x.n
    }
    return 0
}

fn main() [use] -> None {
    let xs: List<Mut A> = List(Mut A { n: 1 })
    let b = B { s: "b" }
    let n = one(get(xs, 0) ?: unreachable!())
    bump(at(xs, 0) ?: unreachable!())
    let v = if n > 0 { get(xs, 0) ?: unreachable!() } else { b }
    let m = either(v)
}
"#;

fn dump(src: &str) -> String {
    // [std-cache] Over the process's std front (TESTS.md T3).
    let program = salvo_testkit::front::clean_program("kt", &[("main.sv", src)]);
    let (text, errors) = salvo_testkit::front::dump_ir("kt", &program, false, None).expect("checks");
    assert!(errors.is_empty(), "{errors:?}");
    text
}

#[test]
fn a_borrow_at_a_kept_position_is_a_borrow_as_arm_by_arm() {
    let ir = dump(SRC);
    assert!(ir.contains("borrow_as[proj Mut A@main -> A@main]("), "{ir}");
    assert!(ir.contains("borrow_as[ref<List<Mut A@main>> Mut A@main -> Mut A@main]("), "{ir}");
    assert!(ir.contains("borrow_as[proj Mut A@main -> A@main, B@main -> B@main]("), "{ir}");
}
