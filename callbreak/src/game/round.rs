use super::{Call, Hand, Trick, Turn};
use super::{Card, Deck};
use crate::{Error, Result};
use serde::Serialize;
use std::array;

const TRICKS: usize = 13;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Round {
    starter: Turn,
    hands: [Hand; 4],
    phase: Phase,
}

/// The fourth call moves a round to `Playing`, where `calls` has no `Option`
/// left: "has everybody called?" stops being a question anything can ask, and
/// the first trick exists from the moment there is one to play into.
#[derive(Debug, Clone, Serialize)]
enum Phase {
    Calling([Option<Call>; 4]),
    Playing {
        calls: [Call; 4],
        tricks: [Option<Trick>; TRICKS],
    },
}

impl Round {
    pub(crate) fn new(starter: Turn) -> Self {
        let mut hands = vec![];
        'dealing: loop {
            let deck: Vec<Card> = Deck::new().collect();
            hands = vec![];
            for i in 0..4 {
                if let Ok(hand) = Hand::try_from(&deck[i * 13..(i + 1) * 13]) {
                    hands.push(hand);
                } else {
                    continue 'dealing;
                }
            }
            break;
        }
        Round {
            starter,
            hands: hands.try_into().expect("must be exactly 4 hands"),
            phase: Phase::Calling([None; 4]),
        }
    }

    pub(crate) fn call(&mut self, call: Call, turn: Turn) -> Result<()> {
        let Phase::Calling(calls) = &self.phase else {
            return Err(Error::NotAcceptingCalls);
        };
        let mut calls = *calls;
        if self.turn()? != turn {
            return Err(Error::NotYourTurn);
        }
        calls[turn] = Some(call);

        self.phase = if turn.next() == self.starter {
            let mut tricks: [Option<Trick>; TRICKS] = array::from_fn(|_| None);
            tricks[0] = Some(Trick::new(self.starter));
            Phase::Playing {
                calls: calls.map(|call| call.expect("every seat has now called")),
                tricks,
            }
        } else {
            Phase::Calling(calls)
        };
        Ok(())
    }

    /// `None` both before the calls are in and after the last trick — two
    /// states a caller cannot act in either way, so both read as one here.
    fn active_trick(&self) -> Option<&Trick> {
        let Phase::Playing { tricks, .. } = &self.phase else {
            return None;
        };
        tricks.iter().flatten().find(|trick| !trick.is_over())
    }

    fn active_trick_mut(&mut self) -> Option<&mut Trick> {
        let Phase::Playing { tricks, .. } = &mut self.phase else {
            return None;
        };
        tricks.iter_mut().flatten().find(|trick| !trick.is_over())
    }

    /// The trick just taken decides who leads the next one. No free slot means
    /// the round is done.
    fn deal_next_trick(&mut self, winner: Turn) {
        let Phase::Playing { tricks, .. } = &mut self.phase else {
            return;
        };
        if let Some(slot) = tricks.iter().position(Option::is_none) {
            tricks[slot] = Some(Trick::new(winner));
        }
    }

    pub(crate) fn play(&mut self, card: Card, turn: Turn) -> Result<()> {
        if turn != self.turn()? {
            return Err(Error::NotYourTurn);
        }
        if !self.get_valid_moves(turn)?.contains(&card) {
            return Err(Error::CardNotPlayable);
        }

        // Everything below is the move itself, and none of it can fail: the
        // check above read this hand and found this card, and it only answers
        // at all while a trick is open. So there is no half-applied move to
        // unwind — a violation here means the round is already corrupt.
        self.hands[turn]
            .play(card)
            .expect("the card was just found in this hand");
        let trick = self
            .active_trick_mut()
            .expect("a trick was open a moment ago");
        trick.play(card).expect("an open trick accepts a card");
        let winner = trick
            .winner()
            .expect("a started trick must have a winner")
            .0;
        let finished = trick.is_over();

        if finished {
            self.deal_next_trick(winner);
        }
        Ok(())
    }

    pub(crate) fn get_valid_moves(&self, turn: Turn) -> Result<Vec<Card>> {
        let trick = self.active_trick().ok_or(Error::NotAcceptingPlays)?;
        if turn != self.turn()? {
            return Err(Error::NotYourTurn);
        }
        let cards: Vec<Card> = self.hands[turn].filter(|_| true).cloned().collect();
        Ok(trick.valid_play_from(&cards))
    }

    pub(crate) fn turn(&self) -> Result<Turn> {
        if let Phase::Calling(calls) = &self.phase {
            let mut next = self.starter;
            while calls[next].is_some() {
                // loop must terminate because it is calling
                next = next.next();
            }
            return Ok(next);
        }
        match self.active_trick() {
            Some(trick) => trick.turn(),
            None => Err(Error::RoundIsOver),
        }
    }

    pub(crate) fn get_hand(&self, turn: Turn) -> &Hand {
        &self.hands[turn]
    }

    /// Owned rather than borrowed: once a round is playing its calls are no
    /// longer `Option`s, so the shape a view wants has to be rebuilt.
    pub(crate) fn get_calls(&self) -> [Option<Call>; 4] {
        match &self.phase {
            Phase::Calling(calls) => *calls,
            Phase::Playing { calls, .. } => calls.map(Some),
        }
    }

    pub(crate) fn get_tricks(&self) -> &[Option<Trick>] {
        match &self.phase {
            Phase::Calling(_) => &[],
            Phase::Playing { tricks, .. } => tricks,
        }
    }

    pub(crate) fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Playing { .. }) && self.active_trick().is_none()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn random_turn() -> Turn {
        Turn::new(rand::random_range(0..=3))
    }

    #[test]
    fn must_err_when_play_before_call() {
        let starter = random_turn();
        let round = Round::new(starter);
        let turn = round.turn().unwrap();
        let action = round.get_valid_moves(turn);
        assert!(action.is_err())
    }

    #[test]
    fn must_be_able_to_call_on_new_round() {
        let mut starter = random_turn();
        let mut round = Round::new(starter);
        for _ in 0..=3 {
            round.call(Call::new(1).unwrap(), starter).unwrap();
            starter = starter.next()
        }
    }

    #[test]
    fn must_err_on_more_than_4_calls() {
        let mut starter = random_turn();
        let mut round = Round::new(starter);
        for _ in 0..=3 {
            round.call(Call::new(1).unwrap(), starter).unwrap();
            starter = starter.next()
        }
        let action = round.call(Call::new(1).unwrap(), starter);
        assert!(action.is_err())
    }

    #[test]
    fn must_err_on_call_out_of_turn() {
        let starter = random_turn();
        let mut round = Round::new(starter);

        let action = round.call(Call::new(1).unwrap(), starter.next());
        assert!(action.is_err());

        round.call(Call::new(1).unwrap(), starter).unwrap();
        let action = round.call(Call::new(1).unwrap(), starter.next());
        assert!(action.is_ok())
    }

    #[test]
    fn must_be_able_to_play_till_end() {
        let mut starter = random_turn();
        let mut round = Round::new(starter);

        for _ in 0..4 {
            round.call(Call::new(3).unwrap(), starter).unwrap();
            starter = starter.next();
        }

        for _trick in 0..13 {
            for _turn in 0..4 {
                let turn = round.turn().unwrap();
                let moves = round.get_valid_moves(turn).unwrap();
                round.play(*moves.first().unwrap(), turn).unwrap();
            }
        }
    }
}
