use super::Agent;
use crate::Result;
use crate::game::{Call, Card, Cards};
use crate::playerview::PlayerView; // TODO: look into this path
use tracing::debug;

#[derive(Debug)]
pub struct Bot;

impl Agent for Bot {
    fn call(&self, _view: &PlayerView) -> Result<Call> {
        let call = Call::new(1).expect("1 must be a valid call");
        debug!(?call, "requesting");
        Ok(call)
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
        debug!(hand = %Cards(&round.hand));
        debug!(%trick); // TODO: Since %trick is used instead of ?trick, may be the logging should
        // be changed to info! instead of debug!
        debug!(%card, "requesting");
        Ok(card)
    }
}
