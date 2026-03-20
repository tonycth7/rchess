// src/puzzle.rs — Lichess puzzle fetch via embedded Python script
//
// Uses python3 (always on Arch/Linux) — no pip, no external deps.
// Includes a pure-Python chess board for FEN reconstruction without python-chess.

use crate::engine::{Board, Castle, Color, Kind};

#[derive(Debug, Clone)]
pub struct Puzzle {
    pub id:             String,
    pub fen:            String,
    pub moves:          Vec<String>,
    pub rating:         u32,
    pub themes:         Vec<String>,
    pub solution_start: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PuzzleState {
    Loading,
    Failed(String),
    Setup,
    WaitingInput,
    CorrectMove,
    WrongMove(String),
    Solved,
}

// ── Embedded Python script ────────────────────────────────────────────────────
// argv: token  theme  rmin  rmax
// Outputs key=value lines then "OK", or "ERROR:message"

const FETCH_SCRIPT: &str = r#"
import sys, json, urllib.request, urllib.error, re

token    = sys.argv[1] if len(sys.argv) > 1 else ""
theme    = sys.argv[2] if len(sys.argv) > 2 else ""
rmin_s   = sys.argv[3] if len(sys.argv) > 3 else "0"
rmax_s   = sys.argv[4] if len(sys.argv) > 4 else "9999"
rmin = int(rmin_s) if rmin_s.isdigit() else 0
rmax = int(rmax_s) if rmax_s.isdigit() else 9999

headers = {"Accept": "application/json", "User-Agent": "rchess/0.7.3"}
if token:
    headers["Authorization"] = "Bearer " + token

# ── Pure-Python chess board for FEN reconstruction ────────────────────────────
INIT_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

def fen_to_state(fen):
    parts = fen.split()
    board = []
    for r in parts[0].split('/'):
        row = []
        for c in r:
            row.extend(['.'] * int(c)) if c.isdigit() else row.append(c)
        board.append(row)
    turn   = parts[1]
    castle = parts[2] if len(parts) > 2 else 'KQkq'
    ep     = parts[3] if len(parts) > 3 else '-'
    half   = int(parts[4]) if len(parts) > 4 else 0
    full   = int(parts[5]) if len(parts) > 5 else 1
    return [board, turn, castle, ep, half, full]

def state_to_fen(s):
    board, turn, castle, ep, half, full = s
    rows = []
    for row in board:
        out = ''; empty = 0
        for c in row:
            if c == '.': empty += 1
            else:
                if empty: out += str(empty); empty = 0
                out += c
        if empty: out += str(empty)
        rows.append(out)
    return '/'.join(rows) + ' ' + turn + ' ' + (castle or '-') + ' ' + ep + ' ' + str(half) + ' ' + str(full)

def to_sq(name):   return (8 - int(name[1]), ord(name[0]) - ord('a'))
def sq_nm(r, c):   return chr(ord('a') + c) + str(8 - r)
def iw(p):         return p != '.' and p.isupper()
def ib(p):         return p != '.' and p.islower()

def can_slide(b, fr, fc, tr, tc, sr, sc):
    r, c = fr + sr, fc + sc
    while (r, c) != (tr, tc):
        if not (0 <= r < 8 and 0 <= c < 8): return False
        if b[r][c] != '.': return False
        r += sr; c += sc
    return True

def can_move(b, fr, fc, tr, tc, w, ep):
    p = b[fr][fc]; pt = p.upper()
    d = b[tr][tc]
    if w and iw(d): return False
    if not w and ib(d): return False
    dr, dc = tr-fr, tc-fc
    if pt == 'P':
        fwd = -1 if w else 1; sr = 6 if w else 1
        if dc == 0:
            if dr == fwd and d == '.': return True
            if dr == 2*fwd and fr == sr and d == '.' and b[fr+fwd][fc] == '.': return True
        elif abs(dc) == 1 and dr == fwd:
            if (w and ib(d)) or (not w and iw(d)): return True
            if ep != '-' and to_sq(ep) == (tr, tc): return True
        return False
    if pt == 'N': return (abs(dr), abs(dc)) in [(2,1),(1,2)]
    if pt == 'K': return abs(dr) <= 1 and abs(dc) <= 1
    if pt in ('B','Q') and abs(dr)==abs(dc) and dr:
        return can_slide(b, fr, fc, tr, tc, 1 if dr>0 else -1, 1 if dc>0 else -1)
    if pt in ('R','Q') and (dr==0 or dc==0):
        return can_slide(b, fr, fc, tr, tc, 0 if dr==0 else (1 if dr>0 else -1), 0 if dc==0 else (1 if dc>0 else -1))
    return False

def find_pt(b, pt, w):
    t = pt.upper() if w else pt.lower()
    return [(r,c) for r in range(8) for c in range(8) if b[r][c]==t]

def apply_san(state, san):
    board, turn, castle, ep, half, full = state
    b = [row[:] for row in board]; w = (turn=='w')
    nep = '-'; nhalf = half+1
    clean = re.sub(r'[+#!?]+$', '', san)
    if clean in ('O-O','0-0'):
        r = 7 if w else 0
        b[r][4]='.'; b[r][5]='R' if w else 'r'
        b[r][6]='K' if w else 'k'; b[r][7]='.'
        nc = castle.replace('K' if w else 'k','').replace('Q' if w else 'q','') or '-'
        return [b,'b' if w else 'w',nc,'-',nhalf,full+(0 if w else 1)]
    if clean in ('O-O-O','0-0-0'):
        r = 7 if w else 0
        b[r][4]='.'; b[r][0]='.'
        b[r][2]='K' if w else 'k'; b[r][3]='R' if w else 'r'
        nc = castle.replace('K' if w else 'k','').replace('Q' if w else 'q','') or '-'
        return [b,'b' if w else 'w',nc,'-',nhalf,full+(0 if w else 1)]
    promo = None
    if '=' in clean: promo=clean[-1]; clean=clean[:-2]
    pt = clean[0] if (clean[0].isupper() and clean[0]!='x') else 'P'
    rest = (clean[1:] if pt!='P' else clean).replace('x','')
    tr, tc = to_sq(rest[-2:]); dis = rest[:-2]
    cands = [(r,c) for r,c in find_pt(b,pt,w) if can_move(b,r,c,tr,tc,w,ep)]
    if dis:
        if dis[0].isalpha(): cands=[x for x in cands if x[1]==ord(dis[0])-ord('a')]
        if dis[0].isdigit(): cands=[x for x in cands if x[0]==8-int(dis[0])]
        if len(dis)==2: cands=[x for x in cands if x==(8-int(dis[1]),ord(dis[0])-ord('a'))]
    if not cands: raise ValueError("No piece for " + san)
    fr,fc = cands[0]; cap=b[tr][tc]
    if pt=='P' and ep!='-' and to_sq(ep)==(tr,tc): b[fr][tc]='.'
    piece=b[fr][fc]; b[fr][fc]='.'
    b[tr][tc]=(promo.upper() if w else promo.lower()) if promo else piece
    if pt=='P' and abs(tr-fr)==2: nep=sq_nm((fr+tr)//2,fc)
    if pt=='P' or cap!='.': nhalf=0
    nc=castle
    if pt=='K': nc=nc.replace('K' if w else 'k','').replace('Q' if w else 'q','')
    if pt=='R':
        if w:
            if (fr,fc)==(7,7): nc=nc.replace('K','')
            if (fr,fc)==(7,0): nc=nc.replace('Q','')
        else:
            if (fr,fc)==(0,7): nc=nc.replace('k','')
            if (fr,fc)==(0,0): nc=nc.replace('q','')
    nc=nc or '-'
    return [b,'b' if w else 'w',nc,nep,nhalf,full+(0 if w else 1)]

def fen_from_pgn(pgn_str, ply):
    state = fen_to_state(INIT_FEN)
    count = 0
    for tok in pgn_str.split():
        if count >= ply: break
        if re.match(r'^\d+\.+$', tok) or tok in ('1-0','0-1','1/2-1/2','*'): continue
        state = apply_san(state, tok)
        count += 1
    return state_to_fen(state)

# ── Fetch puzzle from Lichess ─────────────────────────────────────────────────

data = None
if theme:
    url = "https://lichess.org/api/puzzle/next?angle=" + theme
    try:
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        if e.code != 404:
            print("ERROR:HTTP " + str(e.code) + ": " + str(e.reason), flush=True)
            sys.exit(1)
    except urllib.error.URLError as e:
        print("ERROR:Network error: " + str(e.reason), flush=True)
        sys.exit(1)

if not data:
    url = "https://lichess.org/api/puzzle/daily"
    try:
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode())
    except urllib.error.URLError as e:
        print("ERROR:Network: " + str(e.reason), flush=True)
        sys.exit(1)
    except Exception as e:
        print("ERROR:" + str(e), flush=True)
        sys.exit(1)

puzzle = data.get("puzzle", {})
game   = data.get("game", {})
pid      = puzzle.get("id", "?")
rating   = puzzle.get("rating", 1500)
solution = puzzle.get("solution", [])
themes   = puzzle.get("themes", [])
ply      = puzzle.get("initialPly", 0)
pgn_str  = game.get("pgn", "")

if not solution:
    print("ERROR:No solution moves in response", flush=True)
    sys.exit(1)

fen = puzzle.get("fen") or puzzle.get("initialFen") or game.get("fen") or ""

if not fen and pgn_str and ply > 0:
    try:
        fen = fen_from_pgn(pgn_str, ply)
    except Exception as e:
        print("ERROR:PGN replay failed at ply " + str(ply) + ": " + str(e), flush=True)
        sys.exit(1)

if not fen:
    print("ERROR:Could not determine puzzle position (id=" + pid + ", ply=" + str(ply) + ")", flush=True)
    sys.exit(1)

print("id=" + pid, flush=True)
print("rating=" + str(rating), flush=True)
print("fen=" + fen, flush=True)
print("moves=" + ' '.join(solution), flush=True)
print("themes=" + ' '.join(themes), flush=True)
print("OK", flush=True)
"#;

// ── Fetch function ────────────────────────────────────────────────────────────

pub fn fetch_puzzle(token: &str, theme: &str, _rating_min: u32, _rating_max: u32) -> Result<Puzzle, String> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    rlog!("[puzzle] fetching via python3 theme={}", theme);

    let mut child = Command::new("python3")
        .arg("-")
        .arg(token)
        .arg(theme)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("python3 not found: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(FETCH_SCRIPT.as_bytes());
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("python3 error: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    rlog!("[puzzle] stdout: {}", stdout.trim());
    if !stderr.is_empty() { rlog!("[puzzle] stderr: {}", stderr.trim()); }

    for line in stdout.lines() {
        if let Some(msg) = line.strip_prefix("ERROR:") {
            return Err(msg.trim().to_string());
        }
    }
    if !stdout.contains("OK") {
        return Err(format!("Unexpected output: {}", stdout.chars().take(120).collect::<String>()));
    }

    let mut id = "?".to_string(); let mut fen = String::new();
    let mut moves = vec![]; let mut rating = 1500u32; let mut themes = vec![];

    for line in stdout.lines() {
        if let Some(v) = line.strip_prefix("id=")     { id     = v.trim().to_string(); }
        if let Some(v) = line.strip_prefix("fen=")    { fen    = v.trim().to_string(); }
        if let Some(v) = line.strip_prefix("rating=") { rating = v.trim().parse().unwrap_or(1500); }
        if let Some(v) = line.strip_prefix("moves=")  { moves  = v.split_whitespace().map(|s| s.to_string()).collect(); }
        if let Some(v) = line.strip_prefix("themes=") { themes = v.split_whitespace().map(|s| s.to_string()).collect(); }
    }

    if fen.is_empty()   { return Err("FEN missing from response".into()); }
    if moves.is_empty() { return Err("Moves missing from response".into()); }

    rlog!("[puzzle] ok id={} rating={} moves={}", id, rating, moves.len());
    Ok(Puzzle { id, fen, moves, rating, themes, solution_start: 1 })
}

pub fn fetch_daily_puzzle(token: &str) -> Result<Puzzle, String> {
    fetch_puzzle(token, "", 0, 9999)
}

// ── FEN parser ────────────────────────────────────────────────────────────────

pub fn parse_fen(fen: &str) -> Result<(Board, Color, Option<(usize,usize)>, Castle, u32), String> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() < 2 { return Err("FEN too short".into()); }
    let mut board: Board = [[None; 8]; 8];
    let ranks: Vec<&str> = parts[0].split('/').collect();
    if ranks.len() != 8 { return Err("FEN must have 8 ranks".into()); }
    for (r, rank) in ranks.iter().enumerate() {
        let mut c = 0usize;
        for ch in rank.chars() {
            if ch.is_ascii_digit() {
                c += ch as usize - '0' as usize;
            } else {
                use crate::engine::{Piece, Color as PC};
                let color = if ch.is_uppercase() { PC::White } else { PC::Black };
                let kind  = match ch.to_ascii_lowercase() {
                    'k'=>Kind::K,'q'=>Kind::Q,'r'=>Kind::R,
                    'b'=>Kind::B,'n'=>Kind::N,'p'=>Kind::P,
                    x => return Err(format!("Unknown piece '{x}'")),
                };
                if c >= 8 { return Err("FEN rank overflow".into()); }
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
        let col = (b[0].wrapping_sub(b'a')) as usize;
        let row = 8usize.checked_sub((b[1] - b'0') as usize)?;
        if col < 8 && row < 8 { Some((row, col)) } else { None }
    });
    let fullmove = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);
    Ok((board, turn, ep, cast, fullmove))
}