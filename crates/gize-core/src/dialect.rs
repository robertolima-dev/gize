//! The database dialect seam (ADR-015).
//!
//! Everything that differs between the supported databases — column types, primary-key
//! generation, bind placeholders, pool types, integrity-error codes — lives here, chosen from
//! `stack.database` in the manifest. Postgres is the default and its output is byte-identical
//! to the pre-seam generator; SQLite is the second target (serverless, great for tests).

use crate::FieldType;

/// A supported database dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    Sqlite,
}

impl Dialect {
    /// Pick the dialect from the manifest's `stack.database` (`"sqlite"` → SQLite, else the
    /// Postgres default).
    pub fn from_database(database: &str) -> Self {
        if database.eq_ignore_ascii_case("sqlite") {
            Dialect::Sqlite
        } else {
            Dialect::Postgres
        }
    }

    /// The `id` primary-key column DDL (no trailing comma). Postgres generates the UUID in the
    /// database; SQLite has no UUID generator, so the app supplies it on insert.
    pub fn id_pk_ddl(self) -> &'static str {
        match self {
            Dialect::Postgres => "id UUID PRIMARY KEY DEFAULT gen_random_uuid()",
            Dialect::Sqlite => "id TEXT PRIMARY KEY",
        }
    }

    /// Whether the app must generate `id` on insert (SQLite) rather than the database (Postgres).
    pub fn app_generates_id(self) -> bool {
        matches!(self, Dialect::Sqlite)
    }

    /// The SQL column type for a scalar field.
    pub fn column_type(self, ty: FieldType) -> &'static str {
        match self {
            // Postgres keeps the canonical mapping (ADR-011).
            Dialect::Postgres => ty.sql_type(),
            // SQLite storage classes: TEXT / INTEGER / REAL. UUIDs are stored by sqlx as BLOB
            // but a TEXT-affinity column holds them fine; timestamps are ISO-8601 TEXT.
            Dialect::Sqlite => match ty {
                FieldType::String | FieldType::Uuid | FieldType::DateTime => "TEXT",
                FieldType::Bool | FieldType::I32 | FieldType::I64 => "INTEGER",
                FieldType::F64 => "REAL",
            },
        }
    }

    /// The `TYPE NOT NULL DEFAULT ...` fragment for the `created_at`/`updated_at` columns.
    pub fn timestamp_type_default(self) -> &'static str {
        match self {
            Dialect::Postgres => "TIMESTAMPTZ NOT NULL DEFAULT now()",
            // RFC-3339 with a `Z` so sqlx decodes it straight into `DateTime<Utc>`.
            Dialect::Sqlite => "TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        }
    }

    /// The SQL expression for "now" used in `UPDATE ... SET updated_at = <now>`.
    pub fn now_expr(self) -> &'static str {
        match self {
            Dialect::Postgres => "now()",
            Dialect::Sqlite => "strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        }
    }

    /// A bind placeholder for parameter `n` (1-based): `$n` on Postgres, `?n` on SQLite.
    pub fn placeholder(self, n: usize) -> String {
        match self {
            Dialect::Postgres => format!("${n}"),
            Dialect::Sqlite => format!("?{n}"),
        }
    }

    /// The sqlx pool type used in generated code.
    pub fn pool_type(self) -> &'static str {
        match self {
            Dialect::Postgres => "PgPool",
            Dialect::Sqlite => "SqlitePool",
        }
    }

    /// The sqlx pool-options type.
    pub fn pool_options(self) -> &'static str {
        match self {
            Dialect::Postgres => "PgPoolOptions",
            Dialect::Sqlite => "SqlitePoolOptions",
        }
    }

    /// The sqlx submodule the pool types live in (`sqlx::postgres` / `sqlx::sqlite`).
    pub fn sqlx_module(self) -> &'static str {
        match self {
            Dialect::Postgres => "postgres",
            Dialect::Sqlite => "sqlite",
        }
    }

    /// The sqlx feature to enable in the generated `Cargo.toml`.
    pub fn sqlx_feature(self) -> &'static str {
        match self {
            Dialect::Postgres => "postgres",
            Dialect::Sqlite => "sqlite",
        }
    }

    /// The integrity-error code a unique violation reports (mapped to 409 in generated code).
    /// Postgres SQLSTATE `23505`; SQLite extended result code `2067` (SQLITE_CONSTRAINT_UNIQUE).
    pub fn unique_violation_code(self) -> &'static str {
        match self {
            Dialect::Postgres => "23505",
            Dialect::Sqlite => "2067",
        }
    }

    /// The integrity-error code a foreign-key violation reports (mapped to 409).
    /// Postgres `23503`; SQLite `787` (SQLITE_CONSTRAINT_FOREIGNKEY).
    pub fn foreign_key_violation_code(self) -> &'static str {
        match self {
            Dialect::Postgres => "23503",
            Dialect::Sqlite => "787",
        }
    }

    /// An example `DATABASE_URL` for `.env.example`.
    pub fn example_url(self, project: &str) -> String {
        match self {
            Dialect::Postgres => {
                format!("postgres://postgres:postgres@localhost:5432/{project}")
            }
            Dialect::Sqlite => format!("sqlite://{project}.db?mode=rwc"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_output_is_the_canonical_mapping() {
        let d = Dialect::from_database("postgres");
        assert_eq!(
            d.id_pk_ddl(),
            "id UUID PRIMARY KEY DEFAULT gen_random_uuid()"
        );
        assert_eq!(d.column_type(FieldType::String), "TEXT");
        assert_eq!(d.column_type(FieldType::DateTime), "TIMESTAMPTZ");
        assert_eq!(d.placeholder(3), "$3");
        assert!(!d.app_generates_id());
        assert_eq!(d.pool_type(), "PgPool");
    }

    #[test]
    fn sqlite_maps_types_and_placeholders() {
        let d = Dialect::from_database("sqlite");
        assert_eq!(d.id_pk_ddl(), "id TEXT PRIMARY KEY");
        assert_eq!(d.column_type(FieldType::Uuid), "TEXT");
        assert_eq!(d.column_type(FieldType::Bool), "INTEGER");
        assert_eq!(d.column_type(FieldType::F64), "REAL");
        assert_eq!(d.placeholder(2), "?2");
        assert!(d.app_generates_id());
        assert_eq!(d.pool_type(), "SqlitePool");
        assert_eq!(d.sqlx_feature(), "sqlite");
    }

    #[test]
    fn defaults_to_postgres() {
        assert_eq!(Dialect::from_database("mysql"), Dialect::Postgres);
        assert_eq!(Dialect::from_database(""), Dialect::Postgres);
    }
}
