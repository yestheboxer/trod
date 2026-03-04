# trod

> Persistent directory history with an interactive picker. Stop thinking about directory navigation — just `cd` around. Trod remembers everything.

**trod** tracks every directory you visit and gives you a fuzzy-searchable TUI to jump back. Written in Rust.

## Install

### Homebrew

```
brew install yestheboxer/trod/trod
```

### Cargo

```
cargo install trod
```

## Shell Setup

Add to your `~/.zshrc`:

```zsh
eval "$(trod init zsh)"
```

Or `~/.bashrc`:

```bash
eval "$(trod init bash)"
```

This gives you:
- **Automatic tracking** — every `cd` is recorded, no extra steps
- **`td` alias** — shorthand for `trod`
- **Ctrl-G** — keybinding to launch the interactive picker from anywhere

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

### Shell Integration Commands

```
trod init zsh   # Output zsh integration code
trod init bash  # Output bash integration code
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
