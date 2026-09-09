package salvo.core.console

import salvo.core.string.*

interface Console {
    fun print(message: String)
}

class StdOutConsole : Console {
    override fun print(message: String) {
        kotlin.io.print(message)
    }
}

fun println(console: Console, message: String) {
    console.print(message)
    console.print("\n")
}
