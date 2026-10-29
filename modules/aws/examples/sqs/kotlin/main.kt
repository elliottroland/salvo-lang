package salvo.main

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(e: Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>): String {
    if ((e is Union2.U2<*, *>)) {
        val e_1: salvo.aws.AwsError = ((e as Union2.U2<*, *>).value as salvo.aws.AwsError)
        return "${e_1.code}: ${e_1.message}"
    }
    val e_2: salvo.aws.sqs.SqsError = ((e as Union2.U1<*, *>).value as salvo.aws.sqs.SqsError)
    if ((e_2.code == "QueueDoesNotExist")) {
        return "no such queue"
    }
    val e_3: salvo.aws.sqs.SqsError = ((e as Union2.U1<*, *>).value as salvo.aws.sqs.SqsError)
    return "the service refused: ${e_3.code}"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun roundTrip(sqs: salvo.aws.sqs.Sqs, console: salvo.std.core.console.Console, name: String) {
    val created: Union2<salvo.aws.sqs.CreateQueueOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.sqs.__Codec_CreateQueueOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
        sqs.createQueue(salvo.aws.sqs.CreateQueueInput(queueName = name, attributes = null, tags = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.CreateQueueOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    if ((created is Union2.U2<*, *>)) {
        val created_1: salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((created as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
        salvo.std.core.console.println(console, "create_queue: ${salvo.main.describe(salvo.std.core.checked.detach(created_1))}")
        null
        return
    }
    val created_2: salvo.aws.sqs.CreateQueueOutput = ((created as Union2.U1<*, *>).value as salvo.aws.sqs.CreateQueueOutput)
    val url: String = kotlin.run {
        val __pick_3: String? = created_2.queueUrl
        (if ((__pick_3 != null)) {
            val __picked_4: String = __pick_3!!
            __picked_4
        } else {
            "?"
        })
    }
    salvo.std.core.console.println(console, "created ${url}")
    val sent: Union2<salvo.aws.sqs.SendMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.sqs.__Codec_SendMessageOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
        sqs.sendMessage(salvo.aws.sqs.SendMessageInput(queueUrl = url, messageBody = "hello from Salvo", delaySeconds = null, messageAttributes = null, messageSystemAttributes = null, messageDeduplicationId = null, messageGroupId = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.SendMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    when {
        (sent is Union2.U1<*, *>) -> {
            val sent_6: salvo.aws.sqs.SendMessageOutput = ((sent as Union2.U1<*, *>).value as salvo.aws.sqs.SendMessageOutput)
            salvo.std.core.console.println(console, "sent ${kotlin.run {
                val __pick_7: String? = sent_6.messageId
                (if ((__pick_7 != null)) {
                    val __picked_8: String = __pick_7!!
                    __picked_8
                } else {
                    "?"
                })
            }}")
        }
        (sent is Union2.U2<*, *>) -> {
            val sent_10: salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((sent as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "send_message: ${salvo.main.describe(salvo.std.core.checked.detach(sent_10))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:47:5"))
        }
    }
    val got: Union2<salvo.aws.sqs.ReceiveMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
        sqs.receiveMessage(salvo.aws.sqs.ReceiveMessageInput(queueUrl = url, attributeNames = null, messageSystemAttributeNames = null, messageAttributeNames = null, maxNumberOfMessages = 10, visibilityTimeout = null, waitTimeSeconds = null, receiveRequestAttemptId = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.ReceiveMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    when {
        (got is Union2.U1<*, *>) -> {
            val got_11: salvo.aws.sqs.ReceiveMessageOutput = ((got as Union2.U1<*, *>).value as salvo.aws.sqs.ReceiveMessageOutput)
            val messages: List<salvo.aws.sqs.Message> = kotlin.run {
                val __pick_12: List<salvo.aws.sqs.Message>? = got_11.messages
                (if ((__pick_12 != null)) {
                    val __picked_13: List<salvo.aws.sqs.Message> = __pick_12!!
                    __picked_13
                } else {
                    listOf<salvo.aws.sqs.Message>()
                })
            }
            salvo.std.core.console.println(console, "received ${salvo.std.core.list.sizePlatform(messages)} message(s)")
            for (m in salvo.platform.std.core.list.each(messages)) {
                salvo.std.core.console.println(console, "  ${kotlin.run {
                    val __pick_15: String? = m.body
                    (if ((__pick_15 != null)) {
                        val __picked_16: String = __pick_15!!
                        __picked_16
                    } else {
                        ""
                    })
                }}")
            }
        }
        (got is Union2.U2<*, *>) -> {
            val got_18: salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "receive_message: ${salvo.main.describe(salvo.std.core.checked.detach(got_18))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:55:5"))
        }
    }
    val gone: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.UnitCodec, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
        sqs.deleteQueue(salvo.aws.sqs.DeleteQueueInput(queueUrl = url), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    when {
        (gone is Union2.U1<*, *>) -> {
            val gone_19: Unit = ((gone as Union2.U1<*, *>).value as Unit)
            salvo.std.core.console.println(console, "deleted")
        }
        (gone is Union2.U2<*, *>) -> {
            val gone_20: salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((gone as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "delete_queue: ${salvo.main.describe(salvo.std.core.checked.detach(gone_20))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:69:5"))
        }
    }
}

fun noQueue(url: String): salvo.aws.sqs.SqsError {
    return salvo.aws.sqs.SqsError(code = "QueueDoesNotExist", message = "no queue at ${url}", status = 400, requestId = null)
}

class MemSqs : salvo.aws.sqs.Sqs {
    var queues: salvo.platform.std.core.map.MutMap<String, List<String>> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<String, List<String>>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    override fun createQueue(input: salvo.aws.sqs.CreateQueueInput, reply: salvo.SalvoReply) {
        val url: String = "mem://${input.queueName}"
        if (!(salvo.std.core.map.containsKeyPlatform(queues, url, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.std.core.map.putPlatform(queues, url, listOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.sqs.CreateQueueOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.sqs.CreateQueueOutput(queueUrl = url))), salvo.Union2Codec(salvo.aws.sqs.__Codec_CreateQueueOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun getQueueUrl(input: salvo.aws.sqs.GetQueueUrlInput, reply: salvo.SalvoReply) {
        val url: String = "mem://${input.queueName}"
        if (!(salvo.std.core.map.containsKeyPlatform(queues, url, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<salvo.aws.sqs.GetQueueUrlOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(salvo.main.noQueue(url))))), salvo.Union2Codec(salvo.aws.sqs.__Codec_GetQueueUrlOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.sqs.GetQueueUrlOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.sqs.GetQueueUrlOutput(queueUrl = url))), salvo.Union2Codec(salvo.aws.sqs.__Codec_GetQueueUrlOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun sendMessage(input: salvo.aws.sqs.SendMessageInput, reply: salvo.SalvoReply) {
        val held: List<String>? = salvo.std.core.map.getPlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((held == null)) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<salvo.aws.sqs.SendMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(salvo.main.noQueue(input.queueUrl))))), salvo.Union2Codec(salvo.aws.sqs.__Codec_SendMessageOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        val grown: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
        val held_1: List<String> = held!!
        for (b in salvo.platform.std.core.list.each(held_1)) {
            salvo.std.core.list.addPlatform(grown, b)
        }
        val id: String = "m${(salvo.std.core.list.sizePlatform(grown) + 1)}"
        salvo.std.core.list.addPlatform(grown, input.messageBody)
        val stored: List<String> = grown
        salvo.std.core.map.putPlatform(queues, input.queueUrl, stored, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.sqs.SendMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.sqs.SendMessageOutput(md5OfMessageBody = null, md5OfMessageAttributes = null, md5OfMessageSystemAttributes = null, messageId = id, sequenceNumber = null))), salvo.Union2Codec(salvo.aws.sqs.__Codec_SendMessageOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun receiveMessage(input: salvo.aws.sqs.ReceiveMessageInput, reply: salvo.SalvoReply) {
        val held: List<String>? = salvo.std.core.map.getPlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((held == null)) {
            reply.send(Union2.U2<salvo.aws.sqs.ReceiveMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(salvo.main.noQueue(input.queueUrl))))))
            null
            return
        }
        val out: salvo.platform.std.core.list.MutList<salvo.aws.sqs.Message> = mutableListOf<salvo.aws.sqs.Message>()
        var i: Int = 0
        val held_1: List<String> = held!!
        for (b in salvo.platform.std.core.list.each(held_1)) {
            i = (i + 1)
            salvo.std.core.list.addPlatform(out, salvo.aws.sqs.Message(messageId = "m${i}", receiptHandle = "m${i}", md5OfBody = null, body = b, attributes = null, md5OfMessageAttributes = null, messageAttributes = null))
        }
        val messages: List<salvo.aws.sqs.Message> = out
        reply.send(Union2.U1<salvo.aws.sqs.ReceiveMessageOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.sqs.ReceiveMessageOutput(messages = messages))))
    }
    override fun deleteMessage(input: salvo.aws.sqs.DeleteMessageInput, reply: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun deleteQueue(input: salvo.aws.sqs.DeleteQueueInput, reply: salvo.SalvoReply) {
        if (!(salvo.std.core.map.containsKeyPlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<Unit, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(salvo.main.noQueue(input.queueUrl))))), salvo.Union2Codec(salvo.UnitCodec, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        salvo.std.core.map.removePlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(kotlin.collections.listOf(kotlin.Pair("Faults", salvo.std.core.actor.__PROTO_Faults)))
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    salvo.std.core.console.println(__handle_2, "-- FakeSqs --")
    kotlin.run {
        val __use_3: salvo.aws.sqs.FakeSqs = salvo.aws.sqs.FakeSqs()
        val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
        val __handle_4: salvo.aws.sqs.Sqs = salvo.aws.sqs.__Mon_Sqs(__use_3, __lock___use_3)
        val __handle_5: salvo.aws.sqs.SqsCalls = salvo.aws.sqs.__Mon_SqsCalls(__use_3, __lock___use_3)
        salvo.main.roundTrip(__handle_4, __handle_2, "orders")
        salvo.std.core.console.println(__handle_2, "calls: ${salvo.std.core.list.toStr(__handle_5.calls(), { __a0 -> __a0 })}")
    }
    salvo.std.core.console.println(__handle_2, "-- MemSqs --")
    kotlin.run {
        val __use_6: MemSqs = MemSqs()
        val __lock___use_6 = java.util.concurrent.locks.ReentrantLock()
        val __handle_7: salvo.aws.sqs.Sqs = salvo.aws.sqs.__Mon_Sqs(__use_6, __lock___use_6)
        salvo.main.roundTrip(__handle_7, __handle_2, "orders")
        val missing: Union2<salvo.aws.sqs.GetQueueUrlOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = kotlin.run {
            val (r, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.sqs.__Codec_GetQueueUrlOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
            __handle_7.getQueueUrl(salvo.aws.sqs.GetQueueUrlInput(queueName = "nowhere", queueOwnerAwsAccountId = null), r)
            salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.GetQueueUrlOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
        }
        when {
            (missing is Union2.U1<*, *>) -> {
                val missing_8: salvo.aws.sqs.GetQueueUrlOutput = ((missing as Union2.U1<*, *>).value as salvo.aws.sqs.GetQueueUrlOutput)
                salvo.std.core.console.println(__handle_2, "unexpected: ${kotlin.run {
                    val __pick_9: String? = missing_8.queueUrl
                    (if ((__pick_9 != null)) {
                        val __picked_10: String = __pick_9!!
                        __picked_10
                    } else {
                        "?"
                    })
                }}")
            }
            (missing is Union2.U2<*, *>) -> {
                val missing_12: salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((missing as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
                salvo.std.core.console.println(__handle_2, "get_queue_url: ${salvo.main.describe(salvo.std.core.checked.detach(missing_12))}")
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:179:9"))
            }
        }
    }
}

