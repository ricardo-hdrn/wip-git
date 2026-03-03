mod mcp;

use clap::{CommandFactory, Parser, Subcommand};
use colored::Colorize;

use wip_git::commands;
use wip_git::commands::list::relative_time;

#[derive(Parser)]
#[command(name = "wip", about = "Git stash, but shared.", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Save working state to remote
    Save {
        /// WIP name (default: <branch>-<short-hash>)
        name: Option<String>,

        /// Human description
        #[arg(short, long, default_value = "wip")]
        message: String,

        /// Task/ticket identifier
        #[arg(short, long)]
        task: Option<String>,

        /// Overwrite existing name without prompt
        #[arg(short, long)]
        force: bool,

        /// Include .gitignore'd files
        #[arg(long)]
        include_ignored: bool,

        /// Save and clean working tree (like git stash)
        #[arg(long)]
        stash: bool,

        /// Use specific remote
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Apply changes to working directory
    Load {
        /// WIP name or numeric index (0 = most recent)
        name: String,

        /// Delete remote ref after successful load
        #[arg(long)]
        pop: bool,

        /// On conflict, prefer incoming changes
        #[arg(long, conflicts_with = "ours")]
        theirs: bool,

        /// On conflict, prefer local changes
        #[arg(long, conflicts_with = "theirs")]
        ours: bool,

        /// Use specific remote
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Show diff of a WIP entry
    Show {
        /// WIP name or numeric index
        name: String,

        /// Use specific remote
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// List your WIP entries
    List {
        /// Show all users' WIPs
        #[arg(long)]
        all: bool,

        /// Filter by task/ticket identifier
        #[arg(long)]
        task: Option<String>,

        /// Use specific remote
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Delete a WIP entry from remote
    Drop {
        /// WIP name
        name: String,

        /// Use specific remote
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Clean entries older than N days
    Gc {
        /// Max age (e.g., 7d, 30d)
        #[arg(long, default_value = "30d")]
        expire: String,

        /// Show what would be deleted
        #[arg(long)]
        dry_run: bool,

        /// Use specific remote
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate for
        shell: clap_complete::Shell,
    },

    /// Start MCP server (stdio transport)
    Mcp,
}

fn main() {
    let cli = Cli::parse();

    let command = cli.command.unwrap_or(Command::Save {
        name: None,
        message: "wip".into(),
        task: None,
        force: false,
        include_ignored: false,
        stash: false,
        remote: "origin".into(),
    });

    let result: Result<(), String> = match command {
        Command::Save {
            name,
            message,
            task,
            force,
            include_ignored,
            stash,
            remote,
        } => commands::save::run(name, message, task, force, include_ignored, stash, remote).map(
            |r| {
                if r.clean {
                    println!("{}", "nothing to save — working tree clean".yellow());
                } else {
                    let verb = if r.stashed { "stashed" } else { "saved" };
                    let short_sha = &r.sha[..7];
                    println!(
                        "{} {} → {} ({})",
                        verb.green().bold(),
                        r.name.bold(),
                        r.wip_ref.dimmed(),
                        short_sha
                    );
                    if let Some(ref t) = r.task {
                        println!("  {} {}", "task:".dimmed(), t);
                    }
                    println!("  {} files, {} untracked", r.files, r.untracked);
                }
            },
        ),

        Command::Load {
            name,
            pop,
            theirs,
            ours,
            remote,
        } => commands::load::run(name, pop, theirs, ours, remote).map(|r| {
            if r.conflicts {
                println!(
                    "{} applied with conflicts — resolve them manually",
                    "warning:".yellow().bold()
                );
                println!(
                    "  {} use 'git cherry-pick --abort' to undo",
                    "hint:".dimmed()
                );
                if r.auto_stashed {
                    println!(
                        "  {} local changes stashed — run 'git stash pop' after resolving",
                        "hint:".dimmed()
                    );
                }
            } else {
                println!("{} {}", "loaded".green().bold(), r.name.bold());
                if r.auto_stashed {
                    println!("  {} auto-stashed local changes restored", "note:".dimmed());
                }
            }
            if r.popped {
                println!("  {} remote ref", "dropped".dimmed());
            }
        }),

        Command::Show { name, remote } => commands::show::run(name, remote).map(|r| {
            println!("{} {}", "wip:".bold(), r.name.bold());
            println!("  {} {}", "message:".dimmed(), r.metadata.message);
            println!("  {} {}", "branch:".dimmed(), r.metadata.branch);
            if let Some(ref task) = r.metadata.task {
                println!("  {} {}", "task:".dimmed(), task);
            }
            println!(
                "  {} files, {} untracked",
                r.metadata.files, r.metadata.untracked
            );
            println!();
            println!("{}", r.diff);
        }),

        Command::List { all, task, remote } => commands::list::run(all, task, remote).map(|r| {
            if r.entries.is_empty() {
                println!("{}", "no WIP entries found".dimmed());
            } else {
                for entry in &r.entries {
                    let display = format!("{}/{}", entry.user, entry.name);
                    let age = relative_time(entry.timestamp);
                    let meta = &entry.metadata;
                    print!("  {}", display.bold());
                    if !meta.message.is_empty() && meta.message != "wip" {
                        print!("  \"{}\"", meta.message);
                    }
                    print!("  {}", meta.branch.dimmed());
                    print!("  {}", age.dimmed());
                    if let Some(ref task) = meta.task {
                        print!("  [{}]", task.cyan());
                    }
                    println!();
                }
            }
        }),

        Command::Drop { name, remote } => commands::drop::run(name, remote).map(|r| {
            println!("{} {}", "dropped".green().bold(), r.name.bold());
        }),

        Command::Gc {
            expire,
            dry_run,
            remote,
        } => commands::gc::run(expire, dry_run, remote).map(|r| {
            if r.entries.is_empty() {
                println!("{}", "nothing to clean".dimmed());
            } else {
                for entry in &r.entries {
                    if r.dry_run {
                        println!(
                            "  {} {} ({}d old)",
                            "would drop".yellow(),
                            entry.name,
                            entry.age_days
                        );
                    } else {
                        println!(
                            "  {} {} ({}d old)",
                            "dropped".red(),
                            entry.name,
                            entry.age_days
                        );
                    }
                }
                if r.dry_run {
                    println!(
                        "\n{} entries would be dropped. Run without --dry-run to delete.",
                        r.entries.len()
                    );
                }
            }
        }),

        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "wip", &mut std::io::stdout());
            Ok(())
        }

        Command::Mcp => match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt.block_on(mcp::serve()),
            Err(e) => Err(format!("failed to create async runtime: {e}")),
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {}", "error".red(), e);
        std::process::exit(1);
    }
}
