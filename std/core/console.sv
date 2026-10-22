// [console] The console: standard output, standard error and standard input.
export effect Console {
    // Prints to standard output without emitting a newline.
    fn print(message: Str) -> None => message
    // Prints to standard error without emitting a newline.
    fn printerr(message: Str) -> None => message
    // Reads the next line of standard input, without its line terminator —
    // `None` at the end of the input. Whatever was printed before it is
    // flushed first, so a prompt is on the screen while the program waits.
    fn readln() -> Str?
}

// [console] The host's standard streams.
export threadsafe platform handler StdConsole() of Console

export fn println(message: Str) [Console] -> None => message {
    print(message)
    print("\n")
}
