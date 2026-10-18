# The manifest — `salvo.toml`

A **project** is a directory holding a `salvo.toml`. It says what the sources
are, where the program starts, what to build for, where artifacts go, what
other projects it depends on, and what host libraries its platform code needs —
so the command line and the editor agree on what a project is.

```toml
[project]
name = "collections"
version = "0.1.0"

[build]
src = "salvo"          # the source root, relative to this file (default: here)
main = "salvo/main.sv" # the entry point, when several files declare one (optional)
backend = "*"          # rust, kotlin, or * for every backend
target = "out"         # where `salvo compile` writes, when one dir serves all backends
out = "build"          # where `salvo build` writes the artifact (default: build)
platform = "salvo/platform"  # host implementation files (required if there are any)

[rust]
target = "rust"        # where `salvo compile` writes this backend's output
out = "build/rust"     # where `salvo build` writes this backend's artifact

[kotlin]
target = "kotlin"
out = "build/kotlin"
```

Every command reads it — `salvo run`, `compile`, `build`, `test`, `analyze`,
`platform generate`, the language server — and a flag on the command line wins
over what the manifest states. Under `backend = "*"` a command runs once per
backend: `salvo compile` in an example directory regenerates both checked-in
trees.

A file **belongs to the nearest manifest above it**, and a nested manifest is
a boundary: a directory holding several projects (this repository, with `std/`
and every `examples/*/`) is several programs, and the editor analyses each
document under its own. A file with no manifest above it is analysed under the
editor's workspace root, and told so when that workspace does hold projects.

`[project] std = true` marks the standard library's own tree, so its files are
std to the checker and replace the copy built into the compiler.

## Output: `compile` versus `build`

Two commands write output, and they write different things to different
places:

* **`salvo compile`** emits the generated Rust or Kotlin *source* into its
  **`target`** directory — the deliverable you check in or feed to a host
  build. `[<backend>] target` names it per backend, `[build] target` names one
  directory for every backend, and `--target` overrides both.
* **`salvo build`** produces a persisted, runnable **artifact** in its **`out`**
  directory — a native binary for Rust, a jar plus an `sh` launcher for
  Kotlin. `[<backend>] out` names it per backend, `[build] out` names one base,
  and `--out` overrides both; with none set and a single backend, the default
  is `build`. `salvo build` runs the project's tests first and a failure fails
  the build (no artifact is written); `--skip-tests` builds without them.

Under `backend = "*"`, `salvo build` must be able to put each backend's
artifact somewhere of its own, so **each backend needs an explicit `out`** (a
per-backend `out`, or `[build] out`): with none, the command refuses before
building anything, rather than having two artifacts overwrite one directory.
(`salvo run` and `salvo test` never read these directories — they compile into
a scratch directory they clean up, so a `run` cannot delete a checked-in
`compile` tree.)

## Dependencies

A project can use the modules of **other projects**. It names them under
`[dependencies]`, and says where they are found with `[build] modules`:

```toml
[build]
modules = "salvo_modules"   # the directory dependencies live under, relative to this file

[dependencies]
aws = "0.1.0"               # `salvo_modules/aws/salvo.toml`, at that version
```

Each entry is a directory `<modules>/<name>/` holding a `salvo.toml` of its
own — a dependency is a project, and its own manifest says where its sources
are. The version is checked against the one that manifest states, so the line
is a claim about what is on disk; a mismatch, a directory that is not there, or
a `[dependencies]` table with no `modules` setting is an error naming the file.
A directory under `modules` that no entry names is not loaded.

A dependency's modules are ordinary modules. Their paths come from the
dependency's own layout with no prefix — `salvo_modules/aws/salvo/aws.sv` is
module `aws`, and `salvo/aws/s3.sv` would be `aws.s3` — so the dependency's
author owns its namespace by how the tree is laid out, and two projects
declaring one path collide as two files of your own would. What a dependency
does not export stays private to it. Its `main` is not your program's entry
point; its actor protocols are locked in its own `salvo.lock`, not yours; its
test annexes are never loaded; and its platform files — under the platform root
its own manifest names — are loaded with it, since its platform declarations
need them as much as yours do.

A dependency **may not declare a standard-library module** — a library must
not be able to redefine `core.list` on the programs that use it — unless its
own manifest says `[project] std = true`, in which case it *is* a standard
library and replaces the embedded modules it declares, exactly as a source
tree may. Nothing is transitive yet: a dependency's own `[dependencies]` are
not followed.

In the editor a file inside a dependency belongs to the dependency's own
manifest and is analysed as its own project; from the using project's side,
hovering a name that came from a dependency says which one.

## Host libraries

A `platform` companion may need a library of its target language — an AWS SDK,
a JSON parser. The manifest says which, per backend:

```toml
[rust]
crates = { aws-sdk-s3 = "1.0", mylib = { path = "vendor/mylib" } }   # as Cargo writes them

[kotlin]
artifacts = ["aws.sdk.kotlin:s3:1.0.0"]   # Maven coordinates, resolved by Gradle
libs = "lib/kotlin"                        # a directory of jars, put on the classpath
```

With crates declared the Rust backend writes a `Cargo.toml` beside the emitted
sources and builds with `cargo`; with none, it calls `rustc` on the crate root
as it always has, so a program that needs no host library pays nothing. The
Kotlin backend does the same with Gradle: with artifacts declared it writes a
`build.gradle.kts` beside the sources and has Gradle resolve them, then compiles
with `kotlinc` against the jars — Gradle's rules, so a Kotlin Multiplatform
coordinate like `aws.sdk.kotlin:s3` finds its JVM jar. Every jar under `libs`
joins the classpath too, for libraries no repository has. A dependency's
declarations join the build with it.

Both tools are the ones on PATH unless the manifest names another —
`[rust] cargo = "…"`, `[kotlin] gradle = "…"`, a path relative to the manifest
or a command name. A project's `gradlew` wrapper works as `gradle`, since the
two take the same command line. Only the project being built chooses; a
dependency's setting is ignored. The same crate, or the same artifact, declared
at two versions anywhere in the build is an error naming both — one version per
library, and the build will not choose.

## The lock file

Beside the manifest the compiler keeps **`salvo.lock`**: the protocol hash of
every `actor effect` the project declares, at the project's version. A build
whose protocol changed without a version bump is refused naming the effect —
so two versions of one protocol never meet in a rolling fleet under one name —
and bumping `version` relocks. See [Concurrency](Concurrency.md) for what a
protocol hash is.

## Every valid option

The whole of `salvo.toml`, section by section. An unknown key is an error, so
this is the complete set. Every path is relative to the directory holding the
manifest.

### `[project]`

| Key | Type | Meaning |
|---|---|---|
| `name` | string | The project's name. Used in messages and as the built binary's / jar's base name when it is the entry module's. |
| `version` | string | The project's version. Also the version actor protocols are locked at (`salvo.lock`). |
| `std` | bool (default `false`) | Marks the standard library's own tree: its files check as std (`intrinsic` allowed) and shadow the embedded std. A non-std project leaves this unset. |

### `[build]`

| Key | Type | Meaning |
|---|---|---|
| `src` | string (default `.`) | The source root. |
| `main` | string | The `.sv` file declaring `main`, when several files declare one. With one entry point, optional. |
| `backend` | `"rust"`, `"kotlin"`, or `"*"` | The default backend; `*` is every backend. `--backend` overrides it. |
| `target` | string | Where `salvo compile` writes generated source, for every backend. A `[<backend>] target` overrides it; `--target` overrides both. |
| `out` | string (default `build`) | Where `salvo build` writes the artifact. A `[<backend>] out` overrides it; `--out` overrides both. |
| `platform` | string | The directory of host implementation files for every backend ([platform roots](Backends.md)). No default; required when the project has platform declarations. A `[<backend>] platform` overrides it. |
| `modules` | string | The directory dependencies live under. Required when `[dependencies]` is non-empty. |

### `[rust]` and `[kotlin]`

Per-backend settings. Each `target`, `out` and `platform` overrides the
`[build]` setting of the same name **for that backend only**.

| Key | Backend | Type | Meaning |
|---|---|---|---|
| `target` | both | string | This backend's `salvo compile` output directory, over `[build] target`. |
| `out` | both | string | This backend's `salvo build` artifact directory, over `[build] out`. |
| `platform` | both | string | This backend's platform root, over `[build] platform`. |
| `crates` | rust | table | Cargo dependencies the platform companions need, written verbatim into an emitted `Cargo.toml` (`name = "1.0"`, or `name = { path = "…", features = [ … ] }`). |
| `cargo` | rust | string (default `cargo` on PATH) | The `cargo` to build with when crates are declared: a path or a command name. |
| `artifacts` | kotlin | array of strings | Maven coordinates (`group:artifact:version`) the platform companions need, resolved by Gradle. |
| `gradle` | kotlin | string (default `gradle` on PATH) | The Gradle to resolve `artifacts` with: a path (a `gradlew` wrapper works) or a command name. |
| `libs` | kotlin | string | A directory whose `*.jar` files go on the Kotlin classpath, for libraries no repository has. |

### `[dependencies]`

A table of `name = "version"`. Each entry names a directory
`<[build] modules>/<name>/` holding a project at that exact version. See
[Dependencies](#dependencies).

### How the settings interact

* **CLI flag → manifest → default.** A `--backend`/`--src`/`--target`/`--out`
  flag always wins; the manifest fills what no flag set; a built-in default
  fills the rest.
* **Per-backend over shared.** A `[<backend>]` `target`/`out`/`platform`
  overrides the `[build]` one, for that backend only.
* **`backend = "*"` fans out.** Every command runs once per backend. For
  `salvo build` this means each backend needs its own `out` (see
  [Output](#output-compile-versus-build)); a single `--out` or `[build] out`
  cannot serve two.
* **`out` and `target` are independent** — one is `salvo build`'s artifact
  directory, the other is `salvo compile`'s generated source directory, and a
  project may set either, both, or neither.
* **`platform` is required precisely when there are platform declarations**,
  and the file that declares a `platform handler`/`platform fn`/`platform type`
  needs its host implementation under that root ([Backends](Backends.md)).
* **`modules` is required precisely when `[dependencies]` is non-empty**; a
  `[dependencies]` entry and `crates`/`artifacts` are orthogonal (one names a
  Salvo project, the others name target-language libraries).
* **`std = true` and `[dependencies]`.** Only a `std = true` project may
  declare standard-library modules; a dependency that is not `std = true` may
  not redefine `core.*`.
