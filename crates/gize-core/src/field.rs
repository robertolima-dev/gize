//! Model and field definitions parsed from the CLI (`name:String email:String ...`).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

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

    /// The canonical CLI/manifest spelling of this type (the inverse of [`Self::parse`]).
    /// Used to serialize a field back into a `name:Type` token for `gize.toml` so the
    /// manifest is normalized regardless of the spelling the user typed (ADR-009 revision).
    pub fn as_token(self) -> &'static str {
        match self {
            Self::String => "String",
            Self::Bool => "bool",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Uuid => "Uuid",
            Self::DateTime => "DateTime",
        }
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
    /// Serialize this field back into its canonical `name:Type` token (inverse of
    /// [`Self::parse`] for the type part), for recording in `gize.toml`.
    pub fn to_token(&self) -> String {
        format!("{}:{}", self.name, self.ty.as_token())
    }

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

/// A `belongs_to` relationship declared on a model (ADR-014): a foreign key from this model
/// to `target`'s primary key. `field` is the local name (`author`), which yields the FK
/// column `author_id`. Serialized under `[[module.belongs_to]]` in `gize.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    /// The local relationship name, e.g. `author`.
    pub field: String,
    /// The referenced module/table, e.g. `users`.
    pub target: String,
}

impl Relation {
    /// The foreign-key column this relationship produces: `author` -> `author_id`.
    pub fn fk_column(&self) -> String {
        format!("{}_id", self.field)
    }

    /// The `name:belongs_to:target` token spelling, for reconstructing a model from the
    /// manifest and for round-tripping.
    pub fn to_token(&self) -> String {
        format!("{}:belongs_to:{}", self.field, self.target)
    }
}

/// A model: a name plus its fields and relationships, as produced by
/// `gize make crud Name f:T author:belongs_to:users ...`.
///
/// Each relationship is also expanded into a synthetic `<name>_id: Uuid` [`Field`] appended
/// to `fields`, so every field-driven template (model, dto, repository, …) picks up the
/// foreign-key column for free; only the migration consults `relations` directly, to emit the
/// `FOREIGN KEY` constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    pub name: String,
    pub fields: Vec<Field>,
    pub relations: Vec<Relation>,
}

impl ModelSpec {
    /// Build a model spec from a name and raw tokens. A token of the form
    /// `name:belongs_to:target` is parsed as a relationship; anything else is a scalar
    /// `name:Type` field.
    pub fn parse(name: impl Into<String>, tokens: &[String]) -> Result<Self> {
        let mut fields = Vec::new();
        let mut relations = Vec::new();
        for token in tokens {
            let parts: Vec<&str> = token.splitn(3, ':').collect();
            if parts.len() == 3 && parts[1] == "belongs_to" {
                let (field, target) = (parts[0], parts[2]);
                if field.is_empty() || target.is_empty() {
                    bail!("relationship `{token}` must be in the form name:belongs_to:target");
                }
                relations.push(Relation {
                    field: field.to_string(),
                    target: target.to_string(),
                });
            } else {
                fields.push(Field::parse(token)?);
            }
        }
        // Expand each relationship into its foreign-key column so the code templates render it.
        for r in &relations {
            fields.push(Field {
                name: r.fk_column(),
                ty: FieldType::Uuid,
            });
        }
        Ok(Self {
            name: name.into(),
            fields,
            relations,
        })
    }

    /// Serialize the scalar fields back into canonical `name:Type` tokens, for recording the
    /// model's shape in `gize.toml` (ADR-009 revision). Synthetic foreign-key columns are
    /// omitted — relationships are recorded separately under `[[module.belongs_to]]`.
    pub fn to_field_tokens(&self) -> Vec<String> {
        let fk: std::collections::BTreeSet<String> =
            self.relations.iter().map(Relation::fk_column).collect();
        self.fields
            .iter()
            .filter(|f| !fk.contains(&f.name))
            .map(Field::to_token)
            .collect()
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
        assert!(m.relations.is_empty());
    }

    #[test]
    fn parses_a_belongs_to_relationship() {
        let m = ModelSpec::parse(
            "Post",
            &[
                "title:String".to_string(),
                "author:belongs_to:users".to_string(),
            ],
        )
        .unwrap();
        // The relationship is recorded...
        assert_eq!(m.relations.len(), 1);
        assert_eq!(m.relations[0].field, "author");
        assert_eq!(m.relations[0].target, "users");
        assert_eq!(m.relations[0].fk_column(), "author_id");
        // ...and expanded into a synthetic UUID foreign-key field for codegen.
        let fk = m.fields.iter().find(|f| f.name == "author_id").unwrap();
        assert_eq!(fk.ty, FieldType::Uuid);
        // ...but the FK column is not re-emitted as a scalar field token.
        assert_eq!(m.to_field_tokens(), vec!["title:String".to_string()]);
    }

    #[test]
    fn rejects_malformed_relationship() {
        assert!(ModelSpec::parse("Post", &["author:belongs_to:".to_string()]).is_err());
        assert!(ModelSpec::parse("Post", &[":belongs_to:users".to_string()]).is_err());
    }
}
