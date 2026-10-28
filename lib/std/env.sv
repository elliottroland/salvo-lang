// The process's environment variables, as a capability.
//
// Not part of `std.core`, so it is imported:
//
//     import std.env
//
//     fn main() [use] -> None {
//         use StdConsole()
//         use HostEnv()
//         println(env("HOME") ?: "no home")
//     }
//
// A function whose answer depends on the environment it was launched in has
// a dependency, and Salvo's dependencies live in signatures — the reason
// `Args` is an effect. A test binds a handler holding the
// variables it wants.
// @compilerrules [cli-env, mod-import-module, cli-args]
export effect Env {
    // The value of the environment variable [name], or `None` when it is not
    // set (or, on Rust, not valid Unicode).
    fn env(name: Str) -> Str? => name
}

// The host's environment, read with `System.getenv` /
// `std::env::var`. Stateless, so sharing it across threads needs no
// synchronization.
// @compilerrules [cli-env, platform-threadsafe]
export threadsafe platform handler HostEnv() of Env
