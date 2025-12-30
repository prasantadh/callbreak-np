use crate::deck::{Card, Rank, Suit};

use crate::{Call, Result, agent::Agent, playerview::PlayerView};

pub struct Bot;

impl Agent for Bot {
    fn call(&self, _view: &PlayerView) -> Result<Call> {
        Ok(Call::new(1).expect("1 must be a valid call"))
    }

    fn play(&self, _view: &PlayerView) -> Result<Card> {
        // FIXME: get the trick from playerview then valid
        // move then play a random valid move
        Ok(Card::new(Rank::Six, Suit::Hearts))
    }
}
