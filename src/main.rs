mod cli;
mod db;
mod tui;

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
            if let Some(oldest) = &stats.oldest_entry {
                eprintln!("Oldest entry:        {} (since {})", oldest.path, relative_time(oldest.first_visited));
            }
        }
        Some(Command::Import { source: _ }) => {
            eprintln!("Import not yet implemented");
        }
        None if cli.print_back.is_some() => {
            let db = open_db(&cli)?;
            let n = cli.print_back.unwrap();
            match db.back(n)? {
                Some(path) => println!("{}", path),
                None => eprintln!("No directory {} steps back in history", n),
            }
        }
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

trod() {{
  case "$1" in
    add|list|forget|clean|stats|import|init)
      command trod "$@"
      ;;
    back)
      local dir
      dir=$(command trod --print-back "${{2:-1}}")
      if [[ -n "$dir" ]]; then
        cd "$dir" || return
      fi
      ;;
    "")
      local dir
      dir=$(command trod --print)
      if [[ -n "$dir" ]]; then
        cd "$dir" || return
      fi
      ;;
    *)
      command trod "$@"
      ;;
  esac
}}
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

function trod {{
  case "$1" in
    add|list|forget|clean|stats|import|init)
      command trod "$@"
      ;;
    back)
      local dir
      dir=$(command trod --print-back "${{2:-1}}")
      if [[ -n "$dir" ]]; then
        cd "$dir"
      fi
      ;;
    "")
      local dir
      dir=$(command trod --print)
      if [[ -n "$dir" ]]; then
        cd "$dir"
      fi
      ;;
    *)
      command trod "$@"
      ;;
  esac
}}
alias td=trod

function trod-pick {{
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
