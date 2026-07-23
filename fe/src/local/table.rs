//! The table as Dioxus components: four seats around a felt, the trick in the
//! centre, opponents behind face-down fans, the viewer's hand along the bottom
//! with legal cards raised and clickable, and the scoreboard alongside.
//!
//! This is `cli-fe`'s `tui` module rebuilt for a window. The layout is the same
//! — seats are ordered from the viewer, so you always sit at the bottom — but
//! moves are made by clicking rather than by cursor keys, so the call is picked
//! straight off a row of numbers instead of being nudged up and down.
//!
//! Rendering is driven by a stream of `Update`s, which keeps this module
//! ignorant of where a game comes from: a later networked mode can feed the
//! same components from the api's websocket.

use super::card_art::{CardBack, CardFace};
use super::session;
use super::view::{TableState, Update};
use callbreak::agent::{Action, ClientMessage};
use callbreak::game::{Call, Card};
use dioxus::prelude::*;
use futures::StreamExt;
use std::sync::mpsc::Sender;

/// Seat positions on screen, indexed by offset from the viewer.
const POSITIONS: [&str; 4] = ["bottom", "left", "top", "right"];

/// Everything the table draws, folded together from the engine's updates.
#[derive(Clone)]
struct GameState {
    me: String,
    table: Option<TableState>,
    /// The viewer's hand and legal moves, from the last prompt.
    hand: Vec<Card>,
    playable: Vec<Card>,
    action: Action,
    /// True while the current prompt is unanswered.
    awaiting: bool,
    /// The call being dialled in, until it is committed.
    call_value: u8,
    over: bool,
    status: String,
}

impl GameState {
    fn new(me: String) -> Self {
        Self {
            me,
            table: None,
            hand: vec![],
            playable: vec![],
            action: Action::Call,
            awaiting: false,
            call_value: 1,
            over: false,
            status: "Dealing…".to_string(),
        }
    }

    /// 1-based round, or the first round before anything has been dealt.
    fn round_number(&self) -> usize {
        self.table.as_ref().map_or(1, |table| table.round)
    }

    fn apply(&mut self, update: Update) {
        match update {
            // Someone else acted: refresh the shared table, leave the prompt be.
            Update::Table(table) => self.table = Some(table),
            Update::Prompt {
                table,
                hand,
                action,
                playable,
            } => {
                self.table = Some(table);
                self.hand = hand;
                self.playable = playable;
                self.action = action;
                self.awaiting = true;
                // No instructions: the sheet asks for the call, and the legal
                // cards lift out of the hand on their own.
                if matches!(action, Action::Call) {
                    // Each round is dialled from scratch.
                    self.call_value = 1;
                }
                self.status = match action {
                    Action::Call => String::new(),
                    Action::Break => "Your turn.".to_string(),
                };
            }
        }
    }

    /// The engine's channel closed: the match is done (or the thread died).
    fn finish(&mut self) {
        self.awaiting = false;
        self.over = true;
        self.hand.clear();
        self.playable.clear();
        self.status = match &self.table {
            Some(table) => {
                let totals = table.totals();
                let best = totals
                    .iter()
                    .cloned()
                    .fold(f32::MIN, f32::max);
                let winners: Vec<&str> = table
                    .seats
                    .iter()
                    .zip(totals)
                    .filter(|(_, total)| *total == best)
                    .map(|(seat, _)| seat.name.as_str())
                    .collect();
                format!("Game over — {} wins with {best:.1}", winners.join(" and "))
            }
            None => "Game over.".to_string(),
        };
    }

    fn fail(&mut self, reason: &str) {
        self.over = true;
        self.awaiting = false;
        self.status = format!("Could not start the game: {reason}");
    }
}

/// Local play: ask who is sitting down, then hand over to the table.
#[component]
pub fn LocalPlay() -> Element {
    let mut player = use_signal(|| None::<String>);

    match player() {
        None => rsx! {
            Lobby { on_start: move |name| player.set(Some(name)) }
        },
        // Keyed on the name so "play again" with a different name remounts the
        // component, which is what starts a fresh engine thread.
        Some(name) => rsx! {
            LocalGame { key: "{name}", name }
        },
    }
}

#[component]
fn Lobby(on_start: EventHandler<String>) -> Element {
    let mut name = use_signal(|| "you".to_string());
    let start = move || {
        let entered = name().trim().to_string();
        on_start.call(if entered.is_empty() {
            "you".to_string()
        } else {
            entered
        });
    };

    rsx! {
        div { class: "lobby",
            h1 { "Callbreak" }
            p { class: "lobby-blurb", "Five rounds against three bots, on this machine." }
            input {
                class: "lobby-name",
                value: "{name}",
                autofocus: true,
                oninput: move |event| name.set(event.value()),
                onkeydown: move |event| {
                    if event.key() == Key::Enter {
                        start();
                    }
                },
            }
            button { class: "primary", onclick: move |_| start(), "Take a seat" }
        }
    }
}

#[component]
fn LocalGame(name: String) -> Element {
    let mut state = use_signal(|| GameState::new(name.clone()));
    // Held for the click handlers; `None` until the engine thread is up.
    let moves = use_signal(|| None::<Sender<ClientMessage>>);

    // Starts the engine once on mount and then folds its updates into `state`
    // until the channel closes, which is how we learn the match ended.
    use_future(move || {
        let name = name.clone();
        let mut moves = moves;
        async move {
            match session::start(name) {
                Ok(session) => {
                    moves.set(Some(session.moves));
                    let mut updates = session.updates;
                    while let Some(update) = updates.next().await {
                        state.write().apply(update);
                    }
                    state.write().finish();
                }
                Err(error) => state.write().fail(&error.to_string()),
            }
        }
    });

    // The scoreboard is a reference, not something to watch, so it stays out of
    // the way until asked for.
    let mut show_scores = use_signal(|| false);

    let snapshot = state.read().clone();
    let calling = snapshot.awaiting && matches!(snapshot.action, Action::Call);
    let breaking = snapshot.awaiting && matches!(snapshot.action, Action::Break);

    // Two different sheets share the same slot. Calling forces one open with
    // just the standings and this round's calls; the Scores button asks for the
    // full round-by-round breakdown and wins if both apply.
    let scores_open = show_scores();
    let sheet_open = calling || scores_open;
    let sheet = snapshot
        .table
        .as_ref()
        .filter(|_| sheet_open && !scores_open)
        .map(sheet_lines)
        .unwrap_or_default();

    let pending_call = snapshot.call_value;

    // The round number lives on the felt, so the heading says only who you are.
    let title = format!("You are {}", snapshot.me);

    let seats = seat_views(&snapshot);
    let played: Vec<PlayView> = seats
        .iter()
        .filter_map(|seat| {
            seat.card.map(|card| PlayView {
                position: seat.position,
                card,
                won: seat.won,
                order: seat.order,
            })
        })
        .collect();

    let hand: Vec<HandView> = snapshot
        .hand
        .iter()
        .map(|card| HandView {
            card: *card,
            class: if breaking && snapshot.playable.contains(card) {
                "playable"
            } else {
                "idle"
            },
        })
        .collect();

    let won_tricks: Vec<Vec<WonCard>> = snapshot
        .table
        .as_ref()
        .map(|table| {
            table
                .my_won
                .iter()
                .map(|trick| {
                    trick
                        .iter()
                        .enumerate()
                        .filter_map(|(seat, card)| {
                            card.map(|card| WonCard {
                                card,
                                class: if seat == table.me { "mine" } else { "theirs" },
                            })
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();

    rsx! {
        div { class: "game",
            header { class: "game-head",
                h2 { "{title}" }
                button {
                    class: "ghost",
                    onclick: move |_| {
                        let open = show_scores();
                        show_scores.set(!open);
                    },
                    "Scores"
                }
            }

            div { class: "felt",
                // The round is a fact about the table, so it lives on the table.
                div { class: "round-badge",
                    span { class: "round-label", "Round" }
                    span { class: "round-value", "{snapshot.round_number()}/5" }
                }

                for seat in seats {
                    div { key: "{seat.position}", class: "seat seat-{seat.position}",
                        div { class: "seat-name {seat.name_class}",
                            span { "{seat.name}" }
                            span { class: "seat-progress", "{seat.progress}" }
                        }
                        if seat.show_fan {
                            div { class: "fan",
                                for back in 0..seat.fan_size {
                                    CardBack { key: "{back}", class: "fan-card" }
                                }
                            }
                        }
                    }
                }

                div { class: "centre",
                    for play in played {
                        div {
                            key: "{play.position}",
                            class: "play play-{play.position}",
                            // Later plays sit on top, as they would on a table.
                            style: "z-index: {play.order}",
                            if play.won {
                                div { class: "crown", "★" }
                            }
                            CardFace { card: play.card, class: "played" }
                        }
                    }
                }

                // Anything that surfaces covers the table but never the hand:
                // a call cannot be made without seeing the cards it is about.
                if sheet_open {
                    div {
                        class: "felt-sheet",
                        onclick: move |_| show_scores.set(false),
                        div {
                            class: "sheet",
                            onclick: move |event| event.stop_propagation(),

                            if scores_open {
                                Scoreboard { table: snapshot.table.clone() }
                                button { class: "ghost", onclick: move |_| show_scores.set(false), "Close" }
                            } else {
                                // A label column, so the two number rows say
                                // what they are without a caption above them.
                                div { class: "sheet-row sheet-names",
                                    span { class: "sheet-label" }
                                    for (index, cell) in sheet.names.into_iter().enumerate() {
                                        span { key: "{index}", class: "{cell.class}", "{cell.text}" }
                                    }
                                }
                                div { class: "sheet-row sheet-totals",
                                    span { class: "sheet-label", "Total" }
                                    for (index, cell) in sheet.totals.into_iter().enumerate() {
                                        span { key: "{index}", class: "{cell.class}", "{cell.text}" }
                                    }
                                }
                                div { class: "sheet-row sheet-calls",
                                    span { class: "sheet-label", "Round {snapshot.round_number()}" }
                                    for (index, cell) in sheet.calls.into_iter().enumerate() {
                                        span { key: "{index}", class: "{cell.class}", "{cell.text}" }
                                    }
                                }
                                // A value you nudge, then commit. Both ends are
                                // clamped to what `Call::new` accepts, so an
                                // illegal call can never be submitted.
                                div { class: "call-stepper",
                                    button {
                                        class: "step",
                                        disabled: pending_call <= 1,
                                        onclick: move |_| state.write().call_value = pending_call.saturating_sub(1).max(1),
                                        "−"
                                    }
                                    span { class: "call-value", "{pending_call}" }
                                    button {
                                        class: "step",
                                        disabled: pending_call >= 13,
                                        onclick: move |_| state.write().call_value = (pending_call + 1).min(13),
                                        "+"
                                    }
                                    button {
                                        class: "primary",
                                        onclick: move |_| submit_call(state, moves, pending_call),
                                        "Call"
                                    }
                                }
                            }
                        }
                    }
                }

                // Your hand belongs on the table, not below it.
                div { class: "my-hand",
                    if hand.is_empty() {
                        p { class: "muted", "No cards in hand." }
                    } else {
                        div { class: "hand",
                            for slot in hand {
                                div {
                                    key: "{slot.card}",
                                    class: "hand-slot {slot.class}",
                                    onclick: move |_| submit_play(state, moves, slot.card),
                                    CardFace { card: slot.card, class: "in-hand" }
                                }
                            }
                        }
                    }
                }
            }

            section { class: "won-area",
                div { class: "won-tricks",
                    for (index, trick) in won_tricks.into_iter().enumerate() {
                        div { key: "{index}", class: "won-trick",
                            for card in trick {
                                CardFace { key: "{card.card}", card: card.card, class: card.class }
                            }
                        }
                    }
                }
            }

            footer { class: "status", "{snapshot.status}" }
        }
    }
}

/// The full per-round breakdown, shown only when asked for.
#[component]
fn Scoreboard(table: Option<TableState>) -> Element {
    let Some(table) = table else {
        return rsx! {
            div { class: "scoreboard" }
        };
    };

    let totals = table.totals();
    let headers: Vec<String> = table
        .seats
        .iter()
        .map(|seat| seat.name.chars().take(8).collect())
        .collect();

    let mut rows: Vec<ScoreRow> = (0..5)
        .map(|round| {
            let line = table.rounds.get(round);
            let live = line.is_some_and(|line| !line.complete);
            let cells = (0..4)
                .map(|seat| match line {
                    // Settled: the score, with a minus only when it is a real
                    // loss. Nothing gets a plus.
                    Some(line) if line.complete => format!("{:.1}", line.scores[seat]),
                    // Still being played: tricks taken against the call. A bid
                    // is not missed until the round ends, so no score is shown.
                    Some(line) => match line.calls[seat] {
                        Some(call) => format!("{}/{}", line.won[seat], call),
                        None => "\u{b7}".to_string(),
                    },
                    // Not dealt yet: blank, so nothing reads as a minus.
                    None => String::new(),
                })
                .collect();
            ScoreRow {
                label: format!("R{}", round + 1),
                class: if live { "current" } else { "" },
                cells,
            }
        })
        .collect();
    rows.push(ScoreRow {
        label: "Total".to_string(),
        class: "total-row",
        cells: totals.iter().map(|total| format!("{total:.1}")).collect(),
    });

    rsx! {
        div { class: "scoreboard",
            table { class: "scores",
                thead {
                    tr {
                        th { "" }
                        for (seat, name) in headers.into_iter().enumerate() {
                            th { key: "{seat}", class: if seat == table.me { "me" } else { "" }, "{name}" }
                        }
                    }
                }
                tbody {
                    for (index, row) in rows.into_iter().enumerate() {
                        tr { key: "{index}", class: "{row.class}",
                            th { "{row.label}" }
                            for (seat, cell) in row.cells.into_iter().enumerate() {
                                td { key: "{seat}", "{cell}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One seat as drawn: ordered from the viewer, so `bottom` is always you.
#[derive(Clone)]
struct SeatView {
    position: &'static str,
    name: String,
    name_class: &'static str,
    /// Tricks taken against the call this round, e.g. `2/3`. Empty until the
    /// seat has called.
    progress: String,
    /// Backs to actually draw; a full fan of 13 would swamp the felt.
    fan_size: usize,
    show_fan: bool,
    card: Option<Card>,
    won: bool,
    /// Position in the trick's play order, counted from whoever led.
    order: usize,
}

#[derive(Clone, Copy)]
struct PlayView {
    position: &'static str,
    card: Card,
    won: bool,
    order: usize,
}

#[derive(Clone, Copy)]
struct HandView {
    card: Card,
    class: &'static str,
}

#[derive(Clone, Copy)]
struct WonCard {
    card: Card,
    class: &'static str,
}

/// One scoreboard row: the round label, its four cells, and whether it is the
/// round being played.
struct ScoreRow {
    label: String,
    class: &'static str,
    cells: Vec<String>,
}

/// One cell of the sheet.
struct SheetCell {
    text: String,
    class: &'static str,
}

/// The sheet that covers the felt: three lines, one column per seat, ordered
/// from the viewer so the columns line up with where people are sitting.
#[derive(Default)]
struct SheetLines {
    names: Vec<SheetCell>,
    totals: Vec<SheetCell>,
    calls: Vec<SheetCell>,
}

fn sheet_lines(table: &TableState) -> SheetLines {
    let totals = table.totals();
    let calls = table.rounds.last().map(|line| line.calls);
    let mut lines = SheetLines::default();

    for offset in 0..4 {
        let index = (table.me + offset) % 4;
        let class = if offset == 0 { "me" } else { "" };
        lines.names.push(SheetCell {
            text: table.seats[index].name.clone(),
            class,
        });
        lines.totals.push(SheetCell {
            text: format!("{:.1}", totals[index]),
            class,
        });
        lines.calls.push(SheetCell {
            // A seat later in turn order has not committed yet.
            text: match calls.and_then(|calls| calls[index]) {
                Some(call) => call.to_string(),
                None => "…".to_string(),
            },
            class,
        });
    }
    lines
}

fn seat_views(state: &GameState) -> Vec<SeatView> {
    let Some(table) = &state.table else {
        return vec![];
    };
    let round = table.rounds.last();
    (0..4)
        .map(|offset| {
            let index = (table.me + offset) % 4;
            let seat = &table.seats[index];
            SeatView {
                position: POSITIONS[offset],
                name: seat.name.clone(),
                name_class: if offset == 0 { "me" } else { "" },
                // With the scoreboard hidden, each seat carries its own standing
                // in this round: tricks taken against what it called.
                progress: match round.and_then(|round| {
                    round.calls[index].map(|call| (round.won[index], call))
                }) {
                    Some((won, call)) => format!("{won}/{call}"),
                    None => String::new(),
                },
                fan_size: seat.hand_size.min(5),
                show_fan: offset != 0,
                card: table.trick[index],
                won: table.trick_winner == Some(index),
                order: (index + 4 - table.trick_starter) % 4,
            }
        })
        .collect()
}

/// Answer a call prompt. Illegal values are dropped rather than sent, since the
/// engine asserts on anything it did not offer.
fn submit_call(mut state: Signal<GameState>, moves: Signal<Option<Sender<ClientMessage>>>, value: u8) {
    if !state.read().awaiting {
        return;
    }
    let Ok(call) = Call::new(value) else {
        return;
    };
    if send(&moves, ClientMessage::Call(call)) {
        let mut state = state.write();
        state.awaiting = false;
        // The seat badge shows the call from here on, so it is not repeated.
        state.status = "Waiting for the other players…".to_string();
    }
}

/// Play a card. Only cards the engine listed as legal are sent; the click is
/// ignored otherwise, which is what keeps `Host::run`'s assertion satisfied.
fn submit_play(mut state: Signal<GameState>, moves: Signal<Option<Sender<ClientMessage>>>, card: Card) {
    {
        let state = state.read();
        if !state.awaiting
            || !matches!(state.action, Action::Break)
            || !state.playable.contains(&card)
        {
            return;
        }
    }
    if send(&moves, ClientMessage::Break(card)) {
        let mut state = state.write();
        state.awaiting = false;
        state.status = "Waiting for the other players…".to_string();
        // Show the play at once; the authoritative update follows.
        state.hand.retain(|held| *held != card);
        state.playable.clear();
        if let Some(table) = &mut state.table {
            let me = table.me;
            // A complete trick is only still on the felt while the next one is
            // empty, so being asked to play means we are leading it: sweep the
            // old cards away rather than dropping ours onto them.
            if table.trick.iter().all(Option::is_some) {
                table.trick = [None; 4];
                table.trick_starter = me;
            }
            table.trick[me] = Some(card);
            table.trick_winner = None;
        }
    }
}

fn send(moves: &Signal<Option<Sender<ClientMessage>>>, message: ClientMessage) -> bool {
    match moves.read().as_ref() {
        Some(sender) => sender.send(message).is_ok(),
        None => false,
    }
}
