// AWS, for Salvo programs: the services a program talks to, modelled as
// **actors** — a service is a mailbox you `send` to and a `Reply<T>` answers
// through, so a call across the network has the shape every other
// asynchronous thing in Salvo has. This is the first `[dependencies]` module
// (2026-09-29), and for now it holds only the type a program needs before it
// can talk to anything: where its credentials come from. The services follow,
// each in its own file under `aws/`, with design notes of their own.
//
// Arrives by being declared: `aws = "0.1.0"` under `[dependencies]`, then
// `import aws.ProfileCredentials` or `import aws`. The design — generation from
// the Smithy models, host SDKs wrapped, non-blocking through `Reply`, bodies as
// `std.stream` — is `DESIGN.md` beside this module's manifest.

// Where a **named profile's** credentials are read from: the profile's name
// and the credentials file that holds it, in the layout the AWS CLI writes
// (`[default]` sections of `aws_access_key_id` / `aws_secret_access_key`).
//
// A value, not a capability: reading the file is the job of a handler this
// struct will be handed to, so a test can build one with a path that does not
// exist and nothing happens until something asks. [path] is used exactly as
// written — `~` is not expanded here; the host that opens the file decides
// what a home directory is.
export struct ProfileCredentials : ToStr<self> by auto {
    // The profile's section name in the credentials file.
    profile: Str = "default",
    // The credentials file, as a path the host filesystem understands.
    path: Str = "~/.aws/credentials"
}
