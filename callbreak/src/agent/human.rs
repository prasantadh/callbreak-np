use super::view::Game;
use super::view::{Action, ClientMessage, ServerMessage};
use crate::game::{Call, Card};
use std::fmt::Debug;

/// Mechanism for communication with a human
pub trait Transport: Send + Debug {
    fn send(&mut self, message: ServerMessage) -> Option<()>;
    fn receive(&mut self) -> ClientMessage;
}

/// Agent to facilitate interaction with a human
#[derive(Debug)]
pub struct Human {
    transport: Box<dyn Transport>,
}

impl Human {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }

    pub(super) fn call(&mut self, view: &Game) -> Call {
        let message = ServerMessage {
            action: Action::Call,
            view: view.clone(),
        };
        // FIXME: because others are allowed to implement the transport
        // may be this should have a timeout? But also only we would
        // implement timer for our instance of this game so perhaps it is okay to implement timer
        // in the api? might still be cool to take the timeout as a parameter for Human
        // and use that as a timeout here though. Same goes for play()
        if self.transport.send(message).is_none() {
            let bot = super::Bot;
            return bot.call(view);
        }

        let message = self.transport.receive();
        match message {
            ClientMessage::Call(v) => v,
            _ => {
                let bot = super::Bot;
                bot.call(view)
            }
        }
    }

    pub(super) fn play(&mut self, view: &Game) -> Card {
        let message = ServerMessage {
            action: Action::Break,
            view: view.clone(),
        };
        if self.transport.send(message).is_none() {
            let bot = super::Bot;
            return bot.play(view);
        }
        let message = self.transport.receive();
        match message {
            // FIXME: this card should be in the list of valid moves or else we should use a bot to
            // provide a valid move
            ClientMessage::Break(card) => card,
            _ => {
                let bot = super::Bot;
                bot.play(view)
            }
        }
    }
}
