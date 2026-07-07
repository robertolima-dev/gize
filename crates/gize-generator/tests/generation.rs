//! End-to-end integration and snapshot tests for the generator.
//!
//! The in-crate unit tests inspect `Plan`s; these go further and exercise the real
//! [`Writer`] against a temporary directory, then pin generated file contents with golden
//! snapshots. Together they cover the MVP Definition of Done item "all of the above is
//! covered by integration + snapshot tests in CI":
//!
//! * generation lands the expected tree on disk,
//! * re-running a generator is idempotent and never destroys hand edits (`--force` off),
//! * `--force` overwrites and `--dry-run` writes nothing,
//! * generated code does not drift from reviewed snapshots (template regression guard).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use gize_core::{Api, Dialect, ModelSpec};
use gize_generator::{Options, Plan, Report, Writer, scaffold};

/// Fixed, injected timestamp so migration filenames and contents are deterministic.
const TS: &str = "20260101000000";

/// A unique, empty temp directory. Dependency-free (no `tempfile`): scoped by pid, a
/// nanosecond clock and a process-local counter so concurrent tests never collide.
fn unique_tmpdir() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("gize-it-{}-{nanos}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn apply(root: &Path, plan: &Plan, opts: Options) -> Report {
    Writer::new(opts).apply(root, plan).expect("apply plan")
}

fn product_model() -> ModelSpec {
    ModelSpec::parse(
        "Product",
        &[
            "name:String".to_string(),
            "price:i32".to_string(),
            "active:bool".to_string(),
        ],
    )
    .unwrap()
}

// --------------------------------------------------------------------------------------
// Integration: real file I/O through the Writer
// --------------------------------------------------------------------------------------

#[test]
fn new_project_materializes_expected_tree_on_disk() {
    let root = unique_tmpdir();
    apply(
        &root,
        &scaffold::new_project("shop", true, false, Dialect::Postgres, None, TS),
        Options::default(),
    );

    for rel in [
        "Cargo.toml",
        "gize.toml",
        ".env.example",
        ".gitignore",
        "src/main.rs",
        "src/state.rs",
        "src/router.rs",
        "src/config/mod.rs",
        "src/app/mod.rs",
        "src/app/users/mod.rs",
        "src/app/users/model.rs",
        "src/app/users/handler.rs",
    ] {
        assert!(root.join(rel).is_file(), "expected generated file `{rel}`");
    }
    assert!(
        root.join(format!("migrations/{TS}_create_users.sql"))
            .is_file(),
        "expected the users migration"
    );

    // Reserved layout directories (ADR-005) exist even when empty.
    for dir in ["src/database", "src/middleware", "src/shared", "migrations"] {
        assert!(root.join(dir).is_dir(), "expected directory `{dir}`");
    }
}

#[test]
fn regenerating_is_idempotent_and_preserves_hand_edits() {
    let root = unique_tmpdir();
    let plan = scaffold::new_project("shop", true, false, Dialect::Postgres, None, TS);
    apply(&root, &plan, Options::default());

    // Simulate a developer editing a generated file.
    let edited = root.join("src/app/users/service.rs");
    let marker = "// hand-written business logic — must survive re-generation\n";
    let original = fs::read_to_string(&edited).unwrap();
    fs::write(&edited, format!("{marker}{original}")).unwrap();

    // Re-run the same generator without --force.
    let report = apply(&root, &plan, Options::default());
    assert!(report.created.is_empty(), "re-run should create nothing");
    assert!(
        report.overwritten.is_empty(),
        "re-run without --force must overwrite nothing"
    );
    assert!(
        !report.skipped.is_empty(),
        "existing files should be skipped"
    );

    // The hand edit is intact.
    assert!(
        fs::read_to_string(&edited).unwrap().starts_with(marker),
        "the hand edit must survive re-generation"
    );
}

#[test]
fn force_overwrites_and_dry_run_writes_nothing() {
    let root = unique_tmpdir();
    let plan = scaffold::new_project("shop", false, false, Dialect::Postgres, None, TS);
    apply(&root, &plan, Options::default());

    let sentinel = root.join("Cargo.toml");
    let generated = fs::read_to_string(&sentinel).unwrap();
    fs::write(&sentinel, "TOUCHED").unwrap();

    // --force + --dry-run reports an overwrite but must not touch the file.
    let report = apply(
        &root,
        &plan,
        Options {
            force: true,
            dry_run: true,
        },
    );
    assert!(
        !report.overwritten.is_empty(),
        "dry-run should still report overwrites"
    );
    assert_eq!(
        fs::read_to_string(&sentinel).unwrap(),
        "TOUCHED",
        "dry-run must not write to disk"
    );

    // --force for real restores the generated content.
    apply(
        &root,
        &plan,
        Options {
            force: true,
            dry_run: false,
        },
    );
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), generated);
}

#[test]
fn api_versioning_nests_routes_and_records_the_prefix() {
    // A versioned project nests the app under `/api/v1` and records it in gize.toml (ADR-016).
    let versioned = unique_tmpdir();
    apply(
        &versioned,
        &scaffold::new_project(
            "shop",
            true,
            false,
            Dialect::Postgres,
            Some(Api::from_version("1")),
            TS,
        ),
        Options::default(),
    );
    let router = fs::read_to_string(versioned.join("src/router.rs")).unwrap();
    assert!(
        router.contains(".nest(\"/api/v1\", app::routes())"),
        "versioned router should nest under the prefix: {router}"
    );
    let manifest = fs::read_to_string(versioned.join("gize.toml")).unwrap();
    assert!(manifest.contains("[api]"));
    assert!(manifest.contains("version = \"v1\""));

    // An unversioned project is unchanged: it merges at the root and writes no `[api]` table.
    let plain = unique_tmpdir();
    apply(
        &plain,
        &scaffold::new_project("shop", true, false, Dialect::Postgres, None, TS),
        Options::default(),
    );
    let router = fs::read_to_string(plain.join("src/router.rs")).unwrap();
    assert!(router.contains(".merge(app::routes())"));
    assert!(!router.contains(".nest("));
    assert!(
        !fs::read_to_string(plain.join("gize.toml"))
            .unwrap()
            .contains("[api]")
    );
}

#[test]
fn mysql_project_uses_binary_uuid_and_no_returning() {
    // MySQL has no RETURNING and no UUID generator, so the repository writes then re-reads by
    // the app-generated id, and the schema uses BINARY(16) / VARCHAR(255) (ADR-015 amendment).
    let root = unique_tmpdir();
    apply(
        &root,
        &scaffold::new_project("shop", true, false, Dialect::MySql, None, TS),
        Options::default(),
    );

    let migration =
        fs::read_to_string(root.join(format!("migrations/{TS}_create_users.sql"))).unwrap();
    assert!(migration.contains("id BINARY(16) PRIMARY KEY"));
    assert!(migration.contains("email VARCHAR(255) NOT NULL UNIQUE"));
    assert!(migration.contains("created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP"));

    let repo = fs::read_to_string(root.join("src/app/users/repository.rs")).unwrap();
    assert!(!repo.contains("RETURNING"), "MySQL must not use RETURNING");
    assert!(repo.contains("MySqlPool"));
    // create/update re-read the row via find() after writing.
    assert_eq!(repo.matches("find(pool, id).await").count(), 2);

    let error = fs::read_to_string(root.join("src/app/users/error.rs")).unwrap();
    assert!(error.contains("MySqlDatabaseError"));
    assert!(error.contains("1062 => return Error::Conflict"));

    let state = fs::read_to_string(root.join("src/state.rs")).unwrap();
    assert!(state.contains("MySqlPool"));
    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("\"mysql\""));
    assert!(
        fs::read_to_string(root.join("gize.toml"))
            .unwrap()
            .contains("database = \"mysql\"")
    );
}

#[test]
fn make_crud_lands_resource_with_declared_fields() {
    let root = unique_tmpdir();
    apply(
        &root,
        &scaffold::new_project("shop", false, false, Dialect::Postgres, None, TS),
        Options::default(),
    );
    apply(
        &root,
        &scaffold::make_crud(&product_model(), Dialect::Postgres, TS),
        Options::default(),
    );

    let model_rs = fs::read_to_string(root.join("src/app/products/model.rs")).unwrap();
    assert!(model_rs.contains("pub struct Product"));
    assert!(model_rs.contains("pub price: i32"));
    assert!(model_rs.contains("pub active: bool"));

    let migration =
        fs::read_to_string(root.join(format!("migrations/{TS}_create_products.sql"))).unwrap();
    assert!(migration.contains("CREATE TABLE products"));
    assert!(migration.contains("price INTEGER NOT NULL"));
}

// --------------------------------------------------------------------------------------
// Snapshots: pin generated code so template edits are always reviewed
// --------------------------------------------------------------------------------------

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

fn slug(rel: &str) -> String {
    rel.replace(['/', '.', ' '], "_")
}

/// Compare `actual` to the committed golden file `tests/snapshots/{name}.snap`.
///
/// Run `UPDATE_SNAPSHOTS=1 cargo test` after an *intentional* template change to rewrite
/// the snapshots, then review the diff before committing. Missing snapshots are written on
/// first run rather than failing, so bootstrapping is a single test run.
fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshots_dir().join(format!("{name}.snap"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() || !path.exists() {
        fs::create_dir_all(snapshots_dir()).unwrap();
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {name}.snap: {e}"));
    assert_eq!(
        actual, expected,
        "generated output for `{name}` drifted from its snapshot. \
         If intentional, re-run with UPDATE_SNAPSHOTS=1 and review the diff."
    );
}

fn content<'a>(plan: &'a Plan, rel: &str) -> &'a str {
    plan.ops
        .iter()
        .find(|o| o.path.to_string_lossy() == rel)
        .map(|o| o.contents.as_str())
        .unwrap_or_else(|| panic!("plan has no file `{rel}`"))
}

#[test]
fn project_skeleton_matches_snapshots() {
    // No built-in users here, so the skeleton templates are isolated from the CRUD ones.
    let plan = scaffold::new_project("shop", false, false, Dialect::Postgres, None, TS);
    for rel in [
        "Cargo.toml",
        "gize.toml",
        "src/main.rs",
        "src/router.rs",
        "src/state.rs",
        "src/config/mod.rs",
        "src/app/mod.rs",
    ] {
        assert_snapshot(&format!("project__{}", slug(rel)), content(&plan, rel));
    }
}

#[test]
fn crud_slice_matches_snapshots() {
    let plan = scaffold::make_crud(&product_model(), Dialect::Postgres, TS);
    for rel in [
        "src/app/products/mod.rs",
        "src/app/products/model.rs",
        "src/app/products/dto.rs",
        "src/app/products/error.rs",
        "src/app/products/repository.rs",
        "src/app/products/service.rs",
        "src/app/products/handler.rs",
        "src/app/products/routes.rs",
        "src/app/products/tests.rs",
    ] {
        let leaf = Path::new(rel).file_name().unwrap().to_string_lossy();
        assert_snapshot(&format!("crud__{}", slug(&leaf)), content(&plan, rel));
    }
    assert_snapshot(
        "crud__create_products_sql",
        content(&plan, &format!("migrations/{TS}_create_products.sql")),
    );
}

// --------------------------------------------------------------------------------------
// Integration: `gize sync` reconciliation from the manifest (ADR-009)
// --------------------------------------------------------------------------------------

/// The desired code plan for every module in a project's `gize.toml`, mirroring what the
/// `gize sync` command builds (code files only; migrations are handled by the command).
fn desired_code_plan(root: &Path) -> Plan {
    let text = fs::read_to_string(root.join("gize.toml")).expect("read gize.toml");
    let manifest = gize_core::Manifest::from_toml(&text).expect("parse manifest");
    let mut plan = Plan::new();
    for module in &manifest.modules {
        plan = plan
            .extend(scaffold::module_code(module, Dialect::Postgres).expect("plan module code"));
    }
    plan
}

#[test]
fn sync_rebuilds_a_deleted_module_from_the_manifest() {
    let root = unique_tmpdir();
    // A project with the built-in users plus a Product CRUD, both recorded in gize.toml.
    apply(
        &root,
        &scaffold::new_project("shop", true, false, Dialect::Postgres, None, TS),
        Options::default(),
    );
    apply(
        &root,
        &scaffold::make_crud(&product_model(), Dialect::Postgres, TS),
        Options::default(),
    );
    // make_crud records the module shape the way the CLI does.
    record_products_in_manifest(&root);

    // Simulate a checkout that has the manifest but lost the module's code.
    fs::remove_dir_all(root.join("src/app/products")).unwrap();
    assert!(!root.join("src/app/products/model.rs").exists());

    // Reconcile from the manifest.
    let plan = desired_code_plan(&root);
    let recon = gize_generator::sync::reconcile(&root, &plan).unwrap();
    // users (9 files) are untouched; products (9 files) are missing.
    assert_eq!(recon.unchanged.len(), 9, "users slice should already match");
    assert_eq!(recon.missing.len(), 9, "products slice should be missing");
    assert!(recon.drift.is_empty(), "nothing should have drifted");

    let applied = gize_generator::sync::apply(&root, &recon, false, false).unwrap();
    assert_eq!(applied.created.len(), 9);
    assert!(root.join("src/app/products/model.rs").is_file());

    // A second reconcile is a no-op: the tree matches the manifest exactly.
    let recon2 = gize_generator::sync::reconcile(&root, &desired_code_plan(&root)).unwrap();
    assert!(recon2.is_in_sync(), "re-running sync should be idempotent");
    assert_eq!(recon2.unchanged.len(), 18);
}

#[test]
fn sync_reports_drift_and_preserves_hand_edits_without_force() {
    let root = unique_tmpdir();
    apply(
        &root,
        &scaffold::new_project("shop", true, false, Dialect::Postgres, None, TS),
        Options::default(),
    );

    // A hand edit to a generated file.
    let edited = root.join("src/app/users/service.rs");
    let original = fs::read_to_string(&edited).unwrap();
    fs::write(&edited, format!("{original}\n// hand edit\n")).unwrap();

    let recon = gize_generator::sync::reconcile(&root, &desired_code_plan(&root)).unwrap();
    assert_eq!(
        recon.drift.len(),
        1,
        "the edited file should be flagged as drift"
    );

    // Without force, drift is left untouched.
    let applied = gize_generator::sync::apply(&root, &recon, false, false).unwrap();
    assert_eq!(applied.left.len(), 1);
    assert!(
        fs::read_to_string(&edited)
            .unwrap()
            .contains("// hand edit")
    );

    // With force, it is overwritten back to the manifest's version.
    let recon = gize_generator::sync::reconcile(&root, &desired_code_plan(&root)).unwrap();
    gize_generator::sync::apply(&root, &recon, true, false).unwrap();
    assert!(
        !fs::read_to_string(&edited)
            .unwrap()
            .contains("// hand edit")
    );
}

/// Record the Product module's shape in the project manifest, as `gize make crud` does.
fn record_products_in_manifest(root: &Path) {
    let path = root.join("gize.toml");
    let mut manifest = gize_core::Manifest::from_toml(&fs::read_to_string(&path).unwrap()).unwrap();
    manifest.upsert_module(gize_core::Module {
        name: "products".to_string(),
        fields: product_model().to_field_tokens(),
        belongs_to: Vec::new(),
    });
    fs::write(&path, manifest.to_toml().unwrap()).unwrap();
}

#[test]
fn auth_and_users_slice_match_snapshots() {
    // The security-sensitive generated code (auth module + the users slice that hashes
    // passwords and issues tokens) is pinned so template edits are always reviewed.
    let plan = scaffold::new_project("shop", true, false, Dialect::Postgres, None, TS);
    for rel in [
        "src/auth/mod.rs",
        "src/app/users/handler.rs",
        "src/app/users/routes.rs",
        "src/app/users/dto.rs",
        "src/app/users/error.rs",
    ] {
        assert_snapshot(&format!("auth__{}", slug(rel)), content(&plan, rel));
    }
}
