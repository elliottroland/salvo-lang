// A program that uses a **dependency**: `aws` comes from another project, named
// under `[dependencies]` in `salvo.toml` and found by name under the directory
// `[build] modules` points at (here `../../modules`, so the example shares the
// repository's copy; a project of its own would keep a `salvo_modules/`
// beside its manifest).
//
// A dependency's modules are ordinary modules: they arrive by `import`, they
// are private unless they say `export`, and their paths come from the
// dependency's own layout — `aws.sv` is module `aws`, with no prefix. Hovering
// `aws` on the import line below shows the module's own documentation, the
// comment at the top of its file.
import aws.ProfileCredentials

fn main() [use] {
    use StdOutConsole()

    // The defaults are the AWS CLI's: the `default` profile, in the file the
    // CLI writes.
    let creds = ProfileCredentials {}
    println("profile: ${creds.profile}")
    println("path:    ${creds.path}")

    // The struct prints in its literal's shape — `ToStr<self> by auto` in the
    // dependency stamped a `to_str` over its fields, and interpolation finds it
    // here through the import.
    let staging = ProfileCredentials {profile: "staging", path: "/etc/aws/credentials"}
    println("${staging}")
}
