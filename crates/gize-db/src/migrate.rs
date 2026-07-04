//! Running database migrations via SQLx's runtime migrator (ADR-011).
//!
//! We reuse SQLx's `Migrator`, which loads the `migrations/*.sql` files at runtime, tracks
//! applied versions in its `_sqlx_migrations` table, and applies pending ones in order.
//! The CLI stays synchronous: these helpers own a small current-thread Tokio runtime and
//! block on it, so the `gize` CLI needs no async plumbing.

use std::path::Path;

use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::migrate::{Migration, Migrator};
use sqlx::postgres::PgPoolOptions;

/// Applied vs pending migrations, each labelled `<version>_<description>`.
#[derive(Debug, Default)]
pub struct Status {
    pub applied: Vec<String>,
    pub pending: Vec<String>,
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building the async runtime")
}

async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("connecting to the database")
}

/// Versions already recorded in `_sqlx_migrations`. If the table does not exist yet (no
/// migration has ever run), this is simply empty.
async fn applied_versions(pool: &PgPool) -> Vec<i64> {
    sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

fn label(migration: &Migration) -> String {
    format!("{}_{}", migration.version, migration.description)
}

/// Apply all pending migrations. Returns the labels of the ones newly applied.
pub fn run(database_url: &str, dir: &Path) -> Result<Vec<String>> {
    runtime()?.block_on(async {
        let migrator = Migrator::new(dir).await.context("loading migrations")?;
        let pool = connect(database_url).await?;
        let before = applied_versions(&pool).await;
        migrator.run(&pool).await.context("applying migrations")?;
        let newly = migrator
            .iter()
            .filter(|m| !before.contains(&m.version))
            .map(label)
            .collect();
        Ok(newly)
    })
}

/// Report which migrations are applied and which are pending.
pub fn status(database_url: &str, dir: &Path) -> Result<Status> {
    runtime()?.block_on(async {
        let migrator = Migrator::new(dir).await.context("loading migrations")?;
        let pool = connect(database_url).await?;
        let applied = applied_versions(&pool).await;

        let mut status = Status::default();
        for migration in migrator.iter() {
            if applied.contains(&migration.version) {
                status.applied.push(label(migration));
            } else {
                status.pending.push(label(migration));
            }
        }
        Ok(status)
    })
}
