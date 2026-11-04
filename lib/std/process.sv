// Ending the process with an exit status, as an effect.
//
// Ending the process is something a function does to the world, so, like
// reading the command line, it appears in the signature: a function that may
// exit says `[Process]`, `main` provides the host's exit with
// `use HostProcess()`, and a test can provide a handler that records the
// status instead.
//
// Not part of `std.core`, so it is imported: `import std.process`.
//
// ```
// fn usage() [Console, Process] -> Never {
//     printerr("usage: tool FILE\n")
//     exit(2)
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostProcess()
//     usage()
// }
// ```
//
// **Nothing unwinds.** An `exit` ends the process where it is called: no
// `try` block and no actor fault handler sees it, and values that would
// otherwise have to be closed or released are not, as when the program
// stops on an error.
// @compilerrules [cli-process, mod-import-module, cli-args]

// Ending the process.
export effect Process {
    // Ends the process with status [code], after flushing standard output
    // and standard error, so text printed without a newline is not lost. It
    // does not return. Linear values still live at the call (values that
    // must be used exactly once) are not discharged, as when the program
    // stops on an error.
    // @compilerrules [type-any-never]
    fn exit(code: Int) -> Never
}

// The host's exit, as a [Process] handler: `use HostProcess()`. It flushes
// both standard streams, then calls `exitProcess` on the JVM or
// `std::process::exit` on Rust.
// @compilerrules [cli-process]
export threadsafe platform handler HostProcess() for Process
