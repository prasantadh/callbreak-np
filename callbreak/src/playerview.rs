use serde::{Deserialize, Serialize};

use crate::Game;

// a player see
//      all the players id
//      for each round:
//          - all the calls
//          - all the tricks
//          - her hand (and no one else's hand)
pub struct Context<'a> {
    game: &'a Game,
    player: String,
}

impl<'a> Context<'a> {
    pub(crate) fn new(game: &'a Game, player: String) -> Self {
        Self { game, player }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub players: Vec<String>,
    // FIXME: this struct needs to be enriched with the information that any bot might need to make
    // decisions about call or play
}

impl PlayerView {}

impl<'a> From<Context<'a>> for PlayerView {
    fn from(ctx: Context<'a>) -> Self {
        let Context { game, player } = ctx;
        PlayerView {
            players: game.get_players().to_vec(),
        }
    }
}
