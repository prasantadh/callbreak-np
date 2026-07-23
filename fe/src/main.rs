use dioxus::prelude::*;
use futures::StreamExt;
use tokio_tungstenite_wasm::{connect, Message, WebSocketStream};

/// Hosting a game in-process needs a thread for the blocking engine loop, so
/// local play is native only. See `local`'s module note.
#[cfg(not(target_arch = "wasm32"))]
mod local;
mod menu;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/net")]
    Net {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    // Dioxus installs a DEBUG-level subscriber in debug builds, which fills the
    // window with callbreak's per-turn tracing (`Host::run` logs every view it
    // builds). Claiming the global default first keeps that to warnings up.
    let _ = dioxus::logger::init(dioxus::logger::tracing::Level::WARN);

    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        dioxus::LaunchBuilder::desktop()
            .with_cfg(
                Config::new().with_window(
                    WindowBuilder::new()
                        .with_title("Callbreak")
                        .with_always_on_top(false),
                ),
            )
            .launch(App);
    }
    #[cfg(not(feature = "desktop"))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS } document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

/// Home is the front menu; it walks down to a game from there.
#[component]
fn Home() -> Element {
    rsx! {
        menu::Menu {}
    }
}

/// Networked play against the api. Still a connection smoke test: it opens the
/// socket and shows whatever the server says.
#[component]
pub fn Net() -> Element {
    let mut message = use_signal(String::new);
    let mut ws = use_signal(|| None::<futures::stream::SplitSink<WebSocketStream, Message>>);
    // `use_future` runs once when the component mounts and is cancelled when it
    // unmounts, so the socket is not reopened on every re-render.
    use_future(move || async move {
        // FIXME: this url will have to change and loaded during compilation from ENV
        match connect("ws://127.0.0.1:9001").await {
            Ok(socket) => {
                message.set(String::from("success"));

                let (sender, mut receiver) = socket.split();
                ws.set(Some(sender));

                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(t)) => message.set(t.to_string()),
                        Ok(Message::Binary(t)) => {
                            message.set(format!("cannot parse the server message: {t:?}"))
                        }
                        Ok(Message::Close(frame)) => {
                            message.set(format!("connection closed: {frame:?}"))
                        }
                        Err(e) => message.set(format!("{:?}", e)),
                    }
                }
            }
            Err(v) => message.set(format!("{:?}", v)),
        };
    });
    rsx! {
        div {
            id: "hero",
            img { src: HEADER_SVG, id: "header" }
            div { id: "links",
                a { href: "https://dioxuslabs.com/learn/0.7/", "📚 Learn Dioxus: {message}" }
                a { href: "https://dioxuslabs.com/awesome", "🚀 Awesome Dioxus" }
                a { href: "https://github.com/dioxus-community/", "📡 Community Libraries" }
                a { href: "https://github.com/DioxusLabs/sdk", "⚙️ Dioxus Development Kit" }
                a { href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus", "💫 VSCode Extension" }
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}

/// Router layout. The menu does its own navigation, so there is no chrome here
/// competing with it for the top of the window; `/net` is reachable by URL.
#[component]
fn Navbar() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}
