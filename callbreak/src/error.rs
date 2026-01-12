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
    // Player Errors
    PlayerAlreadyCalled,
    TurnIsAlreadySet,
    // Round Errors
    AllCallsNeededBeforeAnyPlay,
    RoundIsOver,
    PlayerCalledOutOfTurn,
    PlayerPlayedOutOfTurn,
    InvalidRoundId,
    // Trick Errors
    TrickNotInitialized,
    InvalidPlay,
    TrickIsOver,
    NoTrickWinnerYet,
    // Turn Errors
    InvalidValueForTurn,
    NotYourTurn,
    // Game Errors,
    NotAcceptingNewPlayers,
    NotAcceptingCalls,
    NotAcceptingPlay,
    GameHasMaxPossiblePlayers,
    NotEnoughPlayers,
    PlayerAlreadyInGame,
    PlayerNotInGame,
    RoundIsNotOver,
    RoundNotInProgress,
    GameOver,
    // Miscellaneous
    ImpossibleCondition,
    // Agent Error
    AgentSend,
    AgentRecv,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
