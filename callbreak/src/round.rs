use crate::{Call, Hand, Trick, Turn};
use crate::{Error, Result};
use deck::{Card, Deck};

use serde::Serialize;
use std::{array, vec};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Round {
    starter: Turn,
    hands: [Hand; 4],
    calls: [Option<Call>; 4],
    tricks: Vec<Trick>,
}

enum State {
    Calling,
    TrickInProgress,
}

impl Round {
    fn state(&self) -> State {
        if self.calls.iter().any(|call| call.is_none()) {
            State::Calling
        } else {
            State::TrickInProgress
        }
    }

    pub(crate) fn new(starter: Turn) -> Self {
        loop {
            let deck = Deck::new();
            let mut hands = array::from_fn(|_| Hand::new());
            let mut turn = Turn::new(0).expect("0 must be a valid turn");
            // let mut current = 0;
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
                    tricks: vec![],
                };
            }
        }
    }

    pub(crate) fn call(&mut self, call: Call, turn: Turn) -> Result<()> {
        match self.state() {
            State::Calling => {
                // find who is next
                let mut next = self.starter;
                for _ in 0..4 {
                    if self.calls[next].is_none() {
                        break;
                    }
                    next = next.next();
                }
                // set the call
                if next != turn || self.calls[turn].is_some() {
                    Err(Error::NotYourTurn)
                } else {
                    self.calls[turn] = Some(call);
                    Ok(())
                }
            }
            _ => Err(Error::NotAcceptingCalls),
        }
    }

    pub(crate) fn current(&self) -> Result<Turn> {
        let not_all_called = self.calls.iter().any(|v| v.is_none());
        if not_all_called {
            let mut starter = self.starter;
            loop {
                let turn = starter;
                if self.calls[turn].is_none() {
                    return Ok(turn);
                }
                starter = starter.next();
            }
        }

        self.tricks
            .last()
            .ok_or(Error::TrickNotInitialized)?
            .current()
    }

    pub(crate) fn current_trick(&self) -> Result<&Trick> {
        match self.tricks.last() {
            None => Err(Error::TrickNotInitialized),
            Some(trick) => Ok(trick),
        }
    }

    pub(crate) fn all_called(&self) -> bool {
        self.calls.iter().all(|v| v.is_some())
    }

    pub(crate) fn play(&mut self, card: Card, turn: Turn) -> Result<()> {
        if !self.all_called() {
            return Err(Error::AllCallsNeededBeforeAnyPlay);
        }

        if turn != self.current()? {
            return Err(Error::PlayerPlayedOutOfTurn);
        }

        {
            // add the card to the trick
            let trick = self.tricks.last_mut().ok_or(Error::TrickNotInitialized)?;
            let hand = &self.hands[turn];
            let valid_moves = trick.valid_from_hand(hand)?;
            if !valid_moves.contains(&card) {
                return Err(Error::InvalidPlay);
            }
            trick.add_card(card)?;
        }
        {
            // if trick is full but round is not over, add another trick
            let trick = self.tricks.last().ok_or(Error::TrickNotInitialized)?;
            if self.tricks.len() != 13 && trick.current().is_err() {
                self.tricks.push(Trick::new(trick.winning_turn()?));
            }
        }
        Ok(())
    }

    pub(crate) fn is_over(&self) -> bool {
        self.tricks.len() == 13 && self.tricks.last().is_some_and(|t| t.current().is_err())
    }

    pub(crate) fn get_hand(&self, turn: Turn) -> &Hand {
        &self.hands[turn]
    }

    pub(crate) fn get_calls(&self) -> &[Option<Call>] {
        &self.calls
    }
}
