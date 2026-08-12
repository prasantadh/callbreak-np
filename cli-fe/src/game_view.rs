//! UI-facing state built from the `ServerMessage`s the host sends. Reads
//! straight off the engine's `game::Trick`/`Call` (all public now) — no JSON
//! round-trips.
//!
//! Two kinds of update reach the UI: [`Update::Prompt`] on the viewer's turn
//! (with hand and legal moves) and a lighter [`Update::Table`] otherwise.

use callbreak::agent::{Action, Game as GameView, ServerMessage};
use callbreak::game::{Card, Trick};

/// A player's seat as the UI needs it.
#[derive(Debug, Clone)]
pub struct Seat {
    pub name: String,
    /// Cards still in this seat's hand (for the face-down stand-in).
    pub hand_size: usize,
}

/// The shared table: seats, the centre trick, and running score. Built from any
/// seat's view since none of these fields are private to a player.
#[derive(Debug, Clone)]
pub struct TableState {
    pub seats: [Seat; 4],
    /// Index into `seats` of the viewing player.
    pub me: usize,
    /// 1-based round number.
    pub round: usize,
    /// Cards to show in the centre, by seat: the in-progress trick, or the last
    /// completed one while the next trick is still empty.
    pub trick: [Option<Card>; 4],
    /// Seat that won `trick`, once it is complete.
    pub trick_winner: Option<usize>,
    /// All four cards of each trick the viewer has taken this round, by seat.
    pub my_won: Vec<[Option<Card>; 4]>,
    /// Per-round score for every seat, one entry per round that has finished.
    pub round_scores: Vec<[f32; 4]>,
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
    /// `None` when the viewer is not seated at this table. Rendering somebody
    /// else's seat as if it were theirs would be worse than rendering nothing:
    /// every label, card position and highlight would be wrong, silently.
    pub fn table(message: &ServerMessage, my_name: &str) -> Option<Update> {
        Some(Update::Table(TableState::from_view(
            &message.view,
            my_name,
        )?))
    }

    /// `None` when the viewer is not seated at this table, as [`Update::table`].
    pub fn prompt(message: &ServerMessage, my_name: &str) -> Option<Update> {
        let table = TableState::from_view(&message.view, my_name)?;
        let round = message.view.rounds.last().expect("a round is in progress");
        let hand = round.hand.clone();
        let playable = match message.action {
            Action::Break => round
                .tricks
                .last()
                .map(|trick| trick.valid_play_from(&hand))
                .unwrap_or_else(|| hand.clone()),
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
        let (trick, trick_winner) = match shown {
            Some((t, true)) => (*t.cards(), t.winner().map(|(turn, _)| usize::from(turn))),
            Some((t, false)) => (*t.cards(), None),
            None => ([None; 4], None),
        };

        let mut my_won = vec![];
        for trick in tricks.iter().filter(|t| is_full(t)) {
            if let Some((turn, _)) = trick.winner() {
                if usize::from(turn) == me {
                    my_won.push(*trick.cards());
                }
            }
        }

        let round_scores = per_round_scores(view);

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
            trick_winner,
            my_won,
            round_scores,
        })
    }
}

/// Callbreak score per seat for each round in play, updated live: the current
/// round's row reflects tricks won so far, so scores accrue in real time.
fn per_round_scores(view: &GameView) -> Vec<[f32; 4]> {
    let mut rounds = vec![];
    for round in &view.rounds {
        let mut won = [0u8; 4];
        for trick in round.tricks.iter().filter(|t| is_full(t)) {
            if let Some((turn, _)) = trick.winner() {
                won[usize::from(turn)] += 1;
            }
        }
        let mut scores = [0f32; 4];
        for seat in 0..4 {
            if let Some(call) = round.calls[seat] {
                scores[seat] = round_points(u8::from(call), won[seat]);
            }
        }
        rounds.push(scores);
    }
    rounds
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

fn is_full(trick: &Trick) -> bool {
    trick.cards().iter().all(Option::is_some)
}
