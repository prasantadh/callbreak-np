use callbreak::Host;
use callbreak::agent::{AgentKind, Bot, ClientMessage, Human, ServerMessage, Transport};
use callbreak::game::Call;
use std::net::{TcpListener, TcpStream};
use tracing_subscriber::{EnvFilter, fmt};
use tungstenite::{Message, WebSocket, accept};

#[derive(Debug)]
struct PocTransport {
    transport: WebSocket<TcpStream>,
}
impl Transport for PocTransport {
    fn send(&mut self, message: ServerMessage) -> Option<()> {
        let message = serde_json::to_string(&message).expect("must serialize without issue");
        self.transport
            .send(tungstenite::Message::Text(message.into()))
            .ok()
    }

    fn receive(&mut self) -> ClientMessage {
        // deserialize the response into a ClientMessage then return to the server
        match self.transport.read() {
            Ok(Message::Text(text)) => match serde_json::from_str(text.as_str()) {
                Ok(message) => message,
                _ => ClientMessage::Call(Call::new(1).unwrap()),
            },
            _ => ClientMessage::Call(Call::new(1).unwrap()),
        }
    }
}

fn main() {
    fmt()
        .with_env_filter(
            // TODO: currently running on debug level tracing.
            // offer different levels of tracing via env configuration
            EnvFilter::from_default_env().add_directive("callbreak=trace".parse().unwrap()),
        )
        .init();
    let mut host = Host::new();
    for id in 0..3 {
        let agent = callbreak::agent::AgentKind::Bot(Bot);
        host.add_agent(id.to_string(), agent)
            .expect("must be able to add 4 players");
    }

    // add the fourth agent as a net agent
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    let stream = server
        .incoming()
        .next()
        .expect("must wait until a connection arrives")
        .expect("must handover to the websocket correctly");
    let socket = accept(stream).expect("FIXME: uhh not sure what the error could be");
    let transport = PocTransport { transport: socket };
    let human = Human::new(Box::new(transport));
    let agent = AgentKind::Human(human);
    host.add_agent("3".to_string(), agent)
        .expect("must be able to add the fourth player");

    host.run();
}
