// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::Intent;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
