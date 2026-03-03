# Changelog

## 0.1.0

Initial release.

- `wip save` — stage everything, commit to hidden ref, push to remote
- `wip load` — fetch and cherry-pick with 3-way merge
- `wip show` — display metadata and diff
- `wip list` — list WIP entries (yours or everyone's)
- `wip drop` — delete a WIP from the remote
- `wip gc` — expire old entries by age
- `wip completions` — generate shell completions (bash/zsh/fish/etc.)
- `wip mcp` — MCP server over stdio for AI agent integration
- Auto-generated names from `<branch>-<short-hash>`
- Numeric index support (`wip load 0`)
- Task/ticket tagging (`--task PROJ-42`)
- Conflict resolution flags (`--theirs`, `--ours`)
- Dirty working tree rejection on load
