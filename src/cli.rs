use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "trod", version, about = "Persistent directory history with interactive picker")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Pre-fill TUI search filter
    #[arg(long)]
    pub query: Option<String>,

    /// Max entries to show in TUI
    #[arg(long)]
    pub limit: Option<usize>,

    /// Print selected path to stdout instead of launching TUI
    #[arg(long)]
    pub print: bool,

    /// Custom database path
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Record a directory visit (called by shell hook)
    Add {
        /// Directory path to record
        path: String,
    },
    /// Go back n directories in history
    Back {
        /// Number of directories to go back (default: 1)
        #[arg(default_value = "1")]
        n: usize,
    },
    /// Output shell integration code
    Init {
        /// Shell type
        shell: Shell,
    },
    /// Print directory history to stdout
    List {
        /// Sort order
        #[arg(long, default_value = "recent")]
        sort: SortOrder,
        /// Max entries
        #[arg(long)]
        limit: Option<usize>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a directory from history
    Forget {
        /// Directory path to forget
        path: String,
    },
    /// Remove entries for directories that no longer exist
    Clean,
    /// Show usage statistics
    Stats,
    /// Import history from other tools
    Import {
        /// Source tool
        source: ImportSource,
    },
}

#[derive(ValueEnum, Clone)]
pub enum Shell {
    Bash,
    Zsh,
}

#[derive(ValueEnum, Clone)]
pub enum SortOrder {
    Recent,
    Frequent,
}

#[derive(ValueEnum, Clone)]
pub enum ImportSource {
    Zoxide,
    Autojump,
}
