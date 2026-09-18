package salvo.core.actor

import salvo.core.string.*

data class Mailbox(
    val capacity: Int,
)

data class Fault(
    val reason: String,
)

interface Faults {
    fun faulted(fault: Fault)
}

class __Stub_Faults(private val addr: Int) : Faults {
    override fun faulted(fault: Fault) {
        salvo.SalvoSched.send(addr, __Msg_Faults.Faulted(fault))
    }
}

sealed class __Msg_Faults {
    class Faulted(val fault: Fault) : __Msg_Faults()
}

sealed class __Cont_Faults {
    class Faulted() : __Cont_Faults()
}

data class Exit(
    val reason: String,
)

data class Idle(
    val parked_gates: Int,
    val parked_tokens: Int,
)
