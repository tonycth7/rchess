// src/pieces.rs — ASCII art pieces
//
// Two pattern sets:
//   Outline (no fill)   → used for white pieces
//   Filled (`#` fill)   → used for black pieces
//
// All patterns are 5-wide × 3-tall.
// Top/middle rows have 1-space padding on each side (3-char content centered).
// Base row is 5 chars with no padding (extends 1 char on each side = balanced).

use crate::engine::{Kind, Piece};

const W: usize = 5;
const H: usize = 3;

// ── Outline (white pieces) ────────────────────────────────────────────────
// Classic ASCII — K/Q/R use simple forms matching the B/N/P style.
const WK: &[&str] = &[" \\+/ ", " ) ( ", "/___\\"];
const WQ: &[&str] = &[" \\^/ ", " ) ( ", "/___\\"];
const WB: &[&str] = &[" (V) ", " ) ( ", "/___\\"];
const WN: &[&str] = &[" (\"\\ ", " ) ' ", "/___\\"];
const WR: &[&str] = &[" [-] ", " | | ", "/___\\"];
const WP: &[&str] = &["  _  ", " ( ) ", "/___\\"];

// ── Filled (black pieces) ─────────────────────────────────────────────────
// Shade blocks (░▒▓█) for fill — medium shade for body, base.
const BK: &[&str] = &[" \\+/ ", " )▓( ", "/▓▓▓\\"];
const BQ: &[&str] = &[" \\^/ ", " )▓( ", "/▓▓▓\\"];
const BB: &[&str] = &[" (V) ", " )▓( ", "/▓▓▓\\"];
const BN: &[&str] = &[" (\"\\ ", " )▓' ", "/▓▓▓\\"];
const BR: &[&str] = &[" [-] ", " |▓| ", "/▓▓▓\\"];
const BP: &[&str] = &["  _  ", " (▓) ", "/▓▓▓\\"];

fn src(p: Piece) -> &'static [&'static str] {
    let filled = p.c == crate::engine::Color::Black;
    match p.k {
        Kind::K => if filled { BK } else { WK },
        Kind::Q => if filled { BQ } else { WQ },
        Kind::B => if filled { BB } else { WB },
        Kind::N => if filled { BN } else { WN },
        Kind::R => if filled { BR } else { WR },
        Kind::P => if filled { BP } else { WP },
    }
}

/// Render a piece as ASCII art. Returns CELL_H lines, each CELL_W wide.
/// When the cell is larger than 5×3, the pattern is centered.
/// When smaller, it is scaled down.
pub fn render_star_lines(p: Piece, cell_w: usize, cell_h: usize) -> Vec<String> {
    let s = src(p);
    let mut out = Vec::with_capacity(cell_h);
    if cell_w >= W && cell_h >= H {
        let pad_top = (cell_h - H) / 2;
        let pad_left = (cell_w - W) / 2;
        for y in 0..cell_h {
            let mut row = String::with_capacity(cell_w);
            if y < pad_top || y >= pad_top + H {
                for _ in 0..cell_w { row.push(' '); }
            } else {
                let mut chars = s[y - pad_top].chars();
                for _ in 0..pad_left { row.push(' '); }
                for _ in 0..W { row.push(chars.next().unwrap_or(' ')); }
                for _ in 0..(cell_w - pad_left - W) { row.push(' '); }
            }
            out.push(row);
        }
    } else {
        for y in 0..cell_h {
            let sy = y * H / cell_h;
            let chars: Vec<char> = s[sy.min(H - 1)].chars().collect();
            let mut row = String::with_capacity(cell_w);
            for x in 0..cell_w {
                let sx = x * W / cell_w;
                row.push(if sx < W { chars[sx] } else { ' ' });
            }
            out.push(row);
        }
    }
    out
}
