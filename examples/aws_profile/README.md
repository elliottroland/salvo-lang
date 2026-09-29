# aws_profile

A program with a **dependency**: the `aws` module lives in another project
(`modules/aws/` at the repository root), and this one reaches it through its
manifest rather than by copying the source in.

Run it:

```bash
cd examples/aws_profile
cargo run -q --manifest-path ../../Cargo.toml -- run                  # both backends
cargo run -- run --backend rust --src examples/aws_profile/salvo      # from the root
```

## What to look for

**The manifest names the dependency and where dependencies live.** Two lines
in `salvo.toml` do the whole job:

```toml
[build]
modules = "../../modules"     # the directory dependencies are found under

[dependencies]
aws = "0.1.0"                 # `<modules>/aws/salvo.toml`, at that version
```

A dependency is a project: a directory holding its own `salvo.toml`, found by
name under `modules`. The version is checked against the one the dependency's
manifest states, so the line is a claim about what is on disk rather than a
note to self. A project of its own would keep a `salvo_modules/` beside its
manifest; this example points at the repository's shared copy so the module
is written once.

**A dependency's modules are ordinary modules.** `import aws.ProfileCredentials`
reads like any import, the struct is private unless `aws.sv` says `export`,
and the module path comes from the dependency's own layout — `salvo/aws.sv`
is module `aws`, with no prefix from the dependency's name. Its `main`, if it
had one, would not be this program's entry point; its actor protocols are
locked in its own `salvo.lock`, not this one.

**A module has documentation of its own.** The comment at the top of
`modules/aws/salvo/aws.sv` is the module's, because a blank line separates it
from the first declaration (a comment sitting directly above a declaration is
that declaration's). The language server shows it wherever the module is
named: hover `aws` on the import line, or the module in an `@aws` selector.

**What a dependency may not do.** Declare a std module. A dependency whose
tree holds a `core/list.sv` is refused, naming the file, unless its own
manifest says `[project] std = true` — then it *is* a standard library and
replaces the embedded modules it declares, the way `salvo test --src std`
lets the checkout do.

**What the module holds today.** One struct, `ProfileCredentials`: the profile
name and credentials-file path a program needs before it can talk to any
service, with the AWS CLI's defaults. `: ToStr<self> by auto` in the
dependency stamps a `to_str` over its fields, and `"${staging}"` here finds it
through the import. The services themselves — modelled as actors — come
later, each with design notes of its own.
