//! Naming conventions shared across generators.
//!
//! Centralised so every crate agrees on how a model name maps to a module name, a table
//! name, etc. Kept dependency-free (no heavy inflection crate) for the MVP.

/// Convert a `PascalCase` or arbitrary identifier to `snake_case`.
pub fn snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for (i, ch) in input.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Naive pluralisation good enough for table names in the MVP.
pub fn pluralize(word: &str) -> String {
    if word.ends_with('y')
        && !word.ends_with("ay")
        && !word.ends_with("ey")
        && !word.ends_with("oy")
        && !word.ends_with("uy")
    {
        format!("{}ies", &word[..word.len() - 1])
    } else if word.ends_with('s')
        || word.ends_with("x")
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        format!("{word}es")
    } else {
        format!("{word}s")
    }
}

/// The table name for a model (snake_case + pluralized): `User` -> `users`.
pub fn table_name(model: &str) -> String {
    pluralize(&snake_case(model))
}

/// Convert a `snake_case` identifier back to `PascalCase`: `blog_post` -> `BlogPost`.
///
/// The inverse of [`snake_case`] for the identifiers Gize generates, used to recover a
/// model's struct name from its module/table name during `gize sync` (ADR-009 revision).
pub fn pascal_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for word in input.split('_').filter(|w| !w.is_empty()) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

/// Singularize a table name, inverting [`pluralize`] for the forms Gize produces:
/// `users` -> `user`, `categories` -> `category`, `boxes` -> `box`.
///
/// This only needs to invert what [`pluralize`] generates (not English at large), so
/// `table_name` round-trips: `singularize(&table_name("User")) == snake_case("User")`.
pub fn singularize(word: &str) -> String {
    if let Some(stem) = word.strip_suffix("ies") {
        // categories -> category (pluralize turned a trailing `y` into `ies`)
        return format!("{stem}y");
    }
    if let Some(stem) = word.strip_suffix("es") {
        // `es` is only added by pluralize to stems ending in s/x/ch/sh (box -> boxes).
        // Otherwise the trailing `s` is a plain plural over a word ending in `e` (house).
        if stem.ends_with('s')
            || stem.ends_with('x')
            || stem.ends_with("ch")
            || stem.ends_with("sh")
        {
            return stem.to_string();
        }
    }
    if let Some(stem) = word.strip_suffix('s') {
        return stem.to_string();
    }
    word.to_string()
}

/// Recover a model's `PascalCase` struct name from its module/table name:
/// `users` -> `User`, `blog_posts` -> `BlogPost`.
pub fn model_name(module: &str) -> String {
    pascal_case(&singularize(module))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_cases() {
        assert_eq!(snake_case("User"), "user");
        assert_eq!(snake_case("BlogPost"), "blog_post");
        assert_eq!(snake_case("OAuthToken"), "o_auth_token");
    }

    #[test]
    fn pluralizes() {
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("day"), "days");
    }

    #[test]
    fn builds_table_names() {
        assert_eq!(table_name("User"), "users");
        assert_eq!(table_name("BlogPost"), "blog_posts");
        assert_eq!(table_name("Category"), "categories");
    }

    #[test]
    fn pascal_cases() {
        assert_eq!(pascal_case("user"), "User");
        assert_eq!(pascal_case("blog_post"), "BlogPost");
        assert_eq!(pascal_case("o_auth_token"), "OAuthToken");
    }

    #[test]
    fn singularizes() {
        assert_eq!(singularize("users"), "user");
        assert_eq!(singularize("categories"), "category");
        assert_eq!(singularize("boxes"), "box");
        assert_eq!(singularize("dishes"), "dish");
        assert_eq!(singularize("days"), "day");
        assert_eq!(singularize("posts"), "post");
    }

    #[test]
    fn model_name_inverts_table_name() {
        // The round-trip `gize sync` relies on: a generated table name recovers its model.
        for model in ["User", "BlogPost", "Category", "OAuthToken", "Product"] {
            assert_eq!(model_name(&table_name(model)), model, "round-trip {model}");
        }
    }
}
