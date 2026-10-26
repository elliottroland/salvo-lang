// [platform-tree] [cli-process] std's process exit, for `process`'s
// `threadsafe platform handler HostProcess`: flush both standard streams, then
// end the process with the status asked for.
package salvo.platform.std.process

class HostProcess : salvo.std.process.ProcessPlatform {
    // [cli-process] `System.out` flushes only at a newline, so text printed
    // without one is still in its buffer.
    override fun exit(code: Int): Nothing {
        System.out.flush()
        System.err.flush()
        kotlin.system.exitProcess(code)
    }
}
