# trod — Design Document

A modern, persistent directory history tool. Reimagines [madsen/dirhistory](https://github.com/madsen/dirhistory) in Rust with a richer feature set.

**Tagline**: Stop thinking about directory navigation. Just `cd` around. Trod remembers everything.

## Name

**trod** — "paths you've trodden." Short, unique, available on GitHub/Homebrew/crates.io. Shell alias `td` for daily use.

## Language & Stack

- **Rust** — single binary, no runtime deps, Homebrew-friendly
- **ratatui** + **crossterm** — TUI rendering
- **clap** — CLI argument parsing
- **rusqlite** — SQLite via bundled libsqlite3
- **fuzzy-matcher** — fuzzy search in TUI

## Architecture

Single binary, no daemon. SQLite with WAL mode handles concurrent writes.

```
Shell hook (chpwd / PROMPT_COMMAND)
        │
        ▼
   trod add $PWD ──► SQLite (~/.local/share/trod/history.db)
        │
        ▼
   trod (TUI) ◄──── reads from same SQLite
        │
        ▼
   stdout (--print) ──► shell wrapper calls cd
```

### Data Storage

Location: `~/.local/share/trod/history.db` (XDG conventions)

Schema:

```sql
CREATE TABLE directories (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,
    visit_count INTEGER DEFAULT 1,
    last_visited TIMESTAMP NOT NULL,
    first_visited TIMESTAMP NOT NULL
);

CREATE TABLE visits (
    id INTEGER PRIMARY KEY,
    directory_id INTEGER NOT NULL REFERENCES directories(id),
    timestamp TIMESTAMP NOT NULL,
    session_id TEXT
);

CREATE INDEX idx_directories_last_visited ON directories(last_visited);
CREATE INDEX idx_directories_path ON directories(path);
CREATE INDEX idx_visits_timestamp ON visits(timestamp);
```

Two tables: `directories` for aggregate stats (fast lookups), `visits` for detailed timeline.

## CLI Surface

Binary installs as `trod`. Shell integration aliases it to `td`.

```
td                      # Launch interactive TUI picker
td add <path>           # Record a directory visit (called by shell hook)
td back [n]             # Go back n directories in history (default: 1)
td init <shell>         # Output shell integration code (bash|zsh)
td list                 # Print history to stdout (for piping)
td forget <path>        # Remove a directory from history
td clean                # Remove entries for directories that no longer exist
td stats                # Show usage stats (most visited, total tracked, etc.)
td import               # Import from zoxide/autojump/dirhistory databases
```

### Flags

**`td` (TUI mode):**
- `--query <string>` — pre-fill search filter
- `--limit <n>` — max entries to show
- `--print` — output selected path to stdout (used by shell keybinding)

**`td list`:**
- `--sort recent|frequent|frecent` — sort order (default: recent)
- `--limit <n>` — number of entries
- `--json` — JSON output for scripting

**Global:**
- `--db <path>` — custom database location
- `--help` / `--version`

## Shell Integration

`trod init zsh` outputs:

```zsh
chpwd() { command trod add "$PWD" }

alias td=trod

trod-pick() {
  local dir=$(command trod --print)
  if [[ -n "$dir" ]]; then
    cd "$dir"
  fi
}
zle -N trod-pick
bindkey '^G' trod-pick
```

`trod init bash` outputs:

```bash
__trod_hook() { command trod add "$PWD"; }
PROMPT_COMMAND="__trod_hook;${PROMPT_COMMAND}"

alias td=trod

__trod_pick() {
  local dir=$(command trod --print)
  if [[ -n "$dir" ]]; then
    cd "$dir"
  fi
}
bind -x '"\C-g": __trod_pick'
```

## TUI Design

```
┌─ trod ──────────────────────────────────────────┐
│ > search...                                     │
├─────────────────────────────────────────────────┤
│   ~/projects/trod                    2m ago  12 │
│   ~/projects/api-server             15m ago  47 │
│   /etc/nginx                         1h ago   3 │
│   ~/documents/taxes                  2d ago   5 │
│   ~/projects/old-thing               5d ago   2 │
│                                                 │
├─────────────────────────────────────────────────┤
│ ↑↓ navigate  ⏎ select  / filter  esc quit      │
└─────────────────────────────────────────────────┘
```

- Paths shortened with `~`
- Right columns: relative time + visit count
- Fuzzy search filter at top
- Default sort: recent first
- Keybindings: arrows / j/k navigate, Enter select, Esc cancel, d delete entry

## Differentiation from zoxide

| | zoxide | trod |
|---|---|---|
| **Model** | Frecency jump | Visual history browser |
| **Usage** | `z foo` → jumps to best match | `td` → browse + pick |
| **UI** | No TUI | Full TUI with search |
| **Tracking** | Automatic | Automatic |
| **Complements** | Works great alongside trod | Works great alongside zoxide |

**zoxide** = "I know where I want to go, get me there fast"
**trod** = "Show me where I've been, let me pick"

## Deliberate Omissions (v1)

- **No config file** — zero-config. Add `~/.config/trod/config.toml` later if needed.
- **No sync/cloud** — Atuin's territory, massive complexity.
- **No Fish/Nushell/PowerShell** — Bash + Zsh covers ~95% of users. Add later.
- **No frecency sorting** — start with recent + frequent, add frecency algorithm later if needed.

## Distribution

- **Homebrew**: `brew install trod`
- **Cargo**: `cargo install trod`
- **GitHub Releases**: prebuilt binaries for macOS (arm64, x86_64) + Linux (x86_64, aarch64)
