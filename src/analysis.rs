// src/analysis.rs — Move analysis with two selectable backends
//
// ╔══════════════════════════════════════════════════════════════╗
// ║  BACKEND 1 — Built-in (always works, no install needed)     ║
// ║    Uses the same minimax engine as the CPU opponent.        ║
// ║    Depth 2 → fast (~5-50ms per move)                        ║
// ║    Quality: good enough to spot blunders and mistakes       ║
// ╠══════════════════════════════════════════════════════════════╣
// ║  BACKEND 2 — Stockfish (optional, much stronger)            ║
// ║    Install on Arch:  sudo pacman -S stockfish               ║
// ║    Install on Deb:   sudo apt install stockfish             ║
// ║    Install on Mac:   brew install stockfish                 ║
// ║    Communicates via UCI protocol over stdin/stdout           ║
// ║    movetime = 300ms per half-position (600ms per move)      ║
// ╚══════════════════════════════════════════════════════════════╝
//
// Config key (in ~/.config/rchess/rchess_tui.conf):
//   analysis_engine = builtin      ← default
//   analysis_engine = stockfish    ← use Stockfish if installed
//
// If Stockfish is selected but not found, it automatically falls
// back to the built-in engine and logs the reason.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::config::AnalysisEngine;
use crate::engine::{
    start_board, apply, legal, Board, Castle, Color, Kind, Mv,
};
use crate::ai::{evaluate, best_mv_with_score};

// ── Movetime for Stockfish (ms per half-position eval) ────────────────────────
const SF_MOVETIME_MS: u32 = 300;


// ── Public result type ────────────────────────────────────────────────────────
/// Analysis result for one move.
/// `eval_before_cp` and `eval_after_cp` are ALWAYS White-positive centipawns
/// (positive = White is better, negative = Black is better).
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Which history index this belongs to.
    pub move_idx:       usize,
    /// White-positive eval BEFORE the move (centipawns).
    pub eval_before_cp: i32,
    /// White-positive eval AFTER the move (centipawns).
    pub eval_after_cp:  i32,
    /// Best move from the pre-move position in UCI notation (e.g. "e2e4").
    pub best_move_uci:  Option<String>,
    /// Top 3 candidate moves with scores: [(uci, cp), ...]
    pub top_moves:      Vec<(String, i32)>,
    /// Which colour made the move.
    pub color_moved:    Color,
}

// ── Internal commands ─────────────────────────────────────────────────────────
enum AnalyseCmd {
    Analyse {
        move_idx:    usize,
        uci_before:  String,
        uci_after:   String,
        color_moved: Color,
        depth:       u8,
    },
    Quit,
}

// ── Public handle ─────────────────────────────────────────────────────────────
pub struct AnalysisHandle {
    cmd_tx:     Sender<AnalyseCmd>,
    result_rx:  Receiver<AnalysisResult>,
    /// Display name shown in the UI (e.g. "Built-in d2" or "Stockfish").
    pub engine_name: String,
}

impl AnalysisHandle {
    /// Spawn the configured analysis backend.
    /// If Stockfish is requested but not found, falls back to built-in.
    pub fn spawn(choice: AnalysisEngine, depth: u8) -> Self {
        match choice {
            AnalysisEngine::Stockfish => {
                if let Some(handle) = try_spawn_stockfish(depth) {
                    return handle;
                }
                rlog!("[rchess/analysis] Stockfish not found — falling back to built-in d{}", depth);
                rlog!("[rchess/analysis] Install: sudo pacman -S stockfish");
                Self::spawn_builtin(depth)
            }
            AnalysisEngine::Builtin => Self::spawn_builtin(depth),
        }
    }

    fn spawn_builtin(depth: u8) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<AnalyseCmd>();
        let (res_tx, res_rx) = mpsc::channel::<AnalysisResult>();
        thread::spawn(move || builtin_thread(cmd_rx, res_tx, depth));
        rlog!("[rchess/analysis] built-in engine started (depth {})", depth);
        AnalysisHandle {
            cmd_tx,
            result_rx: res_rx,
            engine_name: format!("Built-in d{}", depth),
        }
    }

    /// Queue analysis of a move pair (non-blocking, returns immediately).
    pub fn analyse(
        &self,
        move_idx:    usize,
        uci_before:  &str,
        uci_after:   &str,
        color_moved: Color,
        depth:       u8,
    ) {
        let _ = self.cmd_tx.send(AnalyseCmd::Analyse {
            move_idx,
            uci_before:  uci_before.to_string(),
            uci_after:   uci_after.to_string(),
            color_moved,
            depth,
        });
    }

    /// Poll for a completed result (non-blocking).
    pub fn try_recv(&self) -> Option<AnalysisResult> {
        self.result_rx.try_recv().ok()
    }
}

impl Drop for AnalysisHandle {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(AnalyseCmd::Quit);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BACKEND 1 — Built-in minimax
// ─────────────────────────────────────────────────────────────────────────────

fn builtin_thread(cmd_rx: Receiver<AnalyseCmd>, res_tx: Sender<AnalysisResult>, depth: u8) {
    loop {
        match cmd_rx.recv() {
            Err(_) | Ok(AnalyseCmd::Quit) => break,
            Ok(AnalyseCmd::Analyse { move_idx, uci_before, uci_after, color_moved, depth: cmd_depth }) => {
                let d = cmd_depth.max(1);
                rlog!("[rchess/builtin] move {} ({:?}) depth={}", move_idx, color_moved, d);

                // Position BEFORE the move — run multi-PV (top 3)
                let (board_before, turn_before, ep_before, cast_before) =
                    replay_uci(&uci_before);

                let top = crate::ai::top_n_moves(
                    &board_before, turn_before, ep_before, &cast_before, d, 3
                );

                let best_move_uci = top.first().map(|(mv, _)| mv_to_uci(mv));
                let eval_before_cp = top.first().map(|(_, s)| *s).unwrap_or_else(|| evaluate(&board_before));

                // Top moves as (uci, cp) pairs — White-positive
                let top_moves: Vec<(String, i32)> = top.into_iter()
                    .map(|(mv, score)| (mv_to_uci(&mv), score))
                    .collect();

                // Position AFTER — static eval
                let (board_after, _, _, _) = replay_uci(&uci_after);
                let eval_after_cp = evaluate(&board_after);

                rlog!("[rchess/builtin] before={} after={} best={:?}", eval_before_cp, eval_after_cp, best_move_uci);

                let _ = res_tx.send(AnalysisResult {
                    move_idx,
                    eval_before_cp,
                    eval_after_cp,
                    best_move_uci,
                    top_moves,
                    color_moved,
                });
            }
        }
    }
    rlog!("[rchess/builtin] thread exiting");
    let _ = depth; // suppress unused warning when depth is from cmd
}

// ─────────────────────────────────────────────────────────────────────────────
// BACKEND 2 — Stockfish UCI
// ─────────────────────────────────────────────────────────────────────────────

fn try_spawn_stockfish(depth: u8) -> Option<AnalysisHandle> {
    // Test if stockfish exists
    let child = Command::new("stockfish")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let (cmd_tx, cmd_rx) = mpsc::channel::<AnalyseCmd>();
    let (res_tx, res_rx) = mpsc::channel::<AnalysisResult>();

    thread::spawn(move || stockfish_thread(child, cmd_rx, res_tx, depth));

    rlog!("[rchess/stockfish] engine spawned (movetime {}ms, depth param={})", SF_MOVETIME_MS, depth);
    Some(AnalysisHandle {
        cmd_tx,
        result_rx: res_rx,
        engine_name: format!("Stockfish {}ms", SF_MOVETIME_MS),
    })
}

fn stockfish_thread(
    mut child:  Child,
    cmd_rx:     Receiver<AnalyseCmd>,
    res_tx:     Sender<AnalysisResult>,
    _depth:     u8,
) {
    let stdin  = match child.stdin.take()  { Some(s) => s, None => return };
    let stdout = match child.stdout.take() { Some(s) => s, None => return };

    let mut w      = BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);

    // Initialise UCI
    writeln!(w, "uci").ok();
    writeln!(w, "setoption name Threads value 1").ok();
    writeln!(w, "setoption name Hash value 16").ok();
    writeln!(w, "setoption name MultiPV value 3").ok();
    writeln!(w, "isready").ok();
    w.flush().ok();

    // Wait for "readyok"
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 { return; }
        let t = line.trim();
        rlog!("[rchess/sf] init: {}", t);
        if t == "readyok" { break; }
    }
    rlog!("[rchess/sf] ready");

    loop {
        match cmd_rx.recv() {
            Err(_) | Ok(AnalyseCmd::Quit) => break,
            Ok(AnalyseCmd::Analyse { move_idx, uci_before, uci_after, color_moved, depth: _ }) => {
                rlog!("[rchess/sf] analysing move {} ({:?})", move_idx, color_moved);

                // ── Eval BEFORE the move ──────────────────────────────────────
                let pos_before = if uci_before.trim().is_empty() {
                    "position startpos".to_string()
                } else {
                    format!("position startpos moves {}", uci_before.trim())
                };
                writeln!(w, "{}", pos_before).ok();
                writeln!(w, "go movetime {}", SF_MOVETIME_MS).ok();
                w.flush().ok();

                let (before_stm, best_uci, top3) = sf_read_result_multipv(&mut reader);

                // Stockfish score is side-to-move positive → convert to White-positive
                let eval_before_cp = match color_moved {
                    Color::White =>  before_stm,
                    Color::Black => -before_stm,
                };
                // Convert top_moves scores to White-positive too
                let top_moves: Vec<(String, i32)> = top3.into_iter().map(|(mv, sc)| {
                    let wc = match color_moved { Color::White => sc, Color::Black => -sc };
                    (mv, wc)
                }).collect();

                // ── Eval AFTER the move ───────────────────────────────────────
                writeln!(w, "position startpos moves {}", uci_after.trim()).ok();
                writeln!(w, "go movetime {}", SF_MOVETIME_MS).ok();
                w.flush().ok();

                let (after_stm, _, _) = sf_read_result_multipv(&mut reader);
                let eval_after_cp = match color_moved {
                    Color::White => -after_stm,
                    Color::Black =>  after_stm,
                };

                rlog!(
                    "[rchess/sf] move {}: before={} after={} best={:?}",
                    move_idx, eval_before_cp, eval_after_cp, best_uci
                );

                let _ = res_tx.send(AnalysisResult {
                    move_idx,
                    eval_before_cp,
                    eval_after_cp,
                    best_move_uci: best_uci,
                    top_moves,
                    color_moved,
                });
            }
        }
    }

    writeln!(w, "quit").ok();
    w.flush().ok();
    let _ = child.wait();
    rlog!("[rchess/sf] thread exiting");
}

/// Read Stockfish MultiPV output until "bestmove".
/// Returns (best_score_stm, best_move_uci, top3_moves_with_scores).
/// Scores are from side-to-move perspective.
fn sf_read_result_multipv(
    reader: &mut BufReader<std::process::ChildStdout>,
) -> (i32, Option<String>, Vec<(String, i32)>) {
    // top_pv: map from multipv index (1..3) -> (score, move)
    let mut pv_map: std::collections::HashMap<u8, (i32, String)> = std::collections::HashMap::new();
    let mut best_move: Option<String> = None;
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let t = line.trim();
        if t.starts_with("info") {
            // Extract multipv index
            let tokens: Vec<&str> = t.split_whitespace().collect();
            let pv_idx: u8 = tokens.iter()
                .position(|&x| x == "multipv")
                .and_then(|i| tokens.get(i+1))
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            // Extract score
            if let Some(cp) = parse_score(t) {
                // Extract first move in pv
                let mv = tokens.iter()
                    .position(|&x| x == "pv")
                    .and_then(|i| tokens.get(i+1))
                    .map(|s| s.to_string());
                if let Some(mv) = mv {
                    pv_map.insert(pv_idx, (cp, mv));
                }
            }
        } else if t.starts_with("bestmove") {
            best_move = t.split_whitespace()
                .nth(1)
                .filter(|&s| s != "(none)")
                .map(|s| s.to_string());
            break;
        }
    }

    let best_score = pv_map.get(&1).map(|(s,_)| *s).unwrap_or(0);
    let mut top3: Vec<(String, i32)> = (1u8..=3)
        .filter_map(|i| pv_map.get(&i).map(|(sc, mv)| (mv.clone(), *sc)))
        .collect();
    // If MultiPV didn't give us results, use bestmove with score
    if top3.is_empty() {
        if let Some(mv) = &best_move {
            top3.push((mv.clone(), best_score));
        }
    }
    (best_score, best_move, top3)
}

/// Parse `score cp N` or `score mate M` from a Stockfish info line.
fn parse_score(line: &str) -> Option<i32> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let si = tokens.iter().position(|&t| t == "score")?;
    match tokens.get(si + 1)? {
        &"cp"   => tokens.get(si + 2)?.parse::<i32>().ok(),
        &"mate" => {
            let m: i32 = tokens.get(si + 2)?.parse().ok()?;
            Some(if m > 0 { 30_000 } else { -30_000 })
        }
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Replay a space-separated UCI move list from the starting position.
/// Returns (board, side_to_move, en_passant_sq, castling_rights).
fn replay_uci(moves_str: &str) -> (Board, Color, Option<(usize, usize)>, Castle) {
    let mut board = start_board();
    let mut color = Color::White;
    let mut ep:   Option<(usize, usize)> = None;
    let mut cast  = Castle::all();

    for tok in moves_str.split_whitespace() {
        let b = tok.as_bytes();
        if b.len() < 4 { continue; }
        if !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) { continue; }
        if !(b'a'..=b'h').contains(&b[2]) || !(b'1'..=b'8').contains(&b[3]) { continue; }

        let fr = ((8 - (b[1] - b'0')) as usize, (b[0] - b'a') as usize);
        let to = ((8 - (b[3] - b'0')) as usize, (b[2] - b'a') as usize);
        let promo = b.get(4).and_then(|&p| match p.to_ascii_lowercase() {
            b'q' => Some(Kind::Q), b'r' => Some(Kind::R),
            b'b' => Some(Kind::B), b'n' => Some(Kind::N), _ => None,
        });

        let moves = legal(&board, color, ep, &cast);
        if let Some(&mv) = moves.iter().find(|m| {
            m.fr == fr && m.to == to
            && match promo {
                Some(p) => m.promo == Some(p),
                None    => m.promo.is_none() || m.promo == Some(Kind::Q),
            }
        }) {
            let (nb, ne, nc) = apply(&board, &mv, ep, &cast);
            board = nb; ep = ne; cast = nc;
        }
        color = color.opp();
    }
    (board, color, ep, cast)
}

/// Convert an internal Mv to UCI coordinate notation (e.g. "e2e4", "e7e8q").
fn mv_to_uci(mv: &Mv) -> String {
    let ff = (b'a' + mv.fr.1 as u8) as char;
    let fr = (b'0' + (8 - mv.fr.0) as u8) as char;
    let tf = (b'a' + mv.to.1 as u8) as char;
    let tr = (b'0' + (8 - mv.to.0) as u8) as char;
    let promo = mv.promo.map(|k| match k {
        Kind::Q => 'q', Kind::R => 'r', Kind::B => 'b', _ => 'n',
    }).map(|c| c.to_string()).unwrap_or_default();
    format!("{}{}{}{}{}", ff, fr, tf, tr, promo)
}

/// Convert centipawns to pawn units (for display).
pub fn cp_to_pawns(cp: i32) -> f32 {
    cp as f32 / 100.0
}
