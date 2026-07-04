//! A `Plan` is the set of file operations a generator wants to perform. Building the plan
//! is pure (no I/O), which makes generators easy to test and makes `--dry-run` trivial: we
//! simply render the plan instead of applying it.

use std::path::PathBuf;

/// What a single file operation intends to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Create a new file. If it already exists, the writer will skip it unless `--force`.
    Create,
    /// Create parent directories only (no file contents).
    Mkdir,
}

/// A single planned file operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOp {
    pub kind: OpKind,
    pub path: PathBuf,
    pub contents: String,
}

impl FileOp {
    pub fn create(path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self {
            kind: OpKind::Create,
            path: path.into(),
            contents: contents.into(),
        }
    }

    pub fn mkdir(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: OpKind::Mkdir,
            path: path.into(),
            contents: String::new(),
        }
    }
}

/// An ordered collection of [`FileOp`]s produced by a generator.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub ops: Vec<FileOp>,
}

impl Plan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(mut self, path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        self.ops.push(FileOp::create(path, contents));
        self
    }

    pub fn mkdir(mut self, path: impl Into<PathBuf>) -> Self {
        self.ops.push(FileOp::mkdir(path));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}
