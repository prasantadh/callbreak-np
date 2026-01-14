use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{get, post},
};
use callbreak::{
    Host,
    agent::{Bot, Net},
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct AppState {
    hosts: HashMap<usize, Arc<Mutex<Host>>>,
}

// FIXME: at some point when games are over, I will need to dump the game somewhere
// and release the id for a new game to started with the same id
async fn new(State(mut state): State<AppState>) -> Json<serde_json::Value> {
    loop {
        let id = rand::random_range(0..=1000);
        if state.hosts.contains_key(&id) {
            continue;
        }
        state.hosts.insert(id, Arc::new(Mutex::new(Host::new())));
        return Json(json!({"room": id}));
    }
}

async fn join(
    State(mut state): State<AppState>,
    Path(room): Path<usize>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let host = state.hosts.get(&room).cloned();
        let Some(host) = host else { return };
        let host = host.lock().unwrap(); // FIXME: Do I need to do this in a
        // loop or wait if there is another player being added?
        host.add_agent("X".to_string(), Box::new(Net::new(socket)));
    })
}

#[tokio::main]
async fn main() {
    let state = AppState {
        hosts: HashMap::new(),
    };
    let app = Router::new()
        .route("/new", post(new))
        .route("/join/{room}", get(join))
        .with_state(state);

    let mut host = Host::new();
    host.add_agent(String::from("bot1"), Box::new(Bot))
        .expect("must be able to add a bot");
    host.add_agent(String::from("bot2"), Box::new(Bot))
        .expect("must be able to add second bot");
    host.add_agent(String::from("bot3"), Box::new(Bot))
        .expect("must be able to add third bot");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    println!("Hello, world!");
}
