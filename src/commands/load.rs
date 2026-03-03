use crate::git;
use crate::ref_name;

pub struct LoadResult {
    pub name: String,
    pub conflicts: bool,
    pub popped: bool,
    pub auto_stashed: bool,
}

pub fn run(
    name: String,
    pop: bool,
    theirs: bool,
    ours: bool,
    remote: String,
) -> Result<LoadResult, String> {
    let user = ref_name::user()?;
    let name = ref_name::resolve_name(Some(name))?;
    let wip_ref = ref_name::wip_ref(&name, &user);

    // Auto-stash if working tree is dirty
    let status = git::git_stdout(&["status", "--porcelain"])?;
    let auto_stashed = !status.is_empty();
    if auto_stashed {
        git::git(&[
            "stash",
            "push",
            "--include-untracked",
            "-m",
            "wip-push autostash",
        ])?;
    }

    // Fetch the WIP ref
    git::git(&["fetch", &remote, &wip_ref])?;

    // Cherry-pick --no-commit for 3-way merge
    let mut cp_args = vec!["cherry-pick", "--no-commit", "FETCH_HEAD"];
    if theirs {
        cp_args.extend_from_slice(&["--strategy-option", "theirs"]);
    } else if ours {
        cp_args.extend_from_slice(&["--strategy-option", "ours"]);
    }

    let result = git::git_allow_fail(&cp_args)?;

    let conflicts = result.stderr.contains("CONFLICT") || result.stdout.contains("CONFLICT");

    if conflicts {
        // leave index as-is so user can resolve
    } else if result.stderr.contains("error") {
        return Err(format!("cherry-pick failed: {}", result.stderr));
    } else {
        // Reset index so changes appear as unstaged (like stash pop)
        git::git(&["reset", "HEAD"])?;
    }

    // Restore auto-stashed changes if no conflicts
    if auto_stashed && !conflicts {
        git::git_allow_fail(&["stash", "pop"])?;
    }

    // --pop: delete the remote ref after successful load
    let popped = if pop {
        let delete_refspec = format!(":{wip_ref}");
        git::git(&["push", &remote, &delete_refspec])?;
        true
    } else {
        false
    };

    Ok(LoadResult {
        name,
        conflicts,
        popped,
        auto_stashed,
    })
}
