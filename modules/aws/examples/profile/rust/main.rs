#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "aws.rs"]
pub mod aws;
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
#[path = "std/core/set.rs"]
pub mod std_core_set;
#[path = "std/core/sorted.rs"]
pub mod std_core_sorted;
#[path = "std/core/string.rs"]
pub mod std_core_string;
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

use crate::aws::ProfileCredentials;
use crate::std_core_console::println;
use crate::aws::to_str;


pub fn main() {
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut creds: crate::aws::ProfileCredentials = crate::aws::ProfileCredentials { profile: String::from("default"), path: String::from("~/.aws/credentials") };
    crate::std_core_console::println(&__handle_2, &format!("profile: {}", creds.profile));
    crate::std_core_console::println(&__handle_2, &format!("path:    {}", creds.path));
    let mut staging: crate::aws::ProfileCredentials = crate::aws::ProfileCredentials { profile: String::from("staging"), path: String::from("/etc/aws/credentials") };
    crate::std_core_console::println(&__handle_2, &format!("{}", crate::aws::to_str(&staging)));
}
