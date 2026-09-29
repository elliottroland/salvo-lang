use crate::aws::*;
use crate::collections::*;
use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameAll {
}

impl crate::wire::__Wire for QueueAttributeNameAll {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNamePolicy {
}

impl crate::wire::__Wire for QueueAttributeNamePolicy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameVisibilityTimeout {
}

impl crate::wire::__Wire for QueueAttributeNameVisibilityTimeout {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameMaximumMessageSize {
}

impl crate::wire::__Wire for QueueAttributeNameMaximumMessageSize {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameMessageRetentionPeriod {
}

impl crate::wire::__Wire for QueueAttributeNameMessageRetentionPeriod {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameApproximateNumberOfMessages {
}

impl crate::wire::__Wire for QueueAttributeNameApproximateNumberOfMessages {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameApproximateNumberOfMessagesNotVisible {
}

impl crate::wire::__Wire for QueueAttributeNameApproximateNumberOfMessagesNotVisible {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameCreatedTimestamp {
}

impl crate::wire::__Wire for QueueAttributeNameCreatedTimestamp {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameLastModifiedTimestamp {
}

impl crate::wire::__Wire for QueueAttributeNameLastModifiedTimestamp {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameQueueArn {
}

impl crate::wire::__Wire for QueueAttributeNameQueueArn {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameApproximateNumberOfMessagesDelayed {
}

impl crate::wire::__Wire for QueueAttributeNameApproximateNumberOfMessagesDelayed {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameDelaySeconds {
}

impl crate::wire::__Wire for QueueAttributeNameDelaySeconds {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameReceiveMessageWaitTimeSeconds {
}

impl crate::wire::__Wire for QueueAttributeNameReceiveMessageWaitTimeSeconds {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameRedrivePolicy {
}

impl crate::wire::__Wire for QueueAttributeNameRedrivePolicy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameFifoQueue {
}

impl crate::wire::__Wire for QueueAttributeNameFifoQueue {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameContentBasedDeduplication {
}

impl crate::wire::__Wire for QueueAttributeNameContentBasedDeduplication {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameKmsMasterKeyId {
}

impl crate::wire::__Wire for QueueAttributeNameKmsMasterKeyId {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameKmsDataKeyReusePeriodSeconds {
}

impl crate::wire::__Wire for QueueAttributeNameKmsDataKeyReusePeriodSeconds {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameDeduplicationScope {
}

impl crate::wire::__Wire for QueueAttributeNameDeduplicationScope {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameFifoThroughputLimit {
}

impl crate::wire::__Wire for QueueAttributeNameFifoThroughputLimit {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameRedriveAllowPolicy {
}

impl crate::wire::__Wire for QueueAttributeNameRedriveAllowPolicy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameSqsManagedSseEnabled {
}

impl crate::wire::__Wire for QueueAttributeNameSqsManagedSseEnabled {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueAttributeNameUnknown {
    pub value: String,
}

impl crate::wire::__Wire for QueueAttributeNameUnknown {
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
pub struct MessageSystemAttributeNameAll {
}

impl crate::wire::__Wire for MessageSystemAttributeNameAll {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameSenderId {
}

impl crate::wire::__Wire for MessageSystemAttributeNameSenderId {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameSentTimestamp {
}

impl crate::wire::__Wire for MessageSystemAttributeNameSentTimestamp {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameApproximateReceiveCount {
}

impl crate::wire::__Wire for MessageSystemAttributeNameApproximateReceiveCount {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameApproximateFirstReceiveTimestamp {
}

impl crate::wire::__Wire for MessageSystemAttributeNameApproximateFirstReceiveTimestamp {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameSequenceNumber {
}

impl crate::wire::__Wire for MessageSystemAttributeNameSequenceNumber {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameMessageDeduplicationId {
}

impl crate::wire::__Wire for MessageSystemAttributeNameMessageDeduplicationId {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameMessageGroupId {
}

impl crate::wire::__Wire for MessageSystemAttributeNameMessageGroupId {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameAwsTraceHeader {
}

impl crate::wire::__Wire for MessageSystemAttributeNameAwsTraceHeader {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameDeadLetterQueueSourceArn {
}

impl crate::wire::__Wire for MessageSystemAttributeNameDeadLetterQueueSourceArn {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSystemAttributeNameUnknown {
    pub value: String,
}

impl crate::wire::__Wire for MessageSystemAttributeNameUnknown {
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
pub struct CreateQueueInput {
    pub queue_name: String,
    pub attributes: Option<SalvoMap<String, String>>,
    pub tags: Option<SalvoMap<String, String>>,
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
    pub message_attributes: Option<SalvoMap<String, MessageAttributeValue>>,
    pub message_system_attributes: Option<SalvoMap<String, MessageSystemAttributeValue>>,
    pub message_deduplication_id: Option<String>,
    pub message_group_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageAttributeValue {
    pub string_value: Option<String>,
    pub binary_value: Option<Vec<u8>>,
    pub string_list_values: Option<Vec<String>>,
    pub binary_list_values: Option<Vec<Vec<u8>>>,
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
    pub binary_value: Option<Vec<u8>>,
    pub string_list_values: Option<Vec<String>>,
    pub binary_list_values: Option<Vec<Vec<u8>>>,
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
    pub attribute_names: Option<Vec<Union23<QueueAttributeNameAll, QueueAttributeNamePolicy, QueueAttributeNameVisibilityTimeout, QueueAttributeNameMaximumMessageSize, QueueAttributeNameMessageRetentionPeriod, QueueAttributeNameApproximateNumberOfMessages, QueueAttributeNameApproximateNumberOfMessagesNotVisible, QueueAttributeNameCreatedTimestamp, QueueAttributeNameLastModifiedTimestamp, QueueAttributeNameQueueArn, QueueAttributeNameApproximateNumberOfMessagesDelayed, QueueAttributeNameDelaySeconds, QueueAttributeNameReceiveMessageWaitTimeSeconds, QueueAttributeNameRedrivePolicy, QueueAttributeNameFifoQueue, QueueAttributeNameContentBasedDeduplication, QueueAttributeNameKmsMasterKeyId, QueueAttributeNameKmsDataKeyReusePeriodSeconds, QueueAttributeNameDeduplicationScope, QueueAttributeNameFifoThroughputLimit, QueueAttributeNameRedriveAllowPolicy, QueueAttributeNameSqsManagedSseEnabled, QueueAttributeNameUnknown>>>,
    pub message_system_attribute_names: Option<Vec<Union11<MessageSystemAttributeNameAll, MessageSystemAttributeNameSenderId, MessageSystemAttributeNameSentTimestamp, MessageSystemAttributeNameApproximateReceiveCount, MessageSystemAttributeNameApproximateFirstReceiveTimestamp, MessageSystemAttributeNameSequenceNumber, MessageSystemAttributeNameMessageDeduplicationId, MessageSystemAttributeNameMessageGroupId, MessageSystemAttributeNameAwsTraceHeader, MessageSystemAttributeNameDeadLetterQueueSourceArn, MessageSystemAttributeNameUnknown>>>,
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
    pub attributes: Option<SalvoMap<String, String>>,
    pub md5_of_message_attributes: Option<String>,
    pub message_attributes: Option<SalvoMap<String, MessageAttributeValue>>,
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
pub struct InvalidAddress {
    pub message: Option<String>,
}

impl crate::wire::__Wire for InvalidAddress {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidAttributeName {
    pub message: Option<String>,
}

impl crate::wire::__Wire for InvalidAttributeName {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidAttributeValue {
    pub message: Option<String>,
}

impl crate::wire::__Wire for InvalidAttributeValue {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidIdFormat {
}

impl crate::wire::__Wire for InvalidIdFormat {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidMessageContents {
    pub message: Option<String>,
}

impl crate::wire::__Wire for InvalidMessageContents {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidSecurity {
    pub message: Option<String>,
}

impl crate::wire::__Wire for InvalidSecurity {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsAccessDenied {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsAccessDenied {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsDisabled {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsDisabled {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsInvalidKeyUsage {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsInvalidKeyUsage {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsInvalidState {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsInvalidState {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsNotFound {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsNotFound {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsOptInRequired {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsOptInRequired {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KmsThrottled {
    pub message: Option<String>,
}

impl crate::wire::__Wire for KmsThrottled {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverLimit {
    pub message: Option<String>,
}

impl crate::wire::__Wire for OverLimit {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueDeletedRecently {
    pub message: Option<String>,
}

impl crate::wire::__Wire for QueueDeletedRecently {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueDoesNotExist {
    pub message: Option<String>,
}

impl crate::wire::__Wire for QueueDoesNotExist {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueNameExists {
    pub message: Option<String>,
}

impl crate::wire::__Wire for QueueNameExists {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReceiptHandleIsInvalid {
    pub message: Option<String>,
}

impl crate::wire::__Wire for ReceiptHandleIsInvalid {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestThrottled {
    pub message: Option<String>,
}

impl crate::wire::__Wire for RequestThrottled {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnsupportedOperation {
    pub message: Option<String>,
}

impl crate::wire::__Wire for UnsupportedOperation {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            message: crate::wire::__Wire::__dec(r)?,
        })
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
        self.recorded.push("create_queue".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(CreateQueueOutput { queue_url: None })));
    }

    fn get_queue_url(&mut self, input: GetQueueUrlInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("get_queue_url".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(GetQueueUrlOutput { queue_url: None })));
    }

    fn send_message(&mut self, input: SendMessageInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("send_message".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(SendMessageOutput { md5_of_message_body: None, md5_of_message_attributes: None, md5_of_message_system_attributes: None, message_id: None, sequence_number: None })));
    }

    fn receive_message(&mut self, input: ReceiveMessageInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("receive_message".to_string());
        (reply).send(Box::new(Union2::<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(ReceiveMessageOutput { messages: None }))));
    }

    fn delete_message(&mut self, input: DeleteMessageInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("delete_message".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(())));
    }

    fn delete_queue(&mut self, input: DeleteQueueInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("delete_queue".to_string());
        crate::scheduler::salvo_reply_wire::<Union2<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>>(reply, Union2::<(), Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>::U1(ok(())));
    }
}

impl crate::aws_sqs::__Stateful_SqsCalls for FakeSqs {

    fn calls(&mut self) -> Vec<String> {
        return self.recorded.clone();
    }
}
