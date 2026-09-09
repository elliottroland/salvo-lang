//! [linear-group] `Linear` as a designated obligation group (roadmap R4, user
//! decisions 2026-09-08): declaring `: Linear<self>` *is* declaring how the
//! obligation is discharged, because the group's `close` must be supplied.
//!
//! Two things this settles that the `canbe Linear` spelling could not: a
//! `close` is **required** of anything claiming to be linear, and `discard` no
//! longer discharges — dropping a handle is the leak the obligation exists to
//! prevent. The presence of a `close` never implies linearity either; only the
//! clause does.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n\
     intrinsic type List<T> canbe Mut\n\
     intrinsic fn copy<T>(value: T) [] -> [value] T\n\
     intrinsic fn discard<T canbe Linear>(value: T) [] -> [] None\n\
     intrinsic fn mutable_list<T canbe Linear>(...elems: T[]) [] -> [] Mut List<T>\n\
     intrinsic fn add<T canbe Linear>(list: Mut List<T>, elem: T) [] -> [list: Mut] None\n\
     intrinsic fn size<T canbe Linear>(list: List<T>) [] -> [list] Int\n\
     params Linear<It> {\n    fn close(it: It) -> [] None\n}\n";

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| d.message.clone())
        .collect()
}

const LINES: &str = r#"
struct Lines : Linear<self> {
    name: Str
}

fn close(l: Lines) -> [] None {}

fn open_lines(n: Str) -> [] Lines {
    return Lines { name: n }
}
"#;

/// The `close` implementation is where a linear value legitimately dies — the
/// one place linearity would otherwise make impossible to write.
#[test]
fn declaring_the_obligation_with_its_close_is_clean() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> [] None {{\n    let l = open_lines(\"a\")\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [group-obligation] Declared without one, the error lands at the *struct*
/// and names the signature it wants.
#[test]
fn declaring_it_without_a_close_errors_at_the_struct() {
    let errs = errors(
        "struct Leaky : Linear<self> {\n    fd: Int\n}\n",
    );
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("`Leaky` declares `: Linear`")
            && errs[0].contains("fn close(Leaky) -> None"),
        "got {errs:?}"
    );
}

/// [linear-discard] The rule that changed: `discard` is no longer the escape
/// hatch for a linear value, and the diagnostic names what is.
#[test]
fn discard_cannot_drop_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> [] None {{\n    let l = open_lines(\"a\")\n    discard(l)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`discard` cannot drop a linear value")
            && e.contains("call its `close`")),
        "got {errs:?}"
    );
}

/// It still drops a non-linear one: that is what it is for now.
#[test]
fn discard_still_drops_a_plain_value() {
    let errs = errors("fn go() -> [] None {\n    discard(\"note\")\n}\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The leak diagnostic names `close`, not `discard`.
#[test]
fn a_leak_names_close_as_the_discharge() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> [] None {{\n    let l = open_lines(\"a\")\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("still owns a linear value") && e.contains("`close(l)`")),
        "got {errs:?}"
    );
}

/// [linear-group] A `close` **never** implies linearity (user decision
/// 2026-09-08): only the clause does. Without it the type is ordinary, so
/// letting a value go out of scope is unremarkable — which is what keeps a
/// generated pass with a `defer` composable.
#[test]
fn a_close_alone_does_not_make_a_type_linear() {
    let errs = errors(
        "struct Plain {\n    fd: Int\n}\n\
         fn close(p: Plain) -> [] None {}\n\
         fn make() -> [] Plain {\n    return Plain { fd: 1 }\n}\n\
         fn go() -> [] None {\n    let p = make()\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [canbe-optin] The old spelling is gone from *declarations*: `canbe` grants
/// a qualifier, `:` declares an obligation. The error names the new form.
#[test]
fn canbe_linear_on_a_declaration_names_the_obligation_form() {
    let errs = errors("struct Old canbe Linear {\n    fd: Int\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("linearity is declared as an obligation")
            && e.contains("`: Linear<self>`")),
        "got {errs:?}"
    );
}

/// [linear-generics] But `canbe Linear` on a **type parameter** keeps its
/// spelling: permission on a parameter is not obligation on a declaration.
#[test]
fn canbe_linear_on_a_type_parameter_still_works() {
    let errs = errors(&format!(
        "{LINES}\n\
         fn hold<T canbe Linear>(value: T) -> T {{\n    return value\n}}\n\
         fn go() -> [] None {{\n    let l = hold(open_lines(\"a\"))\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// =================== [linear-composite] — R4 part 2 ===================
//
// The interim rule (user decision 2026-09-08): a linear value may not be
// **stored in a composite**. Its obligation would have to travel with the
// container, and composition plus conditional linearity are one design
// question (roadmap L8), so until that is answered a linear value lives
// only in a local, a parameter or a return value. Every way of putting one
// into a composite is refused *at the store*, which is also what keeps the
// diagnostics single: no follow-on leak for a container that was never
// built.

/// A struct field is the first thing anyone tries; the error lands at the
/// field's type and names it.
#[test]
fn a_struct_field_cannot_hold_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\nstruct Holder {{\n    handle: Lines\n}}\n"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("`Lines` is linear, so it cannot be the type of field \
                          `Holder.handle`")
            && errs[0].contains("keep it in a local, a parameter or a return value"),
        "got {errs:?}"
    );
}

/// A field of *composite* type is refused at the composite, not at the
/// field: the message names the position the value would have landed in.
#[test]
fn a_struct_field_cannot_hold_a_list_of_linear_values() {
    let errs = errors(&format!(
        "{LINES}\nstruct Holder {{\n    handles: Mut List<Lines>\n}}\n"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("`Lines` is linear, so it cannot be a type argument of `List`"),
        "got {errs:?}"
    );
}

/// A handler's state is a composite too — the same refusal, at the same
/// place in the message.
#[test]
fn handler_state_cannot_hold_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\n\
         effect Log {{\n    fn note(m: Str) -> None\n}}\n\
         handler Keeper of Log {{\n    held: Lines = open_lines(\"a\")\n\n    \
         fn note(m: Str) -> None {{}}\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains(
            "`Lines` is linear, so it cannot be the type of field `Keeper.held`"
        )),
        "got {errs:?}"
    );
}

/// Written type positions: an array element, a tuple component, a union
/// arm — and `T?`, which is a union.
#[test]
fn written_composite_positions_are_refused() {
    for (ty, position) in [
        ("Lines[]", "an array's element type"),
        ("(Lines, Int)", "a tuple component"),
        ("Lines | Int", "a union arm"),
        ("Lines?", "a union arm (`T?` is `T | None`)"),
        ("List<Lines>", "a type argument of `List`"),
    ] {
        let errs = errors(&format!(
            "{LINES}\nfn take(x: {ty}) -> [] None {{}}\n"
        ));
        assert!(
            errs.iter().any(|e| e.contains("`Lines` is linear")
                && e.contains(position)),
            "`{ty}` should be refused as {position}, got {errs:?}"
        );
    }
}

/// One store, one error: a nested composite reports at the innermost
/// position rather than once per level.
#[test]
fn a_nested_composite_reports_once() {
    let errs = errors(&format!(
        "{LINES}\nfn take(x: List<List<Lines>>) -> [] None {{}}\n"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
}

/// The inferred stores: an array literal and a tuple literal, which have
/// no written type to refuse.
#[test]
fn literal_composites_are_refused() {
    let arr = errors(&format!(
        "{LINES}\nfn go() -> [] None {{\n    let xs = [open_lines(\"a\")]\n}}\n"
    ));
    assert!(
        arr.iter()
            .any(|e| e.contains("`Lines` is linear") && e.contains("an array element")),
        "got {arr:?}"
    );
    let tup = errors(&format!(
        "{LINES}\nfn go() -> [] None {{\n    let t = (open_lines(\"a\"), 1)\n}}\n"
    ));
    assert!(
        tup.iter()
            .any(|e| e.contains("`Lines` is linear") && e.contains("a tuple component")),
        "got {tup:?}"
    );
}

/// A generic field type hides the store from the struct's declaration, so
/// the literal is where it surfaces.
#[test]
fn a_generic_struct_cannot_be_instantiated_with_a_linear_type() {
    let errs = errors(&format!(
        "{LINES}\nstruct Box<T> {{\n    item: T\n}}\n\
         fn go() -> [] None {{\n    let b = Box {{ item: open_lines(\"a\") }}\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Lines` is linear")
            && e.contains("stored in field `Box.item`")),
        "got {errs:?}"
    );
}

/// The generic *call* store: `add(list, elem)` takes a bare `T` and puts
/// it in a `List<T>`, so the call is the store — and the callee's
/// `<T canbe Linear>` does not help, since no container can carry the
/// obligation yet.
#[test]
fn storing_through_a_generic_call_is_refused() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> [] None {{\n    let xs = mutable_list()\n    \
         add(xs, open_lines(\"a\"))\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Lines` is linear")
            && e.contains("stored in the `Mut List<T>` of `add`")),
        "got {errs:?}"
    );
}

/// But a signature that only *reads* a composite of `T` stores nothing:
/// `size(list: List<T>) -> Int` stays callable from opted-in generic code,
/// which is the shape the refinement demo relies on.
#[test]
fn reading_a_composite_of_a_type_parameter_is_not_a_store() {
    let errs = errors(
        "fn count<T canbe Linear>(list: List<T>) -> [list] Int {\n    return size(list)\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// And the composite refusal is about *linear* content only: an ordinary
/// container of ordinary values is untouched.
#[test]
fn a_composite_of_plain_values_is_unaffected() {
    let errs = errors(&format!(
        "{LINES}\nstruct Holder {{\n    names: Mut List<Str>\n}}\n\
         fn go() -> [] None {{\n    let l = open_lines(\"a\")\n    \
         let h = Holder {{ names: mutable_list(\"x\") }}\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}
