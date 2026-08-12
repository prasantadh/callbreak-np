pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    // Call
    CallValueTooLarge,
    CallValueTooSmall,
    // Hand: the first four reject a deal, the last a card it does not hold
    HandHasNoFaceCard,
    HandHasNoSpade,
    HandIsNotThirteenCards,
    HandHasDuplicateCards,
    CardNotInHand,
    // Round
    RoundIsOver,
    CardNotPlayable,
    // Game and Round
    NotYourTurn,
    NotAcceptingCalls,
    NotAcceptingPlays,
    // Host
    TableNotFull,
    TableIsFull,
    NameAlreadyTaken,
    /// A broken engine invariant — a bug, not a recoverable condition. The
    /// `&'static str` is a breadcrumb for the report.
    Internal(&'static str),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
