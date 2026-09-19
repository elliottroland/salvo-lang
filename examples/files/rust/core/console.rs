use crate::core_iterator::*;
use crate::core_string::*;

pub trait Console {
    fn print(&mut self, message: &String);
}

pub trait __Has_Console {
    fn __get_Console(&mut self) -> &mut dyn Console;
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

pub fn println<__Fx: __Has_Console>(__fx: &mut __Fx, message: &String) {
    __Has_Console::__get_Console(&mut *__fx).print(message);
    __Has_Console::__get_Console(&mut *__fx).print(&("\n".to_string()));
}
