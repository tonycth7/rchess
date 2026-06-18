// src/tt.rs — Transposition table
use crate::engine::Mv;

const TT_BITS: usize = 18;
const TT_SIZE: usize = 1 << TT_BITS;

#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum TTFlag { Exact, Alpha, Beta }

#[derive(Clone,Copy)]
struct TTEntry {
    key:   u64,
    depth: u8,
    score: i32,
    flag:  u8,
    mv:    Option<Mv>,
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        let entry = TTEntry { key: 0, depth: 0, score: 0, flag: 0, mv: None };
        Self { entries: vec![entry; TT_SIZE] }
    }

    pub fn clear(&mut self) {
        for e in self.entries.iter_mut() { e.key = 0; e.mv = None; e.flag = 0; }
    }

    pub fn probe(&self, hash: u64) -> Option<(TTFlag, u8, i32, Option<Mv>)> {
        let e = &self.entries[(hash as usize) & (TT_SIZE - 1)];
        if e.key == hash && e.flag != 0 {
            Some((match e.flag { 1 => TTFlag::Exact, 2 => TTFlag::Alpha, _ => TTFlag::Beta }, e.depth, e.score, e.mv))
        } else { None }
    }

    pub fn store(&mut self, hash: u64, depth: u8, score: i32, flag: TTFlag, mv: Option<Mv>) {
        let e = &mut self.entries[(hash as usize) & (TT_SIZE - 1)];
        e.key = hash; e.depth = depth; e.score = score;
        e.flag = match flag { TTFlag::Exact => 1, TTFlag::Alpha => 2, TTFlag::Beta => 3 };
        e.mv = mv;
    }
}
