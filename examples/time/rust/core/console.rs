use crate::core_iterator::*;
use crate::core_string::*;

pub trait Console {
    fn print(&mut self, message: &String);
}

pub trait __Has_Console {
    fn __get_Console(&mut self) -> &mut dyn Console;
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
