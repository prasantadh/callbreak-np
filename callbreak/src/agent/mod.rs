mod bot;
mod net;

use std::fmt::Debug;

pub use bot::Bot;
pub use net::Net;

use crate::Result;
use crate::game::{Call, Card};
use crate::view::Game;

pub enum AgentKind {
    Bot(Bot),
    Net(Box<Net>),
    Process,
}

// TODO: figure out the : syntax while defining trait
// Also not sure the implications of Send
pub trait Agent: Debug + Send {
    fn call(&mut self, view: &Game) -> Result<Call>;
    fn play(&mut self, view: &Game) -> Result<Card>;
    // TODO: potentially important to send the periodic update to the user
    // particularly if it is a multi-player setup where each player waiting some time for each
    // player to make her move.
    // fn update(&self, view: &PlayerView);
}
