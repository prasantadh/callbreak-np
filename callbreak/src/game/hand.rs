use super::{Card, Rank, Suit};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Display};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Hand(Vec<Card>);

impl Hand {
    pub fn filter<P>(&self, mut predicate: P) -> impl Iterator<Item = &Card>
    where
        P: FnMut(&Card) -> bool,
    {
        self.0.iter().filter(move |card| predicate(card))
    }

    pub fn play(&mut self, card: Card) -> Result<Card> {
        self.0
            .iter()
            .position(|&v| v == card)
            .map(|v| Ok(self.0.swap_remove(v)))
            .ok_or(Error::CardNotInHand)?
    }
}

impl TryFrom<&[Card]> for Hand {
    type Error = Error;
    fn try_from(cards: &[Card]) -> std::result::Result<Self, Self::Error> {
        let mut has_spades = false;
        let mut has_face = false;
        for card in cards.iter() {
            has_face |= card.get_rank() >= Rank::Jack;
            has_spades |= card.get_suit() == Suit::Spades;
        }
        let mut cards = cards.to_vec();
        cards.sort();
        if !has_face {
            Err(Error::HandHasNoFaceCard)
        } else if !has_spades {
            Err(Error::HandHasNoSpade)
        } else if cards.len() != 13 {
            Err(Error::HandIsNotThirteenCards)
        } else if HashSet::<&Card>::from_iter(cards.iter()).len() != 13 {
            Err(Error::HandHasDuplicateCards)
        } else {
            Ok(Self(cards)) // TODO: sort the vector here by suit then rank?
        }
    }
}

impl Display for Hand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for card in self.0.iter() {
            write!(f, "{} of {}", card.get_rank(), card.get_suit())?
        }
        Ok(())
    }
}
