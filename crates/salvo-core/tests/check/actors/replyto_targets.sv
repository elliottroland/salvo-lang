// What a `replyto` may mint toward [actor-replyto].

//--- the_target_must_be_a_send_member
// An answer arrives as a message, so the target must be a `send fn`. Once,
// with nothing about its captures: they are moot until the target is one
// (found 2026-10-28, when a capture count "wants 0 capture(s), found 0"
// came with it).
actor effect Ask {
    send fn go(out: Reply<Int>) => !out
    fn ready() -> Bool
}

effect Count {
    fn total(out: Reply<Int>) -> None => !out
}

handler Asker() [Count] of Ask {
    mailbox { capacity: 1 }

    send fn go(out: Reply<Int>) {
        out.send(1)
    }

    fn ready() -> Bool {
        //~ error[actor-replyto:not-send]
        total(replyto ready())
        return true
    }
}

//--- the_target_needs_a_parameter_for_the_answer
actor effect Ask {
    send fn go(out: Reply<Int>) => !out
    send fn arrived()
}

effect Count {
    fn total(out: Reply<Int>) -> None => !out
}

handler Asker() [Count] of Ask {
    mailbox { capacity: 1 }

    send fn go(out: Reply<Int>) {
        //~ error[actor-replyto:no-answer-param]
        total(replyto arrived())
        out.send(1)
    }

    send fn arrived() {
    }
}
