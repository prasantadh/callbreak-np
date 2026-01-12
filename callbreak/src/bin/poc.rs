use std::net::TcpListener;

use callbreak::{BotAgent, Host, Net};
use tracing_subscriber::{EnvFilter, fmt};
use tungstenite::accept;
fn main() {
    fmt()
        .with_env_filter(
            // TODO: currently running on debug level tracing.
            // offer different levels of tracing via env configuration
            EnvFilter::from_default_env().add_directive("callbreak=trace".parse().unwrap()),
        )
        .init();
    let mut host = Host::new();
    for id in 0..4 {
        let agent = Box::new(BotAgent);
        host.add_agent(id.to_string(), agent)
            .expect("must be able to add 4 players");
    }

    // // add the fourth agent as a net agent
    // let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    // let stream = server
    //     .incoming()
    //     .next()
    //     .expect("must wait until a connection arrives")
    //     .expect("must handover to the websocket correctly");
    // let socket = accept(stream).expect("FIXME: uhh not sure what the error could be");
    // host.add_agent("3".to_string(), Box::new(Net::new(socket)))
    //     .expect("must be able to add the fourth player");
    //
    host.run();
}
