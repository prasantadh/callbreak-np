//! The callbreak api as a library, so it can run as its own binary (see
//! `main.rs`) or be embedded by a client that hosts its own games — the
//! desktop app spawns [`serve_blocking`] on a thread when hosting on a local
//! network, rather than depending on a separately-running server.

use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{get, post},
};
use callbreak::{
    Host,
    agent::{ClientMessage, Remote, Transport, strategy::Random},
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

#[derive(Clone)]
struct AppState {
    hosts: Arc<Mutex<HashMap<usize, Host>>>,
}

#[derive(Debug)]
struct AxumTransport {
    rx: mpsc::UnboundedReceiver<String>,
    tx: mpsc::UnboundedSender<String>,
}

impl AxumTransport {
    fn new(socket: WebSocket) -> Self {
        let (mut tx, mut rx) = socket.split();
        let (tx_out, mut rx_out) = mpsc::unbounded_channel::<String>();
        let (tx_in, rx_in) = mpsc::unbounded_channel::<String>();

        // writer: forward outbound messages, then close the socket once the game
        // ends — dropping the transport closes this channel, ending the loop.
        let mut writer = tokio::spawn(async move {
            while let Some(msg) = rx_out.recv().await {
                if tx.send(Message::text(msg)).await.is_err() {
                    return;
                }
            }
            // Game over: send the closing handshake so the client is notified.
            let _ = tx.send(Message::Close(None)).await;
        });

        // reader
        let mut reader = tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = rx.next().await {
                if tx_in.send(text.to_string()).is_err() {
                    return;
                }
            }
        });

        // When either half finishes, abort the other so no task lingers on a
        // half-open socket.
        tokio::spawn(async move {
            tokio::select! {
                _ = &mut writer => reader.abort(),
                _ = &mut reader => writer.abort(),
            }
        });

        Self {
            rx: rx_in,
            tx: tx_out,
        }
    }
}

impl Transport for AxumTransport {
    fn send(&mut self, message: callbreak::agent::ServerMessage) -> Option<()> {
        self.tx.send(serde_json::to_string(&message).unwrap()).ok()
    }

    fn receive(&mut self) -> Option<ClientMessage> {
        let text = self.rx.blocking_recv()?;
        serde_json::from_str::<ClientMessage>(&text).ok()
    }
}

/// How many seats a new room starts with filled. At least one must be left for
/// the human who opens the room, so this is 0–3.
///
/// Checked while deserialising, the way `Call` is: past this point an
/// out-of-range count is not something the handler can be holding, so there is
/// no test for it to forget.
#[derive(Debug)]
struct BotCount(usize);

impl<'de> Deserialize<'de> for BotCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bots = usize::deserialize(deserializer)?;
        if bots > 3 {
            return Err(serde::de::Error::custom(
                "bots must be between 0 and 3, leaving a seat for you",
            ));
        }
        Ok(BotCount(bots))
    }
}

/// Query for `POST /new`, e.g. `?bots=2`. Seeds the game with 0–3 bots; the
/// rest of the table fills with humans over `/join`.
#[derive(Debug, Deserialize)]
struct NewParams {
    bots: BotCount,
}

// FIXME: at some point when games are over, I will need to dump the game somewhere
// the dumping should likely be done by host.
async fn new(State(state): State<AppState>, Query(params): Query<NewParams>) -> impl IntoResponse {
    let mut hosts = state.hosts.lock().unwrap();
    let id = loop {
        let id = rand::random_range(0..=1000);
        if !hosts.contains_key(&id) {
            break id;
        }
    };

    let mut host = Host::default();
    for i in 0..params.bots.0 {
        let _ = host.add_agent(format!("bot{i}"), Random);
    }
    hosts.insert(id, host);

    Json(json!({"room": id})).into_response()
}

/// The client's first websocket frame, naming the joining human.
#[derive(Debug, Deserialize)]
struct JoinRequest {
    name: String,
}

// FIXME: at some point I need a timer for when the lobby times out
// and when each player times out.
async fn join(
    State(state): State<AppState>,
    Path(room): Path<usize>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        // Reject the connection unless the first frame is a valid handshake.
        let request: JoinRequest = match socket.recv().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str(text.as_str()) {
                Ok(request) => request,
                Err(e) => {
                    eprintln!("rejecting join to room {room}: bad handshake: {e}");
                    return;
                }
            },
            _ => {
                eprintln!("rejecting join to room {room}: no handshake received");
                return;
            }
        };

        let human = Remote::new(AxumTransport::new(socket));

        // Seat the player under the lock, then pull the host out to run once full.
        let host_to_run = {
            let mut hosts = state.hosts.lock().unwrap();
            let Some(host) = hosts.get_mut(&room) else {
                eprintln!(
                    "rejecting {} from room {room}: room unavailable",
                    request.name
                );
                return;
            };
            if let Err(e) = host.add_agent(request.name.clone(), human) {
                eprintln!("rejecting {} from room {room}: {e}", request.name);
                return;
            }
            if host.is_ready() {
                hosts.remove(&room)
            } else {
                None
            }
        };

        if let Some(mut host) = host_to_run {
            tokio::task::spawn_blocking(move || {
                if let Err(e) = host.run() {
                    eprintln!("room {room}: game ended with error: {e}");
                }
            });
        }
    })
}

/// The router with a fresh, empty lobby. Every server instance — the standalone
/// binary and each embedded one — owns its own rooms.
pub fn router() -> Router {
    let state = AppState {
        hosts: Arc::new(Mutex::new(HashMap::new())),
    };
    Router::new()
        .route("/new", post(new))
        .route("/join/{room}", get(join))
        .with_state(state)
}

/// Serve on an already-bound listener until the process ends. The caller owns
/// the runtime and decides what address to bind, so this works both for the
/// standalone binary and for a client embedding the api.
pub async fn serve(listener: tokio::net::TcpListener) {
    axum::serve(listener, router()).await.unwrap();
}

/// Build a runtime and serve on `listener`, blocking the calling thread until
/// the process ends. This is the entry point for a client that hosts its own
/// games: bind a `std` socket, then hand it to a thread running this.
pub fn serve_blocking(listener: std::net::TcpListener) -> std::io::Result<()> {
    listener.set_nonblocking(true)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::from_std(listener)?;
        serve(listener).await;
        Ok(())
    })
}
