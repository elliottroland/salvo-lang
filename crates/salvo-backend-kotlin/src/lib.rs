//! Kotlin backend for the Salvo compiler.

mod emit;

pub use emit::{emit_program, EmittedFile};

use std::path::{Path, PathBuf};

use salvo_backend::{Backend, BackendError};
use salvo_core::Program;

pub struct KotlinBackend;

impl Backend for KotlinBackend {
    fn name(&self) -> &'static str {
        "kotlin"
    }

    fn emit(&self, program: &Program, target_dir: &Path) -> Result<Vec<PathBuf>, BackendError> {
        let files = emit::emit_program(program).map_err(BackendError::Codegen)?;
        let mut written = Vec::with_capacity(files.len());
        for file in files {
            let path = target_dir.join(&file.rel_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &file.content)?;
            written.push(file.rel_path);
        }
        Ok(written)
    }
}
