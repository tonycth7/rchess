use std::io::{self, BufRead, Write};
use crate::rcengine::board::Position;
use crate::rcengine::search::Search;
use crate::rcengine::types::*;

pub struct UciEngine {
    pos: Position,
    search: Search,
}

impl UciEngine {
    pub fn new() -> Self {
        UciEngine {
            pos: Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap(),
            search: Search::new(),
        }
    }

    pub fn uci_loop(&mut self) {
        let stdin = io::stdin();
        let mut info_line = String::new();

        println!("id name RCENGINE 0.1");
        println!("id author RCENGINE Team");
        println!("uciok");
        io::stdout().flush().ok();

        loop {
            info_line.clear();
            if stdin.lock().read_line(&mut info_line).is_err() || info_line.is_empty() {
                break;
            }
            let line = info_line.trim();

            match line {
                "quit" => break,
                "stop" => {
                    // In this synchronous implementation, the search already
                    // completed before we read this. Just acknowledge.
                    // The `go` handler will have already returned bestmove.
                }
                "uci" => {
                    println!("id name RCENGINE 0.1");
                    println!("id author RCENGINE Team");
                    println!("uciok");
                }
                "isready" => println!("readyok"),
                "ucinewgame" => {
                    self.search = Search::new();
                    self.pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
                }
                _ => {
                    if line.starts_with("position") {
                        self.handle_position(line);
                    } else if line.starts_with("go") {
                        self.handle_go(line);
                    }
                }
            }
            io::stdout().flush().ok();
        }
    }

    fn handle_position(&mut self, line: &str) {
        if let Some(fen_start) = line.strip_prefix("position fen ") {
            if let Some(pos) = Position::from_fen(fen_start) {
                self.pos = pos;
            }
        } else if let Some(moves_str) = line.strip_prefix("position startpos moves ") {
            self.pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
            self.apply_moves(moves_str);
        } else if line == "position startpos" {
            self.pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        }
    }

    fn apply_moves(&mut self, moves_str: &str) {
        for token in moves_str.split_whitespace() {
            if let Some(mv) = parse_move(&self.pos, token) {
                self.pos.make_move(mv);
            }
        }
    }

    fn handle_go(&mut self, line: &str) {
        let mut limits = Limits::default();

        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "depth" => { i += 1; limits.depth = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "movetime" => { i += 1; limits.movetime = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "wtime" => { i += 1; limits.wtime = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "btime" => { i += 1; limits.btime = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "winc" => { i += 1; limits.winc = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "binc" => { i += 1; limits.binc = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "movestogo" => { i += 1; limits.movestogo = parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
                "infinite" => limits.infinite = true,
                _ => {}
            }
            i += 1;
        }

        self.search.output_info = true;
        let best = self.search.go(&self.pos, &limits);
        // info + bestmove printed inside go() when output_info is true
    }
}

fn parse_move(pos: &Position, s: &str) -> Option<Move> {
    if s.len() < 4 { return None; }

    let bytes = s.as_bytes();
    let from_file = bytes[0].wrapping_sub(b'a');
    let from_rank = bytes[1].wrapping_sub(b'1');
    let to_file = if bytes.len() > 2 { bytes[2].wrapping_sub(b'a') } else { return None; };
    let to_rank = if bytes.len() > 3 { bytes[3].wrapping_sub(b'1') } else { return None; };

    if from_file > 7 || from_rank > 7 || to_file > 7 || to_rank > 7 { return None; }

    let from = from_rank * 8 + from_file;
    let to = to_rank * 8 + to_file;

    let promo = if bytes.len() > 4 {
        match bytes[4] {
            b'n' => Some(PieceKind::Knight), b'b' => Some(PieceKind::Bishop),
            b'r' => Some(PieceKind::Rook), b'q' => Some(PieceKind::Queen),
            _ => None,
        }
    } else {
        None
    };

    if let Some(pk) = promo {
        Some(Move::new_promo(from, to, pk))
    } else if let Some(piece) = pos.piece_on(from) {
        let kind = piece.0;
        // Check for castling
        if kind == PieceKind::King {
            if from == E1 && to == G1 { return Some(Move::new_castle(from, to)); }
            if from == E1 && to == C1 { return Some(Move::new_castle(from, to)); }
            if from == E8 && to == G8 { return Some(Move::new_castle(from, to)); }
            if from == E8 && to == C8 { return Some(Move::new_castle(from, to)); }
        }
        // Check for en passant
        if kind == PieceKind::Pawn && to == pos.ep_sq {
            return Some(Move::new_ep(from, to));
        }
        Some(Move::new(from, to))
    } else {
        None
    }
}
