use crate::Error;
use crate::Game;
use crate::Result;
use crate::agent::{Game as GameView, Round as RoundView, Strategy};
use crate::game::Turn;
use rand::{rng, seq::SliceRandom};
use tracing::debug;

const SEATS: usize = 4;

/// What `seat` is allowed to know: everyone's calls and tricks, but only its own
/// hand. The engine hands over whole rounds and leaves the redaction here, so
/// the shape of a view can change without the engine hearing about it.
fn rounds_for(game: &Game, seat: Turn) -> Vec<RoundView> {
    game.rounds()
        .map(|round| RoundView {
            calls: round.get_calls(),
            hand: round.get_hand(seat).filter(|_| true).cloned().collect(),
            tricks: round.get_tricks().iter().flatten().cloned().collect(),
        })
        .collect()
}

/// `Host::default()` deals before anyone has sat down. Nobody can see a card
/// until the match runs, so dealing when the room opens rather than when it
/// fills changes nothing a player could observe.
#[derive(Debug, Default)]
pub struct Host {
    agents: Vec<(String, Box<dyn Strategy>)>,
    game: Game,
}

impl Host {
    /// Seats are shuffled once the table fills, so joining first does not mean
    /// leading first. From then on `agents` *is* the seating — index `i` is
    /// seat `i` — and nothing has to be looked up again.
    pub fn add_agent(&mut self, id: String, agent: impl Strategy + 'static) -> Result<()> {
        debug!(?id, ?agent, "attempting to add player to the game");
        if self.is_ready() {
            return Err(Error::TableIsFull);
        }
        if self.agents.iter().any(|(seated, _)| seated == &id) {
            return Err(Error::NameAlreadyTaken);
        }
        self.agents.push((id, Box::new(agent)));

        if self.is_ready() {
            self.agents.shuffle(&mut rng());
            let seating: Vec<&str> = self.agents.iter().map(|(id, _)| id.as_str()).collect();
            debug!(?seating, "seat order for the game");
        }
        Ok(())
    }

    /// `agents` is in seat order, so `players` is too.
    fn view_for(&self, seat: Turn) -> GameView {
        GameView {
            players: self.agents.iter().map(|(id, _)| id.clone()).collect(),
            rounds: rounds_for(&self.game, seat),
        }
    }

    /// Seating is this side's business, so this counts who has sat down rather
    /// than asking the engine.
    pub fn is_ready(&self) -> bool {
        self.agents.len() == SEATS
    }

    pub fn run(&mut self) -> Result<()> {
        if !self.is_ready() {
            return Err(Error::TableNotFull);
        }
        // Show everyone the deal before anyone is asked to act on it.
        self.broadcast();
        for _round in 0..5 {
            for _turn in 0..4 {
                let seat = self
                    .game
                    .turn()
                    .map_err(|_| Error::Internal("a running game must have a turn"))?;
                let view = self.view_for(seat);
                let call = self.agents[usize::from(seat)].1.call(&view);
                self.game
                    .call(seat, call)
                    .map_err(|_| Error::Internal("agents must handle calls without error"))?;
                self.broadcast();
            }

            for _trick in 0..13 {
                for _turn in 0..4 {
                    let seat = self
                        .game
                        .turn()
                        .map_err(|_| Error::Internal("a running game must have a turn"))?;
                    let view = self.view_for(seat);
                    // The seat is handed the legal set and returns one of it,
                    // so there is nothing to check on the way back.
                    let moves = view.legal_moves().ok_or(Error::Internal(
                        "a seat asked to play must have a legal move",
                    ))?;
                    let play = self.agents[usize::from(seat)].1.play(&view, &moves).card();
                    self.game
                        .play(seat, play)
                        .map_err(|_| Error::Internal("agents must handle play without error"))?;
                    self.broadcast();
                }
            }
        }
        Ok(())
    }
    // FIXME: at some point, I need to think about if I want to let the players who have
    // disconnected, resume the game

    fn broadcast(&mut self) {
        let players: Vec<String> = self.agents.iter().map(|(id, _)| id.clone()).collect();
        for (seat, (_, agent)) in self.agents.iter_mut().enumerate() {
            agent.observe(&GameView {
                players: players.clone(),
                rounds: rounds_for(&self.game, Turn::new(seat)),
            });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::agent::strategy::{Random, Sane};

    /// Seat order must not be join order, or whoever asks first always leads.
    #[test]
    fn seats_are_shuffled_as_the_table_fills() {
        let mut shuffled = false;
        for _ in 0..40 {
            let mut host = Host::default();
            for id in 0..4 {
                host.add_agent(format!("p{id}"), Random).unwrap();
            }
            let seating: Vec<&str> = host.agents.iter().map(|(id, _)| id.as_str()).collect();

            // Whatever the order, everyone who joined is seated exactly once.
            let mut sorted = seating.clone();
            sorted.sort();
            assert_eq!(sorted, vec!["p0", "p1", "p2", "p3"]);

            shuffled |= seating != vec!["p0", "p1", "p2", "p3"];
        }
        assert!(shuffled, "seats came out in join order every single time");
    }

    #[test]
    fn cannot_seat_the_same_name_twice() {
        let mut host = Host::default();
        host.add_agent("a".to_string(), Random).unwrap();
        assert_eq!(
            host.add_agent("a".to_string(), Random),
            Err(Error::NameAlreadyTaken)
        );
    }

    #[test]
    fn cannot_seat_a_fifth_player() {
        let mut host = Host::default();
        for id in 0..4 {
            host.add_agent(format!("p{id}"), Random).unwrap();
        }
        assert_eq!(
            host.add_agent("late".to_string(), Random),
            Err(Error::TableIsFull)
        );
    }

    /// The names the engine reports back must be the seating the host holds —
    /// they are now the same list, and this pins that they stay so.
    #[test]
    fn the_view_names_seats_in_the_hosts_own_order() {
        let mut host = Host::default();
        for id in 0..4 {
            host.add_agent(format!("p{id}"), Random).unwrap();
        }
        let seating: Vec<String> = host.agents.iter().map(|(id, _)| id.clone()).collect();
        for seat in 0..4 {
            assert_eq!(host.view_for(Turn::new(seat)).players, seating);
        }
    }

    /// The well-behaved case still plays through.
    #[test]
    fn the_shipped_strategies_play_a_match_to_completion() {
        let mut host = Host::default();
        host.add_agent("random".to_string(), Random).unwrap();
        host.add_agent("also_random".to_string(), Random).unwrap();
        host.add_agent("sane".to_string(), Sane::default()).unwrap();
        host.add_agent("also_sane".to_string(), Sane::default())
            .unwrap();
        assert_eq!(host.run(), Ok(()));
    }
}
