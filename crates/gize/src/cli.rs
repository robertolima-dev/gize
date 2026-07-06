//! The `gize` command tree (ADR-012). Parsing only — each command delegates to a handler
//! in [`crate::commands`].

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "gize",
    version,
    about = "Productivity-first backend framework for Rust"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Flags shared by every generating command (ADR-012 safety model).
#[derive(Debug, Args, Clone, Copy, Default)]
pub struct GenFlags {
    /// Overwrite existing files instead of skipping them.
    #[arg(long, global = true)]
    pub force: bool,
    /// Show what would be written without touching the filesystem.
    #[arg(long, global = true)]
    pub dry_run: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scaffold a new project.
    New {
        /// Project name (also the directory created).
        name: String,
        /// Skip the built-in `users` resource (model, CRUD and migration).
        #[arg(long)]
        no_user: bool,
        /// Generate an OpenAPI spec (`/openapi.json`) and docs UI (`/docs`).
        #[arg(long)]
        openapi: bool,
        #[command(flatten)]
        flags: GenFlags,
    },
    /// Generate application pieces (app, model, crud, migration, admin).
    #[command(subcommand)]
    Make(MakeCommand),
    /// Apply pending database migrations.
    Migrate {
        /// Show applied/pending migrations instead of applying.
        #[arg(long)]
        status: bool,
    },
    /// Run the generated application.
    Serve,
    /// Reconcile the project from gize.toml.
    Sync {
        #[command(flatten)]
        flags: GenFlags,
    },
    /// Diagnose the environment and project.
    Doctor,
    /// Format the project (wrapper around rustfmt).
    Fmt,
    /// Check the project (wrapper around clippy/check).
    Check,
}

#[derive(Debug, Subcommand)]
pub enum MakeCommand {
    /// Create a new application module.
    App {
        name: String,
        #[command(flatten)]
        flags: GenFlags,
    },
    /// Generate a model from `field:Type` definitions.
    Model {
        /// Model name in PascalCase, e.g. `User`.
        name: String,
        /// Field definitions: `name:String email:String active:bool`.
        fields: Vec<String>,
        #[command(flatten)]
        flags: GenFlags,
    },
    /// Generate full CRUD (repository, service, dto, handlers, routes, tests) for a model.
    Crud {
        /// Model name in PascalCase, e.g. `Product`.
        name: String,
        /// Field definitions: `name:String price:i32 active:bool`.
        fields: Vec<String>,
        #[command(flatten)]
        flags: GenFlags,
    },
    /// Generate a database migration.
    Migration {
        /// Optional migration name.
        name: Option<String>,
        #[command(flatten)]
        flags: GenFlags,
    },
    /// Generate an admin interface for a model (Beta).
    Admin {
        name: String,
        #[command(flatten)]
        flags: GenFlags,
    },
}
