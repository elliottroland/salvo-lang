//! Backend abstraction for the Salvo compiler.
//!
//! Each target language implements [`Backend`]. Backends are registered in a
//! [`BackendRegistry`] and selected by name (`salvo compile --backend kotlin`).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use salvo_core::{ModulePath, Program};

#[derive(Debug)]
pub enum BackendError {
    Unsupported(String),
    Io(std::io::Error),
    /// Code-generation errors, one message per problem.
    Codegen(Vec<String>),
    Other(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::Unsupported(msg) => write!(f, "{msg}"),
            BackendError::Io(err) => write!(f, "I/O error: {err}"),
            BackendError::Codegen(msgs) => write!(f, "{}", msgs.join("\n")),
            BackendError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// What [`Backend::emit`] produced: the files written, and the *non-fatal*
/// diagnostics the checker reported on the way [diag-structured].
///
/// Warnings need a channel of their own because they are not failures: a
/// suppressed refinement conflict [qual-refn-ambiguous] leaves a legal
/// program that must still compile, and reporting it through
/// [`BackendError`] would have to abort. Rendered here rather than
/// structured, for the same reason codegen errors are: rendering happens at
/// the backend boundary, and the driver only prints them.
#[derive(Debug, Default)]
pub struct Emitted {
    /// Files written, relative to the target directory.
    pub files: Vec<PathBuf>,
    /// Rendered warnings, one per line, each carrying its own `warning:`
    /// prefix and location.
    pub warnings: Vec<String>,
}

impl Emitted {
    /// An emission that reported nothing.
    pub fn new(files: Vec<PathBuf>) -> Self {
        Emitted {
            files,
            warnings: Vec::new(),
        }
    }
}

impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        BackendError::Io(err)
    }
}

/// A target-language code generator.
pub trait Backend {
    /// The name used to select this backend (`--backend <name>`).
    fn name(&self) -> &'static str;

    /// The extension of the backend's native source files (`kt` for
    /// Kotlin): what this backend emits, what companion files use
    /// [backend-companion], and what `clean`ing stale output targets.
    fn file_extension(&self) -> &'static str;

    /// Emits target source code for the program into `target_dir`.
    /// Returns the files written (relative to `target_dir`) together with
    /// any non-fatal diagnostics [`Emitted`].
    ///
    /// `entry` is the module the driver selected as the program's entry
    /// point (`salvo run --main`), or `None` to let the backend discover
    /// it. It matters wherever the entry shapes the *output* and not just
    /// the launch command: Rust gives the `main`-declaring module the crate
    /// root [rs-crate], so with several candidates the backend must be told
    /// which one, while Kotlin emits a `MainKt` per module and needs
    /// nothing.
    fn emit(
        &self,
        program: &Program,
        target_dir: &Path,
        entry: Option<&ModulePath>,
    ) -> Result<Emitted, BackendError>;

    /// How a human starts the emitted program — a JVM class name, a
    /// `rustc` invocation. `main_module` is the module declaring `main`,
    /// and `emitted` is what [`Backend::emit`] wrote, which is how the
    /// backend can tell whether the *host* owns the entry point
    /// [platform-tree]. Printed by `salvo compile`; the knowledge belongs
    /// to the backend, not to the CLI.
    fn entry_hint(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> String;

    /// [platform-tree] Renders the host implementation skeleton for every
    /// platform effect the program declares: `salvo platform generate`'s
    /// output, as `(path relative to the source root, contents)` pairs.
    /// `entry` is the driver's chosen entry module, which matters for the
    /// same reason it does in [`Backend::emit`] — Rust puts it at the crate
    /// root, so the host addresses its items differently.
    ///
    /// The compiler writes these once and never again — every later drift
    /// between host and interface is a target-language compile error — so
    /// the caller must not overwrite a file that already exists.
    fn platform_skeletons(
        &self,
        program: &Program,
        entry: Option<&ModulePath>,
    ) -> Result<Vec<(PathBuf, String)>, BackendError> {
        let _ = (program, entry);
        Err(BackendError::Unsupported(format!(
            "backend `{}` has no platform interop",
            self.name()
        )))
    }

    /// Builds the emitted sources with the target toolchain and returns the
    /// **command that launches the program** [cli-run] [test-run].
    /// `emitted` is what [`Backend::emit`] wrote (relative to `target_dir`),
    /// so the backend need not re-discover it.
    ///
    /// The build happens here; the caller decides how the program is run —
    /// with inherited stdio ([`Backend::run`]), or with its output captured
    /// and rendered, which is what `salvo test` does with the harness's
    /// protocol. A backend that cannot run programs returns
    /// [`BackendError::Unsupported`], which is the default.
    fn program_command(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> Result<std::process::Command, BackendError> {
        let _ = (target_dir, main_module, emitted);
        Err(BackendError::Unsupported(format!(
            "backend `{}` cannot run programs",
            self.name()
        )))
    }

    /// Builds the emitted sources with the target toolchain and runs the
    /// program, returning its exit code [cli-run]. The program's stdio is
    /// inherited: its output is the command's output.
    ///
    /// Defined in terms of [`Backend::program_command`], which is where a
    /// backend does the work.
    fn run(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> Result<i32, BackendError> {
        let mut command = self.program_command(target_dir, main_module, emitted)?;
        let program = command.get_program().to_string_lossy().to_string();
        match command.status() {
            Ok(status) => Ok(status.code().unwrap_or(1)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Err(BackendError::Unsupported(format!(
                    "`{program}` was not found on PATH: it is needed to run the \
                     emitted code"
                )))
            }
            Err(err) => Err(BackendError::Io(err)),
        }
    }
}

/// Runs a toolchain command with inherited stdio, mapping a missing
/// executable to a message that names it [cli-run]. `Ok(code)` carries the
/// tool's own exit code — a compiler failing is not an error *here*, it is
/// a result the caller reports.
pub fn run_tool(program: &str, args: &[&std::ffi::OsStr]) -> Result<i32, BackendError> {
    use std::process::Command;
    match Command::new(program).args(args).status() {
        Ok(status) => Ok(status.code().unwrap_or(1)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(BackendError::Unsupported(format!(
                "`{program}` was not found on PATH: it is needed to build and run \
                 the emitted code"
            )))
        }
        Err(err) => Err(BackendError::Io(err)),
    }
}

/// Registry of available backends, keyed by name.
#[derive(Default)]
pub struct BackendRegistry {
    backends: BTreeMap<&'static str, Box<dyn Backend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, backend: Box<dyn Backend>) {
        self.backends.insert(backend.name(), backend);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Backend> {
        self.backends.get(name).map(|b| b.as_ref())
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.backends.keys().copied()
    }
}
