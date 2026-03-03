# wip-git — Architecture

**Git stash, but shared.**

One command to save your working state to the remote. One command to load it anywhere else. No branches, no commits in history, no CI triggers, no noise.

## Mental Model

```
git stash push -m "thing"    →    wip save "thing"
git stash list               →    wip list
git stash pop                →    wip load thing
git stash show -p            →    wip show thing
git stash drop               →    wip drop thing
                                  wip gc              (new: auto-expire old entries)
```

Same UX developers already know. The only difference: it works across machines.

## How It Works

### Save

```
wip save "trying new approach"
```

1. `git add -A` (capture staged + unstaged + untracked)
2. Create a commit on a **detached HEAD** (parent = current HEAD)
3. `git push origin <commit>:refs/wip/<user>/<name> -f`
4. Reset back — local branch is untouched

The commit exists **only** in the hidden ref. Never in any branch.

### Load

```
wip load "trying new approach"
```

1. `git fetch origin refs/wip/<user>/<name>`
2. `git cherry-pick --no-commit FETCH_HEAD`
3. Done — changes appear in working directory, no commit created

Because the WIP commit's parent IS the original HEAD, git has a proper merge base. This means **3-way merge** works natively — no dumb `git apply` that fails on any conflict. If there are conflicts, they show up as normal merge conflicts with `<<<<<<<` markers.

### Why This Works

A git commit already stores:
- The full tree (snapshot of all files)
- The parent (merge base for 3-way diff)
- Author + timestamp
- Message (our metadata)

We're not inventing a format. We're using git's own data model as-is, just storing it in a ref namespace that's invisible to normal workflows.

## Storage: Hidden Refs

```
refs/wip/<user>/<name>
```

Example:
```
refs/wip/alice/new-approach
refs/wip/alice/fix-auth
refs/wip/maria/refactor-db
```

Why `refs/wip/*` and not `refs/heads/*` (branches):
- **Not listed** by `git branch -r` — invisible to team
- **No CI triggers** — most CI watches `refs/heads/*` and `refs/tags/*` only
- **No PR creation prompts** — GitHub/GitLab ignore non-head refs
- **Force-pushable** without drama — it's your namespace
- **Fetchable** — standard git protocol, no extensions needed

### User Namespace

`<user>` defaults to git config `user.name` (slugified: lowercase, spaces→hyphens). Configurable via `wip.user` git config.

Each user owns their namespace. No accidental overwrites across people.

## Data Model

### The WIP Commit

```
tree <full snapshot>
parent <HEAD at time of save>
author Alice Dev <alice@example.com> 1706000000 -0300
committer Alice Dev <alice@example.com> 1706000000 -0300

wip: trying new approach

[wip-push]
branch=feature/dual-mode
files=3
untracked=1
```

The structured `[wip-push]` trailer block stores metadata. Everything before it is the human description. Parsed with simple line matching, no YAML/JSON dependency.

### Metadata Fields

| Field | Description |
|-------|------------|
| `branch` | Source branch at save time |
| `task` | Optional task/ticket identifier (e.g., `AUTH-123`) |
| `files` | Number of changed files |
| `untracked` | Number of untracked files included |

Minimal by design. Git already stores author, timestamp, and the full tree.

## CLI Design

### Commands

```
wip save [name] [-m message] [--task ID]  Save working state to remote
wip load <name> [--pop] [--theirs|--ours]  Apply changes to working directory
wip show <name>                    Show diff (like git stash show -p)
wip list [--all] [--task ID]       List your WIPs (--all: everyone's, --task: filter by task)
wip drop <name>                    Delete a WIP from remote
wip gc [--expire=7d]               Clean entries older than N days
```

### Naming

- Explicit: `wip save auth-fix` → `refs/wip/you/auth-fix-01`
- Auto-increment: saving again → `auth-fix-02`, `auth-fix-03`, ...
- No name: `wip save` → `refs/wip/you/<branch>-01`
- Numeric shorthand: `wip load 0` loads most recent (by timestamp)

### Flags

```
wip save
  -m, --message "description"     Human description (default: "wip")
  -t, --task <id>                 Task/ticket identifier (e.g., AUTH-123)
  -f, --force                     Overwrite existing name without prompt
  --include-ignored               Include .gitignore'd files
  --remote <name>                 Use specific remote (default: origin)

wip load
  --pop                           Delete remote ref after successful load
  --theirs                        On conflict, prefer incoming changes
  --ours                          On conflict, prefer local changes
  --files <glob>                  Load only matching files

wip list
  --all                           Show all users' WIPs
  --task <id>                     Filter by task/ticket identifier
  --remote <name>                 Query specific remote

wip gc
  --expire <duration>             Max age (default: 30d)
  --dry-run                       Show what would be deleted
```

## Conflict Handling

The key advantage over `git apply` or raw patch files.

**On load**, `cherry-pick --no-commit` uses git's 3-way merge:

| Scenario | Behavior |
|----------|----------|
| Clean apply | Changes appear in working directory |
| Conflicts | Standard conflict markers, user resolves normally |
| Dirty working dir | Refuse by default, `--force` to stash-then-apply |

If the working directory has uncommitted changes when loading:
1. Auto-stash local changes
2. Apply WIP via cherry-pick
3. Re-apply local stash
4. If conflicts in step 3, leave both visible for manual resolution

This mirrors `git pull --rebase --autostash` behavior.

## Edge Cases

### Same Name, Multiple Saves
Force-push overwrites. Last save wins. No history per WIP entry — if you want checkpoints, use different names. This matches `git stash` behavior (stash@{0} is always latest).

### Untracked Files
Included by default (like `git stash -u`). The save creates a commit that includes everything, so untracked files are part of the tree.

### Large Files / LFS
Out of scope for v1. Git LFS objects would need to be pushed separately. Punt until someone hits it.

### Submodules
Out of scope. The WIP commit captures the submodule pointer (SHA), not its contents. Same as regular git.

### Multiple Remotes
Default: `origin`. Override with `--remote` or `wip.remote` git config.

### Authentication
Uses whatever git transport the remote uses. If you can `git push origin`, you can `wip save`. No custom auth.

## Implementation

### Language: Rust

The Rust code is a thin CLI wrapper. All heavy lifting is done by `git`. Rust's role:

1. **Parse args** — clap gives proper `--help`, subcommands, validation
2. **Build and run git commands** — orchestrate the sequence
3. **Parse git output** — ref lists, commit messages, metadata trailers
4. **Format terminal output** — colored, tables

A bash script could do this in ~200 lines. Rust over bash because:

- **Arg parsing** — clap vs hand-rolled getopts
- **Error handling** — shell error handling is fragile; Rust makes it hard to ignore failures
- **Metadata parsing** — string parsing in bash gets ugly fast
- **Cross-platform** — works on Windows without WSL/cygwin
- **Single binary** — `cargo install` and done, no PATH hacking
- **Consistent** with other tooling (mcp-watch)

### Git Interface: Shell Out to `git`

**Not** using git2-rs (libgit2 bindings) because:
- Inherits user's git config, SSH keys, credential helpers
- GPG signing just works
- LFS hooks just work
- Simpler to implement and debug
- The overhead of spawning git is negligible for this use case

All git commands are wrapped in a `git()` helper that handles error capture and logging.

### Project Structure

```
src/
  main.rs           CLI entry point (clap)
  commands/
    mod.rs
    save.rs         Save working state
    load.rs         Load with cherry-pick + conflict handling
    show.rs         Display diff
    list.rs         List remote refs + parse metadata
    drop.rs         Delete remote ref
    gc.rs           Expire old entries
  git.rs            Git command wrapper (spawn + capture)
  config.rs         Read wip.* git config values
  ref_name.rs       Ref naming logic (slugify, validate)
  metadata.rs       Parse/write [wip-push] trailer block
```

### Dependencies (minimal)

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
chrono = "0.4"                  # timestamp parsing for gc
colored = "2"                   # terminal output
```

Three dependencies. No tokio (everything is synchronous). No serde (no serialization format). No HTTP client.

### Binary Name

`wip` — short, memorable, no conflict with common tools.

### Distribution

Rust binary distributed via npm (same pattern as esbuild, biome, turbo):

```
npm i -g wip-git      # global install
npx wip-git save x    # zero-install
cargo install wip-git  # for Rust users
```

Platform-specific npm packages (`@wip-git/linux-x64`, `@wip-git/darwin-arm64`, etc.) as `optionalDependencies`. A thin JS wrapper resolves the right binary and execs it. Users don't know or care it's Rust — they just get a fast binary from `npm i`.

This keeps the package minimal: no `node_modules` tree, no JS runtime deps, no runtime overhead. The install is one binary + a shim. Rust compiles everything into a single static executable — the npm package is just the delivery mechanism.

## Config

Git-native config via `git config`:

```ini
[wip]
    user = yourname             # override auto-detected username
    remote = origin             # default remote
    expire = 30d                # default gc expiry
```

No config files. Lives in the same `.gitconfig` or `.git/config` users already manage.

## What This Is NOT

- **Not a branch manager** — no merging, no PRs, no history
- **Not a sync tool** — saves a snapshot, not a live sync
- **Not a backup** — WIPs are ephemeral, designed to be dropped
- **Not a collaboration tool** — your namespace is yours; reading others' WIPs is opt-in via `--all`

## Future Considerations (Not v1)

- **Hooks**: `wip.post-save` / `wip.post-load` — run arbitrary commands after save/load (trigger agents, notify, etc.)
- **Encryption**: encrypt the tree before push for sensitive WIPs
- **Diff between WIPs**: `wip diff auth-fix new-approach`
- **MCP integration**: expose WIP operations as MCP tools for AI agents
- **Team feeds**: subscribe to a teammate's WIP updates
