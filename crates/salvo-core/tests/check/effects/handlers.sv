// Effects the checker refuses: a call needing a handler none is in scope for.

//--- missing_effect_handler_is_an_error
// [effect-available] [effect-fn-deps]
fn main() [use] -> None {
    //~ error[effect-available:callee-no-handler]
    println("no console handler used")
}

//--- effect_member_call_requires_handler_in_scope
// [effect-available]
fn main() [use] -> None {
    //~ error[effect-available:no-handler]
    print("no handler")
}
