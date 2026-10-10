//! [proj-opt-slot] [ir-dump] A borrow read at a kept owned position is a
//! `BorrowAs` in the IR, arm by arm — the one-arm case (`proj X` -> `X`)
//! included — so what an arm does is visible, even where a backend ignores it.

use std::path::Path;

use salvo_core::{Program, SourceSet};

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
    let xs: List<Mut A> = list_of(Mut A { n: 1 })
    let b = B { s: "b" }
    let n = one(get(xs, 0)!)
    bump(at(xs, 0)!)
    let v = if n > 0 { get(xs, 0)! } else { b }
    let m = either(v)
}
"#;

fn dump(src: &str) -> String {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, src.to_string(), false);
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
fn a_borrow_at_a_kept_position_is_a_borrow_as_arm_by_arm() {
    let ir = dump(SRC);
    assert!(ir.contains("borrow_as[proj Mut A -> A]("), "{ir}");
    assert!(ir.contains("borrow_as[ref<List<Mut A>> Mut A -> Mut A]("), "{ir}");
    assert!(ir.contains("borrow_as[proj Mut A -> A, B -> B]("), "{ir}");
}
