// server/config.rs — RChess server configuration
//
// Priority order: CLI flags > config file > built-in defaults

use std::path::PathBuf;
use std::io::Write;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host:             String,
    pub port:             u16,
    pub admin_port:       u16,
    pub server_password:  Option<String>,
    pub admin_password:   Option<String>,
    pub max_rooms:        usize,
    pub max_conns_per_ip: u32,
    pub data_dir:         String,
    pub room_ttl_mins:    u64,
    pub server_name:      String,
    pub motd:             String,
    /// Public address embedded in invite links. Auto-detected if empty.
    pub public_host:      String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            host:             "0.0.0.0".into(),
            port:             9001,
            admin_port:       9002,
            server_password:  None,
            admin_password:   None,
            max_rooms:        200,
            max_conns_per_ip: 20,
            data_dir:         "./rchess_data".into(),
            room_ttl_mins:    120,
            server_name:      "RChess Server".into(),
            motd:             String::new(),
            public_host:      String::new(),
        }
    }
}

impl ServerConfig {
    pub fn config_path() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("RCHESS_CONFIG_DIR") {
            return Some(PathBuf::from(dir).join("server.toml"));
        }
        std::env::var("HOME").ok().map(|h|
            PathBuf::from(h).join(".config").join("rchess").join("server.toml"))
    }

    pub fn load(overrides: &[(&str, &str)]) -> Self {
        let mut cfg = Self::default();
        if let Some(path) = Self::config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                cfg.apply_toml(&content);
            }
        }
        for (k, v) in overrides { cfg.apply_key(k, v); }
        cfg
    }

    fn apply_toml(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() { continue; }
            let mut kv = line.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim().trim_matches('"');
            let v = kv.next().unwrap_or("").trim().trim_matches('"');
            self.apply_key(k, v);
        }
    }

    pub fn apply_key(&mut self, k: &str, v: &str) {
        match k {
            "host"             => self.host            = v.to_string(),
            "port"             => { if let Ok(n) = v.parse() { self.port = n; } }
            "admin_port"       => { if let Ok(n) = v.parse() { self.admin_port = n; } }
            "server_password"  => self.server_password = if v.is_empty() { None } else { Some(v.to_string()) },
            "admin_password"   => self.admin_password  = if v.is_empty() { None } else { Some(v.to_string()) },
            "max_rooms"        => { if let Ok(n) = v.parse() { self.max_rooms = n; } }
            "max_conns_per_ip" => { if let Ok(n) = v.parse() { self.max_conns_per_ip = n; } }
            "data_dir"         => self.data_dir        = v.to_string(),
            "room_ttl_mins"    => { if let Ok(n) = v.parse() { self.room_ttl_mins = n; } }
            "server_name"      => self.server_name     = v.to_string(),
            "motd"             => self.motd            = v.to_string(),
            "public_host"      => self.public_host     = v.to_string(),
            _ => {}
        }
    }

    pub fn link_host(&self) -> String {
        if !self.public_host.is_empty() { return self.public_host.clone(); }
        if self.host != "0.0.0.0" && self.host != "::" { return self.host.clone(); }
        get_lan_ip().unwrap_or_else(|_| "localhost".to_string())
    }

    pub fn write_default_if_missing() {
        let Some(path) = Self::config_path() else { return };
        if path.exists() { return; }
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let template = r#"# RChess Server Configuration
# See DEPLOYMENT.md for full documentation

host             = "0.0.0.0"
port             = 9001
admin_port       = 9002

# Public address in invite links (leave empty = auto-detect LAN IP)
# public_host    = "chess.example.com"

server_password  = ""
admin_password   = ""

max_rooms        = 200
max_conns_per_ip = 20

data_dir         = "./rchess_data"
room_ttl_mins    = 120

server_name      = "RChess Server"
motd             = "Welcome! Good luck and have fun."
"#;
        if let Ok(mut f) = std::fs::File::create(&path) {
            let _ = f.write_all(template.as_bytes());
            eprintln!("[config] wrote default config → {}", path.display());
        }
    }
}

fn get_lan_ip() -> Result<String, ()> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|_| ())?;
    sock.connect("8.8.8.8:80").map_err(|_| ())?;
    Ok(sock.local_addr().map_err(|_| ())?.ip().to_string())
}
