use crate::Result;
use crate::playerview::Context;
use crate::{Game, agent::Agent, playerview::PlayerView};
use tracing::debug;

#[derive(Default)]
pub struct Host {
    // TODO: I don't currently know the full implications of using Box<dyn> here. there seems to be
    // an altenative to use AgentKind enum with all options which seems to have performance
    // trade-offs
    agents: Vec<(String, Box<dyn Agent>)>,
    game: Game,
}

impl Host {
    pub fn new() -> Self {
        Self::default()
    }

    // add an agent
    pub fn add_agent(&mut self, id: String, agent: Box<dyn Agent>) -> Result<()> {
        debug!(?id, ?agent, "attempting to add player to the game");
        self.game.add_player(&id)?;
        self.agents.push((id, agent));
        Ok(())
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.game.is_ready()
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
                    .iter()
                    .find(|(id, _)| id == &player)
                    .expect("player must be in agents list");
                let view = PlayerView::from(Context::new(&self.game, &player));
                let call = agent
                    .call(&view)
                    .expect("FIXME: when error, swap out with a bot");
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
                        .iter()
                        .find(|(id, _)| id == &player)
                        .expect("player must be in agents list");
                    let view = PlayerView::from(Context::new(&self.game, &player));
                    let play = agent
                        .play(&view)
                        .expect("FIXME: when error, swap out with a bot");
                    self.game.play(&player, play).expect("FIXME: if this errors, return error to agent or make a move from the unfallible bot");
                }
            }
        }
    }
}
