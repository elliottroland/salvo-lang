package salvo.core.actor

import salvo.core.string.*

data class Mailbox(
    val capacity: Int,
)

data class Exit(
    val reason: String,
)
