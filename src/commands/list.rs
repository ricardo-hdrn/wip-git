use crate::git;
use crate::metadata::WipMetadata;
use crate::ref_name;

pub struct WipEntry {
    pub name: String,
    pub user: String,
    pub sha: String,
    pub metadata: WipMetadata,
    pub timestamp: i64,
}

pub struct ListResult {
    pub entries: Vec<WipEntry>,
}

pub fn run(all: bool, task_filter: Option<String>, remote: String) -> Result<ListResult, String> {
    let user = ref_name::user()?;

    let pattern = if all {
        ref_name::list_pattern(None)
    } else {
        ref_name::list_pattern(Some(&user))
    };

    // List remote refs matching pattern
    let output = git::git_stdout(&["ls-remote", &remote, &format!("{pattern}*")])?;

    if output.is_empty() {
        return Ok(ListResult {
            entries: Vec::new(),
        });
    }

    let mut entries: Vec<WipEntry> = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let sha = parts[0].to_string();
        let refpath = parts[1];

        // Extract user/name from ref path: refs/wip/<user>/<name>
        let suffix = refpath.strip_prefix("refs/wip/").unwrap_or(refpath);
        let (entry_user, entry_name) = match suffix.split_once('/') {
            Some((u, n)) => (u.to_string(), n.to_string()),
            None => (suffix.to_string(), String::new()),
        };

        // Fetch to get metadata and timestamp
        git::git(&["fetch", &remote, refpath])?;
        let msg = git::git_stdout(&["log", "-1", "--format=%B", "FETCH_HEAD"])?;
        let meta = WipMetadata::from_commit_message(&msg);

        // Filter by task if requested
        if let Some(ref task_id) = task_filter
            && meta.task.as_deref() != Some(task_id)
        {
            continue;
        }

        let timestamp_str = git::git_stdout(&["log", "-1", "--format=%ct", "FETCH_HEAD"])?;
        let timestamp: i64 = timestamp_str
            .parse()
            .map_err(|_| format!("bad timestamp for {refpath}"))?;

        entries.push(WipEntry {
            name: entry_name,
            user: entry_user,
            sha,
            metadata: meta,
            timestamp,
        });
    }

    Ok(ListResult { entries })
}

pub fn relative_time(timestamp: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let age = now - timestamp;

    if age < 60 {
        "just now".into()
    } else if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86400 {
        format!("{}h ago", age / 3600)
    } else if age < 86400 * 30 {
        format!("{}d ago", age / 86400)
    } else {
        format!("{}w ago", age / (86400 * 7))
    }
}
