package salvo.main

import salvo.core.console.*
import salvo.core.string.*

interface Clock {
    fun now(): Int
}

class __Mon_Clock(
    private val inner: Clock,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Clock {
    override fun now(): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.now() } finally { lock.unlock() }
    }
}

class TickingClock : Clock {
    private var tick: Int = 0

    override fun now(): Int {
        tick = tick + 5
        return tick
    }
}

fun stamp(clock: Clock, console: Console, label: String) {
    val t = clock.now()
    banner(console, "$label at t=$t")
}

fun banner(console: Console, text: String) {
    println(console, "   $text")
}

interface Logger {
    fun log(message: String)
}

class __Mon_Logger(
    private val inner: Logger,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Logger {
    override fun log(message: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.log(message) } finally { lock.unlock() }
    }
}

class PlainLogger(private val __dep_Console: Console) : Logger {

    override fun log(message: String) {
        println(__dep_Console, "   $message")
    }
}

class QuietLogger : Logger {

    override fun log(message: String) {
    }
}

class Stamped(private val __dep_Logger: Logger, private val __dep_Clock: Clock) : Logger {

    override fun log(message: String) {
        __dep_Logger.log("[t=${__dep_Clock.now()}] $message")
    }
}

class Numbered(private val __dep_Logger: Logger) : Logger {
    private var seen: Int = 0

    override fun log(message: String) {
        seen = seen + 1
        __dep_Logger.log("#$seen $message")
    }
}

fun work(logger: Logger, step: String) {
    logger.log(step)
}

fun interception(logger: Logger, clock: Clock) {
    work(logger, "4. plain")
    val logger2: Logger = Stamped(logger, clock)
    work(logger2, "4. stamped")
    val logger3: Logger = __Mon_Logger(Numbered(logger2))
    work(logger3, "4. numbered, then stamped")
    work(logger3, "4. and again")
}

fun scoping(logger: Logger) {
    work(logger, "5. before the block")
    if (true) {
        val logger2: Logger = QuietLogger()
        work(logger2, "5. this line is swallowed")
    }
    work(logger, "5. after the block, logging again")
}

interface Audit {
    fun record(what: String)
}

class __Mon_Audit(
    private val inner: Audit,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Audit {
    override fun record(what: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.record(what) } finally { lock.unlock() }
    }
}

class ConsoleAudit(private val __dep_Console: Console) : Audit {

    override fun record(what: String) {
        println(__dep_Console, "   audit: $what")
    }
}

class ConsoleMetrics(private val __dep_Console: Console) : Metrics {

    override fun record(what: String) {
        println(__dep_Console, "   metric: $what")
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.setting(copy) } finally { lock.unlock() }
    }
}

class Fixed<T>(private val value: T) : Setting<T> {

    override fun setting(copy: (T) -> T): T {
        return copy(value)
    }
}

fun settings(setting_int: Setting<Int>, setting_string: Setting<String>, console: Console) {
    val retries: Int = setting_int.setting({ __i0 -> __i0 })
    val region = setting_string.setting({ __i0 -> __i0 })
    println(console, "   retries=$retries region=$region")
}

fun main() {
    val console: Console = StdOutConsole()
    val clock: Clock = __Mon_Clock(TickingClock())
    println(console, "1. the clock reads ${clock.now()}, then ${clock.now()}")
    println(console, "2. two effects in one signature:")
    stamp(clock, console, "2. a labelled moment")
    println(console, "3. a logger whose handler needs the console:")
    val logger: Logger = PlainLogger(console)
    work(logger, "3. logged through the console")
    println(console, "4. interception — each `use` wraps the one before it:")
    interception(logger, clock)
    println(console, "5. shadowing is not wrapping:")
    scoping(logger)
    println(console, "6. two effects, one member name:")
    val audit: Audit = ConsoleAudit(console)
    auditOnly(audit, "6. audited only")
    val metrics: Metrics = ConsoleMetrics(console)
    auditAndMeasure(audit, metrics, "6. audited and measured")
    println(console, "7. two instances of one generic effect:")
    val setting_int: Setting<Int> = Fixed<Int>(3)
    val setting_string: Setting<String> = Fixed<String>("eu-west-1")
    settings(setting_int, setting_string, console)
}
