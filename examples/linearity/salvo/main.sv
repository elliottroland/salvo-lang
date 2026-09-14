// Linear types: values the compiler will not let you forget.
//
// A `linear struct` carries a **use obligation**. Every value of one has to
// be consumed on every path, exactly once, and the compiler names the value
// and the path when it is not. That is the whole idea; what this program is
// chosen to show is the parts you would otherwise have to discover:
//
//   1. where the obligation comes from, and what discharges it
//   2. that it *moves* — a consuming call ends your access to the value
//   3. that a keeping call borrows instead, and hands it back
//   4. that the obligation is the value's, and cannot hide in a composite
//   5. how a generic opts in to carrying one (`canbe linear`, `once`)
//
// The sibling example `throw-and-release/` shows the same machinery doing the
// job it exists for: releasing a resource on a path that leaves early.

// ===== 1. the obligation, and its discharge =====
//
// `linear` on the struct is the whole declaration. Nothing else in the
// program has to opt in.
linear struct Ticket {
    id: Int,
    seat: Str
}

fn issue(id: Int, seat: Str) [Console] -> Ticket {
    println("1. issued #${id} for ${seat}")
    return Ticket { id: id, seat: seat }
}

// A **discharger** is any function in this file that consumes a `Ticket` —
// `!ticket` in the deduction clause says so. Inside one the obligation still
// has to end, and `discard` is the terminal: it is legal *only* here, in the
// same file as the declaration, which is what keeps the escape hatch honest.
fn redeem(ticket: Ticket) [Console] -> None => !ticket {
    println("1. redeemed #${ticket.id}")
    discard(ticket)
}

// ===== 2. the obligation moves =====
//
// `redeem` consumes, so after the call the variable is gone. Reading it again
// is an error — "`ticket` was moved" — and so is redeeming it twice, which is
// the double-free this type exists to prevent.
fn one_use() [Console] -> None {
    let ticket = issue(1, "12A")
    redeem(ticket)
    // redeem(ticket)   // error: `ticket` was consumed by the call above
    println("2. gone after one use")
}

// ===== 3. a keeping call borrows it =====
//
// The deduction clause is the difference: `=> ticket` keeps, `=> !ticket`
// consumes. A keeping function can read a linear value freely, because the
// obligation never left the caller — so this one does not have to discharge
// anything, and the caller still owes it afterwards.
fn describe(ticket: Ticket) [Console] -> None => ticket {
    println("3. still holding #${ticket.id} (${ticket.seat})")
}

fn borrow_then_use() [Console] -> None {
    let ticket = issue(2, "3C")
    describe(ticket)
    describe(ticket)
    redeem(ticket)
}

// ===== 4. the obligation belongs to the value =====
//
// Reading a field is free: it neither discharges the obligation nor damages
// the value, so the ticket is still owed afterwards. What a linear value may
// *not* do is hide inside a composite — a tuple component, a union arm, a
// list element — because the obligation cannot yet be tracked through one.
// Each is refused where it is written, and the message says so:
//
//   let pair = (ticket, 1)      // `Ticket` is linear, so it cannot be a
//                              // tuple component
//   let xs = list_of(ticket)    // a linear value cannot be passed in a
//                              // variadic position
//
// So a linear value lives in a local, a parameter or a return value — which
// is enough for the resources it is for.
fn read_a_field() [Console] -> None {
    let ticket = issue(3, "1A")
    let seat = ticket.seat
    println("4. read ${seat}, and #${ticket.id} is still owed")
    redeem(ticket)
}

// ===== 5. a generic that carries the obligation =====
//
// A plain `<T>` may *not* be instantiated with a linear type: nothing in the
// body would honor the obligation, so the compiler refuses at the call. A
// generic opts in with `<T canbe linear>`, which is a promise that the body
// deals with it — and here the body's only move is into the callback.
//
// The callback is `once`: it may be called at most once, which is exactly
// what a consuming callback needs to be. Without `once` the body could call
// it twice and discharge the same obligation twice.
fn hand_over<T canbe linear>(value: T, to: once (t: T) [Console] -> None) [Console] -> None
    => !value, !to =>[to] !t {
    to(value)
}

fn generic_handoff() [Console] -> None {
    let ticket = issue(4, "9B")
    hand_over(ticket, t -> redeem(t))
}

fn main() [use] -> None {
    use StdOutConsole()
    one_use()
    borrow_then_use()
    read_a_field()
    generic_handoff()
}
