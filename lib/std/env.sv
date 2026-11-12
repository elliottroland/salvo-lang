// The process's environment variables, as an effect.
//
// A function whose answer depends on the environment it was launched in has
// a dependency, and Salvo puts dependencies in signatures: such a function
// says `[Env]`, `main` provides the host's variables with `use HostEnv()`,
// and a test provides a handler holding the variables it wants.
//
// Not part of `std.core`, so it is imported: `import std.env`.
//
// ```
// fn home() [Env] -> Str {
//     return env("HOME") ?: "/"
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostEnv()
//     println("home is ${home()}")
// }
// ```
//
// The command line's arguments are a separate effect, `Args`, in [std.args].
// @compilerrules [cli-env, mod-import-module, cli-args]

// Reading environment variables.
export effect Env {
    // The value of the environment variable [name], or `None` when it is not
    // set. On the Rust backend a value that is not valid Unicode is `None`
    // too, since a `Str` could not hold it; the JVM decodes it.
    fn env(name: Str) -> Str? => name
}

// The host's environment, as an [Env] handler: `use HostEnv()`. It reads
// the variables with `System.getenv` on the JVM and `std::env::var` on
// Rust. It holds no state, so threads share it without synchronization.
// @compilerrules [cli-env, platform-threadsafe]
export threadsafe platform handler HostEnv for Env
