effect Console {
    // Prints to the console without emitting a newline
    fn print(message: Str) -> [message] None
}

external handler StdOutConsole of Console

fn println(message: Str) [Console] -> None {
    print(message)
    print("\n")
}