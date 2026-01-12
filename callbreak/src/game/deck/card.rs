use std::cmp::Ordering;
use std::fmt::Display;

use super::{Rank, Suit};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Card {
    rank: Rank,
    suit: Suit,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        // TODO: figure out a way to take both reference and owned values here
        // INFO: This could be rewritten to take both Rank and  &Rank but knowing
        // both Rank and Suit and very light enums, it is easier to just implement
        // Copy on them and pass them as value
        Card { rank, suit }
    }

    pub fn get_suit(&self) -> Suit {
        self.suit
    }

    pub fn get_rank(&self) -> Rank {
        self.rank
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.get_suit().cmp(&other.get_suit()) {
            Ordering::Equal => other.get_rank().cmp(&self.get_rank()),
            ord => ord,
        }
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}: {}}}", self.get_rank(), self.get_suit())
    }
}
