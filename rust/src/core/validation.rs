use crate::high_level::identity::Request;
use crate::core::Intent;

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
