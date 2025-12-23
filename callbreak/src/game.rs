use crate::Call;
use crate::Card;
use crate::Round;
use crate::Turn;
use crate::{Error, Result};
use crate::{Player, PlayerID};

use rand::{rng, seq::SliceRandom};
use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Game {
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
    pub fn new() -> Self {
        Self::default()
    }

    fn state(&self) -> State {
        if self.players.iter().any(|v| v.is_none()) {
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

    pub fn add_player(&mut self, id: &str) -> Result<()> {
        match self.state() {
            State::Lobby => {
                if self
                    .players
                    .iter()
                    .flatten()
                    .any(|player| player.get_id() == id)
                {
                    Err(Error::PlayerAlreadyInGame)
                } else {
                    self.players
                        .iter_mut()
                        .find(|slot| slot.is_none())
                        .expect("an empty slot is expected in the Lobby")
                        .replace(Player::new(id));
                    #[cfg(not(test))]
                    if self.state() == State::RoundInProgress {
                        let mut rng = rng();
                        self.players.shuffle(&mut rng);
                    }
                    Ok(())
                }
            }
            _ => Err(Error::NotAcceptingNewPlayers),
        }
    }

    pub fn get_turn(&self, player_id: &PlayerID) -> Result<Turn> {
        match self.state() {
            State::RoundInProgress => {
                if let Some(turn) = self
                    .players
                    .iter()
                    .flatten()
                    .position(|player| player.get_id() == player_id)
                {
                    Ok(Turn::new(turn))
                } else {
                    Err(Error::PlayerNotInGame)
                }
            }
            _ => Err(Error::NotYourTurn),
        }
    }

    pub fn call(&mut self, player_id: &PlayerID, call: Call) -> Result<()> {
        let turn = self.get_turn(player_id)?;
        match self.state() {
            State::RoundInProgress => {
                // is there a round that is initialized and not full?
                // yes, use this round, else start the next round
                if let Some(round) = self
                    .rounds
                    .iter_mut()
                    .rev()
                    .find_map(|v| v.as_mut())
                    .filter(|round| !round.is_over())
                {
                    round.call(call, turn)
                } else {
                    let slot = self
                        .rounds
                        .iter()
                        .position(|r| r.is_none())
                        .expect("must have available round when the game is not over");
                    self.rounds[slot] = Some(Round::new(Turn::new(slot)));
                    self.rounds[slot]
                        .as_mut()
                        .expect("just initialized round must be available")
                        .call(call, turn)
                }
            }
            _ => Err(Error::NotAcceptingCalls),
        }
    }

    pub fn play(&mut self, player_id: &PlayerID, card: Card) -> Result<()> {
        let turn = self.get_turn(player_id)?;
        match self.state() {
            State::RoundInProgress => {
                if let Some(round) = self.rounds.iter_mut().rev().find_map(|v| v.as_mut()) {
                    round.play(card, turn)
                } else {
                    Err(Error::NotAcceptingPlay) // particularly, still calling it seems
                }
            }
            _ => Err(Error::NotAcceptingPlay),
        }
    }
}
