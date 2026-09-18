package salvo.core.console

import salvo.*
import salvo.core.string.*

interface Console {
    fun print(message: String)
}

class StdOutConsole : Console {
    override fun print(message: String) {
        kotlin.io.print(message)
    }
}

fun<__Fx> println(__fx: __Fx, message: String) where __Fx : __Has_Console {
    __fx.__fx_Console.print(message)
    __fx.__fx_Console.print("\n")
}
