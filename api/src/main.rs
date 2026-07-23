use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use callbreak::{
    Host,
    agent::{AgentKind, Bot, ClientMessage, Human, Transport},
    game::Call,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tracing_subscriber::{EnvFilter, fmt};

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

        // writer
        tokio::spawn(async move {
            while let Some(msg) = rx_out.recv().await {
                let _ = tx.send(Message::text(msg)).await;
            }
        });

        // reader
        tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = rx.next().await {
                let _ = tx_in.send(text.to_string());
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

    fn receive(&mut self) -> callbreak::agent::ClientMessage {
        match self.rx.blocking_recv() {
            Some(v) => match serde_json::from_str::<ClientMessage>(&v) {
                Ok(v) => v,
                _ => ClientMessage::Call(Call::new(1).unwrap()),
            },
            _ => ClientMessage::Call(Call::new(1).unwrap()),
        }
    }
}

/// Query for `POST /new`, e.g. `?bots=2`. Seeds the game with 0–3 bots; the
/// rest of the table fills with humans over `/join`.
#[derive(Debug, Deserialize)]
struct NewParams {
    bots: usize,
}

// FIXME: at some point when games are over, I will need to dump the game somewhere
// the dumping should likely be done by host.
async fn new(State(state): State<AppState>, Query(params): Query<NewParams>) -> impl IntoResponse {
    if params.bots > 3 {
        let body = Json(json!({"error": "bots must be between 0 and 3"}));
        return (StatusCode::BAD_REQUEST, body).into_response();
    }

    let mut hosts = state.hosts.lock().unwrap();
    let id = loop {
        let id = rand::random_range(0..=1000);
        if !hosts.contains_key(&id) {
            break id;
        }
    };

    let mut host = Host::new();
    for i in 0..params.bots {
        let _ = host.add_agent(format!("bot{i}"), AgentKind::Bot(Bot));
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

        let human = AgentKind::Human(Human::new(Box::new(AxumTransport::new(socket))));

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
                host.run();
            });
        }
    })
}

#[tokio::main]
async fn main() {
    fmt()
        .with_env_filter(
            // TODO: currently running on debug level tracing.
            // offer different levels of tracing via env configuration
            // EnvFilter::from_default_env().add_directive("callbreak=trace".parse().unwrap()),
            EnvFilter::from_default_env(),
        )
        .init();
    let state = AppState {
        hosts: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new()
        .route("/new", post(new))
        .route("/join/{room}", get(join))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
