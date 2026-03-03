use crate::config;
use crate::git;

/// Build the full ref path: refs/wip/<user>/<name>
pub fn wip_ref(name: &str, user: &str) -> String {
    format!("refs/wip/{user}/{name}")
}

/// Resolve a WIP name: use as-is, or auto-generate from branch+hash
pub fn resolve_name(name: Option<String>) -> Result<String, String> {
    match name {
        Some(n) => Ok(slugify(&n)),
        None => auto_name(),
    }
}

/// Auto-generate base name from current branch
fn auto_name() -> Result<String, String> {
    let branch = git::git_stdout(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    Ok(slugify(&branch))
}

/// Get the user slug for ref namespace
pub fn user() -> Result<String, String> {
    config::user()
}

/// Ref pattern for listing: refs/wip/<user>/* or refs/wip/*
pub fn list_pattern(user: Option<&str>) -> String {
    match user {
        Some(u) => format!("refs/wip/{u}/"),
        None => "refs/wip/".to_string(),
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wip_ref() {
        assert_eq!(wip_ref("auth-fix", "alice"), "refs/wip/alice/auth-fix");
        assert_eq!(wip_ref("my-wip", "bob"), "refs/wip/bob/my-wip");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Feature/Auth"), "feature-auth");
        assert_eq!(slugify("--leading-trailing--"), "leading-trailing");
        assert_eq!(slugify("UPPER"), "upper");
        assert_eq!(slugify("special!@#chars"), "special---chars");
        assert_eq!(slugify("already-valid"), "already-valid");
    }

    #[test]
    fn test_list_pattern() {
        assert_eq!(list_pattern(Some("alice")), "refs/wip/alice/");
        assert_eq!(list_pattern(None), "refs/wip/");
    }
}
