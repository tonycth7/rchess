# RChess TUI — Complete UI Reference

> Edit this file to track UI changes. All rendered colors, layouts, pieces, highlights, keybindings, screens, and settings are documented below.

---

## 1. Drawing Architecture

All rendering in `src/ui.rs`. Entry: `render()` dispatches per `Screen` variant. Board dimensions auto-computed from terminal size each frame.

**Theme access helpers (`ui.rs:16-24`):**
```rust
fn rgb(c: (u8,u8,u8))  -> Color       // tuple → Color::Rgb
fn bg0(th: Theme) -> Color            // rgb(th.bg_colors().0) — main background
fn bg1(th: Theme) -> Color            // rgb(th.bg_colors().1) — panel background
fn ac(th: Theme)  -> Color            // rgb(th.accent())
fn sq_l(th: Theme)-> Color            // rgb(th.squares().0)  — light square
fn sq_d(th: Theme)-> Color            // rgb(th.squares().1)  — dark square
fn pw(th: Theme)  -> Color            // rgb(th.piece_colors().0) — white pieces
fn pb_c(th: Theme)-> Color            // rgb(th.piece_colors().1) — black pieces
fn sty(c: Color)  -> Style            // Style::default().fg(c)
```

**Named color constants (`ui.rs:26-32`):**

| Name | RGB | Hex | Usage |
|------|-----|-----|-------|
| `CREAM` | `(220, 210, 185)` | `#DCD2B9` | Primary text on panels |
| `DIM` | `(110, 130, 110)` | `#6E826E` | Secondary/dim text |
| `DIMMER` | `(55, 70, 55)` | `#374637` | Very dim / tertiary text |
| `RED` | `(200, 70, 70)` | `#C84646` | Errors, check warnings, Blunder label |
| `TEAL` | `(80, 180, 140)` | `#50B48C` | Active turn, Good/Book label |
| `AMBER` | `(220, 160, 40)` | `#DCA028` | Draw, Stalemate, Inaccuracy label |
| `GREEN` | `(120, 220, 160)` | `#78DCA0` | Correct, success, Good label |

---

## 2. Screens (Screen enum)

| Screen | Render fn | Description |
|--------|-----------|-------------|
| `Menu` | `draw_menu` | Main menu with 8 options |
| `ColorPick` | `draw_color_pick` | Choose white/black for CPU game |
| `Settings` | `draw_settings` | Scrolling settings panel |
| `Game` | `draw_game` | Main game board + sidebar |
| `Promo` | `draw_promo` (overlay on Game) | Pawn promotion picker |
| `DrawOffer` | `draw_draw_offer` (overlay on Game) | Accept/decline draw |
| `Replay` | `draw_replay` | Game replay with eval bar + sparkline |
| `PngPreview` | `draw_png_preview` (overlay on Game) | PNG export confirmation |
| `PgnSaved` | `draw_pgn_saved` (overlay on Game) | PGN save confirmation |
| `FenInput` | `draw_fen_input` | Type/paste FEN string |
| `PgnImport` | `draw_pgn_import` | Type/paste PGN text or file path |
| `Puzzle` | `draw_puzzle` | Lichess daily puzzle |
| `OnlineLobby` | `draw_online_lobby` | Online play lobby |
| `OnlineGame` | `draw_online_game` | Active Lichess game |
| `LoadGame` | `draw_load_game` | Browse saved games |

---

## 3. All Enums

### Mode (`app.rs:37-38`)
- `Mode::PvP` — Two-player hotseat
- `Mode::CPU` — Play vs computer (AI or Stockfish)

### Screen (`app.rs:22-35`)
- `Menu`, `ColorPick`, `Settings`, `Game`, `Promo`, `DrawOffer`, `Replay`, `PngPreview`, `PgnSaved`, `FenInput`, `PgnImport`, `Puzzle`, `OnlineLobby`, `OnlineGame`, `LoadGame`

### UiMode (`config.rs:186-193`)
| Variant | Description | Sidebar panels |
|---------|-------------|----------------|
| `Minimal` | No engine panel | PLAYERS, STATUS, HISTORY, KEYS |
| `Standard` | Compact two-line eval bar (**default**) | PLAYERS, STATUS, EVAL, HISTORY, KEYS |
| `Analysis` | Full engine analysis | PLAYERS, STATUS, ANALYSIS, HISTORY, KEYS |

### PieceStyle (`config.rs:78-89`)
| Variant | Label | Description |
|---------|-------|-------------|
| `Unicode` | `"Unicode symbols"` (**default**) | Single-character chess symbols ♔♕♖♗♘♙ |
| `Ascii` | `"ASCII art (star pieces)"` | 5×3 character patterns |

### MoveHints (`config.rs:135-144`)
| Variant | Label | Empty square | Opponent piece |
|---------|-------|-------------|----------------|
| `Dots` | `"Dots (⬤ on target squares)"` (**default**) | `dot_tint` bg + ⬤ | `dot_tint` bg |
| `Highlight` | `"Highlight (full square glow)"` | `hl_color` bg | `hl_color` bg |
| `None` | `"Off (no hints)"` | nothing | nothing |

### TimeControl (`config.rs:149-181`)
| Variant | Label | Initial ms | Short label |
|---------|-------|-----------|-------------|
| `Infinite` | `"Infinite (no clock)"` | `None` | `∞` |
| `Bullet` | `"Bullet (1 min each)"` | `60_000` | `1'` |
| `Blitz` | `"Blitz (3 min each)"` | `180_000` | `3'` |
| `Rapid` | `"Rapid (10 min each)"` | `600_000` | `10'` |
| `Classical` | `"Classical (30 min each)"` | `1_800_000` | `30'` |

### AiDepth (`config.rs:93-118`)
| Variant | Label | `depth()` | Opponent type |
|---------|-------|-----------|---------------|
| `Easy` | `"Easy (depth 1)"` | 1 | Built-in minimax |
| `Medium` | `"Medium (depth 2)"` | 2 | Built-in minimax |
| `Hard` | `"Hard (depth 3)"` (**default**) | 3 | Built-in minimax |
| `Expert` | `"Expert (depth 4)"` | 4 | Built-in minimax |
| `Stockfish` | `"Stockfish (strongest)"` | 4 (fallback) | External Stockfish UCI |

### AnalysisEngine (`config.rs:215-228`)
| Variant | Label | Description |
|---------|-------|-------------|
| `Builtin` | `"Built-in (minimax, always works)"` (**default**) | Pure Rust minimax |
| `Stockfish` | `"Stockfish (pacman -S stockfish)"` | External UCI engine |

### ClockState (`app.rs:40-41`)
- `Running` — Clock ticking
- `Paused` — Clock stopped
- `Flagged` — Player ran out of time

### GameEnd (`app.rs:153`)
- `Normal` — Regular game over (checkmate/stalemate)
- `Draw` — Game ended by draw agreement
- `Flag` — Game ended by flag fall

### MoveLabel (`app.rs:65`)
| Variant | Icon | Name | Delta threshold |
|---------|------|------|----------------|
| `Book` | `B` | Book | N/A (opening book move) |
| `Good` | `+` | Good | `delta_cp > -inaccuracy_cp` |
| `Inaccuracy` | `?!` | Inaccuracy | `delta_cp > -mistake_cp` |
| `Mistake` | `?` | Mistake | `delta_cp > -blunder_cp` |
| `Blunder` | `??` | Blunder | `delta_cp <= -blunder_cp` |

Default thresholds: `inaccuracy_cp=50`, `mistake_cp=100`, `blunder_cp=300` (configurable in config file)

### PuzzleState (`puzzle.rs:17-32`)
- `Loading` — Fetching from Lichess
- `Failed(String)` — Error message
- `Setup` — Replaying opponent setup moves
- `WaitingInput` — Waiting for human move
- `CorrectMove` — Correct, showing next opponent move
- `WrongMove(String)` — Wrong move played
- `Solved` — Puzzle completed

---

## 4. All Themes — Full Color Palettes

Each theme provides 10 color tokens. Theme data defined in `config.rs:22-73`.

### Theme token reference

| Token | Method | Purpose |
|-------|--------|---------|
| squares light | `squares().0` | Light chessboard squares |
| squares dark | `squares().1` | Dark chessboard squares |
| accent | `accent()` | Borders, titles, highlights |
| select | `select()` | Selected piece background |
| cursor | `cursor()` | Cursor square background |
| last_move | `last_move()` | Last move blend target |
| white_piece | `piece_colors().0` | White piece foreground |
| black_piece | `piece_colors().1` | Black piece foreground |
| bg0 | `bg_colors().0` | Main background (terminal) |
| bg1 | `bg_colors().1` | Panel background |

### Classic (cream/brown)

| Token | RGB | Hex | Color chip |
|-------|-----|-----|------------|
| squares light | `(240, 217, 181)` | `#F0D9B5` | `🟨` |
| squares dark | `(181, 136, 99)` | `#B58863` | `🟫` |
| accent | `(160, 110, 60)` | `#A06E3C` | `🟤` |
| select | `(205, 210, 60)` | `#CDD23C` | `🟡` |
| cursor | `(200, 180, 50)` | `#C8B432` | `🟡` |
| last_move | `(160, 140, 30)` | `#A08C1E` | `🟡` |
| white_piece | `(255, 248, 220)` | `#FFF8DC` | `⬜` |
| black_piece | `(80, 60, 40)` | `#503C28` | `🟤` |
| bg0 | `(36, 22, 12)` | `#24160C` | `⬛` |
| bg1 | `(52, 34, 18)` | `#342212` | `⬛` |

### Tournament (green/gold)

| Token | RGB | Hex | Color chip |
|-------|-----|-----|------------|
| squares light | `(100, 140, 100)` | `#648C64` | `🟩` |
| squares dark | `(50, 80, 50)` | `#325032` | `🟩` |
| accent | `(210, 175, 60)` | `#D2AF3C` | `🟨` |
| select | `(180, 160, 40)` | `#B4A028` | `🟨` |
| cursor | `(200, 180, 50)` | `#C8B432` | `🟨` |
| last_move | `(160, 140, 30)` | `#A08C1E` | `🟨` |
| white_piece | `(240, 240, 240)` | `#F0F0F0` | `⬜` |
| black_piece | `(50, 55, 50)` | `#323732` | `⬛` |
| bg0 | `(14, 20, 14)` | `#0E140E` | `⬛` |
| bg1 | `(22, 30, 22)` | `#161E16` | `⬛` |

### Slate (grey/cyan)

| Token | RGB | Hex | Color chip |
|-------|-----|-----|------------|
| squares light | `(90, 110, 130)` | `#5A6E82` | `🟦` |
| squares dark | `(50, 65, 80)` | `#324150` | `🟦` |
| accent | `(80, 200, 200)` | `#50C8C8` | `🟩` |
| select | `(60, 180, 180)` | `#3CB4B4` | `🟩` |
| cursor | `(200, 180, 50)` | `#C8B432` | `🟨` |
| last_move | `(160, 140, 30)` | `#A08C1E` | `🟨` |
| white_piece | `(230, 230, 240)` | `#E6E6F0` | `⬜` |
| black_piece | `(100, 140, 160)` | `#648CA0` | `🟦` |
| bg0 | `(16, 20, 28)` | `#10141C` | `⬛` |
| bg1 | `(22, 28, 40)` | `#161C28` | `⬛` |

### MatteBlack (dark grey/gold)

| Token | RGB | Hex | Color chip |
|-------|-----|-----|------------|
| squares light | `(55, 55, 55)` | `#373737` | `⬜` |
| squares dark | `(22, 22, 22)` | `#161616` | `⬛` |
| accent | `(200, 160, 30)` | `#C8A01E` | `🟨` |
| select | `(180, 140, 20)` | `#B48C14` | `🟨` |
| cursor | `(220, 180, 30)` | `#DCB41E` | `🟨` |
| last_move | `(160, 130, 20)` | `#A08214` | `🟨` |
| white_piece | `(240, 230, 200)` | `#F0E6C8` | `⬜` |
| black_piece | `(180, 170, 160)` | `#B4AAA0` | `⬜` |
| bg0 | `(8, 8, 8)` | `#080808` | `⬛` |
| bg1 | `(16, 16, 16)` | `#101010` | `⬛` |

---

## 5. Inline Hardcoded Colors

These are NOT theme-driven — they are fixed `Color::Rgb` values baked into `ui.rs`.

| RGB | Hex | Line | Purpose |
|-----|-----|------|---------|
| `(200, 30, 30)` | `#C81E1E` | 350 | Check indicator (king square background) |
| `(0, 255, 140)` | `#00FF8C` | 376 | Hint dot foreground blend target |
| `(220, 200, 40)` | `#DCC828` | 380 | Dot tint blend target (gold) |
| `(200, 240, 80)` | `#C8F050` | 385 | Highlight blend target (light squares) |
| `(40, 220, 150)` | `#28DC96` | 385 | Highlight blend target (dark squares) |
| `(80, 180, 140)` | `#50B48C` | 726 | History BLACK move highlight bg |
| `(210, 175, 60)` | `#D2AF3C` | 802,1517 | Checkmate title + status text |
| `(120, 220, 160)` | `#78DCA0` | 1130 | Sparkline bar: White better |
| `(200, 70, 70)` | `#C84646` | 1132 | Sparkline bar: Black better |
| `(110, 130, 110)` | `#6E826E` | 1134 | Sparkline bar: neutral |
| `(20, 20, 20)` / `(200, 200, 200)` | `#141414`/`#C8C8C8` | 1167 | Eval bar: Black section bg/fg |
| `(230, 230, 210)` / `(30, 30, 30)` | `#E6E6D2`/`#1E1E1E` | 1180 | Eval bar: White section bg/fg |
| `(120, 120, 80)` / `(240, 220, 40)` | `#787850`/`#F0DC28` | 1191 | Eval bar: equal/busy bg/fg |
| `(220, 120, 40)` | `#DC7828` | 1391 | Mistake label (orange) text |

---

## 6. Piece Rendering

### 6a. Unicode Symbols (`engine.rs:15-23`)

Each piece is a single Unicode character:

| Piece | White (code point) | Black (code point) |
|-------|--------------------|--------------------|
| King | `♔` (U+2654) | `♚` (U+265A) |
| Queen | `♕` (U+2655) | `♛` (U+265B) |
| Rook | `♖` (U+2656) | `♜` (U+265C) |
| Bishop | `♗` (U+2657) | `♝` (U+265D) |
| Knight | `♘` (U+2658) | `♞` (U+265E) |
| Pawn | `♙` (U+2659) | `♟` (U+265F) |

**Render method** (`ui.rs:440-461`):
- Symbol is a single Unicode char centered on the middle line (`mid = CELL_H / 2`)
- Decorative bars (`───`) drawn on lines `mid-1` and `mid+1` when `CELL_H ≥ 4`
- Last line: target underline `  ──────  ` when square `is_tgt`
- Style: `Style::default().bg(bg).fg(pc).add_modifier(Modifier::BOLD)`
- Foreground: `pw(th)` for white pieces, `pb_c(th)` for black pieces

### 6b. ASCII Star Pieces (`pieces.rs`)

All patterns are **5 wide × 3 tall**. White = outline, Black = filled with `#`.

**Row layout:**
- Row 1: 1-space padding + 3-char content + 1-space padding (5 chars total)
- Row 2: 1-space padding + 3-char content + 1-space padding
- Row 3: 5 chars, no padding (extends 1 char on each side)

```
     1         2         3         4         5
    12345     12345     12345     12345     12345
```

**White pieces (outline):**

| Piece | Row 1 | Row 2 | Row 3 |
|-------|-------|-------|-------|
| King `WK` | `\ +/` | ` ) ( ` | `/___\` |
| Queen `WQ` | `\ ^/` | ` ) ( ` | `/___\` |
| Bishop `WB` | `( V )` | ` ) ( ` | `/___\` |
| Knight `WN` | `( "\` | ` ) ' ` | `/___\` |
| Rook `WR` | `[ - ]` | ` | | ` | `/___\` |
| Pawn `WP` | `  _  ` | ` ( ) ` | `/___\` |

**Black pieces (filled):**

| Piece | Row 1 | Row 2 | Row 3 |
|-------|-------|-------|-------|
| King `BK` | `\ +/` | ` )#( ` | `/###\` |
| Queen `BQ` | `\ ^/` | ` )#( ` | `/###\` |
| Bishop `BB` | `( V )` | ` )#( ` | `/###\` |
| Knight `BN` | `( "\` | ` )#' ` | `/###\` |
| Rook `BR` | `[ - ]` | ` |#| ` | `/###\` |
| Pawn `BP` | `  _  ` | ` (#) ` | `/###\` |

**Visual representation (5×3 grid, monospace):**

```
WHITE:
        K      Q      B      N      R      P
       \ +/   \ ^/   ( V )  ( "\   [ - ]   _
       ) (    ) (    ) (    ) '    | |    ( )
       /___\  /___\  /___\  /___\  /___\  /___\

BLACK:
        K      Q      B      N      R      P
       \ +/   \ ^/   ( V )  ( "\   [ - ]   _
       )#(    )#(    )#(    )#'    |#|    (#)
       /###\  /###\  /###\  /###\  /###\  /###\
```

**Render function** (`pieces.rs:47-77`):
- `render_star_lines(p, cell_w, cell_h) -> Vec<String>`
- When cell ≥ 5×3: pattern centered with padding `pad_top` and `pad_left`
- When cell < 5×3: nearest-neighbor scaled down (e.g., 12 chars → 6 chars = every other pixel)
- Style: `Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)`
- Foreground colors: same as Unicode mode (theme `pw` / `pb_c`)

### 6c. Piece Colors Per Theme

| Theme | White piece (`pw`) | Black piece (`pb_c`) |
|-------|--------------------|----------------------|
| Classic | `(255, 248, 220)` `#FFF8DC` | `(80, 60, 40)` `#503C28` |
| Tournament | `(240, 240, 240)` `#F0F0F0` | `(50, 55, 50)` `#323732` |
| Slate | `(230, 230, 240)` `#E6E6F0` | `(100, 140, 160)` `#648CA0` |
| MatteBlack | `(240, 230, 200)` `#F0E6C8` | `(180, 170, 160)` `#B4AAA0` |

---

## 7. Square & Highlight System

### Square Colors
- Light: `sq_l(th)` — from `theme.squares().0`
- Dark: `sq_d(th)` — from `theme.squares().1`
- Determined by `(r + c) % 2 == 0`

### Background Priority (highest → lowest, `ui.rs:412-422`)

```
1. Selected (is_sel)      → sel_bg  = rgb(th.select())
2. Check   (is_kchk)       → chk_bg  = Color::Rgb(200, 30, 30)   ← HARDCODED RED
3. Cursor  (is_cur)        → cur_bg  = rgb(th.cursor())
4. Target  (is_tgt):
   - MoveHints::Highlight  → hl_color(is_light)
   - MoveHints::Dots       → dot_tint(is_light)   ← opponent pieces included
5. Last move (is_last)     → lm_color(is_light)    ← 1:2 blend with square
6. Default                 → sq_l(th) or sq_d(th)
```

### Highlight Blend Colors

| Output variable | Target RGB | Description | Used for |
|----------------|-----------|-------------|----------|
| `hint_fg` | `(0, 255, 140)` bright green-cyan | Dot foreground | `center_str("⬤", ...)` |
| `dot_tint` | `(220, 200, 40)` gold | Dot mode square tint | Background of target squares |
| `hl_color` (light) | `(200, 240, 80)` | Highlight mode light sq | Full square background |
| `hl_color` (dark) | `(40, 220, 150)` | Highlight mode dark sq | Full square background |
| `lm_color` | `theme.last_move()` | Last move blend | Both from/to squares |

### Blend Formula (`ui.rs:364-372`)

```rust
fn blend_color(base: Color, target: (u8,u8,u8)) -> Color {
    let t = brightness;  // 0..10, from config highlight_brightness
    Color::Rgb(
        (base.r * (10-t) + target.0 * t) / 10,
        (base.g * (10-t) + target.1 * t) / 10,
        (base.b * (10-t) + target.2 * t) / 10,
    )
}
```

Brightness levels: `0` = barely visible, `5` = default, `10` = vivid neon.

### Target Square Visual Comparison

| Mode | Empty square visual | Opponent piece square visual |
|------|-------------------|-----------------------------|
| **Dots** | `dot_tint` bg + `⬤` mid-line + underline | `dot_tint` bg + underline |
| **Highlight** | `hl_color` bg + underline | `hl_color` bg + underline |
| **None** | nothing | nothing |

### Check Square
- Always hardcoded `Color::Rgb(200, 30, 30)`, never uses theme
- Overrides all other highlights

### Last Move Squares
- Both `from` and `to` of the most recent `HistEntry`
- Color: 1-part square color + 2-parts `theme.last_move()` blended together
- Always overrides default square but NOT selected/check/cursor/target

---

## 8. Board Layout & Cell Dimensions

### Auto-computed each frame (`app.rs:296-312`)

```rust
overhead_w = if show_coords { 8 } else { 2 };  // coord label + border padding
overhead_h = if show_coords { 2 } else { 0 };  // top + bottom coord rows
sidebar    = 22;                                 // width of right side panels
avail_w    = term_w - overhead_w - sidebar;
input_h    = 4;                                  // input bar height
topbar_h   = 1;                                  // top bar height
avail_h    = term_h - topbar_h - input_h - overhead_h;
max_cw     = if Ascii { 14 } else { 20 };        // max cell width per style
CELL_W     = max(3, min(avail_w / 8, max_cw));
CELL_H     = max(2, avail_h / 8);
```

Defaults: `cell_w=8`, `cell_h=4` on a standard 80×24 terminal.

### Mouse hit-testing constants (`app.rs:1238-1243`)
```
TOP_BAR    = 1   — top bar row offset
BORDER     = 1   — board border width
coord_rows = show_coords ? 1 : 0   — coordinate row above/below
coord_cols = show_coords ? 3 : 0   — coordinate column width
```

### Board rendering
- Border: `BorderType::Rounded`, border color `ac(th)`
- Title: `" BOARD "` in `ac(th)` bold
- Background: `bg0(th)`
- Board width formula: `bw = min(3 + 8*CELL_W + 3 + 2, area.width)`
- Row order: `[0,1,2,3,4,5,6,7]` normally, `[7,6,5,4,3,2,1,0]` when flipped
- Col order: same as row order
- Coordinate labels: rank numbers ` 8 .. 1 `, file letters ` a b c d e f g h `
- Label style: `DIM` color

### Input bar (`draw_input`, `ui.rs:506-537`)
- Rounded border, border color varies:
  - `accent` when typing or piece selected
  - `RED` when `self.input_err` is set
  - `DIM` when idle
- Content:
  - Idle: `" [e4] type move or navigate"` (CREAM/DIM)
  - Selected: `" [e4] e2e4→ target + Enter"` (TEAL)
  - Typing: `" [e4] e2e4→ █"` (CREAM, with cursor)
  - Error: `" ✗ illegal move"` (RED, on second line)

---

## 9. Overlay Sizes and Border Styles

| Overlay | Width | Height | Border | Title color |
|---------|-------|--------|--------|-------------|
| Menu | 58 | 28 | Double | accent |
| ColorPick | 52 | 18 | Double | accent |
| Settings | 72 | max(10, area-4) | Double | accent |
| GameOver | 52 | 16 | Double | varies (RED/AMBER/gold) |
| Promo | 50 | 9 | Double | accent |
| DrawOffer | 46 | 11 | Double | AMBER |
| PngPreview | 56 | 24 | Double | GREEN |
| PgnSaved | 62 | 16 | Double | GREEN |
| FenInput | 70 | 18 | Double | accent |
| PgnImport | 72 | 22 | Double | accent |
| LoadGame | 72 | 28 | Rounded | accent |
| OnlineLobby | 56 | dynamic, max 28 | Double | accent |
| Puzzle loading | 40 | 7 | Rounded | accent |
| Puzzle failed | 66 | 19 | Rounded | RED |

Helper: `pad(r, px, py)` → `Rect(x+px, y+py, w-px*2, h-py*2)`. Typically `pad(rect, 2, 1)`.

---

## 10. Menu Screen (`draw_menu`, `ui.rs:67-111`)

```
┌──────────────────────────────────────────┐
│                                          │
│   ♔ ♕ ♗ ♘ ♖ ♙                   ← pw(th) bold
│   ♟ ♜ ♞ ♝ ♛ ♚                   ← pb_c(th) bold
│                                          │
│          R  C H E S S            ← ac(th) bold
│       TERMINAL EDITION  v0.7.3   ← DIM
│       PNG export · PGN · Replay  ← DIMMER
│                                          │
│   ────────────────────────────────────── │ ← DIMMER
│                                          │
│    ▶  ♟  TWO PLAYERS              ← selected: bg=ac(th), fg=bg0(th), bold
│                                          │
│       ◈  VS COMPUTER   [AI+Book]  ← unselected: fg=CREAM
│                                          │
│       ☍  ONLINE PLAY   [relay]          │
│                                          │
│       ⚙  SETTINGS                        │
│                                          │
│       ≡  START FROM FEN                  │
│                                          │
│       ↥  LOAD PGN FILE                   │
│                                          │
│       ⇪  LOAD SAVED GAME                 │
│                                          │
│       ★  DAILY PUZZLE  [Lichess]         │
│                                          │
│   ────────────────────────────────────── │
│                                          │
│   ↑↓ navigate    Enter select    q quit  │ ← DIMMER
└──────────────────────────────────────────┘
```

---

## 11. Settings Screen (`draw_settings`, `ui.rs:147-263`)

```
┌─── ⚙  SETTINGS ───────────────────────────────┐
│                                                │
│   ──────────────────────────────────────────── │  ← DIMMER separator
│                                                │
│   ▲ N more above (scroll indicator)            │  ← DIMMER
│                                                │
│    ▶ Theme              ◀  Classic      ▶      │  ← selected row
│                                                │       ▶ = ac(th)
│       Piece style              Unicode         │       label: ac(th) bold, 18 wide
│                                                │       ◀ value ▶: fg=bg0, bg=ac(th), bold
│       AI difficulty              Hard          │
│                                                │  ← unselected row
│       Move hints                 Dots          │       label: CREAM
│                                                │       value: DIM, "   value   "
│       Time control            Infinite         │
│                                                │
│       Show coords                  ON          │
│                                                │
│       Show clock                   ON          │
│                                                │
│       Flip board                  OFF          │
│                                                │
│       Auto-flip PvP               OFF          │
│                                                │
│       Confirm move                OFF          │
│                                                │
│       Auto-promote                OFF          │
│                                                │
│       UI mode                 Standard         │
│                                                │
│       Analysis engine            Builtin       │
│                                                │
│       Analysis depth        2  (balanced)      │
│                                                │
│       Auto-save PNG                OFF         │
│                                                │
│       Auto-save game               ON          │
│                                                │
│       Highlight bright      5  (default)       │
│                                                │
│       Lichess token          (not set)         │
│                                                │
│   ▼ N more below  (↓ to scroll)                │  ← DIMMER
│                                                │
│   ──────────────────────────────────────────── │
│   w/s save  r reset  ←→/hl change  jk/Esc     │  ← DIMMER (or GREEN "✓ Saved /path" if saved)
└────────────────────────────────────────────────┘
```

### 18 Settings Rows

| # | Label | Values (cycle order) | Config field | Type |
|---|-------|---------------------|--------------|------|
| 0 | Theme | Classic, Tournament, Slate, MatteBlack | `cfg.theme` | `Theme` |
| 1 | Piece style | Unicode, Ascii | `cfg.piece_style` | `PieceStyle` |
| 2 | AI difficulty | Easy, Medium, Hard, Expert, Stockfish | `cfg.ai_depth` | `AiDepth` |
| 3 | Move hints | Dots, Highlight, None | `cfg.move_hints` | `MoveHints` |
| 4 | Time control | Infinite, Bullet, Blitz, Rapid, Classical | `cfg.time_control` | `TimeControl` |
| 5 | Show coords | ON, OFF | `cfg.show_coords` | `bool` |
| 6 | Show clock | ON, OFF | `cfg.show_clock` | `bool` |
| 7 | Flip board | ON, OFF | `cfg.flip_board` | `bool` |
| 8 | Auto-flip PvP | ON, OFF | `cfg.auto_flip` | `bool` |
| 9 | Confirm move | ON, OFF | `cfg.confirm_move` | `bool` |
| 10 | Auto-promote | OFF, ON (queen) | `cfg.auto_promote` | `bool` |
| 11 | UI mode | Minimal, Standard, Analysis | `cfg.ui_mode` | `UiMode` |
| 12 | Analysis engine | Builtin, Stockfish | `cfg.analysis_engine` | `AnalysisEngine` |
| 13 | Analysis depth | 1 (fast), 2 (balanced), 3 (strong) | `cfg.analysis_depth` | `u8` |
| 14 | Auto-save PNG | ON, OFF | `cfg.auto_save_png` | `bool` |
| 15 | Auto-save game | ON, OFF | `cfg.auto_save_game` | `bool` |
| 16 | Highlight bright | 0 (dim), 5 (default), 10 (vivid) | `cfg.highlight_brightness` | `u8 (0-10)` |
| 17 | Lichess token | (not set), (configured ✓) | `cfg.lichess_token` | `String` |

---

## 12. Config Struct & Config File

### Full Config struct (`config.rs:233-267`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `theme` | `Theme` | `Classic` | Visual theme |
| `piece_style` | `PieceStyle` | `Unicode` | Piece rendering style |
| `ai_depth` | `AiDepth` | `Hard` | CPU opponent strength |
| `move_hints` | `MoveHints` | `Dots` | Target square hints |
| `time_control` | `TimeControl` | `Infinite` | Chess clock setting |
| `show_coords` | `bool` | `true` | Show rank/file labels |
| `show_clock` | `bool` | `true` | Show chess clock |
| `flip_board` | `bool` | `false` | Always flip board |
| `auto_flip` | `bool` | `false` | Auto-flip on PvP turn |
| `confirm_move` | `bool` | `false` | Require confirm before move |
| `auto_promote` | `bool` | `false` | Auto-promote pawns to queen |
| `ui_mode` | `UiMode` | `Standard` | Sidebar layout mode |
| `auto_save_png` | `bool` | `false` | Auto-export PNG on game end |
| `auto_save_game` | `bool` | `true` | Auto-save JSON on game end |
| `lichess_token` | `String` | `""` | Lichess API personal token |
| `lichess_username` | `String` | `""` | Lichess username (auto-filled) |
| `blunder_cp` | `i32` | `300` | Centipawn threshold for Blunder |
| `mistake_cp` | `i32` | `100` | Centipawn threshold for Mistake |
| `inaccuracy_cp` | `i32` | `50` | Centipawn threshold for Inaccuracy |
| `stockfish_skill` | `u8` | `10` | Stockfish skill 0-20 (CPU opponent) |
| `analysis_engine` | `AnalysisEngine` | `Builtin` | Analysis backend |
| `analysis_depth` | `u8` | `2` | Analysis depth 1-3 |
| `highlight_brightness` | `u8` | `5` | Highlight intensity 0-10 |

### Config file format

Path: `~/.config/rchess/rchess_tui.conf`

Key-value format, one per line. Lines starting with `#` are comments.

```ini
# Theme options: classic | tournament | slate | matteblack
theme            = classic
# Piece style: unicode | ascii
piece_style      = unicode
# AI depth: 1=easy, 2=medium, 3=hard, 4=expert, 5=stockfish
ai_depth         = 3
# Move hints: dots | highlight | none
move_hints       = dots
# Time control: infinite | bullet | blitz | rapid | classical
time_control     = infinite
# Booleans: true | false
show_coords      = true
show_clock       = true
flip_board       = false
auto_flip        = false
confirm_move     = false
auto_promote     = false
# UI mode: minimal | standard | analysis
ui_mode          = standard
auto_save_png    = false
auto_save_game   = true
lichess_token    =
lichess_username =
# Move analysis thresholds (centipawns)
blunder_cp       = 300
mistake_cp       = 100
inaccuracy_cp    = 50
stockfish_skill  = 10
# Analysis engine: builtin | stockfish
analysis_engine  = builtin
analysis_depth   = 2
# Highlight brightness 0-10
highlight_brightness = 5
```

---

## 13. Game Screen (`draw_game`, `ui.rs:270-286`)

```
┌──────────────────────────────────────────────────────────┐
│ ♟ RChess v0.7.3  Mv 12  [Italian]  [+0.8]  │ PvP       │  ← top bar (bg1)
├────────────────────────────┬─────────────────────────────┤
│                            │  ┌─ PLAYERS ────────┐      │
│                            │  │ ▶ ♔ WHITE [YOU]  │      │
│                            │  │   +3   5:00      │      │
│      ┌─ BOARD ──────┐     │  │   ♜♝♞            │      │
│      │              │     │  │ ────────────────  │      │
│      │   (8×8)      │     │  │   ♚ BLACK [CPU]   │      │
│      │              │     │  │   -3   4:30       │      │
│      │              │     │  │   ♙♙♙♙♙♙♙♙       │      │
│      └──────────────┘     │  └───────────────────┘      │
│                            │  ┌─ STATUS ─────────┐      │
│  ┌─ INPUT ────────────┐   │  │ ▶ WHITE TO MOVE  │      │
│  │ [e4] type move ... │   │  │ Move 12           │      │
│  └─────────────────────┘   │  └──────────────────┘      │
│                            │  ┌─ EVAL ──────────┐      │  ← Standard only
│                            │  │ [+0.8] e2e4     │      │
│                            │  └──────────────────┘      │
│                            │  ┌─ HISTORY ───────┐      │
│                            │  │ 1. e4      e5   │      │
│                            │  │ 2. Nf3     Nc6  │      │
│                            │  └──────────────────┘      │
│                            │  ┌─ KEYS ──────────┐      │
│                            │  │ ↑↓←→ cursor     │      │
│                            │  │ E:PNG G:PGN T   │      │
│                            │  │ n:new s:settings │      │
│                            │  └──────────────────┘      │
└────────────────────────────┴─────────────────────────────┘
```

### Top bar (`draw_topbar`)

Format: `♟ RChess v{VERSION}  Mv {n}  [{opening}]  [{eval}]  │ {mode}`

Status colors (top bar text):
- TEAL: normal play, active turn
- RED: check, checkmate, flag fall
- AMBER: stalemate, draw
- DIM: thinking/engine busy

Background: `bg1(th)`

### Sidebar panels detail

**PLAYERS** — rounded border, title `" PLAYERS "`:
- Row 1: `▶ ` + `♔ WHITE [YOU]` + `  +3` (material advantage) + `  5:00` (clock)
- Row 2: Captured by Black (white pieces taken): `♜♝♞♟♟...`
- Separator: `────` (DIMMER)
- Row 3: ` ` + `♚ BLACK [CPU]` + `  -3` + `  4:30`
- Row 4: Captured by White (black pieces taken): `♙♙♙♙...`
- Clock colors: TEAL (active), DIMMER (inactive), RED (≤30s or flagged)
- `[YOU]` = `pw(th)` color, `[CPU]`/`[AI]` = DIM color
- Active player has `▶` prefix; inactive has two-space prefix

**STATUS** — rounded border, title `" STATUS "`:
- Status strings:
  - `"▶ WHITE TO MOVE"` / `"▶ BLACK TO MOVE"` — TEAL
  - `"⚠ CHECK — WHITE TO MOVE"` / `"⚠ CHECK — BLACK TO MOVE"` — RED
  - `"♝♔ CHECKMATE — WHITE WINNER"` / `"♝♔ CHECKMATE — BLACK WINNER"` — checkmate gold
  - `"½-½ STALEMATE"` — AMBER
  - `"⏳ THINKING..."` — DIM
  - `"⛀⛁ FLAG — WHITE WINNER ⛀⛉"` / `"⛀⛁ FLAG — BLACK WINNER ⛀⛉"` — RED
- Line 2: `"Move 12"` + `"last: [W] e4"`
- Optional PNG/PGN save notice

**EVAL** (Standard mode only) — rounded border, title `" EVAL ~[engine]"` (~=busy, +=idle):
- Line 1: Label icon + delta: `"[B] Book"` / `"[+] Good"` / `"[?!] Inaccuracy"` / `"[?] Mistake"` / `"[??] Blunder"` + `"-1.5p"`
- Line 2: `"Best: e7e5  +0.8"`
- Colors: GREEN (Book/Good), AMBER (Inaccuracy), RED (Mistake/Blunder)

**ANALYSIS** (Analysis mode only) — rounded border, title `" ANALYSIS ~[engine]"`:
- Line 1: `"Move: Nd5"` (bold)
- Line 2: `"Label: [+]"` (colored icon)
- Line 3: `"Delta: -0.5"` (RED if >1.0p, AMBER if >0.5p, GREEN otherwise)
- Line 4: `"Eval: +1.3 → +0.8"`
- Line 5+: `"Best: Bc4 +1.5"` / `"Top: e7e5 +0.8  Nf3 +0.5  g3 +0.3"`
- Or: `"analysing..."` / `"make a move to analyse"`

**HISTORY** — rounded border, title `" HISTORY "`:
- If empty: `"— no moves yet —"` (DIMMER)
- Otherwise numbered pairs: ` 1. e4      e5`
- Current/last move highlighted:
  - White's move cell: white text on accent bg
  - Black's move cell: TEAL text on bg1(th) bg
- Move labels shown inline: `+` (Good), `?!` (Inaccuracy), `?` (Mistake), `??` (Blunder)

**KEYS** — rounded border, title `" KEYS "`:
- Line 1: `"↑↓←→/hjkl cursor   Enter/click move"` (CREAM)
- Line 2: `"E:PNG G:PGN T:(mode) d:draw u:undo r:replay"` (color-coded spans)
- Line 3: `"n:new s:settings q:menu type:notation"` (DIMMER)
- Mode indicator: `min` / `std` / `ana`

### Input bar (`draw_input`, `ui.rs:506-537`)
- Border: accent when active, RED on error, DIM idle
- Line 1: `" [e4] type move or navigate"` (CREAM/DIM)
- Line 2: `" arrows  letter=notation  E=PNG  F=FEN  d=draw  r=replay"` (DIMMER)

### Game Over Overlay (`draw_gameover`, `ui.rs:783-872`)

| Title | Border color | Body |
|-------|-------------|------|
| `FLAG FALL` | RED | `"X ran out of time."` / `"Y wins."` |
| `DRAW` | AMBER | `"Game drawn by agreement."` |
| `CHECKMATE` | `(210,175,60)` gold | `"Y wins!"` / `"X king is mated."` |
| `STALEMATE` | AMBER | `"Game drawn."` / `"No legal moves."` |

Overlay also shows: `"N moves"`, material advantage, total time, FEN (truncated to 45 chars).
Footer: `"n:new  r:replay  E:PNG  G:PGN  u:undo  q:menu"` (color-coded: ac/TEAL/GREEN/TEAL/DIM/DIM)

---

## 14. Replay Screen

```
┌────────────────────────────────────────────────────────────┐
│   REPLAY  Position 12/34                   ← teal top bar  │
├───┬──────────────────────────┬─────────────────────────────┤
│   │                          │  ┌─ POSITION ────────┐     │
│   │                          │  │ 12. Nd5             │     │
│   │     ┌─ REPLAY ───┐      │  │ Move 12 of 34      │     │
│   │     │            │      │  │ eval +0.8           │     │
│   │     │  (board)   │      │  └────────────────────┘     │
│   │     │            │      │  ┌─ REVIEW ───────────┐     │
│   │     └────────────┘      │  │ Move: Nd5           │     │
│   │                          │  │ Label: [+] Good     │     │
│   │                          │  │ Delta: -0.5         │     │
│   │                          │  │ Eval: +1.3 → +0.8  │     │
│   │                          │  │ Best: Bc4  +1.5    │     │
│   │                          │  │      ▁▃▅▇█ (bars)  │     │
│   │                          │  └────────────────────┘     │
│   │                          │  ┌─ HISTORY ────────┐      │
│   │                          │  │ 1. e4      e5    │      │
│   │                          │  │ 2. Nf3     Nc6   │      │
│   │                          │  └──────────────────┘      │
│   │                          │  ┌─ REPLAY KEYS ───┐      │
│   │                          │  │ ←/h prev  →/l nxt│      │
│   │                          │  │ 0 start  $ end   │      │
│   │                          │  │ E:PNG  q back   │      │
│   │                          │  └──────────────────┘      │
└───┴──────────────────────────┴─────────────────────────────┘
```

### Eval bar (4 chars wide, left of board)
- Top (Black better): bg `(20,20,20)`, fg `(200,200,200)`, shows `"-1.5"` eval
- Bottom (White better): bg `(230,230,210)`, fg `(30,30,30)`, shows `"+1.5"` eval
- Middle (equal/busy): bg `(120,120,80)`, fg `(240,220,40)`, shows `"="` or `"~"`

### Sparkline (in REVIEW panel, bottom)
- 5 Unicode block bars: `▁▃▅▇█`
- Green `(120,220,160)` = White better position
- Red `(200,70,70)` = Black better position
- Neutral `(110,130,110)` = equal position

---

## 15. Puzzle Screen

- **Loading**: centered 40×7 rounded border, `"Fetching from Lichess..."` (DIM)
- **Failed**: centered 66×19, RED border, error details, token instructions if applicable
- **Loaded**: Same layout as Game with top bar `"★ DAILY PUZZLE  Rating: X  Themes: Y  (status)"`
- Sidebar PUZZLE panel:
  - `"Puzzle #id"` (CREAM)
  - `"Rating: 1482"` (DIM)
  - `"You are White"` (pw color) / `"You are Black"` (pb_c color)
  - `"Status: Solved!"` (GREEN) / `"Wrong move!"` (RED) / `"Find the best move"` (AMBER)
  - Themes: `"fork, middlegame, ..."` (up to 4, DIMMER)

---

## 16. Online Screens

### OnlineLobby (`draw_online_lobby`, `ui.rs:1977-2036`)

```
┌────────────────────────────────────────────┐
│             ONLINE PLAY                     │
│                                            │
│   Account: username            ← GREEN (connected) / DIM (not)  │
│   Status: Connected to Lichess ← GREEN / DIM │
│                                            │
│   CHALLENGES:                   ← accent bold │
│    opponent (1800, 3+0, Std)   ← DIMMER    │
│                                            │
│   a  accept    d  decline      ← DIM       │
│                                            │
│   c  Challenge user             ← accent   │
│   Enter username: [my_opp___]  ← if challenge_mode active │
│                                            │
│   q  Main menu    s  Settings  ← DIM       │
└────────────────────────────────────────────┘
```

Connection happens on first keypress in lobby. Shows `"Connecting..."` (DIM) while `lichess.is_none()`.

### OnlineGame (`draw_online_game`, `ui.rs:2038-2055`)
- Renders `draw_game()` underneath
- Overlay opponent bar: `"♔ opponent (rating)"` in accent, top-right
- Status line: `app.online_status` in DIM at bottom
- Keybinding: `Esc/q` → OnlineLobby, `d` → draw offer, `x` → resign

---

## 17. FenInput Screen (`draw_fen_input`)

```
┌──────────────────────────────────────────────────────────┐
│                    ≡ START FROM FEN                       │
│                                                          │
│   rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b █     │
│                                                          │
│   ✗ error message (if any, RED)                          │
│                                                          │
│   Example:                                               │
│   rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -  │
│                                                          │
│   Enter to load    Esc to cancel                         │
└──────────────────────────────────────────────────────────┘
```

---

## 18. PgnImport Screen (`draw_pgn_import`)

```
┌──────────────────────────────────────────────────────────┐
│                      IMPORT PGN                           │
│                                                          │
│   Paste PGN or file path:                                │
│                                                          │
│   1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O  ←   │  ← last 6 lines
│                                                          │
│   312 chars (pgn mode)  /  Enter file path: /home/...    │
│                                                          │
│   Enter to load    Backspace to delete    Esc to cancel   │
└──────────────────────────────────────────────────────────┘
```

---

## 19. LoadGame Screen (`draw_load_game`)

```
┌──────────────────────────────────────────────────────────┐
│                     ⇪ SAVED GAMES                         │
│                                                          │
│   ▶ White vs Black  [10+0]  1-0   32 moves — 2024-01-15  │
│                                                          │
│     Alice vs Bob    [10+0]  0-1   45 moves — 2024-01-14  │
│                                                          │
│     ...                                                  │
│                                                          │
│   Enter to load as replay    Esc/q back                  │
└──────────────────────────────────────────────────────────┘
```

Each line shows: `"{white} vs {black} [{time}] {result}  {N} moves — {date}"`

---

## 20. Promo Overlay (`draw_promo`)

```
┌────────────────────────────────────────────────┐
│              PROMOTE PAWN                       │
│                                                │
│       ◀  ♕ Queen (default)  ▶                  │
│                                                │
│   ←→ navigate    Enter select    Esc cancel     │
└────────────────────────────────────────────────┘
```

- 4 pieces cycle: Queen → Rook → Bishop → Knight → Queen...
- Default selection: Queen
- Piece symbol shown in accent color on hover
- Navigation: `←`/`→` or `h`/`l`

---

## 21. DrawOffer Overlay (`draw_draw_offer`)

```
┌──────────────────────────────────────────┐
│           ½ DRAW OFFER ½                  │  ← AMBER bold
│                                          │
│   Your opponent offers a draw.           │
│                                          │
│   y / Y  — Accept                        │
│   n / N  — Decline                       │
└──────────────────────────────────────────┘
```

- Border: AMBER
- Title: `"½ DRAW OFFER ½"` (AMBER bold)
- Body: `"Your opponent offers a draw."` (or `"Offer draw to opponent?"`)
- Accept -> game ends as draw
- Decline -> return to game

---

## 22. Keybinding Reference

### Menu
| Key | Action |
|-----|--------|
| `↑` `k` | Cursor up |
| `↓` `j` | Cursor down |
| `Enter` `Space` | Select item |
| `s` | Open Settings |
| `q` `Esc` | Quit program |

### ColorPick
| Key | Action |
|-----|--------|
| `←` `h` `↑` `k` | Select White |
| `→` `l` `↓` `j` | Select Black |
| `w` | Pick White directly |
| `b` | Pick Black directly |
| `Enter` `Space` | Confirm |
| `Esc` `q` | Back to Menu |

### Settings
| Key | Action |
|-----|--------|
| `↑` `k` | Scroll up |
| `↓` `j` | Scroll down |
| `←` `h` | Cycle value backward |
| `→` `l` | Cycle value forward |
| `w` `s` | Save config to file |
| `r` | Reset to defaults |
| `Esc` `q` | Back to previous screen |
| Mouse scroll | Scroll settings list |

### Game (including OnlineGame, Puzzle via keyboard)
| Key | Action |
|-----|--------|
| `↑` `k` | Cursor up |
| `↓` `j` | Cursor down |
| `←` `h` | Cursor left |
| `→` `l` | Cursor right |
| `Enter` `Space` | Select piece / target square |
| `Esc` | Deselect / clear targets |
| `q` | Go to Menu (OnlineGame: OnlineLobby) |
| `n` | New game / restart |
| `s` | Open Settings |
| `u` | Undo (PvP: 1, CPU: 2, Easy CPU: 1) |
| `E` | Open PNG export preview |
| `F` | Copy FEN to clipboard |
| `G` | Export/save PGN file |
| `r` | Open Replay |
| `T` | Cycle UI mode (Min→Std→Ana) |
| `d` | Offer draw (PvP only, human turn) |
| Letters | Enter notation input mode |
| Mouse click | Select piece/target square |

### Notation Input Mode
| Key | Action |
|-----|--------|
| `Enter` | Execute typed move |
| `Backspace` | Delete last character |
| `Esc` | Cancel input |
| Arrow keys | Move cursor on board |

### OnlineGame extras
| Key | Action |
|-----|--------|
| `Esc` `q` | Back to OnlineLobby |
| `s` | Settings |
| `d` | Offer draw to opponent |
| `x` | Resign game |

### OnlineLobby
| Key | Action |
|-----|--------|
| `Esc` `q` | Back to Menu |
| `c` | Enter challenge mode (type username) |
| `a` | Accept first challenge in list |
| `d` | Decline first challenge |
| `s` | Open Settings |
| `↑` `/` `↓` | Navigate challenge list |

### Challenge Mode (inside OnlineLobby)
| Key | Action |
|-----|--------|
| Alphanumeric `-` `_` | Type username |
| `Backspace` | Delete character |
| `Enter` | Send challenge |
| `Esc` | Cancel challenge |

### DrawOffer
| Key | Action |
|-----|--------|
| `y` `Y` `Enter` | Accept draw |
| `n` `N` `Esc` | Decline, continue playing |

### Promo
| Key | Action |
|-----|--------|
| `←` `h` | Previous piece |
| `→` `l` | Next piece |
| `Enter` `Space` | Confirm promotion |
| `Esc` | Cancel promotion |

### Replay
| Key | Action |
|-----|--------|
| `←` `h` | Previous position |
| `→` `l` | Next position |
| `0` `Home` | Jump to start |
| `$` `End` | Jump to end |
| `E` | Export PNG preview |
| `G` | Save PGN |
| `q` `Esc` | Back to Game |

### FenInput
| Key | Action |
|-----|--------|
| Any char | Append to FEN buffer |
| `Backspace` | Delete character |
| `Enter` | Parse and load FEN |
| `Esc` | Back to Menu |

### PgnImport
| Key | Action |
|-----|--------|
| Any char | Append to PGN buffer |
| `Enter` | Parse and load (text or file path) |
| `Backspace` | Delete character |
| `Esc` | Back to Menu |

### Puzzle
| Key | Action |
|-----|--------|
| Same as Game | Board interaction |
| `Esc` `q` | Back to Menu |
| `n` | New puzzle (re-fetch) |

### LoadGame
| Key | Action |
|-----|--------|
| `↑` `k` | Scroll up |
| `↓` `j` | Scroll down |
| `Enter` `Space` | Load selected as replay |
| `Esc` `q` | Back to Menu |

### PngPreview / PgnSaved
| Key | Action |
|-----|--------|
| `y` `Y` `Enter` | Confirm / Save |
| `n` `N` `Esc` | Cancel |

---

## 23. UI Mode Comparison

| Feature | Minimal | Standard (default) | Analysis |
|---------|---------|-------------------|----------|
| PLAYERS panel | ✓ | ✓ | ✓ |
| STATUS panel | ✓ | ✓ | ✓ |
| EVAL panel (2-line compact) | — | ✓ | — |
| ANALYSIS panel (7-line full) | — | — | ✓ |
| HISTORY panel | ✓ | ✓ | ✓ |
| KEYS panel | ✓ | ✓ | ✓ |
| Top bar eval string | — | ✓ | ✓ |
| Total sidebar panels | 4 | 5 | 5 |

Toggle via `T` key in-game. Mode indicator shown at bottom of KEYS panel: `min` / `std` / `ana`.

---

## 24. Export Details

- **Directory**: `~/rchess_export/`
- **PNG**: Python/Pillow subprocess (`png_export.rs`)
- **PGN**: Written via `app.rs::export_pgn()`
- **Auto-save PNG** (`config.auto_save_png`): exports PNG on game end
- **Auto-save game** (`config.auto_save_game`): saves JSON to `~/.config/rchess/saves/`

---

## 25. Config Save/Load

- **Path**: `~/.config/rchess/rchess_tui.conf`
- **Load**: `Config::load()` at startup — reads file, falls back to `Config::default()`
- **Save**: `cfg.save()` — writes key=value format with comments
- **In-game**: Press `w` or `s` in Settings to save

Save file includes helpful comments documenting all available options.
