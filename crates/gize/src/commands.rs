//! Command handlers. These translate parsed CLI input into generator plans and apply them
//! through the safe [`Writer`].

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use gize_core::naming::{snake_case, table_name};
use gize_core::{Manifest, ModelSpec, Module};
use gize_generator::{Options, Plan, Writer, diff, registry, scaffold, sync};

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
/// so migration ordering is preserved. (A calendar-formatted stamp was considered for
/// readability but rejected — it sorts before existing nanosecond stamps; see ADR-011.)
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
pub fn new_project(name: &str, no_user: bool, openapi: bool, flags: GenFlags) -> Result<()> {
    let root = Path::new(name);
    if root.exists() && !flags.force {
        bail!("directory `{name}` already exists (use --force to generate into it)");
    }

    let plan = scaffold::new_project(name, !no_user, openapi, &migration_timestamp());
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
        if openapi {
            println!("OpenAPI spec at `/openapi.json` and docs at `/docs` once running.");
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

/// Refresh the derived `openapi.json` from the current manifest when `features.openapi` is on,
/// keeping the spec in parity with the routes after a structural change (ADR-010). No-op when
/// the feature is off. The spec is always overwritten (it is generated, not hand-edited).
fn refresh_openapi_if_enabled(flags: GenFlags) -> Result<()> {
    let manifest =
        Manifest::from_toml(&fs::read_to_string("gize.toml").context("reading gize.toml")?)?;
    if !manifest.features.openapi {
        return Ok(());
    }
    if flags.dry_run {
        println!("  update  openapi.json (would refresh the spec)");
    } else {
        fs::write("openapi.json", scaffold::openapi_json(&manifest)?)
            .context("writing openapi.json")?;
        println!("  update  openapi.json (refreshed from gize.toml)");
    }
    Ok(())
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
            belongs_to: model.relations.clone(),
        },
        flags,
    )?;
    refresh_openapi_if_enabled(flags)?;

    if !flags.dry_run {
        println!("\nApply the migration with:\n  gize migrate");
    }
    Ok(())
}

/// `gize make admin` — generate the admin SPA (ADR-006) for every resource in the manifest.
///
/// The admin is a **separate** Vite + React + TypeScript app under `admin/`, data-driven from
/// `gize.toml`. The static shell is written drift-aware; `admin/src/resources.ts` is a derived
/// artifact refreshed from the current manifest. The app reaches the API through a Vite dev
/// proxy, so the backend needs no CORS or other changes.
pub fn make_admin(_name: Option<&str>, flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    let mut manifest =
        Manifest::from_toml(&fs::read_to_string("gize.toml").context("reading gize.toml")?)?;
    if manifest.modules.is_empty() {
        bail!("no resources in gize.toml — add one with `gize make crud` first");
    }

    let report = Writer::new(flags.into())
        .apply(Path::new("."), &scaffold::admin_shell_plan(&manifest))
        .context("generating the admin app")?;
    println!("Admin SPA (admin/):\n{}", report.render(flags.dry_run));

    // Refresh the derived descriptors from the current manifest (always overwritten).
    if flags.dry_run {
        println!("  update  admin/src/resources.ts (would refresh descriptors)");
    } else {
        fs::create_dir_all("admin/src").context("creating admin/src")?;
        fs::write(
            "admin/src/resources.ts",
            scaffold::admin_resources_ts(&manifest)?,
        )
        .context("writing admin/src/resources.ts")?;
        println!("  update  admin/src/resources.ts (from gize.toml)");
    }

    // Record the feature so `gize sync` reconciles the admin.
    if !manifest.features.admin && !flags.dry_run {
        manifest.features.admin = true;
        fs::write("gize.toml", manifest.to_toml()?).context("writing gize.toml")?;
    }

    if !flags.dry_run {
        println!(
            "\nNext:\n  cd admin\n  npm install\n  npm run dev   \
             # http://localhost:5173 (proxies /api to your `gize serve` backend)"
        );
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

/// `gize make migration [name]` (ADR-011).
///
/// - **With a name**: generate a blank, timestamped SQL migration to fill in by hand — the
///   escape hatch for indexes, constraints and backfills.
/// - **Without a name**: diff each module's declared fields (`gize.toml`) against the columns
///   in its existing migrations and emit `ALTER TABLE` migrations to reconcile. New columns
///   are added automatically (nullable, for safety); dropped columns are withheld unless
///   `--force` is given.
pub fn make_migration(name: Option<&str>, flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    match name {
        Some(name) => make_blank_migration(name, flags),
        None => make_diff_migrations(flags),
    }
}

/// The named escape hatch: a single blank, timestamped migration to edit by hand.
fn make_blank_migration(name: &str, flags: GenFlags) -> Result<()> {
    let migration_name = slugify(name);
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

/// Model-change diffing: reconcile each module's table to its `gize.toml` shape (ADR-011).
fn make_diff_migrations(flags: GenFlags) -> Result<()> {
    let manifest =
        Manifest::from_toml(&fs::read_to_string("gize.toml").context("reading gize.toml")?)?;
    let dir = Path::new("migrations");

    let mut plan = Plan::new();
    let mut withheld_drops = 0usize;
    for module in &manifest.modules {
        if module.fields.is_empty() {
            continue; // no declared shape to diff (e.g. a bare `make app` module)
        }
        let model = module.model_spec()?;
        let Some(schema_diff) = diff::diff_model(dir, &module.name, &model)? else {
            println!(
                "  skip    {} (no create migration yet — run `gize make crud`/`gize sync` first)",
                module.name
            );
            continue;
        };
        if schema_diff.is_empty() {
            continue;
        }

        for f in &schema_diff.added {
            println!("  add     {}.{} {}", module.name, f.name, f.ty.sql_type());
        }
        for c in &schema_diff.dropped {
            if flags.force {
                println!("  drop    {}.{c} (--force)", module.name);
            } else {
                println!("  hold    {}.{c} (drop withheld; use --force)", module.name);
                withheld_drops += 1;
            }
        }

        // Only emit a migration when there is something to apply: any added column, or a drop
        // the developer opted into with --force.
        if !schema_diff.added.is_empty() || (flags.force && !schema_diff.dropped.is_empty()) {
            plan = plan.create(
                format!(
                    "migrations/{}_alter_{}.sql",
                    migration_timestamp(),
                    module.name
                ),
                diff::alter_sql(&module.name, &schema_diff, flags.force),
            );
        }
    }

    if plan.is_empty() {
        println!("Schema matches gize.toml — no model-change migrations to generate.");
        if withheld_drops > 0 {
            println!(
                "({withheld_drops} column drop(s) withheld — re-run with --force to emit them.)"
            );
        }
        return Ok(());
    }

    let report = Writer::new(flags.into())
        .apply(Path::new("."), &plan)
        .context("writing alter migrations")?;
    println!("\n{}", report.render(flags.dry_run));
    if !flags.dry_run {
        println!("Review the generated SQL, then apply it with:\n  gize migrate");
    }
    Ok(())
}

/// `gize sync` — reconcile the project from `gize.toml` (ADR-009).
///
/// Regenerates any module declared in the manifest whose code is missing, creates a
/// `CREATE TABLE` migration for any module that lacks one, and wires each module into
/// `src/app/mod.rs`. Files that exist but differ from the manifest are reported as **drift**
/// and left untouched unless `--force` is given; `--dry-run` previews without writing.
pub fn sync(flags: GenFlags) -> Result<()> {
    ensure_in_project()?;
    let manifest =
        Manifest::from_toml(&fs::read_to_string("gize.toml").context("reading gize.toml")?)?;

    if manifest.modules.is_empty() {
        println!("No modules declared in gize.toml — nothing to sync.");
        return Ok(());
    }

    // 1. Desired code files for every declared module (deterministic; no timestamps). The
    //    auth module is part of the skeleton the CRUD routes depend on, so reconcile it too.
    let mut plan = Plan::new().create("src/auth/mod.rs", scaffold::auth_mod_rs());
    for module in &manifest.modules {
        plan = plan.extend(
            scaffold::module_code(module)
                .with_context(|| format!("planning module `{}`", module.name))?,
        );
    }

    // 2. A CREATE TABLE migration only for tables that do not already have one, so re-running
    //    `sync` never spawns duplicate migrations (idempotent — ADR-011). Ordered so a
    //    foreign key's target table is created before the table that references it (ADR-014).
    for module in manifest.modules_in_dependency_order()? {
        if !create_migration_exists(&module.name)? {
            let sql = scaffold::module_migration_sql(module)?;
            plan = plan.create(
                format!(
                    "migrations/{}_create_{}.sql",
                    migration_timestamp(),
                    module.name
                ),
                sql,
            );
        }
    }

    // 2b. OpenAPI (ADR-010): when enabled, reconcile the (static) route module drift-aware.
    //     The spec itself is a derived artifact, refreshed unconditionally after apply below.
    if manifest.features.openapi {
        plan = plan.create("src/app/openapi.rs", scaffold::openapi_module_rs());
    }

    // 2c. Admin (ADR-006): reconcile the static SPA shell drift-aware; the resource
    //     descriptors are a derived artifact, refreshed after apply below.
    if manifest.features.admin {
        plan = plan.extend(scaffold::admin_shell_plan(&manifest));
    }

    // 3. Diff against the filesystem and apply per the safety flags.
    let root = Path::new(".");
    let recon = sync::reconcile(root, &plan)?;
    let applied = sync::apply(root, &recon, flags.force, flags.dry_run)?;
    println!(
        "Reconciling from gize.toml:\n{}",
        applied.render(flags.dry_run)
    );

    // 4. Wire each module into src/app/mod.rs (idempotent; a no-op for already-wired ones).
    for module in &manifest.modules {
        register_in_app_mod(&module.name, flags)?;
    }
    // The OpenAPI module is wired like any module, but is never listed in `[[module]]` (it is
    // not a resource), so register it here when the feature is on, and refresh the derived
    // spec from the current manifest (always overwritten — it is generated, not hand-edited).
    if manifest.features.openapi {
        register_in_app_mod("openapi", flags)?;
        if flags.dry_run {
            println!("  update  openapi.json (would refresh the spec)");
        } else {
            fs::write("openapi.json", scaffold::openapi_json(&manifest)?)
                .context("writing openapi.json")?;
            println!("  update  openapi.json (refreshed from gize.toml)");
        }
    }

    // Refresh the derived admin descriptors from the current manifest (always overwritten).
    if manifest.features.admin {
        if flags.dry_run {
            println!("  update  admin/src/resources.ts (would refresh descriptors)");
        } else {
            fs::create_dir_all("admin/src").context("creating admin/src")?;
            fs::write(
                "admin/src/resources.ts",
                scaffold::admin_resources_ts(&manifest)?,
            )
            .context("writing admin/src/resources.ts")?;
            println!("  update  admin/src/resources.ts (refreshed from gize.toml)");
        }
    }

    // 5. Surface drift explicitly — the conservative default never overwrites hand edits.
    if !recon.drift.is_empty() && !flags.force {
        println!(
            "\n{} file(s) drifted from the manifest and were left untouched. \
             Review them, or re-run `gize sync --force` to overwrite.",
            recon.drift.len()
        );
    }
    Ok(())
}

/// Whether a `*_create_<table>.sql` migration already exists under `migrations/`.
fn create_migration_exists(table: &str) -> Result<bool> {
    let dir = Path::new("migrations");
    if !dir.exists() {
        return Ok(false);
    }
    let suffix = format!("_create_{table}.sql");
    for entry in fs::read_dir(dir).context("reading migrations/")? {
        let name = entry?.file_name();
        if name.to_string_lossy().ends_with(&suffix) {
            return Ok(true);
        }
    }
    Ok(false)
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
    // `.env` is auto-loaded at startup, so these reflect the effective values.
    report("DATABASE_URL set", std::env::var("DATABASE_URL").is_ok());
    report(
        "GIZE_JWT_SECRET set (auth token signing)",
        std::env::var("GIZE_JWT_SECRET").is_ok(),
    );

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
