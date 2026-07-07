//! `gize` — the Gize command-line interface.

mod cli;
mod commands;
mod password;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command, MakeCommand};

fn main() -> Result<()> {
    // Load a project-local `.env` (if present) before dispatching, so commands that read
    // configuration — `migrate`, `serve`, `doctor` — and any child process they spawn
    // (`serve` runs `cargo run`) see `DATABASE_URL`, `PORT`, etc. without a manual `export`.
    // A real environment variable always wins over a `.env` entry.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    match cli.command {
        Command::New {
            name,
            no_user,
            openapi,
            ws,
            database,
            api_version,
            flags,
        } => commands::new_project(
            &name,
            no_user,
            openapi,
            ws,
            &database,
            api_version.as_deref(),
            flags,
        ),

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
            MakeCommand::Admin { name, flags } => commands::make_admin(name.as_deref(), flags),
        },

        Command::Createadmin {
            email,
            name,
            password_env,
        } => commands::create_admin(email, name, password_env),
        Command::Migrate { status } => commands::migrate(status),
        Command::Serve => commands::serve(),
        Command::Sync { flags } => commands::sync(flags),
        Command::Doctor => commands::doctor(),
        Command::Fmt => commands::fmt(),
        Command::Check => commands::check(),
        Command::External(args) => commands::run_external(args),
    }
}
