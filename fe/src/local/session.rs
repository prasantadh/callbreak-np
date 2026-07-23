//! Runs a whole game locally: a `callbreak::Host` on a background thread with
//! three bot seats and one seat for the person at the keyboard. This mirrors
//! `cli-fe`'s host mode — no API, no network.
//!
//! `Host::run` is a blocking loop and every `Transport` method is synchronous,
//! so the engine cannot share the UI thread. The two directions want different
//! channels:
//!
//! - updates travel engine → UI over a futures channel, because the UI awaits
//!   them from a Dioxus task (`unbounded_send` is callable from a plain thread);
//! - moves travel UI → engine over a std channel, because the engine's
//!   `Transport::receive` wants exactly a blocking `recv`.
//!
//! Every seat gets a transport we own, so the UI observes each turn rather than
//! only the human's. Bot seats pause briefly before answering so their plays are
//! watchable.

use super::view::{first_valid, Update};
use callbreak::agent::{Action, AgentKind, ClientMessage, Human, ServerMessage, Transport};
use callbreak::game::Call;
use callbreak::{Error, Host};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// How long a bot lingers before answering, so each play is visible.
#[cfg(not(test))]
const BOT_DELAY: Duration = Duration::from_millis(550);
/// Tests play a whole match, so they cannot afford the pacing.
#[cfg(test)]
const BOT_DELAY: Duration = Duration::ZERO;

/// A fallback answer for when the UI has gone away. On a play turn the wrong
/// variant makes the engine defer to a bot (see `callbreak::agent::Human`), so
/// this winds the host loop down instead of hanging it.
fn fallback() -> ClientMessage {
    ClientMessage::Call(Call::new(1).expect("1 is a legal call"))
}

/// The live ends of a running game: updates to render, moves to submit.
pub struct Session {
    pub updates: UnboundedReceiver<Update>,
    pub moves: Sender<ClientMessage>,
}

/// The human's seat: bridges the blocking `Host` loop to the UI.
#[derive(Debug)]
struct UiTransport {
    name: String,
    updates: UnboundedSender<Update>,
    moves: Receiver<ClientMessage>,
}

impl Transport for UiTransport {
    fn send(&mut self, message: ServerMessage) -> Option<()> {
        self.updates
            .unbounded_send(Update::prompt(&message, &self.name))
            .ok()
    }

    fn receive(&mut self) -> ClientMessage {
        self.moves.recv().unwrap_or_else(|_| fallback())
    }
}

/// A bot seat that also lets the UI spectate. It plays like `callbreak::Bot`
/// but forwards a table update on every turn.
#[derive(Debug)]
struct BotTransport {
    /// The human's name, so table updates resolve the viewer's seat correctly.
    human: String,
    updates: UnboundedSender<Update>,
    /// The last prompt, kept so `receive` can compute this seat's reply.
    last: Option<ServerMessage>,
}

impl Transport for BotTransport {
    fn send(&mut self, message: ServerMessage) -> Option<()> {
        let sent = self
            .updates
            .unbounded_send(Update::table(&message, &self.human))
            .ok();
        self.last = Some(message);
        sent
    }

    fn receive(&mut self) -> ClientMessage {
        thread::sleep(BOT_DELAY);
        match self.last.take() {
            Some(message) => match message.action {
                Action::Call => fallback(),
                Action::Break => ClientMessage::Break(first_valid(&message.view)),
            },
            None => fallback(),
        }
    }
}

/// Seat the human, fill the rest of the table with bots, and start the engine.
///
/// The bot count is not a parameter: a local table is always padded to four and
/// `Game` shuffles the seats once the last player joins, so any split between
/// "bots seated before you" and "after you" is invisible by the time play starts.
pub fn start(name: String) -> Result<Session, Error> {
    let (update_tx, update_rx) = unbounded::<Update>();
    let (move_tx, move_rx) = mpsc::channel::<ClientMessage>();

    let mut host = Host::new();
    host.add_agent(
        name.clone(),
        AgentKind::Human(Human::new(Box::new(UiTransport {
            name: name.clone(),
            updates: update_tx.clone(),
            moves: move_rx,
        }))),
    )?;

    // Fill the remaining seats. The ids skip anything the human already took,
    // since the engine rejects a duplicate name.
    let mut next = 0;
    while !host.is_ready() {
        let id = loop {
            let id = format!("bot{next}");
            next += 1;
            if id != name {
                break id;
            }
        };
        host.add_agent(
            id,
            AgentKind::Human(Human::new(Box::new(BotTransport {
                human: name.clone(),
                updates: update_tx.clone(),
                last: None,
            }))),
        )?;
    }

    // Drop our copy so the UI sees the channel close when the game thread
    // (owning every transport's clone) finishes.
    drop(update_tx);

    // The host loop is blocking; run it off-thread so the UI stays responsive.
    // Nothing joins it: if the window closes, dropping `move_tx` unblocks
    // `receive` and the loop winds itself down on fallback answers.
    thread::spawn(move || host.run());

    Ok(Session {
        updates: update_rx,
        moves: move_tx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::StreamExt;

    /// Play a whole match from the human seat, always answering with the first
    /// legal move the prompt offers.
    ///
    /// This covers the bridge end to end, and doubles as a check on the rules
    /// ported into `view`: `Host::run` asserts that every card it is handed is
    /// one the engine considers legal, so a prompt offering a card the engine
    /// would reject fails here rather than in front of a player.
    #[test]
    fn plays_a_full_match() {
        let session = start("tester".to_string()).expect("a local game seats four");
        let moves = session.moves;
        let mut updates = session.updates;
        let mut prompts = 0;

        block_on(async {
            while let Some(update) = updates.next().await {
                let Update::Prompt {
                    action, playable, ..
                } = update
                else {
                    continue;
                };
                prompts += 1;
                let reply = match action {
                    Action::Call => fallback(),
                    Action::Break => ClientMessage::Break(
                        *playable.first().expect("a break prompt offers a legal card"),
                    ),
                };
                moves.send(reply).expect("the engine is still listening");
            }
        });

        // Five rounds, each one call followed by thirteen plays.
        assert_eq!(prompts, 5 * 14);
    }
}
