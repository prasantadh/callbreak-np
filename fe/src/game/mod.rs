//! Playing a game, whatever is driving it.
//!
//! Both sources — the engine hosted in this process, and the api over a
//! websocket — are reduced to the same pair of channels, so [`table`] renders
//! from a stream of [`view::Update`]s without knowing where they came from:
//!
//! - updates arrive on a futures channel that the UI awaits from a Dioxus task;
//! - moves leave on a futures channel, whose receiving end is either the engine
//!   thread's blocking read or a task writing to the socket.

mod card_art;
pub mod net;
mod table;
mod view;

/// Hosting the engine in this process needs a thread for its blocking loop,
/// which wasm has not got — this covers both a solo game and the api a
/// local-network host embeds.
#[cfg(not(target_arch = "wasm32"))]
mod host_server;
#[cfg(not(target_arch = "wasm32"))]
mod local;

use callbreak::agent::ClientMessage;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use view::Update;

pub use net::Server;
pub use table::{GameTable, Source};

/// Start the api this machine will host a local-network game on, and return the
/// address to hand out. Native only: hosting runs the server in-process.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_lan_host() -> Result<Server, String> {
    host_server::ensure_running()
}

/// The browser cannot host, since it has no room for the server's thread.
#[cfg(target_arch = "wasm32")]
pub fn start_lan_host() -> Result<Server, String> {
    Err("Hosting a game needs the desktop build.".to_string())
}

/// The live ends of a game: updates to render, moves to submit.
pub struct Session {
    pub updates: UnboundedReceiver<Update>,
    pub moves: UnboundedSender<ClientMessage>,
}
