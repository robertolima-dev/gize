//! Plugin API v0 (ADR-008) — **unstable**.
//!
//! A plugin is a [`Generator`]: given the project context and its arguments, it produces a
//! [`Plan`]. Applying the plan goes through the same safe [`Writer`] as the built-in
//! generators, so a third-party generator inherits the whole safety model for free (never
//! clobber without `--force`, honor `--dry-run`, manifest as source of truth). No plugin
//! writes files directly.
//!
//! Two integration paths (ADR-008):
//! - **In-process:** a crate implements [`Generator`] and either builds a custom `gize` or its
//!   own binary, calling [`run`].
//! - **Subcommand fallback:** a `gize-<name>` binary on `PATH` is invoked by `gize <name> …`.
//!
//! The trait and [`GenContext`] are **v0 and may change** until stabilized in the RC.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gize_core::Manifest;

use crate::plan::Plan;
use crate::writer::{Options, Report, Writer};

/// What a generator gets to work with: the project manifest and its root directory.
#[derive(Debug, Clone)]
pub struct GenContext {
    /// The parsed `gize.toml`.
    pub manifest: Manifest,
    /// The project root (paths in the returned [`Plan`] are relative to it).
    pub root: PathBuf,
}

impl GenContext {
    /// Build a context from a project rooted at `root` (reads `root/gize.toml`).
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let manifest_path = root.join("gize.toml");
        let text = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?;
        let manifest = Manifest::from_toml(&text)?;
        Ok(Self { manifest, root })
    }

    /// Build a context from the current directory (the common case for a CLI plugin).
    pub fn from_current_dir() -> Result<Self> {
        Self::from_root(Path::new("."))
    }
}

/// A third-party (or built-in) code generator. Implement this to extend `gize` (ADR-008, v0).
pub trait Generator {
    /// The subcommand name, e.g. `"healthcheck"` (invoked as `gize healthcheck …`).
    fn name(&self) -> &str;

    /// Build a [`Plan`] from the project context and the plugin's arguments. **Pure** — do no
    /// I/O here, so the plan stays testable and `--dry-run` works.
    fn plan(&self, ctx: &GenContext, args: &[String]) -> Result<Plan>;
}

/// Run a generator against the current project: build the context, ask it for a plan, and
/// apply that plan through the safe [`Writer`] (honoring `force`/`dry_run`). This is the entry
/// point a plugin binary calls from `main`.
pub fn run(generator: &dyn Generator, args: &[String], opts: Options) -> Result<Report> {
    let ctx = GenContext::from_current_dir().context("not a gize project here (no gize.toml)")?;
    let plan = generator
        .plan(&ctx, args)
        .with_context(|| format!("plugin `{}` failed to build its plan", generator.name()))?;
    Writer::new(opts).apply(&ctx.root, &plan)
}

/// Apply a generator's plan against an explicit context (useful for tests and custom hosts).
pub fn run_in(
    ctx: &GenContext,
    generator: &dyn Generator,
    args: &[String],
    opts: Options,
) -> Result<Report> {
    let plan = generator.plan(ctx, args)?;
    Writer::new(opts).apply(&ctx.root, &plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;
    impl Generator for Dummy {
        fn name(&self) -> &str {
            "dummy"
        }
        fn plan(&self, _ctx: &GenContext, _args: &[String]) -> Result<Plan> {
            Ok(Plan::new().create("generated/by_plugin.txt", "hello from a plugin\n"))
        }
    }

    #[test]
    fn a_plugin_generates_through_the_safe_writer() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "gize-plugin-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("gize.toml"), "[project]\nname = \"demo\"\n").unwrap();

        let ctx = GenContext::from_root(&root).unwrap();
        let report = run_in(&ctx, &Dummy, &[], Options::default()).unwrap();
        assert_eq!(report.created, vec!["generated/by_plugin.txt".to_string()]);
        assert!(root.join("generated/by_plugin.txt").is_file());

        // Re-running without --force skips (safety model inherited from the Writer).
        let again = run_in(&ctx, &Dummy, &[], Options::default()).unwrap();
        assert_eq!(again.skipped, vec!["generated/by_plugin.txt".to_string()]);
    }
}
