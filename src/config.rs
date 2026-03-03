use crate::git;

/// Resolve the current user slug: `wip.user` config, falling back to `user.name`.
pub fn user() -> Result<String, String> {
    // Try wip.user first, then fall back to user.name
    if let Ok(user) = git::git_stdout(&["config", "wip.user"])
        && !user.is_empty()
    {
        return Ok(user);
    }

    let name = git::git_stdout(&["config", "user.name"])
        .map_err(|_| "could not determine user: set git user.name or wip.user".to_string())?;

    if name.is_empty() {
        return Err("could not determine user: set git user.name or wip.user".to_string());
    }

    Ok(slugify(&name))
}

/// Return the configured expiry threshold (`wip.expire`), defaulting to `"30d"`.
pub fn default_expire() -> String {
    git::git_stdout(&["config", "wip.expire"]).unwrap_or_else(|_| "30d".to_string())
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
