// [ref-anchor] Handle parameters anchored at a container parameter —
// `ref(c)` on each — are accepted, and may be mutated in the body: the
// replacement for the removed `canbe` aliasing clause.
struct Entity canbe Mut { hp: Int }

fn fine(c: List<Mut Entity>, one: ref(c) Mut Entity, other: ref(c) Mut Entity) -> None {
    one.hp = one.hp - 1
    other.hp = other.hp + 1
    return None
}

// ===== [type-no-cycle] a struct may not contain itself =====

// [type-no-cycle] The declaration-site diagnostic the recursive-types plan
// owed (defect closed 2026-09-25): a struct that contains itself without
// indirecting is an infinitely large value. Kotlin never noticed (its fields
// are references) and Rust failed downstream with a raw E0072, so this was a
// [backend-never-wrong] hole. Through a nullable, through a union arm, and
// through an alias.
//~ error[type-no-cycle:contains-itself]
struct Node { value: Int, next: Node? }

//~ error[type-no-cycle:contains-itself]
struct UnionNode { value: Int, next: UnionNode | None }

//~ error[type-no-cycle:contains-itself]
struct Branch { left: Tree, right: Tree }
type Tree = Int | Branch

struct Wrapper<T> { value: T }
//~ error[type-no-cycle:contains-itself]
struct Looped { w: Wrapper<Looped> }

// [type-no-cycle] …and the shapes that indirect stay legal — which is what
// the diagnostic names as the remedy. A container edge is not a cycle edge,
// including when the recursion runs through *two* modules' worth of
// containers, and a generic whose parameter sits inside a container is fine
// (the precision that a name-based container test got wrong).
struct Kids { value: Int, kids: List<Kids> }

struct Dir { files: List<File> }
struct File { parent: List<Dir> }

struct Bag<T> { items: List<T> }
struct N { b: Bag<N> }

struct Row { cells: Cell[] }
struct Cell { row: Row[] }

// [copy-scalar-free] A call handing back a borrowed **copy scalar** borrows
// nothing: `proj Int` *is* `Int` on both backends, which `Ty::qualify`
// already erases from the type — so the value travels like any owned scalar.
// Before this, reading an element out of a `List<Int>` and returning it was
// refused as "a view of `list`", asking for a `copy` neither backend would
// emit. (std's total `first` is the element read handing back a projection.)
fn head(list: NonEmpty List<Int>) -> Int => list {
    return first(list)
}

// [copy-scalar-free] The same read of a *non*-scalar element stays a view:
// the exemption is the scalar's freedom, not a hole in the rule. (The return
// type's mismatch is reported beside it, as it was over the old prelude.)
fn head_str(list: NonEmpty List<Str>) -> Str => list {
    //~ error[fn-syntax:return-mismatch]
    //~ error[fate-derived-readonly:view-moved]
    return first(list)
}
