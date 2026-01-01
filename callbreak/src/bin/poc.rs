use callbreak::{Bot, Host};
use tracing_subscriber::{EnvFilter, fmt};
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
        let agent = Box::new(Bot);
        host.add_agent(id.to_string(), agent)
            .expect("must be able to add 4 players");
    }
    host.run();
}
