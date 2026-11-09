import std.heap

actor effect Counter {
    send fn next_value(reply: Reply<Int>) => !reply
}

handler BasicCounter for Counter {
    mailbox {
        capacity: 16
    }
    state {
        count: Int = 0
    }
    fn next_value {
        todo!()
    }
}

fn main() [use, spawn] {
    use StdConsole()
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

    let s = something()
    if s is Str {
        println("It is a string: ${s}")
    }
}

send fn print_count(int: Int) [Console] {
    println("count: ${int}")
}

fn something() -> Str | Int {
    todo!()
}