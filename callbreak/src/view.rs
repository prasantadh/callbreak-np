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
