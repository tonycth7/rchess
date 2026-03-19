// src/pgn_import.rs — PGN file parser
//
// Parses a .pgn file and returns a list of moves in UCI notation.
// Supports standard SAN notation with disambiguation, captures, promotions.

use crate::engine::*;
use crate::app::parse_notation;

#[derive(Debug)]
pub struct PgnGame {
    pub white:    String,
    pub black:    String,
    pub result:   String,
    pub date:     String,
    pub event:    String,
    /// Moves in UCI coordinate notation (e2e4, g1f3, ...)
    pub uci_moves: Vec<String>,
}

pub fn parse_pgn(content: &str) -> Result<PgnGame, String> {
    let mut white  = "?".to_string();
    let mut black  = "?".to_string();
    let mut result = "*".to_string();
    let mut date   = "?".to_string();
    let mut event  = "?".to_string();

    // Parse headers
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let inner = &line[1..line.len()-1];
            if let Some((key, val)) = inner.split_once(' ') {
                let val = val.trim_matches('"');
                match key {
                    "White"  => white  = val.to_string(),
                    "Black"  => black  = val.to_string(),
                    "Result" => result = val.to_string(),
                    "Date"   => date   = val.to_string(),
                    "Event"  => event  = val.to_string(),
                    _        => {}
                }
            }
        }
    }

    // Extract move text (everything after the headers)
    let move_text = content.lines()
        .skip_while(|l| l.trim().starts_with('[') || l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    // Tokenise: strip move numbers, comments, result tokens
    let tokens: Vec<&str> = move_text.split_whitespace()
        .filter(|t| {
            !t.ends_with('.')
            && !t.starts_with('{')
            && !t.starts_with(';')
            && !matches!(*t, "1-0" | "0-1" | "1/2-1/2" | "*")
        })
        .collect();

    // Replay moves to convert SAN → UCI
    let mut board  = start_board();
    let mut color  = Color::White;
    let mut ep:    Option<(usize,usize)> = None;
    let mut cast   = Castle::all();
    let mut uci_moves = vec![];

    for token in tokens {
        // Skip move numbers like "1." "1..." "12."
        if token.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            continue;
        }
        let san = token.trim_end_matches(['!','?','+','#',' ']);
        match parse_notation(&board, color, ep, &cast, san) {
            Ok(mv) => {
                let ff  = (b'a' + mv.fr.1 as u8) as char;
                let fr  = 8 - mv.fr.0;
                let tf  = (b'a' + mv.to.1 as u8) as char;
                let tr  = 8 - mv.to.0;
                let promo = mv.promo.map(|k| match k {
                    Kind::Q=>"q", Kind::R=>"r", Kind::B=>"b", _=>"n"
                }).unwrap_or("");
                let uci = format!("{}{}{}{}{}", ff, fr, tf, tr, promo);
                uci_moves.push(uci.clone());

                let (nb, ne, nc) = apply(&board, &mv, ep, &cast);
                board = nb; ep = ne; cast = nc;
                color = color.opp();
            }
            Err(e) => {
                crate::rlog!("[rchess/pgn] could not parse '{}': {}", san, e);
                // Continue rather than abort — skip unparseable tokens
            }
        }
    }

    if uci_moves.is_empty() {
        return Err("No moves found in PGN".to_string());
    }

    Ok(PgnGame { white, black, result, date, event, uci_moves })
}

/// Read a PGN file from disk and parse it.
pub fn load_pgn_file(path: &str) -> Result<PgnGame, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path, e))?;
    parse_pgn(&content)
}
