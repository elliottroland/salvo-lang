package salvo.main

import salvo.*

fun<T> NonEmpty_qualifies(list: List<T>): Boolean {
    return (salvo.core.list.sizePlatform(list) > 0)
}

fun head(list: List<Int>): Int {
    val first: Int? = salvo.core.list.getPlatform(list, 0)
    return run {
        val __nn_1: Int? = first
        when {
            (__nn_1 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:30:12"))
            }
            else -> {
                val __some_2: Int = __nn_1!!
                __some_2
            }
        }
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
    for (n in salvo.platform.core.list.each(list)) {
        total = (total + n)
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
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val xs: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    salvo.core.list.addPlatform(xs, 3)
    salvo.core.console.println(__handle_2, "1. head after add: ${head(xs)}")
    val maybeEmpty: List<Int> = listOf<Int>(7, 8)
    if (salvo.core.list.NonEmpty_qualifies(maybeEmpty)) {
        salvo.core.console.println(__handle_2, "2. checked at run time, head is ${head(maybeEmpty)}")
    }
    val plain: Int = 21
    val warm: Int = celsius(21)
    salvo.core.console.println(__handle_2, "2. ${describe__Int(plain)} vs ${describe__CelsiusInt(warm)}")
    run {
        salvo.core.console.println(__handle_2, "2. widened: ${describe__Int(warm)}")
    }
    salvo.core.console.println(__handle_2, "3. sum ${sum(xs)}, head still ${head(xs)}")
    compact(xs)
    salvo.core.list.addPlatform(xs, 9)
    salvo.core.console.println(__handle_2, "3. after compact and add, head is ${head(xs)}")
    val session: Request = authenticate(Request(path = "/orders", touches = 0))
    val fresh: Request = freshen(Request(path = "/health", touches = 0))
    salvo.core.console.println(__handle_2, "4. before: ${handle__AuthenticatedRequest(session)} / ${handle__FreshRequest(fresh)}")
    touch(session)
    touch(fresh)
    salvo.core.console.println(__handle_2, "4. after:  ${handle__AuthenticatedRequest(session)} / ${handle__Request(fresh)}")
}

