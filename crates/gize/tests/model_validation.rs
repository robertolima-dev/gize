use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Project(PathBuf);
impl Project {
    fn new(label: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("gize-validation-{}-{label}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_gize"))
            .current_dir(&root)
            .args(["new", "app", "--no-user", "--openapi"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Self(root)
    }
    fn root(&self) -> PathBuf {
        self.0.join("app")
    }
    fn reject_without_changes(&self, args: &[&str], expected: &str) {
        let before = tree(&self.0);
        let result = Command::new(env!("CARGO_BIN_EXE_gize"))
            .current_dir(self.root())
            .args(args)
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "accepted {args:?}");
        assert!(error.contains(expected), "{args:?}: {error}");
        assert_eq!(tree(&self.0), before, "partial writes for {args:?}");
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(key, None);
                visit(root, &path, out);
            } else {
                out.insert(key, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

#[test]
fn invalid_cli_models_never_write_even_with_force() {
    let project = Project::new("cli");
    for command in ["model", "crud"] {
        for fields in [
            vec!["id:String"],
            vec!["created_at:DateTime"],
            vec!["updated_at:DateTime"],
            vec!["type:String"],
            vec!["name:String", "name:i32"],
            vec!["author_id:Uuid", "author:belongs_to:users"],
            vec!["author:belongs_to:users", "author_id:Uuid"],
            vec!["author:belongs_to:../users"],
            vec!["first-name:String"],
        ] {
            for flag in [None, Some("--force"), Some("--dry-run")] {
                let mut args = vec!["make", command, "Post"];
                args.extend(fields.iter().copied());
                if let Some(flag) = flag {
                    args.push(flag);
                }
                project.reject_without_changes(&args, "invalid model definition");
            }
        }
        project.reject_without_changes(
            &["make", command, "../Escape", "name:String"],
            "invalid model",
        );
    }
    project.reject_without_changes(&["make", "app", "../Escape", "--force"], "invalid module");
}

#[test]
fn invalid_late_manifest_module_prevents_all_generation() {
    let project = Project::new("manifest");
    let path = project.root().join("gize.toml");
    let original = fs::read_to_string(&path).unwrap();
    for invalid in [
        "fields = ['id:String']",
        "fields = ['type:String']",
        "fields = ['name:String','name:i32']",
        "fields = ['author_id:Uuid']\n[[module.belongs_to]]\nfield='author'\ntarget='users'",
    ] {
        fs::write(&path, format!("{original}\n[[module]]\nname='aaa'\nfields=['name:String']\n[[module]]\nname='zzz'\n{invalid}\n")).unwrap();
        for args in [
            vec!["sync", "--force"],
            vec!["sync", "--dry-run"],
            vec!["make", "crud", "Product", "name:String", "--force"],
            vec!["make", "model", "Product", "name:String"],
            vec!["make", "app", "health"],
            vec!["make", "admin"],
            vec!["make", "migration"],
        ] {
            project.reject_without_changes(&args, "module `zzz`");
        }
    }
}

#[test]
fn keyword_checks_follow_database_and_code_only_apps_are_allowed() {
    let project = Project::new("dialect");
    let path = project.root().join("gize.toml");
    let original = fs::read_to_string(&path).unwrap();
    for (database, field, accepted) in [
        ("postgres", "key", true),
        ("postgres", "index", true),
        ("postgres", "database", true),
        ("postgres", "values", true),
        ("sqlite", "key", true),
        ("sqlite", "database", true),
        ("sqlite", "index", false),
        ("sqlite", "values", false),
        ("mysql", "key", false),
        ("mysql", "index", false),
        ("mysql", "database", false),
        ("postgres", "select", false),
        ("postgres", "array", false),
        ("mysql", "range", false),
        ("mysql", "rank", false),
        ("sqlite", "user", true),
    ] {
        fs::write(
            &path,
            original.replace(
                "database = \"postgres\"",
                &format!("database = \"{database}\""),
            ),
        )
        .unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_gize"))
            .current_dir(project.root())
            .args([
                "make",
                "model",
                "Post",
                &format!("{field}:String"),
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert_eq!(
            result.status.success(),
            accepted,
            "{database}.{field}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fs::write(&path, original).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_gize"))
        .current_dir(project.root())
        .args(["make", "app", "user"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(project.root().join("src/app/user/mod.rs").exists());
    let before = tree(&project.root().join("migrations"));
    let result = Command::new(env!("CARGO_BIN_EXE_gize"))
        .current_dir(project.root())
        .args(["sync", "--force"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(tree(&project.root().join("migrations")), before);
}

#[test]
fn existing_manifest_does_not_block_migration_execution_or_manual_escape_hatch() {
    let project = Project::new("existing");
    let path = project.root().join("gize.toml");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        format!("{original}\n[[module]]\nname='api_keys'\nfields=['key:String', 'type:String']\n"),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_gize"))
        .current_dir(project.root())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["migrate", "--status"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(env!("CARGO_BIN_EXE_gize"))
        .current_dir(project.root())
        .args(["make", "migration", "manual_fix"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    project.reject_without_changes(&["sync", "--force"], "module `api_keys`");
}

#[cfg(unix)]
#[test]
fn serve_and_check_reach_cargo_with_legacy_names() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::new("runtime");
    let path = project.root().join("gize.toml");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        format!("{original}\n[[module]]\nname='api_keys'\nfields=['key:String', 'type:String']\n"),
    )
    .unwrap();
    // Exercise the CLI dispatch without compiling or launching an application server.
    let bin = project.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let cargo = bin.join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > cargo-invocation\n",
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    for (args, expected) in [
        (vec!["check"], "clippy"),
        (vec!["serve", "--api-only"], "run"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_gize"))
            .current_dir(project.root())
            .env("PATH", &bin)
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            fs::read_to_string(project.root().join("cargo-invocation"))
                .unwrap()
                .starts_with(expected)
        );
    }
}
