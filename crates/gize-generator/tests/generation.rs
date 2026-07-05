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

use gize_core::ModelSpec;
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
        &scaffold::new_project("shop", true, TS),
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
    let plan = scaffold::new_project("shop", true, TS);
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
    let plan = scaffold::new_project("shop", false, TS);
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
fn make_crud_lands_resource_with_declared_fields() {
    let root = unique_tmpdir();
    apply(
        &root,
        &scaffold::new_project("shop", false, TS),
        Options::default(),
    );
    apply(
        &root,
        &scaffold::make_crud(&product_model(), TS),
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
    let plan = scaffold::new_project("shop", false, TS);
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
    let plan = scaffold::make_crud(&product_model(), TS);
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
