package salvo.main

import salvo.aws.ProfileCredentials
import salvo.aws.toStr
import salvo.core.console.Console
import salvo.core.console.println

fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val creds = ProfileCredentials()
    println(console, "profile: ${creds.profile}")
    println(console, "path:    ${creds.path}")
    val staging = ProfileCredentials(profile = "staging", path = "/etc/aws/credentials")
    println(console, "${toStr(staging)}")
}
