// src/book.rs — Built-in opening book
// Lines are stored as sequences of coordinate-notation moves.
// On each AI turn we check if current game matches a known prefix,
// then pick a book continuation weighted by frequency.

use crate::app::{parse_notation, HistEntry};
use crate::engine::{Board, Castle, Color, Mv};

/// Every element is a full opening line (coordinate notation, white/black alternating).
/// Lines are sourced from common ECO openings.
const LINES: &[&[&str]] = &[
    // ── Ruy López ────────────────────────────────────────────────────────────
    &["e2e4","e7e5","g1f3","b8c6","f1b5","a7a6","b5a4","g8f6","e1g1","f8e7","f1e1","b7b5","a4b3"],
    &["e2e4","e7e5","g1f3","b8c6","f1b5","g8f6","e1g1","f8c5","c2c3","e8g8"],
    &["e2e4","e7e5","g1f3","b8c6","f1b5","a7a6","b5c6","d7c6","d2d4","e5d4"],
    // ── Italian Game ─────────────────────────────────────────────────────────
    &["e2e4","e7e5","g1f3","b8c6","f1c4","f8c5","c2c3","g8f6","d2d4","e5d4","c3d4","c5b4","c1d2"],
    &["e2e4","e7e5","g1f3","b8c6","f1c4","g8f6","d2d4","e5d4","e1g1","f8c5"],
    // ── Scotch Game ──────────────────────────────────────────────────────────
    &["e2e4","e7e5","g1f3","b8c6","d2d4","e5d4","f3d4","g8f6","d4c6","b7c6","e4e5","d8e7","d1e2"],
    &["e2e4","e7e5","g1f3","b8c6","d2d4","e5d4","f3d4","f8c5","c1e3","d8f6"],
    // ── Sicilian Defence ─────────────────────────────────────────────────────
    &["e2e4","c7c5","g1f3","d7d6","d2d4","c5d4","f3d4","g8f6","b1c3","a7a6","c1e3","e7e5","d4b3"],
    &["e2e4","c7c5","g1f3","b8c6","d2d4","c5d4","f3d4","g7g6","c2c4","g8f6"],
    &["e2e4","c7c5","b1c3","b8c6","g2g3","g7g6","f1g2","f8g7","d2d3","d7d6"],
    &["e2e4","c7c5","g1f3","e7e6","d2d4","c5d4","f3d4","a7a6","f1d3","g8f6"],
    // ── French Defence ───────────────────────────────────────────────────────
    &["e2e4","e7e6","d2d4","d7d5","b1c3","g8f6","c1g5","f8e7","e4e5","f6d7","g5e7","d8e7"],
    &["e2e4","e7e6","d2d4","d7d5","e4e5","c7c5","c2c3","b8c6","g1f3","d8b6"],
    // ── Caro-Kann ────────────────────────────────────────────────────────────
    &["e2e4","c7c6","d2d4","d7d5","b1c3","d5e4","c3e4","g8f6","e4f6","e7f6","g1f3","f8d6"],
    &["e2e4","c7c6","d2d4","d7d5","e4e5","c8f5","g1f3","e7e6","f1e2","g8e7"],
    // ── Pirc / Modern ────────────────────────────────────────────────────────
    &["e2e4","d7d6","d2d4","g8f6","b1c3","g7g6","g1f3","f8g7","f1e2","e8g8","e1g1","c7c6"],
    // ── Queen's Gambit ───────────────────────────────────────────────────────
    &["d2d4","d7d5","c2c4","e7e6","b1c3","g8f6","c1g5","f8e7","e2e3","e8g8","g1f3","b7b6"],
    &["d2d4","d7d5","c2c4","c7c6","b1c3","g8f6","g1f3","e7e6","e2e3","b8d7","f1d3"],
    &["d2d4","d7d5","c2c4","d5c4","g1f3","g8f6","e2e3","e7e6","f1c4","c7c5"],
    // ── King's Indian Defence ────────────────────────────────────────────────
    &["d2d4","g8f6","c2c4","g7g6","b1c3","f8g7","e2e4","d7d6","g1f3","e8g8","f1e2","e7e5"],
    &["d2d4","g8f6","c2c4","g7g6","b1c3","f8g7","e2e4","d7d6","f2f3","e8g8","c1e3","b8c6"],
    // ── Nimzo-Indian ─────────────────────────────────────────────────────────
    &["d2d4","g8f6","c2c4","e7e6","b1c3","f8b4","e2e3","e8g8","f1d3","d7d5","g1f3","c7c5"],
    &["d2d4","g8f6","c2c4","e7e6","b1c3","f8b4","d1c2","d7d5","a2a3","b4c3","c2c3","b8c6"],
    // ── Queen's Indian ───────────────────────────────────────────────────────
    &["d2d4","g8f6","c2c4","e7e6","g1f3","b7b6","g2g3","c8b7","f1g2","f8e7","e1g1","e8g8"],
    // ── Grünfeld Defence ─────────────────────────────────────────────────────
    &["d2d4","g8f6","c2c4","g7g6","b1c3","d7d5","c4d5","f6d5","e2e4","d5c3","b2c3","f8g7"],
    // ── London System ────────────────────────────────────────────────────────
    &["d2d4","d7d5","g1f3","g8f6","c1f4","e7e6","e2e3","c7c5","c2c3","b8c6","b1d2","f8d6"],
    &["d2d4","g8f6","g1f3","d7d5","c1f4","e7e6","e2e3","c7c5","c2c3","b8c6"],
    // ── English Opening ──────────────────────────────────────────────────────
    &["c2c4","e7e5","b1c3","g8f6","g1f3","b8c6","g2g3","d7d5","c4d5","f6d5","f1g2"],
    &["c2c4","c7c5","g1f3","g8f6","b1c3","d7d5","c4d5","f6d5","e2e3","e7e6"],
    // ── Bird's Opening ───────────────────────────────────────────────────────
    &["f2f4","d7d5","g1f3","g8f6","e2e3","g7g6","b2b3","f8g7","c1b2","e8g8"],
    // ── Réti Opening ─────────────────────────────────────────────────────────
    &["g1f3","d7d5","g2g3","g8f6","f1g2","g7g6","e1g1","f8g7","d2d3","e8g8","b1d2"],
];

/// Return a book move for the current position, or None if out of book.
pub fn book_move(
    history:  &[HistEntry],
    board:    &Board,
    color:    Color,
    ep:       Option<(usize, usize)>,
    cast:     &Castle,
) -> Option<Mv> {
    // Build stripped history for matching (strip +/# suffixes)
    let hist: Vec<&str> = history.iter()
        .map(|h| {
            let n = h.notation.as_str();
            n.trim_end_matches('#').trim_end_matches('+')
        })
        .collect();
    let depth = hist.len();

    // Gather all lines that match current history and have a next move
    let mut candidates: Vec<&str> = LINES.iter()
        .filter(|line| {
            line.len() > depth
                && hist.iter().zip(line.iter()).all(|(h, l)| *h == *l)
        })
        .map(|line| line[depth])
        .collect();

    if candidates.is_empty() { return None; }

    // Deterministic but varied: pick using depth as seed
    candidates.sort_unstable();
    candidates.dedup();
    let pick = depth % candidates.len();

    parse_notation(board, color, ep, cast, candidates[pick]).ok()
}
