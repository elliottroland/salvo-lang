// Rust defines for core.console [backend-define-handler].

define handler StdOutConsole of Console {
    define fn print(message: Str) {
        inline: ``
        print!("{}", ${message})
        ``
    }
}
