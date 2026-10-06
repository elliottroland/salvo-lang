use crate::unions::*;
use crate::aws::AwsError;
use crate::core_actor::__Stateful_Faults as _;
use crate::core_actor::__Stateless_Faults as _;
use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::core_list::List;
use crate::core_map::Map;
use crate::core_result::ok;
use crate::core_string::Str;

#[derive(Clone, Debug, PartialEq)]
pub struct CreateQueueInput {
    pub queue_name: String,
    pub attributes: Option<Map<String, String>>,
    pub tags: Option<Map<String, String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateQueueOutput {
    pub queue_url: Option<String>,
}

impl crate::wire::__Wire for CreateQueueOutput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.queue_url, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            queue_url: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetQueueUrlInput {
    pub queue_name: String,
    pub queue_owner_aws_account_id: Option<String>,
}

impl crate::wire::__Wire for GetQueueUrlInput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.queue_name, out);
        crate::wire::__Wire::__enc(&self.queue_owner_aws_account_id, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            queue_name: crate::wire::__Wire::__dec(r)?,
            queue_owner_aws_account_id: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetQueueUrlOutput {
    pub queue_url: Option<String>,
}

impl crate::wire::__Wire for GetQueueUrlOutput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.queue_url, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            queue_url: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SendMessageInput {
    pub queue_url: String,
    pub message_body: String,
    pub delay_seconds: Option<i32>,
    pub message_attributes: Option<Map<String, MessageAttributeValue>>,
    pub message_system_attributes: Option<Map<String, MessageSystemAttributeValue>>,
    pub message_deduplication_id: Option<String>,
    pub message_group_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageAttributeValue {
    pub string_value: Option<String>,
    pub binary_value: Option<Bytes>,
    pub string_list_values: Option<Vec<String>>,
    pub binary_list_values: Option<Vec<Bytes>>,
    pub data_type: String,
}

impl crate::wire::__Wire for MessageAttributeValue {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.string_value, out);
        crate::wire::__Wire::__enc(&self.binary_value, out);
        crate::wire::__Wire::__enc(&self.string_list_values, out);
        crate::wire::__Wire::__enc(&self.binary_list_values, out);
        crate::wire::__Wire::__enc(&self.data_type, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            string_value: crate::wire::__Wire::__dec(r)?,
            binary_value: crate::wire::__Wire::__dec(r)?,
            string_list_values: crate::wire::__Wire::__dec(r)?,
            binary_list_values: crate::wire::__Wire::__dec(r)?,
            data_type: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeValue {
    pub string_value: Option<String>,
    pub binary_value: Option<Bytes>,
    pub string_list_values: Option<Vec<String>>,
    pub binary_list_values: Option<Vec<Bytes>>,
    pub data_type: String,
}

impl crate::wire::__Wire for MessageSystemAttributeValue {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.string_value, out);
        crate::wire::__Wire::__enc(&self.binary_value, out);
        crate::wire::__Wire::__enc(&self.string_list_values, out);
        crate::wire::__Wire::__enc(&self.binary_list_values, out);
        crate::wire::__Wire::__enc(&self.data_type, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            string_value: crate::wire::__Wire::__dec(r)?,
            binary_value: crate::wire::__Wire::__dec(r)?,
            string_list_values: crate::wire::__Wire::__dec(r)?,
            binary_list_values: crate::wire::__Wire::__dec(r)?,
            data_type: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SendMessageOutput {
    pub md5_of_message_body: Option<String>,
    pub md5_of_message_attributes: Option<String>,
    pub md5_of_message_system_attributes: Option<String>,
    pub message_id: Option<String>,
    pub sequence_number: Option<String>,
}

impl crate::wire::__Wire for SendMessageOutput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.md5_of_message_body, out);
        crate::wire::__Wire::__enc(&self.md5_of_message_attributes, out);
        crate::wire::__Wire::__enc(&self.md5_of_message_system_attributes, out);
        crate::wire::__Wire::__enc(&self.message_id, out);
        crate::wire::__Wire::__enc(&self.sequence_number, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            md5_of_message_body: crate::wire::__Wire::__dec(r)?,
            md5_of_message_attributes: crate::wire::__Wire::__dec(r)?,
            md5_of_message_system_attributes: crate::wire::__Wire::__dec(r)?,
            message_id: crate::wire::__Wire::__dec(r)?,
            sequence_number: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReceiveMessageInput {
    pub queue_url: String,
    pub attribute_names: Option<Vec<String>>,
    pub message_system_attribute_names: Option<Vec<String>>,
    pub message_attribute_names: Option<Vec<String>>,
    pub max_number_of_messages: Option<i32>,
    pub visibility_timeout: Option<i32>,
    pub wait_time_seconds: Option<i32>,
    pub receive_request_attempt_id: Option<String>,
}

impl crate::wire::__Wire for ReceiveMessageInput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.queue_url, out);
        crate::wire::__Wire::__enc(&self.attribute_names, out);
        crate::wire::__Wire::__enc(&self.message_system_attribute_names, out);
        crate::wire::__Wire::__enc(&self.message_attribute_names, out);
        crate::wire::__Wire::__enc(&self.max_number_of_messages, out);
        crate::wire::__Wire::__enc(&self.visibility_timeout, out);
        crate::wire::__Wire::__enc(&self.wait_time_seconds, out);
        crate::wire::__Wire::__enc(&self.receive_request_attempt_id, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            queue_url: crate::wire::__Wire::__dec(r)?,
            attribute_names: crate::wire::__Wire::__dec(r)?,
            message_system_attribute_names: crate::wire::__Wire::__dec(r)?,
            message_attribute_names: crate::wire::__Wire::__dec(r)?,
            max_number_of_messages: crate::wire::__Wire::__dec(r)?,
            visibility_timeout: crate::wire::__Wire::__dec(r)?,
            wait_time_seconds: crate::wire::__Wire::__dec(r)?,
            receive_request_attempt_id: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReceiveMessageOutput {
    pub messages: Option<Vec<Message>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub message_id: Option<String>,
    pub receipt_handle: Option<String>,
    pub md5_of_body: Option<String>,
    pub body: Option<String>,
    pub attributes: Option<Map<String, String>>,
    pub md5_of_message_attributes: Option<String>,
    pub message_attributes: Option<Map<String, MessageAttributeValue>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteMessageInput {
    pub queue_url: String,
    pub receipt_handle: String,
}

impl crate::wire::__Wire for DeleteMessageInput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.queue_url, out);
        crate::wire::__Wire::__enc(&self.receipt_handle, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            queue_url: crate::wire::__Wire::__dec(r)?,
            receipt_handle: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteQueueInput {
    pub queue_url: String,
}

impl crate::wire::__Wire for DeleteQueueInput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.queue_url, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            queue_url: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SqsError {
    pub code: String,
    pub message: String,
    pub status: i32,
    pub request_id: Option<String>,
}

impl crate::wire::__Wire for SqsError {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.code, out);
        crate::wire::__Wire::__enc(&self.message, out);
        crate::wire::__Wire::__enc(&self.status, out);
        crate::wire::__Wire::__enc(&self.request_id, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            code: crate::wire::__Wire::__dec(r)?,
            message: crate::wire::__Wire::__dec(r)?,
            status: crate::wire::__Wire::__dec(r)?,
            request_id: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub type SqsFailure = Union2<SqsError, AwsError>;

/// Factories for the host: one per arm of the union [platform-factory].
impl SqsFailure {
    pub fn sqs_error(value: SqsError) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn aws_error(value: AwsError) -> Self {
        crate::unions::Union2::U2(value)
    }
}

pub trait __Stateless_Sqs: Send + Sync {
    fn create_queue(&self, input: CreateQueueInput, reply: crate::scheduler::SalvoReply);
    fn get_queue_url(&self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply);
    fn send_message(&self, input: SendMessageInput, reply: crate::scheduler::SalvoReply);
    fn receive_message(&self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply);
    fn delete_message(&self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply);
    fn delete_queue(&self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply);
}

pub trait __Stateful_Sqs: Send {
    fn create_queue(&mut self, input: CreateQueueInput, reply: crate::scheduler::SalvoReply);
    fn get_queue_url(&mut self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply);
    fn send_message(&mut self, input: SendMessageInput, reply: crate::scheduler::SalvoReply);
    fn receive_message(&mut self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply);
    fn delete_message(&mut self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply);
    fn delete_queue(&mut self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply);
}

pub struct Sqs {
    inner: __Inner_Sqs,
}

pub enum __Inner_Sqs {
    Shared(std::sync::Arc<dyn __Stateless_Sqs>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Sqs>>),
}

impl Clone for Sqs {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Sqs::Shared(h) => __Inner_Sqs::Shared(h.clone()),
            __Inner_Sqs::Locked(h) => __Inner_Sqs::Locked(h.clone()),
        } }
    }
}

impl Sqs {
    pub fn shared<__H: __Stateless_Sqs + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Sqs::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Sqs>) -> Self {
        Self { inner: __Inner_Sqs::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Sqs + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Sqs::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Sqs>>) -> Self {
        Self { inner: __Inner_Sqs::Locked(inner) }
    }
    pub fn create_queue(&self, input: CreateQueueInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sqs::Shared(h) => h.create_queue(input, reply),
            __Inner_Sqs::Locked(h) => h.lock().unwrap().create_queue(input, reply),
        }
    }
    pub fn get_queue_url(&self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sqs::Shared(h) => h.get_queue_url(input, reply),
            __Inner_Sqs::Locked(h) => h.lock().unwrap().get_queue_url(input, reply),
        }
    }
    pub fn send_message(&self, input: SendMessageInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sqs::Shared(h) => h.send_message(input, reply),
            __Inner_Sqs::Locked(h) => h.lock().unwrap().send_message(input, reply),
        }
    }
    pub fn receive_message(&self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sqs::Shared(h) => h.receive_message(input, reply),
            __Inner_Sqs::Locked(h) => h.lock().unwrap().receive_message(input, reply),
        }
    }
    pub fn delete_message(&self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sqs::Shared(h) => h.delete_message(input, reply),
            __Inner_Sqs::Locked(h) => h.lock().unwrap().delete_message(input, reply),
        }
    }
    pub fn delete_queue(&self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sqs::Shared(h) => h.delete_queue(input, reply),
            __Inner_Sqs::Locked(h) => h.lock().unwrap().delete_queue(input, reply),
        }
    }
}

pub trait __Stateless_SqsCalls: Send + Sync {
    fn calls(&self) -> Vec<String>;
}

pub trait __Stateful_SqsCalls: Send {
    fn calls(&mut self) -> Vec<String>;
}

pub struct SqsCalls {
    inner: __Inner_SqsCalls,
}

pub enum __Inner_SqsCalls {
    Shared(std::sync::Arc<dyn __Stateless_SqsCalls>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_SqsCalls>>),
}

impl Clone for SqsCalls {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_SqsCalls::Shared(h) => __Inner_SqsCalls::Shared(h.clone()),
            __Inner_SqsCalls::Locked(h) => __Inner_SqsCalls::Locked(h.clone()),
        } }
    }
}

impl SqsCalls {
    pub fn shared<__H: __Stateless_SqsCalls + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_SqsCalls::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_SqsCalls>) -> Self {
        Self { inner: __Inner_SqsCalls::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_SqsCalls + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_SqsCalls::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_SqsCalls>>) -> Self {
        Self { inner: __Inner_SqsCalls::Locked(inner) }
    }
    pub fn calls(&self) -> Vec<String> {
        match &self.inner {
            __Inner_SqsCalls::Shared(h) => h.calls(),
            __Inner_SqsCalls::Locked(h) => h.lock().unwrap().calls(),
        }
    }
}

pub struct FakeSqs {
    recorded: Vec<String>,
}

impl FakeSqs {
    pub fn new() -> Self {
        Self {
            recorded: vec![],
        }
    }
}

impl crate::aws_sqs::__Stateful_Sqs for FakeSqs {

    fn create_queue(&mut self, input: CreateQueueInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform(&mut self.recorded, "create_queue".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(CreateQueueOutput { queue_url: None })));
    }

    fn get_queue_url(&mut self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform(&mut self.recorded, "get_queue_url".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(GetQueueUrlOutput { queue_url: None })));
    }

    fn send_message(&mut self, input: SendMessageInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform(&mut self.recorded, "send_message".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(SendMessageOutput { md5_of_message_body: None, md5_of_message_attributes: None, md5_of_message_system_attributes: None, message_id: None, sequence_number: None })));
    }

    fn receive_message(&mut self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform(&mut self.recorded, "receive_message".to_string());
        (reply).send(std::boxed::Box::new(Union2::<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>::U1(ok(ReceiveMessageOutput { messages: None }))));
    }

    fn delete_message(&mut self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform(&mut self.recorded, "delete_message".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<(), Checked<Union2<SqsError, AwsError>>>::U1(ok(())));
    }

    fn delete_queue(&mut self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform(&mut self.recorded, "delete_queue".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union2<SqsError, AwsError>>>>(reply, Union2::<(), Checked<Union2<SqsError, AwsError>>>::U1(ok(())));
    }
}

impl crate::aws_sqs::__Stateful_SqsCalls for FakeSqs {

    fn calls(&mut self) -> Vec<String> {
        return self.recorded.clone();
    }
}
