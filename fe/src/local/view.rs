//! UI-facing state built from the `ServerMessage`s the host sends.
//!
//! Ported from `cli-fe`'s `game_view` module, which is a binary crate and so
//! cannot be depended on. The two copies should stay in step; the accessors
//! listed below would let both of them shrink.
//!
//! The view types on callbreak (`agent::Game`/`agent::Round`) expose their
//! fields, but `game::Trick` keeps its cards private and `game::Call` hides its
//! count. Both derive `Serialize`, so we recover them by round-tripping through
//! JSON. Accessors on callbreak — `Trick::cards()`, `Call::value()`, or public
//! fields — would let us drop these round-trips and the ported rules below.
//!
//! Two kinds of update reach the UI: a [`Update::Prompt`] when it is the
//! viewer's turn (carries their hand and legal moves), and a lighter
//! [`Update::Table`] on every other seat's turn so the centre, scores, and
//! trick counts refresh on each play.

use callbreak::agent::{Action, Game as GameView, ServerMessage};
use callbreak::game::{Call, Card, Suit, Trick};
use serde::Deserialize;

/// A player's seat as the UI needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    pub name: String,
    /// Cards still in this seat's hand (for the face-down stand-in).
    pub hand_size: usize,
}

/// The shared table: seats, the centre trick, and running score. Built from any
/// seat's view since none of these fields are private to a player.
#[derive(Debug, Clone, PartialEq)]
pub struct TableState {
    pub seats: [Seat; 4],
    /// Index into `seats` of the viewing player.
    pub me: usize,
    /// 1-based round number.
    pub round: usize,
    /// Cards to show in the centre, by seat: the in-progress trick, or the last
    /// completed one while the next trick is still empty.
    pub trick: [Option<Card>; 4],
    /// Seat that led `trick`. Play order runs from here, which is what decides
    /// how the cards overlap in the centre.
    pub trick_starter: usize,
    /// Seat that won `trick`, once it is complete.
    pub trick_winner: Option<usize>,
    /// All four cards of each trick the viewer has taken this round, by seat.
    pub my_won: Vec<[Option<Card>; 4]>,
    /// One entry per round that has been dealt, the last of which may still be
    /// in progress.
    pub rounds: Vec<RoundLine>,
}

/// A round as the scoreboard needs it. `score` is only meaningful once
/// `complete` is set: a bid is not missed until the round is over, so the
/// running value of an unfinished round would show as a loss it has not taken.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundLine {
    pub complete: bool,
    pub calls: [Option<u8>; 4],
    pub won: [u8; 4],
    pub scores: [f32; 4],
}

/// What the host is telling the UI.
#[derive(Debug, Clone)]
pub enum Update {
    /// Someone else acted; refresh the shared table only.
    Table(TableState),
    /// The viewer's turn; also carries their hand and legal moves.
    Prompt {
        table: TableState,
        hand: Vec<Card>,
        action: Action,
        playable: Vec<Card>,
    },
}

impl Update {
    pub fn table(message: &ServerMessage, my_name: &str) -> Update {
        Update::Table(TableState::from_view(&message.view, my_name))
    }

    pub fn prompt(message: &ServerMessage, my_name: &str) -> Update {
        let table = TableState::from_view(&message.view, my_name);
        let round = message.view.rounds.last().expect("a round is in progress");
        // Sorted so the fan reads as suits in a row; `Card`'s ordering groups by
        // suit and runs high to low within one.
        let mut hand = round.hand.clone();
        hand.sort();
        let playable = match message.action {
            Action::Break => match current_trick(round) {
                Some(trick) => trick.valid_moves(&hand),
                None => hand.clone(),
            },
            Action::Call => vec![],
        };
        Update::Prompt {
            table,
            hand,
            action: message.action,
            playable,
        }
    }
}

impl TableState {
    fn from_view(view: &GameView, my_name: &str) -> TableState {
        let me = view.players.iter().position(|p| p == my_name).unwrap_or(0);
        let round_idx = view.rounds.len().saturating_sub(1);
        let round = &view.rounds[round_idx];

        let tricks: Vec<TrickWire> = round.tricks.iter().map(TrickWire::from_trick).collect();
        let full_count = tricks.iter().filter(|t| t.is_full()).count();
        let in_progress = tricks.last().filter(|t| !t.is_full());

        // The centre shows the live trick, or the last completed one while the
        // next trick has yet to see a card.
        let shown = match in_progress {
            Some(t) if t.cards.iter().any(Option::is_some) => Some((t, false)),
            _ => tricks.iter().rev().find(|t| t.is_full()).map(|t| (t, true)),
        };
        let (trick, trick_starter, trick_winner) = match shown {
            Some((t, true)) => (t.cards, t.starter, t.winner().map(|(seat, _)| seat)),
            Some((t, false)) => (t.cards, t.starter, None),
            None => ([None; 4], 0, None),
        };

        let mut my_won = vec![];
        for trick in tricks.iter().filter(|t| t.is_full()) {
            if let Some((seat, _)) = trick.winner() {
                if seat == me {
                    my_won.push(trick.cards);
                }
            }
        }

        let rounds = per_round_lines(view);

        let hand_size: [usize; 4] = std::array::from_fn(|i| {
            let played_current = in_progress.is_some_and(|t| t.cards[i].is_some());
            13usize
                .saturating_sub(full_count)
                .saturating_sub(played_current as usize)
        });

        let seats = std::array::from_fn(|i| Seat {
            name: view.players.get(i).cloned().unwrap_or_default(),
            hand_size: hand_size[i],
        });

        TableState {
            seats,
            me,
            round: round_idx + 1,
            trick,
            trick_starter,
            trick_winner,
            my_won,
            rounds,
        }
    }

    /// Running total per seat. Only settled rounds count, so the total never
    /// shows a loss the current round might still avoid.
    pub fn totals(&self) -> [f32; 4] {
        let mut totals = [0f32; 4];
        for line in self.rounds.iter().filter(|line| line.complete) {
            for (total, score) in totals.iter_mut().zip(line.scores) {
                *total += score;
            }
        }
        totals
    }
}

/// The first legal card a bot would play from `view`, mirroring
/// `callbreak::agent::Bot::play`.
pub fn first_valid(view: &GameView) -> Card {
    let round = view.rounds.last().expect("a round is in progress");
    let trick = current_trick(round).expect("a trick is in progress on a break turn");
    *trick
        .valid_moves(&round.hand)
        .first()
        .expect("a break turn has a legal card")
}

/// Calls, tricks taken and score per seat for each round dealt so far. Tricks
/// accrue live; the score only means anything once all thirteen are played.
fn per_round_lines(view: &GameView) -> Vec<RoundLine> {
    let mut lines = vec![];
    for round in &view.rounds {
        let tricks: Vec<TrickWire> = round.tricks.iter().map(TrickWire::from_trick).collect();
        let mut won = [0u8; 4];
        let mut full = 0;
        for trick in tricks.iter().filter(|t| t.is_full()) {
            full += 1;
            if let Some((seat, _)) = trick.winner() {
                won[seat] += 1;
            }
        }
        let calls: [Option<u8>; 4] =
            std::array::from_fn(|seat| round.calls[seat].as_ref().map(call_value));
        let scores = std::array::from_fn(|seat| match calls[seat] {
            Some(call) => round_points(call, won[seat]),
            None => 0.0,
        });
        lines.push(RoundLine {
            complete: full == 13,
            calls,
            won,
            scores,
        });
    }
    lines
}

/// Standard callbreak scoring: make the bid for `+call` plus 0.1 per overtrick,
/// miss it for `-call`. Computed here because the engine does not track score.
fn round_points(call: u8, won: u8) -> f32 {
    if won >= call {
        call as f32 + 0.1 * (won - call) as f32
    } else {
        -(call as f32)
    }
}

fn current_trick(round: &callbreak::agent::Round) -> Option<TrickWire> {
    round
        .tricks
        .last()
        .map(TrickWire::from_trick)
        .filter(|t| !t.is_full())
}

/// One trick decoded off the wire, mirroring `game::Trick`'s serialized shape:
/// `{ "starter": <seat>, "cards": [Option<Card>; 4] }`.
#[derive(Debug, Clone, Deserialize)]
struct TrickWire {
    starter: usize,
    cards: [Option<Card>; 4],
}

impl TrickWire {
    fn from_trick(trick: &Trick) -> Self {
        // Round-trip because `Trick` exposes no accessors (see the module note).
        let json = serde_json::to_string(trick).expect("Trick must serialize");
        serde_json::from_str(&json).expect("Trick must decode into TrickWire")
    }

    fn is_full(&self) -> bool {
        self.cards.iter().all(Option::is_some)
    }

    fn winner(&self) -> Option<(usize, Card)> {
        let starter = self.cards[self.starter]?;
        let winning_suit = if self.cards.iter().flatten().any(|c| c.get_suit() == Suit::Spades) {
            Suit::Spades
        } else {
            starter.get_suit()
        };
        self.cards
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|c| (i, *c)))
            .filter(|(_, c)| c.get_suit() == winning_suit)
            .max_by_key(|(_, c)| c.get_rank())
    }

    /// Port of `game::Trick::valid_play_from`. Kept in sync by hand; exposing
    /// the engine's version would let us delete this.
    fn valid_moves(&self, hand: &[Card]) -> Vec<Card> {
        if self.is_full() {
            return vec![];
        }
        let starter = match self.cards[self.starter] {
            Some(card) => card,
            None => return hand.to_vec(),
        };
        let winner = self.winner().expect("a started trick has a winner").1;
        let (s, w) = (starter.get_suit(), winner.get_suit());
        let candidates: [Vec<Card>; 4] = if s == w {
            [
                hand.iter()
                    .filter(|c| c.get_suit() == s && c.get_rank() > winner.get_rank())
                    .cloned()
                    .collect(),
                hand.iter().filter(|c| c.get_suit() == s).cloned().collect(),
                // spades handled above when s == spades; harmless for other suits
                hand.iter()
                    .filter(|c| c.get_suit() == Suit::Spades)
                    .cloned()
                    .collect(),
                hand.to_vec(),
            ]
        } else {
            // implies the trick has been trumped: w == spades, s != spades
            [
                hand.iter().filter(|c| c.get_suit() == s).cloned().collect(),
                hand.iter()
                    .filter(|c| c.get_suit() == w && c.get_rank() > winner.get_rank())
                    .cloned()
                    .collect(),
                hand.to_vec(),
                vec![],
            ]
        };
        candidates
            .into_iter()
            .find(|v| !v.is_empty())
            .unwrap_or_default()
    }
}

/// Pull the count out of a `Call`. It serializes as a bare number; an accessor
/// on callbreak would make this unnecessary.
fn call_value(call: &Call) -> u8 {
    serde_json::to_string(call)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
