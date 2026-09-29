use crate::core_iterator::*;
use crate::core_string::*;

pub trait __Stateless_Console: Send + Sync {
    fn print(&self, message: &String);
}

pub trait __Stateful_Console: Send {
    fn print(&mut self, message: &String);
}

pub struct Console {
    inner: __Inner_Console,
}

pub enum __Inner_Console {
    Shared(std::sync::Arc<dyn __Stateless_Console>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Console>>),
}

impl Clone for Console {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Console::Shared(h) => __Inner_Console::Shared(h.clone()),
            __Inner_Console::Locked(h) => __Inner_Console::Locked(h.clone()),
        } }
    }
}

impl Console {
    pub fn shared<__H: __Stateless_Console + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Console::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Console>) -> Self {
        Self { inner: __Inner_Console::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Console + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Console::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Console>>) -> Self {
        Self { inner: __Inner_Console::Locked(inner) }
    }
    pub fn print(&self, message: &String) {
        match &self.inner {
            __Inner_Console::Shared(h) => h.print(message),
            __Inner_Console::Locked(h) => h.lock().unwrap().print(message),
        }
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

impl crate::core_console::__Stateless_Console for StdOutConsole {
    fn print(&self, message: &String) {
        print!("{}", message)
    }
}

pub fn println(console: &crate::core_console::Console, message: &String) {
    console.print(message);
    console.print(&("\n".to_string()));
}
