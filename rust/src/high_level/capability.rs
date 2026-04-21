use crate::core::ActionKind;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct CapabilityToken {
    pub restricted_actions: u64,
}

impl CapabilityToken {
    pub fn empty() -> Self {
        Self {
            restricted_actions: !0,
        }
    }

    pub fn allow_all() -> Self {
        Self {
            restricted_actions: 0,
        }
    }

    pub fn new(restricted_bits: u64) -> Self {
        Self {
            restricted_actions: restricted_bits,
        }
    }

    pub fn allows(&self, kind: ActionKind) -> bool {
        let bit = 1u64 << (kind as u64);
        (self.restricted_actions & bit) == 0
    }
}

pub struct CapabilityRegistry {
    pub map: HashMap<u32, CapabilityToken>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, uid: u32) -> Option<&CapabilityToken> {
        self.map.get(&uid)
    }

    pub fn insert(&mut self, uid: u32, token: CapabilityToken) {
        self.map.insert(uid, token);
    }

    pub fn allows(&self, principal: &crate::high_level::identity::Principal, kind: crate::core::ActionKind) -> bool {
        let uid = match principal {
            crate::high_level::identity::Principal::System => 0,
            crate::high_level::identity::Principal::User(u) => *u,
            crate::high_level::identity::Principal::Addon(u) => *u,
        };
        if let Some(token) = self.get(uid) {
            token.allows(kind)
        } else {
            false // default to denied
        }
    }
}
