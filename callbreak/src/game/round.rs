use super::{Call, Hand, Trick, Turn};
use super::{Card, Deck};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::array;
use std::ops::{Index, IndexMut};
use tracing::debug;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Round {
    starter: Turn,
    hands: [Hand; 4],
    calls: [Option<Call>; 4],
    tricks: [Option<Trick>; 13],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    Calling,
    TrickInProgress,
    Over,
}

impl Round {
    fn state(&self) -> State {
        if self.calls.iter().any(|call| call.is_none()) {
            State::Calling
        } else if self
            .tricks
            .last()
            .expect("must have the last slot available by definition")
            .as_ref()
            .is_some_and(|t| t.is_over())
        {
            State::Over
        } else {
            State::TrickInProgress
        }
    }

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
            calls: [None; 4],
            tricks: array::from_fn(|_| None),
        }
    }

    pub(crate) fn call(&mut self, call: Call, turn: Turn) -> Result<()> {
        match self.state() {
            State::Calling => {
                if self.turn()? != turn {
                    return Err(Error::NotYourTurn);
                } else {
                    self.calls[turn] = Some(call);
                }
                // if all calls are made, start a trick
                if turn.next() == self.starter {
                    self.tricks[0] = Some(Trick::new(self.starter))
                }
                Ok(())
            }
            _ => Err(Error::NotAcceptingCalls),
        }
    }

    pub(crate) fn play(&mut self, card: Card, turn: Turn) -> Result<()> {
        match self.state() {
            State::TrickInProgress => {
                let slot = self
                    .tricks
                    .iter()
                    .position(|t| t.as_ref().is_some_and(|t| !t.is_over()))
                    .expect("must have an active trick in this state");

                let winner = {
                    if turn != self.turn()? {
                        return Err(Error::NotYourTurn);
                    }
                    if !self.get_valid_moves(turn)?.contains(&card) {
                        return Err(Error::InvalidPlay);
                    }
                    // TODO: if hand.play() passes trick.play() must not fail.
                    // The code doesn't read like a single transaction at the moment.
                    // There might be a way to write it better?
                    let trick = self.tricks[slot]
                        .as_mut()
                        .expect("current trick must be available");
                    self.hands[turn].play(card)?;
                    trick.play(card)?;
                    trick
                        .winner()
                        .expect("a started trick must have a winner")
                        .0
                };

                if slot != 12
                    && self.tricks[slot]
                        .as_ref()
                        .is_some_and(|trick| trick.is_over())
                {
                    self.tricks[slot + 1] = Some(Trick::new(winner))
                }
                Ok(())
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }

    pub(crate) fn get_valid_moves(&self, turn: Turn) -> Result<Vec<Card>> {
        match self.state() {
            State::TrickInProgress => {
                if turn != self.turn()? {
                    return Err(Error::NotYourTurn);
                }
                let slot = self
                    .tricks
                    .iter()
                    .position(|t| t.as_ref().is_some_and(|t| !t.is_over()))
                    .expect("must have an active trick in this state");
                let cards: Vec<Card> = self.hands[turn].filter(|_| true).cloned().collect();
                Ok(self.tricks[slot].as_ref().unwrap().valid_play_from(&cards))
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }

    pub(crate) fn turn(&self) -> Result<Turn> {
        match self.state() {
            State::Calling => {
                let mut next = self.starter;
                while self.calls[next].is_some() {
                    // loop must terminate because it is calling
                    next = next.next();
                }
                Ok(next)
            }
            State::TrickInProgress => {
                let slot = self
                    .tricks
                    .iter()
                    .position(|t| t.as_ref().is_some_and(|t| !t.is_over()))
                    .expect("must have an active trick in this state");
                self.tricks[slot]
                    .as_ref()
                    .expect("active trick must be Some(trick)")
                    .turn()
            }
            State::Over => Err(Error::RoundIsOver),
        }
    }

    pub(crate) fn get_hand(&self, turn: Turn) -> Hand {
        self.hands[turn].clone()
    }

    pub(crate) fn get_calls(&self) -> [Option<Call>; 4] {
        self.calls.into()
    }

    pub(crate) fn get_tricks(&self) -> Vec<Trick> {
        // TODO: there is probably some performance optimizations to be had here
        self.tricks.iter().flatten().cloned().collect()
    }

    pub(crate) fn is_over(&self) -> bool {
        self.state() == State::Over
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub struct RoundId(usize);
impl RoundId {
    pub fn new(value: usize) -> Result<Self> {
        match value {
            v if v < 5 => Ok(RoundId(value)),
            _ => Err(Error::InvalidRoundId),
        }
    }

    pub fn max() -> Self {
        Self(5)
    }
}
impl From<RoundId> for usize {
    fn from(value: RoundId) -> Self {
        value.0
    }
}
impl<T> Index<RoundId> for Vec<T> {
    type Output = T;
    fn index(&self, index: RoundId) -> &Self::Output {
        self.index(index.0)
    }
}

impl<T> IndexMut<RoundId> for Vec<T> {
    fn index_mut(&mut self, index: RoundId) -> &mut Self::Output {
        &mut self[index.0]
    }
}

impl<T> Index<RoundId> for [T] {
    type Output = T;
    fn index(&self, index: RoundId) -> &Self::Output {
        self.index(index.0)
    }
}

impl<T> IndexMut<RoundId> for [T] {
    fn index_mut(&mut self, index: RoundId) -> &mut Self::Output {
        &mut self[index.0]
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
