// src/net/protocol.rs — Lichess event types (re-exported for convenience)
//
// This module now re-exports types from the lichess module so that
// existing code importing from crate::net::protocol still works.

pub use crate::lichess::{
    LichessEvent, GameStreamEvent, Challenge, ApiResult,
};
