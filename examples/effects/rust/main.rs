#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/console.rs"]
pub mod core_console;
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

use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_string::*;

pub trait Clock {
    fn now(&mut self) -> i32;
}

pub struct __Handle_Clock {
    inner: std::sync::Arc<std::sync::Mutex<dyn Clock + Send>>,
}

impl Clone for __Handle_Clock {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Clock {
    pub fn new<__H: Clock + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Clock + Send>>) -> Self {
        Self { inner }
    }
}

impl Clock for __Handle_Clock {
    fn now(&mut self) -> i32 {
        self.inner.lock().unwrap().now()
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

impl Clock for TickingClock {

    fn now(&mut self) -> i32 {
        self.tick = self.tick + 5;
        return self.tick;
    }
}

pub fn stamp(clock: &mut crate::__Handle_Clock, console: &mut crate::core_console::__Handle_Console, label: &String) {
    let mut t = clock.now();
    banner(console, &(format!("{} at t={}", label.clone(), t)));
}

pub fn banner(console: &mut crate::core_console::__Handle_Console, text: &String) {
    println(console, &(format!("   {}", text.clone())));
}

pub trait Logger {
    fn log(&mut self, message: &String);
}

pub struct __Handle_Logger {
    inner: std::sync::Arc<std::sync::Mutex<dyn Logger + Send>>,
}

impl Clone for __Handle_Logger {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Logger {
    pub fn new<__H: Logger + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Logger + Send>>) -> Self {
        Self { inner }
    }
}

impl Logger for __Handle_Logger {
    fn log(&mut self, message: &String) {
        self.inner.lock().unwrap().log(message)
    }
}

#[derive(Clone)]
pub struct PlainLogger {
    __dep_Console: crate::core_console::__Handle_Console,
}

impl PlainLogger {
    pub fn new(__dep_Console: crate::core_console::__Handle_Console) -> Self {
        Self {
            __dep_Console,
        }
    }
}

impl Logger for PlainLogger {

    fn log(&mut self, message: &String) {
        println(&mut self.__dep_Console, &(format!("   {}", message.clone())));
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

impl Logger for QuietLogger {

    fn log(&mut self, message: &String) {
    }
}

#[derive(Clone)]
pub struct Stamped {
    __dep_Logger: crate::__Handle_Logger,
    __dep_Clock: crate::__Handle_Clock,
}

impl Stamped {
    pub fn new(__dep_Logger: crate::__Handle_Logger, __dep_Clock: crate::__Handle_Clock) -> Self {
        Self {
            __dep_Logger,
            __dep_Clock,
        }
    }
}

impl Logger for Stamped {

    fn log(&mut self, message: &String) {
        self.__dep_Logger.log(&(format!("[t={}] {}", self.__dep_Clock.now(), message.clone())));
    }
}

pub struct Numbered {
    seen: i32,
    __dep_Logger: crate::__Handle_Logger,
}

impl Numbered {
    pub fn new(__dep_Logger: crate::__Handle_Logger) -> Self {
        Self {
            seen: 0,
            __dep_Logger,
        }
    }
}

impl Logger for Numbered {

    fn log(&mut self, message: &String) {
        self.seen = self.seen + 1;
        self.__dep_Logger.log(&(format!("#{} {}", self.seen, message.clone())));
    }
}

pub fn work(logger: &mut crate::__Handle_Logger, step: &String) {
    logger.log(step);
}

pub fn interception(logger: &mut crate::__Handle_Logger, clock: &mut crate::__Handle_Clock) {
    work(logger, &("4. plain".to_string()));
    let mut logger2 = crate::__Handle_Logger::new(Stamped::new(logger.clone(), clock.clone()));
    work(&mut logger2, &("4. stamped".to_string()));
    let mut logger3 = crate::__Handle_Logger::new(Numbered::new(logger2.clone()));
    work(&mut logger3, &("4. numbered, then stamped".to_string()));
    work(&mut logger3, &("4. and again".to_string()));
}

pub fn scoping(logger: &mut crate::__Handle_Logger) {
    work(logger, &("5. before the block".to_string()));
    if true {
        let mut logger2 = crate::__Handle_Logger::new(QuietLogger::new());
        work(&mut logger2, &("5. this line is swallowed".to_string()));
    }
    work(logger, &("5. after the block, logging again".to_string()));
}

pub trait Audit {
    fn record(&mut self, what: &String);
}

pub struct __Handle_Audit {
    inner: std::sync::Arc<std::sync::Mutex<dyn Audit + Send>>,
}

impl Clone for __Handle_Audit {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Audit {
    pub fn new<__H: Audit + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Audit + Send>>) -> Self {
        Self { inner }
    }
}

impl Audit for __Handle_Audit {
    fn record(&mut self, what: &String) {
        self.inner.lock().unwrap().record(what)
    }
}

pub trait Metrics {
    fn record(&mut self, what: &String);
}

pub struct __Handle_Metrics {
    inner: std::sync::Arc<std::sync::Mutex<dyn Metrics + Send>>,
}

impl Clone for __Handle_Metrics {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Metrics {
    pub fn new<__H: Metrics + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Metrics + Send>>) -> Self {
        Self { inner }
    }
}

impl Metrics for __Handle_Metrics {
    fn record(&mut self, what: &String) {
        self.inner.lock().unwrap().record(what)
    }
}

#[derive(Clone)]
pub struct ConsoleAudit {
    __dep_Console: crate::core_console::__Handle_Console,
}

impl ConsoleAudit {
    pub fn new(__dep_Console: crate::core_console::__Handle_Console) -> Self {
        Self {
            __dep_Console,
        }
    }
}

impl Audit for ConsoleAudit {

    fn record(&mut self, what: &String) {
        println(&mut self.__dep_Console, &(format!("   audit: {}", what.clone())));
    }
}

#[derive(Clone)]
pub struct ConsoleMetrics {
    __dep_Console: crate::core_console::__Handle_Console,
}

impl ConsoleMetrics {
    pub fn new(__dep_Console: crate::core_console::__Handle_Console) -> Self {
        Self {
            __dep_Console,
        }
    }
}

impl Metrics for ConsoleMetrics {

    fn record(&mut self, what: &String) {
        println(&mut self.__dep_Console, &(format!("   metric: {}", what.clone())));
    }
}

pub fn audit_only(audit: &mut crate::__Handle_Audit, what: &String) {
    audit.record(what);
}

pub fn audit_and_measure(audit: &mut crate::__Handle_Audit, metrics: &mut crate::__Handle_Metrics, what: &String) {
    audit.record(what);
    metrics.record(what);
}

pub trait Setting<T> {
    fn setting(&mut self, copy: &mut dyn FnMut(&T) -> T) -> T;
}

pub struct __Handle_Setting<T: 'static> {
    inner: std::sync::Arc<std::sync::Mutex<dyn Setting<T> + Send>>,
}

impl<T: 'static> Clone for __Handle_Setting<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: 'static> __Handle_Setting<T> {
    pub fn new<__H: Setting<T> + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Setting<T> + Send>>) -> Self {
        Self { inner }
    }
}

impl<T: 'static> Setting<T> for __Handle_Setting<T> {
    fn setting(&mut self, copy: &mut dyn FnMut(&T) -> T) -> T {
        self.inner.lock().unwrap().setting(copy)
    }
}

#[derive(Clone)]
pub struct Fixed<T: Clone + 'static> {
    value: T,
}

impl<T: Clone + 'static> Fixed<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
        }
    }
}

impl<T: Clone + 'static> Setting<T> for Fixed<T> {

    fn setting(&mut self, copy: &mut dyn FnMut(&T) -> T) -> T {
        return copy(&self.value);
    }
}

pub fn settings(setting_i32: &mut crate::__Handle_Setting<i32>, setting_string: &mut crate::__Handle_Setting<String>, console: &mut crate::core_console::__Handle_Console) {
    let mut retries: i32 = setting_i32.setting(&mut |__i0| __i0.clone());
    let mut region = setting_string.setting(&mut |__i0| __i0.clone());
    println(console, &(format!("   retries={} region={}", retries, region)));
}

pub fn main() {
    let mut console = crate::core_console::__Handle_Console::new(StdOutConsole::new());
    let mut clock = crate::__Handle_Clock::new(TickingClock::new());
    println(&mut console, &(format!("1. the clock reads {}, then {}", clock.now(), clock.now())));
    println(&mut console, &("2. two effects in one signature:".to_string()));
    stamp(&mut clock, &mut console, &("2. a labelled moment".to_string()));
    { let __a1 = &("3. a logger whose handler needs the console:".to_string()); println(&mut console, __a1) };
    let mut logger = crate::__Handle_Logger::new(PlainLogger::new(console.clone()));
    work(&mut logger, &("3. logged through the console".to_string()));
    println(&mut console, &("4. interception — each `use` wraps the one before it:".to_string()));
    interception(&mut logger, &mut clock);
    println(&mut console, &("5. shadowing is not wrapping:".to_string()));
    scoping(&mut logger);
    println(&mut console, &("6. two effects, one member name:".to_string()));
    let mut audit = crate::__Handle_Audit::new(ConsoleAudit::new(console.clone()));
    audit_only(&mut audit, &("6. audited only".to_string()));
    let mut metrics = crate::__Handle_Metrics::new(ConsoleMetrics::new(console.clone()));
    audit_and_measure(&mut audit, &mut metrics, &("6. audited and measured".to_string()));
    println(&mut console, &("7. two instances of one generic effect:".to_string()));
    let mut setting_i32 = crate::__Handle_Setting::<i32>::new(Fixed::<i32>::new(3));
    let mut setting_string = crate::__Handle_Setting::<String>::new(Fixed::<String>::new("eu-west-1".to_string()));
    settings(&mut setting_i32, &mut setting_string, &mut console);
}
