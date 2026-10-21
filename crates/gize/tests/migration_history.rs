use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Project(PathBuf);
impl Project {
    fn new(name: &str, dialect: &str, fields: &[&str], sql: &str) -> Self {
        let root = std::env::temp_dir().join(format!("gize-history-{}-{name}", std::process::id()));
        fs::create_dir_all(root.join("migrations")).unwrap();
        let project = Self(root);
        project.manifest(dialect, fields);
        fs::write(project.0.join("migrations/1_create.sql"), sql).unwrap();
        project
    }
    fn manifest(&self, dialect: &str, fields: &[&str]) {
        fs::write(self.0.join("gize.toml"), format!(
            "[project]\nname = \"blog\"\n[stack]\nframework = \"axum\"\ndatabase = \"{dialect}\"\norm = \"sqlx\"\n[[module]]\nname = \"posts\"\nfields = {fields:?}\n"
        )).unwrap();
    }
    fn run(&self, args: &[&str], success: bool) -> String {
        let out = Command::new(env!("CARGO_BIN_EXE_gize"))
            .current_dir(&self.0)
            .env(
                "DATABASE_URL",
                format!("sqlite://{}?mode=rwc", self.0.join("test.db").display()),
            )
            .args(args)
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.status.success(), success, "{args:?}: {text}");
        text
    }
    fn count(&self) -> usize {
        fs::read_dir(self.0.join("migrations")).unwrap().count()
    }
    fn next_migration(&self, sql: &str) {
        let version = fs::read_dir(self.0.join("migrations"))
            .unwrap()
            .map(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .split('_')
                    .next()
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
            })
            .max()
            .unwrap()
            + 1;
        fs::write(self.0.join(format!("migrations/{version}_manual.sql")), sql).unwrap();
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn drop_reintroduce_and_backfill_are_not_regenerated() {
    let project = Project::new(
        "cycle",
        "sqlite",
        &["title:String"],
        "CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, views INTEGER NOT NULL); INSERT INTO posts VALUES (1, 'existing', 3);",
    );
    project.run(&["make", "migration", "--force"], true);
    assert_eq!(project.count(), 2);
    assert!(
        project
            .run(&["make", "migration", "--force"], true)
            .contains("Schema matches")
    );
    assert_eq!(project.count(), 2);
    project.manifest("sqlite", &["title:String", "views:i64"]);
    project.run(&["make", "migration"], true);
    assert_eq!(project.count(), 3);
    let pending = project.run(&["make", "migration"], true);
    assert!(pending.contains("allows NULL"));
    assert!(!pending.contains("Schema matches"));
    assert_eq!(project.count(), 3);
    project.run(&["migrate"], true);
    project.next_migration("CREATE TABLE posts_new (id INTEGER PRIMARY KEY, title TEXT NOT NULL, views INTEGER NOT NULL); INSERT INTO posts_new SELECT id, title, COALESCE(views, 0) FROM posts; DROP TABLE posts; ALTER TABLE posts_new RENAME TO posts;");
    assert!(
        project
            .run(&["make", "migration"], true)
            .contains("Schema matches")
    );
    project.run(&["migrate"], true);
    assert_eq!(project.count(), 4);
}

#[test]
fn type_warnings_preserve_custom_sql_and_allow_independent_additions() {
    let project = Project::new(
        "custom",
        "postgres",
        &[
            "title:String",
            "published:DateTime",
            "price:f64",
            "active:bool",
        ],
        "CREATE EXTENSION IF NOT EXISTS pgcrypto; CREATE TABLE posts (id UUID PRIMARY KEY, title VARCHAR(120) NOT NULL, published TIMESTAMP NOT NULL, price NUMERIC(10,2) NOT NULL); CREATE TABLE tags (id UUID PRIMARY KEY, post_id UUID, code VARCHAR(40) NOT NULL); ALTER TABLE tags ADD CONSTRAINT fk FOREIGN KEY (post_id) REFERENCES posts(id);",
    );
    let initial = fs::read(project.0.join("migrations/1_create.sql")).unwrap();
    let mut manifest = fs::read_to_string(project.0.join("gize.toml")).unwrap();
    manifest
        .push_str("\n[[module]]\nname = \"tags\"\nfields = [\"post_id:Uuid\", \"code:String\", \"label:String\"]\n");
    fs::write(project.0.join("gize.toml"), manifest).unwrap();
    let preview = project.run(&["make", "migration", "--dry-run"], true);
    assert!(preview.contains("VARCHAR(120)"));
    assert!(preview.contains("TIMESTAMP"));
    assert!(preview.contains("NUMERIC(10,2)"));
    assert!(preview.contains("tags.label"));
    assert!(preview.contains("tags.code: existing SQL type VARCHAR(40)"));
    assert_eq!(project.count(), 1);
    project.run(&["make", "migration", "--force"], true);
    assert_eq!(project.count(), 3);
    assert_eq!(
        fs::read(project.0.join("migrations/1_create.sql")).unwrap(),
        initial
    );
    for entry in fs::read_dir(project.0.join("migrations")).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap() == "1_create.sql" {
            continue;
        }
        let sql = fs::read_to_string(path).unwrap();
        assert!(sql.contains("ADD COLUMN"));
        assert!(!sql.lines().any(|line| line.starts_with("ALTER TABLE")
            && line.split(';').next().unwrap().contains("ALTER COLUMN")));
        assert!(!sql.contains("TYPE VARCHAR"));
    }
    let repeated = project.run(&["make", "migration"], true);
    assert!(!repeated.contains("Schema matches"));
    assert_eq!(project.count(), 3);
}

#[test]
fn unsupported_column_effects_still_fail_before_any_write() {
    let project = Project::new(
        "unknown",
        "postgres",
        &["views:i32", "title:String"],
        "CREATE TABLE posts (views INTEGER NOT NULL);",
    );
    project.next_migration("ALTER TABLE posts ENABLE ROW LEVEL SECURITY;");
    let error = project.run(&["make", "migration", "--force"], false);
    assert!(error.contains("unsupported ALTER TABLE"));
    assert_eq!(project.count(), 2);
}

#[test]
fn make_app_modules_have_no_missing_schema_warning() {
    let project = Project::new("app", "postgres", &[], "-- no tables\n");
    let output = project.run(&["make", "migration"], true);
    assert!(!output.contains("table absent"));
    assert!(!output.contains("manual schema work"));
    assert!(output.contains("Schema matches"));
    assert_eq!(project.count(), 1);
    project.run(&["new", "health_project", "--no-user"], true);
    let generated = Project(project.0.join("health_project"));
    generated.run(&["make", "app", "health"], true);
    let output = generated.run(&["make", "migration"], true);
    assert!(!output.contains("table absent"));
    assert!(output.contains("Schema matches"));
}

#[test]
fn removing_last_explicit_field_is_detected() {
    let project = Project::new(
        "empty",
        "postgres",
        &[],
        "CREATE TABLE posts (id UUID PRIMARY KEY, views INTEGER NOT NULL);",
    );
    let held = project.run(&["make", "migration"], true);
    assert!(held.contains("drop(s) withheld"));
    assert!(!held.contains("Schema matches"));
    project.run(&["make", "migration", "--force"], true);
    assert_eq!(project.count(), 2);
    assert!(
        project
            .run(&["make", "migration"], true)
            .contains("Schema matches")
    );
}
