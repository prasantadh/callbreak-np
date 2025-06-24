pub mod call;
pub mod callbreak;
pub mod error;
pub mod hand;
pub mod player;
pub mod round;
pub mod trick;
pub mod turn;

pub use call::Call;
pub use callbreak::Callbreak;
pub use deck::Card;
pub use error::{Error, Result};
pub use hand::Hand;
pub use player::{Player, PlayerID};
pub use round::Round;
pub use trick::Trick;
pub use turn::Turn;

#[cfg(test)]
mod tests {
    use deck::{Rank, Suit};
    use tracing_subscriber::EnvFilter;

    use super::*;
    use std::sync::OnceLock;

    static TRACING: OnceLock<()> = OnceLock::new();
    fn init_tracing() {
        TRACING.get_or_init(|| {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .without_time()
                .with_target(false)
                .init()
        });
    }

    #[test]
    fn can_add_a_player() {
        init_tracing();
        let mut game = Callbreak::new();
        game.add_player("1").unwrap();
    }

    #[test]
    fn cannot_add_same_player_twice() {
        let mut game = Callbreak::new();
        game.add_player("1").unwrap();
        let action = game.add_player("1");
        assert_eq!(action, Err(Error::PlayerAlreadyInGame));
    }

    #[test]
    fn cannot_add_more_than_four_players() {
        let mut game = Callbreak::new();
        game.add_player("1").unwrap();
        game.add_player("2").unwrap();
        game.add_player("3").unwrap();
        game.add_player("4").unwrap();
        let action = game.add_player("5");
        assert_eq!(action, Err(Error::GameHasMaxPossiblePlayers));
    }

    #[test]
    fn each_player_has_a_spade_card() {
        for _ in 1..100 {
            let mut game = Callbreak::new();
            let player1 = game.add_player("1").unwrap().clone();
            let _ = game.add_player("2").unwrap();
            let _ = game.add_player("3").unwrap();
            let _ = game.add_player("4").unwrap();
            let action = game.get_hand(&player1).unwrap();
            let mut has_spade = false;
            for card in action.get_cards().iter() {
                if card.get_suit() == Suit::Spades {
                    has_spade = true;
                }
            }
            assert!(has_spade)
        }
    }

    #[test]
    fn each_player_has_a_face_card() {
        for _ in 1..100 {
            let mut game = Callbreak::new();
            let player1 = game.add_player("1").unwrap().clone();
            let _ = game.add_player("2").unwrap();
            let _ = game.add_player("3").unwrap();
            let _ = game.add_player("4").unwrap();
            let action = game.get_hand(&player1).unwrap();
            let mut has_face = false;
            for card in action.get_cards().iter() {
                if card.get_rank() == Rank::Jack
                    || card.get_rank() == Rank::Queen
                    || card.get_rank() == Rank::King
                    || card.get_rank() == Rank::Ace
                {
                    has_face = true;
                }
            }
            assert!(has_face)
        }
    }

    #[test]
    fn a_player_can_call() {
        let mut game = Callbreak::new();
        let mut players = vec![];
        for i in 0..4 {
            let player = game.add_player(format!("{i}")).unwrap().clone();
            players.push(player);
        }
        for player in players {
            if game.get_turn(&player) == Turn::new(0) {
                game.call(&player, Call::new(1).unwrap()).unwrap();
            }
        }
    }

    #[test]
    fn a_player_cannot_call_out_of_turn() {
        let mut game = Callbreak::new();
        let mut players = vec![];
        for i in 0..4 {
            let player = game.add_player(format!("{i}")).unwrap().clone();
            players.push(player);
        }
        for player in players {
            if game.get_turn(&player) != Turn::new(0) {
                let action = game.call(&player, Call::new(1).unwrap());
                assert_eq!(action, Err(Error::PlayerCalledOutOfTurn));
            }
        }
    }

    #[test]
    fn a_player_can_play_first_card() {
        init_tracing();
        let mut game = Callbreak::new();
        let mut players = vec![];
        for i in 0..4 {
            let player = game.add_player(format!("{i}")).unwrap().clone();
            players.push(player);
        }
        for i in 0..4 {
            for player in players.iter() {
                if game.get_turn(player) == Turn::new(i) {
                    game.call(player, Call::new(3).unwrap()).unwrap();
                }
            }
        }
        for player in players.iter() {
            if game.get_turn(player) == game.current() {
                let hand = game.get_hand(player).unwrap();
                game.play(player, hand.get_cards()[0]).unwrap();
            }
        }
        println!("{}", serde_json::to_string(&game).unwrap());
    }

    #[test]
    fn a_player_cannot_play_out_of_turn() {
        todo!();
    }

    #[test]
    fn round_i_must_be_started_by_turn_i() {
        todo!();
    }

    #[test]
    fn game_can_be_played_to_completion() {
        let mut game = Callbreak::new();
        for i in 0..4 {
            game.add_player(format!("{i}")).unwrap();
        }
        // currently players are being shuffled
        let players = game.get_players().to_vec();
        println!("players: {:?}", game.get_players());
        println!("calls: {:?}", game.get_calls());

        // each player receives a hand
        for player in players.iter() {
            println!("{:?}", game.get_hand(player).unwrap());
        }

        for round in 0..5 {
            // each player calls three
            for _ in 0..4 {
                let current = game.current().unwrap();
                let player = players[current].clone();
                game.call(&player, Call::new(3).unwrap()).unwrap();
            }

            // for thirteen tricks, when it is your turn, play something
            for trick in 0..13 {
                for turn in 0..4 {
                    println!("Round {round:?} Trick {trick:?} => Turn {turn:?}");
                    let current = game.current().unwrap();
                    let player = players[current].clone();
                    println!("{player:?} attempting a move");
                    let valids = game.get_valid_play(&player).unwrap();
                    println!("Valids: {valids:?}");
                    game.play(&player, valids[0]).unwrap();
                    if !(trick == 12 && turn == 3) {
                        println!("Trick: {:?}", game.get_trick().unwrap());
                    }
                    println!();
                }
            }
        }
    }
}
