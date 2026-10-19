use std::sync::OnceLock;
use crate::rcengine::types::*;

// ── Attack tables (pre-computed at startup) ───────────────────────────────

pub struct Tables {
    pub w_pawn_att: [u64; 64],
    pub b_pawn_att: [u64; 64],
    pub knight_att: [u64; 64],
    pub king_att:   [u64; 64],
    bishop_magic: [Magic; 64],
    rook_magic:   [Magic; 64],
    bishop_table: Vec<u64>,
    rook_table:   Vec<u64>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

pub fn tables() -> &'static Tables {
    TABLES.get().expect("movegen not initialized")
}

#[derive(Clone, Copy)]
struct Magic {
    mask: u64,
    magic: u64,
    shift: u32,
    bits: u64,
    offset: u32,
}
impl Magic {
    const fn empty() -> Self { Magic { mask: 0, magic: 0, shift: 0, bits: 0, offset: 0 } }
    fn index(&self, occ: u64) -> usize {
        let raw = ((occ & self.mask).wrapping_mul(self.magic) >> self.shift) as usize;
        (raw & self.bits as usize) + self.offset as usize
    }
}

// ── Verified magic data (magic-bitboard-finder) ──────────────────────────

const BISHOP_MAGICS_FIXED: [u64; 64] = [
    0x2020128008c80084, 0x4000218008500112, 0x401002008, 0x408104040880001,
    0x4000022448084018, 0x404010041082000, 0x206424044, 0x8a0814203202104,
    0xa244081001220020, 0x500020802002040, 0x60220080108, 0x200800210100200,
    0x2008001200040422, 0xc000000411801000, 0x40400411088010, 0x2c010948a009004,
    0x41011410408a080, 0x4400420104040444, 0x100008004102081, 0x101100400800,
    0x200010082088200, 0x202002004041108, 0x400008000480808, 0x20248200420,
    0x80200411891004, 0xc080080840204090, 0x4110041020800110, 0x50a020200002081,
    0x808000800880802, 0x1001020080001050, 0x400400408101008, 0x840004008401009,
    0x888104a100252040, 0x8200500880080812, 0x2000008804082020, 0x10010040440040,
    0x28002d001083004, 0x40a0080920111010, 0x410d0200c222, 0x8000040042040059,
    0x110101000801, 0x8b08802401000228, 0x800508400808008, 0x7048081004102081,
    0x2882004010090, 0x65400280a1010012, 0x8580809d024061, 0x8202040408080082,
    0x8100208400821020, 0x20010440228, 0x2002004208100, 0x8301601000001044,
    0x200210300210410, 0x4020042002002, 0x1040429046100428, 0x1040101000c09080,
    0x808080480201, 0x22040884048044, 0x38002000208410, 0x2020002021090,
    0x1400c42004040, 0x17c0000604050862, 0x806434d001010902, 0x3020801010e,
];

const BISHOP_SHIFTS_FIXED: [u32; 64] = [
    52, 45, 41, 47, 36, 42, 39, 46,
    58, 58, 45, 45, 35, 53, 53, 51,
    57, 51, 48, 54, 47, 46, 51, 54,
    51, 53, 52, 46, 42, 48, 54, 48,
    53, 53, 51, 54, 50, 53, 50, 51,
    56, 57, 50, 55, 51, 49, 49, 53,
    50, 42, 55, 42, 46, 56, 44, 51,
    49, 54, 51, 52, 56, 51, 50, 50,
];

const BISHOP_MASKS_FIXED: [u64; 64] = [
    0x40201008040200, 0x402010080400, 0x4020100a00, 0x40221400,
    0x2442800, 0x204085000, 0x20408102000, 0x2040810204000,
    0x20100804020000, 0x40201008040000, 0x4020100a0000, 0x4022140000,
    0x244280000, 0x20408500000, 0x2040810200000, 0x4081020400000,
    0x10080402000200, 0x20100804000400, 0x4020100a000a00, 0x402214001400,
    0x24428002800, 0x2040850005000, 0x4081020002000, 0x8102040004000,
    0x8040200020400, 0x10080400040800, 0x20100a000a1000, 0x40221400142200,
    0x2442800284400, 0x4085000500800, 0x8102000201000, 0x10204000402000,
    0x4020002040800, 0x8040004081000, 0x100a000a102000, 0x22140014224000,
    0x44280028440200, 0x8500050080400, 0x10200020100800, 0x20400040201000,
    0x2000204081000, 0x4000408102000, 0xa000a10204000, 0x14001422400000,
    0x28002844020000, 0x50005008040200, 0x20002010080400, 0x40004020100800,
    0x20408102000, 0x40810204000, 0xa1020400000, 0x142240000000,
    0x284402000000, 0x500804020000, 0x201008040200, 0x402010080400,
    0x2040810204000, 0x4081020400000, 0xa102040000000, 0x14224000000000,
    0x28440200000000, 0x50080402000000, 0x20100804020000, 0x40201008040200,
];

const ROOK_MAGICS_FIXED: [u64; 64] = [
    0x5220000420100008, 0x9081000100400080, 0x10040010020001, 0x8120020001600400,
    0x84002000104004, 0x420010000408020, 0x801000200040001, 0x20f4008204004108,
    0x28840002000104, 0x800200800100020, 0x1120000808020001, 0x20008080100008,
    0x400008080080004, 0x100008080020004, 0x10002000401, 0xa128004800441,
    0x20002420000810, 0x50000804020008, 0x8809004040080010, 0x896c202020040002,
    0x40400400020, 0x220200100008, 0x1008004040010080, 0xcd08000204011,
    0x50002000c0014011, 0x82002224002500, 0x2000080020040020, 0x3200040040080040,
    0x2400001001000801, 0x8000802004001004, 0x2020080200808001, 0x282408002102d,
    0x810000400080008, 0x1220401004001008, 0x804008004010, 0x4050010008080080,
    0x40004004004002, 0x81000100404002, 0x80008001008002, 0x8000042441000892,
    0x84050308040008, 0x103840008001010, 0x101000802000801, 0x2080100800808004,
    0x10000801010, 0x801008000401010, 0x10002008080, 0x4048202080410002,
    0x42000080010001, 0xa00200800211000a, 0x62000400105, 0x108020001001010,
    0x2030804004002004, 0x8020010020, 0x8020004000802, 0x20200410c440086,
    0x4081020448101, 0x6802100081241, 0x1003000a04102041, 0x105002010040209,
    0x1483001004020801, 0x1802820400080081, 0x40204402002081, 0x108102200108041,
];

const ROOK_SHIFTS_FIXED: [u32; 64] = [
    50, 47, 50, 51, 48, 51, 46, 47,
    48, 52, 42, 46, 46, 46, 46, 45,
    43, 52, 45, 44, 49, 48, 52, 51,
    52, 50, 52, 53, 47, 49, 45, 51,
    50, 43, 45, 50, 45, 45, 46, 52,
    41, 44, 50, 53, 51, 51, 53, 52,
    47, 48, 47, 51, 49, 52, 47, 45,
    51, 52, 52, 52, 52, 52, 51, 51,
];

const BISHOP_BITS_FIXED: [u64; 64] = [
    0x3f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x3f,
    0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f,
    0x1f, 0x1f, 0x7f, 0x7f, 0x7f, 0x7f, 0x1f, 0x1f,
    0x1f, 0x1f, 0x7f, 0x1ff, 0x1ff, 0x7f, 0x1f, 0x1f,
    0x1f, 0x1f, 0x7f, 0x1ff, 0x1ff, 0x7f, 0x1f, 0x1f,
    0x1f, 0x1f, 0x7f, 0x7f, 0x7f, 0x7f, 0x1f, 0x1f,
    0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f,
    0x3f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x3f,
];

const ROOK_BITS_FIXED: [u64; 64] = [
    0xfff, 0x7ff, 0x7ff, 0x7ff, 0x7ff, 0x7ff, 0x7ff, 0xfff,
    0x7ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x7ff,
    0x7ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x7ff,
    0x7ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x7ff,
    0x7ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x7ff,
    0x7ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x7ff,
    0x7ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x3ff, 0x7ff,
    0xfff, 0x7ff, 0x7ff, 0x7ff, 0x7ff, 0xfff, 0x7ff, 0xfff,
];

const ROOK_MASKS_FIXED: [u64; 64] = [
    0x101010101017e, 0x202020202027c, 0x404040404047a, 0x8080808080876,
    0x1010101010106e, 0x2020202020205e, 0x4040404040403e, 0x8080808080807e,
    0x1010101017e00, 0x2020202027c00, 0x4040404047a00, 0x8080808087600,
    0x10101010106e00, 0x20202020205e00, 0x40404040403e00, 0x80808080807e00,
    0x10101017e0100, 0x20202027c0200, 0x40404047a0400, 0x8080808760800,
    0x101010106e1000, 0x202020205e2000, 0x404040403e4000, 0x808080807e8000,
    0x101017e010100, 0x202027c020200, 0x404047a040400, 0x8080876080800,
    0x1010106e101000, 0x2020205e202000, 0x4040403e404000, 0x8080807e808000,
    0x1017e01010100, 0x2027c02020200, 0x4047a04040400, 0x8087608080800,
    0x10106e10101000, 0x20205e20202000, 0x40403e40404000, 0x80807e80808000,
    0x17e0101010100, 0x27c0202020200, 0x47a0404040400, 0x8760808080800,
    0x106e1010101000, 0x205e2020202000, 0x403e4040404000, 0x807e8080808000,
    0x7e010101010100, 0x7c020202020200, 0x7a040404040400, 0x76080808080800,
    0x6e101010101000, 0x5e202020202000, 0x3e404040404000, 0x7e808080808000,
    0x7e01010101010100, 0x7c02020202020200, 0x7a04040404040400, 0x7608080808080800,
    0x6e10101010101000, 0x5e20202020202000, 0x3e40404040404000, 0x7e80808080808000,
];

/// Initialize all attack tables and magic bitboard tables.
/// Must be called once before using the engine.
pub fn init() {
    let mut w_pawn_att = [0u64; 64];
    let mut b_pawn_att = [0u64; 64];
    let mut knight_att = [0u64; 64];
    let mut king_att   = [0u64; 64];
    for sq in 0..64 {
        w_pawn_att[sq as usize] = pawn_attacks(sq as Square, Color::White);
        b_pawn_att[sq as usize] = pawn_attacks(sq as Square, Color::Black);
        knight_att[sq as usize] = knight_attacks(sq as Square);
        king_att[sq as usize]   = king_attacks(sq as Square);
    }
    let (bishop_magic, bishop_table) = build_bishop_tables();
    let (rook_magic, rook_table) = build_rook_tables();
    TABLES.set(Tables {
        w_pawn_att, b_pawn_att, knight_att, king_att,
        bishop_magic, rook_magic, bishop_table, rook_table,
    }).ok();
}

const FILE_A: u64 = 0x0101010101010101;
const FILE_B: u64 = 0x0202020202020202;
const FILE_G: u64 = 0x4040404040404040;
const FILE_H: u64 = 0x8080808080808080;
const RANK_1: u64 = 0x00000000000000FF;
const RANK_2: u64 = 0x000000000000FF00;
const RANK_7: u64 = 0x00FF000000000000;
const RANK_8: u64 = 0xFF00000000000000;

fn pawn_attacks(sq: Square, c: Color) -> u64 {
    let b = 1u64 << sq;
    match c {
        // << 7 is up-left; only the H-file wraps, so mask the destination file.
        Color::White => ((b << 7) & !FILE_H) | ((b << 9) & !FILE_A),
        // >> 7 is down-right; >> 9 is down-left.
        Color::Black => ((b >> 7) & !FILE_A) | ((b >> 9) & !FILE_H),
    }
}

fn knight_attacks(sq: Square) -> u64 {
    let b = 1u64 << sq;
    let mut attacks = 0;
    attacks |= (b << 17) & !FILE_A;  // up 2, left 1
    attacks |= (b << 15) & !FILE_H;  // up 2, right 1
    attacks |= (b << 10) & !FILE_A & !FILE_B; // up 1, left 2
    attacks |= (b << 6)  & !FILE_G & !FILE_H; // up 1, right 2
    attacks |= (b >> 6)  & !FILE_A & !FILE_B; // down 1, left 2
    attacks |= (b >> 10) & !FILE_G & !FILE_H; // down 1, right 2
    attacks |= (b >> 15) & !FILE_A;  // down 2, left 1
    attacks |= (b >> 17) & !FILE_H;  // down 2, right 1
    attacks
}

fn king_attacks(sq: Square) -> u64 {
    let b = 1u64 << sq;
    let mut attacks = 0;
    attacks |= b << 8;   // up
    attacks |= b >> 8;   // down
    attacks |= (b << 1) & !FILE_A; // right
    attacks |= (b >> 1) & !FILE_H; // left
    attacks |= (b << 9) & !FILE_A;  // up-right
    attacks |= (b << 7) & !FILE_H;  // up-left
    attacks |= (b >> 7) & !FILE_A;  // down-right
    attacks |= (b >> 9) & !FILE_H;  // down-left
    attacks
}

fn bishop_attacks_ob(sq: Square, occ: u64) -> u64 {
    let mut attacks = 0;
    let f = sq_file(sq) as i8;
    let sq = sq as i8;
    for &(df, dr) in &[(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        for i in 1..8 {
            let s = sq + df * i + dr * i * 8;
            if !(0..64).contains(&s) { break; }
            if (s & 7) != f + df * i { break; }
            attacks |= 1u64 << s;
            if occ & (1u64 << s) != 0 { break; }
        }
    }
    attacks
}

fn rook_attacks_ob(sq: Square, occ: u64) -> u64 {
    let mut attacks = 0;
    let r = sq_rank(sq) as i8;
    let f = sq_file(sq) as i8;
    let sq = sq as i8;
    for &(df, dr) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for i in 1..8 {
            let s = sq + df * i + dr * i * 8;
            if !(0..64).contains(&s) { break; }
            if df != 0 && (s >> 3) != r { break; }
            if dr != 0 && (s & 7) != f { break; }
            attacks |= 1u64 << s;
            if occ & (1u64 << s) != 0 { break; }
        }
    }
    attacks
}

fn build_bishop_tables() -> ([Magic; 64], Vec<u64>) {
    let mut magic_tab = [Magic::empty(); 64];
    let mut table = Vec::new();
    let mut offset = 0u32;
    for sq in 0..64 {
        let mask = BISHOP_MASKS_FIXED[sq as usize];
        let bits = BISHOP_BITS_FIXED[sq as usize];
        let shift = BISHOP_SHIFTS_FIXED[sq as usize];
        let magic = BISHOP_MAGICS_FIXED[sq as usize];
        let size = (bits as usize) + 1;
        magic_tab[sq as usize] = Magic { mask, magic, shift, bits, offset };
        let table_start = table.len();
        table.resize(table_start + size, 0);
        for occ in &all_subsets(mask) {
            let m = Magic { mask, magic, shift, bits, offset };
            let idx = m.index(*occ);
            table[idx] = bishop_attacks_ob(sq as Square, *occ);
        }
        offset += size as u32;
    }
    (magic_tab, table)
}

fn build_rook_tables() -> ([Magic; 64], Vec<u64>) {
    let mut magic_tab = [Magic::empty(); 64];
    let mut table = Vec::new();
    let mut offset = 0u32;
    for sq in 0..64 {
        let mask = ROOK_MASKS_FIXED[sq as usize];
        let bits = ROOK_BITS_FIXED[sq as usize];
        let shift = ROOK_SHIFTS_FIXED[sq as usize];
        let magic = ROOK_MAGICS_FIXED[sq as usize];
        let size = (bits as usize) + 1;
        magic_tab[sq as usize] = Magic { mask, magic, shift, bits, offset };
        table.resize(table.len() + size, 0);
        for occ in &all_subsets(mask) {
            let m = Magic { mask, magic, shift, bits, offset };
            let idx = m.index(*occ);
            table[idx] = rook_attacks_ob(sq as Square, *occ);
        }
        offset += size as u32;
    }
    (magic_tab, table)
}

fn all_subsets(mask: u64) -> Vec<u64> {
    let mut subsets = Vec::new();
    let mut sub = 0u64;
    loop {
        subsets.push(sub);
        sub = sub.wrapping_sub(mask) & mask;
        if sub == 0 { break; }
    }
    subsets
}


// ── Public attack functions ───────────────────────────────────────────────

pub fn bishop_attacks(sq: Square, occ: u64) -> u64 {
    let t = tables();
    let m = &t.bishop_magic[sq as usize];
    t.bishop_table[m.index(occ)]
}

pub fn rook_attacks(sq: Square, occ: u64) -> u64 {
    let t = tables();
    let m = &t.rook_magic[sq as usize];
    t.rook_table[m.index(occ)]
}

pub fn queen_attacks(sq: Square, occ: u64) -> u64 {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

// ── Move generation ───────────────────────────────────────────────────────

const MAX_MOVES: usize = 256;

pub struct MoveList {
    pub moves: [Move; MAX_MOVES],
    pub len: usize,
}

impl MoveList {
    pub fn new() -> Self { MoveList { moves: [Move(0); MAX_MOVES], len: 0 } }
    pub fn push(&mut self, mv: Move) { if self.len < MAX_MOVES { self.moves[self.len] = mv; self.len += 1; } }
    pub fn iter(&self) -> impl Iterator<Item = &Move> { self.moves[..self.len].iter() }
    pub fn is_empty(&self) -> bool { self.len == 0 }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;
    fn index(&self, idx: usize) -> &Move { &self.moves[idx] }
}

/// Generate all legal pseudo-legal moves for the position
pub fn generate_moves(pos: &crate::rcengine::board::Position) -> MoveList {
    let mut ml = MoveList::new();
    let us = pos.side;
    let them = pos.side.flip();
    let occ = pos.occupancy();
    let us_occ = pos.us();
    let them_occ = pos.them();
    let target = !us_occ;
    let us_base = us.idx() * 6;

    // Pawns
    let pawns = pos.bb[us_base];
    let t = tables();
    if pawns != 0 {
        // `double_push_rank` is the rank index pawns start on (0 = rank 1).
        let (promote_rank, double_push_rank) = match us {
            Color::White => (RANK_8, 1),
            Color::Black => (RANK_1, 6),
        };

        let mut p = pawns;
        while p != 0 {
            let sq = p.trailing_zeros() as Square;
            let mask = 1u64 << sq;

            // Single push
            let push_sq = if us == Color::White { sq + 8 } else { sq - 8 };
            if push_sq < 64 && occ & (1u64 << push_sq) == 0 {
                let to_mask = 1u64 << push_sq;
                if to_mask & promote_rank != 0 {
                    for &pk in &[PieceKind::Queen, PieceKind::Rook, PieceKind::Bishop, PieceKind::Knight] {
                        ml.push(Move::new_promo(sq, push_sq, pk));
                    }
                } else {
                    ml.push(Move::new(sq, push_sq));
                }

                // Double push
                let dbl_sq = if us == Color::White { sq + 16 } else { sq - 16 };
                if dbl_sq < 64 && (sq_rank(sq) == double_push_rank) && occ & (1u64 << dbl_sq) == 0 {
                    ml.push(Move::new(sq, dbl_sq));
                }
            }

            // Captures — only onto enemy pieces; en passant is handled below.
            let att = if us == Color::White { t.w_pawn_att[sq as usize] } else { t.b_pawn_att[sq as usize] };
            let mut cap_targets = att & them_occ;
            while cap_targets != 0 {
                let to_sq = cap_targets.trailing_zeros() as Square;
                let to_mask = 1u64 << to_sq;
                if to_mask & promote_rank != 0 {
                    for &pk in &[PieceKind::Queen, PieceKind::Rook, PieceKind::Bishop, PieceKind::Knight] {
                        ml.push(Move::new_promo(sq, to_sq, pk));
                    }
                } else {
                    ml.push(Move::new(sq, to_sq));
                }
                cap_targets &= cap_targets - 1;
            }

            // En passant
            if pos.ep_sq != NO_SQ && pos.ep_sq < 64 {
                let ep_mask = 1u64 << pos.ep_sq;
                if att & ep_mask != 0 {
                    ml.push(Move::new_ep(sq, pos.ep_sq));
                }
            }

            p &= p - 1;
        }
    }

    // Knights
    let knights = pos.bb[us_base + 1];
    let mut p = knights;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = t.knight_att[sq as usize] & target;
        while att != 0 {
            let to = att.trailing_zeros() as Square;
            ml.push(Move::new(sq, to));
            att &= att - 1;
        }
        p &= p - 1;
    }

    // Bishops
    let bishops = pos.bb[us_base + 2];
    let mut p = bishops;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = bishop_attacks(sq, occ) & target;
        while att != 0 {
            let to = att.trailing_zeros() as Square;
            ml.push(Move::new(sq, to));
            att &= att - 1;
        }
        p &= p - 1;
    }

    // Rooks
    let rooks = pos.bb[us_base + 3];
    let mut p = rooks;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = rook_attacks(sq, occ) & target;
        while att != 0 {
            let to = att.trailing_zeros() as Square;
            ml.push(Move::new(sq, to));
            att &= att - 1;
        }
        p &= p - 1;
    }

    // Queens
    let queens = pos.bb[us_base + 4];
    let mut p = queens;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = queen_attacks(sq, occ) & target;
        while att != 0 {
            let to = att.trailing_zeros() as Square;
            ml.push(Move::new(sq, to));
            att &= att - 1;
        }
        p &= p - 1;
    }

    // King
    let king = pos.bb[us_base + 5];
    if king == 0 { return ml; }
    let sq = king.trailing_zeros() as Square;
    let mut att = t.king_att[sq as usize] & target;
    while att != 0 {
        let to = att.trailing_zeros() as Square;
        ml.push(Move::new(sq, to));
        att &= att - 1;
    }

    // Castling
    if !pos.is_attacked(pos.king_sq(us), them) {
        let (kingside, queenside) = match us {
            Color::White => (CASTLE_WK, CASTLE_WQ),
            Color::Black => (CASTLE_BK, CASTLE_BQ),
        };
        let (k_from, k_to, k_through, q_from, q_to, q_through) = match us {
            Color::White => (E1, G1, &[E1, F1, G1], E1, C1, &[E1, D1, C1]),
            Color::Black => (E8, G8, &[E8, F8, G8], E8, C8, &[E8, D8, C8]),
        };
        // Kingside
        if pos.castling & kingside != 0 {
            let k_occ = k_through.iter().any(|&s| s != k_from && occ & (1u64 << s) != 0);
            if !k_occ && !pos.is_attacked(k_through[1], them) {
                ml.push(Move::new_castle(k_from, k_to));
            }
        }
        // Queenside
        if pos.castling & queenside != 0 {
            let q_occ = q_through.iter().any(|&s| s != q_from && occ & (1u64 << s) != 0);
            if !q_occ && !pos.is_attacked(q_through[1], them) {
                ml.push(Move::new_castle(q_from, q_to));
            }
        }
    }

    ml
}

/// Generate only capture moves (for quiescence search)
pub fn generate_captures(pos: &crate::rcengine::board::Position) -> MoveList {
    let mut ml = MoveList::new();
    let us = pos.side;
    let occ = pos.occupancy();
    let them_occ = pos.them();
    let us_base = us.idx() * 6;

    // Pawn captures (including EP)
    let pawns = pos.bb[us_base];
    let t = tables();
    let mut p = pawns;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let att = if us == Color::White { t.w_pawn_att[sq as usize] } else { t.b_pawn_att[sq as usize] };
        let mut cap = att & them_occ;
        while cap != 0 {
            let to = cap.trailing_zeros() as Square;
            if (1u64 << to) & (if us==Color::White{RANK_8}else{RANK_1}) != 0 {
                for &pk in &[PieceKind::Queen, PieceKind::Rook, PieceKind::Bishop, PieceKind::Knight] {
                    ml.push(Move::new_promo(sq, to, pk));
                }
            } else {
                ml.push(Move::new(sq, to));
            }
            cap &= cap - 1;
        }
        if pos.ep_sq != NO_SQ && att & (1u64 << pos.ep_sq) != 0 {
            ml.push(Move::new_ep(sq, pos.ep_sq));
        }
        p &= p - 1;
    }

    // Non-capture pawn promotions (critical in quiescence)
    let free_occ = !occ;
    p = pawns;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let promo_rank = if us == Color::White { RANK_8 } else { RANK_1 };
        let push_sq = if us == Color::White { sq + 8 } else { sq - 8 };
        if (1u64 << sq) & (if us == Color::White { RANK_7 } else { RANK_2 }) != 0
            && free_occ & (1u64 << push_sq) != 0
        {
            for &pk in &[PieceKind::Queen, PieceKind::Rook, PieceKind::Bishop, PieceKind::Knight] {
                ml.push(Move::new_promo(sq, push_sq, pk));
            }
        }
        p &= p - 1;
    }

    // Knight captures
    let knights = pos.bb[us_base + 1];
    let mut p = knights;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = t.knight_att[sq as usize] & them_occ;
        while att != 0 { ml.push(Move::new(sq, att.trailing_zeros() as Square)); att &= att - 1; }
        p &= p - 1;
    }

    // Bishop captures
    let bishops = pos.bb[us_base + 2];
    let mut p = bishops;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = bishop_attacks(sq, occ) & them_occ;
        while att != 0 { ml.push(Move::new(sq, att.trailing_zeros() as Square)); att &= att - 1; }
        p &= p - 1;
    }

    // Rook captures
    let rooks = pos.bb[us_base + 3];
    let mut p = rooks;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = rook_attacks(sq, occ) & them_occ;
        while att != 0 { ml.push(Move::new(sq, att.trailing_zeros() as Square)); att &= att - 1; }
        p &= p - 1;
    }

    // Queen captures
    let queens = pos.bb[us_base + 4];
    let mut p = queens;
    while p != 0 {
        let sq = p.trailing_zeros() as Square;
        let mut att = queen_attacks(sq, occ) & them_occ;
        while att != 0 { ml.push(Move::new(sq, att.trailing_zeros() as Square)); att &= att - 1; }
        p &= p - 1;
    }

    // King captures
    let king = pos.bb[us_base + 5];
    if king == 0 { return ml; }
    let sq = king.trailing_zeros() as Square;
    let mut att = t.king_att[sq as usize] & them_occ;
    while att != 0 { ml.push(Move::new(sq, att.trailing_zeros() as Square)); att &= att - 1; }

    ml
}

/// Score moves for ordering (best first)
pub fn score_move(pos: &crate::rcengine::board::Position, mv: Move) -> i32 {
    // Captures sorted by SEE (net gain), non-captures by promotion/castle bonus
    let see_score = see(pos, mv);
    if see_score > 0 {
        return see_score + 10_000;
    }
    if pos.piece_on(mv.to()).is_some() {
        return see_score + 5_000; // losing capture, still before non-captures
    }

    // Promotions
    if mv.promo().is_some() {
        return 3_000;
    }

    // Castling
    if mv.is_castle() {
        return 50;
    }

    0
}

/// Piece values for SEE (centipawns)
const SEE_VALUES: [i32; 6] = [100, 300, 300, 500, 900, 0];

/// Static Exchange Evaluation — simulates the exchange on the target square
/// Returns estimated net gain for the moving side (negative = losing trade)
pub fn see(pos: &crate::rcengine::board::Position, mv: Move) -> i32 {
    let to = mv.to();
    let from = mv.from();
    let us = pos.side;
    let them = us.flip();
    let t = tables();

    // Value of captured piece (0 for non-captures)
    let captured_val = pos.piece_on(to).map(|p| SEE_VALUES[p.0.idx()]).unwrap_or(0);
    // Promotion gain (queen - pawn)
    let promo_gain = mv.promo().map(|pk| SEE_VALUES[pk.idx()] - SEE_VALUES[0]).unwrap_or(0);

    let initial_swap_val = captured_val + promo_gain;
    // Non-capture, non-promo has no SEE for ordering
    if initial_swap_val == 0 { return 0; }

    // Occupancy without the moving piece at `from` and without the captured piece at `to`
    let mut occ = (pos.occupancy() ^ (1u64 << from)) ^ (1u64 << to);

    // All attackers of the target square (including x-ray)
    let all_attackers = attackers_to(pos, to, occ);

    // Start with opponent's turn — they can recapture
    let mut side = them;
    let mut balance = initial_swap_val;
    let mut attackers = all_attackers;

    loop {
        // Remove pieces that no longer attack (x-ray through removed pieces)
        attackers &= attackers_to(pos, to, occ);

        // Find least valuable attacker of current side
        let lva = least_valuable_attacker(pos, side, attackers);
        if lva >= 64 { break; }
        let lva_mask = 1u64 << lva;

        // Remove this attacker from occupancy (it moves to `to` but we don't track it)
        occ ^= lva_mask;
        attackers ^= lva_mask;

        // The current side gains the value of the piece they just captured
        let attacker_val = pos.piece_on(lva).map(|p| SEE_VALUES[p.0.idx()]).unwrap_or(0);
        balance = -balance + attacker_val;

        if balance < 0 { break; }

        side = side.flip();
    }

    // The initial side is winning if the final balance is positive
    balance
}

/// Find all pieces of both colors that attack a given square (with X-ray)
fn attackers_to(pos: &crate::rcengine::board::Position, sq: Square, occ: u64) -> u64 {
    let t = tables();
    let mut atk = 0u64;
    let sq_u = sq as usize;

    // Pawn attacks: b_pawn_att[sq] gives squares where white pawns would be to attack sq
    // w_pawn_att[sq] gives squares where black pawns would be to attack sq
    atk |= t.b_pawn_att[sq_u] & pos.bb[0];  // white pawns attacking sq
    atk |= t.w_pawn_att[sq_u] & pos.bb[6];  // black pawns attacking sq

    // Knight attacks
    atk |= t.knight_att[sq_u] & (pos.bb[1] | pos.bb[7]);

    // King attacks
    atk |= t.king_att[sq_u] & (pos.bb[5] | pos.bb[11]);

    // Bishop/Queen attacks (diagonal)
    let bq = bishop_attacks(sq, occ);
    atk |= bq & (pos.bb[2] | pos.bb[8]);  // bishops
    atk |= bq & (pos.bb[4] | pos.bb[10]); // queens

    // Rook/Queen attacks (straight)
    let rq = rook_attacks(sq, occ);
    atk |= rq & (pos.bb[3] | pos.bb[9]);  // rooks
    atk |= rq & (pos.bb[4] | pos.bb[10]); // queens

    atk
}

/// Find the least valuable attacker of the given side from the attacker bitboard
fn least_valuable_attacker(pos: &crate::rcengine::board::Position, side: Color, attackers: u64) -> Square {
    let base = side.idx() * 6;

    // Check in order: Pawn, Knight, Bishop, Rook, Queen, King
    for kind_idx in 0..6 {
        let pieces = pos.bb[base + kind_idx] & attackers;
        if pieces != 0 {
            return pieces.trailing_zeros() as Square;
        }
    }
    NO_SQ
}



#[cfg(test)]
mod tests {
    use super::{generate_moves, init};
    use crate::rcengine::board::Position;

    /// Standard perft: counts leaf nodes, using `make_move`'s legality check to
    /// filter pseudo-legal moves. Depths are deliberately small so the whole
    /// suite stays well under a second.
    fn perft(pos: &mut Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = generate_moves(pos);
        let mut nodes = 0;
        for &mv in moves.iter() {
            if pos.make_move(mv) {
                nodes += if depth == 1 { 1 } else { perft(pos, depth - 1) };
                pos.undo_move();
            }
        }
        nodes
    }

    fn check(fen: &str, depth: u32, expected: u64) {
        init();
        let mut pos = Position::from_fen(fen).expect("valid FEN");
        assert_eq!(perft(&mut pos, depth), expected, "perft({depth}) for {fen}");
    }

    const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    const POSITION_3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    const POSITION_4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    const POSITION_5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    const POSITION_6: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

    #[test]
    fn perft_startpos() {
        check(STARTPOS, 1, 20);
        check(STARTPOS, 2, 400);
        check(STARTPOS, 3, 8_902);
    }

    #[test]
    fn perft_kiwipete() {
        check(KIWIPETE, 1, 48);
        check(KIWIPETE, 2, 2_039);
    }

    #[test]
    fn perft_position_3() {
        check(POSITION_3, 1, 14);
        check(POSITION_3, 2, 191);
        check(POSITION_3, 3, 2_812);
    }

    #[test]
    fn perft_position_4() {
        check(POSITION_4, 1, 6);
        check(POSITION_4, 2, 264);
        check(POSITION_4, 3, 9_467);
    }

    #[test]
    fn perft_position_5() {
        check(POSITION_5, 1, 44);
        check(POSITION_5, 2, 1_486);
        check(POSITION_5, 3, 62_379);
    }

    #[test]
    fn perft_position_6() {
        check(POSITION_6, 1, 46);
        check(POSITION_6, 2, 2_079);
        check(POSITION_6, 3, 89_890);
    }

    #[test]
    fn fen_parsing() {
        let pos = Position::from_fen(STARTPOS).expect("startpos parses");
        assert_eq!(pos.bb.iter().map(|b| b.count_ones()).sum::<u32>(), 32);
        assert!(Position::from_fen("not a fen").is_none());
        assert!(Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").is_some());
        // Wrong rank count must be rejected.
        assert!(Position::from_fen("8/8/8/8/8/8/8 w - - 0 1").is_none());
    }
}
