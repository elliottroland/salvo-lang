import heap

actor effect Counter {
    send fn next_value(reply: Reply<Int>) => !reply
}

handler BasicCounter of Counter {
    mailbox {
        capacity: 16
    }

    count: Int = 0

    send fn next_value(reply: Reply<Int>) => !reply {
        count++
        reply.send(count)
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let counter = spawn BasicCounter() on thread()
    use counter
    
    next_value(replyto print_count())

    let value = waitfor out {
        next_value(out)
        // TODO: The type of `out` should be `Never` here
        // out
        // next_value(out)
    }
    println("the value is: ${value}")


}

fn print_count(int: Int) [Console] {
    println("count: ${int}")
}