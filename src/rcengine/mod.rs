// RCENGINE — Next-generation chess engine in Rust
//
// Modules:
//   types   - Core types (Square, Move, Piece, Color, etc.)
//   board   - Bitboard-based position representation with FEN
//   movegen - Legal move generation via magic bitboards
//   search  - Alpha-beta search, evaluation, transposition table
//   uci     - UCI protocol interface
//   zobrist - Zobrist hashing constants (lazy-initialized)
//
// Usage:
//   1. Call rcengine::init() once at startup
//   2. Create Position, generate moves, run search, etc.

pub mod types;
pub mod board;
pub mod movegen;
pub mod search;
pub mod uci;
pub mod zobrist;

/// Initialize the engine (must be called once before use).
pub fn init() {
    movegen::init();
}
