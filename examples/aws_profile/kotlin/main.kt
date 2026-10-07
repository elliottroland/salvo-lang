package salvo.main

import salvo.*

fun main() {
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val creds: salvo.aws.ProfileCredentials = salvo.aws.ProfileCredentials(profile = "default", path = "~/.aws/credentials")
    salvo.core.console.println(__handle_2, "profile: ${creds.profile}")
    salvo.core.console.println(__handle_2, "path:    ${creds.path}")
    val staging: salvo.aws.ProfileCredentials = salvo.aws.ProfileCredentials(profile = "staging", path = "/etc/aws/credentials")
    salvo.core.console.println(__handle_2, "${salvo.aws.toStr(staging)}")
}

