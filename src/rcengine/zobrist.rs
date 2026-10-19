//! Zobrist hashing constants — generated once at runtime.
use std::sync::OnceLock;

static ZOBRIST: OnceLock<ZobristTable> = OnceLock::new();

pub struct ZobristTable {
    pub pieces: [[u64; 64]; 12],
    pub castling: [u64; 16],
    pub ep: [u64; 64],
    pub side: [u64; 2],
}

fn init_zobrist() -> ZobristTable {
    let mut rng = ZobristRng::new(0x123456789ABCDEF);
    let mut z = ZobristTable {
        pieces: [[0; 64]; 12],
        castling: [0; 16],
        ep: [0; 64],
        side: [0; 2],
    };
    for i in 0..12 { for j in 0..64 { z.pieces[i][j] = rng.next(); } }
    for i in 0..16 { z.castling[i] = rng.next(); }
    for i in 0..64 { z.ep[i] = rng.next(); }
    z.side[0] = rng.next();
    z.side[1] = rng.next();
    z
}

pub fn zobrist() -> &'static ZobristTable {
    ZOBRIST.get_or_init(init_zobrist)
}

/// Simple splitmix64 PRNG for generating Zobrist keys
struct ZobristRng(u64);
impl ZobristRng {
    fn new(seed: u64) -> Self { ZobristRng(seed) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}
