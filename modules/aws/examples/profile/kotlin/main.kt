package salvo.main

import salvo.*

fun main() {
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val creds: salvo.aws.ProfileCredentials = salvo.aws.ProfileCredentials(profile = "default", path = "~/.aws/credentials")
    salvo.std.core.console.println(__handle_2, "profile: ${creds.profile}")
    salvo.std.core.console.println(__handle_2, "path:    ${creds.path}")
    val staging: salvo.aws.ProfileCredentials = salvo.aws.ProfileCredentials(profile = "staging", path = "/etc/aws/credentials")
    salvo.std.core.console.println(__handle_2, "${salvo.aws.toStr(staging)}")
}

