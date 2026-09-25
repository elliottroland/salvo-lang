use crate::core_iterator::*;
use crate::core_string::*;

pub trait Console {
    fn print(&mut self, message: &String);
}

pub trait __Share_Console: Console + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Console>;
}

impl<__H: Console + Clone + Send + 'static> __Share_Console for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Console> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Console {
    inner: Box<dyn __Share_Console>,
}

impl Clone for __Mon_Console {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Console {
    pub fn new(inner: Box<dyn __Share_Console>) -> Self {
        Self { inner }
    }
}

impl Console for __Mon_Console {
    fn print(&mut self, message: &String) {
        self.inner.print(message)
    }
}

pub struct __Lock_Console<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Console<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Console<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Console + Send> Console for __Lock_Console<H> {
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

pub fn println(console: &mut dyn Console, message: &String) {
    console.print(message);
    console.print(&("\n".to_string()));
}
