//! Playing over the api.
//!
//! Two steps, matching the two routes the api exposes:
//!
//! - `POST /new?bots=N` opens a room and answers with its number;
//! - `GET  /join/{room}` upgrades to a websocket whose first frame must name
//!   the joining human. After that the socket carries `ServerMessage` inward
//!   and `ClientMessage` outward, both as JSON — the same types the engine
//!   uses locally, so [`super::view`] and [`super::table`] need no changes.
//!
//! One difference from local play is worth knowing: the api only talks to a
//! seat when that seat has to act, so updates arrive on your turn and not on
//! anyone else's. The table therefore holds still between your turns rather
//! than following each play the way it does in a local game.

use super::view::Update;
use super::Session;
use callbreak::agent::{Action, ClientMessage, ServerMessage};
use futures::channel::mpsc::unbounded;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::future::Future;
use tokio_tungstenite_wasm::{connect, Message};

/// The api this build talks to for internet play, baked in when it is built:
///
/// ```text
/// CALLBREAK_API=api.example.com:3000 dx build --release --platform macos
/// ```
///
/// Falls back to this machine, which is what a development build wants.
const INTERNET_API: &str = match option_env!("CALLBREAK_API") {
    Some(host) => host,
    None => THIS_MACHINE,
};

/// Where the api listens by default. It binds `0.0.0.0`, so one instance serves
/// both loopback and the local network.
const THIS_MACHINE: &str = "127.0.0.1:3000";
const DEFAULT_PORT: u16 = 3000;

/// Which api to talk to, as `host:port`.
///
/// Internet play uses the address compiled in above. Local-network play uses
/// the api running on the host's own machine: they hand out their address, and
/// everyone else types it in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Server(String);

impl Server {
    /// The api this build was pointed at.
    pub fn internet() -> Self {
        Server(INTERNET_API.to_string())
    }

    /// An address typed in by someone joining over the local network. A bare
    /// host gets the default port; a scheme, if pasted in, is dropped.
    ///
    /// Bracketed IPv6 would need more care than the `:` test below, which is
    /// fine while this is for handing `192.168.x.y` to someone in the room.
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input
            .trim()
            .trim_start_matches("http://")
            .trim_start_matches("ws://")
            .trim_end_matches('/');

        if trimmed.is_empty() {
            return Err("Enter the host's address, like 192.168.1.5:3000.".to_string());
        }
        if trimmed.contains(char::is_whitespace) {
            return Err("An address has no spaces in it.".to_string());
        }

        Ok(Server(if trimmed.contains(':') {
            trimmed.to_string()
        } else {
            format!("{trimmed}:{DEFAULT_PORT}")
        }))
    }

    /// For showing the host what to pass on.
    pub fn address(&self) -> &str {
        &self.0
    }
}

/// This machine's address on the local network, so a host can read it off the
/// screen instead of hunting for it. Just the ip — the port is whatever the
/// embedded server ends up bound to.
///
/// Connecting a UDP socket sends nothing — it only asks the routing table which
/// interface would carry the traffic, and that interface's address is the one
/// peers can reach. Returns `None` when there is no route out at all.
#[cfg(not(target_arch = "wasm32"))]
pub fn local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

/// The api's reply to `POST /new`.
#[derive(Debug, Deserialize)]
struct NewRoom {
    room: usize,
}

/// Open a room seeded with `bots` bots and return its number. The remaining
/// seats stay open for humans, so a room with three bots starts the moment the
/// host takes the fourth.
pub async fn create_room(server: &Server, bots: usize) -> Result<usize, String> {
    let url = format!("http://{}/new?bots={bots}", server.address());
    let response = reqwest::Client::new()
        .post(&url)
        .send()
        .await
        .map_err(|error| format!("could not reach the api at {}: {error}", server.address()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("the api refused to open a room: {status}"));
    }

    let room: NewRoom = response
        .json()
        .await
        .map_err(|error| format!("the api's answer made no sense: {error}"))?;
    Ok(room.room)
}

/// Take a seat in `room` as `name`.
///
/// Returns the session together with the future that works the socket. The
/// caller spawns it, which keeps this module free of any particular runtime —
/// the app hands it to Dioxus, the tests await it directly.
///
/// Fails if the api cannot be reached. A room that has gone, is full, or
/// already seats that name is refused by closing the socket without a word, so
/// that shows up later as a session that ends before the first prompt.
pub async fn join(
    server: &Server,
    name: String,
    room: usize,
) -> Result<(Session, impl Future<Output = ()>), String> {
    let socket = connect(format!("ws://{}/join/{room}", server.address()))
        .await
        .map_err(|error| format!("could not reach the api at {}: {error}", server.address()))?;
    let (mut sink, mut stream) = socket.split();

    // The api rejects the connection unless the first frame is this handshake.
    let handshake = serde_json::json!({ "name": name }).to_string();
    sink.send(Message::text(handshake))
        .await
        .map_err(|error| format!("the api dropped the handshake: {error}"))?;

    let (update_tx, update_rx) = unbounded::<Update>();
    let (move_tx, mut move_rx) = unbounded::<ClientMessage>();

    let outgoing = async move {
        while let Some(message) = move_rx.next().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if sink.send(Message::text(text)).await.is_err() {
                break;
            }
        }
    };

    let incoming = async move {
        while let Some(frame) = stream.next().await {
            let Ok(Message::Text(text)) = frame else {
                // A close frame, a binary frame, or a broken socket all mean
                // the same thing here: no more prompts are coming.
                break;
            };
            let Ok(message) = serde_json::from_str::<ServerMessage>(&text) else {
                continue;
            };
            // Somebody else moved: refresh the table, ask nothing. Replying to
            // one of these would desync this seat for the rest of the match, so
            // it must never become a prompt.
            let update = match message.action {
                Action::Update => Update::table(&message.view, &name),
                _ => Update::prompt(&message, &name),
            };
            // The server sent a table this player is not seated at; there is
            // nothing coherent to draw.
            let Some(update) = update else { continue };
            if update_tx.unbounded_send(update).is_err() {
                break;
            }
        }
    };

    let session = Session {
        updates: update_rx,
        moves: move_tx,
    };
    Ok((session, async move {
        futures::future::join(outgoing, incoming).await;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use callbreak::agent::Action;
    use callbreak::game::Call;
    use std::time::Duration;

    /// One call and thirteen plays, five times over.
    const PROMPTS_PER_MATCH: usize = 5 * 14;

    /// Drive a seated session to the end, answering every prompt with the first
    /// legal move, and return how many prompts it saw.
    ///
    /// It runs until the update stream ends, which happens when the api closes
    /// the socket at the end of the match — so reaching the expected count also
    /// confirms the server hangs up when the game is over.
    async fn play_out(session: Session, pump: impl std::future::Future<Output = ()>) -> usize {
        let mut updates = session.updates;
        let moves = session.moves;
        let play = async move {
            let mut prompts = 0;
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
                    .expect("the socket is still open");
            }
            prompts
        };

        let (_, prompts) =
            tokio::time::timeout(Duration::from_secs(120), futures::future::join(pump, play))
                .await
                .expect("the match finishes inside two minutes");
        prompts
    }

    /// Open a room, take a seat, and play the match out against a separately
    /// running api.
    ///
    /// Ignored by default because it needs `cargo run -p api` listening.
    /// `CALLBREAK_TEST_API=host:port` points it at a specific instance — set it
    /// to this machine's LAN address to exercise the local-network path.
    ///
    /// ```text
    /// cargo test -p fe --no-default-features --features desktop -- --ignored
    /// ```
    ///
    /// What it pins down is the contract with the server: the shape of the
    /// handshake, that `ServerMessage`/`ClientMessage` survive the round trip
    /// as JSON, and that the moves we offer are ones the engine accepts — the
    /// api asserts legality exactly as local play does.
    #[tokio::test]
    #[ignore = "needs a running api"]
    async fn plays_a_match_against_the_api() {
        // Three bots and us: the table is full the moment we sit down.
        let server = match std::env::var("CALLBREAK_TEST_API") {
            Ok(address) => Server::parse(&address).expect("a usable test address"),
            Err(_) => Server::parse(THIS_MACHINE).expect("loopback is a valid address"),
        };
        eprintln!("playing against {}", server.address());
        let room = create_room(&server, 3).await.expect("the api opens a room");
        let (session, pump) = join(&server, "tester".to_string(), room)
            .await
            .expect("the api seats us");

        assert_eq!(play_out(session, pump).await, PROMPTS_PER_MATCH);
    }

    /// Play a whole match against the api this process embeds — no separately
    /// running server. This is the local-network host's path end to end: start
    /// the in-process api, open a room on it, sit down, and finish.
    ///
    /// Ignored by default because it binds a real socket and asks the routing
    /// table for this machine's address, both of which want a live network.
    #[tokio::test]
    #[ignore = "binds a socket and needs a network route"]
    async fn hosts_a_match_on_the_embedded_api() {
        let server = crate::game::start_lan_host().expect("the embedded api starts");
        // Re-hosting reuses the one server rather than opening another.
        assert_eq!(
            crate::game::start_lan_host().unwrap().address(),
            server.address()
        );

        let room = create_room(&server, 3)
            .await
            .expect("the embedded api opens a room");
        let (session, pump) = join(&server, "host".to_string(), room)
            .await
            .expect("the embedded api seats us");

        assert_eq!(play_out(session, pump).await, PROMPTS_PER_MATCH);
    }
}

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn a_bare_host_gets_the_default_port() {
        assert_eq!(
            Server::parse("192.168.1.5").unwrap().address(),
            "192.168.1.5:3000"
        );
    }

    #[test]
    fn an_explicit_port_is_kept() {
        assert_eq!(
            Server::parse(" 192.168.1.5:9001 ").unwrap().address(),
            "192.168.1.5:9001"
        );
    }

    /// People paste what they see, so a scheme should not break the join.
    #[test]
    fn a_pasted_scheme_is_dropped() {
        assert_eq!(
            Server::parse("http://10.0.0.2:3000/").unwrap().address(),
            "10.0.0.2:3000"
        );
    }

    #[test]
    fn nonsense_is_refused() {
        assert!(Server::parse("   ").is_err());
        assert!(Server::parse("192.168.1.5 3000").is_err());
    }
}
