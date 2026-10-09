//! [type-basic] [ir-op] [ir-dump] An operator on a basic type is an `Op`,
//! dumped inline; on any other type, `Str` included, it is a call of the
//! `cmp`/`eq` it resolved to.

use std::path::Path;

use salvo_core::{Program, SourceSet};

const SRC: &str = r#"
struct P { n: Int }

fn cmp(a: P, b: P) -> Int {
    return a.n - b.n
}

fn bigger<T>(a: T, b: T, ?Ordered<T>) -> Bool => a, b {
    return a > b
}

fn main() [use] -> None {
    use StdOutConsole
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
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, SRC.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(!expansion.diagnostics.iter().any(|d| d.is_error()));
    let program = Program { files: sources.files, modules, companions: sources.companions };
    let (text, errors) = salvo_ir::dump_program(&program, false, None).expect("checks");
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
    // `Str` is not basic: its operators are calls, intrinsic or not.
    for call in [
        "(call core.compare::cmp(Str, Str)(read s, \"c\") < 0)",
        "call core.compare::eq(Str, Str)(read s, \"b\")",
        "!(call core.compare::eq(Str, Str)(read s, \"a\"))",
    ] {
        assert!(ir.contains(call), "missing `{call}` in:\n{ir}");
    }
    // A declared `cmp`, and a forwarded implicit one, are called the same way.
    assert!(ir.contains("(call main::cmp(construct P { n: 1 }, construct P { n: 2 }) < 0)"), "{ir}");
    assert!(ir.contains("(call local cmp(read a, read b) > 0)"), "{ir}");
    // No basic-type comparison goes through `cmp`/`eq`; the intrinsic is only
    // the value passed to a generic.
    assert!(!ir.contains("call core.compare::cmp(Int, Int)("), "{ir}");
    assert!(!ir.contains("call core.compare::eq(Int, Int)("), "{ir}");
    assert!(ir.contains("fn core.compare::cmp(Int, Int)"), "{ir}");
}
