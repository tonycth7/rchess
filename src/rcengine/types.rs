use std::fmt;

pub type Square = u8;
pub const A1: Square = 0;  pub const B1: Square = 1;  pub const C1: Square = 2;  pub const D1: Square = 3;
pub const E1: Square = 4;  pub const F1: Square = 5;  pub const G1: Square = 6;  pub const H1: Square = 7;
pub const A2: Square = 8;  pub const B2: Square = 9;  pub const C2: Square = 10; pub const D2: Square = 11;
pub const E2: Square = 12; pub const F2: Square = 13; pub const G2: Square = 14; pub const H2: Square = 15;
pub const A3: Square = 16; pub const B3: Square = 17; pub const C3: Square = 18; pub const D3: Square = 19;
pub const E3: Square = 20; pub const F3: Square = 21; pub const G3: Square = 22; pub const H3: Square = 23;
pub const A4: Square = 24; pub const B4: Square = 25; pub const C4: Square = 26; pub const D4: Square = 27;
pub const E4: Square = 28; pub const F4: Square = 29; pub const G4: Square = 30; pub const H4: Square = 31;
pub const A5: Square = 32; pub const B5: Square = 33; pub const C5: Square = 34; pub const D5: Square = 35;
pub const E5: Square = 36; pub const F5: Square = 37; pub const G5: Square = 38; pub const H5: Square = 39;
pub const A6: Square = 40; pub const B6: Square = 41; pub const C6: Square = 42; pub const D6: Square = 43;
pub const E6: Square = 44; pub const F6: Square = 45; pub const G6: Square = 46; pub const H6: Square = 47;
pub const A7: Square = 48; pub const B7: Square = 49; pub const C7: Square = 50; pub const D7: Square = 51;
pub const E7: Square = 52; pub const F7: Square = 53; pub const G7: Square = 54; pub const H7: Square = 55;
pub const A8: Square = 56; pub const B8: Square = 57; pub const C8: Square = 58; pub const D8: Square = 59;
pub const E8: Square = 60; pub const F8: Square = 61; pub const G8: Square = 62; pub const H8: Square = 63;
pub const NO_SQ: Square = 64;

pub fn sq_file(sq: Square) -> u8 { sq & 7 }
pub fn sq_rank(sq: Square) -> u8 { sq >> 3 }
pub fn sq_make(file: u8, rank: u8) -> Square { rank * 8 + file }
pub fn sq_name(sq: Square) -> String {
    if sq >= 64 { return "none".into(); }
    format!("{}{}", (b'a' + sq_file(sq)) as char, sq_rank(sq) + 1)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color { White, Black }
impl Color {
    pub const fn idx(self) -> usize { match self { Color::White => 0, Color::Black => 1 } }
    pub const fn flip(self) -> Color { match self { Color::White => Color::Black, Color::Black => Color::White } }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceKind { Pawn, Knight, Bishop, Rook, Queen, King }
impl PieceKind {
    pub const fn idx(self) -> usize { self as usize }
}
pub const NUM_KINDS: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece(pub PieceKind, pub Color);
impl Piece {
    pub fn idx(self) -> usize { self.1.idx() * 6 + self.0.idx() }
    pub fn from_idx(i: usize) -> Option<Piece> {
        if i >= 12 { return None; }
        let c = if i < 6 { Color::White } else { Color::Black };
        let k = match i % 6 { 0 => PieceKind::Pawn, 1 => PieceKind::Knight, 2 => PieceKind::Bishop,
                               3 => PieceKind::Rook, 4 => PieceKind::Queen, 5 => PieceKind::King, _ => unreachable!() };
        Some(Piece(k, c))
    }
    pub fn is_white(self) -> bool { self.1 == Color::White }
}

/// Compact move encoding (16 bits)
/// bits 0-5: from square
/// bits 6-11: to square
/// bits 12-14: promotion piece (0=none, 1=Knight, 2=Bishop, 3=Rook, 4=Queen)
/// bit 15: special flag (castling or ep)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(pub u16);

impl Move {
    pub fn new(from: Square, to: Square) -> Self { Move((from as u16) | ((to as u16) << 6)) }
    pub fn new_promo(from: Square, to: Square, kind: PieceKind) -> Self {
        let promo = match kind { PieceKind::Knight => 1, PieceKind::Bishop => 2, PieceKind::Rook => 3, PieceKind::Queen => 4, _ => 0 };
        Move((from as u16) | ((to as u16) << 6) | (promo << 12))
    }
    pub fn new_castle(from: Square, to: Square) -> Self { Move((from as u16) | ((to as u16) << 6) | (1 << 15)) }
    pub fn new_ep(from: Square, to: Square) -> Self { Move((from as u16) | ((to as u16) << 6) | (1 << 15)) }

    pub fn from(self) -> Square { (self.0 & 0x3F) as Square }
    pub fn to(self) -> Square { ((self.0 >> 6) & 0x3F) as Square }
    pub fn promo(self) -> Option<PieceKind> {
        match (self.0 >> 12) & 7 { 1 => Some(PieceKind::Knight), 2 => Some(PieceKind::Bishop),
                                   3 => Some(PieceKind::Rook), 4 => Some(PieceKind::Queen), _ => None }
    }
    pub fn is_castle(self) -> bool { (self.0 >> 15) & 1 != 0 && { let d = self.to() as i8 - self.from() as i8; d == 2 || d == -2 } }
    pub fn is_ep(self) -> bool { (self.0 >> 15) & 1 != 0 && { let d = self.to() as i8 - self.from() as i8; d != 2 && d != -2 } }
    pub fn is_special(self) -> bool { (self.0 >> 15) & 1 != 0 }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", sq_name(self.from()), sq_name(self.to()))?;
        if let Some(p) = self.promo() {
            write!(f, "{}", match p { PieceKind::Knight=>'n',PieceKind::Bishop=>'b',PieceKind::Rook=>'r',PieceKind::Queen=>'q',_=>'?' })?;
        }
        Ok(())
    }
}
impl fmt::Debug for Move { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self) } }

pub const PIECE_CHAR: &[char; 12] = &['P','N','B','R','Q','K','p','n','b','r','q','k'];
pub fn char_to_piece(c: char) -> Option<Piece> {
    let idx = PIECE_CHAR.iter().position(|&x| x == c)?;
    Piece::from_idx(idx)
}

/// Castling rights bitmask
pub const CASTLE_WK: u8 = 1; pub const CASTLE_WQ: u8 = 2;
pub const CASTLE_BK: u8 = 4; pub const CASTLE_BQ: u8 = 8;

/// Score type (centipawns)
pub type Score = i32;

pub const INF: Score = 100_000;
pub const MATE: Score = 99_999;
pub const DRAW: Score = 0;

/// Time management
#[derive(Clone)]
#[derive(Default)]
pub struct Limits {
    pub depth: u32,
    pub movetime: u64,
    pub wtime: u64,
    pub btime: u64,
    pub winc: u64,
    pub binc: u64,
    pub movestogo: u32,
    pub infinite: bool,
}

