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
#[path = "aws/sqs.rs"]
pub mod aws_sqs;
#[path = "core/actor.rs"]
pub mod core_actor;
#[path = "core/basic.rs"]
pub mod core_basic;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/compare.rs"]
pub mod core_compare;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/deque.rs"]
pub mod core_deque;
#[path = "core/index.rs"]
pub mod core_index;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/other.rs"]
pub mod core_other;
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "runtime/routing.rs"]
pub mod runtime_routing;
#[path = "platform/core/bytes.rs"]
pub mod platform_core_bytes;
#[path = "platform/core/console.rs"]
pub mod platform_core_console;
#[path = "platform/core/deque.rs"]
pub mod platform_core_deque;
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
#[path = "platform/runtime/routing.rs"]
pub mod platform_runtime_routing;
#[path = "platform/runtime.rs"]
pub mod platform_runtime;

use crate::aws::AwsError;
use crate::core_checked::Checked;
use crate::core_console::Console;
use crate::aws_sqs::CreateQueueInput;
use crate::aws_sqs::CreateQueueOutput;
use crate::aws_sqs::DeleteMessageInput;
use crate::aws_sqs::DeleteQueueInput;
use crate::aws_sqs::GetQueueUrlInput;
use crate::aws_sqs::GetQueueUrlOutput;
use crate::core_map::Map;
use crate::aws_sqs::Message;
use crate::core_map::Mut_Map_platform;
use crate::aws_sqs::ReceiveMessageInput;
use crate::aws_sqs::ReceiveMessageOutput;
use crate::aws_sqs::SendMessageInput;
use crate::aws_sqs::SendMessageOutput;
use crate::aws_sqs::Sqs;
use crate::aws_sqs::SqsError;
use crate::core_list::add_platform;
use crate::core_checked::checked;
use crate::core_map::contains_key_platform;
use crate::core_checked::detach;
use crate::core_result::err;
use crate::core_map::get_platform;
use crate::core_result::ok;
use crate::core_console::println;
use crate::core_map::put_platform;
use crate::core_map::remove_platform;
use crate::core_list::size_platform;
use crate::core_list::to_str;


pub fn describe(mut e: crate::unions::Union2<&crate::aws_sqs::SqsError, &crate::aws::AwsError>) -> String {
    if matches!(e, crate::unions::Union2::U2(_)) {
        let mut e_1 = match e { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return format!("{}: {}", e_1.code, e_1.message);
    };
    let mut e_2 = match e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    if (match &e_2.code { __v => __v == &"QueueDoesNotExist", #[allow(unreachable_patterns)] _ => false }) {
        return String::from("no such queue");
    };
    let mut e_3 = match e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    return format!("the service refused: {}", e_3.code.clone());
}

pub fn round_trip(sqs: &crate::aws_sqs::Sqs, console: &crate::core_console::Console, name: &String) {
    let mut created: crate::unions::Union2<crate::aws_sqs::CreateQueueOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union2<crate::aws_sqs::CreateQueueOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.create_queue(crate::aws_sqs::CreateQueueInput { queue_name: (name).clone(), attributes: None, tags: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_sqs::CreateQueueOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(created, crate::unions::Union2::U2(_)) {
        let mut created_1 = match created { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("create_queue: {}", crate::describe((&(crate::core_checked::detach::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(created_1))).as_ref())));
        return;
    };
    let mut created_2 = match &created { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut url: String = {
        let mut __pick_3 = &created_2.queue_url;
        if __pick_3.is_some() {
            let mut __picked_4 = __pick_3.as_ref().unwrap();
            __picked_4.clone()
        } else {
            String::from("?")
        }
    };
    crate::core_console::println(console, &format!("created {}", url));
    let mut sent: crate::unions::Union2<crate::aws_sqs::SendMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union2<crate::aws_sqs::SendMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.send_message(crate::aws_sqs::SendMessageInput { queue_url: (url).clone(), message_body: String::from("hello from Salvo"), delay_seconds: None, message_attributes: None, message_system_attributes: None, message_deduplication_id: None, message_group_id: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_sqs::SendMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(sent, crate::unions::Union2::U1(_)) {
        let mut sent_6 = match &sent { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("sent {}", {
            let mut __pick_7 = &sent_6.message_id;
            if __pick_7.is_some() {
                let mut __picked_8 = __pick_7.as_ref().unwrap();
                __picked_8.clone()
            } else {
                String::from("?")
            }
        }));
    } else if matches!(sent, crate::unions::Union2::U2(_)) {
        let mut sent_10 = match sent { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("send_message: {}", crate::describe((&(crate::core_checked::detach::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(sent_10))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:47:5");
    };
    let mut got: crate::unions::Union2<crate::aws_sqs::ReceiveMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        sqs.receive_message(crate::aws_sqs::ReceiveMessageInput { queue_url: (url).clone(), attribute_names: None, message_system_attribute_names: None, message_attribute_names: None, max_number_of_messages: Some(10i32), visibility_timeout: None, wait_time_seconds: None, receive_request_attempt_id: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_sqs::ReceiveMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(got, crate::unions::Union2::U1(_)) {
        let mut got_11 = match &got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut messages: Vec<crate::aws_sqs::Message> = {
            let mut __pick_12 = &got_11.messages;
            if __pick_12.is_some() {
                let mut __picked_13 = __pick_12.as_ref().unwrap();
                __picked_13.clone()
            } else {
                vec![]
            }
        };
        crate::core_console::println(console, &format!("received {} message(s)", crate::core_list::size_platform::<crate::aws_sqs::Message>(&messages)));
        for mut m in messages.iter() {
            crate::core_console::println(console, &format!("  {}", {
                let mut __pick_15 = &m.body;
                if __pick_15.is_some() {
                    let mut __picked_16 = __pick_15.as_ref().unwrap();
                    __picked_16.clone()
                } else {
                    String::from("")
                }
            }));
        }
    } else if matches!(got, crate::unions::Union2::U2(_)) {
        let mut got_18 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("receive_message: {}", crate::describe((&(crate::core_checked::detach::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(got_18))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:55:5");
    };
    let mut gone: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.delete_queue(crate::aws_sqs::DeleteQueueInput { queue_url: (url).clone() }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(gone, crate::unions::Union2::U1(_)) {
        let mut gone_19 = match &gone { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &String::from("deleted"));
    } else if matches!(gone, crate::unions::Union2::U2(_)) {
        let mut gone_20 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("delete_queue: {}", crate::describe((&(crate::core_checked::detach::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(gone_20))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:69:5");
    };
}

pub fn no_queue(mut url: String) -> crate::aws_sqs::SqsError {
    return crate::aws_sqs::SqsError { code: String::from("QueueDoesNotExist"), message: format!("no queue at {}", url), status: 400i32, request_id: None };
}

pub struct MemSqs {
    queues: crate::core_map::Map<String, Vec<String>>,
}

impl MemSqs {
    pub fn new() -> Self {
        Self {
            queues: crate::core_map::Mut_Map_platform::<String, Vec<String>>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))
        }
    }
}

impl crate::aws_sqs::__Stateful_Sqs for MemSqs {
    fn create_queue(&mut self, input: crate::aws_sqs::CreateQueueInput, reply: crate::scheduler::SalvoReply) {
        let mut url: String = format!("mem://{}", input.queue_name);
        if !(crate::core_map::contains_key_platform::<String, Vec<String>>(&self.queues, &url, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            crate::core_map::put_platform::<String, Vec<String>>(&mut self.queues, (url).clone(), vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        };
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_sqs::CreateQueueOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_sqs::CreateQueueOutput>(crate::aws_sqs::CreateQueueOutput { queue_url: Some(url) })));
    }
    fn get_queue_url(&mut self, input: crate::aws_sqs::GetQueueUrlInput, reply: crate::scheduler::SalvoReply) {
        let mut url: String = format!("mem://{}", input.queue_name);
        if !(crate::core_map::contains_key_platform::<String, Vec<String>>(&self.queues, &url, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_sqs::GetQueueUrlOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_queue(url))))));
            return;
        };
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_sqs::GetQueueUrlOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_sqs::GetQueueUrlOutput>(crate::aws_sqs::GetQueueUrlOutput { queue_url: Some(url) })));
    }
    fn send_message(&mut self, input: crate::aws_sqs::SendMessageInput, reply: crate::scheduler::SalvoReply) {
        let mut held: Option<&Vec<String>> = crate::core_map::get_platform::<String, Vec<String>>(&self.queues, &input.queue_url, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if held.is_none() {
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_sqs::SendMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_queue((input.queue_url).clone()))))));
            return;
        };
        let mut grown: Vec<String> = vec![];
        let mut held_1 = held.unwrap();
        for mut b in held_1.iter() {
            crate::core_list::add_platform::<String>(&mut grown, (b).clone());
        }
        let mut id: String = format!("m{}", i32::wrapping_add(crate::core_list::size_platform::<String>(&grown), 1i32));
        crate::core_list::add_platform::<String>(&mut grown, (input.message_body).clone());
        let mut stored: Vec<String> = grown.clone();
        crate::core_map::put_platform::<String, Vec<String>>(&mut self.queues, (input.queue_url).clone(), stored, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_sqs::SendMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_sqs::SendMessageOutput>(crate::aws_sqs::SendMessageOutput { md5_of_message_body: None, md5_of_message_attributes: None, md5_of_message_system_attributes: None, message_id: Some(id), sequence_number: None })));
    }
    fn receive_message(&mut self, input: crate::aws_sqs::ReceiveMessageInput, reply: crate::scheduler::SalvoReply) {
        let mut held: Option<&Vec<String>> = crate::core_map::get_platform::<String, Vec<String>>(&self.queues, &input.queue_url, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if held.is_none() {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_sqs::ReceiveMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_queue((input.queue_url).clone())))))));
            return;
        };
        let mut out: Vec<crate::aws_sqs::Message> = vec![];
        let mut i: i32 = 0i32;
        let mut held_1 = held.unwrap();
        for mut b in held_1.iter() {
            i = i32::wrapping_add(i, 1i32);
            crate::core_list::add_platform::<crate::aws_sqs::Message>(&mut out, crate::aws_sqs::Message { message_id: Some(format!("m{}", i)), receipt_handle: Some(format!("m{}", i)), md5_of_body: None, body: Some((b).clone()), attributes: None, md5_of_message_attributes: None, message_attributes: None });
        }
        let mut messages: Vec<crate::aws_sqs::Message> = out.clone();
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_sqs::ReceiveMessageOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_sqs::ReceiveMessageOutput>(crate::aws_sqs::ReceiveMessageOutput { messages: Some(messages) }))));
    }
    fn delete_message(&mut self, input: crate::aws_sqs::DeleteMessageInput, reply: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<()>(())));
    }
    fn delete_queue(&mut self, input: crate::aws_sqs::DeleteQueueInput, reply: crate::scheduler::SalvoReply) {
        if !(crate::core_map::contains_key_platform::<String, Vec<String>>(&self.queues, &input.queue_url, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::no_queue((input.queue_url).clone()))))));
            return;
        };
        crate::core_map::remove_platform::<String, Vec<String>>(&mut self.queues, &input.queue_url, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<()>(())));
    }
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string())]);
    let mut __use_1: crate::core_console::__Platform_StdConsole = crate::core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    crate::core_console::println(&__handle_2, &String::from("-- FakeSqs --"));
    {
        let __use_3 = std::sync::Arc::new(std::sync::Mutex::new(crate::aws_sqs::FakeSqs::new()));
        let __handle_4 = crate::aws_sqs::Sqs::share_locked(__use_3.clone());
        let __handle_5 = crate::aws_sqs::SqsCalls::share_locked(__use_3.clone());
        crate::round_trip(&__handle_4, &__handle_2, &String::from("orders"));
        crate::core_console::println(&__handle_2, &format!("calls: {}", crate::core_list::to_str(&__handle_5.calls(), &mut |__a0| format!("{}", __a0))));
    };
    crate::core_console::println(&__handle_2, &String::from("-- MemSqs --"));
    {
        let mut __use_6: crate::MemSqs = crate::MemSqs::new();
        let __handle_7 = crate::aws_sqs::Sqs::locked(__use_6);
        crate::round_trip(&__handle_7, &__handle_2, &String::from("orders"));
        let mut missing: crate::unions::Union2<crate::aws_sqs::GetQueueUrlOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>> = {
            let (mut r, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union2<crate::aws_sqs::GetQueueUrlOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            __handle_7.get_queue_url(crate::aws_sqs::GetQueueUrlInput { queue_name: String::from("nowhere"), queue_owner_aws_account_id: None }, r);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_sqs::GetQueueUrlOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>>>().expect("the awaited answer")
        };
        if matches!(missing, crate::unions::Union2::U1(_)) {
            let mut missing_8 = match &missing { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::core_console::println(&__handle_2, &format!("unexpected: {}", {
                let mut __pick_9 = &missing_8.queue_url;
                if __pick_9.is_some() {
                    let mut __picked_10 = __pick_9.as_ref().unwrap();
                    __picked_10.clone()
                } else {
                    String::from("?")
                }
            }));
        } else if matches!(missing, crate::unions::Union2::U2(_)) {
            let mut missing_12 = match missing { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(&__handle_2, &format!("get_queue_url: {}", crate::describe((&(crate::core_checked::detach::<crate::unions::Union2<crate::aws_sqs::SqsError, crate::aws::AwsError>>(missing_12))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:179:9");
        };
    };
}
