//! Command handlers. These translate parsed CLI input into generator plans and apply them
//! through the safe [`Writer`].

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use gize_core::naming::{snake_case, table_name};
use gize_core::{Manifest, ModelSpec, Module};
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

/// A high-resolution, monotonic stamp for migration filenames and sqlx versions.
///
/// Nanoseconds since the Unix epoch. Second-resolution stamps collided when two resources
/// were generated within the same second (two migrations sharing one sqlx version — a
/// duplicate-key error on `migrate`); nanoseconds make each invocation's stamp distinct.
/// It also stays strictly greater than any earlier (including second-based, 0.5.0) stamp,
/// so migration ordering is preserved. A calendar-formatted stamp lands with the
/// migration-diffing work.
fn migration_timestamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:020}")
}

/// `gize new <name>` — scaffold a project into a new directory named `name`. Unless
/// `no_user` is set, a built-in `users` resource (model, CRUD, migration with an `is_admin`
/// flag) is generated and wired in.
pub fn new_project(name: &str, no_user: bool, flags: GenFlags) -> Result<()> {
    let root = Path::new(name);
    if root.exists() && !flags.force {
        bail!("directory `{name}` already exists (use --force to generate into it)");
    }

    let plan = scaffold::new_project(name, !no_user, &migration_timestamp());
    let report = Writer::new(flags.into())
        .apply(root, &plan)
        .with_context(|| format!("scaffolding project `{name}`"))?;

    println!(
        "Created project `{name}`:\n{}",
        report.render(flags.dry_run)
    );
    if !flags.dry_run {
        if !no_user {
            println!(
                "\nIncludes a `users` resource (id, name, email, password, is_admin). \
                 Set DATABASE_URL, then `gize migrate` to create the table."
            );
        }
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

/// Register a module by name only (`gize make app`). Add-only: never clobbers a module that
/// already carries a declared shape (e.g. one created by `make crud`).
fn register_in_manifest(module: &str, flags: GenFlags) -> Result<()> {
    let source = fs::read_to_string("gize.toml").context("reading gize.toml")?;
    let mut manifest = Manifest::from_toml(&source)?;

    if !manifest.add_module(module) {
        println!("  skip    gize.toml (module already listed)");
        return Ok(());
    }
    write_manifest(&manifest, flags, "added module")
}

/// Record a module's full shape (name + fields) in the manifest (`gize make crud`). Upserts,
/// so re-running with changed fields refreshes the recorded shape (ADR-009 revision).
fn record_module_in_manifest(module: Module, flags: GenFlags) -> Result<()> {
    let source = fs::read_to_string("gize.toml").context("reading gize.toml")?;
    let mut manifest = Manifest::from_toml(&source)?;

    if manifest.module(&module.name) == Some(&module) {
        println!("  skip    gize.toml (module already up to date)");
        return Ok(());
    }
    let existed = manifest.module(&module.name).is_some();
    manifest.upsert_module(module);
    write_manifest(
        &manifest,
        flags,
        if existed {
            "updated module"
        } else {
            "added module"
        },
    )
}

/// Persist a manifest, honoring `--dry-run`, with a consistent one-line report.
fn write_manifest(manifest: &Manifest, flags: GenFlags, what: &str) -> Result<()> {
    if flags.dry_run {
        println!("  update  gize.toml (would record module)");
    } else {
        fs::write("gize.toml", manifest.to_toml()?).context("writing gize.toml")?;
        println!("  update  gize.toml ({what})");
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
    record_module_in_manifest(
        Module {
            name: module.clone(),
            fields: model.to_field_tokens(),
            belongs_to: Vec::new(),
        },
        flags,
    )?;

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

/// `gize make migration [name]` — generate a blank, timestamped SQL migration to fill in by
/// hand (ADR-011). Model-driven `CREATE TABLE` files come from `make model`/`make crud`; this
/// is the escape hatch for everything else (indexes, constraints, backfills).
pub fn make_migration(name: Option<&str>, flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    let migration_name = slugify(name.unwrap_or("migration"));
    if migration_name.is_empty() {
        bail!("migration name must contain at least one letter or digit");
    }

    let plan = scaffold::make_migration(&migration_name, &migration_timestamp());
    let report = Writer::new(flags.into())
        .apply(Path::new("."), &plan)
        .context("generating migration")?;
    println!(
        "Generated migration `{migration_name}`:\n{}",
        report.render(flags.dry_run)
    );
    if !flags.dry_run {
        println!("\nEdit the SQL, then apply it with:\n  gize migrate");
    }
    Ok(())
}

/// `gize fmt` — format the project with rustfmt (thin wrapper; ADR-012).
pub fn fmt() -> Result<()> {
    ensure_in_project()?;
    run_cargo(&["fmt", "--all"], "cargo fmt")
}

/// `gize check` — lint the project with clippy, denying warnings (thin wrapper; ADR-012).
pub fn check() -> Result<()> {
    ensure_in_project()?;
    run_cargo(
        &["clippy", "--all-targets", "--", "-D", "warnings"],
        "cargo clippy",
    )
}

/// Turn a free-text migration name into a filename-safe snake_case slug. Handles both
/// PascalCase (`AddIndexToUsers`) and loose text (`add index to users`) by snake-casing first,
/// then collapsing every run of non-alphanumeric characters into a single `_`.
fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_sep = false;
    for ch in snake_case(input).chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('_');
            }
            out.push(ch);
            pending_sep = false;
        } else {
            pending_sep = true;
        }
    }
    out
}

/// Run a `cargo` subcommand inheriting stdio, mapping a non-zero exit into an error.
fn run_cargo(args: &[&str], label: &str) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .args(args)
        .status()
        .with_context(|| format!("failed to launch `{label}` — is cargo installed?"))?;
    if !status.success() {
        bail!("`{label}` exited with a non-zero status");
    }
    Ok(())
}

/// `gize migrate [--status]` — apply pending SQL migrations (ADR-011), or report state.
pub fn migrate(show_status: bool) -> Result<()> {
    ensure_in_project()?;
    let database_url = std::env::var("DATABASE_URL").context(
        "DATABASE_URL must be set — in your environment or a project `.env` \
         (e.g. postgres://user:pass@localhost:5432/dbname)",
    )?;
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

    report("cargo available", which("cargo"));
    report("rustfmt available", which("rustfmt"));
    report(
        "inside a gize project (gize.toml)",
        Path::new("gize.toml").exists(),
    );
    report("`.env` file present", Path::new(".env").exists());
    // `.env` is auto-loaded at startup, so this reflects the effective value.
    report("DATABASE_URL set", std::env::var("DATABASE_URL").is_ok());

    Ok(())
}

fn report(label: &str, ok: bool) {
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

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_handles_pascal_case_and_loose_text() {
        assert_eq!(slugify("AddIndexToUsers"), "add_index_to_users");
        assert_eq!(slugify("add index to users"), "add_index_to_users");
        assert_eq!(slugify("  Add  Index  "), "add_index");
        assert_eq!(slugify("add-index_to.users"), "add_index_to_users");
        assert_eq!(slugify("migration"), "migration");
        assert_eq!(slugify("!!!"), "");
    }
}
