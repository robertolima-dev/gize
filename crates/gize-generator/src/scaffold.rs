//! High-level generators. Each returns a pure [`Plan`]; applying it is the [`Writer`]'s
//! job. This separation keeps generation testable and makes `--dry-run` free.

use gize_core::naming::table_name;
use gize_core::{Manifest, ModelSpec};
use gize_templates::{crud, model, module, project};

use crate::plan::Plan;

/// Plan for `gize new <name>`: a complete, compiling project skeleton (ADR-005).
pub fn new_project(name: &str) -> Plan {
    let manifest = Manifest::new(name);

    Plan::new()
        .create("Cargo.toml", project::cargo_toml(name))
        .create("gize.toml", project::gize_toml(&manifest))
        .create(".env.example", project::env_example(name))
        .create(".gitignore", "/target\n.env\n")
        .create("src/main.rs", project::main_rs())
        .create("src/state.rs", project::state_rs())
        .create("src/router.rs", project::router_rs())
        .create("src/config/mod.rs", project::config_mod_rs())
        .create("src/app/mod.rs", project::app_mod_rs())
        // Layout directories reserved by ADR-005 so the tree does not churn later.
        .mkdir("src/database")
        .mkdir("src/middleware")
        .mkdir("src/shared")
        .mkdir("migrations")
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

    Plan::new()
        .create(format!("{dir}/mod.rs"), crud::mod_rs(model))
        .create(format!("{dir}/model.rs"), model::model_rs(model))
        .create(format!("{dir}/dto.rs"), crud::dto_rs(model))
        .create(format!("{dir}/error.rs"), crud::error_rs(model))
        .create(format!("{dir}/repository.rs"), crud::repository_rs(model))
        .create(format!("{dir}/service.rs"), crud::service_rs(model))
        .create(format!("{dir}/handler.rs"), crud::handler_rs(model))
        .create(format!("{dir}/routes.rs"), crud::routes_rs(model))
        .create(format!("{dir}/tests.rs"), crud::tests_rs(model))
        .create(
            format!("migrations/{timestamp}_create_{table}.sql"),
            model::migration_sql(model),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_plan_includes_core_files() {
        let plan = new_project("shop");
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        assert!(paths.contains(&"Cargo.toml".to_string()));
        assert!(paths.contains(&"gize.toml".to_string()));
        assert!(paths.contains(&"src/main.rs".to_string()));
        assert!(paths.contains(&"src/app/mod.rs".to_string()));
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
