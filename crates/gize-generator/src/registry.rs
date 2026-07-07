//! Idempotent edits to "registry" files — the ones `gize make app` must update in place
//! rather than create: `src/app/mod.rs` (module + route wiring).
//!
//! MVP approach: marker-based insertion. The generated `app/mod.rs` carries two markers
//! (`gize:modules` and `gize:module-routes`); we insert around them and short-circuit if
//! the module is already registered, which makes re-runs a no-op. ADR-004 earmarks a
//! `syn`-based editor as a hardening follow-up; the pure-function boundary here keeps that
//! swap internal.

use anyhow::{Result, bail};

const MODULES_MARKER: &str = "// gize:modules (do not remove this marker)";
const ROUTES_MARKER: &str = "// gize:module-routes (do not remove this marker)";

/// The result of a registry edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Whether the edit changed anything (false = already registered).
    pub changed: bool,
    /// The (possibly unchanged) file content.
    pub content: String,
}

/// Register `module` in an `app/mod.rs` source: add `mod <module>;` and merge its routes.
/// Idempotent — if the module is already present, returns `changed: false`.
pub fn register_module(source: &str, module: &str) -> Result<Edit> {
    let mod_decl = format!("mod {module};");
    if source.contains(&mod_decl) {
        return Ok(Edit {
            changed: false,
            content: source.to_string(),
        });
    }

    if !source.contains(MODULES_MARKER) || !source.contains(ROUTES_MARKER) {
        bail!(
            "src/app/mod.rs is missing gize markers; cannot register `{module}` automatically. \
             Re-add the `// gize:modules` and `// gize:module-routes` markers or wire the module by hand."
        );
    }

    // Insert the `mod` declaration and the `.merge(...)` call in registration order. The CLI runs
    // rustfmt over `app/mod.rs` after this edit (ADR-020), which sorts the `mod` declarations and
    // collapses/wraps the merge chain, so the file lands rustfmt-clean regardless of module count.
    let with_mod = insert_after_marker(source, MODULES_MARKER, &mod_decl);
    let merge_call = format!("        .merge({module}::routes())");
    let content = insert_before_marker(&with_mod, ROUTES_MARKER, &merge_call);

    Ok(Edit {
        changed: true,
        content,
    })
}

/// Insert `new_line` immediately after the first line containing `marker`.
fn insert_after_marker(source: &str, marker: &str, new_line: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in source.lines() {
        out.push(line.to_string());
        if line.contains(marker) {
            out.push(new_line.to_string());
        }
    }
    finish(out, source)
}

/// Insert `new_line` immediately before the first line containing `marker`.
fn insert_before_marker(source: &str, marker: &str, new_line: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in source.lines() {
        if line.contains(marker) {
            out.push(new_line.to_string());
        }
        out.push(line.to_string());
    }
    finish(out, source)
}

/// Re-join lines, preserving a trailing newline if the source had one.
fn finish(lines: Vec<String>, source: &str) -> String {
    let mut s = lines.join("\n");
    if source.ends_with('\n') {
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP_MOD: &str = r#"use axum::Router;

use crate::state::AppState;

// gize:modules (do not remove this marker)

pub fn routes() -> Router<AppState> {
    Router::new()
    // gize:module-routes (do not remove this marker)
}
"#;

    #[test]
    fn registers_a_new_module() {
        let edit = register_module(APP_MOD, "users").unwrap();
        assert!(edit.changed);
        assert!(edit.content.contains("mod users;"));
        assert!(edit.content.contains(".merge(users::routes())"));
        // markers are preserved for the next registration
        assert!(edit.content.contains(MODULES_MARKER));
        assert!(edit.content.contains(ROUTES_MARKER));
    }

    #[test]
    fn is_idempotent() {
        let first = register_module(APP_MOD, "users").unwrap();
        let second = register_module(&first.content, "users").unwrap();
        assert!(!second.changed);
        assert_eq!(first.content, second.content);
    }

    #[test]
    fn registers_multiple_modules() {
        let first = register_module(APP_MOD, "users").unwrap();
        let second = register_module(&first.content, "products").unwrap();
        assert!(second.changed);
        assert!(second.content.contains("mod users;"));
        assert!(second.content.contains("mod products;"));
        assert!(second.content.contains(".merge(users::routes())"));
        assert!(second.content.contains(".merge(products::routes())"));
    }

    #[test]
    fn fails_without_markers() {
        assert!(register_module("fn routes() {}", "users").is_err());
    }
}
