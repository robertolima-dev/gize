//! High-level generators. Each returns a pure [`Plan`]; applying it is the [`Writer`]'s
//! job. This separation keeps generation testable and makes `--dry-run` free.

use anyhow::Result;
use gize_core::naming::table_name;
use gize_core::{Api, Dialect, Manifest, ModelSpec, Module};
use gize_templates::{auth, crud, model, module, openapi, project, user, ws};

use crate::plan::Plan;
use crate::registry;

/// The nine code files of a CRUD vertical slice (everything but the migration), shared by
/// `gize new`'s built-in users slice, `gize make crud`, and `gize sync` so all three paths
/// emit byte-identical code. When `is_user` is set, the password-hiding `model.rs` is used.
fn crud_files(model: &ModelSpec, dir: &str, is_user: bool, dialect: Dialect) -> Plan {
    // The users slice customizes the files that auth touches (password-hiding model, hashing
    // handlers, register/login routes, an error type with `Unauthorized`); everything else is
    // the generic CRUD template. See `gize_templates::user`.
    let (model_rs, dto_rs, error_rs, handler_rs, routes_rs) = if is_user {
        (
            user::model_rs(),
            user::dto_rs(),
            user::error_rs(dialect),
            user::handler_rs(dialect),
            user::routes_rs(),
        )
    } else {
        (
            model::model_rs(model),
            crud::dto_rs(model),
            crud::error_rs(model, dialect),
            crud::handler_rs(model),
            crud::routes_rs(model),
        )
    };
    Plan::new()
        .create(format!("{dir}/mod.rs"), crud::mod_rs(model))
        .create(format!("{dir}/model.rs"), model_rs)
        .create(format!("{dir}/dto.rs"), dto_rs)
        .create(format!("{dir}/error.rs"), error_rs)
        .create(
            format!("{dir}/repository.rs"),
            crud::repository_rs(model, dialect),
        )
        .create(
            format!("{dir}/service.rs"),
            crud::service_rs(model, dialect),
        )
        .create(format!("{dir}/handler.rs"), handler_rs)
        .create(format!("{dir}/routes.rs"), routes_rs)
        .create(format!("{dir}/tests.rs"), crud::tests_rs(model))
}

/// The generated `src/auth/mod.rs` skeleton (ADR-013), so `gize sync` can reconcile the auth
/// module that the CRUD routes depend on.
pub fn auth_mod_rs() -> String {
    auth::mod_rs()
}

/// The `src/app/openapi.rs` route module (ADR-010). Static (does not depend on the modules),
/// so it can be reconciled drift-aware by `gize sync`.
pub fn openapi_module_rs() -> String {
    openapi::module_rs()
}

/// The `openapi.json` spec rendered from the manifest (ADR-010). This is a *derived* artifact
/// — always regenerated from the manifest, never drift-protected.
pub fn openapi_json(manifest: &Manifest) -> Result<String> {
    let spec = gize_openapi::spec_json(manifest)?;
    serde_json::to_string_pretty(&spec).map_err(Into::into)
}

/// The OpenAPI slice (ADR-010): the `src/app/openapi.rs` route module plus the generated
/// `openapi.json`. Used by `gize new --openapi` and reconciled by `gize sync` when
/// `features.openapi` is on. The module is registered in `app/mod.rs` by the command, like
/// any module. The spec describes the *resource* routes only — the openapi module is wired
/// but never listed in `[[module]]`, so it does not appear in its own spec.
pub fn openapi_slice(manifest: &Manifest) -> Result<Plan> {
    Ok(Plan::new()
        .create("src/app/openapi.rs", openapi::module_rs())
        .create("openapi.json", openapi_json(manifest)?))
}

/// The generated `admin/src/resources.ts` — the manifest-derived resource descriptors
/// (ADR-006). A derived artifact, always regenerated from the current manifest.
pub fn admin_resources_ts(manifest: &Manifest) -> Result<String> {
    gize_admin::resources_ts(manifest)
}

/// The admin SPA **shell** (ADR-006): every file of the separate Vite + React + TypeScript app
/// under `admin/` except the derived `src/resources.ts`. Static, so it is reconciled
/// drift-aware by `gize sync`; the descriptors are written separately (see
/// [`admin_resources_ts`]). Talks to the API through a Vite dev proxy — no backend changes.
pub fn admin_shell_plan(manifest: &Manifest) -> Plan {
    let project = &manifest.project.name;
    Plan::new()
        .create("admin/package.json", gize_admin::package_json(project))
        .create("admin/vite.config.ts", gize_admin::vite_config())
        .create("admin/tsconfig.json", gize_admin::tsconfig())
        .create("admin/index.html", gize_admin::index_html())
        .create("admin/.env.example", gize_admin::env_example())
        .create("admin/.gitignore", gize_admin::gitignore())
        .create("admin/src/main.tsx", gize_admin::main_tsx())
        .create("admin/src/styles.css", gize_admin::styles_css())
        .create("admin/src/theme.ts", gize_admin::theme_ts())
        .create("admin/src/icons.tsx", gize_admin::icons_tsx())
        .create("admin/src/toast.tsx", gize_admin::toast_tsx())
        .create("admin/src/api.ts", gize_admin::api_ts())
        .create("admin/src/auth.tsx", gize_admin::auth_tsx())
        .create("admin/src/App.tsx", gize_admin::app_tsx())
        .create("admin/src/Resource.tsx", gize_admin::resource_tsx())
        .create(
            "admin/src/ResourceForm.tsx",
            gize_admin::resource_form_tsx(),
        )
}

/// The code files (no migration) for a module reconstructed from its manifest entry, for
/// `gize sync` (ADR-009 revision). The built-in `users` module keeps its special `model.rs`.
pub fn module_code(module: &Module, dialect: Dialect) -> Result<Plan> {
    let model = module.model_spec()?;
    let dir = format!("src/app/{}", module.name);
    Ok(crud_files(&model, &dir, module.name == "users", dialect))
}

/// The `CREATE TABLE` migration SQL for a module reconstructed from its manifest entry. The
/// built-in `users` table keeps its special migration (`email UNIQUE`, `is_admin` default).
pub fn module_migration_sql(module: &Module, dialect: Dialect) -> Result<String> {
    let model = module.model_spec()?;
    Ok(if module.name == "users" {
        user::migration_sql(dialect)
    } else {
        model::migration_sql(&model, dialect)
    })
}

/// Plan for `gize new <name>`: a complete, compiling project skeleton (ADR-005).
///
/// When `with_user` is set (the default), a built-in `users` resource is scaffolded and
/// wired in — model, CRUD and a migration with an `is_admin` flag — so a fresh project has
/// authentication-ready data from the start. When `with_openapi` is set, an OpenAPI spec +
/// `/docs` UI are generated and wired (ADR-010). `timestamp` names the users migration and is
/// injected (not read from the clock) to keep generation pure and tests deterministic.
pub fn new_project(
    name: &str,
    with_user: bool,
    with_openapi: bool,
    with_ws: bool,
    dialect: Dialect,
    api: Option<Api>,
    timestamp: &str,
) -> Plan {
    let mut manifest = Manifest::new(name);
    // Auth is generated for every project (ADR-013): the `src/auth` module is emitted below
    // and write routes are guarded, so the manifest reflects it.
    manifest.features.authentication = true;
    manifest.features.openapi = with_openapi;
    manifest.features.websocket = with_ws;
    // Record the API version prefix, if any, so the router nests routes under it and
    // `gize sync` keeps the same mount (ADR-016).
    manifest.api = api;
    // Record the chosen database so `gize sync` regenerates against the same dialect (ADR-015).
    manifest.stack.database = dialect.sqlx_feature().to_string();
    if with_user {
        // Record the built-in users module with its full shape so `gize sync` can
        // reconcile/rebuild it from the manifest alone (ADR-009 revision).
        manifest.upsert_module(Module {
            name: "users".to_string(),
            fields: user::spec().to_field_tokens(),
            belongs_to: Vec::new(),
        });
    }

    // Pre-wire the built-in modules into app/mod.rs using the same registry edit `make app`
    // uses, keeping a single source of truth for the module/route wiring format.
    let mut app_mod = project::app_mod_rs();
    for module in [
        (with_user, "users"),
        (with_openapi, "openapi"),
        (with_ws, "ws"),
    ] {
        if module.0 {
            app_mod = registry::register_module(&app_mod, module.1)
                .expect("app_mod_rs template carries the gize markers")
                .content;
        }
    }

    let mut plan = Plan::new()
        .create("Cargo.toml", project::cargo_toml(name, dialect, with_ws))
        .create("gize.toml", project::gize_toml(&manifest))
        .create(".env.example", project::env_example(name, dialect))
        .create(".gitignore", "/target\n.env\n")
        .create("src/main.rs", project::main_rs())
        .create("src/state.rs", project::state_rs(dialect))
        .create(
            "src/router.rs",
            project::router_rs(manifest.api.as_ref().map(Api::mount_path).as_deref()),
        )
        .create("src/config/mod.rs", project::config_mod_rs())
        .create("src/auth/mod.rs", auth::mod_rs())
        .create("src/app/mod.rs", app_mod)
        // Layout directories reserved by ADR-005 so the tree does not churn later.
        .mkdir("src/database")
        .mkdir("src/middleware")
        .mkdir("src/shared")
        .mkdir("migrations");

    if with_user {
        plan = plan.extend(user_slice(dialect, timestamp));
    }
    if with_openapi {
        // A fresh manifest is valid, so the spec renders; propagate any error defensively.
        plan = plan
            .extend(openapi_slice(&manifest).expect("fresh manifest yields a valid OpenAPI spec"));
    }
    if with_ws {
        plan = plan.extend(ws_module_plan());
    }
    plan
}

/// The optional WebSocket module (ADR-018): a self-contained `src/app/ws/` with a typed echo
/// endpoint. Static (does not depend on the modules), so `gize sync` reconciles it drift-aware
/// when `features.websocket` is on, like the OpenAPI module.
pub fn ws_module_plan() -> Plan {
    Plan::new()
        .create("src/app/ws/mod.rs", ws::mod_rs())
        .create("src/app/ws/message.rs", ws::message_rs())
        .create("src/app/ws/handler.rs", ws::handler_rs())
        .create("src/app/ws/routes.rs", ws::routes_rs())
}

/// The built-in `users` vertical slice for a fresh project: the generic CRUD templates plus
/// a password-hiding model and a users migration with an `is_admin` flag (see [`user`]).
fn user_slice(dialect: Dialect, timestamp: &str) -> Plan {
    crud_files(&user::spec(), "src/app/users", true, dialect).create(
        format!("migrations/{timestamp}_create_users.sql"),
        user::migration_sql(dialect),
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
pub fn make_model(model: &ModelSpec, dialect: Dialect, timestamp: &str) -> Plan {
    let module = table_name(&model.name);
    let table = &module;

    Plan::new()
        .create(format!("src/app/{module}/model.rs"), model::model_rs(model))
        .create(
            format!("migrations/{timestamp}_create_{table}.sql"),
            model::migration_sql(model, dialect),
        )
}

/// Plan for `gize make crud <Name> field:Type ...`: a full, compiling resource — model,
/// DTOs, SQLx repository, service, Axum handlers, routes, error, tests, and the migration.
/// Registration in `app/mod.rs` / `gize.toml` is handled by the command.
pub fn make_crud(model: &ModelSpec, dialect: Dialect, timestamp: &str) -> Plan {
    let table = table_name(&model.name);
    let dir = format!("src/app/{table}");

    crud_files(model, &dir, false, dialect).create(
        format!("migrations/{timestamp}_create_{table}.sql"),
        model::migration_sql(model, dialect),
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
        let plan = new_project(
            "shop",
            false,
            false,
            false,
            Dialect::Postgres,
            None,
            "20260704120000",
        );
        let paths = paths_of(&plan);
        assert!(paths.contains(&"Cargo.toml".to_string()));
        assert!(paths.contains(&"gize.toml".to_string()));
        assert!(paths.contains(&"src/main.rs".to_string()));
        assert!(paths.contains(&"src/app/mod.rs".to_string()));
    }

    #[test]
    fn new_project_without_user_omits_users_slice() {
        let plan = new_project(
            "shop",
            false,
            false,
            false,
            Dialect::Postgres,
            None,
            "20260704120000",
        );
        let paths = paths_of(&plan);
        assert!(!paths.iter().any(|p| p.starts_with("src/app/users/")));
        assert!(!paths.iter().any(|p| p.ends_with("_create_users.sql")));
        // app/mod.rs and gize.toml stay empty of the users module.
        assert!(!content_of(&plan, "src/app/mod.rs").contains("mod users;"));
        assert!(!content_of(&plan, "gize.toml").contains("users"));
    }

    #[test]
    fn new_project_scaffolds_and_wires_users_by_default() {
        let plan = new_project(
            "shop",
            true,
            false,
            false,
            Dialect::Postgres,
            None,
            "20260704120000",
        );
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
        let plan = make_model(&model, Dialect::Postgres, "20260704120000");
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
        let plan = make_crud(&model, Dialect::Postgres, "20260704120000");
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
    fn new_project_with_openapi_includes_spec_and_module() {
        let plan = new_project(
            "blog",
            true,
            true,
            false,
            Dialect::Postgres,
            None,
            "20260704120000",
        );
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        assert!(paths.contains(&"src/app/openapi.rs".to_string()));
        assert!(paths.contains(&"openapi.json".to_string()));
        // The spec is valid JSON describing the users routes; the openapi module is wired.
        let spec = plan
            .ops
            .iter()
            .find(|o| o.path.display().to_string() == "openapi.json")
            .map(|o| o.contents.as_str())
            .unwrap();
        assert!(spec.contains("\"/users\""));
        let app_mod = plan
            .ops
            .iter()
            .find(|o| o.path.display().to_string() == "src/app/mod.rs")
            .map(|o| o.contents.as_str())
            .unwrap();
        assert!(app_mod.contains("mod openapi;"));
        assert!(app_mod.contains(".merge(openapi::routes())"));
    }

    #[test]
    fn new_project_without_openapi_omits_it() {
        let plan = new_project(
            "blog",
            true,
            false,
            false,
            Dialect::Postgres,
            None,
            "20260704120000",
        );
        let paths: Vec<_> = plan
            .ops
            .iter()
            .map(|o| o.path.display().to_string())
            .collect();
        assert!(!paths.contains(&"openapi.json".to_string()));
        assert!(!paths.iter().any(|p| p == "src/app/openapi.rs"));
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
