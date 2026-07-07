//! Argon2id password hashing for `gize createadmin` (ADR-017).
//!
//! Lives in the CLI (not `gize-auth`) so the published `gize` binary has no unpublished
//! dependency. It mirrors the generated `src/auth/mod.rs` hashing exactly, so an admin created
//! here logs in through the app's normal flow. The generated project keeps its own copy of this
//! logic — the "you own the code" philosophy — so the two are intentionally independent.

use anyhow::{Context, Result};
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHasher, SaltString};

/// Hash a plaintext password with Argon2id (default parameters), returning the PHC string the
/// generated `verify_password` accepts.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("hashing the password")
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    #[test]
    fn hash_is_not_plaintext_and_verifies() {
        let hash = hash_password("correct horse battery staple").unwrap();
        // Never store the plaintext; produce a PHC-format Argon2 hash.
        assert!(!hash.contains("correct horse"));
        assert!(hash.starts_with("$argon2"));
        // The hash verifies against the original password (the generated login relies on this).
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password("correct horse battery staple".as_bytes(), &parsed)
                .is_ok()
        );
    }

    #[test]
    fn each_hash_is_salted_uniquely() {
        // Two hashes of the same password differ (random salt) — guards against an unsalted
        // regression.
        assert_ne!(
            hash_password("hunter2000").unwrap(),
            hash_password("hunter2000").unwrap()
        );
    }
}
