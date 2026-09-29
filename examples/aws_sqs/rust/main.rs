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
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;

use crate::aws::*;
use crate::aws_sqs::*;
use crate::collections::*;
use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn describe(e: &Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>) -> String {
    if matches!(e, Union21::U21(_)) {
        return format!("{}: {}", e.u21().clone().code.clone(), e.u21().clone().message.clone());
    }
    if matches!(e, Union21::U16(_)) {
        return "no such queue".to_string();
    }
    return "the service refused".to_string();
}

pub fn round_trip(sqs: &crate::aws_sqs::Sqs, console: &crate::core_console::Console, name: &String) {
    let mut created = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.create_queue(CreateQueueInput { queue_name: name.clone(), attributes: None, tags: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>().expect("the awaited answer")
    };
    if matches!(created, Union2::U2(_)) {
        println(console, &(format!("create_queue: {}", describe(&(detach(created.u2().clone()))))));
        return;
    }
    let mut url = if created.u1().clone().queue_url.is_some() { created.u1().clone().queue_url.as_ref().unwrap().clone() } else { "?".to_string() };
    println(console, &(format!("created {}", url)));
    let mut sent = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.send_message(SendMessageInput { queue_url: url.clone(), message_body: "hello from Salvo".to_string(), delay_seconds: None, message_attributes: None, message_system_attributes: None, message_deduplication_id: None, message_group_id: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>().expect("the awaited answer")
    };
    match sent {
        Union2::U1(_) => {
            println(console, &(format!("sent {}", if sent.u1().clone().message_id.is_some() { sent.u1().clone().message_id.as_ref().unwrap().clone() } else { "?".to_string() })));
        }
        Union2::U2(_) => {
            println(console, &(format!("send_message: {}", describe(&(detach(sent.u2().clone()))))));
        }
    }
    let mut got = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        sqs.receive_message(ReceiveMessageInput { queue_url: url.clone(), max_number_of_messages: Some(10), attribute_names: None, message_system_attribute_names: None, message_attribute_names: None, visibility_timeout: None, wait_time_seconds: None, receive_request_attempt_id: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>().expect("the awaited answer")
    };
    match got {
        Union2::U1(_) => {
            let mut messages: Vec<Message> = if got.u1().clone().messages.is_some() { got.u1().clone().messages.as_ref().unwrap().clone() } else { vec![] };
            println(console, &(format!("received {} message(s)", (messages.len() as i32))));
            for m in &messages {
                println(console, &(format!("  {}", if m.body.is_some() { m.body.as_ref().unwrap().clone() } else { "".to_string() })));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("receive_message: {}", describe(&(detach(got.u2().clone()))))));
        }
    }
    let mut gone = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        sqs.delete_queue(DeleteQueueInput { queue_url: url.clone() }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>().expect("the awaited answer")
    };
    match gone {
        Union2::U1(_) => {
            println(console, &("deleted".to_string()));
        }
        Union2::U2(_) => {
            println(console, &(format!("delete_queue: {}", describe(&(detach(gone.u2().clone()))))));
        }
    }
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
        crate::scheduler::salvo_reply_wire::<Union2<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(CreateQueueOutput { queue_url: Some(url) })));
    }

    fn get_queue_url(&mut self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply) {
        let mut url = format!("mem://{}", input.queue_name.clone());
        if !self.queues.contains_key(&url) {
            crate::scheduler::salvo_reply_wire::<Union2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U2(err(checked(Union21::<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>::U16(QueueDoesNotExist { message: Some(url) })))));
            return;
        }
        crate::scheduler::salvo_reply_wire::<Union2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(GetQueueUrlOutput { queue_url: Some(url) })));
    }

    fn send_message(&mut self, input: SendMessageInput, reply: crate::scheduler::SalvoReply) {
        let mut held = self.queues.get(&input.queue_url);
        if held.is_none() {
            crate::scheduler::salvo_reply_wire::<Union2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U2(err(checked(Union21::<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>::U16(QueueDoesNotExist { message: Some(input.queue_url.clone()) })))));
            return;
        }
        let mut grown = vec![];
        for mut b in held.unwrap().clone() {
            grown.push(b.clone());
        }
        let mut id = format!("m{}", (grown.len() as i32) + 1);
        grown.push(input.message_body.clone());
        let mut stored: Vec<String> = grown;
        self.queues.insert(input.queue_url.clone(), stored);
        crate::scheduler::salvo_reply_wire::<Union2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(SendMessageOutput { message_id: Some(id), md5_of_message_body: None, md5_of_message_attributes: None, md5_of_message_system_attributes: None, sequence_number: None })));
    }

    fn receive_message(&mut self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply) {
        let mut held = self.queues.get(&input.queue_url);
        if held.is_none() {
            (reply).send(Box::new(Union2::<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U2(err(checked(Union21::<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>::U16(QueueDoesNotExist { message: Some(input.queue_url.clone()) }))))));
            return;
        }
        let mut out = vec![];
        let mut i = 0;
        for mut b in held.unwrap().clone() {
            i = i + 1;
            out.push(Message { message_id: Some(format!("m{}", i)), receipt_handle: Some(format!("m{}", i)), body: Some(b.clone()), md5_of_body: None, attributes: None, md5_of_message_attributes: None, message_attributes: None });
        }
        let mut messages: Vec<Message> = out;
        (reply).send(Box::new(Union2::<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(ReceiveMessageOutput { messages: Some(messages) }))));
    }

    fn delete_message(&mut self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(())));
    }

    fn delete_queue(&mut self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply) {
        if !self.queues.contains_key(&input.queue_url) {
            crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U2(err(checked(Union21::<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>::U16(QueueDoesNotExist { message: Some(input.queue_url.clone()) })))));
            return;
        }
        self.queues.remove(&input.queue_url);
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(())));
    }
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string())]);
    let console = crate::core_console::Console::shared(StdOutConsole::new());
    println(&console, &("-- FakeSqs --".to_string()));
    if true {
        let __inst = std::sync::Arc::new(std::sync::Mutex::new(FakeSqs::new()));
        let sqs2 = crate::aws_sqs::Sqs::share_locked(__inst.clone());
        let sqs_calls = crate::aws_sqs::SqsCalls::share_locked(__inst.clone());
        round_trip(&sqs2, &console, &("orders".to_string()));
        println(&console, &(format!("calls: {}", format!("[{}]", sqs_calls.calls().iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    }
    println(&console, &("-- MemSqs --".to_string()));
    if true {
        let sqs3 = crate::aws_sqs::Sqs::locked(MemSqs::new());
        round_trip(&sqs3, &console, &("orders".to_string()));
        let mut missing = {
            let (mut r, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            sqs3.get_queue_url(GetQueueUrlInput { queue_name: "nowhere".to_string(), queue_owner_aws_account_id: None }, r);
            *crate::scheduler::salvo_wait(__wid).downcast::<Union2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>().expect("the awaited answer")
        };
        match missing {
            Union2::U1(_) => {
                println(&console, &(format!("unexpected: {}", if missing.u1().clone().queue_url.is_some() { missing.u1().clone().queue_url.as_ref().unwrap().clone() } else { "?".to_string() })));
            }
            Union2::U2(_) => {
                println(&console, &(format!("get_queue_url: {}", describe(&(detach(missing.u2().clone()))))));
            }
        }
    }
}
