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
#[path = "std/core/map.rs"]
pub mod std_core_map;
#[path = "std/core/range.rs"]
pub mod std_core_range;
#[path = "std/core/result.rs"]
pub mod std_core_result;
#[path = "std/core/set.rs"]
pub mod std_core_set;
#[path = "std/core/sorted.rs"]
pub mod std_core_sorted;
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
#[path = "platform/std/core/map.rs"]
pub mod platform_std_core_map;
#[path = "platform/std/core/set.rs"]
pub mod platform_std_core_set;
#[path = "platform/std/core/sorted.rs"]
pub mod platform_std_core_sorted;
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
use crate::std_args::Missing;
use crate::std_args::MissingOne;
use crate::std_args::MissingValue;
use crate::std_core_list::NonEmpty__List_qualifies;
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
        return format!("serve {} on {}:{} as {}, {}, {} worker(s)", crate::std_core_string::join_platform(&c_1.files, &String::from(",")), c_1.net.host, c_1.port, c_1.output.clone(), secure, {
            let mut __pick_3 = &c_1.net.workers;
            if __pick_3.is_some() {
                let mut __picked_4 = __pick_3.unwrap();
                __picked_4
            } else {
                1i32
            }
        });
    } else if matches!(c, Some(crate::unions::Union2::U2(_))) {
        let mut c_6 = match &c { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
        let mut s = &c_6.source;
        let mut from: String = if matches!(s, crate::unions::Union3::U1(_)) {
            let mut s_7 = match &s { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
            format!("{} (timeout {}s)", s_7.url, s_7.timeout)
        } else if matches!(s, crate::unions::Union3::U2(_)) {
            let mut s_8 = match &s { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
            format!("file {}", s_8.file)
        } else if matches!(s, crate::unions::Union3::U3(_)) {
            let mut s_9 = match &s { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
            String::from("standard input")
        } else {
            panic!("salvo: unhandled union arm at main:104:24");
        };
        return format!("fetch {} with {} header(s)", from, crate::std_core_list::size_platform::<String>(&c_6.header));
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
    crate::show(&__handle_2, &vec![String::from("serve"), String::from("-p"), String::from("80"), String::from("index.html"), String::from("app.js")]);
    crate::show(&__handle_2, &vec![String::from("-v"), String::from("serve"), String::from("--host=0.0.0.0"), String::from("--workers"), String::from("4"), String::from("--cert"), String::from("c.pem"), String::from("--key"), String::from("k.pem"), String::from("--output"), String::from("json"), String::from("site")]);
    crate::show(&__handle_2, &vec![String::from("fetch"), String::from("--url"), String::from("https://example.com"), String::from("--header"), String::from("Accept: text/html"), String::from("--header"), String::from("X-Id: 7")]);
    crate::show(&__handle_2, &vec![String::from("fetch"), String::from("--from-stdin")]);
    crate::show(&__handle_2, &vec![String::from("serve"), String::from("-p"), String::from("0"), String::from("--output"), String::from("xml"), String::from("--cert"), String::from("c.pem")]);
    crate::show(&__handle_2, &vec![String::from("fetch"), String::from("--url"), String::from("u"), String::from("--file"), String::from("f")]);
    crate::show(&__handle_2, &vec![String::from("deploy")]);
    crate::show(&__handle_2, &vec![String::from("--help")]);
    crate::show(&__handle_2, &vec![String::from("serve"), String::from("--help")]);
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
    let mut required__c1__c2__c3__c4: bool = false;
    let mut many__c1__c2__c3__c4: bool = false;
    let mut switch__c1__c2__c3__c4: bool = false;
    let mut default_text__c1__c2__c3__c4: Option<String> = None;
    let mut choices__c1__c2__c3__c4: Vec<String> = vec![];
    let mut exclusive__c1__c2__c3__c4: bool = false;
    switch__c1__c2__c3__c4 = true;
    if exclusive__c1__c2__c3__c4 {
        crate::std_args::add_one(&mut groups, &String::from("verbose"), required__c1__c2__c3__c4, Some(String::from("Print more.")));
    };
    let mut short__c1__c2__c3__c4: Option<char> = None;
    short__c1__c2__c3__c4 = Some('v');
    let mut short__c1__c2__c3__c4_1 = short__c1__c2__c3__c4.unwrap();
    let mut opt__c1__c2__c3__c4: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("verbose"), short: Some(short__c1__c2__c3__c4_1), takes_value: !(switch__c1__c2__c3__c4), repeats: many__c1__c2__c3__c4, required: (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help: Some(String::from("Print more.")), default_text: default_text__c1__c2__c3__c4, choices: choices__c1__c2__c3__c4, group: None };
    if (!(exclusive__c1__c2__c3__c4) || !(crate::std_core_list::is_empty::<String>(&opt__c1__c2__c3__c4.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c1__c2__c3__c4);
    };
    command_fields = i32::wrapping_add(command_fields, 1i32);
    let mut child__c8__c9__c11__c12__c13: crate::std_args::Spec = crate::arg_spec__SpecServe();
    crate::std_core_list::add_platform::<crate::std_args::CmdSpec>(&mut commands, crate::std_args::CmdSpec { name: (child__c8__c9__c11__c12__c13.name).clone(), about: (child__c8__c9__c11__c12__c13.about).clone() });
    let mut child__c8__c9__c11__c14__c15: crate::std_args::Spec = crate::arg_spec__SpecFetch();
    crate::std_core_list::add_platform::<crate::std_args::CmdSpec>(&mut commands, crate::std_args::CmdSpec { name: (child__c8__c9__c11__c14__c15.name).clone(), about: (child__c8__c9__c11__c14__c15.about).clone() });
    let mut name: String = crate::std_args::command_name(&String::from("Tool"));
    return crate::std_args::Spec { name: name, about: Some(String::from("A toy file server.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkToolOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Tool, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut s__c8__c9__c10__c11__c12: crate::std_args::Spec = crate::arg_spec__SpecServe();
    if crate::std_args::is_command(found, &s__c8__c9__c10__c11__c12.name) {
        let mut r__c8__c9__c10__c11__c12: crate::unions::Union2<crate::Serve, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::std_args::parse_named::<crate::Serve>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecServe(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkServeOrErrsNonEmptyListArgError(__a0));
        crate::std_args::group_errors::<crate::Serve>(&mut errors, r__c8__c9__c10__c11__c12, 0i32);
    };
    let mut s__c8__c9__c10__c13__c14: crate::std_args::Spec = crate::arg_spec__SpecFetch();
    if crate::std_args::is_command(found, &s__c8__c9__c10__c13__c14.name) {
        let mut r__c8__c9__c10__c13__c14: crate::unions::Union2<crate::Fetch, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::std_args::parse_named::<crate::Fetch>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecFetch(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkFetchOrErrsNonEmptyListArgError(__a0));
        crate::std_args::group_errors::<crate::Fetch>(&mut errors, r__c8__c9__c10__c13__c14, 0i32);
    };
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Tool>(crate::Tool { verbose: crate::std_args::eval({ let mut found = found.clone(); move || -> bool {
        return crate::std_args::given(&found, &String::from("verbose"));
    } }), command: crate::std_args::eval({ let mut found = found.clone(); move || -> Option<crate::unions::Union2<crate::Serve, crate::Fetch>> {
        let mut s__c24__c25__c26__c27__c28: crate::std_args::Spec = crate::arg_spec__SpecServe();
        if crate::std_args::is_command(&found, &s__c24__c25__c26__c27__c28.name) {
            return Some(crate::unions::Union2::U1(crate::std_args::built::<crate::Serve>(crate::std_args::parse_named::<crate::Serve>(&found.rest, Some((found.path).clone()), &mut || crate::arg_spec__SpecServe(), &mut |__a0: &crate::std_args::Found| crate::from_found__Found_OkServeOrErrsNonEmptyListArgError(__a0)))));
        };
        let mut s__c24__c25__c26__c29__c30: crate::std_args::Spec = crate::arg_spec__SpecFetch();
        if crate::std_args::is_command(&found, &s__c24__c25__c26__c29__c30.name) {
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
    let mut child__c1__c2__c3__c4: crate::std_args::Spec = crate::arg_spec__SpecNet();
    crate::std_args::merge_group(&mut opts, &mut groups, &child__c1__c2__c3__c4, &String::from("net"), &String::from("flat"), None);
    let mut required__c5__c6__c7__c8: bool = false;
    let mut many__c5__c6__c7__c8: bool = false;
    let mut switch__c5__c6__c7__c8: bool = false;
    let mut default_text__c5__c6__c7__c8: Option<String> = None;
    let mut choices__c5__c6__c7__c8: Vec<String> = vec![];
    let mut exclusive__c5__c6__c7__c8: bool = false;
    default_text__c5__c6__c7__c8 = Some(format!("{}", {
        let mut __default_1: i32 = 8080i32;
        if !(crate::std_core_range::InRange__Int_qualifies(__default_1, 1i32, 65535i32)) { panic!("salvo: {} at main:36:16", String::from("the default for field `port` is not `InRange(1, 65535) Int`")) };
        __default_1
    }));
    if exclusive__c5__c6__c7__c8 {
        crate::std_args::add_one(&mut groups, &String::from("port"), required__c5__c6__c7__c8, Some(String::from("Port to listen on.")));
    };
    let mut short__c5__c6__c7__c8: Option<char> = None;
    short__c5__c6__c7__c8 = Some('p');
    let mut short__c5__c6__c7__c8_2 = short__c5__c6__c7__c8.unwrap();
    let mut default_text__c5__c6__c7__c8_3 = default_text__c5__c6__c7__c8.as_ref().unwrap();
    let mut opt__c5__c6__c7__c8: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("port"), short: Some(short__c5__c6__c7__c8_2), takes_value: !(switch__c5__c6__c7__c8), repeats: many__c5__c6__c7__c8, required: (required__c5__c6__c7__c8 && !(exclusive__c5__c6__c7__c8)), help: Some(String::from("Port to listen on.")), default_text: Some(default_text__c5__c6__c7__c8_3.clone()), choices: choices__c5__c6__c7__c8, group: None };
    if (!(exclusive__c5__c6__c7__c8) || !(crate::std_core_list::is_empty::<String>(&opt__c5__c6__c7__c8.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c5__c6__c7__c8);
    };
    let mut required__c16__c17__c18__c19: bool = false;
    let mut many__c16__c17__c18__c19: bool = false;
    let mut switch__c16__c17__c18__c19: bool = false;
    let mut default_text__c16__c17__c18__c19: Option<String> = None;
    let mut choices__c16__c17__c18__c19: Vec<String> = vec![];
    let mut exclusive__c16__c17__c18__c19: bool = false;
    default_text__c16__c17__c18__c19 = Some(format!("{}", String::from("text")));
    crate::std_core_list::add_platform::<String>(&mut choices__c16__c17__c18__c19, crate::std_args::literal_text(&String::from("\"json\"")));
    crate::std_core_list::add_platform::<String>(&mut choices__c16__c17__c18__c19, crate::std_args::literal_text(&String::from("\"text\"")));
    if exclusive__c16__c17__c18__c19 {
        crate::std_args::add_one(&mut groups, &String::from("output"), required__c16__c17__c18__c19, Some(String::from("Output format.")));
    };
    let mut short__c16__c17__c18__c19: Option<char> = None;
    let mut default_text__c16__c17__c18__c19_4 = default_text__c16__c17__c18__c19.as_ref().unwrap();
    let mut opt__c16__c17__c18__c19: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("output"), short: short__c16__c17__c18__c19, takes_value: !(switch__c16__c17__c18__c19), repeats: many__c16__c17__c18__c19, required: (required__c16__c17__c18__c19 && !(exclusive__c16__c17__c18__c19)), help: Some(String::from("Output format.")), default_text: Some(default_text__c16__c17__c18__c19_4.clone()), choices: choices__c16__c17__c18__c19, group: None };
    if (!(exclusive__c16__c17__c18__c19) || !(crate::std_core_list::is_empty::<String>(&opt__c16__c17__c18__c19.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c16__c17__c18__c19);
    };
    let mut child__c33__c34__c35: crate::std_args::Spec = crate::arg_spec__SpecTls();
    crate::std_args::merge_group(&mut opts, &mut groups, &child__c33__c34__c35, &String::from("tls"), &String::from("together"), Some(String::from("TLS (give both or neither).")));
    let mut required__c36__c37__c38__c39: bool = false;
    let mut many__c36__c37__c38__c39: bool = false;
    let mut switch__c36__c37__c38__c39: bool = false;
    let mut default_text__c36__c37__c38__c39: Option<String> = None;
    let mut choices__c36__c37__c38__c39: Vec<String> = vec![];
    let mut exclusive__c36__c37__c38__c39: bool = false;
    many__c36__c37__c38__c39 = true;
    required__c36__c37__c38__c39 = true;
    if exclusive__c36__c37__c38__c39 {
        crate::std_args::add_one(&mut groups, &String::from("files"), required__c36__c37__c38__c39, Some(String::from("Files to serve.")));
    };
    let mut short__c36__c37__c38__c39: Option<char> = None;
    let mut opt__c36__c37__c38__c39: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("files"), short: short__c36__c37__c38__c39, takes_value: !(switch__c36__c37__c38__c39), repeats: many__c36__c37__c38__c39, required: (required__c36__c37__c38__c39 && !(exclusive__c36__c37__c38__c39)), help: Some(String::from("Files to serve.")), default_text: default_text__c36__c37__c38__c39, choices: choices__c36__c37__c38__c39, group: None };
    if exclusive__c36__c37__c38__c39 {
        panic!("salvo: positional `files` is a choice of option groups at main:49:16");
    };
    crate::std_core_list::add_platform::<crate::std_args::ArgSpec>(&mut args, crate::std_args::ArgSpec { field: (opt__c36__c37__c38__c39.field).clone(), help: (opt__c36__c37__c38__c39.help).clone(), required: opt__c36__c37__c38__c39.required, many: opt__c36__c37__c38__c39.repeats });
    let mut name: String = crate::std_args::command_name(&String::from("Serve"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Serve files over HTTP.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkServeOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Serve, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut r__c1__c2__c3__c4: crate::unions::Union2<crate::Net, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkNetOrErrsNonEmptyListArgError(found);
    crate::std_args::group_errors::<crate::Net>(&mut errors, r__c1__c2__c3__c4, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    let mut positional__c5__c6__c7__c8__c9__c10__c11: bool = false;
    let mut shown__c5__c6__c7__c8__c9__c10__c11: String = crate::std_args::shown_name(&String::from("port"), positional__c5__c6__c7__c8__c9__c10__c11);
    let mut what__c5__c6__c7__c8__c9__c10__c11: String = String::from("a valid value");
    what__c5__c6__c7__c8__c9__c10__c11 = String::from("an integer");
    crate::std_args::check_value::<i32>(&mut errors, found, &String::from("port"), &shown__c5__c6__c7__c8__c9__c10__c11, &what__c5__c6__c7__c8__c9__c10__c11, &mut |mut v| -> Option<String> {
        if !(crate::std_core_range::InRange__Int_qualifies(*v, 1i32, 65535i32)) {
            Some(format!("{} is not in 1..65535", *v))
        } else {
            None
        }
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    let mut s__c25__c26__c27: crate::std_args::Spec = crate::arg_spec__SpecTls();
    if crate::std_args::any_given(found, &s__c25__c26__c27) {
        let mut r__c25__c26__c27: crate::unions::Union2<crate::Tls, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkTlsOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::Tls>(&mut errors, r__c25__c26__c27, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut positional__c28__c29__c30__c31__c32__c33__c34: bool = false;
    positional__c28__c29__c30__c31__c32__c33__c34 = true;
    let mut shown__c28__c29__c30__c31__c32__c33__c34: String = crate::std_args::shown_name(&String::from("files"), positional__c28__c29__c30__c31__c32__c33__c34);
    let mut what__c28__c29__c30__c31__c32__c33__c34: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("files"), &shown__c28__c29__c30__c31__c32__c33__c34, &what__c28__c29__c30__c31__c32__c33__c34, &mut |mut _v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Serve>(crate::Serve { net: crate::std_args::eval({ let mut found = found.clone(); move || -> crate::Net {
        return crate::std_args::built::<crate::Net>(crate::from_found__Found_OkNetOrErrsNonEmptyListArgError(&found));
    } }), port: crate::std_args::eval({ let mut found = found.clone(); move || -> i32 {
        return crate::std_args::read_value::<i32, i32>(&found, &String::from("port"), Some({
            let mut __default_1: i32 = 8080i32;
            if !(crate::std_core_range::InRange__Int_qualifies(__default_1, 1i32, 65535i32)) { panic!("salvo: {} at main:36:16", String::from("the default for field `port` is not `InRange(1, 65535) Int`")) };
            __default_1
        }), &mut |mut v__c41__c42__c43__c44__c45__c46__c47__c48| -> Option<i32> {
            if crate::std_core_range::InRange__Int_qualifies(v__c41__c42__c43__c44__c45__c46__c47__c48, 1i32, 65535i32) {
                Some(v__c41__c42__c43__c44__c45__c46__c47__c48)
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    } }), output: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        let mut v__c52__c53__c54__c55__c56__c57: Option<String> = crate::std_args::value_of(&found, &String::from("output"));
        if !(v__c52__c53__c54__c55__c56__c57.is_none()) {
            if (match &v__c52__c53__c54__c55__c56__c57 { Some(__v) => __v == &"json", #[allow(unreachable_patterns)] _ => false }) {
                let mut v__c52__c53__c54__c55__c56__c57_2 = v__c52__c53__c54__c55__c56__c57.unwrap();
                return v__c52__c53__c54__c55__c56__c57_2;
            };
            if (match &v__c52__c53__c54__c55__c56__c57 { Some(__v) => __v == &"text", #[allow(unreachable_patterns)] _ => false }) {
                let mut v__c52__c53__c54__c55__c56__c57_3 = v__c52__c53__c54__c55__c56__c57.unwrap();
                return v__c52__c53__c54__c55__c56__c57_3;
            };
        };
        return String::from("text");
    } }), tls: crate::std_args::eval({ let mut found = found.clone(); move || -> Option<crate::Tls> {
        let mut s__c65__c66__c67: crate::std_args::Spec = crate::arg_spec__SpecTls();
        if crate::std_args::any_given(&found, &s__c65__c66__c67) {
            return Some(crate::std_args::built::<crate::Tls>(crate::from_found__Found_OkTlsOrErrsNonEmptyListArgError(&found)));
        };
        return None;
    } }), files: crate::std_args::eval({ let mut found = found.clone(); move || -> Vec<String> {
        let mut all__c68__c69__c70__c71__c72__c73__c74__c75: Vec<String> = crate::std_args::read_all::<String>(&found, &String::from("files"), &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
        if crate::std_core_list::NonEmpty__List_qualifies(&all__c68__c69__c70__c71__c72__c73__c74__c75) {
            return all__c68__c69__c70__c71__c72__c73__c74__c75;
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
    let mut required__c1__c2__c3__c4: bool = false;
    let mut many__c1__c2__c3__c4: bool = false;
    let mut switch__c1__c2__c3__c4: bool = false;
    let mut default_text__c1__c2__c3__c4: Option<String> = None;
    let mut choices__c1__c2__c3__c4: Vec<String> = vec![];
    let mut exclusive__c1__c2__c3__c4: bool = false;
    default_text__c1__c2__c3__c4 = Some(String::from("127.0.0.1"));
    if exclusive__c1__c2__c3__c4 {
        crate::std_args::add_one(&mut groups, &String::from("host"), required__c1__c2__c3__c4, Some(String::from("Address to bind.")));
    };
    let mut short__c1__c2__c3__c4: Option<char> = None;
    let mut default_text__c1__c2__c3__c4_1 = default_text__c1__c2__c3__c4.as_ref().unwrap();
    let mut opt__c1__c2__c3__c4: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("host"), short: short__c1__c2__c3__c4, takes_value: !(switch__c1__c2__c3__c4), repeats: many__c1__c2__c3__c4, required: (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help: Some(String::from("Address to bind.")), default_text: Some(default_text__c1__c2__c3__c4_1.clone()), choices: choices__c1__c2__c3__c4, group: None };
    if (!(exclusive__c1__c2__c3__c4) || !(crate::std_core_list::is_empty::<String>(&opt__c1__c2__c3__c4.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c1__c2__c3__c4);
    };
    let mut required__c11__c12__c13__c14: bool = false;
    let mut many__c11__c12__c13__c14: bool = false;
    let mut switch__c11__c12__c13__c14: bool = false;
    let mut default_text__c11__c12__c13__c14: Option<String> = None;
    let mut choices__c11__c12__c13__c14: Vec<String> = vec![];
    let mut exclusive__c11__c12__c13__c14: bool = false;
    if exclusive__c11__c12__c13__c14 {
        crate::std_args::add_one(&mut groups, &String::from("workers"), required__c11__c12__c13__c14, Some(String::from("Connections served at once.")));
    };
    let mut short__c11__c12__c13__c14: Option<char> = None;
    let mut opt__c11__c12__c13__c14: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("workers"), short: short__c11__c12__c13__c14, takes_value: !(switch__c11__c12__c13__c14), repeats: many__c11__c12__c13__c14, required: (required__c11__c12__c13__c14 && !(exclusive__c11__c12__c13__c14)), help: Some(String::from("Connections served at once.")), default_text: default_text__c11__c12__c13__c14, choices: choices__c11__c12__c13__c14, group: None };
    if (!(exclusive__c11__c12__c13__c14) || !(crate::std_core_list::is_empty::<String>(&opt__c11__c12__c13__c14.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c11__c12__c13__c14);
    };
    let mut name: String = crate::std_args::command_name(&String::from("Net"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Network options.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkNetOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Net, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional__c1__c2__c3__c4__c5__c6__c7: bool = false;
    let mut shown__c1__c2__c3__c4__c5__c6__c7: String = crate::std_args::shown_name(&String::from("host"), positional__c1__c2__c3__c4__c5__c6__c7);
    let mut what__c1__c2__c3__c4__c5__c6__c7: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("host"), &shown__c1__c2__c3__c4__c5__c6__c7, &what__c1__c2__c3__c4__c5__c6__c7, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    let mut positional__c11__c12__c13__c14__c15__c16__c17: bool = false;
    let mut shown__c11__c12__c13__c14__c15__c16__c17: String = crate::std_args::shown_name(&String::from("workers"), positional__c11__c12__c13__c14__c15__c16__c17);
    let mut what__c11__c12__c13__c14__c15__c16__c17: String = String::from("a valid value");
    what__c11__c12__c13__c14__c15__c16__c17 = String::from("an integer");
    crate::std_args::check_value::<i32>(&mut errors, found, &String::from("workers"), &shown__c11__c12__c13__c14__c15__c16__c17, &what__c11__c12__c13__c14__c15__c16__c17, &mut |mut _v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Net>(crate::Net { host: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("host"), Some(String::from("127.0.0.1")), &mut |mut v__c21__c22__c23__c24__c25__c26__c27__c28| -> Option<String> {
            if true {
                Some(v__c21__c22__c23__c24__c25__c26__c27__c28.clone())
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
    let mut required__c1__c2__c3__c4: bool = false;
    let mut many__c1__c2__c3__c4: bool = false;
    let mut switch__c1__c2__c3__c4: bool = false;
    let mut default_text__c1__c2__c3__c4: Option<String> = None;
    let mut choices__c1__c2__c3__c4: Vec<String> = vec![];
    let mut exclusive__c1__c2__c3__c4: bool = false;
    required__c1__c2__c3__c4 = true;
    if exclusive__c1__c2__c3__c4 {
        crate::std_args::add_one(&mut groups, &String::from("cert"), required__c1__c2__c3__c4, Some(String::from("Certificate file.")));
    };
    let mut short__c1__c2__c3__c4: Option<char> = None;
    let mut opt__c1__c2__c3__c4: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("cert"), short: short__c1__c2__c3__c4, takes_value: !(switch__c1__c2__c3__c4), repeats: many__c1__c2__c3__c4, required: (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help: Some(String::from("Certificate file.")), default_text: default_text__c1__c2__c3__c4, choices: choices__c1__c2__c3__c4, group: None };
    if (!(exclusive__c1__c2__c3__c4) || !(crate::std_core_list::is_empty::<String>(&opt__c1__c2__c3__c4.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c1__c2__c3__c4);
    };
    let mut required__c11__c12__c13__c14: bool = false;
    let mut many__c11__c12__c13__c14: bool = false;
    let mut switch__c11__c12__c13__c14: bool = false;
    let mut default_text__c11__c12__c13__c14: Option<String> = None;
    let mut choices__c11__c12__c13__c14: Vec<String> = vec![];
    let mut exclusive__c11__c12__c13__c14: bool = false;
    required__c11__c12__c13__c14 = true;
    if exclusive__c11__c12__c13__c14 {
        crate::std_args::add_one(&mut groups, &String::from("key"), required__c11__c12__c13__c14, Some(String::from("Private key file.")));
    };
    let mut short__c11__c12__c13__c14: Option<char> = None;
    let mut opt__c11__c12__c13__c14: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("key"), short: short__c11__c12__c13__c14, takes_value: !(switch__c11__c12__c13__c14), repeats: many__c11__c12__c13__c14, required: (required__c11__c12__c13__c14 && !(exclusive__c11__c12__c13__c14)), help: Some(String::from("Private key file.")), default_text: default_text__c11__c12__c13__c14, choices: choices__c11__c12__c13__c14, group: None };
    if (!(exclusive__c11__c12__c13__c14) || !(crate::std_core_list::is_empty::<String>(&opt__c11__c12__c13__c14.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c11__c12__c13__c14);
    };
    let mut name: String = crate::std_args::command_name(&String::from("Tls"));
    return crate::std_args::Spec { name: name, about: None, opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkTlsOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Tls, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional__c1__c2__c3__c4__c5__c6__c7: bool = false;
    let mut shown__c1__c2__c3__c4__c5__c6__c7: String = crate::std_args::shown_name(&String::from("cert"), positional__c1__c2__c3__c4__c5__c6__c7);
    let mut what__c1__c2__c3__c4__c5__c6__c7: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("cert"), &shown__c1__c2__c3__c4__c5__c6__c7, &what__c1__c2__c3__c4__c5__c6__c7, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    let mut positional__c11__c12__c13__c14__c15__c16__c17: bool = false;
    let mut shown__c11__c12__c13__c14__c15__c16__c17: String = crate::std_args::shown_name(&String::from("key"), positional__c11__c12__c13__c14__c15__c16__c17);
    let mut what__c11__c12__c13__c14__c15__c16__c17: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("key"), &shown__c11__c12__c13__c14__c15__c16__c17, &what__c11__c12__c13__c14__c15__c16__c17, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Tls>(crate::Tls { cert: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("cert"), None, &mut |mut v__c21__c22__c23__c24__c25__c26__c27__c28| -> Option<String> {
            if true {
                Some(v__c21__c22__c23__c24__c25__c26__c27__c28.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }), key: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("key"), None, &mut |mut v__c31__c32__c33__c34__c35__c36__c37__c38| -> Option<String> {
            if true {
                Some(v__c31__c32__c33__c34__c35__c36__c37__c38.clone())
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
    let mut required__c1__c2__c3__c4: bool = false;
    let mut many__c1__c2__c3__c4: bool = false;
    let mut switch__c1__c2__c3__c4: bool = false;
    let mut default_text__c1__c2__c3__c4: Option<String> = None;
    let mut choices__c1__c2__c3__c4: Vec<String> = vec![];
    let mut exclusive__c1__c2__c3__c4: bool = false;
    required__c1__c2__c3__c4 = true;
    let mut child__c1__c2__c3__c4__c5__c6__c9__c10__c11__c12: crate::std_args::Spec = crate::arg_spec__SpecFromUrl();
    crate::std_args::merge_group(&mut opts, &mut groups, &child__c1__c2__c3__c4__c5__c6__c9__c10__c11__c12, &String::from("source/FromUrl"), &String::from("arm"), None);
    exclusive__c1__c2__c3__c4 = true;
    let mut child__c1__c2__c3__c4__c5__c6__c9__c10__c13__c14: crate::std_args::Spec = crate::arg_spec__SpecFromFile();
    crate::std_args::merge_group(&mut opts, &mut groups, &child__c1__c2__c3__c4__c5__c6__c9__c10__c13__c14, &String::from("source/FromFile"), &String::from("arm"), None);
    exclusive__c1__c2__c3__c4 = true;
    let mut child__c1__c2__c3__c4__c5__c6__c9__c10__c15__c16: crate::std_args::Spec = crate::arg_spec__SpecFromStdin();
    crate::std_args::merge_group(&mut opts, &mut groups, &child__c1__c2__c3__c4__c5__c6__c9__c10__c15__c16, &String::from("source/FromStdin"), &String::from("arm"), None);
    exclusive__c1__c2__c3__c4 = true;
    if exclusive__c1__c2__c3__c4 {
        crate::std_args::add_one(&mut groups, &String::from("source"), required__c1__c2__c3__c4, Some(String::from("Where to read.")));
    };
    let mut short__c1__c2__c3__c4: Option<char> = None;
    let mut opt__c1__c2__c3__c4: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("source"), short: short__c1__c2__c3__c4, takes_value: !(switch__c1__c2__c3__c4), repeats: many__c1__c2__c3__c4, required: (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help: Some(String::from("Where to read.")), default_text: default_text__c1__c2__c3__c4, choices: choices__c1__c2__c3__c4, group: None };
    if (!(exclusive__c1__c2__c3__c4) || !(crate::std_core_list::is_empty::<String>(&opt__c1__c2__c3__c4.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c1__c2__c3__c4);
    };
    let mut required__c18__c19__c20__c21: bool = false;
    let mut many__c18__c19__c20__c21: bool = false;
    let mut switch__c18__c19__c20__c21: bool = false;
    let mut default_text__c18__c19__c20__c21: Option<String> = None;
    let mut choices__c18__c19__c20__c21: Vec<String> = vec![];
    let mut exclusive__c18__c19__c20__c21: bool = false;
    many__c18__c19__c20__c21 = true;
    if exclusive__c18__c19__c20__c21 {
        crate::std_args::add_one(&mut groups, &String::from("header"), required__c18__c19__c20__c21, Some(String::from("A header line to send; repeatable.")));
    };
    let mut short__c18__c19__c20__c21: Option<char> = None;
    let mut opt__c18__c19__c20__c21: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("header"), short: short__c18__c19__c20__c21, takes_value: !(switch__c18__c19__c20__c21), repeats: many__c18__c19__c20__c21, required: (required__c18__c19__c20__c21 && !(exclusive__c18__c19__c20__c21)), help: Some(String::from("A header line to send; repeatable.")), default_text: default_text__c18__c19__c20__c21, choices: choices__c18__c19__c20__c21, group: None };
    if (!(exclusive__c18__c19__c20__c21) || !(crate::std_core_list::is_empty::<String>(&opt__c18__c19__c20__c21.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c18__c19__c20__c21);
    };
    let mut name: String = crate::std_args::command_name(&String::from("Fetch"));
    return crate::std_args::Spec { name: name, about: Some(String::from("Fetch a document.")), opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFetchOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::Fetch, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut s__c1__c2__c3__c4__c5__c6__c7__c8: crate::std_args::Spec = crate::arg_spec__SpecFromUrl();
    if crate::std_args::any_given(found, &s__c1__c2__c3__c4__c5__c6__c7__c8) {
        let mut r__c1__c2__c3__c4__c5__c6__c7__c8: crate::unions::Union2<crate::FromUrl, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkFromUrlOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::FromUrl>(&mut errors, r__c1__c2__c3__c4__c5__c6__c7__c8, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut s__c1__c2__c3__c4__c5__c6__c9__c10: crate::std_args::Spec = crate::arg_spec__SpecFromFile();
    if crate::std_args::any_given(found, &s__c1__c2__c3__c4__c5__c6__c9__c10) {
        let mut r__c1__c2__c3__c4__c5__c6__c9__c10: crate::unions::Union2<crate::FromFile, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkFromFileOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::FromFile>(&mut errors, r__c1__c2__c3__c4__c5__c6__c9__c10, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut s__c1__c2__c3__c4__c5__c6__c11__c12: crate::std_args::Spec = crate::arg_spec__SpecFromStdin();
    if crate::std_args::any_given(found, &s__c1__c2__c3__c4__c5__c6__c11__c12) {
        let mut r__c1__c2__c3__c4__c5__c6__c11__c12: crate::unions::Union2<crate::FromStdin, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> = crate::from_found__Found_OkFromStdinOrErrsNonEmptyListArgError(found);
        crate::std_args::group_errors::<crate::FromStdin>(&mut errors, r__c1__c2__c3__c4__c5__c6__c11__c12, crate::std_core_list::size_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&found.errors));
    };
    let mut positional__c13__c14__c15__c16__c17__c18__c19: bool = false;
    let mut shown__c13__c14__c15__c16__c17__c18__c19: String = crate::std_args::shown_name(&String::from("header"), positional__c13__c14__c15__c16__c17__c18__c19);
    let mut what__c13__c14__c15__c16__c17__c18__c19: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("header"), &shown__c13__c14__c15__c16__c17__c18__c19, &what__c13__c14__c15__c16__c17__c18__c19, &mut |mut _v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::Fetch>(crate::Fetch { source: crate::std_args::eval({ let mut found = found.clone(); move || -> crate::unions::Union3<crate::FromUrl, crate::FromFile, crate::FromStdin> {
        let mut s__c21__c22__c23__c24__c25__c26__c27__c28: crate::std_args::Spec = crate::arg_spec__SpecFromUrl();
        if crate::std_args::any_given(&found, &s__c21__c22__c23__c24__c25__c26__c27__c28) {
            return crate::unions::Union3::U1(crate::std_args::built::<crate::FromUrl>(crate::from_found__Found_OkFromUrlOrErrsNonEmptyListArgError(&found)));
        };
        let mut s__c21__c22__c23__c24__c25__c26__c29__c30: crate::std_args::Spec = crate::arg_spec__SpecFromFile();
        if crate::std_args::any_given(&found, &s__c21__c22__c23__c24__c25__c26__c29__c30) {
            return crate::unions::Union3::U2(crate::std_args::built::<crate::FromFile>(crate::from_found__Found_OkFromFileOrErrsNonEmptyListArgError(&found)));
        };
        let mut s__c21__c22__c23__c24__c25__c26__c31__c32: crate::std_args::Spec = crate::arg_spec__SpecFromStdin();
        if crate::std_args::any_given(&found, &s__c21__c22__c23__c24__c25__c26__c31__c32) {
            return crate::unions::Union3::U3(crate::std_args::built::<crate::FromStdin>(crate::from_found__Found_OkFromStdinOrErrsNonEmptyListArgError(&found)));
        };
        let mut v__c21__c22__c23__c24__c25__c26: Option<String> = crate::std_args::value_of(&found, &String::from("source"));
        if !(v__c21__c22__c23__c24__c25__c26.is_none()) {
        };
        panic!("salvo: `source` was checked as required at main:73:5")
    } }), header: crate::std_args::eval({ let mut found = found.clone(); move || -> Vec<String> {
        let mut all__c41__c42__c43__c44__c45__c46__c47__c48: Vec<String> = crate::std_args::read_all::<String>(&found, &String::from("header"), &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
        {
            return all__c41__c42__c43__c44__c45__c46__c47__c48;
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
    let mut required__c1__c2__c3__c4: bool = false;
    let mut many__c1__c2__c3__c4: bool = false;
    let mut switch__c1__c2__c3__c4: bool = false;
    let mut default_text__c1__c2__c3__c4: Option<String> = None;
    let mut choices__c1__c2__c3__c4: Vec<String> = vec![];
    let mut exclusive__c1__c2__c3__c4: bool = false;
    required__c1__c2__c3__c4 = true;
    if exclusive__c1__c2__c3__c4 {
        crate::std_args::add_one(&mut groups, &String::from("url"), required__c1__c2__c3__c4, Some(String::from("Where to fetch from.")));
    };
    let mut short__c1__c2__c3__c4: Option<char> = None;
    let mut opt__c1__c2__c3__c4: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("url"), short: short__c1__c2__c3__c4, takes_value: !(switch__c1__c2__c3__c4), repeats: many__c1__c2__c3__c4, required: (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help: Some(String::from("Where to fetch from.")), default_text: default_text__c1__c2__c3__c4, choices: choices__c1__c2__c3__c4, group: None };
    if (!(exclusive__c1__c2__c3__c4) || !(crate::std_core_list::is_empty::<String>(&opt__c1__c2__c3__c4.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c1__c2__c3__c4);
    };
    let mut required__c11__c12__c13__c14: bool = false;
    let mut many__c11__c12__c13__c14: bool = false;
    let mut switch__c11__c12__c13__c14: bool = false;
    let mut default_text__c11__c12__c13__c14: Option<String> = None;
    let mut choices__c11__c12__c13__c14: Vec<String> = vec![];
    let mut exclusive__c11__c12__c13__c14: bool = false;
    default_text__c11__c12__c13__c14 = Some(format!("{}", 30i32));
    if exclusive__c11__c12__c13__c14 {
        crate::std_args::add_one(&mut groups, &String::from("timeout"), required__c11__c12__c13__c14, Some(String::from("Seconds to wait.")));
    };
    let mut short__c11__c12__c13__c14: Option<char> = None;
    let mut default_text__c11__c12__c13__c14_1 = default_text__c11__c12__c13__c14.as_ref().unwrap();
    let mut opt__c11__c12__c13__c14: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("timeout"), short: short__c11__c12__c13__c14, takes_value: !(switch__c11__c12__c13__c14), repeats: many__c11__c12__c13__c14, required: (required__c11__c12__c13__c14 && !(exclusive__c11__c12__c13__c14)), help: Some(String::from("Seconds to wait.")), default_text: Some(default_text__c11__c12__c13__c14_1.clone()), choices: choices__c11__c12__c13__c14, group: None };
    if (!(exclusive__c11__c12__c13__c14) || !(crate::std_core_list::is_empty::<String>(&opt__c11__c12__c13__c14.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c11__c12__c13__c14);
    };
    let mut name: String = crate::std_args::command_name(&String::from("FromUrl"));
    return crate::std_args::Spec { name: name, about: None, opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFromUrlOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::FromUrl, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional__c1__c2__c3__c4__c5__c6__c7: bool = false;
    let mut shown__c1__c2__c3__c4__c5__c6__c7: String = crate::std_args::shown_name(&String::from("url"), positional__c1__c2__c3__c4__c5__c6__c7);
    let mut what__c1__c2__c3__c4__c5__c6__c7: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("url"), &shown__c1__c2__c3__c4__c5__c6__c7, &what__c1__c2__c3__c4__c5__c6__c7, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    let mut positional__c11__c12__c13__c14__c15__c16__c17: bool = false;
    let mut shown__c11__c12__c13__c14__c15__c16__c17: String = crate::std_args::shown_name(&String::from("timeout"), positional__c11__c12__c13__c14__c15__c16__c17);
    let mut what__c11__c12__c13__c14__c15__c16__c17: String = String::from("a valid value");
    what__c11__c12__c13__c14__c15__c16__c17 = String::from("an integer");
    crate::std_args::check_value::<i32>(&mut errors, found, &String::from("timeout"), &shown__c11__c12__c13__c14__c15__c16__c17, &what__c11__c12__c13__c14__c15__c16__c17, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_IntOpt(__a0));
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::FromUrl>(crate::FromUrl { url: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("url"), None, &mut |mut v__c23__c24__c25__c26__c27__c28__c29__c30| -> Option<String> {
            if true {
                Some(v__c23__c24__c25__c26__c27__c28__c29__c30.clone())
            } else {
                None
            }
        }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    } }), timeout: crate::std_args::eval({ let mut found = found.clone(); move || -> i32 {
        return crate::std_args::read_value::<i32, i32>(&found, &String::from("timeout"), Some(30i32), &mut |mut v__c33__c34__c35__c36__c37__c38__c39__c40| -> Option<i32> {
            if true {
                Some(v__c33__c34__c35__c36__c37__c38__c39__c40)
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
    let mut required__c1__c2__c3__c4: bool = false;
    let mut many__c1__c2__c3__c4: bool = false;
    let mut switch__c1__c2__c3__c4: bool = false;
    let mut default_text__c1__c2__c3__c4: Option<String> = None;
    let mut choices__c1__c2__c3__c4: Vec<String> = vec![];
    let mut exclusive__c1__c2__c3__c4: bool = false;
    required__c1__c2__c3__c4 = true;
    if exclusive__c1__c2__c3__c4 {
        crate::std_args::add_one(&mut groups, &String::from("file"), required__c1__c2__c3__c4, Some(String::from("A file to read instead.")));
    };
    let mut short__c1__c2__c3__c4: Option<char> = None;
    let mut opt__c1__c2__c3__c4: crate::std_args::OptSpec = crate::std_args::OptSpec { field: String::from("file"), short: short__c1__c2__c3__c4, takes_value: !(switch__c1__c2__c3__c4), repeats: many__c1__c2__c3__c4, required: (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help: Some(String::from("A file to read instead.")), default_text: default_text__c1__c2__c3__c4, choices: choices__c1__c2__c3__c4, group: None };
    if (!(exclusive__c1__c2__c3__c4) || !(crate::std_core_list::is_empty::<String>(&opt__c1__c2__c3__c4.choices))) {
        crate::std_core_list::add_platform::<crate::std_args::OptSpec>(&mut opts, opt__c1__c2__c3__c4);
    };
    let mut name: String = crate::std_args::command_name(&String::from("FromFile"));
    return crate::std_args::Spec { name: name, about: None, opts: opts, args: args, groups: groups, commands: commands, command_required: command_required, command_fields: command_fields };
}

pub fn from_found__Found_OkFromFileOrErrsNonEmptyListArgError(found: &crate::std_args::Found) -> crate::unions::Union2<crate::FromFile, Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>> {
    let mut errors: Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>> = vec![];
    for mut e in found.errors.iter() {
        crate::std_core_list::add_platform::<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>(&mut errors, (e).clone());
    }
    let mut positional__c1__c2__c3__c4__c5__c6__c7: bool = false;
    let mut shown__c1__c2__c3__c4__c5__c6__c7: String = crate::std_args::shown_name(&String::from("file"), positional__c1__c2__c3__c4__c5__c6__c7);
    let mut what__c1__c2__c3__c4__c5__c6__c7: String = String::from("a valid value");
    crate::std_args::check_value::<String>(&mut errors, found, &String::from("file"), &shown__c1__c2__c3__c4__c5__c6__c7, &what__c1__c2__c3__c4__c5__c6__c7, &mut |mut v| -> Option<String> {
        None
    }, &mut |__a0: &String| crate::std_core_string::from_str__Str_StrOpt(__a0));
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::FromFile>(crate::FromFile { file: crate::std_args::eval({ let mut found = found.clone(); move || -> String {
        return crate::std_args::read_value::<String, String>(&found, &String::from("file"), None, &mut |mut v__c11__c12__c13__c14__c15__c16__c17__c18| -> Option<String> {
            if true {
                Some(v__c11__c12__c13__c14__c15__c16__c17__c18.clone())
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
    if crate::std_core_list::NonEmpty__List_qualifies(&errors) {
        return crate::unions::Union2::U2(crate::std_core_result::Errs::<Vec<crate::unions::Union11<crate::std_args::UnknownOption, crate::std_args::MissingValue, crate::std_args::Missing, crate::std_args::Invalid, crate::std_args::Unexpected, crate::std_args::Repeated, crate::std_args::Requires, crate::std_args::Conflict, crate::std_args::MissingOne, crate::std_args::UnknownCommand, crate::std_args::HelpRequested>>>(errors));
    };
    return crate::unions::Union2::U1(crate::std_core_result::Ok::<crate::FromStdin>(crate::FromStdin {}));
}
