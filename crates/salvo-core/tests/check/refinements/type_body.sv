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

//--- a named union's body holds refinements beside its fns
// [type-union-body] [qual-refn-type] The same rules as a struct's: about its
// own type, and only a claim its author is trusted with — generic, or
// declared in its file.
struct Dot {}

struct Line {}

qualifier Seen<T> for T

type Mark = Dot | Line {
    refn see(m: Mark) => m: +Seen

    fn see(m: Mark) [] -> None => m {}
}

fn needs_seen(m: Seen Mark) [] -> Int => m {
    return 1
}

fn probe(m: Mark, n: Mark) [] -> Int => m, n {
    see(m)
    let _ok = needs_seen(m)
    //~ error[call-resolve:no-overload]
    return needs_seen(n)
}

//--- a named union speaks only about its own values
struct Dot {}

struct Held<T> {
    value: T
}

type Mark<T> = Dot | Held<T> {
    //~ error[qual-refn-type:other-subject]
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

//--- nor about a claim declared elsewhere about another type
import q.Filled

struct Dot {}

struct Line {}

type Mark = Dot | Line {
    //~ error[qual-refn-type:foreign-qualifier]
    refn see(m: Mark) => m: +Filled

    fn see(m: Mark) [] -> None => m {}
}

// FILE: q.sv
export qualifier Filled<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

//--- a named union's body holds fns and refns only
// [type-union-body] Its data is in its arms. (A parse error.) The members
// after the mistake are still read, so the call below reports nothing.
struct Dot {}

struct Line {}

type Mark = Dot | Line {
    //~ error[type-union-body:only-fns]
    at: Int

    fn weight(m: Mark) [] -> Int => m {
        return 1
    }
}

fn probe(m: Mark) [] -> Int => m {
    return weight(m)
}

//--- an alias of one type has no body
// [type-union-body] A body is for a named union, an opaque type or a struct;
// `Id` is another name for `Int`. The body is skipped whole: one diagnostic.
//~ error[type-union-body:not-a-union]
type Id = Int {
    fn next(id: Id) -> Id => id {
        return id + 1
    }
}
