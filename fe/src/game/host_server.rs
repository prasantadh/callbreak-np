//! Hosting on a local network runs the api inside this process, so the app
//! carries its own server rather than depending on one already running.
//!
//! `api::serve_blocking` owns its runtime and never returns, so it gets a
//! thread of its own. The socket is bound here, before the thread starts, so
//! the port is known straight away and connections queue in the OS backlog
//! until the server begins accepting — there is no window where a room opened
//! by the host would be refused.
//!
//! One instance serves the whole process: the listener binds `0.0.0.0`, so the
//! same server answers the host's own loopback and every peer on the network.
//! Re-hosting reuses it rather than opening a second.

use super::net::{self, Server};
use std::net::TcpListener;
use std::sync::Mutex;
use std::thread;

/// The running server's address, or `None` until one is started. A `Mutex`
/// rather than `OnceLock` so the bind-and-spawn is done once even if two calls
/// race — the second waits, sees the address, and reuses it.
static RUNNING: Mutex<Option<Server>> = Mutex::new(None);

/// Start the embedded api if it is not already up, and return the address peers
/// on the network use to reach it. Idempotent: later calls return the same
/// address without starting a second server.
pub fn ensure_running() -> Result<Server, String> {
    let mut running = RUNNING.lock().unwrap();
    if let Some(server) = running.as_ref() {
        return Ok(server.clone());
    }

    // The address to hand out has to be one peers can reach, not loopback.
    let ip = net::local_ip()
        .ok_or_else(|| "This machine has no network address to host on.".to_string())?;

    // Port 0 lets the OS pick a free port, so hosting never collides with
    // another server on this machine. The host reads the full address off the
    // screen regardless, so the exact port does not need to be fixed.
    let listener = TcpListener::bind("0.0.0.0:0")
        .map_err(|error| format!("Could not open a server socket: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("Could not read the server's port: {error}"))?
        .port();

    thread::Builder::new()
        .name("callbreak-api".to_string())
        .spawn(move || {
            if let Err(error) = api::serve_blocking(listener) {
                eprintln!("the embedded api stopped: {error}");
            }
        })
        .map_err(|error| format!("Could not start the server thread: {error}"))?;

    let server = Server::parse(&format!("{ip}:{port}")).expect("host:port is a valid address");
    *running = Some(server.clone());
    Ok(server)
}
