// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::Event;
use crate::core::state_view::StateView;
use crate::high_level::capability::CapabilityToken;
use crate::low_level::reactor::Event as ReactorEvent;

pub struct AddonSpec {
    pub id: u32,
    pub capability: CapabilityToken,
    pub max_actions_per_tick: u32,
}

pub trait Addon {
    fn on_reactor_event(
        &mut self,
        _state: &dyn StateView,
        _event: &ReactorEvent,
    ) -> Vec<crate::high_level::identity::Request> {
        Vec::new()
    }
    fn on_core_event(
        &mut self,
        _state: &dyn StateView,
        _event: &Event,
    ) -> Vec<crate::high_level::identity::Request> {
        Vec::new()
    }
    /// Return a pure policy-state snapshot if this addon is the PreloadAddon,
    /// otherwise `None`.  The runtime uses this to build the status report
    /// without downcasting.
    fn preload_snapshot(&self) -> Option<crate::high_level::api::PreloadSnapshot> {
        None
    }
}

pub struct NoOpAddon;
impl Addon for NoOpAddon {}

pub struct EchoAddon;
impl Addon for EchoAddon {}
