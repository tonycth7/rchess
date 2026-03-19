<div align="center">

# ♟ rChess

A chess game that lives in your terminal. Pure Rust.

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://rustup.rs)
[![License](https://img.shields.io/badge/license-MIT-green)](#)

</div>

---

## What it is

rchess is a fully playable terminal chess game.

Play against a friend, fight the built-in AI, get your moves analysed in real time, replay games, and export board snapshots as PNG — all without leaving the keyboard (mouse support included if you want it).

---

## Install

### Quick install (Cargo)

```bash
cargo install --git https://github.com/tonycth7/rchess
rchess
````

### Build from source

```bash
git clone https://github.com/tonycth7/rchess
cd rchess
cargo build --release
./target/release/rchess
```

---

## Full feature support

If you want **everything working (analysis + PNG export):**

```bash
sudo pacman -S stockfish       # stronger move analysis
sudo pacman -S python-pillow   # PNG board export
```

### Optional

```bash
# if you don't have a Nerd Font
sudo pacman -S ttf-freefont    # Unicode chess symbols (♔♕♖)
```

---

## AUR (coming soon)

```bash
# paru
paru -S rchess

# yay
yay -S rchess
```

> Not published yet — PKGBUILD will be added soon.

---

## Playing

Navigate with arrow keys or `hjkl`.

* Select a piece → select destination
* Or type moves directly: `e2e4`, `Nf3`, `O-O`
* Input bar appears automatically when typing

| Key             | What it does                  |
| --------------- | ----------------------------- |
| `hjkl` / arrows | Move cursor                   |
| `Enter` / click | Select & move                 |
| Type a move     | `e2e4`, `Nf3`, `O-O`, `O-O-O` |
| `u`             | Undo                          |
| `r`             | Replay the game               |
| `E` (Shift)     | Export board as PNG           |
| `T` (Shift)     | Cycle analysis panel          |
| `d`             | Offer draw (PvP only)         |
| `s`             | Settings                      |
| `q`             | Main menu                     |

---

## Move analysis

After every move, a background engine labels what just happened:

|      | Label      | Meaning            |
| ---- | ---------- | ------------------ |
| `B`  | Book       | Known opening line |
| `+`  | Good       | Sound move         |
| `?!` | Inaccuracy | Small slip         |
| `?`  | Mistake    | Significant error  |
| `??` | Blunder    | Serious mistake    |

* Updates appear **live** in the history panel
* UI never freezes during analysis

Press `T` to switch modes:

* **Minimal** — no engine info
* **Standard** — compact labels
* **Analysis** — full engine panel (top moves, eval change, replay graph)

---

### Engine options

The built-in engine works out of the box. Stockfish is optional but much stronger.

| Engine           | Typical speed   | How to use                   |
| ---------------- | --------------- | ---------------------------- |
| Built-in depth 1 | ~1ms            | Default                      |
| Built-in depth 2 | ~5–50ms         | Settings → Analysis depth    |
| Built-in depth 3 | ~50–500ms       | Settings → Analysis depth    |
| Stockfish        | ~300ms/position | Install + enable in settings |

---

## Config

Press `s` → Settings → `w` to save
Or edit manually:

```ini
# ~/.config/rchess/rchess_tui.conf

theme           = classic      # classic tournament mocha slate midnight crimson
ai_depth        = 3            # 1 2 3 4
time_control    = infinite     # infinite bullet blitz rapid classical
ui_mode         = standard     # minimal standard analysis
analysis_engine = builtin      # builtin stockfish
analysis_depth  = 2            # 1=fast  2=balanced  3=strong
auto_save_png   = false        # auto-save PNG on game end
```

---

## PNG export

Press `Shift+E` during a game → preview → `Y`

Saved to:

```
~/rchess_export/
```

* Detects fonts via `fc-list`
* Uses Nerd Font if available
* Falls back to geometric pieces if not

---

## Opening book

Built-in opening book (~15 moves):

* Ruy López, Sicilian, French, Caro-Kann
* Queen’s Gambit, King’s Indian, Nimzo-Indian
* London, Réti, English

Live display:

```
[Sicilian — Najdorf]
```

---

## Files saved automatically

```
~/rchess_export/
├── *.pgn   # saved games
└── *.png   # board snapshots
```

* PGN → saved on game end
* PNG → manual or auto (`auto_save_png = true`)

---

## Debugging

Logs are written to:

```bash
tail -f /tmp/rchess.log
```

Never pollutes the terminal UI.

---

## Requirements

|                 | Version   | Notes                                  |
| --------------- | --------- | -------------------------------------- |
| Rust + Cargo    | 1.70+     | [https://rustup.rs](https://rustup.rs) |
| Terminal        | 115 × 32+ | Smaller works in Minimal mode          |
| Stockfish       | optional  | Strong analysis                        |
| Python + Pillow | optional  | PNG export                             |

---

## Preview

<div align="center">

*Classic theme — mid-game with analysis panel*

![rchess in-game screenshot](assets/rchess_board1.png)

*Exported PNG — board position after 12 moves*

![rchess exported PNG](assets/rchess_board2.png)

</div>
```

---
