//! Hooking your own strategy into a game.
//!
//!     cargo run --example custom_strategy
//!
//! A seat is anything that implements [`Strategy`]. There are three methods and
//! only two of them are required:
//!
//! - `call` and `play` are asked of you on your turn;
//! - `observe` is told to you after *every* move at *every* seat, so it is how
//!   you watch the rest of the table. It defaults to doing nothing.
//!
//! Three things are worth knowing before you write one:
//!
//! 1. **You cannot play an illegal card.** `play` is handed the `LegalMoves`
//!    and returns one of them, and a `LegalMove` can only come from that set.
//!    There is no validation step to get wrong and no rejected-move error to
//!    handle, because neither is expressible.
//! 2. **You always have something to return.** `moves` is never empty, so
//!    `moves.first()` is always there. `max_by_key` and `min_by_key` hand back
//!    a move rather than an `Option` for the same reason — a strategy built on
//!    them needs no fallback branch.
//! 3. **The view is written from the table's point of view, not yours.**
//!    `calls` is indexed by seat and [`Trick::winner`] names a seat, so you have
//!    to locate yourself in it — see `seat_of` below.
//!
//! What is left is judgement. A bad choice loses tricks; it cannot crash a
//! match or be turned away by the engine.

use callbreak::Host;
use callbreak::agent::strategy::{Random, Strategy};
use callbreak::agent::{Game as View, LegalMove, LegalMoves};
use callbreak::game::{Call, Card, Rank, Suit, Turn};

/// Bids its strong cards and always plays the highest card it is allowed to.
#[derive(Debug)]
struct HighRoller {
    /// The name this strategy is seated under, so it can find itself in the
    /// view. Pass the same string to `Host::add_agent`.
    name: String,
    tricks_won: usize,
}

impl Strategy for HighRoller {
    fn call(&mut self, view: &View) -> Call {
        let hand = view.rounds.last().map(|round| round.hand.as_slice());
        let honours = count(hand, |card| card.get_rank() >= Rank::King);
        let spades = count(hand, |card| card.get_suit() == Suit::Spades);
        // `Call::new` rejects anything outside 1..=13, so clamp rather than
        // hand it a value it will refuse.
        Call::new((honours + spades / 3).clamp(1, 13) as u8).expect("clamped into range")
    }

    fn play<'v>(&mut self, _view: &'v View, moves: &LegalMoves<'v>) -> LegalMove<'v> {
        // No `unwrap`, no fallback branch: the set is never empty.
        moves.max_by_key(|card| card.get_rank())
    }

    /// Called after every move at every seat — including the three this
    /// strategy is not playing. This is where a real strategy would build up
    /// whatever it wants to remember: cards already gone, who is void in what,
    /// how the other seats bid.
    fn observe(&mut self, view: &View) {
        let Some(seat) = seat_of(view, &self.name) else {
            return;
        };
        println!("{}", serde_json::to_string(view).unwrap());
        let won = tricks_won(view, seat);
        if won > self.tricks_won {
            self.tricks_won = won;
            println!("  {} takes a trick — {won} so far", self.name);
        }
    }
}

/// Cards in the hand matching `predicate`, or zero before a hand is dealt.
fn count(hand: Option<&[Card]>, predicate: impl Fn(&Card) -> bool) -> usize {
    hand.map_or(0, |cards| cards.iter().filter(|c| predicate(c)).count())
}

/// Find yourself in the view.
///
/// The seat order is `view.players`, so a strategy that knows the name it was
/// seated under can locate itself.
fn seat_of(view: &View, name: &str) -> Option<Turn> {
    view.players
        .iter()
        .position(|player| player == name)
        .map(Turn::new)
}

/// How many tricks `seat` has taken so far.
///
/// Only completed tricks count: [`Trick::winner`] reports a provisional leader
/// the moment the first card is down, so counting mid-trick would inflate this.
fn tricks_won(view: &View, seat: Turn) -> usize {
    view.rounds
        .iter()
        .flat_map(|round| round.tricks.iter())
        .filter(|trick| trick.is_over())
        .filter_map(|trick| trick.winner())
        .filter(|(winner, _)| *winner == seat)
        .count()
}

fn main() {
    let mut host = Host::default();

    // Your strategy takes one seat; `Random` fills the rest.
    // `add_agent` takes it by value, so anything you want to read afterwards
    // has to live outside the strategy — behind an `Arc<Mutex<_>>`, say.
    host.add_agent(
        "high".to_string(),
        HighRoller {
            name: "high".to_string(),
            tricks_won: 0,
        },
    )
    .expect("a fresh table has room");

    for id in 0..3 {
        host.add_agent(format!("bot{id}"), Random)
            .expect("a fresh table has room");
    }

    // Seats are shuffled once the table fills, then five rounds are played
    // through to the end.
    host.run().expect("a full table plays to completion");
    println!("\nmatch complete");
}
