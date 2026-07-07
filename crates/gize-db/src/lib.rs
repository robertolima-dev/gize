//! Data-layer conventions for Gize (ADR-003, ADR-011).
//!
//! MVP scope is intentionally thin: it centralises the mapping between Gize field types
//! and Postgres column types so generators and (future) migration diffing agree. The
//! SQLx pool wiring lives in generated app code, not here.

use gize_core::FieldType;

pub mod admin;
pub mod migrate;

/// The Postgres column type for a Gize field type. Single source of truth reused by the
/// migration templates.
pub fn pg_column_type(ty: FieldType) -> &'static str {
    ty.sql_type()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_types() {
        assert_eq!(pg_column_type(FieldType::String), "TEXT");
        assert_eq!(pg_column_type(FieldType::Uuid), "UUID");
        assert_eq!(pg_column_type(FieldType::DateTime), "TIMESTAMPTZ");
    }
}
