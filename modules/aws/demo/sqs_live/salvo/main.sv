// Amazon SQS for real: creates a queue, sends a message, receives it, deletes
// it and the queue — through `aws.sqs.host`'s `HostSqs`, the platform's SDK.
//
// Not part of the test suite: it needs the SDKs and a service to talk to. It
// points at `http://localhost:4566` — `local_sqs.py` beside it, or LocalStack;
// see ../../README.md.

import aws
import aws.sqs
import aws.sqs.host

fn say(what: Str, e: SqsError) [Console] -> None => what, e {
    if e is AwsError {
        println("${what} failed: ${e.code}: ${e.message}")
        return None
    }
    if e is QueueDoesNotExist {
        println("${what} failed: no such queue")
        return None
    }
    println("${what} failed: the service said no")
}

fn main() [use] {
    use StdOutConsole()
    use HostSqs(AwsConfig {
        credentials: ProfileCredentials {},
        region: Region { code: "eu-west-1" },
        endpoint: "http://localhost:4566"
    })
    let created = waitfor r: Reply<Ok CreateQueueOutput | Err Checked<SqsError>> {
        create_queue(CreateQueueInput { queue_name: "salvo-demo" }, r)
    }
    when created {
        is Err { say("create_queue", detach(created)) }
        is Ok {
            let url = created.queue_url ?: "?"
            println("created ${url}")
            let sent = waitfor r: Reply<Ok SendMessageOutput | Err Checked<SqsError>> {
                send_message(SendMessageInput { queue_url: copy(url), message_body: "hello from Salvo" }, r)
            }
            when sent {
                is Ok { println("sent ${sent.message_id ?: "?"}") }
                is Err { say("send_message", detach(sent)) }
            }
            let got = waitfor r: Reply<Ok ReceiveMessageOutput | Err Checked<SqsError>> {
                receive_message(ReceiveMessageInput { queue_url: copy(url), wait_time_seconds: 1 }, r)
            }
            when got {
                is Ok {
                    for m in got.messages ?: list_of<Message>() {
                        println("received: ${m.body ?: ""}")
                    }
                }
                is Err { say("receive_message", detach(got)) }
            }
            let gone = waitfor r: Reply<Ok None | Err Checked<SqsError>> {
                delete_queue(DeleteQueueInput { queue_url: copy(url) }, r)
            }
            when gone {
                is Ok { println("deleted the queue") }
                is Err { say("delete_queue", detach(gone)) }
            }
            // A modeled error arrives as its own arm of `SqsError`.
            let missing = waitfor r: Reply<Ok GetQueueUrlOutput | Err Checked<SqsError>> {
                get_queue_url(GetQueueUrlInput { queue_name: "salvo-demo" }, r)
            }
            when missing {
                is Ok { println("unexpected: ${missing.queue_url ?: "?"}") }
                is Err { say("get_queue_url", detach(missing)) }
            }
        }
    }
}
