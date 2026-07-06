//! Test utilities for Gize-generated applications (WS-B5).
//!
//! Two small, dependency-free helpers make it practical to test a generated app end-to-end:
//! [`EphemeralSqlite`] gives a throwaway SQLite database (no server needed — ideal for CI),
//! and [`App`] spawns a compiled app binary against it and waits until it is accepting
//! connections. Both clean up on `Drop`. Bring your own HTTP client for the assertions.
//!
//! ```no_run
//! use gize_testing::{App, EphemeralSqlite};
//!
//! let db = EphemeralSqlite::new();
//! let app = App::spawn("target/debug/blog", &db)?;
//! // app.base_url() is now serving; make requests, assert, then drop to shut down.
//! # Ok::<(), std::io::Error>(())
//! ```

use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A throwaway SQLite database file, removed when dropped. Serverless — perfect for CI.
pub struct EphemeralSqlite {
    path: PathBuf,
}

impl EphemeralSqlite {
    /// Create a unique temp database path (the file is created by the app/migrator on first
    /// connect via `?mode=rwc`).
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let name = format!(
            "gize-test-{}-{}.db",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        Self {
            path: std::env::temp_dir().join(name),
        }
    }

    /// The `sqlite://…?mode=rwc` URL for `DATABASE_URL` (creates the file if missing).
    pub fn url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.path.display())
    }
}

impl Default for EphemeralSqlite {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EphemeralSqlite {
    fn drop(&mut self) {
        // Best-effort cleanup of the db file and its sqlite side-files.
        let _ = std::fs::remove_file(&self.path);
        let name = self
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        for suffix in ["-wal", "-shm"] {
            let mut side = self.path.clone();
            side.set_file_name(format!("{name}{suffix}"));
            let _ = std::fs::remove_file(&side);
        }
    }
}

/// Pick a free TCP port by binding to `:0` and reading back the assigned port.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("binding a free port")
        .local_addr()
        .expect("reading the local address")
        .port()
}

/// A spawned generated app. Killed when dropped.
pub struct App {
    child: Child,
    port: u16,
}

impl App {
    /// Spawn `binary` against `db` on a free port, waiting until it accepts connections.
    ///
    /// Sets `DATABASE_URL`, `PORT` and a throwaway `GIZE_JWT_SECRET` in the child's
    /// environment. Assumes migrations have already been applied to `db`.
    pub fn spawn(binary: &str, db: &EphemeralSqlite) -> std::io::Result<Self> {
        let port = free_port();
        let child = Command::new(binary)
            .env("DATABASE_URL", db.url())
            .env("PORT", port.to_string())
            .env("GIZE_JWT_SECRET", "gize-testing-secret")
            .spawn()?;

        let mut app = App { child, port };
        app.wait_until_ready(Duration::from_secs(30))?;
        Ok(app)
    }

    /// The base URL, e.g. `http://127.0.0.1:8080`.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Block until the app's port accepts a TCP connection, or `timeout` elapses.
    fn wait_until_ready(&mut self, timeout: Duration) -> std::io::Result<()> {
        let deadline = Instant::now() + timeout;
        let addr = format!("127.0.0.1:{}", self.port);
        loop {
            if TcpStream::connect(&addr).is_ok() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("app did not start listening on {addr} within {timeout:?}"),
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_sqlite_urls_are_unique_and_rwc() {
        let a = EphemeralSqlite::new();
        let b = EphemeralSqlite::new();
        assert_ne!(a.url(), b.url());
        assert!(a.url().starts_with("sqlite://"));
        assert!(a.url().ends_with("?mode=rwc"));
    }

    #[test]
    fn free_port_is_bindable() {
        let port = free_port();
        // The port was free; binding it now should succeed (nothing grabbed it yet).
        assert!(
            TcpListener::bind(("127.0.0.1", port)).is_ok(),
            "expected the reported free port to be bindable"
        );
    }
}
