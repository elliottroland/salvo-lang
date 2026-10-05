#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions/mod.rs"]
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
#[path = "aws.rs"]
pub mod aws;
#[path = "aws/sqs.rs"]
pub mod aws_sqs;
#[path = "core/actor.rs"]
pub mod core_actor;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/deque.rs"]
pub mod core_deque;
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
#[path = "platform/core/string.rs"]
pub mod platform_core_string;
#[path = "platform/runtime/routing.rs"]
pub mod platform_runtime_routing;
#[path = "platform/runtime.rs"]
pub mod platform_runtime;

use crate::collections::*;
use crate::unions::*;
use crate::aws::AwsError;
use crate::aws_sqs::CreateQueueInput;
use crate::aws_sqs::CreateQueueOutput;
use crate::aws_sqs::DeleteMessageInput;
use crate::aws_sqs::DeleteQueueInput;
use crate::aws_sqs::FakeSqs;
use crate::aws_sqs::GetQueueUrlInput;
use crate::aws_sqs::GetQueueUrlOutput;
use crate::aws_sqs::Message;
use crate::aws_sqs::ReceiveMessageInput;
use crate::aws_sqs::ReceiveMessageOutput;
use crate::aws_sqs::SendMessageInput;
use crate::aws_sqs::SendMessageOutput;
use crate::aws_sqs::SqsError;
use crate::aws_sqs::__Stateful_Sqs as _;
use crate::aws_sqs::__Stateful_SqsCalls as _;
use crate::aws_sqs::__Stateless_Sqs as _;
use crate::aws_sqs::__Stateless_SqsCalls as _;
use crate::core_actor::__Stateful_Faults as _;
use crate::core_actor::__Stateless_Faults as _;
use crate::core_checked::Checked;
use crate::core_checked::checked;
use crate::core_checked::detach;
use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;
use crate::core_list::to_str;
use crate::core_result::err;
use crate::core_result::ok;

pub fn describe(e: &Union2<SqsError, AwsError>) -> String {
    if matches!(e, Union2::U2(_)) {
        return format!("{}: {}", e.u2().clone().code.clone(), e.u2().clone().message.clone());
    }
    if matches!(e.u1().clone().code, ref __v if (*__v == "QueueDoesNotExist")) {
        return "no such queue".to_string();
    }
    return format!("the service refused: {}", format!("{}", e.u1().clone().code.clone()));
}

pub fn round_trip(sqs: &crate::aws_sqs::Sqs, console: &crate::core_console::Console, name: &String) {
    let mut created = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.create_queue(CreateQueueInput { queue_name: name.clone(), attributes: None, tags: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>>().expect("the awaited answer")
    };
    if matches!(created, Union2::U2(_)) {
        println(console, &(format!("create_queue: {}", describe(&(detach((match created { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        return;
    }
    let mut url = if created.u1().clone().queue_url.is_some() { created.u1().clone().queue_url.as_ref().unwrap().clone() } else { "?".to_string() };
    println(console, &(format!("created {}", url)));
    let mut sent = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.send_message(SendMessageInput { queue_url: url.clone(), message_body: "hello from Salvo".to_string(), delay_seconds: None, message_attributes: None, message_system_attributes: None, message_deduplication_id: None, message_group_id: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>>().expect("the awaited answer")
    };
    match sent {
        Union2::U1(_) => {
            println(console, &(format!("sent {}", if sent.u1().clone().message_id.is_some() { sent.u1().clone().message_id.as_ref().unwrap().clone() } else { "?".to_string() })));
        }
        Union2::U2(_) => {
            println(console, &(format!("send_message: {}", describe(&(detach((match sent { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut got = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        sqs.receive_message(ReceiveMessageInput { queue_url: url.clone(), max_number_of_messages: Some(10), attribute_names: None, message_system_attribute_names: None, message_attribute_names: None, visibility_timeout: None, wait_time_seconds: None, receive_request_attempt_id: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>>().expect("the awaited answer")
    };
    match got {
        Union2::U1(_) => {
            let mut messages: Vec<Message> = if got.u1().clone().messages.is_some() { got.u1().clone().messages.as_ref().unwrap().clone() } else { vec![] };
            println(console, &(format!("received {} message(s)", crate::core_list::size_platform(&messages))));
            for m in crate::platform_core_list::each(&messages) {
                println(console, &(format!("  {}", if m.body.is_some() { m.body.as_ref().unwrap().clone() } else { "".to_string() })));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("receive_message: {}", describe(&(detach((match got { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut gone = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<(), Checked<Union2<SqsError, AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.delete_queue(DeleteQueueInput { queue_url: url.clone() }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<(), Checked<Union2<SqsError, AwsError>>>>().expect("the awaited answer")
    };
    match gone {
        Union2::U1(_) => {
            println(console, &("deleted".to_string()));
        }
        Union2::U2(_) => {
            println(console, &(format!("delete_queue: {}", describe(&(detach((match gone { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
}

pub fn no_queue(url: String) -> SqsError {
    return SqsError { code: "QueueDoesNotExist".to_string(), message: format!("no queue at {}", url), status: 400, request_id: None };
}

pub struct MemSqs {
    queues: SalvoMap<String, Vec<String>>,
}

impl MemSqs {
    pub fn new() -> Self {
        Self {
            queues: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
        }
    }
}

impl crate::aws_sqs::__Stateful_Sqs for MemSqs {

    fn create_queue(&mut self, input: CreateQueueInput, reply: crate::scheduler::SalvoReply) {
        let mut url = format!("mem://{}", input.queue_name.clone());
        if !self.queues.contains_key(&url) {
            self.queues.insert(url.clone(), vec![]);
        }
        crate::scheduler::salvo_reply_wire::<Union2<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(CreateQueueOutput { queue_url: Some(url) })));
    }

    fn get_queue_url(&mut self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply) {
        let mut url = format!("mem://{}", input.queue_name.clone());
        if !self.queues.contains_key(&url) {
            crate::scheduler::salvo_reply_wire::<Union2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>::U2(err(checked(Union2::<SqsError, AwsError>::U1(no_queue(url))))));
            return;
        }
        crate::scheduler::salvo_reply_wire::<Union2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(GetQueueUrlOutput { queue_url: Some(url) })));
    }

    fn send_message(&mut self, input: SendMessageInput, reply: crate::scheduler::SalvoReply) {
        let mut held = self.queues.get(&input.queue_url);
        if held.is_none() {
            crate::scheduler::salvo_reply_wire::<Union2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>::U2(err(checked(Union2::<SqsError, AwsError>::U1(no_queue(input.queue_url.clone()))))));
            return;
        }
        let mut grown = vec![];
        for mut b in crate::platform_core_list::each(&(held.unwrap().clone())).map(|__x| __x.clone()) {
            crate::core_list::add_platform(&mut grown, b.clone());
        }
        let mut id = format!("m{}", crate::core_list::size_platform(&grown) + 1);
        crate::core_list::add_platform(&mut grown, input.message_body.clone());
        let mut stored: Vec<String> = grown;
        self.queues.insert(input.queue_url.clone(), stored);
        crate::scheduler::salvo_reply_wire::<Union2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(SendMessageOutput { message_id: Some(id), md5_of_message_body: None, md5_of_message_attributes: None, md5_of_message_system_attributes: None, sequence_number: None })));
    }

    fn receive_message(&mut self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply) {
        let mut held = self.queues.get(&input.queue_url);
        if held.is_none() {
            (reply).send(std::boxed::Box::new(Union2::<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>::U2(err(checked(Union2::<SqsError, AwsError>::U1(no_queue(input.queue_url.clone())))))));
            return;
        }
        let mut out = vec![];
        let mut i = 0;
        for mut b in crate::platform_core_list::each(&(held.unwrap().clone())).map(|__x| __x.clone()) {
            i = i + 1;
            crate::core_list::add_platform(&mut out, Message { message_id: Some(format!("m{}", i)), receipt_handle: Some(format!("m{}", i)), body: Some(b.clone()), md5_of_body: None, attributes: None, md5_of_message_attributes: None, message_attributes: None });
        }
        let mut messages: Vec<Message> = out;
        (reply).send(std::boxed::Box::new(Union2::<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(ReceiveMessageOutput { messages: Some(messages) }))));
    }

    fn delete_message(&mut self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<(), Checked<Union2<SqsError, AwsError>>>::U1(ok(())));
    }

    fn delete_queue(&mut self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply) {
        if !self.queues.contains_key(&input.queue_url) {
            crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<(), Checked<Union2<SqsError, AwsError>>>::U2(err(checked(Union2::<SqsError, AwsError>::U1(no_queue(input.queue_url.clone()))))));
            return;
        }
        self.queues.remove(&input.queue_url);
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<(), Checked<Union2<SqsError, AwsError>>>::U1(ok(())));
    }
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string())]);
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    println(&console, &("-- FakeSqs --".to_string()));
    if true {
        let __inst = std::sync::Arc::new(std::sync::Mutex::new(FakeSqs::new()));
        let sqs2 = crate::aws_sqs::Sqs::share_locked(__inst.clone());
        let sqs_calls = crate::aws_sqs::SqsCalls::share_locked(__inst.clone());
        round_trip(&sqs2, &console, &("orders".to_string()));
        println(&console, &(format!("calls: {}", to_str(&sqs_calls.calls(), &mut |__i0| format!("{}", __i0)))));
    }
    println(&console, &("-- MemSqs --".to_string()));
    if true {
        let sqs3 = crate::aws_sqs::Sqs::locked(MemSqs::new());
        round_trip(&sqs3, &console, &("orders".to_string()));
        let mut missing = {
            let (mut r, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            sqs3.get_queue_url(GetQueueUrlInput { queue_name: "nowhere".to_string(), queue_owner_aws_account_id: None }, r);
            *crate::scheduler::salvo_wait(__wid).downcast::<Union2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>>().expect("the awaited answer")
        };
        match missing {
            Union2::U1(_) => {
                println(&console, &(format!("unexpected: {}", if missing.u1().clone().queue_url.is_some() { missing.u1().clone().queue_url.as_ref().unwrap().clone() } else { "?".to_string() })));
            }
            Union2::U2(_) => {
                println(&console, &(format!("get_queue_url: {}", describe(&(detach((match missing { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
    }
}
