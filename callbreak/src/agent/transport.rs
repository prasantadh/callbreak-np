use super::view::{ClientMessage, ServerMessage};
use std::fmt::Debug;

/// A channel to an external player (human or remote bot). `send` a prompt,
/// `receive` the reply.
///
/// Both halves report failure rather than inventing an answer: a `None` from
/// either means "this client has nothing usable to say", and the seat's
/// fallback strategy decides the move instead. Implementations must never
/// fabricate a `ClientMessage` to signal a dead channel — a synthesised reply
/// is indistinguishable from a real one, and the engine would act on it.
pub trait Transport: Send + Debug {
    fn send(&mut self, message: ServerMessage) -> Option<()>;
    fn receive(&mut self) -> Option<ClientMessage>;
}
