use callbreak::{Host, agent::Bot};
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::State,
    routing::{get, post},
};

#[derive(Clone)]
pub struct AppState {
    host: Arc<Mutex<Host>>,
}

async fn new(State(state): State<AppState>) -> &'static str {
    // let mut host = state.host.get_mut().unwrap();
    /*
        let host = state.game.get_mut().unwrap();
        match host {
            None => {
                *host = Some(Host::new());
            }
            Some(v) => {
                // check if the game is over
                // if yes init
                // if no reject
                if v.is_over() {
                } else {
                    // return error with host is in progress
                }
            }
        }
    */

    "Hello world!"
}

#[tokio::main]
async fn main() {
    let state = AppState {
        host: Arc::new(Mutex::new(Host::new())),
    };
    let app = Router::new()
        .route("/new", post(new))
        .route("/status", get(new))
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
