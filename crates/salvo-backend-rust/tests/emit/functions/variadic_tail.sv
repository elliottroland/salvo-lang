// [fn-variadic] mixing plain arguments with a `...spread`.

//--- mixed_tail_assembled_once
// The list constructors consume their tail (`=> !first, !rest`, user decision
// 2026-10-30), so a spread moves its array into the call: a lone spread *is*
// the tail, passed as it is, and one among other elements is extended into the
// assembled vector by value — neither is cloned.
export fn main() [use] {
    use StdConsole()
    let rest = array_of(2, 3)
    //~ emits: crate::std_core_list::List__T_TArray::<i32>(1i32, rest)
    let all = List(1, ...rest)
    let more = array_of(5, 6)
    //~ emits: { let mut __v = Vec::new(); __v.push(4i32); __v.extend(more); __v }
    //~ !emits: more.iter().cloned()
    let both = List(3, 4, ...more)
    println("${size(all)} ${size(both)}")
}
