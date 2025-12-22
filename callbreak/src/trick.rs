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
        println!("trick setup with starter: {starter:?}");
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
            return vec![];
        };
        let winner = self
            .winner()
            .expect("must have a winner when there is a starter")
            .1;

        let candidates = match (starter.get_suit(), winner.get_suit()) {
            (s, w) if s == w => [
                hand.get_cards()
                    .iter()
                    .filter(|card| card.get_suit() == s && card.get_rank() > winner.get_rank())
                    .cloned()
                    .collect::<Vec<Card>>(),
                hand.get_cards()
                    .iter()
                    .filter(|card| card.get_suit() == s)
                    .cloned()
                    .collect::<Vec<Card>>(),
                // if s = spades this is already handled above but simplifies code for other suits
                hand.get_cards()
                    .iter()
                    .filter(|card| card.get_suit() == Suit::Spades)
                    .cloned()
                    .collect::<Vec<Card>>(),
                hand.get_cards().to_vec(),
            ],
            (s, w) if s != w => [
                // implies w == Spades and s!= Spades
                hand.get_cards()
                    .iter()
                    .filter(|card| card.get_suit() == s)
                    .cloned()
                    .collect::<Vec<Card>>(),
                hand.get_cards()
                    .iter()
                    .filter(|card| card.get_suit() == w && card.get_rank() > winner.get_rank())
                    .cloned()
                    .collect(),
                hand.get_cards().to_vec(),
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

    /*
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
    */
}
