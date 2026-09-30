// The Salvo client generator for AWS services: a smithy-build plugin
// (`salvo-client-codegen`) plus a small driver that runs smithy-build over
// `../smithy-build.json`. See ../README.md for how to run it.
plugins {
    java
    application
}

repositories {
    mavenCentral()
}

val smithyVersion = "1.74.0"

dependencies {
    implementation("software.amazon.smithy:smithy-model:$smithyVersion")
    implementation("software.amazon.smithy:smithy-build:$smithyVersion")
    implementation("software.amazon.smithy:smithy-aws-traits:$smithyVersion")
    implementation("software.amazon.smithy:smithy-rules-engine:$smithyVersion")
    implementation("software.amazon.smithy:smithy-aws-endpoints:$smithyVersion")
    implementation("software.amazon.smithy:smithy-waiters:$smithyVersion")
    implementation("software.amazon.smithy:smithy-aws-smoke-test-model:$smithyVersion")
    implementation("software.amazon.smithy:smithy-smoke-test-traits:$smithyVersion")
    implementation("software.amazon.smithy:smithy-aws-cloudformation-traits:$smithyVersion")
    implementation("software.amazon.smithy:smithy-aws-iam-traits:$smithyVersion")
    // [D2, decision 14] The Kotlin SDK's member and operation names come from
    // smithy-kotlin's own naming functions, so the glue names exactly what
    // aws-sdk-kotlin generated. (smithy-rs is not published; the Rust names
    // use the same word-boundary splitter, which smithy-rs copied.)
    implementation("software.amazon.smithy.kotlin:smithy-kotlin-codegen:0.35.28")
}

java {
    toolchain { languageVersion.set(JavaLanguageVersion.of(21)) }
}

application {
    mainClass.set("salvo.codegen.Main")
}

// `./gradlew generate` — runs smithy-build over ../smithy-build.json and copies
// each service's output into ../salvo (the generated files are checked in).
tasks.register<JavaExec>("generate") {
    group = "salvo"
    description = "Regenerate the Salvo AWS service modules from the pinned models."
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("salvo.codegen.Main")
    workingDir = projectDir.parentFile
    args = listOf("smithy-build.json")
}

// `./gradlew fetchKotlinSdk` — resolves the Kotlin SDK artifacts the host glue
// compiles against and copies the whole closure into ../lib/kotlin, which
// `[kotlin] libs` in ../salvo.toml puts on the classpath. The coordinates are
// the ones `[kotlin] artifacts` records; keep the two in step.
val kotlinSdk by configurations.creating
dependencies {
    kotlinSdk("aws.sdk.kotlin:s3:1.9.11")
    kotlinSdk("aws.sdk.kotlin:sqs:1.9.11")
}
tasks.register<Sync>("fetchKotlinSdk") {
    group = "salvo"
    description = "Copy the Kotlin SDK jars the host glue needs into ../lib/kotlin."
    from(kotlinSdk)
    into(projectDir.parentFile.resolve("lib/kotlin"))
}
