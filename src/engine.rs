// src/engine.rs
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum Color{White,Black}
impl Color{
    pub fn opp(self)->Self{match self{Color::White=>Color::Black,Color::Black=>Color::White}}
    pub fn name(self)->&'static str{match self{Color::White=>"WHITE",Color::Black=>"BLACK"}}
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum Kind{P,N,B,R,Q,K}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub struct Piece{pub c:Color,pub k:Kind}
impl Piece{
    pub fn sym(self)->&'static str{
        match(self.c,self.k){
            (Color::White,Kind::K)=>"♔",(Color::White,Kind::Q)=>"♕",
            (Color::White,Kind::R)=>"♖",(Color::White,Kind::B)=>"♗",
            (Color::White,Kind::N)=>"♘",(Color::White,Kind::P)=>"♙",
            (Color::Black,Kind::K)=>"♚",(Color::Black,Kind::Q)=>"♛",
            (Color::Black,Kind::R)=>"♜",(Color::Black,Kind::B)=>"♝",
            (Color::Black,Kind::N)=>"♞",(Color::Black,Kind::P)=>"♟",
        }
    }
}
pub type Board=[[Option<Piece>;8];8];
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub struct Mv{pub fr:(usize,usize),pub to:(usize,usize),pub promo:Option<Kind>,pub ep:bool,pub castle:u8}
impl Mv{
    pub fn basic(fr:(usize,usize),to:(usize,usize))->Self{Self{fr,to,promo:None,ep:false,castle:0}}
}
#[derive(Clone,Copy,Debug)]
pub struct Castle{pub wk:bool,pub wq:bool,pub bk:bool,pub bq:bool}
impl Castle{
    pub fn all()->Self{Self{wk:true,wq:true,bk:true,bq:true}}
    pub fn ks(self,c:Color)->bool{if c==Color::White{self.wk}else{self.bk}}
    pub fn qs(self,c:Color)->bool{if c==Color::White{self.wq}else{self.bq}}
    pub fn revoke_all(&mut self,c:Color){if c==Color::White{self.wk=false;self.wq=false;}else{self.bk=false;self.bq=false;}}
    pub fn revoke_ks(&mut self,c:Color){if c==Color::White{self.wk=false;}else{self.bk=false;}}
    pub fn revoke_qs(&mut self,c:Color){if c==Color::White{self.wq=false;}else{self.bq=false;}}
}
pub fn start_board()->Board{
    let mut b:Board=[[None;8];8];
    let back=[Kind::R,Kind::N,Kind::B,Kind::Q,Kind::K,Kind::B,Kind::N,Kind::R];
    for(c,&k) in back.iter().enumerate(){
        b[0][c]=Some(Piece{c:Color::Black,k});b[7][c]=Some(Piece{c:Color::White,k});
    }
    for c in 0..8{b[1][c]=Some(Piece{c:Color::Black,k:Kind::P});b[6][c]=Some(Piece{c:Color::White,k:Kind::P});}
    b
}
fn ib(r:i32,c:i32)->bool{r>=0&&r<8&&c>=0&&c<8}
fn slide(b:&Board,r:usize,c:usize,col:Color,dirs:&[(i32,i32)],out:&mut Vec<Mv>){
    for&(dr,dc) in dirs{
        let(mut nr,mut nc)=(r as i32+dr,c as i32+dc);
        while ib(nr,nc){
            let(ur,uc)=(nr as usize,nc as usize);
            match b[ur][uc]{
                Some(p) if p.c==col=>break,
                Some(_)=>{out.push(Mv::basic((r,c),(ur,uc)));break;}
                None=>out.push(Mv::basic((r,c),(ur,uc))),
            }
            nr+=dr;nc+=dc;
        }
    }
}
pub fn pseudo(b:&Board,r:usize,c:usize,ep:Option<(usize,usize)>,cast:&Castle,allow_castle:bool)->Vec<Mv>{
    let p=match b[r][c]{Some(p)=>p,None=>return vec![]};
    let mut out=vec![];
    match p.k{
        Kind::P=>{
            let dir:i32=if p.c==Color::White{-1}else{1};
            let sr=if p.c==Color::White{6}else{1};
            let pr=if p.c==Color::White{0}else{7};
            let nr=r as i32+dir;
            if ib(nr,c as i32){
                let ur=nr as usize;
                if b[ur][c].is_none(){
                    if ur==pr{for k in[Kind::Q,Kind::R,Kind::B,Kind::N]{out.push(Mv{fr:(r,c),to:(ur,c),promo:Some(k),ep:false,castle:0});}}
                    else{
                        out.push(Mv::basic((r,c),(ur,c)));
                        let nr2=ur as i32+dir;
                        if r==sr&&ib(nr2,c as i32)&&b[nr2 as usize][c].is_none(){out.push(Mv::basic((r,c),(nr2 as usize,c)));}
                    }
                }
                for dc in[-1i32,1]{
                    let nc2=c as i32+dc;if!ib(nr,nc2){continue;}
                    let(ur2,uc2)=(nr as usize,nc2 as usize);
                    let is_ep=ep.map(|e|e==(ur2,uc2)).unwrap_or(false);
                    let is_cap=b[ur2][uc2].map(|q|q.c!=p.c).unwrap_or(false);
                    if is_cap||is_ep{
                        if ur2==pr{for k in[Kind::Q,Kind::R,Kind::B,Kind::N]{out.push(Mv{fr:(r,c),to:(ur2,uc2),promo:Some(k),ep:is_ep,castle:0});}}
                        else{out.push(Mv{fr:(r,c),to:(ur2,uc2),promo:None,ep:is_ep,castle:0});}
                    }
                }
            }
        }
        Kind::N=>{for(dr,dc) in[(2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2)]{
            let(nr,nc)=(r as i32+dr,c as i32+dc);if!ib(nr,nc){continue;}
            let(ur,uc)=(nr as usize,nc as usize);
            if b[ur][uc].map(|q|q.c==p.c).unwrap_or(false){continue;}
            out.push(Mv::basic((r,c),(ur,uc)));
        }}
        Kind::B=>slide(b,r,c,p.c,&[(1,1),(1,-1),(-1,1),(-1,-1)],&mut out),
        Kind::R=>slide(b,r,c,p.c,&[(1,0),(-1,0),(0,1),(0,-1)],&mut out),
        Kind::Q=>slide(b,r,c,p.c,&[(1,1),(1,-1),(-1,1),(-1,-1),(1,0),(-1,0),(0,1),(0,-1)],&mut out),
        Kind::K=>{
            for dr in -1i32..=1{for dc in -1i32..=1{
                if dr==0&&dc==0{continue;}
                let(nr,nc)=(r as i32+dr,c as i32+dc);if!ib(nr,nc){continue;}
                let(ur,uc)=(nr as usize,nc as usize);
                if b[ur][uc].map(|q|q.c==p.c).unwrap_or(false){continue;}
                out.push(Mv::basic((r,c),(ur,uc)));
            }}
            if allow_castle{
                let kr=if p.c==Color::White{7}else{0};let opp=p.c.opp();
                if r==kr&&c==4{
                    if cast.ks(p.c)&&b[kr][5].is_none()&&b[kr][6].is_none()
                        &&!attacked(b,kr,4,opp)&&!attacked(b,kr,5,opp)&&!attacked(b,kr,6,opp){
                        out.push(Mv{fr:(r,c),to:(kr,6),promo:None,ep:false,castle:1});
                    }
                    if cast.qs(p.c)&&b[kr][3].is_none()&&b[kr][2].is_none()&&b[kr][1].is_none()
                        &&!attacked(b,kr,4,opp)&&!attacked(b,kr,3,opp)&&!attacked(b,kr,2,opp){
                        out.push(Mv{fr:(r,c),to:(kr,2),promo:None,ep:false,castle:2});
                    }
                }
            }
        }
    }
    out
}
pub fn attacked(b:&Board,r:usize,c:usize,by:Color)->bool{
    let nc=Castle{wk:false,wq:false,bk:false,bq:false};
    for br in 0..8{for bc in 0..8{
        if let Some(p)=b[br][bc]{
            if p.c==by&&pseudo(b,br,bc,None,&nc,false).iter().any(|m|m.to==(r,c)){return true;}
        }
    }}
    false
}
pub fn king_sq(b:&Board,c:Color)->Option<(usize,usize)>{
    for r in 0..8{for cc in 0..8{if let Some(p)=b[r][cc]{if p.c==c&&p.k==Kind::K{return Some((r,cc));}}}}
    None
}
pub fn in_check(b:&Board,c:Color)->bool{king_sq(b,c).map(|(r,cc)|attacked(b,r,cc,c.opp())).unwrap_or(false)}
pub fn apply(b:&Board,mv:&Mv,_ep:Option<(usize,usize)>,cast:&Castle)->(Board,Option<(usize,usize)>,Castle){
    let mut nb=*b;let mut nc=*cast;
    let p=nb[mv.fr.0][mv.fr.1].unwrap();
    let mut new_ep=None;
    if mv.ep{let cap_r=if p.c==Color::White{mv.to.0+1}else{mv.to.0.wrapping_sub(1)};nb[cap_r][mv.to.1]=None;}
    if p.k==Kind::P&&(mv.to.0 as i32-mv.fr.0 as i32).abs()==2{new_ep=Some(((mv.fr.0+mv.to.0)/2,mv.fr.1));}
    match mv.castle{1=>{nb[mv.fr.0][5]=nb[mv.fr.0][7];nb[mv.fr.0][7]=None;}2=>{nb[mv.fr.0][3]=nb[mv.fr.0][0];nb[mv.fr.0][0]=None;}_=>{}}
    if p.k==Kind::K{nc.revoke_all(p.c);}
    if p.k==Kind::R{match(p.c,mv.fr){(Color::White,(7,7))=>nc.revoke_ks(Color::White),(Color::White,(7,0))=>nc.revoke_qs(Color::White),(Color::Black,(0,7))=>nc.revoke_ks(Color::Black),(Color::Black,(0,0))=>nc.revoke_qs(Color::Black),_=>{}}}
    match mv.to{(7,7)=>nc.revoke_ks(Color::White),(7,0)=>nc.revoke_qs(Color::White),(0,7)=>nc.revoke_ks(Color::Black),(0,0)=>nc.revoke_qs(Color::Black),_=>{}}
    let placed=mv.promo.map(|k|Piece{c:p.c,k}).unwrap_or(p);
    nb[mv.to.0][mv.to.1]=Some(placed);nb[mv.fr.0][mv.fr.1]=None;
    (nb,new_ep,nc)
}
pub fn legal(b:&Board,color:Color,ep:Option<(usize,usize)>,cast:&Castle)->Vec<Mv>{
    let mut out=vec![];
    for r in 0..8{for c in 0..8{
        if let Some(p)=b[r][c]{if p.c==color{
            for mv in pseudo(b,r,c,ep,cast,true){
                let(nb,_,_)=apply(b,&mv,ep,cast);
                if!in_check(&nb,color){out.push(mv);}
            }
        }}
    }}
    out
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum Status{Active,Check,Checkmate,Stalemate}
pub fn game_status(b:&Board,color:Color,ep:Option<(usize,usize)>,cast:&Castle)->Status{
    let moves=legal(b,color,ep,cast);let chk=in_check(b,color);
    match(moves.is_empty(),chk){(true,true)=>Status::Checkmate,(true,false)=>Status::Stalemate,(false,true)=>Status::Check,_=>Status::Active}
}

/// Convert a move to SAN notation (for PGN export).
/// Requires the board state *before* the move is applied.
pub fn mv_to_san(board: &Board, mv: &Mv, ep: Option<(usize,usize)>, cast: &Castle) -> String {
    if mv.castle == 1 { return "O-O".to_string(); }
    if mv.castle == 2 { return "O-O-O".to_string(); }

    let piece = match board[mv.fr.0][mv.fr.1] { Some(p) => p, None => return "??".to_string() };
    let is_cap = board[mv.to.0][mv.to.1].is_some() || mv.ep;
    let mut s = String::new();

    // Piece letter (pawns get none)
    s.push_str(match piece.k {
        Kind::P => "", Kind::N => "N", Kind::B => "B",
        Kind::R => "R", Kind::Q => "Q", Kind::K => "K",
    });

    // Disambiguation for pieces other than pawns/kings
    if !matches!(piece.k, Kind::P | Kind::K) {
        let ambiguous: Vec<Mv> = legal(board, piece.c, ep, cast)
            .into_iter()
            .filter(|m| m.to == mv.to && m.fr != mv.fr
                && board[m.fr.0][m.fr.1].map(|p| p.k == piece.k).unwrap_or(false))
            .collect();
        if !ambiguous.is_empty() {
            let same_col = ambiguous.iter().any(|m| m.fr.1 == mv.fr.1);
            let same_row = ambiguous.iter().any(|m| m.fr.0 == mv.fr.0);
            if !same_col {
                s.push((b'a' + mv.fr.1 as u8) as char);
            } else if !same_row {
                s.push(char::from_digit((8 - mv.fr.0) as u32, 10).unwrap_or('?'));
            } else {
                s.push((b'a' + mv.fr.1 as u8) as char);
                s.push(char::from_digit((8 - mv.fr.0) as u32, 10).unwrap_or('?'));
            }
        }
    }

    // Pawn capture: include source file
    if piece.k == Kind::P && is_cap {
        s.push((b'a' + mv.fr.1 as u8) as char);
    }

    if is_cap { s.push('x'); }

    s.push((b'a' + mv.to.1 as u8) as char);
    s.push(char::from_digit((8 - mv.to.0) as u32, 10).unwrap_or('?'));

    if let Some(k) = mv.promo {
        s.push('=');
        s.push(match k { Kind::Q=>'Q', Kind::R=>'R', Kind::B=>'B', _=>'N' });
    }
    s
}

// ── Zobrist hashing ───────────────────────────────────────────────────────────
pub fn piece_index(c: Color, k: Kind) -> usize {
    let ki = match k { Kind::P=>0, Kind::N=>1, Kind::B=>2, Kind::R=>3, Kind::Q=>4, Kind::K=>5 };
    match c { Color::White => ki, Color::Black => ki + 6 }
}

struct XorShift64(u64);
impl XorShift64 {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

struct ZobristTable {
    pieces: [[u64; 64]; 12],
    side:   u64,
    ep:     [u64; 8],
    castle: [u64; 4],
}

use std::sync::OnceLock;
static ZOBRIST: OnceLock<ZobristTable> = OnceLock::new();

fn init_zobrist() -> ZobristTable {
    let mut rng = XorShift64(1804289383);
    let mut pieces = [[0u64; 64]; 12];
    for set in pieces.iter_mut() {
        for sq in set.iter_mut() { *sq = rng.next(); }
    }
    ZobristTable { pieces, side: rng.next(), ep: [rng.next(),rng.next(),rng.next(),rng.next(),rng.next(),rng.next(),rng.next(),rng.next()], castle: [rng.next(),rng.next(),rng.next(),rng.next()] }
}

pub fn zobrist_hash(b: &Board, stm: Color, ep: Option<(usize,usize)>, cast: &Castle) -> u64 {
    let z = ZOBRIST.get_or_init(init_zobrist);
    let mut h = 0u64;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = b[r][c] {
                h ^= z.pieces[piece_index(p.c, p.k)][r * 8 + c];
            }
        }
    }
    if stm == Color::Black { h ^= z.side; }
    if let Some((_, cf)) = ep { h ^= z.ep[cf]; }
    if cast.wk { h ^= z.castle[0]; } if cast.wq { h ^= z.castle[1]; }
    if cast.bk { h ^= z.castle[2]; } if cast.bq { h ^= z.castle[3]; }
    h
}
