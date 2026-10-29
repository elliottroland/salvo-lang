
pub trait __Stateless_Console: Send + Sync {
    fn print(&self, message: &String);
    fn printerr(&self, message: &String);
    fn readln(&self) -> Option<String>;
}

pub trait __Stateful_Console: Send {
    fn print(&mut self, message: &String);
    fn printerr(&mut self, message: &String);
    fn readln(&mut self) -> Option<String>;
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
    pub fn printerr(&self, message: &String) {
        match &self.inner {
            __Inner_Console::Shared(h) => h.printerr(message),
            __Inner_Console::Locked(h) => h.lock().unwrap().printerr(message),
        }
    }
    pub fn readln(&self) -> Option<String> {
        match &self.inner {
            __Inner_Console::Shared(h) => h.readln(),
            __Inner_Console::Locked(h) => h.lock().unwrap().readln(),
        }
    }
}

/// The adapter a `use` of a platform handler of `Console` constructs [platform-abi].
pub struct __Platform_Console<T>(pub T);

/// What a `threadsafe platform handler` of `Console` implements [platform-abi].
pub trait ConsolePlatformSync: Send + Sync {
    fn print(&self, message: &String);
    fn printerr(&self, message: &String);
    fn readln(&self) -> Option<String>;
}

impl<T: ConsolePlatformSync> __Stateless_Console for __Platform_Console<T> {
    fn print(&self, message: &String) {
        self.0.print(message)
    }
    fn printerr(&self, message: &String) {
        self.0.printerr(message)
    }
    fn readln(&self) -> Option<String> {
        self.0.readln()
    }
}

pub type __Platform_StdConsole = crate::std_core_console::__Platform_Console<crate::platform_std_core_console::StdConsole>;

impl __Platform_StdConsole {
    pub fn new() -> Self {
        crate::std_core_console::__Platform_Console(crate::platform_std_core_console::StdConsole::new())
    }
}

pub fn println(console: &crate::std_core_console::Console, message: &String) {
    console.print(message);
    console.print(&String::from("\n"));
}
