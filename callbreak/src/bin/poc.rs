use callbreak::{Bot, Host};
fn main() {
    let mut host = Host::new();
    for id in 0..4 {
        let agent = Box::new(Bot);
        host.add_agent(id.to_string(), agent)
            .expect("must be able to add 4 players");
    }
    host.run();
}
