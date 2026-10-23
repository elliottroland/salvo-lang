use crate::core_console::Console;
use crate::args_lex::EndToken;
use crate::args_lex::LongToken;
use crate::core_string::Mut_Str;
use crate::core_result::Ok;
use crate::process::Process;
use crate::args_lex::ShortToken;
use crate::args_lex::WordToken;
use crate::core_list::add_platform;
use crate::core_string::append_platform;
use crate::core_string::char_at_platform;
use crate::core_list::contains;
use crate::core_result::errs;
use crate::core_list::get_platform;
use crate::core_string::join_platform;
use crate::args_lex::lex;
use crate::core_string::repeat;
use crate::core_string::replace_platform;
use crate::core_string::split_last;
use crate::core_string::split_once;
use crate::core_string::starts_with_platform;
use crate::core_string::substr_platform;
use crate::core_string::to_lower_platform;
use crate::core_string::to_upper_platform;
use crate::core_string::trim_platform;
use crate::core_string::trim_prefix_platform;
use crate::core_string::trim_suffix_platform;


pub trait __Stateless_Args: Send + Sync {
    fn args(&self) -> Vec<String>;
}

pub trait __Stateful_Args: Send {
    fn args(&mut self) -> Vec<String>;
}

pub struct Args {
    inner: __Inner_Args,
}

pub enum __Inner_Args {
    Shared(std::sync::Arc<dyn __Stateless_Args>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Args>>),
}

impl Clone for Args {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Args::Shared(h) => __Inner_Args::Shared(h.clone()),
            __Inner_Args::Locked(h) => __Inner_Args::Locked(h.clone()),
        } }
    }
}

impl Args {
    pub fn shared<__H: __Stateless_Args + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Args::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Args>) -> Self {
        Self { inner: __Inner_Args::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Args + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Args::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Args>>) -> Self {
        Self { inner: __Inner_Args::Locked(inner) }
    }
    pub fn args(&self) -> Vec<String> {
        match &self.inner {
            __Inner_Args::Shared(h) => h.args(),
            __Inner_Args::Locked(h) => h.lock().unwrap().args(),
        }
    }
}

/// The adapter a `use` of a platform handler of `Args` constructs [platform-abi].
pub struct __Platform_Args<T>(pub T);

/// What a `threadsafe platform handler` of `Args` implements [platform-abi].
pub trait ArgsPlatformSync: Send + Sync {
    fn args(&self) -> Vec<String>;
}

impl<T: ArgsPlatformSync> __Stateless_Args for __Platform_Args<T> {
    fn args(&self) -> Vec<String> {
        self.0.args()
    }
}

pub type __Platform_HostArgs = crate::args::__Platform_Args<crate::platform_args::HostArgs>;

impl __Platform_HostArgs {
    pub fn new() -> Self {
        crate::args::__Platform_Args(crate::platform_args::HostArgs::new())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnknownOption {
    pub option: String,
}

impl crate::wire::__Wire for UnknownOption {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.option, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            option: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MissingValue {
    pub option: String,
}

impl crate::wire::__Wire for MissingValue {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.option, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            option: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Missing {
    pub what: String,
}

impl crate::wire::__Wire for Missing {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.what, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            what: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Invalid {
    pub what: String,
    pub value: String,
    pub reason: String,
}

impl crate::wire::__Wire for Invalid {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.what, out);
        crate::wire::__Wire::__enc(&self.value, out);
        crate::wire::__Wire::__enc(&self.reason, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            what: crate::wire::__Wire::__dec(r)?,
            value: crate::wire::__Wire::__dec(r)?,
            reason: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unexpected {
    pub value: String,
}

impl crate::wire::__Wire for Unexpected {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Repeated {
    pub option: String,
}

impl crate::wire::__Wire for Repeated {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.option, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            option: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Requires {
    pub option: String,
    pub needs: String,
}

impl crate::wire::__Wire for Requires {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.option, out);
        crate::wire::__Wire::__enc(&self.needs, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            option: crate::wire::__Wire::__dec(r)?,
            needs: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    pub option: String,
    pub other: String,
}

impl crate::wire::__Wire for Conflict {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.option, out);
        crate::wire::__Wire::__enc(&self.other, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            option: crate::wire::__Wire::__dec(r)?,
            other: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MissingOne {
    pub options: Vec<String>,
}

impl crate::wire::__Wire for MissingOne {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.options, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            options: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnknownCommand {
    pub name: String,
}

impl crate::wire::__Wire for UnknownCommand {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HelpRequested {
    pub text: String,
}

impl crate::wire::__Wire for HelpRequested {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.text, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            text: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn message(mut e: crate::unions::Union11<&crate::args::UnknownOption, &crate::args::MissingValue, &crate::args::Missing, &crate::args::Invalid, &crate::args::Unexpected, &crate::args::Repeated, &crate::args::Requires, &crate::args::Conflict, &crate::args::MissingOne, &crate::args::UnknownCommand, &crate::args::HelpRequested>) -> String {
    return if matches!(e, crate::unions::Union11::U1(_)) {
        let mut e_1 = match e { crate::unions::Union11::U1(__v) => __v, _ => unreachable!() };
        return format!("unknown option '{}'", e_1.option);
    } else if matches!(e, crate::unions::Union11::U2(_)) {
        let mut e_2 = match e { crate::unions::Union11::U2(__v) => __v, _ => unreachable!() };
        return format!("option '{}' needs a value", e_2.option);
    } else if matches!(e, crate::unions::Union11::U3(_)) {
        let mut e_3 = match e { crate::unions::Union11::U3(__v) => __v, _ => unreachable!() };
        return format!("'{}' is required", e_3.what);
    } else if matches!(e, crate::unions::Union11::U4(_)) {
        let mut e_4 = match e { crate::unions::Union11::U4(__v) => __v, _ => unreachable!() };
        return format!("invalid value '{}' for '{}': {}", e_4.value, e_4.what, e_4.reason);
    } else if matches!(e, crate::unions::Union11::U5(_)) {
        let mut e_5 = match e { crate::unions::Union11::U5(__v) => __v, _ => unreachable!() };
        return format!("unexpected argument '{}'", e_5.value);
    } else if matches!(e, crate::unions::Union11::U6(_)) {
        let mut e_6 = match e { crate::unions::Union11::U6(__v) => __v, _ => unreachable!() };
        return format!("option '{}' was given more than once", e_6.option);
    } else if matches!(e, crate::unions::Union11::U7(_)) {
        let mut e_7 = match e { crate::unions::Union11::U7(__v) => __v, _ => unreachable!() };
        return format!("'{}' requires '{}'", e_7.option, e_7.needs);
    } else if matches!(e, crate::unions::Union11::U8(_)) {
        let mut e_8 = match e { crate::unions::Union11::U8(__v) => __v, _ => unreachable!() };
        return format!("'{}' cannot be used with '{}'", e_8.option, e_8.other);
    } else if matches!(e, crate::unions::Union11::U9(_)) {
        let mut e_9 = match e { crate::unions::Union11::U9(__v) => __v, _ => unreachable!() };
        return format!("one of '{}' is required", crate::core_string::join_platform(&e_9.options, &String::from("', '")));
    } else if matches!(e, crate::unions::Union11::U10(_)) {
        let mut e_10 = match e { crate::unions::Union11::U10(__v) => __v, _ => unreachable!() };
        return format!("unrecognized command '{}'", e_10.name);
    } else if matches!(e, crate::unions::Union11::U11(_)) {
        let mut e_11 = match e { crate::unions::Union11::U11(__v) => __v, _ => unreachable!() };
        return String::from("help was requested");
    } else {
        panic!("salvo: unhandled union arm at args:122:5");
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptSpec {
    pub field: String,
    pub short: Option<char>,
    pub takes_value: bool,
    pub repeats: bool,
    pub required: bool,
    pub help: Option<String>,
    pub default_text: Option<String>,
    pub choices: Vec<String>,
    pub group: Option<String>,
}

impl crate::wire::__Wire for OptSpec {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.field, out);
        crate::wire::__Wire::__enc(&self.short, out);
        crate::wire::__Wire::__enc(&self.takes_value, out);
        crate::wire::__Wire::__enc(&self.repeats, out);
        crate::wire::__Wire::__enc(&self.required, out);
        crate::wire::__Wire::__enc(&self.help, out);
        crate::wire::__Wire::__enc(&self.default_text, out);
        crate::wire::__Wire::__enc(&self.choices, out);
        crate::wire::__Wire::__enc(&self.group, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            field: crate::wire::__Wire::__dec(r)?,
            short: crate::wire::__Wire::__dec(r)?,
            takes_value: crate::wire::__Wire::__dec(r)?,
            repeats: crate::wire::__Wire::__dec(r)?,
            required: crate::wire::__Wire::__dec(r)?,
            help: crate::wire::__Wire::__dec(r)?,
            default_text: crate::wire::__Wire::__dec(r)?,
            choices: crate::wire::__Wire::__dec(r)?,
            group: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupSpec {
    pub key: String,
    pub kind: String,
    pub required: bool,
    pub heading: Option<String>,
}

impl crate::wire::__Wire for GroupSpec {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.key, out);
        crate::wire::__Wire::__enc(&self.kind, out);
        crate::wire::__Wire::__enc(&self.required, out);
        crate::wire::__Wire::__enc(&self.heading, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            key: crate::wire::__Wire::__dec(r)?,
            kind: crate::wire::__Wire::__dec(r)?,
            required: crate::wire::__Wire::__dec(r)?,
            heading: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArgSpec {
    pub field: String,
    pub help: Option<String>,
    pub required: bool,
    pub many: bool,
}

impl crate::wire::__Wire for ArgSpec {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.field, out);
        crate::wire::__Wire::__enc(&self.help, out);
        crate::wire::__Wire::__enc(&self.required, out);
        crate::wire::__Wire::__enc(&self.many, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            field: crate::wire::__Wire::__dec(r)?,
            help: crate::wire::__Wire::__dec(r)?,
            required: crate::wire::__Wire::__dec(r)?,
            many: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub name: String,
    pub about: Option<String>,
    pub opts: Vec<crate::args::OptSpec>,
    pub args: Vec<crate::args::ArgSpec>,
    pub groups: Vec<crate::args::GroupSpec>,
    pub commands: Vec<crate::args::CmdSpec>,
    pub command_required: bool,
    pub command_fields: i32,
}

impl crate::wire::__Wire for Spec {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
        crate::wire::__Wire::__enc(&self.about, out);
        crate::wire::__Wire::__enc(&self.opts, out);
        crate::wire::__Wire::__enc(&self.args, out);
        crate::wire::__Wire::__enc(&self.groups, out);
        crate::wire::__Wire::__enc(&self.commands, out);
        crate::wire::__Wire::__enc(&self.command_required, out);
        crate::wire::__Wire::__enc(&self.command_fields, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
            about: crate::wire::__Wire::__dec(r)?,
            opts: crate::wire::__Wire::__dec(r)?,
            args: crate::wire::__Wire::__dec(r)?,
            groups: crate::wire::__Wire::__dec(r)?,
            commands: crate::wire::__Wire::__dec(r)?,
            command_required: crate::wire::__Wire::__dec(r)?,
            command_fields: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CmdSpec {
    pub name: String,
    pub about: Option<String>,
}

impl crate::wire::__Wire for CmdSpec {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
        crate::wire::__Wire::__enc(&self.about, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
            about: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn merge_group(opts: &mut Vec<crate::args::OptSpec>, groups: &mut Vec<crate::args::GroupSpec>, child: &crate::args::Spec, key: &String, kind: &String, mut heading: Option<String>) {
    if !(crate::core_list::is_empty::<crate::args::ArgSpec>(&child.args)) {
        panic!("salvo: {} at args:218:9", format!("`{}` is a group of options, and `{}` takes positionals", key, child.name));
    };
    crate::core_list::add_platform::<crate::args::GroupSpec>(&mut *groups, crate::args::GroupSpec { key: (key).clone(), kind: (kind).clone(), required: false, heading: {
        let mut __pick_1 = &heading;
        if __pick_1.is_some() {
            let mut __picked_2 = __pick_1.as_ref().unwrap();
            Some(__picked_2.clone())
        } else {
            (child.about).clone()
        }
    } });
    for mut g in child.groups.iter() {
        crate::core_list::add_platform::<crate::args::GroupSpec>(&mut *groups, crate::args::GroupSpec { key: format!("{}/{}", key, g.key), kind: (g.kind).clone(), required: g.required, heading: (g.heading).clone() });
    }
    for mut o in child.opts.iter() {
        let mut group: String = if o.group.is_none() {
            (key).clone()
        } else {
            let mut __narrowed_4 = o.group.as_ref().unwrap();
            format!("{}/{}", key, __narrowed_4)
        };
        crate::core_list::add_platform::<crate::args::OptSpec>(&mut *opts, crate::args::OptSpec { field: (o.field).clone(), short: o.short.clone(), takes_value: o.takes_value, repeats: o.repeats, required: o.required, help: (o.help).clone(), default_text: (o.default_text).clone(), choices: (o.choices).clone(), group: Some(group.clone()) });
    }
    if ((&kind[..] == &String::from("arm")[..]) && crate::core_list::is_empty::<crate::args::OptSpec>(&child.opts)) {
        crate::core_list::add_platform::<crate::args::OptSpec>(&mut *opts, crate::args::OptSpec { field: crate::core_string::replace_platform(&crate::args::command_name(&child.name), &String::from("-"), &String::from("_")), short: None, takes_value: false, repeats: false, required: false, help: (child.about).clone(), default_text: None, choices: vec![], group: Some((key).clone()) });
    };
}

pub fn add_one(groups: &mut Vec<crate::args::GroupSpec>, key: &String, mut required: bool, mut heading: Option<String>) {
    crate::core_list::add_platform::<crate::args::GroupSpec>(&mut *groups, crate::args::GroupSpec { key: (key).clone(), kind: String::from("one"), required: required, heading: heading.clone() });
}

pub fn any_given(found: &crate::args::Found, spec: &crate::args::Spec) -> bool {
    if crate::core_list::is_empty::<crate::args::OptSpec>(&spec.opts) {
        return crate::args::given(found, &crate::core_string::replace_platform(&crate::args::command_name(&spec.name), &String::from("-"), &String::from("_")));
    };
    for mut o in spec.opts.iter() {
        if crate::args::given(found, &o.field) {
            return true;
        };
    }
    return false;
}

pub fn long_name(field: &String) -> String {
    return format!("--{}", crate::core_string::replace_platform(field, &String::from("_"), &String::from("-")));
}

pub fn arg_name(field: &String) -> String {
    return format!("<{}>", crate::core_string::replace_platform(field, &String::from("_"), &String::from("-")));
}

pub fn command_name(type_name: &String) -> String {
    let mut last: String = type_name.clone();
    let mut cut: Option<(String, String)> = crate::core_string::split_last(type_name, &String::from("."));
    if !(cut.is_none()) {
        let mut cut_1 = cut.as_ref().unwrap();
        let mut __destructured_2: (String, String) = cut_1.clone();
        let mut __unused_3: String = __destructured_2.0;
        let mut tail: String = __destructured_2.1;
        last = tail.clone();
    };
    let mut out: String = crate::core_string::Mut_Str(vec![String::from("")]);
    let mut first: bool = true;
    for mut c in last.chars() {
        let mut lower: String = crate::core_string::to_lower_platform(&format!("{}", c));
        if (!(first) && !((&lower[..] == &format!("{}", c)[..]))) {
            crate::core_string::append_platform(&mut out, &String::from("-"));
        };
        crate::core_string::append_platform(&mut out, &lower);
        first = false;
    }
    return out;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub field: String,
    pub value: String,
}

impl crate::wire::__Wire for Hit {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.field, out);
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            field: crate::wire::__Wire::__dec(r)?,
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub hits: Vec<crate::args::Hit>,
    pub errors: Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>,
    pub help: bool,
    pub command: Option<String>,
    pub rest: Vec<String>,
    pub path: String,
}

impl crate::wire::__Wire for Found {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.hits, out);
        crate::wire::__Wire::__enc(&self.errors, out);
        crate::wire::__Wire::__enc(&self.help, out);
        crate::wire::__Wire::__enc(&self.command, out);
        crate::wire::__Wire::__enc(&self.rest, out);
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            hits: crate::wire::__Wire::__dec(r)?,
            errors: crate::wire::__Wire::__dec(r)?,
            help: crate::wire::__Wire::__dec(r)?,
            command: crate::wire::__Wire::__dec(r)?,
            rest: crate::wire::__Wire::__dec(r)?,
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn scan(spec: &crate::args::Spec, args: &Vec<String>) -> crate::args::Found {
    let mut tokens: Vec<crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>> = crate::args_lex::lex(args);
    let mut hits: Vec<crate::args::Hit> = vec![];
    let mut errors: Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>> = vec![];
    let mut words: Vec<String> = vec![];
    let mut help: bool = false;
    let mut command: Option<String> = None;
    let mut failed_command: bool = false;
    let mut rest: Vec<String> = vec![];
    let mut n: i32 = crate::core_list::size_platform::<crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>>(&tokens);
    let mut i: i32 = 0i32;
    loop {
        if !((i < n)) {
            break;
        };
        let mut t: crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken> = ({
            let mut __nn_1: Option<&crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>> = crate::core_list::get_platform::<crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>>(&tokens, i);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at args:335:22");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }).clone();
        i = i32::wrapping_add(i, 1i32);
        if matches!(t, crate::unions::Union4::U4(_)) {
            let mut t_3 = match &t { crate::unions::Union4::U4(__v) => __v, _ => unreachable!() };
        } else if matches!(t, crate::unions::Union4::U3(_)) {
            let mut t_4 = match &t { crate::unions::Union4::U3(__v) => __v, _ => unreachable!() };
            if ((!(crate::core_list::is_empty::<crate::args::CmdSpec>(&spec.commands)) && command.is_none()) && !(failed_command)) {
                if crate::args::has_command(spec, &t_4.value) {
                    command = Some((t_4.value).clone());
                    loop {
                        if !((i < n)) {
                            break;
                        };
                        crate::core_list::add_platform::<String>(&mut rest, ({
                            let mut __nn_5: Option<&String> = crate::core_list::get_platform::<String>(args, i);
                            if __nn_5.is_none() {
                                panic!("salvo: value is absent at args:346:44");
                            } else {
                                let mut __some_6 = __nn_5.unwrap();
                                __some_6
                            }
                        }).clone());
                        i = i32::wrapping_add(i, 1i32);
                    }
                } else {
                    crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U10(crate::args::UnknownCommand { name: (t_4.value).clone() }));
                    failed_command = true;
                    i = n;
                };
            } else {
                crate::core_list::add_platform::<String>(&mut words, t_4.value.clone());
            };
        } else if matches!(t, crate::unions::Union4::U1(_)) {
            let mut t_7 = match t { crate::unions::Union4::U1(__v) => __v, _ => unreachable!() };
            if (&t_7.name[..] == &String::from("help")[..]) {
                help = true;
            } else {
                let mut o: Option<crate::args::OptSpec> = crate::args::long_opt(spec, &t_7.name);
                if o.is_none() {
                    crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U1(crate::args::UnknownOption { option: format!("--{}", t_7.name) }));
                } else {
                    let mut o_8 = o.as_ref().unwrap();
                    let mut flag: String = crate::args::long_name(&o_8.field);
                    let mut attached: Option<String> = t_7.value;
                    if (!(o_8.takes_value) && attached.is_none()) {
                        crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (o_8.field).clone(), value: String::from("") });
                    } else if !(o_8.takes_value) {
                        crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U4(crate::args::Invalid { what: flag.clone(), value: {
                            let mut __pick_9 = &attached;
                            if __pick_9.is_some() {
                                let mut __picked_10 = __pick_9.as_ref().unwrap();
                                __picked_10.clone()
                            } else {
                                String::from("")
                            }
                        }, reason: String::from("the option takes no value") }));
                    } else if !(attached.is_none()) {
                        let mut attached_12 = attached.as_ref().unwrap();
                        crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (o_8.field).clone(), value: attached_12.clone() });
                    } else {
                        let mut v: Option<String> = crate::args::word_at(&tokens, i);
                        if v.is_none() {
                            crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U2(crate::args::MissingValue { option: flag.clone() }));
                        } else {
                            let mut v_13 = v.as_ref().unwrap();
                            crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (o_8.field).clone(), value: v_13.clone() });
                            i = i32::wrapping_add(i, 1i32);
                        };
                    };
                };
            };
        } else if matches!(t, crate::unions::Union4::U2(_)) {
            let mut t_14 = match &t { crate::unions::Union4::U2(__v) => __v, _ => unreachable!() };
            let mut letters = &t_14.letters;
            let mut k: i32 = 0i32;
            let mut len: i32 = crate::core_string::size_platform(letters);
            loop {
                if !((k < len)) {
                    break;
                };
                let mut c: char = {
                    let mut __nn_15: Option<char> = crate::core_string::char_at_platform(letters, k);
                    if __nn_15.is_none() {
                        panic!("salvo: value is absent at args:396:29");
                    } else {
                        let mut __some_16 = __nn_15.unwrap();
                        __some_16
                    }
                };
                k = i32::wrapping_add(k, 1i32);
                if (c == 'h') {
                    help = true;
                } else {
                    let mut o: Option<crate::args::OptSpec> = crate::args::short_opt(spec, c);
                    if o.is_none() {
                        crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U1(crate::args::UnknownOption { option: format!("-{}", c) }));
                    } else {
                        let mut o_17 = o.as_ref().unwrap();
                        if !(o_17.takes_value) {
                            let mut o_18 = o.as_ref().unwrap();
                            crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (o_18.field).clone(), value: String::from("") });
                        } else {
                            if (k < len) {
                                let mut o_19 = o.as_ref().unwrap();
                                crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (o_19.field).clone(), value: {
                                    let mut __nn_20: Option<String> = crate::core_string::substr_platform(letters, k, len);
                                    if __nn_20.is_none() {
                                        panic!("salvo: value is absent at args:412:78");
                                    } else {
                                        let mut __some_21 = __nn_20.unwrap();
                                        __some_21
                                    }
                                } });
                                k = len;
                            } else {
                                let mut v: Option<String> = crate::args::word_at(&tokens, i);
                                if v.is_none() {
                                    crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U2(crate::args::MissingValue { option: format!("-{}", c) }));
                                } else {
                                    let mut o_22 = o.as_ref().unwrap();
                                    let mut v_23 = v.as_ref().unwrap();
                                    crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (o_22.field).clone(), value: v_23.clone() });
                                    i = i32::wrapping_add(i, 1i32);
                                };
                            };
                        };
                    };
                };
            }
        } else {
            panic!("salvo: unhandled union arm at args:337:9");
        };
    }
    for mut o in spec.opts.iter() {
        let mut flag: String = crate::args::long_name(&o.field);
        let mut count: i32 = 0i32;
        for mut h in hits.iter() {
            if (&h.field[..] == &o.field[..]) {
                count = i32::wrapping_add(count, 1i32);
                if (!(crate::core_list::is_empty::<String>(&o.choices)) && !(crate::core_list::contains::<String>(&o.choices, &h.value, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])))) {
                    crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U4(crate::args::Invalid { what: (flag).clone(), value: (h.value).clone(), reason: format!("expected one of {}", crate::core_string::join_platform(&o.choices, &String::from(", "))) }));
                };
            };
        }
        if ((count > 1i32) && !(o.repeats)) {
            crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U6(crate::args::Repeated { option: (flag).clone() }));
        };
        if ((count == 0i32) && o.required) {
            let mut why: Option<String> = crate::args::unmet_group(spec, &hits, (&o.group).as_ref());
            if why.is_some() {
                let mut why_24 = why.as_ref().unwrap();
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U7(crate::args::Requires { option: why_24.clone(), needs: (flag).clone() }));
            } else if crate::args::live(spec, &hits, (&o.group).as_ref()) {
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U3(crate::args::Missing { what: (flag).clone() }));
            } else {
            };
        };
    }
    for mut g in spec.groups.iter() {
        if ((&g.kind[..] == &String::from("one")[..]) && crate::args::live(spec, &hits, Some(&g.key))) {
            let mut first: Option<String> = None;
            let mut names: Vec<String> = vec![];
            for mut a in spec.groups.iter() {
                if ((&a.kind[..] == &String::from("arm")[..]) && crate::args::parent_is(&a.key, &g.key)) {
                    crate::core_list::add_platform::<String>(&mut names, crate::args::first_option(spec, &a.key));
                    let mut here: Option<String> = crate::args::given_in(spec, &hits, &a.key);
                    if !(here.is_none()) {
                        if first.is_none() {
                            let mut here_25 = here.as_ref().unwrap();
                            first = Some(here_25.clone());
                        } else {
                            let mut here_26 = here.as_ref().unwrap();
                            let mut first_27 = first.as_ref().unwrap();
                            crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U8(crate::args::Conflict { option: here_26.clone(), other: (first_27).clone() }));
                        };
                    };
                };
            }
            if (first.is_none() && g.required) {
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U9(crate::args::MissingOne { options: names.clone() }));
            };
        };
    }
    let mut w: i32 = 0i32;
    let mut nw: i32 = crate::core_list::size_platform::<String>(&words);
    for mut a in spec.args.iter() {
        if a.many {
            if ((w == nw) && a.required) {
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U3(crate::args::Missing { what: crate::args::arg_name(&a.field) }));
            };
            loop {
                if !((w < nw)) {
                    break;
                };
                crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (a.field).clone(), value: ({
                    let mut __nn_28: Option<&String> = crate::core_list::get_platform::<String>(&words, w);
                    if __nn_28.is_none() {
                        panic!("salvo: value is absent at args:488:67");
                    } else {
                        let mut __some_29 = __nn_28.unwrap();
                        __some_29
                    }
                }).clone() });
                w = i32::wrapping_add(w, 1i32);
            }
        } else {
            if (w < nw) {
                crate::core_list::add_platform::<crate::args::Hit>(&mut hits, crate::args::Hit { field: (a.field).clone(), value: ({
                    let mut __nn_30: Option<&String> = crate::core_list::get_platform::<String>(&words, w);
                    if __nn_30.is_none() {
                        panic!("salvo: value is absent at args:494:71");
                    } else {
                        let mut __some_31 = __nn_30.unwrap();
                        __some_31
                    }
                }).clone() });
                w = i32::wrapping_add(w, 1i32);
            } else if a.required {
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U3(crate::args::Missing { what: crate::args::arg_name(&a.field) }));
            } else {
            };
        };
    }
    loop {
        if !((w < nw)) {
            break;
        };
        crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U5(crate::args::Unexpected { value: ({
            let mut __nn_32: Option<&String> = crate::core_list::get_platform::<String>(&words, w);
            if __nn_32.is_none() {
                panic!("salvo: value is absent at args:505:46");
            } else {
                let mut __some_33 = __nn_32.unwrap();
                __some_33
            }
        }).clone() }));
        w = i32::wrapping_add(w, 1i32);
    }
    if (((!(crate::core_list::is_empty::<crate::args::CmdSpec>(&spec.commands)) && command.is_none()) && !(failed_command)) && spec.command_required) {
        crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut errors, crate::unions::Union11::U3(crate::args::Missing { what: String::from("<command>") }));
    };
    return crate::args::Found { hits: hits.clone(), errors: errors.clone(), help: help, command: command.clone(), rest: rest.clone(), path: (spec.name).clone() };
}

pub fn parent_key(key: &String) -> Option<String> {
    let mut cut: Option<(String, String)> = crate::core_string::split_last(key, &String::from("/"));
    if cut.is_none() {
        return None;
    };
    let mut cut_1 = cut.unwrap();
    let mut __destructured_2: (String, String) = cut_1;
    let mut parent: String = __destructured_2.0;
    let mut __unused_3: String = __destructured_2.1;
    return Some(parent.clone());
}

pub fn parent_is(key: &String, parent: &String) -> bool {
    let mut p: Option<String> = crate::args::parent_key(key);
    if p.is_none() {
        return false;
    };
    let mut p_1 = p.as_ref().unwrap();
    return (&p_1[..] == &parent[..]);
}

pub fn in_group(mut group: Option<&String>, key: &String) -> bool {
    if group.is_none() {
        return false;
    };
    let mut group_1 = group.unwrap();
    return (&group_1[..] == &key[..]);
}

pub fn given_in(spec: &crate::args::Spec, hits: &Vec<crate::args::Hit>, key: &String) -> Option<String> {
    for mut h in hits.iter() {
        for mut o in spec.opts.iter() {
            if ((&o.field[..] == &h.field[..]) && !(o.group.is_none())) {
                let mut __narrowed_1 = o.group.as_ref().unwrap();
                let mut g: String = (__narrowed_1).clone();
                if ((&g[..] == &key[..]) || crate::core_string::starts_with_platform(&g, &format!("{}/", key))) {
                    return Some(crate::args::long_name(&o.field));
                };
            };
        }
    }
    return None;
}

pub fn first_option(spec: &crate::args::Spec, key: &String) -> String {
    for mut o in spec.opts.iter() {
        if (o.required && crate::args::in_group((&o.group).as_ref(), key)) {
            return crate::args::long_name(&o.field);
        };
    }
    for mut o in spec.opts.iter() {
        if crate::args::in_group((&o.group).as_ref(), key) {
            return crate::args::long_name(&o.field);
        };
    }
    return (key).clone();
}

pub fn live(spec: &crate::args::Spec, hits: &Vec<crate::args::Hit>, mut key: Option<&String>) -> bool {
    let mut k: Option<String> = (match key.clone() { Some(__v) => Some((__v).clone()), None => None, #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
    loop {
        if !(!(k.is_none())) {
            break;
        };
        let mut k_1 = k.as_ref().unwrap();
        let mut here: String = (k_1).clone();
        for mut g in spec.groups.iter() {
            if (((&g.key[..] == &here[..]) && ((&g.kind[..] == &String::from("together")[..]) || (&g.kind[..] == &String::from("arm")[..]))) && (crate::args::given_in(spec, hits, &here)).is_none()) {
                return false;
            };
        }
        k = crate::args::parent_key(&here);
    }
    return true;
}

pub fn unmet_group(spec: &crate::args::Spec, hits: &Vec<crate::args::Hit>, mut key: Option<&String>) -> Option<String> {
    let mut k: Option<String> = (match key.clone() { Some(__v) => Some((__v).clone()), None => None, #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
    loop {
        if !(!(k.is_none())) {
            break;
        };
        let mut k_1 = k.as_ref().unwrap();
        let mut here: String = (k_1).clone();
        for mut g in spec.groups.iter() {
            if ((&g.key[..] == &here[..]) && ((&g.kind[..] == &String::from("together")[..]) || (&g.kind[..] == &String::from("arm")[..]))) {
                let mut why: Option<String> = crate::args::given_in(spec, hits, &here);
                if (!(why.is_none()) && crate::args::live(spec, hits, (&(crate::args::parent_key(&here))).as_ref())) {
                    let mut why_2 = why.as_ref().unwrap();
                    return Some(why_2.clone());
                };
                return None;
            };
        }
        k = crate::args::parent_key(&here);
    }
    return None;
}

pub fn has_command(spec: &crate::args::Spec, name: &String) -> bool {
    for mut c in spec.commands.iter() {
        if (&c.name[..] == &name[..]) {
            return true;
        };
    }
    return false;
}

pub fn is_command(found: &crate::args::Found, name: &String) -> bool {
    let mut c = &found.command;
    if c.is_none() {
        return false;
    };
    let mut c_1 = c.as_ref().unwrap();
    return (&c_1[..] == &name[..]);
}

pub fn long_opt(spec: &crate::args::Spec, name: &String) -> Option<crate::args::OptSpec> {
    for mut o in spec.opts.iter() {
        if (&crate::args::long_name(&o.field)[..] == &format!("--{}", name)[..]) {
            return Some((o).clone());
        };
    }
    return None;
}

pub fn short_opt(spec: &crate::args::Spec, mut c: char) -> Option<crate::args::OptSpec> {
    for mut o in spec.opts.iter() {
        if {
            let mut __cmp_1 = &o.short;
            if __cmp_1.is_some() {
                let mut __arm_2 = __cmp_1.unwrap();
                (__arm_2 == c)
            } else {
                false
            }
        } {
            return Some((o).clone());
        };
    }
    return None;
}

pub fn word_at(tokens: &Vec<crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>>, mut i: i32) -> Option<String> {
    let mut t: Option<&crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>> = crate::core_list::get_platform::<crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken>>(tokens, i);
    if t.is_none() {
        return None;
    };
    let mut t_1 = t.unwrap();
    let mut w: crate::unions::Union4<crate::args_lex::LongToken, crate::args_lex::ShortToken, crate::args_lex::WordToken, crate::args_lex::EndToken> = (t_1).clone();
    if matches!(w, crate::unions::Union4::U3(_)) {
        let mut w_2 = match &w { crate::unions::Union4::U3(__v) => __v, _ => unreachable!() };
        return Some(w_2.value.clone());
    };
    return None;
}

pub fn given(found: &crate::args::Found, field: &String) -> bool {
    for mut h in found.hits.iter() {
        if (&h.field[..] == &field[..]) {
            return true;
        };
    }
    return false;
}

pub fn value_of(found: &crate::args::Found, field: &String) -> Option<String> {
    for mut h in found.hits.iter() {
        if (&h.field[..] == &field[..]) {
            return Some((h.value).clone());
        };
    }
    return None;
}

pub fn values_of(found: &crate::args::Found, field: &String) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for mut h in found.hits.iter() {
        if (&h.field[..] == &field[..]) {
            crate::core_list::add_platform::<String>(&mut out, (h.value).clone());
        };
    }
    return out;
}

pub fn usage(spec: &crate::args::Spec) -> String {
    let mut out: String = crate::core_string::Mut_Str(vec![format!("Usage: {}", spec.name)]);
    if !(crate::core_list::is_empty::<crate::args::OptSpec>(&spec.opts)) {
        crate::core_string::append_platform(&mut out, &String::from(" [OPTIONS]"));
    };
    let mut none: Vec<crate::args::Hit> = vec![];
    for mut o in spec.opts.iter() {
        if (o.required && crate::args::live(spec, &none, (&o.group).as_ref())) {
            crate::core_string::append_platform(&mut out, &format!(" {} <{}>", crate::args::long_name(&o.field), crate::args::value_name(&o.field)));
        };
    }
    for mut a in spec.args.iter() {
        let mut name: String = crate::args::arg_name(&a.field);
        let mut shown: String = if a.many {
            format!("{}...", name)
        } else {
            name.clone()
        };
        crate::core_string::append_platform(&mut out, &if a.required {
            format!(" {}", shown)
        } else {
            format!(" [{}]", shown)
        });
    }
    if !(crate::core_list::is_empty::<crate::args::CmdSpec>(&spec.commands)) {
        crate::core_string::append_platform(&mut out, &if spec.command_required {
            String::from(" <COMMAND>")
        } else {
            String::from(" [COMMAND]")
        });
    };
    return out;
}

pub fn value_name(field: &String) -> String {
    return crate::core_string::to_upper_platform(&crate::core_string::replace_platform(field, &String::from("_"), &String::from("-")));
}

pub fn help_text(spec: &crate::args::Spec) -> String {
    let mut out: String = crate::core_string::Mut_Str(vec![String::from("")]);
    if !(spec.about.is_none()) {
        let mut __narrowed_1 = spec.about.as_ref().unwrap();
        crate::core_string::append_platform(&mut out, &format!("{}\n\n", __narrowed_1));
    };
    crate::core_string::append_platform(&mut out, &format!("{}\n", crate::args::usage(spec)));
    let mut lefts: Vec<String> = vec![];
    let mut rights: Vec<String> = vec![];
    for mut a in spec.args.iter() {
        crate::core_list::add_platform::<String>(&mut lefts, if a.many {
            format!("{}...", crate::args::arg_name(&a.field))
        } else {
            crate::args::arg_name(&a.field)
        });
        crate::core_list::add_platform::<String>(&mut rights, crate::args::one_line((&a.help).as_ref()));
    }
    let mut nargs: i32 = crate::core_list::size_platform::<String>(&lefts);
    for mut c in spec.commands.iter() {
        crate::core_list::add_platform::<String>(&mut lefts, (c.name).clone());
        crate::core_list::add_platform::<String>(&mut rights, crate::args::one_line((&c.about).as_ref()));
    }
    let mut ncommands: i32 = i32::wrapping_sub(crate::core_list::size_platform::<String>(&lefts), nargs);
    let mut sections: Vec<String> = vec![];
    let mut section_of: Vec<String> = vec![];
    for mut o in spec.opts.iter() {
        let mut heading: String = crate::args::section(spec, (&o.group).as_ref());
        if (!((&heading[..] == &String::from("")[..])) && !(crate::core_list::contains::<String>(&sections, &heading, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])))) {
            crate::core_list::add_platform::<String>(&mut sections, (heading).clone());
        };
        crate::core_list::add_platform::<String>(&mut section_of, heading);
    }
    for mut o in spec.opts.iter() {
        let mut left: String = crate::core_string::Mut_Str(vec![if o.short.is_none() {
            String::from("    ")
        } else {
            let mut __narrowed_2 = o.short.unwrap();
            format!("-{}, ", __narrowed_2)
        }]);
        crate::core_string::append_platform(&mut left, &crate::args::long_name(&o.field));
        if o.takes_value {
            crate::core_string::append_platform(&mut left, &format!(" <{}>", crate::args::value_name(&o.field)));
        };
        crate::core_list::add_platform::<String>(&mut lefts, (left).clone());
        let mut right: String = crate::core_string::Mut_Str(vec![crate::args::one_line((&o.help).as_ref())]);
        if !(crate::core_list::is_empty::<String>(&o.choices)) {
            crate::core_string::append_platform(&mut right, &format!(" [possible values: {}]", crate::core_string::join_platform(&o.choices, &String::from(", "))));
        };
        if !(o.default_text.is_none()) {
            let mut __narrowed_3 = o.default_text.as_ref().unwrap();
            crate::core_string::append_platform(&mut right, &format!(" [default: {}]", __narrowed_3));
        };
        crate::core_list::add_platform::<String>(&mut rights, crate::core_string::trim_platform(&right));
    }
    crate::core_list::add_platform::<String>(&mut lefts, String::from("-h, --help"));
    crate::core_list::add_platform::<String>(&mut rights, String::from("Print help"));
    let mut width: i32 = 0i32;
    for mut l in lefts.iter() {
        if (crate::core_string::size_platform(l) > width) {
            width = crate::core_string::size_platform(l);
        };
    }
    let mut help_row: i32 = i32::wrapping_sub(crate::core_list::size_platform::<String>(&lefts), 1i32);
    let mut rows: Vec<i32> = vec![];
    if (nargs > 0i32) {
        crate::core_string::append_platform(&mut out, &String::from("\nArguments:\n"));
        let mut i: i32 = 0i32;
        loop {
            if !((i < nargs)) {
                break;
            };
            crate::core_list::add_platform::<i32>(&mut rows, i);
            i = i32::wrapping_add(i, 1i32);
        }
        crate::args::help_rows(&mut out, &lefts, &rights, &rows, width);
    };
    if (ncommands > 0i32) {
        crate::core_string::append_platform(&mut out, &String::from("\nCommands:\n"));
        let mut commands: Vec<i32> = vec![];
        let mut c: i32 = 0i32;
        loop {
            if !((c < ncommands)) {
                break;
            };
            crate::core_list::add_platform::<i32>(&mut commands, i32::wrapping_add(nargs, c));
            c = i32::wrapping_add(c, 1i32);
        }
        crate::args::help_rows(&mut out, &lefts, &rights, &commands, width);
    };
    let mut first_opt: i32 = i32::wrapping_add(nargs, ncommands);
    let mut mine: Vec<i32> = vec![];
    let mut k: i32 = 0i32;
    for mut s in section_of.iter() {
        if (&s[..] == &String::from("")[..]) {
            crate::core_list::add_platform::<i32>(&mut mine, i32::wrapping_add(first_opt, k));
        };
        k = i32::wrapping_add(k, 1i32);
    }
    crate::core_list::add_platform::<i32>(&mut mine, help_row);
    crate::core_string::append_platform(&mut out, &String::from("\nOptions:\n"));
    crate::args::help_rows(&mut out, &lefts, &rights, &mine, width);
    for mut heading in sections.iter() {
        let mut these: Vec<i32> = vec![];
        let mut j: i32 = 0i32;
        for mut s in section_of.iter() {
            if (&s[..] == &heading[..]) {
                crate::core_list::add_platform::<i32>(&mut these, i32::wrapping_add(first_opt, j));
            };
            j = i32::wrapping_add(j, 1i32);
        }
        crate::core_string::append_platform(&mut out, &format!("\n{}:\n", heading));
        crate::args::help_rows(&mut out, &lefts, &rights, &these, width);
    }
    return out;
}

pub fn one_line(mut text: Option<&String>) -> String {
    if text.is_none() {
        return String::from("");
    };
    let mut text_1 = text.unwrap();
    return crate::core_string::replace_platform(text_1, &String::from("\n"), &String::from(" "));
}

pub fn help_rows(out: &mut String, lefts: &Vec<String>, rights: &Vec<String>, rows: &Vec<i32>, mut width: i32) {
    for mut i in rows.iter().copied() {
        let mut l: &String = {
            let mut __nn_1: Option<&String> = crate::core_list::get_platform::<String>(lefts, i);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at args:844:17");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        };
        let mut r: &String = {
            let mut __nn_3: Option<&String> = crate::core_list::get_platform::<String>(rights, i);
            if __nn_3.is_none() {
                panic!("salvo: value is absent at args:845:17");
            } else {
                let mut __some_4 = __nn_3.unwrap();
                __some_4
            }
        };
        if crate::core_string::is_empty(r) {
            crate::core_string::append_platform(&mut *out, &format!("  {}\n", l));
        } else {
            crate::core_string::append_platform(&mut *out, &format!("  {}{}  {}\n", l, crate::core_string::repeat(&String::from(" "), i32::wrapping_sub(width, crate::core_string::size_platform(l))), r));
        };
    }
}

pub fn section(spec: &crate::args::Spec, mut group: Option<&String>) -> String {
    if group.is_none() {
        return String::from("");
    };
    let mut group_1 = group.unwrap();
    let mut top: String = (group_1).clone();
    let mut cut: Option<(String, String)> = crate::core_string::split_once(&top, &String::from("/"));
    if !(cut.is_none()) {
        let mut cut_2 = cut.as_ref().unwrap();
        let mut __destructured_3: (String, String) = cut_2.clone();
        let mut head: String = __destructured_3.0;
        let mut __unused_4: String = __destructured_3.1;
        top = head.clone();
    };
    for mut g in spec.groups.iter() {
        if ((&g.key[..] == &top[..]) && !(g.heading.is_none())) {
            let mut __narrowed_5 = g.heading.as_ref().unwrap();
            return crate::core_string::trim_suffix_platform(&crate::args::one_line(Some(__narrowed_5)), &String::from("."));
        };
    }
    return String::from("");
}

pub fn parse_args<T: Clone>(args: &Vec<String>, arg_spec: &mut dyn FnMut() -> crate::args::Spec, from_found: &mut dyn FnMut(&crate::args::Found) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> {
    return crate::args::parse_named::<T>(args, None, &mut *arg_spec, &mut *from_found);
}

pub fn parse_named<T: Clone>(args: &Vec<String>, mut prefix: Option<String>, arg_spec: &mut dyn FnMut() -> crate::args::Spec, from_found: &mut dyn FnMut(&crate::args::Found) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> {
    let mut base: crate::args::Spec = arg_spec();
    crate::args::check_spec(&base);
    let mut spec: crate::args::Spec = if prefix.is_none() {
        base.clone()
    } else {
        let mut prefix_1 = prefix.as_ref().unwrap();
        {
            let mut __spread_2: crate::args::Spec = base.clone();
            crate::args::Spec { name: format!("{} {}", prefix_1, base.name), about: __spread_2.about, opts: __spread_2.opts, args: __spread_2.args, groups: __spread_2.groups, commands: __spread_2.commands, command_required: __spread_2.command_required, command_fields: __spread_2.command_fields }
        }
    };
    let mut found: crate::args::Found = crate::args::scan(&spec, args);
    if found.help {
        let mut help: crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested> = crate::unions::Union11::U11(crate::args::HelpRequested { text: crate::args::help_text(&spec) });
        return crate::unions::Union2::U2(crate::core_result::errs::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(help));
    };
    let mut r: crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> = from_found(&found);
    if matches!(r, crate::unions::Union2::U2(_)) {
        let mut r_3 = match &r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        for mut e in r_3.iter() {
            let mut one: crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested> = (e).clone();
            if matches!(one, crate::unions::Union11::U11(_)) {
                let mut one_4 = match &one { crate::unions::Union11::U11(__v) => __v, _ => unreachable!() };
                let mut help: crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested> = crate::unions::Union11::U11(one_4.clone());
                return crate::unions::Union2::U2(crate::core_result::errs::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(help));
            };
        }
    };
    return r;
}

pub fn args_or_exit<T: Clone>(console: &crate::core_console::Console, process: &crate::process::Process, args: &Vec<String>, arg_spec: &mut dyn FnMut() -> crate::args::Spec, from_found: &mut dyn FnMut(&crate::args::Found) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) -> T {
    let mut parsed: crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> = crate::args::parse_args::<T>(args, &mut *arg_spec, &mut *from_found);
    return if matches!(parsed, crate::unions::Union2::U1(_)) {
        let mut parsed_1 = match parsed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return parsed_1;
    } else if matches!(parsed, crate::unions::Union2::U2(_)) {
        let mut parsed_2 = match &parsed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        { crate::args::exit_with::<T>(console, process, parsed_2, &mut *arg_spec, &mut *from_found); unreachable!() }
    } else {
        panic!("salvo: unhandled union arm at args:978:5");
    };
}

pub fn parse_valid<T: Clone, V: Clone>(args: &Vec<String>, validate: &mut dyn FnMut(T) -> std::ops::ControlFlow<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>, V>, arg_spec: &mut dyn FnMut() -> crate::args::Spec, from_found: &mut dyn FnMut(&crate::args::Found) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) -> crate::unions::Union2<V, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> {
    let mut parsed: crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> = crate::args::parse_args::<T>(args, &mut *arg_spec, &mut *from_found);
    return if matches!(parsed, crate::unions::Union2::U2(_)) {
        let mut parsed_1 = match &parsed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(parsed_1.clone());
    } else if matches!(parsed, crate::unions::Union2::U1(_)) {
        let mut parsed_2 = match parsed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut checked: crate::unions::Union2<V, crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>> = ('try_1: {
            crate::unions::Union2::U1((match validate(parsed_2) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => break 'try_1 crate::unions::Union2::U2(__m) }))
        });
        if matches!(checked, crate::unions::Union2::U1(_)) {
            let mut checked_3 = match checked { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::Ok::<V>(checked_3));
        } else if matches!(checked, crate::unions::Union2::U2(_)) {
            let mut checked_4 = match checked { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            let mut e: crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested> = checked_4;
            return crate::unions::Union2::U2(crate::core_result::errs::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(e));
        } else {
            panic!("salvo: unhandled union arm at args:1010:13");
        }
    } else {
        panic!("salvo: unhandled union arm at args:1004:5");
    };
}

pub fn valid_args_or_exit<T: Clone, V: Clone>(console: &crate::core_console::Console, process: &crate::process::Process, args: &Vec<String>, validate: &mut dyn FnMut(T) -> std::ops::ControlFlow<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>, V>, arg_spec: &mut dyn FnMut() -> crate::args::Spec, from_found: &mut dyn FnMut(&crate::args::Found) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) -> V {
    let mut parsed: crate::unions::Union2<V, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>> = crate::args::parse_valid::<T, V>(args, &mut *validate, &mut *arg_spec, &mut *from_found);
    return if matches!(parsed, crate::unions::Union2::U1(_)) {
        let mut parsed_1 = match parsed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return parsed_1;
    } else if matches!(parsed, crate::unions::Union2::U2(_)) {
        let mut parsed_2 = match &parsed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        { crate::args::exit_with::<T>(console, process, parsed_2, &mut *arg_spec, &mut *from_found); unreachable!() }
    } else {
        panic!("salvo: unhandled union arm at args:1026:5");
    };
}

pub fn exit_with<T: Clone>(console: &crate::core_console::Console, process: &crate::process::Process, errors: &Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>, arg_spec: &mut dyn FnMut() -> crate::args::Spec, from_found: &mut dyn FnMut(&crate::args::Found) -> crate::unions::Union2<T, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) {
    for mut e in errors.iter() {
        let mut one: crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested> = (e).clone();
        if matches!(one, crate::unions::Union11::U11(_)) {
            let mut one_1 = match &one { crate::unions::Union11::U11(__v) => __v, _ => unreachable!() };
            console.print(&one_1.text);
            process.exit(0i32);
        };
    }
    for mut e in errors.iter() {
        console.printerr(&format!("error: {}\n", crate::args::message((&((e).clone())).as_ref())));
    }
    let mut spec: crate::args::Spec = arg_spec();
    console.printerr(&format!("\n{}\n\nFor more information, try '--help'.\n", crate::args::usage(&spec)));
    return { process.exit(2i32); unreachable!() };
}

pub fn check_spec(spec: &crate::args::Spec) {
    let mut shorts: Vec<char> = vec![];
    let mut fields: Vec<String> = vec![];
    for mut o in spec.opts.iter() {
        if crate::core_list::contains::<String>(&fields, &o.field, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])) {
            panic!("salvo: {} at args:1062:13", format!("two options of `{}` are spelled `{}`", spec.name, crate::args::long_name(&o.field)));
        };
        crate::core_list::add_platform::<String>(&mut fields, (o.field).clone());
        if (&o.field[..] == &String::from("help")[..]) {
            panic!("salvo: {} at args:1066:13", format!("`--help` is the parser's own: name the field of `{}` something else", spec.name));
        };
        let mut s = &o.short;
        if !(s.is_none()) {
            let mut s_1 = s.unwrap();
            if (s_1 == 'h') {
                panic!("salvo: {} at args:1071:17", format!("`Short('h')` on `{}` takes help's short name", o.field));
            };
            let mut s_2 = s.unwrap();
            if crate::core_list::contains::<char>(&shorts, &s_2, &mut |__a0: &char, __a1: &char| ((*__a0) == (*__a1))) {
                panic!("salvo: {} at args:1074:17", format!("two options of `{}` have the short name `-{}`", spec.name, s_2));
            };
            let mut s_3 = s.unwrap();
            crate::core_list::add_platform::<char>(&mut shorts, s_3);
        };
    }
    for mut g in spec.groups.iter() {
        if (&g.kind[..] == &String::from("arm")[..]) {
            let mut count: i32 = 0i32;
            let mut required: bool = false;
            for mut o in spec.opts.iter() {
                if crate::args::in_group((&o.group).as_ref(), &g.key) {
                    count = i32::wrapping_add(count, 1i32);
                    required = (required || o.required);
                };
            }
            if (((count > 0i32) && !(required)) && !(((count == 1i32) && !(crate::args::opt_takes_value(spec, &g.key))))) {
                panic!("salvo: {} at args:1092:17", format!("every option of `{}` is optional, so nothing chooses that arm: make one required", g.key));
            };
        };
    }
    if (!(crate::core_list::is_empty::<crate::args::CmdSpec>(&spec.commands)) && !(crate::core_list::is_empty::<crate::args::ArgSpec>(&spec.args))) {
        panic!("salvo: {} at args:1098:9", format!("`{}` takes a sub-command and positionals: whether a word is one or the other would depend on its spelling", spec.name));
    };
    if (spec.command_fields > 1i32) {
        panic!("salvo: {} at args:1101:9", format!("`{}` has more than one `Subcommand` field", spec.name));
    };
    let mut names: Vec<String> = vec![];
    for mut c in spec.commands.iter() {
        if crate::core_list::contains::<String>(&names, &c.name, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])) {
            panic!("salvo: {} at args:1106:13", format!("two sub-commands of `{}` are named `{}`", spec.name, c.name));
        };
        crate::core_list::add_platform::<String>(&mut names, (c.name).clone());
    }
    let mut optional: bool = false;
    let mut many: bool = false;
    for mut a in spec.args.iter() {
        if many {
            panic!("salvo: {} at args:1114:13", format!("positional `{}` follows one that takes every remaining word", a.field));
        };
        if (a.required && optional) {
            panic!("salvo: {} at args:1117:13", format!("required positional `{}` follows an optional one", a.field));
        };
        optional = (optional || !(a.required));
        many = a.many;
    }
}

pub fn opt_takes_value(spec: &crate::args::Spec, key: &String) -> bool {
    for mut o in spec.opts.iter() {
        if crate::args::in_group((&o.group).as_ref(), key) {
            return o.takes_value;
        };
    }
    return true;
}

pub fn eval<Q: Clone>(f: impl FnOnce() -> Q) -> Q {
    return f();
}

pub fn read_value<B: Clone, Q: Clone>(found: &crate::args::Found, field: &String, mut default: Option<Q>, refine: &mut dyn FnMut(B) -> Option<Q>, from_str: &mut dyn FnMut(&String) -> Option<B>) -> Q {
    let mut s: Option<String> = crate::args::value_of(found, field);
    if !(s.is_none()) {
        let mut s_1 = s.as_ref().unwrap();
        let mut v: Option<B> = from_str(s_1);
        if !(v.is_none()) {
            let mut v_2 = v.unwrap();
            let mut q: Option<Q> = refine(v_2);
            if !(q.is_none()) {
                let mut q_3 = q.unwrap();
                return q_3;
            };
        };
        panic!("salvo: {} at args:1156:9", format!("`{}`'s value was checked", field));
    };
    if !(default.is_none()) {
        let mut default_4 = default.unwrap();
        return default_4;
    };
    return panic!("salvo: {} at args:1161:5", format!("`{}` was checked as required", field));
}

pub fn read_optional<E: Clone>(found: &crate::args::Found, field: &String, from_str: &mut dyn FnMut(&String) -> Option<E>) -> Option<E> {
    let mut s: Option<String> = crate::args::value_of(found, field);
    if s.is_none() {
        return None;
    };
    let mut s_1 = s.as_ref().unwrap();
    return from_str(s_1);
}

pub fn read_all<E: Clone>(found: &crate::args::Found, field: &String, from_str: &mut dyn FnMut(&String) -> Option<E>) -> Vec<E> {
    let mut out: Vec<E> = vec![];
    for mut s in crate::args::values_of(found, field).into_iter() {
        let mut v: Option<E> = from_str(&(s).clone());
        if !(v.is_none()) {
            let mut v_1 = v.unwrap();
            crate::core_list::add_platform::<E>(&mut out, v_1);
        };
    }
    return out;
}

pub fn check_value<B: Clone>(errors: &mut Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>, found: &crate::args::Found, field: &String, shown: &String, what: &String, test: &mut dyn FnMut(&B) -> Option<String>, from_str: &mut dyn FnMut(&String) -> Option<B>) {
    for mut s in crate::args::values_of(found, field).into_iter() {
        let mut v: Option<B> = from_str(&(s).clone());
        if v.is_none() {
            crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut *errors, crate::unions::Union11::U4(crate::args::Invalid { what: (shown).clone(), value: (s).clone(), reason: format!("expected {}", what) }));
        } else {
            let mut v_1 = v.as_ref().unwrap();
            let mut why: Option<String> = test(v_1);
            if !(why.is_none()) {
                let mut why_2 = why.as_ref().unwrap();
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut *errors, crate::unions::Union11::U4(crate::args::Invalid { what: (shown).clone(), value: (s).clone(), reason: why_2.clone() }));
            };
        };
    }
}

pub fn shown_name(field: &String, mut positional: bool) -> String {
    if positional {
        return crate::args::arg_name(field);
    };
    return crate::args::long_name(field);
}

pub fn literal_text(name: &String) -> String {
    return crate::core_string::trim_suffix_platform(&crate::core_string::trim_prefix_platform(name, &String::from("\"")), &String::from("\""));
}

pub fn group_errors<C: Clone>(errors: &mut Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>, mut r: crate::unions::Union2<C, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>, mut skip: i32) {
    if matches!(r, crate::unions::Union2::U2(_)) {
        let mut r_1 = match &r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut i: i32 = 0i32;
        for mut e in r_1.iter() {
            if (i >= skip) {
                crate::core_list::add_platform::<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>(&mut *errors, (e).clone());
            };
            i = i32::wrapping_add(i, 1i32);
        }
    };
}

pub fn built<C: Clone>(mut r: crate::unions::Union2<C, Vec<crate::unions::Union11<crate::args::UnknownOption, crate::args::MissingValue, crate::args::Missing, crate::args::Invalid, crate::args::Unexpected, crate::args::Repeated, crate::args::Requires, crate::args::Conflict, crate::args::MissingOne, crate::args::UnknownCommand, crate::args::HelpRequested>>>) -> C {
    return if matches!(r, crate::unions::Union2::U1(_)) {
        let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return r_1;
    } else if matches!(r, crate::unions::Union2::U2(_)) {
        let mut r_2 = match &r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        panic!("salvo: a group failed after its checks passed at args:1237:19")
    } else {
        panic!("salvo: unhandled union arm at args:1235:5");
    };
}
