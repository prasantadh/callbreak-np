use crate::Result;
use crate::game::{Call, Card};

use super::Agent;
use crate::playerview::PlayerView; // TODO: look into this path

pub struct Bot;

impl Agent for Bot {
    fn call(&self, _view: &PlayerView) -> Result<Call> {
        Ok(Call::new(1).expect("1 must be a valid call"))
    }

    fn play(&self, view: &PlayerView) -> Result<Card> {
        let round = view
            .rounds
            .last()
            .expect("must call play() on a valid round");
        let trick = round
            .tricks
            .last()
            .expect("must have a valid trick on a valid round");
        let card = *trick
            .valid_play_from(&round.hand)
            .first()
            .expect("must have a valid card to play");
        println!("Playing {:?}", card);
        Ok(card)
    }
}
