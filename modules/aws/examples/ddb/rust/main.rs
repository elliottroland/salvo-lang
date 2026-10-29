#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "aws.rs"]
pub mod aws;
#[path = "aws/ddb.rs"]
pub mod aws_ddb;
#[path = "std/core/actor.rs"]
pub mod std_core_actor;
#[path = "std/core/basic.rs"]
pub mod std_core_basic;
#[path = "std/core/buffer.rs"]
pub mod std_core_buffer;
#[path = "std/core/bytes.rs"]
pub mod std_core_bytes;
#[path = "std/core/checked.rs"]
pub mod std_core_checked;
#[path = "std/core/compare.rs"]
pub mod std_core_compare;
#[path = "std/core/console.rs"]
pub mod std_core_console;
#[path = "std/core/deque.rs"]
pub mod std_core_deque;
#[path = "std/core/index.rs"]
pub mod std_core_index;
#[path = "std/core/iterator.rs"]
pub mod std_core_iterator;
#[path = "std/core/list.rs"]
pub mod std_core_list;
#[path = "std/core/map.rs"]
pub mod std_core_map;
#[path = "std/core/other.rs"]
pub mod std_core_other;
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
#[path = "std/runtime.rs"]
pub mod std_runtime;
#[path = "std/runtime/routing.rs"]
pub mod std_runtime_routing;
#[path = "std/runtime/timers.rs"]
pub mod std_runtime_timers;
#[path = "std/time.rs"]
pub mod std_time;
#[path = "platform/std/core/buffer.rs"]
pub mod platform_std_core_buffer;
#[path = "platform/std/core/bytes.rs"]
pub mod platform_std_core_bytes;
#[path = "platform/std/core/console.rs"]
pub mod platform_std_core_console;
#[path = "platform/std/core/deque.rs"]
pub mod platform_std_core_deque;
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
#[path = "platform/std/runtime/routing.rs"]
pub mod platform_std_runtime_routing;
#[path = "platform/std/runtime.rs"]
pub mod platform_std_runtime;
#[path = "platform/std/time.rs"]
pub mod platform_std_time;

use crate::aws_ddb::AttributeDefinition;
use crate::aws_ddb::AttributeValueB;
use crate::aws_ddb::AttributeValueBool;
use crate::aws_ddb::AttributeValueBs;
use crate::aws_ddb::AttributeValueL;
use crate::aws_ddb::AttributeValueM;
use crate::aws_ddb::AttributeValueN;
use crate::aws_ddb::AttributeValueNs;
use crate::aws_ddb::AttributeValueNull;
use crate::aws_ddb::AttributeValueS;
use crate::aws_ddb::AttributeValueSs;
use crate::aws::AwsError;
use crate::std_core_bytes::Bytes;
use crate::std_core_checked::Checked;
use crate::std_core_console::Console;
use crate::aws_ddb::CreateTableInput;
use crate::aws_ddb::CreateTableOutput;
use crate::aws_ddb::Ddb;
use crate::aws_ddb::DdbError;
use crate::aws_ddb::DeleteItemInput;
use crate::aws_ddb::DeleteItemOutput;
use crate::aws_ddb::DeleteTableInput;
use crate::aws_ddb::DeleteTableOutput;
use crate::aws_ddb::GetItemInput;
use crate::aws_ddb::GetItemOutput;
use crate::aws_ddb::KeySchemaElement;
use crate::std_core_map::Map;
use crate::std_core_map::Map_platform;
use crate::std_core_map::Mut_Map_platform;
use crate::std_core_set::Mut_Set_platform;
use crate::aws_ddb::PutItemInput;
use crate::aws_ddb::PutItemOutput;
use crate::aws_ddb::QueryInput;
use crate::aws_ddb::QueryOutput;
use crate::std_core_set::Set;
use crate::std_core_checked::checked;
use crate::std_core_map::contains_key_platform;
use crate::std_core_set::contains_platform;
use crate::std_core_checked::detach;
use crate::std_core_result::err;
use crate::std_core_map::get_platform;
use crate::std_core_string::join_platform;
use crate::std_core_map::keys_platform;
use crate::std_core_result::ok;
use crate::std_core_console::println;
use crate::std_core_map::put_platform;
use crate::std_core_list::sort;
use crate::std_core_list::to_str;


pub fn show(mut v: crate::unions::Union10<&crate::aws_ddb::AttributeValueS, &crate::aws_ddb::AttributeValueN, &crate::aws_ddb::AttributeValueB, &crate::aws_ddb::AttributeValueSs, &crate::aws_ddb::AttributeValueNs, &crate::aws_ddb::AttributeValueBs, &crate::aws_ddb::AttributeValueM, &crate::aws_ddb::AttributeValueL, &crate::aws_ddb::AttributeValueBool, &crate::aws_ddb::AttributeValueNull>) -> String {
    return if matches!(v, crate::unions::Union10::U1(_)) {
        let mut v_1 = match v { crate::unions::Union10::U1(__v) => __v, _ => unreachable!() };
        return format!("\"{}\"", v_1.value);
    } else if matches!(v, crate::unions::Union10::U2(_)) {
        let mut v_2 = match v { crate::unions::Union10::U2(__v) => __v, _ => unreachable!() };
        return v_2.value.clone();
    } else if matches!(v, crate::unions::Union10::U9(_)) {
        let mut v_3 = match v { crate::unions::Union10::U9(__v) => __v, _ => unreachable!() };
        return format!("{}", v_3.value);
    } else if matches!(v, crate::unions::Union10::U10(_)) {
        let mut v_4 = match v { crate::unions::Union10::U10(__v) => __v, _ => unreachable!() };
        return String::from("null");
    } else if matches!(v, crate::unions::Union10::U4(_)) {
        let mut v_5 = match v { crate::unions::Union10::U4(__v) => __v, _ => unreachable!() };
        return format!("{{{}}}", crate::std_core_string::join_platform(&v_5.value, &String::from(", ")));
    } else if matches!(v, crate::unions::Union10::U5(_)) {
        let mut v_6 = match v { crate::unions::Union10::U5(__v) => __v, _ => unreachable!() };
        return format!("{{{}}}", crate::std_core_string::join_platform(&v_6.value, &String::from(", ")));
    } else if matches!(v, crate::unions::Union10::U3(_)) {
        let mut v_7 = match v { crate::unions::Union10::U3(__v) => __v, _ => unreachable!() };
        return format!("<{} bytes>", crate::std_core_bytes::size_platform(&v_7.value));
    } else if matches!(v, crate::unions::Union10::U6(_)) {
        let mut v_8 = match v { crate::unions::Union10::U6(__v) => __v, _ => unreachable!() };
        return format!("<{} blobs>", crate::std_core_list::size_platform::<crate::std_core_bytes::Bytes>(&v_8.value));
    } else if matches!(v, crate::unions::Union10::U8(_)) {
        let mut v_9 = match v { crate::unions::Union10::U8(__v) => __v, _ => unreachable!() };
        return format!("[{} items]", crate::std_core_list::size_platform::<crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(&v_9.value));
    } else if matches!(v, crate::unions::Union10::U7(_)) {
        let mut v_10 = match v { crate::unions::Union10::U7(__v) => __v, _ => unreachable!() };
        return format!("{{{} attrs}}", crate::std_core_map::size_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(&v_10.value));
    } else {
        panic!("salvo: unhandled union arm at main:22:5");
    };
}

pub fn show_item(item: &crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>) -> String {
    let mut names: Vec<String> = vec![];
    for mut k in crate::std_core_map::keys_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(item).into_iter() {
        crate::std_core_list::add_platform::<String>(&mut names, (k).clone());
    }
    let mut ks: Vec<String> = names.clone();
    let mut parts: Vec<String> = vec![];
    for mut k in crate::std_core_list::sort::<String>(&ks, &mut |__a0: &String, __a1: &String| (Ord::cmp(&__a0[..], &__a1[..]) as i32)).into_iter() {
        let mut v: Option<&crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> = crate::std_core_map::get_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(item, &k, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if v.is_none() {
            continue;
        };
        let mut v_1 = v.unwrap();
        crate::std_core_list::add_platform::<String>(&mut parts, format!("{}={}", k, crate::show((&((v_1).clone())).as_ref())));
    }
    let mut joined: Vec<String> = parts.clone();
    return format!("{{{}}}", crate::std_core_string::join_platform(&joined, &String::from(", ")));
}

pub fn rebuild(item: &crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>) -> crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> {
    let mut out: crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> = crate::std_core_map::Mut_Map_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    for mut k in crate::platform_std_core_map::each(item).map(|__x| __x.clone()) {
        let mut v: Option<&crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> = crate::std_core_map::get_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(item, &k, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if v.is_none() {
            continue;
        };
        let mut v_1 = v.unwrap();
        crate::std_core_map::put_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(&mut out, (k).clone(), (v_1).clone(), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    }
    let mut result: crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> = out.clone();
    return result;
}

pub fn s(mut value: String) -> crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull> {
    return crate::unions::Union10::U1(crate::aws_ddb::AttributeValueS { value: value });
}

pub fn n(mut value: String) -> crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull> {
    return crate::unions::Union10::U2(crate::aws_ddb::AttributeValueN { value: value });
}

pub fn boolean(mut value: bool) -> crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull> {
    return crate::unions::Union10::U9(crate::aws_ddb::AttributeValueBool { value: value });
}

pub fn no_table(mut name: String) -> crate::aws_ddb::DdbError {
    return crate::aws_ddb::DdbError { code: String::from("ResourceNotFoundException"), message: format!("no table named {}", name), status: 400i32, request_id: None, item: None, throttling_reasons: None };
}

pub fn round_trip(ddb: &crate::aws_ddb::Ddb, console: &crate::std_core_console::Console, table: &String) {
    let mut made: crate::unions::Union2<crate::aws_ddb::CreateTableOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        ddb.create_table(crate::aws_ddb::CreateTableInput { attribute_definitions: Some(vec![crate::aws_ddb::AttributeDefinition { attribute_name: String::from("id"), attribute_type: String::from("S") }]), table_name: (table).clone(), key_schema: Some(vec![crate::aws_ddb::KeySchemaElement { attribute_name: String::from("id"), key_type: String::from("HASH") }]), local_secondary_indexes: None, global_secondary_indexes: None, billing_mode: None, provisioned_throughput: None, stream_specification: None, sse_specification: None, tags: None, table_class: None, deletion_protection_enabled: None, warm_throughput: None, resource_policy: None, on_demand_throughput: None, global_table_source_arn: None, global_table_settings_replication_mode: None, vector_indexes: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_ddb::CreateTableOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(made, crate::unions::Union2::U1(_)) {
        let mut made_1 = match &made { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("created table {}", table));
    } else if matches!(made, crate::unions::Union2::U2(_)) {
        let mut made_2 = match made { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("create_table: {}", crate::describe((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(made_2))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:107:5");
    };
    let mut item: crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> = crate::std_core_map::Map_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(vec![(String::from("id"), crate::s(String::from("user-42"))), (String::from("name"), crate::s(String::from("Ada"))), (String::from("age"), crate::n(String::from("37"))), (String::from("active"), crate::boolean(true))], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_console::println(console, &format!("put {}", crate::show_item(&item)));
    let mut put: crate::unions::Union2<crate::aws_ddb::PutItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        ddb.put_item(crate::aws_ddb::PutItemInput { table_name: (table).clone(), item: item, expected: None, return_values: None, return_consumed_capacity: None, return_item_collection_metrics: None, conditional_operator: None, condition_expression: None, expression_attribute_names: None, expression_attribute_values: None, return_values_on_condition_check_failure: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_ddb::PutItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(put, crate::unions::Union2::U1(_)) {
        let mut put_3 = match &put { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    } else if matches!(put, crate::unions::Union2::U2(_)) {
        let mut put_4 = match put { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("put_item: {}", crate::describe((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(put_4))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:124:5");
    };
    let mut got: crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        ddb.get_item(crate::aws_ddb::GetItemInput { table_name: (table).clone(), key: crate::std_core_map::Map_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(vec![(String::from("id"), crate::s(String::from("user-42")))], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])), attributes_to_get: None, consistent_read: None, return_consumed_capacity: None, projection_expression: None, expression_attribute_names: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(got, crate::unions::Union2::U1(_)) {
        let mut got_5 = match &got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut found = &got_5.item;
        if found.is_none() {
            crate::std_core_console::println(console, &String::from("got nothing"));
        } else {
            let mut found_6 = found.as_ref().unwrap();
            crate::std_core_console::println(console, &format!("got {}", crate::show_item(found_6)));
        };
    } else if matches!(got, crate::unions::Union2::U2(_)) {
        let mut got_7 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("get_item: {}", crate::describe((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(got_7))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:133:5");
    };
    let mut gone: crate::unions::Union2<crate::aws_ddb::DeleteItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        ddb.delete_item(crate::aws_ddb::DeleteItemInput { table_name: (table).clone(), key: crate::std_core_map::Map_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(vec![(String::from("id"), crate::s(String::from("user-42")))], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])), expected: None, conditional_operator: None, return_values: None, return_consumed_capacity: None, return_item_collection_metrics: None, condition_expression: None, expression_attribute_names: None, expression_attribute_values: None, return_values_on_condition_check_failure: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_ddb::DeleteItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(gone, crate::unions::Union2::U1(_)) {
        let mut gone_8 = match &gone { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &String::from("deleted user-42"));
    } else if matches!(gone, crate::unions::Union2::U2(_)) {
        let mut gone_9 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("delete_item: {}", crate::describe((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(gone_9))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:149:5");
    };
}

pub fn describe(mut e: crate::unions::Union2<&crate::aws_ddb::DdbError, &crate::aws::AwsError>) -> String {
    if matches!(e, crate::unions::Union2::U2(_)) {
        let mut e_1 = match e { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return format!("{}: {}", e_1.code, e_1.message);
    };
    let mut e_2 = match e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    if (match &e_2.code { __v => __v == &"ResourceNotFoundException", #[allow(unreachable_patterns)] _ => false }) {
        return String::from("no such table");
    };
    let mut e_3 = match e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    return format!("the service refused: {}", e_3.code);
}

pub struct MemDdb {
    rows: crate::std_core_map::Map<String, crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>>,
    tables: crate::std_core_set::Set<String>,
}

impl MemDdb {
    pub fn new() -> Self {
        Self {
            rows: crate::std_core_map::Mut_Map_platform::<String, crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])),
            tables: crate::std_core_set::Mut_Set_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))
        }
    }
}

impl crate::aws_ddb::__Stateful_Ddb for MemDdb {
    fn get_item(&mut self, input: crate::aws_ddb::GetItemInput, reply: crate::scheduler::SalvoReply) {
        if !(crate::std_core_set::contains_platform::<String>(&self.tables, &input.table_name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>(crate::std_core_checked::checked::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_table((input.table_name).clone())))))));
            return;
        };
        let mut row_key: String = format!("{}\t{}", input.table_name, crate::key_text(&input.key, String::from("id")));
        let mut row: Option<&crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>> = crate::std_core_map::get_platform::<String, crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>>(&self.rows, &row_key, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if row.is_none() {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::GetItemOutput>(crate::aws_ddb::GetItemOutput { item: None, consumed_capacity: None }))));
            return;
        };
        let mut row_1 = row.unwrap();
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::GetItemOutput>(crate::aws_ddb::GetItemOutput { item: Some(crate::rebuild(row_1)), consumed_capacity: None }))));
    }
    fn put_item(&mut self, input: crate::aws_ddb::PutItemInput, reply: crate::scheduler::SalvoReply) {
        if !(crate::std_core_set::contains_platform::<String>(&self.tables, &input.table_name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::PutItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>(crate::std_core_checked::checked::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_table((input.table_name).clone())))))));
            return;
        };
        let mut row_key: String = format!("{}\t{}", input.table_name, crate::key_text(&input.item, String::from("id")));
        crate::std_core_map::put_platform::<String, crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>>(&mut self.rows, row_key, crate::rebuild(&input.item), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::PutItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::PutItemOutput>(crate::aws_ddb::PutItemOutput { attributes: None, consumed_capacity: None, item_collection_metrics: None }))));
    }
    fn delete_item(&mut self, input: crate::aws_ddb::DeleteItemInput, reply: crate::scheduler::SalvoReply) {
        if !(crate::std_core_set::contains_platform::<String>(&self.tables, &input.table_name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::DeleteItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>(crate::std_core_checked::checked::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_table((input.table_name).clone())))))));
            return;
        };
        let mut row_key: String = format!("{}\t{}", input.table_name, crate::key_text(&input.key, String::from("id")));
        if crate::std_core_map::contains_key_platform::<String, crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>>(&self.rows, &row_key, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])) {
            crate::std_core_map::remove_platform::<String, crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>>(&mut self.rows, &row_key, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        };
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::DeleteItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::DeleteItemOutput>(crate::aws_ddb::DeleteItemOutput { attributes: None, consumed_capacity: None, item_collection_metrics: None }))));
    }
    fn query(&mut self, input: crate::aws_ddb::QueryInput, reply: crate::scheduler::SalvoReply) {
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::QueryOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::QueryOutput>(crate::aws_ddb::QueryOutput { items: None, count: 0i32, scanned_count: 0i32, last_evaluated_key: None, consumed_capacity: None }))));
    }
    fn create_table(&mut self, input: crate::aws_ddb::CreateTableInput, reply: crate::scheduler::SalvoReply) {
        crate::std_core_set::add_platform::<String>(&mut self.tables, (input.table_name).clone(), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::CreateTableOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::CreateTableOutput>(crate::aws_ddb::CreateTableOutput { table_description: None }))));
    }
    fn delete_table(&mut self, input: crate::aws_ddb::DeleteTableInput, reply: crate::scheduler::SalvoReply) {
        if !(crate::std_core_set::contains_platform::<String>(&self.tables, &input.table_name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::DeleteTableOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>(crate::std_core_checked::checked::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_table((input.table_name).clone())))))));
            return;
        };
        crate::std_core_set::remove_platform::<String>(&mut self.tables, &input.table_name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_ddb::DeleteTableOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::std_core_result::ok::<crate::aws_ddb::DeleteTableOutput>(crate::aws_ddb::DeleteTableOutput { table_description: None }))));
    }
}

pub fn key_text(item: &crate::std_core_map::Map<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>, mut name: String) -> String {
    let mut found: Option<&crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>> = crate::std_core_map::get_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(item, &name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    if found.is_none() {
        return String::from("");
    };
    let mut found_1 = found.unwrap();
    let mut v: crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull> = (found_1).clone();
    return if matches!(v, crate::unions::Union10::U1(_)) {
        let mut v_2 = match &v { crate::unions::Union10::U1(__v) => __v, _ => unreachable!() };
        return (v_2.value).clone();
    } else if matches!(v, crate::unions::Union10::U2(_)) {
        let mut v_3 = match &v { crate::unions::Union10::U2(__v) => __v, _ => unreachable!() };
        return (v_3.value).clone();
    } else if matches!(v, crate::unions::Union10::U3(_)) {
        let mut v_4 = match &v { crate::unions::Union10::U3(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U4(_)) {
        let mut v_5 = match &v { crate::unions::Union10::U4(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U5(_)) {
        let mut v_6 = match &v { crate::unions::Union10::U5(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U6(_)) {
        let mut v_7 = match &v { crate::unions::Union10::U6(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U7(_)) {
        let mut v_8 = match &v { crate::unions::Union10::U7(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U8(_)) {
        let mut v_9 = match &v { crate::unions::Union10::U8(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U9(_)) {
        let mut v_10 = match &v { crate::unions::Union10::U9(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else if matches!(v, crate::unions::Union10::U10(_)) {
        let mut v_11 = match &v { crate::unions::Union10::U10(__v) => __v, _ => unreachable!() };
        return String::from("");
    } else {
        panic!("salvo: unhandled union arm at main:246:5");
    };
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::std_core_actor::__PROTO_Faults.to_string()), ("Timer".to_string(), crate::std_time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::std_time::__PROTO_TimerCtl.to_string()), ("Wheel".to_string(), crate::std_runtime_timers::__PROTO_Wheel.to_string())]);
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    crate::std_core_console::println(&__handle_2, &String::from("-- FakeDdb --"));
    {
        let __use_3 = std::sync::Arc::new(std::sync::Mutex::new(crate::aws_ddb::FakeDdb::new()));
        let __handle_4 = crate::aws_ddb::Ddb::share_locked(__use_3.clone());
        let __handle_5 = crate::aws_ddb::DdbCalls::share_locked(__use_3.clone());
        crate::round_trip(&__handle_4, &__handle_2, &String::from("users"));
        crate::std_core_console::println(&__handle_2, &format!("calls: {}", crate::std_core_list::to_str(&__handle_5.calls(), &mut |__a0| format!("{}", __a0))));
    };
    crate::std_core_console::println(&__handle_2, &String::from("-- MemDdb --"));
    {
        let mut __use_6: crate::MemDdb = crate::MemDdb::new();
        let __handle_7 = crate::aws_ddb::Ddb::locked(__use_6);
        crate::round_trip(&__handle_7, &__handle_2, &String::from("users"));
        let mut missing: crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>> = {
            let (mut r, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
            __handle_7.get_item(crate::aws_ddb::GetItemInput { table_name: String::from("nowhere"), key: crate::std_core_map::Map_platform::<String, crate::unions::Union10<crate::aws_ddb::AttributeValueS, crate::aws_ddb::AttributeValueN, crate::aws_ddb::AttributeValueB, crate::aws_ddb::AttributeValueSs, crate::aws_ddb::AttributeValueNs, crate::aws_ddb::AttributeValueBs, crate::aws_ddb::AttributeValueM, crate::aws_ddb::AttributeValueL, crate::aws_ddb::AttributeValueBool, crate::aws_ddb::AttributeValueNull>>(vec![(String::from("id"), crate::s(String::from("x")))], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])), attributes_to_get: None, consistent_read: None, return_consumed_capacity: None, projection_expression: None, expression_attribute_names: None }, r);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_ddb::GetItemOutput, crate::std_core_checked::Checked<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>>>().expect("the awaited answer")
        };
        if matches!(missing, crate::unions::Union2::U1(_)) {
            let mut missing_8 = match &missing { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(&__handle_2, &String::from("unexpected ok"));
        } else if matches!(missing, crate::unions::Union2::U2(_)) {
            let mut missing_9 = match missing { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(&__handle_2, &format!("get_item: {}", crate::describe((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::aws_ddb::DdbError, crate::aws::AwsError>>(missing_9))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:281:9");
        };
    };
}
