pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    // Call Errors
    CallValueTooLarge,
    CallValueTooSmall,
    // Hand Errors
    HandIsFull,
    HandHasCardAlready,
    HandDoesNotHaveThisCard,
    RequiresFaceCard,
    RequiresSpades,
    Not13Cards,
    HasDuplicateCards,
    // Round Errors
    RoundIsOver,
    // Trick Errors
    InvalidPlay,
    // Turn Errors
    NotYourTurn,
    // Game Errors,
    NotAcceptingNewPlayers,
    NotAcceptingCalls,
    NotAcceptingPlay,
    PlayerAlreadyInGame,
    PlayerNotInGame,
    RoundIsNotOver,
    RoundNotInProgress,
    // Agent Error
    AgentSend,
    AgentRecv,
    NotTheSolicitedResponse,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
