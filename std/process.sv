// [cli-process] Ending the process with an exit status, as a capability.
//
// Not part of `core`, so it is imported [mod-import-module]:
//
//     import process
//
//     fn main() [use] -> None {
//         use StdConsole()
//         use HostProcess()
//         println("usage: tool FILE")
//         exit(2)
//     }
//
// Ending the process is something a function does to the world, so — like
// reading argv [cli-args] — it is an effect, bound once in `main` and
// threaded to whoever needs it. A test binds a handler that records the code
// instead.
export effect Process {
    // Ends the process with status [code], after flushing standard output
    // and standard error. It does not return: linear values still live are
    // not discharged, as at a trap [type-any-never].
    fn exit(code: Int) -> Never
}

// [cli-process] The host's exit: flush both standard streams, then
// `std::process::exit` / `exitProcess`. Nothing unwinds, so no `catch` or
// actor fault handling sees it.
export threadsafe platform handler HostProcess() of Process
