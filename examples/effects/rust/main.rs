#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
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
use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;
use crate::core_string::Str;

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
            tick: 0,
        }
    }
}

impl crate::__Stateful_Clock for TickingClock {

    fn now(&mut self) -> i32 {
        self.tick = i32::wrapping_add(self.tick, 5);
        return self.tick;
    }
}

pub fn stamp(clock: &crate::Clock, console: &crate::core_console::Console, label: &String) {
    let mut t = clock.now();
    banner(console, &(format!("{} at t={}", label.clone(), t)));
}

pub fn banner(console: &crate::core_console::Console, text: &String) {
    println(console, &(format!("   {}", text.clone())));
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
    __dep_Console: crate::core_console::Console,
}

impl PlainLogger {
    pub fn new(__dep_Console: crate::core_console::Console) -> Self {
        Self {
            __dep_Console,
        }
    }
}

impl crate::__Stateless_Logger for PlainLogger {

    fn log(&self, message: &String) {
        println(&self.__dep_Console, &(format!("   {}", message.clone())));
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
    __dep_Logger: crate::Logger,
    __dep_Clock: crate::Clock,
}

impl Stamped {
    pub fn new(__dep_Logger: crate::Logger, __dep_Clock: crate::Clock) -> Self {
        Self {
            __dep_Logger,
            __dep_Clock,
        }
    }
}

impl crate::__Stateless_Logger for Stamped {

    fn log(&self, message: &String) {
        self.__dep_Logger.log(&(format!("[t={}] {}", self.__dep_Clock.now(), message.clone())));
    }
}

pub struct Numbered {
    seen: i32,
    __dep_Logger: crate::Logger,
}

impl Numbered {
    pub fn new(__dep_Logger: crate::Logger) -> Self {
        Self {
            seen: 0,
            __dep_Logger,
        }
    }
}

impl crate::__Stateful_Logger for Numbered {

    fn log(&mut self, message: &String) {
        self.seen = i32::wrapping_add(self.seen, 1);
        self.__dep_Logger.log(&(format!("#{} {}", self.seen, message.clone())));
    }
}

pub fn work(logger: &crate::Logger, step: &String) {
    logger.log(step);
}

pub fn interception(logger: &crate::Logger, clock: &crate::Clock) {
    work(logger, &("4. plain".to_string()));
    let logger2 = crate::Logger::shared(Stamped::new(logger.clone(), clock.clone()));
    work(&logger2, &("4. stamped".to_string()));
    let logger3 = crate::Logger::locked(Numbered::new(logger2.clone()));
    work(&logger3, &("4. numbered, then stamped".to_string()));
    work(&logger3, &("4. and again".to_string()));
}

pub fn scoping(logger: &crate::Logger) {
    work(logger, &("5. before the block".to_string()));
    if true {
        let logger2 = crate::Logger::shared(QuietLogger::new());
        work(&logger2, &("5. this line is swallowed".to_string()));
    }
    work(logger, &("5. after the block, logging again".to_string()));
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
    __dep_Console: crate::core_console::Console,
}

impl ConsoleAudit {
    pub fn new(__dep_Console: crate::core_console::Console) -> Self {
        Self {
            __dep_Console,
        }
    }
}

impl crate::__Stateless_Audit for ConsoleAudit {

    fn record(&self, what: &String) {
        println(&self.__dep_Console, &(format!("   audit: {}", what.clone())));
    }
}

#[derive(Clone)]
pub struct ConsoleMetrics {
    __dep_Console: crate::core_console::Console,
}

impl ConsoleMetrics {
    pub fn new(__dep_Console: crate::core_console::Console) -> Self {
        Self {
            __dep_Console,
        }
    }
}

impl crate::__Stateless_Metrics for ConsoleMetrics {

    fn record(&self, what: &String) {
        println(&self.__dep_Console, &(format!("   metric: {}", what.clone())));
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
            value,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> crate::__Stateless_Setting<T> for Fixed<T> {

    fn setting(&self, copy: &mut dyn FnMut(&T) -> T) -> T {
        return copy(&self.value);
    }
}

pub fn settings(setting_i32: &crate::Setting<i32>, setting_string: &crate::Setting<String>, console: &crate::core_console::Console) {
    let mut retries: i32 = setting_i32.setting(&mut |__i0| __i0.clone());
    let mut region = setting_string.setting(&mut |__i0| __i0.clone());
    println(console, &(format!("   retries={} region={}", retries, region)));
}

pub fn main() {
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let clock = crate::Clock::locked(TickingClock::new());
    println(&console, &(format!("1. the clock reads {}, then {}", clock.now(), clock.now())));
    println(&console, &("2. two effects in one signature:".to_string()));
    stamp(&clock, &console, &("2. a labelled moment".to_string()));
    { let __a1 = &("3. a logger whose handler needs the console:".to_string()); println(&console, __a1) };
    let logger = crate::Logger::shared(PlainLogger::new(console.clone()));
    work(&logger, &("3. logged through the console".to_string()));
    println(&console, &("4. interception — each `use` wraps the one before it:".to_string()));
    interception(&logger, &clock);
    println(&console, &("5. shadowing is not wrapping:".to_string()));
    scoping(&logger);
    println(&console, &("6. two effects, one member name:".to_string()));
    let audit = crate::Audit::shared(ConsoleAudit::new(console.clone()));
    audit_only(&audit, &("6. audited only".to_string()));
    let metrics = crate::Metrics::shared(ConsoleMetrics::new(console.clone()));
    audit_and_measure(&audit, &metrics, &("6. audited and measured".to_string()));
    println(&console, &("7. two instances of one generic effect:".to_string()));
    let setting_i32 = crate::Setting::<i32>::shared(Fixed::<i32>::new(3));
    let setting_string = crate::Setting::<String>::shared(Fixed::<String>::new("eu-west-1".to_string()));
    settings(&setting_i32, &setting_string, &console);
}
