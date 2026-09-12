effect Console {
    // Prints to the console without emitting a newline
    fn print(message: Str) -> None => message
}

intrinsic handler StdOutConsole of Console

fn println(message: Str) [Console] -> None {
    print(message)
    print("\n")
}