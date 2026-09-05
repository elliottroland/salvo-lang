//! Kotlin backend for the Salvo compiler.

mod emit;
mod intrinsics;

pub use emit::{emit_program, EmittedFile};

use std::path::{Path, PathBuf};

use salvo_backend::{run_tool, Backend, BackendError};
use salvo_core::{ModulePath, Program};

/// Where `kotlinc` puts the compiled classes inside the target directory
/// [kt-run]. Dot-prefixed so a target nested in the source tree stays
/// invisible to source discovery [mod-ignore].
const CLASSES_DIR: &str = ".salvo_classes";

/// The name Kotlin gives a file's top-level facade class: the file name with
/// its first letter capitalized and `Kt` appended (`other.kt` ->
/// `OtherKt`). A module's emitted file is named after its last segment.
fn facade_class(module: &ModulePath) -> String {
    let stem = module.0.last().map(String::as_str).unwrap_or("main");
    let mut chars = stem.chars();
    let capitalized = match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    };
    format!("{capitalized}Kt")
}

pub struct KotlinBackend;

impl Backend for KotlinBackend {
    fn name(&self) -> &'static str {
        "kotlin"
    }

    fn file_extension(&self) -> &'static str {
        "kt"
    }

    /// Kotlin compiles a top-level `main` in every module that declares
    /// one, so the entry selection changes only the launch class
    /// ([`Backend::entry_hint`]) and never the emitted code.
    fn emit(
        &self,
        program: &Program,
        target_dir: &Path,
        _entry: Option<&ModulePath>,
    ) -> Result<Vec<PathBuf>, BackendError> {
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

    /// The JVM entry class. Kotlin names a file's facade class after the
    /// **file**, not the function: module `other` is emitted as `other.kt`
    /// in package `salvo.other`, so its top-level `main` lands in
    /// `salvo.other.OtherKt`. (Before `salvo run` exercised it, this hint
    /// read `…MainKt` unconditionally — correct only because every entry
    /// file so far was `main.sv`.)
    fn entry_hint(&self, _target_dir: &Path, main_module: &ModulePath) -> String {
        format!("salvo.{main_module}.{}", facade_class(main_module))
    }

    /// [kt-run] `kotlinc` every emitted `.kt` file into a classes directory
    /// inside the target, then `kotlin -cp` that directory with the entry
    /// class. Companion files [backend-companion] are `.kt` too, so they
    /// are part of `emitted` and compile with the rest.
    fn run(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> Result<i32, BackendError> {
        use std::ffi::OsStr;

        let classes = target_dir.join(CLASSES_DIR);
        let sources: Vec<PathBuf> = emitted
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "kt"))
            .map(|p| target_dir.join(p))
            .collect();
        if sources.is_empty() {
            return Err(BackendError::Other(
                "nothing to run: no Kotlin sources were emitted".to_string(),
            ));
        }

        let mut args: Vec<&OsStr> = sources.iter().map(|p| p.as_os_str()).collect();
        args.push(OsStr::new("-d"));
        args.push(classes.as_os_str());
        let code = run_tool("kotlinc", &args)?;
        if code != 0 {
            return Err(BackendError::Other(format!(
                "kotlinc failed with exit code {code}"
            )));
        }

        let entry = self.entry_hint(target_dir, main_module);
        run_tool(
            "kotlin",
            &[OsStr::new("-cp"), classes.as_os_str(), OsStr::new(&entry)],
        )
    }
}
