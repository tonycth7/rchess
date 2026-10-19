use crate::rcengine::types::*;
use crate::rcengine::zobrist::zobrist;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Board state snapshot for undoing moves
#[derive(Clone)]
pub struct StateInfo {
    pub mv: Move,
    pub castling: u8,
    pub ep_sq: Square,
    pub halfmove: u16,
    pub captured: Option<Piece>,
    pub key: u64,
}

/// Bitboard-based chess position
pub struct Position {
    /// 12 bitboards: [W_Pawn, W_Knight, W_Bishop, W_Rook, W_Queen, W_King,
    ///                B_Pawn, B_Knight, B_Bishop, B_Rook, B_Queen, B_King]
    pub bb: [u64; 12],
    pub side: Color,
    pub castling: u8,
    pub ep_sq: Square,
    pub halfmove: u16,
    pub fullmove: u16,
    pub key: u64,
    pub history: Vec<StateInfo>,
    pub game_ply: u16,
}

impl Position {
    pub fn new() -> Self {
        Position {
            bb: [0; 12],
            side: Color::White,
            castling: 0,
            ep_sq: NO_SQ,
            halfmove: 0,
            fullmove: 1,
            key: 0,
            history: Vec::with_capacity(256),
            game_ply: 0,
        }
    }

    pub fn from_fen(fen: &str) -> Option<Self> {
        let mut pos = Position::new();
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 2 { return None; }

        // Walk the board field from A8 to H1. `sq` always points at the square
        // currently being filled.
        let mut sq: Square = A8;
        for c in parts[0].chars() {
            match c {
                '/' => {
                    // A slash may only follow a complete rank, and there must
                    // be another rank below it.
                    if sq < 16 || !sq.is_multiple_of(8) { return None; }
                    sq -= 16;
                }
                '1'..='8' => sq += c as u8 - b'0',
                _ => {
                    if sq >= 64 { return None; }
                    let piece = char_to_piece(c)?;
                    pos.bb[piece.idx()] |= 1u64 << sq;
                    sq += 1;
                }
            }
        }
        // Exactly eight ranks must have been consumed.
        if sq != 8 { return None; }

        pos.side = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return None,
        };

        if parts.len() > 2 {
            for c in parts[2].chars() {
                match c { 'K' => pos.castling |= CASTLE_WK, 'Q' => pos.castling |= CASTLE_WQ,
                          'k' => pos.castling |= CASTLE_BK, 'q' => pos.castling |= CASTLE_BQ, _ => {} }
            }
        }

        if parts.len() > 3 && parts[3] != "-" {
            let bytes = parts[3].as_bytes();
            if bytes.len() >= 2 {
                let f = bytes[0].wrapping_sub(b'a');
                let r = bytes[1].wrapping_sub(b'1');
                if f < 8 && r < 8 { pos.ep_sq = r * 8 + f; }
            }
        }

        if parts.len() > 4 { pos.halfmove = parts[4].parse().unwrap_or(0); }
        if parts.len() > 5 { pos.fullmove = parts[5].parse().unwrap_or(1); }

        pos.key = pos.compute_zobrist();
        Some(pos)
    }

    pub fn occupancy(&self) -> u64 {
        let mut occ = 0;
        for i in 0..12 { occ |= self.bb[i]; }
        occ
    }

    pub fn us(&self) -> u64 { self.color_bb(self.side) }
    pub fn them(&self) -> u64 { self.color_bb(self.side.flip()) }

    pub fn color_bb(&self, c: Color) -> u64 {
        let base = c.idx() * 6;
        self.bb[base] | self.bb[base+1] | self.bb[base+2] | self.bb[base+3] | self.bb[base+4] | self.bb[base+5]
    }

    pub fn king_sq(&self, c: Color) -> Square {
        let bb = self.bb[c.idx() * 6 + 5];
        if bb == 0 { return 64; }
        bb.trailing_zeros() as Square
    }

    pub fn piece_on(&self, sq: Square) -> Option<Piece> {
        let mask = 1u64 << sq;
        for i in 0..12 {
            if self.bb[i] & mask != 0 { return Piece::from_idx(i); }
        }
        None
    }

    pub fn is_check(&self) -> bool {
        let ks = self.king_sq(self.side);
        self.is_attacked(ks, self.side.flip())
    }

    pub fn is_checkmate(&self) -> bool {
        self.is_check() && !self.has_legal()
    }

    pub fn is_stalemate(&self) -> bool {
        !self.is_check() && !self.has_legal()
    }

    fn has_legal(&self) -> bool {
        let moves = crate::rcengine::movegen::generate_moves(self);
        for &mv in moves.iter() {
            let mut pos = self.clone();
            if !pos.make_move(mv) { continue; }
            let us = self.side;
            if !pos.is_attacked(pos.king_sq(us), us.flip()) { return true; }
        }
        false
    }

    pub fn is_attacked(&self, sq: Square, by: Color) -> bool {
        if sq >= 64 { return false; }
        let occ = self.occupancy();
        let by_base = by.idx() * 6;
        let t = crate::rcengine::movegen::tables();

        let pawn_att = if by == Color::White { t.b_pawn_att[sq as usize] } else { t.w_pawn_att[sq as usize] };
        if pawn_att & self.bb[by_base] != 0 ||
           t.knight_att[sq as usize] & self.bb[by_base + 1] != 0 ||
           t.king_att[sq as usize] & self.bb[by_base + 5] != 0 { return true; }

        let bq = self.bb[by_base + 2] | self.bb[by_base + 4];
        let rq = self.bb[by_base + 3] | self.bb[by_base + 4];
        crate::rcengine::movegen::bishop_attacks(sq, occ) & bq != 0 ||
        crate::rcengine::movegen::rook_attacks(sq, occ) & rq != 0
    }

    pub fn make_move(&mut self, mv: Move) -> bool {
        let from = mv.from();
        let to = mv.to();
        let us = self.side;
        let them = us.flip();
        let us_base = us.idx() * 6;
        let them_base = them.idx() * 6;

        let from_mask = 1u64 << from;
        let to_mask = 1u64 << to;

        let captured = self.piece_on(to);
        if captured == Some(Piece(PieceKind::King, them)) { return false; }

        // Find piece at source
        let mut piece_idx = 12;
        for i in us_base..us_base + 6 {
            if self.bb[i] & from_mask != 0 { piece_idx = i; break; }
        }
        if piece_idx == 12 { return false; }

        let piece = Piece::from_idx(piece_idx).unwrap();
        let kind = piece.0;

        // Save state
        let state = StateInfo {
            mv,
            castling: self.castling,
            ep_sq: self.ep_sq,
            halfmove: self.halfmove,
            captured,
            key: self.key,
        };

        // Move piece
        self.bb[piece_idx] ^= from_mask;
        if let Some(cap) = captured {
            self.bb[cap.idx()] ^= to_mask;
        }

        if let Some(pk) = mv.promo() {
            self.bb[us_base + pk.idx()] |= to_mask;
        } else {
            self.bb[piece_idx] |= to_mask;
        }

        // En passant
        if mv.is_ep() {
            let ep_sq = if us == Color::White { to - 8 } else { to + 8 };
            let ep_mask = 1u64 << ep_sq;
            for i in them_base..them_base + 6 {
                if self.bb[i] & ep_mask != 0 { self.bb[i] ^= ep_mask; break; }
            }
        }

        // Castling rook
        if mv.is_castle() {
            match to {
                G1 => { self.bb[us_base + 3] ^= (1u64 << H1) | (1u64 << F1); }
                C1 => { self.bb[us_base + 3] ^= (1u64 << A1) | (1u64 << D1); }
                G8 => { self.bb[us_base + 3] ^= (1u64 << H8) | (1u64 << F8); }
                C8 => { self.bb[us_base + 3] ^= (1u64 << A8) | (1u64 << D8); }
                _ => {}
            }
        }

        // En passant square
        self.ep_sq = NO_SQ;
        if kind == PieceKind::Pawn {
            let diff = to.abs_diff(from);
            if diff == 16 {
                self.ep_sq = if us == Color::White { from + 8 } else { from - 8 };
            }
        }

        // Castling rights
        self.castling &= castle_mask(from);
        self.castling &= castle_mask(to);

        // 50-move
        self.halfmove = if kind == PieceKind::Pawn || captured.is_some() { 0 } else { self.halfmove + 1 };

        // Switch turn
        self.side = them;
        self.game_ply += 1;
        if self.side == Color::White { self.fullmove += 1; }

        // Update Zobrist
        self.key = zobrist_update(state.key, mv, piece, captured,
                                  state.castling, self.castling,
                                  state.ep_sq, self.ep_sq);

        // Legality check
        if self.is_attacked(self.king_sq(us), them) {
            self.unmake(state);
            return false;
        }

        self.history.push(state);
        true
    }

    fn unmake(&mut self, state: StateInfo) {
        let mv = state.mv;
        let from = mv.from();
        let to = mv.to();
        let us = self.side.flip();
        let us_base = us.idx() * 6;

        let to_mask = 1u64 << to;

        let mut piece_idx = 12;
        for i in us_base..us_base + 6 {
            if self.bb[i] & to_mask != 0 { piece_idx = i; break; }
        }
        if piece_idx == 12 { return; }

        // Remove from dest, restore to source
        self.bb[piece_idx] ^= to_mask;
        self.bb[piece_idx] |= 1u64 << from;

        // Undo promotion
        if mv.promo().is_some() {
            self.bb[piece_idx] ^= 1u64 << from;
            self.bb[us_base] |= 1u64 << from;
        }

        // Restore captured
        if let Some(cap) = state.captured {
            self.bb[cap.idx()] |= to_mask;
        }

        // Undo EP
        if mv.is_ep() {
            let ep_sq = if us == Color::White { to - 8 } else { to + 8 };
            let ep_pawn = us.flip().idx() * 6;
            self.bb[ep_pawn] |= 1u64 << ep_sq;
        }

        // Undo castling rook
        if mv.is_castle() {
            match to {
                G1 => { self.bb[us_base + 3] ^= (1u64 << H1) | (1u64 << F1); }
                C1 => { self.bb[us_base + 3] ^= (1u64 << A1) | (1u64 << D1); }
                G8 => { self.bb[us_base + 3] ^= (1u64 << H8) | (1u64 << F8); }
                C8 => { self.bb[us_base + 3] ^= (1u64 << A8) | (1u64 << D8); }
                _ => {}
            }
        }

        // Restore state
        self.castling = state.castling;
        self.ep_sq = state.ep_sq;
        self.halfmove = state.halfmove;
        self.key = state.key;
        self.side = us;
        self.game_ply -= 1;
        if self.side == Color::Black { self.fullmove -= 1; }
    }

    pub fn make_null_move(&mut self) -> bool {
        if self.is_check() { return false; }
        let state = StateInfo {
            mv: Move(0),
            castling: self.castling,
            ep_sq: self.ep_sq,
            halfmove: self.halfmove,
            captured: None,
            key: self.key,
        };
        self.ep_sq = NO_SQ;
        self.side = self.side.flip();
        self.key ^= crate::rcengine::zobrist::zobrist().side[0] ^ crate::rcengine::zobrist::zobrist().side[1];
        if state.ep_sq != NO_SQ {
            self.key ^= crate::rcengine::zobrist::zobrist().ep[state.ep_sq as usize];
        }
        self.history.push(state);
        true
    }

    pub fn undo_null_move(&mut self) -> bool {
        if let Some(state) = self.history.pop() {
            self.castling = state.castling;
            self.ep_sq = state.ep_sq;
            self.halfmove = state.halfmove;
            self.key = state.key;
            self.side = self.side.flip();
            true
        } else { false }
    }

    pub fn undo_move(&mut self) {
        if let Some(state) = self.history.pop() {
            self.unmake(state);
        }
    }

    pub fn compute_zobrist(&self) -> u64 {
        let z = zobrist();
        let mut h = 0u64;
        for sq in 0..64 {
            let mask = 1u64 << sq;
            for i in 0..12 {
                if self.bb[i] & mask != 0 {
                    h ^= z.pieces[i][sq as usize];
                }
            }
        }
        h ^= z.castling[self.castling as usize];
        if self.ep_sq != NO_SQ {
            h ^= z.ep[self.ep_sq as usize];
        }
        h ^= z.side[self.side.idx()];
        h
    }

    pub fn perft(&self, depth: u32) -> u64 {
        if depth == 0 { return 1; }
        let moves = crate::rcengine::movegen::generate_moves(self);
        if depth == 1 { return moves.len as u64; }
        let mut total = 0;
        for &mv in moves.iter() {
            let mut pos = self.clone();
            if pos.make_move(mv) {
                total += pos.perft(depth - 1);
            }
        }
        total
    }

    pub fn perft_divide(&self, depth: u32) {
        let moves = crate::rcengine::movegen::generate_moves(self);
        let mut total = 0;
        for &mv in moves.iter() {
            let mut pos = self.clone();
            if pos.make_move(mv) {
                let cnt = if depth <= 1 { 1 } else { pos.perft(depth - 1) };
                println!("{}: {}", mv, cnt);
                total += cnt;
            }
        }
        println!("\nTotal: {}", total);
    }
}

impl Clone for Position {
    fn clone(&self) -> Self {
        Position {
            bb: self.bb,
            side: self.side,
            castling: self.castling,
            ep_sq: self.ep_sq,
            halfmove: self.halfmove,
            fullmove: self.fullmove,
            key: self.key,
            history: Vec::new(),
            game_ply: self.game_ply,
        }
    }
}

// Incremental Zobrist deltas need the before/after state for castling and
// en-passant; grouping them into tuples would obscure the call site.
#[allow(clippy::too_many_arguments)]
fn zobrist_update(old_key: u64, mv: Move, piece: Piece, captured: Option<Piece>,
                  old_castling: u8, new_castling: u8,
                  old_ep: Square, new_ep: Square) -> u64 {
    let z = zobrist();
    let mut key = old_key;

    // Castling rights change
    key ^= z.castling[old_castling as usize] ^ z.castling[new_castling as usize];

    // EP square change
    if old_ep != NO_SQ {
        key ^= z.ep[old_ep as usize];
    }
    if new_ep != NO_SQ {
        key ^= z.ep[new_ep as usize];
    }

    let from = mv.from();
    let to = mv.to();

    key ^= z.pieces[piece.idx()][from as usize];
    key ^= z.pieces[if let Some(pk) = mv.promo() { Piece(pk, piece.1) } else { piece }.idx()][to as usize];

    if let Some(cap) = captured {
        key ^= z.pieces[cap.idx()][to as usize];
    }

    if mv.is_ep() {
        let ep_sq = if piece.1 == Color::White { to - 8 } else { to + 8 };
        let ep_pawn = Piece(PieceKind::Pawn, piece.1.flip());
        key ^= z.pieces[ep_pawn.idx()][ep_sq as usize];
    }

    if mv.is_castle() {
        let (rook_from, rook_to) = match to {
            G1 => (H1, F1), C1 => (A1, D1),
            G8 => (H8, F8), C8 => (A8, D8),
            _ => unreachable!(),
        };
        let rook = Piece(PieceKind::Rook, piece.1);
        key ^= z.pieces[rook.idx()][rook_from as usize];
        key ^= z.pieces[rook.idx()][rook_to as usize];
    }

    key ^= z.side[0] ^ z.side[1];
    key
}

fn castle_mask(sq: Square) -> u8 {
    match sq {
        A1 => !CASTLE_WQ, H1 => !CASTLE_WK,
        A8 => !CASTLE_BQ, H8 => !CASTLE_BK,
        E1 => !(CASTLE_WK | CASTLE_WQ),
        E8 => !(CASTLE_BK | CASTLE_BQ),
        _ => 0xFF,
    }
}
