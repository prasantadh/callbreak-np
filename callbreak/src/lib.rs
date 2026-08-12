//! A callbreak engine, and the seats that play it.
//!
//! Seat four [`Strategy`](agent::Strategy)s at a [`Host`] and call
//! [`run`](Host::run); it deals five rounds, prompts each seat in turn, and
//! plays the match to the end. See `examples/custom_strategy.rs` for a working
//! one in about thirty lines.
//!
//! # Layers
//!
//! [`game`] holds the rules and the state of a match. It deals in seats, never
//! names, and depends on nothing above it — the intent is that it stops
//! changing while the layers over it keep moving.
//!
//! [`agent`] is what a seat sees and says: a redacted view of the table, the
//! moves open to it, and the [`Transport`](agent::Transport) that carries those
//! to a player in another process.
//!
//! [`Host`] owns the table. It is the only thing that knows which name sits in
//! which seat, and the only thing that drives a match forward.
//!
//! # Writing a strategy
//!
//! Implement [`Strategy`](agent::Strategy). Neither of its methods can fail,
//! and neither can produce a move the rules disallow — not as a rule you are
//! asked to follow, but because
//! [`play`](agent::Strategy::play) is handed the legal moves and returns one of
//! them, and there is no other way to build one. A player outside the process
//! gets the same guarantee through [`Remote`](agent::Remote), which turns an
//! illegal or missing reply into a move by its fallback strategy rather than
//! into an error.

pub mod agent;
mod error;
pub mod game;
mod host;

pub use error::Error;
pub use host::Host;

use error::Result;
use game::Game;
