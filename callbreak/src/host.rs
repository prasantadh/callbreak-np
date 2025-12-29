use crate::{Game, agent::Agent, playerview::PlayerView};

struct Host {
    // TODO: I don't currently know the full implications of using Box<dyn> here. there seems to be
    // an altenative to use AgentKind enum with all options which seems to have performance
    // trade-offs
    agents: [Option<Box<dyn Agent>>; 4],
    game: Game,
}

impl Host {
    // add an agent
    pub(crate) fn add_agent(&mut self) {}
    pub(crate) fn run(&mut self) {
        // May be this is a great place to shuffle players instead of the Game?
        // FIXME: need to repeat all of the following for five rounds
        for (turn, agent) in self.agents.iter().flatten().enumerate() {
            let agent = agent.as_ref();
            let view = PlayerView::new(&self.game, "fixme".to_string());
            let call = agent.call(&view);
            // TODO: this call can return an error, in which case get a call from botagent
            self.game.call("fixme", call).unwrap();
        }
        // FIXME: need to repeat this loop again for 13 tricks
        for (turn, agent) in self.agents.iter().flatten().enumerate() {
            let agent = agent.as_ref();
            let view = PlayerView::new(&self.game, "fixme".to_string());
            // TODO: this play can return an error, in which case get a play from botagent
            let play = agent.play(&view);
            self.game.play("fixme", play).unwrap();
        }
        todo!()
    }
}
