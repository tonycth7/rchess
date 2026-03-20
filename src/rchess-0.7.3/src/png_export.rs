// src/png_export.rs — board-to-PNG renderer (Python Pillow subprocess)
//
// Tries Unicode chess symbols (♔♕♖♗♘♙♚♛♜♝♞♟) first using fonts that
// have these glyphs. Falls back to geometric circle+letter if no suitable
// font is found. Works out of the box on Arch Linux with any of:
//   sudo pacman -S ttf-freefont          ← best chess glyph coverage
//   sudo pacman -S noto-fonts            ← very good Unicode coverage
//   sudo pacman -S ttf-dejavu            ← good fallback
//   sudo pacman -S ttf-liberation        ← another good option
//
// Pillow itself:  sudo pacman -S python-pillow

use std::path::PathBuf;
use std::process::Command;
use crate::config::Theme;
use crate::engine::{Board, Color as PC, Kind};

pub fn render_board_png(
    board:     &Board,
    theme:     Theme,
    flipped:   bool,
    last_from: Option<(usize, usize)>,
    last_to:   Option<(usize, usize)>,
    out_path:  &PathBuf,
) -> bool {
    let mut board_str = String::with_capacity(64);
    for r in 0..8usize {
        for c in 0..8usize {
            let ch = match board[r][c] {
                None => '.',
                Some(p) => {
                    let b = match p.k {
                        Kind::K=>'k', Kind::Q=>'q', Kind::R=>'r',
                        Kind::B=>'b', Kind::N=>'n', Kind::P=>'p',
                    };
                    if p.c == PC::White { b.to_ascii_uppercase() } else { b }
                }
            };
            board_str.push(ch);
        }
    }

    let lf = last_from.map(|(r,c)| r*8+c).unwrap_or(64);
    let lt = last_to  .map(|(r,c)| r*8+c).unwrap_or(64);

    let (ls, ds) = theme.squares();
    let lm       = theme.last_move();
    let (bg, _)  = theme.bg_colors();
    let acc      = theme.accent();
    let (wp, bp) = theme.piece_colors();
    let out_str  = out_path.to_string_lossy().to_string();

    let script = build_script(
        &board_str, flipped, lf, lt,
        ls, ds, lm, bg, acc, wp, bp, &out_str,
    );

    let ts = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    };
    let tmp = std::env::temp_dir().join(format!("rchess_render_{}.py", ts));
    if std::fs::write(&tmp, &script).is_err() {
        crate::rlog!("[rchess/png] failed to write temp script");
        return false;
    }

    let result = Command::new("python3").arg(&tmp).output();
    let _ = std::fs::remove_file(&tmp);

    match result {
        Ok(o) => {
            if o.status.success() {
                crate::rlog!("[rchess/png] saved: {}", out_path.display());
                true
            } else {
                let e = String::from_utf8_lossy(&o.stderr);
                crate::rlog!("[rchess/png] python3 error: {}", e.trim());
                false
            }
        }
        Err(e) => {
            crate::rlog!("[rchess/png] spawn failed: {} — install python-pillow", e);
            false
        }
    }
}

fn build_script(
    board_str: &str, flipped: bool,
    lf: usize, lt: usize,
    ls: (u8,u8,u8), ds: (u8,u8,u8), lm: (u8,u8,u8),
    bg: (u8,u8,u8), acc: (u8,u8,u8),
    wp: (u8,u8,u8), bp: (u8,u8,u8),
    out: &str,
) -> String {
    format!(
r##"from PIL import Image, ImageDraw, ImageFont
import os, sys

BOARD = "{bs}"
FLIP  = {fl}
LF, LT = {lf}, {lt}

CELL   = 96
MARGIN = 40
SIZE   = 8 * CELL + 2 * MARGIN

LS  = ({ls0},{ls1},{ls2})
DS  = ({ds0},{ds1},{ds2})
LMC = ({lm0},{lm1},{lm2})
BG  = ({bg0},{bg1},{bg2})
ACC = ({ac0},{ac1},{ac2})
WP  = ({wp0},{wp1},{wp2})
BP  = ({bp0},{bp1},{bp2})

# ── Unicode chess symbols ─────────────────────────────────────────────────────
SYMS = {{
    'K': u'\u2654', 'Q': u'\u2655', 'R': u'\u2656', 'B': u'\u2657', 'N': u'\u2658', 'P': u'\u2659',
    'k': u'\u265a', 'q': u'\u265b', 'r': u'\u265c', 'b': u'\u265d', 'n': u'\u265e', 'p': u'\u265f',
}}
# ASCII letter fallback
LETTERS = {{'K':'K','Q':'Q','R':'R','B':'B','N':'N','P':'P',
            'k':'K','q':'Q','r':'R','b':'B','n':'N','p':'P'}}

# ── Font search using fc-list (works with ANY installed font on Linux) ────────
import subprocess, glob, os

def fc_list_chess():
    """Use fontconfig to find fonts that contain chess glyphs (U+2654)."""
    try:
        out = subprocess.check_output(
            ['fc-list', ':charset=2654', '--format=%{{file}}\n'],
            stderr=subprocess.DEVNULL, timeout=3
        ).decode().strip()
        fonts = [f.strip() for f in out.split('\n') if f.strip()]
        return fonts
    except Exception:
        return []

def find_nerd_fonts():
    """Find Nerd Font TTFs — they patch the base font and keep all Unicode."""
    patterns = [
        # User installs (most common with yay/paru)
        os.path.expanduser('~/.local/share/fonts/**/*.ttf'),
        os.path.expanduser('~/.local/share/fonts/*.ttf'),
        # System-wide from AUR packages
        '/usr/share/fonts/TTF/*Nerd*.ttf',
        '/usr/share/fonts/TTF/*NF*.ttf',
        '/usr/share/fonts/nerd-fonts*/*.ttf',
        '/usr/share/fonts/**/*Nerd*.ttf',
        '/usr/share/fonts/**/*NF.ttf',
    ]
    found = []
    for p in patterns:
        found.extend(glob.glob(p, recursive=True))
    # Prefer Mono variants (better terminal rendering, same Unicode coverage)
    mono   = [f for f in found if 'Mono' in f or 'mono' in f]
    others = [f for f in found if f not in mono]
    return mono + others

# Priority order:
# 1. Fonts confirmed by fontconfig to have chess glyphs
# 2. Nerd Fonts (retain full Unicode of base font)
# 3. Known good fonts by path
UNICODE_FONTS = (
    fc_list_chess()          # best: fontconfig-confirmed chess glyphs
    + find_nerd_fonts()      # user's Nerd Fonts - keep all base Unicode
    + [
        # Arch Linux explicit paths (ttf-freefont package)
        '/usr/share/fonts/TTF/FreeSerif.ttf',
        '/usr/share/fonts/TTF/FreeSerifBold.ttf',
        '/usr/share/fonts/TTF/FreeSans.ttf',
        '/usr/share/fonts/freefont/FreeSerif.ttf',
        # Noto (noto-fonts package)
        '/usr/share/fonts/noto/NotoSerif-Regular.ttf',
        '/usr/share/fonts/noto/NotoSans-Regular.ttf',
        # Debian / Ubuntu
        '/usr/share/fonts/truetype/freefont/FreeSerif.ttf',
        '/usr/share/fonts/truetype/freefont/FreeSerifBold.ttf',
        '/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf',
        # Windows / Wine
        'C:/Windows/Fonts/seguisym.ttf',
        # macOS
        '/System/Library/Fonts/Supplemental/Arial Unicode.ttf',
    ]
)

# For coordinate labels — just needs ASCII, use user's Nerd Font for consistency
COORD_FONTS = find_nerd_fonts() + [
    '/usr/share/fonts/TTF/DejaVuSans-Bold.ttf',
    '/usr/share/fonts/TTF/DejaVuSans.ttf',
    '/usr/share/fonts/noto/NotoSans-Bold.ttf',
    '/usr/share/fonts/noto/NotoSans-Regular.ttf',
    '/usr/share/fonts/liberation/LiberationSans-Bold.ttf',
    '/usr/share/fonts/TTF/FreeSans.ttf',
    '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',
    '/System/Library/Fonts/Helvetica.ttc',
] + list(UNICODE_FONTS)

def load_font(paths, size):
    for p in paths:
        try:
            return ImageFont.truetype(p, size), p
        except Exception:
            pass
    return ImageFont.load_default(), 'default'

def has_chess_glyph(font):
    """Test if the font can render the King glyph."""
    try:
        bb = font.getbbox(u'\u2654')
        w = bb[2] - bb[0]
        return w > 4  # default font gives tiny/zero width for unknown glyphs
    except Exception:
        return False

def clamp(t):
    return tuple(max(0, min(255, v)) for v in t)

def blend(a, b, t=0.42):
    return clamp(tuple(int(av*(1-t) + bv*t) for av, bv in zip(a, b)))

def text_box(font, txt):
    try:
        bb = font.getbbox(txt)
        return bb[2]-bb[0], bb[3]-bb[1], bb[0], bb[1]
    except Exception:
        return 20, 20, 0, 0

# ── Load fonts ────────────────────────────────────────────────────────────────
pfont, pfont_path = load_font(UNICODE_FONTS, 60)
cfont, _ = load_font(COORD_FONTS, 15)

use_unicode = has_chess_glyph(pfont)

# If unicode font didn't load properly, fall back to any font for letter rendering
if not use_unicode:
    pfont, pfont_path = load_font(COORD_FONTS, 52)

# ── Build canvas ──────────────────────────────────────────────────────────────
img  = Image.new('RGB', (SIZE, SIZE), BG)
draw = ImageDraw.Draw(img)

row_ord = list(range(7, -1, -1)) if FLIP else list(range(8))
col_ord = list(range(7, -1, -1)) if FLIP else list(range(8))

# Draw squares
for vr, r in enumerate(row_ord):
    for vc, c in enumerate(col_ord):
        base = LS if (r + c) % 2 == 0 else DS
        col  = blend(base, LMC) if r*8+c in (LF, LT) else base
        px   = MARGIN + vc * CELL
        py   = MARGIN + vr * CELL
        draw.rectangle([px, py, px+CELL-1, py+CELL-1], fill=col)

# Draw pieces
for vr, r in enumerate(row_ord):
    for vc, c in enumerate(col_ord):
        ch = BOARD[r * 8 + c]
        if ch == '.':
            continue
        is_white = ch.isupper()
        px = MARGIN + vc * CELL
        py = MARGIN + vr * CELL
        cx = px + CELL // 2
        cy = py + CELL // 2

        if use_unicode:
            # ── Unicode symbol rendering ──────────────────────────────────────
            sym = SYMS[ch]
            # Draw a subtle shadow/outline
            fg = WP if is_white else BP
            shadow = clamp(tuple(max(0, v - 80) for v in fg)) if is_white else (0, 0, 0)
            tw, th, ox, oy = text_box(pfont, sym)
            tx = cx - tw // 2 - ox
            ty = cy - th // 2 - oy - 2  # slight upward nudge
            # Outline
            for ddx, ddy in [(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)]:
                draw.text((tx+ddx, ty+ddy), sym, font=pfont, fill=shadow)
            draw.text((tx, ty), sym, font=pfont, fill=fg)
        else:
            # ── Geometric circle + letter fallback ────────────────────────────
            rad = CELL // 2 - 8
            fg  = WP if is_white else BP
            rim = clamp(tuple(v - 50 for v in fg)) if is_white else clamp(tuple(v + 60 for v in BP))
            shadow = clamp(tuple(v - 70 for v in fg))
            # Drop shadow
            draw.ellipse([cx-rad+3, cy-rad+3, cx+rad+3, cy+rad+3], fill=shadow)
            # Main circle
            draw.ellipse([cx-rad, cy-rad, cx+rad, cy+rad], fill=fg, outline=rim, width=3)
            # Inner ring
            if rad > 8:
                hi = clamp(tuple(v + 30 for v in fg))
                draw.ellipse([cx-rad+6, cy-rad+6, cx+rad-6, cy+rad-6], outline=hi, width=1)
            # Letter
            letter = LETTERS[ch]
            txt_col = clamp(tuple(max(0, v - 100) for v in WP)) if is_white else WP
            tw, th, ox, oy = text_box(pfont, letter)
            tx = cx - tw // 2 - ox
            ty = cy - th // 2 - oy
            for ddx, ddy in [(-1,0),(1,0),(0,-1),(0,1)]:
                draw.text((tx+ddx, ty+ddy), letter, font=pfont, fill=shadow)
            draw.text((tx, ty), letter, font=pfont, fill=txt_col)

# Board border
bx = MARGIN - 4
bs = 8 * CELL + 7
draw.rectangle([bx, bx, bx+bs, bx+bs], outline=ACC, width=4)

# Coordinate labels
cc = clamp(tuple(v + 100 for v in BG))
def dlabel(txt, cx, cy):
    tw, th, ox, oy = text_box(cfont, txt)
    draw.text((cx - tw//2 - ox, cy - th//2 - oy), txt, font=cfont, fill=cc)

for vc, c in enumerate(col_ord):
    cx = MARGIN + vc * CELL + CELL // 2
    lbl = chr(ord('a') + c)
    dlabel(lbl, cx, MARGIN // 2)
    dlabel(lbl, cx, MARGIN + 8*CELL + MARGIN//2)

for vr, r in enumerate(row_ord):
    cy  = MARGIN + vr * CELL + CELL // 2
    lbl = str(8 - r)
    dlabel(lbl, MARGIN // 2, cy)
    dlabel(lbl, MARGIN + 8*CELL + MARGIN//2, cy)

# Save
os.makedirs(os.path.dirname("{out}") or ".", exist_ok=True)
img.save("{out}")
print("saved:", "{out}", "unicode:", use_unicode, "font:", pfont_path)
"##,
        bs  = board_str,
        fl  = if flipped { "True" } else { "False" },
        lf  = lf, lt = lt,
        ls0=ls.0,ls1=ls.1,ls2=ls.2, ds0=ds.0,ds1=ds.1,ds2=ds.2,
        lm0=lm.0,lm1=lm.1,lm2=lm.2, bg0=bg.0,bg1=bg.1,bg2=bg.2,
        ac0=acc.0,ac1=acc.1,ac2=acc.2, wp0=wp.0,wp1=wp.1,wp2=wp.2,
        bp0=bp.0,bp1=bp.1,bp2=bp.2, out=out,
    )
}
