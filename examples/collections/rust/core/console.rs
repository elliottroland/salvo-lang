use crate::core_iterator::*;
use crate::core_string::*;

pub trait Console {
    fn print(&mut self, message: &String);
}

#[derive(Clone)]
pub struct __Mon_Console {
    inner: std::sync::Arc<std::sync::Mutex<dyn Console + Send>>,
}

impl __Mon_Console {
    pub fn new(inner: std::sync::Arc<std::sync::Mutex<dyn Console + Send>>) -> Self {
        Self { inner }
    }
}

impl Console for __Mon_Console {
    fn print(&mut self, message: &String) {
        self.inner.lock().unwrap().print(message)
    }
}

pub struct StdOutConsole {
}

impl StdOutConsole {
    pub fn new() -> Self {
        Self { }
    }
}

impl Console for StdOutConsole {
    fn print(&mut self, message: &String) {
        print!("{}", message)
    }
}

pub fn println(console: &mut dyn Console, message: &String) {
    console.print(message);
    console.print(&("\n".to_string()));
}
