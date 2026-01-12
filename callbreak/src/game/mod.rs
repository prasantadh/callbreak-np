mod call;
mod deck;
mod hand;
mod round;
mod trick;
mod turn;

pub use call::Call;
pub(crate) use deck::Deck;
pub use deck::{Card, Rank, Suit};
pub use hand::Hand;
pub use round::RoundId;
pub use trick::Trick;

use crate::{Error, PlayerView, Result, RoundView};
use rand::{rng, seq::SliceRandom};
use round::Round;
use serde::Serialize;
use std::array;
use tracing::debug;
use turn::Turn;

type Player = String;

#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Game {
    players: [Option<Player>; 4],
    rounds: [Option<Round>; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    Lobby,
    RoundInProgress,
    Over,
}

impl Game {
    fn state(&self) -> State {
        if self.players.iter().any(|v| v.as_ref().is_none()) {
            State::Lobby
        } else if self
            .rounds
            .iter()
            .all(|round| round.as_ref().is_some_and(|v| v.is_over()))
        {
            State::Over
        } else {
            State::RoundInProgress
        }
    }

    pub(crate) fn add_player(&mut self, id: &str) -> Result<()> {
        match self.state() {
            State::Lobby => {
                if self.players.iter().flatten().any(|player| player == id) {
                    Err(Error::PlayerAlreadyInGame)
                } else {
                    self.players
                        .iter_mut()
                        .find(|slot| slot.is_none())
                        .expect("an empty slot is expected in the Lobby")
                        .replace(id.to_string());
                    if self.state() == State::RoundInProgress {
                        let mut rng = rng();
                        self.players.shuffle(&mut rng);
                        self.rounds[0] = Some(Round::new(Turn::new(0)));
                        let players: Vec<String> = self
                            .players
                            .iter()
                            .flatten()
                            .map(|p| p.to_string())
                            .collect();
                        debug!(?players, "players order after shuffle for the game");
                    }
                    Ok(())
                }
            }
            _ => Err(Error::NotAcceptingNewPlayers),
        }
    }

    fn player_id_to_turn(&self, player_id: &str) -> Result<Turn> {
        match self.state() {
            State::RoundInProgress => {
                if let Some(turn) = self
                    .players
                    .iter()
                    .flatten()
                    .position(|player| player == player_id)
                {
                    Ok(Turn::new(turn))
                } else {
                    Err(Error::PlayerNotInGame)
                }
            }
            _ => Err(Error::NotYourTurn),
        }
    }

    pub(crate) fn call(&mut self, player_id: &str, call: Call) -> Result<()> {
        match self.state() {
            State::RoundInProgress => {
                let turn = self.player_id_to_turn(player_id)?;
                self.rounds
                    .iter_mut()
                    .rev()
                    .find_map(|v| v.as_mut())
                    .expect("must have an active round in this state")
                    .call(call, turn)
            }
            _ => Err(Error::NotAcceptingCalls),
        }
    }

    pub(crate) fn play(&mut self, player_id: &str, card: Card) -> Result<()> {
        match self.state() {
            State::RoundInProgress => {
                let turn = self.player_id_to_turn(player_id)?;
                let slot = self
                    .rounds
                    .iter()
                    .position(|r| r.as_ref().is_some_and(|r| !r.is_over()))
                    .expect("must have an active round in this state");
                self.rounds[slot]
                    .as_mut()
                    .expect("must have an active round in this state")
                    .play(card, turn)?;
                if slot != 4 && self.rounds[slot].as_ref().unwrap().is_over() {
                    self.rounds[slot + 1] = Some(Round::new(Turn::new(slot + 1)));
                }
                Ok(())
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }

    pub(crate) fn turn(&self) -> Result<String> {
        match self.state() {
            State::RoundInProgress => {
                let turn = self
                    .rounds
                    .iter()
                    .rev()
                    .find_map(|r| r.as_ref())
                    .expect("must always have Some(round)")
                    .turn()?;
                Ok(self.players[turn]
                    .as_ref()
                    .expect("must have all players in this state")
                    .to_string())
            }
            _ => Err(Error::NotYourTurn),
        }
    }

    pub(crate) fn get_valid_moves(&self, player: &str) -> Result<Vec<Card>> {
        match self.state() {
            State::RoundInProgress => {
                let turn = self.player_id_to_turn(player)?;
                self.rounds
                    .iter()
                    .rev()
                    .find_map(|r| r.as_ref())
                    .expect("must always have Some(round)")
                    .get_valid_moves(turn)
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }

    pub(crate) fn get_hand(&self, player: &str) -> Result<Hand> {
        let turn = self.player_id_to_turn(player)?;
        Ok(self
            .rounds
            .iter()
            .rev()
            .find_map(|r| r.as_ref())
            .expect("must always have Some(round)")
            .get_hand(turn)
            .clone())
    }

    pub(crate) fn get_players(&self) -> Vec<String> {
        self.players
            .iter()
            .flatten()
            .map(|p| p.to_string())
            .collect()
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.state() == State::RoundInProgress
    }

    pub(crate) fn is_over(&self) -> bool {
        self.state() == State::Over
    }

    fn get_calls(&self, round_id: RoundId) -> [Option<Call>; 4] {
        match &self.rounds[round_id] {
            Some(round) => round.get_calls().clone(),
            None => array::from_fn(|_| None),
        }
    }

    pub(crate) fn get_current_round_id(&self) -> Result<RoundId> {
        match self.state() {
            State::RoundInProgress => {
                let id = self.rounds.iter().flatten().count() - 1;
                Ok(RoundId::new(id).expect("must be a valid round id"))
            }
            _ => Err(Error::RoundNotInProgress),
        }
    }

    // TODO: may be the return type for this function needs to be an option?
    pub(crate) fn get_tricks(&self, round_id: RoundId) -> Vec<Trick> {
        match &self.rounds[round_id] {
            Some(round) => round.get_tricks().iter().flatten().cloned().collect(),
            None => vec![],
        }
    }

    pub(crate) fn build_view_for(&self, player: &Player) -> Result<PlayerView> {
        match self.state() {
            State::Lobby => Ok(PlayerView {
                players: self.players.iter().flatten().cloned().collect(),
                rounds: vec![],
            }),
            _ => {
                // there is more to do here
                let mut rounds = vec![];
                for round in self.rounds.iter().flatten() {
                    let roundview = RoundView {
                        calls: *round.get_calls(),
                        hand: round
                            .get_hand(self.player_id_to_turn(player)?)
                            .filter(|_| true)
                            .cloned()
                            .collect(),
                        tricks: round.get_tricks().iter().flatten().cloned().collect(),
                    };
                    rounds.push(roundview);
                }
                Ok(PlayerView {
                    players: self.players.iter().flatten().cloned().collect(),
                    rounds,
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::Rank::*;
    use crate::game::Suit::*;

    #[test]
    fn can_play_to_completion() {
        let mut game = Game::default();

        for turn in 0..4 {
            game.add_player(turn.to_string().as_str()).unwrap();
        }

        for _round in 0..5 {
            for _turn in 0..4 {
                let player = game.turn().unwrap();
                game.call(&player, Call::new(3).unwrap()).unwrap();
            }
            for _trick in 0..13 {
                for _turn in 0..4 {
                    let player = game.turn().unwrap();
                    let moves = game.get_valid_moves(&player).unwrap();
                    game.play(&player, *moves.first().unwrap()).unwrap();
                }
            }
        }
    }

    #[test]
    fn cannot_add_same_player_twice() {
        let mut game = Game::default();
        game.add_player("0").unwrap();
        let action = game.add_player("0");
        assert_eq!(action, Err(Error::PlayerAlreadyInGame))
    }

    #[test]
    fn cannot_add_more_than_four_players() {
        let mut game = Game::default();
        for i in 0..4 {
            game.add_player(&i.to_string()).unwrap();
        }
        let action = game.add_player("0");
        assert_eq!(action, Err(Error::NotAcceptingNewPlayers))
    }

    #[test]
    fn cannot_call_before_all_players_have_joined() {
        let mut game = Game::default();
        game.add_player("0").unwrap();
        let action = game.call("0", Call::new(3).unwrap());
        assert_eq!(action, Err(Error::NotAcceptingCalls))
    }

    #[test]
    fn cannot_call_out_of_turn() {
        let mut game = Game::default();
        for i in 0..4 {
            game.add_player(i.to_string().as_str()).unwrap();
        }
        let player = game.turn().unwrap();
        let action = match player.as_str() {
            "1" => game.call("2", Call::new(3).unwrap()),
            _ => game.call("1", Call::new(3).unwrap()),
        };
        assert_eq!(action, Err(Error::NotYourTurn));
    }

    #[test]
    fn cannot_play_while_in_lobby() {
        let mut game = Game::default();
        for i in 0..3 {
            game.add_player(&i.to_string()).unwrap();
        }
        let action = game.play("0", Card::new(Seven, Clubs));
        assert_eq!(action, Err(Error::NotAcceptingPlay))
    }

    #[test]
    fn each_player_has_a_spade_card_and_a_face_card() {
        let mut game = Game::default();
        for i in 0..4 {
            game.add_player(&i.to_string()).unwrap();
        }
        for i in 0..4 {
            assert!(
                game.get_hand(&i.to_string())
                    .unwrap()
                    .filter(|_| true)
                    .any(|c| c.get_suit() == Spades),
            );
            assert!(
                game.get_hand(&i.to_string())
                    .unwrap()
                    .filter(|_| true)
                    .any(|c| c.get_rank() >= Jack)
            );
        }
    }

    #[test]
    fn round_i_must_be_started_by_turn_i() {
        let mut game = Game::default();

        for turn in 0..4 {
            game.add_player(turn.to_string().as_str()).unwrap();
        }
        let players = game.get_players();

        for round in 0..5 {
            for turn in 0..4 {
                let mut player = game.turn().unwrap();
                if turn == 0 {
                    // starter must be player i for Round i
                    player = players[Turn::new(round)].clone();
                }
                game.call(&player, Call::new(3).unwrap()).unwrap();
            }
            for trick in 0..13 {
                for turn in 0..4 {
                    let mut player = game.turn().unwrap();
                    if (trick, turn) == (0, 0) {
                        // starter must be player i for Round i
                        player = players[Turn::new(round)].clone();
                    };
                    let moves = game.get_valid_moves(&player).unwrap();
                    game.play(&player, *moves.first().unwrap()).unwrap();
                }
            }
        }
    }

    #[test]
    fn cannot_play_out_of_turn() {
        let mut game = Game::default();
        for turn in 0..4 {
            game.add_player(turn.to_string().as_str()).unwrap();
        }
        for _ in 0..4 {
            let player = game.turn().unwrap();
            game.call(&player, Call::new(3).unwrap()).unwrap();
        }
        let players = game.get_players();
        let action = game.play(&players[1], Card::new(Six, Diamonds));
        // ^ what card is used doesn't really matter
        assert_eq!(action, Err(Error::NotYourTurn));
    }
}
