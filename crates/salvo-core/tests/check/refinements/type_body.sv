// [qual-refn-type] A type's body may hold refinements, for a claim its author
// is trusted with: a generic one (`for T`), or one declared in the same file.
// std's `NonEmpty` is the generic claim; a `Stack` here says what its own
// `push` does to it.

export struct Stack<T> canbe Mut {
    items: Mut List<T>

    // Pushing makes the stack non-empty.
    refn push(s: Mut Stack<T>, value: T) => s: +NonEmpty
}

export fn size<T>(s: Stack<T>) [] -> Int => s {
    return s.items.size()
}

export fn push<T>(s: Mut Stack<T>, value: T) [] -> None => s: Mut, !value {
    s.items.add(value)
}

fn needs_some<T>(s: NonEmpty Stack<T>) -> Int => s {
    return size(s)
}

fn main() [use] {
    let s = Mut Stack<Int> { items: Mut List() }
    push(s, 1)
    // `push`'s refinement established the claim.
    let _n = needs_some(s)
    // And the generic claim is tested at the stack's own `size`.
    let t = Mut Stack<Int> { items: Mut List() }
    if t is NonEmpty {
        let _m = needs_some(t)
    }
}

//--- the trust rule lets the type's file establish the generic claim
// [qual-refn-type] [deduce-reapply] `+NonEmpty` in a return type, trusted in
// the file that declares the subject type.
export struct Bag<T> canbe Mut {
    items: Mut List<T>
}

export fn size<T>(b: Bag<T>) [] -> Int => b {
    return b.items.size()
}

export fn bag_of<T>(first: T) [] -> +NonEmpty Bag<T> => !first {
    return Bag<T> { items: Mut List(first) }
}

fn needs_some<T>(b: NonEmpty Bag<T>) -> Int => b {
    return size(b)
}

fn main() [use] {
    let _n = needs_some(bag_of(1))
}

//--- another file may not establish it
// The trust is the type's file's, not every file's: the generic claim about a
// type declared elsewhere is refused as before (a `-> +Q T` return is a
// constructor of `Q` [qual-ctor-fn]).
import q.Bag

//~ error[qual-ctor-same-file:other-file]
fn bag_of<T>(first: T) [] -> +NonEmpty Bag<T> => !first {
    return Bag<T> { items: Mut List(first) }
}

// FILE: q.sv
export struct Bag<T> canbe Mut {
    items: Mut List<T>
}

export fn size<T>(b: Bag<T>) [] -> Int => b {
    return b.items.size()
}

//--- a type speaks only about its own values
export struct Box<T> canbe Mut {
    value: T

    //~ error[qual-refn-type:other-subject]
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

//--- nor about a claim over a type declared elsewhere
// `Filled` is about lists, and `q.sv` declares it: a `Box` has no say.
import q.Filled

export struct Box<T> canbe Mut {
    value: T

    //~ error[qual-refn-type:foreign-qualifier]
    refn put(b: Mut Box<T>, value: T) => b: +Filled
}

export fn put<T>(b: Mut Box<T>, value: T) [] -> None => b: Mut, !value {
    b.value = value
}

// FILE: q.sv
export qualifier Filled<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

//--- an opaque type's body holds refinements only
// `Cell` is declared all the same (the body is skipped), so nothing cascades.
platform type Cell<T> {
    //~ error[qual-refn-type:only-refns]
    fn get(c: Cell<T>) -> T
}

//--- a precondition goes on the parameter it is about
// [qual-refn-narrow] A narrower type on a parameter the refinement states
// nothing about would read as no condition at all.
export struct Text canbe Mut {
    chars: Mut Str

    //~ error[qual-refn-narrow:other-param]
    refn add(t: Mut Text, more: NonEmpty Str) => t: +NonEmpty
}

export fn size(t: Text) [] -> Int => t {
    return t.chars.size()
}

export fn add(t: Mut Text, more: Str) [] -> None => t: Mut, more {
    t.chars.append(more)
}
