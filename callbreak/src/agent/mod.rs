mod bot;
use deck::Card;

use crate::{Call, playerview::PlayerView};

pub trait Agent {
    fn call(&self, view: &PlayerView) -> Call;
    fn play(&self, view: &PlayerView) -> Card;
}
