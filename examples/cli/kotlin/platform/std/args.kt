// [platform-tree] [cli-args] std's command-line arguments, for `args`'s
// `threadsafe platform handler HostArgs`: the arguments the process was
// launched with, program name excluded. The JVM entry point's array already
// excludes argv[0]; the emitted `main` captures it into `salvo.SalvoArgs` as
// the program starts, and this handler reads it back.
package salvo.platform.std.args

class HostArgs : salvo.std.args.ArgsPlatform {
    override fun args(): List<String> = salvo.SalvoArgs.get()
}
