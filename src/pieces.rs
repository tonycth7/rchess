// src/pieces.rs — Scalable chess piece renderer
//
// Three tiers based on cell width:
//   CELL_W < 6  → bold Unicode symbols   (font-dependent)
//   CELL_W 6–10 → half-block art          (█▀▄, no font dep.)
//   CELL_W > 10 → detailed half-block art (larger source bitmap)
//
// Half-block art uses ↗ characters to get 2× vertical resolution:
//   █ = both halves filled, ▀ = top half, ▄ = bottom half, ' ' = empty

use crate::config::{Theme, PieceStyle};
use crate::engine::{Color as PC, Kind, Piece};

// ── Piece bitmap definitions ──────────────────────────────────────────────────
// Each piece is a 6×8 grid of bits (6 wide × 8 half-rows).
// Then scaled to cell_w × cell_h via nearest-neighbor.
// Bitmap encoded as u64: row 0 = bits 0..5, row 1 = bits 6..11, etc. (LSB = left)

macro_rules! bp { ($($r:expr),+) => {{
    let mut v: Vec<u64> = Vec::new();
    $(let row: u64 = 0 $(| (if $r & (1 << (5 - i)) != 0 { 1 << (i + v.len() * 6) } else { 0 } ) for i in 0..6);+; v.push(row);)+
    let mut result: u64 = 0;
    for (i, &r) in v.iter().enumerate() { result |= r << (i * 6); }
    result
}};}

// Simpler: store as array of row bits (6 bits per row, packed into u64, LSB=left pixel)
// Row 0 = bits 0-5, Row 1 = bits 6-11, etc.
// Using `bp!(0bNNNNNN, 0bNNNNNN, ...)` macro.

// Unused — piece bitmaps are now defined as `*_BITS` byte arrays below.
const fn piece_bits(_: Kind) -> u64 { 0 }

// I'll use a simpler approach: just store rows as u8 bytes, 8 per piece.
// Each u8 has bits 0-5 used (LSB = leftmost pixel in row).

const fn pr(bits: u64, row: usize) -> u8 {
    ((bits >> (row * 6)) & 0x3F) as u8
}

// 6 wide × 8 tall bitmaps for each piece kind.
// Bits: 1 = filled, 0 = empty. Row 0 = top.
// Stored as [u8; 8] where each byte = 6 bits (bits 0-5 = cols 0-5, LSB=left).

const fn make_rows(row0: u8, row1: u8, row2: u8, row3: u8, row4: u8, row5: u8, row6: u8, row7: u8) -> [u8; 8] {
    [row0, row1, row2, row3, row4, row5, row6, row7]
}

// Each piece: 6 columns × 8 rows of sub-pixels.
// LSB = leftmost column.

// Pawn: round head, tapered body
const PAWN_BITS: [u8; 8] = make_rows(
    0b001100, // ..##..
    0b001100, // ..##..
    0b011110, // .####.
    0b011110, // .####.
    0b111111, // ######
    0b011110, // .####.
    0b001100, // ..##..
    0b000000, // ......
);

// Knight: horse head facing right
const KNIGHT_BITS: [u8; 8] = make_rows(
    0b001111, // ..####
    0b011110, // .####.
    0b001100, // ..##..
    0b011110, // .####.
    0b111100, // ####..
    0b011100, // .###..
    0b001000, // ..#...
    0b000000, // ......
);

// Bishop: tall mitre / diamond
const BISHOP_BITS: [u8; 8] = make_rows(
    0b000100, // ...#..
    0b001110, // ..###.
    0b001110, // ..###.
    0b011110, // .####.
    0b111111, // ######
    0b011110, // .####.
    0b010010, // .#..#.
    0b000000, // ......
);

// Rook: castle with battlements
const ROOK_BITS: [u8; 8] = make_rows(
    0b101010, // #.#.#.
    0b111111, // ######
    0b011110, // .####.
    0b011110, // .####.
    0b011110, // .####.
    0b011110, // .####.
    0b011110, // .####.
    0b000000, // ......
);

// Queen: crown with 5 points, tall
const QUEEN_BITS: [u8; 8] = make_rows(
    0b010010, // .#..#.
    0b101010, // #.#.#.
    0b111111, // ######
    0b111111, // ######
    0b111111, // ######
    0b011110, // .####.
    0b001100, // ..##..
    0b000000, // ......
);

// King: cross on top, solid
const KING_BITS: [u8; 8] = make_rows(
    0b001100, // ..##..
    0b001100, // ..##..
    0b011110, // .####.
    0b111111, // ######
    0b111111, // ######
    0b011110, // .####.
    0b001100, // ..##..
    0b000000, // ......
);

fn piece_bitmap(kind: Kind) -> &'static [u8; 8] {
    match kind {
        Kind::P => &PAWN_BITS,
        Kind::N => &KNIGHT_BITS,
        Kind::B => &BISHOP_BITS,
        Kind::R => &ROOK_BITS,
        Kind::Q => &QUEEN_BITS,
        Kind::K => &KING_BITS,
    }
}

/// Larger 8×8 bitmaps for detailed mode (CELL_W > 10)
const PAWN_8: [u8; 8] = make_rows(0b00010000, 0b00111000, 0b00111000, 0b01111100, 0b11111110, 0b01111100, 0b00010000, 0b00000000);
const KNIGHT_8: [u8; 8] = make_rows(0b00011110, 0b00111100, 0b00011000, 0b00111100, 0b01111000, 0b00111000, 0b00010000, 0b00000000);
const BISHOP_8: [u8; 8] = make_rows(0b00001000, 0b00011100, 0b00011100, 0b00111110, 0b01111111, 0b00111110, 0b00100100, 0b00000000);
const ROOK_8: [u8; 8] = make_rows(0b10101010, 0b11111111, 0b00111100, 0b00111100, 0b00111100, 0b00111100, 0b00111100, 0b00000000);
const QUEEN_8: [u8; 8] = make_rows(0b00100100, 0b01010101, 0b11111111, 0b11111111, 0b01111110, 0b00111100, 0b00011000, 0b00000000);
const KING_8: [u8; 8] = make_rows(0b00011000, 0b00011000, 0b00111100, 0b01111110, 0b11111111, 0b00111100, 0b00011000, 0b00000000);

fn piece_bitmap_8(kind: Kind) -> &'static [u8; 8] {
    match kind {
        Kind::P => &PAWN_8,
        Kind::N => &KNIGHT_8,
        Kind::B => &BISHOP_8,
        Kind::R => &ROOK_8,
        Kind::Q => &QUEEN_8,
        Kind::K => &KING_8,
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Determine which render tier to use based on cell width.
pub fn render_style(cell_w: usize) -> &'static str {
    if cell_w < 6 { "unicode" }
    else if cell_w <= 10 { "halfblock" }
    else { "detail" }
}

/// Render a single mid-line piece symbol (for non-blocks styles like Unicode/Letters).
/// Returns centered string of width `cell_w`.
pub fn render_sym(p: Piece, style: &PieceStyle, cell_w: usize) -> String {
    let sym = match style {
        PieceStyle::Unicode => p.sym().to_string(),
        PieceStyle::Letters => {
            let letter = match p.k { Kind::K=>'K',Kind::Q=>'Q',Kind::R=>'R',Kind::B=>'B',Kind::N=>'N',Kind::P=>'P' };
            if p.c == PC::White { letter.to_string() } else { letter.to_lowercase().to_string() }
        }
        PieceStyle::FatLetters => {
            let letter = match p.k { Kind::K=>'K',Kind::Q=>'Q',Kind::R=>'R',Kind::B=>'B',Kind::N=>'N',Kind::P=>'P' };
            if p.c == PC::White { format!("[{}]", letter) } else { format!("[{}]", letter.to_lowercase()) }
        }
        PieceStyle::Blocks => {
            // For Blocks mode at small cells, fall back to unicode
            if cell_w < 6 {
                p.sym().to_string()
            } else {
                return render_halfblock(p, cell_w, 1); // single line
            }
        }
    };
    center_str(&sym, cell_w)
}

/// Render a half-block piece using █▀▄ characters.
/// Returns CELL_H lines, each being a centered string of width CELL_W.
pub fn render_block_lines(p: Piece, cell_w: usize, cell_h: usize) -> Vec<String> {
    let sub_h = cell_h * 2; // vertical sub-pixel resolution
    let src = if cell_w > 10 { piece_bitmap_8(p.k) } else { piece_bitmap(p.k) };
    let src_w = if cell_w > 10 { 8 } else { 6 };
    let src_h = 8;

    let mut lines = Vec::with_capacity(cell_h);

    for line_y in 0..cell_h {
        let mut line_str = String::with_capacity(cell_w);
        for x in 0..cell_w {
            // Map output cell (x, line_y) to source sub-pixels
            let sx = x * src_w / cell_w;
            let sy_top    = (line_y * 2 + 0) * src_h / sub_h;
            let sy_bottom = (line_y * 2 + 1) * src_h / sub_h;

            let top    = sy_top < src_h    && sx < src_w    && (src[sy_top] >> (src_w - 1 - sx)) & 1 != 0;
            let bottom = sy_bottom < src_h && sx < src_w && (src[sy_bottom] >> (src_w - 1 - sx)) & 1 != 0;

            let ch = match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            };
            line_str.push(ch);
        }
        lines.push(line_str);
    }
    lines
}

/// Render a single centered line of half-block piece (for draw_board classic mode).
fn render_halfblock(p: Piece, cell_w: usize, _line_idx: usize) -> String {
    let src = piece_bitmap(p.k);
    let src_w = 6;
    let src_h = 8;
    let mid_row = 4; // middle of 8 source rows

    let mut out = String::with_capacity(cell_w);
    for x in 0..cell_w {
        let sx = x * src_w / cell_w;
        let filled = src[mid_row.min(src_h - 1)] >> (src_w - 1 - sx) & 1 != 0;
        out.push(if filled { '█' } else { ' ' });
    }
    out
}

/// Format a string centered in a field of width `w`.
pub fn center_str(s: &str, w: usize) -> String {
    let len = s.len();
    if len >= w { return s[..w.min(len)].to_string(); }
    let pad = w - len;
    let left = pad / 2;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(pad - left))
}
