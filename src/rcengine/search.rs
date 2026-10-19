use crate::rcengine::board::Position;
use crate::rcengine::types::*;
use crate::rcengine::movegen;
use std::time::Instant;

// ── Piece-Square Tables (MG/EG tapered) ──────────────────────────────────

const PAWN_TABLE: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

const PAWN_TABLE_EG: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    40, 40, 40, 40, 40, 40, 40, 40,
    10, 10, 15, 20, 20, 15, 10, 10,
     5,  5, 10, 20, 20, 10,  5,  5,
     0,  0,  5, 15, 15,  5,  0,  0,
     0,  0,  0,  5,  5,  0,  0,  0,
     5,  5,  5,-10,-10,  5,  5,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

const KNIGHT_TABLE: [i16; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

const KNIGHT_TABLE_EG: [i16; 64] = [
    -60,-50,-40,-40,-40,-40,-50,-60,
    -50,-30,-10,-10,-10,-10,-30,-50,
    -40,-10,  5, 10, 10,  5,-10,-40,
    -40,-10, 10, 20, 20, 10,-10,-40,
    -40,-10, 10, 20, 20, 10,-10,-40,
    -40,-10,  5, 10, 10,  5,-10,-40,
    -50,-30,-10,-10,-10,-10,-30,-50,
    -60,-50,-40,-40,-40,-40,-50,-60,
];

const BISHOP_TABLE: [i16; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

const BISHOP_TABLE_EG: [i16; 64] = [
    -30,-15,-15,-15,-15,-15,-15,-30,
    -15,  5,  0,  0,  0,  0,  5,-15,
    -15,  0, 15, 15, 15, 15,  0,-15,
    -15, 10, 10, 15, 15, 10, 10,-15,
    -15,  0, 15, 15, 15, 15,  0,-15,
    -15, 15, 15, 15, 15, 15, 15,-15,
    -15, 10,  5,  5,  5,  5, 10,-15,
    -30,-15,-15,-15,-15,-15,-15,-30,
];

const ROOK_TABLE: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
];

const ROOK_TABLE_EG: [i16; 64] = [
     5,  5,  5, 10, 10,  5,  5,  5,
     0, 10, 15, 15, 15, 15, 10,  0,
    -5,  5, 10, 10, 10, 10,  5, -5,
    -5,  5, 10, 10, 10, 10,  5, -5,
    -5,  5, 10, 10, 10, 10,  5, -5,
     0,  5, 10, 10, 10, 10,  5,  0,
     5, 10, 15, 15, 15, 15, 10,  5,
     5,  5,  5, 10, 10,  5,  5,  5,
];

const QUEEN_TABLE: [i16; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

const QUEEN_TABLE_EG: [i16; 64] = [
    -30,-15,-15, -5, -5,-15,-15,-30,
    -15,  0,  0,  5,  5,  0,  0,-15,
    -15,  0,  5, 10, 10,  5,  0,-15,
     -5,  0,  5, 10, 10,  5,  0, -5,
     -5,  0,  5, 10, 10,  5,  0, -5,
    -15,  0,  5, 10, 10,  5,  0,-15,
    -15,  0,  5,  5,  5,  5,  0,-15,
    -30,-15,-15, -5, -5,-15,-15,-30,
];

const KING_TABLE: [i16; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
];

const KING_TABLE_EG: [i16; 64] = [
    -60,-40,-30,-20,-20,-30,-40,-60,
    -40,-20,  0, 10, 10,  0,-20,-40,
    -30,  0, 15, 25, 25, 15,  0,-30,
    -20, 10, 25, 40, 40, 25, 10,-20,
    -20, 10, 25, 40, 40, 25, 10,-20,
    -30,  0, 15, 25, 25, 15,  0,-30,
    -40,-20,  0, 10, 10,  0,-20,-40,
    -60,-40,-30,-20,-20,-30,-40,-60,
];

fn pst_mg(kind: PieceKind) -> &'static [i16; 64] {
    match kind {
        PieceKind::Pawn => &PAWN_TABLE, PieceKind::Knight => &KNIGHT_TABLE,
        PieceKind::Bishop => &BISHOP_TABLE, PieceKind::Rook => &ROOK_TABLE,
        PieceKind::Queen => &QUEEN_TABLE, PieceKind::King => &KING_TABLE,
    }
}

fn pst_eg(kind: PieceKind) -> &'static [i16; 64] {
    match kind {
        PieceKind::Pawn => &PAWN_TABLE_EG, PieceKind::Knight => &KNIGHT_TABLE_EG,
        PieceKind::Bishop => &BISHOP_TABLE_EG, PieceKind::Rook => &ROOK_TABLE_EG,
        PieceKind::Queen => &QUEEN_TABLE_EG, PieceKind::King => &KING_TABLE_EG,
    }
}

// ── Material values ──────────────────────────────────────────────────────

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

// Phase: non-pawn material remaining → [0, 256] (0=endgame, 256=opening)
fn game_phase(pos: &Position) -> i32 {
    let mut total = 0i32;
    for c in &[Color::White, Color::Black] {
        let base = c.idx() * 6;
        total += (pos.bb[base + 1].count_ones() as i32) * 320;
        total += (pos.bb[base + 2].count_ones() as i32) * 330;
        total += (pos.bb[base + 3].count_ones() as i32) * 500;
        total += (pos.bb[base + 4].count_ones() as i32) * 900;
    }
    let phase = total / 25; // 6400 max → 256
    if phase > 256 { 256 } else { phase }
}

// ── Evaluation ───────────────────────────────────────────────────────────

pub fn evaluate(pos: &Position) -> Score {
    let mut score = 0i32;
    let phase = game_phase(pos);

    for c in &[Color::White, Color::Black] {
        let base = c.idx() * 6;
        let sign = if *c == Color::White { 1 } else { -1 };

        for (k, &val) in PIECE_VALUES.iter().enumerate() {
            let mut bb = pos.bb[base + k];
            let kind = match k { 0 => PieceKind::Pawn, 1 => PieceKind::Knight, 2 => PieceKind::Bishop,
                                 3 => PieceKind::Rook, 4 => PieceKind::Queen, 5 => PieceKind::King, _ => unreachable!() };
            let mg_table = pst_mg(kind);
            let eg_table = pst_eg(kind);

            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let mg_val = if *c == Color::White { mg_table[sq] } else { mg_table[63 - sq] };
                let eg_val = if *c == Color::White { eg_table[sq] } else { eg_table[63 - sq] };
                let pst_val = (mg_val as i32 * phase + eg_val as i32 * (256 - phase)) / 256;
                score += sign * (val + pst_val);
                bb &= bb - 1;
            }
        }

        // Mobility bonus (simple count of attacks)
        let occ = pos.occupancy();
        let us_occ = if *c == Color::White { pos.us() } else { pos.them() };
        let mut mob = 0i32;
        // Knights
        let mut bb_k = pos.bb[base + 1];
        while bb_k != 0 {
            let sq = bb_k.trailing_zeros() as Square;
            mob += (movegen::tables().knight_att[sq as usize] & !us_occ).count_ones() as i32;
            bb_k &= bb_k - 1;
        }
        // Bishops
        let mut bb_b = pos.bb[base + 2];
        while bb_b != 0 {
            let sq = bb_b.trailing_zeros() as Square;
            mob += (movegen::bishop_attacks(sq, occ) & !us_occ).count_ones() as i32;
            bb_b &= bb_b - 1;
        }
        // Rooks
        let mut bb_r = pos.bb[base + 3];
        while bb_r != 0 {
            let sq = bb_r.trailing_zeros() as Square;
            mob += (movegen::rook_attacks(sq, occ) & !us_occ).count_ones() as i32;
            bb_r &= bb_r - 1;
        }
        // Queens
        let mut bb_q = pos.bb[base + 4];
        while bb_q != 0 {
            let sq = bb_q.trailing_zeros() as Square;
            mob += (movegen::queen_attacks(sq, occ) & !us_occ).count_ones() as i32;
            bb_q &= bb_q - 1;
        }

        score += sign * mob;

        // Bishop pair bonus
        if pos.bb[base + 2].count_ones() >= 2 {
            score += sign * 30;
        }

        // Rook on open/semi-open file
        let mut bb_r2 = pos.bb[base + 3];
        while bb_r2 != 0 {
            let sq = bb_r2.trailing_zeros() as usize;
            let file_mask = 0x0101010101010101u64 << (sq & 7);
            let our_pawns = pos.bb[base] & file_mask;
            if our_pawns == 0 {
                let their_base = (base + 6) % 12;
                let their_pawns = pos.bb[their_base] & file_mask;
                score += sign * if their_pawns == 0 { 20 } else { 10 };
            }
            bb_r2 &= bb_r2 - 1;
        }

        // Rook on 7th rank bonus
        let rook_7th = if *c == Color::White { 0x00FF000000000000u64 } else { 0x000000000000FF00u64 };
        let mut bb_r3 = pos.bb[base + 3];
        while bb_r3 != 0 {
            let sq = bb_r3.trailing_zeros() as usize;
            if (1u64 << sq) & rook_7th != 0 {
                score += sign * 20;
            }
            bb_r3 &= bb_r3 - 1;
        }

        // Pawn structure
        let our_pawns = pos.bb[base];
        let their_base = (base + 6) % 12;
        let their_pawns = pos.bb[their_base];
        let mut bb_p = our_pawns;
        while bb_p != 0 {
            let sq = bb_p.trailing_zeros() as usize;
            let file = sq & 7;
            let rank = sq >> 3;

            // Isolated pawn: no friendly pawns on adjacent files
            let adj_mask = if file == 0 {
                0x0202020202020202u64
            } else if file == 7 {
                0x0101010101010101u64
            } else {
                (0x0101010101010101u64 << (file - 1)) | (0x0101010101010101u64 << (file + 1))
            };
            if our_pawns & adj_mask == 0 {
                score += sign * -15;
            }

            // Passed pawn: no enemy pawns on same or adjacent files in forward direction
            if rank < 7 && rank > 0 {
                let this_file_mask = 0x0101010101010101u64 << file;
                let forward_mask = if *c == Color::White {
                    (!0u64) << ((rank + 1) * 8)
                } else {
                    (!0u64) >> ((8 - rank) * 8)
                };
                let file_adj_all = adj_mask | this_file_mask;
                let enemy_forward = their_pawns & file_adj_all & forward_mask;
                if enemy_forward == 0 {
                    score += sign * 30;
                    // Connected passed pawn bonus
                    if our_pawns & adj_mask != 0 {
                        score += sign * 10;
                    }
                }
            }

            bb_p &= bb_p - 1;
        }
        // Doubled pawns: count pawns per file, penalty for stacks
        for f in 0..8 {
            let fm = 0x0101010101010101u64 << f;
            let cnt = (our_pawns & fm).count_ones();
            if cnt > 1 {
                score += sign * -(cnt as i32 - 1) * 15;
            }
        }
    }

    // Tempo bonus: side to move has a small inherent advantage
    score += if pos.side == Color::White { 20 } else { -20 };

    score
}

// ── Transposition Table ──────────────────────────────────────────────────

const TT_SIZE: usize = 1 << 20; // ~1M entries
const TT_MASK: u64 = (TT_SIZE - 1) as u64;

#[derive(Clone, Copy)]
struct TTEntry {
    key: u64,
    depth: u8,
    flag: u8, // 0=exact, 1=lowerbound, 2=upperbound
    score: i16,
    best_move: u16,
}

const TT_FLAG_EXACT: u8 = 0;
const TT_FLAG_LOWER: u8 = 1;
const TT_FLAG_UPPER: u8 = 2;

struct TranspositionTable {
    entries: Vec<TTEntry>,
}

impl TranspositionTable {
    fn new() -> Self {
        TranspositionTable { entries: vec![TTEntry { key: 0, depth: 0, flag: 0, score: 0, best_move: 0 }; TT_SIZE] }
    }

    fn probe(&self, key: u64) -> Option<&TTEntry> {
        let entry = &self.entries[(key & TT_MASK) as usize];
        if entry.key == key { Some(entry) } else { None }
    }

    fn store(&mut self, key: u64, depth: u8, flag: u8, score: i16, best_move: u16) {
        let idx = (key & TT_MASK) as usize;
        let entry = &mut self.entries[idx];
        if depth >= entry.depth || entry.key != key {
            *entry = TTEntry { key, depth, flag, score, best_move };
        }
    }

    fn clear(&mut self) {
        for e in &mut self.entries { *e = TTEntry { key: 0, depth: 0, flag: 0, score: 0, best_move: 0 }; }
    }
}

// ── Search ───────────────────────────────────────────────────────────────

pub struct Search {
    pub output_info: bool,
    tt: TranspositionTable,
    killers: [[u16; 2]; 64],
    history: [[[i32; 64]; 64]; 2],  // [side][from][to] quiet move bonus
    nodes: u64,
    start_time: Instant,
    time_limit: u128,
    stop: bool,
    pos: Position,
    ply: u32,
}

impl Search {
    pub fn new() -> Self {
        Search {
            output_info: false,
            tt: TranspositionTable::new(),
            killers: [[0; 2]; 64],
            history: [[[0; 64]; 64]; 2],
            nodes: 0,
            start_time: Instant::now(),
            time_limit: u128::MAX,
            stop: false,
            pos: Position::new(),
            ply: 0,
        }
    }

    /// Find best move with iterative deepening
    pub fn go(&mut self, pos: &Position, limits: &Limits) -> Move {
        self.pos = pos.clone();
        self.nodes = 0;
        self.stop = false;
        self.start_time = Instant::now();
        self.ply = 0;
        self.killers = [[0; 2]; 64];
        self.history = [[[0; 64]; 64]; 2];

        // Time management
        if limits.movetime > 0 {
            self.time_limit = limits.movetime as u128;
        } else if limits.infinite {
            self.time_limit = u128::MAX;
        } else if limits.wtime > 0 || limits.btime > 0 {
            let our_time = if pos.side == Color::White { limits.wtime } else { limits.btime };
            let inc = if pos.side == Color::White { limits.winc } else { limits.binc };
            let movestogo = if limits.movestogo > 0 { limits.movestogo } else { 30 };
            let time_per_move = (our_time as u128) / movestogo as u128;
            let inc_bonus = (inc as u128) * 3 / 4;
            self.time_limit = time_per_move + inc_bonus;
            self.time_limit = self.time_limit.min((our_time as u128) * 3 / 4);
        } else {
            self.time_limit = u128::MAX;
        }

        let max_depth = if limits.depth > 0 { limits.depth } else { 64 };
        let moves = movegen::generate_moves(&self.pos);
        let mut best = if moves.len > 0 { moves[0] } else { Move::new(E2, E4) };

        for depth in 1..=max_depth {
            if self.stop { break; }
            let score = self.aspiration_window(depth);

            if self.stop { break; }

            // Get best move from TT
            if let Some(entry) = self.tt.probe(pos.key) {
                best = Move(entry.best_move);
            }

            // Print UCI info line if requested
            if self.output_info {
                let score_str = if score.abs() > MATE - 100 {
                    let mate_ply = (MATE - score.abs()) / 2;
                    if score > 0 { format!("mate {}", mate_ply) } else { format!("mate -{}", mate_ply) }
                } else {
                    format!("cp {}", score)
                };
                let pv = self.pv(5);
                let elapsed = self.elapsed_ms().max(1) as u64;
                let nps = (self.nodes * 1000) / elapsed;
                println!("info depth {} score {} nodes {} nps {} pv {}",
                    depth, score_str, self.nodes, nps, pv);
            }

            // Check if we should stop
            if self.elapsed_ms() > self.time_limit.saturating_sub(50) {
                break;
            }
        }

        // Final info line with bestmove found
        if self.output_info {
            println!("bestmove {}", best);
        }

        best
    }

    /// Find best move with score (for analysis)
    pub fn go_with_score(&mut self, pos: &Position, limits: &Limits) -> (Move, Score) {
        let best = self.go(pos, limits);
        // Get score from TT
        let score = self.tt.probe(pos.key)
            .map(|e| tt_score(e.score, 0))
            .unwrap_or(DRAW);
        (best, score)
    }

    fn aspiration_window(&mut self, depth: u32) -> Score {
        let mut alpha = -INF;
        let mut beta = INF;
        let delta = 25;

        // Try to use TT score for window
        if let Some(entry) = self.tt.probe(self.pos.key) {
            let score = entry.score as Score;
            alpha = score - delta;
            beta = score + delta;
        }

        loop {
            let score = self.alpha_beta(depth, 0, alpha, beta);

            if score <= alpha {
                alpha -= delta * 4;
                continue;
            }
            if score >= beta {
                beta += delta * 4;
                continue;
            }
            return score;
        }
    }

    /// True when remaining material is so low that null-move is unsafe (zugzwang risk)
    fn is_zugzwang(&self) -> bool {
        let us = self.pos.side;
        let us_base = us.idx() * 6;
        let them_base = (us_base + 6) % 12;
        let total_minor = (self.pos.bb[us_base + 1] | self.pos.bb[us_base + 2]
                        | self.pos.bb[them_base + 1] | self.pos.bb[them_base + 2]).count_ones();
        let total_heavy = (self.pos.bb[us_base + 3] | self.pos.bb[us_base + 4]
                        | self.pos.bb[them_base + 3] | self.pos.bb[them_base + 4]).count_ones();
        total_minor <= 2 && total_heavy == 0
    }

    fn alpha_beta(&mut self, depth: u32, ply: u32, mut alpha: Score, mut beta: Score) -> Score {
        self.nodes += 1;

        // Check time
        if self.nodes & 0x3FFF == 0 && self.elapsed_ms() > self.time_limit {
            self.stop = true;
            return alpha;
        }

        // Check extension: extend search 1 ply when in check
        let in_check = self.pos.is_check();
        let ext = if in_check { 1 } else { 0 };

        // Quiescence search at leaf
        if depth == 0 && !in_check {
            return self.quiescence(alpha, beta);
        }
        let eff_depth = depth + ext;

        let mate_lo = -(MATE as Score) + ply as Score;
        if alpha < mate_lo { alpha = mate_lo; }
        let mate_hi = (MATE as Score) - (ply + 1) as Score;
        if beta > mate_hi { beta = mate_hi; }
        if alpha >= beta { return alpha; }

        // TT probe
        let tt_move = self.tt.probe(self.pos.key).and_then(|e| {
            if e.depth as u32 >= eff_depth {
                let score = tt_score(e.score, ply);
                match e.flag {
                    TT_FLAG_EXACT => return Some(Move(e.best_move)),
                    TT_FLAG_LOWER if score >= beta => return Some(Move(e.best_move)),
                    TT_FLAG_UPPER if score <= alpha => return Some(Move(e.best_move)),
                    _ => {}
                }
            }
            None
        });

        // Null-move pruning (skip in zugzwang-prone positions)
        if eff_depth >= 3 && !in_check && !self.is_zugzwang() {
            let r = if eff_depth >= 6 { 3 } else { 2 };
            self.pos.make_null_move();
            self.ply += 1;
            let null_score = -self.alpha_beta(eff_depth - 1 - r, ply + 1, -beta, -beta + 1);
            self.ply -= 1;
            self.pos.undo_null_move();
            if self.stop { return alpha; }
            if null_score >= beta {
                return beta;
            }
        }

        // Internal Iterative Deepening: if no TT move and depth >= 3, do a shallow search
        let iid_move = if tt_move.is_none() && eff_depth >= 3 && !in_check {
            let iid_depth = eff_depth / 2;
            self.alpha_beta(iid_depth, ply, alpha, beta);
            self.tt.probe(self.pos.key).map(|e| Move(e.best_move))
        } else {
            None
        };

        // Generate moves
        let moves = movegen::generate_moves(&self.pos);
        if moves.is_empty() {
            return if self.pos.is_check() { -MATE + ply as Score } else { DRAW };
        }

        // Move ordering: place TT move first, then killers
        let mut ordered = [Move(0); 256];
        let mut ordered_len = 0;
        let mut has_tt = false;
        if let Some(ttm) = tt_move.or(iid_move) {
            ordered[0] = ttm; ordered_len = 1; has_tt = true;
        }
        // Add non-TT, non-killer moves
        let k_idx = (ply as usize).min(63);
        let k0 = Move(self.killers[k_idx][0]);
        let k1 = Move(self.killers[k_idx][1]);
        for i in 0..moves.len {
            if has_tt && moves[i] == ordered[0] { continue; }
            if moves[i] == k0 || moves[i] == k1 { continue; }
            ordered[ordered_len] = moves[i];
            ordered_len += 1;
        }
        // Place killers at front (after TT)
        if k0.0 != 0 {
            let idx = if has_tt { 1 } else { 0 };
            for j in (idx..ordered_len).rev() { ordered[j+1] = ordered[j]; }
            ordered[idx] = k0; ordered_len += 1;
        }
        if k1.0 != 0 && k1 != k0 {
            let cnt = if has_tt { 1 } else { 0 } + if k0.0 != 0 { 1 } else { 0 };
            for j in (cnt..ordered_len).rev() { ordered[j+1] = ordered[j]; }
            ordered[cnt] = k1; ordered_len += 1;
        }
        // Score remaining moves (MVV-LVA + history)
        let skip = if has_tt { 1 } else { 0 } + if k0.0 != 0 { 1 } else { 0 } + if k1.0 != 0 { 1 } else { 0 };
        if ordered_len > skip {
            ordered[skip..ordered_len].sort_by_cached_key(|&mv| {
                let base = movegen::score_move(&self.pos, mv);
                let hist = self.history[self.pos.side.idx()][mv.from() as usize][mv.to() as usize];
                -(base + hist / 100)
            });
        }

        let mut best_score = -INF;
        let mut best_move = ordered[0];
        let mut alpha_raised = false;

        // Static evaluation for futility pruning at low depths
        let static_eval = evaluate(&self.pos);

        for (i, &mv) in ordered.iter().enumerate().take(ordered_len) {
            // Futility pruning: at low depths, skip quiet moves that can't possibly beat alpha
            let is_capture = self.pos.piece_on(mv.to()).is_some();
            let is_quiet = !is_capture && mv.promo().is_none() && mv.to() != self.pos.ep_sq && !mv.is_castle();
            if is_quiet && eff_depth <= 3 && !in_check && alpha > -MATE + 200 {
                let margin = (eff_depth as Score) * 200;
                if static_eval + margin < alpha {
                    continue;
                }
            }

            // SEE pruning: skip losing captures at low depth
            if eff_depth <= 2 && !in_check && is_capture {
                let see_val = movegen::see(&self.pos, mv);
                if see_val < -300 {
                    continue;
                }
            }

            // Make move — if illegal (king in check), skip
            if !self.pos.make_move(mv) { continue; }
            self.ply += 1;
            let score = if i == 0 {
                -self.alpha_beta(eff_depth - 1, ply + 1, -beta, -alpha)
            } else if is_quiet && eff_depth >= 3 && i >= 4 && !self.pos.is_check() {
                // Late Move Reduction
                let r = 1u32 + ((i as u32).saturating_sub(4)).min(12) / 4;
                let r = r.min(eff_depth.saturating_sub(1));
                let new_depth = eff_depth - r;
                let mut s = -self.alpha_beta(new_depth, ply + 1, -alpha - 1, -alpha);
                if s > alpha {
                    s = -self.alpha_beta(eff_depth - 1, ply + 1, -beta, -alpha);
                }
                s
            } else {
                -self.alpha_beta(eff_depth - 1, ply + 1, -beta, -alpha)
            };

            self.ply -= 1;
            self.pos.undo_move();

            if self.stop { return alpha; }

            if score > best_score {
                best_score = score;
                best_move = mv;
            }
            if score > alpha {
                alpha = score;
                alpha_raised = true;
            }
            if alpha >= beta {
                // Store quiet move as killer
                if self.pos.piece_on(mv.to()).is_none() && mv.promo().is_none() {
                    let p = (ply as usize).min(63);
                    if self.killers[p][0] != mv.0 {
                        self.killers[p][1] = self.killers[p][0];
                        self.killers[p][0] = mv.0;
                    }
                    // Update history heuristic (cap to prevent dominance)
                    let bonus = eff_depth as i32 * eff_depth as i32;
                    let hist = &mut self.history[self.pos.side.idx()][mv.from() as usize][mv.to() as usize];
                    *hist = (*hist + bonus).clamp(-16384, 16384);
                }
                break;
            }
        }

        // TT store
        let flag = if !alpha_raised { TT_FLAG_UPPER }
                   else if alpha >= beta { TT_FLAG_LOWER }
                   else { TT_FLAG_EXACT };
        self.tt.store(self.pos.key, eff_depth as u8, flag, tt_score_rev(alpha, ply), best_move.0);

        alpha
    }

    fn quiescence(&mut self, mut alpha: Score, beta: Score) -> Score {
        self.nodes += 1;

        if self.nodes & 0x7FFF == 0 && self.elapsed_ms() > self.time_limit {
            self.stop = true;
            return alpha;
        }

        let stand_pat = evaluate(&self.pos);
        if stand_pat > alpha { alpha = stand_pat; }
        if alpha >= beta { return alpha; }

        // Generate all moves, keep captures + non-capture checks
        let all_moves = movegen::generate_moves(&self.pos);
        let mut ordered = [Move(0); 256];
        let mut len = 0u32;
        for i in 0..all_moves.len {
            let mv = all_moves[i];
            // Captures and promotions always included
            let is_capture = self.pos.piece_on(mv.to()).is_some();
            let is_promo = mv.promo().is_some();
            if is_capture || is_promo {
                ordered[len as usize] = mv;
                len += 1;
                continue;
            }
            // Non-capture checks: try the move to see if it gives check
            if self.pos.make_move(mv) {
                let gives_check = self.pos.is_check();
                self.pos.undo_move();
                if gives_check {
                    ordered[len as usize] = mv;
                    len += 1;
                }
            }
        }
        if len == 0 { return alpha; }

        // Sort by SEE score (already handles captures, promos, checks)
        ordered[..len as usize].sort_by_key(|&mv| -movegen::score_move(&self.pos, mv));

        for &mv in ordered.iter().take(len as usize) {
            // Delta pruning for captures
            if let Some(cap) = self.pos.piece_on(mv.to()) {
                let cap_val = PIECE_VALUES[cap.0.idx()];
                if stand_pat + cap_val + 200 < alpha {
                    continue;
                }
            }

            if !self.pos.make_move(mv) { continue; }
            self.ply += 1;

            let score = -self.quiescence(-beta, -alpha);

            self.ply -= 1;
            self.pos.undo_move();

            if self.stop { return alpha; }

            if score > alpha { alpha = score; }
            if alpha >= beta { break; }
        }

        alpha
    }

    fn elapsed_ms(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }

    /// Extract principal variation from TT (up to `max_len` moves)
    pub fn pv(&self, max_len: usize) -> String {
        let mut pos = self.pos.clone();
        let mut seen = std::collections::HashSet::new();
        let mut moves = Vec::new();
        seen.insert(pos.key);
        for _ in 0..max_len {
            if let Some(entry) = self.tt.probe(pos.key) {
                let mv = Move(entry.best_move);
                if mv.0 == 0 { break; }
                if let Some(piece) = pos.piece_on(mv.from()) {
                    if piece.0 == PieceKind::King && mv.from() == pos.king_sq(pos.side) {
                        // check legality by attempting make_move
                        let mut c = pos.clone();
                        if c.make_move(mv) {
                            moves.push(format!("{}", mv));
                            if !seen.insert(c.key) { break; }
                            pos = c;
                            continue;
                        }
                    }
                }
                // simple check: move must be pseudo-legal
                let mut c = pos.clone();
                if c.make_move(mv) {
                    moves.push(format!("{}", mv));
                    if !seen.insert(c.key) { break; }
                    pos = c;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if moves.is_empty() { "none".to_string() } else { moves.join(" ") }
    }
}

fn tt_score(score: i16, ply: u32) -> Score {
    if (score as Score) > MATE - 256 { score as Score - ply as Score }
    else if (score as Score) < -(MATE as Score) + 256 { score as Score + ply as Score }
    else { score as Score }
}

fn tt_score_rev(score: Score, ply: u32) -> i16 {
    if score > MATE - 256 { (score + ply as Score) as i16 }
    else if score < -(MATE as Score) + 256 { (score - ply as Score) as i16 }
    else { score as i16 }
}
