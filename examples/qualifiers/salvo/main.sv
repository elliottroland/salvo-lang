// Qualifiers: what a claim about a value is, where the compiler gets one from,
// and the difference between a claim about *contents* and one about *where the
// handle came from*.
//
// Every qualifier erases: none of this exists in the generated Rust or Kotlin.
// What it decides is which overload is selected and which calls are legal, so
// the output below is the compiler's reasoning made observable.

// ===== 1. a state qualifier, with a predicate =====
//
// The default kind: a claim about the value's *contents*. The `qualifies` body
// is what an `is` check runs, so the claim can be established at run time as
// well as by construction.
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return size(list) > 0
    }

    // A refinement: what a function the qualifier does not own does to its
    // claim. `add` has never heard of `NonEmpty` — appending an element
    // nevertheless cannot leave a list empty, and the qualifier is the party
    // that knows it, so it says so here [qual-refn].
    refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]
}

// Only callable while the compiler still believes the list is non-empty, so
// there is no `None` in the result and no check in the body.
fn head(list: NonEmpty List<Int>) -> [list] Int {
    let first = get(list, 0)
    return first!
}

// ===== 2. where a claim comes from =====
//
// Three ways, all visible below: a constructor function asserts one, an `is`
// check establishes one at run time, and a refinement carries one through a
// call that knows nothing about it.

// A constructive qualifier has no predicate — there is nothing to test, the
// claim is made by going through the constructor [qual-ctor-fn].
qualifier Celsius of Int

fn celsius(degrees: Int) -> [] Int as Celsius {
    return degrees
}

// Overloading by qualifier: the more qualified signature is the more specific
// one, so the claim decides which of these a call reaches [fn-overload-rank].
fn describe(temp: Int) -> [temp] Str {
    return "${temp} (no unit)"
}

fn describe(temp: Celsius Int) -> [temp] Str {
    return "${temp}°C"
}

// ===== 3. what the compiler infers =====
//
// A deduction list says what a call does to its arguments, and an unwritten
// one is inferred from the body. `sum` keeps its list and mutates nothing, so
// a caller's claims survive the call; a function taking `Mut` and listing its
// survivors exhaustively is how a claim gets dropped.
fn sum(list: List<Int>) -> Int {
    let total = 0
    for n in list {
        total = total + n
    }
    return total
}

// `[list: Mut]` is exhaustive: `Mut` is the only claim listed, so every other
// one — `NonEmpty` included — is dropped at the call. That is what a function
// which may empty the list has to declare, and the *contract* is what the
// caller reads: `core.list` has no removal function yet, so this body has
// nothing to do.
fn compact(list: Mut List<Int>) -> [list: Mut] None {
}

// ===== 4. provenance: a claim about the handle, not the contents =====
//
// `provenance` says the claim is about where this value came from. That is
// what exempts it from the rule that a mutating call invalidates unlisted
// claims — a request does not stop having been authenticated because someone
// wrote to it.
struct Request canbe Mut {
    // What was asked for.
    path: Str,
    // How many times a handler has touched this request.
    touches: Int
}

provenance qualifier Authenticated of Request

// Minting a provenance claim: there is no predicate to satisfy, so the
// constructor is the only way in [qual-ctor-fn].
fn authenticate(request: Mut Request) -> [] Mut Request as Authenticated {
    return request
}

// A state claim about the same struct, for the contrast: this one *is* about
// contents, so a mutating call takes it away.
qualifier Fresh of Request

fn freshen(request: Mut Request) -> [] Mut Request as Fresh {
    return request
}

// An ordinary mutating call. It lists `Mut` as the only survivor, and knows
// nothing about either claim.
fn touch(request: Mut Request) -> [request: Mut] None {
    request.touches = request.touches + 1
}

fn handle(request: Request) -> [request] Str {
    return "plain ${request.path}"
}

fn handle(request: Authenticated Request) -> [request] Str {
    return "authenticated ${request.path}"
}

fn handle(request: Fresh Request) -> [request] Str {
    return "fresh ${request.path}"
}

fn main() [use] {
    use StdOutConsole()

    // ----- 1 & 2: establishing a claim -----

    // Nothing is claimed yet, so `head` is not callable: the checker has an
    // empty list as far as it knows.
    let xs: Mut List<Int> = mutable_list()

    // `add`'s refinement supplies the claim, without `add` mentioning it.
    add(xs, 3)
    println("1. head after add: ${head(xs)}")

    // The predicate is what an `is` check runs, which is how a list that
    // arrives from somewhere unknown earns the claim.
    let maybe_empty = list(7, 8)
    if maybe_empty is NonEmpty {
        println("2. checked at run time, head is ${head(maybe_empty)}")
    }

    // A constructor asserts one instead, and the overload set reads it.
    let plain = 21
    let warm = celsius(21)
    println("2. ${describe(plain)} vs ${describe(warm)}")

    // `^` is the dual of `is`: it reads the subject with the claim *removed*,
    // which is how the less specific overload is reached deliberately.
    if warm ^ Celsius {
        println("2. widened: ${describe(warm)}")
    }

    // ----- 3: what survives a call -----

    // `sum` keeps the list untouched, so the claim is still there afterwards.
    println("3. sum ${sum(xs)}, head still ${head(xs)}")

    // `compact` lists `Mut` as the only survivor, so `NonEmpty` is gone
    // afterwards and `head(xs)` would no longer compile. Re-establishing it is
    // an `add` away.
    compact(xs)
    add(xs, 9)
    println("3. after compact and add, head is ${head(xs)}")

    // ----- 4: state versus provenance -----

    let session = authenticate(Mut Request { path: "/orders", touches: 0 })
    let fresh = freshen(Mut Request { path: "/health", touches: 0 })

    println("4. before: ${handle(session)} / ${handle(fresh)}")

    // One mutating call, two outcomes: the provenance claim survives it, the
    // state claim does not — so the second call reaches a different overload
    // than it did a line ago.
    touch(session)
    touch(fresh)
    println("4. after:  ${handle(session)} / ${handle(fresh)}")
}
