define handler StdOutConsole of Console {
    define fn print(message: Str) {
        inline: ``
        kotlin.io.print(${message})
        ``
    }
}