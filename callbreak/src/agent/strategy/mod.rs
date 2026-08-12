//! The trait a seat implements, and the three the crate ships with.
//!
//! [`Random`] is the baseline to measure against. [`Sane`] is the one meant to grow into something worth
//! playing, and is what a [`Remote`](super::Remote) falls back on.

mod random;
mod sane;

pub use random::Random;
pub use sane::Sane;

use super::view::{Game, LegalMove, LegalMoves};
use crate::game::Call;
use std::fmt::Debug;

/// A seat's brain: picks moves from the view alone. `&mut self` allows stateful play.
///
/// Neither method can fail, and neither can produce a move the rules disallow.
/// That is not a promise you are asked to keep — it is what the signatures
/// mean:
///
/// - `play` is handed the [`LegalMoves`] and returns one of them. There is no
///   way to build a [`LegalMove`] except from that set, so an illegal card is
///   not something you can express, even by mistake.
/// - `moves` is never empty, so [`LegalMoves::first`] is always available and
///   there is no case where you have nothing to return.
///
/// The upshot for anyone writing a strategy: the only bugs left are bad
/// *judgement*, which no type can catch. A wrong card still loses tricks — it
/// just cannot crash a match or be rejected by the engine.
pub trait Strategy: Send + Debug {
    fn call(&mut self, view: &Game) -> Call;

    fn play<'v>(&mut self, view: &'v Game, moves: &LegalMoves<'v>) -> LegalMove<'v>;

    /// The table moved. Called for *every* seat after *every* state change, not
    /// just the seat about to act, so a player watching from another process
    /// sees the game advance in real time.
    ///
    /// The default does nothing: a brain that only thinks when it is asked has
    /// no use for this, so implementing [`Strategy`] costs no more than before.
    fn observe(&mut self, _view: &Game) {}
}
