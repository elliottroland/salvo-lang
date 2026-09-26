//! [linear-container] [linear-state] Linearity in **collections** (LC-1…LC-5,
//! user decisions 2026-09-15/16): a container is linear exactly when its
//! element type is, obligations enter with `add`/`put`, leave one at a time
//! with `remove_first`/`remove`, and the container's own terminal is `drain`.
//!
//! The rules these cover, in the order the design states them:
//!
//! * **LC-1/LC-5** conditional contagion: `intrinsic type List<T canbe
//!   linear>` makes `List<Reply<Str>>` a linear type and `List<Int>` a plain
//!   one, with no use-site spelling anywhere.
//! * **LC-2** the surface: take-by-move answering `T?`, the displacing
//!   `replace`, `drain` as the terminal, `get` still closed (it aliases).
//! * **LC-3** which containers: `List` and `Map` *values* yes; `Set`,
//!   `SortedSet` and map **keys** refused, because dedup is dropping.
//! * **LC-4** handler state: a process owns its obligations, and an
//!   activation leaves every state field whole.
//!
//! The std surface here is a stub (`intrinsic` is std-only
//! [intrinsic-std-only]), mirroring the real declarations in
//! `std/core/list.sv` and `std/core/map.sv`.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "\
export intrinsic type Int
export intrinsic type Str
export intrinsic type Bool
export intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value
export intrinsic type List<T canbe linear> canbe Mut
export intrinsic fn mut_list_of<T canbe linear>(...elems: T[]) [] -> Mut List<T>
export intrinsic fn add<T canbe linear>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem
export intrinsic fn size<T canbe linear>(list: List<T>) [] -> Int => list
export intrinsic fn get<T>(list: List<T>, index: Int) [] -> (proj(list) T)? => list, index
export intrinsic fn remove_first<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut
export intrinsic fn drain<T canbe linear>(list: List<T>, each: (x: T) -> None) [] -> None
=>[each] !x => !list, each
export intrinsic type Map<K, V canbe linear> canbe Mut
export intrinsic fn mut_map_of<K, V canbe linear>(...entries: (K, V)[]) [] -> Mut Map<K, V>
export intrinsic fn put<K, V>(map: Mut Map<K, V>, key: K, value: V) [] -> None
=> map: Mut, !key, !value
export intrinsic fn replace<K, V canbe linear>(map: Mut Map<K, V>, key: K, value: V) [] -> V?
=> map: Mut, !key, !value
export intrinsic fn remove<K, V canbe linear>(map: Mut Map<K, V>, key: K) [] -> V? => map: Mut, key
export intrinsic fn drain<K, V canbe linear>(map: Map<K, V>, each: (x: V) -> None) [] -> None
=>[each] !x => !map, each
export intrinsic type Set<T> canbe Mut
export intrinsic fn mut_set_of<T>(...elems: T[]) [] -> Mut Set<T>
";

/// A linear token with a discharger, and a plain struct for the negative
/// half of every conditional rule.
const TOKEN: &str = "\
linear struct Token { id: Int }

fn spend(t: Token) [] -> None => !t {
    discard(t)
}

fn mint(id: Int) [] -> Token {
    return Token { id: id }
}
";

fn diagnostics(src: &str) -> Vec<String> {
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
        format!("{TOKEN}\n{src}"),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, parse) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = parse.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
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

// ===== LC-1 / LC-5: the container is linear when its element is =====

/// [linear-container] The whole of LC-1 in one program: a list of obligations
/// owes, and `drain` is what discharges it — each element landing in the
/// callback, which consumes it.
#[test]
fn a_list_of_obligations_owes_and_drain_discharges_it() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let queue: Mut List<Token> = mut_list_of()
    add(queue, mint(1))
    add(queue, mint(2))
    drain(queue, spend)
}
",
    );
    assert!(errs.is_empty(), "the drained list must check clean: {errs:?}");
}

/// [linear-container] …and without the terminal it is an ordinary leak, whose
/// diagnostic names **`drain`** rather than the element's own discharger:
/// the container is what owes.
#[test]
fn an_undrained_container_is_a_leak_naming_drain() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let queue: Mut List<Token> = mut_list_of()
    add(queue, mint(1))
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`queue` still owns a linear value")
            && m.contains("discharge it with `drain`")),
        "expected a leak naming the terminal: {errs:?}"
    );
}

/// [linear-container] The conditional half: the *same* container type with a
/// plain element owes nothing, and no annotation anywhere says which — the
/// element's declaration is the only source of linearity.
#[test]
fn a_plain_instantiation_of_the_same_container_owes_nothing() {
    let errs = diagnostics(
        "\
fn go() [] -> Int {
    let ns: Mut List<Int> = mut_list_of()
    add(ns, 1)
    return size(ns)
}
",
    );
    assert!(errs.is_empty(), "a `List<Int>` must stay plain: {errs:?}");
}

/// [linear-container] Nesting is structural: a list of lists of obligations
/// owes as a whole, since the judgment recurses through the opted position.
#[test]
fn nesting_carries_the_obligation_through() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let outer: Mut List<Mut List<Token>> = mut_list_of()
    add(outer, mut_list_of())
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`outer` still owns a linear value")),
        "a nested container must owe: {errs:?}"
    );
}

// ===== LC-2: how obligations enter, move through, and leave =====

/// [linear-container] Take-by-move answers `T?`, and the absence check *is*
/// the union narrow [linear-union-arm]: the `None` arm owes nothing, the
/// value arm owes and is discharged in the branch.
#[test]
fn take_by_move_hands_over_one_obligation() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let queue: Mut List<Token> = mut_list_of()
    add(queue, mint(1))
    let first = remove_first(queue)
    when first {
        is Token { spend(first) }
        is None {}
    }
    drain(queue, spend)
}
",
    );
    assert!(errs.is_empty(), "take-by-move must check clean: {errs:?}");
}

/// [linear-container] …and dropping what was taken is the ordinary leak: the
/// obligation left the container and landed in a local.
#[test]
fn a_taken_obligation_still_has_to_be_discharged() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let queue: Mut List<Token> = mut_list_of()
    add(queue, mint(1))
    let first = remove_first(queue)
    drain(queue, spend)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`first` still owns a linear value")),
        "the taken element must owe: {errs:?}"
    );
}

/// [linear-container] `get` stays closed to obligations: it answers a
/// **borrow**, so allowing it would let one value be discharged twice. The
/// refusal is the instantiation ban [linear-generics], unchanged by LC-2.
#[test]
fn get_is_still_refused_for_obligations() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let queue: Mut List<Token> = mut_list_of()
    add(queue, mint(1))
    let peek = get(queue, 0)
    drain(queue, spend)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot instantiate generic parameter")
            && m.contains("`get`")),
        "`get` must refuse a linear element: {errs:?}"
    );
}

/// [linear-container] A map's values may be obligations: `put` is closed to
/// them (it drops what it overwrites) and `replace` is the form that hands the
/// displaced value back.
#[test]
fn a_map_of_obligations_writes_with_replace() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let table: Mut Map<Int, Token> = mut_map_of()
    let displaced = replace(table, 1, mint(1))
    when displaced {
        is Token { spend(displaced) }
        is None {}
    }
    let taken = remove(table, 1)
    when taken {
        is Token { spend(taken) }
        is None {}
    }
    drain(table, spend)
}
",
    );
    assert!(errs.is_empty(), "the map surface must check clean: {errs:?}");
}

/// [linear-container] `put` is refused for a linear value, because it answers
/// nothing: what it overwrote would be dropped in silence.
#[test]
fn put_is_refused_for_obligations() {
    let errs = diagnostics(
        "\
fn go() [] -> None {
    let table: Mut Map<Int, Token> = mut_map_of()
    put(table, 1, mint(1))
    drain(table, spend)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot instantiate generic parameter")
            && m.contains("`put`")),
        "`put` must refuse a linear value: {errs:?}"
    );
}

// ===== LC-3: which containers participate =====

/// [linear-container] A set is refused, and the diagnostic says **why**:
/// insertion deduplicates, and dedup is a silent drop no API reshaping can
/// fix. The wording is the point — this is semantics, not a fence.
#[test]
fn a_set_of_obligations_is_refused_because_dedup_is_dropping() {
    let errs = diagnostics("fn go(s: Mut Set<Token>) [] -> None => !s {}\n");
    assert!(
        errs.iter().any(|m| m.contains("deduplicates")
            && m.contains("silently dropping an obligation")),
        "the set refusal must explain dedup: {errs:?}"
    );
}

/// [linear-container] And a map **key** is refused for the same reason, with
/// the values named as where obligations go.
#[test]
fn a_map_key_cannot_be_an_obligation() {
    let errs = diagnostics("fn go(m: Mut Map<Token, Int>) [] -> None => !m {}\n");
    assert!(
        errs.iter().any(|m| m.contains("keys** are compared and retained")
            && m.contains("*values* are where obligations go")),
        "the key refusal must explain itself: {errs:?}"
    );
}

/// [linear-container] Arrays and tuples stay out of the first round, so they
/// keep the general composite refusal — which now names the containers that
/// *do* hold obligations.
#[test]
fn arrays_and_tuples_are_still_refused() {
    for ty in ["Token[]", "(Token, Int)"] {
        let errs = diagnostics(&format!("fn go(x: {ty}) [] -> None => !x {{}}\n"));
        assert!(
            errs.iter().any(|m| m.contains("`Token` is linear")
                && m.contains("Mut List<Token>")),
            "`{ty}` should be refused with the container remedy: {errs:?}"
        );
    }
}

// ===== LC-4: handler state =====

/// [linear-state] The customer's shape: a handler parks obligations in state,
/// takes them out one at a time, and drains what is left — putting a fresh
/// container back, because an activation may not leave a hole.
#[test]
fn a_handler_may_park_obligations_in_state() {
    let errs = diagnostics(
        "\
effect Parking {
    fn park(t: Token) -> None => !t
    fn release() -> None
    fn shutdown() -> None
}

handler Parked() of Parking {
    waiting: Mut List<Token> = mut_list_of()

    fn park(t: Token) -> None {
        add(waiting, t)
    }

    fn release() -> None {
        let next = remove_first(waiting)
        when next {
            is Token { spend(next) }
            is None {}
        }
    }

    fn shutdown() -> None {
        drain(waiting, spend)
        waiting = mut_list_of()
    }
}
",
    );
    assert!(errs.is_empty(), "parking obligations in state must be legal: {errs:?}");
}

/// [linear-state] The rule that makes it sound: a member that moves state out
/// and returns without putting anything back would leave the process with a
/// hole a later activation would read.
#[test]
fn an_activation_must_leave_its_state_whole() {
    let errs = diagnostics(
        "\
effect Parking {
    fn shutdown() -> None
}

handler Parked() of Parking {
    waiting: Mut List<Token> = mut_list_of()

    fn shutdown() -> None {
        drain(waiting, spend)
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("the state field `waiting` was moved out of")
            && m.contains("assign it a value first")),
        "a hole in state must be refused: {errs:?}"
    );
}

/// [linear-state] A **bare** obligation in state is refused instead: it could
/// be stored and never taken out again (taking it is the hole above, and
/// nothing could be put back), so the obligation would have no reachable
/// discharge at all. The container is the remedy, and the diagnostic names it.
#[test]
fn a_bare_obligation_in_state_is_refused_naming_the_container() {
    let errs = diagnostics(
        "\
effect Parking {
    fn park(t: Token) -> None => !t
}

handler Parked() of Parking {
    held: Token = mint(0)

    fn park(t: Token) -> None {
        spend(t)
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("would hold an obligation this handler can never discharge")
            && m.contains("Mut List<Token>")),
        "a bare obligation in state must be refused: {errs:?}"
    );
}

/// [linear-state] The whole-state rule is about *obligations*, not about
/// storage in general: a `Copy` scalar field is handed over by value on every
/// read, which leaves no hole — the exemption [effect-state-store] already
/// grants, and the reason this check reads the field's type.
#[test]
fn a_scalar_state_field_is_unaffected() {
    let errs = diagnostics(
        "\
effect Counting {
    fn bump(n: Int) -> None
    fn total() -> Int
}

handler Counter() of Counting {
    sum: Int = 0

    fn bump(n: Int) -> None {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}
",
    );
    assert!(errs.is_empty(), "a scalar state field must be unaffected: {errs:?}");
}
