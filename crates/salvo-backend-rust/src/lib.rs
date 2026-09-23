//! Rust backend for the Salvo compiler.

mod emit;
mod intrinsics;

pub use emit::{
    emit_program, emit_program_reporting, emit_program_with_entry, platform_skeletons,
    EmittedFile,
};

use std::path::{Path, PathBuf};

use salvo_backend::{run_tool, Backend, BackendError, Emitted};
use salvo_core::{ModulePath, Program};

/// Where the linked binary goes inside the target directory [rs-run].
/// Dot-prefixed so a target nested in the source tree stays invisible to
/// source discovery [mod-ignore].
const BIN_DIR: &str = ".salvo_bin";

pub struct RustBackend;

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn file_extension(&self) -> &'static str {
        "rs"
    }

    /// [rs-crate] The crate root is the `main`-declaring module, so a
    /// program with several candidates needs `entry` to say which — building
    /// the wrong one leaves the `mod` declarations behind and rustc reports
    /// unresolved imports.
    fn emit(
        &self,
        program: &Program,
        target_dir: &Path,
        entry: Option<&ModulePath>,
    ) -> Result<Emitted, BackendError> {
        let (files, warnings) = emit::emit_program_reporting(program, entry)
            .map_err(BackendError::Codegen)?;
        let mut written = Vec::with_capacity(files.len());
        for file in files {
            let path = target_dir.join(&file.rel_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &file.content)?;
            written.push(file.rel_path);
        }
        Ok(Emitted {
            files: written,
            warnings,
        })
    }

    /// [platform-tree] The crate root is still the file to build: when the
    /// host owns `main`, the root's own `fn main` delegates to it, so the
    /// `rustc` invocation is unchanged.
    fn entry_hint(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        _emitted: &[PathBuf],
    ) -> String {
        let root = target_dir.join(crate_root(main_module));
        format!(
            "{} (build with: rustc --edition 2021 {})",
            root.display(),
            root.display()
        )
    }

    fn platform_skeletons(
        &self,
        program: &Program,
        entry: Option<&ModulePath>,
    ) -> Result<Vec<(PathBuf, String)>, BackendError> {
        let files =
            emit::platform_skeletons(program, entry).map_err(BackendError::Codegen)?;
        Ok(files
            .into_iter()
            .map(|f| (f.rel_path, f.content))
            .collect())
    }

    /// [rs-run] The module declaring `main` is the crate root, so one
    /// `rustc` invocation on that file builds the whole program (the other
    /// modules are reached through its `mod` declarations). The binary goes
    /// inside the target directory, then runs.
    fn program_command(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        _emitted: &[PathBuf],
    ) -> Result<std::process::Command, BackendError> {
        use std::ffi::OsStr;

        let root = target_dir.join(crate_root(main_module));
        if !root.is_file() {
            return Err(BackendError::Other(format!(
                "nothing to run: expected the crate root `{}` to have been emitted",
                root.display()
            )));
        }
        let bin_dir = target_dir.join(BIN_DIR);
        std::fs::create_dir_all(&bin_dir)?;
        let bin = bin_dir.join(main_module.0.last().map(String::as_str).unwrap_or("program"));

        let code = run_tool(
            "rustc",
            &[
                OsStr::new("--edition"),
                OsStr::new("2021"),
                root.as_os_str(),
                OsStr::new("-o"),
                bin.as_os_str(),
            ],
        )?;
        if code != 0 {
            return Err(BackendError::Other(format!(
                "rustc failed with exit code {code}"
            )));
        }
        Ok(std::process::Command::new(bin))
    }
}

/// The emitted crate-root path for the module declaring `main`
/// (`main` -> `main.rs`, `app.entry` -> `app/entry.rs`), relative to the
/// target directory. See BACKEND_SPEC.rust.md.
fn crate_root(main_module: &ModulePath) -> PathBuf {
    let mut path = PathBuf::new();
    for part in &main_module.0 {
        path.push(part);
    }
    path.set_extension("rs");
    path
}
