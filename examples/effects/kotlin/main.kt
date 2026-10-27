package salvo.main

import salvo.*

interface Clock {
    fun now(): Int
}

class __Mon_Clock(
    private val inner: Clock,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Clock {
    override fun now(): Int {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.now() } finally { lock.unlock() }
    }
}

class TickingClock : Clock {
    var tick: Int = 0
    override fun now(): Int {
        tick = (tick + 5)
        return tick
    }
}

fun stamp(clock: Clock, console: salvo.std.core.console.Console, label: String) {
    val t: Int = clock.now()
    salvo.main.banner(console, "${label} at t=${t}")
}

fun banner(console: salvo.std.core.console.Console, text: String) {
    salvo.std.core.console.println(console, "   ${text}")
}

interface Logger {
    fun log(message: String)
}

class __Mon_Logger(
    private val inner: Logger,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Logger {
    override fun log(message: String) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.log(message) } finally { lock.unlock() }
    }
}

class PlainLogger(private val __dep0: salvo.std.core.console.Console) : Logger {
    override fun log(message: String) {
        salvo.std.core.console.println(__dep0, "   ${message}")
    }
}

class QuietLogger : Logger {
    override fun log(message: String) {
    }
}

class Stamped(private val __dep0: Logger, private val __dep1: Clock) : Logger {
    override fun log(message: String) {
        __dep0.log("[t=${__dep1.now()}] ${message}")
    }
}

class Numbered(private val __dep0: Logger) : Logger {
    var seen: Int = 0
    override fun log(message: String) {
        seen = (seen + 1)
        __dep0.log("#${seen} ${message}")
    }
}

fun work(logger: Logger, step: String) {
    logger.log(step)
}

fun interception(logger: Logger, clock: Clock) {
    salvo.main.work(logger, "4. plain")
    val __use_1: Stamped = Stamped(logger, clock)
    val __lock___use_1 = java.util.concurrent.locks.ReentrantLock()
    val __handle_2: Logger = __Mon_Logger(__use_1, __lock___use_1)
    salvo.main.work(__handle_2, "4. stamped")
    val __use_3: Numbered = Numbered(__handle_2)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Logger = __Mon_Logger(__use_3, __lock___use_3)
    salvo.main.work(__handle_4, "4. numbered, then stamped")
    salvo.main.work(__handle_4, "4. and again")
}

fun scoping(logger: Logger) {
    salvo.main.work(logger, "5. before the block")
    kotlin.run {
        val __use_1: QuietLogger = QuietLogger()
        val __lock___use_1 = java.util.concurrent.locks.ReentrantLock()
        val __handle_2: Logger = __Mon_Logger(__use_1, __lock___use_1)
        salvo.main.work(__handle_2, "5. this line is swallowed")
    }
    salvo.main.work(logger, "5. after the block, logging again")
}

interface Audit {
    fun record(what: String)
}

class __Mon_Audit(
    private val inner: Audit,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Audit {
    override fun record(what: String) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.record(what) } finally { lock.unlock() }
    }
}

interface Metrics {
    fun record(what: String)
}

class __Mon_Metrics(
    private val inner: Metrics,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Metrics {
    override fun record(what: String) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.record(what) } finally { lock.unlock() }
    }
}

class ConsoleAudit(private val __dep0: salvo.std.core.console.Console) : Audit {
    override fun record(what: String) {
        salvo.std.core.console.println(__dep0, "   audit: ${what}")
    }
}

class ConsoleMetrics(private val __dep0: salvo.std.core.console.Console) : Metrics {
    override fun record(what: String) {
        salvo.std.core.console.println(__dep0, "   metric: ${what}")
    }
}

fun auditOnly(audit: Audit, what: String) {
    audit.record(what)
}

fun auditAndMeasure(audit: Audit, metrics: Metrics, what: String) {
    audit.record(what)
    metrics.record(what)
}

interface Setting<T> {
    fun setting(copy: (T) -> T): T
}

class __Mon_Setting<T>(
    private val inner: Setting<T>,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Setting<T> {
    override fun setting(copy: (T) -> T): T {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.setting(copy) } finally { lock.unlock() }
    }
}

class Fixed<T>(private val value: T) : Setting<T> {
    override fun setting(copy: (T) -> T): T {
        return copy(value)
    }
}

fun settings(setting: Setting<Int>, setting__1: Setting<String>, console: salvo.std.core.console.Console) {
    val retries: Int = setting.setting({ __a0 -> __a0 })
    val region: String = setting__1.setting({ __a0 -> __a0 })
    salvo.std.core.console.println(console, "   retries=${retries} region=${region}")
}

fun main() {
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val __use_3: TickingClock = TickingClock()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Clock = __Mon_Clock(__use_3, __lock___use_3)
    salvo.std.core.console.println(__handle_2, "1. the clock reads ${__handle_4.now()}, then ${__handle_4.now()}")
    salvo.std.core.console.println(__handle_2, "2. two effects in one signature:")
    salvo.main.stamp(__handle_4, __handle_2, "2. a labelled moment")
    salvo.std.core.console.println(__handle_2, "3. a logger whose handler needs the console:")
    val __use_5: PlainLogger = PlainLogger(__handle_2)
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: Logger = __Mon_Logger(__use_5, __lock___use_5)
    salvo.main.work(__handle_6, "3. logged through the console")
    salvo.std.core.console.println(__handle_2, "4. interception — each `use` wraps the one before it:")
    salvo.main.interception(__handle_6, __handle_4)
    salvo.std.core.console.println(__handle_2, "5. shadowing is not wrapping:")
    salvo.main.scoping(__handle_6)
    salvo.std.core.console.println(__handle_2, "6. two effects, one member name:")
    val __use_7: ConsoleAudit = ConsoleAudit(__handle_2)
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: Audit = __Mon_Audit(__use_7, __lock___use_7)
    salvo.main.auditOnly(__handle_8, "6. audited only")
    val __use_9: ConsoleMetrics = ConsoleMetrics(__handle_2)
    val __lock___use_9 = java.util.concurrent.locks.ReentrantLock()
    val __handle_10: Metrics = __Mon_Metrics(__use_9, __lock___use_9)
    salvo.main.auditAndMeasure(__handle_8, __handle_10, "6. audited and measured")
    salvo.std.core.console.println(__handle_2, "7. two instances of one generic effect:")
    val __use_11: Fixed<Int> = Fixed<Int>(value = 3)
    val __lock___use_11 = java.util.concurrent.locks.ReentrantLock()
    val __handle_12: Setting<Int> = __Mon_Setting<Int>(__use_11, __lock___use_11)
    val __use_13: Fixed<String> = Fixed<String>(value = "eu-west-1")
    val __lock___use_13 = java.util.concurrent.locks.ReentrantLock()
    val __handle_14: Setting<String> = __Mon_Setting<String>(__use_13, __lock___use_13)
    salvo.main.settings(__handle_12, __handle_14, __handle_2)
}

