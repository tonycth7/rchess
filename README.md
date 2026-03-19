<div align="center">

# ♟ rchess

A chess game that lives in your terminal. Pure Rust, no dependencies beyond a keyboard.

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://rustup.rs)
[![License](https://img.shields.io/badge/license-MIT-green)](#)

</div>

---

## What it is

rchess is a fully playable terminal chess game. Play against a friend, fight the built-in AI, get your moves analysed in real time, replay games, and export board snapshots as PNG — all from your terminal, all without leaving the keyboard (though mouse support is there if you want it).

---

## Getting started

```bash
# You need Rust — get it at rustup.rs if you don't have it
git clone https://github.com/yourname/rchess
cd rchess
cargo build --release
./target/release/rchess
```

That's it. Zero extra installs to play.

**Want more features?**

```bash
sudo pacman -S python-pillow   # PNG board export
sudo pacman -S stockfish       # stronger move analysis
sudo pacman -S ttf-freefont    # Unicode chess symbols in PNG (♔♕♖)
```

---

## Playing

Navigate with arrow keys or `hjkl`. Click a piece, click the destination. Or just type the move — `e2e4`, `Nf3`, `O-O` — an input bar appears automatically as soon as you start typing.

| Key | What it does |
|-----|-------------|
| `hjkl` / arrows | Move cursor |
| `Enter` / click | Select & move |
| Type a move | `e2e4`, `Nf3`, `O-O`, `O-O-O` |
| `u` | Undo |
| `r` | Replay the game |
| `E` (Shift) | Export board as PNG |
| `T` (Shift) | Cycle analysis panel |
| `d` | Offer draw (PvP only) |
| `s` | Settings |
| `q` | Main menu |

---

## Move analysis

After every move, a background engine quietly labels what just happened:

| | Label | Meaning |
|-|-------|---------|
| `B` | Book | Known opening line |
| `+` | Good | Sound move |
| `?!` | Inaccuracy | Small slip |
| `?` | Mistake | Significant error |
| `??` | Blunder | Serious mistake |

Labels appear in the history panel as they arrive — the UI never freezes for analysis.

Press `T` to cycle through **Minimal** (no engine info), **Standard** (compact label bar), and **Analysis** (full panel with top 3 candidate moves, eval before → after, and a sparkline across the whole game in Replay).

### Engine options

The built-in engine always works with no install. Stockfish is optional but much stronger.

| Engine | Typical speed | How to use |
|--------|--------------|-----------|
| Built-in depth 1 | ~1ms | Default |
| Built-in depth 2 | ~5–50ms | Settings → Analysis depth |
| Built-in depth 3 | ~50–500ms | Settings → Analysis depth |
| Stockfish | 300ms/position | `sudo pacman -S stockfish`, then Settings |

---

## Config

Press `s` → Settings → `w` to save. Or edit the file directly — it's picked up on next launch.

```ini
# ~/.config/rchess/rchess_tui.conf

theme           = classic      # classic tournament mocha slate midnight crimson
ai_depth        = 3            # 1 2 3 4
time_control    = infinite     # infinite bullet blitz rapid classical
ui_mode         = standard     # minimal standard analysis
analysis_engine = builtin      # builtin stockfish
analysis_depth  = 2            # 1=fast  2=balanced  3=strong
auto_save_png   = false        # set true to auto-save a PNG when each game ends
```

---

## PNG export

Hit `Shift+E` during a game, preview the board, press `Y`. Saves to `~/rchess_export/`.

The renderer detects fonts with chess glyphs via `fc-list` — your Nerd Font will likely work automatically. Falls back to geometric circles with letters if nothing suitable is found.

---

## Opening book

The CPU plays from a built-in book covering the first ~15 moves of the most common openings: Ruy López, Sicilian, French, Caro-Kann, Queen's Gambit, King's Indian, Nimzo-Indian, London, Réti, English, and more. The current opening name shows in the top bar while you're in book — `[Sicilian — Najdorf]`, `[Ruy López — Berlin]`, etc.

---

## Files saved automatically

Everything lands in `~/rchess_export/`:

- **PGN** — saved when the game ends
- **PNG** — saved when you press `Shift+E`, or on game end if `auto_save_png = true`

---

## Debugging

All internal logs go to `/tmp/rchess.log` — never to the terminal.

```bash
tail -f /tmp/rchess.log
```

---

## Requirements

| | Version | Notes |
|-|---------|-------|
| Rust + Cargo | 1.70+ | [rustup.rs](https://rustup.rs) |
| Terminal size | 115 × 32+ | Works smaller in Minimal mode |
| Python + Pillow | any | PNG export only, optional |
| Stockfish | any | Move analysis only, optional |

---

## Preview

<div align="center">

*Classic theme — mid-game with analysis panel*

![rchess in-game screenshot](assets/rchess_board1.png)

*Exported PNG — board position after 12 moves*

![rchess exported PNG](assets/rchess_board2.png)

</div>
