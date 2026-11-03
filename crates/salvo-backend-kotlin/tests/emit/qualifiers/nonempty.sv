// [col-nonempty] std's generic `NonEmpty`: one `qualifies`, its `size` the
// subject's own.

//--- the_generic_claim_is_tested_at_the_subjects_size
// The test binds `T` from the subject, so each call passes that type's `size`
// — the list's for a list, the string's for a string — and a `Mut Str`
// subject is dropped to a `String` for it, since `Mut Str` is a
// `StringBuilder` here.
export fn main() [use] -> None {
    use StdConsole
    let xs = List<Int>()
    //~ emits: salvo.std.core.nonempty.NonEmpty_qualifies(xs, { __a0 -> salvo.std.core.list.sizePlatform(__a0) })
    if xs is NonEmpty {
        println("list")
    }
    let s = Mut Str()
    s.append("a")
    //~ emits: salvo.std.core.nonempty.NonEmpty_qualifies(s.toString(), { __a0 -> salvo.std.core.string.sizePlatform(__a0) })
    if s is NonEmpty {
        println("str")
    }
}
