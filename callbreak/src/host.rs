use crate::Game;
use crate::Result;
use crate::agent::AgentKind;
use tracing::debug;

#[derive(Default, Debug)]
pub struct Host {
    // TODO: I don't currently know the full implications of using Box<dyn> here. there seems to be
    // an altenative to use AgentKind enum with all options which seems to have performance
    // trade-offs
    agents: Vec<(String, AgentKind)>,
    game: Game,
}

impl Host {
    pub fn new() -> Self {
        Self::default()
    }

    // add an agent
    pub fn add_agent(&mut self, id: String, agent: AgentKind) -> Result<()> {
        debug!(?id, ?agent, "attempting to add player to the game");
        self.game.add_player(&id)?;
        self.agents.push((id, agent));
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.game.is_ready()
    }

    pub fn is_over(&self) -> bool {
        self.game.is_over()
    }

    pub fn run(&mut self) {
        // FIXME: should return an error if there are not currently 4 players
        for _round in 0..5 {
            // request a call
            debug!(?_round);
            for _turn in 0..4 {
                let player = self.game.turn().expect("the next turn must be available");
                debug!(?player, "requesting call from ");
                let (_, agent) = self
                    .agents
                    .iter_mut()
                    .find(|(id, _)| id == &player)
                    .expect("player must be in agents list");
                let playerview = self
                    .game
                    .build_view_for(&player)
                    .expect("must have a view for this player");
                debug!(?playerview);
                let call = agent.call(&playerview);
                self.game.call(&player, call).expect(
                    "FIXME: if this errors, return error to agent. should make unfallible bot",
                );
            }

            // request a break
            for _trick in 0..13 {
                debug!(?_trick);
                for _turn in 0..4 {
                    let player = self.game.turn().expect("the next turn must be available");
                    debug!(?player, "requesting break from");
                    let (_, agent) = self
                        .agents
                        .iter_mut()
                        .find(|(id, _)| id == &player)
                        .expect("player must be in agents list");
                    let playerview = self
                        .game
                        .build_view_for(&player)
                        .expect("must have a view for this player");
                    // FIXME: figure out a better way of displaying playerview
                    debug!(?playerview);
                    let play = agent.play(&playerview);
                    // FIXME: temporary fix to get rid of dead code warning. but if this condition
                    // is not met, the user is messing around with us. swap out with a bot.
                    assert!(self.game.get_valid_moves(&player).unwrap().contains(&play));
                    self.game.play(&player, play).expect("FIXME: if this errors, return error to agent or make a move from the unfallible bot");
                }
            }
        }
    }
}
