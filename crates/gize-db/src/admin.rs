//! Creating the first admin user (`gize createadmin`, ADR-017).
//!
//! Connects with SQLx's `AnyPool` (like [`crate::migrate`]) so one code path serves Postgres,
//! SQLite and MySQL. Because the `Any` driver passes SQL through unchanged, the `INSERT` uses
//! the dialect's own placeholder style and id strategy — and binds the id as raw bytes so the
//! stored value is byte-for-byte what the generated app reads back for its `uuid::Uuid` id.

use anyhow::{Context, Result, bail};
use gize_core::Dialect;

use crate::migrate::{connect, runtime};

/// Insert an admin user (`is_admin = true`) into the `users` table. `password_hash` must already
/// be an Argon2 PHC string (see `gize-auth`). Rejects a duplicate email up front and reports a
/// missing `users` table with guidance to migrate first.
pub fn create(
    database_url: &str,
    dialect: Dialect,
    email: &str,
    name: &str,
    password_hash: &str,
) -> Result<()> {
    runtime()?.block_on(async {
        let pool = connect(database_url).await?;

        // Fail early (and clearly) if the schema has not been migrated yet.
        let existing = sqlx::query(&format!(
            "SELECT id FROM users WHERE email = {} LIMIT 1",
            dialect.placeholder(1)
        ))
        .bind(email.to_string())
        .fetch_optional(&pool)
        .await
        .context("querying the `users` table — does it exist? run `gize migrate` first")?;
        if existing.is_some() {
            bail!("an account with email `{email}` already exists");
        }

        // Postgres lets the `id` column default generate the UUID; SQLite/MySQL take the
        // app-generated id as its 16 raw bytes, which `Any` stores as a BLOB / `BINARY(16)`.
        let app_id = dialect.app_generates_id();
        let columns = if app_id {
            "id, name, email, password, is_admin"
        } else {
            "name, email, password, is_admin"
        };
        let count = if app_id { 5 } else { 4 };
        let placeholders = (1..=count)
            .map(|i| dialect.placeholder(i))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("INSERT INTO users ({columns}) VALUES ({placeholders})");

        let mut query = sqlx::query(&sql);
        if app_id {
            query = query.bind(uuid::Uuid::new_v4().as_bytes().to_vec());
        }
        query
            .bind(name.to_string())
            .bind(email.to_string())
            .bind(password_hash.to_string())
            .bind(true)
            .execute(&pool)
            .await
            .context("inserting the admin user")?;

        Ok(())
    })
}
