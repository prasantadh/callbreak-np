use std::hint::select_unpredictable;

use crate::Call;
use crate::Card;
use crate::Hand;
use crate::Round;
use crate::Trick;
use crate::Turn;
use crate::{Error, Result};
use crate::{Player, PlayerID};

use rand::{rng, seq::SliceRandom};
use serde::Serialize;
use tracing::info;

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
                    Turn::new(turn)
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
                    // FIXME: feels like Turn can just return Self and not Result<Self>
                    // so we could create turn from any usize actually. right now
                    // it will crash everything so letting it pass
                    let starter = Turn::new(slot % 4).expect("must be a valid turn");
                    self.rounds[slot] = Some(Round::new(starter));
                    self.rounds[slot]
                        .as_mut()
                        .expect("just initialized round is available")
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

    /*
    pub fn get_valid_play(&self, player: &Player) -> Result<Vec<Card>> {
        // send the cards that are valid for the player play
        // in the current round
        let turn = self.get_turn(player)?;
        let current = self.current()?;
        println!(
            "get_valid_play: {player:?} of {turn:?} attempting move while current {current:?}"
        );
        if turn != current {
            return Err(Error::PlayerPlayedOutOfTurn);
        }
        match self.rounds.last() {
            None => Err(Error::NotEnoughPlayers),
            Some(round) => {
                let trick = round.current_trick()?;
                let valids = trick.valid_from_hand(self.get_hand(player)?)?;
                Ok(valids)
            }
        }
    }
    */
}
