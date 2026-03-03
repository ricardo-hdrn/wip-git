use crate::git;
use crate::metadata::WipMetadata;
use crate::ref_name;

pub struct SaveResult {
    pub name: String,
    pub wip_ref: String,
    pub sha: String,
    pub files: usize,
    pub untracked: usize,
    pub task: Option<String>,
    pub clean: bool,
    pub stashed: bool,
}

/// Find the next auto-increment suffix for a base name.
/// Scans `refs/wip/<user>/<base>-*` on the remote, parses `-NN` suffixes,
/// and returns `<base>-{max+1:02}`.
fn next_increment(base: &str, user: &str, remote: &str) -> Result<String, String> {
    let pattern = ref_name::wip_ref(&format!("{base}-*"), user);
    let ls = git::git_stdout(&["ls-remote", remote, &pattern])
        .map_err(|e| format!("could not reach remote '{}': {}", remote, e))?;

    let prefix = format!("refs/wip/{user}/{base}-");
    let max_n = ls
        .lines()
        .filter_map(|line| {
            let refname = line.split_whitespace().nth(1)?;
            let suffix = refname.strip_prefix(&prefix)?;
            suffix.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);

    Ok(format!("{base}-{:02}", max_n + 1))
}

pub fn run(
    name: Option<String>,
    message: String,
    task: Option<String>,
    force: bool,
    include_ignored: bool,
    stash: bool,
    remote: String,
) -> Result<SaveResult, String> {
    let user = ref_name::user()?;
    let base = ref_name::resolve_name(name)?;
    let (name, wip_ref) = if force {
        // --force: exact name, overwrite (backward compat)
        let wip_ref = ref_name::wip_ref(&base, &user);
        (base, wip_ref)
    } else {
        // auto-increment
        let name = next_increment(&base, &user, &remote)?;
        let wip_ref = ref_name::wip_ref(&name, &user);
        (name, wip_ref)
    };

    // Remember current state
    let original_head = git::git_stdout(&["rev-parse", "HEAD"])?;
    let branch = git::git_stdout(&["rev-parse", "--abbrev-ref", "HEAD"])?;

    // Stage everything
    if include_ignored {
        git::git(&["add", "-A", "--force"])?;
    } else {
        git::git(&["add", "-A"])?;
    }

    // Count what we're saving
    let status = git::git_stdout(&["status", "--porcelain"])?;
    if status.is_empty() {
        return Ok(SaveResult {
            name,
            wip_ref,
            sha: String::new(),
            files: 0,
            untracked: 0,
            task,
            clean: true,
            stashed: false,
        });
    }

    let files = status.lines().count();
    let untracked = status
        .lines()
        .filter(|l| l.starts_with("A ") || l.starts_with("??"))
        .count();

    // Build commit message with metadata
    let meta = WipMetadata {
        message: message.clone(),
        branch: branch.clone(),
        task: task.clone(),
        files,
        untracked,
    };

    // Create detached commit
    let commit_msg = meta.to_commit_message();
    git::git(&["commit", "--allow-empty", "-m", &commit_msg])?;
    let wip_sha = git::git_stdout(&["rev-parse", "HEAD"])?;

    // Push to hidden ref
    let refspec = format!("{wip_sha}:{wip_ref}");
    let push_result = git::git(&["push", &remote, &refspec, "--force"]);

    // Reset back regardless of push result
    if stash {
        git::git(&["reset", &original_head, "--hard"])?;
        git::git(&["clean", "-fd"])?;
    } else {
        git::git(&["reset", &original_head, "--mixed"])?;
    }

    // Now check if push succeeded
    push_result?;

    Ok(SaveResult {
        name,
        wip_ref,
        sha: wip_sha,
        files,
        untracked,
        task,
        clean: false,
        stashed: stash,
    })
}
