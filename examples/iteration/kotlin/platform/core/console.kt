// [platform-tree] std's console, for `core.console`'s
// `threadsafe platform handler StdOutConsole` (ROADMAP 0.5): standard output.
package salvo.platform.core.console

class StdOutConsole : salvo.core.console.ConsolePlatform {
    override fun print(message: String) {
        kotlin.io.print(message)
    }
}
