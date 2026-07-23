//! The front menu: a small screen stack rather than routes, since every step
//! is a choice and none of them is worth a URL yet.
//!
//! ```text
//! Host ────┬─ Internet ──────┐
//!          └─ Local network ─┴─ pick bots ─ room number
//!
//! Join ────┬─ Internet ──────┬─ enter a room number
//!          │                 └─ join any room
//!          └─ Local network ─── enter a room number
//!
//! Just me ─── a local game against bots
//! ```
//!
//! Only "just me" is implemented. Everything networked renders its real
//! controls and then stops at an explicit "on hold" panel: the flow is worth
//! having laid out, but a room number that no server has reserved would be a
//! lie, so none is invented.

use dioxus::prelude::*;

/// How a game is carried. The two differ in what joining offers: only the
/// internet can match you with strangers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Network {
    Internet,
    Lan,
}

impl Network {
    fn label(self) -> &'static str {
        match self {
            Network::Internet => "internet",
            Network::Lan => "local network",
        }
    }
}

#[derive(Clone, PartialEq)]
enum Screen {
    Root,
    /// Choosing which network to host on.
    Host,
    /// Choosing which network to join over.
    Join,
    /// A game against bots, in this process.
    JustMe,
    /// Seats and bots, before the room exists.
    HostSetup(Network),
    Room {
        network: Network,
        bots: usize,
    },
    /// How to find a room, once the network is chosen.
    JoinWhere(Network),
    /// Typing in a room number.
    Code(Network),
    /// A leaf we cannot serve yet. Carries where "back" should return to.
    OnHold {
        title: String,
        detail: String,
        back: Box<Screen>,
    },
}

impl Screen {
    /// Where "back" goes. `None` is the root.
    fn parent(&self) -> Option<Screen> {
        match self {
            Screen::Root => None,
            Screen::Host | Screen::Join | Screen::JustMe => Some(Screen::Root),
            Screen::HostSetup(_) => Some(Screen::Host),
            Screen::Room { network, .. } => Some(Screen::HostSetup(*network)),
            Screen::JoinWhere(_) => Some(Screen::Join),
            Screen::Code(network) => Some(Screen::JoinWhere(*network)),
            Screen::OnHold { back, .. } => Some((**back).clone()),
        }
    }
}

/// A leaf for something the api does not do yet.
fn on_hold(title: &str, detail: &str, back: Screen) -> Screen {
    Screen::OnHold {
        title: title.to_string(),
        detail: detail.to_string(),
        back: Box::new(back),
    }
}

#[component]
pub fn Menu() -> Element {
    let mut screen = use_signal(|| Screen::Root);
    let current = screen();
    let show_back = current.parent().is_some();

    let body = match current {
        Screen::Root => rsx! {
            h1 { "Callbreak" }
            div { class: "options",
                button { class: "option", onclick: move |_| screen.set(Screen::Host), "Host" }
                button { class: "option", onclick: move |_| screen.set(Screen::Join), "Join" }
                button { class: "option", onclick: move |_| screen.set(Screen::JustMe), "Just me" }
            }
        },

        Screen::Host => rsx! {
            h1 { "Host" }
            div { class: "options",
                button { class: "option", onclick: move |_| screen.set(Screen::HostSetup(Network::Internet)), "Over the internet" }
                button { class: "option", onclick: move |_| screen.set(Screen::HostSetup(Network::Lan)), "On the local network" }
            }
        },

        Screen::Join => rsx! {
            h1 { "Join" }
            div { class: "options",
                button { class: "option", onclick: move |_| screen.set(Screen::JoinWhere(Network::Internet)), "Over the internet" }
                button { class: "option", onclick: move |_| screen.set(Screen::JoinWhere(Network::Lan)), "On the local network" }
            }
        },

        // The engine runs in this process, which needs a thread wasm has not got.
        #[cfg(not(target_arch = "wasm32"))]
        Screen::JustMe => rsx! {
            crate::local::LocalPlay {}
        },
        #[cfg(target_arch = "wasm32")]
        Screen::JustMe => rsx! {
            h1 { "Just me" }
            p { class: "muted",
                "A solo game hosts the engine in this process, which needs a thread the browser cannot give it. Use the desktop build."
            }
        },

        Screen::HostSetup(network) => rsx! {
            h1 { "Host · {network.label()}" }
            HostSettings {
                on_create: move |bots| screen.set(Screen::Room { network, bots }),
            }
        },

        Screen::Room { network, bots } => rsx! {
            h1 { "Your room" }
            p { "Hosting over the {network.label()} with {bots} bot(s). You are seated; the rest of the table is open." }
            div { class: "room-code",
                span { class: "room-code-label", "Room number" }
                span { class: "room-code-value", "— — — —" }
            }
            p { class: "muted",
                "Room numbers are issued by the api, which is not wired up yet, so there is nothing to share here."
            }
        },

        Screen::JoinWhere(Network::Internet) => rsx! {
            h1 { "Join · internet" }
            div { class: "options",
                button { class: "option", onclick: move |_| screen.set(Screen::Code(Network::Internet)), "Enter a room number" }
                button {
                    class: "option",
                    onclick: move |_| screen.set(on_hold(
                        "Join any room",
                        "Matchmaking needs the api. Nothing to match against yet.",
                        Screen::JoinWhere(Network::Internet),
                    )),
                    "Join any room"
                }
            }
        },

        Screen::JoinWhere(Network::Lan) => rsx! {
            h1 { "Join · local network" }
            p { class: "muted", "On a local network there is nobody to match you, so you need the host's room number." }
            div { class: "options",
                button { class: "option", onclick: move |_| screen.set(Screen::Code(Network::Lan)), "Enter a room number" }
            }
        },

        Screen::Code(network) => rsx! {
            h1 { "Enter a room number" }
            CodeEntry {
                on_join: move |code: String| screen.set(on_hold(
                    &format!("Joining room {code}"),
                    "Rooms live on the api, which is not wired up yet.",
                    Screen::Code(network),
                )),
            }
        },

        Screen::OnHold { title, detail, .. } => rsx! {
            h1 { "{title}" }
            p { class: "muted", "{detail}" }
        },
    };

    rsx! {
        div { class: "menu",
            if show_back {
                button {
                    class: "back",
                    onclick: move |_| {
                        if let Some(previous) = screen().parent() {
                            screen.set(previous);
                        }
                    },
                    "\u{2039} Back"
                }
            }
            {body}
        }
    }
}

/// Host settings: how many seats the bots take before anyone joins.
#[component]
fn HostSettings(on_create: EventHandler<usize>) -> Element {
    let mut bots = use_signal(|| 0usize);
    let chosen = bots();
    let choices: Vec<(usize, &'static str)> = (0..=3)
        .map(|count| {
            (
                count,
                if count == chosen {
                    "choice selected"
                } else {
                    "choice"
                },
            )
        })
        .collect();

    rsx! {
        div { class: "field",
            span { class: "field-label", "Bots at the table" }
            div { class: "choices",
                for (count, class) in choices {
                    button {
                        key: "{count}",
                        class: "{class}",
                        onclick: move |_| bots.set(count),
                        "{count}"
                    }
                }
            }
        }
        p { class: "muted", "You take a seat yourself; bots fill {chosen} of the other three, and players take the rest." }
        button { class: "primary", onclick: move |_| on_create.call(chosen), "Create room" }
    }
}

#[component]
fn CodeEntry(on_join: EventHandler<String>) -> Element {
    let mut code = use_signal(String::new);
    let submit = move || {
        let entered = code().trim().to_string();
        if !entered.is_empty() {
            on_join.call(entered);
        }
    };

    rsx! {
        input {
            class: "lobby-name",
            value: "{code}",
            placeholder: "Room number",
            autofocus: true,
            oninput: move |event| code.set(event.value()),
            onkeydown: move |event| {
                if event.key() == Key::Enter {
                    submit();
                }
            },
        }
        button { class: "primary", onclick: move |_| submit(), "Join" }
    }
}
