package salvo.aws.sqs

import salvo.*
import salvo.aws.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

object QueueAttributeName {

    class All

    class Policy

    class VisibilityTimeout

    class MaximumMessageSize

    class MessageRetentionPeriod

    class ApproximateNumberOfMessages

    class ApproximateNumberOfMessagesNotVisible

    class CreatedTimestamp

    class LastModifiedTimestamp

    class QueueArn

    class ApproximateNumberOfMessagesDelayed

    class DelaySeconds

    class ReceiveMessageWaitTimeSeconds

    class RedrivePolicy

    class FifoQueue

    class ContentBasedDeduplication

    class KmsMasterKeyId

    class KmsDataKeyReusePeriodSeconds

    class DeduplicationScope

    class FifoThroughputLimit

    class RedriveAllowPolicy

    class SqsManagedSseEnabled

    data class Unknown(
        val value: String,
    )
}

object __Codec_QueueAttributeName_All : salvo.WireCodec<QueueAttributeName.All> {
    override fun enc(v: QueueAttributeName.All, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.All = QueueAttributeName.All()
}

object __Codec_QueueAttributeName_Policy : salvo.WireCodec<QueueAttributeName.Policy> {
    override fun enc(v: QueueAttributeName.Policy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.Policy = QueueAttributeName.Policy()
}

object __Codec_QueueAttributeName_VisibilityTimeout : salvo.WireCodec<QueueAttributeName.VisibilityTimeout> {
    override fun enc(v: QueueAttributeName.VisibilityTimeout, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.VisibilityTimeout = QueueAttributeName.VisibilityTimeout()
}

object __Codec_QueueAttributeName_MaximumMessageSize : salvo.WireCodec<QueueAttributeName.MaximumMessageSize> {
    override fun enc(v: QueueAttributeName.MaximumMessageSize, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.MaximumMessageSize = QueueAttributeName.MaximumMessageSize()
}

object __Codec_QueueAttributeName_MessageRetentionPeriod : salvo.WireCodec<QueueAttributeName.MessageRetentionPeriod> {
    override fun enc(v: QueueAttributeName.MessageRetentionPeriod, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.MessageRetentionPeriod = QueueAttributeName.MessageRetentionPeriod()
}

object __Codec_QueueAttributeName_ApproximateNumberOfMessages : salvo.WireCodec<QueueAttributeName.ApproximateNumberOfMessages> {
    override fun enc(v: QueueAttributeName.ApproximateNumberOfMessages, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.ApproximateNumberOfMessages = QueueAttributeName.ApproximateNumberOfMessages()
}

object __Codec_QueueAttributeName_ApproximateNumberOfMessagesNotVisible : salvo.WireCodec<QueueAttributeName.ApproximateNumberOfMessagesNotVisible> {
    override fun enc(v: QueueAttributeName.ApproximateNumberOfMessagesNotVisible, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.ApproximateNumberOfMessagesNotVisible = QueueAttributeName.ApproximateNumberOfMessagesNotVisible()
}

object __Codec_QueueAttributeName_CreatedTimestamp : salvo.WireCodec<QueueAttributeName.CreatedTimestamp> {
    override fun enc(v: QueueAttributeName.CreatedTimestamp, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.CreatedTimestamp = QueueAttributeName.CreatedTimestamp()
}

object __Codec_QueueAttributeName_LastModifiedTimestamp : salvo.WireCodec<QueueAttributeName.LastModifiedTimestamp> {
    override fun enc(v: QueueAttributeName.LastModifiedTimestamp, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.LastModifiedTimestamp = QueueAttributeName.LastModifiedTimestamp()
}

object __Codec_QueueAttributeName_QueueArn : salvo.WireCodec<QueueAttributeName.QueueArn> {
    override fun enc(v: QueueAttributeName.QueueArn, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.QueueArn = QueueAttributeName.QueueArn()
}

object __Codec_QueueAttributeName_ApproximateNumberOfMessagesDelayed : salvo.WireCodec<QueueAttributeName.ApproximateNumberOfMessagesDelayed> {
    override fun enc(v: QueueAttributeName.ApproximateNumberOfMessagesDelayed, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.ApproximateNumberOfMessagesDelayed = QueueAttributeName.ApproximateNumberOfMessagesDelayed()
}

object __Codec_QueueAttributeName_DelaySeconds : salvo.WireCodec<QueueAttributeName.DelaySeconds> {
    override fun enc(v: QueueAttributeName.DelaySeconds, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.DelaySeconds = QueueAttributeName.DelaySeconds()
}

object __Codec_QueueAttributeName_ReceiveMessageWaitTimeSeconds : salvo.WireCodec<QueueAttributeName.ReceiveMessageWaitTimeSeconds> {
    override fun enc(v: QueueAttributeName.ReceiveMessageWaitTimeSeconds, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.ReceiveMessageWaitTimeSeconds = QueueAttributeName.ReceiveMessageWaitTimeSeconds()
}

object __Codec_QueueAttributeName_RedrivePolicy : salvo.WireCodec<QueueAttributeName.RedrivePolicy> {
    override fun enc(v: QueueAttributeName.RedrivePolicy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.RedrivePolicy = QueueAttributeName.RedrivePolicy()
}

object __Codec_QueueAttributeName_FifoQueue : salvo.WireCodec<QueueAttributeName.FifoQueue> {
    override fun enc(v: QueueAttributeName.FifoQueue, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.FifoQueue = QueueAttributeName.FifoQueue()
}

object __Codec_QueueAttributeName_ContentBasedDeduplication : salvo.WireCodec<QueueAttributeName.ContentBasedDeduplication> {
    override fun enc(v: QueueAttributeName.ContentBasedDeduplication, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.ContentBasedDeduplication = QueueAttributeName.ContentBasedDeduplication()
}

object __Codec_QueueAttributeName_KmsMasterKeyId : salvo.WireCodec<QueueAttributeName.KmsMasterKeyId> {
    override fun enc(v: QueueAttributeName.KmsMasterKeyId, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.KmsMasterKeyId = QueueAttributeName.KmsMasterKeyId()
}

object __Codec_QueueAttributeName_KmsDataKeyReusePeriodSeconds : salvo.WireCodec<QueueAttributeName.KmsDataKeyReusePeriodSeconds> {
    override fun enc(v: QueueAttributeName.KmsDataKeyReusePeriodSeconds, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.KmsDataKeyReusePeriodSeconds = QueueAttributeName.KmsDataKeyReusePeriodSeconds()
}

object __Codec_QueueAttributeName_DeduplicationScope : salvo.WireCodec<QueueAttributeName.DeduplicationScope> {
    override fun enc(v: QueueAttributeName.DeduplicationScope, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.DeduplicationScope = QueueAttributeName.DeduplicationScope()
}

object __Codec_QueueAttributeName_FifoThroughputLimit : salvo.WireCodec<QueueAttributeName.FifoThroughputLimit> {
    override fun enc(v: QueueAttributeName.FifoThroughputLimit, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.FifoThroughputLimit = QueueAttributeName.FifoThroughputLimit()
}

object __Codec_QueueAttributeName_RedriveAllowPolicy : salvo.WireCodec<QueueAttributeName.RedriveAllowPolicy> {
    override fun enc(v: QueueAttributeName.RedriveAllowPolicy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.RedriveAllowPolicy = QueueAttributeName.RedriveAllowPolicy()
}

object __Codec_QueueAttributeName_SqsManagedSseEnabled : salvo.WireCodec<QueueAttributeName.SqsManagedSseEnabled> {
    override fun enc(v: QueueAttributeName.SqsManagedSseEnabled, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.SqsManagedSseEnabled = QueueAttributeName.SqsManagedSseEnabled()
}

object __Codec_QueueAttributeName_Unknown : salvo.WireCodec<QueueAttributeName.Unknown> {
    override fun enc(v: QueueAttributeName.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): QueueAttributeName.Unknown = QueueAttributeName.Unknown(salvo.StrCodec.dec(inp))
}

object MessageSystemAttributeName {

    class All

    class SenderId

    class SentTimestamp

    class ApproximateReceiveCount

    class ApproximateFirstReceiveTimestamp

    class SequenceNumber

    class MessageDeduplicationId

    class MessageGroupId

    class AwsTraceHeader

    class DeadLetterQueueSourceArn

    data class Unknown(
        val value: String,
    )
}

object __Codec_MessageSystemAttributeName_All : salvo.WireCodec<MessageSystemAttributeName.All> {
    override fun enc(v: MessageSystemAttributeName.All, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.All = MessageSystemAttributeName.All()
}

object __Codec_MessageSystemAttributeName_SenderId : salvo.WireCodec<MessageSystemAttributeName.SenderId> {
    override fun enc(v: MessageSystemAttributeName.SenderId, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.SenderId = MessageSystemAttributeName.SenderId()
}

object __Codec_MessageSystemAttributeName_SentTimestamp : salvo.WireCodec<MessageSystemAttributeName.SentTimestamp> {
    override fun enc(v: MessageSystemAttributeName.SentTimestamp, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.SentTimestamp = MessageSystemAttributeName.SentTimestamp()
}

object __Codec_MessageSystemAttributeName_ApproximateReceiveCount : salvo.WireCodec<MessageSystemAttributeName.ApproximateReceiveCount> {
    override fun enc(v: MessageSystemAttributeName.ApproximateReceiveCount, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.ApproximateReceiveCount = MessageSystemAttributeName.ApproximateReceiveCount()
}

object __Codec_MessageSystemAttributeName_ApproximateFirstReceiveTimestamp : salvo.WireCodec<MessageSystemAttributeName.ApproximateFirstReceiveTimestamp> {
    override fun enc(v: MessageSystemAttributeName.ApproximateFirstReceiveTimestamp, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.ApproximateFirstReceiveTimestamp = MessageSystemAttributeName.ApproximateFirstReceiveTimestamp()
}

object __Codec_MessageSystemAttributeName_SequenceNumber : salvo.WireCodec<MessageSystemAttributeName.SequenceNumber> {
    override fun enc(v: MessageSystemAttributeName.SequenceNumber, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.SequenceNumber = MessageSystemAttributeName.SequenceNumber()
}

object __Codec_MessageSystemAttributeName_MessageDeduplicationId : salvo.WireCodec<MessageSystemAttributeName.MessageDeduplicationId> {
    override fun enc(v: MessageSystemAttributeName.MessageDeduplicationId, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.MessageDeduplicationId = MessageSystemAttributeName.MessageDeduplicationId()
}

object __Codec_MessageSystemAttributeName_MessageGroupId : salvo.WireCodec<MessageSystemAttributeName.MessageGroupId> {
    override fun enc(v: MessageSystemAttributeName.MessageGroupId, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.MessageGroupId = MessageSystemAttributeName.MessageGroupId()
}

object __Codec_MessageSystemAttributeName_AwsTraceHeader : salvo.WireCodec<MessageSystemAttributeName.AwsTraceHeader> {
    override fun enc(v: MessageSystemAttributeName.AwsTraceHeader, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.AwsTraceHeader = MessageSystemAttributeName.AwsTraceHeader()
}

object __Codec_MessageSystemAttributeName_DeadLetterQueueSourceArn : salvo.WireCodec<MessageSystemAttributeName.DeadLetterQueueSourceArn> {
    override fun enc(v: MessageSystemAttributeName.DeadLetterQueueSourceArn, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.DeadLetterQueueSourceArn = MessageSystemAttributeName.DeadLetterQueueSourceArn()
}

object __Codec_MessageSystemAttributeName_Unknown : salvo.WireCodec<MessageSystemAttributeName.Unknown> {
    override fun enc(v: MessageSystemAttributeName.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeName.Unknown = MessageSystemAttributeName.Unknown(salvo.StrCodec.dec(inp))
}

data class CreateQueueInput(
    val queue_name: String,
    val attributes: Map<String, String>? = null,
    val tags: Map<String, String>? = null,
)

data class CreateQueueOutput(
    val queue_url: String? = null,
)

object __Codec_CreateQueueOutput : salvo.WireCodec<CreateQueueOutput> {
    override fun enc(v: CreateQueueOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.queue_url, out)
    }
    override fun dec(inp: salvo.WireIn): CreateQueueOutput = CreateQueueOutput(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetQueueUrlInput(
    val queue_name: String,
    val queue_owner_aws_account_id: String? = null,
)

object __Codec_GetQueueUrlInput : salvo.WireCodec<GetQueueUrlInput> {
    override fun enc(v: GetQueueUrlInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_name, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.queue_owner_aws_account_id, out)
    }
    override fun dec(inp: salvo.WireIn): GetQueueUrlInput = GetQueueUrlInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetQueueUrlOutput(
    val queue_url: String? = null,
)

object __Codec_GetQueueUrlOutput : salvo.WireCodec<GetQueueUrlOutput> {
    override fun enc(v: GetQueueUrlOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.queue_url, out)
    }
    override fun dec(inp: salvo.WireIn): GetQueueUrlOutput = GetQueueUrlOutput(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class SendMessageInput(
    val queue_url: String,
    val message_body: String,
    val delay_seconds: Int? = null,
    val message_attributes: Map<String, MessageAttributeValue>? = null,
    val message_system_attributes: Map<String, MessageSystemAttributeValue>? = null,
    val message_deduplication_id: String? = null,
    val message_group_id: String? = null,
)

data class MessageAttributeValue(
    val string_value: String? = null,
    val binary_value: salvo.SalvoBytes? = null,
    val string_list_values: List<String>? = null,
    val binary_list_values: List<salvo.SalvoBytes>? = null,
    val data_type: String,
)

object __Codec_MessageAttributeValue : salvo.WireCodec<MessageAttributeValue> {
    override fun enc(v: MessageAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.string_value, out)
        salvo.OptCodec(salvo.BytesCodec).enc(v.binary_value, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.string_list_values, out)
        salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).enc(v.binary_list_values, out)
        salvo.StrCodec.enc(v.data_type, out)
    }
    override fun dec(inp: salvo.WireIn): MessageAttributeValue = MessageAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BytesCodec).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).dec(inp), salvo.StrCodec.dec(inp))
}

data class MessageSystemAttributeValue(
    val string_value: String? = null,
    val binary_value: salvo.SalvoBytes? = null,
    val string_list_values: List<String>? = null,
    val binary_list_values: List<salvo.SalvoBytes>? = null,
    val data_type: String,
)

object __Codec_MessageSystemAttributeValue : salvo.WireCodec<MessageSystemAttributeValue> {
    override fun enc(v: MessageSystemAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.string_value, out)
        salvo.OptCodec(salvo.BytesCodec).enc(v.binary_value, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.string_list_values, out)
        salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).enc(v.binary_list_values, out)
        salvo.StrCodec.enc(v.data_type, out)
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeValue = MessageSystemAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BytesCodec).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).dec(inp), salvo.StrCodec.dec(inp))
}

data class SendMessageOutput(
    val md5_of_message_body: String? = null,
    val md5_of_message_attributes: String? = null,
    val md5_of_message_system_attributes: String? = null,
    val message_id: String? = null,
    val sequence_number: String? = null,
)

object __Codec_SendMessageOutput : salvo.WireCodec<SendMessageOutput> {
    override fun enc(v: SendMessageOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.md5_of_message_body, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.md5_of_message_attributes, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.md5_of_message_system_attributes, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.message_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sequence_number, out)
    }
    override fun dec(inp: salvo.WireIn): SendMessageOutput = SendMessageOutput(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiveMessageInput(
    val queue_url: String,
    val attribute_names: List<Union23<QueueAttributeName.All, QueueAttributeName.Policy, QueueAttributeName.VisibilityTimeout, QueueAttributeName.MaximumMessageSize, QueueAttributeName.MessageRetentionPeriod, QueueAttributeName.ApproximateNumberOfMessages, QueueAttributeName.ApproximateNumberOfMessagesNotVisible, QueueAttributeName.CreatedTimestamp, QueueAttributeName.LastModifiedTimestamp, QueueAttributeName.QueueArn, QueueAttributeName.ApproximateNumberOfMessagesDelayed, QueueAttributeName.DelaySeconds, QueueAttributeName.ReceiveMessageWaitTimeSeconds, QueueAttributeName.RedrivePolicy, QueueAttributeName.FifoQueue, QueueAttributeName.ContentBasedDeduplication, QueueAttributeName.KmsMasterKeyId, QueueAttributeName.KmsDataKeyReusePeriodSeconds, QueueAttributeName.DeduplicationScope, QueueAttributeName.FifoThroughputLimit, QueueAttributeName.RedriveAllowPolicy, QueueAttributeName.SqsManagedSseEnabled, QueueAttributeName.Unknown>>? = null,
    val message_system_attribute_names: List<Union11<MessageSystemAttributeName.All, MessageSystemAttributeName.SenderId, MessageSystemAttributeName.SentTimestamp, MessageSystemAttributeName.ApproximateReceiveCount, MessageSystemAttributeName.ApproximateFirstReceiveTimestamp, MessageSystemAttributeName.SequenceNumber, MessageSystemAttributeName.MessageDeduplicationId, MessageSystemAttributeName.MessageGroupId, MessageSystemAttributeName.AwsTraceHeader, MessageSystemAttributeName.DeadLetterQueueSourceArn, MessageSystemAttributeName.Unknown>>? = null,
    val message_attribute_names: List<String>? = null,
    val max_number_of_messages: Int? = null,
    val visibility_timeout: Int? = null,
    val wait_time_seconds: Int? = null,
    val receive_request_attempt_id: String? = null,
)

object __Codec_ReceiveMessageInput : salvo.WireCodec<ReceiveMessageInput> {
    override fun enc(v: ReceiveMessageInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_url, out)
        salvo.OptCodec(salvo.ListCodec(salvo.Union23Codec(__Codec_QueueAttributeName_All, __Codec_QueueAttributeName_Policy, __Codec_QueueAttributeName_VisibilityTimeout, __Codec_QueueAttributeName_MaximumMessageSize, __Codec_QueueAttributeName_MessageRetentionPeriod, __Codec_QueueAttributeName_ApproximateNumberOfMessages, __Codec_QueueAttributeName_ApproximateNumberOfMessagesNotVisible, __Codec_QueueAttributeName_CreatedTimestamp, __Codec_QueueAttributeName_LastModifiedTimestamp, __Codec_QueueAttributeName_QueueArn, __Codec_QueueAttributeName_ApproximateNumberOfMessagesDelayed, __Codec_QueueAttributeName_DelaySeconds, __Codec_QueueAttributeName_ReceiveMessageWaitTimeSeconds, __Codec_QueueAttributeName_RedrivePolicy, __Codec_QueueAttributeName_FifoQueue, __Codec_QueueAttributeName_ContentBasedDeduplication, __Codec_QueueAttributeName_KmsMasterKeyId, __Codec_QueueAttributeName_KmsDataKeyReusePeriodSeconds, __Codec_QueueAttributeName_DeduplicationScope, __Codec_QueueAttributeName_FifoThroughputLimit, __Codec_QueueAttributeName_RedriveAllowPolicy, __Codec_QueueAttributeName_SqsManagedSseEnabled, __Codec_QueueAttributeName_Unknown))).enc(v.attribute_names, out)
        salvo.OptCodec(salvo.ListCodec(salvo.Union11Codec(__Codec_MessageSystemAttributeName_All, __Codec_MessageSystemAttributeName_SenderId, __Codec_MessageSystemAttributeName_SentTimestamp, __Codec_MessageSystemAttributeName_ApproximateReceiveCount, __Codec_MessageSystemAttributeName_ApproximateFirstReceiveTimestamp, __Codec_MessageSystemAttributeName_SequenceNumber, __Codec_MessageSystemAttributeName_MessageDeduplicationId, __Codec_MessageSystemAttributeName_MessageGroupId, __Codec_MessageSystemAttributeName_AwsTraceHeader, __Codec_MessageSystemAttributeName_DeadLetterQueueSourceArn, __Codec_MessageSystemAttributeName_Unknown))).enc(v.message_system_attribute_names, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.message_attribute_names, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.max_number_of_messages, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.visibility_timeout, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.wait_time_seconds, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.receive_request_attempt_id, out)
    }
    override fun dec(inp: salvo.WireIn): ReceiveMessageInput = ReceiveMessageInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.Union23Codec(__Codec_QueueAttributeName_All, __Codec_QueueAttributeName_Policy, __Codec_QueueAttributeName_VisibilityTimeout, __Codec_QueueAttributeName_MaximumMessageSize, __Codec_QueueAttributeName_MessageRetentionPeriod, __Codec_QueueAttributeName_ApproximateNumberOfMessages, __Codec_QueueAttributeName_ApproximateNumberOfMessagesNotVisible, __Codec_QueueAttributeName_CreatedTimestamp, __Codec_QueueAttributeName_LastModifiedTimestamp, __Codec_QueueAttributeName_QueueArn, __Codec_QueueAttributeName_ApproximateNumberOfMessagesDelayed, __Codec_QueueAttributeName_DelaySeconds, __Codec_QueueAttributeName_ReceiveMessageWaitTimeSeconds, __Codec_QueueAttributeName_RedrivePolicy, __Codec_QueueAttributeName_FifoQueue, __Codec_QueueAttributeName_ContentBasedDeduplication, __Codec_QueueAttributeName_KmsMasterKeyId, __Codec_QueueAttributeName_KmsDataKeyReusePeriodSeconds, __Codec_QueueAttributeName_DeduplicationScope, __Codec_QueueAttributeName_FifoThroughputLimit, __Codec_QueueAttributeName_RedriveAllowPolicy, __Codec_QueueAttributeName_SqsManagedSseEnabled, __Codec_QueueAttributeName_Unknown))).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.Union11Codec(__Codec_MessageSystemAttributeName_All, __Codec_MessageSystemAttributeName_SenderId, __Codec_MessageSystemAttributeName_SentTimestamp, __Codec_MessageSystemAttributeName_ApproximateReceiveCount, __Codec_MessageSystemAttributeName_ApproximateFirstReceiveTimestamp, __Codec_MessageSystemAttributeName_SequenceNumber, __Codec_MessageSystemAttributeName_MessageDeduplicationId, __Codec_MessageSystemAttributeName_MessageGroupId, __Codec_MessageSystemAttributeName_AwsTraceHeader, __Codec_MessageSystemAttributeName_DeadLetterQueueSourceArn, __Codec_MessageSystemAttributeName_Unknown))).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiveMessageOutput(
    val messages: List<Message>? = null,
)

data class Message(
    val message_id: String? = null,
    val receipt_handle: String? = null,
    val md5_of_body: String? = null,
    val body: String? = null,
    val attributes: Map<String, String>? = null,
    val md5_of_message_attributes: String? = null,
    val message_attributes: Map<String, MessageAttributeValue>? = null,
)

data class DeleteMessageInput(
    val queue_url: String,
    val receipt_handle: String,
)

object __Codec_DeleteMessageInput : salvo.WireCodec<DeleteMessageInput> {
    override fun enc(v: DeleteMessageInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_url, out)
        salvo.StrCodec.enc(v.receipt_handle, out)
    }
    override fun dec(inp: salvo.WireIn): DeleteMessageInput = DeleteMessageInput(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class DeleteQueueInput(
    val queue_url: String,
)

object __Codec_DeleteQueueInput : salvo.WireCodec<DeleteQueueInput> {
    override fun enc(v: DeleteQueueInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_url, out)
    }
    override fun dec(inp: salvo.WireIn): DeleteQueueInput = DeleteQueueInput(salvo.StrCodec.dec(inp))
}

data class InvalidAddress(
    val message: String? = null,
)

object __Codec_InvalidAddress : salvo.WireCodec<InvalidAddress> {
    override fun enc(v: InvalidAddress, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidAddress = InvalidAddress(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class InvalidAttributeName(
    val message: String? = null,
)

object __Codec_InvalidAttributeName : salvo.WireCodec<InvalidAttributeName> {
    override fun enc(v: InvalidAttributeName, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidAttributeName = InvalidAttributeName(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class InvalidAttributeValue(
    val message: String? = null,
)

object __Codec_InvalidAttributeValue : salvo.WireCodec<InvalidAttributeValue> {
    override fun enc(v: InvalidAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidAttributeValue = InvalidAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

class InvalidIdFormat

object __Codec_InvalidIdFormat : salvo.WireCodec<InvalidIdFormat> {
    override fun enc(v: InvalidIdFormat, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): InvalidIdFormat = InvalidIdFormat()
}

data class InvalidMessageContents(
    val message: String? = null,
)

object __Codec_InvalidMessageContents : salvo.WireCodec<InvalidMessageContents> {
    override fun enc(v: InvalidMessageContents, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidMessageContents = InvalidMessageContents(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class InvalidSecurity(
    val message: String? = null,
)

object __Codec_InvalidSecurity : salvo.WireCodec<InvalidSecurity> {
    override fun enc(v: InvalidSecurity, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidSecurity = InvalidSecurity(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsAccessDenied(
    val message: String? = null,
)

object __Codec_KmsAccessDenied : salvo.WireCodec<KmsAccessDenied> {
    override fun enc(v: KmsAccessDenied, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsAccessDenied = KmsAccessDenied(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsDisabled(
    val message: String? = null,
)

object __Codec_KmsDisabled : salvo.WireCodec<KmsDisabled> {
    override fun enc(v: KmsDisabled, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsDisabled = KmsDisabled(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsInvalidKeyUsage(
    val message: String? = null,
)

object __Codec_KmsInvalidKeyUsage : salvo.WireCodec<KmsInvalidKeyUsage> {
    override fun enc(v: KmsInvalidKeyUsage, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsInvalidKeyUsage = KmsInvalidKeyUsage(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsInvalidState(
    val message: String? = null,
)

object __Codec_KmsInvalidState : salvo.WireCodec<KmsInvalidState> {
    override fun enc(v: KmsInvalidState, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsInvalidState = KmsInvalidState(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsNotFound(
    val message: String? = null,
)

object __Codec_KmsNotFound : salvo.WireCodec<KmsNotFound> {
    override fun enc(v: KmsNotFound, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsNotFound = KmsNotFound(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsOptInRequired(
    val message: String? = null,
)

object __Codec_KmsOptInRequired : salvo.WireCodec<KmsOptInRequired> {
    override fun enc(v: KmsOptInRequired, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsOptInRequired = KmsOptInRequired(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class KmsThrottled(
    val message: String? = null,
)

object __Codec_KmsThrottled : salvo.WireCodec<KmsThrottled> {
    override fun enc(v: KmsThrottled, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): KmsThrottled = KmsThrottled(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class OverLimit(
    val message: String? = null,
)

object __Codec_OverLimit : salvo.WireCodec<OverLimit> {
    override fun enc(v: OverLimit, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): OverLimit = OverLimit(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class QueueDeletedRecently(
    val message: String? = null,
)

object __Codec_QueueDeletedRecently : salvo.WireCodec<QueueDeletedRecently> {
    override fun enc(v: QueueDeletedRecently, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): QueueDeletedRecently = QueueDeletedRecently(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class QueueDoesNotExist(
    val message: String? = null,
)

object __Codec_QueueDoesNotExist : salvo.WireCodec<QueueDoesNotExist> {
    override fun enc(v: QueueDoesNotExist, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): QueueDoesNotExist = QueueDoesNotExist(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class QueueNameExists(
    val message: String? = null,
)

object __Codec_QueueNameExists : salvo.WireCodec<QueueNameExists> {
    override fun enc(v: QueueNameExists, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): QueueNameExists = QueueNameExists(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiptHandleIsInvalid(
    val message: String? = null,
)

object __Codec_ReceiptHandleIsInvalid : salvo.WireCodec<ReceiptHandleIsInvalid> {
    override fun enc(v: ReceiptHandleIsInvalid, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): ReceiptHandleIsInvalid = ReceiptHandleIsInvalid(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class RequestThrottled(
    val message: String? = null,
)

object __Codec_RequestThrottled : salvo.WireCodec<RequestThrottled> {
    override fun enc(v: RequestThrottled, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): RequestThrottled = RequestThrottled(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class UnsupportedOperation(
    val message: String? = null,
)

object __Codec_UnsupportedOperation : salvo.WireCodec<UnsupportedOperation> {
    override fun enc(v: UnsupportedOperation, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): UnsupportedOperation = UnsupportedOperation(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

interface Sqs {
    fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply)
    fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply)
    fun send_message(input: SendMessageInput, reply: salvo.SalvoReply)
    fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply)
    fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply)
    fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply)
}

class __Mon_Sqs(private val inner: Sqs) : Sqs {
    override fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.create_queue(input, reply) }
    override fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.get_queue_url(input, reply) }
    override fun send_message(input: SendMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.send_message(input, reply) }
    override fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.receive_message(input, reply) }
    override fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.delete_message(input, reply) }
    override fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.delete_queue(input, reply) }
}

interface SqsCalls {
    fun calls(): List<String>
}

class __Mon_SqsCalls(private val inner: SqsCalls) : SqsCalls {
    override fun calls(): List<String> =
        synchronized(inner) { inner.calls() }
}

class FakeSqs : Sqs, SqsCalls {
    private var recorded: MutableList<String> = mutableListOf<String>()

    override fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply) {
        recorded.add("create_queue")
        salvo.SalvoSched.replyWire(reply, U2_1<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(CreateQueueOutput())), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply) {
        recorded.add("get_queue_url")
        salvo.SalvoSched.replyWire(reply, U2_1<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(GetQueueUrlOutput())), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun send_message(input: SendMessageInput, reply: salvo.SalvoReply) {
        recorded.add("send_message")
        salvo.SalvoSched.replyWire(reply, U2_1<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(SendMessageOutput())), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply) {
        recorded.add("receive_message")
        reply.send(U2_1<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(ReceiveMessageOutput())))
    }

    override fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply) {
        recorded.add("delete_message")
        salvo.SalvoSched.replyWire(reply, U2_1<Unit, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply) {
        recorded.add("delete_queue")
        salvo.SalvoSched.replyWire(reply, U2_1<Unit, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun calls(): List<String> {
        return recorded.toMutableList()
    }
}
