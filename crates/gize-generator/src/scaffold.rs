//! High-level generators. Each returns a pure [`Plan`]; applying it is the [`Writer`]'s
//! job. This separation keeps generation testable and makes `--dry-run` free.

use anyhow::Result;
use gize_core::naming::table_name;
use gize_core::{Manifest, ModelSpec, Module};
use gize_templates::{crud, model, module, project, user};

use crate::plan::Plan;
use crate::registry;

/// The nine code files of a CRUD vertical slice (everything but the migration), shared by
/// `gize new`'s built-in users slice, `gize make crud`, and `gize sync` so all three paths
/// emit byte-identical code. When `is_user` is set, the password-hiding `model.rs` is used.
fn crud_files(model: &ModelSpec, dir: &str, is_user: bool) -> Plan {
    let model_rs = if is_user {
        user::model_rs()
    } else {
        model::model_rs(model)
    };
    Plan::new()
        .create(format!("{dir}/mod.rs"), crud::mod_rs(model))
        .create(format!("{dir}/model.rs"), model_rs)
        .create(format!("{dir}/dto.rs"), crud::dto_rs(model))
        .create(format!("{dir}/error.rs"), crud::error_rs(model))
        .create(format!("{dir}/repository.rs"), crud::repository_rs(model))
        .create(format!("{dir}/service.rs"), crud::service_rs(model))
        .create(format!("{dir}/handler.rs"), crud::handler_rs(model))
        .create(format!("{dir}/routes.rs"), crud::routes_rs(model))
        .create(format!("{dir}/tests.rs"), crud::tests_rs(model))
}

/// The code files (no migration) for a module reconstructed from its manifest entry, for
/// `gize sync` (ADR-009 revision). The built-in `users` module keeps its special `model.rs`.
pub fn module_code(module: &Module) -> Result<Plan> {
    let model = module.model_spec()?;
    let dir = format!("src/app/{}", module.name);
    Ok(crud_files(&model, &dir, module.name == "users"))
}

/// The `CREATE TABLE` migration SQL for a module reconstructed from its manifest entry. The
/// built-in `users` table keeps its special migration (`email UNIQUE`, `is_admin` default).
pub fn module_migration_sql(module: &Module) -> Result<String> {
    let model = module.model_spec()?;
    Ok(if module.name == "users" {
        user::migration_sql()
    } else {
        model::migration_sql(&model)
    })
}

/// Plan for `gize new <name>`: a complete, compiling project skeleton (ADR-005).
///
/// When `with_user` is set (the default), a built-in `users` resource is scaffolded and
/// wired in — model, CRUD and a migration with an `is_admin` flag — so a fresh project has
/// authentication-ready data from the start. `timestamp` names the users migration and is
/// injected (not read from the clock) to keep generation pure and tests deterministic.
pub fn new_project(name: &str, with_user: bool, timestamp: &str) -> Plan {
    let mut manifest = Manifest::new(name);
    if with_user {
        // Record the built-in users module with its full shape so `gize sync` can
        // reconcile/rebuild it from the manifest alone (ADR-009 revision).
        manifest.upsert_module(Module {
            name: "users".to_string(),
            fields: user::spec().to_field_tokens(),
            belongs_to: Vec::new(),
        });
    }

    // Pre-wire `users` into app/mod.rs by running the same registry edit `make app` uses,
    // keeping a single source of truth for the module/route wiring format.
    let app_mod = if with_user {
        registry::register_module(&project::app_mod_rs(), "users")
            .expect("app_mod_rs template carries the gize markers")
            .content
    } else {
        project::app_mod_rs()
    };

    let plan = Plan::new()
        .create("Cargo.toml", project::cargo_toml(name))
        .create("gize.toml", project::gize_toml(&manifest))
        .create(".env.example", project::env_example(name))
        .create(".gitignore", "/target\n.env\n")
        .create("src/main.rs", project::main_rs())
        .create("src/state.rs", project::state_rs())
        .create("src/router.rs", project::router_rs())
        .create("src/config/mod.rs", project::config_mod_rs())
        .create("src/app/mod.rs", app_mod)
        // Layout directories reserved by ADR-005 so the tree does not churn later.
        .mkdir("src/database")
        .mkdir("src/middleware")
        .mkdir("src/shared")
        .mkdir("migrations");

    if with_user {
        plan.extend(user_slice(timestamp))
    } else {
        plan
    }
}

/// The built-in `users` vertical slice for a fresh project: the generic CRUD templates plus
/// a password-hiding model and a users migration with an `is_admin` flag (see [`user`]).
fn user_slice(timestamp: &str) -> Plan {
    crud_files(&user::spec(), "src/app/users", true).create(
        format!("migrations/{timestamp}_create_users.sql"),
        user::migration_sql(),
    )
}

/// Plan for `gize make app <name>`: a full, compiling module skeleton (ADR-005).
///
/// The module directory equals the module name verbatim (already snake_cased by the CLI).
/// Registration of the module in `app/mod.rs` and `gize.toml` is handled by the command,
/// not the plan, because those are edits to existing files (see [`crate::registry`]).
pub fn make_app(module: &str) -> Plan {
    let dir = format!("src/app/{module}");
    Plan::new()
        .create(format!("{dir}/mod.rs"), module::mod_rs(module))
        .create(
            format!("{dir}/model.rs"),
            module::model_placeholder_rs(module),
        )
        .create(format!("{dir}/dto.rs"), module::dto_rs(module))
        .create(
            format!("{dir}/repository.rs"),
            module::repository_rs(module),
        )
        .create(format!("{dir}/service.rs"), module::service_rs(module))
        .create(format!("{dir}/error.rs"), module::error_rs(module))
        .create(format!("{dir}/handler.rs"), module::handler_rs(module))
        .create(format!("{dir}/routes.rs"), module::routes_rs(module))
        .create(format!("{dir}/tests.rs"), module::tests_rs(module))
}

/// Plan for `gize make model <Name> field:Type ...`: a model struct + its migration.
///
/// The module directory is the pluralized snake_case of the model (matching the table and
/// the `gize make app` convention, e.g. `User` -> `users`). `timestamp` is injected (not
/// read from the clock) so generation stays pure and tests are deterministic.
pub fn make_model(model: &ModelSpec, timestamp: &str) -> Plan {
    let module = table_name(&model.name);
    let table = &module;

    Plan::new()
        .create(format!("src/app/{module}/model.rs"), model::model_rs(model))
        .create(
            format!("migrations/{timestamp}_create_{table}.sql"),
            model::migration_sql(model),
        )
}

/// Plan for `gize make crud <Name> field:Type ...`: a full, compiling resource — model,
/// DTOs, SQLx repository, service, Axum handlers, routes, error, tests, and the migration.
/// Registration in `app/mod.rs` / `gize.toml` is handled by the command.
pub fn make_crud(model: &ModelSpec, timestamp: &str) -> Plan {
    let table = table_name(&model.name);
    let dir = format!("src/app/{table}");

    crud_files(model, &dir, false).create(
        format!("migrations/{timestamp}_create_{table}.sql"),
        model::migration_sql(model),
    )
}

/// Plan for `gize make migration <name>`: a single blank, timestamped SQL file the developer
/// fills in by hand. `name` is already snake_cased by the CLI; `timestamp` is injected so
/// generation stays pure and tests are deterministic.
pub fn make_migration(name: &str, timestamp: &str) -> Plan {
    Plan::new().create(
        format!("migrations/{timestamp}_{name}.sql"),
        model::blank_migration_sql(name),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths_of(plan: &Plan) -> Vec<String> {
        plan.ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect()
    }

    fn content_of<'a>(plan: &'a Plan, path: &str) -> &'a str {
        plan.ops
            .iter()
            .find(|o| o.path.display().to_string() == path)
            .map(|o| o.contents.as_str())
            .unwrap_or_else(|| panic!("plan has no op for {path}"))
    }

    #[test]
    fn new_project_plan_includes_core_files() {
        let plan = new_project("shop", false, "20260704120000");
        let paths = paths_of(&plan);
        assert!(paths.contains(&"Cargo.toml".to_string()));
        assert!(paths.contains(&"gize.toml".to_string()));
        assert!(paths.contains(&"src/main.rs".to_string()));
        assert!(paths.contains(&"src/app/mod.rs".to_string()));
    }

    #[test]
    fn new_project_without_user_omits_users_slice() {
        let plan = new_project("shop", false, "20260704120000");
        let paths = paths_of(&plan);
        assert!(!paths.iter().any(|p| p.starts_with("src/app/users/")));
        assert!(!paths.iter().any(|p| p.ends_with("_create_users.sql")));
        // app/mod.rs and gize.toml stay empty of the users module.
        assert!(!content_of(&plan, "src/app/mod.rs").contains("mod users;"));
        assert!(!content_of(&plan, "gize.toml").contains("users"));
    }

    #[test]
    fn new_project_scaffolds_and_wires_users_by_default() {
        let plan = new_project("shop", true, "20260704120000");
        let paths = paths_of(&plan);
        for file in ["mod.rs", "model.rs", "dto.rs", "repository.rs", "routes.rs"] {
            assert!(
                paths.contains(&format!("src/app/users/{file}")),
                "missing users/{file}"
            );
        }
        assert!(paths.contains(&"migrations/20260704120000_create_users.sql".to_string()));

        // The users module is wired into app/mod.rs and listed in gize.toml.
        let app_mod = content_of(&plan, "src/app/mod.rs");
        assert!(app_mod.contains("mod users;"));
        assert!(app_mod.contains(".merge(users::routes())"));
        assert!(content_of(&plan, "gize.toml").contains("users"));

        // The migration carries the admin flag and hides nothing schema-wise.
        let migration = content_of(&plan, "migrations/20260704120000_create_users.sql");
        assert!(migration.contains("is_admin BOOLEAN NOT NULL DEFAULT false"));
    }

    #[test]
    fn make_model_plan_has_model_and_migration() {
        let model = ModelSpec::parse("User", &["name:String".to_string()]).unwrap();
        let plan = make_model(&model, "20260704120000");
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        assert!(paths.contains(&"src/app/users/model.rs".to_string()));
        assert!(paths.contains(&"migrations/20260704120000_create_users.sql".to_string()));
    }

    #[test]
    fn make_crud_plan_has_slice_and_migration() {
        let model = ModelSpec::parse("Product", &["name:String".to_string()]).unwrap();
        let plan = make_crud(&model, "20260704120000");
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        assert!(paths.contains(&"src/app/products/repository.rs".to_string()));
        assert!(paths.contains(&"src/app/products/handler.rs".to_string()));
        assert!(paths.contains(&"src/app/products/dto.rs".to_string()));
        assert!(paths.contains(&"migrations/20260704120000_create_products.sql".to_string()));
    }

    #[test]
    fn make_migration_plan_is_single_timestamped_file() {
        let plan = make_migration("add_index_to_users", "20260704120000");
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        assert_eq!(
            paths,
            vec!["migrations/20260704120000_add_index_to_users.sql".to_string()]
        );
    }

    #[test]
    fn make_app_plan_has_full_module() {
        let plan = make_app("users");
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        for file in [
            "mod.rs",
            "model.rs",
            "dto.rs",
            "repository.rs",
            "service.rs",
            "error.rs",
            "handler.rs",
            "routes.rs",
            "tests.rs",
        ] {
            assert!(
                paths.contains(&format!("src/app/users/{file}")),
                "missing {file}"
            );
        }
    }
}
