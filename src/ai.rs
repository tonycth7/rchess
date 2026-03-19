// src/ai.rs — Minimax + alpha-beta + PST evaluation + opening book
use crate::app::HistEntry;
use crate::book::book_move;
use crate::engine::{Board, Castle, Color, Kind, Mv, in_check, legal, apply};

fn pv(k: Kind) -> i32 {
    match k { Kind::P=>100, Kind::N=>320, Kind::B=>330, Kind::R=>500, Kind::Q=>900, Kind::K=>20000 }
}

const PST_P: [[i32;8];8] = [[0,0,0,0,0,0,0,0],[50,50,50,50,50,50,50,50],[10,10,20,30,30,20,10,10],[5,5,10,25,25,10,5,5],[0,0,0,20,20,0,0,0],[5,-5,-10,0,0,-10,-5,5],[5,10,10,-20,-20,10,10,5],[0,0,0,0,0,0,0,0]];
const PST_N: [[i32;8];8] = [[-50,-40,-30,-30,-30,-30,-40,-50],[-40,-20,0,0,0,0,-20,-40],[-30,0,10,15,15,10,0,-30],[-30,5,15,20,20,15,5,-30],[-30,0,15,20,20,15,0,-30],[-30,5,10,15,15,10,5,-30],[-40,-20,0,5,5,0,-20,-40],[-50,-40,-30,-30,-30,-30,-40,-50]];
const PST_B: [[i32;8];8] = [[-20,-10,-10,-10,-10,-10,-10,-20],[-10,0,0,0,0,0,0,-10],[-10,0,5,10,10,5,0,-10],[-10,5,5,10,10,5,5,-10],[-10,0,10,10,10,10,0,-10],[-10,10,10,10,10,10,10,-10],[-10,5,0,0,0,0,5,-10],[-20,-10,-10,-10,-10,-10,-10,-20]];
const PST_R: [[i32;8];8] = [[0,0,0,0,0,0,0,0],[5,10,10,10,10,10,10,5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[0,0,0,5,5,0,0,0]];
const PST_Q: [[i32;8];8] = [[-20,-10,-10,-5,-5,-10,-10,-20],[-10,0,0,0,0,0,0,-10],[-10,0,5,5,5,5,0,-10],[-5,0,5,5,5,5,0,-5],[0,0,5,5,5,5,0,-5],[-10,5,5,5,5,5,0,-10],[-10,0,5,0,0,0,0,-10],[-20,-10,-10,-5,-5,-10,-10,-20]];
const PST_K: [[i32;8];8] = [[-30,-40,-40,-50,-50,-40,-40,-30],[-30,-40,-40,-50,-50,-40,-40,-30],[-30,-40,-40,-50,-50,-40,-40,-30],[-30,-40,-40,-50,-50,-40,-40,-30],[-20,-30,-30,-40,-40,-30,-30,-20],[-10,-20,-20,-20,-20,-20,-20,-10],[20,20,0,0,0,0,20,20],[20,30,10,0,0,10,30,20]];

fn pst(k: Kind, r: usize, c: usize) -> i32 {
    match k { Kind::P=>PST_P[r][c], Kind::N=>PST_N[r][c], Kind::B=>PST_B[r][c],
              Kind::R=>PST_R[r][c], Kind::Q=>PST_Q[r][c], Kind::K=>PST_K[r][c] }
}

pub fn evaluate(b: &Board) -> i32 {
    let mut s = 0i32;
    for r in 0..8 { for c in 0..8 {
        if let Some(p) = b[r][c] {
            let pr = if p.c == Color::White { r } else { 7-r };
            let v  = pv(p.k) + pst(p.k, pr, c);
            if p.c == Color::White { s += v; } else { s -= v; }
        }
    }}
    s
}

pub fn minimax(b: &Board, depth: u8, mut alpha: i32, mut beta: i32,
               is_max: bool, ep: Option<(usize,usize)>, cast: &Castle) -> i32 {
    let color  = if is_max { Color::White } else { Color::Black };
    let moves  = legal(b, color, ep, cast);
    if depth == 0 || moves.is_empty() {
        if moves.is_empty() {
            return if in_check(b, color) { if is_max { -99_000 } else { 99_000 } } else { 0 };
        }
        return evaluate(b);
    }
    if is_max {
        let mut best = i32::MIN;
        for mv in &moves {
            let (nb,ne,nc) = apply(b, mv, ep, cast);
            let v = minimax(&nb, depth-1, alpha, beta, false, ne, &nc);
            if v > best { best = v; }
            if best > alpha { alpha = best; }
            if beta <= alpha { break; }
        }
        best
    } else {
        let mut best = i32::MAX;
        for mv in &moves {
            let (nb,ne,nc) = apply(b, mv, ep, cast);
            let v = minimax(&nb, depth-1, alpha, beta, true, ne, &nc);
            if v < best { best = v; }
            if best < beta { beta = best; }
            if beta <= alpha { break; }
        }
        best
    }
}

pub fn best_mv(
    b:       &Board,
    color:   Color,
    ep:      Option<(usize,usize)>,
    cast:    &Castle,
    depth:   u8,
    history: &[HistEntry],
) -> Option<Mv> {
    // Try opening book first
    if let Some(mv) = book_move(history, b, color, ep, cast) {
        return Some(mv);
    }

    let moves = legal(b, color, ep, cast);
    if moves.is_empty() { return None; }

    let is_max   = color == Color::White;
    let mut bval = if is_max { i32::MIN } else { i32::MAX };
    let mut best = None;

    for mv in &moves {
        let (nb, ne, nc) = apply(b, mv, ep, cast);
        let v = minimax(&nb, depth-1, i32::MIN, i32::MAX, !is_max, ne, &nc);
        if (is_max && v > bval) || (!is_max && v < bval) { bval = v; best = Some(*mv); }
    }
    best
}
