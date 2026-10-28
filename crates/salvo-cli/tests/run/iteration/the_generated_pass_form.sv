// [iter-fn] the generated-pass form
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_container_combinator

struct Countdown {
    from: Int
}

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    state CountdownIter2 {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListNext<Int> => bag {
    return iter(bag.items)
}

fn total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c), ?Next<It, Int>) -> Int =>[iter] c => c {
    let sum = 0
    let p = iter(c)
    for n in p {
        sum = sum + n
    }
    return sum
}

fn main() [use] {
    use StdConsole()
    let c = Countdown { from: 3 }
    println("generated pass: ${total(c)}")
    let b = Bag { items: List(4, 5) }
    println("written iter: ${total(b)}")
    // A literal, not `List(1, 2, 3)`: a *qualified* argument type
    // (`NonEmpty List<Int>` [col-of-nonempty]) currently stops `It` binding
    // through the `?iter` position, and the sibling `next` at the nearer rung
    // then wins — an open defect, with this line as its repro (ROADMAP.md).
    println("a list: ${total(List(1, 2, 3))}")
}
//= stdout
//= generated pass: 6
//= written iter: 9
//= a list: 6

//--- the_deduction_clause_projections

struct View canbe Mut {
    items: proj List<Int>,
    at: Int
}

fn view(items: List<Int>) -> Mut View {
    return Mut View { items: items, at: 0 }
}

fn repoint(v: Mut View, other: List<Int>) -> None => v: Mut, v.items: proj(other) {
    v.items = other
    v.at = 0
}

fn either(a: List<Int>, b: List<Int>, flag: Bool) -> proj(a, b) List<Int> {
    if flag {
        return a
    }
    return b
}

fn main() [use] {
    use StdConsole()
    let a = List(1, 2)
    let b = List(3, 4, 5)
    println("${size(either(a, b, true))} ${size(either(a, b, false))}")
    let v = view(a)
    repoint(v, b)
    println("${size(v.items)} at ${v.at}")
}
//= stdout
//= 2 3
//= 3 at 0

//--- field_disjoint_access

struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

struct Pair {
    left: Mut List<Str>,
    right: Mut List<Str>
}

fn touch(list: Mut List<Str>) -> None => list: Mut {
    add(list, "t")
    return None
}

fn main() [use] {
    use StdConsole()
    // read one field, mutate another
    let p = Person { name: "ann", tags: Mut List("x") }
    let n = p.name
    add(p.tags, "y")
    println("A ${n} ${size(p.tags)}")

    // two disjoint mutable fields
    let q = Pair { left: Mut List("l"), right: Mut List("r") }
    let l = q.left
    add(q.right, "r2")
    println("C ${size(l)} ${size(q.right)}")

    // read a field, hand a disjoint field to a mutating fn
    let p2 = Person { name: "dee", tags: Mut List("x") }
    let n2 = p2.name
    touch(p2.tags)
    println("D ${n2} ${size(p2.tags)}")

    // assignment to a disjoint field
    let p3 = Mut Person { name: "eve", tags: Mut List("x") }
    let n3 = p3.name
    p3.tags = Mut List("q", "r")
    println("E ${n3} ${size(p3.tags)}")
}
//= stdout
//= A ann 2
//= C 1 2
//= D dee 2
//= E eve 2

//--- a_generic_subject_iter_fn

struct Box<T> {
    items: List<T>
}

iter fn iter<T>(b: Box<T>) -> Emitted (proj(b) T) | Finished {
    state BoxIter {
        at: Int = 0
    }
    let e = get(b.items, at)
    if e is None {
        return finished()
    }
    at = at + 1
    return emitted(e)
}

fn main() [use] {
    use StdConsole()
    let b = Box<Str> { items: List("a", "b") }
    for s in iter(b) {
        println(s)
    }
    let n = Box<Int> { items: List(1, 2, 3) }
    println("${reduce(iter(n), 0, (acc, x) -> acc + x)}")
}
//= stdout
//= a
//= b
//= 6

//--- inc_dec

fn main() [use] {
    use StdConsole()
    let i = 5
    i++
    i--
    ++i
    --i
    println("statements: ${i}")

    let a = 10
    let post = a++
    println("post: ${post} ${a}")
    let b = 10
    let pre = ++b
    println("pre: ${pre} ${b}")
    let c = 10
    let cd = c--
    println("post-dec: ${cd} ${c}")
    let d = 10
    let dd = --d
    println("pre-dec: ${dd} ${d}")
}
//= stdout
//= statements: 5
//= post: 10 11
//= pre: 11 11
//= post-dec: 10 9
//= pre-dec: 9 9

//--- the_iter_fn_form

struct Countdown {
    from: Int
}

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    state CountdownIter {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

struct Fibs {
    count: Int
}

iter fn iter(f: Fibs) -> Emitted Int | Finished {
    state FibsIter {
        a: Int = 0,
        b: Int = 1,
        made: Int = 0
    }
    if made >= f.count {
        return finished()
    }
    let now = copy(a)
    let sum = a + b
    a = copy(b)
    b = copy(sum)
    made = made + 1
    return emitted(now)
}

struct Noisy {
    limit: Int
}

iter fn iter(n: Noisy) [Console] -> Emitted Int | Finished {
    state NoisyIter {
        at: Int = 0
    }
    if at >= n.limit {
        println("  done")
        return finished()
    }
    println("  turn ${at}")
    at = at + 1
    return emitted(copy(at))
}

struct Row {
    label: Str,
    times: Int
}

fn describe(r: Row) -> Str => r {
    return "${r.label}!"
}

iter fn iter(r: Row) -> Emitted Str | Finished {
    state RowIter {
        left: Int = r.times
    }
    if left <= 0 {
        return finished()
    }
    left = left - 1
    return emitted(describe(r))
}

fn main() [use] {
    use StdConsole()

    let c = Countdown { from: 3 }
    for n in c {
        println("n ${n}")
    }
    for n in c {
        println("again ${n}")
    }

    let p = iter(c)
    let first = next(p)
    when first {
        is Emitted { println("first ${first}") }
        is Finished { println("empty") }
    }
    for n in p {
        println("rest ${n}")
    }

    println("fib total ${reduce(iter(Fibs { count: 7 }), 0, (acc: Int, n: Int) -> acc + n)}")

    let noisy = Noisy { limit: 2 }
    for v in noisy {
        println("v ${v}")
    }
    let row = Row { label: "hey", times: 2 }
    for s in row {
        println("s ${s}")
    }
}
//= stdout
//= n 3
//= n 2
//= n 1
//= again 3
//= again 2
//= again 1
//= first 3
//= rest 2
//= rest 1
//= fib total 20
//=   turn 0
//= v 1
//=   turn 1
//= v 2
//=   done
//= s hey!
//= s hey!

//--- an_optional_borrow_unwrap

struct Person { name: Str }

fn take(p: Person) -> Str => !p { return p.name }

fn main() [use] {
    use StdConsole()
    // Annotated to a plain list: the element constructor claims `NonEmpty`
    // [col-of-nonempty], and on a non-empty list `first` answers an element
    // rather than the optional this case is about.
    let people: List<Person> = List(Person { name: "ann" }, Person { name: "bob" })
    let head = first(people)
    if head is None {
        return
    }
    println("${take(copy(head))}")
    println("${head.name}")
}
//= stdout
//= ann
//= ann

//--- a_partial_move

struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

fn eat(list: Mut List<Str>) -> None => !list {
    add(list, "eaten")
    return None
}

fn main() [use] {
    use StdConsole()
    // hand one field away, keep reading the other
    let p = Person { name: "ann", tags: Mut List("x") }
    eat(p.tags)
    println("1 ${p.name}")

    // the same through a move-mode binding
    let q = Person { name: "bob", tags: Mut List("y") }
    let t = q.tags
    add(t, "z")
    println("2 ${q.name} ${size(t)}")

    // put the field back, and the whole value works again
    let u = Mut Person { name: "eve", tags: Mut List("t") }
    eat(u.tags)
    u.tags = Mut List("new", "pair")
    println("3 ${u.name} ${size(u.tags)}")
}
//= stdout
//= 1 ann
//= 2 bob 2
//= 3 eve 2
