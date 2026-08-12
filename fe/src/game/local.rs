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
//! Only the human's seat is a `Transport`; the other three are the engine's own
//! `Random`, slowed so their plays are watchable. The UI still observes every turn
//! because the engine broadcasts the table to all four seats after each move,
//! so the bots' plays reach the screen through the human's transport without
//! the bots holding a channel of their own.

use super::view::Update;
use super::Session;
use callbreak::agent::strategy::{Random, Strategy};
use callbreak::agent::{
    Action, ClientMessage, Game as GameView, LegalMove, LegalMoves, Remote, ServerMessage,
    Transport,
};
use callbreak::game::Call;
use callbreak::{Error, Host};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::executor::block_on;
use futures::StreamExt;
use std::thread;
use std::time::Duration;

/// How long a bot lingers before answering, so each play is visible.
#[cfg(not(test))]
const BOT_DELAY: Duration = Duration::from_millis(550);
/// Tests play a whole match, so they cannot afford the pacing.
#[cfg(test)]
const BOT_DELAY: Duration = Duration::ZERO;

/// The human's seat: bridges the blocking `Host` loop to the UI.
#[derive(Debug)]
struct UiTransport {
    name: String,
    updates: UnboundedSender<Update>,
    moves: UnboundedReceiver<ClientMessage>,
}

impl Transport for UiTransport {
    fn send(&mut self, message: ServerMessage) -> Option<()> {
        let update = match message.action {
            // Somebody else moved: refresh the table, ask nothing.
            Action::Update => Update::table(&message.view, &self.name)?,
            _ => Update::prompt(&message, &self.name)?,
        };
        self.updates.unbounded_send(update).ok()
    }

    fn receive(&mut self) -> Option<ClientMessage> {
        // This runs on the engine thread and nowhere else, so parking it here
        // is exactly what `Transport::receive` is for. `None` means the window
        // closed, and this seat's fallback finishes the match in its place.
        block_on(self.moves.next())
    }
}

/// The engine's [`Random`], slowed down. Somebody is watching a local
/// table, so a seat that answers instantly reads as cards appearing from nowhere.
#[derive(Debug, Default)]
struct PacedBot {
    brain: Random,
}

impl Strategy for PacedBot {
    fn call(&mut self, view: &GameView) -> Call {
        thread::sleep(BOT_DELAY);
        self.brain.call(view)
    }

    fn play<'v>(&mut self, view: &'v GameView, moves: &LegalMoves<'v>) -> LegalMove<'v> {
        thread::sleep(BOT_DELAY);
        self.brain.play(view, moves)
    }
}

/// Seat the human, fill the rest of the table with bots, and start the engine.
///
/// The bot count is not a parameter: a local table is always padded to four and
/// `Game` shuffles the seats once the last player joins, so any split between
/// "bots seated before you" and "after you" is invisible by the time play starts.
pub fn start(name: String) -> Result<Session, Error> {
    let (update_tx, update_rx) = unbounded::<Update>();
    let (move_tx, move_rx) = unbounded::<ClientMessage>();

    let mut host = Host::default();
    host.add_agent(
        name.clone(),
        Remote::new(UiTransport {
            name: name.clone(),
            updates: update_tx.clone(),
            moves: move_rx,
        }),
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
        host.add_agent(id, PacedBot::default())?;
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
    use callbreak::agent::Action;
    use futures::executor::block_on;
    use futures::StreamExt;

    /// Play a whole match from the human seat, always answering with the first
    /// legal move the prompt offers.
    ///
    /// This covers the bridge end to end: the human seat, the three paced bot
    /// seats, and the fold in `view` that decides which cards a prompt offers.
    /// `Host::run` asserts that every card it is handed is legal, so a prompt
    /// offering one the engine would reject fails here.
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
                    Action::Call => ClientMessage::Call(Call::new(1).expect("1 is a legal call")),
                    Action::Update => unreachable!("a prompt never carries an update"),
                    Action::Break => ClientMessage::Break(
                        *playable
                            .first()
                            .expect("a break prompt offers a legal card"),
                    ),
                };
                moves
                    .unbounded_send(reply)
                    .expect("the engine is still listening");
            }
        });

        // Five rounds, each one call followed by thirteen plays.
        assert_eq!(prompts, 5 * 14);
    }
}
