// src/log.rs — Silent file logger. Never touches stderr/stdout.
//
// All debug output goes to /tmp/rchess.log — never bleeds into the TUI.
// Usage anywhere:  rlog!("message {} {}", a, b);
// Watch live:      tail -f /tmp/rchess.log

use std::fs::OpenOptions;
use std::io::Write;

pub fn write_log(msg: String) {
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rchess.log")
    {
        let _ = writeln!(f, "{}", msg);
    }
}

// The macro must NOT have a trailing semicolon inside the expansion,
// because it's used in expression positions (e.g. `=> rlog!(...)` match arms).
// Using `{ ... }` block form with no semicolon after the call fixes the lint.
#[macro_export]
macro_rules! rlog {
    ($($arg:tt)*) => {{
        $crate::log::write_log(format!($($arg)*))
    }};
}
