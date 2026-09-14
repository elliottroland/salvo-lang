package salvo.main

import salvo.*
import salvo.core.console.*
import salvo.core.string.*

interface Clock {
    fun now(): Int
}

class TickingClock : Clock {
    private var tick: Int = 0

    override fun now(): Int {
        tick = tick + 5
        return tick
    }
}

fun<__Fx> stamp(__fx: __Fx, label: String) where __Fx : __Has_Clock, __Fx : __Has_Console {
    val t = __fx.__fx_Clock.now()
    banner(__fx, "$label at t=$t")
}

fun<__Fx> banner(__fx: __Fx, text: String) where __Fx : __Has_Console {
    println(__fx, "   $text")
}

interface Logger {
    fun log(message: String)
}

class PlainLogger(private val __fx: __Fx_1) : Logger {

    override fun log(message: String) {
        println(__fx, "   $message")
    }
}

class QuietLogger : Logger {

    override fun log(message: String) {
    }
}

class Stamped(private val __fx: __Fx_2) : Logger {

    override fun log(message: String) {
        __fx.__fx_Logger.log("[t=${__fx.__fx_Clock.now()}] $message")
    }
}

class Numbered(private val __fx: __Fx_3) : Logger {
    private var seen: Int = 0

    override fun log(message: String) {
        seen = seen + 1
        __fx.__fx_Logger.log("#$seen $message")
    }
}

fun<__Fx> work(__fx: __Fx, step: String) where __Fx : __Has_Logger {
    __fx.__fx_Logger.log(step)
}

fun<__Fx> interception(__fx: __Fx) where __Fx : __Has_Logger, __Fx : __Has_Clock {
    work(__fx, "4. plain")
    val __fx2 = __Fx_2(__fx.__fx_Clock, Stamped(__Fx_2(__fx.__fx_Clock, __fx.__fx_Logger)))
    work(__fx2, "4. stamped")
    val __fx3 = __Fx_2(__fx2.__fx_Clock, Numbered(__Fx_3(__fx2.__fx_Logger)))
    work(__fx3, "4. numbered, then stamped")
    work(__fx3, "4. and again")
}

fun<__Fx> scoping(__fx: __Fx) where __Fx : __Has_Logger {
    work(__fx, "5. before the block")
    if (true) {
        val __fx2 = __Fx_3(QuietLogger())
        work(__fx2, "5. this line is swallowed")
    }
    work(__fx, "5. after the block, logging again")
}

interface Audit {
    fun record(what: String)
}

interface Metrics {
    fun record(what: String)
}

class ConsoleAudit(private val __fx: __Fx_1) : Audit {

    override fun record(what: String) {
        println(__fx, "   audit: $what")
    }
}

class ConsoleMetrics(private val __fx: __Fx_1) : Metrics {

    override fun record(what: String) {
        println(__fx, "   metric: $what")
    }
}

fun<__Fx> audit_only(__fx: __Fx, what: String) where __Fx : __Has_Audit {
    __fx.__fx_Audit.record(what)
}

fun<__Fx> audit_and_measure(__fx: __Fx, what: String) where __Fx : __Has_Audit, __Fx : __Has_Metrics {
    __fx.__fx_Audit.record(what)
    __fx.__fx_Metrics.record(what)
}

interface Setting<T> {
    fun setting(): T
}

class Fixed<T>(private val value: T) : Setting<T> {

    override fun setting(): T {
        return value
    }
}

fun<__Fx> settings(__fx: __Fx) where __Fx : __Has_Setting_Int, __Fx : __Has_Setting_String, __Fx : __Has_Console {
    val retries: Int = __fx.__fx_Setting_Int.setting()
    val region = __fx.__fx_Setting_String.setting()
    println(__fx, "   retries=$retries region=$region")
}

fun main() {
    val __fx = __Fx_1(StdOutConsole())
    val __fx2 = __Fx_4(TickingClock(), __fx.__fx_Console)
    println(__fx2, "1. the clock reads ${__fx2.__fx_Clock.now()}, then ${__fx2.__fx_Clock.now()}")
    println(__fx2, "2. two effects in one signature:")
    stamp(__fx2, "2. a labelled moment")
    println(__fx2, "3. a logger whose handler needs the console:")
    val __fx3 = __Fx_5(__fx2.__fx_Clock, __fx2.__fx_Console, PlainLogger(__Fx_1(__fx2.__fx_Console)))
    work(__fx3, "3. logged through the console")
    println(__fx3, "4. interception — each `use` wraps the one before it:")
    interception(__fx3)
    println(__fx3, "5. shadowing is not wrapping:")
    scoping(__fx3)
    println(__fx3, "6. two effects, one member name:")
    val __fx4 = __Fx_6(ConsoleAudit(__Fx_1(__fx3.__fx_Console)), __fx3.__fx_Clock, __fx3.__fx_Console, __fx3.__fx_Logger)
    audit_only(__fx4, "6. audited only")
    val __fx5 = __Fx_7(__fx4.__fx_Audit, __fx4.__fx_Clock, __fx4.__fx_Console, __fx4.__fx_Logger, ConsoleMetrics(__Fx_1(__fx4.__fx_Console)))
    audit_and_measure(__fx5, "6. audited and measured")
    println(__fx5, "7. two instances of one generic effect:")
    val __fx6 = __Fx_8(__fx5.__fx_Audit, __fx5.__fx_Clock, __fx5.__fx_Console, __fx5.__fx_Logger, __fx5.__fx_Metrics, Fixed<Int>(3))
    val __fx7 = __Fx_9(__fx6.__fx_Audit, __fx6.__fx_Clock, __fx6.__fx_Console, __fx6.__fx_Logger, __fx6.__fx_Metrics, __fx6.__fx_Setting_Int, Fixed<String>("eu-west-1"))
    settings(__fx7)
}

class __Fx_1(
    override val __fx_Console: Console,
) : __Has_Console

class __Fx_2(
    override val __fx_Clock: Clock,
    override val __fx_Logger: Logger,
) : __Has_Clock, __Has_Logger

class __Fx_3(
    override val __fx_Logger: Logger,
) : __Has_Logger

class __Fx_4(
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
) : __Has_Clock, __Has_Console

class __Fx_5(
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
    override val __fx_Logger: Logger,
) : __Has_Clock, __Has_Console, __Has_Logger

class __Fx_6(
    override val __fx_Audit: Audit,
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
    override val __fx_Logger: Logger,
) : __Has_Audit, __Has_Clock, __Has_Console, __Has_Logger

class __Fx_7(
    override val __fx_Audit: Audit,
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
    override val __fx_Logger: Logger,
    override val __fx_Metrics: Metrics,
) : __Has_Audit, __Has_Clock, __Has_Console, __Has_Logger, __Has_Metrics

class __Fx_8(
    override val __fx_Audit: Audit,
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
    override val __fx_Logger: Logger,
    override val __fx_Metrics: Metrics,
    override val __fx_Setting_Int: Setting<Int>,
) : __Has_Audit, __Has_Clock, __Has_Console, __Has_Logger, __Has_Metrics, __Has_Setting_Int

class __Fx_9(
    override val __fx_Audit: Audit,
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
    override val __fx_Logger: Logger,
    override val __fx_Metrics: Metrics,
    override val __fx_Setting_Int: Setting<Int>,
    override val __fx_Setting_String: Setting<String>,
) : __Has_Audit, __Has_Clock, __Has_Console, __Has_Logger, __Has_Metrics, __Has_Setting_Int, __Has_Setting_String
