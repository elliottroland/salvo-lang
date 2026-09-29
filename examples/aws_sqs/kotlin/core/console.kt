package salvo.core.console

import salvo.core.string.*

interface Console {
    fun print(message: String)
}

class __Mon_Console(private val inner: Console) : Console {
    override fun print(message: String) =
        synchronized(inner) { inner.print(message) }
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
