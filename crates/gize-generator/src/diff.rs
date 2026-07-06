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

use anyhow::{Context, Result};
use gize_core::{Field, ModelSpec};

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
}

impl SchemaDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.dropped.is_empty()
    }
}

/// Compute the diff for `model` (a module's declared shape) against the columns already
/// present in its table's generated migrations under `migrations_dir`.
///
/// Returns `Ok(None)` when the table has no `CREATE TABLE` migration yet — there is nothing
/// to diff against, and the caller should scaffold the resource (`make crud` / `sync`) first.
pub fn diff_model(
    migrations_dir: &Path,
    table: &str,
    model: &ModelSpec,
) -> Result<Option<SchemaDiff>> {
    let Some(known) = known_columns(migrations_dir, table)? else {
        return Ok(None);
    };

    let declared: BTreeSet<&str> = model.fields.iter().map(|f| f.name.as_str()).collect();

    let added = model
        .fields
        .iter()
        .filter(|f| !known.contains(&f.name))
        .cloned()
        .collect();

    let dropped = known
        .iter()
        .filter(|c| !declared.contains(c.as_str()))
        .filter(|c| !IMPLICIT_COLUMNS.contains(&c.as_str()))
        .cloned()
        .collect();

    Ok(Some(SchemaDiff { added, dropped }))
}

/// The set of column names a table already has, parsed from every generated migration that
/// touches it (`*_create_<table>.sql` and any `ALTER TABLE <table> ADD COLUMN`). Returns
/// `None` if the table has no create migration yet.
pub fn known_columns(migrations_dir: &Path, table: &str) -> Result<Option<BTreeSet<String>>> {
    if !migrations_dir.exists() {
        return Ok(None);
    }
    let mut columns = BTreeSet::new();
    let mut saw_create = false;

    // Sort by filename so migrations are read in version (timestamp) order.
    let mut files: Vec<_> = std::fs::read_dir(migrations_dir)
        .with_context(|| format!("reading {}", migrations_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sql"))
        .collect();
    files.sort();

    for path in files {
        let sql = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let (created, cols) = columns_in_sql(&sql, table);
        saw_create |= created;
        columns.extend(cols);
    }

    Ok(saw_create.then_some(columns))
}

/// Extract the columns a single migration file declares for `table`. Returns whether it
/// contained the `CREATE TABLE`, plus the column names found (from the create body and any
/// `ADD COLUMN`s). We parse only the SQL shapes Gize itself generates.
fn columns_in_sql(sql: &str, table: &str) -> (bool, BTreeSet<String>) {
    let mut cols = BTreeSet::new();
    let mut created = false;

    // CREATE TABLE <table> ( ... ); — match the balanced closing paren, since column
    // defaults like `gen_random_uuid()` contain their own parentheses.
    let needle = format!("CREATE TABLE {table}");
    if let Some(start) = sql.find(&needle) {
        let after = &sql[start..];
        if let Some(open) = after.find('(') {
            let bytes = after.as_bytes();
            let mut depth = 0i32;
            for (i, &b) in bytes.iter().enumerate().skip(open) {
                match b {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            created = true;
                            for line in after[open + 1..i].lines() {
                                if let Some(name) = column_name_of(line) {
                                    cols.insert(name);
                                }
                            }
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // ALTER TABLE <table> ADD COLUMN <name> ...
    let alter = format!("ALTER TABLE {table} ADD COLUMN ");
    for line in sql.lines() {
        if let Some(rest) = line.trim().strip_prefix(&alter) {
            if let Some(name) = rest.split_whitespace().next() {
                cols.insert(name.trim_matches('"').to_string());
            }
        }
    }

    (created, cols)
}

/// The column name a `CREATE TABLE` body line declares, or `None` for blank lines and
/// table-level constraints. Column lines start with a lowercase snake_case identifier
/// (`name TEXT NOT NULL,`); constraints start with an uppercase keyword (`FOREIGN KEY ...`,
/// `PRIMARY KEY ...`) and are skipped.
fn column_name_of(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches(',');
    let first = trimmed.split_whitespace().next()?;
    let starts_lower = first.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    if starts_lower {
        Some(first.to_string())
    } else {
        None
    }
}

/// Render an `ALTER TABLE` migration that reconciles `table` to its declared shape.
///
/// Added columns are emitted **nullable** (not `NOT NULL`) even though Gize's create tables
/// are `NOT NULL`: adding a `NOT NULL` column without a default to a table that may hold rows
/// fails, so the safe move is a nullable column plus a `-- TODO` to backfill and tighten
/// (ADR-011 revision). Drops are only included when `force` is set.
pub fn alter_sql(table: &str, diff: &SchemaDiff, force: bool) -> String {
    let mut out = format!("-- Migration: alter {table} (reconcile from gize.toml)\n");
    out.push_str("-- Generated by `gize make migration` from a model change (see ADR-011).\n\n");

    for f in &diff.added {
        out.push_str(&format!(
            "ALTER TABLE {table} ADD COLUMN {name} {ty}; \
             -- TODO: nullable for safety — backfill, then `ALTER COLUMN {name} SET NOT NULL`\n",
            name = f.name,
            ty = f.ty.sql_type(),
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
        let dir = std::env::temp_dir().join(format!(
            "gize-diff-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
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
    fn no_create_migration_yields_none() {
        let dir = tmpdir();
        assert!(
            diff_model(&dir, "posts", &model(&["title:String"]))
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

        let diff = diff_model(&dir, "posts", &model(&["title:String", "views:i32"]))
            .unwrap()
            .unwrap();
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].name, "views");
        assert!(diff.dropped.is_empty());

        let sql = alter_sql("posts", &diff, false);
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
        let diff = diff_model(&dir, "posts", &model(&["title:String", "body:String"]))
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
        let diff = diff_model(&dir, "posts", &model(&["title:String", "views:i32"]))
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

        let diff = diff_model(&dir, "posts", &model(&["title:String"]))
            .unwrap()
            .unwrap();
        assert_eq!(diff.dropped, vec!["subtitle".to_string()]);
        // implicit columns never proposed for drop
        assert!(!diff.dropped.iter().any(|c| c == "id"));

        let withheld = alter_sql("posts", &diff, false);
        assert!(withheld.contains("DROP of `subtitle` withheld"));
        // No *executable* drop statement — the drop is only mentioned inside a `--` comment.
        assert!(!has_executable_drop(&withheld));

        let forced = alter_sql("posts", &diff, true);
        assert!(has_executable_drop(&forced));
        assert!(forced.contains("DROP COLUMN subtitle;"));
    }
}
