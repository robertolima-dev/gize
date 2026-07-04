//! Templates for `gize make model`: the model struct and its migration.

use gize_core::ModelSpec;
use gize_core::naming::table_name;

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

/// Render a `CREATE TABLE` migration for a model (ADR-011: SQL-first, Postgres).
pub fn migration_sql(model: &ModelSpec) -> String {
    let table = table_name(&model.name);
    let mut columns = String::new();
    for f in &model.fields {
        // MVP keeps every generated column NOT NULL for clarity; nullability tuning is a
        // follow-up once optional fields (`name:String?`) land.
        columns.push_str(&format!(
            "    {name} {sql} NOT NULL,\n",
            name = f.name,
            sql = f.ty.sql_type(),
        ));
    }

    format!(
        r#"-- Migration: create {table}
CREATE TABLE {table} (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
{columns}    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#
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
        let out = migration_sql(&user());
        assert!(out.contains("CREATE TABLE users"));
        assert!(out.contains("name TEXT NOT NULL"));
        assert!(out.contains("active BOOLEAN NOT NULL"));
    }
}
