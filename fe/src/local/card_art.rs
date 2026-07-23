//! Card faces drawn as inline SVG.
//!
//! No crate ships usable playing-card artwork — the closest, `cardito`, is a
//! CLI that lays out prototype cards from a template — and the public SVG decks
//! (Bellot's LGPL set, Knoll's CC0 set) would drag a licence into the repo for
//! 52 files. Drawing them is cheap enough: a card face is a corner index and a
//! pip layout, and both are fully specified by tradition.
//!
//! Pips follow the standard arrangement — columns left/centre/right, rows
//! spread evenly down the face — with everything below the midline rotated, as
//! on a real deck. Courts get a framed monogram rather than a portrait.

use callbreak::game::{Card, Rank, Suit};
use dioxus::prelude::*;

/// The face is drawn in a 100x140 box and scaled by CSS from there.
const W: f32 = 100.0;
const H: f32 = 140.0;

/// Pip columns.
const LEFT: f32 = 30.0;
const CENTRE: f32 = 50.0;
const RIGHT: f32 = 70.0;

/// Top and bottom of the pip field; rows are interpolated between them.
const TOP: f32 = 34.0;
const BOTTOM: f32 = 106.0;

fn row(fraction: f32) -> f32 {
    TOP + fraction * (BOTTOM - TOP)
}

pub fn suit_glyph(suit: Suit) -> &'static str {
    match suit {
        Suit::Spades => "♠",
        Suit::Hearts => "♥",
        Suit::Diamonds => "♦",
        Suit::Clubs => "♣",
    }
}

fn suit_colour(suit: Suit) -> &'static str {
    match suit {
        Suit::Hearts | Suit::Diamonds => "#c0392b",
        Suit::Spades | Suit::Clubs => "#1f1f1f",
    }
}

/// One pip: where it sits and whether it stands on its head.
#[derive(Clone, Copy)]
struct Pip {
    x: f32,
    y: f32,
    flipped: bool,
}

fn pip(x: f32, fraction: f32) -> Pip {
    Pip {
        x,
        y: row(fraction),
        // The bottom half of a real card is printed upside down.
        flipped: fraction > 0.5,
    }
}

/// The traditional pip arrangement for each rank. Courts and the ace are drawn
/// separately, so they return nothing here.
fn pips(rank: Rank) -> Vec<Pip> {
    let pair = |fraction: f32| vec![pip(LEFT, fraction), pip(RIGHT, fraction)];
    match rank {
        Rank::Two => vec![pip(CENTRE, 0.0), pip(CENTRE, 1.0)],
        Rank::Three => vec![pip(CENTRE, 0.0), pip(CENTRE, 0.5), pip(CENTRE, 1.0)],
        Rank::Four => [pair(0.0), pair(1.0)].concat(),
        Rank::Five => [pair(0.0), vec![pip(CENTRE, 0.5)], pair(1.0)].concat(),
        Rank::Six => [pair(0.0), pair(0.5), pair(1.0)].concat(),
        Rank::Seven => [pair(0.0), vec![pip(CENTRE, 0.25)], pair(0.5), pair(1.0)].concat(),
        Rank::Eight => [
            pair(0.0),
            vec![pip(CENTRE, 0.25)],
            pair(0.5),
            vec![pip(CENTRE, 0.75)],
            pair(1.0),
        ]
        .concat(),
        Rank::Nine => [
            pair(0.0),
            pair(1.0 / 3.0),
            vec![pip(CENTRE, 0.5)],
            pair(2.0 / 3.0),
            pair(1.0),
        ]
        .concat(),
        Rank::Ten => [
            pair(0.0),
            vec![pip(CENTRE, 1.0 / 6.0)],
            pair(1.0 / 3.0),
            pair(2.0 / 3.0),
            vec![pip(CENTRE, 5.0 / 6.0)],
            pair(1.0),
        ]
        .concat(),
        Rank::Ace | Rank::Jack | Rank::Queen | Rank::King => vec![],
    }
}

fn is_court(rank: Rank) -> bool {
    matches!(rank, Rank::Jack | Rank::Queen | Rank::King)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pip count is the whole point of a pip layout, and it is the one
    /// thing about the art that can be wrong rather than merely ugly.
    #[test]
    fn spot_cards_show_their_rank_in_pips() {
        let expected = [
            (Rank::Two, 2),
            (Rank::Three, 3),
            (Rank::Four, 4),
            (Rank::Five, 5),
            (Rank::Six, 6),
            (Rank::Seven, 7),
            (Rank::Eight, 8),
            (Rank::Nine, 9),
            (Rank::Ten, 10),
        ];
        for (rank, count) in expected {
            assert_eq!(pips(rank).len(), count, "{rank} should show {count} pips");
        }
    }

    /// The ace and the courts are drawn by hand, not from the pip grid.
    #[test]
    fn ace_and_courts_use_their_own_art() {
        for rank in [Rank::Ace, Rank::Jack, Rank::Queen, Rank::King] {
            assert!(pips(rank).is_empty(), "{rank} is drawn separately");
        }
    }

    /// `Rank::ALL` is crate-private on callbreak, so the deck is listed here.
    const RANKS: [Rank; 13] = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    #[test]
    fn pips_stay_on_the_card_and_flip_below_the_midline() {
        for rank in RANKS {
            for spot in pips(rank) {
                assert!(spot.x > 0.0 && spot.x < W, "{rank} pip escapes sideways");
                assert!(spot.y > 0.0 && spot.y < H, "{rank} pip escapes vertically");
                assert_eq!(
                    spot.flipped,
                    spot.y > row(0.5),
                    "{rank} pip is upside down on the wrong half"
                );
            }
        }
    }
}

#[component]
pub fn CardFace(card: Card, class: &'static str) -> Element {
    let suit = card.get_suit();
    let rank = card.get_rank();
    let colour = suit_colour(suit);
    let glyph = suit_glyph(suit);
    let label = rank.to_string();

    let spots = pips(rank);
    let court = is_court(rank);
    let ace = matches!(rank, Rank::Ace);

    rsx! {
        div { class: "card {class}",
            svg {
                class: "card-art",
                view_box: "0 0 {W} {H}",
                role: "img",

                rect {
                    x: "1", y: "1", width: "{W - 2.0}", height: "{H - 2.0}", rx: "8",
                    fill: "#fdfdfa", stroke: "#c8c8bd", stroke_width: "1.5",
                }

                // Corner index, and the same again rotated into the far corner.
                g { fill: "{colour}", font_family: "Georgia, 'Times New Roman', serif", font_weight: "700",
                    text { x: "12", y: "23", font_size: "23", text_anchor: "middle", "{label}" }
                    text { x: "12", y: "42", font_size: "19", text_anchor: "middle", "{glyph}" }
                    g { transform: "rotate(180 {W / 2.0} {H / 2.0})",
                        text { x: "12", y: "23", font_size: "23", text_anchor: "middle", "{label}" }
                        text { x: "12", y: "42", font_size: "19", text_anchor: "middle", "{glyph}" }
                    }
                }

                if ace {
                    text {
                        x: "{CENTRE}", y: "{row(0.5)}", font_size: "56",
                        text_anchor: "middle", dominant_baseline: "central", fill: "{colour}",
                        "{glyph}"
                    }
                }

                if court {
                    g {
                        rect {
                            x: "24", y: "32", width: "52", height: "76", rx: "5",
                            fill: "none", stroke: "{colour}", stroke_width: "1.5", opacity: "0.55",
                        }
                        rect {
                            x: "28", y: "36", width: "44", height: "68", rx: "3",
                            fill: "{colour}", opacity: "0.07",
                        }
                        text {
                            x: "{CENTRE}", y: "62", font_size: "34", text_anchor: "middle",
                            dominant_baseline: "central", fill: "{colour}",
                            font_family: "Georgia, 'Times New Roman', serif", font_weight: "700",
                            "{label}"
                        }
                        text {
                            x: "{CENTRE}", y: "88", font_size: "26", text_anchor: "middle",
                            dominant_baseline: "central", fill: "{colour}",
                            "{glyph}"
                        }
                    }
                }

                for (index, spot) in spots.into_iter().enumerate() {
                    text {
                        key: "{index}",
                        x: "{spot.x}",
                        y: "{spot.y}",
                        font_size: "24",
                        text_anchor: "middle",
                        dominant_baseline: "central",
                        fill: "{colour}",
                        transform: if spot.flipped { format!("rotate(180 {} {})", spot.x, spot.y) } else { String::new() },
                        "{glyph}"
                    }
                }
            }
        }
    }
}

/// A face-down card: a lattice back, so an opponent's fan reads as cards
/// rather than as coloured rectangles.
#[component]
pub fn CardBack(class: &'static str) -> Element {
    rsx! {
        div { class: "card back {class}",
            svg {
                class: "card-art",
                view_box: "0 0 {W} {H}",
                role: "img",

                rect {
                    x: "1", y: "1", width: "{W - 2.0}", height: "{H - 2.0}", rx: "8",
                    fill: "#20386e", stroke: "#101f42", stroke_width: "1.5",
                }
                rect {
                    x: "7", y: "7", width: "{W - 14.0}", height: "{H - 14.0}", rx: "5",
                    fill: "none", stroke: "#7d92c7", stroke_width: "1", opacity: "0.7",
                }
                // A diagonal lattice, clipped to the inner panel.
                g { stroke: "#5f77b4", stroke_width: "1.6", opacity: "0.75",
                    for step in 0..14 {
                        line {
                            key: "a{step}",
                            x1: "{7.0 + step as f32 * 14.0}", y1: "7",
                            x2: "7", y2: "{7.0 + step as f32 * 14.0}",
                        }
                    }
                    for step in 0..14 {
                        line {
                            key: "b{step}",
                            x1: "{W - 7.0 - step as f32 * 14.0}", y1: "7",
                            x2: "{W - 7.0}", y2: "{7.0 + step as f32 * 14.0}",
                        }
                    }
                }
            }
        }
    }
}
