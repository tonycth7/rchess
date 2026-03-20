<div align="center">

# ♟ rchess

A fully-featured chess game that runs entirely in your terminal.

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://rustup.rs)
[![AUR](https://img.shields.io/badge/AUR-rchess-blue?logo=archlinux)](https://aur.archlinux.org/packages/rchess)
[![License](https://img.shields.io/badge/license-MIT-green)](#)

</div>

---

## What it does

rchess is a terminal chess game with a built-in AI, real-time move analysis, opening book, Lichess daily puzzles, PNG/PGN export, and full mouse support. No GUI needed.

---

## Installation

### Arch Linux — AUR

```bash
yay -S rchess
# or
paru -S rchess
```

### From source

```bash
git clone https://github.com/yourname/rchess
cd rchess
cargo build --release
./target/release/rchess
```
or just 
```bash
cargo install --git https://github.com/tonycth7/rchess
```
### Optional dependencies

```bash
sudo pacman -S python-pillow   # PNG board export (Shift+E)
sudo pacman -S stockfish       # stronger move analysis
sudo pacman -S ttf-freefont    # Unicode chess symbols in PNG export (only if you don't have a Nerd Font)
```

---

## Controls

| Key | Action |
|-----|--------|
| `↑↓←→` / `hjkl` | Move cursor |
| `Enter` / `Space` / click | Select & move |
| Type a move | `e2e4` · `Nf3` · `O-O` · `O-O-O` |
| `u` | Undo |
| `T` | Cycle UI mode (Minimal / Standard / Analysis) |
| `E` (Shift) | Export board as PNG |
| `G` (Shift) | Save game as PGN |
| `r` | Replay game |
| `d` | Offer draw (PvP) |
| `n` | New game |
| `s` | Settings |
| `q` | Menu |

### Replay controls

| Key | Action |
|-----|--------|
| `←/h` · `→/l` | Step back / forward |
| `0` / `$` | Jump to start / end |
| `E` | Export current position as PNG |

---

## Move analysis

Every move is evaluated in the background and labelled:

| Icon | Label | Threshold |
|------|-------|-----------|
| `B` | Book | Opening book move |
| `+` | Good | < 0.50p loss |
| `?!` | Inaccuracy | 0.50 – 1.00p loss |
| `?` | Mistake | 1.00 – 3.00p loss |
| `??` | Blunder | > 3.00p loss |

Labels appear live in the history panel. In **Analysis mode** (`T`) you also get the top 3 candidate moves, eval before/after, and a vertical eval bar in Replay (shows who's winning at each position, like chess.com).

### Engine options

| Engine | Speed | How to enable |
|--------|-------|---------------|
| Built-in d1 | ~1ms | Settings → Analysis depth |
| Built-in d2 | ~5–50ms | Default |
| Built-in d3 | ~50–500ms | Settings → Analysis depth |
| Stockfish | 300ms/pos | `sudo pacman -S stockfish`, Settings → Analysis engine |

---

## Piece styles

| Style | Example | Notes |
|-------|---------|-------|
| Unicode | ♔ ♕ ♖ ♗ ♘ ♙ | Default — requires a font with chess glyphs |
| ASCII | K Q R B N P | Works on any terminal |
| Bracketed | \[K\] \[Q\] \[R\] | Bold and chunky |
| **Block art** | ██ ██ | Pieces drawn using block characters across the full cell |

> **Block art note** — Block art pieces fill the entire cell with `█` characters to create recognisable shapes (battlements for rooks, cross for kings, etc.). They look best on terminals with a cell size of at least 8×4. **On small terminals or with small cell sizes they may appear broken or unreadable.** Recommended minimum: 130 columns × 40 rows with `cell_w = 8` or larger.

---

## Themes

| Theme | Description |
|-------|-------------|
| Classic | Cream/brown board — traditional look |
| Tournament | Green/gold — tournament style |
| Mocha | Brown/warm tones |
| Slate | Grey/cyan — modern minimal |
| Midnight | Navy/purple — dark |
| Crimson | Red/white — bold |
| Matte Black | Dark grey squares, gold accent, silver & ivory pieces |
| **Ocean** | **Navy/cyan — deep blue board with icy white and navy pieces** |

---

## Config file

**Location:** `~/.config/rchess/rchess_tui.conf`

Press `w` in Settings to save. All options:

```ini
theme                = classic    # classic tournament mocha slate midnight crimson matteblack ocean
piece_style          = unicode    # unicode letters fatletters blocks
ai_depth             = 3          # 1 2 3 4
move_hints           = dots       # dots highlight none
time_control         = infinite   # infinite bullet blitz rapid classical
show_coords          = true
show_clock           = true
flip_board           = false
auto_flip            = false
confirm_move         = false
ui_mode              = standard   # minimal standard analysis
analysis_engine      = builtin    # builtin stockfish
analysis_depth       = 2          # 1=fast  2=balanced  3=strong
blunder_cp           = 300
mistake_cp           = 100
inaccuracy_cp        = 50
stockfish_skill      = 10         # 0-20 when Stockfish is used
auto_save_png        = false
highlight_brightness = 5          # 0=dim  5=default  10=vivid
cell_w               = 8          # board cell width in chars  (increase for bigger board)
cell_h               = 4          # board cell height in lines (keep ≈ cell_w/2 for square cells)
lichess_token        = lip_xxx    # optional — removes rate limits on daily puzzle
```

### Board size tuning

Terminal characters are not square — their exact proportions depend on your font. If the board looks too tall or too wide, adjust `cell_w` and `cell_h` until it looks right. A good starting ratio is `cell_w ≈ cell_h × 2`.

```ini
cell_w = 8   cell_h = 4    # default — square on most fonts
cell_w = 10  cell_h = 4    # wider cells — try if board looks too tall
cell_w = 8   cell_h = 3    # shorter cells — try if board looks too wide
```

You can also change these live in Settings → Board cell width / Board cell height.

---

## PNG export

`Shift+E` → preview → `Y` to confirm. Saved to `~/rchess_export/`.

Fonts with chess glyphs are auto-detected via `fc-list`. If you have a Nerd Font installed it will be used automatically — no extra install needed.

If you don't have a Nerd Font:

```bash
sudo pacman -S ttf-freefont
```

---

## Lichess daily puzzle

Main menu → **Daily Puzzle**. Full mouse support — click pieces to move. The puzzle board shows the eval bar.

Works anonymously, but a free token removes rate limits:

1. Sign up at [lichess.org](https://lichess.org)
2. Go to **lichess.org/account/security** → Personal API tokens
3. Click **Generate** — give it any name, **leave all permissions unchecked**
4. Copy the `lip_` token into your config:

```ini
lichess_token = lip_xxxxxxxxxxxxxxxxxxxx
```

---

## Opening book

CPU uses a built-in book for the first ~15 moves. The opening name shows in the top bar `[Sicilian — Najdorf]`.

Covered: Ruy López, Italian, Scotch, Sicilian (Najdorf, Dragon, Kan), French, Caro-Kann, Queen's Gambit, King's Indian, Nimzo-Indian, London, Réti, English, and more.

---

## Exports

Everything saved to `~/rchess_export/`:

| File | Trigger |
|------|---------|
| `rchess_board_<ts>.png` | `Shift+E`, or on game end if `auto_save_png = true` |
| `rchess_<movenum>.pgn` | `Shift+G`, or automatically on game end |

---

## Terminal requirements

| Mode | Recommended |
|------|-------------|
| Minimal | 100 × 28 |
| Standard | 120 × 34 |
| Analysis | 130 × 40 |
| Block art pieces | 130 × 40 minimum |
=======
| | Version | Notes |
|-|---------|-------|
| Rust + Cargo | 1.70+ | [rustup.rs](https://rustup.rs) — build dependency only |
| Terminal | 115×32+ recommended | Smaller works in Minimal mode |
| Python + Pillow | any | PNG export only |
| Stockfish | any | Analysis only |

---

## Debug log

All internal output goes to `/tmp/rchess.log` — the terminal is never polluted.

```bash
tail -f /tmp/rchess.log
```

---

## Preview

<div align="center">

![rchess gameplay](assets/rchess_board1.png)

![rchess exported PNG](assets/rchess_board2.png)

</div>
