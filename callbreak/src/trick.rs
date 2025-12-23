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
    // during deserialization, which should not happen.
    // on second thought, why should this support deserialization at all?!
    pub(crate) fn new(starter: Turn) -> Self {
        Trick {
            starter_turn: starter,
            cards: [None; 4],
        }
    }

    pub(crate) fn play(&mut self, card: Card, hand: &mut Hand) -> Result<()> {
        let valid_moves = self.valid_play_from(hand);
        if valid_moves.contains(&card) {
            let next = self.next()?;
            hand.play(card)?;
            self.cards[next] = Some(card);
            Ok(())
        } else {
            Err(Error::InvalidPlay)
        }
    }

    pub(crate) fn next(&self) -> Result<Turn> {
        if self.is_over() {
            Err(Error::NotAcceptingPlay)
        } else {
            let mut turn = self.starter_turn;
            while self.cards[turn].is_some() {
                // this loop must terminate because the trick is not over
                turn = turn.next();
            }
            Ok(turn)
        }
    }

    pub(crate) fn is_over(&self) -> bool {
        self.cards.iter().all(|c| c.is_some())
    }

    pub(crate) fn starter(&self) -> (Turn, Option<Card>) {
        (self.starter_turn, self.cards[self.starter_turn])
    }

    pub(crate) fn winner(&self) -> Option<(Turn, Card)> {
        // if there is a spade card, return the max spade card by rank
        // else return the max ranked card of the starter suit
        match self.starter() {
            (_, None) => None,
            (_, Some(starter)) => {
                let winning_suit = if self
                    .cards
                    .iter()
                    .flatten()
                    .any(|card| card.get_suit() == Suit::Spades)
                {
                    Suit::Spades
                } else {
                    starter.get_suit()
                };
                let winner = self
                    .cards
                    .iter()
                    .enumerate()
                    .filter_map(|(i, opt)| opt.as_ref().map(|card| (i, card)))
                    .filter(|(_, card)| card.get_suit() == winning_suit)
                    .max_by_key(|(_, card)| card.get_rank())
                    .expect("must have a winner");
                Some((Turn::new(winner.0), *winner.1))
            }
        }
    }

    fn valid_play_from(&self, hand: &Hand) -> Vec<Card> {
        let starter = if let (_, Some(card)) = self.starter() {
            card
        } else {
            return hand.filter(|_| true).cloned().collect();
        };
        let winner = self
            .winner()
            .expect("must have a winner when there is a starter")
            .1;

        let candidates: [Vec<Card>; 4] = match (starter.get_suit(), winner.get_suit()) {
            (s, w) if s == w => [
                hand.filter(|card| card.get_suit() == s && card.get_rank() > winner.get_rank())
                    .cloned()
                    .collect(),
                hand.filter(|card| card.get_suit() == s).cloned().collect(),
                // if s = spades this is already handled above but simplifies code for other suits
                hand.filter(|card| card.get_suit() == Suit::Spades)
                    .cloned()
                    .collect(),
                hand.filter(|_| true).cloned().collect(),
            ],
            (s, w) if s != w => [
                // implies w == Spades and s!= Spades
                hand.filter(|card| card.get_suit() == s).cloned().collect(),
                hand.filter(|card| card.get_suit() == w && card.get_rank() > winner.get_rank())
                    .cloned()
                    .collect(),
                hand.filter(|_| true).cloned().collect(),
                vec![],
            ],
            (s, w) => panic!(
                "this starter: {:?} winner: {:?} combination must not be possible",
                s, w
            ),
        };

        match candidates.iter().find(|v| !v.is_empty()) {
            Some(v) => v.clone(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use deck::Rank;

    #[test]
    fn must_play_winning_rank_of_starter_suit() {}

    #[test]
    fn must_play_any_of_starter_suit() {}

    #[test]
    fn must_play_spade_if_missing_starter_suit() {}

    #[test]
    fn must_play_winning_spade_if_missing_starter_suit() {}

    /*
    #[test]
    fn valid_play_from_same_suit_winner_is_correct() {
        let mut trick = Trick::new(Turn::new(0));
        let mut hand = Hand::new();
        hand.add_card(Card::new(Rank::Six, Suit::Clubs)).unwrap();
        hand.add_card(Card::new(Rank::Seven, Suit::Clubs)).unwrap();
        hand.add_card(Card::new(Rank::Eight, Suit::Spades)).unwrap();
        hand.add_card(Card::new(Rank::Nine, Suit::Spades)).unwrap();

        let moves = trick.valid_play_from(&hand);
        println!("==> {:?}", moves);

        trick
            .play(Card::new(Rank::Six, Suit::Clubs), &mut hand)
            .unwrap();
        trick
            .play(Card::new(Rank::Seven, Suit::Clubs), &mut hand)
            .unwrap();

        let moves = trick.valid_play_from(&hand);
        println!("==> {:?}", moves);
        trick
            .play(Card::new(Rank::Eight, Suit::Clubs), &mut hand)
            .unwrap();
        trick
            .play(Card::new(Rank::Nine, Suit::Clubs), &mut hand)
            .unwrap();
    }
    */
}
