//! Backend abstraction for the Salvo compiler.
//!
//! Each target language implements [`Backend`]. Backends are registered in a
//! [`BackendRegistry`] and selected by name (`salvo compile --backend kotlin`).

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use salvo_core::Program;

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

impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        BackendError::Io(err)
    }
}

/// A target-language code generator.
pub trait Backend {
    /// The name used to select this backend (`--backend <name>`) and to
    /// match define files (`module.<name>.sv`).
    fn name(&self) -> &'static str;

    /// Emits target source code for the program into `target_dir`.
    /// Returns the list of files written (relative to `target_dir`).
    fn emit(
        &self,
        program: &Program,
        target_dir: &Path,
    ) -> Result<Vec<std::path::PathBuf>, BackendError>;
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
