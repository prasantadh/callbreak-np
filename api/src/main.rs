use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{get, post},
};
use callbreak::{
    Host,
    agent::{AgentKind, Bot, ClientMessage, Human, Transport},
    game::Call,
};
use futures::{SinkExt, StreamExt};
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

// FIXME: at some point when games are over, I will need to dump the game somewhere
// and release the id for a new game to start with the same id
async fn new(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut hosts = state.hosts.lock().unwrap();
    loop {
        println!("attempting");
        let id = rand::random_range(0..=1000);
        if hosts.contains_key(&id) {
            continue;
        }
        hosts.insert(id, Host::new());
        return Json(json!({"room": id}));
    }
}

async fn join(
    State(state): State<AppState>,
    Path(room): Path<usize>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    println!("a player has attempted join");
    // loop or wait if there is another player being added?
    ws.on_upgrade(move |socket| async move {
        let mut hosts = state.hosts.lock().unwrap();
        let Some(host) = hosts.get_mut(&room) else {
            return;
        };
        host.add_agent("bot1".to_string(), AgentKind::Bot(Bot))
            .unwrap();
        host.add_agent("bot2".to_string(), AgentKind::Bot(Bot))
            .unwrap();
        host.add_agent("bot3".to_string(), AgentKind::Bot(Bot))
            .unwrap();
        let human = Human::new(Box::new(AxumTransport::new(socket)));
        let _ = host.add_agent("ME".to_string(), AgentKind::Human(human));
        // FIXME: ^ is an error, should handle it
        let ready = host.is_ready();
        drop(hosts);
        if ready {
            let mut host = {
                let mut hosts = state.hosts.lock().unwrap();
                hosts.remove(&room)
            }
            .unwrap();
            tokio::task::spawn_blocking(move || {
                host.run();
            });
        }
    })
}

#[tokio::main]
async fn main() {
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
