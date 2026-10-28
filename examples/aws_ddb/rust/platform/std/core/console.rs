// [platform-tree] [std-console] std's console, for `std.core.console`'s
// `threadsafe platform handler StdConsole` (ROADMAP 0.5): standard output and
// standard error, with no buffering of their own, so a line printed before a
// trap is seen; and standard input, a line at a time.

use std::io::{BufRead, Write};

pub struct StdConsole;

impl StdConsole {
    pub fn new() -> Self {
        StdConsole
    }
}

impl crate::std_core_console::ConsolePlatformSync for StdConsole {
    fn print(&self, message: &String) {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(message.as_bytes());
    }

    fn printerr(&self, message: &String) {
        let mut err = std::io::stderr().lock();
        let _ = err.write_all(message.as_bytes());
    }

    fn readln(&self) -> Option<String> {
        // A prompt printed without a newline is still in stdout's buffer.
        let _ = std::io::stdout().lock().flush();
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => {
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Some(line)
            }
        }
    }
}
