// [platform-tree] [cli-env] std's environment variables, for `env`'s
// `threadsafe platform handler HostEnv`.

pub struct HostEnv;

impl HostEnv {
    pub fn new() -> Self {
        HostEnv
    }
}

impl crate::std_env::EnvPlatformSync for HostEnv {
    // [cli-env] `std::env::var` fails for a variable that is not set and for
    // one that is not valid Unicode; both answer `None`, since a `Str` cannot
    // hold the second.
    fn env(&self, name: &String) -> Option<String> {
        std::env::var(name).ok()
    }
}
