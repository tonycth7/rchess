# ♟ rchess

A fast, keyboard-first chess experience in your terminal.

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://rustup.rs)
[![License](https://img.shields.io/badge/license-MIT-green)](#)

---

## What is rchess?

rchess is a fully playable terminal chess game with:

- ♟ PvP and built-in AI
- ⚡ Real-time move analysis
- 🎮 Keyboard-first controls (mouse optional)
- 🧠 Opening book (~15 moves deep)
- 🔁 Game replay + history
- 🖼 PNG board export

No GUI. No browser. Just your terminal.

---

## Install

### Cargo (recommended)

```
cargo install --git https://github.com/tonycth7/rchess
rchess
```

### Build from source

```
git clone https://github.com/tonycth7/rchess
cd rchess
cargo build --release
./target/release/rchess
```

---

## Full feature support

If you want **everything working (analysis + PNG export):**

```
sudo pacman -S stockfish   # strong analysis engine
sudo pacman -S python-pillow   # PNG export
```

---

### Optional

```
sudo pacman -S ttf-freefont   # if you don't have a Nerd Font
```

---

### AUR (coming soon)

```
paru -S rchess
yay -S rchess
```

> Not published yet — PKGBUILD coming soon.

---

## Controls

| Key | Action |
|-----|--------|
| hjkl / arrows | Move cursor |
| Enter / click | Select / move |
| e2e4, Nf3, O-O | Type moves |
| u | Undo |
| r | Replay |
| E | Export PNG |
| T | Toggle analysis |
| d | Offer draw |
| s | Settings |
| q | Menu |

---

## Move analysis

| Label | Meaning |
|-------|--------|
| B | Book |
| + | Good |
| ?! | Inaccuracy |
| ? | Mistake |
| ?? | Blunder |

---

## Engine

| Engine | Notes |
|--------|------|
| Built-in | Fast |
| Stockfish | Strong |

---

## Config

```
~/.config/rchess/rchess_tui.conf
```

---

## Export

Saved to:

```
~/rchess_export/
```

---

## Debug

```
tail -f /tmp/rchess.log
```

---

## Requirements

- Rust 1.70+
- Terminal 115x32+
- Stockfish (for full analysis)
- Python Pillow (for PNG export)
