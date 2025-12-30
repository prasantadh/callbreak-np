use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Player {
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
