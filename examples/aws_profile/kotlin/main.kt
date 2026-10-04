package salvo.main

import salvo.aws.*
import salvo.core.console.*

fun main() {
    val console: Console = StdOutConsole()
    val creds = ProfileCredentials()
    println(console, "profile: ${creds.profile}")
    println(console, "path:    ${creds.path}")
    val staging = ProfileCredentials(profile = "staging", path = "/etc/aws/credentials")
    println(console, "${toStr__8(staging)}")
}
