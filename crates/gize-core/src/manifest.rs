//! The `gize.toml` project manifest (ADR-009).
//!
//! This is the declarative source of truth for a Gize project's shape. It is owned by the
//! CLI and drives `gize sync`. Runtime configuration (DB URL, secrets) deliberately lives
//! in the environment, not here.
//!
//! Since the Alpha (ADR-009 revision) the manifest is *rich*: each module records its
//! fields and relationships, so the project can be reconciled (`gize sync`) and rebuilt from
//! the manifest alone. The legacy `[modules] list = [...]` form (names only) is still read
//! for backward compatibility and rewritten into the `[[module]]` form on the next write.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::naming::{model_name, table_name};
use crate::{ModelSpec, Relation};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub project: Project,
    #[serde(default)]
    pub stack: Stack,
    #[serde(default)]
    pub features: Features,
    /// Optional API route versioning (ADR-016). Absent for unversioned projects, so their
    /// `gize.toml` stays clean and their router output is byte-identical to before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<Api>,
    /// Modules that make up the app, in the rich `[[module]]` form. Kept sorted by name for
    /// deterministic output. Skipped when empty so a bare project has a clean manifest.
    #[serde(default, rename = "module", skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<Module>,
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

/// API route versioning for the project (ADR-016). Its presence turns root-mounted CRUD
/// routes (`/products`) into versioned ones (`/api/v1/products`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Api {
    /// The normalized version segment, e.g. `v1`.
    pub version: String,
    /// The prefix the versioned API is mounted under (default `/api`).
    pub prefix: String,
}

impl Api {
    /// The default prefix a versioned API mounts under.
    pub const DEFAULT_PREFIX: &'static str = "/api";

    /// Build from a raw `--api-version` value, normalizing `1` and `v1` both to `v1`.
    pub fn from_version(raw: &str) -> Self {
        let raw = raw.trim();
        let digits = raw.strip_prefix(['v', 'V']).unwrap_or(raw);
        Self {
            version: format!("v{digits}"),
            prefix: Self::DEFAULT_PREFIX.to_string(),
        }
    }

    /// Like [`Api::from_version`], but only `1`, `v1`, `V2`… are accepted: the version becomes a
    /// URL path segment, so `abc` or `v1/../x` is an error rather than a route.
    pub fn parse_version(raw: &str) -> Result<Self> {
        let api = Self::from_version(raw);
        anyhow::ensure!(
            is_version_segment(&api.version),
            "invalid API version `{}` (use a number, e.g. `--api-version 1` or `v1`)",
            raw.trim()
        );
        Ok(api)
    }

    /// The full mount path the app is nested under, e.g. `/api/v1`.
    pub fn mount_path(&self) -> String {
        format!("{}/{}", self.prefix.trim_end_matches('/'), self.version)
    }
}

/// `v` followed by one or more ASCII digits, as [`Api::from_version`] normalizes it.
fn is_version_segment(version: &str) -> bool {
    version
        .strip_prefix('v')
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// `/` or `/seg/seg…`, each segment ASCII letters, digits, `-` or `_`.
fn is_mount_prefix(prefix: &str) -> bool {
    prefix.strip_prefix('/').is_some_and(|rest| {
        rest.trim_end_matches('/').is_empty()
            || rest.trim_end_matches('/').split('/').all(|seg| {
                !seg.is_empty()
                    && seg
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            })
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Features {
    #[serde(default)]
    pub authentication: bool,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub openapi: bool,
    #[serde(default)]
    pub websocket: bool,
}

/// One application module as recorded in `gize.toml` (ADR-009 revision).
///
/// `name` is the module/table name as used on disk (snake_case, e.g. `users`, `products`).
/// `model` is the struct name, recorded only when it cannot be derived back from `name`
/// (`Movie` -> `movies` would read back as `Movy`); see [`Module::model_name`].
/// `fields` reuse the CLI's `name:Type` grammar verbatim (see [`crate::field`]) so the
/// manifest and the command line share one definition of a model. `belongs_to` records the
/// module's 1-N relationships (ADR-014).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Module {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub belongs_to: Vec<Relation>,
}

impl Module {
    /// A module with just a name and no declared shape (used by `gize make app`, and when
    /// upgrading a legacy names-only manifest).
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: None,
            fields: Vec::new(),
            belongs_to: Vec::new(),
        }
    }

    /// The module recorded for `model`, as `gize make crud` writes it to the manifest. The
    /// struct name is stored only when deriving it from the table name would not give it back,
    /// so manifests of models that round-trip (`Post` -> `posts`) stay as they were.
    pub fn for_model(model: &ModelSpec) -> Self {
        let name = table_name(&model.name);
        let derived = model_name(&name);
        Self {
            model: (derived != model.name).then(|| model.name.clone()),
            name,
            fields: model.to_field_tokens(),
            belongs_to: model.relations.clone(),
        }
    }

    /// The model's struct name: the recorded `model`, or else derived from the module/table
    /// name (`posts` -> `Post`), which is what manifests written before `model` existed rely on.
    pub fn model_name(&self) -> String {
        self.model.clone().unwrap_or_else(|| model_name(&self.name))
    }

    /// Recover the model definition this module describes, for regenerating its code during
    /// `gize sync` (ADR-009 revision). The struct name comes from [`Module::model_name`]; the
    /// fields are re-parsed from their `name:Type` tokens.
    pub fn model_spec(&self) -> Result<ModelSpec> {
        crate::validation::validate_identifier(&self.name, "module")?;
        // Feed the scalar field tokens plus a `name:belongs_to:target` token per relationship,
        // so `ModelSpec::parse` reconstructs the same fields (incl. FK columns) it produced
        // at `make crud` time.
        let mut tokens = self.fields.clone();
        tokens.extend(self.belongs_to.iter().map(Relation::to_token));
        ModelSpec::parse(self.model_name(), &tokens)
            .with_context(|| format!("module `{}` has an invalid field definition", self.name))
    }
}

/// The legacy `[modules] list = [...]` table (names only), read for backward compatibility.
#[derive(Debug, Clone, Deserialize)]
struct LegacyModules {
    #[serde(default)]
    list: Vec<String>,
}

/// Wire format for parsing: accepts both the rich `[[module]]` array and the legacy
/// `[modules]` table so old manifests keep loading (ADR-009 revision).
#[derive(Debug, Deserialize)]
struct RawManifest {
    project: Project,
    #[serde(default)]
    stack: Stack,
    #[serde(default)]
    features: Features,
    #[serde(default)]
    api: Option<Api>,
    #[serde(default, rename = "module")]
    module: Vec<Module>,
    #[serde(default)]
    modules: Option<LegacyModules>,
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
            api: None,
            modules: Vec::new(),
        }
    }

    /// Parse a manifest from TOML text, accepting both the rich and legacy module forms.
    pub fn from_toml(text: &str) -> Result<Self> {
        let raw: RawManifest = toml::from_str(text).context("failed to parse gize.toml")?;
        // Prefer the rich `[[module]]` form; fall back to the legacy names-only list.
        let mut modules = raw.module;
        if modules.is_empty() {
            if let Some(legacy) = raw.modules {
                modules = legacy.list.into_iter().map(Module::named).collect();
            }
        }
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        let manifest = Self {
            project: raw.project,
            stack: raw.stack,
            features: raw.features,
            api: raw.api,
            modules,
        };
        manifest.validate_stack_and_api()?;
        Ok(manifest)
    }

    /// Reject `[stack]` and `[api]` values Gize would otherwise misread: an unknown database
    /// would generate PostgreSQL code, and a framework or ORM other than Axum/SQLx would be
    /// ignored. Checked on every load, so no command runs against such a manifest.
    fn validate_stack_and_api(&self) -> Result<()> {
        crate::Dialect::parse(&self.stack.database).context("in gize.toml `[stack] database`")?;
        anyhow::ensure!(
            self.stack.framework.eq_ignore_ascii_case("axum"),
            "unsupported framework `{}` in gize.toml `[stack]` (Gize 1.x generates Axum only)",
            self.stack.framework
        );
        anyhow::ensure!(
            self.stack.orm.eq_ignore_ascii_case("sqlx"),
            "unsupported orm `{}` in gize.toml `[stack]` (Gize 1.x generates SQLx only)",
            self.stack.orm
        );
        if let Some(api) = &self.api {
            anyhow::ensure!(
                is_version_segment(&api.version),
                "invalid `[api] version = \"{}\"` in gize.toml (expected `v` and a number, e.g. `v1`)",
                api.version
            );
            anyhow::ensure!(
                is_mount_prefix(&api.prefix),
                "invalid `[api] prefix = \"{}\"` in gize.toml (expected a path such as `/api`)",
                api.prefix
            );
        }
        Ok(())
    }

    /// Validate every module before any consumer can write a partial generation plan.
    pub fn validate(&self) -> Result<()> {
        let mut names = std::collections::BTreeSet::new();
        for module in &self.modules {
            anyhow::ensure!(
                names.insert(&module.name),
                "duplicate module `{}`",
                module.name
            );
            crate::validation::validate_identifier(&module.name, "module")?;
            if let Some(model) = &module.model {
                // The templates derive the table, routes and paths from the struct name, so a
                // recorded name must lead back to this module.
                anyhow::ensure!(
                    table_name(model) == module.name,
                    "module `{}` in gize.toml records model `{model}`, whose table would be `{}`; \
                     the model of `{}` must be named so its table is `{}`",
                    module.name,
                    table_name(model),
                    module.name,
                    module.name
                );
            }
            if !module.fields.is_empty() || !module.belongs_to.is_empty() {
                let dialect = crate::Dialect::from_database(&self.stack.database);
                // Every generation command validates the whole manifest, so an older module
                // can block work on a new one: say which module and how to get unblocked.
                crate::validation::validate_sql_identifier_for_dialect(
                    &module.name,
                    "table",
                    dialect,
                )
                .and_then(|()| module.model_spec()?.validate_for_dialect(dialect))
                .with_context(|| {
                    format!(
                        "module `{}` in gize.toml cannot be generated; rename the name reported \
                         below in gize.toml (and rename an existing table/column with a named migration: \
                         `gize make migration <name>`)",
                        module.name
                    )
                })?;
            }
        }
        Ok(())
    }

    /// Serialize the manifest to TOML text (always in the rich `[[module]]` form).
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("failed to serialize manifest")
    }

    /// Find a module by name.
    pub fn module(&self, name: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Register a module by name only, keeping the list sorted and unique. Returns `true` if
    /// it was newly added (idempotent — ADR-012 safety model). Used by `gize make app`, which
    /// has no fields yet. Does not touch an existing module's declared shape.
    pub fn add_module(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.modules.iter().any(|m| m.name == name) {
            return false;
        }
        self.modules.push(Module::named(name));
        self.modules.sort_by(|a, b| a.name.cmp(&b.name));
        true
    }

    /// The modules ordered so every `belongs_to` target comes before the module that
    /// references it (ADR-014). Used when creating migrations so a foreign key's target table
    /// already exists. Errors on a dependency cycle. Targets that are not themselves modules
    /// (pre-existing tables) impose no ordering.
    pub fn modules_in_dependency_order(&self) -> Result<Vec<&Module>> {
        use std::collections::{HashMap, HashSet};

        let by_name: HashMap<&str, &Module> =
            self.modules.iter().map(|m| (m.name.as_str(), m)).collect();
        let mut ordered = Vec::new();
        let mut done: HashSet<&str> = HashSet::new();
        let mut on_stack: HashSet<&str> = HashSet::new();

        fn visit<'a>(
            m: &'a Module,
            by_name: &HashMap<&str, &'a Module>,
            done: &mut HashSet<&'a str>,
            on_stack: &mut HashSet<&'a str>,
            ordered: &mut Vec<&'a Module>,
        ) -> Result<()> {
            if done.contains(m.name.as_str()) {
                return Ok(());
            }
            if !on_stack.insert(m.name.as_str()) {
                anyhow::bail!("cyclic belongs_to relationship involving `{}`", m.name);
            }
            for r in &m.belongs_to {
                if r.target != m.name {
                    if let Some(target) = by_name.get(r.target.as_str()) {
                        visit(target, by_name, done, on_stack, ordered)?;
                    }
                }
            }
            on_stack.remove(m.name.as_str());
            done.insert(m.name.as_str());
            ordered.push(m);
            Ok(())
        }

        for m in &self.modules {
            visit(m, &by_name, &mut done, &mut on_stack, &mut ordered)?;
        }
        Ok(ordered)
    }

    /// Insert or replace a module's full declaration (name + fields + relationships),
    /// keeping the list sorted. Returns `true` if the module was newly added, `false` if an
    /// existing entry was replaced. Used by `gize make model`/`make crud`, which know the
    /// module's shape, and by `gize sync`.
    pub fn upsert_module(&mut self, module: Module) -> bool {
        match self.modules.iter_mut().find(|m| m.name == module.name) {
            Some(existing) => {
                *existing = module;
                false
            }
            None => {
                self.modules.push(module);
                self.modules.sort_by(|a, b| a.name.cmp(&b.name));
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_toml() {
        let mut m = Manifest::new("shop");
        m.features.admin = true;
        m.upsert_module(Module {
            name: "products".to_string(),
            model: None,
            fields: vec!["name:String".to_string(), "price:i32".to_string()],
            belongs_to: vec![],
        });
        let text = m.to_toml().unwrap();
        let parsed = Manifest::from_toml(&text).unwrap();
        assert_eq!(m, parsed);
    }

    #[test]
    fn roundtrips_a_relationship() {
        let mut m = Manifest::new("blog");
        m.upsert_module(Module {
            name: "posts".to_string(),
            model: None,
            fields: vec!["title:String".to_string()],
            belongs_to: vec![Relation {
                field: "author".to_string(),
                target: "users".to_string(),
            }],
        });
        let text = m.to_toml().unwrap();
        assert!(text.contains("[[module.belongs_to]]"));
        assert_eq!(m, Manifest::from_toml(&text).unwrap());
    }

    #[test]
    fn adding_module_is_idempotent() {
        let mut m = Manifest::new("shop");
        assert!(m.add_module("users"));
        assert!(!m.add_module("users"));
        assert_eq!(m.modules.len(), 1);
        assert_eq!(m.modules[0].name, "users");
    }

    #[test]
    fn upsert_replaces_shape_without_duplicating() {
        let mut m = Manifest::new("shop");
        assert!(m.add_module("products")); // empty shape first
        assert!(!m.upsert_module(Module {
            name: "products".to_string(),
            model: None,
            fields: vec!["name:String".to_string()],
            belongs_to: vec![],
        }));
        assert_eq!(m.modules.len(), 1);
        assert_eq!(m.modules[0].fields, vec!["name:String".to_string()]);
    }

    #[test]
    fn reads_legacy_names_only_manifest() {
        // A gize.toml written before the ADR-009 revision.
        let text = r#"
            [project]
            name = "shop"

            [modules]
            list = ["products", "users"]
        "#;
        let m = Manifest::from_toml(text).unwrap();
        assert_eq!(m.modules.len(), 2);
        // Sorted, and carried as empty-shaped modules.
        assert_eq!(m.modules[0].name, "products");
        assert_eq!(m.modules[1].name, "users");
        assert!(m.modules[0].fields.is_empty());
        // Re-serializing upgrades it to the rich form (no more `[modules] list`).
        let upgraded = m.to_toml().unwrap();
        assert!(upgraded.contains("[[module]]"));
        assert!(!upgraded.contains("list ="));
    }

    #[test]
    fn orders_modules_so_targets_precede_dependents() {
        let mut m = Manifest::new("blog");
        // Declared out of dependency order and alphabetically: comments -> posts -> users.
        m.upsert_module(Module {
            name: "comments".to_string(),
            model: None,
            fields: vec!["body:String".to_string()],
            belongs_to: vec![Relation {
                field: "post".to_string(),
                target: "posts".to_string(),
            }],
        });
        m.upsert_module(Module {
            name: "posts".to_string(),
            model: None,
            fields: vec!["title:String".to_string()],
            belongs_to: vec![Relation {
                field: "author".to_string(),
                target: "users".to_string(),
            }],
        });
        m.upsert_module(Module::named("users"));

        let ordered: Vec<&str> = m
            .modules_in_dependency_order()
            .unwrap()
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        let pos = |n: &str| ordered.iter().position(|x| *x == n).unwrap();
        assert!(
            pos("users") < pos("posts"),
            "users before posts: {ordered:?}"
        );
        assert!(
            pos("posts") < pos("comments"),
            "posts before comments: {ordered:?}"
        );
    }

    #[test]
    fn dependency_ordering_detects_cycles() {
        let mut m = Manifest::new("loop");
        m.upsert_module(Module {
            name: "a".to_string(),
            model: None,
            fields: vec![],
            belongs_to: vec![Relation {
                field: "b".to_string(),
                target: "b".to_string(),
            }],
        });
        m.upsert_module(Module {
            name: "b".to_string(),
            model: None,
            fields: vec![],
            belongs_to: vec![Relation {
                field: "a".to_string(),
                target: "a".to_string(),
            }],
        });
        assert!(m.modules_in_dependency_order().is_err());
    }

    #[test]
    fn api_version_normalizes_and_builds_mount_path() {
        // Both `1` and `v1` normalize to the same `v1` segment.
        assert_eq!(Api::from_version("1").version, "v1");
        assert_eq!(Api::from_version("v1").version, "v1");
        assert_eq!(Api::from_version("V2").version, "v2");
        assert_eq!(Api::from_version(" 3 ").version, "v3");
        assert_eq!(Api::from_version("1").mount_path(), "/api/v1");
    }

    #[test]
    fn api_version_must_be_a_number() {
        assert_eq!(Api::parse_version("v2").unwrap().version, "v2");
        for bad in ["abc", "v", "", "v1/../x", "1.0", "v1beta"] {
            assert!(Api::parse_version(bad).is_err(), "`{bad}` was accepted");
        }
    }

    #[test]
    fn loading_rejects_stack_and_api_values_gize_would_misread() {
        let load = |extra: &str| Manifest::from_toml(&format!("[project]\nname='a'\n{extra}"));
        assert!(load("").is_ok());
        assert!(load("[stack]\nframework='axum'\ndatabase='SQLite'\norm='sqlx'").is_ok());
        assert!(load("[api]\nversion='v1'\nprefix='/api'").is_ok());
        assert!(load("[api]\nversion='v1'\nprefix='/'").is_ok());

        let err = |extra: &str| format!("{:#}", load(extra).unwrap_err());
        let typo = err("[stack]\nframework='axum'\ndatabase='sqllite'\norm='sqlx'");
        assert!(typo.contains("unknown database `sqllite`"), "{typo}");
        assert!(
            err("[stack]\nframework='actix'\ndatabase='postgres'\norm='sqlx'").contains("actix")
        );
        assert!(
            err("[stack]\nframework='axum'\ndatabase='postgres'\norm='diesel'").contains("diesel")
        );
        assert!(err("[api]\nversion='vabc'\nprefix='/api'").contains("vabc"));
        for prefix in ["api", "/api/../x", "/a b", ""] {
            assert!(
                load(&format!("[api]\nversion='v1'\nprefix='{prefix}'")).is_err(),
                "prefix `{prefix}` was accepted"
            );
        }
    }

    #[test]
    fn api_section_roundtrips_and_is_omitted_when_absent() {
        // Present: the `[api]` table survives a round-trip.
        let mut m = Manifest::new("shop");
        m.api = Some(Api::from_version("1"));
        let text = m.to_toml().unwrap();
        assert!(text.contains("[api]"));
        assert!(text.contains("version = \"v1\""));
        assert_eq!(m, Manifest::from_toml(&text).unwrap());

        // Absent: no `[api]` table is written, keeping unversioned manifests clean.
        let bare = Manifest::new("shop");
        assert!(!bare.to_toml().unwrap().contains("[api]"));
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
        assert!(m.modules.is_empty());
    }

    #[test]
    fn empty_project_omits_modules_section() {
        let m = Manifest::new("empty");
        let text = m.to_toml().unwrap();
        assert!(!text.contains("[[module]]"));
        assert!(!text.contains("[modules]"));
    }
    #[test]
    fn loads_legacy_modules_but_rejects_invalid_generation() {
        for body in [
            "[[module]]\nname='posts'\nfields=['id:String']",
            "[[module]]\nname='posts'\nfields=['name:String','name:i32']",
            "[[module]]\nname='posts'\nfields=['author_id:Uuid']\n[[module.belongs_to]]\nfield='author'\ntarget='users'",
            "[[module]]\nname='posts'\n[[module.belongs_to]]\nfield='author'\ntarget='../users'",
            "[[module]]\nname='../posts'",
            "[[module]]\nname='posts'\n[[module]]\nname='posts'",
            "[modules]\nlist=['mod']",
            "[[module]]\nname='movies'\nmodel='Film'\nfields=['title:String']",
            "[[module]]\nname='movies'\nmodel='movie'\nfields=['title:String']",
        ] {
            let text = format!("[project]\nname='test'\n{body}");
            let manifest = Manifest::from_toml(&text).unwrap();
            assert!(manifest.validate().is_err(), "accepted {body}");
            assert!(manifest.to_toml().is_ok());
        }
    }

    #[test]
    fn records_the_model_name_only_when_it_does_not_round_trip() {
        let spec = |name: &str| ModelSpec::parse(name, &["title:String".to_string()]).unwrap();

        // `movies` reads back as `Movy`, so the struct name is recorded and wins.
        let movie = Module::for_model(&spec("Movie"));
        assert_eq!(movie.name, "movies");
        assert_eq!(movie.model.as_deref(), Some("Movie"));
        assert_eq!(movie.model_spec().unwrap().name, "Movie");

        // Names that round-trip keep the manifest as it was before `model` existed.
        for name in ["Post", "Category", "Box", "BlogPost", "Status"] {
            let module = Module::for_model(&spec(name));
            assert_eq!(module.model, None, "{name}");
            assert_eq!(module.model_spec().unwrap().name, name);
        }

        let mut m = Manifest::new("films");
        m.upsert_module(movie.clone());
        let text = m.to_toml().unwrap();
        assert!(text.contains("model = \"Movie\""));
        let back = Manifest::from_toml(&text).unwrap();
        back.validate().unwrap();
        assert_eq!(back.module("movies"), Some(&movie));

        // A manifest written before `model` existed still derives the name.
        let legacy = Manifest::from_toml(
            "[project]\nname='blog'\n[[module]]\nname='posts'\nfields=['title:String']",
        )
        .unwrap();
        assert_eq!(legacy.modules[0].model_spec().unwrap().name, "Post");
    }
}
