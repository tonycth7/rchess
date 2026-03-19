// src/puzzle.rs — Lichess daily puzzle via HTTP (uses ureq)
//
// Fetches: https://lichess.org/api/puzzle/daily
// Falls back gracefully if network is unavailable.

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
    let mut req = ureq::get("https://lichess.org/api/puzzle/daily")
        .set("Accept", "application/json")
        .set("User-Agent", "rchess/0.7.3 (terminal chess)");
    if !token.is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", token));
        rlog!("[rchess/puzzle] using API token");
    }
    let response = req
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("Network error: {}", e))?;

    let body = response.into_string().map_err(|e| format!("Read error: {}", e))?;
    rlog!("[rchess/puzzle] HTTP OK, body length: {}", body.len());
    parse_puzzle_json(&body)
}

// Lichess daily puzzle JSON structure (2024):
// {
//   "game": { "pgn": "e4 e5 ..." },
//   "puzzle": {
//     "id": "abc12",
//     "rating": 1482,
//     "solution": ["e2e4","e7e5",...],   <- JSON ARRAY not a string
//     "themes": ["fork","middlegame",...] <- JSON ARRAY
//   },
//   "user": null
// }
// The FEN is in "game" -> look for "fen": or reconstruct from "initialFen".
// Note: the puzzle FEN field may be at root or inside "puzzle" object.
fn parse_puzzle_json(json: &str) -> Result<Puzzle, String> {
    rlog!("[rchess/puzzle] raw JSON first 400: {}", &json[..json.len().min(400)]);

    let puzzle_obj = find_str_after(json, "puzzle").unwrap_or_default();
    let game_obj   = find_str_after(json, "game").unwrap_or_default();

    // Puzzle ID
    let id = extract_str_val(&puzzle_obj, "id")
        .unwrap_or_else(|| "?".to_string());

    // Rating
    let rating = extract_num_val(&puzzle_obj, "rating").unwrap_or(1500);

    // Solution: UCI moves array in puzzle.solution
    let moves = extract_json_str_array(&puzzle_obj, "solution")
        .or_else(|| extract_json_str_array(json, "solution"))
        .or_else(|| extract_str_val(json, "moves")
            .map(|s| s.split_whitespace().map(|m| m.to_string()).collect()))
        .unwrap_or_default();

    // Themes
    let themes = extract_json_str_array(&puzzle_obj, "themes")
        .unwrap_or_default();

    // initialPly: how many half-moves into the game the puzzle starts
    let initial_ply = extract_num_val(&puzzle_obj, "initialPly")
        .unwrap_or(0) as usize;

    // FEN: Lichess does NOT provide a fen field — we must replay game.pgn
    // up to initialPly half-moves to reach the puzzle start position.
    // Fallback: try direct fen fields anyway for forward-compat.
    let fen = extract_str_val(&puzzle_obj, "fen")
        .or_else(|| extract_str_val(&puzzle_obj, "initialFen"))
        .or_else(|| extract_str_val(&game_obj, "fen"))
        .or_else(|| extract_str_val(json, "fen"))
        .or_else(|| {
            // Primary path: replay game.pgn up to initialPly
            let pgn = extract_str_val(&game_obj, "pgn")?;
            rlog!("[rchess/puzzle] replaying pgn ({} chars) to ply {}", pgn.len(), initial_ply);
            fen_after_san_ply(&pgn, initial_ply)
        })
        .unwrap_or_default();

    rlog!("[rchess/puzzle] id={} rating={} ply={} fen_len={} moves={} themes={}",
        id, rating, initial_ply, fen.len(), moves.len(), themes.len());

    if fen.is_empty() {
        // Helpful error showing top-level keys
        let top_keys: Vec<&str> = json.split('"')
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .map(|(_, s)| s)
            .filter(|s| !s.is_empty() && !s.contains('{') && !s.contains('}'))
            .take(10)
            .collect();
        return Err(format!("Could not determine puzzle position. Top keys: {}",
            top_keys.join(", ")));
    }
    if moves.is_empty() {
        return Err("No solution moves in puzzle response".to_string());
    }

    rlog!("[rchess/puzzle] success — fen={}", &fen[..fen.len().min(60)]);
    Ok(Puzzle { id, fen, moves, rating, themes, solution_start: 1 })
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

/// Encode board state as a FEN string.
fn board_to_fen(
    board: &crate::engine::Board,
    color: crate::engine::Color,
    ep:    Option<(usize,usize)>,
    cast:  &crate::engine::Castle,
) -> String {
    use crate::engine::{Color as C, Kind};
    let mut ranks = vec![];
    for r in 0..8 {
        let mut rank = String::new();
        let mut empty = 0u8;
        for c in 0..8 {
            match board[r][c] {
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

/// Get the substring starting from just after the first occurrence of `"key":{`
fn find_str_after(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    if rest.starts_with('{') {
        // Find matching closing brace
        let mut depth = 0usize;
        let mut end   = 0usize;
        for (i, ch) in rest.char_indices() {
            match ch { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { end = i; break; } } _ => {} }
        }
        Some(rest[..=end].to_string())
    } else {
        Some(rest.to_string())
    }
}

/// Extract a plain string value: "key":"value"
fn extract_str_val(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start  = json.find(&needle)? + needle.len();
    let end    = json[start..].find('"')? + start;
    let val    = json[start..end].to_string();
    if val.is_empty() { None } else { Some(val) }
}

/// Extract a number value: "key":1234
fn extract_num_val(json: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    let end    = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if end == 0 { return None; }
    rest[..end].parse().ok()
}

/// Extract a JSON string array: "key":["val1","val2",...]
fn extract_json_str_array(json: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{}\":[", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = &json[start..];
    // Find closing ]
    let end = rest.find(']').unwrap_or(rest.len());
    let inner = &rest[..end];
    let items: Vec<String> = inner.split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() { None } else { Some(items) }
}

/// Parse a FEN string into board state.
/// Returns (board, turn, ep, castling, fullmove) or an error string.
pub fn parse_fen(fen: &str) -> Result<(Board, Color, Option<(usize,usize)>, Castle, u32), String> {
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
