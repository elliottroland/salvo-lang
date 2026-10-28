// Platform handlers [platform-handler]: what a `use` of one needs.

//--- a_missing_host_file_for_a_platform_handler_names_the_command
// [platform-handler] [platform-tree] A `use` of a platform handler with no
// host file is an error naming the command, not generated code that
// references a class nobody wrote — the same reasoning as the platform
// entry point's missing-host error [backend-never-wrong].
// [platform-handler] The phase-4 shape in miniature (FILE_SYSTEM.md §5.8),
// `PLATFORM_HANDLER_DEMO` in codegen_tests.rs.
effect RawClock {
    fn raw_now() [] -> Int
}

// The host implements this one, in Kotlin or Rust: bodyless here, and
// constructed at its `use` like any handler.
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
    //~ refused[platform-tree:missing-host-file]
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
