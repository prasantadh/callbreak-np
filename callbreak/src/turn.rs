use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Turn(usize);

const MAX_TURN: usize = 3;

impl Turn {
    pub fn new(value: usize) -> Self {
        Turn(value % 4)
    }

    pub fn next(&self) -> Self {
        let next = (self.0 + 1) % 4;
        assert!(next <= MAX_TURN);
        Self(next)
    }
}

impl<T> Index<Turn> for Vec<T> {
    type Output = T;
    fn index(&self, turn: Turn) -> &Self::Output {
        self.index(turn.0)
    }
}

impl<T> IndexMut<Turn> for Vec<T> {
    fn index_mut(&mut self, turn: Turn) -> &mut Self::Output {
        &mut self[turn.0]
    }
}

impl<T> Index<Turn> for [T] {
    type Output = T;
    fn index(&self, turn: Turn) -> &Self::Output {
        self.index(turn.0)
    }
}

impl<T> IndexMut<Turn> for [T] {
    fn index_mut(&mut self, turn: Turn) -> &mut Self::Output {
        &mut self[turn.0]
    }
}
