#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
#[path = "collections.rs"]
pub mod collections;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/nonempty.rs"]
pub mod core_nonempty;
#[path = "core/seq.rs"]
pub mod core_seq;
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

pub trait __Has_Clock {
    fn __get_Clock(&mut self) -> &mut dyn Clock;
}

pub trait __Share_Clock: Clock + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Clock>;
}

impl<__H: Clock + Clone + Send + 'static> __Share_Clock for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Clock> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Clock {
    inner: Box<dyn __Share_Clock>,
}

impl Clone for __Mon_Clock {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Clock {
    pub fn new(inner: Box<dyn __Share_Clock>) -> Self {
        Self { inner }
    }
}

impl Clock for __Mon_Clock {
    fn now(&mut self) -> i32 {
        self.inner.now()
    }
}

pub struct __Lock_Clock<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Clock<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Clock<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Clock + Send> Clock for __Lock_Clock<H> {
    fn now(&mut self) -> i32 {
        self.inner.lock().unwrap().now()
    }
}

impl __Has_Clock for __Mon_Clock {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        self
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

pub fn stamp<__Fx: __Has_Clock + __Has_Console>(__fx: &mut __Fx, label: &String) {
    let mut t = __Has_Clock::__get_Clock(&mut *__fx).now();
    banner(&mut *__fx, &(format!("{} at t={}", label.clone(), t)));
}

pub fn banner<__Fx: __Has_Console>(__fx: &mut __Fx, text: &String) {
    println(&mut *__fx, &(format!("   {}", text.clone())));
}

pub trait Logger {
    fn log(&mut self, message: &String);
}

pub trait __Has_Logger {
    fn __get_Logger(&mut self) -> &mut dyn Logger;
}

pub trait __Share_Logger: Logger + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Logger>;
}

impl<__H: Logger + Clone + Send + 'static> __Share_Logger for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Logger> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Logger {
    inner: Box<dyn __Share_Logger>,
}

impl Clone for __Mon_Logger {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Logger {
    pub fn new(inner: Box<dyn __Share_Logger>) -> Self {
        Self { inner }
    }
}

impl Logger for __Mon_Logger {
    fn log(&mut self, message: &String) {
        self.inner.log(message)
    }
}

pub struct __Lock_Logger<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Logger<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Logger<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Logger + Send> Logger for __Lock_Logger<H> {
    fn log(&mut self, message: &String) {
        self.inner.lock().unwrap().log(message)
    }
}

impl __Has_Logger for __Mon_Logger {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        self
    }
}

#[derive(Clone)]
pub struct PlainLogger {
    __dep_Console: crate::core_console::__Mon_Console,
}

impl PlainLogger {
    pub fn new(__dep_Console: crate::core_console::__Mon_Console) -> Self {
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

pub struct Stamped {
}

impl Stamped {
    pub fn new() -> Self {
        Self {
        }
    }
}

pub struct __Deps_Stamped<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Logger + ?Sized> __Has_Logger for __Deps_Stamped<'a, __P> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        __Has_Logger::__get_Logger(&mut *self.__p)
    }
}

impl<'a, __P: __Has_Clock + ?Sized> __Has_Clock for __Deps_Stamped<'a, __P> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__p)
    }
}

pub trait __Impl_Stamped {

    fn log<__Fx: __Has_Logger + __Has_Clock>(&mut self, __fx: &mut __Fx, message: &String);
}

impl __Impl_Stamped for Stamped {

    fn log<__Fx: __Has_Logger + __Has_Clock>(&mut self, __fx: &mut __Fx, message: &String) {
        { let __a1 = &(format!("[t={}] {}", __Has_Clock::__get_Clock(&mut *__fx).now(), message.clone())); __Has_Logger::__get_Logger(&mut *__fx).log(__a1) };
    }
}

pub struct Numbered {
    seen: i32,
}

impl Numbered {
    pub fn new() -> Self {
        Self {
            seen: 0,
        }
    }
}

pub struct __Deps_Numbered<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Logger + ?Sized> __Has_Logger for __Deps_Numbered<'a, __P> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        __Has_Logger::__get_Logger(&mut *self.__p)
    }
}

pub trait __Impl_Numbered {

    fn log<__Fx: __Has_Logger>(&mut self, __fx: &mut __Fx, message: &String);
}

impl __Impl_Numbered for Numbered {

    fn log<__Fx: __Has_Logger>(&mut self, __fx: &mut __Fx, message: &String) {
        self.seen = self.seen + 1;
        __Has_Logger::__get_Logger(&mut *__fx).log(&(format!("#{} {}", self.seen, message.clone())));
    }
}

pub fn work<__Fx: __Has_Logger>(__fx: &mut __Fx, step: &String) {
    __Has_Logger::__get_Logger(&mut *__fx).log(step);
}

pub fn interception<__Fx: __Has_Logger + __Has_Clock>(__fx: &mut __Fx) {
    work(&mut *__fx, &("4. plain".to_string()));
    let mut __fx2 = __Fx_interception_1 { __outer: &mut *__fx, __h: Stamped::new() };
    work(&mut __fx2, &("4. stamped".to_string()));
    let mut __fx3 = __Fx_interception_2 { __outer: &mut __fx2, __h: Numbered::new() };
    work(&mut __fx3, &("4. numbered, then stamped".to_string()));
    work(&mut __fx3, &("4. and again".to_string()));
}

pub fn scoping<__Fx: __Has_Logger>(__fx: &mut __Fx) {
    work(&mut *__fx, &("5. before the block".to_string()));
    if true {
        let mut __fx2 = __Fx_scoping_3 { __outer: &mut *__fx, __h: QuietLogger::new() };
        work(&mut __fx2, &("5. this line is swallowed".to_string()));
    }
    work(&mut *__fx, &("5. after the block, logging again".to_string()));
}

pub trait Audit {
    fn record(&mut self, what: &String);
}

pub trait __Has_Audit {
    fn __get_Audit(&mut self) -> &mut dyn Audit;
}

pub trait __Share_Audit: Audit + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Audit>;
}

impl<__H: Audit + Clone + Send + 'static> __Share_Audit for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Audit> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Audit {
    inner: Box<dyn __Share_Audit>,
}

impl Clone for __Mon_Audit {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Audit {
    pub fn new(inner: Box<dyn __Share_Audit>) -> Self {
        Self { inner }
    }
}

impl Audit for __Mon_Audit {
    fn record(&mut self, what: &String) {
        self.inner.record(what)
    }
}

pub struct __Lock_Audit<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Audit<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Audit<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Audit + Send> Audit for __Lock_Audit<H> {
    fn record(&mut self, what: &String) {
        self.inner.lock().unwrap().record(what)
    }
}

impl __Has_Audit for __Mon_Audit {
    fn __get_Audit(&mut self) -> &mut dyn Audit {
        self
    }
}

pub trait Metrics {
    fn record(&mut self, what: &String);
}

pub trait __Has_Metrics {
    fn __get_Metrics(&mut self) -> &mut dyn Metrics;
}

pub trait __Share_Metrics: Metrics + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Metrics>;
}

impl<__H: Metrics + Clone + Send + 'static> __Share_Metrics for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Metrics> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Metrics {
    inner: Box<dyn __Share_Metrics>,
}

impl Clone for __Mon_Metrics {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Metrics {
    pub fn new(inner: Box<dyn __Share_Metrics>) -> Self {
        Self { inner }
    }
}

impl Metrics for __Mon_Metrics {
    fn record(&mut self, what: &String) {
        self.inner.record(what)
    }
}

pub struct __Lock_Metrics<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Metrics<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Metrics<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Metrics + Send> Metrics for __Lock_Metrics<H> {
    fn record(&mut self, what: &String) {
        self.inner.lock().unwrap().record(what)
    }
}

impl __Has_Metrics for __Mon_Metrics {
    fn __get_Metrics(&mut self) -> &mut dyn Metrics {
        self
    }
}

#[derive(Clone)]
pub struct ConsoleAudit {
    __dep_Console: crate::core_console::__Mon_Console,
}

impl ConsoleAudit {
    pub fn new(__dep_Console: crate::core_console::__Mon_Console) -> Self {
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
    __dep_Console: crate::core_console::__Mon_Console,
}

impl ConsoleMetrics {
    pub fn new(__dep_Console: crate::core_console::__Mon_Console) -> Self {
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

pub fn audit_only<__Fx: __Has_Audit>(__fx: &mut __Fx, what: &String) {
    __Has_Audit::__get_Audit(&mut *__fx).record(what);
}

pub fn audit_and_measure<__Fx: __Has_Audit + __Has_Metrics>(__fx: &mut __Fx, what: &String) {
    __Has_Audit::__get_Audit(&mut *__fx).record(what);
    __Has_Metrics::__get_Metrics(&mut *__fx).record(what);
}

pub trait Setting<T> {
    fn setting(&mut self, copy: &mut dyn FnMut(T) -> T) -> T;
}

pub trait __Has_Setting<T> {
    fn __get_Setting(&mut self) -> &mut dyn Setting<T>;
}

pub trait __Share_Setting<T: 'static>: Setting<T> + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Setting<T>>;
}

impl<T: 'static, __H: Setting<T> + Clone + Send + 'static> __Share_Setting<T> for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Setting<T>> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Setting<T: 'static> {
    inner: Box<dyn __Share_Setting<T>>,
}

impl<T: 'static> Clone for __Mon_Setting<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl<T: 'static> __Mon_Setting<T> {
    pub fn new(inner: Box<dyn __Share_Setting<T>>) -> Self {
        Self { inner }
    }
}

impl<T: 'static> Setting<T> for __Mon_Setting<T> {
    fn setting(&mut self, copy: &mut dyn FnMut(T) -> T) -> T {
        self.inner.setting(copy)
    }
}

pub struct __Lock_Setting<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Setting<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Setting<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<T: 'static, H: Setting<T> + Send> Setting<T> for __Lock_Setting<H> {
    fn setting(&mut self, copy: &mut dyn FnMut(T) -> T) -> T {
        self.inner.lock().unwrap().setting(copy)
    }
}

impl<T: 'static> __Has_Setting<T> for __Mon_Setting<T> {
    fn __get_Setting(&mut self) -> &mut dyn Setting<T> {
        self
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

    fn setting(&mut self, copy: &mut dyn FnMut(T) -> T) -> T {
        return copy(self.value.clone());
    }
}

pub fn settings<__Fx: __Has_Setting<i32> + __Has_Setting<String> + __Has_Console>(__fx: &mut __Fx) {
    let mut retries: i32 = __Has_Setting::<i32>::__get_Setting(&mut *__fx).setting(&mut |__i0| __i0.clone());
    let mut region = __Has_Setting::<String>::__get_Setting(&mut *__fx).setting(&mut |__i0| __i0.clone());
    println(&mut *__fx, &(format!("   retries={} region={}", retries, region)));
}

pub fn main() {
    let mut __bind = StdOutConsole::new();
    let __handle = crate::core_console::__Mon_Console::new(Box::new(__bind.clone()));
    let mut __fx = __Fx_main_4 { __h: __bind };
    let mut __fx2 = __Fx_main_5 { __outer: &mut __fx, __h: crate::__Lock_Clock::new(TickingClock::new()) };
    { let __a1 = &(format!("1. the clock reads {}, then {}", __Has_Clock::__get_Clock(&mut __fx2).now(), __Has_Clock::__get_Clock(&mut __fx2).now())); println(&mut __fx2, __a1) };
    println(&mut __fx2, &("2. two effects in one signature:".to_string()));
    stamp(&mut __fx2, &("2. a labelled moment".to_string()));
    println(&mut __fx2, &("3. a logger whose handler needs the console:".to_string()));
    let mut __fx3 = __Fx_main_6 { __outer: &mut __fx2, __h: PlainLogger::new(__handle.clone()) };
    work(&mut __fx3, &("3. logged through the console".to_string()));
    println(&mut __fx3, &("4. interception — each `use` wraps the one before it:".to_string()));
    interception(&mut __fx3);
    println(&mut __fx3, &("5. shadowing is not wrapping:".to_string()));
    scoping(&mut __fx3);
    println(&mut __fx3, &("6. two effects, one member name:".to_string()));
    let mut __fx4 = __Fx_main_7 { __outer: &mut __fx3, __h: ConsoleAudit::new(__handle.clone()) };
    audit_only(&mut __fx4, &("6. audited only".to_string()));
    let mut __fx5 = __Fx_main_8 { __outer: &mut __fx4, __h: ConsoleMetrics::new(__handle.clone()) };
    audit_and_measure(&mut __fx5, &("6. audited and measured".to_string()));
    println(&mut __fx5, &("7. two instances of one generic effect:".to_string()));
    let mut __fx6 = __Fx_main_9 { __outer: &mut __fx5, __h: Fixed::<i32>::new(3) };
    let mut __fx7 = __Fx_main_10 { __outer: &mut __fx6, __h: Fixed::<String>::new("eu-west-1".to_string()) };
    settings(&mut __fx7);
}

pub trait __Prov_Clock_Logger: __Has_Clock + __Has_Logger {}
impl<T: __Has_Clock + __Has_Logger + ?Sized> __Prov_Clock_Logger for T {}

pub struct __Fx_interception_1<'a, __H> {
    __outer: &'a mut dyn __Prov_Clock_Logger,
    __h: __H,
}

impl<'a, __H> __Has_Clock for __Fx_interception_1<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H: __Impl_Stamped> Logger for __Fx_interception_1<'a, __H> {
    fn log(&mut self, message: &String) {
        let Self { __outer, __h } = self;
        let mut __deps = __Deps_Stamped{ __p: &mut **__outer };
        __Impl_Stamped::log(__h, &mut __deps, message)
    }
}

impl<'a, __H: __Impl_Stamped> __Has_Logger for __Fx_interception_1<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        self
    }
}

pub struct __Fx_interception_2<'a, __H> {
    __outer: &'a mut dyn __Prov_Clock_Logger,
    __h: __H,
}

impl<'a, __H> __Has_Clock for __Fx_interception_2<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H: __Impl_Numbered> Logger for __Fx_interception_2<'a, __H> {
    fn log(&mut self, message: &String) {
        let Self { __outer, __h } = self;
        let mut __deps = __Deps_Numbered{ __p: &mut **__outer };
        __Impl_Numbered::log(__h, &mut __deps, message)
    }
}

impl<'a, __H: __Impl_Numbered> __Has_Logger for __Fx_interception_2<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        self
    }
}

pub struct __Fx_scoping_3<'a, __H> {
    __outer: &'a mut dyn __Has_Logger,
    __h: __H,
}

impl<'a, __H: Logger> __Has_Logger for __Fx_scoping_3<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        &mut self.__h
    }
}

pub struct __Fx_main_4<__H> {
    __h: __H,
}

impl<__H: Console> __Has_Console for __Fx_main_4<__H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        &mut self.__h
    }
}

pub struct __Fx_main_5<'a, __H> {
    __outer: &'a mut dyn __Has_Console,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_5<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Clock> __Has_Clock for __Fx_main_5<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        &mut self.__h
    }
}

pub trait __Prov_Clock_Console: __Has_Clock + __Has_Console {}
impl<T: __Has_Clock + __Has_Console + ?Sized> __Prov_Clock_Console for T {}

pub struct __Fx_main_6<'a, __H> {
    __outer: &'a mut dyn __Prov_Clock_Console,
    __h: __H,
}

impl<'a, __H> __Has_Clock for __Fx_main_6<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_6<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Logger> __Has_Logger for __Fx_main_6<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        &mut self.__h
    }
}

pub trait __Prov_Clock_Console_Logger: __Has_Clock + __Has_Console + __Has_Logger {}
impl<T: __Has_Clock + __Has_Console + __Has_Logger + ?Sized> __Prov_Clock_Console_Logger for T {}

pub struct __Fx_main_7<'a, __H> {
    __outer: &'a mut dyn __Prov_Clock_Console_Logger,
    __h: __H,
}

impl<'a, __H> __Has_Clock for __Fx_main_7<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_7<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Logger for __Fx_main_7<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        __Has_Logger::__get_Logger(&mut *self.__outer)
    }
}

impl<'a, __H: Audit> __Has_Audit for __Fx_main_7<'a, __H> {
    fn __get_Audit(&mut self) -> &mut dyn Audit {
        &mut self.__h
    }
}

pub trait __Prov_Audit_Clock_Console_Logger: __Has_Audit + __Has_Clock + __Has_Console + __Has_Logger {}
impl<T: __Has_Audit + __Has_Clock + __Has_Console + __Has_Logger + ?Sized> __Prov_Audit_Clock_Console_Logger for T {}

pub struct __Fx_main_8<'a, __H> {
    __outer: &'a mut dyn __Prov_Audit_Clock_Console_Logger,
    __h: __H,
}

impl<'a, __H> __Has_Audit for __Fx_main_8<'a, __H> {
    fn __get_Audit(&mut self) -> &mut dyn Audit {
        __Has_Audit::__get_Audit(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Clock for __Fx_main_8<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_8<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Logger for __Fx_main_8<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        __Has_Logger::__get_Logger(&mut *self.__outer)
    }
}

impl<'a, __H: Metrics> __Has_Metrics for __Fx_main_8<'a, __H> {
    fn __get_Metrics(&mut self) -> &mut dyn Metrics {
        &mut self.__h
    }
}

pub trait __Prov_Audit_Clock_Console_Logger_Metrics: __Has_Audit + __Has_Clock + __Has_Console + __Has_Logger + __Has_Metrics {}
impl<T: __Has_Audit + __Has_Clock + __Has_Console + __Has_Logger + __Has_Metrics + ?Sized> __Prov_Audit_Clock_Console_Logger_Metrics for T {}

pub struct __Fx_main_9<'a, __H> {
    __outer: &'a mut dyn __Prov_Audit_Clock_Console_Logger_Metrics,
    __h: __H,
}

impl<'a, __H> __Has_Audit for __Fx_main_9<'a, __H> {
    fn __get_Audit(&mut self) -> &mut dyn Audit {
        __Has_Audit::__get_Audit(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Clock for __Fx_main_9<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_9<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Logger for __Fx_main_9<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        __Has_Logger::__get_Logger(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Metrics for __Fx_main_9<'a, __H> {
    fn __get_Metrics(&mut self) -> &mut dyn Metrics {
        __Has_Metrics::__get_Metrics(&mut *self.__outer)
    }
}

impl<'a, __H: Setting<i32>> __Has_Setting<i32> for __Fx_main_9<'a, __H> {
    fn __get_Setting(&mut self) -> &mut dyn Setting<i32> {
        &mut self.__h
    }
}

pub trait __Prov_Audit_Clock_Console_Logger_Metrics_Setting_i32: __Has_Audit + __Has_Clock + __Has_Console + __Has_Logger + __Has_Metrics + __Has_Setting<i32> {}
impl<T: __Has_Audit + __Has_Clock + __Has_Console + __Has_Logger + __Has_Metrics + __Has_Setting<i32> + ?Sized> __Prov_Audit_Clock_Console_Logger_Metrics_Setting_i32 for T {}

pub struct __Fx_main_10<'a, __H> {
    __outer: &'a mut dyn __Prov_Audit_Clock_Console_Logger_Metrics_Setting_i32,
    __h: __H,
}

impl<'a, __H> __Has_Audit for __Fx_main_10<'a, __H> {
    fn __get_Audit(&mut self) -> &mut dyn Audit {
        __Has_Audit::__get_Audit(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Clock for __Fx_main_10<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_10<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Logger for __Fx_main_10<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger {
        __Has_Logger::__get_Logger(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Metrics for __Fx_main_10<'a, __H> {
    fn __get_Metrics(&mut self) -> &mut dyn Metrics {
        __Has_Metrics::__get_Metrics(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Setting<i32> for __Fx_main_10<'a, __H> {
    fn __get_Setting(&mut self) -> &mut dyn Setting<i32> {
        __Has_Setting::<i32>::__get_Setting(&mut *self.__outer)
    }
}

impl<'a, __H: Setting<String>> __Has_Setting<String> for __Fx_main_10<'a, __H> {
    fn __get_Setting(&mut self) -> &mut dyn Setting<String> {
        &mut self.__h
    }
}
