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
