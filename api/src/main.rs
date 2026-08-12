//! The standalone api server. The request handling lives in the library (see
//! `lib.rs`) so the desktop client can embed the same server when it hosts a
//! local-network game; this binary is the deployment that runs on its own.

use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() {
    fmt()
        .with_env_filter(
            // TODO: currently running on debug level tracing.
            // offer different levels of tracing via env configuration
            // EnvFilter::from_default_env().add_directive("callbreak=trace".parse().unwrap()),
            EnvFilter::from_default_env(),
        )
        .init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    api::serve(listener).await;
}
