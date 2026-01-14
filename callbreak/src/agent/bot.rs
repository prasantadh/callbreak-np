use super::view::Game;
use crate::game::{Call, Card};

/// Agent to facilitate interaction with a bot
#[derive(Debug)]
pub struct Bot;

impl Bot {
    pub(super) fn call(&self, _view: &Game) -> Call {
        Call::new(1).unwrap()
    }

    pub(super) fn play(&self, view: &Game) -> Card {
        let round = view
            .rounds
            .last()
            .expect("must call play() on a valid round");
        let trick = round
            .tricks
            .last()
            .expect("must have a valid trick on a valid round");
        *trick
            .valid_play_from(&round.hand)
            .first()
            .expect("must have a valid card to play")
    }
}
