//! `gize` — the Gize command-line interface.

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command, MakeCommand};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::New {
            name,
            no_user,
            flags,
        } => commands::new_project(&name, no_user, flags),

        Command::Make(make) => match make {
            MakeCommand::Model {
                name,
                fields,
                flags,
            } => commands::make_model(&name, &fields, flags),
            MakeCommand::App { name, flags } => commands::make_app(&name, flags),
            MakeCommand::Crud {
                name,
                fields,
                flags,
            } => commands::make_crud(&name, &fields, flags),
            MakeCommand::Migration { name, flags } => {
                commands::make_migration(name.as_deref(), flags)
            }
            MakeCommand::Admin { name, .. } => commands::not_yet(
                &format!("make admin {name}"),
                "generate an admin UI (List/Create/Edit/Show/Delete) for the model",
            ),
        },

        Command::Migrate { status } => commands::migrate(status),
        Command::Serve => commands::serve(),
        Command::Sync { .. } => commands::not_yet(
            "sync",
            "reconcile the project from gize.toml (idempotent, dry-run first)",
        ),
        Command::Doctor => commands::doctor(),
        Command::Fmt => commands::fmt(),
        Command::Check => commands::check(),
    }
}
