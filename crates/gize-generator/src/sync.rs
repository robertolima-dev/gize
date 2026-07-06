//! Reconciliation for `gize sync` (ADR-009): compare a desired [`Plan`] against the
//! filesystem and classify each file as **missing**, **drifted** (present but different), or
//! **unchanged**. Applying is conservative — missing files are created, but a drifted file is
//! never overwritten without `--force`, so hand edits are safe (roadmap risk: destructive
//! `sync`). Directories in the plan are implicit and ignored here.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::plan::{FileOp, OpKind, Plan};

/// The result of diffing a desired plan against what is on disk. Read-only: computing it
/// writes nothing.
#[derive(Debug, Default)]
pub struct Reconciliation {
    /// Files the plan wants that do not exist yet — safe to create.
    pub missing: Vec<FileOp>,
    /// Files that exist but whose contents differ from the plan — reported, not touched
    /// unless the caller opts into `--force`.
    pub drift: Vec<FileOp>,
    /// Files that already match the plan exactly.
    pub unchanged: Vec<String>,
}

impl Reconciliation {
    /// Whether there is nothing to create and nothing has drifted.
    pub fn is_in_sync(&self) -> bool {
        self.missing.is_empty() && self.drift.is_empty()
    }
}

/// Compare `plan` against the tree rooted at `root`, writing nothing.
pub fn reconcile(root: &Path, plan: &Plan) -> Result<Reconciliation> {
    let mut recon = Reconciliation::default();
    for op in &plan.ops {
        if op.kind != OpKind::Create {
            continue; // directories are created implicitly when their files are written
        }
        let path = root.join(&op.path);
        let display = op.path.display().to_string();
        if !path.exists() {
            recon.missing.push(op.clone());
        } else {
            let current =
                fs::read_to_string(&path).with_context(|| format!("reading {display}"))?;
            if current == op.contents {
                recon.unchanged.push(display);
            } else {
                recon.drift.push(op.clone());
            }
        }
    }
    Ok(recon)
}

/// What `apply` did (or, under `dry_run`, would do).
#[derive(Debug, Default)]
pub struct Applied {
    pub created: Vec<String>,
    pub overwritten: Vec<String>,
    /// Drifted files left untouched because `--force` was not given.
    pub left: Vec<String>,
    pub unchanged: Vec<String>,
}

impl Applied {
    /// A `git status`-style summary of the reconciliation.
    pub fn render(&self, dry_run: bool) -> String {
        if self.created.is_empty()
            && self.overwritten.is_empty()
            && self.left.is_empty()
            && self.unchanged.is_empty()
        {
            return "already in sync — nothing to do".to_string();
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
        for p in &self.left {
            out.push_str(&format!(
                "  drift   {p} (differs from manifest; use --force to overwrite)\n"
            ));
        }
        // Unchanged files are summarized, not listed — a synced project has many of them.
        if !self.unchanged.is_empty() {
            out.push_str(&format!(
                "  ok      {} file(s) already match the manifest\n",
                self.unchanged.len()
            ));
        }
        out
    }
}

/// Apply a reconciliation: always create missing files; overwrite drifted files only when
/// `force`. `dry_run` computes the same report but writes nothing.
pub fn apply(root: &Path, recon: &Reconciliation, force: bool, dry_run: bool) -> Result<Applied> {
    let mut applied = Applied {
        unchanged: recon.unchanged.clone(),
        ..Applied::default()
    };
    for op in &recon.missing {
        if !dry_run {
            write_file(root, op)?;
        }
        applied.created.push(op.path.display().to_string());
    }
    for op in &recon.drift {
        let display = op.path.display().to_string();
        if force {
            if !dry_run {
                write_file(root, op)?;
            }
            applied.overwritten.push(display);
        } else {
            applied.left.push(display);
        }
    }
    Ok(applied)
}

fn write_file(root: &Path, op: &FileOp) -> Result<()> {
    let path = root.join(&op.path);
    let display = op.path.display().to_string();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating parent for {display}"))?;
    }
    fs::write(&path, &op.contents).with_context(|| format!("writing {display}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gize-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn classifies_missing_drift_and_unchanged() {
        let root = tmpdir();
        fs::write(root.join("same.txt"), "v1").unwrap();
        fs::write(root.join("changed.txt"), "old").unwrap();
        let plan = Plan::new()
            .create("same.txt", "v1")
            .create("changed.txt", "new")
            .create("new.txt", "fresh");

        let recon = reconcile(&root, &plan).unwrap();
        assert_eq!(recon.unchanged, vec!["same.txt".to_string()]);
        assert_eq!(recon.missing.len(), 1);
        assert_eq!(recon.missing[0].path.display().to_string(), "new.txt");
        assert_eq!(recon.drift.len(), 1);
        assert!(!recon.is_in_sync());
    }

    #[test]
    fn apply_creates_missing_but_leaves_drift_without_force() {
        let root = tmpdir();
        fs::write(root.join("changed.txt"), "old").unwrap();
        let plan = Plan::new()
            .create("changed.txt", "new")
            .create("new.txt", "fresh");
        let recon = reconcile(&root, &plan).unwrap();

        let applied = apply(&root, &recon, false, false).unwrap();
        assert_eq!(applied.created, vec!["new.txt".to_string()]);
        assert_eq!(applied.left, vec!["changed.txt".to_string()]);
        // drift file untouched; missing file created
        assert_eq!(fs::read_to_string(root.join("changed.txt")).unwrap(), "old");
        assert_eq!(fs::read_to_string(root.join("new.txt")).unwrap(), "fresh");
    }

    #[test]
    fn force_overwrites_drift() {
        let root = tmpdir();
        fs::write(root.join("changed.txt"), "old").unwrap();
        let plan = Plan::new().create("changed.txt", "new");
        let recon = reconcile(&root, &plan).unwrap();

        let applied = apply(&root, &recon, true, false).unwrap();
        assert_eq!(applied.overwritten, vec!["changed.txt".to_string()]);
        assert_eq!(fs::read_to_string(root.join("changed.txt")).unwrap(), "new");
    }

    #[test]
    fn dry_run_writes_nothing() {
        let root = tmpdir();
        let plan = Plan::new().create("new.txt", "fresh");
        let recon = reconcile(&root, &plan).unwrap();
        let applied = apply(&root, &recon, false, true).unwrap();
        assert_eq!(applied.created, vec!["new.txt".to_string()]);
        assert!(!root.join("new.txt").exists());
    }
}
