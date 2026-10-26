// [platform-tree] [cli-env] std's environment variables, for `env`'s
// `threadsafe platform handler HostEnv`.
package salvo.platform.std.env

class HostEnv : salvo.std.env.EnvPlatform {
    override fun env(name: String): String? = System.getenv(name)
}
