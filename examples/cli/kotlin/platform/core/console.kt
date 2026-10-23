// [platform-tree] [console] std's console, for `core.console`'s
// `threadsafe platform handler StdConsole` (ROADMAP 0.5): standard output,
// standard error, and standard input a line at a time.
package salvo.platform.core.console

class StdConsole : salvo.core.console.ConsolePlatform {
    override fun print(message: String) {
        kotlin.io.print(message)
    }

    override fun printerr(message: String) {
        System.err.print(message)
        System.err.flush()
    }

    override fun readln(): String? {
        // A prompt printed without a newline is still in stdout's buffer.
        System.out.flush()
        return readlnOrNull()
    }
}
