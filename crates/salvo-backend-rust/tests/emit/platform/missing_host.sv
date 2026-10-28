// Platform handlers [platform-handler].

//--- a_missing_host_file_for_a_platform_handler_names_the_command
// [platform-handler] The phase-4 shape in miniature (FILE_SYSTEM.md §5.8):
// a host handler of an ordinary effect at the bottom, an ordinary Salvo
// handler depending on it above, and application code that names neither.
// The dependency switches the program into the fused emission, which is the
// path phase 4 will take. Verbatim the Kotlin backend's demo.
//
// [platform-handler] [platform-tree] [backend-never-wrong] A `use` of a
// platform handler with no host file is an error naming the command, not
// generated code referencing a struct nobody wrote.
effect RawClock {
    fn raw_now() [] -> Int
}

// The host implements this one, in Kotlin or Rust: bodyless here, and
// constructed at its `use` like any handler.
//~ refused[platform-tree:missing-host-file]
platform handler HostRawClock(offset: Int) of RawClock

effect Clock {
    fn stamp(label: Str) -> Str => label
}

handler DefaultClock [RawClock] of Clock {
    fn stamp(label: Str) -> Str => label {
        return "${label}@${raw_now()}"
    }
}

fn main() [use] {
    use StdConsole()
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
