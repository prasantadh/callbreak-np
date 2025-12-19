use deck::{Card, Suit};
use serde::{Deserialize, Serialize};

use crate::Hand;
use crate::Turn;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trick {
    starter_turn: Turn,
    cards: [Option<Card>; 4],
}

impl Trick {
    // TODO: is it nicer to also take a [Option<Card>; 4]
    // so that we can verify stuff like [None, Some(), None, Some()]
    // during deserialization, which should not happen
    pub(crate) fn new(starter: Turn) -> Self {
        println!("trick setup with starter: {starter:?}");
        Trick {
            starter_turn: starter,
            cards: [None; 4],
        }
    }

    pub(crate) fn add_card(&mut self, card: Card) -> Result<()> {
        let current = self.current()?;
        self.cards[current] = Some(card);
        Ok(())
    }

    pub(crate) fn current(&self) -> Result<Turn> {
        let mut current = self.starter_turn;
        for _ in 0..self.cards.len() {
            if self.cards[current].is_none() {
                return Ok(current);
            }
            current = current.next();
        }
        Err(Error::TrickIsOver)
    }

    fn winning_card(&self) -> Result<Card> {
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

    pub(crate) fn winning_turn(&self) -> Result<Turn> {
        let winner = self.winning_card()?;
        // if there is a winning_card there must be a winning turn
        if let Some(position) = self.cards.iter().position(|c| *c == Some(winner)) {
            return Turn::new(position);
        }
        Err(Error::NoTrickWinnerYet)
    }

    fn starting_card(&self) -> Option<Card> {
        self.cards[self.starter_turn]
    }

    // TODO: look into if this must return a vector
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
