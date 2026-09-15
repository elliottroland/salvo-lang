package salvo.main

import salvo.core.console.*
import salvo.core.string.*

data class Ticket(
    val id: Int,
    val seat: String,
)

fun issue(console: Console, id: Int, seat: String): Ticket {
    println(console, "1. issued #$id for $seat")
    return Ticket(id = id, seat = seat)
}

fun redeem(console: Console, ticket: Ticket) {
    println(console, "1. redeemed #${ticket.id}")
    (ticket).let {}
}

fun one_use(console: Console) {
    val ticket = issue(console, 1, "12A")
    redeem(console, ticket)
    println(console, "2. gone after one use")
}

fun describe(console: Console, ticket: Ticket) {
    println(console, "3. still holding #${ticket.id} (${ticket.seat})")
}

fun borrow_then_use(console: Console) {
    val ticket = issue(console, 2, "3C")
    describe(console, ticket)
    describe(console, ticket)
    redeem(console, ticket)
}

fun read_a_field(console: Console) {
    val ticket = issue(console, 3, "1A")
    val seat = ticket.seat
    println(console, "4. read $seat, and #${ticket.id} is still owed")
    redeem(console, ticket)
}

fun<T> hand_over(console: Console, value: T, to: (Console, T) -> Unit) {
    to(console, value)
}

fun generic_handoff(console: Console) {
    val ticket = issue(console, 4, "9B")
    hand_over(console, ticket, { console2: Console, t -> redeem(console2, t) })
}

fun main() {
    val console: Console = StdOutConsole()
    one_use(console)
    borrow_then_use(console)
    read_a_field(console)
    generic_handoff(console)
}
