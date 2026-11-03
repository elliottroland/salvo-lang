// [fn-variadic] mixing plain arguments with a `...spread`.

//--- mixed_tail_assembled_once
// The shape: a lone `...spread` after the leading arguments *is* the tail.
// The constructor keeps it (`=> !first, rest`), so a spread of a place stays
// usable, and the call clones it into the owned parameter [fn-variadic].
export fn main() [use] {
    use StdConsole()
    let rest = array_of(2, 3)
    // [col-of-nonempty] The list constructors are Salvo now, so this is an
    // ordinary call: the leading element, then the tail.
    //~ emits: crate::std_core_list::List__T_TArray::<i32>(1i32, rest.clone())
    let all = List(1, ...rest)
    println("${size(all)}")
}
