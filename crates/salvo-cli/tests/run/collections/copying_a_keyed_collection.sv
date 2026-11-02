// [copy-plan] [kt-copy] `copy` of a keyed collection — `Map`, `Set`,
// `SortedMap`, `SortedSet`, plain or `Mut` — is a fresh collection of the
// same entries, each value copied by its own plan (ROADMAP 0m: the Kotlin
// backend had no plan for any of them, and refused the program).
//
// Compile-and-run data ([test-run-data]).

//--- a_copy_is_its_own_collection
fn main() [use] {
    use StdConsole()
    let m = Map(("a", 1), ("b", 2))
    let c = copy(m)
    println("${size(c)} ${size(m)}")
    let mm = Mut Map(("a", 1))
    let cc = copy(mm)
    put(cc, "b", 2)
    println("${size(mm)} ${size(cc)}")
    let s = Mut Set("x")
    let t = copy(s)
    add(t, "y")
    println("${size(s)} ${size(t)}")
    let ss = Mut SortedSet(3, 1)
    let st = copy(ss)
    add(st, 2)
    println("${size(ss)} ${size(st)}")
    let sm = Mut SortedMap(("k", 1))
    let sc = copy(sm)
    put(sc, "j", 2)
    println("${size(sm)} ${size(sc)}")
    let lists = Mut Map(("a", Mut List(1)))
    let lc = copy(lists)
    println("${size(lc)} ${size(lists)}")
    let taken = remove(lc, "a")
    if taken is Mut List<Int> l {
        add(l, 2)
        println("${size(l)}")
    }
    let kept = remove(lists, "a")
    if kept is Mut List<Int> l {
        println("${size(l)}")
    }
}
//= stdout
//= 2 2
//= 1 2
//= 1 2
//= 2 3
//= 1 2
//= 1 1
//= 2
//= 1
