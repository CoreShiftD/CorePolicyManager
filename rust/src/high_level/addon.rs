use crate::core::Intent;
use crate::high_level::capability::CapabilityToken;
use crate::low_level::reactor::Event;
use crate::core::state_view::StateView;

pub struct AddonSpec {
    pub id: u32,
    pub capability: CapabilityToken,
    pub max_actions_per_tick: u32,
}

pub trait Addon {
    fn on_event(&mut self, state: &dyn StateView, _event: &Event) -> Vec<crate::high_level::identity::Request>;
}

pub struct NoOpAddon;
impl Addon for NoOpAddon {
    fn on_event(&mut self, _state: &dyn StateView, __event: &Event) -> Vec<crate::high_level::identity::Request> {
        Vec::new()
    }
}

pub struct EchoAddon;
impl Addon for EchoAddon {
    fn on_event(&mut self, _state: &dyn StateView, _event: &Event) -> Vec<crate::high_level::identity::Request> {
        // Echo: Return empty intent vector, or some dummy intent if needed.
        // For testing we keep it returning empty unless we want a real Intent.
        Vec::new()
    }
}
