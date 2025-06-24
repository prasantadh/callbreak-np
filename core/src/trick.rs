use deck::{Card, Suit};
use serde::{Deserialize, Serialize};

use crate::Hand;
use crate::Turn;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trick {
    // TODO: might be prudent to rename this into starter_turn
    // and starter() into starter_card
    starter: Turn,
    cards: Vec<Option<Card>>,
}

impl Trick {
    pub fn new(starter: Turn) -> Self {
        println!("trick setup with starter: {starter:?}");
        Trick {
            starter,
            cards: vec![None; 4],
        }
    }

    pub fn add_card(&mut self, card: Card) -> Result<()> {
        let current = self.current()?;
        self.cards[current] = Some(card);
        Ok(())
    }

    pub fn current(&self) -> Result<Turn> {
        let mut current = self.starter;
        for _ in 0..self.cards.len() {
            if self.cards[current].is_none() {
                return Ok(current);
            }
            current = current.next();
        }
        Err(Error::TrickIsFull)
    }

    pub fn is_full(&self) -> bool {
        self.cards.iter().all(|v| v.is_some())
    }

    pub fn winning_card(&self) -> Result<Card> {
        let mut winner = self.starting_card().ok_or(Error::NoTrickWinnerYet)?;
        for card in self.cards.iter().flatten() {
            if (winner.get_suit() != Suit::Spades && card.get_suit() == Suit::Spades)
                || (card.get_suit() == winner.get_suit() && card.get_rank() > winner.get_rank())
            {
                winner = *card;
            }
        }
        Ok(winner)
    }

    pub fn winning_turn(&self) -> Result<Turn> {
        let winner = self.winning_card()?;
        // if there is a winning_card there must be a winning turn
        if let Some(position) = self.cards.iter().position(|c| *c == Some(winner)) {
            return Turn::new(position);
        }
        Err(Error::NoTrickWinnerYet)
    }

    pub fn starting_card(&self) -> Option<Card> {
        self.cards[self.starter]
    }

    pub fn starting_turn(&self) -> Turn {
        self.starter
    }

    pub fn get_turn_from_card(&self, card: Card) -> Option<Turn> {
        if let Some(position) = self.cards.iter().position(|c| *c == Some(card)) {
            return Some(Turn::new(position).unwrap());
        }
        None
    }

    pub(crate) fn valid_from_hand(&self, hand: &Hand) -> Result<Vec<Card>> {
        let starter = match self.starting_card() {
            None => return Ok(hand.get_cards().to_vec()),
            Some(v) => v,
        };
        let winner = self.winning_card()?;

        if starter.get_suit() == Suit::Spades {
            let winning_spades: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| {
                    match v.get_suit() == Suit::Spades && v.get_rank() > winner.get_rank() {
                        true => Some(*v),
                        false => None,
                    }
                })
                .collect();
            if !winning_spades.is_empty() {
                return Ok(winning_spades);
            };

            let any_spades: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| match v.get_suit() == Suit::Spades {
                    true => Some(*v),
                    false => None,
                })
                .collect();
            if !any_spades.is_empty() {
                return Ok(any_spades);
            };
        } else if winner.get_suit() == Suit::Spades {
            let any_starter_suit: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| match v.get_suit() == starter.get_suit() {
                    true => Some(*v),
                    false => None,
                })
                .collect();
            if !any_starter_suit.is_empty() {
                return Ok(any_starter_suit);
            };

            let winning_spades: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| {
                    match v.get_suit() == Suit::Spades && v.get_rank() > winner.get_rank() {
                        true => Some(*v),
                        false => None,
                    }
                })
                .collect();
            if !winning_spades.is_empty() {
                return Ok(winning_spades);
            }
        } else {
            let winning_starter_suit: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| {
                    match v.get_suit() == starter.get_suit() && v.get_rank() > winner.get_rank() {
                        true => Some(*v),
                        false => None,
                    }
                })
                .collect();
            if !winning_starter_suit.is_empty() {
                return Ok(winning_starter_suit);
            }

            let any_starter_suit: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| match v.get_suit() == starter.get_suit() {
                    true => Some(*v),
                    false => None,
                })
                .collect();
            if !any_starter_suit.is_empty() {
                return Ok(any_starter_suit);
            }

            let any_spades: Vec<Card> = hand
                .get_cards()
                .iter()
                .filter_map(|v| match v.get_suit() == Suit::Spades {
                    true => Some(*v),
                    false => None,
                })
                .collect();
            if !any_spades.is_empty() {
                return Ok(any_spades);
            }
        }
        Ok(hand.get_cards().to_vec())
    }
}
