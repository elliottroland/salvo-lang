//! [linear-obligation] [linear-state] The linear rules that depend on paths
//! and moves, which the IR reports since R8 piece 7 (PIPELINE.md §8): a value
//! consumed (moved, discharged, decomposed, narrowed to a non-linear arm) on
//! some paths that come back together but not on the others — at the `if`,
//! the `when` or the loop where they join (`ir::check`) — an assignment over a
//! local that still owes (`ir::check`; over a state field, `ir::flow`), and a
//! handler member that leaves a state field holding obligations moved out of
//! (`ir::flow`). The corpus was checked against the checker's own versions,
//! switched on beside the IR's, before they were removed: same diagnostics,
//! same spans, same words, except the cases marked below, where the IR is the
//! one that is right. Each case's diagnostics are pinned, the path-exit leaks
//! and uses after a move the IR reported already included.

use std::collections::BTreeMap;

use salvo_core::Program;

/// Shared by every case (each case is its own module).
const HELPERS: &str = "linear struct Token { id: Int }\n\nfn spend(t: Token) [] -> None => !t {\n    discard(t)\n}\n\nfn mint(id: Int) [] -> Token {\n    return Token { id: id }\n}\n\nfn peek(t: Token) [] -> Int {\n    return t.id\n}\n\nlinear struct Pair { a: Token, b: Token }\n\nfn close(p: Pair) [] -> None => !p {\n    spend(p.a)\n    spend(p.b)\n}\n\nfn mkpair() [] -> Pair {\n    return Pair { a: mint(1), b: mint(2) }\n}\n\nfn maybe(c: Bool) [] -> Token? {\n    if c {\n        return mint(1)\n    }\n    return None\n}\n\nfn res(c: Bool) [] -> Ok Token | Err Str {\n    if c {\n        return mint(1)\n    }\n    return \"no\"\n}\n\n";

/// `(name, source)`.
const CASES: &[(&str, &str)] = &[
    ("if_one_arm", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n    }\n}\n"),
    ("if_else_one", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n    } else {\n        let n = peek(h)\n    }\n}\n"),
    ("if_both", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n    } else {\n        spend(h)\n    }\n}\n"),
    ("elif_chain", "fn f(c: Bool, d: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n    } elif d {\n        spend(h)\n    }\n}\n"),
    ("elif_chain_full", "fn f(c: Bool, d: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n    } elif d {\n        spend(h)\n    } else {\n        spend(h)\n    }\n}\n"),
    ("arm_returns", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n        return\n    }\n    spend(h)\n}\n"),
    ("nested_if", "fn f(c: Bool, d: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        if d {\n            spend(h)\n        }\n    } else {\n        spend(h)\n    }\n}\n"),
    ("two_names", "fn f(c: Bool) [] -> None {\n    let b = mint(1)\n    let a = mint(2)\n    if c {\n        spend(a)\n        spend(b)\n    }\n}\n"),
    ("param_owned", "fn f(t: Token, c: Bool) [] -> None => !t {\n    if c {\n        spend(t)\n    }\n}\n"),
    ("param_kept", "fn f(t: Token, c: Bool) [] -> Int {\n    if c {\n        return peek(t)\n    }\n    return 0\n}\n"),
    ("opt_if_is", "fn f(c: Bool) [] -> None {\n    let m = maybe(c)\n    if m is Token t {\n        spend(t)\n    }\n}\n"),
    ("opt_if_is_drop", "fn f(c: Bool) [] -> None {\n    let m = maybe(c)\n    if m is Token t {\n        let n = peek(t)\n    }\n}\n"),
    ("res_when", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    when r {\n        is Ok { spend(r) }\n        is Err { }\n    }\n}\n"),
    ("res_when_drop", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    when r {\n        is Ok { let n = peek(r) }\n        is Err { }\n    }\n}\n"),
    ("res_when_bind", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    when r {\n        is Ok t { spend(t) }\n        is Err e { }\n    }\n}\n"),
    ("res_when_bind_drop", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    when r {\n        is Ok t { let n = peek(t) }\n        is Err e { }\n    }\n}\n"),
    ("res_when_two", "fn f(c: Bool, d: Bool) [] -> None {\n    let r = res(c)\n    let h = mint(3)\n    when r {\n        is Ok { spend(r)\n            spend(h) }\n        is Err { }\n    }\n}\n"),
    ("res_if_err", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    if r is Err e {\n        return\n    }\n    spend(r)\n}\n"),
    ("res_elvis", "fn f(c: Bool) [] -> None {\n    let m = maybe(c) ?: mint(5)\n    spend(m)\n}\n"),
    ("while_break", "fn f(c: Bool, d: Bool) [] -> None {\n    let h = mint(1)\n    while c {\n        if d {\n            spend(h)\n            break\n        }\n    }\n}\n"),
    ("while_break_all", "fn f(c: Bool, d: Bool) [] -> None {\n    let h = mint(1)\n    while c {\n        if d {\n            break\n        }\n    }\n    spend(h)\n}\n"),
    ("while_true_break", "fn f(d: Bool) [] -> None {\n    let h = mint(1)\n    while true {\n        if d {\n            spend(h)\n            break\n        }\n    }\n}\n"),
    ("for_break", "fn f(xs: List<Int>) [] -> None {\n    let h = mint(1)\n    for x in xs {\n        if x > 2 {\n            spend(h)\n            break\n        }\n    }\n}\n"),
    ("for_body_consumes", "fn f(xs: List<Int>) [] -> None {\n    let h = mint(1)\n    for x in xs {\n        spend(h)\n    }\n}\n"),
    ("loop_inner_if", "fn f(xs: List<Int>) [] -> None {\n    for x in xs {\n        let h = mint(x)\n        if x > 2 {\n            spend(h)\n        }\n    }\n}\n"),
    ("loop_inner_ok", "fn f(xs: List<Int>) [] -> None {\n    for x in xs {\n        let h = mint(x)\n        if x > 2 {\n            spend(h)\n            continue\n        }\n        spend(h)\n    }\n}\n"),
    ("pair_decompose_one", "fn f(c: Bool) [] -> None {\n    let p = mkpair()\n    if c {\n        spend(p.a)\n        spend(p.b)\n    }\n}\n"),
    ("pair_decompose_both", "fn f(c: Bool) [] -> None {\n    let p = mkpair()\n    if c {\n        spend(p.a)\n        spend(p.b)\n    } else {\n        close(p)\n    }\n}\n"),
    ("pair_half", "fn f(c: Bool) [] -> None {\n    let p = mkpair()\n    spend(p.a)\n    if c {\n        spend(p.b)\n    }\n}\n"),
    ("alias_moves", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    let a = h\n    if c {\n        spend(a)\n    }\n}\n"),
    ("alias_moves_both", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    let a = h\n    if c {\n        spend(a)\n    } else {\n        spend(h)\n    }\n}\n"),
    ("assert_arm", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    assert!(r is Ok)\n    if c {\n        spend(r)\n    }\n}\n"),
    ("bool_and", "fn f(c: Bool, d: Bool) [] -> None {\n    let h = mint(1)\n    if c && d {\n        spend(h)\n    } else {\n        spend(h)\n    }\n}\n"),
    ("expr_if", "fn f(c: Bool) [] -> Int {\n    let h = mint(1)\n    let n = if c { peek(h) } else { 0 }\n    spend(h)\n    return n\n}\n"),
    ("expr_if_consume", "fn f(c: Bool) [] -> Int {\n    let h = mint(1)\n    let n = if c {\n        spend(h)\n        1\n    } else { 0 }\n    return n\n}\n"),
    ("assign_live", "fn f() [] -> None {\n    let h = mint(1)\n    h = mint(2)\n    spend(h)\n}\n"),
    ("assign_after", "fn f() [] -> None {\n    let h = mint(1)\n    spend(h)\n    h = mint(2)\n    spend(h)\n}\n"),
    ("assign_after_leak", "fn f() [] -> None {\n    let h = mint(1)\n    spend(h)\n    h = mint(2)\n}\n"),
    ("assign_in_arm", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n        h = mint(2)\n    }\n    spend(h)\n}\n"),
    ("assign_in_arm_live", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        h = mint(2)\n    }\n    spend(h)\n}\n"),
    ("assign_after_join", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    if c {\n        spend(h)\n    }\n    h = mint(2)\n    spend(h)\n}\n"),
    ("assign_opt_none", "fn f(c: Bool) [] -> None {\n    let m = maybe(c)\n    if m is Token t {\n        spend(t)\n    }\n    m = maybe(c)\n    if m is Token u {\n        spend(u)\n    }\n}\n"),
    ("assign_opt_live", "fn f(c: Bool) [] -> None {\n    let m = maybe(c)\n    m = maybe(c)\n    if m is Token u {\n        spend(u)\n    }\n}\n"),
    ("assign_opt_to_none", "fn f(c: Bool) [] -> None {\n    let m = maybe(c)\n    if m is Token t {\n        spend(t)\n    }\n    m = None\n}\n"),
    ("assign_loop", "fn f(xs: List<Int>) [] -> None {\n    let h = mint(1)\n    for x in xs {\n        spend(h)\n        h = mint(x)\n    }\n    spend(h)\n}\n"),
    ("assign_loop_live", "fn f(xs: List<Int>) [] -> None {\n    let h = mint(1)\n    for x in xs {\n        h = mint(x)\n    }\n    spend(h)\n}\n"),
    ("assign_after_alias", "fn f() [] -> None {\n    let h = mint(1)\n    let a = h\n    spend(a)\n    h = mint(2)\n    spend(h)\n}\n"),
    ("assign_alias_unmoved", "fn f() [] -> None {\n    let h = mint(1)\n    let a = h\n    let n = peek(a)\n    h = mint(2)\n    spend(h)\n}\n"),
    ("assign_pair_decomposed", "fn f() [] -> None {\n    let p = mkpair()\n    spend(p.a)\n    spend(p.b)\n    p = mkpair()\n    close(p)\n}\n"),
    ("assign_pair_half", "fn f() [] -> None {\n    let p = mkpair()\n    spend(p.a)\n    p = mkpair()\n    close(p)\n}\n"),
    ("assign_res_err", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    if r is Err e {\n        r = res(c)\n    }\n    if r is Ok t {\n        spend(t)\n    }\n}\n"),
    ("assign_two_in_a_row", "fn f() [] -> None {\n    let h = mint(1)\n    h = mint(2)\n    h = mint(3)\n    spend(h)\n}\n"),
    ("assign_field_alias_pending", "fn f() [] -> None {\n    let p = mkpair()\n    let a = p.a\n    spend(a)\n    spend(p.b)\n    p = mkpair()\n    close(p)\n}\n"),
    ("assign_whole_alias_pending", "fn f() [] -> None {\n    let h = mint(1)\n    let a = h\n    h = mint(2)\n    spend(a)\n    spend(h)\n}\n"),
    ("state_drain", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        drain(waiting, spend)\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_drain_back", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        drain(waiting, spend)\n        waiting = Mut List()\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_drain_arm", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        if size(waiting) > 0 {\n            drain(waiting, spend)\n        }\n        let n = 1\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_drain_arm_back", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        if size(waiting) > 0 {\n            drain(waiting, spend)\n            waiting = Mut List()\n        }\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_drain_return", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        if size(waiting) > 0 {\n            drain(waiting, spend)\n            return\n        }\n        drain(waiting, spend)\n        waiting = Mut List()\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_assign_live", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        waiting = Mut List()\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_assign_arm", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        if size(waiting) > 0 {\n            drain(waiting, spend)\n        }\n        waiting = Mut List()\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_loop", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        for i in range(0, 3) {\n            drain(waiting, spend)\n            waiting = Mut List()\n        }\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_loop_hole", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        while size(waiting) > 0 {\n            drain(waiting, spend)\n            break\n        }\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_one_ok", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        drain(waiting, spend)\n        waiting = Mut List()\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("elif_opt", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    if r is Err e {\n        let n = 1\n    } elif c {\n        spend(r)\n    } else {\n        spend(r)\n    }\n}\n"),
    ("elif_opt_miss", "fn f(c: Bool) [] -> None {\n    let r = res(c)\n    if r is Err e {\n        let n = 1\n    } elif c {\n        spend(r)\n    }\n}\n"),
    ("when_three", "fn f(c: Bool) [] -> None {\n    let h = mint(1)\n    let r = res(c)\n    when r {\n        is Ok { spend(r)\n            spend(h) }\n        is Err { spend(h) }\n    }\n}\n"),
    ("state_return_value", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n    fn all() -> Mut List<Token>\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        let n = 1\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n\n    fn all() -> Mut List<Token> {\n        return waiting\n    }\n}\n"),
    ("state_in_lambda", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        let g = () -> { let n = 1 }\n        g()\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_two_returns", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    fn shutdown() -> None {\n        if size(waiting) > 3 {\n            drain(waiting, spend)\n            return\n        }\n        if size(waiting) > 1 {\n            drain(waiting, spend)\n            return\n        }\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
    ("state_init", "effect Parking {\n    fn shutdown() -> None\n    fn park(t: Token) -> None => !t\n    fn one() -> Token?\n}\n\nhandler Parked() of Parking {\n    state {\n        waiting: Mut List<Token> = Mut List()\n    }\n\n    init {\n        drain(waiting, spend)\n    }\n\n    fn shutdown() -> None {\n        let n = 1\n    }\n\n    fn park(t: Token) -> None => !t {\n        add(waiting, t)\n    }\n\n    fn one() -> Token? {\n        return remove_first(waiting)\n    }\n}\n"),
];

/// What each case reports, `(line, column, kind)` within the case: `some`
/// (consumed on some paths but not others), `drop` (an assignment drops a
/// value that still owes), `hole` (a state field left moved out), `leak` (a
/// path-exit leak), `use` (a use after a move). Where the checker said
/// something else before R8 piece 7, the case says so.
const EXPECTED: &[(&str, &[(u32, u32, &str)])] = &[
    ("if_one_arm", &[(3, 5, "some")]),
    ("if_else_one", &[(3, 5, "some")]),
    ("if_both", &[]),
    ("elif_chain", &[(3, 5, "some")]),
    ("elif_chain_full", &[]),
    ("arm_returns", &[]),
    ("nested_if", &[(4, 9, "some")]),
    ("two_names", &[(4, 5, "some"), (4, 5, "some")]),
    ("param_owned", &[(2, 5, "some")]),
    ("param_kept", &[]),
    ("opt_if_is", &[]),
    ("opt_if_is_drop", &[(3, 5, "some")]),
    ("res_when", &[]),
    ("res_when_drop", &[(3, 5, "some")]),
    ("res_when_bind", &[]),
    ("res_when_bind_drop", &[(3, 5, "some")]),
    ("res_when_two", &[(4, 5, "some")]),
    ("res_if_err", &[]),
    ("res_elvis", &[]),
    ("while_break", &[(3, 5, "some")]),
    ("while_break_all", &[]),
    // The checker reported `some` at (3, 5): it joined the loop's end state with
    // the `break`, but `while true` leaves only by its `break`.
    ("while_true_break", &[]),
    ("for_break", &[(3, 5, "some")]),
    // The leak and the use were the IR's already.
    ("for_body_consumes", &[(2, 13, "leak"), (4, 15, "use")]),
    ("loop_inner_if", &[(4, 9, "some")]),
    ("loop_inner_ok", &[]),
    // The checker reported nothing (its join did not count decomposition as
    // settling); the IR's path-exit leak caught it.
    ("pair_decompose_one", &[(3, 5, "some")]),
    // The checker reported `some` at (3, 5): decomposed on one path, consumed on
    // the other, settled on both.
    ("pair_decompose_both", &[]),
    // The checker reported nothing: after its join `p.a` and `p.b` were both
    // moved (union), so it took `p` for settled; the IR's leak caught it.
    ("pair_half", &[(4, 5, "some")]),
    // The checker reported nothing (the move-mode binding owed on the path that
    // skipped the use); the IR's leak caught it.
    ("alias_moves", &[(4, 5, "some")]),
    // The checker's linear rules reported nothing; the use was the IR's already.
    ("alias_moves_both", &[(4, 5, "some"), (7, 15, "use")]),
    ("assert_arm", &[(4, 5, "some")]),
    ("bool_and", &[]),
    ("expr_if", &[]),
    ("expr_if_consume", &[(3, 13, "some")]),
    ("assign_live", &[(3, 5, "drop")]),
    ("assign_after", &[]),
    ("assign_after_leak", &[(4, 9, "leak")]),
    ("assign_in_arm", &[]),
    ("assign_in_arm_live", &[(4, 9, "drop")]),
    ("assign_after_join", &[(3, 5, "some")]),
    ("assign_opt_none", &[]),
    ("assign_opt_live", &[(3, 5, "drop")]),
    ("assign_opt_to_none", &[]),
    ("assign_loop", &[]),
    ("assign_loop_live", &[(4, 9, "drop")]),
    ("assign_after_alias", &[]),
    // Before R8 piece 7 the IR also reported a false leak of `a`, a borrow-mode
    // alias (now: an alias owes nothing).
    ("assign_alias_unmoved", &[(5, 5, "drop")]),
    ("assign_pair_decomposed", &[]),
    ("assign_pair_half", &[(4, 5, "drop")]),
    // The checker reported `some` at (3, 5): narrowing `r` to `Err` marked it
    // settled, and the reassignment did not clear the mark.
    ("assign_res_err", &[]),
    ("assign_two_in_a_row", &[(3, 5, "drop"), (4, 5, "drop")]),
    ("assign_field_alias_pending", &[]),
    ("assign_whole_alias_pending", &[]),
    ("state_drain", &[(12, 8, "hole")]),
    ("state_drain_back", &[]),
    ("state_drain_arm", &[(12, 8, "hole")]),
    ("state_drain_arm_back", &[]),
    ("state_drain_return", &[(15, 13, "hole")]),
    // The checker reported nothing: it treated a state field as a kept
    // parameter, which never owes.
    ("state_assign_live", &[(13, 9, "drop")]),
    ("state_assign_arm", &[]),
    ("state_loop", &[]),
    ("state_loop_hole", &[(12, 8, "hole")]),
    ("state_one_ok", &[]),
    ("elif_opt", &[]),
    ("elif_opt_miss", &[(3, 5, "some")]),
    ("when_three", &[]),
    ("state_return_value", &[(26, 9, "hole")]),
    ("state_in_lambda", &[]),
    ("state_two_returns", &[(15, 13, "hole"), (19, 13, "hole")]),
    ("state_init", &[(12, 5, "hole")]),
];

fn program() -> Program {
    // [std-cache] Over the process's std front (TESTS.md T3).
    let files: Vec<(String, String)> = CASES.iter().map(|(name, body)| (format!("{name}.sv"), format!("{HELPERS}{body}"))).collect();
    let files: Vec<(&str, &str)> = files.iter().map(|(n, b)| (n.as_str(), b.as_str())).collect();
    salvo_testkit::front::clean_program("kt", &files)
}

fn kind(message: &str) -> &'static str {
    if message.contains("owns a linear value that is consumed on some paths but not others; discharge or move it on every path") {
        "some"
    } else if message.contains("drops the linear value it still owns; move it onward or `discard(") {
        "drop"
    } else if message.contains("was moved out of and nothing was put back, so this activation would leave the handler with a hole") {
        "hole"
    } else if message.contains("still owns a linear value when the path leaves") {
        "leak"
    } else if message.contains("cannot be used") {
        "use"
    } else {
        panic!("unexpected diagnostic: {message}")
    }
}

#[test]
fn the_ir_reports_the_linear_flow_rules() {
    let program = program();
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    // The checker reports none of them (nor anything else on the corpus).
    let errors: Vec<String> = checked
        .errors
        .iter()
        .filter(|d| d.is_error() && !program.files[d.file].is_std)
        .map(|d| d.render(&program.files))
        .collect();
    assert!(errors.is_empty(), "the checker reports errors in the corpus:\n{}", errors.join("\n"));
    let helper_lines = HELPERS.lines().count() as u32;
    let mut got: BTreeMap<String, Vec<(u32, u32, &'static str)>> = BTreeMap::new();
    for d in salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked) {
        let file = &program.files[d.file];
        let (line, col) = salvo_syntax::span::line_col(&file.content, d.span.start);
        let name = file.name.trim_end_matches(".sv").to_string();
        got.entry(name).or_default().push((line - helper_lines, col, kind(&d.message)));
    }
    for v in got.values_mut() {
        v.sort();
    }
    if std::env::var_os("SALVO_PRINT_EXPECTED").is_some() {
        for (name, _) in CASES {
            let v = got.get(*name).cloned().unwrap_or_default();
            let items: Vec<String> = v.iter().map(|(l, c, k)| format!("({l}, {c}, \"{k}\")")).collect();
            println!("    (\"{name}\", &[{}]),", items.join(", "));
        }
    }
    assert_eq!(EXPECTED.len(), CASES.len(), "every case is pinned");
    let mut wrong = Vec::new();
    for ((name, _), (ename, expected)) in CASES.iter().zip(EXPECTED) {
        assert_eq!(name, ename, "EXPECTED follows CASES");
        let actual = got.get(*name).cloned().unwrap_or_default();
        if actual != expected.to_vec() {
            wrong.push(format!("{name}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
