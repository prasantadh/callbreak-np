//! UI-facing state built from the `ServerMessage`s the host sends.
//!
//! This is a fold, not a reimplementation: `Trick::cards`, `Trick::winner` and
//! `Trick::valid_play_from` are the engine's own, so the rules live in one
//! place and the legal moves offered here are by construction the ones
//! `Host::run` will accept. What remains is presentation — seat offsets, the
//! centre's overlap order, and scoring, which the engine does not track.
//!
//! Two kinds of update reach the UI: a [`Update::Prompt`] when it is the
//! viewer's turn (carries their hand and legal moves), and a lighter
//! [`Update::Table`] on every other seat's turn so the centre, scores, and
//! trick counts refresh on each play.

// Local play is compiled out on wasm, and it is the only caller of the
// spectator half of this module.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use callbreak::agent::{Action, Game as GameView, ServerMessage};
use callbreak::game::{Card, Trick};

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
    /// A seat that is not ours acted; only the shared table changes.
    ///
    /// `None` when the viewer is not seated at this table. Rendering somebody
    /// else's seat as if it were theirs would be worse than rendering nothing:
    /// every label, card position and highlight would be wrong, silently.
    pub fn table(view: &GameView, my_name: &str) -> Option<Update> {
        Some(Update::Table(TableState::from_view(view, my_name)?))
    }

    /// `None` when the viewer is not seated at this table, as [`Update::table`].
    pub fn prompt(message: &ServerMessage, my_name: &str) -> Option<Update> {
        let table = TableState::from_view(&message.view, my_name)?;
        let round = message.view.rounds.last().expect("a round is in progress");
        // Sorted so the fan reads as suits in a row; `Card`'s ordering groups by
        // suit and runs high to low within one.
        let mut hand = round.hand.clone();
        hand.sort();
        let playable = match message.action {
            Action::Break => match current_trick(round) {
                // The engine's own rule, so the cards we light up are exactly
                // the ones it will accept back.
                Some(trick) => trick.valid_play_from(&hand),
                None => hand.clone(),
            },
            // An update is not a prompt and never reaches here; callers route it
            // to `table` instead. Nothing is playable either way.
            Action::Call | Action::Update => vec![],
        };
        Some(Update::Prompt {
            table,
            hand,
            action: message.action,
            playable,
        })
    }
}

impl TableState {
    fn from_view(view: &GameView, my_name: &str) -> Option<TableState> {
        let me = view.players.iter().position(|p| p == my_name)?;
        let round_idx = view.rounds.len().saturating_sub(1);
        let round = &view.rounds[round_idx];

        let tricks = &round.tricks;
        let full_count = tricks.iter().filter(|t| is_full(t)).count();
        let in_progress = tricks.last().filter(|t| !is_full(t));

        // The centre shows the live trick, or the last completed one while the
        // next trick has yet to see a card.
        let shown = match in_progress {
            Some(t) if t.cards().iter().any(Option::is_some) => Some((t, false)),
            _ => tricks.iter().rev().find(|t| is_full(t)).map(|t| (t, true)),
        };
        let (trick, trick_starter, trick_winner) = match shown {
            // A winner only means anything once every seat has played.
            Some((t, complete)) => (
                *t.cards(),
                starter_of(t),
                complete.then(|| winner_seat(t)).flatten(),
            ),
            None => ([None; 4], 0, None),
        };

        let my_won = tricks
            .iter()
            .filter(|t| is_full(t))
            .filter(|t| winner_seat(t) == Some(me))
            .map(|t| *t.cards())
            .collect();

        let rounds = per_round_lines(view);

        let hand_size: [usize; 4] = std::array::from_fn(|i| {
            let played_current = in_progress.is_some_and(|t| t.cards()[i].is_some());
            13usize
                .saturating_sub(full_count)
                .saturating_sub(played_current as usize)
        });

        let seats = std::array::from_fn(|i| Seat {
            name: view.players.get(i).cloned().unwrap_or_default(),
            hand_size: hand_size[i],
        });

        Some(TableState {
            seats,
            me,
            round: round_idx + 1,
            trick,
            trick_starter,
            trick_winner,
            my_won,
            rounds,
        })
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

/// Calls, tricks taken and score per seat for each round dealt so far. Tricks
/// accrue live; the score only means anything once all thirteen are played.
fn per_round_lines(view: &GameView) -> Vec<RoundLine> {
    let mut lines = vec![];
    for round in &view.rounds {
        let mut won = [0u8; 4];
        let mut full = 0;
        for trick in round.tricks.iter().filter(|t| is_full(t)) {
            full += 1;
            if let Some(seat) = winner_seat(trick) {
                won[seat] += 1;
            }
        }
        let calls: [Option<u8>; 4] = std::array::from_fn(|seat| round.calls[seat].map(u8::from));
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

/// The trick still being played, if any. A round's last trick is complete only
/// at the very end of the round, so this is `None` just once per round.
fn current_trick(round: &callbreak::agent::Round) -> Option<&Trick> {
    round.tricks.last().filter(|t| !is_full(t))
}

fn is_full(trick: &Trick) -> bool {
    trick.cards().iter().all(Option::is_some)
}

/// Which seat is taking the trick. Meaningful part-way through, but only the
/// callers that check the trick is complete treat it as final.
fn winner_seat(trick: &Trick) -> Option<usize> {
    trick.winner().map(|(turn, _)| usize::from(turn))
}

/// Which seat led.
fn starter_of(trick: &Trick) -> usize {
    usize::from(trick.starter().0)
}
