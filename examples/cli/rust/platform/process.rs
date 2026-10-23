// [platform-tree] [cli-process] std's process exit, for `process`'s
// `threadsafe platform handler HostProcess`: flush both standard streams, then
// end the process with the status asked for.

use std::io::Write;

pub struct HostProcess;

impl HostProcess {
    pub fn new() -> Self {
        HostProcess
    }
}

impl crate::process::ProcessPlatformSync for HostProcess {
    // [cli-process] stdout is line-buffered, so text printed without a
    // newline is still in its buffer; `std::process::exit` runs no
    // destructors, so it is flushed here.
    fn exit(&self, code: i32) -> ! {
        let _ = std::io::stdout().lock().flush();
        let _ = std::io::stderr().lock().flush();
        std::process::exit(code)
    }
}
