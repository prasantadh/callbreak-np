use super::{Hand, Turn};
use crate::deck::{Card, Suit};
use crate::{Error, Result};

use serde::{Deserialize, Serialize};

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

    pub(crate) fn play(&mut self, card: Card) -> Result<()> {
        if self.is_over() {
            Err(Error::NotAcceptingPlay)
        } else {
            let next = self.turn().expect("must have next available when not over");
            // TODO: may be a trick should not accept a duplicate card either?
            self.cards[next] = Some(card);
            Ok(())
        }
    }

    pub(crate) fn turn(&self) -> Result<Turn> {
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

    pub(crate) fn valid_play_from(&self, hand: &Hand) -> Vec<Card> {
        if self.is_over() {
            return vec![];
        }

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
    use crate::deck::Rank::*;
    use crate::deck::Suit::*;

    fn random_turn() -> Turn {
        Turn::new(rand::random_range(0..=3))
    }

    fn hand_with_0_clubs_234qk_diamonds_234qk_hearts_23k_spades() -> Hand {
        let cards = vec![
            Card::new(Two, Diamonds),
            Card::new(Three, Diamonds),
            Card::new(Four, Diamonds),
            Card::new(Queen, Diamonds),
            Card::new(King, Diamonds),
            Card::new(Two, Hearts),
            Card::new(Three, Hearts),
            Card::new(Four, Hearts),
            Card::new(Queen, Hearts),
            Card::new(King, Hearts),
            Card::new(Two, Spades),
            Card::new(Three, Spades),
            Card::new(King, Spades),
        ];
        Hand::try_from(cards.as_slice()).unwrap()
    }

    #[test]
    fn starter_card_is_none_for_new_trick() {
        let card = Trick::new(random_turn()).starter().1;
        assert!(card.is_none())
    }

    #[test]
    fn starter_card_is_some_after_first_play() {
        let mut trick = Trick::new(random_turn());
        let card = Card::new(Five, Hearts);
        trick.play(card).unwrap();
        trick.play(Card::new(Six, Hearts)).unwrap();
        trick.play(Card::new(Two, Hearts)).unwrap();
        let starter = trick.starter().1;
        assert!(starter.is_some_and(|c| c == card));
    }

    #[test]
    fn winner_is_none_when_no_play() {
        let trick = Trick::new(random_turn());
        assert!(trick.winner().is_none());
    }

    #[test]
    fn winner_is_correct_when_all_plays_are_non_spades() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Queen, Clubs)).unwrap();
        trick.play(Card::new(King, Clubs)).unwrap();
        trick.play(Card::new(Two, Clubs)).unwrap();
        trick.play(Card::new(Ace, Hearts)).unwrap();
        assert!(trick.winner().is_some_and(
            |(turn, card)| turn == trick.starter().0.next() && card == Card::new(King, Clubs)
        ));
    }

    #[test]
    fn winner_is_correct_when_non_spade_starter_is_spaded() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Queen, Clubs)).unwrap();
        trick.play(Card::new(King, Clubs)).unwrap();
        trick.play(Card::new(Two, Spades)).unwrap();
        trick.play(Card::new(Ace, Hearts)).unwrap();
        assert!(
            trick
                .winner()
                .is_some_and(|(turn, card)| turn == trick.starter().0.next().next()
                    && card == Card::new(Two, Spades))
        );
    }

    #[test]
    fn winner_is_correct_when_non_spade_starter_is_spaded_twice() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Queen, Clubs)).unwrap();
        trick.play(Card::new(King, Clubs)).unwrap();
        trick.play(Card::new(Two, Spades)).unwrap();
        trick.play(Card::new(Four, Spades)).unwrap();
        assert!(trick.winner().is_some_and(|(turn, card)| turn
            == trick.starter().0.next().next().next()
            && card == Card::new(Four, Spades)));
    }

    #[test]
    fn winner_is_correct_when_all_plays_are_spades() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Queen, Spades)).unwrap();
        trick.play(Card::new(King, Spades)).unwrap();
        trick.play(Card::new(Two, Spades)).unwrap();
        trick.play(Card::new(Seven, Spades)).unwrap();
        assert!(trick.winner().is_some_and(
            |(turn, card)| turn == trick.starter().0.next() && card == Card::new(King, Spades)
        ));
    }

    #[test]
    fn only_winning_rank_of_starter_suit_is_valid_play() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Seven, Diamonds)).unwrap();

        let hand = hand_with_0_clubs_234qk_diamonds_234qk_hearts_23k_spades();
        assert!(trick.valid_play_from(&hand).len() == 2)
    }

    #[test]
    fn only_starter_suit_is_valid_is_valid() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Ace, Diamonds)).unwrap();

        let hand = hand_with_0_clubs_234qk_diamonds_234qk_hearts_23k_spades();
        assert!(trick.valid_play_from(&hand).len() == 5)
    }

    #[test]
    fn only_spade_is_valid_play_if_missing_starter_suit() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Ace, Clubs)).unwrap();

        let hand = hand_with_0_clubs_234qk_diamonds_234qk_hearts_23k_spades();
        assert!(trick.valid_play_from(&hand).len() == 3)
    }

    #[test]
    fn only_winning_spade_is_valid_play_if_missing_starter_suit() {
        let mut trick = Trick::new(random_turn());
        trick.play(Card::new(Ace, Clubs)).unwrap();
        trick.play(Card::new(Five, Spades)).unwrap();

        let hand = hand_with_0_clubs_234qk_diamonds_234qk_hearts_23k_spades();
        assert!(trick.valid_play_from(&hand).len() == 1)
    }
}
