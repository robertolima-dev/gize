//! Model and field definitions parsed from the CLI (`name:String email:String ...`).

use anyhow::{Context, Result, bail};

/// The set of scalar field types Gize understands in the MVP.
///
/// Kept deliberately small and explicit (ADR-003 / ADR-011): each variant maps to a Rust
/// type and a SQL column type. Unknown types are rejected early with a helpful error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Bool,
    I32,
    I64,
    F64,
    Uuid,
    DateTime,
}

impl FieldType {
    /// Parse a field type from its CLI spelling (case-insensitive).
    pub fn parse(raw: &str) -> Result<Self> {
        let ty = match raw.to_ascii_lowercase().as_str() {
            "string" | "str" => Self::String,
            "bool" | "boolean" => Self::Bool,
            "i32" | "int" => Self::I32,
            "i64" | "bigint" | "long" => Self::I64,
            "f64" | "float" | "double" => Self::F64,
            "uuid" => Self::Uuid,
            "datetime" | "timestamp" => Self::DateTime,
            other => bail!(
                "unknown field type `{other}` (supported: String, Bool, i32, i64, f64, Uuid, DateTime)"
            ),
        };
        Ok(ty)
    }

    /// The idiomatic Rust type used in generated structs.
    pub fn rust_type(self) -> &'static str {
        match self {
            Self::String => "String",
            Self::Bool => "bool",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Uuid => "uuid::Uuid",
            Self::DateTime => "chrono::DateTime<chrono::Utc>",
        }
    }

    /// Whether the mapped Rust type is `Copy`. Used by generators to decide between
    /// `.bind(x)` (Copy) and `.bind(x.clone())` (owned), keeping generated code
    /// clippy-clean (`clone_on_copy`).
    pub fn is_copy(self) -> bool {
        // `String` is the only non-Copy type in the MVP set; `uuid::Uuid` and
        // `chrono::DateTime<Utc>` are both Copy.
        !matches!(self, Self::String)
    }

    /// The PostgreSQL column type used in generated migrations (ADR-011).
    pub fn sql_type(self) -> &'static str {
        match self {
            Self::String => "TEXT",
            Self::Bool => "BOOLEAN",
            Self::I32 => "INTEGER",
            Self::I64 => "BIGINT",
            Self::F64 => "DOUBLE PRECISION",
            Self::Uuid => "UUID",
            Self::DateTime => "TIMESTAMPTZ",
        }
    }
}

/// A single field of a model: `name:Type`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
}

impl Field {
    /// Parse one `name:Type` token.
    pub fn parse(token: &str) -> Result<Self> {
        let (name, ty) = token
            .split_once(':')
            .with_context(|| format!("field `{token}` must be in the form name:Type"))?;
        if name.is_empty() {
            bail!("field `{token}` has an empty name");
        }
        Ok(Self {
            name: name.to_string(),
            ty: FieldType::parse(ty)?,
        })
    }
}

/// A model: a name plus its fields, as produced by `gize make model Name f:T ...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    pub name: String,
    pub fields: Vec<Field>,
}

impl ModelSpec {
    /// Build a model spec from a name and raw `name:Type` tokens.
    pub fn parse(name: impl Into<String>, tokens: &[String]) -> Result<Self> {
        let fields = tokens
            .iter()
            .map(|t| Field::parse(t))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            name: name.into(),
            fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_field() {
        let f = Field::parse("email:String").unwrap();
        assert_eq!(f.name, "email");
        assert_eq!(f.ty, FieldType::String);
        assert_eq!(f.ty.rust_type(), "String");
        assert_eq!(f.ty.sql_type(), "TEXT");
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(Field::parse("x:Blob").is_err());
    }

    #[test]
    fn rejects_malformed_token() {
        assert!(Field::parse("notype").is_err());
    }

    #[test]
    fn parses_a_model() {
        let m = ModelSpec::parse(
            "User",
            &["name:String".to_string(), "active:bool".to_string()],
        )
        .unwrap();
        assert_eq!(m.name, "User");
        assert_eq!(m.fields.len(), 2);
        assert_eq!(m.fields[1].ty, FieldType::Bool);
    }
}
