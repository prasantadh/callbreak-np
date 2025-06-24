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
pub struct Callbreak {
    players: Vec<Player>,
    rounds: Vec<Round>,
}

impl Callbreak {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_player<S: Into<PlayerID>>(&mut self, id: S) -> Result<&Player> {
        if self.players.len() == 4 {
            return Err(Error::GameHasMaxPossiblePlayers);
        }

        let id: PlayerID = id.into();
        for player in self.players.iter() {
            if id == *player.get_id() {
                return Err(Error::PlayerAlreadyInGame);
            }
        }

        info!("player: {id:?} added");
        self.players.push(Player::new(id));
        if self.players.len() == 4 {
            // may be I just shuffle the player array instead of assigning turns
            // TODO: might also be nice to make shuffling players optional
            let mut rng = rng();
            self.players.shuffle(&mut rng);
            self.add_round()?;
        }
        Ok(self.players.last().unwrap())
    }

    pub fn current(&self) -> Result<Turn> {
        match self.rounds.last() {
            None => Err(Error::NotEnoughPlayers),
            Some(round) => round.current(),
        }
    }

    pub fn get_turn(&self, player: &Player) -> Result<Turn> {
        let turn = self
            .players
            .iter()
            .position(|p| *p == *player)
            .ok_or(Error::PlayerNotInGame)?;
        Turn::new(turn)
    }

    pub fn get_calls(&self) -> Result<&[Option<Call>]> {
        // TODO: this is currently only sending for the last round,
        // when we should be sending back for all the rounds with this function
        match self.rounds.last() {
            None => Err(Error::NotEnoughPlayers),
            Some(round) => Ok(round.get_calls()),
        }
    }

    pub fn get_hand(&self, player: &Player) -> Result<&Hand> {
        match self.rounds.last() {
            None => Err(Error::NotEnoughPlayers),
            Some(round) => {
                let turn = self.get_turn(player)?;
                Ok(round.get_hand(turn))
            }
        }
    }

    pub fn call(&mut self, player: &Player, call: Call) -> Result<()> {
        let turn = self.get_turn(player)?;
        info!("player {player:?} called {call:?}");
        match self.rounds.last_mut() {
            None => Err(Error::NotEnoughPlayers),
            Some(round) => round.call(call, turn),
        }
    }

    pub fn play(&mut self, player: &Player, card: Card) -> Result<()> {
        let turn = self.get_turn(player)?;
        match self.rounds.last_mut() {
            None => return Err(Error::NotEnoughPlayers),
            Some(round) => round.play(card, turn)?,
        };

        let round = self.rounds.last().unwrap();
        if round.is_over() && self.rounds.len() != 5 {
            self.rounds
                .push(Round::new(Turn::new(self.rounds.len() % 4)?));
        }
        Ok(())
    }

    fn add_round(&mut self) -> Result<()> {
        if self.players.len() < 4 {
            return Err(Error::NotEnoughPlayers);
        }

        match self.rounds.last() {
            None => {
                self.rounds.push(Round::new(Turn::new(0)?));
                info!("round 0 added: {:?} is first to play", self.current());
            }
            Some(r) => {
                let round_id = self.rounds.len();
                if r.is_over() && round_id == 5 {
                    return Err(Error::GameOver);
                } else if r.is_over() {
                    let turn = self.rounds.len() % 4;
                    self.rounds.push(Round::new(Turn::new(turn)?));
                    info!("round {round_id:?} added");
                } else {
                    return Err(Error::RoundIsNotOver);
                }
            }
        };
        Ok(())
    }

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

    pub fn get_players(&self) -> &[Player] {
        &self.players
    }

    pub fn get_trick(&self) -> Result<&Trick> {
        self.rounds
            .last()
            .ok_or(Error::NotEnoughPlayers)?
            .current_trick()
    }
}
