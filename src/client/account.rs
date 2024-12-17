use serde::{
    Deserialize,
    Serialize,
};

use crate::ui::provider::descriptor::VecSerialize;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Account {
    pub servername: String,
    pub server: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
    pub server_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct Accounts {
    pub accounts: Vec<Account>,
}

impl VecSerialize<Account> for Vec<Account> {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).expect("Failed to serialize Vec<Descriptor>")
    }
}
