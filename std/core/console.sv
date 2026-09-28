export effect Console {
    // Prints to the console without emitting a newline
    fn print(message: Str) -> None => message
}

export intrinsic handler StdOutConsole of Console

export fn println(message: Str) [Console] -> None {
    print(message)
    print("\n")
}