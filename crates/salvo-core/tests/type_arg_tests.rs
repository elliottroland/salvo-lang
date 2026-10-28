//! [call-type-args] What the checker records about a generic call's type
//! arguments, beyond its diagnostics: the resolved bindings a backend's
//! intrinsic lowering renders when it has to spell an element type out
//! [backend-intrinsic]. The diagnostics are test data,
//! `tests/check/type_args/` (TESTS.md T1); this reads the checker's table,
//! which annotations cannot.

use salvo_core::Ty;

/// The resolved bindings are recorded per call, in the callee's declaration
/// order — this is what a `define fn` template interpolates as `${T}`. Over
/// the real std, with only the rows of the user's file kept (std's are
/// seeded into every check).
#[test]
fn resolved_type_arguments_are_recorded_per_call() {
    let out = salvo_testkit::front::check_main("fn probe() -> Int {\n    let _xs: Mut List<Str> = Mut List()\n    return 1\n}\n");
    let errors = out.errors();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    let recorded: Vec<&Vec<Ty>> = out.checked.call_type_args.iter().filter(|(k, _)| k.file() >= user).map(|(_, v)| v).collect();
    assert!(
        recorded.iter().any(|args| args.len() == 1 && args[0] == Ty::named("Str")),
        "expected `T = Str` to be recorded, got {recorded:?}"
    );
}
