//! [manifest] The project manifest, `salvo.toml`: what a project is — its
//! source root, entry point, default backend and output directories — so the
//! CLI and the language server agree, and so an editor opened on a directory
//! holding several projects analyses each as its own program (user decisions
//! 2026-09-24 for the direction, 2026-09-29 for the shape; ROADMAP §4).
//!
//! ```toml
//! [project]
//! name = "collections"
//! version = "0.1.0"
//!
//! [build]
//! src = "salvo"          # the source root, relative to this file (default ".")
//! main = "salvo/main.sv" # the entry point, when there are several (optional)
//! backend = "*"          # the default backend: "rust", "kotlin", or "*" for all
//! target = "out"         # the output directory, when one serves every backend
//!
//! [rust]
//! target = "rust"        # per-backend output directory
//!
//! [kotlin]
//! target = "kotlin"
//! ```
//!
//! [manifest-discovery] A file belongs to the **nearest ancestor** directory
//! holding a `salvo.toml`; a nested manifest is a boundary, so the parent's
//! tree does not include it. With no manifest above a file, the CLI needs
//! `--src` as before and the language server falls back to the workspace root
//! — and says so at the file. Precedence everywhere: a CLI flag, then the
//! manifest, then the built-in default.
//!
//! `[project] std = true` marks the standard library's own tree: its files are
//! std to the checker (`intrinsic` allowed) and shadow the embedded copy
//! [std-shadow], which is what makes the repository root openable.

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const MANIFEST_FILE: &str = "salvo.toml";
/// [protocol-lock] The lock file beside the manifest.
pub const LOCK_FILE: &str = "salvo.lock";

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub build: BuildSection,
    #[serde(default)]
    pub rust: BackendSection,
    #[serde(default)]
    pub kotlin: BackendSection,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    pub name: Option<String>,
    pub version: Option<String>,
    /// The standard library's own tree [std-shadow].
    #[serde(default)]
    pub std: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BuildSection {
    /// The source root, relative to the manifest's directory. Default `.`.
    pub src: Option<String>,
    /// The entry file, relative to the manifest's directory.
    pub main: Option<String>,
    /// `rust`, `kotlin`, or `*` for every backend.
    pub backend: Option<String>,
    /// One output directory for every backend, relative to the manifest's
    /// directory; a `[backend]` section's `target` overrides it.
    pub target: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BackendSection {
    pub target: Option<String>,
}

/// A manifest found on disk, with where it was found.
#[derive(Debug, Clone)]
pub struct Project {
    /// The directory holding `salvo.toml`.
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Project {
    /// [manifest-discovery] The nearest manifest at or above `start` (a file
    /// or a directory). `Err` is a manifest that exists and does not parse.
    pub fn find(start: &Path) -> Result<Option<Project>, String> {
        let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
        let mut dir: Option<&Path> = if start.is_dir() { Some(&start) } else { start.parent() };
        while let Some(d) = dir {
            let candidate = d.join(MANIFEST_FILE);
            if candidate.is_file() {
                return Project::load(&candidate).map(Some);
            }
            dir = d.parent();
        }
        Ok(None)
    }

    /// Reads and parses one manifest file.
    pub fn load(path: &Path) -> Result<Project, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?;
        let manifest: Manifest = toml::from_str(&text).map_err(|e| {
            format!("`{}`: {} [manifest]", path.display(), e.message())
        })?;
        if let Some(b) = &manifest.build.backend {
            if !matches!(b.as_str(), "rust" | "kotlin" | "*") {
                return Err(format!(
                    "`{}`: `[build] backend` is `{b}`; it names a backend (`rust`, `kotlin`) or \
                     `*` for every backend [manifest]",
                    path.display()
                ));
            }
        }
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        Ok(Project { dir, manifest })
    }

    /// The source root: `[build] src`, or the manifest's directory.
    pub fn src(&self) -> PathBuf {
        match &self.manifest.build.src {
            Some(s) => self.dir.join(s),
            None => self.dir.clone(),
        }
    }

    pub fn main(&self) -> Option<PathBuf> {
        self.manifest.build.main.as_ref().map(|m| self.dir.join(m))
    }

    /// The backends `[build] backend` names: one, or both for `*`. `None`
    /// when the manifest is silent.
    pub fn backends(&self) -> Option<Vec<String>> {
        match self.manifest.build.backend.as_deref() {
            None => None,
            Some("*") => Some(vec!["rust".to_string(), "kotlin".to_string()]),
            Some(b) => Some(vec![b.to_string()]),
        }
    }

    /// The output directory for `backend`: its own section's `target`, else
    /// `[build] target`, else `None`.
    pub fn target(&self, backend: &str) -> Option<PathBuf> {
        let own = match backend {
            "rust" => self.manifest.rust.target.as_ref(),
            "kotlin" => self.manifest.kotlin.target.as_ref(),
            _ => None,
        };
        own.or(self.manifest.build.target.as_ref())
            .map(|t| self.dir.join(t))
    }

    pub fn is_std(&self) -> bool {
        self.manifest.project.std
    }

    pub fn lock_path(&self) -> PathBuf {
        self.dir.join(LOCK_FILE)
    }
}

/// [manifest-discovery] Whether `dir` is a **nested project's** root relative
/// to `root`: it holds a manifest and is not `root` itself. A source walk stops
/// at it.
pub fn is_nested_project(root: &Path, dir: &Path) -> bool {
    dir != root && dir.join(MANIFEST_FILE).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_parses_and_resolves_its_paths() {
        let text = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nbackend = \"*\"\n\n[rust]\ntarget = \"rust\"\n";
        let manifest: Manifest = toml::from_str(text).unwrap();
        let p = Project {
            dir: PathBuf::from("/x"),
            manifest,
        };
        assert_eq!(p.src(), PathBuf::from("/x/salvo"));
        assert_eq!(p.backends(), Some(vec!["rust".to_string(), "kotlin".to_string()]));
        assert_eq!(p.target("rust"), Some(PathBuf::from("/x/rust")));
        assert_eq!(p.target("kotlin"), None);
        assert!(!p.is_std());
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let text = "[build]\nsource = \"salvo\"\n";
        assert!(toml::from_str::<Manifest>(text).is_err());
    }
}
