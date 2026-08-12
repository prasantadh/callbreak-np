use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Turn(u8);

const MAX_TURN: u8 = 3;

impl Turn {
    pub fn new(value: usize) -> Self {
        Turn((value % 4) as u8)
    }

    pub fn next(self) -> Self {
        let next = (self.0 + 1) % 4;
        Self(next)
    }
}

impl Display for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> Index<Turn> for Vec<T> {
    type Output = T;
    fn index(&self, turn: Turn) -> &Self::Output {
        self.index(usize::from(turn))
    }
}

impl<T> IndexMut<Turn> for Vec<T> {
    fn index_mut(&mut self, turn: Turn) -> &mut Self::Output {
        &mut self[usize::from(turn)]
    }
}

impl<T> Index<Turn> for [T] {
    type Output = T;
    fn index(&self, turn: Turn) -> &Self::Output {
        self.index(usize::from(turn))
    }
}

impl<T> IndexMut<Turn> for [T] {
    fn index_mut(&mut self, turn: Turn) -> &mut Self::Output {
        &mut self[usize::from(turn)]
    }
}

impl From<Turn> for usize {
    fn from(value: Turn) -> Self {
        // `u8 -> usize` widens losslessly, so this needs no cast.
        value.0.into()
    }
}

impl<'de> Deserialize<'de> for Turn {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let seat = u8::deserialize(deserializer)?;
        match seat {
            0..=MAX_TURN => Ok(Turn(seat)),
            _ => Err(serde::de::Error::custom(format!(
                "a seat must be 0..={MAX_TURN}, got {seat}"
            ))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn every_seat_round_trips() {
        for seat in 0..=MAX_TURN {
            let turn = Turn::new(seat.into());
            let json = serde_json::to_string(&turn).unwrap();
            assert_eq!(json, seat.to_string());
            assert_eq!(serde_json::from_str::<Turn>(&json).unwrap(), turn);
        }
    }

    #[test]
    fn must_reject_a_seat_from_the_wire_that_is_out_of_range() {
        assert!(serde_json::from_str::<Turn>("200").is_err());
        assert!(serde_json::from_str::<Turn>("4").is_err());
    }

    #[test]
    fn must_reject_a_trick_whose_starter_is_out_of_range() {
        let json = r#"{"starter":200,"cards":[null,null,null,null]}"#;
        assert!(serde_json::from_str::<crate::game::Trick>(json).is_err());
    }
}
