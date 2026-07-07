//! Runtime configuration loaded from the environment.

/// Application configuration. Extend as needed.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Self { port }
    }
}
