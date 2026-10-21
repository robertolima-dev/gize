//! Model-change migration diffing (ADR-011 revision).
//!
//! The checked-in migration SQL is the schema of record: we parse the columns a table
//! already has from its generated `.sql` files (never touching a live database), diff them
//! against the fields declared for the module in `gize.toml`, and emit the `ALTER TABLE`
//! needed to reconcile. Additive changes (new columns) are safe and generated automatically;
//! destructive ones (dropping a column) are gated behind `--force`, and a rename — which at
//! the column level is indistinguishable from a drop plus an add — is always surfaced for a
//! human to decide rather than inferred.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use gize_core::{Dialect, Field, ModelSpec};

/// Columns Gize always adds via the template, regardless of the model's declared fields.
/// They are never proposed as drops.
const IMPLICIT_COLUMNS: [&str; 3] = ["id", "created_at", "updated_at"];

/// The schema delta between a module's declared fields and its migrated columns.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SchemaDiff {
    /// Declared fields with no column yet — safe, additive `ADD COLUMN`.
    pub added: Vec<Field>,
    /// Columns present in the migrations but no longer declared — potential drops (or the
    /// old half of a rename). Reported; only emitted under `--force`.
    pub dropped: Vec<String>,
    /// Differences from generator defaults; preserve existing SQL and report warnings.
    pub changed: Vec<TypeChange>,
    /// Non-optional model fields whose SQL columns still allow NULL (backfill pending).
    pub nullable: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeChange {
    pub name: String,
    pub from: String,
    pub to: String,
}

impl SchemaDiff {
    /// No column additions, removals or type changes. Callers must also inspect
    /// `nullable` for pending backfill/NOT NULL work.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.dropped.is_empty() && self.changed.is_empty()
    }
}

/// Compute the diff for `model` (a module's declared shape) against the columns already
/// present in its table's generated migrations under `migrations_dir`.
///
/// Returns `Ok(None)` when the table does not exist after replay — there is nothing
/// to diff against, and the caller should scaffold the resource (`make crud` / `sync`) first.
pub fn diff_model(
    migrations_dir: &Path,
    table: &str,
    model: &ModelSpec,
    dialect: Dialect,
) -> Result<Option<SchemaDiff>> {
    Ok(SchemaHistory::load(migrations_dir, dialect)?.diff(table, model))
}

/// One immutable replay shared by every module in a CLI invocation.
pub struct SchemaHistory {
    tables: std::collections::BTreeMap<String, crate::schema::Table>,
    dialect: Dialect,
}

impl SchemaHistory {
    pub fn load(dir: &Path, dialect: Dialect) -> Result<Self> {
        Ok(Self {
            tables: crate::schema::replay(dir, dialect)?,
            dialect,
        })
    }

    pub fn diff(&self, table: &str, model: &ModelSpec) -> Option<SchemaDiff> {
        let known = self.tables.get(table)?;
        let dialect = self.dialect;

        let declared: BTreeSet<&str> = model.fields.iter().map(|f| f.name.as_str()).collect();

        let added = model
            .fields
            .iter()
            .filter(|f| !known.contains_key(&f.name))
            .cloned()
            .collect();

        let dropped = known
            .keys()
            .filter(|c| !declared.contains(c.as_str()))
            .filter(|c| !IMPLICIT_COLUMNS.contains(&c.as_str()))
            .cloned()
            .collect();

        let changed = model
            .fields
            .iter()
            .filter_map(|f| {
                let existing = known.get(&f.name)?;
                let desired = crate::schema::normalize_type(dialect.column_type(f.ty), dialect);
                (existing.ty != desired).then(|| TypeChange {
                    name: f.name.clone(),
                    from: existing.ty.clone(),
                    to: desired,
                })
            })
            .collect();
        let nullable = model
            .fields
            .iter()
            .filter(|f| known.get(&f.name).is_some_and(|c| c.nullable))
            .map(|f| f.name.clone())
            .collect();
        Some(SchemaDiff {
            added,
            dropped,
            changed,
            nullable,
        })
    }
}

/// The current columns after replaying forward migrations (PostgreSQL compatibility helper).
pub fn known_columns(migrations_dir: &Path, table: &str) -> Result<Option<BTreeSet<String>>> {
    known_columns_for_dialect(migrations_dir, table, Dialect::Postgres)
}

/// Replay columns using the selected dialect (including MySQL quoted identifiers).
pub fn known_columns_for_dialect(
    dir: &Path,
    table: &str,
    dialect: Dialect,
) -> Result<Option<BTreeSet<String>>> {
    Ok(SchemaHistory::load(dir, dialect)?
        .tables
        .remove(table)
        .map(|columns| columns.into_keys().collect()))
}

/// Render an `ALTER TABLE` migration that reconciles `table` to its declared shape.
///
/// Added columns are emitted **nullable** (not `NOT NULL`) even though Gize's create tables
/// are `NOT NULL`: adding a `NOT NULL` column without a default to a table that may hold rows
/// fails, so the safe move is a nullable column plus a `-- TODO` to backfill and tighten
/// (ADR-011 revision). Drops are only included when `force` is set. Type changes
/// are not inferred: callers must handle `changed` with a manual migration.
pub fn alter_sql(table: &str, diff: &SchemaDiff, dialect: Dialect, force: bool) -> String {
    let mut out = format!("-- Migration: alter {table} (reconcile from gize.toml)\n");
    out.push_str("-- Generated by `gize make migration` from a model change (see ADR-011).\n\n");

    for f in &diff.added {
        let ty = dialect.column_type(f.ty);
        let tighten = match dialect {
            Dialect::Postgres => format!("ALTER TABLE {table} ALTER COLUMN {} SET NOT NULL", f.name),
            Dialect::MySql => format!("ALTER TABLE {table} MODIFY COLUMN {} {ty} NOT NULL", f.name),
            Dialect::Sqlite => "rebuild the table with a NOT NULL constraint, preserving data, indexes, triggers and foreign keys".to_string(),
        };
        out.push_str(&format!(
            "ALTER TABLE {table} ADD COLUMN {name} {ty}; \
             -- TODO: nullable for safety — backfill, then {tighten}\n",
            name = f.name,
        ));
    }

    for col in &diff.dropped {
        if force {
            out.push_str(&format!("ALTER TABLE {table} DROP COLUMN {col};\n"));
        } else {
            out.push_str(&format!(
                "-- DROP of `{col}` withheld: re-run with --force to emit \
                 `ALTER TABLE {table} DROP COLUMN {col};` (may be a rename — review first)\n"
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmpdir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        // pid + a process-local counter make the path unique even when two tests run in the
        // same nanosecond on parallel threads.
        let dir = std::env::temp_dir().join(format!(
            "gize-diff-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn model(fields: &[&str]) -> ModelSpec {
        ModelSpec::parse(
            "Post",
            &fields.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
        .unwrap()
    }

    /// Whether the SQL contains an actual (non-comment) `DROP COLUMN` statement.
    fn has_executable_drop(sql: &str) -> bool {
        sql.lines().any(|l| {
            let l = l.trim_start();
            !l.starts_with("--") && l.contains("DROP COLUMN")
        })
    }

    #[test]
    fn alter_columns_use_selected_dialect_and_backfill_guidance() {
        let fields = model(&[
            "title:String",
            "active:bool",
            "views:i32",
            "total:i64",
            "score:f64",
            "owner:Uuid",
            "published:DateTime",
        ])
        .fields;
        for (dialect, types, guidance) in [
            (
                Dialect::Postgres,
                [
                    "TEXT",
                    "BOOLEAN",
                    "INTEGER",
                    "BIGINT",
                    "DOUBLE PRECISION",
                    "UUID",
                    "TIMESTAMPTZ",
                ],
                "ALTER TABLE posts ALTER COLUMN published SET NOT NULL",
            ),
            (
                Dialect::Sqlite,
                [
                    "TEXT", "INTEGER", "INTEGER", "INTEGER", "REAL", "TEXT", "TEXT",
                ],
                "rebuild the table with a NOT NULL constraint",
            ),
            (
                Dialect::MySql,
                [
                    "VARCHAR(255)",
                    "BOOLEAN",
                    "INT",
                    "BIGINT",
                    "DOUBLE",
                    "BINARY(16)",
                    "DATETIME",
                ],
                "ALTER TABLE posts MODIFY COLUMN published DATETIME NOT NULL",
            ),
        ] {
            let diff = SchemaDiff {
                added: fields.clone(),
                dropped: vec!["obsolete".into()],
                ..SchemaDiff::default()
            };
            let sql = alter_sql("posts", &diff, dialect, false);
            let statements: Vec<_> = sql
                .lines()
                .filter(|line| line.starts_with("ALTER TABLE"))
                .map(|line| line.split_once(';').unwrap().0)
                .collect();
            let expected: Vec<_> = fields
                .iter()
                .zip(types)
                .map(|(field, ty)| format!("ALTER TABLE posts ADD COLUMN {} {ty}", field.name))
                .collect();
            assert_eq!(statements, expected, "{dialect:?}");
            assert!(sql.contains(guidance), "{dialect:?}: {sql}");
            assert!(!has_executable_drop(&sql));
            assert!(has_executable_drop(&alter_sql(
                "posts", &diff, dialect, true
            )));
            if dialect == Dialect::Sqlite {
                assert!(!sql.contains("ALTER COLUMN"));
                assert!(!sql.contains("MODIFY COLUMN"));
            }
        }
    }

    #[test]
    fn no_create_migration_yields_none() {
        let dir = tmpdir();
        assert!(
            diff_model(&dir, "posts", &model(&["title:String"]), Dialect::Postgres)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn detects_added_column() {
        let dir = tmpdir();
        fs::write(
            dir.join("001_create_posts.sql"),
            "CREATE TABLE posts (\n  id UUID PRIMARY KEY,\n  title TEXT NOT NULL,\n  created_at TIMESTAMPTZ NOT NULL,\n  updated_at TIMESTAMPTZ NOT NULL\n);\n",
        )
        .unwrap();

        let diff = diff_model(
            &dir,
            "posts",
            &model(&["title:String", "views:i32"]),
            Dialect::Postgres,
        )
        .unwrap()
        .unwrap();
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].name, "views");
        assert!(diff.dropped.is_empty());

        let sql = alter_sql("posts", &diff, Dialect::Postgres, false);
        assert!(sql.contains("ALTER TABLE posts ADD COLUMN views INTEGER;"));
        assert!(sql.contains("SET NOT NULL"));
    }

    #[test]
    fn parses_columns_despite_parenthesized_defaults() {
        // Regression: `gen_random_uuid()` / `now()` contain parens; the create body must be
        // matched by the *balanced* closing paren, not the first `)`.
        let dir = tmpdir();
        fs::write(
            dir.join("001_create_posts.sql"),
            "-- Migration: create posts\nCREATE TABLE posts (\n    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n    title TEXT NOT NULL,\n    body TEXT NOT NULL,\n    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()\n);\n",
        )
        .unwrap();

        let known = known_columns(&dir, "posts").unwrap().unwrap();
        assert!(known.contains("title"), "title should be parsed");
        assert!(known.contains("body"), "body should be parsed");
        assert!(known.contains("updated_at"), "updated_at should be parsed");

        // A model matching the table exactly has no diff.
        let diff = diff_model(
            &dir,
            "posts",
            &model(&["title:String", "body:String"]),
            Dialect::Postgres,
        )
        .unwrap()
        .unwrap();
        assert!(
            diff.is_empty(),
            "matching model should produce no diff, got {diff:?}"
        );
    }

    #[test]
    fn already_migrated_alter_column_is_known() {
        let dir = tmpdir();
        fs::write(
            dir.join("001_create_posts.sql"),
            "CREATE TABLE posts (\n  id UUID PRIMARY KEY,\n  title TEXT NOT NULL\n);\n",
        )
        .unwrap();
        fs::write(
            dir.join("002_alter_posts.sql"),
            "ALTER TABLE posts ADD COLUMN views INTEGER;\n",
        )
        .unwrap();

        // `views` already migrated -> no longer counted as added.
        let diff = diff_model(
            &dir,
            "posts",
            &model(&["title:String", "views:i32"]),
            Dialect::Postgres,
        )
        .unwrap()
        .unwrap();
        assert!(diff.is_empty(), "views is already migrated");
    }

    #[test]
    fn dropped_column_is_withheld_without_force() {
        let dir = tmpdir();
        fs::write(
            dir.join("001_create_posts.sql"),
            "CREATE TABLE posts (\n  id UUID PRIMARY KEY,\n  title TEXT NOT NULL,\n  subtitle TEXT NOT NULL\n);\n",
        )
        .unwrap();

        let diff = diff_model(&dir, "posts", &model(&["title:String"]), Dialect::Postgres)
            .unwrap()
            .unwrap();
        assert_eq!(diff.dropped, vec!["subtitle".to_string()]);
        // implicit columns never proposed for drop
        assert!(!diff.dropped.iter().any(|c| c == "id"));

        let withheld = alter_sql("posts", &diff, Dialect::Postgres, false);
        assert!(withheld.contains("DROP of `subtitle` withheld"));
        // No *executable* drop statement — the drop is only mentioned inside a `--` comment.
        assert!(!has_executable_drop(&withheld));

        let forced = alter_sql("posts", &diff, Dialect::Postgres, true);
        assert!(has_executable_drop(&forced));
        assert!(forced.contains("DROP COLUMN subtitle;"));
    }
    #[test]
    fn replays_drops_reintroductions_and_numeric_versions() {
        let dir = tmpdir();
        fs::write(dir.join("2_create_posts.up.sql"), "CREATE TABLE posts (id UUID PRIMARY KEY, title TEXT NOT NULL, views INTEGER NOT NULL);").unwrap();
        fs::write(dir.join("2_create_posts.down.sql"), "DROP TABLE posts;").unwrap();
        fs::write(
            dir.join("10_drop.sql"),
            "ALTER TABLE posts DROP COLUMN views;",
        )
        .unwrap();
        let diff = diff_model(&dir, "posts", &model(&["title:String"]), Dialect::Postgres)
            .unwrap()
            .unwrap();
        assert!(diff.is_empty());
        let diff = diff_model(
            &dir,
            "posts",
            &model(&["title:String", "views:i64"]),
            Dialect::Postgres,
        )
        .unwrap()
        .unwrap();
        assert_eq!(diff.added[0].name, "views");
        fs::write(
            dir.join("11_reintroduce.sql"),
            alter_sql("posts", &diff, Dialect::Postgres, false),
        )
        .unwrap();
        let diff = diff_model(
            &dir,
            "posts",
            &model(&["title:String", "views:i64"]),
            Dialect::Postgres,
        )
        .unwrap()
        .unwrap();
        assert!(diff.is_empty());
        assert_eq!(diff.nullable, ["views"]);
        fs::write(dir.join("12_tighten.sql"), "UPDATE posts SET views = 0 WHERE views IS NULL; ALTER TABLE posts ALTER COLUMN views SET NOT NULL;").unwrap();
        assert!(
            diff_model(
                &dir,
                "posts",
                &model(&["title:String", "views:i64"]),
                Dialect::Postgres
            )
            .unwrap()
            .unwrap()
            .nullable
            .is_empty()
        );
    }

    #[test]
    fn recognizes_manual_type_changes_in_postgres_and_mysql() {
        for (dialect, create_type, alter) in [
            (
                Dialect::Postgres,
                "INTEGER",
                "ALTER TABLE posts ALTER COLUMN views TYPE BIGINT USING views::bigint;",
            ),
            (
                Dialect::MySql,
                "INT",
                "ALTER TABLE posts MODIFY COLUMN views BIGINT NOT NULL;",
            ),
        ] {
            let dir = tmpdir();
            fs::write(
                dir.join("1_create.sql"),
                format!("CREATE TABLE posts (views {create_type} NOT NULL);"),
            )
            .unwrap();
            let desired = model(&["views:i64"]);
            let diff = diff_model(&dir, "posts", &desired, dialect)
                .unwrap()
                .unwrap();
            assert!(!diff.is_empty());
            assert_eq!(diff.changed.len(), 1);
            assert_eq!(diff.changed[0].from, "INTEGER");
            assert_eq!(diff.changed[0].to, "BIGINT");
            fs::write(dir.join("2_change.sql"), alter).unwrap();
            let diff = diff_model(&dir, "posts", &desired, dialect)
                .unwrap()
                .unwrap();
            assert!(diff.is_empty());
            assert!(diff.nullable.is_empty());
        }
    }

    #[test]
    fn sqlite_rebuild_replaces_type_and_nullability() {
        let dir = tmpdir();
        fs::write(dir.join("1_create.sql"), "CREATE TABLE posts (views TEXT);").unwrap();
        assert_eq!(
            diff_model(&dir, "posts", &model(&["views:i32"]), Dialect::Sqlite)
                .unwrap()
                .unwrap()
                .changed
                .len(),
            1
        );
        fs::write(dir.join("2_rebuild.sql"), "BEGIN; CREATE TABLE posts_new (views INTEGER NOT NULL); INSERT INTO posts_new SELECT CAST(views AS INTEGER) FROM posts; DROP TABLE posts; ALTER TABLE posts_new RENAME TO posts; COMMIT;").unwrap();
        let diff = diff_model(&dir, "posts", &model(&["views:i32"]), Dialect::Sqlite)
            .unwrap()
            .unwrap();
        assert!(diff.is_empty());
        assert!(diff.nullable.is_empty());
    }

    #[test]
    fn parses_comments_quotes_multiline_and_exact_table_names() {
        let dir = tmpdir();
        fs::write(
            dir.join("1_create.sql"),
            r#"
            -- CREATE TABLE posts (fake INTEGER);
            CREATE TABLE posts_archive (unrelated INTEGER);
            create table "posts" ("title" TEXT NOT NULL DEFAULT 'a; b, (c)', views INT4 NOT NULL);
            /* ALTER TABLE posts DROP COLUMN title; */
            ALTER TABLE "posts"
                RENAME COLUMN "views" TO "count";
            ALTER TABLE posts DROP COLUMN count, ADD COLUMN total BIGINT NOT NULL;
        "#,
        )
        .unwrap();
        let diff = diff_model(
            &dir,
            "posts",
            &model(&["title:String", "total:i64"]),
            Dialect::Postgres,
        )
        .unwrap()
        .unwrap();
        assert!(diff.is_empty());
        assert!(diff.nullable.is_empty());
    }

    #[test]
    fn rejects_unknown_or_inconsistent_history_with_file_context() {
        for sql in [
            "ALTER TABLE posts ENABLE ROW LEVEL SECURITY;",
            "ALTER TABLE posts DROP COLUMN missing;",
            "ALTER TABLE posts ADD COLUMN views INTEGER;",
            "CREATE TABLE posts (views INTEGER);",
            "CREATE TABLE copied AS SELECT * FROM posts;",
            "CREATE TABLE public.other (views INTEGER);",
            "this is not SQL;",
            "DROP FUNCTION old_function() CASCADE;",
            "DROP TRIGGER old_trigger ON posts CASCADE;",
            "DROP EXTENSION pgcrypto;",
            "DROP TYPE mood CASCADE;",
            "ALTER TYPE mood ADD VALUE 'sleepy';",
        ] {
            let dir = tmpdir();
            fs::write(
                dir.join("1_create.sql"),
                "CREATE TABLE posts (views INTEGER NOT NULL);",
            )
            .unwrap();
            fs::write(dir.join("2_manual.sql"), sql).unwrap();
            let err =
                diff_model(&dir, "posts", &model(&["views:i32"]), Dialect::Postgres).unwrap_err();
            assert!(format!("{err:#}").contains("2_manual.sql"), "{err:#}");
        }
    }

    #[test]
    fn generated_create_templates_replay_for_all_dialects() {
        for dialect in [Dialect::Postgres, Dialect::Sqlite, Dialect::MySql] {
            let dir = tmpdir();
            let resource = model(&[
                "title:String",
                "active:bool",
                "views:i32",
                "total:i64",
                "score:f64",
                "owner:Uuid",
                "published:DateTime",
                "author:belongs_to:users",
            ]);
            fs::write(
                dir.join("1_users.sql"),
                gize_templates::user::migration_sql(dialect),
            )
            .unwrap();
            fs::write(
                dir.join("2_posts.sql"),
                gize_templates::model::migration_sql(&resource, dialect),
            )
            .unwrap();
            let diff = diff_model(&dir, "posts", &resource, dialect)
                .unwrap()
                .unwrap();
            assert!(diff.is_empty(), "{dialect:?}: {diff:?}");
            assert!(diff.nullable.is_empty());
        }
    }

    #[test]
    fn duplicate_versions_are_not_silently_ordered() {
        let dir = tmpdir();
        fs::write(dir.join("1_a.sql"), "CREATE TABLE posts (views INTEGER);").unwrap();
        fs::write(dir.join("1_b.sql"), "ALTER TABLE posts DROP COLUMN views;").unwrap();
        assert!(
            format!(
                "{:#}",
                diff_model(&dir, "posts", &model(&[]), Dialect::Postgres).unwrap_err()
            )
            .contains("duplicate migration version")
        );
    }
    #[test]
    fn mysql_change_column_and_conditional_operations_replay() {
        let dir = tmpdir();
        fs::write(dir.join("1_create.sql"), "CREATE TABLE posts (old INT NOT NULL); CREATE TABLE IF NOT EXISTS posts (ignored TEXT); ALTER TABLE posts CHANGE COLUMN old views BIGINT NOT NULL; ALTER TABLE posts DROP COLUMN IF EXISTS missing;").unwrap();
        let diff = diff_model(&dir, "posts", &model(&["views:i64"]), Dialect::MySql)
            .unwrap()
            .unwrap();
        assert!(diff.is_empty());
        assert!(diff.nullable.is_empty());
        fs::write(
            dir.join("2_conditional.sql"),
            "/*!80000 ALTER TABLE posts DROP COLUMN views */;",
        )
        .unwrap();
        assert!(
            format!(
                "{:#}",
                diff_model(&dir, "posts", &model(&["views:i64"]), Dialect::MySql).unwrap_err()
            )
            .contains("executable comment")
        );
    }

    #[test]
    fn sqlite_foreign_key_pragmas_do_not_change_column_shape() {
        let dir = tmpdir();
        fs::write(dir.join("1_create.sql"), "PRAGMA foreign_keys = 0; CREATE TABLE posts (views INTEGER NOT NULL); PRAGMA foreign_keys = 1;").unwrap();
        assert!(
            diff_model(&dir, "posts", &model(&["views:i32"]), Dialect::Sqlite)
                .unwrap()
                .unwrap()
                .is_empty()
        );
        fs::write(
            dir.join("2_unknown_pragma.sql"),
            "PRAGMA writable_schema = 1;",
        )
        .unwrap();
        assert!(
            format!(
                "{:#}",
                diff_model(&dir, "posts", &model(&["views:i32"]), Dialect::Sqlite).unwrap_err()
            )
            .contains("unsupported PRAGMA")
        );
    }
    #[test]
    fn postgres_auxiliary_ddl_does_not_block_columns() {
        let dir = tmpdir();
        fs::write(
            dir.join("1_schema.sql"),
            r#"
CREATE TYPE mood AS ENUM ('happy', 'sad');
CREATE TYPE legacy_mood AS ENUM ('meh');
DROP TYPE IF EXISTS legacy_mood;
DROP FUNCTION IF EXISTS old_function();
DROP TRIGGER IF EXISTS old_trigger ON posts;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE TABLE tags (id UUID PRIMARY KEY);
CREATE TABLE posts (id UUID PRIMARY KEY, tag_id UUID, title VARCHAR(120) NOT NULL);
ALTER TABLE posts ADD CONSTRAINT fk FOREIGN KEY (tag_id) REFERENCES tags(id);
ALTER TABLE posts ADD CONSTRAINT title_nonempty CHECK (title <> '');
ALTER TABLE posts DROP CONSTRAINT title_nonempty;
COMMENT ON TABLE posts IS 'custom table';
GRANT SELECT ON posts TO PUBLIC;
REVOKE SELECT ON posts FROM PUBLIC;
CREATE VIEW post_titles AS SELECT title FROM posts;
CREATE FUNCTION touch_post() RETURNS trigger AS $$ BEGIN RETURN NEW; END; $$ LANGUAGE plpgsql;
CREATE TRIGGER touch BEFORE UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION touch_post();
"#,
        )
        .unwrap();
        let history = SchemaHistory::load(&dir, Dialect::Postgres).unwrap();
        // Remove the source after loading: multiple module diffs use the same snapshot.
        fs::remove_file(dir.join("1_schema.sql")).unwrap();
        let diff = history
            .diff(
                "posts",
                &model(&["tag_id:Uuid", "title:String", "views:i32"]),
            )
            .unwrap();
        assert_eq!(diff.added[0].name, "views");
        assert_eq!(diff.changed[0].from, "VARCHAR(120)");
        assert!(history.diff("tags", &model(&[])).unwrap().is_empty());
    }

    #[test]
    fn primary_key_constraints_update_nullability() {
        let dir = tmpdir();
        fs::write(dir.join("1_schema.sql"), "CREATE TABLE posts (views INTEGER); ALTER TABLE posts ADD CONSTRAINT posts_pk PRIMARY KEY (views); ALTER TABLE posts DROP CONSTRAINT posts_pk;").unwrap();
        let diff = diff_model(&dir, "posts", &model(&["views:i32"]), Dialect::Postgres)
            .unwrap()
            .unwrap();
        assert!(diff.nullable.is_empty()); // PostgreSQL retains the implied NOT NULL.
    }

    #[test]
    fn mysql_known_columns_supports_backticks() {
        let dir = tmpdir();
        fs::write(
            dir.join("1_schema.sql"),
            "CREATE TABLE `posts` (`title` VARCHAR(120) NOT NULL);",
        )
        .unwrap();
        assert!(
            known_columns_for_dialect(&dir, "posts", Dialect::MySql)
                .unwrap()
                .unwrap()
                .contains("title")
        );
    }
}
