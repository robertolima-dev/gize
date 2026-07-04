//! Command handlers. These translate parsed CLI input into generator plans and apply them
//! through the safe [`Writer`].

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use gize_core::naming::{snake_case, table_name};
use gize_core::{Manifest, ModelSpec};
use gize_generator::{Options, Writer, registry, scaffold};

use crate::cli::GenFlags;

impl From<GenFlags> for Options {
    fn from(f: GenFlags) -> Self {
        Options {
            force: f.force,
            dry_run: f.dry_run,
        }
    }
}

/// A UTC-ish timestamp `YYYYMMDDHHMMSS` for migration filenames. Good enough for ordering
/// in the MVP; a proper chrono-based stamp lands with the migration diffing work.
fn migration_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Not calendar-accurate, but monotonic and unique enough for MVP ordering.
    format!("{secs:014}")
}

/// `gize new <name>` — scaffold a project into a new directory named `name`.
pub fn new_project(name: &str, flags: GenFlags) -> Result<()> {
    let root = Path::new(name);
    if root.exists() && !flags.force {
        bail!("directory `{name}` already exists (use --force to generate into it)");
    }

    let plan = scaffold::new_project(name);
    let report = Writer::new(flags.into())
        .apply(root, &plan)
        .with_context(|| format!("scaffolding project `{name}`"))?;

    println!(
        "Created project `{name}`:\n{}",
        report.render(flags.dry_run)
    );
    if !flags.dry_run {
        println!("Next:\n  cd {name}\n  cp .env.example .env\n  gize serve");
    }
    Ok(())
}

/// `gize make app <name>` — scaffold a module and wire it into `app/mod.rs` and
/// `gize.toml` (idempotently; ADR-004 / ADR-012).
pub fn make_app(name: &str, flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    let module = snake_case(name);

    // 1. Generate the module's own files through the safe writer.
    let plan = scaffold::make_app(&module);
    let report = Writer::new(flags.into())
        .apply(Path::new("."), &plan)
        .context("generating module files")?;
    println!(
        "Generated module `{module}`:\n{}",
        report.render(flags.dry_run)
    );

    // 2. Register the module + its routes in src/app/mod.rs (edit of an existing file).
    register_in_app_mod(&module, flags)?;

    // 3. Record the module in the manifest.
    register_in_manifest(&module, flags)?;

    if !flags.dry_run {
        println!("\nDefine its model with:\n  gize make model <Name> field:Type ... --force");
    }
    Ok(())
}

/// Insert `mod <module>;` and `.merge(<module>::routes())` into `src/app/mod.rs`.
fn register_in_app_mod(module: &str, flags: GenFlags) -> Result<()> {
    let path = Path::new("src/app/mod.rs");
    let source = fs::read_to_string(path).context("reading src/app/mod.rs")?;
    let edit = registry::register_module(&source, module)?;

    if !edit.changed {
        println!("  skip    src/app/mod.rs (module already registered)");
        return Ok(());
    }
    if flags.dry_run {
        println!("  update  src/app/mod.rs (would register module + routes)");
    } else {
        fs::write(path, edit.content).context("writing src/app/mod.rs")?;
        println!("  update  src/app/mod.rs (registered module + routes)");
    }
    Ok(())
}

/// Add the module to the manifest's `[modules]` list.
fn register_in_manifest(module: &str, flags: GenFlags) -> Result<()> {
    let source = fs::read_to_string("gize.toml").context("reading gize.toml")?;
    let mut manifest = Manifest::from_toml(&source)?;

    if !manifest.add_module(module) {
        println!("  skip    gize.toml (module already listed)");
        return Ok(());
    }
    if flags.dry_run {
        println!("  update  gize.toml (would add module to [modules])");
    } else {
        fs::write("gize.toml", manifest.to_toml()?).context("writing gize.toml")?;
        println!("  update  gize.toml (added module to [modules])");
    }
    Ok(())
}

/// `gize make crud <Name> field:Type ...` — generate a full, wired CRUD resource.
pub fn make_crud(name: &str, fields: &[String], flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    let model = ModelSpec::parse(name, fields).context("invalid model definition")?;
    if model.fields.is_empty() {
        bail!(
            "`gize make crud` needs at least one field, e.g. \
             `gize make crud Product name:String price:i32`"
        );
    }
    let module = table_name(&model.name);

    let plan = scaffold::make_crud(&model, &migration_timestamp());
    let report = Writer::new(flags.into())
        .apply(Path::new("."), &plan)
        .context("generating CRUD")?;
    println!(
        "Generated CRUD for `{}`:\n{}",
        model.name,
        report.render(flags.dry_run)
    );

    register_in_app_mod(&module, flags)?;
    register_in_manifest(&module, flags)?;

    if !flags.dry_run {
        println!("\nApply the migration with:\n  gize migrate");
    }
    Ok(())
}

/// `gize make model <Name> field:Type ...` — generate a model + migration in the current
/// project.
pub fn make_model(name: &str, fields: &[String], flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    let model = ModelSpec::parse(name, fields).context("invalid model definition")?;
    let plan = scaffold::make_model(&model, &migration_timestamp());
    let report = Writer::new(flags.into())
        .apply(Path::new("."), &plan)
        .context("generating model")?;

    println!(
        "Generated model `{name}`:\n{}",
        report.render(flags.dry_run)
    );
    Ok(())
}

/// `gize migrate [--status]` — apply pending SQL migrations (ADR-011), or report state.
pub fn migrate(show_status: bool) -> Result<()> {
    ensure_in_project()?;
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set, e.g. postgres://user:pass@localhost:5432/dbname")?;
    let dir = Path::new("migrations");

    if show_status {
        let status = gize_db::migrate::status(&database_url, dir)?;
        if status.applied.is_empty() && status.pending.is_empty() {
            println!("No migrations found in ./migrations.");
            return Ok(());
        }
        println!("Applied:");
        if status.applied.is_empty() {
            println!("  (none)");
        }
        for m in &status.applied {
            println!("  [x] {m}");
        }
        println!("Pending:");
        if status.pending.is_empty() {
            println!("  (none)");
        }
        for m in &status.pending {
            println!("  [ ] {m}");
        }
        return Ok(());
    }

    let newly = gize_db::migrate::run(&database_url, dir)?;
    if newly.is_empty() {
        println!("Database is up to date — no pending migrations.");
    } else {
        println!("Applied {} migration(s):", newly.len());
        for m in newly {
            println!("  {m}");
        }
    }
    Ok(())
}

/// `gize serve` — build and run the generated application via `cargo run`.
pub fn serve() -> Result<()> {
    ensure_in_project()?;
    println!("Starting the application with `cargo run` (Ctrl-C to stop)...\n");
    let status = std::process::Command::new("cargo")
        .arg("run")
        .status()
        .context("failed to launch `cargo run` — is cargo installed?")?;
    if !status.success() {
        bail!("the application exited with a non-zero status");
    }
    Ok(())
}

/// `gize doctor` — sanity-check the environment and project.
pub fn doctor() -> Result<()> {
    println!("gize doctor\n");

    check("cargo available", which("cargo"));
    check("rustfmt available", which("rustfmt"));
    check(
        "inside a gize project (gize.toml)",
        Path::new("gize.toml").exists(),
    );
    check("DATABASE_URL set", std::env::var("DATABASE_URL").is_ok());

    Ok(())
}

fn check(label: &str, ok: bool) {
    let mark = if ok { "ok  " } else { "warn" };
    println!("  [{mark}] {label}");
}

fn which(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ensure_in_project() -> Result<()> {
    if !Path::new("gize.toml").exists() {
        bail!("not a gize project (no gize.toml here). Run `gize new <name>` first.");
    }
    Ok(())
}

/// Handlers not yet implemented in the MVP skeleton. They report the planned behaviour so
/// the command surface is complete and discoverable (see BACKLOG.md / roadmap).
pub fn not_yet(command: &str, planned: &str) -> Result<()> {
    println!("`gize {command}` is planned but not implemented in the MVP skeleton yet.");
    println!("Planned behaviour: {planned}");
    println!("Tracked in BACKLOG.md.");
    Ok(())
}
