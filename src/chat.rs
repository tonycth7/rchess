// src/chat.rs — Chat UI + encryption for online multiplayer

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum Sender { Me, Opponent, Server }

impl Sender {
    pub fn label(&self) -> &'static str {
        match self { Sender::Me => "You", Sender::Opponent => "Opp", Sender::Server => "Srv" }
    }
    pub fn color(&self) -> Color {
        match self {
            Sender::Me       => Color::Rgb(100, 220, 130),
            Sender::Opponent => Color::Rgb(110, 180, 255),
            Sender::Server   => Color::Rgb(220, 160, 255),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChatMsg {
    pub sender: Sender,
    pub text:   String,
    pub tick:   u64,
}

// ── Chat state ────────────────────────────────────────────────────────────────

pub struct ChatState {
    pub messages: Vec<ChatMsg>,
    pub input:    String,
    pub focused:  bool,
    pub unread:   usize,
    tick:         u64,
    pub capacity: usize,
    /// Opponent display name shown in chat header
    pub opp_name: String,
}

impl ChatState {
    pub fn new() -> Self {
        ChatState {
            messages: vec![], input: String::new(),
            focused: false, unread: 0, tick: 0,
            capacity: 300, opp_name: String::new(),
        }
    }

    pub fn tick(&mut self) { self.tick += 1; }

    pub fn clear(&mut self) {
        self.messages.clear(); self.input.clear();
        self.focused = false; self.unread = 0; self.tick = 0;
        self.opp_name.clear();
    }

    pub fn push_opponent(&mut self, text: String) {
        self.push(Sender::Opponent, text);
        if !self.focused { self.unread += 1; }
    }
    pub fn push_server(&mut self, text: String) {
        self.push(Sender::Server, text);
        if !self.focused { self.unread += 1; }
    }
    pub fn push_me(&mut self, text: String)  { self.push(Sender::Me, text); }

    /// Push a historical message (replayed on rejoin) without incrementing unread.
    pub fn push_history(&mut self, sender: Sender, text: String) {
        if self.messages.len() >= self.capacity { self.messages.remove(0); }
        self.messages.push(ChatMsg { sender, text, tick: 0 });
    }

    fn push(&mut self, sender: Sender, text: String) {
        if self.messages.len() >= self.capacity { self.messages.remove(0); }
        self.messages.push(ChatMsg { sender, text, tick: self.tick });
    }

    pub fn open(&mut self)  { self.focused = true;  self.unread = 0; }
    pub fn close(&mut self) { self.focused = false; }
    pub fn toggle(&mut self) { if self.focused { self.close(); } else { self.open(); } }
    pub fn has_unread(&self) -> bool { self.unread > 0 }

    pub fn badge(&self) -> String {
        if self.unread > 0 { format!(" [{}]", self.unread) }
        else if !self.messages.is_empty() { " [chat]".to_string() }
        else { String::new() }
    }

    /// Returns Some(plaintext) when user presses Enter to send.
    pub fn handle_key(&mut self, code: KeyCode) -> ChatKeyResult {
        if self.focused {
            match code {
                KeyCode::Esc => { self.close(); ChatKeyResult::Closed }
                KeyCode::Enter => {
                    let text = self.input.trim().to_string();
                    if text.is_empty() { return ChatKeyResult::None; }
                    self.input.clear();
                    ChatKeyResult::Send(text)
                }
                KeyCode::Backspace => { self.input.pop(); ChatKeyResult::None }
                KeyCode::Char(c) if self.input.len() < 400 => {
                    self.input.push(c); ChatKeyResult::None
                }
                _ => ChatKeyResult::None,
            }
        } else {
            match code {
                KeyCode::Char('c') | KeyCode::Char('C') => { self.open(); ChatKeyResult::Opened }
                _ => ChatKeyResult::None,
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ChatKeyResult { None, Send(String), Opened, Closed }

// ── Encryption (XOR + hex, keyed from room code) ──────────────────────────────

pub fn derive_key(room_code: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let src = room_code.as_bytes();
    let mut h: u64 = 0x9e3779b97f4a7c15;
    for (i, slot) in key.iter_mut().enumerate() {
        let b = src[i % src.len().max(1)];
        h = h.wrapping_mul(0x6c62272e07bb0142)
             .wrapping_add((b as u64) ^ (i as u64).wrapping_mul(0x2545f4914f6cdd1d));
        *slot = (h ^ (h >> 32)) as u8;
    }
    key
}

pub fn encrypt(plaintext: &str, room_code: &str) -> String {
    if room_code.is_empty() { return plaintext.to_string(); }
    let key = derive_key(room_code);
    plaintext.as_bytes().iter().enumerate()
        .map(|(i, &b)| format!("{:02x}", b ^ key[i % 32]))
        .collect()
}

pub fn decrypt(hex: &str, room_code: &str) -> Option<String> {
    // If no room code yet, return as-is
    if room_code.is_empty() { return Some(hex.to_string()); }

    // Attempt XOR decryption: payload must be even-length hex
    if hex.len() >= 2 && hex.len() % 2 == 0 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let key = derive_key(room_code);
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        let hex_b = hex.as_bytes();
        let mut ok = true;
        let mut i = 0;
        while i + 1 < hex_b.len() {
            let hi = hex_nibble(hex_b[i]);
            let lo = hex_nibble(hex_b[i+1]);
            if hi > 15 || lo > 15 { ok = false; break; }
            bytes.push((hi << 4) | lo);
            i += 2;
        }
        if ok {
            let out: Vec<u8> = bytes.iter().enumerate().map(|(i, &b)| b ^ key[i % 32]).collect();
            if let Ok(s) = String::from_utf8(out) {
                // Valid decryption: all printable ASCII
                if !s.is_empty() && s.chars().all(|c| c >= ' ' || c == '\t') {
                    return Some(s);
                }
            }
        }
    }

    // Not encrypted (server system message, or empty room code) — return verbatim
    Some(hex.to_string())
}

fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 255,
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

pub fn render(chat: &ChatState, area: Rect, f: &mut Frame, accent: Color, my_color: &str) {
    let [msgs_area, input_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
    ]).areas(area);

    let visible = msgs_area.height.saturating_sub(2) as usize;
    let start   = chat.messages.len().saturating_sub(visible);

    let lines: Vec<Line> = chat.messages[start..].iter().map(|m| {
        let label_width = 4usize;
        let label = format!("{:>label_width$}", m.sender.label());
        let dim_col = Color::Rgb(60, 70, 60);
        Line::from(vec![
            Span::styled("│", Style::default().fg(dim_col)),
            Span::styled(
                format!(" {} ", label),
                Style::default().fg(m.sender.color()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(dim_col)),
            Span::styled(m.text.clone(), Style::default().fg(Color::Rgb(215, 205, 185))),
        ])
    }).collect();

    let empty_hint = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  No messages — press C to open chat",
            Style::default().fg(Color::Rgb(55, 70, 55)),
        )]),
    ];

    let opp_label = if chat.opp_name.is_empty() {
        if my_color == "white" { "vs Black".to_string() } else { "vs White".to_string() }
    } else {
        format!("vs {}", chat.opp_name)
    };

    let title = format!(" CHAT {} {} ",
        opp_label,
        if chat.has_unread() { format!("({} new)", chat.unread) } else { String::new() }
    );

    let border_col = if chat.focused { accent } else { Color::Rgb(40, 55, 40) };

    f.render_widget(
        Paragraph::new(if lines.is_empty() { empty_hint } else { lines })
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_col))
                .title(Span::styled(title, Style::default().fg(accent).add_modifier(Modifier::BOLD)))),
        msgs_area,
    );

    // Input bar
    let (bc, content) = if chat.focused {
        (accent, format!(" {}▌", chat.input))
    } else {
        (Color::Rgb(40, 55, 40), "  [C] chat  [Esc] close".to_string())
    };
    let input_style = if chat.focused {
        Style::default().fg(Color::Rgb(230, 220, 190))
    } else {
        Style::default().fg(Color::Rgb(70, 90, 70))
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(content, input_style)))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(bc))),
        input_area,
    );
}

pub fn render_badge(chat: &ChatState, f: &mut Frame, area: Rect) {
    if chat.messages.is_empty() { return; }
    let Some(last) = chat.messages.last() else { return };
    let preview: String = last.text.chars().take(35).collect();
    let ellipsis = if last.text.len() > 35 { "…" } else { "" };
    let text = format!("[{}] {}{}", last.sender.label(), preview, ellipsis);
    f.render_widget(
        Paragraph::new(Span::styled(text, Style::default().fg(Color::Rgb(100, 200, 130)))),
        area,
    );
}
