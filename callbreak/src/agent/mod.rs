mod bot;
mod human;
mod view;

use crate::game::{Call, Card};
pub use bot::Bot;
pub use human::Human;
pub use human::Transport;
pub use view::{Action, ClientMessage, ServerMessage};
pub use view::{Game, Round};

#[derive(Debug)]
pub enum AgentKind {
    Bot(Bot),
    Human(Human),
}

impl AgentKind {
    pub(crate) fn call(&mut self, view: &Game) -> Call {
        match self {
            Self::Bot(bot) => bot.call(view),
            Self::Human(human) => human.call(view),
        }
    }

    pub(crate) fn play(&mut self, view: &Game) -> Card {
        match self {
            Self::Bot(bot) => bot.play(view),
            Self::Human(human) => human.play(view),
        }
    }

    // TODO: potentially important to send the periodic update to the user
    // particularly if it is a multi-player setup where each player waiting some time for each
    // player to make her move.
    // pub(crate) fn update(&self, view: &PlayerView);
}
