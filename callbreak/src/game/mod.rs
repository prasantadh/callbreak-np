mod call;
mod deck;
mod hand;
mod round;
mod trick;
mod turn;

pub use call::Call;
use deck::Deck;
pub use deck::{Card, Rank, Suit};
pub use hand::Hand;
pub use trick::Trick;
pub use turn::Turn;

use crate::{Error, Result};
pub(crate) use round::Round;
use serde::Serialize;
use std::array;

const ROUNDS: usize = 5;

/// Who is sitting where is the [`Host`](crate::Host)'s business: the engine
/// deals in seats, so it never needs a name and cannot disagree with the host
/// about which name belongs to which seat.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Game {
    rounds: [Option<Round>; ROUNDS],
}

impl Default for Game {
    /// There is no un-started `Game`: the host is what knows when four people
    /// are seated, so "the match has not begun" is not a state the engine can
    /// be asked about — or forget to check for.
    fn default() -> Self {
        let mut game = Self {
            rounds: array::from_fn(|_| None),
        };
        game.deal_next();
        game
    }
}

impl Game {
    /// Round `i` is led by seat `i`, so the slot a round lands in is also who
    /// starts it. No free slot means no round left to deal.
    fn deal_next(&mut self) {
        if let Some(slot) = self.rounds.iter().position(Option::is_none) {
            self.rounds[slot] = Some(Round::new(Turn::new(slot)));
        }
    }

    /// At most one round is ever unfinished: they are dealt in order, and the
    /// next only once the current one ends. Holding the `Some` is what proves
    /// the match is still running, so nothing below asserts it again.
    fn active_round(&self) -> Option<&Round> {
        self.rounds.iter().flatten().find(|round| !round.is_over())
    }

    fn active_round_mut(&mut self) -> Option<&mut Round> {
        self.rounds
            .iter_mut()
            .flatten()
            .find(|round| !round.is_over())
    }

    pub(crate) fn call(&mut self, turn: Turn, call: Call) -> Result<()> {
        self.active_round_mut()
            .ok_or(Error::NotAcceptingCalls)?
            .call(call, turn)
    }

    pub(crate) fn play(&mut self, turn: Turn, card: Card) -> Result<()> {
        let round = self.active_round_mut().ok_or(Error::NotAcceptingPlays)?;
        round.play(card, turn)?;
        if round.is_over() {
            self.deal_next();
        }
        Ok(())
    }

    pub(crate) fn turn(&self) -> Result<Turn> {
        self.active_round().ok_or(Error::NotYourTurn)?.turn()
    }

    /// Every round dealt so far, oldest first. What a seat is allowed to see of
    /// one is the caller's decision, not the engine's.
    pub(crate) fn rounds(&self) -> impl Iterator<Item = &Round> {
        self.rounds.iter().flatten()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::Rank::*;
    use crate::game::Suit::*;

    fn legal_card(game: &Game, seat: Turn) -> Card {
        *game
            .active_round()
            .expect("the match is running")
            .get_valid_moves(seat)
            .expect("the seat to act has moves")
            .first()
            .expect("the legal set is never empty")
    }

    #[test]
    fn can_play_to_completion() {
        let mut game = Game::default();

        for _round in 0..5 {
            for _turn in 0..4 {
                let seat = game.turn().unwrap();
                game.call(seat, Call::new(3).unwrap()).unwrap();
            }
            for _trick in 0..13 {
                for _turn in 0..4 {
                    let seat = game.turn().unwrap();
                    game.play(seat, legal_card(&game, seat)).unwrap();
                }
            }
        }
    }

    // Nothing tests that a match rejects a call or a play before it starts:
    // `Game::default` deals, so there is no un-started game to try it on.

    #[test]
    fn cannot_call_out_of_turn() {
        let mut game = Game::default();
        // Any seat but the one to act.
        let next = game.turn().unwrap().next();
        assert_eq!(
            game.call(next, Call::new(3).unwrap()),
            Err(Error::NotYourTurn)
        );
    }

    #[test]
    fn each_player_has_a_spade_card_and_a_face_card() {
        let mut game = Game::default();
        let round = game.rounds[0].as_mut().unwrap();
        for i in 0..4 {
            assert!(
                round
                    .get_hand(Turn::new(i))
                    .filter(|_| true)
                    .any(|c| c.get_suit() == Spades),
            );
            assert!(
                round
                    .get_hand(Turn::new(i))
                    .filter(|_| true)
                    .any(|c| c.get_rank() >= Jack)
            );
        }
    }

    #[test]
    fn round_i_must_be_started_by_turn_i() {
        let mut game = Game::default();

        for round in 0..5 {
            assert_eq!(
                game.turn().unwrap(),
                Turn::new(round),
                "round {round} must be called first by seat {round}"
            );
            for _turn in 0..4 {
                let seat = game.turn().unwrap();
                game.call(seat, Call::new(3).unwrap()).unwrap();
            }
            assert_eq!(
                game.turn().unwrap(),
                Turn::new(round),
                "round {round} must be led by seat {round}"
            );
            for _trick in 0..13 {
                for _turn in 0..4 {
                    let seat = game.turn().unwrap();
                    game.play(seat, legal_card(&game, seat)).unwrap();
                }
            }
        }
    }

    #[test]
    fn cannot_play_out_of_turn() {
        let mut game = Game::default();
        for _ in 0..4 {
            let seat = game.turn().unwrap();
            game.call(seat, Call::new(3).unwrap()).unwrap();
        }
        // Any seat but the one to act; what card is used doesn't matter.
        let next = game.turn().unwrap().next();
        assert_eq!(
            game.play(next, Card::new(Six, Diamonds)),
            Err(Error::NotYourTurn)
        );
    }
}
