//! `cli-fe`: a terminal client for callbreak.
//!
//! Modes (`--mode`):
//! - `host`: play a whole game locally against bots, driven straight through
//!   `callbreak::Host` — no API, no network.
//! - `wifi`: LAN play for when the internet is down. Not implemented yet.
//! - `net`:  the usual internet game where anyone can join. Not implemented yet.
//!
//! A later `replay` path will feed a dumped game into the same UI, so [`tui`]
//! renders from a stream of snapshots and stays ignorant of their source.

mod game_view;
mod host_mode;
mod tui;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    Host,
    Wifi,
    Net,
}

#[derive(Debug, Parser)]
#[command(name = "cli-fe", about = "Terminal client for callbreak")]
struct Args {
    /// How to run: local `host`, LAN `wifi`, or internet `net`.
    #[arg(long, value_enum, default_value_t = Mode::Net)]
    mode: Mode,

    /// Bots to seat before humans join (0-3).
    #[arg(long, default_value_t = 3)]
    bots: usize,

    /// The human player's name. Prompted for if omitted in host mode.
    #[arg(long)]
    name: Option<String>,
}

fn main() {
    let args = Args::parse();
    let result = match args.mode {
        Mode::Host => host_mode::run(args.bots, args.name),
        // wifi is left for later; it needs peer discovery on the LAN.
        Mode::Wifi => {
            eprintln!("wifi mode (LAN play) is not implemented yet");
            Ok(())
        }
        // net will drive the game over the api's websocket; not wired up yet.
        Mode::Net => {
            eprintln!("net mode is not implemented yet; try `--mode host` for a local game");
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
