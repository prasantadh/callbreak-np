use crate::{Call, Hand, Trick, Turn};
use crate::{Error, Result};
use deck::{Card, Deck};

use serde::Serialize;
use std::array;

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
                // find who is next
                let mut next = self.starter;
                while self.calls[next].is_some() {
                    // loop must terminate because it is calling
                    next = next.next();
                }

                // set the call
                if next != turn {
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
                    let trick = self.tricks[slot]
                        .as_mut()
                        .expect("current trick must be available");
                    if turn != trick.turn()? {
                        return Err(Error::NotYourTurn);
                    }
                    let hand = &mut self.hands[turn];
                    if trick.valid_play_from(hand).contains(&card) {
                        return Err(Error::InvalidPlay);
                    }
                    // TODO: there is some issue here where if hand.play() passes
                    // trick.play() must not fail. The code doesn't enforce that at the moment
                    // which could lead to logic bugs in the future being overlooked.
                    // a potential solution is to move valid_play_from() to this mod from trick?
                    hand.play(card)?;
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
                let slot = self
                    .tricks
                    .iter()
                    .position(|t| t.as_ref().is_some_and(|t| !t.is_over()))
                    .expect("must have an active trick in this state");
                let trick = self.tricks[slot]
                    .as_ref()
                    .expect("active trick must be Some(trick)");
                if turn != trick.turn()? {
                    return Err(Error::NotYourTurn);
                }
                Ok(trick.valid_play_from(&self.hands[turn]))
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }

    pub(crate) fn turn(&self) -> Result<Turn> {
        match self.state() {
            State::Calling => Ok(Turn::new(0)),
            State::TrickInProgress => Ok(Turn::new(1)),
            State::Over => Err(Error::RoundIsOver),
        }
    }

    pub(crate) fn is_over(&self) -> bool {
        self.state() == State::Over
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn random_turn() -> Turn {
        Turn::new(rand::random_range(0..=3))
    }

    #[test]
    fn must_err_when_play_before_call() {}

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

    /*
    #[test]
    fn must_be_able_to_play_till_end() {
        let mut starter = random_turn();
        let mut round = Round::new(starter);

        for _ in 0..4 {
            round.call(Call::new(3).unwrap(), starter).unwrap();
            starter = starter.next();
        }

        for trick in 0..13 {
            for turn in 0..4 {
                let moves = round.get_valid_moves(round.turn()).unwrap();
                round.play(moves.first().unwrap(), turn)
                // get a valid play
                // play anything from there
            }
        }
    }
    */
}
