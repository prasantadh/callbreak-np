use crate::game::{Call, Card, Trick};
use serde::{Deserialize, Serialize};

/// View of the whole game that will be sent to each player.
///
/// Just experimenting with adding more details
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerView {
    pub players: Vec<String>,
    pub rounds: Vec<RoundView>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoundView {
    pub calls: [Option<Call>; 4],
    pub hand: Vec<Card>,
    pub tricks: Vec<Trick>,
}
