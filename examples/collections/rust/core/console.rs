use crate::core_iterator::*;
use crate::core_string::*;

pub trait Console {
    fn print(&mut self, message: &String);
}

pub struct __Handle_Console {
    inner: std::sync::Arc<std::sync::Mutex<dyn Console + Send>>,
}

impl Clone for __Handle_Console {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Console {
    pub fn new<__H: Console + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Console + Send>>) -> Self {
        Self { inner }
    }
}

impl Console for __Handle_Console {
    fn print(&mut self, message: &String) {
        self.inner.lock().unwrap().print(message)
    }
}

#[derive(Clone)]
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

pub fn println(console: &mut crate::core_console::__Handle_Console, message: &String) {
    console.print(message);
    console.print(&("\n".to_string()));
}
