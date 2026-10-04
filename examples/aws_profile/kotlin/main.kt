package salvo.main

import salvo.aws.*
import salvo.core.console.*

fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val creds = ProfileCredentials()
    println(console, "profile: ${creds.profile}")
    println(console, "path:    ${creds.path}")
    val staging = ProfileCredentials(profile = "staging", path = "/etc/aws/credentials")
    println(console, "${toStr__9(staging)}")
}
