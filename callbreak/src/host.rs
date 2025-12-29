use crate::Result;
use crate::playerview::Context;
use crate::{Game, agent::Agent, playerview::PlayerView};

struct Host {
    // TODO: I don't currently know the full implications of using Box<dyn> here. there seems to be
    // an altenative to use AgentKind enum with all options which seems to have performance
    // trade-offs
    agents: Vec<(String, Box<dyn Agent>)>,
    game: Game,
}

impl Host {
    pub(crate) fn new() -> Self {
        Host {
            game: Game::new(),
            agents: vec![],
        }
    }

    // add an agent
    pub(crate) fn add_agent(&mut self, id: String, agent: Box<dyn Agent>) -> Result<()> {
        self.game.add_player(&id)?;
        self.agents.push((id, agent));
        Ok(())
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.game.is_ready()
    }

    pub(crate) fn run(&mut self) {
        for _round in 0..5 {
            // request a call
            for _turn in 0..4 {
                let player = self.game.turn().expect("the next turn must be available");
                let (_, agent) = self
                    .agents
                    .iter()
                    .find(|(id, _)| id == &player)
                    .expect("player must be in agents list");
                let view = PlayerView::from(Context::new(&self.game, player));
                agent
                    .call(&view)
                    .expect("FIXME: when error, swap out with a bot");
            }

            // request a break
            for _trick in 0..13 {
                for _turn in 0..4 {
                    let player = self.game.turn().expect("the next turn must be available");
                    let (_, agent) = self
                        .agents
                        .iter()
                        .find(|(id, _)| id == &player)
                        .expect("player must be in agents list");
                    let view = PlayerView::from(Context::new(&self.game, player));
                    agent
                        .play(&view)
                        .expect("FIXME: when error, swap out with a bot");
                }
            }
        }
    }
}
