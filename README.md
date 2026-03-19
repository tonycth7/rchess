# ♟ rChess TUI

A fully-featured terminal chess game written in pure Rust — no web, no GUI, just your terminal.

## Build & run

```bash
cargo run --release
```

Requires **Rust 1.75+**. Zero system dependencies beyond `ratatui` and `crossterm`.

---

## Controls

### In-game

| Key | Action |
|-----|--------|
| `↑ ↓ ← →` | Move cursor |
| `h j k l` | Move cursor (Vim-style) |
| `Enter` / `Space` | Select piece / confirm move |
| `Esc` | Cancel selection |
| Any letter | Start typing a move in notation |
| `u` | Undo last move |
| `n` | New game |
| `s` | Open settings |
| `q` | Back to main menu |

### Typing moves

Press any letter to open the notation input box:

| You type | Meaning |
|----------|---------|
| `e2e4` | Coordinate — pawn from e2 to e4 |
| `Nf3` | Knight to f3 |
| `Qg4` | Queen to g4 |
| `exd5` | Pawn captures d5 |
| `O-O` | Kingside castle |
| `O-O-O` | Queenside castle |
| `e8=Q` | Promote pawn to queen |

Press `Enter` to execute, `Esc` to cancel, `Backspace` to delete.

---

## Modes

### Two players
Both players share the keyboard. Enable **Auto-flip** in settings to flip the board after each move so each player always sees their pieces at the bottom.

### vs Computer
Choose your colour. The AI uses **minimax with alpha-beta pruning** and **piece-square tables**.

| Level | Depth | Speed |
|-------|-------|-------|
| Easy | 1 ply | Instant |
| Medium | 2 ply | < 0.1s |
| Hard | 3 ply | ~0.4s |
| Expert | 4 ply | 1–5s |

---

## Chess clock

| Control | Time each |
|---------|-----------|
| Infinite | No clock |
| Bullet | 1 minute |
| Blitz | 3 minutes |
| Rapid | 10 minutes |
| Classical | 30 minutes |

Clock starts after White's first move. Under 30s it turns red. Under 10s it shows tenths (`0:09.4`). Flag fall ends the game immediately with an overlay.

---

## Undo (`u`)

| Mode | What gets undone |
|------|-----------------|
| Two-player | Last half-move |
| CPU Easy | Your last move only |
| CPU Medium/Hard/Expert | Your move + AI's reply |

---

## Settings (`s`)

Navigate `↑↓`, change values `←→`, save `w`, reset `r`.

| Setting | Options |
|---------|---------|
| Theme | Classic · Tournament · Mocha · Slate · Midnight · Crimson |
| Piece style | Unicode ♔♕♖ · ASCII K Q R · Bracketed [K][Q] |
| AI difficulty | Easy · Medium · Hard · Expert |
| Move hints | Dots · Highlight · Off |
| Time control | Infinite · Bullet · Blitz · Rapid · Classical |
| Show coords | ON / OFF |
| Show clock | ON / OFF |
| Flip board | ON / OFF |
| Auto-flip PvP | ON / OFF |
| Confirm move | ON / OFF |

Saved to `~/.rchess_tui.conf` (plain text, hand-editable).

---

## Project structure

```
src/
  main.rs     Terminal setup, 50ms event loop
  engine.rs   Board, move generation, check detection
  ai.rs       Minimax + alpha-beta + piece-square tables
  config.rs   Settings, Theme/TimeControl enums, file I/O
  app.rs      App state, chess clock, keyboard handlers, undo
  ui.rs       ratatui renderer — all screens and panels
```

---

## What's implemented

- [x] Full legal moves (pins, castling, en passant, promotion)
- [x] Check / checkmate / stalemate detection
- [x] Chess clock — Bullet / Blitz / Rapid / Classical / Infinite
- [x] Flag fall (time-out loss)
- [x] CPU AI — minimax, alpha-beta, piece-square tables
- [x] Notation input — coordinate and SAN
- [x] Undo
- [x] 6 colour themes with matching backgrounds
- [x] Auto-flip board in two-player mode
- [x] Move history, captured pieces, settings persistence

---

## Ideas for future improvements

### Gameplay
- **Increments** — add time per move (e.g. 3+2 blitz)
- **Draw offers** — offer / accept / decline a draw
- **50-move rule & threefold repetition** — automatic draw detection
- **FEN import/export** — start from any position, copy current to clipboard

### AI
- **Move ordering** — sort captures/checks first for better alpha-beta cutoffs
- **Quiescence search** — avoid horizon effect by extending noisy leaf nodes
- **Opening book** — load a Polyglot `.bin` file for natural early play
- **Iterative deepening** — always have a best move ready even if time runs out

### Interface
- **PGN export** — save completed games as standard `.pgn` files
- **Replay mode** — step through a game move by move
- **Mouse support** — crossterm supports mouse events; click to select pieces

---

## Config file

`~/.rchess_tui.conf`:

```ini
theme        = classic       # classic tournament mocha slate midnight crimson
piece_style  = unicode       # unicode letters fatletters
ai_depth     = 3             # 1 2 3 4
move_hints   = dots          # dots highlight none
time_control = infinite      # infinite bullet blitz rapid classical
show_coords  = true
show_clock   = true
flip_board   = false
auto_flip    = false
confirm_move = false
```
