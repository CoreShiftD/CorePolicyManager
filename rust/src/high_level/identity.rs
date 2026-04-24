use crate::core::Intent;

#[derive(Clone, Debug)]
#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Principal {
    System,
    User(u32),
    Addon(u32),
}

impl Principal {
    pub fn new_user(uid: u32) -> Self {
        if uid == 0 {
            Principal::System
        } else {
            Principal::User(uid)
        }
    }
}

#[derive(Clone)]
pub struct Request {
    pub client_id: Option<u32>,
    pub principal: Principal,
    pub intent: Intent,
    pub cause: crate::core::CauseId,
}
