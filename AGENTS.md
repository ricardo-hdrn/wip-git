# AGENTS.md — wip-git

Git checkpoint tool that pushes dirty working state to hidden refs. Like `git stash` but shared across machines. Use `--stash` for stash-like save-and-clean behavior.

## Commands

```
wip save [name] [-m msg] [--task ID] [--force] [--stash] [--include-ignored] [--remote R]
wip load <name|index> [--pop] [--theirs|--ours] [--remote R]
wip show <name|index> [--remote R]
wip list [--all] [--task ID] [--remote R]
wip drop <name> [--remote R]
wip gc [--expire 30d] [--dry-run] [--remote R]
wip completions <shell>
wip mcp
```

Bare `wip` (no subcommand) defaults to `wip save`.

## Key behaviors

- **Checkpoint by default**: `wip save` keeps the working tree dirty. Only `--stash` cleans it (hard reset + `git clean -fd`).
- **Auto-increment naming**: `wip save foo` creates `foo-01`, then `foo-02`, etc. `--force` overwrites the exact name.
- **No name**: `wip save` uses `<branch>-NN`.
- **Numeric index**: `wip load 0` loads the most recent entry.
- **3-way merge on load**: Uses `git cherry-pick --no-commit`. Conflicts appear as normal markers. `--theirs`/`--ours` auto-resolve.
- **Auto-stash on load**: If the working tree is dirty, local changes are stashed and restored after load.
- **`--pop`**: Deletes the remote ref after successful load.

## Ref namespace

```
refs/wip/<user>/<name>
```

Hidden from `git branch`, CI, and PR prompts. User is auto-detected from git config or `wip.user`.

## Architecture

- **Language**: Rust, shells out to `git` (no libgit2)
- **Entry point**: `src/main.rs` (clap CLI) → `src/commands/*.rs`
- **Key modules**: `git.rs` (git subprocess), `ref_name.rs` (ref/name resolution), `metadata.rs` (commit message encoding)
- **Tests**: `tests/integration.rs` — creates a bare remote + local clone per test
