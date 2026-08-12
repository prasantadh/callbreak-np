use super::Transport;
use super::strategy::{Sane, Strategy};
use super::view::{Action, ClientMessage, Game, LegalMove, LegalMoves, ServerMessage};
use crate::game::Call;

#[derive(Debug)]
pub struct Remote<T: Transport> {
    transport: T,
    fallback: Box<dyn Strategy>,
}

impl<T: Transport> Remote<T> {
    pub fn new(transport: T) -> Self {
        Self::with_fallback(transport, Sane::default())
    }

    /// A remote seat with a stand-in of your choosing.
    pub fn with_fallback(transport: T, fallback: impl Strategy + 'static) -> Self {
        Self {
            transport,
            fallback: Box::new(fallback),
        }
    }

    /// One prompt, one reply. `None` covers a send that never landed and a
    /// client that had nothing to say.
    fn ask(&mut self, action: Action, view: &Game) -> Option<ClientMessage> {
        self.transport.send(ServerMessage {
            action,
            view: view.clone(),
        })?;
        self.transport.receive()
    }
}

// FIXME: there should likely be a debug message somewhere that the remote
// failed and the decision fell back to local strategy. also makes sense to
// implement somewhere that after three/X fallbacks, ditch remote and switch
// to local strategy fully for the remainder of the game
impl<T: Transport> Strategy for Remote<T> {
    fn call(&mut self, view: &Game) -> Call {
        match self.ask(Action::Call, view) {
            Some(ClientMessage::Call(call)) => call,
            _ => self.fallback.call(view),
        }
    }

    fn play<'v>(&mut self, view: &'v Game, moves: &LegalMoves<'v>) -> LegalMove<'v> {
        match self.ask(Action::Break, view) {
            // The one place an untrusted card enters the engine. `find` is the
            // parse: it yields proof this seat may play the card, or nothing —
            // in which case this seat's own stand-in covers the turn.
            Some(ClientMessage::Break(card)) => moves
                .find(card)
                .unwrap_or_else(|| self.fallback.play(view, moves)),
            _ => self.fallback.play(view, moves),
        }
    }

    fn observe(&mut self, view: &Game) {
        let _ = self.transport.send(ServerMessage {
            action: Action::Update,
            view: view.clone(),
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Host;
    use crate::agent::strategy::Random;
    use crate::game::{Card, Rank, Suit};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// A client that is simply not there.
    #[derive(Debug)]
    struct Dead;

    impl Transport for Dead {
        fn send(&mut self, _message: ServerMessage) -> Option<()> {
            None
        }

        fn receive(&mut self) -> Option<ClientMessage> {
            None
        }
    }

    /// Plays like [`Random`], and counts how many turns it was asked to cover.
    #[derive(Debug)]
    struct Stand(Arc<AtomicUsize>);

    impl Strategy for Stand {
        fn call(&mut self, _view: &Game) -> Call {
            self.0.fetch_add(1, Ordering::Relaxed);
            Call::new(1).expect("one is a legal call")
        }

        fn play<'v>(&mut self, _view: &'v Game, moves: &LegalMoves<'v>) -> LegalMove<'v> {
            self.0.fetch_add(1, Ordering::Relaxed);
            moves.first()
        }
    }

    /// The seat's own stand-in covers it, rather than the host's last resort.
    #[test]
    fn a_configured_fallback_plays_every_turn_the_client_cannot() {
        let turns = Arc::new(AtomicUsize::new(0));
        let mut host = Host::default();
        host.add_agent(
            "absent".to_string(),
            Remote::with_fallback(Dead, Stand(turns.clone())),
        )
        .unwrap();
        for id in 0..3 {
            host.add_agent(format!("bot{id}"), Random).unwrap();
        }
        host.run().unwrap();

        // Five rounds of one call and thirteen plays, none of them answered by
        // the client, all of them covered by the configured stand-in.
        assert_eq!(turns.load(Ordering::Relaxed), 5 * 14);
    }

    /// The same seat, but the client sends an illegal card rather than nothing.
    /// It is still the configured stand-in that covers the turn.
    #[test]
    fn a_configured_fallback_also_covers_an_illegal_reply() {
        let turns = Arc::new(AtomicUsize::new(0));
        let mut host = Host::default();
        host.add_agent(
            "cheat".to_string(),
            Remote::with_fallback(Cheat, Stand(turns.clone())),
        )
        .unwrap();
        for id in 0..3 {
            host.add_agent(format!("bot{id}"), Random).unwrap();
        }
        host.run().unwrap();

        // Every call (`Cheat` always answers with a card) plus every play where
        // it named a card it does not hold — it can hold the ace of spades at
        // most once, so at least twelve of its thirteen plays.
        let covered = turns.load(Ordering::Relaxed);
        assert!(
            covered >= 5 + 5 * 12,
            "stand-in covered only {covered} turns"
        );
    }

    /// A client that answers every prompt by naming one fixed card, whether or
    /// not it holds it and whether or not it was asked for a card at all.
    #[derive(Debug)]
    struct Cheat;

    impl Transport for Cheat {
        fn send(&mut self, _message: ServerMessage) -> Option<()> {
            Some(())
        }

        fn receive(&mut self) -> Option<ClientMessage> {
            Some(ClientMessage::Break(Card::new(Rank::Ace, Suit::Spades)))
        }
    }

    /// It can hold the ace of spades at most once, so most of its thirteen
    /// plays name a card it does not have.
    #[test]
    fn a_client_naming_an_illegal_card_cannot_end_the_game() {
        let mut host = Host::default();
        host.add_agent("cheat".to_string(), Remote::new(Cheat))
            .unwrap();
        for id in 0..3 {
            host.add_agent(format!("bot{id}"), Random).unwrap();
        }
        assert!(host.run().is_ok());
    }

    /// Records every action this seat is told about, in order.
    #[derive(Debug)]
    struct Spy(Arc<Mutex<Vec<Action>>>);

    impl Transport for Spy {
        fn send(&mut self, message: ServerMessage) -> Option<()> {
            self.0.lock().unwrap().push(message.action);
            Some(())
        }

        /// Never answers, so every turn of its own falls back to a bot. The
        /// point is what it is *told*, not what it plays.
        fn receive(&mut self) -> Option<ClientMessage> {
            None
        }
    }

    #[test]
    fn every_seat_is_told_about_moves_it_did_not_make() {
        let seen = Arc::new(Mutex::new(vec![]));
        let mut host = Host::default();
        host.add_agent("spy".to_string(), Remote::new(Spy(seen.clone())))
            .unwrap();
        for id in 0..3 {
            host.add_agent(format!("bot{id}"), Random).unwrap();
        }
        host.run().unwrap();

        let seen = seen.lock().unwrap();
        let updates = seen.iter().filter(|a| **a == Action::Update).count();
        let prompts = seen.iter().filter(|a| **a != Action::Update).count();

        // The opening deal, then every call and every play of the match — all
        // 56 moves a round, not just this seat's 14.
        assert_eq!(updates, 1 + 5 * (4 + 52));
        assert_eq!(prompts, 5 * 14);
    }
}
