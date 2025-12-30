use super::{Rank, Suit};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub(crate) struct Card {
    rank: Rank,
    suit: Suit,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        // TODO: figure out a way to take both reference and owned values here
        Card { rank, suit }
    }

    pub fn get_suit(&self) -> Suit {
        self.suit
    }

    pub fn get_rank(&self) -> Rank {
        self.rank
    }
}
