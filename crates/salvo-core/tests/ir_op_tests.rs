//! [type-basic] [ir-op] [ir-dump] An operator on a basic type is an `Op`,
//! dumped inline; on any other type, `Str` included, it is a call of the
//! `cmp`/`eq` it resolved to.



const SRC: &str = r#"
struct P { n: Int }

fn cmp(a: P, b: P) -> Int {
    return a.n - b.n
}

fn bigger<T>(a: T, b: T, ?Ordered<T>) -> Bool => a, b {
    return a > b
}

fn main() [use] -> None {
    use StdConsole
    let i = 3
    let l: Long = 4
    let d = 1.5
    let s = "b"
    println("${i < 4} ${i == 3} ${i != 2} ${l >= i} ${d > 1.0} ${true > false} ${'a' < 'b'}")
    println("${s < "c"} ${s == "b"} ${s != "a"}")
    println("${P { n: 1 } < P { n: 2 }}")
    println("${bigger(2, 7)}")
}
"#;

fn dump() -> String {
    // [std-cache] Over the process's std front (TESTS.md T3).
    let program = salvo_testkit::front::clean_program("kt", &[("main.sv", SRC)]);
    let (text, errors) = salvo_testkit::front::dump_ir("kt", &program, false, None).expect("checks");
    assert!(errors.is_empty(), "{errors:?}");
    text
}

#[test]
fn basic_types_compare_with_the_host_operator_and_the_rest_call() {
    let ir = dump();
    // Basic types: the operator itself, widened per [op-promote].
    for op in [
        "(read i < 4)",
        "(read i == 3)",
        "(read i != 2)",
        "(read l >= widen[Long](read i))",
        "(read d > 1.0)",
        "(true > false)",
        "('a' < 'b')",
    ] {
        assert!(ir.contains(op), "missing `{op}` in:\n{ir}");
    }
    // `Str` is not basic: its operators are calls, intrinsic or not, of the
    // fns declared beside it [unique-shape].
    for call in [
        "(call std.core.string::cmp(read s, \"c\") < 0)",
        "call std.core.string::eq(read s, \"b\")",
        "!(call std.core.string::eq(read s, \"a\"))",
    ] {
        assert!(ir.contains(call), "missing `{call}` in:\n{ir}");
    }
    // A declared `cmp`, and a forwarded implicit one, are called the same way.
    assert!(ir.contains("(call main::cmp(construct P { n: 1 }, construct P { n: 2 }) < 0)"), "{ir}");
    assert!(ir.contains("(call local cmp(read a, read b) > 0)"), "{ir}");
    // No basic-type comparison goes through `cmp`/`eq`; the intrinsic is only
    // the value passed to a generic.
    assert!(!ir.contains("call std.core.basic::cmp(Int, Int)("), "{ir}");
    assert!(!ir.contains("call std.core.basic::eq(Int, Int)("), "{ir}");
    assert!(ir.contains("fn std.core.basic::cmp(Int, Int)"), "{ir}");
}
