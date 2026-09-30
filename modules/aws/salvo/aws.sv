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

// Credentials from the process environment: `AWS_ACCESS_KEY_ID`,
// `AWS_SECRET_ACCESS_KEY` and, when set, `AWS_SESSION_TOKEN`.
export struct EnvironmentCredentials {}

// The SDKs' default provider chain: environment, then the shared profile
// files, then the container and instance metadata endpoints — what a program
// running inside AWS wants.
export struct DefaultChain {}

// Where a service client's credentials come from.
export type Credentials = ProfileCredentials | EnvironmentCredentials | DefaultChain

// An AWS region by its code: `eu-west-1`, `us-east-1`.
export struct Region { code: Str }

// How to reach a service: whose credentials, which region, and — for a local
// stand-in such as LocalStack — an endpoint to use instead of the region's.
export struct AwsConfig {
    credentials: Credentials = DefaultChain {},
    region: Region,
    // A full URL (`http://localhost:4566`); absent means the region's endpoint.
    endpoint: Str? = None
}

// A failure the service model does not name: throttling the model did not
// declare, an authentication or signing failure, a transport error, a response
// the SDK could not parse. [code] is the service's error code when it sent
// one, otherwise a short description of the kind of failure.
export struct AwsError { code: Str, message: Str }

// ===== for host code =====

// The profile [c] names, or absent when the credentials come from elsewhere:
// what a service's host code reads to configure its SDK client [host-splice].
export fn profile_of(c: Credentials) [] -> ProfileCredentials? => c {
    if c is ProfileCredentials {
        return copy(c)
    }
    return None
}

// Whether [c] reads the process environment.
export fn uses_environment(c: Credentials) [] -> Bool => c {
    return c is EnvironmentCredentials
}
