use deck::Card;

use crate::{Call, agent::Agent, playerview::PlayerView};

pub struct Bot;

impl Agent for Bot {
    fn call(&self, _view: &PlayerView) -> crate::Call {
        Call::new(1).expect("1 must be a valid call")
    }

    fn play(&self, _view: &PlayerView) -> deck::Card {
        Card::new(deck::Rank::Six, deck::Suit::Hearts)
    }
}
