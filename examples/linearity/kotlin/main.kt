package salvo.main

import salvo.core.console.Console
import salvo.core.console.println
import salvo.core.deque.addLastPlatform
import salvo.core.deque.drain
import salvo.core.deque.mutDequeOf
import salvo.core.deque.removeFirstPlatform
import salvo.core.deque.sizePlatform
import salvo.core.list.first
import salvo.core.map.next
import salvo.core.set.next
import salvo.core.string.next

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

fun issue(console: Console, id: Int, seat: String): Ticket {
    println(console, "1. issued #$id for $seat")
    return Ticket(id = id, seat = seat)
}

fun redeem(console: Console, ticket: Ticket) {
    println(console, "1. redeemed #${ticket.id}")
    (ticket).let {}
}

fun oneUse(console: Console) {
    val ticket = issue(console, 1, "12A")
    redeem(console, ticket)
    println(console, "2. gone after one use")
}

fun describe(console: Console, ticket: Ticket) {
    println(console, "3. still holding #${ticket.id} (${ticket.seat})")
}

fun borrowThenUse(console: Console) {
    val ticket = issue(console, 2, "3C")
    describe(console, ticket)
    describe(console, ticket)
    redeem(console, ticket)
}

fun readAField(console: Console) {
    val ticket = issue(console, 3, "1A")
    val seat = ticket.seat
    println(console, "4. read $seat, and #${ticket.id} is still owed")
    redeem(console, ticket)
}

fun<T> handOver(console: Console, value: T, to: (Console, T) -> Unit) {
    to(console, value)
}

fun genericHandoff(console: Console) {
    val ticket = issue(console, 4, "9B")
    handOver(console, ticket, { console2: Console, t -> redeem(console2, t) })
}

fun scrap(ticket: Ticket) {
    (ticket).let {}
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun aQueueOfTickets(console: Console) {
    val queue: salvo.platform.core.deque.MutDeque<Ticket> = mutDequeOf()
    addLastPlatform(queue, issue(console, 5, "2B"))
    addLastPlatform(queue, issue(console, 6, "2C"))
    println(console, "6. queued ${sizePlatform(queue)}")
    val first = removeFirstPlatform(queue)
    when {
        first != null -> {
            redeem(console, first)
        }
        else -> {
        }
    }
    while (true) {
        var __is1 = removeFirstPlatform(queue)
        if (!(__is1 != null)) break
        val next = __is1 as Ticket
        redeem(console, next)
    }
    drain(queue, ::scrap)
    println(console, "6. queue drained")
}

fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    oneUse(console)
    borrowThenUse(console)
    readAField(console)
    genericHandoff(console)
    aQueueOfTickets(console)
}
