use crate::git;
use crate::metadata::WipMetadata;
use crate::ref_name;

pub struct ShowResult {
    pub name: String,
    pub metadata: WipMetadata,
    pub diff: String,
}

pub fn run(name: String, remote: String) -> Result<ShowResult, String> {
    let user = ref_name::user()?;
    let name = ref_name::resolve_name(Some(name))?;
    let wip_ref = ref_name::wip_ref(&name, &user);

    // Fetch the ref
    git::git(&["fetch", &remote, &wip_ref])?;

    // Show metadata from commit message
    let msg = git::git_stdout(&["log", "-1", "--format=%B", "FETCH_HEAD"])?;
    let metadata = WipMetadata::from_commit_message(&msg);

    // Show the diff (compare commit to its parent)
    let diff = git::git_stdout(&["diff", "FETCH_HEAD~1..FETCH_HEAD"])?;

    Ok(ShowResult {
        name,
        metadata,
        diff,
    })
}
