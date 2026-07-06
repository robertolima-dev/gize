//! Code generation engine for Gize.
//!
//! Two responsibilities:
//! 1. Turn [`gize_core`] specs into file contents (via [`gize_templates`]).
//! 2. Write those files **safely** — never clobbering user code without `--force`, and
//!    supporting `--dry-run` (ADR-012).

pub mod diff;
pub mod plan;
pub mod plugin;
pub mod registry;
pub mod sync;
pub mod writer;

pub mod scaffold;

pub use plan::{FileOp, OpKind, Plan};
pub use plugin::{GenContext, Generator};
pub use registry::{Edit, register_module};
pub use writer::{Options, Report, Writer};
