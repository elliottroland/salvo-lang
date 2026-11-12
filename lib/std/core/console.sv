// The program's standard streams: printing to standard output and standard
// error, and reading lines from standard input.
//
// Like every interaction with the world in Salvo, the console is an
// **effect**: a function that prints says `[Console]` in its signature, and
// its caller either says so too or provides a handler with `use`. `main`
// provides the host's streams with `use StdConsole()`; a test can provide a
// handler of its own that records what was printed.
//
// ```
// fn greet(name: Str) [Console] -> None => name {
//     println("hello, ${name}")
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     print("your name? ")
//     let name = readln() ?: "stranger"                // None at end of input
//     greet(name)
//     printerr("done\n")
// }
// ```
//
// **Newlines are explicit.** [print] and [printerr] write exactly the text
// they are given; [println] adds `"\n"`, and prints any value with a
// `to_str` as well as a `Str`. Standard output is flushed before
// [readln] waits, so a prompt printed without a newline is on the screen.
// Under `salvo test` standard input is closed, so `readln` answers `None`
// at once.
// @compilerrules [std-console]

// The console: standard output, standard error and standard input.
//
// Its members are called like functions wherever the effect is available.
// The handler for the host's streams is [StdConsole].
// @compilerrules [std-console]
export effect Console {
    // Prints to standard output without emitting a newline.
    fn print(message: Str) -> None => message
    // Prints to standard error without emitting a newline.
    fn printerr(message: Str) -> None => message
    // Reads the next line of standard input, without its line terminator
    // (`\n` or `\r\n`), or `None` at the end of the input or on a read
    // error. Whatever was printed before it is flushed first, so a prompt is
    // on the screen while the program waits.
    fn readln() -> Str?
}

// The host's standard streams, as a [Console] handler: `use StdConsole()`.
//
// It is declared `threadsafe`: the host's streams may be entered from
// several threads at once, so the handler is shared without a lock around
// each call.
// @compilerrules [std-console]
export threadsafe platform handler StdConsole for Console

// Prints to standard output, followed by a newline.
//
// This overload prints a `Str` as it is; the one below prints any other
// value through its `to_str`.
//
// ```
// println("total: ${1 + 2}")                           // total: 3
// ```
export fn println(message: Str) [Console] -> None => message {
    print(message)
    print("\n")
}

// Prints a value's text form to standard output, followed by a newline.
//
// Unlike `println(message: Str)`, this overload takes a value of any type
// and prints what its `to_str` answers, so `println(n)` says what
// `println("${n}")` does. The `to_str` is the one a call `to_str(value)`
// would reach where `println` is called: std's for a scalar or a list, or
// the one declared beside a type of your own. Pass another with
// `println(n, to_str = hex)`. A `Str` goes to the overload above, which is
// the more specific.
//
// ```
// struct Point { x: Int, y: Int }
//
// fn to_str(p: Point) -> Str {
//     return "(${p.x}, ${p.y})"
// }
//
// fn show() [Console] -> None {
//     println(42)                                      // 42
//     println(List(1, 2, 3))                           // [1, 2, 3]
//     println(Point { x: 1, y: 2 })                    // (1, 2)
// }
// ```
//
// A type with no `to_str` in scope does not print: the call is an error
// naming the `?to_str` nothing fills. A named union prints when it
// declares a `to_str` of its own; an anonymous one (`Int | Str`, or a
// `T?`) cannot have one, so print it as `println("${u}")`, which renders
// the arm it holds.
//
// @param value   the value to print
// @param to_str  found at the call site: renders `value` as text
// @compilerrules [std-console, implicit-resolve, interp-to-str]
export fn println<T>(value: T, ?to_str: (T) -> Str) [Console] -> None => value {
    println(to_str(value))
}
