define handler StdOutConsole of Console {
    define fn print(message: Str) {
        inline: ``
        print(${message})
        ``
    }
}