mod bot;

use crate::Result;
use crate::deck::Card;
use crate::game::Call;
use crate::playerview::PlayerView;

pub(crate) trait Agent {
    fn call(&self, view: &PlayerView) -> Result<Call>;
    fn play(&self, view: &PlayerView) -> Result<Card>;
    // potentially important to send the periodic update to the user
    // particularly if it is a multi-player setup where each player waiting some time for each
    // player to make her move
    // fn update(&self, view: &PlayerView);
}
