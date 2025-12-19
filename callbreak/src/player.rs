use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerID(String);

pub type ID = str;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Player {
    // player ID is expected to be unique
    id: String,
}

impl Player {
    pub fn new(id: &str) -> Self {
        Player { id: id.to_owned() }
    }

    pub(crate) fn get_id(&self) -> &str {
        self.id.as_ref()
    }
}

impl<S: Into<String>> From<S> for PlayerID {
    fn from(value: S) -> Self {
        PlayerID(value.into())
    }
}
