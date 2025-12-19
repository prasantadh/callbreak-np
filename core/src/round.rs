use crate::{Call, Hand, Trick, Turn};
use crate::{Error, Result};
use deck::{Card, Deck};

use serde::Serialize;
use std::vec;

#[derive(Debug, Clone, Serialize)]
pub struct Round {
    starter: Turn,
    hands: Vec<Hand>,
    calls: Vec<Option<Call>>,
    tricks: Vec<Trick>,
}

impl Round {
    pub(crate) fn new(starter: Turn) -> Self {
        let mut hands;
        loop {
            let deck = Deck::new();
            hands = vec![Hand::new(); 4];
            let mut current = 0;
            for card in deck {
                // this unwrap looks bad but also should never fault
                hands[current].add_card(card).unwrap();
                current = (current + 1) % 4;
            }
            if hands.iter().all(|hand| hand.is_valid()) {
                break;
            }
        }

        Round {
            starter,
            hands,
            calls: vec![None; 4],
            tricks: vec![],
        }
    }

    pub(crate) fn call(&mut self, call: Call, turn: Turn) -> Result<()> {
        let mut starter = self.starter;
        // all calls up to that point must be None

        while starter != turn {
            if self.calls[starter].is_none() {
                return Err(Error::PlayerCalledOutOfTurn);
            }
            starter = starter.next()
        }

        // the call for requested turn must be None
        if self.calls[turn].is_some() {
            return Err(Error::PlayerAlreadyCalled);
        }

        // set the value
        self.calls[turn] = Some(call);

        // initialize a trick if all calls have been made
        if self.calls.iter().all(|v| v.is_some()) {
            self.tricks.push(Trick::new(self.starter));
        }
        Ok(())
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
