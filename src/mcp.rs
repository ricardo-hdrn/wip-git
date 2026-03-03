use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;

use wip_git::commands;

#[derive(Clone)]
pub struct WipServer {
    tool_router: ToolRouter<Self>,
}

// ── Tool parameter structs ─────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for saving working tree changes")]
struct SaveParams {
    #[schemars(description = "WIP name (auto-generated from branch if omitted)")]
    name: Option<String>,
    #[schemars(description = "Human description of the changes")]
    message: Option<String>,
    #[schemars(description = "Task/ticket identifier (e.g. JIRA-123)")]
    task: Option<String>,
    #[schemars(description = "Overwrite existing WIP with same name")]
    force: Option<bool>,
    #[schemars(description = "Include .gitignore'd files")]
    include_ignored: Option<bool>,
    #[schemars(description = "Save and clean working tree (like git stash)")]
    stash: Option<bool>,
    #[schemars(description = "Git remote name (default: origin)")]
    remote: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for loading saved changes")]
struct LoadParams {
    #[schemars(description = "WIP name to load")]
    name: String,
    #[schemars(description = "Delete remote ref after successful load")]
    pop: Option<bool>,
    #[schemars(description = "On conflict, prefer incoming changes")]
    theirs: Option<bool>,
    #[schemars(description = "On conflict, prefer local changes")]
    ours: Option<bool>,
    #[schemars(description = "Git remote name (default: origin)")]
    remote: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for showing a WIP entry")]
struct ShowParams {
    #[schemars(description = "WIP name to show")]
    name: String,
    #[schemars(description = "Git remote name (default: origin)")]
    remote: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for listing WIP entries")]
struct ListParams {
    #[schemars(description = "Show all users' WIPs (not just yours)")]
    all: Option<bool>,
    #[schemars(description = "Filter by task/ticket identifier")]
    task: Option<String>,
    #[schemars(description = "Git remote name (default: origin)")]
    remote: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for deleting a WIP entry")]
struct DropParams {
    #[schemars(description = "WIP name to delete")]
    name: String,
    #[schemars(description = "Git remote name (default: origin)")]
    remote: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for garbage-collecting old WIP entries")]
struct GcParams {
    #[schemars(description = "Max age threshold (e.g. 7d, 24h). Default: 30d")]
    expire: Option<String>,
    #[schemars(description = "Show what would be deleted without deleting")]
    dry_run: Option<bool>,
    #[schemars(description = "Git remote name (default: origin)")]
    remote: Option<String>,
}

// ── Tool implementations ───────────────────────────────────────────

#[tool_router]
impl WipServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Save working tree changes (staged + unstaged + untracked) to a shared remote ref. Like git stash push, but pushes to refs/wip/ on the remote so you can load it from another machine."
    )]
    async fn wip_save(
        &self,
        Parameters(p): Parameters<SaveParams>,
    ) -> Result<CallToolResult, McpError> {
        let r = commands::save::run(
            p.name,
            p.message.unwrap_or_else(|| "wip".into()),
            p.task,
            p.force.unwrap_or(false),
            p.include_ignored.unwrap_or(false),
            p.stash.unwrap_or(false),
            p.remote.unwrap_or_else(|| "origin".into()),
        )
        .map_err(|e| McpError::internal_error(e, None))?;

        let text = if r.clean {
            "Nothing to save — working tree clean.".to_string()
        } else {
            let verb = if r.stashed { "Stashed" } else { "Saved" };
            let mut out = format!(
                "{verb} \"{}\" → {} ({})\n{} files, {} untracked",
                r.name,
                r.wip_ref,
                &r.sha[..7],
                r.files,
                r.untracked
            );
            if let Some(ref t) = r.task {
                out.push_str(&format!("\nTask: {t}"));
            }
            out
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Apply saved WIP changes to the current working directory via cherry-pick. Like git stash pop, but pulls from a remote ref."
    )]
    async fn wip_load(
        &self,
        Parameters(p): Parameters<LoadParams>,
    ) -> Result<CallToolResult, McpError> {
        let r = commands::load::run(
            p.name,
            p.pop.unwrap_or(false),
            p.theirs.unwrap_or(false),
            p.ours.unwrap_or(false),
            p.remote.unwrap_or_else(|| "origin".into()),
        )
        .map_err(|e| McpError::internal_error(e, None))?;

        let mut text = if r.conflicts {
            let mut s = format!(
                "Loaded \"{}\" with CONFLICTS — resolve them manually.",
                r.name
            );
            if r.auto_stashed {
                s.push_str("\nLocal changes stashed — run 'git stash pop' after resolving.");
            }
            s
        } else {
            let mut s = format!("Loaded \"{}\" successfully.", r.name);
            if r.auto_stashed {
                s.push_str("\nAuto-stashed local changes restored.");
            }
            s
        };
        if r.popped {
            text.push_str("\nRemote ref dropped.");
        }

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Show metadata (message, branch, task, file count) and full diff of a WIP entry."
    )]
    async fn wip_show(
        &self,
        Parameters(p): Parameters<ShowParams>,
    ) -> Result<CallToolResult, McpError> {
        let r = commands::show::run(p.name, p.remote.unwrap_or_else(|| "origin".into()))
            .map_err(|e| McpError::internal_error(e, None))?;

        let m = &r.metadata;
        let mut text = format!(
            "WIP: {}\n  message: {}\n  branch: {}\n  files: {}, untracked: {}",
            r.name, m.message, m.branch, m.files, m.untracked
        );
        if let Some(ref task) = m.task {
            text.push_str(&format!("\n  task: {task}"));
        }
        text.push_str(&format!("\n\n{}", r.diff));

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "List WIP entries on the remote. Shows name, user, and short SHA for each entry."
    )]
    async fn wip_list(
        &self,
        Parameters(p): Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        let r = commands::list::run(
            p.all.unwrap_or(false),
            p.task,
            p.remote.unwrap_or_else(|| "origin".into()),
        )
        .map_err(|e| McpError::internal_error(e, None))?;

        let text = if r.entries.is_empty() {
            "No WIP entries found.".to_string()
        } else {
            let mut lines = Vec::new();
            for entry in &r.entries {
                let short = &entry.sha[..7.min(entry.sha.len())];
                let age = wip_git::commands::list::relative_time(entry.timestamp);
                let meta = &entry.metadata;
                let mut line = format!("{}/{} {}", entry.user, entry.name, short);
                if !meta.message.is_empty() && meta.message != "wip" {
                    line.push_str(&format!(" \"{}\"", meta.message));
                }
                line.push_str(&format!(" {} {}", meta.branch, age));
                if let Some(ref task) = meta.task {
                    line.push_str(&format!(" [{}]", task));
                }
                lines.push(line);
            }
            lines.join("\n")
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Delete a WIP entry from the remote.")]
    async fn wip_drop(
        &self,
        Parameters(p): Parameters<DropParams>,
    ) -> Result<CallToolResult, McpError> {
        let r = commands::drop::run(p.name, p.remote.unwrap_or_else(|| "origin".into()))
            .map_err(|e| McpError::internal_error(e, None))?;

        let text = format!("Dropped \"{}\" ({})", r.name, r.wip_ref);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Garbage-collect WIP entries older than the given threshold. Only affects your own entries."
    )]
    async fn wip_gc(
        &self,
        Parameters(p): Parameters<GcParams>,
    ) -> Result<CallToolResult, McpError> {
        let r = commands::gc::run(
            p.expire.unwrap_or_else(|| "30d".into()),
            p.dry_run.unwrap_or(false),
            p.remote.unwrap_or_else(|| "origin".into()),
        )
        .map_err(|e| McpError::internal_error(e, None))?;

        let text = if r.entries.is_empty() {
            "Nothing to clean.".to_string()
        } else {
            let mut lines: Vec<String> = r
                .entries
                .iter()
                .map(|e| {
                    let verb = if r.dry_run { "would drop" } else { "dropped" };
                    format!("{} {} ({}d old)", verb, e.name, e.age_days)
                })
                .collect();
            if r.dry_run {
                lines.push(format!(
                    "\n{} entries would be dropped. Run without dry_run to delete.",
                    r.entries.len()
                ));
            }
            lines.join("\n")
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

// ── ServerHandler ──────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for WipServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "wip".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Git stash, but shared across machines. \
                 Save/load working tree changes via hidden remote refs (refs/wip/)."
                    .into(),
            ),
            ..Default::default()
        }
    }
}

// ── Entry point ────────────────────────────────────────────────────

pub async fn serve() -> Result<(), String> {
    let service = WipServer::new()
        .serve(stdio())
        .await
        .map_err(|e| e.to_string())?;

    service.waiting().await.map_err(|e| e.to_string())?;
    Ok(())
}
