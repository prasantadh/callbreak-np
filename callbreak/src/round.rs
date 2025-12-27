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
            .clone()
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
                    Err(Error::NotYourTurn)
                } else {
                    self.calls[turn] = Some(call);
                    Ok(())
                }
            }
            _ => Err(Error::NotAcceptingCalls),
        }
    }

    pub(crate) fn play(&mut self, card: Card, turn: Turn) -> Result<()> {
        match self.state() {
            State::TrickInProgress => {
                // is there a trick that is currently running and not full?
                // yes, play into that. else start a new trick
                let trick = if let Some(trick) = self
                    .tricks
                    .iter_mut()
                    .rev()
                    .find_map(|t| t.as_mut())
                    .filter(|trick| !trick.is_over())
                {
                    trick
                } else {
                    let slot = self
                        .tricks
                        .iter()
                        .position(|trick| trick.is_none())
                        .expect("must have available trick when round is not over");
                    // winner from last trick or the starter of this round
                    let next = if slot == 0 {
                        self.starter
                    } else {
                        self.tricks[slot - 1]
                            .as_ref()
                            .expect("must have last trick initialized")
                            .winner()
                            .expect("must have winner for the last trick")
                            .0
                    };
                    self.tricks[slot] = Some(Trick::new(next));
                    self.tricks[slot]
                        .as_mut()
                        .expect("just initialized trick must be available")
                };

                let next = trick.turn().expect("must have the next play available");
                if next != turn {
                    return Err(Error::NotYourTurn);
                }

                let hand = &mut self.hands[next];
                if !trick.valid_play_from(hand).contains(&card) {
                    return Err(Error::InvalidPlay);
                }
                hand.play(card).expect("must be playable from this hand");
                trick.play(card).expect("must be playable into this trick");
                Ok(())
            }
            _ => Err(Error::NotAcceptingPlay),
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
            for _turn in 0..4 {
                let trick = &mut round.tricks[trick]
                let turn = round.turn();
                let moves = round.trick().get_valid_moves(round.hands[turn]);
                // get a valid play
                // play anything from there
            }
        }
    }
    */
}
