// Reading an optional: a `?:` pick or a narrowed read borrows the `Option`,
// and the position decides whether the payload is cloned [rs-opt-borrow]
// [rs-read-mode].

//--- an_owned_optional_local_is_read_through_a_borrow
// [rs-opt-borrow] Two `?:` picks on one optional **local** are two *reads*:
// the checker allows them (reading an optional is free), so the lowering must
// not move the value the first time. The form is the one a narrowed read
// already takes — borrow the `Option`, unwrap the borrow, clone the payload
// only where an owned value is wanted (found 2026-09-23 writing
// `lib/std/test.test.sv` with the since-removed postfix `!`).
fn len_of(s: Str) -> Int => s {
    return size(s)
}

fn shout(s: Str) -> Str => !s {
    return s
}

fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    // Non-Copy payload: borrowed, unwrapped, cloned — so the second read still
    // has something to read.
    // Two reads of the same local, both of them borrowing the `Option`.
    //~ emits 2: = &maybe;
    // A read moved the local.
    //~ !emits: maybe.unwrap()
    println("once ${size(maybe ?: "")}")
    println("twice ${size(maybe ?: "")}")
    let kept: Str? = "hello"
    // [rs-read-mode] A **kept** parameter is a read: the unwrap answers the
    // `&String` straight into the position — no clone, and no second `&`.
    //~ emits: crate::len_of({
    //~ emits: let mut __picked_10 = __pick_9.as_ref().unwrap();
    //~|             __picked_10
    //~|
    println("kept ${len_of(kept ?: unreachable!())}")
    let owned: Str? = "hello"
    // …and a *consuming* one still gets a value of its own.
    //~ emits: let mut __picked_13 = __pick_12.as_ref().unwrap();
    //~|             __picked_13.clone()
    //~|
    println("owned ${shout(owned ?: unreachable!())}")
    let n: Int? = 3
    // A Copy payload needs none of it: the `Option` is Copy, so `.unwrap`
    // moves nothing.
    //~ emits: let mut __picked_16 = __pick_15.unwrap();
    println("copy ${(n ?: 0) + (n ?: 0)}")
}

//--- an_intrinsic_argument_takes_the_intrinsics_own_mode
// [rs-read-mode] An **intrinsic** argument takes its mode from the
// intrinsic's own declaration, not from the position the call sits in:
// `contains(str, needle) => str, needle` keeps both, so a `?:` pick on a local
// hands the lowering the reference it already has. `add(list: Mut List<T>,
// elem: T) => !elem` keeps the *list* and consumes the element — so the list
// is reached mutably and the element arrives owned.
fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    if contains(maybe ?: unreachable!(), "ell") { println("found") }
    // A kept parameter: the unwrap's `&String` goes straight into the template.
    //~ emits: let mut __picked_4 = __pick_3.as_ref().unwrap();
    //~|             __picked_4
    //~|         } else {
    //~|             panic!("salvo: unreachable at main:10:26")
    //~|         }
    //~|     }, &String::from("ell"))
    // A kept intrinsic argument or receiver is not cloned.
    //~ !emits: .clone().contains(
    //~ !emits: .clone().to_uppercase()
    //~ !emits: .clone().len()
    let other: Str? = "hello"
    println("upper ${to_upper(other ?: unreachable!())}")
    let xs: Mut List<Int>? = Mut List(1)
    // [rs-narrow-mut] A `Mut` parameter reaches the payload *mutably*. Rendered
    // owned (as it was until 2026-09-23) this read
    // `xs.as_ref().expect(…).clone().push(3)`, which appends to the clone and
    // prints `xs 1` where Kotlin prints `xs 2`.
    //~ emits: let mut __pick_9 = &mut xs;
    //~ emits: let mut __picked_10 = __pick_9.as_mut().unwrap();
    //~|             &mut *__picked_10
    //~|
    //~ !emits: .clone()
    add(xs ?: unreachable!(), 3)
    println("xs ${size(xs)}")
}

//--- a_narrowed_read_borrows_unless_the_position_owns
// [rs-read-mode] A **narrowed** read is the same question as a `?:` pick, one path
// over: `narrow_unwrap` renders through a borrow either way, and the
// `.clone()` is the position's to ask for.
fn len_of(s: Str) -> Int => s { return size(s) }
fn shout(s: Str) -> Str => !s { return s }

struct Box { label: Str? }

fn main() [use] {
    use StdConsole()
    let name: Str? = "hello"
    // The narrowing is consumed by its last use, so the payload is moved out
    // once; kept positions borrow it — no clone anywhere.
    //~ emits: let mut name_3 = name.unwrap();
    //~ emits: crate::std_core_string::size_platform(&name_3)
    //~ emits: crate::len_of(&name_3)
    //~ !emits: .clone()
    if name is Str {
        println("size ${size(name)}")
        println("kept ${len_of(name)}")
        // A binding never consumed borrows; the consuming parameter takes the value.
        //~ emits: let mut held = &name_3;
        let held: Str = name
        println("held ${held}")
        //~ emits: crate::shout(name_3)
        println("owned ${shout(name)}")
    }
    let b = Box { label: "boxed" }
    // A narrowed **field** read is a borrow of the field's payload.
    //~ emits: let mut __narrowed_4 = b.label.as_ref().unwrap();
    //~ emits: crate::len_of(__narrowed_4)
    if b.label is Str {
        println("field ${len_of(b.label)}")
    }
    let n: Int? = 4
    // A Copy payload is copied out of the representation.
    //~ emits: let mut n_5 = n.unwrap();
    //~ emits: i32::wrapping_add(n_5, n_5)
    if n is Int {
        println("copy ${n + n}")
    }
}
