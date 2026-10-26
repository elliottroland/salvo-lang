package salvo.main

import salvo.*

data class Ticket(
    val id: Int,
    val seat: String,
)

object __Codec_Ticket : salvo.WireCodec<Ticket> {
    override fun enc(v: Ticket, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.id, out)
        salvo.StrCodec.enc(v.seat, out)
    }
    override fun dec(inp: salvo.WireIn): Ticket = Ticket(salvo.IntCodec.dec(inp), salvo.StrCodec.dec(inp))
}

fun issue(console: salvo.std.core.console.Console, id: Int, seat: String): Ticket {
    salvo.std.core.console.println(console, "1. issued #${id} for ${seat}")
    return Ticket(id = id, seat = seat)
}

fun redeem(console: salvo.std.core.console.Console, ticket: Ticket) {
    salvo.std.core.console.println(console, "1. redeemed #${ticket.id}")
    kotlin.run { ticket; Unit }
}

fun oneUse(console: salvo.std.core.console.Console) {
    val ticket: Ticket = issue(console, 1, "12A")
    redeem(console, ticket)
    salvo.std.core.console.println(console, "2. gone after one use")
}

fun describe(console: salvo.std.core.console.Console, ticket: Ticket) {
    salvo.std.core.console.println(console, "3. still holding #${ticket.id} (${ticket.seat})")
}

fun borrowThenUse(console: salvo.std.core.console.Console) {
    val ticket: Ticket = issue(console, 2, "3C")
    describe(console, ticket)
    describe(console, ticket)
    redeem(console, ticket)
}

fun readAField(console: salvo.std.core.console.Console) {
    val ticket: Ticket = issue(console, 3, "1A")
    val seat: String = ticket.seat
    salvo.std.core.console.println(console, "4. read ${seat}, and #${ticket.id} is still owed")
    redeem(console, ticket)
}

fun<T> handOver(console: salvo.std.core.console.Console, value: T, to: (salvo.std.core.console.Console, T) -> Unit) {
    to(console, value)
}

fun genericHandoff(console: salvo.std.core.console.Console) {
    val ticket: Ticket = issue(console, 4, "9B")
    handOver(console, ticket, fun(__leff0: salvo.std.core.console.Console, t: Ticket) {
        return redeem(__leff0, t)
    })
}

fun scrap(ticket: Ticket) {
    kotlin.run { ticket; Unit }
}

fun aQueueOfTickets(console: salvo.std.core.console.Console) {
    val queue: salvo.platform.std.core.deque.MutDeque<Ticket> = salvo.std.core.deque.mutDequeOf()
    salvo.std.core.deque.addLastPlatform(queue, issue(console, 5, "2B"))
    salvo.std.core.deque.addLastPlatform(queue, issue(console, 6, "2C"))
    salvo.std.core.console.println(console, "6. queued ${salvo.std.core.deque.sizePlatform(queue)}")
    val first: Ticket? = salvo.std.core.deque.removeFirstPlatform(queue)
    when {
        (first != null) -> {
            val first_1: Ticket = first!!
            redeem(console, first_1)
        }
        (first == null) -> {
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:136:5"))
        }
    }
    while (true) {
        val __subject_2: Ticket? = salvo.std.core.deque.removeFirstPlatform(queue)
        if (!((__subject_2 != null))) {
            break
        }
        val next: Ticket = __subject_2!!
        redeem(console, next)
    }
    salvo.std.core.deque.drain(queue, ::scrap)
    salvo.std.core.console.println(console, "6. queue drained")
}

fun main() {
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    oneUse(__handle_2)
    borrowThenUse(__handle_2)
    readAField(__handle_2)
    genericHandoff(__handle_2)
    aQueueOfTickets(__handle_2)
}

