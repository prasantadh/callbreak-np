use serde::{Deserialize, Serialize};

use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerID(Rc<str>);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Player {
    // player ID is expected to be unique
    id: PlayerID,
}

impl Player {
    pub fn new(id: PlayerID) -> Self {
        Player { id }
    }

    pub(crate) fn get_id(&self) -> &PlayerID {
        &self.id
    }
}

impl<S: Into<Rc<str>>> From<S> for PlayerID {
    fn from(value: S) -> Self {
        PlayerID(value.into())
    }
}
