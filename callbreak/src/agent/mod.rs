mod bot;
mod net;
mod view;

use std::fmt::Debug;

pub use bot::Bot;
pub use net::Net;
pub use view::{PlayerView, RoundView};

use crate::Result;
use crate::game::{Call, Card};

// TODO: figure out the : syntax while defining trait
pub trait Agent: Debug {
    fn call(&mut self, view: &PlayerView) -> Result<Call>;
    fn play(&mut self, view: &PlayerView) -> Result<Card>;
    // TODO: potentially important to send the periodic update to the user
    // particularly if it is a multi-player setup where each player waiting some time for each
    // player to make her move.
    // fn update(&self, view: &PlayerView);
}
