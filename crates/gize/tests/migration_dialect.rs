use std::fs;
use std::path::Path;
use std::process::Command;

fn run(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_gize"))
        .current_dir(root)
        .env(
            "DATABASE_URL",
            format!("sqlite://{}?mode=rwc", root.join("test.db").display()),
        )
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn migration_command_uses_manifest_dialect() {
    for (database, uuid_type, datetime_type) in [
        ("postgres", "UUID", "TIMESTAMPTZ"),
        ("sqlite", "TEXT", "TEXT"),
        ("mysql", "BINARY(16)", "DATETIME"),
    ] {
        let root = std::env::temp_dir().join(format!(
            "gize-migration-dialect-{}-{database}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("migrations")).unwrap();
        fs::write(root.join("gize.toml"), format!(r#"
[project]
name = "blog"
[stack]
framework = "axum"
database = "{database}"
orm = "sqlx"
[[module]]
name = "posts"
fields = ["title:String", "active:bool", "views:i32", "total:i64", "score:f64", "owner:Uuid", "published:DateTime"]
"#)).unwrap();
        fs::write(root.join("migrations/001_create_posts.sql"),
            "CREATE TABLE posts (\n  id INTEGER PRIMARY KEY,\n  title TEXT NOT NULL\n);\nINSERT INTO posts (id, title) VALUES (1, 'existing');\n").unwrap();
        if database == "mysql" {
            let path = root.join("migrations/001_create_posts.sql");
            let sql = fs::read_to_string(&path)
                .unwrap()
                .replace("title TEXT", "title VARCHAR(255)");
            fs::write(path, sql).unwrap();
        }
        let before = fs::read_to_string(root.join("gize.toml")).unwrap();
        let preview = run(&root, &["make", "migration", "--dry-run"]);
        assert!(preview.contains(&format!("posts.owner {uuid_type}")));
        assert!(preview.contains(&format!("posts.published {datetime_type}")));
        assert_eq!(fs::read_dir(root.join("migrations")).unwrap().count(), 1);
        run(&root, &["make", "migration"]);
        let migration = fs::read_dir(root.join("migrations"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with("_alter_posts.sql")
            })
            .unwrap();
        let sql = fs::read_to_string(migration).unwrap();
        assert!(sql.contains(&format!("ADD COLUMN owner {uuid_type};")));
        assert!(sql.contains(&format!("ADD COLUMN published {datetime_type};")));
        assert_eq!(fs::read_to_string(root.join("gize.toml")).unwrap(), before);
        run(&root, &["make", "migration"]);
        assert_eq!(fs::read_dir(root.join("migrations")).unwrap().count(), 2);
        if database == "sqlite" {
            // Apply every supported scalar type against a populated table. A second
            // migrate verifies that the SQLx migration history remains valid.
            run(&root, &["migrate"]);
            run(&root, &["migrate"]);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
