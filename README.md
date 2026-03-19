<div align="center">

# ♟ rchess

**A terminal chess game built in pure Rust**

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)
![ratatui](https://img.shields.io/badge/ratatui-0.29-blueviolet)
![crossterm](https://img.shields.io/badge/crossterm-0.28-blue)
![License](https://img.shields.io/badge/license-MIT-green)

*Full-featured chess in your terminal — no GUI, no Electron, just Rust.*

</div>

---

## Example of Exported PNG

![Preview](assets/rchess_board1.png)

![Preview](assets/rchess_board2.png)

---

## Features

| Category | What's included |
|----------|----------------|
| **Game modes** | Two-player (PvP) · vs CPU |
| **AI engine** | Minimax + alpha-beta pruning + piece-square tables, depth 1–4 |
| **Opening book** | 31 lines — Ruy López, Sicilian, French, Caro-Kann, Queen's Gambit, King's Indian, London, Réti, and more |
| **Opening detection** | ECO-style name shown in topbar (`[Sicilian — Najdorf]`) |
| **Move analysis** | Per-move evaluation with Book / Good / Inaccuracy / Mistake / Blunder labels |
| **Multi-PV** | Top 3 candidate moves with scores in Analysis panel |
| **Eval sparkline** | ASCII bar chart of evaluation across all moves in Replay view |
| **Stockfish** | Optional drop-in via UCI — enable in Settings |
| **PNG export** | Board rendered as image using Python Pillow; auto-detects Unicode chess fonts via `fc-list`, falls back to geometric pieces |
| **PGN export** | Auto-saved on game end to `~/rchess_export/` |
| **Replay** | Step through any completed game; see eval + label per move |
| **6 themes** | Classic · Tournament · Mocha · Slate · Midnight · Crimson |
| **3 piece styles** | Unicode ♔♕♖ · ASCII K Q R · Bracketed [K][Q][R] |
| **3 UI modes** | Minimal · Standard · Analysis (toggle with `T`) |
| **Time controls** | Infinite · Bullet · Blitz · Rapid · Classical |
| **Mouse support** | Click to select and move pieces |
| **Notation input** | Type `e2e4`, `Nf3`, `O-O`, `O-O-O` during play |
| **Undo** | Step back one ply (or two in CPU mode) |
| **Draw offers** | PvP only |
| **Pawn promotion** | Interactive picker |
| **Board flip** | Manual or auto-flip for PvP |
| **Debug log** | All internal output goes to `/tmp/rchess.log` — terminal stays clean |

---

## Installation

### Arch Linux (recommended)

```bash
# 1. Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && source ~/.cargo/env

# 2. Build
git clone https://github.com/yourname/rchess && cd rchess
cargo build --release

# 3. Run
./target/release/rchess

# 4. Optional: PNG export
sudo pacman -S python-pillow

# 5. Optional: Stockfish (much stronger move analysis)
sudo pacman -S stockfish

# 6. Optional: Unicode chess symbols in PNG
#    Your Nerd Font likely works already.
#    If not: sudo pacman -S ttf-freefont
```

### Ubuntu / Debian

```bash
sudo apt install python3-pil stockfish
cargo build --release
```

### macOS

```bash
brew install stockfish
pip3 install pillow
cargo build --release
```

---

## Controls

### In-Game

| Key | Action |
|-----|--------|
| `↑↓←→` / `hjkl` | Move cursor |
| `Enter` / `Space` | Select piece / confirm move |
| `Esc` | Deselect / cancel |
| Mouse click | Select and move pieces |
| Any letter | Start typing notation (`e2e4`, `Nf3`, `O-O`) |
| `Backspace` | Delete typed character |
| `u` | Undo last move |
| `d` | Offer draw *(PvP only)* |
| `r` | Open replay |
| `E` (Shift+E) | Export board as PNG (preview first) |
| `T` (Shift+T) | Cycle UI mode: Minimal → Standard → Analysis |
| `n` | New game |
| `s` | Settings |
| `q` | Back to menu |

### Notation Input

Type while in-game — a cursor appears automatically:

```
e2e4        coordinate  (from-square + to-square)
Nf3         SAN piece notation
Qxd5        SAN with capture
O-O         kingside castle
O-O-O       queenside castle
e7e8q       pawn promotion to queen
```

### Replay

| Key | Action |
|-----|--------|
| `←` / `h` | Previous move |
| `→` / `l` | Next move |
| `0` / `Home` | Jump to start |
| `$` / `End` | Jump to final position |
| `E` | Export current position as PNG |
| `q` / `Esc` | Back to game |

### Settings Screen

| Key | Action |
|-----|--------|
| `↑↓` / `jk` | Navigate options |
| `←→` / `hl` | Change value |
| `w` | Save to config file |
| `r` | Reset to defaults |
| `Esc` / `q` | Back to menu |

---

## Settings Reference

| Setting | Options | Default |
|---------|---------|---------|
| Theme | Classic · Tournament · Mocha · Slate · Midnight · Crimson | Classic |
| Piece style | Unicode ♔♕♖ · ASCII K Q R · Bracketed [K][Q][R] | Unicode |
| AI difficulty | Easy (d1) · Medium (d2) · Hard (d3) · Expert (d4) | Hard |
| Move hints | Dots · Highlight · Off | Dots |
| Time control | Infinite · Bullet · Blitz · Rapid · Classical | Infinite |
| Show coords | ON / OFF | ON |
| Show clock | ON / OFF | ON |
| Flip board | ON / OFF | OFF |
| Auto-flip PvP | ON / OFF | OFF |
| Confirm move | ON / OFF | OFF |
| UI mode | Minimal · Standard · Analysis | Standard |
| Analysis engine | Built-in · Stockfish | Built-in |
| Analysis depth | 1 (fast) · 2 (balanced) · 3 (strong) | 2 |

---

## Config File

**Location:** `~/.config/rchess/rchess_tui.conf`

Created automatically when you press `w` in Settings. You can also edit it directly — the game picks up changes on next launch:

```ini
# ~/.config/rchess/rchess_tui.conf

theme            = classic        # classic tournament mocha slate midnight crimson
piece_style      = unicode        # unicode letters fatletters
ai_depth         = 3              # 1=Easy  2=Medium  3=Hard  4=Expert
move_hints       = dots           # dots highlight none
time_control     = infinite       # infinite bullet blitz rapid classical
show_coords      = true
show_clock       = true
flip_board       = false
auto_flip        = false
confirm_move     = false
ui_mode          = standard       # minimal standard analysis
analysis_engine  = builtin        # builtin stockfish
analysis_depth   = 2              # 1=fast  2=balanced  3=strong (built-in only)
```

---

## Move Analysis

After each move, the background analysis engine evaluates the position before and after. The difference (in pawns) determines the classification:

| Label | Delta | Icon | Meaning |
|-------|-------|------|---------|
| Book | — | `B` | Move is in the opening book |
| Good | > −0.50p | `+` | Strong or neutral move |
| Inaccuracy | −0.50 to −1.00p | `?!` | Small mistake |
| Mistake | −1.00 to −3.00p | `?` | Significant error |
| Blunder | < −3.00p | `??` | Serious blunder |

Labels appear **live** in the history panel as analysis completes, without freezing the UI. In **Analysis mode** (`T` to toggle), you also see:
- Top 3 candidate moves with scores
- Exact eval before → after
- Evaluation sparkline in Replay

### Built-in Engine

Pure Rust minimax, always available, no install needed.

| Depth | Speed | Quality |
|-------|-------|---------|
| 1 | ~1ms | Catches obvious blunders |
| 2 | ~5–50ms | Good tactical awareness *(default)* |
| 3 | ~50–500ms | Stronger, misses fewer tactics |

### Stockfish

```bash
# Install on Arch
sudo pacman -S stockfish

# Enable in Settings → Analysis engine → Stockfish
# or directly in config:
analysis_engine = stockfish
```

Uses `MultiPV 3` to show top 3 moves. If Stockfish is selected but not found, falls back to the built-in engine automatically.

---

## PNG Export

Press `Shift+E` → preview → `Y` to save.

Files saved to: `~/rchess_export/rchess_board_<timestamp>.png`

### Font Detection for Chess Symbols

The renderer uses `fc-list :charset=2654` to find fonts with chess glyphs (♔♕♖♗♘♙). Detection priority:

1. **`fc-list` confirmed** — fontconfig finds fonts with chess Unicode
2. **Your Nerd Fonts** — scanned from `~/.local/share/fonts/`
3. **Known paths** — FreeSerif, Noto, DejaVu
4. **Fallback** — geometric circles with letter labels (K Q R B N P)

If you have a Nerd Font and it's based on a Unicode-capable base font, it will be used automatically. Otherwise:

```bash
sudo pacman -S ttf-freefont    # guaranteed chess glyph support on Arch
```

---

## Exported Files

| File | Location | Trigger |
|------|----------|---------|
| PNG board image | `~/rchess_export/rchess_board_<timestamp>.png` | `Shift+E` in-game or replay |
| PGN game record | `~/rchess_export/rchess_<movenum>.pgn` | Automatic on game end |

---

## Opening Book & Detection

The CPU uses a built-in opening book for the first ~15 moves. When a game is in-book, the current opening name is shown in the topbar (e.g. `[Sicilian — Najdorf]`).

**Covered openings:**

<details>
<summary>1.e4 openings</summary>

- Ruy López (Closed, Exchange, Berlin)
- Italian Game (Giuoco Piano, Two Knights)
- Scotch Game
- Sicilian Defence (Najdorf, Dragon, Kan, Closed, Open)
- French Defence (Classical, Advance, Tarrasch)
- Caro-Kann (Classical, Advance)
- Pirc / Modern Defence

</details>

<details>
<summary>1.d4 openings</summary>

- Queen's Gambit (Accepted, Declined, Slav)
- King's Indian Defence (Classical, Sämisch)
- Nimzo-Indian (Rubinstein, Classical)
- Queen's Indian Defence
- Grünfeld Defence (Exchange)
- London System

</details>

<details>
<summary>Flank openings</summary>

- English Opening (Reversed Sicilian, Symmetrical)
- Réti Opening
- Bird's Opening

</details>

---

## Terminal Requirements

| Mode | Min width | Min height |
|------|-----------|------------|
| Minimal | 100 cols | 30 rows |
| Standard | 115 cols | 32 rows |
| Analysis | 120 cols | 36 rows |

> Board cells are 9 chars × 5 rows — visually square at standard terminal font ratios.

---

## Project Structure

```
rchess/
├── Cargo.toml
└── src/
    ├── main.rs        — entry point, event loop (50ms tick)
    ├── log.rs         — silent file logger → /tmp/rchess.log
    ├── engine.rs      — chess rules: legal moves, apply, check detection
    ├── ai.rs          — minimax + alpha-beta + PSTs + top_n_moves()
    ├── book.rs        — opening book (31 lines) + ECO name detection
    ├── analysis.rs    — background analysis: built-in + Stockfish UCI, MultiPV
    ├── app.rs         — application state, key handlers, game logic
    ├── ui.rs          — ratatui rendering (board, sidebar, overlays, sparkline)
    ├── config.rs      — all settings, themes, enums
    └── png_export.rs  — board → PNG via Python Pillow subprocess
```

---

## Building from Source

```bash
cargo build --release        # optimised (use this)
cargo build                  # debug build (AI is slower)

./target/release/rchess      # run
./target/release/rchess -v   # version
./target/release/rchess -h   # help

# Watch debug output (separate terminal)
tail -f /tmp/rchess.log
```

**Dependencies** — fetched automatically by Cargo:

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.29 | Terminal UI rendering |
| `crossterm` | 0.28 | Cross-platform terminal control |

No other Rust dependencies. Python + Pillow only needed for PNG export.
