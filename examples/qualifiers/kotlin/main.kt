package salvo.main

import salvo.core.console.Console
import salvo.core.console.println
import salvo.core.list.addPlatform
import salvo.core.list.first
import salvo.core.list.getPlatform
import salvo.core.list.sizePlatform

fun<T> NonEmpty_qualifies(list: List<T>): Boolean {
    return sizePlatform(list) > 0
}

fun head(list: List<Int>): Int {
    val first = getPlatform(list, 0)
    return (first ?: throw AssertionError("salvo: value is absent at main:30:12"))
}

fun celsius(degrees: Int): Int {
    return degrees
}

fun describe__Int(temp: Int): String {
    return "$temp (no unit)"
}

fun describe__CelsiusInt(temp: Int): String {
    return "$temp°C"
}

fun sum(list: List<Int>): Int {
    var total = 0
    for (n in salvo.platform.core.list.each(list)) {
        total = total + n
    }
    return total
}

fun compact(list: salvo.platform.core.list.MutList<Int>) {
}

data class Request(
    var path: String,
    var touches: Int,
)

object __Codec_Request : salvo.WireCodec<Request> {
    override fun enc(v: Request, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
        salvo.IntCodec.enc(v.touches, out)
    }
    override fun dec(inp: salvo.WireIn): Request = Request(salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun authenticate(request: Request): Request {
    return request
}

fun freshen(request: Request): Request {
    return request
}

fun touch(request: Request) {
    request.touches = request.touches + 1
}

fun handle__Request(request: Request): String {
    return "plain ${request.path}"
}

fun handle__AuthenticatedRequest(request: Request): String {
    return "authenticated ${request.path}"
}

fun handle__FreshRequest(request: Request): String {
    return "fresh ${request.path}"
}

fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val xs: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    addPlatform(xs, 3)
    println(console, "1. head after add: ${head(xs)}")
    val maybeEmpty = listOf<Int>(7, 8)
    if (NonEmpty_qualifies(maybeEmpty)) {
        println(console, "2. checked at run time, head is ${head(maybeEmpty)}")
    }
    val plain = 21
    val warm = celsius(21)
    println(console, "2. ${describe__Int(plain)} vs ${describe__CelsiusInt(warm)}")
    if (true) {
        println(console, "2. widened: ${describe__Int(warm)}")
    }
    println(console, "3. sum ${sum(xs)}, head still ${head(xs)}")
    compact(xs)
    addPlatform(xs, 9)
    println(console, "3. after compact and add, head is ${head(xs)}")
    val session = authenticate(Request(path = "/orders", touches = 0))
    val fresh = freshen(Request(path = "/health", touches = 0))
    println(console, "4. before: ${handle__AuthenticatedRequest(session)} / ${handle__FreshRequest(fresh)}")
    touch(session)
    touch(fresh)
    println(console, "4. after:  ${handle__AuthenticatedRequest(session)} / ${handle__Request(fresh)}")
}
