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
            .is_some_and(|t| t.as_ref().unwrap().is_over())
        {
            State::Over
        } else {
            State::TrickInProgress
        }
    }

    pub(crate) fn new(starter: Turn) -> Self {
        loop {
            let deck = Deck::new();
            let mut hands = array::from_fn(|_| Hand::new());
            let mut turn = Turn::new(0);
            for card in deck {
                hands[turn]
                    .add_card(card)
                    .expect("hands must have enough space for cards");
                turn = turn.next();
            }
            if hands.iter().all(|hand| hand.is_valid()) {
                return Round {
                    starter,
                    hands,
                    calls: [None; 4],
                    tricks: array::from_fn(|_| None),
                };
            }
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
                if let Some(trick) = self
                    .tricks
                    .iter_mut()
                    .rev()
                    .find_map(|t| t.as_mut())
                    .filter(|trick| !trick.is_over())
                {
                    let next = trick.next()?;
                    if next != turn {
                        Err(Error::NotYourTurn)
                    } else {
                        trick.play(card, &mut self.hands[next])
                    }
                } else {
                    let slot = self
                        .tricks
                        .iter()
                        .position(|trick| trick.is_none())
                        .expect("must have available trick when round is not over");
                    // winner from last round or the starter of this round
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
                        .play(card, &mut self.hands[next])
                }
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }

    pub(crate) fn is_over(&self) -> bool {
        self.state() == State::Over
    }
}
