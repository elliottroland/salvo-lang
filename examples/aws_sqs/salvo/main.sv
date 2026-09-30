// Amazon SQS from Salvo, with no AWS anywhere: the program is written against
// the generated `Sqs` effect, and two handlers that need no SDK stand in for
// the service.
//
// `aws.sqs` is generated from the service's Smithy model
// (`modules/aws/codegen`). Every operation takes a `Reply` and returns at once
// — the host implementation, `aws.sqs.host`'s `HostSqs`, answers from the
// SDK's own async machinery — so a caller either parks a continuation
// (`replyto`) or waits for the answer (`waitfor`), as it would for any actor.
//
// Nothing here imports `aws.sqs.host`, so the build reaches no host glue and
// needs neither SDK: a dependency's host libraries join a build only when its
// platform code is reached.

import aws
import aws.sqs

// What a failure says, in a line.
fn describe(e: SqsFailure) [] -> Str => e {
    if e is AwsError {
        return "${e.code}: ${e.message}"
    }
    // The service's own error: its code is a literal from the model, or
    // `Other` for one the model does not name.
    if e.code is "QueueDoesNotExist" {
        return "no such queue"
    }
    return "the service refused: ${e.code}"
}

// A small workflow against whatever `Sqs` is bound: make a queue, send to it,
// read it back, clean up. It declares `[Sqs]` and nothing else.
fn round_trip(name: Str) [Sqs, Console] => name {
    let created = waitfor r: Reply<Ok CreateQueueOutput | Err Checked<SqsFailure>> {
        create_queue(CreateQueueInput { queue_name: copy(name) }, r)
    }
    if created is Err {
        println("create_queue: ${describe(detach(created))}")
        return None
    }
    let url = created.queue_url ?: "?"
    println("created ${url}")

    let sent = waitfor r: Reply<Ok SendMessageOutput | Err Checked<SqsFailure>> {
        send_message(SendMessageInput { queue_url: copy(url), message_body: "hello from Salvo" }, r)
    }
    when sent {
        is Ok { println("sent ${sent.message_id ?: "?"}") }
        is Err { println("send_message: ${describe(detach(sent))}") }
    }

    let got = waitfor r: Reply<Ok ReceiveMessageOutput | Err Checked<SqsFailure>> {
        receive_message(ReceiveMessageInput { queue_url: copy(url), max_number_of_messages: 10 }, r)
    }
    when got {
        is Ok {
            let messages: List<Message> = got.messages ?: list_of<Message>()
            println("received ${size(messages)} message(s)")
            for m in messages {
                println("  ${m.body ?: ""}")
            }
        }
        is Err { println("receive_message: ${describe(detach(got))}") }
    }

    let gone = waitfor r: Reply<Ok None | Err Checked<SqsFailure>> {
        delete_queue(DeleteQueueInput { queue_url: copy(url) }, r)
    }
    when gone {
        is Ok { println("deleted") }
        is Err { println("delete_queue: ${describe(detach(gone))}") }
    }
}

// The error the service answers for a queue it does not have.
fn no_queue(url: Str) [] -> SqsError => !url {
    return SqsError { code: "QueueDoesNotExist", message: "no queue at ${url}", status: 400 }
}

// A queue service in memory, written the way a test would write one: the
// generated effect is the contract, so any handler of it will do. Queues are
// keyed by URL; a received message stays until deleted, as in SQS.
handler MemSqs() of Sqs {
    queues: Mut Map<Str, List<Str>> = mut_map_of()

    fn create_queue(input: CreateQueueInput, reply: Reply<Ok CreateQueueOutput | Err Checked<SqsFailure>>) -> None
    => !input, !reply {
        let url = "mem://${input.queue_name}"
        if !contains_key(queues, url) {
            put(queues, copy(url), list_of<Str>())
        }
        reply.send(ok(CreateQueueOutput { queue_url: url }))
    }

    fn get_queue_url(input: GetQueueUrlInput, reply: Reply<Ok GetQueueUrlOutput | Err Checked<SqsFailure>>) -> None
    => !input, !reply {
        let url = "mem://${input.queue_name}"
        if !contains_key(queues, url) {
            reply.send(err(checked<SqsFailure>(no_queue(url))))
            return None
        }
        reply.send(ok(GetQueueUrlOutput { queue_url: url }))
    }

    fn send_message(input: SendMessageInput, reply: Reply<Ok SendMessageOutput | Err Checked<SqsFailure>>) -> None
    => !input, !reply {
        let held = get(queues, input.queue_url)
        if held is None {
            reply.send(err(checked<SqsFailure>(no_queue(copy(input.queue_url)))))
            return None
        }
        let grown = mut_list_of<Str>()
        for b in held {
            grown.add(copy(b))
        }
        let id = "m${size(grown) + 1}"
        grown.add(copy(input.message_body))
        let stored: List<Str> = grown
        put(queues, copy(input.queue_url), stored)
        reply.send(ok(SendMessageOutput { message_id: id }))
    }

    fn receive_message(input: ReceiveMessageInput, reply: Reply<Ok ReceiveMessageOutput | Err Checked<SqsFailure>>) -> None
    => !input, !reply {
        let held = get(queues, input.queue_url)
        if held is None {
            reply.send(err(checked<SqsFailure>(no_queue(copy(input.queue_url)))))
            return None
        }
        let out = mut_list_of<Message>()
        let i = 0
        for b in held {
            i = i + 1
            out.add(Message { message_id: "m${i}", receipt_handle: "m${i}", body: copy(b) })
        }
        let messages: List<Message> = out
        reply.send(ok(ReceiveMessageOutput { messages: messages }))
    }

    fn delete_message(input: DeleteMessageInput, reply: Reply<Ok None | Err Checked<SqsFailure>>) -> None
    => !input, !reply {
        reply.send(ok(None))
    }

    fn delete_queue(input: DeleteQueueInput, reply: Reply<Ok None | Err Checked<SqsFailure>>) -> None
    => !input, !reply {
        if !contains_key(queues, input.queue_url) {
            reply.send(err(checked<SqsFailure>(no_queue(copy(input.queue_url)))))
            return None
        }
        remove(queues, input.queue_url)
        reply.send(ok(None))
    }
}

fn main() [use] {
    use StdOutConsole()

    // 1. The generated recording fake: every call answered as an empty success
    //    where the output allows one, and noted by name.
    println("-- FakeSqs --")
    if true {
        use FakeSqs()
        round_trip("orders")
        println("calls: ${calls()}")
    }

    // 2. A double with real queue semantics, for a test that needs answers.
    println("-- MemSqs --")
    if true {
        use MemSqs()
        round_trip("orders")
        // A modeled error arrives as `SqsError` with the model's code.
        let missing = waitfor r: Reply<Ok GetQueueUrlOutput | Err Checked<SqsFailure>> {
            get_queue_url(GetQueueUrlInput { queue_name: "nowhere" }, r)
        }
        when missing {
            is Ok { println("unexpected: ${missing.queue_url ?: "?"}") }
            is Err { println("get_queue_url: ${describe(detach(missing))}") }
        }
    }
}
