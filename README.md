# wip-git

**Git checkpoint, shared across machines.**

One command to snapshot your working state to the remote. One command to load it
anywhere else. No branches, no commits in history, no CI triggers, no noise.

Your working tree stays dirty by default — it's a checkpoint, not a stash.
Want stash behavior? Use `--stash` to save and clean.

```
wip save auth-fix -m "halfway through OAuth"   # checkpoint to remote
# … switch machines …
wip load auth-fix                               # pull it back
```

## Install

```bash
cargo install wip-git          # Rust toolchain
# or
npm i -g wip-git              # prebuilt binaries via npm
```

## Quick Start

```bash
# save everything (staged + unstaged + untracked)
wip save my-feature -m "WIP login page"

# save and clean working tree (like git stash)
wip save my-feature --stash

# list your saved WIPs
wip list

# load on another machine (3-way merge via cherry-pick)
wip load my-feature

# show the diff without loading
wip show my-feature

# delete a WIP from the remote
wip drop my-feature

# clean entries older than 7 days
wip gc --expire 7d
```

## How It Works

`wip save` stages everything, creates a temporary commit, pushes it to a
hidden ref (`refs/wip/<user>/<name>`), then resets — your local branch is
untouched and your working tree stays dirty. With `--stash`, the working tree
is cleaned instead (hard reset + clean).

`wip load` fetches that ref and applies it via `git cherry-pick --no-commit`,
giving you a proper 3-way merge. Conflicts show up as normal git conflict
markers.

Hidden refs (`refs/wip/*`) are invisible to `git branch`, don't trigger CI,
and don't create PR prompts.

## Commands

| Command | Description |
|---------|-------------|
| `wip save [name] [-m msg] [--task ID] [--stash]` | Save working state to remote |
| `wip load <name> [--pop] [--theirs\|--ours]` | Apply changes via 3-way merge |
| `wip show <name>` | Show metadata and diff |
| `wip list [--all] [--task ID]` | List your WIP entries |
| `wip drop <name>` | Delete a WIP from remote |
| `wip gc [--expire 30d] [--dry-run]` | Clean old entries |
| `wip completions <shell>` | Generate shell completions |
| `wip mcp` | Start MCP server (stdio) |

## Naming

- Explicit: `wip save auth-fix` → `refs/wip/you/auth-fix-01`
- Auto-increment: saving the same name again → `auth-fix-02`, `auth-fix-03`, ...
- No name: `wip save` → `refs/wip/you/<branch>-01`
- Exact name with `--force`: `wip save auth-fix --force` overwrites without incrementing
- Numeric index: `wip load 0` loads the most recent entry

## Configuration

Git-native config — no extra files:

```ini
[wip]
    user = yourname         # override auto-detected username
    remote = origin         # default remote (flag: --remote)
    expire = 30d            # default gc expiry
```

## Shell Completions

```bash
# bash
wip completions bash > ~/.bash_completion.d/wip

# zsh
wip completions zsh > ~/.zfunc/_wip

# fish
wip completions fish > ~/.config/fish/completions/wip.fish
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for design decisions, data model, and
implementation details.

## License

[MIT](LICENSE)
