//! The front menu: a small screen stack rather than routes, since every step
//! is a choice and none of them is worth a URL yet.
//!
//! ```text
//! Host ────┬─ Internet ────── name + bots ─ room number ─ take your seat
//!          └─ Local network ─ name + bots ─ room number + this machine's
//!                                           address ─ take your seat
//!
//! Join ────┬─ Internet ──────┬─ name + room number ─ take your seat
//!          │                 └─ join any room (on hold)
//!          └─ Local network ─ name + host address + room number
//!
//! Just me ─── name ─ a game against bots, hosted in this process
//! ```
//!
//! The difference between the two networks is only which api is on the other
//! end. Internet play uses the address compiled into the build; local-network
//! play uses the api on the host's own machine, which binds `0.0.0.0` and so is
//! already reachable from the LAN — the host reads their address off the room
//! screen and everyone else types it in.

use crate::game::{net, GameTable, Server, Source};
use dioxus::prelude::*;

/// Which api is on the other end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Network {
    Internet,
    Lan,
}

#[derive(Clone, PartialEq)]
enum Screen {
    Root,
    /// Choosing which network to host on.
    Host,
    /// Choosing which network to join over.
    Join,
    /// Naming yourself before a game against bots.
    JustMe,
    /// Your name and how many seats the bots take.
    HostSetup(Network),
    /// The room the api opened, waiting for you to sit down.
    Room {
        network: Network,
        bots: usize,
        name: String,
    },
    /// How to find a room over the internet.
    JoinWhere,
    /// Your name, the room number, and — on a LAN — the host's address.
    Code(Network),
    /// At the table.
    Play {
        name: String,
        source: Source,
    },
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
            Screen::JoinWhere => Some(Screen::Join),
            // Only the internet path offers a choice worth going back to.
            Screen::Code(Network::Internet) => Some(Screen::JoinWhere),
            Screen::Code(Network::Lan) => Some(Screen::Join),
            // Leaving a table is leaving the game, so it goes all the way back.
            Screen::Play { .. } => Some(Screen::Root),
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
                button { class: "option", onclick: move |_| screen.set(Screen::JoinWhere), "Over the internet" }
                button { class: "option", onclick: move |_| screen.set(Screen::Code(Network::Lan)), "On the local network" }
            }
        },

        // The engine runs in this process, which needs a thread wasm has not got.
        #[cfg(not(target_arch = "wasm32"))]
        Screen::JustMe => rsx! {
            h1 { "Just me" }
            p { class: "lobby-blurb", "Five rounds against three bots, on this machine." }
            NameEntry {
                action: "Start",
                on_submit: move |name| screen.set(Screen::Play { name, source: Source::Local }),
            }
        },
        #[cfg(target_arch = "wasm32")]
        Screen::JustMe => rsx! {
            h1 { "Just me" }
            p { class: "muted",
                "A solo game hosts the engine in this process, which needs a thread the browser cannot give it. Use the desktop build, or host a room over the internet."
            }
        },

        Screen::HostSetup(network) => rsx! {
            h1 {
                if network == Network::Internet { "Host · internet" } else { "Host · local network" }
            }
            HostSettings {
                on_create: move |(name, bots)| screen.set(Screen::Room { network, bots, name }),
            }
        },
        Screen::Room {
            network,
            bots,
            name,
        } => rsx! {
            h1 { "Your room" }
            HostRoom {
                lan: network == Network::Lan,
                bots,
                on_seat: move |(name, server, room)| screen.set(Screen::Play {
                    name,
                    source: Source::Net { server, room },
                }),
                name,
            }
        },

        Screen::JoinWhere => rsx! {
            h1 { "Join · internet" }
            div { class: "options",
                button { class: "option", onclick: move |_| screen.set(Screen::Code(Network::Internet)), "Enter a room number" }
                button {
                    class: "option",
                    onclick: move |_| screen.set(on_hold(
                        "Join any room",
                        "Matchmaking needs the api to keep a list of open rooms, which it does not do yet. A room number works now.",
                        Screen::JoinWhere,
                    )),
                    "Join any room"
                }
            }
        },
        Screen::Code(network) => rsx! {
            h1 {
                if network == Network::Internet { "Join · internet" } else { "Join · local network" }
            }
            JoinForm {
                // Over the internet the server is known; on a LAN the host has
                // to tell you where they are.
                ask_for_address: network == Network::Lan,
                on_join: move |(name, server, room)| screen.set(Screen::Play {
                    name,
                    source: Source::Net { server, room },
                }),
            }
        },

        Screen::Play { name, source } => rsx! {
            GameTable { name, source }
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
                    "‹ Back"
                }
            }
            {body}
        }
    }
}

/// Who you are, for any game that needs to know. There is no default: a name
/// is how a seat is identified, so nothing starts until one is typed.
#[component]
fn NameEntry(action: &'static str, on_submit: EventHandler<String>) -> Element {
    let mut name = use_signal(String::new);
    let entered = name().trim().to_string();
    let ready = !entered.is_empty();
    let submit = move || {
        let entered = name().trim().to_string();
        if !entered.is_empty() {
            on_submit.call(entered);
        }
    };

    rsx! {
        input {
            class: "lobby-name",
            value: "{name}",
            placeholder: "Your name",
            autofocus: true,
            oninput: move |event| name.set(event.value()),
            onkeydown: move |event| {
                if event.key() == Key::Enter {
                    submit();
                }
            },
        }
        button {
            class: "primary",
            disabled: !ready,
            onclick: move |_| submit(),
            "{action}"
        }
    }
}

/// Host settings: who you are, and how many seats the bots take before anyone
/// joins. The api seeds the room with them; the rest stay open for humans.
#[component]
fn HostSettings(on_create: EventHandler<(String, usize)>) -> Element {
    let mut name = use_signal(String::new);
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
    let open = 3 - chosen;
    let entered = name().trim().to_string();
    let ready = !entered.is_empty();

    rsx! {
        input {
            class: "lobby-name",
            value: "{name}",
            placeholder: "Your name",
            autofocus: true,
            oninput: move |event| name.set(event.value()),
        }
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
        p { class: "muted",
            if open == 0 {
                "You and three bots: the game starts the moment you sit down."
            } else {
                "You, {chosen} bot(s), and {open} seat(s) for other players. The game starts once the table is full."
            }
        }
        button {
            class: "primary",
            disabled: !ready,
            onclick: move |_| {
                let entered = name().trim().to_string();
                if !entered.is_empty() {
                    on_create.call((entered, chosen));
                }
            },
            "Open the room"
        }
    }
}

/// Opens a room and shows its number, which is the thing to pass on to whoever
/// is playing. On a local network it first starts this machine's own server —
/// so the app carries the api rather than depending on one already running —
/// and also shows the address peers reach it at. Sitting down is a separate
/// step so the number and address can be read and shared first.
#[component]
fn HostRoom(
    lan: bool,
    name: String,
    bots: usize,
    on_seat: EventHandler<(String, Server, usize)>,
) -> Element {
    // Holds the server too, since a LAN host learns its own address only once
    // the embedded api has bound a port.
    let mut room = use_signal(|| None::<Result<(Server, usize), String>>);

    use_future(move || async move {
        // Internet play talks to the compiled-in api; a LAN host runs its own
        // in this process, then opens the room on it.
        let server = if lan {
            match crate::game::start_lan_host() {
                Ok(server) => server,
                Err(error) => return room.set(Some(Err(error))),
            }
        } else {
            Server::internet()
        };
        let opened = match net::create_room(&server, bots).await {
            Ok(number) => Ok((server, number)),
            Err(error) => Err(error),
        };
        room.set(Some(opened));
    });

    match room() {
        None => rsx! {
            p { class: "muted", "Opening a room…" }
        },
        Some(Err(error)) => rsx! {
            p { class: "muted", "{error}" }
        },
        Some(Ok((server, number))) => rsx! {
            div { class: "room-code",
                span { class: "room-code-label", "Room number" }
                span { class: "room-code-value open", "{number}" }
            }
            if lan {
                div { class: "room-code",
                    span { class: "room-code-label", "This machine" }
                    span { class: "room-code-address", "{server.address()}" }
                }
                p { class: "muted", "Players on this network need both: the address to reach you, and the room number." }
            } else if bots == 3 {
                p { class: "muted", "The table is full once you sit down." }
            } else {
                p { class: "muted", "Share the room number with whoever is playing. The game starts once the table is full." }
            }
            button {
                class: "primary",
                onclick: move |_| on_seat.call((name.clone(), server.clone(), number)),
                "Take your seat"
            }
        },
    }
}

/// Your name, the room number, and — joining over a local network — the address
/// the host read off their screen.
#[component]
fn JoinForm(ask_for_address: bool, on_join: EventHandler<(String, Server, usize)>) -> Element {
    let mut name = use_signal(String::new);
    let mut address = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut error = use_signal(String::new);

    let ready = !name().trim().is_empty();

    let mut submit = move || {
        let who = name().trim().to_string();
        if who.is_empty() {
            return;
        }

        let server = if ask_for_address {
            match Server::parse(&address()) {
                Ok(server) => server,
                Err(problem) => {
                    error.set(problem);
                    return;
                }
            }
        } else {
            Server::internet()
        };

        let Ok(room) = code().trim().parse::<usize>() else {
            error.set("A room number is digits only — check what the host sent you.".to_string());
            return;
        };

        on_join.call((who, server, room));
    };

    rsx! {
        input {
            class: "lobby-name",
            value: "{name}",
            placeholder: "Your name",
            oninput: move |event| name.set(event.value()),
        }
        if ask_for_address {
            input {
                class: "lobby-name",
                value: "{address}",
                placeholder: "Host address, e.g. 192.168.1.5:3000",
                autofocus: true,
                oninput: move |event| {
                    error.set(String::new());
                    address.set(event.value());
                },
            }
        }
        input {
            class: "lobby-name",
            value: "{code}",
            placeholder: "Room number",
            autofocus: !ask_for_address,
            oninput: move |event| {
                error.set(String::new());
                code.set(event.value());
            },
            onkeydown: move |event| {
                if event.key() == Key::Enter {
                    submit();
                }
            },
        }
        if !error().is_empty() {
            p { class: "muted", "{error}" }
        }
        button {
            class: "primary",
            disabled: !ready,
            onclick: move |_| submit(),
            "Take your seat"
        }
    }
}
