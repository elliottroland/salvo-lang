// [test-warm-kotlinc] A warm Kotlin compiler for the test suite.
//
// A cold `kotlinc` pays a JVM start and an un-JIT-ted compiler on every call
// (2.6s for an empty file, 7s for a hello-world program with its slice of std;
// the suite made 130 of them, 2,496s summed, 2026-10-29). This keeps one
// compiler JVM alive and runs `K2JVMCompiler` in it once per request: the same
// program compiles in about a second once the JIT has warmed up.
//
// One request at a time per server: the client (`client.rs`, installed as
// `kotlinc` on the tests' PATH) holds the lock of a slot while it uses that
// slot's server, so a compile never shares a JVM with another one. The server
// exits when it has been idle for a while, or after a number of compiles so a
// long session cannot accumulate whatever a compile leaves behind; the next
// client starts a fresh one.
//
// Protocol, over a Unix-domain socket in a 0700 directory (never TCP): the
// client writes a count and that many length-prefixed UTF-8 arguments, already
// absolute where they are paths; the server answers the exit code and the
// compiler's messages (what `kotlinc` would print on stderr), length-prefixed.
package salvo.testkit

import org.jetbrains.kotlin.cli.common.messages.MessageRenderer
import org.jetbrains.kotlin.cli.jvm.K2JVMCompiler
import org.jetbrains.kotlin.config.Services
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.PrintStream
import java.net.StandardProtocolFamily
import java.net.UnixDomainSocketAddress
import java.nio.channels.Channels
import java.nio.channels.ServerSocketChannel
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import kotlin.system.exitProcess

// `CLITool.exec` with a renderer is protected; the public one renders paths
// relative to this JVM's working directory, which is not the caller's. Full
// paths say the same thing wherever the caller stands.
private val execWithRenderer: java.lang.reflect.Method? =
    generateSequence<Class<*>>(K2JVMCompiler::class.java) { it.superclass }
        .flatMap { it.declaredMethods.asSequence() }
        .firstOrNull { m ->
            m.name == "exec" && m.parameterTypes.size == 4 && m.parameterTypes[2] == MessageRenderer::class.java
        }
        ?.also { it.isAccessible = true }

private fun compile(err: PrintStream, argv: Array<String>): Int {
    val compiler = K2JVMCompiler()
    val method = execWithRenderer ?: return compiler.exec(err, *argv).code
    val exit = method.invoke(compiler, err, Services.EMPTY, MessageRenderer.PLAIN_FULL_PATHS, argv)
    return (exit as org.jetbrains.kotlin.cli.common.ExitCode).code
}

fun main(args: Array<String>) {
    // Asked at install time: which JVM the real `kotlinc` runs on.
    if (args.firstOrNull() == "--java") {
        println(System.getProperty("java.home"))
        println(System.getProperty("java.specification.version"))
        return
    }
    val socket = Path.of(args[0])
    val idleMillis = args[1].toLong() * 1000
    val maxCompiles = args[2].toInt()
    Files.deleteIfExists(socket)
    val server = ServerSocketChannel.open(StandardProtocolFamily.UNIX)
    server.bind(UnixDomainSocketAddress.of(socket))
    val last = AtomicLong(System.currentTimeMillis())
    val busy = AtomicBoolean(false)
    val stop = {
        try {
            Files.deleteIfExists(socket)
        } catch (_: Exception) {
        }
        exitProcess(0)
    }
    val watchdog = Thread {
        while (true) {
            Thread.sleep(1000)
            if (!busy.get() && System.currentTimeMillis() - last.get() > idleMillis) {
                stop()
            }
            // A socket someone removed is a request to go.
            if (!Files.exists(socket)) {
                exitProcess(0)
            }
        }
    }
    watchdog.isDaemon = true
    watchdog.start()
    var compiles = 0
    while (true) {
        val channel = server.accept()
        busy.set(true)
        try {
            val input = DataInputStream(BufferedInputStream(Channels.newInputStream(channel)))
            val output = DataOutputStream(BufferedOutputStream(Channels.newOutputStream(channel)))
            val count = input.readInt()
            val argv = Array(count) {
                val bytes = ByteArray(input.readInt())
                input.readFully(bytes)
                String(bytes, Charsets.UTF_8)
            }
            val messages = ByteArrayOutputStream()
            val err = PrintStream(messages, true, "UTF-8")
            val code = try {
                compile(err, argv)
            } catch (t: Throwable) {
                t.printStackTrace(err)
                2
            }
            err.flush()
            val bytes = messages.toByteArray()
            output.writeInt(code)
            output.writeInt(bytes.size)
            output.write(bytes)
            output.flush()
        } catch (e: Exception) {
            System.err.println("kotlinc server: a request failed: $e")
        } finally {
            try {
                channel.close()
            } catch (_: Exception) {
            }
            compiles++
            last.set(System.currentTimeMillis())
            busy.set(false)
        }
        if (compiles >= maxCompiles) {
            stop()
        }
    }
}
