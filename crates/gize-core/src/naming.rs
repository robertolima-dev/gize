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
}
