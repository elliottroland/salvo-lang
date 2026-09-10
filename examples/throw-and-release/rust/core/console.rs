use crate::core_iterator::*;
use crate::core_string::*;

pub trait Console {
    fn print(&mut self, message: &String);
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
