package salvo.main

import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.console.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.nonempty.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<T> NonEmpty_qualifies(list: List<T>): Boolean {
    return list.size > 0
}

fun head(list: List<Int>): Int {
    val first = list.getOrNull(0)
    return (first ?: throw AssertionError("salvo: value is absent at main:30:12"))
}

fun celsius(degrees: Int): Int {
    return degrees
}

fun describe(temp: Int): String {
    return "$temp (no unit)"
}

fun describe__Celsius(temp: Int): String {
    return "$temp°C"
}

fun sum(list: List<Int>): Int {
    var total = 0
    for (n in list) {
        total = total + n
    }
    return total
}

fun compact(list: MutableList<Int>) {
}

data class Request(
    var path: String,
    var touches: Int,
)

fun authenticate(request: Request): Request {
    return request
}

fun freshen(request: Request): Request {
    return request
}

fun touch(request: Request) {
    request.touches = request.touches + 1
}

fun handle(request: Request): String {
    return "plain ${request.path}"
}

fun handle__Authenticated(request: Request): String {
    return "authenticated ${request.path}"
}

fun handle__Fresh(request: Request): String {
    return "fresh ${request.path}"
}

fun main() {
    val console: Console = StdOutConsole()
    val xs: MutableList<Int> = mutableListOf<Int>()
    xs.add(3)
    println(console, "1. head after add: ${head(xs)}")
    val maybe_empty = listOf<Int>(7, 8)
    if (NonEmpty_qualifies(maybe_empty)) {
        println(console, "2. checked at run time, head is ${head(maybe_empty)}")
    }
    val plain = 21
    val warm = celsius(21)
    println(console, "2. ${describe(plain)} vs ${describe__Celsius(warm)}")
    if (true) {
        println(console, "2. widened: ${describe(warm)}")
    }
    println(console, "3. sum ${sum(xs)}, head still ${head(xs)}")
    compact(xs)
    xs.add(9)
    println(console, "3. after compact and add, head is ${head(xs)}")
    val session = authenticate(Request(path = "/orders", touches = 0))
    val fresh = freshen(Request(path = "/health", touches = 0))
    println(console, "4. before: ${handle__Authenticated(session)} / ${handle__Fresh(fresh)}")
    touch(session)
    touch(fresh)
    println(console, "4. after:  ${handle__Authenticated(session)} / ${handle(fresh)}")
}
