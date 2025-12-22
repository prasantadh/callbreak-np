use crate::{Error, Result};

use deck::{Card, Rank, Suit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
// TODO: is it better to handle this as [Option<Card>; 13]
pub struct Hand(Vec<Card>);

impl Hand {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_card(&mut self, card: Card) -> Result<()> {
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

    pub(crate) fn get_cards(&self) -> &[Card] {
        self.0.as_ref()
    }

    pub(crate) fn play(&mut self, card: Card) -> Result<()> {
        let idx = self.0.iter().position(|v| *v == card);
        if let Some(idx) = idx {
            self.0.remove(idx);
            Ok(())
        } else {
            Err(Error::HandDoesNotHaveThisCard)
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
        let mut has_spades = false;
        let mut has_face = false;
        for card in self.0.iter() {
            has_face |= card.get_rank() >= Rank::Jack;
            has_spades |= card.get_suit() == Suit::Spades;
        }
        has_face && has_spades && self.0.len() == 13
    }
}
