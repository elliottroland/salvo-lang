#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "std/args.rs"]
pub mod std_args;
#[path = "std/args/lex.rs"]
pub mod std_args_lex;
#[path = "std/core/basic.rs"]
pub mod std_core_basic;
#[path = "std/core/checked.rs"]
pub mod std_core_checked;
#[path = "std/core/compare.rs"]
pub mod std_core_compare;
#[path = "std/core/console.rs"]
pub mod std_core_console;
#[path = "std/core/index.rs"]
pub mod std_core_index;
#[path = "std/core/iterator.rs"]
pub mod std_core_iterator;
#[path = "std/core/list.rs"]
pub mod std_core_list;
#[path = "std/core/nonempty.rs"]
pub mod std_core_nonempty;
#[path = "std/core/range.rs"]
pub mod std_core_range;
#[path = "std/core/result.rs"]
pub mod std_core_result;
#[path = "std/core/string.rs"]
pub mod std_core_string;
#[path = "std/process.rs"]
pub mod std_process;
#[path = "std/throw.rs"]
pub mod std_throw;
#[path = "platform/std/args.rs"]
pub mod platform_std_args;
#[path = "platform/std/core/console.rs"]
pub mod platform_std_core_console;
#[path = "platform/std/core/list.rs"]
pub mod platform_std_core_list;
#[path = "platform/std/core/string.rs"]
pub mod platform_std_core_string;
#[path = "platform/std/process.rs"]
pub mod platform_std_process;

use crate::std_args::ArgSpec;
use crate::std_args::CmdSpec;
use crate::std_args::Conflict;
use crate::std_core_console::Console;
use crate::std_core_result::Errs;
use crate::std_args::Found;
use crate::std_args::GroupSpec;
use crate::std_args::HelpRequested;
use crate::std_core_range::InRange__Int_qualifies;
use crate::std_args::Invalid;
use crate::std_core_list::List__T_TArray;
use crate::std_args::Missing;
use crate::std_args::MissingOne;
use crate::std_args::MissingValue;
use crate::std_core_nonempty::NonEmpty_qualifies;
use crate::std_core_result::Ok;
use crate::std_args::OptSpec;
use crate::std_args::Repeated;
use crate::std_args::Requires;
use crate::std_args::Spec;
use crate::std_args::Unexpected;
use crate::std_args::UnknownCommand;
use crate::std_args::UnknownOption;
use crate::std_args::add_one;
use crate::std_core_list::add_platform;
use crate::std_args::any_given;
use crate::std_args::args_or_exit;
use crate::std_args::built;
use crate::std_args::check_value;
use crate::std_args::command_name;
use crate::std_args::eval;
use crate::std_core_string::from_str__Str_IntOpt;
use crate::std_core_string::from_str__Str_StrOpt;
use crate::std_args::given;
use crate::std_args::group_errors;
use crate::std_args::is_command;
use crate::std_core_list::is_empty;
use crate::std_core_string::join_platform;
use crate::std_args::literal_text;
use crate::std_args::merge_group;
use crate::std_args::message;
use crate::std_args::parse_args;
use crate::std_args::parse_named;
use crate::std_core_console::println;
use crate::std_args::read_all;
use crate::std_args::read_optional;
use crate::std_args::read_value;
use crate::std_args::shown_name;
use crate::std_core_list::size_platform;
use crate::std_args::value_of;


#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub verbose: bool,
    pub command: Option<crate::unions::Union2<crate::Serve, crate::Fetch>>,
}

impl crate::wire::__Wire for Tool {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.verbose, out);
        crate::wire::__Wire::__enc(&self.command, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            verbose: crate::wire::__Wire::__dec(r)?,
            command: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Serve {
    pub net: crate::Net,
    pub port: i32,
    pub output: String,
    pub tls: Option<crate::Tls>,
    pub files: Vec<String>,
}

impl crate::wire::__Wire for Serve {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.net, out);
        crate::wire::__Wire::__enc(&self.port, out);
        crate::wire::__Wire::__enc(&self.output, out);
        crate::wire::__Wire::__enc(&self.tls, out);
        crate::wire::__Wire::__enc(&self.files, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            net: crate::wire::__Wire::__dec(r)?,
            port: crate::wire::__Wire::__dec(r)?,
            output: crate::wire::__Wire::__dec(r)?,
            tls: crate::wire::__Wire::__dec(r)?,
            files: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Net {
    pub host: String,
    pub workers: Option<i32>,
}

impl crate::wire::__Wire for Net {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.host, out);
        crate::wire::__Wire::__enc(&self.workers, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            host: crate::wire::__Wire::__dec(r)?,
            workers: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tls {
    pub cert: String,
    pub key: String,
}

impl crate::wire::__Wire for Tls {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.cert, out);
        crate::wire::__Wire::__enc(&self.key, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            cert: crate::wire::__Wire::__dec(r)?,
            key: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fetch {
    pub source: crate::unions::Union3<crate::FromUrl, crate::FromFile, crate::FromStdin>,
    pub header: Vec<String>,
}

impl crate::wire::__Wire for Fetch {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.source, out);
        crate::wire::__Wire::__enc(&self.header, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            source: crate::wire::__Wire::__dec(r)?,
            header: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FromUrl {
    pub url: String,
    pub timeout: i32,
}

impl crate::wire::__Wire for FromUrl {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.url, out);
        crate::wire::__Wire::__enc(&self.timeout, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            url: crate::wire::__Wire::__dec(r)?,
            timeout: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FromFile {
    pub file: String,
}

impl crate::wire::__Wire for FromFile {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.file, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            file: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FromStdin {
}

impl crate::wire::__Wire for FromStdin {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub fn describe(t: &crate::Tool) -> String {
    let mut c = &t.command;
    return if matches!(c, Some(crate::unions::Union2::U1(_))) {
        let mut c_1 = match &c { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
        let mut tls = &c_1.tls;
        let mut secure: String = if tls.is_none() {
            String::from("plain")
        } else {
            let mut tls_2 = tls.as_ref().unwrap();
            format!("tls {}", tls_2.cert)
        };
        return format!("serve {} on {}:{} as {}, {}, {} worker(s)", crate::std_core_string::join_platform(&c_1.files, &String::from(",")), c_1.net.host, c_1.port, c_1.output, secure, if c_1.net.workers.is_some() {
            let mut __narrowed_3 = c_1.net.workers.unwrap();
            __narrowed_3
        } else {
            1i32
        });
    } else if matches!(c, Some(crate::unions::Union2::U2(_))) {
        let mut c_4 = match &c { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
        let mut s = &c_4.source;
        let mut from: String = if matches!(s, crate::unions::Union3::U1(_)) {
            let mut s_5 = match &s { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
            format!("{} (timeout {}s)", s_5.url, s_5.timeout)
        } else if matches!(s, crate::unions::Union3::U2(_)) {
            let mut s_6 = match &s { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
            format!("file {}", s_6.file)
        } else if matches!(s, crate::unions::Union3::U3(_)) {
            let mut s_7 = match &s { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
            String::from("standard input")
        } else {
            panic!("salvo: unhandled union arm at main:104:24");
        };
        return format!("fetch {} with {} header(s)", from, crate::std_core_list::size_platform::<String>(&c_4.header));
    } else if c.is_none() {
        return String::from("nothing to do");
    } else {
        panic!("salvo: unhandled union arm at main:96:5");
    };
}

pub fn show(console: &crate::std_core_console::Console, line: &Vec<String>) {
    crate::std_core_console::println(console, &format!("$ tool {}", crate::std_core_string::join_platform(line, &String::from(" "))));
    let mut r: crate::unions::Union2<crate::Tool, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::std_args::parse_args::<crate::Tool>(line, &mut || crate::arg_spec__SpecTool(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkToolOrErrsNonEmptyListArgError(__a0));
    if matches!(r, crate::unions::Union2::U1(_)) {
        let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut loud: String = if r_1.verbose {
            String::from(" (verbose)")
        } else {
            String::from("")
        };
        crate::std_core_console::println(console, &format!("  {}{}", crate::describe(r_1), loud));
    } else if matches!(r, crate::unions::Union2::U2(_)) {
        let mut r_2 = match &r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        for mut e in r_2.iter() {
            let mut one: crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested> = (e).clone();
            if matches!(one, crate::unions::Union11::U11(_)) {
                let mut one_3 = match &one { crate::unions::Union11::U11(__v) => __v, _ => unreachable!() };
                console.print(&one_3.text);
            } else {
                let mut one_4 = (match one.clone() { crate::unions::Union11::U1(__v) => crate::unions::Union10::U1(__v), crate::unions::Union11::U2(__v) => crate::unions::Union10::U2(__v), crate::unions::Union11::U3(__v) => crate::unions::Union10::U3(__v), crate::unions::Union11::U4(__v) => crate::unions::Union10::U4(__v), crate::unions::Union11::U5(__v) => crate::unions::Union10::U5(__v), crate::unions::Union11::U6(__v) => crate::unions::Union10::U6(__v), crate::unions::Union11::U7(__v) => crate::unions::Union10::U7(__v), crate::unions::Union11::U8(__v) => crate::unions::Union10::U8(__v), crate::unions::Union11::U9(__v) => crate::unions::Union10::U9(__v), crate::unions::Union11::U10(__v) => crate::unions::Union10::U10(__v), crate::unions::Union11::U11(__v) => unreachable!("salvo: unreachable union arm"), #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
                crate::std_core_console::println(console, &format!("  error: {}", crate::std_args::message((&((match one_4.clone() { crate::unions::Union10::U1(__v) => crate::unions::Union11::U1(__v), crate::unions::Union10::U2(__v) => crate::unions::Union11::U2(__v), crate::unions::Union10::U3(__v) => crate::unions::Union11::U3(__v), crate::unions::Union10::U4(__v) => crate::unions::Union11::U4(__v), crate::unions::Union10::U5(__v) => crate::unions::Union11::U5(__v), crate::unions::Union10::U6(__v) => crate::unions::Union11::U6(__v), crate::unions::Union10::U7(__v) => crate::unions::Union11::U7(__v), crate::unions::Union10::U8(__v) => crate::unions::Union11::U8(__v), crate::unions::Union10::U9(__v) => crate::unions::Union11::U9(__v), crate::unions::Union10::U10(__v) => crate::unions::Union11::U10(__v), #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") }))).as_ref())));
            };
        }
    } else {
        panic!("salvo: unhandled union arm at main:120:5");
    };
}

pub fn main() {
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut __use_3: crate::std_args::__Platform_HostArgs = crate::std_args::__Platform_HostArgs::new();
    let __handle_4 = crate::std_args::Args::shared(__use_3);
    let mut __use_5: crate::std_process::__Platform_HostProcess = crate::std_process::__Platform_HostProcess::new();
    let __handle_6 = crate::std_process::Process::shared(__use_5);
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("serve"), vec![String::from("-p"), String::from("80"), String::from("index.html"), String::from("app.js")]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("-v"), vec![String::from("serve"), String::from("--host=0.0.0.0"), String::from("--workers"), String::from("4"), String::from("--cert"), String::from("c.pem"), String::from("--key"), String::from("k.pem"), String::from("--output"), String::from("json"), String::from("site")]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("fetch"), vec![String::from("--url"), String::from("https://example.com"), String::from("--header"), String::from("Accept: text/html"), String::from("--header"), String::from("X-Id: 7")]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("fetch"), vec![String::from("--from-stdin")]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("serve"), vec![String::from("-p"), String::from("0"), String::from("--output"), String::from("xml"), String::from("--cert"), String::from("c.pem")]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("fetch"), vec![String::from("--url"), String::from("u"), String::from("--file"), String::from("f")]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("deploy"), vec![]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("--help"), vec![]));
    crate::show(&__handle_2, &crate::std_core_list::List__T_TArray::<String>(String::from("serve"), vec![String::from("--help")]));
    let mut tool: crate::Tool = crate::std_args::args_or_exit::<crate::Tool>(&__handle_2, &__handle_6, &__handle_4.args(), &mut || crate::arg_spec__SpecTool(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkToolOrErrsNonEmptyListArgError(__a0));
    crate::std_core_console::println(&__handle_2, &format!("argv: {}", crate::describe(&tool)));
}

pub fn arg_spec__SpecTool() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    switch = true;
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("verbose"), required, Some(String::from("Print more.")));
    };
    let mut short: Option<char> = None;
    short = Some('v');
    let mut short_1 = short.unwrap();
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("verbose"), short: Some(short_1), takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("Print more.")), default_text: default_text, choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    command_fields = i32::wrapping_add(command_fields, 1i32);
    let mut child: crate::std_args::Spec = crate::arg_spec__SpecServe();
    crate::std_core_list::add_platform::<crate::std_args::CmdSpec>(&mut commands, crate::std_args::CmdSpec { name: (child.name).clone(), about: (child.about).clone() });
    let mut child_2: crate::std_args::Spec = crate::arg_spec__SpecFetch();
    crate::std_core_list::add_platform::<crate::std_args::CmdSpec>(&mut commands, crate::std_args::CmdSpec { name: (child_2.name).clone(), about: (child_2.about).clone() });
    let mut name: String = crate::std_args::command_name(&String::from("Tool"));
    return crate::std_args::Spec { name: name, about: Some(String::from("A toy file server.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkToolOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Tool, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut s: crate::std_args::Spec = crate::arg_spec__SpecServe();
    if crate::std_args::is_command(found, &s.name) {
        let mut r: crate::unions::Union2<crate::Serve, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::std_args::parse_named::<crate::Serve>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecServe(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkServeOrErrsNonEmptyListArgError(__a0));
        crate::std_args::group_errors::<crate::Serve>(&mut errors, r, 0i32);
    };
    let mut s_1: crate::std_args::Spec = crate::arg_spec__SpecFetch();
    if crate::std_args::is_command(found, &s_1.name) {
        let mut r: crate::unions::Union2<crate::Fetch, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::std_args::parse_named::<crate::Fetch>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecFetch(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkFetchOrErrsNonEmptyListArgError(__a0));
        crate::std_args::group_errors::<crate::Fetch>(&mut errors, r, 0i32);
    };
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Tool>(crate::Tool { verbose: crate::std_args::eval({ let mut found = found.clone(); move || -> bool {
        return crate::std_args::given(&found, &String::from("verbose"));
    } }), command: crate::std_args::eval({ let mut found = found.clone(); let mut s = s.clone(); move || -> Option<crate::unions::Union2<crate::Serve, crate::Fetch>> {
        let mut s: crate::std_args::Spec = crate::arg_spec__SpecServe();
        if crate::std_args::is_command(&found, &s.name) {
            return Some(crate::unions::Union2::U1(crate::std_args::built::<crate::Serve>(crate::std_args::parse_named::<crate::Serve>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecServe(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkServeOrErrsNonEmptyListArgError(__a0)))));
        };
        let mut s_2: crate::std_args::Spec = crate::arg_spec__SpecFetch();
        if crate::std_args::is_command(&found, &s_2.name) {
            return Some(crate::unions::Union2::U2(crate::std_args::built::<crate::Fetch>(crate::std_args::parse_named::<crate::Fetch>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecFetch(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkFetchOrErrsNonEmptyListArgError(__a0)))));
        };
        return None;
    } }) }));
}

pub fn arg_spec__SpecServe() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut child: crate::std_args::Spec = crate::arg_spec__SpecNet();
    crate::std_args::merge_group(&mut opts, &mut groups, &child, &String::from("net"), &String::from("flat"), None);
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    default_text = Some(format!("{}", {
        let mut __default_1: i32 = 8080i32;
        if !(crate::std_core_range::InRange__Int_qualifies(__default_1, 1i32, 65535i32)) { panic!("salvo: {} at main:36:16", String::from("the default for field `port` is not `InRange(1, 65535) Int`")) };
        __default_1
    }));
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("port"), required, Some(String::from("Port to listen on.")));
    };
    let mut short: Option<char> = None;
    short = Some('p');
    let mut short_2 = short.unwrap();
    let mut default_text_3 = default_text.as_ref().unwrap();
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("port"), short: Some(short_2), takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("Port to listen on.")), default_text: Some(default_text_3.clone()), choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    let mut required_4: bool = false;
    let mut many_5: bool = false;
    let mut switch_6: bool = false;
    let mut default_text_7: Option<String> = None;
    let mut choices_8: Vec<String> = vec![];
    let mut exclusive_9: bool = false;
    default_text_7 = Some(String::from("text"));
    crate::std_core_list::add_platform::<String>(&mut choices_8, crate::std_args::literal_text(&String::from("\"json\"")));
    crate::std_core_list::add_platform::<String>(&mut choices_8, crate::std_args::literal_text(&String::from("\"text\"")));
    if exclusive_9 {
        crate::std_args::add_one(&mut groups, &String::from("output"), required_4, Some(String::from("Output format.")));
    };
    let mut short_10: Option<char> = None;
    let mut default_text_11 = default_text_7.as_ref().unwrap();
    let mut opt_12: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("output"), short: short_10, takes_value: !(switch_6), repeats: many_5, required: (required_4 && !(exclusive_9)), help: Some(String::from("Output format.")), default_text: Some(default_text_11.clone()), choices: choices_8, group: None };
    if (!(exclusive_9) || !(crate::std_core_list::is_empty::<String>(&opt_12.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt_12);
    };
    let mut child_13: crate::std_args::Spec = crate::arg_spec__SpecTls();
    crate::std_args::merge_group(&mut opts, &mut groups, &child_13, &String::from("tls"), &String::from("together"), Some(String::from("TLS (give both or neither).")));
    let mut required_14: bool = false;
    let mut many_15: bool = false;
    let mut switch_16: bool = false;
    let mut default_text_17: Option<String> = None;
    let mut choices_18: Vec<String> = vec![];
    let mut exclusive_19: bool = false;
    many_15 = true;
    required_14 = true;
    if exclusive_19 {
        crate::std_args::add_one(&mut groups, &String::from("files"), required_14, Some(String::from("Files to serve.")));
    };
    let mut short_20: Option<char> = None;
    let mut opt_21: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("files"), short: short_20, takes_value: !(switch_16), repeats: many_15, required: (required_14 && !(exclusive_19)), help: Some(String::from("Files to serve.")), default_text: default_text_17, choices: choices_18, group: None };
    if exclusive_19 {
        panic!("salvo: positional `files` is a choice of option groups at main:49:16");
    };
    crate::std_core_list::add_platform::<crate::std_args::ArgSpec>(&mut args, crate::std_args::ArgSpec { field: (opt_21.field).clone(), help: (opt_21.help).clone(), required: opt_21.required, many: opt_21.repeats });
    let mut name: String = crate::std_args::command_name(&String::from("Serve"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Serve files over HTTP.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkServeOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Serve, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut r: crate::unions::Union2<crate::Net, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkNetOrErrsNonEmptyListArgError(found);
    crate::std_args::group_errors::<crate::Net>(&mut errors, r, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    let mut positional: bool = false;
    let mut shown: String = crate::std_args::shown_name(&String::from("port"), positional);
    let mut what: String = String::from("a valid value");
    what = String::from("an integer");
    crate::std_args::check_value::<i32>(&mut errors, found, &String::from("port"), &shown, &what, &mut |mut v| -> Option<String> {
        if !(crate::std_core_range::InRange__Int_qualifies(*v, 1i32, 65535i32)) {
            Some(format!("{} is not in 1..65535", *v))
        } else {
            None
        }
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    let mut s: crate::std_args::Spec = crate::arg_spec__SpecTls();
    if crate::std_args::any_given(found, &s) {
        let mut r: crate::unions::Union2<crate::Tls, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkTlsOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::Tls>(&mut errors, r, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut positional_1: bool = false;
    positional_1 = true;
    let mut shown_2: String = crate::std_args::shown_name(&String::from("files"), positional_1);
    let mut what_3: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("files"), &shown_2, &what_3, &mut |mut _v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Serve>(crate::Serve { net: crate::std_args::eval({ let mut found = found.clone(); move || -> crate::Net {
        return crate::std_args::built::<crate::Net>(crate::from_found__Found_OkNetOrErrsNonEmptyListArgError(&found));
    } }), port: crate::std_args::eval({ let mut found = found.clone(); move || -> i32 {
        return crate::std_args::read_value::<i32, i32>(&found, &String::from("port"), Some({
            let mut __default_4: i32 = 8080i32;
            if !(crate::std_core_range::InRange__Int_qualifies(__default_4, 1i32, 65535i32)) { panic!("salvo: {} at main:36:16", String::from("the default for field `port` is not `InRange(1, 65535) Int`")) };
            __default_4
        }), &mut |mut v| -> Option<i32> {
            if crate::std_core_range::InRange__Int_qualifies(v, 1i32, 65535i32) {
                Some(v)
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    } }), output: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        let mut v: Option<String> = crate::std_args::value_of(&found, &String::from("output"));
        if !(v.is_none()) {
            if (match &v { Some(__v) => __v == &"json", #[allow(unreachable_patterns)] _ => false }) {
                let mut v_5 = v.unwrap();
                return v_5;
            };
            if (match &v { Some(__v) => __v == &"text", #[allow(unreachable_patterns)] _ => false }) {
                let mut v_6 = v.unwrap();
                return v_6;
            };
        };
        return String::from("text");
    } }), tls: crate::std_args::eval({ let mut found = found.clone(); let mut s = s.clone(); move || -> Option<crate::Tls> {
        let mut s: crate::std_args::Spec = crate::arg_spec__SpecTls();
        if crate::std_args::any_given(&found, &s) {
            return Some(crate::std_args::built::<crate::Tls>(crate::from_found__Found_OkTlsOrErrsNonEmptyListArgError(&found)));
        };
        return None;
    } }), files: crate::std_args::eval({ let mut found = found.clone(); move || -> Vec<String> {
        let mut all: Vec<String> = crate::std_args::read_all::<String>(&found, &String::from("files"), &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
        if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<String>>(&all, &mut |__a0: &Vec<String>| crate::std_core_list::size_platform(__a0)) {
            return all;
        };
        panic!("salvo: `files` was checked as required at main:49:16")
    } }) }));
}

pub fn arg_spec__SpecNet() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    default_text = Some(String::from("127.0.0.1"));
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("host"), required, Some(String::from("Address to bind.")));
    };
    let mut short: Option<char> = None;
    let mut default_text_1 = default_text.as_ref().unwrap();
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("host"), short: short, takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("Address to bind.")), default_text: Some(default_text_1.clone()), choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    let mut required_2: bool = false;
    let mut many_3: bool = false;
    let mut switch_4: bool = false;
    let mut default_text_5: Option<String> = None;
    let mut choices_6: Vec<String> = vec![];
    let mut exclusive_7: bool = false;
    if exclusive_7 {
        crate::std_args::add_one(&mut groups, &String::from("workers"), required_2, Some(String::from("Connections served at once.")));
    };
    let mut short_8: Option<char> = None;
    let mut opt_9: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("workers"), short: short_8, takes_value: !(switch_4), repeats: many_3, required: (required_2 && !(exclusive_7)), help: Some(String::from("Connections served at once.")), default_text: default_text_5, choices: choices_6, group: None };
    if (!(exclusive_7) || !(crate::std_core_list::is_empty::<String>(&opt_9.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt_9);
    };
    let mut name: String = crate::std_args::command_name(&String::from("Net"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Network options.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkNetOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Net, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional: bool = false;
    let mut shown: String = crate::std_args::shown_name(&String::from("host"), positional);
    let mut what: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("host"), &shown, &what, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    let mut positional_1: bool = false;
    let mut shown_2: String = crate::std_args::shown_name(&String::from("workers"), positional_1);
    let mut what_3: String = String::from("a valid value");
    what_3 = String::from("an integer");
    crate::std_args::check_value::<i32>(&mut errors, found, &String::from("workers"), &shown_2, &what_3, &mut |mut _v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Net>(crate::Net { host: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("host"), Some(String::from("127.0.0.1")), &mut |mut v| -> Option<String> {
            if true {
                Some(v.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }), workers: crate::std_args::eval({ let mut found = found.clone(); move || -> Option<i32> {
        return crate::std_args::read_optional::<i32>(&found, &String::from("workers"), &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    } }) }));
}

pub fn arg_spec__SpecTls() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    required = true;
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("cert"), required, Some(String::from("Certificate file.")));
    };
    let mut short: Option<char> = None;
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("cert"), short: short, takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("Certificate file.")), default_text: default_text, choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    let mut required_1: bool = false;
    let mut many_2: bool = false;
    let mut switch_3: bool = false;
    let mut default_text_4: Option<String> = None;
    let mut choices_5: Vec<String> = vec![];
    let mut exclusive_6: bool = false;
    required_1 = true;
    if exclusive_6 {
        crate::std_args::add_one(&mut groups, &String::from("key"), required_1, Some(String::from("Private key file.")));
    };
    let mut short_7: Option<char> = None;
    let mut opt_8: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("key"), short: short_7, takes_value: !(switch_3), repeats: many_2, required: (required_1 && !(exclusive_6)), help: Some(String::from("Private key file.")), default_text: default_text_4, choices: choices_5, group: None };
    if (!(exclusive_6) || !(crate::std_core_list::is_empty::<String>(&opt_8.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt_8);
    };
    let mut name: String = crate::std_args::command_name(&String::from("Tls"));
    return crate::std_args::Spec { name: name, about: None, opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkTlsOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Tls, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional: bool = false;
    let mut shown: String = crate::std_args::shown_name(&String::from("cert"), positional);
    let mut what: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("cert"), &shown, &what, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    let mut positional_1: bool = false;
    let mut shown_2: String = crate::std_args::shown_name(&String::from("key"), positional_1);
    let mut what_3: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("key"), &shown_2, &what_3, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Tls>(crate::Tls { cert: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("cert"), None, &mut |mut v| -> Option<String> {
            if true {
                Some(v.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }), key: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("key"), None, &mut |mut v| -> Option<String> {
            if true {
                Some(v.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }) }));
}

pub fn arg_spec__SpecFetch() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    required = true;
    let mut child: crate::std_args::Spec = crate::arg_spec__SpecFromUrl();
    crate::std_args::merge_group(&mut opts, &mut groups, &child, &String::from("source/FromUrl"), &String::from("arm"), None);
    exclusive = true;
    let mut child_1: crate::std_args::Spec = crate::arg_spec__SpecFromFile();
    crate::std_args::merge_group(&mut opts, &mut groups, &child_1, &String::from("source/FromFile"), &String::from("arm"), None);
    exclusive = true;
    let mut child_2: crate::std_args::Spec = crate::arg_spec__SpecFromStdin();
    crate::std_args::merge_group(&mut opts, &mut groups, &child_2, &String::from("source/FromStdin"), &String::from("arm"), None);
    exclusive = true;
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("source"), required, Some(String::from("Where to read.")));
    };
    let mut short: Option<char> = None;
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("source"), short: short, takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("Where to read.")), default_text: default_text, choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    let mut required_3: bool = false;
    let mut many_4: bool = false;
    let mut switch_5: bool = false;
    let mut default_text_6: Option<String> = None;
    let mut choices_7: Vec<String> = vec![];
    let mut exclusive_8: bool = false;
    many_4 = true;
    if exclusive_8 {
        crate::std_args::add_one(&mut groups, &String::from("header"), required_3, Some(String::from("A header line to send; repeatable.")));
    };
    let mut short_9: Option<char> = None;
    let mut opt_10: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("header"), short: short_9, takes_value: !(switch_5), repeats: many_4, required: (required_3 && !(exclusive_8)), help: Some(String::from("A header line to send; repeatable.")), default_text: default_text_6, choices: choices_7, group: None };
    if (!(exclusive_8) || !(crate::std_core_list::is_empty::<String>(&opt_10.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt_10);
    };
    let mut name: String = crate::std_args::command_name(&String::from("Fetch"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Fetch a document.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFetchOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Fetch, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut s: crate::std_args::Spec = crate::arg_spec__SpecFromUrl();
    if crate::std_args::any_given(found, &s) {
        let mut r: crate::unions::Union2<crate::FromUrl, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkFromUrlOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::FromUrl>(&mut errors, r, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut s_1: crate::std_args::Spec = crate::arg_spec__SpecFromFile();
    if crate::std_args::any_given(found, &s_1) {
        let mut r: crate::unions::Union2<crate::FromFile, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkFromFileOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::FromFile>(&mut errors, r, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut s_2: crate::std_args::Spec = crate::arg_spec__SpecFromStdin();
    if crate::std_args::any_given(found, &s_2) {
        let mut r: crate::unions::Union2<crate::FromStdin, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkFromStdinOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::FromStdin>(&mut errors, r, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut positional: bool = false;
    let mut shown: String = crate::std_args::shown_name(&String::from("header"), positional);
    let mut what: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("header"), &shown, &what, &mut |mut _v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Fetch>(crate::Fetch { source: crate::std_args::eval({ let mut found = found.clone(); let mut s = s.clone(); move || -> crate::unions::Union3<crate::FromUrl, crate::FromFile, crate::FromStdin> {
        let mut s: crate::std_args::Spec = crate::arg_spec__SpecFromUrl();
        if crate::std_args::any_given(&found, &s) {
            return crate::unions::Union3::U1(crate::std_args::built::<crate::FromUrl>(crate::from_found__Found_OkFromUrlOrErrsNonEmptyListArgError(&found)));
        };
        let mut s_3: crate::std_args::Spec = crate::arg_spec__SpecFromFile();
        if crate::std_args::any_given(&found, &s_3) {
            return crate::unions::Union3::U2(crate::std_args::built::<crate::FromFile>(crate::from_found__Found_OkFromFileOrErrsNonEmptyListArgError(&found)));
        };
        let mut s_4: crate::std_args::Spec = crate::arg_spec__SpecFromStdin();
        if crate::std_args::any_given(&found, &s_4) {
            return crate::unions::Union3::U3(crate::std_args::built::<crate::FromStdin>(crate::from_found__Found_OkFromStdinOrErrsNonEmptyListArgError(&found)));
        };
        let mut v: Option<String> = crate::std_args::value_of(&found, &String::from("source"));
        if !(v.is_none()) {
        };
        panic!("salvo: `source` was checked as required at main:73:5")
    } }), header: crate::std_args::eval({ let mut found = found.clone(); move || -> Vec<String> {
        let mut all: Vec<String> = crate::std_args::read_all::<String>(&found, &String::from("header"), &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
        {
            return all;
        };
        panic!("salvo: `header` was checked as required at main:75:5")
    } }) }));
}

pub fn arg_spec__SpecFromUrl() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    required = true;
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("url"), required, Some(String::from("Where to fetch from.")));
    };
    let mut short: Option<char> = None;
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("url"), short: short, takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("Where to fetch from.")), default_text: default_text, choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    let mut required_1: bool = false;
    let mut many_2: bool = false;
    let mut switch_3: bool = false;
    let mut default_text_4: Option<String> = None;
    let mut choices_5: Vec<String> = vec![];
    let mut exclusive_6: bool = false;
    default_text_4 = Some(format!("{}", 30i32));
    if exclusive_6 {
        crate::std_args::add_one(&mut groups, &String::from("timeout"), required_1, Some(String::from("Seconds to wait.")));
    };
    let mut short_7: Option<char> = None;
    let mut default_text_8 = default_text_4.as_ref().unwrap();
    let mut opt_9: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("timeout"), short: short_7, takes_value: !(switch_3), repeats: many_2, required: (required_1 && !(exclusive_6)), help: Some(String::from("Seconds to wait.")), default_text: Some(default_text_8.clone()), choices: choices_5, group: None };
    if (!(exclusive_6) || !(crate::std_core_list::is_empty::<String>(&opt_9.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt_9);
    };
    let mut name: String = crate::std_args::command_name(&String::from("FromUrl"));
    return crate::std_args::Spec { name: name, about: None, opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFromUrlOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::FromUrl, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional: bool = false;
    let mut shown: String = crate::std_args::shown_name(&String::from("url"), positional);
    let mut what: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("url"), &shown, &what, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    let mut positional_1: bool = false;
    let mut shown_2: String = crate::std_args::shown_name(&String::from("timeout"), positional_1);
    let mut what_3: String = String::from("a valid value");
    what_3 = String::from("an integer");
    crate::std_args::check_value::<i32>(&mut errors, found, &String::from("timeout"), &shown_2, &what_3, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::FromUrl>(crate::FromUrl { url: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("url"), None, &mut |mut v| -> Option<String> {
            if true {
                Some(v.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }), timeout: crate::std_args::eval({ let mut found = found.clone(); move || -> i32 {
        return crate::std_args::read_value::<i32, i32>(&found, &String::from("timeout"), Some(30i32), &mut |mut v| -> Option<i32> {
            if true {
                Some(v)
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    } }) }));
}

pub fn arg_spec__SpecFromFile() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut required: bool = false;
    let mut many: bool = false;
    let mut switch: bool = false;
    let mut default_text: Option<String> = None;
    let mut choices: Vec<String> = vec![];
    let mut exclusive: bool = false;
    required = true;
    if exclusive {
        crate::std_args::add_one(&mut groups, &String::from("file"), required, Some(String::from("A file to read instead.")));
    };
    let mut short: Option<char> = None;
    let mut opt: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("file"), short: short, takes_value: !(switch), repeats: many, required: (required && !(exclusive)), help: Some(String::from("A file to read instead.")), default_text: default_text, choices: choices, group: None };
    if (!(exclusive) || !(crate::std_core_list::is_empty::<String>(&opt.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt);
    };
    let mut name: String = crate::std_args::command_name(&String::from("FromFile"));
    return crate::std_args::Spec { name: name, about: None, opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFromFileOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::FromFile, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional: bool = false;
    let mut shown: String = crate::std_args::shown_name(&String::from("file"), positional);
    let mut what: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("file"), &shown, &what, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::FromFile>(crate::FromFile { file: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("file"), None, &mut |mut v| -> Option<String> {
            if true {
                Some(v.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }) }));
}

pub fn arg_spec__SpecFromStdin() -> crate::std_args::Spec {
    let mut opts: Vec<crate::std_args::OptSpec> = vec![];
    let mut args: Vec<crate::std_args::ArgSpec> = vec![];
    let mut groups: Vec<crate::std_args::GroupSpec> = vec![];
    let mut commands: Vec<crate::std_args::CmdSpec> = vec![];
    let mut command_required: bool = false;
    let mut command_fields: i32 = 0i32;
    let mut name: String = crate::std_args::command_name(&String::from("FromStdin"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Read standard input.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFromStdinOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::FromStdin, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    if crate::std_core_nonempty::NonEmpty_qualifies::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(&errors, &mut |__a0: &Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>| crate::std_core_list::size_platform(__a0)) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::FromStdin>(crate::FromStdin {}));
}
