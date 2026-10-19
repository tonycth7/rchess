// src/puzzle.rs — Lichess daily puzzle via ureq HTTP client

use crate::engine::{Board, Castle, Color, Kind, Mv, legal, apply, start_board, game_status, Status};

#[derive(Debug, Clone)]
pub struct Puzzle {
    pub id:          String,
    pub fen:         String,
    pub moves:       Vec<String>,   // UCI solution moves (all moves incl. opponent setup)
    pub rating:      u32,
    pub themes:      Vec<String>,
    /// Which move index the HUMAN must play (moves[solution_start] is the first human move)
    pub solution_start: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PuzzleState {
    /// Fetching from Lichess
    Loading,
    /// Failed to fetch
    Failed(String),
    /// Playing the opponent setup moves automatically
    Setup,
    /// Waiting for human to move
    WaitingInput,
    /// Human played correct move, show next opponent move
    CorrectMove,
    /// Human played wrong move
    WrongMove(String),
    /// Puzzle solved!
    Solved,
}

pub fn fetch_daily_puzzle(token: &str) -> Result<Puzzle, String> {
    let agent = crate::http::agent(
        std::time::Duration::from_secs(10),
        Some(std::time::Duration::from_secs(20)),
    );
    let mut req = agent
        .get("https://lichess.org/api/puzzle/daily")
        .header("Accept", "application/json")
        .header("User-Agent", concat!("rchess/", env!("CARGO_PKG_VERSION"), " (terminal chess)"));
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {token}"));
        rlog!("[rchess/puzzle] using API token");
    }
    rlog!("[rchess/puzzle] fetching daily puzzle");
    let mut resp = req
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?;
    let body = crate::http::read_text(&mut resp)?;
    rlog!("[rchess/puzzle] OK, body length: {}", body.len());
    parse_puzzle_json(&body)
}

fn parse_puzzle_json(text: &str) -> Result<Puzzle, String> {
    let preview: String = text.chars().take(200).collect();
    rlog!("[rchess/puzzle] raw JSON first 200 chars: {preview}");

    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    let puzzle = &v["puzzle"];
    let game   = &v["game"];

    let id = puzzle["id"].as_str().unwrap_or("?").to_string();
    let rating = puzzle["rating"].as_u64().unwrap_or(1500) as u32;
    let initial_ply = puzzle["initialPly"].as_u64().unwrap_or(0) as usize;

    let moves: Vec<String> = puzzle["solution"].as_array()
        .map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let themes: Vec<String> = puzzle["themes"].as_array()
        .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let fen = if let Some(f) = puzzle["fen"].as_str()
        .or_else(|| puzzle["initialFen"].as_str())
        .or_else(|| game["fen"].as_str())
        .or_else(|| v["fen"].as_str())
    {
        f.to_string()
    } else if let Some(pgn) = game["pgn"].as_str() {
        rlog!("[rchess/puzzle] replaying pgn ({} chars) to ply {}", pgn.len(), initial_ply);
        fen_after_san_ply(pgn, initial_ply).unwrap_or_default()
    } else {
        String::new()
    };

    rlog!("[rchess/puzzle] id={} rating={} ply={} fen_len={} moves={} themes={}",
        id, rating, initial_ply, fen.len(), moves.len(), themes.len());

    if fen.is_empty() {
        let keys: Vec<String> = v.as_object()
            .map(|o| o.keys().take(10).cloned().collect())
            .unwrap_or_default();
        return Err(format!("Could not determine puzzle position. Top keys: {}", keys.join(", ")));
    }
    if moves.is_empty() {
        return Err("No solution moves in puzzle response".to_string());
    }

    rlog!("[rchess/puzzle] success — fen={}", &fen[..fen.len().min(60)]);
    Ok(Puzzle { id, fen, moves, rating, themes, solution_start: initial_ply % 2 })
}

/// Replay SAN moves (space-separated, as Lichess PGN field) up to `ply` half-moves.
/// Returns the FEN of the resulting position, or None on parse error.
fn fen_after_san_ply(pgn_moves: &str, ply: usize) -> Option<String> {
    use crate::engine::{start_board, apply, legal, Color, Castle};

    let mut board = start_board();
    let mut color = Color::White;
    let mut ep:   Option<(usize,usize)> = None;
    let mut cast  = Castle::all();
    let mut count = 0usize;

    for token in pgn_moves.split_whitespace() {
        if count >= ply { break; }
        // Skip move numbers like "1." "12."
        if token.ends_with('.') || token.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            continue;
        }
        // Strip result tokens
        if matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*") { break; }

        let san = token.trim_end_matches(['+', '#', '!', '?'].as_ref());
        match crate::app::parse_notation(&board, color, ep, &cast, san) {
            Ok(mv) => {
                let (nb, ne, nc) = apply(&board, &mv, ep, &cast);
                board = nb; ep = ne; cast = nc;
                color = color.opp();
                count += 1;
            }
            Err(e) => {
                rlog!("[rchess/puzzle] san parse failed '{}' at ply {}: {}", san, count, e);
                return None;
            }
        }
    }

    Some(board_to_fen(&board, color, ep, &cast))
}

fn board_to_fen(
    board: &crate::engine::Board,
    color: crate::engine::Color,
    ep:    Option<(usize,usize)>,
    cast:  &crate::engine::Castle,
) -> String {
    use crate::engine::{Color as C, Kind};
    let mut ranks = vec![];
    for row in board.iter() {
        let mut rank = String::new();
        let mut empty = 0u8;
        for cell in row.iter() {
            match cell {
                None => empty += 1,
                Some(p) => {
                    if empty > 0 { rank.push((b'0' + empty) as char); empty = 0; }
                    let ch = match p.k {
                        Kind::K => 'k', Kind::Q => 'q', Kind::R => 'r',
                        Kind::B => 'b', Kind::N => 'n', Kind::P => 'p',
                    };
                    rank.push(if p.c == C::White { ch.to_ascii_uppercase() } else { ch });
                }
            }
        }
        if empty > 0 { rank.push((b'0' + empty) as char); }
        ranks.push(rank);
    }

    let turn    = if color == C::White { "w" } else { "b" };
    let mut cas = String::new();
    if cast.wk { cas.push('K'); }
    if cast.wq { cas.push('Q'); }
    if cast.bk { cas.push('k'); }
    if cast.bq { cas.push('q'); }
    if cas.is_empty() { cas.push('-'); }

    let ep_str = match ep {
        None        => "-".to_string(),
        Some((r,c)) => format!("{}{}", (b'a' + c as u8) as char, 8 - r),
    };

    format!("{} {} {} {} 0 1", ranks.join("/"), turn, cas, ep_str)
}

/// Parsed FEN components: board, side to move, en-passant square, castling
/// rights, and fullmove number.
pub type FenPosition = (Board, Color, Option<(usize, usize)>, Castle, u32);

/// Parse a FEN string into board state.
/// Returns a [`FenPosition`] or an error string.
pub fn parse_fen(fen: &str) -> Result<FenPosition, String> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() < 2 { return Err("FEN too short".to_string()); }

    let mut board: Board = [[None; 8]; 8];
    let ranks: Vec<&str> = parts[0].split('/').collect();
    if ranks.len() != 8 { return Err("FEN must have 8 ranks".to_string()); }

    for (r, rank) in ranks.iter().enumerate() {
        let mut c = 0usize;
        for ch in rank.chars() {
            if ch.is_ascii_digit() {
                c += ch as usize - '0' as usize;
            } else {
                use crate::engine::{Piece, Color as PC};
                let color = if ch.is_uppercase() { PC::White } else { PC::Black };
                let kind  = match ch.to_ascii_lowercase() {
                    'k' => Kind::K, 'q' => Kind::Q, 'r' => Kind::R,
                    'b' => Kind::B, 'n' => Kind::N, 'p' => Kind::P,
                    x   => return Err(format!("Unknown piece '{}'", x)),
                };
                if c >= 8 { return Err("FEN rank overflow".to_string()); }
                board[r][c] = Some(Piece { c: color, k: kind });
                c += 1;
            }
        }
    }

    let turn = match parts[1] { "w" => Color::White, _ => Color::Black };

    let cast_str = parts.get(2).unwrap_or(&"-");
    let cast = Castle {
        wk: cast_str.contains('K'), wq: cast_str.contains('Q'),
        bk: cast_str.contains('k'), bq: cast_str.contains('q'),
    };

    let ep = parts.get(3).and_then(|s| {
        if *s == "-" { return None; }
        let b = s.as_bytes();
        if b.len() < 2 { return None; }
        let c = (b[0] - b'a') as usize;
        let r = (8 - (b[1] - b'0')) as usize;
        Some((r, c))
    });

    let fullmove = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);

    Ok((board, turn, ep, cast, fullmove))
}
