//! Local play: a whole game hosted in this process against bots.
//!
//! Native only. `callbreak::Host::run` is a blocking loop and `Transport`'s
//! methods are synchronous, so the engine needs a thread of its own — which
//! wasm does not have. The web build offers networked play instead, once the
//! api is wired up.

mod card_art;
mod session;
mod table;
mod view;

pub use table::LocalPlay;
