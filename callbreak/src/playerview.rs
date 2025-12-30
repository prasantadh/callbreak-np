use serde::{Deserialize, Serialize};

use crate::{
    Game,
    game::{Call, Card, RoundId, Trick},
};

pub struct Context<'a> {
    game: &'a Game,
    player: &'a str,
}

impl<'a> Context<'a> {
    pub(crate) fn new(game: &'a Game, player: &'a str) -> Self {
        Self { game, player }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub players: Vec<String>,
    pub rounds: Vec<RoundView>, // FIXME: this struct needs to be enriched with the information that any bot might need to make
                                // decisions about call or play
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoundView {
    pub calls: [Option<Call>; 4],
    pub hand: Vec<Card>,
    pub tricks: Vec<Trick>,
}

impl PlayerView {}

impl<'a> From<Context<'a>> for PlayerView {
    fn from(ctx: Context<'a>) -> Self {
        let Context { game, player } = ctx;
        match game.get_current_round_id() {
            Ok(current_round_id) => {
                let mut rounds = vec![];
                for i in 0..=(current_round_id.into()) {
                    let round_id = RoundId::new(i).expect("must be a valid round id");
                    let round_view = RoundView {
                        calls: game.get_calls(round_id),
                        hand: if round_id == current_round_id {
                            game.get_hand(player)
                                .expect("must be a valid hand for this round")
                                .filter(|_| true)
                                .cloned()
                                .collect()
                        } else {
                            vec![]
                        },
                        tricks: game.get_tricks(round_id),
                    };
                    rounds.push(round_view);
                }
                PlayerView {
                    players: game.get_players().to_vec(),
                    rounds,
                }
            }
            Err(_) => PlayerView {
                players: game.get_players().to_vec(),
                rounds: vec![],
            },
        }
    }
}
