// src/ai.rs — Minimax + alpha-beta + TT + iterative deepening + quiescence + improved eval
use std::sync::{Mutex, OnceLock};
use crate::app::HistEntry;
use crate::book::book_move;
use crate::engine::*;
use crate::tt::{TranspositionTable, TTFlag};

// ── Piece values ──────────────────────────────────────────────────────────────
fn pv(k: Kind) -> i32 { match k { Kind::P=>100, Kind::N=>320, Kind::B=>330, Kind::R=>500, Kind::Q=>900, Kind::K=>20000 } }
fn ph(k: Kind) -> i32 { match k { Kind::P=>0, Kind::N=>1, Kind::B=>1, Kind::R=>2, Kind::Q=>4, Kind::K=>0 } }

// ── PST middlegame ────────────────────────────────────────────────────────────
const PST_P: [[i32;8];8] = [[0,0,0,0,0,0,0,0],[50,50,50,50,50,50,50,50],[10,10,20,30,30,20,10,10],[5,5,10,25,25,10,5,5],[0,0,0,20,20,0,0,0],[5,-5,-10,0,0,-10,-5,5],[5,10,10,-20,-20,10,10,5],[0,0,0,0,0,0,0,0]];
const PST_N: [[i32;8];8] = [[-50,-40,-30,-30,-30,-30,-40,-50],[-40,-20,0,0,0,0,-20,-40],[-30,0,10,15,15,10,0,-30],[-30,5,15,20,20,15,5,-30],[-30,0,15,20,20,15,0,-30],[-30,5,10,15,15,10,5,-30],[-40,-20,0,5,5,0,-20,-40],[-50,-40,-30,-30,-30,-30,-40,-50]];
const PST_B: [[i32;8];8] = [[-20,-10,-10,-10,-10,-10,-10,-20],[-10,0,0,0,0,0,0,-10],[-10,0,5,10,10,5,0,-10],[-10,5,5,10,10,5,5,-10],[-10,0,10,10,10,10,0,-10],[-10,10,10,10,10,10,10,-10],[-10,5,0,0,0,0,5,-10],[-20,-10,-10,-10,-10,-10,-10,-20]];
const PST_R: [[i32;8];8] = [[0,0,0,0,0,0,0,0],[5,10,10,10,10,10,10,5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[-5,0,0,0,0,0,0,-5],[0,0,0,5,5,0,0,0]];
const PST_Q: [[i32;8];8] = [[-20,-10,-10,-5,-5,-10,-10,-20],[-10,0,0,0,0,0,0,-10],[-10,0,5,5,5,5,0,-10],[-5,0,5,5,5,5,0,-5],[0,0,5,5,5,5,0,-5],[-10,5,5,5,5,5,0,-10],[-10,0,5,0,0,0,0,-10],[-20,-10,-10,-5,-5,-10,-10,-20]];
const PST_K: [[i32;8];8] = [[-30,-40,-40,-50,-50,-40,-40,-30],[-30,-40,-40,-50,-50,-40,-40,-30],[-30,-40,-40,-50,-50,-40,-40,-30],[-30,-40,-40,-50,-50,-40,-40,-30],[-20,-30,-30,-40,-40,-30,-30,-20],[-10,-20,-20,-20,-20,-20,-20,-10],[20,20,0,0,0,0,20,20],[20,30,10,0,0,10,30,20]];

// ── PST endgame ───────────────────────────────────────────────────────────────
const EG_P: [[i32;8];8] = [[0,0,0,0,0,0,0,0],[80,80,80,80,80,80,80,80],[60,60,60,60,60,60,60,60],[40,40,40,40,40,40,40,40],[20,20,20,20,20,20,20,20],[10,10,10,10,10,10,10,10],[10,10,10,10,10,10,10,10],[0,0,0,0,0,0,0,0]];
const EG_N: [[i32;8];8] = [[-40,-30,-20,-20,-20,-20,-30,-40],[-30,-10,5,5,5,5,-10,-30],[-20,5,15,20,20,15,5,-20],[-20,5,15,20,20,15,5,-20],[-20,5,15,20,20,15,5,-20],[-20,5,15,20,20,15,5,-20],[-30,-10,5,5,5,5,-10,-30],[-40,-30,-20,-20,-20,-20,-30,-40]];
const EG_B: [[i32;8];8] = [[-20,-10,-10,-10,-10,-10,-10,-20],[-10,0,5,5,5,5,0,-10],[-10,5,10,10,10,10,5,-10],[-10,5,10,10,10,10,5,-10],[-10,5,10,10,10,10,5,-10],[-10,5,10,10,10,10,5,-10],[-10,0,5,5,5,5,0,-10],[-20,-10,-10,-10,-10,-10,-10,-20]];
const EG_R: [[i32;8];8] = [[5,5,5,5,5,5,5,5],[10,10,10,10,10,10,10,10],[5,5,5,5,5,5,5,5],[0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0],[-5,-5,-5,-5,-5,-5,-5,-5]];
const EG_Q: [[i32;8];8] = [[-10,-5,-5,-5,-5,-5,-5,-10],[-5,0,0,0,0,0,0,-5],[-5,0,5,5,5,5,0,-5],[-5,0,5,5,5,5,0,-5],[-5,0,5,5,5,5,0,-5],[-5,0,5,5,5,5,0,-5],[-5,0,0,0,0,0,0,-5],[-10,-5,-5,-5,-5,-5,-5,-10]];
const EG_K: [[i32;8];8] = [[-20,-10,-10,-10,-10,-10,-10,-20],[-10,5,5,10,10,5,5,-10],[-10,5,10,15,15,10,5,-10],[-10,10,15,20,20,15,10,-10],[-10,10,15,20,20,15,10,-10],[-10,5,10,15,15,10,5,-10],[-10,5,5,10,10,5,5,-10],[-20,-10,-10,-10,-10,-10,-10,-20]];

fn pst_mg(k: Kind, r: usize, c: usize) -> i32 {
    match k { Kind::P=>PST_P[r][c], Kind::N=>PST_N[r][c], Kind::B=>PST_B[r][c], Kind::R=>PST_R[r][c], Kind::Q=>PST_Q[r][c], Kind::K=>PST_K[r][c] }
}
fn pst_eg(k: Kind, r: usize, c: usize) -> i32 {
    match k { Kind::P=>EG_P[r][c], Kind::N=>EG_N[r][c], Kind::B=>EG_B[r][c], Kind::R=>EG_R[r][c], Kind::Q=>EG_Q[r][c], Kind::K=>EG_K[r][c] }
}

// ── Global TT ─────────────────────────────────────────────────────────────────
fn tt() -> &'static Mutex<TranspositionTable> {
    static TT: OnceLock<Mutex<TranspositionTable>> = OnceLock::new();
    TT.get_or_init(|| Mutex::new(TranspositionTable::new()))
}

// ── Search context (per-thread) ───────────────────────────────────────────────
struct SearchCtx {
    killers: Vec<[Option<Mv>;2]>,
    history: [[i32;64];12],
}
impl SearchCtx {
    fn new() -> Self { Self { killers: vec![[None;2];64], history: [[0i32;64];12] } }
}

// ── Improved evaluation ───────────────────────────────────────────────────────
pub fn evaluate(b: &Board) -> i32 {
    let mut mg = 0i32; let mut eg = 0i32; let mut phase = 0i32;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = b[r][c] {
                let pr = if p.c == Color::White { r } else { 7 - r };
                let pc = pv(p.k);
                if p.c == Color::White { mg += pc + pst_mg(p.k,pr,c); eg += pc + pst_eg(p.k,pr,c); }
                else { mg -= pc + pst_mg(p.k,pr,c); eg -= pc + pst_eg(p.k,pr,c); }
                phase += ph(p.k);
            }
        }
    }
    if phase > 24 { phase = 24; }
    // Pawn structure
    let (wd,wi,wp) = pawn_analysis(b, Color::White);
    let (bd,bi,bp) = pawn_analysis(b, Color::Black);
    mg += (bd - wd) * 10 + (bi - wi) * 15 + (wp - bp) * 25;
    // King safety
    mg += king_safety(b, Color::White) - king_safety(b, Color::Black);
    // Mobility
    mg += (mobility(b,Color::White) as i32 - mobility(b,Color::Black) as i32) * 2;
    (mg * phase + eg * (24 - phase)) / 24
}

fn pawn_analysis(b: &Board, color: Color) -> (i32,i32,i32) {
    let mut files = [0i32;8]; let mut hi = [0i32;8];
    let opp = color.opp();
    for r in 0..8 { for c in 0..8 {
        if let Some(p) = b[r][c] {
            if p.c == color && p.k == Kind::P {
                files[c] += 1; let pr = if color == Color::White { r as i32 } else { 7 - r as i32 };
                if pr > hi[c] { hi[c] = pr; }
            }
        }
    }}
    let mut double = 0i32; let mut iso = 0i32; let mut passed = 0i32;
    for c in 0..8 {
        if files[c] > 1 { double += files[c] - 1; }
        let l = c > 0 && files[c-1] > 0; let r = c < 7 && files[c+1] > 0;
        if files[c] > 0 && !l && !r { iso += files[c]; }
    }
    passed = 0;
    for r in 0..8 { for c in 0..8 {
        if let Some(p) = b[r][c] {
            if p.c == color && p.k == Kind::P {
                let mut is_passed = true;
                let (min_r,max_r) = if color == Color::White { (0, r) } else { (r+1, 7) };
                for rr in min_r..=max_r {
                    for cc in c.saturating_sub(1)..=7.min(c+1) {
                        if let Some(ep) = b[rr][cc] { if ep.c == opp && ep.k == Kind::P { is_passed = false; } }
                    }
                }
                if is_passed { passed += 1; }
            }
        }
    }}
    (double, iso, passed)
}

fn king_safety(b: &Board, color: Color) -> i32 {
    let ksq = match king_sq(b, color) { Some(s) => s, None => return 0 };
    let dir: i32 = if color == Color::White { -1 } else { 1 };
    let mut sc = 0i32;
    // Pawn shield (two ranks in front)
    for rank in 1..=2 {
        let fr = ksq.0 as i32 + dir * rank;
        if fr >= 0 && fr < 8 {
            for dc in -1..=1 {
                let fc = ksq.1 as i32 + dc;
                if fc >= 0 && fc < 8 {
                    match b[fr as usize][fc as usize] { Some(p) if p.c == color && p.k == Kind::P => sc += (14 - rank * 4), _ => {} }
                }
            }
        }
    }
    // Open files near king penalty
    for dc in -2i32..=2 {
        let fc = ksq.1 as i32 + dc;
        if fc >= 0 && fc < 8 {
            let mut has_pawn = false;
            for r in 0..8 { if let Some(p) = b[r][fc as usize] { if p.c == color && p.k == Kind::P { has_pawn = true; break; } } }
            if !has_pawn { sc -= 8i32.saturating_sub(dc.abs() * 2).max(2); }
        }
    }
    sc
}

fn mobility(b: &Board, color: Color) -> usize {
    let nc = Castle { wk:false, wq:false, bk:false, bq:false };
    let mut n = 0usize;
    for r in 0..8 { for c in 0..8 {
        if let Some(p) = b[r][c] { if p.c == color { n += pseudo(b, r, c, None, &nc, false).len(); } }
    }}
    n
}

// ── Move ordering ─────────────────────────────────────────────────────────────
fn order_moves(b: &Board, moves: &[Mv], color: Color, ply: usize, tt_move: Option<Mv>, ctx: &SearchCtx) -> Vec<(Mv,i32)> {
    let mut sc: Vec<(Mv,i32)> = moves.iter().map(|mv| {
        let mut s = 0i32;
        if tt_move == Some(*mv) { s += 10_000_000; }
        if mv.ep { s += 1_000_000 + 100; }
        else if let Some(t) = b[mv.to.0][mv.to.1] {
            let a = b[mv.fr.0][mv.fr.1].unwrap();
            s += pv(t.k) * 10 - pv(a.k) + 1_000_000;
        }
        let k = ctx.killers[ply.clamp(0,63)];
        if k[0] == Some(*mv) { s += 500_000; } else if k[1] == Some(*mv) { s += 250_000; }
        let pi = piece_index(color, b[mv.fr.0][mv.fr.1].unwrap().k);
        s += ctx.history[pi][mv.to.0 * 8 + mv.to.1];
        (*mv, s)
    }).collect();
    sc.sort_by(|a,b| b.1.cmp(&a.1));
    sc
}

// ── Quiescence search ─────────────────────────────────────────────────────────
fn quiesce(b: &Board, mut alpha: i32, mut beta: i32, is_max: bool, ep: Option<(usize,usize)>, cast: &Castle, ctx: &mut SearchCtx) -> i32 {
    let sp = evaluate(b);
    if is_max {
        if sp >= beta { return beta; }
        if sp > alpha { alpha = sp; }
    } else {
        if sp <= alpha { return alpha; }
        if sp < beta { beta = sp; }
    }
    let color = if is_max { Color::White } else { Color::Black };
    let moves = legal(b, color, ep, cast);
    let mut caps: Vec<(&Mv, i32)> = moves.iter().filter(|mv| mv.ep || b[mv.to.0][mv.to.1].is_some() || mv.promo.is_some())
        .map(|mv| {
            let s = if mv.ep { 100 } else if let Some(t) = b[mv.to.0][mv.to.1] {
                let a = b[mv.fr.0][mv.fr.1].unwrap(); pv(t.k) * 10 - pv(a.k)
            } else { 0 };
            (mv, s)
        }).collect();
    caps.sort_by(|a,b| b.1.cmp(&a.1));
    for (mv,_) in caps {
        let (nb,ne,nc) = apply(b, mv, ep, cast);
        let sc = quiesce(&nb, alpha, beta, !is_max, ne, &nc, ctx);
        if is_max {
            if sc > alpha { alpha = sc; }
            if alpha >= beta { return beta; }
        } else {
            if sc < beta { beta = sc; }
            if beta <= alpha { return alpha; }
        }
    }
    if is_max { alpha } else { beta }
}

// ── Alpha-beta with TT, killers, history ──────────────────────────────────────
fn alpha_beta(b: &Board, depth: u8, mut alpha: i32, mut beta: i32, is_max: bool, ep: Option<(usize,usize)>, cast: &Castle, ply: u8, ctx: &mut SearchCtx, lock: &mut TranspositionTable) -> i32 {
    let hash = zobrist_hash(b, if is_max { Color::White } else { Color::Black }, ep, cast);
    if let Some((fl,td,ts,bm)) = lock.probe(hash) {
        if td >= depth {
            match fl {
                TTFlag::Exact => return ts,
                TTFlag::Alpha => if ts <= alpha { return ts; }
                TTFlag::Beta  => if ts >= beta  { return ts; }
            }
        }
    }
    if depth == 0 { return quiesce(b, alpha, beta, is_max, ep, cast, ctx); }
    let color = if is_max { Color::White } else { Color::Black };
    let moves = legal(b, color, ep, cast);
    if moves.is_empty() {
        return if in_check(b, color) { if is_max { -99000 + ply as i32 } else { 99000 - ply as i32 } } else { 0 };
    }
    let tt_move = lock.probe(hash).and_then(|(_,_,_,bm)| bm);
    let ordered = order_moves(b, &moves, color, ply as usize, tt_move, ctx);
    let old_a = alpha;
    let mut best = if is_max { i32::MIN } else { i32::MAX };
    let mut best_mv = None;
    for (mv,_) in &ordered {
        let (nb,ne,nc) = apply(b, mv, ep, cast);
        let sc = alpha_beta(&nb, depth-1, alpha, beta, !is_max, ne, &nc, ply+1, ctx, lock);
        if is_max {
            if sc > best { best = sc; best_mv = Some(*mv); }
            if best > alpha { alpha = best; }
        } else {
            if sc < best { best = sc; best_mv = Some(*mv); }
            if best < beta { beta = best; }
        }
        if alpha >= beta {
            // Store killer if it's a quiet move
            if b[mv.to.0][mv.to.1].is_none() && !mv.ep {
                let p = ply as usize;
                let k = ctx.killers[p][0];
                ctx.killers[p][1] = k; ctx.killers[p][0] = Some(*mv);
                // History update
                let pi = piece_index(color, b[mv.fr.0][mv.fr.1].unwrap().k);
                ctx.history[pi][mv.to.0 * 8 + mv.to.1] += (depth as i32) * (depth as i32);
            }
            break;
        }
    }
    let flag = if alpha >= beta { TTFlag::Beta } else if best > old_a { TTFlag::Exact } else { TTFlag::Alpha };
    lock.store(hash, depth, best, flag, best_mv);
    best
}

// ── Iterative deepening root search ───────────────────────────────────────────
fn search_root(b: &Board, color: Color, ep: Option<(usize,usize)>, cast: &Castle, max_depth: u8, ctx: &mut SearchCtx, lock: &mut TranspositionTable) -> (i32, Option<Mv>) {
    let moves = legal(b, color, ep, cast);
    if moves.is_empty() { return (evaluate(b), None); }
    let is_max = color == Color::White;
    let mut best_mv = moves[0];
    let mut best_sc = if is_max { i32::MIN } else { i32::MAX };
    for d in 1..=max_depth {
        best_sc = if is_max { i32::MIN } else { i32::MAX };
        let mut alpha = i32::MIN + 1; let mut beta = i32::MAX - 1;
        for mv in &moves {
            let (nb,ne,nc) = apply(b, mv, ep, cast);
            let sc = alpha_beta(&nb, d-1, alpha, beta, !is_max, ne, &nc, 1, ctx, lock);
            if is_max {
                if sc > best_sc { best_sc = sc; best_mv = *mv; }
                if sc > alpha { alpha = sc; }
            } else {
                if sc < best_sc { best_sc = sc; best_mv = *mv; }
                if sc < beta { beta = sc; }
            }
            if alpha >= beta { break; }
        }
        if (is_max && best_sc > 98000) || (!is_max && best_sc < -98000) { break; }
    }
    (best_sc, Some(best_mv))
}

// ── Public API ────────────────────────────────────────────────────────────────
pub fn best_mv(b: &Board, color: Color, ep: Option<(usize,usize)>, cast: &Castle, depth: u8, history: &[HistEntry]) -> Option<Mv> {
    if let Some(mv) = book_move(history, b, color, ep, cast) { return Some(mv); }
    let moves = legal(b, color, ep, cast);
    if moves.is_empty() { return None; }
    let mut ctx = SearchCtx::new();
    let mut lock = tt().lock().unwrap();
    lock.clear();
    drop(lock); // Don't hold lock during search — re-acquire per call
    let mut lock = tt().lock().unwrap();
    let (_, mv) = search_root(b, color, ep, cast, depth, &mut ctx, &mut lock);
    mv
}

pub fn best_mv_with_score(b: &Board, color: Color, ep: Option<(usize,usize)>, cast: &Castle, depth: u8) -> (Option<Mv>, i32) {
    let moves = legal(b, color, ep, cast);
    if moves.is_empty() { return (None, evaluate(b)); }
    let mut ctx = SearchCtx::new();
    let mut lock = tt().lock().unwrap();
    lock.clear();
    let (sc, mv) = search_root(b, color, ep, cast, depth, &mut ctx, &mut lock);
    (mv, sc)
}

pub fn top_n_moves(b: &Board, color: Color, ep: Option<(usize,usize)>, cast: &Castle, depth: u8, n: usize) -> Vec<(Mv, i32)> {
    let moves = legal(b, color, ep, cast);
    if moves.is_empty() { return vec![]; }
    let mut ctx = SearchCtx::new();
    let mut lock = tt().lock().unwrap();
    lock.clear();
    let is_max = color == Color::White;
    let mut scored: Vec<(Mv,i32)> = moves.iter().map(|mv| {
        let (nb,ne,nc) = apply(b, mv, ep, cast);
        let sc = alpha_beta(&nb, depth, i32::MIN+1, i32::MAX-1, !is_max, ne, &nc, 1, &mut ctx, &mut lock);
        (*mv, sc)
    }).collect();
    if is_max { scored.sort_by(|a,b| b.1.cmp(&a.1)); } else { scored.sort_by(|a,b| a.1.cmp(&b.1)); }
    scored.truncate(n);
    scored
}
