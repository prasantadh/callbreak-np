use crate::game::{Call, Card, Trick};
use serde::{Deserialize, Serialize};

/// View of the game sent to each player
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub players: Vec<String>,
    pub rounds: Vec<Round>,
}

/// View of a round sent to each player
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Round {
    pub calls: [Option<Call>; 4],
    pub hand: Vec<Card>,
    pub tricks: Vec<Trick>,
}

/// Action that is requested of a player
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Call,
    Break,
    // TODO: might eventually have to include update on the action
}

/// Message sent to a human agent for action
// TODO: should this be sent to all agents as a shared communication mechanism?
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMessage {
    pub action: Action,
    pub view: Game,
}

/// Message expected from a human agent
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientMessage {
    Call(Call),
    Break(Card),
    //TODO: also allow clients to request for update?
}
