use crate::game::{Call, Card, Trick};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// View of the game sent to each player
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub players: Vec<String>,
    pub rounds: Vec<Round>,
}

impl Game {
    /// The moves open to this seat, or `None` when there is nothing to play
    /// into — no round dealt, or no trick under way.
    ///
    /// This is the only source of [`LegalMove`], so it is the only way any
    /// card reaches the engine. A strategy cannot name a card it has not been
    /// offered, and [`Remote`](super::Remote) cannot pass one through from a
    /// client without looking it up here first.
    pub fn legal_moves(&self) -> Option<LegalMoves<'_>> {
        let round = self.rounds.last()?;
        let cards = round.tricks.last()?.valid_play_from(&round.hand);
        (!cards.is_empty()).then_some(LegalMoves {
            cards,
            _view: PhantomData,
        })
    }
}

/// A card checked against one specific position.
///
/// Only [`Game::legal_moves`] mints one, so holding a `LegalMove` *is* the
/// proof that the move is allowed — there is nothing left to validate. The
/// lifetime ties it to the view it came from, so a strategy cannot keep one on
/// `self` and play it again on a later turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalMove<'v> {
    card: Card,
    _view: PhantomData<&'v Game>,
}

impl<'v> LegalMove<'v> {
    fn new(card: Card) -> Self {
        Self {
            card,
            _view: PhantomData,
        }
    }

    pub fn card(self) -> Card {
        self.card
    }
}

/// Every move open to a seat, guaranteed to hold at least one.
///
/// That guarantee is what lets [`first`](Self::first) and the pickers below
/// hand back a [`LegalMove`] rather than an `Option`, which in turn is what
/// lets [`Strategy::play`](super::Strategy::play) promise an answer.
#[derive(Debug, Clone)]
pub struct LegalMoves<'v> {
    cards: Vec<Card>,
    _view: PhantomData<&'v Game>,
}

#[allow(clippy::len_without_is_empty)] // there is no empty state to ask about
impl<'v> LegalMoves<'v> {
    /// Never zero.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// The underlying cards, for a caller that only wants to show them.
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn iter(&self) -> impl Iterator<Item = LegalMove<'v>> + '_ {
        self.cards.iter().copied().map(LegalMove::new)
    }

    pub fn first(&self) -> LegalMove<'v> {
        LegalMove::new(self.cards[0])
    }

    /// The `i`th move, wrapping — so an index from anywhere is safe to use.
    pub fn nth(&self, i: usize) -> LegalMove<'v> {
        LegalMove::new(self.cards[i % self.cards.len()])
    }

    /// `None` when this seat may not play that card. This is the parse that
    /// turns an untrusted card off the wire into a move, or into nothing.
    pub fn find(&self, card: Card) -> Option<LegalMove<'v>> {
        self.cards.contains(&card).then(|| LegalMove::new(card))
    }

    /// Highest by `key`. Returns a move, not an `Option`, because the set is
    /// never empty — so a strategy built on it needs no fallback branch.
    pub fn max_by_key<K: Ord>(&self, mut key: impl FnMut(Card) -> K) -> LegalMove<'v> {
        LegalMove::new(
            *self
                .cards
                .iter()
                .max_by_key(|card| key(**card))
                .expect("a legal move set is never empty"),
        )
    }

    /// Lowest by `key`. Never empty, as [`max_by_key`](Self::max_by_key).
    pub fn min_by_key<K: Ord>(&self, mut key: impl FnMut(Card) -> K) -> LegalMove<'v> {
        LegalMove::new(
            *self
                .cards
                .iter()
                .min_by_key(|card| key(**card))
                .expect("a legal move set is never empty"),
        )
    }
}

/// View of a round sent to each player
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Round {
    pub calls: [Option<Call>; 4],
    pub hand: Vec<Card>,
    pub tricks: Vec<Trick>,
}

/// Action that is requested of a player
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Call,
    Break,
    Update,
}

/// Message sent to an agent: a prompt for [`Action::Call`] or [`Action::Break`],
/// an unsolicited refresh for [`Action::Update`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMessage {
    pub action: Action,
    pub view: Game,
}

/// Message expected from a human agent
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientMessage {
    Call(Call),
    Break(Card),
    //TODO: also allow clients to request for update?
    //this would be important for allowing users to rejoin
}
