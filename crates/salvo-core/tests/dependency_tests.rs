//! [manifest-deps] Project dependencies: found by name under `[build]
//! modules`, at the version declared; loaded as ordinary source trees whose
//! std-ness follows their own manifest. And [doc-module]: the checker
//! records where an `@module` selector resolved, for the module hover.

use std::fs;
use std::path::{Path, PathBuf};

use salvo_core::{check_program, resolve, ModulePath, Program, Project, SourceSet, Symbols};

fn dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("dependency_{test}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a dependency project `<root>/salvo_modules/<name>/` with one file
/// `salvo/<file>` and the given manifest extras (`std = true`, say).
fn write_dependency(root: &Path, name: &str, project_extra: &str, file: &str, source: &str) {
    let dep = root.join("salvo_modules").join(name);
    fs::create_dir_all(dep.join("salvo").join(Path::new(file).parent().unwrap())).unwrap();
    fs::write(
        dep.join("salvo.toml"),
        format!(
            "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n{project_extra}\n[build]\nsrc = \"salvo\"\n"
        ),
    )
    .unwrap();
    fs::write(dep.join("salvo").join(file), source).unwrap();
}

/// A project manifest declaring `deps` under `salvo_modules`.
fn write_project(root: &Path, deps: &[&str]) {
    let mut text = String::from("[build]\nmodules = \"salvo_modules\"\n\n[dependencies]\n");
    for d in deps {
        text.push_str(&format!("{d} = \"0.1.0\"\n"));
    }
    fs::write(root.join("salvo.toml"), text).unwrap();
}

// [manifest-deps] A dependency is found under `[build] modules` by name, at
// the version declared; a manifest naming dependencies with nowhere to find
// them is refused at load.
#[test]
fn dependencies_are_found_by_name_under_the_modules_directory() {
    let root = dir("manifest");
    write_dependency(&root, "aws", "", "aws.sv", "export struct P { path: Str }\n");
    write_project(&root, &["aws"]);
    let project = Project::load(&root.join("salvo.toml")).unwrap();
    let deps = project.dependencies().unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "aws");
    assert_eq!(deps[0].project.src(), root.join("salvo_modules/aws/salvo"));
    assert!(!deps[0].project.is_std());

    // The declared version is checked against the dependency's own.
    fs::write(
        root.join("salvo.toml"),
        "[build]\nmodules = \"salvo_modules\"\n\n[dependencies]\naws = \"0.2.0\"\n",
    )
    .unwrap();
    let err = Project::load(&root.join("salvo.toml")).unwrap().dependencies().unwrap_err();
    assert!(err.contains("`0.2.0`") && err.contains("`0.1.0`"), "{err}");

    // A declared dependency with no directory is an error naming the path.
    write_project(&root, &["gcp"]);
    let err = Project::load(&root.join("salvo.toml")).unwrap().dependencies().unwrap_err();
    assert!(err.contains("gcp/salvo.toml") && err.contains("does not exist"), "{err}");

    // A directory under `modules` that no entry names is simply not loaded.
    write_project(&root, &[]);
    assert!(Project::load(&root.join("salvo.toml")).unwrap().dependencies().unwrap().is_empty());

    // `[dependencies]` without `[build] modules` is refused at load.
    fs::write(root.join("salvo.toml"), "[dependencies]\naws = \"0.1.0\"\n").unwrap();
    let err = Project::load(&root.join("salvo.toml")).unwrap_err();
    assert!(err.contains("[build] modules"), "{err}");
}

/// A `SourceSet` holding one fake embedded std module, `core.list`.
fn with_embedded_std() -> SourceSet {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/list.sv",
        ModulePath::parse("core.list"),
        "export intrinsic type List<T>\n".to_string(),
        true,
    );
    sources
}

// [manifest-deps] A dependency's files are loaded like the project's own —
// module paths from its layout, no prefix — named by absolute path, tagged
// with the dependency's name, and not std.
#[test]
fn a_dependency_is_loaded_as_an_ordinary_tree_tagged_with_its_name() {
    let root = dir("load");
    write_dependency(&root, "aws", "", "aws/s3.sv", "export struct Bucket { name: Str }\n");
    fs::write(
        root.join("salvo_modules/aws/salvo/aws.sv"),
        "// The aws module.\n\nexport struct P { path: Str }\n",
    )
    .unwrap();
    // A dependency's test annexes are never loaded.
    fs::write(root.join("salvo_modules/aws/salvo/aws.test.sv"), "test \"t\" {}\n").unwrap();
    write_project(&root, &["aws"]);
    let project = Project::load(&root.join("salvo.toml")).unwrap();

    let mut sources = with_embedded_std();
    for dep in project.dependencies().unwrap() {
        let errors = sources.add_dependency(&dep, "");
        assert!(errors.is_empty(), "{errors:?}");
    }
    let mut modules: Vec<String> = sources
        .files
        .iter()
        .filter(|f| f.dependency.as_deref() == Some("aws"))
        .map(|f| f.module.to_string())
        .collect();
    modules.sort();
    assert_eq!(modules, ["aws", "aws.s3"]);
    let aws = sources.files.iter().find(|f| f.module.to_string() == "aws").unwrap();
    assert!(Path::new(&aws.name).is_absolute(), "{}", aws.name);
    assert!(!aws.is_std && !aws.is_test);
    assert!(sources.files.iter().all(|f| !f.is_test));
    // The embedded std is untouched.
    assert!(sources.files.iter().any(|f| f.module.to_string() == "core.list" && f.is_std));
}

// [manifest-deps] A dependency may not redefine the standard library on its
// users: a module the embedded std also declares is refused, naming the file —
// unless the dependency's own manifest says `std = true`, in which case it
// replaces the embedded copy like any std tree [std-shadow].
#[test]
fn a_dependency_declares_std_modules_only_when_its_manifest_says_std() {
    let root = dir("std");
    write_dependency(&root, "sneaky", "", "core/list.sv", "export fn size() -> Int { return 0 }\n");
    write_dependency(&root, "stdfork", "std = true\n", "core/list.sv", "export intrinsic type List<T>\n// fork\n");
    write_project(&root, &["sneaky", "stdfork"]);
    let project = Project::load(&root.join("salvo.toml")).unwrap();
    let deps = project.dependencies().unwrap();

    let mut sources = with_embedded_std();
    let errors = sources.add_dependency(&deps[0], "");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("`sneaky`") && errors[0].contains("`core.list`") && errors[0].contains("std = true"),
        "{}",
        errors[0]
    );

    let mut sources = with_embedded_std();
    let errors = sources.add_dependency(&deps[1], "");
    assert!(errors.is_empty(), "{errors:?}");
    let list: Vec<_> = sources.files.iter().filter(|f| f.module.to_string() == "core.list").collect();
    assert_eq!(list.len(), 1, "the embedded copy was replaced");
    assert!(list[0].is_std && list[0].is_shadow);
    assert_eq!(list[0].dependency.as_deref(), Some("stdfork"));
    assert!(list[0].content.contains("fork"));

    // `mark_std_tree` (a `std = true` *project*) leaves a dependency alone.
    let mut sources = with_embedded_std();
    write_dependency(&root, "plain", "", "plain.sv", "export struct P { path: Str }\n");
    write_project(&root, &["plain"]);
    let project = Project::load(&root.join("salvo.toml")).unwrap();
    for dep in project.dependencies().unwrap() {
        assert!(sources.add_dependency(&dep, "").is_empty());
    }
    sources.add("own.sv", ModulePath::parse("own"), String::new(), false);
    sources.mark_std_tree();
    let plain = sources.files.iter().find(|f| f.module.to_string() == "plain").unwrap();
    assert!(!plain.is_std);
    let own = sources.files.iter().find(|f| f.module.to_string() == "own").unwrap();
    assert!(own.is_std);
}

/// The base types the checker needs, as a std file [intrinsic-std-only].
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export intrinsic type Long\nexport intrinsic type Any\nexport intrinsic type Never\n";

// [doc-module] The checker records the module an `@module` selector resolved
// to, at the selector's span — the full path, whatever suffix was written
// [mod-suffix] — for a call and for a fn passed by name.
#[test]
fn module_selectors_are_recorded_with_their_resolved_module() {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        ModulePath::parse("core.prelude"),
        STD_PRELUDE.to_string(),
        true,
    );
    let shapes = "export fn label(n: Int) -> Str { return \"s\" }\n";
    let main = "import shop.shapes.label\n\
                fn label(n: Int) -> Str { return \"m\" }\n\
                fn apply(f: (Int) -> Str, n: Int) -> Str { return f(n) }\n\
                fn main() -> Str {\n    let a = label@shapes(1)\n    return apply(label@shop.shapes, 2)\n}\n";
    sources.add("shop/shapes.sv", ModulePath::parse("shop.shapes"), shapes.to_string(), false);
    sources.add("main.sv", ModulePath::parse("main"), main.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "{diagnostics:?}");
        modules.push(module);
    }
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let symbols = Symbols::collect(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let errors: Vec<_> = checked.errors.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");

    let main_idx = 2;
    let text = |span: salvo_syntax::Span| &main[span.start as usize..span.end as usize];
    let mut refs: Vec<(String, String)> = checked
        .module_refs
        .iter()
        .filter(|((file, _), _)| *file == main_idx)
        .map(|((_, span), module)| (text(*span).to_string(), module.to_string()))
        .collect();
    refs.sort();
    assert_eq!(
        refs,
        [
            ("shapes".to_string(), "shop.shapes".to_string()),
            ("shop.shapes".to_string(), "shop.shapes".to_string()),
        ]
    );
}
