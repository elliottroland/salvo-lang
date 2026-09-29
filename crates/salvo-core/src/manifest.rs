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
//!
//! [manifest-deps] **Dependencies** are other projects, found by name under
//! one directory (user decisions 2026-09-29):
//!
//! ```toml
//! [build]
//! modules = "salvo_modules"   # where dependencies live, relative to this file
//!
//! [dependencies]
//! aws = "0.1.0"               # `salvo_modules/aws/salvo.toml`, at that version
//! ```
//!
//! Every `[dependencies]` entry names a directory `<modules>/<name>/` holding
//! a manifest whose `[project] version` is the one declared; a missing
//! directory, a missing `modules` setting, or a different version is an
//! error naming the file. A directory under `modules` that no entry names is
//! ignored. A dependency's modules take their paths from its own layout,
//! exactly as std's and the project's do — there is no prefix — and it may
//! declare std modules only when its own manifest says `std = true`. Nothing
//! is transitive yet: a dependency's own `[dependencies]` are not followed.

use std::collections::BTreeMap;
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
    /// [manifest-deps] Dependency name → required version. Sorted, so the
    /// load order is stable.
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
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
    /// [manifest-deps] The directory holding the dependencies, relative to
    /// the manifest's directory. Required when `[dependencies]` is not empty.
    pub modules: Option<String>,
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

/// [manifest-deps] One dependency of a project: the name it is declared
/// under, and its own project.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub project: Project,
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
        // [manifest-deps] A dependency has to be found somewhere.
        if manifest.build.modules.is_none() {
            if let Some(name) = manifest.dependencies.keys().next() {
                return Err(format!(
                    "`{}`: `[dependencies]` names `{name}`, but nothing says where dependencies \
                     live — add `[build] modules = \"salvo_modules\"` (the directory holding \
                     `{name}/{MANIFEST_FILE}`) [manifest-deps]",
                    path.display()
                ));
            }
        }
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        Ok(Project { dir, manifest })
    }

    /// [manifest-deps] The directory dependencies are found under, when the
    /// manifest names one.
    pub fn modules_dir(&self) -> Option<PathBuf> {
        self.manifest.build.modules.as_ref().map(|m| self.dir.join(m))
    }

    /// [manifest-deps] The projects `[dependencies]` names, by name, each
    /// loaded from `<modules>/<name>/salvo.toml`. `Err` for an entry whose
    /// directory has no manifest, or whose manifest states another version
    /// than the one declared — the version is a claim about what is on disk,
    /// and a claim nothing checks is decoration.
    pub fn dependencies(&self) -> Result<Vec<Dependency>, String> {
        let mut out = Vec::new();
        if self.manifest.dependencies.is_empty() {
            return Ok(out);
        }
        let Some(modules) = self.modules_dir() else {
            // `load` refuses this shape; a hand-built manifest may still have it.
            return Err(format!(
                "`{}`: `[dependencies]` without `[build] modules` [manifest-deps]",
                self.dir.join(MANIFEST_FILE).display()
            ));
        };
        for (name, version) in &self.manifest.dependencies {
            let manifest_path = modules.join(name).join(MANIFEST_FILE);
            if !manifest_path.is_file() {
                return Err(format!(
                    "`{}`: dependency `{name}` is declared, but `{}` does not exist \
                     [manifest-deps]",
                    self.dir.join(MANIFEST_FILE).display(),
                    manifest_path.display()
                ));
            }
            let project = Project::load(&manifest_path)?;
            let found = project.manifest.project.version.as_deref().unwrap_or("(none)");
            if found != version {
                return Err(format!(
                    "`{}`: dependency `{name}` is declared at version `{version}`, but `{}` \
                     says `{found}` [manifest-deps]",
                    self.dir.join(MANIFEST_FILE).display(),
                    manifest_path.display()
                ));
            }
            out.push(Dependency {
                name: name.clone(),
                project,
            });
        }
        Ok(out)
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
