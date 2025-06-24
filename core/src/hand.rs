use crate::{Error, Result};

use deck::{Card, Rank, Suit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Hand(Vec<Card>);

impl Hand {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_card(&mut self, card: Card) -> Result<()> {
        if self.0.contains(&card) {
            return Err(Error::HandHasCardAlready);
        }

        match self.0.len() {
            v if v < 13 => {
                self.0.push(card);
                Ok(())
            }
            _ => Err(Error::HandIsFull),
        }
    }

    pub fn get_cards(&self) -> &[Card] {
        self.0.as_ref()
    }

    pub fn contains(&self, card: Card) -> bool {
        self.0.contains(&card)
    }

    pub fn remove(&mut self, card: Card) {
        self.0.retain(|v| *v != card);
    }

    pub fn is_valid(&self) -> bool {
        let mut has_spades = false;
        let mut has_face = false;
        for card in self.0.iter() {
            has_face |= card.get_rank() >= Rank::Jack;
            has_spades |= card.get_suit() == Suit::Spades;
        }
        has_face && has_spades && self.0.len() == 13
    }
}
