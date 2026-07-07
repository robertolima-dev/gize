//! The database dialect seam (ADR-015).
//!
//! Everything that differs between the supported databases — column types, primary-key
//! generation, bind placeholders, pool types, integrity-error handling, and whether
//! `RETURNING` is available — lives here, chosen from `stack.database` in the manifest.
//! Postgres is the default and its output is byte-identical to the pre-seam generator; SQLite
//! is the serverless second target; MySQL is the third (ADR-015 amendment).

use crate::FieldType;

/// A supported database dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    Sqlite,
    MySql,
}

impl Dialect {
    /// Pick the dialect from the manifest's `stack.database` (`"sqlite"` → SQLite,
    /// `"mysql"` → MySQL, else the Postgres default).
    pub fn from_database(database: &str) -> Self {
        if database.eq_ignore_ascii_case("sqlite") {
            Dialect::Sqlite
        } else if database.eq_ignore_ascii_case("mysql") {
            Dialect::MySql
        } else {
            Dialect::Postgres
        }
    }

    /// The `id` primary-key column DDL (no trailing comma). Postgres generates the UUID in the
    /// database; SQLite and MySQL have no UUID generator, so the app supplies it on insert.
    /// MySQL uses `BINARY(16)` — the type sqlx encodes/decodes `uuid::Uuid` to natively, so the
    /// generated model, binds and `FromRow` stay uniform across dialects.
    pub fn id_pk_ddl(self) -> &'static str {
        match self {
            Dialect::Postgres => "id UUID PRIMARY KEY DEFAULT gen_random_uuid()",
            Dialect::Sqlite => "id TEXT PRIMARY KEY",
            Dialect::MySql => "id BINARY(16) PRIMARY KEY",
        }
    }

    /// Whether the app must generate `id` on insert (SQLite, MySQL) rather than the database
    /// (Postgres).
    pub fn app_generates_id(self) -> bool {
        matches!(self, Dialect::Sqlite | Dialect::MySql)
    }

    /// Whether the dialect supports `INSERT/UPDATE ... RETURNING *`. Postgres and SQLite do;
    /// MySQL does not, so the generated repository re-reads the row with a `SELECT` after
    /// writing (it knows the `id`, which the app generates).
    pub fn supports_returning(self) -> bool {
        !matches!(self, Dialect::MySql)
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
            // MySQL. `String` is `VARCHAR(255)` (indexable, unlike `TEXT` without a prefix
            // length, so `email UNIQUE` works); UUIDs are `BINARY(16)` (sqlx's native mapping).
            Dialect::MySql => match ty {
                FieldType::String => "VARCHAR(255)",
                FieldType::Uuid => "BINARY(16)",
                FieldType::DateTime => "DATETIME",
                FieldType::Bool => "BOOLEAN",
                FieldType::I32 => "INT",
                FieldType::I64 => "BIGINT",
                FieldType::F64 => "DOUBLE",
            },
        }
    }

    /// The `TYPE NOT NULL DEFAULT ...` fragment for the `created_at`/`updated_at` columns.
    pub fn timestamp_type_default(self) -> &'static str {
        match self {
            Dialect::Postgres => "TIMESTAMPTZ NOT NULL DEFAULT now()",
            // RFC-3339 with a `Z` so sqlx decodes it straight into `DateTime<Utc>`.
            Dialect::Sqlite => "TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            Dialect::MySql => "DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP",
        }
    }

    /// The SQL expression for "now" used in `UPDATE ... SET updated_at = <now>`.
    pub fn now_expr(self) -> &'static str {
        match self {
            Dialect::Postgres => "now()",
            Dialect::Sqlite => "strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            Dialect::MySql => "now()",
        }
    }

    /// A bind placeholder for parameter `n` (1-based): `$n` on Postgres, `?n` on SQLite, and a
    /// bare positional `?` on MySQL (its protocol binds positionally, without an index).
    pub fn placeholder(self, n: usize) -> String {
        match self {
            Dialect::Postgres => format!("${n}"),
            Dialect::Sqlite => format!("?{n}"),
            Dialect::MySql => "?".to_string(),
        }
    }

    /// The sqlx pool type used in generated code.
    pub fn pool_type(self) -> &'static str {
        match self {
            Dialect::Postgres => "PgPool",
            Dialect::Sqlite => "SqlitePool",
            Dialect::MySql => "MySqlPool",
        }
    }

    /// The sqlx pool-options type.
    pub fn pool_options(self) -> &'static str {
        match self {
            Dialect::Postgres => "PgPoolOptions",
            Dialect::Sqlite => "SqlitePoolOptions",
            Dialect::MySql => "MySqlPoolOptions",
        }
    }

    /// The sqlx submodule the pool types live in (`sqlx::postgres` / `sqlx::sqlite` /
    /// `sqlx::mysql`).
    pub fn sqlx_module(self) -> &'static str {
        match self {
            Dialect::Postgres => "postgres",
            Dialect::Sqlite => "sqlite",
            Dialect::MySql => "mysql",
        }
    }

    /// The sqlx feature to enable in the generated `Cargo.toml`.
    pub fn sqlx_feature(self) -> &'static str {
        match self {
            Dialect::Postgres => "postgres",
            Dialect::Sqlite => "sqlite",
            Dialect::MySql => "mysql",
        }
    }

    /// The full `if let sqlx::Error::Database(..) { .. }` block, inside `From<sqlx::Error>`, that
    /// classifies a database integrity error into `Error::Conflict` (unique violation) or
    /// `Error::ForeignKey`. Indented to sit at the 8-space body level of the `from` function.
    ///
    /// Postgres and SQLite expose distinct codes through `.code()` (SQLSTATE / extended result
    /// code). MySQL collapses both to SQLSTATE `23000`, so it must read the numeric error code
    /// (`1062` duplicate key, `1452` foreign key) off the concrete `MySqlDatabaseError`.
    pub fn integrity_error_mapping(self) -> String {
        let inner = match self {
            // Postgres SQLSTATE; SQLite extended result codes (2067 unique, 787 foreign key).
            Dialect::Postgres | Dialect::Sqlite => {
                let (unique, fk) = if self == Dialect::Postgres {
                    ("23505", "23503")
                } else {
                    ("2067", "787")
                };
                [
                    "            match db.code().as_deref() {".to_string(),
                    format!("                Some(\"{unique}\") => return Error::Conflict, // unique violation"),
                    format!("                Some(\"{fk}\") => return Error::ForeignKey, // foreign-key violation"),
                    "                _ => {}".to_string(),
                    "            }".to_string(),
                ]
                .join("\n")
            }
            Dialect::MySql => [
                "            if let Some(mysql) = db.try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>() {",
                "                match mysql.number() {",
                "                    1062 => return Error::Conflict, // duplicate key",
                "                    1452 => return Error::ForeignKey, // foreign-key violation",
                "                    _ => {}",
                "                }",
                "            }",
            ]
            .join("\n"),
        };
        format!("        if let sqlx::Error::Database(ref db) = error {{\n{inner}\n        }}")
    }

    /// An example `DATABASE_URL` for `.env.example`.
    pub fn example_url(self, project: &str) -> String {
        match self {
            Dialect::Postgres => {
                format!("postgres://postgres:postgres@localhost:5432/{project}")
            }
            Dialect::Sqlite => format!("sqlite://{project}.db?mode=rwc"),
            Dialect::MySql => format!("mysql://root:root@localhost:3306/{project}"),
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
    fn mysql_uses_binary_uuid_positional_binds_and_no_returning() {
        let d = Dialect::from_database("mysql");
        assert_eq!(d.id_pk_ddl(), "id BINARY(16) PRIMARY KEY");
        assert!(d.app_generates_id());
        assert!(!d.supports_returning());
        assert_eq!(d.column_type(FieldType::String), "VARCHAR(255)");
        assert_eq!(d.column_type(FieldType::Uuid), "BINARY(16)");
        assert_eq!(d.column_type(FieldType::DateTime), "DATETIME");
        assert_eq!(d.column_type(FieldType::Bool), "BOOLEAN");
        // MySQL binds positionally: every placeholder is a bare `?`.
        assert_eq!(d.placeholder(1), "?");
        assert_eq!(d.placeholder(4), "?");
        assert_eq!(d.pool_type(), "MySqlPool");
        assert_eq!(d.sqlx_feature(), "mysql");
        // Its integrity mapping reads the numeric error code, not SQLSTATE.
        let mapping = d.integrity_error_mapping();
        assert!(mapping.contains("MySqlDatabaseError"));
        assert!(mapping.contains("1062 => return Error::Conflict"));
        assert!(mapping.contains("1452 => return Error::ForeignKey"));
    }

    #[test]
    fn returning_dialects_map_codes_via_sqlstate() {
        // Postgres and SQLite support RETURNING and classify errors by `.code()`.
        for d in [Dialect::Postgres, Dialect::Sqlite] {
            assert!(d.supports_returning());
            assert!(d.integrity_error_mapping().contains("db.code()"));
        }
    }

    #[test]
    fn defaults_to_postgres() {
        assert_eq!(Dialect::from_database("mysql"), Dialect::MySql);
        assert_eq!(Dialect::from_database("cockroach"), Dialect::Postgres);
        assert_eq!(Dialect::from_database(""), Dialect::Postgres);
    }
}
