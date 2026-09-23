//! [linear-group] `Linear` as a designated obligation group (roadmap R4, user
//! decisions 2026-09-08): declaring `: Linear<self>` *is* declaring how the
//! obligation is discharged, because the group's `close` must be supplied.
//!
//! Two things this settles that the `canbe linear` spelling could not: a
//! `close` is **required** of anything claiming to be linear, and `discard` no
//! longer discharges — dropping a handle is the leak the obligation exists to
//! prevent. The presence of a `close` never implies linearity either; only the
//! clause does.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic type List<T canbe linear> canbe Mut\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n\
     export intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value\n\
     export intrinsic fn mut_list_of<T canbe linear>(...elems: T[]) [] -> Mut List<T>\n\
     export intrinsic fn add<T canbe linear>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n\
     export intrinsic fn size<T canbe linear>(list: List<T>) [] -> Int => list\n\
     export intrinsic fn remove_first<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut\n\
     export intrinsic fn drain<T canbe linear>(list: List<T>, each: (x: T) -> None) [] -> None\
     =>[each] !x => !list, each\n\
     \
     export qualifier Emitted<T> of T\n\
     export struct Finished {}\n\
     export fn emitted<T canbe linear>(value: T) [] -> +Emitted T => !value {\n    return value\n}\n\
     export fn finished() [] -> Finished {\n    return Finished {}\n}\n\
     export params Yield<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n";

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
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

/// [linear-group] Two user files: `main.sv` and `other.sv`. The same-file
/// discharger rule is *about* file boundaries, so testing it needs both sides
/// of one.
fn errors_in_files(main_src: &str, other_src: &str) -> Vec<String> {
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
        main_src.to_string(),
        false,
    );
    sources.add(
        "other.sv",
        SourceSet::classify(Path::new("other.sv")).unwrap(),
        other_src.to_string(),
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
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const LINES: &str = r#"
linear struct Lines {
    name: Str
}

fn close(l: Lines) -> None => !l { discard(l) }

fn open_lines(n: Str) -> Lines => !n {
    return Lines { name: n }
}
"#;

/// The `close` implementation is where a linear value legitimately dies — the
/// one place linearity would otherwise make impossible to write.
#[test]
fn declaring_the_obligation_with_its_close_is_clean() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let l = open_lines(\"a\")\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [group-obligation] Declared without one, the error lands at the *struct*
/// and names the signature it wants.
#[test]
fn a_linear_struct_without_a_discharger_errors_at_the_struct() {
    let errs = errors(
        "linear struct Leaky {\n    fd: Int\n}\n",
    );
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("linear struct `Leaky` has no discharger")
            && errs[0].contains("nothing in this file consumes a `Leaky`"),
        "got {errs:?}"
    );
}

/// [linear-discard] The rule that changed: `discard` is no longer the escape
/// hatch for a linear value, and the diagnostic names what is.
#[test]
fn discard_cannot_drop_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let l = open_lines(\"a\")\n    discard(l)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`discard` cannot drop a linear value")
            && e.contains("discharge it with `close`")),
        "got {errs:?}"
    );
}

/// It still drops a non-linear one: that is what it is for now.
#[test]
fn discard_still_drops_a_plain_value() {
    let errs = errors("fn go() -> None {\n    discard(\"note\")\n}\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The leak diagnostic names `close`, not `discard`.
#[test]
fn a_leak_names_close_as_the_discharge() {
    let errs = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let l = open_lines(\"a\")\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("still owns a linear value") && e.contains("`close`")),
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
         fn close(p: Plain) -> None => !p { discard(p) }\n\
         fn make() -> Plain {\n    return Plain { fd: 1 }\n}\n\
         fn go() -> None {\n    let p = make()\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [canbe-optin] The old spelling is gone from *declarations*: `canbe` grants
/// a qualifier, `:` declares an obligation. The error names the new form.
#[test]
fn canbe_linear_on_a_declaration_names_the_obligation_form() {
    let errs = errors("struct Old canbe linear {\n    fd: Int\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("linearity is declared as an obligation")
            && e.contains("`linear struct`")),
        "got {errs:?}"
    );
}

/// [linear-generics] But `canbe linear` on a **type parameter** keeps its
/// spelling: permission on a parameter is not obligation on a declaration.
#[test]
fn canbe_linear_on_a_type_parameter_still_works() {
    let errs = errors(&format!(
        "{LINES}\n\
         fn hold<T canbe linear>(value: T) -> T {{\n    return value\n}}\n\
         fn go() -> None {{\n    let l = hold(open_lines(\"a\"))\n    close(l)\n}}\n"
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
/// field's type and the remedy is the marker: a concrete linear field is
/// legal exactly on a `linear struct` (user decision 2026-09-12).
#[test]
fn a_struct_field_cannot_hold_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\nstruct Holder {{\n    handle: Lines\n}}\n"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("makes the container a resource too")
            && errs[0].contains("declare `linear struct Holder`"),
        "got {errs:?}"
    );
}

/// A field of *composite* type is refused at the composite, not at the
/// field: the message names the position the value would have landed in.
#[test]
fn a_struct_field_holding_a_container_of_obligations_needs_the_marker() {
    // [linear-container] LC-3 (user decision 2026-09-15): a `List<Lines>` is
    // itself linear now, so a struct field of that type makes the *struct* a
    // resource — and contagion stays **spelled**: the marker is the remedy,
    // exactly as it is for a concrete linear field.
    let errs = errors(&format!(
        "{LINES}\nstruct Holder {{\n    handles: Mut List<Lines>\n}}\n"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("field `Holder.handles` makes the container a resource too")
            && errs[0].contains("declare `linear struct Holder`"),
        "got {errs:?}"
    );
}

/// A handler's state is a composite too — the same refusal, at the same
/// place in the message.
#[test]
fn handler_state_holds_containers_but_not_bare_obligations() {
    // [linear-state] LC-4 (user decision 2026-09-15): a process owns its
    // obligations, and the shapes the concurrency surface is for live in
    // handler state — but a **bare** obligation in a field could never be
    // taken back out (that would leave a hole, and nothing could be put
    // back), so the container is the form and the diagnostic says so.
    let errs = errors(&format!(
        "{LINES}\n\
         effect Log {{\n    fn note(m: Str) -> None => !m\n}}\n\
         handler Keeper of Log {{\n    held: Lines = open_lines(\"a\")\n\n    \
         fn note(m: Str) -> None {{}}\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e
            .contains("field `Keeper.held` would hold an obligation this handler can never discharge")
            && e.contains("Mut List<Lines>")),
        "got {errs:?}"
    );

    // A *container* field is legal, and an activation that drains it puts a
    // fresh one back — which is the whole of LC-4's discipline.
    let errs = errors(&format!(
        "{LINES}\n\
         effect Log {{\n    fn note(m: Str) -> None => !m\n}}\n\
         handler Keeper of Log {{\n    held: Mut List<Lines> = mut_list_of()\n\n    \
         fn note(m: Str) -> None {{\n        drain(held, close)\n        \
         held = mut_list_of()\n    }}\n}}\n"
    ));
    assert!(errs.is_empty(), "a drained-and-restored state field is legal: {errs:?}");
}

/// Written type positions: an array element, a tuple component, a union
/// arm — and `T?`, which is a union.
#[test]
fn written_composite_positions_are_refused() {
    for (ty, position) in [
        ("Lines[]", "an array's element type"),
        ("(Lines, Int)", "a tuple component"),
    ] {
        let errs = errors(&format!(
            "{LINES}\nfn take(x: {ty}) -> None => !x {{}}\n"
        ));
        assert!(
            errs.iter().any(|e| e.contains("`Lines` is linear")
                && e.contains(position)),
            "`{ty}` should be refused as {position}, got {errs:?}"
        );
    }
    // [linear-union-arm] Union arms are *not* composites (O-C2, user
    // decision 2026-09-12): a union value is one handle, so a written
    // linear arm is legal and the obligation is the union's — the
    // fallible-open shape. `T?` follows. The body owes: consuming `x`
    // satisfies it, so `=> !x` with an empty body leaks and the *arm*
    // legality is what these assert.
    for ty in ["Lines | Int", "Lines?"] {
        let errs = errors(&format!(
            "{LINES}\nfn take(x: {ty}) -> None => !x {{}}\n"
        ));
        assert!(
            !errs.iter().any(|e| e.contains("cannot be") && e.contains("union arm")),
            "`{ty}` should be a legal written type now, got {errs:?}"
        );
    }
}

/// One store, one error: a nested composite reports at the innermost
/// position rather than once per level.
#[test]
fn a_nested_composite_reports_once() {
    let errs = errors(&format!(
        "{LINES}\nfn take(x: List<List<Lines>>) -> None => !x {{}}\n"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
}

/// The inferred stores: an array literal and a tuple literal, which have
/// no written type to refuse.
#[test]
fn literal_composites_are_refused() {
    let arr = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let xs = [open_lines(\"a\")]\n}}\n"
    ));
    assert!(
        arr.iter()
            .any(|e| e.contains("`Lines` is linear") && e.contains("an array element")),
        "got {arr:?}"
    );
    let tup = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let t = (open_lines(\"a\"), 1)\n}}\n"
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
         fn go() -> None {{\n    let b = Box {{ item: open_lines(\"a\") }}\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Lines` is linear")
            && e.contains("stored in field `Box.item`")),
        "got {errs:?}"
    );
}

/// The generic *call* store: `add(list, elem)` takes a bare `T` and puts
/// it in a `List<T>`, so the call is the store — and the callee's
/// `<T canbe linear>` does not help, since no container can carry the
/// obligation yet.
#[test]
fn storing_into_a_container_makes_the_container_owe() {
    // [linear-container] LC-1/LC-2 (user decisions 2026-09-15/16): the store
    // is no longer the error — the *container* carries the obligation, and the
    // leak diagnostic names its terminal.
    let errs = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let xs: Mut List<Lines> = mut_list_of()\n    \
         add(xs, open_lines(\"a\"))\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`xs` still owns a linear value")
            && e.contains("discharge it with `drain`")),
        "got {errs:?}"
    );
    // …and draining it discharges every element through the callback.
    let errs = errors(&format!(
        "{LINES}\nfn go() -> None {{\n    let xs: Mut List<Lines> = mut_list_of()\n    \
         add(xs, open_lines(\"a\"))\n    drain(xs, close)\n}}\n"
    ));
    assert!(errs.is_empty(), "got {errs:?}");
}

/// But a signature that only *reads* a composite of `T` stores nothing:
/// `size(list: List<T>) -> Int` stays callable from opted-in generic code,
/// which is the shape the refinement demo relies on.
#[test]
fn reading_a_composite_of_a_type_parameter_is_not_a_store() {
    let errs = errors(
        "fn count<T canbe linear>(list: List<T>) -> Int => list {\n    return size(list)\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// And the composite refusal is about *linear* content only: an ordinary
/// container of ordinary values is untouched.
#[test]
fn a_composite_of_plain_values_is_unaffected() {
    let errs = errors(&format!(
        "{LINES}\nstruct Holder {{\n    names: Mut List<Str>\n}}\n\
         fn go() -> None {{\n    let l = open_lines(\"a\")\n    \
         let h = Holder {{ names: mut_list_of(\"x\") }}\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// ===== [iter-generic-drive] the release a generic drive owes =====

/// The protocol and a linear pass, for the drive tests below: a `Handle` that
/// declares both obligations, plus the `close` that discharges its linearity.
const HANDLE: &str = "\
linear struct Handle : Yield<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) [] -> Emitted Int | Finished => h: Mut {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) [] -> None => !h { discard(h) }
";

/// [linear-generics] [linear-group] A generic pass the fn **owns** —
/// `<It canbe linear>`, `=> !it` — may be carrying a resource, and the loop
/// is *not* a discharge site (user decision 2026-09-12): driving it leaves
/// the obligation live, so a body that ends without terminating it leaks.
#[test]
fn owning_a_possibly_linear_pass_still_owes_after_the_loop() {
    let errs = errors(&format!(
        "{HANDLE}\n\
         fn drain<It canbe linear>(it: Mut It, ?Yield<It, Int>) [] -> Int => !it {{\n\
         \x20   let sum = 0\n\
         \x20   for n in it {{\n\
         \x20       sum = sum + n\n\
         \x20   }}\n\
         \x20   return sum\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("still owns a linear value")
                || e.contains("cannot return while `it` still owns a linear value")),
        "expected the obligation to survive the loop, got {errs:?}"
    );
}

/// The consuming-callback pattern replaces the `?Linear<It>` spread (user
/// decision 2026-09-12): the fn takes `end: (x: It) -> None` with a
/// consuming contract, drives the pass in place, and hands it to `end` —
/// an explicit discharge on the one path out.
#[test]
fn a_consuming_callback_supplies_the_release() {
    let errs = errors(&format!(
        "{HANDLE}\n\
         fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) [] -> Int => !it =>[end] !x {{\n\
         \x20   let sum = 0\n\
         \x20   for n in it {{\n\
         \x20       sum = sum + n\n\
         \x20   }}\n\
         \x20   end(it)\n\
         \x20   return sum\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [iter-drive-in-place] A pass the fn **keeps** needs nothing: the obligation
/// stayed with the caller, the loop advances the pass where it lives, and
/// closing it here would be the caller's use-after-close.
#[test]
fn keeping_the_pass_leaves_the_release_to_the_caller() {
    let errs = errors(&format!(
        "{HANDLE}\n\
         fn peek<It canbe linear>(it: Mut It, ?Yield<It, Int>) [] -> Int => it: Mut {{\n\
         \x20   let sum = 0\n\
         \x20   for n in it {{\n\
         \x20       sum = sum + n\n\
         \x20   }}\n\
         \x20   return sum\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// And a pass that is *not* opted in needs no release either: a non-linear pass
/// may be abandoned, which is the same latitude a hand-written `while` has
/// [linear-group].
#[test]
fn a_plain_generic_pass_needs_no_release() {
    let errs = errors(&format!(
        "{HANDLE}\n\
         fn drain<It>(it: Mut It, ?Yield<It, Int>) [] -> Int => !it {{\n\
         \x20   let sum = 0\n\
         \x20   for n in it {{\n\
         \x20       sum = sum + n\n\
         \x20   }}\n\
         \x20   return sum\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// =================== [linear-union-arm] — O-C2 ===================

/// The fallible-open shape (user decision 2026-09-12): a written linear
/// union arm is legal, the un-narrowed union owes, and narrowing decides —
/// the `Ok` branch discharges with `close`, the `Err` branch owes nothing
/// (an `Err Str` never held the handle).
const FALLIBLE: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T canbe linear>(value: T) -> +Ok T => !value {
    return value
}

fn err<T canbe linear>(value: T) -> +Err T => !value {
    return value
}

fn open(flag: Bool, n: Str) -> Ok Lines | Err Str {
    if flag {
        return ok(open_lines(n))
    }
    return err("empty")
}
"#;

#[test]
fn a_linear_union_arm_settles_by_narrowing() {
    let errs = errors(&format!(
        "{LINES}{FALLIBLE}\n\
         fn happy(flag: Bool, n: Str) -> None {{\n    \
         let r = open(flag, n)\n    \
         when r {{\n        \
         is Ok {{ close(r) }}\n        \
         is Err {{ }}\n    \
         }}\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn an_unclosed_linear_arm_still_leaks() {
    let errs = errors(&format!(
        "{LINES}{FALLIBLE}\n\
         fn leak(flag: Bool, n: Str) -> None {{\n    \
         let r = open(flag, n)\n    \
         when r {{\n        \
         is Ok {{ }}\n        \
         is Err {{ }}\n    \
         }}\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("linear")),
        "expected a leak error, got {errs:?}"
    );
}

/// `T?` falls out of the union rule: `None` owes nothing, so the untaken
/// branch of `if h is Lines s { close(s) }` is settled by the else-narrow.
#[test]
fn an_optional_linear_handle_owes_nothing_on_none() {
    let errs = errors(&format!(
        "{LINES}\n\
         fn find(flag: Bool, n: Str) -> Lines? {{\n    \
         if flag {{\n        return open_lines(n)\n    }}\n    \
         return None\n}}\n\
         fn maybe(flag: Bool, n: Str) -> None {{\n    \
         let h = find(flag, n)\n    \
         if h is Lines s {{\n        close(s)\n    }}\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The un-narrowed union still owes: binding it and doing nothing leaks.
#[test]
fn an_unnarrowed_linear_union_leaks() {
    let errs = errors(&format!(
        "{LINES}{FALLIBLE}\n\
         fn leak(flag: Bool, n: Str) -> None {{\n    let r = open(flag, n)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("still owns a linear value")),
        "expected a leak error, got {errs:?}"
    );
}

// =================== the same-file discharge model ===================

/// [linear-discard] `drop` (std's uniform consuming callback) deliberately
/// lacks `canbe linear`: handing it a linear value is refused by the
/// instantiation ban — `drop` never discharges an obligation.
#[test]
fn drop_refuses_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\nfn drop<T>(value: T) -> None => !value {{}}\n\
         fn go() -> None {{\n    let l = open_lines(\"a\")\n    drop(l)\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("cannot instantiate generic parameter")
                && e.contains("does not honor the use obligation")),
        "got {errs:?}"
    );
}

/// [linear-group] A discharger may *forward* instead of discarding: the
/// obligation terminates by moving into another same-file consumer.
#[test]
fn a_discharger_may_forward_to_another() {
    let errs = errors(&format!(
        "{LINES}\nfn shutdown(l: Lines) -> None => !l {{\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [linear-group] A same-file consuming fn that neither discards nor
/// forwards leaks — being a discharger is not an exemption (user decision
/// 2026-09-12: the obligation must terminate on every path).
#[test]
fn a_discharger_that_terminates_nothing_leaks() {
    let errs = errors(&format!(
        "{LINES}\nfn vanish(l: Lines) -> None => !l {{\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("still owns a linear value")),
        "got {errs:?}"
    );
}

/// [linear-group] Alternative dischargers: two same-file consuming fns are
/// both legal terminals, and the leak diagnostic names them all.
#[test]
fn alternative_dischargers_are_all_named() {
    let errs = errors(
        "linear struct Thread {\n    id: Int\n}\n\
         fn stop(t: Thread) -> None => !t { discard(t) }\n\
         fn join(t: Thread) -> None => !t { discard(t) }\n\
         fn spawn() -> Thread {\n    return Thread { id: 1 }\n}\n\
         fn go() -> None {\n    let t = spawn()\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("still owns a linear value")
                && e.contains("`join`")
                && e.contains("`stop`")),
        "got {errs:?}"
    );
}

/// [linear-group] The cache shape: a discharge with a context parameter is
/// an ordinary same-file consuming fn — nothing extra to declare.
#[test]
fn a_context_carrying_discharge_works() {
    let errs = errors(
        "struct Cache canbe Mut {\n    size: Int\n}\n\
         linear struct Entry {\n    id: Int\n}\n\
         fn remove(cache: Mut Cache, e: Entry) -> None => cache: Mut, !e { discard(e) }\n\
         fn mint() -> Entry {\n    return Entry { id: 1 }\n}\n\
         fn go(cache: Mut Cache) -> None => cache: Mut {\n    \
         let e = mint()\n    remove(cache, e)\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// =================== [linear-generics] conditional containers ===================

/// The conditional-container matrix (user decision 2026-09-12):
/// `struct Box<T canbe linear>` with `T` reaching a field is linear exactly
/// when the instantiation is.
const BOX: &str = r#"
struct Box<T canbe linear> {
    item: T
}

fn unbox<T canbe linear>(box: Box<T>) -> T => !box {
    return box.item
}
"#;

#[test]
fn a_conditional_container_owes_when_instantiated_linear() {
    let errs = errors(&format!(
        "{LINES}{BOX}\n\
         fn go(n: Str) -> None {{\n    \
         let b = Box<Lines> {{ item: open_lines(n) }}\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("still owns a linear value") && e.contains("`unbox`")),
        "got {errs:?}"
    );
}

#[test]
fn a_conditional_container_forwards_through_its_discharger() {
    let errs = errors(&format!(
        "{LINES}{BOX}\n\
         fn go(n: Str) -> None {{\n    \
         let b = Box<Lines> {{ item: open_lines(n) }}\n    \
         let l = unbox(b)\n    close(l)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn a_plain_instantiation_owes_nothing() {
    let errs = errors(&format!(
        "{LINES}{BOX}\n\
         fn go() -> None {{\n    \
         let b = Box<Int> {{ item: 42 }}\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [linear-group] Decomposition settles the container: `unbox` returns the
/// linear field and owes nothing further — proven by `BOX` checking clean
/// in the tests above. The negative: a conditional container with no
/// same-file discharger errors at the struct.
#[test]
fn a_conditional_container_without_a_discharger_errors() {
    let errs = errors(&format!(
        "{LINES}\nstruct Bare<T canbe linear> {{\n    item: T\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("conditionally linear") && e.contains("no discharger")),
        "got {errs:?}"
    );
}

/// [linear-composite] A concrete linear field requires the marker, and with
/// it the container is an ordinary linear struct: stored, forwarded,
/// discharged as a unit.
#[test]
fn a_concrete_linear_field_needs_the_marker() {
    let errs = errors(&format!(
        "{LINES}\nlinear struct Session {{\n    file: Lines\n}}\n\
         fn end_session(s: Session) -> None => !s {{\n    close(s.file)\n}}\n\
         fn go(n: Str) -> None {{\n    \
         let s = Session {{ file: open_lines(n) }}\n    end_session(s)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// [linear-generics] The instantiation ban reaches effect members' own
// generics (the hole closed 2026-09-12): binding a member's `T` to a
// linear type would hand the obligation to handlers that never promised
// to honor it.
#[test]
fn an_effect_member_generic_cannot_smuggle_a_linear_value() {
    let errs = errors(&format!(
        "{LINES}\n\
         effect Sink {{\n    fn swallow<T>(x: T) -> None => !x\n}}\n\
         fn go(l: Lines) [Sink] -> None => !l {{\n    swallow(l)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("cannot instantiate generic parameter `T` of effect member `swallow`")
                && e.contains("does not declare `<T canbe linear>`")
        }),
        "got {errs:?}"
    );
}

// [fn-value-select] [linear-generics] A generic fn passed *by name*
// instantiates from the position's expected fn type (closed 2026-09-12):
// std's `drop` fills a consuming-callback position for a plain pass — and
// the instantiation ban still refuses it for a linear one.
#[test]
fn a_generic_fn_value_instantiates_from_the_position() {
    let errs = errors(&format!(
        "{HANDLE}\n\
         fn drop<T>(value: T) -> None => !value {{}}\n\
         struct Counter : Yield<self, Int> canbe Mut {{\n    at: Int\n}}\n\
         fn next(c: Mut Counter) [] -> Emitted Int | Finished => c: Mut {{\n    \
         if c.at <= 0 {{\n        return finished()\n    }}\n    \
         let v = copy(c.at)\n    c.at = c.at - 1\n    return emitted(v)\n}}\n\
         fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) [] -> Int => !it =>[end] !x {{\n    \
         let sum = 0\n    for n in it {{\n        sum = sum + n\n    }}\n    \
         end(it)\n    return sum\n}}\n\
         fn go() -> Int {{\n    \
         let c = Mut Counter {{ at: 3 }}\n    return drain(c, drop)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn a_generic_fn_value_still_refuses_a_linear_instantiation() {
    let errs = errors(&format!(
        "{HANDLE}\n\
         fn drop<T>(value: T) -> None => !value {{}}\n\
         fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) [] -> Int => !it =>[end] !x {{\n    \
         let sum = 0\n    for n in it {{\n        sum = sum + n\n    }}\n    \
         end(it)\n    return sum\n}}\n\
         fn go() -> Int {{\n    \
         let h = open_handle(3)\n    return drain(h, drop)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("cannot instantiate generic parameter `T` of `drop` with linear type")
        }),
        "got {errs:?}"
    );
}

// ===== effect members as dischargers [linear-group] =====

/// [linear-group] The amendment the bare-members decision entailed (user
/// decision 2026-09-14, FILE_SYSTEM.md §5.8): a **consuming effect member**
/// declared in the linear type's own file joins the discharge set, so a token
/// whose only `close` is an `Fs` member has a legal death — and every
/// handler's implementation of that member is a discharge context, which is
/// where `discard` terminates the obligation.
const TOKEN_EFFECT: &str = "\
export linear struct Token {
    handle: Int
}

export effect Sink {
    fn close(t: Token) -> Str => !t
    fn peek(t: Token) -> Int => t
}
";

#[test]
fn a_consuming_effect_member_is_a_discharger() {
    let errs = errors(&format!(
        "{TOKEN_EFFECT}\nhandler Direct of Sink {{\n    \
             fn close(t: Token) -> Str => !t {{\n        \
                 discard(t)\n        return \"closed\"\n    }}\n    \
             fn peek(t: Token) -> Int => t {{\n        return t.handle\n    }}\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [linear-group] The handler may live anywhere — a test double in another
/// module discharges too, because discharger status is the *member's*.
#[test]
fn a_handler_in_another_file_discharges_the_member_it_implements() {
    let errs = errors_in_files(
        TOKEN_EFFECT,
        "import main.Sink\nimport main.Token\n\n\
         handler Fake of Sink {\n    \
             fn close(t: Token) -> Str => !t {\n        \
                 discard(t)\n        return \"faked\"\n    }\n    \
             fn peek(t: Token) -> Int => t {\n        return 0\n    }\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [linear-group] [linear-discard] A **keeping** member is not a discharger:
/// its body may not `discard` what it promised back.
#[test]
fn a_keeping_member_body_may_not_discard() {
    let errs = errors(&format!(
        "{TOKEN_EFFECT}\nhandler Direct of Sink {{\n    \
             fn close(t: Token) -> Str => !t {{\n        \
                 discard(t)\n        return \"closed\"\n    }}\n    \
             fn peek(t: Token) -> Int => t {{\n        \
                 discard(t)\n        return 0\n    }}\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|m| m.contains("`discard` cannot drop a linear value (`Token`) here")),
        "got {errs:?}"
    );
}

/// [linear-group] The same-file rule still binds: a member of an effect
/// declared *elsewhere* than the type does not grant discharge, or any module
/// could dispose of another's linear values.
#[test]
fn a_member_in_another_file_than_the_type_is_no_discharger() {
    let errs = errors_in_files(
        "export linear struct Token {\n    handle: Int\n}\n\n\
         export fn close(t: Token) -> None => !t {\n    discard(t)\n}\n",
        "import main.Token\n\n\
         effect Sink {\n    fn take(t: Token) -> None => !t\n}\n\n\
         handler Direct of Sink {\n    \
             fn take(t: Token) -> None => !t {\n        discard(t)\n    }\n}\n",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("`discard` cannot drop a linear value (`Token`) here")),
        "got {errs:?}"
    );
}

/// [linear-group] An **overloaded** consuming member gives each of its bodies
/// its *own* contract: `close(InStream)` may discard an `InStream`, not an
/// `OutStream` — the fs shape exactly [effect-member-overload].
#[test]
fn each_member_overload_discharges_only_its_own_type() {
    let errs = errors(
        "linear struct InStream {\n    handle: Int\n}\n\
         linear struct OutStream {\n    handle: Int\n}\n\n\
         effect Fs {\n    \
             fn close(s: InStream) -> Str => !s\n    \
             fn close(s: OutStream) -> Str => !s\n}\n\n\
         handler MemFs of Fs {\n    \
             fn close(s: InStream) -> Str => !s {\n        \
                 let other = OutStream { handle: 9 }\n        \
                 discard(other)\n        discard(s)\n        return \"in\"\n    }\n    \
             fn close(s: OutStream) -> Str => !s {\n        \
                 discard(s)\n        return \"out\"\n    }\n}\n",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("`discard` cannot drop a linear value (`OutStream`) here")),
        "expected the cross-token discard to be refused, got {errs:?}"
    );
    assert_eq!(errs.len(), 1, "and nothing else: {errs:?}");
}

/// [linear-group] A leak still leaks, and the hint now names the *member* as
/// the terminal.
#[test]
fn a_leak_names_the_member_as_the_discharge() {
    let errs = errors(&format!(
        "{TOKEN_EFFECT}\nhandler Direct of Sink {{\n    \
             fn close(t: Token) -> Str => !t {{\n        \
                 discard(t)\n        return \"closed\"\n    }}\n    \
             fn peek(t: Token) -> Int => t {{\n        return t.handle\n    }}\n}}\n\n\
         fn leak() [Sink] -> Int {{\n    \
             let t = Token {{ handle: 1 }}\n    return peek(t)\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|m| m.contains("still owns a linear value") && m.contains("`close`")),
        "got {errs:?}"
    );
}

// ===== [linear-group] The opaque form: `linear intrinsic type` =====

/// Two std files, so the same-file discharger rule has a boundary to be
/// about: the prelude, plus a `token` module whose contents the test writes.
/// `intrinsic` is std-only [intrinsic-std-only], so an opaque linear type can
/// only be declared this way.
fn errors_with_std_module(token_src: &str, main_src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/token.sv",
        SourceSet::classify(Path::new("core/token.sv")).unwrap(),
        token_src.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        main_src.to_string(),
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
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

/// [linear-group] `linear intrinsic type` carries the obligation on a type
/// whose representation belongs to the backend (user decision 2026-09-15 —
/// `Reply<T>` is a scheduler handle, so there is nothing to make a `linear
/// struct` of). The declaration-site rule is the struct's, unchanged: the
/// declaring file must contain a discharger.
#[test]
fn a_linear_intrinsic_type_needs_a_discharger_in_its_own_file() {
    let errs = errors_with_std_module(
        "export linear intrinsic type Token<T>\n",
        "fn probe() -> Int {\n    return 1\n}\n",
    );
    assert!(
        errs.iter().any(|m| m.contains("linear intrinsic type `Token`")
            && m.contains("no discharger")),
        "expected the legal-death error at the declaration, got {errs:?}"
    );
}

/// With a consuming `intrinsic fn` beside it — the shape `Reply<T>` uses —
/// the declaration is accepted, and *values* of the type owe like any other
/// linear value: dropping one is a leak, sending it discharges it.
#[test]
fn a_linear_intrinsic_type_owes_and_its_intrinsic_discharges() {
    const TOKEN: &str = "\
export linear intrinsic type Token<T>
export intrinsic fn deliver<T>(token: Token<T>, value: T) [] -> None => !token, !value
export intrinsic fn mint_token() [] -> Token<Int>
";
    let clean = errors_with_std_module(
        TOKEN,
        "\
fn answer() -> None {
    let t = mint_token()
    return deliver(t, 1)
}
",
    );
    assert!(
        clean.is_empty(),
        "the declaration and its discharge must check clean, got {clean:?}"
    );

    let leaked = errors_with_std_module(
        TOKEN,
        "\
fn answer() -> Int {
    let t = mint_token()
    return 1
}
",
    );
    assert!(
        leaked
            .iter()
            .any(|m| m.contains("still owns a linear value") && m.contains("`deliver`")),
        "a dropped token must be a leak, named with its discharger, got {leaked:?}"
    );
}

/// The modifier belongs to a *declaration*, never to an alias: an alias is a
/// second name for a type that already decided whether it owes.
#[test]
fn an_alias_cannot_be_declared_linear() {
    let source = "export linear intrinsic type Handle<T> = Int\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("alias cannot be linear")),
        "expected the alias refusal: {diagnostics:?}"
    );
}
