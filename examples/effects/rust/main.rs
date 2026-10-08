#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/basic.rs"]
pub mod core_basic;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/compare.rs"]
pub mod core_compare;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/index.rs"]
pub mod core_index;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "platform/core/console.rs"]
pub mod platform_core_console;
#[path = "platform/core/list.rs"]
pub mod platform_core_list;
#[path = "platform/core/map.rs"]
pub mod platform_core_map;
#[path = "platform/core/set.rs"]
pub mod platform_core_set;
#[path = "platform/core/sorted.rs"]
pub mod platform_core_sorted;
#[path = "platform/core/string.rs"]
pub mod platform_core_string;

use crate::core_console::Console;
use crate::core_console::println;


pub trait __Stateless_Clock: Send + Sync {
    fn now(&self) -> i32;
}

pub trait __Stateful_Clock: Send {
    fn now(&mut self) -> i32;
}

pub struct Clock {
    inner: __Inner_Clock,
}

pub enum __Inner_Clock {
    Shared(std::sync::Arc<dyn __Stateless_Clock>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Clock>>),
}

impl Clone for Clock {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Clock::Shared(h) => __Inner_Clock::Shared(h.clone()),
            __Inner_Clock::Locked(h) => __Inner_Clock::Locked(h.clone()),
        } }
    }
}

impl Clock {
    pub fn shared<__H: __Stateless_Clock + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Clock::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Clock>) -> Self {
        Self { inner: __Inner_Clock::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Clock + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Clock::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Clock>>) -> Self {
        Self { inner: __Inner_Clock::Locked(inner) }
    }
    pub fn now(&self) -> i32 {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.now(),
            __Inner_Clock::Locked(h) => h.lock().unwrap().now(),
        }
    }
}

pub struct TickingClock {
    tick: i32,
}

impl TickingClock {
    pub fn new() -> Self {
        Self {
            tick: 0i32
        }
    }
}

impl crate::__Stateful_Clock for TickingClock {
    fn now(&mut self) -> i32 {
        self.tick = i32::wrapping_add(self.tick, 5i32);
        return self.tick;
    }
}

pub fn stamp(clock: &crate::Clock, console: &crate::core_console::Console, label: &String) {
    let mut t: i32 = clock.now();
    crate::banner(console, &format!("{} at t={}", label, t));
}

pub fn banner(console: &crate::core_console::Console, text: &String) {
    crate::core_console::println(console, &format!("   {}", text));
}

pub trait __Stateless_Logger: Send + Sync {
    fn log(&self, message: &String);
}

pub trait __Stateful_Logger: Send {
    fn log(&mut self, message: &String);
}

pub struct Logger {
    inner: __Inner_Logger,
}

pub enum __Inner_Logger {
    Shared(std::sync::Arc<dyn __Stateless_Logger>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Logger>>),
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Logger::Shared(h) => __Inner_Logger::Shared(h.clone()),
            __Inner_Logger::Locked(h) => __Inner_Logger::Locked(h.clone()),
        } }
    }
}

impl Logger {
    pub fn shared<__H: __Stateless_Logger + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Logger::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Logger>) -> Self {
        Self { inner: __Inner_Logger::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Logger + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Logger::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Logger>>) -> Self {
        Self { inner: __Inner_Logger::Locked(inner) }
    }
    pub fn log(&self, message: &String) {
        match &self.inner {
            __Inner_Logger::Shared(h) => h.log(message),
            __Inner_Logger::Locked(h) => h.lock().unwrap().log(message),
        }
    }
}

#[derive(Clone)]
pub struct PlainLogger {
    __dep0: crate::core_console::Console,
}

impl PlainLogger {
    pub fn new(__dep0: crate::core_console::Console) -> Self {
        Self {
            __dep0
        }
    }
}

impl crate::__Stateless_Logger for PlainLogger {
    fn log(&self, message: &String) {
        crate::core_console::println(&self.__dep0, &format!("   {}", message));
    }
}

#[derive(Clone)]
pub struct QuietLogger {
}

impl QuietLogger {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl crate::__Stateless_Logger for QuietLogger {
    fn log(&self, message: &String) {
    }
}

#[derive(Clone)]
pub struct Stamped {
    __dep0: crate::Logger,
    __dep1: crate::Clock,
}

impl Stamped {
    pub fn new(__dep0: crate::Logger, __dep1: crate::Clock) -> Self {
        Self {
            __dep0,
            __dep1
        }
    }
}

impl crate::__Stateless_Logger for Stamped {
    fn log(&self, message: &String) {
        self.__dep0.log(&format!("[t={}] {}", self.__dep1.now(), message));
    }
}

pub struct Numbered {
    __dep0: crate::Logger,
    seen: i32,
}

impl Numbered {
    pub fn new(__dep0: crate::Logger) -> Self {
        Self {
            __dep0,
            seen: 0i32
        }
    }
}

impl crate::__Stateful_Logger for Numbered {
    fn log(&mut self, message: &String) {
        self.seen = i32::wrapping_add(self.seen, 1i32);
        self.__dep0.log(&format!("#{} {}", self.seen, message));
    }
}

pub fn work(logger: &crate::Logger, step: &String) {
    logger.log(step);
}

pub fn interception(logger: &crate::Logger, clock: &crate::Clock) {
    crate::work(logger, &String::from("4. plain"));
    let mut __use_1: crate::Stamped = crate::Stamped::new(logger.clone(), clock.clone());
    let __handle_2 = crate::Logger::shared(__use_1);
    crate::work(&__handle_2, &String::from("4. stamped"));
    let mut __use_3: crate::Numbered = crate::Numbered::new(__handle_2.clone());
    let __handle_4 = crate::Logger::locked(__use_3);
    crate::work(&__handle_4, &String::from("4. numbered, then stamped"));
    crate::work(&__handle_4, &String::from("4. and again"));
}

pub fn scoping(logger: &crate::Logger) {
    crate::work(logger, &String::from("5. before the block"));
    {
        let mut __use_1: crate::QuietLogger = crate::QuietLogger::new();
        let __handle_2 = crate::Logger::shared(__use_1);
        crate::work(&__handle_2, &String::from("5. this line is swallowed"));
    };
    crate::work(logger, &String::from("5. after the block, logging again"));
}

pub trait __Stateless_Audit: Send + Sync {
    fn record(&self, what: &String);
}

pub trait __Stateful_Audit: Send {
    fn record(&mut self, what: &String);
}

pub struct Audit {
    inner: __Inner_Audit,
}

pub enum __Inner_Audit {
    Shared(std::sync::Arc<dyn __Stateless_Audit>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Audit>>),
}

impl Clone for Audit {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Audit::Shared(h) => __Inner_Audit::Shared(h.clone()),
            __Inner_Audit::Locked(h) => __Inner_Audit::Locked(h.clone()),
        } }
    }
}

impl Audit {
    pub fn shared<__H: __Stateless_Audit + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Audit::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Audit>) -> Self {
        Self { inner: __Inner_Audit::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Audit + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Audit::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Audit>>) -> Self {
        Self { inner: __Inner_Audit::Locked(inner) }
    }
    pub fn record(&self, what: &String) {
        match &self.inner {
            __Inner_Audit::Shared(h) => h.record(what),
            __Inner_Audit::Locked(h) => h.lock().unwrap().record(what),
        }
    }
}

pub trait __Stateless_Metrics: Send + Sync {
    fn record(&self, what: &String);
}

pub trait __Stateful_Metrics: Send {
    fn record(&mut self, what: &String);
}

pub struct Metrics {
    inner: __Inner_Metrics,
}

pub enum __Inner_Metrics {
    Shared(std::sync::Arc<dyn __Stateless_Metrics>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Metrics>>),
}

impl Clone for Metrics {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Metrics::Shared(h) => __Inner_Metrics::Shared(h.clone()),
            __Inner_Metrics::Locked(h) => __Inner_Metrics::Locked(h.clone()),
        } }
    }
}

impl Metrics {
    pub fn shared<__H: __Stateless_Metrics + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Metrics::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Metrics>) -> Self {
        Self { inner: __Inner_Metrics::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Metrics + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Metrics::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Metrics>>) -> Self {
        Self { inner: __Inner_Metrics::Locked(inner) }
    }
    pub fn record(&self, what: &String) {
        match &self.inner {
            __Inner_Metrics::Shared(h) => h.record(what),
            __Inner_Metrics::Locked(h) => h.lock().unwrap().record(what),
        }
    }
}

#[derive(Clone)]
pub struct ConsoleAudit {
    __dep0: crate::core_console::Console,
}

impl ConsoleAudit {
    pub fn new(__dep0: crate::core_console::Console) -> Self {
        Self {
            __dep0
        }
    }
}

impl crate::__Stateless_Audit for ConsoleAudit {
    fn record(&self, what: &String) {
        crate::core_console::println(&self.__dep0, &format!("   audit: {}", what));
    }
}

#[derive(Clone)]
pub struct ConsoleMetrics {
    __dep0: crate::core_console::Console,
}

impl ConsoleMetrics {
    pub fn new(__dep0: crate::core_console::Console) -> Self {
        Self {
            __dep0
        }
    }
}

impl crate::__Stateless_Metrics for ConsoleMetrics {
    fn record(&self, what: &String) {
        crate::core_console::println(&self.__dep0, &format!("   metric: {}", what));
    }
}

pub fn audit_only(audit: &crate::Audit, what: &String) {
    audit.record(what);
}

pub fn audit_and_measure(audit: &crate::Audit, metrics: &crate::Metrics, what: &String) {
    audit.record(what);
    metrics.record(what);
}

pub trait __Stateless_Setting<T>: Send + Sync {
    fn setting(&self, copy: &mut dyn FnMut(&T) -> T) -> T;
}

pub trait __Stateful_Setting<T>: Send {
    fn setting(&mut self, copy: &mut dyn FnMut(&T) -> T) -> T;
}

pub struct Setting<T: 'static> {
    inner: __Inner_Setting<T>,
}

pub enum __Inner_Setting<T: 'static> {
    Shared(std::sync::Arc<dyn __Stateless_Setting<T>>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Setting<T>>>),
}

impl<T: 'static> Clone for Setting<T> {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Setting::Shared(h) => __Inner_Setting::Shared(h.clone()),
            __Inner_Setting::Locked(h) => __Inner_Setting::Locked(h.clone()),
        } }
    }
}

impl<T: 'static> Setting<T> {
    pub fn shared<__H: __Stateless_Setting<T> + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Setting::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Setting<T>>) -> Self {
        Self { inner: __Inner_Setting::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Setting<T> + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Setting::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Setting<T>>>) -> Self {
        Self { inner: __Inner_Setting::Locked(inner) }
    }
    pub fn setting(&self, copy: &mut dyn FnMut(&T) -> T) -> T {
        match &self.inner {
            __Inner_Setting::Shared(h) => h.setting(copy),
            __Inner_Setting::Locked(h) => h.lock().unwrap().setting(copy),
        }
    }
}

#[derive(Clone)]
pub struct Fixed<T: Clone + Send + Sync + 'static> {
    value: T,
}

impl<T: Clone + Send + Sync + 'static> Fixed<T> {
    pub fn new(value: T) -> Self {
        Self {
            value
        }
    }
}

impl<T: Clone + Send + Sync + 'static> crate::__Stateless_Setting<T> for Fixed<T> {
    fn setting(&self, copy: &mut dyn FnMut(&T) -> T) -> T {
        return copy(&self.value);
    }
}

pub fn settings(setting: &crate::Setting<i32>, setting__1: &crate::Setting<String>, console: &crate::core_console::Console) {
    let mut retries: i32 = setting.setting(&mut |__a0| *__a0);
    let mut region: String = setting__1.setting(&mut |__a0| (__a0).clone());
    crate::core_console::println(console, &format!("   retries={} region={}", retries, region));
}

pub fn main() {
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut __use_3: crate::TickingClock = crate::TickingClock::new();
    let __handle_4 = crate::Clock::locked(__use_3);
    crate::core_console::println(&__handle_2, &format!("1. the clock reads {}, then {}", __handle_4.now(), __handle_4.now()));
    crate::core_console::println(&__handle_2, &String::from("2. two effects in one signature:"));
    crate::stamp(&__handle_4, &__handle_2, &String::from("2. a labelled moment"));
    crate::core_console::println(&__handle_2, &String::from("3. a logger whose handler needs the console:"));
    let mut __use_5: crate::PlainLogger = crate::PlainLogger::new(__handle_2.clone());
    let __handle_6 = crate::Logger::shared(__use_5);
    crate::work(&__handle_6, &String::from("3. logged through the console"));
    crate::core_console::println(&__handle_2, &String::from("4. interception — each `use` wraps the one before it:"));
    crate::interception(&__handle_6, &__handle_4);
    crate::core_console::println(&__handle_2, &String::from("5. shadowing is not wrapping:"));
    crate::scoping(&__handle_6);
    crate::core_console::println(&__handle_2, &String::from("6. two effects, one member name:"));
    let mut __use_7: crate::ConsoleAudit = crate::ConsoleAudit::new(__handle_2.clone());
    let __handle_8 = crate::Audit::shared(__use_7);
    crate::audit_only(&__handle_8, &String::from("6. audited only"));
    let mut __use_9: crate::ConsoleMetrics = crate::ConsoleMetrics::new(__handle_2.clone());
    let __handle_10 = crate::Metrics::shared(__use_9);
    crate::audit_and_measure(&__handle_8, &__handle_10, &String::from("6. audited and measured"));
    crate::core_console::println(&__handle_2, &String::from("7. two instances of one generic effect:"));
    let mut __use_11: crate::Fixed<i32> = crate::Fixed::new(3i32);
    let __handle_12 = crate::Setting::shared(__use_11);
    let mut __use_13: crate::Fixed<String> = crate::Fixed::new(String::from("eu-west-1"));
    let __handle_14 = crate::Setting::shared(__use_13);
    crate::settings(&__handle_12, &__handle_14, &__handle_2);
}
