use super::{Agent, PlayerView};
use crate::{
    error::Error,
    game::{Call, Card},
};
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use tracing::debug;
use tungstenite::{Message, WebSocket};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Call,
    Break,
    // TODO: might eventually have to include update on the action
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMessage {
    action: Action,
    state: PlayerView,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientMessage {
    Call(Call),
    Break(Card),
    //TODO: also allow clients to request for update?
}

#[derive(Debug)]
pub struct Net {
    transport: WebSocket<TcpStream>,
}

impl Net {
    pub fn new(transport: WebSocket<TcpStream>) -> Self {
        Self { transport }
    }
}

impl Agent for Net {
    // add code here
    fn call(&mut self, view: &super::PlayerView) -> crate::error::Result<crate::game::Call> {
        // send a call message in json
        let view = view.clone();
        let message = ServerMessage {
            action: Action::Call,
            state: view,
        };
        let message = serde_json::to_string(&message).expect("must serialize without issue");
        self.transport
            .send(tungstenite::Message::Text(message.into()))
            .map_err(|_| Error::AgentSend)?;

        let c = ClientMessage::Call(Call::new(3).unwrap());
        let c = serde_json::to_string(&c).unwrap();
        debug!(?c);

        // deserialize the response into a ClientMessage then return to the server
        let response = self.transport.read().map_err(|_| Error::AgentRecv)?;
        let response = match response {
            Message::Text(v) => serde_json::from_str(v.as_str()).map_err(|_| Error::AgentRecv)?,
            _ => return Err(Error::AgentRecv),
        };
        match response {
            ClientMessage::Call(v) => Ok(v),
            _ => Err(Error::AgentRecv),
        }
    }

    fn play(&mut self, view: &super::PlayerView) -> crate::error::Result<crate::game::Card> {
        // send a break message in json
        let view = view.clone();
        let message = ServerMessage {
            action: Action::Break,
            state: view,
        };
        let message = serde_json::to_string(&message).expect("must serialize without issue");
        self.transport
            .send(tungstenite::Message::Text(message.into()))
            .map_err(|_| Error::AgentSend)?;

        // deserialize the response into a ClientMessage then return to the server
        let response = self.transport.read().map_err(|_| Error::AgentRecv)?;
        let response = match response {
            Message::Text(v) => serde_json::from_str(v.as_str()).map_err(|_| Error::AgentRecv)?,
            _ => return Err(Error::AgentRecv),
        };
        match response {
            ClientMessage::Break(v) => Ok(v),
            _ => Err(Error::AgentRecv),
        }
    }
}
