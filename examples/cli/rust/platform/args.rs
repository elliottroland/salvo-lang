// [platform-tree] [cli-args] std's command-line arguments, for `args`'s
// `threadsafe platform handler HostArgs`: the process's argv with the program
// name dropped, so both backends answer the same list.

pub struct HostArgs;

impl HostArgs {
    pub fn new() -> Self {
        HostArgs
    }
}

impl crate::args::ArgsPlatformSync for HostArgs {
    // [cli-args] `std::env::args()` leads with the program name; `skip(1)`
    // drops it, leaving the arguments the JVM `main`'s array already excludes.
    fn args(&self) -> Vec<String> {
        std::env::args().skip(1).collect()
    }
}
