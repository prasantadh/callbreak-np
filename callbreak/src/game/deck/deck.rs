use super::{Card, Rank, Suit};

use rand::{rng, seq::SliceRandom};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    idx: usize,
    cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shuffle(&mut self) {
        let mut rng = rng();
        self.cards.shuffle(&mut rng);
    }
}

impl Default for Deck {
    fn default() -> Self {
        let mut deck = Deck {
            cards: vec![],
            idx: 0,
        };
        for rank in Rank::ALL {
            for suit in Suit::ALL {
                deck.cards.push(Card::new(*rank, *suit));
            }
        }
        // TODO: How is it that I am able to return a stack variable here?
        deck.shuffle();
        deck
    }
}

impl Iterator for Deck {
    type Item = Card;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.cards.get(self.idx);
        self.idx += 1;
        next.cloned()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn new_deck_works() {
        let deck = Deck::new();
        assert_eq!(deck.cards.len(), 52);
        for rank in Rank::ALL {
            for suit in Suit::ALL {
                let card = Card::new(*rank, *suit);
                assert!(deck.cards.contains(&card));
            }
        }
    }
}
