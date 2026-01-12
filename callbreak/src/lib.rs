pub mod agent;
mod error;
pub mod game;
mod host;

pub use agent::{Agent, PlayerView, RoundView};
pub use error::Error;
pub use host::Host;

use error::Result;
use game::Game;
