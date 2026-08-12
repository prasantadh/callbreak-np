//! Local game: a `callbreak::Host` runs a full match straight through the
//! crate, no API in the loop.
//!
//! Only the human holds a `Transport`. Every other seat is the engine's own
//! `Random`, wrapped in a delay so its plays are watchable. The UI still sees
//! every turn because the engine broadcasts the table to all four seats after
//! each move, so the human's transport is told about the bots' plays without
//! the bots needing a channel of their own.

use crate::game_view::Update;
use crate::tui;
use callbreak::Host;
use callbreak::agent::Game as GameView;
use callbreak::agent::strategy::{Random, Strategy};
use callbreak::agent::{
    Action, ClientMessage, LegalMove, LegalMoves, Remote, ServerMessage, Transport,
};
use callbreak::game::Call;
use std::error::Error;
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// How long a bot lingers before answering, so each play is visible.
const BOT_DELAY: Duration = Duration::from_millis(550);

/// The human's seat: bridges the blocking `Host` loop to the UI. `send`/`receive`
/// each hop a channel to the UI thread, mirroring the api's `AxumTransport`.
#[derive(Debug)]
struct TuiTransport {
    name: String,
    updates: Sender<Update>,
    moves: Receiver<ClientMessage>,
}

impl Transport for TuiTransport {
    fn send(&mut self, message: ServerMessage) -> Option<()> {
        let update = match message.action {
            // Somebody else moved: refresh the table, ask nothing.
            Action::Update => Update::table(&message, &self.name)?,
            _ => Update::prompt(&message, &self.name)?,
        };
        self.updates.send(update).ok()
    }

    fn receive(&mut self) -> Option<ClientMessage> {
        // Block until the UI reports the move. A closed channel means the UI is
        // gone; saying so lets this seat's fallback finish the match in this
        // seat instead of hanging.
        self.moves.recv().ok()
    }
}

/// The engine's `Random`, slowed down. A local table is watched by a person, so a
/// seat that answers instantly reads as cards appearing from nowhere.
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

pub fn run(bots: usize, name: Option<String>) -> Result<(), Box<dyn Error>> {
    let name = match name {
        Some(name) => name,
        None => prompt_name()?,
    };

    let (update_tx, update_rx) = mpsc::channel::<Update>();
    let (move_tx, move_rx) = mpsc::channel::<ClientMessage>();

    // Seat the requested bots (capped so the human always gets a seat), the
    // human, then pad with bots until the table is full — a local game is
    // always immediately playable.
    let mut host = Host::default();
    let bots = bots.min(3);
    let seat_bot = |host: &mut Host, id: usize| -> Result<(), Box<dyn Error>> {
        host.add_agent(format!("bot{id}"), PacedBot::default())?;
        Ok(())
    };
    for id in 0..bots {
        seat_bot(&mut host, id)?;
    }
    let human = TuiTransport {
        name: name.clone(),
        updates: update_tx.clone(),
        moves: move_rx,
    };
    host.add_agent(name.clone(), Remote::new(human))?;
    let mut filler = bots;
    while !host.is_ready() {
        seat_bot(&mut host, filler)?;
        filler += 1;
    }
    // Drop our copy of the sender so the UI sees the channel close when the game
    // thread (owning every transport's clone) finishes.
    drop(update_tx);

    // The host loop is blocking; run it off-thread so the UI owns the terminal.
    let game = thread::spawn(move || host.run());

    tui::run(name, update_rx, move_tx)?;

    // If the UI exited first, dropping `move_tx` unblocks the host's receive().
    let _ = game.join();
    Ok(())
}

fn prompt_name() -> io::Result<String> {
    print!("Enter your name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();
    Ok(if name.is_empty() {
        "you".to_string()
    } else {
        name
    })
}
