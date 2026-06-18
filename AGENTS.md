# RChess TUI — Agent Guide

**Single Rust binary crate** (`ratatui` + `crossterm`), no workspace, no tests.

## Build & run

```sh
cargo build --release
./target/release/rchess
```

No test, lint, or typecheck commands exist.

## Key architecture

| Module | Role |
|--------|------|
| `main.rs` | Terminal setup, event loop (50 ms tick) |
| `app.rs` | All UI state, key/mouse handlers, AI polling, undo |
| `engine.rs` | Board types, legal move generation, FEN/PGN, `parse_notation` |
| `ai.rs` | Minimax engine (depth 1-4) + Stockfish opponent |
| `analysis.rs` | Background move analysis (built-in or Stockfish via UCI) |
| `config.rs` | Config + all enums (themes, piece styles, UI modes) |
| `ui.rs` | Ratatui rendering |
| `pieces.rs` | Block/half-block piece bitmap renderer |
| `book.rs` | Opening book (~15 moves) |
| `puzzle.rs` | Lichess daily puzzle fetcher |
| `png_export.rs` | Calls Python/Pillow subprocess for PNG export |
| `pgn_import.rs` | PGN file parser |
| `log.rs` | `rlog!()` macro — writes to `/tmp/rchess.log` |

## Debug logging

All internal output goes to `/tmp/rchess.log`. The terminal is never polluted.

```sh
tail -f /tmp/rchess.log
```

Use the `rlog!()` macro anywhere in the codebase.

## Config

- Path: `~/.config/rchess/rchess_tui.conf`
- Saved via `w` in Settings (or programmatically via `cfg.save()`)
- `Config::load()` — reads file, falls back to `Config::default()`

## Analysis engine

Two backends, set in config or Settings:
- **Builtin** (default): pure-Rust minimax, depth 1-3. Always works.
- **Stockfish**: via UCI stdin/stdout, 300 ms per position. Auto-fallback to builtin if not found.

Analysis engine watchdog: resets if busy > 5 s (handles silent thread death).

## Notable conventions

- `#![allow(dead_code)]` + `#![allow(unused)]` — both set at crate level
- `Cargo.toml` sets `dead_code = "warn"` in `[lints.rust]`
- Undo: CPU mode pops 2 moves (player + AI), PvP mode pops 1
- `T` key cycles UI mode in-game (Minimal → Standard → Analysis → Minimal)
- `flipped()`: board rotation for CPU Black or `flip_board`/`auto_flip` settings
- Stockfish CPU opponent has a 3 s timeout, falls back to minimax d4
- Block art pieces require **130×40 terminal minimum**
- PNG export uses Python + Pillow subprocess (not Rust image crate)
- Lichess puzzle: anonymous works; providing a `lip_` token removes rate limits

## CLI

```
rchess           Start the game
rchess --version Print version
rchess --help    Print help
```

## AUR package

Maintained at `https://aur.archlinux.org/packages/rchess`. PKGBUILD and `.SRCINFO` are in the repo root.
