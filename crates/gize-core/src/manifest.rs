//! The `gize.toml` project manifest (ADR-009).
//!
//! This is the declarative source of truth for a Gize project's shape. It is owned by the
//! CLI and drives `gize sync`. Runtime configuration (DB URL, secrets) deliberately lives
//! in the environment, not here.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub project: Project,
    #[serde(default)]
    pub stack: Stack,
    #[serde(default)]
    pub features: Features,
    #[serde(default)]
    pub modules: Modules,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stack {
    pub framework: String,
    pub database: String,
    pub orm: String,
}

impl Default for Stack {
    fn default() -> Self {
        // MVP defaults (ADR-002 / ADR-003).
        Self {
            framework: "axum".to_string(),
            database: "postgres".to_string(),
            orm: "sqlx".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Features {
    #[serde(default)]
    pub authentication: bool,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub openapi: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Modules {
    #[serde(default)]
    pub list: Vec<String>,
}

impl Manifest {
    /// Create a fresh manifest for a new project with MVP defaults.
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            project: Project {
                name: project_name.into(),
            },
            stack: Stack::default(),
            features: Features::default(),
            modules: Modules::default(),
        }
    }

    /// Parse a manifest from TOML text.
    pub fn from_toml(text: &str) -> Result<Self> {
        toml::from_str(text).context("failed to parse gize.toml")
    }

    /// Serialize the manifest to TOML text.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("failed to serialize manifest")
    }

    /// Register a module, keeping the list sorted and unique. Returns `true` if it was
    /// newly added (idempotent — ADR-012 safety model).
    pub fn add_module(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.modules.list.contains(&name) {
            return false;
        }
        self.modules.list.push(name);
        self.modules.list.sort();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_toml() {
        let mut m = Manifest::new("shop");
        m.features.admin = true;
        m.add_module("products");
        let text = m.to_toml().unwrap();
        let parsed = Manifest::from_toml(&text).unwrap();
        assert_eq!(m, parsed);
    }

    #[test]
    fn adding_module_is_idempotent() {
        let mut m = Manifest::new("shop");
        assert!(m.add_module("users"));
        assert!(!m.add_module("users"));
        assert_eq!(m.modules.list, vec!["users".to_string()]);
    }

    #[test]
    fn parses_minimal_manifest() {
        let text = r#"
            [project]
            name = "blog"
        "#;
        let m = Manifest::from_toml(text).unwrap();
        assert_eq!(m.project.name, "blog");
        // defaults fill in the stack
        assert_eq!(m.stack.framework, "axum");
    }
}
