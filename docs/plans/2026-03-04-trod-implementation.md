# trod Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build `trod`, a persistent directory history tool with interactive TUI picker, in Rust.

**Architecture:** Single binary, no daemon. Shell hooks call `trod add` on every `cd`, SQLite stores history, TUI picker lets users browse and select. Shell integration aliases `trod` to `td`.

**Tech Stack:** Rust, clap (CLI), rusqlite (SQLite), ratatui + crossterm (TUI), fuzzy-matcher (search)

**Design doc:** `docs/plans/2026-03-04-trod-design.md`

---

### Task 0: Install Rust Toolchain

**Step 1: Install rustup and stable toolchain**

Run: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`
Then: `source "$HOME/.cargo/env"`
Expected: `rustc --version` prints version

**Step 2: Verify**

Run: `cargo --version`
Expected: prints version

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/db.rs`
- Create: `src/cli.rs`

**Step 1: Initialize cargo project**

Run: `cargo init /Users/mo/personal_code/dir --name trod`

**Step 2: Add dependencies to Cargo.toml**

Replace `Cargo.toml` with:

```toml
[package]
name = "trod"
version = "0.1.0"
edition = "2021"
description = "A persistent directory history tool with interactive TUI picker"
license = "MIT"
repository = "https://github.com/YOUR_USER/trod"

[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = "0.4"
dirs = "5"
anyhow = "1"
fuzzy-matcher = "0.3"
ratatui = "0.29"
crossterm = "0.28"
```

**Step 3: Set up module structure**

Create `src/lib.rs`:
```rust
pub mod cli;
pub mod db;
```

Create `src/cli.rs` (empty placeholder):
```rust
// CLI argument definitions (clap)
```

Create `src/db.rs` (empty placeholder):
```rust
// Database layer
```

Replace `src/main.rs` with:
```rust
use anyhow::Result;

mod cli;
mod db;

fn main() -> Result<()> {
    Ok(())
}
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: compiles with no errors (warnings OK)

**Step 5: Commit**

```bash
git init
echo "target/" > .gitignore
git add .
git commit -m "feat: initialize trod project with dependencies"
```

---

### Task 2: Database Layer

**Files:**
- Modify: `src/db.rs`
- Create: `tests/db_tests.rs`

**Step 1: Write failing tests for database operations**

Create `tests/db_tests.rs`:

```rust
use trod::db::Database;
use std::path::Path;
use tempfile::NamedTempFile;

fn test_db() -> Database {
    let tmp = NamedTempFile::new().unwrap();
    Database::open(tmp.path()).unwrap()
}

#[test]
fn test_add_directory() {
    let db = test_db();
    db.add("/home/user/projects").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].path, "/home/user/projects");
    assert_eq!(dirs[0].visit_count, 1);
}

#[test]
fn test_add_directory_twice_increments_count() {
    let db = test_db();
    db.add("/home/user/projects").unwrap();
    db.add("/home/user/projects").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].visit_count, 2);
}

#[test]
fn test_list_recent_ordering() {
    let db = test_db();
    db.add("/first").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.add("/second").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs[0].path, "/second");
    assert_eq!(dirs[1].path, "/first");
}

#[test]
fn test_forget_directory() {
    let db = test_db();
    db.add("/home/user/projects").unwrap();
    db.forget("/home/user/projects").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn test_list_frequent_ordering() {
    let db = test_db();
    db.add("/rare").unwrap();
    db.add("/common").unwrap();
    db.add("/common").unwrap();
    db.add("/common").unwrap();

    let dirs = db.list_frequent(10).unwrap();
    assert_eq!(dirs[0].path, "/common");
    assert_eq!(dirs[1].path, "/rare");
}

#[test]
fn test_clean_removes_nonexistent() {
    let db = test_db();
    db.add("/this/path/does/not/exist/at/all").unwrap();
    let removed = db.clean().unwrap();
    assert_eq!(removed, 1);

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn test_stats() {
    let db = test_db();
    db.add("/a").unwrap();
    db.add("/b").unwrap();
    db.add("/a").unwrap();

    let stats = db.stats().unwrap();
    assert_eq!(stats.total_directories, 2);
    assert_eq!(stats.total_visits, 3);
}

#[test]
fn test_back_returns_nth_previous() {
    let db = test_db();
    db.add("/first").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.add("/second").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.add("/third").unwrap();

    // back(1) = second most recent
    let path = db.back(1).unwrap();
    assert_eq!(path, Some("/second".to_string()));

    let path = db.back(2).unwrap();
    assert_eq!(path, Some("/first".to_string()));

    let path = db.back(10).unwrap();
    assert_eq!(path, None);
}
```

**Step 2: Add tempfile dev-dependency**

Add to `Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test db_tests`
Expected: FAIL — `Database` not defined

**Step 4: Implement the database layer**

Replace `src/db.rs` with:

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: String,
    pub visit_count: i64,
    pub last_visited: DateTime<Utc>,
    pub first_visited: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Stats {
    pub total_directories: i64,
    pub total_visits: i64,
    pub most_visited: Option<DirEntry>,
    pub oldest_entry: Option<DirEntry>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_default() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .context("Could not determine data directory")?
            .join("trod");
        std::fs::create_dir_all(&data_dir)?;
        Self::open(&data_dir.join("history.db"))
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS directories (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                visit_count INTEGER DEFAULT 1,
                last_visited TEXT NOT NULL,
                first_visited TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS visits (
                id INTEGER PRIMARY KEY,
                directory_id INTEGER NOT NULL REFERENCES directories(id) ON DELETE CASCADE,
                timestamp TEXT NOT NULL,
                session_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_directories_last_visited ON directories(last_visited);
            CREATE INDEX IF NOT EXISTS idx_directories_path ON directories(path);
            CREATE INDEX IF NOT EXISTS idx_visits_timestamp ON visits(timestamp);",
        )?;
        Ok(())
    }

    pub fn add(&self, path: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO directories (path, visit_count, last_visited, first_visited)
             VALUES (?1, 1, ?2, ?2)
             ON CONFLICT(path) DO UPDATE SET
                visit_count = visit_count + 1,
                last_visited = ?2",
            params![path, now],
        )?;
        // Also record in visits table
        let dir_id: i64 = self.conn.query_row(
            "SELECT id FROM directories WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO visits (directory_id, timestamp) VALUES (?1, ?2)",
            params![dir_id, now],
        )?;
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<DirEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, visit_count, last_visited, first_visited
             FROM directories
             ORDER BY last_visited DESC
             LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit as i64], |row| {
                Ok(DirEntry {
                    path: row.get(0)?,
                    visit_count: row.get(1)?,
                    last_visited: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    first_visited: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn list_frequent(&self, limit: usize) -> Result<Vec<DirEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, visit_count, last_visited, first_visited
             FROM directories
             ORDER BY visit_count DESC, last_visited DESC
             LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit as i64], |row| {
                Ok(DirEntry {
                    path: row.get(0)?,
                    visit_count: row.get(1)?,
                    last_visited: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    first_visited: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub fn forget(&self, path: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM visits WHERE directory_id IN (SELECT id FROM directories WHERE path = ?1)",
            params![path],
        )?;
        self.conn
            .execute("DELETE FROM directories WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn clean(&self) -> Result<usize> {
        let paths: Vec<String> = {
            let mut stmt = self.conn.prepare("SELECT path FROM directories")?;
            stmt.query_map([], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut removed = 0;
        for path in paths {
            if !Path::new(&path).exists() {
                self.forget(&path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn stats(&self) -> Result<Stats> {
        let total_directories: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM directories", [], |row| row.get(0))?;
        let total_visits: i64 = self
            .conn
            .query_row("SELECT COALESCE(SUM(visit_count), 0) FROM directories", [], |row| {
                row.get(0)
            })?;
        let most_visited = self.list_frequent(1)?.into_iter().next();
        let oldest_entry = {
            let mut stmt = self.conn.prepare(
                "SELECT path, visit_count, last_visited, first_visited
                 FROM directories ORDER BY first_visited ASC LIMIT 1",
            )?;
            stmt.query_map([], |row| {
                Ok(DirEntry {
                    path: row.get(0)?,
                    visit_count: row.get(1)?,
                    last_visited: row
                        .get::<_, String>(2)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    first_visited: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .next()
        };
        Ok(Stats {
            total_directories,
            total_visits,
            most_visited,
            oldest_entry,
        })
    }

    pub fn back(&self, n: usize) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT path FROM directories ORDER BY last_visited DESC LIMIT 1 OFFSET ?1",
        )?;
        let result = stmt
            .query_map(params![n as i64], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(result.into_iter().next())
    }

    pub fn all_entries(&self) -> Result<Vec<DirEntry>> {
        self.list_recent(10000)
    }
}
```

**Step 5: Run tests**

Run: `cargo test --test db_tests`
Expected: all 8 tests PASS

**Step 6: Commit**

```bash
git add src/db.rs tests/db_tests.rs Cargo.toml
git commit -m "feat: implement database layer with SQLite"
```

---

### Task 3: CLI Argument Parsing

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Define CLI with clap**

Replace `src/cli.rs` with:

```rust
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
```

**Step 2: Wire up main.rs with command dispatch**

Replace `src/main.rs` with:

```rust
mod cli;
mod db;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, Shell, SortOrder};
use db::Database;

fn open_db(cli: &Cli) -> Result<Database> {
    match &cli.db {
        Some(path) => Database::open(path),
        None => Database::open_default(),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Add { path }) => {
            let db = open_db(&cli)?;
            db.add(path)?;
        }
        Some(Command::Back { n }) => {
            let db = open_db(&cli)?;
            match db.back(*n)? {
                Some(path) => println!("{}", path),
                None => eprintln!("No directory {} steps back in history", n),
            }
        }
        Some(Command::Init { shell }) => {
            print_init(shell);
        }
        Some(Command::List { sort, limit, json }) => {
            let db = open_db(&cli)?;
            let limit = limit.unwrap_or(50);
            let entries = match sort {
                SortOrder::Recent => db.list_recent(limit)?,
                SortOrder::Frequent => db.list_frequent(limit)?,
            };
            if *json {
                print_json(&entries)?;
            } else {
                print_list(&entries);
            }
        }
        Some(Command::Forget { path }) => {
            let db = open_db(&cli)?;
            db.forget(path)?;
            eprintln!("Forgot: {}", path);
        }
        Some(Command::Clean) => {
            let db = open_db(&cli)?;
            let removed = db.clean()?;
            eprintln!("Removed {} non-existent directories", removed);
        }
        Some(Command::Stats) => {
            let db = open_db(&cli)?;
            let stats = db.stats()?;
            eprintln!("Directories tracked: {}", stats.total_directories);
            eprintln!("Total visits:        {}", stats.total_visits);
            if let Some(top) = &stats.most_visited {
                eprintln!("Most visited:        {} ({} visits)", top.path, top.visit_count);
            }
        }
        Some(Command::Import { source: _ }) => {
            eprintln!("Import not yet implemented");
        }
        None => {
            // TUI mode — will be implemented in Task 5
            let db = open_db(&cli)?;
            let entries = db.list_recent(cli.limit.unwrap_or(500))?;
            if cli.print {
                // Will be replaced by TUI in Task 5
                if let Some(first) = entries.first() {
                    println!("{}", first.path);
                }
            } else {
                eprintln!("TUI not yet implemented. Use 'trod list' for now.");
                print_list(&entries);
            }
        }
    }
    Ok(())
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(rest) = path.strip_prefix(home.to_str().unwrap_or("")) {
            return format!("~{}", rest);
        }
    }
    path.to_string()
}

fn relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}

fn print_list(entries: &[db::DirEntry]) {
    for entry in entries {
        println!(
            "{:<50} {:>10} {:>4}",
            shorten_path(&entry.path),
            relative_time(entry.last_visited),
            entry.visit_count
        );
    }
}

fn print_json(entries: &[db::DirEntry]) -> Result<()> {
    print!("[");
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!(
            "\n  {{\"path\":\"{}\",\"visit_count\":{},\"last_visited\":\"{}\"}}",
            entry.path.replace('\\', "\\\\").replace('"', "\\\""),
            entry.visit_count,
            entry.last_visited.to_rfc3339()
        );
    }
    println!("\n]");
    Ok(())
}

fn print_init(shell: &Shell) {
    match shell {
        Shell::Bash => print!(
            r#"# trod shell integration for bash
__trod_hook() {{ command trod add "$PWD"; }}
PROMPT_COMMAND="__trod_hook;${{PROMPT_COMMAND}}"

alias td=trod

__trod_pick() {{
  local dir
  dir=$(command trod --print)
  if [[ -n "$dir" ]]; then
    cd "$dir" || return
  fi
}}
bind -x '"\C-g": __trod_pick'
"#
        ),
        Shell::Zsh => print!(
            r#"# trod shell integration for zsh
autoload -U add-zsh-hook
__trod_hook() {{ command trod add "$PWD" }}
add-zsh-hook chpwd __trod_hook

alias td=trod

trod-pick() {{
  local dir
  dir=$(command trod --print)
  if [[ -n "$dir" ]]; then
    cd "$dir"
  fi
  zle reset-prompt
}}
zle -N trod-pick
bindkey '^G' trod-pick
"#
        ),
    }
}
```

**Step 3: Verify it compiles and help works**

Run: `cargo build`
Then: `cargo run -- --help`
Expected: prints help with all subcommands listed

**Step 4: Smoke test the CLI**

Run: `cargo run -- add /tmp && cargo run -- list && cargo run -- stats`
Expected: `/tmp` appears in list, stats show 1 directory

**Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: implement CLI argument parsing and command dispatch"
```

---

### Task 4: Shell Integration Tests

**Files:**
- Create: `tests/cli_tests.rs`

**Step 1: Write integration tests for CLI commands**

Create `tests/cli_tests.rs`:

```rust
use std::process::Command;
use tempfile::NamedTempFile;

fn trod(db: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trod"));
    cmd.arg("--db").arg(db);
    cmd
}

#[test]
fn test_add_and_list() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    let output = trod(db).args(["add", "/tmp"]).output().unwrap();
    assert!(output.status.success());

    let output = trod(db).arg("list").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("/tmp"));
}

#[test]
fn test_back() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/first"]).output().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    trod(db).args(["add", "/second"]).output().unwrap();

    let output = trod(db).args(["back", "1"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().contains("/first"));
}

#[test]
fn test_forget() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/tmp"]).output().unwrap();
    trod(db).args(["forget", "/tmp"]).output().unwrap();

    let output = trod(db).arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("/tmp"));
}

#[test]
fn test_list_json() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    trod(db).args(["add", "/tmp"]).output().unwrap();

    let output = trod(db).args(["list", "--json"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"path\":\"/tmp\""));
}

#[test]
fn test_init_bash() {
    let output = Command::new(env!("CARGO_BIN_EXE_trod"))
        .args(["init", "bash"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PROMPT_COMMAND"));
    assert!(stdout.contains("alias td=trod"));
}

#[test]
fn test_init_zsh() {
    let output = Command::new(env!("CARGO_BIN_EXE_trod"))
        .args(["init", "zsh"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("chpwd"));
    assert!(stdout.contains("alias td=trod"));
}
```

**Step 2: Run integration tests**

Run: `cargo test --test cli_tests`
Expected: all 6 tests PASS

**Step 3: Commit**

```bash
git add tests/cli_tests.rs
git commit -m "test: add CLI integration tests"
```

---

### Task 5: Interactive TUI Picker

**Files:**
- Create: `src/tui.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

**Step 1: Create TUI module**

Create `src/tui.rs`:

```rust
use crate::db::DirEntry;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io::stdout;

pub struct TuiPicker {
    entries: Vec<DirEntry>,
    filtered: Vec<usize>,
    query: String,
    list_state: ListState,
    matcher: SkimMatcherV2,
}

impl TuiPicker {
    pub fn new(entries: Vec<DirEntry>, initial_query: Option<String>) -> Self {
        let query = initial_query.unwrap_or_default();
        let mut picker = Self {
            entries,
            filtered: Vec::new(),
            query,
            list_state: ListState::default(),
            matcher: SkimMatcherV2::default(),
        };
        picker.update_filter();
        if !picker.filtered.is_empty() {
            picker.list_state.select(Some(0));
        }
        picker
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let mut scored: Vec<(usize, i64)> = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    self.matcher
                        .fuzzy_match(&entry.path, &self.query)
                        .map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        // Reset selection
        if self.filtered.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, self.filtered.len() as i32 - 1) as usize;
        self.list_state.select(Some(next));
    }

    fn selected_path(&self) -> Option<String> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .map(|&idx| self.entries[idx].path.clone())
    }

    pub fn run(mut self) -> Result<Option<String>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        let result = loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => break None,
                    KeyCode::Enter => break self.selected_path(),
                    KeyCode::Up | KeyCode::BackTab => self.move_selection(-1),
                    KeyCode::Down | KeyCode::Tab => self.move_selection(1),
                    KeyCode::Char('k') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.move_selection(-1)
                    }
                    KeyCode::Char('j') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.move_selection(1)
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.update_filter();
                    }
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.update_filter();
                    }
                    _ => {}
                }
            }
        };

        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(result)
    }

    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // search bar
                Constraint::Min(1),   // list
                Constraint::Length(1), // help bar
            ])
            .split(f.area());

        // Search bar
        let search = Paragraph::new(format!("> {}", self.query))
            .block(Block::default().borders(Borders::ALL).title(" trod "));
        f.render_widget(search, chunks[0]);

        // Directory list
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&idx| {
                let entry = &self.entries[idx];
                let path = shorten_path(&entry.path);
                let time = relative_time(entry.last_visited);
                let count = entry.visit_count;
                let width = chunks[1].width as usize;
                let right = format!("{:>10} {:>4}", time, count);
                let left_width = width.saturating_sub(right.len() + 2);
                let line = format!("  {:<left_width$}{}", path, right);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Help bar
        let help = Paragraph::new(" ↑↓ navigate  ⏎ select  esc quit")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[2]);
    }
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(rest) = path.strip_prefix(home.to_str().unwrap_or("")) {
            return format!("~{}", rest);
        }
    }
    path.to_string()
}

fn relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}
```

**Step 2: Add tui module to lib.rs**

Update `src/lib.rs`:
```rust
pub mod cli;
pub mod db;
pub mod tui;
```

**Step 3: Wire TUI into main.rs**

In `src/main.rs`, add `mod tui;` at the top, then replace the `None` arm in the match:

```rust
        None => {
            let db = open_db(&cli)?;
            let entries = db.list_recent(cli.limit.unwrap_or(500))?;
            if entries.is_empty() {
                eprintln!("No directory history yet. cd around and trod will remember.");
                return Ok(());
            }
            let picker = tui::TuiPicker::new(entries, cli.query.clone());
            match picker.run()? {
                Some(path) => {
                    if cli.print {
                        println!("{}", path);
                    } else {
                        println!("{}", path);
                    }
                }
                None => {}
            }
        }
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: compiles successfully

**Step 5: Manual smoke test**

Run: `cargo run -- add /tmp && cargo run -- add /var && cargo run -- add /etc`
Then: `cargo run`
Expected: TUI launches showing 3 directories. Typing filters. Enter selects. Esc quits.

**Step 6: Commit**

```bash
git add src/tui.rs src/lib.rs src/main.rs
git commit -m "feat: implement interactive TUI directory picker"
```

---

### Task 6: Polish & README

**Files:**
- Create: `README.md`

**Step 1: Write README**

Create `README.md`:

````markdown
# trod

> Persistent directory history with an interactive picker. Stop thinking about directory navigation — just `cd` around. Trod remembers everything.

**trod** tracks every directory you visit and gives you a fuzzy-searchable TUI to jump back. It's the spiritual successor to [dirhistory](https://github.com/madsen/dirhistory), reimagined in Rust.

## Install

```
cargo install trod
```

Or with Homebrew (coming soon):

```
brew install trod
```

## Setup

Add to your `~/.zshrc`:

```zsh
eval "$(trod init zsh)"
```

Or `~/.bashrc`:

```bash
eval "$(trod init bash)"
```

This gives you:
- Automatic directory tracking on every `cd`
- `td` alias for `trod`
- **Ctrl-G** keybinding to launch the picker

## Usage

```
td              # Launch interactive picker
td back         # Go to previous directory
td back 3       # Go back 3 directories
td list         # Print history to stdout
td stats        # Show usage statistics
td forget /path # Remove a path from history
td clean        # Prune deleted directories
```

## How it compares to zoxide

| | zoxide | trod |
|---|---|---|
| **Model** | Frecency jump | Visual history browser |
| **Usage** | `z foo` → jumps to best match | `td` → browse + pick |
| **UI** | No TUI | Full TUI with fuzzy search |

**zoxide** = "I know where I want to go, get me there fast"
**trod** = "Show me where I've been, let me pick"

They work great together.

## License

MIT
````

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README"
```

---

### Task 7: Final Verification

**Step 1: Run all tests**

Run: `cargo test`
Expected: all tests pass

**Step 2: Build release binary**

Run: `cargo build --release`
Expected: binary at `target/release/trod`

**Step 3: End-to-end smoke test with release binary**

```bash
./target/release/trod add /tmp
./target/release/trod add /var/log
./target/release/trod add /etc
./target/release/trod list
./target/release/trod stats
./target/release/trod back 1
./target/release/trod init zsh
./target/release/trod clean
./target/release/trod forget /tmp
./target/release/trod list
```

Expected: all commands work correctly

**Step 4: Commit any remaining changes**

```bash
git add -A
git commit -m "chore: final polish for v0.1.0"
```
