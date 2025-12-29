use crate::Game;

// a player see
//      all the players id
//      for each round:
//          - all the calls
//          - all the tricks
//          - her hand (and no one else's hand)
pub struct PlayerView {
    players: [String; 4],
    // FIXME: this struct needs to be enriched with the information that any bot might need to make
    // decisions about call or play
}

impl PlayerView {
    pub(crate) fn new(game: &Game, player: String) -> Self {
        todo!()
    }
}

// FIXME: would be nice to have PlayerView::From(Game)
