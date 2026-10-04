// [platform-tree] std's console, for `core.console`'s
// `threadsafe platform handler StdOutConsole` (ROADMAP 0.5): standard output,
// with no buffering of its own, so a line printed before a trap is seen.

use std::io::Write;

pub struct StdOutConsole;

impl StdOutConsole {
    pub fn new() -> Self {
        StdOutConsole
    }
}

impl crate::core_console::ConsolePlatformSync for StdOutConsole {
    fn print(&self, message: &String) {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(message.as_bytes());
    }
}
