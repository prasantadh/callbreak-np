use super::{Random, Strategy};
use crate::agent::view::{Game, LegalMove, LegalMoves};
use crate::game::Call;

/// A seat that plays competently enough to stand in for a person.
///
/// This is what a [`Remote`](crate::agent::Remote) reaches for when its client
/// goes quiet, so it is the strategy a disconnected player comes back to find
/// having played their hand. That makes it worth more than the minimum: the
/// point is to not wreck someone's match while they reconnect.
///
/// Right now it is a placeholder that plays like [`Random`]. The heuristic goes
/// here — the type and its place in the crate are settled, so filling it in
/// changes nothing at any call site.
#[derive(Debug, Default)]
pub struct Sane {
    inner: Random,
}

impl Strategy for Sane {
    fn call(&mut self, view: &Game) -> Call {
        self.inner.call(view)
    }

    fn play<'v>(&mut self, view: &'v Game, moves: &LegalMoves<'v>) -> LegalMove<'v> {
        self.inner.play(view, moves)
    }
}
