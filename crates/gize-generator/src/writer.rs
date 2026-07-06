//! Applies a [`Plan`] to the filesystem, honouring the Gize safety model (ADR-012):
//! never overwrite an existing file unless `force` is set, and write nothing at all when
//! `dry_run` is set.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::plan::{OpKind, Plan};

/// Options controlling how a plan is applied.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Overwrite files that already exist.
    pub force: bool,
    /// Compute and report actions but touch no files.
    pub dry_run: bool,
}

/// What actually happened (or would happen) for each file in a plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub created: Vec<String>,
    pub overwritten: Vec<String>,
    pub skipped: Vec<String>,
}

impl Report {
    fn is_empty(&self) -> bool {
        self.created.is_empty() && self.overwritten.is_empty() && self.skipped.is_empty()
    }

    /// A human-readable, `git status`-style summary.
    pub fn render(&self, dry_run: bool) -> String {
        if self.is_empty() {
            return "nothing to do".to_string();
        }
        let mut out = String::new();
        if dry_run {
            out.push_str("dry-run: no files written\n");
        }
        for p in &self.created {
            out.push_str(&format!("  create  {p}\n"));
        }
        for p in &self.overwritten {
            out.push_str(&format!("  force   {p}\n"));
        }
        for p in &self.skipped {
            out.push_str(&format!(
                "  skip    {p} (exists; use --force to overwrite)\n"
            ));
        }
        out
    }
}

/// The safe file writer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Writer {
    opts: Options,
}

impl Writer {
    pub fn new(opts: Options) -> Self {
        Self { opts }
    }

    /// Apply a plan rooted at `root`. Relative op paths are resolved against `root`.
    pub fn apply(&self, root: &Path, plan: &Plan) -> Result<Report> {
        let mut report = Report::default();

        for op in &plan.ops {
            let path = root.join(&op.path);
            let display = op.path.display().to_string();

            match op.kind {
                OpKind::Mkdir => {
                    if !self.opts.dry_run {
                        fs::create_dir_all(&path)
                            .with_context(|| format!("creating directory {display}"))?;
                    }
                }
                OpKind::Create => {
                    let exists = path.exists();
                    if exists && !self.opts.force {
                        report.skipped.push(display);
                        continue;
                    }

                    if !self.opts.dry_run {
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(parent)
                                .with_context(|| format!("creating parent for {display}"))?;
                        }
                        fs::write(&path, &op.contents)
                            .with_context(|| format!("writing {display}"))?;
                    }

                    if exists {
                        report.overwritten.push(display);
                    } else {
                        report.created.push(display);
                    }
                }
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Plan;

    fn tmpdir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::temp_dir().join(format!("gize-writer-{}", std::process::id()));
        let unique = base.join(COUNTER.fetch_add(1, Ordering::Relaxed).to_string());
        fs::create_dir_all(&unique).unwrap();
        unique
    }

    #[test]
    fn creates_new_files() {
        let root = tmpdir();
        let plan = Plan::new().create("a.txt", "hello");
        let report = Writer::new(Options::default()).apply(&root, &plan).unwrap();
        assert_eq!(report.created, vec!["a.txt".to_string()]);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "hello");
    }

    #[test]
    fn skips_existing_without_force() {
        let root = tmpdir();
        fs::write(root.join("a.txt"), "original").unwrap();
        let plan = Plan::new().create("a.txt", "new");
        let report = Writer::new(Options::default()).apply(&root, &plan).unwrap();
        assert_eq!(report.skipped, vec!["a.txt".to_string()]);
        // untouched
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "original");
    }

    #[test]
    fn overwrites_with_force() {
        let root = tmpdir();
        fs::write(root.join("a.txt"), "original").unwrap();
        let plan = Plan::new().create("a.txt", "new");
        let opts = Options {
            force: true,
            dry_run: false,
        };
        let report = Writer::new(opts).apply(&root, &plan).unwrap();
        assert_eq!(report.overwritten, vec!["a.txt".to_string()]);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "new");
    }

    #[test]
    fn dry_run_writes_nothing() {
        let root = tmpdir();
        let plan = Plan::new().create("a.txt", "hello");
        let opts = Options {
            force: false,
            dry_run: true,
        };
        let report = Writer::new(opts).apply(&root, &plan).unwrap();
        assert_eq!(report.created, vec!["a.txt".to_string()]);
        assert!(!root.join("a.txt").exists());
    }
}
