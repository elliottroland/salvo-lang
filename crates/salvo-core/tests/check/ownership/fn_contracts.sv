// Fn types carry contracts (L7d).

//--- l7d_fn_type_contracts
// [fn-contract] L7d: fn types carry contracts — named parameters plus a
// standard deduction clause (`=>[f] !v` consumes, unannotated keeps
// everything). Calls through fn
// values apply the contract (consumption, double-use, interprocedural
// propagation); lambdas checked against a keeping contract may not
// consume their kept parameters; a consuming fn cannot be passed where
// a keeping one is expected (checked via the named-fn contract).
fn apply_consuming(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v => !data {
    return f(data)
}

fn apply_keeping(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] v => data {
    return f(data) + f(data)
}

fn eat(v: List<Int>) -> Int => !v {
    return 0
}

// A consuming named fn cannot fit a keeping contract.
fn consuming_where_keeping_bad() -> Int {
    let xs = List(1, 2)
    //~ error[call-resolve:no-overload]
    return apply_keeping(eat, xs)
}

// A kept lambda parameter cannot be consumed.
fn kept_param_consumed_bad() -> Int {
    let xs = List(1, 2)
    return apply_keeping((v: List<Int>) -> {
        //~ error[fn-contract:consume-kept]
        let w = eat(v)
        return w
    }, xs)
}

// caller_keeps is clean.
fn caller_keeps() -> Int {
    let xs = List(1, 2)
    let n = apply_keeping((v: List<Int>) -> { return size(v) }, xs)
    return n + size(xs)
}

//--- l7d_uses_after_a_move_through_a_contract
// [flow-use-after-move] The uses after a move through a fn value's
// contract are the IR flow pass's.
fn apply_consuming(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v => !data {
    return f(data)
}

// Double use through a consuming contract.
fn double_use_bad(f: (v: List<Int>) -> Int, data: List<Int>) -> Int =>[f] !v => !data {
    let _a = f(data)
    //~ error[flow-use-after-move:consumed]
    return f(data)
}

// The consuming contract propagates: the caller's argument dies.
fn caller_loses() -> Int {
    let xs = List(1, 2)
    let n = apply_consuming((v: List<Int>) -> { return size(v) }, xs)
    //~ error[flow-use-after-move:consumed]
    return n + size(xs)
}
