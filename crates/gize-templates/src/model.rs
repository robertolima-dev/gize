//! Templates for `gize make model`: the model struct and its migration.

use gize_core::naming::table_name;
use gize_core::{Dialect, ModelSpec};

/// Render `model.rs` for a model: an `sqlx::FromRow` struct plus an `id` and timestamps.
pub fn model_rs(model: &ModelSpec) -> String {
    let mut fields = String::new();
    for f in &model.fields {
        fields.push_str(&format!("    pub {}: {},\n", f.name, f.ty.rust_type()));
    }

    format!(
        r#"use serde::{{Deserialize, Serialize}};

/// The `{name}` domain model, mapped to the `{table}` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct {name} {{
    pub id: uuid::Uuid,
{fields}    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}}
"#,
        name = model.name,
        table = table_name(&model.name),
    )
}

/// Render a `CREATE TABLE` migration for a model (ADR-011: SQL-first). The `dialect` chooses
/// the column types, primary-key generation and timestamp defaults (ADR-015).
pub fn migration_sql(model: &ModelSpec, dialect: Dialect) -> String {
    let table = table_name(&model.name);
    let mut columns = String::new();
    for f in &model.fields {
        // MVP keeps every generated column NOT NULL for clarity; nullability tuning is a
        // follow-up once optional fields (`name:String?`) land.
        columns.push_str(&format!(
            "    {name} {sql} NOT NULL,\n",
            name = f.name,
            sql = dialect.column_type(f.ty),
        ));
    }

    // Foreign-key constraints for each `belongs_to` relationship (ADR-014). The FK column
    // itself is emitted by the loop above (it is a synthetic `<name>_id` field); this adds
    // the referential constraint.
    let mut constraints = String::new();
    for r in &model.relations {
        constraints.push_str(&format!(
            ",\n    FOREIGN KEY ({col}) REFERENCES {target}(id)",
            col = r.fk_column(),
            target = r.target,
        ));
    }

    let ts = dialect.timestamp_type_default();
    format!(
        r#"-- Migration: create {table}
CREATE TABLE {table} (
    {id_pk},
{columns}    created_at {ts},
    updated_at {ts}{constraints}
);
"#,
        id_pk = dialect.id_pk_ddl(),
    )
}

/// Render a blank migration for `gize make migration <name>` (ADR-011: SQL-first, Postgres).
///
/// This is the hand-written escape hatch: an empty, timestamped file the developer fills in
/// (indexes, constraints, data backfills, column changes) — everything the model-driven
/// `CREATE TABLE` generator does not cover yet.
pub fn blank_migration_sql(name: &str) -> String {
    format!(
        "-- Migration: {name}\n\
         -- Write your forward schema changes below (SQL-first, Postgres; see ADR-011).\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> ModelSpec {
        ModelSpec::parse(
            "User",
            &["name:String".to_string(), "active:bool".to_string()],
        )
        .unwrap()
    }

    #[test]
    fn model_struct_has_fields_and_metadata() {
        let out = model_rs(&user());
        assert!(out.contains("pub struct User"));
        assert!(out.contains("pub name: String,"));
        assert!(out.contains("pub active: bool,"));
        assert!(out.contains("pub id: uuid::Uuid,"));
    }

    #[test]
    fn migration_creates_table() {
        let out = migration_sql(&user(), Dialect::Postgres);
        assert!(out.contains("CREATE TABLE users"));
        assert!(out.contains("name TEXT NOT NULL"));
        assert!(out.contains("active BOOLEAN NOT NULL"));
    }

    #[test]
    fn migration_follows_the_sqlite_dialect() {
        let out = migration_sql(&user(), Dialect::Sqlite);
        assert!(out.contains("id TEXT PRIMARY KEY"));
        assert!(out.contains("active INTEGER NOT NULL")); // bool -> INTEGER
        assert!(out.contains("created_at TEXT NOT NULL DEFAULT (strftime"));
        assert!(!out.contains("gen_random_uuid"));
        assert!(!out.contains("TIMESTAMPTZ"));
    }

    #[test]
    fn blank_migration_names_and_is_empty_of_schema() {
        let out = blank_migration_sql("add_index_to_users");
        assert!(out.contains("-- Migration: add_index_to_users"));
        assert!(!out.contains("CREATE TABLE"));
    }
}
