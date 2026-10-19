# RChess TUI — Agent Guide

**Single Rust binary crate** (`ratatui` + `crossterm`), no workspace. Target zero warnings.

## Build & run

```
cargo build --release
cargo clippy --all-targets   # must stay warning-free
cargo test --release         # movegen perft, config, lichess parsing
./target/release/rchess
```

`cargo fmt` is not enforced: the tree is intentionally unformatted.

## Debug logging

```
rlog!("any message");          // in code
tail -f /tmp/rchess.log         // watch output
```

All internal log output goes to `/tmp/rchess.log` — stdin/stdout owned by terminal UI.

## Key architecture

| Module | Role |
|--------|------|
| `main.rs` | Terminal setup, 50 ms event loop, CLI dispatch |
| `app.rs` (~2100 L) | All UI state, key/mouse handlers, AI/analysis/lichess polling, undo |
| `engine.rs` | Board types, legal movegen with mailbox, FEN/PGN |
| `ai.rs` | Minimax thread (depth 1-4) + Stockfish opponent thread |
| `analysis.rs` | Background move analysis via built-in or Stockfish UCI |
| `config.rs` | Config struct + all enums (Theme, PieceStyle, UiMode, etc.) |
| `ui.rs` (~2100 L) | Ratatui rendering — board, menu, settings, all screens |
| `pieces.rs` | 5×3 ASCII / Unicode / block-art piece patterns |
| `book.rs` | Opening book (~15 moves) |
| `puzzle.rs` | Lichess daily puzzle via `ureq` HTTP client |
| `save.rs` | Auto-save/load games as JSON to `~/.config/rchess/saves/` |
| `png_export.rs` | Python/Pillow subprocess for PNG export |
| `pgn_import.rs` | PGN file parser |
| `tt.rs` | Transposition table (used by built-in search) |
| `log.rs` | Defines the `rlog!()` macro |
| `lichess.rs` | Lichess API client (ureq + native-tls) — event/game streaming, challenges, moves |
| `net/` | Thin wrapper re-exporting lichess types (no more TCP server) |
| `rcengine/` | Bitboard engine (movegen, search, eval, UCI) — standalone `--engine` flag |

## CLI flags

| Flag | Action |
|------|--------|
| *(none)* | Start the TUI game |
| `--version` | Print version and exit |
| `--help` | Print help and exit |
| `--engine` | Run rcengine as UCI engine (stdin/stdout) |

### rcengine submodules

`rcengine/` is a self-contained bitboard engine with 6 modules:
`types`, `board`, `movegen`, `search`, `uci`, `zobrist`.
Call `rcengine::init()` once before use (called automatically in both `--engine` and normal modes).
Magic bitboard tables initialized in `movegen::init()`.

#### Engine features (search.rs)

| Feature | Detail |
|---------|--------|
| **Iterative deepening** | Depth 1–64, aspiration windows (±25 cp) |
| **TT** | 1M-entry transposition table (replace-if-deeper) |
| **Null-move pruning** | Depth ≥3, reduction R=2 (R=3 at depth ≥6) |
| **LMR** | Late Move Reduction — reduce depth by 1 after move 4 at depth ≥3 |
| **Check extensions** | Extend search 1 ply when side to move in check |
| **Killer moves** | 2 killers per ply, stored on beta cutoff for quiet moves |
| **History heuristic** | `[side][from][to]` table, updated on beta cutoffs with depth² |
| **Move ordering** | TT move → killers → MVV-LVA + history |

#### Evaluation features (search.rs)

| Feature | Value |
|---------|-------|
| **MG/EG PST tapering** | Separate MG and EG piece-square tables, interpolated by game phase (0–256) |
| **Bishop pair bonus** | +30 cp |
| **Rook on open file** | +20 cp (open), +10 cp (semi-open) |
| **Passed pawn** | +30 cp |
| **Isolated pawn** | −15 cp |
| **Doubled pawn** | −15 cp per extra pawn |
| **Mobility** | Per-piece-type attack count (knights via table, bishops/rooks/queens via magic) |

#### Instant startup (movegen.rs)

Pre-defined magic arrays `BISHOP_MAGICS_FIXED`/`ROOK_MAGICS_FIXED` replace brute-force `find_magic()` (was 30–90s startup → <1s).

#### Key guards (board.rs)

- `make_move()` rejects king captures (returns false) — prevents panic in subsequent `generate_moves()`
- `king_sq()` returns 64 when king bitboard is 0; `is_attacked()` returns false for `sq >= 64`
- `generate_moves()` guards against missing king at `movegen.rs:486`

## Config

- Path: `~/.config/rchess/rchess_tui.conf`
- Save from Settings with `w`, or programmatically via `cfg.save()`
- `Config::load()` — reads file, falls back to `Config::default()`
- `Setting::ALL` in `config.rs` is the single source of truth for the settings
  screen and the key handler; add a variant there when adding a setting, or the
  two lists will drift out of sync.
- The Lichess token can be entered in Settings (row 19, masked). It is written
  with explicit `(key, value)` pairs — never positional `format!` arguments.

## Online play (Lichess)

- Token needs the `board:play` scope; enter it in Settings (row 19) or the
  config file. The token is never echoed in full by the UI.
- A seek is `POST /api/board/seek` and its **response stream must stay open**:
  closing it cancels the seek. `SeekHandle` in `lichess.rs` owns the live
  response; dropping it cancels the seek.
- The server move list is authoritative. `sync_online_state()` diffs it against
  `online_moves` and replays new moves with `push_server_move()`; our own move
  is never echoed back to the API.
- Only standard / from-position games are supported. Other variants are refused
  in the lobby and aborted if they arrive through the event stream.
- Draw and takeback endpoints take `yes`/`no`, not `true`/`false`.
- Leaving the lobby (Esc) drops the event stream so a challenge cannot yank the
  user into a game from the main menu.

## Analysis engine

Two backends (config/settings toggle):
- **Builtin** (default): pure-Rust minimax, depth 1-3
- **Stockfish**: UCI via stdin/stdout, ~300 ms per position. Falls back to builtin if Stockfish not found.
- Watchdog in `app.rs:440` resets if analysis engine busy >5 s (handles silent thread death).

## Phase State

All phases implemented and verified (14/14 items passing).

### Phase D — Online Robustness (complete)
- **Exponential backoff**: Event/game stream reconnect starts at 1s, doubles, caps at 60s (was fixed 5s/3s)
- **Shutdown flags**: `Arc<AtomicBool>` returned from spawn methods; `stop_game_stream()` signals clean exit
- **Key bindings**: `t` takeback, `v` claim victory, `n` blocked (shows guidance message)
- **Draw offer fix**: `handle_draw_offer_key` fallthrough — unrecognized keys decline draw and pass to `handle_online_game_key` (prevents key swallowing)
- **Stream cleanup**: `stop_game_stream()` called on `Esc`/`q` from online game

### Phase E — UI Features (complete)
- **Engine-hint overlay**: `show_hints` toggled by `e` key; best move UCI parsed and added as board arrow
- **Move arrows**: `arrows: Vec<(from, to)>` pushed on `execute()`, popped on `undo()`, cleared on `start_game()`; rendered with `arrow_color` (accent blend) on from/to squares
- **Game result dialog**: Enhanced with `F:FEN` key hint; all game keys work as overlay
- **Auto-promote**: `Config::auto_promote` (settings row 10); skips promo dialog in `do_select()`; notation path defaults to queen

## Conventions

- `#![allow(dead_code)]` + `#![allow(unused)]` at crate level (`main.rs:2-3`) overriding `[lints.rust]` in Cargo.toml.
- Undo: CPU mode pops 2 (player + AI), PvP pops 1.
- FEN clipboard: `copy_to_clipboard()` at `app.rs:2012` — tries `xclip`, falls back to `wl-copy`.
- Exports to `~/rchess_export/` (PNG and PGN).
- Lichess API uses `ureq` + `native-tls` (system OpenSSL — avoids ring/rustls linking issues in makepkg).
- `lichess_token` in config required for online play (same token as puzzles).
- Online play replaced the old TCP server; `--server` flag is removed.
- Settings screens save `prev_screen` before entry; `Esc` / `q` restores it.
- **Seek API restriction**: `/api/board/seek` requires `limit*60 + increment*40 > 300` for non-bot accounts. Invalid controls (1+0, 3+0, 5+0, 3+2, 5+3) return 400. Valid controls from app: 10+0, 5+5, 15+10, 30+0.

## AUR

Maintained at `https://aur.archlinux.org/packages/rchess`. `PKGBUILD` + `.SRCINFO` in repo root.
