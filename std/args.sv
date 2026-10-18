// [cli-args] The program's command-line arguments, as a capability.
//
// Not part of `core`, so it is imported — one line brings it in
// [mod-import-module]:
//
//     import args
//
// A function that reads argv is a function whose answer depends on how the
// process was launched, so — like reading a clock or the console — it is an
// effect, not a global. `Args` is bound once in `main` and threaded to
// whoever needs it [addr-capability]; nothing reads argv behind the back of
// the signature.
//
//     import args
//
//     fn main() [use] -> None {
//         use HostArgs()
//         for a in args() {
//             println(a)
//         }
//     }
//
// `args()` answers the arguments **after the program name** — the one form
// both backends produce identically. On the JVM the entry point's array
// already excludes argv[0]; on Rust `std::env::args()` includes it and the
// host handler drops it. A tool that wants the program name reads it from
// neither here: it is not an argument.
export effect Args {
    // The command-line arguments the process was launched with, program name
    // excluded, in order.
    fn args() -> List<Str>
}

// [cli-args] The host's argv, read from the process the one way each backend
// offers: the JVM `main`'s array (captured as the program starts), and
// Rust's `std::env::args`. Stateless, so sharing it across threads needs no
// synchronization [threadsafe-platform].
export threadsafe platform handler HostArgs() of Args
