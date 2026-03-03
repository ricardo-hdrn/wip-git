use crate::git;
use crate::ref_name;

pub struct ExpiredEntry {
    pub name: String,
    pub wip_ref: String,
    pub age_days: i64,
}

pub struct GcResult {
    pub entries: Vec<ExpiredEntry>,
    pub dry_run: bool,
}

pub fn run(expire: String, dry_run: bool, remote: String) -> Result<GcResult, String> {
    let user = ref_name::user()?;
    let pattern = ref_name::list_pattern(Some(&user));
    let max_age_secs = parse_duration(&expire)?;

    // List all our WIP refs
    let output = git::git_stdout(&["ls-remote", &remote, &format!("{pattern}*")])?;

    if output.is_empty() {
        return Ok(GcResult {
            entries: Vec::new(),
            dry_run,
        });
    }

    let now = chrono::Utc::now().timestamp();
    let mut entries = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let refpath = parts[1];

        // Fetch to inspect commit timestamp
        git::git(&["fetch", &remote, refpath])?;
        let timestamp_str = git::git_stdout(&["log", "-1", "--format=%ct", "FETCH_HEAD"])?;
        let timestamp: i64 = timestamp_str
            .parse()
            .map_err(|_| format!("bad timestamp for {refpath}"))?;

        let age = now - timestamp;
        if age > max_age_secs {
            let display = refpath.strip_prefix("refs/wip/").unwrap_or(refpath);
            let days = age / 86400;

            if !dry_run {
                let delete_refspec = format!(":{refpath}");
                git::git(&["push", &remote, &delete_refspec])?;
            }

            entries.push(ExpiredEntry {
                name: display.to_string(),
                wip_ref: refpath.to_string(),
                age_days: days,
            });
        }
    }

    Ok(GcResult { entries, dry_run })
}

pub fn parse_duration(s: &str) -> Result<i64, String> {
    let s = s.trim();
    if let Some(days) = s.strip_suffix('d') {
        let n: i64 = days.parse().map_err(|_| format!("invalid duration: {s}"))?;
        Ok(n * 86400)
    } else if let Some(hours) = s.strip_suffix('h') {
        let n: i64 = hours
            .parse()
            .map_err(|_| format!("invalid duration: {s}"))?;
        Ok(n * 3600)
    } else {
        Err(format!("invalid duration: {s} (use e.g. 7d, 24h)"))
    }
}
