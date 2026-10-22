// [cli-env] The process's environment variables, as a capability.
//
// Not part of `core`, so it is imported [mod-import-module]:
//
//     import env
//
//     fn main() [use] -> None {
//         use StdConsole()
//         use HostEnv()
//         println(env("HOME") ?: "no home")
//     }
//
// A function whose answer depends on the environment it was launched in has
// a dependency, and Salvo's dependencies live in signatures — the reason
// `Args` is an effect [cli-args]. A test binds a handler holding the
// variables it wants.
export effect Env {
    // The value of the environment variable [name], or `None` when it is not
    // set (or, on Rust, not valid Unicode).
    fn env(name: Str) -> Str? => name
}

// [cli-env] The host's environment, read with `System.getenv` /
// `std::env::var`. Stateless, so sharing it across threads needs no
// synchronization [threadsafe-platform].
export threadsafe platform handler HostEnv() of Env
