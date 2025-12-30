use core::Game;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::State,
    routing::{get, post},
};

#[derive(Debug, Clone)]
pub struct AppState {
    game: Arc<Mutex<Option<Game>>>,
}

async fn new(State(state): State<AppState>) -> &'static str {
    // should take {data: playerid}
    // return { players: [_, _, X, _],
    //          rounds: [
    //              {
    //                  hand: [c1, c2, ..., c13]},
    //                  calls: [Y, Y, _, _]],
    //                  tricks: [
    //                      [c1, c2, c3, c4],
    //                      ...
    //                      [c1, c2, c3, c4]
    //                  ]
    //              },
    println!("{:?}", state.game);
    let game = state.game.get_mut().unwrap();
    match game {
        None => {
            *game = Some(Game::new());
        }
        Some(v) => {
            // check if the game is over
            // if yes init
            // if no reject
            if v.over() {

            } else {
                return error with game is in progress
            }
        }
    }

    "Hello world!"
}

// TODO: before I implement call, I need a middleware
// that will deserialize a jwt token, fetch a game from storage
// then pass that on to the call endpoint
// with the player state, game state, and the call value.
// Will likely also need another middleware that will save the state back

#[tokio::main]
async fn main() {
    let state = AppState {
        game: Arc::new(Mutex::new(None)),
    };
    let app = Router::new()
        .route("/new", post(new))
        .route("/call", post(new))
        .route("/break", post(new))
        .route("/status", get(new))
        .route("/moves", get(new))
        .with_state(state);
    // can already start adding three players
    // the incoming user would be the fourth.
    // would have the first hand available immediately.
    // would call for
    let mut game = Game::new();
    game.add_player("bot1").unwrap();
    game.add_player("bot2").unwrap();
    game.add_player("bot3").unwrap();
    // need an endpoint for joining

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    println!("Hello, world!");
}
