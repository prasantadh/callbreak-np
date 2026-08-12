//! A seat is anything that implements [`Strategy`]. The [`strategy`] module
//! carries the ones the crate ships with; [`Remote`] is a seat played from
//! outside the process, over a [`Transport`]. Extend by writing a `Strategy`,
//! not by editing this module.
//!
//! TODO: (public strategy/simulation API, deferred):
//!   1. `Host::run` should return scores/winner; move scoring into the engine.
//!   2. Seeded deck (`Host::with_seed`) for reproducible deals.
//!   3. `Remote` should bound how long a client may take over a turn, and give
//!      the seat away to its fallback once that runs out.

mod remote;
pub mod strategy;
mod transport;
mod view;

pub use remote::Remote;
pub use strategy::Strategy;
pub use transport::Transport;
pub use view::{Action, ClientMessage, Game, LegalMove, LegalMoves, Round, ServerMessage};
