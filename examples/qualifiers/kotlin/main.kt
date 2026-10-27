package salvo.main

import salvo.*

fun<T> NonEmpty_qualifies(list: List<T>): Boolean {
    return (salvo.std.core.list.sizePlatform(list) > 0)
}

fun head(list: List<Int>): Int {
    val first: Int? = salvo.std.core.list.getPlatform(list, 0)
    return kotlin.run {
        val __pick_1: Int? = first
        (if ((__pick_1 != null)) {
            val __picked_2: Int = __pick_1!!
            __picked_2
        } else {
            throw AssertionError("salvo: unreachable at main:30:21")
        })
    }
}

fun celsius(degrees: Int): Int {
    return degrees
}

fun describe__Int(temp: Int): String {
    return "${temp} (no unit)"
}

fun describe__CelsiusInt(temp: Int): String {
    return "${temp}°C"
}

fun sum(list: List<Int>): Int {
    var total: Int = 0
    for (n in salvo.platform.std.core.list.each(list)) {
        total = (total + n)
    }
    return total
}

fun compact(list: salvo.platform.std.core.list.MutList<Int>) {
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
    request.touches = (request.touches + 1)
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
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val xs: salvo.platform.std.core.list.MutList<Int> = mutableListOf<Int>()
    salvo.std.core.list.addPlatform(xs, 3)
    salvo.std.core.console.println(__handle_2, "1. head after add: ${salvo.main.head(xs)}")
    val maybeEmpty: List<Int> = listOf<Int>(7, 8)
    if (salvo.std.core.list.NonEmpty_qualifies(maybeEmpty)) {
        salvo.std.core.console.println(__handle_2, "2. checked at run time, head is ${salvo.main.head(maybeEmpty)}")
    }
    val plain: Int = 21
    val warm: Int = salvo.main.celsius(21)
    salvo.std.core.console.println(__handle_2, "2. ${salvo.main.describe__Int(plain)} vs ${salvo.main.describe__CelsiusInt(warm)}")
    kotlin.run {
        salvo.std.core.console.println(__handle_2, "2. widened: ${salvo.main.describe__Int(warm)}")
    }
    salvo.std.core.console.println(__handle_2, "3. sum ${salvo.main.sum(xs)}, head still ${salvo.main.head(xs)}")
    salvo.main.compact(xs)
    salvo.std.core.list.addPlatform(xs, 9)
    salvo.std.core.console.println(__handle_2, "3. after compact and add, head is ${salvo.main.head(xs)}")
    val session: Request = salvo.main.authenticate(Request(path = "/orders", touches = 0))
    val fresh: Request = salvo.main.freshen(Request(path = "/health", touches = 0))
    salvo.std.core.console.println(__handle_2, "4. before: ${salvo.main.handle__AuthenticatedRequest(session)} / ${salvo.main.handle__FreshRequest(fresh)}")
    salvo.main.touch(session)
    salvo.main.touch(fresh)
    salvo.std.core.console.println(__handle_2, "4. after:  ${salvo.main.handle__AuthenticatedRequest(session)} / ${salvo.main.handle__Request(fresh)}")
}

