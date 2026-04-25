// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::Intent;
use crate::high_level::identity::Request;

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidJobId,
    InvalidHandle,
    DuplicateProcessAssignment,
    IllegalTransition,
    Unknown,
}

pub fn validate_request(
    req: &Request,
    state: &dyn crate::core::state_view::StateView,
) -> Result<(), ErrorCode> {
    match &req.intent {
        Intent::Submit { .. } => {
            // Further submit validation if needed
        }
        Intent::Control { id, .. } | Intent::Query { id } => {
            let job = state.job(*id);
            if job.is_none() {
                return Err(ErrorCode::InvalidJobId);
            }
        }
        _ => {}
    }
    Ok(())
}
