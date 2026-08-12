use super::Strategy;
use crate::agent::view::{Game, LegalMove, LegalMoves};
use crate::game::Call;

/// Bids one and plays a legal card picked uniformly at random.
///
/// Useful as a baseline: a strategy that cannot beat this one over enough
/// matches is not doing anything.
#[derive(Debug, Default)]
pub struct Random;

impl Strategy for Random {
    fn call(&mut self, _view: &Game) -> Call {
        Call::new(1).expect("one is a legal call")
    }

    fn play<'v>(&mut self, _view: &'v Game, moves: &LegalMoves<'v>) -> LegalMove<'v> {
        moves.nth(rand::random_range(0..moves.len()))
    }
}
